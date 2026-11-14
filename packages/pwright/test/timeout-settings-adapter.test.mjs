import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const createAdapter = createRequire(import.meta.url)('../src/timeout-settings.cjs');

function fixture(overrides = {}) {
  const calls = [];
  const session = {
    setPageDefaultTimeout(...args) { calls.push(['page', ...args]); },
    setContextDefaultTimeout(...args) { calls.push(['context', ...args]); },
    ...overrides,
  };
  const context = { _id: 'context-1', _browser: { _session: session } };
  const page = { _id: 'page-1', _context: context };
  const adapter = createAdapter({
    mapError: (error, api) => new Error(`${api}: ${error.message}`),
    UnsupportedOperationError: class extends Error {},
  });
  return { adapter, calls, context, page };
}

test('Page timeout setters publish generic and navigation settings to Rust', () => {
  // Arrange
  const { adapter, calls, page } = fixture();

  // Act
  adapter.setPage(page, 12, false, 'page.setDefaultTimeout');
  adapter.setPage(page, 34, true, 'page.setDefaultNavigationTimeout');

  // Assert
  assert.deepEqual(calls, [
    ['page', 'page-1', false, 12],
    ['page', 'page-1', true, 34],
  ]);
});

test('Context timeout setters publish generic and navigation settings to Rust', () => {
  // Arrange
  const { adapter, calls, context } = fixture();

  // Act
  adapter.setContext(context, 56, false, 'browserContext.setDefaultTimeout');
  adapter.setContext(context, 78, true, 'browserContext.setDefaultNavigationTimeout');

  // Assert
  assert.deepEqual(calls, [
    ['context', 'context-1', false, 56],
    ['context', 'context-1', true, 78],
  ]);
});

test('timeout settings reject invalid values before native state changes', () => {
  // Arrange
  const { adapter, calls, page } = fixture();

  // Act / Assert
  assert.throws(
    () => adapter.setPage(page, -1, false, 'page.setDefaultTimeout'),
    /timeout must be an integer/,
  );
  assert.equal(calls.length, 0);
});

test('Context navigation timeout accepts undefined as a reset', () => {
  // Arrange
  const { adapter, calls, context } = fixture();

  // Act
  adapter.setContext(context, undefined, true, 'browserContext.setDefaultNavigationTimeout');

  // Assert
  assert.deepEqual(calls, [['context', 'context-1', true, undefined]]);
});

test('Page navigation timeout accepts undefined as a reset', () => {
  // Arrange
  const { adapter, calls, page } = fixture();

  // Act
  adapter.setPage(page, undefined, true, 'page.setDefaultNavigationTimeout');

  // Assert
  assert.deepEqual(calls, [['page', 'page-1', true, undefined]]);
});

test('Context generic timeout accepts undefined as a reset', () => {
  // Arrange
  const { adapter, calls, context } = fixture();

  // Act
  adapter.setContext(context, undefined, false, 'browserContext.setDefaultTimeout');

  // Assert
  assert.deepEqual(calls, [['context', 'context-1', false, undefined]]);
});

test('zero remains an explicit timeout instead of a reset', () => {
  // Arrange
  const { adapter, calls, page } = fixture();

  // Act
  adapter.setPage(page, 0, true, 'page.setDefaultNavigationTimeout');

  // Assert
  assert.deepEqual(calls, [['page', 'page-1', true, 0]]);
});

test('null is rejected rather than treated as a reset', () => {
  // Arrange
  const { adapter, calls, context } = fixture();
  let error;

  // Act
  try {
    adapter.setContext(context, null, true, 'browserContext.setDefaultNavigationTimeout');
  } catch (caught) {
    error = caught;
  }

  // Assert
  assert.match(error?.message ?? '', /timeout must be an integer/);
  assert.equal(calls.length, 0);
});

test('reset on a closed Context maps the native target-closed error', () => {
  // Arrange
  const { adapter, context } = fixture({
    setContextDefaultTimeout() { throw new Error('TargetClosed'); },
  });
  let error;

  // Act
  try {
    adapter.setContext(context, undefined, true, 'browserContext.setDefaultNavigationTimeout');
  } catch (caught) {
    error = caught;
  }

  // Assert
  assert.equal(error?.message, 'browserContext.setDefaultNavigationTimeout: TargetClosed');
});
