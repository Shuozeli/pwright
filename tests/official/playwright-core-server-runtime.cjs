'use strict';

const fs = require('node:fs');
const net = require('node:net');
const path = require('node:path');

const bind = process.env.PWRIGHT_OFFICIAL_SERVER_IP;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const crossOriginForwardEnabled = process.env.PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD === '1';
if (!bind) throw new Error('PWRIGHT_OFFICIAL_SERVER_IP is required');
if (!host) throw new Error('PWRIGHT_OFFICIAL_SERVER_HOST is required');
if (!upstream) throw new Error('PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM is required');
if (process.env.PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD &&
    !['0', '1'].includes(process.env.PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD))
  throw new Error('PWRIGHT_OFFICIAL_CROSS_ORIGIN_FORWARD must be 0 or 1');

const originalListen = net.Server.prototype.listen;
net.Server.prototype.listen = function pwrightOfficialTailscaleListen(...args) {
  if (typeof args[0] === 'number') {
    if (typeof args[1] === 'string') args[1] = bind;
    else args.splice(1, 0, bind);
  } else if (args[0] && typeof args[0] === 'object' && 'port' in args[0]) {
    args[0] = { ...args[0], host: bind };
  }
  return originalListen.apply(this, args);
};

const serverFixtures = path.join(path.resolve(upstream), 'tests', 'config', 'serverFixtures.ts');
const clientCertificates = path.join(
  path.resolve(upstream),
  'tests',
  'library',
  'client-certificates.spec.ts',
);
const fetchProxy = path.join(
  path.resolve(upstream),
  'tests',
  'library',
  'fetch-proxy.spec.ts',
);
const proxyConfig = path.join(
  path.resolve(upstream),
  'tests',
  'config',
  'proxy.ts',
);

function sourcePath(value) {
  if (value instanceof URL) return value.protocol === 'file:' ? value.pathname : '';
  return typeof value === 'string' ? path.resolve(value) : '';
}

