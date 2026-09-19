// Offline synchronous channel-schema kernel comparison only. The oracle is
// Playwright 1.58.2's real validator and ChannelOwner wire context, configured
// for normal in-process raw Buffers and isUnderTest=false. The candidate calls
// the same Rust kernel as the selected preparation's channel phase through a
// stateless diagnostic: no ticket/session authority, transport, server phase,
// channel API error prefix/stack rewriting, Buffer snapshot or upload is tested.
// Real upstream WritableStream brands are explicitly outside this subset.
// Source: lib/protocol/validator.js:2197, validatorPrimitives.js:57–150,
// client/channelOwner.js:109–114,186–189 and inProcessFactory.js:31–47.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { channelPorts } = require('../src/file-input-channel.cjs');
const { prepareChannel } = require('../src/file-input-preflight.cjs');
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
assert.equal(typeof native.inspectFileInputChannel, 'function');

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { findValidator } = requireCore(join(core, 'lib/protocol/validator.js'));
const { ChannelOwner } = requireCore(join(core, 'lib/client/channelOwner.js'));
const { ElementHandle } = requireCore(join(core, 'lib/client/elementHandle.js'));
const { Frame } = requireCore(join(core, 'lib/client/frame.js'));
const { TimeoutSettings } = requireCore(join(core, 'lib/client/timeoutSettings.js'));
const { nodePlatform } = requireCore(join(core, 'lib/server/utils/nodePlatform.js'));
const oracleContext = Reflect.apply(ChannelOwner.prototype._validatorToWireContext, {
  _connection: { rawBuffers: () => true }, _platform: { isUnderTest: () => false },
}, []);
const oracle = findValidator('ElementHandle', 'setInputFiles', 'Params');
const environment = new native.NativeEnvironment(setImmediate,
  Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength').get);

function validate(kind, params) {
  return kind === 'oracle' ? oracle(params, '', oracleContext) : native.inspectFileInputChannel(params, channelPorts);
}
function payload(buffer = Buffer.from([1, 2])) { return { name: 'bytes.bin', mimeType: 'application/octet-stream', buffer }; }
function paired(name, exercise) {
  test(`offline channel kernel: ${name}`, { timeout: 5000 }, () => {
    // Arrange/Act: fresh values and callbacks per implementation.
    const expected = exercise('oracle');
    const actual = exercise('candidate');
    // Assert
    assert.deepEqual(actual, expected);
  });
}
function failure(kind, params) {
  let rejected;
  try { validate(kind, params); } catch (error) { rejected = error; }
  assert.ok(rejected instanceof Error, 'the actual synchronous validator must reject');
  return rejected;
}

test('offline channel kernel: frozen ports expose language operations without a schema driver', () => {
  // Arrange
  const expected = ['read', 'assign', 'is_undefined', 'is_null', 'is_array', 'is_string_object',
    'is_number_object', 'is_buffer', 'value_of', 'map', 'add', 'mapper', 'error'];
  // Act
  const actual = Object.keys(channelPorts);
  // Assert
  assert.equal(Object.isFrozen(channelPorts), true);
  assert.deepEqual(actual, expected);
  for (const name of actual) assert.equal(typeof channelPorts[name], 'function');
  assert.equal(channelPorts.error('example').constructor.name, 'ValidationError');
});

paired('schema reads all declared fields and payload members in fixed order', kind => {
  // Arrange
  const trace = [];
  const markedArray = (label, values) => Object.defineProperty(values, 'map', { get() {
    trace.push(`${label}:map`);
    return function (callback) { assert.equal(this, values); return Array.prototype.map.call(this, callback); };
  } });
  const bytes = Buffer.from([1]);
  const item = new Proxy({ name: 'one.bin', mimeType: 'text/plain', buffer: bytes }, {
    get(target, key) { trace.push(`item:${String(key)}`); return Reflect.get(target, key); },
  });
  const params = new Proxy({ payloads: markedArray('payloads', [item]), localDirectory: undefined,
    directoryStream: undefined, localPaths: markedArray('paths', ['one']), streams: markedArray('streams', []), timeout: 17 }, {
    get(target, key) { trace.push(`params:${String(key)}`); return Reflect.get(target, key); },
  });
  // Act
  const result = validate(kind, params);
  // Assert
  assert.equal(result.payloads[0].buffer, bytes);
  assert.deepEqual(trace, ['params:payloads', 'payloads:map', 'item:name', 'item:mimeType', 'item:buffer',
    'params:localDirectory', 'params:directoryStream', 'params:localPaths', 'paths:map',
    'params:streams', 'streams:map', 'params:timeout']);
  return { trace, keys: Object.keys(result), payloadKeys: Object.keys(result.payloads[0]) };
});

paired('optional undefined fields are omitted and unknown fields are not read', kind => {
  // Arrange
  const unknown = { get() { throw new Error('unknown field must not be read'); }, enumerable: true };
  const item = { name: 'one.bin', mimeType: undefined, buffer: Buffer.from([1]) };
  for (const key of ['lastModifiedMs', 'unknown']) Object.defineProperty(item, key, unknown);
  Object.defineProperty(item, Symbol('unknown'), unknown);
  const params = { payloads: [item], timeout: 17 };
  for (const key of ['noWaitAfter', 'unknown', '__testHookSomething']) Object.defineProperty(params, key, unknown);
  // Act
  const result = validate(kind, params);
  // Assert
  assert.deepEqual(Object.keys(result), ['payloads', 'timeout']);
  assert.deepEqual(Object.keys(result.payloads[0]), ['name', 'buffer']);
  return { keys: Object.keys(result), itemKeys: Object.keys(result.payloads[0]) };
});

paired('known inherited fields are read and object-shaped arrays are accepted', kind => {
  // Arrange
  const item = [];
  Object.setPrototypeOf(item, Object.assign(Object.create(Array.prototype), payload()));
  const params = Object.create({ payloads: [item], timeout: 9 });
  // Act
  const result = validate(kind, params);
  // Assert
  assert.equal(result.payloads[0].buffer, item.buffer);
  return { name: result.payloads[0].name, timeout: result.timeout, keys: Object.keys(result) };
});

for (const [label, arrange] of [
  ['null params', () => null], ['undefined params', () => undefined], ['function params', () => () => {}],
  ['number params', () => 1], ['null payloads', () => ({ payloads: null, timeout: 1 })],
  ['array-like payloads', () => ({ payloads: { 0: payload(), length: 1 }, timeout: 1 })],
  ['null payload', () => ({ payloads: [null], timeout: 1 })],
  ['missing name', () => ({ payloads: [{ buffer: Buffer.from([1]) }], timeout: 1 })],
  ['numeric name', () => ({ payloads: [{ ...payload(), name: 1 }], timeout: 1 })],
  ['null mimeType', () => ({ payloads: [{ ...payload(), mimeType: null }], timeout: 1 })],
  ['typed array buffer', () => ({ payloads: [payload(new Uint8Array([1]))], timeout: 1 })],
  ['null buffer', () => ({ payloads: [payload(null)], timeout: 1 })],
  ['numeric localDirectory', () => ({ localDirectory: 1, timeout: 1 })],
  ['nonarray localPaths', () => ({ localPaths: 'path', timeout: 1 })],
  ['numeric localPath', () => ({ localPaths: [1], timeout: 1 })],
  ['nonarray streams', () => ({ streams: {}, timeout: 1 })],
  ['missing timeout', () => ({ payloads: [] })], ['string timeout', () => ({ payloads: [], timeout: '1' })],
  ['null timeout', () => ({ payloads: [], timeout: null })],
]) paired(`rejects ${label} with the schema path`, kind => {
  // Arrange / Act
  const error = failure(kind, arrange());
  // Assert
  assert.equal(error.constructor.name, 'ValidationError');
  return { name: error.name, message: error.message };
});

paired('first invalid payload field stops later getters and later top-level validation', kind => {
  // Arrange
  const trace = [];
  const params = {
    payloads: [{ get name() { trace.push('name'); return 42; }, get mimeType() { throw new Error('later mimeType'); } }],
    get localDirectory() { throw new Error('later directory'); }, get timeout() { throw new Error('later timeout'); },
  };
  // Act
  const error = failure(kind, params);
  // Assert
  assert.deepEqual(trace, ['name']);
  return { trace, message: error.message };
});

for (const field of ['name', 'timeout']) for (const output of ['primitive', 'undefined', 'object']) {
  paired(`boxed ${field} returns arbitrary ${output} valueOf output without revalidation`, kind => {
    // Arrange
    const value = output === 'primitive' ? (field === 'name' ? 42 : 'raw timeout')
      : output === 'object' ? { raw: field } : undefined;
    const boxed = field === 'name' ? new String('ignored') : new Number(99);
    let calls = 0;
    boxed.valueOf = function () { assert.equal(this, boxed); calls++; return value; };
    const params = { payloads: [{ ...payload(), ...(field === 'name' ? { name: boxed } : {}) }], timeout: field === 'timeout' ? boxed : 1 };
    // Act
    const result = validate(kind, params);
    const target = field === 'name' ? result.payloads[0] : result;
    // Assert: even a required field disappears when boxed valueOf returns undefined.
    assert.equal(target[field], value);
    assert.equal(Object.hasOwn(target, field), output !== 'undefined');
    assert.equal(calls, 1);
    return { calls, owns: Object.hasOwn(target, field), type: typeof target[field] };
  });
}

paired('raw Buffer identity remains mutable and the custom serializer is never called', kind => {
  // Arrange
  const bytes = Buffer.from([1, 2]);
  bytes.toString = () => { throw new Error('client schema must not serialize bytes'); };
  // Act
  const result = validate(kind, { payloads: [payload(bytes)], timeout: 1 });
  bytes[0] = 9;
  // Assert
  assert.equal(result.payloads[0].buffer, bytes);
  return [...result.payloads[0].buffer];
});

paired('Buffer prototype branding is not replaced by an eager native byte extraction', kind => {
  // Arrange: no byte-reading method may touch this language-only prototype object.
  const bytes = Object.create(Buffer.prototype);
  Object.defineProperty(bytes, 'byteLength', { get() { throw new Error('no byte extraction'); } });
  // Act
  const result = validate(kind, { payloads: [payload(bytes)], timeout: 1 });
  // Assert
  assert.equal(result.payloads[0].buffer, bytes);
  return result.payloads[0].buffer === bytes;
});

paired('array map preserves holes and species construction', kind => {
  // Arrange
  const trace = [];
  class ResultArray extends Array {}
  class Payloads extends Array { static get [Symbol.species]() { trace.push('species'); return ResultArray; } }
  const values = new Payloads(3);
  values[0] = payload(); values[2] = payload();
  // Act
  const result = validate(kind, { payloads: values, timeout: 1 });
  // Assert
  assert.ok(result.payloads instanceof ResultArray);
  assert.equal(1 in result.payloads, false);
  return { trace, length: result.payloads.length, keys: Object.keys(result.payloads) };
});

for (const output of ['number', 'undefined', 'object']) paired(`custom map ${output} result is not projected to a native array`, kind => {
  // Arrange
  const resultValue = output === 'number' ? 17 : output === 'object' ? {
    get then() { throw new Error('nested map result must not be Promise-assimilated'); },
  } : undefined;
  const values = [];
  values.map = function (callback) { assert.equal(this, values); assert.equal(callback.length, 2); return resultValue; };
  // Act
  const result = validate(kind, { payloads: values, timeout: 1 });
  // Assert
  assert.equal(result.payloads, resultValue);
  return { owns: Object.hasOwn(result, 'payloads'), type: typeof result.payloads };
});

paired('escaped fresh arity-two map callbacks validate after the original call returns', kind => {
  // Arrange
  const callbacks = [];
  const values = [];
  values.map = callback => { callbacks.push(callback); return []; };
  // Act
  validate(kind, { payloads: values, timeout: 1 });
  validate(kind, { payloads: values, timeout: 1 });
  const item = payload();
  const later = callbacks[0](item, 7);
  let error;
  try { callbacks[1]({ name: 42 }, 'later'); } catch (value) { error = value; }
  // Assert
  assert.notEqual(callbacks[0], callbacks[1]);
  assert.equal(callbacks[0].length, 2);
  assert.equal(Object.hasOwn(callbacks[0], 'prototype'), false);
  assert.throws(() => new callbacks[0](item, 0), TypeError);
  assert.equal(later.buffer, item.buffer);
  assert.match(error.message, /payloads\[later\]\.name/);
  return { count: callbacks.length, message: error.message, keys: Object.keys(later) };
});

paired('custom callback index concatenation precedes payload validation and uses default coercion', kind => {
  // Arrange
  const trace = [];
  const index = { [Symbol.toPrimitive](hint) { trace.push(`index:${hint}`); return 'custom'; } };
  const values = [];
  values.map = callback => callback({ get name() { trace.push('name'); return 42; } }, index);
  // Act
  const error = failure(kind, { payloads: values, timeout: 1 });
  // Assert
  assert.deepEqual(trace, ['index:default', 'name']);
  return { trace, message: error.message };
});

paired('a Symbol callback index throws before any payload getter', kind => {
  // Arrange
  const values = [];
  let reads = 0;
  values.map = callback => callback({ get name() { reads++; return 'unused'; } }, Symbol('index'));
  // Act
  const error = failure(kind, { payloads: values, timeout: 1 });
  // Assert
  assert.equal(reads, 0);
  assert.ok(error instanceof TypeError);
  return { reads, name: error.name, message: error.message };
});

paired('ordinary output assignment invokes inherited setters at both schema levels', kind => {
  // Arrange
  const saved = new Map(['payloads', 'name'].map(key => [key, Object.getOwnPropertyDescriptor(Object.prototype, key)]));
  const trace = [];
  let result;
  try {
    for (const key of saved.keys()) Object.defineProperty(Object.prototype, key, { configurable: true, set(value) {
      trace.push(key);
      Object.defineProperty(this, key, { value, enumerable: true, configurable: true, writable: true });
    } });
    // Act
    result = validate(kind, { payloads: [payload()], timeout: 1 });
  } finally {
    for (const [key, descriptor] of saved) {
      if (descriptor) Object.defineProperty(Object.prototype, key, descriptor);
      else delete Object.prototype[key];
    }
  }
  // Assert
  assert.deepEqual(trace, ['name', 'payloads']);
  return { trace, name: result.payloads[0].name };
});

for (const phase of ['payload getter', 'map getter', 'valueOf', 'output setter']) paired(`${phase} thrown Error keeps identity without API prefix`, kind => {
  // Arrange
  const sentinel = new Error(`original ${phase}`);
  const params = { payloads: [payload()], timeout: 1 };
  const throwOriginal = () => { throw sentinel; };
  const saved = Object.getOwnPropertyDescriptor(Object.prototype, 'payloads');
  let error;
  try {
    if (phase === 'payload getter') Object.defineProperty(params.payloads[0], 'name', { get: throwOriginal });
    if (phase === 'map getter') Object.defineProperty(params.payloads, 'map', { get: throwOriginal });
    if (phase === 'valueOf') { params.timeout = new Number(1); params.timeout.valueOf = throwOriginal; }
    if (phase === 'output setter') Object.defineProperty(Object.prototype, 'payloads', { configurable: true, set: throwOriginal });
    // Act
    error = failure(kind, params);
  } finally {
    if (phase === 'output setter') {
      if (saved) Object.defineProperty(Object.prototype, 'payloads', saved);
      else delete Object.prototype.payloads;
    }
  }
  // Assert
  assert.equal(error, sentinel);
  assert.equal(error.message, `original ${phase}`);
  return error.message;
});

for (const field of ['directoryStream', 'streams']) paired(`ordinary invalid ${field} reads _object once then rejects before timeout`, kind => {
  // Arrange
  const trace = [];
  const stream = { get _object() { trace.push('_object'); return { _type: 'WritableStream', _guid: 'not-a-brand' }; } };
  const params = { [field]: field === 'streams' ? [stream] : stream, get timeout() { throw new Error('timeout must not run'); } };
  // Act
  const error = failure(kind, params);
  // Assert
  assert.deepEqual(trace, ['_object']);
  return { trace, message: error.message };
});

paired('null stream preserves the language member-read TypeError', kind => {
  // Arrange / Act
  const error = failure(kind, { directoryStream: null, timeout: 1 });
  // Assert
  assert.ok(error instanceof TypeError);
  return { name: error.name, message: error.message };
});

paired('invalid stream getter exceptions preserve identity before any branding result', kind => {
  // Arrange
  const sentinel = new Error('stream _object getter');
  const stream = { get _object() { throw sentinel; } };
  // Act
  const error = failure(kind, { streams: [stream], timeout: 1 });
  // Assert
  assert.equal(error, sentinel);
  return error.message;
});

paired('missing and mutually conflicting inputs both pass the client schema', kind => {
  // Arrange / Act
  const empty = validate(kind, { timeout: 1 });
  const conflicting = validate(kind, { payloads: [], localDirectory: '', localPaths: [], streams: [], timeout: 1 });
  // Assert: exactly-one-input policy belongs to deferred prepareFilesForUpload,
  // not this client schema. No server helper or final upload is faked here.
  assert.deepEqual(Object.keys(empty), ['timeout']);
  return { empty: Object.keys(empty), conflicting: Object.keys(conflicting) };
});

for (const [label, timeout] of [['zero', 0], ['negative zero', -0], ['fractional', 1.5], ['negative', -1],
  ['NaN', NaN], ['positive infinity', Infinity], ['negative infinity', -Infinity]]) paired(`raw ${label} timeout is preserved without native deadline coercion`, kind => {
  // Arrange / Act
  const result = validate(kind, { payloads: [], timeout });
  // Assert
  assert.equal(Object.is(result.timeout, timeout), true);
  return { preserved: Object.is(result.timeout, timeout), keys: Object.keys(result) };
});

for (const thrownKind of ['primitive', 'object']) paired(`raw ${thrownKind} getter throw is not replaced by channel API wrapping`, kind => {
  // Arrange
  const sentinel = thrownKind === 'primitive' ? 'original primitive' : Object.freeze({ original: true });
  const item = { ...payload(), get name() { throw sentinel; } };
  let thrown;
  // Act
  try { validate(kind, { payloads: [item], timeout: 1 }); } catch (error) { thrown = error; }
  // Assert: wrapping belongs to the public channel caller, not this kernel.
  assert.equal(thrown, sentinel);
  return { identity: thrown === sentinel, type: typeof thrown };
});

function languageDriver(kind, files, options, trace) {
  const platform = { ...nodePlatform, isDebugMode: () => false };
  const contextSettings = kind === 'oracle' ? new TimeoutSettings(platform)
    : environment.createFileInputTimeoutSettings(false);
  const pageSettings = kind === 'oracle' ? new TimeoutSettings(platform, contextSettings) : contextSettings.child();
  pageSettings.setDefaultTimeout(17);
  const frame = Object.create(Frame.prototype);
  frame._platform = platform;
  frame._page = { _timeoutSettings: pageSettings };
  const owner = { page: () => ({ context: () => ({}) }) };
  const state = { params: undefined, validated: undefined, aborts: 0 };
  const receiver = {
    ownerFrame() { trace.push('owner'); return owner; },
    _platform: platform,
    _frame: frame,
    // Real ElementHandle body plus a defined synchronous channel-kernel seam.
    // No Connection._wrapApiCall, transport reply or fake final upload occurs.
    _elementChannel: { setInputFiles(params) {
      trace.push('channel');
      state.params = params;
      state.validated = oracle(params, '', oracleContext);
    } },
  };
  if (kind === 'oracle') return { state, promise: Reflect.apply(ElementHandle.prototype.setInputFiles, receiver, [files, options]) };
  let context, converted;
  // Explicit stateless adapter, NOT a NativeFileInputPreparation or authority
  // proof. Every language phase calls its actual selected Rust diagnostic.
  const preparation = {
    beginOwnerLookup(ports) { return native.inspectFileInputPreflightOwner(receiver, ports); },
    prepareContext(value, ports) { context = native.inspectFileInputPreflightContext(value, ports); },
    convertInput(ports) { return native.inspectFileInputPreflightConversion(context, files, ports); },
    finishConversion(value) { converted = value; },
    assembleArguments(ports) {
      state.params = native.inspectFileInputArguments(converted, options, pageSettings, ports);
      return state.params;
    },
    validateChannel(ports) {
      trace.push('channel');
      state.validated = native.inspectFileInputChannel(state.params, ports);
      return state.validated;
    },
    abort() { state.aborts++; },
  };
  return { state, preparation, promise: prepareChannel(preparation) };
}

function pairedDriver(name, exercise) {
  test(`offline channel driver adapter: ${name}`, { timeout: 5000 }, async () => {
    // Arrange/Act: real upstream method versus real Rust kernels in a stateless
    // adapter. Actual ticket phase/liveness tests remain a separate gate.
    const expected = await exercise('oracle');
    const actual = await exercise('candidate');
    // Assert
    assert.deepEqual(actual, expected);
  });
}

pairedDriver('channel validation follows assembly in the same conversion continuation', async kind => {
  // Arrange
  const trace = [];
  const files = [{ ...payload(), get name() { trace.push('name'); return 'ordered.bin'; } }];
  const options = { get marker() {
    trace.push('options'); queueMicrotask(() => trace.push('microtask:options')); return true;
  } };
  // Act
  const run = languageDriver(kind, files, options, trace);
  await run.promise;
  // Assert
  assert.deepEqual(trace, ['owner', 'options', 'channel', 'name', 'microtask:options']);
  assert.equal(run.state.validated.payloads[0].name, 'ordered.bin');
  assert.equal(run.state.aborts, 0);
  return { trace, keys: Object.keys(run.state.validated) };
});

pairedDriver('unknown assembled then is stripped without assimilating the argument result', async kind => {
  // Arrange
  const trace = [];
  let thenCalls = 0;
  const hostileThen = () => { thenCalls++; throw new Error('assembled params must not be awaited'); };
  const options = { get then() { trace.push('option:then'); return hostileThen; } };
  // Act
  const run = languageDriver(kind, [], options, trace);
  await run.promise;
  // Assert
  assert.equal(run.state.params.then, hostileThen);
  assert.equal(Object.hasOwn(run.state.validated, 'then'), false);
  assert.equal(thenCalls, 0);
  assert.equal(run.state.aborts, 0);
  return { trace, thenCalls, keys: Object.keys(run.state.validated) };
});

pairedDriver('channel getter error preserves identity and candidate driver aborts its adapter', async kind => {
  // Arrange
  const trace = [];
  const sentinel = new Error('channel payload getter');
  const files = [{ ...payload(), get name() { trace.push('name'); throw sentinel; } }];
  // Act
  const run = languageDriver(kind, files, {}, trace);
  const error = await run.promise.then(() => assert.fail('getter must reject'), value => value);
  // Assert
  assert.equal(error, sentinel);
  assert.equal(error.message, 'channel payload getter');
  if (kind === 'candidate') assert.equal(run.state.aborts, 1);
  assert.equal(run.state.validated, undefined);
  return { trace, message: error.message, identity: error === sentinel };
});
