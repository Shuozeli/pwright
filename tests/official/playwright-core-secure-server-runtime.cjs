'use strict';

// This preload is opt-in. Both pinned fixture servers use one task-owned trusted
// certificate and remain bound to the Tailscale address by the common preload.
// The upstream source on disk is unchanged.
const fs = require('node:fs');
const path = require('node:path');

const upstream = process.env.PWRIGHT_PLAYWRIGHT_CORE_UPSTREAM;
const host = process.env.PWRIGHT_OFFICIAL_SERVER_HOST;
const certificate = process.env.PWRIGHT_OFFICIAL_SECURE_TLS_CERT;
const privateKey = process.env.PWRIGHT_OFFICIAL_SECURE_TLS_KEY;
if (!upstream || !host || !certificate || !privateKey)
  throw new Error('secure official Core fixture requires upstream, MagicDNS host, and task-owned TLS files');
if (!path.isAbsolute(certificate) || !path.isAbsolute(privateKey))
  throw new Error('secure official Core TLS files must have absolute paths');
for (const file of [certificate, privateKey]) {
  const metadata = fs.statSync(file);
  if (!metadata.isFile() || (metadata.mode & 0o077) !== 0)
    throw new Error('secure official Core TLS files must be regular owner-only files');
}

const testServerSource = path.join(path.resolve(upstream), 'tests', 'config', 'testserver', 'index.ts');
const serverFixturesSource = path.join(path.resolve(upstream), 'tests', 'config', 'serverFixtures.ts');
const originalCertOptions = [
  '  static async certOptions() {',
  '    return {',
  "      key: await fs.promises.readFile(path.join(__dirname, 'key.pem')),",
  "      cert: await fs.promises.readFile(path.join(__dirname, 'cert.pem')),",
  "      passphrase: 'aaaa',",
  '    };',
  '  }',
].join('\n');
const secureCertOptions = [
  '  static async certOptions() {',
  '    return {',
  '      key: await fs.promises.readFile(process.env.PWRIGHT_OFFICIAL_SECURE_TLS_KEY!),',
  '      cert: await fs.promises.readFile(process.env.PWRIGHT_OFFICIAL_SECURE_TLS_CERT!),',
  '    };',
  '  }',
].join('\n');
const originalOrdinaryServer = 'const server = await TestServer.create(assetsPath, port, loopback);';
const secureOrdinaryServer = 'const server = await TestServer.createHTTPS(assetsPath, port, loopback);';
const originalSecondServer = [
  'const httpsServer = await TestServer.createHTTPS(assetsPath, httpsPort, loopback);',
  '    httpsServer.enableHTTPCache(cachedPath);',
].join('\n');
const secureSecondServer = [
  originalSecondServer,
  "    Object.defineProperty(server, 'CROSS_PROCESS_PREFIX', { value: httpsServer.PREFIX });",
].join('\n');

function exactOnce(source, before, label) {
  if (source.split(before).length !== 2)
    throw new Error(`pinned official TestServer ${label} source drifted`);
}

function rewriteSecureFixtureSource(file, value) {
  const pathname = file instanceof URL ? (file.protocol === 'file:' ? file.pathname : '') :
    typeof file === 'string' ? path.resolve(file) : '';
  if (pathname !== testServerSource && pathname !== serverFixturesSource) return value;
  const buffer = Buffer.isBuffer(value);
  const source = buffer ? value.toString('utf8') : value;
  if (typeof source !== 'string') return value;
  let rewritten;
  if (pathname === testServerSource) {
    exactOnce(source, originalCertOptions, 'certificate options');
    rewritten = source.replace(originalCertOptions, secureCertOptions);
  } else {
    exactOnce(source, originalOrdinaryServer, 'ordinary fixture construction');
    exactOnce(source, originalSecondServer, 'second fixture construction');
    rewritten = source.replace(originalOrdinaryServer, secureOrdinaryServer)
      .replace(originalSecondServer, secureSecondServer);
  }
  return buffer ? Buffer.from(rewritten) : rewritten;
}

const originalReadFileSync = fs.readFileSync;
fs.readFileSync = function pwrightOfficialSecureReadFileSync(file, ...args) {
  return rewriteSecureFixtureSource(file, originalReadFileSync.call(this, file, ...args));
};

const originalReadFile = fs.readFile;
fs.readFile = function pwrightOfficialSecureReadFile(file, ...args) {
  const callback = args.at(-1);
  if (typeof callback !== 'function') return originalReadFile.call(this, file, ...args);
  args[args.length - 1] = (error, value) => {
    if (error) callback(error);
    else callback(null, rewriteSecureFixtureSource(file, value));
  };
  return originalReadFile.call(this, file, ...args);
};

const originalPromiseReadFile = fs.promises.readFile.bind(fs.promises);
fs.promises.readFile = async function pwrightOfficialSecurePromiseReadFile(file, ...args) {
  return rewriteSecureFixtureSource(file, await originalPromiseReadFile(file, ...args));
};

module.exports = { rewriteSecureFixtureSource };
