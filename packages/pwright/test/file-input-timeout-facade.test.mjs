import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';
import { test } from 'node:test';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const debugUrl = new URL('../src/file-input-debug.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const debugSource = readFileSync(debugUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function debugCapture(env) {
  const module = { exports: {} };
  compileFunction(debugSource, ['module', 'process'], { filename: fileURLToPath(debugUrl) })(module, { env });
  return module.exports;
}

// Executes the production facade and its ordinary adapters. Only native module
// loading is replaced: this proves forwarding/cache behavior, not Rust brands,
// native timeout policy, Handle inheritance, session authority or browser work.
function fixture({ selected = true, env = {}, initializeError, cleanupError, signalWait } = {}) {
  const events = [];
  const sessions = [];
  const capturedDebug = debugCapture(env);
  const artifact = '/test-only/pwright-facade.node';
  const contextDescriptors = ['context-a', 'context-b'].map(id => ({ id, settings: { context: id } }));
  const descriptors = new Map(contextDescriptors.map(({ id }) => [id,
    [1, 2].map(number => {
      const pageId = `${id}-page-${number}`;
      const frameId = `${pageId}-main`;
      const tree = {
        identities: [{ id: frameId, parentId: null }, { id: `${pageId}-child`, parentId: frameId }],
        frames() { return this.identities; },
        metadata(id) { return { url: () => `https://example.invalid/${id}`, name: () => id }; },
      };
      return { id: pageId, mainFrameId: frameId, frameTree: tree };
    })]));

  class NativeEnvironment {
    constructor(...args) { events.push({ action: 'environment', args }); }
    async connectOverCDP(...args) {
      events.push({ action: 'connect', args });
      const session = {
        contexts: () => contextDescriptors,
        pages: id => descriptors.get(id),
        connected: true,
        initialized: false,
        publishedPages: new Set([...descriptors.values()].flat().map(({ id }) => id)),
        isConnected() { return this.connected; },
        setLifecycleCallback(callback) { this.lifecycle = callback; events.push({ action: 'lifecycle' }); },
        sessionSignal() {
          events.push({ action: 'signal' });
          // Unresolved observation only: no timer, worker, native task or I/O.
          return { wait: signalWait ?? (() => new Promise(() => {})) };
        },
        async close() {
          events.push({ action: 'close' });
          if (cleanupError) throw cleanupError;
          this.connected = false;
        },
        async closePage(id) { events.push({ action: 'closePage', id }); this.publishedPages.delete(id); },
        async handleDispose(handle) { events.push({ action: 'dispose', handle }); },
        async frameEvaluateHandle(...args) {
          assert.equal(this, session);
          events.push({ action: 'legacyEvaluate', args });
          return { isElement: () => false };
        },
        async frameEvaluate(...args) {
          assert.equal(this, session);
          events.push({ action: 'evaluate', args });
          return 'evaluated';
        },
      };
      if (selected) Object.assign(session, {
        initializeFileInputTimeouts(...args) {
          assert.equal(this, session);
          assert.equal(this.initialized, false);
          events.push({ action: 'initialize', args });
          if (initializeError) throw initializeError;
          this.initialized = true;
        },
        fileInputContextTimeoutSettings(id) {
          assert.equal(this, session);
          assert.equal(this.initialized, true);
          events.push({ action: 'contextCell', id });
          throw new Error('Facade must not query Context policy');
        },
        fileInputPageTimeoutSettings(id) {
          assert.equal(this, session);
          assert.equal(this.initialized, true);
          events.push({ action: 'pageCell', id });
          throw new Error('Facade must not query Page policy, especially before lifecycle commit');
        },
        async frameEvaluateHandleWithPageTimeout(...args) {
          assert.equal(this, session);
          events.push({ action: 'scopedEvaluate', args });
          if (!this.publishedPages.has(args[4])) throw new Error('simulated native closed Page');
          return { isElement: () => false };
        },
      });
      sessions.push({ session });
      return session;
    }
  }

  const module = { exports: {} };
  function require(specifier) {
    if (specifier === './file-input-debug.cjs') return capturedDebug;
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: selected ? 'retained-file-input' : 'handles' },
      verifyNativeArtifact: () => events.push({ action: 'verify' }),
    };
    if (specifier === artifact) { events.push({ action: 'load' }); return { NativeEnvironment }; }
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], { filename: fileURLToPath(indexUrl) })(require, module, module.exports);
  return { api: module.exports, events, sessions, descriptors, contextDescriptors, capturedDebug };
}

