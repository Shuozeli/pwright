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
const output = process.env.PWRIGHT_PAGE_ENUMERATION_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
assert.equal(typeof globalThis.gc, 'function');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
async function settled(predicate) {
  const end = performance.now() + 5000;
  while (!predicate()) { assert.ok(performance.now() < end, 'Page collection publication deadline'); await delay(5); }
}
const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
let oracle, candidate;
try {
  const session = await observer.newBrowserCDPSession();
  try {
    async function collect(api, label) {
      const browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
      const observations = [];
      let owner;
      async function workload() {
        const context = await browser.newContext();
        try {
          const page = await context.newPage();
          const url = `about:blank#pwright-pages-${label}-${process.pid}`;
          await page.goto(url, { timeout: 5000 });
          const targets = (await session.send('Target.getTargets')).targetInfos.filter(target => target.url === url);
          assert.equal(targets.length, 1);
          owner = targets[0].browserContextId;
          assert.equal(typeof owner, 'string');
          const root = page.mainFrame();
          const first = context.pages();
          observations.push({ id: 'created', count: first.length, same: first[0] === page,
            root: first[0].mainFrame() === root, context: page.context() === context });

          const before = label === 'candidate' ? browser.pwrightFrameTreeMetrics() : null;
          for (let index = 0; index < 2000; index++) {
            const pages = context.pages();
            assert.equal(pages.length, 1);
            assert.equal(pages[0], page);
            assert.notEqual(pages, first);
            assert.ok(browser.contexts().includes(context));
          }
          if (before) assert.deepEqual(browser.pwrightFrameTreeMetrics(), before, 'cached enumeration must not allocate native wrappers or retain conversion bytes');
          observations.push({ id: 'cached', stable: context.pages()[0] === page, root: context.pages()[0].mainFrame() === root });

          observations.push({ id: 'evaluation-activation',
            functionValue: await page.evaluate(() => navigator.userActivation.isActive),
            expressionValue: await page.evaluate('navigator.userActivation.isActive') });

          const popupUrl = `${url}-popup`;
          const opened = await page.evaluate(url => Boolean(window.open(url)), popupUrl);
          console.log(JSON.stringify({ label, popupOpened: opened }));
          assert.equal(opened, true, 'Public evaluate must allow the requested popup');
          await settled(() => context.pages().length === 2 && context.pages().some(value => value.url() === popupUrl));
          const popup = context.pages().find(value => value !== page);
          const popupRoot = popup.mainFrame();
          observations.push({ id: 'discovered', count: context.pages().length, same: context.pages().includes(popup),
            root: context.pages().find(value => value === popup).mainFrame() === popupRoot,
            context: popup.context() === context, rootOwner: popupRoot.page() === popup });
          const populated = label === 'candidate' ? browser.pwrightFrameTreeMetrics() : null;
          for (let index = 0; index < 2000; index++) assert.equal(context.pages().find(value => value !== page), popup);
          if (populated) assert.deepEqual(browser.pwrightFrameTreeMetrics(), populated);
          await popup.close();
          observations.push({ id: 'removed', count: context.pages().length, retainedUrl: popup.url() === popupUrl,
            retainedRoot: popup.mainFrame() === popupRoot, remaining: context.pages()[0] === page });

          // External close can leave more cached IDs than the live Page cap.
          const retired = [];
          for (let index = 0; index < 20; index++) {
            const child = await context.newPage();
            const childUrl = `${url}-retired-${index}`;
            await child.goto(childUrl, { timeout: 5000 });
            const owned = (await session.send('Target.getTargets')).targetInfos.filter(target => target.browserContextId === owner && target.url === childUrl);
            assert.equal(owned.length, 1);
            retired.push(child);
            await session.send('Target.closeTarget', { targetId: owned[0].targetId });
          }
          await settled(() => context.pages().length === 1);
          observations.push({ id: 'retired-cache', count: context.pages().length, same: context.pages()[0] === page,
            retained: retired.every(child => child.context() === context) });
        } finally {
          await diagnosticDeadline(context.close(), 'Page enumeration Context cleanup');
          if (owner) {
            assert.ok(!(await session.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
            assert.ok(!(await session.send('Target.getBrowserContexts')).browserContextIds.includes(owner));
          }
        }
        observations.push({ id: 'closed-context', pages: context.pages().length, registered: browser.contexts().includes(context) });
      }
      try { await workload(); }
      finally { await diagnosticDeadline(browser.close(), 'Page enumeration candidate/oracle disconnect'); }
      let metrics = null;
      if (label === 'candidate') {
        await settled(() => { globalThis.gc(); return browser.pwrightFrameTreeMetrics().states === 1; });
        metrics = { tree: browser.pwrightFrameTreeMetrics(), host: browser.pwrightMetrics(), urls: browser.pwrightMetadataMetrics() };
        // The closed Browser still owns its fixed metadata-only publication
        // port. Its single lease lasts with the wrapper; callbacks and native
        // groups are already retired. No browser runtime is retained by it.
        assert.equal(metrics.tree.revisions, 1);
        assert.equal(metrics.urls.entries, 0);
        assert.equal(metrics.host.activeLeases, 0);
        assert.equal(metrics.host.activeOperations, 0);
      }
      return { observations, metrics };
    }
    oracle = await collect(upstream, 'oracle');
    candidate = await collect(await import('../src/index.mjs'), 'candidate');
  } finally { await session.detach(); }
} finally { await diagnosticDeadline(observer.close(), 'Page enumeration observer disconnect'); }
const cases = oracle.observations.map((expected, index) => {
  const actual = candidate.observations[index];
  let match = false;
  try { assert.deepEqual(actual, expected); match = true; } catch { /* Preserve discrepancies in the report. */ }
  return { id: expected.id, match, oracle: expected, candidate: actual };
});
const artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
const encoded = JSON.stringify({ artifact, cases, metrics: candidate.metrics }, null, 2);
assert.ok(Buffer.byteLength(encoded) < 128 * 1024);
await writeFile(output, encoded, { flag: 'wx' });
for (const sample of cases) console.log(`${sample.match ? 'MATCH' : 'FAIL'} ${sample.id}`);
assert.equal(cases.filter(sample => sample.match).length, cases.length);
console.log(`Public Page enumeration: ${cases.length}/${cases.length}; native wrappers reclaimed; owned targets absent`);
