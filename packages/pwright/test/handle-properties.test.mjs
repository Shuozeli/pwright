import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { setImmediate } from 'node:timers/promises';
import { test } from 'node:test';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = require('playwright');
const { chromium: candidate, compatibility } = await import('@shuozeli/pwright');
assert.equal(compatibility.nativeProfile, 'handles');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const phaseTrace = process.env.PWRIGHT_HANDLE_PROPERTIES_TRACE === '1';

function traced(label, phase, operation) {
  if (!phaseTrace) return operation();
  const start = performance.now();
  const emit = (event, error) => process.stderr.write(`${JSON.stringify({
    diagnostic: 'handle-properties-phase', timestamp: new Date().toISOString(),
    label, phase, event, elapsedMs: Math.round(performance.now() - start),
    ...(error === undefined ? {} : { error: String(error).slice(0, 512) }),
  })}\n`);
  emit('start');
  let result;
  try { result = operation(); }
  catch (error) { emit('error', error); throw error; }
  return Promise.resolve(result).then(value => {
    emit('ok');
    return value;
  }, error => { emit('error', error); throw error; });
}

async function withPage(type, exercise, label) {
  const trace = (phase, operation) => traced(label, phase, operation);
  const browser = await trace('Browser connect', () => type.connectOverCDP(endpoint, { timeout: 5000 }));
  const held = [];
  let context, primary;
  try {
    context = await trace('Context create', () => browser.newContext());
    const page = await trace('Page create', () => context.newPage());
    const retain = async (source, argument) => {
      const handle = await trace('Source Handle create', () => page.evaluateHandle(source, argument));
      held.push(handle);
      return handle;
    };
    const properties = async source => {
      const result = await trace('Property batch publication', () => source.getProperties());
      held.push(...result.values());
      return result;
    };
    return await trace('Exercise', () => exercise({ browser, page, retain, properties }));
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    const failures = [];
    const attempt = async (phase, action) => {
      try { await trace(phase, () => diagnosticDeadline(action(), phase)); }
      catch (error) { failures.push(error); }
    };
    for (const [index, handle] of held.entries()) await attempt(`Cleanup Handle ${index + 1}`, () => handle.dispose());
    if (context) await attempt('Cleanup Context', () => context.close());
    await attempt('Cleanup Browser', () => browser.close());
    if (browser.pwrightMetrics) {
      await attempt('Zero ownership with wrappers retained', async () => {
        for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().reservedBytes; turn++) await setImmediate();
        assert.equal(browser.pwrightMetrics().activeLeases, 0);
        assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
        assert.equal(browser.pwrightMetrics().activeOperations, 0);
        assert.equal(browser.pwrightFfiMetrics().pending, 0);
        assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      });
    }
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures, 'property batch cleanup failed');
  }
}

