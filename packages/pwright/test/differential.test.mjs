import assert from 'node:assert/strict';
import { test, beforeEach, afterEach } from 'node:test';
import { createRequire } from 'node:module';
import { Worker } from 'node:worker_threads';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { chromium as candidate } from '@shuozeli/pwright';
import { findFixturePage, fixtureTitle } from './fixture-selection.mjs';
import { queueReadsWithoutDelivering, blockUntilResultsAreQueued, zeroFfiMetrics } from './ffi-pressure.mjs';
import { withNavigationResponseGate } from './navigation-response-gate.mjs';
import { createLifecycleTrace } from './lifecycle-trace.mjs';

const traceStart = performance.now();
const trace = createLifecycleTrace(line => process.stdout.write(line), () => Math.round(performance.now() - traceStart));
beforeEach(context => {
  process.send?.({ event: 'pwright-case', phase: 'start', name: context.name });
  trace('test:start', context.name);
});
afterEach(context => {
  trace('test:end', context.name);
  process.send?.({ event: 'pwright-case', phase: 'end', name: context.name });
});

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
const { chromium: oracle } = oracleRequire('playwright');
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2', 'the oracle must remain pinned');
const endpoint = process.env.PWRIGHT_TEST_CDP;
if (!endpoint) throw new Error('Set PWRIGHT_TEST_CDP explicitly after the dedicated Chrome/Pwright preflight');
const fixtureBase = process.env.PWRIGHT_TEST_BASE;
if (!fixtureBase) throw new Error('Set PWRIGHT_TEST_BASE to the verified serve-lib fixture directory URL');
const fixtureBind = process.env.PWRIGHT_TEST_BIND;
if (!fixtureBind) throw new Error('Set PWRIGHT_TEST_BIND to the verified Tailscale IP for dynamic stalled-response cases');

function topologySnapshot(browser) {
  const contexts = browser.contexts();
  return {
    version: browser.version(), connected: browser.isConnected(), type: browser.browserType().name(),
    contexts: contexts.map((context, index) => ({
      identity: context === browser.contexts()[index], browser: context.browser() === browser,
      pages: context.pages().map((page, pageIndex) => ({
        identity: page === context.pages()[pageIndex], context: page.context() === context,
        frameIdentity: page.mainFrame() === page.mainFrame(),
        framePage: page.mainFrame().page() === page, parent: page.mainFrame().parentFrame(),
      })),
    })),
  };
}

const zeroMetrics = {
  connected: false, liveObjects: 0, pendingCalls: 0, topologyDrivers: 0,
  activeLeases: 0, liveScopes: 0, retainedEventBytes: 0,
  activeOperations: 0, cleanupInFlight: 0,
};

async function readTopology(type, options) {
  const browser = await type.connectOverCDP(options);
  try { return topologySnapshot(browser); }
  finally { await browser.close(); }
}

test('existing Context, Page and main Frame snapshots agree with Playwright', async () => {
  // Arrange
  const expected = await readTopology(oracle, { endpointURL: endpoint });
  assert.ok(expected.contexts.length > 0, 'default Context must exist at attach completion');
  assert.ok(expected.contexts.some(context => context.pages.length > 0), 'preflight must create a Page');
  // Act
  const actual = await readTopology(candidate, { endpointURL: endpoint });
  // Assert
  assert.deepEqual(actual, expected);
});

async function closeSnapshot(type) {
  const browser = await type.connectOverCDP(endpoint);
  const contexts = browser.contexts();
  const events = [];
  browser.on('disconnected', value => events.push({ event: 'disconnected', identity: value === browser }));
  try {
    await Promise.all([browser.close(), browser.close()]);
    return {
      observed: { connected: browser.isConnected(), contexts: browser.contexts().length,
        retainedContextPages: contexts.map(context => context.pages().length), events },
      metrics: browser.pwrightMetrics?.(),
    };
  } finally { await browser.close(); }
}

test('concurrent close is idempotent, signals once and drains native resources', async () => {
  // Arrange
  const before = await readTopology(oracle, { endpointURL: endpoint });
  const expected = await closeSnapshot(oracle);
  // Act
  const actual = await closeSnapshot(candidate);
  // Assert
  assert.deepEqual(actual.observed, expected.observed);
  assert.deepEqual(actual.metrics, zeroMetrics);
  assert.deepEqual(await readTopology(oracle, { endpointURL: endpoint }), before,
    'externally owned Chrome and all preexisting Pages must survive');
});

test('direct browser WebSocket attachment has the same topology', async () => {
  // Arrange
  const discovery = await (await fetch(`${endpoint.replace(/\/$/, '')}/json/version`)).json();
  const options = { endpointURL: discovery.webSocketDebuggerUrl, timeout: 5000 };
  const expected = await readTopology(oracle, options);
  // Act
  const actual = await readTopology(candidate, options);
  // Assert
  assert.deepEqual(actual, expected);
});