test('debug language port captures a frozen raw tuple once without interpreting values', () => {
  // Arrange
  const env = { PWDEBUG: '', npm_config_pwdebug: 'false', npm_package_config_pwdebug: 'console' };
  // Act
  const values = debugCapture(env);
  env.PWDEBUG = '1';
  env.npm_config_pwdebug = '0';
  delete env.npm_package_config_pwdebug;
  // Assert
  assert.deepEqual(values, ['', 'false', 'console']);
  assert.equal(Object.isFrozen(values), true);
  assert.throws(() => { values[0] = 'changed'; }, TypeError);
});

test('debug language port leaves absent values undefined and preserves fallback inputs', () => {
  // Arrange
  const env = { npm_package_config_pwdebug: 'FALSE' };
  // Act
  const values = debugCapture(env);
  // Assert
  assert.deepEqual(values, [undefined, undefined, 'FALSE']);
});

test('facade import captures debug without loading any native artifact', () => {
  // Arrange / Act
  const f = fixture({ env: { PWDEBUG: 'console' } });
  // Assert
  assert.deepEqual(f.events, []);
  assert.deepEqual(f.capturedDebug, ['console', undefined, undefined]);
});

test('each selected connection initializes once with the import-time tuple before Browser setup', async () => {
  // Arrange
  const env = { PWDEBUG: '', npm_config_pwdebug: '1', npm_package_config_pwdebug: 'console' };
  const f = fixture({ env });
  env.PWDEBUG = 'changed-after-import';
  // Act
  const first = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const second = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  // Assert
  assert.deepEqual(f.events.map(value => value.action), [
    'load', 'verify', 'environment', 'connect', 'initialize', 'lifecycle', 'signal',
    'connect', 'initialize', 'lifecycle', 'signal',
  ]);
  assert.deepEqual(f.events.filter(value => value.action === 'initialize').map(value => value.args), [
    ['', '1', 'console'], ['', '1', 'console'],
  ]);
  await first.close();
  await second.close();
});

test('idle Context, Page and Frame wrappers do not query or allocate timeout policy bindings', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  // Act
  const contexts = browser.contexts();
  const pages = contexts.map(context => context.pages());
  const frames = pages.flat().map(page => page.frames());
  const repeatedContexts = browser.contexts();
  const repeatedPages = contexts.map(context => context.pages());
  // Assert
  for (let index = 0; index < contexts.length; index++) {
    const context = contexts[index];
    assert.equal(context, repeatedContexts[index]);
    assert.equal(context._settings, f.contextDescriptors[index].settings);
    assert.equal(context._fileInputTimeoutSettings, undefined);
    for (let pageIndex = 0; pageIndex < pages[index].length; pageIndex++) {
      const page = pages[index][pageIndex];
      assert.equal(page, repeatedPages[index][pageIndex]);
      assert.equal(page._fileInputTimeoutSettings, undefined);
    }
  }
  assert.equal(frames.length, 4);
  assert.equal(f.events.filter(value => value.action === 'contextCell').length, 0);
  assert.equal(f.events.filter(value => value.action === 'pageCell').length, 0);
  await browser.close();
});

