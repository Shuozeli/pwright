import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

class UnsupportedOperationError extends Error {}
const responseBodies = createRequire(import.meta.url)('../src/response-body.cjs')({
  mapError: (error, api) => new Error(`${api}: ${error.message}`),
  UnsupportedOperationError,
});

test('Response body delegates only navigation identities to Rust', async () => {
  // Arrange
  const calls = [];
  const session = {
    async frameResponseBody(pageId, frameId, requestId) {
      calls.push([pageId, frameId, requestId]);
      return Buffer.from([0, 1, 254, 255]);
    },
  };

  // Act
  const response = responseBodies.create(session, '7', '8', 'request-9');
  const body = await response.body();
  const repeated = await response.body();

  // Assert
  assert.deepEqual(calls, [['7', '8', 'request-9']]);
  assert.deepEqual(body, Buffer.from([0, 1, 254, 255]));
  assert.deepEqual(repeated, body);
});

test('Response body keeps missing capability and native failures explicit', async () => {
  // Arrange
  const absent = {};
  const failing = { async frameResponseBody() { throw new Error('closed'); } };

  // Act
  const unsupported = responseBodies.create(absent, '7', '8', 'request-9').body();
  const failed = responseBodies.create(failing, '7', '8', 'request-9').body();

  // Assert
  await assert.rejects(unsupported, UnsupportedOperationError);
  await assert.rejects(failed, { message: 'response.body: closed' });
});

test('Response text and JSON share the same native body read', async () => {
  // Arrange
  let calls = 0;
  const session = {
    async frameResponseBody() {
      calls += 1;
      return Buffer.from('{"ok":true}', 'utf8');
    },
  };
  const response = responseBodies.create(session, '7', '8', 'request-9');

  // Act
  const text = await response.text();
  const json = await response.json();

  // Assert
  assert.equal(text, '{"ok":true}');
  assert.deepEqual(json, { ok: true });
  assert.equal(calls, 1);
});

test('Response body waits for terminal readiness before its one native read', async () => {
  // Arrange
  const calls = [];
  const session = {
    async frameResponseBody() {
      calls.push('body');
      return Buffer.from('ready');
    },
  };
  const response = responseBodies.create(
    session,
    '7',
    '8',
    'native-request-9',
    async () => { calls.push('finished'); },
  );

  // Act
  const body = await response.body();

  // Assert
  assert.deepEqual(calls, ['finished', 'body']);
  assert.equal(body.toString(), 'ready');
});
