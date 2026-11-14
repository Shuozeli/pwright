import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { test } from 'node:test';
import lifecycleEvents from '../src/lifecycle-events.cjs';
import FrameCache from '../src/frame-cache.cjs';

function fixture() {
  const page = Object.assign(new EventEmitter(), {
    _consoleMessages: [],
    _networkRequests: new Map(),
    _networkRequestRefs: new Map(),
    _retireNetworkRequestCache() {
      this._networkRequests.clear();
      this._networkRequestRefs.clear();
    },
  });
  const context = Object.assign(new EventEmitter(), { _pages: new Map([['2', page]]) });
  const browser = { _contexts: new Map([['1', context]]) };
  return { page, context, browser, prepare: lifecycleEvents(new WeakRef(browser)) };
}

function frameFixture() {
  const value = fixture();
  const root = { _id: '1' };
  const rows = [{ id: '1', parentId: null }];
  const bindings = [];
  value.page._frameCache = new FrameCache({
    frames: () => rows.slice(),
    metadata: id => { bindings.push(id); return { id }; },
  }, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));
  value.page.mainFrame = () => root;
  value.page.frames = () => value.page._frameCache.frames();
  value.prepare = lifecycleEvents(new WeakRef(value.browser), undefined, true);
  return { ...value, root, rows, bindings };
}

test('Page preparation preserves identity without emitting or changing caches', () => {
  // Arrange
  const { page, context, prepare } = fixture();
  const calls = [];
  page.on('close', function (value) { calls.push([this, value, context._pages.has('2')]); });
  // Act
  const emit = prepare(0, '1', '2');
  // Assert
  assert.equal(context._pages.get('2'), page);
  assert.deepEqual(calls, []);
  assert.equal(typeof emit, 'function');
});

test('Page invocation removes cached membership before EventEmitter dispatch', () => {
  // Arrange
  const { page, context, prepare } = fixture();
  const calls = [];
  page.on('close', function (value) { calls.push([this, value, context._pages.has('2')]); });
  const emit = prepare(0, '1', '2');
  // Act
  emit();
  // Assert
  assert.deepEqual(calls, [[page, page, false]]);
});

test('Context invocation preserves receiver and removes cached membership', () => {
  // Arrange
  const { context, browser, prepare } = fixture();
  const calls = [];
  context.on('close', function (value) { calls.push([this, value, browser._contexts.has('1')]); });
  const emit = prepare(1, '1', '');
  // Act
  emit();
  // Assert
  assert.deepEqual(calls, [[context, context, false]]);
});

test('Page lifecycle notices preserve Page identity and event order', () => {
  // Arrange
  const { page, prepare } = fixture();
  const calls = [];
  page.on('domcontentloaded', value => calls.push(['domcontentloaded', value]));
  page.on('load', value => calls.push(['load', value]));
  const domContentLoaded = prepare(4, '1', '2');
  const load = prepare(5, '1', '2');
  // Act
  domContentLoaded();
  load();
  // Assert
  assert.deepEqual(calls, [['domcontentloaded', page], ['load', page]]);
});

test('main-frame navigation notice publishes the canonical Frame', () => {
  // Arrange
  const { page, prepare } = fixture();
  const frame = { id: 'main' };
  page.mainFrame = () => frame;
  let received;
  page.on('framenavigated', value => { received = value; });
  const emit = prepare(7, '1', '2');
  // Act
  emit();
  // Assert
  assert.equal(received, frame);
});

test('Frame attachment emits after live projection and preserves canonical identity', () => {
  // Arrange
  const { page, prepare, root, rows, bindings } = frameFixture();
  const calls = [];
  page.on('frameattached', frame => calls.push([frame, page.frames()[1], frame.parent]));
  const emit = prepare(11, '1', '2', '2');
  rows.push({ id: '2', parentId: '1' });

  // Act
  emit();

  // Assert
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], calls[0][1]);
  assert.equal(calls[0][2], root);
  assert.deepEqual(bindings, ['2']);
});

