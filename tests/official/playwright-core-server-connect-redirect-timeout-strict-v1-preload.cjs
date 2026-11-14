'use strict';

// Observe the unchanged upstream test and its redirect/abort fixture only.
const crypto = require('node:crypto');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');

const mode = process.env.PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_MODE;
const receipt = process.env.PWRIGHT_CORE_SERVER_REDIRECT_TIMEOUT_V1_RECEIPT;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !receipt || !upstream)
  throw new Error('strict redirect timeout guard requires mode, upstream and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(host || ''))
  throw new Error('strict redirect timeout guard requires full Tailscale MagicDNS');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(bind || ''))
  throw new Error('strict redirect timeout guard requires Tailscale IPv4 bind');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`strict redirect timeout guard forbids ${key}`);
}

const source = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const sourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const fixture = path.join(path.resolve(upstream), 'tests/config/testserver/index.ts');
const fixtureSha256 = '4330e0b94c88fdba07401efd3f4cd30a229bb48da037d8c880cd79ba0bae7307';
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex');
const sourceText = fs.readFileSync(source, 'utf8');
const fixtureText = fs.readFileSync(fixture, 'utf8');
if (sha256(sourceText) !== sourceSha256 || sha256(fixtureText) !== fixtureSha256)
  throw new Error('strict redirect timeout upstream source drifted');
for (const snippet of [
  "test('should timeout after redirect when connecting over http', async ({ connect, server }) => {",
  "server.setRedirect('/connect/json', '/connect/slow');",
  "server.setRoute('/connect/slow', (req, res) => {",
  "req.socket.on('close', () => aborted = true);",
  "const error = await connect(`${server.PREFIX}/connect`, { timeout: 2000, headers: { 'Connection': 'Close' } }).catch(e => e);",
  "expect(error.message).toContain('Timeout 2000ms exceeded.');",
  'await expect.poll(() => aborted).toBe(true);',
]) {
  if (sourceText.split(snippet).length !== 2)
    throw new Error('strict redirect timeout test body or assertion drifted');
}
for (const snippet of [
  'res.writeHead(302, { ...headers, location: to });',
  'res.end();',
]) {
  if (!fixtureText.includes(snippet)) throw new Error('strict redirect timeout fixture drifted');
}

let ownerBound = null;
if (mode === 'candidate') {
  if (process.env.PWRIGHT_NATIVE_PROFILE !== 'playwright-server-connect-session')
    throw new Error('candidate redirect timeout diagnostic requires session profile');
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const candidate = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(name => candidate[name]._playwright === candidate);
  if (!ownerBound) throw new Error('candidate private fixture owner is absent');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('oracle redirect timeout diagnostic forbids candidate selection');
}

function record(event, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    event, pid: process.pid, workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    mode, fixtureHost: host, fixtureBind: bind, ownerBound,
    cdpEndpointPresent: false, ...extra,
  }) + '\n');
}
record('source-verified', { sourceSha256, fixtureSha256 });

const originalEmit = http.Server.prototype.emit;
let requestId = 0;
http.Server.prototype.emit = function strictRedirectTimeoutServerEmit(event, request, ...rest) {
  if (event === 'request' && request && ['/connect/json', '/connect/slow'].includes(request.url)) {
    const id = ++requestId;
    const requestPath = request.url;
    const requestHost = request.headers.host ?? null;
    record('fixture-request', { requestId: id, requestPath, requestHost, requestMethod: request.method });
    const response = rest[0];
    if (requestPath === '/connect/json') {
      let redirectLocation = null;
      const writeHead = response.writeHead;
      response.writeHead = function strictRedirectTimeoutWriteHead(...args) {
        for (const headers of args) {
          if (!headers || typeof headers !== 'object' || Array.isArray(headers)) continue;
          for (const [name, value] of Object.entries(headers)) {
            if (name.toLowerCase() === 'location') redirectLocation = value;
          }
        }
        return writeHead.apply(this, args);
      };
      response.once('finish', () => record('redirect-response', {
        requestId: id, status: response.statusCode,
        location: response.getHeader('location') ?? redirectLocation,
      }));
    } else {
      request.socket.once('close', () => record('slow-socket-closed', { requestId: id }));
      response.once('finish', () => record('slow-response-finished', { requestId: id }));
    }
  }
  return originalEmit.call(this, event, request, ...rest);
};
