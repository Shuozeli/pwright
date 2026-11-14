import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import vm from 'node:vm';

const require = createRequire(import.meta.url);
class TimeoutError extends Error {}
class UnsupportedOperationError extends Error {}
function validateOptions(options, supported, api) {
  if (!options || typeof options !== 'object' || Array.isArray(options))
    throw new TypeError(`${api}: options must be an object`);
  for (const key of Object.keys(options)) {
    if (!supported.includes(key)) throw new UnsupportedOperationError(`${api} option ${key}`);
  }
}
const waitForEvent = require('../src/wait-for-event.cjs')({ TimeoutError, validateOptions });
const pageRoutes = require('../src/page-routes.cjs')({
  mapError: error => error,
  validateOptions,
  UnsupportedOperationError,
});
const requestEvents = require('../src/request-events.cjs')({
  Request: pageRoutes.Request,
  mapError: error => error,
  validateOptions,
  UnsupportedOperationError,
  waitForEvent,
});

function fixture() {
  const session = {
    nextRequestNotice() {},
    pageDefaultTimeout() { return 250; },
    pageUrlGlob(_pageId, glob) { return glob === '**/accepted' ? '.*/accepted$' : '.*'; },
  };
  const browser = { _session: session, _contexts: new Map() };
  const context = { _browser: browser, _pages: new Map() };
  const page = Object.assign(new EventEmitter(), {
    _context: context,
    _id: '2',
    _networkRequests: new Map(),
    frames: () => [],
  });
  context._pages.set('2', page);
  browser._contexts.set('1', context);
  return { browser, page, session };
}

function notice(id, url, kind = 'request', overrides = {}) {
  return {
    pageId: '2',
    kind,
    request: {
      id,
      url,
      method: 'POST',
      resourceType: 'fetch',
      headers: [{ name: 'content-type', value: 'application/json' }],
      postData: Buffer.from('{"answer":42}'),
      postDataJsonAvailable: true,
      postDataJson: { answer: 42 },
      postDataJsonError: null,
      isNavigationRequest: false,
      frameId: null,
      ...overrides,
    },
    failureText: null,
  };
}

test('waitForRequest installs its listener synchronously before the user action', async () => {
  // Arrange
  const { browser, page } = fixture();

  // Act
  const pending = requestEvents.wait(page, '**/accepted');
  const armedBeforeAction = page.listenerCount('request');
  requestEvents.dispatch(browser, notice('7', 'https://example.test/accepted'));
  const request = await pending;

  // Assert
  assert.equal(armedBeforeAction, 1);
  assert.equal(request.url(), 'https://example.test/accepted');
  assert.equal(page.listenerCount('request'), 0);
});

test('waitForRequest skips nonmatches and awaits an asynchronous predicate', async () => {
  // Arrange
  const { browser, page } = fixture();
  const seen = [];
  const pending = requestEvents.wait(page, async request => {
    seen.push(request.url());
    await Promise.resolve();
    return request.url().endsWith('/accepted');
  });

  // Act
  requestEvents.dispatch(browser, notice('8', 'https://example.test/skipped'));
  await Promise.resolve();
  requestEvents.dispatch(browser, notice('9', 'https://example.test/accepted'));
  const request = await pending;

  // Assert
  assert.deepEqual(seen, [
    'https://example.test/skipped',
    'https://example.test/accepted',
  ]);
  assert.equal(request.url(), 'https://example.test/accepted');
});

test('waitForRequest preserves predicate errors and removes the listener', async () => {
  // Arrange
  const { browser, page } = fixture();
  const expected = new TypeError('predicate failed');
  const pending = requestEvents.wait(page, () => { throw expected; });

  // Act
  requestEvents.dispatch(browser, notice('10', 'https://example.test/request'));

  // Assert
  await assert.rejects(pending, error => error === expected);
  assert.equal(page.listenerCount('request'), 0);
});

test('waitForRequest timeout uses the Page default and retires its listener', async () => {
  // Arrange
  const { page, session } = fixture();
  session.pageDefaultTimeout = () => 5;

  // Act
  const pending = requestEvents.wait(page, '**/never');

  // Assert
  await assert.rejects(pending, error => error instanceof TimeoutError
    && error.message.includes('page.waitForRequest: Timeout 5ms exceeded while waiting for event "request"')
    && error.message.includes('waiting for request "**/never"'));
  assert.equal(page.listenerCount('request'), 0);
});

