import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
class UnsupportedOperationError extends Error {}
const responseHeaders = require('../src/response-headers.cjs')({
  mapError(error, api) { return new Error(`${api}: ${error.message}`); },
  UnsupportedOperationError,
});

test('response.allHeaders folds ordered Rust pairs without discarding duplicate values', async () => {
  // Arrange
  const owner = Object.freeze({ hop: 1 });
  const pairs = [
    { name: 'Content-Type', value: 'text/plain' },
    { name: 'X-Trace', value: 'first' },
    { name: 'x-trace', value: 'second' },
    { name: 'Set-Cookie', value: 'a=1' },
    { name: 'set-cookie', value: 'b=2' },
  ];
  const observed = [];
  const session = { async responseRawHeaders(value) { observed.push(value); return pairs; } };

  // Act
  const headers = await responseHeaders.all(session, owner);
  headers['x-trace'] = 'mutated';
  const second = await responseHeaders.all(session, owner);

  // Assert
  assert.deepEqual(observed, [owner, owner]);
  assert.deepEqual(second, {
    'content-type': 'text/plain',
    'x-trace': 'first, second',
    'set-cookie': 'a=1\nb=2',
  });
  assert.equal(pairs[1].value, 'first');
});

test('response.allHeaders fails closed without a native history owner or binding', async () => {
  // Arrange
  const owner = Object.freeze({ hop: 1 });

  // Act and assert
  await assert.rejects(responseHeaders.all({ responseRawHeaders() {} }, null), UnsupportedOperationError);
  await assert.rejects(responseHeaders.all({}, owner), UnsupportedOperationError);
});

test('response.allHeaders keeps native failures visible at its public API', async () => {
  // Arrange
  const session = { async responseRawHeaders() { throw new Error('page closed'); } };

  // Act and assert
  await assert.rejects(responseHeaders.all(session, {}), /response\.allHeaders: page closed/);
});

test('response.headerValue matches names case-insensitively and folds duplicate values', async () => {
  // Arrange
  const owner = Object.freeze({ hop: 1 });
  const observed = [];
  const session = {
    async responseRawHeaders(value) {
      observed.push(value);
      return [
        { name: 'X-Trace', value: 'first' },
        { name: 'x-trace', value: 'second' },
      ];
    },
  };

  // Act
  const value = await responseHeaders.value(session, owner, 'X-TRACE');

  // Assert
  assert.equal(value, 'first, second');
  assert.deepEqual(observed, [owner]);
});

test('response.headerValue joins separate set-cookie values with newlines', async () => {
  // Arrange
  const session = {
    async responseRawHeaders() {
      return [
        { name: 'Set-Cookie', value: 'a=1' },
        { name: 'set-cookie', value: 'b=2' },
      ];
    },
  };

  // Act
  const value = await responseHeaders.value(session, {}, 'SET-COOKIE');

  // Assert
  assert.equal(value, 'a=1\nb=2');
});

test('response.headerValue returns null for an absent header, including object prototype names', async () => {
  // Arrange
  const session = { async responseRawHeaders() { return []; } };

  // Act
  const value = await responseHeaders.value(session, {}, 'toString');

  // Assert
  assert.equal(value, null);
});

test('response.headerValue fails closed without a native history owner or binding', async () => {
  // Arrange
  const owner = Object.freeze({ hop: 1 });

  // Act and assert
  await assert.rejects(responseHeaders.value({ responseRawHeaders() {} }, null, 'x'), UnsupportedOperationError);
  await assert.rejects(responseHeaders.value({}, owner, 'x'), UnsupportedOperationError);
});

test('response.headerValue labels native failures with its public API', async () => {
  // Arrange
  const session = { async responseRawHeaders() { throw new Error('page closed'); } };

  // Act and assert
  await assert.rejects(responseHeaders.value(session, {}, 'x'), /response\.headerValue: page closed/);
});

test('response.headerValue rejects a non-string name before copying native headers', async () => {
  // Arrange
  let copied = false;
  const session = { async responseRawHeaders() { copied = true; return []; } };

  // Act
  const result = responseHeaders.value(session, {}, 1);

  // Assert
  await assert.rejects(result, /response\.headerValue: name must be a string/);
  assert.equal(copied, false);
});
