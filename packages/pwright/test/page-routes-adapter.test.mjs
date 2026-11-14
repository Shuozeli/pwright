import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
class UnsupportedOperationError extends Error {}
const pageRoutes = require('../src/page-routes.cjs')({
  mapError(error, api) { return new Error(`${api}: ${error.message}`); },
  validateOptions(options, supported, api) {
    if (!options || typeof options !== 'object' || Array.isArray(options))
      throw new TypeError(`${api}: options must be an object`);
    for (const key of Object.keys(options)) {
      if (!supported.includes(key)) throw new UnsupportedOperationError(`${api} option ${key}`);
    }
  },
  UnsupportedOperationError,
  createResponseFromNotice(_page, notice, request) { return { notice, request }; },
});

function fixture() {
  const calls = [];
  const session = {
    async pageRoute(...args) { calls.push(['route', ...args]); return '17'; },
    async pageRouteContinue(...args) { calls.push(['continue', ...args]); },
    async pageRouteContinueWithHeaders(...args) { calls.push(['continueWithHeaders', ...args]); },
    async pageRouteFallback(...args) { calls.push(['fallback', ...args]); },
    async pageRouteFallbackWithUrl(...args) { calls.push(['fallbackWithUrl', ...args]); },
    async pageRouteAbort(...args) { calls.push(['abort', ...args]); },
    async pageRouteFulfill(...args) { calls.push(['fulfill', ...args]); },
  };
  const browser = { _session: session, _contexts: new Map() };
  const context = { _browser: browser, _pages: new Map() };
  const page = {
    _context: context,
    _id: '2',
    _routes: new Map(),
    frames: () => [{ _id: '3' }],
  };
  context._pages.set('2', page);
  browser._contexts.set('1', context);
  return { calls, session, browser, context, page };
}

function routeCall(overrides = {}) {
  return {
    callId: '9',
    registrationId: '17',
    contextId: '1',
    pageId: '2',
    frameId: '3',
    url: 'https://example.test/app.css',
    method: 'GET',
    resourceType: 'Stylesheet',
    headers: [{ name: 'accept', value: 'text/css' }],
    postData: undefined,
    isNavigationRequest: false,
    expires: false,
    ...overrides,
  };
}

function enableUnroute({ calls, session }) {
  let nextRegistration = 17;
  session.pageRoute = async (...args) => {
    calls.push(['route', ...args]);
    return String(nextRegistration++);
  };
  session.pageUnroute = async (...args) => { calls.push(['unroute', ...args]); };
}

test('routed Request retains captured POST data when a later network notice omits it', async () => {
  // Arrange
  let requestEvents;
  const historyRoutes = require('../src/page-routes.cjs')({
    mapError: error => error,
    validateOptions(_options, _supported, _api) {},
    UnsupportedOperationError,
    canonicalizeRequest: (...args) => requestEvents.canonicalize(...args),
  });
  requestEvents = require('../src/request-events.cjs')({
    Request: historyRoutes.Request,
    mapError: error => error,
    validateOptions(_options, _supported, _api) {},
    UnsupportedOperationError,
    waitForEvent() {},
  });
  const { browser, page } = fixture();
  page._networkRequests = new Map();
  let emittedRequest;
  page.emit = (_kind, request) => { emittedRequest = request; };
  const body = Buffer.alloc(2 * 1024 * 1024, 0x30);
  const owner = Object.freeze({ id: 'post-owner' });
  let routedRequest;
  page._routes.set('17', async route => {
    routedRequest = route.request();
    await route.abort();
  });
  await historyRoutes.dispatch(browser, routeCall({
    requestId: 'post-1', historyOwner: owner, method: 'POST',
    resourceType: 'Fetch', postData: body,
  }));

  // Act
  requestEvents.dispatch(browser, {
    pageId: '2', kind: 'request', historyOwner: owner,
    request: {
      id: 'post-1', url: 'https://example.test/app.css', method: 'POST',
      resourceType: 'fetch', headers: [], postData: null,
      isNavigationRequest: false, frameId: '3', redirectedFrom: null,
    },
  });

  // Assert
  assert.equal(emittedRequest, routedRequest);
  assert.equal(routedRequest.postDataBuffer(), body);
  assert.equal(routedRequest.postData(), body.toString());
});

