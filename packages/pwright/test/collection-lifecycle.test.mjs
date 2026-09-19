import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const pause = new Int32Array(new SharedArrayBuffer(4));

async function emptyContext(type) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const sample = () => ({ registered: browser.contexts().includes(context),
      pages: context.pages().length, connected: browser.isConnected() });
    const initial = sample();
    const before = browser.pwrightContextMetadataMetrics?.().activeContexts;
    // Act
    const pending = context.close().then(() => ({ ok: true }), error => ({ error }));
    if (browser.pwrightContextMetadataMetrics) {
      assert.equal(typeof before, 'number');
      for (let i = 0; i < 500 && browser.pwrightContextMetadataMetrics().activeContexts === before; i++)
        Atomics.wait(pause, 0, 0, 10);
      assert.ok(browser.pwrightContextMetadataMetrics().activeContexts < before, 'native empty Context retired');
    } else Atomics.wait(pause, 0, 0, 100);
    const blocked = sample();
    const result = await diagnosticDeadline(pending, 'empty Context close');
    if (result.error) throw result.error;
    return { initial, blocked, after: sample() };
  } finally {
    try { if (browser.isConnected()) await context?.close(); }
    finally {
      await diagnosticDeadline(browser.close(), 'empty Context browser cleanup');
      if (browser.pwrightMetrics) {
        assert.equal(browser.pwrightMetrics().activeLeases, 0);
        assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
        for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
        assert.equal(browser.pwrightFfiMetrics().pending, 0);
        assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      }
    }
  }
}

test('empty Context removal requires publication without a Page-close token', { timeout: 30000 }, async () => {
  // Arrange
  const initial = { registered: true, pages: 0, connected: true };
  const expected = await emptyContext(oracle);
  assert.deepEqual(expected, { initial, blocked: initial, after: { ...initial, registered: false } });
  // Act
  const actual = await emptyContext(candidate);
  // Assert
  process.stdout.write(`# Empty Context publication ${JSON.stringify({ expected, actual })}\n`);
  assert.deepEqual(actual, expected);
});
