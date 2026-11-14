'use strict';

// VM-only tests: every launch implementation below is an in-memory fake.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const Module = require('node:module');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const vm = require('node:vm');

const root = path.resolve(__dirname, '../..');
const upstream = path.resolve(root, '../../../thirdparty/playwright-v1.58.0');
const manifest = path.join(__dirname, 'playwright-core-dummy-node-launch-manifest.json');
const asset = path.join(upstream, 'tests/assets/dummy_bad_browser_executable.js');
const testMode = path.join(upstream, 'tests/config/testMode.ts');
const preload = fs.readFileSync(path.join(__dirname, 'playwright-core-dummy-node-launch-preload.cjs'), 'utf8');
const sha256 = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');

function fixture(t, overrides = {}, fakeFs = fs) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'pwright-dummy-guard-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const receipt = path.join(directory, 'receipt.jsonl');
  fs.writeFileSync(receipt, '');
  const calls = [];
  const implementation = {
    chromium: {
      async launch(options) {
        calls.push(options);
        throw new Error('Browser logs: dummy exited');
      },
    },
  };
  const nativeModule = {
    _load(request) {
      if (request === 'playwright-core') return implementation;
      throw new Error(`unexpected module ${request}`);
    },
  };
  const process = {
    env: {
      PWRIGHT_DUMMY_LAUNCH_MODE: 'candidate',
      PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM: upstream,
      PWRIGHT_DUMMY_LAUNCH_MANIFEST: manifest,
      PWRIGHT_DUMMY_LAUNCH_MANIFEST_SHA256: sha256(manifest),
      PWRIGHT_DUMMY_LAUNCH_RECEIPT: receipt,
      PWTEST_MODE: 'default',
      ...overrides,
    },
  };
  const fakeRequire = name => {
    if (name === 'node:crypto') return crypto;
    if (name === 'node:fs') return fakeFs;
    if (name === 'node:module') return nativeModule;
    if (name === 'node:path') return path;
    throw new Error(`unexpected preload dependency ${name}`);
  };
  return {
    install() { vm.runInNewContext(preload, { require: fakeRequire, process }); },
    implementation, nativeModule, receipt, calls,
  };
}

test('guard rejects inherited CDP before installing a launch hook', t => {
  // Arrange
  const guarded = fixture(t, { PWRIGHT_CANDIDATE_CDP_ENDPOINT: 'http://127.0.0.1:9222' });

  // Act and Assert
  assert.throws(() => guarded.install(), /forbids PWRIGHT_CANDIDATE_CDP_ENDPOINT/);
  assert.equal(fs.readFileSync(guarded.receipt, 'utf8'), '');
});

test('guard rejects inherited NODE_PATH before installing a launch hook', t => {
  // Arrange
  const guarded = fixture(t, { NODE_PATH: '/tmp/bypass' });

  // Act and Assert
  assert.throws(() => guarded.install(), /forbids NODE_PATH/);
  assert.equal(fs.readFileSync(guarded.receipt, 'utf8'), '');
});

test('guard rejects an installed-browser path before the public launch', async t => {
  // Arrange
  const guarded = fixture(t);
  guarded.install();
  guarded.nativeModule._load('playwright-core', { filename: testMode }, false);

  // Act and Assert
  await assert.rejects(
    guarded.implementation.chromium.launch({ executablePath: '/Applications/Google Chrome.app' }),
    /differs from pinned asset/,
  );
  assert.equal(guarded.calls.length, 0);
  assert.equal(fs.readFileSync(guarded.receipt, 'utf8'), '');
});

test('guard rejects even a pinned path with additional launch options', async t => {
  // Arrange
  const guarded = fixture(t);
  guarded.install();
  guarded.nativeModule._load('playwright-core', { filename: testMode }, false);

  // Act and Assert
  await assert.rejects(
    guarded.implementation.chromium.launch({ executablePath: asset, channel: 'chrome' }),
    /only explicit executablePath/,
  );
  assert.equal(guarded.calls.length, 0);
});

test('guard rejects changed dummy asset bytes before the public launch', async t => {
  // Arrange
  const fakeFs = Object.assign(Object.create(fs), {
    readFileSync(file, ...args) {
      if (String(file) === asset) return Buffer.from('#!/usr/bin/env node\nprocess.exit(0);\n');
      return fs.readFileSync(file, ...args);
    },
  });
  const guarded = fixture(t, {}, fakeFs);
  guarded.install();
  guarded.nativeModule._load('playwright-core', { filename: testMode }, false);

  // Act and Assert
  await assert.rejects(
    guarded.implementation.chromium.launch({ executablePath: asset }),
    /not the pinned regular file/,
  );
  assert.equal(guarded.calls.length, 0);
});

test('only a fake failing launch can produce the required two-event receipt', async t => {
  // Arrange
  const guarded = fixture(t);
  guarded.install();
  guarded.nativeModule._load('playwright-core', { filename: testMode }, false);

  // Act
  await assert.rejects(
    guarded.implementation.chromium.launch({ executablePath: asset }),
    /Browser logs: dummy exited/,
  );

  // Assert
  assert.equal(guarded.calls.length, 1);
  assert.deepEqual(fs.readFileSync(guarded.receipt, 'utf8').trim().split('\n').map(line => JSON.parse(line).event),
    ['launch-invoked', 'launch-failed']);
});
