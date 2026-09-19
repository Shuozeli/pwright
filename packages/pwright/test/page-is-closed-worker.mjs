import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { parentPort, workerData } from 'node:worker_threads';

// Test controller only. The subject's JS event loop is deliberately blocked;
// this separate worker confirms actual native closure, not elapsed wall time.
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium } = require('playwright');
assert.equal(workerData.endpoint, 'http://127.0.0.1:9222');
const state = new Int32Array(workerData.state);
let browser;
try {
  browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
  const pages = browser.contexts().flatMap(context => context.pages())
    .filter(page => page.url() === workerData.marker);
  assert.equal(pages.length, 1, 'exactly one controller-owned Page');
  parentPort.postMessage('ready');
  Atomics.wait(state, 0, 0, 10000);
  assert.equal(Atomics.load(state, 0), 1, 'parent explicitly starts native close');
  await pages[0].close();
  Atomics.store(state, 0, 2);
  Atomics.notify(state, 0);
} catch (error) {
  Atomics.store(state, 0, -1);
  Atomics.notify(state, 0);
  throw error;
} finally {
  await browser?.close();
}
