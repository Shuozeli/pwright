import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import util from 'node:util';

const createApiRequest = createRequire(import.meta.url)('../src/api-request.cjs');

class UnsupportedOperationError extends Error {}

function validateOptions(options, supported, api) {
  if (!options || typeof options !== 'object' || Array.isArray(options))
    throw new TypeError(`${api}: options must be an object`);
  for (const key of Object.keys(options)) {
    if (!supported.includes(key)) throw new UnsupportedOperationError(`${api} option ${key}`);
  }
}

function fixture() {
  const constructions = [];
  const calls = [];
  class NativeApiResponse {
    url = 'http://example.test/result';
    status = 201;
    statusText = 'Created';
    ok = true;
    disposed = false;
    headersArray() {
      return [
        { name: 'Content-Type', value: 'application/json' },
        { name: 'Set-Cookie', value: 'a=1' },
        { name: 'Set-Cookie', value: 'b=2' },
      ];
    }
    body() { return Buffer.from('{"ok":true}'); }
    log() { return ['request', 'response']; }
    dispose() { this.disposed = true; }
  }
  class NativeApiRequestContext {
    constructor(options) {
      this.options = options;
      this.cookies = [];
      this.disposed = false;
      constructions.push(this);
    }
    async execute(method, url, options) {
      calls.push({ method, url, options });
      return new NativeApiResponse();
    }
    async addCookies(cookies) { this.cookies.push(...cookies.map(cookie => ({ ...cookie }))); }
    async storageCookies() { return this.cookies.map(cookie => ({ ...cookie })); }
    get storageOriginsJson() { return this.options.storageOriginsJson; }
    dispose(reason) { this.disposed = true; this.reason = reason; }
    metrics() {
      return { disposed: this.disposed, inFlight: 0, liveResponses: 0, retainedBodyBytes: 0, retainedLogBytes: 0 };
    }
  }
  const request = createApiRequest({
    nativeAddon: () => ({
      NativeApiRequestContext,
      apiRequestClientCertificatesEnabled: () => true,
      apiRequestHttpProxyEnabled: () => true,
      apiRequestSocks5ProxyEnabled: () => true,
      apiRequestProxyBypassEnabled: () => true,
    }),
    validateOptions,
    UnsupportedOperationError,
  });
  return { request, constructions, calls };
}

test('API request adapter delegates all seven verbs and exposes Playwright-shaped responses', async () => {
  // Arrange
  const { request, constructions, calls } = fixture();
  const context = await request.newContext({
    baseURL: 'http://example.test',
    userAgent: 'pwright-test',
    extraHTTPHeaders: { 'x-context': 'one' },
    timeout: 1234,
    acceptDownloads: true,
  });
  const invocations = [
    ['fetch', 'CUSTOM'],
    ['delete', 'DELETE'],
    ['get', 'GET'],
    ['head', 'HEAD'],
    ['patch', 'PATCH'],
    ['post', 'POST'],
    ['put', 'PUT'],
  ];

  // Act
  const responses = [];
  for (const [name, method] of invocations) {
    responses.push(await context[name]('/resource', {
      ...(name === 'fetch' ? { method } : {}),
      data: { name },
      headers: { 'x-method': name },
    }));
  }

  // Assert
  assert.equal(request._contexts.has(context), true);
  assert.equal(constructions.length, 1);
  assert.deepEqual(constructions[0].options, {
    baseUrl: 'http://example.test',
    userAgent: 'pwright-test',
    extraHeaders: [{ name: 'x-context', value: 'one' }],
    timeoutMs: 1234,
    failOnStatusCode: false,
    ignoreHttpsErrors: false,
    maxRedirects: 20,
    httpUsername: undefined,
    httpPassword: undefined,
    httpOrigin: undefined,
    httpSendAlways: false,
    storageOriginsJson: '[]',
    clientCertificates: [],
    proxyServer: undefined,
    proxyUsername: undefined,
    proxyPassword: undefined,
    proxyBypass: undefined,
  });
  assert.deepEqual(calls.map(call => call.method), invocations.map(([, method]) => method));
  for (const call of calls) {
    assert.equal(call.url, '/resource');
    assert.equal(call.options.headers.at(-1).name, 'content-type');
    assert.equal(call.options.body.toString(), JSON.stringify({ name: call.options.headers[0].value }));
  }
  assert.equal(responses[0].url(), 'http://example.test/result');
  assert.equal(responses[0].status(), 201);
  assert.equal(responses[0].statusText(), 'Created');
  assert.equal(responses[0].ok(), true);
  assert.deepEqual(responses[0].headers(), {
    'content-type': 'application/json',
    'set-cookie': 'a=1, b=2',
  });
  assert.equal(await responses[0].text(), '{"ok":true}');
  assert.deepEqual(await responses[0].json(), { ok: true });
  assert.deepEqual(await responses[0].log(), ['request', 'response']);
  assert.deepEqual(await responses[0]._fetchLog(), ['request', 'response']);
  assert.match(responses[0][util.inspect.custom](), /^APIResponse: 201 Created\n/);
  await responses[0].dispose();
  await context.dispose({ reason: 'done' });
  assert.equal(constructions[0].reason, 'done');
  assert.equal(request._contexts.size, 0);
});

