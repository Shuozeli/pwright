import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, open, writeFile } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Supervisor belongs to test infrastructure, never the production Node package.
const output = process.env.PWRIGHT_SHARED_OUTPUT;
assert.ok(output && isAbsolute(output), 'Set PWRIGHT_SHARED_OUTPUT to an absolute new directory');
await mkdir(output); // Exclusive ownership; never overwrite another report.
const log = await open(join(output, 'runner.log'), 'wx');
const child = spawn(process.execPath, ['--import', 'tsx', 'src/run-node-ffi.ts', ...process.argv.slice(2)], {
  cwd: fileURLToPath(new URL('../', import.meta.url)),
  env: process.env, stdio: ['ignore', 'pipe', 'pipe'],
});
let diagnostics = '', logBytes = 0, overflow = false, timedOut = false;
let hardStop;
const deadline = setTimeout(() => {
  timedOut = true;
  child.kill('SIGTERM');
  hardStop = setTimeout(() => child.kill('SIGKILL'), 5000);
}, 600_000);
let writePending = Promise.resolve();
let logError;
function record(chunk, stream) {
  stream.write(chunk);
  logBytes += chunk.byteLength;
  if (logBytes > 2 * 1024 * 1024) { overflow = true; child.kill('SIGKILL'); return; }
  diagnostics += chunk.toString();
  writePending = writePending.then(() => log.write(chunk)).then(() => undefined, error => {
    logError = String(error);
    child.kill('SIGKILL');
  });
}
child.stdout.on('data', chunk => record(chunk, process.stdout));
child.stderr.on('data', chunk => record(chunk, process.stderr));
let status = null, signal = null, spawnError;
try { [status, signal] = await once(child, 'close'); }
catch (error) { spawnError = String(error); }
finally {
  clearTimeout(deadline);
  clearTimeout(hardStop);
  await writePending;
  await log.close();
}
const nativeFailure = /panicked at|fatal runtime error|Pwright environment cleanup failed/.test(diagnostics);
await writeFile(join(output, 'supervisor.json'), JSON.stringify({
  status, signal, spawnError, logError, overflow, timedOut, nativeFailure,
  passed: status === 0 && !overflow && !timedOut && !nativeFailure && !spawnError && !logError,
}, null, 2) + '\n', { flag: 'wx' });
process.exitCode = nativeFailure || overflow || timedOut || spawnError || logError || signal ? 2 : status ?? 2;
