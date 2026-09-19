// NODE179 offline carrier-projection kernel comparison only. The oracle calls
// actual Playwright 1.58.2 ElementHandle._setInputFiles and throws STOP at its
// first evaluateHandleInUtility boundary. The candidate calls the actual Rust
// diagnostic, not a fake production implementation. No marking witness, DOM,
// native ticket/session, root reclamation, native query or upload is exercised.
// Raw projected-reference/output-shape checks are labelled diagnostic contracts:
// the upstream stop boundary exposes flags, not all three local variables.
// Source: pinned playwright-core/lib/server/dom.js:550–570.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { projectionPorts } = require('../src/file-input-projection.cjs');
const { nativeArtifact, verifyNativeArtifact } = require('../src/native-artifact.cjs');
assert.ok(['retained-file-input', 'retained-file-input-combined', 'retained-target-marking',
  'retained-target-marking-combined', 'retained-target-prevalidation',
  'retained-target-prevalidation-combined', 'retained-input-instrumentation',
  'retained-input-instrumentation-combined', 'retained-call-argument-serialization',
  'retained-call-argument-serialization-combined', 'retained-serialized-payload-import',
  'retained-serialized-payload-import-combined', 'retained-serialized-payload-mutation',
  'retained-serialized-payload-mutation-combined'].includes(nativeArtifact.profile));
const native = require(nativeArtifact.path);
verifyNativeArtifact(native, nativeArtifact);
assert.equal(typeof native.inspectFileInputProjection, 'function', 'rebuilt projection diagnostic is required');

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { ElementHandle } = requireCore(join(core, 'lib/server/dom.js'));
const limits = { timeout: 5000 };

function begin(kind, carrier) {
  const state = { failed: false, error: undefined, flags: undefined, diagnostic: undefined, evaluations: 0 };
  if (kind === 'candidate') {
    try {
      const value = native.inspectFileInputProjection(carrier, projectionPorts);
      state.diagnostic = value;
      state.flags = { rawMultiple: value.rawMultiple, multiple: value.multiple, directoryUpload: value.directoryUpload };
    } catch (error) { state.failed = true; state.error = error; }
    state.settled = Promise.resolve();
    return state;
  }
  const stop = new Error('STOP: native precheck boundary; never upload');
  const receiver = {
    evaluateHandleInUtility(callback, flags) {
      assert.equal(typeof callback, 'function');
      state.evaluations++;
      state.flags = { rawMultiple: flags.multiple, multiple: !!flags.multiple, directoryUpload: flags.directoryUpload };
      throw stop;
    },
    instrumentation: { onBeforeInputAction() { assert.fail('must stop before instrumentation'); } },
  };
  const pending = Reflect.apply(ElementHandle.prototype._setInputFiles, receiver,
    [{ metadata: {}, race: value => Promise.resolve(value) }, carrier]);
  // Attach immediately, including for getter failures. Normalizing the async
  // oracle failure does not claim identical public sync/async API surfaces.
  state.settled = pending.then(() => assert.fail('oracle must stop before upload'), error => {
    if (error !== stop) { state.failed = true; state.error = error; }
  });
  return state;
}
async function finish(state) {
  await state.settled;
  if (state.failed) throw state.error;
  return state.flags;
}
async function project(kind, carrier) { return finish(begin(kind, carrier)); }
async function failure(kind, carrier) {
  const state = begin(kind, carrier);
  await state.settled;
  assert.equal(state.failed, true, 'projection must fail');
  assert.equal(state.evaluations, 0, 'failure must precede the native boundary');
  return state.error;
}
function paired(name, exercise) {
  test('offline projection kernel: ' + name, limits, async () => {
    // Arrange / Act: each real implementation receives fresh language values.
    const expected = await exercise('oracle');
    const actual = await exercise('candidate');
    // Assert
    assert.deepEqual(actual, expected);
  });
}
function errorShape(error) {
  return { name: error.name, constructor: error.constructor.name, message: error.message };
}

test('projection port contract: frozen primitives contain no traversal or native policy', limits, () => {
  // Arrange / Act
  const names = Object.keys(projectionPorts);
  // Assert
  assert.equal(Object.isFrozen(projectionPorts), true);
  assert.deepEqual(names, ['read', 'greater_than', 'destructure_error']);
  for (const name of names) assert.equal(typeof projectionPorts[name], 'function');
});

