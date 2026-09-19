// One public workload on the pinned oracle and Rust addon. Polling below is an
// explicit settled-metadata barrier, not a claim of Frame-event/timing parity.
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
const fixture = new URL(process.env.PWRIGHT_DRAG_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_FRAME_TREE_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
assert.equal(typeof globalThis.gc, 'function', 'Run Node with --expose-gc for last-wrapper release checks');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');

async function settled(predicate, label) {
  const deadline = performance.now() + 5000;
  while (!predicate()) {
    assert.ok(performance.now() < deadline, `Metadata settlement timed out: ${label}`);
    await delay(5);
  }
}

const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
let oracle, candidate;
try {
  const session = await observer.newBrowserCDPSession();
  try {
    async function collect(api, label) {
      const browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
      const observations = [];
      async function sample(mode) {
        const context = await browser.newContext();
        let owner;
        try {
          const page = await context.newPage();
          const url = new URL(fixture);
          url.searchParams.set('frame-mode', mode);
          url.searchParams.set('frame-tree-run', `${process.pid}-${label}-${mode}`);
          await page.goto(url.href, { timeout: 5000 });
          const targets = (await session.send('Target.getTargets')).targetInfos.filter(target => target.url === url.href);
          assert.equal(targets.length, 1);
          owner = targets[0].browserContextId;
          assert.equal(typeof owner, 'string');
          const main = page.mainFrame();
          await settled(() => main.childFrames().length === 1, 'initial child');
          const child = main.childFrames()[0];
          assert.equal(await child.title(), 'Pwright child Frame keyboard fixture');
          await settled(() => child.name() === 'keyboard-frame', 'initial navigation name');
          observations.push({ id: `${mode}/initial`, count: page.frames().length,
            name: child.name(), mainName: main.name(),
            parent: child.parentFrame() === main, rootParent: main.parentFrame(),
            owner: child.page() === page, same: page.frames().includes(child),
            detached: child.isDetached(), mainDetached: main.isDetached() });
          const metric = label === 'candidate' ? browser.pwrightFrameTreeMetrics() : null;
          const metadataMetric = label === 'candidate' ? browser.pwrightMetadataMetrics() : null;
          for (let index = 0; index < 1000; index++) {
            assert.equal(page.frames()[1], child);
            assert.equal(main.childFrames()[0], child);
            assert.equal(child.name(), 'keyboard-frame');
          }
          if (metric) assert.deepEqual(browser.pwrightFrameTreeMetrics(), metric, 'enumeration must release native conversion storage');
          if (metadataMetric) assert.deepEqual(browser.pwrightMetadataMetrics(), metadataMetric, 'name reads must not accumulate native storage');

          await child.evaluate(() => { window.name = 'window-你好'; });
          observations.push({ id: `${mode}/name-window-mutation`, name: child.name(),
            actualWindowName: await child.evaluate(() => window.name) });
          await page.evaluate(() => { document.querySelector('iframe').name = 'attribute-你好'; });
          observations.push({ id: `${mode}/name-attribute-mutation`, name: child.name() });
          await child.evaluate(() => { location.hash = 'name-same-document'; });
          await settled(() => child.url().endsWith('#name-same-document'), 'same-document URL');
          observations.push({ id: `${mode}/name-same-document`, name: child.name() });
          await page.evaluate(() => {
            const child = document.querySelector('iframe');
            const next = new URL(child.src);
            next.searchParams.set('name-navigation', '1');
            child.src = next.href;
          });
          await settled(() => child.url().includes('name-navigation=1'), 'full-navigation URL');
          assert.equal(await child.title(), 'Pwright child Frame keyboard fixture');
          observations.push({ id: `${mode}/name-navigation`, name: child.name(),
            same: main.childFrames()[0] === child });

          await child.evaluate(() => {
            const nested = document.createElement('iframe');
            nested.srcdoc = '<title>nested child</title>'; document.body.append(nested);
          });
          await settled(() => page.frames().length === 3 && child.childFrames().length === 1, 'nested attachment');
          const nested = child.childFrames()[0];
          observations.push({ id: `${mode}/nested`, count: page.frames().length,
            name: nested.name(),
            parent: nested.parentFrame() === child, rootChildren: main.childFrames().length,
            owner: nested.page() === page, same: page.frames().includes(nested) });

          await page.evaluate(() => document.querySelector('iframe').remove());
          await settled(() => page.frames().length === 1 && child.isDetached() && nested.isDetached(), 'subtree removal');
          observations.push({ id: `${mode}/removed`, count: page.frames().length,
            retainedName: child.name(), nestedName: nested.name(),
            parent: child.parentFrame() === main, nestedParent: nested.parentFrame() === child,
            children: child.childFrames().length, rootChildren: main.childFrames().length,
            detached: child.isDetached(), nestedDetached: nested.isDetached(),
            retainedURL: child.url().includes('raw-drag-frame.html') });

          await page.evaluate(() => {
            const replacement = document.createElement('iframe');
            replacement.srcdoc = '<title>replacement child</title>'; document.body.append(replacement);
          });
          await settled(() => page.frames().length === 2, 'replacement attachment');
          const replacement = main.childFrames()[0];
          observations.push({ id: `${mode}/replacement`, fresh: replacement !== child,
            parent: replacement.parentFrame() === main, detached: replacement.isDetached(),
            retiredAbsent: !page.frames().includes(child) && !page.frames().includes(nested) });
          await page.close();
          observations.push({ id: `${mode}/closed`, count: page.frames().length,
            mainName: main.name(), childName: replacement.name(), retiredName: child.name(),
            children: main.childFrames().length, same: main.childFrames()[0] === replacement,
            mainDetached: main.isDetached(), childDetached: replacement.isDetached(),
            parent: replacement.parentFrame() === main, owner: replacement.page() === page });
        } finally {
          await diagnosticDeadline(context.close(), `${label} Frame Context cleanup`);
          if (owner) {
            assert.ok(!(await session.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
            assert.ok(!(await session.send('Target.getBrowserContexts')).browserContextIds.includes(owner));
          }
        }
      }
      try { for (const mode of ['local', 'sandbox']) await sample(mode); }
      finally { await diagnosticDeadline(browser.close(), `${label} Frame disconnect`); }
      let metrics = null;
      if (label === 'candidate') {
        await settled(() => {
          globalThis.gc();
          const value = browser.pwrightFrameTreeMetrics();
          // NODE-115: retained closed Browser owns exactly one fixed
          // publication-port lease, independently of all Frame wrappers.
          return value.states === 1 && value.revisions === 1;
        }, 'last native Frame wrapper finalization');
        metrics = { host: browser.pwrightMetrics(), tree: browser.pwrightFrameTreeMetrics(), urls: browser.pwrightMetadataMetrics() };
        assert.equal(metrics.host.activeLeases, 0);
        assert.equal(metrics.host.activeOperations, 0);
        assert.equal(metrics.host.cleanupInFlight, 0);
        assert.equal(metrics.urls.entries, 0);
        assert.equal(metrics.urls.nameBytes, 0);
        assert.equal(metrics.urls.pendingPublications, 0);
      }
      return { observations, metrics };
    }
    oracle = await collect(upstream, 'oracle');
    candidate = await collect(await import('../src/index.mjs'), 'candidate');
  } finally { await session.detach(); }
} finally { await diagnosticDeadline(observer.close(), 'Frame observer disconnect'); }
const cases = oracle.observations.map((expected, index) => {
  const actual = candidate.observations[index];
  let match = false;
  try { assert.deepEqual(actual, expected); match = true; } catch { /* Preserve the discrepancy in the report. */ }
  return { id: expected.id, match, oracle: expected, candidate: actual };
});
const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const encoded = JSON.stringify({ oracle: 'playwright@1.58.2', artifact, nativeProfile: artifactConfig.nativeArtifact.profile,
  cases, metrics: candidate.metrics }, null, 2);
assert.ok(Buffer.byteLength(encoded) <= 128 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
for (const sample of cases) console.log(`${sample.match ? 'MATCH' : 'FAIL'} ${sample.id}`);
assert.equal(cases.filter(sample => sample.match).length, cases.length);
console.log(`Public Frame tree: ${cases.length}/${cases.length}; owned Contexts/targets absent; native wrappers released`);
