import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction, runInNewContext } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function fixture() {
  const calls = [];
  const nativeOption = {
    isElement: () => true,
    preview: () => 'OPTION',
    bindWrapper(wrapper) { calls.push(['bindWrapper', wrapper]); },
  };
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
    async frameEvaluateHandle(...args) {
      calls.push(['frameEvaluateHandle', ...args]);
      return nativeOption;
    },
    locator(frameId, selector) {
      calls.push(['locator', frameId, selector]);
      return {
        async act(...args) {
          calls.push(['act', ...args]);
          return ['white'];
        },
      };
    },
  };
  class NativeEnvironment {
    async connectOverCDP() { return session; }
  }
  const artifact = '/test-only/page-select-option-handle.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'handles' },
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

test('Page.selectOption forwards the retained option identity to the native Locator', async () => {
  // Arrange
  const value = fixture();
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const option = await page.evaluateHandle('document.getElementById("whiteOption")');

  // Act
  const selected = await page.selectOption('select', option, { timeout: 91 });

  // Assert
  assert.deepEqual(selected, ['white']);
  assert.deepEqual(value.calls.find(call => call[0] === 'locator'), ['locator', 'main-frame', 'select']);
  const action = value.calls.find(call => call[0] === 'act');
  assert.equal(action[1], 'selectOption');
  assert.equal(action[2], 91);
  assert.strictEqual(action[4], option);
});

const addonSource = readFileSync(new URL('../../../crates/pwright-node-addon/src/locator/actions.rs', import.meta.url), 'utf8');
const predicateSource = addonSource.match(/const OPTION_INDEX_SOURCE: &str = "([^"]+)";/)?.[1];

for (const { name, target, option, result } of (() => {
  const first = { tagName: 'OPTION', isConnected: true };
  const second = { tagName: 'OPTION', isConnected: true };
  const foreign = { tagName: 'OPTION', isConnected: true };
  const detached = { tagName: 'OPTION', isConnected: false };
  const select = { tagName: 'SELECT', options: [first, second] };
  return [
    { name: 'the second retained option of the target select', target: select, option: second, result: '1' },
    { name: 'an option from a different select', target: select, option: foreign, result: 'invalid-option' },
    { name: 'a detached option', target: select, option: detached, result: 'invalid-option' },
    { name: 'a non-select target', target: { tagName: 'DIV' }, option: first, result: 'invalid-target' },
  ];
})()) {
  test(`native select-option predicate checks ${name} by object identity`, () => {
    // Arrange
    assert.equal(typeof predicateSource, 'string');
    const check = runInNewContext(`(${predicateSource})`);

    // Act
    const actual = check(target, option);

    // Assert
    assert.equal(actual, result);
  });
}