function rewriteServerFixtures(source, enableCrossOriginForward) {
  const loopbackBefore = "loopback: [undefined, { scope: 'worker', option: true }],";
  const loopbackAfter = "loopback: [process.env.PWRIGHT_OFFICIAL_SERVER_HOST, { scope: 'worker', option: true }],";
  if (!source.includes(loopbackBefore)) throw new Error('official server fixture loopback source drifted');
  source = source.replace(loopbackBefore, loopbackAfter);
  if (!enableCrossOriginForward) return source;

  const fixtureBefore = `    const proxyServer = await TestProxy.create(proxyPort);

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
  const fixtureAfter = `    const proxyServer = await TestProxy.create(proxyPort);

    let crossOriginForward;
    try {
      crossOriginForward = await require(${JSON.stringify(__filename)}).startCrossOriginForward(server.PORT);
      Object.defineProperty(server, 'CROSS_PROCESS_PREFIX', {
        value: \`http://\${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:\${crossOriginForward.port}\`,
      });

      if (new URL(server.CROSS_PROCESS_PREFIX).origin === new URL(server.PREFIX).origin)
        throw new Error('official cross-origin forwarder did not create a distinct origin');
      await run({
        asset: (p: string) => path.join(__dirname, '..', 'assets', ...p.split('/')),
        server,
        httpsServer,
        socksPort,
        proxyServer,
      });
    } finally {
      try {
        if (crossOriginForward) await crossOriginForward.close();
      } finally {
        await Promise.all([
          server.stop(),
          httpsServer.stop(),
          socksServer.close(),
          proxyServer.stop(),
        ]);
      }
    }`;
  if (!source.includes(fixtureBefore)) throw new Error('official cross-origin server fixture source drifted');
  return source.replace(fixtureBefore, fixtureAfter);
}

async function startCrossOriginForward(targetPort) {
  if (!crossOriginForwardEnabled) throw new Error('cross-origin forwarding is not enabled');
  if (!Number.isInteger(targetPort) || targetPort < 1 || targetPort > 65535)
    throw new Error('cross-origin forwarding requires a valid target port');

  const sockets = new Set();
  const listener = net.createServer(client => {
    const target = net.connect({ host: bind, port: targetPort });
    sockets.add(client);
    sockets.add(target);
    const closePair = () => {
      client.destroy();
      target.destroy();
    };
    client.on('error', closePair);
    target.on('error', closePair);
    client.on('close', () => {
      sockets.delete(client);
      target.destroy();
    });
    target.on('close', () => {
      sockets.delete(target);
      client.destroy();
    });
    client.pipe(target);
    target.pipe(client);
  });
  await new Promise((resolve, reject) => {
    listener.once('error', reject);
    listener.listen({ host: bind, port: 0 }, () => {
      listener.removeListener('error', reject);
      resolve();
    });
  });
  const address = listener.address();
  if (!address || typeof address === 'string') throw new Error('cross-origin forwarder has no TCP address');
  let closing;
  return {
    port: address.port,
    close() {
      if (closing) return closing;
      closing = new Promise((resolve, reject) => {
        listener.close(error => error ? reject(error) : resolve());
        for (const socket of sockets) socket.destroy();
      });
      return closing;
    },
  };
}

function rewrite(pathname, value) {
  if (pathname !== serverFixtures && pathname !== clientCertificates && pathname !== fetchProxy && pathname !== proxyConfig) return value;
  const buffer = Buffer.isBuffer(value);
  let source = buffer ? value.toString('utf8') : value;
  if (typeof source !== 'string') return value;
  if (pathname === serverFixtures) {
    source = rewriteServerFixtures(source, crossOriginForwardEnabled);
  } else if (pathname === clientCertificates) {
    const before = "const host = options?.useFakeLocalhost ? 'local.playwright' : '127.0.0.1';";
    const after = 'const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST!;';
    const proxyBefore = '`localhost:${proxyServer.PORT}`';
    const proxyAfter = '`${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:${proxyServer.PORT}`';
    if (!source.includes(before) || !source.includes(proxyBefore))
      throw new Error('official client-certificate server source drifted');
    source = source.replace(before, after).replaceAll(proxyBefore, proxyAfter);
  } else if (pathname === fetchProxy) {
    const httpBefore = '`localhost:${proxyServer.PORT}`';
    const httpAfter = '`${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:${proxyServer.PORT}`';
    const httpsBefore = '`https://localhost:${port}`';
    const httpsAfter = '`https://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:${port}`';
    const socksBefore = '`socks5://localhost:${socksPort}`';
    const socksAfter = '`socks5://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:${socksPort}`';
    if (!source.includes(httpBefore) || !source.includes(httpsBefore) || !source.includes(socksBefore))
      throw new Error('official proxy server source drifted');
    source = source
      .replaceAll(httpBefore, httpAfter)
      .replace(httpsBefore, httpsAfter)
      .replace(socksBefore, socksAfter);
  } else {
    const forwardHost = '`127.0.0.1:${port}`';
    const tailscaleHost = '`${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:${port}`';
    const allowedSocksHosts = "['127.0.0.1', 'fake-localhost-127-0-0-1.nip.io', 'localhost']";
    const tailscaleSocksHosts = "['127.0.0.1', 'fake-localhost-127-0-0-1.nip.io', 'localhost', process.env.PWRIGHT_OFFICIAL_SERVER_HOST!]";
    const socksTarget = "target.connect(forwardPort, '127.0.0.1');";
    const tailscaleSocksTarget = 'target.connect(forwardPort, process.env.PWRIGHT_OFFICIAL_SERVER_HOST!);';
    const socksAddress = '`socks5://127.0.0.1:${port}`';
    const tailscaleSocksAddress = '`socks5://${process.env.PWRIGHT_OFFICIAL_SERVER_HOST!}:${port}`';
    if (!source.includes(forwardHost) || !source.includes(allowedSocksHosts) ||
        !source.includes(socksTarget) || !source.includes(socksAddress))
      throw new Error('official proxy forwarding source drifted');
    source = source
      .replaceAll(forwardHost, tailscaleHost)
      .replace(allowedSocksHosts, tailscaleSocksHosts)
      .replace(socksTarget, tailscaleSocksTarget)
      .replace(socksAddress, tailscaleSocksAddress);
  }
  return buffer ? Buffer.from(source) : source;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function pwrightOfficialReadFileSync(file, ...args) {
  return rewrite(sourcePath(file), originalReadFileSync.call(this, file, ...args));
};

const originalReadFile = fs.readFile;
fs.readFile = function pwrightOfficialReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewrite(sourcePath(file), value));
  };
  return originalReadFile.call(this, file, ...args);
};

const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function pwrightOfficialPromiseReadFile(file, ...args) {
  return rewrite(sourcePath(file), await originalPromiseReadFile(file, ...args));
};

module.exports = { rewriteServerFixtures, startCrossOriginForward };
