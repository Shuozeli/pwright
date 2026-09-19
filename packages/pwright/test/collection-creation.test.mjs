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
const outcome = promise => promise.then(value => ({ value }), error => ({ error }));

function blockedUntil(browser, ready, label) {
  if (!browser.pwrightMetrics) {
    // The reference driver shares this blocked JS thread; this is a language
    // observation, not a claim that its native command has already completed.
    Atomics.wait(pause, 0, 0, 100);
    return;
  }
  for (let attempt = 0; attempt < 500; attempt++) {
    if (ready()) return;
    Atomics.wait(pause, 0, 0, 10);
  }
  assert.fail(`${label}: native readiness was not observed within five seconds; ${JSON.stringify(browser.pwrightFfiMetrics())}`);
}

async function disconnect(browser) {
  if (!browser) return;
  await diagnosticDeadline(browser.close(), 'creation test disconnect');
  if (browser.pwrightMetrics) {
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
    for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
    assert.equal(browser.pwrightFfiMetrics().pending, 0);
    assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
  }
}

async function explicit(type, kind, count, replace = false) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let owner;
  const pending = [];
  const created = [];
  try {
    owner = await browser.newContext();
    const anchor = await owner.newPage();
    const list = () => kind === 'Context' ? browser.contexts() : owner.pages();
    const retired = kind === 'Context' ? owner : anchor;
    const retained = list();
    const originals = [...retained];
    const sample = () => {
      const current = list();
      return { added: current.filter(value => !originals.includes(value)).length,
        oldListed: current.includes(retired), oldPageClosed: anchor.isClosed(),
        fresh: current !== retained, retained: retained.length === originals.length &&
          retained.every((value, index) => value === originals[index]) };
    };
    const initial = sample();
    if (browser.pwrightFfiMetrics) assert.equal(browser.pwrightFfiMetrics().commands, 0);
    // Act: all native requests are issued before the synchronous blocked read.
    if (replace) pending.push(outcome(retired.close()));
    for (let i = 0; i < count; i++) pending.push(outcome(
      kind === 'Context' ? browser.newContext() : owner.newPage()));
    // Close uses its separate reserved lane, not queuedCommands. Wait for the
    // creation results here; NODE-101 independently proves blocked retirement.
    blockedUntil(browser, () => browser.pwrightFfiMetrics().queuedCommands >= count,
      `${kind} creation completions queued`);
    if (browser.pwrightFfiMetrics) process.stdout.write(`# Creation native queue ${JSON.stringify({ kind,
      count, replace, metrics: browser.pwrightFfiMetrics() })}\n`);
    const blocked = sample();
    const results = await Promise.all(pending);
    for (const result of results) if (result.error) throw result.error;
    created.push(...results.slice(replace ? 1 : 0).map(result => result.value));
    const after = sample();
    const current = list();
    const again = list();
    const oldSurvivors = current.filter(value => originals.includes(value));
    const expectedSurvivors = originals.filter(value => current.includes(value));
    // Concurrent native Page initialization need not finish in invocation order.
    // Preserve observed collection order across reads and surviving old objects,
    // but do not invent cross-run ordering for independent concurrent creations.
    const stableOrder = current.length === again.length &&
      current.every((value, index) => value === again[index]) &&
      oldSurvivors.every((value, index) => value === expectedSurvivors[index]);
    process.stdout.write(`# Creation order ${JSON.stringify({ side: type === candidate ? 'candidate' : 'oracle',
      kind, count, replace, invocationIndices: current.filter(value => !originals.includes(value))
        .map(value => created.indexOf(value)) })}\n`);
    const returned = created.every(value => current.includes(value) && (kind === 'Context'
      ? value.browser() === browser && value.pages().length === 0
      : value.context() === owner && value.mainFrame().page() === value && !value.isClosed()));
    return { initial, blocked, after, stableOrder, returned };
  } finally {
    // Pending creations remain owned even if the synchronous assertion fails.
    try {
      const results = await diagnosticDeadline(Promise.all(pending), 'pending creation cleanup');
      if (kind === 'Context') for (const result of results)
        if (result.value) await diagnosticDeadline(result.value.close(), 'created Context cleanup');
      await diagnosticDeadline(owner?.close(), 'anchor Context cleanup');
    } finally { await disconnect(browser); }
  }
}

