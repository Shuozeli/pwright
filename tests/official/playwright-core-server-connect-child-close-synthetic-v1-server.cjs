'use strict';

// Browser-free Playwright wire fixture for one official-derived close test.
// No browser is spawned; this fixture only implements the exact handshake,
// Browser.newContext and BrowserContext.newPage protocol graph.
const { createServer } = require('node:http');
const { lookup } = require('node:dns/promises');
const fs = require('node:fs');
const path = require('node:path');

const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_CHILD_CLOSE_SYNTHETIC_RECEIPT;
const mode = process.env.PWRIGHT_CORE_CHILD_CLOSE_SYNTHETIC_MODE;
if (!bind || !host || !upstream || !receipt || !['oracle', 'candidate'].includes(mode))
  throw new Error('child-close synthetic server requires explicit Tailscale and diagnostic settings');
for (const key of ['PWRIGHT_CDP_ENDPOINT', 'PWRIGHT_CDP_WS_URL',
  'PWRIGHT_CANDIDATE_CDP_ENDPOINT', 'PWRIGHT_ORACLE_CDP_ENDPOINT']) {
  if (process.env[key]) throw new Error(`child-close synthetic server forbids ${key}`);
}
const { WebSocketServer } = require(require.resolve('ws', { paths: [path.resolve(upstream)] }));
const endpointPath = '/browser-child-close-synthetic';
const server = createServer((request, response) => response.writeHead(404).end());
const websocket = new WebSocketServer({ noServer: true, maxPayload: 65536 });
let connectionCount = 0;
let stopping = false;

function record(kind, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    kind, mode, pid: process.pid, workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    fixtureHost: host, fixtureBind: bind, synthetic: true,
    browserProcessLaunched: false, ...extra,
  }) + '\n');
}

function create(parent, type, guid, initializer) {
  return { guid: parent, method: '__create__', params: { type, guid, initializer } };
}

function bootstrap(id) {
  return [
    create('', 'BrowserType', 'chromium@1', { name: 'chromium', executablePath: '/browser' }),
    create('', 'BrowserType', 'firefox@1', { name: 'firefox', executablePath: '/browser' }),
    create('', 'BrowserType', 'webkit@1', { name: 'webkit', executablePath: '/browser' }),
    create('', 'Android', 'android@1', {}),
    create('', 'Electron', 'electron@1', {}),
    create('chromium@1', 'Browser', 'browser@1', { name: 'chromium', version: '155.0' }),
    create('', 'Playwright', 'playwright@1', {
      chromium: { guid: 'chromium@1' }, firefox: { guid: 'firefox@1' },
      webkit: { guid: 'webkit@1' }, android: { guid: 'android@1' },
      electron: { guid: 'electron@1' }, preLaunchedBrowser: { guid: 'browser@1' },
    }),
    { id, result: { playwright: { guid: 'playwright@1' } } },
  ];
}

server.on('upgrade', (request, socket, head) => {
  if (request.url !== endpointPath || websocket.clients.size !== 0) {
    socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n');
    socket.destroy();
    return;
  }
  websocket.handleUpgrade(request, socket, head, ws => websocket.emit('connection', ws, request));
});
websocket.on('connection', (ws, request) => {
  const connection = ++connectionCount;
  let phase = 'initialize';
  let serverInitiated = false;
  record('upgrade', { connection, requestPath: request.url,
    browserHeader: request.headers['x-playwright-browser'] ?? null });
  const send = frame => ws.send(JSON.stringify(frame));
  const reject = reason => { record('protocol-error', { connection, reason }); ws.close(1002, reason); };
  ws.on('message', (payload, binary) => {
    if (binary || payload.length > 65536) return reject('binary or oversized frame');
    let message;
    try { message = JSON.parse(payload.toString('utf8')); }
    catch { return reject('malformed JSON'); }
    if (!Number.isSafeInteger(message.id) || message.id < 1) return reject('invalid command id');
    if (phase === 'initialize') {
      if (message.guid !== '' || message.method !== 'initialize' || message.params?.sdkLanguage !== 'javascript')
        return reject('unexpected initialize');
      record('root-initialize', { connection, id: message.id, sdkLanguage: message.params.sdkLanguage });
      for (const frame of bootstrap(message.id)) send(frame);
      record('bootstrap-sent', { connection, frames: 8, version: '155.0' });
      phase = 'newContext';
      return;
    }
    if (phase === 'newContext') {
      if (message.guid !== 'browser@1' || message.method !== 'newContext')
        return reject(`unexpected ${message.guid}.${message.method} before newContext`);
      send(create('browser@1', 'Tracing', 'request-tracing@1', {}));
      send(create('browser@1', 'APIRequestContext', 'request@1', { tracing: { guid: 'request-tracing@1' } }));
      send(create('browser@1', 'Tracing', 'context-tracing@1', {}));
      send(create('browser@1', 'BrowserContext', 'context@1', {
        isChromium: true, requestContext: { guid: 'request@1' },
        tracing: { guid: 'context-tracing@1' }, options: {},
      }));
      send({ guid: 'browser@1', method: 'context', params: { context: { guid: 'context@1' } } });
      send({ id: message.id, result: { context: { guid: 'context@1' } } });
      record('context-created', { connection, commandId: message.id,
        guid: 'context@1', initializerFrames: 4, contextEvent: true });
      phase = 'newPage';
      return;
    }
    if (phase === 'newPage') {
      if (message.guid !== 'context@1' || message.method !== 'newPage')
        return reject(`unexpected ${message.guid}.${message.method} before newPage`);
      send(create('context@1', 'Frame', 'frame@1', {
        url: 'about:blank', name: '', loadStates: [],
      }));
      send(create('context@1', 'Page', 'page@1', {
        mainFrame: { guid: 'frame@1' }, isClosed: false,
      }));
      send({ guid: 'context@1', method: 'page', params: { page: { guid: 'page@1' } } });
      send({ id: message.id, result: { page: { guid: 'page@1' } } });
      record('page-created', { connection, commandId: message.id,
        guid: 'page@1', initializerFrames: 2, pageEvent: true });
      phase = 'ready';
      return;
    }
    reject(`unexpected ${message.guid}.${message.method} after page creation`);
  });
  ws.on('close', (code, reason) => record('close', {
    connection, code, reason: reason.toString('utf8'), serverInitiated,
  }));
  ws.on('error', error => record('websocket-error', { connection, code: error.code ?? null }));
  ws.on('child-close-shutdown', () => { serverInitiated = true; });
});

async function start() {
  const resolved = await lookup(host, { family: 4 });
  if (resolved.address !== bind) throw new Error('MagicDNS host differs from Tailscale bind');
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, bind, resolve);
  });
  const port = server.address().port;
  record('listening', { port, endpointPath });
  process.stdout.write(`Listening on ws://${host}:${port}${endpointPath}\n`);
}

process.on('SIGINT', () => {
  if (stopping) return;
  stopping = true;
  record('stop-requested', { openClients: websocket.clients.size });
  server.close();
  for (const client of websocket.clients) {
    client.emit('child-close-shutdown');
    client.close(1001, 'Synthetic server shutdown');
  }
  setTimeout(() => {
    for (const client of websocket.clients) client.terminate();
    websocket.close();
    setTimeout(() => { record('stop'); process.exit(0); }, 50);
  }, 200);
  setTimeout(() => process.exit(1), 3000).unref();
});
start().catch(error => {
  record('start-error', { message: String(error) });
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});
