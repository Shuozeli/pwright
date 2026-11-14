import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

class UnsupportedOperationError extends Error {}
const responseFinished = createRequire(import.meta.url)('../src/response-finished.cjs')({
  mapError: (error, api) => new Error(`${api}: ${error.message}`),
  UnsupportedOperationError,
});

test('Response finished and body readiness share one opaque native owner wait', async () => {
  // Arrange
  const owner = Object.freeze({ id: 'opaque-owner' });
  const calls = [];
  const session = {
    async responseFinished(value) { calls.push(value); },
  };
  const response = responseFinished.create(session, owner);

  // Act
  const [, finished] = await Promise.all([response.beforeBody(), response.finished()]);

  // Assert
  assert.deepEqual(calls, [owner]);
  assert.equal(finished, null);
});

test('Navigation body needs no event owner while public finished stays explicit', async () => {
  // Arrange
  const response = responseFinished.create({}, undefined);

  // Act
  await response.beforeBody();
  const finished = response.finished();

  // Assert
  await assert.rejects(finished, UnsupportedOperationError);
});

test('Response finished maps native terminal failure at each public API boundary', async () => {
  // Arrange
  const failure = new Error('Response failed to finish');
  const response = responseFinished.create({ async responseFinished() { throw failure; } }, {});

  // Act
  const body = response.beforeBody();

  // Assert
  await assert.rejects(body, { message: 'response.body: Response failed to finish' });
});
