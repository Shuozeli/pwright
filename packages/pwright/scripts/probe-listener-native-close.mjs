// Run the original isolated listener scenarios with unchanged deadlines and a
// transparent loopback transport. Keep failures, including fatal child errors.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import artifact from '../src/native-artifact.cjs';
import { withCdpCommandRelay } from '../test/cdp-command-relay.mjs';
import { CloseTrace } from '../test/native-close-trace.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
const output = process.env.PWRIGHT_CLOSE_REPORT;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(output && isAbsolute(output));
const report = { schema: 'pwright.listener-close-probe.v1', profile: artifact.nativeArtifact.profile,
  sha256: createHash('sha256').update(await readFile(artifact.nativeArtifact.path)).digest('hex'), samples: [] };

async function run(scenario, relay) {
  const child = spawn(process.execPath, [fileURLToPath(new URL('./lifecycle-listener-child.mjs', import.meta.url)), 'candidate', scenario], {
    env: { ...process.env, PWRIGHT_LISTENER_DIAGNOSTIC_RELAY: relay }, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '', stderr = '', outBytes = 0, errBytes = 0, failure;
  function fail(message) { failure ??= message; child.kill('SIGKILL'); }
  child.stdout.on('data', raw => {
    outBytes += raw.length;
    if (outBytes > 64 * 1024) fail('listener stdout exceeded 64 KiB');
    else stdout += raw.toString();
  });
  child.stderr.on('data', raw => {
    errBytes += raw.length;
    if (errBytes > 64 * 1024) fail('listener stderr exceeded 64 KiB');
    else stderr += raw.toString();
  });
  const deadline = setTimeout(() => fail('listener child exceeded 15 seconds'), 15_000);
  try {
    const [status, signal] = await once(child, 'close');
    const result = { status, signal, failure, stdout, stderr };
    if (status === 0 && !failure) {
      const observed = JSON.parse(stdout);
      assert.equal(observed.schema, 'pwright.listener-semantics.v1');
      assert.equal(observed.scenario, scenario);
      assert.equal(observed.observation.outcome, 'resolved');
    }
    return result;
  } finally { clearTimeout(deadline); }
}

try {
  for (let round = 0; round < 8; round++) {
    for (const scenario of ['mutation', 'close-reentry']) {
      const trace = new CloseTrace(() => performance.now());
      const sample = { round, scenario };
      report.samples.push(sample);
      try {
        sample.child = await withCdpCommandRelay(endpoint, trace, relay => run(scenario, relay));
      } finally { sample.native = trace.report(); }
      const targets = await fetch(`${endpoint}/json/list`, { signal: AbortSignal.timeout(5000) }).then(response => response.json());
      sample.ownedRemaining = targets.filter(value => trace.created.includes(value.id)).map(value => value.id);
      process.stdout.write(`${JSON.stringify({ round, scenario, status: sample.child.status,
        unanswered: sample.native.unanswered, remaining: sample.ownedRemaining })}\n`);
      assert.deepEqual(sample.ownedRemaining, [], 'stop before adding owners if cleanup left a target');
      if (sample.child.status !== 0 || sample.child.failure) {
        process.exitCode = 1;
        // First failure changes the next action: inspect it before more traffic.
        throw new Error(`captured ${scenario} child failure`);
      }
    }
  }
} catch (error) {
  report.failure = String(error).slice(0, 4096);
  process.exitCode = 1;
} finally {
  const encoded = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(encoded) <= 4 * 1024 * 1024);
  await writeFile(output, encoded, { flag: 'wx' });
}
