import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { errors } from '@shuozeli/pwright';

const createApi = createRequire(import.meta.url)('../src/file-input.cjs');
const setInputFiles = createApi({
  mapError: error => error,
  UnsupportedOperationError: errors.UnsupportedOperationError,
  validateOptions: (options, keys) => {
    assert.ok(options && typeof options === 'object');
    for (const key of Object.keys(options)) if (!keys.includes(key)) throw new TypeError(key);
  },
});

test('upload facade forwards original input once and a Buffer shape port, without reading getters', async () => {
  // Arrange
  let reads = 0;
  const files = { get name() { reads++; throw new Error('must remain native ingress'); } };
  const calls = [];
  const native = { async setInputFiles(...args) { calls.push(args); } };
  // Act
  await setInputFiles(native, files, { timeout: 17, noWaitAfter: false });
  // Assert
  assert.equal(reads, 0);
  assert.equal(calls.length, 1);
  assert.equal(calls[0][0], files);
  assert.equal(calls[0][1], 17);
  assert.equal(calls[0][2](Buffer.alloc(0)), true);
  assert.equal(calls[0][2](new Uint8Array(0)), false);
});

test('ordinary native profile explicitly rejects uploads before reading payloads', async () => {
  // Arrange
  const files = { get name() { throw new Error('not selected'); } };
  // Act
  const result = await setInputFiles({}, files).catch(error => error);
  // Assert
  assert.ok(result instanceof errors.UnsupportedOperationError);
  assert.match(result.message, /requires the file-input native profile/);
});

test('selector upload facade passes Frame identity and strictness without constructing JS Locators', async () => {
  // Arrange
  const calls = [];
  const session = { async frameSetInputFiles(...args) { calls.push(args); } };
  const files = [];
  // Act
  await setInputFiles.forFrame(session, '42', 'input', files, { strict: true, timeout: 71 }, 'frame.setInputFiles');
  // Assert
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].slice(0, 4), ['42', 'input', files, { timeoutMs: 71, strict: true }]);
  assert.equal(calls[0][4](Buffer.alloc(0)), true);
});

test('selector upload default is non-strict', async () => {
  // Arrange
  let received;
  const session = { async frameSetInputFiles(_frame, _selector, _files, options) { received = options; } };
  // Act
  await setInputFiles.forFrame(session, '42', 'input', [], {}, 'page.setInputFiles');
  // Assert
  assert.deepEqual(received, { timeoutMs: 30000, strict: false });
});

for (const options of [{ timeout: -1 }, { timeout: 1.5 }, { timeout: 2 ** 32 }, { noWaitAfter: 'yes' }, { force: true }]) {
  test(`upload rejects malformed options ${JSON.stringify(options)} before native ingress`, async () => {
    // Arrange
    let called = false;
    const native = { async setInputFiles() { called = true; } };
    // Act
    const result = await setInputFiles(native, [], options).catch(error => error);
    // Assert
    assert.ok(result instanceof TypeError);
    assert.equal(called, false);
  });
}
