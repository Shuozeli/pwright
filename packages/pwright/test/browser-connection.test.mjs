import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const pause = new Int32Array(new SharedArrayBuffer(4));

function nativeRetiredWhileNodeBlocked(browser) {
  if (!browser.pwrightMetrics) { Atomics.wait(pause, 0, 0, 100); return; }
  // Diagnostic native state, not the public getter under test. Require actual
  // host retirement while no JS publication continuation can run.
  for (let attempt = 0; attempt < 500; attempt++) {
    const metrics = browser.pwrightMetrics();
    if (!metrics.connected && metrics.topologyDrivers === 0 && metrics.activeLeases === 0) return;
    Atomics.wait(pause, 0, 0, 10);
  }
  assert.fail('native host did not retire within the bounded blocked-Node probe');
}

function observe(browser, page) {
  const events = [];
  browser.on('disconnected', value => events.push({ identity: value === browser,
    connected: browser.isConnected(), pageClosed: page.isClosed() }));
  return () => ({ connected: browser.isConnected(), pageClosed: page.isClosed(), events: [...events] });
}

async function disconnect(browser) {
  if (!browser) return;
  await diagnosticDeadline(browser.close(), 'Browser-state cleanup');
  if (browser.pwrightMetrics) {
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
    for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
    assert.equal(browser.pwrightFfiMetrics().pending, 0);
    assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
  }
}

async function explicit(type, mode) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    const sample = observe(browser, page);
    // Act
    const pending = (mode === 'context' ? context.close()
      : mode === 'concurrent' ? Promise.all([browser.close(), browser.close()]) : browser.close())
      .then(() => ({ ok: true }), error => ({ error }));
    if (mode === 'context') Atomics.wait(pause, 0, 0, 100);
    else nativeRetiredWhileNodeBlocked(browser);
    const blocked = sample();
    const result = await pending;
    if (result.error) throw result.error;
    const after = sample();
    if (mode !== 'context') await browser.close();
    return { blocked, after, repeated: sample() };
  } finally {
    try { if (browser.isConnected()) await context?.close(); }
    finally { await disconnect(browser); }
  }
}

async function transport(type, mode) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let context, browser, worker;
  const marker = `about:blank#node100-${mode}`;
  try {
    context = await controller.newContext();
    const nativePage = await context.newPage();
    await nativePage.goto(marker);
    const buffer = new SharedArrayBuffer(4);
    const state = new Int32Array(buffer);
    worker = new Worker(new URL('./browser-connection-worker.mjs', import.meta.url), {
      workerData: { endpoint, state: buffer },
    });
    const exit = once(worker, 'exit').then(([code]) => ({ code }), error => ({ error }));
    const [ready] = await diagnosticDeadline(once(worker, 'message'), 'transport controller ready');
    browser = await type.connectOverCDP(ready.url, { timeout: 5000 });
    const pages = browser.contexts().flatMap(value => value.pages()).filter(page => page.url() === marker);
    assert.equal(pages.length, 1);
    const page = pages[0];
    const sample = observe(browser, page);
    const disconnected = mode === 'drop' ? once(browser, 'disconnected') : undefined;
    // Act: the worker progresses the actual transport while this Node thread
    // cannot deliver promise continuations or emit the disconnected event.
    worker.postMessage(mode);
    Atomics.wait(state, 0, 0, 10000);
    assert.equal(Atomics.load(state, 0), 1, 'independent transport controller completed');
    if (mode === 'drop') nativeRetiredWhileNodeBlocked(browser);
    const blocked = sample();
    if (disconnected) await diagnosticDeadline(disconnected, 'public disconnect event');
    else assert.equal(await page.evaluate(() => 42), 42, 'live relay still carries native commands');
    const after = sample();
    const nativeAlive = await nativePage.evaluate(() => 42);
    await disconnect(browser);
    worker.postMessage('finish');
    const finished = await diagnosticDeadline(exit, 'transport controller exit');
    if (finished.error) throw finished.error;
    assert.equal(finished.code, 0);
    return { blocked, after, nativeAlive };
  } finally {
    try { await disconnect(browser); }
    finally {
      try { await worker?.terminate(); }
      finally { try { await context?.close(); } finally { await controller.close(); } }
    }
  }
}

const open = { connected: true, pageClosed: false, events: [] };
const closed = { connected: false, pageClosed: true,
  events: [{ identity: true, connected: false, pageClosed: true }] };
const cases = [
  ...['close', 'concurrent', 'context'].map(mode => {
    const after = mode === 'context' ? { ...open, pageClosed: true } : closed;
    return { name: `pending ${mode}`, run: type => explicit(type, mode), expected: { blocked: open, after, repeated: after } };
  }),
  ...['drop', 'keep'].map(mode => ({ name: `worker transport ${mode}`, run: type => transport(type, mode),
    expected: { blocked: open, after: mode === 'drop' ? closed : open, nativeAlive: 42 } })),
];
for (const entry of cases) test(`Browser.isConnected publication: ${entry.name}`, { timeout: 30000 }, async () => {
  // Arrange
  const expected = await entry.run(oracle);
  assert.deepEqual(expected, entry.expected);
  // Act
  const actual = await entry.run(candidate);
  // Assert
  process.stdout.write(`# Browser publication ${JSON.stringify({ case: entry.name, expected, actual })}\n`);
  assert.deepEqual(actual, expected);
});
