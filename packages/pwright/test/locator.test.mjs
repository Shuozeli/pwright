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
assert.equal(endpoint, 'http://127.0.0.1:9222', 'run the dedicated Chrome/Pwright preflight');
assert.ok(fixtureBase, 'use the verified serve-lib fixture route');

async function withPage(type, action) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(`${fixtureBase}/locator.html`);
    return await action(page, browser, context);
  } finally {
    try { await context?.close(); } finally { await browser.close(); }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().liveObjects, 0);
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

paired('CSS queries are lazy values and preserve Page identity', async page => {
  const query = page.locator('.item');
  const filtered = query.filter();
  return { samePage: query.page() === page, distinct: filtered !== query,
    count: await query.count(), frameCount: await page.mainFrame().locator('.item').count(),
    missing: await page.locator('#not-present').count() };
});

paired('Locator collections and nth retain order and exact text', async page => {
  const items = page.locator('.item');
  return { texts: await items.allTextContents(), inner: await items.allInnerTexts(),
    first: await items.first().getAttribute('data-key'), last: await items.last().getAttribute('data-key'),
    second: await items.nth(1).textContent(), outside: await items.nth(99).count() };
});

paired('Locator scalar reads preserve empty and missing attributes and Unicode', async page => {
  const content = page.locator('#content');
  return { text: await content.textContent(), inner: await content.innerText(), html: await content.innerHTML(),
    empty: await content.getAttribute('data-present'), missing: await content.getAttribute('data-missing'),
    unicode: await content.getAttribute('data-unicode'), input: await page.locator('#input').inputValue() };
});

paired('Locator state reads match visible hidden disabled editable and checked states', async page => ({
  visible: await page.locator('#content').isVisible(), hidden: await page.locator('.hidden').isHidden(),
  absent: await page.locator('#missing').isVisible({ timeout: 1 }),
  absentHidden: await page.locator('#missing').isHidden(),
  disabled: await page.locator('#disabled').isDisabled(), enabled: await page.locator('#input').isEnabled(),
  editable: await page.locator('#input').isEditable(), notEditable: await page.locator('#disabled').isEditable(),
  checked: await page.locator('#checked').isChecked(), unchecked: await page.locator('#unchecked').isChecked(),
}));

paired('Locator and or descendants and has filters compose without changing their bases', async page => {
  const items = page.locator('.item');
  return { and: await items.and(page.locator('.first')).allTextContents(),
    or: await page.locator('.first').or(page.locator('.hidden')).allTextContents(),
    within: await page.locator('#list').locator('li').count(),
    locatorWithin: await page.locator('#list').locator(page.locator('li')).count(),
    has: await items.filter({ has: page.locator('span'), hasText: 'Beta' }).allTextContents(),
    hasNot: await items.filter({ hasNot: page.locator('strong') }).count(),
    notText: await items.filter({ hasNotText: 'Beta' }).count(), visible: await items.filter({ visible: true }).count(),
    emptyExclusion: await items.filter({ hasNotText: '' }).count(), unchanged: await items.count() };
});

paired('CSS queries pierce an open shadow root', async page => ({
  count: await page.locator('.shadow-button').count(), text: await page.locator('.shadow-button').textContent(),
}));

const semantic = exercise => async page => {
  await page.goto(`${fixtureBase}/semantic-locators.html`);
  return exercise(page);
};

paired('getByText normalizes whitespace and distinguishes exact strings from regular expressions', semantic(async page => ({
  exact: await page.getByText('Hello 中文 world', { exact: true }).getAttribute('id'),
  substring: await page.getByText('HELLO 中文').getAttribute('id'),
  regex: await page.getByText(/hello.*中文/i).getAttribute('id'),
  missing: await page.getByText('hello 中文 world', { exact: true }).count(),
  nested: await page.getByText('file', { exact: true }).getAttribute('id'),
})));

