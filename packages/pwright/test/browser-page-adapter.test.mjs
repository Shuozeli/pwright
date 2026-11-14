import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';
import { test } from 'node:test';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function descriptor(id) {
  const mainFrameId = `${id}-main`;
  return {
    id,
    contextId: 'implicit-context',
    mainFrameId,
    frameTree: {
      frames: () => [{ id: mainFrameId, parentId: null }],
      metadata: () => ({ url: () => 'about:blank', name: () => '' }),
    },
  };
}

function fixture(offline = false, navigation = false, timeoutSettings = true) {
  const calls = [];
  let connected = true;
  let lifecycleCallback;
  let visibleContextIds;
  let visiblePageIds;
  const session = {
    isConnected: () => connected,
    setLifecycleCallback(callback) { lifecycleCallback = callback; },
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    contextSnapshot(ids) { return { ids: visibleContextIds ?? ids, newContexts: [] }; },
    pageSnapshot(_contextId, ids) { return { ids: visiblePageIds ?? ids, newPages: [] }; },
    async newBrowserPage(baseURL, acceptDownloads) {
      calls.push(['newBrowserPage', baseURL, acceptDownloads]);
      return {
        context: { id: 'implicit-context', settings: { resolveUrl: url => url } },
        page: descriptor('implicit-page'),
      };
    },
    async closePage(id) { calls.push(['closePage', id]); },
    async closeContext(id) { calls.push(['closeContext', id]); },
    async close() { calls.push(['close']); connected = false; },
  };
  if (offline) {
    session.newContextWithOffline = async (...args) => {
      calls.push(['newContextWithOffline', ...args]);
      return { id: 'offline-context', settings: {} };
    };
    session.setContextOffline = async (...args) => {
      calls.push(['setContextOffline', ...args]);
    };
  }
  if (navigation) {
    if (timeoutSettings) {
      session.pageDefaultTimeout = (pageId, isNavigation) => {
        calls.push(['pageDefaultTimeout', pageId, isNavigation]);
        return 1_234;
      };
    }
    session.pageGoto = async (...args) => {
      calls.push(['pageGoto', ...args]);
      return { response: null };
    };
    session.pageReload = async (...args) => {
      calls.push(['pageReload', ...args]);
      return { response: null };
    };
    session.frameGoto = async (...args) => {
      calls.push(['frameGoto', ...args]);
      return { response: null };
    };
  }
  class NativeEnvironment {
    async connectOverCDP() { return session; }
  }
  const artifact = '/test-only/browser-page.node';
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
  return {
    api: module.exports,
    calls,
    session,
    lifecycle: (...args) => lifecycleCallback(...args),
    showContexts(ids) { visibleContextIds = ids; },
    showPages(ids) { visiblePageIds = ids; },
  };
}

function retainPendingRequest(page) {
  const { Request } = sourceRequire('../src/page-routes.cjs')({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: Error,
  });
  const owner = { id: 'retained-native-history-owner' };
  const request = new Request(page, {
    id: 'pending-request',
    url: 'https://example.test/pending',
    method: 'POST',
    resourceType: 'fetch',
    headers: [{ name: 'X-Request', value: 'kept' }],
    postData: null,
    isNavigationRequest: false,
    frameId: null,
  }, owner);
  const response = { status: () => 201 };
  request._setResponse(response);
  page._networkRequests.set('pending-request', request);
  page._networkRequestRefs = new Map([['pending-request', new WeakRef(request)]]);
  return { request, response, owner };
}

test('Browser newPage publishes the atomic Rust Context and Page result', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  const page = await browser.newPage({ baseURL: 'https://example.invalid/root/' });

  // Assert
  assert.equal(page.context().browser(), browser);
  assert.deepEqual(browser.contexts(), [page.context()]);
  assert.deepEqual(page.context().pages(), [page]);
  assert.deepEqual(value.calls, [[
    'newBrowserPage', 'https://example.invalid/root/', false,
  ]]);
});

test('Page unroute delegates one string registration to the selected Rust owner', async () => {
  // Arrange
  const value = fixture();
  value.session.pageRoute = async (...args) => {
    value.calls.push(['pageRoute', ...args]);
    return '17';
  };
  value.session.pageUnroute = async (...args) => { value.calls.push(['pageUnroute', ...args]); };
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const handler = () => {};
  await page.route('**/*.css', handler);

  // Act
  await page.unroute('**/*.css', handler);

  // Assert
  assert.deepEqual(value.calls, [
    ['newBrowserPage', undefined, false],
    ['pageRoute', 'implicit-page', '**/*.css', undefined],
    ['pageUnroute', 'implicit-page', ['17']],
  ]);
  assert.equal(page._routes.size, 0);
});

