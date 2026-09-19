// Offline language/timeout-kernel differential only. Actual pinned client and
// Frame/TimeoutSettings bodies run against metadata plus a channel-argument
// capture seam. Candidate adapters invoke real stateless N-API kernels, not
// actual preparation tickets/session authority. No channel validation, native
// deadlines, filesystem, upload submission or browser behavior is exercised.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { ports, argumentPorts, preflight } = require('../src/file-input-preflight.cjs');
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
for (const name of ['inspectFileInputPreflightOwner', 'inspectFileInputPreflightContext',
  'inspectFileInputPreflightConversion', 'inspectFileInputArguments', 'NativeFileInputTimeoutSettings'])
  assert.equal(typeof native[name], 'function', `selected artifact exports ${name}`);
assert.equal(typeof argumentPorts, 'object', 'selected language argument ports exist');

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const load = path => requireCore(join(core, 'lib', path));
const { ElementHandle } = load('client/elementHandle.js');
const { Frame } = load('client/frame.js');
const { TimeoutSettings } = load('client/timeoutSettings.js');
const { nodePlatform } = load('server/utils/nodePlatform.js');
const environment = new native.NativeEnvironment(setImmediate,
  Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength').get);
assert.equal(typeof environment.createFileInputTimeoutSettings, 'function');

function paired(name, exercise) {
  test(`offline retained arguments: ${name}`, { timeout: 5000 }, async () => {
    // Arrange/Act: independent settings and language values for each side.
    const expected = await exercise('oracle');
    const actual = await exercise('candidate');
    // Assert
    assert.deepEqual(actual, expected);
  });
}

function settings(kind, { debug = false, contextDefault, pageDefault } = {}) {
  const platform = { ...nodePlatform, isDebugMode: () => debug };
  const context = kind === 'oracle' ? new TimeoutSettings(platform) : environment.createFileInputTimeoutSettings(debug);
  const page = kind === 'oracle' ? new TimeoutSettings(platform, context) : context.child();
  context.setDefaultTimeout(contextDefault);
  page.setDefaultTimeout(pageDefault);
  return { context, page, platform };
}

function metadataFrame(state) {
  const frame = Object.create(Frame.prototype);
  frame._platform = state.platform;
  frame._page = { _timeoutSettings: state.page, context: () => ({}) };
  return frame;
}

function whole(kind, state, files, options) {
  const creation = metadataFrame(state);
  // Owner metadata is deliberately separate from the creation Frame's settings.
  const owner = { page: () => ({ context: () => ({}) }) };
  const captured = { calls: 0, params: undefined, aborts: 0 };
  const receiver = {
    ownerFrame: () => owner,
    _platform: state.platform,
    _frame: creation,
    _elementChannel: { async setInputFiles(params) { captured.calls++; captured.params = params; } },
  };
  if (kind === 'oracle') {
    return { captured, promise: Reflect.apply(ElementHandle.prototype.setInputFiles, receiver, [files, options]) };
  }
  let context;
  let converted;
  // Stateless diagnostic adapter only: no fake native session or ticket is
  // presented as authority. The real exported driver owns both await points.
  const preparation = {
    beginOwnerLookup(language) { return native.inspectFileInputPreflightOwner(receiver, language); },
    prepareContext(value, language) { context = native.inspectFileInputPreflightContext(value, language); },
    convertInput(language) { return native.inspectFileInputPreflightConversion(context, files, language); },
    finishConversion(value) { converted = value; },
    assembleArguments(language) {
      captured.params = native.inspectFileInputArguments(converted, options, state.page, language);
      captured.calls++;
      return captured.params;
    },
    abort() { captured.aborts++; },
  };
  return { captured, promise: preflight(preparation) };
}

function direct(kind, state, converted, options) {
  if (kind === 'candidate') return native.inspectFileInputArguments(converted, options, state.page, argumentPorts);
  // Explicit direct-kernel oracle: the pinned client's argument expression,
  // using the real Frame timeout method and real TimeoutSettings hierarchy.
  // Unlike whole(), this does not exercise upstream conversion/await handling.
  return { ...converted, ...options, timeout: metadataFrame(state)._timeout(options) };
}

test('offline retained arguments: argument language ports are frozen and minimal', { timeout: 5000 }, () => {
  // Arrange/Act
  const names = Object.keys(argumentPorts);
  // Assert
  assert.equal(Object.isFrozen(argumentPorts), true);
  assert.deepEqual(names, ['spread', 'timeout']);
  for (const name of names) assert.equal(typeof argumentPorts[name], 'function');
  assert.equal(typeof ports.owner_frame, 'function');
});

test('offline retained arguments: timeout settings use actual module-local raw class guards', { timeout: 5000 }, () => {
  // Arrange
  const state = settings('candidate');
  const className = 'NativeFileInputTimeoutSettings';
  const forged = Object.create(native.NativeFileInputTimeoutSettings.prototype);
  const proxy = new Proxy(state.page, {});
  // Act
  const names = native.nativeClassNames();
  // Assert: prove brand rejection before any argument layout could be read.
  assert.equal(names.filter(name => name === className).length, 1);
  assert.equal(native.nativeClassMatches(className, state.context), true);
  assert.equal(native.nativeClassMatches(className, state.page), true);
  assert.equal(native.nativeClassMatches(className, forged), false);
  assert.equal(native.nativeClassMatches(className, proxy), false);
  assert.throws(() => native.inspectFileInputArguments({}, {}, forged, argumentPorts));
  assert.throws(() => native.inspectFileInputArguments({}, {}, proxy, argumentPorts));
  assert.throws(() => new native.NativeFileInputTimeoutSettings());
  assert.equal(typeof state.page.child, 'function');
  assert.throws(() => state.page.child(), 'normal timeout scope is Context then Page, not a third child');
});

for (const [label, config, expected] of [
  ['builtin default', {}, 30000],
  ['Context inheritance', { contextDefault: 37 }, 37],
  ['Page zero', { contextDefault: 37, pageDefault: 0 }, 0],
  ['Page fraction', { contextDefault: 37, pageDefault: 1.5 }, 1.5],
  ['debug before defaults', { debug: true, contextDefault: 37, pageDefault: 91 }, 0],
]) {
  paired(label, async kind => {
    // Arrange
    const state = settings(kind, config);
    // Act
    const run = whole(kind, state, [], {});
    await run.promise;
    // Assert
    assert.equal(run.captured.calls, 1);
    assert.equal(run.captured.params.timeout, expected);
    return run.captured.params;
  });
}

for (const [label, timeout] of [['zero', 0], ['negative zero', -0], ['fraction', 1.5], ['negative', -1],
  ['NaN', NaN], ['infinity', Infinity], ['negative infinity', -Infinity]]) {
  paired(`raw ${label} option is preserved before channel validation`, async kind => {
    // Arrange: this is not a statement about server/native deadline support.
    const state = settings(kind, { debug: true, contextDefault: 23, pageDefault: 41 });
    // Act
    const run = whole(kind, state, [], { timeout });
    await run.promise;
    // Assert
    assert.equal(run.captured.calls, 1);
    assert.equal(Object.is(run.captured.params.timeout, timeout), true);
    return run.captured.params;
  });
}

for (const [label, options] of [['undefined', undefined], ['null', null], ['false', false], ['zero', 0],
  ['number', 7], ['string', 'hi'], ['symbol', Symbol('options')], ['bigint', 1n]]) {
  paired(`${label} options retain spread and falsy normalization semantics`, async kind => {
    // Arrange
    const state = settings(kind, { pageDefault: 111 });
    // Act
    const run = whole(kind, state, [], options);
    await run.promise;
    // Assert
    assert.equal(run.captured.calls, 1);
    assert.equal(run.captured.params.timeout, 111);
    assert.deepEqual(Object.keys(run.captured.params), label === 'string' ? ['0', '1', 'payloads', 'timeout'] : ['payloads', 'timeout']);
    return run.captured.params;
  });
}

paired('options proxy order, enumerable symbols and existing timeout key position', async kind => {
  // Arrange
  const state = settings(kind);
  const trace = [];
  const symbol = Symbol('option');
  const target = Object.create(null);
  Object.defineProperties(target, {
    timeout: { value: 17, configurable: true, enumerable: true },
    hidden: { value: 'not copied', configurable: true, enumerable: false },
    label: { value: 'copied', configurable: true, enumerable: true },
    [symbol]: { value: 'symbol value', configurable: true, enumerable: true },
  });
  const label = key => key === symbol ? 'symbol' : key;
  const options = new Proxy(target, {
    ownKeys() { trace.push('keys'); return [symbol, 'timeout', 'hidden', 'label']; },
    getOwnPropertyDescriptor(object, key) { trace.push(`descriptor:${label(key)}`); return Reflect.getOwnPropertyDescriptor(object, key); },
    get(object, key, receiver) { trace.push(`get:${label(key)}`); return Reflect.get(object, key, receiver); },
  });
  // Act
  const run = whole(kind, state, [], options);
  await run.promise;
  // Assert
  assert.equal(run.captured.calls, 1);
  assert.deepEqual(trace, ['keys', 'descriptor:symbol', 'get:symbol', 'descriptor:timeout', 'get:timeout',
    'descriptor:hidden', 'descriptor:label', 'get:label', 'get:timeout', 'get:timeout']);
  assert.deepEqual(Reflect.ownKeys(run.captured.params), ['payloads', 'timeout', 'label', symbol]);
  assert.equal(run.captured.params[symbol], 'symbol value');
  return { trace, timeout: run.captured.params.timeout, symbol: run.captured.params[symbol] };
});

paired('options payload replacement is not sized or copied again', async kind => {
  // Arrange
  const state = settings(kind);
  let reads = 0;
  const source = { get buffer() { reads++; return Buffer.from([1]); } };
  const bytes = Buffer.from([9]);
  Object.defineProperty(bytes, 'byteLength', { get() { throw new Error('replacement size must not run'); } });
  const replacement = [{ name: 'replacement', buffer: bytes }];
  // Act
  const run = whole(kind, state, source, { get payloads() { return replacement; } });
  await run.promise;
  // Assert
  assert.equal(reads, 2);
  assert.equal(run.captured.params.payloads, replacement);
  assert.equal(run.captured.params.payloads[0].buffer, bytes);
  return { reads, sameArray: run.captured.params.payloads === replacement, byte: bytes[0] };
});

paired('numeric typeof check returns a changed third getter value without conversion', async kind => {
  // Arrange
  const state = settings(kind);
  const returned = { raw: 'not a number' };
  const values = ['spread', 1, returned];
  let reads = 0;
  const options = { get timeout() { return values[reads++]; } };
  // Act
  const run = whole(kind, state, [], options);
  await run.promise;
  // Assert
  assert.equal(reads, 3);
  assert.equal(run.captured.params.timeout, returned);
  return { reads, same: run.captured.params.timeout === returned };
});

paired('Context fallback rereads the original options object', async kind => {
  // Arrange
  const state = settings(kind, { contextDefault: 73 });
  const values = ['spread', undefined, 1, 'raw Context return'];
  let reads = 0;
  const options = { get timeout() { return values[reads++]; } };
  // Act
  const run = whole(kind, state, [], options);
  await run.promise;
  // Assert
  assert.equal(reads, 4);
  assert.equal(run.captured.params.timeout, 'raw Context return');
  return { reads, timeout: run.captured.params.timeout };
});

paired('an options getter updates the live Context default before timeout resolution', async kind => {
  // Arrange
  const state = settings(kind, { contextDefault: 37 });
  const options = { get marker() { state.context.setDefaultTimeout(73); return true; } };
  // Act
  const run = whole(kind, state, [], options);
  await run.promise;
  // Assert
  assert.equal(run.captured.params.timeout, 73);
  return run.captured.params;
});

paired('a timeout getter updates its own live Page default after lookup has begun', async kind => {
  // Arrange
  const state = settings(kind, { contextDefault: 37, pageDefault: 17 });
  let reads = 0;
  const options = { get timeout() {
    if (++reads === 2) state.page.setDefaultTimeout(29);
    return undefined;
  } };
  // Act
  const run = whole(kind, state, [], options);
  await run.promise;
  // Assert: the getter must not run under a settings RefCell borrow.
  assert.equal(reads, 2);
  assert.equal(run.captured.params.timeout, 29);
  return { reads, timeout: run.captured.params.timeout };
});

paired('clearing Page default and mutating Context during its getter uses live inheritance', async kind => {
  // Arrange
  const state = settings(kind, { contextDefault: 41, pageDefault: 88 });
  let reads = 0;
  const options = { get timeout() {
    reads++;
    if (reads === 2) state.page.setDefaultTimeout(undefined);
    if (reads === 3) state.context.setDefaultTimeout(73);
    return undefined;
  } };
  // Act
  const run = whole(kind, state, [], options);
  await run.promise;
  // Assert
  assert.equal(reads, 3);
  assert.equal(run.captured.params.timeout, 73);
  return { reads, timeout: run.captured.params.timeout };
});

for (const phase of ['ownKeys', 'descriptor', 'symbol-getter', 'timeout-return']) {
  paired(`${phase} error keeps its original identity before channel wrapping`, async kind => {
    // Arrange
    const state = settings(kind);
    const primary = new Error(`${phase} assembly failure`);
    const symbol = Symbol('option');
    let reads = 0;
    const target = {
      get [symbol]() { if (phase === 'symbol-getter') throw primary; return true; },
      get timeout() { if (++reads === 3 && phase === 'timeout-return') throw primary; return 1; },
    };
    const options = new Proxy(target, {
      ownKeys(object) { if (phase === 'ownKeys') throw primary; return Reflect.ownKeys(object); },
      getOwnPropertyDescriptor(object, key) { if (phase === 'descriptor') throw primary; return Reflect.getOwnPropertyDescriptor(object, key); },
    });
    // Act
    const run = whole(kind, state, [], options);
    const error = await run.promise.then(() => assert.fail('must reject'), value => value);
    // Assert
    assert.equal(error, primary);
    assert.equal(primary.message, `${phase} assembly failure`);
    assert.equal(run.captured.calls, 0);
    if (kind === 'candidate') assert.equal(run.captured.aborts, 1);
    return { same: error === primary, message: primary.message, reads };
  });
}

paired('direct kernel copies converted proxy before options proxy without retaining getters', async kind => {
  // Arrange
  const state = settings(kind, { pageDefault: 111 });
  const trace = [];
  const proxy = (prefix, target) => new Proxy(target, {
    ownKeys(object) { trace.push(`${prefix}:keys`); return Reflect.ownKeys(object); },
    getOwnPropertyDescriptor(object, key) { trace.push(`${prefix}:descriptor:${key}`); return Reflect.getOwnPropertyDescriptor(object, key); },
    get(object, key, receiver) { trace.push(`${prefix}:get:${key}`); return Reflect.get(object, key, receiver); },
  });
  const converted = proxy('converted', { payloads: ['original'], timeout: -1 });
  const options = proxy('options', { payloads: ['replacement'], extra: true });
  // Act
  const params = direct(kind, state, converted, options);
  // Assert
  assert.deepEqual(trace, ['converted:keys', 'converted:descriptor:payloads', 'converted:get:payloads',
    'converted:descriptor:timeout', 'converted:get:timeout', 'options:keys', 'options:descriptor:payloads',
    'options:get:payloads', 'options:descriptor:extra', 'options:get:extra', 'options:get:timeout']);
  assert.deepEqual(params, { payloads: ['replacement'], timeout: 111, extra: true });
  assert.equal(Object.getOwnPropertyDescriptor(params, 'payloads').get, undefined);
  return { trace, params };
});

paired('whole driver assembles immediately after conversion assimilation without an extra await', async kind => {
  // Arrange: the poison targets only the one-property converted payload carrier.
  const state = settings(kind);
  const trace = [];
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  const converted = Object.create(null);
  Object.defineProperty(converted, 'payloads', { enumerable: true, get() { trace.push('converted'); return []; } });
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (!Object.hasOwn(this, 'payloads') || Reflect.ownKeys(this).length !== 1) return undefined;
    trace.push('then:get');
    return resolve => { trace.push('then:call'); resolve(converted); queueMicrotask(() => trace.push('after:resolve')); };
  } });
  try {
    const options = { get marker() { trace.push('options'); return true; } };
    // Act
    const run = whole(kind, state, [], options);
    await run.promise;
    // Assert
    assert.equal(run.captured.calls, 1);
    assert.deepEqual(trace, ['then:get', 'then:call', 'converted', 'options', 'after:resolve']);
    return trace;
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});

paired('assimilated converted getter error happens before any options getter', async kind => {
  // Arrange
  const state = settings(kind);
  const primary = Object.freeze({ original: 'converted getter' });
  let reads = 0;
  const converted = Object.create(null);
  Object.defineProperty(converted, 'payloads', { enumerable: true, get() { throw primary; } });
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (!Object.hasOwn(this, 'payloads') || Reflect.ownKeys(this).length !== 1) return undefined;
    return resolve => resolve(converted);
  } });
  try {
    const options = { get timeout() { reads++; return 1; } };
    // Act
    const run = whole(kind, state, [], options);
    const error = await run.promise.then(() => assert.fail('must reject'), value => value);
    // Assert
    assert.equal(error, primary);
    assert.equal(reads, 0);
    assert.equal(run.captured.calls, 0);
    if (kind === 'candidate') assert.equal(run.captured.aborts, 1);
    return { same: error === primary, reads };
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});
