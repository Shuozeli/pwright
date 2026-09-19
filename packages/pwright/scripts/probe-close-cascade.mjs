// Bounded diagnostic: optional explicit retirement hold, no production timeout or browser launch.
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
import { withCdpCommandRelay } from '../test/cdp-command-relay.mjs';
import { CloseTrace } from '../test/native-close-trace.mjs';
import { HeldRetirementTrace } from '../test/held-retirement-trace.mjs';
import { diagnosticDeadline } from './probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const oracle = require('playwright').chromium;
const endpoint = process.env.PWRIGHT_TEST_CDP;
const bind = process.env.PWRIGHT_TEST_BIND;
const host = process.env.PWRIGHT_TEST_HOST;
const output = process.env.PWRIGHT_CLOSE_REPORT;
const side = process.env.PWRIGHT_CLOSE_SIDE;
const control = process.env.PWRIGHT_CLOSE_CONTROL ?? 'transparent';
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(bind && host?.endsWith('.ts.net'));
assert.ok(output && isAbsolute(output));
assert.ok(['oracle', 'candidate'].includes(side));
assert.ok(['transparent', 'hold-retirement'].includes(control));
if (side === 'candidate') assert.ok(artifact.nativeFeatures(artifact.nativeArtifact.profile).includes('frame-navigation'));
const type = side === 'candidate' ? candidate : oracle;
const errorValue = error => ({ code: error?.code, message: String(error?.message ?? error).slice(0, 4096) });
const report = { schema: 'pwright.close-cascade-probe.v1', side, control, profile: artifact.nativeArtifact.profile,
  sha256: side === 'candidate' ? createHash('sha256').update(await readFile(artifact.nativeArtifact.path)).digest('hex') : undefined,
  cases: [] };

