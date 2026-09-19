import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import * as api from '@shuozeli/pwright';

test('Handle native selection requires its own binary marker and fixed path', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const binary = { handlesEnabled: () => true };
  // Act
  const selected = resolveNativeArtifact('handles');
  // Assert
  assert.deepEqual(nativeFeatures('handles'), ['handles']);
  assert.match(selected.path, /\/native\/handles\/pwright\.node$/);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
  assert.throws(() => verifyNativeArtifact({}, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('native profiles select distinct fixed paths without loading a binary', () => {
  // Arrange
  const { resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  // Act
  const base = resolveNativeArtifact('default');
  const selected = resolveNativeArtifact('drag-runtime');
  // Assert
  assert.match(base.path, /\/native\/pwright\.node$/);
  assert.match(selected.path, /\/native\/drag-runtime\/pwright\.node$/);
  assert.notEqual(base.path, selected.path);
  assert.equal(Object.isFrozen(selected), true);
});

test('native profile selection rejects unknown names and path injection', () => {
  // Arrange
  const { resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['', '../default', '/tmp/custom.node', 'drag-source', undefined, null, ['default'], { toString: () => 'default' }]) {
    // Act / Assert
    assert.throws(() => resolveNativeArtifact(profile), TypeError);
  }
});

test('eight native profiles enforce independent compiled feature combinations', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const profiles = ['default', 'drag-runtime', 'frame-navigation', 'drag-frame-navigation',
    'extra-headers', 'drag-extra-headers', 'frame-extra-headers', 'drag-frame-extra-headers'];
  for (const [index, profile] of profiles.entries()) {
    const selected = resolveNativeArtifact(profile);
    // Act
    const features = nativeFeatures(profile);
    // Assert
    assert.equal(Object.isFrozen(features), true);
    assert.equal(features.includes('drag-runtime'), Boolean(index & 1));
    assert.equal(features.includes('frame-navigation'), Boolean(index & 2));
    assert.equal(features.includes('extra-headers'), Boolean(index & 4));
    if (profile !== 'default') assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    for (let compiled = 0; compiled < 8; compiled++) {
      const binary = { dragRuntimeEnabled: () => Boolean(compiled & 1), frameNavigationEnabled: () => Boolean(compiled & 2), extraHeadersEnabled: () => Boolean(compiled & 4) };
      if (compiled === index) assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
      else assert.throws(() => verifyNativeArtifact(binary, selected), /profile mismatch/);
    }
  }
});

test('concurrent close profiles require their own compiled marker and isolated path', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['concurrent-close', 'concurrent-close-drag-frame-extra-headers']) {
    const selected = resolveNativeArtifact(profile);
    const combined = profile !== 'concurrent-close';
    const binary = { dragRuntimeEnabled: () => combined, frameNavigationEnabled: () => combined,
      extraHeadersEnabled: () => combined, concurrentCloseEnabled: () => true };
    // Act
    const features = nativeFeatures(profile);
    // Assert
    assert.ok(features.includes('concurrent-close'));
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, concurrentCloseEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(combined ? 'drag-frame-extra-headers' : 'default')), /profile mismatch/);
  }
});

test('upload profiles require the upload marker without selecting it in ordinary binaries', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['file-input', 'file-input-concurrent-close-drag-frame-extra-headers']) {
    const combined = profile !== 'file-input';
    const selected = resolveNativeArtifact(profile);
    const binary = { fileInputEnabled: () => true, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
      concurrentCloseEnabled: () => combined };
    // Act
    const features = nativeFeatures(profile);
    // Assert
    assert.ok(features.includes('file-input'));
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, fileInputEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, fileInputEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(combined ? 'concurrent-close-drag-frame-extra-headers' : 'default')), /profile mismatch/);
  }
});

test('retained upload preparation profiles require all implied markers and isolated paths', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-file-input', 'retained-file-input-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true, retainedFileInputEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      ...(combined ? ['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'] : [])].sort());
    assert.equal(Object.isFrozen(features), true);
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    for (const marker of ['handlesEnabled', 'fileInputEnabled', 'retainedFileInputEnabled']) {
      assert.throws(() => verifyNativeArtifact({ ...binary, [marker]: undefined }, selected), /profile mismatch/);
      assert.throws(() => verifyNativeArtifact({ ...binary, [marker]: () => false }, selected), /profile mismatch/);
    }
  }
});

