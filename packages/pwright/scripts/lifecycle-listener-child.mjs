// Isolated diagnostic process: expected listener exceptions must never install
// process-level handlers in the test runner or production package.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { setImmediate, setTimeout } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { listenerEndpoint } from './listener-endpoint.mjs';

// Independent child limit also survives termination of the outer test runner.
// It does not keep an otherwise clean process alive or change an API deadline.
const hardDeadline = globalThis.setTimeout(() => {
  process.stderr.write('isolated listener child exceeded 14 seconds\n');
  process.exit(124);
}, 14_000);
hardDeadline.unref();

const [implementation, scenario, errorMode = 'capture'] = process.argv.slice(2);
assert.ok(['capture', 'unhandled'].includes(errorMode));
assert.ok(errorMode === 'capture' || scenario === 'disconnect-throw');
assert.ok(['reference', 'candidate'].includes(implementation));
assert.ok(['mutation', 'async-gate', 'close-reentry', 'popup-throw', 'async-reject', 'context-close-page-throw', 'browser-close-page-throw', 'browser-close-page-throw-object', 'browser-close-page-throw-primitive', 'browser-close-page-async-reject', 'browser-close-page-reentry', 'browser-close-context-throw', 'disconnect-throw', 'disconnect-throw-object', 'disconnect-throw-primitive', 'disconnect-throw-reentry', 'disconnect-async-reject'].includes(scenario));
const endpoint = listenerEndpoint(process.env.PWRIGHT_TEST_CDP, process.env.PWRIGHT_LISTENER_DIAGNOSTIC_RELAY);
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const type = implementation === 'reference' ? require('playwright').chromium : candidate;
const sentinel = ['disconnect-throw-object', 'browser-close-page-throw-object'].includes(scenario) ? Object.freeze({ marker: 'node110' })
  : ['disconnect-throw-primitive', 'browser-close-page-throw-primitive'].includes(scenario) ? 110 : new Error('node110 expected listener failure');
const errors = [];
let exceptionReentry;
function receiveError(kind, error) {
  if (error !== sentinel) throw error; // Unrelated failures remain fatal.
  assert.ok(errors.length < 4, 'bounded expected error receipts');
  errors.push({ kind, sameError: true });
  if (scenario === 'disconnect-throw-reentry') {
    exceptionReentry = browser.close();
    record('exception-handler-returned');
  }
}
const onException = error => receiveError('uncaughtException', error);
const onRejection = error => receiveError('unhandledRejection', error);
if (errorMode === 'capture') {
  process.on('uncaughtException', onException);
  process.on('unhandledRejection', onRejection);
}

