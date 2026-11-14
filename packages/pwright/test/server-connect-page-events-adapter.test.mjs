import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const source = readFileSync(indexUrl, 'utf8');
const indexPath = fileURLToPath(indexUrl);
const sourceRequire = createRequire(indexUrl);
const artifacts = sourceRequire('./native-artifact.cjs');
const profile = 'playwright-server-connect-page-events';
const sessionFeature = 'playwright-server-connect-session';

function id(guid, generation) { return { guid, generation }; }
function deferred() {
  let resolve;
  const promise = new Promise(complete => { resolve = complete; });
  return { promise, resolve };
}

class FakePage {
  constructor(context, guid, generation) {
    this.context = context;
    this.id = id(guid, generation);
    this.open = true;
    this.globs = [];
  }
  guid() { return this.id.guid; }
  createdSequence() { return this.id.generation; }
  isOpen() { return this.open && this.context.isOpen(); }
  urlGlob(pattern) {
    this.globs.push(pattern);
    if (pattern === '**/foo') return '^.*\\/foo$';
    if (pattern === 'https://example.test/a\\*b')
      return '^https:\\/\\/example\\.test\\/a\\*b$';
    return '^' + pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '$';
  }
}

class FakeContext {
  constructor(remote, guid, generation) {
    this.remote = remote;
    this.id = id(guid, generation);
    this.open = true;
    this.pages = [];
  }
  guid() { return this.id.guid; }
  createdSequence() { return this.id.generation; }
  isOpen() { return this.open && this.remote.connected; }
  pageGuids() { return this.pages.filter(page => page.isOpen()).map(page => page.guid()); }
  async newPage() {
    const page = new FakePage(this, this.remote.nextPageGuid(), this.remote.nextGeneration());
    this.pages.push(page);
    this.remote.subscriptionAcks += 2;
    if (this.remote.beforePageReturn) await this.remote.beforePageReturn(page);
    return page;
  }
  async close() {
    if (!this.open) return;
    if (this.closeError) throw this.closeError;
    this.open = false;
    for (const page of this.pages) {
      if (!page.open) continue;
      page.open = false;
      this.remote.push({ kind: 'pageClosed', object: page.id });
    }
    this.remote.push({ kind: 'contextClosed', object: this.id });
  }
}

class FakeRemote {
  constructor() {
    this.connected = true;
    this.contexts = [];
    this.pending = [];
    this.notices = [];
    this.reported = [];
    this.subscriptionAcks = 0;
    this.sequence = 7;
    this.reusePageGuid = false;
    this.closes = 0;
    this.noticeFailure = null;
  }
  nextGeneration() { return String(++this.sequence); }
  nextPageGuid() { return this.reusePageGuid ? 'page@same' : `page@${this.sequence + 1}`; }
  version() { return '155.0'; }
  name() { return 'chromium'; }
  isConnected() { return this.connected; }
  supportsContextClose() { return true; }
  contextGuids() { return this.contexts.filter(context => context.isOpen()).map(context => context.guid()); }
  async newContext() {
    const context = new FakeContext(this, `context@${this.contexts.length + 1}`, this.nextGeneration());
    this.contexts.push(context);
    if (this.beforeContextReturn) await this.beforeContextReturn(context);
    return context;
  }
  reportCallbackException(error) { this.reported.push(error); }
  nextPageNotice() {
    if (this.noticeFailure) return Promise.reject(this.noticeFailure);
    if (this.notices.length) return Promise.resolve(this.notices.shift());
    return new Promise((resolve, reject) => this.pending.push({ resolve, reject }));
  }
  push(notice) {
    const pending = this.pending.shift();
    if (pending) pending.resolve(notice);
    else this.notices.push(notice);
  }
  failNotice(error) {
    this.noticeFailure = error;
    for (const pending of this.pending.splice(0)) pending.reject(error);
  }
  async close() {
    if (!this.connected) return;
    if (this.closeError) throw this.closeError;
    this.closes += 1;
    for (const context of this.contexts) await context.close();
    this.connected = false;
    this.push({ kind: 'browserClosed', object: id('browser@1', '6') });
    this.push({ kind: 'disconnected' });
  }
  remoteClose() { return this.close(); }
}

