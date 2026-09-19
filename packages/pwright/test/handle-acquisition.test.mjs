import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { WebSocket } from './cdp-command-relay.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const { chromium: candidate, compatibility } = await import('@shuozeli/pwright');
assert.equal(compatibility.nativeProfile, 'handles');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const phaseTrace = process.env.PWRIGHT_HANDLE_ACQUISITION_TRACE === '1';
let fixtureSequence = 0;

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
    if (nodes.length === 8) return { truncated: 'error count' };
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
    // Count serialized bytes too: control characters expand under JSON escaping.
    // Retain the tree shape while shrinking text; diagnostics never dump stacks
    // or an AggregateError's full, potentially unbounded child collection.
    for (const bytes of [512, 256, 128, 64, 32, 16, 8]) {
      if (Buffer.byteLength(JSON.stringify(tree)) <= 6144) break;
      for (const node of nodes) {
        for (const key of ['name', 'message', 'code', 'stack', 'thrown']) {
          if (node[key] !== undefined) node[key] = boundedText(node[key], bytes);
        }
      }
    }
    return tree;
  } catch (failure) {
    return { diagnosticFailure: boundedText(failure instanceof Error ? failure.message : typeof failure, 256) };
  }
}

function traced(label, phase, operation) {
  if (!phaseTrace) return operation();
  const start = performance.now();
  const emit = (event, error) => {
    const record = {
      diagnostic: 'handle-acquisition-phase', timestamp: new Date().toISOString(),
      label: boundedText(label, 256), phase: boundedText(phase, 256), event,
      elapsedMs: Math.round(performance.now() - start),
      ...(error === undefined ? {} : { error: errorTree(error) }),
    };
    let line = JSON.stringify(record);
    // The error tree is capped separately; this last bound includes metadata.
    if (Buffer.byteLength(line) > 8191) {
      record.label = boundedText(label, 64);
      record.phase = boundedText(phase, 64);
      line = JSON.stringify(record);
    }
    process.stderr.write(`${line}\n`);
  };
  emit('start');
  let pending;
  try { pending = operation(); }
  catch (error) { emit('error', error); throw error; }
  return Promise.resolve(pending).then(value => {
    emit('ok');
    return value;
  }, error => { emit('error', error); throw error; });
}

// An independent, bounded diagnostic connection cannot inherit the subject's
// pending-call/close deadlock. It never auto-attaches or touches unrelated Pages.
async function recoveryConnection(trace) {
  const discovery = await trace('Observer endpoint discovery', () => fetch(`${endpoint}/json/version`, { signal: AbortSignal.timeout(5000) }));
  assert.equal(discovery.status, 200);
  const url = new URL((await discovery.json()).webSocketDebuggerUrl);
  assert.equal(url.protocol, 'ws:');
  assert.equal(url.hostname, '127.0.0.1');
  assert.equal(url.port, '9222');
  const socket = new WebSocket(url.href, { maxPayload: 1024 * 1024 });
  const pending = new Map();
  let nextId = 0;
  const fail = error => {
    for (const waiter of pending.values()) waiter.reject(error);
    pending.clear();
  };
  socket.on('error', fail);
  socket.on('close', () => fail(new Error('acquisition recovery connection closed')));
  socket.on('message', raw => {
    try {
      const message = JSON.parse(raw.toString());
      const waiter = pending.get(message.id);
      if (!waiter) return;
      pending.delete(message.id);
      if (message.error) waiter.reject(new Error(`Recovery CDP: ${JSON.stringify(message.error).slice(0, 512)}`));
      else waiter.resolve(message.result);
    } catch (error) { fail(error); socket.terminate(); }
  });
  try { await trace('Observer WebSocket open', () => diagnosticDeadline(once(socket, 'open'), 'acquisition recovery connection')); }
  catch (error) { socket.terminate(); throw error; }
  return {
    async send(method, params = {}, sessionId) {
      assert.equal(socket.readyState, WebSocket.OPEN);
      assert.ok(pending.size < 4, 'bounded recovery command inventory');
      const id = ++nextId;
      const result = new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }), error => {
          if (error) { pending.delete(id); reject(error); }
        });
      });
      try { return await trace(`Observer ${method}`, () => diagnosticDeadline(result, `acquisition recovery ${method}`)); }
      finally { pending.delete(id); }
    },
    close() { socket.terminate(); },
  };
}

