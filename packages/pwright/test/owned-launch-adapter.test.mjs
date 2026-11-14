import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function fixture(profile = 'owned-launch', executableExists = () => true,
    startupFailure = new Error('PWRIGHT_BACKEND: Failed to launch the browser process.\nBrowser logs:\n[exit=1]')) {
  const calls = [];
  const contexts = new Set();
  let connected = true;
  const session = {
    isConnected: () => connected,
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    setLifecycleCallback(callback) { this.lifecycleCallback = callback; },
    contextSnapshot(ids) {
      return {
        ids: [...contexts],
        newContexts: [...contexts].filter(id => !ids.includes(id)).map(id => ({ id, settings: {} })),
      };
    },
    async newContext(baseURL) {
      calls.push(['newContext', baseURL]);
      contexts.add('fresh-context');
      return { id: 'fresh-context', settings: {} };
    },
    publishDisconnect: () => true,
    async close() { calls.push(['close']); connected = false; },
  };
  class NativeEnvironment {
    async launchChromium(...args) {
      calls.push(['launchChromium', ...args]);
      return session;
    }
    async launchChromiumWithTracesDir(...args) {
      calls.push(['launchChromiumWithTracesDir', ...args]);
      return session;
    }
    async observeBidiStartupFailure(...args) {
      calls.push(['observeBidiStartupFailure', ...args]);
      if (startupFailure) throw startupFailure;
    }
    async connectOverCDP(...args) {
      calls.push(['connectOverCDP', ...args]);
      return session;
    }
  }
  const artifact = '/test-only/owned-launch.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: executableExists };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile },
      nativeFeatures: selected => selected === 'owned-launch' ? ['owned-launch']
        : selected === 'playwright-trace' ? ['owned-launch', 'playwright-trace']
          : selected === 'bidi-startup-failure'
            ? ['owned-launch', 'playwright-trace', 'bidi-startup-failure'] : [],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, module, module.exports);
  return { api: module.exports, calls, session };
}

test('fresh Chromium launch delegates explicit options and returns a Browser with no default Context', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium', headless: false, timeout: 42 });
  assert.deepEqual(browser.contexts(), []);
  const context = await browser.newContext();
  await browser.close();

  // Assert
  assert.equal(browser.browserType(), api.chromium);
  assert.equal(context.browser(), browser);
  assert.deepEqual(calls, [
    ['launchChromium', '/test-only/chromium', false, 42],
    ['newContext', undefined],
    ['close'],
  ]);
  assert.equal(api.compatibility.browserOwnership, 'external-and-owned');
});

test('opt-in BiDi Chromium startup failure uses the explicit executable and inherited defaults', async () => {
  // Arrange
  const { api, calls } = fixture('bidi-startup-failure');
  api._defaultLaunchOptions = {
    channel: 'bidi-chrome-canary',
    firefoxUserPrefs: { 'remote.bidi.dismiss_file_pickers.enabled': true },
    tracesDir: '/test-only/traces',
    handleSIGINT: false,
    headless: true,
  };

  // Act
  const launching = api.chromium.launch({ executablePath: '/test-only/exit-one.js' });

  // Assert
  await assert.rejects(launching, {
    message: 'browserType.launch: Failed to launch the browser process.\nBrowser logs:\n[exit=1]',
  });
  assert.deepEqual(calls, [[
    'observeBidiStartupFailure', 'chromium', '/test-only/exit-one.js', 30_000,
    '/test-only/traces', process.cwd(),
  ]]);
});

test('opt-in BiDi Firefox startup failure reaches the distinct Rust route', async () => {
  // Arrange
  const { api, calls } = fixture('bidi-startup-failure');
  api._defaultLaunchOptions = { channel: 'moz-firefox-nightly', firefoxUserPrefs: {}, tracesDir: '' };

  // Act
  const launching = api.firefox.launch({ executablePath: '/test-only/exit-one.js', timeout: 17 });

  // Assert
  await assert.rejects(launching, /Browser logs:/);
  assert.deepEqual(calls, [[
    'observeBidiStartupFailure', 'firefox', '/test-only/exit-one.js', 17, '', process.cwd(),
  ]]);
});

