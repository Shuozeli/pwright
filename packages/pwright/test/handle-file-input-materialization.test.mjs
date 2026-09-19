// NODE176 offline language-kernel evidence only. The direct-helper cases call
// pinned Playwright 1.58.2 prepareFilesForUpload and the selected Rust diagnostic.
// They deliberately admit values that a public channel might reject earlier.
// Separate deferred cases run client schema -> setImmediate -> server schema ->
// helper. Neither fixture creates a native ticket/session, sends a protocol
// command, implements error transport, touches a file, or claims upload success.
// Sources: lib/server/fileUploadUtils.js:44–78; protocol/validator.js:2197–2207;
// server/dispatchers/dispatcher.js:196–213,259–269; inProcessFactory.js:46–47.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { channelPorts, serverValidationPorts } = require('../src/file-input-channel.cjs');
const { materializationPorts, asyncMaterialize } = require('../src/file-input-materialization.cjs');
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
for (const name of ['inspectFileInputChannel', 'inspectFileInputServerChannel', 'inspectFileInputMaterialization'])
  assert.equal(typeof native[name], 'function', name);

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const load = path => requireCore(join(core, 'lib', path));
const { findValidator } = load('protocol/validator.js');
const { ChannelOwner } = load('client/channelOwner.js');
const { DispatcherConnection } = load('server/dispatchers/dispatcher.js');
const { prepareFilesForUpload } = load('server/fileUploadUtils.js');
const oracleValidator = findValidator('ElementHandle', 'setInputFiles', 'Params');
const clientContext = Reflect.apply(ChannelOwner.prototype._validatorToWireContext, {
  _connection: { rawBuffers: () => true }, _platform: { isUnderTest: () => false },
}, []);
// A real empty server registry, not a forged WritableStream authority. Override
// only the ambient test flag so optional __testHook fields stay out of scope.
const serverConnection = new DispatcherConnection(true);
const serverContext = { ...serverConnection._validatorFromWireContext(), isUnderTest: () => false };
const serverFrame = { _page: { browserContext: { _browser: { _isCollocatedWithServer: false } } } };

function payload(buffer = Buffer.from([1, 2])) {
  return { name: 'bytes.bin', mimeType: 'application/octet-stream', buffer };
}
function validateClient(kind, params) {
  return kind === 'oracle' ? oracleValidator(params, '', clientContext)
    : native.inspectFileInputChannel(params, channelPorts);
}
function validateServer(kind, params) {
  return kind === 'oracle' ? oracleValidator(params, '', serverContext)
    : native.inspectFileInputServerChannel(params, serverValidationPorts);
}
function materialize(kind, params) {
  // Actual upstream helper is async even on its synchronous Buffer branch.
  // Returning the diagnostic's raw carrier from this thin async adapter models
  // OUTER return assimilation only, never awaiting nested serializer/map values.
  return kind === 'oracle' ? prepareFilesForUpload(serverFrame, params) : asyncMaterialize({
    // Stateless adapter only; this is not a NativeFileInputPreparation.
    beginMaterialization(ports) { return native.inspectFileInputMaterialization(params, ports); },
  });
}
function paired(group, name, exercise) {
  test('offline ' + group + ': ' + name, { timeout: 5000 }, async () => {
    // Arrange / Act: independent language values for each real implementation.
    const expected = await exercise('oracle');
    const actual = await exercise('candidate');
    // Assert
    assert.deepEqual(actual, expected);
  });
}
const direct = (name, exercise) => paired('direct materialization helper', name, exercise);
const server = (name, exercise) => paired('server-direction schema', name, exercise);
const scheduled = (name, exercise) => paired('deferred two-validator seam', name, exercise);
function captureSync(action) {
  try { return { value: action() }; } catch (error) { return { error }; }
}
async function capture(action) {
  try { return { value: await action() }; } catch (error) { return { error }; }
}
function errorShape(error) {
  assert.ok(error instanceof Error);
  return { constructor: error.constructor.name, name: error.name, message: error.message };
}
function ownShape(object) {
  return Object.keys(object).map(key => {
    const descriptor = Object.getOwnPropertyDescriptor(object, key);
    return { key, enumerable: descriptor.enumerable, configurable: descriptor.configurable,
      writable: descriptor.writable, data: Object.hasOwn(descriptor, 'value') };
  });
}

