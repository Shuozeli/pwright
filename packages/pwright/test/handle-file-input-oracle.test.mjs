// Offline characterization, not browser/native conformance. The real pinned
// ElementHandle, Frame, Connection, channel validators and upload preparation
// run below. Only owner replies, page/context metadata and transport delivery
// are fixtures; no Browser, CDP connection, native addon or mutation is created.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const load = path => requireCore(join(core, 'lib', path));
const { ElementHandle } = load('client/elementHandle.js');
const { Frame } = load('client/frame.js');
const { Connection } = load('client/connection.js');
const { TimeoutSettings } = load('client/timeoutSettings.js');
const { nodePlatform } = load('server/utils/nodePlatform.js');
const { findValidator } = load('protocol/validator.js');
const { serializeError } = load('server/errors.js');
const { prepareFilesForUpload } = load('server/fileUploadUtils.js');
const fixture = fileURLToPath(new URL('../../../crates/pwright-file-input-source/tests/fixture.txt', import.meta.url));

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

function oracle(t, options = {}) {
  const trace = [];
  const sent = [];
  const prepared = [];
  const access = [];
  const ownerArrived = deferred();
  const immediates = new Set();
  let closed = false;
  const platform = {
    ...nodePlatform,
    // Keep ambient debug/test flags from adding logging/getter passes.
    isUnderTest: () => false, isDebugMode: () => false, isLogEnabled: () => false,
    fs: () => ({ promises: { stat: async path => {
      trace.push('path:stat');
      return nodePlatform.fs().promises.stat(path);
    } } }),
  };
  const connection = new Connection(platform);
  connection.useRawBuffers();
  const frame = (id, timeout) => {
    const result = new Frame(connection, 'Frame', id, { url: 'about:blank', name: id, loadStates: [] });
    const context = {
      _connection: connection,
      _checkFileAccess: path => access.push({ context: id, path }),
    };
    const settings = new TimeoutSettings(platform);
    settings.setDefaultTimeout(timeout);
    result._page = { context: () => context, _timeoutSettings: settings };
    return result;
  };
  const creation = frame('creation-frame', 111);
  const owner = frame('owner-frame', 222);
  const handle = new ElementHandle(creation, 'ElementHandle', 'retained-element', { preview: 'JSHandle@node' });
  const serverFrame = { _page: { browserContext: { _browser: { _isCollocatedWithServer: false } } } };
  const serverValidation = {
    binary: 'buffer', isUnderTest: () => false,
    tChannelImpl() { throw new Error('stream channels are outside this offline fixture'); },
  };
  const schedule = action => {
    const token = setImmediate(() => { immediates.delete(token); if (!closed) action(); });
    immediates.add(token);
  };
  const respond = message => schedule(() => connection.dispatch(message));
  connection.onmessage = message => {
    assert.equal(sent.length < 8, true, 'offline request bound');
    sent.push(message);
    trace.push(`send:${message.method}`);
    // Same deferred raw-Buffer transport shape as lib/inProcessFactory.js.
    // This seam does not model server ProgressController or DOM execution.
    schedule(() => {
      (async () => {
        assert.equal(message.guid, 'retained-element');
        if (message.method === 'ownerFrame') {
          ownerArrived.resolve();
          if (options.ownerGate) await options.ownerGate;
          if (options.ownerError) throw options.ownerError;
          trace.push('owner:reply');
          respond({ id: message.id, result: options.noOwner ? {} : { frame: { guid: owner._guid } } });
        } else if (message.method === 'dispose') {
          // JSHandleDispatcher sends __dispose__ before its command response.
          // Exercise real client metadata retirement, not native DOM disposal.
          respond({ guid: message.guid, method: '__dispose__', params: {} });
          respond({ id: message.id, result: {} });
        } else {
          assert.equal(message.method, 'setInputFiles');
          const params = findValidator('ElementHandle', 'setInputFiles', 'Params')(message.params, '', serverValidation);
          trace.push('server:materialize');
          prepared.push(await prepareFilesForUpload(serverFrame, params));
          trace.push('server:prepared');
          respond({ id: message.id, result: {} });
        }
      })().catch(error => respond({ id: message.id, error: serializeError(error) }));
    });
  };
  t.after(() => {
    const outstanding = connection._callbacks.size;
    closed = true;
    for (const token of immediates) clearImmediate(token);
    connection.close('offline fixture cleanup');
    assert.equal(outstanding, 0, 'offline channel callbacks settled before fixture cleanup');
  });
  return { handle, owner, creation, connection, trace, sent, prepared, access, ownerArrived: ownerArrived.promise };
}

