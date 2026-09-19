// Offline N-API language-kernel differential evidence only. The candidate uses
// stateless diagnostics, not a native preparation ticket or session. The oracle
// runs real pinned convertInputFiles and ElementHandle.setInputFiles bodies;
// metadata and the stop at the first options read are fixtures. No browser,
// channel validation, file materialization or public upload parity is claimed.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { ports, preflight } = require('../src/file-input-preflight.cjs');
const { nativeArtifact, verifyNativeArtifact } = require('../src/native-artifact.cjs');
assert.ok(['retained-file-input', 'retained-file-input-combined', 'retained-target-marking',
  'retained-target-marking-combined', 'retained-target-prevalidation',
  'retained-target-prevalidation-combined', 'retained-input-instrumentation',
  'retained-input-instrumentation-combined', 'retained-call-argument-serialization',
  'retained-call-argument-serialization-combined', 'retained-serialized-payload-import',
  'retained-serialized-payload-import-combined', 'retained-serialized-payload-mutation',
  'retained-serialized-payload-mutation-combined'].includes(nativeArtifact.profile),
  'select an explicitly built retained-file-input artifact');
const native = require(nativeArtifact.path);
verifyNativeArtifact(native, nativeArtifact);
for (const name of ['inspectFileInputPreflightOwner', 'inspectFileInputPreflightContext', 'inspectFileInputPreflightConversion'])
  assert.equal(typeof native[name], 'function', `selected artifact exports ${name}`);

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { ElementHandle, convertInputFiles } = requireCore(join(core, 'lib/client/elementHandle.js'));
const { nodePlatform } = requireCore(join(core, 'lib/server/utils/nodePlatform.js'));

function paired(name, exercise) {
  test(`offline preflight kernel: ${name}`, { timeout: 5000 }, async () => {
    // Arrange/Act: fresh language values per implementation, no shared owners.
    const expected = await exercise('oracle');
    const actual = await exercise('candidate');
    // Assert
    assert.deepEqual(actual, expected);
  });
}

async function nativeConversion(context, files) {
  // Match the upstream async function's return assimilation and exception job.
  return native.inspectFileInputPreflightConversion(context, files, ports);
}
function conversion(kind, files, context = {}) {
  return kind === 'oracle' ? convertInputFiles(nodePlatform, files, context) : nativeConversion(context, files);
}
function owner(context = {}) { return { page() { return { context() { return context; } }; } }; }

function start(kind, receiver, files, trace) {
  const stop = Object.freeze({ boundary: 'first post-conversion options read' });
  const state = { aborts: 0, converted: undefined };
  if (kind === 'oracle') {
    // Real upstream method, minimal metadata only. Throwing the first option
    // getter prevents later timeout, channel and server phases from running.
    Object.defineProperties(receiver, {
      _platform: { value: nodePlatform },
      _elementChannel: { value: { setInputFiles() { throw new Error('channel must not run'); } } },
      _frame: { value: { _timeout() { throw new Error('timeout must not run'); } } },
    });
    const options = { get stop() { trace.push('continuation'); throw stop; } };
    return { promise: Reflect.apply(ElementHandle.prototype.setInputFiles, receiver, [files, options]), stop, state };
  }
  let context;
  // Test-only stateless kernel adapter: this must not be mistaken for a raw
  // NativeFileInputPreparation or proof of its type/session/revocation guards.
  const preparation = {
    beginOwnerLookup(language) { return native.inspectFileInputPreflightOwner(receiver, language); },
    prepareContext(value, language) { context = native.inspectFileInputPreflightContext(value, language); },
    convertInput(language) { return native.inspectFileInputPreflightConversion(context, files, language); },
    finishConversion(value) { state.converted = value; trace.push('continuation'); throw stop; },
    abort() { state.aborts++; },
  };
  return { promise: preflight(preparation), stop, state };
}

test('offline preflight kernel: language ports are frozen and contain no operation driver', { timeout: 5000 }, () => {
  // Arrange
  const names = ['owner_frame', 'page', 'context', 'is_array', 'slice', 'some', 'every', 'reduce',
    'buffer', 'byte_length', 'add', 'at_least', 'unary', 'binary', 'error'];
  // Act
  const actual = Object.keys(ports);
  // Assert
  assert.equal(Object.isFrozen(ports), true);
  assert.deepEqual(actual, names);
  for (const name of names) assert.equal(typeof ports[name], 'function');
});

