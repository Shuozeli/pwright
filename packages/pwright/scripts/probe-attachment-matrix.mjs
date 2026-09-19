// Bounded diagnostic, not a parity test. No Page is created, navigated or closed.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { diagnosticDeadline } from './probe-deadline.mjs';
import artifactConfig from '../src/native-artifact.cjs';
import * as candidate from '../src/index.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const output = process.env.PWRIGHT_ATTACHMENT_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const oracle = requireOracle('playwright');
const apis = { oracle, candidate };
const layouts = [
  { subject: 'oracle', observer: null },
  { subject: 'candidate', observer: null },
  { subject: 'oracle', observer: 'oracle' },
  { subject: 'candidate', observer: 'oracle' },
  { subject: 'candidate', observer: 'candidate' },
  { subject: 'oracle', observer: 'oracle', observerSession: true },
  { subject: 'candidate', observer: 'oracle', observerSession: true },
];
const observations = [];
let observer, subject, control;
// Diagnostic process bound, independently of API deadlines and cleanup bounds.
const deadline = setTimeout(() => { console.error('Attachment matrix exceeded 180s'); process.exit(1); }, 180_000);
try {
  for (let round = 0; round < 2; round++) {
    for (const layout of layouts) {
      let phase = 'observer';
      const started = performance.now();
      const record = { round, ...layout };
      const timed = async (key, operation) => {
        const began = performance.now();
        try { return await operation(); }
        finally { record[key] = Math.round(performance.now() - began); }
      };
      console.log(JSON.stringify({ event: 'start', ...record }));
      try {
        if (layout.observer) observer = await timed('observerConnectMs', () => apis[layout.observer].chromium.connectOverCDP(endpoint, { timeout: 5000 }));
        if (layout.observerSession) control = await timed('observerSessionMs', () => diagnosticDeadline(observer.newBrowserCDPSession(), 'attachment observer session creation'));
        phase = 'subject';
        subject = await timed('subjectConnectMs', () => apis[layout.subject].chromium.connectOverCDP(endpoint, { timeout: 5000 }));
        record.ok = true;
      } catch (error) {
        record.ok = false; record.phase = phase;
        record.error = String(error.message).slice(0, 1024);
      } finally {
        // Cleanup errors are terminal: never continue a matrix with a live owner.
        if (subject) {
          await timed('subjectCloseMs', () => diagnosticDeadline(subject.close(), 'attachment subject disconnect'));
          if (layout.subject === 'candidate') {
            record.metrics = subject.pwrightMetrics();
            assert.equal(record.metrics.activeLeases, 0);
            assert.equal(record.metrics.activeOperations, 0);
          }
          subject = undefined;
        }
        if (observer) {
          if (control) {
            await diagnosticDeadline(control.detach(), 'attachment observer session detach');
            control = undefined;
          }
          await timed('observerCloseMs', () => diagnosticDeadline(observer.close(), 'attachment observer disconnect'));
          observer = undefined;
        }
      }
      record.elapsedMs = Math.round(performance.now() - started);
      observations.push(record);
      console.log(JSON.stringify({ event: 'finished', ...record }));
    }
  }
  const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
  await writeFile(output, JSON.stringify({ artifact, observations }, null, 2), { flag: 'wx' });
} finally { clearTimeout(deadline); }