direct('root getter order and per-payload reads are distinct from channel order', async kind => {
  // Arrange
  const trace = [];
  const bytes = Buffer.from([1]);
  bytes.toString = function (encoding) {
    assert.equal(this, bytes); trace.push('serialize:' + encoding); return 'AQ==';
  };
  const item = new Proxy({ ...payload(bytes), lastModifiedMs: 99 }, {
    get(target, key) { trace.push('item:' + String(key)); return Reflect.get(target, key); },
  });
  const params = new Proxy({ payloads: [item] }, {
    get(target, key) { trace.push('params:' + String(key)); return Reflect.get(target, key); },
  });
  // Act
  const result = await materialize(kind, params);
  // Assert
  assert.deepEqual(trace, ['params:payloads', 'params:streams', 'params:directoryStream',
    'params:localPaths', 'params:localDirectory', 'item:name', 'item:mimeType',
    'item:buffer', 'serialize:base64', 'item:lastModifiedMs']);
  return { trace, result };
});

direct('exactly-one check uses an observable five-entry array filter with Boolean', async kind => {
  // Arrange
  const original = Array.prototype.filter;
  const trace = [];
  const values = [payload()];
  let pending;
  try {
    Array.prototype.filter = function (callback, ...rest) {
      trace.push({ length: this.length, first: this[0] === values,
        othersUndefined: this.slice(1).every(value => value === undefined),
        callbackIsBoolean: callback === Boolean, extraArguments: rest.length });
      return Reflect.apply(original, this, [callback, ...rest]);
    };
    // Act
    pending = materialize(kind, { payloads: values });
  } finally { Array.prototype.filter = original; }
  const result = await pending;
  // Assert
  assert.deepEqual(trace, [{ length: 5, first: true, othersUndefined: true, callbackIsBoolean: true, extraArguments: 0 }]);
  return { trace, payloadCount: result.filePayloads.length };
});

direct('selection literal order differs from getter order and uses the current Boolean callback', async kind => {
  // Arrange
  const original = globalThis.Boolean;
  const trace = [];
  const values = [payload()];
  const params = new Proxy({ payloads: values, localPaths: 0, localDirectory: '',
    streams: null, directoryStream: false }, {
    get(target, key) { trace.push('get:' + String(key)); return Reflect.get(target, key); },
  });
  let pending;
  try {
    globalThis.Boolean = function (value, index, array) {
      trace.push({ index, value: value === values ? 'payloads' : value, length: array.length,
        allOwn: [0, 1, 2, 3, 4].every(key => Object.hasOwn(array, key)) });
      return original(value);
    };
    // Act
    pending = materialize(kind, params);
  } finally { globalThis.Boolean = original; }
  const result = await pending;
  // Assert
  assert.deepEqual(trace.slice(0, 5), ['get:payloads', 'get:streams', 'get:directoryStream', 'get:localPaths', 'get:localDirectory']);
  assert.deepEqual(trace.slice(5).map(entry => entry.value), ['payloads', 0, '', null, false]);
  assert.ok(trace.slice(5).every(entry => entry.length === 5 && entry.allOwn));
  return { trace, localPaths: result.localPaths, localDirectory: result.localDirectory };
});

for (const lengthKind of ['number', 'boxed']) {
  direct('filter custom ' + lengthKind + ' length is compared strictly to one', async kind => {
    // Arrange
    const original = Array.prototype.filter;
    const trace = [];
    let pending, serialized = 0;
    const bytes = Buffer.from([1]); bytes.toString = () => { serialized++; return 'AQ=='; };
    try {
      Array.prototype.filter = function () {
        return { get length() { trace.push('length'); return lengthKind === 'number' ? 1 : new Number(1); } };
      };
      // Act
      pending = materialize(kind, { payloads: [payload(bytes)] });
    } finally { Array.prototype.filter = original; }
    const outcome = await capture(() => pending);
    // Assert
    assert.deepEqual(trace, ['length']);
    assert.equal(serialized, lengthKind === 'number' ? 1 : 0);
    return { trace, serialized, error: outcome.error && errorShape(outcome.error),
      value: outcome.value && outcome.value.filePayloads[0].buffer };
  });
}

