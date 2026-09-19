import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';
import { findFixturePage } from './fixture-selection.mjs';
import { queueReadsWithoutDelivering, blockUntilResultsAreQueued } from './ffi-pressure.mjs';

const browser = await chromium.connectOverCDP(workerData.endpoint);
const page = await findFixturePage(browser.contexts().flatMap(context => context.pages()), workerData.fixtureUrl);
const outcomes = queueReadsWithoutDelivering(page, browser);
const metrics = blockUntilResultsAreQueued(browser);
parentPort.postMessage(metrics);
// The parent terminates this environment with known queued Rust results.
Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 30_000);
await Promise.all(outcomes);
await browser.close();
