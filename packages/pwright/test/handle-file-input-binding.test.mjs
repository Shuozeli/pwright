// LIVE GATE — UNEXECUTED when authored for NODE-174 and extended through NODE-191.
// Requires the externally
// managed dedicated loopback Chrome and a selected retained-file-input build.
// This is actual NativeHandle/preparation-ticket coverage up to ArgumentsReady
// or raw Materialized/Marked followed by abort, not public ElementHandle.setInputFiles.
// The NODE-177 cases author real session/ticket registration checks and scheduled
// in-memory serialization, not full dispatcher/native mutation/GC parity.
// NODE-181 cases constrain observable original-source identity, language error
// order and explicit-disposal retention. Existing source metadata could satisfy
// them: no later native Weak upgrade or collected-wrapper behavior is proved.
// NODE-190 cases import the serialized graph into one bounded Rust owner and
// release its direct V8 roots. NODE-191 then consumes that owner in one
// non-retargeting utility-world CDP mutation. The final NODE-191 matrix reaches
// pinned Playwright's in-process instrumentation to compare payloads, events,
// listener errors, late type changes, deadlines and lifecycle races at the
// same post-prevalidation boundary, plus an explicit maximum-payload gate.
// Cleanup accounting covers native-host/FFI/preparation resources only, not
// the separate timeout-policy ledgers retained by raw client metadata wrappers.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { isDeepStrictEqual } from 'node:util';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { bindingFailureDiagnostic, isActiveOwnershipIdle, isFullyRetiredOwnership, isIdleWithOwnerLookupPreparation } from './handle-file-input-binding-observation.mjs';

const require = createRequire(import.meta.url);
const { preflight, prepareChannel } = require('../src/file-input-preflight.cjs');
const { serverValidationPorts } = require('../src/file-input-channel.cjs');
const { asyncMaterialize } = require('../src/file-input-materialization.cjs');
const { prepareServerPayloads, prepareMarkedTarget, prepareProjectedTarget,
  preparePrevalidatedTarget, prepareInstrumentedTarget,
  prepareSerializedTarget, prepareImportedTarget, prepareMutatedTarget } = require('../src/file-input-dispatch.cjs');
const { serializeFileInputCallArgument } = require('../src/file-input-call-argument.cjs');
const debugInputs = require('../src/file-input-debug.cjs');
const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracleChromium } = oracleRequire('playwright');
const { chromium, compatibility } = await import('@shuozeli/pwright');
assert.ok(['retained-file-input', 'retained-file-input-combined', 'retained-target-marking',
  'retained-target-marking-combined', 'retained-target-prevalidation',
  'retained-target-prevalidation-combined', 'retained-input-instrumentation',
  'retained-input-instrumentation-combined', 'retained-call-argument-serialization',
  'retained-call-argument-serialization-combined', 'retained-serialized-payload-import',
  'retained-serialized-payload-import-combined', 'retained-serialized-payload-mutation',
  'retained-serialized-payload-mutation-combined'].includes(compatibility.nativeProfile));
const targetMutation = compatibility.nativeProfile.startsWith('retained-serialized-payload-mutation');
const targetImport = targetMutation || compatibility.nativeProfile.startsWith('retained-serialized-payload-import');
const targetSerialization = targetImport || compatibility.nativeProfile.startsWith('retained-call-argument-serialization');
const targetInstrumentation = targetSerialization || compatibility.nativeProfile.startsWith('retained-input-instrumentation');
const targetPrevalidation = targetInstrumentation || compatibility.nativeProfile.startsWith('retained-target-prevalidation');
const targetMarking = targetPrevalidation || compatibility.nativeProfile.startsWith('retained-target-marking');
assert.equal(process.env.PWRIGHT_TEST_CDP, 'http://127.0.0.1:9222');
// The package script injects this before import so parent/default traversal is
// observable even when the surrounding shell normally enables the inspector.
assert.equal(debugInputs[0], '0', 'run this gate with PWDEBUG=0 before module import');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const EXERCISE_MS = 20000;
const CLEANUP_PHASE_MS = 3000;
const CASE_MS = 35000; // 20s exercise + four 3s cleanup phases + 3s margin.
const MAX_SOURCE_BYTES = 50 * 1024 * 1024 - 1;
const MAX_MEMORY_CASE_MS = 120000;
let poisonedFixture;

async function bounded(pending, label, timeout, onTimeout = () => {}) {
  let timer;
  try {
    return await Promise.race([pending, new Promise((_, reject) => {
      timer = setTimeout(() => {
        onTimeout();
        reject(new Error(`Diagnostic ${label} exceeded ${timeout}ms`));
      }, timeout);
    })]);
  } finally { clearTimeout(timer); }
}

function preparationMetrics(session) {
  const value = session.fileInputPreparationMetrics();
  return { pending: value.pending, rootContainers: value.rootContainers, rootedValues: value.rootedValues };
}

function enterServerMaterialization(session, preparation) {
  if (targetMarking) {
    session.handleBeginFileInputCall(preparation);
    return session.handleStartFileInputDeadline(preparation);
  }
  session.handleCheckFileInputDispatch(preparation);
  return undefined;
}

async function ownership(browser, label, active = () => {}, signal, observation = {}) {
  let observing = true;
  let previous;
  try {
    return await diagnosticDeadline((async () => {
      while (observing) {
        active();
        if (signal?.aborted) throw new Error('ownership cleanup observation cancelled');
        const host = browser.pwrightMetrics();
        const ffi = browser.pwrightFfiMetrics();
        const value = {
          liveObjects: host.liveObjects, activeLeases: host.activeLeases, liveScopes: host.liveScopes,
          pendingCalls: host.pendingCalls, activeOperations: host.activeOperations, cleanupInFlight: host.cleanupInFlight,
          ffiPending: ffi.pending, ffiQueued: ffi.queued, ffiQueuedCommands: ffi.queuedCommands, ffiRunning: ffi.running,
          ffiReservedBytes: ffi.reservedBytes, ffiWatches: ffi.watches,
          preparation: preparationMetrics(browser._session),
        };
        // Active baselines permit armed watches, not queued delivery awaiting
        // rearm. Keep full counter comparisons; final cleanup requires zeros.
        const idle = observation.retired
          ? isFullyRetiredOwnership(host, ffi, value.preparation)
          : observation.ownerLookupBaseline
            ? isIdleWithOwnerLookupPreparation(host, ffi, value.preparation, observation.ownerLookupBaseline)
            : isActiveOwnershipIdle(host, ffi, value.preparation);
        const encoded = JSON.stringify(value);
        if (idle && previous === encoded) return value;
        previous = idle ? encoded : undefined;
        await setImmediate();
      }
      throw new Error('ownership observation cancelled');
    })(), label);
  } finally { observing = false; }
}

async function exactOwnership(browser, label, expected, active = () => {}) {
  return diagnosticDeadline((async () => {
    for (;;) {
      const current = await ownership(browser, `${label} sample`, active);
      if (isDeepStrictEqual(current, expected)) return current;
      await setImmediate();
    }
  })(), label);
}

async function nativeRejection(pending, label, pattern) {
  // A diagnostic deadline must escape, never count as the expected rejection.
  const result = await diagnosticDeadline(Promise.resolve(pending).then(
    value => ({ rejected: false, value }), error => ({ rejected: true, error }),
  ), label);
  assert.equal(result.rejected, true);
  assert.ok(result.error instanceof Error);
  if (pattern) assert.match(result.error.message, pattern);
  return result.error;
}

async function withPage(exercise, exerciseMs = EXERCISE_MS) {
  if (poisonedFixture) throw new Error('Binding fixture blocked by unresolved prior cleanup', { cause: poisonedFixture });
  const browsers = [];
  const handles = [];
  const preparations = [];
  const contexts = [];
  const lateRetirements = new Set();
  const failures = [];
  const cleanupDetails = [];
  const cleanupFailure = (error, phase = 'late acquisition or exercise timeout', index) => {
    failures.push(error);
    cleanupDetails.push({ phase, index, error });
    poisonedFixture ??= error;
  };
  let primary, retiring = false;
  const active = () => {
    if (retiring) throw new Error('binding fixture is retiring');
  };
  const run = action => { active(); return action(); };
  // A late acquisition cannot resume the exercise or wait for JS GC. Normal
  // cleanup still attempts every owned Context/connection and checks native
  // host, FFI and preparation metrics (not timeout-policy ledger reclamation).
  const retireLate = action => {
    cleanupFailure(new Error('Late acquisition requires best-effort retirement; fixture cannot be reused'));
    const pending = bounded(Promise.resolve().then(action), 'late acquisition retirement', CLEANUP_PHASE_MS);
    lateRetirements.add(pending);
    void pending.then(
      () => lateRetirements.delete(pending),
      error => { lateRetirements.delete(pending); cleanupFailure(error); },
    );
  };
  const connect = async () => {
    active();
    const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
    browsers.push(browser);
    if (retiring) { retireLate(() => browser.close()); active(); }
    return browser;
  };
  const body = async () => {
    const browser = await connect();
    active();
    const session = browser._session;
    for (const method of ['frameEvaluateHandleWithPageTimeout', 'handlePrepareFileInput',
      'handleFileInputPreparationFrame', 'handleCheckFileInputDispatch', 'fileInputPreparationMetrics'])
      assert.equal(typeof session[method], 'function', `selected raw export ${method}`);
    assert.equal(typeof session.handleMarkFileInputTarget,
      targetMarking ? 'function' : 'undefined', 'target marking stays profile-gated');
    assert.equal(typeof session.handlePrevalidateFileInputTarget,
      targetPrevalidation ? 'function' : 'undefined', 'target prevalidation stays profile-gated');
    for (const method of ['handleBeginFileInputInstrumentation', 'handleFinishFileInputInstrumentation'])
      assert.equal(typeof session[method], targetInstrumentation ? 'function' : 'undefined',
        `${method} stays profile-gated`);
    assert.equal(typeof session.handleSerializeFileInputPayloads,
      targetSerialization ? 'function' : 'undefined',
      'late call-argument serialization stays profile-gated');
    assert.equal(typeof session.handleImportSerializedFileInputPayloads,
      targetImport ? 'function' : 'undefined',
      'serialized payload import stays profile-gated');
    assert.equal(typeof session.handleMutateSerializedFileInputPayloads,
      targetMutation ? 'function' : 'undefined',
      'serialized payload mutation stays profile-gated');
    for (const method of ['handleBeginFileInputCall', 'handleStartFileInputDeadline'])
      assert.equal(typeof session[method], targetMarking ? 'function' : 'undefined',
        `${method} stays profile-gated`);
    const context = await run(() => browser.newContext());
    contexts.push(context);
    if (retiring) { retireLate(() => context.close()); active(); }
    const page = await run(() => context.newPage());
    active();
    assert.deepEqual(preparationMetrics(session), { pending: 0, rootContainers: 0, rootedValues: 0 });
    const remember = (handle, owner = session) => {
      handles.push({ owner, handle });
      if (retiring) retireLate(() => owner.handleDispose(handle));
      return handle;
    };
    const keep = async (pending, owner = session) => {
      const handle = remember(await pending, owner);
      active();
      return handle;
    };
    const produce = (source = 'document.createElement("input")', creationPage = page, claimedPage = creationPage) => run(() => keep(
      session.frameEvaluateHandleWithPageTimeout(creationPage.mainFrame()._id, source, false, undefined, claimedPage._id),
    ));
    const prepare = (handle, ownerPage = page, options = {}, owner = session) => run(() => {
      // Upstream permits public ownerFrame override. The chosen DOM-owner
      // metadata must not replace the raw Handle's creation-Page timeout cell.
      const receiver = { ownerFrame: () => ownerPage.mainFrame() };
      const preparation = owner.handlePrepareFileInput(handle, receiver, [], options);
      preparations.push(preparation);
      return preparation;
    });
    return exercise({ browser, session, context, page, connect, keep, remember, produce, prepare, preparations, active, run });
  };
  try {
    return await bounded(body(), 'binding setup/exercise', exerciseMs, () => {
      retiring = true;
      // Even successful cleanup of the currently known owners cannot prove
      // reclamation of an acquisition that has not settled yet. Never open
      // another fixture after this deadline; late retirement is best effort.
      cleanupFailure(new Error('Binding exercise deadline leaves late acquisition state unverified'));
    });
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    retiring = true;
    const phase = async (label, actions) => {
      const controller = new AbortController();
      // Invoke every action before awaiting any completion. A rejection or
      // timeout never prevents subsequent Context/all-connection phases.
      const pending = actions.map(action => Promise.resolve().then(() => action(controller.signal)));
      try {
        const outcomes = await bounded(Promise.allSettled(pending), label, CLEANUP_PHASE_MS,
          () => controller.abort());
        for (const [index, outcome] of outcomes.entries())
          if (outcome.status === 'rejected') cleanupFailure(outcome.reason, label, index);
      } catch (error) { cleanupFailure(error, label); }
      finally { controller.abort(); }
    };
    // Hold all raw wrappers/tickets strongly through explicit cleanup metrics.
    await phase('abort preparations and dispose raw Handles', [
      ...preparations.map(preparation => () => preparation.abort()),
      // At most five raw owners in these cases, within the eight-call budget.
      ...handles.map(({ owner, handle }) => () => owner.handleDispose(handle)),
    ]);
    await phase('retire every owned Context', contexts.map(context => () => context.close()));
    await phase('retire every connection', browsers.map(browser => () => browser.close()));
    await phase('zero native-host/FFI/preparation resources', [
      () => Promise.all([...lateRetirements]),
      ...browsers.map(browser => async signal => {
        const value = await ownership(browser, 'final binding ownership', undefined, signal, { retired: true });
        assert.equal(value.activeLeases, 0);
        assert.equal(value.pendingCalls, 0);
        assert.equal(value.activeOperations, 0);
        assert.equal(value.cleanupInFlight, 0);
        assert.equal(value.ffiPending, 0);
        assert.equal(value.ffiReservedBytes, 0);
        assert.deepEqual(value.preparation, { pending: 0, rootContainers: 0, rootedValues: 0 });
      }),
    ]);
    if (primary || failures.length) {
      // Logging is best effort only; never replace the original thrown value or
      // stop any cleanup phase because a diagnostic property/write failed.
      try { process.stderr.write(`${bindingFailureDiagnostic(primary, cleanupDetails)}\n`); }
      catch { /* The original failure still escapes below. */ }
    }
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures,
      'retained binding cleanup failed', primary ? { cause: primary } : undefined);
  }
}

