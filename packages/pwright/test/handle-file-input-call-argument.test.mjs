// Differential coverage for the synchronous Node-language adapter only. This
// file creates no browser, native session, retained target or upload mutation.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { serializeFileInputCallArgument } = require('../src/file-input-call-argument.cjs');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { serializeAsCallArgument } = requireCore(join(core, 'lib/utils/isomorphic/utilityScriptSerializers.js'));
const oracle = value => serializeAsCallArgument(value, item => ({ fallThrough: item }));

test('late call-argument serializer matches pinned payload and special-value graphs', () => {
  // Arrange
  const cycle = { label: 'cycle' };
  cycle.self = cycle;
  const value = [{
    name: 'bytes.bin', mimeType: 'application/octet-stream', buffer: 'AQID',
    lastModifiedMs: -0, cycle, nan: NaN, infinity: Infinity, bigint: 9n,
    date: new Date('2026-09-19T00:00:00.000Z'), regexp: /pw/gi,
    typed: new Uint8Array([1, 2, 3]),
  }];
  // Act
  const actual = serializeFileInputCallArgument(value);
  const expected = oracle(value);
  // Assert
  assert.deepEqual(actual, expected);
  assert.equal(actual.a[0].o.find(entry => entry.k === 'buffer').v, value[0].buffer);
});

test('late call-argument serializer omits throwing object fields exactly like pinned Playwright', () => {
  // Arrange
  const sentinel = new Error('object getter');
  const value = { keep: 1, get omit() { throw sentinel; } };
  // Act
  const actual = serializeFileInputCallArgument(value);
  const expected = oracle(value);
  // Assert
  assert.deepEqual(actual, expected);
  assert.deepEqual(actual.o.map(entry => entry.k), ['keep']);
});

test('late call-argument serializer preserves the original throwing array getter', () => {
  // Arrange
  const sentinel = new Error('array getter');
  const value = [];
  Object.defineProperty(value, 0, { get() { throw sentinel; } });
  value.length = 1;
  // Act
  let actual;
  try { serializeFileInputCallArgument(value); } catch (error) { actual = error; }
  // Assert
  assert.equal(actual, sentinel);
});

test('late call-argument serializer matches pinned empty-object toJSON fallback', () => {
  // Arrange
  const value = Object.create(null);
  Object.defineProperty(value, 'toJSON', {
    enumerable: false,
    value() { return { projected: 'yes' }; },
  });
  // Act
  const actual = serializeFileInputCallArgument(value);
  const expected = oracle(value);
  // Assert
  assert.deepEqual(actual, expected);
  assert.equal(actual.o[0].k, 'projected');
});

test('late call-argument serializer does not assimilate Promise-shaped leaves', () => {
  // Arrange
  let thenReads = 0;
  const value = { then() { thenReads++; } };
  // Act
  const actual = serializeFileInputCallArgument(value);
  const expected = oracle(value);
  // Assert
  assert.deepEqual(actual, expected);
  assert.equal(thenReads, 0);
});
