'use strict';

const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const test = require('node:test');

function tailscaleEndpoint() {
  try {
    const ip = execFileSync('tailscale', ['ip', '-4'], { encoding: 'utf8' }).trim();
    const status = JSON.parse(execFileSync('tailscale', ['status', '--json'], { encoding: 'utf8' }));
    const host = status.Self.DNSName.replace(/\.$/, '');
    if (!ip || !host) return null;
    return { ip, host };
  } catch {
    return null;
  }
}

const endpoint = tailscaleEndpoint();
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM || path.resolve(__dirname, '../..');
const upstreamFixturePath = path.join(upstream, 'tests', 'config', 'serverFixtures.ts');
const upstreamFixtureSource = fs.existsSync(upstreamFixturePath)
  ? fs.readFileSync(upstreamFixturePath, 'utf8')
  : null;
process.env.PWRIGHT_OFFICIAL_SERVER_IP = endpoint?.ip || 'missing-tailscale-interface';
process.env.PWRIGHT_OFFICIAL_SERVER_HOST = endpoint?.host || 'missing-tailscale-host.invalid';
process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM = upstream;
process.env.PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD = '1';
const { rewriteServerFixtures, startCrossOriginForward } = require('./playwright-core-server-runtime.cjs');

const fixtureSource = `  loopback: [undefined, { scope: 'worker', option: true }],
    const proxyServer = await TestProxy.create(proxyPort);

    await run({
      asset: (p: string) => path.join(__dirname, '..', 'assets', ...p.split('/')),
      server,
      httpsServer,
      socksPort,
      proxyServer,
    });

    await Promise.all([
      server.stop(),
      httpsServer.stop(),
      socksServer.close(),
      proxyServer.stop(),
    ]);`;

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen({ host: endpoint.ip, port: 0 }, () => {
      server.removeListener('error', reject);
      resolve(server.address().port);
    });
  });
}

function request(port) {
  return new Promise((resolve, reject) => {
    const stream = http.get({
      agent: false,
      host: endpoint.ip,
      port,
      path: '/shared-route',
      headers: { Host: `${endpoint.host}:${port}` },
    }, response => {
      let body = '';
      response.setEncoding('utf8');
      response.on('data', chunk => { body += chunk; });
      response.on('end', () => resolve({ status: response.statusCode, body }));
    });
    stream.on('error', reject);
  });
}

async function close(server) {
  server.closeAllConnections();
  await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
}

test('opt-in source rewrite changes the origin and keeps legacy fixtures unchanged', () => {
  // Arrange
  const source = fixtureSource;

  // Act
  const optedIn = rewriteServerFixtures(source, true);
  const legacy = rewriteServerFixtures(source, false);

  // Assert
  assert.match(optedIn, /startCrossOriginForward\(server\.PORT\)/);
  assert.match(optedIn, /let crossOriginForward;\s*try \{\s*crossOriginForward = await/);
  assert.match(optedIn, /new URL\(server\.CROSS_PROCESS_PREFIX\)\.origin === new URL\(server\.PREFIX\)\.origin/);
  assert.match(optedIn, /finally \{\s*try \{\s*if \(crossOriginForward\) await crossOriginForward\.close\(\)/);
  assert.match(optedIn, /finally \{\s*await Promise\.all\(\[\s*server\.stop\(\)/);
  assert.doesNotMatch(legacy, /startCrossOriginForward/);
  assert.match(legacy, /await run\(\{/);
  assert.match(optedIn, /loopback: \[process\.env\.PWRIGHT_OFFICIAL_SERVER_HOST/);
});

test('checked-out upstream server fixture rewrites and transpiles cleanly', { skip: !upstreamFixtureSource }, () => {
  // Arrange
  const typescript = require(path.join(upstream, 'node_modules', 'typescript'));

  // Act
  const rewritten = rewriteServerFixtures(upstreamFixtureSource, true);
  const result = typescript.transpileModule(rewritten, {
    fileName: upstreamFixturePath,
    reportDiagnostics: true,
    compilerOptions: { module: typescript.ModuleKind.CommonJS },
  });

  // Assert
  assert.match(rewritten, /startCrossOriginForward\(server\.PORT\)/);
  assert.match(rewritten, /official cross-origin forwarder did not create a distinct origin/);
  assert.deepEqual(result.diagnostics.filter(item => item.category === typescript.DiagnosticCategory.Error), []);
});

test('fixture startup failure still stops its upstream servers', { skip: !upstreamFixtureSource }, async () => {
  // Arrange
  const typescript = require(path.join(upstream, 'node_modules', 'typescript'));
  const rewritten = rewriteServerFixtures(fixtureSource, true);
  const fragment = rewritten.slice(rewritten.indexOf('    const proxyServer = await TestProxy.create(proxyPort);'));
  const transpiled = typescript.transpileModule(`async function fixture() {\n${fragment}\n}`, {
    fileName: 'serverFixtureFailure.ts',
    compilerOptions: { module: typescript.ModuleKind.CommonJS },
  }).outputText;
  const stopped = [];
  const fakeServer = { PORT: 8907, stop: async () => { stopped.push('http'); } };
  const fakeHTTPS = { stop: async () => { stopped.push('https'); } };
  const fakeSocks = { close: async () => { stopped.push('socks'); } };
  const fakeProxy = { stop: async () => { stopped.push('proxy'); } };
  const fixture = new Function(
    'require', 'TestProxy', 'proxyPort', 'server', 'httpsServer', 'socksServer',
    'socksPort', 'run', 'path', '__dirname', `${transpiled}\nreturn fixture;`,
  )(
    () => ({ startCrossOriginForward: async () => { throw new Error('forwarder bind failed'); } }),
    { create: async () => fakeProxy }, 8910, fakeServer, fakeHTTPS, fakeSocks,
    8909, async () => { throw new Error('run should not start'); }, path, __dirname,
  );

  // Act
  const failure = fixture();

  // Assert
  await assert.rejects(failure, /forwarder bind failed/);
  assert.deepEqual(stopped.sort(), ['http', 'https', 'proxy', 'socks']);
});

test('forwarded origin shares primary HTTP route state', { skip: !endpoint }, async () => {
  // Arrange
  let content = 'before';
  const primary = http.createServer((_request, response) => response.end(content));
  const primaryPort = await listen(primary);
  let forwarder;
  try {
    // Act
    forwarder = await startCrossOriginForward(primaryPort);
    content = 'after';
    const [primaryResponse, forwardedResponse] = await Promise.all([
      request(primaryPort), request(forwarder.port),
    ]);

    // Assert
    assert.notEqual(
      new URL(`http://${endpoint.host}:${primaryPort}`).origin,
      new URL(`http://${endpoint.host}:${forwarder.port}`).origin,
    );
    assert.equal(primaryResponse.status, 200);
    assert.deepEqual(forwardedResponse, primaryResponse);
    assert.equal(forwardedResponse.body, 'after');
  } finally {
    if (forwarder) await forwarder.close();
    await close(primary);
  }
});

test('closing the forwarder rejects later connections', { skip: !endpoint }, async () => {
  // Arrange
  const primary = http.createServer((_request, response) => response.end('ok'));
  const primaryPort = await listen(primary);
  const forwarder = await startCrossOriginForward(primaryPort);
  try {
    await request(forwarder.port);

    // Act
    await forwarder.close();

    // Assert
    await assert.rejects(request(forwarder.port), { code: 'ECONNREFUSED' });
  } finally {
    await forwarder.close();
    await close(primary);
  }
});
