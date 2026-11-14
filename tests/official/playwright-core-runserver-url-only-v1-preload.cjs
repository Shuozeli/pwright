'use strict';

// This narrow preload changes only the endpoint printed by the pinned
// `playwright-core run-server` child. The common official preload owns the
// actual net.Server.listen Tailscale bind; verify the socket before printing.
const fs = require('node:fs');
const net = require('node:net');
const path = require('node:path');

const expectedSuite = 'library-server-connect-version-mismatch-chromium-url-v1';
const suite = process.env.PWRIGHT_OFFICIAL_RUNSERVER_URL_SUITE;
const bind = process.env.PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_BIND;
const host = process.env.PWRIGHT_OFFICIAL_RUNSERVER_EXPECT_HOST;
const receipt = process.env.PWRIGHT_OFFICIAL_RUNSERVER_RECEIPT;
if (suite !== expectedSuite || net.isIP(bind) !== 4 || !host?.endsWith('.ts.net') || !receipt)
  throw new Error('RunServer URL-only preload requires the pinned Tailscale suite and receipt');

const isRunServer = path.basename(process.argv[1] || '') === 'cli.js' &&
  process.argv[1].includes('/packages/playwright-core/') && process.argv[2] === 'run-server';
if (isRunServer) {
  const listeners = [];
  const originalListen = net.Server.prototype.listen;
  net.Server.prototype.listen = function runServerObservedListen(...args) {
    const result = originalListen.apply(this, args);
    listeners.push(this);
    return result;
  };

  const originalLog = console.log;
  console.log = function runServerEndpointLog(...args) {
    if (args.length !== 1 || typeof args[0] !== 'string' || !args[0].startsWith('Listening on '))
      return originalLog.apply(this, args);
    const match = /^Listening on ws:\/\/localhost:(\d+)\/$/.exec(args[0]);
    if (!match) throw new Error('Pinned RunServer emitted an unexpected endpoint');
    const port = Number(match[1]);
    const bound = listeners.filter(server => {
      const address = server.address();
      return address && typeof address !== 'string' && address.port === port;
    });
    if (bound.length !== 1) throw new Error('RunServer endpoint has no unique observed listener');
    const address = bound[0].address();
    if (!address || typeof address === 'string' || address.address !== bind || address.family !== 'IPv4')
      throw new Error('RunServer listener was not bound to the verified Tailscale IPv4');
    const adaptedEndpoint = `ws://${host}:${port}/`;
    fs.writeFileSync(receipt, JSON.stringify({
      schema: 1,
      suite,
      pid: process.pid,
      boundAddress: address.address,
      boundPort: port,
      sourceEndpoint: `ws://localhost:${port}/`,
      adaptedEndpoint,
    }) + '\n', { flag: 'wx', mode: 0o600 });
    return originalLog.call(this, `Listening on ${adaptedEndpoint}`);
  };
}
