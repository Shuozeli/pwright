import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const createSelectors = createRequire(import.meta.url)('../src/selectors.cjs');

function adapter(environment, overrides = {}) {
  return createSelectors({
    getNativeEnvironment: () => environment,
    mapError(error, api) {
      const match = /^(PWRIGHT_[A-Z_]+): ([\s\S]*)$/.exec(error.message);
      return match ? new Error(`${api}: ${match[2]}`) : error;
    },
    validateOptions(options, supported, api) {
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new Error(`${api}: unsupported ${key}`);
      }
    },
    ...overrides,
  });
}

test('Selectors serializes functions and delegates content-script ownership to Rust', async () => {
  // Arrange
  const calls = [];
  const selectors = adapter({
    registerSelectorEngine(...args) { calls.push(args); },
    setTestIdAttribute() {},
  });
  const createEngine = () => ({ query() {}, queryAll() { return []; } });

  // Act
  await selectors.register('tag', createEngine, { contentScript: true });

  // Assert
  assert.deepEqual(calls, [[
    'tag',
    `(${createEngine.toString()})(undefined)`,
    true,
  ]]);
});

test('Selectors reads path sources before entering the native registry', async () => {
  // Arrange
  const calls = [];
  const reads = [];
  const selectors = adapter({
    registerSelectorEngine(...args) { calls.push(args); },
    setTestIdAttribute() {},
  }, {
    async readSelectorFile(...args) {
      reads.push(args);
      return '({ query() {}, queryAll() { return []; } })';
    },
  });

  // Act
  await selectors.register('fileEngine', { path: '/tmp/engine.js' });

  // Assert
  assert.deepEqual(reads, [['/tmp/engine.js', 'utf8']]);
  assert.deepEqual(calls, [[
    'fileEngine',
    '({ query() {}, queryAll() { return []; } })',
    false,
  ]]);
});

test('Selectors exposes the official lowercase duplicate-registration error', async () => {
  // Arrange
  const names = new Set();
  const selectors = adapter({
    registerSelectorEngine(name) {
      if (names.has(name)) {
        throw new Error(
          `PWRIGHT_BACKEND: Selectors.register: "${name}" selector engine has been already registered`,
        );
      }
      names.add(name);
    },
    setTestIdAttribute() {},
  });
  const engine = () => ({ query() {}, queryAll() { return []; } });
  await selectors.register('alreadyRegistered', engine);

  // Act
  const duplicate = selectors.register('alreadyRegistered', engine);

  // Assert
  await assert.rejects(duplicate, {
    message: 'selectors.register: "alreadyRegistered" selector engine has been already registered',
  });
});

test('Selectors exposes the official invalid-name error', async () => {
  // Arrange
  const selectors = adapter({
    registerSelectorEngine() {
      throw new Error(
        'PWRIGHT_BACKEND: Selectors.register: Selector engine name may only contain [a-zA-Z0-9_] characters',
      );
    },
    setTestIdAttribute() {},
  });

  // Act
  const registration = selectors.register('$', '({ query() {}, queryAll() { return []; } })');

  // Assert
  await assert.rejects(registration, {
    message: 'selectors.register: Selector engine name may only contain [a-zA-Z0-9_] characters',
  });
});

test('Selectors exposes the official predefined-engine error', async () => {
  // Arrange
  const selectors = adapter({
    registerSelectorEngine() {
      throw new Error(
        'PWRIGHT_BACKEND: Selectors.register: "css" is a predefined selector engine',
      );
    },
    setTestIdAttribute() {},
  });

  // Act
  const registration = selectors.register('css', '({ query() {}, queryAll() { return []; } })');

  // Assert
  await assert.rejects(registration, {
    message: 'selectors.register: "css" is a predefined selector engine',
  });
});

test('Selectors forwards test-id configuration synchronously', () => {
  // Arrange
  const values = [];
  const selectors = adapter({
    registerSelectorEngine() {},
    setTestIdAttribute(value) { values.push(value); },
  });

  // Act
  const result = selectors.setTestIdAttribute('data-pwright-id');

  // Assert
  assert.equal(result, undefined);
  assert.deepEqual(values, ['data-pwright-id']);
});