direct('result and payload objects contain enumerable writable own data fields', async kind => {
  // Arrange / Act
  const result = await materialize(kind, { payloads: [payload()] });
  // Assert
  assert.equal(Object.getPrototypeOf(result), Object.prototype);
  assert.equal(Object.getPrototypeOf(result.filePayloads[0]), Object.prototype);
  assert.deepEqual(Object.keys(result), ['localPaths', 'localDirectory', 'filePayloads']);
  assert.deepEqual(Object.keys(result.filePayloads[0]), ['name', 'mimeType', 'buffer', 'lastModifiedMs']);
  assert.equal(Object.hasOwn(result.filePayloads[0], 'lastModifiedMs'), true);
  assert.equal(result.filePayloads[0].lastModifiedMs, undefined);
  return { outer: ownShape(result), item: ownShape(result.filePayloads[0]) };
});

direct('object-literal output bypasses inherited setters rather than ordinary assignment', async kind => {
  // Arrange
  const keys = ['localPaths', 'localDirectory', 'filePayloads', 'name', 'mimeType', 'buffer', 'lastModifiedMs'];
  const saved = keys.map(key => [key, Object.getOwnPropertyDescriptor(Object.prototype, key)]);
  let calls = 0;
  let pending;
  try {
    for (const key of keys) Object.defineProperty(Object.prototype, key, {
      configurable: true, set() { calls++; throw new Error('inherited output setter must not run'); },
    });
    // Act: both bodies execute synchronously before the adapter returns.
    pending = materialize(kind, { payloads: [payload()] });
  } finally {
    for (const [key, descriptor] of saved) {
      if (descriptor) Object.defineProperty(Object.prototype, key, descriptor);
      else delete Object.prototype[key];
    }
  }
  const result = await pending;
  // Assert
  assert.equal(calls, 0);
  return { calls, outer: ownShape(result), item: ownShape(result.filePayloads[0]) };
});

direct('custom serializer receives the original Buffer and exactly one base64 argument', async kind => {
  // Arrange
  const bytes = Buffer.from([1, 2]);
  const calls = [];
  bytes.toString = function (...args) { calls.push({ receiver: this === bytes, args }); return 'CQg='; };
  // Act
  const result = await materialize(kind, { payloads: [payload(bytes)] });
  // Assert
  assert.deepEqual(calls, [{ receiver: true, args: ['base64'] }]);
  assert.equal(result.filePayloads[0].buffer, 'CQg=');
  return { calls, source: [...bytes], serialized: result.filePayloads[0].buffer };
});

for (const output of ['undefined', 'number', 'object', 'promise']) {
  direct('raw ' + output + ' serializer return remains nested without type checking or await', async kind => {
    // Arrange
    let thenReads = 0;
    const raw = output === 'undefined' ? undefined : output === 'number' ? 17
      : output === 'promise' ? Promise.resolve('later') : {
        get then() { thenReads++; throw new Error('nested output must not be assimilated'); },
      };
    const bytes = Buffer.from([1]);
    bytes.toString = () => raw;
    // Act
    const result = await materialize(kind, { payloads: [payload(bytes)] });
    // Assert
    assert.equal(result.filePayloads[0].buffer, raw);
    assert.equal(Object.hasOwn(result.filePayloads[0], 'buffer'), true);
    assert.equal(thenReads, 0);
    return { identical: result.filePayloads[0].buffer === raw, thenReads, type: typeof raw };
  });
}

for (const stage of ['serializer getter', 'serializer call', 'lastModifiedMs getter']) {
  direct(stage + ' thrown Error keeps original identity without transport rewriting', async kind => {
    // Arrange
    const sentinel = new Error('original ' + stage);
    const bytes = Buffer.from([1]);
    const item = payload(bytes);
    const throwOriginal = () => { throw sentinel; };
    if (stage === 'serializer getter') Object.defineProperty(bytes, 'toString', { get: throwOriginal });
    if (stage === 'serializer call') bytes.toString = throwOriginal;
    if (stage === 'lastModifiedMs getter') Object.defineProperty(item, 'lastModifiedMs', { get: throwOriginal });
    // Act
    const outcome = await capture(() => materialize(kind, { payloads: [item] }));
    // Assert
    assert.equal(outcome.error, sentinel);
    return { identical: outcome.error === sentinel, message: sentinel.message };
  });
}

direct('per-item synchronous snapshots do not change when a later serializer mutates earlier bytes', async kind => {
  // Arrange
  const trace = [];
  const first = Buffer.from([1]), second = Buffer.from([2]);
  first.toString = function (encoding) { trace.push('first'); return Buffer.prototype.toString.call(this, encoding); };
  second.toString = function (encoding) {
    trace.push('second'); first[0] = 9; second[0] = 8;
    return Buffer.prototype.toString.call(this, encoding);
  };
  // Act
  const result = await materialize(kind, { payloads: [payload(first), payload(second)] });
  // Assert
  assert.deepEqual(result.filePayloads.map(item => item.buffer), ['AQ==', 'CA==']);
  return { trace, first: [...first], second: [...second], values: result.filePayloads.map(item => item.buffer) };
});

