import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const createAdapter = createRequire(import.meta.url)('../src/wait-for-function.cjs');

function fixture(overrides = {}) {
  const calls = [];
  const native = {};
  const wrapper = {};
  const session = {
    async frameWaitForFunction(...args) {
      calls.push(args);
      return native;
    },
    ...overrides,
  };
  const frame = {
    _id: 'frame-1',
    _page: {
      _id: 'page-1',
      _context: {
        _browser: { _session: session },
        _resolveHandleFrame: id => `resolved:${id}`,
      },
    },
  };
  const adapter = createAdapter({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    validateOptions(options, supported, api) {
      if (!options || typeof options !== 'object' || Array.isArray(options))
        throw new TypeError(`${api}: options must be an object`);
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new Error(`${api}: unsupported ${key}`);
      }
    },
    UnsupportedOperationError: class extends Error {},
    async createHandle(actualSession, actualNative, resolveFrame) {
      assert.equal(actualSession, session);
      assert.equal(actualNative, native);
      assert.equal(resolveFrame('frame-2'), 'resolved:frame-2');
      return wrapper;
    },
  });
  return { adapter, calls, frame, wrapper };
}

test('waitForFunction delegates default rAF polling and returns the retained wrapper', async () => {
  // Arrange
  const { adapter, calls, frame, wrapper } = fixture();
  const predicate = value => value.ready;
  const argument = { ready: true };

  // Act
  const result = await adapter.wait(frame, predicate, argument, {}, 'page.waitForFunction');

  // Assert
  assert.equal(result, wrapper);
  assert.deepEqual(calls, [[
    'frame-1',
    'page-1',
    String(predicate),
    argument,
    { isFunction: true, pollingMs: 0, timeoutMs: undefined },
  ]]);
});

test('waitForFunction forwards bounded interval and timeout choices', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  await adapter.wait(
    frame,
    'window.ready',
    undefined,
    { polling: 25, timeout: 400 },
    'frame.waitForFunction',
  );

  // Assert
  assert.deepEqual(calls[0], [
    'frame-1',
    'page-1',
    'window.ready',
    undefined,
    { isFunction: false, pollingMs: 25, timeoutMs: 400 },
  ]);
});

test('waitForFunction rejects invalid polling before native admission', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const result = adapter.wait(
    frame,
    'true',
    undefined,
    { polling: 0 },
    'page.waitForFunction',
  );

  // Assert
  await assert.rejects(result, /Cannot poll with non-positive interval/);
  assert.equal(calls.length, 0);
});

test('waitForFunction reports the official unknown polling option', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const result = adapter.wait(
    frame,
    'true',
    undefined,
    { polling: 'mutation' },
    'page.waitForFunction',
  );

  // Assert
  await assert.rejects(result, /Unknown polling option: mutation/);
  assert.equal(calls.length, 0);
});
