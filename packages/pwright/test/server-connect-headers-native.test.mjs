import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { arch, platform, release } from 'node:os';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { nativeArtifact, nativeFeatures, verifyNativeArtifact } = require('../src/native-artifact.cjs');
const profile = 'playwright-server-connect-headers';
const identity = ['chromium', [platform(), arch(), release()]];

function selectedEnvironment() {
  assert.equal(nativeArtifact.profile, profile);
  assert.deepEqual(nativeFeatures(profile), [profile]);
  const addon = require(nativeArtifact.path);
  verifyNativeArtifact(addon, nativeArtifact);
  return new addon.NativeEnvironment(() => {}, () => 0);
}

test('real addon rejects defaults that exceed the bounded WebSocket header count', () => {
  // Arrange
  const environment = selectedEnvironment();
  const headers = Array.from({ length: 62 }, (_, index) => [`x-${index}`, 'v']);

  // Act
  const connect = () => environment.connectPlaywrightServerHeaders(
    'ws://example.invalid/ws', 100, headers, ...identity);

  // Assert
  assert.throws(connect, /WebSocket header count exceeds 64|header count.*64/i);
});

test('real addon rejects a reserved handshake header before opening a socket', () => {
  // Arrange
  const environment = selectedEnvironment();
  const secret = 'private-value';

  // Act
  const connect = () => environment.connectPlaywrightServerHeaders(
    'ws://example.invalid/ws', 100, [['Sec-WebSocket-Key', secret]], ...identity);

  // Assert
  assert.throws(connect, error => /header/i.test(error.message)
    && !error.message.includes(secret));
});

test('real addon bounds the runtime identity tuple before opening a socket', () => {
  // Arrange
  const environment = selectedEnvironment();

  // Act
  const connect = () => environment.connectPlaywrightServerHeaders(
    'ws://example.invalid/ws', 100, [], 'chromium', ['darwin', 'arm64', 'x'.repeat(129)]);

  // Assert
  assert.throws(connect, /runtime identity exceeds its field byte limits/);
});

test('real addon requires exactly three runtime identity fields', () => {
  // Arrange
  const environment = selectedEnvironment();

  // Act
  const connect = () => environment.connectPlaywrightServerHeaders(
    'ws://example.invalid/ws', 100, [], 'chromium', ['darwin', 'arm64']);

  // Assert
  assert.throws(connect, /runtime identity must contain platform, architecture, and release/);
});

test('real addon requires string runtime identity fields', () => {
  // Arrange
  const environment = selectedEnvironment();

  // Act
  const connect = () => environment.connectPlaywrightServerHeaders(
    'ws://example.invalid/ws', 100, [], 'chromium', ['darwin', 1, '25.6.0']);

  // Assert
  assert.throws(connect, /runtime identity fields must be strings/);
});