async function withOraclePage(exercise) {
  const browser = await oracleChromium.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  let primary;
  try {
    context = await browser.newContext();
    return await exercise(await context.newPage());
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    const failures = [];
    try { if (context) await diagnosticDeadline(context.close(), 'oracle mutation Context cleanup'); }
    catch (error) { failures.push(error); }
    try { await diagnosticDeadline(browser.close(), 'oracle mutation Browser disconnect'); }
    catch (error) { failures.push(error); }
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures,
      'oracle mutation cleanup failed', primary ? { cause: primary } : undefined);
  }
}

async function observedFiles(page, id, before, after) {
  return page.evaluate(async ({ id, before, after }) => {
    const input = document.getElementById(id);
    return {
      files: input.files === null ? null : await Promise.all([...input.files].map(async file => ({
        name: file.name,
        type: file.type,
        bytes: [...new Uint8Array(await file.arrayBuffer())],
        modified: file.lastModified >= before && file.lastModified <= after ? 'call-time' : file.lastModified,
      }))),
      events: globalThis.__node191MatrixEvents,
      errors: globalThis.__node191MatrixErrors,
    };
  }, { id, before, after });
}

async function installObservedInput(page, { id = 'node191-matrix', multiple = false, throwOnInput = false } = {}) {
  await page.evaluate(({ id, multiple, throwOnInput }) => {
    const input = Object.assign(document.createElement('input'), { id, type: 'file', multiple });
    globalThis.__node191MatrixEvents = [];
    globalThis.__node191MatrixErrors = [];
    globalThis.addEventListener('error', event => {
      globalThis.__node191MatrixErrors.push(event.message);
      event.preventDefault();
    });
    for (const type of ['input', 'change']) input.addEventListener(type, event => {
      globalThis.__node191MatrixEvents.push({
        type: event.type, bubbles: event.bubbles, cancelable: event.cancelable,
        composed: event.composed, target: event.target === input,
      });
    });
    if (throwOnInput) input.addEventListener('input', () => { throw new Error('node191-listener-sentinel'); });
    document.body.append(input);
  }, { id, multiple, throwOnInput });
}

async function warmMutationMarker(fixture, raw, createPreparation) {
  const { browser, session, page, prepare, active } = fixture;
  const before = await ownership(browser, 'before matrix marker warmup', active);
  const warm = (createPreparation ?? (options => prepare(raw, page, options)))({
    timeout: 0,
    payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
  });
  await diagnosticDeadline(prepareMarkedTarget(session, warm), 'matrix marker warmup');
  const held = browser.pwrightMetrics();
  assert.ok(held.activeLeases > before.activeLeases);
  assert.equal(warm.abort(), true);
  return diagnosticDeadline((async () => {
    for (;;) {
      const current = await ownership(browser, 'matrix marker warmup cleanup sample', active);
      if (current.activeLeases < held.activeLeases) return current;
      await setImmediate();
    }
  })(), 'matrix marker warmup cleanup');
}

function deferred() {
  let resolve;
  const promise = new Promise(fulfill => { resolve = fulfill; });
  return { promise, resolve };
}

function installOracleInputInstrumentation(page, listener) {
  // Pinned test-only access to Playwright's in-process server object lets the
  // oracle pause at the same awaited onBeforeInputAction boundary as Pwright.
  // This is not package production code and is guarded by the exact 1.58.2
  // assertion above.
  const connection = page._connection;
  assert.equal(typeof connection?.toImpl, 'function');
  const pageImpl = connection.toImpl(page);
  const contextImpl = connection.toImpl(page.context());
  assert.equal(typeof pageImpl?.instrumentation?.addListener, 'function');
  pageImpl.instrumentation.addListener(listener, contextImpl);
  return () => pageImpl.instrumentation.removeListener(listener);
}

async function collectGarbage() {
  assert.equal(typeof global.gc, 'function', 'run the maximum-payload gate with --expose-gc');
  for (let index = 0; index < 3; index++) {
    global.gc();
    await setImmediate();
  }
}

async function readyAndAbort(fixture, raw, { ownerPage = fixture.page, options = {}, channel = false } = {}) {
  const { browser, session, prepare, active } = fixture;
  const baseline = await ownership(browser, 'before preparation', active);
  active();
  const preparation = prepare(raw, ownerPage, options);
  const initial = preparationMetrics(session);
  assert.equal(initial.pending, baseline.preparation.pending + 1);
  assert.equal(initial.rootContainers, baseline.preparation.rootContainers + 1);
  assert.equal(initial.rootedValues, baseline.preparation.rootedValues + 3);
  const creationFrame = session.handleFileInputPreparationFrame(preparation);
  const result = await diagnosticDeadline(channel ? prepareChannel(preparation) : preflight(preparation),
    channel ? 'actual ticket to ChannelReady' : 'actual ticket to ArgumentsReady');
  active();
  assert.equal(result, preparation);
  assert.equal(preparation.isPending(), true);
  assert.equal(session.handleFileInputPreparationFrame(preparation), creationFrame);
  assert.deepEqual(preparationMetrics(session), {
    pending: baseline.preparation.pending + 1,
    rootContainers: baseline.preparation.rootContainers + 1,
    rootedValues: baseline.preparation.rootedValues + 6,
  });
  assert.equal(preparation.abort(), true);
  assert.equal(preparation.isPending(), false);
  assert.equal(preparation.abort(), false);
  assert.deepEqual(await ownership(browser, 'aborted preparation reclamation', active), baseline);
  return creationFrame;
}

test('live binding: owner override does not replace the creation Page timeout policy', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { session, context, page, produce, active, run } = fixture;
  const ownerPage = await run(() => context.newPage());
  active();
  const raw = await produce();
  // Update after publication: the Handle must retain the canonical live cell,
  // not a flattened snapshot of the old absent Page default.
  const creationSettings = run(() => session.fileInputPageTimeoutSettings(page._id));
  run(() => creationSettings.setDefaultTimeout(17));
  assert.equal(raw.isElement(), true);
  let timeoutReads = 0;
  const options = { get timeout() { timeoutReads++; return undefined; } };
  // Act
  const creationFrame = await readyAndAbort(fixture, raw, { ownerPage, options });
  // Assert: spread + creation Page lookup = two reads. The owner Page has no
  // default, so wrongly choosing it would traverse its Context and read three.
  assert.equal(creationFrame, page.mainFrame()._id);
  assert.notEqual(creationFrame, ownerPage.mainFrame()._id);
  assert.equal(timeoutReads, 2);
}));

test('live binding: generic Handle descendants retain policy through evaluateHandle, property and batch publication', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { session, page, produce, keep, remember, active, run } = fixture;
  const source = await produce('(() => { const node = document.createElement("input"); return { first: node, second: node }; })()');
  assert.equal(source.isElement(), false);
  const evaluated = await keep(run(() => session.handleEvaluateHandle(source, 'value => value.first', true, undefined)));
  const property = await keep(run(() => session.handleGetProperty(source, 'first')));
  const batch = await run(() => session.handleGetProperties(source));
  // Register every returned owner before another await or assertion.
  for (const value of batch) remember(value.handle);
  active();
  assert.deepEqual(batch.map(value => value.name), ['first', 'second']);
  await run(() => session.handleDispose(source));
  const children = [evaluated, property, ...batch.map(value => value.handle)];
  // Act
  const creationFrames = [];
  for (const child of children) {
    assert.equal(child.isElement(), true);
    creationFrames.push(await readyAndAbort(fixture, child));
  }
  // Assert
  assert.equal(children.length, 4);
  assert.equal(new Set(children).size, 4);
  assert.ok(creationFrames.every(id => id === page.mainFrame()._id));
}));

test('live binding: disposed source keeps client preparation metadata but cannot perform native reads', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { session, page, produce, run } = fixture;
  const raw = await produce();
  await run(() => session.handleDispose(raw));
  // Act
  const creationFrame = await readyAndAbort(fixture, raw);
  // Assert
  assert.equal(creationFrame, page.mainFrame()._id);
  await nativeRejection(Promise.resolve().then(() => run(() => session.handleEvaluate(raw, 'value => value.tagName', true, undefined))),
    'disposed raw source read', /disposed/i);
}));

