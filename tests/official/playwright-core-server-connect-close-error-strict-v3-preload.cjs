'use strict';

// Source-pinned, browser-free Close diagnostic guard. No upstream test source is rewritten.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const mode = process.env.PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_RECEIPT;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const fixtureBind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
  throw new Error('strict server-connect Close guard requires mode, upstream and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost || ''))
  throw new Error('strict server-connect Close guard requires full Tailscale MagicDNS host');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(fixtureBind || ''))
  throw new Error('strict server-connect Close guard requires a Tailscale IPv4 bind');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`strict server-connect Close guard forbids ${key}`);
}

const source = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const expectedSourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const actualSourceSha256 = crypto.createHash('sha256').update(fs.readFileSync(source)).digest('hex');
if (actualSourceSha256 !== expectedSourceSha256)
  throw new Error(`strict server-connect Close source drifted: ${actualSourceSha256}`);
const text = fs.readFileSync(source, 'utf8');
for (const snippet of [
  "test('should print custom ws close error', async ({ connect, server }) => {",
  'server.onceWebSocketConnection((ws, request) => {',
  "ws.on('message', message => {",
  "ws.close(4123, 'Oh my!');",
  'const error = await connect(`ws://localhost:${server.PORT}/ws`).catch(e => e);',
  "expect(error.message).toContain('browserType.connect: Oh my!');",
]) {
  if (text.split(snippet).length !== 2)
    throw new Error('strict server-connect Close test body or assertion drifted');
}

let ownerBound = null;
if (mode === 'candidate') {
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const implementation = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(
    name => implementation[name]._playwright === implementation);
  if (!ownerBound) throw new Error('candidate BrowserType private fixture backpointer was not bound');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('strict oracle server-connect Close guard forbids candidate selection');
}

fs.appendFileSync(receipt, JSON.stringify({
  pid: process.pid,
  workerIndex: process.env.TEST_WORKER_INDEX ?? null,
  mode,
  sourceSha256: actualSourceSha256,
  fixtureHost,
  fixtureBind,
  ownerBound,
  cdpEndpointPresent: false,
  sourceAdapted: false,
}) + '\n');
