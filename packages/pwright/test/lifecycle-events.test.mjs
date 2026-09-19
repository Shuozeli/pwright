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
const referenceOnly = process.env.PWRIGHT_LIFECYCLE_REFERENCE === '1';
assert.ok(process.env.PWRIGHT_LIFECYCLE_REFERENCE === undefined || referenceOnly);
const pause = new Int32Array(new SharedArrayBuffer(4));

async function disconnect(browser) {
  if (!browser) return;
  await diagnosticDeadline(browser.close(), 'event test disconnect');
  if (browser.pwrightMetrics) {
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
    for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
    assert.equal(browser.pwrightFfiMetrics().pending, 0);
    assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
  }
}

function recording(browser, context, pages) {
  const trace = [], remove = [];
  const retained = context.pages(), original = [...retained];
  const snapshot = () => ({ connected: browser.isConnected(),
    contextListed: browser.contexts().includes(context),
    listedPages: pages.filter(page => context.pages().includes(page)).length,
    closedPages: pages.filter(page => page.isClosed()).length,
    retained: retained.length === original.length &&
      retained.every((page, index) => page === original[index]) });
  const record = (kind, sameArgument) => {
    assert.ok(trace.length < 32, 'event trace is bounded');
    trace.push({ kind, sameArgument, ...snapshot() });
  };
  const listen = (target, event, handler) => {
    target.on(event, handler);
    remove.push(() => target.off(event, handler));
  };
  return { trace, snapshot, record, listen, stop: () => remove.forEach(dispose => dispose()) };
}

async function ownedClose(type, kind) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context, log;
  try {
    context = await browser.newContext();
    const pages = [await context.newPage(), await context.newPage()];
    log = recording(browser, context, pages);
    pages.forEach((page, index) => log.listen(page, 'close', argument => {
      process.stdout.write(`# Closed Page ordinal ${JSON.stringify({ side: type === candidate ? 'candidate' : 'oracle', kind, index })}\n`);
      log.record('page-close', argument === page);
    }));
    log.listen(context, 'close', argument => log.record('context-close', argument === context));
    log.listen(browser, 'disconnected', argument => log.record('disconnected', argument === browser));
    const target = kind === 'Page' ? pages[0] : kind === 'Context' ? context : browser;
    // Act
    await target.close();
    log.record('resolved', true);
    const first = structuredClone(log.trace);
    await target.close();
    // Assert: a repeated close must not emit again.
    assert.deepEqual(log.trace, first);
    return { trace: first, final: log.snapshot() };
  } finally {
    log?.stop();
    try { if (browser.isConnected()) await context?.close(); }
    finally { await disconnect(browser); }
  }
}

async function creation(type, popup) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context, log;
  try {
    context = await browser.newContext();
    const anchor = popup ? await context.newPage() : undefined;
    const pages = anchor ? [anchor] : [];
    log = recording(browser, context, pages);
    let announced, popupArgument;
    log.listen(context, 'page', page => {
      announced = page;
      if (!pages.includes(page)) pages.push(page);
      log.record('context-page', page.context() === context && context.pages().includes(page));
    });
    if (anchor) log.listen(anchor, 'popup', page => {
      popupArgument = page;
      log.record('popup', page === announced && page.context() === context);
    });
    let page;
    // Act
    if (popup) {
      const opened = await anchor.evaluate(() => !!window.open('about:blank#node105-popup'));
      assert.equal(opened, true, 'native popup creation succeeds');
      await diagnosticDeadline((async () => {
        for (let attempt = 0; attempt < 1000; attempt++) {
          const found = context.pages().filter(value => value.url() === 'about:blank#node105-popup');
          if (found.length) { assert.equal(found.length, 1); page = found[0]; return; }
          await setTimeout(5);
        }
        assert.fail('popup must be discovered');
      })(), 'popup collection delivery');
    } else page = await context.newPage();
    if (!pages.includes(page)) pages.push(page);
    if (!popup) log.record('resolved', true);
    // Assert
    return { trace: structuredClone(log.trace), final: log.snapshot(),
      samePage: page === announced && (!popup || page === popupArgument),
      frameIdentity: page.mainFrame().page() === page };
  } finally {
    log?.stop();
    try { await context?.close(); }
    finally { await disconnect(browser); }
  }
}