function fixture() {
  const remote = new FakeRemote();
  const calls = [];
  const artifact = '/test-only/server-connect-page-events.node';
  class NativeEnvironment {
    async connectPlaywrightServerPageEvents(...args) {
      calls.push(args);
      return remote;
    }
    connectPlaywrightServer() { throw new Error('legacy connect port was selected'); }
  }
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: path => path === artifact };
    if (specifier === 'node:os') return {
      platform: () => 'darwin', arch: () => 'arm64', release: () => '25.0.0',
    };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile },
      nativeFeatures: name => name === profile ? [sessionFeature, profile] : [],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(source, ['require', 'module', 'exports', '__dirname'], {
    filename: indexPath,
  })(require, module, module.exports, dirname(indexPath));
  return { api: module.exports, remote, calls };
}

function requestNotice(page, requestId, url) {
  return {
    kind: 'request', page: page._native.id, object: requestId,
    request: {
      url, method: 'GET', resourceType: 'document',
      headers: [{ name: 'X-Test', value: 'yes' }], isNavigationRequest: true,
    },
  };
}

function responseNotice(page, responseId, requestId, url) {
  return {
    kind: 'response', page: page._native.id, object: responseId,
    response: {
      url, status: 201, statusText: 'Created',
      headers: [{ name: 'Content-Type', value: 'text/plain' }],
      fromServiceWorker: false, request: requestId,
    },
    requestSnapshot: requestNotice(page, requestId, url).request,
  };
}

test('Page-notice profile requires the complete opt-in native ABI', () => {
  // Arrange
  class NativeEnvironment {
    connectPlaywrightServer() {}
    connectPlaywrightServerPageEvents() {}
  }
  class NativeRemoteBrowser {
    version() {}
    name() {}
    isConnected() {}
    close() {}
    waitTerminal() {}
    nextPageNotice() {}
    reportCallbackException() {}
    contextGuids() {}
    newContext() {}
  }
  class NativeRemoteContext {
    guid() {}
    createdSequence() {}
    isOpen() {}
    pageGuids() {}
    newPage() {}
  }
  class NativeRemotePage {
    guid() {}
    createdSequence() {}
    urlGlob() {}
    isOpen() {}
  }
  const binary = {
    nativeAbiVersion: () => 1,
    NativeEnvironment, NativeRemoteBrowser, NativeRemoteContext, NativeRemotePage,
    playwrightServerConnectSessionEnabled: () => true,
    playwrightServerConnectSessionAbiShape: () => 2,
    playwrightServerConnectPageEventsEnabled: () => true,
    playwrightServerConnectPageEventsAbiShape: () => 1,
  };
  const selected = artifacts.resolveNativeArtifact(profile);

  // Act and Assert
  assert.deepEqual(artifacts.nativeFeatures(profile), [sessionFeature, profile]);
  assert.equal(artifacts.nativeFeatures('default').includes(profile), false);
  assert.doesNotThrow(() => artifacts.verifyNativeArtifact(binary, selected));
  assert.throws(() => artifacts.verifyNativeArtifact({ ...binary,
    playwrightServerConnectPageEventsEnabled: () => false,
  }, selected), /profile mismatch/);
  assert.throws(() => artifacts.verifyNativeArtifact({ ...binary,
    playwrightServerConnectPageEventsAbiShape: () => 0,
  }, selected), /profile mismatch/);
  assert.throws(() => artifacts.verifyNativeArtifact({ ...binary,
    NativeRemoteBrowser: class extends NativeRemoteBrowser { get nextPageNotice() { return undefined; } },
  }, selected), /profile mismatch/);
  assert.throws(() => artifacts.verifyNativeArtifact({ ...binary,
    NativeRemotePage: class extends NativeRemotePage { get urlGlob() { return undefined; } },
  }, selected), /profile mismatch/);
  assert.throws(() => artifacts.verifyNativeArtifact(binary,
    artifacts.resolveNativeArtifact(sessionFeature)), /profile mismatch/);
});

