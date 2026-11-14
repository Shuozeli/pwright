'use strict';

// Browser-free protocol fixture for one unchanged pinned Python test body.
const { createServer } = require('node:http');
const { promises: dns } = require('node:dns');
const fs = require('node:fs');
const path = require('node:path');

const bind = process.env.PWRIGHT_OFFICIAL_TAILSCALE_IP;
const host = process.env.PWRIGHT_OFFICIAL_TAILSCALE_HOST;
const upstream = process.env.PWRIGHT_PYTHON_NAV_CLOSED_CORE_UPSTREAM;
const receipt = process.env.PWRIGHT_PYTHON_NAV_CLOSED_RECEIPT;
const mode = process.env.PWRIGHT_PYTHON_NAV_CLOSED_MODE;
const engine = process.env.PWRIGHT_PYTHON_NAV_CLOSED_ENGINE;
if (!bind || !host || !upstream || !receipt || !['oracle', 'candidate'].includes(mode) ||
    !['chromium', 'firefox', 'webkit'].includes(engine)) {
  throw new Error('navigation-closed server requires a pinned lane and engine');
}
const { WebSocketServer } = require(require.resolve('ws', { paths: [path.resolve(upstream)] }));
const endpointPath = '/browser-synthetic-python-navigation-closed-v1';
const server = createServer((_request, response) => response.writeHead(404).end());
const websocket = new WebSocketServer({ noServer: true, maxPayload: 65536 });
let stopping = false;

function record(kind, extra = {}) {
  fs.appendFileSync(receipt, JSON.stringify({
    kind, mode, engine, fixtureHost: host, fixtureBind: bind, pid: process.pid,
    synthetic: true, browserProcessLaunched: false, ...extra,
  }) + '\n');
}

function send(ws, frame) { ws.send(JSON.stringify(frame)); }
function create(ws, parent, type, guid, initializer) {
  send(ws, { guid: parent, method: '__create__', params: { type, guid, initializer } });
}

function initialize(ws, id) {
  for (const name of ['chromium', 'firefox', 'webkit'])
    create(ws, '', 'BrowserType', `${name}@1`, { name, executablePath: '/browser' });
  create(ws, '', 'Android', 'android@1', {});
  create(ws, '', 'Electron', 'electron@1', {});
  create(ws, `${engine}@1`, 'Browser', 'browser@1', { name: engine, version: '155.0' });
  create(ws, '', 'Playwright', 'playwright@1', {
    chromium: { guid: 'chromium@1' }, firefox: { guid: 'firefox@1' },
    webkit: { guid: 'webkit@1' }, android: { guid: 'android@1' },
    electron: { guid: 'electron@1' }, preLaunchedBrowser: { guid: 'browser@1' },
  });
  send(ws, { id, result: { playwright: { guid: 'playwright@1' } } });
  record('bootstrap-sent', { id });
}

function newContext(ws, id) {
  create(ws, 'browser@1', 'Tracing', 'context-tracing@1', {});
  create(ws, 'browser@1', 'Tracing', 'request-tracing@1', {});
  create(ws, 'browser@1', 'APIRequestContext', 'request@1', {
    tracing: { guid: 'request-tracing@1' },
  });
  create(ws, 'browser@1', 'BrowserContext', 'context@1', {
    options: {}, tracing: { guid: 'context-tracing@1' },
    requestContext: { guid: 'request@1' },
  });
  send(ws, { guid: 'browser@1', method: 'context', params: { context: { guid: 'context@1' } } });
  send(ws, { id, result: { context: { guid: 'context@1' } } });
}

function newPage(ws, id) {
  create(ws, 'context@1', 'Frame', 'frame@1', {
    name: '', url: 'about:blank', loadStates: [],
  });
  create(ws, 'context@1', 'Page', 'page@1', {
    mainFrame: { guid: 'frame@1' }, isClosed: false,
  });
  send(ws, { guid: 'context@1', method: 'page', params: { page: { guid: 'page@1' } } });
  send(ws, { id, result: { page: { guid: 'page@1' } } });
}

server.on('upgrade', (request, socket, head) => {
  if (request.url !== endpointPath || websocket.clients.size !== 0 ||
      request.headers['x-playwright-browser'] !== engine) {
    socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n');
    socket.destroy();
    return;
  }
  websocket.handleUpgrade(request, socket, head, ws => websocket.emit('connection', ws));
});

websocket.on('connection', ws => {
  let phase = 0;
  record('upgrade');
  ws.on('message', (payload, binary) => {
    if (binary || payload.length > 65536) { ws.close(1002, 'Invalid frame'); return; }
    let message;
    try { message = JSON.parse(payload.toString('utf8')); }
    catch { ws.close(1002, 'Malformed JSON'); return; }
    record('command', { phase, guid: message.guid, method: message.method, id: message.id });
    if (!Number.isSafeInteger(message.id) || message.id < 1) {
      ws.close(1002, 'Invalid command id'); return;
    }
    if (phase === 0 && message.guid === '' && message.method === 'initialize' &&
        message.params?.sdkLanguage === 'python') {
      phase = 1;
      initialize(ws, message.id);
    } else if (phase === 1 && message.guid === 'browser@1' &&
               message.method === 'newContext') {
      phase = 2;
      newContext(ws, message.id);
    } else if (phase === 2 && message.guid === 'context@1' &&
               message.method === 'newPage') {
      phase = 3;
      newPage(ws, message.id);
    } else {
      record('protocol-error', { phase });
      ws.close(1002, 'Unexpected command');
    }
  });
  ws.on('close', (code, reason) => record('close', {
    code, reason: reason.toString('utf8'), phase,
  }));
  ws.on('error', error => record('websocket-error', { code: error.code ?? null }));
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
  server.close();
  for (const client of websocket.clients) client.terminate();
  websocket.close();
  setTimeout(() => { record('stop'); process.exit(0); }, 50);
  setTimeout(() => process.exit(1), 3000).unref();
});
start().catch(error => {
  record('start-error', { message: String(error) });
  process.stderr.write(`${error.stack || error}\n`);
  process.exitCode = 1;
});
