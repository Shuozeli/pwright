// Bounded, identity-only CDP diagnostics. This is not a parity or timing fix.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { setImmediate } from 'node:timers/promises';
import { chromium } from '@shuozeli/pwright';
import artifact from '../src/native-artifact.cjs';
import { withCdpCommandRelay } from '../test/cdp-command-relay.mjs';
import { CloseTrace } from '../test/native-close-trace.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
const output = process.env.PWRIGHT_CLOSE_REPORT;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(output && isAbsolute(output));
const rounds = 8;
const report = { schema: 'pwright.close-probe.v1', profile: artifact.nativeArtifact.profile,
  sha256: createHash('sha256').update(await readFile(artifact.nativeArtifact.path)).digest('hex'), rounds: [] };
const errorValue = error => ({ code: error.code, message: String(error.message).slice(0, 4096) });


try {
  for (let round = 0; round < rounds; round++) {
    const trace = new CloseTrace(() => performance.now());
    const observation = { round, errors: [], trace: trace.receipts };
    report.rounds.push(observation);
    await withCdpCommandRelay(endpoint, trace, async relay => {
      const browser = await chromium.connectOverCDP(relay, { timeout: 5000 });
      let context;
      try {
        context = await browser.newContext();
        const first = await context.newPage();
        const second = await context.newPage();
        let nested;
        first.once('close', () => {
          nested = second.close().then(() => undefined, errorValue);
        });
        const primary = await first.close().then(() => undefined, errorValue);
        const secondary = await nested;
        if (primary) observation.errors.push({ phase: 'first Page close', ...primary });
        if (secondary) observation.errors.push({ phase: 'reentrant Page close', ...secondary });
        if (!nested && !primary) throw new Error('first Page close did not publish its listener');
      } catch (error) {
        observation.errors.push({ phase: 'setup or operation', ...errorValue(error) });
      } finally {
        try { if (browser.isConnected()) await context?.close(); }
        catch (error) { observation.errors.push({ phase: 'Context cleanup', ...errorValue(error) }); }
        try { await browser.close(); }
        catch (error) { observation.errors.push({ phase: 'Browser cleanup', ...errorValue(error) }); }
        observation.unanswered = trace.unanswered();
        observation.ffiBeforeDrain = browser.pwrightFfiMetrics();
        for (let index = 0; index < 100 && browser.pwrightFfiMetrics().pending; index++) await setImmediate();
        observation.resources = { host: browser.pwrightMetrics(), ffi: browser.pwrightFfiMetrics() };
        assert.equal(observation.resources.host.activeLeases, 0);
        assert.equal(observation.resources.host.cleanupInFlight, 0);
        assert.equal(observation.resources.ffi.pending, 0);
        assert.equal(observation.resources.ffi.reservedBytes, 0);
      }
    });
    // Read-only inventory; never close a target not explicitly created here.
    const targets = await fetch(`${endpoint}/json/list`, { signal: AbortSignal.timeout(5000) }).then(response => response.json());
    observation.ownedRemaining = targets.filter(value => trace.created.includes(value.id)).map(value => value.id);
    process.stdout.write(`${JSON.stringify(observation)}\n`);
    assert.deepEqual(observation.ownedRemaining, [], 'stop before adding more owners if cleanup left a target');
    if (observation.errors.length) process.exitCode = 1;
  }
} catch (error) {
  report.failure = errorValue(error);
  process.exitCode = 1;
} finally {
  const encoded = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(encoded) <= 1024 * 1024);
  await writeFile(output, encoded, { flag: 'wx' });
}
