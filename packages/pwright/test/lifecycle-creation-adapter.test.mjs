import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { test } from 'node:test';
import lifecycleEvents from '../src/lifecycle-events.cjs';

function fixture() {
  const made = [];
  const browser = { _contexts: new Map() };
  const create = {
    context(browser, value) {
      made.push(['context', value]);
      return Object.assign(new EventEmitter(), { browser, _id: value.id, _pages: new Map() });
    },
    page(context, value) {
      made.push(['page', value]);
      return Object.assign(new EventEmitter(), { context, _id: value.id });
    },
  };
  return { browser, made, create, prepare: lifecycleEvents(new WeakRef(browser), create) };
}
const contextValue = { id: '1' }, pageValue = { id: '2' };

test('ready request admits only missing identities without allocating wrappers', () => {
  // Arrange
  const { browser, made, prepare } = fixture();
  // Act
  const request = prepare(2, '1', '2');
  // Assert
  assert.equal(request.contextNeeded, true);
  assert.equal(request.pageNeeded, true);
  assert.deepEqual(made, []);
  assert.equal(browser._contexts.size, 0);
});

test('ready conversion creates wrappers without publishing cache membership', () => {
  // Arrange
  const { browser, made, prepare } = fixture();
  const request = prepare(2, '1', '2');
  // Act
  const emit = request.prepare(contextValue, pageValue);
  // Assert
  assert.deepEqual(made, [['context', contextValue], ['page', pageValue]]);
  assert.equal(typeof emit, 'function');
  assert.equal(browser._contexts.size, 0);
});

test('ready invocation installs identity before listener dispatch', () => {
  // Arrange
  const { browser, create, prepare } = fixture();
  const calls = [];
  const makeContext = create.context;
  create.context = (...args) => {
    const context = makeContext(...args);
    context.on('page', function (page) {
      calls.push([this, page, browser._contexts.get('1'), context._pages.get('2')]);
    });
    return context;
  };
  const emit = prepare(2, '1', '2').prepare(contextValue, pageValue);
  // Act
  emit();
  // Assert
  const context = browser._contexts.get('1'), page = context._pages.get('2');
  assert.deepEqual(calls, [[context, page, context, page]]);
  assert.equal(page.context, context);
});

test('existing Context requires only a Page descriptor', () => {
  // Arrange
  const { browser, made, create, prepare } = fixture();
  const context = create.context(browser, contextValue);
  browser._contexts.set('1', context);
  made.length = 0;
  // Act
  const request = prepare(2, '1', '2');
  const emit = request.prepare(undefined, pageValue);
  // Assert
  assert.equal(request.contextNeeded, false);
  assert.equal(request.pageNeeded, true);
  assert.deepEqual(made, [['page', pageValue]]);
  assert.equal(context._pages.size, 0);
  assert.equal(typeof emit, 'function');
});

test('already acquired Page requires no new descriptor and keeps exact identity', () => {
  // Arrange
  const { browser, made, prepare } = fixture();
  prepare(2, '1', '2').prepare(contextValue, pageValue)();
  const context = browser._contexts.get('1'), page = context._pages.get('2');
  const received = [];
  context.on('page', value => received.push(value));
  made.length = 0;
  // Act
  const request = prepare(2, '1', '2');
  request.prepare(undefined, undefined)();
  // Assert
  assert.equal(request.contextNeeded, false);
  assert.equal(request.pageNeeded, false);
  assert.deepEqual(made, []);
  assert.deepEqual(received, [page]);
});

test('failed Page conversion never installs a partially prepared Context', () => {
  // Arrange
  const { browser, create, prepare } = fixture();
  const sentinel = { conversion: 'failed' };
  create.page = () => { throw sentinel; };
  let thrown;
  // Act
  try { prepare(2, '1', '2').prepare(contextValue, pageValue); }
  catch (error) { thrown = error; }
  // Assert
  assert.equal(thrown, sentinel);
  assert.equal(browser._contexts.size, 0);
});

test('throwing Context.page listener preserves committed identity', () => {
  // Arrange
  const { browser, create, prepare } = fixture();
  const context = create.context(browser, contextValue), sentinel = { listener: 'failed' };
  browser._contexts.set('1', context);
  context.on('page', () => { throw sentinel; });
  const emit = prepare(2, '1', '2').prepare(undefined, pageValue);
  let thrown;
  // Act
  try { emit(); } catch (error) { thrown = error; }
  // Assert
  assert.equal(thrown, sentinel);
  assert.equal(context._pages.get('2')._id, '2');
  assert.equal(browser._contexts.get('1'), context);
});

test('popup uses already committed Page identities with the correct receiver', () => {
  // Arrange
  const { browser, prepare } = fixture();
  prepare(2, '1', '2').prepare(contextValue, pageValue)();
  prepare(2, '1', '3').prepare(undefined, { id: '3' })();
  const context = browser._contexts.get('1');
  const opener = context._pages.get('2'), popup = context._pages.get('3'), calls = [];
  opener.on('popup', function (value) { calls.push([this, value]); });
  const emit = prepare(3, '1', '3', '2');
  assert.deepEqual(calls, []);
  // Act
  emit();
  // Assert
  assert.deepEqual(calls, [[opener, popup]]);
});

test('unacquired popup emitter is not synthesized', () => {
  // Arrange
  const { browser, made, prepare } = fixture();
  prepare(2, '1', '2').prepare(contextValue, pageValue)();
  made.length = 0;
  // Act
  const result = prepare(3, '1', '2', '99');
  // Assert
  assert.equal(result, undefined);
  assert.deepEqual(made, []);
  assert.equal(browser._contexts.get('1')._pages.size, 1);
});

test('popup preceding its announced identity fails before listener dispatch', () => {
  // Arrange
  const { browser, prepare } = fixture();
  prepare(2, '1', '2').prepare(contextValue, pageValue)();
  const calls = [];
  browser._contexts.get('1')._pages.get('2').on('popup', value => calls.push(value));
  let thrown;
  // Act
  try { prepare(3, '1', '3', '2'); } catch (error) { thrown = error; }
  // Assert
  assert.ok(thrown instanceof TypeError);
  assert.deepEqual(calls, []);
});
