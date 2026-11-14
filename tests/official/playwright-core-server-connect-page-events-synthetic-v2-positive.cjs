'use strict';

// Separate positive-delivery witness. The three selected upstream teardown
// tests are never edited or injected with events.
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { spawn } = require('node:child_process');
const { once } = require('node:events');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');

const mode = process.env.PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const candidate = process.env.PWRIGHT_NODE_CANDIDATE;
const fixture = process.env.PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_SERVER;
const receipt = process.env.PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_RECEIPT;
if (!['oracle', 'candidate'].includes(mode) || !upstream || !fixture || !receipt)
  throw new Error('positive probe requires a pinned lane, upstream and fixture');
if (mode === 'candidate' && (!candidate ||
    process.env.PWRIGHT_NATIVE_PROFILE !== 'playwright-server-connect-page-events'))
  throw new Error('positive candidate probe requires selected addon snapshot');
if (mode === 'oracle' && (candidate || process.env.PWRIGHT_NATIVE_PROFILE))
  throw new Error('positive oracle probe must not load Pwright');
if (!path.isAbsolute(fixture) || !fs.statSync(fixture).isFile())
  throw new Error('positive probe fixture is not a regular absolute file');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`positive probe forbids ${key}`);
}

const eventUrl = 'https://example.test/report';
const downloadUrl = 'https://example.test/report.csv';
const expectedFilename = 'report.csv';
const token = crypto.randomBytes(24).toString('hex');
const implementation = mode === 'oracle'
  ? require(path.join(path.resolve(upstream), 'packages/playwright-core'))
  : require(path.resolve(candidate));

function startServer() {
  const child = spawn(process.execPath, [fixture], {
    env: { ...process.env, NODE_OPTIONS: '',
      PWRIGHT_CORE_PAGE_EVENTS_SYNTHETIC_PROBE_TOKEN: token },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stderr.on('data', data => { stderr += data.toString(); });
  const endpoint = new Promise((resolve, reject) => {
    child.stdout.on('data', data => {
      stdout += data.toString();
      const match = stdout.match(/^Listening on (ws:\/\/[^\s]+)\n/m);
      if (match) resolve(match[1]);
    });
    child.on('exit', (code, signal) => reject(new Error(
      `synthetic server exited before listening (${code}, ${signal}): ${stderr}`)));
    child.on('error', reject);
  });
  return { child, endpoint };
}

function trigger(endpoint) {
  const url = new URL(endpoint.replace(/^ws:/, 'http:'));
  url.pathname = '/emit-positive';
  return new Promise((resolve, reject) => {
    const request = http.request(url, {
      method: 'POST', headers: { 'x-pwright-probe-token': token },
    }, response => {
      response.resume();
      response.on('end', () => {
        if (response.statusCode !== 204) reject(new Error(`positive trigger status ${response.statusCode}`));
        else resolve();
      });
    });
    request.on('error', reject);
    request.setTimeout(5000, () => request.destroy(new Error('positive trigger timed out')));
    request.end();
  });
}

async function stopServer(child) {
  if (child.exitCode !== null || child.signalCode !== null) return child.exitCode;
  child.kill('SIGINT');
  const [code] = await once(child, 'exit');
  return code;
}

async function main() {
  const server = startServer();
  let browser;
  try {
    const endpoint = await server.endpoint;
    browser = await implementation.chromium.connect(endpoint);
    const page = await browser.newPage();
    const received = [];
    for (const kind of ['request', 'response', 'download'])
      page.on(kind, value => received.push({ kind, value }));
    const requestPromise = page.waitForRequest(eventUrl, { timeout: 5000 });
    const responsePromise = page.waitForResponse(eventUrl, { timeout: 5000 });
    const downloadPromise = page.waitForEvent('download', { timeout: 5000 });
    await trigger(endpoint);
    const [request, response, download] = await Promise.all([
      requestPromise, responsePromise, downloadPromise,
    ]);
    assert.deepEqual(received.map(item => item.kind), ['request', 'response', 'download']);
    assert.equal(received[0].value, request);
    assert.equal(received[1].value, response);
    assert.equal(received[2].value, download);
    assert.equal(request.url(), eventUrl);
    assert.equal(request.method(), 'POST');
    assert.equal(response.url(), eventUrl);
    assert.equal(response.status(), 201);
    assert.equal(response.request(), request);
    assert.equal(download.url(), downloadUrl);
    assert.equal(download.suggestedFilename(), expectedFilename);
    await browser.close();
    browser = null;
    const exitCode = await stopServer(server.child);
    assert.equal(exitCode, 0);
    process.stdout.write(JSON.stringify({
      mode,
      positiveEventDelivery: true,
      waiterResolved: ['request', 'response', 'download'],
      emitterOrder: received.map(item => item.kind),
      request: { url: request.url(), method: request.method() },
      response: { url: response.url(), status: response.status(), requestIdentity: true },
      download: { url: download.url(), suggestedFilename: download.suggestedFilename() },
      syntheticServerExitCode: exitCode,
    }) + '\n');
  } finally {
    try { await browser?.close(); } catch {}
    await stopServer(server.child);
  }
}

main().catch(error => {
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});