test('Handle creation on different Pages forwards distinct Page IDs without JS policy acquisition', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const pages = browser.contexts().flatMap(context => context.pages());
  // Act
  const handles = await Promise.all(pages.map(page => page.evaluateHandle('42')));
  // Assert
  for (const page of pages) assert.equal(page._fileInputTimeoutSettings, undefined);
  assert.deepEqual(f.events.filter(value => value.action === 'scopedEvaluate').map(value => value.args[4]), pages.map(page => page._id));
  assert.equal(f.events.filter(value => value.action === 'pageCell').length, 0);
  assert.equal(f.events.filter(value => value.action === 'contextCell').length, 0);
  for (const handle of handles) await handle.dispose();
  await browser.close();
});

test('Page, main Frame and child Frame creation forward the same creation Page ID with untouched arguments', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = browser.contexts()[0].pages()[0];
  const [main, child] = page.frames();
  const fn = value => ({ value });
  const argument = { retained: 'original identity' };
  // Act
  const handles = await Promise.all([
    page.evaluateHandle(fn, argument), main.evaluateHandle('({ main: true })', argument),
    child.evaluateHandle(fn, argument),
  ]);
  // Assert
  const calls = f.events.filter(value => value.action === 'scopedEvaluate');
  assert.deepEqual(calls.map(value => value.args[0]), [main._id, main._id, child._id]);
  assert.deepEqual(calls.map(value => value.args[1]), [String(fn), '({ main: true })', String(fn)]);
  assert.deepEqual(calls.map(value => value.args[2]), [true, false, true]);
  for (const { args } of calls) {
    assert.equal(args.length, 5);
    assert.equal(args[3], argument);
    assert.equal(args[4], page._id);
  }
  assert.equal(f.events.filter(value => value.action === 'pageCell').length, 0);
  assert.equal(f.events.some(value => value.action === 'legacyEvaluate'), false);
  for (const handle of handles) await handle.dispose();
  await browser.close();
});

test('selected Frame creation works when only the scoped native method is exported', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  delete f.sessions[0].session.frameEvaluateHandle;
  const page = browser.contexts()[0].pages()[0];
  // Act
  const handle = await page.evaluateHandle('42');
  // Assert
  assert.equal(f.events.filter(value => value.action === 'scopedEvaluate').length, 1);
  await handle.dispose();
  await browser.close();
});

test('legacy profiles preserve four-argument Handle creation without timeout metadata queries', async () => {
  // Arrange
  const f = fixture({ selected: false });
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  const argument = { original: true };
  // Act
  const handle = await page.evaluateHandle('42', argument);
  // Assert
  assert.equal(context._fileInputTimeoutSettings, undefined);
  assert.equal(page._fileInputTimeoutSettings, undefined);
  assert.deepEqual(f.events.filter(value => value.action === 'legacyEvaluate').map(value => value.args), [
    [page.mainFrame()._id, '42', false, argument],
  ]);
  assert.equal(f.events.some(value => ['initialize', 'contextCell', 'pageCell', 'scopedEvaluate'].includes(value.action)), false);
  await handle.dispose();
  await browser.close();
});

for (const selected of [false, true]) test(`expression conversion remains once and immediately before ${selected ? 'scoped' : 'legacy'} forwarding`, async () => {
  // Arrange
  const f = fixture({ selected });
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = browser.contexts()[0].pages()[0];
  const fn = () => 42;
  fn.toString = () => { f.events.push({ action: 'expression' }); return '() => 42'; };
  f.events.length = 0;
  // Act
  const handle = await page.evaluateHandle(fn);
  // Assert
  assert.deepEqual(f.events.map(value => value.action), ['expression', selected ? 'scopedEvaluate' : 'legacyEvaluate']);
  assert.equal(f.events.at(-1).args[1], '() => 42');
  await handle.dispose();
  await browser.close();
});

