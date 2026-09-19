import { workerData, parentPort } from 'node:worker_threads';
import { createRequire } from 'node:module';
import { once } from 'node:events';
import { findFixturePage } from './fixture-selection.mjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
const { chromium } = oracleRequire('playwright');
const signal = new Int32Array(workerData.signal);
const browser = await chromium.connectOverCDP(workerData.endpoint);
try {
  const page = await findFixturePage(browser.contexts().flatMap(context => context.pages()), workerData.fixtureUrl);
  parentPort.postMessage('ready');
  await once(parentPort, 'message');
  await page.evaluate(() => { history.replaceState(null, '', '#blocked-publication'); });
  Atomics.store(signal, 0, 1);
  Atomics.notify(signal, 0);
} catch (error) {
  Atomics.store(signal, 0, -1);
  Atomics.notify(signal, 0);
  throw error;
} finally { await browser.close(); }
