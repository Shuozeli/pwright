// NODE177 offline dispatch-boundary evidence, NOT browser/upload conformance.
// Pinned cases use real DispatcherConnection, Dispatcher, SdkObject and the
// instrumentation pipeline with an ElementHandle-schema probe, not a DOM
// ElementHandle. Its command handler always throws at command entry.
// Candidate cases run real Rust language kernels through explicit stateless
// session/preparation adapters. Adapter roots/revocation are fixture state:
// they prove driver ordering/cleanup calls, not native ticket authority,
// native root reclamation, lifecycle races or per-session type-tag checks.
// Selected candidate cases exercise the Node deadline coordinator and the
// pinned late call-argument adapter, but no public error rewriting, browser or
// mutation parity.
// Sources: pinned lib/server/dispatchers/dispatcher.js:252–350,
// instrumentation.js:46–63, inProcessFactory.js:46–47.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { prepareServerPayloads, prepareMarkedTarget, prepareProjectedTarget,
  preparePrevalidatedTarget, prepareInstrumentedTarget,
  prepareSerializedTarget, prepareImportedTarget,
  prepareMutatedTarget } = require('../src/file-input-dispatch.cjs');
const { serializeFileInputCallArgument } = require('../src/file-input-call-argument.cjs');
const { continueAfterChannel } = require('../src/file-input-preflight.cjs');
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
for (const name of ['inspectFileInputPreflightOwner', 'inspectFileInputPreflightContext',
  'inspectFileInputPreflightConversion', 'inspectFileInputArguments', 'inspectFileInputChannel',
  'inspectFileInputServerChannel', 'inspectFileInputMaterialization', 'inspectFileInputProjection'])
  assert.equal(typeof native[name], 'function', name);

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const load = path => requireCore(join(core, 'lib', path));
const { DispatcherConnection, Dispatcher } = load('server/dispatchers/dispatcher.js');
const { createRootSdkObject, SdkObject } = load('server/instrumentation.js');
const { findValidator } = load('protocol/validator.js');
const { ChannelOwner } = load('client/channelOwner.js');
const clientValidator = findValidator('ElementHandle', 'setInputFiles', 'Params');
const clientContext = Reflect.apply(ChannelOwner.prototype._validatorToWireContext, {
  _connection: { rawBuffers: () => true }, _platform: { isUnderTest: () => false },
}, []);
const environment = new native.NativeEnvironment(setImmediate,
  Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength').get);
const options = { timeout: 5000 };

function payload(buffer = Buffer.from([1])) {
  return { name: 'bytes.bin', mimeType: 'application/octet-stream', buffer };
}
async function rejection(promise) {
  return promise.then(() => assert.fail('operation must reject'), error => error);
}
function oracleFixture(t, hooks = {}) {
  const trace = [], replies = [];
  const connection = new DispatcherConnection(true);
  connection.onmessage = message => { if (message.id) replies.push(message); };
  const root = createRootSdkObject();
  const object = new SdkObject(root, 'schema-boundary', 'node177-offline-target');
  const boundaryError = new Error('STOP: command boundary only, no upload implementation');
  class SchemaProbe extends Dispatcher {
    async setInputFiles(params) {
      trace.push('command-boundary');
      hooks.command?.(params);
      throw boundaryError;
    }
  }
  const scope = new Dispatcher(connection, new SdkObject(root, 'scope', 'node177-offline-scope'), 'Root', {});
  const create = () => new SchemaProbe(scope, object, 'ElementHandle', { preview: 'JSHandle@node' });
  const target = create();
  root.instrumentation.addListener({
    async onBeforeCall() { trace.push('onBefore'); await hooks.before?.(); },
    async onAfterCall() { trace.push('onAfter'); },
  });
  t.after(() => { if (!scope._disposed) scope._dispose(); });
  return {
    trace, target, scope, create, connection,
    async dispatch(params) {
      await connection.dispatch({ id: 1, guid: object.guid, method: 'setInputFiles', params, metadata: {} });
      assert.equal(replies.length, 1);
      assert.equal(replies[0].result, undefined, 'fixture never reports upload success');
      return replies[0];
    },
  };
}

test('pinned dispatcher metadata: missing registration rejects before any server Params getter', options, async t => {
  // Arrange
  const h = oracleFixture(t);
  h.target._dispose();
  const params = { get payloads() { assert.fail('missing target must skip server getters'); } };
  // Act
  const reply = await h.dispatch(params);
  // Assert
  assert.equal(reply.error.error.name, 'TargetClosedError');
  assert.deepEqual(h.trace, []);
});

test('pinned dispatcher metadata: schema error wins over retirement inside an invalid getter', options, async t => {
  // Arrange
  const h = oracleFixture(t);
  const params = { get payloads() {
    h.trace.push('payloads'); h.target._dispose(); return [{ name: 42, buffer: Buffer.from([1]) }];
  }, get timeout() { assert.fail('later schema field must not be read'); } };
  // Act
  const reply = await h.dispatch(params);
  // Assert: ValidationError inherits Error.name; transport preserves "Error".
  assert.equal(reply.error.error.name, 'Error');
  assert.equal(reply.error.error.message, 'payloads[0].name: expected string, got number');
  assert.deepEqual(h.trace, ['payloads']);
});

test('pinned dispatcher metadata: valid schema completes before post-await retirement rejection', options, async t => {
  // Arrange
  const h = oracleFixture(t);
  const params = { get payloads() { h.trace.push('payloads'); h.target._dispose(); return []; },
    get timeout() { h.trace.push('timeout'); return 0; } };
  // Act
  const reply = await h.dispatch(params);
  // Assert
  assert.equal(reply.error.error.name, 'TargetClosedError');
  assert.deepEqual(h.trace, ['payloads', 'timeout', 'onBefore', 'onAfter']);
});

test('pinned dispatcher metadata: retirement in instrumentation skips command entry', options, async t => {
  // Arrange
  const h = oracleFixture(t, { before() { h.target._dispose(); } });
  // Act
  const reply = await h.dispatch({ payloads: [], timeout: 0 });
  // Assert
  assert.equal(reply.error.error.name, 'TargetClosedError');
  assert.deepEqual(h.trace, ['onBefore', 'onAfter']);
});

test('pinned dispatcher metadata: identity replacement with the same GUID is rejected', options, async t => {
  // Arrange
  let replacement;
  const h = oracleFixture(t, { before() { h.target._dispose(); replacement = h.create(); } });
  // Act
  const reply = await h.dispatch({ payloads: [], timeout: 0 });
  // Assert
  assert.notEqual(replacement, h.target);
  assert.equal(h.connection._dispatcherByGuid.get(h.target._guid), replacement);
  assert.equal(reply.error.error.name, 'TargetClosedError');
  assert.deepEqual(h.trace, ['onBefore', 'onAfter']);
});

test('pinned dispatcher metadata: schema microtask retirement wins at identity recheck', options, async t => {
  // Arrange
  const h = oracleFixture(t);
  const params = { get payloads() {
    h.trace.push('payloads');
    queueMicrotask(() => { h.trace.push('microtask:retire'); h.target._dispose(); });
    return [];
  }, get timeout() { h.trace.push('timeout'); return 0; } };
  // Act
  const reply = await h.dispatch(params);
  // Assert
  assert.equal(reply.error.error.name, 'TargetClosedError');
  assert.deepEqual(h.trace, ['payloads', 'timeout', 'onBefore', 'microtask:retire', 'onAfter']);
});

test('pinned dispatcher metadata: parent scope retirement removes descendant targets', options, async t => {
  // Arrange: generic SDK scope, not a simulated Page-close event or browser.
  const h = oracleFixture(t);
  h.scope._dispose();
  // Act
  const reply = await h.dispatch({ get payloads() { assert.fail('retired descendant must skip getters'); } });
  // Assert
  assert.equal(reply.error.error.name, 'TargetClosedError');
  assert.equal(h.connection._dispatcherByGuid.has(h.target._guid), false);
  assert.deepEqual(h.trace, []);
});

test('pinned dispatcher metadata: present target reaches only the deliberate command boundary', options, async t => {
  // Arrange
  const h = oracleFixture(t);
  // Act
  const reply = await h.dispatch({ payloads: [], timeout: 0 });
  // Assert
  assert.equal(reply.error.error.message, 'STOP: command boundary only, no upload implementation');
  assert.deepEqual(h.trace, ['onBefore', 'command-boundary', 'onAfter']);
});

function serverGetterFiles(onName) {
  // A permitted custom CLIENT map returns an unchecked array. Thus the row's
  // name accessor genuinely survives to the SECOND validator; no fixture
  // mutation is secretly inserted between native client/server kernels.
  const bytes = Buffer.from([1]);
  const files = [];
  files.slice = () => files;
  files.map = () => [{ ...payload(bytes), get name() { onName(bytes); return 'server.bin'; } }];
  return { files, bytes };
}

test('pinned dispatcher metadata: second-schema Buffer mutation runs before command entry', options, async t => {
  // Arrange
  let observed;
  const h = oracleFixture(t, { command(params) { observed = params.payloads[0].buffer[0]; } });
  const { files } = serverGetterFiles(bytes => {
    h.trace.push('server:name');
    queueMicrotask(() => { bytes[0] = 9; h.trace.push('microtask:bytes'); });
  });
  const params = clientValidator({ payloads: files, timeout: 0 }, '', clientContext);
  // Act
  const reply = await h.dispatch(params);
  // Assert: command entry, not serialization/browser mutation, is observed.
  assert.equal(observed, 9);
  assert.equal(reply.error.error.message, 'STOP: command boundary only, no upload implementation');
  assert.deepEqual(h.trace, ['server:name', 'onBefore', 'microtask:bytes', 'command-boundary', 'onAfter']);
});

function candidateFixture(files = [], inputOptions = {}, hooks = {}) {
  const trace = [];
  const roots = new Map([['files', files], ['options', inputOptions]]);
  const state = { trace, roots, aborts: 0, checks: 0, revoked: false, retired: false, result: undefined };
  const revoked = new Error('fixture preparation roots revoked');
  const retired = new Error('fixture target registration retired');
  const owner = { page: () => ({ context: () => ({}) }) };
  const receiver = { ownerFrame() { trace.push('owner'); return owner; } };
  const settings = environment.createFileInputTimeoutSettings(false).child();
  settings.setDefaultTimeout(0);
  const guard = phase => {
    trace.push(phase);
    if (state.revoked) throw revoked;
  };
  const preparation = {
    beginOwnerLookup(ports) { guard('begin-owner'); return native.inspectFileInputPreflightOwner(receiver, ports); },
    prepareContext(value, ports) {
      guard('context'); roots.set('context', native.inspectFileInputPreflightContext(value, ports));
    },
    convertInput(ports) {
      guard('convert'); return native.inspectFileInputPreflightConversion(roots.get('context'), roots.get('files'), ports);
    },
    finishConversion(value) { guard('converted'); roots.set('converted', value); },
    assembleArguments(ports) {
      guard('arguments');
      roots.set('params', native.inspectFileInputArguments(roots.get('converted'), roots.get('options'), settings, ports));
    },
    validateChannel(ports) {
      guard('client-schema');
      roots.set('client', native.inspectFileInputChannel(roots.get('params'), ports));
      hooks.afterClient?.(state);
    },
    validateServerParams(ports) {
      guard('server-schema');
      roots.set('server', native.inspectFileInputServerChannel(roots.get('client'), ports));
      hooks.afterServer?.(state);
    },
    beginMaterialization(ports) {
      guard('materialize');
      return native.inspectFileInputMaterialization(roots.get('server'), ports);
    },
    finishMaterialization(value) {
      guard('materialized'); state.result = value; roots.set('materialized', value);
    },
    projectAfterMarking(ports) {
      guard('projected');
      state.projection = native.inspectFileInputProjection(roots.get('materialized'), ports);
      return {
        multiple: state.projection.multiple,
        directoryUpload: state.projection.directoryUpload,
      };
    },
    abort() { state.aborts++; trace.push('abort'); roots.clear(); },
  };
  const session = {
    handleCheckFileInputDispatch(value) {
      assert.equal(value, preparation, 'driver must check its exact supplied ticket');
      state.checks++; trace.push('check:' + state.checks);
      hooks.check?.(state, value);
      if (state.revoked) throw revoked;
      if (state.retired) throw retired;
    },
  };
  return { state, preparation, session, receiver, revoked, retired,
    run: () => prepareServerPayloads(session, preparation) };
}

test('candidate dispatch adapter: kernels advance through two checks to the Materialized stop point', options, async () => {
  // Arrange
  const h = candidateFixture([payload()]);
  // Act
  const result = await h.run();
  // Assert
  assert.equal(result, undefined, 'caller retains its ticket; the internal stop point is not upload success');
  assert.deepEqual(h.state.trace, ['begin-owner', 'owner', 'context', 'convert', 'converted', 'arguments',
    'client-schema', 'check:1', 'server-schema', 'check:2', 'materialize', 'materialized']);
  assert.equal(h.state.result.filePayloads[0].buffer, 'AQ==');
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
  assert.equal(h.state.roots.size, 0, 'fixture cleanup only');
});

test('candidate selected progress: call identity precedes deadline and one coordinator reaches Marked', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = value => {
    assert.equal(value, h.preparation);
    h.state.trace.push('call:call@1');
    return 'call@1';
  };
  h.session.handleStartFileInputDeadline = value => {
    assert.equal(value, h.preparation);
    h.state.trace.push('deadline:0');
    return 0;
  };
  h.session.handleMarkFileInputTarget = async value => {
    assert.equal(value, h.preparation);
    h.state.trace.push('marked');
  };

  // Act
  await prepareMarkedTarget(h.session, h.preparation);

  // Assert: the selected deadline gate replaces, rather than supplements, the
  // old second dispatch check and the caller still owns the parked ticket.
  assert.deepEqual(h.state.trace, ['begin-owner', 'owner', 'context', 'convert', 'converted', 'arguments',
    'client-schema', 'check:1', 'server-schema', 'call:call@1', 'deadline:0',
    'materialize', 'materialized', 'marked']);
  assert.equal(h.state.checks, 1);
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected projection: marked completion precedes every carrier read on the same progress', options, async () => {
  // Arrange
  const trace = [];
  const payloads = { get length() { trace.push('payloads.length'); return 2; } };
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.preparation.finishMaterialization = value => {
    h.state.trace.push('materialized');
    h.state.result = value;
    h.state.roots.set('materialized', {
      get filePayloads() { trace.push('filePayloads'); return payloads; },
      get localPaths() { trace.push('localPaths'); return undefined; },
      get localDirectory() { trace.push('localDirectory'); return ''; },
    });
  };
  h.session.handleBeginFileInputCall = () => 'call@1';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => { trace.push('marked'); };

  // Act
  await prepareProjectedTarget(h.session, h.preparation);

  // Assert
  assert.deepEqual(trace, ['marked', 'filePayloads', 'localPaths', 'localDirectory', 'payloads.length']);
  assert.equal(h.state.projection.filePayloads, payloads);
  assert.equal(h.state.projection.multiple, true);
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected prevalidation follows projection on the same progress', options, async () => {
  // Arrange
  const h = candidateFixture([payload(), payload(Buffer.from([2]))], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => { h.state.trace.push('call'); return 'call@1'; };
  h.session.handleStartFileInputDeadline = () => { h.state.trace.push('deadline'); return 0; };
  h.session.handleMarkFileInputTarget = async () => { h.state.trace.push('marked'); };
  h.session.handlePrevalidateFileInputTarget = async value => {
    assert.equal(value, h.preparation);
    assert.equal(h.state.projection.multiple, true);
    assert.equal(h.state.projection.directoryUpload, false);
    h.state.trace.push('prevalidated');
  };

  // Act
  await preparePrevalidatedTarget(h.session, h.preparation);

  // Assert
  assert.deepEqual(h.state.trace.slice(-6), [
    'deadline', 'materialize', 'materialized', 'marked', 'projected', 'prevalidated',
  ]);
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected input instrumentation awaits the original receiver after prevalidation', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => { h.state.trace.push('call'); return 'call@7'; };
  h.session.handleStartFileInputDeadline = () => { h.state.trace.push('deadline'); return 0; };
  h.session.handleMarkFileInputTarget = async () => { h.state.trace.push('marked'); };
  h.session.handlePrevalidateFileInputTarget = async () => { h.state.trace.push('prevalidated'); };
  h.session.handleBeginFileInputInstrumentation = value => {
    assert.equal(value, h.preparation);
    h.state.trace.push('instrumentation:begin');
    return h.receiver;
  };
  h.session.handleFinishFileInputInstrumentation = value => {
    assert.equal(value, h.preparation);
    h.state.trace.push('instrumentation:finish');
  };
  const instrumentation = Object.freeze({
    async onBeforeInputAction(progress, receiver, point, box) {
      assert.equal(progress.callId, 'call@7');
      assert.equal(typeof progress.race, 'function');
      assert.equal(receiver, h.receiver);
      assert.equal(point, undefined);
      assert.equal(box, undefined);
      h.state.trace.push('instrumentation:hook');
      await Promise.resolve();
      h.state.trace.push('instrumentation:awaited');
    },
  });

  // Act
  await prepareInstrumentedTarget(h.session, h.preparation, instrumentation);

  // Assert
  assert.deepEqual(h.state.trace.slice(-8), [
    'materialized', 'marked', 'projected', 'prevalidated', 'instrumentation:begin',
    'instrumentation:hook', 'instrumentation:awaited', 'instrumentation:finish',
  ]);
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected input instrumentation rejection revokes without committing finish', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 0 });
  const sentinel = new Error('instrumentation refused input action');
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@1';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => {};
  h.session.handlePrevalidateFileInputTarget = async () => {};
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => assert.fail('rejected hook must not commit');

  // Act
  const error = await rejection(prepareInstrumentedTarget(h.session, h.preparation, {
    async onBeforeInputAction() { throw sentinel; },
  }));

  // Assert
  assert.equal(error, sentinel);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate selected late serializer observes instrumentation microtasks after finish', options, async () => {
  // Arrange
  const leaf = { value: 'before' };
  const bytes = Buffer.from([1]);
  bytes.toString = () => leaf;
  const h = candidateFixture([payload(bytes)], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@9';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => { h.state.trace.push('marked'); };
  h.session.handlePrevalidateFileInputTarget = async () => { h.state.trace.push('prevalidated'); };
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => { h.state.trace.push('instrumentation:finish'); };
  h.session.handleSerializeFileInputPayloads = (preparation, serializer) => {
    assert.equal(preparation, h.preparation);
    assert.equal(serializer, serializeFileInputCallArgument);
    h.state.trace.push('serialized');
    h.state.serialized = serializer(h.state.projection.filePayloads);
  };
  const instrumentation = {
    async onBeforeInputAction() {
      h.state.trace.push('instrumentation:hook');
      queueMicrotask(() => { leaf.value = 'after'; h.state.trace.push('instrumentation:microtask'); });
    },
  };

  // Act
  await prepareSerializedTarget(h.session, h.preparation, instrumentation);

  // Assert
  assert.deepEqual(h.state.trace.slice(-7), [
    'marked', 'projected', 'prevalidated', 'instrumentation:hook',
    'instrumentation:microtask', 'instrumentation:finish', 'serialized',
  ]);
  const row = h.state.serialized.a[0].o;
  const buffer = row.find(entry => entry.k === 'buffer').v;
  assert.equal(buffer.o.find(entry => entry.k === 'value').v, 'after');
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected payload import occurs only after late serialization', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@10';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => { h.state.trace.push('marked'); };
  h.session.handlePrevalidateFileInputTarget = async () => { h.state.trace.push('prevalidated'); };
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => { h.state.trace.push('instrumented'); };
  h.session.handleSerializeFileInputPayloads = (_preparation, serializer) => {
    h.state.serialized = serializer(h.state.projection.filePayloads);
    h.state.trace.push('serialized');
  };
  h.session.handleImportSerializedFileInputPayloads = preparation => {
    assert.equal(preparation, h.preparation);
    assert.ok(h.state.serialized);
    h.state.trace.push('imported');
  };

  // Act
  await prepareImportedTarget(h.session, h.preparation);

  // Assert
  assert.deepEqual(h.state.trace.slice(-6), [
    'marked', 'projected', 'prevalidated', 'instrumented', 'serialized', 'imported',
  ]);
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected payload mutation occurs after import and shares the deadline race', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@12';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => { h.state.trace.push('marked'); };
  h.session.handlePrevalidateFileInputTarget = async () => { h.state.trace.push('prevalidated'); };
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => { h.state.trace.push('instrumented'); };
  h.session.handleSerializeFileInputPayloads = (_preparation, serializer) => {
    h.state.serialized = serializer(h.state.projection.filePayloads);
    h.state.trace.push('serialized');
  };
  h.session.handleImportSerializedFileInputPayloads = () => { h.state.trace.push('imported'); };
  h.session.handleMutateSerializedFileInputPayloads = async preparation => {
    assert.equal(preparation, h.preparation);
    assert.ok(h.state.serialized);
    h.state.trace.push('mutated');
  };

  // Act
  await prepareMutatedTarget(h.session, h.preparation);

  // Assert
  assert.deepEqual(h.state.trace.slice(-7), [
    'marked', 'projected', 'prevalidated', 'instrumented', 'serialized', 'imported', 'mutated',
  ]);
  assert.equal(h.state.aborts, 0);
  h.preparation.abort();
});

test('candidate selected payload mutation rejection revokes the exact preparation', options, async () => {
  // Arrange
  const sentinel = new Error('native payload mutation failed');
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@13';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => {};
  h.session.handlePrevalidateFileInputTarget = async () => {};
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => {};
  h.session.handleSerializeFileInputPayloads = () => {};
  h.session.handleImportSerializedFileInputPayloads = () => {};
  h.session.handleMutateSerializedFileInputPayloads = async () => { throw sentinel; };

  // Act
  const error = await rejection(prepareMutatedTarget(h.session, h.preparation));

  // Assert
  assert.equal(error, sentinel);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate selected late serializer rejection revokes the exact preparation', options, async () => {
  // Arrange
  const sentinel = new Error('late nested array getter');
  const hostile = [];
  Object.defineProperty(hostile, 0, { get() { throw sentinel; } });
  hostile.length = 1;
  const bytes = Buffer.from([1]);
  bytes.toString = () => hostile;
  const h = candidateFixture([payload(bytes)], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@1';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => {};
  h.session.handlePrevalidateFileInputTarget = async () => {};
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => {};
  h.session.handleSerializeFileInputPayloads = (_preparation, serializer) =>
    serializer(h.state.projection.filePayloads);

  // Act
  const error = await rejection(prepareSerializedTarget(h.session, h.preparation));

  // Assert
  assert.equal(error, sentinel);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate selected payload import rejection revokes the exact preparation', options, async () => {
  // Arrange
  const sentinel = new Error('serialized payload import capacity');
  const h = candidateFixture([payload()], { timeout: 0 });
  h.preparation.abortDeadline = () => assert.fail('zero timeout must not schedule a deadline');
  h.session.handleBeginFileInputCall = () => 'call@11';
  h.session.handleStartFileInputDeadline = () => 0;
  h.session.handleMarkFileInputTarget = async () => {};
  h.session.handlePrevalidateFileInputTarget = async () => {};
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => {};
  h.session.handleSerializeFileInputPayloads = () => {};
  h.session.handleImportSerializedFileInputPayloads = () => { throw sentinel; };

  // Act
  const error = await rejection(prepareImportedTarget(h.session, h.preparation));

  // Assert
  assert.equal(error, sentinel);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate selected input instrumentation shares the one Node deadline', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 1 });
  let deadlineAborts = 0;
  h.session.handleBeginFileInputCall = () => 'call@1';
  h.session.handleStartFileInputDeadline = () => 1;
  h.session.handleMarkFileInputTarget = async () => {};
  h.session.handlePrevalidateFileInputTarget = async () => {};
  h.session.handleBeginFileInputInstrumentation = () => h.receiver;
  h.session.handleFinishFileInputInstrumentation = () => assert.fail('deadline must skip finish');
  h.preparation.abortDeadline = () => {
    deadlineAborts++;
    h.state.roots.clear();
    return true;
  };

  // Act
  const error = await rejection(prepareInstrumentedTarget(h.session, h.preparation, {
    async onBeforeInputAction() { return new Promise(() => {}); },
  }));

  // Assert
  assert.equal(error.code, 'PWRIGHT_TIMEOUT');
  assert.equal(deadlineAborts, 1);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate selected progress: Node timer rejects and revokes a pending materialization', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], { timeout: 1 });
  let deadlineAborts = 0;
  h.session.handleBeginFileInputCall = () => { h.state.trace.push('call'); return 'call@1'; };
  h.session.handleStartFileInputDeadline = () => { h.state.trace.push('deadline'); return 1; };
  h.session.handleMarkFileInputTarget = () => assert.fail('timeout must precede marking');
  h.preparation.beginMaterialization = () => {
    h.state.trace.push('materialize:pending');
    return new Promise(() => {});
  };
  h.preparation.abortDeadline = () => {
    deadlineAborts++;
    h.state.trace.push('deadline-abort');
    h.state.roots.clear();
    return true;
  };

  // Act
  const error = await rejection(prepareMarkedTarget(h.session, h.preparation));

  // Assert
  assert.equal(error.code, 'PWRIGHT_TIMEOUT');
  assert.equal(error.message, 'PWRIGHT_TIMEOUT: Timeout 1ms exceeded');
  assert.equal(deadlineAborts, 1);
  assert.equal(h.state.aborts, 1, 'driver catch remains idempotent cleanup');
  assert.deepEqual(h.state.trace.slice(-4), ['deadline', 'materialize:pending', 'deadline-abort', 'abort']);
  assert.equal(h.state.roots.size, 0);
});

for (const behavior of ['throwing getter', 'rejecting thenable']) {
  test('candidate dispatch adapter: ticket ' + behavior + ' is never read or assimilated', options, async () => {
    // Arrange
    const sentinel = new Error('ticket then must remain untouched');
    const h = candidateFixture([payload()]);
    let reads = 0, calls = 0;
    Object.defineProperty(h.preparation, 'then', { get() {
      reads++;
      if (behavior === 'throwing getter') throw sentinel;
      return (_resolve, reject) => { calls++; reject(sentinel); };
    } });
    // Act
    const result = await h.run();
    // Assert: caller still owns the fixture's Materialized roots. A void return
    // avoids transferring this user-controlled ticket through Promise resolve.
    assert.equal(result, undefined);
    assert.equal(reads, 0);
    assert.equal(calls, 0);
    assert.equal(h.state.trace.at(-1), 'materialized');
    assert.equal(h.state.aborts, 0);
    assert.equal(h.state.roots.has('materialized'), true);
    h.preparation.abort();
    assert.equal(h.state.aborts, 1);
    assert.equal(h.state.roots.size, 0);
  });
}

test('candidate dispatch adapter: client schema rejection never reaches the scheduled target gate', options, async () => {
  // Arrange
  const h = candidateFixture([{ ...payload(), name: 42 }]);
  // Act
  const error = await rejection(h.run());
  // Assert
  assert.match(error.message, /^payloads\[0\]\.name: expected string, got number$/);
  assert.equal(h.state.checks, 0);
  assert.equal(h.state.trace.includes('server-schema'), false);
  assert.equal(h.state.trace.includes('materialize'), false);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: first gate rejects before second-schema getters', options, async () => {
  // Arrange
  let reads = 0;
  const { files } = serverGetterFiles(() => { reads++; });
  const h = candidateFixture(files, {}, { afterClient(state) { state.retired = true; } });
  // Act
  const error = await rejection(h.run());
  // Assert
  assert.equal(error, h.retired);
  assert.equal(reads, 0);
  assert.equal(h.state.checks, 1);
  assert.equal(h.state.trace.includes('server-schema'), false);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: second-schema microtask mutation precedes Buffer serialization', options, async () => {
  // Arrange
  const events = [];
  const { files, bytes } = serverGetterFiles(value => {
    events.push('server:name');
    queueMicrotask(() => { value[0] = 9; events.push('microtask:bytes'); });
  });
  bytes.toString = function (encoding) {
    events.push('serialize'); return Buffer.prototype.toString.call(this, encoding);
  };
  const h = candidateFixture(files);
  // Act
  await h.run();
  // Assert
  assert.deepEqual(events, ['server:name', 'microtask:bytes', 'serialize']);
  assert.equal(h.state.result.filePayloads[0].buffer, 'CQ==');
  assert.equal(h.state.checks, 2);
  h.preparation.abort();
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: second-schema microtask retirement wins before materialization', options, async () => {
  // Arrange
  let h, serializers = 0;
  const { files, bytes } = serverGetterFiles(() => queueMicrotask(() => { h.state.retired = true; }));
  bytes.toString = () => { serializers++; return 'AQ=='; };
  h = candidateFixture(files);
  // Act
  const error = await rejection(h.run());
  // Assert
  assert.equal(error, h.retired);
  assert.equal(h.state.checks, 2);
  assert.equal(serializers, 0);
  assert.equal(h.state.trace.includes('materialize'), false);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: revocation while dispatch is queued aborts fixture roots', options, async () => {
  // Arrange
  const h = candidateFixture([payload()], {}, { afterClient(state) {
    queueMicrotask(() => { state.revoked = true; });
  } });
  // Act
  const error = await rejection(h.run());
  // Assert: these counters describe the adapter, not native ledger/root proofs.
  assert.equal(error, h.revoked);
  assert.equal(h.state.checks, 1);
  assert.equal(h.state.trace.includes('server-schema'), false);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: foreign-session gate error precedes server getters', options, async () => {
  // Arrange: the fake rejection proves forwarding/phase order only. Module
  // type tags and real session authority are separate raw-addon/host tests.
  let reads = 0;
  const { files } = serverGetterFiles(() => { reads++; });
  const h = candidateFixture(files);
  const sentinel = new Error('fixture foreign session');
  const foreignSession = { handleCheckFileInputDispatch(value) {
    assert.equal(value, h.preparation); throw sentinel;
  } };
  // Act
  const error = await rejection(prepareServerPayloads(foreignSession, h.preparation));
  // Assert
  assert.equal(error, sentinel);
  assert.equal(reads, 0);
  assert.equal(h.state.trace.includes('server-schema'), false);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: continuation is invoked before client-schema queued microtasks', options, async () => {
  // Arrange
  const trace = [];
  const h = candidateFixture([], {}, { afterClient() {
    trace.push('validated');
    queueMicrotask(() => trace.push('microtask'));
  } });
  // Act: production continuation seam itself, no reconstructed async driver.
  const result = await continueAfterChannel(h.preparation, value => {
    assert.equal(value, h.preparation); trace.push('continuation'); return value;
  });
  // Assert
  assert.equal(result, h.preparation);
  assert.deepEqual(trace, ['validated', 'continuation', 'microtask']);
  h.preparation.abort();
  assert.equal(h.state.roots.size, 0);
});

test('candidate dispatch adapter: outer materialization rejection aborts without committing result', options, async () => {
  // Arrange
  const sentinel = new Error('outer carrier then rejection');
  const saved = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
  let installed = false;
  const h = candidateFixture([], {}, { afterServer() {
    Object.defineProperty(Object.prototype, 'then', { configurable: true, get() {
      if (!Object.hasOwn(this, 'filePayloads')) return undefined;
      throw sentinel;
    } });
    installed = true;
  } });
  let error;
  try {
    // Act
    error = await rejection(h.run());
  } finally {
    if (installed) {
      if (saved) Object.defineProperty(Object.prototype, 'then', saved);
      else delete Object.prototype.then;
    }
  }
  // Assert
  assert.equal(error, sentinel);
  assert.equal(h.state.trace.includes('materialize'), true);
  assert.equal(h.state.trace.includes('materialized'), false);
  assert.equal(h.state.aborts, 1);
  assert.equal(h.state.roots.size, 0);
});
