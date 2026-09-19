import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { Worker } from 'node:worker_threads';
import { once } from 'node:events';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { zeroFfiMetrics } from './ffi-pressure.mjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
const { chromium: oracle } = oracleRequire('playwright');
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixtureBase = process.env.PWRIGHT_TEST_BASE;
assert.equal(endpoint, 'http://127.0.0.1:9222', 'use the dedicated Chrome/Pwright preflight');
assert.ok(fixtureBase && new URL(fixtureBase).hostname.endsWith('.ts.net'), 'use verified serve-lib fixtures');

async function withPage(type, action) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context, page;
  try {
    context = await browser.newContext();
    page = await context.newPage();
    await page.goto(`${fixtureBase}/index.html`, { timeout: 5000 });
    return await action(page, browser, context);
  } finally {
    const errors = [];
    for (const close of [() => page?.close(), () => context?.close(), () => browser.close()]) {
      try { await close(); } catch (error) { errors.push(error); }
    }
    if (browser.pwrightMetrics) {
      assert.deepEqual(browser.pwrightMetrics(), {
        connected: false, liveObjects: 0, pendingCalls: 0, topologyDrivers: 0,
        activeLeases: 0, liveScopes: 0, retainedEventBytes: 0, activeOperations: 0, cleanupInFlight: 0,
      });
      const deadline = Date.now() + 2000;
      while (browser.pwrightFfiMetrics().pending && Date.now() < deadline) await setImmediate();
      assert.deepEqual(browser.pwrightFfiMetrics(), zeroFfiMetrics);
      assert.deepEqual(browser.pwrightConversionMetrics(), { pending: 0, binaryReferences: 0, backingBytes: 0 });
    }
    if (errors.length) throw new AggregateError(errors, 'evaluation fixture cleanup failed');
  }
}

