import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import api from '../src/index.cjs';

test('owned launch reports bounded output from an exiting synthetic executable', { timeout: 10_000 }, async () => {
  // Arrange: the only executable selected by the public API is this shell
  // script. It does not start or connect to any browser.
  assert.equal(api.compatibility.nativeProfile, 'owned-launch');
  const root = mkdtempSync(join(tmpdir(), 'pwright-native-launch-log-'));
  const profiles = join(root, 'profiles');
  const executablePath = join(root, 'exiting-synthetic-executable');
  const pidFile = join(root, 'synthetic.pid');
  const previousTmpdir = process.env.TMPDIR;
  const previousPidFile = process.env.PWRIGHT_SYNTHETIC_PID_FILE;
  let browser;
  try {
    mkdirSync(profiles);
    writeFileSync(executablePath, [
      '#!/bin/sh',
      'printf "%s\\n" "$$" > "$PWRIGHT_SYNTHETIC_PID_FILE"',
      'i=0',
      'while [ "$i" -lt 256 ]; do',
      '  printf 0123456789abcdef',
      '  i=$((i + 1))',
      'done',
      'printf "\\nPWRIGHT_SYNTHETIC_TAIL\\n"',
      'printf "PWRIGHT_SYNTHETIC_STDERR\\n" >&2',
      'exit 1',
      '',
    ].join('\n'));
    chmodSync(executablePath, 0o700);
    process.env.TMPDIR = profiles;
    process.env.PWRIGHT_SYNTHETIC_PID_FILE = pidFile;

    // Act: no tracesDir or inherited Playwright worker launch defaults.
    let launchError;
    try {
      browser = await api.chromium.launch({ executablePath, handleSIGINT: false, timeout: 2_000 });
    } catch (error) {
      launchError = error;
    }

    // Assert: the native process really ran, its bounded stdout/stderr
    // reached the public error, and the failed lease released its profile.
    assert.ok(launchError instanceof Error, 'synthetic executable must fail before CDP');
    assert.equal(launchError.code, 'PWRIGHT_BACKEND');
    assert.match(launchError.message, /Browser logs:/);
    assert.match(launchError.message, /\[line truncated\]/);
    assert.match(launchError.message, /PWRIGHT_SYNTHETIC_TAIL/);
    assert.match(launchError.message, /PWRIGHT_SYNTHETIC_STDERR/);
    assert.ok(Buffer.byteLength(launchError.message, 'utf8') <= 4_200);
    assert.deepEqual(readdirSync(profiles), []);
    const childPid = Number(readFileSync(pidFile, 'utf8').trim());
    assert.ok(Number.isSafeInteger(childPid) && childPid > 0);
    assert.throws(() => process.kill(childPid, 0), { code: 'ESRCH' });
  } finally {
    if (browser) await browser.close();
    if (previousTmpdir === undefined) delete process.env.TMPDIR;
    else process.env.TMPDIR = previousTmpdir;
    if (previousPidFile === undefined) delete process.env.PWRIGHT_SYNTHETIC_PID_FILE;
    else process.env.PWRIGHT_SYNTHETIC_PID_FILE = previousPidFile;
    rmSync(root, { recursive: true, force: true });
  }
});
