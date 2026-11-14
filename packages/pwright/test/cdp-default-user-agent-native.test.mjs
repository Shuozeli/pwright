import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { nativeArtifact, nativeFeatures, verifyNativeArtifact } = require('../src/native-artifact.cjs');

function selectedEnvironment() {
  assert.ok(nativeFeatures(nativeArtifact.profile).includes('cdp-default-user-agent'));
  const addon = require(nativeArtifact.path);
  verifyNativeArtifact(addon, nativeArtifact);
  return new addon.NativeEnvironment(() => {}, () => 0);
}

const identity = ['darwin', 'arm64', '25.0.0'];

test('real Node addon routes no-options CDP attach through selected UA port', async () => {
  // Arrange
  const environment = selectedEnvironment();
  // Act
  const pending = environment.connectOverCDPWithDefaultUserAgent('not-a-url', 100, [], ...identity);
  // Assert
  await assert.rejects(pending, /CDP endpoint must be an absolute HTTP\(S\) or WS\(S\) URL/);
});

test('default UA consumes the bounded header count', async () => {
  // Arrange
  const environment = selectedEnvironment();
  const headers = Array.from({ length: 32 }, (_, index) => [`x-${index}`, 'v']);
  // Act
  const pending = environment.connectOverCDPWithDefaultUserAgent('not-a-url', 100, headers, ...identity);
  // Assert
  await assert.rejects(pending, /CDP handshake header count exceeds 32/);
});

test('explicit mixed-case empty UA preserves all 32 caller slots', async () => {
  // Arrange
  const environment = selectedEnvironment();
  const headers = [['uSeR-aGeNt', ''], ...Array.from({ length: 31 }, (_, index) => [`x-${index}`, 'v'])];
  // Act
  const pending = environment.connectOverCDPWithDefaultUserAgent('not-a-url', 100, headers, ...identity);
  // Assert
  await assert.rejects(pending, /CDP endpoint must be an absolute HTTP\(S\) or WS\(S\) URL/);
});

test('invalid caller header rejects before UA runtime observation', () => {
  // Arrange
  const environment = selectedEnvironment();
  const invalid = [['bad name', 'private-value']];
  // Act
  const call = () => environment.connectOverCDPWithDefaultUserAgent('not-a-url', 100, invalid, ...identity);
  // Assert
  assert.throws(call, error => /invalid CDP handshake header name/.test(error.message)
    && !error.message.includes('private-value'));
});
