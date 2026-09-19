import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
for (const [target, event] of [[page, 'close'], [context, 'close'], [browser, 'disconnected']]) {
  target.on(event, () => { throw new Error('environment teardown must not dispatch JS listeners'); });
}
void page.close().catch(error => { throw error; });
parentPort.postMessage('ready');
// Native receipt work runs independently; the owner terminates this Worker
// while the relay holds retirement, without permitting Node event delivery.
Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 12000);
throw new Error('native confirmation Worker exceeded its observation deadline');
