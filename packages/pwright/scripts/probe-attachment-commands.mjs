// Bounded command-identity trace of the actual Rust host, not Node API parity.
// Observer I/O stays asynchronous while the Rust child runs; spawnSync here
// would freeze Playwright's observer and create a false attachment experiment.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
const binary = process.env.PWRIGHT_ATTACHMENT_TRACE_BINARY;
const output = process.env.PWRIGHT_ATTACHMENT_COMMAND_REPORT;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(binary && isAbsolute(binary) && output && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const report = { scope: 'existing-attach-probe-layout-comparison', artifact: createHash('sha256').update(await readFile(binary)).digest('hex'), observations: [] };

function trace() {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, ['--ignored', '--nocapture', '--exact', 'report_unanswered_attach_commands_without_changing_the_deadline'], {
      env: { ...process.env, PWRIGHT_TEST_CDP: endpoint }, stdio: ['ignore', 'pipe', 'pipe'],
    });
    const stdout = [], stderr = [];
    let outBytes = 0, errBytes = 0, failure;
    const stop = reason => { failure ??= reason; child.kill('SIGKILL'); };
    const deadline = setTimeout(() => stop('Rust trace child exceeded 30000ms'), 30_000);
    child.stdout.on('data', chunk => {
      outBytes += chunk.length;
      if (outBytes > 256 * 1024) stop('Rust trace stdout exceeded 256 KiB');
      else stdout.push(chunk);
    });
    child.stderr.on('data', chunk => {
      errBytes += chunk.length;
      if (errBytes > 64 * 1024) stop('Rust trace stderr exceeded 64 KiB');
      else stderr.push(chunk);
    });
    child.on('error', error => { failure = String(error); });
    child.on('close', (code, signal) => {
      clearTimeout(deadline);
      const errors = Buffer.concat(stderr).toString('utf8');
      if (failure || /panicked at|fatal runtime error/.test(errors))
        return reject(new Error(failure ?? `Rust trace failed (${code}, ${signal}): ${errors.slice(0, 4096)}`));
      try {
        const lines = Buffer.concat(stdout).toString('utf8').split('\n');
        const line = lines.find(line => line.startsWith('{"schema":"pwright.attach-command-probe.v1"'));
        assert.ok(line, `existing attach probe must emit a bounded report (exit ${code}, ${signal})`);
        const sample = JSON.parse(line);
        // A reported attach timeout is a diagnostic observation. Any other
        // test/process failure remains terminal evidence-capture failure.
        assert.ok(code === 0 || (code === 101 && sample.error));
        resolve(sample);
      }
      catch (error) { reject(error); }
    });
  });
}

function summarize(sample) {
  assert.equal(sample.overflow, false);
  assert.ok(Object.keys(sample.pending).length <= 32 && sample.recentCompleted.length <= 128);
  assert.ok(Array.isArray(sample.lifecycle) && sample.lifecycle.length <= 64,
    'the rebuilt probe must include bounded native lifecycle identities');
  for (const receipt of sample.lifecycle) {
    assert.ok(['Target.detachedFromTarget', 'Target.targetDestroyed'].includes(receipt.method));
    assert.ok(receipt.sessionId.length <= 128 && receipt.targetId.length <= 128);
    assert.ok(receipt.unansweredCommandIds.length <= 32);
  }
  if (!sample.error) assert.ok(sample.commands > 0, 'successful attachment must include captured commands');
  const completed = sample.recentCompleted.map(command => ({ id: command.id, method: command.method,
    sessionId: command.session_id, targetId: sample.sessions[command.session_id], elapsedMs: command.response_ms - command.prepared_ms }));
  completed.sort((a, b) => b.elapsedMs - a.elapsedMs);
  return { preparedWithoutObservedResponse: Object.values(sample.pending).map(command => ({
    ...command, nativeRetirementReceipts: sample.lifecycle.filter(receipt => receipt.unansweredCommandIds.includes(command.id)),
  })), nativeLifecycle: sample.lifecycle, slowestResponses: completed.slice(0, 10) };
}

const layouts = ['alone', 'observer', 'observer-browser-session'];
let observer, control;
try {
  for (let round = 0; round < 2; round++) {
    for (const layout of layouts) {
      const observation = { round, layout };
      let phase = 'observer';
      report.observations.push(observation);
      try {
        if (layout !== 'alone') observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
        if (layout === 'observer-browser-session') control = await diagnosticDeadline(observer.newBrowserCDPSession(), 'trace observer session creation');
        phase = 'Rust host';
        observation.trace = await trace();
        observation.summary = summarize(observation.trace);
        console.log(JSON.stringify({ round, layout, connected: !observation.trace.error,
          error: observation.trace.error, ...observation.summary }));
      } catch (error) {
        observation.failure = { phase, message: String(error).slice(0, 4096) };
        if (phase === 'Rust host') process.exitCode = 1;
        console.log(JSON.stringify({ round, layout, ...observation.failure }));
      } finally {
        // Failure to clean up ends the experiment rather than adding more owners.
        if (control) { await diagnosticDeadline(control.detach(), 'trace observer session detach'); control = undefined; }
        if (observer) { await diagnosticDeadline(observer.close(), 'trace observer disconnect'); observer = undefined; }
      }
    }
  }
} catch (error) {
  report.failure = String(error).slice(0, 4096);
  process.exitCode = 1;
} finally {
  const encoded = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(encoded) <= 2 * 1024 * 1024);
  await writeFile(output, encoded, { flag: 'wx' });
}
