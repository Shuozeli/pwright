// Test-only sampling contracts, not a browser/FFI implementation. Active
// baselines retain armed watches, never queued watch results awaiting delivery.
export function isActiveOwnershipIdle(host, ffi, preparation) {
  return host.pendingCalls === 0 && host.activeOperations === 0 && host.cleanupInFlight === 0
    && ffi.commands === 0 && ffi.connects === 0 && ffi.closes === 0
    && ffi.queued === 0 && ffi.queuedCommands === 0
    && ffi.pending === ffi.watches && ffi.running === ffi.watches
    && preparation.pending === 0 && preparation.rootContainers === 0 && preparation.rootedValues === 0;
}

// One preparation
// waiting for its owner holds a Command admission but has not started a future.
// Compare watch work with an already stable baseline instead of assuming no
// long-lived watches exist. Normal zero-command cleanup does not use this gate.
export function isIdleWithOwnerLookupPreparation(host, ffi, preparation, baseline) {
  return host.pendingCalls === 0 && host.activeOperations === 0 && host.cleanupInFlight === 0
    && ffi.commands === 1 && ffi.connects === 0 && ffi.closes === 0 && ffi.queuedCommands === 0
    && ffi.queued === 0 && ffi.pending === ffi.watches + 1 && ffi.running === ffi.watches
    && ffi.pending === baseline.ffiPending + 1
    && ffi.queued === baseline.ffiQueued && ffi.running === baseline.ffiRunning
    && ffi.watches === baseline.ffiWatches
    && preparation.pending === 1 && preparation.rootContainers === 1 && preparation.rootedValues === 3;
}

// Final retirement is stricter than active-session idleness: a watch can still
// own an FFI admission while its close notification is awaiting JS delivery.
// Sampling that stable-but-nonzero state twice is not reclamation evidence.
export function isFullyRetiredOwnership(host, ffi, preparation) {
  return host.connected === false && host.liveObjects === 0
    && host.activeLeases === 0 && host.liveScopes === 0
    && host.pendingCalls === 0 && host.activeOperations === 0 && host.cleanupInFlight === 0
    && ffi.pending === 0 && ffi.queued === 0 && ffi.queuedCommands === 0 && ffi.running === 0
    && ffi.reservedBytes === 0 && ffi.commands === 0 && ffi.connects === 0
    && ffi.closes === 0 && ffi.watches === 0
    && preparation.pending === 0 && preparation.rootContainers === 0 && preparation.rootedValues === 0;
}

function boundedText(value, bytes) {
  const text = typeof value === 'string' ? value : String(value);
  let end = Math.min(text.length, bytes);
  while (Buffer.byteLength(text.slice(0, end)) > bytes) end--;
  return text.slice(0, end);
}

function errorTree(error) {
  const seen = new Set();
  const nodes = [];
  const visit = (value, depth) => {
    const node = {};
    nodes.push(node);
    if (value !== null && (typeof value === 'object' || typeof value === 'function')) {
      if (seen.has(value)) return Object.assign(node, { truncated: 'cycle' });
      seen.add(value);
    }
    if (value instanceof Error) {
      node.name = boundedText(value.name, 64);
      node.message = boundedText(value.message, 512);
      if (value.code !== undefined) node.code = boundedText(value.code, 64);
      if (value.stack) node.stack = boundedText(value.stack, 768);
      if (depth === 3) {
        if (value.cause !== undefined || value instanceof AggregateError) node.truncated = 'error depth';
      } else {
        if (value instanceof AggregateError && Array.isArray(value.errors)) {
          node.errors = [];
          for (const child of value.errors) {
            if (nodes.length === 8) { node.omittedErrors = value.errors.length - node.errors.length; break; }
            node.errors.push(visit(child, depth + 1));
          }
        }
        if (value.cause !== undefined) {
          if (nodes.length < 8) node.cause = visit(value.cause, depth + 1);
          else node.omittedCause = true;
        }
      }
    } else node.thrown = boundedText(value, 512);
    return node;
  };
  try {
    const tree = visit(error, 0);
    for (const bytes of [512, 256, 128, 64, 32, 16, 8]) {
      if (Buffer.byteLength(JSON.stringify(tree)) <= 6144) break;
      for (const node of nodes)
        for (const key of ['name', 'message', 'code', 'stack', 'thrown'])
          if (node[key] !== undefined) node[key] = boundedText(node[key], bytes);
    }
    return tree;
  } catch {
    return { diagnosticFailure: 'error property inspection failed' };
  }
}

// Diagnostic wrappers label phases only; callers continue throwing the original
// error/cleanup AggregateError. At most eight tree nodes, depth three and 8 KiB
// of JSON are emitted even for cycles, giant messages or hostile error getters.
export function bindingFailureDiagnostic(primary, failures) {
  const labeled = [];
  if (primary !== undefined) labeled.push(new Error('exercise', { cause: primary }));
  for (const { phase, index, error } of failures.slice(0, 8))
    labeled.push(new Error(`${boundedText(phase, 128)} [${index ?? 'phase'}]`, { cause: error }));
  const record = {
    diagnostic: 'handle-binding-failure',
    error: errorTree(new AggregateError(labeled, 'retained binding failure details')),
    omittedCleanupFailures: Math.max(0, failures.length - 8),
  };
  const encoded = JSON.stringify(record);
  return Buffer.byteLength(encoded) <= 8191 ? encoded
    : '{"diagnostic":"handle-binding-failure","truncated":"diagnostic byte limit"}';
}