const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
let context;
const pages = [], trace = [], removals = [];
function remember(page) { if (!pages.includes(page)) pages.push(page); }
function snapshot() {
  return { connected: browser.isConnected(), contextListed: browser.contexts().includes(context),
    listed: pages.map(page => context.pages().includes(page)), closed: pages.map(page => page.isClosed()) };
}
function record(kind, sameArgument = true, sameReceiver = true) {
  assert.ok(trace.length < 32, 'bounded listener trace');
  trace.push({ kind, sameArgument, sameReceiver, ...snapshot() });
}
function listen(target, event, handler, once = false) {
  target[once ? 'once' : 'on'](event, handler);
  removals.push(() => target.off(event, handler));
}
async function turns() { for (let i = 0; i < 8; i++) await setImmediate(); }
async function created() { const page = await context.newPage(); remember(page); return page; }
let observation;
try {
  context = await browser.newContext();
  let outcome = 'resolved';
  if (scenario === 'mutation') {
    let mutated = false;
    function later(page) { record('removed-listener', pages.includes(page), this === context); }
    function added(page) { record('added-listener', pages.includes(page), this === context); }
    listen(context, 'page', function(page) {
      remember(page); record('first-listener', page.context() === context, this === context);
      if (!mutated) {
        mutated = true; context.off('page', later); listen(context, 'page', added);
      }
    });
    listen(context, 'page', later);
    listen(context, 'page', function(page) { record('once-listener', pages.includes(page), this === context); }, true);
    await created(); record('resolved-first');
    await created(); record('resolved-second');
  } else if (scenario === 'async-gate') {
    const gate = Promise.withResolvers(), finished = Promise.withResolvers();
    let entered = false;
    listen(context, 'page', async function(page) {
      entered = true; remember(page); record('async-enter', page.context() === context, this === context);
      await gate.promise;
      record('async-exit', pages.includes(page), this === context); finished.resolve();
    });
    listen(context, 'page', function(page) { record('next-listener', pages.includes(page), this === context); });
    await created(); record('command-resolved');
    gate.resolve();
    if (entered) await finished.promise;
  } else if (scenario === 'close-reentry') {
    const first = await created(), second = await created();
    let nested;
    listen(first, 'close', function(page) {
      record('first-close', page === first, this === first);
      nested = second.close();
      record('nested-call-returned');
    });
    listen(second, 'close', function(page) { record('second-close', page === second, this === second); });
    await first.close();
    if (nested) await nested;
    record('commands-settled');
  } else if (scenario === 'popup-throw') {
    const opener = await created();
    listen(context, 'page', function(page) {
      remember(page); record('throwing-context-page', page.context() === context, this === context);
      throw sentinel;
    }, true);
    listen(context, 'page', function(page) {
      remember(page); record('following-context-page', page.context() === context, this === context);
    });
    listen(opener, 'popup', function(page) { record('opener-popup', pages.includes(page), this === opener); });
    assert.equal(await opener.evaluate(() => !!window.open('about:blank#node110-popup')), true);
    let popup;
    for (let i = 0; i < 1000; i++) {
      popup = context.pages().find(page => page.url() === 'about:blank#node110-popup');
      if (popup) break;
      await setTimeout(5);
    }
    assert.ok(popup, 'native popup must be discovered');
    remember(popup);
    await created(); // Prove later independent native notifications still progress.
    record('following-command-resolved');
  } else if (scenario === 'async-reject') {
    listen(context, 'page', async function(page) {
      remember(page); record('rejecting-listener', page.context() === context, this === context);
      throw sentinel;
    });
    listen(context, 'page', function(page) { record('next-listener', pages.includes(page), this === context); });
    await created(); record('command-resolved');
  } else if (['context-close-page-throw', 'browser-close-page-throw', 'browser-close-page-throw-object', 'browser-close-page-throw-primitive', 'browser-close-page-async-reject', 'browser-close-page-reentry', 'browser-close-context-throw'].includes(scenario)) {
    await created(); await created();
    let firstClose = true;
    for (const page of [...pages]) {
      function closing(value) {
        const first = firstClose;
        if (first) {
          firstClose = false;
          // Native sibling close order is not the assertion. Normalize only
          // this diagnostic's snapshot array, not Context.pages() or wrappers.
          const position = pages.indexOf(page);
          [pages[0], pages[position]] = [pages[position], pages[0]];
        }
        const throws = first && !['browser-close-context-throw', 'browser-close-page-reentry'].includes(scenario);
        record(throws ? 'throwing-page-close' : 'page-close', value === page, this === page);
        if (first && scenario === 'browser-close-page-reentry') {
          exceptionReentry = browser.close();
          record('reentry-returned');
        }
        if (throws) throw sentinel;
      }
      listen(page, 'close', scenario === 'browser-close-page-async-reject'
        ? async function(value) { closing.call(this, value); } : closing);
      listen(page, 'close', function(value) { record('next-page-listener', value === page, this === page); });
    }
    listen(context, 'close', function(value) {
      record('context-close', value === context, this === context);
      if (scenario === 'browser-close-context-throw') throw sentinel;
    });
    listen(context, 'close', function(value) { record('next-context-listener', value === context, this === context); });
    listen(browser, 'disconnected', function(value) { record('disconnected', value === browser, this === browser); });
    await (scenario === 'context-close-page-throw' ? context.close() : browser.close());
    if (exceptionReentry) await exceptionReentry;
    record('close-settled');
  } else {
    await created();
    function throwing(value) {
      record('throwing-disconnected', value === browser, this === browser);
      throw sentinel;
    }
    listen(browser, 'disconnected', scenario === 'disconnect-async-reject'
      ? async function(value) { throwing.call(this, value); } : throwing);
    listen(browser, 'disconnected', function(value) { record('next-disconnected', value === browser, this === browser); });
    try { await browser.close(); }
    catch (error) {
      assert.equal(error, sentinel, 'only the listener error may reject close');
      outcome = 'rejected-listener-error';
    }
    if (exceptionReentry) await exceptionReentry;
    record('close-settled');
  }
  await turns();
  observation = { outcome, trace: structuredClone(trace), errors: structuredClone(errors), final: snapshot() };
} finally {
  for (const remove of removals) remove();
  try { if (browser.isConnected()) await context?.close(); }
  finally {
    await browser.close();
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let i = 0; i < 100 && browser.pwrightFfiMetrics().pending; i++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}
await turns();
assert.deepEqual(errors, observation.errors, 'no delayed listener failure lost during cleanup');
process.off('uncaughtException', onException);
process.off('unhandledRejection', onRejection);
const report = JSON.stringify({ schema: 'pwright.listener-semantics.v1', implementation, scenario, observation });
assert.ok(Buffer.byteLength(report) <= 64 * 1024, 'bounded report');
process.stdout.write(report + '\n');
