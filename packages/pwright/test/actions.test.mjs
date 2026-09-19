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
assert.ok(fixtureBase);

async function withPage(type, action) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(`${fixtureBase}/actions.html`);
    return await action(page, browser, context);
  } finally {
    try { await context?.close(); } finally { await browser.close(); }
    if (browser.pwrightMetrics) {
      const metrics = browser.pwrightMetrics();
      assert.equal(metrics.activeLeases, 0); assert.equal(metrics.activeOperations, 0);
      assert.equal(metrics.cleanupInFlight, 0); assert.equal(metrics.pendingCalls, 0);
      assert.equal(browser.pwrightFfiMetrics().commands, 0);
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
const recorded = page => page.evaluate(() => events.filter(event => !['mouseenter','focus','blur'].includes(event.type)));

paired('click uses trusted native input and returns undefined', async page => {
  const result = await page.locator('#button').click();
  return { result, events: await recorded(page) };
});
paired('click supports button count position and temporary modifiers', async page => {
  await page.locator('#button').click({ clickCount: 2, position: { x: 7, y: 11 }, modifiers: ['Shift'], delay: 5 });
  await page.locator('#button').click({ button: 'right' });
  return recorded(page);
});
paired('dblclick produces the browser double-click sequence', async page => {
  await page.locator('#button').dblclick(); return recorded(page);
});
paired('trial click has no pointer click or checkbox mutation', async page => {
  await page.locator('#button').click({ trial: true });
  await page.locator('#check').check({ trial: true });
  return { clicked: await page.evaluate(() => events.filter(event => event.type === 'click')), checked: await page.locator('#check').isChecked() };
});
paired('Locator actions preserve strictness', async page => {
  try { await page.locator('.duplicate').click({ timeout: 200 }); return 'resolved'; }
  catch (error) { return { name: error.name, strict: /strict/i.test(error.message) }; }
});
paired('disabled click times out and force does not synthesize a click event', async page => {
  let name;
  try { await page.locator('#disabled').click({ timeout: 100 }); } catch (error) { name = error.name; }
  await page.locator('#disabled').click({ force: true });
  return { name, events: await recorded(page) };
});
paired('fill and clear preserve Unicode and native input event behavior', async page => {
  const input = page.locator('#input');
  await input.fill('你好 é 🦀'); const filled = await input.inputValue();
  await input.clear(); return { filled, empty: await input.inputValue(), events: await recorded(page) };
});
paired('fill targets textarea contenteditable and associated labels', async page => {
  await page.locator('#area').fill('line one\nline two');
  await page.locator('#editable').fill('rich text');
  await page.locator('#input-label').fill('label value');
  return { area: await page.locator('#area').inputValue(), editable: await page.locator('#editable').textContent(), input: await page.locator('#input').inputValue(), events: await recorded(page) };
});
paired('direct date fill dispatches the same input and change events', async page => {
  await page.locator('#date').fill('2026-09-07');
  return { value: await page.locator('#date').inputValue(), events: await recorded(page) };
});
paired('press uses native key chords and selection rather than replacing the value', async page => {
  const input = page.locator('#input');
  await input.press('ControlOrMeta+A'); await input.press('Shift+Z');
  return { value: await input.inputValue(), events: await recorded(page) };
});
paired('check uncheck and setChecked compose to the desired state', async page => {
  const checkbox = page.locator('#check');
  await checkbox.check(); await checkbox.check(); await checkbox.uncheck(); await checkbox.setChecked(true);
  return { checked: await checkbox.isChecked(), events: await recorded(page) };
});
paired('associated-label check and radio uncheck preserve native rules', async page => {
  await page.locator('#check-label').check();
  let rejected = false;
  try { await page.locator('#radio').uncheck(); } catch (error) { rejected = error.name === 'Error'; }
  return { checked: await page.locator('#check').isChecked(), radio: await page.locator('#radio').isChecked(), rejected };
});
paired('selectOption supports strings conjunctive descriptors DOM ordering and null', async page => {
  const single = page.locator('#select'), multiple = page.locator('#multiple');
  const string = await single.selectOption('Beta');
  const descriptor = await single.selectOption({ value: 'c', label: 'Gamma', index: 2 });
  const ordered = await multiple.selectOption([{ value: 'c' }, { index: 0 }]);
  const strings = await multiple.selectOption(['b', 'a']);
  const cleared = await multiple.selectOption(null);
  return { string, descriptor, ordered, strings, cleared, events: await recorded(page) };
});
paired('selectOption rejects a heterogeneous string and descriptor array', async page => {
  try { await page.locator('#multiple').selectOption(['c', { index: 0 }]); return 'resolved'; }
  catch (error) { return error.name; }
});
paired('selectOption waits for a requested option to be added', async page => {
  const waiting = page.locator('#select').selectOption('later', { timeout: 5000 });
  const outcome = waiting.then(value => ({ value }), error => ({ error: error.name }));
  await page.evaluate(() => { document.querySelector('#select').add(new Option('Later', 'later')); });
  return outcome;
});
paired('focus blur and non-focusable no-op preserve active element and events', async page => {
  await page.locator('#input').focus(); const focused = await page.evaluate(() => document.activeElement.id);
  await page.locator('#input').blur(); await page.locator('#plain').focus();
  return { focused, final: await page.evaluate(() => document.activeElement.id), events: await page.evaluate(() => events.filter(event => ['focus','blur'].includes(event.type))) };
});
paired('hover is native and does not click', async page => {
  await page.locator('#button').hover();
  return { hovered: await page.evaluate(() => document.querySelector('#button').matches(':hover')), clicked: await recorded(page) };
});
paired('scrollIntoViewIfNeeded reveals an offscreen target without clicking', async page => {
  await page.locator('#far').scrollIntoViewIfNeeded();
  const before = await page.evaluate(() => scrollY);
  await page.locator('#far').scrollIntoViewIfNeeded();
  return page.evaluate(before => { const box = document.querySelector('#far').getBoundingClientRect(); return { visible: box.top >= 0 && box.bottom <= innerHeight, stable: scrollY === before, clicked: events.some(event => event.type === 'click') }; }, before);
});
paired('click waits for its document navigation and publishes the resulting URL', async page => {
  await page.locator('#link').click();
  return { url: page.url(), frame: page.mainFrame().url(), title: await page.title() };
});
paired('press Enter waits for its form navigation and publishes the resulting URL', async page => {
  await page.locator('#submit-input').press('Enter');
  return { url: page.url(), frame: page.mainFrame().url(), title: await page.title() };
});
paired('concurrent DOM evaluation and Locator reads use the document reached by Enter (NODE-021)', async page => {
  await page.locator('#submit-input').press('Enter');
  return Promise.all([
    page.title(), page.mainFrame().content(), page.evaluate(() => document.URL),
    page.locator('h1').textContent({ timeout: 5000 }),
  ]);
});
paired('actions following repeated commit navigations act exactly once on each replacement document', async page => {
  const retained = page.mainFrame();
  const input = page.locator('#input');
  const observations = [];
  for (let document = 1; document <= 3; document += 1) {
    await page.goto(`${fixtureBase}/actions.html?world-generation=${document}`, { waitUntil: 'commit', timeout: 5000 });
    await input.fill(`document ${document}`, { timeout: 5000 });
    observations.push({
      retainedFrame: retained === page.mainFrame(), value: await input.inputValue(),
      url: await page.evaluate(() => document.URL),
      inputs: await page.evaluate(() => events.filter(event => event.type === 'input')),
    });
  }
  return observations;
});
paired('Page close cancels an actionability wait while preserving its Browser', async (page, browser) => {
  let settled = false;
  const pending = page.locator('#missing').click({ timeout: 30000 }).then(() => { settled = true; return 'resolved'; }, error => { settled = true; return /closed/i.test(error.message) ? 'closed' : error.name; });
  const deadline = Date.now() + 5000;
  do { await setImmediate(); } while (browser.pwrightMetrics && !browser.pwrightMetrics().activeOperations && Date.now() < deadline);
  assert.equal(settled, false);
  await page.close(); return { result: await pending, connected: browser.isConnected() };
});
test('invalid and oversized action arguments release admission without browser mutation', { timeout: 20000 }, async () => {
  // Arrange
  await withPage(candidate, async (page, browser) => {
    const input = page.locator('#input');
    // Act
    const calls = [input.fill('x'.repeat(1024 * 1024 + 1)), input.fill('\ud800'), input.click({ position: { x: NaN, y: 1 } }), input.click({ unknown: true }), input.press('HyperSpace'), page.locator('#select').selectOption(Array(1025).fill('x'))];
    const outcomes = await Promise.allSettled(calls);
    // Assert
    assert.ok(outcomes.every(outcome => outcome.status === 'rejected'));
    assert.equal(await input.inputValue(), 'old');
    assert.equal(browser.pwrightFfiMetrics().commands, 0);
    assert.equal(browser.pwrightMetrics().activeOperations, 0);
  });
});

paired('literal aliases and physical key codes respond differently to held Shift', async page => {
  const input = page.locator('#input');
  const values = [];
  for (const key of ['Shift+a', 'Shift+1', 'Shift+KeyA', 'Shift+Digit1']) {
    await input.fill('');
    await input.press(key);
    values.push(await input.inputValue());
  }
  return { values, events: await recorded(page) };
});

paired('keyboard state after a timed-out delayed press matches Playwright (NODE-022)', async page => {
  let name;
  try { await page.locator('#input').press('Shift+Z', { delay: 30000, timeout: 250 }); }
  catch (error) { name = error.name; }
  await page.locator('#other').press('a');
  return { name, value: await page.locator('#other').inputValue(), finalKey: await page.evaluate(() => events.filter(event => event.type === 'keydown').at(-1)) };
});

test('Worker termination drains a held key transaction and removes its exact Context', { timeout: 20000 }, async () => {
  // Arrange
  const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  const url = `${fixtureBase}/actions.html#worker-held-key`;
  const worker = new Worker(new URL('./worker-actions.mjs', import.meta.url), { workerData: { endpoint, url } });
  try {
    const [metrics] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
    assert.ok(metrics.activeOperations >= 1);
    const cdp = await observer.newBrowserCDPSession();
    const { targetInfos } = await cdp.send('Target.getTargets');
    const targets = targetInfos.filter(target => target.type === 'page' && target.url === url);
    assert.equal(targets.length, 1);
    const target = targets[0]; assert.ok(target.browserContextId);
    // Act
    await worker.terminate();
    // Assert
    const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
    const { targetInfos: after } = await cdp.send('Target.getTargets');
    assert.equal(browserContextIds.includes(target.browserContextId), false);
    assert.equal(after.some(value => value.targetId === target.targetId), false);
    assert.ok(observer.isConnected());
  } finally { await worker.terminate(); await observer.close(); }
});
