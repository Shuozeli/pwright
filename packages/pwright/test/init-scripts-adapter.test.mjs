import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const adapter = createRequire(import.meta.url)('../src/init-scripts.cjs')({
  mapError: (error, api) => new Error(`${api}: ${error.message}`),
  UnsupportedOperationError: class extends Error {},
});

test('Page init-script adapter only normalizes source before native registration', async () => {
  // Arrange
  const calls = [];
  const session = { async pageAddInitScript(...args) { calls.push(args); } };
  const page = {
    _id: 'page-7',
    context() { return { browser() { return { _session: session }; } }; },
  };

  // Act
  await adapter.add(page, function boot() { globalThis.ready = true; }, false);
  await adapter.add(page, { content: '// comment' }, false);

  // Assert
  assert.equal(calls.length, 2);
  assert.equal(calls[0][0], 'page-7');
  assert.match(calls[0][1], /^\(function boot\(\)/);
  assert.equal(calls[1][1], '// comment\n');
});

test('Page init-script adapter rejects an option object without path or content', async () => {
  // Arrange
  const page = { context() { throw new Error('must not reach native'); } };

  // Act
  const error = await adapter.add(page, { foo: 'bar' }, false).catch(value => value);

  // Assert
  assert.match(error.message, /Either path or content property must be present/);
});

test('BrowserContext init-script adapter delegates the Context identity', async () => {
  // Arrange
  const calls = [];
  const session = { async contextAddInitScript(...args) { calls.push(args); } };
  const context = {
    _id: 'context-3',
    browser() { return { _session: session }; },
  };

  // Act
  await adapter.addContext(context, 'globalThis.ready = true', false);

  // Assert
  assert.deepEqual(calls, [['context-3', 'globalThis.ready = true\n']]);
});