test('selected Page-notice profile publishes real request, response and download payloads', async () => {
  // Arrange
  const { api, remote, calls } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const events = [];
  page.on('request', request => events.push(['request', request.url()]));
  page.on('response', response => events.push(['response', response.status()]));
  page.on('download', download => events.push(['download', download.suggestedFilename()]));
  const requestWait = page.waitForRequest('**/foo');
  const responseWait = page.waitForResponse(/\/foo$/);
  const downloadWait = page.waitForEvent('download');
  const requestId = id('request@1', '10');
  const url = 'https://example.test/foo';

  // Act
  remote.push(requestNotice(page, requestId, url));
  remote.push(responseNotice(page, id('response@1', '11'), requestId, url));
  remote.push({ kind: 'download', page: page._native.id, object: id('artifact@1', '12'),
    url, suggestedFilename: 'foo.txt' });
  const [request, response, download] = await Promise.all([requestWait, responseWait, downloadWait]);

  // Assert
  assert.equal(remote.subscriptionAcks, 2);
  assert.deepEqual(calls, [[
    'ws://example.test/ws', 30_000, [], 'chromium', ['darwin', 'arm64', '25.0.0'],
  ]]);
  assert.deepEqual(page._native.globs, ['**/foo']);
  assert.deepEqual(events, [['request', url], ['response', 201], ['download', 'foo.txt']]);
  assert.equal(request.method(), 'GET');
  assert.equal(request.headers()['x-test'], 'yes');
  assert.equal(response.request(), request);
  assert.equal(response.ok(), true);
  assert.equal(response.headers()['content-type'], 'text/plain');
  assert.equal(download.url(), url);
  assert.equal(download.suggestedFilename(), 'foo.txt');
  await assert.rejects(download.path(), { code: 'PWRIGHT_UNSUPPORTED' });
  await browser.close();
});

test('escaped URL glob and async predicate match projected metadata', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const url = 'https://example.test/a*b';
  const escaped = page.waitForRequest('https://example.test/a\\*b');
  const predicate = page.waitForResponse(async response => response.status() === 201);
  const requestId = id('request@1', '10');

  // Act
  remote.push(requestNotice(page, requestId, url));
  remote.push(responseNotice(page, id('response@1', '11'), requestId, url));
  const [request, response] = await Promise.all([escaped, predicate]);

  // Assert
  assert.equal(request.url(), url);
  assert.equal(response.status(), 201);
  assert.deepEqual(page._native.globs, ['https://example.test/a\\*b']);
  await browser.close();
});

test('generic waitForEvent predicate skips a download before matching the next', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const wait = page.waitForEvent('download', {
    timeout: 0, predicate: download => download.suggestedFilename() === 'match.txt',
  });

  // Act
  remote.push({ kind: 'download', page: page._native.id, object: id('artifact@1', '12'),
    url: 'https://example.test/skip', suggestedFilename: 'skip.txt' });
  remote.push({ kind: 'download', page: page._native.id, object: id('artifact@2', '13'),
    url: 'https://example.test/match', suggestedFilename: 'match.txt' });
  const download = await wait;

  // Assert
  assert.equal(download.url(), 'https://example.test/match');
  await browser.close();
});