test('live original source: substituting a live native receiver cannot replace the disposed preparation target', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: both inputs are real native wrappers. Only the first argument is
  // the admitted source; the second is an overridable language receiver.
  const { browser, session, context, page, produce, preparations, active, run } = fixture;
  const otherPage = await run(() => context.newPage());
  active();
  const original = await produce();
  const substitute = await produce('document.createElement("input")', otherPage);
  let releaseOwner;
  const ownerWait = new Promise(resolve => { releaseOwner = resolve; });
  const trace = [];
  Object.defineProperty(substitute, 'ownerFrame', { value() {
    assert.equal(this, substitute);
    trace.push('owner');
    return ownerWait;
  } });
  const preparation = run(() => session.handlePrepareFileInput(original, substitute, [], {}));
  preparations.push(preparation);
  const pending = prepareChannel(preparation).then(
    value => ({ value }), error => ({ error }),
  );
  let outcome;

  // Act: retire the original while the public owner lookup is still waiting.
  try {
    assert.deepEqual(trace, ['owner']);
    await run(() => session.handleDispose(original));
    active();
    assert.equal(preparation.isPending(), true);
  } finally {
    releaseOwner(otherPage.mainFrame());
    outcome = await diagnosticDeadline(pending, 'substitute receiver client continuation');
  }

  // Assert: client language work can finish, but the existing dispatch gate
  // still checks the original target. This is not a later Weak-upgrade test.
  active();
  assert.deepEqual(outcome, { value: preparation });
  assert.equal(session.handleFileInputPreparationFrame(preparation), page.mainFrame()._id);
  assert.notEqual(page.mainFrame()._id, otherPage.mainFrame()._id);
  assert.throws(() => session.handleCheckFileInputDispatch(preparation), /closed/i);
  assert.equal(preparation.isPending(), false);
  assert.equal(await run(() => session.handleEvaluate(substitute, 'value => value.tagName', true, undefined)), 'INPUT');
  active();
  assert.deepEqual(preparationMetrics(session), { pending: 0, rootContainers: 0, rootedValues: 0 });
  assert.equal((await ownership(browser, 'substitute receiver rejected target cleanup', active)).pendingCalls, 0);
}));

test('live original source: pending owner lookup does not pin explicitly disposed native resources', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: baseline is before the only native Handle. The pending ticket's
  // separate FFI/JS roots remain admitted throughout the native-retention check.
  const { browser, session, page, produce, preparations, active, run } = fixture;
  const baseline = await ownership(browser, 'before awaited-owner source creation', active);
  const original = await produce();
  let releaseOwner;
  const ownerWait = new Promise(resolve => { releaseOwner = resolve; });
  let ownerCalls = 0, settled = false;
  const receiver = { ownerFrame() { ownerCalls++; return ownerWait; } };
  const preparation = run(() => session.handlePrepareFileInput(original, receiver, [], {}));
  preparations.push(preparation);
  const pending = preflight(preparation).then(
    value => { settled = true; return { value }; },
    error => { settled = true; return { error }; },
  );
  let outcome;

  // Act / Assert: disposal must settle before resolving the owner wait. Holding
  // a preparation and its original wrapper is not permission to pin a native
  // owner. This explicit-disposal observation is deliberately not GC evidence.
  try {
    assert.equal(ownerCalls, 1);
    await run(() => session.handleDispose(original));
    const duringWait = await ownership(browser, 'native reclamation during owner wait', active, undefined,
      { ownerLookupBaseline: baseline });
    active();
    for (const field of ['liveObjects', 'activeLeases', 'liveScopes', 'pendingCalls', 'activeOperations', 'cleanupInFlight'])
      assert.equal(duringWait[field], baseline[field], `native baseline field ${field}`);
    assert.equal(settled, false);
    assert.equal(preparation.isPending(), true);
    assert.deepEqual(duringWait.preparation, { pending: 1, rootContainers: 1, rootedValues: 3 });
  } finally {
    releaseOwner(page.mainFrame());
    outcome = await diagnosticDeadline(pending, 'disposed source client continuation');
  }
  active();
  assert.deepEqual(outcome, { value: preparation });
  assert.equal(session.handleFileInputPreparationFrame(preparation), page.mainFrame()._id);
  assert.equal(preparation.abort(), true);
  assert.deepEqual(await ownership(browser, 'awaited-owner preparation abort reclamation', active), baseline);
}));

test('live original source: disposal during owner wait does not replace a later client getter exception', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: no dispatcher/native phase is reached. A future weak binding must
  // not add a disposal check between the owner await and the client conversion.
  const { browser, session, page, produce, preparations, active, run } = fixture;
  const baseline = await ownership(browser, 'before deferred client failure source', active);
  const original = await produce();
  const expected = new Error('client slice getter wins after source disposal');
  const trace = [];
  let releaseOwner;
  const ownerWait = new Promise(resolve => { releaseOwner = resolve; });
  const receiver = { ownerFrame() { trace.push('owner'); return ownerWait; } };
  const files = [];
  Object.defineProperty(files, 'slice', { get() { trace.push('slice'); throw expected; } });
  const options = { get timeout() { trace.push('timeout'); return 1; } };
  const preparation = run(() => session.handlePrepareFileInput(original, receiver, files, options));
  preparations.push(preparation);
  const pending = prepareChannel(preparation).then(
    value => ({ value }), error => ({ error }),
  );
  let outcome;

  // Act
  try {
    assert.deepEqual(trace, ['owner']);
    await run(() => session.handleDispose(original));
    active();
    assert.deepEqual(trace, ['owner']);
  } finally {
    releaseOwner(page.mainFrame());
    outcome = await diagnosticDeadline(pending, 'client getter after source disposal');
  }

  // Assert: these are raw internal errors, not public API-prefix parity.
  active();
  assert.deepEqual(outcome, { error: expected });
  assert.equal(outcome.error, expected);
  assert.equal(expected.message, 'client slice getter wins after source disposal');
  assert.deepEqual(trace, ['owner', 'slice']);
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'deferred client failure reclamation', active), baseline);
}));

test('live binding: closed creation Page preserves bound preparation metadata without authorizing browser use', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { session, page, produce, run } = fixture;
  const raw = await produce();
  const frameId = page.mainFrame()._id;
  await run(() => page.close());
  // Act
  const creationFrame = await readyAndAbort(fixture, raw);
  // Assert: the public owner override supplies retained local metadata only.
  assert.equal(creationFrame, frameId);
  await nativeRejection(Promise.resolve().then(() => run(() => session.handleEvaluate(raw, 'value => value.tagName', true, undefined))),
    'closed-Page raw source read', /closed|destroyed|detached|navigation/i);
}));

test('live binding: wrong creation Page rejects publication and releases the produced native owner', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, context, page, produce, active, run } = fixture;
  const otherPage = await run(() => context.newPage());
  active();
  const baseline = await ownership(browser, 'before wrong Page producer', active);
  // Act: both Pages are real members; only result provenance is wrong. keep()
  // still owns an unexpected successful result so test failure cannot leak it.
  const rejected = produce('(() => { globalThis.__bindingWrongPageRan = true; return document.createElement("input"); })()', page, otherPage);
  // Assert
  await nativeRejection(rejected, 'wrong creation Page publication', /creation Page/i);
  assert.equal(await run(() => page.evaluate(() => globalThis.__bindingWrongPageRan)), true);
  assert.deepEqual(await ownership(browser, 'wrong Page native compensation', active), baseline);
}));

test('live binding: a same-environment sibling session cannot prepare another session source', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, session, page, produce, connect, preparations, active, run } = fixture;
  const raw = await produce();
  const sibling = await connect();
  const baseline = await ownership(browser, 'before foreign source preparation', active);
  let ownerReads = 0;
  const receiver = { ownerFrame() { ownerReads++; return page.mainFrame(); } };
  // Act
  let rejected;
  try {
    const unexpected = run(() => sibling._session.handlePrepareFileInput(raw, receiver, [], {}));
    preparations.push(unexpected);
  } catch (error) { rejected = error; }
  // Assert
  assert.ok(rejected instanceof Error);
  assert.match(rejected.message, /another session/i);
  assert.equal(ownerReads, 0);
  assert.deepEqual(preparationMetrics(sibling._session), preparationMetrics(session));
  assert.deepEqual(await ownership(browser, 'foreign preparation rejection reclamation', active), baseline);
}));

test('live channel: validated ticket retains six roots then aborts to its native resource baseline', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { page, produce } = fixture;
  const raw = await produce();
  // Act
  const creationFrame = await readyAndAbort(fixture, raw, { channel: true });
  // Assert
  assert.equal(creationFrame, page.mainFrame()._id);
}));

test('live channel: channel getter failure aborts actual preparation and preserves the raw exception', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const original = new Error('channel getter failed');
  const options = { payloads: [{ get name() { throw original; }, buffer: Buffer.from([1]) }] };
  const baseline = await ownership(browser, 'before channel getter failure', active);
  const preparation = prepare(raw, page, options);
  // Act: options override converted payloads after the size/conversion phase.
  const error = await nativeRejection(prepareChannel(preparation), 'actual channel getter rejection');
  // Assert: this internal helper does not implement public API error wrapping.
  active();
  assert.equal(error, original);
  assert.equal(error.message, 'channel getter failed');
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'channel failure root reclamation', active), baseline);
}));

test('live materialization: raw stages retain six roots and preserve params through outer await', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: manual kernel sequencing with registration checks, not execution.
  const { browser, session, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const baseline = await ownership(browser, 'before raw materialization', active);
  const preparation = prepare(raw, page, { payloads: [{ name: 'one.txt', buffer: Buffer.from('one') }] });
  await diagnosticDeadline(prepareChannel(preparation), 'channel before materialization');
  await setImmediate();
  active();
  session.handleCheckFileInputDispatch(preparation);
  preparation.validateServerParams(serverValidationPorts);
  enterServerMaterialization(session, preparation);
  // Act
  const carrier = await diagnosticDeadline(asyncMaterialize(preparation), 'raw materialization');
  active();
  preparation.finishMaterialization(carrier);
  // Assert
  assert.deepEqual(carrier.filePayloads, [{ name: 'one.txt', mimeType: 'text/plain', buffer: 'b25l', lastModifiedMs: undefined }]);
  assert.deepEqual(preparationMetrics(session), { pending: 1, rootContainers: 1, rootedValues: 6 });
  assert.equal(preparation.abort(), true);
  assert.deepEqual(await ownership(browser, 'materialized root reclamation', active), baseline);
}));

test('live materialization: serializer exception aborts without a native execution', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, session, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const original = new Error('serializer failed');
  const bytes = Buffer.from([1]);
  bytes.toString = () => { throw original; };
  const baseline = await ownership(browser, 'before serializer failure', active);
  const preparation = prepare(raw, page, { payloads: [{ name: 'one.bin', buffer: bytes }] });
  await diagnosticDeadline(prepareChannel(preparation), 'channel before serializer failure');
  active();
  session.handleCheckFileInputDispatch(preparation);
  preparation.validateServerParams(serverValidationPorts);
  enterServerMaterialization(session, preparation);
  // Act
  const error = await nativeRejection(asyncMaterialize(preparation), 'serializer rejection');
  // Assert: raw helper errors have not crossed server error transport.
  active();
  assert.equal(error, original);
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'serializer failure reclamation', active), baseline);
}));