for (const [engine, channel] of [
  ['chromium', 'bidi-chromium'],
  ['chromium', 'bidi-chrome-canary'],
  ['chromium', 'bidi-chrome-stable'],
  ['firefox', 'moz-firefox'],
  ['firefox', 'moz-firefox-beta'],
  ['firefox', 'moz-firefox-nightly'],
]) {
  test(`BiDi startup failure selects the bounded ${channel} channel`, async () => {
    // Arrange
    const { api, calls } = fixture('bidi-startup-failure');
    api._defaultLaunchOptions = { channel, firefoxUserPrefs: {} };

    // Act
    const launching = api[engine].launch({ executablePath: '/test-only/exit-one.js' });

    // Assert
    await assert.rejects(launching, /Browser logs:/);
    assert.equal(calls.length, 1);
    assert.equal(calls[0][1], engine);
  });
}

test('BiDi startup failure remains unavailable in the older owned-launch profile', async () => {
  // Arrange
  const { api, calls } = fixture('owned-launch');
  api._defaultLaunchOptions = { channel: 'bidi-chromium', firefoxUserPrefs: {} };

  // Act
  const launching = api.chromium.launch({ executablePath: '/test-only/exit-one.js' });

  // Assert
  await assert.rejects(launching, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, []);
});

test('BiDi startup route defers missing-executable validation to the Rust boundary', async () => {
  // Arrange
  const { api, calls } = fixture('bidi-startup-failure',
    path => path === '/test-only/owned-launch.node');
  api._defaultLaunchOptions = { channel: 'moz-firefox', tracesDir: '/test-only/traces' };

  // Act
  const launching = api.firefox.launch({ executablePath: '/test-only/missing-browser' });

  // Assert
  await assert.rejects(launching, /Browser logs:/);
  assert.deepEqual(calls, [[
    'observeBidiStartupFailure', 'firefox', '/test-only/missing-browser', 30_000,
    '/test-only/traces', process.cwd(),
  ]]);
});

test('BiDi startup failure refuses unknown channels and inherited-only executables', async () => {
  // Arrange
  const unknown = fixture('bidi-startup-failure');
  unknown.api._defaultLaunchOptions = { channel: 'chrome', firefoxUserPrefs: {} };
  const inherited = fixture('bidi-startup-failure');
  inherited.api._defaultLaunchOptions = {
    channel: 'moz-firefox', firefoxUserPrefs: {}, executablePath: '/test-only/exit-one.js',
  };

  // Act
  const unknownLaunch = unknown.api.chromium.launch({ executablePath: '/test-only/exit-one.js' });
  const inheritedLaunch = inherited.api.firefox.launch();

  // Assert
  await assert.rejects(unknownLaunch, { code: 'PWRIGHT_UNSUPPORTED' });
  await assert.rejects(inheritedLaunch, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(unknown.calls, []);
  assert.deepEqual(inherited.calls, []);
});

test('BiDi startup route rejects invalid preferences before acquiring native environment', async () => {
  // Arrange
  const { api, calls } = fixture('bidi-startup-failure');
  api._defaultLaunchOptions = { channel: 'moz-firefox-beta', firefoxUserPrefs: [] };

  // Act
  const launching = api.firefox.launch({ executablePath: '/test-only/exit-one.js' });

  // Assert
  await assert.rejects(launching, { name: 'TypeError', message: 'browserType.launch: firefoxUserPrefs must be an object' });
  assert.deepEqual(calls, []);
});

test('BiDi startup route never returns a Browser when its native observer resolves', async () => {
  // Arrange
  const { api, calls } = fixture('bidi-startup-failure', () => true, null);
  api._defaultLaunchOptions = { channel: 'bidi-chromium' };

  // Act
  const launching = api.chromium.launch({ executablePath: '/test-only/exit-one.js' });

  // Assert
  await assert.rejects(launching, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(calls.length, 1);
});

test('fresh Chromium launch uses inherited worker defaults when explicit options are absent', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = {
    executablePath: '/test-only/chromium',
    headless: false,
    timeout: 42,
  };

  // Act
  const browser = await api.chromium.launch();
  await browser.close();

  // Assert
  assert.deepEqual(calls, [
    ['launchChromium', '/test-only/chromium', false, 42],
    ['close'],
  ]);
});

test('explicit launch options override inherited defaults after reading each default getter once', async () => {
  // Arrange
  const { api, calls } = fixture();
  let headlessReads = 0;
  api._defaultLaunchOptions = {
    executablePath: '/test-only/default-chromium',
    get headless() { headlessReads++; return false; },
    timeout: 100,
  };

  // Act
  const browser = await api.chromium.launch({
    executablePath: '/test-only/explicit-chromium',
    headless: true,
    timeout: 17,
  });
  await browser.close();

  // Assert
  assert.equal(headlessReads, 1);
  assert.deepEqual(calls, [
    ['launchChromium', '/test-only/explicit-chromium', true, 17],
    ['close'],
  ]);
});

test('an explicit port rejection precedes invalid inherited launch arguments', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = { args: ['http://example.test/default-page'] };

  // Act
  const launching = api.chromium.launch({ port: 1234 });

  // Assert
  await assert.rejects(launching, { message: 'Cannot specify a port without launching as a server.' });
  assert.deepEqual(calls, []);
});

