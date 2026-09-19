import assert from 'node:assert/strict';
import { parentPort, workerData } from 'node:worker_threads';
import { setImmediate } from 'node:timers/promises';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.url);
const pending = page.locator('#input').press('Shift+Z', { delay: 30000, timeout: 40000 }).catch(error => error);
const deadline = Date.now() + 5000;
while (!await page.evaluate(() => events.some(event => event.type === 'keydown' && event.key === 'Z'))) {
  assert.ok(Date.now() < deadline, 'terminate only after the native key has gone down');
  await setImmediate();
}
parentPort.postMessage({ activeOperations: browser.pwrightMetrics().activeOperations });
await pending;
await browser.close();
