import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const upstreamRoot = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const upstreamPackage = upstreamRoot && join(upstreamRoot, 'packages/playwright-core');
const macArm64 = process.platform === 'darwin' && process.arch === 'arm64';
const differentialAvailable = macArm64 && upstreamPackage && existsSync(join(upstreamPackage, 'index.js'));

const expectedMacArm64Suffixes = {
  chromium: 'chromium-1208/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing',
  firefox: 'firefox-1509/firefox/Nightly.app/Contents/MacOS/firefox',
  webkit: 'webkit-2248/pw_run.sh',
};

const childSource = `
  const api = require(process.argv[1]);
  function change(values) {
    for (const [key, value] of Object.entries(values)) {
      if (key === '__cwd') {
        process.chdir(value);
        continue;
      }
      if (value === null) delete process.env[key];
      else process.env[key] = value;
    }
  }
  function paths() {
    return Object.fromEntries(['chromium', 'firefox', 'webkit'].map(engine => [engine, api[engine].executablePath()]));
  }
  change(JSON.parse(process.argv[2]));
  const first = paths();
  change(JSON.parse(process.argv[3]));
  process.stdout.write(JSON.stringify({ first, second: paths() }));
`;

function cleanEnvironment(overrides) {
  const environment = { ...process.env };
  for (const name of [
    'PLAYWRIGHT_BROWSERS_PATH',
    'npm_config_playwright_browsers_path',
    'npm_package_config_playwright_browsers_path',
    'INIT_CWD',
    'npm_config_init_cwd',
    'npm_package_config_init_cwd',
    'XDG_CACHE_HOME',
    'LOCALAPPDATA',
    'PLAYWRIGHT_HOST_PLATFORM_OVERRIDE',
  ]) delete environment[name];
  return { ...environment, ...overrides };
}

function publicPaths(entry, environment, beforeFirst = {}, afterFirst = {}) {
  const completed = spawnSync(process.execPath, [
    '-e', childSource, entry, JSON.stringify(beforeFirst), JSON.stringify(afterFirst),
  ], {
    cwd: packageRoot,
    env: cleanEnvironment(environment),
    encoding: 'utf8',
  });
  assert.equal(completed.status, 0, completed.stderr);
  return JSON.parse(completed.stdout);
}