paired('getByLabel follows for wrapping aria-label and aria-labelledby associations', semantic(async page => ({
  for: await page.getByLabel('E-mail address', { exact: true }).inputValue(),
  wrapped: await page.getByLabel('Wrapped label').inputValue(),
  aria: await page.getByLabel(/^aria name$/i).inputValue(),
  nickname: await page.mainFrame().getByLabel('Nickname').inputValue(),
  shadow: await page.getByLabel('Shadow label').inputValue(),
})));

paired('attribute and test-id queries preserve their distinct exactness and regex contracts', semantic(async page => ({
  placeholder: await page.getByPlaceholder(/E-MAIL/i).inputValue(),
  alt: await page.getByAltText('small CAT').getAttribute('id'),
  title: await page.getByTitle('More information', { exact: true }).textContent(),
  titleRegex: await page.getByTitle(/shadow title/i).textContent(),
  id: await page.getByTestId('Exact ID').getAttribute('id'),
  idWrongCase: await page.getByTestId('EXACT ID').count(),
  ids: await page.getByTestId(/^exact id$/i).count(),
})));

paired('getByRole applies names states levels and hidden filtering in the existing Rust selector runtime', semantic(async page => ({
  name: await page.getByRole('button', { name: /^save file$/i }).count(),
  exact: await page.getByRole('button', { name: 'save file', exact: true }).count(),
  checked: await page.getByRole('checkbox', { checked: true }).getAttribute('id'),
  disabled: await page.getByRole('button', { disabled: true }).getAttribute('id'),
  pressed: await page.getByRole('button', { pressed: true }).getAttribute('id'),
  expanded: await page.getByRole('button', { expanded: true }).getAttribute('id'),
  selected: await page.getByRole('tab', { selected: true }).getAttribute('id'),
  level: await page.getByRole('heading', { level: 2 }).getAttribute('id'),
  hiddenDefault: await page.getByRole('button', { name: 'Hidden action' }).count(),
  hidden: await page.getByRole('button', { name: 'Hidden action', includeHidden: true }).getAttribute('id'),
})));

paired('semantic Locators compose through Page Frame and relative Locator roots', semantic(async page => {
  const group = page.locator('#first');
  const queries = { text: group.getByText('Save file', { exact: true }),
    role: group.getByRole('button', { name: 'Save file' }), label: group.getByLabel('Nickname'),
    title: group.getByTitle('More information'), alt: group.getByAltText('cat'),
    placeholder: group.getByPlaceholder('e-mail'), id: group.getByTestId('save'),
    and: page.getByRole('button').and(page.getByTestId('save')),
    has: page.locator('section').filter({ has: page.getByLabel('Nickname') }),
    shadow: page.mainFrame().getByRole('button', { name: 'Shadow save' }) };
  const values = {};
  for (const [name, query] of Object.entries(queries)) {
    assert.equal(await query.count(), 1, `${name} must resolve exactly one fixture element`);
    values[name] = await query.getAttribute('id', { timeout: 5000 });
  }
  return values;
}));

paired('semantic recipes re-resolve replacement elements and can perform native input', semantic(async page => {
  const query = page.getByTestId('save');
  await page.evaluate(() => { document.querySelector('#save').outerHTML = '<button id="replacement" data-testid="save" onclick="this.textContent=\'clicked\'">replacement</button>'; });
  await query.click();
  return { id: await query.getAttribute('id'), text: await query.textContent(), identity: query.page() === page };
}));

paired('accessible names prefer permitted role content over title while retaining explicit labels and empty-content fallback', semantic(async page => ({
  button: await page.getByRole('button', { name: 'Visible name', exact: true }).getAttribute('id', { timeout: 5000 }),
  fallback: await page.getByRole('button', { name: 'Fallback name' }).getAttribute('id', { timeout: 5000 }),
  aria: await page.getByRole('button', { name: 'ARIA winner' }).getAttribute('id', { timeout: 5000 }),
  link: await page.getByRole('link', { name: 'Visible link' }).getAttribute('id', { timeout: 5000 }),
  role: await page.getByRole('button', { name: 'Role content' }).getAttribute('id', { timeout: 5000 }),
  tooltipNotName: await page.getByRole('button', { name: 'Wrong tooltip' }).count(),
})));

