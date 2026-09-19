// Pure observation-predicate regressions. Numeric samples are deliberately not
// a fake browser or proof that any real preparation/native cleanup succeeded.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { bindingFailureDiagnostic, isActiveOwnershipIdle, isFullyRetiredOwnership, isIdleWithOwnerLookupPreparation } from './handle-file-input-binding-observation.mjs';

function samples() {
  return {
    host: { pendingCalls: 0, activeOperations: 0, cleanupInFlight: 0 },
    ffi: { commands: 1, connects: 0, closes: 0, queuedCommands: 0, pending: 2, queued: 0, running: 1, watches: 1 },
    preparation: { pending: 1, rootContainers: 1, rootedValues: 3 },
    baseline: { ffiPending: 1, ffiQueued: 0, ffiRunning: 1, ffiWatches: 1 },
  };
}

test('active baseline rejects queued watch delivery and accepts the rearmed watch without removing counters', () => {
  // Arrange: same live admission/bytes, distinct publication states observed in
  // the live repeat. Two equal queued samples were not a settled baseline.
  const { host } = samples();
  const preparation = { pending: 0, rootContainers: 0, rootedValues: 0 };
  const queued = { commands: 0, connects: 0, closes: 0, queuedCommands: 0, pending: 1, queued: 1, running: 0, watches: 1 };
  const armed = { ...queued, queued: 0, running: 1 };
  // Act
  const queuedIdle = isActiveOwnershipIdle(host, queued, preparation);
  const armedIdle = isActiveOwnershipIdle(host, armed, preparation);
  // Assert
  assert.equal(queuedIdle, false);
  assert.equal(armedIdle, true);
});

test('active baseline rejects unaccounted FFI work and pending preparation despite a zero command sample', () => {
  // Arrange
  const { host, ffi: pendingFfi, preparation } = samples();
  const ffi = { ...pendingFfi, commands: 0, pending: 1 };
  const empty = { pending: 0, rootContainers: 0, rootedValues: 0 };
  const cases = [
    ['pending', 0], ['pending', 2], ['running', 0], ['running', 2],
    ['queued', 1], ['queuedCommands', 1], ['commands', 1], ['connects', 1], ['closes', 1],
  ];
  for (const [key, value] of cases) {
    // Act
    const idle = isActiveOwnershipIdle(host, { ...ffi, [key]: value }, empty);
    // Assert
    assert.equal(idle, false, `ffi.${key}=${value}`);
  }
  assert.equal(isActiveOwnershipIdle(host, ffi, preparation), false);
  for (const key of Object.keys(host))
    assert.equal(isActiveOwnershipIdle({ ...host, [key]: 1 }, ffi, empty), false, `host.${key}`);
});

test('owner-wait observation permits exactly one preparation command alongside unchanged watch work', () => {
  // Arrange
  const { host, ffi, preparation, baseline } = samples();
  // Act
  const idle = isIdleWithOwnerLookupPreparation(host, ffi, preparation, baseline);
  // Assert
  assert.equal(idle, true);
});

test('owner-wait observation rejects missing or additional command admissions and preparation roots', () => {
  // Arrange
  const cases = [
    ['ffi', 'commands', 0], ['ffi', 'commands', 2], ['ffi', 'pending', 1], ['ffi', 'pending', 3],
    ['preparation', 'pending', 0], ['preparation', 'pending', 2],
    ['preparation', 'rootContainers', 0], ['preparation', 'rootContainers', 2],
    ['preparation', 'rootedValues', 2], ['preparation', 'rootedValues', 4],
  ];
  for (const [group, key, value] of cases) {
    const sample = samples();
    sample[group][key] = value;
    // Act
    const idle = isIdleWithOwnerLookupPreparation(sample.host, sample.ffi, sample.preparation, sample.baseline);
    // Assert
    assert.equal(idle, false, `${group}.${key}=${value}`);
  }
});