function payload(buffer = Buffer.from([1, 2, 3])) {
  return { name: 'bytes.bin', mimeType: 'application/octet-stream', buffer };
}
function offline(name, exercise) {
  test(`pinned offline retained upload: ${name}`, { timeout: 5000 }, exercise);
}

function argumentAssemblyBoundary(h) {
  // Test-only stop immediately after the real client's object-literal argument
  // assembly. This seam observes raw values before channel validation; it is not
  // a substitute for the separate real-channel/server cases below.
  const captured = [];
  h.handle.ownerFrame = async () => h.owner;
  h.handle._elementChannel = { async setInputFiles(params) { captured.push(params); } };
  return captured;
}

offline('all size getters precede option getters and channel field validation', async t => {
  // Arrange
  const h = oracle(t);
  const file = name => {
    const buffer = Buffer.from([1]);
    Object.defineProperty(buffer, 'byteLength', { get() { h.trace.push(`${name}:byteLength`); return 1; } });
    return {
      get name() { h.trace.push(`${name}:name`); return `${name}.bin`; },
      get mimeType() { h.trace.push(`${name}:mimeType`); return 'application/octet-stream'; },
      get buffer() { h.trace.push(`${name}:buffer`); return buffer; },
    };
  };
  const files = [file('first'), file('second')];
  const options = {
    get unknown() { h.trace.push('option:unknown'); return 1; },
    get noWaitAfter() { h.trace.push('option:noWaitAfter'); return 'ignored by schema'; },
    get timeout() { h.trace.push('option:timeout'); return 17; },
  };
  // Act
  await h.handle.setInputFiles(files, options);
  // Assert
  assert.deepEqual(h.trace, [
    'send:ownerFrame', 'owner:reply',
    'first:buffer', 'first:buffer', 'first:byteLength',
    'second:buffer', 'second:buffer', 'second:byteLength',
    'option:unknown', 'option:noWaitAfter', 'option:timeout', 'option:timeout', 'option:timeout',
    'first:name', 'first:mimeType', 'first:buffer', 'second:name', 'second:mimeType', 'second:buffer',
    'send:setInputFiles', 'server:materialize', 'server:prepared',
  ]);
  assert.deepEqual(Object.keys(h.sent[1].params), ['payloads', 'timeout']);
  assert.equal(h.sent[1].params.timeout, 17);
});

offline('awaits owner reply before any files or options getters', async t => {
  // Arrange
  const gate = deferred();
  const h = oracle(t, { ownerGate: gate.promise });
  let reads = 0;
  const files = { get buffer() { reads++; return Buffer.from([1]); }, name: 'a.bin' };
  const options = { get timeout() { reads++; return 10; } };
  // Act
  const pending = h.handle.setInputFiles(files, options);
  const observed = pending.catch(error => { throw error; });
  try {
    await h.ownerArrived;
    // Assert the actual channel call is waiting, not merely unstarted.
    assert.equal(reads, 0);
    assert.deepEqual(h.sent.map(message => message.method), ['ownerFrame']);
  } finally { gate.resolve(); }
  await observed;
  assert.ok(reads > 0);
});

for (const failure of ['null', 'channel-error']) {
  offline(`${failure} owner failure precedes payload and options errors`, async t => {
    // Arrange
    const h = oracle(t, { noOwner: failure === 'null', ownerError: failure === 'channel-error' ? new Error('owner unavailable') : undefined });
    let reads = 0;
    const files = { get buffer() { reads++; throw new Error('payload must not run'); } };
    const options = { get timeout() { reads++; throw new Error('options must not run'); } };
    // Act
    const result = h.handle.setInputFiles(files, options);
    // Assert
    await assert.rejects(result, { message: failure === 'null'
      ? 'Cannot set input files to detached element' : 'elementHandle.setInputFiles: owner unavailable' });
    assert.equal(reads, 0);
    assert.equal(h.sent.length, 1);
    assert.deepEqual(h.prepared, []);
  });
}

