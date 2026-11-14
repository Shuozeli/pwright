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
const profile = 'playwright-server-connect-session';

class FakeRemotePage {
  constructor(guid, context) { this.id = guid; this.context = context; this.open = true; }
  guid() { return this.id; }
  isOpen() { return this.open && this.context.isOpen(); }
}

class FakeRemoteContext {
  constructor(guid, remote) { this.id = guid; this.remote = remote; this.open = true; this.pages = []; this.closes = 0; }
  guid() { return this.id; }
  isOpen() { return this.open && this.remote.connected; }
  pageGuids() { return this.pages.filter(page => page.open).map(page => page.guid()); }
  async newPage() {
    if (this.newPageError) throw this.newPageError;
    const page = new FakeRemotePage(`page@${this.pages.length + 1}`, this);
    this.pages.push(page);
    return page;
  }
  async close() {
    this.closes += 1;
    if (this.closeError) throw this.closeError;
    this.open = false;
  }
}

class FakeRemote {
  constructor() {
    this.connected = true;
    this.closes = 0;
    this.reported = [];
    this.contexts = [];
    this.terminal = new Promise(resolve => { this.resolveTerminal = resolve; });
  }
  version() { return '155.0'; }
  name() { return 'chromium'; }
  isConnected() { return this.connected; }
  supportsContextClose() { return true; }
  contextGuids() { return this.connected ? this.contexts.filter(context => context.open).map(context => context.guid()) : []; }
  async newContext() {
    const context = new FakeRemoteContext(`context@${this.contexts.length + 1}`, this);
    this.contexts.push(context);
    return context;
  }
  waitTerminal() { return this.terminal; }
  reportCallbackException(error) { this.reported.push(error); }
  async close() {
    this.closes += 1;
    this.connected = false;
    this.resolveTerminal(false);
  }
  remoteClose() {
    this.connected = false;
    this.resolveTerminal(false);
  }
}

function fixture(selected = true) {
  const calls = [];
  const remote = new FakeRemote();
  const artifact = '/test-only/server-connect-session.node';
  let loads = 0;
  class NativeEnvironment {
    async connectPlaywrightServer(...args) {
      calls.push(args);
      return remote;
    }
  }
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: path => path === artifact };
    if (specifier === 'node:os') return {
      platform: () => 'darwin', arch: () => 'arm64', release: () => '25.0.0',
    };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: selected ? profile : 'default' },
      nativeFeatures: name => name === profile ? [profile] : [],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) {
      loads += 1;
      return { NativeEnvironment };
    }
    return sourceRequire(specifier);
  }
  compileFunction(source, ['require', 'module', 'exports', '__dirname'], {
    filename: indexPath,
  })(require, module, module.exports, dirname(indexPath));
  return { api: module.exports, calls, remote, loads: () => loads };
}

test('session profile requires a distinct complete native ABI shape', () => {
  // Arrange
  class NativeEnvironment { connectPlaywrightServer() {} }
  class NativeRemoteBrowser {
    version() {}
    name() {}
    isConnected() {}
    close() {}
    waitTerminal() {}
    reportCallbackException() {}
    contextGuids() {}
    newContext() {}
  }
  class NativeRemoteContext {
    guid() {}
    isOpen() {}
    pageGuids() {}
    newPage() {}
  }
  class NativeRemotePage { guid() {} isOpen() {} }
  const binary = {
    nativeAbiVersion: () => 1,
    NativeEnvironment,
    NativeRemoteBrowser,
    NativeRemoteContext,
    NativeRemotePage,
    playwrightServerConnectSessionEnabled: () => true,
    playwrightServerConnectSessionAbiShape: () => 2,
  };
  const selected = artifacts.resolveNativeArtifact(profile);

  // Act and Assert
  assert.deepEqual(artifacts.nativeFeatures(profile), [profile]);
  assert.equal(artifacts.nativeFeatures('default').includes(profile), false);
  assert.doesNotThrow(() => artifacts.verifyNativeArtifact(binary, selected));
  assert.throws(() => artifacts.verifyNativeArtifact({ ...binary,
    playwrightServerConnectSessionAbiShape: undefined,
  }, selected), /profile mismatch/);
  assert.throws(() => artifacts.verifyNativeArtifact({ ...binary,
    NativeRemoteBrowser: class { version() {} },
  }, selected), /profile mismatch/);
  assert.throws(() => artifacts.verifyNativeArtifact(binary,
    artifacts.resolveNativeArtifact('default')), /profile mismatch/);
});

