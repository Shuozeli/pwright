import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = requireOracle('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const fixtureBase = process.env.PWRIGHT_TEST_BASE;
const bind = process.env.PWRIGHT_TEST_BIND;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(fixtureBase && new URL(fixtureBase).hostname.endsWith('.ts.net'));
assert.ok(bind, 'explicit Tailscale bind required');
const trace = phase => process.stdout.write(`# reload-phase ${phase}\n`);

async function withPage(type, exercise) {
  const side = type === candidate ? 'candidate' : 'oracle';
  trace(`${side}:connect:start`);
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  trace(`${side}:connect:done`);
  let context;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    trace(`${side}:exercise:start`);
    return await exercise(page);
  } finally {
    trace(`${side}:context-close:start`);
    try { await context?.close(); } finally {
      trace(`${side}:browser-close:start`);
      await browser.close();
      trace(`${side}:browser-close:done`);
    }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
  }
}

function paired(name, exercise) {
  test(name, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

for (const [kind, url] of [
  ['blank', 'about:blank'], ['data', 'data:text/html,<title>reload-ffi</title>'],
  ['HTTP', `${fixtureBase}/index.html#preserved`],
]) for (const waitUntil of ['commit', 'domcontentloaded', 'load', 'networkidle']) {
  paired(`${kind} reload ${waitUntil} matches response, document replacement and history`, async page => {
    await page.goto(url, { timeout: 5000 });
    const length = await page.evaluate(() => { window.__reloadMarker = true; return history.length; });
    const response = await page.reload({ waitUntil, timeout: 5000 });
    const state = await page.evaluate(() => ({ marker: typeof window.__reloadMarker, length: history.length }));
    assert.equal(state.marker, 'undefined');
    assert.equal(state.length, length);
    return { url: page.url(), state, response: response && {
      url: response.url(), status: response.status(), statusText: response.statusText(),
      ok: response.ok(), headers: response.headers(), frame: response.frame() === page.mainFrame(),
    } };
  });
}

async function withServer(handler, exercise) {
  const server = createServer(handler);
  server.listen(0, bind);
  await once(server, 'listening');
  const base = `http://${new URL(fixtureBase).hostname}:${server.address().port}`;
  try { return await exercise(base); }
  finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}

test('reload resubmits the existing POST rather than navigating with GET', { timeout: 25000 }, async () => {
  // Arrange: dynamic form server; no static directory is exposed here.
  await withServer((request, response) => {
    request.setEncoding('utf8');
    let body = '';
    request.on('data', chunk => {
      body += chunk;
      if (body.length > 256) request.destroy(new Error('fixture request body exceeds bound'));
    });
    request.on('end', () => {
      response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store', 'X-Method': request.method, 'X-Payload': body });
      if (request.url === '/form') response.end('<form action="/posted" method="post"><input name="payload" value="preserved"><button>Send</button></form>');
      else response.end(`<title>${request.method}</title><div id="posted">${request.method}</div>`);
    });
  }, async base => {
    const exercise = async page => {
      await page.goto(`${base}/form`, { timeout: 5000 });
      await page.locator('button').click({ timeout: 5000 });
      assert.equal(await page.locator('#posted').textContent({ timeout: 5000 }), 'POST');
      assert.equal(await page.title(), 'POST');
      const response = await page.reload({ timeout: 5000 });
      return { title: await page.title(), method: response.headers()['x-method'], payload: response.headers()['x-payload'], status: response.status() };
    };
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.deepEqual(actual, expected);
    assert.equal(actual.method, 'POST');
    assert.equal(actual.payload, 'payload=preserved');
  });
});

async function heldReload(page, mode, boundary = 'response-arrival') {
  let count = 0;
  let arrived;
  let committed;
  const started = new Promise(resolve => { arrived = resolve; });
  const documentExecuted = new Promise(resolve => { committed = resolve; });
  return withServer((request, response) => {
    if (request.url === '/new-document-executed' && request.method === 'POST') {
      response.writeHead(204).end();
      trace('post-commit:new-document-executed');
      committed();
      return;
    }
    if (request.url !== '/document') { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
    response.write('<title>held reload</title>');
    if (++count === 1) response.end('<p>initial</p>');
    else {
      if (boundary === 'post-commit') {
        // Only the replacement document sends this beacon. Padding avoids
        // conflating an unconsumed short response with an executed document.
        response.write(`<!--${' '.repeat(2048)}--><script>fetch('/new-document-executed', { method: 'POST' })</script>`);
      }
      response.once('close', () => trace(`${boundary}:held-response-closed`));
      arrived();
    }
  }, async base => {
    await page.goto(`${base}/document`, { timeout: 5000 });
    // The original arrival gate retains its 1 s API timeout. The independent
    // post-commit case explicitly budgets 5 s for executing its causal beacon.
    const pending = page.reload({ waitUntil: 'load', timeout: boundary === 'post-commit' ? 5000 : 1000 }).then(
      () => { throw new Error('reload resolved before body completion'); }, error => error);
    await diagnosticDeadline(Promise.race([started, pending.then(error => {
      throw new Error('reload settled before the held response arrived', { cause: error });
    })]), 'reload fixture response arrival');
    trace(`${mode}:response-arrived`);
    if (boundary === 'post-commit') {
      await diagnosticDeadline(Promise.race([documentExecuted, pending.then(error => {
        throw new Error('reload settled before the new document executed', { cause: error });
      })]), 'replacement document execution');
      trace('post-commit:close-boundary-reached');
    }
    if (mode === 'page-close') {
      trace('page-close:start');
      await diagnosticDeadline(page.close(), `Page close during ${boundary} reload`);
      trace('page-close:done');
    }
    const error = await pending;
    trace(`${mode}:reload-settled`);
    if (mode === 'timeout') { assert.equal(error.name, 'TimeoutError'); return error.name; }
    assert.match(error.message, /closed/i);
    return 'closed';
  });
}

for (const mode of ['timeout', 'page-close']) paired(
  `reload ${mode} settles after a causally held document response`, page => heldReload(page, mode));

test('Rust Page close during held reload independently drains native ownership', { timeout: 20000 }, async () => {
  // Arrange / Act: independent coverage even when the preceding oracle fails.
  const result = await withPage(candidate, page => heldReload(page, 'page-close'));
  // Assert: withPage has already checked both native and FFI resource release.
  assert.equal(result, 'closed');
});

paired('Page close after replacement document execution settles a still-loading reload',
  page => heldReload(page, 'page-close', 'post-commit'));

paired('reload on a closed Page rejects instead of starting new native work', async page => {
  await page.close();
  await assert.rejects(page.reload({ timeout: 5000 }), /closed/i);
  return 'closed';
});
