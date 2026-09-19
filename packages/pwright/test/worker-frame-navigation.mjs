import assert from 'node:assert/strict';
import { once } from 'node:events';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const { endpoint, fixture, destination, mode } = workerData;
assert.ok(['owned-running', 'borrowed-running', 'owned-queued'].includes(mode));
const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
let page;
if (mode === 'borrowed-running') {
  const matches = browser.contexts().flatMap(context => context.pages()).filter(page => page.url() === fixture);
  assert.equal(matches.length, 1, 'exact borrowed fixture must be unique');
  [page] = matches;
} else {
  const context = await browser.newContext();
  page = await context.newPage();
  await page.goto(fixture, { timeout: 5000 });
}
const begin = once(parentPort, 'message');
parentPort.postMessage({ phase: 'ready' });
assert.equal((await begin)[0], 'navigate');
const outcome = page.mainFrame().goto(destination, { waitUntil: 'load', timeout: 30000 })
  .then(response => ({ status: response?.status() }), error => ({ error: error.code }));

if (mode === 'owned-queued') {
  // Diagnostic only: keep the JS callback unserviced while the Rust result is
  // queued. The FFI result owner must survive until environment cleanup.
  const clock = new Int32Array(new SharedArrayBuffer(4));
  const limit = Date.now() + 10000;
  while (browser.pwrightFfiMetrics().queuedCommands !== 1) {
    assert.ok(Date.now() < limit, 'Frame result did not reach the settlement queue');
    Atomics.wait(clock, 0, 0, 5);
  }
  parentPort.postMessage({ phase: 'queued', ffi: browser.pwrightFfiMetrics() });
  Atomics.wait(clock, 0, 0, 30000);
} else {
  parentPort.postMessage({ phase: 'started', ffi: browser.pwrightFfiMetrics() });
}
const result = await outcome;
parentPort.postMessage({ phase: 'settled', result });
await browser.close();
