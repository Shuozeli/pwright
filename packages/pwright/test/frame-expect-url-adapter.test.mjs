import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const createAdapter = createRequire(import.meta.url)('../src/frame-expect-url.cjs');

class UnsupportedOperationError extends Error {}

function fixture(frameWaitForUrl, enabled = true) {
  const calls = [];
  const session = {
    async frameWaitForUrl(pageId, frameId, matcher, waitUntil, timeout) {
      calls.push([pageId, frameId, waitUntil, timeout]);
      return frameWaitForUrl(matcher);
    },
  };
  const frame = {
    _id: 'frame-2',
    _page: { _id: 'page-1', _context: { _browser: { _session: session } } },
    url: () => 'https://example.test/last',
  };
  const adapter = createAdapter({
    enabled,
    UnsupportedOperationError,
    validateOptions(options, supported, api) {
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new TypeError(`${api}: unsupported ${key}`);
      }
    },
    mapError(error, api) {
      if (!error.message.startsWith('PWRIGHT_')) return error;
      const match = /^(PWRIGHT_[A-Z_]+): (.*)$/.exec(error.message);
      const mapped = new Error(`${api}: ${match[2]}`);
      mapped.code = match[1];
      return mapped;
    },
  });
  return { adapter, frame, calls };
}

test('selected Context options expose its accepted baseURL to official matchers', () => {
  // Arrange
  const { adapter } = fixture(() => undefined);
  const context = {};

  // Act
  adapter.setContextBaseURL(context, 'https://example.test/base/');

  // Assert
  assert.equal(context._baseURL, 'https://example.test/base/');
  assert.deepEqual(context._options, { baseURL: 'https://example.test/base/' });
  assert.equal(Object.isFrozen(context._options), true);
});

test('unselected Context keeps its v1 baseURL shape', () => {
  // Arrange
  const { adapter } = fixture(() => undefined, false);
  const context = {};

  // Act
  adapter.setContextBaseURL(context, 'https://example.test/base/');

  // Assert
  assert.equal(context._baseURL, 'https://example.test/base/');
  assert.equal(Object.hasOwn(context, '_options'), false);
});

test('exact current URL match delegates observation and timeout to Rust', async () => {
  // Arrange
  const url = 'https://example.test/b';
  const { adapter, frame, calls } = fixture(matcher => {
    assert.equal(matcher(url), true);
    return url;
  });

  // Act
  const result = await adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: url }], isNot: false, timeout: 321,
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: { value: url } });
  assert.deepEqual(calls, [['page-1', 'frame-2', 'commit', 321]]);
});

test('serialized RegExp and ignoreCase match the observed URL', async () => {
  // Arrange
  const url = 'https://example.test/B';
  const { adapter, frame } = fixture(matcher => {
    assert.equal(matcher(url), true);
    return url;
  });

  // Act
  const result = await adapter.expect(frame, 'to.have.url', {
    expectedText: [{ regexSource: '^https://example\\.test/b$', regexFlags: '', ignoreCase: true }],
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: { value: url } });
});

test('negative URL expectation succeeds when the current URL does not match', async () => {
  // Arrange
  const url = 'https://example.test/other';
  const { adapter, frame } = fixture(matcher => {
    assert.equal(matcher(url), true);
    return url;
  });

  // Act
  const result = await adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: 'https://example.test/b' }], isNot: true,
  });

  // Assert
  assert.deepEqual(result, { matches: false, received: { value: url } });
});

test('native timeout returns an expectation mismatch with the last received URL', async () => {
  // Arrange
  const url = 'https://example.test/old';
  const { adapter, frame } = fixture(matcher => {
    assert.equal(matcher(url), false);
    throw new Error('PWRIGHT_TIMEOUT: Timeout 25ms exceeded');
  });

  // Act
  const result = await adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: 'https://example.test/b' }], timeout: 25,
  });

  // Assert
  assert.deepEqual(result, {
    matches: false,
    received: { value: url },
    timedOut: true,
    log: ['Timeout 25ms exceeded'],
  });
});