test('malformed endpoint returns a rejected Error in both implementations', async () => {
  // Arrange
  const capture = async type => {
    try { await type.connectOverCDP('not-an-endpoint', { timeout: 1000 }); }
    catch (error) { return { name: error.name, error: error instanceof Error }; }
    throw new Error('invalid endpoint unexpectedly connected');
  };
  const expected = await capture(oracle);
  // Act
  const actual = await capture(candidate);
  // Assert
  assert.deepEqual(actual, expected);
});

test('sequential sessions release every recorded Rust lease', async () => {
  // Arrange
  const samples = [];
  // Act
  for (let index = 0; index < 5; index++) {
    const browser = await candidate.connectOverCDP(endpoint);
    await browser.close();
    samples.push(browser.pwrightMetrics());
  }
  // Assert
  assert.deepEqual(samples, Array.from({ length: 5 }, () => zeroMetrics));
});

test('worker environment teardown does not retain a Node-owned session', async () => {
  // Arrange
  const before = await readTopology(oracle, { endpointURL: endpoint });
  const worker = new Worker(new URL('./worker.mjs', import.meta.url), { workerData: endpoint });
  try { await once(worker, 'message', { signal: AbortSignal.timeout(5000) }); }
  catch (error) { await worker.terminate(); throw error; }
  // Act
  const exit = await worker.terminate();
  // Assert
  assert.equal(exit, 1);
  assert.deepEqual(await readTopology(candidate, { endpointURL: endpoint }), before);
});

async function withFixturePage(type, action, initialPath = 'index.html') {
  trace('fixture:controller-connect:start');
  const controller = await oracle.connectOverCDP(endpoint);
  trace('fixture:controller-connect:done');
  const setupPage = await controller.contexts()[0].newPage();
  trace('fixture:native-page-created');
  let browser;
  try {
    await setupPage.goto(`${fixtureBase}/${initialPath}`);
    await setupPage.evaluate(title => { document.title = title; }, fixtureTitle);
    trace(type === oracle ? 'fixture:oracle-connect:start' : 'fixture:candidate-connect:start');
    browser = await type.connectOverCDP(endpoint);
    trace('fixture:tested-connect:done');
    const pages = browser.contexts().flatMap(context => context.pages());
    const page = await findFixturePage(pages, setupPage.url());
    trace('fixture:action:start');
    return await action(page, browser, setupPage);
  } finally {
    // Always release the exact test-owned Page, including when native close
    // reports a cleanup failure. Never leave it for the next case to inherit.
    trace('fixture:browser-close:start');
    try { await browser?.close(); trace('fixture:browser-close:done'); }
    finally {
      trace('fixture:page-close:start');
      try { await setupPage.close(); trace('fixture:page-close:done'); }
      finally { trace('fixture:controller-close:start'); await controller.close(); trace('fixture:controller-close:done'); }
    }
    if (browser?.pwrightMetrics) assert.deepEqual(browser.pwrightMetrics(), zeroMetrics);
    if (browser?.pwrightFfiMetrics) {
      await waitForSnapshot(() => browser.pwrightFfiMetrics().pending === 0);
      assert.deepEqual(browser.pwrightFfiMetrics(), zeroFfiMetrics);
    }
  }
}

test('Page and main Frame title and content reads agree byte-for-byte', async () => {
  // Arrange
  const read = async page => ({ title: await page.title(), content: await page.content(),
    frameTitle: await page.mainFrame().title(), frameContent: await page.mainFrame().content() });
  const expected = await withFixturePage(oracle, read);
  // Act
  const actual = await withFixturePage(candidate, read);
  // Assert
  assert.deepEqual(actual, expected);
});

async function withForeignContextPage(type, action) {
  trace('foreign:controller-connect:start');
  const controller = await oracle.connectOverCDP(endpoint);
  trace('foreign:controller-connect:done');
  const ownedContext = await controller.newContext();
  trace('foreign:context-created');
  let browser;
  try {
    const marker = await ownedContext.newPage();
    await marker.goto(`${fixtureBase}/index.html`);
    await marker.evaluate(title => { document.title = title; }, fixtureTitle);
    trace(type === oracle ? 'foreign:oracle-connect:start' : 'foreign:candidate-connect:start');
    browser = await type.connectOverCDP(endpoint);
    trace('foreign:tested-connect:done');
    const page = await findFixturePage(browser.contexts().flatMap(context => context.pages()), marker.url());
    trace('foreign:action:start');
    return await action(page.context(), browser, ownedContext);
  } finally {
    trace('foreign:browser-close:start');
    try { await browser?.close(); trace('foreign:browser-close:done'); }
    finally {
      trace('foreign:context-close:start');
      try { await ownedContext.close(); trace('foreign:context-close:done'); }
      finally { trace('foreign:controller-close:start'); await controller.close(); trace('foreign:controller-close:done'); }
    }
    if (browser?.pwrightMetrics) assert.deepEqual(browser.pwrightMetrics(), zeroMetrics);
    if (browser?.pwrightFfiMetrics) {
      await waitForSnapshot(() => browser.pwrightFfiMetrics().pending === 0);
      assert.deepEqual(browser.pwrightFfiMetrics(), zeroFfiMetrics);
    }
  }
}

