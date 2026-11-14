import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';
import { createCaseWatchdog } from './case-watchdog.mjs';

// Test infrastructure only: the production package starts no child process.
// A background Rust panic must fail the gate even if Node assertions passed.
const lane = process.argv[2] ?? 'differential';
const navigationCloseLane = lane === 'frame-navigation-close' || lane === 'file-input' || lane === 'handle-file-input' || lane === 'handle-file-input-binding' || lane === 'handle-owner-frame' || lane === 'handles' || lane === 'handle-module' || lane === 'handle-producers' || lane === 'handle-acquisition' || lane === 'handle-properties' || lane === 'native-class-module';
if (!navigationCloseLane && !['differential', 'evaluation', 'locator', 'locator-evaluation', 'actions', 'typing', 'mouse', 'mouse-public', 'frames-public', 'frames-lookup', 'frame-navigation', 'history-navigation', 'frame-navigation-workers', 'pages-public', 'page-is-closed', 'browser-connection', 'collection-publication', 'collection-lifecycle', 'collection-creation', 'lifecycle-events', 'lifecycle-listeners', 'driver-close-attribution', 'native-close-confirmation', 'terminal-dispatch', 'context-base', 'context-input-boundary', 'extra-headers', 'header-child-sessions', 'attachment-retirement', 'detach-retirement', 'queued-context-retirement', 'navigation-peer-disconnect', 'glob-ffi', 'screenshot', 'reload'].includes(lane)) throw new Error(`Unknown differential lane: ${lane}`);
// TAP emits each failure's details immediately. The default spec reporter can
// defer them until process exit, losing the original error on a deadline kill.
const args = lane === 'mouse-public' ? ['scripts/compare-mouse.mjs']
  : lane === 'frames-public' ? ['--expose-gc', 'scripts/compare-frame-tree.mjs']
  : lane === 'frames-lookup' ? ['--expose-gc', 'scripts/compare-frame-lookup.mjs']
  : lane === 'pages-public' ? ['--expose-gc', 'scripts/compare-page-enumeration.mjs']
  : lane === 'context-base' ? ['--expose-gc', 'scripts/compare-context-base.mjs']
  : lane === 'glob-ffi' ? ['--expose-gc', 'scripts/compare-glob-ffi.mjs']
  : ['--test-reporter=tap', ...(['locator', 'handle-file-input-binding'].includes(lane) ? ['--expose-gc'] : []),
    `test/${lane}.test.mjs`];
const child = spawn(process.execPath, args, {
  cwd: fileURLToPath(new URL('../', import.meta.url)),
  env: process.env, stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
});
let diagnostics = '';
let overflow = false;
let outputBytes = 0;
child.stdout.on('data', chunk => {
  outputBytes += chunk.length;
  if (outputBytes > 4 * 1024 * 1024) { overflow = true; child.kill('SIGKILL'); }
  else process.stdout.write(chunk);
});
child.stderr.on('data', chunk => {
  process.stderr.write(chunk);
  if (diagnostics.length + chunk.length > 1024 * 1024) { overflow = true; child.kill('SIGKILL'); }
  else diagnostics += chunk.toString();
});
// A complete lifecycle corpus can legitimately take over 180 s while every
// API stays within its original deadline. Keep an independent 60 s case bound
// and 600 s total; other lanes retain their existing 180 s supervisor budget.
const watchdog = lane === 'differential' ? createCaseWatchdog({
  schedule: setTimeout, cancel: clearTimeout,
  fail: reason => { process.stderr.write(`${reason}\n`); overflow = true; child.kill('SIGKILL'); },
}) : undefined;
if (watchdog) child.on('message', value => watchdog.message(value));
const deadline = setTimeout(() => child.kill('SIGKILL'), lane === 'differential' ? 600_000 : 180_000);
let status, signal;
try { [status, signal] = await once(child, 'close'); }
finally { clearTimeout(deadline); watchdog?.close(); }
if (status !== 0 || overflow || /panicked at|fatal runtime error|Pwright environment cleanup failed/.test(diagnostics))
  throw new Error(`Native differential lane failed (exit ${status}, signal ${signal})`);
