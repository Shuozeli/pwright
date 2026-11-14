import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = require('../src/native-artifact.cjs');

function markerBinary(discovery, closeReason) {
  return {
    nativeAbiVersion: () => 1,
    NativeEnvironment: class {},
    NativeSession: class {},
    cdpDiscoveryErrorsEnabled: () => discovery,
    cdpWsCloseReasonEnabled: () => closeReason,
  };
}

test('CDP error leaves and combined Page profile keep exact opt-in feature sets', () => {
  // Arrange
  const pageWithUa = nativeFeatures('official-conformance-page-cdp-default-user-agent');

  // Act
  const discovery = nativeFeatures('cdp-discovery-errors');
  const closeReason = nativeFeatures('cdp-ws-close-reason');
  const combined = nativeFeatures('official-conformance-page-cdp-errors');

  // Assert
  assert.deepEqual(discovery, ['cdp-discovery-errors']);
  assert.deepEqual(closeReason, ['cdp-ws-close-reason']);
  assert.deepEqual(combined, [...pageWithUa, 'cdp-discovery-errors', 'cdp-ws-close-reason']);
  assert.equal(nativeFeatures('default').includes('cdp-discovery-errors'), false);
  assert.equal(nativeFeatures('default').includes('cdp-ws-close-reason'), false);
  assert.equal(pageWithUa.includes('cdp-discovery-errors'), false);
  assert.equal(pageWithUa.includes('cdp-ws-close-reason'), false);
  assert.notEqual(resolveNativeArtifact('cdp-discovery-errors').path, resolveNativeArtifact('cdp-ws-close-reason').path);
});

test('CDP discovery error leaf requires its own compiled marker', () => {
  // Arrange
  const selected = resolveNativeArtifact('cdp-discovery-errors');
  const binary = markerBinary(true, false);

  // Act
  const verify = () => verifyNativeArtifact(binary, selected);

  // Assert
  assert.doesNotThrow(verify);
  assert.throws(() => verifyNativeArtifact(markerBinary(false, false), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('CDP WebSocket close-reason leaf requires its own compiled marker', () => {
  // Arrange
  const selected = resolveNativeArtifact('cdp-ws-close-reason');
  const binary = markerBinary(false, true);

  // Act
  const verify = () => verifyNativeArtifact(binary, selected);

  // Assert
  assert.doesNotThrow(verify);
  assert.throws(() => verifyNativeArtifact(markerBinary(false, false), selected), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});