test('live materialization: callback abort cannot resurrect replacement roots', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, session, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const bytes = Buffer.from([1]);
  const baseline = await ownership(browser, 'before serializer revocation', active);
  const preparation = prepare(raw, page, { payloads: [{ name: 'one.bin', buffer: bytes }] });
  bytes.toString = () => { assert.equal(preparation.abort(), true); return 'AQ=='; };
  await diagnosticDeadline(prepareChannel(preparation), 'channel before serializer revocation');
  active();
  session.handleCheckFileInputDispatch(preparation);
  preparation.validateServerParams(serverValidationPorts);
  enterServerMaterialization(session, preparation);
  // Act
  await nativeRejection(asyncMaterialize(preparation), 'callback revocation');
  // Assert
  active();
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'revocation reclamation', active), baseline);
}));

test('live materialization: abort between begin and finish revokes the awaited carrier', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, session, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const baseline = await ownership(browser, 'before outer await revocation', active);
  const preparation = prepare(raw, page);
  await diagnosticDeadline(prepareChannel(preparation), 'channel before outer await revocation');
  active();
  session.handleCheckFileInputDispatch(preparation);
  preparation.validateServerParams(serverValidationPorts);
  enterServerMaterialization(session, preparation);
  const pending = asyncMaterialize(preparation);
  assert.equal(preparation.abort(), true);
  const carrier = await diagnosticDeadline(pending, 'carrier after abort');
  active();
  // Act / Assert
  assert.throws(() => preparation.finishMaterialization(carrier), Error);
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'outer await revocation reclamation', active), baseline);
}));

test('live dispatch: scheduled driver reaches Materialized and explicit abort reclaims its ticket', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: this driver intentionally stops before DOM validation or upload.
  const { browser, session, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const baseline = await ownership(browser, 'before scheduled materialization', active);
  const bytes = Buffer.from('one');
  const calls = [];
  bytes.toString = function (encoding) {
    calls.push({ receiver: this, encoding });
    return 'b25l';
  };
  const preparation = prepare(raw, page, { payloads: [{ name: 'one.txt', buffer: bytes }] });

  // Act
  const result = await diagnosticDeadline(prepareServerPayloads(session, preparation),
    'actual ticket scheduled to Materialized');

  // Assert
  active();
  assert.equal(result, undefined);
  assert.equal(preparation.isPending(), true);
  assert.deepEqual(calls, [{ receiver: bytes, encoding: 'base64' }]);
  assert.deepEqual(preparationMetrics(session), { pending: 1, rootContainers: 1, rootedValues: 6 });
  assert.equal(preparation.abort(), true);
  assert.equal(preparation.abort(), false);
  assert.deepEqual(await ownership(browser, 'scheduled materialization abort reclamation', active), baseline);
}));

test('live native marking: one Node progress advances Materialized to Marked on the same FFI ticket',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce();
    const baseline = await ownership(browser, 'before native target marking', active);
    const preparation = prepare(raw, page, {
      payloads: [{ name: 'one.txt', buffer: Buffer.from('one') }],
    });
    // Act: call identity, one Node deadline, materialization and marking are
    // owned by one coordinator; no JS-supplied marker identity is accepted.
    const result = await diagnosticDeadline(prepareMarkedTarget(session, preparation),
      'actual original Handle target marking');

    // Assert: completion returned to the same parked 1 MiB command admission;
    // no second ticket, queued result or running worker remains.
    active();
    assert.equal(result, undefined);
    assert.equal(preparation.isPending(), true);
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: baseline.preparation.rootedValues + 6,
    });
    const marked = browser.pwrightFfiMetrics();
    assert.equal(marked.pending, baseline.ffiPending + 1);
    assert.equal(marked.queued, baseline.ffiQueued);
    assert.equal(marked.queuedCommands, baseline.ffiQueuedCommands);
    assert.equal(marked.running, baseline.ffiRunning);
    assert.equal(marked.reservedBytes, baseline.ffiReservedBytes + 1024 * 1024);
    assert.equal(preparation.abort(), true);
    await setImmediate();
    assert.deepEqual(preparationMetrics(session), baseline.preparation);
    const reclaimed = browser.pwrightFfiMetrics();
    assert.equal(reclaimed.pending, baseline.ffiPending);
    assert.equal(reclaimed.queued, baseline.ffiQueued);
    assert.equal(reclaimed.running, baseline.ffiRunning);
    assert.equal(reclaimed.reservedBytes, baseline.ffiReservedBytes);
  }));

test('live native marking: protocol call identities are monotonic and independent of preparation IDs',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange
    const { browser, session, page, produce, prepare, active, run } = fixture;
    const baseline = await ownership(browser, 'before public call identities', active);
    const firstRaw = await produce();
    const secondRaw = await produce();
    const first = prepare(firstRaw, page, { timeout: 0 });
    const second = prepare(secondRaw, page, { timeout: 0 });
    for (const preparation of [first, second]) {
      await diagnosticDeadline(prepareChannel(preparation), 'channel before call identity');
      session.handleCheckFileInputDispatch(preparation);
      preparation.validateServerParams(serverValidationPorts);
    }

    // Act
    const identities = [
      session.handleBeginFileInputCall(first),
      session.handleBeginFileInputCall(second),
    ];

    // Assert
    active();
    assert.deepEqual(identities, ['call@1', 'call@2']);
    assert.equal(first.abort(), true);
    assert.equal(second.abort(), true);
    await run(() => session.handleDispose(firstRaw));
    await run(() => session.handleDispose(secondRaw));
    assert.deepEqual(await ownership(browser, 'public call identity reclamation', active), baseline);
  }));

test('live native projection: marked owner and ten roots remain on the same parked ticket until abort',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce();
    // Establish the independently persistent utility-world marker first. The
    // projection assertion below must measure only its per-call owner, not
    // incorrectly demand that ordinary call abort destroy shared marker state.
    const beforeWarm = await ownership(browser, 'before persistent marker cache warmup', active);
    const warm = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
    });
    await diagnosticDeadline(prepareMarkedTarget(session, warm), 'marker cache warmup');
    const warmHeld = browser.pwrightMetrics();
    assert.ok(warmHeld.activeLeases > beforeWarm.activeLeases);
    assert.equal(warm.abort(), true);
    const baseline = await diagnosticDeadline((async () => {
      for (;;) {
        const current = await ownership(browser, 'marker warmup cleanup sample', active);
        if (current.activeLeases < warmHeld.activeLeases) return current;
        await setImmediate();
      }
    })(), 'persistent marker cache warmup cleanup');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [
        { name: 'one.txt', buffer: Buffer.from('one') },
        { name: 'two.txt', buffer: Buffer.from('two') },
      ],
    });

    // Act
    await diagnosticDeadline(prepareProjectedTarget(session, preparation),
      'actual marked carrier projection');

    // Assert: four exact raw projection references extend the six prior roots;
    // the marked native owner remains charged inside the same FFI result.
    active();
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: baseline.preparation.rootedValues + 10,
    });
    const projected = browser.pwrightMetrics();
    assert.ok(projected.activeLeases > baseline.activeLeases);
    const ffi = browser.pwrightFfiMetrics();
    assert.equal(ffi.pending, baseline.ffiPending + 1);
    assert.equal(ffi.reservedBytes, baseline.ffiReservedBytes + 1024 * 1024);
    assert.equal(preparation.abort(), true);
    assert.deepEqual(await exactOwnership(browser, 'retained carrier projection reclamation', baseline, active), baseline);
  }));

test('live native prevalidation: marked owner becomes validated on the same parked ticket until abort',
  { timeout: CASE_MS, skip: !targetPrevalidation }, async () => withPage(async fixture => {
    // Arrange: use one real file payload so a non-multiple INPUT is valid.
    // Warm the independently persistent marker cache, then establish a baseline
    // after its transient per-call owner has retired.
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const beforeWarm = await ownership(browser, 'before prevalidation marker warmup', active);
    const warm = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
    });
    await diagnosticDeadline(prepareMarkedTarget(session, warm), 'prevalidation marker warmup');
    const warmHeld = browser.pwrightMetrics();
    assert.ok(warmHeld.activeLeases > beforeWarm.activeLeases);
    assert.equal(warm.abort(), true);
    const baseline = await diagnosticDeadline((async () => {
      for (;;) {
        const current = await ownership(browser, 'prevalidation warmup cleanup sample', active);
        if (current.activeLeases < warmHeld.activeLeases) return current;
        await setImmediate();
      }
    })(), 'prevalidation marker warmup cleanup');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'one.txt', buffer: Buffer.from('one') }],
    });

    // Act
    await diagnosticDeadline(preparePrevalidatedTarget(session, preparation),
      'actual retained DOM target prevalidation');

    // Assert: the ten existing roots and original parked FFI permit remain;
    // the validated Rust owner adds only native leases until terminal abort.
    active();
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: baseline.preparation.rootedValues + 10,
    });
    const validated = browser.pwrightMetrics();
    assert.ok(validated.activeLeases > baseline.activeLeases);
    const ffi = browser.pwrightFfiMetrics();
    assert.equal(ffi.pending, baseline.ffiPending + 1);
    assert.equal(ffi.queued, baseline.ffiQueued);
    assert.equal(ffi.queuedCommands, baseline.ffiQueuedCommands);
    assert.equal(ffi.running, baseline.ffiRunning);
    assert.equal(ffi.reservedBytes, baseline.ffiReservedBytes + 1024 * 1024);
    assert.equal(preparation.abort(), true);
    assert.deepEqual(await exactOwnership(browser, 'retained DOM prevalidation reclamation', baseline, active), baseline);
  }));

test('live native input instrumentation: awaited no-listener boundary preserves the validated owner and ticket',
  { timeout: CASE_MS, skip: !targetInstrumentation }, async () => withPage(async fixture => {
    // Arrange: warm only the independently persistent marker service, then
    // measure the new call from a stable zero-preparation baseline.
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const beforeWarm = await ownership(browser, 'before instrumentation marker warmup', active);
    const warm = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
    });
    await diagnosticDeadline(prepareMarkedTarget(session, warm), 'instrumentation marker warmup');
    const warmHeld = browser.pwrightMetrics();
    assert.ok(warmHeld.activeLeases > beforeWarm.activeLeases);
    assert.equal(warm.abort(), true);
    const baseline = await diagnosticDeadline((async () => {
      for (;;) {
        const current = await ownership(browser, 'instrumentation warmup cleanup sample', active);
        if (current.activeLeases < warmHeld.activeLeases) return current;
        await setImmediate();
      }
    })(), 'instrumentation marker warmup cleanup');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'one.txt', buffer: Buffer.from('one') }],
    });

    // Act
    await diagnosticDeadline(prepareInstrumentedTarget(session, preparation),
      'awaited retained input instrumentation');

    // Assert: instrumentation introduces no root, permit or native owner. The
    // exact ten roots, 1 MiB FFI admission and prevalidated owner stay parked.
    active();
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: baseline.preparation.rootedValues + 10,
    });
    const instrumented = browser.pwrightMetrics();
    assert.ok(instrumented.activeLeases > baseline.activeLeases);
    const ffi = browser.pwrightFfiMetrics();
    assert.equal(ffi.pending, baseline.ffiPending + 1);
    assert.equal(ffi.queued, baseline.ffiQueued);
    assert.equal(ffi.queuedCommands, baseline.ffiQueuedCommands);
    assert.equal(ffi.running, baseline.ffiRunning);
    assert.equal(ffi.reservedBytes, baseline.ffiReservedBytes + 1024 * 1024);
    assert.equal(preparation.abort(), true);
    assert.deepEqual(await exactOwnership(browser, 'input instrumentation reclamation', baseline, active), baseline);
  }));