direct('serializer-queued microtasks run after every sibling snapshot', async kind => {
  // Arrange
  const trace = [];
  const first = Buffer.from([1]), second = Buffer.from([2]);
  first.toString = function (encoding) {
    trace.push('first'); queueMicrotask(() => { second[0] = 9; trace.push('microtask'); });
    return Buffer.prototype.toString.call(this, encoding);
  };
  second.toString = function (encoding) { trace.push('second'); return Buffer.prototype.toString.call(this, encoding); };
  // Act
  const result = await materialize(kind, { payloads: [payload(first), payload(second)] });
  // Assert
  assert.deepEqual(trace, ['first', 'second', 'microtask']);
  assert.equal(result.filePayloads[1].buffer, 'Ag==');
  return { trace, second: [...second], value: result.filePayloads[1].buffer };
});

direct('lastModifiedMs is read after serialization and preserves the resulting raw value', async kind => {
  // Arrange
  const trace = [];
  const raw = { get then() { throw new Error('nested modified value must not be assimilated'); } };
  let modified = 1;
  const bytes = Buffer.from([1]);
  bytes.toString = () => { trace.push('serialize'); modified = raw; return 'AQ=='; };
  const item = { ...payload(bytes), get lastModifiedMs() { trace.push('modified'); return modified; } };
  // Act
  const result = await materialize(kind, { payloads: [item] });
  // Assert
  assert.deepEqual(trace, ['serialize', 'modified']);
  assert.equal(result.filePayloads[0].lastModifiedMs, raw);
  return { trace, same: result.filePayloads[0].lastModifiedMs === raw };
});

for (const mimeType of [undefined, '', null, false, 0]) {
  direct('falsy MIME ' + String(mimeType) + ' re-reads name before inference', async kind => {
    // Arrange
    const trace = [];
    let names = 0;
    const item = { get name() { trace.push('name'); return ++names === 1 ? 'stored.bin' : 'derived.TXT'; },
      get mimeType() { trace.push('mimeType'); return mimeType; }, buffer: Buffer.from([1]) };
    // Act
    const result = await materialize(kind, { payloads: [item] });
    // Assert
    assert.equal(names, 2);
    assert.equal(result.filePayloads[0].name, 'stored.bin');
    return { trace, name: result.filePayloads[0].name, mime: result.filePayloads[0].mimeType };
  });
}

direct('truthy raw MIME is retained and prevents name coercion', async kind => {
  // Arrange
  const mimeType = { [Symbol.toPrimitive]() { throw new Error('raw MIME must not be coerced'); } };
  const name = { [Symbol.toPrimitive]() { throw new Error('explicit MIME prevents name coercion'); } };
  // Act
  const result = await materialize(kind, { payloads: [{ name, mimeType, buffer: Buffer.from([1]) }] });
  // Assert
  assert.equal(result.filePayloads[0].name, name);
  assert.equal(result.filePayloads[0].mimeType, mimeType);
  return { sameName: result.filePayloads[0].name === name, sameMime: result.filePayloads[0].mimeType === mimeType };
});

direct('MIME inference coerces a fallback name using the upstream language hint', async kind => {
  // Arrange
  const trace = [];
  const name = { [Symbol.toPrimitive](hint) { trace.push(hint); return 'folder/file.TXT'; } };
  // Act
  const result = await materialize(kind, { payloads: [{ name, buffer: Buffer.from([1]) }] });
  // Assert
  assert.equal(result.filePayloads[0].name, name);
  assert.equal(result.filePayloads[0].mimeType, 'text/plain');
  return { trace, mime: result.filePayloads[0].mimeType };
});

for (const name of ['file.json', 'file.html', 'file.no-known-node176-extension', '.gitignore', 'path/no-extension']) {
  direct('MIME inference or octet-stream fallback for ' + name, async kind => {
    // Arrange / Act
    const result = await materialize(kind, { payloads: [{ name, buffer: Buffer.from([1]) }] });
    // Assert: expected MIME is produced by the pinned implementation, not a
    // separately reconstructed extension table in the test.
    return result.filePayloads[0].mimeType;
  });
}