paired('invalid role filters fail on use rather than while creating a lazy Locator', semantic(async page => {
  let query;
  try { query = page.getByRole('link', { pressed: true }); }
  catch (error) { return { phase: 'construction', name: error.name }; }
  try { await query.count(); return { phase: 'unexpected-success' }; }
  catch (error) { return { phase: 'operation', name: error.name }; }
}));

paired('global role-name regex matches every sibling and does not mutate the caller RegExp', semantic(async page => {
  const pattern = /run/gi;
  pattern.lastIndex = 2;
  const query = page.locator('#regex').getByRole('button', { name: pattern });
  return { first: await query.count(), second: await query.count(), lastIndex: pattern.lastIndex };
}));

test('semantic input rejection returns query capacity without dispatch or lossy Unicode conversion', async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const before = browser.pwrightLocatorMetrics();
    // Act
    const invalid = () => page.getByText('\ud800');
    // Assert
    assert.throws(invalid, { code: 'PWRIGHT_UNSUPPORTED' });
    assert.deepEqual(browser.pwrightLocatorMetrics(), before);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    assert.throws(() => page.getByRole('button', { description: 'not in pinned API' }), { code: 'PWRIGHT_UNSUPPORTED' });
    assert.deepEqual(browser.pwrightLocatorMetrics(), before);
  });
});

paired('retained Locator re-resolves replaced nodes and documents', async page => {
  const query = page.locator('#mutable');
  const before = await query.textContent();
  await page.evaluate(() => { document.querySelector('#mutable').outerHTML = '<div id="mutable">after</div>'; });
  const after = await query.textContent();
  await page.goto(`${fixtureBase}/locator.html`);
  return { before, after, navigated: await query.textContent() };
});

paired('single-element reads reject strictness violations rather than choosing the first match', async page => {
  try { await page.locator('.item').textContent({ timeout: 200 }); return { resolved: true }; }
  catch (error) { return { name: error.name, strict: /strict/i.test(error.message) }; }
});

paired('missing single-element reads reject with TimeoutError', async page => {
  try { await page.locator('#missing').textContent({ timeout: 100 }); return { resolved: true }; }
  catch (error) { return { name: error.name }; }
});

paired('empty selector errors occur on use rather than Locator construction', async page => {
  const query = page.locator('');
  try { await query.count(); return { resolved: true }; }
  catch (error) { return { name: error.name }; }
});

paired('cross-Frame composition rejects during construction', async (page, _browser, context) => {
  const sibling = await context.newPage();
  try {
    try { page.locator('body').and(sibling.locator('body')); return { combined: true }; }
    catch (error) { return { name: error.name, sameFrame: /same frame/i.test(error.message) }; }
  } finally { await sibling.close(); }
});

paired('cross-connection Locators reject even when they address the same native Page', async (page, browser) => {
  await page.goto(`${fixtureBase}/locator.html#cross-connection`);
  const type = browser.pwrightMetrics ? candidate : oracle;
  const other = await type.connectOverCDP(endpoint, { timeout: 5000 });
  try {
    const matches = other.contexts().flatMap(context => context.pages()).filter(value => value.url() === page.url());
    assert.equal(matches.length, 1);
    try { page.locator('body').and(matches[0].locator('body')); return { combined: true }; }
    catch (error) { return { name: error.name, sameFrame: /same frame/i.test(error.message) }; }
  } finally { await other.close(); }
});

