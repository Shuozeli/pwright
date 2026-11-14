import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';
import { dirname, isAbsolute } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { compileFunction } from 'node:vm';

const indexUrl = new URL('../src/index.cjs', import.meta.url);
const indexPath = fileURLToPath(indexUrl);
const sourceRequire = createRequire(indexUrl);
const artifactPolicy = sourceRequire('../src/native-artifact.cjs');
const addonPath = process.env.PWRIGHT_CONTEXT_CLOSE_ADDON;
const tailscaleIp = process.env.TAILSCALE_IP;
const tailscaleHost = process.env.TAILSCALE_HOST;
const canRun = Boolean(addonPath && isAbsolute(addonPath)
  && tailscaleIp && tailscaleHost?.endsWith('.ts.net'));

function frame(opcode, payload = Buffer.alloc(0)) {
  if (payload.length < 126)
    return Buffer.concat([Buffer.from([0x80 | opcode, payload.length]), payload]);
  assert.ok(payload.length <= 0xffff, 'bounded fixture frame');
  const header = Buffer.alloc(4);
  header[0] = 0x80 | opcode;
  header[1] = 126;
  header.writeUInt16BE(payload.length, 2);
  return Buffer.concat([header, payload]);
}

function readFrames(socket, onFrame) {
  let pending = Buffer.alloc(0);
  socket.on('data', chunk => {
    pending = Buffer.concat([pending, chunk]);
    while (pending.length >= 2) {
      const opcode = pending[0] & 0x0f;
      const masked = Boolean(pending[1] & 0x80);
      let length = pending[1] & 0x7f;
      let offset = 2;
      if (length === 126) {
        if (pending.length < 4) return;
        length = pending.readUInt16BE(2);
        offset = 4;
      }
      assert.ok(masked, 'client frames must be masked');
      assert.ok(length <= 64 * 1024, 'bounded fixture frame');
      if (pending.length < offset + 4 + length) return;
      const mask = pending.subarray(offset, offset + 4);
      const payload = Buffer.from(pending.subarray(offset + 4, offset + 4 + length));
      for (let index = 0; index < payload.length; index++) payload[index] ^= mask[index % 4];
      pending = pending.subarray(offset + 4 + length);
      onFrame(opcode, payload);
    }
  });
}

function bootstrap() {
  const types = ['chromium', 'firefox', 'webkit'];
  const frames = types.map(name => ({
    guid: '', method: '__create__', params: {
      type: 'BrowserType', guid: `${name}@1`,
      initializer: { name, executablePath: '/browser' },
    },
  }));
  frames.push(
    { guid: '', method: '__create__', params: { type: 'Android', guid: 'android@1', initializer: {} } },
    { guid: '', method: '__create__', params: { type: 'Electron', guid: 'electron@1', initializer: {} } },
    { guid: 'chromium@1', method: '__create__', params: { type: 'Browser', guid: 'browser@1', initializer: { name: 'chromium', version: '155.0' } } },
    { guid: '', method: '__create__', params: { type: 'Playwright', guid: 'playwright@1', initializer: {
      chromium: { guid: 'chromium@1' }, firefox: { guid: 'firefox@1' },
      webkit: { guid: 'webkit@1' }, android: { guid: 'android@1' },
      electron: { guid: 'electron@1' }, preLaunchedBrowser: { guid: 'browser@1' },
    } } },
    { id: 1, result: { playwright: { guid: 'playwright@1' } } },
  );
  return frames;
}