test('same-document main-frame navigation emits the existing main Frame', () => {
  // Arrange
  const { page, prepare, root, bindings } = frameFixture();
  const calls = [];
  page.on('framenavigated', frame => calls.push(frame));
  const emit = prepare(12, '1', '2', '1');

  // Act
  emit();

  // Assert
  assert.deepEqual(calls, [root]);
  assert.deepEqual(bindings, []);
});

test('child navigation reuses the Frame emitted for its attachment', () => {
  // Arrange
  const { page, prepare, rows } = frameFixture();
  const attached = [];
  const navigated = [];
  page.on('frameattached', frame => attached.push(frame));
  page.on('framenavigated', frame => navigated.push(frame));
  rows.push({ id: '2', parentId: '1' });
  prepare(11, '1', '2', '2')();

  // Act
  prepare(12, '1', '2', '2')();

  // Assert
  assert.equal(attached.length, 1);
  assert.equal(navigated.length, 1);
  assert.equal(navigated[0], attached[0]);
});

test('newly attached Frame after retirement gets a distinct identity', () => {
  // Arrange
  const { page, prepare, rows } = frameFixture();
  const attached = [];
  page.on('frameattached', frame => attached.push(frame));
  rows.push({ id: '2', parentId: '1' });
  prepare(11, '1', '2', '2')();
  rows.splice(1, 1, { id: '3', parentId: '1' });

  // Act
  prepare(11, '1', '2', '3')();

  // Assert
  assert.equal(attached.length, 2);
  assert.notEqual(attached[1], attached[0]);
  assert.equal(page.frames()[1], attached[1]);
  assert.deepEqual([...page._frameCache._frames.keys()], ['1', '3']);
});

test('unobserved Frame notices do not materialize wrappers', () => {
  // Arrange
  const { page, prepare, rows, bindings } = frameFixture();
  rows.push({ id: '2', parentId: '1' });

  // Act
  const emit = prepare(11, '1', '2', '2');

  // Assert
  assert.equal(emit, undefined);
  assert.deepEqual(bindings, []);
  assert.deepEqual([...page._frameCache._frames.keys()], ['1']);
});

test('Frame notice tags remain unavailable without the opt-in profile', () => {
  // Arrange
  const { prepare } = fixture();

  // Act
  const useFrameNotice = () => prepare(11, '1', '2', '3');

  // Assert
  assert.throws(useFrameNotice, /Unknown native lifecycle notice/);
});

test('Frame detach notice publishes the canonical retired Frame after commit', () => {
  // Arrange
  const { page, prepare } = fixture();
  const frame = { id: 'child', isDetached: () => true };
  const calls = [];
  page._frameCache = {
    detached(id) { calls.push(['bind', id]); return frame; },
    finishDetached(id) { calls.push(['release', id]); },
  };
  page.on('framedetached', value => calls.push(['event', value, value.isDetached()]));
  const emit = prepare(10, '1', '2', '3');
  assert.deepEqual(calls, []);

  // Act
  emit();

  // Assert
  assert.deepEqual(calls, [['bind', '3'], ['event', frame, true], ['release', '3']]);
});

test('Frame detach notice releases its target when wrapper materialization throws', () => {
  // Arrange
  const { page, prepare } = fixture();
  const failure = { reason: 'wrapper materialization failed' };
  const calls = [];
  page._frameCache = {
    detached(id) { calls.push(['bind', id]); throw failure; },
    finishDetached(id) { calls.push(['release', id]); },
  };
  page.on('framedetached', () => calls.push(['event']));
  const emit = prepare(10, '1', '2', '3');

  // Act
  let thrown;
  try { emit(); } catch (error) { thrown = error; }

  // Assert
  assert.equal(thrown, failure);
  assert.deepEqual(calls, [['bind', '3'], ['release', '3']]);
});

