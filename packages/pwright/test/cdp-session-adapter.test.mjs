import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

const settle = () => new Promise(resolve => setImmediate(resolve));

class FakeBatch {
  constructor(native, events) {
    this.native = native;
    this.events = events;
    this.cursor = 0;
  }
  peek() { return this.events[this.cursor] ?? null; }
  ackNext() {
    this.native.acked.push(this.events[this.cursor]);
    this.cursor += 1;
  }
  rollback() {
    this.native.rollbacks += 1;
    this.native.events.unshift(...this.events.slice(this.cursor));
  }
}

function fixture() {
  const calls = [];
  const native = {
    events: [],
    acked: [],
    rollbacks: 0,
    listenerErrors: [],
    watchers: [],
    onLeaseClaim: undefined,
    isConnected: () => true,
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    reportCallbackException(error) { this.listenerErrors.push(error); },
    async newBrowserPage() {
      return {
        context: { id: 'context-1', settings: { resolveUrl: url => url } },
        page: {
          id: 'page-1',
          mainFrameId: 'frame-1',
          frameTree: {
            metadata: () => ({ url: () => 'about:blank', name: () => '' }),
            frames: () => [{ id: 'frame-1', parentId: null }],
          },
        },
      };
    },
    async frameGoto(...args) {
      calls.push(['frameGoto', ...args]);
      this.events.push(['Network.requestWillBeSent', { request: { url: args[1] } }]);
      return { response: null };
    },
    async closePage(pageId) { calls.push(['closePage', pageId]); },
    async closeContext(contextId) { calls.push(['closeContext', contextId]); },
    async close() { calls.push(['close']); },
    async cdpSessionCreate(...args) {
      calls.push(['create', ...args]);
      return {
        token: () => 7n,
        claim: () => { this.onLeaseClaim?.(); calls.push(['leaseClaim']); },
        abort: () => { calls.push(['leaseAbort']); },
      };
    },
    async cdpSessionSend(token, method, paramsJson) {
      calls.push(['send', token, method, JSON.parse(paramsJson)]);
      if (method === 'Runtime.evaluate')
        this.events.push(['Runtime.consoleAPICalled', { value: 'log' }]);
      return { result: { value: 3 } };
    },
    async cdpSessionClaim(token, maxEvents) {
      calls.push(['claim', token, maxEvents]);
      return new FakeBatch(this, this.events.splice(0, maxEvents));
    },
    async cdpSessionDetach(token) { calls.push(['detach', token]); },
    nextCdpSessionEventGeneration(after) {
      const pending = deferred();
      this.watchers.push({ after, ...pending });
      return pending.promise;
    },
  };
  class NativeEnvironment {
    async connectOverCDP() { return native; }
  }
  const artifact = '/test-only/cdp-session.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'default' },
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

async function openPage(value) {
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  return { browser, page, context: page.context() };
}

test('CDPSession requires all native methods before creating a child', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  value.native.cdpSessionClaim = undefined;

  // Act
  const pending = context.newCDPSession(page);

  // Assert
  await assert.rejects(pending, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(value.calls, []);
});

test('CDPSession requires the native idle-event watch before creating a child', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  value.native.nextCdpSessionEventGeneration = undefined;

  // Act
  const pending = context.newCDPSession(page);

  // Assert
  await assert.rejects(pending, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(value.calls, []);
});

test('CDPSession accepts Page and exact main Frame, and claims each lease after wrapper registration', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  value.native.onLeaseClaim = () => assert.equal(context._cdpSessions.has(7n), true);

  // Act
  const pageSession = await context.newCDPSession(page);
  await pageSession.detach();
  const frameSession = await context.newCDPSession(page.mainFrame());

  // Assert
  assert.deepEqual(value.calls, [
    ['create', 'context-1', 'page-1', null],
    ['leaseClaim'],
    ['detach', 7n],
    ['create', 'context-1', 'page-1', 'frame-1'],
    ['leaseClaim'],
  ]);
  assert.equal(context._cdpSessions.get(7n), frameSession);
});

