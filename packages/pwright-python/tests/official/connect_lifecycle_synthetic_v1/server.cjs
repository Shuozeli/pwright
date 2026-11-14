'use strict';

// Browser-free Playwright protocol fixture for the pinned Python lifecycle tests.
const { createServer } = require('node:http');
const { promises: dns } = require('node:dns');
const fs = require('node:fs');
const path = require('node:path');

const bind = process.env.PWRIGHT_OFFICIAL_TAILSCALE_IP;
const host = process.env.PWRIGHT_OFFICIAL_TAILSCALE_HOST;
const upstream = process.env.PWRIGHT_PYTHON_LIFECYCLE_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_PYTHON_LIFECYCLE_RECEIPT;
const mode = process.env.PWRIGHT_PYTHON_LIFECYCLE_MODE;
const expectedEngine = process.env.PWRIGHT_PYTHON_LIFECYCLE_ENGINE;
if (!bind || !host || !upstream || !receipt || !mode || !expectedEngine)
  throw new Error('synthetic Python lifecycle server requires pinned fixture settings');
if (!['chromium', 'firefox', 'webkit'].includes(expectedEngine))
  throw new Error('unexpected engine');
const { WebSocketServer } = require(require.resolve('ws', { paths: [path.resolve(upstream)] }));
const endpointPath = '/browser-synthetic-python';
const server = createServer((_request, response) => response.writeHead(404).end());
const websocket = new WebSocketServer({ noServer: true, maxPayload: 65536 });
let connectionCount = 0;
let stopping = false;

function record(kind, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    kind, mode, fixtureHost: host, fixtureBind: bind, pid: process.pid,
    engine: expectedEngine, synthetic: true, browserProcessLaunched: false,
    ...extra,
  }) + '\n');
}

function frames(id) {
  const browserTypeGuid = `${expectedEngine}@1`;
  return [
    { guid: '', method: '__create__', params: { type: 'BrowserType', guid: 'chromium@1', initializer: { name: 'chromium', executablePath: '/browser' } } },
    { guid: '', method: '__create__', params: { type: 'BrowserType', guid: 'firefox@1', initializer: { name: 'firefox', executablePath: '/browser' } } },
    { guid: '', method: '__create__', params: { type: 'BrowserType', guid: 'webkit@1', initializer: { name: 'webkit', executablePath: '/browser' } } },
    { guid: '', method: '__create__', params: { type: 'Android', guid: 'android@1', initializer: {} } },
    { guid: '', method: '__create__', params: { type: 'Electron', guid: 'electron@1', initializer: {} } },
    { guid: browserTypeGuid, method: '__create__', params: { type: 'Browser', guid: 'browser@1', initializer: { name: expectedEngine, version: '155.0' } } },
    { guid: '', method: '__create__', params: { type: 'Playwright', guid: 'playwright@1', initializer: {
      chromium: { guid: 'chromium@1' }, firefox: { guid: 'firefox@1' }, webkit: { guid: 'webkit@1' },
      android: { guid: 'android@1' }, electron: { guid: 'electron@1' }, preLaunchedBrowser: { guid: 'browser@1' },
    } } },
    { id, result: { playwright: { guid: 'playwright@1' } } },
  ];
}

server.on('upgrade', (request, socket, head) => {
  if (request.url !== endpointPath || websocket.clients.size >= 1 ||
      request.headers['x-playwright-browser'] !== expectedEngine) {
    socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n');
    socket.destroy();
    return;
  }
  websocket.handleUpgrade(request, socket, head, ws => websocket.emit('connection', ws, request));
});
websocket.on('connection', (ws, request) => {
  const connection = ++connectionCount;
  let initialized = false;
  let forcedByFixtureTeardown = false;
  record('upgrade', { connection, requestPath: request.url,
    browserHeader: request.headers['x-playwright-browser'] });
  ws.on('message', (payload, binary) => {
    if (initialized || binary || payload.length > 65536) {
      ws.close(1002, 'Unexpected protocol frame');
      return;
    }
    let message;
    try { message = JSON.parse(payload.toString('utf8')); }
    catch { ws.close(1002, 'Malformed initialize'); return; }
    if (message.guid !== '' || message.method !== 'initialize' || !Number.isSafeInteger(message.id) ||
        message.id < 1 || message.params?.sdkLanguage !== 'python') {
      ws.close(1002, 'Unexpected initialize');
      return;
    }
    initialized = true;
    record('root-initialize', { connection, id: message.id, sdkLanguage: 'python' });
    for (const frame of frames(message.id)) ws.send(JSON.stringify(frame));
    record('bootstrap-sent', { connection, frames: 8, version: '155.0' });
  });
  ws.on('close', (code, reason) => record('close', {
    connection, code, reason: reason.toString('utf8'), forcedByFixtureTeardown,
  }));
  ws.on('error', error => record('websocket-error', { connection, code: error.code ?? null }));
  ws.on('synthetic-force-teardown', () => { forcedByFixtureTeardown = true; });
});

async function start() {
  const resolved = await dns.lookup(host, { family: 4 });
  if (resolved.address !== bind) throw new Error('MagicDNS does not resolve to Tailscale bind');
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