direct('MIME name coercion errors precede the Buffer serializer', async kind => {
  // Arrange
  const sentinel = new Error('name coercion');
  let calls = 0;
  const bytes = Buffer.from([1]); bytes.toString = () => { calls++; return 'AQ=='; };
  const name = { [Symbol.toPrimitive]() { throw sentinel; } };
  // Act
  const outcome = await capture(() => materialize(kind, { payloads: [{ name, buffer: bytes }] }));
  // Assert
  assert.equal(outcome.error, sentinel);
  assert.equal(calls, 0);
  return { calls, identity: outcome.error === sentinel };
});

direct('MIME String replace and lower-case calls retain their observable order', async kind => {
  // Arrange: these are language primitives, not a test-side MIME implementation.
  const trace = [];
  const originalReplace = String.prototype.replace;
  const originalLowerCase = String.prototype.toLowerCase;
  const name = { [Symbol.toPrimitive](hint) { trace.push('coerce:' + hint); return 'UPPER/FOLDER/FILE.TxT'; } };
  let pending;
  try {
    String.prototype.replace = function (...args) {
      trace.push({ method: 'replace', receiver: String(this), pattern: String(args[0]), replacement: args[1] });
      return Reflect.apply(originalReplace, this, args);
    };
    String.prototype.toLowerCase = function (...args) {
      trace.push({ method: 'lower', receiver: String(this), arguments: args.length });
      return Reflect.apply(originalLowerCase, this, args);
    };
    // Act
    pending = materialize(kind, { payloads: [{ name, buffer: Buffer.from([1]) }] });
  } finally {
    String.prototype.replace = originalReplace;
    String.prototype.toLowerCase = originalLowerCase;
  }
  const result = await pending;
  // Assert: the oracle supplies the precise primitive invocation trace.
  assert.equal(result.filePayloads[0].mimeType, 'text/plain');
  assert.equal(trace[0], 'coerce:string');
  return { trace, mime: result.filePayloads[0].mimeType };
});

for (const name of ['BEFORE\nAFTER/file.TXT', 'folder/BEFORE\nAFTER.TXT', 'folder\\name.TXT']) {
  direct('MIME inference preserves pinned regex behavior for ' + JSON.stringify(name), async kind => {
    // Arrange / Act
    const result = await materialize(kind, { payloads: [{ name, buffer: Buffer.from([1]) }] });
    // Assert: newline-sensitive regex behavior must not be replaced by a
    // generally equivalent-looking native path extension calculation.
    return result.filePayloads[0].mimeType;
  });
}

for (const [label, name] of [
  ['512 KiB directory prefix', 'x'.repeat(512 * 1024) + '/file.TXT'],
  ['512 KiB basename prefix', 'x'.repeat(512 * 1024) + '.TXT'],
  ['thirteen-byte unknown extension', 'file.abcdefghijklm'],
  ['512 KiB unknown extension', 'file.' + 'x'.repeat(512 * 1024)],
  ['uppercase unknown extension', 'file.UNKNOWN_NODE176'],
  ['prototype-looking extension', 'file.__proto__'],
  ['constructor-looking extension', 'file.constructor'],
]) {
  direct('bounded final-extension lookup preserves ' + label, async kind => {
    // Arrange / Act: large names remain language values. This proves returned
    // semantics only, not a measurement of native allocations or V8 byte bounds.
    const result = await materialize(kind, { payloads: [{ name, buffer: Buffer.from([1]) }] });
    // Assert
    assert.equal(result.filePayloads[0].name, name);
    return result.filePayloads[0].mimeType;
  });
}

direct('MIME String coercion accepts Symbol names without template-literal coercion', async kind => {
  // Arrange
  const name = Symbol('node176-name');
  // Act
  const result = await materialize(kind, { payloads: [{ name, buffer: Buffer.from([1]) }] });
  // Assert
  assert.equal(result.filePayloads[0].name, name);
  return result.filePayloads[0].mimeType;
});

