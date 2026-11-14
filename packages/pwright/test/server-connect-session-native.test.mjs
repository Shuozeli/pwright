import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { lookup } from 'node:dns/promises';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { nativeArtifact, nativeFeatures, verifyNativeArtifact } = require('../src/native-artifact.cjs');
const profile = 'playwright-server-connect-session';
const tailscaleIp = process.env.TAILSCALE_IP;
const tailscaleHost = process.env.TAILSCALE_HOST;
const canRun = Boolean(tailscaleIp && tailscaleHost?.endsWith('.ts.net'));

function bootstrapFrames() {
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
    { id: 1, result: { playwright: { guid: 'playwright@1' } } },
  ];
}

function serverFrame(opcode, payload = Buffer.alloc(0)) {
  if (payload.length < 126) return Buffer.concat([Buffer.from([0x80 | opcode, payload.length]), payload]);
  assert.ok(payload.length <= 0xffff);
  const header = Buffer.alloc(4);
  header[0] = 0x80 | opcode;
  header[1] = 126;
  header.writeUInt16BE(payload.length, 2);
  return Buffer.concat([header, payload]);
}

function readClientFrames(socket, onFrame) {
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
      } else if (length === 127) {
        if (pending.length < 10) return;
        length = Number(pending.readBigUInt64BE(2));
        offset = 10;
      }
      assert.ok(masked, 'client WebSocket frames must be masked');
      assert.ok(length <= 64 * 1024, 'synthetic client frame bound');
      if (pending.length < offset + 4 + length) return;
      const mask = pending.subarray(offset, offset + 4);
      const payload = Buffer.from(pending.subarray(offset + 4, offset + 4 + length));
      for (let index = 0; index < payload.length; index++) payload[index] ^= mask[index % 4];
      pending = pending.subarray(offset + 4 + length);
      onFrame(opcode, payload);
    }
  });
}