test('owner-wait observation rejects native work, queued delivery and changed running/watch work', () => {
  // Arrange
  const cases = [
    ['host', 'pendingCalls', 1], ['host', 'activeOperations', 1], ['host', 'cleanupInFlight', 1],
    ['ffi', 'connects', 1], ['ffi', 'closes', 1], ['ffi', 'queuedCommands', 1], ['ffi', 'queued', 1],
    ['ffi', 'running', 0], ['ffi', 'running', 2], ['ffi', 'watches', 0], ['ffi', 'watches', 2],
  ];
  for (const [group, key, value] of cases) {
    const sample = samples();
    sample[group][key] = value;
    // Act
    const idle = isIdleWithOwnerLookupPreparation(sample.host, sample.ffi, sample.preparation, sample.baseline);
    // Assert
    assert.equal(idle, false, `${group}.${key}=${value}`);
  }
});

function retiredSamples() {
  return {
    host: { connected: false, liveObjects: 0, activeLeases: 0, liveScopes: 0, pendingCalls: 0, activeOperations: 0, cleanupInFlight: 0 },
    ffi: { commands: 0, connects: 0, closes: 0, queuedCommands: 0, pending: 0, queued: 0, running: 0, watches: 0, reservedBytes: 0 },
    preparation: { pending: 0, rootContainers: 0, rootedValues: 0 },
  };
}

test('final retirement observation rejects a stable pending watch until its FFI admission settles', () => {
  // Arrange: the previous command-only idle predicate accepted this sample.
  const { host, ffi, preparation } = retiredSamples();
  const pendingWatch = { ...ffi, pending: 1, running: 1, watches: 1, reservedBytes: 16384 };
  // Act
  const waiting = isFullyRetiredOwnership(host, pendingWatch, preparation);
  const retired = isFullyRetiredOwnership(host, ffi, preparation);
  // Assert
  assert.equal(waiting, false);
  assert.equal(retired, true);
});

test('final retirement observation requires every asserted native and FFI preparation resource to be zero', () => {
  // Arrange
  const sample = retiredSamples();
  const cases = Object.entries(sample).flatMap(([group, fields]) =>
    Object.keys(fields).map(key => [group, key]));
  for (const [group, key] of cases) {
    const value = retiredSamples();
    value[group][key] = key === 'connected' ? true : 1;
    // Act
    const retired = isFullyRetiredOwnership(value.host, value.ffi, value.preparation);
    // Assert
    assert.equal(retired, false, `${group}.${key} must be retired`);
  }
});

test('binding failure diagnostic labels primary and cleanup action without mutating errors', () => {
  // Arrange
  const primary = new Error('client getter');
  const cleanup = new Error('release failed');
  const failure = { phase: 'dispose raw Handles', index: 2, error: cleanup };
  // Act
  const value = JSON.parse(bindingFailureDiagnostic(primary, [failure]));
  // Assert
  assert.equal(value.error.errors[0].message, 'exercise');
  assert.equal(value.error.errors[0].cause.message, 'client getter');
  assert.equal(value.error.errors[1].message, 'dispose raw Handles [2]');
  assert.equal(value.error.errors[1].cause.message, 'release failed');
  assert.equal(failure.error, cleanup);
  assert.equal(primary.message, 'client getter');
  assert.equal(cleanup.message, 'release failed');
});

test('binding failure diagnostic bounds nested cyclic and large error output', () => {
  // Arrange
  const primary = new AggregateError([], '\u0000'.repeat(10000));
  primary.errors.push(primary, ...Array.from({ length: 30 }, () => new Error('巨'.repeat(10000))));
  const failures = Array.from({ length: 20 }, (_, index) => ({ phase: 'cleanup', index, error: primary }));
  // Act
  const encoded = bindingFailureDiagnostic(primary, failures);
  const value = JSON.parse(encoded);
  // Assert
  assert.ok(Buffer.byteLength(encoded) <= 8191);
  assert.equal(value.omittedCleanupFailures, 12);
  assert.match(encoded, /cycle|omittedErrors|error depth/);
});

test('binding failure diagnostic cannot replace a rejection when error inspection throws', () => {
  // Arrange
  const original = new Error('original');
  Object.defineProperty(original, 'message', { get() { throw new Error('diagnostic getter'); } });
  // Act
  const encoded = bindingFailureDiagnostic(original, []);
  // Assert
  assert.equal(JSON.parse(encoded).error.diagnosticFailure, 'error property inspection failed');
});
