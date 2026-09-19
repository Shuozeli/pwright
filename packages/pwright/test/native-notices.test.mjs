import assert from 'node:assert/strict';
import { test } from 'node:test';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installNativeNotices } from '../scripts/native-notices.mjs';

test('native packaging preserves both host license texts and upstream attribution', () => {
  // Arrange
  const workspace = fileURLToPath(new URL('../../../', import.meta.url));
  const destination = mkdtempSync(join(tmpdir(), 'pwright-native-notices-'));
  try {
    // Act
    installNativeNotices(workspace, destination);
    // Assert
    for (const name of ['NOTICE', 'LICENSE-MIT', 'LICENSE-APACHE-2.0']) {
      assert.deepEqual(readFileSync(join(destination, `pwright-node-host.${name}`)),
        readFileSync(join(workspace, 'crates/pwright-node-host', name)));
    }
    assert.match(readFileSync(join(destination, 'pwright-node-host.NOTICE'), 'utf8'), /Copyright \(c\) Microsoft Corporation/);
    assert.equal(existsSync(join(destination, 'pwright-file-input-fs.MIME-LICENSE')), false);
  } finally { rmSync(destination, { recursive: true }); }
});

test('selected upload packaging preserves MIME data and Playwright helper notices', () => {
  // Arrange
  const workspace = fileURLToPath(new URL('../../../', import.meta.url));
  const destination = mkdtempSync(join(tmpdir(), 'pwright-upload-notices-'));
  try {
    // Act
    installNativeNotices(workspace, destination, ['file-input']);
    // Assert
    for (const [crate, names] of [
      ['pwright-file-input-fs', ['MIME-LICENSE']],
      ['pwright-backend-cdp-file-input', ['NOTICE', 'LICENSE.playwright']],
    ]) {
      for (const name of names)
        assert.deepEqual(readFileSync(join(destination, `${crate}.${name}`)),
          readFileSync(join(workspace, 'crates', crate, name)));
    }
  } finally { rmSync(destination, { recursive: true }); }
});