test('an invalid native CDP token aborts its unclaimed attachment immediately', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  let aborted = 0;
  value.native.cdpSessionCreate = async () => ({
    token: () => -1n,
    claim: () => { throw new Error('must not claim'); },
    abort: () => { aborted += 1; },
  });

  // Act
  const pending = context.newCDPSession(page);

  // Assert
  await assert.rejects(pending, /native token is invalid/);
  assert.equal(aborted, 1);
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(value.native.watchers.length, 0);
});

test('a failed native lease claim removes the wrapper and aborts once', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  let aborted = 0;
  value.native.cdpSessionCreate = async () => ({
    token: () => 7n,
    claim: () => { throw new Error('PWRIGHT_TARGET_CLOSED: attach retired'); },
    abort: () => { aborted += 1; },
  });

  // Act
  const pending = context.newCDPSession(page);

  // Assert
  await assert.rejects(pending, { code: 'PWRIGHT_TARGET_CLOSED' });
  assert.equal(aborted, 1);
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(value.native.watchers.length, 0);
});

test('CDPSession rejects non-Page targets and child Frames before native dispatch', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const child = Object.create(Object.getPrototypeOf(page.mainFrame()));
  child._page = page;
  child._id = 'child-frame';

  // Act / Assert
  await assert.rejects(context.newCDPSession(context), /page: expected Page or Frame/);
  await assert.rejects(context.newCDPSession({}), /page: expected Page or Frame/);
  await assert.rejects(context.newCDPSession(child), /part of the parent frame's session/);
  assert.deepEqual(value.calls, []);
});

test('send and Page.goto deliver ordered method and generic events before returning', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const session = await context.newCDPSession(page);
  const delivered = [];
  session.on('Runtime.consoleAPICalled', params => delivered.push(['console', params.value]));
  session.on('Network.requestWillBeSent', params => delivered.push(['request', params.request.url]));
  session.on('event', event => delivered.push(['event', event.method]));

  // Act
  const result = await session.send('Runtime.evaluate', { expression: '1 + 2' });
  const response = await page.goto('https://example.test/');

  // Assert
  assert.deepEqual(result, { result: { value: 3 } });
  assert.equal(response, null);
  assert.deepEqual(delivered, [
    ['console', 'log'],
    ['event', 'Runtime.consoleAPICalled'],
    ['request', 'https://example.test/'],
    ['event', 'Network.requestWillBeSent'],
  ]);
  assert.deepEqual(value.native.acked.map(([method]) => method), [
    'Runtime.consoleAPICalled', 'Network.requestWillBeSent',
  ]);
  assert.equal(value.native.events.length, 0);
  assert.ok(value.calls.some(call => call[0] === 'claim' && call[2] === 64));
});

test('CDP event listener exceptions are reported and acknowledged without failing send', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  const session = await context.newCDPSession(page);
  const methodError = new Error('method callback failed');
  const genericError = new Error('generic callback failed');
  session.once('Runtime.consoleAPICalled', () => { throw methodError; });
  session.once('event', () => { throw genericError; });

  // Act
  const result = await session.send('Runtime.evaluate');

  // Assert
  assert.deepEqual(result, { result: { value: 3 } });
  assert.deepEqual(value.native.listenerErrors, [methodError, genericError]);
  assert.equal(value.native.events.length, 0);
  assert.equal(value.native.acked.length, 1);
  assert.equal(browser.isConnected(), true);
});

test('throwing CDP close listeners are reported after retiring every sibling wrapper', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  let nextToken = 7n;
  value.native.cdpSessionCreate = async () => {
    const token = nextToken++;
    return { token: () => token, claim: () => token, abort: () => {} };
  };
  const first = await context.newCDPSession(page);
  const second = await context.newCDPSession(page);
  const firstError = new Error('first close callback failed');
  const secondError = new Error('second close callback failed');
  first.on('close', () => { throw firstError; });
  second.on('close', () => { throw secondError; });

  // Act
  await page.close();

  // Assert
  assert.deepEqual(value.native.listenerErrors, [firstError, secondError]);
  assert.equal(context._cdpSessions.size, 0);
  await assert.rejects(first.send('Runtime.enable'), /Target page, context or browser has been closed/);
  await assert.rejects(second.send('Runtime.enable'), /Target page, context or browser has been closed/);
});

