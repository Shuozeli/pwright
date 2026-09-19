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
const phaseTrace = process.env.PWRIGHT_HANDLE_PHASE_TRACE === '1';

function traced(label, phase, operation) {
  if (!phaseTrace) return operation();
  const started = performance.now();
  const emit = (event, details = {}) => process.stderr.write(`${JSON.stringify({
    diagnostic: 'handle-producer-phase', timestamp: new Date().toISOString(),
    label, phase, event, elapsedMs: Math.round(performance.now() - started), ...details,
  })}\n`);
  emit('start');
  let pending;
  try { pending = operation(); }
  catch (error) {
    emit('end', { outcome: 'error', error: String(error).slice(0, 200) });
    throw error;
  }
  return Promise.resolve(pending).then(value => {
    emit('end', { outcome: 'ok' });
    return value;
  }, error => {
    emit('end', { outcome: 'error', error: String(error).slice(0, 200) });
    throw error;
  });
}

async function withPage(type, exercise, label) {
  const browser = await traced(label, 'Browser connectOverCDP', () => type.connectOverCDP(endpoint, { timeout: 5000 }));
  const held = [];
  let acquisition = 0;
  let context;
  let primary;
  try {
    context = await traced(label, 'Context create', () => browser.newContext());
    const page = await traced(label, 'Page create', () => context.newPage());
    const keep = async promise => {
      const handle = await traced(label, `Handle result ${++acquisition} wait`, () => promise);
      held.push(handle);
      return handle;
    };
    const retain = (source, argument) => keep(page.evaluateHandle(source, argument));
    const properties = async handle => {
      const result = await traced(label, 'getProperties result wait', () => handle.getProperties());
      held.push(...result.values());
      return result;
    };
    return await traced(label, 'Exercise', () => exercise({ page, retain, keep, properties, held }));
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    const failures = [];
    for (const [index, handle] of held.entries()) {
      try { await traced(label, `Handle cleanup ${index + 1}`, () => diagnosticDeadline(handle.dispose(), 'producer Handle disposal')); }
      catch (error) { failures.push(error); }
    }
    try { if (context) await traced(label, 'Context cleanup', () => diagnosticDeadline(context.close(), 'producer Context cleanup')); }
    catch (error) { failures.push(error); }
    try { await traced(label, 'Browser cleanup', () => diagnosticDeadline(browser.close(), 'producer Browser disconnect')); }
    catch (error) { failures.push(error); }
    if (browser.pwrightMetrics) {
      // All JS wrappers remain alive: native ownership must not await GC.
      await traced(label, 'Ownership checks', async () => {
        assert.equal(browser.pwrightMetrics().activeLeases, 0);
        assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
        for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().reservedBytes !== 0; turn++) await setImmediate();
        assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      });
    }
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures, 'producer cleanup failed');
  }
}

function paired(name, exercise) {
  test(`paired Handle producer: ${name}`, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await traced(`oracle: ${name}`, 'Run', () => withPage(oracle, exercise, `oracle: ${name}`));
    // Act
    const actual = await traced(`candidate: ${name}`, 'Run', () => withPage(candidate, exercise, `candidate: ${name}`));
    // Assert
    assert.deepEqual(actual, expected);
  });
}

paired('evaluateHandle function produces a retained primitive', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => ({ answer: 40 }));
  // Act
  const result = await keep(source.evaluateHandle((value, increment) => value.answer + increment, 2));
  // Assert
  assert.equal(result.asElement(), null);
  const value = await result.jsonValue();
  assert.equal(value, 42);
  return value;
});

paired('evaluateHandle string awaits its expression without invoking a function result', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => ({ ignored: true }));
  // Act
  const result = await keep(source.evaluateHandle('Promise.resolve(() => 42)'));
  // Assert
  const value = await result.evaluate(fn => ({ type: typeof fn, answer: fn() }));
  assert.deepEqual(value, { type: 'function', answer: 42 });
  return value;
});

paired('evaluateHandle preserves nested Handle aliases and argument cycles', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => ({ answer: 40 }));
  const increment = await retain(() => ({ amount: 2 }));
  const argument = { first: increment, nested: [increment] };
  argument.self = argument;
  // Act
  const result = await keep(source.evaluateHandle((source, argument) => ({
    source, first: argument.first, nested: argument.nested[0],
    cycle: argument === argument.self,
  }), argument));
  // Assert
  const value = await result.evaluate(value => ({
    answer: value.source.answer + value.first.amount,
    alias: value.first === value.nested,
    cycle: value.cycle,
  }));
  assert.deepEqual(value, { answer: 42, alias: true, cycle: true });
  return value;
});

