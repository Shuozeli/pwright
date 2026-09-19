import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
const { chromium: oracle } = requireOracle('playwright');
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixtureBase = process.env.PWRIGHT_TEST_BASE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(fixtureBase && new URL(fixtureBase).hostname.endsWith('.ts.net'));
const zero = { pending: 0, rawReferences: 0, binaryReferences: 0, backingBytes: 0, exceptions: 0 };

async function withPage(type, action) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(`${fixtureBase}/locator.html`, { timeout: 5000 });
    return await action(page, browser, context);
  } finally {
    try { await context?.close(); } finally { await browser.close(); }
    if (browser.pwrightMetrics) {
      assert.deepEqual(browser.pwrightLocatorConversionMetrics(), zero);
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
    }
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

paired('Locator evaluates the selected element in its main world with one argument', async page => {
  await page.evaluate('globalThis.locatorMainValue = 37');
  return page.locator('#content').evaluate((element, arg) => ({
    id: element.id, text: element.textContent, value: arg.value, main: globalThis.locatorMainValue,
  }), { value: '汉字 😀' });
});

paired('Locator evaluation requires attachment but not visibility or actionability', page =>
  page.locator('.hidden').evaluate(element => ({ display: getComputedStyle(element).display, text: element.textContent })));

paired('Locator resolution happens before source and getter conversion', async page => {
  const order = [];
  const fn = (element, arg) => [element.id, arg.value];
  const original = fn.toString();
  fn.toString = () => { order.push('source'); return original; };
  let value = 'before';
  const input = { get value() { order.push('getter'); return value; } };
  const pending = page.locator('#late-evaluation').evaluate(fn, input, { timeout: 3000 });
  const immediate = order.slice();
  value = 'after';
  await page.evaluate(() => { const element = document.createElement('div'); element.id = 'late-evaluation'; document.body.append(element); });
  return { result: await pending, immediate, order };
});

for (const [name, selector] of [['missing', '#absent'], ['strict', '.item']]) {
  paired(`${name} Locator failure does not invoke source or argument conversion`, async page => {
    let calls = 0;
    const fn = () => 1;
    fn.toString = () => { calls++; return '() => 1'; };
    const outcome = await page.locator(selector).evaluate(fn, { get value() { calls++; return 1; } }, { timeout: 200 })
      .then(() => 'resolved', error => name === 'missing' ? error.name : /strict mode violation/.test(error.message));
    return { outcome, calls };
  });
}

paired('Locator timeout stops at prepared element and does not limit the user Promise', page =>
  page.locator('#content').evaluate(element => new Promise(resolve => setTimeout(() => resolve(element.id), 800)), undefined, { timeout: 300 }));

for (const phase of ['source', 'getter']) {
  paired(`${phase} exceptions retain identity and dispose the prepared element`, async page => {
    const thrown = new RangeError(`${phase} fixture error`);
    const fn = element => element.id;
    const input = phase === 'getter' ? { get value() { throw thrown; } } : undefined;
    if (phase === 'source') fn.toString = () => { throw thrown; };
    const identical = await page.locator('#content').evaluate(fn, input).then(() => false, error => error === thrown);
    return { identical, usable: await page.locator('#content').evaluate(element => element.id) };
  });
}

paired('binary argument copying follows getter-scheduled microtasks', page => {
  const bytes = new Uint8Array([1, 2]);
  return page.locator('#content').evaluate((element, arg) => [element.id, ...arg.bytes], {
    get bytes() { queueMicrotask(() => { bytes[0] = 9; }); return bytes; },
  });
});

paired('Locator evaluation preserves rich by-value results', page => page.locator('#content').evaluate(element => ({
  id: element.id, bigint: 9007199254740993123n, nan: NaN, negativeZero: -0,
  typed: new Uint16Array([1, 65535]), date: new Date('2020-01-02T03:04:05Z'),
})));

paired('Locator string expressions and ignored arguments match Handle semantics', page =>
  page.locator('#content').evaluate('Promise.resolve(42)', { value: 7 }));

paired('browser exceptions do not poison the next Locator evaluation', async page => {
  const message = await page.locator('#content').evaluate(() => { throw new Error('browser locator fixture'); })
    .then(() => false, error => error.message.includes('browser locator fixture'));
  return { message, next: await page.locator('#content').evaluate(element => element.id) };
});

test('unresolved Locator calls retain bounded Node roots and return all reservations', { timeout: 20000 }, async () => {
  // Arrange
  await withPage(candidate, async (page, browser) => {
    // Act
    const pending = Array.from({ length: 20 }, () => page.locator('#absent').evaluate(element => element.id, undefined, { timeout: 250 })
      .then(() => 'resolved', error => error.code));
    const metrics = browser.pwrightLocatorConversionMetrics();
    const results = await Promise.all(pending);
    // Assert
    assert.ok(metrics.pending > 0 && metrics.pending <= 8);
    assert.equal(metrics.rawReferences, metrics.pending);
    assert.ok(results.includes('PWRIGHT_RESOURCE_EXHAUSTED'));
    assert.ok(results.every(code => ['PWRIGHT_TIMEOUT', 'PWRIGHT_RESOURCE_EXHAUSTED'].includes(code)));
    assert.deepEqual(browser.pwrightLocatorConversionMetrics(), zero);
  });
});

for (const thrown of [undefined, null, 'primitive fixture', 37]) {
  paired(`getter throwing ${String(thrown)} follows upstream API error decoration`, page =>
    page.locator('#content').evaluate(element => element.id, { get value() { throw thrown; } })
      .then(() => ({ fulfilled: true }), error => ({ identical: Object.is(error, thrown), name: error?.name, message: error?.message })));
}

paired('Context close while a Locator is unresolved skips user conversion', async (page, browser, context) => {
  let called = false;
  const pending = page.locator('#absent').evaluate(element => element.id, { get value() { called = true; return 1; } })
    .then(() => false, error => /closed/i.test(error.message));
  await context.close();
  return { closed: await pending, called };
});

for (const mode of ['raw', 'binary', 'running', 'queued']) {
  test(`Worker destruction reclaims ${mode} Locator state and its exact native Context`, { timeout: 25000 }, async () => {
    // Arrange
    const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
    const url = `${fixtureBase}/locator.html#worker-locator-evaluation-${mode}`;
    const worker = new Worker(new URL('./worker-locator-evaluation.mjs', import.meta.url), { workerData: { endpoint, url, mode } });
    let cdp;
    try {
      const [metrics] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
      cdp = await observer.newBrowserCDPSession();
      const { targetInfos } = await cdp.send('Target.getTargets');
      const owned = targetInfos.filter(target => target.type === 'page' && target.url === url);
      assert.equal(owned.length, 1);
      assert.ok(owned[0].browserContextId);
      if (mode === 'running') {
        // Inspect only the exact test-owned target; prove the user program ran.
        const deadline = Date.now() + 10000;
        for (;;) {
          const targets = await cdp.send('Target.getTargets');
          if (targets.targetInfos.find(target => target.targetId === owned[0].targetId)?.title === 'pwright-locator-evaluation-running') break;
          assert.ok(Date.now() < deadline, 'Locator user program did not start');
          await setImmediate();
        }
      } else {
        assert.equal(metrics.ffi.commands, 1);
        assert.equal(metrics.conversion.pending, 1);
        assert.equal(metrics.conversion.rawReferences, mode === 'raw' ? 1 : 0);
        assert.equal(metrics.conversion.binaryReferences, mode === 'binary' ? 1 : 0);
        assert.equal(metrics.conversion.backingBytes, mode === 'binary' ? 256 * 1024 : 0);
        if (mode === 'queued') assert.equal(metrics.ffi.queuedCommands, 1);
      }
      // Act
      const exit = await worker.terminate();
      // Assert
      assert.equal(exit, 1);
      const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
      const after = await cdp.send('Target.getTargets');
      assert.ok(!browserContextIds.includes(owned[0].browserContextId));
      assert.ok(!after.targetInfos.some(target => target.targetId === owned[0].targetId));
      assert.ok(observer.isConnected());
    } finally {
      await worker.terminate();
      await cdp?.detach();
      await observer.close();
    }
  });
}

test('Worker termination preserves a borrowed default-Context Page and its running user Promise', { timeout: 25000 }, async () => {
  // Arrange: the observer owns this exact fixture target, not the Worker. Never
  // close the shared default Context or any other target while cleaning up.
  const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let page, worker;
  try {
    page = await observer.contexts()[0].newPage();
    const url = `${fixtureBase}/locator.html#worker-locator-evaluation-borrowed`;
    await page.goto(url, { timeout: 5000 });
    worker = new Worker(new URL('./worker-locator-evaluation.mjs', import.meta.url), { workerData: { endpoint, url, mode: 'borrowed' } });
    await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
    await page.waitForFunction(() => globalThis.__pwrightBorrowedCalls === 1, null, { timeout: 5000 });
    // Act
    const exit = await worker.terminate();
    // Assert before fixture close: termination did not close or navigate this
    // borrowed Page or resolve its Promise just to make native cleanup pass.
    assert.equal(exit, 1);
    assert.equal(page.isClosed(), false);
    assert.equal(page.url(), url);
    assert.equal(await page.evaluate(() => globalThis.__pwrightBorrowedCalls), 1);
    await page.evaluate(() => { globalThis.__pwrightBorrowedResume(); delete globalThis.__pwrightBorrowedResume; });
    assert.equal(await page.evaluate(() => globalThis.__pwrightBorrowedCalls), 2);
    assert.ok(observer.isConnected());
  } finally {
    await worker?.terminate();
    await page?.close();
    await observer.close();
  }
});
