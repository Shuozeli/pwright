import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const FrameCache = require('../src/frame-cache.cjs');
class UnsupportedOperationError extends Error {}
const { Request } = require('../src/page-routes.cjs')({
  mapError: error => error,
  validateOptions: () => {},
  UnsupportedOperationError,
});

function requestMetadata(frameId = '3') {
  return {
    url: 'https://example.test/child',
    method: 'GET',
    resourceType: 'document',
    headers: [],
    postData: null,
    isNavigationRequest: true,
    frameId,
  };
}

test('Request.frame retains its creation Frame after that Frame detaches', () => {
  // Arrange
  const root = { _id: '1' };
  let liveFrames = [{ id: '1', parentId: null }, { id: '3', parentId: '1' }];
  let enumerations = 0;
  const cache = new FrameCache({
    frames: () => { enumerations += 1; return liveFrames; },
    metadata: id => ({ id }),
  }, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));
  const page = { _frameCache: cache, frames: () => cache.frames() };
  const frame = page.frames()[1];
  const request = new Request(page, requestMetadata());
  const beforeDetach = enumerations;

  // Act
  liveFrames = [{ id: '1', parentId: null }];
  page.frames();

  // Assert
  assert.equal(beforeDetach, 1);
  assert.equal(enumerations, 2);
  assert.deepEqual([...cache._frames.keys()], ['1']);
  assert.equal(request.frame(), frame);
});

test('Request.frame remembers a Frame first published after Request construction', () => {
  // Arrange
  const frame = { _id: '3' };
  let liveFrames = [];
  const page = { frames: () => liveFrames };
  const request = new Request(page, requestMetadata());

  // Act
  liveFrames = [frame];
  const resolved = request.frame();
  liveFrames = [];

  // Assert
  assert.equal(resolved, frame);
  assert.equal(request.frame(), frame);
});

test('Request construction does not require an unselected Page Frame cache', () => {
  // Arrange
  let lookups = 0;
  const page = {
    _frameCache: null,
    frames() {
      lookups += 1;
      throw new UnsupportedOperationError('page.frames is not selected');
    },
  };

  // Act
  const request = new Request(page, requestMetadata());

  // Assert
  assert.equal(lookups, 0);
  assert.throws(() => request.frame(), UnsupportedOperationError);
  assert.equal(lookups, 1);
});
