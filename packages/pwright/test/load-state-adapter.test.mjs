import assert from 'node:assert/strict';
import { test } from 'node:test';
import loadStateAdapter from '../src/load-state.cjs';

class UnsupportedOperationError extends Error {}

function fixture(overrides = {}, mapError = (error, api) => new Error(`${api}: mapped ${error.message}`)) {
  const calls = [];
  const session = {
    pageDefaultTimeout(page, navigation) {
      calls.push(['default', page, navigation]);
      return 4321;
    },
    async frameWaitForLoadState(page, frame, state, timeout) {
      calls.push(['wait', page, frame, state, timeout]);
    },
    ...overrides,
  };
  const page = { _id: 'page-1', _context: { _browser: { _session: session } } };
  const frame = { _id: 'frame-2', _page: page };
  const adapter = loadStateAdapter({
    mapError,
    validateOptions(options, supported, api) {
      if (!options || typeof options !== 'object' || Array.isArray(options))
        throw new TypeError(`${api}: options must be an object`);
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new UnsupportedOperationError(`${api} option ${key}`);
      }
    },
    UnsupportedOperationError,
  });
  return { adapter, calls, frame };
}

test('default load wait obtains the effective navigation timeout from Rust', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  await adapter.wait(frame);

  // Assert
  assert.deepEqual(calls, [
    ['default', 'page-1', true],
    ['wait', 'page-1', 'frame-2', 'load', 4321],
  ]);
});

test('explicit lifecycle and timeout reach the native Frame identity unchanged', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  await adapter.wait(frame, 'domcontentloaded', { timeout: 17 }, 'page.waitForLoadState');

  // Assert
  assert.deepEqual(calls, [['wait', 'page-1', 'frame-2', 'domcontentloaded', 17]]);
});

test('invalid lifecycle is rejected before native work', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const error = await adapter.wait(frame, 'bad').catch(value => value);

  // Assert
  assert.match(error.message, /state: expected one of \(load\|domcontentloaded\|networkidle\|commit\)/);
  assert.deepEqual(calls, []);
});

test('invalid options and timeout are rejected before native work', async () => {
  for (const options of [null, { unsupported: true }, { timeout: -1 }, { timeout: 1.5 }]) {
    // Arrange
    const { adapter, calls, frame } = fixture();

    // Act
    const error = await adapter.wait(frame, 'load', options).catch(value => value);

    // Assert
    assert.ok(error instanceof Error);
    assert.deepEqual(calls, []);
  }
});

test('native failures are mapped with the public API name', async () => {
  // Arrange
  const { adapter, frame } = fixture({
    async frameWaitForLoadState() { throw new Error('sentinel'); },
  });

  // Act
  const error = await adapter.wait(frame, 'load', { timeout: 1 }, 'page.waitForLoadState')
    .catch(value => value);

  // Assert
  assert.equal(error.message, 'page.waitForLoadState: mapped sentinel');
});

test('timeout parity adds punctuation and restores the captured public call site', async () => {
  // Arrange
  const { adapter, frame } = fixture({
    async frameWaitForLoadState() { throw new Error('native timeout'); },
  }, (_error, api) => {
    const mapped = new Error(`${api}: Timeout 1ms exceeded`);
    mapped.name = 'TimeoutError';
    mapped.code = 'PWRIGHT_TIMEOUT';
    return mapped;
  });
  const apiStack = { stack: 'Error\n    at official-test.spec.ts:38:9' };

  // Act
  const error = await adapter.wait(
    frame,
    'load',
    { timeout: 1 },
    'page.waitForLoadState',
    apiStack,
  ).catch(value => value);

  // Assert
  assert.equal(error.message, 'page.waitForLoadState: Timeout 1ms exceeded.');
  assert.equal(error.stack.split('\n')[1], '    at official-test.spec.ts:38:9');
});
