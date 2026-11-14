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
const profile = 'playwright-server-connect-close-error';

function fixture(selected, result) {
  const calls = [];
  const artifact = '/test-only/server-connect-close.node';
  let nativeLoads = 0;
  class NativeEnvironment {
    async connectPlaywrightServerCloseError(...args) {
      calls.push(args);
      if (result) throw result;
    }
  }
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return {
      existsSync(path) {
        assert.equal(path, artifact);
        return true;
      },
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

test('close-error profile has an exact marker and native port proof', () => {
  // Arrange
  class NativeEnvironment {
    connectPlaywrightServerCloseError() {}
  }
  const binary = {
    nativeAbiVersion: () => 1,
    NativeEnvironment,
    playwrightServerConnectCloseErrorEnabled: () => true,
  };

  // Act
  const verify = () => artifactConfig.verifyNativeArtifact(
    binary, artifactConfig.resolveNativeArtifact(profile));

  // Assert
  assert.deepEqual(artifactConfig.nativeFeatures(profile), [profile]);
  assert.equal(artifactConfig.nativeFeatures('default').includes(profile), false);
  assert.doesNotThrow(verify);
  assert.throws(() => artifactConfig.verifyNativeArtifact({ ...binary,
    playwrightServerConnectCloseErrorEnabled: () => false,
  }, artifactConfig.resolveNativeArtifact(profile)), /profile mismatch/);
  assert.throws(() => artifactConfig.verifyNativeArtifact(binary,
    artifactConfig.resolveNativeArtifact('default')), /profile mismatch/);
});

test('default profile still rejects connect before loading native code', async () => {
  // Arrange
  const { api, nativeLoads } = fixture(false);

  // Act
  const connection = api.chromium.connect('ws://example.test/ws');

  // Assert
  await assert.rejects(connection, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(nativeLoads(), 0);
});

test('close-error profile forwards endpoint, timeout, headers, and close reason', async () => {
  // Arrange
  const nativeError = new Error('PWRIGHT_BACKEND: Oh my! (WebSocket close code 4123)');
  const { api, calls } = fixture(true, nativeError);
  const endpoint = 'ws://example.test/ws?secret=MYSECRET';

  // Act
  const connection = api.chromium.connect(endpoint, {
    timeout: 2500,
    headers: { 'x-playwright-launch-options': '{}' },
    __testHookRedirectPortForwarding: undefined,
  });

  // Assert
  await assert.rejects(connection, error => {
    assert.match(error.message, /browserType\.connect: Oh my!/);
    assert.match(error.message, /4123/);
    assert.doesNotMatch(error.message, /MYSECRET/);
    return true;
  });
  assert.deepEqual(calls, [[endpoint, 2500, [['x-playwright-launch-options', '{}']]]]);
});

test('successful root initialize cannot produce a remote Browser wrapper', async () => {
  // Arrange
  const { api, calls } = fixture(true);

  // Act
  const connection = api.chromium.connect('ws://example.test/ws');

  // Assert
  await assert.rejects(connection, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.equal(calls.length, 1);
});
