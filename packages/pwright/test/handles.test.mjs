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

async function withPage(type, exercise, expectedFfiBytes = 0) {
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  const held = [];
  let context;
  let primary;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    const retain = async (source, argument) => {
      const handle = await page.evaluateHandle(source, argument);
      held.push(handle);
      return handle;
    };
    return await exercise({ page, retain, held, type });
  } catch (error) {
    primary = error;
    throw error;
  } finally {
    const failures = [];
    for (const handle of held) {
      try { await diagnosticDeadline(handle.dispose(), 'Handle disposal'); }
      catch (error) { failures.push(error); }
    }
    try { if (context) await diagnosticDeadline(context.close(), 'Handle Context cleanup'); }
    catch (error) { failures.push(error); }
    try { await diagnosticDeadline(browser.close(), 'Handle Browser disconnect'); }
    catch (error) { failures.push(error); }
    if (browser.pwrightMetrics) {
      // Keep every disposed JS wrapper alive while checking Rust ownership.
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      // FFI belongs to the shared Node environment, unlike host/session leases.
      // A nested fixture must return to its live outer fixture's measured
      // baseline; the outermost fixture still requires complete zeroing.
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().reservedBytes !== expectedFfiBytes; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, expectedFfiBytes);
    }
    if (failures.length) throw new AggregateError(primary ? [primary, ...failures] : failures, 'Handle cleanup failed');
  }
}