test('API request default User-Agent is derived without launching a helper process', async () => {
  // Arrange
  const { request, constructions } = fixture();

  // Act
  const context = await request.newContext();

  // Assert
  assert.match(
    constructions[0].options.userAgent,
    /^Playwright\/1\.58\.0 \([^;]+; (?:macOS|windows|linux|unknown) [^)]+\) node\/\d+\.\d+(?: CI\/1)?$/,
  );
  await context.dispose();
});

test('API request adapter maps native timeout and rejects unsupported options', async () => {
  // Arrange
  class NativeApiRequestContext {
    async execute() { throw new Error('API request timed out after 25ms'); }
    dispose() {}
    metrics() { return {}; }
  }
  const request = createApiRequest({
    nativeAddon: () => ({ NativeApiRequestContext }),
    validateOptions,
    UnsupportedOperationError,
  });
  const context = await request.newContext({ userAgent: 'pwright-test' });

  // Act
  const timedOut = context.get('http://example.test');

  // Assert
  await assert.rejects(timedOut, /apiRequestContext\.get: Timeout 25ms exceeded/);
  await assert.rejects(
    context.get('http://example.test', { form: { value: 'unsupported' } }),
    UnsupportedOperationError,
  );
  await assert.rejects(
    context.fetch('http://example.test', { maxRedirects: -1 }),
    /'maxRedirects' must be greater than or equal to '0'/,
  );
});

test('API request adapter validates client certificate shape before capability selection', async () => {
  // Arrange
  let capabilitySelections = 0;
  const request = createApiRequest({
    nativeAddon() {
      capabilitySelections += 1;
      return {};
    },
    validateOptions,
    UnsupportedOperationError,
  });

  // Act
  const missingMaterial = request.newContext({
    clientCertificates: [{ origin: 'test' }],
  });
  const mixedPfxAndPem = request.newContext({
    clientCertificates: [{
      origin: 'test',
      certPath: 'cert.pem',
      keyPath: 'key.pem',
      pfxPath: 'client.pfx',
      passphrase: 'secret',
    }],
  });

  // Assert
  await assert.rejects(missingMaterial, /None of cert, key, passphrase or pfx is specified/);
  await assert.rejects(mixedPfxAndPem, /pfx is specified together with cert, key or passphrase/);
  assert.equal(capabilitySelections, 0);
});

test('API request adapter forwards a PFX passphrase to the optional native capability', async () => {
  // Arrange
  const { request, constructions } = fixture();
  const pfx = Buffer.from([1, 2, 3]);

  // Act
  const context = await request.newContext({
    clientCertificates: [{
      origin: 'https://example.test',
      pfx,
      passphrase: 'secure',
    }],
  });

  // Assert
  assert.deepEqual(constructions[0].options.clientCertificates, [{
    origin: 'https://example.test',
    cert: undefined,
    key: undefined,
    pfx,
    passphrase: 'secure',
  }]);
  await context.dispose();
});

test('API request adapter composes client certificates with HTTP proxy selection', async () => {
  // Arrange
  const { request, constructions } = fixture();
  const cert = Buffer.from([1, 2]);
  const key = Buffer.from([3, 4]);

  // Act
  const context = await request.newContext({
    clientCertificates: [{ origin: 'https://example.test', cert, key }],
    proxy: { server: 'proxy.example.test:3128' },
  });

  // Assert
  assert.equal(constructions[0].options.proxyServer, 'proxy.example.test:3128');
  assert.deepEqual(constructions[0].options.clientCertificates, [{
    origin: 'https://example.test', cert, key, pfx: undefined, passphrase: undefined,
  }]);
  await context.dispose();
});