test('routed Request shares native hop identity with un-intercepted redirect successors', async () => {
  // Arrange
  let requestEvents;
  const historyRoutes = require('../src/page-routes.cjs')({
    mapError: error => error,
    validateOptions(_options, _supported, _api) {},
    UnsupportedOperationError,
    canonicalizeRequest: (...args) => requestEvents.canonicalize(...args),
  });
  requestEvents = require('../src/request-events.cjs')({
    Request: historyRoutes.Request,
    mapError: error => error,
    validateOptions(_options, _supported, _api) {},
    UnsupportedOperationError,
    waitForEvent() {},
  });
  const { browser, page } = fixture();
  page._networkRequests = new Map();
  const observed = [];
  page.emit = (event, request) => { if (event === 'request') observed.push(request); };
  const intercepted = [];
  page._routes.set('17', async route => {
    intercepted.push(route.request());
    await route.continue();
  });
  const owners = [1, 2, 3].map(id => Object.freeze({ id: `owner-${id}` }));
  const urls = ['/one.css', '/two.css', '/three.css'].map(path => `https://example.test${path}`);
  const notice = (index, kind) => ({
    pageId: '2', kind, historyOwner: owners[index],
    request: {
      id: `css-${index + 1}`,
      url: urls[index],
      method: 'GET',
      resourceType: 'stylesheet',
      headers: [],
      postData: null,
      isNavigationRequest: false,
      frameId: '3',
      redirectedFrom: index === 0 ? null : `css-${index}`,
    },
  });

  // Act
  await historyRoutes.dispatch(browser, routeCall({
    url: urls[0], requestId: 'css-1', redirectedFrom: null,
    historyOwner: owners[0],
  }));
  requestEvents.dispatch(browser, notice(0, 'request'));
  requestEvents.dispatch(browser, notice(0, 'requestfinished'));
  requestEvents.dispatch(browser, notice(1, 'request'));
  requestEvents.dispatch(browser, notice(1, 'requestfinished'));
  requestEvents.dispatch(browser, notice(2, 'request'));

  // Assert
  assert.equal(intercepted.length, 1);
  assert.equal(intercepted[0], observed[0]);
  assert.equal(intercepted[0].redirectedTo(), observed[1]);
  assert.equal(observed[1].redirectedTo(), observed[2]);
  assert.equal(observed[2].redirectedTo(), null);
  assert.deepEqual([intercepted[0], observed[1], observed[2]].map(request => request.url()), urls);
  assert.deepEqual([intercepted[0], observed[1], observed[2]].map(request => request.resourceType()),
    ['stylesheet', 'stylesheet', 'stylesheet']);
});

test('Page route registration stores the handler only after Rust returns its identity', async () => {
  // Arrange
  const { calls, page } = fixture();
  const handler = () => {};

  // Act
  await pageRoutes.route(page, '**/*.css', handler, { times: 2 });

  // Assert
  assert.deepEqual(calls, [['route', '2', '**/*.css', 2]]);
  assert.equal(page._routes.get('17'), handler);
});

test('Page unroute with a handler removes only its exact registration identities', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  const first = () => {};
  const second = () => {};
  await pageRoutes.route(value.page, '**/*.css', first, { times: 2 });
  await pageRoutes.route(value.page, '**/*.css', second, { times: 3 });
  await pageRoutes.route(value.page, '**/*.js', first);

  // Act
  await pageRoutes.unroute(value.page, '**/*.css', first);

  // Assert
  assert.deepEqual(value.calls.at(-1), ['unroute', '2', ['17']]);
  assert.deepEqual([...value.page._routes.keys()], ['18', '19']);
  assert.deepEqual([...value.page._routeRegistrations.keys()], ['18', '19']);
  assert.equal(value.page._routeRegistrations.get('18').handler, second);
});

