import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';
import { chromium } from '@shuozeli/pwright';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
const { chromium: oracle } = requireOracle('playwright');
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixture = process.env.PWRIGHT_MOUSE_FIXTURE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(fixture && new URL(fixture).hostname.endsWith('.ts.net'));

test('Mouse profile rejects invalid values before changing native input state or retaining FFI commands', { timeout: 20000 }, async () => {
  // Arrange
  const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
  try {
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(fixture, { timeout: 5000 });
    await page.mouse.move(10, 20);
    await page.evaluate(() => window.resetMouse());
    const invalid = [
      () => page.mouse.move(90, 80, { steps: 10001 }),
      () => page.mouse.move(90, 80, { steps: NaN }),
      () => page.mouse.move(90, 80, { steps: 1.5 }),
      () => page.mouse.click(90, 80, { clickCount: 33 }),
      () => page.mouse.click(90, 80, { clickCount: NaN }),
      () => page.mouse.down({ clickCount: 1.5 }),
      () => page.mouse.up({ clickCount: 2 ** 31 }),
      () => page.mouse.click(90, 80, { delay: -1 }),
      () => page.mouse.click(90, 80, { delay: 2 ** 32 }),
      () => page.mouse.dblclick(90, 80, { delay: Infinity }),
      () => page.mouse.down({ button: 'back' }),
      () => page.mouse.wheel(0, Infinity),
    ];
    // Act / Assert: each invalid operation independently preserves the same state.
    for (const call of invalid) {
      await assert.rejects(call(), TypeError);
      assert.equal(browser.pwrightFfiMetrics().commands, 0);
      assert.deepEqual(await page.evaluate(() => window.mouseSnapshot()), { records: [], overflow: false, wheels: 0 });
    }
    await assert.rejects(page.mouse.click(90, 80, { force: true }), { code: 'PWRIGHT_UNSUPPORTED' });
    await page.mouse.down();
    await page.mouse.up();
    const records = await page.evaluate(() => window.mouseSnapshot().records);
    assert.ok(records.length > 0);
    assert.ok(records.every(event => event.x === 10 && event.y === 20));
    await context.close();
  } finally { await browser.close(); }
  assert.equal(browser.pwrightMetrics().activeLeases, 0);
  assert.equal(browser.pwrightFfiMetrics().commands, 0);
});

for (const borrowed of [false, true]) {
  test(`Worker termination retires a native Mouse click on ${borrowed ? 'borrowed' : 'owned'} Page`, { timeout: 20000 }, async () => {
    // Arrange
    const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
    const cdp = await observer.newBrowserCDPSession();
    const url = `${fixture}#mouse-worker-${borrowed}-${process.pid}`;
    let context;
    let page;
    let worker;
    let target;
    try {
      if (borrowed) {
        context = await observer.newContext();
        page = await context.newPage();
        await page.goto(url, { timeout: 5000 });
      }
      worker = new Worker(new URL('./worker-mouse.mjs', import.meta.url), { workerData: { endpoint, url, borrowed } });
      const [metrics] = await once(worker, 'message', { signal: AbortSignal.timeout(10000) });
      assert.ok(metrics.activeOperations >= 1);
      assert.ok(metrics.ffiCommands >= 1);
      const { targetInfos } = await cdp.send('Target.getTargets');
      const targets = targetInfos.filter(target => target.type === 'page' && target.url === url);
      assert.equal(targets.length, 1);
      target = targets[0];
      assert.ok(target.browserContextId);
      // Act
      await worker.terminate();
      // Assert
      const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
      const { targetInfos: after } = await cdp.send('Target.getTargets');
      assert.equal(browserContextIds.includes(target.browserContextId), borrowed);
      assert.equal(after.some(value => value.targetId === target.targetId), borrowed);
      if (borrowed) {
        const snapshot = await page.evaluate(() => window.mouseSnapshot());
        assert.equal(snapshot.overflow, false);
        assert.ok(snapshot.records.some(event => event.type === 'mousedown'));
        assert.equal(snapshot.records.some(event => ['mouseup', 'click'].includes(event.type)), false);
      }
      assert.ok(observer.isConnected());
    } finally {
      try {
        await worker?.terminate();
        await context?.close();
        if (target) {
          const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
          const { targetInfos } = await cdp.send('Target.getTargets');
          assert.equal(browserContextIds.includes(target.browserContextId), false);
          assert.equal(targetInfos.some(value => value.targetId === target.targetId), false);
        }
      } finally { await observer.close(); }
    }
  });
}