paired('public owner, page and context receivers precede all payload reads', async kind => {
  // Arrange
  const trace = [];
  const context = {};
  const page = { get context() { trace.push('context:get'); return function () {
    assert.equal(this, page); trace.push('context:call'); return context;
  }; } };
  const frame = { get page() { trace.push('page:get'); return function () {
    assert.equal(this, frame); trace.push('page:call'); return page;
  }; } };
  const receiver = { get ownerFrame() { trace.push('owner:get'); return function () {
    assert.equal(this, receiver); trace.push('owner:call'); return frame;
  }; } };
  const bytes = { get byteLength() { trace.push('byteLength'); return 1; } };
  const item = { get buffer() { trace.push('buffer'); return bytes; } };
  const items = {
    get some() { trace.push('some:get'); return function (predicate) {
      assert.equal(this, items); trace.push('some:call'); return predicate(item);
    }; },
    get reduce() { trace.push('reduce:get'); return function (reducer, initial) {
      assert.equal(this, items); trace.push('reduce:call'); return reducer(initial, item);
    }; },
  };
  const files = [];
  Object.defineProperty(files, 'slice', { get() { trace.push('slice:get'); return function () {
    assert.equal(this, files); trace.push('slice:call'); return items;
  }; } });
  // Act
  const run = start(kind, receiver, files, trace);
  const error = await run.promise.catch(value => value);
  // Assert
  assert.equal(error, run.stop);
  assert.deepEqual(trace, ['owner:get', 'owner:call', 'page:get', 'page:call', 'context:get', 'context:call',
    'slice:get', 'slice:call', 'some:get', 'some:call', 'reduce:get', 'reduce:call',
    'buffer', 'buffer', 'byteLength', 'continuation']);
  if (kind === 'candidate') assert.equal(run.state.aborts, 1);
  return trace;
});

paired('overridden owner thenable is awaited before files are read', async kind => {
  // Arrange
  const trace = [];
  const files = [];
  files.slice = function () { trace.push('slice'); return []; };
  const thenable = { get then() { trace.push('then:get'); return function (resolve) {
    assert.equal(this, thenable); trace.push('then:call'); resolve(owner());
  }; } };
  const receiver = { ownerFrame() { trace.push('owner'); return thenable; } };
  // Act
  const run = start(kind, receiver, files, trace);
  trace.push('returned');
  const error = await run.promise.catch(value => value);
  // Assert
  assert.equal(error, run.stop);
  assert.deepEqual(trace, ['owner', 'then:get', 'returned', 'then:call', 'slice', 'continuation']);
  return trace;
});

for (const stage of ['owner', 'page', 'context', 'conversion']) {
  paired(`${stage} thrown value keeps identity and the correct await boundary`, async kind => {
    // Arrange
    const trace = [];
    const primary = Object.freeze({ original: stage });
    const fail = () => { trace.push(stage); throw primary; };
    const frame = stage === 'page' ? { page: fail }
      : stage === 'context' ? { page() { return { context: fail }; } } : owner();
    const receiver = { ownerFrame: stage === 'owner' ? fail : () => frame };
    const files = stage === 'conversion' ? { get buffer() { return fail(); } } : [];
    // Act
    const run = start(kind, receiver, files, trace);
    const settled = run.promise.catch(error => { trace.push('rejected'); return error; });
    queueMicrotask(() => { trace.push('tick:1'); queueMicrotask(() => trace.push('tick:2')); });
    const error = await settled;
    await Promise.resolve();
    // Assert: conversion is itself async; context/page argument evaluation is not.
    assert.equal(error, primary);
    assert.equal(trace.includes('continuation'), false);
    if (stage === 'conversion') assert.ok(trace.indexOf('tick:2') < trace.indexOf('rejected'));
    else assert.ok(trace.indexOf('rejected') < trace.indexOf('tick:2'));
    if (kind === 'candidate') assert.equal(run.state.aborts, 1);
    return trace;
  });
}