paired('Page close cancels a pending Locator read without disconnecting its Browser', async (page, browser) => {
  const query = page.locator('#missing');
  let settled = false;
  const pending = query.textContent({ timeout: 30000 }).then(
    () => { settled = true; return { resolved: true }; },
    error => { settled = true; return { name: error.name, message: error.message }; },
  );
  // Observe admission without queuing another command behind the pending
  // selector operation. The oracle channel receives read before close; for
  // Rust additionally require the native operation token to be present.
  const deadline = Date.now() + 5000;
  do { await setImmediate(); }
  while (browser.pwrightMetrics && browser.pwrightMetrics().activeOperations === 0 && Date.now() < deadline);
  assert.equal(settled, false, 'close must interrupt a genuinely pending read');
  if (browser.pwrightMetrics) assert.ok(browser.pwrightMetrics().activeOperations > 0);
  await page.close();
  const outcome = await pending;
  assert.match(outcome.message, /closed/i, 'pending Locator read must report target closure');
  return { outcome: { name: outcome.name, closed: true }, connected: browser.isConnected(), samePage: query.page() === page };
});

paired('pure Locator construction survives browser close but later reads reject', async (page, browser) => {
  await browser.close();
  const query = page.locator('body');
  try { await query.count(); return { resolved: true }; }
  catch (error) { return { name: error.name, closed: /closed/i.test(error.message), page: query.page() === page }; }
});

paired('a same-Frame title read completes while a Locator is waiting for an element (NODE-020)', async page => {
  const waiting = page.locator('#missing').textContent({ timeout: 3000 }).then(
    () => ({ completed: 'waiting-read', name: 'resolved' }),
    error => ({ completed: 'waiting-read', name: error.name }),
  );
  const title = page.title().then(value => ({ completed: 'title', value }));
  const first = await Promise.race([waiting, title]);
  const [end, titleResult] = await Promise.all([waiting, title]);
  return { first: first.completed, timeout: end.name, title: titleResult.value };
});

async function collectUntil(browser, expected) {
  assert.equal(typeof global.gc, 'function', 'the Locator test supervisor must enable explicit test-only GC');
  const deadline = Date.now() + 5000;
  do {
    global.gc();
    await setImmediate();
    if (browser.pwrightLocatorMetrics().recipes === expected) return;
  } while (Date.now() < deadline);
  assert.equal(browser.pwrightLocatorMetrics().recipes, expected, 'Node finalization must reclaim query recipes');
}

paired('evaluate can create an element while its same-Frame Locator read is waiting', async page => {
  const text = page.locator('#created-during-wait').textContent({ timeout: 5000 });
  // Attach rejection handling immediately even when another command fails.
  const observed = text.then(value => ({ value }), error => ({ error: error.name }));
  await page.evaluate(() => {
    const element = document.createElement('div');
    element.id = 'created-during-wait';
    element.textContent = 'created concurrently';
    document.body.append(element);
  });
  return observed;
});

paired('independent same-Frame Locator reads complete concurrently without changing their values', async page => {
  const query = page.locator('.item');
  return Promise.all([
    query.count(), query.allTextContents(), query.first().textContent(),
    query.last().getAttribute('data-key'), page.locator('#content').isVisible(),
    page.title(), page.content(),
  ]);
});

test('Page close drains concurrent and queued Locator reads under full command capacity', async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const failures = [];
    const reads = Array.from({ length: 10 }, () => page.locator('#missing').textContent({ timeout: 30000 }).then(
      () => ({ resolved: true }), error => {
        failures.push({ code: error.code, message: error.message });
        return { code: error.code, name: error.name };
      },
    ));
    const deadline = Date.now() + 5000;
    while (browser.pwrightMetrics().activeOperations < 10 && Date.now() < deadline) await setImmediate();
    assert.equal(browser.pwrightMetrics().activeOperations, 10);
    assert.equal(browser.pwrightFfiMetrics().commands, 10);
    assert.ok(browser.pwrightFfiMetrics().reservedBytes <= 64 * 1024 * 1024);
    // Act
    await page.close();
    const results = await Promise.all(reads);
    // Assert
    for (const result of results) assert.deepEqual(result,
      { code: 'PWRIGHT_TARGET_CLOSED', name: 'Error' }, JSON.stringify(failures));
    assert.equal(browser.pwrightMetrics().activeOperations, 0);
    assert.equal(browser.pwrightMetrics().pendingCalls, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    assert.equal(browser.isConnected(), true);
  });
});