paired('all three carrier reads precede both multiplicity reads', async kind => {
  // Arrange
  const trace = [];
  const carrier = {
    get filePayloads() { trace.push('filePayloads'); return { get length() { trace.push('payloads.length'); return 0; } }; },
    get localPaths() { trace.push('localPaths'); return { get length() { trace.push('paths.length'); return 0; } }; },
    get localDirectory() { trace.push('localDirectory'); return ''; },
  };
  // Act
  const flags = await project(kind, carrier);
  // Assert
  assert.deepEqual(trace, ['filePayloads', 'localPaths', 'localDirectory', 'payloads.length', 'paths.length']);
  assert.deepEqual(flags, { rawMultiple: false, multiple: false, directoryUpload: false });
  return { trace, flags };
});

for (const [label, paths] of [
  ['undefined', undefined], ['null', null], ['false', false], ['zero', 0],
  ['negative zero', -0], ['NaN', NaN], ['zero bigint', 0n], ['empty string', ''],
]) {
  paired('raw multiplicity preserves falsy paths: ' + label, async kind => {
    // Arrange
    const carrier = { filePayloads: [], localPaths: paths };
    // Act
    const flags = await project(kind, carrier);
    // Assert
    assert.equal(Object.is(flags.rawMultiple, paths), true);
    assert.equal(flags.multiple, false);
    assert.equal(flags.directoryUpload, false);
    return flags;
  });
}

for (const [label, value] of [
  ['undefined', undefined], ['null', null], ['false', false], ['zero', 0],
  ['negative zero', -0], ['NaN', NaN], ['zero bigint', 0n], ['empty string', ''],
]) {
  paired('falsy payloads skip length and allow paths: ' + label, async kind => {
    // Arrange
    let reads = 0;
    const carrier = { filePayloads: value, localPaths: { get length() { reads++; return 2; } } };
    // Act
    const flags = await project(kind, carrier);
    // Assert
    assert.equal(reads, 1);
    assert.deepEqual(flags, { rawMultiple: true, multiple: true, directoryUpload: false });
    return { reads, flags };
  });
}

for (const [label, length, expected] of [
  ['number two', 2, true], ['number one', 1, false], ['bigint two', 2n, true],
  ['bigint one', 1n, false], ['numeric string', '2', true], ['nonnumeric string', 'x', false],
  ['NaN', NaN, false], ['infinity', Infinity, true],
]) {
  paired('relational coercion retains JS semantics: ' + label, async kind => {
    // Arrange
    const carrier = { filePayloads: { length }, localPaths: [] };
    // Act
    const flags = await project(kind, carrier);
    // Assert
    assert.equal(flags.rawMultiple, expected);
    return flags;
  });
}

paired('number-hint conversion runs once after all carrier getters', async kind => {
  // Arrange
  const trace = [];
  const length = { [Symbol.toPrimitive](hint) { trace.push('coerce:' + hint); return '2'; } };
  const carrier = {
    get filePayloads() { trace.push('payloads'); return { get length() { trace.push('length'); return length; } }; },
    get localPaths() { trace.push('paths'); return undefined; },
    get localDirectory() { trace.push('directory'); return false; },
  };
  // Act
  const flags = await project(kind, carrier);
  // Assert
  assert.deepEqual(trace, ['payloads', 'paths', 'directory', 'length', 'coerce:number']);
  assert.equal(flags.multiple, true);
  return { trace, flags };
});

paired('valueOf falls back to toString for length conversion', async kind => {
  // Arrange
  const trace = [];
  const length = { valueOf() { trace.push('valueOf'); return {}; }, toString() { trace.push('toString'); return '2'; } };
  // Act
  const flags = await project(kind, { filePayloads: { length } });
  // Assert
  assert.deepEqual(trace, ['valueOf', 'toString']);
  return { trace, flags };
});

paired('Symbol length rejects without normalizing the thrown TypeError', async kind => {
  // Arrange / Act
  const error = await failure(kind, { filePayloads: { length: Symbol('length') } });
  // Assert
  assert.ok(error instanceof TypeError);
  return errorShape(error);
});

paired('length conversion returning an object rejects', async kind => {
  // Arrange / Act
  const error = await failure(kind, { filePayloads: { length: { [Symbol.toPrimitive]() { return {}; } } } });
  // Assert
  assert.ok(error instanceof TypeError);
  return errorShape(error);
});

paired('true payload multiplicity skips the captured paths length', async kind => {
  // Arrange
  const trace = [];
  const carrier = { filePayloads: { length: 2 },
    get localPaths() { trace.push('paths'); return { get length() { assert.fail('short-circuited'); } }; },
    get localDirectory() { trace.push('directory'); return {}; },
  };
  // Act
  const flags = await project(kind, carrier);
  // Assert
  assert.deepEqual(trace, ['paths', 'directory']);
  assert.deepEqual(flags, { rawMultiple: true, multiple: true, directoryUpload: true });
  return { trace, flags };
});

