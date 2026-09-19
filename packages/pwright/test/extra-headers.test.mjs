import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test, before, after } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { withHeaderCommandGate, marker, fault } from './header-command-gate.mjs';

// Test infrastructure only: both public APIs use the same dynamic wire fixture.
const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const bind = process.env.PWRIGHT_TEST_BIND;
const base = process.env.PWRIGHT_TEST_BASE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(bind, 'explicit Tailscale bind required');
assert.ok(base && new URL(base).hostname.endsWith('.ts.net'));
assert.ok(process.env.PWRIGHT_NATIVE_PROFILE?.includes('extra-headers'));
const server = createServer({ maxHeaderSize: 8192 }, (request, response) => {
  if (request.url.startsWith('/node085-referrer?')) navigationRequests++;
  response.writeHead(200, {
    'content-type': 'text/html', 'cache-control': 'no-store',
    'x-seen-layer': request.headers['x-layer'] ?? '<absent>',
    'x-seen-context': request.headers['x-context'] ?? '<absent>',
    'x-seen-referer': request.headers.referer ?? '<absent>',
    'x-seen-node089': request.headers['x-node089'] ?? '<absent>',
  });
  response.end('<title>header differential</title>');
});
server.maxConnections = 8;
server.requestTimeout = 5000;
let url;
let navigationRequests = 0;
before(async () => {
  server.listen(0, bind);
  await once(server, 'listening');
  url = new URL('/node082-header-ffi', base);
  url.port = String(server.address().port);
  process.stdout.write(`# header fixture ${url.href}\n`);
});
after(async () => {
  const closed = once(server, 'close');
  server.close();
  server.closeAllConnections();
  await closed;
});

async function withPage(type, exercise) {
  const side = type === candidate ? 'candidate' : 'oracle';
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    return await exercise(await context.newPage(), context);
  } finally {
    // Only the fresh, API-owned Context is closed; borrowed shared Pages survive.
    try { await diagnosticDeadline(context?.close(), `${side} Context cleanup`); }
    finally { await diagnosticDeadline(browser.close(), `${side} Browser disconnect`); }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}
function paired(name, exercise) {
  test(name, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.deepEqual(actual, expected);
  });
}
async function observed(page) {
  const response = await page.goto(url.href, { timeout: 5000 });
  const headers = response.headers();
  return [headers['x-seen-layer'], headers['x-seen-context']];
}

for (const scope of ['Page', 'Context']) {
  paired(`${scope} overlapping setters retain invocation order without a busy-writer error`, async (page, context) => {
    // Arrange
    const owner = scope === 'Page' ? page : context;
    // Act: all three publications happen before observing any native result.
    const pending = ['first', 'second', 'third'].map(value => owner.setExtraHTTPHeaders({ 'X-Layer': value }));
    const results = await Promise.allSettled(pending);
    // Assert against upstream completion and the actual subsequent wire value.
    return { results: results.map(result => ({ status: result.status,
      error: result.status === 'rejected' ? result.reason.message.split('\n')[0] : null })),
    wire: await observed(page) };
  });
}

paired('Context partial fanout failure preserves Page overrides and a following setter', async (page, context) => {
  // Arrange: the Page override masks the invalid Context value only on one Page.
  await context.setExtraHTTPHeaders({ 'X-Layer': 'context', 'X-Context': 'old' });
  await page.setExtraHTTPHeaders({ 'X-Layer': 'page' });
  const second = await context.newPage();
  // Act: reject one branch while another branch has a valid effective map.
  const [failure] = await Promise.allSettled([context.setExtraHTTPHeaders({
    'X-Layer': 'bad\nvalue', 'X-Context': 'transient',
  })]);
  await context.setExtraHTTPHeaders({ 'X-Layer': 'healthy', 'X-Context': 'after' });
  // Assert against upstream error and both actual wire outcomes after recovery.
  return { failure: failure.status,
    error: failure.status === 'rejected' ? failure.reason.message.split('\n')[0] : null,
    first: await observed(page), second: await observed(second) };
});

