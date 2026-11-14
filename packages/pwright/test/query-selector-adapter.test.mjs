import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const createAdapter = createRequire(import.meta.url)('../src/query-selector.cjs');

test('query selector returns the Rust-owned ElementHandle unchanged', async () => {
  // Arrange
  const element = {};
  const calls = [];
  const locator = {
    async _queryElementHandle() { calls.push(['queryElementHandle']); return element; },
  };
  const frame = {
    locator(selector) {
      calls.push(['locator', selector]);
      return locator;
    },
  };

  // Act
  const result = await createAdapter().one(frame, 'div', 'page.$');

  // Assert
  assert.equal(result, element);
  assert.equal(calls.length, 2);
  assert.deepEqual(calls[0], ['locator', 'div']);
  assert.deepEqual(calls[1], ['queryElementHandle']);
});

test('query selector uses the Rust non-strict query without deriving a typed first filter', async () => {
  // Arrange
  const element = {};
  let derivedFirst = false;
  const locator = {
    async _queryElementHandle() { return element; },
    first() { derivedFirst = true; throw new Error('must remain in the atomic query'); },
  };
  const frame = {
    locator() { return locator; },
  };

  // Act
  const result = await createAdapter().one(frame, 'div', 'page.$');

  // Assert
  assert.equal(result, element);
  assert.equal(derivedFirst, false);
});

test('query selector propagates native element publication failures', async () => {
  // Arrange
  const failure = new Error('Expected a Node but got [object Array]');
  const frame = {
    locator() {
      return {
        async _queryElementHandle() { throw failure; },
      };
    },
  };

  // Act
  const result = createAdapter().one(frame, 'bad=selector', 'frame.$');

  // Assert
  await assert.rejects(result, error => error === failure);
});

test('query selector preserves the native unknown-engine diagnostic', async () => {
  // Arrange
  const failure = new Error(
    'Unknown engine "neverregister" while parsing selector neverregister=ignored',
  );
  const frame = {
    locator() {
      return {
        async _queryElementHandle() { throw failure; },
      };
    },
  };

  // Act
  const result = createAdapter().one(frame, 'neverregister=ignored', 'page.$');

  // Assert
  await assert.rejects(result, error => error === failure);
});

test('query selector rejects null with the Playwright protocol validation diagnostic', async () => {
  // Arrange
  let called = false;
  const frame = {
    locator() { called = true; return { _queryElementHandle() { throw new Error('unreachable'); } }; },
  };

  // Act
  const result = createAdapter().one(frame, null, 'page.$');

  // Assert
  await assert.rejects(result, /page\.\$: selector: expected string, got object/);
  assert.equal(called, false);
});

test('query selector returns Rust null from the same atomic Handle query', async () => {
  // Arrange
  const calls = [];
  const frame = {
    locator() {
      return {
        async _queryElementHandle() {
          calls.push('queryElementHandle');
          return null;
        },
      };
    },
  };

  // Act
  const result = await createAdapter().one(frame, '.missing', 'frame.$');

  // Assert
  assert.equal(result, null);
  assert.deepEqual(calls, ['queryElementHandle']);
});

test('query selector all returns the Rust-owned ElementHandle batch unchanged', async () => {
  // Arrange
  const elements = [{ id: 1 }, { id: 2 }];
  const calls = [];
  const frame = {
    locator(selector) {
      calls.push(['locator', selector]);
      return {
        async _queryElementHandles() {
          calls.push(['queryElementHandles']);
          return elements;
        },
      };
    },
  };

  // Act
  const result = await createAdapter().all(frame, 'div', 'page.$$');

  // Assert
  assert.equal(result, elements);
  assert.deepEqual(calls, [['locator', 'div'], ['queryElementHandles']]);
});

test('query selector all preserves the Rust-owned empty batch', async () => {
  // Arrange
  const elements = [];
  const frame = {
    locator() { return { async _queryElementHandles() { return elements; } }; },
  };

  // Act
  const result = await createAdapter().all(frame, '.missing', 'frame.$$');

  // Assert
  assert.equal(result, elements);
});

test('query selector all rejects non-string selectors before native evaluation', async () => {
  // Arrange
  let called = false;
  const frame = {
    locator() { called = true; throw new Error('unreachable'); },
  };

  // Act
  const result = createAdapter().all(frame, null, 'page.$$');

  // Assert
  await assert.rejects(result, /page\.\$\$: selector: expected string, got object/);
  assert.equal(called, false);
});
