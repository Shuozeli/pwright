import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

function adapter() {
  return createRequire(import.meta.url)('../src/request-gc.cjs')({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    UnsupportedOperationError: class extends Error {},
  });
}

test('Page GC delegates exactly one Page identity to Rust', async () => {
  // Arrange
  const calls = [];
  const session = { async pageRequestGc(pageId) { calls.push(pageId); } };

  // Act
  await adapter()(session, 'page-7');

  // Assert
  assert.deepEqual(calls, ['page-7']);
});

test('Page GC maps the native error with the public API name', async () => {
  // Arrange
  const session = { async pageRequestGc() { throw new Error('closed'); } };

  // Act
  const pending = adapter()(session, 'page-7');

  // Assert
  await assert.rejects(pending, { message: 'page.requestGC: closed' });
});
