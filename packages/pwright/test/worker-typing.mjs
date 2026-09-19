import assert from 'node:assert/strict';
import { parentPort, workerData } from 'node:worker_threads';
import { setImmediate } from 'node:timers/promises';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.url);
await page.evaluate(() => window.resetTyping('#text'));
const pending = page.locator('#text').pressSequentially('ab', { delay: 30000, timeout: 40000 }).catch(error => error);
const deadline = Date.now() + 5000;
while (!await page.evaluate(() => window.typingSnapshot().events.some(event => event.type === 'keydown' && event.key === 'a'))) {
  assert.ok(Date.now() < deadline, 'terminate only after the native first key');
  await setImmediate();
}
parentPort.postMessage({ activeOperations: browser.pwrightMetrics().activeOperations });
await pending;
await browser.close();