function paired(name, exercise) {
  test(name, { timeout: 20000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

paired('by-value evaluation preserves scalar types and awaits expression Promises', async page => {
  const expressions = ['undefined', 'null', 'true', '1.5', 'NaN', 'Infinity', '-Infinity', '-0',
    '9007199254740993123456789n', '"汉字 𠮷"', 'Symbol("x")', '() => 123', 'Promise.resolve(42)'];
  const results = [];
  for (const expression of expressions) results.push(await page.evaluate(expression));
  return results;
});

paired('Page and main Frame evaluate in the global main world without host-local variables', async page => {
  await page.evaluate('var evaluationGlobal = 37');
  return {
    variable: await page.mainFrame().evaluate('typeof evaluationGlobal === "undefined" ? null : evaluationGlobal'),
    globalThis: await page.evaluate(() => this === globalThis),
    hidden: await page.evaluate('typeof c'),
    closure: await page.evaluate(() => typeof handles),
    title: await page.evaluate(() => document.title),
  };
});

paired('function argument preserves special scalars and omitted means exactly one undefined argument', async page => ({
  omitted: await page.evaluate(function () { return [arguments.length, arguments[0]]; }),
  explicit: await page.evaluate(value => value, undefined),
  values: await page.evaluate(value => value, [undefined, null, NaN, Infinity, -Infinity, -0, 17n, Symbol('x')]),
  ignored: await page.evaluate('40 + 2', { ignored: true }),
}));

paired('method shorthand, async methods and ordinary functions normalize without running in Node', async page => {
  const methods = { add(value) { return value + 2; }, async double(value) { return value * 2; } };
  return [await page.evaluate(methods.add, 40), await page.evaluate(methods.double, 21),
    await page.evaluate(async value => await Promise.resolve(value + 1), 41)];
});

paired('cyclic and shared object/array identities survive both value boundaries', async page => {
  const leaf = { answer: 42 }, input = { first: leaf, second: leaf, array: [leaf] };
  input.self = input;
  const result = await page.evaluate(value => value, input);
  assert.equal(result.self, result);
  assert.equal(result.first, result.second);
  assert.equal(result.array[0], result.first);
  const generated = await page.evaluate(() => { const a = []; a.push(a); return a; });
  assert.equal(generated[0], generated);
  return { result, generated };
});

paired('Date, URL and RegExp retain their types and values', async page => {
  const value = { date: new Date('2024-04-05T12:13:14.000Z'), url: new URL('https://example.com/a?q=你好'), re: /a+b/giu };
  const result = await page.evaluate(value => value, value);
  assert.ok(result.date instanceof Date);
  assert.ok(result.url instanceof URL);
  assert.ok(result.re instanceof RegExp);
  return { date: result.date.toISOString(), url: result.url.href, regexp: [result.re.source, result.re.flags] };
});

paired('all standard typed arrays preserve byte offsets and their element kinds', async page => {
  const constructors = [Int8Array, Uint8Array, Uint8ClampedArray, Int16Array, Uint16Array,
    Int32Array, Uint32Array, Float32Array, Float64Array, BigInt64Array, BigUint64Array];
  const results = [];
  for (const Constructor of constructors) {
    const isBig = Constructor === BigInt64Array || Constructor === BigUint64Array;
    const input = new Constructor([isBig ? 0n : 0, isBig ? 42n : 42, isBig ? 7n : 7]).subarray(1);
    const result = await page.evaluate(value => value, input);
    assert.equal(result.constructor, Constructor);
    results.push([...result]);
  }
  return results;
});

paired('Error values are distinct from thrown exceptions', async page => {
  const input = new TypeError('argument error');
  input.stack = 'fixed stack';
  const value = await page.evaluate(value => value, input);
  assert.ok(value instanceof Error);
  return { name: value.name, message: value.message, stack: value.stack };
});

paired('immediate TypedArray mutation after evaluate matches upstream transport timing (NODE-017)', async page => {
  const input = { bytes: new Uint8Array([1, 2, 3]), object: { count: 42 } };
  const result = page.evaluate(async value => { await Promise.resolve(); return value; }, input);
  input.bytes.fill(99);
  input.object.count = -1;
  return await result;
});

for (const phase of ['microtask', 'immediate']) {
  paired(`TypedArray mutation in a later ${phase} has matching transport timing`, async page => {
    const input = new Uint8Array([1, 2, 3]);
    const result = page.evaluate(value => value, input);
    if (phase === 'microtask') queueMicrotask(() => input.fill(99));
    else globalThis.setImmediate(() => input.fill(99));
    return await result;
  });
}

paired('TypedArray detachment between conversion and dispatch matches', async page => {
  const input = new Uint8Array([1, 2, 3]);
  const result = page.evaluate(value => value, input).then(
    value => ({ value }), error => ({ name: error.name, message: error.message.split('\n')[0] }),
  );
  structuredClone(input.buffer, { transfer: [input.buffer] });
  return await result;
});

for (const length of [2, 6]) {
  paired(`resizing an evaluation input backing buffer to ${length} bytes matches`, async page => {
    const buffer = new ArrayBuffer(4, { maxByteLength: 8 });
    const input = new Uint8Array(buffer);
    input.set([1, 2, 3, 4]);
    const result = page.evaluate(value => value, input).then(
      value => ({ value }), error => ({ name: error.name, message: error.message.split('\n')[0] }),
    );
    buffer.resize(length);
    return await result;
  });
}

paired('later argument getters mutate a captured binary view before dispatch', async page => {
  const bytes = new Uint8Array([1, 2, 3]);
  const input = { bytes, get later() { bytes.fill(77); return 42; } };
  return await page.evaluate(value => value, input);
});

paired('fixed subarray range survives backing growth and a replaced constructor property', async page => {
  const backing = new ArrayBuffer(16, { maxByteLength: 32 });
  const input = new Uint16Array(backing, 4, 2);
  input.set([123, 456]);
  const result = page.evaluate(value => value, input);
  input.constructor = Uint8Array;
  backing.resize(32);
  return await result;
});

paired('a browser exception rejects rather than becoming an Error return value', async page => {
  try { await page.mainFrame().evaluate(() => { throw new TypeError('fixture failure'); }); }
  catch (error) { return { name: error.name, prefix: error.message.split('\n')[0] }; }
  assert.fail('thrown exception unexpectedly resolved');
});

paired('plain object key ordering, inherited properties, sparse arrays and Buffer arguments match', async page => {
  const input = Object.assign(Object.create({ inherited: true }), { b: 1, 3: 'three', a: 2 });
  input[Symbol('ignored')] = 'symbol';
  const object = await page.evaluate(value => ({ keys: Object.keys(value), value }), input);
  return { object, sparse: await page.evaluate(value => value, [, 1, , 3]),
    nulls: await page.evaluate(value => value, { '\0key\0': '\0value\0', empty: '' }),
    buffer: await page.evaluate(value => value, Buffer.from([1, 2, 255])) };
});

paired('evaluate command completion publishes synchronous URL changes even on rejection', async page => {
  await page.evaluate(() => history.replaceState(null, '', '#evaluation'));
  const success = [page.url(), page.mainFrame().url()];
  await page.evaluate(() => { history.replaceState(null, '', '#evaluation-error'); throw new Error('expected'); }).catch(() => {});
  return { success, error: [page.url(), page.mainFrame().url()] };
});

paired('evaluation uses the replacement main execution context after navigation', async page => {
  await page.evaluate(() => { globalThis.evaluationMarker = 'old'; });
  await page.goto(`${fixtureBase}/second.html`, { timeout: 5000 });
  return { marker: await page.evaluate(() => typeof globalThis.evaluationMarker),
    title: await page.mainFrame().evaluate(() => document.title), url: page.url() };
});

test('input conversion rejects before browser side effects and releases FFI admission', { timeout: 20000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    await page.evaluate(() => { globalThis.evaluationCount = 0; });
    const oversized = '界'.repeat(100000);
    // Act
    const outcome = page.evaluate(value => { globalThis.evaluationCount++; return value; }, oversized);
    // Assert
    await assert.rejects(outcome, { code: 'PWRIGHT_INVALID_ARGUMENT' });
    assert.equal(await page.evaluate(() => globalThis.evaluationCount), 0);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
  });
});

test('conversion exceptions propagate without retaining borrowed JS input or FFI leases', { timeout: 20000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const input = { get fail() { throw new Error('input getter failure'); } };
    // Act
    const result = page.evaluate(value => value, input);
    // Assert
    await assert.rejects(result, /input getter failure/);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    assert.equal(await page.evaluate('42'), 42);
  });
});

