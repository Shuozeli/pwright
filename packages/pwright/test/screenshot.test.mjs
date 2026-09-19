import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = requireOracle('playwright');
const requireCore = createRequire(requireOracle.resolve('playwright'));
// Decoder code is test-only and comes from the same pinned oracle distribution.
const { PNG, jpegjs } = requireCore(join(dirname(requireCore.resolve('playwright-core')), 'lib/utilsBundle.js'));
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixtureBase = process.env.PWRIGHT_TEST_BASE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(fixtureBase && new URL(fixtureBase).hostname.endsWith('.ts.net'));

async function withPage(type, exercise) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(`${fixtureBase}/screenshot.html`, { timeout: 5000 });
    return await exercise(page, browser);
  } finally {
    try { await context?.close(); } finally { await browser.close(); }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}

function image(buffer, format = 'png') {
  assert.ok(Buffer.isBuffer(buffer));
  const decoded = format === 'png' ? PNG.sync.read(buffer) : jpegjs.decode(buffer);
  const rgba = [...decoded.data.subarray((10 * decoded.width + 10) * 4, (10 * decoded.width + 10) * 4 + 4)];
  return { width: decoded.width, height: decoded.height, rgba,
    pixels: createHash('sha256').update(decoded.data).digest('hex') };
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

for (const [name, options] of [
  ['default PNG viewport', {}],
  ['JPEG default quality', { type: 'jpeg' }],
  ['JPEG explicit quality', { type: 'jpeg', quality: 30 }],
  ['full-page PNG', { fullPage: true }],
  ['viewport CSS scale', { scale: 'css' }],
  ['fractional viewport clip', { clip: { x: 5.5, y: 10.5, width: 120.5, height: 80.5 } }],
]) {
  paired(`${name} matches dimensions and decoded pixels`, async page => {
    const buffer = await page.screenshot({ ...options, timeout: 5000 });
    return image(buffer, options.type);
  });
}

paired('clip is trimmed at the viewport edge', async page => image(await page.screenshot({
  clip: { x: 1200, y: 650, width: 300, height: 300 }, timeout: 5000,
})));

paired('concurrent same-Page screenshots both settle with independent results', async page => {
  const buffers = await Promise.all([page.screenshot({ timeout: 5000 }), page.screenshot({ timeout: 5000 })]);
  const before = image(buffers[1]);
  buffers[0].fill(0);
  return { before, after: image(buffers[1]) };
});

test('returned Buffer survives Context and Browser closure without retaining host leases', { timeout: 20000 }, async () => {
  // Arrange
  const expected = await withPage(oracle, page => page.screenshot({ timeout: 5000 }));
  // Act
  const actual = await withPage(candidate, page => page.screenshot({ timeout: 5000 }));
  // Assert: withPage has already closed/drained both native owners.
  assert.deepEqual(image(actual), image(expected));
  actual[0] = 0;
  assert.equal(expected[0], 0x89);
});

paired('closing a Page drains queued screenshots without retaining native results', async page => {
  const captures = Array.from({ length: 4 }, () => page.screenshot({ timeout: 5000 }).then(
    value => ({ status: 'fulfilled', value }), error => ({ status: 'rejected', error })));
  await page.close();
  const settled = await Promise.all(captures);
  assert.equal(settled.length, 4);
  for (const result of settled) {
    if (result.status === 'fulfilled') image(result.value);
    else assert.match(result.error.message, /closed|Target/i);
  }
  // Whether native capture wins the close race is not an ordered observation.
  return { settled: settled.length };
});

for (const mode of ['queued', 'published']) {
  test(`Worker termination reclaims ${mode} screenshot state and its exact native Context`, { timeout: 25000 }, async () => {
    // Arrange: capture an independent oracle reference before creating the Worker.
    const expected = await withPage(oracle, async page => image(await page.screenshot({ timeout: 5000 })));
    const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
    const clock = new Int32Array(new SharedArrayBuffer(4));
    const url = `${fixtureBase}/screenshot.html#worker-screenshot-${mode}`;
    const worker = new Worker(new URL('./worker-screenshot.mjs', import.meta.url), {
      workerData: { endpoint, url, clock: clock.buffer },
    });
    let cdp;
    try {
      const [queued] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
      assert.equal(queued.stage, 'queued');
      assert.equal(queued.ffi.queuedCommands, 4);
      cdp = await observer.newBrowserCDPSession();
      const { targetInfos } = await cdp.send('Target.getTargets');
      const owned = targetInfos.filter(target => target.type === 'page' && target.url === url);
      assert.equal(owned.length, 1);
      assert.ok(owned[0].browserContextId);
      let buffers;
      if (mode === 'published') {
        const publication = once(worker, 'message', { signal: AbortSignal.timeout(10000) });
        Atomics.store(clock, 0, 1);
        Atomics.notify(clock, 0);
        const [published] = await publication;
        assert.equal(published.stage, 'published');
        buffers = published.buffers.map(value => Buffer.from(value));
      }
      // Act: destroy the environment with either retained Rust results or Node Buffers.
      const exit = await worker.terminate();
      // Assert: native Context disposal is acknowledged before Worker exit.
      assert.equal(exit, 1);
      const { browserContextIds } = await cdp.send('Target.getBrowserContexts');
      const after = await cdp.send('Target.getTargets');
      assert.ok(!browserContextIds.includes(owned[0].browserContextId));
      assert.ok(!after.targetInfos.some(target => target.targetId === owned[0].targetId));
      assert.ok(observer.isConnected());
      if (buffers) {
        assert.equal(buffers.length, 4);
        for (const buffer of buffers) assert.deepEqual(image(buffer), expected);
        buffers[0].fill(0);
        assert.deepEqual(image(buffers[1]), expected);
      }
    } finally {
      await worker.terminate();
      await cdp?.detach();
      await observer.close();
    }
  });
}
