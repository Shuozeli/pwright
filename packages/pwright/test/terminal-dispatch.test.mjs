import assert from 'node:assert/strict';
import { test } from 'node:test';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers/promises';
import { Worker } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const oracle = require('playwright').chromium;

for (const queued of [false, true]) test(`environment teardown drops terminal callbacks: queued=${queued}`, { timeout: 25000 }, async () => {
  // Arrange
  const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  const marker = `about:blank#node115-worker-${process.pid}-${queued}`;
  const cdp = await observer.newBrowserCDPSession();
  const before = (await cdp.send('Target.getTargets')).targetInfos.filter(value => value.type === 'page').map(value => value.targetId);
  const worker = new Worker(new URL('./terminal-dispatch-worker.mjs', import.meta.url), { workerData: { endpoint, marker, queued } });
  try {
    const [ready] = await diagnosticDeadline(once(worker, 'message'), 'terminal Worker ready');
    assert.equal(ready, 'ready');
    const owned = (await cdp.send('Target.getTargets')).targetInfos.filter(value => value.url === marker);
    assert.equal(owned.length, queued ? 0 : 1);
    // Act
    await diagnosticDeadline(worker.terminate(), 'terminal Worker environment cleanup');
    // Assert
    const targets = (await cdp.send('Target.getTargets')).targetInfos;
    assert.equal(targets.some(value => value.url === marker), false);
    for (const id of before) assert.ok(targets.some(value => value.targetId === id), 'external Page survives cleanup');
    if (owned.length) assert.equal((await cdp.send('Target.getBrowserContexts')).browserContextIds.includes(owned[0].browserContextId), false);
  } finally {
    await worker.terminate();
    await cdp.detach();
    await observer.close();
  }
});

test('retired dispatch ports release active slots while retained Browsers remain valid', { timeout: 30000 }, async () => {
  // Arrange
  const retained = [];
  let bytes;
  try {
    // Act: exceed the sixteen concurrent-port bound using retired wrappers.
    for (let index = 0; index < 20; index++) {
      const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
      retained.push(browser);
      let notifications = 0;
      browser.on('disconnected', () => notifications++);
      await browser.close();
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      const metadata = browser.pwrightFrameTreeMetrics();
      bytes ??= metadata.bytes;
      // Assert each retirement before the next admission; no cumulative journal.
      assert.equal(notifications, 1);
      assert.equal(metadata.revisions, 1, 'only the retained fixed port body remains');
      assert.equal(metadata.bytes, bytes);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetadataMetrics().pendingPublications, 0);
    }
    assert.equal(retained.length, 20);
    assert.ok(retained.every(browser => !browser.isConnected() && browser.contexts().length === 0));
  } finally {
    for (const browser of retained) await browser.close();
  }
});

async function transportCut(type) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let context, browser, worker;
  try {
    context = await controller.newContext();
    const original = await context.newPage();
    const marker = `about:blank#node115-transport-${process.pid}`;
    await original.goto(marker);
    worker = new Worker(new URL('./browser-connection-worker.mjs', import.meta.url), {
      workerData: { endpoint, state: new SharedArrayBuffer(4) },
    });
    const exit = once(worker, 'exit');
    const [ready] = await diagnosticDeadline(once(worker, 'message'), 'terminal relay ready');
    browser = await type.connectOverCDP(ready.url, { timeout: 5000 });
    const page = browser.contexts().flatMap(value => value.pages()).find(value => value.url() === marker);
    assert.ok(page);
    const owner = page.context();
    const trace = [];
    for (const [target, event] of [[page, 'close'], [owner, 'close'], [browser, 'disconnected']]) {
      target.on(event, function(value) {
        trace.push({ event: target === page ? 'page' : target === owner ? 'context' : 'browser',
          argument: value === target, receiver: this === target, connected: browser.isConnected(),
          closed: page.isClosed(), contextListed: browser.contexts().includes(owner), listed: owner.pages().includes(page) });
      });
    }
    const disconnected = once(browser, 'disconnected');
    worker.postMessage('drop');
    await diagnosticDeadline(disconnected, 'terminal transport notifications');
    assert.equal(await original.evaluate(() => 42), 42, 'external browser ownership survives client transport loss');
    worker.postMessage('finish');
    const [code] = await diagnosticDeadline(exit, 'terminal relay shutdown');
    assert.equal(code, 0);
    return trace;
  } finally {
    try { if (browser) await browser.close(); }
    finally {
      try { await worker?.terminate(); }
      finally { try { await context?.close(); } finally { await controller.close(); } }
    }
  }
}

test('transport loss publishes Page and Context state before disconnected', { timeout: 30000 }, async () => {
  // Arrange
  const expected = await transportCut(oracle);
  assert.deepEqual(expected.map(value => value.event), ['page', 'context', 'browser']);
  // Act
  const actual = await transportCut(chromium);
  // Assert
  assert.deepEqual(actual, expected);
});