paired('directory getter mutation affects captured objects but does not reread carrier fields', async kind => {
  // Arrange
  const trace = [];
  const original = { length: 0 }, replacement = { get length() { assert.fail('must not reread payloads'); } };
  const carrier = {
    get filePayloads() { trace.push('payloads'); return original; },
    get localPaths() { trace.push('paths'); return undefined; },
    get localDirectory() {
      trace.push('directory'); original.length = 2;
      Object.defineProperty(carrier, 'filePayloads', { value: replacement });
      return '';
    },
  };
  // Act
  const state = begin(kind, carrier);
  const flags = await finish(state);
  // Assert: upstream trace/flags prove the capture; diagnostic additionally exposes identity.
  assert.deepEqual(trace, ['payloads', 'paths', 'directory']);
  assert.equal(flags.multiple, true);
  if (kind === 'candidate') assert.equal(state.diagnostic.filePayloads, original);
  return { trace, flags };
});

for (const [index, key] of ['filePayloads', 'localPaths', 'localDirectory'].entries()) {
  paired('carrier getter error stops exactly at ' + key, async kind => {
    // Arrange
    const trace = [], sentinel = new Error('getter:' + key);
    const carrier = {};
    for (const field of ['filePayloads', 'localPaths', 'localDirectory'])
      Object.defineProperty(carrier, field, { get() { trace.push(field); if (field === key) throw sentinel; return []; } });
    // Act
    const error = await failure(kind, carrier);
    // Assert
    assert.equal(error, sentinel);
    assert.deepEqual(trace, ['filePayloads', 'localPaths', 'localDirectory'].slice(0, index + 1));
    return trace;
  });
}

paired('length getter failure follows all three carrier reads', async kind => {
  // Arrange
  const trace = [], sentinel = new Error('length');
  const carrier = {
    get filePayloads() { trace.push('payloads'); return { get length() { trace.push('length'); throw sentinel; } }; },
    get localPaths() { trace.push('paths'); return []; },
    get localDirectory() { trace.push('directory'); return ''; },
  };
  // Act
  const error = await failure(kind, carrier);
  // Assert
  assert.equal(error, sentinel);
  assert.deepEqual(trace, ['payloads', 'paths', 'directory', 'length']);
  return trace;
});

paired('relational coercion preserves arbitrary thrown-value identity', async kind => {
  // Arrange
  const sentinel = { thrown: 'not an Error' };
  // Act
  const error = await failure(kind, { filePayloads: { length: { [Symbol.toPrimitive]() { throw sentinel; } } } });
  // Assert
  assert.equal(error, sentinel);
  return error;
});

for (const carrier of [null, undefined]) {
  paired('nullish carrier preserves literal destructuring error: ' + String(carrier), async kind => {
    // Arrange / Act
    const error = await failure(kind, carrier);
    // Assert: compare exact pinned message, not only the error family.
    assert.ok(error instanceof TypeError);
    return errorShape(error);
  });
}

paired('inherited and proxy carrier reads preserve their original receiver', async kind => {
  // Arrange
  const trace = [];
  let carrier;
  const inherited = Object.create(null, { filePayloads: { get() {
    assert.equal(this, carrier); trace.push('inherited'); return [];
  } } });
  carrier = new Proxy(Object.create(inherited), { get(target, key, receiver) {
    trace.push('get:' + String(key)); return Reflect.get(target, key, receiver);
  } });
  // Act
  const flags = await project(kind, carrier);
  // Assert
  assert.deepEqual(trace, ['get:filePayloads', 'inherited', 'get:localPaths', 'get:localDirectory']);
  return { trace, flags };
});

paired('carrier and captured fields are never awaited or inspected for then', async kind => {
  // Arrange
  let thenReads = 0;
  const noThen = value => Object.defineProperty(value, 'then', { get() { thenReads++; throw new Error('unexpected assimilation'); } });
  const payloads = noThen({ length: 0 }), paths = noThen({ length: 0 }), directory = noThen({});
  const carrier = noThen({ filePayloads: payloads, localPaths: paths, localDirectory: directory });
  // Act
  const flags = await project(kind, carrier);
  // Assert
  assert.equal(thenReads, 0);
  assert.deepEqual(flags, { rawMultiple: false, multiple: false, directoryUpload: true });
  return { thenReads, flags };
});

paired('projection never indexes, maps, serializes or copies payload bytes', async kind => {
  // Arrange
  const bytes = Buffer.from([1]);
  bytes.toString = () => assert.fail('Buffer serialization is a separate phase');
  const files = [bytes];
  Object.defineProperty(files, 0, { get() { assert.fail('payload index must remain deferred'); } });
  Object.defineProperty(files, 'map', { get() { assert.fail('payload map must remain deferred'); } });
  // Act
  const flags = await project(kind, { filePayloads: files });
  // Assert
  assert.equal(bytes[0], 1);
  return flags;
});

