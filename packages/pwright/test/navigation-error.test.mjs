import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';

const createNavigationError = createRequire(import.meta.url)('../src/navigation-error.cjs');

class TimeoutError extends Error {}

function fixture() {
  return createNavigationError({
    TimeoutError,
    mapError(error, api) {
      const mapped = error.timeout
        ? new TimeoutError(`${api}: Timeout 5000ms exceeded.`)
        : new Error(`${api}: ${error.message}`);
      mapped.code = error.code;
      return mapped;
    },
  });
}

test('navigation timeout adds the resolved URL and lifecycle call log', () => {
  // Arrange
  const mapNavigationError = fixture();

  // Act
  const error = mapNavigationError(
    { timeout: true, code: 'PWRIGHT_TIMEOUT' },
    'page.goto',
    'https://example.test/next',
    'networkidle',
  );

  // Assert
  assert.equal(error.message, 'page.goto: Timeout 5000ms exceeded.\nCall log:\n  - navigating to "https://example.test/next", waiting until "networkidle"');
});

test('HTTPS navigation backend error retains its diagnostic and adds the URL', () => {
  // Arrange
  const mapNavigationError = fixture();

  // Act
  const error = mapNavigationError(
    { message: 'Backend: document navigation failed: net::ERR_CERT_AUTHORITY_INVALID', code: 'PWRIGHT_BACKEND' },
    'page.goto',
    'https://example.test/redirect/1.html',
    'load',
  );

  // Assert
  assert.equal(error.code, 'PWRIGHT_BACKEND');
  assert.equal(error.message, 'page.goto: Backend: document navigation failed: net::ERR_CERT_AUTHORITY_INVALID\nCall log:\n  - navigating to "https://example.test/redirect/1.html", waiting until "load"');
});

test('conflicting referer backend error includes the attempted navigation URL', () => {
  // Arrange
  const mapNavigationError = fixture();

  // Act
  const error = mapNavigationError(
    { message: '"referer" is already specified as extra HTTP header', code: 'PWRIGHT_BACKEND' },
    'page.goto',
    'http://example.test/grid.html',
    'load',
  );

  // Assert
  assert.equal(error.code, 'PWRIGHT_BACKEND');
  assert.match(error.message, /^page\.goto: "referer" is already specified as extra HTTP header/);
  assert.match(error.message, /http:\/\/example\.test\/grid\.html/);
});

test('invalid URL backend error keeps the original user input in the call log', () => {
  // Arrange
  const mapNavigationError = fixture();

  // Act
  const error = mapNavigationError(
    { message: 'Backend: document navigation failed: net::ERR_NAME_NOT_RESOLVED', code: 'PWRIGHT_BACKEND' },
    'page.goto',
    'https://www.youtube Panel Title.com/',
    'load',
  );

  // Assert
  assert.equal(error.code, 'PWRIGHT_BACKEND');
  assert.match(error.toString(), /Panel Title/);
});
