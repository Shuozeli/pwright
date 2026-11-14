'use strict';

const fs = require('node:fs/promises');

const api = 'browserType.launchPersistentContext';

function browserError(message) {
  return new Error(`${api}: ${message}`);
}

function protocolString(value) {
  // Playwright's protocol tString accepts a boxed String and unwraps it.
  // Other value types belong to protocol validation, not this preflight.
  return value instanceof String ? value.valueOf() : value;
}

async function material(content, path, readFile) {
  if (content) return content;
  if (path) return readFile(path);
  return undefined;
}

async function verifyClientCertificates(certificates, readFile, getNative) {
  if (!certificates?.length) return;

  // Playwright reads material paths on the client before the server checks
  // certificate combinations. Keep that order without acquiring a browser.
  const normalized = await Promise.all(certificates.map(async certificate => ({
    origin: protocolString(certificate.origin),
    cert: await material(certificate.cert, certificate.certPath, readFile),
    key: await material(certificate.key, certificate.keyPath, readFile),
    pfx: await material(certificate.pfx, certificate.pfxPath, readFile),
    passphrase: protocolString(certificate.passphrase),
  })));

  if (getNative) {
    const presence = normalized.map(certificate => ({
      origin: Boolean(certificate.origin),
      cert: Boolean(certificate.cert),
      key: Boolean(certificate.key),
      pfx: Boolean(certificate.pfx),
      passphrase: Boolean(certificate.passphrase),
    }));
    const reason = getNative().persistentContextCertificateError(presence);
    if (reason) throw browserError(reason);
    return;
  }

  for (const certificate of normalized) {
    if (!certificate.origin)
      throw browserError('clientCertificates.origin is required');
    if (!certificate.cert && !certificate.key && !certificate.passphrase && !certificate.pfx)
      throw browserError('None of cert, key, passphrase or pfx is specified');
    if (certificate.cert && !certificate.key)
      throw browserError('cert is specified without key');
    if (!certificate.cert && certificate.key)
      throw browserError('key is specified without cert');
    if (certificate.pfx && (certificate.cert || certificate.key))
      throw browserError('pfx is specified together with cert, key or passphrase');
  }
}

function verifyDefaultArguments(browserName, options, getNative) {
  const channel = protocolString(options.channel);
  if (!Array.isArray(options.args))
    return;

  const args = options.args.map(protocolString);
  if (options.ignoreDefaultArgs && !Array.isArray(options.ignoreDefaultArgs))
    return;
  if (getNative) {
    const reason = getNative().persistentContextArgumentError(browserName,
      Array.from(args, arg => typeof arg === 'string' ? arg : null),
      typeof channel === 'string' ? channel : undefined);
    if (reason) throw browserError(reason);
    return;
  }
  const profileArg = browserName === 'firefox' ? '--profile' : '--user-data-dir';
  if (args.some(arg => typeof arg === 'string' &&
      (browserName === 'firefox'
        ? arg.startsWith('-profile') || arg.startsWith('--profile')
        : arg.startsWith('--user-data-dir')))) {
    throw browserError(`Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '${profileArg}' argument`);
  }
  if (browserName === 'chromium' && args.some(arg => typeof arg === 'string' && arg.startsWith('--remote-debugging-pipe')))
    throw browserError('Playwright manages remote debugging connection itself.');
  const rejectsPageArgument = browserName !== 'firefox'
    || (typeof channel === 'string' && channel.startsWith('moz-'));
  if (rejectsPageArgument && args.some(arg => typeof arg === 'string' && !arg.startsWith('-')))
    throw browserError('Arguments can not specify page to be opened');
}

module.exports = async function preflightPersistentContext(browserName, options, readFile = fs.readFile, getNative) {
  await verifyClientCertificates(options.clientCertificates, readFile, getNative);
  verifyDefaultArguments(browserName, options, getNative);
};
