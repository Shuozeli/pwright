import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const createAdapter = createRequire(import.meta.url)('../src/wait-for-url.cjs');

function adapter() {
  return createAdapter({
    mapError(error, api) {
      const mapped = new Error(`${api}: ${error.message}`);
      mapped.code = error.code;
      return mapped;
    },
    validateOptions(options, supported, api) {
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new Error(`${api}: unsupported ${key}`);
      }
    },
    UnsupportedOperationError: class extends Error {},
  });
}

test('URL wait delegates the event loop to native code with navigation defaults', async () => {
  // Arrange
  const calls = [];
  const session = {
    pageUrlGlob(pageId, glob) {
      calls.push(['glob', pageId, glob]);
      return '^https://example\\.com/grid\\.html$';
    },
    pageDefaultTimeout(pageId, navigation) {
      calls.push(['timeout', pageId, navigation]);
      return 4321;
    },
    async frameWaitForUrl(pageId, frameId, matcher, waitUntil, timeout) {
      calls.push(['wait', pageId, frameId, matcher('https://example.com/grid.html'), waitUntil, timeout]);
    },
  };
  const frame = { _id: 'frame-2', _page: { _id: 'page-1', _context: { _browser: { _session: session } } } };

  // Act
  await adapter().wait(frame, '**/grid.html', { waitUntil: 'domcontentloaded' }, 'frame.waitForURL');

  // Assert
  assert.deepEqual(calls, [
    ['timeout', 'page-1', true],
    ['glob', 'page-1', '**/grid.html'],
    ['wait', 'page-1', 'frame-2', true, 'domcontentloaded', 4321],
  ]);
});

test('URL wait preserves JavaScript RegExp and URL predicate semantics', () => {
  // Arrange
  const { prepareMatcher } = adapter();
  const session = { pageUrlGlob() { throw new Error('not used'); } };
  const regex = /third\.html/;
  const seen = [];

  // Act
  const regexResult = prepareMatcher(session, 'page-1', regex, 'page.waitForURL')('https://example.com/third.html');
  const predicateResult = prepareMatcher(session, 'page-1', value => {
    seen.push(value);
    return value.pathname === '/child.html';
  }, 'page.waitForURL')('https://example.com/child.html');

  // Assert
  assert.equal(regexResult, true);
  assert.equal(predicateResult, true);
  assert.equal(seen[0] instanceof URL, true);
});

test('URL predicate exceptions are not reclassified as parse failures', () => {
  // Arrange
  const expected = new TypeError('predicate failed');
  expected.code = 'ERR_INVALID_URL';
  const matcher = adapter().prepareMatcher({}, 'page-1', () => { throw expected; }, 'page.waitForURL');

  // Act / Assert
  assert.throws(() => matcher('https://example.com/'), error => error === expected);
});

test('URL wait reports Playwright-shaped timeout punctuation', async () => {
  // Arrange
  const session = {
    pageUrlGlob() { return 'never'; },
    async frameWaitForUrl() {
      const error = new Error('Timeout 2500ms exceeded');
      error.code = 'PWRIGHT_TIMEOUT';
      throw error;
    },
  };
  const frame = { _id: 'frame-1', _page: { _id: 'page-1', _context: { _browser: { _session: session } } } };

  // Act
  const pending = adapter().wait(frame, '**/frame.html', { timeout: 2500 }, 'page.waitForURL');

  // Assert
  await assert.rejects(pending, { message: 'page.waitForURL: Timeout 2500ms exceeded.' });
});
