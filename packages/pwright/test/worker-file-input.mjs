import assert from 'node:assert/strict';
import { once } from 'node:events';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const { endpoint, fixtureUrl, phase } = workerData;
assert.ok(['resolving', 'queued'].includes(phase));
const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(fixtureUrl, { timeout: 5000 });
await page.evaluate(() => {
  const input = document.createElement('input');
  input.type = 'file'; input.id = 'file'; document.body.append(input);
});
const begin = once(parentPort, 'message');
parentPort.postMessage({ phase: 'ready' });
assert.equal((await begin)[0], 'upload');
const outcome = page.locator(phase === 'queued' ? '#file' : '#missing').setInputFiles({
  name: 'worker.bin', mimeType: 'application/octet-stream', buffer: Buffer.alloc(1024 * 1024, 73),
}, { timeout: 30000 }).then(() => ({ status: 'fulfilled' }), error => ({ error: error.message }));
if (phase === 'queued') {
  // Test-only synchronous barrier: native upload completes while JS cannot
  // consume its Promise result. No production polling or private access.
  const clock = new Int32Array(new SharedArrayBuffer(4));
  const limit = Date.now() + 10000;
  while (browser.pwrightFfiMetrics().queuedCommands !== 1) {
    assert.ok(Date.now() < limit, 'upload result did not reach the settlement queue');
    Atomics.wait(clock, 0, 0, 5);
  }
  parentPort.postMessage({ phase, ffi: browser.pwrightFfiMetrics() });
  Atomics.wait(clock, 0, 0, 30000);
} else {
  parentPort.postMessage({ phase, ffi: browser.pwrightFfiMetrics() });
}
parentPort.postMessage({ phase: 'settled', result: await outcome });
await context.close();
await browser.close();
