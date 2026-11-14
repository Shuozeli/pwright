'use strict';

// This transport-only shim changes the destination hostname to the bound
// fixture's full MagicDNS name. The two official assertions are untouched.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const receipt = process.env.PWRIGHT_CORE_CDP_URL_GUARD_RECEIPT;
if (!upstream || !fixtureHost || !receipt)
  throw new Error('strict CDP URL fixture requires upstream, host, and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost))
  throw new Error('strict CDP URL fixture requires full Tailscale MagicDNS host');

const target = path.join(path.resolve(upstream), 'tests/library/chromium/connect-over-cdp.spec.ts');
const expectedSha256 = 'be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c';
const withSlash = 'browserType.connectOverCDP(`http://localhost:${server.PORT}/browser/?foo=bar`).catch(() => {})';
const withoutSlash = 'browserType.connectOverCDP(`http://localhost:${server.PORT}/browser?foo=bar`).catch(() => {})';
let recorded = false;

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? fileURLToPath(value) : '';
  return typeof value === 'string' ? path.resolve(value) : '';
}

function rewrite(file, raw) {
  if (sourcePath(file) !== target) return raw;
  const isBuffer = Buffer.isBuffer(raw);
  const source = isBuffer ? raw.toString('utf8') : raw;
  if (typeof source !== 'string') throw new Error('strict CDP URL source is not text');
  const actualSha256 = crypto.createHash('sha256').update(source).digest('hex');
  if (actualSha256 !== expectedSha256)
    throw new Error(`strict CDP URL source drifted: ${actualSha256}`);
  if (source.split(withSlash).length !== 2 || source.split(withoutSlash).length !== 2)
    throw new Error('strict CDP URL endpoint markers drifted');
  const adapted = source
      .replace(withSlash, `browserType.connectOverCDP(\`http://${fixtureHost}:\${server.PORT}/browser/?foo=bar\`).catch(() => {})`)
      .replace(withoutSlash, `browserType.connectOverCDP(\`http://${fixtureHost}:\${server.PORT}/browser?foo=bar\`).catch(() => {})`);
  if (!recorded) {
    fs.appendFileSync(receipt, JSON.stringify({
      pid: process.pid,
      workerIndex: process.env.TEST_WORKER_INDEX ?? null,
      sourceSha256: expectedSha256,
      adaptedSha256: crypto.createHash('sha256').update(adapted).digest('hex'),
      fixtureHost,
      replacements: { urlWithSlash: 1, urlWithoutSlash: 1 },
    }) + '\n');
    recorded = true;
  }
  return isBuffer ? Buffer.from(adapted) : adapted;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function strictCdpUrlReadFileSync(file, ...args) {
  return rewrite(file, originalReadFileSync.call(this, file, ...args));
};
const originalReadFile = fs.readFile;
fs.readFile = function strictCdpUrlReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};
const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function strictCdpUrlPromiseReadFile(file, ...args) {
  return rewrite(file, await originalPromiseReadFile(file, ...args));
};
