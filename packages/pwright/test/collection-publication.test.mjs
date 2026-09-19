import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate, setTimeout } from 'node:timers/promises';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const referenceOnly = process.env.PWRIGHT_COLLECTION_REFERENCE === '1';
assert.ok(process.env.PWRIGHT_COLLECTION_REFERENCE === undefined || referenceOnly);
const pause = new Int32Array(new SharedArrayBuffer(4));

function observe(browser, context, pages) {
  // Retain copies and object identities; ignore other tasks' borrowed Pages.
  const contextsBefore = browser.contexts();
  const pagesBefore = context.pages();
  assert.ok(contextsBefore.includes(context));
  assert.ok(pages.every(page => pagesBefore.includes(page)));
  const orderedPages = values => values.filter(page => pages.includes(page)).map(page => pages.indexOf(page));
  const orderBefore = orderedPages(pagesBefore);
  const selectPages = values => orderedPages(values).sort((left, right) => left - right);
  return () => {
    const contexts = browser.contexts();
    const currentPages = context.pages();
    return {
      connected: browser.isConnected(),
      registered: contexts.includes(context),
      pages: selectPages(currentPages),
      closed: pages.map(page => page.isClosed()),
      retainedContext: contextsBefore.includes(context),
      retainedPages: selectPages(pagesBefore),
      freshArrays: contexts !== contextsBefore && currentPages !== pagesBefore,
      stableOrder: JSON.stringify(orderedPages(currentPages)) === JSON.stringify(orderBefore.filter(id => currentPages.includes(pages[id]))),
      retainedOrder: JSON.stringify(orderedPages(pagesBefore)) === JSON.stringify(orderBefore),
    };
  };
}

function awaitNativeRetirement(browser, mode, framesBefore) {
  if (!browser.pwrightMetrics) { Atomics.wait(pause, 0, 0, 100); return; }
  // Diagnostic native state only. No language promise or event can progress in
  // this synchronous loop, and no public collection under test is polled here.
  for (let attempt = 0; attempt < 500; attempt++) {
    if (mode === 'page' || mode === 'context') {
      if (browser.pwrightMetadataMetrics().activeFrames < framesBefore) return;
    } else {
      const metrics = browser.pwrightMetrics();
      if (!metrics.connected && metrics.topologyDrivers === 0 && metrics.activeLeases === 0) return;
    }
    Atomics.wait(pause, 0, 0, 10);
  }
  assert.fail('native collection retirement was not observed within 5 seconds');
}

async function disconnect(browser) {
  if (!browser) return;
  await diagnosticDeadline(browser.close(), 'collection test disconnect');
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
    const pages = [await context.newPage(), await context.newPage()];
    const sample = observe(browser, context, pages);
    const initial = sample();
    const frames = browser.pwrightMetadataMetrics?.().activeFrames;
    // Act
    const pending = (mode === 'page' ? pages[0].close()
      : mode === 'context' ? context.close() : browser.close())
      .then(() => ({ ok: true }), error => ({ error }));
    awaitNativeRetirement(browser, mode, frames);
    const blocked = sample();
    const result = await pending;
    if (result.error) throw result.error;
    return { initial, blocked, after: sample() };
  } finally {
    try { if (browser.isConnected()) await context?.close(); }
    finally { await disconnect(browser); }
  }
}

async function externalClose(browser, page, marker, sample) {
  const buffer = new SharedArrayBuffer(4);
  const state = new Int32Array(buffer);
  const worker = new Worker(new URL('./page-is-closed-worker.mjs', import.meta.url), {
    workerData: { endpoint, marker, state: buffer },
  });
  const exit = once(worker, 'exit').then(([code]) => ({ code }), error => ({ error }));
  try {
    const [ready] = await diagnosticDeadline(once(worker, 'message'), 'Page controller ready');
    assert.equal(ready, 'ready');
    const frames = browser.pwrightMetadataMetrics?.().activeFrames;
    // Act
    Atomics.store(state, 0, 1);
    Atomics.notify(state, 0);
    Atomics.wait(state, 0, 1, 10000);
    assert.equal(Atomics.load(state, 0), 2, 'Worker confirmed native Page closure');
    awaitNativeRetirement(browser, 'page', frames);
    const blocked = sample();
    await diagnosticDeadline((async () => {
      for (let attempt = 0; attempt < 1000 && !page.isClosed(); attempt++) await setTimeout(5);
      assert.equal(page.isClosed(), true);
    })(), 'Page close publication');
    const finished = await diagnosticDeadline(exit, 'Page controller exit');
    if (finished.error) throw finished.error;
    assert.equal(finished.code, 0);
    return { blocked, after: sample() };
  } finally { await worker.terminate(); }
}

