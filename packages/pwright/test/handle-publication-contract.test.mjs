import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const handleApi = createRequire(import.meta.url)('../src/handles.cjs');

function api(mapError = error => error) {
  return handleApi({ mapError, UnsupportedOperationError: Error });
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((done, failed) => { resolve = done; reject = failed; });
  return { promise, resolve, reject };
}

function failingNative(phase, primary) {
  return {
    isElement() {
      if (phase === 'classification') throw primary;
      return false;
    },
    bindWrapper(wrapper) {
      if (phase === 'binding') throw primary;
      if (phase === 'marker') Object.preventExtensions(wrapper);
    },
  };
}

test('ElementHandle ownerFrame forwards only its native receiver and returns the canonical wrapper', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const frame = { identity: 'canonical Frame' };
  const calls = [];
  const session = { async handleOwnerFrame(value) { calls.push(['native', value]); return '42'; } };
  const handle = await api().create(session, native, id => { calls.push(['resolve', id]); return frame; });

  // Act: surplus arguments are ignored, not inspected or forwarded.
  const ignored = { get selector() { throw new Error('surplus argument inspected'); } };
  const owner = await handle.ownerFrame(ignored);

  // Assert
  assert.equal(owner, frame);
  assert.deepEqual(calls, [['native', native], ['resolve', '42']]);
});

test('ElementHandle ownerFrame returns native null without resolver lookup or extra native calls', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async handleOwnerFrame(value) { calls.push(value); return null; } };
  const handle = await api().create(session, native, () => { throw new Error('null must not resolve a Frame'); });

  // Act
  const owner = await handle.ownerFrame();

  // Assert
  assert.equal(owner, null);
  assert.deepEqual(calls, [native]);
});

test('ElementHandle ownerFrame exposes missing resolver wiring instead of silently returning null', async () => {
  // Arrange: only legacy unit fixtures may omit the production resolver.
  const native = { isElement: () => true, bindWrapper() {} };
  let nativeCalls = 0;
  const session = { async handleOwnerFrame() { nativeCalls++; return '42'; } };
  const handle = await api().create(session, native);

  // Act
  const result = handle.ownerFrame();

  // Assert
  await assert.rejects(result, /ElementHandle owner Frame resolver is missing/);
  assert.equal(nativeCalls, 0);
});

for (const producer of ['evaluateHandle', 'getProperty', 'getProperties']) {
  test(`${producer} forwards the source resolver to ElementHandle descendants`, async () => {
    // Arrange
    const source = { isElement: () => false, bindWrapper() {} };
    const child = { isElement: () => true, bindWrapper() {} };
    const frame = { identity: 'existing child Frame' };
    const calls = [];
    const session = {
      async handleEvaluateHandle(value) { assert.equal(value, source); return child; },
      async handleGetProperty(value) { assert.equal(value, source); return child; },
      async handleGetProperties(value) { assert.equal(value, source); return [{ name: 'node', handle: child }]; },
      async handleOwnerFrame(value) { calls.push(['native', value]); return '84'; },
    };
    const wrapper = await api().create(session, source, id => { calls.push(['resolve', id]); return frame; });
    const descendant = producer === 'evaluateHandle' ? await wrapper.evaluateHandle(value => value.node)
      : producer === 'getProperty' ? await wrapper.getProperty('node') : (await wrapper.getProperties()).get('node');

    // Act
    const owner = await descendant.ownerFrame();

    // Assert
    assert.equal(typeof wrapper.ownerFrame, 'undefined');
    assert.equal(descendant.asElement(), descendant);
    assert.equal(owner, frame);
    assert.deepEqual(calls, [['native', child], ['resolve', '84']]);
  });
}

for (const rejection of ['synchronous', 'asynchronous']) {
  test(`ElementHandle ownerFrame maps ${rejection} native rejection without disposing its source`, async () => {
    // Arrange
    const native = { isElement: () => true, bindWrapper() {} };
    const primary = new Error('native owner lookup failed');
    const mapped = new Error('mapped owner lookup failure');
    const mappings = [];
    const session = { handleOwnerFrame(value) {
      assert.equal(value, native);
      if (rejection === 'synchronous') throw primary;
      return Promise.reject(primary);
    } };
    const handle = await api((error, name) => { mappings.push({ error, name }); return mapped; })
      .create(session, native, () => { throw new Error('failed native lookup must not resolve a Frame'); });

    // Act
    const result = handle.ownerFrame();

    // Assert
    await assert.rejects(result, error => error === mapped);
    assert.deepEqual(mappings, [{ error: primary, name: 'elementHandle.ownerFrame' }]);
  });
}

