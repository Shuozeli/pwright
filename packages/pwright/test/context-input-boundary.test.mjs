import assert from 'node:assert/strict';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium } from '@shuozeli/pwright';
import artifactConfig from '../src/native-artifact.cjs';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const selectedHeaders = artifactConfig.nativeFeatures(artifactConfig.nativeArtifact.profile).includes('extra-headers');

async function close(browser) {
  await diagnosticDeadline(browser.close(), 'Context input boundary cleanup');
  for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
  assert.equal(browser.pwrightFfiMetrics().pending, 0);
  assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
  assert.equal(browser.pwrightMetrics().activeLeases, 0);
}

for (const headers of [false, true]) {
  for (const [encoding, baseURL] of [['ASCII', 'x'.repeat(16385)], ['UTF-8', '界'.repeat(5462)]]) {
    test(`${headers ? 'header' : 'plain'} Context rejects oversized ${encoding} baseURL as TypeError`,
      { timeout: 15000, skip: headers && !selectedHeaders }, async () => {
        // Arrange
        const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
        try {
          const contexts = browser.contexts();
          const before = browser.pwrightContextMetadataMetrics();
          const reserved = browser.pwrightFfiMetrics().reservedBytes;
          const options = headers ? { baseURL, extraHTTPHeaders: {} } : { baseURL };
          // Act
          const pending = browser.newContext(options);
          // Assert
          await assert.rejects(pending, error => {
            assert.ok(error instanceof TypeError);
            assert.equal(error.code, 'PWRIGHT_INVALID_ARGUMENT');
            assert.match(error.message, /baseURL exceeds 16384 UTF-8 bytes/);
            return true;
          });
          assert.deepEqual(browser.pwrightContextMetadataMetrics(), before);
          assert.deepEqual(browser.contexts(), contexts);
          assert.equal(browser.pwrightFfiMetrics().reservedBytes, reserved);
          assert.equal(browser.pwrightFfiMetrics().commands, 0);
        } finally { await close(browser); }
      });
  }
}

test('header getter exceptions retain identity before oversized baseURL validation',
  { timeout: 15000, skip: !selectedHeaders }, async () => {
    // Arrange
    const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
    try {
      const before = browser.pwrightContextMetadataMetrics();
      const reserved = browser.pwrightFfiMetrics().reservedBytes;
      const sentinel = new Error('PWRIGHT_INVALID_ARGUMENT: user getter sentinel');
      const options = { baseURL: 'x'.repeat(16385), extraHTTPHeaders: {
        get 'X-Test'() { throw sentinel; },
      } };
      // Act
      const pending = browser.newContext(options);
      // Assert
      await assert.rejects(pending, error => error === sentinel);
      assert.deepEqual(browser.pwrightContextMetadataMetrics(), before);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, reserved);
      assert.equal(browser.pwrightFfiMetrics().commands, 0);
    } finally { await close(browser); }
  });