offline('size-pass getter failure preserves identity without channel prefix', async t => {
  // Arrange
  const h = oracle(t);
  const original = new Error('size getter failed');
  let optionsReads = 0;
  const files = { get buffer() { throw original; }, get name() { throw new Error('name must not run'); } };
  // Act
  const result = h.handle.setInputFiles(files, { get timeout() { optionsReads++; return 1; } });
  // Assert
  await assert.rejects(result, error => error === original);
  assert.equal(original.message, 'size getter failed');
  assert.equal(optionsReads, 0);
  assert.equal(h.sent.length, 1);
});

offline('option getter failure wins over invalid payload name and keeps identity', async t => {
  // Arrange
  const h = oracle(t);
  const original = new Error('options getter failed');
  const files = { ...payload(), name: 42 };
  // Act
  const result = h.handle.setInputFiles(files, { get unknown() { throw original; } });
  // Assert
  await assert.rejects(result, error => error === original);
  assert.equal(original.message, 'options getter failed');
  assert.equal(h.sent.length, 1);
});

offline('channel payload getter error preserves identity and receives API prefix', async t => {
  // Arrange
  const h = oracle(t);
  const original = new Error('name getter failed');
  const files = { ...payload(), get name() { throw original; } };
  // Act
  const result = h.handle.setInputFiles(files, { timeout: 1 });
  // Assert
  await assert.rejects(result, error => error === original);
  assert.equal(original.message, 'elementHandle.setInputFiles: name getter failed');
  assert.equal(h.sent.length, 1);
});

for (const phase of ['size', 'channel']) {
  offline(`${phase} primitive throw follows the real channel wrapping boundary`, async t => {
    // Arrange
    const h = oracle(t);
    const files = phase === 'size' ? { get buffer() { throw 'primitive failure'; } }
      : { ...payload(), get name() { throw 'primitive failure'; } };
    // Act
    const error = await h.handle.setInputFiles(files).then(() => assert.fail('must reject'), value => value);
    // Assert
    if (phase === 'size') assert.equal(error, 'primitive failure');
    else { assert.ok(error instanceof TypeError); assert.match(error.message, /message/); }
    assert.equal(h.sent.length, 1);
  });
}

offline('conversion await exposes a size-getter microtask before option/channel reads', async t => {
  // Arrange
  const h = oracle(t);
  const buffer = Buffer.from([1]);
  let reads = 0;
  let name = 'before.bin';
  const files = {
    get buffer() {
      if (++reads === 1) queueMicrotask(() => { name = 'after.bin'; h.trace.push('microtask:name'); });
      return buffer;
    },
    get name() { h.trace.push('channel:name'); return name; },
  };
  // Act
  await h.handle.setInputFiles(files, { get timeout() { h.trace.push('option:timeout'); return 1; } });
  // Assert
  assert.ok(h.trace.indexOf('microtask:name') < h.trace.indexOf('option:timeout'));
  assert.ok(h.trace.indexOf('option:timeout') < h.trace.indexOf('channel:name'));
  assert.equal(h.prepared[0].filePayloads[0].name, 'after.bin');
});

offline('raw Buffer bytes remain mutable until deferred server preparation', async t => {
  // Arrange
  const h = oracle(t);
  const buffer = Buffer.from([1, 2]);
  let reads = 0;
  const intrinsicToString = buffer.toString;
  buffer.toString = function (...args) { h.trace.push('buffer:serialize'); return Reflect.apply(intrinsicToString, this, args); };
  const files = { name: 'mutable.bin', get buffer() {
    if (++reads === 3) queueMicrotask(() => { buffer[0] = 9; h.trace.push('microtask:bytes'); });
    return buffer;
  } };
  // Act
  await h.handle.setInputFiles(files);
  // Assert
  assert.equal(h.sent[1].params.payloads[0].buffer, buffer, 'client channel forwards the same Buffer');
  assert.ok(h.trace.indexOf('send:setInputFiles') < h.trace.indexOf('microtask:bytes'));
  assert.ok(h.trace.indexOf('microtask:bytes') < h.trace.indexOf('server:materialize'));
  assert.ok(h.trace.indexOf('server:materialize') < h.trace.indexOf('buffer:serialize'));
  assert.equal(h.trace.filter(event => event === 'buffer:serialize').length, 1);
  assert.deepEqual([...Buffer.from(h.prepared[0].filePayloads[0].buffer, 'base64')], [9, 2]);
});