async function heldNativeUpdate(type, scope, failPrimary) {
  return withHeaderCommandGate(endpoint, async (relay, gate) => {
    const browser = await type.connectOverCDP(relay, { timeout: 5000 });
    let context;
    try {
      // Arrange: commands are only intercepted after this owned Context is ready.
      context = await browser.newContext();
      const page = await context.newPage();
      const pages = scope === 'Page' ? [page] : [page, await context.newPage()];
      const owner = scope === 'Page' ? page : context;
      await owner.setExtraHTTPHeaders({ [marker]: 'old' });
      gate.arm({ failPrimary });
      // Act: hold the first Page's primary or compensation receipt. Context
      // dispatch must not start its second Page before that receipt is released.
      let settled = false;
      const failed = owner.setExtraHTTPHeaders({ [marker]: 'proposal' })
        .then(() => ({ status: 'fulfilled' }), error => ({
          status: 'rejected', error: error.message.split('\n')[0],
        })).then(result => { settled = true; return result; });
      await gate.held(1);
      if (failPrimary) {
        const result = await failed;
        // Assert: no compensation receipt was released to settle that API call.
        assert.equal(result.status, 'rejected');
        assert.ok(result.error.includes(fault));
      } else {
        assert.equal(settled, false, 'success waits for its native ACK');
      }
      const held = gate.counts();
      assert.deepEqual(held, { primary: 1, compensation: failPrimary ? 1 : 0, held: 1 });
      await owner.setExtraHTTPHeaders({ [marker]: 'after' });
      const wire = [];
      for (const target of pages) {
        const response = await target.goto(url.href, { timeout: 5000 });
        wire.push(response.headers()['x-seen-node089']);
      }
      assert.deepEqual(wire, pages.map(() => 'after'));
      if (!failPrimary) assert.equal(settled, false, 'success remains pending until the held ACK is released');
      const lateReceipt = scope === 'Context' ? gate.nextReceipt() : undefined;
      gate.release();
      // Context compensation resumes its Page loop after the first ACK. Its
      // second Page must read then-current desired values, not frozen old ones.
      const late = await lateReceipt;
      if (late) assert.deepEqual(late, { value: 'after', error: undefined });
      const result = await failed;
      const afterDrain = [];
      for (const target of pages) {
        const response = await target.goto(url.href, { timeout: 5000 });
        afterDrain.push(response.headers()['x-seen-node089']);
      }
      assert.deepEqual(afterDrain, pages.map(() => 'after'));
      // A fresh public transaction and navigation also succeed after old ACKs.
      await owner.setExtraHTTPHeaders({ [marker]: 'final' });
      const final = (await page.goto(url.href, { timeout: 5000 })).headers()['x-seen-node089'];
      return { result, held, wire, late, afterDrain, final };
    } finally {
      gate.release();
      try { await diagnosticDeadline(context?.close(), 'gated Context cleanup'); }
      finally { await diagnosticDeadline(browser.close(), 'gated Browser disconnect'); }
      if (browser.pwrightMetrics) {
        assert.equal(browser.pwrightMetrics().activeLeases, 0);
        assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
        for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
        assert.equal(browser.pwrightFfiMetrics().pending, 0);
        assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      }
    }
  });
}
for (const scope of ['Page', 'Context']) {
  for (const failPrimary of [false, true]) {
    const behavior = failPrimary ? 'failure rejects while compensation receipts remain held'
      : 'success waits while a later setter updates headers';
    test(`${scope} native ${behavior}`, { timeout: 25000 }, async () => {
      // Arrange
      const expected = await heldNativeUpdate(oracle, scope, failPrimary);
      // Act
      const actual = await heldNativeUpdate(candidate, scope, failPrimary);
      // Assert
      assert.deepEqual(actual, expected);
    });
  }
}