test('inherited launch arguments reach the same pre-spawn validation as explicit arguments', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = {
    executablePath: '/test-only/chromium',
    args: ['http://example.test/default-page'],
  };

  // Act
  const launching = api.chromium.launch();

  // Assert
  await assert.rejects(launching, { message: 'Arguments can not specify page to be opened' });
  assert.deepEqual(calls, []);
});

test('unsupported inherited options remain explicit failures after the merge', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = {
    executablePath: '/test-only/chromium',
    handleSIGINT: false,
    tracesDir: '/test-only/traces',
  };

  // Act
  const launching = api.chromium.launch();

  // Assert
  await assert.rejects(launching, { code: 'PWRIGHT_UNSUPPORTED', message: 'browserType.launch option tracesDir is not supported by the current Pwright native profile' });
  assert.deepEqual(calls, []);
});

test('opt-in trace path preserves inherited relative tracesDir for the Rust launch boundary', async () => {
  // Arrange
  const { api, calls } = fixture('playwright-trace');
  api._defaultLaunchOptions = {
    executablePath: '/test-only/chromium',
    tracesDir: 'relative/traces',
  };

  // Act
  const browser = await api.chromium.launch();
  await browser.close();

  // Assert
  assert.deepEqual(calls, [
    ['launchChromiumWithTracesDir', '/test-only/chromium', true, 30_000, 'relative/traces', process.cwd()],
    ['close'],
  ]);
});

test('opt-in trace path validates type before entering the Rust launch boundary', async () => {
  // Arrange
  const { api, calls } = fixture('playwright-trace');

  // Act
  const launching = api.chromium.launch({ executablePath: '/test-only/chromium', tracesDir: 42 });

  // Assert
  await assert.rejects(launching, { name: 'TypeError', message: 'browserType.launch: tracesDir must be a string' });
  assert.deepEqual(calls, []);
});

test('empty tracesDir reaches the selected Rust method without inventing a caller path', async () => {
  // Arrange
  const { api, calls } = fixture('playwright-trace');

  // Act
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium', tracesDir: '' });
  await browser.close();

  // Assert
  assert.deepEqual(calls, [
    ['launchChromiumWithTracesDir', '/test-only/chromium', true, 30_000, '', process.cwd()],
    ['close'],
  ]);
});

test('explicit handleSIGINT false selects the existing no-handler launch policy', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium', handleSIGINT: false });
  await browser.close();

  // Assert
  assert.deepEqual(calls, [
    ['launchChromium', '/test-only/chromium', true, 30_000],
    ['close'],
  ]);
});

test('explicit handleSIGINT false overrides inherited true before launch', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = { executablePath: '/test-only/chromium', handleSIGINT: true };

  // Act
  const browser = await api.chromium.launch({ handleSIGINT: false });
  await browser.close();

  // Assert
  assert.deepEqual(calls, [
    ['launchChromium', '/test-only/chromium', true, 30_000],
    ['close'],
  ]);
});

test('handleSIGINT true stays unsupported before acquiring the native environment', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = { executablePath: '/test-only/chromium', handleSIGINT: true };

  // Act
  const launching = api.chromium.launch();

  // Assert
  await assert.rejects(launching, {
    code: 'PWRIGHT_UNSUPPORTED',
    message: 'browserType.launch option handleSIGINT=true is not supported by the current Pwright native profile',
  });
  assert.deepEqual(calls, []);
});

test('nonboolean handleSIGINT fails preflight before acquiring the native environment', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act
  const launching = api.chromium.launch({ executablePath: '/test-only/chromium', handleSIGINT: 'false' });

  // Assert
  await assert.rejects(launching, { name: 'TypeError', message: 'browserType.launch: handleSIGINT must be a boolean' });
  assert.deepEqual(calls, []);
});

test('inherited launch options never invent a missing browser executable', async () => {
  // Arrange
  const { api, calls } = fixture();
  api._defaultLaunchOptions = { headless: false };

  // Act
  const launching = api.chromium.launch();

  // Assert
  await assert.rejects(launching, { name: 'TypeError', message: 'browserType.launch: executablePath must be a nonempty absolute path' });
  assert.deepEqual(calls, []);
});

