import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

// Intentionally keep a live session until the owning Node environment ends.
const browser = await chromium.connectOverCDP(workerData);
parentPort.postMessage({ connected: browser.isConnected() });
parentPort.on('message', () => browser.isConnected());