test('Page unroute without a handler removes all matching URL registrations in order', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  const handler = () => {};
  await pageRoutes.route(value.page, '**/*.css', handler);
  await pageRoutes.route(value.page, '**/*.js', handler);
  await pageRoutes.route(value.page, '**/*.css', handler);

  // Act
  await pageRoutes.unroute(value.page, '**/*.css');

  // Assert
  assert.deepEqual(value.calls.at(-1), ['unroute', '2', ['17', '19']]);
  assert.deepEqual([...value.page._routes.keys()], ['18']);
  assert.deepEqual([...value.page._routeRegistrations.keys()], ['18']);
});

test('Page unroute waits for an earlier registration before selecting its ID', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  let releaseRegistration;
  value.session.pageRoute = async (...args) => {
    value.calls.push(['route', ...args]);
    await new Promise(resolve => { releaseRegistration = resolve; });
    return '17';
  };
  const handler = () => {};
  const registration = pageRoutes.route(value.page, '**/*.css', handler);
  const removal = pageRoutes.unroute(value.page, '**/*.css', handler);
  await Promise.resolve();

  // Act
  releaseRegistration();
  await Promise.all([registration, removal]);

  // Assert
  assert.deepEqual(value.calls, [
    ['route', '2', '**/*.css', undefined],
    ['unroute', '2', ['17']],
  ]);
  assert.equal(value.page._routes.size, 0);
});

test('Page unroute keeps registration metadata when the Rust owner rejects removal', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  const handler = () => {};
  await pageRoutes.route(value.page, '**/*.css', handler);
  value.session.pageUnroute = async () => { throw new Error('native removal failed'); };

  // Act
  const removal = pageRoutes.unroute(value.page, '**/*.css', handler);

  // Assert
  await assert.rejects(removal, /native removal failed/);
  assert.equal(value.page._routes.get('17'), handler);
  assert.deepEqual([...value.page._routeRegistrations.keys()], ['17']);
});

test('Page unroute does not invoke Rust for a nonmatching URL or handler', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  const handler = () => {};
  await pageRoutes.route(value.page, '**/*.css', handler);
  const initialCalls = value.calls.length;

  // Act
  await pageRoutes.unroute(value.page, '**/*.css', () => {});

  // Assert
  assert.equal(value.calls.length, initialCalls);
  assert.equal(value.page._routes.get('17'), handler);
});

test('Page unroute rejects RegExp matchers without the opt-in native binding', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);

  // Act
  const nonString = pageRoutes.unroute(value.page, /css/);

  // Assert
  await assert.rejects(nonString, /route-regexp native profile/);
  assert.deepEqual(value.calls, []);
});

test('Page route delegates a RegExp source and supported flags to its opt-in native binding', async () => {
  // Arrange
  const value = fixture();
  const handler = () => {};
  value.session.pageRouteRegExp = async (...args) => {
    value.calls.push(['regexp', ...args]);
    return '23';
  };

  // Act
  await pageRoutes.route(value.page, /empty\.html/imsu, handler, { times: 2 });

  // Assert
  assert.deepEqual(value.calls, [['regexp', '2', 'empty\\.html', 'imsu', 2]]);
  assert.equal(value.page._routes.get('23'), handler);
  assert.equal(value.page._routeRegistrations, undefined);
});

test('Page route rejects a RegExp without the opt-in native binding before registration', async () => {
  // Arrange
  const value = fixture();

  // Act
  const registration = pageRoutes.route(value.page, /empty\.html/, () => {});

  // Assert
  await assert.rejects(registration, /route-regexp native profile/);
  assert.deepEqual(value.calls, []);
  assert.equal(value.page._routes.size, 0);
});

for (const flag of ['g', 'y', 'd']) {
  test(`Page route rejects unsupported RegExp flag ${flag} before native work`, async () => {
    // Arrange
    const value = fixture();
    value.session.pageRouteRegExp = async () => { throw new Error('native should not be called'); };

    // Act
    const registration = pageRoutes.route(value.page, new RegExp('empty', flag), () => {});

    // Assert
    await assert.rejects(registration, /route-regexp supports only i, m, s, and u/);
    assert.deepEqual(value.calls, []);
    assert.equal(value.page._routes.size, 0);
  });
}