test('one pump rejects waiters before disconnected and before Browser.close resolves', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const order = [];
  page.on('close', () => order.push('page'));
  page.context().on('close', () => order.push('context'));
  browser.on('disconnected', () => order.push('disconnected'));
  const request = page.waitForRequest('**/never', { timeout: 0 }).catch(error => {
    order.push('request-rejected'); return error;
  });
  const response = page.waitForResponse('**/never', { timeout: 0 }).catch(error => {
    order.push('response-rejected'); return error;
  });
  const download = page.waitForEvent('download', { timeout: 0 }).catch(error => {
    order.push('download-rejected'); return error;
  });

  // Act
  await browser.close();
  order.push('close-resolved');
  const errors = await Promise.all([request, response, download]);

  // Assert
  assert.equal(remote.closes, 1);
  for (const error of errors) {
    assert.match(error.message, /Target page, context or browser has been closed/);
    assert.doesNotMatch(error.message, /Timeout/);
  }
  assert.deepEqual(order, [
    'page', 'request-rejected', 'response-rejected', 'download-rejected',
    'context', 'disconnected', 'close-resolved',
  ]);
});

test('remote close rejects download waiter before disconnected listener', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const order = [];
  const rejected = page.waitForEvent('download', { timeout: 0 }).catch(() => order.push('rejected'));
  browser.on('disconnected', () => order.push('disconnected'));
  const disconnected = new Promise(resolve => browser.once('disconnected', resolve));

  // Act
  await remote.remoteClose();
  await disconnected;
  await rejected;

  // Assert
  assert.deepEqual(order, ['rejected', 'disconnected']);
});

test('Browser.close waits for the typed terminal notice after native close resolves', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  remote.close = async () => { remote.connected = false; };
  const order = [];
  browser.on('disconnected', () => order.push('disconnected'));

  // Act
  const closing = browser.close().then(() => order.push('close-resolved'));
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(order, []);
  remote.push({ kind: 'browserClosed', object: id('browser@1', '6') });
  remote.push({ kind: 'disconnected' });
  await closing;

  // Assert
  assert.deepEqual(order, ['disconnected', 'close-resolved']);
});

test('native Browser.close refusal rejects without reporting a terminal success', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  remote.closeError = new Error('remote browser close refused');

  // Act and Assert
  await assert.rejects(browser.close(), /remote browser close refused/);
  assert.equal(browser.isConnected(), true);
  remote.closeError = null;
  await browser.close();
});

test('a disconnected notice without browserClosed fails the selected stream', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const disconnected = new Promise(resolve => browser.once('disconnected', resolve));

  // Act
  remote.push({ kind: 'disconnected' });
  await disconnected;

  // Assert
  await assert.rejects(browser.close(), /disconnected before browserClosed/);
});

test('32 waiters share a Page cap and release slots after delivery and close', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const waits = Array.from({ length: 32 }, () => page.waitForEvent('download', { timeout: 0 }));

  // Act
  await assert.rejects(page.waitForRequest('**/foo', { timeout: 0 }), /waiter capacity exceeds 32/);
  remote.push({ kind: 'download', page: page._native.id, object: id('artifact@1', '12'),
    url: 'https://example.test/foo', suggestedFilename: 'foo.txt' });
  await Promise.all(waits);
  const later = page.waitForEvent('download', { timeout: 0 }).catch(error => error);
  await browser.close();

  // Assert
  assert.equal(page._remoteWaiters.size, 0);
  assert.match((await later).message, /Target page, context or browser has been closed/);
});

test('recycled Page GUID cannot deliver an old generation notice to its successor', async () => {
  // Arrange
  const { api, remote } = fixture();
  remote.reusePageGuid = true;
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const oldPage = await context.newPage();
  oldPage._native.open = false;
  remote.push({ kind: 'pageClosed', object: oldPage._native.id });
  await new Promise(resolve => setImmediate(resolve));
  const page = await context.newPage();
  const wait = page.waitForRequest('**/foo', { timeout: 0 });

  // Act
  remote.push(requestNotice(oldPage, id('request@old', '10'), 'https://example.test/foo'));
  remote.push({ kind: 'pageClosed', object: oldPage._native.id });
  remote.push(requestNotice(page, id('request@new', '11'), 'https://example.test/foo'));
  const request = await wait;

  // Assert
  assert.equal(request._id.guid, 'request@new');
  assert.equal(page.isClosed(), false);
  await browser.close();
});

