import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

// Test infrastructure only. The candidate's production path is the Rust FFI.
const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const base = process.env.PWRIGHT_TEST_BASE;
const bind = process.env.PWRIGHT_TEST_BIND;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(base && new URL(base).hostname.endsWith('.ts.net'));
assert.ok(bind, 'explicit Tailscale bind required');
assert.ok(['frame-navigation', 'drag-frame-navigation', 'frame-extra-headers', 'drag-frame-extra-headers', 'concurrent-close-drag-frame-extra-headers'].includes(process.env.PWRIGHT_NATIVE_PROFILE));

async function withPage(type, exercise, options = {}) {
  const side = type === candidate ? 'candidate' : 'oracle';
  process.stdout.write(`# frame-navigation ${side}:connect\n`);
  const browser = await type.connectOverCDP(endpoint, { timeout: 5000 });
  let context;
  try {
    context = await browser.newContext(options);
    const page = await context.newPage();
    return await exercise(page);
  } finally {
    try { await diagnosticDeadline(context?.close(), `${side} Context cleanup`); }
    finally { await diagnosticDeadline(browser.close(), `${side} Browser disconnect`); }
    if (browser.pwrightMetrics) {
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
    }
    process.stdout.write(`# frame-navigation ${side}:closed\n`);
  }
}

