import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function facade(profile) {
  const calls = [];
  const session = {
    sessionSignal: () => ({ wait: () => new Promise(() => {}) }),
    isConnected: () => true,
  };
  class NativeEnvironment {
    async connectOverCDP(endpoint, timeout) {
      calls.push(['plain', endpoint, timeout]);
      return session;
    }
    async connectOverCDPWithHeaders(endpoint, timeout, headers) {
      calls.push(['headers', endpoint, timeout, headers]);
      return session;
    }
    async connectOverCDPWithDefaultUserAgent(...args) {
      calls.push(['default-ua', ...args]);
      return session;
    }
  }
  const artifact = '/test-only/cdp-default-user-agent.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === 'node:os') return {
      platform: () => 'darwin', arch: () => 'arm64', release: () => '25.0.0',
    };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile },
      nativeFeatures: () => profile === 'cdp-default-user-agent'
        || profile === 'official-conformance-page-cdp-default-user-agent'
        ? ['cdp-handshake-headers', 'cdp-default-user-agent']
        : ['cdp-handshake-headers'],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) return { NativeEnvironment };
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, module, module.exports);
  return { api: module.exports, calls };
}

test('default User-Agent profile has an independent native marker and port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = sourceRequire('./native-artifact.cjs');
  const profile = resolveNativeArtifact('cdp-default-user-agent');
  class NativeEnvironment {
    connectOverCDPWithHeaders() {}
    connectOverCDPWithDefaultUserAgent() {}
  }
  const binary = {
    nativeAbiVersion: () => 1,
    NativeEnvironment,
    NativeSession: class {},
    cdpHandshakeHeadersEnabled: () => true,
    cdpDefaultUserAgentEnabled: () => true,
  };

  // Act
  const features = nativeFeatures(profile.profile);

  // Assert
  assert.deepEqual(features, ['cdp-handshake-headers', 'cdp-default-user-agent']);
  assert.equal(nativeFeatures('default').includes('cdp-default-user-agent'), false);
  assert.deepEqual(nativeFeatures('official-conformance-page-cdp-default-user-agent').slice(-2), features);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, profile));
  assert.throws(() => verifyNativeArtifact({ ...binary, cdpDefaultUserAgentEnabled: () => false }, profile), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeEnvironment: class { connectOverCDPWithHeaders() {} } }, profile), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('cdp-handshake-headers')), /profile mismatch/);
});

test('selected default User-Agent profile routes no-options attach through Rust', async () => {
  // Arrange
  const { api, calls } = facade('cdp-default-user-agent');
  // Act
  await api.chromium.connectOverCDP('ws://example.invalid/ws');
  // Assert
  assert.deepEqual(calls, [[
    'default-ua', 'ws://example.invalid/ws', 30_000, [], 'darwin', 'arm64', '25.0.0',
  ]]);
});

test('combined Page and default User-Agent profile keeps the selected Rust port', async () => {
  // Arrange
  const { api, calls } = facade('official-conformance-page-cdp-default-user-agent');
  // Act
  await api.chromium.connectOverCDP('ws://example.invalid/ws');
  // Assert
  assert.deepEqual(calls, [[
    'default-ua', 'ws://example.invalid/ws', 30_000, [], 'darwin', 'arm64', '25.0.0',
  ]]);
});

test('selected default User-Agent profile forwards explicit headers as raw pairs', async () => {
  // Arrange
  const { api, calls } = facade('cdp-default-user-agent');
  // Act
  await api.chromium.connectOverCDP({
    wsEndpoint: 'ws://example.invalid/ws',
    headers: { 'uSeR-aGeNt': '', foo: 'bar' },
    timeout: 100,
  });
  // Assert
  assert.deepEqual(calls, [[
    'default-ua', 'ws://example.invalid/ws', 100,
    [['uSeR-aGeNt', ''], ['foo', 'bar']], 'darwin', 'arm64', '25.0.0',
  ]]);
});

test('existing handshake-only profile keeps its established native path', async () => {
  // Arrange
  const { api, calls } = facade('cdp-handshake-headers');
  // Act
  await api.chromium.connectOverCDP('ws://example.invalid/ws', { headers: { foo: 'bar' } });
  // Assert
  assert.deepEqual(calls, [['headers', 'ws://example.invalid/ws', 30_000, [['foo', 'bar']]]]);
});
