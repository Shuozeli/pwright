'use strict';

// Build the pinned upstream Core checkout without invoking npm or npx. The
// official test and fixture sources remain tracked, unmodified upstream code.

const assert = require('node:assert/strict');
const childProcess = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const PINNED_COMMIT = '961381ec73a9fa2661d92504eab1a6bd6eaffc75';
const EXPECTED_LOCAL_LINKS = [
  'packages/playwright-browser-chromium:dependencies:playwright-core',
  'packages/playwright-browser-firefox:dependencies:playwright-core',
  'packages/playwright-browser-webkit:dependencies:playwright-core',
  'packages/playwright-chromium:dependencies:playwright-core',
  'packages/playwright-client:dependencies:playwright-core',
  'packages/playwright-ct-core:dependencies:playwright',
  'packages/playwright-ct-core:dependencies:playwright-core',
  'packages/playwright-ct-react:dependencies:@playwright/experimental-ct-core',
  'packages/playwright-ct-react17:dependencies:@playwright/experimental-ct-core',
  'packages/playwright-ct-svelte:dependencies:@playwright/experimental-ct-core',
  'packages/playwright-ct-vue:dependencies:@playwright/experimental-ct-core',
  'packages/playwright-firefox:dependencies:playwright-core',
  'packages/playwright-test:dependencies:playwright',
  'packages/playwright-webkit:dependencies:playwright-core',
  'packages/playwright:dependencies:playwright-core',
].sort();
const NESTED_LOCK_DIRS = [
  'tests/playwright-test/stable-test-runner',
  'packages/playwright/bundles/babel',
  'packages/playwright/bundles/expect',
  'packages/playwright/bundles/utils',
  'packages/playwright-core/bundles/utils',
  'packages/playwright-core/bundles/zip',
  'packages/playwright-core/bundles/mcp',
];
const VITE_DIRS = [
  'packages/trace-viewer',
  'packages/html-reporter',
  'packages/recorder',
];
const WORKSPACE_CONFIG = 'packages:\n  - packages/*\nnodeLinker: hoisted\n';

function run(command, args, cwd, extraEnv = {}) {
  const result = childProcess.spawnSync(command, args, {
    cwd,
    env: { ...process.env, ...extraEnv },
    stdio: 'inherit',
  });
  if (result.error) throw result.error;
  if (result.status !== 0 || result.signal)
    throw new Error(`${command} ${args.join(' ')} failed in ${cwd}: status=${result.status} signal=${result.signal}`);
}

function git(upstream, ...args) {
  return childProcess.execFileSync('git', ['-C', upstream, ...args], { encoding: 'utf8' }).trim();
}

function requireCleanPinnedCheckout(upstream) {
  assert.equal(git(upstream, 'rev-parse', 'HEAD'), PINNED_COMMIT, 'wrong upstream commit');
  assert.equal(git(upstream, 'status', '--porcelain', '--untracked-files=no'), '',
    'upstream has tracked changes');
}

function packageEntries(upstream) {
  const directory = path.join(upstream, 'packages');
  const entries = [];
  for (const item of fs.readdirSync(directory, { withFileTypes: true })) {
    if (!item.isDirectory()) continue;
    const relative = `packages/${item.name}`;
    const manifestPath = path.join(directory, item.name, 'package.json');
    if (!fs.existsSync(manifestPath)) continue;
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    assert.equal(typeof manifest.name, 'string');
    assert.equal(typeof manifest.version, 'string');
    entries.push({ relative, name: manifest.name, version: manifest.version });
  }
  assert.equal(entries.length, 19, 'upstream workspace package count drifted');
  return entries;
}

function rewriteWorkspaceLinks(lock, packages) {
  assert.equal(Object.keys(lock.importers || {}).length, 20,
    'imported root lock lacks the pinned workspace importers');
  const locals = new Map(packages.map(item => [item.name, item]));
  assert.equal(locals.size, packages.length, 'duplicate upstream workspace package name');
  const changes = [];
  for (const [importer, descriptor] of Object.entries(lock.importers)) {
    for (const kind of ['dependencies', 'devDependencies', 'optionalDependencies']) {
      for (const [name, record] of Object.entries(descriptor[kind] || {})) {
        const local = locals.get(name);
        if (!local || record.specifier !== local.version) continue;
        const link = `link:${path.posix.relative(importer, local.relative)}`;
        assert.ok(record.version === local.version ||
          record.version.startsWith(`${local.version}(`) || record.version === link,
          `unexpected local dependency resolution: ${importer}:${name}`);
        record.version = link;
        changes.push(`${importer}:${kind}:${name}`);
      }
    }
  }
  assert.deepEqual(changes.sort(), EXPECTED_LOCAL_LINKS,
    'pinned upstream local package dependency set drifted');
  return lock;
}

function linkRootWorkspacePackages(upstream, packages) {
  const rootModules = path.join(upstream, 'node_modules');
  for (const item of packages) {
    const target = path.join(upstream, item.relative);
    const link = path.join(rootModules, ...item.name.split('/'));
    const existing = fs.lstatSync(link, { throwIfNoEntry: false });
    if (existing) {
      assert.equal(fs.realpathSync(link), fs.realpathSync(target),
        `root workspace package is not pinned local source: ${item.name}`);
      continue;
    }
    fs.mkdirSync(path.dirname(link), { recursive: true });
    fs.symlinkSync(path.relative(path.dirname(link), target), link, 'dir');
  }
}