offline('a later metadata getter mutates earlier payload bytes before snapshot', async t => {
  // Arrange
  const h = oracle(t);
  const first = payload(Buffer.from([1]));
  const second = { ...payload(), get name() { first.buffer[0] = 7; return 'later.bin'; } };
  // Act
  await h.handle.setInputFiles([first, second]);
  // Assert
  assert.deepEqual([...Buffer.from(h.prepared[0].filePayloads[0].buffer, 'base64')], [7]);
});

offline('paths stat before options, use owner Context, and materialize afterward', async t => {
  // Arrange
  const h = oracle(t);
  const options = { get unknown() { h.trace.push('option:unknown'); return 1; } };
  // Act
  await h.handle.setInputFiles(fixture, options);
  // Assert
  assert.deepEqual(h.trace, ['send:ownerFrame', 'owner:reply', 'path:stat', 'option:unknown', 'send:setInputFiles', 'server:materialize', 'server:prepared']);
  assert.deepEqual(h.access, [{ context: 'owner-frame', path: fixture }]);
  assert.equal(h.sent[1].params.timeout, 111, 'timeout uses creation Frame, not owner Frame');
  assert.deepEqual(h.sent[1].params.localPaths, [fixture]);
  assert.equal(h.sent[1].params.payloads, undefined);
  assert.equal(h.prepared[0].localPaths, undefined);
  assert.equal(h.prepared[0].filePayloads[0].name, 'fixture.txt');
  assert.ok(Buffer.from(h.prepared[0].filePayloads[0].buffer, 'base64').length > 0);
});

offline('bad path rejection precedes options getters', async t => {
  // Arrange
  const h = oracle(t);
  let reads = 0;
  // Act
  const result = h.handle.setInputFiles(`${fixture}.missing-node169`, { get timeout() { reads++; throw new Error('options must not run'); } });
  // Assert
  await assert.rejects(result, { code: 'ENOENT' });
  assert.equal(reads, 0);
  assert.equal(h.sent.length, 1);
  assert.deepEqual(h.prepared, []);
});

offline('protocol-field options overwrite converted payloads after their size pass', async t => {
  // Arrange
  const h = oracle(t);
  const replacement = payload(Buffer.from([9]));
  let originalReads = 0;
  const original = { name: 42, get buffer() { originalReads++; return Buffer.from([1]); } };
  // Act
  await h.handle.setInputFiles(original, { payloads: [replacement], timeout: 1.5 });
  // Assert
  assert.equal(originalReads, 2, 'original input was sized, but replacement was channel-validated');
  assert.equal(h.sent[1].params.timeout, 1.5, 'client timeout accepts a fractional number');
  assert.equal(h.sent[1].params.payloads[0].buffer, replacement.buffer);
  assert.deepEqual([...Buffer.from(h.prepared[0].filePayloads[0].buffer, 'base64')], [9]);
});

offline('public ownerFrame override is invoked by the real client method', async t => {
  // Arrange
  const h = oracle(t);
  let calls = 0;
  h.handle.ownerFrame = async () => { calls++; return h.owner; };
  // Act
  await h.handle.setInputFiles(payload());
  // Assert
  assert.equal(calls, 1);
  assert.deepEqual(h.sent.map(message => message.method), ['setInputFiles']);
});

for (const kind of ['Buffer', 'empty']) {
  offline(`overridden owner page/context calls precede ${kind} file inspection`, async t => {
    // Arrange
    const h = oracle(t);
    const page = h.owner.page();
    const context = page.context();
    h.handle.ownerFrame = async () => { h.trace.push('owner:override'); return h.owner; };
    h.owner.page = function () {
      assert.equal(this, h.owner);
      h.trace.push('owner:page');
      return page;
    };
    page.context = function () {
      assert.equal(this, page);
      h.trace.push('owner:context');
      return context;
    };
    const files = kind === 'Buffer' ? [payload()] : [];
    Object.defineProperty(files, 'slice', { get() {
      h.trace.push('files:slice');
      return Array.prototype.slice;
    } });
    // Act
    await h.handle.setInputFiles(files);
    // Assert
    assert.deepEqual(h.trace.slice(0, 4), ['owner:override', 'owner:page', 'owner:context', 'files:slice']);
    assert.deepEqual(h.sent.map(message => message.method), ['setInputFiles']);
    assert.equal(h.prepared[0].filePayloads.length, kind === 'Buffer' ? 1 : 0);
  });
}

