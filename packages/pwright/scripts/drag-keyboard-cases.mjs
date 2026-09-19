// Shared public test actions. No browser automation logic belongs to this runner.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { diagnosticDeadline } from './probe-deadline.mjs';
const cases = JSON.parse(await readFile(new URL('../../../tests/data/raw-drag-keyboard-cases.json', import.meta.url), 'utf8'));
assert.equal(cases.length, 10);

export async function collectDragKeyboard(api, { endpoint, fixture, label, observeOwned, assertRemoved }) {
  const browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
  const observations = [];
  let result, metrics;
  try {
    for (const sample of cases) {
      assert.ok(sample.keys.length <= 4);
      const context = await diagnosticDeadline(browser.newContext(), 'drag-keyboard Context');
      const url = new URL(fixture);
      url.searchParams.set('drag-keyboard-case', `${label}-${sample.id}-${process.pid}`);
      try {
        const page = await diagnosticDeadline(context.newPage(), 'drag-keyboard Page');
        await page.goto(url.href, { timeout: 5000 });
        if (observeOwned) await observeOwned(url.href);
        await page.mouse.move(40, 40);
        if (sample.drag) {
          await page.mouse.down();
          await diagnosticDeadline(page.mouse.move(70, 40), 'capture drag');
          assert.ok(await page.evaluate(() => dragFixture.events.includes('dragstart')));
        }
        for (const key of sample.keys) {
          assert.ok(['down', 'up', 'press', 'locatorpress', 'locatorsequential'].includes(key.kind));
          const operation = key.kind === 'locatorpress'
            ? page.locator('#keyboard').press(key.key, { timeout: key.timeout ? 2000 : 5000, delay: key.timeout ? 60000 : 0, noWaitAfter: true })
            : key.kind === 'locatorsequential'
              ? page.locator('#keyboard').pressSequentially(key.text, { timeout: 5000 })
              : page.keyboard[key.kind](key.key);
          if (key.timeout) await assert.rejects(diagnosticDeadline(operation, `${sample.id}/${key.kind}`), { name: 'TimeoutError' });
          else await diagnosticDeadline(operation, `${sample.id}/${key.kind}`);
        }
        await diagnosticDeadline(page.mouse.move(220, 40), 'move after Escape');
        await diagnosticDeadline(page.mouse.up(), 'drop after Escape');
        const snapshot = await page.evaluate(() => dragFixture);
        assert.equal(snapshot.overflow, false);
        observations.push({ id: sample.id, snapshot });
      } finally {
        await diagnosticDeadline(context.close(), 'drag-keyboard Context cleanup');
        if (assertRemoved) await assertRemoved(url.href);
      }
    }
    result = { version: '1.58.2', browser: browser.version(), observations };
  } finally {
    await diagnosticDeadline(browser.close(), `${label} drag-keyboard disconnect`);
    metrics = browser.pwrightMetrics?.();
    if (metrics) {
      assert.equal(metrics.activeLeases, 0);
      assert.equal(metrics.activeOperations, 0);
      assert.equal(metrics.cleanupInFlight, 0);
    }
  }
  return { result, metrics };
}