direct('transformed extension uses JavaScript property-key coercion after length decisions', async kind => {
  // Arrange
  const trace = [];
  const original = String.prototype.toLowerCase;
  const extension = {
    get length() { trace.push('extension:length'); return 3; },
    [Symbol.toPrimitive](hint) { trace.push('extension:key:' + hint); return 'txt'; },
  };
  const basename = {
    get length() { trace.push('basename:length'); return 10; },
    replace(pattern, replacement) {
      trace.push('basename:replace:' + String(pattern) + ':' + replacement);
      return { toLowerCase() { trace.push('extension:lower'); return extension; } };
    },
  };
  let pending;
  try {
    String.prototype.toLowerCase = function () { trace.push('basename:lower'); return basename; };
    // Act
    pending = materialize(kind, { payloads: [{ name: 'file.txt', buffer: Buffer.from([1]) }] });
  } finally { String.prototype.toLowerCase = original; }
  const result = await pending;
  // Assert: this tests the actual pinned member operations, not a native
  // approximation based only on the original file name's extension.
  assert.equal(result.filePayloads[0].mimeType, 'text/plain');
  assert.ok(trace.includes('extension:key:string'));
  return { trace, mime: result.filePayloads[0].mimeType };
});

direct('payload array map preserves sparse entries and custom species', async kind => {
  // Arrange
  const trace = [];
  class Result extends Array {}
  class Values extends Array { static get [Symbol.species]() { trace.push('species'); return Result; } }
  const values = new Values(3); values[0] = payload(); values[2] = payload();
  // Act
  const result = await materialize(kind, { payloads: values });
  // Assert
  assert.ok(result.filePayloads instanceof Result);
  assert.equal(1 in result.filePayloads, false);
  return { trace, length: result.filePayloads.length, keys: Object.keys(result.filePayloads) };
});

for (const output of ['undefined', 'number', 'object']) {
  direct('custom map ' + output + ' result stays raw and nested', async kind => {
    // Arrange
    let thenReads = 0;
    const raw = output === 'undefined' ? undefined : output === 'number' ? 3 : {
      get then() { thenReads++; throw new Error('nested map result must not be awaited'); },
    };
    const values = [];
    values.map = function (callback) { assert.equal(this, values); assert.equal(callback.length, 1); return raw; };
    // Act
    const result = await materialize(kind, { payloads: values });
    // Assert
    assert.equal(result.filePayloads, raw);
    assert.equal(Object.hasOwn(result, 'filePayloads'), true);
    assert.equal(thenReads, 0);
    return { same: result.filePayloads === raw, thenReads };
  });
}

direct('escaped map callbacks remain fresh unary nonconstructible functions', async kind => {
  // Arrange
  const callbacks = [];
  const values = [];
  values.map = callback => { callbacks.push(callback); return []; };
  // Act
  await materialize(kind, { payloads: values });
  await materialize(kind, { payloads: values });
  const item = callbacks[0](payload(), 'ignored index', 'ignored array');
  const laterBytes = Buffer.from([9]);
  const later = callbacks[1]({ ...payload(laterBytes), lastModifiedMs: 'raw later metadata' });
  // Assert
  assert.notEqual(callbacks[0], callbacks[1]);
  assert.equal(callbacks[0].length, 1);
  assert.equal(Object.hasOwn(callbacks[0], 'prototype'), false);
  assert.throws(() => new callbacks[0](payload()), TypeError);
  assert.equal(item.buffer, 'AQI=');
  assert.equal(later.buffer, 'CQ==');
  assert.equal(later.lastModifiedMs, 'raw later metadata');
  // Runtime behavior after return does not itself prove absence of native
  // captures; the kernel's separate static ZST assertion owns that proof.
  return { count: callbacks.length, item, later };
});

direct('outer async carrier assimilates inherited then while nested values stay raw', async kind => {
  // Arrange
  const saved = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  const trace = [];
  let pending;
  try {
    Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
      if (!Object.hasOwn(this, 'filePayloads')) return undefined;
      trace.push('outer:then');
      return resolve => { trace.push('outer:resolve'); resolve('assimilated outer carrier'); };
    } });
    // Act: the helper's async return reads then before it returns its Promise.
    pending = materialize(kind, { payloads: [payload()] });
  } finally {
    if (saved) Object.defineProperty(Object.prototype, 'then', saved);
    else delete Object.prototype.then;
  }
  const result = await pending;
  // Assert
  assert.equal(result, 'assimilated outer carrier');
  assert.deepEqual(trace, ['outer:then', 'outer:resolve']);
  return { result, trace };
});

direct('empty payloads materialize without serializers', async kind => {
  // Arrange / Act
  const result = await materialize(kind, { payloads: [] });
  // Assert
  assert.deepEqual(result.filePayloads, []);
  return { result, own: ownShape(result) };
});

