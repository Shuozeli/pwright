import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createReadStream, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { isAbsolute } from 'node:path';
import { test } from 'node:test';
import { setImmediate } from 'node:timers';
import { setImmediate as nextTurn } from 'node:timers/promises';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(import.meta.url);
const { nativeArtifact } = require('../src/native-artifact.cjs');
assert.equal(nativeArtifact.profile, 'handles');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const names = [
  'NativeContextSettings', 'NativeEnvironment', 'NativeFrameNode', 'NativeFrameTree',
  'NativeHandle', 'NativeLocator', 'NativeSession', 'NativeSessionSignal',
];

function fixturePath(variable) {
  const path = process.env[variable];
  assert.ok(path && isAbsolute(path), `${variable} must identify an independently copied addon`);
  return realpathSync(path);
}

async function digest(path) {
  const hash = createHash('sha256');
  // Stream binary evidence; do not retain the complete addon in JS memory.
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex');
}

const primaryPath = realpathSync(nativeArtifact.path);
const legacyPath = fixturePath('PWRIGHT_PEER_ADDON');
const sameBuildPath = fixturePath('PWRIGHT_SAME_BUILD_ADDON');
assert.equal(new Set([primaryPath, legacyPath, sameBuildPath]).size, 3);
const [primaryDigest, legacyDigest, sameBuildDigest] = await Promise.all(
  [primaryPath, legacyPath, sameBuildPath].map(digest),
);
assert.equal(sameBuildDigest, primaryDigest, 'same-build peer must be an exact copy of the current binary');
assert.notEqual(legacyDigest, primaryDigest, 'legacy fixture must remain the earlier binary');
const primary = require(primaryPath);
const legacy = require(legacyPath);
const sameBuild = require(sameBuildPath);

function assertInventory(addon, label, guarded) {
  const exported = Object.entries(addon)
    .filter(([name, value]) => name.startsWith('Native') && typeof value === 'function')
    .map(([name]) => name).sort();
  assert.deepEqual(exported, names, `${label}: exported raw-class inventory`);
  assert.equal(addon.handlesEnabled(), true, `${label}: handles capability`);
  if (guarded) {
    assert.equal(typeof addon.nativeClassMatches, 'function', `${label}: safe brand probe must exist`);
    assert.equal(typeof addon.nativeClassNames, 'function', `${label}: brand inventory must exist`);
    assert.deepEqual([...addon.nativeClassNames()].sort(), names, `${label}: guarded class inventory`);
  }
}

assertInventory(primary, 'current', true);
assertInventory(sameBuild, 'same-build peer', true);
assertInventory(legacy, 'legacy peer', false);
for (const name of names) {
  assert.notEqual(primary[name], sameBuild[name], `${name}: copied binary must load independently`);
  assert.notEqual(primary[name], legacy[name], `${name}: legacy binary must load independently`);
}

async function createFixture(addon, label, fixtures) {
  const maximum = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength').get;
  const fixture = { addon, label, objects: {} };
  fixtures.push(fixture); // Cleanup also covers partially constructed fixtures.
  fixture.environment = new addon.NativeEnvironment(setImmediate, maximum);
  fixture.session = await fixture.environment.connectOverCDP(endpoint, 5000);
  fixture.context = await fixture.session.newContext();
  fixture.page = await fixture.session.newPage(fixture.context.id);
  fixture.handle = await fixture.session.frameEvaluateHandle(
    fixture.page.mainFrameId, '({ answer: 42 })', false, undefined,
  );
  fixture.objects = {
    NativeContextSettings: fixture.context.settings,
    NativeEnvironment: fixture.environment,
    NativeFrameNode: fixture.page.mainFrameBinding ?? fixture.page.frameTree.metadata(fixture.page.mainFrameId),
    NativeFrameTree: fixture.page.frameTree,
    NativeHandle: fixture.handle,
    NativeLocator: fixture.session.locator(fixture.page.mainFrameId, 'html'),
    NativeSession: fixture.session,
    NativeSessionSignal: fixture.session.sessionSignal(),
  };
  for (const name of names) assert.ok(fixture.objects[name], `${label}: fixture for ${name}`);
  return fixture;
}