test('oversized browser results reject and release result reservations', { timeout: 20000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const produce = () => 'x'.repeat(300000);
    // Act
    const result = page.evaluate(produce);
    // Assert
    await assert.rejects(result, { code: 'PWRIGHT_RESOURCE_EXHAUSTED' });
    assert.equal(await page.evaluate('42'), 42);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
  });
});

for (const [name, input] of [
  ['depth', () => { let value = {}; for (let i = 0; i < 65; i++) value = { child: value }; return value; }],
  ['nodes', () => Array(4096).fill(null)],
  ['binary bytes', () => new Uint8Array(256 * 1024 + 1)],
]) {
  test(`oversized argument ${name} reject before execution`, { timeout: 20000 }, async () => {
    await withPage(candidate, async (page, browser) => {
      // Arrange
      const value = input();
      // Act
      const result = page.evaluate(value => { globalThis.unexpectedEvaluation = true; return value; }, value);
      // Assert
      await assert.rejects(result, { code: 'PWRIGHT_INVALID_ARGUMENT' });
      assert.equal(await page.evaluate('typeof unexpectedEvaluation'), 'undefined');
      assert.equal(browser.pwrightFfiMetrics().commands, 0);
    });
  });
}

test('source size and explicitly unsupported values fail without silent conversion', { timeout: 20000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const source = ' '.repeat(32 * 1024 + 1);
    const values = ['\ud800', new Uint8Array(new SharedArrayBuffer(16)), new Date(NaN)];
    // Act
    const oversized = page.evaluate(source);
    // Assert
    await assert.rejects(oversized, { code: 'PWRIGHT_INVALID_ARGUMENT' });
    for (const value of values) await assert.rejects(page.evaluate(value => value, value), { code: 'PWRIGHT_UNSUPPORTED' });
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    assert.equal(await page.evaluate('42'), 42);
  });
});