let unicodeSetsFlagAvailable = true;
try { new RegExp('empty', 'v'); }
catch { unicodeSetsFlagAvailable = false; }
test('Page route rejects the unsupported RegExp v flag before native work', { skip: !unicodeSetsFlagAvailable }, async () => {
  // Arrange
  const value = fixture();
  value.session.pageRouteRegExp = async () => { throw new Error('native should not be called'); };

  // Act
  const registration = pageRoutes.route(value.page, new RegExp('empty', 'v'), () => {});

  // Assert
  await assert.rejects(registration, /route-regexp supports only i, m, s, and u/);
  assert.deepEqual(value.calls, []);
  assert.equal(value.page._routes.size, 0);
});

test('Page route bounds a RegExp source before native work', async () => {
  // Arrange
  const value = fixture();
  value.session.pageRouteRegExp = async () => { throw new Error('native should not be called'); };

  // Act
  const registration = pageRoutes.route(value.page, new RegExp('x'.repeat(4097)), () => {});

  // Assert
  await assert.rejects(registration, /RegExp source exceeds 4096 UTF-8 bytes/);
  assert.deepEqual(value.calls, []);
  assert.equal(value.page._routes.size, 0);
});

test('Page unroute compares RegExp source and flags, handler identity, and matcher kind', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  let nextRegistration = 20;
  value.session.pageRouteRegExp = async (...args) => {
    value.calls.push(['regexp', ...args]);
    return String(nextRegistration++);
  };
  const first = () => {};
  const second = () => {};
  await pageRoutes.route(value.page, 'empty', first);
  await pageRoutes.route(value.page, /empty/i, first);
  await pageRoutes.route(value.page, /empty/i, second);
  await pageRoutes.route(value.page, /empty/m, first);
  await pageRoutes.unroute(value.page, /empty/i, () => {});
  await pageRoutes.unroute(value.page, /empty/, first);

  // Act
  await pageRoutes.unroute(value.page, /empty/i, first);

  // Assert
  assert.deepEqual(value.calls.at(-1), ['unroute', '2', ['20']]);
  assert.deepEqual([...value.page._routes.keys()], ['17', '21', '22']);
  assert.deepEqual([...value.page._routeRegistrations.keys()], ['17', '21', '22']);
});

test('Page unroute waits for an earlier RegExp registration and removes only its returned ID', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  let releaseRegistration;
  value.session.pageRouteRegExp = async (...args) => {
    value.calls.push(['regexp', ...args]);
    return new Promise(resolve => { releaseRegistration = resolve; });
  };
  const handler = () => {};
  const registration = pageRoutes.route(value.page, /\.css$/, handler);
  const removal = pageRoutes.unroute(value.page, /\.css$/, handler);
  await Promise.resolve();

  // Act
  releaseRegistration('24');
  await Promise.all([registration, removal]);

  // Assert
  assert.deepEqual(value.calls, [
    ['regexp', '2', '\\.css$', '', undefined],
    ['unroute', '2', ['24']],
  ]);
  assert.equal(value.page._routes.size, 0);
  assert.equal(value.page._routeRegistrations.size, 0);
});

test('Page route leaves no RegExp registration when its native owner rejects', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  value.session.pageRouteRegExp = async () => { throw new Error('native regex limit'); };

  // Act
  const registration = pageRoutes.route(value.page, /\.css$/, () => {});

  // Assert
  await assert.rejects(registration, /native regex limit/);
  assert.equal(value.page._routes.size, 0);
  assert.equal(value.page._routeRegistrations, undefined);
  assert.equal(value.page._pendingRouteMutations, 0);
});

