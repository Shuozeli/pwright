import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function pageDescriptor(id) {
  const mainFrameId = id.replace('page-', 'frame-');
  return {
    id,
    mainFrameId,
    frameTree: {
      metadata: () => ({ url: () => 'about:blank', name: () => '' }),
      frames: () => [{ id: mainFrameId, parentId: null }],
    },
  };
}

function fixture() {
  const calls = [];
  const native = {
    events: [],
    claimed: [],
    watchCount: 0,
    isConnected: () => true,
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    reportCallbackException(error) { throw error; },
    async cdpBrowserSessionCreate() {
      calls.push(['attach-browser']);
      return {
        token: () => 101n,
        claim: () => calls.push(['claim-lease']),
        abort: () => calls.push(['abort-lease']),
      };
    },
    async cdpSessionCreate() {
      calls.push(['attach-page']);
      return {
        token: () => 102n,
        claim: () => calls.push(['claim-page-lease']),
        abort: () => calls.push(['abort-page-lease']),
      };
    },
    async cdpSessionSend(token, method, paramsJson) {
      calls.push(['send', token, method, JSON.parse(paramsJson)]);
      return { product: 'Chrome' };
    },
    async cdpSessionClaim(token, maxEvents) {
      calls.push(['claim-events', token, maxEvents]);
      const events = this.events.splice(0, maxEvents);
      let cursor = 0;
      return {
        peek: () => events[cursor] ?? null,
        ackNext: () => { this.claimed.push(events[cursor]); cursor += 1; },
        rollback: () => { this.events.unshift(...events.slice(cursor)); },
      };
    },
    async cdpSessionDetach(token) { calls.push(['detach', token]); },
    nextCdpSessionEventGeneration() {
      this.watchCount += 1;
      return new Promise(() => {});
    },
    async newBrowserPage() {
      this.events.push(['Target.targetCreated', { targetInfo: { targetId: 'page-target' } }]);
      return {
        context: { id: 'context-1', settings: { resolveUrl: url => url } },
        page: pageDescriptor('page-1'),
      };
    },
    async newContext() {
      return { id: 'context-2', settings: { resolveUrl: url => url } };
    },
    async newPage(id) {
      calls.push(['new-page', id]);
      this.events.push(['Target.targetCreated', { targetInfo: { targetId: 'page-target-2' } }]);
      return pageDescriptor('page-2');
    },
    async closeContext(id) { calls.push(['close-context', id]); },
    async closePage(id) { calls.push(['close-page', id]); },
    async close() { calls.push(['close-browser']); },
  };
  class NativeEnvironment {
    async connectOverCDP() { return native; }
  }
  const artifact = '/test-only/browser-cdp-session.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'browser-cdp-session' },
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, module, module.exports);
  return { api: module.exports, native, calls };
}

test('browser CDP session requires its opt-in native attachment method', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  native.cdpBrowserSessionCreate = undefined;
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  const pending = browser.newBrowserCDPSession();

  // Assert
  await assert.rejects(pending, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, []);
});

test('browser CDP session sends on its own lease and publishes target creation before newPage returns', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  const session = await browser.newBrowserCDPSession();
  const events = [];
  session.on('Target.targetCreated', params => events.push(params.targetInfo.targetId));

  // Act
  const result = await session.send('Browser.getVersion');
  const page = await browser.newPage();

  // Assert
  assert.deepEqual(result, { product: 'Chrome' });
  assert.equal(page.context().browser(), browser);
  assert.deepEqual(events, ['page-target']);
  assert.deepEqual(native.claimed.map(([method]) => method), ['Target.targetCreated']);
  assert.ok(calls.some(call => call[0] === 'send' && call[1] === 101n));
});

test('closing a Page Context leaves the browser CDP session live until explicit detach', async () => {
  // Arrange
  const { api, calls } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  const session = await browser.newBrowserCDPSession();
  const page = await browser.newPage();
  const closed = [];
  session.on('close', () => closed.push('closed'));

  // Act
  await page.context().close();
  await session.send('Browser.getVersion');
  await session.detach();

  // Assert
  assert.deepEqual(closed, ['closed']);
  assert.ok(calls.some(call => call[0] === 'close-context'));
  assert.ok(calls.some(call => call[0] === 'send' && call[1] === 101n));
  assert.deepEqual(calls.at(-1), ['detach', 101n]);
});

