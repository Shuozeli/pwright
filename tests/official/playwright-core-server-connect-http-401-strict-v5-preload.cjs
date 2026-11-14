'use strict';

// Only the one line-116 fixture URL is host-adapted. Assertions are unchanged.
const crypto = require('node:crypto');
const fs = require('node:fs');
const net = require('node:net');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const mode = process.env.PWRIGHT_CORE_SERVER_CONNECT_HTTP_401_V5_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_SERVER_CONNECT_HTTP_401_V5_RECEIPT;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const fixtureBind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
  throw new Error('strict HTTP-401 guard requires mode, upstream and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost || ''))
  throw new Error('strict HTTP-401 guard requires a full Tailscale MagicDNS host');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(fixtureBind || ''))
  throw new Error('strict HTTP-401 guard requires a Tailscale IPv4 bind');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`strict HTTP-401 guard forbids ${key}`);
}

const sourceFile = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const sourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const adaptedSha256 = 'b2bef73c0c226ced95ccc3110e97c3c6c49aa9b3f4e96f1e0820a254e014ded6';
const before = 'const error = await connect(`ws://localhost:${server.PORT}/ws-401`).catch(e => e);';
const after = 'const error = await connect(`ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}:${server.PORT}/ws-401`).catch(e => e);';
const sha256 = text => crypto.createHash('sha256').update(text).digest('hex');
const source = fs.readFileSync(sourceFile, 'utf8');
if (sha256(source) !== sourceSha256 || source.split(before).length !== 2)
  throw new Error('strict HTTP-401 source or fixture URL drifted');
const marker = "test('should print HTTP error', async ({ connect, server }) => {";
if (source.split(marker).length !== 2)
  throw new Error('strict HTTP-401 test marker drifted');
const body = source.slice(source.indexOf(marker), source.indexOf('\n    test(', source.indexOf(marker) + marker.length));
for (const assertion of [before, "expect(error.message).toContain('401');", "expect(error.message).toContain('Unauthorized body');"])
  if (body.split(assertion).length !== 2) throw new Error('strict HTTP-401 assertion drifted');
if (sha256(source.replace(before, after)) !== adaptedSha256)
  throw new Error('strict HTTP-401 adapted source digest drifted');

let ownerBound = null;
if (mode === 'candidate') {
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const implementation = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(
    name => implementation[name]._playwright === implementation);
  if (!ownerBound) throw new Error('candidate BrowserType private fixture backpointer was not bound');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('strict oracle HTTP-401 guard forbids candidate selection');
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
  if (typeof value !== 'string' || sha256(value) !== sourceSha256 || value.split(before).length !== 2)
    throw new Error('strict HTTP-401 source read drifted');
  const adapted = value.replace(before, after);
  if (sha256(adapted) !== adaptedSha256)
    throw new Error('strict HTTP-401 source adaptation drifted');
  if (!recordedSource) {
    record({ event: 'source-adapted', sourceSha256, adaptedSha256, replacements: 1 });
    recordedSource = true;
  }
  return isBuffer ? Buffer.from(adapted) : adapted;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function strictHttp401ReadFileSync(file, ...args) {
  return rewrite(file, originalReadFileSync.call(this, file, ...args));
};
const originalReadFile = fs.readFile;
fs.readFile = function strictHttp401ReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};
const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function strictHttp401PromiseReadFile(file, ...args) {
  return rewrite(file, await originalPromiseReadFile(file, ...args));
};

// Observe, but never alter, the pinned TestServer's actual HTTP refusal bytes.
const expectedResponse = 'HTTP/1.1 401 Unauthorized\r\n\r\nUnauthorized body';
const originalWrite = net.Socket.prototype.write;
net.Socket.prototype.write = function strictHttp401ObservedWrite(chunk, ...args) {
  if (chunk === expectedResponse || (Buffer.isBuffer(chunk) && chunk.equals(Buffer.from(expectedResponse))))
    record({ event: 'fixture-401-sent', responseSha256: sha256(expectedResponse) });
  return originalWrite.call(this, chunk, ...args);
};
