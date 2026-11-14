import assert from 'node:assert/strict';
import {
  chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync,
  readdirSync, realpathSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, relative, resolve } from 'node:path';
import test from 'node:test';
import api from '../src/index.cjs';

test('trace profile prepares a relative caller directory before synthetic process startup', { timeout: 10_000 }, async () => {
  // Arrange
  assert.equal(api.compatibility.nativeProfile, 'playwright-trace');
  const root = mkdtempSync(join(tmpdir(), 'pwright-trace-dir-native-'));
  const profiles = join(root, 'profiles');
  const requested = join(root, 'caller', 'nested-traces');
  const relativeRequested = relative(process.cwd(), requested);
  const executablePath = join(root, 'exiting-synthetic-executable');
  const receipt = join(root, 'process-receipt');
  const previousTmpdir = process.env.TMPDIR;
  const previousReceipt = process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT;
  const previousDirectory = process.env.PWRIGHT_TRACE_SYNTHETIC_DIRECTORY;
  try {
    mkdirSync(profiles);
    writeFileSync(executablePath, [
      '#!/bin/sh',
      'if [ -d "$PWRIGHT_TRACE_SYNTHETIC_DIRECTORY" ]; then',
      '  printf "prepared\\n" > "$PWRIGHT_TRACE_SYNTHETIC_RECEIPT"',
      'else',
      '  printf "missing\\n" > "$PWRIGHT_TRACE_SYNTHETIC_RECEIPT"',
      'fi',
      'printf "PWRIGHT_TRACE_SYNTHETIC_EXIT\\n" >&2',
      'exit 1',
      '',
    ].join('\n'));
    chmodSync(executablePath, 0o700);
    process.env.TMPDIR = profiles;
    process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT = receipt;
    process.env.PWRIGHT_TRACE_SYNTHETIC_DIRECTORY = requested;

    // Act: this executable exits before any CDP attachment; no browser runs.
    const error = await api.chromium.launch({
      executablePath,
      handleSIGINT: false,
      timeout: 2_000,
      tracesDir: relativeRequested,
    }).catch(value => value);

    // Assert
    assert.ok(error instanceof Error);
    assert.equal(error.code, 'PWRIGHT_BACKEND');
    assert.match(error.message, /PWRIGHT_TRACE_SYNTHETIC_EXIT/);
    assert.equal(readFileSync(receipt, 'utf8'), 'prepared\n');
    assert.equal(realpathSync(resolve(process.cwd(), relativeRequested)), realpathSync(requested));
    assert.deepEqual(readdirSync(profiles), []);
    // The failed browser lease owns only its temporary profile. The caller's
    // trace directory remains available after the failed launch.
    assert.equal(existsSync(requested), true);
  } finally {
    if (previousTmpdir === undefined) delete process.env.TMPDIR;
    else process.env.TMPDIR = previousTmpdir;
    if (previousReceipt === undefined) delete process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT;
    else process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT = previousReceipt;
    if (previousDirectory === undefined) delete process.env.PWRIGHT_TRACE_SYNTHETIC_DIRECTORY;
    else process.env.PWRIGHT_TRACE_SYNTHETIC_DIRECTORY = previousDirectory;
    rmSync(root, { recursive: true, force: true });
  }
});

test('invalid trace-directory path fails before synthetic process admission', { timeout: 10_000 }, async () => {
  // Arrange
  assert.equal(api.compatibility.nativeProfile, 'playwright-trace');
  const root = mkdtempSync(join(tmpdir(), 'pwright-trace-dir-invalid-'));
  const profiles = join(root, 'profiles');
  const file = join(root, 'not-a-directory');
  const executablePath = join(root, 'synthetic-executable');
  const receipt = join(root, 'process-receipt');
  const previousTmpdir = process.env.TMPDIR;
  const previousReceipt = process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT;
  try {
    mkdirSync(profiles);
    writeFileSync(file, 'existing caller file');
    writeFileSync(executablePath, [
      '#!/bin/sh',
      'printf "started\\n" > "$PWRIGHT_TRACE_SYNTHETIC_RECEIPT"',
      'exit 1',
      '',
    ].join('\n'));
    chmodSync(executablePath, 0o700);
    process.env.TMPDIR = profiles;
    process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT = receipt;

    // Act
    const error = await api.chromium.launch({
      executablePath,
      tracesDir: join(file, 'nested'),
    }).catch(value => value);

    // Assert
    assert.ok(error instanceof Error);
    assert.equal(error.code, 'PWRIGHT_INVALID_ARGUMENT');
    assert.match(error.message, /tracesDir/);
    assert.equal(existsSync(receipt), false);
    assert.deepEqual(readdirSync(profiles), []);
    assert.equal(readFileSync(file, 'utf8'), 'existing caller file');
  } finally {
    if (previousTmpdir === undefined) delete process.env.TMPDIR;
    else process.env.TMPDIR = previousTmpdir;
    if (previousReceipt === undefined) delete process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT;
    else process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT = previousReceipt;
    rmSync(root, { recursive: true, force: true });
  }
});

test('trace directory is created before an invalid executable is inspected', { timeout: 10_000 }, async () => {
  // Arrange
  assert.equal(api.compatibility.nativeProfile, 'playwright-trace');
  const root = mkdtempSync(join(tmpdir(), 'pwright-trace-dir-executable-'));
  const requested = join(root, 'new-traces');
  try {
    // Act
    const error = await api.chromium.launch({
      executablePath: join(root, 'missing-executable'),
      tracesDir: requested,
    }).catch(value => value);

    // Assert
    assert.ok(error instanceof Error);
    assert.equal(error.code, 'PWRIGHT_INVALID_ARGUMENT');
    assert.match(error.message, /executablePath/);
    assert.equal(existsSync(requested), true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('empty tracesDir reaches Rust as no explicit caller directory', { timeout: 10_000 }, async () => {
  // Arrange
  assert.equal(api.compatibility.nativeProfile, 'playwright-trace');
  const root = mkdtempSync(join(tmpdir(), 'pwright-trace-dir-empty-'));
  const executablePath = join(root, 'exiting-synthetic-executable');
  const receipt = join(root, 'process-receipt');
  const previousReceipt = process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT;
  try {
    writeFileSync(executablePath, [
      '#!/bin/sh',
      'printf "started\\n" > "$PWRIGHT_TRACE_SYNTHETIC_RECEIPT"',
      'exit 1',
      '',
    ].join('\n'));
    chmodSync(executablePath, 0o700);
    process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT = receipt;

    // Act
    const error = await api.chromium.launch({
      executablePath,
      tracesDir: '',
      timeout: 2_000,
    }).catch(value => value);

    // Assert: an empty string follows upstream's falsey no-path policy, but
    // still enters the selected N-API method rather than the old addon port.
    assert.ok(error instanceof Error);
    assert.equal(error.code, 'PWRIGHT_BACKEND');
    assert.equal(readFileSync(receipt, 'utf8'), 'started\n');
  } finally {
    if (previousReceipt === undefined) delete process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT;
    else process.env.PWRIGHT_TRACE_SYNTHETIC_RECEIPT = previousReceipt;
    rmSync(root, { recursive: true, force: true });
  }
});
