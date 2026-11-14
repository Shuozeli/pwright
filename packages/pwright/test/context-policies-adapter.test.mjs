import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

class UnsupportedOperationError extends Error {}

const policies = createRequire(import.meta.url)('../src/context-policies.cjs')({
  UnsupportedOperationError,
});
const artifacts = createRequire(import.meta.url)('../src/native-artifact.cjs');

test('CSP bypass validates the public boolean before native selection', () => {
  // Arrange
  const options = { bypassCSP: 'yes' };

  // Act
  const validate = () => policies.validate(options);

  // Assert
  assert.throws(validate, /bypassCSP must be a boolean/);
});

test('disabled CSP bypass preserves ordinary Context creation without a capability', () => {
  // Arrange
  const session = {};
  const options = { bypassCSP: false };

  // Act
  policies.validate(options);
  const pending = policies.create(session, options, undefined);

  // Assert
  assert.equal(pending, undefined);
});

test('enabled CSP bypass delegates the base URL once to the native Rust owner', async () => {
  // Arrange
  const calls = [];
  const expected = Promise.resolve('context');
  const session = {
    newContextWithBypassCsp(...args) {
      calls.push(args);
      return expected;
    },
  };

  // Act
  const pending = policies.create(session, { bypassCSP: true }, 'https://example.test/base/');

  // Assert
  assert.equal(pending, expected);
  assert.deepEqual(calls, [['https://example.test/base/', true]]);
  assert.equal(await pending, 'context');
});

test('enabled CSP bypass rejects missing and uncomposed capabilities explicitly', () => {
  // Arrange
  const session = { newContextWithBypassCsp() { throw new Error('must not run'); } };

  // Act
  const missing = () => policies.create({}, { bypassCSP: true }, undefined);
  const combined = () => policies.create(session, { bypassCSP: true, extraHTTPHeaders: {} }, undefined);

  // Assert
  assert.throws(missing, UnsupportedOperationError);
  assert.throws(combined, /bypassCSP with extraHTTPHeaders or proxy/);
});

test('offline validates the public boolean before native selection', () => {
  // Arrange
  const options = { offline: 'yes' };

  // Act
  const validate = () => policies.validate(options);

  // Assert
  assert.throws(validate, /offline must be a boolean/);
});

test('offline Context creation selects only its opt-in native capability', async () => {
  // Arrange
  const calls = [];
  const session = {
    newContextWithOffline(...args) {
      calls.push(args);
      return Promise.resolve('context');
    },
  };

  // Act
  const pending = policies.create(session, { offline: true }, 'https://example.test/base/');

  // Assert
  assert.equal(await pending, 'context');
  assert.deepEqual(calls, [['https://example.test/base/', true]]);
  assert.throws(() => policies.create({}, { offline: true }, undefined), /select context-offline native profile/);
  assert.throws(() => policies.create(session, { offline: true, bypassCSP: true }, undefined), /bypassCSP with offline/);
});

test('offline rejects uncomposed device selection before either native call', () => {
  // Arrange
  const options = { offline: true, hasTouch: true };

  // Act
  const validate = () => policies.validate(options);

  // Assert
  assert.throws(validate, /offline with hasTouch=true/);
});

test('offline native profile is independently installable', () => {
  // Arrange
  const defaultFeatures = artifacts.nativeFeatures('default');

  // Act
  const offlineFeatures = artifacts.nativeFeatures('context-offline');

  // Assert
  assert.deepEqual(offlineFeatures, ['context-offline']);
  assert.ok(!defaultFeatures.includes('context-offline'));
});