for (const phase of ['classification', 'binding', 'marker']) {
  test(`wrapper ${phase} failure requests disposal immediately and awaits its acknowledgement`, async () => {
    // Arrange
    const primary = new Error(`${phase} failed`);
    const native = failingNative(phase, primary);
    const completion = deferred();
    const disposals = [];
    const session = { handleDispose(value) { disposals.push(value); return completion.promise; } };
    let settled = false;

    // Act
    const result = api().create(session, native);
    const observed = result.then(() => { settled = true; }, error => { settled = true; return error; });

    // Assert: invoking the async factory must already have requested cleanup.
    assert.deepEqual(disposals, [native]);
    await Promise.resolve();
    assert.equal(settled, false);
    completion.resolve();
    const error = await observed;
    if (phase === 'marker') assert.ok(error instanceof TypeError);
    else assert.equal(error, primary);
    assert.equal(settled, true);
    assert.equal(disposals.length, 1);
  });
}

for (const rejection of ['synchronous', 'asynchronous']) {
  test(`wrapper failure preserves both publication and ${rejection} disposal failures`, async () => {
    // Arrange
    const primary = new Error('binding failed');
    const cleanup = new Error('cleanup failed');
    const native = failingNative('binding', primary);
    const disposals = [];
    const session = {
      handleDispose(value) {
        disposals.push(value);
        if (rejection === 'synchronous') throw cleanup;
        return Promise.reject(cleanup);
      },
    };

    // Act
    const result = api().create(session, native);

    // Assert
    await assert.rejects(result, error => {
      assert.ok(error instanceof AggregateError);
      assert.equal(error.cause, primary);
      assert.deepEqual(error.errors, [primary, cleanup]);
      return true;
    });
    assert.deepEqual(disposals, [native]);
  });
}

for (const element of [false, true]) {
  test(`successful ${element ? 'ElementHandle' : 'JSHandle'} wrapping retains the native owner`, async () => {
    // Arrange
    const wrappers = [];
    const native = { isElement: () => element, bindWrapper: wrapper => wrappers.push(wrapper) };
    const disposals = [];
    const session = { handleDispose: value => disposals.push(value) };

    // Act
    const wrapper = await api().create(session, native);

    // Assert
    assert.deepEqual(wrappers, [wrapper]);
    assert.equal(wrapper.asElement(), element ? wrapper : null);
    assert.equal(Object.prototype.toString.call(wrapper), '[object PwrightJSHandle]');
    assert.deepEqual(disposals, []);
  });
}

for (const producer of ['evaluateHandle', 'getProperty', 'getProperties']) {
  test(`${producer} maps a child publication failure only after child cleanup completes`, async () => {
    // Arrange
    const primary = new Error('child publication failed');
    const mapped = new Error('mapped publication failure');
    const completion = deferred();
    const target = { isElement: () => false, bindWrapper() {} };
    const child = failingNative('binding', primary);
    const disposals = [];
    const mappings = [];
    const session = {
      async handleEvaluateHandle(value) { assert.equal(value, target); return child; },
      async handleGetProperty(value) { assert.equal(value, target); return child; },
      async handleGetProperties(value) { assert.equal(value, target); return [{ name: 'child', handle: child }]; },
      handleDispose(value) { disposals.push(value); return completion.promise; },
    };
    const wrapper = await api((error, name) => { mappings.push({ error, name }); return mapped; }).create(session, target);

    // Act
    const result = producer === 'evaluateHandle'
      ? wrapper.evaluateHandle(value => value)
      : producer === 'getProperty' ? wrapper.getProperty('child') : wrapper.getProperties();
    const observed = result.catch(error => error);

    // Assert: producer has delivered the child; cleanup, not GC, owns release.
    await Promise.resolve();
    assert.deepEqual(disposals, [child]);
    assert.deepEqual(mappings, []);
    completion.resolve();
    assert.equal(await observed, mapped);
    assert.deepEqual(mappings, [{ error: primary, name: `jsHandle.${producer}` }]);
    assert.deepEqual(disposals, [child]);
  });
}

