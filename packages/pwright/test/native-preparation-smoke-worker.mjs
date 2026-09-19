import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers';
import { parentPort, workerData } from 'node:worker_threads';

const require = createRequire(import.meta.url);
const { resolveNativeArtifact, verifyNativeArtifact } = require('../src/native-artifact.cjs');
const addon = require(workerData.addon);
verifyNativeArtifact(addon, resolveNativeArtifact(workerData.profile));
const names = [
  'NativeContextSettings', 'NativeEnvironment', 'NativeFileInputPreparation', 'NativeFileInputTimeoutSettings',
  'NativeFrameNode', 'NativeFrameTree', 'NativeHandle', 'NativeLocator',
  'NativeSession', 'NativeSessionSignal',
];

// Arrange: loading the selected artifact must expose exactly the gated class
// inventory, with the safe no-cast diagnostic and its compiled feature marker.
assert.equal(addon.retainedFileInputEnabled(), true);
const payloadMutation = workerData.profile.startsWith('retained-serialized-payload-mutation');
const payloadImport = payloadMutation || workerData.profile.startsWith('retained-serialized-payload-import');
const serialization = payloadImport || workerData.profile.startsWith('retained-call-argument-serialization');
const instrumentation = serialization || workerData.profile.startsWith('retained-input-instrumentation');
const prevalidation = instrumentation || workerData.profile.startsWith('retained-target-prevalidation');
const marking = prevalidation || workerData.profile.startsWith('retained-target-marking');
assert.equal(addon.retainedTargetMarkingEnabled(), marking);
assert.equal(addon.retainedTargetPrevalidationEnabled(), prevalidation);
assert.equal(addon.retainedInputInstrumentationEnabled(), instrumentation);
assert.equal(addon.retainedCallArgumentSerializationEnabled(), serialization);
assert.equal(addon.retainedSerializedPayloadImportEnabled(), payloadImport);
assert.equal(addon.retainedSerializedPayloadMutationEnabled(), payloadMutation);
assert.deepEqual(Object.entries(addon).filter(([name, value]) => name.startsWith('Native') && typeof value === 'function')
  .map(([name]) => name).sort(), names);
assert.deepEqual([...addon.nativeClassNames()].sort(), names);
assert.equal(typeof addon.nativeClassMatches, 'function');
const maximum = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength')?.get;
assert.equal(typeof maximum, 'function', 'Node must expose its backing-size intrinsic');

// Act: real constructor and real N-API type-tag checks, with no connect call.
const environment = new addon.NativeEnvironment(setImmediate, maximum);
const timeoutSettings = environment.createFileInputTimeoutSettings(false);
const pageTimeouts = timeoutSettings.child();
pageTimeouts.setDefaultTimeout(12.5);
assert.equal(addon.nativeClassMatches('NativeFileInputTimeoutSettings', pageTimeouts), true);
assert.throws(() => new addon.NativeFileInputTimeoutSettings(), Error);
for (const value of [undefined, null, {}, environment, Object.create(addon.NativeFileInputTimeoutSettings.prototype)])
  assert.throws(() => Reflect.apply(addon.NativeFileInputTimeoutSettings.prototype.child, value, []), Error);

// The new producer lives in a feature-gated N-API implementation block. Prove
// its actual runtime registration and receiver guard, not only compilation.
const sessionMethods = [
  ['initializeFileInputTimeouts', [undefined, undefined, undefined]],
  ['fileInputContextTimeoutSettings', ['1']],
  ['fileInputPageTimeoutSettings', ['1']],
  ['frameEvaluateHandleWithPageTimeout', ['1', '1', false, undefined, '1']],
  ['handleCheckFileInputDispatch', [undefined]],
  ...(marking ? [
    ['handleBeginFileInputCall', [undefined]],
    ['handleStartFileInputDeadline', [undefined]],
    ['handleMarkFileInputTarget', [undefined]],
  ] : []),
  ...(prevalidation ? [['handlePrevalidateFileInputTarget', [undefined]]] : []),
  ...(instrumentation ? [
    ['handleBeginFileInputInstrumentation', [undefined]],
    ['handleFinishFileInputInstrumentation', [undefined]],
  ] : []),
  ...(serialization ? [['handleSerializeFileInputPayloads', [undefined, undefined]]] : []),
  ...(payloadImport ? [['handleImportSerializedFileInputPayloads', [undefined]]] : []),
  ...(payloadMutation ? [['handleMutateSerializedFileInputPayloads', [undefined]]] : []),
];
const forgedSession = Object.create(addon.NativeSession.prototype);
for (const [name, args] of sessionMethods) {
  assert.equal(typeof addon.NativeSession.prototype[name], 'function');
  assert.throws(() => Reflect.apply(addon.NativeSession.prototype[name], forgedSession, args), Error);
}

// Assert: raw tickets are host-produced, not constructible or forgeable from
// their prototype. No foreign payload is ever unwrapped by this smoke lane.
assert.equal(addon.nativeClassMatches('NativeEnvironment', environment), true);
assert.equal(addon.nativeClassMatches('NativeFileInputPreparation', environment), false);
assert.throws(() => new addon.NativeFileInputPreparation(), Error);
assert.equal(typeof addon.inspectFileInputChannel, 'function');
assert.equal(typeof addon.inspectFileInputServerChannel, 'function');
assert.equal(typeof addon.inspectFileInputMaterialization, 'function');
assert.equal(typeof addon.inspectFileInputProjection, 'function');
const preparationMethods = [['abort', []], ['isPending', []], ['validateChannel', [{}]],
  ['validateServerParams', [{}]], ['beginMaterialization', [{}]], ['finishMaterialization', [undefined]],
  ...(marking ? [['abortDeadline', []], ['projectAfterMarking', [{}]]] : [])];
for (const [name] of preparationMethods)
  assert.equal(typeof addon.NativeFileInputPreparation.prototype[name], 'function');
for (const value of [undefined, null, 1, 'ticket', {}, environment, Object.create(addon.NativeFileInputPreparation.prototype), new Proxy({}, {})]) {
  assert.equal(addon.nativeClassMatches('NativeFileInputPreparation', value), false);
  for (const [name, args] of preparationMethods)
    assert.throws(() => Reflect.apply(addon.NativeFileInputPreparation.prototype[name], value, args), Error);
}
parentPort.postMessage({ constructorAndClassChecks: true, classes: names.length, selectedSessionMethods: sessionMethods.length, selectedPreparationMethods: preparationMethods.length, actualTickets: 0, browserConnections: 0 });
parentPort.close();