async function fixture(t) {
  const observed = { discovery: [], upgrade: [], initialize: [], commands: [], close: 0 };
  const sockets = new Set();
  let firstSendJson;
  let firstSocket;
  const server = createServer((request, response) => {
    observed.discovery.push({ url: request.url, headers: request.headers });
    const body = JSON.stringify({ wsEndpointPath: '/browser-123' });
    response.writeHead(200, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) });
    response.end(body);
  });
  server.on('connection', socket => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });
  server.on('upgrade', (request, socket) => {
    observed.upgrade.push({ url: request.url, headers: request.headers });
    socket.on('error', () => {});
    const accept = createHash('sha1').update(`${request.headers['sec-websocket-key']}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    const sendJson = value => socket.write(serverFrame(1, Buffer.from(JSON.stringify(value))));
    firstSendJson ??= sendJson;
    firstSocket ??= socket;
    readClientFrames(socket, (opcode, payload) => {
      if (opcode === 1) {
        const message = JSON.parse(payload.toString('utf8'));
        if (message.method === 'initialize') {
          observed.initialize.push(message);
          for (const frame of bootstrapFrames()) sendJson(frame);
        } else if (message.method === 'newContext') {
          observed.commands.push(message);
          const guid = 'context@1';
          sendJson({ guid: 'browser@1', method: '__create__', params: { type: 'BrowserContext', guid, initializer: {} } });
          sendJson({ id: message.id, result: { context: { guid } } });
        } else if (message.method === 'newPage') {
          observed.commands.push(message);
          const guid = 'page@1';
          sendJson({ guid: message.guid, method: '__create__', params: { type: 'Page', guid, initializer: {} } });
          sendJson({ id: message.id, result: { page: { guid } } });
        } else {
          assert.fail(`unexpected synthetic command ${message.method}`);
        }
      } else if (opcode === 8) {
        observed.close++;
        socket.end(serverFrame(8, payload));
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
  return {
    endpoint: `http://${tailscaleHost}:${server.address().port}/prefix?token=synthetic`,
    observed,
    sendFirst: value => firstSendJson(value),
    closeFirst: () => firstSocket.end(serverFrame(8, Buffer.from([0x03, 0xe9]))),
  };
}

test('real release addon connects a supervised Browser through Tailscale-bound HTTP and WebSocket', { skip: !canRun }, async t => {
  // Arrange
  assert.equal(nativeArtifact.profile, profile);
  assert.deepEqual(nativeFeatures(profile), [profile]);
  const addon = require(nativeArtifact.path);
  verifyNativeArtifact(addon, nativeArtifact);
  assert.ok(addon.nativeClassNames().includes('NativeRemoteBrowser'));
  const resolved = await lookup(tailscaleHost, { family: 4 });
  assert.equal(resolved.address, tailscaleIp);
  const { endpoint, observed, sendFirst } = await fixture(t);
  const pwright = require('../src/index.cjs');
  const retainedBrowsers = [];
  let firstContext;
  let firstPage;

  // Act: retained, closed wrappers must not consume the sixteen live-session slots.
  for (let index = 0; index < 17; index++) {
    const suppliedEndpoint = index === 0 ? ` \t${endpoint}\r\n` : endpoint;
    const browser = await pwright.chromium.connect(suppliedEndpoint, {
      timeout: 5_000,
      headers: { 'X-Test-Token': 'synthetic' },
    });
    retainedBrowsers.push(browser);
    assert.equal(browser.browserType(), pwright.chromium);
    assert.equal(addon.nativeClassMatches('NativeRemoteBrowser', browser._native), true);
    assert.equal(addon.nativeClassMatches('NativeSession', browser._native), false);
    assert.equal(browser.version(), '155.0');
    assert.equal(browser.isConnected(), true);
    assert.deepEqual(browser.contexts(), []);
    if (index === 0) {
      const context = await browser.newContext();
      firstContext = context;
      assert.equal(addon.nativeClassMatches('NativeRemoteContext', context._native), true);
      assert.equal(context.browser(), browser);
      assert.deepEqual(browser.contexts(), [context]);
      assert.deepEqual(context.pages(), []);
      const page = await context.newPage();
      firstPage = page;
      assert.equal(addon.nativeClassMatches('NativeRemotePage', page._native), true);
      assert.equal(page.context(), context);
      assert.deepEqual(context.pages(), [page]);
      assert.equal(page.isClosed(), false);
      sendFirst({ guid: 'page@1', method: '__dispose__', params: {} });
      const disposeDeadline = Date.now() + 1_000;
      while (!page.isClosed() && Date.now() < disposeDeadline)
        await new Promise(resolve => setTimeout(resolve, 10));
      assert.equal(page.isClosed(), true);
      assert.deepEqual(context.pages(), []);
      assert.throws(() => context.close(), { code: 'PWRIGHT_UNSUPPORTED' });
      assert.throws(() => page.goto('https://example.test'), { code: 'PWRIGHT_UNSUPPORTED' });
    }
    const disconnected = new Promise(resolve => browser.once('disconnected', resolve));
    await browser.close();
    assert.equal(await disconnected, browser);
    await browser.close();
    assert.equal(browser.isConnected(), false);
    if (index === 0) {
      assert.equal(firstPage.isClosed(), true);
      assert.deepEqual(firstContext.pages(), []);
      await assert.rejects(firstContext.newPage(), { code: 'PWRIGHT_TARGET_CLOSED' });
    }
  }
  await assert.rejects(pwright.chromium.connect(endpoint.replace('/prefix', '/pre\nfix'), {
    timeout: 5_000,
  }), { code: 'PWRIGHT_INVALID_ARGUMENT' });

  // Assert
  const closeDeadline = Date.now() + 2_000;
  while (observed.close < 17 && Date.now() < closeDeadline)
    await new Promise(resolve => setTimeout(resolve, 10));
  assert.equal(retainedBrowsers.length, 17);
  assert.equal(observed.discovery.length, 17);
  assert.equal(observed.upgrade.length, 17);
  assert.equal(observed.initialize.length, 17);
  assert.equal(observed.close, 17);
  assert.deepEqual(observed.commands.map(command => [command.guid, command.method]), [
    ['browser@1', 'newContext'], ['context@1', 'newPage'],
  ]);
  assert.deepEqual(observed.commands.map(command => command.params), [{}, {}]);
  assert.deepEqual(observed.commands.map(command => command.metadata.apiName),
    ['browser.newContext', 'browserContext.newPage']);
  assert.ok(observed.commands.every(command => Number.isSafeInteger(command.metadata.wallTime)
    && command.metadata.internal === false));
  for (let index = 0; index < 17; index++) {
    const discovery = observed.discovery[index];
    const upgrade = observed.upgrade[index];
    assert.equal(discovery.url, '/prefix/json?token=synthetic');
    assert.match(discovery.headers['user-agent'], /^Playwright\//);
    assert.equal(discovery.headers['x-test-token'], undefined);
    assert.equal(upgrade.url, '/prefix/browser-123?token=synthetic');
    assert.equal(upgrade.headers['x-test-token'], 'synthetic');
    assert.deepEqual(observed.initialize[index], {
      id: 1, guid: '', method: 'initialize', params: { sdkLanguage: 'javascript' },
      metadata: { wallTime: observed.initialize[index].metadata.wallTime, apiName: 'Connection.init', internal: false },
    });
  }
});

test('remote binding projects unsupported event bursts without disconnecting', { skip: !canRun }, async t => {
  // Arrange
  const { endpoint, sendFirst } = await fixture(t);
  const pwright = require('../src/index.cjs');
  const browser = await pwright.chromium.connect(endpoint, { timeout: 5_000 });

  // Act: the creation reply below is ordered after all 160 server events.
  for (let index = 0; index < 160; index++)
    sendFirst({ guid: 'browser@1', method: 'syntheticNoise', params: { index } });
  const context = await browser.newContext();

  // Assert
  assert.equal(browser.isConnected(), true);
  assert.equal(context._native.isOpen(), true);
  assert.deepEqual(browser.contexts(), [context]);
  await browser.close();
});

test('server terminal close emits Page then Context then Browser from a real remote addon', { skip: !canRun }, async t => {
  // Arrange
  const { endpoint, closeFirst } = await fixture(t);
  const pwright = require('../src/index.cjs');
  const browser = await pwright.chromium.connect(endpoint, { timeout: 5_000 });
  const context = await browser.newContext();
  const page = await context.newPage();
  const events = [];
  page.on('close', value => {
    assert.equal(value, page);
    assert.equal(page.isClosed(), true);
    events.push('page');
  });
  context.on('close', value => {
    assert.equal(value, context);
    events.push('context');
  });
  browser.on('disconnected', value => {
    assert.equal(value, browser);
    events.push('browser');
  });
  const disconnected = new Promise(resolve => browser.once('disconnected', resolve));

  // Act
  closeFirst();
  await disconnected;

  // Assert
  assert.deepEqual(events, ['page', 'context', 'browser']);
  assert.equal(browser.isConnected(), false);
  assert.deepEqual(browser.contexts(), []);
  assert.deepEqual(context.pages(), []);
  await browser.close();
  assert.deepEqual(events, ['page', 'context', 'browser']);
});

test('one outer timeout bounds delayed discovery plus stalled WebSocket bootstrap', { skip: !canRun }, async t => {
  // Arrange
  const sockets = new Set();
  const timers = new Set();
  let upgraded = false;
  const server = createServer((_request, response) => {
    const timer = setTimeout(() => {
      timers.delete(timer);
      const body = JSON.stringify({ wsEndpointPath: '/browser-123' });
      response.writeHead(200, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) });
      response.end(body);
    }, 600);
    timers.add(timer);
  });
  server.on('connection', socket => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });
  server.on('upgrade', (request, socket) => {
    upgraded = true;
    socket.on('error', () => {});
    const accept = createHash('sha1').update(`${request.headers['sec-websocket-key']}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    // Intentionally never answer Root.initialize.
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, tailscaleIp, resolve);
  });
  t.after(async () => {
    for (const timer of timers) clearTimeout(timer);
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  });
  const pwright = require('../src/index.cjs');
  const endpoint = `http://${tailscaleHost}:${server.address().port}/prefix`;

  // Act
  const started = performance.now();
  await assert.rejects(pwright.chromium.connect(endpoint, { timeout: 1_000 }),
    { code: 'PWRIGHT_TIMEOUT' });
  const elapsed = performance.now() - started;

  // Assert: separate 1-second phase budgets would exceed this bound.
  assert.equal(upgraded, true);
  assert.ok(elapsed >= 600 && elapsed < 1_450, `whole connect took ${elapsed} ms`);
});
