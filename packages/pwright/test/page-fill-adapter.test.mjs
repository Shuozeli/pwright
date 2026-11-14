import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);
const invalidTarget = 'Element is not an <input>, <textarea> or [contenteditable] element';

function fixture(nativeError) {
  const calls = [];
  const session = {
    isConnected: () => true,
    setLifecycleCallback() {},
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    contextSnapshot(ids) { return { ids, newContexts: [] }; },
    pageSnapshot(_contextId, ids) { return { ids, newPages: [] }; },
    async newBrowserPage() {
      return {
        context: { id: 'context', settings: {} },
        page: {
          id: 'page',
          mainFrameId: 'main-frame',
          frameTree: {
            frames: () => [{ id: 'main-frame', parentId: null }],
            metadata: () => ({ url: () => 'about:blank', name: () => '' }),
          },
        },
      };
    },
    locator(frameId, selector) {
      calls.push(['locator', frameId, selector]);
      return {
        async act(...args) {
          calls.push(['act', ...args]);
          if (nativeError) throw nativeError;
        },
      };
    },
  };
  class NativeEnvironment {
    async connectOverCDP() { return session; }
  }
  const artifact = '/test-only/page-fill.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'default' },
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, module, module.exports);
  return { api: module.exports, calls };
}

async function pageFor(value) {
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  return browser.newPage();
}

test('Page.fill forwards its string and options to the native Locator action', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.fill('input', 'ready', { force: true, timeout: 91 });

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', 'input'],
    ['act', 'fill', 91, { timeout: 91, force: true, noWaitAfter: undefined }, 'ready'],
  ]);
});

test('Page.fill presents the invalid-target error with its Page API name and call log', async () => {
  // Arrange
  const value = fixture(new Error(`PWRIGHT_BACKEND: Backend: ${invalidTarget}`));
  const page = await pageFor(value);

  // Act
  const operation = page.fill('select', '');

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message,
      `page.fill: Error: ${invalidTarget}\nCall log:\n  - waiting for locator('select')`);
    assert.equal(error.code, 'PWRIGHT_BACKEND');
    assert.equal(error.constructor, Error);
    return true;
  });
  assert.deepEqual(value.calls.map(call => call[0]), ['locator', 'act']);
});

test('Locator.fill retains its own error identity and does not borrow the Page call log', async () => {
  // Arrange
  const value = fixture(new Error(`PWRIGHT_BACKEND: Backend: ${invalidTarget}`));
  const page = await pageFor(value);

  // Act
  const operation = page.locator('select').fill('');

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message, `locator.fill: Backend: ${invalidTarget}`);
    assert.equal(error.code, 'PWRIGHT_BACKEND');
    return true;
  });
});

test('Page.fill does not rewrite unrelated backend errors', async () => {
  // Arrange
  const value = fixture(new Error('PWRIGHT_BACKEND: Backend: Input of type "file" cannot be filled'));
  const page = await pageFor(value);

  // Act
  const operation = page.fill('input', 'ready');

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message, 'page.fill: Backend: Input of type "file" cannot be filled');
    assert.equal(error.code, 'PWRIGHT_BACKEND');
    return true;
  });
});

test('Page.fill rejects a numeric value before native action admission', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  const operation = page.fill('textarea', 123);

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message, 'page.fill: value: expected string, got number');
    assert.equal(error.constructor, TypeError);
    return true;
  });
  assert.deepEqual(value.calls.map(call => call[0]), ['locator']);
});

test('Locator.fill rejects a null value before native action admission', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  const operation = page.locator('textarea').fill(null);

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message, 'locator.fill: value: expected string, got object');
    assert.equal(error.constructor, TypeError);
    return true;
  });
  assert.deepEqual(value.calls.map(call => call[0]), ['locator']);
});

test('Page.fill unwraps a boxed string as Playwright does', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.fill('input', new String('ready'));

  // Assert
  assert.equal(value.calls[1][4], 'ready');
});
