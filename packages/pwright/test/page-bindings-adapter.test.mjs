import assert from 'node:assert/strict';
import { test } from 'node:test';

import createPageBindings from '../src/page-bindings.cjs';

class UnsupportedOperationError extends Error {}

function adapter() {
  return createPageBindings({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    UnsupportedOperationError,
    createHandle: async (_session, native) => ({ native }),
  });
}

test('binding dispatch passes Playwright source and serializable arguments to callback', async () => {
  // Arrange
  const delivered = [];
  const session = {
    acknowledgePageBindingCall() {},
    async deliverPageBindingResult(...args) { delivered.push(args); },
  };
  const context = { _browser: { _session: session } };
  const page = {
    _context: context,
    _bindings: new Map(),
    context: () => context,
  };
  const frame = { id: 'frame' };
  let source;
  page._bindings.set('add', {
    needsHandle: false,
    callback(value, left, right) {
      source = value;
      return left + right;
    },
  });

  // Act
  await adapter().dispatch(page, frame, {
    callId: '9',
    name: 'add',
    argumentsJson: '[5,6]',
  });

  // Assert
  assert.deepEqual(source, { context, page, frame });
  assert.deepEqual(delivered, [['9', { result: 11 }]]);
});

test('binding dispatch returns callback errors to the browser instead of rejecting watch loop', async () => {
  // Arrange
  const delivered = [];
  const session = {
    acknowledgePageBindingCall() {},
    async deliverPageBindingResult(...args) { delivered.push(args); },
  };
  const context = { _browser: { _session: session } };
  const page = {
    _context: context,
    _bindings: new Map([['woof', { needsHandle: false, callback() { throw new Error('WOOF'); } }]]),
    context: () => context,
  };

  // Act
  await adapter().dispatch(page, {}, {
    callId: '10',
    name: 'woof',
    argumentsJson: '[]',
  });

  // Assert
  assert.equal(delivered.length, 1);
  assert.equal(delivered[0][1].error.message, 'WOOF');
});

test('binding dispatch preserves a thrown null as the rejection value', async () => {
  // Arrange
  const delivered = [];
  const session = {
    acknowledgePageBindingCall() {},
    async deliverPageBindingResult(...args) { delivered.push(args); },
  };
  const context = { _browser: { _session: session } };
  const page = {
    _context: context,
    _bindings: new Map([['woof', { needsHandle: false, callback() { throw null; } }]]),
    context: () => context,
  };

  // Act
  await adapter().dispatch(page, {}, { callId: '11', name: 'woof', argumentsJson: '[]' });

  // Assert
  assert.deepEqual(delivered[0][1], { error: null });
});

test('binding dispatch publishes a native Handle wrapper for handle mode', async () => {
  // Arrange
  const delivered = [];
  const native = { id: 'native-handle' };
  const session = {
    acknowledgePageBindingCall() {},
    async takePageBindingHandle() { return native; },
    async deliverPageBindingResult(...args) { delivered.push(args); },
  };
  const context = { _browser: { _session: session }, _resolveHandleFrame() {} };
  let received;
  const page = {
    _context: context,
    _bindings: new Map([['handle', {
      needsHandle: true,
      callback(_source, value) { received = value; return 17; },
    }]]),
    context: () => context,
  };

  // Act
  await adapter().dispatch(page, {}, { callId: '12', name: 'handle', argumentsJson: '[]', needsHandle: true });

  // Assert
  assert.deepEqual(received, { native });
  assert.deepEqual(delivered, [['12', { result: 17 }]]);
});

test('binding result delivery failure is nonfatal during navigation', async () => {
  // Arrange
  const session = {
    acknowledgePageBindingCall() {},
    async deliverPageBindingResult() { throw new Error('execution context was destroyed'); },
  };
  const context = { _browser: { _session: session } };
  const page = {
    _context: context,
    _bindings: new Map([['compute', { needsHandle: false, callback: () => 17 }]]),
    context: () => context,
  };

  // Act
  const result = await adapter().dispatch(page, {}, { callId: '13', name: 'compute', argumentsJson: '[]' });

  // Assert
  assert.equal(result, undefined);
});

test('binding watch acknowledges language publication after callback invocation without awaiting its result', async () => {
  // Arrange
  const order = [];
  let resolveResult;
  let reads = 0;
  const session = {
    nextPageBindingCall() {
      reads += 1;
      order.push(`next:${reads}`);
      if (reads === 1) {
        return {
          callId: '14', contextId: 'context', pageId: 'page', frameId: 'frame',
          name: 'observe', argumentsJson: '[]',
        };
      }
      return new Promise(() => {});
    },
    acknowledgePageBindingCall(id) { order.push(`ack:${id}`); },
    async deliverPageBindingResult(id) { order.push(`deliver:${id}`); },
  };
  const frame = { _id: 'frame' };
  const context = { _browser: { _session: session }, _pages: new Map() };
  const page = {
    _context: context,
    _bindings: new Map([['observe', {
      needsHandle: false,
      callback() {
        order.push('callback');
        return new Promise(resolve => { resolveResult = resolve; });
      },
    }]]),
    context: () => context,
    frames: () => [frame],
  };
  context._pages.set('page', page);
  const browser = {
    _contexts: new Map([['context', context]]),
    _didWatchFail(error) { throw error; },
  };

  // Act
  adapter().watch(session, { deref: () => browser });
  await new Promise(resolve => setImmediate(resolve));

  // Assert
  assert.deepEqual(order, ['next:1', 'callback', 'ack:14', 'next:2']);
  resolveResult(17);
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(order, ['next:1', 'callback', 'ack:14', 'next:2', 'deliver:14']);
});

test('failed native registration rolls back callback identity', async () => {
  // Arrange
  const session = {
    async pageExposeBinding() { throw new Error('native registration failed'); },
  };
  const page = {
    _id: '7',
    _context: { _browser: { _session: session } },
    _bindings: new Map(),
  };

  // Act
  const error = await adapter().expose(page, 'add', () => 1).catch(error => error);

  // Assert
  assert.match(error.message, /page\.exposeBinding: native registration failed/);
  assert.equal(page._bindings.has('add'), false);
});

test('duplicate registration keeps the calling Playwright API name', async () => {
  // Arrange
  const page = {
    _context: { _browser: { _session: { pageExposeBinding() {} } } },
    _bindings: new Map([['foo', { needsHandle: false, callback() {} }]]),
  };

  // Act
  const error = await adapter().expose(page, 'foo', () => {}, 'page.exposeFunction').catch(error => error);

  // Assert
  assert.equal(error.message, 'page.exposeFunction: Function "foo" has been already registered');
});
