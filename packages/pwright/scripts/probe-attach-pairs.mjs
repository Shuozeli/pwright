// Diagnostic ABBA prerequisite benchmark only; no addon or production timing gate.
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222', 'run the dedicated Chrome/Pwright preflight');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium } = requireOracle('playwright');
const observer = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
try {
  // The caller supplies the exact WebSocket verified during preflight; Rust
  // rejects credentials, non-loopback hosts, other ports and page endpoints.
  const options = {
    cwd: fileURLToPath(new URL('../../../', import.meta.url)),
    env: { ...process.env, CARGO_INCREMENTAL: '0' },
    timeout: 90000, maxBuffer: 128 * 1024, killSignal: 'SIGKILL',
  };
  try {
    const { stdout, stderr } = await promisify(execFile)('cargo', [
      'test', '--release', '--locked', '-p', 'pwright-node-host', '--test', 'attach_pair_probe',
      '--', '--ignored', '--nocapture',
    ], options);
    process.stdout.write(stdout);
    process.stderr.write(stderr);
  } catch (error) {
    if (typeof error.stdout === 'string') process.stdout.write(error.stdout);
    if (typeof error.stderr === 'string') process.stderr.write(error.stderr);
    throw error;
  }
} finally { await diagnosticDeadline(observer.close(), 'oracle cleanup'); }