paired('evaluateHandle result retains mutation identity after the source is disposed', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => ({ child: { answer: 41 } }));
  const child = await keep(source.evaluateHandle(value => value.child));
  await source.evaluate(value => { value.child.answer++; });
  // Act
  await source.dispose();
  // Assert
  const value = await child.jsonValue();
  assert.deepEqual(value, { answer: 42 });
  return value;
});

for (const [kind, source] of [
  ['function', value => value],
  ['string', '42'],
]) {
  paired(`evaluateHandle ${kind} traverses argument getters before disposed-target rejection`, async ({ retain, keep }) => {
    // Arrange
    const target = await retain(() => ({ answer: 42 }));
    await target.dispose();
    let reads = 0;
    const argument = { get later() { reads++; return 7; } };
    // Act / Assert
    await assert.rejects(keep(target.evaluateHandle(source, argument)));
    assert.equal(reads, 1);
    return reads;
  });
}

paired('evaluateHandle rejects a disposed argument after later getters without running the function', async ({ page, retain, keep }) => {
  // Arrange
  const target = await retain(() => ({ answer: 42 }));
  const disposed = await retain(() => ({ value: 7 }));
  await disposed.dispose();
  let reads = 0;
  const argument = { disposed, get later() { reads++; return 9; } };
  // Act / Assert
  await assert.rejects(keep(target.evaluateHandle((value, argument) => {
    globalThis.producerShouldNotRun = true;
    return { value, argument };
  }, argument)));
  const ran = await page.evaluate(() => globalThis.producerShouldNotRun === true);
  assert.equal(reads, 1);
  assert.equal(ran, false);
  return { reads, ran };
});

paired('getProperty returns a Handle for a missing property', async ({ retain, keep }) => {
  // Arrange
  const target = await retain(() => ({ answer: 42 }));
  // Act
  const property = await keep(target.getProperty('missing'));
  // Assert
  const value = await property.jsonValue();
  assert.equal(value, undefined);
  assert.equal(property.asElement(), null);
  return value;
});

paired('getProperty reads an inherited property', async ({ retain, keep }) => {
  // Arrange
  const target = await retain(() => Object.create({ inherited: { answer: 42 } }));
  // Act
  const property = await keep(target.getProperty('inherited'));
  // Assert
  const value = await property.jsonValue();
  assert.deepEqual(value, { answer: 42 });
  return value;
});

paired('getProperty invokes an accessor exactly once', async ({ retain, keep }) => {
  // Arrange
  const target = await retain(() => ({ reads: 0, get answer() { this.reads++; return 42; } }));
  // Act
  const property = await keep(target.getProperty('answer'));
  // Assert
  const value = { answer: await property.jsonValue(), reads: await target.evaluate(value => value.reads) };
  assert.deepEqual(value, { answer: 42, reads: 1 });
  return value;
});

paired('getProperty retains a Promise-valued property without awaiting it', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => ({ value: Promise.resolve(42) }));
  // Act
  const property = await keep(source.getProperty('value'));
  // Assert
  const value = {
    tag: await property.evaluate(value => Object.prototype.toString.call(value)),
    same: await source.evaluate((source, property) => source.value === property, property),
  };
  assert.deepEqual(value, { tag: '[object Promise]', same: true });
  return value;
});

paired('getProperty does not inspect the returned object then accessor', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => {
    const source = { reads: 0, value: { answer: 42 } };
    Object.defineProperty(source.value, 'then', {
      enumerable: true,
      get() { source.reads++; return undefined; },
    });
    return source;
  });
  // Act
  const property = await keep(source.getProperty('value'));
  // Assert
  const value = {
    reads: await source.evaluate(source => source.reads),
    answer: await property.evaluate(value => value.answer),
    same: await source.evaluate((source, property) => source.value === property, property),
  };
  assert.deepEqual(value, { reads: 0, answer: 42, same: true });
  return value;
});

paired('getProperty characterizes a property literally named then', async ({ retain, keep }) => {
  // Arrange
  const source = await retain(() => ({ calls: 0 }));
  await source.evaluate(value => {
    value.then = resolve => { value.calls++; resolve({ marker: 7 }); };
  });
  // Act
  let returnedHandle = false;
  try {
    await keep(source.getProperty('then'));
    returnedHandle = true;
  } catch {
    // The pinned oracle uses the requested name on its temporary carrier.
    // Characterize success/failure and assimilation, not error wording; this
    // then function always resolves, so the probe cannot wait on it forever.
  }
  // Assert: the paired harness compares this bounded observation to the oracle.
  const calls = await source.evaluate(value => value.calls);
  return { returnedHandle, calls };
});