for (const scope of ['Page', ...(process.env.PWRIGHT_NATIVE_PROFILE.includes('frame') ? ['Frame'] : [])]) {
  paired(`${scope} navigation keeps its policy before a later setter changes desired headers`, async (page) => {
    const target = scope === 'Page' ? page : page.mainFrame();
    const outcomes = [];
    for (let iteration = 0; iteration < 8; iteration++) {
      // Arrange
      const old = `${url.origin}/before-${iteration}`;
      const next = `${url.origin}/after-${iteration}`;
      await page.setExtraHTTPHeaders({ Referer: old });
      const before = navigationRequests;
      // Act: a later setter must not erase the already observed conflict.
      const navigating = target.goto(`${url.origin}/node085-referrer?reverse=${iteration}`, {
        referer: next, timeout: 5000,
      });
      const setting = page.setExtraHTTPHeaders({ Referer: next });
      const [navigation, set] = await Promise.allSettled([navigating, setting]);
      // Assert against the actual upstream public trace.
      outcomes.push({ set: set.status, navigation: navigation.status,
        error: navigation.status === 'rejected' ? navigation.reason.message.split('\n')[0] : null,
        requests: navigationRequests - before });
    }
    return outcomes;
  });

  paired(`${scope} navigation sees a preceding setter without awaiting its native acknowledgement`, async (page) => {
    const target = scope === 'Page' ? page : page.mainFrame();
    const outcomes = [];
    for (let iteration = 0; iteration < 8; iteration++) {
      // Arrange
      const old = `${url.origin}/old-${iteration}`;
      const next = `${url.origin}/new-${iteration}`;
      await page.setExtraHTTPHeaders({ Referer: old });
      const before = navigationRequests;
      // Act: both public promises are observed immediately, without serializing.
      const setting = page.setExtraHTTPHeaders({ Referer: next });
      const navigating = target.goto(`${url.origin}/node085-referrer?pending=${iteration}`, {
        referer: old, timeout: 5000,
      });
      const [set, navigation] = await Promise.allSettled([setting, navigating]);
      // Assert in the paired comparison; preserve a failing candidate trace.
      outcomes.push({ set: set.status, navigation: navigation.status,
        error: navigation.status === 'rejected' ? navigation.reason.message.split('\n')[0] : null,
        requests: navigationRequests - before });
    }
    return outcomes;
  });

  paired(`${scope} goto reconciles the first original Page Referer, not merged Context headers`, async (page, context) => {
    const origin = url.origin;
    const first = `${origin}/first`;
    const last = `${origin}/last`;
    const fromContext = `${origin}/context`;
    const cases = [
      { id: 'absent' },
      { id: 'context', context: { Referer: fromContext } },
      { id: 'context-option', context: { Referer: fromContext }, option: first },
      { id: 'page', page: { Referer: first } },
      { id: 'page-same', page: { Referer: first }, option: first },
      { id: 'page-conflict', page: { Referer: first }, option: last, conflict: true },
      { id: 'duplicates', page: { Referer: first, referer: last } },
      { id: 'duplicates-first-option', page: { Referer: first, referer: last }, option: first },
      { id: 'duplicates-last-conflict', page: { Referer: first, referer: last }, option: last, conflict: true },
      { id: 'empty-conflict', page: { Referer: '' }, option: first, conflict: true },
      { id: 'empty-option', page: { Referer: '' }, option: '' },
      { id: 'context-page', context: { Referer: fromContext }, page: { referer: first } },
      { id: 'clear-page', context: { Referer: fromContext }, page: { Referer: first }, clear: true, option: last },
    ];
    const result = [];
    for (const spec of cases) {
      // Arrange
      await page.setExtraHTTPHeaders({});
      await context.setExtraHTTPHeaders(spec.context ?? {});
      if (spec.page) await page.setExtraHTTPHeaders(spec.page);
      if (spec.clear) await page.setExtraHTTPHeaders({});
      const target = scope === 'Page' ? page : page.mainFrame();
      const before = navigationRequests;
      const previousUrl = page.url();
      let wire, error;
      // Act
      try {
        const response = await target.goto(`${origin}/node085-referrer?case=${spec.id}`, {
          timeout: 5000, ...(spec.option !== undefined ? { referer: spec.option } : {}),
        });
        wire = response.headers()['x-seen-referer'];
      } catch (caught) { error = { name: caught.name, message: caught.message.split('\n')[0] }; }
      // Assert: conflicts reject before browser navigation, not after wire I/O.
      const requests = navigationRequests - before;
      if (spec.conflict) {
        assert.match(error?.message ?? '', /"referer" is already specified as extra HTTP header/);
        assert.equal(requests, 0);
        assert.equal(page.url(), previousUrl);
      } else { assert.equal(error, undefined); assert.equal(requests, 1); }
      result.push({ id: spec.id, wire, error, requests });
    }
    return result;
  });
}