test('listener exception is reported without suppressing later response delivery', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const failure = new Error('consumer threw');
  page.on('request', () => { throw failure; });
  const responseWait = page.waitForResponse(/\/foo$/);
  const requestId = id('request@1', '10');

  // Act
  remote.push(requestNotice(page, requestId, 'https://example.test/foo'));
  remote.push(responseNotice(page, id('response@1', '11'), requestId, 'https://example.test/foo'));
  const response = await responseWait;

  // Assert
  assert.equal(response.status(), 201);
  assert.deepEqual(remote.reported, [failure]);
  await browser.close();
});

test('throwing Context listener is reported without suppressing Page delivery', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const failure = new Error('context consumer threw');
  const delivered = [];
  page.context().on('request', () => { throw failure; });
  page.on('request', request => delivered.push(request.url()));
  const wait = page.waitForRequest('**/foo', { timeout: 0 });

  // Act
  remote.push(requestNotice(page, id('request@1', '10'), 'https://example.test/foo'));
  const request = await wait;

  // Assert
  assert.equal(request.url(), 'https://example.test/foo');
  assert.deepEqual(delivered, ['https://example.test/foo']);
  assert.deepEqual(remote.reported, [failure]);
  await browser.close();
});

test('a matching waiter settles before its Page listener closes the Page', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const wait = page.waitForRequest('**/foo', { timeout: 0 });
  let closing;
  page.on('request', () => { closing = page.close(); });

  // Act
  remote.push(requestNotice(page, id('request@1', '10'), 'https://example.test/foo'));
  const request = await wait;
  await closing;
  await page.context().close();

  // Assert
  assert.equal(request.url(), 'https://example.test/foo');
  assert.equal(page.isClosed(), true);
  assert.equal(browser.isConnected(), true);
  await browser.close();
});

test('Context close before facade publication rejects creation and releases its bounded fence', { timeout: 1_000 }, async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const entered = deferred();
  const release = deferred();
  let native;
  remote.beforeContextReturn = async context => {
    native = context;
    remote.push({ kind: 'contextClosed', object: context.id });
    entered.resolve();
    await release.promise;
  };

  // Act
  const creating = browser.newContext();
  await entered.promise;
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(native.isOpen(), true); // The native graph can lag the close notice.
  assert.equal(browser._creationFences.contexts.terminals.size, 1);
  release.resolve();
  await assert.rejects(creating, { code: 'PWRIGHT_TARGET_CLOSED' });

  // Assert
  assert.equal(browser._contexts.size, 0);
  assert.equal(browser._creationFences.contexts.inFlight, 0);
  assert.equal(browser._creationFences.contexts.terminals.size, 0);
  native.open = false;
  remote.beforeContextReturn = null;
  const later = await browser.newContext();
  assert.deepEqual(browser.contexts(), [later]);
  await browser.close();
});

test('Page close before facade publication rejects creation and leaves later Page events usable', { timeout: 1_000 }, async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const entered = deferred();
  const release = deferred();
  let native;
  remote.beforePageReturn = async page => {
    native = page;
    remote.push({ kind: 'pageClosed', object: page.id });
    entered.resolve();
    await release.promise;
  };

  // Act
  const creating = context.newPage();
  await entered.promise;
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(native.isOpen(), true);
  assert.equal(browser._creationFences.pages.terminals.size, 1);
  release.resolve();
  await assert.rejects(creating, { code: 'PWRIGHT_TARGET_CLOSED' });

  // Assert
  assert.equal(context._pages.size, 0);
  assert.equal(browser._creationFences.pages.inFlight, 0);
  assert.equal(browser._creationFences.pages.terminals.size, 0);
  native.open = false;
  remote.beforePageReturn = null;
  const later = await context.newPage();
  const requestWait = later.waitForRequest('**/foo', { timeout: 0 });
  remote.push(requestNotice(later, id('request@later', '70'), 'https://example.test/foo'));
  assert.equal((await requestWait).url(), 'https://example.test/foo');
  await browser.close();
});

