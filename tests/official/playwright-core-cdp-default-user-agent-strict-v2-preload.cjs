'use strict';

// Redirect only the pinned test's fixture endpoint. Its connectOverCDP call,
// supplied headers, and official assertions remain byte-for-byte unchanged.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const receipt = process.env.PWRIGHT_CORE_CDP_DEFAULT_UA_V2_GUARD_RECEIPT;
if (!upstream || !fixtureHost || !receipt)
  throw new Error('strict CDP default User-Agent fixture requires upstream, host, and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost))
  throw new Error('strict CDP default User-Agent fixture requires full Tailscale MagicDNS host');

const target = path.join(path.resolve(upstream), 'tests/library/chromium/connect-over-cdp.spec.ts');
const expectedSha256 = 'be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c';
const title = "test('should send default User-Agent header with connect request'";
const wsBefore = 'wsEndpoint: `ws://localhost:${server.PORT}/ws`';
let recorded = false;

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? fileURLToPath(value) : '';
  return typeof value === 'string' ? path.resolve(value) : '';
}

function rewrite(file, raw) {
  if (sourcePath(file) !== target) return raw;
  const isBuffer = Buffer.isBuffer(raw);
  const source = isBuffer ? raw.toString('utf8') : raw;
  if (typeof source !== 'string') throw new Error('strict CDP default User-Agent source is not text');
  const actualSha256 = crypto.createHash('sha256').update(source).digest('hex');
  if (actualSha256 !== expectedSha256)
    throw new Error(`strict CDP default User-Agent source drifted: ${actualSha256}`);
  const start = source.indexOf(title);
  if (start < 0 || source.indexOf(title, start + title.length) !== -1)
    throw new Error('strict CDP default User-Agent test title drifted');
  const nextTest = source.indexOf('\ntest(', start + title.length);
  const end = nextTest < 0 ? source.length : nextTest;
  const body = source.slice(start, end);
  if (body.split(wsBefore).length !== 2)
    throw new Error('strict CDP default User-Agent endpoint marker drifted');
  const changed = body.replace(wsBefore, `wsEndpoint: \`ws://${fixtureHost}:\${server.PORT}/ws\``);
  const modified = source.slice(0, start) + changed + source.slice(end);
  if (!recorded) {
    fs.appendFileSync(receipt, JSON.stringify({
      pid: process.pid,
      workerIndex: process.env.TEST_WORKER_INDEX ?? null,
      sourceSha256: expectedSha256,
      adaptedSha256: crypto.createHash('sha256').update(modified).digest('hex'),
      fixtureHost,
      replacements: { websocket: 1 },
    }) + '\n');
    recorded = true;
  }
  return isBuffer ? Buffer.from(modified) : modified;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function strictCdpDefaultUaReadFileSync(file, ...args) {
  return rewrite(file, originalReadFileSync.call(this, file, ...args));
};
const originalReadFile = fs.readFile;
fs.readFile = function strictCdpDefaultUaReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};
const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function strictCdpDefaultUaPromiseReadFile(file, ...args) {
  return rewrite(file, await originalPromiseReadFile(file, ...args));
};
