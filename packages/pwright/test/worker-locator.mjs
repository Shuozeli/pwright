import assert from 'node:assert/strict';
import { parentPort, workerData } from 'node:worker_threads';
import { setImmediate } from 'node:timers/promises';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.url);
const locator = page.locator('#missing');
const pending = locator.textContent({ timeout: 30000 }).catch(error => error);
const deadline = Date.now() + 5000;
while (!browser.pwrightMetrics().activeOperations) {
  assert.ok(Date.now() < deadline, 'the Locator command must start before termination');
  await setImmediate();
}
parentPort.postMessage({ ...browser.pwrightLocatorMetrics(), activeOperations: browser.pwrightMetrics().activeOperations });
await pending;
await browser.close();