paired('creation headers are installed for the first Page and coexist with baseURL', async (_page, existing) => {
  const headers = { 'X-Layer': 'created', 'X-Context': 'initial' };
  const context = await existing.browser().newContext({ baseURL: url.href, extraHTTPHeaders: headers });
  headers['X-Layer'] = 'mutated-after-creation';
  try {
    const page = await context.newPage();
    const response = await page.goto('./created', { timeout: 5000 });
    const first = response.headers()['x-seen-layer'];
    await page.setExtraHTTPHeaders({ 'x-layer': 'override' });
    const overridden = await observed(page);
    await page.setExtraHTTPHeaders({});
    const inherited = await observed(page);
    await context.setExtraHTTPHeaders({});
    const cleared = await observed(await context.newPage());
    assert.equal(first, 'created');
    return { first, overridden, inherited, cleared };
  } finally { await context.close(); }
});

paired('creation skips falsy headers and copies only own enumerable option fields', async (_page, existing) => {
  const result = [];
  for (const extraHTTPHeaders of [undefined, null, false, 0, '', NaN, 0n]) {
    const context = await existing.browser().newContext({ extraHTTPHeaders });
    try { result.push(await observed(await context.newPage())); }
    finally { await context.close(); }
  }
  const inherited = Object.create({ extraHTTPHeaders: { 'X-Layer': 'not-an-own-option' } });
  const context = await existing.browser().newContext(inherited);
  try { result.push(await observed(await context.newPage())); }
  finally { await context.close(); }
  assert.ok(result.every(([layer]) => layer === '<absent>'));
  return result;
});

paired('creation option getters run once while header values follow two-phase conversion', async (_page, existing) => {
  let optionsRead = 0;
  let headerReads = 0;
  const options = { get extraHTTPHeaders() {
    optionsRead++;
    return { get 'X-Layer'() { return `value-${++headerReads}`; } };
  } };
  const context = await existing.browser().newContext(options);
  try {
    assert.equal(optionsRead, 1);
    assert.equal(headerReads, 2);
    return { optionsRead, headerReads, wire: await observed(await context.newPage()) };
  } finally { await context.close(); }
});

paired('creation language errors preserve phase, parameter path and thrown identity', async (_page, existing) => {
  const result = [];
  const browser = existing.browser();
  const before = browser.contexts().length;
  const boxed = new String('boxed');
  boxed.valueOf = () => 123;
  for (const input of [{ 'X-Layer': 1 }, Object.create({ 'X-Layer': 1 }), { 'X-Layer': boxed }]) {
    let caught;
    try { await browser.newContext({ extraHTTPHeaders: input }); }
    catch (error) { caught = { name: error.name, message: error.message }; }
    assert.ok(caught);
    assert.equal(browser.contexts().length, before);
    result.push(caught);
  }
  for (const thrown of [null, 17, new Error('creation getter')]) {
    let caught;
    try { await browser.newContext({ extraHTTPHeaders: { get 'X-Layer'() { throw thrown; } } }); }
    catch (error) { caught = error; }
    assert.equal(caught, thrown);
    result.push(caught === thrown);
  }
  return result;
});

