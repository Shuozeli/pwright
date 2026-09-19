// Offline selected-artifact smoke only. No connection or actual preparation
// ticket is created: session provenance, revocation and ingress need other gates.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { realpathSync } from 'node:fs';
import { Worker } from 'node:worker_threads';
import { test } from 'node:test';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(import.meta.url);
const { nativeArtifact } = require('../src/native-artifact.cjs');
assert.ok(['retained-file-input', 'retained-file-input-combined', 'retained-target-marking',
  'retained-target-marking-combined', 'retained-target-prevalidation',
  'retained-target-prevalidation-combined', 'retained-input-instrumentation',
  'retained-input-instrumentation-combined', 'retained-call-argument-serialization',
  'retained-call-argument-serialization-combined', 'retained-serialized-payload-import',
  'retained-serialized-payload-import-combined', 'retained-serialized-payload-mutation',
  'retained-serialized-payload-mutation-combined'].includes(nativeArtifact.profile),
  'select an explicitly built retained-file-input artifact');
const addon = realpathSync(nativeArtifact.path);

test('selected raw preparation class and environment constructor retire in an offline Worker', { timeout: 25000 }, async () => {
  // Arrange: addon loading and environment construction occur only in this
  // Worker, so its normal exit runs the real environment cleanup hook.
  const worker = new Worker(new URL('./native-preparation-smoke-worker.mjs', import.meta.url), {
    workerData: { addon, profile: nativeArtifact.profile }, stdout: true, stderr: true,
  });
  const messages = [];
  let stderr = '';
  let stdout = '';
  let exceededOutput = false;
  let exited = false;
  const append = (previous, chunk) => {
    const next = previous + chunk.toString();
    if (Buffer.byteLength(next) > 8192) exceededOutput = true;
    return next.slice(0, 8192);
  };
  worker.stdout.on('data', chunk => { stdout = append(stdout, chunk); });
  worker.stderr.on('data', chunk => { stderr = append(stderr, chunk); });
  const completion = new Promise((resolve, reject) => {
    worker.on('message', message => {
      if (messages.length >= 2) reject(new Error('offline smoke message bound exceeded'));
      else messages.push(message);
    });
    worker.once('error', reject);
    worker.once('exit', code => { exited = true; resolve(code); });
  });
  try {
    // Act
    const code = await diagnosticDeadline(completion, 'offline preparation Worker exit');
    // Assert
    assert.equal(code, 0, stderr);
    assert.equal(exceededOutput, false, 'offline Worker diagnostic output remains bounded');
    assert.equal(messages.length, 1);
    const payloadMutation = nativeArtifact.profile.startsWith('retained-serialized-payload-mutation');
    const payloadImport = payloadMutation || nativeArtifact.profile.startsWith('retained-serialized-payload-import');
    const serialization = payloadImport || nativeArtifact.profile.startsWith('retained-call-argument-serialization');
    const instrumentation = serialization || nativeArtifact.profile.startsWith('retained-input-instrumentation');
    const prevalidation = instrumentation || nativeArtifact.profile.startsWith('retained-target-prevalidation');
    const marking = prevalidation || nativeArtifact.profile.startsWith('retained-target-marking');
    assert.deepEqual(messages[0], { constructorAndClassChecks: true, classes: 10,
      selectedSessionMethods: payloadMutation ? 14 : payloadImport ? 13 : serialization ? 12 : instrumentation ? 11 : prevalidation ? 9 : marking ? 8 : 5,
      selectedPreparationMethods: marking ? 8 : 6,
      actualTickets: 0, browserConnections: 0 });
    assert.doesNotMatch(stderr, /Pwright environment cleanup failed/);
    assert.equal(stdout, '');
  } finally {
    if (!exited) await diagnosticDeadline(worker.terminate(), 'offline preparation Worker termination');
  }
});
