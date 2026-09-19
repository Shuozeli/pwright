import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const options = require('../src/screenshot.cjs')({ validateOptions(value, keys) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new TypeError('options object required');
  if (Object.keys(value).some(key => !keys.includes(key))) throw new Error('unsupported option');
} });

test('screenshot defaults marshal without any browser connection', () => {
  // Arrange
  const input = {};
  // Act
  const result = options(input);
  // Assert
  assert.deepEqual(result, { format: 0, quality: undefined, fullPage: false, scale: 0, clip: undefined, timeoutMs: 30000 });
});

for (const [name, input] of [
  ['quality on PNG', { quality: 80 }], ['quality overflow', { type: 'jpeg', quality: 101 }],
  ['fractional timeout', { timeout: 0.5 }], ['nonfinite clip', { clip: { x: 0, y: 0, width: Infinity, height: 10 } }],
  ['unsupported filesystem path', { path: 'capture.png' }],
]) {
  test(`screenshot rejects ${name}`, () => {
    // Arrange
    const capture = () => options(input);
    // Act / Assert
    assert.throws(capture);
  });
}
