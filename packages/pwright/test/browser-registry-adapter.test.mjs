import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const indexPath = fileURLToPath(indexUrl);
const sourceRequire = createRequire(indexUrl);
const captureSnapshot = sourceRequire('./browser-registry-snapshot.cjs');

function fixture(profile = 'browser-registry', environment = { PLAYWRIGHT_BROWSERS_PATH: '/registry-A' }) {
  const calls = [];
  const artifact = '/test-only/browser-registry.node';
  let nativeLoads = 0;
  let environments = 0;
  let snapshotCalls = 0;
  class NativeEnvironment {
    constructor() { environments += 1; }
  }
  const binary = {
    NativeEnvironment,
    browserExecutablePathSnapshot(engine, packageRoot, snapshot) {
      calls.push([engine, packageRoot, snapshot]);
      return `/registry-does-not-exist/${engine}`;
    },
  };
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return {
      existsSync(path) {
        if (path !== artifact) throw new Error(`unexpected browser-file probe: ${path}`);
        return true;
      },
    };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile },
      nativeFeatures: selected => selected === 'browser-registry' ? ['browser-registry'] : selected === 'owned-launch' ? ['owned-launch'] : [],
      verifyNativeArtifact() {},
    };
    if (specifier === './browser-registry-snapshot.cjs') return () => {
      snapshotCalls += 1;
      return captureSnapshot(environment, '/test-only/cwd', '/test-only/home');
    };
    if (specifier === artifact) { nativeLoads += 1; return binary; }
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports', '__dirname'], {
    filename: indexPath,
  })(require, module, module.exports, dirname(indexPath));
  return { api: module.exports, calls, nativeLoads: () => nativeLoads, environments: () => environments, snapshotCalls: () => snapshotCalls };
}

for (const engine of ['chromium', 'firefox', 'webkit']) {
  test(`${engine} BrowserType path is synchronous Rust metadata without a session or file probe`, () => {
    // Arrange
    const { api, calls, nativeLoads, environments, snapshotCalls } = fixture();
    const packageRoot = dirname(dirname(indexPath));

    // Act
    const path = api[engine].executablePath();

    // Assert
    assert.equal(path, `/registry-does-not-exist/${engine}`);
    assert.deepEqual(calls, [[engine, packageRoot, {
      browsersPath: '/registry-A',
      initCwd: undefined,
      xdgCacheHome: undefined,
      localAppData: undefined,
      home: '/test-only/home',
      cwd: '/test-only/cwd',
      hostPlatformOverride: undefined,
    }]]);
    assert.equal(snapshotCalls(), 1);
    assert.equal(nativeLoads(), 1);
    assert.equal(environments(), 0);
  });
}

test('browser-registry snapshots environment at import before the first native call', () => {
  // Arrange
  const environment = { PLAYWRIGHT_BROWSERS_PATH: '/registry-A' };
  const { api, calls, nativeLoads, snapshotCalls } = fixture('browser-registry', environment);
  environment.PLAYWRIGHT_BROWSERS_PATH = '/registry-B';
  assert.equal(nativeLoads(), 0);

  // Act
  api.chromium.executablePath();
  environment.PLAYWRIGHT_BROWSERS_PATH = '/registry-C';
  api.firefox.executablePath();

  // Assert
  assert.equal(snapshotCalls(), 1);
  assert.equal(nativeLoads(), 1);
  assert.equal(calls[0][2], calls[1][2]);
  assert.equal(calls[0][2].browsersPath, '/registry-A');
  assert.ok(Object.isFrozen(calls[0][2]));
});

test('snapshot honors direct, npm_config, and npm_package_config precedence', () => {
  // Arrange
  const environment = {
    PLAYWRIGHT_BROWSERS_PATH: '',
    npm_config_playwright_browsers_path: '/npm-config',
    npm_package_config_playwright_browsers_path: '/npm-package',
    npm_config_init_cwd: 'relative-install',
    npm_package_config_init_cwd: '/ignored',
  };

  // Act
  const snapshot = captureSnapshot(environment, '/task', '/home');

  // Assert
  assert.equal(snapshot.browsersPath, '');
  assert.equal(snapshot.initCwd, 'relative-install');
  assert.equal(snapshot.cwd, '/task');
  assert.equal(snapshot.home, '/home');
  assert.ok(Object.isFrozen(snapshot));
});

test('snapshot uses package config when higher-priority names are absent', () => {
  // Arrange
  const environment = { npm_package_config_playwright_browsers_path: '/npm-package' };

  // Act
  const snapshot = captureSnapshot(environment, '/task', '/home');

  // Assert
  assert.equal(snapshot.browsersPath, '/npm-package');
});

for (const profile of ['default', 'owned-launch']) {
  test(`${profile} profile rejects BrowserType metadata before loading native code`, () => {
    // Arrange
    const { api, calls, nativeLoads, environments, snapshotCalls } = fixture(profile);

    // Act / Assert
    assert.throws(() => api.chromium.executablePath(), {
      name: 'UnsupportedOperationError',
      code: 'PWRIGHT_UNSUPPORTED',
    });
    assert.deepEqual(calls, []);
    assert.equal(nativeLoads(), 0);
    assert.equal(environments(), 0);
    assert.equal(snapshotCalls(), 0);
  });
}

for (const [profile, expected] of [
  ['browser-registry', ['browser-registry']],
  ['default', []],
  ['owned-launch', ['owned-launch']],
]) {
  test(`${profile} feature selection remains independent of browser registry`, () => {
    // Arrange
    const { nativeFeatures } = sourceRequire('./native-artifact.cjs');

    // Act
    const features = nativeFeatures(profile);

    // Assert
    assert.deepEqual(features, expected);
  });
}

test('browser-registry profile resolves its own native artifact', () => {
  // Arrange
  const { resolveNativeArtifact } = sourceRequire('./native-artifact.cjs');

  // Act
  const selected = resolveNativeArtifact('browser-registry');

  // Assert
  assert.match(selected.path, /\/native\/browser-registry\/pwright\.node$/);
});

for (const { name, profile, change, expectedMismatch } of [
  { name: 'matching marker and export', profile: 'browser-registry', change: {}, expectedMismatch: false },
  { name: 'missing snapshot executable-path export', profile: 'browser-registry', change: { browserExecutablePathSnapshot: undefined }, expectedMismatch: true },
  { name: 'stale two-argument executable-path export', profile: 'browser-registry', change: { browserExecutablePath: () => '/stale' }, expectedMismatch: true },
  { name: 'disabled marker', profile: 'browser-registry', change: { browserRegistryEnabled: () => false }, expectedMismatch: true },
  { name: 'registry binary selected as default', profile: 'default', change: {}, expectedMismatch: true },
]) {
  test(`native verifier ${expectedMismatch ? 'rejects' : 'accepts'} ${name}`, () => {
    // Arrange
    const { resolveNativeArtifact, verifyNativeArtifact } = sourceRequire('./native-artifact.cjs');
    class NativeEnvironment {}
    const selected = resolveNativeArtifact(profile);
    const binary = {
      nativeAbiVersion: () => 1,
      NativeEnvironment,
      browserRegistryEnabled: () => true,
      browserExecutablePathSnapshot: () => '/expected/not-installed',
      ...change,
    };
    let error;

    // Act
    try { verifyNativeArtifact(binary, selected); } catch (caught) { error = caught; }

    // Assert
    if (expectedMismatch) assert.match(error?.message ?? '', /profile mismatch/);
    else assert.equal(error, undefined);
  });
}
