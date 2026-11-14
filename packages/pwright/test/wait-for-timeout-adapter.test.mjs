import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const createAdapter = createRequire(import.meta.url)('../src/wait-for-timeout.cjs');

function fixture(overrides = {}) {
  const calls = [];
  const session = {
    async waitForTimeout(value) { calls.push(value); },
    ...overrides,
  };
  const frame = { _page: { _context: { _browser: { _session: session } } } };
  const adapter = createAdapter({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    UnsupportedOperationError: class extends Error {},
  });
  return { adapter, calls, frame };
}

test('waitForTimeout delegates the exact duration to the Rust timer leaf', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  await adapter.wait(frame, 42, 'page.waitForTimeout');

  // Assert
  assert.deepEqual(calls, [42]);
});

test('waitForTimeout preserves zero without creating a Node timer', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  await adapter.wait(frame, 0, 'frame.waitForTimeout');

  // Assert
  assert.deepEqual(calls, [0]);
});

test('waitForTimeout rejects an invalid duration before native admission', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const result = adapter.wait(frame, -1, 'page.waitForTimeout');

  // Assert
  await assert.rejects(result, /timeout must be an integer/);
  assert.equal(calls.length, 0);
});
