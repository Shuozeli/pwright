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
    async act(...args) {
      calls.push(['act', name, ...args]);
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
  const artifact = '/test-only/page-checked.node';
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

test('Page check selects the first match and forwards action options to Rust', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);
  const options = { force: true, noWaitAfter: true, position: { x: 3, y: 4 }, timeout: 91, trial: true };

  // Act
  await page.check('input', options);

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', 'input'],
    ['nth', 0],
    ['act', 'first', 'check', 91, {
      timeout: 91, force: true, position: { x: 3, y: 4 }, trial: true, noWaitAfter: true,
    }, undefined],
  ]);
});

test('Page uncheck with strict mode keeps unique-match resolution in Rust', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.uncheck('#radio', { strict: true });

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', '#radio'],
    ['act', 'base', 'uncheck', 30_000, {
      timeout: undefined, force: undefined, position: undefined, trial: undefined,
      noWaitAfter: undefined,
    }, undefined],
  ]);
});

test('Page setChecked forwards the desired Boolean state through the native action port', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.setChecked('#checkbox', false, { strict: true, trial: true });

  // Assert
  assert.deepEqual(value.calls, [
    ['locator', 'main-frame', '#checkbox'],
    ['act', 'base', 'setChecked', 30_000, {
      timeout: undefined, force: undefined, position: undefined, trial: true,
      noWaitAfter: undefined,
    }, false],
  ]);
});

test('Page checked actions reject invalid and unsupported options before native admission', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  const invalidStrict = page.check('input', { strict: 'yes' });
  const unsupportedScroll = page.uncheck('input', { scroll: 'none' });

  // Assert
  await assert.rejects(invalidStrict, /page\.check: strict must be a boolean/);
  await assert.rejects(unsupportedScroll, /page\.uncheck option scroll is not supported/);
  assert.deepEqual(value.calls, []);
});

test('Page checked action maps native checkbox errors to the Page API name', async () => {
  // Arrange
  const value = fixture(new Error('PWRIGHT_INVALID_ARGUMENT: Cannot uncheck radio button'));
  const page = await pageFor(value);

  // Act
  const operation = page.uncheck('#radio');

  // Assert
  await assert.rejects(operation, error => {
    assert.equal(error.message, 'page.uncheck: Cannot uncheck radio button');
    assert.equal(error.code, 'PWRIGHT_INVALID_ARGUMENT');
    return true;
  });
  assert.deepEqual(value.calls.map(call => call[0]), ['locator', 'nth', 'act']);
});
