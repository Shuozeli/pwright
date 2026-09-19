// Controlled fixture-body release, not a production workaround or timeout change.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { isAbsolute } from 'node:path';
import { setImmediate, setTimeout } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import artifact from '../src/native-artifact.cjs';
import { CloseTrace } from '../test/native-close-trace.mjs';
import { withCdpCommandRelay } from '../test/cdp-command-relay.mjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const side = process.env.PWRIGHT_CLOSE_SIDE;
const endpoint = process.env.PWRIGHT_TEST_CDP;
const bind = process.env.PWRIGHT_TEST_BIND;
const host = process.env.PWRIGHT_TEST_HOST;
const output = process.env.PWRIGHT_CLOSE_REPORT;
assert.ok(['oracle', 'candidate'].includes(side));
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(bind && host?.endsWith('.ts.net'));
assert.ok(output && isAbsolute(output));
assert.ok(artifact.nativeFeatures(artifact.nativeArtifact.profile).includes('frame-navigation'));
const type = side === 'candidate' ? candidate : require('playwright').chromium;
const errorValue = error => ({ code: error?.code, message: String(error?.message ?? error).slice(0, 4096) });
const report = { schema: 'pwright.page-close-body-probe.v1', side, profile: artifact.nativeArtifact.profile,
  sha256: side === 'candidate' ? createHash('sha256').update(await readFile(artifact.nativeArtifact.path)).digest('hex') : undefined,
  cases: [] };

async function run(boundary) {
  const started = performance.now();
  const trace = new CloseTrace(() => performance.now(), method => method.startsWith('Target.') ||
    ['Page.navigate', 'Page.stopLoading'].includes(method));
  const record = { boundary, settlement: {}, phases: [], errors: [] };
  report.cases.push(record);
  let held;
  let signalArrival;
  const arrival = new Promise(resolve => { signalArrival = resolve; });
  const server = createServer({ maxHeaderSize: 8192 }, (request, response) => {
    if (request.url !== '/held') { response.writeHead(404).end(); return; }
    assert.equal(held, undefined, 'one exact navigation, no retries');
    held = response;
    response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
    response.write(`<title>held Frame</title><!--${' '.repeat(2048)}-->`);
    signalArrival();
  });
  server.maxConnections = 8;
  server.listen(0, bind);
  await once(server, 'listening');
  const url = `http://${host}:${server.address().port}/held`;
  const mark = phase => {
    assert.ok(record.phases.length < 8);
    record.phases.push({ phase, atMs: performance.now() - started,
      settled: Object.keys(record.settlement), bodyDestroyed: held?.destroyed,
      bodyEnded: held?.writableEnded, nativeCloses: trace.receipts.filter(r => r.method === 'Target.closeTarget').length,
      destroyed: trace.lifecycle.filter(r => r.method === 'Target.targetDestroyed' && trace.created.includes(r.target)).length,
      contextDisposals: trace.completed.concat(trace.unanswered()).filter(r => r.method === 'Target.disposeBrowserContext').length });
  };
  const settle = (name, promise) => promise.then(() => {
    record.settlement[name] = { ok: true, atMs: performance.now() - started };
  }, error => { record.settlement[name] = { error: errorValue(error), atMs: performance.now() - started }; });
  const waitFor = predicate => diagnosticDeadline((async () => {
    while (!predicate()) { trace.check(); await setTimeout(5); }
  })(), 'native Page close/body phase');
  try {
    await withCdpCommandRelay(endpoint, trace, async relay => {
      const browser = await type.connectOverCDP(relay, { timeout: 5000 });
      let context, pendingPage, pendingNavigation;
      try {
        context = await browser.newContext();
        const page = await context.newPage();
        pendingNavigation = settle('navigation', page.mainFrame().goto(url, { timeout: 5000 }));
        await diagnosticDeadline(arrival, 'response fragment arrival');
        if (boundary === 'committed') {
          await waitFor(() => page.url() === url || record.settlement.navigation);
          assert.equal(page.url(), url);
          assert.equal(record.settlement.navigation, undefined);
        }
        mark('close-start');
        pendingPage = settle('page', page.close());
        await waitFor(() => trace.receipts.some(r => r.method === 'Target.closeTarget'));
        mark('native-close-replied');
        // A bounded diagnostic observation interval, not an API timeout/assertion.
        await setTimeout(200);
        mark('before-body-release');
        assert.equal(record.phases.at(-1).contextDisposals, 0, 'no ancestor close may unblock this observation');
        held.end();
        mark('body-release-requested');
        await diagnosticDeadline(pendingPage, 'Page close after fixture body release');
        await diagnosticDeadline(pendingNavigation, 'navigation after fixture body release');
        mark('page-settled');
        assert.equal(record.settlement.page.ok, true);
      } catch (error) { record.errors.push({ phase: 'operation', ...errorValue(error) }); }
      finally {
        held?.end();
        try { if (context) await diagnosticDeadline(context.close(), 'owned Context cleanup'); }
        catch (error) { record.errors.push({ phase: 'Context cleanup', ...errorValue(error) }); }
        try { await diagnosticDeadline(browser.close(), 'Browser cleanup'); }
        catch (error) { record.errors.push({ phase: 'Browser cleanup', ...errorValue(error) }); }
        for (const pending of [pendingPage, pendingNavigation]) if (pending) {
          try { await diagnosticDeadline(pending, 'original operation cleanup'); }
          catch (error) { record.errors.push({ phase: 'operation cleanup', ...errorValue(error) }); }
        }
        if (browser.pwrightMetrics) {
          for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
          record.resources = { host: browser.pwrightMetrics(), ffi: browser.pwrightFfiMetrics() };
          for (const [key, value] of Object.entries(record.resources.host)) assert.equal(value, key === 'connected' ? false : 0);
          for (const value of Object.values(record.resources.ffi)) assert.equal(value, 0);
        }
      }
    });
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    record.trace = trace.report();
  }
  const targets = await fetch(`${endpoint}/json/list`, { signal: AbortSignal.timeout(5000) }).then(r => r.json());
  record.ownedRemaining = targets.filter(t => trace.created.includes(t.id)).map(t => t.id);
  assert.deepEqual(record.ownedRemaining, []);
  process.stdout.write(`${JSON.stringify({ side, boundary, errors: record.errors, settlement: record.settlement, phases: record.phases })}\n`);
  if (record.errors.length) process.exitCode = 1;
}

try { for (const boundary of ['arrival', 'committed']) await run(boundary); }
catch (error) { report.failure = errorValue(error); process.exitCode = 1; }
finally {
  const encoded = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(encoded) <= 512 * 1024);
  await writeFile(output, encoded, { flag: 'wx' });
}
