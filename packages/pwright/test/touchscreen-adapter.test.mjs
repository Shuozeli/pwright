import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

class UnsupportedOperationError extends Error {}

const Touchscreen = createRequire(import.meta.url)('../src/touchscreen.cjs')({
  mapError(error, api) { error.api = api; return error; },
  UnsupportedOperationError,
});

test('touchscreen validates both coordinates before native work', async () => {
  // Arrange
  const calls = [];
  const touchscreen = new Touchscreen({ touchscreenTap(...args) { calls.push(args); } }, 'page-1');

  // Act
  const first = touchscreen.tap(Number.NaN, 2);
  const second = touchscreen.tap(1, Number.POSITIVE_INFINITY);

  // Assert
  await assert.rejects(first, /coordinates must be finite/);
  await assert.rejects(second, /coordinates must be finite/);
  assert.deepEqual(calls, []);
});

test('touchscreen delegates one finite pair to the Rust port', async () => {
  // Arrange
  const calls = [];
  const touchscreen = new Touchscreen({
    async touchscreenTap(...args) { calls.push(args); },
  }, 'page-7');

  // Act
  await touchscreen.tap(40.25, -6.5);

  // Assert
  assert.deepEqual(calls, [['page-7', 40.25, -6.5]]);
});

test('touchscreen reports an omitted native capability explicitly', async () => {
  // Arrange
  const touchscreen = new Touchscreen({}, 'page-2');

  // Act
  const pending = touchscreen.tap(1, 2);

  // Assert
  await assert.rejects(pending, UnsupportedOperationError);
});

test('touchscreen maps a native failure with the public API name', async () => {
  // Arrange
  const expected = new Error('native failure');
  const touchscreen = new Touchscreen({ async touchscreenTap() { throw expected; } }, 'page-3');

  // Act
  const pending = touchscreen.tap(1, 2);

  // Assert
  await assert.rejects(pending, error => error === expected && error.api === 'touchscreen.tap');
});