test('fresh-owned Browser.close emits one Context close after native shutdown, including repeated calls', async () => {
  // Arrange
  const { api, calls, session } = fixture();
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium' });
  const context = await browser.newContext();
  const events = [];
  const order = [];
  context.on('close', function (closed) {
    events.push([this, closed, browser._contexts.has(context._id)]);
    order.push('context');
  });
  browser.on('disconnected', () => order.push('browser'));
  context._request = { _disposeFromOwner(reason) { calls.push(['disposeRequest', reason]); } };
  const nativeClose = session.close;
  let nativeCloses = 0;
  session.close = async () => {
    if (nativeCloses++ === 0) assert.deepEqual(events, []);
    await nativeClose();
  };

  // Act
  await Promise.all([browser.close(), browser.close()]);
  await browser.close();

  // Assert
  assert.deepEqual(events, [[context, context, false]]);
  assert.deepEqual(order, ['context', 'browser']);
  assert.deepEqual(calls.filter(([kind]) => kind === 'disposeRequest'), [['disposeRequest', 'Browser closed']]);
  assert.equal(browser.isConnected(), false);
});

test('fresh-owned Browser.close does not repeat a native Context close notice', async () => {
  // Arrange
  const { api, calls, session } = fixture();
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium' });
  const context = await browser.newContext();
  const events = [];
  context.on('close', closed => events.push(closed));
  context._request = { _disposeFromOwner(reason) { calls.push(['disposeRequest', reason]); } };
  const nativeClose = session.close;
  session.close = async () => {
    const commit = session.lifecycleCallback(1, context._id, '');
    commit();
    await nativeClose();
  };

  // Act
  await browser.close();

  // Assert
  assert.deepEqual(events, [context]);
  assert.deepEqual(calls.filter(([kind]) => kind === 'disposeRequest'), [['disposeRequest', 'Browser closed']]);
});

test('fresh-owned disconnect watcher emits Context close before Browser disconnected during shutdown', async () => {
  // Arrange
  const { api, session } = fixture();
  let releaseSignal;
  session.sessionSignal = () => ({ wait: () => new Promise(resolve => { releaseSignal = resolve; }) });
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium' });
  await Promise.resolve();
  const context = await browser.newContext();
  const events = [];
  context.on('close', () => events.push('context'));
  browser.on('disconnected', () => events.push('browser'));
  const nativeClose = session.close;
  session.close = async () => {
    releaseSignal(false);
    await Promise.resolve();
    await nativeClose();
  };

  // Act
  await browser.close();

  // Assert
  assert.deepEqual(events, ['context', 'browser']);
});

test('fresh-owned Context listener exception preserves shutdown and reaches the native exception channel', async () => {
  // Arrange
  const { api, session } = fixture();
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium' });
  const context = await browser.newContext();
  const listenerError = new Error('listener failed');
  const reported = [];
  const events = [];
  session.reportCallbackException = error => reported.push(error);
  context.on('close', () => { throw listenerError; });
  browser.on('disconnected', () => events.push('disconnected'));

  // Act
  await browser.close();

  // Assert
  assert.deepEqual(reported, [listenerError]);
  assert.deepEqual(events, ['disconnected']);
  assert.equal(browser.isConnected(), false);
});

test('failed fresh-owned shutdown does not fabricate a Context close event', async () => {
  // Arrange
  const { api, session } = fixture();
  const browser = await api.chromium.launch({ executablePath: '/test-only/chromium' });
  const context = await browser.newContext();
  const events = [];
  context.on('close', closed => events.push(closed));
  session.close = async () => { throw new Error('native shutdown failed'); };

  // Act
  const closing = browser.close();

  // Assert
  await assert.rejects(closing, /native shutdown failed/);
  assert.deepEqual(events, []);
  assert.equal(browser._contexts.get(context._id), context);
});

for (const profile of ['default', 'owned-launch']) {
  test(`${profile} CDP attachment does not synthesize Context close on Browser.close`, async () => {
    // Arrange
    const { api } = fixture(profile);
    const browser = await api.chromium.connectOverCDP('http://example.invalid:9222');
    const context = await browser.newContext();
    const events = [];
    context.on('close', closed => events.push(closed));

    // Act
    await browser.close();

    // Assert
    assert.deepEqual(events, []);
  });
}

