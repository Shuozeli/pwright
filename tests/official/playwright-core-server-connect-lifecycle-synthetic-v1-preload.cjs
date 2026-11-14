'use strict';

// Only the upstream run-server fixture launcher is replaced. Both selected
// BrowserType.connect test bodies and all their assertions remain unchanged.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const mode = process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_RECEIPT;
const fixture = process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_SERVER;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !upstream || !receipt || !fixture)
  throw new Error('synthetic lifecycle preload requires its exact mode, sources and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(host || ''))
  throw new Error('synthetic lifecycle fixture requires full Tailscale MagicDNS');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(bind || ''))
  throw new Error('synthetic lifecycle fixture requires a Tailscale IPv4 bind');
if (!path.isAbsolute(fixture) || !fs.statSync(fixture).isFile())
  throw new Error('synthetic lifecycle server must be an absolute regular file');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`synthetic lifecycle fixture forbids ${key}`);
}
const argv = process.argv;
if (argv.some(value => value.endsWith('/remote-server-impl.js')) ||
    (argv.some(value => value.endsWith('/cli.js')) && argv.includes('run-server')))
  throw new Error('synthetic lifecycle guard forbids an upstream browser server');

const source = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const remoteFixture = path.join(path.resolve(upstream), 'tests/config/remoteServer.ts');
const expectedSourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const expectedFixtureSha256 = 'ceb98f2d613b5d1639732c3fa647eba661045ebdce799246d6fc9872f5643f56';
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex');
const sourceText = fs.readFileSync(source, 'utf8');
const fixtureText = fs.readFileSync(remoteFixture, 'utf8');
if (sha256(sourceText) !== expectedSourceSha256 || sha256(fixtureText) !== expectedFixtureSha256)
  throw new Error('pinned upstream connect test or remote-server fixture drifted');
for (const snippet of [
  "test('disconnected event should have browser as argument', async ({ connect, startRemoteServer }) => {",
  "test('should set the browser connected state', async ({ connect, startRemoteServer }) => {",
  "const remoteServer = await startRemoteServer(kind);",
  "expect(disconnected).toBe(browser);",
  "expect(remote.isConnected()).toBe(true);",
  "expect(remote.isConnected()).toBe(false);",
]) {
  if (!sourceText.includes(snippet)) throw new Error(`selected upstream assertion drifted: ${snippet}`);
}
const before = "const command = ['node', path.join(__dirname, '..', '..', 'packages', 'playwright-core', 'cli.js'), 'run-server'];";
const after = "const command = ['node', process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_SERVER!];";
if (fixtureText.split(before).length !== 2)
  throw new Error('upstream run-server launcher changed or appears more than once');
const adaptedFixture = fixtureText.replace(before, after);
const adaptedFixtureSha256 = sha256(adaptedFixture);

let ownerBound = null;
if (mode === 'candidate') {
  if (process.env.PWRIGHT_NATIVE_PROFILE !== 'playwright-server-connect-session')
    throw new Error('synthetic lifecycle candidate requires the remote-session profile');
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const candidate = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(name => candidate[name]._playwright === candidate);
  if (!ownerBound) throw new Error('candidate BrowserType fixture backpointer is missing');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('synthetic lifecycle oracle must not load Pwright');
}

function record(kind, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    kind, mode, pid: process.pid, workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    upstreamSourceSha256: expectedSourceSha256,
    upstreamFixtureSha256: expectedFixtureSha256,
    adaptedFixtureSha256, fixtureHost: host, fixtureBind: bind,
    testSourceAdapted: false, fixtureAdapted: true, ownerBound,
    cdpEndpointPresent: false, ...extra,
  }) + '\n');
}
record('preload');

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? fileURLToPath(value) : '';
  if (Buffer.isBuffer(value)) return path.resolve(value.toString('utf8'));
  return typeof value === 'string' ? path.resolve(value) : '';
}
function adapt(value, raw) {
  if (sourcePath(value) !== remoteFixture) return raw;
  const isBuffer = Buffer.isBuffer(raw);
  const text = isBuffer ? raw.toString('utf8') : raw;
  if (typeof text !== 'string' || sha256(text) !== expectedFixtureSha256)
    throw new Error('upstream remote-server fixture read was not the pinned source');
  record('fixture-source-read');
  return isBuffer ? Buffer.from(adaptedFixture) : adaptedFixture;
}
const readFileSync = fs.readFileSync;
fs.readFileSync = function syntheticLifecycleReadFileSync(value, ...args) {
  return adapt(value, readFileSync.call(this, value, ...args));
};
const readFile = fs.readFile;
fs.readFile = function syntheticLifecycleReadFile(value, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return readFile.call(this, value, ...args);
  args[args.length - 1] = (error, raw) => {
    if (error) return callback(error);
    try { callback(null, adapt(value, raw)); }
    catch (adaptError) { callback(adaptError); }
  };
  return readFile.call(this, value, ...args);
};
const promiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function syntheticLifecyclePromiseReadFile(value, ...args) {
  return adapt(value, await promiseReadFile(value, ...args));
};
