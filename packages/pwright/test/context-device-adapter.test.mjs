import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

class UnsupportedOperationError extends Error {}

const device = createRequire(import.meta.url)('../src/context-device.cjs')({
  UnsupportedOperationError,
});

test('hasTouch validates the public boolean before native selection', () => {
  // Arrange
  const options = { hasTouch: 'yes' };

  // Act
  const validate = () => device.validate(options);

  // Assert
  assert.throws(validate, /hasTouch must be a boolean/);
});

test('disabled touch emulation preserves ordinary Context creation without a capability', () => {
  // Arrange
  const session = {};
  const options = { hasTouch: false };

  // Act
  device.validate(options);
  const pending = device.create(session, options, undefined);

  // Assert
  assert.equal(pending, undefined);
});

test('enabled touch emulation delegates the base URL once to the Rust owner', async () => {
  // Arrange
  const calls = [];
  const expected = Promise.resolve('context');
  const session = {
    newContextWithHasTouch(...args) {
      calls.push(args);
      return expected;
    },
  };

  // Act
  device.validate({ hasTouch: true });
  const pending = device.create(session, { hasTouch: true }, 'https://example.test/base/');

  // Assert
  assert.equal(pending, expected);
  assert.deepEqual(calls, [['https://example.test/base/', true]]);
  assert.equal(await pending, 'context');
});

test('enabled touch emulation rejects missing and uncomposed capabilities explicitly', () => {
  // Arrange
  const session = { newContextWithHasTouch() { throw new Error('must not run'); } };

  // Act
  const missing = () => device.create({}, { hasTouch: true }, undefined);
  const combined = () => device.validate({ hasTouch: true, bypassCSP: true });

  // Assert
  assert.throws(missing, UnsupportedOperationError);
  assert.throws(combined, /hasTouch=true with bypassCSP, extraHTTPHeaders or proxy/);
});
