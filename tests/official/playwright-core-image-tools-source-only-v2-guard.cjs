'use strict';

// The v2 adapter label is explicit. The pinned v1 guard supplies the unchanged
// source/PNG AST check and no-I/O hooks, preserving the earlier v1 receipt.

const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const workerThreads = require('node:worker_threads');

const V1_GUARD = path.join(__dirname, 'playwright-core-image-tools-source-only-guard.cjs');
const V1_MANIFEST = path.join(__dirname, 'playwright-core-internal-source-only-image-tools-v1-manifest.json');
const V2_MANIFEST = path.join(__dirname, 'playwright-core-internal-source-only-image-tools-v2-manifest.json');
const V1_GUARD_SHA256 = '2d2f86427433f8567c049356727170cd628059f9fb9141ee916a6116f237fce2';
const V1_MANIFEST_SHA256 = '54e0a93447e2c12cdb597a294f1b25377e1756e6f7303f758e85835292ec2313';
const RUNNER = 'tests/playwright-test/stable-test-runner/node_modules/@playwright/test/cli.js';
const WORKER = 'tests/playwright-test/stable-test-runner/node_modules/playwright/lib/common/process.js';

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

// Requiring the v1 module must only import its exported hooks, not activate
// its v1 manifest check against a v2 process environment.
const active = process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
delete process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
let v1;
try {
  v1 = require(V1_GUARD);
} finally {
  if (active === undefined)
    delete process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE;
  else
    process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE = active;
}

function verifySuite(manifestPath, suiteId, upstream) {
  if (path.resolve(manifestPath) !== V2_MANIFEST ||
      sha256(V1_GUARD) !== V1_GUARD_SHA256 ||
      sha256(V1_MANIFEST) !== V1_MANIFEST_SHA256)
    throw new Error('image-tools v2 source-only guard dependencies changed');
  const original = JSON.parse(fs.readFileSync(V1_MANIFEST, 'utf8'));
  const expected = JSON.parse(JSON.stringify(original));
  delete expected.suites[0].candidateAdapter;
  expected.suites[0].sourceOnlyAdapter = 'upstream-private-source-only-no-pwright-runtime';
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  assert.deepStrictEqual(manifest, expected, 'image-tools v2 manifest changed more than the source-only adapter label');
  return v1.verifySuite(V1_MANIFEST, suiteId, upstream);
}

function activate() {
  const installed = Symbol.for('pwright.coreImageToolsSourceOnlyV2GuardInstalled');
  if (globalThis[installed]) return;
  const manifest = process.env.PWRIGHT_CORE_SOURCE_ONLY_MANIFEST;
  const suite = process.env.PWRIGHT_CORE_SOURCE_ONLY_SUITE;
  const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
  const receipt = process.env.PWRIGHT_CORE_SOURCE_ONLY_GUARD_RECEIPT;
  if (!manifest || !suite || !upstream || !receipt)
    throw new Error('image-tools v2 source-only guard environment is incomplete');
  const source = verifySuite(manifest, suite, upstream);
  const workerScript = path.join(upstream, WORKER);
  const isWorker = !workerThreads.isMainThread || typeof process.send === 'function' ||
      process.env.PWRIGHT_CORE_SOURCE_ONLY_TEST_WORKER === '1';
  const runnerScript = path.join(upstream, RUNNER);
  if (process.argv[1] && path.resolve(process.argv[1]) !== (isWorker ? workerScript : runnerScript))
    throw new Error(`image-tools v2 source-only guard is attached to an unapproved Node entrypoint: ${process.argv[1]}`);
  v1.installNoIoGuard({ allowRunnerFork: !isWorker, workerScript });
  globalThis[installed] = true;
  fs.appendFileSync(receipt, JSON.stringify({
    kind: !workerThreads.isMainThread ? 'loader-thread-no-io' :
      isWorker ? 'worker-no-io' : 'runner-no-io',
    pid: process.pid, threadId: workerThreads.threadId, suite, case: source.case,
  }) + '\n', { flag: 'a' });
}

if (require.main === module) {
  if (process.argv[2] !== '--check' || process.argv.length !== 6)
    throw new Error('usage: v2 guard --check MANIFEST SUITE UPSTREAM');
  process.stdout.write(JSON.stringify(verifySuite(process.argv[3], process.argv[4], process.argv[5])) + '\n');
} else if (process.env.PWRIGHT_CORE_SOURCE_ONLY_ACTIVE === '1') {
  activate();
}

module.exports = { verifySuite };