test('API request adapter composes client certificates with SOCKS5 selection', async () => {
  // Arrange
  const { request, constructions } = fixture();
  const pfx = Buffer.from([1, 2, 3]);

  // Act
  const context = await request.newContext({
    clientCertificates: [{ origin: 'https://example.test', pfx, passphrase: 'secure' }],
    proxy: { server: 'socks5://proxy.example.test:1080' },
  });

  // Assert
  assert.equal(constructions[0].options.proxyServer, 'socks5://proxy.example.test:1080');
  assert.deepEqual(constructions[0].options.clientCertificates, [{
    origin: 'https://example.test', cert: undefined, key: undefined, pfx, passphrase: 'secure',
  }]);
  await context.dispose();
});

test('API request adapter forwards bounded proxy selection to its optional native capability', async () => {
  // Arrange
  const { request, constructions } = fixture();

  // Act
  const context = await request.newContext({
    proxy: {
      server: 'proxy.example.test:3128',
      username: 'user',
      password: 'secret',
    },
  });

  // Assert
  assert.equal(constructions[0].options.proxyServer, 'proxy.example.test:3128');
  assert.equal(constructions[0].options.proxyUsername, 'user');
  assert.equal(constructions[0].options.proxyPassword, 'secret');
  assert.equal(constructions[0].options.proxyBypass, undefined);
  await context.dispose();
});

test('API request adapter forwards proxy bypass to its independent native capability', async () => {
  // Arrange
  const { request, constructions } = fixture();

  // Act
  const context = await request.newContext({
    proxy: {
      server: 'proxy.example.test:3128',
      bypass: 'one.example.test, .another.test',
    },
  });

  // Assert
  assert.equal(constructions[0].options.proxyServer, 'proxy.example.test:3128');
  assert.equal(constructions[0].options.proxyBypass, 'one.example.test, .another.test');
  await context.dispose();
});

test('API request adapter rejects proxy bypass when its native leaf is absent', async () => {
  // Arrange
  let constructions = 0;
  class NativeApiRequestContext {
    constructor() { constructions += 1; }
  }
  const request = createApiRequest({
    nativeAddon: () => ({
      NativeApiRequestContext,
      apiRequestHttpProxyEnabled: () => true,
      apiRequestProxyBypassEnabled: () => false,
    }),
    validateOptions,
    UnsupportedOperationError,
  });

  // Act
  const context = request.newContext({
    proxy: { server: 'proxy.example.test:3128', bypass: 'example.test' },
  });

  // Assert
  await assert.rejects(context, /select api-request-proxy-bypass native profile/);
  assert.equal(constructions, 0);
});

test('API request adapter rejects proxy selection when its native leaf is absent', async () => {
  // Arrange
  let constructions = 0;
  class NativeApiRequestContext {
    constructor() { constructions += 1; }
  }
  const request = createApiRequest({
    nativeAddon: () => ({
      NativeApiRequestContext,
      apiRequestHttpProxyEnabled: () => false,
    }),
    validateOptions,
    UnsupportedOperationError,
  });

  // Act
  const context = request.newContext({ proxy: { server: 'proxy.example.test:3128' } });

  // Assert
  await assert.rejects(context, /select api-request-http-proxy native profile/);
  assert.equal(constructions, 0);
});

test('API request adapter forwards SOCKS5 selection only to its optional native capability', async () => {
  // Arrange
  const { request, constructions } = fixture();

  // Act
  const context = await request.newContext({
    proxy: { server: 'socks5://proxy.example.test:1080' },
  });

  // Assert
  assert.equal(constructions[0].options.proxyServer, 'socks5://proxy.example.test:1080');
  assert.equal(constructions[0].options.proxyUsername, undefined);
  assert.equal(constructions[0].options.proxyPassword, undefined);
  await context.dispose();
});