async function withFixture(type, exercise, signal, label) {
  const trace = (phase, operation) => traced(label, phase, operation);
  const recovery = await trace('Observer connection create', () => recoveryConnection(trace));
  const marker = `pwright-acquisition-${process.pid}-${++fixtureSequence}`;
  const url = `about:blank#${marker}`;
  const held = [];
  const watched = [];
  let browser, context, page, target, sessionId, cleanupPromise;
  let primary;
  const watch = (promise, isHandle = false) => {
    // Attach both handlers in the same turn as API invocation: close may reject
    // before its own promise resolves, and no unhandled-rejection grace is used.
    const state = { settled: false };
    state.promise = Promise.resolve(trace(`${isHandle ? 'Handle acquisition' : 'Owner close'} ${watched.length + 1} settlement`, () => promise)).then(value => {
      state.settled = true;
      if (isHandle) held.push(value);
      return { outcome: 'resolved', value };
    }, error => {
      state.settled = true;
      return { outcome: 'rejected', error };
    });
    watched.push(state);
    return state;
  };
  const keep = async promise => {
    const handle = await trace(`Retained Handle ${held.length + 1} acquisition`, () => promise);
    held.push(handle);
    return handle;
  };
  async function nativeCleanup(resolvePending) {
    if (!target) return;
    const targets = (await recovery.send('Target.getTargets')).targetInfos;
    const current = targets.find(value => value.targetId === target.targetId);
    const failures = [];
    if (resolvePending && current) {
      // A saved target ID is insufficient authority after navigation: require
      // the exact Context, URL and in-world fixture marker before resolution.
      assert.equal(current.browserContextId, target.browserContextId);
      assert.equal(current.url, url);
      try {
        const result = await recovery.send('Runtime.evaluate', {
          expression: `(() => { const key = ${JSON.stringify(marker)}; const state = globalThis[key]; if (location.href !== ${JSON.stringify(url)} || !state || state.marker !== key) throw Error('recovery marker mismatch'); if (state.resolve) { state.resolve(null); return true; } return false; })()`,
          awaitPromise: false, returnByValue: true,
        }, sessionId);
        assert.equal(result.exceptionDetails, undefined, 'recovery resolver must execute only in the owned world');
      } catch (error) { failures.push(error); }
    }
    const contexts = (await recovery.send('Target.getBrowserContexts')).browserContextIds;
    if (contexts.includes(target.browserContextId)) {
      // Captured from the sole Page in our newly created Context, never from
      // default/borrowed Context inventory. Dispose it even if resolution failed.
      assert.ok(targets.filter(value => value.browserContextId === target.browserContextId)
        .every(value => value.targetId === target.targetId));
      await recovery.send('Target.disposeBrowserContext', { browserContextId: target.browserContextId });
    }
    assert.equal((await recovery.send('Target.getBrowserContexts')).browserContextIds.includes(target.browserContextId), false);
    if (failures.length) throw new AggregateError(failures, 'acquisition resolver recovery failed');
  }
  function cleanup() {
    cleanupPromise ??= trace('Fixture cleanup', () => diagnosticDeadline((async () => {
      const failures = [];
      const attempt = async (phase, action) => { try { await trace(phase, action); } catch (error) { failures.push(error); } };
      // Recovery is outside the observation. A hanging close remains a failing
      // test; the resolver is never invoked while measuring ordinary behavior.
      if (watched.some(state => !state.settled)) await attempt('Cleanup unresolved native acquisition', () => nativeCleanup(true));
      await attempt('Cleanup join watched operations', () => Promise.all(watched.map(state => state.promise)));
      for (const [index, handle] of held.entries()) await attempt(`Cleanup Handle ${index + 1}`, () => handle.dispose());
      if (context) await attempt('Cleanup Context close', () => context.close());
      if (browser) await attempt('Cleanup Browser close', () => browser.close());
      await attempt('Cleanup exact native Context', () => nativeCleanup(false));
      if (browser?.pwrightMetrics) {
        await attempt('Cleanup ownership assertions', async () => {
          // Keep wrappers alive; no GC-dependent ownership release is accepted.
          for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().reservedBytes; turn++) await setImmediate();
          assert.equal(browser.pwrightMetrics().activeLeases, 0);
          assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
          assert.equal(browser.pwrightMetrics().activeOperations, 0);
          assert.equal(browser.pwrightFfiMetrics().pending, 0);
          assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
        });
      }
      if (failures.length) throw new AggregateError(failures, 'acquisition fixture cleanup failed');
    })(), 'acquisition fixture cleanup')).finally(() => trace('Observer connection cleanup', () => recovery.close()));
    return cleanupPromise;
  }
  const abortCleanup = () => { void cleanup().catch(() => {}); };
  signal.addEventListener('abort', abortCleanup, { once: true });
  try {
    browser = await trace('Browser connectOverCDP', () => type.connectOverCDP(endpoint, { timeout: 5000 }));
    context = await trace('Context create', () => browser.newContext());
    page = await trace('Page create', () => context.newPage());
    await trace('Marker create', () => page.evaluate(key => {
      globalThis[key] = { marker: key, started: false };
      location.hash = key;
    }, marker));
    const targets = (await recovery.send('Target.getTargets')).targetInfos.filter(value => value.url === url);
    assert.equal(targets.length, 1, 'unique owned acquisition target');
    [target] = targets;
    assert.equal(typeof target.browserContextId, 'string');
    assert.ok(target.browserContextId.length > 0, 'never recover the default Context');
    ({ sessionId } = await recovery.send('Target.attachToTarget', { targetId: target.targetId, flatten: true }));
    assert.equal(typeof sessionId, 'string');
    return await trace('Exercise', () => exercise({ browser, context, page, marker, watch, keep, trace }));
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    signal.removeEventListener('abort', abortCleanup);
    try { await cleanup(); }
    catch (error) { throw new AggregateError(primary ? [primary, error] : [error], 'acquisition case failed cleanup'); }
  }
}

