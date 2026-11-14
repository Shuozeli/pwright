'use strict';

const os = require('node:os');
const util = require('node:util');
const fs = require('node:fs/promises');
const { readFileSync } = require('node:fs');

module.exports = function apiRequestApi({ nativeAddon, validateOptions, UnsupportedOperationError }) {
  const contexts = new Set();

  function rejectUnsupportedRecognized(options, supported, recognized, api) {
    for (const key of Object.keys(options)) {
      if (recognized.includes(key) && !supported.includes(key))
        throw new UnsupportedOperationError(`${api} option ${key}`);
    }
  }

  function integer(value, name, fallback) {
    value ??= fallback;
    if (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff)
      throw new TypeError(`${name} must be an integer in [0, 4294967295]`);
    return value;
  }

  function headers(value, api) {
    if (value === undefined) return [];
    if (!value || typeof value !== 'object') throw new TypeError(`${api}: headers must be an object or array`);
    const entries = Array.isArray(value)
      ? value.map(item => {
        if (!item || typeof item !== 'object') throw new TypeError(`${api}: header must be an object`);
        return [item.name, item.value];
      })
      : Object.entries(value);
    return entries.map(([name, headerValue]) => {
      if (typeof name !== 'string' || typeof headerValue !== 'string')
        throw new TypeError(`${api}: header name and value must be strings`);
      return { name, value: headerValue };
    });
  }

  function defaultUserAgent() {
    let identifier = 'unknown';
    let version = 'unknown';
    try {
      if (process.platform === 'darwin') {
        identifier = 'macOS';
        const systemVersion = readFileSync(
          '/System/Library/CoreServices/SystemVersion.plist',
          'utf8',
        );
        const productVersion = /<key>ProductVersion<\/key>\s*<string>([^<]+)<\/string>/.exec(systemVersion)?.[1];
        if (productVersion) version = productVersion.split('.').slice(0, 2).join('.');
      } else if (process.platform === 'win32') {
        identifier = 'windows';
        version = os.release().split('.').slice(0, 2).join('.');
      } else if (process.platform === 'linux') {
        identifier = 'linux';
      }
    } catch (_) {}
    const nodeVersion = process.version.substring(1).split('.').slice(0, 2).join('.');
    const ci = process.env.CI ? ' CI/1' : '';
    return `Playwright/1.58.0 (${os.arch()}; ${identifier} ${version}) node/${nodeVersion}${ci}`;
  }

  function validateClientCertificates(value, api) {
    if (value === undefined) return;
    if (!Array.isArray(value))
      throw new TypeError(`${api}: clientCertificates must be an array`);
    for (const certificate of value) {
      if (!certificate || typeof certificate !== 'object' || Array.isArray(certificate))
        throw new TypeError(`${api}: clientCertificates entries must be objects`);
      const hasCert = certificate.cert !== undefined || certificate.certPath !== undefined;
      const hasKey = certificate.key !== undefined || certificate.keyPath !== undefined;
      const hasPfx = certificate.pfx !== undefined || certificate.pfxPath !== undefined;
      const hasPassphrase = certificate.passphrase !== undefined;
      if (typeof certificate.origin !== 'string' || certificate.origin.length === 0)
        throw new Error('clientCertificates.origin is required');
      if (!hasCert && !hasKey && !hasPfx && !hasPassphrase)
        throw new Error('None of cert, key, passphrase or pfx is specified');
      if (hasCert && !hasKey) throw new Error('cert is specified without key');
      if (!hasCert && hasKey) throw new Error('key is specified without cert');
      if (hasPfx && (hasCert || hasKey))
        throw new Error('pfx is specified together with cert, key or passphrase');
      for (const path of ['certPath', 'keyPath', 'pfxPath']) {
        if (certificate[path] !== undefined && typeof certificate[path] !== 'string')
          throw new TypeError(`${api}: clientCertificates.${path} must be a string`);
      }
      for (const content of ['cert', 'key', 'pfx']) {
        if (certificate[content] !== undefined && !Buffer.isBuffer(certificate[content]))
          throw new TypeError(`${api}: clientCertificates.${content} must be a Buffer`);
      }
      if (hasPassphrase && typeof certificate.passphrase !== 'string')
        throw new TypeError(`${api}: clientCertificates.passphrase must be a string`);
    }
  }

  function validateProxy(proxy, api) {
    if (proxy === undefined) return undefined;
    if (!proxy || typeof proxy !== 'object' || Array.isArray(proxy))
      throw new TypeError(`${api}: proxy must be an object`);
    for (const key of Object.keys(proxy)) {
      if (!['server', 'username', 'password', 'bypass'].includes(key))
        throw new UnsupportedOperationError(`${api} option proxy.${key}`);
    }
    if (typeof proxy.server !== 'string' || proxy.server.length === 0)
      throw new TypeError(`${api}: proxy.server must be a non-empty string`);
    if (proxy.username !== undefined && typeof proxy.username !== 'string')
      throw new TypeError(`${api}: proxy.username must be a string`);
    if (proxy.password !== undefined && typeof proxy.password !== 'string')
      throw new TypeError(`${api}: proxy.password must be a string`);
    if (proxy.bypass !== undefined && typeof proxy.bypass !== 'string')
      throw new TypeError(`${api}: proxy.bypass must be a string`);
    return /^socks/i.test(proxy.server) ? 'socks' : 'http';
  }

  async function normalizeClientCertificates(value) {
    if (value === undefined) return [];
    return Promise.all(value.map(async certificate => ({
      origin: certificate.origin,
      cert: certificate.cert === undefined
        ? certificate.certPath === undefined ? undefined : await fs.readFile(certificate.certPath)
        : Buffer.from(certificate.cert),
      key: certificate.key === undefined
        ? certificate.keyPath === undefined ? undefined : await fs.readFile(certificate.keyPath)
        : Buffer.from(certificate.key),
      pfx: certificate.pfx === undefined
        ? certificate.pfxPath === undefined ? undefined : await fs.readFile(certificate.pfxPath)
        : Buffer.from(certificate.pfx),
      passphrase: certificate.passphrase,
    })));
  }

  function mapRequestError(error, api) {
    if (error instanceof UnsupportedOperationError) return error;
    const message = error && typeof error.message === 'string' ? error.message : String(error);
    const timeout = /^API request timed out after (\d+)ms$/.exec(message);
    if (timeout) return new Error(`${api}: Timeout ${timeout[1]}ms exceeded`);
    if (message === 'API response body is disposed') return new Error(`${api}: Response has been disposed`);
    return new Error(`${api}: ${message}`);
  }

  function contextOptions(options, origins) {
    const api = 'request.newContext';
    options = { ...options };
    const supported = [
      'baseURL', 'userAgent', 'extraHTTPHeaders', 'timeout', 'failOnStatusCode',
      'ignoreHTTPSErrors', 'maxRedirects', 'httpCredentials', 'storageState',
      'clientCertificates', 'proxy',
    ];
    rejectUnsupportedRecognized(
      options,
      supported,
      supported,
      api,
    );
    if (options.baseURL !== undefined && typeof options.baseURL !== 'string')
      throw new TypeError(`${api}: baseURL must be a string`);
    if (options.userAgent !== undefined && typeof options.userAgent !== 'string')
      throw new TypeError(`${api}: userAgent must be a string`);
    if (options.failOnStatusCode !== undefined && typeof options.failOnStatusCode !== 'boolean')
      throw new TypeError(`${api}: failOnStatusCode must be a boolean`);
    if (options.ignoreHTTPSErrors !== undefined && typeof options.ignoreHTTPSErrors !== 'boolean')
      throw new TypeError(`${api}: ignoreHTTPSErrors must be a boolean`);
    const credentials = options.httpCredentials;
    if (credentials !== undefined) {
      if (!credentials || typeof credentials !== 'object' || Array.isArray(credentials))
        throw new TypeError(`${api}: httpCredentials must be an object`);
      for (const key of Object.keys(credentials)) {
        if (!['username', 'password', 'origin', 'send'].includes(key))
          throw new UnsupportedOperationError(`${api} option httpCredentials.${key}`);
      }
      if (typeof credentials.username !== 'string' || typeof credentials.password !== 'string')
        throw new TypeError(`${api}: httpCredentials username and password must be strings`);
      if (credentials.origin !== undefined && typeof credentials.origin !== 'string')
        throw new TypeError(`${api}: httpCredentials origin must be a string`);
      if (credentials.send !== undefined && !['unauthorized', 'always'].includes(credentials.send))
        throw new TypeError(`${api}: httpCredentials.send must be "unauthorized" or "always"`);
    }
    const proxy = options.proxy;
    validateProxy(proxy, api);
    return {
      baseUrl: options.baseURL,
      userAgent: options.userAgent ?? defaultUserAgent(),
      extraHeaders: headers(options.extraHTTPHeaders, api),
      timeoutMs: integer(options.timeout, `${api}: timeout`, 30_000),
      failOnStatusCode: options.failOnStatusCode ?? false,
      ignoreHttpsErrors: options.ignoreHTTPSErrors ?? false,
      maxRedirects: integer(options.maxRedirects, `${api}: maxRedirects`, 20),
      httpUsername: credentials?.username,
      httpPassword: credentials?.password,
      httpOrigin: credentials?.origin,
      httpSendAlways: credentials?.send === 'always',
      storageOriginsJson: JSON.stringify(origins),
      clientCertificates: options.clientCertificates,
      proxyServer: proxy?.server,
      proxyUsername: proxy?.username,
      proxyPassword: proxy?.password,
      proxyBypass: proxy?.bypass,
    };
  }

  function storageCookie(value, api) {
    if (!value || typeof value !== 'object' || Array.isArray(value))
      throw new TypeError(`${api}: storageState cookie must be an object`);
    const stringFields = ['name', 'value', 'domain', 'path', 'sameSite'];
    for (const field of stringFields) {
      if (typeof value[field] !== 'string')
        throw new TypeError(`${api}: storageState cookie ${field} must be a string`);
    }
    if (typeof value.expires !== 'number' || !Number.isFinite(value.expires))
      throw new TypeError(`${api}: storageState cookie expires must be a finite number`);
    if (typeof value.httpOnly !== 'boolean' || typeof value.secure !== 'boolean')
      throw new TypeError(`${api}: storageState cookie flags must be booleans`);
    if (!['Strict', 'Lax', 'None'].includes(value.sameSite))
      throw new TypeError(`${api}: storageState cookie sameSite must be Strict, Lax, or None`);
    return {
      name: value.name,
      value: value.value,
      domain: value.domain,
      path: value.path,
      expires: value.expires,
      httpOnly: value.httpOnly,
      secure: value.secure,
      sameSite: value.sameSite,
    };
  }

  async function normalizeStorageState(value) {
    const api = 'request.newContext';
    if (value === undefined) return { cookies: [], origins: [] };
    if (typeof value === 'string') {
      const source = await fs.readFile(value, 'utf8');
      value = JSON.parse(source);
    }
    if (!value || typeof value !== 'object' || Array.isArray(value))
      throw new TypeError(`${api}: storageState must be an object or path`);
    const cookies = value.cookies ?? [];
    const origins = value.origins ?? [];
    if (!Array.isArray(cookies) || !Array.isArray(origins))
      throw new TypeError(`${api}: storageState cookies and origins must be arrays`);
    return {
      cookies: cookies.map(cookie => storageCookie(cookie, api)),
      origins: JSON.parse(JSON.stringify(origins)),
    };
  }

  function requestOptions(options, api) {
    options = { ...options };
    const supported = [
      'headers', 'data', 'timeout', 'failOnStatusCode', 'ignoreHTTPSErrors',
      'maxRedirects', 'maxRetries', 'method', '__testHookLookup',
    ];
    rejectUnsupportedRecognized(
      options,
      supported,
      [...supported, 'form', 'multipart', 'params', 'request'],
      api,
    );
    let body;
    const requestHeaders = headers(options.headers, api);
    if (options.data !== undefined) {
      if (Buffer.isBuffer(options.data) || ArrayBuffer.isView(options.data)) {
        body = Buffer.from(options.data.buffer, options.data.byteOffset, options.data.byteLength);
      } else if (typeof options.data === 'string') {
        const json = requestHeaders.some(header =>
          header.name.toLowerCase() === 'content-type' && header.value === 'application/json');
        if (json) {
          try {
            JSON.parse(options.data);
            body = Buffer.from(options.data);
          } catch (_) {
            body = Buffer.from(JSON.stringify(options.data));
          }
        } else {
          body = Buffer.from(options.data);
        }
      } else {
        body = Buffer.from(JSON.stringify(options.data));
        if (!requestHeaders.some(header => header.name.toLowerCase() === 'content-type'))
          requestHeaders.push({ name: 'content-type', value: 'application/json' });
      }
    }
    if (options.failOnStatusCode !== undefined && typeof options.failOnStatusCode !== 'boolean')
      throw new TypeError(`${api}: failOnStatusCode must be a boolean`);
    if (options.ignoreHTTPSErrors !== undefined && typeof options.ignoreHTTPSErrors !== 'boolean')
      throw new TypeError(`${api}: ignoreHTTPSErrors must be a boolean`);
    if (options.maxRedirects !== undefined && options.maxRedirects < 0)
      throw new Error(`'maxRedirects' must be greater than or equal to '0'`);
    if (options.maxRetries !== undefined && options.maxRetries < 0)
      throw new Error(`'maxRetries' must be greater than or equal to '0'`);
    return {
      headers: requestHeaders,
      body,
      timeoutMs: options.timeout === undefined ? undefined : integer(options.timeout, `${api}: timeout`),
      failOnStatusCode: options.failOnStatusCode,
      ignoreHttpsErrors: options.ignoreHTTPSErrors,
      maxRedirects: options.maxRedirects === undefined ? undefined : integer(options.maxRedirects, `${api}: maxRedirects`),
      maxRetries: integer(options.maxRetries, `${api}: maxRetries`, 0),
    };
  }

  class APIResponse {
    constructor(native) { this._native = native; }
    url() { return this._native.url; }
    status() { return this._native.status; }
    statusText() { return this._native.statusText; }
    ok() { return this._native.ok; }
    headersArray() { return this._native.headersArray().map(header => ({ ...header })); }
    headers() {
      const result = {};
      for (const { name, value } of this.headersArray()) {
        const key = name.toLowerCase();
        result[key] = result[key] === undefined ? value : `${result[key]}, ${value}`;
      }
      return result;
    }
    async body() {
      try { return Buffer.from(this._native.body()); }
      catch (error) { throw mapRequestError(error, 'apiResponse.body'); }
    }
    async text() { return (await this.body()).toString('utf8'); }
    async json() { return JSON.parse(await this.text()); }
    async dispose() { this._native.dispose(); }
    async log() {
      try { return this._native.log(); }
      catch (error) { throw mapRequestError(error, 'apiResponse.log'); }
    }
    // Playwright Test's toBeOK matcher reads the same Rust-owned bounded log
    // through this client-internal port. Keep the public log() method as the
    // single conversion/error boundary.
    async _fetchLog() { return this.log(); }
    [util.inspect.custom]() {
      const headers = this.headersArray().map(({ name, value }) => `  ${name}: ${value}`);
      return `APIResponse: ${this.status()} ${this.statusText()}\n${headers.join('\n')}`;
    }
  }

  class APIRequestContext {
    constructor(native, baseURL) { this._native = native; this._baseURL = baseURL; this._disposed = false; }
    async _send(apiName, method, url, options) {
      if (typeof url !== 'string') throw new TypeError(`${apiName}: url must be a string`);
      if (this._disposed)
        throw new Error(`${apiName}: Target page, context or browser has been closed`);
      try {
        const nativeOptions = requestOptions(options, apiName);
        const lookup = options?.__testHookLookup;
        if (lookup !== undefined) {
          if (typeof lookup !== 'function')
            throw new TypeError(`${apiName}: __testHookLookup must be a function`);
          const target = this._baseURL === undefined ? new URL(url) : new URL(url, this._baseURL);
          const addresses = await lookup(target.hostname);
          if (!Array.isArray(addresses))
            throw new TypeError(`${apiName}: __testHookLookup must return an array`);
          nativeOptions.lookupAddresses = addresses.map(value => {
            if (!value || typeof value !== 'object' || typeof value.address !== 'string')
              throw new TypeError(`${apiName}: resolver entries must contain an address string`);
            if (value.family !== undefined && value.family !== 4 && value.family !== 6)
              throw new TypeError(`${apiName}: resolver family must be 4 or 6`);
            return value.address;
          });
        }
        return new APIResponse(await this._native.execute(method, url, nativeOptions));
      } catch (error) {
        if (this._disposed && error?.message?.startsWith('API request context is disposed'))
          throw new Error(`${apiName}: Request context disposed.`);
        throw mapRequestError(error, apiName);
      }
    }
    fetch(url, options = {}) {
      const method = options?.method ?? 'GET';
      if (typeof method !== 'string') throw new TypeError('apiRequestContext.fetch: method must be a string');
      return this._send('apiRequestContext.fetch', method, url, options);
    }
    delete(url, options = {}) { return this._send('apiRequestContext.delete', 'DELETE', url, options); }
    get(url, options = {}) { return this._send('apiRequestContext.get', 'GET', url, options); }
    head(url, options = {}) { return this._send('apiRequestContext.head', 'HEAD', url, options); }
    patch(url, options = {}) { return this._send('apiRequestContext.patch', 'PATCH', url, options); }
    post(url, options = {}) { return this._send('apiRequestContext.post', 'POST', url, options); }
    put(url, options = {}) { return this._send('apiRequestContext.put', 'PUT', url, options); }
    async dispose(options = {}) {
      options = { ...options };
      validateOptions(options, ['reason'], 'apiRequestContext.dispose');
      if (options.reason !== undefined && typeof options.reason !== 'string')
        throw new TypeError('apiRequestContext.dispose: reason must be a string');
      this._disposeFromOwner(options.reason);
    }
    _disposeFromOwner(reason) {
      if (this._disposed) return;
      this._native.dispose(reason);
      this._disposed = true;
      contexts.delete(this);
    }
    async storageState(options = {}) {
      options = { ...options };
      validateOptions(options, ['indexedDB', 'path'], 'apiRequestContext.storageState');
      if (options.indexedDB !== undefined && typeof options.indexedDB !== 'boolean')
        throw new TypeError('apiRequestContext.storageState: indexedDB must be a boolean');
      if (options.path !== undefined && typeof options.path !== 'string')
        throw new TypeError('apiRequestContext.storageState: path must be a string');
      let origins = JSON.parse(this._native.storageOriginsJson);
      if (!options.indexedDB) {
        origins = origins.map(origin => {
          const { indexedDB: _indexedDB, ...withoutIndexedDB } = origin;
          return withoutIndexedDB;
        });
      }
      const state = {
        cookies: (await this._native.storageCookies()).map(cookie => ({ ...cookie })),
        origins,
      };
      if (options.path !== undefined)
        await fs.writeFile(options.path, JSON.stringify(state, undefined, 2));
      return state;
    }
    pwrightMetrics() { return this._native.metrics(); }
  }

  return Object.freeze({
    _contexts: contexts,
    _validateProxy: validateProxy,
    _fromNative(native, baseURL) { return new APIRequestContext(native, baseURL); },
    async newContext(options = {}) {
      options = { ...options };
      validateClientCertificates(options.clientCertificates, 'request.newContext');
      const proxyKind = validateProxy(options.proxy, 'request.newContext');
      const clientCertificates = await normalizeClientCertificates(options.clientCertificates);
      const addon = nativeAddon();
      if (clientCertificates.length
          && (typeof addon.apiRequestClientCertificatesEnabled !== 'function'
            || !addon.apiRequestClientCertificatesEnabled())) {
        throw new UnsupportedOperationError(
          'request.newContext (select api-request-client-certificates native profile)',
        );
      }
      if (proxyKind === 'http'
          && (typeof addon.apiRequestHttpProxyEnabled !== 'function'
            || !addon.apiRequestHttpProxyEnabled())) {
        throw new UnsupportedOperationError(
          'request.newContext (select api-request-http-proxy native profile)',
        );
      }
      if (proxyKind === 'socks'
          && (typeof addon.apiRequestSocks5ProxyEnabled !== 'function'
            || !addon.apiRequestSocks5ProxyEnabled())) {
        throw new UnsupportedOperationError(
          'request.newContext (select api-request-socks5-proxy native profile)',
        );
      }
      if (options.proxy?.bypass !== undefined
          && (typeof addon.apiRequestProxyBypassEnabled !== 'function'
            || !addon.apiRequestProxyBypassEnabled())) {
        throw new UnsupportedOperationError(
          'request.newContext (select api-request-proxy-bypass native profile)',
        );
      }
      const { NativeApiRequestContext } = addon;
      if (typeof NativeApiRequestContext !== 'function')
        throw new UnsupportedOperationError('request.newContext (select api-request native profile)');
      const state = await normalizeStorageState(options.storageState);
      let native;
      try {
        native = new NativeApiRequestContext(contextOptions(
          { ...options, clientCertificates },
          state.origins,
        ));
        if (state.cookies.length) await native.addCookies(state.cookies);
      }
      catch (error) { throw mapRequestError(error, 'request.newContext'); }
      const context = new APIRequestContext(native, options.baseURL);
      contexts.add(context);
      return context;
    },
  });
};
