// Diagnostic only: record fixture events while neither provider sends input.
// Never filter these events out of the public Mouse conformance comparison.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import artifactConfig from '../src/native-artifact.cjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_MOUSE_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_IDLE_MOUSE_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const deadline = setTimeout(() => { console.error('Idle Mouse diagnostic exceeded 30s'); process.exit(1); }, 30_000);
try {
  const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
  const samples = [];
  try {
    const inventory = await observer.newBrowserCDPSession();
    try {
      for (const [label, api] of [['oracle', upstream], ['candidate', await import('../src/index.mjs')]]) {
        const browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
        try {
          const context = await browser.newContext();
          let owner;
          try {
            const page = await context.newPage();
            const url = new URL(fixture);
            url.searchParams.set('idle-mouse-probe', `${label}-${process.pid}`);
            await page.goto(url.href, { timeout: 5000 });
            const targets = (await inventory.send('Target.getTargets')).targetInfos.filter(target => target.url === url.href);
            assert.equal(targets.length, 1);
            owner = targets[0].browserContextId;
            assert.equal(typeof owner, 'string');
            for (const phase of ['initial', 'second']) {
              await page.evaluate(() => window.resetMouse());
              // No Mouse/Keyboard API or CDP Input command occurs in this interval.
              await delay(2000);
              const snapshot = await page.evaluate(() => window.mouseSnapshot());
              samples.push({ provider: label, phase, inputCalls: 0, observationMs: 2000, snapshot });
            }
          } finally {
            await diagnosticDeadline(context.close(), `${label} idle Mouse Context cleanup`);
            if (owner) {
              assert.ok(!(await inventory.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
              assert.ok(!(await inventory.send('Target.getBrowserContexts')).browserContextIds.includes(owner));
            }
          }
        } finally { await diagnosticDeadline(browser.close(), `${label} idle Mouse disconnect`); }
      }
    } finally { await inventory.detach(); }
  } finally { await diagnosticDeadline(observer.close(), 'Idle Mouse observer disconnect'); }
  const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
  const report = JSON.stringify({ diagnostic: true, artifact, samples }, null, 2);
  assert.ok(Buffer.byteLength(report) <= 256 * 1024);
  await writeFile(output, report, { flag: 'wx' });
  for (const sample of samples) console.log(`${sample.provider}/${sample.phase}: ${sample.snapshot.records.length} events with zero input calls; overflow=${sample.snapshot.overflow}`);
} finally { clearTimeout(deadline); }