async function discovered(type) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let owner, browser, worker, exit;
  const anchorUrl = 'about:blank#node104-discovery-anchor';
  const marker = 'about:blank#node104-discovery-created';
  try {
    owner = await controller.newContext();
    const ownedAnchor = await owner.newPage();
    await ownedAnchor.goto(anchorUrl);
    browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
    const anchors = browser.contexts().flatMap(context => context.pages())
      .filter(page => page.url() === anchorUrl);
    assert.equal(anchors.length, 1);
    const anchor = anchors[0], context = anchor.context();
    const retained = context.pages();
    const matching = values => values.filter(page => page.url() === marker);
    const sample = () => {
      const current = context.pages();
      return { added: matching(current).length, oldListed: current.includes(anchor),
        oldPageClosed: anchor.isClosed(), fresh: current !== retained,
        retained: retained.includes(anchor) && matching(retained).length === 0 };
    };
    const buffer = new SharedArrayBuffer(4), state = new Int32Array(buffer);
    worker = new Worker(new URL('./collection-creation-worker.mjs', import.meta.url), {
      workerData: { endpoint, anchor: anchorUrl, marker, state: buffer },
    });
    exit = once(worker, 'exit').then(([code]) => ({ code }), error => ({ error }));
    const [ready] = await diagnosticDeadline(once(worker, 'message'), 'creation controller ready');
    assert.equal(ready, 'ready');
    const initial = sample();
    const frames = browser.pwrightMetadataMetrics?.().activeFrames;
    // Act
    Atomics.store(state, 0, 1);
    Atomics.notify(state, 0);
    Atomics.wait(state, 0, 1, 10000);
    assert.equal(Atomics.load(state, 0), 2, 'independent Worker confirms Target.createTarget response');
    blockedUntil(browser, () => browser.pwrightMetadataMetrics().activeFrames > frames,
      'borrowed Page native Frame admission');
    const blocked = sample();
    await diagnosticDeadline((async () => {
      for (let attempt = 0; attempt < 1000 && matching(context.pages()).length === 0; attempt++)
        await setTimeout(5);
      assert.equal(matching(context.pages()).length, 1);
    })(), 'discovered Page publication');
    const page = matching(context.pages())[0];
    const result = { initial, blocked, after: sample(),
      stableOrder: context.pages().indexOf(anchor) < context.pages().indexOf(page),
      returned: matching(context.pages())[0] === page && page.context() === context &&
        page.mainFrame().page() === page && !page.isClosed() };
    await disconnect(browser);
    assert.equal(await ownedAnchor.evaluate(() => 42), 42, 'borrowed disconnect preserves owner');
    return result;
  } finally {
    try { await disconnect(browser); }
    finally {
      try {
        if (worker) {
          worker.postMessage('finish');
          try {
            const finished = await diagnosticDeadline(exit, 'creation controller cleanup');
            if (finished.error) throw finished.error;
            assert.equal(finished.code, 0);
          } finally { await worker.terminate(); }
        }
      } finally { try { await owner?.close(); } finally { await controller.close(); } }
    }
  }
}

const initial = { added: 0, oldListed: true, oldPageClosed: false, fresh: true, retained: true };
const cases = [
  ...['Context', 'Page'].flatMap(kind => [1, 2].map(count => ({
    name: `${count} explicit ${kind} creation`, run: type => explicit(type, kind, count), count }))),
  ...['Context', 'Page'].map(kind => ({ name: `${kind} retirement plus replacement`,
    run: type => explicit(type, kind, 1, true), count: 1, replace: true })),
  { name: 'Worker-confirmed borrowed Page discovery', run: discovered, count: 1 },
];
for (const entry of cases) test(`${referenceOnly ? 'reference only' : 'paired'} collection creation: ${entry.name}`,
  { timeout: 30000 }, async () => {
    // Arrange
    const expected = await entry.run(oracle);
    process.stdout.write(`# Creation reference ${JSON.stringify({ case: entry.name, value: expected })}\n`);
    assert.deepEqual(expected, { initial, blocked: initial,
      after: { ...initial, added: entry.count, oldListed: !entry.replace, oldPageClosed: !!entry.replace },
      stableOrder: true, returned: true });
    if (referenceOnly) return;
    // Act
    const actual = await entry.run(candidate);
    // Assert
    process.stdout.write(`# Creation candidate ${JSON.stringify({ case: entry.name, value: actual })}\n`);
    assert.deepEqual(actual, expected);
  });
