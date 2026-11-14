import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { createServer, isIP } from 'node:net';
import { createRequire } from 'node:module';
import { arch, platform, release } from 'node:os';
import { lookup } from 'node:dns/promises';
import { test } from 'node:test';

const addonPath = process.env.PWRIGHT_VERSION_MISMATCH_ADDON;
const require = createRequire(import.meta.url);
const bind = process.env.TAILSCALE_IP;
const host = process.env.TAILSCALE_HOST;
const canRun = Boolean(addonPath && existsSync(addonPath) && isIP(bind) === 4 && host?.endsWith('.ts.net'));

function boxedMismatch(extraLine = '') {
  const lines = [
    'Playwright version mismatch:',
    '  - server version: v1.2',
    '  - client version: v1.58',
    '',
    'If you are using VSCode extension, restart VSCode.',
    extraLine,
    '<3 Playwright Team',
  ];
  const width = Math.max(...lines.map(line => line.length));
  const border = '═'.repeat(width + 2);
  return `╔${border}╗\n${lines.map(line => `║ ${line.padEnd(width)} ║`).join('\n')}\n╚${border}╝`;
}

async function rejectedUpgrade(t, status, body) {
  const addresses = new Set((await lookup(host, { family: 4, all: true })).map(item => item.address));
  assert.deepEqual([...addresses], [bind]);
  const sockets = new Set();
  const server = createServer(socket => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
    socket.once('data', () => {
      socket.write(`HTTP/1.1 ${status}\r\n\r\n`);
      setImmediate(() => socket.end(body));
    });
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, bind, resolve);
  });
  t.after(async () => {
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  });
  assert.equal(server.address().address, bind);
  const addon = require(addonPath);
  const native = new addon.NativeEnvironment(() => {}, () => 0);
  return native.connectPlaywrightServer(
    `ws://${host}:${server.address().port}/`,
    2_000,
    [],
    'chromium',
    [platform(), arch(), release()],
  );
}

test('native full-session 428 publishes only constrained version text', { skip: !canRun }, async t => {
  // Arrange
  const body = boxedMismatch('Bearer PRIVATE');

  // Act
  const error = await rejectedUpgrade(t, '428 Precondition Required', body).catch(error => error);

  // Assert
  assert.match(error.message, /PWRIGHT_BACKEND: Playwright version mismatch:/);
  assert.match(error.message, /server version: v1\.2/);
  assert.match(error.message, /client version: v1\.58/);
  assert.doesNotMatch(error.message, /PRIVATE|ws:\/\//);
});

test('native malformed 428 keeps remote body redacted', { skip: !canRun }, async t => {
  // Arrange
  const body = 'Bearer PRIVATE';

  // Act
  const error = await rejectedUpgrade(t, '428 Precondition Required', body).catch(error => error);

  // Assert
  assert.equal(error.message, 'PWRIGHT_BACKEND: Playwright Server WebSocket failed');
});

test('native non-428 keeps remote body redacted', { skip: !canRun }, async t => {
  // Arrange
  const body = boxedMismatch('Bearer PRIVATE');

  // Act
  const error = await rejectedUpgrade(t, '401 Unauthorized', body).catch(error => error);

  // Assert
  assert.equal(error.message, 'PWRIGHT_BACKEND: Playwright Server WebSocket failed');
});