test('Page route dispatches the native-selected RegExp registration identity', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  let nextRegistration = 20;
  value.session.pageRouteRegExp = async (...args) => {
    value.calls.push(['regexp', ...args]);
    return String(nextRegistration++);
  };
  const observed = [];
  await pageRoutes.route(value.page, /\.css$/, async route => {
    observed.push('first');
    await route.continue();
  });
  await pageRoutes.route(value.page, /\.css$/, async route => {
    observed.push('second');
    await route.continue();
  });

  // Act
  await pageRoutes.dispatch(value.browser, routeCall({ registrationId: '21' }));

  // Assert
  assert.deepEqual(observed, ['second']);
  assert.deepEqual(value.calls.at(-1), ['continue', '9']);
  assert.deepEqual([...value.page._routeRegistrations.keys()], ['20', '21']);
});

test('Page unroute bounds string matcher bytes before native work', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);

  // Act
  const oversized = pageRoutes.unroute(value.page, 'x'.repeat(4097));

  // Assert
  await assert.rejects(oversized, /exceeds 4096 UTF-8 bytes/);
  assert.deepEqual(value.calls, []);
});

test('Page unroute requires a callable handler before native work', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);

  // Act
  const invalidHandler = pageRoutes.unroute(value.page, '**/*.css', 42);

  // Assert
  await assert.rejects(invalidHandler, /handler must be a function/);
  assert.deepEqual(value.calls, []);
});

test('Page unroute bounds pending registration mutations before native work', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  let releaseRegistration;
  value.session.pageRoute = async () => new Promise(resolve => { releaseRegistration = resolve; });
  const pending = [pageRoutes.route(value.page, '**/*.css', () => {})];
  for (let index = 0; index < 255; index++)
    pending.push(pageRoutes.unroute(value.page, '**/*.js'));
  await Promise.resolve();

  // Act
  const overflow = pageRoutes.unroute(value.page, '**/*.js');

  // Assert
  await assert.rejects(overflow, /pending route mutations exceed 256/);
  releaseRegistration('17');
  await Promise.all(pending);
  assert.equal(value.page._pendingRouteMutations, 0);
});

test('Page unroute requires its distinct opt-in native binding', async () => {
  // Arrange
  const { calls, page } = fixture();

  // Act
  const unsupported = pageRoutes.unroute(page, '**/*.css');

  // Assert
  await assert.rejects(unsupported, /page-unroute native profile/);
  assert.deepEqual(calls, []);
});

test('Route call keeps the request paused after a synchronous handler publishes the wrapper', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });

  // Act
  await pageRoutes.dispatch(browser, routeCall());

  // Assert
  assert.equal(published.request().url(), 'https://example.test/app.css');
  assert.equal(published.request().frame()._id, '3');
  assert.deepEqual(published.request().headers(), { accept: 'text/css' });
  assert.deepEqual(calls, []);
});

test('Route request allHeaders resolves a normalized native snapshot including cookies', async () => {
  // Arrange
  const { browser, page } = fixture();
  const nativeHeaders = [
    { name: 'Accept', value: 'text/css' },
    { name: 'Cookie', value: 'session=original' },
    { name: 'X-Trace', value: 'retained' },
  ];
  let headerPromise;
  page._routes.set('17', route => { headerPromise = route.request().allHeaders(); });

  // Act
  await pageRoutes.dispatch(browser, routeCall({ headers: nativeHeaders }));

  // Assert
  assert.ok(headerPromise instanceof Promise);
  assert.deepEqual(await headerPromise, {
    accept: 'text/css',
    cookie: 'session=original',
    'x-trace': 'retained',
  });
});

test('Route request allHeaders cannot mutate its retained headers or native snapshot', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  const nativeHeaders = [
    { name: 'Cookie', value: 'session=original' },
    { name: 'Accept', value: 'text/css' },
  ];
  let retainedRequest;
  let secondRead;
  page._routes.set('17', async route => {
    retainedRequest = route.request();
    const override = await retainedRequest.allHeaders();
    override.cookie = 'session=overridden';
    override.accept = 'application/json';
    secondRead = await retainedRequest.allHeaders();
    await route.continue({ headers: override });
  });

  // Act
  await pageRoutes.dispatch(browser, routeCall({ headers: nativeHeaders }));

  // Assert
  assert.deepEqual(secondRead, { cookie: 'session=original', accept: 'text/css' });
  assert.deepEqual(retainedRequest.headers(), secondRead);
  assert.deepEqual(nativeHeaders, [
    { name: 'Cookie', value: 'session=original' },
    { name: 'Accept', value: 'text/css' },
  ]);
  assert.deepEqual(calls, [['continueWithHeaders', '9', [
    { name: 'cookie', value: 'session=overridden' },
    { name: 'accept', value: 'application/json' },
  ]]]);
});

