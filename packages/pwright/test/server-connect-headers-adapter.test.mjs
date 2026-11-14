import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexSource = readFileSync(indexUrl, 'utf8');
const indexPath = fileURLToPath(indexUrl);
const sourceRequire = createRequire(indexUrl);
const artifactConfig = sourceRequire('./native-artifact.cjs');
const profile = 'playwright-server-connect-headers';

function fixture(selected, result) {
  const calls = [];
  const artifact = '/test-only/server-connect-headers.node';
  let nativeLoads = 0;
  class NativeEnvironment {
    async connectPlaywrightServerHeaders(...args) {
      calls.push(args);
      if (result) throw result;
    }
  }
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return { existsSync: path => path === artifact };
    if (specifier === 'node:os') return {
      platform: () => 'darwin', arch: () => 'arm64', release: () => '25.0.0',
    };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: selected ? profile : 'default' },
      nativeFeatures: name => name === profile ? [profile] : [],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) {
      nativeLoads += 1;
      return { NativeEnvironment };
    }
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports', '__dirname'], {
    filename: indexPath,
  })(require, module, module.exports, dirname(indexPath));
  return { api: module.exports, calls, nativeLoads: () => nativeLoads };
}

test('headers profile has an exact marker and native port proof', () => {
  // Arrange
  class NativeEnvironment {
    connectPlaywrightServerHeaders() {}
  }
  const binary = {
    nativeAbiVersion: () => 1,
    NativeEnvironment,
    playwrightServerConnectHeadersEnabled: () => true,
    playwrightServerConnectHeadersAbiShape: () => 2,
  };

  // Act
  const verify = () => artifactConfig.verifyNativeArtifact(
    binary, artifactConfig.resolveNativeArtifact(profile));

  // Assert
  assert.deepEqual(artifactConfig.nativeFeatures(profile), [profile]);
  assert.equal(artifactConfig.nativeFeatures('default').includes(profile), false);
  assert.doesNotThrow(verify);
  assert.throws(() => artifactConfig.verifyNativeArtifact({ ...binary,
    playwrightServerConnectHeadersEnabled: () => false,
  }, artifactConfig.resolveNativeArtifact(profile)), /profile mismatch/);
  assert.throws(() => artifactConfig.verifyNativeArtifact({ ...binary,
    playwrightServerConnectHeadersAbiShape: undefined,
  }, artifactConfig.resolveNativeArtifact(profile)), /profile mismatch/);
  assert.throws(() => artifactConfig.verifyNativeArtifact(binary,
    artifactConfig.resolveNativeArtifact('default')), /profile mismatch/);
});

test('default profile still rejects connect without loading native code', async () => {
  // Arrange
  const { api, nativeLoads } = fixture(false);

  // Act
  const connection = api.chromium.connect('ws://example.test/ws');

  // Assert
  await assert.rejects(connection, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(nativeLoads(), 0);
});

test('selected profile passes only runtime observations and raw headers to Rust', async () => {
  // Arrange
  const nativeError = new Error('PWRIGHT_UNSUPPORTED: remote Playwright Browser sessions are not implemented');
  const { api, calls } = fixture(true, nativeError);
  const endpoint = 'ws://example.test/ws?secret=MYSECRET';

  // Act
  const connection = api.chromium.connect(endpoint, {
    timeout: 100,
    headers: { 'User-Agent': 'Playwright', foo: 'bar' },
    __testHookRedirectPortForwarding: undefined,
  });

  // Assert
  await assert.rejects(connection, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, [[
    endpoint, 100, [['User-Agent', 'Playwright'], ['foo', 'bar']],
    'chromium', ['darwin', 'arm64', '25.0.0'],
  ]]);
});

test('selected profile leaves absent User-Agent for Rust to synthesize', async () => {
  // Arrange
  const { api, calls } = fixture(true);

  // Act
  const connection = api.firefox.connect('ws://example.test/ws', {
    headers: { foo: 'bar' },
  });

  // Assert
  await assert.rejects(connection, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, [[
    'ws://example.test/ws', 30_000, [['foo', 'bar']],
    'firefox', ['darwin', 'arm64', '25.0.0'],
  ]]);
});
