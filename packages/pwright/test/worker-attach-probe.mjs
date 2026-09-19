import { parentPort, workerData } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const started = performance.now();
let browser;
const report = { mode: 'worker', timeoutMs: 5000 };
try {
  browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
  report.attachMs = performance.now() - started;
  report.pages = browser.contexts().reduce((count, context) => count + context.pages().length, 0);
} catch (error) {
  report.error = { name: error.name, code: error.code, message: error.message.slice(0, 4096) };
} finally {
  try { await browser?.close(); }
  catch (error) { report.cleanupError = String(error).slice(0, 4096); }
}
report.elapsedMs = performance.now() - started;
parentPort.postMessage(report);
