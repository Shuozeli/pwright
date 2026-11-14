import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import * as api from '@shuozeli/pwright';

const source = new URL('../src/device-descriptors.json', import.meta.url);

test('device descriptors preserve the pinned Playwright v1.58.0 snapshot', () => {
  // Arrange
  const bytes = readFileSync(source);

  // Act
  const digest = createHash('sha256').update(bytes).digest('hex');

  // Assert
  assert.equal(digest, 'ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353');
});

test('devices expose the complete registry without loading the native runtime', () => {
  // Arrange
  const require = createRequire(import.meta.url);

  // Act
  const iphone = api.devices['iPhone 6'];

  // Assert
  assert.equal(Object.keys(api.devices).length, 143);
  assert.deepEqual(iphone.viewport, { width: 375, height: 667 });
  assert.equal(iphone.defaultBrowserType, 'webkit');
  assert.equal(Object.keys(require.cache).some(path => path.endsWith('/pwright.node')), false);
});

test('every device descriptor has Playwright-compatible public fields', () => {
  // Arrange
  const descriptors = Object.entries(api.devices);

  // Act
  const invalid = descriptors.filter(([, descriptor]) =>
    typeof descriptor.userAgent !== 'string' ||
    !Number.isInteger(descriptor.viewport?.width) ||
    !Number.isInteger(descriptor.viewport?.height) ||
    typeof descriptor.deviceScaleFactor !== 'number' ||
    typeof descriptor.isMobile !== 'boolean' ||
    typeof descriptor.hasTouch !== 'boolean' ||
    !['chromium', 'firefox', 'webkit'].includes(descriptor.defaultBrowserType));

  // Assert
  assert.deepEqual(invalid, []);
});