test('Browser newPage closes through the Rust-owned implicit Context route', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();

  // Act
  await page.close();

  // Assert
  assert.deepEqual(value.calls, [
    ['newBrowserPage', undefined, false],
    ['closePage', 'implicit-page'],
  ]);
});

test('Context close retires Page request caches while preserving an externally held Request', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const retained = retainPendingRequest(page);

  // Act
  await page.context().close();

  // Assert
  assert.deepEqual(value.calls, [
    ['newBrowserPage', undefined, false],
    ['closeContext', 'implicit-context'],
  ]);
  assert.equal(page._networkRequests.size, 0);
  assert.equal(page._networkRequestRefs.size, 0);
  assert.equal(retained.request.url(), 'https://example.test/pending');
  assert.equal(retained.request.method(), 'POST');
  assert.deepEqual(retained.request.headers(), { 'x-request': 'kept' });
  assert.equal(await retained.request.response(), retained.response);
  assert.equal(retained.request._historyOwner, retained.owner);
});

test('Browser close retires Page request caches while preserving an externally held Request', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const retained = retainPendingRequest(page);

  // Act
  await browser.close();

  // Assert
  assert.deepEqual(value.calls, [
    ['newBrowserPage', undefined, false],
    ['close'],
  ]);
  assert.equal(page._networkRequests.size, 0);
  assert.equal(page._networkRequestRefs.size, 0);
  assert.equal(retained.request.url(), 'https://example.test/pending');
  assert.equal(retained.request.method(), 'POST');
  assert.deepEqual(retained.request.headers(), { 'x-request': 'kept' });
  assert.equal(await retained.request.response(), retained.response);
  assert.equal(retained.request._historyOwner, retained.owner);
});

test('Native Context close notice retires Page request caches without mutating held Request', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const retained = retainPendingRequest(page);
  const publish = value.lifecycle(1, 'implicit-context');

  // Act
  publish();

  // Assert
  assert.equal(page._networkRequests.size, 0);
  assert.equal(page._networkRequestRefs.size, 0);
  assert.equal(retained.request.url(), 'https://example.test/pending');
  assert.equal(await retained.request.response(), retained.response);
});

test('Native Page close notice retires Page request caches without mutating held Request', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const retained = retainPendingRequest(page);
  const publish = value.lifecycle(0, 'implicit-context', 'implicit-page');

  // Act
  publish();

  // Assert
  assert.equal(page._networkRequests.size, 0);
  assert.equal(page._networkRequestRefs.size, 0);
  assert.equal(retained.request.url(), 'https://example.test/pending');
  assert.equal(await retained.request.response(), retained.response);
});

test('Context snapshot pruning retires caches on a user-held Context and Page', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const context = page.context();
  const retained = retainPendingRequest(page);
  value.showContexts([]);

  // Act
  const contexts = browser.contexts();

  // Assert
  assert.deepEqual(contexts, []);
  assert.equal(context._pages.get('implicit-page'), page);
  assert.equal(page._networkRequests.size, 0);
  assert.equal(page._networkRequestRefs.size, 0);
  assert.equal(retained.request.url(), 'https://example.test/pending');
  assert.equal(await retained.request.response(), retained.response);
});

test('Page snapshot pruning retires caches on a user-held Page', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const context = page.context();
  const retained = retainPendingRequest(page);
  value.showPages([]);

  // Act
  const pages = context.pages();

  // Assert
  assert.deepEqual(pages, []);
  assert.equal(page._networkRequests.size, 0);
  assert.equal(page._networkRequestRefs.size, 0);
  assert.equal(retained.request.url(), 'https://example.test/pending');
  assert.equal(await retained.request.response(), retained.response);
});

test('Browser newPage accepts explicit download rejection without selecting download events', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  await browser.newPage({ acceptDownloads: false });

  // Assert
  assert.deepEqual(value.calls, [['newBrowserPage', undefined, false]]);
});

test('Browser newPage rejects download acceptance before native admission', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  const operation = browser.newPage({ acceptDownloads: true });

  // Assert
  await assert.rejects(operation, /select download-events native profile/);
  assert.deepEqual(value.calls, []);
});

test('Browser newPage rejects uninstalled Context options before native admission', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  const operation = browser.newPage({ locale: 'fr-FR' });

  // Assert
  await assert.rejects(operation, /browser\.newPage option locale is not supported/);
  assert.deepEqual(value.calls, []);
});

test('Browser newContext matches null viewport conflict errors before native admission', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const cases = [
    [{ noViewport: true, deviceScaleFactor: 1 }, '"deviceScaleFactor" option is not supported with null "viewport"'],
    [{ noViewport: true, isMobile: true }, '"isMobile" option is not supported with null "viewport"'],
  ];

  // Act and Assert
  for (const [options, message] of cases)
    await assert.rejects(browser.newContext(options), error => error.message === `browser.newContext: ${message}`);
  assert.deepEqual(value.calls, []);
});

