import assert from 'node:assert/strict';
import net from 'node:net';
import { test } from 'node:test';

import { request } from '@shuozeli/pwright';

test('native SOCKS5 profile resolves the target hostname at the proxy', async t => {
  // Arrange
  const bind = process.env.TAILSCALE_IP;
  const host = process.env.TAILSCALE_HOST;
  assert.ok(bind, 'TAILSCALE_IP is required');
  assert.ok(host, 'TAILSCALE_HOST is required');
  const requested = [];
  const proxy = net.createServer(socket => {
    let phase = 0;
    let buffered = Buffer.alloc(0);
    socket.on('data', data => {
      buffered = Buffer.concat([buffered, data]);
      while (true) {
        if (phase === 0) {
          if (buffered.length < 3) return;
          assert.deepEqual([...buffered.subarray(0, 3)], [0x05, 0x01, 0x00]);
          buffered = buffered.subarray(3);
          socket.write(Buffer.from([0x05, 0x00]));
          phase = 1;
          continue;
        }
        if (phase === 1) {
          if (buffered.length < 5) return;
          assert.deepEqual([...buffered.subarray(0, 4)], [0x05, 0x01, 0x00, 0x03]);
          const length = buffered[4];
          if (buffered.length < 7 + length) return;
          const targetHost = buffered.subarray(5, 5 + length).toString();
          const targetPort = buffered.readUInt16BE(5 + length);
          requested.push({ host: targetHost, port: targetPort });
          buffered = buffered.subarray(7 + length);
          socket.write(Buffer.from([0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0]));
          phase = 2;
          continue;
        }
        if (phase === 2) {
          if (!buffered.includes('\r\n\r\n')) return;
          socket.end([
            'HTTP/1.1 200 OK',
            'Connection: close',
            'Content-Type: application/json',
            'Content-Length: 19',
            '',
            '{"through":"socks"}',
          ].join('\r\n'));
          phase = 3;
        }
        return;
      }
    });
  });
  await new Promise((resolve, reject) => {
    proxy.once('error', reject);
    proxy.listen(0, bind, resolve);
  });
  t.after(() => new Promise((resolve, reject) => {
    proxy.close(error => error ? reject(error) : resolve());
  }));
  const context = await request.newContext({
    proxy: { server: `socks5://${host}:${proxy.address().port}` },
  });
  t.after(() => context.dispose({ reason: 'native SOCKS5 test complete' }));

  // Act
  const response = await context.get('http://unresolved.example:8080/simple.json');

  // Assert
  assert.equal(response.status(), 200);
  assert.deepEqual(await response.json(), { through: 'socks' });
  assert.deepEqual(requested, [{ host: 'unresolved.example', port: 8080 }]);
});
