// Diagnostic only. Does not retry, modify the browser profile or count as parity.
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { monitorEventLoopDelay } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222', 'run the dedicated Chrome/Pwright preflight');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = requireOracle('playwright');
const reports = [];

async function mainAttach(mode) {
  const started = performance.now();
  let browser;
  const report = { mode, timeoutMs: 5000 };
  try {
    browser = await candidate.connectOverCDP(endpoint, { timeout: 5000 });
    report.attachMs = performance.now() - started;
    report.pages = browser.contexts().reduce((count, context) => count + context.pages().length, 0);
  } catch (error) {
    report.error = { name: error.name, code: error.code, message: error.message.slice(0, 4096) };
  } finally {
    try { await diagnosticDeadline(browser?.close(), 'candidate cleanup'); }
    catch (error) { report.cleanupError = String(error).slice(0, 4096); }
  }
  report.elapsedMs = performance.now() - started;
  reports.push(report);
  console.log(JSON.stringify(report));
}

await mainAttach('main-without-oracle');
const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
try {
  const cdp = await diagnosticDeadline(observer.newBrowserCDPSession(), 'browser CDP session');
  try {
    const delay = monitorEventLoopDelay({ resolution: 10 });
    const initial = await diagnosticDeadline(cdp.send('SystemInfo.getProcessInfo'), 'initial process sample');
    const cpu = process.cpuUsage();
    const started = performance.now();
    const utilization = performance.eventLoopUtilization();
    delay.enable();
    await mainAttach('main-with-oracle');
    delay.disable();
    const final = await diagnosticDeadline(cdp.send('SystemInfo.getProcessInfo'), 'final process sample');
    assert.ok(initial.processInfo.length <= 128 && final.processInfo.length <= 128, 'diagnostic process capacity exceeded');
    const report = {
      mode: 'oracle-overlap-resource-sample', elapsedMs: performance.now() - started,
      nodeCpuMicros: process.cpuUsage(cpu), eventLoop: performance.eventLoopUtilization(utilization),
      eventLoopDelayMaxMs: delay.max / 1e6,
      chromeCpuSeconds: final.processInfo.map(process => ({
        id: process.id, type: process.type,
        delta: process.cpuTime - (initial.processInfo.find(before => before.id === process.id)?.cpuTime ?? process.cpuTime),
      })),
    };
    console.log(JSON.stringify(report));
  } finally { await diagnosticDeadline(cdp.detach(), 'CDP detach'); }
  const worker = new Worker(new URL('../test/worker-attach-probe.mjs', import.meta.url), { workerData: { endpoint } });
  try {
    const [report] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
    reports.push(report);
    console.log(JSON.stringify(report));
  } finally { await diagnosticDeadline(worker.terminate(), 'Worker termination'); }
  const options = {
    cwd: fileURLToPath(new URL('../../../', import.meta.url)),
    env: { ...process.env, CARGO_INCREMENTAL: '0' },
    timeout: 20000, maxBuffer: 256 * 1024, killSignal: 'SIGKILL',
  };
  try {
    const { stdout, stderr } = await promisify(execFile)('cargo', ['test', '--release', '-p', 'pwright-node-host',
      '--test', 'attach_probe', '--', '--ignored', '--nocapture'], options);
    process.stdout.write(stdout);
    process.stderr.write(stderr);
  } catch (error) {
    if (typeof error.stdout === 'string') process.stdout.write(error.stdout);
    if (typeof error.stderr === 'string') process.stderr.write(error.stderr);
    reports.push({ mode: 'safe-host-with-oracle', error: String(error).slice(0, 4096) });
  }
} finally { await diagnosticDeadline(observer.close(), 'oracle cleanup'); }
assert.ok(reports.every(report => !report.error && !report.cleanupError), 'diagnostic attachment failed; inspect each report');