for (const [name, input] of [
  ['a tiny view over a large backing', () => new Uint8Array(new ArrayBuffer(1024 * 1024 + 1), 0, 1)],
  ['a growable backing maximum', () => new Uint8Array(new ArrayBuffer(1, { maxByteLength: 1024 * 1024 + 1 }))],
  ['aggregate retained backings', () => [new Uint8Array(new ArrayBuffer(768 * 1024), 0, 1), new Uint8Array(new ArrayBuffer(768 * 1024), 0, 1)]],
]) {
  test(`binary preparation rejects ${name} before retaining an unbounded reference`, { timeout: 20000 }, async () => {
    await withPage(candidate, async (page, browser) => {
      // Arrange
      const value = input();
      // Act
      const result = page.evaluate(value => value, value);
      // Assert
      await assert.rejects(result, { code: 'PWRIGHT_INVALID_ARGUMENT' });
      assert.equal(browser.pwrightFfiMetrics().commands, 0);
      assert.deepEqual(browser.pwrightConversionMetrics(), { pending: 0, binaryReferences: 0, backingBytes: 0 });
    });
  });
}

test('a later getter failure reclaims already captured binary views', { timeout: 20000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const input = { bytes: new Uint8Array(256 * 1024), get fail() { throw new Error('after binary capture'); } };
    // Act
    const result = page.evaluate(value => value, input);
    // Assert
    await assert.rejects(result, /after binary capture/);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    assert.deepEqual(browser.pwrightConversionMetrics(), { pending: 0, binaryReferences: 0, backingBytes: 0 });
  });
});

test('prepared binary commands consume admission without starting native browser work', { timeout: 20000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const input = new Uint8Array(256 * 1024);
    // Act
    const outcomes = Array.from({ length: 16 }, () => page.evaluate(value => value, input).then(
      value => ({ bytes: value.byteLength }), error => ({ code: error.code }),
    ));
    const conversion = browser.pwrightConversionMetrics();
    const ffi = browser.pwrightFfiMetrics();
    // Assert
    assert.ok(conversion.pending > 0 && conversion.pending < 16);
    assert.equal(conversion.pending, ffi.commands);
    assert.equal(conversion.binaryReferences, conversion.pending);
    assert.equal(conversion.backingBytes, conversion.pending * input.byteLength);
    assert.ok(ffi.reservedBytes >= conversion.pending * 8 * 1024 * 1024);
    assert.ok(ffi.reservedBytes <= 64 * 1024 * 1024);
    assert.equal(browser.pwrightMetrics().activeOperations, 0);
    assert.equal(ffi.queuedCommands, 0);
    const results = await Promise.all(outcomes);
    assert.equal(results.filter(result => result.bytes === input.byteLength).length, conversion.pending);
    assert.equal(results.filter(result => result.code === 'PWRIGHT_RESOURCE_EXHAUSTED').length, 16 - conversion.pending);
    assert.deepEqual(browser.pwrightConversionMetrics(), { pending: 0, binaryReferences: 0, backingBytes: 0 });
  });
});