async function cleanupFixtures(fixtures, primaryFailure) {
  const failures = [];
  for (const fixture of [...fixtures].reverse()) {
    for (const [operation, close] of [
      ['Handle disposal', () => fixture.handle && fixture.session.handleDispose(fixture.handle)],
      ['Context close', () => fixture.context && fixture.session.closeContext(fixture.context.id)],
      ['Session close', () => fixture.session?.close()],
    ]) {
      try { await diagnosticDeadline(Promise.resolve().then(close), `${fixture.label} ${operation}`); }
      catch (error) { failures.push(error); }
    }
  }
  for (const fixture of fixtures) {
    if (!fixture.session) continue;
    try {
      // Keep all raw wrappers, including disposed Handles, alive during checks.
      assert.equal(fixture.session.metrics().activeLeases, 0, `${fixture.label}: host leases`);
      assert.equal(fixture.session.metrics().cleanupInFlight, 0, `${fixture.label}: cleanup work`);
      for (let turn = 0; turn < 100 && fixture.session.ffiMetrics().reservedBytes !== 0; turn++) await nextTurn();
      assert.equal(fixture.session.ffiMetrics().reservedBytes, 0, `${fixture.label}: FFI bytes`);
      assert.equal(fixture.session.ffiMetrics().pending, 0, `${fixture.label}: pending FFI`);
    } catch (error) { failures.push(error); }
  }
  if (failures.length) throw new AggregateError(
    primaryFailure ? [primaryFailure, ...failures] : failures, 'native-class fixture cleanup failed',
  );
}

function assertBrands(local, foreign) {
  assert.throws(() => local.addon.nativeClassMatches('NativeUnknown', {}), /Unknown native class/);
  assert.throws(() => local.addon.nativeClassMatches('x'.repeat(65), {}), /Unknown native class/);
  assert.throws(() => local.addon.nativeClassMatches('é'.repeat(33), {}), /Unknown native class/);
  for (const [index, name] of names.entries()) {
    const matches = value => local.addon.nativeClassMatches(name, value);
    assert.equal(matches(local.objects[name]), true, `${local.label}: own ${name} brand`);
    assert.equal(matches(foreign.objects[name]), false, `${local.label}: reject ${foreign.label} ${name} brand`);
    assert.equal(matches(local.objects[names[(index + 1) % names.length]]), false, `${name}: wrong native class`);
    assert.equal(matches(Object.create(local.addon[name].prototype)), false, `${name}: plain prototype spoof`);
    assert.equal(matches(new Proxy(local.objects[name], {})), false, `${name}: Proxy cannot copy native brand`);
    for (const primitive of [undefined, null, true, 0, 1n, '', Symbol('unbranded')]) {
      assert.equal(matches(primitive), false, `${name}: primitive ${typeof primitive} cannot carry a class brand`);
    }
    assert.equal(matches(() => {}), false, `${name}: a function cannot carry a class brand`);

    const object = foreign.objects[name];
    const original = Object.getPrototypeOf(object);
    Object.setPrototypeOf(object, local.addon[name].prototype);
    try { assert.equal(matches(object), false, `${name}: changing foreign prototype cannot copy native brand`); }
    finally { Object.setPrototypeOf(object, original); }
  }
}

function rejectForeign(local, foreign, name, invoke, operation) {
  // Safe red boundary: the diagnostic accepts Unknown and checks only the tag.
  // A build sharing tags with the foreign module fails HERE, before any raw
  // class argument conversion or receiver method can cast its native payload.
  assert.equal(local.addon.nativeClassMatches(name, foreign.objects[name]), false, `${operation}: brand must reject before invocation`);
  assert.throws(invoke, `${operation}: foreign class must reject synchronously`);
}

function rawArguments(local, foreign) {
  const handle = foreign.objects.NativeHandle;
  let getterReads = 0;
  const argument = { get later() { getterReads++; return 1; } };
  const handleCalls = [
    ['handleOwnerFrame', () => local.session.handleOwnerFrame(handle)],
    ['handleJsonValue', () => local.session.handleJsonValue(handle)],
    ['handleEvaluate', () => local.session.handleEvaluate(handle, '(value, arg) => value.answer', true, argument)],
    ['handleEvaluateHandle', () => local.session.handleEvaluateHandle(handle, '(value, arg) => value', true, argument)],
    ['handleGetProperty', () => local.session.handleGetProperty(handle, 'answer')],
    ['handleDispose', () => local.session.handleDispose(handle)],
  ];
  for (const [name, invoke] of handleCalls) {
    assert.equal(typeof local.session[name], 'function', `${name}: required raw method exists`);
    rejectForeign(local, foreign, 'NativeHandle', invoke, name);
  }
  assert.equal(getterReads, 0, 'foreign Handle rejects before argument getter traversal');

  assert.equal(typeof local.objects.NativeLocator.combine, 'function', 'Locator.combine must exist');
  rejectForeign(local, foreign, 'NativeLocator', () =>
    local.objects.NativeLocator.combine(foreign.objects.NativeLocator, 'and'), 'Locator.combine');
  let predicateCalls = 0;
  assert.equal(typeof local.objects.NativeFrameTree.lookup, 'function', 'FrameTree.lookup must exist');
  rejectForeign(local, foreign, 'NativeContextSettings', () => local.objects.NativeFrameTree.lookup(
    undefined, () => { predicateCalls++; return true; }, foreign.objects.NativeContextSettings, [],
  ), 'FrameTree.lookup');
  assert.equal(predicateCalls, 0, 'foreign settings reject before URL predicate invocation');
}