test('event timeout preserves Playwright diagnostics and the captured public call site', async () => {
  // Arrange
  const { page } = fixture();
  const apiStack = { stack: 'Error\n    at official-test.spec.ts:47:9' };

  // Act
  const pending = waitForEvent(
    page,
    'request',
    { predicate: () => false, timeout: 1 },
    'page.waitForEvent',
    250,
    { apiStack },
  );

  // Assert
  await assert.rejects(pending, error => error instanceof TimeoutError
    && error.message === 'page.waitForEvent: Timeout 1ms exceeded while waiting for event "request"'
    && error.stack.split('\n')[1] === '    at official-test.spec.ts:47:9');
  assert.equal(page.listenerCount('request'), 0);
});

test('waitForRequest accepts a RegExp created in another VM context', async () => {
  // Arrange
  const { browser, page } = fixture();
  const regexp = vm.runInContext('new RegExp(/accepted$/)', vm.createContext());
  const pending = requestEvents.wait(page, regexp);

  // Act
  requestEvents.dispatch(browser, notice('12', 'https://example.test/accepted'));
  const request = await pending;

  // Assert
  assert.equal(request.url(), 'https://example.test/accepted');
  assert.equal(page.listenerCount('request'), 0);
});

test('waitForRequest timeout log truncates long string matchers at fifty characters', async () => {
  // Arrange
  const { page } = fixture();
  const matcher = 'long-'.repeat(14) + '.css';

  // Act
  const pending = requestEvents.wait(page, matcher, { timeout: 1 });

  // Assert
  await assert.rejects(pending, error => error.message.includes(
    `waiting for request "${matcher.slice(0, 49)}…"`,
  ));
  assert.equal(page.listenerCount('request'), 0);
});

test('waitForRequest with no timeout remains armed until a request matches', async () => {
  // Arrange
  const { browser, page } = fixture();
  const pending = requestEvents.wait(page, '**/accepted', { timeout: 0 });

  // Act
  await new Promise(resolve => setTimeout(resolve, 5));
  requestEvents.dispatch(browser, notice('13', 'https://example.test/accepted'));
  const request = await pending;

  // Assert
  assert.equal(request.url(), 'https://example.test/accepted');
  assert.equal(page.listenerCount('request'), 0);
});

test('request lifecycle notices preserve wrapper identity and retire the cache', () => {
  // Arrange
  const { browser, page } = fixture();
  const published = [];
  page.on('request', request => published.push(request));
  page.on('requestfinished', request => published.push(request));

  // Act
  requestEvents.dispatch(browser, notice('11', 'https://example.test/request'));
  requestEvents.dispatch(browser, notice('11', 'https://example.test/request', 'requestfinished'));

  // Assert
  assert.equal(published.length, 2);
  assert.equal(published[0], published[1]);
  assert.equal(page._networkRequests.size, 0);
});

test('request lifecycle reads a native response and preserves failure snapshots', async () => {
  // Arrange
  const owner = Object.freeze({ id: 'opaque-request-owner' });
  const nativeResponse = { pageId: '2', response: { status: 200 } };
  const response = { status: () => nativeResponse.response.status };
  const seenOwners = [];
  const lifecycleRoutes = require('../src/page-routes.cjs')({
    mapError: error => error,
    validateOptions,
    UnsupportedOperationError,
    createResponseFromNotice(_page, noticeValue) {
      assert.equal(noticeValue, nativeResponse);
      return response;
    },
  });
  const lifecycleEvents = require('../src/request-events.cjs')({
    Request: lifecycleRoutes.Request,
    mapError: error => error,
    validateOptions,
    UnsupportedOperationError,
    waitForEvent,
  });
  const { browser, page, session } = fixture();
  session.requestResponse = async value => {
    seenOwners.push(value);
    return value === owner ? nativeResponse : null;
  };
  const published = [];
  page.on('request', request => published.push(request));
  page.on('requestfinished', request => published.push(request));
  page.on('requestfailed', request => published.push(request));

  // Act
  lifecycleEvents.dispatch(browser, { ...notice('31', 'https://example.test/success'), historyOwner: owner });
  const beforeFailure = published[0].failure();
  const awaited = await published[0].response();
  lifecycleEvents.dispatch(browser, {
    ...notice('31', 'https://example.test/success', 'requestfinished'),
    historyOwner: owner,
  });
  const failedOwner = Object.freeze({ id: 'failed-request-owner' });
  lifecycleEvents.dispatch(browser, {
    ...notice('32', 'https://example.test/failed'),
    historyOwner: failedOwner,
  });
  lifecycleEvents.dispatch(browser, {
    ...notice('32', 'https://example.test/failed', 'requestfailed'),
    historyOwner: failedOwner,
    failureText: 'net::ERR_EMPTY_RESPONSE',
  });
  const failedResponse = await published[2].response();

  // Assert
  assert.equal(beforeFailure, null);
  assert.equal(awaited, response);
  assert.equal(failedResponse, null);
  assert.deepEqual(seenOwners, [owner, failedOwner]);
  assert.equal(published[0], published[1]);
  assert.equal(published[2], published[3]);
  assert.equal(published[0].failure(), null);
  assert.deepEqual(published[2].failure(), { errorText: 'net::ERR_EMPTY_RESPONSE' });
  assert.equal(page._networkRequests.size, 0);
});

