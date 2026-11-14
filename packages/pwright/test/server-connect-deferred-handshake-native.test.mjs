import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { arch, platform, release } from 'node:os';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const addonPath = process.env.PWRIGHT_DEFERRED_HEADER_ADDON;
const tailscaleIp = process.env.TAILSCALE_IP;
const tailscaleHost = process.env.TAILSCALE_HOST;
const canRun = Boolean(addonPath && tailscaleIp && tailscaleHost?.endsWith('.ts.net'));
const identity = [platform(), arch(), release()];

function environment() {
  const addon = require(addonPath);
  assert.equal(addon.playwrightServerConnectSessionEnabled(), true);
  return new addon.NativeEnvironment(() => {}, () => 0);
}

async function fixture(t, handler) {
  const sockets = new Set();
  const upgrades = [];
  const server = createServer(handler);
  server.on('upgrade', (request, socket) => {
    upgrades.push(request.url);
    socket.destroy();
  });
  server.on('connection', socket => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, tailscaleIp, resolve);
  });
  t.after(async () => {
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  });
  return { endpoint: `http://${tailscaleHost}:${server.address().port}/connect`, upgrades };
}

test('HTTP redirect timeout precedes reserved WebSocket header rejection', { skip: !canRun }, async t => {
  // Arrange
  const observed = [];
  let slowSocketClosed;
  const closed = new Promise(resolve => { slowSocketClosed = resolve; });
  const { endpoint, upgrades } = await fixture(t, (request, response) => {
    observed.push(request.url);
    if (request.url === '/connect/json') {
      response.writeHead(302, { location: '/connect/slow' });
      response.end();
    } else if (request.url === '/connect/slow') {
      request.socket.once('close', slowSocketClosed);
    } else {
      assert.fail(`unexpected discovery path ${request.url}`);
    }
  });
  const native = environment();

  // Act
  const connection = native.connectPlaywrightServer(
    endpoint, 200, [['Connection', 'Close']], 'chromium', identity);

  // Assert
  await assert.rejects(connection, /Timeout 200ms exceeded/);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('slow socket stayed open')), 2_000);
    closed.then(() => { clearTimeout(timer); resolve(); }, reject);
  });
  assert.deepEqual(observed, ['/connect/json', '/connect/slow']);
  assert.deepEqual(upgrades, []);
});

test('successful HTTP discovery rejects reserved header before any WebSocket upgrade', { skip: !canRun }, async t => {
  // Arrange
  const observed = [];
  const { endpoint, upgrades } = await fixture(t, (request, response) => {
    observed.push(request.url);
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end(JSON.stringify({ wsEndpointPath: '/browser-123' }));
  });
  const native = environment();

  // Act
  const connection = native.connectPlaywrightServer(
    endpoint, 2_000, [['Connection', 'Close']], 'chromium', identity);

  // Assert
  await assert.rejects(connection, /handshake-controlled header is reserved/);
  assert.deepEqual(observed, ['/connect/json']);
  assert.deepEqual(upgrades, []);
});

test('direct WS retains strict preflight and HTTP preflight counts reserved headers', { skip: !canRun }, () => {
  // Arrange
  const native = environment();
  const headers = [
    ['Connection', 'Close'],
    ...Array.from({ length: 61 }, (_, index) => [`x-${index}`, 'v']),
  ];

  // Act
  const direct = () => native.connectPlaywrightServer(
    'ws://example.invalid/browser', 200, [['Connection', 'Close']], 'chromium', identity);
  const overBudget = () => native.connectPlaywrightServer(
    'http://example.invalid/connect', 200, headers, 'chromium', identity);

  // Assert
  assert.throws(direct, /handshake-controlled header is reserved/);
  assert.throws(overBudget, /header count.*64/i);
});
