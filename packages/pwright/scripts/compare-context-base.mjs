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
const output = process.env.PWRIGHT_CONTEXT_BASE_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
const fixture = process.env.PWRIGHT_CONTEXT_BASE_FIXTURE;
assert.ok(fixture?.startsWith('http://macbookpro-yuacx.tail8f3b66.ts.net:'));
assert.equal(typeof globalThis.gc, 'function');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const observerSessionOrder = process.env.PWRIGHT_CONTEXT_OBSERVER_SESSION ?? 'before';
assert.ok(['before', 'after'].includes(observerSessionOrder));
const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
const owners = new Set();
const report = { artifact: createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex'), observerSessionOrder, cases: [], boundaries: [] };
let session;
async function settled(predicate) {
  const end = performance.now() + 5000;
  while (!predicate()) { assert.ok(performance.now() < end, 'Context reclamation deadline'); await delay(10); }
}
async function collect(api, label) {
  const browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
  const candidate = label === 'candidate';
  const observations = [];
  const baseline = candidate ? browser.pwrightContextMetadataMetrics() : null;
  let retainedSettings;
  try {
    if (!session) session = await diagnosticDeadline(observer.newBrowserCDPSession(), 'Late observer session');
    async function workload() {
      const cases = [
        { id: 'directory-relative', baseURL: fixture, url: './empty.html#relative' },
        { id: 'file-relative', baseURL: `${fixture}content.html`, url: './empty.html#file' },
        { id: 'query-relative', baseURL: `${fixture}empty.html`, url: '?value=hello world#query' },
        { id: 'opaque-hash', baseURL: 'about:blank', url: '#opaque' },
        { id: 'empty-base', baseURL: '', url: 'about:blank#empty' },
        { id: 'invalid-base', baseURL: 'invalid base', url: 'about:blank#invalid' },
        { id: 'absent-base', url: 'about:blank#absent' },
      ];
      for (const spec of cases) {
        const context = await browser.newContext(spec.baseURL === undefined ? {} : { baseURL: spec.baseURL });
        try {
          const page = await context.newPage();
          await page.goto(spec.url, { timeout: 5000 });
          const targets = (await session.send('Target.getTargets')).targetInfos.filter(value => value.url === page.url());
          const owned = targets.find(value => value.browserContextId);
          assert.ok(owned, 'created Page must have native Context ownership');
          owners.add(owned.browserContextId);
          observations.push({ id: spec.id, url: page.url(), same: browser.contexts().includes(context), pageOwner: page.context() === context });
          if (candidate) {
            const before = browser.pwrightContextMetadataMetrics();
            for (let index = 0; index < 2000; index++) assert.ok(browser.contexts().includes(context));
            assert.deepEqual(browser.pwrightContextMetadataMetrics(), before, 'cached Context enumeration consumes no new native binding');
          }
        } finally { await diagnosticDeadline(context.close(), 'Context baseURL cleanup'); }
        observations.push({ id: `${spec.id}-closed`, listed: browser.contexts().includes(context), pages: context.pages().length });
      }
      // Enumerate while two native creations are pending. Every observed owned
      // wrapper must be the one returned by creation, with its original base.
      const existing = new Set(browser.contexts());
      const creating = [browser.newContext({ baseURL: 'about:blank' }), browser.newContext({ baseURL: `${fixture}empty.html` })];
      let pending = true;
      const combined = Promise.all(creating).finally(() => { pending = false; });
      const seen = new Set();
      while (pending) { for (const context of browser.contexts()) seen.add(context); await delay(1); }
      const contexts = await combined;
      try {
        for (const observed of seen) {
          if (!existing.has(observed)) assert.ok(contexts.includes(observed), 'racing enumeration must reuse the created wrapper');
        }
        for (let index = 0; index < contexts.length; index++) {
          const context = contexts[index];
          assert.ok(browser.contexts().includes(context));
          const page = await context.newPage();
          await page.goto(`#isolated-${index}`, { timeout: 5000 });
          const target = (await session.send('Target.getTargets')).targetInfos.find(value => value.url === page.url() && value.browserContextId);
          assert.ok(target); owners.add(target.browserContextId);
          observations.push({ id: `concurrent-${index}`, url: page.url(), cached: browser.contexts().includes(context) });
        }
      } finally { for (const context of contexts) await diagnosticDeadline(context.close(), 'Concurrent Context cleanup'); }
    }
    await workload();
    if (candidate) {
      await settled(() => { globalThis.gc(); return browser.pwrightContextMetadataMetrics().states <= 1; });
      const before = browser.pwrightContextMetadataMetrics();
      const nativeBefore = await session.send('Target.getBrowserContexts');
      await assert.rejects(browser.newContext({ baseURL: 42 }), TypeError);
      await assert.rejects(browser.newContext({ baseURL: 'x'.repeat(16 * 1024 + 1) }), TypeError);
      await assert.rejects(browser.newContext({ viewport: { width: 800, height: 600 } }), /Unsupported/);
      assert.deepEqual(browser.pwrightContextMetadataMetrics(), before);
      assert.deepEqual(await session.send('Target.getBrowserContexts'), nativeBefore);
      report.boundaries.push({ id: 'invalid-and-unsupported-options-before-native-creation' });
      const retained = [];
      try {
        for (let index = 0; index < 130; index++) {
          let context;
          try { context = await browser.newContext({ baseURL: 'about:blank' }); }
          catch (error) { assert.equal(error.code, 'PWRIGHT_RESOURCE_EXHAUSTED'); break; }
          retained.push(context);
          await context.close();
        }
        assert.ok(retained.length > 0 && retained.length < 130);
        const nativeBefore = await session.send('Target.getBrowserContexts');
        await assert.rejects(browser.newContext({ baseURL: 'about:blank' }), { code: 'PWRIGHT_RESOURCE_EXHAUSTED' });
        assert.deepEqual(await session.send('Target.getBrowserContexts'), nativeBefore, 'metadata exhaustion must precede native creation');
        const before = browser.pwrightContextMetadataMetrics();
        assert.equal(before.activeContexts, 0); assert.equal(before.pendingCreations, 0); assert.equal(before.conversions, 0);
        // Internal boundary check, not public Playwright API conformance.
        assert.equal(retained[0]._settings.resolveUrl('#retained'), 'about:blank#retained');
        retainedSettings = retained[0]._settings;
        assert.deepEqual(browser.pwrightContextMetadataMetrics(), before);
        report.boundaries.push({ id: 'retained-settings-and-pre-native-exhaustion', retained: retained.length, metrics: before });
        const OriginalURL = globalThis.URL;
        let depth = 0, peak = 0, exhausted = false;
        try {
          globalThis.URL = class {
            constructor() { throw new Error('URL constructor sentinel'); }
          };
          assert.equal(retainedSettings.resolveUrl('#fallback'), '#fallback');
          globalThis.URL = class {
            toString() { return 'x'.repeat(16 * 1024 + 1); }
            get origin() { return 'https://example.test'; }
          };
          assert.throws(() => retainedSettings.resolveUrl('#oversized'), /PWRIGHT_INVALID_ARGUMENT/);
          globalThis.URL = class extends OriginalURL {
            constructor(input, base) {
              depth++; peak = Math.max(peak, depth);
              try {
                assert.ok(depth < 128, 'native conversion admission must bound reentry');
                try { retainedSettings.resolveUrl('#nested'); }
                catch (error) { assert.match(String(error), /PWRIGHT_RESOURCE_EXHAUSTED/); exhausted = true; }
              } finally { depth--; }
              super(input, base);
            }
          };
          assert.equal(retainedSettings.resolveUrl('#reentry'), 'about:blank#reentry');
          assert.equal(exhausted, true);
          assert.ok(peak > 1 && peak < 128);
        } finally { globalThis.URL = OriginalURL; }
        assert.deepEqual(browser.pwrightContextMetadataMetrics(), before, 'foreign failures and reentry release all conversion leases');
        report.boundaries.push({ id: 'foreign-url-fallback-output-bound-and-reentry', peak });
      } finally { retained.length = 0; }
    }
  } finally {
    await diagnosticDeadline(browser.close(), `${label} disconnect`);
    if (observerSessionOrder === 'after' && session) { await session.detach(); session = null; }
  }
  if (candidate) {
    assert.equal(retainedSettings.resolveUrl('#disconnected'), 'about:blank#disconnected');
    retainedSettings = undefined;
    await settled(() => { globalThis.gc(); return browser.pwrightContextMetadataMetrics().states === 0; });
    const final = browser.pwrightContextMetadataMetrics();
    assert.deepEqual(final, baseline);
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    report.boundaries.push({ id: 'last-reference-reclamation', metrics: final, host: browser.pwrightMetrics(), tree: browser.pwrightFrameTreeMetrics() });
  }
  return observations;
}
try {
  if (observerSessionOrder === 'before') session = await observer.newBrowserCDPSession();
  const oracle = await collect(upstream, 'oracle');
  report.oracle = oracle;
  const candidate = await collect(await import('../src/index.mjs'), 'candidate');
  report.cases = oracle.map((expected, index) => ({ id: expected.id, oracle: expected, candidate: candidate[index], match: JSON.stringify(expected) === JSON.stringify(candidate[index]) }));
  assert.equal(candidate.length, oracle.length);
  assert.ok(report.cases.every(value => value.match));
  if (!session) session = await observer.newBrowserCDPSession();
  const remaining = await session.send('Target.getBrowserContexts');
  for (const owner of owners) assert.ok(!remaining.browserContextIds.includes(owner), 'owned Context must be absent');
} catch (error) { report.error = String(error.stack ?? error); throw error; }
finally {
  try { if (session) await session.detach(); }
  finally {
    await diagnosticDeadline(observer.close(), 'Context observer disconnect');
    const encoded = JSON.stringify(report, null, 2);
    assert.ok(Buffer.byteLength(encoded) < 128 * 1024);
    await writeFile(output, encoded, { flag: 'wx' });
  }
}
console.log(`Context baseURL: ${report.cases.length}/${report.cases.length}; ${report.boundaries.length} boundary checks`);
