import assert from 'node:assert/strict';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.marker);
for (const [target, event] of [[page, 'close'], [context, 'close'], [browser, 'disconnected']]) {
  target.on(event, () => { throw new Error('teardown must not invoke JS listeners'); });
}
const pause = new Int32Array(new SharedArrayBuffer(4));
if (workerData.queued) {
  void browser.close();
  for (let attempt = 0; attempt < 500; attempt++) {
    const metrics = browser.pwrightMetrics();
    // Taking Session state closes the native admission gate before teardown
    // has produced its publication cut. Wait for that cut, not just the gate.
    if (!metrics.connected && metrics.activeLeases === 0 && metrics.topologyDrivers === 0
      && browser.pwrightMetadataMetrics().pendingPublications > 0) break;
    Atomics.wait(pause, 0, 0, 10);
  }
  assert.equal(browser.pwrightMetrics().connected, false);
  assert.equal(browser.pwrightMetrics().activeLeases, 0);
  assert.equal(browser.pwrightMetrics().topologyDrivers, 0);
  assert.equal(page.isClosed(), false, 'Node has not delivered the terminal cut');
  assert.ok(browser.pwrightMetadataMetrics().pendingPublications > 0);
}
parentPort.postMessage('ready');
// Only the test owner terminates this environment. Never release JS delivery.
Atomics.wait(pause, 0, 0, 12000);
throw new Error('terminal Worker was not terminated before its bounded deadline');
