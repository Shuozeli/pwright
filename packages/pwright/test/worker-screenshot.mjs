import assert from 'node:assert/strict';
import { once } from 'node:events';
import { setImmediate } from 'node:timers/promises';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';
import { zeroFfiMetrics } from './ffi-pressure.mjs';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.url, { timeout: 5000 });
const captures = Array.from({ length: 4 }, () => page.screenshot({ timeout: 5000 }).then(
  value => ({ value }), error => ({ error: { code: error.code, message: error.message } }),
));
const clock = new Int32Array(workerData.clock);
const deadline = Date.now() + 10000;
while (browser.pwrightFfiMetrics().queuedCommands !== 4) {
  assert.ok(Date.now() < deadline, 'four native screenshot results did not reach the settlement queue');
  // Keep the Node event loop blocked: native completion cannot deliver Buffers yet.
  Atomics.wait(clock, 0, 0, 5);
}
const ffi = browser.pwrightFfiMetrics();
assert.equal(ffi.commands, 4);
assert.ok(ffi.reservedBytes >= 4 * (2 * 2 * 1024 * 1024 + 4096));
assert.ok(ffi.reservedBytes <= 64 * 1024 * 1024);
assert.equal(browser.pwrightMetrics().activeOperations, 0);
parentPort.postMessage({ stage: 'queued', ffi });
Atomics.wait(clock, 0, 0, 30000);
const settled = await Promise.all(captures);
for (const result of settled) assert.ok(Buffer.isBuffer(result.value), JSON.stringify(result.error));
await context.close();
await browser.close();
assert.equal(browser.pwrightMetrics().activeLeases, 0);
for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
assert.deepEqual(browser.pwrightFfiMetrics(), zeroFfiMetrics);
// Install the lifetime fence before posting. The parent terminates this Worker
// while its original Buffers remain in `settled`; messages are structured copies.
const lifetime = once(parentPort, 'message');
parentPort.postMessage({ stage: 'published', buffers: settled.map(result => result.value) });
await lifetime;
