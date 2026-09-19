// Same public lookup workload on the installed version-pinned oracle and addon.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import vm from 'node:vm';
import artifactConfig from '../src/native-artifact.cjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');
const fixture = new URL(process.env.PWRIGHT_FRAME_LOOKUP_FIXTURE);
assert.equal(fixture.protocol, 'http:');
assert.ok(fixture.hostname.endsWith('.ts.net'));
const output = process.env.PWRIGHT_FRAME_LOOKUP_REPORT;
assert.ok(typeof output === 'string' && isAbsolute(output));
assert.equal(typeof globalThis.gc, 'function');
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const upstream = requireOracle('playwright');
const report = { artifact: createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex'), cases: [], boundaries: [] };
const observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
const session = await observer.newBrowserCDPSession();
const owners = new Set();
async function settled(predicate) {
  const end = performance.now() + 5000;
  while (!predicate()) { assert.ok(performance.now() < end, 'Frame metadata settlement deadline'); await delay(10); }
}
async function collect(api, label) {
  // Pre-existing nested topology exercises connect-time reconstruction rather
  // than only the later frameAttached publication sequence.
  const existingContext = await observer.newContext();
  const existingUrl = new URL('frame-lookup.html', fixture);
  existingUrl.searchParams.set('lookup-existing', label);
  let browser;
  const observations = [];
  const candidate = label === 'candidate';
  try {
    const existingPage = await existingContext.newPage();
    await existingPage.goto(existingUrl.href, { timeout: 5000 });
    const targets = (await session.send('Target.getTargets')).targetInfos;
    const owner = targets.find(target => target.url === existingUrl.href && target.browserContextId)?.browserContextId;
    assert.ok(owner); owners.add(owner);
    browser = await api.chromium.connectOverCDP(endpoint, { timeout: 5000 });
    async function importedOrder() {
      const page = browser.contexts().flatMap(context => context.pages()).find(page => page.url() === existingUrl.href);
      assert.ok(page, 'pre-existing fixture Page must be imported');
      await settled(() => page.frames().length === 4);
      observations.push({ id: 'pre-existing-order', value: page.frames().map(frame => ({ name: frame.name(),
        file: new URL(frame.url()).pathname.split('/').at(-1), hash: new URL(frame.url()).hash })) });
      observations.push({ id: 'pre-existing-duplicate-first', value: page.frame('duplicate').url() });
    }
    await importedOrder();
    await diagnosticDeadline(existingContext.close(), 'Pre-existing lookup Context cleanup');
    async function workload() {
      const context = await browser.newContext({ baseURL: fixture.href });
      let page;
      try {
        page = await context.newPage();
        await page.goto('frame-lookup.html', { timeout: 5000 });
        const targets = (await session.send('Target.getTargets')).targetInfos;
        const owner = targets.find(target => target.url === page.url() && target.browserContextId)?.browserContextId;
        assert.ok(owner); owners.add(owner);
        if (candidate) {
          // Native metadata settlement only: do not create child JS wrappers
          // before testing the selection's binding footprint.
          await settled(() => page._frameCache._tree.frames().length === 4);
          const before = browser.pwrightFrameTreeMetrics();
          assert.equal(page.frame('missing'), null);
          assert.deepEqual(browser.pwrightFrameTreeMetrics(), before);
          const selected = page.frame('nested');
          assert.ok(selected);
          const after = browser.pwrightFrameTreeMetrics();
          assert.equal(after.states, before.states + 2, 'bind only nested Frame and its uncached parent');
          assert.equal(after.revisions, before.revisions);
          report.boundaries.push({ id: 'no-match-and-selected-ancestry-only', before, after });
        }
        await settled(() => page.frames().length === 4 && page.frames().every(frame => frame.url().startsWith(fixture.origin)));
        const identity = frame => frame ? { name: frame.name(), path: new URL(frame.url()).pathname.split('/').at(-1), hash: new URL(frame.url()).hash,
          cached: page.frames().includes(frame), owner: frame.page() === page,
          parent: frame.parentFrame()?.name() ?? null } : null;
        const record = (id, value) => observations.push({ id, value });
        record('initial-order', page.frames().map(identity));
        for (const [id, selector] of [
          ['duplicate-name', 'duplicate'], ['nested-name', { name: 'nested' }], ['missing-name', 'missing'],
          ['absolute-url', { url: page.url() }], ['relative-url', { url: 'empty.html#nested' }],
          ['deep-glob', { url: '**/empty.html#*' }], ['glob-alternative', { url: '**/{frame-lookup-parent,empty}.html*' }],
          ['single-star-no-slash', { url: '*empty*' }], ['regex', { url: /#nested$/ }],
          ['cross-realm-regex', { url: vm.runInNewContext('/#nested$/') }],
          ['predicate-url', { url: value => { assert.ok(value instanceof URL); return value.hash === '#nested'; } }],
          ['predicate-promise-truthiness', { url: () => Promise.resolve(false) }],
          ['falsy-name-url', { name: '', url: '**/empty.html#last' }],
          ['name-wins', { name: 'nested', url: () => { throw new Error('must not run'); } }],
          ['truthy-nonstring-name', { name: 17, url: () => { throw new Error('must not run'); } }],
        ]) record(id, identity(page.frame(selector)));
        const visited = [];
        record('short-circuit-result', identity(page.frame({ url: url => { visited.push(url.hash); return url.hash === '#nested'; } })));
        record('short-circuit-visits', visited);
        const stateful = /frame-lookup\.html$/g;
        record('stateful-regex', [page.frame({ url: stateful }), page.frame({ url: stateful }), page.frame({ url: stateful })].map(identity));
        let testReceiver;
        const custom = /unused/;
        custom.test = function (url) { testReceiver = this; return url.endsWith('#last'); };
        record('custom-regex-test', { result: identity(page.frame({ url: custom })), receiver: testReceiver === custom });
        const reads = [];
        record('getters', { result: identity(page.frame({ get name() { reads.push('name'); return 'nested'; }, get url() { reads.push('url'); return /unused/; } })), reads });
        for (const [index, thrown] of [null, undefined, 73, 'sentinel', { marker: 'identity' }, new Error('PWRIGHT_TARGET_CLOSED caller error')].entries()) {
          let caught = false;
          try { page.frame({ url: () => { throw thrown; } }); } catch (error) { caught = true; assert.equal(error, thrown); }
          record(`predicate-throw-${index}`, caught);
        }
        const sentinel = {};
        assert.throws(() => page.frame({ name: 'nested', get url() { throw sentinel; } }), error => error === sentinel);
        record('url-getter-throw', true);
        const throwingRegex = /anything/;
        Object.defineProperty(throwingRegex, 'test', { get() { throw sentinel; } });
        assert.throws(() => page.frame({ url: throwingRegex }), error => error === sentinel);
        record('regex-test-getter-throw', true);
        let malformed;
        try { page.frame({ url: '**/{unfinished' }); } catch (error) { malformed = error.name; }
        record('malformed-glob', malformed);
        assert.equal(malformed, 'SyntaxError');
        for (const [index, selector] of ['', {}, { name: '' }, { url: '' }, { url: 42 }].entries()) {
          let message;
          try { page.frame(selector); } catch (error) { message = error.message; }
          assert.equal(typeof message, 'string'); record(`invalid-selector-${index}`, message);
        }
        const nested = page.frame('nested');
        record('reentrant-identity', page.frame({ url: url => { page.frames(); page.frame('duplicate'); return url.hash === '#nested'; } }) === nested);
        if (candidate) {
          const before = browser.pwrightFrameTreeMetrics();
          for (let index = 0; index < 1000; index++) assert.equal(page.frame('nested'), nested);
          assert.deepEqual(browser.pwrightFrameTreeMetrics(), before);
          assert.throws(() => page.frame({ url: 'x'.repeat(16 * 1024 + 1) }), /PWRIGHT_RESOURCE_EXHAUSTED/);
          assert.deepEqual(browser.pwrightFrameTreeMetrics(), before);
          let depth = 0, peak = 0, exhausted = false;
          const recur = () => {
            depth++; peak = Math.max(peak, depth);
            try {
              assert.ok(depth < 200, 'native admission must bound callback recursion');
              try { page.frame({ url: recur }); } catch (error) { assert.match(String(error), /PWRIGHT_RESOURCE_EXHAUSTED/); exhausted = true; }
              return true;
            } finally { depth--; }
          };
          assert.equal(page.frame({ url: recur }), page.mainFrame());
          assert.ok(exhausted); assert.deepEqual(browser.pwrightFrameTreeMetrics(), before);
          report.boundaries.push({ id: 'cached-repeat-oversize-and-recursion-release', peak, metrics: before });
        }
        await page.evaluate(() => document.querySelector('iframe').remove());
        await settled(() => page.frames().length === 2);
        record('detached-selection', { missing: page.frame('nested') === null, detached: nested.isDetached(), next: identity(page.frame('duplicate')) });
        await page.close();
        record('closed-lookup', identity(page.frame({ url: '**/empty.html#last' })));
      } finally { await diagnosticDeadline(context.close(), 'Lookup Context cleanup'); }
      recordAfterDisconnect = () => ({ same: page.frame({ url: '**/frame-lookup.html' }) === page.mainFrame() });
    }
    let recordAfterDisconnect;
    await workload();
    await diagnosticDeadline(browser.close(), 'Lookup browser disconnect');
    observations.push({ id: 'disconnected-lookup', value: recordAfterDisconnect() });
    recordAfterDisconnect = undefined;
    if (candidate) report.candidate = observations;
    if (candidate) {
      // Disconnect retires imported external Pages too; the pre-connect live
      // tree baseline is not the post-GC baseline. The fixed index, admitted
      // queue storage and retained Node publication port remain.
      // NODE-115: the retained closed Browser keeps exactly one fixed port
      // lease; all temporary Frame bindings must still finalize.
      await settled(() => { globalThis.gc(); const value = browser.pwrightFrameTreeMetrics(); return value.states === 1 && value.revisions === 1; });
      assert.equal(browser.pwrightFrameTreeMetrics().bytes, 64600,
        'NODE-119 fixed Node metadata baseline; no Frame or ready-token lease remains');
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      report.boundaries.push({ id: 'last-wrapper-release', tree: browser.pwrightFrameTreeMetrics(), host: browser.pwrightMetrics() });
    }
  } finally {
    try { await diagnosticDeadline(existingContext.close(), 'Pre-existing lookup fallback cleanup'); }
    finally { if (browser?.isConnected()) await diagnosticDeadline(browser.close(), 'Failed lookup disconnect'); }
  }
  return observations;
}
try {
  report.oracle = await collect(upstream, 'oracle');
  const actual = await collect(await import('../src/index.mjs'), 'candidate');
  report.cases = report.oracle.map((expected, index) => ({ id: expected.id, oracle: expected.value, candidate: actual[index]?.value, match: JSON.stringify(expected) === JSON.stringify(actual[index]) }));
  assert.equal(actual.length, report.oracle.length);
  assert.ok(report.cases.every(value => value.match), 'public Frame lookup discrepancy');
  const remaining = await session.send('Target.getBrowserContexts');
  for (const owner of owners) assert.ok(!remaining.browserContextIds.includes(owner));
} catch (error) { report.error = String(error.stack ?? error); throw error; }
finally {
  try { await session.detach(); } finally {
    await diagnosticDeadline(observer.close(), 'Lookup observer disconnect');
    const encoded = JSON.stringify(report, null, 2);
    assert.ok(Buffer.byteLength(encoded) < 256 * 1024);
    await writeFile(output, encoded, { flag: 'wx' });
  }
}
console.log(`Frame lookup: ${report.cases.length}/${report.cases.length}; ${report.boundaries.length} boundary checks`);