offline('overridden owner page/context failures preserve identity before files/options', async t => {
  for (const phase of ['page', 'context']) {
    // Arrange
    const h = oracle(t);
    const primary = new Error(`${phase} lookup failed`);
    const reads = [];
    h.handle.ownerFrame = async () => h.owner;
    h.owner.page = () => {
      reads.push('page');
      if (phase === 'page') throw primary;
      return { context() { reads.push('context'); throw primary; } };
    };
    const files = [];
    Object.defineProperty(files, 'slice', { get() { throw new Error('files must not run'); } });
    const options = { get timeout() { throw new Error('options must not run'); } };
    // Act
    const result = h.handle.setInputFiles(files, options);
    // Assert
    await assert.rejects(result, error => error === primary);
    assert.equal(primary.message, `${phase} lookup failed`);
    assert.deepEqual(reads, phase === 'page' ? ['page'] : ['page', 'context']);
    assert.deepEqual(h.sent, []);
  }
});

offline('empty-input conversion still yields to microtasks before options', async t => {
  // Arrange
  const h = oracle(t);
  const files = [];
  Object.defineProperty(files, 'slice', { get() {
    h.trace.push('files:slice');
    queueMicrotask(() => h.trace.push('conversion:microtask'));
    return Array.prototype.slice;
  } });
  const options = { get unknown() { h.trace.push('option:unknown'); return true; } };
  // Act
  await h.handle.setInputFiles(files, options);
  // Assert
  assert.deepEqual(h.trace, ['send:ownerFrame', 'owner:reply', 'files:slice', 'conversion:microtask',
    'option:unknown', 'send:setInputFiles', 'server:materialize', 'server:prepared']);
  assert.deepEqual(h.prepared[0].filePayloads, []);
});

offline('size-getter disposal reentry does not bypass later client validation', async t => {
  // Arrange
  const h = oracle(t);
  const primary = new Error('later name getter failed');
  let disposal;
  let reads = 0;
  const buffer = Buffer.from([1]);
  const files = {
    get buffer() {
      if (++reads === 1)
        disposal = h.handle.dispose().then(() => ({ ok: true }), error => ({ error }));
      return buffer;
    },
    get name() { throw primary; },
  };
  let disposed;
  // Act
  const result = h.handle.setInputFiles(files);
  try {
    // Assert the real channel validator still runs after dispose was requested.
    await assert.rejects(result, error => error === primary);
  } finally {
    disposed = await disposal;
  }
  assert.deepEqual(disposed, { ok: true });
  assert.equal(reads, 2);
  assert.equal(primary.message, 'elementHandle.setInputFiles: later name getter failed');
  assert.deepEqual(h.sent.map(message => message.method), ['ownerFrame', 'dispose']);
  assert.equal(h.connection.getObjectWithKnownName('retained-element'), undefined,
    'the actual Connection applied the dispatcher-shaped disposal event');
  assert.deepEqual(h.prepared, []);
});

offline('server preparation uses the Buffer custom base64 serializer output', async t => {
  // Arrange: the actual helper invokes this method; raw-copy substitution would
  // produce different bytes. No browser or candidate semantics are asserted.
  const h = oracle(t);
  const buffer = Buffer.from([1, 2]);
  const calls = [];
  buffer.toString = function (...args) {
    calls.push({ receiver: this, args });
    return Buffer.from([9, 8]).toString('base64');
  };
  // Act
  await h.handle.setInputFiles(payload(buffer));
  // Assert
  assert.equal(calls.length, 1);
  assert.equal(calls[0].receiver, buffer);
  assert.deepEqual(calls[0].args, ['base64']);
  assert.deepEqual([...buffer], [1, 2]);
  assert.deepEqual([...Buffer.from(h.prepared[0].filePayloads[0].buffer, 'base64')], [9, 8]);
});