test('Request.failure prefers the current native history owner outcome', () => {
  // Arrange
  const { browser, page, session } = fixture();
  const owner = Object.freeze({ id: 'native-failure-owner' });
  let nativeFailure = null;
  session.requestFailure = value => {
    assert.equal(value, owner);
    return nativeFailure;
  };
  let request;
  page.once('request', value => { request = value; });
  requestEvents.dispatch(browser, { ...notice('35', 'https://example.test/failure'), historyOwner: owner });

  // Act
  const pendingFailure = request.failure();
  nativeFailure = 'net::ERR_CONNECTION_RESET';
  const settledFailure = request.failure();

  // Assert
  assert.equal(pendingFailure, null);
  assert.deepEqual(settledFailure, { errorText: 'net::ERR_CONNECTION_RESET' });
});

test('Request.frame avoids the frames capability when native frame identity is absent', () => {
  // Arrange
  const { browser, page } = fixture();
  page.frames = () => { throw new Error('frame navigation not selected'); };
  let request;
  page.once('request', value => { request = value; });

  // Act
  requestEvents.dispatch(browser, notice('36', 'https://example.test/no-frame'));
  const frame = request.frame();

  // Assert
  assert.equal(frame, null);
});

test('redirected Requests preserve both links after terminal cache retirement', () => {
  // Arrange
  const { browser, page, session } = fixture();
  const firstOwner = Object.freeze({ id: 'retained-first' });
  const finalOwner = Object.freeze({ id: 'retained-final' });
  let nativeLookups = 0;
  session.requestRedirectedFrom = () => { nativeLookups += 1; return null; };
  const published = [];
  page.on('request', request => published.push(request));

  // Act
  requestEvents.dispatch(browser, {
    ...notice('41', 'https://example.test/redirect'), historyOwner: firstOwner,
  });
  requestEvents.dispatch(browser, {
    ...notice('41', 'https://example.test/redirect', 'requestfinished'), historyOwner: firstOwner,
  });
  requestEvents.dispatch(browser, {
    ...notice('42', 'https://example.test/final', 'request', { redirectedFrom: '41' }),
    historyOwner: finalOwner,
  });
  requestEvents.dispatch(browser, {
    ...notice('42', 'https://example.test/final', 'requestfinished', { redirectedFrom: '41' }),
    historyOwner: finalOwner,
  });

  // Assert
  assert.equal(published[1].redirectedFrom(), published[0]);
  assert.equal(published[0].redirectedTo(), published[1]);
  assert.equal(published[0].redirectedFrom(), null);
  assert.equal(nativeLookups, 0);
  assert.equal(page._networkRequests.size, 0);
});

test('redirectedFrom reconstructs a predecessor after its weak cache entry is lost', () => {
  // Arrange
  const { browser, page, session } = fixture();
  const firstOwner = Object.freeze({ id: 'lost-first' });
  const finalOwner = Object.freeze({ id: 'lost-final' });
  const firstMetadata = notice('43', 'https://example.test/redirect').request;
  const queried = [];
  session.requestRedirectedFrom = owner => {
    queried.push(owner);
    return { request: firstMetadata, historyOwner: firstOwner };
  };
  let finalRequest;
  page.on('request', request => {
    if (request.url().endsWith('/final')) finalRequest = request;
  });

  // Act
  requestEvents.dispatch(browser, { ...notice('43', 'https://example.test/redirect'), historyOwner: firstOwner });
  requestEvents.dispatch(browser, {
    ...notice('43', 'https://example.test/redirect', 'requestfinished'), historyOwner: firstOwner,
  });
  page._networkRequestRefs.delete('43');
  requestEvents.dispatch(browser, {
    ...notice('44', 'https://example.test/final', 'request', { redirectedFrom: '43' }),
    historyOwner: finalOwner,
  });
  const predecessor = finalRequest.redirectedFrom();

  // Assert
  assert.equal(predecessor.url(), 'https://example.test/redirect');
  assert.equal(predecessor.redirectedTo(), finalRequest);
  assert.equal(finalRequest.redirectedFrom(), predecessor);
  assert.deepEqual(queried, [finalOwner]);
  assert.equal(page._networkRequests.get('43'), undefined);
});

