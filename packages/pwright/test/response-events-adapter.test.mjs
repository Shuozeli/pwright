import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
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
const responseEvents = require('../src/response-events.cjs')({
  createResponse: (_page, metadata) => ({
    url: () => metadata.url,
    status: () => metadata.status,
    text: async () => metadata.body,
    nativeRequestId: metadata.nativeRequestId,
    finishedOwner: metadata.finishedOwner,
    historyOwner: metadata.historyOwner,
  }),
  mapError: error => error,
  validateOptions,
  UnsupportedOperationError,
  waitForEvent,
});

function fixture() {
  const session = {
    nextResponseNotice() {},
    pageDefaultTimeout() { return 250; },
    pageUrlGlob(_pageId, glob) { return glob === '**/accepted' ? '.*/accepted$' : '.*'; },
  };
  const browser = { _session: session, _contexts: new Map(), isConnected: () => true };
  const context = Object.assign(new EventEmitter(), { _browser: browser, _pages: new Map() });
  const page = Object.assign(new EventEmitter(), { _context: context, _id: '2' });
  context._pages.set('2', page);
  browser._contexts.set('1', context);
  return { browser, context, page, session };
}

function notice(url, body = '', status = 200) {
  return {
    pageId: '2',
    nativeRequestId: 'native-request-1',
    finishedOwner: Object.freeze({ id: 'finished-1' }),
    historyOwner: Object.freeze({ id: 'history-1' }),
    response: {
      requestId: 'request-1',
      request: null,
      url,
      status,
      statusText: status === 200 ? 'OK' : 'Not Found',
      ok: status >= 200 && status <= 299,
      headers: {},
      body,
    },
  };
}

test('waitForResponse arms synchronously and publishes one Response to Context then Page', async () => {
  // Arrange
  const { browser, context, page } = fixture();
  const published = [];
  context.on('response', response => published.push(['context', response]));
  page.on('response', response => published.push(['page', response]));

  // Act
  const pending = responseEvents.wait(page, '**/accepted');
  const armedBeforeAction = page.listenerCount('response');
  responseEvents.dispatch(browser, notice('https://example.test/accepted'));
  const response = await pending;

  // Assert
  assert.equal(armedBeforeAction, 2);
  assert.deepEqual(published.map(([owner]) => owner), ['context', 'page']);
  assert.equal(published[0][1], published[1][1]);
  assert.equal(response, published[1][1]);
  assert.equal(response.nativeRequestId, 'native-request-1');
  assert.equal(response.finishedOwner.id, 'finished-1');
  assert.equal(response.historyOwner.id, 'history-1');
  assert.equal(page.listenerCount('response'), 1);
});

test('waitForResponse skips nonmatches and awaits an asynchronous body predicate', async () => {
  // Arrange
  const { browser, page } = fixture();
  const seen = [];
  const pending = responseEvents.wait(page, async response => {
    seen.push(response.url());
    return (await response.text()).includes('accepted');
  });

  // Act
  responseEvents.dispatch(browser, notice('https://example.test/skipped', 'no'));
  await Promise.resolve();
  await Promise.resolve();
  responseEvents.dispatch(browser, notice('https://example.test/accepted', 'accepted body'));
  const response = await pending;

  // Assert
  assert.deepEqual(seen, [
    'https://example.test/skipped',
    'https://example.test/accepted',
  ]);
  assert.equal(response.url(), 'https://example.test/accepted');
});

test('waitForResponse accepts a RegExp created in another VM context', async () => {
  // Arrange
  const { browser, page } = fixture();
  const regexp = vm.runInContext('new RegExp(/accepted$/)', vm.createContext());
  const pending = responseEvents.wait(page, regexp);

  // Act
  responseEvents.dispatch(browser, notice('https://example.test/accepted'));
  const response = await pending;

  // Assert
  assert.equal(response.url(), 'https://example.test/accepted');
});

test('waitForResponse preserves predicate errors and retires its listener', async () => {
  // Arrange
  const { browser, page } = fixture();
  const expected = new TypeError('predicate failed');
  const pending = responseEvents.wait(page, () => { throw expected; });

  // Act
  responseEvents.dispatch(browser, notice('https://example.test/response'));

  // Assert
  await assert.rejects(pending, error => error === expected);
  assert.equal(page.listenerCount('response'), 0);
});

test('waitForResponse uses the Page default and Playwright matcher log', async () => {
  // Arrange
  const { page, session } = fixture();
  session.pageDefaultTimeout = () => 5;

  // Act
  const pending = responseEvents.wait(page, 'never.css');

  // Assert
  await assert.rejects(pending, error => error instanceof TimeoutError
    && error.message.includes('page.waitForResponse: Timeout 5ms exceeded while waiting for event "response"')
    && error.message.includes('waiting for response "never.css"'));
  assert.equal(page.listenerCount('response'), 0);
});

test('waitForResponse timeout restores the captured public call site', async () => {
  // Arrange
  const { page } = fixture();
  const apiStack = { stack: 'Error\n    at official-response.spec.ts:42:9' };

  // Act
  const pending = responseEvents.wait(page, () => false, { timeout: 1 }, apiStack);

  // Assert
  await assert.rejects(pending, error => error instanceof TimeoutError
    && error.stack.split('\n')[1] === '    at official-response.spec.ts:42:9');
});

test('waitForResponse with no timeout stays armed until a response matches', async () => {
  // Arrange
  const { browser, page } = fixture();
  const pending = responseEvents.wait(page, '**/accepted', { timeout: 0 });

  // Act
  await new Promise(resolve => setTimeout(resolve, 5));
  responseEvents.dispatch(browser, notice('https://example.test/accepted'));
  const response = await pending;

  // Assert
  assert.equal(response.url(), 'https://example.test/accepted');
  assert.equal(page.listenerCount('response'), 0);
});

test('response watcher consumes native notices sequentially without retaining Browser', async () => {
  // Arrange
  const { browser, page, session } = fixture();
  const queue = [notice('https://example.test/accepted')];
  let resolveFinished;
  const finished = new Promise(resolve => { resolveFinished = resolve; });
  session.nextResponseNotice = async () => {
    if (queue.length) return queue.shift();
    resolveFinished();
    return null;
  };
  const pending = responseEvents.wait(page, '**/accepted');

  // Act
  responseEvents.watch(session, new WeakRef(browser));
  const response = await pending;
  await finished;

  // Assert
  assert.equal(response.url(), 'https://example.test/accepted');
});