function borrowedReceivers(local, foreign) {
  const cases = [
    ['NativeEnvironment', 'connectOverCDP', [endpoint, 5000]],
    ['NativeSession', 'version', []],
    ['NativeSessionSignal', 'wait', []],
    ['NativeFrameTree', 'frames', []],
    ['NativeFrameNode', 'url', []],
    ['NativeContextSettings', 'resolveUrl', ['about:blank']],
    ['NativeLocator', 'read', ['count', 5000, undefined]],
    ['NativeHandle', 'preview', []],
  ];
  for (const [name, method, args] of cases) {
    const borrowed = local.addon[name].prototype[method];
    assert.equal(typeof borrowed, 'function', `${name}.${method}: required borrowed method exists`);
    rejectForeign(local, foreign, name, () => Reflect.apply(
      borrowed, foreign.objects[name], args,
    ), `${name}.${method} borrowed receiver`);
  }
  // V8 may reject these at its FunctionTemplate signature before Rust executes.
  // These are live boundary regressions, not standalone proof of cast ordering;
  // that proof also requires the locked-source validation-order gate and the
  // Rust proof that every raw class uses a distinct module-local TypeTag.
}

function accounting(fixture) {
  const host = fixture.session.metrics();
  return {
    ffi: fixture.session.ffiMetrics(),
    queries: fixture.session.locatorMetrics(),
    pendingCalls: host.pendingCalls,
    activeOperations: host.activeOperations,
    cleanupInFlight: host.cleanupInFlight,
  };
}

async function assertUsable(fixture) {
  const { addon, objects, session } = fixture;
  assert.deepEqual(await session.handleJsonValue(objects.NativeHandle), { answer: 42 });
  assert.equal(objects.NativeHandle.isElement(), false);
  assert.equal(typeof objects.NativeHandle.preview(), 'string');
  assert.equal(objects.NativeFrameNode.url(), 'about:blank');
  assert.equal(objects.NativeFrameNode.isDetached(), false);
  assert.ok(objects.NativeFrameTree.frames().some(frame => frame.id === fixture.page.mainFrameId));
  assert.equal(objects.NativeFrameTree.isClosed(), false);
  assert.equal(objects.NativeContextSettings.resolveUrl('about:blank'), 'about:blank');
  assert.equal(await objects.NativeLocator.read('count', 5000, undefined), 1);
  assert.equal(Reflect.apply(addon.NativeSession.prototype.version, session, []), session.version());
}

for (const [name, localAddon, foreignAddon] of [
  ['current rejects legacy classes', primary, legacy],
  ['current rejects independent same-build classes', primary, sameBuild],
  ['same-build copy rejects current classes', sameBuild, primary],
]) {
  test(`native-class module isolation: ${name}`, { timeout: 25000 }, async () => {
    const fixtures = [];
    let primaryFailure;
    try {
      // Arrange
      const local = await createFixture(localAddon, 'local', fixtures);
      const foreign = await createFixture(foreignAddon, 'foreign', fixtures);
      await assertUsable(local);
      await assertUsable(foreign);
      assertBrands(local, foreign);
      const localBefore = accounting(local);
      const foreignBefore = accounting(foreign);
      // Act
      rawArguments(local, foreign);
      borrowedReceivers(local, foreign);
      // Assert
      assert.deepEqual(accounting(local), localBefore, 'rejections must not admit local work');
      assert.deepEqual(accounting(foreign), foreignBefore, 'rejections must not admit foreign work');
      await assertUsable(local);
      await assertUsable(foreign);
    } catch (error) {
      primaryFailure = error;
      throw error;
    } finally {
      await cleanupFixtures(fixtures, primaryFailure);
    }
  });
}
