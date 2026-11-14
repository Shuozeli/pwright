'use strict';

// Official-derived internal diagnostic: adapt only the pinned line-130 fixture
// host expression. The upstream file on disk and its assertion are untouched.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const mode = process.env.PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V4_MODE;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_SERVER_CONNECT_CLOSE_ERROR_V4_RECEIPT;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const fixtureBind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
if (!['candidate', 'oracle'].includes(mode) || !upstream || !receipt)
  throw new Error('v4 Close guard requires mode, upstream and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost || ''))
  throw new Error('v4 Close guard requires full Tailscale MagicDNS host');
if (!/^100\.(?:6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.(?:[0-9]{1,3})\.(?:[0-9]{1,3})$/.test(fixtureBind || ''))
  throw new Error('v4 Close guard requires a Tailscale IPv4 bind');
for (const key of ['PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT',
  'PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL']) {
  if (process.env[key]) throw new Error(`v4 Close guard forbids ${key}`);
}
const guardKey = Symbol.for('pwright.server-connect-close.v4.guard');
const prior = process[guardKey];

const source = path.join(path.resolve(upstream), 'tests/library/browsertype-connect.spec.ts');
const serverFixture = path.join(path.resolve(upstream), 'tests/config/serverFixtures.ts');
const testServerSource = path.join(path.resolve(upstream), 'tests/config/testserver/index.ts');
const sourceSha256 = '9de4d9c8c71b8509887c81220f2eb0042e89e146c65d0311b020d191814ed2bc';
const serverFixtureSha256 = '4c0039a149bd61e8707adddc626b345632561bd343749293ddbe9458f6ac617b';
const adaptedSha256 = '923655fa0381086cd6702fd1aa3c81535e0bf4c06c5d5df826c392bc79aa6a25';
const before = 'const error = await connect(`ws://localhost:${server.PORT}/ws`).catch(e => e);';
const after = 'const error = await connect(`ws://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST}:${server.PORT}/ws`).catch(e => e);';
const nativeReadFileSync = prior?.nativeReadFileSync ?? fs.readFileSync;
const sha256 = value => crypto.createHash('sha256').update(value).digest('hex');
const sourceText = nativeReadFileSync.call(fs, source, 'utf8');
if (sha256(sourceText) !== sourceSha256 || sourceText.split(before).length !== 2)
  throw new Error('v4 Close upstream test source or exact fixture endpoint drifted');
for (const snippet of [
  "test('should print custom ws close error', async ({ connect, server }) => {",
  "ws.close(4123, 'Oh my!');",
  "expect(error.message).toContain('browserType.connect: Oh my!');",
]) {
  if (sourceText.split(snippet).length !== 2)
    throw new Error('v4 Close upstream assertion or custom Close drifted');
}
const adaptedText = sourceText.replace(before, after);
if (sha256(adaptedText) !== adaptedSha256)
  throw new Error('v4 Close adapted source digest drifted');
const serverFixtureText = nativeReadFileSync.call(fs, serverFixture, 'utf8');
if (sha256(serverFixtureText) !== serverFixtureSha256 ||
    serverFixtureText.split('const port = 8907 + workerInfo.workerIndex * 4;').length !== 2)
  throw new Error('v4 Close server fixture port formula drifted');

let ownerBound = null;
if (mode === 'candidate') {
  if (process.env.PWRIGHT_NATIVE_PROFILE !== 'playwright-server-connect-close-error')
    throw new Error('v4 Close candidate profile drifted');
  require('./playwright-core-browser-type-owner-v2-preload.cjs');
  const implementation = require(path.resolve(process.env.PWRIGHT_NODE_CANDIDATE || ''));
  ownerBound = ['chromium', 'firefox', 'webkit'].every(
    name => implementation[name]._playwright === implementation);
  if (!ownerBound) throw new Error('v4 Close candidate BrowserType owner was not bound');
} else if (process.env.PWRIGHT_NODE_CANDIDATE || process.env.PWRIGHT_NATIVE_PROFILE) {
  throw new Error('v4 Close oracle forbids candidate selection');
}

if (prior) {
  if (prior.mode !== mode || prior.upstream !== upstream || prior.receipt !== receipt ||
      prior.fixtureHost !== fixtureHost || prior.fixtureBind !== fixtureBind)
    throw new Error('v4 Close guard reloaded with different runtime identity');
} else {
  process[guardKey] = { mode, upstream, receipt, fixtureHost, fixtureBind, nativeReadFileSync };

  function workerIndex() {
    const raw = process.env.TEST_WORKER_INDEX;
    if (raw === undefined) return null;
    if (!/^(?:0|1)$/.test(raw)) throw new Error('v4 Close requires worker index 0 or 1');
    return Number(raw);
  }

  function record(kind, details = {}) {
    fs.appendFileSync(receipt, JSON.stringify({
      kind,
      pid: process.pid,
      workerIndex: process.env.TEST_WORKER_INDEX ?? null,
      mode,
      sourceSha256,
      adaptedSourceSha256: adaptedSha256,
      fixtureHost,
      fixtureBind,
      ownerBound,
      cdpEndpointPresent: false,
      sourceAdapted: true,
      fixtureAdapted: true,
      ...details,
    }) + '\n');
  }

  record('preload', { replacements: 1 });

  function sourcePath(file) {
    if (file instanceof URL) return file.protocol === 'file:' ? fileURLToPath(file) : '';
    if (Buffer.isBuffer(file)) return path.resolve(file.toString('utf8'));
    return typeof file === 'string' ? path.resolve(file) : '';
  }

  function adapt(file, raw) {
    if (sourcePath(file) !== source) return raw;
    const isBuffer = Buffer.isBuffer(raw);
    const text = isBuffer ? raw.toString('utf8') : raw;
    if (typeof text !== 'string' || sha256(text) !== sourceSha256 || text.split(before).length !== 2)
      throw new Error('v4 Close source read was not the pinned original');
    const modified = text.replace(before, after);
    if (sha256(modified) !== adaptedSha256)
      throw new Error('v4 Close source adaptation was not exact');
    record('source-read', { replacements: 1 });
    return isBuffer ? Buffer.from(modified) : modified;
  }

  const originalReadFileSync = fs.readFileSync;
  fs.readFileSync = function v4CloseReadFileSync(file, ...args) {
    return adapt(file, originalReadFileSync.call(this, file, ...args));
  };
  const originalReadFile = fs.readFile;
  fs.readFile = function v4CloseReadFile(file, ...args) {
    const callback = args.at(-1);
    if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
    args[args.length - 1] = (error, value) => {
      if (error) return callback(error);
      let modified;
      try {
        modified = adapt(file, value);
      } catch (adaptError) {
        return callback(adaptError);
      }
      return callback(null, modified);
    };
    return originalReadFile.call(this, file, ...args);
  };
  const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
  fs.promises.readFile = async function v4ClosePromiseReadFile(file, ...args) {
    return adapt(file, await originalPromiseReadFile(file, ...args));
  };

  const wsEntry = require.resolve('ws', { paths: [path.dirname(testServerSource)] });
  const { WebSocketServer } = require(wsEntry);
  const originalHandleUpgrade = WebSocketServer.prototype.handleUpgrade;
  WebSocketServer.prototype.handleUpgrade = function v4CloseHandleUpgrade(request, socket, head, callback) {
    const index = workerIndex();
    if (index === null) return originalHandleUpgrade.call(this, request, socket, head, callback);
    const port = 8907 + index * 4;
    if (request.url !== '/ws' || socket.localPort !== port)
      throw new Error('v4 Close saw a WebSocket upgrade outside the exact pinned fixture endpoint');
    const requestHost = request.headers.host;
    if (socket.localAddress !== fixtureBind || requestHost !== `${fixtureHost}:${port}`)
      throw new Error('v4 Close upgrade did not reach the Tailscale fixture listener');
    const originalUrl = `ws://localhost:${port}/ws`;
    const mappedUrl = `ws://${fixtureHost}:${port}/ws`;
    const listener = {
      originalUrl,
      mappedUrl,
      port,
      listenerAddress: socket.localAddress,
      listenerPort: socket.localPort,
      listenerFamily: socket.localFamily,
      requestHost,
    };
    return originalHandleUpgrade.call(this, request, socket, head, ws => {
      record('ws-upgrade', listener);
      ws.on('message', message => {
        record('ws-message', { ...listener, messageBytes: Buffer.byteLength(message) });
      });
      ws.on('close', (code, reason) => {
        record('ws-close', { ...listener, code, reason: reason.toString('utf8') });
      });
      return callback(ws);
    });
  };
}