test('detach and Page.close emit close once and reject later protocol calls', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const first = await context.newCDPSession(page);
  const closed = [];
  first.on('close', session => closed.push(session));

  // Act
  await first.detach();
  const second = await context.newCDPSession(page);
  second.on('close', session => closed.push(session));
  await page.close();

  // Assert
  assert.deepEqual(closed, [first, second]);
  assert.equal(context._cdpSessions.size, 0);
  await assert.rejects(first.send('Runtime.enable'), /Target page, context or browser has been closed/);
  await assert.rejects(second.detach(), /Target page, context or browser has been closed/);
});

test('CDPSession rejects non-finite or oversized params before native send', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const session = await context.newCDPSession(page);

  // Act / Assert
  await assert.rejects(session.send('Runtime.evaluate', { value: Number.NaN }), /finite numbers/);
  await assert.rejects(session.send('Runtime.evaluate', { value: 'x'.repeat(1024 * 1024) }), /1 MiB/);
  assert.equal(value.calls.some(call => call[0] === 'send'), false);
});

test('concurrent send fences serialize claims without copying events into a JS queue', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const session = await context.newCDPSession(page);
  const originalClaim = value.native.cdpSessionClaim;
  let activeClaims = 0;
  let peakClaims = 0;
  value.native.cdpSessionClaim = async function (...args) {
    activeClaims += 1;
    peakClaims = Math.max(peakClaims, activeClaims);
    try {
      await Promise.resolve();
      return await originalClaim.apply(this, args);
    } finally { activeClaims -= 1; }
  };

  // Act
  await Promise.all([
    session.send('Runtime.enable'),
    session.send('Debugger.enable'),
  ]);

  // Assert
  assert.equal(peakClaims, 1);
  assert.equal(value.native.rollbacks, 2);
  assert.equal(value.native.events.length, 0);
});

test('CDPSession listener registrations have a fixed capacity', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const session = await context.newCDPSession(page);
  for (let index = 0; index < 64; index += 1)
    session.on(`Runtime.event${index}`, () => {});

  // Act / Assert
  assert.throws(() => session.once('Runtime.overflow', () => {}), /handler limit exceeded \(64\)/);
});

test('a lazy Browser watch delivers idle CDP events and re-arms once per generation', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  assert.equal(value.native.watchers.length, 0);
  const session = await context.newCDPSession(page);
  const delivered = [];
  session.on('Runtime.consoleAPICalled', params => delivered.push(params.value));
  assert.equal(value.native.watchers.length, 1);
  value.native.events.push(['Runtime.consoleAPICalled', { value: 'idle' }]);

  // Act
  value.native.watchers[0].resolve(1n);
  await settle();

  // Assert
  assert.deepEqual(delivered, ['idle']);
  assert.equal(value.native.acked.length, 1);
  assert.deepEqual(value.native.watchers.map(watch => watch.after), [0n, 1n]);
  assert.equal(browser._cdpSessionWatchState.pending, true);
});

test('two CDP children on one Browser share its single pending idle watch', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  let nextToken = 7n;
  value.native.cdpSessionCreate = async () => {
    const token = nextToken++;
    return { token: () => token, claim: () => token, abort: () => {} };
  };

  // Act
  const first = await context.newCDPSession(page);
  const second = await context.newCDPSession(page);

  // Assert
  assert.notEqual(first, second);
  assert.equal(context._cdpSessions.size, 2);
  assert.equal(browser._cdpSessionWatchState.pending, true);
  assert.deepEqual(value.native.watchers.map(watch => watch.after), [0n]);
});

test('the final CDP session stops the Browser watch without starting another', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  const session = await context.newCDPSession(page);
  const pending = value.native.watchers[0];

  // Act
  await session.detach();
  pending.resolve(1n);
  await settle();

  // Assert
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(browser._cdpSessionWatchState.active, false);
  assert.equal(value.native.watchers.length, 1);
});

test('Browser close stops a pending CDP watch and never re-arms it', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  await context.newCDPSession(page);
  const pending = value.native.watchers[0];

  // Act
  await browser.close();
  pending.resolve(1n);
  await settle();

  // Assert
  assert.equal(browser.isConnected(), false);
  assert.equal(browser._cdpSessionWatchState.terminal, true);
  assert.equal(value.native.watchers.length, 1);
});