offline('server serializer errors roundtrip through the real error transport', async t => {
  // Arrange
  const h = oracle(t);
  const primary = new Error('custom Buffer serializer failed');
  const buffer = Buffer.from([1]);
  let calls = 0;
  buffer.toString = function (encoding) {
    assert.equal(this, buffer);
    assert.equal(encoding, 'base64');
    calls++;
    throw primary;
  };
  // Act
  const error = await h.handle.setInputFiles(payload(buffer)).then(
    () => assert.fail('server preparation must reject'), error => error);
  // Assert: server serializeError and client Connection parsing create a new
  // Error. This differs from earlier client getter errors that retain identity.
  assert.ok(error instanceof Error);
  assert.notEqual(error, primary);
  assert.equal(error.name, 'Error');
  assert.equal(error.message, 'elementHandle.setInputFiles: custom Buffer serializer failed');
  assert.equal(primary.message, 'custom Buffer serializer failed');
  assert.equal(calls, 1);
  assert.deepEqual(h.sent.map(message => message.method), ['ownerFrame', 'setInputFiles']);
  assert.deepEqual(h.prepared, []);
});

offline('server payload snapshots occur in per-item serializer order', async t => {
  // Arrange
  const h = oracle(t);
  const first = Buffer.from([1]);
  const second = Buffer.from([2]);
  const intrinsicToString = Buffer.prototype.toString;
  const calls = [];
  first.toString = function (encoding) {
    calls.push('first');
    return Reflect.apply(intrinsicToString, this, [encoding]);
  };
  second.toString = function (encoding) {
    calls.push('second');
    first[0] = 9;
    second[0] = 8;
    return Reflect.apply(intrinsicToString, this, [encoding]);
  };
  // Act
  await h.handle.setInputFiles([payload(first), payload(second)]);
  // Assert: changing the earlier Buffer cannot change its already-produced
  // base64 string, while the current payload observes its serializer mutation.
  assert.deepEqual(calls, ['first', 'second']);
  assert.deepEqual(h.prepared[0].filePayloads.map(file => [...Buffer.from(file.buffer, 'base64')]), [[1], [8]]);
  assert.deepEqual([...first], [9]);
  assert.deepEqual([...second], [8]);
});

offline('argument assembly follows proxy ownKeys, descriptors and enumerable symbol getters', async t => {
  // Arrange
  const h = oracle(t);
  const captured = argumentAssemblyBoundary(h);
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
    ownKeys() { trace.push('ownKeys'); return [symbol, 'timeout', 'hidden', 'label']; },
    getOwnPropertyDescriptor(object, key) { trace.push(`descriptor:${label(key)}`); return Reflect.getOwnPropertyDescriptor(object, key); },
    get(object, key, receiver) { trace.push(`get:${label(key)}`); return Reflect.get(object, key, receiver); },
  });
  // Act
  await h.handle.setInputFiles([], options);
  // Assert: the final timeout replaces its existing key without moving it;
  // symbols remain copied even though the later channel schema ignores them.
  assert.equal(captured.length, 1);
  assert.deepEqual(trace, ['ownKeys', 'descriptor:symbol', 'get:symbol', 'descriptor:timeout', 'get:timeout',
    'descriptor:hidden', 'descriptor:label', 'get:label', 'get:timeout', 'get:timeout']);
  assert.deepEqual(Reflect.ownKeys(captured[0]), ['payloads', 'timeout', 'label', symbol]);
  assert.equal(captured[0][symbol], 'symbol value');
  assert.equal(Object.hasOwn(captured[0], 'hidden'), false);
  assert.equal(captured[0].timeout, 17);
  assert.deepEqual(h.sent, []);
});

offline('an options payload getter replaces converted input without a second size pass', async t => {
  // Arrange
  const h = oracle(t);
  const trace = [];
  const original = { name: 42, get buffer() { trace.push('original:buffer'); return Buffer.from([1]); } };
  const replacementBytes = Buffer.from([9]);
  Object.defineProperty(replacementBytes, 'byteLength', { get() { throw new Error('replacement must not be sized'); } });
  const replacement = payload(replacementBytes);
  const options = { get payloads() { trace.push('option:payloads'); return [replacement]; } };
  // Act
  await h.handle.setInputFiles(original, options);
  // Assert: original metadata is invalid but is never channel-validated.
  assert.deepEqual(trace, ['original:buffer', 'original:buffer', 'option:payloads']);
  assert.equal(h.sent[1].params.payloads[0].buffer, replacementBytes);
  assert.deepEqual([...Buffer.from(h.prepared[0].filePayloads[0].buffer, 'base64')], [9]);
});

