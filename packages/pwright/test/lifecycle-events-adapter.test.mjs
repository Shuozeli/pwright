import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { test } from 'node:test';
import lifecycleEvents from '../src/lifecycle-events.cjs';

function fixture() {
  const page = new EventEmitter();
  const context = Object.assign(new EventEmitter(), { _pages: new Map([['2', page]]) });
  const browser = { _contexts: new Map([['1', context]]) };
  return { page, context, browser, prepare: lifecycleEvents(new WeakRef(browser)) };
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
  try { prepare(4, '1', '2'); } catch (error) { thrown = error; }
  // Assert
  assert.ok(thrown instanceof TypeError);
});