test('retained target-marking profiles add only their explicit compiled marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-target-marking', 'retained-target-marking-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetMarkingEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetMarkingEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: () => true }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-file-input-combined' : 'retained-file-input')), /profile mismatch/);
  }
});

test('retained target-prevalidation profiles add only their explicit compiled marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-target-prevalidation', 'retained-target-prevalidation-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation',
      ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-target-marking-combined' : 'retained-target-marking')), /profile mismatch/);
  }
});

test('retained input-instrumentation profiles add only their addon phase marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-input-instrumentation', 'retained-input-instrumentation-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedInputInstrumentationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedInputInstrumentationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-target-prevalidation-combined' : 'retained-target-prevalidation')), /profile mismatch/);
  }
});

test('retained call-argument-serialization profiles add only their addon phase marker', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-call-argument-serialization', 'retained-call-argument-serialization-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      retainedCallArgumentSerializationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      'retained-call-argument-serialization', ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedCallArgumentSerializationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedCallArgumentSerializationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-input-instrumentation-combined' : 'retained-input-instrumentation')), /profile mismatch/);
  }
});

test('retained serialized-payload-import profiles add the host and addon ownership leaf', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-serialized-payload-import', 'retained-serialized-payload-import-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      retainedCallArgumentSerializationEnabled: () => true,
      retainedSerializedPayloadImportEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      'retained-call-argument-serialization', 'retained-serialized-payload-import',
      ...(combined ? ['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadImportEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadImportEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-call-argument-serialization-combined' : 'retained-call-argument-serialization')), /profile mismatch/);
  }
});

test('retained serialized-payload-mutation profiles add only the terminal mutation leaf', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  for (const profile of ['retained-serialized-payload-mutation', 'retained-serialized-payload-mutation-combined']) {
    const combined = profile.endsWith('-combined');
    const binary = {
      handlesEnabled: () => true, fileInputEnabled: () => true,
      retainedFileInputEnabled: () => true, retainedTargetMarkingEnabled: () => true,
      retainedTargetPrevalidationEnabled: () => true,
      retainedInputInstrumentationEnabled: () => true,
      retainedCallArgumentSerializationEnabled: () => true,
      retainedSerializedPayloadImportEnabled: () => true,
      retainedSerializedPayloadMutationEnabled: () => true,
      concurrentCloseEnabled: () => combined, dragRuntimeEnabled: () => combined,
      frameNavigationEnabled: () => combined, extraHeadersEnabled: () => combined,
    };
    // Act
    const selected = resolveNativeArtifact(profile);
    const features = nativeFeatures(profile);
    // Assert
    assert.deepEqual([...features].sort(), ['handles', 'file-input', 'retained-file-input',
      'retained-target-marking', 'retained-target-prevalidation', 'retained-input-instrumentation',
      'retained-call-argument-serialization', 'retained-serialized-payload-import',
      'retained-serialized-payload-mutation', ...(combined ? ['concurrent-close', 'drag-runtime',
        'frame-navigation', 'extra-headers'] : [])].sort());
    assert.ok(selected.path.endsWith(`/native/${profile}/pwright.node`));
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadMutationEnabled: undefined }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedSerializedPayloadMutationEnabled: () => false }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact(
      combined ? 'retained-serialized-payload-import-combined' : 'retained-serialized-payload-import')), /profile mismatch/);
  }
});

