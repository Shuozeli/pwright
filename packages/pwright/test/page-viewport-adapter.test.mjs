import assert from 'node:assert/strict';
import { test } from 'node:test';
import pageViewportFactory from '../src/page-viewport.cjs';

function adapter(calls) {
  return pageViewportFactory({
    mapError: error => error,
    validateOptions(options, allowed, api) {
      calls.push(['validateOptions', options, allowed, api]);
    },
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
}

test('Page setViewportSize delegates one validated page-scoped override to Rust', async () => {
  // Arrange
  const calls = [];
  const viewport = { width: 500, height: 400 };
  const session = {
    async pageSetViewportSize(pageId, width, height) {
      calls.push(['pageSetViewportSize', pageId, width, height]);
    },
  };

  // Act
  await adapter(calls).set(session, 'page-id', viewport);

  // Assert
  assert.deepEqual(calls, [
    ['validateOptions', viewport, ['width', 'height'], 'page.setViewportSize'],
    ['pageSetViewportSize', 'page-id', 500, 400],
  ]);
});

test('Page setViewportSize rejects noninteger dimensions before native execution', async () => {
  // Arrange
  const calls = [];
  const session = { async pageSetViewportSize() { calls.push(['native']); } };

  // Act
  const operation = adapter(calls).set(session, 'page-id', { width: 500.5, height: 400 });

  // Assert
  await assert.rejects(operation, /width and height must be integers/);
  assert.equal(calls.some(call => call[0] === 'native'), false);
});