paired('falsy owner fails before any files or context getter', async kind => {
  // Arrange
  const trace = [];
  const receiver = { ownerFrame() { trace.push('owner'); return null; } };
  const files = { get buffer() { throw new Error('files must not run'); } };
  // Act
  const run = start(kind, receiver, files, trace);
  const error = await run.promise.catch(value => value);
  // Assert
  assert.equal(error.message, 'Cannot set input files to detached element');
  assert.deepEqual(trace, ['owner']);
  return { message: error.message, trace };
});

paired('empty conversion still yields to a slice-scheduled microtask', async kind => {
  // Arrange
  const trace = [];
  const files = [];
  files.slice = function () { trace.push('slice'); queueMicrotask(() => trace.push('microtask')); return []; };
  // Act
  const run = start(kind, { ownerFrame: () => owner() }, files, trace);
  const error = await run.promise.catch(value => value);
  // Assert
  assert.equal(error, run.stop);
  assert.deepEqual(trace, ['slice', 'microtask', 'continuation']);
  return trace;
});

paired('ordinary empty carrier observes inherited then and its replacement value', async kind => {
  // Arrange: narrow the poison to this test's payload carrier, then restore it
  // before handing control to the test runner. No unresolved thenable is used.
  const trace = [];
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  const replacement = Object.assign(Object.create(null), { marker: 'assimilated' });
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (!Object.hasOwn(this, 'payloads')) return undefined;
    trace.push('carrier:then:get');
    return function (resolve) { trace.push('carrier:then:call'); resolve(replacement); };
  } });
  try {
    // Act
    const result = await conversion(kind, []);
    // Assert
    assert.equal(result, replacement);
    assert.deepEqual(trace, ['carrier:then:get', 'carrier:then:call']);
    return trace;
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});

paired('carrier then getter exceptions preserve original identity', async kind => {
  // Arrange
  const primary = Object.freeze({ original: 'carrier getter' });
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (Object.hasOwn(this, 'payloads')) throw primary;
    return undefined;
  } });
  try {
    // Act
    const error = await conversion(kind, []).catch(value => value);
    // Assert
    assert.equal(error, primary);
    return { identity: error === primary };
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});

paired('the preparation driver awaits the carrier before its continuation', async kind => {
  // Arrange
  const trace = [];
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  const replacement = Object.assign(Object.create(null), { marker: 'replacement' });
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (!Object.hasOwn(this, 'payloads')) return undefined;
    trace.push('carrier:then:get');
    return function (resolve) { trace.push('carrier:then:call'); resolve(replacement); };
  } });
  try {
    // Act
    const run = start(kind, { ownerFrame: () => owner() }, [], trace);
    const error = await run.promise.catch(value => value);
    // Assert
    assert.equal(error, run.stop);
    assert.deepEqual(trace, ['carrier:then:get', 'carrier:then:call', 'continuation']);
    if (kind === 'candidate') assert.equal(run.state.converted, replacement);
    return trace;
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});

paired('Array.isArray is an observable call whose result uses truthiness', async kind => {
  // Arrange: restrict the override to one exact input, leaving runner behavior alone.
  const trace = [];
  const items = [];
  const files = { slice() { trace.push('slice'); return items; } };
  const previous = Array.isArray;
  Array.isArray = function (value) {
    if (value !== files) return Reflect.apply(previous, this, [value]);
    assert.equal(this, Array); trace.push('isArray'); return {};
  };
  let result;
  try {
    // Act
    result = await conversion(kind, files);
  } finally {
    Array.isArray = previous;
  }
  // Assert
  assert.equal(result.payloads, items);
  assert.deepEqual(trace, ['isArray', 'slice']);
  return trace;
});

