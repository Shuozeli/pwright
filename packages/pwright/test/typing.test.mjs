import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';
import { setImmediate, setTimeout as wait } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
const { chromium: oracle } = requireOracle('playwright');
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixture = process.env.PWRIGHT_INPUT_FIXTURE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(fixture && new URL(fixture).hostname.endsWith('.ts.net'));
const cases = JSON.parse(await readFile(new URL('../../../tests/data/sequential-typing-cases.json', import.meta.url), 'utf8'));

async function withPage(type, exercise) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(fixture, { timeout: 5000 });
    return await exercise(page, browser, context);
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
async function snapshot(page) {
  const value = JSON.parse(await page.evaluate(() => JSON.stringify(window.typingSnapshot())));
  assert.equal(value.overflow, false);
  return value;
}

for (const method of ['type', 'pressSequentially']) {
  // Public Keyboard now exercises every catalog case, including held modifiers.
  for (const sample of cases) {
    paired(`${method}: ${sample.id} matches exact native events and value`, async page => {
      await page.evaluate(selector => window.resetTyping(selector), sample.selector);
      if (sample.modifier) await page.keyboard.down(sample.modifier);
      let result;
      try {
        result = await page.locator(sample.selector)[method](sample.text, { delay: sample.delayMs, timeout: 5000 });
      } finally {
        if (sample.modifier) await page.keyboard.up(sample.modifier);
      }
      return { result, snapshot: await snapshot(page) };
    });
  }
}

paired('typing resets an unfocused input selection but preserves a focused selection', async page => {
  await page.evaluate(() => {
    document.body.insertAdjacentHTML('beforeend', '<input id="input" value="old"><input id="other">');
    document.querySelector('#input').setSelectionRange(2, 2);
    document.querySelector('#other').focus();
  });
  const input = page.locator('#input');
  await input.pressSequentially('A');
  const unfocused = await input.inputValue();
  await page.evaluate(() => document.querySelector('#input').setSelectionRange(1, 2));
  await input.type('B');
  return { unfocused, focused: await input.inputValue() };
});
paired('empty typing still resolves and focuses the strict target', async page => {
  await page.locator('#text').type('');
  return page.evaluate(() => document.activeElement.id);
});
paired('typing preserves strict selection failure', async page => {
  try { await page.locator('textarea, div').pressSequentially('x', { timeout: 500 }); return 'resolved'; }
  catch (error) { return { name: error.name, strict: /strict/i.test(error.message) }; }
});
paired('typing a missing target uses the overall deadline', async page => {
  try { await page.locator('#missing').type('a', { timeout: 100 }); return 'resolved'; }
  catch (error) { return error.name; }
});
paired('typing on readonly input sends keys without pretending the value changed', async page => {
  await page.evaluate(() => { window.resetTyping('#text'); document.querySelector('#text').readOnly = true; });
  await page.locator('#text').pressSequentially('a中');
  return snapshot(page);
});
paired('noWaitAfter is a typing compatibility no-op', async page => {
  await page.evaluate(() => window.resetTyping('#text'));
  await page.locator('#text').type('ab', { noWaitAfter: false });
  await page.locator('#text').pressSequentially('cd', { noWaitAfter: true });
  return snapshot(page);
});
paired('Page close cancels sequential typing after a confirmed first key', async page => {
  await page.evaluate(() => window.resetTyping('#text'));
  const pending = page.locator('#text').type('ab', { delay: 30000, timeout: 40000 }).then(() => 'resolved', error => error.name);
  const deadline = Date.now() + 5000;
  while (!(await snapshot(page)).events.some(event => event.type === 'keydown' && event.key === 'a')) {
    assert.ok(Date.now() < deadline, 'first native key must be observed');
    await setImmediate();
  }
  await page.close();
  return pending;
});
async function overlap(page, text, selector = '#text') {
  await page.evaluate(() => window.resetTyping('#text'));
  const locator = page.locator('#text');
  const first = locator.pressSequentially('ab', { delay: 30000, timeout: 40000 }).catch(error => error.name);
  const deadline = Date.now() + 5000;
  while (!(await snapshot(page)).events.some(event => event.type === 'keydown' && event.key === 'a')) {
    assert.ok(Date.now() < deadline, 'first call must be inside its native delay');
    await setImmediate();
  }
  const second = page.locator(selector).type(text, { timeout: 1000 }).then(() => 'resolved', error => error.name);
  let progress, observed;
  try {
    // Diagnostic observation only: never replace an API timeout or classify
    // this timer as an API result. Close our Page before awaiting queued cleanup.
    progress = await Promise.race([second, wait(2000).then(() => 'pending-at-observation')]);
    observed = await snapshot(page);
  } finally {
    await page.close();
    await Promise.all([first, second]);
  }
  return { progress, snapshot: observed, second: await second };
}
paired('a second typing call can progress during a confirmed first-key delay', page => overlap(page, 'C'));
paired('same-key overlapping typing preserves native repeat and explicit keyup', page => overlap(page, 'a'));
paired('missing-target timeout does not wait for or release another typing action', page => overlap(page, 'C', '#missing'));
test('typing rejects unpaired UTF-16 and excessive input explicitly', { timeout: 15000 }, async () => {
  await withPage(candidate, async page => {
    await assert.rejects(page.locator('#text').type('\ud800'), /unpaired UTF-16/);
    await assert.rejects(page.locator('#text').pressSequentially('a'.repeat(1024 * 1024 + 1)), /exceeds/);
  });
});
test('Worker termination cancels a confirmed sequential key and disposes its Context', { timeout: 20000 }, async () => {
  const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  const url = `${fixture}#typing-worker`;
  const worker = new Worker(new URL('./worker-typing.mjs', import.meta.url), { workerData: { endpoint, url } });
  try {
    const [metrics] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
    assert.ok(metrics.activeOperations >= 1);
    const cdp = await observer.newBrowserCDPSession();
    const { targetInfos } = await cdp.send('Target.getTargets');
    const targets = targetInfos.filter(target => target.type === 'page' && target.url === url);
    assert.equal(targets.length, 1);
    const target = targets[0]; assert.ok(target.browserContextId);
    await worker.terminate();
    const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
    const { targetInfos: after } = await cdp.send('Target.getTargets');
    assert.equal(browserContextIds.includes(target.browserContextId), false);
    assert.equal(after.some(value => value.targetId === target.targetId), false);
    assert.ok(observer.isConnected());
  } finally { await worker.terminate(); await observer.close(); }
});