function batchNative(element = false) {
  const state = { classifications: 0, wrappers: [] };
  return {
    state,
    isElement() { state.classifications++; return element; },
    bindWrapper(wrapper) { state.wrappers.push(wrapper); },
  };
}

test('empty native property snapshot publishes an empty Map without disposal', async () => {
  // Arrange
  const disposals = [];
  const session = { handleDispose: value => disposals.push(value) };
  // Act
  const properties = await api().createBatch(session, []);
  // Assert
  assert.ok(properties instanceof Map);
  assert.equal(properties.size, 0);
  assert.deepEqual(disposals, []);
});

test('batch publication preserves Rust order, classifies once, and owns aliases independently', async () => {
  // Arrange
  const target = batchNative();
  const children = [batchNative(), batchNative(true), batchNative(), batchNative(), batchNative()];
  const names = ['10', '2', '__proto__', 'then', ''];
  const entries = children.map((handle, index) => ({ name: names[index], handle }));
  const sharedValue = { answer: 42 };
  const disposals = [];
  const calls = [];
  const session = {
    handleGetProperties(value) { calls.push(value); return entries; },
    handleJsonValue(value) { assert.ok(!disposals.includes(value)); return sharedValue; },
    handleDispose(value) { disposals.push(value); },
  };
  const wrapper = await api().create(session, target);
  // Act
  const properties = await wrapper.getProperties();
  // Assert
  assert.deepEqual(calls, [target]);
  assert.deepEqual([...properties.keys()], names);
  assert.deepEqual(disposals, []);
  for (const [index, handle] of children.entries()) {
    const result = properties.get(names[index]);
    assert.equal(handle.state.classifications, 1);
    assert.deepEqual(handle.state.wrappers, [result]);
    assert.equal(result.asElement(), index === 1 ? result : null);
    assert.equal(Object.prototype.toString.call(result), '[object PwrightJSHandle]');
  }
  const first = properties.get('10');
  const alias = properties.get('__proto__');
  assert.notEqual(first, alias);
  // Independent owner release is a subsequent lifecycle observation.
  await first.dispose();
  assert.equal(await alias.jsonValue(), sharedValue);
  assert.deepEqual(disposals, [children[0]]);
  await alias.dispose();
  assert.deepEqual(disposals, [children[0], children[2]]);
});

for (const phase of ['classification', 'binding', 'marker']) {
  test(`batch ${phase} failure releases previous, failing and unvisited owners exactly once`, async () => {
    // Arrange
    const primary = new Error(`${phase} failed`);
    const before = batchNative();
    const failing = failingNative(phase, primary);
    const after = batchNative(true);
    const entries = [before, failing, after].map((handle, index) => ({ name: String(index), handle }));
    const disposals = [];
    const session = { handleDispose(value) { disposals.push(value); } };
    // Act
    const observed = await api().createBatch(session, entries).catch(error => error);
    // Assert
    if (phase === 'marker') assert.ok(observed instanceof TypeError);
    else assert.equal(observed, primary);
    assert.deepEqual(disposals, [before, failing, after]);
    assert.equal(before.state.wrappers.length, 1);
    assert.equal(after.state.classifications, 0);
    assert.equal(after.state.wrappers.length, 0);
  });
}

