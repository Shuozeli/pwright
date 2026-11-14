import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const contextCookies = require('../src/context-cookies.cjs')({
  mapError: (error, api) => new Error(`${api}: ${error.message}`),
  UnsupportedOperationError: class UnsupportedOperationError extends Error {},
});

test('BrowserContext.addCookies forwards its exact Context and cookie inputs to Rust', async () => {
  // Arrange
  const cookies = [{ url: 'https://example.test/', name: 'foo', value: 'bar' }];
  const calls = [];
  const session = { async contextAddCookies(...args) { calls.push(args); } };
  const context = { _id: 'context-7', browser: () => ({ _session: session }) };

  // Act
  await contextCookies.add(context, cookies);

  // Assert
  assert.deepEqual(calls, [['context-7', cookies]]);
  assert.equal(calls[0][1], cookies);
});

test('BrowserContext.addCookies remains unavailable when the selected native leaf is absent', async () => {
  // Arrange
  const context = { _id: 'context-7', browser: () => ({ _session: {} }) };

  // Act
  const pending = contextCookies.add(context, []);

  // Assert
  await assert.rejects(pending, /context-cookies native profile/);
});

test('BrowserContext.addCookies maps a Rust failure to its public API name', async () => {
  // Arrange
  const session = { async contextAddCookies() { throw new Error('invalid cookie URL'); } };
  const context = { _id: 'context-7', browser: () => ({ _session: session }) };

  // Act
  const pending = contextCookies.add(context, [{ name: 'foo', value: 'bar', url: ':' }]);

  // Assert
  await assert.rejects(pending, { message: 'browserContext.addCookies: invalid cookie URL' });
});