paired('singleton wrapping defines its own index without invoking a prototype setter', async kind => {
  // Arrange: other array writes retain ordinary own-data-property behavior.
  const item = { buffer: { byteLength: 1 } };
  const previous = Object.getOwnPropertyDescriptor(Array.prototype, '0');
  let intercepted = 0;
  Object.defineProperty(Array.prototype, '0', { configurable: true, set(value) {
    if (value === item) { intercepted++; throw new Error('singleton must use own data property'); }
    Object.defineProperty(this, '0', { value, configurable: true, enumerable: true, writable: true });
  } });
  let result;
  try {
    // Act
    result = await conversion(kind, item);
  } finally {
    if (previous) Object.defineProperty(Array.prototype, '0', previous);
    else delete Array.prototype[0];
  }
  // Assert
  assert.equal(intercepted, 0);
  assert.equal(result.payloads[0], item);
  assert.equal(Object.hasOwn(result.payloads, '0'), true);
  return { intercepted, ownsIndex: Object.hasOwn(result.payloads, '0') };
});

paired('some and every use truthiness with distinct fresh unary arrows', async kind => {
  // Arrange
  const observed = [];
  let somePredicate;
  const items = {
    some(predicate) {
      somePredicate = predicate;
      observed.push(predicate.length, Object.hasOwn(predicate, 'prototype'), predicate(), predicate('path'), predicate(new String('path')));
      assert.throws(() => Reflect.construct(predicate, []), TypeError);
      return {}; // Truthy, not a Boolean: enter the paths branch.
    },
    every(predicate) {
      observed.push(predicate !== somePredicate, predicate.length, predicate(3, 'ignored'));
      return ''; // Falsy, not a Boolean: reject before filesystem work.
    },
  };
  const files = [];
  files.slice = () => items;
  // Act
  const error = await conversion(kind, files).catch(value => value);
  // Assert
  assert.equal(error.message, 'File paths cannot be mixed with buffers');
  assert.deepEqual(observed, [1, false, false, true, false, true, 1, false]);
  return { observed, message: error.message };
});

paired('falsy some result skips every and reduce result receives JS comparison coercion', async kind => {
  // Arrange
  const trace = [];
  const items = {
    some() { trace.push('some'); return ''; },
    get every() { throw new Error('every must not run'); },
    reduce(reducer, initial) {
      trace.push(`reduce:${initial}`);
      return { [Symbol.toPrimitive](hint) { trace.push(`coerce:${hint}`); return 0; } };
    },
  };
  const files = [];
  files.slice = () => items;
  // Act
  const result = await conversion(kind, files);
  // Assert
  assert.equal(result.payloads, items);
  assert.deepEqual(trace, ['some', 'reduce:0', 'coerce:number']);
  return trace;
});

paired('the reducer reads buffer twice and uses the second value for byteLength', async kind => {
  // Arrange
  const trace = [];
  let reads = 0;
  const first = { get byteLength() { throw new Error('first buffer byteLength must not run'); } };
  const second = { get byteLength() { trace.push('byteLength:second'); return 1; } };
  const item = { get buffer() { trace.push('buffer'); return ++reads === 1 ? first : second; } };
  // Act
  const result = await conversion(kind, item);
  // Assert
  assert.equal(result.payloads[0], item);
  assert.deepEqual(trace, ['buffer', 'buffer', 'byteLength:second']);
  return trace;
});

paired('falsy buffer performs only one getter read', async kind => {
  // Arrange
  let reads = 0;
  const item = { get buffer() { reads++; return 0; } };
  // Act
  const result = await conversion(kind, item);
  // Assert
  assert.equal(result.payloads[0], item);
  assert.equal(reads, 1);
  return { reads };
});

paired('reducer addition retains JavaScript string concatenation and primitive coercion', async kind => {
  // Arrange
  const trace = [];
  let captured;
  const item = { buffer: { byteLength: { [Symbol.toPrimitive](hint) { trace.push(hint); return '7'; } } } };
  const items = {
    some: () => false,
    reduce(reducer, initial) { captured = reducer(initial, item); return captured; },
  };
  const files = [];
  files.slice = () => items;
  // Act
  const result = await conversion(kind, files);
  // Assert
  assert.equal(result.payloads, items);
  assert.equal(captured, '07');
  assert.deepEqual(trace, ['default']);
  return { captured, trace };
});

