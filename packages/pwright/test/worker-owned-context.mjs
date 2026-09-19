import { workerData, parentPort } from 'node:worker_threads';
import { once } from 'node:events';
import { chromium } from '@shuozeli/pwright';

const browser = await chromium.connectOverCDP(workerData.endpoint);
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(workerData.url);
parentPort.postMessage('ready');
// The owner terminates this environment while native Context/Page state lives.
await once(parentPort, 'message');
await browser.close();
