'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const test = require('node:test');
const vm = require('node:vm');

const preload = fs.readFileSync(path.join(__dirname, 'playwright-core-owned-launch-preload.cjs'), 'utf8');
const upstream = '/test-only/playwright';
const testModeSource = path.join(upstream, 'tests', 'config', 'testMode.ts');
const executable = '/test-only/Chrome';

function fixture(mode, marker = {}) {
  const calls = [];
  const lines = [];
  const implementation = {
    chromium: {
      async launch(options) {
        calls.push(options);
        return { close: async () => {} };
      },
    },
  };
  const nativeModule = {
    _load(request) {
      if (request === 'playwright-core') return implementation;
      throw new Error(`unexpected require ${request}`);
    },
  };
  const process = {
    env: {
      PWRIGHT_OFFICIAL_OWNED_LAUNCH_MODE: mode,
      PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM: upstream,
      PWRIGHT_OFFICIAL_OWNED_LAUNCH_RECEIPT: '/test-only/receipt',
      CRPATH: executable,
      ...marker,
    },
  };
  const requireFake = name => {
    if (name === 'node:fs') return { appendFileSync(_file, line) { lines.push(JSON.parse(line)); } };
    if (name === 'node:module') return nativeModule;
    if (name === 'node:path') return path;
    throw new Error(`unexpected preload dependency ${name}`);
  };
  vm.runInNewContext(preload, { require: requireFake, process });
  return { calls, implementation, lines, nativeModule };
}

test('candidate owned lane forwards pinned defaults into genuine public launch', async () => {
  // Arrange
  const { calls, implementation, lines, nativeModule } = fixture('candidate');
  nativeModule._load('playwright-core', { filename: testModeSource }, false);
  implementation._defaultLaunchOptions = {
    executablePath: executable,
    headless: false,
    handleSIGINT: false,
    tracesDir: '/test-only/traces',
  };

  // Act
  const browser = await implementation.chromium.launch();

  // Assert
  assert.equal(typeof browser.close, 'function');
  assert.deepEqual(JSON.parse(JSON.stringify(calls)), [{ executablePath: executable, headless: false }]);
  assert.deepEqual(lines.map(event => event.event), ['launch-invoked', 'launch-succeeded']);
  assert.ok(lines.every(event => event.publicMethod === 'chromium.launch'));
});

test('oracle owned lane keeps official default-option merger in control', async () => {
  // Arrange
  const { calls, implementation, lines, nativeModule } = fixture('oracle');
  nativeModule._load('playwright-core', { filename: testModeSource }, false);
  implementation._defaultLaunchOptions = { executablePath: executable, headless: true };

  // Act
  await implementation.chromium.launch();

  // Assert
  assert.deepEqual(calls, [undefined]);
  assert.deepEqual(lines.map(event => event.event), ['launch-invoked', 'launch-succeeded']);
});

test('owned lane rejects CDP adapter inheritance before installing hooks', () => {
  // Arrange
  const inherited = { PWRIGHT_CANDIDATE_CDP_ENDPOINT: 'http://127.0.0.1:9222' };

  // Act and Assert
  assert.throws(() => fixture('candidate', inherited), /forbids the historical CDP adapter/);
});

test('owned one-spec lane rejects explicit launch options', async () => {
  // Arrange
  const { implementation, nativeModule } = fixture('candidate');
  nativeModule._load('playwright-core', { filename: testModeSource }, false);
  implementation._defaultLaunchOptions = { executablePath: executable, headless: true };

  // Act and Assert
  await assert.rejects(implementation.chromium.launch({ args: ['--foo'] }), /forbids explicit launch options/);
});