function paired(name, exercise) {
  test(`paired Handle properties: ${name}`, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise, `oracle: ${name}`);
    // Act
    const actual = await withPage(candidate, exercise, `candidate: ${name}`);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

async function rejectedRead(promise, label) {
  const outcome = await diagnosticDeadline(Promise.resolve(promise).then(
    () => ({ rejected: false }), error => ({ rejected: true, error }),
  ), label);
  // A watchdog rejection escapes this helper and fails the test. It is never
  // accepted as evidence that the closed target rejected an actual API read.
  assert.equal(outcome.rejected, true);
  assert.ok(outcome.error instanceof Error);
  return outcome.error;
}

async function ownershipSnapshot(browser, label) {
  let polling = true;
  let previous;
  try {
    return await diagnosticDeadline((async () => {
      while (polling) {
        const host = browser.pwrightMetrics();
        const ffi = browser.pwrightFfiMetrics();
        const snapshot = {
          liveObjects: host.liveObjects, activeLeases: host.activeLeases, liveScopes: host.liveScopes,
          pendingCalls: host.pendingCalls, activeOperations: host.activeOperations, cleanupInFlight: host.cleanupInFlight,
          ffiPending: ffi.pending, ffiQueued: ffi.queued, ffiQueuedCommands: ffi.queuedCommands,
          ffiReservedBytes: ffi.reservedBytes, ffiWatches: ffi.watches,
        };
        // The live Browser's watcher is legitimate admission, not zero. Compare
        // its measured baseline after ordinary commands/cleanup have settled.
        const idle = host.pendingCalls === 0 && host.activeOperations === 0 && host.cleanupInFlight === 0
          && ffi.commands === 0 && ffi.connects === 0 && ffi.closes === 0 && ffi.queuedCommands === 0;
        const encoded = JSON.stringify(snapshot);
        if (idle && encoded === previous) return snapshot;
        previous = idle ? encoded : undefined;
        await setImmediate();
      }
      throw new Error('ownership observation cancelled');
    })(), label);
  } finally { polling = false; }
}

paired('ordered own data properties retain independently usable children', async ({ retain, properties }) => {
  // Arrange
  const source = await retain(() => {
    const value = Object.create({ inherited: { answer: 99 } });
    value.alpha = { answer: 1 };
    value[9] = { answer: 9 };
    value.zeta = { answer: 3 };
    value[2] = { answer: 2 };
    Object.defineProperty(value, '__proto__', { enumerable: true, value: { answer: 4 } });
    Object.defineProperty(value, 'reads', { writable: true, value: 0 });
    Object.defineProperty(value, 'hidden', { value: { answer: 88 } });
    Object.defineProperty(value, 'accessor', { enumerable: true, get() { this.reads++; return 77; } });
    return value;
  });
  // Act
  const result = await properties(source);
  // Assert
  assert.ok(result instanceof Map);
  const names = [...result.keys()];
  assert.deepEqual(names, ['2', '9', 'alpha', 'zeta', '__proto__']);
  assert.equal(await source.evaluate(value => value.reads), 0);
  assert.equal(new Set(result.values()).size, 5);
  await source.dispose();
  const answers = [];
  for (const child of result.values()) {
    assert.equal(child.asElement(), null);
    answers.push(await child.evaluate(value => value.answer));
  }
  assert.deepEqual(answers, [2, 9, 1, 3, 4]);
  return { names, answers };
});

paired('DOM property keeps ElementHandle classification and detached-node identity', async ({ page, retain, properties }) => {
  // Arrange
  const source = await retain(() => {
    const node = document.createElement('div');
    node.id = 'property-batch-original';
    node.textContent = 'original';
    document.body.appendChild(node);
    return { node, plain: { answer: 42 } };
  });
  const result = await properties(source);
  const node = result.get('node');
  assert.equal(node.asElement(), node);
  assert.equal(result.get('plain').asElement(), null);
  // Act
  await page.evaluate(() => {
    const original = document.getElementById('property-batch-original');
    const replacement = original.cloneNode();
    replacement.textContent = 'replacement';
    original.replaceWith(replacement);
  });
  await source.dispose();
  // Assert
  assert.equal(node.asElement(), node);
  const observed = await node.evaluate(value => ({ text: value.textContent, connected: value.isConnected }));
  assert.deepEqual(observed, { text: 'original', connected: false });
  assert.equal(await page.evaluate(() => document.getElementById('property-batch-original').textContent), 'replacement');
  return observed;
});

paired('aliased children survive source and sibling disposal with shared object identity', async ({ retain, properties }) => {
  // Arrange
  const source = await retain(() => {
    const child = { answer: 41 };
    return { first: child, second: child };
  });
  const result = await properties(source);
  const first = result.get('first');
  const second = result.get('second');
  assert.notEqual(first, second);
  assert.equal(await first.evaluate((value, other) => value === other, second), true);
  await first.evaluate(value => { value.answer++; });
  await source.dispose();
  // Act
  await first.dispose();
  // Assert
  const observed = await second.jsonValue();
  assert.deepEqual(observed, { answer: 42 });
  await first.dispose();
  assert.deepEqual(await second.jsonValue(), observed);
  return observed;
});

paired('Page.close invalidates property reads while sibling disposal remains acknowledged', async ({ page, retain, properties }) => {
  // Arrange
  const source = await retain(() => ({ first: { answer: 1 }, second: { answer: 2 }, node: document.createElement('span') }));
  const result = await properties(source);
  const node = result.get('node');
  assert.equal(node.asElement(), node);
  // Act
  await diagnosticDeadline(page.close(), 'property owner Page close');
  // Assert
  assert.equal(page.isClosed(), true);
  await rejectedRead(source.getProperties(), 'closed source property enumeration');
  for (const child of result.values()) {
    await rejectedRead(child.evaluate(value => value), 'closed property read');
    await diagnosticDeadline(child.dispose(), 'closed property disposal');
    await diagnosticDeadline(child.dispose(), 'repeated closed property disposal');
  }
  await source.dispose();
  assert.equal(node.asElement(), node);
  // withPage checks exact zero host/FFI ownership after close, keeping source
  // and every sibling wrapper alive until those accounting assertions finish.
  return { closed: page.isClosed(), children: result.size, disposed: true };
});

const defineProperty = Object.defineProperty;
const getOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
const hasOwn = Object.hasOwn;
function poisonSetter(prototype, key, matches) {
  const previous = getOwnPropertyDescriptor(prototype, key);
  assert.equal(previous, undefined, 'publication poison requires an unmodified prototype slot');
  let calls = 0;
  defineProperty(prototype, key, {
    configurable: true,
    get() { return undefined; },
    set(value) {
      if (matches(value)) {
        calls++;
        throw new Error(`native property publication invoked inherited ${key} setter`);
      }
      // Preserve ordinary unrelated assignments during the single async call;
      // the test runner and Node's own arrays must not become failure targets.
      defineProperty(this, key, { value, writable: true, enumerable: true, configurable: true });
    },
  });
  return {
    calls: () => calls,
    restore() { delete prototype[key]; },
  };
}

function nativePropertyName(value) {
  return value === 'poison-first' || value === 'poison-second';
}
function nativeHandle(value) {
  return value !== null && typeof value === 'object' && typeof value.isElement === 'function';
}
async function poisonSource(retain) {
  return retain(() => ({
    'poison-first': { answer: 42 },
    'poison-second': document.createElement('div'),
  }));
}
async function assertPublishedPoisonResult(result) {
  assert.deepEqual([...result.keys()], ['poison-first', 'poison-second']);
  assert.deepEqual(await result.get('poison-first').jsonValue(), { answer: 42 });
  const element = result.get('poison-second');
  assert.equal(element.asElement(), element);
  assert.equal(await element.evaluate(value => value.nodeName), 'DIV');
}

// These are candidate-specific N-API boundary tests, not claims about a
// Playwright server's unrelated Node object construction implementation.
test('native property publication bypasses poisoned Object.prototype name and handle setters', { timeout: 25000 }, async () => {
  await withPage(candidate, async ({ retain, properties }) => {
    // Arrange
    const source = await poisonSource(retain);
    const name = poisonSetter(Object.prototype, 'name', nativePropertyName);
    let handle, result;
    try {
      handle = poisonSetter(Object.prototype, 'handle', nativeHandle);
      // Act
      result = await diagnosticDeadline(properties(source), 'Object prototype property publication');
    } finally {
      handle?.restore();
      name.restore();
    }
    // Assert: inherited setters cannot intercept or swallow native owners.
    assert.equal(name.calls(), 0);
    assert.equal(handle.calls(), 0);
    assert.equal(getOwnPropertyDescriptor(Object.prototype, 'name'), undefined);
    assert.equal(getOwnPropertyDescriptor(Object.prototype, 'handle'), undefined);
    await assertPublishedPoisonResult(result);
  }, 'candidate: Object prototype publication');
});

test('native property publication bypasses a poisoned Array.prototype result-index setter', { timeout: 25000 }, async () => {
  await withPage(candidate, async ({ retain, properties }) => {
    // Arrange
    const source = await poisonSource(retain);
    const index = poisonSetter(Array.prototype, '0', value => value !== null && typeof value === 'object'
      && hasOwn(value, 'name') && nativePropertyName(value.name) && hasOwn(value, 'handle') && nativeHandle(value.handle));
    let result;
    try {
      // Act
      result = await diagnosticDeadline(properties(source), 'Array prototype property publication');
    } finally { index.restore(); }
    // Assert
    assert.equal(index.calls(), 0);
    assert.equal(getOwnPropertyDescriptor(Array.prototype, '0'), undefined);
    await assertPublishedPoisonResult(result);
  }, 'candidate: Array prototype publication');
});

test('configured 64-Handle capacity admits 63 siblings and reuses their released slots without GC', { timeout: 25000 }, async () => {
  await withPage(candidate, async ({ browser, retain, properties }) => {
    // Arrange: the selected Rust handles profile configures MAX_HANDLES = 64;
    // this is a capacity contract, not a Playwright-wide property-count limit.
    const childCount = 63;
    const source = await retain(count => Object.fromEntries(Array.from({ length: count }, (_unused, index) => [
      `capacity-child-${index}`, { index },
    ])), childCount);
    const baseline = await ownershipSnapshot(browser, 'source-only capacity baseline');
    const first = await properties(source);
    assert.equal(first.size, childCount);
    assert.equal(first.size + 1, 64, 'source plus independently owned children fill the configured capacity');
    const full = await ownershipSnapshot(browser, 'full Handle capacity baseline');
    // Act
    const overflow = await rejectedRead(properties(source), 'full-capacity property batch rejection');
    // Assert: failed enumeration cannot consume or release the existing batch.
    assert.equal(overflow.code, 'PWRIGHT_RESOURCE_EXHAUSTED');
    assert.deepEqual(await ownershipSnapshot(browser, 'capacity after rejected batch'), full);
    assert.equal(await first.get('capacity-child-0').evaluate(value => value.index), 0);
    assert.equal(await first.get('capacity-child-62').evaluate(value => value.index), 62);
    for (const child of first.values()) await child.dispose();
    assert.deepEqual(await ownershipSnapshot(browser, 'capacity after acknowledged disposal'), baseline);
    assert.equal(await source.evaluate(value => Object.keys(value).length), childCount);
    // Reuse the entire capacity while source and all disposed wrappers remain
    // strongly referenced. No GC request, source disposal or reconnect occurs.
    const reused = await properties(source);
    assert.deepEqual([...reused.keys()], [...first.keys()]);
    assert.equal(reused.size, childCount);
    assert.notEqual(reused.get('capacity-child-0'), first.get('capacity-child-0'));
    assert.equal(await reused.get('capacity-child-62').evaluate(value => value.index), 62);
    for (const child of reused.values()) await child.dispose();
    assert.deepEqual(await ownershipSnapshot(browser, 'capacity after reused batch disposal'), baseline);
    assert.equal(first.size, childCount);
    assert.equal(reused.size, childCount);
  }, 'candidate: full configured Handle capacity');
});

test('live Map.set failure disposes published and unvisited native children without GC', { timeout: 25000 }, async () => {
  await withPage(candidate, async ({ browser, retain, properties }) => {
    // Arrange
    const prefix = 'rollback-child-';
    const source = await retain(() => ({
      'rollback-child-first': { answer: 1 },
      'rollback-child-second': { answer: 2 },
      'rollback-child-unvisited': { answer: 3 },
    }));
    const baseline = await ownershipSnapshot(browser, 'source-only rollback baseline');
    const primary = new Error('second rollback-prefixed Map insertion failed');
    const localRequire = createRequire(import.meta.url);
    const { nativeArtifact } = localRequire('../src/native-artifact.cjs');
    const { NativeSession } = localRequire(nativeArtifact.path);
    const nativePrototype = NativeSession.prototype;
    const nativeDescriptor = getOwnPropertyDescriptor(nativePrototype, 'handleGetProperties');
    const mapDescriptor = getOwnPropertyDescriptor(Map.prototype, 'set');
    assert.equal(typeof nativeDescriptor?.value, 'function');
    assert.ok(nativeDescriptor.configurable || nativeDescriptor.writable, 'exported native method must permit a reversible test observer');
    const originalNative = nativeDescriptor.value;
    const originalSet = mapDescriptor.value;
    const exposedWrappers = [];
    let nativeRows, nativeOwner, targetedInsertions = 0, failure;
    try {
      // Test-only boundary observation on an exported raw class method. Forward
      // the exact receiver/arguments/result without reading JS private fields.
      // Retain even the unvisited raw child so GC cannot satisfy the assertions.
      defineProperty(nativePrototype, 'handleGetProperties', {
        ...nativeDescriptor,
        value: async function (...args) {
          const rows = await Reflect.apply(originalNative, this, args);
          if (Array.isArray(rows) && rows.length === 3 && rows.every(row => row.name.startsWith(prefix))) {
            nativeRows = rows;
            nativeOwner = this;
          }
          return rows;
        },
      });
      defineProperty(Map.prototype, 'set', {
        ...mapDescriptor,
        value: function (key, value) {
          if (typeof key === 'string' && key.startsWith(prefix) && typeof value?.dispose === 'function') {
            exposedWrappers.push(value);
            if (++targetedInsertions === 2) throw primary;
          }
          return Reflect.apply(originalSet, this, [key, value]);
        },
      });
      let observed;
      try {
        // Act: attach rejection handling immediately; watchdogs cannot pass as
        // the expected publication failure, and all poison is scoped to this call.
        observed = await rejectedRead(properties(source), 'poisoned Map publication rollback');
      } finally {
        defineProperty(Map.prototype, 'set', mapDescriptor);
        defineProperty(nativePrototype, 'handleGetProperties', nativeDescriptor);
      }
      // Assert before fallback cleanup: all three raw classes remain reachable,
      // including the child that never reached Map.set or a public JS wrapper.
      assert.equal(observed, primary);
      assert.equal(targetedInsertions, 2);
      assert.equal(exposedWrappers.length, 2);
      assert.ok(Array.isArray(nativeRows), 'exported native boundary observer retained the complete result batch');
      assert.equal(nativeRows.length, 3);
      for (const row of nativeRows) assert.throws(() => row.handle.isElement(), /disposed/);
      for (const wrapper of exposedWrappers) await rejectedRead(wrapper.evaluate(value => value), 'rolled-back wrapper read');
      assert.equal(await source.evaluate(value => value['rollback-child-unvisited'].answer), 3);
      assert.deepEqual(await ownershipSnapshot(browser, 'rollback returns every native owner'), baseline);
      assert.equal(nativeRows.length, 3);
      assert.equal(exposedWrappers.length, 2);
    } catch (error) {
      failure = error;
      throw error;
    } finally {
      defineProperty(Map.prototype, 'set', mapDescriptor);
      defineProperty(nativePrototype, 'handleGetProperties', nativeDescriptor);
      const cleanupFailures = [];
      // This recovery runs only after the observations above (including a red
      // assertion). It cannot turn a missed rollback into a passing result.
      for (const row of nativeRows ?? []) {
        try { await diagnosticDeadline(nativeOwner.handleDispose(row.handle), 'observed raw child cleanup'); }
        catch (error) { cleanupFailures.push(error); }
      }
      if (cleanupFailures.length) throw new AggregateError(failure ? [failure, ...cleanupFailures] : cleanupFailures, 'rollback observer cleanup failed');
    }
  }, 'candidate: live Map publication rollback');
});
