'use strict';

// Synthetic Playwright protocol fixture, not a BrowserServer and not a browser.
const { createServer } = require('node:http');
const { promises: dns } = require('node:dns');
const fs = require('node:fs');
const path = require('node:path');

const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_RECEIPT;
if (!bind || !host || !upstream || !receipt)
  throw new Error('synthetic server requires Tailscale bind, host, upstream and receipt');
const { WebSocketServer } = require(require.resolve('ws', { paths: [path.resolve(upstream)] }));
const endpointPath = '/browser-synthetic';
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
    kind, mode: process.env.PWRIGHT_CORE_LIFECYCLE_SYNTHETIC_MODE,
    pid: process.pid, workerIndex: process.env.TEST_WORKER_INDEX ?? null,
    fixtureHost: host, fixtureBind: bind, synthetic: true,
    browserProcessLaunched: false, ...extra,
  }) + '\n');
}

function bootstrapFrames(id) {
  return [
    { guid: '', method: '__create__', params: { type: 'BrowserType', guid: 'chromium@1', initializer: { name: 'chromium', executablePath: '/browser' } } },
    { guid: '', method: '__create__', params: { type: 'BrowserType', guid: 'firefox@1', initializer: { name: 'firefox', executablePath: '/browser' } } },
    { guid: '', method: '__create__', params: { type: 'BrowserType', guid: 'webkit@1', initializer: { name: 'webkit', executablePath: '/browser' } } },
    { guid: '', method: '__create__', params: { type: 'Android', guid: 'android@1', initializer: {} } },
    { guid: '', method: '__create__', params: { type: 'Electron', guid: 'electron@1', initializer: {} } },
    { guid: 'chromium@1', method: '__create__', params: { type: 'Browser', guid: 'browser@1', initializer: { name: 'chromium', version: '155.0' } } },
    { guid: '', method: '__create__', params: { type: 'Playwright', guid: 'playwright@1', initializer: {
      chromium: { guid: 'chromium@1' }, firefox: { guid: 'firefox@1' }, webkit: { guid: 'webkit@1' },
      android: { guid: 'android@1' }, electron: { guid: 'electron@1' }, preLaunchedBrowser: { guid: 'browser@1' },
    } } },
    { id, result: { playwright: { guid: 'playwright@1' } } },
  ];
}

server.on('upgrade', (request, socket, head) => {
  if (request.url !== endpointPath || websocket.clients.size >= 4) {
    socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n');
    socket.destroy();
    return;
  }
  websocket.handleUpgrade(request, socket, head, ws => websocket.emit('connection', ws, request));
});
websocket.on('connection', (ws, request) => {
  const connection = ++nextConnection;
  let initialized = false;
  let forcedByFixtureTeardown = false;
  record('upgrade', { connection, requestPath: request.url,
    browserHeader: request.headers['x-playwright-browser'] ?? null });
  ws.on('message', (payload, binary) => {
    if (initialized || binary || payload.length > 65536) {
      ws.close(1002, 'Unexpected protocol frame');
      return;
    }
    let message;
    try { message = JSON.parse(payload.toString('utf8')); }
    catch { ws.close(1002, 'Malformed initialize'); return; }
    if (message.guid !== '' || message.method !== 'initialize' || !Number.isSafeInteger(message.id) ||
        message.id < 1 || !['javascript', 'python'].includes(message.params?.sdkLanguage)) {
      ws.close(1002, 'Unexpected initialize');
      return;
    }
    initialized = true;
    record('root-initialize', { connection, id: message.id, sdkLanguage: message.params.sdkLanguage });
    for (const frame of bootstrapFrames(message.id)) ws.send(JSON.stringify(frame));
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
  if (resolved.address !== bind) throw new Error('fixture MagicDNS does not resolve to its Tailscale bind');
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
  // Give the client's Close frame time to reach the receipt before forcing
  // teardown. This bounded grace is for evidence, not a browser lifetime.
  setTimeout(() => {
    for (const client of websocket.clients) {
      client.emit('synthetic-force-teardown');
      client.terminate();
    }
    websocket.close();
    setTimeout(() => {
      record('stop');
      process.exit(0);
    }, 50);
  }, 200);
  setTimeout(() => process.exit(1), 3000).unref();
});
start().catch(error => {
  record('start-error', { message: String(error) });
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});