paired('invalid creation syntax is deferred to Page initialization and Context can recover', async (_page, existing) => {
  const context = await existing.browser().newContext({ extraHTTPHeaders: { 'X-Layer': 'bad\nvalue' } });
  try {
    let message;
    try { await context.newPage(); }
    catch (error) { message = error.message; }
    assert.ok(message);
    assert.equal(context.pages().length, 0);
    await context.setExtraHTTPHeaders({ 'X-Layer': 'recovered' });
    return { message, wire: await observed(await context.newPage()) };
  } finally { await context.close(); }
});

paired('Context inheritance, Page override and clearing across new session routes', async (page, context) => {
  await context.setExtraHTTPHeaders({ 'X-Layer': 'context-one', 'X-Context': 'one' });
  const second = await context.newPage();
  const results = [await observed(page), await observed(second)];
  await page.setExtraHTTPHeaders({ 'X-Layer': 'first', 'x-layer': 'page' });
  results.push(await observed(page));
  await context.setExtraHTTPHeaders({ 'X-Layer': 'context-two', 'X-Context': 'two' });
  results.push(await observed(page), await observed(second));
  await page.setExtraHTTPHeaders({});
  results.push(await observed(page));
  await context.setExtraHTTPHeaders({});
  results.push(await observed(page), await observed(second));
  assert.deepEqual(results, [
    ['context-one', 'one'], ['context-one', 'one'], ['page', 'one'],
    ['page', 'two'], ['context-two', 'two'], ['context-two', 'two'],
    ['<absent>', '<absent>'], ['<absent>', '<absent>'],
  ]);
  return results;
});

paired('later duplicate Page value shadows an earlier invalid value before native validation', async page => {
  await page.setExtraHTTPHeaders({ 'X-Layer': 'bad\nvalue', 'x-layer': 'valid' });
  return observed(page);
});

paired('Page override masks invalid Context syntax until the override is removed', async (page, context) => {
  await page.setExtraHTTPHeaders({ 'X-Layer': 'page' });
  await context.setExtraHTTPHeaders({ 'X-Layer': 'bad\nvalue' });
  const before = await observed(page);
  let message;
  try { await page.setExtraHTTPHeaders({}); }
  catch (error) { message = error.message; }
  assert.ok(message, 'unmasking invalid Context value must fail');
  const after = await observed(page);
  await context.setExtraHTTPHeaders({});
  await page.setExtraHTTPHeaders({});
  return { before, message, after, cleared: await observed(page) };
});

paired('empty Context accepts invalid syntax and can clear it before creating a Page', async (page, context) => {
  await page.close();
  await context.setExtraHTTPHeaders({ 'bad name': 'bad\nvalue' });
  await context.setExtraHTTPHeaders({});
  return observed(await context.newPage());
});

paired('invalid Context header Page initialization fails locally and remains recoverable', async (page, context) => {
  await page.close();
  await context.setExtraHTTPHeaders({ 'X-Layer': 'bad\nvalue' });
  let message;
  try { await diagnosticDeadline(context.newPage(), 'invalid-header Page creation'); }
  catch (error) { message = error.message; }
  assert.ok(message, 'native Page initialization must reject the invalid header');
  const failedPages = context.pages();
  assert.equal(failedPages.length, 0, 'failed Page is closed before creation rejects');
  await context.setExtraHTTPHeaders({});
  const fresh = await context.newPage();
  const recovered = await observed(fresh);
  let frameRecovered;
  if (process.env.PWRIGHT_NATIVE_PROFILE.includes('frame')) {
    const response = await fresh.mainFrame().goto(url.href, { timeout: 5000, waitUntil: 'load' });
    frameRecovered = response.status();
  }
  return { message, failedPages: failedPages.length,
    recovered, frameRecovered, totalPages: context.pages().length };
});

