'use strict';

// Browser-free protocol server for unchanged remote-waiter shutdown assertions.
const { createServer } = require('node:http');
const { promises: dns } = require('node:dns');
const fs = require('node:fs');
const path = require('node:path');

const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_WAITERS_SYNTHETIC_RECEIPT;
const mode = process.env.PWRIGHT_CORE_WAITERS_SYNTHETIC_MODE;
if (!bind || !host || !upstream || !receipt || !['candidate', 'oracle'].includes(mode))
  throw new Error('synthetic waiters server requires pinned lane and Tailscale address');
const { WebSocketServer } = require(require.resolve('ws', { paths: [path.resolve(upstream)] }));
const endpointPath = '/browser-synthetic-waiters-v1';
const server = createServer((request, response) => {
  if (request.method !== 'GET' || request.url !== '/json') {
    response.writeHead(404).end();
    return;
  }
  record('discovery', { requestPath: request.url });
  response.writeHead(200, { 'content-type': 'application/json' });
  response.end(JSON.stringify({ wsEndpointPath: endpointPath }));
});
const websocket = new WebSocketServer({ noServer: true, maxPayload: 65536 });
let nextConnection = 0;
let stopping = false;

function record(kind, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    kind, mode, pid: process.pid, workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    fixtureHost: host, fixtureBind: bind, synthetic: true,
    browserProcessLaunched: false, ...extra,
  }) + '\n');
}

function send(ws, frame) { ws.send(JSON.stringify(frame)); }
function create(ws, parent, type, guid, initializer) {
  send(ws, { guid: parent, method: '__create__', params: { type, guid, initializer } });
}
function initialize(ws, id, connection) {
  for (const name of ['chromium', 'firefox', 'webkit'])
    create(ws, '', 'BrowserType', `${name}@1`, { name, executablePath: '/browser' });
  create(ws, '', 'Android', 'android@1', {});
  create(ws, '', 'Electron', 'electron@1', {});
  create(ws, 'chromium@1', 'Browser', 'browser@1', { name: 'chromium', version: '155.0' });
  create(ws, '', 'Playwright', 'playwright@1', {
    chromium: { guid: 'chromium@1' }, firefox: { guid: 'firefox@1' },
    webkit: { guid: 'webkit@1' }, android: { guid: 'android@1' },
    electron: { guid: 'electron@1' }, preLaunchedBrowser: { guid: 'browser@1' },
  });
  send(ws, { id, result: { playwright: { guid: 'playwright@1' } } });
  record('bootstrap-sent', { connection, frames: 8, version: '155.0' });
}
function newContext(ws, id, connection) {
  create(ws, 'browser@1', 'Tracing', 'context-tracing@1', {});
  create(ws, 'browser@1', 'Tracing', 'request-tracing@1', {});
  create(ws, 'browser@1', 'APIRequestContext', 'request@1', {
    tracing: { guid: 'request-tracing@1' },
  });
  create(ws, 'browser@1', 'BrowserContext', 'context@1', {
    isChromium: true, options: {}, tracing: { guid: 'context-tracing@1' },
    requestContext: { guid: 'request@1' },
  });
  send(ws, { guid: 'browser@1', method: 'context', params: { context: { guid: 'context@1' } } });
  send(ws, { id, result: { context: { guid: 'context@1' } } });
  record('new-context-replied', { connection, id, contextGuid: 'context@1' });
}
function newPage(ws, id, connection) {
  create(ws, 'context@1', 'Frame', 'frame@1', { name: '', url: 'about:blank', loadStates: [] });
  create(ws, 'context@1', 'Page', 'page@1', {
    mainFrame: { guid: 'frame@1' }, isClosed: false,
  });
  send(ws, { guid: 'context@1', method: 'page', params: { page: { guid: 'page@1' } } });
  send(ws, { id, result: { page: { guid: 'page@1' } } });
  record('new-page-replied', { connection, id, pageGuid: 'page@1' });
}

server.on('upgrade', (request, socket, head) => {
  if (request.url !== endpointPath || websocket.clients.size !== 0 ||
      request.headers['x-playwright-browser'] !== 'chromium') {
    socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n');
    socket.destroy();
    return;
  }
  websocket.handleUpgrade(request, socket, head, ws => websocket.emit('connection', ws, request));
});
websocket.on('connection', (ws, request) => {
  const connection = ++nextConnection;
  let phase = 0;
  let forcedByFixtureTeardown = false;
  record('upgrade', { connection, requestPath: request.url,
    browserHeader: request.headers['x-playwright-browser'] ?? null });
  ws.on('message', (payload, binary) => {
    if (binary || payload.length > 65536) { ws.close(1002, 'Invalid frame'); return; }
    let message;
    try { message = JSON.parse(payload.toString('utf8')); }
    catch { ws.close(1002, 'Malformed JSON'); return; }
    record('command', { connection, phase, guid: message.guid, method: message.method, id: message.id });
    if (!Number.isSafeInteger(message.id) || message.id < 1) {
      ws.close(1002, 'Invalid command id'); return;
    }
    if (phase === 0 && message.guid === '' && message.method === 'initialize' &&
        message.params?.sdkLanguage === 'javascript') {
      phase = 1;
      record('root-initialize', { connection, id: message.id, sdkLanguage: 'javascript' });
      initialize(ws, message.id, connection);
    } else if (phase === 1 && message.guid === 'browser@1' && message.method === 'newContext') {
      phase = 2;
      newContext(ws, message.id, connection);
    } else if (phase === 2 && message.guid === 'context@1' && message.method === 'newPage') {
      phase = 3;
      newPage(ws, message.id, connection);
    } else if (phase === 3 && ['page@1', 'context@1'].includes(message.guid) &&
               message.method === 'waitForEventInfo' &&
               typeof message.params?.info?.waitId === 'string' &&
               ['before', 'after', 'log'].includes(message.params.info.phase) &&
               (message.params.info.event === undefined || typeof message.params.info.event === 'string') &&
               (message.params.info.message === undefined || typeof message.params.info.message === 'string') &&
               (message.params.info.error === undefined || typeof message.params.info.error === 'string')) {
      send(ws, { id: message.id, result: {} });
      record('wait-event-info-replied', { connection, id: message.id, guid: message.guid,
        waitId: message.params.info.waitId, phase: message.params.info.phase,
        event: message.params.info.event ?? null });
    } else if (phase === 3 && ['page@1', 'context@1'].includes(message.guid) &&
               message.method === 'updateSubscription' &&
               ['request', 'response'].includes(message.params?.event) &&
               typeof message.params.enabled === 'boolean') {
      send(ws, { id: message.id, result: {} });
      record('subscription-replied', { connection, id: message.id,
        guid: message.guid, event: message.params.event, enabled: message.params.enabled });
    } else {
      record('protocol-error', { connection, phase, guid: message.guid, method: message.method });
      ws.close(1002, 'Unexpected command');
    }
  });
  ws.on('close', (code, reason) => record('close', {
    connection, code, reason: reason.toString('utf8'), phase, forcedByFixtureTeardown,
  }));
  ws.on('error', error => record('websocket-error', { connection, code: error.code ?? null }));
  ws.on('synthetic-force-teardown', () => { forcedByFixtureTeardown = true; });
});

async function start() {
  const resolved = await dns.lookup(host, { family: 4 });
  if (resolved.address !== bind) throw new Error('fixture MagicDNS does not resolve to Tailscale bind');
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
  record('stop-requested');
  server.close();
  setTimeout(() => {
    for (const client of websocket.clients) {
      client.emit('synthetic-force-teardown');
      client.terminate();
    }
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