function paired(name, exercise) {
  test(`paired Handle: ${name}`, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

paired('retained expression evaluation has no internal lexical bindings', async ({ retain }) => {
  const handle = await retain('[typeof c, typeof handles, typeof decode, typeof argument]');
  return handle.jsonValue();
});

paired('retained function creation uses the global non-strict world', async ({ retain }) => {
  const handle = await retain(function () { return [this === globalThis, typeof c, typeof decode]; });
  return handle.jsonValue();
});

paired('retained string declarations persist in the global world', async ({ page, retain }) => {
  await retain('var handleExpressionGlobal = 42; handleExpressionGlobal');
  return page.evaluate('globalThis.handleExpressionGlobal');
});

paired('string Handle evaluation returns expressions without invoking function results', async ({ retain }) => {
  const handle = await retain(() => ({ answer: 7 }));
  return [await handle.evaluate('6 * 7'), await handle.evaluate('Promise.resolve(43)'), await handle.evaluate('() => 99')];
});

paired('string expressions still traverse ordinary argument getters', async ({ retain }) => {
  const handle = await retain(() => ({ answer: 7 }));
  let reads = 0;
  const result = await handle.evaluate('42', { get ignored() { reads++; return 9; } });
  return { result, reads };
});

paired('disposed expression target still traverses ordinary argument getters', async ({ retain }) => {
  const handle = await retain(() => ({ answer: 7 }));
  await handle.dispose();
  let reads = 0;
  await assert.rejects(handle.evaluate('42', { get ignored() { reads++; return 9; } }));
  return reads;
});

paired('string expressions accept owned Handle arguments before ignoring their values', async ({ page, retain, held }) => {
  const argument = await retain(() => ({ answer: 7 }));
  const first = await page.evaluate('40 + 2', { argument });
  const second = await page.mainFrame().evaluateHandle('({ answer: 43 })', { argument });
  held.push(second);
  return [first, await second.jsonValue(), await argument.evaluate('44', { argument })];
});

paired('native expressions preserve the full source and argument node budgets', async ({ page, retain }) => {
  const handle = await retain(() => ({ answer: 7 }));
  const argument = [handle, ...Array(4094).fill(false)];
  const source = ' '.repeat(32768 - 2) + '42';
  return [await page.evaluate(source, argument), await handle.evaluate(source, argument)];
});

paired('string expression rejects disposed argument after later getters run', async ({ page, retain }) => {
  const handle = await retain(() => ({ answer: 7 }));
  await handle.dispose();
  let reads = 0;
  await assert.rejects(page.evaluate('globalThis.shouldNotRun = true', {
    handle, get later() { reads++; return 9; },
  }));
  return [reads, await page.evaluate('globalThis.shouldNotRun === undefined')];
});

paired('primitive jsonValue and non-element identity', async ({ retain }) => {
  const handle = await retain(() => 42);
  assert.equal(handle.asElement(), null);
  assert.equal(await handle.jsonValue(), 42);
  return handle.evaluate((value, increment) => value + increment, 3);
});

paired('object identity survives mutation', async ({ retain }) => {
  const handle = await retain(() => ({ answer: 42 }));
  await handle.evaluate(value => { value.answer += 1; });
  const result = await handle.jsonValue();
  assert.deepEqual(result, { answer: 43 });
  return result;
});

paired('ElementHandle keeps the removed node, not a matching replacement', async ({ page, retain }) => {
  await page.evaluate(() => { document.body.innerHTML = '<div id="target">original</div>'; });
  const handle = await retain(() => document.querySelector('#target'));
  assert.equal(handle.asElement(), handle);
  await page.evaluate(() => {
    const original = document.querySelector('#target');
    const replacement = original.cloneNode();
    replacement.textContent = 'replacement';
    original.replaceWith(replacement);
  });
  const result = await handle.evaluate(value => ({ text: value.textContent, connected: value.isConnected }));
  assert.deepEqual(result, { text: 'original', connected: false });
  return result;
});

paired('Frame creation accepts an expression', async ({ page, held }) => {
  const handle = await page.mainFrame().evaluateHandle('({ answer: 7 })');
  held.push(handle);
  return handle.jsonValue();
});

paired('dispose is repeatable and rejects subsequent reads', async ({ retain }) => {
  const handle = await retain(() => ({ answer: 42 }));
  await Promise.all([handle.dispose(), handle.dispose()]);
  await handle.dispose();
  await assert.rejects(handle.jsonValue());
  await assert.rejects(handle.evaluate(value => value.answer));
  return 'disposed';
});

paired('disposed wrappers do not consume the 64 native result slots', async ({ retain, held }) => {
  for (let value = 0; value < 70; value++) {
    const handle = await retain(value => value, value);
    assert.equal(await handle.jsonValue(), value);
    await handle.dispose();
  }
  assert.equal(held.length, 70);
  return held.length;
});

paired('release after owning Page retirement', async ({ page, retain }) => {
  const handle = await retain(() => document.body);
  await page.close();
  await handle.dispose();
  return 'closed';
});

paired('release receipt remains usable after native session shutdown', async ({ page, retain }) => {
  const handle = await retain(() => ({ answer: 42 }));
  await page.context().close();
  await page.context().browser().close();
  await handle.dispose();
  return 'disconnected';
});

paired('creation and reads retain deferred binary-copy timing', async ({ retain }) => {
  const bytes = new Uint8Array([1, 2]);
  const creating = retain(value => ({ bytes: [...value] }), bytes);
  bytes[0] = 9;
  const handle = await creating;
  const input = new Uint8Array([3]);
  const reading = handle.evaluate((value, arg) => [...value.bytes, ...arg], input);
  input[0] = 8;
  return reading;
});

paired('nested Handle argument reaches the original browser object', async ({ page, retain }) => {
    const handle = await retain(() => ({ answer: 42 }));
    const result = await page.evaluate(value => value.nested.answer, { nested: handle });
    assert.equal(result, 42);
    return result;
});

paired('a primitive Handle can be a root argument', async ({ page, retain }) => {
  const handle = await retain(() => 41);
  const result = await page.evaluate(value => value + 1, handle);
  assert.equal(result, 42);
  return result;
});

paired('nested aliases and cycles preserve graph identity', async ({ page, retain }) => {
  const handle = await retain(() => ({ answer: 42 }));
  const argument = { first: handle, nested: [handle] };
  argument.self = argument;
  const result = await page.evaluate(value => ({ same: value.first === value.nested[0], cycle: value === value.self, answer: value.first.answer }), argument);
  assert.deepEqual(result, { same: true, cycle: true, answer: 42 });
  return result;
});

paired('Frame Handle creation accepts an existing object reference', async ({ retain }) => {
  const original = await retain(() => ({ answer: 41 }));
  const alias = await retain(value => value.nested, { nested: original });
  await alias.evaluate(value => { value.answer++; });
  const result = await original.jsonValue();
  assert.deepEqual(result, { answer: 42 });
  return result;
});

paired('Handle evaluation accepts another retained object', async ({ retain }) => {
  const left = await retain(() => ({ answer: 40 }));
  const right = await retain(() => ({ increment: 2 }));
  const result = await left.evaluate((left, arg) => left.answer + arg.right.increment, { right });
  assert.equal(result, 42);
  return result;
});

paired('DOM Handle argument is the original removed node', async ({ page, retain }) => {
  const handle = await retain(() => { const node = document.createElement('div'); node.textContent = 'retained'; return node; });
  const result = await page.evaluate(node => ({ text: node.textContent, attached: node.isConnected }), handle);
  assert.deepEqual(result, { text: 'retained', attached: false });
  return result;
});

paired('owned references coexist with deferred binary copying', async ({ page, retain }) => {
  const handle = await retain(() => ({ answer: 40 }));
  const bytes = new Uint8Array([1]);
  const result = page.evaluate(value => value.object.answer + value.bytes[0], { object: handle, bytes });
  bytes[0] = 2;
  assert.equal(await result, 42);
  return result;
});

paired('disposed argument rejection follows ordinary getter traversal', async ({ page, retain }) => {
  const handle = await retain(() => ({ answer: 42 }));
  await handle.dispose();
  let getters = 0;
  const argument = { handle, get after() { getters++; return 1; } };
  await assert.rejects(page.evaluate(value => value, argument));
  assert.equal(getters, 1);
  return getters;
});

paired('plain objects cannot forge a native Handle using visible ID fields', async ({ page }) => {
  const argument = { _guid: 'handle@forged', _type: 'JSHandle', objectId: '1', answer: 42 };
  const result = await page.evaluate(value => value, argument);
  assert.deepEqual(result, argument);
  return result;
});

paired('foreign-session arguments reject before executing the browser program', async ({ page, type }) => {
  await page.evaluate(() => { window.node161Calls = 0; });
  const baseline = page.context().browser().pwrightFfiMetrics?.().reservedBytes ?? 0;
  return withPage(type, async ({ retain }) => {
    const foreign = await retain(() => ({ answer: 42 }));
    await assert.rejects(page.evaluate(value => { window.node161Calls++; return value.answer; }, foreign));
    const calls = await page.evaluate(() => window.node161Calls);
    assert.equal(calls, 0);
    return calls;
  }, baseline);
});

paired('Frame value evaluation needs no temporary Handle when all 64 slots are occupied', async ({ page, retain, held }) => {
  for (let answer = 0; answer < 64; answer++) await retain(answer => ({ answer }), answer);
  assert.equal(held.length, 64);
  const result = await page.evaluate(value => value.first.answer + value.last.answer, { first: held[0], last: held[63] });
  assert.equal(result, 63);
  return result;
});

test('candidate native type guard rejects a prototype-spoofed wrapper as a NativeHandle', { timeout: 15000 }, async () => {
  await withPage(candidate, async ({ page, retain }) => {
    // Arrange
    const localRequire = createRequire(import.meta.url);
    const { nativeArtifact } = localRequire('../src/native-artifact.cjs');
    const { NativeHandle } = localRequire(nativeArtifact.path);
    const handle = await retain(() => ({ answer: 42 }));
    const original = Object.getPrototypeOf(handle);
    Object.setPrototypeOf(handle, NativeHandle.prototype);
    try {
      // Act / Assert: instanceof is insufficient; native type tags must reject
      // this sidecar before napi-rs casts it to a different native class.
      assert.throws(() => page.context().browser()._session.handleJsonValue(handle), /NativeHandle|type tag|class/i);
    } finally {
      Object.setPrototypeOf(handle, original);
    }
    assert.deepEqual(await handle.jsonValue(), { answer: 42 });
  });
});
