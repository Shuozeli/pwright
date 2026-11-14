import assert from 'node:assert/strict';
import { test } from 'node:test';
import locatorFactory from '../src/locator.cjs';

function locatorWithScreenshot() {
  const calls = [];
  const bytes = Buffer.from([1, 2, 3]);
  const native = {
    async screenshot(options) {
      calls.push(['screenshot', options]);
      return bytes;
    },
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = { _id: 'frame', page: () => page };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions(options, allowed, name) {
      for (const key of Object.keys(options)) {
        if (!allowed.includes(key)) throw new Error(`${name} option ${key}`);
      }
    },
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  return { bytes, calls, locator: api.create(frame, '.box') };
}

test('Locator screenshot delegates normalized capture options to Rust', async () => {
  // Arrange
  const value = locatorWithScreenshot();

  // Act
  const bytes = await value.locator.screenshot({ type: 'jpeg', quality: 80, scale: 'css', timeout: 73 });

  // Assert
  assert.equal(bytes, value.bytes);
  assert.deepEqual(value.calls, [['screenshot', {
    format: 1,
    quality: 80,
    fullPage: false,
    scale: 1,
    clip: undefined,
    timeoutMs: 73,
  }]]);
});

test('Locator screenshot rejects Page-only fullPage before native execution', async () => {
  // Arrange
  const value = locatorWithScreenshot();

  // Act
  const operation = value.locator.screenshot({ fullPage: true });

  // Assert
  await assert.rejects(operation, /locator\.screenshot option fullPage/);
  assert.deepEqual(value.calls, []);
});
