import assert from 'node:assert/strict';
import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
try {
  let page;
  if (workerData.borrowed) {
    const pages = browser.contexts().flatMap(context => context.pages()).filter(page => page.url() === workerData.url);
    assert.equal(pages.length, 1);
    page = pages[0];
  } else {
    page = await (await browser.newContext()).newPage();
    await page.goto(workerData.url, { timeout: 5000 });
  }
  await page.evaluate(() => window.resetMouse());
  const pending = page.mouse.click(100, 120, { delay: 60000 }).catch(error => error);
  let observed = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    observed = await page.evaluate(() => window.mouseSnapshot().records.some(event => event.type === 'mousedown'));
    if (observed) break;
  }
  assert.equal(observed, true, 'native mousedown must precede Worker termination');
  parentPort.postMessage({ activeOperations: browser.pwrightMetrics().activeOperations, ffiCommands: browser.pwrightFfiMetrics().commands });
  await pending;
} finally { await browser.close(); }
