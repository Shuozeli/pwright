// Diagnostic only. Fixed six attachments, no retry or compatibility exemption.
// CDP is observation-only on the single fixture Page owned by this probe.
import assert from 'node:assert/strict';
import artifactConfig from '../src/native-artifact.cjs';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from './probe-deadline.mjs';
import { WorldInventory } from './probe-world-inventory.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222', 'requires dedicated Chrome/Pwright preflight');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = requireOracle('playwright');
const nativeSha256 = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const observer = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
let page;
try {
  page = await diagnosticDeadline(observer.contexts()[0].newPage(), 'owned fixture Page');
  const session = await diagnosticDeadline(page.context().newCDPSession(page), 'owned Page CDP session');
  try {
    const { frameTree } = await diagnosticDeadline(session.send('Page.getFrameTree'), 'owned frame identity');
    const inventory = new WorldInventory(frameTree.frame.id);
    session.on('Runtime.executionContextCreated', event => inventory.created(event));
    const snapshot = async label => {
      await diagnosticDeadline(session.send('Runtime.disable'), 'disable inventory');
      inventory.reset();
      await diagnosticDeadline(session.send('Runtime.enable'), 'snapshot inventory');
      const sample = { label, ...inventory.snapshot() };
      console.log(JSON.stringify(sample));
      assert.equal(sample.overflow, false, 'diagnostic metadata capacity exceeded');
      assert.equal(sample.counts.main, 1, 'owned main-frame snapshot incomplete');
      return sample;
    };
    console.log(JSON.stringify({ schema: 'pwright.retained-world-probe.v1', nativeSha256, nativeProfile: artifactConfig.nativeArtifact.profile,
      chrome: observer.version(), oracle: '1.58.2', timeoutMs: 5000,
      note: 'Context counts are not RSS, heap size or a timeout root-cause proof.' }));
    const baseline = await snapshot('baseline');
    for (const [index, name] of ['candidate', 'oracle', 'oracle', 'candidate', 'candidate', 'oracle'].entries()) {
      const type = name === 'candidate' ? candidate : oracle;
      const started = performance.now();
      let browser;
      const row = { label: `${index + 1}-${name}`, timeoutMs: 5000 };
      try {
        browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
        row.attachMs = performance.now() - started;
      } catch (error) {
        row.error = String(error).slice(0, 4096);
      } finally {
        try { await diagnosticDeadline(browser?.close(), 'sample attachment cleanup'); }
        catch (error) { row.cleanupError = String(error).slice(0, 4096); }
      }
      row.elapsedMs = performance.now() - started;
      console.log(JSON.stringify(row));
      assert.ok(!row.error && !row.cleanupError, 'attachment failed; stop without retry');
      assert.equal(page.isClosed(), false, 'borrowed fixture must survive disconnect');
      if (browser.pwrightMetrics) {
        const metrics = browser.pwrightMetrics();
        assert.equal(metrics.activeLeases, 0);
        assert.equal(metrics.cleanupInFlight, 0);
      }
      await snapshot(`${row.label}-after-disconnect`);
    }
    // Only this owned blank document is reloaded. Counts alone cannot distinguish
    // init-script registration from Chromium retaining named WindowProxy worlds.
    await page.reload({ timeout: 5000 });
    const reloaded = await snapshot('owned-document-after-reload');
    assert.notEqual(reloaded.mainContextIds[0], baseline.mainContextIds[0], 'reload must create a new main world');
  } finally { await diagnosticDeadline(session.detach(), 'owned CDP detach'); }
} finally {
  try { await diagnosticDeadline(page?.close(), 'owned fixture cleanup'); }
  finally { await diagnosticDeadline(observer.close(), 'observer disconnect'); }
}
console.log(JSON.stringify({ label: 'complete', reloadCreatedNewMainWorld: true, ownedFixtureCleanupComplete: true }));