test('opt-in profile preserves CDP attach and launch defaults', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act
  const owned = await api.chromium.launch({ executablePath: '/test-only/chromium' });
  await owned.close();
  const attached = await api.chromium.connectOverCDP('http://example.invalid:9222');
  await attached.close();

  // Assert
  assert.deepEqual(calls, [
    ['launchChromium', '/test-only/chromium', true, 30_000],
    ['close'],
    ['connectOverCDP', 'http://example.invalid:9222', 30_000],
    ['close'],
  ]);
});

test('owned-launch profile does not enter a browser for invalid persistent-context arguments', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act
  const launch = api.chromium.launchPersistentContext('unused-profile', {
    args: ['http://example.test/page'],
  });

  // Assert
  await assert.rejects(launch, {
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  });
  assert.deepEqual(calls, []);
});

test('owned-launch profile still rejects valid persistent-context launches', async () => {
  // Arrange
  const { api, calls } = fixture();

  // Act
  const launch = api.chromium.launchPersistentContext('unused-profile');

  // Assert
  await assert.rejects(launch, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, []);
});

test('launch rejects unsupported engines, options and invalid values before native acquisition', async () => {
  // Arrange
  const { api, calls } = fixture();
  const invalid = [
    api.firefox.launch({ executablePath: '/test-only/firefox' }),
    api.webkit.launch({ executablePath: '/test-only/webkit' }),
    api.chromium.launch({ executablePath: '/test-only/chromium', args: ['--unsupported'] }),
    api.chromium.launch({ executablePath: 'relative/chromium' }),
    api.chromium.launch({ executablePath: '/test-only/chromium', headless: 'yes' }),
    api.chromium.launch({ executablePath: '/test-only/chromium', timeout: -1 }),
  ];

  // Act
  const results = await Promise.allSettled(invalid);

  // Assert
  assert.ok(results.every(result => result.status === 'rejected'));
  assert.deepEqual(results.map(result => result.reason?.code ?? result.reason?.name), [
    'PWRIGHT_UNSUPPORTED',
    'PWRIGHT_UNSUPPORTED',
    'PWRIGHT_UNSUPPORTED',
    'TypeError',
    'TypeError',
    'TypeError',
  ]);
  assert.deepEqual(calls, []);
});

for (const { engine, args, expected } of [
  {
    engine: 'firefox',
    args: ['--user-data-dir=random-path', '--profile=random-path'],
    expected: "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--profile' argument",
  },
  {
    engine: 'firefox',
    args: ['-profile=random-path'],
    expected: "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--profile' argument",
  },
  {
    engine: 'webkit',
    args: ['--user-data-dir=random-path', '--profile=random-path'],
    expected: "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--user-data-dir' argument",
  },
  {
    engine: 'firefox',
    args: ['http://example.com'],
    expected: 'Arguments can not specify page to be opened',
  },
  {
    engine: 'webkit',
    args: ['http://example.com'],
    expected: 'Arguments can not specify page to be opened',
  },
]) {
  test(`${engine} rejects ${args[0]} before native launch`, async () => {
    // Arrange
    const { api, calls } = fixture();

    // Act
    const result = api[engine].launch({ args });

    // Assert
    await assert.rejects(result, error => error.constructor === Error && error.message === expected);
    assert.deepEqual(calls, []);
  });
}

test('launch rejects a nonexistent relative executable before native acquisition', async () => {
  // Arrange
  const { api, calls } = fixture('owned-launch', path => path !== 'random-invalid-path');

  // Act
  const result = api.chromium.launch({ executablePath: 'random-invalid-path' });

  // Assert
  await assert.rejects(result, error => error.constructor === Error &&
    error.message === "browserType.launch: Failed to launch chromium because executable doesn't exist at random-invalid-path");
  assert.deepEqual(calls, []);
});

test('launch rejects a nonexistent absolute executable before native acquisition', async () => {
  // Arrange
  const { api, calls } = fixture('owned-launch', path => path !== '/test-only/missing-chromium');

  // Act
  const result = api.chromium.launch({ executablePath: '/test-only/missing-chromium' });

  // Assert
  await assert.rejects(result, error => error.constructor === Error &&
    error.message === "browserType.launch: Failed to launch chromium because executable doesn't exist at /test-only/missing-chromium");
  assert.deepEqual(calls, []);
});

test('default profile keeps launch unsupported before loading the addon', async () => {
  // Arrange
  const { api, calls } = fixture('default');

  // Act
  const result = api.chromium.launch({ executablePath: '/test-only/chromium' });

  // Assert
  await assert.rejects(result, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, []);
  assert.equal(api.compatibility.browserOwnership, 'external');
});
