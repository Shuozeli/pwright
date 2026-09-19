import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

// Test transport only. No production import, browser launch or public listener.
const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const playwrightRequire = createRequire(oracleRequire.resolve('playwright/package.json'));
const core = dirname(playwrightRequire.resolve('playwright-core/package.json'));
const { ws: WebSocket, wsServer: WebSocketServer } = playwrightRequire(join(core, 'lib/utilsBundle.js'));

export async function withCdpCommandRelay(endpoint, gate, action) {
  assert.equal(endpoint, 'http://127.0.0.1:9222');
  const discovery = await fetch(`${endpoint}/json/version`, { signal: AbortSignal.timeout(5000) });
  assert.equal(discovery.status, 200);
  const websocket = new URL((await discovery.json()).webSocketDebuggerUrl);
  assert.equal(websocket.protocol, 'ws:');
  assert.equal(websocket.hostname, '127.0.0.1');
  assert.equal(websocket.port, '9222');
  const server = createServer((_request, response) => response.writeHead(404).end());
  server.maxConnections = 2;
  const sockets = new Set();
  const wsServer = new WebSocketServer({ noServer: true, maxPayload: 4 * 1024 * 1024 });
  server.on('connection', socket => {
    sockets.add(socket);
    socket.once('close', () => sockets.delete(socket));
  });
  let connected = false;
  let upstream;
  server.on('upgrade', (request, socket, head) => {
    if (connected) { socket.destroy(); return; }
    connected = true;
    upstream = new WebSocket(websocket.href, { maxPayload: 4 * 1024 * 1024 });
    upstream.on('error', error => { gate.fail(error); socket.destroy(); });
    upstream.once('open', () => wsServer.handleUpgrade(request, socket, head, client => {
      client.on('error', error => gate.fail(error));
      client.on('message', raw => {
        try { gate.command(raw, client, upstream); }
        catch (error) { gate.fail(error); client.close(); }
      });
      upstream.on('message', raw => {
        try { gate.response(raw, client); }
        catch (error) { gate.fail(error); client.close(); }
      });
      client.once('close', () => upstream.close());
      upstream.once('close', () => client.close());
    }));
  });
  // CDP's security boundary overrides ordinary application Tailscale binding.
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  const relay = `ws://127.0.0.1:${server.address().port}`;
  process.stdout.write(`# diagnostic CDP gate ${relay}\n`);
  try {
    const result = await action(relay, gate);
    gate.check();
    return result;
  } finally {
    gate.release();
    upstream?.terminate();
    for (const client of wsServer.clients) client.terminate();
    wsServer.close();
    for (const socket of sockets) socket.destroy();
    await new Promise(resolve => server.close(resolve));
  }
}

export { WebSocket };