for (const params of [{}, { payloads: [], localPaths: [] }]) {
  direct('exactly-one input rejection occurs before payload serialization ' + Object.keys(params).join(','), async kind => {
    // Arrange / Act
    const outcome = await capture(() => materialize(kind, params));
    // Assert
    return errorShape(outcome.error);
  });
}

server('raw Buffer identity and unknown metadata stripping survive the second schema', async kind => {
  // Arrange
  const bytes = Buffer.from([1]);
  bytes.toString = () => { throw new Error('server schema must not serialize bytes'); };
  const params = { payloads: [{ ...payload(bytes), lastModifiedMs: 99 }], timeout: 1 };
  // Act
  const result = validateServer(kind, params);
  // Assert
  assert.equal(result.payloads[0].buffer, bytes);
  assert.equal(Object.hasOwn(result.payloads[0], 'lastModifiedMs'), false);
  return { keys: Object.keys(result), item: Object.keys(result.payloads[0]) };
});

server('timeout remains a required schema field even when payloads are empty', async kind => {
  // Arrange / Act
  const outcome = captureSync(() => validateServer(kind, { payloads: [] }));
  // Assert
  assert.match(outcome.error.message, /^timeout: expected float, got undefined$/);
  return errorShape(outcome.error);
});

for (const [label, stream] of [['null', null], ['undefined', undefined], ['number', 1],
  ['missing guid', {}], ['numeric guid', { guid: 1 }]]) {
  server('invalid stream wire shape ' + label, async kind => {
    // Arrange / Act
    const outcome = captureSync(() => validateServer(kind, { streams: [stream], timeout: 1 }));
    // Assert
    return errorShape(outcome.error);
  });
}

for (const second of ['plain', 'object', 'symbol']) {
  server('missing-guid error reads guid twice and coerces second ' + second + ' value', async kind => {
    // Arrange
    const trace = [];
    const later = second === 'plain' ? 'second-guid' : second === 'symbol' ? Symbol('guid') : {
      [Symbol.toPrimitive](hint) { trace.push('coerce:' + hint); return 'coerced-guid'; },
    };
    let reads = 0;
    const stream = { get guid() { trace.push('guid'); return ++reads === 1 ? 'first-guid' : later; } };
    // Act
    const outcome = captureSync(() => validateServer(kind, { directoryStream: stream, timeout: 1 }));
    // Assert
    assert.equal(reads, 2);
    return { trace, error: errorShape(outcome.error) };
  });
}

server('wire guid getter throw retains identity without public channel wrapping', async kind => {
  // Arrange
  const sentinel = new Error('wire guid getter');
  const stream = { get guid() { throw sentinel; } };
  // Act
  const outcome = captureSync(() => validateServer(kind, { streams: [stream], timeout: 1 }));
  // Assert
  assert.equal(outcome.error, sentinel);
  return { same: outcome.error === sentinel, message: sentinel.message };
});

server('escaped stream mapper keeps server direction after the original call returns', async kind => {
  // Arrange
  const callbacks = [];
  const streams = [];
  streams.map = callback => { callbacks.push(callback); return []; };
  // Act
  validateServer(kind, { streams, timeout: 1 });
  const trace = [];
  const outcome = captureSync(() => callbacks[0]({
    get guid() { trace.push('guid'); return 'absent-late-guid'; },
    get _object() { throw new Error('server callback must not switch to client branding'); },
  }, 4));
  // Assert
  assert.deepEqual(trace, ['guid', 'guid']);
  assert.equal(callbacks[0].length, 2);
  assert.match(outcome.error.message, /streams\[4\]: no object with guid absent-late-guid/);
  return { trace, error: errorShape(outcome.error) };
});

function deferredBoundary(kind, params, trace) {
  // The client pass is synchronous. Only the server pass/helper crosses this
  // deliberately explicit transport-shaped task boundary. No Connection
  // response is synthesized, and no final upload promise exists in this seam.
  trace.push('client:validate');
  const client = validateClient(kind, params);
  trace.push('client:done');
  return new Promise((resolve, reject) => {
    setImmediate(() => {
      let serverParams;
      try {
        trace.push('server:validate');
        serverParams = validateServer(kind, client);
        trace.push('server:done', 'materialize');
      } catch (error) { reject(error); return; }
      materialize(kind, serverParams).then(value => resolve({ client, server: serverParams, materialized: value }), reject);
    });
  });
}

