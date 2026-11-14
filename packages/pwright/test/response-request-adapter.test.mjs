import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
class UnsupportedOperationError extends Error {}
const pageRoutes = require('../src/page-routes.cjs')({
  mapError: error => error,
  validateOptions: () => {},
  UnsupportedOperationError,
});
const responseRequests = require('../src/response-request.cjs')({
  Request: pageRoutes.Request,
  UnsupportedOperationError,
});

function requestMetadata() {
  return {
    url: 'https://example.test/document',
    method: 'POST',
    resourceType: 'document',
    headers: [
      { name: 'Referer', value: 'https://example.test/source' },
      { name: 'Content-Type', value: 'text/plain' },
    ],
    postData: Buffer.from('body'),
    postDataJsonAvailable: true,
    postDataJson: { body: true },
    postDataJsonError: null,
    isNavigationRequest: true,
    frameId: '3',
  };
}

test('Response request wraps the Rust navigation snapshot without reconstructing headers', () => {
  // Arrange
  const frame = { _id: '3' };
  const page = { frames: () => [frame] };
  const metadata = requestMetadata();

  // Act
  const request = responseRequests.get(responseRequests.create(page, metadata));

  // Assert
  assert.equal(request.url(), metadata.url);
  assert.equal(request.method(), 'POST');
  assert.equal(request.resourceType(), 'document');
  assert.deepEqual(request.headers(), {
    referer: 'https://example.test/source',
    'content-type': 'text/plain',
  });
  assert.deepEqual(request.headersArray(), metadata.headers);
  assert.deepEqual(request.postDataBuffer(), Buffer.from('body'));
  assert.equal(request.postData(), 'body');
  assert.deepEqual(request.postDataJSON(), { body: true });
  assert.equal(request.isNavigationRequest(), true);
  assert.equal(request.frame(), frame);
});

test('Request postDataJSON preserves the Rust parser error', () => {
  // Arrange
  const page = { frames: () => [] };
  const metadata = requestMetadata();
  metadata.postDataJson = null;
  metadata.postDataJsonError = 'POST data is not a valid JSON object: <not a json>';
  const request = responseRequests.get(responseRequests.create(page, metadata));

  // Act
  const read = () => request.postDataJSON();

  // Assert
  assert.throws(read, /POST data is not a valid JSON object: <not a json>/);
});

test('Request postDataJSON preserves parser-selected null without guessing from omitted values', () => {
  // Arrange
  const page = { frames: () => [] };
  const metadata = requestMetadata();
  metadata.postData = null;
  metadata.postDataJson = null;
  metadata.postDataJsonError = null;
  const request = responseRequests.get(responseRequests.create(page, metadata));

  // Act
  const value = request.postDataJSON();

  // Assert
  assert.equal(value, null);
});

test('Request postDataJSON reports an unselected parser explicitly', () => {
  // Arrange
  const page = { frames: () => [] };
  const metadata = requestMetadata();
  delete metadata.postDataJsonAvailable;
  delete metadata.postDataJson;
  delete metadata.postDataJsonError;
  const request = responseRequests.get(responseRequests.create(page, metadata));

  // Act
  const read = () => request.postDataJSON();

  // Assert
  assert.throws(read, UnsupportedOperationError);
});

test('Response request reports an unselected native capability explicitly', () => {
  // Arrange
  const missing = responseRequests.create({ frames: () => [] }, undefined);

  // Act
  const read = () => responseRequests.get(missing);

  // Assert
  assert.throws(read, UnsupportedOperationError);
});
