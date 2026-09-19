// Real public Frame wrappers fill the native state quota. Inspect exact owned
// native targets to detect creation before a failed JS root-binding admission.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import * as api from '../src/index.mjs';
import artifactConfig from '../src/native-artifact.cjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const output = process.env.PWRIGHT_PAGE_BINDING_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
async function settled(predicate) {
  const deadline = performance.now() + 5000;
  while (!predicate()) { assert.ok(performance.now() < deadline, 'Frame publication deadline'); await delay(5); }
}
const deadline = setTimeout(() => { console.error('Page binding probe exceeded 180s'); process.exit(1); }, 180_000);
let result;
try {
  const observer = await requireOracle('playwright').chromium.connectOverCDP(endpoint, { timeout: 5000 });
  try {
    const session = await observer.newBrowserCDPSession();
    try {
      const browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
      try {
        const context = await browser.newContext();
        let owner;
        try {
          const page = await context.newPage();
          const url = `about:blank#pwright-node054-${process.pid}`;
          await page.goto(url, { timeout: 5000 });
          const targets = (await session.send('Target.getTargets')).targetInfos.filter(target => target.url === url);
          assert.equal(targets.length, 1);
          owner = targets[0].browserContextId;
          assert.equal(typeof owner, 'string');
          let quotaPage;
          const initialStates = browser.pwrightFrameTreeMetrics().states;
          if (initialStates % 2 === 0) {
            // Ambient browser Frames can make the starting count even. Keep
            // the exact 253-state gate: a second owned Page admits four states,
            // and its unenumerated child adds one node, not a JS Frame binding.
            quotaPage = await context.newPage();
            await quotaPage.evaluate(() => {
              const frame = document.createElement('iframe');
              frame.srcdoc = '<title>quota parity</title>'; document.body.append(frame);
            });
            await settled(() => browser.pwrightFrameTreeMetrics().states === initialStates + 5);
          }
          const retained = [];
          while (browser.pwrightFrameTreeMetrics().states < 253) {
            assert.ok(retained.length < 127, 'Frame retention diagnostic bound');
            await page.evaluate(() => {
              const frame = document.createElement('iframe'); frame.srcdoc = '<title>retained</title>'; document.body.append(frame);
            });
            await settled(() => page.frames().length === 2);
            const child = page.mainFrame().childFrames()[0];
            retained.push(child);
            await page.evaluate(() => document.querySelector('iframe').remove());
            await settled(() => child.isDetached() && page.frames().length === 1);
          }
          const before = browser.pwrightFrameTreeMetrics();
          assert.equal(before.states, 253, 'Three slots remain for tree/node/root but not the tree wrapper');
          const beforeTargets = (await session.send('Target.getTargets')).targetInfos.filter(target => target.browserContextId === owner).map(target => target.targetId);
          let errorCode;
          try { await context.newPage(); } catch (error) { errorCode = error.code; }
          const afterTargets = (await session.send('Target.getTargets')).targetInfos.filter(target => target.browserContextId === owner).map(target => target.targetId);
          result = { before, after: browser.pwrightFrameTreeMetrics(), errorCode,
            retained: retained.length, unexpectedTargets: afterTargets.filter(id => !beforeTargets.includes(id)) };
          // Keep all wrappers observably live through the creation attempt.
          assert.ok(retained.every(frame => frame.isDetached() && frame.page() === page));
          assert.ok(!quotaPage || quotaPage.context() === context);
        } finally {
          await diagnosticDeadline(context.close(), 'Page binding Context cleanup');
          if (owner) {
            assert.ok(!(await session.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
            assert.ok(!(await session.send('Target.getBrowserContexts')).browserContextIds.includes(owner));
          }
        }
      } finally { await diagnosticDeadline(browser.close(), 'Page binding candidate disconnect'); }
    } finally { await session.detach(); }
  } finally { await diagnosticDeadline(observer.close(), 'Page binding observer disconnect'); }
  const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
  await writeFile(output, JSON.stringify({ artifact, result }, null, 2), { flag: 'wx' });
  console.log(JSON.stringify(result));
  assert.equal(result.errorCode, 'PWRIGHT_RESOURCE_EXHAUSTED');
  assert.equal(result.unexpectedTargets.length, 0, 'Rejected newPage must not create a native target');
  assert.deepEqual(result.after, result.before, 'Failed admission must restore metadata leases');
} finally { clearTimeout(deadline); }
