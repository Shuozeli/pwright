// Actual native boundary, not public page.frame conformance. One owned blank Page
// supplies the real metadata ledger; no static fixture/server is needed.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join, isAbsolute } from 'node:path';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { globBaseCases } from './glob-base-cases.mjs';
import { diagnosticDeadline } from './probe-deadline.mjs';
import artifactConfig from '../src/native-artifact.cjs';
import { chromium } from '../src/index.mjs';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { resolveGlobToRegexPattern } = requireCore(join(core, 'lib/utils/isomorphic/urlMatch.js'));
const upstream = requireOracle('playwright');
const endpoint = process.env.PWRIGHT_CDP_ENDPOINT;
const output = process.env.PWRIGHT_GLOB_FFI_REPORT;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(output && isAbsolute(output) && typeof globalThis.gc === 'function');
const report = { scope: 'native-glob-conversion-port', oracle: 'playwright@1.58.2', total: 0, mismatches: [], checks: [] };
const observerSessionOrder = process.env.PWRIGHT_GLOB_OBSERVER_SESSION ?? 'before';
assert.ok(['before', 'after'].includes(observerSessionOrder));
report.observerSessionOrder = observerSessionOrder;
report.artifact = createHash('sha256').update(await readFile(artifactConfig.nativeArtifact.path)).digest('hex');
let browser, context, tree, observer, control, owner;
try {
  observer = await upstream.chromium.connectOverCDP(endpoint, { timeout: 5000 });
  if (observerSessionOrder === 'before') control = await observer.newBrowserCDPSession();
  browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
  if (observerSessionOrder === 'after') control = await observer.newBrowserCDPSession();
  context = await browser.newContext();
  let page = await context.newPage();
  const marker = `about:blank#pwright-glob-ffi-${process.pid}`;
  await page.goto(marker, { timeout: 5000 });
  const owned = (await control.send('Target.getTargets')).targetInfos.filter(target => target.url === marker);
  assert.equal(owned.length, 1);
  owner = owned[0].browserContextId;
  assert.equal(typeof owner, 'string');
  tree = page._frameCache._tree;
  assert.equal(typeof tree.globPattern, 'function');
  const before = browser.pwrightFrameTreeMetrics();
  report.processMemoryBefore = process.memoryUsage();
  const cases = globBaseCases();
  assert.ok(cases.length <= 4096);
  for (const sample of cases) {
    const expected = resolveGlobToRegexPattern(sample.baseURL, sample.glob);
    const actual = tree.globPattern(sample.glob, sample.baseURL);
    if (actual !== expected) report.mismatches.push({ ...sample, expected, actual });
  }
  report.total = cases.length;
  assert.deepEqual(browser.pwrightFrameTreeMetrics(), before);
  report.checks.push('1647 calls release all temporary native charges');

  const saved = globalThis.URL;
  let calls = 0;
  try {
    globalThis.URL = class { constructor(...args) { calls++; return new saved(...args); } };
    assert.equal(tree.globPattern('**/a', 'invalid'), resolveGlobToRegexPattern('invalid', '**/a'));
    assert.equal(tree.globPattern('about:blank'), '^about:blank$');
    assert.equal(calls, 0);
    assert.throws(() => tree.globPattern('x'.repeat(16385)), /PWRIGHT_RESOURCE_EXHAUSTED/);
    assert.throws(() => tree.globPattern('x', 'x'.repeat(16385)), /PWRIGHT_RESOURCE_EXHAUSTED/);
    assert.throws(() => tree.globPattern('\ud800'), /PWRIGHT_UNSUPPORTED/);
    assert.equal(calls, 0, 'invalid input must reject before invoking foreign URL code');
    report.checks.push('bypass and pre-parser input rejection');

    const thrown = { sentinel: true };
    globalThis.URL = class { constructor() { throw thrown; } };
    assert.equal(tree.globPattern('a/$&/b'), resolveGlobToRegexPattern(undefined, 'a/$&/b'));
    report.checks.push('caught resolver exception preserves pinned fallback');

    globalThis.URL = class { toString() { return 'x'.repeat(262145); } get origin() { return 'null'; } };
    assert.throws(() => tree.globPattern('a'), /PWRIGHT_RESOURCE_EXHAUSTED/);
    report.checks.push('oversized foreign output rejected before native copy');

    let depth = 0, peak = 0, exhaustion = false;
    const input = 'a'.repeat(16384);
    globalThis.URL = class {
      constructor(...args) {
        depth++; peak = Math.max(peak, depth);
        try {
          assert.ok(depth < 128, 'nested conversion must exhaust before unbounded recursion');
          tree.globPattern(input, 'https://a.test/');
        } catch (error) {
          if (/PWRIGHT_RESOURCE_EXHAUSTED/.test(String(error))) exhaustion = true;
          else throw error;
        } finally { depth--; }
        return new saved(...args);
      }
    };
    try { tree.globPattern(input, 'https://a.test/'); }
    catch (error) { assert.match(String(error), /PWRIGHT_RESOURCE_EXHAUSTED/); }
    assert.equal(exhaustion, true);
    assert.ok(peak < 128);
    report.recursionPeak = peak;
    report.checks.push('nested synchronous conversions share admission and fully roll back');
  } finally { globalThis.URL = saved; }
  assert.deepEqual(browser.pwrightFrameTreeMetrics(), before);
  await diagnosticDeadline(context.close(), 'glob Context close');
  context = null; page = null;
  assert.ok(!(await control.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
  assert.ok(!(await control.send('Target.getBrowserContexts')).browserContextIds.includes(owner));
  await diagnosticDeadline(browser.close(), 'glob disconnect');
  assert.equal(tree.globPattern('a', 'https://a.test/'), '^https://a\\.test/a$');
  report.checks.push('retained closed tree works without runtime or native targets');
  tree = null;
  for (let attempt = 0; attempt < 100; attempt++) {
    globalThis.gc();
    if (browser.pwrightFrameTreeMetrics().states === 1) break;
    await delay(20);
  }
  report.metrics = { tree: browser.pwrightFrameTreeMetrics(), host: browser.pwrightMetrics(), urls: browser.pwrightMetadataMetrics() };
  report.processMemoryAfter = process.memoryUsage();
  assert.equal(report.metrics.tree.states, 1);
  assert.equal(report.metrics.tree.revisions, 0);
  assert.equal(report.metrics.urls.entries, 0);
  assert.equal(report.metrics.host.activeLeases, 0);
  assert.equal(report.metrics.host.activeOperations, 0);
  report.checks.push('last-wrapper finalization releases native metadata');
} catch (error) {
  report.failure = String(error).slice(0, 4096);
  process.exitCode = 1;
} finally {
  try {
    if (context) await diagnosticDeadline(context.close(), 'glob failure Context cleanup');
    if (browser) await diagnosticDeadline(browser.close(), 'glob failure disconnect');
    if (control && owner) assert.ok(!(await control.send('Target.getTargets')).targetInfos.some(target => target.browserContextId === owner));
    if (control) await control.detach();
    if (observer) await diagnosticDeadline(observer.close(), 'glob observer disconnect');
  } catch (error) { report.cleanupFailure = String(error).slice(0, 4096); process.exitCode = 1; }
  const encoded = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(encoded) < 512 * 1024);
  await writeFile(output, encoded, { flag: 'wx' });
}
console.log(`${report.failure || report.cleanupFailure ? 'FAILED native' : 'Native'} glob conversion: ${report.total - report.mismatches.length}/${report.total}; ${report.checks.length} boundary checks`);
assert.equal(report.failure, undefined);
assert.equal(report.cleanupFailure, undefined);
assert.deepEqual(report.mismatches, []);