for (const scope of ['Page', 'Context']) {
  paired(`${scope} language values and native syntax errors`, async (page, context) => {
    const target = scope === 'Page' ? page : context;
    const result = [];
    const inputs = [
      { 'X-Layer': 'plain' }, { 'X-Layer': undefined },
      { 'X-Layer': new String('boxed') },
      Object.create({ 'X-Layer': 'inherited' }),
      { 'X-Layer': 1 }, { 'X-Layer': null }, { 'X-Layer': true },
      { 'X-Layer': 1n }, { 'X-Layer': Symbol('header') }, { 'X-Layer': () => 1 },
      Object.create({ 'X-Layer': 1 }),
      { 'bad name': 'v' }, { '': 'v' }, { '非ascii': 'v' }, { 'X-Layer': 'bad\nvalue' },
      null, undefined, 1, true,
    ];
    for (const input of inputs) {
      let error;
      try { await target.setExtraHTTPHeaders(input); }
      catch (caught) { error = { name: caught?.name, message: caught?.message }; }
      result.push({ error, wire: await observed(page) });
    }
    return result;
  });
  paired(`${scope} getters, boxed valueOf and thrown value identity`, async (page, context) => {
    const target = scope === 'Page' ? page : context;
    const result = [];
    let reads = 0;
    await target.setExtraHTTPHeaders({ get 'X-Layer'() { return `read-${++reads}`; } });
    result.push({ reads, wire: await observed(page) });
    assert.equal(reads, 2);
    const boxed = new String('original');
    boxed.valueOf = () => 'value-of';
    boxed.toString = () => { throw new Error('must not use ToString'); };
    await target.setExtraHTTPHeaders({ 'X-Layer': boxed });
    result.push(await observed(page));
    for (const thrown of [new Error('getter sentinel'), { sentinel: true }, null, 'text']) {
      let caught, rejected = false;
      try { await target.setExtraHTTPHeaders({ get 'X-Layer'() { throw thrown; } }); }
      catch (error) { caught = error; rejected = true; }
      assert.equal(rejected, true);
      assert.equal(caught, thrown);
      result.push(await observed(page));
    }
    return result;
  });
  paired(`${scope} collects language values before boxed conversion`, async (page, context) => {
    const target = scope === 'Page' ? page : context;
    const trace = [];
    const boxed = new String('boxed');
    const headers = {
      get 'X-Layer'() { trace.push('layer'); return boxed; },
      'X-Context': 'before-unbox',
    };
    boxed.valueOf = () => {
      trace.push('valueOf');
      headers['X-Context'] = 'after-unbox';
      return 'unboxed';
    };
    await target.setExtraHTTPHeaders(headers);
    return { trace, wire: await observed(page) };
  });
  paired(`${scope} prototype enumeration and conversion errors`, async (page, context) => {
    const target = scope === 'Page' ? page : context;
    const result = [];
    const shadowed = Object.create({ 'X-Layer': 'must-be-shadowed' });
    Object.defineProperty(shadowed, 'X-Layer', { value: 'hidden', enumerable: false });
    const deleted = Object.create({ 'X-Context': 'prototype' });
    let reads = 0;
    Object.defineProperty(deleted, 'X-Layer', { enumerable: true, get() {
      if (++reads === 2) delete deleted['X-Context'];
      return 'getter';
    } });
    deleted['X-Context'] = 'own';
    const wrongBox = new String('box');
    wrongBox.valueOf = () => 42;
    const thrownValues = [new Error('valueOf sentinel'), null, 'text', 42,
      Object.freeze(new Error('frozen')), { message: 'object' }];
    const throwingInputs = thrownValues.map(thrown => {
      const box = new String('box');
      box.valueOf = () => { throw thrown; };
      return { 'X-Layer': box };
    });
    for (const input of [shadowed, deleted, { 'X-Layer': wrongBox }, ...throwingInputs]) {
      let error;
      try { await target.setExtraHTTPHeaders(input); }
      catch (caught) { error = { name: caught?.name, message: caught?.message }; }
      result.push({ error, wire: await observed(page) });
    }
    return result;
  });
}