async function fixture(t) {
  const sockets = new Set();
  const commands = [];
  const server = createServer((_request, response) => {
    const body = JSON.stringify({ wsEndpointPath: '/browser-context-close' });
    response.writeHead(200, {
      'content-type': 'application/json', 'content-length': Buffer.byteLength(body),
    });
    response.end(body);
  });
  server.on('connection', socket => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });
  server.on('upgrade', (request, socket) => {
    socket.on('error', () => {});
    const accept = createHash('sha1').update(`${request.headers['sec-websocket-key']}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    const send = value => socket.write(frame(1, Buffer.from(JSON.stringify(value))));
    readFrames(socket, (opcode, payload) => {
      if (opcode === 8) {
        socket.end(frame(8, payload));
        return;
      }
      assert.equal(opcode, 1);
      const message = JSON.parse(payload.toString('utf8'));
      if (message.method === 'initialize') {
        for (const item of bootstrap()) send(item);
        return;
      }
      commands.push(message);
      if (message.method === 'newContext') {
        send({ guid: 'browser@1', method: '__create__', params: { type: 'BrowserContext', guid: 'context@1', initializer: {} } });
        send({ id: message.id, result: { context: { guid: 'context@1' } } });
      } else if (message.method === 'newPage') {
        send({ guid: 'context@1', method: '__create__', params: { type: 'Page', guid: 'page@1', initializer: {} } });
        send({ id: message.id, result: { page: { guid: 'page@1' } } });
      } else if (message.method === 'close') {
        send({ guid: 'page@1', method: '__dispose__', params: {} });
        send({ guid: 'context@1', method: '__dispose__', params: {} });
        send({ id: message.id, result: {} });
      } else {
        assert.fail(`unexpected command ${message.method}`);
      }
    });
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, tailscaleIp, resolve);
  });
  t.after(async () => {
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  });
  return { endpoint: `http://${tailscaleHost}:${server.address().port}/fixture`, commands };
}

function publicApi() {
  const artifact = { path: addonPath, profile: 'playwright-server-connect-session' };
  const binary = sourceRequire(addonPath);
  artifactPolicy.verifyNativeArtifact(binary, artifact);
  assert.equal(typeof binary.NativeRemoteContext.prototype.close, 'function');
  assert.equal(typeof binary.NativeRemoteBrowser.prototype.supportsContextClose, 'function');
  const module = { exports: {} };
  function require(specifier) {
    if (specifier === './native-artifact.cjs')
      return { ...artifactPolicy, nativeArtifact: artifact };
    if (specifier === addonPath) return binary;
    return sourceRequire(specifier);
  }
  compileFunction(readFileSync(indexPath, 'utf8'),
    ['require', 'module', 'exports', '__dirname'], { filename: indexPath })(
    require, module, module.exports, dirname(indexPath));
  return module.exports;
}

test('release remote Context.close sends one correlated command and retires Page first',
  { skip: !canRun }, async t => {
    // Arrange
    const { endpoint, commands } = await fixture(t);
    const api = publicApi();
    const browser = await api.chromium.connect(endpoint);
    const context = await browser.newContext();
    const page = await context.newPage();
    const events = [];
    page.on('close', () => events.push('page'));
    context.on('close', () => events.push('context'));
    browser.on('disconnected', () => events.push('browser'));

    // Act
    await context.close();
    await context.close();

    // Assert
    assert.deepEqual(events, ['page', 'context']);
    assert.deepEqual(commands.map(command => [command.guid, command.method]), [
      ['browser@1', 'newContext'], ['context@1', 'newPage'], ['context@1', 'close'],
    ]);
    assert.deepEqual(commands.map(command => command.params), [{}, {}, {}]);
    assert.equal(commands[2].metadata.apiName, 'browserContext.close');
    assert.equal(page.isClosed(), true);
    assert.deepEqual(context.pages(), []);
    assert.deepEqual(browser.contexts(), []);
    assert.equal(browser.isConnected(), true);
    await browser.close();
    assert.deepEqual(events, ['page', 'context', 'browser']);
  });

test('release remote Browser.newPage owns implicit Context until Page.close',
  { skip: !canRun }, async t => {
    // Arrange
    const { endpoint, commands } = await fixture(t);
    const api = publicApi();
    const browser = await api.chromium.connect(endpoint);
    const page = await browser.newPage();
    const context = page.context();
    const events = [];
    page.on('close', () => events.push('page'));
    context.on('close', () => events.push('context'));
    browser.on('disconnected', () => events.push('browser'));

    // Act
    await page.close();
    await page.close();

    // Assert
    assert.deepEqual(events, ['page', 'context']);
    assert.deepEqual(commands.map(command => [command.guid, command.method]), [
      ['browser@1', 'newContext'], ['context@1', 'newPage'], ['context@1', 'close'],
    ]);
    assert.deepEqual(commands.map(command => command.params), [{}, {}, {}]);
    assert.equal(commands[2].metadata.apiName, 'browserContext.close');
    assert.equal(page.isClosed(), true);
    assert.deepEqual(browser.contexts(), []);
    assert.equal(browser.isConnected(), true);
    await browser.close();
    assert.deepEqual(events, ['page', 'context', 'browser']);
  });