paired('ToBoolean never calls replaced global Boolean or object coercion', async kind => {
  // Arrange
  const original = globalThis.Boolean;
  const directory = { [Symbol.toPrimitive]() { assert.fail('truthiness does not coerce objects'); } };
  let state;
  try {
    globalThis.Boolean = () => assert.fail('ToBoolean is not global Boolean');
    // Act: restore globals before awaiting either implementation.
    state = begin(kind, { filePayloads: [], localPaths: 0, localDirectory: directory });
  } finally { globalThis.Boolean = original; }
  const flags = await finish(state);
  // Assert
  assert.deepEqual(flags, { rawMultiple: 0, multiple: false, directoryUpload: true });
  return flags;
});

paired('projection finishes synchronously before a getter-scheduled mutation microtask', async kind => {
  // Arrange
  const trace = [], payloads = { length: 0 };
  const carrier = {
    get filePayloads() { trace.push('payloads'); queueMicrotask(() => { trace.push('microtask'); payloads.length = 2; }); return payloads; },
    get localPaths() { trace.push('paths'); return []; },
    get localDirectory() { trace.push('directory'); return undefined; },
  };
  // Act
  const state = begin(kind, carrier);
  trace.push('caller');
  const immediate = [...trace];
  const flags = await finish(state);
  // Assert
  assert.deepEqual(immediate, ['payloads', 'paths', 'directory', 'caller']);
  assert.deepEqual(trace, [...immediate, 'microtask']);
  assert.equal(flags.multiple, false);
  return { trace, flags };
});

paired('getter reentry runs an independent stateless projection without corrupting outer values', async kind => {
  // Arrange
  const trace = [];
  let inner;
  const carrier = {
    get filePayloads() {
      trace.push('outer:get');
      inner = begin(kind, { filePayloads: { length: 2 }, localDirectory: true });
      trace.push('inner:returned');
      assert.deepEqual(inner.flags, { rawMultiple: true, multiple: true, directoryUpload: true });
      return [];
    },
    localPaths: null,
  };
  // Act
  const outer = begin(kind, carrier);
  const [outerFlags, innerFlags] = await Promise.all([finish(outer), finish(inner)]);
  // Assert: no ticket, registry exclusivity or reclamation claim is made here.
  assert.deepEqual(trace, ['outer:get', 'inner:returned']);
  assert.deepEqual(outerFlags, { rawMultiple: null, multiple: false, directoryUpload: false });
  return { trace, outerFlags, innerFlags };
});

test('projection diagnostic contract: exact raw field identities and own data properties', limits, () => {
  // Arrange
  const filePayloads = [], localPaths = { length: 0 }, localDirectory = {};
  const expectedKeys = ['filePayloads', 'localPaths', 'localDirectory', 'rawMultiple', 'multiple', 'directoryUpload'];
  // Act
  const result = native.inspectFileInputProjection({ filePayloads, localPaths, localDirectory }, projectionPorts);
  // Assert: native diagnostic output, not an upstream public return object.
  assert.deepEqual(Object.keys(result), expectedKeys);
  assert.equal(result.filePayloads, filePayloads);
  assert.equal(result.localPaths, localPaths);
  assert.equal(result.localDirectory, localDirectory);
  for (const key of expectedKeys) assert.equal(Object.hasOwn(Object.getOwnPropertyDescriptor(result, key), 'value'), true);
});

test('projection diagnostic contract: inherited output setters are bypassed', limits, () => {
  // Arrange
  const keys = ['filePayloads', 'localPaths', 'localDirectory', 'rawMultiple', 'multiple', 'directoryUpload'];
  const originals = keys.map(key => Object.getOwnPropertyDescriptor(Object.prototype, key));
  const carrier = { filePayloads: [], localPaths: 0, localDirectory: false };
  let result;
  try {
    for (const key of keys) Object.defineProperty(Object.prototype, key, {
      configurable: true, set() { assert.fail('inherited output setter: ' + key); },
    });
    // Act
    result = native.inspectFileInputProjection(carrier, projectionPorts);
  } finally {
    for (const [index, key] of keys.entries()) {
      if (originals[index]) Object.defineProperty(Object.prototype, key, originals[index]);
      else delete Object.prototype[key];
    }
  }
  // Assert
  assert.deepEqual(Object.keys(result), keys);
  assert.equal(result.rawMultiple, 0);
});
