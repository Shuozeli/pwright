import assert from 'node:assert/strict';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

assert.ok(['prepared', 'running', 'queued'].includes(workerData.mode));
const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.url, { timeout: 5000 });
const outcomes = Array.from({ length: 3 }, () => (workerData.mode === 'prepared'
  ? page.evaluate(value => value, new Uint8Array(256 * 1024))
  : page.evaluate(workerData.mode === 'queued' ? () => new Uint8Array(256 * 1024) : () => new Promise(() => {}))
).catch(error => ({ error: error.code })));
const clock = new Int32Array(new SharedArrayBuffer(4));
const deadline = Date.now() + 10000;
while (workerData.mode !== 'prepared' && (workerData.mode === 'queued'
  ? browser.pwrightFfiMetrics().queuedCommands !== 3
  : browser.pwrightMetrics().activeOperations !== 3)) {
  assert.ok(Date.now() < deadline, 'evaluation did not reach the expected native state');
  Atomics.wait(clock, 0, 0, 10);
}
parentPort.postMessage({ ffi: browser.pwrightFfiMetrics(), host: browser.pwrightMetrics(), conversion: browser.pwrightConversionMetrics() });
// Parent terminates this exact environment before any evaluation resolver runs.
Atomics.wait(clock, 0, 0, 30000);
await browser.close();
await Promise.all(outcomes);