test('query pressure is bounded before browser work and GC returns its independent budget', async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    await collectUntil(browser, 0);
    const values = [];
    let failure;
    // Act
    for (let index = 0; index < 1100; index++) {
      try { values.push(page.locator('body')); }
      catch (error) { failure = error; break; }
    }
    // Assert
    assert.equal(failure?.code, 'PWRIGHT_RESOURCE_EXHAUSTED');
    assert.equal(values.length, 1024);
    assert.equal(browser.pwrightLocatorMetrics().recipes, 1024);
    assert.ok(browser.pwrightLocatorMetrics().bytes <= 8 * 1024 * 1024);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    values.length = 0;
    await collectUntil(browser, 0);
    assert.deepEqual(browser.pwrightLocatorMetrics(), { recipes: 0, bytes: 0 });
  });
});

test('invalid filter options reject without allocating another native recipe', async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    const query = page.locator('body');
    const before = browser.pwrightLocatorMetrics();
    // Act
    const outcome = () => query.filter({ unsupported: true });
    // Assert
    assert.throws(outcome, { code: 'PWRIGHT_UNSUPPORTED' });
    assert.deepEqual(browser.pwrightLocatorMetrics(), before);
  });
});

test('large lazy selectors reach the byte budget before the recipe-count limit', async () => {
  await withPage(candidate, async (page, browser) => {
    // Arrange
    await collectUntil(browser, 0);
    const source = 'x'.repeat(64 * 1024);
    const values = [];
    let failure;
    // Act
    for (let index = 0; index < 1024; index++) {
      try { values.push(page.locator(source)); }
      catch (error) { failure = error; break; }
    }
    // Assert
    assert.equal(failure?.code, 'PWRIGHT_RESOURCE_EXHAUSTED');
    assert.ok(values.length > 0 && values.length < 1024);
    const metrics = browser.pwrightLocatorMetrics();
    assert.equal(metrics.recipes, values.length);
    assert.ok(metrics.bytes > 7 * 1024 * 1024 && metrics.bytes <= 8 * 1024 * 1024);
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    values.length = 0;
    await collectUntil(browser, 0);
    assert.deepEqual(browser.pwrightLocatorMetrics(), { recipes: 0, bytes: 0 });
  });
});

test('Worker teardown cancels a pending Locator read and removes only its owned native Context', async () => {
  // Arrange
  const observer = await oracle.connectOverCDP(endpoint);
  const url = `${fixtureBase}/locator.html#locator-worker`;
  const worker = new Worker(new URL('./worker-locator.mjs', import.meta.url), { workerData: { endpoint, url } });
  try {
    const [metrics] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
    assert.ok(metrics.recipes >= 2 && metrics.activeOperations >= 1);
    const cdp = await observer.newBrowserCDPSession();
    const { targetInfos } = await cdp.send('Target.getTargets');
    const targets = targetInfos.filter(target => target.type === 'page' && target.url === url);
    assert.equal(targets.length, 1);
    const target = targets[0];
    assert.ok(target.browserContextId);
    // Act
    const exit = await worker.terminate();
    // Assert
    assert.equal(exit, 1);
    const deadline = Date.now() + 5000;
    let removed = false;
    do {
      const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
      const { targetInfos: after } = await cdp.send('Target.getTargets');
      removed = !browserContextIds.includes(target.browserContextId) && !after.some(page => page.targetId === target.targetId);
      if (removed) break;
      await setImmediate();
    } while (Date.now() < deadline);
    assert.ok(removed, 'exact Worker-owned native Context and Page must disappear');
    assert.ok(observer.isConnected());
  } finally { await worker.terminate(); await observer.close(); }
});
