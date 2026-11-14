'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { test } = require('node:test');

const commonPreload = path.join(__dirname, 'playwright-core-server-runtime.cjs');
const securePreload = path.join(__dirname, 'playwright-core-secure-server-runtime.cjs');
const pinnedTestServerSource = [
  '  static async certOptions() {',
  '    return {',
  "      key: await fs.promises.readFile(path.join(__dirname, 'key.pem')),",
  "      cert: await fs.promises.readFile(path.join(__dirname, 'cert.pem')),",
  "      passphrase: 'aaaa',",
  '    };',
  '  }',
].join('\n');
const pinnedServerFixturesSource = [
  "loopback: [undefined, { scope: 'worker', option: true }],",
  'const server = await TestServer.create(assetsPath, port, loopback);',
  'server.enableHTTPCache(cachedPath);',
  'const httpsPort = port + 1;',
  'const httpsServer = await TestServer.createHTTPS(assetsPath, httpsPort, loopback);',
  '    httpsServer.enableHTTPCache(cachedPath);',
].join('\n');

function withFixture(testServerSource, serverFixturesSource, assertFixture) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'pwright-core-secure-preload-test-'));
  try {
    const upstream = path.join(directory, 'upstream');
    const testServer = path.join(upstream, 'tests', 'config', 'testserver', 'index.ts');
    const serverFixtures = path.join(upstream, 'tests', 'config', 'serverFixtures.ts');
    fs.mkdirSync(path.dirname(testServer), { recursive: true });
    fs.writeFileSync(testServer, testServerSource);
    fs.writeFileSync(serverFixtures, serverFixturesSource);
    const certificate = path.join(directory, 'certificate.pem');
    const privateKey = path.join(directory, 'private-key.pem');
    fs.writeFileSync(certificate, 'test-certificate', { mode: 0o600 });
    fs.writeFileSync(privateKey, 'test-private-key', { mode: 0o600 });
    assertFixture({ upstream, testServer, serverFixtures, certificate, privateKey });
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

function runPreloaded(fixture, program, file = fixture.testServer) {
  return spawnSync(process.execPath, [
    '--require', commonPreload,
    '--require', securePreload,
    '-e', program, file, fixture.serverFixtures,
  ], {
    encoding: 'utf8',
    env: {
      ...process.env,
      NODE_OPTIONS: '',
      PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM: fixture.upstream,
      PWRIGHT_OFFICIAL_SERVER_IP: '100.64.0.1',
      PWRIGHT_OFFICIAL_SERVER_HOST: 'host.tailnet.ts.net',
      PWRIGHT_OFFICIAL_SECURE_TLS_CERT: fixture.certificate,
      PWRIGHT_OFFICIAL_SECURE_TLS_KEY: fixture.privateKey,
    },
  });
}

test('secure preload composes with common preload across sync and async reads', () => {
  // Arrange
  withFixture(pinnedTestServerSource, pinnedServerFixturesSource, fixture => {
    const program = `
      const fs = require('node:fs');
      Promise.all(process.argv.slice(1).map(async file => [
        fs.readFileSync(file, 'utf8'),
        fs.readFileSync(file).toString('utf8'),
        await fs.promises.readFile(file, 'utf8'),
        await new Promise((resolve, reject) => fs.readFile(file, 'utf8', (error, source) => error ? reject(error) : resolve(source))),
      ])).then(values => process.stdout.write(JSON.stringify(values)));
    `;

    // Act
    const execution = runPreloaded(fixture, program);

    // Assert
    assert.equal(execution.status, 0, execution.stderr);
    const [testServerVersions, serverFixtureVersions] = JSON.parse(execution.stdout);
    assert.equal(testServerVersions.length, 4);
    assert.equal(serverFixtureVersions.length, 4);
    for (const source of testServerVersions) {
      assert.match(source, /PWRIGHT_OFFICIAL_SECURE_TLS_KEY/);
      assert.match(source, /PWRIGHT_OFFICIAL_SECURE_TLS_CERT/);
      assert.doesNotMatch(source, /passphrase: 'aaaa'/);
    }
    for (const source of serverFixtureVersions) {
      assert.match(source, /TestServer\.createHTTPS\(assetsPath, port, loopback\)/);
      assert.match(source, /CROSS_PROCESS_PREFIX', \{ value: httpsServer\.PREFIX \}/);
      assert.match(source, /loopback: \[process\.env\.PWRIGHT_OFFICIAL_SERVER_HOST/);
    }
  });
});

test('secure preload rejects pinned certificate source drift', () => {
  // Arrange
  withFixture(pinnedTestServerSource.replace("passphrase: 'aaaa'", "passphrase: 'changed'"), pinnedServerFixturesSource, fixture => {
    const program = "require('node:fs').readFileSync(process.argv[1], 'utf8')";

    // Act
    const execution = runPreloaded(fixture, program);

    // Assert
    assert.notEqual(execution.status, 0);
    assert.match(execution.stderr, /pinned official TestServer certificate options source drifted/);
  });
});

test('secure preload rejects pinned ordinary server fixture drift', () => {
  // Arrange
  withFixture(pinnedTestServerSource, pinnedServerFixturesSource.replace('TestServer.create(assetsPath', 'TestServer.createHTTPS(assetsPath'), fixture => {
    const program = "require('node:fs').readFileSync(process.argv[1], 'utf8')";

    // Act
    const execution = runPreloaded(fixture, program, fixture.serverFixtures);

    // Assert
    assert.notEqual(execution.status, 0);
    assert.match(execution.stderr, /pinned official TestServer ordinary fixture construction source drifted/);
  });
});

test('same trusted hostname on two HTTPS ports has distinct origins', () => {
  // Arrange
  const ordinary = new URL('https://host.tailnet.ts.net:8907');
  const crossProcess = new URL('https://host.tailnet.ts.net:8908');

  // Act
  const sameOrigin = ordinary.origin === crossProcess.origin;

  // Assert
  assert.equal(sameOrigin, false);
  assert.equal(ordinary.hostname, crossProcess.hostname);
});