test('redirectedFrom reconstructs a three-hop chain without retaining every completed wrapper', () => {
  // Arrange
  const { browser, page, session } = fixture();
  const owners = [1, 2, 3].map(id => Object.freeze({ id: `hop-${id}` }));
  const firstMetadata = notice('61', 'https://example.test/first').request;
  const secondMetadata = notice('62', 'https://example.test/second', 'request', {
    redirectedFrom: '61',
  }).request;
  const predecessors = new Map([
    [owners[2], { request: secondMetadata, historyOwner: owners[1] }],
    [owners[1], { request: firstMetadata, historyOwner: owners[0] }],
  ]);
  const queried = [];
  session.requestRedirectedFrom = owner => {
    queried.push(owner);
    return predecessors.get(owner) ?? null;
  };
  let last;
  page.on('request', request => {
    if (request.url().endsWith('/third')) last = request;
  });

  // Act
  requestEvents.dispatch(browser, { ...notice('61', 'https://example.test/first'), historyOwner: owners[0] });
  requestEvents.dispatch(browser, {
    ...notice('61', 'https://example.test/first', 'requestfinished'), historyOwner: owners[0],
  });
  page._networkRequestRefs.delete('61');
  requestEvents.dispatch(browser, {
    ...notice('62', 'https://example.test/second', 'request', { redirectedFrom: '61' }),
    historyOwner: owners[1],
  });
  requestEvents.dispatch(browser, {
    ...notice('62', 'https://example.test/second', 'requestfinished', { redirectedFrom: '61' }),
    historyOwner: owners[1],
  });
  page._networkRequestRefs.delete('62');
  requestEvents.dispatch(browser, {
    ...notice('63', 'https://example.test/third', 'request', { redirectedFrom: '62' }),
    historyOwner: owners[2],
  });
  const middle = last.redirectedFrom();
  const first = middle.redirectedFrom();

  // Assert
  assert.deepEqual([first.url(), middle.url(), last.url()], [
    'https://example.test/first',
    'https://example.test/second',
    'https://example.test/third',
  ]);
  assert.equal(first.redirectedTo(), middle);
  assert.equal(middle.redirectedTo(), last);
  assert.equal(last.redirectedFrom(), middle);
  assert.equal(middle.redirectedFrom(), first);
  assert.deepEqual(queried, [owners[2], owners[1]]);
  assert.equal(page._networkRequests.size, 1);
});

test('merged native dequeue publishes Request, Response and terminal events in one order', async () => {
  // Arrange
  const { browser, page, session } = fixture();
  const published = [];
  const response = { pageId: '2', response: { url: 'https://example.test/document' } };
  const queue = [
    { kind: 'request', requestNotice: notice('51', 'https://example.test/document') },
    { kind: 'response', responseNotice: response },
    { kind: 'request', requestNotice: notice('51', 'https://example.test/document', 'requestfinished') },
  ];
  session.nextNetworkNotice = async () => queue.shift() ?? null;
  page.on('request', () => published.push('request'));
  page.on('requestfinished', () => published.push('requestfinished'));
  const finished = new Promise(resolve => page.once('requestfinished', resolve));

  // Act
  const selected = requestEvents.watchCombined(session, new WeakRef(browser), (_browser, value) => {
    assert.equal(value, response);
    published.push('response');
  });
  await finished;

  // Assert
  assert.equal(selected, true);
  assert.deepEqual(published, ['request', 'response', 'requestfinished']);
});

test('waitForRequest rejects an unselected native capability explicitly', async () => {
  // Arrange
  const { page, session } = fixture();
  delete session.nextRequestNotice;

  // Act
  const pending = requestEvents.wait(page, '**');

  // Assert
  await assert.rejects(pending, UnsupportedOperationError);
  assert.equal(page.listenerCount('request'), 0);
});