async function run(pageFirst) {
  const started = performance.now();
  const Trace = control === 'hold-retirement' ? HeldRetirementTrace : CloseTrace;
  const trace = new Trace(() => performance.now(), method => method.startsWith('Target.') || ['Page.navigate', 'Page.stopLoading'].includes(method));
  const record = { pageFirst, events: [], errors: [], settlement: {}, phases: [] };
  let probeBrowser;
  report.cases.push(record);
  const mark = phase => {
    assert.ok(record.phases.length < 16);
    record.phases.push({ phase, atMs: performance.now() - started, receipts: trace.receipts.length,
      settled: Object.keys(record.settlement),
      heldRetirements: trace.heldCount?.() ?? 0,
      activeOperations: probeBrowser?.pwrightMetrics?.().activeOperations,
      activeFfiCloses: probeBrowser?.pwrightFfiMetrics?.().closes,
      disposalSent: trace.completed.concat(trace.unanswered()).some(value => value.method === 'Target.disposeBrowserContext'),
      destroyed: trace.lifecycle.filter(value => value.method === 'Target.targetDestroyed' && trace.created.includes(value.target)).length });
  };
  const settle = (name, promise) => promise.then(() => {
    record.settlement[name] = { ok: true, atMs: performance.now() - started };
  }, error => { record.settlement[name] = { error: errorValue(error), atMs: performance.now() - started }; });
  let signalArrival;
  const arrival = new Promise(resolve => { signalArrival = resolve; });
  const server = createServer((request, response) => {
    if (request.url !== '/held') { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
    const content = `<title>held Frame</title><!--${' '.repeat(2048)}-->`;
    if (control === 'hold-retirement') response.end(content);
    else response.write(content);
    signalArrival();
  });
  server.listen(0, bind);
  await once(server, 'listening');
  let pendingPage, pendingContext, pendingNavigation;
  try {
    await withCdpCommandRelay(endpoint, trace, async relay => {
      const browser = await type.connectOverCDP(relay, { timeout: 5000 });
      probeBrowser = browser;
      try {
        const context = await browser.newContext();
        const page = await context.newPage();
        for (const [owner, name] of [[page, 'page'], [context, 'context'], [browser, 'browser']]) {
          owner.on(name === 'browser' ? 'disconnected' : 'close', function(value) {
            assert.ok(record.events.length < 8);
            record.events.push({ name, atMs: performance.now() - started, sameArgument: value === owner,
              sameReceiver: this === owner, closed: page.isClosed(), connected: browser.isConnected() });
          });
        }
        pendingNavigation = settle('navigation', page.mainFrame().goto(`http://${host}:${server.address().port}/held`, { timeout: 5000 }));
        await diagnosticDeadline(arrival, 'response fragment queued');
        if (control === 'hold-retirement') {
          await pendingNavigation;
          assert.equal(record.settlement.navigation.ok, true);
        }
        mark(control === 'hold-retirement' ? 'response-completed' : 'response-fragment-queued');
        if (pageFirst) {
          if (control === 'hold-retirement') trace.holding = true;
          pendingPage = settle('page', page.close());
          await diagnosticDeadline((async () => {
            while (!trace.receipts.some(value => value.method === 'Target.closeTarget') && !record.settlement.page) {
              trace.check(); await setTimeout(10);
            }
          })(), 'Page close response observed');
          if (control === 'hold-retirement') {
            await diagnosticDeadline((async () => {
              while (!trace.heldCount()) { trace.check(); await setTimeout(10); }
            })(), 'exact native Page retirement held');
          }
          mark('page-close-response-observed');
          record.pendingAtContextStart = !record.settlement.page;
        }
        pendingContext = settle('context', context.close());
        if (pageFirst && control === 'hold-retirement') {
          assert.equal(record.pendingAtContextStart, true, 'controlled case must keep Page.close pending');
          try {
            await diagnosticDeadline((async () => {
              while (!trace.completed.concat(trace.unanswered()).some(value => value.method === 'Target.disposeBrowserContext')) {
                trace.check(); await setTimeout(10);
              }
            })(), 'Context disposal dispatch before releasing Page retirement');
          } catch (error) { record.errors.push({ phase: 'ancestor native dispatch', ...errorValue(error) }); }
          mark('ancestor-dispatch-observed-or-deadline');
          trace.release();
        }
        try { await diagnosticDeadline(pendingContext, 'Context close while response held'); }
        catch (error) { record.errors.push({ phase: 'Context close observation', ...errorValue(error) }); }
        mark('context-close-observed-or-deadline');
      } catch (error) {
        record.errors.push({ phase: 'setup or operation', ...errorValue(error) });
      } finally {
        trace.release();
        // Preserve primary evidence independently of cleanup errors. Do not
        // await the same blocked Context again before interrupting its Browser.
        try { await diagnosticDeadline(browser.close(), 'Browser cleanup'); }
        catch (error) { record.errors.push({ phase: 'Browser cleanup', ...errorValue(error) }); }
        mark('browser-cleanup-observed');
        for (const [name, pending] of [['Page', pendingPage], ['Context', pendingContext], ['navigation', pendingNavigation]]) {
          if (!pending) continue;
          try { await diagnosticDeadline(pending, `${name} original pending operation retirement`); }
          catch (error) { record.errors.push({ phase: `${name} retirement`, ...errorValue(error) }); }
        }
        if (browser.pwrightMetrics) {
          for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
          record.resources = { host: browser.pwrightMetrics(), ffi: browser.pwrightFfiMetrics() };
          assert.equal(record.resources.host.activeLeases, 0);
          assert.equal(record.resources.host.cleanupInFlight, 0);
          assert.equal(record.resources.ffi.pending, 0);
        }
      }
    });
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    record.trace = trace.report();
  }
  const targets = await fetch(`${endpoint}/json/list`, { signal: AbortSignal.timeout(5000) }).then(response => response.json());
  record.ownedRemaining = targets.filter(value => trace.created.includes(value.id)).map(value => value.id);
  assert.deepEqual(record.ownedRemaining, [], 'do not add another case while an owned target remains');
  // A diagnostic with an observed command is not a successful close gate.
  // Verify public settlement and ordering as well as native resource retirement.
  assert.equal(record.settlement.context?.ok, true, 'Context.close must fulfill');
  if (pageFirst) assert.equal(record.settlement.page?.ok, true, 'Page.close must fulfill');
  assert.deepEqual(record.events.map(({ name, sameArgument, sameReceiver, closed, connected }) =>
    ({ name, sameArgument, sameReceiver, closed, connected })), [
    { name: 'page', sameArgument: true, sameReceiver: true, closed: true, connected: true },
    { name: 'context', sameArgument: true, sameReceiver: true, closed: true, connected: true },
    { name: 'browser', sameArgument: true, sameReceiver: true, closed: true, connected: false },
  ]);
  process.stdout.write(`${JSON.stringify({ side, pageFirst, errors: record.errors, settlement: record.settlement, phases: record.phases })}\n`);
  if (record.errors.length) process.exitCode = 1;
}

try {
  for (const pageFirst of [false, true]) await run(pageFirst);
} catch (error) {
  report.failure = errorValue(error);
  process.exitCode = 1;
} finally {
  const encoded = JSON.stringify(report, null, 2);
  assert.ok(Buffer.byteLength(encoded) <= 1024 * 1024);
  await writeFile(output, encoded, { flag: 'wx' });
}