test('native child retirement closes only its CDP wrapper when an idle watch wakes', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  const session = await context.newCDPSession(page);
  const closed = [];
  session.on('close', value => closed.push(value));
  value.native.cdpSessionClaim = async () => {
    throw new Error('PWRIGHT_TARGET_CLOSED: Target page, context or browser has been closed');
  };

  // Act
  value.native.watchers[0].resolve(1n);
  await settle();

  // Assert
  assert.deepEqual(closed, [session]);
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(browser.isConnected(), true);
  assert.equal(value.native.watchers.length, 1);
});

test('malformed native CDP events remain fail-closed instead of using the listener exception channel', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  await context.newCDPSession(page);
  value.native.events.push(['Runtime.invalid', []]);

  // Act
  value.native.watchers[0].resolve(1n);
  await settle();
  await settle();

  // Assert
  assert.equal(browser.isConnected(), false);
  assert.ok(value.calls.some(call => call[0] === 'close'));
  assert.deepEqual(value.native.listenerErrors, []);
  assert.equal(value.native.acked.length, 0);
});

test('a failed Browser close resumes the one pending CDP watch', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  await context.newCDPSession(page);
  value.native.close = async () => { throw new Error('close failed'); };

  // Act
  await assert.rejects(browser.close(), /close failed/);
  value.native.watchers[0].resolve(1n);
  await settle();

  // Assert
  assert.equal(browser.isConnected(), true);
  assert.equal(browser._cdpSessionWatchState.active, true);
  assert.deepEqual(value.native.watchers.map(watch => watch.after), [0n, 1n]);
});

test('late native CDP lease remains unclaimed while Browser close is pending', async () => {
  // Arrange
  const value = fixture();
  const { browser, context, page } = await openPage(value);
  const attachment = deferred();
  const closing = deferred();
  let claimed = 0;
  let aborted = 0;
  value.native.cdpSessionCreate = () => attachment.promise;
  value.native.close = () => closing.promise;
  const creation = context.newCDPSession(page);
  const closure = browser.close();

  // Act
  attachment.resolve({ token: () => 7n, claim: () => { claimed += 1; }, abort: () => { aborted += 1; } });
  await assert.rejects(creation, /Target page, context or browser has been closed/);
  closing.resolve();
  await closure;

  // Assert
  assert.equal(claimed, 0);
  assert.equal(aborted, 1);
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(value.native.watchers.length, 0);
});

test('late native CDP lease remains unclaimed while Context close is pending', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const attachment = deferred();
  const closing = deferred();
  let claimed = 0;
  let aborted = 0;
  value.native.cdpSessionCreate = () => attachment.promise;
  value.native.closeContext = () => closing.promise;
  const creation = context.newCDPSession(page);
  const closure = context.close();

  // Act
  attachment.resolve({ token: () => 7n, claim: () => { claimed += 1; }, abort: () => { aborted += 1; } });
  await assert.rejects(creation, /Target page, context or browser has been closed/);
  closing.resolve();
  await closure;

  // Assert
  assert.equal(claimed, 0);
  assert.equal(aborted, 1);
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(value.native.watchers.length, 0);
});

test('late native CDP lease remains unclaimed while Page close is pending', async () => {
  // Arrange
  const value = fixture();
  const { context, page } = await openPage(value);
  const attachment = deferred();
  const closing = deferred();
  let claimed = 0;
  let aborted = 0;
  value.native.cdpSessionCreate = () => attachment.promise;
  value.native.closePage = () => closing.promise;
  const creation = context.newCDPSession(page);
  const closure = page.close();

  // Act
  attachment.resolve({ token: () => 7n, claim: () => { claimed += 1; }, abort: () => { aborted += 1; } });
  await assert.rejects(creation, /Target page, context or browser has been closed/);
  closing.resolve();
  await closure;

  // Assert
  assert.equal(claimed, 0);
  assert.equal(aborted, 1);
  assert.equal(context._cdpSessions.size, 0);
  assert.equal(value.native.watchers.length, 0);
});