function assertPinnedUpstream() {
  assert.equal(
    execFileSync('git', ['-C', upstreamRoot, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    '961381ec73a9fa2661d92504eab1a6bd6eaffc75',
  );
}

function compareWithOracle(environment, beforeFirst = {}, afterFirst = {}) {
  assertPinnedUpstream();
  const candidate = publicPaths(packageRoot, environment, beforeFirst, afterFirst);
  const oracle = publicPaths(upstreamPackage, environment, beforeFirst, afterFirst);
  assert.deepEqual(candidate, oracle);
  assert.deepEqual(candidate.first, candidate.second);
  return candidate;
}

for (const [engine, suffix] of Object.entries(expectedMacArm64Suffixes)) {
  test(`${engine} returns pinned metadata without installing a browser`, { skip: !macArm64 }, () => {
    // Arrange
    const temporary = mkdtempSync(join(tmpdir(), 'pwright-browser-registry-'));
    const registry = join(temporary, 'not-installed');

    try {
      // Act
      const actual = publicPaths(packageRoot, { PLAYWRIGHT_BROWSERS_PATH: registry }).first[engine];

      // Assert
      assert.equal(actual, join(registry, suffix));
      assert.equal(existsSync(actual), false);
    } finally {
      rmSync(temporary, { recursive: true });
    }
  });
}

test('absolute registry path stays at import value before and after first call', { skip: !differentialAvailable }, () => {
  // Arrange
  const registry = join(tmpdir(), 'pwright-registry-A-not-installed');
  const environment = { PLAYWRIGHT_BROWSERS_PATH: registry };
  const beforeFirst = { PLAYWRIGHT_BROWSERS_PATH: join(tmpdir(), 'pwright-registry-B-not-installed') };
  const afterFirst = { PLAYWRIGHT_BROWSERS_PATH: join(tmpdir(), 'pwright-registry-C-not-installed') };

  // Act
  const paths = compareWithOracle(environment, beforeFirst, afterFirst);

  // Assert
  for (const [engine, suffix] of Object.entries(expectedMacArm64Suffixes))
    assert.equal(paths.first[engine], join(registry, suffix));
});

test('npm configuration fallback is fixed at import', { skip: !differentialAvailable }, () => {
  // Arrange
  const npmRegistry = join(tmpdir(), 'pwright-npm-registry-A-not-installed');
  const environment = {
    npm_config_playwright_browsers_path: npmRegistry,
    npm_package_config_playwright_browsers_path: join(tmpdir(), 'pwright-npm-package-not-installed'),
  };

  // Act
  const paths = compareWithOracle(environment, {
    npm_config_playwright_browsers_path: join(tmpdir(), 'pwright-npm-registry-B-not-installed'),
  });

  // Assert
  assert.equal(paths.first.chromium, join(npmRegistry, expectedMacArm64Suffixes.chromium));
});

test('package configuration fallback applies when higher-priority names are absent', { skip: !differentialAvailable }, () => {
  // Arrange
  const registry = join(tmpdir(), 'pwright-package-registry-not-installed');

  // Act
  const paths = compareWithOracle({ npm_package_config_playwright_browsers_path: registry });

  // Assert
  assert.equal(paths.first.webkit, join(registry, expectedMacArm64Suffixes.webkit));
});

test('direct empty browser path chooses the platform cache rather than npm fallback', { skip: !differentialAvailable }, () => {
  // Arrange
  const home = join(tmpdir(), 'pwright-empty-home-not-installed');
  const environment = {
    HOME: home,
    PLAYWRIGHT_BROWSERS_PATH: '',
    npm_config_playwright_browsers_path: join(tmpdir(), 'pwright-ignored-npm'),
  };

  // Act
  const paths = compareWithOracle(environment);

  // Assert
  assert.equal(paths.first.chromium, join(home, 'Library/Caches/ms-playwright', expectedMacArm64Suffixes.chromium));
});

test('package-local override stays package-local after environment mutation', { skip: !differentialAvailable }, () => {
  // Arrange
  assertPinnedUpstream();
  const environment = { PLAYWRIGHT_BROWSERS_PATH: '0' };
  const mutation = { PLAYWRIGHT_BROWSERS_PATH: join(tmpdir(), 'pwright-ignored-registry') };

  // Act
  const candidate = publicPaths(packageRoot, environment, mutation);
  const oracle = publicPaths(upstreamPackage, environment, mutation);

  // Assert
  for (const [engine, suffix] of Object.entries(expectedMacArm64Suffixes)) {
    assert.equal(relative(join(packageRoot, '.local-browsers'), candidate.first[engine]), suffix);
    assert.equal(relative(join(upstreamPackage, '.local-browsers'), oracle.first[engine]), suffix);
  }
  assert.deepEqual(candidate.first, candidate.second);
  assert.deepEqual(oracle.first, oracle.second);
});

test('relative browser path and relative INIT_CWD use the import cwd', { skip: !differentialAvailable }, () => {
  // Arrange
  const environment = { PLAYWRIGHT_BROWSERS_PATH: 'cache/../registry', INIT_CWD: 'install-root' };

  // Act
  const paths = compareWithOracle(environment, { INIT_CWD: 'changed-root' });

  // Assert
  assert.equal(paths.first.chromium, join(packageRoot, 'install-root/registry', expectedMacArm64Suffixes.chromium));
});

test('relative browser path without INIT_CWD ignores later cwd changes', { skip: !differentialAvailable }, () => {
  // Arrange
  const temporary = mkdtempSync(join(tmpdir(), 'pwright-browser-registry-cwd-'));
  const afterFirstDirectory = join(temporary, 'after-first');
  const environment = { PLAYWRIGHT_BROWSERS_PATH: 'registry-not-installed' };
  mkdirSync(afterFirstDirectory);

  try {
    // Act
    const paths = compareWithOracle(environment, { __cwd: temporary }, { __cwd: afterFirstDirectory });

    // Assert
    assert.equal(paths.first.chromium, join(packageRoot, 'registry-not-installed', expectedMacArm64Suffixes.chromium));
  } finally {
    rmSync(temporary, { recursive: true });
  }
});

test('absolute browser path ignores a relative INIT_CWD', { skip: !differentialAvailable }, () => {
  // Arrange
  const registry = join(tmpdir(), 'pwright-absolute-root-not-installed');

  // Act
  const paths = compareWithOracle({ PLAYWRIGHT_BROWSERS_PATH: registry, INIT_CWD: 'relative-root' });

  // Assert
  assert.equal(paths.first.firefox, join(registry, expectedMacArm64Suffixes.firefox));
});

test('Mac cache home is fixed at import and ignores XDG_CACHE_HOME', { skip: !differentialAvailable }, () => {
  // Arrange
  const home = join(tmpdir(), 'pwright-home-A-not-installed');
  const environment = { HOME: home, XDG_CACHE_HOME: join(tmpdir(), 'pwright-xdg-ignored') };

  // Act
  const paths = compareWithOracle(environment, { HOME: join(tmpdir(), 'pwright-home-B-not-installed') });

  // Assert
  assert.equal(paths.first.webkit, join(home, 'Library/Caches/ms-playwright', expectedMacArm64Suffixes.webkit));
});

test('host override changes executable layout but not Mac cache root', { skip: !differentialAvailable }, () => {
  // Arrange
  const home = join(tmpdir(), 'pwright-override-home-not-installed');
  const environment = { HOME: home, PLAYWRIGHT_HOST_PLATFORM_OVERRIDE: 'debian11-arm64' };

  // Act
  const paths = compareWithOracle(environment, {
    PLAYWRIGHT_HOST_PLATFORM_OVERRIDE: 'ubuntu24.04-arm64',
  });

  // Assert
  const cache = join(home, 'Library/Caches/ms-playwright');
  assert.equal(paths.first.chromium, join(cache, 'chromium-1208/chrome-linux/chrome'));
  assert.equal(paths.first.firefox, join(cache, 'firefox-1509/firefox/firefox'));
  assert.equal(paths.first.webkit, join(cache, 'webkit_debian11_arm64_special-2105/pw_run.sh'));
});