test('Context offline creation and runtime toggles delegate to opt-in Rust methods', async () => {
  // Arrange
  const value = fixture(true);
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');

  // Act
  const context = await browser.newContext({ offline: true });
  await context.setOffline(false);
  await context.setOffline(true);

  // Assert
  assert.deepEqual(value.calls, [
    ['newContextWithOffline', undefined, true],
    ['setContextOffline', 'offline-context', false],
    ['setContextOffline', 'offline-context', true],
  ]);
  await assert.rejects(context.setOffline('yes'), /offline must be a boolean/);
});

test('Context runtime offline toggle requires its opt-in native capability', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();

  // Act
  const pending = page.context().setOffline(true);

  // Assert
  await assert.rejects(pending, /select context-offline native profile/);
  assert.deepEqual(value.calls, [['newBrowserPage', undefined, false]]);
});

const navigationCases = [
  {
    api: 'Page.goto',
    omitted: page => page.goto('https://example.test/page'),
    explicit: page => page.goto('https://example.test/page', { timeout: 0 }),
    invalid: page => page.goto('https://example.test/page', { waitUntil: 'networkidle2' }),
    omittedCall: ['frameGoto', ['implicit-page', 'implicit-page-main'], 'https://example.test/page', 'load', 1_234, undefined],
    explicitCall: ['frameGoto', ['implicit-page', 'implicit-page-main'], 'https://example.test/page', 'load', 0, undefined],
  },
  {
    api: 'Page.reload',
    omitted: page => page.reload(),
    explicit: page => page.reload({ timeout: 42 }),
    invalid: page => page.reload({ waitUntil: 'networkidle2' }),
    omittedCall: ['pageReload', 'implicit-page', 'load', 1_234],
    explicitCall: ['pageReload', 'implicit-page', 'load', 42],
  },
  {
    api: 'Frame.goto',
    omitted: page => page.mainFrame().goto('https://example.test/frame'),
    explicit: page => page.mainFrame().goto('https://example.test/frame', { timeout: 77 }),
    invalid: page => page.mainFrame().goto('https://example.test/frame', { waitUntil: 'networkidle2' }),
    omittedCall: ['frameGoto', ['implicit-page', 'implicit-page-main'], 'https://example.test/frame', 'load', 1_234, undefined],
    explicitCall: ['frameGoto', ['implicit-page', 'implicit-page-main'], 'https://example.test/frame', 'load', 77, undefined],
  },
];

for (const navigation of navigationCases) {
  test(`${navigation.api} inherits Rust-owned page navigation timeout when omitted`, async () => {
    // Arrange
    const value = fixture(false, true);
    const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
    const page = await browser.newPage();
    value.calls.length = 0;

    // Act
    await navigation.omitted(page);

    // Assert
    assert.deepEqual(value.calls, [
      ['pageDefaultTimeout', 'implicit-page', true],
      navigation.omittedCall,
    ]);
  });

  test(`${navigation.api} preserves explicit timeout without reading defaults`, async () => {
    // Arrange
    const value = fixture(false, true);
    const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
    const page = await browser.newPage();
    value.calls.length = 0;

    // Act
    await navigation.explicit(page);

    // Assert
    assert.deepEqual(value.calls, [navigation.explicitCall]);
  });

  test(`${navigation.api} maps a closed-page navigation timeout getter error`, async () => {
    // Arrange
    const value = fixture(false, true);
    const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
    const page = await browser.newPage();
    value.calls.length = 0;
    value.session.pageDefaultTimeout = (pageId, isNavigation) => {
      value.calls.push(['pageDefaultTimeout', pageId, isNavigation]);
      throw new Error('PWRIGHT_TARGET_CLOSED: Target page, context or browser has been closed');
    };

    // Act
    const operation = navigation.omitted(page);

    // Assert
    await assert.rejects(operation, error => error.code === 'PWRIGHT_TARGET_CLOSED'
      && error.message === `${navigation.api.toLowerCase()}: Target page, context or browser has been closed`);
    assert.deepEqual(value.calls, [['pageDefaultTimeout', 'implicit-page', true]]);
  });

  test(`${navigation.api} rejects invalid waitUntil before a closed-page timeout getter`, async () => {
    // Arrange
    const value = fixture(false, true);
    const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
    const page = await browser.newPage();
    value.calls.length = 0;
    value.session.pageDefaultTimeout = () => {
      value.calls.push(['pageDefaultTimeout']);
      throw new Error('PWRIGHT_TARGET_CLOSED: Target page, context or browser has been closed');
    };

    // Act
    const operation = navigation.invalid(page);

    // Assert
    await assert.rejects(operation, error => error.message === `${navigation.api.toLowerCase()}: waitUntil: expected one of (load|domcontentloaded|networkidle|commit)`);
    assert.deepEqual(value.calls, []);
  });
}