test('Context closure during pending Page publication orders listeners and settles close and waiters', { timeout: 1_000 }, async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const livePage = await context.newPage();
  const order = [];
  livePage.on('close', () => order.push('page-close'));
  context.on('close', () => order.push('context-close'));
  const closed = new Promise(resolve => context.once('close', resolve));
  const waiter = livePage.waitForEvent('download', { timeout: 0 }).catch(error => {
    order.push('waiter-rejected');
    return error;
  });
  const entered = deferred();
  const release = deferred();
  remote.beforePageReturn = async page => {
    remote.push({ kind: 'pageClosed', object: livePage._native.id });
    remote.push({ kind: 'pageClosed', object: page.id });
    remote.push({ kind: 'contextClosed', object: context._native.id });
    entered.resolve();
    await release.promise;
  };

  // Act
  const creating = context.newPage();
  await entered.promise;
  await closed;
  const waiterError = await waiter;
  await context.close();
  order.push('context-close-resolved');
  release.resolve();
  await assert.rejects(creating, { code: 'PWRIGHT_TARGET_CLOSED' });

  // Assert
  assert.equal(waiterError.code, 'PWRIGHT_TARGET_CLOSED');
  assert.doesNotMatch(waiterError.message, /Timeout/);
  assert.deepEqual(order, ['page-close', 'waiter-rejected', 'context-close', 'context-close-resolved']);
  assert.equal(browser._creationFences.pages.terminals.size, 0);
  context._native.open = false;
  remote.beforePageReturn = null;
  await browser.close();
});

test('a false close predicate keeps the terminal waiter pending until its timeout', { timeout: 1_000 }, async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const matched = page.waitForEvent('close', { timeout: 0, predicate: value => value === page });
  const rejected = page.waitForEvent('close', { timeout: 50, predicate: () => false });
  const undefinedResult = page.waitForEvent('close', { timeout: 50, predicate: () => undefined });

  // Act
  remote.push({ kind: 'pageClosed', object: page._native.id });
  const value = await matched;
  assert.equal(page._remoteWaiters.size, 2);
  const errors = await Promise.all([rejected.catch(error => error), undefinedResult.catch(error => error)]);

  // Assert
  assert.equal(value, page);
  assert.deepEqual(errors.map(error => error.name), ['TimeoutError', 'TimeoutError']);
  assert.equal(page._remoteWaiters.size, 0);
  await browser.close();
});

test('a false crash predicate also times out while a true one resolves', { timeout: 1_000 }, async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const matched = page.waitForEvent('crash', { timeout: 0, predicate: value => value === page });
  const rejected = page.waitForEvent('crash', { timeout: 50, predicate: () => false });

  // Act
  remote.push({ kind: 'pageCrashed', object: page._native.id });
  const value = await matched;
  assert.equal(page._remoteWaiters.size, 1);
  const error = await rejected.catch(failure => failure);

  // Assert
  assert.equal(value, page);
  assert.equal(error.name, 'TimeoutError');
  assert.equal(page._remoteWaiters.size, 0);
  await browser.close();
});

test('an asynchronous terminal predicate failure is returned to its waiter', { timeout: 1_000 }, async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const failure = new Error('terminal predicate failed');
  const wait = page.waitForEvent('close', { timeout: 0, predicate: async () => { throw failure; } });

  // Act
  remote.push({ kind: 'pageClosed', object: page._native.id });
  const error = await wait.catch(value => value);

  // Assert
  assert.equal(error, failure);
  await browser.close();
});

