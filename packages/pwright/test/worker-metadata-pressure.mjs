import assert from 'node:assert/strict';
import { once } from 'node:events';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';
import { findFixturePage } from './fixture-selection.mjs';

const browser = await chromium.connectOverCDP(workerData.endpoint);
const page = await findFixturePage(browser.contexts().flatMap(context => context.pages()), workerData.fixtureUrl);
const before = page.url();
assert.equal(page.mainFrame().pwrightMetadataMetrics().unpublishedUrl, false);
const state = new Int32Array(workerData.signal);
parentPort.postMessage('ready');
await once(parentPort, 'message');
Atomics.store(state, 0, 1);
Atomics.notify(state, 0);
Atomics.wait(state, 0, 1, 10_000);
assert.equal(Atomics.load(state, 0), 2, 'owner must complete external navigation');
const deadline = Date.now() + 5000;
while (!page.mainFrame().pwrightMetadataMetrics().unpublishedUrl) {
  assert.ok(Date.now() < deadline, 'native metadata did not reach the blocked Worker');
  Atomics.wait(state, 0, 2, 10);
}
assert.equal(page.url(), before, 'native metadata cannot publish on a blocked JS loop');
parentPort.postMessage(browser.pwrightMetadataMetrics());
// Parent terminates this environment with the native update still unpublished.
Atomics.wait(state, 0, 2, 30_000);
await browser.close();