test('live native input instrumentation: a foreign session cannot cancel the waiting owner ticket',
  { timeout: CASE_MS, skip: !targetInstrumentation }, async () => withPage(async fixture => {
    // Arrange
    const { session, page, connect, produce, prepare, active } = fixture;
    const raw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'owner.txt', buffer: Buffer.from('owner') }],
    });
    await diagnosticDeadline(preparePrevalidatedTarget(session, preparation),
      'owner ticket parked after prevalidation');
    const sibling = await connect();

    // Act / Assert: provenance rejection occurs before exclusive phase checkout.
    assert.throws(() => sibling._session.handleBeginFileInputInstrumentation(preparation), /another session/);
    assert.equal(preparation.isPending(), true);
    const receiver = session.handleBeginFileInputInstrumentation(preparation);
    active();
    assert.equal(typeof receiver.ownerFrame, 'function');
    assert.equal(preparation.isPending(), true);
    session.handleFinishFileInputInstrumentation(preparation);
    assert.equal(preparation.isPending(), true);
    assert.equal(preparation.abort(), true);
  }));

test('live native late serialization: one rooted graph preserves the validated owner and ticket',
  { timeout: CASE_MS, skip: !targetSerialization }, async () => withPage(async fixture => {
    // Arrange: warm the persistent marker service, then retain one nested
    // object returned by Buffer serialization so the actual N-API phase can
    // expose and compare the pinned serialized graph without browser mutation.
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const beforeWarm = await ownership(browser, 'before serialization marker warmup', active);
    const warm = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
    });
    await diagnosticDeadline(prepareMarkedTarget(session, warm), 'serialization marker warmup');
    const warmHeld = browser.pwrightMetrics();
    assert.ok(warmHeld.activeLeases > beforeWarm.activeLeases);
    assert.equal(warm.abort(), true);
    const baseline = await diagnosticDeadline((async () => {
      for (;;) {
        const current = await ownership(browser, 'serialization warmup cleanup sample', active);
        if (current.activeLeases < warmHeld.activeLeases) return current;
        await setImmediate();
      }
    })(), 'serialization marker warmup cleanup');
    const nested = { state: 'before' };
    const bytes = Buffer.from([1]);
    bytes.toString = () => nested;
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'one.bin', mimeType: 'application/octet-stream', buffer: bytes }],
    });
    await diagnosticDeadline(prepareInstrumentedTarget(session, preparation, {
      async onBeforeInputAction() {
        queueMicrotask(() => { nested.state = 'after'; });
      },
    }), 'instrumented owner before actual serialization');
    let captured;
    const capture = value => (captured = serializeFileInputCallArgument(value));

    // Act
    session.handleSerializeFileInputPayloads(preparation, capture);

    // Assert: one new serialized-graph root is added without a second FFI
    // admission or native target. Its large string leaves remain V8 values.
    active();
    const row = captured.a[0].o;
    const buffer = row.find(entry => entry.k === 'buffer').v;
    assert.equal(buffer.o.find(entry => entry.k === 'state').v, 'after');
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: baseline.preparation.rootedValues + 11,
    });
    const serialized = browser.pwrightMetrics();
    assert.ok(serialized.activeLeases > baseline.activeLeases);
    const ffi = browser.pwrightFfiMetrics();
    assert.equal(ffi.pending, baseline.ffiPending + 1);
    assert.equal(ffi.queued, baseline.ffiQueued);
    assert.equal(ffi.queuedCommands, baseline.ffiQueuedCommands);
    assert.equal(ffi.running, baseline.ffiRunning);
    assert.equal(ffi.reservedBytes, baseline.ffiReservedBytes + 1024 * 1024);
    assert.equal(preparation.abort(), true);
    assert.deepEqual(await exactOwnership(browser, 'late serialization reclamation', baseline, active), baseline);
  }));

test('live native late serialization: a foreign session cannot cancel the instrumented owner',
  { timeout: CASE_MS, skip: !targetSerialization }, async () => withPage(async fixture => {
    // Arrange
    const { session, page, connect, produce, prepare } = fixture;
    const raw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'owner.txt', buffer: Buffer.from('owner') }],
    });
    await diagnosticDeadline(prepareInstrumentedTarget(session, preparation),
      'owner ticket parked after instrumentation');
    const sibling = await connect();

    // Act / Assert: provenance is rejected before exclusive checkout, while
    // the rightful session can serialize and then explicitly revoke its owner.
    assert.throws(() => sibling._session.handleSerializeFileInputPayloads(
      preparation, serializeFileInputCallArgument), /another session/i);
    assert.equal(preparation.isPending(), true);
    session.handleSerializeFileInputPayloads(preparation, serializeFileInputCallArgument);
    assert.equal(preparation.isPending(), true);
    assert.equal(preparation.abort(), true);
  }));

test('live native serialized import: exact owner transfers eleven V8 roots into one bounded Rust slot',
  { timeout: CASE_MS, skip: !targetImport }, async () => withPage(async fixture => {
    // Arrange: warm the independently persistent target-marker service, then
    // establish a stable zero-preparation baseline before creating this call.
    const { browser, session, page, connect, produce, prepare, active } = fixture;
    const raw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const sibling = await connect();
    const beforeWarm = await ownership(browser, 'before serialized import marker warmup', active);
    const warm = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
    });
    await diagnosticDeadline(prepareMarkedTarget(session, warm), 'serialized import marker warmup');
    const warmHeld = browser.pwrightMetrics();
    assert.ok(warmHeld.activeLeases > beforeWarm.activeLeases);
    assert.equal(warm.abort(), true);
    const baseline = await diagnosticDeadline((async () => {
      for (;;) {
        const current = await ownership(browser, 'serialized import warmup cleanup sample', active);
        if (current.activeLeases < warmHeld.activeLeases) return current;
        await setImmediate();
      }
    })(), 'serialized import marker warmup cleanup');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'import.bin', mimeType: 'application/octet-stream', buffer: Buffer.from([0, 255, 1]) }],
    });
    await diagnosticDeadline(prepareSerializedTarget(session, preparation),
      'serialized owner before bounded Rust import');
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: baseline.preparation.rootedValues + 11,
    });
    const serialized = browser.pwrightMetrics();
    const serializedFfi = browser.pwrightFfiMetrics();

    // Act / Assert: session provenance wins before the single large owner is
    // admitted; the rightful import then releases every direct V8 root.
    assert.throws(() => sibling._session.handleImportSerializedFileInputPayloads(preparation),
      /another session/i);
    const metrics = session.handleImportSerializedFileInputPayloads(preparation);
    assert.ok(metrics.depth > 0 && metrics.nodes > 0 && metrics.entries > 0);
    assert.ok(metrics.stringBytes >= 'import.bin'.length + 'AP8B'.length);
    assert.deepEqual(preparationMetrics(session), {
      pending: baseline.preparation.pending + 1,
      rootContainers: baseline.preparation.rootContainers + 1,
      rootedValues: 0,
    });
    const imported = browser.pwrightMetrics();
    assert.ok(imported.activeLeases > serialized.activeLeases);
    const ffi = browser.pwrightFfiMetrics();
    assert.equal(ffi.pending, serializedFfi.pending);
    assert.equal(ffi.reservedBytes, serializedFfi.reservedBytes);
    assert.equal(preparation.abort(), true);
    assert.deepEqual(await exactOwnership(browser, 'serialized import reclamation', baseline, active), baseline);
  }));

test('live native serialized mutation: exact bytes/events reach file inputs and late non-file inputs no-op',
  { timeout: CASE_MS, skip: !targetMutation }, async () => withPage(async fixture => {
    // Arrange: warm the persistent exact-world marker so the baseline contains
    // only reusable state, never a per-call owner or preparation ticket.
    const { browser, session, page, produce, prepare, active, run } = fixture;
    const raw = await produce(`(() => {
      const input = Object.assign(document.createElement('input'), { type: 'file', id: 'node191-file' });
      globalThis.__node191Events = [];
      for (const type of ['input', 'change']) input.addEventListener(type, event => {
        globalThis.__node191Events.push({
          type: event.type, bubbles: event.bubbles, cancelable: event.cancelable,
          composed: event.composed, target: event.target === input,
        });
      });
      document.body.append(input);
      return input;
    })()`);
    const beforeWarm = await ownership(browser, 'before serialized mutation marker warmup', active);
    const warm = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'warm.txt', buffer: Buffer.from('warm') }],
    });
    await diagnosticDeadline(prepareMarkedTarget(session, warm), 'serialized mutation marker warmup');
    const warmHeld = browser.pwrightMetrics();
    assert.ok(warmHeld.activeLeases > beforeWarm.activeLeases);
    assert.equal(warm.abort(), true);
    const baseline = await diagnosticDeadline((async () => {
      for (;;) {
        const current = await ownership(browser, 'serialized mutation warmup cleanup sample', active);
        if (current.activeLeases < warmHeld.activeLeases) return current;
        await setImmediate();
      }
    })(), 'serialized mutation marker warmup cleanup');
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{
        name: 'mutation.bin', mimeType: 'application/x-node191',
        buffer: Buffer.from([0, 255, 1]), lastModifiedMs: 123,
      }],
    });

    // Act
    const beforeMutation = Date.now();
    const result = await diagnosticDeadline(
      prepareMutatedTarget(session, preparation),
      'serialized payload mutation terminal settlement');
    const afterMutation = Date.now();

    // Assert: the coordinator exposes no raw ticket, and successful mutation
    // terminally removes the exact registry entry instead of encoding success
    // as abort or waiting for JavaScript GC.
    assert.equal(result, undefined);
    assert.equal(preparation.isPending(), false);
    assert.equal(preparation.abort(), false);
    assert.deepEqual(preparationMetrics(session), baseline.preparation);
    const dom = await run(() => page.evaluate(async () => {
      const input = document.getElementById('node191-file');
      const file = input.files[0];
      return {
        length: input.files.length,
        name: file.name,
        type: file.type,
        lastModified: file.lastModified,
        bytes: Array.from(new Uint8Array(await file.arrayBuffer())),
        events: globalThis.__node191Events,
      };
    }));
    const { lastModified, ...stableDom } = dom;
    assert.ok(lastModified >= beforeMutation && lastModified <= afterMutation,
      'the channel-stripped lastModifiedMs defaults to the browser File creation time');
    assert.deepEqual(stableDom, {
      length: 1,
      name: 'mutation.bin',
      type: 'application/x-node191',
      bytes: [0, 255, 1],
      events: [
        { type: 'input', bubbles: true, cancelable: false, composed: true, target: true },
        { type: 'change', bubbles: true, cancelable: false, composed: false, target: true },
      ],
    });
    assert.deepEqual(await exactOwnership(browser, 'serialized mutation reclamation', baseline, active), baseline);

    // Pinned injectedScript.setInputFiles performs its type=file check only in
    // the final utility call and returns a string that the server ignores.
    const textRaw = await produce(`(() => {
      const input = Object.assign(document.createElement('input'), { type: 'text', id: 'node191-text' });
      globalThis.__node191TextEvents = [];
      for (const type of ['input', 'change']) input.addEventListener(type, () => globalThis.__node191TextEvents.push(type));
      document.body.append(input);
      return input;
    })()`);
    const noOp = prepare(textRaw, page, {
      timeout: 0,
      payloads: [{ name: 'ignored.txt', buffer: Buffer.from('ignored') }],
    });

    // Act / Assert: ignored helper return is still a successful terminal call.
    await diagnosticDeadline(prepareMutatedTarget(session, noOp), 'late non-file mutation no-op');
    assert.equal(noOp.isPending(), false);
    assert.equal(noOp.abort(), false);
    assert.deepEqual(await run(() => page.evaluate(() => {
      const input = document.getElementById('node191-text');
      return { files: input.files, value: input.value, events: globalThis.__node191TextEvents };
    })), { files: null, value: '', events: [] });
    await run(() => session.handleDispose(textRaw));
    assert.deepEqual(await exactOwnership(browser, 'late non-file mutation reclamation', baseline, active), baseline);
  }));

