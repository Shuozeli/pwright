import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate, setTimeout } from 'node:timers/promises';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { withCdpCommandRelay } from './cdp-command-relay.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const referenceOnly = process.env.PWRIGHT_IS_CLOSED_REFERENCE === '1';
assert.ok(process.env.PWRIGHT_IS_CLOSED_REFERENCE === undefined || referenceOnly);

async function eventually(predicate, label) {
  await diagnosticDeadline((async () => {
    for (let attempt = 0; attempt < 1000; attempt++) {
      if (predicate()) return;
      await setTimeout(5);
    }
    assert.fail(label);
  })(), label);
}

async function disconnect(browser) {
  if (!browser) return;
  await diagnosticDeadline(browser.close(), 'Page-state test Browser disconnect');
  if (browser.pwrightMetrics) {
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
    for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
    assert.equal(browser.pwrightFfiMetrics().pending, 0);
    assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
  }
}

async function owned(type, mode) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    const root = page.mainFrame();
    const initial = page.isClosed();
    // Act: retain the public Page/Frame references across the selected close.
    const closing = mode === 'page' ? page.close()
      : mode === 'context' ? context.close() : browser.close();
    const immediate = page.isClosed();
    await closing;
    const after = page.isClosed();
    if (mode === 'page') await page.close();
    return { initial, immediate, after, retained: page.isClosed(),
      rootDetached: root.isDetached(), sameRoot: page.mainFrame() === root };
  } finally {
    try { if (browser.isConnected()) await context?.close(); }
    finally { await disconnect(browser); }
  }
}

// No replay/fake events: only disconnect this test's real relay transport.
class TransportGate {
  #client;
  #error;
  command(raw, client, upstream) { this.#client = client; upstream.send(raw, { binary: false }); }
  response(raw, client) { client.send(raw, { binary: false }); }
  terminate() { assert.ok(this.#client); this.#client.terminate(); }
  release() {}
  fail(error) { this.#error ??= error; }
  check() { if (this.#error) throw this.#error; }
}

async function blockedClose(page, marker) {
  const shared = new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT);
  const state = new Int32Array(shared);
  const worker = new Worker(new URL('./page-is-closed-worker.mjs', import.meta.url), {
    workerData: { endpoint, marker, state: shared },
  });
  let workerError;
  worker.on('error', error => { workerError = error; });
  const exit = once(worker, 'exit').then(([code]) => ({ code }), error => ({ error }));
  try {
    const [ready] = await diagnosticDeadline(once(worker, 'message'), 'native-close controller ready');
    assert.equal(ready, 'ready');
    const initial = page.isClosed();
    // Act: give native Rust I/O a confirmed closure while preventing Node from
    // delivering a close event/publication callback in this synchronous turn.
    Atomics.store(state, 0, 1);
    Atomics.notify(state, 0);
    Atomics.wait(state, 0, 1, 5000);
    assert.equal(Atomics.load(state, 0), 2, 'worker confirmed native Page.close completion');
    const beforeNodeDelivery = page.isClosed();
    await eventually(() => page.isClosed(), 'closure published after Node yields');
    const finished = await diagnosticDeadline(exit, 'native-close controller drained');
    if (workerError) throw workerError;
    if (finished.error) throw finished.error;
    assert.equal(finished.code, 0);
    return { initial, beforeNodeDelivery, after: page.isClosed() };
  } finally { await worker.terminate(); }
}

async function borrowed(type, mode) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  const marker = `about:blank#node098-is-closed-${mode}`;
  try {
    context = await controller.newContext();
    const nativePage = await context.newPage();
    await nativePage.goto(marker);
    const exercise = async (address, gate) => {
      let browser;
      try {
        browser = await type.connectOverCDP(address, { timeout: 5000 });
        const pages = browser.contexts().flatMap(item => item.pages()).filter(page => page.url() === marker);
        assert.equal(pages.length, 1, 'subject borrows the exact controller-owned Page');
        const page = pages[0];
        const initial = page.isClosed();
        if (mode === 'blocked') return await blockedClose(page, marker);
        if (mode === 'disconnect' || mode === 'transport') {
          // Act: retire only the borrowing connection, not the native target.
          if (gate) {
            gate.terminate();
            await eventually(() => !browser.isConnected(), 'transport disconnect published');
          } else await browser.close();
          return { initial, after: page.isClosed(), nativeAlive: await nativePage.evaluate(() => 42) };
        }
        if (mode === 'crash') {
          const session = await context.newCDPSession(nativePage);
          const crashed = once(nativePage, 'crash');
          // Act: crash only this fresh Page's renderer, never the browser.
          const crashing = session.send('Page.crash').then(() => ({ ok: true }), error => ({ error: error.message }));
          await diagnosticDeadline(crashed, 'controller observed real renderer crash');
          const afterCrash = page.isClosed();
          await nativePage.close();
          await diagnosticDeadline(crashing, 'crash command settled after native Page closure');
          await eventually(() => page.isClosed(), 'crashed Page subsequently closes');
          return { initial, afterCrash, afterClose: page.isClosed() };
        }
        // Act: a different connection closes the native Page.
        await nativePage.close();
        await eventually(() => page.isClosed(), 'external Page closure published');
        return { initial, after: page.isClosed(), rootDetached: page.mainFrame().isDetached() };
      } finally { await disconnect(browser); }
    };
    return mode === 'transport'
      ? await withCdpCommandRelay(endpoint, new TransportGate(), exercise)
      : await exercise(endpoint);
  } finally {
    try { await context?.close(); } finally { await controller.close(); }
  }
}

const cases = [
  ...['page', 'context', 'browser'].map(mode => ({ name: `${mode} close retains Page state`,
    run: type => owned(type, mode), expected: { initial: false, immediate: false, after: true,
      retained: true, rootDetached: false, sameRoot: true } })),
  ...['disconnect', 'transport'].map(mode => ({ name: `borrowed Page after ${mode}`,
    run: type => borrowed(type, mode), expected: { initial: false, after: true, nativeAlive: 42 } })),
  { name: 'external native Page close', run: type => borrowed(type, 'external'),
    expected: { initial: false, after: true, rootDetached: false } },
  { name: 'renderer crash is not Page close', run: type => borrowed(type, 'crash'),
    expected: { initial: false, afterCrash: false, afterClose: true } },
  { name: 'native close waits for Node publication', run: type => borrowed(type, 'blocked'),
    expected: { initial: false, beforeNodeDelivery: false, after: true } },
];
for (const entry of cases) test(`${referenceOnly ? 'reference only' : 'paired'} Page.isClosed: ${entry.name}`,
  { timeout: 30000 }, async () => {
    // Arrange: characterize the pinned real upstream implementation first.
    const expected = await entry.run(oracle);
    assert.deepEqual(expected, entry.expected);
    process.stdout.write(`# Page.isClosed reference ${JSON.stringify({ case: entry.name, value: expected })}\n`);
    if (referenceOnly) return;
    // Act
    const actual = await entry.run(candidate);
    // Assert
    assert.deepEqual(actual, expected);
  });