async function withAttachedBrowser(type, action) {
  trace(type === oracle ? 'attached:oracle-connect:start' : 'attached:candidate-connect:start');
  const browser = await type.connectOverCDP(endpoint);
  trace('attached:connect:done');
  try { return await action(browser); }
  finally {
    trace('attached:close:start');
    await browser.close();
    trace('attached:close:done');
    if (browser.pwrightMetrics) assert.deepEqual(browser.pwrightMetrics(), zeroMetrics);
    if (browser.pwrightFfiMetrics) {
      await waitForSnapshot(() => browser.pwrightFfiMetrics().pending === 0);
      assert.deepEqual(browser.pwrightFfiMetrics(), zeroFfiMetrics);
    }
  }
}

test('owned Context creation and close preserve isolation, identities and sibling Pages', async () => {
  const exercise = async browser => {
    const original = browser.contexts();
    const contexts = [];
    let observer;
    try {
      contexts.push(await browser.newContext());
      contexts.push(await browser.newContext({}));
      const pages = await Promise.all(contexts.map(context => context.newPage()));
      await Promise.all(pages.map((page, index) => page.goto(`${fixtureBase}/index.html#owned-${index}`)));
      observer = await oracle.connectOverCDP(endpoint);
      const observedPages = pages.map(page => observer.contexts().flatMap(context => context.pages())
        .find(observed => observed.url() === page.url()));
      assert.ok(observedPages.every(Boolean), 'independent connection must observe both owned Pages');
      await observedPages[0].evaluate(() => localStorage.setItem('context-isolation', 'first'));
      const isolated = await observedPages[1].evaluate(() => localStorage.getItem('context-isolation'));
      const initialized = {
        different: contexts[0] !== contexts[1], count: browser.contexts().length - original.length,
        identity: contexts.every(context => browser.contexts().includes(context)),
        parent: contexts.every(context => context.browser() === browser),
        pages: pages.every((page, index) => contexts[index].pages()[0] === page && page.context() === contexts[index]),
        isolated,
      };
      await Promise.all([contexts[0].close(), contexts[0].close()]);
      await contexts[0].close();
      let closedCreation;
      try { await contexts[0].newPage(); }
      catch (error) { closedCreation = { name: error.name, closed: error.message.includes('closed') }; }
      assert.ok(closedCreation, 'closed Context must reject newPage');
      return { initialized, closedCreation, closedPages: contexts[0].pages().length,
        removed: !browser.contexts().includes(contexts[0]), connected: browser.isConnected(),
        sibling: await pages[1].title(), original: original.every(context => browser.contexts().includes(context)) };
    } finally {
      await observer?.close();
      for (const context of contexts) await context.close();
    }
  };
  // Arrange
  const expected = await withAttachedBrowser(oracle, exercise);
  // Act
  const actual = await withAttachedBrowser(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('closing the default Context disconnects only its attachment', async () => {
  const exercise = async browser => {
    let disconnected = 0;
    browser.on('disconnected', () => { disconnected++; });
    const context = browser.contexts()[0];
    await context.close();
    await context.close();
    return { connected: browser.isConnected(), contexts: browser.contexts().length,
      pages: context.pages().length, disconnected };
  };
  // Arrange
  const run = type => withFixturePage(type, async (_page, browser, fixtureController) => {
    const result = await exercise(browser);
    // Keep the creator's exact Page reference. A different preflight URL must
    // not break this check or let a surviving unrelated Page mask deletion.
    assert.equal(fixtureController.isClosed(), false);
    assert.equal(await fixtureController.title(), fixtureTitle);
    return result;
  }, 'index.html#default-context-close');
  const expected = await run(oracle);
  // Act
  const actual = await run(candidate);
  // Assert
  assert.deepEqual(actual, expected);
});

test('new Context Pages use Playwright default viewport and media settings', async () => {
  const exercise = async browser => {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      await page.goto(`${fixtureBase}/context-defaults.html`);
      const body = (await page.content()).match(/<body>([^<]*)<\/body>/);
      assert.ok(body, 'fixture must capture native defaults before observer attachment');
      return JSON.parse(body[1]);
    } finally { await context.close(); }
  };
  // Arrange
  const expected = await withAttachedBrowser(oracle, exercise);
  // Act
  const actual = await withAttachedBrowser(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

async function exactNativeContext(cdp, url) {
  const { targetInfos } = await cdp.send('Target.getTargets');
  const matches = targetInfos.filter(target => target.type === 'page' && target.url === url);
  assert.equal(matches.length, 1, 'fixture must resolve to exactly one native target');
  assert.ok(matches[0].browserContextId);
  return { id: matches[0].browserContextId, target: matches[0].targetId };
}

async function waitForNativeContextRemoval(cdp, context) {
  for (let turn = 0; turn < 100; turn++) {
    const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
    const { targetInfos } = await cdp.send('Target.getTargets');
    if (!browserContextIds.includes(context.id) && !targetInfos.some(target => target.targetId === context.target)) return;
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  assert.fail('owned native Context or Page survived connection teardown');
}

test('browser disconnect disposes its owned native Context and Page but keeps external Chrome alive', async () => {
  const exercise = async browser => {
    const context = await browser.newContext();
    let observer;
    try {
      const page = await context.newPage();
      await page.goto(`${fixtureBase}/index.html#owned-disconnect`);
      observer = await oracle.connectOverCDP(endpoint);
      const cdp = await observer.newBrowserCDPSession();
      const native = await exactNativeContext(cdp, page.url());
      await browser.close();
      await waitForNativeContextRemoval(cdp, native);
      await context.close();
      await page.close();
      return { contexts: browser.contexts().length, pages: context.pages().length,
        connected: browser.isConnected(), url: page.url(), externalConnected: observer.isConnected() };
    } finally { await observer?.close(); await context.close(); }
  };
  // Arrange
  const expected = await withAttachedBrowser(oracle, exercise);
  // Act
  const actual = await withAttachedBrowser(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('Worker destruction disposes its exact owned native Context and Page', async () => {
  // Arrange
  const observer = await oracle.connectOverCDP(endpoint);
  const url = `${fixtureBase}/index.html#worker-owned-context`;
  const worker = new Worker(new URL('./worker-owned-context.mjs', import.meta.url), {
    workerData: { endpoint, url },
  });
  try {
    await once(worker, 'message', { signal: AbortSignal.timeout(10_000) });
    const cdp = await observer.newBrowserCDPSession();
    const native = await exactNativeContext(cdp, url);
    // Act
    const exit = await worker.terminate();
    // Assert
    assert.equal(exit, 1);
    await waitForNativeContextRemoval(cdp, native);
    assert.ok(observer.isConnected());
  } finally { await worker.terminate(); await observer.close(); }
});

test('unsupported Context options reject before creation or disposal', async () => {
  await withAttachedBrowser(candidate, async browser => {
    // Arrange
    const context = await browser.newContext();
    const before = browser.contexts();
    try {
      // Act
      const results = await Promise.allSettled([
        browser.newContext({ viewport: { width: 640, height: 480 } }),
        context.close({ reason: 'not implemented' }),
      ]);
      // Assert
      assert.deepEqual(results.map(result => result.status), ['rejected', 'rejected']);
      assert.deepEqual(results.map(result => result.reason.code), ['PWRIGHT_UNSUPPORTED', 'PWRIGHT_UNSUPPORTED']);
      assert.deepEqual(browser.contexts(), before);
    } finally { await context.close(); }
  });
});

test('Context.newPage returns an initialized Page and ordinary Page.close is idempotent', async () => {
  const exercise = async context => {
    const before = context.pages().length;
    const page = await context.newPage();
    try {
      const initial = {
        url: page.url(), frameUrl: page.mainFrame().url(), title: await page.title(),
        context: page.context() === context, frame: page.mainFrame().page() === page,
        identity: context.pages().includes(page), added: context.pages().length === before + 1,
      };
      await page.goto(`${fixtureBase}/index.html`);
      const title = await page.title();
      const lastUrl = page.url();
      await Promise.all([page.close(), page.close({ runBeforeUnload: false })]);
      await page.close();
      let closedRead;
      try { await page.content(); }
      catch (error) { closedRead = { name: error.name, closed: error.message.includes('closed') }; }
      assert.ok(closedRead, 'closed Page must reject content reads');
      return { initial, title, removed: context.pages().length === before,
        retainedUrl: page.url() === lastUrl, closedRead };
    } finally { await page.close(); }
  };
  // Arrange
  const expected = await withForeignContextPage(oracle, exercise);
  // Act
  const actual = await withForeignContextPage(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('concurrent Context.newPage calls publish distinct usable Pages and close exactly those Pages', async () => {
  const exercise = async context => {
    const originalPages = context.pages();
    const created = [];
    try {
      const results = await Promise.allSettled([context.newPage(), context.newPage()]);
      for (const result of results) if (result.status === 'fulfilled') created.push(result.value);
      for (const result of results) if (result.status === 'rejected') throw result.reason;
      const pages = context.pages();
      const observed = {
        uniquePages: new Set(created).size,
        uniqueFrames: new Set(created.map(page => page.mainFrame())).size,
        visible: created.every(page => pages.includes(page)),
        originalIdentity: originalPages.every(page => pages.includes(page)),
        titles: await Promise.all(created.map(page => page.title())),
        urls: created.map(page => page.url()),
      };
      await Promise.all(created.map(page => page.close()));
      return { ...observed, removed: created.every(page => !context.pages().includes(page)),
        originalSurvives: originalPages.every(page => context.pages().includes(page)) };
    } finally { await Promise.all(created.map(page => page.close())); }
  };
  // Arrange
  const expected = await withForeignContextPage(oracle, exercise);
  // Act
  const actual = await withForeignContextPage(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('ordinary Page.close resolves after external closure or browser disconnection', async () => {
  const exercise = async (page, browser, controllerPage) => {
    const url = page.url();
    await controllerPage.close();
    await waitForSnapshot(() => !page.context().pages().includes(page));
    await page.close();
    await browser.close();
    await page.close();
    return { retainedUrl: page.url() === url, connected: browser.isConnected() };
  };
  // Arrange
  const expected = await withFixturePage(oracle, exercise);
  // Act
  const actual = await withFixturePage(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('unsupported lifecycle options reject before creating or closing a Page', async () => {
  // Arrange
  await withFixturePage(candidate, async page => {
    const context = page.context();
    const before = context.pages();
    // Act
    const results = await Promise.allSettled([
      context.newPage({}), page.close({ runBeforeUnload: true }),
      page.close({ reason: 'not implemented' }), page.close({ runBeforeUnload: 'false' }),
    ]);
    // Assert
    assert.deepEqual(results.map(result => result.status), Array(4).fill('rejected'));
    assert.deepEqual(results.slice(0, 3).map(result => result.reason.code), Array(3).fill('PWRIGHT_UNSUPPORTED'));
    assert.ok(results[3].reason instanceof TypeError);
    assert.deepEqual(context.pages(), before);
    assert.equal(await page.title(), fixtureTitle);
  });
});

test('concurrent title and content reads on one Frame match Playwright', async () => {
  // Arrange
  const read = page => Promise.all([
    page.title(), page.content(), page.mainFrame().title(), page.mainFrame().content(),
  ]);
  const expected = await withFixturePage(oracle, read);
  // Act
  const actual = await withFixturePage(candidate, read);
  // Assert
  assert.deepEqual(actual, expected);
});

test('URL visibility waits for a JS event-loop turn after external navigation (NODE-009)', async () => {
  const observe = async (page, browser) => {
    const before = page.url();
    if (page.mainFrame().pwrightMetadataMetrics)
      assert.equal(page.mainFrame().pwrightMetadataMetrics().unpublishedUrl, false);
    const signal = new SharedArrayBuffer(4);
    const state = new Int32Array(signal);
    const worker = new Worker(new URL('./worker-url-publication.mjs', import.meta.url), {
      workerData: { endpoint, signal, fixtureUrl: page.url() },
    });
    try {
      await once(worker, 'message', { signal: AbortSignal.timeout(10_000) });
      worker.postMessage('navigate');
      Atomics.wait(state, 0, 0, 10_000);
      assert.equal(Atomics.load(state, 0), 1, 'external navigation must complete independently of this JS loop');
      if (browser.pwrightMetadataMetrics) {
        // Native receipt is proven, but no Node callback can publish it yet.
        const deadline = Date.now() + 5000;
        while (!page.mainFrame().pwrightMetadataMetrics().unpublishedUrl) {
          assert.ok(Date.now() < deadline, 'native publication was not queued');
          Atomics.wait(state, 0, 1, 10);
        }
        assert.ok(browser.pwrightMetadataMetrics().pendingPublications > 0);
      }
      const withinTurn = { page: page.url() === before, frame: page.mainFrame().url() === before };
      await waitForSnapshot(() => page.url().endsWith('#blocked-publication'));
      return { withinTurn, afterTurn: page.url() === page.mainFrame().url() };
    } finally { await worker.terminate(); }
  };
  // Arrange
  const expected = await withFixturePage(oracle, observe);
  assert.deepEqual(expected, { withinTurn: { page: true, frame: true }, afterTurn: true });
  // Act
  const actual = await withFixturePage(candidate, observe);
  // Assert
  assert.deepEqual(actual, expected);
});

test('blocked JS cannot admit unbounded native settlements, including errors', async () => {
  await withFixturePage(candidate, async (page, browser, fixtureController) => {
    // Arrange
    await fixtureController.evaluate(() => { document.body.textContent = '界'.repeat(280_000); });
    const expectedBytes = Buffer.byteLength(await page.content());
    assert.ok(expectedBytes > 800_000 && expectedBytes < 1024 * 1024);
    // Act
    const pending = queueReadsWithoutDelivering(page, browser);
    blockUntilResultsAreQueued(browser);
    const outcomes = await Promise.all(pending);
    // Assert
    assert.ok(outcomes.some(value => value.ok && value.bytes === expectedBytes));
    assert.ok(outcomes.filter(value => value.code === 'PWRIGHT_RESOURCE_EXHAUSTED').length >= 32);
    assert.ok(outcomes.every(value => value.ok || value.code === 'PWRIGHT_RESOURCE_EXHAUSTED'));
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
  });
});

test('Worker destruction discards queued URL publications without JS delivery', async () => {
  await withFixturePage(oracle, async (_page, _browser, fixtureController) => {
    // Arrange
    const before = await readTopology(oracle, { endpointURL: endpoint });
    const signal = new SharedArrayBuffer(4);
    const state = new Int32Array(signal);
    const worker = new Worker(new URL('./worker-metadata-pressure.mjs', import.meta.url), {
      workerData: { endpoint, signal, fixtureUrl: fixtureController.url() },
    });
    try {
      await once(worker, 'message', { signal: AbortSignal.timeout(10_000) });
      const queued = once(worker, 'message', { signal: AbortSignal.timeout(10_000) });
      worker.postMessage('block');
      Atomics.wait(state, 0, 0, 5000);
      assert.equal(Atomics.load(state, 0), 1);
      await fixtureController.evaluate(() => { history.replaceState(null, '', '#worker-unpublished'); });
      Atomics.store(state, 0, 2);
      Atomics.notify(state, 0);
      const [metrics] = await queued;
      assert.ok(metrics.pendingPublications > 0 && metrics.pendingPublications <= 256);
      // Act
      const exit = await worker.terminate();
      // Assert
      assert.equal(exit, 1);
      assert.deepEqual(await readTopology(candidate, { endpointURL: endpoint }), before);
    } finally { await worker.terminate(); }
  });
});

test('close has reserved admission when all command settlement slots are full', async () => {
  await withFixturePage(candidate, async (page, browser) => {
    // Arrange
    const pending = queueReadsWithoutDelivering(page, browser);
    // Act
    const close = browser.close();
    assert.equal(browser.pwrightFfiMetrics().closes, 1);
    await close;
    await Promise.all(pending);
    // Assert
    assert.deepEqual(browser.pwrightMetrics(), zeroMetrics);
  });
});

test('oversized UTF-8 arguments release admission without starting native work', async () => {
  await withFixturePage(candidate, async (page, browser) => {
    // Arrange
    const before = browser.pwrightFfiMetrics();
    const oldUrl = page.url();
    // Act
    const result = page.goto(`https://example.invalid/${'界'.repeat(6000)}`);
    // Assert
    await assert.rejects(result, { name: 'TypeError', code: 'PWRIGHT_INVALID_ARGUMENT' });
    assert.deepEqual(browser.pwrightFfiMetrics(), before);
    assert.equal(page.url(), oldUrl);
  });
});

test('Worker destruction reclaims known queued native results without JS delivery', async () => {
  await withFixturePage(oracle, async page => {
    // Arrange
    const before = await readTopology(oracle, { endpointURL: endpoint });
    const worker = new Worker(new URL('./worker-ffi-pressure.mjs', import.meta.url), {
      workerData: { endpoint, fixtureUrl: page.url() },
    });
    try {
      const [queued] = await once(worker, 'message', { signal: AbortSignal.timeout(20_000) });
      assert.equal(queued.commands, 32);
      assert.equal(queued.queuedCommands, 32);
      // Act
      const exit = await worker.terminate();
      // Assert
      assert.equal(exit, 1);
      assert.deepEqual(await readTopology(candidate, { endpointURL: endpoint }), before);
    } finally { await worker.terminate(); }
  });
});

for (const waitUntil of ['commit', 'domcontentloaded', 'load', 'networkidle']) {
  test(`goto waitUntil=${waitUntil} returns matching response metadata`, async () => {
    // Arrange
    const navigate = async page => {
      const response = await page.goto(`${fixtureBase}/second.html`, { waitUntil, timeout: 5000 });
      return { url: response.url(), status: response.status(), statusText: response.statusText(),
        pageUrl: page.url(), frameUrl: page.mainFrame().url(),
        ok: response.ok(), headers: response.headers(), frameIdentity: response.frame() === page.mainFrame() };
    };
    const expected = await withFixturePage(oracle, navigate);
    // Act
    const actual = await withFixturePage(candidate, navigate);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

test('document replacement keeps the main Frame usable for native reads', async () => {
  // Arrange
  const navigateAndRead = async page => {
    const frame = page.mainFrame();
    await page.goto(`${fixtureBase}/second.html`);
    return { title: await page.title(), content: await frame.content(), identity: frame === page.mainFrame() };
  };
  const expected = await withFixturePage(oracle, navigateAndRead);
  // Act
  const actual = await withFixturePage(candidate, navigateAndRead);
  // Assert
  assert.deepEqual(actual, expected);
});

test('same-document and about:blank navigations publish URLs and return null responses', async () => {
  // Arrange
  const navigate = async page => {
    const sameDocument = await page.goto(`${fixtureBase}/index.html#section`);
    const sameDocumentUrl = page.url();
    const blank = await page.goto('about:blank');
    return { sameDocument, sameDocumentUrl, blank, blankUrl: page.mainFrame().url() };
  };
  const expected = await withFixturePage(oracle, navigate);
  // Act
  const actual = await withFixturePage(candidate, navigate);
  // Assert
  assert.deepEqual(actual, expected);
});

test('initial Page and main Frame URLs include an existing fragment synchronously', async () => {
  // Arrange
  const read = page => ({ page: page.url(), frame: page.mainFrame().url() });
  const expected = await withFixturePage(oracle, read, 'index.html#initial');
  // Act
  const actual = await withFixturePage(candidate, read, 'index.html#initial');
  // Assert
  assert.deepEqual(actual, expected);
  assert.equal(actual.page, `${fixtureBase}/index.html#initial`);
});

async function waitForSnapshot(predicate) {
  const deadline = Date.now() + 5000;
  while (!predicate()) {
    assert.ok(Date.now() < deadline, 'native event-driven snapshot did not converge');
    await new Promise(resolve => setTimeout(resolve, 10));
  }
}

for (const action of ['document', 'pushState', 'replaceState', 'hash']) {
  test(`external ${action} navigation updates both retained URL readers`, async () => {
    // Arrange
    const observe = async (page, _browser, controller) => {
      const frame = page.mainFrame();
      const destination = action === 'document' ? `${fixtureBase}/second.html#external` : `${fixtureBase}/index.html#${action}`;
      if (action === 'document') await controller.goto(destination);
      else await controller.evaluate(({ action, destination }) => {
        if (action === 'hash') location.hash = '#hash';
        else history[action]({}, '', destination);
      }, { action, destination });
      // A separate client has its own event delivery order. No candidate
      // operation is issued to refresh this purely synchronous native state.
      await waitForSnapshot(() => page.url() === destination);
      return { page: page.url(), frame: frame.url(), identity: frame === page.mainFrame() };
    };
    const expected = await withFixturePage(oracle, observe);
    // Act
    const actual = await withFixturePage(candidate, observe);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

test('browser disconnect freezes URL metadata without retaining live resources', async () => {
  // Arrange
  const observe = async (page, browser, controller) => {
    const frame = page.mainFrame();
    await page.goto(`${fixtureBase}/second.html#last`);
    await browser.close();
    await controller.goto(`${fixtureBase}/index.html#after-disconnect`);
    if (browser.pwrightMetadataMetrics) {
      const metadata = browser.pwrightMetadataMetrics();
      assert.equal(metadata.activeFrames, 0);
      assert.ok(metadata.entries >= 1 && metadata.entries <= 128);
      assert.ok(metadata.urlBytes > 0 && metadata.urlBytes <= 2 * 1024 * 1024);
      assert.deepEqual(browser.pwrightMetrics(), zeroMetrics);
    }
    return { page: page.url(), frame: frame.url() };
  };
  const expected = await withFixturePage(oracle, observe);
  // Act
  const actual = await withFixturePage(candidate, observe);
  // Assert
  assert.deepEqual(actual, expected);
  assert.equal(actual.page, `${fixtureBase}/second.html#last`);
});

test('externally closed Page retains its last Frame URL after topology retirement', async () => {
  // Arrange
  const observe = async (page, _browser, controller) => {
    const frame = page.mainFrame();
    await page.goto(`${fixtureBase}/second.html#closed`);
    await controller.close();
    await waitForSnapshot(() => !page.context().pages().includes(page));
    return { page: page.url(), frame: frame.url() };
  };
  const expected = await withFixturePage(oracle, observe);
  // Act
  const actual = await withFixturePage(candidate, observe);
  // Assert
  assert.deepEqual(actual, expected);
});

test('reads on retained wrappers reject after browser close', async () => {
  // Arrange
  const readClosed = async (page, browser) => {
    await browser.close();
    try { await page.content(); }
    catch (error) { return { name: error.name, closed: error.message.includes('closed') }; }
    throw new Error('closed-page read unexpectedly succeeded');
  };
  const expected = await withFixturePage(oracle, readClosed);
  // Act
  const actual = await withFixturePage(candidate, readClosed);
  // Assert
  assert.deepEqual(actual, expected);
});

async function withStalledResponse(action) {
  // Dynamic protocol fixture only; static HTML stays on serve-lib.
  let received;
  const request = new Promise(resolve => { received = resolve; });
  const server = createServer((_request, response) => {
    response.once('close', () => server.emit('fixture-response-closed'));
    received();
  });
  server.listen(0, fixtureBind);
  await once(server, 'listening');
  const url = `http://${new URL(fixtureBase).hostname}:${server.address().port}/hold`;
  const waitForClose = () => once(server, 'fixture-response-closed', { signal: AbortSignal.timeout(5000) });
  try { return await action(url, request, waitForClose); }
  finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}

test('goto load waits for its committed document body while independent Rust-backed reads run', async () => {
  const exercise = browser => withNavigationResponseGate(fixtureBind, new URL(fixtureBase).hostname,
    async ({ url, started, release }) => {
      const context = await browser.newContext();
      try {
        const page = await context.newPage();
        let settled = false;
        const navigation = page.goto(url, { waitUntil: 'load', timeout: 5000 }).then(
          response => { settled = true; return { response }; },
          error => { settled = true; return { error }; },
        );
        await Promise.race([started, navigation.then(() => { throw new Error('Navigation ended before the held response arrived'); })]);
        // Observe the native published URL, not an observer connection's
        // independently delivered events. No candidate operation refreshes it.
        await waitForSnapshot(() => page.url() === url);
        const before = { title: await page.title(), state: await page.evaluate(() => document.readyState), settled };
        release();
        const outcome = await navigation;
        if (outcome.error) throw outcome.error;
        return { before, status: outcome.response.status(), state: await page.evaluate(() => document.readyState) };
      } finally { await context.close(); }
    });
  // Arrange
  const expected = await withAttachedBrowser(oracle, exercise);
  assert.deepEqual(expected, { before: { title: 'Caller-held document', state: 'loading', settled: false }, status: 200, state: 'complete' });
  // Act
  const actual = await withAttachedBrowser(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('stalled HTTP navigation rejects with the upstream TimeoutError class', async () => {
  // Arrange
  const timedOut = page => withStalledResponse(async url => {
    try { await page.goto(url, { timeout: 500 }); }
    catch (error) { return { name: error.name, timeout: error.message.includes('500ms') }; }
    throw new Error('stalled navigation unexpectedly finished');
  });
  const expected = await withFixturePage(oracle, timedOut);
  // Act
  const actual = await withFixturePage(candidate, timedOut);
  // Assert
  assert.deepEqual(actual, expected);
});

test('browser close cancels in-flight navigation and drains cleanup before resolving', async () => {
  // Arrange
  const cancel = (page, browser) => withStalledResponse(async (url, arrived) => {
    const navigation = page.goto(url, { timeout: 5000 }).then(
      () => { throw new Error('stalled navigation unexpectedly finished'); },
      error => ({ name: error.name, closed: error.message.includes('closed') }),
    );
    await arrived;
    await browser.close();
    return await navigation;
  });
  const expected = await withFixturePage(oracle, cancel);
  // Act
  const actual = await withFixturePage(candidate, cancel);
  // Assert
  assert.deepEqual(actual, expected);
});

test('ordinary Page.close cancels its in-flight navigation while keeping the browser connected', async () => {
  const cancel = (page, browser) => withStalledResponse(async (url, arrived, waitForClose) => {
    const navigation = page.goto(url, { timeout: 5000 }).then(
      () => { throw new Error('stalled navigation unexpectedly finished'); },
      error => ({ name: error.name, closed: error.message.includes('closed') }),
    );
    await arrived;
    const responseClosed = waitForClose();
    await page.close();
    await responseClosed;
    return { navigation: await navigation, connected: browser.isConnected(),
      removed: !page.context().pages().includes(page) };
  });
  // Arrange
  const expected = await withFixturePage(oracle, cancel);
  // Act
  const actual = await withFixturePage(candidate, cancel);
  // Assert
  assert.deepEqual(actual, expected);
});

test('owned Context close cancels descendant navigation without disconnecting the browser', async () => {
  const exercise = browser => withStalledResponse(async (url, arrived, waitForClose) => {
    const context = await browser.newContext();
    try {
      const page = await context.newPage();
      const navigation = page.goto(url, { timeout: 5000 }).then(
        () => { throw new Error('stalled navigation unexpectedly finished'); },
        error => ({ name: error.name, closed: error.message.includes('closed') }),
      );
      await arrived;
      const responseClosed = waitForClose();
      await context.close();
      await responseClosed;
      return { navigation: await navigation, pages: context.pages().length,
        connected: browser.isConnected(), removed: !browser.contexts().includes(context) };
    } finally { await context.close(); }
  });
  // Arrange
  const expected = await withAttachedBrowser(oracle, exercise);
  // Act
  const actual = await withAttachedBrowser(candidate, exercise);
  // Assert
  assert.deepEqual(actual, expected);
});

test('Worker destruction cancels a pending native navigation even while its Promise retains Rust state', async () => {
  // Arrange
  const result = await withFixturePage(oracle, page => withStalledResponse(async (url, arrived, waitForClose) => {
    const worker = new Worker(new URL('./worker-navigation.mjs', import.meta.url), {
      workerData: { endpoint, url, fixtureUrl: page.url() },
    });
    try {
      await Promise.race([arrived, once(worker, 'error').then(([error]) => { throw error; })]);
      const closed = waitForClose();
      // Act
      const code = await worker.terminate();
      await closed;
      return code;
    } finally { await worker.terminate(); }
  }));
  // Assert
  assert.equal(result, 1);
});

test('title ignores main-world Document.prototype overrides (NODE-010 regression)', async () => {
  // Arrange
  const read = async (page, _browser, setupPage) => {
    await setupPage.evaluate(() => Object.defineProperty(Document.prototype, 'title', {
      get() { return 'page-world-intercepted-title'; }, configurable: true,
    }));
    return await page.title();
  };
  const expected = await withFixturePage(oracle, read);
  // Act
  const actual = await withFixturePage(candidate, read);
  // Assert
  assert.equal(actual, expected);
});
