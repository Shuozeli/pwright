'use strict';

// Adapt only the two pinned fixture destination URLs; preserve the test bodies
// and their assertions. Observe the real HTTP upgrade at the fixture server.
const crypto = require('node:crypto');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const mode = process.env.PWRIGHT_CORE_SERVER_CONNECT_HEADERS_V1_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_SERVER_CONNECT_HEADERS_V1_RECEIPT;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const fixtureBind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
  throw new Error('strict connect headers guard requires mode, upstream and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost || ''))
  throw new Error('strict connect headers guard requires a full Tailscale MagicDNS host');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(fixtureBind || ''))
  throw new Error('strict connect headers guard requires a Tailscale IPv4 bind');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`strict connect headers guard forbids ${key}`);
}

const sourceFile = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const sourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const beforeExtra = "connect(`ws://localhost:${server.PORT}/ws`, {\n          headers: {\n            'User-Agent': 'Playwright',";
const beforeDefault = "connect(`ws://localhost:${server.PORT}/ws`, {\n          headers: {\n            'foo': 'bar',";
const adaptedHost = '${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}';
const afterExtra = beforeExtra.replace('localhost', adaptedHost);
const afterDefault = beforeDefault.replace('localhost', adaptedHost);
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex');
const source = fs.readFileSync(sourceFile, 'utf8');
if (sha256(source) !== sourceSha256 || source.split(beforeExtra).length !== 2 ||
    source.split(beforeDefault).length !== 2)
  throw new Error('strict connect headers source or fixture URLs drifted');
const adaptedSha256 = sha256(source.replace(beforeExtra, afterExtra).replace(beforeDefault, afterDefault));

let ownerBound = null;
if (mode === 'candidate') {
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const implementation = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(
    name => implementation[name]._playwright === implementation);
  if (!ownerBound) throw new Error('candidate BrowserType fixture owner was not bound');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('strict oracle connect headers guard forbids candidate selection');
}

function record(value) {
  fs.appendFileSync(receipt, JSON.stringify({
    pid: process.pid,
    workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    mode,
    fixtureHost,
    fixtureBind,
    ownerBound,
    cdpEndpointPresent: false,
    ...value,
  }) + '\n');
}

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? fileURLToPath(value) : '';
  return typeof value === 'string' ? path.resolve(value) : '';
}

let recordedSource = false;
function rewrite(file, raw) {
  if (sourcePath(file) !== sourceFile) return raw;
  const isBuffer = Buffer.isBuffer(raw);
  const value = isBuffer ? raw.toString('utf8') : raw;
  if (typeof value !== 'string' || sha256(value) !== sourceSha256)
    throw new Error('strict connect headers source read drifted');
  const adapted = value.replace(beforeExtra, afterExtra).replace(beforeDefault, afterDefault);
  if (sha256(adapted) !== adaptedSha256)
    throw new Error('strict connect headers adaptation drifted');
  if (!recordedSource) {
    record({ event: 'source-adapted', sourceSha256, adaptedSha256, replacements: 2 });
    recordedSource = true;
  }
  return isBuffer ? Buffer.from(adapted) : adapted;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function strictConnectHeadersReadFileSync(file, ...args) {
  return rewrite(file, originalReadFileSync.call(this, file, ...args));
};
const originalReadFile = fs.readFile;
fs.readFile = function strictConnectHeadersReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};
const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function strictConnectHeadersPromiseReadFile(file, ...args) {
  return rewrite(file, await originalPromiseReadFile(file, ...args));
};

const originalEmit = http.Server.prototype.emit;
let upgradeOrdinal = 0;
http.Server.prototype.emit = function strictConnectHeadersServerEmit(event, ...args) {
  if (event === 'upgrade' && args[0]?.url === '/ws') {
    const [request, socket] = args;
    const headers = request.headers;
    const host = headers.host;
    if (socket.localAddress !== fixtureBind ||
        !new RegExp(`^${fixtureHost.replaceAll('.', '\\.')}:[0-9]+$`).test(host || ''))
      throw new Error('strict connect headers upgrade did not reach the Tailscale fixture');
    const names = ['host', 'user-agent', 'foo', 'x-playwright-browser'];
    const protocolNames = ['connection', 'upgrade', 'sec-websocket-version', 'sec-websocket-key'];
    const rawValues = new Map();
    for (let index = 0; index < request.rawHeaders.length; index += 2) {
      const name = request.rawHeaders[index].toLowerCase();
      rawValues.set(name, [...(rawValues.get(name) || []), request.rawHeaders[index + 1]]);
    }
    record({
      event: 'fixture-upgrade-received',
      upgradeOrdinal: ++upgradeOrdinal,
      method: request.method,
      url: request.url,
      localAddress: socket.localAddress,
      localPort: socket.localPort,
      assertedHeaders: Object.fromEntries(names.map(name => [name, headers[name] ?? null])),
      rawAssertedHeaders: Object.fromEntries(names.map(name => [name, rawValues.get(name) || []])),
      protocolHeaderCounts: Object.fromEntries(
        protocolNames.map(name => [name, (rawValues.get(name) || []).length])),
    });
  }
  return originalEmit.call(this, event, ...args);
};
