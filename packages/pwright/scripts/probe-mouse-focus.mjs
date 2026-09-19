// Diagnose target activation separately from public input conformance.
// All targets belong to the probe Context; no event filtering is performed.
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
const output = process.env.PWRIGHT_MOUSE_FOCUS_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const deadline = setTimeout(() => { console.error('Mouse focus diagnostic exceeded 30s'); process.exit(1); }, 30_000);
const samples = [];
let failure;
try {
  const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
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
            url.searchParams.set('mouse-focus-probe', `${label}-${process.pid}`);
            await page.goto(url.href, { timeout: 5000 });
            const targets = (await inventory.send('Target.getTargets')).targetInfos.filter(target => target.url === url.href);
            assert.equal(targets.length, 1);
            owner = targets[0].browserContextId;
            assert.equal(typeof owner, 'string');
            const guard = await context.newPage();
            const guardURL = `about:blank#mouse-focus-guard-${label}-${process.pid}`;
            await guard.goto(guardURL, { timeout: 5000 });
            const guards = (await inventory.send('Target.getTargets')).targetInfos.filter(target => target.url === guardURL && target.browserContextId === owner);
            assert.equal(guards.length, 1);
            for (const mode of ['foreground', 'background']) {
              await inventory.send('Target.activateTarget', { targetId: mode === 'foreground' ? targets[0].targetId : guards[0].targetId });
              await page.evaluate(() => window.resetMouse());
              await delay(2000);
              const idle = await page.evaluate(() => window.mouseSnapshot());
              await page.evaluate(() => window.resetMouse());
              await page.mouse.down();
              await page.mouse.up();
              const input = await page.evaluate(() => window.mouseSnapshot());
              samples.push({ provider: label, mode, idleMs: 2000, idle, inputCalls: ['down', 'up'], input });
            }
          } finally {
            await diagnosticDeadline(context.close(), `${label} focus Context cleanup`);
            if (owner) {
              assert.ok(!(await inventory.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
              assert.ok(!(await inventory.send('Target.getBrowserContexts')).browserContextIds.includes(owner));
            }
          }
        } finally { await diagnosticDeadline(browser.close(), `${label} focus disconnect`); }
      }
    } finally { await inventory.detach(); }
  } finally { await diagnosticDeadline(observer.close(), 'Mouse focus observer disconnect'); }
} catch (error) {
  failure = { name: error.name, message: error.message };
  process.exitCode = 1;
} finally { clearTimeout(deadline); }
const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const report = JSON.stringify({ diagnostic: true, artifact, samples, failure }, null, 2);
assert.ok(Buffer.byteLength(report) <= 256 * 1024);
await writeFile(output, report, { flag: 'wx' });
for (const sample of samples) console.log(`${sample.provider}/${sample.mode}: idle=${sample.idle.records.length}, input=${sample.input.records.map(event => event.type).join(',')}`);
if (failure) console.error(`${failure.name}: ${failure.message}`);