test('Browser close retires its CDP session once', async () => {
  // Arrange
  const { api } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  const session = await browser.newBrowserCDPSession();
  let closes = 0;
  session.on('close', () => { closes += 1; });

  // Act
  await browser.close();

  // Assert
  assert.equal(closes, 1);
  await assert.rejects(session.send('Browser.getVersion'), /has been closed/);
});

test('Browser and Page CDP sessions share one idle watch', async () => {
  // Arrange
  const { api, native } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();

  // Act
  await browser.newBrowserCDPSession();
  await page.context().newCDPSession(page);

  // Assert
  assert.equal(native.watchCount, 1);
});

test('invalid browser-root delivery lease is aborted before watch registration', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  native.cdpBrowserSessionCreate = async () => ({
    token: () => -1n,
    claim: () => { throw new Error('must not claim'); },
    abort: () => calls.push(['abort-lease']),
  });
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  const pending = browser.newBrowserCDPSession();

  // Assert
  await assert.rejects(pending, /native token is invalid/);
  assert.deepEqual(calls, [['abort-lease']]);
  assert.equal(native.watchCount, 0);
});

test('Browser.newPage closes its new Context when Browser CDP flush fails', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  await browser.newBrowserCDPSession();
  const primary = new Error('CDP flush failed');
  native.cdpSessionClaim = async () => { throw primary; };

  // Act
  const pending = browser.newPage();

  // Assert
  await assert.rejects(pending, error => error === primary);
  assert.deepEqual(calls.filter(call => call[0] === 'close-context'), [['close-context', 'context-1']]);
  assert.equal(browser._contexts.has('context-1'), false);
});

test('Browser.newPage reports both CDP flush and Context cleanup failures', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  await browser.newBrowserCDPSession();
  const primary = new Error('CDP flush failed');
  const cleanup = new Error('Context cleanup failed');
  native.cdpSessionClaim = async () => { throw primary; };
  native.closeContext = async id => { calls.push(['close-context', id]); throw cleanup; };

  // Act
  const pending = browser.newPage();

  // Assert
  await assert.rejects(pending, error => {
    assert.ok(error instanceof AggregateError);
    assert.deepEqual(error.errors, [primary, cleanup]);
    assert.equal(error.cause, primary);
    return true;
  });
  assert.deepEqual(calls.filter(call => call[0] === 'close-context'), [['close-context', 'context-1']]);
  assert.equal(browser._contexts.has('context-1'), true);
});

test('BrowserContext.newPage closes its new Page when Browser CDP flush fails', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  const context = await browser.newContext();
  await browser.newBrowserCDPSession();
  const primary = new Error('CDP flush failed');
  native.cdpSessionClaim = async () => { throw primary; };

  // Act
  const pending = context.newPage();

  // Assert
  await assert.rejects(pending, error => error === primary);
  assert.deepEqual(calls.filter(call => call[0] === 'close-page'), [['close-page', 'page-2']]);
  assert.equal(context._pages.has('page-2'), false);
  assert.equal(browser._contexts.has('context-2'), true);
});

test('BrowserContext.newPage reports both CDP flush and Page cleanup failures', async () => {
  // Arrange
  const { api, native, calls } = fixture();
  const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
  const context = await browser.newContext();
  await browser.newBrowserCDPSession();
  const primary = new Error('CDP flush failed');
  const cleanup = new Error('Page cleanup failed');
  native.cdpSessionClaim = async () => { throw primary; };
  native.closePage = async id => { calls.push(['close-page', id]); throw cleanup; };

  // Act
  const pending = context.newPage();

  // Assert
  await assert.rejects(pending, error => {
    assert.ok(error instanceof AggregateError);
    assert.deepEqual(error.errors, [primary, cleanup]);
    assert.equal(error.cause, primary);
    return true;
  });
  assert.deepEqual(calls.filter(call => call[0] === 'close-page'), [['close-page', 'page-2']]);
  assert.equal(context._pages.has('page-2'), true);
});