test('existing artifact profiles keep their exact features and reject retained preparation binaries', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const existing = {
    default: [], handles: ['handles'], 'file-input': ['file-input'],
    'file-input-concurrent-close-drag-frame-extra-headers': ['file-input', 'concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'],
    'drag-runtime': ['drag-runtime'], 'frame-navigation': ['frame-navigation'],
    'drag-frame-navigation': ['drag-runtime', 'frame-navigation'],
    'extra-headers': ['extra-headers'], 'drag-extra-headers': ['drag-runtime', 'extra-headers'],
    'frame-extra-headers': ['frame-navigation', 'extra-headers'],
    'drag-frame-extra-headers': ['drag-runtime', 'frame-navigation', 'extra-headers'],
    'concurrent-close': ['concurrent-close'],
    'concurrent-close-drag-frame-extra-headers': ['concurrent-close', 'drag-runtime', 'frame-navigation', 'extra-headers'],
  };
  for (const [profile, expected] of Object.entries(existing)) {
    const binary = {
      handlesEnabled: () => expected.includes('handles'), fileInputEnabled: () => expected.includes('file-input'),
      dragRuntimeEnabled: () => expected.includes('drag-runtime'), frameNavigationEnabled: () => expected.includes('frame-navigation'),
      extraHeadersEnabled: () => expected.includes('extra-headers'), concurrentCloseEnabled: () => expected.includes('concurrent-close'),
      retainedTargetPrevalidationEnabled: () => false,
    };
    // Act
    const features = nativeFeatures(profile);
    const selected = resolveNativeArtifact(profile);
    // Assert
    assert.deepEqual(features, expected, `${profile}: previous exact feature selection`);
    assert.doesNotThrow(() => verifyNativeArtifact(binary, selected));
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedFileInputEnabled: () => true }, selected), /profile mismatch/);
    assert.throws(() => verifyNativeArtifact({ ...binary, retainedTargetPrevalidationEnabled: () => true }, selected), /profile mismatch/);
  }
});

test('selected native profile requires a matching compiled capability', () => {
  // Arrange
  const { verifyNativeArtifact, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const selected = resolveNativeArtifact('drag-runtime');
  // Act / Assert
  assert.throws(() => verifyNativeArtifact({}, selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ dragRuntimeEnabled: () => false }, selected), /profile mismatch/);
  assert.doesNotThrow(() => verifyNativeArtifact({ dragRuntimeEnabled: () => true }, selected));
});

test('default accepts the earlier default binary but refuses selected capability', () => {
  // Arrange
  const { verifyNativeArtifact, resolveNativeArtifact } = createRequire(import.meta.url)('../src/native-artifact.cjs');
  const base = resolveNativeArtifact('default');
  // Act / Assert
  assert.doesNotThrow(() => verifyNativeArtifact({}, base));
  assert.doesNotThrow(() => verifyNativeArtifact({ dragRuntimeEnabled: () => false }, base));
  assert.throws(() => verifyNativeArtifact({ dragRuntimeEnabled: () => true }, base), /profile mismatch/);
});

test('Keyboard facade delegates five operations and fractional delay with the same Page ID', async () => {
  // Arrange
  const calls = [];
  const Keyboard = createRequire(import.meta.url)('../src/keyboard.cjs')({ mapError: error => error, validateOptions: () => {} });
  const keyboard = new Keyboard({ async keyboard(...args) { calls.push(args); } }, '42');
  // Act
  await keyboard.down('Shift');
  await keyboard.up('Shift');
  await keyboard.press('KeyA', { delay: 0.5 });
  await keyboard.insertText('你好');
  await keyboard.type('text');
  // Assert
  assert.deepEqual(calls, [['42', 'down', 'Shift', 0], ['42', 'up', 'Shift', 0], ['42', 'press', 'KeyA', 0.5], ['42', 'insertText', '你好', 0], ['42', 'type', 'text', 0]]);
});

test('Keyboard invalid arguments reject asynchronously before native delegation', async () => {
  // Arrange
  let called = false;
  const Keyboard = createRequire(import.meta.url)('../src/keyboard.cjs')({ mapError: error => error, validateOptions: () => {} });
  const keyboard = new Keyboard({ async keyboard() { called = true; } }, '42');
  let result;
  // Act
  assert.doesNotThrow(() => { result = keyboard.press('A', { delay: NaN }); });
  // Assert
  await assert.rejects(result, TypeError);
  assert.equal(called, false);
});

