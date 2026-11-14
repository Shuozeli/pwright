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
const artifact = '/test-only/persistent-context-preflight.node';

function fixture() {
  const calls = [];
  let nativeLoads = 0;
  const binary = {
    persistentContextCertificateError(certificates) {
      calls.push(['certificates', certificates]);
      return certificates.some(certificate => !certificate.origin)
        ? 'clientCertificates.origin is required' : null;
    },
    persistentContextArgumentError(engine, args, channel) {
      calls.push(['arguments', engine, args, channel]);
      return args.some(arg => typeof arg === 'string' && !arg.startsWith('-'))
        ? 'Arguments can not specify page to be opened' : null;
    },
  };
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === 'node:fs') return {
      existsSync(path) {
        assert.equal(path, artifact);
        return true;
      },
    };
    if (specifier === './native-artifact.cjs') return {
      nativeArtifact: { path: artifact, profile: 'persistent-context-preflight' },
      nativeFeatures: () => ['persistent-context-preflight'],
      verifyNativeArtifact() {},
    };
    if (specifier === artifact) {
      nativeLoads += 1;
      return binary;
    }
    return sourceRequire(specifier);
  }
  compileFunction(indexSource, ['require', 'module', 'exports', '__dirname'], {
    filename: indexPath,
  })(require, module, module.exports, dirname(indexPath));
  return { api: module.exports, calls, nativeLoads: () => nativeLoads };
}

test('native certificate rejection precedes argument conversion and preserves Buffer truthiness', async () => {
  // Arrange
  const { api, calls, nativeLoads } = fixture();
  const certificate = { origin: '', cert: Buffer.alloc(0), key: Buffer.alloc(0) };

  // Act
  const result = api.chromium.launchPersistentContext('unused-profile', {
    clientCertificates: [certificate],
    args: ['https://example.invalid/'],
  });

  // Assert
  await assert.rejects(result, {
    name: 'Error',
    message: 'browserType.launchPersistentContext: clientCertificates.origin is required',
  });
  assert.deepEqual(calls, [['certificates', [{
    origin: false,
    cert: true,
    key: true,
    pfx: false,
    passphrase: false,
  }]]]);
  assert.equal(nativeLoads(), 1);
});

test('native argument decision receives boxed strings and inert non-string slots', async () => {
  // Arrange
  const { api, calls, nativeLoads } = fixture();

  // Act
  const result = api.firefox.launchPersistentContext('unused-profile', {
    channel: new String('moz-firefox'),
    args: [new String('https://example.invalid/'), 42],
  });

  // Assert
  await assert.rejects(result, {
    name: 'Error',
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  });
  assert.deepEqual(calls, [['arguments', 'firefox', ['https://example.invalid/', null], 'moz-firefox']]);
  assert.equal(nativeLoads(), 1);
});

test('ignored default arguments never load the pure native decision profile', async () => {
  // Arrange
  const { api, calls, nativeLoads } = fixture();

  // Act
  const result = api.webkit.launchPersistentContext('unused-profile', {
    ignoreDefaultArgs: true,
    args: ['https://example.invalid/'],
  });

  // Assert
  await assert.rejects(result, { code: 'PWRIGHT_UNSUPPORTED' });
  assert.deepEqual(calls, []);
  assert.equal(nativeLoads(), 0);
});