test('default profile does not load native code for BrowserType.connect', async () => {
  // Arrange
  const { api, loads } = fixture(false);

  // Act and Assert
  await assert.rejects(api.chromium.connect('ws://example.test/ws'), { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(loads(), 0);
});

test('selected profile returns a live remote Browser and disconnects once with itself', async () => {
  // Arrange
  const { api, calls, remote } = fixture();
  const browser = await api.chromium.connect({
    wsEndpoint: 'ws://example.test/ws?secret=synthetic',
    timeout: 0,
    headers: { foo: 'bar' },
  });
  const disconnected = [];
  browser.on('disconnected', value => disconnected.push(value));

  // Act
  assert.equal(browser.browserType(), api.chromium);
  assert.equal(browser.version(), '155.0');
  assert.equal(browser.isConnected(), true);
  await browser.close();
  await browser.close();

  // Assert
  assert.equal(browser.isConnected(), false);
  assert.deepEqual(disconnected, [browser]);
  assert.equal(remote.closes, 1);
  assert.deepEqual(calls, [[
    'ws://example.test/ws?secret=synthetic', 0, [['foo', 'bar']],
    'chromium', ['darwin', 'arm64', '25.0.0'],
  ]]);
});

test('remote-side terminal state emits disconnected without an explicit close', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.firefox.connect('http://example.test/prefix');
  const disconnected = new Promise(resolve => browser.once('disconnected', resolve));

  // Act
  remote.remoteClose();
  const value = await disconnected;

  // Assert
  assert.equal(value, browser);
  assert.equal(browser.isConnected(), false);
  await browser.close();
  assert.equal(remote.closes, 0);
});

test('remote terminal close emits Page then Context then Browser once', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  const events = [];
  page.on('close', value => {
    assert.equal(value, page);
    assert.equal(page.isClosed(), true);
    events.push('page');
  });
  context.on('close', value => {
    assert.equal(value, context);
    assert.deepEqual(context.pages(), []);
    events.push('context');
  });
  browser.on('disconnected', value => {
    assert.equal(value, browser);
    assert.deepEqual(browser.contexts(), []);
    events.push('browser');
  });
  const disconnected = new Promise(resolve => browser.once('disconnected', resolve));

  // Act
  remote.remoteClose();
  await disconnected;
  await browser.close();

  // Assert
  assert.deepEqual(events, ['page', 'context', 'browser']);
  assert.equal(remote.closes, 0);
});

test('explicit Browser.close emits terminal child closes before disconnected', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.webkit.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  const events = [];
  page.on('close', () => events.push('page'));
  context.on('close', () => events.push('context'));
  browser.on('disconnected', () => events.push('browser'));

  // Act
  await browser.close();
  await new Promise(resolve => setImmediate(resolve));

  // Assert
  assert.deepEqual(events, ['page', 'context', 'browser']);
  assert.equal(remote.closes, 1);
});

test('throwing Page close listener cannot suppress Context or Browser close', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.firefox.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  const original = new Error('page listener failed');
  const events = [];
  page.on('close', () => { throw original; });
  context.on('close', () => events.push('context'));
  browser.on('disconnected', () => events.push('browser'));
  const disconnected = new Promise(resolve => browser.once('disconnected', resolve));

  // Act
  remote.remoteClose();
  await disconnected;

  // Assert
  assert.deepEqual(events, ['context', 'browser']);
  assert.deepEqual(remote.reported, [original]);
  assert.equal(page.isClosed(), true);
});

test('remote Browser projects created Context and Page identities without fabrication', async () => {
  // Arrange
  const { api } = fixture();
  const browser = await api.webkit.connect('ws://example.test/ws');

  // Act
  assert.deepEqual(browser.contexts(), []);
  const context = await browser.newContext();
  const page = await context.newPage();

  // Assert
  assert.equal(context.browser(), browser);
  assert.deepEqual(browser.contexts(), [context]);
  assert.equal(page.context(), context);
  assert.deepEqual(context.pages(), [page]);
  assert.equal(page.isClosed(), false);
  assert.throws(() => page.goto('https://example.test'), { code: 'PWRIGHT_UNSUPPORTED' });
  await assert.rejects(browser.newContext({ viewport: { width: 800, height: 600 } }),
    { code: 'PWRIGHT_UNSUPPORTED' });
  await assert.rejects(browser.newPage({ baseURL: 'https://example.test' }),
    { code: 'PWRIGHT_UNSUPPORTED' });
  await browser.close();
  assert.equal(page.isClosed(), true);
});

test('explicit remote Context.close retires child Page first without disconnecting Browser', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  const events = [];
  page.on('close', value => { assert.equal(value, page); events.push('page'); });
  context.on('close', value => { assert.equal(value, context); events.push('context'); });
  browser.on('disconnected', () => events.push('browser'));

  // Act
  await context.close();
  await context.close();

  // Assert
  assert.deepEqual(events, ['page', 'context']);
  assert.equal(page.isClosed(), true);
  assert.deepEqual(context.pages(), []);
  assert.deepEqual(browser.contexts(), []);
  assert.equal(browser.isConnected(), true);
  assert.equal(remote.contexts[0].closes, 1);
  await browser.close();
  assert.deepEqual(events, ['page', 'context', 'browser']);
});

