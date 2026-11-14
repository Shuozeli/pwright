'use strict';

// Only the pinned WS destination in the third selected test changes host.
// The first two tests use server.PREFIX, which the common fixture preload
// already resolves to the same-port full MagicDNS host. Assertions stay intact.
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const { fileURLToPath } = require('node:url');

const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const fixtureHost = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const receipt = process.env.PWRIGHT_CORE_CDP_ERROR_SLICE_GUARD_RECEIPT;
if (!upstream || !fixtureHost || !receipt)
  throw new Error('strict CDP error slice requires upstream, host, and receipt');
if (!/^[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*\.ts\.net$/.test(fixtureHost))
  throw new Error('strict CDP error slice requires full Tailscale MagicDNS host');

const target = path.join(path.resolve(upstream), 'tests/library/chromium/connect-over-cdp.spec.ts');
const expectedSha256 = 'be1e552ad56652303ce851bf026aef01cbba7ffd634e8bfc36c3313e2586226c';
const titles = [
  "test('should report an expected error when the endpointURL returns a non-expected status code'",
  "test('should report an expected error when the endpoint URL JSON webSocketDebuggerUrl is undefined'",
  "test('should print custom ws close error'",
];
const exactAssertions = [
  ')).rejects.toThrowError(`browserType.connectOverCDP: Unexpected status 404 when connecting to ${server.PREFIX}/json/version/`);',
  ")).rejects.toThrowError('browserType.connectOverCDP: Invalid URL');",
  'expect(error.message).toContain(`Browser logs:\\n\\nOh my!\\n`);',
];
const wsBefore = 'browserType.connectOverCDP(`ws://localhost:${server.PORT}/ws`).catch(e => e)';
let recorded = false;

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? fileURLToPath(value) : '';
  return typeof value === 'string' ? path.resolve(value) : '';
}

function selectedBodies(source) {
  return titles.map((title, index) => {
    const start = source.indexOf(title);
    if (start < 0 || source.indexOf(title, start + title.length) !== -1)
      throw new Error('strict CDP error slice title drifted');
    const nextTest = source.indexOf('\ntest(', start + title.length);
    const end = nextTest < 0 ? source.length : nextTest;
    const body = source.slice(start, end);
    if (!body.includes(exactAssertions[index]))
      throw new Error('strict CDP error slice assertion drifted');
    return { start, end, body };
  });
}

function rewrite(file, raw) {
  if (sourcePath(file) !== target) return raw;
  const isBuffer = Buffer.isBuffer(raw);
  const source = isBuffer ? raw.toString('utf8') : raw;
  if (typeof source !== 'string') throw new Error('strict CDP error slice source is not text');
  const actualSha256 = crypto.createHash('sha256').update(source).digest('hex');
  if (actualSha256 !== expectedSha256)
    throw new Error(`strict CDP error slice source drifted: ${actualSha256}`);
  const bodies = selectedBodies(source);
  const ws = bodies[2];
  if (ws.body.split(wsBefore).length !== 2)
    throw new Error('strict CDP error slice WS endpoint marker drifted');
  const changed = ws.body.replace(
      wsBefore,
      `browserType.connectOverCDP(\`ws://${fixtureHost}:\${server.PORT}/ws\`).catch(e => e)`);
  const adapted = source.slice(0, ws.start) + changed + source.slice(ws.end);
  if (!recorded) {
    fs.appendFileSync(receipt, JSON.stringify({
      pid: process.pid,
      workerIndex: process.env.TEST_WORKER_INDEX ?? null,
      sourceSha256: expectedSha256,
      adaptedSha256: crypto.createHash('sha256').update(adapted).digest('hex'),
      fixtureHost,
      replacements: { websocket: 1, serverPrefixCases: 0 },
    }) + '\n');
    recorded = true;
  }
  return isBuffer ? Buffer.from(adapted) : adapted;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function strictCdpErrorSliceReadFileSync(file, ...args) {
  return rewrite(file, originalReadFileSync.call(this, file, ...args));
};
const originalReadFile = fs.readFile;
fs.readFile = function strictCdpErrorSliceReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};
const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function strictCdpErrorSlicePromiseReadFile(file, ...args) {
  return rewrite(file, await originalPromiseReadFile(file, ...args));
};