for (const phase of ['construction', 'first insertion', 'later insertion']) {
  test(`Map ${phase} failure compensates the complete native batch`, async () => {
    // Arrange
    const primary = new Error(`Map ${phase} failed`);
    const children = [batchNative(), batchNative(true), batchNative()];
    const entries = children.map((handle, index) => ({ name: String(index), handle }));
    const disposals = [];
    const session = { handleDispose(value) { disposals.push(value); } };
    const factory = api();
    const OriginalMap = globalThis.Map;
    let insertions = 0;
    class FailingMap extends OriginalMap {
      constructor() {
        if (phase === 'construction') throw primary;
        super();
      }
      set(name, value) {
        insertions++;
        if (insertions === (phase === 'first insertion' ? 1 : 2)) throw primary;
        return super.set(name, value);
      }
    }
    // Act: restore the global synchronously before awaiting any cleanup work.
    let result;
    try {
      globalThis.Map = FailingMap;
      result = factory.createBatch(session, entries);
    } finally { globalThis.Map = OriginalMap; }
    const observed = await result.catch(error => error);
    // Assert
    assert.equal(observed, primary);
    assert.deepEqual(disposals, children);
    const visited = phase === 'construction' ? 0 : phase === 'first insertion' ? 1 : 2;
    assert.deepEqual(children.map(child => child.state.classifications), children.map((_child, index) => index < visited ? 1 : 0));
  });
}

test('batch rollback preserves every cleanup failure and still attempts subsequent owners', async () => {
  // Arrange
  const primary = new Error('classification failed');
  const synchronous = new Error('first synchronous cleanup failure');
  const asynchronous = new Error('second asynchronous cleanup failure');
  const children = [failingNative('classification', primary), batchNative(), batchNative()];
  const entries = children.map((handle, index) => ({ name: String(index), handle }));
  const disposals = [];
  const session = {
    handleDispose(value) {
      disposals.push(value);
      if (value === children[0]) throw synchronous;
      if (value === children[1]) return Promise.reject(asynchronous);
    },
  };
  // Act
  const observed = await api().createBatch(session, entries).catch(error => error);
  // Assert
  assert.ok(observed instanceof AggregateError);
  assert.equal(observed.cause, primary);
  assert.deepEqual(observed.errors, [primary, synchronous, asynchronous]);
  assert.deepEqual(disposals, children);
});

test('batch rollback admits disposal sequentially and awaits the last acknowledgement', async () => {
  // Arrange
  const primary = new Error('binding failed');
  const children = [failingNative('binding', primary), batchNative()];
  const entries = children.map((handle, index) => ({ name: String(index), handle }));
  const first = deferred();
  const last = deferred();
  const lastStarted = deferred();
  const disposals = [];
  const session = {
    handleDispose(value) {
      disposals.push(value);
      if (value === children[0]) return first.promise;
      lastStarted.resolve();
      return last.promise;
    },
  };
  let settled = false;
  // Act
  const observed = api().createBatch(session, entries).catch(error => { settled = true; return error; });
  // Assert
  assert.deepEqual(disposals, [children[0]]);
  await Promise.resolve();
  assert.equal(settled, false);
  first.resolve();
  await lastStarted.promise;
  assert.deepEqual(disposals, children);
  assert.equal(settled, false);
  last.resolve();
  assert.equal(await observed, primary);
  assert.equal(settled, true);
});

test('getProperties forwards only its native target without inspecting surplus arguments', async () => {
  // Arrange
  const target = batchNative();
  const calls = [];
  let reads = 0;
  const ignored = { get value() { reads++; throw new Error('ignored argument getter'); } };
  const session = { handleGetProperties(...args) { calls.push(args); return []; } };
  const wrapper = await api().create(session, target);
  // Act
  const properties = await wrapper.getProperties(ignored);
  // Assert
  assert.equal(properties.size, 0);
  assert.deepEqual(calls, [[target]]);
  assert.equal(reads, 0);
});

for (const rejection of ['synchronous', 'asynchronous']) {
  test(`getProperties maps ${rejection} native rejection without releasing its source`, async () => {
    // Arrange
    const primary = new Error('native target is disposed');
    const mapped = new Error('mapped target error');
    const target = batchNative();
    const disposals = [];
    const mappings = [];
    const session = {
      handleGetProperties(value) {
        assert.equal(value, target);
        if (rejection === 'synchronous') throw primary;
        return Promise.reject(primary);
      },
      handleDispose(value) { disposals.push(value); },
    };
    const wrapper = await api((error, name) => { mappings.push({ error, name }); return mapped; }).create(session, target);
    // Act
    const observed = await wrapper.getProperties().catch(error => error);
    // Assert
    assert.equal(observed, mapped);
    assert.deepEqual(mappings, [{ error: primary, name: 'jsHandle.getProperties' }]);
    assert.deepEqual(disposals, []);
  });
}
