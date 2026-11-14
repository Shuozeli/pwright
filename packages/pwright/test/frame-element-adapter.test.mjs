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
          id: 'page', mainFrameId: 'main',
          frameTree: {
            frames: () => [
              { id: 'main', parentId: null },
              { id: 'child', parentId: 'main' },
            ],
            metadata: () => ({ url: () => 'about:blank', name: () => '', isDetached: () => false }),
          },
        },
      };
    },
    async frameElement(id) {
      calls.push(['frameElement', id]);
      return { isElement: () => true, bindWrapper() {} };
    },
    async handleDispose(native) { calls.push(['handleDispose', native]); },
  };
  class NativeEnvironment {
    async connectOverCDP() { return session; }
  }
  const artifact = '/test-only/frame-element.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'frame-element' },
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, module, module.exports);
  return { api: module.exports, calls, session };
}

async function childFrame(value) {
  const browser = await value.api.chromium.connectOverCDP('http://example.invalid:9222');
  const page = await browser.newPage();
  const [main, child] = page.frames();
  assert.equal(child.parentFrame(), main);
  return child;
}

test('Frame.frameElement forwards only child identity and publishes the native ElementHandle', async () => {
  // Arrange
  const value = fixture();
  const child = await childFrame(value);

  // Act
  const owner = await child.frameElement();

  // Assert
  assert.equal(owner.asElement(), owner);
  assert.deepEqual(value.calls, [['frameElement', 'child']]);
});

test('Frame.frameElement fails closed when the opt-in native port is absent', async () => {
  // Arrange
  const value = fixture();
  const child = await childFrame(value);
  delete value.session.frameElement;

  // Act
  const result = child.frameElement();

  // Assert
  await assert.rejects(result, error => error.code === 'PWRIGHT_UNSUPPORTED');
  assert.deepEqual(value.calls, []);
});

test('Frame.frameElement preserves the native detached-frame failure', async () => {
  // Arrange
  const value = fixture();
  const child = await childFrame(value);
  value.session.frameElement = async id => {
    value.calls.push(['frameElement', id]);
    throw new Error('PWRIGHT_BACKEND: Frame has been detached.');
  };

  // Act
  const result = child.frameElement();

  // Assert
  await assert.rejects(result, /frame\.frameElement: Frame has been detached\./);
  assert.deepEqual(value.calls, [['frameElement', 'child']]);
});

test('Frame.frameElement releases a native owner when wrapper publication fails', async () => {
  // Arrange
  const value = fixture();
  const child = await childFrame(value);
  const native = { isElement: () => { throw new Error('classification failed'); } };
  value.session.frameElement = async id => {
    value.calls.push(['frameElement', id]);
    return native;
  };

  // Act
  const result = child.frameElement();

  // Assert
  await assert.rejects(result, /classification failed/);
  assert.deepEqual(value.calls, [['frameElement', 'child'], ['handleDispose', native]]);
});