test('non-timeout native failures are not converted into matcher failures', async () => {
  // Arrange
  const { adapter, frame } = fixture(() => {
    throw new Error('PWRIGHT_TARGET_CLOSED: Page closed');
  });

  // Act
  const pending = adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: 'https://example.test/b' }],
  });

  // Assert
  await assert.rejects(pending, { code: 'PWRIGHT_TARGET_CLOSED' });
});

test('official matcher fields are accepted when cancellation is not requested', async () => {
  // Arrange
  const url = 'https://example.test/b';
  const { adapter, frame } = fixture(matcher => {
    assert.equal(matcher(url), true);
    return url;
  });

  // Act
  const result = await adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: url }], isNot: false, timeout: 123,
    signal: undefined, title: 'expect.toHaveURL',
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: { value: url } });
});

test('live cancellation requests fail closed before starting a native wait', async () => {
  // Arrange
  const { adapter, frame, calls } = fixture(() => undefined);

  // Act
  const pending = adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: 'https://example.test/b' }], signal: new AbortController().signal,
  });

  // Assert
  await assert.rejects(pending, UnsupportedOperationError);
  assert.equal(calls.length, 0);
});

test('unsupported expressions fail before starting a native wait', async () => {
  // Arrange
  const { adapter, frame, calls } = fixture(() => undefined);

  // Act
  const pending = adapter.expect(frame, 'to.have.title', { expectedText: [{ string: 'title' }] });

  // Assert
  await assert.rejects(pending, UnsupportedOperationError);
  assert.equal(calls.length, 0);
});

test('unselected profile cannot activate URL expectations', async () => {
  // Arrange
  const { adapter, frame, calls } = fixture(() => undefined, false);

  // Act
  const pending = adapter.expect(frame, 'to.have.url', {
    expectedText: [{ string: 'https://example.test/b' }],
  });

  // Assert
  await assert.rejects(pending, UnsupportedOperationError);
  assert.equal(calls.length, 0);
});

test('selected Page and Context wire the official matcher bridge end to end', async () => {
  // Arrange
  const indexUrl = new URL('../src/index.cjs', import.meta.url);
  const source = readFileSync(indexUrl, 'utf8');
  const sourceRequire = createRequire(indexUrl);
  const artifact = '/test-only/frame-expect-url.node';
  const url = 'https://example.test/b';
  const nativeCalls = [];
  const session = {
    isConnected: () => true,
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    async newBrowserPage(baseURL, acceptDownloads) {
      nativeCalls.push(['newBrowserPage', baseURL, acceptDownloads]);
      return {
        context: { id: 'context-1', settings: { resolveUrl: value => value } },
        page: {
          id: 'page-1', mainFrameId: 'frame-1',
          mainFrameMetadata: { url: () => url, name: () => '' },
        },
      };
    },
    async frameWaitForUrl(pageId, frameId, matcher, waitUntil, timeout) {
      nativeCalls.push(['frameWaitForUrl', pageId, frameId, waitUntil, timeout]);
      assert.equal(matcher(url), true);
      return url;
    },
  };
  class NativeEnvironment {
    async connectOverCDP() { return session; }
  }
  const syntheticModule = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'official-conformance-page-goto-burst-20' },
      nativeFeatures: () => ['wait-for-url'],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(source, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, syntheticModule, syntheticModule.exports);
  const browser = await syntheticModule.exports.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage({ baseURL: 'https://example.test/' });

  // Act
  const result = await page.mainFrame()._expect('to.have.url', {
    expectedText: [{ string: url }], isNot: false, timeout: 123,
    signal: undefined, title: undefined,
  });

  // Assert
  assert.deepEqual(page.context()._options, { baseURL: 'https://example.test/' });
  assert.equal(page._apiName, 'Page');
  assert.deepEqual(result, { matches: true, received: { value: url } });
  assert.deepEqual(nativeCalls, [
    ['newBrowserPage', 'https://example.test/', false],
    ['frameWaitForUrl', 'page-1', 'frame-1', 'commit', 123],
  ]);
});
