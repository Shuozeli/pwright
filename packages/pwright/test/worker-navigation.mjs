import { workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';
import { findFixturePage } from './fixture-selection.mjs';

const browser = await chromium.connectOverCDP(workerData.endpoint);
const pages = browser.contexts().flatMap(context => context.pages());
const page = await findFixturePage(pages, workerData.fixtureUrl);
// The owner will destroy this environment after its HTTP request arrives.
await page.goto(workerData.url, { timeout: 30_000 });