test('Page console notice publishes one Playwright-shaped message to Page and Context', () => {
  // Arrange
  const { page, context, prepare } = fixture();
  const calls = [];
  page.on('console', message => calls.push(['page', message]));
  context.on('console', message => calls.push(['context', message]));
  const emit = prepare(6, '1', '2', undefined, 'log', 'press', '["press"]', 'https://example.test/app.js', 7, 9);
  // Act
  emit();
  // Assert
  assert.equal(calls.length, 2);
  assert.equal(calls[0][0], 'page');
  assert.equal(calls[1][0], 'context');
  assert.equal(calls[0][1], calls[1][1]);
  assert.equal(calls[0][1].page(), page);
  assert.equal(calls[0][1].type(), 'log');
  assert.equal(calls[0][1].text(), 'press');
  assert.equal(calls[0][1].args().length, 1);
  assert.deepEqual(calls[0][1].location(), { url: 'https://example.test/app.js', lineNumber: 7, columnNumber: 9 });
});

test('Page console history keeps the newest two hundred of three hundred one messages', () => {
  // Arrange
  const { page, prepare } = fixture();
  const messages = Array.from({ length: 301 }, (_, index) =>
    prepare(6, '1', '2', undefined, 'log', String(index), `[${index}]`, '', 0, 0));

  // Act
  for (const emit of messages) emit();

  // Assert
  assert.equal(page._consoleMessages.length, 200);
  assert.equal(page._consoleMessages[0].text(), '101');
  assert.equal(page._consoleMessages[199].text(), '300');
});

for (const sentinel of [new Error('listener'), { reason: 'listener' }, 7]) {
  test(`listener throw preserves original ${typeof sentinel} and stops same-emitter siblings`, () => {
    // Arrange
    const { page, prepare } = fixture();
    let following = false;
    page.on('close', () => { throw sentinel; });
    page.on('close', () => { following = true; });
    const emit = prepare(0, '1', '2');
    let thrown;
    // Act
    try { emit(); } catch (error) { thrown = error; }
    // Assert
    assert.equal(thrown, sentinel);
    assert.equal(following, false);
  });
}

test('listener mutation and once semantics remain EventEmitter-owned', () => {
  // Arrange
  const { page, prepare } = fixture();
  const calls = [];
  const removed = () => calls.push('removed');
  page.on('close', () => { calls.push('first'); page.removeListener('close', removed); page.on('close', () => calls.push('new')); });
  page.on('close', removed);
  page.once('close', () => calls.push('once'));
  const emit = prepare(0, '1', '2');
  // Act
  emit();
  // Assert
  assert.deepEqual(calls, ['first', 'removed', 'once']);
  assert.equal(page.listeners('close').includes(removed), false);
  assert.equal(page.listenerCount('close'), 2);
});

test('unacquired wrappers are not synthesized during terminal delivery', () => {
  // Arrange
  const { context, browser, prepare } = fixture();
  // Act
  const results = [prepare(0, '1', '99'), prepare(1, '99', '')];
  // Assert
  assert.deepEqual(results, [undefined, undefined]);
  assert.equal(context._pages.size, 1);
  assert.equal(browser._contexts.size, 1);
});

test('collected Browser needs no callback', () => {
  // Arrange: deterministic weak-reference capability, no GC timing assumption.
  const prepare = lifecycleEvents({ deref: () => undefined });
  // Act
  const result = prepare(0, '1', '2');
  // Assert
  assert.equal(result, undefined);
});

test('unknown notices fail conversion before dispatch', () => {
  // Arrange
  const { prepare } = fixture();
  let thrown;
  // Act
  try { prepare(8, '1', '2'); } catch (error) { thrown = error; }
  // Assert
  assert.ok(thrown instanceof TypeError);
});

test('an unsupported host-only notice cannot emit a different Page event', () => {
  // Arrange
  const { page, context, prepare } = fixture();
  const events = [];
  page.on('load', () => events.push('load'));
  context.on('page', () => events.push('page'));
  let thrown;

  // Act
  try { prepare(11, '1', '2'); } catch (error) { thrown = error; }

  // Assert
  assert.ok(thrown instanceof TypeError);
  assert.match(thrown.message, /Unknown native lifecycle notice/);
  assert.deepEqual(events, []);
});