test('Page and Frame evaluation stringify non-function runtime values like Locator evaluation', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = browser.contexts()[0].pages()[0];
  const child = page.frames()[1];
  f.events.length = 0;
  // Act
  const values = await Promise.all([page.evaluate(42), child.evaluate(false)]);
  const handle = await page.evaluateHandle(null);
  // Assert
  assert.deepEqual(values, ['evaluated', 'evaluated']);
  assert.deepEqual(f.events.filter(value => value.action === 'evaluate').map(value => value.args.slice(1, 3)), [
    ['42', false], ['false', false],
  ]);
  assert.deepEqual(f.events.find(value => value.action === 'scopedEvaluate').args.slice(1, 3), ['null', false]);
  await handle.dispose();
  await browser.close();
});

test('lifecycle preparation does not query unpublished Page membership and first post-commit Handle creation succeeds', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const descriptor = f.descriptors.get('context-a')[0];
  const session = f.sessions[0].session;
  session.publishedPages.delete(descriptor.id);
  // Act
  const notice = session.lifecycle(2, 'context-a', descriptor.id);
  const publish = notice.prepare(f.contextDescriptors[0], descriptor);
  const preparedLookups = f.events.filter(value => value.action === 'pageCell' || value.action === 'contextCell');
  // Native commits membership before invoking the prepared publication callback.
  session.publishedPages.add(descriptor.id);
  publish();
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  const handle = await page.frames()[1].evaluateHandle('42');
  // Assert
  assert.equal(notice.contextNeeded, true);
  assert.equal(notice.pageNeeded, true);
  assert.deepEqual(preparedLookups, []);
  assert.equal(context._fileInputTimeoutSettings, undefined);
  assert.equal(page._fileInputTimeoutSettings, undefined);
  assert.equal(f.events.filter(value => value.action === 'contextCell').length, 0);
  assert.equal(f.events.filter(value => value.action === 'pageCell').length, 0);
  assert.equal(f.events.find(value => value.action === 'scopedEvaluate').args[4], descriptor.id);
  await handle.dispose();
  await browser.close();
});

test('source String conversion precedes a simulated asynchronous native closed-Page error without JS policy lookup', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = browser.contexts()[0].pages()[0];
  await page.close();
  const fn = () => 42;
  fn.toString = () => { f.events.push({ action: 'expression' }); return '42'; };
  f.events.length = 0;
  // Act
  const result = page.evaluateHandle(fn);
  // Assert
  await assert.rejects(result, /simulated native closed Page/);
  assert.deepEqual(f.events.map(value => value.action), ['expression', 'scopedEvaluate']);
  assert.equal(page._fileInputTimeoutSettings, undefined);
  assert.equal(f.events[1].args[4], page._id);
  // This fixture proves only facade order. It does not materialize arguments
  // or establish Rust argument-error precedence against actual closed metadata.
  await browser.close();
});

test('a throwing source String conversion on a closed Page reaches no native producer or metadata lookup', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = browser.contexts()[0].pages()[0];
  await page.close();
  const sentinel = new Error('source String conversion');
  const fn = () => 42;
  fn.toString = () => { throw sentinel; };
  f.events.length = 0;
  // Act
  const result = page.evaluateHandle(fn);
  // Assert
  await assert.rejects(result, error => error === sentinel);
  assert.deepEqual(f.events, []);
  await browser.close();
});

test('retained local Frame/Page wrappers keep their creation Page identity without JS policy after membership removal', async () => {
  // Arrange
  const f = fixture();
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = browser.contexts()[0].pages()[0];
  const child = page.frames()[1];
  const pageId = page._id;
  const descriptor = f.descriptors.get('context-a')[0];
  // Act
  descriptor.frameTree.identities = descriptor.frameTree.identities.slice(0, 1);
  const frames = page.frames();
  await page.close();
  await browser.close();
  f.sessions[0].session.publishedPages.clear();
  // Assert: local identity only; this does not claim a closed Frame can act.
  assert.deepEqual(frames, [page.mainFrame()]);
  assert.equal(child.page(), page);
  assert.equal(child.page()._id, pageId);
  assert.equal(page._fileInputTimeoutSettings, undefined);
  assert.equal(page._getFileInputTimeoutSettings, undefined);
  assert.equal(f.events.filter(value => value.action === 'pageCell').length, 0);
});