async function borrowedClose(type, external) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let owner, browser, log, worker;
  try {
    owner = await controller.newContext();
    const owned = [await owner.newPage(), await owner.newPage()];
    const markers = [0, 1].map(index => `about:blank#node105-${external ? 'external' : 'disconnect'}-${index}`);
    for (let i = 0; i < owned.length; i++) await owned[i].goto(markers[i]);
    browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
    const all = browser.contexts().flatMap(context => context.pages());
    const pages = markers.map(marker => {
      const matching = all.filter(page => page.url() === marker);
      assert.equal(matching.length, 1);
      return matching[0];
    });
    const context = pages[0].context();
    assert.equal(pages[1].context(), context);
    log = recording(browser, context, pages);
    for (const page of pages) log.listen(page, 'close', argument => log.record('page-close', argument === page));
    log.listen(context, 'close', argument => log.record('context-close', argument === context));
    log.listen(browser, 'disconnected', argument => log.record('disconnected', argument === browser));
    let blocked;
    // Act
    if (external) {
      const buffer = new SharedArrayBuffer(4), state = new Int32Array(buffer);
      worker = new Worker(new URL('./page-is-closed-worker.mjs', import.meta.url), {
        workerData: { endpoint, marker: markers[0], state: buffer },
      });
      const exit = once(worker, 'exit').then(([code]) => ({ code }), error => ({ error }));
      const [ready] = await diagnosticDeadline(once(worker, 'message'), 'native close controller ready');
      assert.equal(ready, 'ready');
      const frames = browser.pwrightMetadataMetrics?.().activeFrames;
      Atomics.store(state, 0, 1);
      Atomics.notify(state, 0);
      Atomics.wait(state, 0, 1, 10000);
      assert.equal(Atomics.load(state, 0), 2, 'Worker confirms actual native closure');
      if (browser.pwrightMetadataMetrics) {
        for (let i = 0; i < 500 && browser.pwrightMetadataMetrics().activeFrames >= frames; i++)
          Atomics.wait(pause, 0, 0, 10);
        assert.ok(browser.pwrightMetadataMetrics().activeFrames < frames);
      } else Atomics.wait(pause, 0, 0, 100);
      blocked = { events: log.trace.length, ...log.snapshot() };
      await diagnosticDeadline((async () => {
        for (let i = 0; i < 1000 && !pages[0].isClosed(); i++) await setTimeout(5);
        assert.equal(pages[0].isClosed(), true);
      })(), 'closed Page publication');
      const finished = await diagnosticDeadline(exit, 'native close controller exit');
      if (finished.error) throw finished.error;
      assert.equal(finished.code, 0);
    } else {
      await browser.close();
      log.record('resolved', true);
      for (const page of owned) assert.equal(await page.evaluate(() => 42), 42);
    }
    assert.equal(await owned[1].evaluate(() => 42), 42, 'surviving native sibling remains usable');
    return { trace: structuredClone(log.trace), final: log.snapshot(), ...(external ? { blocked } : {}) };
  } finally {
    log?.stop();
    try { await disconnect(browser); }
    finally {
      try { await worker?.terminate(); }
      finally { try { await owner?.close(); } finally { await controller.close(); } }
    }
  }
}

const cases = [
  { name: 'explicit newPage', run: type => creation(type, false), events: ['context-page', 'resolved'] },
  ...['Page', 'Context', 'Browser'].map(kind => ({ name: `owned ${kind} close`,
    run: type => ownedClose(type, kind), events: kind === 'Page' ? ['page-close', 'resolved']
      : ['page-close', 'page-close', 'context-close', ...(kind === 'Browser' ? ['disconnected'] : []), 'resolved'] })),
  { name: 'borrowed Browser disconnect', run: type => borrowedClose(type, false),
    events: ['page-close', 'page-close', 'context-close', 'disconnected', 'resolved'] },
  { name: 'Worker-confirmed external Page close', run: type => borrowedClose(type, true), events: ['page-close'] },
  { name: 'popup creation', run: type => creation(type, true), events: ['context-page', 'popup'] },
];
for (const entry of cases) test(`${referenceOnly ? 'reference only' : 'paired'} lifecycle events: ${entry.name}`,
  { timeout: 30000 }, async () => {
    // Arrange
    const expected = await entry.run(oracle);
    process.stdout.write(`# Lifecycle event reference ${JSON.stringify({ case: entry.name, value: expected })}\n`);
    assert.deepEqual(expected.trace.map(value => value.kind), entry.events);
    assert.ok(expected.trace.every(value => value.sameArgument));
    let closedPages = 0, contextListed = true, connected = true;
    const total = entry.name === 'explicit newPage' ? 1 : 2;
    for (const observation of expected.trace) {
      if (observation.kind === 'page-close') closedPages++;
      if (observation.kind === 'context-close') contextListed = false;
      if (observation.kind === 'disconnected') connected = false;
      assert.deepEqual(observation, { kind: observation.kind, sameArgument: true,
        connected, contextListed, listedPages: total - closedPages, closedPages, retained: true });
    }
    assert.deepEqual(expected.final, { connected, contextListed,
      listedPages: total - closedPages, closedPages, retained: true });
    if ('samePage' in expected) assert.equal(expected.samePage, true);
    if ('frameIdentity' in expected) assert.equal(expected.frameIdentity, true);
    if ('blocked' in expected) assert.deepEqual(expected.blocked, { events: 0,
      connected: true, contextListed: true, listedPages: 2, closedPages: 0, retained: true });
    if (referenceOnly) return;
    // Act
    const actual = await entry.run(candidate);
    // Assert
    process.stdout.write(`# Lifecycle event candidate ${JSON.stringify({ case: entry.name, value: actual })}\n`);
    assert.deepEqual(actual, expected);
  });