test('Page.goForward delegates lifecycle and inherited timeout to Rust and preserves response Frame', async () => {
  // Arrange
  const value = fixture(false, true);
  value.session.pageGoForward = async (...args) => {
    value.calls.push(['pageGoForward', ...args]);
    return { response: {
      url: 'https://example.test/next',
      status: 200,
      statusText: 'OK',
      ok: true,
      headers: { 'content-type': 'text/html' },
    } };
  };
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  const response = await page.goForward({ waitUntil: 'domcontentloaded' });

  // Assert
  assert.deepEqual(value.calls, [
    ['pageDefaultTimeout', 'implicit-page', true],
    ['pageGoForward', 'implicit-page', 'domcontentloaded', 1_234],
  ]);
  assert.equal(response.url(), 'https://example.test/next');
  assert.equal(response.status(), 200);
  assert.equal(response.frame(), page.mainFrame());
});

test('Page.goForward returns null at the end of history and honors an explicit timeout', async () => {
  // Arrange
  const value = fixture(false, true);
  value.session.pageGoForward = async (...args) => {
    value.calls.push(['pageGoForward', ...args]);
    return null;
  };
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  const response = await page.goForward({ timeout: 0, waitUntil: 'commit' });

  // Assert
  assert.equal(response, null);
  assert.deepEqual(value.calls, [['pageGoForward', 'implicit-page', 'commit', 0]]);
});

test('Page.goForward remains opt-in and rejects invalid lifecycle before native work', async () => {
  // Arrange
  const value = fixture(false, true);
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  const unsupported = page.goForward();
  const invalid = page.goForward({ waitUntil: 'ready' });

  // Assert
  await assert.rejects(unsupported, { code: 'PWRIGHT_UNSUPPORTED' });
  await assert.rejects(invalid, /page\.goForward: waitUntil: expected one of/);
  assert.deepEqual(value.calls, []);
});

test('Page.goto retains the 30-second fallback without native timeout settings', async () => {
  // Arrange
  const value = fixture(false, true, false);
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  await page.goto('https://example.test/page');

  // Assert
  assert.deepEqual(value.calls, [
    ['frameGoto', ['implicit-page', 'implicit-page-main'], 'https://example.test/page', 'load', 30_000, undefined],
  ]);
});

test('Page.goto retains the Page executor when Frame navigation is not selected', async () => {
  // Arrange
  const value = fixture(false, true);
  delete value.session.frameGoto;
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  await page.goto('https://example.test/page');

  // Assert
  assert.deepEqual(value.calls, [
    ['pageDefaultTimeout', 'implicit-page', true],
    ['pageGoto', 'implicit-page', 'https://example.test/page', 'load', 1_234, undefined],
  ]);
});

test('public Response.headerValue reads its retained native history owner', async () => {
  // Arrange
  const value = fixture(false, true);
  const owner = Object.freeze({ hop: 1 });
  value.session.frameGoto = async () => ({ response: {
    url: 'https://example.test/page',
    status: 200,
    statusText: 'OK',
    ok: true,
    headers: {},
    historyOwner: owner,
  } });
  value.session.responseRawHeaders = async observedOwner => {
    value.calls.push(['responseRawHeaders', observedOwner]);
    return [{ name: 'Access-Control-Allow-Origin', value: 'https://example.test' }];
  };
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const response = await page.goto('https://example.test/page');
  value.calls.length = 0;

  // Act
  const header = await response.headerValue('access-control-allow-origin');

  // Assert
  assert.equal(header, 'https://example.test');
  assert.deepEqual(value.calls, [['responseRawHeaders', owner]]);
});

test('Page.goto normalizes upstream networkidle0 alias before native dispatch', async () => {
  // Arrange
  const value = fixture(false, true);
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  await page.goto('https://example.test/page', { waitUntil: 'networkidle0' });

  // Assert
  assert.deepEqual(value.calls, [
    ['pageDefaultTimeout', 'implicit-page', true],
    ['frameGoto', ['implicit-page', 'implicit-page-main'], 'https://example.test/page', 'networkidle', 1_234, undefined],
  ]);
});

test('Frame goto retains native feature gate before reading default timeout', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  value.calls.length = 0;

  // Act
  const navigation = page.mainFrame().goto('https://example.test/frame');

  // Assert
  await assert.rejects(navigation, /select frame-navigation native profile/);
  assert.deepEqual(value.calls, []);
});
