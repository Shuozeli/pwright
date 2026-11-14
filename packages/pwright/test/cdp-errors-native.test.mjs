import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { nativeArtifact, nativeFeatures, verifyNativeArtifact } = require('../src/native-artifact.cjs');

test('combined CDP error artifact proves both Rust error capabilities', async () => {
  // Arrange
  assert.equal(nativeArtifact.profile, 'official-conformance-page-cdp-errors');
  const features = nativeFeatures(nativeArtifact.profile);
  const addon = require(nativeArtifact.path);

  // Act
  verifyNativeArtifact(addon, nativeArtifact);
  const environment = new addon.NativeEnvironment(() => {}, () => 0);
  const pending = environment.connectOverCDPWithDefaultUserAgent(
    'not-a-url', 100, [], 'darwin', 'arm64', '25.0.0');

  // Assert
  assert.equal(features.includes('cdp-discovery-errors'), true);
  assert.equal(features.includes('cdp-ws-close-reason'), true);
  assert.equal(addon.cdpDiscoveryErrorsEnabled(), true);
  assert.equal(addon.cdpWsCloseReasonEnabled(), true);
  await assert.rejects(pending, /CDP endpoint must be an absolute HTTP\(S\) or WS\(S\) URL/);
});