async function borrowed(type, mode) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let owner, browser, worker;
  const markers = [0, 1].map(value => `about:blank#node101-${mode}-${value}`);
  try {
    owner = await controller.newContext();
    const nativePages = [await owner.newPage(), await owner.newPage()];
    for (let i = 0; i < nativePages.length; i++) await nativePages[i].goto(markers[i]);
    let address = endpoint, state, exit;
    if (mode === 'drop' || mode === 'keep') {
      const buffer = new SharedArrayBuffer(4);
      state = new Int32Array(buffer);
      worker = new Worker(new URL('./browser-connection-worker.mjs', import.meta.url), {
        workerData: { endpoint, state: buffer },
      });
      exit = once(worker, 'exit').then(([code]) => ({ code }), error => ({ error }));
      const [ready] = await diagnosticDeadline(once(worker, 'message'), 'transport controller ready');
      address = ready.url;
    }
    browser = await type.connectOverCDP(address, { timeout: 5000 });
    const allPages = browser.contexts().flatMap(context => context.pages());
    const pages = markers.map(marker => {
      const matching = allPages.filter(page => page.url() === marker);
      assert.equal(matching.length, 1);
      return matching[0];
    });
    const context = pages[0].context();
    assert.equal(pages[1].context(), context);
    const sample = observe(browser, context, pages);
    const initial = sample();
    if (mode === 'external-page') {
      const result = await externalClose(browser, pages[0], markers[0], sample);
      assert.equal(await nativePages[1].evaluate(() => 42), 42);
      return { initial, ...result };
    }
    const disconnected = mode === 'drop' ? once(browser, 'disconnected') : undefined;
    // Act
    let pending;
    if (worker) {
      worker.postMessage(mode);
      Atomics.wait(state, 0, 0, 10000);
      assert.equal(Atomics.load(state, 0), 1, 'independent transport control completed');
    } else pending = browser.close().then(() => ({ ok: true }), error => ({ error }));
    if (mode !== 'keep') awaitNativeRetirement(browser, 'browser');
    const blocked = sample();
    if (disconnected) await diagnosticDeadline(disconnected, 'disconnected delivery');
    if (pending) { const result = await pending; if (result.error) throw result.error; }
    if (mode === 'keep') assert.equal(await pages[0].evaluate(() => 42), 42);
    const after = sample();
    for (const page of nativePages) assert.equal(await page.evaluate(() => 42), 42);
    await disconnect(browser);
    if (worker) {
      worker.postMessage('finish');
      const finished = await diagnosticDeadline(exit, 'transport controller exit');
      if (finished.error) throw finished.error;
      assert.equal(finished.code, 0);
    }
    return { initial, blocked, after };
  } finally {
    try { await disconnect(browser); }
    finally {
      try { await worker?.terminate(); }
      finally { try { await owner?.close(); } finally { await controller.close(); } }
    }
  }
}

const initial = { connected: true, registered: true, pages: [0, 1], closed: [false, false],
  retainedContext: true, retainedPages: [0, 1], freshArrays: true, stableOrder: true, retainedOrder: true };
const pageClosed = { ...initial, pages: [1], closed: [true, false] };
const contextClosed = { ...initial, registered: false, pages: [], closed: [true, true] };
const browserClosed = { ...contextClosed, connected: false };
const cases = [
  ...['page', 'context', 'browser'].map(mode => ({ name: `owned ${mode} close`,
    run: type => owned(type, mode), after: mode === 'page' ? pageClosed : mode === 'context' ? contextClosed : browserClosed })),
  ...['disconnect', 'drop', 'keep', 'external-page'].map(mode => ({ name: `borrowed ${mode}`,
    run: type => borrowed(type, mode), after: mode === 'keep' ? initial : mode === 'external-page' ? pageClosed : browserClosed })),
];
for (const entry of cases) test(`${referenceOnly ? 'reference only' : 'paired'} collection publication: ${entry.name}`,
  { timeout: 30000 }, async () => {
    // Arrange
    const expected = await entry.run(oracle);
    process.stdout.write(`# Collection reference ${JSON.stringify({ case: entry.name, value: expected })}\n`);
    assert.deepEqual(expected, { initial, blocked: initial, after: entry.after });
    if (referenceOnly) return;
    // Act
    const actual = await entry.run(candidate);
    // Assert
    process.stdout.write(`# Collection candidate ${JSON.stringify({ case: entry.name, value: actual })}\n`);
    assert.deepEqual(actual, expected);
  });