test('API request adapter rejects SOCKS5 selection when its native leaf is absent', async () => {
  // Arrange
  let constructions = 0;
  class NativeApiRequestContext {
    constructor() { constructions += 1; }
  }
  const request = createApiRequest({
    nativeAddon: () => ({
      NativeApiRequestContext,
      apiRequestHttpProxyEnabled: () => true,
      apiRequestSocks5ProxyEnabled: () => false,
    }),
    validateOptions,
    UnsupportedOperationError,
  });

  // Act
  const context = request.newContext({
    proxy: { server: 'socks5://proxy.example.test:1080' },
  });

  // Assert
  await assert.rejects(context, /select api-request-socks5-proxy native profile/);
  assert.equal(constructions, 0);
});

test('API request adapter preserves serialized JSON strings and stringifies plain strings', async () => {
  // Arrange
  const { request, calls } = fixture();
  const context = await request.newContext({ userAgent: 'pwright-test' });
  const headers = { 'content-type': 'application/json' };

  // Act
  await context.post('http://example.test', { headers, data: '{"ready":true}' });
  await context.post('http://example.test', { headers, data: 'plain' });

  // Assert
  assert.equal(calls[0].options.body.toString(), '{"ready":true}');
  assert.equal(calls[1].options.body.toString(), '"plain"');
});

test('API request adapter distinguishes in-flight disposal from later calls', async () => {
  // Arrange
  let rejectExecution;
  class NativeApiRequestContext {
    execute() { return new Promise((_resolve, reject) => { rejectExecution = reject; }); }
    dispose() {}
    metrics() { return {}; }
  }
  const request = createApiRequest({
    nativeAddon: () => ({ NativeApiRequestContext }),
    validateOptions,
    UnsupportedOperationError,
  });
  const context = await request.newContext({ userAgent: 'pwright-test' });
  const pending = context.get('http://example.test');

  // Act
  await context.dispose();
  rejectExecution(new Error('API request context is disposed'));

  // Assert
  await assert.rejects(pending, /apiRequestContext\.get: Request context disposed\./);
  await assert.rejects(
    context.get('http://example.test'),
    /apiRequestContext\.get: Target page, context or browser has been closed/,
  );
});

test('API request adapter forwards test resolver addresses without rewriting the URL host', async () => {
  // Arrange
  const { request, calls } = fixture();
  const context = await request.newContext({ baseURL: 'http://base.example.test' });
  let resolvedHostname;

  // Act
  await context.get('http://www.one.com/resource', {
    __testHookLookup(hostname) {
      resolvedHostname = hostname;
      return [{ address: '127.0.0.1', family: 4 }];
    },
  });

  // Assert
  assert.equal(resolvedHostname, 'www.one.com');
  assert.equal(calls[0].url, 'http://www.one.com/resource');
  assert.deepEqual(calls[0].options.lookupAddresses, ['127.0.0.1']);
});

test('API request adapter imports and exports native-backed storage state', async () => {
  // Arrange
  const { request, constructions } = fixture();
  const storageState = {
    cookies: [{
      name: 'session', value: 'one', domain: '.example.test', path: '/', expires: -1,
      httpOnly: true, secure: true, sameSite: 'Lax',
    }],
    origins: [{
      origin: 'https://example.test',
      localStorage: [{ name: 'key', value: 'value' }],
      indexedDB: [{ name: 'db', version: 1, stores: [] }],
    }],
  };
  const context = await request.newContext({ storageState });

  // Act
  const exported = await context.storageState({ indexedDB: true });

  // Assert
  assert.deepEqual(exported, storageState);
  assert.deepEqual(constructions[0].cookies, storageState.cookies);
});

test('API request adapter round-trips storage state through a file', async () => {
  // Arrange
  const { request } = fixture();
  const directory = await mkdtemp(join(tmpdir(), 'pwright-api-request-'));
  const path = join(directory, 'state.json');
  const storageState = {
    cookies: [{
      name: 'session', value: 'one', domain: 'example.test', path: '/', expires: -1,
      httpOnly: false, secure: false, sameSite: 'Lax',
    }],
    origins: [],
  };

  try {
    const first = await request.newContext({ storageState });

    // Act
    const exported = await first.storageState({ path });
    const second = await request.newContext({ storageState: path });

    // Assert
    assert.deepEqual(JSON.parse(await readFile(path, 'utf8')), exported);
    assert.deepEqual(await second.storageState(), storageState);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
