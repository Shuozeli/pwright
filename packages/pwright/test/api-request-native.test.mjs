import assert from 'node:assert/strict';
import http from 'node:http';
import net from 'node:net';
import { test } from 'node:test';

import { request } from '@shuozeli/pwright';

test('native API request profile executes all seven public verbs through Rust', async t => {
  // Arrange
  const bind = process.env.TAILSCALE_IP;
  const host = process.env.TAILSCALE_HOST;
  assert.ok(bind, 'TAILSCALE_IP is required');
  assert.ok(host, 'TAILSCALE_HOST is required');
  const received = [];
  const server = http.createServer((incoming, outgoing) => {
    const chunks = [];
    incoming.on('data', chunk => chunks.push(chunk));
    incoming.on('end', () => {
      received.push({ method: incoming.method, body: Buffer.concat(chunks).toString() });
      outgoing.writeHead(201, 'Created', {
        'content-type': 'application/json',
        'set-cookie': ['first=one; Path=/', 'second=two; Path=/'],
      });
      outgoing.end(JSON.stringify({ method: incoming.method }));
    });
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, bind, resolve);
  });
  t.after(() => new Promise((resolve, reject) => {
    server.close(error => error ? reject(error) : resolve());
  }));
  const port = server.address().port;
  const context = await request.newContext({
    baseURL: `http://${host}:${port}`,
    userAgent: 'pwright-native-test',
  });
  t.after(() => context.dispose({ reason: 'native test complete' }));
  const calls = [
    ['fetch', 'GET'],
    ['delete', 'DELETE'],
    ['get', 'GET'],
    ['head', 'HEAD'],
    ['patch', 'PATCH'],
    ['post', 'POST'],
    ['put', 'PUT'],
  ];

  // Act
  const responses = [];
  for (const [name, method] of calls) {
    responses.push(await context[name](`/method/${name}`, {
      ...(name === 'fetch' ? { method } : {}),
      ...(name === 'head' ? {} : { data: name }),
    }));
  }

  // Assert
  assert.deepEqual(received.map(entry => entry.method), calls.map(([, method]) => method));
  assert.deepEqual(received.map(entry => entry.body), ['fetch', 'delete', 'get', '', 'patch', 'post', 'put']);
  for (const [index, response] of responses.entries()) {
    assert.equal(response.status(), 201);
    assert.equal(response.statusText(), 'Created');
    assert.equal(response.ok(), true);
    assert.equal(response.headers()['content-type'], 'application/json');
    assert.equal(response.headersArray().filter(header => header.name.toLowerCase() === 'set-cookie').length, 2);
    if (calls[index][0] === 'head') assert.equal((await response.body()).length, 0);
    else assert.deepEqual(await response.json(), { method: calls[index][1] });
    await response.dispose();
  }
  assert.deepEqual(context.pwrightMetrics(), {
    disposed: false,
    inFlight: 0,
    liveResponses: 0,
    retainedBodyBytes: 0,
    retainedLogBytes: 0,
  });
});

test('native HTTP-proxy profile establishes an authenticated CONNECT tunnel through Rust', async t => {
  // Arrange
  const bind = process.env.TAILSCALE_IP;
  const host = process.env.TAILSCALE_HOST;
  assert.ok(bind, 'TAILSCALE_IP is required');
  assert.ok(host, 'TAILSCALE_HOST is required');
  const target = http.createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end('{"through":"proxy"}');
  });
  await new Promise((resolve, reject) => {
    target.once('error', reject);
    target.listen(0, bind, resolve);
  });
  const targetPort = target.address().port;
  const connectRequests = [];
  const proxy = http.createServer();
  proxy.on('connect', (request, client, head) => {
    connectRequests.push({
      authority: request.url,
      authorization: request.headers['proxy-authorization'],
    });
    const upstream = net.connect(targetPort, bind, () => {
      client.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      if (head.length) upstream.write(head);
      client.pipe(upstream);
      upstream.pipe(client);
    });
    upstream.on('error', error => client.destroy(error));
  });
  await new Promise((resolve, reject) => {
    proxy.once('error', reject);
    proxy.listen(0, bind, resolve);
  });
  t.after(() => Promise.all([
    new Promise((resolve, reject) => target.close(error => error ? reject(error) : resolve())),
    new Promise((resolve, reject) => proxy.close(error => error ? reject(error) : resolve())),
  ]));
  const context = await request.newContext({
    proxy: {
      server: `${host}:${proxy.address().port}`,
      username: 'user',
      password: 'secret',
    },
  });
  t.after(() => context.dispose({ reason: 'native proxy test complete' }));

  // Act
  const response = await context.get(`http://non-existent.example:${targetPort}/simple.json`);

  // Assert
  assert.equal(response.status(), 200);
  assert.deepEqual(await response.json(), { through: 'proxy' });
  assert.deepEqual(connectRequests, [{
    authority: `non-existent.example:${targetPort}`,
    authorization: 'Basic dXNlcjpzZWNyZXQ=',
  }]);
});

test('native proxy-bypass profile selects the direct Rust connector for a matching hostname', async t => {
  // Arrange
  const bind = process.env.TAILSCALE_IP;
  const host = process.env.TAILSCALE_HOST;
  assert.ok(bind, 'TAILSCALE_IP is required');
  assert.ok(host, 'TAILSCALE_HOST is required');
  const target = http.createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end('{"through":"direct"}');
  });
  await new Promise((resolve, reject) => {
    target.once('error', reject);
    target.listen(0, bind, resolve);
  });
  const connectRequests = [];
  const proxy = http.createServer();
  proxy.on('connect', (request, client) => {
    connectRequests.push(request.url);
    client.destroy();
  });
  await new Promise((resolve, reject) => {
    proxy.once('error', reject);
    proxy.listen(0, bind, resolve);
  });
  t.after(() => Promise.all([
    new Promise((resolve, reject) => target.close(error => error ? reject(error) : resolve())),
    new Promise((resolve, reject) => proxy.close(error => error ? reject(error) : resolve())),
  ]));
  const context = await request.newContext({
    proxy: {
      server: `${host}:${proxy.address().port}`,
      bypass: `unrelated.example, .${host}`,
    },
  });
  t.after(() => context.dispose({ reason: 'native proxy bypass test complete' }));

  // Act
  const response = await context.get(`http://${host}:${target.address().port}/simple.json`);

  // Assert
  assert.equal(response.status(), 200);
  assert.deepEqual(await response.json(), { through: 'direct' });
  assert.deepEqual(connectRequests, []);
});
