import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function fixture() {
  const calls = [];
  const nativeLocator = name => ({
    nth(index) {
      calls.push(['nth', index]);
      return nativeLocator('first');
    },
    async act(...args) { calls.push(['act', name, ...args]); },
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
  const artifact = '/test-only/page-click.node';
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

test('Page click chooses the first selector match and passes pointer options to Rust', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.click('button', { button: 'right', clickCount: 3, timeout: 91 });

  // Assert
  assert.deepEqual(value.calls.slice(0, 2), [
    ['locator', 'main-frame', 'button'],
    ['nth', 0],
  ]);
  assert.deepEqual(value.calls[2].slice(0, 5), ['act', 'first', 'click', 91, {
    timeout: 91, button: 'right', clickCount: 3, delay: undefined,
    force: undefined, modifiers: undefined, noWaitAfter: undefined,
    position: undefined, trial: undefined,
  }]);
});

test('Page and Frame selector clicks honor strict mode without weakening Locator strictness', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);
  const frame = page.mainFrame();

  // Act
  await page.click('#page', { strict: true });
  await frame.click('#frame', { strict: true });
  await page.locator('#locator').click();

  // Assert
  assert.deepEqual(value.calls.filter(call => call[0] === 'nth'), []);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act').map(call => call.slice(1, 4)), [
    ['base', 'click', 30_000],
    ['base', 'click', 30_000],
    ['base', 'click', 30_000],
  ]);
});

test('Frame click and Page dblclick use first-match selector semantics', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.mainFrame().click('button', { strict: false, trial: true });
  await page.dblclick('button', { trial: true, timeout: 37 });

  // Assert
  assert.deepEqual(value.calls.filter(call => call[0] === 'nth'), [['nth', 0], ['nth', 0]]);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act').map(call => [
    call[1], call[2], call[3], call[4].trial,
  ]), [
    ['first', 'click', 30_000, true],
    ['first', 'dblclick', 37, true],
  ]);
});

test('Frame dblclick keeps explicit strict resolution in Rust', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  await page.mainFrame().dblclick('button', { strict: true, button: 'left' });

  // Assert
  assert.deepEqual(value.calls.filter(call => call[0] === 'nth'), []);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act').map(call => call.slice(1, 4)), [
    ['base', 'dblclick', 30_000],
  ]);
});

test('selector clicks reject invalid strict types and unsupported options before native admission', async () => {
  // Arrange
  const value = fixture();
  const page = await pageFor(value);

  // Act
  const invalidPageStrict = page.click('button', { strict: 'yes' });
  const invalidFrameStrict = page.mainFrame().dblclick('button', { strict: 1 });
  const unsupportedDoubleCount = page.dblclick('button', { clickCount: 3 });

  // Assert
  await assert.rejects(invalidPageStrict, /page\.click: strict must be a boolean/);
  await assert.rejects(invalidFrameStrict, /frame\.dblclick: strict must be a boolean/);
  await assert.rejects(unsupportedDoubleCount, /page\.dblclick option clickCount is not supported/);
  assert.deepEqual(value.calls, []);
});