scheduled('queued Buffer mutation precedes deferred materialization but follows client projection', async kind => {
  // Arrange
  const trace = [];
  const bytes = Buffer.from([1]);
  bytes.toString = function (encoding) { trace.push('serialize'); return Buffer.prototype.toString.call(this, encoding); };
  const item = { ...payload(bytes), get buffer() {
    queueMicrotask(() => { bytes[0] = 9; trace.push('microtask'); }); return bytes;
  } };
  // Act
  const result = await deferredBoundary(kind, { payloads: [item], timeout: 1 }, trace);
  // Assert
  assert.deepEqual(trace, ['client:validate', 'client:done', 'microtask', 'server:validate', 'server:done', 'materialize', 'serialize']);
  assert.equal(result.client.payloads[0].buffer, bytes);
  assert.equal(result.server.payloads[0].buffer, bytes);
  return { trace, value: result.materialized.filePayloads[0].buffer };
});

for (const invalid of ['boxed name', 'boxed timeout', 'boxed timeout undefined', 'custom map']) {
  scheduled(invalid + ' output accepted by client fails server before serializer', async kind => {
    // Arrange
    const trace = [];
    let calls = 0;
    const bytes = Buffer.from([1]); bytes.toString = () => { calls++; return 'AQ=='; };
    const item = payload(bytes);
    const params = { payloads: [item], timeout: 1 };
    if (invalid === 'boxed name') { item.name = new String('name'); item.name.valueOf = () => 17; }
    if (invalid === 'boxed timeout') { params.timeout = new Number(1); params.timeout.valueOf = () => 'late invalid'; }
    if (invalid === 'boxed timeout undefined') { params.timeout = new Number(1); params.timeout.valueOf = () => undefined; }
    if (invalid === 'custom map') params.payloads.map = () => ({ not: 'an array' });
    // Act
    const outcome = await capture(() => deferredBoundary(kind, params, trace));
    // Assert
    assert.deepEqual(trace, ['client:validate', 'client:done', 'server:validate']);
    assert.equal(calls, 0);
    return { trace, calls, error: errorShape(outcome.error) };
  });
}

scheduled('raw metadata is projected twice but Buffer bytes remain shared', async kind => {
  // Arrange
  const trace = [];
  const bytes = Buffer.from([1]);
  const item = { ...payload(bytes), lastModifiedMs: 99 };
  // Act
  const pending = deferredBoundary(kind, { payloads: [item], timeout: 1 }, trace);
  item.name = 'changed-after-projection.bin'; bytes[0] = 7;
  const result = await pending;
  // Assert
  assert.notEqual(result.client.payloads[0], item);
  assert.notEqual(result.server.payloads[0], result.client.payloads[0]);
  assert.equal(result.materialized.filePayloads[0].name, 'bytes.bin');
  assert.equal(result.materialized.filePayloads[0].lastModifiedMs, undefined);
  return { name: result.materialized.filePayloads[0].name, bytes: result.materialized.filePayloads[0].buffer };
});

scheduled('direct helper serializer errors remain local until a separate error-transport phase', async kind => {
  // Arrange
  const trace = [];
  const sentinel = new Error('local materialization error');
  const bytes = Buffer.from([1]); bytes.toString = () => { throw sentinel; };
  // Act
  const outcome = await capture(() => deferredBoundary(kind, { payloads: [payload(bytes)], timeout: 1 }, trace));
  // Assert: no serializeError/Connection parsing/API prefix is hidden here.
  assert.equal(outcome.error, sentinel);
  assert.equal(sentinel.message, 'local materialization error');
  return { trace, identity: outcome.error === sentinel, message: sentinel.message };
});

for (const [label, params] of [
  ['localPaths', { localPaths: ['/node176/no-file-is-read'] }],
  ['localDirectory', { localDirectory: '/node176/no-directory-is-read' }],
  ['streams', { streams: [{ path() { throw new Error('unsupported stream must not be invoked'); } }] }],
  ['directoryStream', { directoryStream: { path() { throw new Error('unsupported stream must not be invoked'); } } }],
]) {
  test('candidate materialization unsupported contract only: ' + label, { timeout: 5000 }, () => {
    // Arrange / Act: do NOT call the real helper for these excluded file/stream
    // branches. This is a fail-closed candidate contract, not paired parity.
    const outcome = captureSync(() => native.inspectFileInputMaterialization(params, materializationPorts));
    // Assert
    assert.ok(outcome.error instanceof Error);
    assert.match(outcome.error.message, /PWRIGHT_UNSUPPORTED/);
  });
}
