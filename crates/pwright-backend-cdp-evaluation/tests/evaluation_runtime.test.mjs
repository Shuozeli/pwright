import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('../src/evaluation.rs', import.meta.url), 'utf8');
const match = source.match(/pub const EVALUATION_RUNTIME: &str = r#"([\s\S]*?)"#;/);
assert.ok(match, 'the test must execute the exact Rust-owned evaluation runtime');

function evaluate(pageFunction, overrides = {}, sandbox = {}) {
  sandbox.setTimeout = setTimeout;
  sandbox.clearTimeout = clearTimeout;
  const runtime = vm.runInNewContext(`(${match[1]})`, sandbox);
  return runtime({
    source: pageFunction,
    mode: 'function',
    hasArgument: false,
    argument: null,
    receiverSlot: null,
    errorBytes: 4096,
    maxDepth: 16,
    maxNodes: 128,
    maxEntries: 128,
    maxStringBytes: 1024,
    maxInlineStringBytes: 1024,
    chunkUtf16Units: 0,
    maxBinaryBytes: 1024,
    ...overrides,
  });
}

function record(value) {
  return JSON.parse(JSON.stringify(value));
}

test('a synchronous non-thenable result settles before the next timer turn', () => {
  // Arrange
  const pageFunction = '() => { setTimeout(() => { globalThis.timerFired = true; }, 0); return undefined; }';

  // Act
  const result = evaluate(pageFunction);

  // Assert
  assert.equal(typeof result?.then, 'undefined');
  assert.deepEqual(record(result), { ok: true, value: { t: 'u' } });
});

test('a synchronous expression also returns its envelope without a Promise', () => {
  // Arrange
  const expression = '21 * 2';

  // Act
  const result = evaluate(expression, { mode: 'expression' });

  // Assert
  assert.equal(typeof result?.then, 'undefined');
  assert.deepEqual(record(result), { ok: true, value: { t: 'num', v: 42 } });
});

test('a genuine Promise is awaited before its value is encoded', async () => {
  // Arrange
  const pageFunction = '() => Promise.resolve(42)';

  // Act
  const pending = evaluate(pageFunction);
  const result = await pending;

  // Assert
  assert.equal(typeof pending?.then, 'function');
  assert.deepEqual(record(result), { ok: true, value: { t: 'num', v: 42 } });
});

test('a user thenable is awaited before its value is encoded', async () => {
  // Arrange
  const pageFunction = '() => ({ then(resolve) { resolve("ready"); } })';

  // Act
  const result = await evaluate(pageFunction);

  // Assert
  assert.deepEqual(record(result), { ok: true, value: { t: 'str', v: 'ready' } });
});

test('a side-effecting then getter is read again by Promise assimilation', async () => {
  // Arrange
  const sandbox = {};
  const pageFunction = `() => {
    const value = {};
    Object.defineProperty(value, 'then', { get() {
      const read = globalThis.thenReads = (globalThis.thenReads ?? 0) + 1;
      return function(resolve) {
        globalThis.thenReceiverIsValue = this === value;
        resolve(read);
      };
    } });
    return value;
  }`;

  // Act
  const result = await evaluate(pageFunction, {}, sandbox);

  // Assert
  assert.equal(sandbox.thenReads, 2);
  assert.equal(sandbox.thenReceiverIsValue, true);
  assert.deepEqual(record(result), { ok: true, value: { t: 'num', v: 2 } });
});

test('a native same-realm Promise keeps its fulfillment despite an own then getter', async () => {
  // Arrange
  const sandbox = {};
  const pageFunction = `() => {
    const promise = Promise.resolve(42);
    Object.defineProperty(promise, 'then', { get() {
      globalThis.promiseThenReads = (globalThis.promiseThenReads ?? 0) + 1;
      return resolve => resolve(99);
    } });
    return promise;
  }`;

  // Act
  const result = await evaluate(pageFunction, {}, sandbox);

  // Assert
  assert.equal(sandbox.promiseThenReads, 1);
  assert.deepEqual(record(result), { ok: true, value: { t: 'num', v: 42 } });
});

test('a function with a then property serializes as undefined synchronously', () => {
  // Arrange
  const sandbox = {};
  const pageFunction = `() => {
    const value = () => 42;
    Object.defineProperty(value, 'then', { get() {
      globalThis.functionThenRead = true;
      return resolve => resolve('wrong');
    } });
    return value;
  }`;

  // Act
  const result = evaluate(pageFunction, {}, sandbox);

  // Assert
  assert.equal(typeof result?.then, 'undefined');
  assert.deepEqual(record(result), { ok: true, value: { t: 'u' } });
  assert.equal(sandbox.functionThenRead, undefined);
});

test('a synchronous page exception is encoded without an asynchronous turn', () => {
  // Arrange
  const pageFunction = '() => { throw new TypeError("sync boom"); }';

  // Act
  const result = evaluate(pageFunction);

  // Assert
  assert.equal(typeof result?.then, 'undefined');
  assert.equal(result.ok, false);
  assert.equal(result.kind, 'javascript');
  assert.equal(result.error.name, 'TypeError');
  assert.equal(result.error.message, 'sync boom');
  assert.match(result.error.stack, /evaluate/);
});

test('a rejected Promise remains a structured page exception', async () => {
  // Arrange
  const pageFunction = '() => Promise.reject(new Error("async boom"))';

  // Act
  const result = await evaluate(pageFunction);

  // Assert
  assert.equal(result.ok, false);
  assert.equal(result.kind, 'javascript');
  assert.equal(result.error.message, 'async boom');
});

test('a rejecting user thenable remains a structured page exception', async () => {
  // Arrange
  const pageFunction = '() => ({ then(_resolve, reject) { reject(new TypeError("thenable boom")); } })';

  // Act
  const result = await evaluate(pageFunction);

  // Assert
  assert.equal(result.ok, false);
  assert.equal(result.kind, 'javascript');
  assert.equal(result.error.name, 'TypeError');
  assert.equal(result.error.message, 'thenable boom');
});

test('a thenable cannot overturn its first resolution', async () => {
  // Arrange
  const pageFunction = '() => ({ then(resolve, reject) { resolve(17); reject(new Error("late")); throw new Error("later"); } })';

  // Act
  const result = await evaluate(pageFunction);

  // Assert
  assert.deepEqual(record(result), { ok: true, value: { t: 'num', v: 17 } });
});

test('a throwing then getter is reported as a structured page exception', () => {
  // Arrange
  const pageFunction = '() => Object.defineProperty({}, "then", { get() { throw new TypeError("then boom"); } })';

  // Act
  const result = evaluate(pageFunction);

  // Assert
  assert.equal(typeof result?.then, 'undefined');
  assert.equal(result.ok, false);
  assert.equal(result.kind, 'javascript');
  assert.equal(result.error.name, 'TypeError');
  assert.equal(result.error.message, 'then boom');
});

test('typed object serialization and byte limits stay unchanged', () => {
  // Arrange
  const pageFunction = '() => ({ answer: 42, values: [null, true] })';

  // Act
  const object = evaluate(pageFunction);
  const capped = evaluate('() => "abcdef"', { maxStringBytes: 4, maxInlineStringBytes: 4 });
  const clipped = evaluate('() => { throw new Error("lengthy message"); }', { errorBytes: 4 });

  // Assert
  assert.deepEqual(record(object), {
    ok: true,
    value: { t: 'object', id: 1, v: [
      { k: 'answer', v: { t: 'num', v: 42 } },
      { k: 'values', v: { t: 'array', id: 2, v: [{ t: 'null' }, { t: 'bool', v: true }] } },
    ] },
  });
  assert.equal(capped.kind, 'resource-exhausted');
  assert.ok(Buffer.byteLength(clipped.error.message, 'utf8') <= 4);
  assert.ok(Buffer.byteLength(clipped.error.stack, 'utf8') <= 4);
});

test('a Promise result uses the same string-byte cap', async () => {
  // Arrange
  const pageFunction = '() => Promise.resolve("abcdef")';

  // Act
  const result = await evaluate(pageFunction,
    { maxStringBytes: 4, maxInlineStringBytes: 4 });

  // Assert
  assert.equal(result.ok, false);
  assert.equal(result.kind, 'resource-exhausted');
});

test('selected chunked-result serialization keeps ordinary values inline', () => {
  // Arrange
  const selected = { chunkUtf16Units: 4, maxInlineStringBytes: 3, maxStringBytes: 10 };

  // Act
  const ordinary = evaluate('() => 42', selected);
  const largeString = evaluate('() => "abcdef"', selected);

  // Assert
  assert.equal(typeof ordinary, 'string');
  assert.ok(ordinary.startsWith('__pwright_evaluation_json__'));
  assert.deepEqual(JSON.parse(ordinary.slice('__pwright_evaluation_json__'.length)),
    { ok: true, value: { t: 'num', v: 42 } });
  assert.deepEqual(record(largeString),
    { ok: true, value: { t: 'chunked-str', v: 'abcdef', b: 6 } });
});
