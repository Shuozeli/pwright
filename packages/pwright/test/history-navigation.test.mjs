import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = requireOracle('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const base = process.env.PWRIGHT_TEST_BASE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(base && new URL(base).hostname.endsWith('.ts.net'));
assert.equal(process.env.PWRIGHT_NATIVE_PROFILE, 'official-conformance-page-bindings');

async function withPage(type, exercise) {
  const side = type === candidate ? 'candidate' : 'oracle';
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    return await exercise(page);
  } finally {
    try {
      await diagnosticDeadline(context?.close(), `${side} Context cleanup`);
    } finally {
      await diagnosticDeadline(browser.close(), `${side} Browser disconnect`);
    }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++)
        await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}

function paired(name, exercise) {
  test(name, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise);

    // Act
    const actual = await withPage(candidate, exercise);

    // Assert
    assert.deepEqual(actual, expected);
  });
}

function responseValue(response, page) {
  if (response === null) return null;
  assert.equal(response.frame(), page.mainFrame());
  return {
    url: response.url(),
    status: response.status(),
    statusText: response.statusText(),
    ok: response.ok(),
    contentType: response.headers()['content-type'],
  };
}

paired('goBack at the first native history entry returns null', async page => {
  // Arrange
  const initial = page.url();

  // Act
  const response = await page.goBack({ timeout: 5000 });

  // Assert
  assert.equal(response, null);
  return { initial, url: page.url(), response };
});

for (const waitUntil of ['commit', 'domcontentloaded', 'load', 'networkidle']) {
  paired(`new-document goBack ${waitUntil} returns the previous response`, async page => {
    // Arrange
    const previous = `${base}/index.html?history-navigation=previous-${waitUntil}`;
    await page.goto(previous, { timeout: 5000 });
    await page.goto(`${base}/empty.html?history-navigation=current-${waitUntil}`, {
      timeout: 5000,
    });

    // Act
    const response = await page.goBack({ waitUntil, timeout: 5000 });

    // Assert
    assert.equal(page.url(), previous);
    return { url: page.url(), response: responseValue(response, page) };
  });
}

paired('same-document goBack preserves the document and returns null', async page => {
  // Arrange
  const initial = `${base}/empty.html?history-navigation=same-document`;
  await page.goto(initial, { timeout: 5000 });
  await page.evaluate(() => {
    window.historyNavigationMarker = 'retained';
    history.pushState(null, '', '#one');
    history.pushState(null, '', '#two');
  });

  // Act
  const response = await page.goBack({ timeout: 5000 });

  // Assert
  assert.equal(response, null);
  return page.evaluate(() => ({
    marker: window.historyNavigationMarker,
    url: location.href,
    state: history.state,
  }));
});

paired('invalid goBack lifecycle rejects without changing the current entry', async page => {
  // Arrange
  const initial = `${base}/empty.html?history-navigation=invalid`;
  await page.goto(initial, { timeout: 5000 });

  // Act
  const error = await page.goBack({ waitUntil: 'ready', timeout: 1000 })
    .then(() => null, value => value);

  // Assert
  assert.ok(error instanceof Error);
  assert.equal(page.url(), initial);
  return { rejected: true, url: page.url() };
});

paired('goBack on a closed Page rejects before starting history work', async page => {
  // Arrange
  await page.close();

  // Act
  const error = await page.goBack({ timeout: 1000 }).then(() => null, value => value);

  // Assert
  assert.ok(error instanceof Error);
  return 'closed';
});