function paired(name, exercise) {
  test(`paired Handle acquisition: ${name}`, { timeout: 25000 }, async t => {
    // Arrange
    const expected = await traced(`oracle: ${name}`, 'Run', () => withFixture(oracle, exercise, t.signal, `oracle: ${name}`));
    // Act
    const actual = await traced(`candidate: ${name}`, 'Run', () => withFixture(candidate, exercise, t.signal, `candidate: ${name}`));
    // Assert
    assert.deepEqual(actual, expected);
  });
}

async function started(page, marker, pending) {
  let polling = true;
  try {
    await diagnosticDeadline((async () => {
      while (polling) {
        assert.equal(pending.settled, false, 'acquisition must still be pending before owner close');
        if (await page.evaluate(key => globalThis[key].started, marker)) return;
        await setImmediate();
      }
    })(), 'pending acquisition start marker');
  } finally { polling = false; }
  assert.equal(pending.settled, false);
}

function pendingAcquisition(key) {
  const state = globalThis[key];
  return new Promise(resolve => {
    state.resolve = resolve;
    state.started = true;
  });
}

for (const scope of ['page', 'context', 'browser']) {
  paired(`pending Page.evaluateHandle rejects on ${scope}.close`, async fixture => {
    // Arrange: only failure cleanup can resolve this Promise, never a timer.
    const { page, marker, watch, trace } = fixture;
    const pending = watch(page.evaluateHandle(pendingAcquisition, marker), true);
    await trace('Acquisition start marker poll', () => started(page, marker, pending));
    // Act
    const closing = watch(fixture[scope].close());
    const [closed, result] = await diagnosticDeadline(Promise.all([closing.promise, pending.promise]), `${scope} close and pending Handle settlement`);
    // Assert: diagnostic deadline errors are not normalized to target closure.
    assert.equal(closed.outcome, 'resolved', closed.error?.stack);
    assert.equal(result.outcome, 'rejected');
    assert.ok(result.error instanceof Error);
    assert.match(result.error.message, /Target page, context or browser has been closed/);
    return { close: closed.outcome, acquisition: result.outcome };
  });
}

paired('pending JSHandle.evaluateHandle rejects on Page.close', async ({ page, marker, watch, keep, trace }) => {
  // Arrange
  const source = await keep(page.evaluateHandle(() => ({ answer: 42 })));
  const pending = watch(source.evaluateHandle((_source, key) => {
    const state = globalThis[key];
    return new Promise(resolve => { state.resolve = resolve; state.started = true; });
  }, marker), true);
  await trace('Acquisition start marker poll', () => started(page, marker, pending));
  // Act
  const closing = watch(page.close());
  const [closed, result] = await diagnosticDeadline(Promise.all([closing.promise, pending.promise]), 'Page close and targeted Handle settlement');
  // Assert
  assert.equal(closed.outcome, 'resolved', closed.error?.stack);
  assert.equal(result.outcome, 'rejected');
  assert.ok(result.error instanceof Error);
  assert.match(result.error.message, /Target page, context or browser has been closed/);
  return { close: closed.outcome, acquisition: result.outcome };
});

for (const [kind, expression] of [
  ['Error', () => { throw new Error('acquisition error sentinel'); }],
  ['primitive', () => { throw 'acquisition primitive sentinel'; }],
  ['nested object', () => { throw { nested: { value: 42 } }; }],
]) {
  paired(`Frame.evaluateHandle ${kind} exception leaves the Frame usable`, async ({ page, keep, watch }) => {
    // Arrange
    const frame = page.mainFrame();
    // Act
    const observed = await diagnosticDeadline(watch(frame.evaluateHandle(expression), true).promise, `${kind} exception settlement`);
    // Assert: compare normal error shape, not engine stacks or API prefixes.
    assert.equal(observed.outcome, 'rejected');
    assert.ok(observed.error instanceof Error);
    assert.equal(observed.error.name, 'Error');
    assert.ok(observed.error.message.length > 0);
    if (kind !== 'nested object') assert.match(observed.error.message, /acquisition .* sentinel/);
    const later = await keep(frame.evaluateHandle(() => ({ answer: 42 })));
    assert.equal(await later.evaluate(value => value.answer), 42);
    return { outcome: observed.outcome, name: observed.error.name, usable: true };
  });
}