offline('numeric timeout check returns the next getter result without coercion', async t => {
  // Arrange
  const h = oracle(t);
  const captured = argumentAssemblyBoundary(h);
  const values = ['spread value', 1, 'raw second read'];
  let reads = 0;
  const options = { get timeout() { return values[reads++]; } };
  // Act
  await h.handle.setInputFiles([], options);
  // Assert: the real TimeoutSettings typeof check and return are distinct reads.
  assert.equal(reads, 3);
  assert.equal(captured.length, 1);
  assert.equal(captured[0].timeout, 'raw second read');
  assert.deepEqual(h.sent, []);
});

offline('a changed nonnumeric timeout reaches and fails the real channel validator', async t => {
  // Arrange
  const h = oracle(t);
  const values = [0, 1, 'not a float'];
  let reads = 0;
  const options = { get timeout() { return values[reads++]; } };
  // Act
  const error = await h.handle.setInputFiles([], options).then(() => assert.fail('must reject'), value => value);
  // Assert
  assert.equal(reads, 3);
  assert.equal(error.message, 'elementHandle.setInputFiles: timeout: expected float, got string');
  assert.deepEqual(h.sent.map(message => message.method), ['ownerFrame']);
  assert.deepEqual(h.prepared, []);
});

offline('nonnumeric timeout check uses the creation default and overwrites the spread value', async t => {
  // Arrange
  const h = oracle(t);
  const values = [7, 'use default'];
  let reads = 0;
  const options = { get timeout() { return values[reads++]; } };
  // Act
  await h.handle.setInputFiles([], options);
  // Assert
  assert.equal(reads, 2);
  assert.equal(h.sent[1].params.timeout, 111);
});

for (const [label, timeout] of [['fractional', 1.5], ['zero', 0], ['negative', -1],
  ['NaN', NaN], ['positive infinity', Infinity], ['negative infinity', -Infinity]]) {
  offline(`${label} numeric timeout passes the raw-buffer client and server schemas unchanged`, async t => {
    // Arrange: no server ProgressController/browser runs in this fixture. This
    // characterizes tFloat forwarding, not a supported native deadline domain.
    const h = oracle(t);
    // Act
    await h.handle.setInputFiles([], { timeout });
    // Assert
    assert.equal(Object.is(h.sent[1].params.timeout, timeout), true);
    assert.deepEqual(h.prepared[0].filePayloads, []);
  });
}

for (const [label, options] of [['undefined', undefined], ['null', null], ['false', false], ['zero', 0],
  ['number', 7], ['string', 'hi'], ['symbol', Symbol('options')], ['bigint', 1n]]) {
  offline(`${label} options use real object spread and Frame timeout normalization`, async t => {
    // Arrange
    const h = oracle(t);
    const captured = argumentAssemblyBoundary(h);
    // Act
    await h.handle.setInputFiles([], options);
    // Assert: Frame._timeout supplies options || {}. Truthy primitives still
    // undergo ordinary property access; a string also spreads its own indices.
    assert.equal(captured.length, 1);
    assert.equal(captured[0].timeout, 111);
    assert.deepEqual(Object.keys(captured[0]), label === 'string' ? ['0', '1', 'payloads', 'timeout'] : ['payloads', 'timeout']);
    if (label === 'string') assert.deepEqual([captured[0][0], captured[0][1]], ['h', 'i']);
    assert.deepEqual(h.sent, []);
  });
}

offline('Buffer upload defaults belong to the creation Frame even when its timeout is zero', async t => {
  // Arrange
  const h = oracle(t);
  h.creation.page()._timeoutSettings.setDefaultTimeout(0);
  h.owner.page()._timeoutSettings.setDefaultTimeout(987);
  // Act
  await h.handle.setInputFiles(payload());
  // Assert
  assert.equal(h.sent[1].params.timeout, 0);
  assert.equal(h.owner.page()._timeoutSettings.defaultTimeout(), 987);
});