test('live mutation differential: empty and multiple payloads match pinned Playwright',
  { timeout: CASE_MS, skip: !targetMutation }, async () => {
    // Arrange: produce the oracle result through its public retained Handle.
    const payloads = () => [
      { name: 'alpha.txt', mimeType: 'text/plain', buffer: Buffer.from('alpha') },
      { name: 'omega.bin', mimeType: 'application/x-node191', buffer: Buffer.from([0, 255, 1]) },
    ];
    const expected = await withOraclePage(async page => {
      await installObservedInput(page, { multiple: true });
      const handle = await page.$('#node191-matrix');
      const beforeMultiple = Date.now();
      await handle.setInputFiles(payloads(), { timeout: 2000 });
      const afterMultiple = Date.now();
      const multiple = await observedFiles(page, 'node191-matrix', beforeMultiple, afterMultiple);
      await page.evaluate(() => {
        globalThis.__node191MatrixEvents = [];
        globalThis.__node191MatrixErrors = [];
      });
      const beforeEmpty = Date.now();
      await handle.setInputFiles([], { timeout: 2000 });
      const afterEmpty = Date.now();
      const empty = await observedFiles(page, 'node191-matrix', beforeEmpty, afterEmpty);
      await handle.dispose();
      return { multiple, empty };
    });

    // Act: run the identical payload sequence through the terminal Rust owner.
    const actual = await withPage(async fixture => {
      const { browser, session, page, produce, prepare, active, run } = fixture;
      await run(() => installObservedInput(page, { multiple: true }));
      const raw = await produce('document.getElementById("node191-matrix")');
      const baseline = await warmMutationMarker(fixture, raw);
      const beforeMultiple = Date.now();
      await diagnosticDeadline(prepareMutatedTarget(session, prepare(raw, page, {
        timeout: 0, payloads: payloads(),
      })), 'multiple serialized payload mutation');
      const afterMultiple = Date.now();
      const multiple = await run(() => observedFiles(page, 'node191-matrix', beforeMultiple, afterMultiple));
      assert.deepEqual(await exactOwnership(browser, 'multiple mutation reclamation', baseline, active), baseline);
      await run(() => page.evaluate(() => {
        globalThis.__node191MatrixEvents = [];
        globalThis.__node191MatrixErrors = [];
      }));
      const beforeEmpty = Date.now();
      await diagnosticDeadline(prepareMutatedTarget(session, prepare(raw, page, {
        timeout: 0, payloads: [],
      })), 'empty serialized payload mutation');
      const afterEmpty = Date.now();
      const empty = await run(() => observedFiles(page, 'node191-matrix', beforeEmpty, afterEmpty));
      assert.deepEqual(await exactOwnership(browser, 'empty mutation reclamation', baseline, active), baseline);
      return { multiple, empty };
    });

    // Assert: bytes, names, MIME, call-time modified values and event flags all match.
    assert.deepEqual(actual, expected);
  });

test('live mutation differential: a throwing input listener is reported but does not suppress change',
  { timeout: CASE_MS, skip: !targetMutation }, async () => {
    // Arrange
    const payload = () => ({ name: 'listener.txt', mimeType: 'text/plain', buffer: Buffer.from('listener') });
    const expected = await withOraclePage(async page => {
      await installObservedInput(page, { throwOnInput: true });
      const handle = await page.$('#node191-matrix');
      const before = Date.now();
      await handle.setInputFiles(payload(), { timeout: 2000 });
      const after = Date.now();
      const result = await observedFiles(page, 'node191-matrix', before, after);
      await handle.dispose();
      return result;
    });

    // Act
    const actual = await withPage(async fixture => {
      const { browser, session, page, produce, prepare, active, run } = fixture;
      await run(() => installObservedInput(page, { throwOnInput: true }));
      const raw = await produce('document.getElementById("node191-matrix")');
      const baseline = await warmMutationMarker(fixture, raw);
      const before = Date.now();
      await diagnosticDeadline(prepareMutatedTarget(session, prepare(raw, page, {
        timeout: 0, payloads: [payload()],
      })), 'throwing-listener serialized payload mutation');
      const after = Date.now();
      const result = await run(() => observedFiles(page, 'node191-matrix', before, after));
      assert.deepEqual(await exactOwnership(browser, 'throwing-listener mutation reclamation', baseline, active), baseline);
      return result;
    });

    // Assert
    assert.deepEqual(actual, expected);
    assert.deepEqual(actual.errors, ['Uncaught Error: node191-listener-sentinel']);
    assert.deepEqual(actual.events.map(event => event.type), ['input', 'change']);
  });

test('live mutation differential: a type change after prevalidation is the same successful no-op',
  { timeout: CASE_MS, skip: !targetMutation }, async () => {
    // Arrange: Playwright's injected helper returns text for a non-file input;
    // ElementHandle._setInputFiles ignores that return and resolves normally.
    const payload = () => ({ name: 'ignored.txt', mimeType: 'text/plain', buffer: Buffer.from('ignored') });
    const expected = await withOraclePage(async page => {
      await installObservedInput(page);
      const handle = await page.$('#node191-matrix');
      await handle.evaluate(input => { input.type = 'text'; });
      const before = Date.now();
      await handle.setInputFiles(payload(), { timeout: 2000 });
      const after = Date.now();
      const result = await observedFiles(page, 'node191-matrix', before, after);
      await handle.dispose();
      return result;
    });

    // Act: candidate mutation pauses after its exact original target has been
    // marked, prevalidated and instrumented, then observes the changed type in
    // the final non-retargeting utility-world call.
    const actual = await withPage(async fixture => {
      const { browser, session, page, produce, prepare, active, run } = fixture;
      await run(() => installObservedInput(page));
      const raw = await produce('document.getElementById("node191-matrix")');
      const baseline = await warmMutationMarker(fixture, raw);
      const preparation = prepare(raw, page, { timeout: 0, payloads: [payload()] });
      const entered = deferred();
      const release = deferred();
      const pending = prepareMutatedTarget(session, preparation, {
        onBeforeInputAction() { entered.resolve(); return release.promise; },
      });
      await diagnosticDeadline(entered.promise, 'late type-change instrumentation entry');
      let result;
      try {
        await run(() => page.evaluate(() => { document.getElementById('node191-matrix').type = 'text'; }));
        const before = Date.now();
        release.resolve();
        await diagnosticDeadline(pending, 'late type-change mutation no-op');
        const after = Date.now();
        result = await run(() => observedFiles(page, 'node191-matrix', before, after));
      } finally {
        release.resolve();
      }
      assert.equal(preparation.isPending(), false);
      assert.equal(preparation.abort(), false);
      assert.deepEqual(await exactOwnership(browser, 'late type-change reclamation', baseline, active), baseline);
      return result;
    });

    // Assert
    assert.deepEqual(actual, expected);
    assert.deepEqual(actual, { files: null, events: [], errors: [] });
  });

test('live mutation differential: timeout at the awaited instrumentation boundary cancels before mutation',
  { timeout: CASE_MS, skip: !targetMutation }, async () => {
    // Arrange / Act: hold both implementations at their pinned
    // onBeforeInputAction checkpoint until the single public deadline wins.
    const expected = await withOraclePage(async page => {
      await installObservedInput(page);
      const handle = await page.$('#node191-matrix');
      const entered = deferred();
      const release = deferred();
      const listener = { onBeforeInputAction() { entered.resolve(); return release.promise; } };
      const remove = installOracleInputInstrumentation(page, listener);
      let error;
      try {
        const pending = handle.setInputFiles({
          name: 'timeout.txt', mimeType: 'text/plain', buffer: Buffer.from('timeout'),
        }, { timeout: 25 });
        await diagnosticDeadline(entered.promise, 'oracle cancellation instrumentation entry');
        error = await nativeRejection(pending, 'oracle instrumentation cancellation');
      } finally {
        release.resolve();
        remove();
      }
      const state = await observedFiles(page, 'node191-matrix', 0, 0);
      await handle.dispose();
      return { error, state };
    });

    const actual = await withPage(async fixture => {
      const { browser, session, page, produce, prepare, active, run } = fixture;
      await run(() => installObservedInput(page));
      const raw = await produce('document.getElementById("node191-matrix")');
      const baseline = await warmMutationMarker(fixture, raw);
      const preparation = prepare(raw, page, {
        timeout: 25,
        payloads: [{ name: 'timeout.txt', mimeType: 'text/plain', buffer: Buffer.from('timeout') }],
      });
      const entered = deferred();
      const release = deferred();
      let error;
      try {
        const pending = prepareMutatedTarget(session, preparation, {
          onBeforeInputAction() { entered.resolve(); return release.promise; },
        });
        await diagnosticDeadline(entered.promise, 'candidate cancellation instrumentation entry');
        error = await nativeRejection(pending, 'candidate instrumentation cancellation');
      } finally {
        release.resolve();
      }
      const state = await run(() => observedFiles(page, 'node191-matrix', 0, 0));
      assert.deepEqual(await exactOwnership(browser, 'instrumentation cancellation reclamation', baseline, active), baseline);
      return { error, state };
    });

    // Assert: public error wrappers differ because NODE-191 intentionally does
    // not expose ElementHandle.setInputFiles yet; the deadline and side effect
    // semantics are identical.
    assert.equal(expected.error.name, 'TimeoutError');
    assert.match(expected.error.message, /Timeout 25ms exceeded/);
    assert.equal(actual.error.code, 'PWRIGHT_TIMEOUT');
    assert.match(actual.error.message, /Timeout 25ms exceeded/);
    assert.deepEqual(actual.state, expected.state);
    assert.deepEqual(actual.state, { files: [], events: [], errors: [] });
  });

