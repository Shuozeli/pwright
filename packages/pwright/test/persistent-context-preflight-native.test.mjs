import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { nativeArtifact, verifyNativeArtifact } = require('../src/native-artifact.cjs');
const selected = nativeArtifact.profile === 'persistent-context-preflight';

test('opt-in release addon exports only the selected pure preflight ports', { skip: !selected }, () => {
  // Arrange
  const binary = require(nativeArtifact.path);

  // Act
  verifyNativeArtifact(binary, nativeArtifact);

  // Assert
  assert.equal(binary.persistentContextPreflightEnabled(), true);
  assert.equal(binary.ownedLaunchEnabled(), false);
  assert.equal(binary.browserRegistryEnabled(), false);
  assert.equal(binary.persistentContextCertificateError([]), null);
  assert.equal(binary.persistentContextArgumentError('chromium', [], undefined), null);
});

test('real Rust decision preserves certificate-first public error without browser acquisition', { skip: !selected }, async () => {
  // Arrange
  const api = require('../src/index.cjs');
  const options = {
    clientCertificates: [{ origin: '', cert: Buffer.alloc(0), key: Buffer.alloc(0) }],
    args: ['https://example.invalid/'],
  };

  // Act
  const result = api.chromium.launchPersistentContext('unused-profile', options);

  // Assert
  await assert.rejects(result, {
    name: 'Error',
    message: 'browserType.launchPersistentContext: clientCertificates.origin is required',
  });
});

test('real Rust decision accepts boxed strings and ignores non-string argument slots', { skip: !selected }, async () => {
  // Arrange
  const api = require('../src/index.cjs');
  const binary = require(nativeArtifact.path);

  // Act
  const result = api.firefox.launchPersistentContext('unused-profile', {
    channel: new String('moz-firefox'),
    args: [42, new String('https://example.invalid/')],
  });

  // Assert
  assert.equal(binary.persistentContextArgumentError('firefox', [null, 'https://example.invalid/'], 'moz-firefox'),
    'Arguments can not specify page to be opened');
  await assert.rejects(result, {
    name: 'Error',
    message: 'browserType.launchPersistentContext: Arguments can not specify page to be opened',
  });
});

test('valid pure preflight never claims persistent browser support', { skip: !selected }, async () => {
  // Arrange
  const api = require('../src/index.cjs');

  // Act
  const result = api.webkit.launchPersistentContext('unused-profile', { args: ['--safe-argument'] });

  // Assert
  await assert.rejects(result, { code: 'PWRIGHT_UNSUPPORTED' });
});