function paired(name, exercise, options) {
  test(name, { timeout: 25000 }, async () => {
    // Arrange
    const expected = await withPage(oracle, exercise, options);
    // Act
    const actual = await withPage(candidate, exercise, options);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

function responseValue(response, frame) {
  if (response === null) return null;
  const headers = response.headers();
  assert.equal(response.frame(), frame);
  assert.equal(response.frame().page(), frame.page());
  return {
    url: response.url(), status: response.status(), statusText: response.statusText(),
    ok: response.ok(), contentType: headers['content-type'],
  };
}

for (const waitUntil of ['commit', 'domcontentloaded', 'load', 'networkidle']) {
  paired(`main Frame HTTP ${waitUntil} response and synchronous URL`, async page => {
    const frame = page.mainFrame();
    const response = await frame.goto(`${base}/empty.html`, { waitUntil, timeout: 5000 });
    assert.equal(page.mainFrame(), frame);
    assert.equal(frame.url(), `${base}/empty.html`);
    return { response: responseValue(response, frame), url: frame.url(), pageUrl: page.url() };
  });
  for (const mode of ['local', 'sandbox']) paired(
    `${mode} child Frame ${waitUntil} preserves parent and sibling`, async page => {
      await page.goto(`${base}/frame-navigation.html?mode=${mode}`, { timeout: 5000 });
      const frame = page.frame('navigation-target');
      const sibling = page.frame('unaffected-sibling');
      assert.ok(frame && sibling);
      const parentUrl = page.url();
      const siblingUrl = sibling.url();
      const response = await frame.goto(`${base}/empty.html#child`, { waitUntil, timeout: 5000 });
      assert.equal(page.frame('navigation-target'), frame);
      assert.equal(page.url(), parentUrl);
      assert.equal(sibling.url(), siblingUrl);
      return { response: responseValue(response, frame), url: frame.url(), parentUrl, siblingUrl };
    });
}

for (const url of ['about:blank', 'data:text/html,<title>frame navigation</title>']) {
  paired(`main Frame ${url.split(':')[0]} returns no response`, async page => {
    const frame = page.mainFrame();
    const response = await frame.goto(url, { timeout: 5000 });
    assert.equal(response, null);
    return frame.url();
  });
}

paired('hash-only navigation returns null and preserves document', async page => {
  const frame = page.mainFrame();
  await frame.goto(`${base}/empty.html`, { timeout: 5000 });
  await frame.evaluate(() => { window.frameNavigationMarker = 'retained'; });
  const response = await frame.goto(`${base}/empty.html#same-document`, { timeout: 5000 });
  assert.equal(response, null);
  return { url: frame.url(), marker: await frame.evaluate(() => window.frameNavigationMarker) };
});

paired('HTTP 404 resolves a response instead of failing navigation', async page => {
  const frame = page.mainFrame();
  const response = await frame.goto(`${base}/missing-node076-document.html`, { timeout: 5000 });
  assert.equal(response.status(), 404);
  assert.equal(response.ok(), false);
  return responseValue(response, frame);
});

paired('Context baseURL resolves Frame navigation before Rust dispatch', async page => {
  const frame = page.mainFrame();
  const response = await frame.goto('empty.html#relative', { timeout: 5000 });
  assert.equal(frame.url(), `${base}/empty.html#relative`);
  return responseValue(response, frame);
}, { baseURL: `${base}/` });

async function withServer(handler, exercise) {
  const server = createServer(handler);
  server.listen(0, bind);
  await once(server, 'listening');
  const origin = `http://${new URL(base).hostname}:${server.address().port}`;
  try { return await exercise(origin); }
  finally { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}

test('Frame referer reaches the native request', { timeout: 25000 }, async () => {
  // Arrange: dynamic request echo, not a static file server.
  await withServer((request, response) => {
    response.writeHead(200, { 'Content-Type': 'text/html', 'X-Seen-Referer': request.headers.referer ?? '' });
    response.end('<title>referer echo</title>');
  }, async origin => {
    const exercise = async page => {
      const response = await page.mainFrame().goto(origin, { referer: `${origin}/source`, timeout: 5000 });
      return response.headers()['x-seen-referer'];
    };
    const expected = await withPage(oracle, exercise);
    // Act
    const actual = await withPage(candidate, exercise);
    // Assert
    assert.equal(actual, expected);
    assert.equal(actual, `${origin}/source`);
  });
});

async function heldNavigation(page, mode, boundary = 'arrival') {
  let signalArrival;
  const arrival = new Promise(resolve => { signalArrival = resolve; });
  let signalExecution;
  const execution = new Promise(resolve => { signalExecution = resolve; });
  return withServer((request, response) => {
    if (request.url === '/executed' && request.method === 'POST') {
      response.writeHead(204).end();
      signalExecution();
      return;
    }
    if (request.url !== '/held') { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
    response.write(`<title>held Frame</title><!--${' '.repeat(2048)}-->`);
    if (boundary === 'executed') response.write('<script>fetch("/executed", { method: "POST" })</script>');
    signalArrival();
  }, async origin => {
    const frame = page.mainFrame();
    const pending = frame.goto(`${origin}/held`, { waitUntil: mode === 'commit' ? 'commit' : 'load', timeout: mode === 'timeout' ? 1000 : 5000 });
    const outcome = pending.then(response => ({ response }), error => ({ error }));
    await diagnosticDeadline(arrival, 'held Frame response arrival');
    if (boundary === 'executed') {
      await diagnosticDeadline(Promise.race([execution, outcome.then(result => {
        throw new Error('Frame navigation settled before document execution', { cause: result.error });
      })]), 'held Frame document execution');
      process.stdout.write('# frame-navigation document-executed\n');
    }
    if (mode === 'page-close') {
      process.stdout.write('# frame-navigation page-close:start\n');
      await diagnosticDeadline(page.close(), 'Page close during held Frame navigation');
      process.stdout.write('# frame-navigation page-close:done\n');
    }
    const result = await outcome;
    if (mode === 'commit') {
      assert.ok(result.response);
      // This response body is deliberately still open when goto resolves.
      assert.equal(result.response.frame(), frame);
      return result.response.status();
    }
    assert.ok(result.error, 'load must not resolve while the body is held');
    if (mode === 'timeout') { assert.equal(result.error.name, 'TimeoutError'); return result.error.name; }
    assert.match(result.error.message, /closed/i);
    return 'closed';
  });
}
for (const mode of ['commit', 'timeout']) paired(
  `held response ${mode} settles with native ownership drained`, page => heldNavigation(page, mode));

test('Rust Frame navigation close drains native owners independently of the oracle', { timeout: 25000 }, async () => {
  // Arrange / Act
  const result = await withPage(candidate, page => heldNavigation(page, 'page-close'));
  // Assert
  assert.equal(result, 'closed');
});

paired('detached Frame navigation rejects without navigating its Page', async page => {
  await page.goto(`${base}/frame-navigation.html?mode=local`, { timeout: 5000 });
  const frame = page.frame('navigation-target');
  assert.ok(frame);
  const parentUrl = page.url();
  await page.evaluate(() => document.querySelector('iframe[name="navigation-target"]').remove());
  await assert.rejects(frame.goto(`${base}/empty.html`, { timeout: 1000 }));
  assert.equal(page.url(), parentUrl);
  return { detached: frame.isDetached(), parentUrl };
});

paired('invalid waitUntil rejects without navigating', async page => {
  const frame = page.mainFrame();
  const initial = frame.url();
  await assert.rejects(frame.goto(`${base}/empty.html`, { waitUntil: 'ready', timeout: 1000 }));
  assert.equal(frame.url(), initial);
  return frame.url();
});

paired('retained Response keeps its Frame identity after Page close', async page => {
  const frame = page.mainFrame();
  const response = await frame.goto(`${base}/empty.html`, { timeout: 5000 });
  const original = responseValue(response, frame);
  await page.close();
  assert.equal(response.frame(), frame);
  assert.deepEqual(responseValue(response, frame), original);
  return original;
});

paired('Page close after new Frame document execution rejects the pending navigation',
  page => heldNavigation(page, 'page-close', 'executed'));

for (const waitUntil of ['domcontentloaded', 'load', 'networkidle']) paired(
  `Frame ${waitUntil} timeout does not cancel browser loading`, async page => {
    let finishBody;
    let signalExecution;
    let requestCount = 0;
    const execution = new Promise(resolve => { signalExecution = resolve; });
    return withServer((request, response) => {
      if (request.url === '/executed' && request.method === 'POST') {
        response.writeHead(204).end();
        signalExecution();
        return;
      }
      if (request.url !== '/held') { response.writeHead(404).end(); return; }
      assert.equal(++requestCount, 1, 'timeout must not retry the navigation');
      response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
      response.write(`<!--${' '.repeat(2048)}--><script>fetch('/executed', { method: 'POST' })</script>`);
      finishBody = () => response.end('<script>window.navigationBodyCompleted = true</script>');
    }, async origin => {
      const frame = page.mainFrame();
      const outcome = frame.goto(`${origin}/held`, { waitUntil, timeout: 2000 }).then(
        response => ({ response }), error => ({ error }));
      await diagnosticDeadline(Promise.race([execution, outcome.then(result => {
        throw new Error('navigation settled before timeout-continuation fixture executed', { cause: result.error });
      })]), 'timeout-continuation document execution');
      const result = await outcome;
      assert.equal(result.error?.name, 'TimeoutError');
      assert.equal(await frame.evaluate(() => document.readyState), 'loading');
      // A server-controlled continuation proves the browser request remains
      // live after the API deadline. This is not a production retry/wait loop.
      finishBody();
      const completed = await frame.evaluate(() => new Promise(resolve => {
        if (document.readyState === 'complete') resolve(window.navigationBodyCompleted === true);
        else window.addEventListener('load', () => resolve(window.navigationBodyCompleted === true), { once: true });
      }));
      assert.equal(completed, true);
      assert.equal(frame.url(), `${origin}/held`);
      return { completed, requestCount, urlSuffix: new URL(frame.url()).pathname };
    });
  });

// Keep this reproducer last: a browser-side close stall must be recorded as a
// failed oracle case, without preventing independent Rust boundary coverage.
paired('held response page-close matches upstream settlement', page => heldNavigation(page, 'page-close'));