test('live native serialized mutation: navigation after prevalidation rejects and reclaims ownership',
  { timeout: CASE_MS, skip: !targetMutation }, async () => {
    // Arrange / Act: pause pinned Playwright at the same awaited boundary and
    // commit a replacement main document before either mutation may resume.
    const expected = await withOraclePage(async page => {
      await installObservedInput(page);
      const handle = await page.$('#node191-matrix');
      const entered = deferred();
      const release = deferred();
      const listener = { onBeforeInputAction() { entered.resolve(); return release.promise; } };
      const remove = installOracleInputInstrumentation(page, listener);
      let outcome;
      try {
        const pending = handle.setInputFiles({
          name: 'navigation.txt', mimeType: 'text/plain', buffer: Buffer.from('navigation'),
        }, { timeout: 2000 });
        await diagnosticDeadline(entered.promise, 'oracle navigation instrumentation entry');
        await page.goto('data:text/html,<title>node191-oracle-after-navigation</title>');
        release.resolve();
        outcome = await diagnosticDeadline(pending.then(
          value => ({ status: 'resolved', value }),
          error => ({ status: 'rejected', error }),
        ), 'oracle navigation mutation settlement');
      } finally {
        release.resolve();
        remove();
        await handle.dispose();
      }
      return outcome;
    });

    const actual = await withPage(async fixture => {
      // Arrange: stop after the original element has been marked and
      // prevalidated in its utility world.
      const { browser, session, page, produce, prepare, active, run } = fixture;
      await run(() => installObservedInput(page));
      const raw = await produce('document.getElementById("node191-matrix")');
      const baseline = await warmMutationMarker(fixture, raw);
      const preparation = prepare(raw, page, {
        timeout: 0,
        payloads: [{ name: 'navigation.txt', buffer: Buffer.from('navigation') }],
      });
      const entered = deferred();
      const release = deferred();
      const pending = prepareMutatedTarget(session, preparation, {
        onBeforeInputAction() { entered.resolve(); return release.promise; },
      });
      await diagnosticDeadline(entered.promise, 'navigation race instrumentation entry');
      let error;
      try {
        await run(() => page.goto('data:text/html,<title>node191-after-navigation</title>', {
          waitUntil: 'load', timeout: 5000,
        }));
        assert.equal(await run(() => page.title()), 'node191-after-navigation');
        release.resolve();
        error = await nativeRejection(pending, 'navigation race mutation rejection');
      } finally {
        release.resolve();
      }
      assert.equal(preparation.isPending(), false);
      const idle = await ownership(browser, 'navigation race reclamation', active);
      assert.deepEqual(idle.preparation, { pending: 0, rootContainers: 0, rootedValues: 0 });
      assert.equal(idle.activeOperations, 0);
      assert.equal(idle.ffiReservedBytes, baseline.ffiReservedBytes);
      return error;
    });

    // Assert: preserve the observed discrepancy. Pinned Playwright ignores the
    // lost old-document target and resolves; NODE-191 fails closed while still
    // reclaiming every native owner. A future public compatibility layer must
    // choose this policy explicitly instead of hiding the difference.
    assert.deepEqual(expected, { status: 'resolved', value: undefined });
    assert.match(actual.message, /context|navigation|destroyed|closed/i);
  });

test('live native serialized mutation: frame detach after prevalidation rejects and reclaims ownership',
  { timeout: CASE_MS, skip: !targetMutation }, async () => {
    // Arrange / Act: pin the upstream server at the same input-action
    // instrumentation boundary, then detach the target's owner Frame.
    const expected = await withOraclePage(async page => {
      await page.evaluate(async () => {
        const iframe = document.createElement('iframe');
        iframe.name = 'node191-child';
        const loaded = new Promise(resolve => iframe.addEventListener('load', resolve, { once: true }));
        iframe.srcdoc = '<body></body>';
        document.body.append(iframe);
        await loaded;
      });
      const frame = page.frames().find(candidate => candidate !== page.mainFrame() && candidate.name() === 'node191-child');
      assert.ok(frame);
      await installObservedInput(frame);
      const handle = await frame.$('#node191-matrix');
      const entered = deferred();
      const release = deferred();
      const listener = { onBeforeInputAction() { entered.resolve(); return release.promise; } };
      const remove = installOracleInputInstrumentation(page, listener);
      let error;
      try {
        const pending = handle.setInputFiles({
          name: 'detached.txt', mimeType: 'text/plain', buffer: Buffer.from('detached'),
        }, { timeout: 2000 });
        await diagnosticDeadline(entered.promise, 'oracle frame-detach instrumentation entry');
        await page.evaluate(() => document.querySelector('iframe[name="node191-child"]').remove());
        await diagnosticDeadline((async () => {
          while (!frame.isDetached()) await setImmediate();
        })(), 'oracle frame-detach lifecycle publication');
        release.resolve();
        error = await nativeRejection(pending, 'oracle frame-detach mutation rejection');
      } finally {
        release.resolve();
        remove();
        await handle.dispose();
      }
      return error;
    });

    const actual = await withPage(async fixture => {
      // Arrange
      const { browser, session, page, keep, preparations, active, run } = fixture;
      await run(() => page.evaluate(async () => {
        const iframe = document.createElement('iframe');
        iframe.name = 'node191-child';
        const loaded = new Promise(resolve => iframe.addEventListener('load', resolve, { once: true }));
        iframe.srcdoc = '<body></body>';
        document.body.append(iframe);
        await loaded;
      }));
      const frame = run(() => page.frames().find(candidate => candidate !== page.mainFrame() && candidate.name() === 'node191-child'));
      assert.ok(frame);
      await run(() => installObservedInput(frame));
      const raw = await keep(session.frameEvaluateHandleWithPageTimeout(
        frame._id, 'document.getElementById("node191-matrix")', false, undefined, page._id));
      const createPreparation = options => run(() => {
        const preparation = session.handlePrepareFileInput(raw, { ownerFrame: () => frame }, [], options);
        preparations.push(preparation);
        return preparation;
      });
      const baseline = await warmMutationMarker(fixture, raw, createPreparation);
      const preparation = createPreparation({
        timeout: 0,
        payloads: [{ name: 'detached.txt', buffer: Buffer.from('detached') }],
      });
      const entered = deferred();
      const release = deferred();
      const pending = prepareMutatedTarget(session, preparation, {
        onBeforeInputAction() { entered.resolve(); return release.promise; },
      });
      await diagnosticDeadline(entered.promise, 'frame-detach race instrumentation entry');
      let error;
      try {
        await run(() => page.evaluate(() => document.querySelector('iframe[name="node191-child"]').remove()));
        await diagnosticDeadline((async () => {
          while (!frame.isDetached()) await setImmediate();
        })(), 'frame-detach lifecycle publication');
        release.resolve();
        error = await nativeRejection(pending, 'frame-detach race mutation rejection');
      } finally {
        release.resolve();
      }
      assert.equal(preparation.isPending(), false);
      const idle = await ownership(browser, 'frame-detach race reclamation', active);
      assert.deepEqual(idle.preparation, { pending: 0, rootContainers: 0, rootedValues: 0 });
      assert.equal(idle.activeOperations, 0);
      assert.equal(idle.ffiReservedBytes, baseline.ffiReservedBytes);
      return error;
    });

    // Assert
    assert.match(expected.message, /context|detached|destroyed|closed/i);
    assert.match(actual.message, /context|detached|destroyed|closed/i);
  });

test('live native serialized mutation: abort after native start rejects the waiter and settles cleanup',
  { timeout: CASE_MS, skip: !targetMutation }, async () => withPage(async fixture => {
    // Arrange
    const { browser, session, page, produce, prepare, active, run } = fixture;
    await run(() => installObservedInput(page));
    const raw = await produce('document.getElementById("node191-matrix")');
    const baseline = await warmMutationMarker(fixture, raw);
    const preparation = prepare(raw, page, {
      timeout: 0,
      payloads: [{ name: 'cancelled.bin', mimeType: 'application/x-node191', buffer: Buffer.from([0, 255, 1]) }],
    });
    await diagnosticDeadline(prepareImportedTarget(session, preparation), 'mutation cancellation import');

    // Act: the N-API promise has started the Rust mutation before synchronous
    // revocation removes the public preparation ticket.
    const pending = session.handleMutateSerializedFileInputPayloads(preparation);
    assert.equal(preparation.abort(), true);
    const error = await nativeRejection(pending, 'started mutation cancellation');

    // Assert: started native work owns its cleanup independently of the waiter.
    assert.match(error.message, /closed/i);
    assert.equal(preparation.isPending(), false);
    const result = await run(() => page.evaluate(async () => {
      const input = document.getElementById('node191-matrix');
      const file = input.files[0];
      return file ? {
        name: file.name, type: file.type,
        bytes: [...new Uint8Array(await file.arrayBuffer())],
      } : null;
    }));
    assert.equal(result, null, 'revocation before the native poll must prevent the browser side effect');
    assert.deepEqual(await exactOwnership(browser, 'started mutation cancellation reclamation', baseline, active), baseline);
  }));

test('live native serialized mutation: maximum admitted payload stays bounded and fully reclaimable',
  { timeout: MAX_MEMORY_CASE_MS,
    skip: !targetMutation || process.env.PWRIGHT_MAX_PAYLOAD_GATE !== '1' }, async () => {
    // Arrange
    await collectGarbage();
    const before = process.memoryUsage();
    let admitted;

    // Act
    await withPage(async fixture => {
      const { browser, session, page, produce, prepare, active, run } = fixture;
      await run(() => installObservedInput(page));
      const raw = await produce('document.getElementById("node191-matrix")');
      const baseline = await warmMutationMarker(fixture, raw);
      const bytes = Buffer.alloc(MAX_SOURCE_BYTES, 0xa5);
      const beforeMutation = Date.now();
      await prepareMutatedTarget(session, prepare(raw, page, {
        timeout: 0,
        payloads: [{ name: 'maximum.bin', mimeType: 'application/octet-stream', buffer: bytes }],
      }));
      const afterMutation = Date.now();
      admitted = process.memoryUsage();
      const result = await run(() => page.evaluate(async () => {
        const file = document.getElementById('node191-matrix').files[0];
        const first = new Uint8Array(await file.slice(0, 1).arrayBuffer())[0];
        const last = new Uint8Array(await file.slice(-1).arrayBuffer())[0];
        return { name: file.name, type: file.type, size: file.size, lastModified: file.lastModified, first, last };
      }));
      assert.deepEqual({ ...result, lastModified: 'call-time' }, {
        name: 'maximum.bin', type: 'application/octet-stream', size: MAX_SOURCE_BYTES,
        lastModified: 'call-time', first: 0xa5, last: 0xa5,
      });
      assert.ok(result.lastModified >= beforeMutation && result.lastModified <= afterMutation);
      assert.deepEqual(await exactOwnership(browser, 'maximum payload mutation reclamation', baseline, active), baseline);
    }, 90000);
    await collectGarbage();
    const after = process.memoryUsage();

    // Assert: this is a process gate, not an allocator identity claim. It
    // bounds the transient Node resident increase and proves the 50 MiB Buffer
    // is no longer retained after ticket/Context/connection teardown.
    const rssGrowth = admitted.rss - before.rss;
    const retainedArrayBuffers = after.arrayBuffers - before.arrayBuffers;
    assert.ok(rssGrowth < 768 * 1024 * 1024, `maximum payload grew Node RSS by ${rssGrowth} bytes`);
    assert.ok(retainedArrayBuffers < 4 * 1024 * 1024,
      `maximum payload retained ${retainedArrayBuffers} ArrayBuffer bytes`);
    process.stdout.write(`${JSON.stringify({ maximumPayloadMemory: {
      profile: compatibility.nativeProfile, sourceBytes: MAX_SOURCE_BYTES,
      rssBefore: before.rss, rssAdmitted: admitted.rss, rssAfterGc: after.rss,
      arrayBuffersBefore: before.arrayBuffers, arrayBuffersAfterGc: after.arrayBuffers,
    } })}\n`);
  });

