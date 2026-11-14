'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { after, test } = require('node:test');
const { buildCommand, rewriteWorkspaceLinks } = require('./bootstrap_playwright_core455_source_only.cjs');

const upstream = fs.mkdtempSync(path.join(os.tmpdir(), 'pwright-core455-bootstrap-test-'));
after(() => fs.rmSync(upstream, { recursive: true, force: true }));

function directory(relative) {
  const result = path.join(upstream, relative);
  fs.mkdirSync(result, { recursive: true });
  return result;
}

test('maps only the pinned upstream npm ci steps to pnpm frozen install', () => {
  // Arrange
  const cwd = directory('packages/playwright/bundles/expect');
  const invocation = ['ci', '--save=false', '--fund=false', '--audit=false', '--omit=optional'];

  // Act
  const mapped = buildCommand('npm', invocation, { cwd }, upstream);

  // Assert
  assert.deepEqual(mapped, ['pnpm', ['--config.node-linker=hoisted', 'install',
    '--ignore-workspace', '--frozen-lockfile', '--ignore-scripts']]);
});

test('rejects unapproved npm build children instead of running npm', () => {
  // Arrange
  const cwd = directory('packages/playwright/bundles/expect');

  // Act
  const wrongArgs = () => buildCommand('npm', ['install'], { cwd }, upstream);
  const wrongDirectory = () => buildCommand('npm',
    ['ci', '--save=false', '--fund=false', '--audit=false'],
    { cwd: directory('tests/library') }, upstream);

  // Assert
  assert.throws(wrongArgs, /unapproved npm invocation/);
  assert.throws(wrongDirectory, /unapproved npm cwd/);
});

test('maps only the pinned Vite build children to pnpm exec', () => {
  // Arrange
  const cwd = directory('packages/trace-viewer');

  // Act
  const mapped = buildCommand('npx', ['vite', 'build'], { cwd }, upstream);
  const forbidden = () => buildCommand('npx', ['playwright', 'install'], { cwd }, upstream);

  // Assert
  assert.deepEqual(mapped, ['pnpm', ['exec', 'vite', 'build']]);
  assert.throws(forbidden, /unapproved npx invocation/);
});

test('leaves upstream node build helpers unchanged', () => {
  // Arrange
  const args = ['utils/generate_injected.js'];

  // Act
  const mapped = buildCommand('node', args, { cwd: upstream }, upstream);

  // Assert
  assert.deepEqual(mapped, ['node', args]);
});

function lockFixture() {
  const importers = { '.': {} };
  const references = [
    ['packages/playwright-browser-chromium', 'playwright-core'],
    ['packages/playwright-browser-firefox', 'playwright-core'],
    ['packages/playwright-browser-webkit', 'playwright-core'],
    ['packages/playwright-chromium', 'playwright-core'],
    ['packages/playwright-client', 'playwright-core'],
    ['packages/playwright-ct-core', 'playwright'],
    ['packages/playwright-ct-core', 'playwright-core'],
    ['packages/playwright-ct-react', '@playwright/experimental-ct-core'],
    ['packages/playwright-ct-react17', '@playwright/experimental-ct-core'],
    ['packages/playwright-ct-svelte', '@playwright/experimental-ct-core'],
    ['packages/playwright-ct-vue', '@playwright/experimental-ct-core'],
    ['packages/playwright-firefox', 'playwright-core'],
    ['packages/playwright-test', 'playwright'],
    ['packages/playwright-webkit', 'playwright-core'],
    ['packages/playwright', 'playwright-core'],
  ];
  for (const [importer, name] of references) {
    importers[importer] ??= { dependencies: {} };
    importers[importer].dependencies[name] = {
      specifier: '1.58.0',
      version: name === '@playwright/experimental-ct-core' ? '1.58.0(vite@6.4.1)' : '1.58.0',
    };
  }
  let padding = 0;
  while (Object.keys(importers).length < 20)
    importers[`packages/padding-${padding++}`] = {};
  return { importers };
}

test('rewrites exactly fifteen pinned local workspace references', () => {
  // Arrange
  const lock = lockFixture();
  const packages = [
    { name: 'playwright', relative: 'packages/playwright', version: '1.58.0' },
    { name: 'playwright-core', relative: 'packages/playwright-core', version: '1.58.0' },
    { name: '@playwright/experimental-ct-core', relative: 'packages/playwright-ct-core', version: '1.58.0' },
  ];

  // Act
  rewriteWorkspaceLinks(lock, packages);

  // Assert
  assert.equal(lock.importers['packages/playwright'].dependencies['playwright-core'].version,
    'link:../playwright-core');
  assert.equal(lock.importers['packages/playwright-test'].dependencies.playwright.version,
    'link:../playwright');
  assert.equal(lock.importers['packages/playwright-ct-react'].dependencies['@playwright/experimental-ct-core'].version,
    'link:../playwright-ct-core');
});

test('rejects a mismatched imported local version', () => {
  // Arrange
  const lock = lockFixture();
  lock.importers['packages/playwright'].dependencies['playwright-core'].version = '1.58.1';
  const packages = [
    { name: 'playwright', relative: 'packages/playwright', version: '1.58.0' },
    { name: 'playwright-core', relative: 'packages/playwright-core', version: '1.58.0' },
    { name: '@playwright/experimental-ct-core', relative: 'packages/playwright-ct-core', version: '1.58.0' },
  ];

  // Act
  const rewrite = () => rewriteWorkspaceLinks(lock, packages);

  // Assert
  assert.throws(rewrite, /unexpected local dependency resolution/);
});
