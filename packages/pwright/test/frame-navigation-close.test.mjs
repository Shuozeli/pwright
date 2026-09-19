import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { setImmediate, setTimeout } from 'node:timers/promises';
import { test } from 'node:test';
import { chromium as candidate } from '@shuozeli/pwright';
import artifact from '../src/native-artifact.cjs';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const oracle = require('playwright').chromium;
assert.ok(artifact.nativeFeatures(artifact.nativeArtifact.profile).includes('frame-navigation'));
const endpoint = process.env.PWRIGHT_TEST_CDP;
const bind = process.env.PWRIGHT_TEST_BIND;
const host = process.env.PWRIGHT_TEST_HOST;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(bind && host?.endsWith('.ts.net'));

function navigation(page, url, options) {
  let value;
  const promise = page.mainFrame().goto(url, options).then(response => {
    value = { outcome: 'success', status: response?.status() ?? null };
    return value;
  }, error => {
    value = { outcome: /Target page, context or browser has been closed/.test(error.message)
      ? 'target-closed' : 'navigation-error' };
    return value;
  });
  return { promise, settled: () => value };
}

async function committed(page, url, pending) {
  await diagnosticDeadline((async () => {
    while (page.url() !== url) {
      assert.equal(pending.settled(), undefined, 'navigation must still wait for load');
      await setTimeout(5);
    }
  })(), 'published document commit');
  assert.equal(pending.settled(), undefined, 'a committed streaming document has not loaded');
}

async function withFixture(type, exercise) {
  const bodies = new Map();
  const server = createServer({ maxHeaderSize: 8192 }, (request, response) => {
    if (request.url === '/fail') { request.socket.destroy(); return; }
    if (!request.url?.startsWith('/held/')) { response.writeHead(404).end(); return; }
    assert.ok(bodies.size < 2, 'bounded held response inventory');
    assert.equal(bodies.has(request.url), false);
    bodies.set(request.url, response);
    response.on('close', () => bodies.delete(request.url));
    response.writeHead(200, { 'content-type': 'text/html', 'cache-control': 'no-store' });
    response.write(`<title>committed streaming document</title><!--${' '.repeat(8192)}-->`);
  });
  server.maxConnections = 16;
  server.requestTimeout = 5000;
  server.listen(0, bind);
  await once(server, 'listening');
  const base = `http://${host}:${server.address().port}`;
  let browser;
  const contexts = [];
  try {
    browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
    return await exercise({
      base,
      async page() {
        assert.ok(contexts.length < 2);
        const context = await browser.newContext();
        contexts.push(context);
        return { context, page: await context.newPage() };
      },
      finish(path) {
        assert.ok(bodies.has(path), 'finish only the exact held response');
        bodies.get(path).end();
      },
    });
  } finally {
    try {
      for (const context of contexts) await diagnosticDeadline(context.close(), 'owned Context cleanup');
    } finally {
      try { if (browser) await diagnosticDeadline(browser.close(), 'Browser disconnect'); }
      finally {
        server.closeAllConnections();
        await new Promise(resolve => server.close(resolve));
      }
    }
    if (browser?.pwrightMetrics) {
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      const metrics = browser.pwrightMetrics();
      assert.equal(metrics.activeLeases, 0);
      assert.equal(metrics.cleanupInFlight, 0);
      assert.equal(metrics.activeOperations, 0);
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}

function paired(name, exercise, expected) {
  test(name, { timeout: 25000 }, async () => {
    // Arrange
    const reference = await withFixture(oracle, exercise);
    assert.deepEqual(reference, expected, 'pinned upstream contract');
    // Act
    const actual = await withFixture(candidate, exercise);
    // Assert
    assert.deepEqual(actual, reference);
  });
}

for (const scope of ['context', 'page']) {
  paired(`Frame.goto committed document interrupted by ${scope} close`, async fixture => {
    const owned = await fixture.page();
    const url = `${fixture.base}/held/one`;
    const pending = navigation(owned.page, url, { timeout: 5000 });
    await committed(owned.page, url, pending);
    await diagnosticDeadline(owned[scope].close(), 'close committed navigation owner');
    return await diagnosticDeadline(pending.promise, 'navigation terminal result');
  }, { outcome: 'target-closed' });
}

paired('Frame.goto network failure completed before close remains a network error', async fixture => {
  const { context, page } = await fixture.page();
  const pending = navigation(page, `${fixture.base}/fail`, { timeout: 5000 });
  const result = await diagnosticDeadline(pending.promise, 'pre-close network failure');
  await context.close();
  return result;
}, { outcome: 'navigation-error' });

paired('closing one Context does not cancel a sibling committed navigation', async fixture => {
  const first = await fixture.page();
  const second = await fixture.page();
  const firstUrl = `${fixture.base}/held/first`;
  const secondUrl = `${fixture.base}/held/second`;
  const a = navigation(first.page, firstUrl, { timeout: 5000 });
  const b = navigation(second.page, secondUrl, { timeout: 5000 });
  await committed(first.page, firstUrl, a);
  await committed(second.page, secondUrl, b);
  await first.context.close();
  const closed = await diagnosticDeadline(a.promise, 'first navigation close');
  assert.equal(b.settled(), undefined);
  fixture.finish('/held/second');
  const survivor = await diagnosticDeadline(b.promise, 'sibling navigation load');
  return { closed, survivor };
}, { closed: { outcome: 'target-closed' }, survivor: { outcome: 'success', status: 200 } });

paired('Frame.goto commit result already delivered is not changed by a later close', async fixture => {
  const { context, page } = await fixture.page();
  const pending = navigation(page, `${fixture.base}/held/commit`, { waitUntil: 'commit', timeout: 5000 });
  const result = await diagnosticDeadline(pending.promise, 'commit-only navigation');
  await context.close();
  return result;
}, { outcome: 'success', status: 200 });