test('live native serialized import: the selected budget admits only one retained graph',
  { timeout: CASE_MS, skip: !targetImport }, async () => withPage(async fixture => {
    // Arrange
    const { session, page, produce, prepare } = fixture;
    const firstRaw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const secondRaw = await produce('Object.assign(document.createElement("input"), { type: "file" })');
    const first = prepare(firstRaw, page, { timeout: 0,
      payloads: [{ name: 'first.txt', buffer: Buffer.from('first') }] });
    const second = prepare(secondRaw, page, { timeout: 0,
      payloads: [{ name: 'second.txt', buffer: Buffer.from('second') }] });
    await diagnosticDeadline(prepareSerializedTarget(session, first), 'first serialized import owner');
    await diagnosticDeadline(prepareSerializedTarget(session, second), 'second serialized import owner');

    // Act
    session.handleImportSerializedFileInputPayloads(first);
    assert.throws(() => session.handleImportSerializedFileInputPayloads(second),
      /serialized file-input payload import capacity exceeded/i);

    // Assert
    assert.equal(first.isPending(), true);
    assert.equal(second.isPending(), true);
    assert.equal(first.abort(), true);
    assert.equal(second.abort(), true);
  }));

test('live native marking: Node deadline revokes the actual ticket during outer materialization await',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange: install the inherited then only from inside serialization, after
    // the public timer has started. Earlier owner/client conversion therefore
    // remains unaffected while the outer materialized carrier never settles.
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce();
    const baseline = await ownership(browser, 'before materialization deadline', active);
    const saved = Object.getOwnPropertyDescriptor(Object.prototype, 'then');
    const trace = [];
    const bytes = Buffer.from([1]);
    let restored = false;
    const restore = () => {
      if (restored) return;
      restored = true;
      if (saved) Object.defineProperty(Object.prototype, 'then', saved);
      else delete Object.prototype.then;
    };
    bytes.toString = () => {
      trace.push('serialize');
      Object.defineProperty(Object.prototype, 'then', {
        configurable: true,
        value() { trace.push('carrier-then'); },
      });
      return 'AQ==';
    };
    const preparation = prepare(raw, page, {
      payloads: [{ name: 'deadline.bin', buffer: bytes }],
      timeout: 10,
    });

    // Act
    let error;
    try {
      error = await diagnosticDeadline(prepareMarkedTarget(session, preparation).then(
        () => assert.fail('materialization deadline must reject'),
        failure => { restore(); return failure; },
      ), 'actual materialization deadline');
    } finally {
      restore();
    }

    // Assert
    active();
    assert.equal(error.code, 'PWRIGHT_TIMEOUT');
    assert.equal(error.message, 'PWRIGHT_TIMEOUT: Timeout 10ms exceeded');
    assert.deepEqual(trace, ['serialize', 'carrier-then']);
    assert.equal(preparation.isPending(), false);
    assert.deepEqual(await ownership(browser, 'materialization deadline reclamation', active), baseline);
  }));

test('live native marking: synchronous abort revokes running or queued work and its roots',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange
    const { browser, session, page, produce, prepare, active } = fixture;
    const raw = await produce();
    const baseline = await ownership(browser, 'before target-mark cancellation', active);
    const preparation = prepare(raw, page, {
      payloads: [{ name: 'cancel.txt', buffer: Buffer.from('cancel') }],
    });
    await diagnosticDeadline(prepareServerPayloads(session, preparation),
      'actual ticket scheduled to Materialized before cancellation');

    // Act: the completion callback cannot run on this Node turn before abort.
    const pending = session.handleMarkFileInputTarget(preparation);
    assert.equal(preparation.abort(), true);
    const error = await nativeRejection(pending, 'cancelled native target marking');

    // Assert
    active();
    assert.match(error.message, /closed/i);
    assert.equal(preparation.isPending(), false);
    const idle = await ownership(browser, 'cancelled marking reclamation', active);
    assert.equal(idle.ffiPending, baseline.ffiPending);
    assert.equal(idle.ffiQueued, baseline.ffiQueued);
    assert.equal(idle.ffiRunning, baseline.ffiRunning);
    assert.equal(idle.ffiReservedBytes, baseline.ffiReservedBytes);
    assert.deepEqual(idle.preparation, baseline.preparation);
  }));

test('live native marking: disposal after Materialized rejects the bound original and aborts the ticket',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange
    const { browser, session, page, produce, prepare, active, run } = fixture;
    const raw = await produce();
    const preparation = prepare(raw, page, {
      payloads: [{ name: 'disposed.txt', buffer: Buffer.from('disposed') }],
    });
    await diagnosticDeadline(prepareServerPayloads(session, preparation),
      'actual ticket scheduled to Materialized before disposal');
    await run(() => session.handleDispose(raw));

    // Act
    const error = await nativeRejection(
      session.handleMarkFileInputTarget(preparation),
      'disposed original target marking');

    // Assert
    active();
    assert.match(error.message, /JSHandle is disposed/);
    assert.equal(preparation.isPending(), false);
    const idle = await ownership(browser, 'disposed marking reclamation', active);
    assert.deepEqual(idle.preparation, { pending: 0, rootContainers: 0, rootedValues: 0 });
  }));

test('live native marking: a foreign session cannot cancel the owner ticket',
  { timeout: CASE_MS, skip: !targetMarking }, async () => withPage(async fixture => {
    // Arrange
    const { session, page, connect, produce, prepare, active } = fixture;
    const raw = await produce();
    const preparation = prepare(raw, page, {
      payloads: [{ name: 'owner.txt', buffer: Buffer.from('owner') }],
    });
    await diagnosticDeadline(prepareServerPayloads(session, preparation),
      'owner ticket scheduled to Materialized');
    const sibling = await connect();

    // Act / Assert: provenance rejection occurs before native phase checkout.
    assert.throws(() => sibling._session.handleMarkFileInputTarget(preparation), /another session/);
    assert.equal(preparation.isPending(), true);
    const result = await diagnosticDeadline(
      session.handleMarkFileInputTarget(preparation),
      'owner target marking after foreign rejection');
    active();
    assert.equal(result, undefined);
    assert.equal(preparation.isPending(), true);
    assert.equal(preparation.abort(), true);
  }));

test('live dispatch: driver serializer failure preserves the exception and aborts the actual ticket', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, session, page, produce, prepare, active } = fixture;
  const raw = await produce();
  const baseline = await ownership(browser, 'before scheduled serializer failure', active);
  const original = new Error('scheduled serializer failure');
  const bytes = Buffer.from([1]);
  bytes.toString = () => { throw original; };
  const preparation = prepare(raw, page, { payloads: [{ name: 'one.bin', buffer: bytes }] });

  // Act
  const error = await nativeRejection(prepareServerPayloads(session, preparation),
    'scheduled actual-ticket serializer rejection');

  // Assert: no server error transport/API prefix is claimed by this driver.
  active();
  assert.equal(error, original);
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'scheduled serializer failure reclamation', active), baseline);
}));

test('live dispatch: disposed source still runs client conversion but rejects before serialization', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange
  const { browser, session, page, produce, preparations, active, run } = fixture;
  const raw = await produce();
  await run(() => session.handleDispose(raw));
  const baseline = await ownership(browser, 'before disposed-source scheduled conversion', active);
  const trace = [];
  const bytes = Buffer.from([1]);
  bytes.toString = () => { trace.push('serialize'); return 'AQ=='; };
  const files = [{ get name() { trace.push('client-name'); return 'one.bin'; }, buffer: bytes }];
  files.slice = function () { trace.push('slice'); return Array.prototype.slice.call(this); };
  const receiver = { ownerFrame() { trace.push('owner'); return page.mainFrame(); } };
  const preparation = run(() => session.handlePrepareFileInput(raw, receiver, files, {}));
  preparations.push(preparation);

  // Act
  await nativeRejection(prepareServerPayloads(session, preparation),
    'disposed-source scheduled dispatch rejection', /closed/i);

  // Assert
  active();
  assert.deepEqual(trace, ['owner', 'slice', 'client-name']);
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'disposed dispatch root reclamation', active), baseline);
}));

test('live dispatch: a sibling-session gate cannot abort the legitimate owner ticket', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: test the raw gate, not driver catch/abort error handling.
  const { browser, session, page, produce, prepare, connect, active, run } = fixture;
  const raw = await produce();
  const sibling = await connect();
  const baseline = await ownership(browser, 'before foreign dispatch gate', active);
  const preparation = prepare(raw, page);
  await diagnosticDeadline(prepareChannel(preparation), 'channel before foreign gate');
  active();
  const before = preparationMetrics(session);

  // Act
  assert.throws(() => run(() => sibling._session.handleCheckFileInputDispatch(preparation)), /another session/i);

  // Assert: the rightful owner can still advance the very same ticket.
  assert.equal(preparation.isPending(), true);
  assert.deepEqual(preparationMetrics(session), before);
  session.handleCheckFileInputDispatch(preparation);
  preparation.validateServerParams(serverValidationPorts);
  enterServerMaterialization(session, preparation);
  const carrier = await diagnosticDeadline(asyncMaterialize(preparation), 'rightful ticket after foreign gate');
  active();
  preparation.finishMaterialization(carrier);
  assert.deepEqual(carrier.filePayloads, []);
  assert.equal(preparation.abort(), true);
  assert.deepEqual(await ownership(browser, 'foreign dispatch gate reclamation', active), baseline);
}));

test('live dispatch: server-schema microtask disposal wins before the second gate and serialization', { timeout: CASE_MS }, async () => withPage(async fixture => {
  // Arrange: custom client map returns a real array unchecked, so its accessor
  // is first read by the second schema, not a synthetic between-phase hook.
  const { browser, session, page, produce, prepare, active, run } = fixture;
  const baseline = await ownership(browser, 'before schema-retired source creation', active);
  const raw = await produce();
  const trace = [];
  let retirement;
  const bytes = Buffer.from([1]);
  bytes.toString = () => { trace.push('serialize'); return 'AQ=='; };
  const payloads = [];
  payloads.map = () => {
    trace.push('client-map');
    return [{
      get name() {
        trace.push('server-name');
        queueMicrotask(() => {
          trace.push('dispose');
          // Observe every retirement rejection immediately, even if the
          // driver fails before the test reaches the explicit cleanup await.
          try {
            retirement = run(() => session.handleDispose(raw)).then(
              () => ({ released: true }), error => ({ error }),
            );
          } catch (error) { retirement = Promise.resolve({ error }); }
        });
        return 'one.bin';
      },
      buffer: bytes,
    }];
  };
  const preparation = prepare(raw, page, { payloads });

  // Act
  await nativeRejection(prepareServerPayloads(session, preparation),
    'second dispatch gate after queued source disposal', /closed/i);

  // Assert
  active();
  assert.deepEqual(trace, ['client-map', 'server-name', 'dispose']);
  assert.ok(retirement, 'server schema must schedule actual source retirement');
  assert.deepEqual(await diagnosticDeadline(retirement, 'schema-triggered source retirement'), { released: true });
  active();
  assert.equal(preparation.isPending(), false);
  assert.deepEqual(await ownership(browser, 'schema-retired ticket reclamation', active), baseline);
}));
