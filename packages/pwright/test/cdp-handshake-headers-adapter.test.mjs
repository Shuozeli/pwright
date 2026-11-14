import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const sourceRequire = createRequire(indexUrl);

function facade(selected) {
  const calls = [];
  let nativeLoads = 0;
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
  }
  const artifact = '/test-only/cdp-handshake-headers.node';
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: () => true };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: selected ? 'cdp-handshake-headers' : 'default' },
      nativeFeatures: () => selected ? ['cdp-handshake-headers'] : [],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) {
      nativeLoads += 1;
      return { NativeEnvironment };
    }
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports'], {
    filename: fileURLToPath(indexUrl),
  })(require, module, module.exports);
  return { api: module.exports, calls, nativeLoads: () => nativeLoads };
}

test('CDP handshake header profile proves its marker and distinct native port', () => {
  // Arrange
  const { nativeFeatures, resolveNativeArtifact, verifyNativeArtifact } = sourceRequire('./native-artifact.cjs');
  const profile = resolveNativeArtifact('cdp-handshake-headers');
  class NativeEnvironment {
    connectOverCDPWithHeaders() {}
  }
  const binary = {
    nativeAbiVersion: () => 1,
    NativeEnvironment,
    NativeSession: class {},
    cdpHandshakeHeadersEnabled: () => true,
  };

  // Act
  const enabled = nativeFeatures(profile.profile);

  // Assert
  assert.deepEqual(enabled, ['cdp-handshake-headers']);
  assert.equal(nativeFeatures('default').includes('cdp-handshake-headers'), false);
  assert.doesNotThrow(() => verifyNativeArtifact(binary, profile));
  assert.throws(() => verifyNativeArtifact({ ...binary, cdpHandshakeHeadersEnabled: () => false }, profile), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact({ ...binary, NativeEnvironment: class {} }, profile), /profile mismatch/);
  assert.throws(() => verifyNativeArtifact(binary, resolveNativeArtifact('default')), /profile mismatch/);
});

test('unselected CDP handshake headers fail before loading native code', async () => {
  // Arrange
  const { api, calls, nativeLoads } = facade(false);

  // Act
  const pending = api.chromium.connectOverCDP('ws://example.invalid/ws', { headers: { foo: 'bar' } });

  // Assert
  await assert.rejects(pending, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(nativeLoads(), 0);
  assert.deepEqual(calls, []);
});

test('selected CDP handshake headers preserve explicit WebSocket endpoint and pairs', async () => {
  // Arrange
  const { api, calls } = facade(true);
  const endpoint = 'ws://example.invalid/ws';

  // Act
  const browser = await api.chromium.connectOverCDP({
    wsEndpoint: endpoint,
    headers: { 'User-Agent': 'Playwright', foo: 'bar' },
    timeout: 100,
  });

  // Assert
  assert.equal(browser.browserType(), api.chromium);
  assert.deepEqual(calls, [['headers', endpoint, 100, [['User-Agent', 'Playwright'], ['foo', 'bar']]]]);
});

test('selected CDP handshake headers pass unchanged HTTP discovery URL and pairs', async () => {
  // Arrange
  const { api, calls } = facade(true);
  const endpoint = 'http://example.invalid/browser/?foo=bar';

  // Act
  await api.chromium.connectOverCDP(endpoint, { headers: { 'User-Agent': 'Playwright', foo: 'bar' } });

  // Assert
  assert.deepEqual(calls, [['headers', endpoint, 30_000, [['User-Agent', 'Playwright'], ['foo', 'bar']]]]);
});

test('deprecated wsEndpoint without headers retains the existing two-argument native path', async () => {
  // Arrange
  const { api, calls } = facade(false);

  // Act
  await api.chromium.connectOverCDP({ wsEndpoint: 'ws://example.invalid/ws' });

  // Assert
  assert.deepEqual(calls, [['plain', 'ws://example.invalid/ws', 30_000]]);
});

test('CDP handshake header count is bounded before native access', async () => {
  // Arrange
  const { api, calls, nativeLoads } = facade(true);
  const headers = Object.fromEntries(Array.from({ length: 33 }, (_, index) => [`x-${index}`, 'v']));

  // Act
  const pending = api.chromium.connectOverCDP('ws://example.invalid/ws', { headers });

  // Assert
  await assert.rejects(pending, /count exceeds 32/);
  assert.equal(nativeLoads(), 0);
  assert.deepEqual(calls, []);
});

test('CDP handshake header values must be strings before native access', async () => {
  // Arrange
  const { api, calls, nativeLoads } = facade(true);

  // Act
  const pending = api.chromium.connectOverCDP('ws://example.invalid/ws', { headers: { foo: 42 } });

  // Assert
  await assert.rejects(pending, TypeError);
  assert.equal(nativeLoads(), 0);
  assert.deepEqual(calls, []);
});

test('CDP handshake header name length is bounded before native access', async () => {
  // Arrange
  const { api, calls, nativeLoads } = facade(true);
  const headers = { ['x'.repeat(257)]: 'v' };

  // Act
  const pending = api.chromium.connectOverCDP('ws://example.invalid/ws', { headers });

  // Assert
  await assert.rejects(pending, /name exceeds 256 bytes/);
  assert.equal(nativeLoads(), 0);
  assert.deepEqual(calls, []);
});

test('CDP handshake header value length is bounded without leaking its content', async () => {
  // Arrange
  const { api, calls, nativeLoads } = facade(true);
  const secret = 'super-secret-' + 'x'.repeat(4097);

  // Act
  const pending = api.chromium.connectOverCDP('ws://example.invalid/ws', { headers: { authorization: secret } });

  // Assert
  await assert.rejects(pending, error => /value exceeds 4096 bytes/.test(error.message) && !error.message.includes(secret));
  assert.equal(nativeLoads(), 0);
  assert.deepEqual(calls, []);
});

test('CDP handshake header aggregate bytes are bounded before native access', async () => {
  // Arrange
  const { api, calls, nativeLoads } = facade(true);
  const headers = { a: 'x'.repeat(4096), b: 'y'.repeat(4096) };

  // Act
  const pending = api.chromium.connectOverCDP('ws://example.invalid/ws', { headers });

  // Assert
  await assert.rejects(pending, /headers exceed 8192 bytes/);
  assert.equal(nativeLoads(), 0);
  assert.deepEqual(calls, []);
});