test('legacy remote addon without Context.close keeps the existing session usable', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  remote.contexts[0].close = undefined;

  // Act and Assert
  await assert.rejects(context.close(), { code: 'PWRIGHT_UNSUPPORTED' });
  remote.supportsContextClose = undefined;
  await assert.rejects(browser.newPage(), { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(remote.contexts.length, 1);
  assert.equal(browser.isConnected(), true);
  assert.deepEqual(browser.contexts(), [context]);
  await browser.close();
});

test('Browser.newPage owns an implicit Context and Page.close retires both', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const page = await browser.newPage();
  const context = page.context();
  const events = [];
  page.on('close', () => events.push('page'));
  context.on('close', () => events.push('context'));
  browser.on('disconnected', () => events.push('browser'));
  assert.deepEqual(browser.contexts(), [context]);

  // Act
  await page.close();
  await page.close();

  // Assert
  assert.deepEqual(events, ['page', 'context']);
  assert.equal(page.isClosed(), true);
  assert.deepEqual(browser.contexts(), []);
  assert.equal(browser.isConnected(), true);
  assert.equal(remote.contexts[0].closes, 1);
  await browser.close();
  assert.deepEqual(events, ['page', 'context', 'browser']);
});

test('Browser.newPage cleans implicit Context after remote Page creation failure', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const originalNewContext = remote.newContext.bind(remote);
  remote.newContext = async () => {
    const context = await originalNewContext();
    context.newPageError = new Error('remote Page creation failed');
    return context;
  };

  // Act
  await assert.rejects(browser.newPage(), /remote Page creation failed/);

  // Assert
  assert.equal(remote.contexts.length, 1);
  assert.equal(remote.contexts[0].closes, 1);
  assert.deepEqual(browser.contexts(), []);
  assert.equal(browser.isConnected(), true);
  await browser.close();
});

test('remote Context.close refusal preserves live wrappers and emits no close event', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();
  remote.contexts[0].closeError = new Error('server refusal');
  const events = [];
  page.on('close', () => events.push('page'));
  context.on('close', () => events.push('context'));

  // Act
  await assert.rejects(context.close(), /server refusal/);

  // Assert
  assert.deepEqual(events, []);
  assert.deepEqual(browser.contexts(), [context]);
  assert.deepEqual(context.pages(), [page]);
  assert.equal(browser.isConnected(), true);
  await browser.close();
});

test('Context inventory refuses a remote object without a typed wrapper', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  remote.contexts.push(new FakeRemoteContext('external@1', remote));

  // Act and Assert
  assert.throws(() => browser.contexts(), { code: 'PWRIGHT_UNSUPPORTED' });
  await browser.close();
});

test('GUID recycling evicts stale Context and Page wrappers before inventory lookup', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const context = await browser.newContext();
  const page = await context.newPage();

  // Act: the remote graph disposes each object and reuses its GUID.
  remote.contexts[0].pages[0].open = false;
  remote.contexts[0].pages.push(new FakeRemotePage(page._native.guid(), remote.contexts[0]));
  assert.throws(() => context.pages(), { code: 'PWRIGHT_UNSUPPORTED' });
  remote.contexts[0].open = false;
  remote.contexts.push(new FakeRemoteContext(context._native.guid(), remote));

  // Assert
  assert.throws(() => browser.contexts(), { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(context._pages.size, 0);
  assert.equal(browser._contexts.size, 0);
  await browser.close();
});

test('listener exceptions use the native exception channel without rejecting close', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const original = new Error('listener failed');
  browser.on('disconnected', () => { throw original; });

  // Act
  await browser.close();

  // Assert
  assert.equal(browser.isConnected(), false);
  assert.equal(remote.closes, 1);
  assert.deepEqual(remote.reported, [original]);
});

test('remote terminal watcher catches a thrown disconnected listener', async () => {
  // Arrange
  const { api, remote } = fixture();
  const browser = await api.chromium.connect('ws://example.test/ws');
  const original = new Error('remote disconnect listener failed');
  browser.on('disconnected', () => { throw original; });

  // Act
  remote.remoteClose();
  await new Promise(resolve => setImmediate(resolve));

  // Assert
  assert.equal(browser.isConnected(), false);
  assert.deepEqual(remote.reported, [original]);
  await browser.close();
  assert.equal(remote.closes, 0);
});

test('unsupported remote forwarding options fail before native admission', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act and Assert
  await assert.rejects(api.chromium.connect('ws://example.test/ws', { exposeNetwork: '*' }),
    { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, []);
});