test('Route request allHeaders preserves handler errors and aborts an unhandled call', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  const failure = new Error('handler failed after reading headers');
  let observedHeaders;
  page._routes.set('17', async route => {
    observedHeaders = await route.request().allHeaders();
    throw failure;
  });

  // Act
  const dispatched = pageRoutes.dispatch(browser, routeCall());

  // Assert
  await assert.rejects(dispatched, error => error === failure);
  assert.deepEqual(observedHeaders, { accept: 'text/css' });
  assert.deepEqual(calls, [['abort', '9', 'failed']]);
});

test('Route request response uses the exact Rust history owner captured before callback', async () => {
  // Arrange
  const { browser, page, session } = fixture();
  const firstHopOwner = { hop: 'first', chargedByRust: true };
  const responseNotice = { response: { status: 404 } };
  const observedOwners = [];
  session.requestResponse = async owner => {
    observedOwners.push(owner);
    return responseNotice;
  };
  let retainedRequest;
  page._routes.set('17', route => { retainedRequest = route.request(); });

  // Act
  await pageRoutes.dispatch(browser, routeCall({ historyOwner: firstHopOwner }));
  const response = await retainedRequest.response();

  // Assert
  assert.deepEqual(observedOwners, [firstHopOwner]);
  assert.equal(response.notice, responseNotice);
  assert.equal(response.request, retainedRequest);
  assert.equal((await retainedRequest.response()), response);
  assert.deepEqual(observedOwners, [firstHopOwner]);
});

test('Route request response fails closed when Rust did not capture a history owner', async () => {
  // Arrange
  const { browser, page, session } = fixture();
  session.requestResponse = async () => { throw new Error('must not look up by URL or native ID'); };
  let retainedRequest;
  page._routes.set('17', route => { retainedRequest = route.request(); });

  // Act
  await pageRoutes.dispatch(browser, routeCall({ historyOwner: null }));

  // Assert
  await assert.rejects(retainedRequest.response(), /request\.response/);
});

test('Published Route continue delegates one settlement to its Rust call owner', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  await published.continue();

  // Assert
  assert.deepEqual(calls, [['continue', '9']]);
  await assert.rejects(published.continue(), /Route is already handled!/);
});

test('Published Route continue forwards bounded headers to the opt-in Rust owner', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  await published.continue({ headers: { accept: 'text/css', 'x-test': 'yes' } });

  // Assert
  assert.deepEqual(calls, [['continueWithHeaders', '9', [
    { name: 'accept', value: 'text/css' },
    { name: 'x-test', value: 'yes' },
  ]]]);
});

test('Published Route continue rejects malformed headers before claiming the Rust call', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  const invalidValue = published.continue({ headers: { accept: 42 } });

  // Assert
  await assert.rejects(invalidValue, /header accept must be a string/);
  assert.deepEqual(calls, []);
});

test('Published Route continue bounds header bytes before native conversion', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  const oversized = published.continue({ headers: { accept: 'x'.repeat(64 * 1024 + 1) } });

  // Assert
  await assert.rejects(oversized, /headers exceed byte limit/);
  assert.deepEqual(calls, []);
});

test('Published Route continue does not silently discard headers with an old native profile', async () => {
  // Arrange
  const { calls, browser, page, session } = fixture();
  delete session.pageRouteContinueWithHeaders;
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  const unsupported = published.continue({ headers: { 'x-test': 'yes' } });

  // Assert
  await assert.rejects(unsupported, /route-header-url-overrides native profile/);
  assert.deepEqual(calls, []);
});