paired('browser close while binary input is prepared rejects and releases the evaluation', async (page, browser) => {
  const outcome = page.evaluate(async value => { await new Promise(() => {}); return value; }, new Uint8Array([42])).then(
    () => ({ resolved: true }), error => ({ closed: /closed/i.test(error.message) }),
  );
  await browser.close();
  return await outcome;
});

paired('closing an owned Page cancels a never-resolving evaluation', async page => {
  const outcome = page.evaluate(() => new Promise(() => {})).then(
    () => ({ rejected: false }), error => ({ rejected: true, closed: /closed/i.test(error.message) }),
  );
  await page.close();
  return await outcome;
});

test('blocked JS retains bounded native evaluation results while close keeps admission', { timeout: 25000 }, async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const waits = Array.from({ length: 16 }, () => page.evaluate(() => new Uint8Array(128000)).then(
      value => ({ bytes: value.byteLength }), error => ({ error: error.code }),
    ));
    const initial = browser.pwrightFfiMetrics();
    assert.ok(initial.commands > 0 && initial.commands < 16);
    // Act
    const deadline = Date.now() + 10000;
    const clock = new Int32Array(new SharedArrayBuffer(4));
    while (browser.pwrightFfiMetrics().queuedCommands < initial.commands) {
      assert.ok(Date.now() < deadline, 'evaluation results did not queue while JS was blocked');
      Atomics.wait(clock, 0, 0, 10);
    }
    const queued = browser.pwrightFfiMetrics();
    const close = browser.close();
    // Assert
    assert.equal(queued.commands, initial.commands);
    assert.ok(queued.reservedBytes >= initial.commands * 8 * 1024 * 1024);
    assert.ok(queued.reservedBytes <= 64 * 1024 * 1024);
    const results = await Promise.all(waits);
    assert.ok(results.some(result => result.bytes === 128000));
    assert.ok(results.some(result => result.error === 'PWRIGHT_RESOURCE_EXHAUSTED'));
    await close;
  });
});

for (const mode of ['prepared', 'running', 'queued']) {
  test(`Worker destruction reclaims ${mode} evaluation state and its exact native Context`, { timeout: 25000 }, async () => {
    // Arrange
    const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
    const url = `${fixtureBase}/index.html#worker-evaluation-${mode}`;
    const worker = new Worker(new URL('./worker-evaluation.mjs', import.meta.url), { workerData: { endpoint, url, mode } });
    try {
      const [metrics] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
      assert.equal(metrics.ffi.commands, 3);
      assert.ok(metrics.ffi.reservedBytes >= 3 * 8 * 1024 * 1024);
      assert.ok(metrics.ffi.reservedBytes <= 64 * 1024 * 1024);
      if (mode === 'prepared') {
        assert.equal(metrics.host.activeOperations, 0);
        assert.equal(metrics.ffi.queuedCommands, 0);
        assert.deepEqual(metrics.conversion, { pending: 3, binaryReferences: 3, backingBytes: 3 * 256 * 1024 });
      } else {
        assert.equal(mode === 'queued' ? metrics.ffi.queuedCommands : metrics.host.activeOperations, 3);
        assert.deepEqual(metrics.conversion, { pending: 0, binaryReferences: 0, backingBytes: 0 });
      }
      const cdp = await observer.newBrowserCDPSession();
      const { targetInfos } = await cdp.send('Target.getTargets');
      const owned = targetInfos.filter(target => target.type === 'page' && target.url === url);
      assert.equal(owned.length, 1);
      assert.ok(owned[0].browserContextId);
      // Act
      const exit = await worker.terminate();
      // Assert
      assert.equal(exit, 1);
      const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
      const after = await cdp.send('Target.getTargets');
      assert.ok(!browserContextIds.includes(owned[0].browserContextId));
      assert.ok(!after.targetInfos.some(target => target.targetId === owned[0].targetId));
      assert.ok(observer.isConnected());
    } finally { await worker.terminate(); await observer.close(); }
  });
}
