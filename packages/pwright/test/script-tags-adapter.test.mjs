import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const createAdapter = createRequire(import.meta.url)('../src/script-tags.cjs');

function adapter() {
  return createAdapter({
    validateOptions(options, supported, api) {
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new Error(`${api}: unsupported option ${key}`);
      }
    },
  });
}

test('script-tag adapter delegates normalized content to retained Handle evaluation', async () => {
  // Arrange
  const expected = { asElement: () => ({}) };
  const calls = [];
  const frame = {
    async evaluateHandle(source, value) {
      calls.push({ source, value });
      return expected;
    },
  };

  // Act
  const result = await adapter().add(frame, { content: 'globalThis.answer = 42', type: 'module' });

  // Assert
  assert.equal(result, expected);
  assert.equal(typeof calls[0].source, 'function');
  assert.deepEqual(calls[0].value, {
    url: null,
    content: 'globalThis.answer = 42',
    type: 'module',
  });
});

test('script-tag adapter rejects missing source before evaluation', async () => {
  // Arrange
  let evaluations = 0;
  const frame = { evaluateHandle() { evaluations++; } };

  // Act
  const result = adapter().add(frame, {});

  // Assert
  await assert.rejects(result, /Provide an object with a `url`, `path` or `content` property/);
  assert.equal(evaluations, 0);
});

test('inline module publication does not wait for a load event', async () => {
  // Arrange
  const previousDocument = globalThis.document;
  const listeners = new Map();
  globalThis.document = {
    createElement() { return {}; },
    addEventListener(type, listener) { listeners.set(type, listener); },
    removeEventListener(type, listener) {
      if (listeners.get(type) === listener) listeners.delete(type);
    },
    head: { appendChild() {} },
  };
  let published;
  const frame = {
    async evaluateHandle(source, value) {
      published = await source(value);
      return published;
    },
  };

  try {
    // Act
    const result = await adapter().add(frame, { content: 'export default 42', type: 'module' });

    // Assert
    assert.equal(result, published);
    assert.equal(published.type, 'module');
    assert.equal(listeners.size, 0);
  } finally {
    globalThis.document = previousDocument;
  }
});

test('inline CSP violation rejects before publishing the handle', async () => {
  // Arrange
  const previousDocument = globalThis.document;
  const listeners = new Map();
  globalThis.document = {
    createElement() { return {}; },
    addEventListener(type, listener) { listeners.set(type, listener); },
    removeEventListener(type, listener) {
      if (listeners.get(type) === listener) listeners.delete(type);
    },
    head: {
      appendChild() {
        listeners.get('securitypolicyviolation')?.({
          blockedURI: 'inline',
          violatedDirective: "script-src 'self'",
        });
      },
    },
  };
  const frame = { evaluateHandle: (source, value) => source(value) };

  try {
    // Act
    const result = adapter().add(frame, { content: 'globalThis.blocked = true' });

    // Assert
    await assert.rejects(result, /Content Security Policy|script-src/);
    assert.equal(listeners.size, 0);
  } finally {
    globalThis.document = previousDocument;
  }
});