test('selected FFI admission rejects oversized Rust copies and releases failed input tickets', { timeout: 25000 }, async () => {
  await withPage(candidate, async page => {
    // Arrange
    const inputs = [Object.fromEntries(Array.from({ length: 257 }, (_, i) => [`X-${i}`, 'v'])),
      { 'X-Layer': 'x'.repeat(65537) }];
    for (const input of inputs) {
      // Act
      const result = page.setExtraHTTPHeaders(input);
      // Assert
      await assert.rejects(result, /exceeds 256|exceed 65536/);
    }
    await page.setExtraHTTPHeaders({ 'X-Layer': 'after-rejection' });
    assert.deepEqual(await observed(page), ['after-rejection', '<absent>']);
  });
});

test('failed initialization slots retire across more Pages than the fixed table capacity', { timeout: 25000 }, async () => {
  await withPage(candidate, async (page, context) => {
    await page.close();
    await context.setExtraHTTPHeaders({ 'X-Layer': 'bad\nvalue' });
    for (let iteration = 0; iteration < 18; iteration++) {
      // Arrange: a Context retaining invalid syntax, with no existing Pages.
      assert.equal(context.pages().length, 0);
      // Act
      const result = context.newPage();
      // Assert
      await assert.rejects(result, /Invalid header value/);
      const failed = context.pages();
      assert.equal(failed.length, 0);
    }
    await context.setExtraHTTPHeaders({});
    assert.deepEqual(await observed(await context.newPage()), ['<absent>', '<absent>']);
  });
});

test('creation input admission leaves no Context and closed Context headers reclaim capacity', { timeout: 25000 }, async () => {
  await withPage(candidate, async (_page, existing) => {
    // Arrange
    const browser = existing.browser();
    const before = browser.contexts().length;
    const inputs = [Object.fromEntries(Array.from({ length: 257 }, (_, i) => [`X-${i}`, 'v'])),
      { 'X-Layer': 'x'.repeat(65537) }];
    for (const extraHTTPHeaders of inputs) {
      // Act
      const result = browser.newContext({ extraHTTPHeaders });
      // Assert
      await assert.rejects(result, /exceeds 256|exceed 65536/);
      assert.equal(browser.contexts().length, before);
      assert.equal(browser.pwrightContextMetadataMetrics().pendingCreations, 0);
    }
    for (let iteration = 0; iteration < 18; iteration++) {
      const context = await browser.newContext({ extraHTTPHeaders: { 'X-Layer': `created-${iteration}` } });
      await context.close();
      assert.equal(browser.contexts().length, before);
    }
    const context = await browser.newContext({ extraHTTPHeaders: { 'X-Layer': 'after-retirement' } });
    try { assert.deepEqual(await observed(await context.newPage()), ['after-retirement', '<absent>']); }
    finally { await context.close(); }
  });
});

test('navigation header admission reclaims after repeated pre-dispatch conflicts', { timeout: 25000 }, async () => {
  await withPage(candidate, async page => {
    // Arrange
    await page.setExtraHTTPHeaders({ Referer: `${url.origin}/retained` });
    const before = navigationRequests;
    for (let index = 0; index < 17; index++) {
      // Act
      const result = page.goto(`${url.origin}/node085-referrer?rejected=${index}`, {
        referer: `${url.origin}/conflicting`, timeout: 5000,
      });
      // Assert
      await assert.rejects(result, /"referer" is already specified as extra HTTP header/);
      assert.equal(navigationRequests, before);
    }
    await page.setExtraHTTPHeaders({});
    const response = await page.goto(`${url.origin}/node085-referrer?recovered`, {
      referer: `${url.origin}/explicit`, timeout: 5000,
    });
    assert.equal(response.headers()['x-seen-referer'], `${url.origin}/explicit`);
  });
});