offline('TimeoutSettings parent recursion repeats the options getter in every scope', async t => {
  // Arrange: real settings instances, not an emulated timeout resolver.
  const h = oracle(t);
  const trace = [];
  const platform = { ...h.connection._platform, isDebugMode() { trace.push('debug'); return false; } };
  const grandparent = new TimeoutSettings(platform);
  grandparent.setDefaultTimeout(731);
  const parent = new TimeoutSettings(platform, grandparent);
  h.creation.page()._timeoutSettings = new TimeoutSettings(platform, parent);
  const options = { get timeout() { trace.push('timeout'); return undefined; } };
  // Act
  await h.handle.setInputFiles([], options);
  // Assert: one spread read, then one typeof read and debug check per scope.
  assert.deepEqual(trace, ['timeout', 'timeout', 'debug', 'timeout', 'debug', 'timeout', 'debug']);
  assert.equal(h.sent[1].params.timeout, 731);
});

for (const phase of ['ownKeys', 'descriptor', 'symbol-getter', 'timeout-return']) {
  offline(`${phase} argument assembly error keeps identity before channel wrapping`, async t => {
    // Arrange
    const h = oracle(t);
    const primary = new Error(`${phase} failed before channel`);
    const symbol = Symbol('throwing option');
    let timeoutReads = 0;
    const target = {
      get [symbol]() { if (phase === 'symbol-getter') throw primary; return true; },
      get timeout() { if (++timeoutReads === 3 && phase === 'timeout-return') throw primary; return 1; },
    };
    const options = new Proxy(target, {
      ownKeys(object) { if (phase === 'ownKeys') throw primary; return Reflect.ownKeys(object); },
      getOwnPropertyDescriptor(object, key) {
        if (phase === 'descriptor') throw primary;
        return Reflect.getOwnPropertyDescriptor(object, key);
      },
    });
    const file = { buffer: Buffer.from([1]), get name() { throw new Error('channel must not run'); } };
    // Act
    const error = await h.handle.setInputFiles(file, options).then(() => assert.fail('must reject'), value => value);
    // Assert
    assert.equal(error, primary);
    assert.equal(primary.message, `${phase} failed before channel`);
    assert.deepEqual(h.sent.map(message => message.method), ['ownerFrame']);
    assert.deepEqual(h.prepared, []);
    if (phase === 'timeout-return') assert.equal(timeoutReads, 3);
  });
}

offline('options run in the immediate continuation after converted carrier assimilation', async t => {
  // Arrange: poison only the exact ordinary one-property payload carrier and
  // resolve synchronously to a null-prototype replacement. No pending job leaks.
  const h = oracle(t);
  const trace = [];
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  const converted = Object.create(null);
  Object.defineProperty(converted, 'payloads', { enumerable: true, get() { trace.push('converted:payloads'); return []; } });
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (!Object.hasOwn(this, 'payloads') || Reflect.ownKeys(this).length !== 1) return undefined;
    trace.push('carrier:then:get');
    return function (resolve) {
      trace.push('carrier:then:call');
      resolve(converted);
      queueMicrotask(() => trace.push('after:resolve'));
    };
  } });
  try {
    const options = { get marker() { trace.push('options'); return true; } };
    // Act
    await h.handle.setInputFiles([], options);
    // Assert: an extra await between conversion and options would move options
    // behind after:resolve and violate this observed ordering.
    assert.deepEqual(trace, ['carrier:then:get', 'carrier:then:call', 'converted:payloads', 'options', 'after:resolve']);
    assert.deepEqual(h.prepared[0].filePayloads, []);
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});

offline('assimilated converted getter errors precede options and keep unprefixed identity', async t => {
  // Arrange
  const h = oracle(t);
  const primary = new Error('converted payload getter failed');
  let optionsReads = 0;
  const previous = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  const converted = Object.create(null);
  Object.defineProperty(converted, 'payloads', { enumerable: true, get() { throw primary; } });
  Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
    if (!Object.hasOwn(this, 'payloads') || Reflect.ownKeys(this).length !== 1) return undefined;
    return resolve => resolve(converted);
  } });
  try {
    const options = { get timeout() { optionsReads++; return 1; } };
    // Act
    const error = await h.handle.setInputFiles([], options).then(() => assert.fail('must reject'), value => value);
    // Assert
    assert.equal(error, primary);
    assert.equal(primary.message, 'converted payload getter failed');
    assert.equal(optionsReads, 0);
    assert.deepEqual(h.sent.map(message => message.method), ['ownerFrame']);
  } finally {
    if (previous) Object.defineProperty(Object.prototype, 'then', previous);
    else delete Object.prototype.then;
  }
});