test('pending async predicate rejects with target closed when Page closes', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  let entered;
  const predicateEntered = new Promise(resolve => { entered = resolve; });
  let accept;
  const decision = new Promise(resolve => { accept = resolve; });
  const wait = page.waitForRequest(async () => {
    entered();
    return decision;
  }, { timeout: 0 });

  // Act
  remote.push(requestNotice(page, id('request@1', '10'), 'https://example.test/foo'));
  await predicateEntered;
  remote.push({ kind: 'pageClosed', object: page._native.id });
  const error = await wait.catch(value => value);
  accept(true);

  // Assert
  assert.equal(error.code, 'PWRIGHT_TARGET_CLOSED');
  assert.match(error.message, /Target page, context or browser has been closed/);
  assert.doesNotMatch(error.message, /Timeout/);
  await browser.close();
});

test('resolved async predicate for an earlier event wins over a later Page close', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const wait = page.waitForResponse(async response => response.status() === 201, { timeout: 0 });
  const closed = new Promise(resolve => page.once('close', resolve));
  const requestId = id('request@1', '10');

  // Act
  remote.push(responseNotice(page, id('response@1', '11'), requestId, 'https://example.test/foo'));
  remote.push({ kind: 'pageClosed', object: page._native.id });
  const response = await wait;
  await closed;

  // Assert
  assert.equal(response.status(), 201);
  assert.equal(page.isClosed(), true);
  await browser.close();
});

test('RegExp matcher preserves the caller lastIndex across notices like pinned Playwright', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const regexp = /\/foo$/g;
  const url = 'https://example.test/foo';
  const firstWait = page.waitForRequest(regexp, { timeout: 0 });

  // Act
  // Pinned Playwright v1.58.0 and v1.58.2 urlMatches call match.test(urlString)
  // on this same RegExp, so the second identical URL resets lastIndex.
  remote.push(requestNotice(page, id('request@1', '10'), url));
  await firstWait;
  assert.equal(regexp.lastIndex, url.length);
  const secondWait = page.waitForRequest(regexp, { timeout: 0 });
  const skipped = new Promise(resolve => {
    page.on('request', request => {
      if (request._id.guid === 'request@2') resolve();
    });
  });
  remote.push(requestNotice(page, id('request@2', '11'), url));
  await skipped;
  assert.equal(regexp.lastIndex, 0);
  assert.equal(page._remoteWaiters.size, 1);
  remote.push(requestNotice(page, id('request@3', '12'), url));
  const second = await secondWait;

  // Assert
  assert.equal(second._id.guid, 'request@3');
  assert.equal(regexp.lastIndex, url.length);
  await browser.close();
});

test('native Context.close refusal rejects promptly and leaves the session usable', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  remote.contexts[0].closeError = new Error('remote close refused');

  // Act and Assert
  await assert.rejects(context.close(), /remote close refused/);
  assert.equal(page.isClosed(), false);
  assert.deepEqual(browser.contexts(), [context]);
  remote.contexts[0].closeError = null;
  await browser.close();
});

test('projection failure releases pending close calls and reports failure after ordered teardown', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  const order = [];
  page.on('close', () => order.push('page'));
  context.on('close', () => order.push('context'));
  browser.on('disconnected', () => order.push('disconnected'));
  const waiter = page.waitForEvent('download', { timeout: 0 }).catch(error => {
    order.push('waiter-rejected');
    return error;
  });
  context._native.close = () => new Promise(() => {});
  remote.close = () => new Promise(() => {});
  const contextClose = context.close();
  const browserClose = browser.close();
  const failure = new Error('projector failed');

  // Act
  remote.failNotice(failure);
  const [waiterError, contextError, browserError] = await Promise.all([
    waiter,
    contextClose.catch(error => error),
    browserClose.catch(error => error),
  ]);

  // Assert
  assert.equal(waiterError.code, 'PWRIGHT_TARGET_CLOSED');
  assert.equal(contextError, failure);
  assert.equal(browserError, failure);
  assert.deepEqual(order, ['page', 'waiter-rejected', 'context', 'disconnected']);
  await assert.rejects(browser.close(), error => error === failure);
  await assert.rejects(context.close(), error => error === failure);
});
