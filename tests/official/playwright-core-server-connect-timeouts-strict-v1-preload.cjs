'use strict';

// Adapt only the two literal loopback URLs. Test assertions stay unchanged.
const crypto = require('node:crypto');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const mode = process.env.PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_SERVER_CONNECT_TIMEOUTS_V1_RECEIPT;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
  throw new Error('strict timeout guard requires mode, upstream and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(host || ''))
  throw new Error('strict timeout guard requires full Tailscale MagicDNS');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(bind || ''))
  throw new Error('strict timeout guard requires Tailscale IPv4 bind');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`strict timeout guard forbids ${key}`);
}

const sourceFile = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const expectedSourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const adaptedSha256 = '84991e12f684fc2f13d2aa915c40fbf450e6481de27eea7f9e149bbc84411cbb';
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex');
const source = fs.readFileSync(sourceFile, 'utf8');
if (sha256(source) !== expectedSourceSha256)
  throw new Error('strict timeout source drifted');
const before = [
  'const e = await connect(`ws://localhost:${server.PORT}/ws-slow`, {',
  'const e = await connect(`ws://localhost:${server.PORT}/ws`, {',
];
const after = before.map(value => value.replace('ws://localhost',
  'ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}'));
let adaptedSource = source;
for (let index = 0; index < before.length; ++index) {
  if (adaptedSource.split(before[index]).length !== 2)
    throw new Error('strict timeout fixture URL changed or repeated');
  adaptedSource = adaptedSource.replace(before[index], after[index]);
}
if (sha256(adaptedSource) !== adaptedSha256)
  throw new Error('strict timeout adapted source digest drifted');
for (const assertion of [
  "expect(e.message).toContain('browserType.connect: Timeout 1000ms exceeded');",
  "expect(e.message).toContain('browserType.connect: Timeout 100ms exceeded');",
]) {
  if (source.split(assertion).length !== 2 || adaptedSource.split(assertion).length !== 2)
    throw new Error('strict timeout assertion drifted');
}
const fixture = fs.readFileSync(path.join(path.resolve(upstream), 'tests/config/testserver/index.ts'), 'utf8');
for (const snippet of [
  "if (pathname === '/ws-slow')",
  'await new Promise(f => setTimeout(f, 2000));',
  "if (!['/ws', '/ws-slow'].includes(pathname))",
]) {
  if (fixture.split(snippet).length !== 2) throw new Error('upstream timeout fixture drifted');
}

let ownerBound = null;
if (mode === 'candidate') {
  if (process.env.PWRIGHT_NATIVE_PROFILE !== 'playwright-server-connect-session')
    throw new Error('candidate timeout diagnostic requires session profile');
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const candidate = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(name => candidate[name]._playwright === candidate);
  if (!ownerBound) throw new Error('candidate private fixture owner is absent');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('oracle timeout diagnostic forbids candidate selection');
}

function record(event, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    event, pid: process.pid, workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    mode, fixtureHost: host, fixtureBind: bind, ownerBound,
    cdpEndpointPresent: false, ...extra,
  }) + '\n');
}

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? fileURLToPath(value) : '';
  if (Buffer.isBuffer(value)) return path.resolve(value.toString('utf8'));
  return typeof value === 'string' ? path.resolve(value) : '';
}
const adaptedWorkers = new Set();
function rewrite(file, raw) {
  if (sourcePath(file) !== sourceFile) return raw;
  const isBuffer = Buffer.isBuffer(raw);
  const value = isBuffer ? raw.toString('utf8') : raw;
  if (typeof value !== 'string' || sha256(value) !== expectedSourceSha256)
    throw new Error('strict timeout source read drifted');
  const worker = process.env.TEST_WORKER_INDEX ?? 'controller';
  if (!adaptedWorkers.has(worker)) {
    adaptedWorkers.add(worker);
    record('source-adapted', {
      sourceSha256: expectedSourceSha256, adaptedSha256, replacements: 2,
    });
  }
  return isBuffer ? Buffer.from(adaptedSource) : adaptedSource;
}
const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function strictTimeoutReadFileSync(file, ...args) {
  return rewrite(file, originalReadFileSync.call(this, file, ...args));
};
const originalReadFile = fs.readFile;
fs.readFile = function strictTimeoutReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};
const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function strictTimeoutPromiseReadFile(file, ...args) {
  return rewrite(file, await originalPromiseReadFile(file, ...args));
};

const originalEmit = http.Server.prototype.emit;
http.Server.prototype.emit = function strictTimeoutServerEmit(event, request, ...rest) {
  if (event === 'upgrade' && request && ['/ws', '/ws-slow'].includes(request.url)) {
    record('fixture-upgrade-request', {
      requestPath: request.url, requestHost: request.headers.host ?? null,
    });
  }
  return originalEmit.call(this, event, request, ...rest);
};
