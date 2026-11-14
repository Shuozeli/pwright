'use strict';
const { types: { isRegExp } } = require('node:util');

module.exports = function pageRoutes({ mapError, validateOptions, UnsupportedOperationError, createResponseFromNotice, canonicalizeRequest }) {
  const MAX_PENDING_ROUTE_MUTATIONS = 256;
  const MAX_REGEXP_SOURCE_BYTES = 4096;
  const SUPPORTED_REGEXP_FLAGS = /^[imsu]*$/;

  function routeMatcher(url, session, api) {
    if (typeof url === 'string') return { kind: 'string', url };
    if (!isRegExp(url))
      throw new UnsupportedOperationError(`${api} non-string URL matcher`);
    if (typeof session.pageRouteRegExp !== 'function')
      throw new UnsupportedOperationError(`${api} RegExp URL matcher (select route-regexp native profile)`);
    const source = url.source;
    const flags = url.flags;
    if (Buffer.byteLength(source) > MAX_REGEXP_SOURCE_BYTES)
      throw new TypeError(`${api}: RegExp source exceeds ${MAX_REGEXP_SOURCE_BYTES} UTF-8 bytes`);
    if (!SUPPORTED_REGEXP_FLAGS.test(flags))
      throw new UnsupportedOperationError(`${api} RegExp flags ${flags} (route-regexp supports only i, m, s, and u)`);
    return { kind: 'regexp', url, source, flags };
  }

  function sameRouteMatcher(registration, matcher) {
    if (matcher.kind === 'string')
      return typeof registration.url === 'string' && registration.url === matcher.url;
    return registration.regexpSource === matcher.source && registration.regexpFlags === matcher.flags;
  }

  function registerRoute(session, page, matcher, times) {
    return matcher.kind === 'string'
      ? session.pageRoute(page._id, matcher.url, times)
      : session.pageRouteRegExp(page._id, matcher.source, matcher.flags, times);
  }

  function enqueueRouteMutation(page, api, mutation) {
    const pending = page._pendingRouteMutations ?? 0;
    if (pending >= MAX_PENDING_ROUTE_MUTATIONS)
      throw new Error(`${api}: pending route mutations exceed ${MAX_PENDING_ROUTE_MUTATIONS}`);
    page._pendingRouteMutations = pending + 1;
    const previous = page._routeMutationTail ?? Promise.resolve();
    const result = previous.then(mutation);
    page._routeMutationTail = result.then(() => undefined, () => undefined);
    return result.finally(() => { page._pendingRouteMutations -= 1; });
  }

  class Request {
    constructor(page, call, historyOwner = null) {
      this._page = page;
      this._call = call;
      this._frame = this._captureFrame(call.frameId);
      this._historyOwner = historyOwner;
      this._response = null;
      this._responsePromise = null;
      this._failureText = null;
      this._redirectedFrom = null;
      this._redirectedTo = null;
      this._headers = Object.fromEntries(
        call.headers.map(({ name, value }) => [name.toLowerCase(), value]),
      );
    }

    _adoptNetworkNotice(call, historyOwner) {
      // Network.requestWillBeSent may omit a body already captured by Fetch
      // for this hop. Keep that body when a later notice updates its metadata.
      this._call = typeof call.id === 'string' && call.id === this._call.id
        && call.postData == null && this._call.postData != null
        ? { ...call, postData: this._call.postData }
        : call;
      if (this._frame == null) this._frame = this._captureFrame(this._call.frameId);
      if (historyOwner != null) this._historyOwner = historyOwner;
      this._headers = Object.fromEntries(
        call.headers.map(({ name, value }) => [name.toLowerCase(), value]),
      );
    }

    _setResponse(response) { this._response = response; }
    _setFailure(failureText) { this._failureText = failureText; }
    _linkRedirectedFrom(previous) {
      this._redirectedFrom = previous;
      previous._redirectedTo = this;
    }

    url() { return this._call.url; }
    method() { return this._call.method; }
    resourceType() { return this._call.resourceType; }
    headers() { return { ...this._headers }; }
    allHeaders() { return Promise.resolve({ ...this._headers }); }
    headersArray() { return this._call.headers.map(({ name, value }) => ({ name, value })); }
    postDataBuffer() { return this._call.postData ?? null; }
    postData() { return this._call.postData?.toString('utf8') ?? null; }
    postDataJSON() {
      if (this._call.postDataJsonAvailable !== true)
        throw new UnsupportedOperationError('request.postDataJSON (select request-post-data-json native profile)');
      if (this._call.postDataJsonError !== null && this._call.postDataJsonError !== undefined)
        throw new Error(this._call.postDataJsonError);
      return this._call.postDataJson ?? null;
    }
    isNavigationRequest() { return this._call.isNavigationRequest; }
    response() {
      if (this._response) return Promise.resolve(this._response);
      if (this._responsePromise) return this._responsePromise;
      const session = this._page._context._browser._session;
      if (this._historyOwner == null || typeof session.requestResponse !== 'function'
          || typeof createResponseFromNotice !== 'function')
        return Promise.reject(new UnsupportedOperationError('request.response (select request-lifecycle native profile)'));
      this._responsePromise = Promise.resolve().then(() => session.requestResponse(this._historyOwner))
        .then(notice => notice == null ? null : createResponseFromNotice(this._page, notice, this))
        .catch(error => { throw mapError(error, 'request.response'); });
      return this._responsePromise;
    }
    failure() {
      const session = this._page._context._browser._session;
      let failureText = this._failureText;
      if (this._historyOwner != null && typeof session.requestFailure === 'function') {
        try { failureText = session.requestFailure(this._historyOwner); }
        catch (error) { throw mapError(error, 'request.failure'); }
      }
      return failureText == null ? null : { errorText: failureText };
    }
    redirectedFrom() { return this._redirectedFrom; }
    redirectedTo() { return this._redirectedTo; }
    _captureFrame(frameId) {
      // Capturing an existing wrapper must not enumerate native Frames or
      // require a capability that this Page did not select.
      if (frameId == null) return null;
      return this._page._frameCache?.knownFrame?.(frameId) ?? null;
    }
    frame() {
      if (this._frame != null) return this._frame;
      if (this._call.frameId == null) return null;
      // A network notice may precede its Frame publication. Resolve it later
      // if it was not known when this Request wrapper was created.
      this._frame = this._page.frames().find(frame => frame._id === this._call.frameId) ?? null;
      return this._frame;
    }
  }

  class Route {
    constructor(page, call) {
      this._page = page;
      this._callId = call.callId;
      // The opt-in native history owner carries the network event's exact hop
      // identity. Share that Request with the event stream so a redirect that
      // is not re-intercepted still links back to the routed Request object.
      this._request = call.historyOwner != null && typeof call.requestId === 'string'
        && typeof canonicalizeRequest === 'function'
        ? canonicalizeRequest(page, {
          ...call,
          id: call.requestId,
          redirectedFrom: call.redirectedFrom ?? null,
        }, call.historyOwner)
        : new Request(page, call, call.historyOwner ?? null);
      this._handled = false;
    }

    request() { return this._request; }

    async continue(options = {}) {
      validateOptions(options, ['headers'], 'route.continue');
      const session = this._page._context._browser._session;
      let headers;
      if (options.headers !== undefined) {
        if (options.headers === null || typeof options.headers !== 'object' || Array.isArray(options.headers))
          throw new TypeError('route.continue: headers must be an object');
        if (typeof session.pageRouteContinueWithHeaders !== 'function')
          throw new UnsupportedOperationError('route.continue headers (select route-header-url-overrides native profile)');
        const entries = Object.entries(options.headers);
        if (entries.length > 256)
          throw new TypeError('route.continue: headers exceed count limit (256)');
        let bytes = 0;
        headers = entries.map(([name, value]) => {
          if (typeof value !== 'string')
            throw new TypeError(`route.continue: header ${name} must be a string`);
          bytes += Buffer.byteLength(name) + Buffer.byteLength(value);
          if (bytes > 64 * 1024)
            throw new TypeError('route.continue: headers exceed byte limit (65536)');
          return { name, value };
        });
      }
      this._claim('route.continue');
      try {
        if (headers === undefined) await session.pageRouteContinue(this._callId);
        else await session.pageRouteContinueWithHeaders(this._callId, headers);
      } catch (error) {
        this._handled = false;
        throw mapError(error, 'route.continue');
      }
    }

    async fallback(options = {}) {
      validateOptions(options, ['url'], 'route.fallback');
      const session = this._page._context._browser._session;
      if (options.url !== undefined) {
        if (typeof options.url !== 'string')
          throw new TypeError('route.fallback: url must be a string');
        if (typeof session.pageRouteFallbackWithUrl !== 'function')
          throw new UnsupportedOperationError('route.fallback URL (select route-header-url-overrides native profile)');
      }
      this._claim('route.fallback');
      try {
        if (options.url === undefined) await session.pageRouteFallback(this._callId);
        else await session.pageRouteFallbackWithUrl(this._callId, options.url);
      } catch (error) {
        this._handled = false;
        throw mapError(error, 'route.fallback');
      }
    }

    async abort(errorCode) {
      if (errorCode !== undefined && typeof errorCode !== 'string')
        throw new TypeError('route.abort: errorCode must be a string');
      this._claim('route.abort');
      try {
        await this._page._context._browser._session.pageRouteAbort(this._callId, errorCode);
      } catch (error) {
        this._handled = false;
        throw mapError(error, 'route.abort');
      }
    }

    async fulfill(options = {}) {
      const api = 'route.fulfill';
      validateOptions(options, ['status', 'headers', 'body', 'json', 'path', 'contentType', 'response'], api);
      if (options.path !== undefined)
        throw new UnsupportedOperationError(`${api} path (select route-fulfill-path native profile)`);
      if (options.response !== undefined)
        throw new UnsupportedOperationError(`${api} response (select route-fulfill-api-response native profile)`);
      if (options.body !== undefined && options.json !== undefined)
        throw new Error(`${api}: json and body are mutually exclusive`);
      const status = options.status ?? 200;
      if (!Number.isInteger(status) || status < 100 || status > 999)
        throw new TypeError(`${api}: status must be an integer in [100, 999]`);
      if (options.contentType !== undefined && typeof options.contentType !== 'string')
        throw new TypeError(`${api}: contentType must be a string`);
      if (options.headers !== undefined &&
          (options.headers === null || typeof options.headers !== 'object' || Array.isArray(options.headers)))
        throw new TypeError(`${api}: headers must be an object`);
      let headers = Object.entries(options.headers ?? {}).map(([name, value]) => {
        if (typeof value !== 'string') throw new TypeError(`${api}: header ${name} must be a string`);
        return { name, value };
      });
      let body = options.body;
      if (options.json !== undefined) {
        const encoded = JSON.stringify(options.json);
        if (encoded === undefined) throw new TypeError(`${api}: json must be serializable`);
        body = encoded;
      }
      let contentType = options.contentType;
      if (contentType === undefined && options.json !== undefined) contentType = 'application/json';
      if (contentType !== undefined) {
        headers = headers.filter(header => header.name.toLowerCase() !== 'content-type');
        headers.push({ name: 'content-type', value: contentType });
      }
      if (body === undefined) body = Buffer.alloc(0);
      else if (typeof body === 'string') body = Buffer.from(body);
      else if (!Buffer.isBuffer(body)) throw new TypeError(`${api}: body must be a string or Buffer`);
      this._claim(api);
      try {
        await this._page._context._browser._session.pageRouteFulfill(
          this._callId, status, headers, body,
        );
      } catch (error) {
        this._handled = false;
        throw mapError(error, api);
      }
    }
    async fetch() { throw new UnsupportedOperationError('route.fetch'); }

    _claim(api) {
      if (this._handled) throw new Error(`${api}: Route is already handled!`);
      this._handled = true;
    }
  }

  async function route(page, url, handler, options = {}) {
    const api = 'page.route';
    validateOptions(options, ['times'], api);
    const session = page._context._browser._session;
    const matcher = routeMatcher(url, session, api);
    if (typeof handler !== 'function') throw new TypeError(`${api}: handler must be a function`);
    const times = options.times;
    if (times !== undefined && (!Number.isInteger(times) || times < 1 || times > 0xffff_ffff))
      throw new TypeError(`${api}: times must be an integer in [1, 4294967295]`);
    if (typeof session.pageRoute !== 'function')
      throw new UnsupportedOperationError(`${api} (select page-routes native profile)`);
    if (typeof session.pageUnroute !== 'function') {
      let registration;
      try { registration = await registerRoute(session, page, matcher, times); }
      catch (error) { throw mapError(error, api); }
      page._routes.set(String(registration), handler);
      return;
    }
    return enqueueRouteMutation(page, api, async () => {
      let registration;
      try { registration = await registerRoute(session, page, matcher, times); }
      catch (error) { throw mapError(error, api); }
      const id = String(registration);
      page._routes.set(id, handler);
      page._routeRegistrations ??= new Map();
      page._routeRegistrations.set(id, matcher.kind === 'string'
        ? { url, handler }
        : { url, handler, regexpSource: matcher.source, regexpFlags: matcher.flags });
    });
  }

  async function unroute(page, url, handler) {
    const api = 'page.unroute';
    const session = page._context._browser._session;
    const matcher = routeMatcher(url, session, api);
    if (matcher.kind === 'string' && Buffer.byteLength(url) > 4 * 1024)
      throw new TypeError(`${api}: URL matcher exceeds 4096 UTF-8 bytes`);
    if (handler !== undefined && typeof handler !== 'function')
      throw new TypeError(`${api}: handler must be a function`);
    if (typeof session.pageUnroute !== 'function')
      throw new UnsupportedOperationError(`${api} (select page-unroute native profile)`);
    return enqueueRouteMutation(page, api, async () => {
      const registrations = page._routeRegistrations;
      if (!registrations) return;
      const ids = [...registrations]
        .filter(([, registration]) => sameRouteMatcher(registration, matcher)
          && (handler === undefined || registration.handler === handler))
        .map(([id]) => id);
      if (ids.length === 0) return;
      try { await session.pageUnroute(page._id, ids); }
      catch (error) { throw mapError(error, api); }
      for (const id of ids) {
        registrations.delete(id);
        page._routes.delete(id);
      }
    });
  }

  async function dispatch(browser, call) {
    const context = browser._contexts.get(call.contextId);
    const page = context?._pages.get(call.pageId);
    const handler = page?._routes.get(call.registrationId);
    if (!page || !handler) {
      await browser._session.pageRouteFallback(call.callId);
      return;
    }
    if (call.expires) {
      page._routes.delete(call.registrationId);
      page._routeRegistrations?.delete(call.registrationId);
    }
    const intercepted = new Route(page, call);
    try {
      await handler(intercepted, intercepted.request());
    } catch (error) {
      if (!intercepted._handled) {
        try { await intercepted.abort('failed'); } catch {}
      }
      throw error;
    }
  }

  function watch(session, browserRef) {
    if (typeof session.nextPageRouteCall !== 'function') return;
    const next = () => {
      Promise.resolve().then(() => session.nextPageRouteCall()).then(call => {
        if (!call) return;
        const browser = browserRef.deref();
        if (!browser) {
          Promise.resolve(session.pageRouteFallback(call.callId)).catch(() => {});
          return;
        }
        dispatch(browser, call).then(next, error => browser._didWatchFail(error));
      }, error => browserRef.deref()?._didWatchFail(error));
    };
    next();
  }

  return { Request, Route, dispatch, route, unroute, watch };
};