test('selected initialization failure closes the unpublished session and preserves the primary error', async () => {
  // Arrange
  const primary = new Error('initialization failed');
  const f = fixture({ initializeError: primary });
  // Act
  const result = f.api.chromium.connectOverCDP('http://example.invalid:9222');
  // Assert
  await assert.rejects(result, error => error === primary);
  assert.deepEqual(f.events.map(value => value.action), ['load', 'verify', 'environment', 'connect', 'initialize', 'close']);
  assert.equal(f.sessions[0].session.connected, false);
});

test('initialization and unpublished-session cleanup failures retain both errors', async () => {
  // Arrange
  const primary = new Error('initialization failed');
  const cleanup = new Error('session cleanup failed');
  const f = fixture({ initializeError: primary, cleanupError: cleanup });
  // Act
  const result = f.api.chromium.connectOverCDP('http://example.invalid:9222');
  // Assert
  await assert.rejects(result, error => {
    assert.ok(error instanceof AggregateError);
    assert.equal(error.cause, primary);
    assert.deepEqual(error.errors, [primary, cleanup]);
    return true;
  });
  assert.equal(f.events.filter(value => value.action === 'close').length, 1);
  assert.equal(f.events.some(value => value.action === 'signal'), false);
});

test('session watch rejection closes the session and publishes one disconnect without an unhandled rejection', async () => {
  // Arrange
  const failure = new Error('session signal failed');
  let rejectWatch;
  const f = fixture({ signalWait: () => new Promise((_, reject) => { rejectWatch = reject; }) });
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  const unhandled = [];
  const onUnhandled = error => unhandled.push(error);
  let disconnects = 0;
  browser.on('disconnected', () => { disconnects++; });
  process.on('unhandledRejection', onUnhandled);
  try {
    // Act
    rejectWatch(failure);
    await new Promise(resolve => setImmediate(resolve));
    // Assert
    assert.equal(browser.isConnected(), false);
    assert.equal(browser._watchFailure, failure);
    assert.equal(f.events.filter(value => value.action === 'close').length, 1);
    assert.equal(disconnects, 1);
    assert.deepEqual(unhandled, []);
  } finally {
    process.off('unhandledRejection', onUnhandled);
  }
});

test('session watch and cleanup failures preserve both causes and still disconnect locally', async () => {
  // Arrange
  const failure = new Error('session signal failed');
  const cleanup = new Error('session cleanup failed');
  let rejectWatch;
  const f = fixture({
    cleanupError: cleanup,
    signalWait: () => new Promise((_, reject) => { rejectWatch = reject; }),
  });
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  let disconnects = 0;
  browser.on('disconnected', () => { disconnects++; });
  // Act
  rejectWatch(failure);
  await new Promise(resolve => setImmediate(resolve));
  // Assert
  assert.equal(browser.isConnected(), false);
  assert.ok(browser._watchFailure instanceof AggregateError);
  assert.equal(browser._watchFailure.cause, failure);
  assert.deepEqual(browser._watchFailure.errors, [failure, cleanup]);
  assert.equal(disconnects, 1);
});

test('synchronous session watch failure is contained and closes the session', async () => {
  // Arrange
  const failure = new Error('synchronous session signal failure');
  const f = fixture({ signalWait: () => { throw failure; } });
  // Act
  const browser = await f.api.chromium.connectOverCDP('http://example.invalid:9222');
  await new Promise(resolve => setImmediate(resolve));
  // Assert
  assert.equal(browser.isConnected(), false);
  assert.equal(browser._watchFailure, failure);
  assert.equal(f.events.filter(value => value.action === 'close').length, 1);
});