test('Published Route fallback forwards its replacement URL to the opt-in Rust owner', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  await published.fallback({ url: 'https://example.test/bar' });

  // Assert
  assert.deepEqual(calls, [['fallbackWithUrl', '9', 'https://example.test/bar']]);
});

test('Published Route fallback rejects a non-string URL before native conversion', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  const invalid = published.fallback({ url: 42 });

  // Assert
  await assert.rejects(invalid, /url must be a string/);
  assert.deepEqual(calls, []);
});

test('Published Route fallback does not silently discard URL with an old native profile', async () => {
  // Arrange
  const { calls, browser, page, session } = fixture();
  delete session.pageRouteFallbackWithUrl;
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  const unsupported = published.fallback({ url: 'https://example.test/bar' });

  // Assert
  await assert.rejects(unsupported, /route-header-url-overrides native profile/);
  assert.deepEqual(calls, []);
});

test('Published Route fulfill normalizes public values before one Rust settlement', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  await published.fulfill({
    status: 201,
    headers: { 'Content-Type': 'ignored', 'x-test': 'yes' },
    contentType: 'text/plain',
    body: 'héllo',
  });

  // Assert
  assert.deepEqual(calls, [[
    'fulfill', '9', 201,
    [{ name: 'x-test', value: 'yes' }, { name: 'content-type', value: 'text/plain' }],
    Buffer.from('héllo'),
  ]]);
  await assert.rejects(published.fulfill(), /Route is already handled!/);
});

for (const method of ['fulfill', 'continue', 'fallback', 'abort']) {
  test(`Published Route ${method} rejects a second settlement without calling Rust again`, async () => {
    // Arrange
    const { calls, browser, page } = fixture();
    let published;
    page._routes.set('17', route => { published = route; });
    await pageRoutes.dispatch(browser, routeCall());
    await published[method]();
    const settledCalls = calls.length;

    // Act
    const repeat = published[method]();

    // Assert
    await assert.rejects(repeat, {
      message: `route.${method}: Route is already handled!`,
    });
    assert.equal(settledCalls, 1);
    assert.equal(calls.length, settledCalls);
    assert.equal(calls[0][0], method);
  });
}

test('Published Route fulfill JSON selects the default content type', async () => {
  // Arrange
  const { calls, browser, page } = fixture();
  let published;
  page._routes.set('17', route => { published = route; });
  await pageRoutes.dispatch(browser, routeCall());

  // Act
  await published.fulfill({ json: { answer: 42 } });

  // Assert
  assert.deepEqual(calls, [[
    'fulfill', '9', 200,
    [{ name: 'content-type', value: 'application/json' }],
    Buffer.from('{"answer":42}'),
  ]]);
});

test('Missing registration falls back through Rust instead of leaving Chromium paused', async () => {
  // Arrange
  const { calls, browser } = fixture();

  // Act
  await pageRoutes.dispatch(browser, routeCall({ registrationId: '404' }));

  // Assert
  assert.deepEqual(calls, [['fallback', '9']]);
});

test('Expiring route registration is retired before user dispatch', async () => {
  // Arrange
  const { browser, page } = fixture();
  let presentDuringHandler = true;
  page._routes.set('17', () => { presentDuringHandler = page._routes.has('17'); });

  // Act
  await pageRoutes.dispatch(browser, routeCall({ expires: true }));

  // Assert
  assert.equal(presentDuringHandler, false);
  assert.equal(page._routes.has('17'), false);
});

test('Expiring finite-times registration removes its unroute identity before dispatch', async () => {
  // Arrange
  const value = fixture();
  enableUnroute(value);
  const handler = () => {};
  await pageRoutes.route(value.page, '**/*.css', handler, { times: 1 });
  const registrationCalls = value.calls.length;

  // Act
  await pageRoutes.dispatch(value.browser, routeCall({ expires: true }));

  // Assert
  assert.equal(value.page._routes.has('17'), false);
  assert.equal(value.page._routeRegistrations.has('17'), false);
  assert.equal(value.calls.length, registrationCalls);
});