paired('getProperty accessor failure leaves the source and later property reads usable', async ({ retain, keep }) => {
  // Arrange
  const target = await retain(() => ({
    reads: 0,
    answer: 42,
    get broken() { this.reads++; throw new Error('producer getter failed'); },
  }));
  // Act
  const failed = keep(target.getProperty('broken'));
  // Assert
  await assert.rejects(failed, /producer getter failed/);
  const source = await target.evaluate(value => ({ reads: value.reads, answer: value.answer }));
  const property = await keep(target.getProperty('answer'));
  const value = { source, answer: await property.jsonValue() };
  assert.deepEqual(value, { source: { reads: 1, answer: 42 }, answer: 42 });
  return value;
});

paired('getProperty preserves an own __proto__ property as data', async ({ retain, keep }) => {
  // Arrange
  const target = await retain(() => Object.defineProperty({}, '__proto__', {
    enumerable: true, value: { answer: 42 },
  }));
  // Act
  const property = await keep(target.getProperty('__proto__'));
  // Assert
  const value = await property.jsonValue();
  assert.deepEqual(value, { answer: 42 });
  return value;
});

paired('getProperty returns the retained DOM node, not a matching replacement', async ({ page, retain, keep }) => {
  // Arrange
  const target = await retain(() => {
    const node = document.createElement('div');
    node.id = 'producer-node';
    node.textContent = 'original';
    document.body.appendChild(node);
    return { node };
  });
  const property = await keep(target.getProperty('node'));
  // Act
  await page.evaluate(() => {
    const original = document.querySelector('#producer-node');
    const replacement = original.cloneNode();
    replacement.textContent = 'replacement';
    original.replaceWith(replacement);
  });
  // Assert
  assert.equal(property.asElement(), property);
  const value = await property.evaluate(node => ({ text: node.textContent, connected: node.isConnected }));
  assert.deepEqual(value, { text: 'original', connected: false });
  return value;
});

paired('getProperties returns own enumerable data without invoking getters', async ({ retain, properties }) => {
  // Arrange
  const target = await retain(() => {
    const value = Object.assign(Object.create({ inherited: 7 }), { own: 42, missingValue: undefined });
    Object.defineProperty(value, 'reads', { value: 0, writable: true });
    Object.defineProperty(value, 'hidden', { value: 9 });
    Object.defineProperty(value, 'getter', { enumerable: true, get() { this.reads++; return 11; } });
    return value;
  });
  // Act
  const result = await properties(target);
  // Assert
  assert.ok(result instanceof Map);
  const value = {
    names: [...result.keys()],
    own: await result.get('own').jsonValue(),
    missingValue: await result.get('missingValue').jsonValue(),
    reads: await target.evaluate(value => value.reads),
  };
  assert.deepEqual(value, { names: ['own', 'missingValue'], own: 42, missingValue: undefined, reads: 0 });
  return value;
});

paired('getProperties preserves indexed then insertion property order', async ({ retain, properties }) => {
  // Arrange
  const target = await retain(() => {
    const value = {};
    value.zebra = 'z';
    value[9] = 'nine';
    value.alpha = 'a';
    value[2] = 'two';
    return value;
  });
  // Act
  const result = await properties(target);
  // Assert
  const names = [...result.keys()];
  assert.deepEqual(names, ['2', '9', 'zebra', 'alpha']);
  return names;
});

paired('getProperties gives aliases independent ownership from the source and each other', async ({ retain, properties }) => {
  // Arrange
  const target = await retain(() => {
    const child = { answer: 41 };
    return { first: child, second: child };
  });
  const result = await properties(target);
  const first = result.get('first');
  const second = result.get('second');
  await first.evaluate(value => { value.answer++; });
  await target.dispose();
  // Act
  await first.dispose();
  // Assert
  const value = await second.jsonValue();
  assert.deepEqual(value, { answer: 42 });
  return value;
});

paired('getProperties yields no properties for a primitive Handle', async ({ retain, properties }) => {
  // Arrange
  const target = await retain(() => 'primitive');
  // Act
  const result = await properties(target);
  // Assert
  assert.ok(result instanceof Map);
  assert.equal(result.size, 0);
  return [...result];
});

paired('getProperties classifies a DOM-valued property as an ElementHandle', async ({ retain, properties }) => {
  // Arrange
  const target = await retain(() => {
    const node = document.createElement('div');
    node.textContent = 'retained';
    return { node };
  });
  // Act
  const result = await properties(target);
  // Assert
  const node = result.get('node');
  assert.equal(node.asElement(), node);
  const value = await node.evaluate(node => ({ text: node.textContent, connected: node.isConnected }));
  assert.deepEqual(value, { text: 'retained', connected: false });
  return value;
});