test('Mouse facade delegates six methods without maintaining pointer state', async () => {
  // Arrange
  const calls = [];
  const Mouse = createRequire(import.meta.url)('../src/mouse.cjs')({ mapError: error => error, validateOptions: () => {} });
  const mouse = new Mouse({ async mouse(...args) { calls.push(args); } }, '42');
  const defaults = { button: 0, steps: 1, clickCount: 1, delayMs: 0 };
  // Act
  await mouse.move(10.5, 20, { steps: 4 });
  await mouse.down({ button: 'right', clickCount: 2 });
  await mouse.up();
  await mouse.click(30, 40, { delay: 0.5, clickCount: 3 });
  await mouse.dblclick(50, 60, { button: 'middle' });
  await mouse.wheel(-1.5, 2.5);
  // Assert
  assert.deepEqual(calls, [
    ['42', 'move', 10.5, 20, { ...defaults, steps: 4 }],
    ['42', 'down', 0, 0, { ...defaults, button: 1, clickCount: 2 }],
    ['42', 'up', 0, 0, defaults],
    ['42', 'click', 30, 40, { ...defaults, delayMs: 0.5, clickCount: 3 }],
    ['42', 'dblclick', 50, 60, { ...defaults, button: 2 }],
    ['42', 'wheel', -1.5, 2.5, defaults],
  ]);
});

test('Mouse rejects wrong JS types asynchronously before native delegation', async () => {
  // Arrange
  let calls = 0;
  const Mouse = createRequire(import.meta.url)('../src/mouse.cjs')({ mapError: error => error, validateOptions: () => {} });
  const mouse = new Mouse({ async mouse() { calls++; } }, '42');
  // Act
  const outcomes = await Promise.allSettled([
    mouse.move('1', 2), mouse.wheel(0, Infinity), mouse.down({ button: 'back' }),
    mouse.click(1, 2, { clickCount: '3' }),
  ]);
  // Assert
  assert.equal(calls, 0);
  for (const outcome of outcomes) {
    assert.equal(outcome.status, 'rejected');
    assert.ok(outcome.reason instanceof TypeError);
  }
});

test('Mouse maps Rust failures with the public method name', async () => {
  // Arrange
  const nativeError = new Error('native failure');
  const Mouse = createRequire(import.meta.url)('../src/mouse.cjs')({ mapError: (error, api) => new Error(`${api}: ${error.message}`), validateOptions: () => {} });
  const mouse = new Mouse({ async mouse() { throw nativeError; } }, '42');
  // Act
  const result = mouse.dblclick(10, 20);
  // Assert
  await assert.rejects(result, { message: 'mouse.dblclick: native failure' });
});

test('ESM and CommonJS share the public API and error identity', () => {
  // Arrange
  const require = createRequire(import.meta.url);
  // Act
  const cjs = require('@shuozeli/pwright');
  // Assert
  assert.equal(cjs.chromium, api.chromium);
  assert.equal(cjs.errors.TimeoutError, api.errors.TimeoutError);
  assert.equal(cjs.chromium.name(), 'chromium');
  assert.equal(cjs.compatibility.apiCoverage, 'experimental-subset');
  assert.equal(Object.keys(require.cache).some(path => path.endsWith('/pwright.node')), false,
    'import alone must not load the native runtime');
});

test('unsupported async APIs reject without throwing synchronously', async () => {
  // Arrange
  let result;
  // Act
  assert.doesNotThrow(() => { result = api.chromium.launch(); });
  // Assert
  await assert.rejects(result, api.errors.UnsupportedOperationError);
});

test('unsupported options fail before loading native code or connecting', async () => {
  // Arrange
  const options = { headers: { 'X-Test': 'unused' } };
  // Act
  const result = api.chromium.connectOverCDP('http://127.0.0.1:9222', options);
  // Assert
  await assert.rejects(result, { code: 'PWRIGHT_UNSUPPORTED' });
});

test('invalid timeouts are not truncated by the FFI integer conversion', async () => {
  // Arrange
  const timeout = 2 ** 32;
  // Act
  const result = api.chromium.connectOverCDP('http://127.0.0.1:9222', { timeout });
  // Assert
  await assert.rejects(result, TypeError);
});
