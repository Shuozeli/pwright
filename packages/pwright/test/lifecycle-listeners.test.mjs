import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const referenceOnly = process.env.PWRIGHT_LISTENER_REFERENCE === '1';
assert.ok(process.env.PWRIGHT_LISTENER_REFERENCE === undefined || referenceOnly);
const cases = [
  { name: 'mutation', kinds: ['first-listener', 'removed-listener', 'once-listener', 'resolved-first', 'first-listener', 'added-listener', 'resolved-second'], error: undefined },
  { name: 'async-gate', kinds: ['async-enter', 'next-listener', 'command-resolved', 'async-exit'], error: undefined },
  { name: 'close-reentry', kinds: ['first-close', 'nested-call-returned', 'second-close', 'commands-settled'], error: undefined },
  { name: 'popup-throw', kinds: ['throwing-context-page', 'following-context-page', 'following-command-resolved'], error: 'uncaughtException' },
  { name: 'async-reject', kinds: ['rejecting-listener', 'next-listener', 'command-resolved'], error: 'unhandledRejection' },
  { name: 'context-close-page-throw', kinds: ['throwing-page-close', 'page-close', 'next-page-listener', 'context-close', 'next-context-listener', 'close-settled'], error: 'uncaughtException' },
  { name: 'browser-close-page-throw', kinds: ['throwing-page-close', 'page-close', 'next-page-listener', 'context-close', 'next-context-listener', 'disconnected', 'close-settled'], error: 'uncaughtException' },
  ...['browser-close-page-throw-object', 'browser-close-page-throw-primitive'].map(name => ({ name, kinds: ['throwing-page-close', 'page-close', 'next-page-listener', 'context-close', 'next-context-listener', 'disconnected', 'close-settled'], error: 'uncaughtException' })),
  { name: 'browser-close-page-async-reject', kinds: ['throwing-page-close', 'next-page-listener', 'page-close', 'next-page-listener', 'context-close', 'next-context-listener', 'disconnected', 'close-settled'], error: 'unhandledRejection' },
  { name: 'browser-close-page-reentry', kinds: ['page-close', 'reentry-returned', 'next-page-listener', 'page-close', 'next-page-listener', 'context-close', 'next-context-listener', 'disconnected', 'close-settled'], error: undefined },
  { name: 'browser-close-context-throw', kinds: ['page-close', 'next-page-listener', 'page-close', 'next-page-listener', 'context-close', 'disconnected', 'close-settled'], error: 'uncaughtException' },
  { name: 'disconnect-throw', kinds: ['throwing-disconnected', 'close-settled'], error: 'uncaughtException' },
  { name: 'disconnect-throw-object', kinds: ['throwing-disconnected', 'close-settled'], error: 'uncaughtException' },
  { name: 'disconnect-throw-primitive', kinds: ['throwing-disconnected', 'close-settled'], error: 'uncaughtException' },
  { name: 'disconnect-throw-reentry', kinds: ['throwing-disconnected', 'exception-handler-returned', 'close-settled'], error: 'uncaughtException' },
  { name: 'disconnect-async-reject', kinds: ['throwing-disconnected', 'next-disconnected', 'close-settled'], error: 'unhandledRejection' },
];

async function run(implementation, scenario, unhandled = false) {
  const child = spawn(process.execPath, [fileURLToPath(new URL('../scripts/lifecycle-listener-child.mjs', import.meta.url)), implementation, scenario, unhandled ? 'unhandled' : 'capture'], {
    cwd: fileURLToPath(new URL('../', import.meta.url)), env: process.env, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '', stderr = '', outBytes = 0, errBytes = 0, failure;
  function fail(message) { failure ??= message; child.kill('SIGKILL'); }
  child.stdout.on('data', chunk => {
    outBytes += chunk.length;
    if (outBytes > 64 * 1024) fail('listener report exceeds 64 KiB');
    else stdout += chunk.toString();
  });
  child.stderr.on('data', chunk => {
    errBytes += chunk.length;
    if (errBytes > 64 * 1024) fail('listener diagnostics exceed 64 KiB');
    else stderr += chunk.toString();
  });
  const timer = setTimeout(() => fail('isolated listener scenario exceeded 15 seconds'), 15_000);
  let status, signal;
  try { [status, signal] = await once(child, 'close'); }
  finally { clearTimeout(timer); }
  assert.equal(failure, undefined, failure);
  if (unhandled) {
    assert.equal(status, 1, `unhandled listener must exit unsuccessfully: ${stderr}`);
    assert.equal(signal, null);
    assert.match(stderr, /node110 expected listener failure/);
    assert.equal(stdout, '', 'uncaught failure cannot reach a successful report');
    const value = { exitCode: status, originalErrorReported: true };
    process.stdout.write(`# Lifecycle listener unhandled ${implementation} ${JSON.stringify(value)}\n`);
    return value;
  }
  assert.equal(status, 0, `${implementation}/${scenario}: exit ${status}, signal ${signal}\n${stderr}`);
  assert.doesNotMatch(stderr, /panicked at|fatal runtime error|Pwright environment cleanup failed/);
  const report = JSON.parse(stdout);
  assert.equal(report.schema, 'pwright.listener-semantics.v1');
  assert.equal(report.implementation, implementation);
  assert.equal(report.scenario, scenario);
  assert.ok(report.observation.trace.length <= 32);
  assert.ok(report.observation.errors.length <= 4);
  process.stdout.write(`# Lifecycle listener ${implementation} ${JSON.stringify(report)}\n`);
  return report.observation;
}

for (const entry of cases) test(`${referenceOnly ? 'reference only' : 'paired'} lifecycle listeners: ${entry.name}`, { timeout: 35000 }, async () => {
  // Arrange
  const expected = await run('reference', entry.name);
  assert.equal(expected.outcome, 'resolved');
  assert.deepEqual(expected.trace.map(event => event.kind), entry.kinds);
  assert.ok(expected.trace.every(event => event.sameArgument && event.sameReceiver));
  assert.deepEqual(expected.errors, entry.error ? [{ kind: entry.error, sameError: true }] : []);
  if (entry.name === 'close-reentry') {
    assert.deepEqual(expected.trace[0].closed, [true, false]);
    assert.deepEqual(expected.trace[0].listed, [false, true]);
    assert.deepEqual(expected.trace[1].closed, [true, false]);
    assert.deepEqual(expected.trace[2].closed, [true, true]);
  }
  if (['context-close-page-throw', 'browser-close-page-throw', 'browser-close-context-throw'].includes(entry.name)) {
    assert.deepEqual(expected.trace[0].closed, [true, false]);
    assert.deepEqual(expected.trace[0].listed, [false, true]);
    assert.equal(expected.trace[0].contextListed, true);
    const contextClose = expected.trace.find(event => event.kind === 'context-close');
    assert.deepEqual(contextClose.closed, [true, true]);
    assert.deepEqual(contextClose.listed, [false, false]);
    assert.equal(contextClose.contextListed, false);
  }
  if (referenceOnly) return;
  // Act
  const actual = await run('candidate', entry.name);
  // Assert
  assert.deepEqual(actual, expected);
});

test(`${referenceOnly ? 'reference only' : 'paired'} lifecycle listeners: disconnect-unhandled`, { timeout: 35000 }, async () => {
  // Arrange: no process-level handler intercepts Node's default failure path.
  const expected = await run('reference', 'disconnect-throw', true);
  if (referenceOnly) return;
  // Act
  const actual = await run('candidate', 'disconnect-throw', true);
  // Assert
  assert.deepEqual(actual, expected);
});