paired('reducer mixed Number and BigInt rejects with JavaScript TypeError', async kind => {
  // Arrange
  const item = { buffer: { byteLength: 1n } };
  // Act
  const error = await conversion(kind, item).catch(value => value);
  // Assert: do not compare runtime-specific source excerpts in error messages.
  assert.ok(error instanceof TypeError);
  return { name: error.name };
});

paired('user coercion errors preserve original identity', async kind => {
  // Arrange
  const primary = Object.freeze({ original: 'addition coercion' });
  const item = { buffer: { byteLength: { [Symbol.toPrimitive]() { throw primary; } } } };
  // Act
  const error = await conversion(kind, item).catch(value => value);
  // Assert
  assert.equal(error, primary);
  return { identity: error === primary };
});

paired('escaped predicate and reducer remain callable with correct arity', async kind => {
  // Arrange
  let predicate;
  let reducer;
  const items = {
    some(callback) { predicate = callback; return false; },
    reduce(callback) { reducer = callback; return 0; },
  };
  const files = [];
  files.slice = () => items;
  // Act: invoke after the native callback scope and conversion await have ended.
  await conversion(kind, files);
  const sum = reducer(7, { buffer: { byteLength: 3 } }, 'ignored', 'ignored');
  const string = predicate('path', 'ignored');
  const missing = predicate();
  let emptyError;
  try { reducer(); } catch (error) { emptyError = error; }
  // Assert
  assert.equal(sum, 10);
  assert.equal(string, true);
  assert.equal(missing, false);
  assert.ok(emptyError instanceof TypeError);
  assert.deepEqual([predicate.length, reducer.length], [1, 2]);
  assert.equal(Object.hasOwn(reducer, 'prototype'), false);
  assert.throws(() => Reflect.construct(reducer, []), TypeError);
  return { sum, string, missing, length: [predicate.length, reducer.length], emptyError: emptyError.name };
});

for (const bytes of [50 * 1024 * 1024 - 1, 50 * 1024 * 1024]) {
  paired(`size threshold is checked without allocating ${bytes} bytes`, async kind => {
    // Arrange
    const item = { buffer: { byteLength: bytes } };
    // Act
    const outcome = await conversion(kind, item).then(value => ({ value }), error => ({ error }));
    // Assert
    if (bytes < 50 * 1024 * 1024) {
      assert.equal(outcome.value.payloads[0], item);
      return { accepted: true };
    }
    assert.equal(outcome.error.message, 'Cannot set buffer larger than 50Mb, please write it to a file and pass its path instead.');
    return { accepted: false, message: outcome.error.message };
  });
}

paired('conversion keeps Buffer identity and performs no byte snapshot or serializer call', async kind => {
  // Arrange
  const bytes = Buffer.from([1, 2, 3]);
  bytes.toString = () => { throw new Error('preflight must not serialize'); };
  const item = { name: 'bytes.bin', mimeType: 'application/octet-stream', buffer: bytes };
  // Act
  const result = await conversion(kind, item);
  bytes[0] = 9;
  // Assert
  assert.equal(result.payloads[0], item);
  assert.equal(result.payloads[0].buffer, bytes);
  assert.equal(result.payloads[0].buffer[0], 9);
  return { sameItem: result.payloads[0] === item, sameBuffer: result.payloads[0].buffer === bytes, first: bytes[0] };
});

paired('mixed strings and payloads fail before any Buffer getter', async kind => {
  // Arrange
  let reads = 0;
  const item = { get buffer() { reads++; throw new Error('size must not run'); } };
  // Act
  const error = await conversion(kind, ['file.txt', item]).catch(value => value);
  // Assert
  assert.equal(error.message, 'File paths cannot be mixed with buffers');
  assert.equal(reads, 0);
  return { message: error.message, reads };
});

test('offline preflight kernel: valid path-only input stops at the explicit unsupported boundary', { timeout: 5000 }, () => {
  // Arrange: classification only. There is no path stat, file read or upload.
  const context = {};
  // Act
  let error;
  try { native.inspectFileInputPreflightConversion(context, ['not-read.txt'], ports); }
  catch (value) { error = value; }
  // Assert
  assert.ok(error instanceof Error);
  assert.match(error.message, /PWRIGHT_UNSUPPORTED/);
});