function buildCommand(command, args, options, upstream) {
  const cwd = options?.cwd && fs.realpathSync(options.cwd);
  const canonicalUpstream = fs.realpathSync(upstream);
  const nested = new Set(NESTED_LOCK_DIRS.map(relative => path.join(canonicalUpstream, relative)));
  const vite = new Set(VITE_DIRS.map(relative => path.join(canonicalUpstream, relative)));
  if (command === 'npm') {
    assert.ok(nested.has(cwd), `unapproved npm cwd: ${cwd}`);
    assert.ok(
      JSON.stringify(args) === JSON.stringify(['ci', '--save=false', '--fund=false', '--audit=false']) ||
      JSON.stringify(args) === JSON.stringify(['ci', '--save=false', '--fund=false', '--audit=false', '--omit=optional']),
      `unapproved npm invocation: ${args.join(' ')}`,
    );
    return ['pnpm', ['--config.node-linker=hoisted', 'install', '--ignore-workspace',
      '--frozen-lockfile', '--ignore-scripts']];
  }
  if (command === 'npx') {
    assert.ok(vite.has(cwd) && Array.isArray(args) && args[0] === 'vite',
      `unapproved npx invocation: ${args?.join(' ')} in ${cwd}`);
    return ['pnpm', ['exec', ...args]];
  }
  return [command, args];
}

function installBuildPreload() {
  const upstream = fs.realpathSync(process.env.PWRIGHT_CORE455_UPSTREAM || '');
  requireCleanPinnedCheckout(upstream);
  const original = childProcess.spawn;
  childProcess.spawn = function pnpmOnlyUpstreamBuild(command, args, options) {
    const [mappedCommand, mappedArgs] = buildCommand(command, args, options, upstream);
    if (mappedCommand !== command)
      process.stdout.write(`[core455 pnpm build] ${command} -> ${mappedCommand} in ${options.cwd}\n`);
    return original.call(this, mappedCommand, mappedArgs, options);
  };
}

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function bootstrap(upstreamArgument) {
  const upstream = fs.realpathSync(upstreamArgument);
  requireCleanPinnedCheckout(upstream);
  const workspaceConfig = path.join(upstream, 'pnpm-workspace.yaml');
  if (fs.existsSync(workspaceConfig))
    assert.equal(fs.readFileSync(workspaceConfig, 'utf8'), WORKSPACE_CONFIG,
      'upstream pnpm workspace configuration differs from the pinned bootstrap');
  else
    fs.writeFileSync(workspaceConfig, WORKSPACE_CONFIG);

  const packages = packageEntries(upstream);
  const installEnv = {
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD: '1',
    ELECTRON_SKIP_BINARY_DOWNLOAD: '1',
  };
  run('pnpm', ['import'], upstream, installEnv);
  run('pnpm', ['--config.node-linker=hoisted', 'install', '--frozen-lockfile', '--ignore-scripts'],
    upstream, installEnv);

  const lockPath = path.join(upstream, 'pnpm-lock.yaml');
  const YAML = require(path.join(upstream, 'node_modules/yaml'));
  const lock = YAML.parse(fs.readFileSync(lockPath, 'utf8'));
  fs.writeFileSync(lockPath, YAML.stringify(rewriteWorkspaceLinks(lock, packages)));
  run('pnpm', ['--config.node-linker=hoisted', 'install', '--frozen-lockfile', '--ignore-scripts'],
    upstream, installEnv);
  linkRootWorkspacePackages(upstream, packages);

  for (const relative of NESTED_LOCK_DIRS) {
    const directory = path.join(upstream, relative);
    run('pnpm', ['import', '--ignore-workspace'], directory, installEnv);
    run('pnpm', ['--config.node-linker=hoisted', 'install', '--ignore-workspace',
      '--frozen-lockfile', '--ignore-scripts'], directory, installEnv);
  }

  run(process.execPath, ['--require', __filename, 'utils/build/build.js'], upstream, {
    ...installEnv,
    PWRIGHT_CORE455_BUILD_PRELOAD: '1',
    PWRIGHT_CORE455_UPSTREAM: upstream,
  });
  for (const relative of [
    'packages/playwright/lib/program.js',
    'packages/playwright/lib/common/process.js',
    'packages/playwright-core/lib/server/deviceDescriptors.js',
    'packages/playwright-core/lib/utils/isomorphic/urlMatch.js',
  ])
    assert.ok(fs.statSync(path.join(upstream, relative)).isFile(), `missing built output: ${relative}`);
  requireCleanPinnedCheckout(upstream);
  process.stdout.write(`Built clean pinned Core455 source with pnpm only: ${PINNED_COMMIT}\n`);
  process.stdout.write(`Derived pnpm lock SHA-256: ${sha256(lockPath)}\n`);
}

if (process.env.PWRIGHT_CORE455_BUILD_PRELOAD === '1') {
  installBuildPreload();
} else if (require.main === module) {
  if (process.argv.length !== 3)
    throw new Error('usage: bootstrap_playwright_core455_source_only.cjs PINNED_UPSTREAM_DIR');
  bootstrap(process.argv[2]);
}

module.exports = { buildCommand, packageEntries, rewriteWorkspaceLinks, linkRootWorkspacePackages };
