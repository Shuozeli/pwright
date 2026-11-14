import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function fixture(nativeError) {
  const calls = [];
  const nativeLocator = name => ({
    nth(index) {
      calls.push(['nth', index]);
      return nativeLocator('first');
    },
    async dispatchEvent(...args) {
      calls.push(['dispatchEvent', name, ...args]);
      if (nativeError) throw nativeError;
    },
  });
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
      return nativeLocator('base');
    },
  };
  class NativeEnvironment {
    async connectOverCDP() { return session; }
  }
  const artifact = '/test-only/page-dispatch-event.node';
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

test('Page dispatchEvent selects the first match and forwards the initializer to Rust', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);
  const eventInit = { detail: { count: 2 }, bubbles: true };

  // Act
  await page.dispatchEvent('button', 'click', eventInit, { timeout: 73 });

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', 'button'],
    ['nth', 0],
    ['dispatchEvent', 'first', 'click', eventInit, 73],
  ]);
});

test('Page dispatchEvent strict mode leaves unique-match resolution to Rust', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.dispatchEvent('#only', 'focus', undefined, { strict: true });

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', '#only'],
    ['dispatchEvent', 'base', 'focus', {}, 30_000],
  ]);
});

test('Page dispatchEvent preserves null eventInit without language-side serialization', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.dispatchEvent('input', 'change', null, { strict: false });

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', 'input'],
    ['nth', 0],
    ['dispatchEvent', 'first', 'change', null, 30_000],
  ]);
});

test('Page dispatchEvent rejects invalid options and type before selecting a Locator', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  const invalidStrict = page.dispatchEvent('button', 'click', {}, { strict: 'yes' });
  const unsupported = page.dispatchEvent('button', 'click', {}, { trial: true });
  const invalidType = page.dispatchEvent('button', 42);

  // Assert
  await assert.rejects(invalidStrict, /page\.dispatchEvent: strict must be a boolean/);
  await assert.rejects(unsupported, /page\.dispatchEvent option trial is not supported/);
  await assert.rejects(invalidType, /page\.dispatchEvent: type must be a string/);
  assert.deepEqual(value.calls, []);
});

test('Page dispatchEvent preserves typed native errors from the Locator path', async () => {
  // Arrange
  const value = fixture(new Error('PWRIGHT_TIMEOUT: Timeout 73ms exceeded'));
  const page = await pageFor(value);

  // Act
  const operation = page.dispatchEvent('button', 'click', {}, { timeout: 73 });

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.name, 'TimeoutError');
    assert.equal(error.code, 'PWRIGHT_TIMEOUT');
    return true;
  });
  assert.deepEqual(value.calls.map(call => call[0]), ['locator', 'nth', 'dispatchEvent']);
});

test('Locator dispatchEvent presents a foreign-world Handle error like Playwright', async () => {
  // Arrange
  const value = fixture(new Error('PWRIGHT_BACKEND: ProtocolViolation: Handle `24` belongs to a different execution world'));
  const page = await pageFor(value);

  // Act
  const operation = page.locator('div').dispatchEvent('drop', { dataTransfer: {} });

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message, 'locator.dispatchEvent: JSHandles can be evaluated only in the context they were created!');
    return true;
  });
  assert.deepEqual(value.calls.map(call => call[0]), ['locator', 'dispatchEvent']);
});
