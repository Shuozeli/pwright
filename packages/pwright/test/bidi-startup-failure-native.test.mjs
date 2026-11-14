import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const api = require('../src/index.cjs');

function fixture() {
  const directory = mkdtempSync(join(tmpdir(), 'pwright-bidi-startup-native-'));
  const executable = join(directory, 'exit-seven.sh');
  writeFileSync(executable, `#!/bin/sh
case "$1" in
  --user-data-dir=*) profile="\${1#--user-data-dir=}" ;;
  --profile) profile="$2" ;;
  *) exit 9 ;;
esac
if [ ! -d "$profile" ]; then exit 10; fi
printf 'profile=%s\\n' "$profile" >&2
printf 'owned startup output\\n'
exit 7
`);
  chmodSync(executable, 0o700);
  return { directory, executable, tracesDir: join(directory, 'traces') };
}

for (const [engine, channel] of [
  ['chromium', 'bidi-chromium'],
  ['firefox', 'moz-firefox'],
]) {
  test(`native ${engine} failure-only startup observes the real child and removes its profile`, async () => {
    // Arrange
    const { directory, executable, tracesDir } = fixture();

    try {
      // Act
      const launching = api[engine].launch({
        executablePath: executable,
        channel,
        firefoxUserPrefs: {},
        tracesDir,
        handleSIGINT: false,
      });

      // Assert
      await assert.rejects(launching, error => {
        assert.match(error.message, /Browser logs:/);
        assert.match(error.message, /owned startup output/);
        assert.match(error.message, /exit status: 7|exit code 7|exit 7/);
        const profile = /profile=([^\s]+)/.exec(error.message)?.[1];
        assert.ok(profile, 'the explicit child must observe a live profile');
        assert.equal(existsSync(profile), false, 'the Rust-owned profile must be removed');
        return true;
      });
      assert.equal(existsSync(tracesDir), true);
    } finally {
      rmSync(directory, { recursive: true });
    }
  });
}

test('native failure-only route prepares traces before rejecting a missing executable', async () => {
  // Arrange
  const directory = mkdtempSync(join(tmpdir(), 'pwright-bidi-startup-native-'));
  const tracesDir = join(directory, 'traces');
  const missing = join(directory, 'missing-browser');

  try {
    // Act
    const launching = api.firefox.launch({
      executablePath: missing,
      channel: 'moz-firefox-beta',
      tracesDir,
    });

    // Assert
    await assert.rejects(launching, error => {
      assert.doesNotMatch(error.message, /Browser logs:/);
      assert.match(error.message, /executablePath/);
      return true;
    });
    assert.equal(existsSync(tracesDir), true);
  } finally {
    rmSync(directory, { recursive: true });
  }
});
