'use strict';

// Rust owns request observation, bounded queues, lifecycle reconciliation and
// URL-glob compilation. This adapter only publishes immutable snapshots and
// installs Playwright-shaped language matchers on the Page EventEmitter.
module.exports = function requestEvents({
  Request,
  mapError,
  validateOptions,
  UnsupportedOperationError,
  waitForEvent,
}) {
  const isRegExp = value => value instanceof RegExp
    || Object.prototype.toString.call(value) === '[object RegExp]';

  const trimWithEllipsis = value => {
    const characters = [...value];
    return characters.length > 50
      ? `${characters.slice(0, 49).join('')}…`
      : characters.join('');
  };

  function matcherLog(expected) {
    if (typeof expected === 'string') return `waiting for request "${trimWithEllipsis(expected)}"`;
    if (isRegExp(expected))
      return `waiting for request /${trimWithEllipsis(expected.source)}/${expected.flags}`;
    return undefined;
  }

  function prepareMatcher(page, expected, api) {
    if (typeof expected === 'string') {
      const session = page._context._browser._session;
      if (typeof session.pageUrlGlob !== 'function')
        throw new UnsupportedOperationError(`${api} (select request-events native profile)`);
      let pattern;
      try { pattern = session.pageUrlGlob(page._id, expected); }
      catch (error) { throw mapError(error, api); }
      const regex = new RegExp(pattern);
      return request => regex.test(request.url());
    }
    if (isRegExp(expected)) return request => expected.test(request.url());
    if (typeof expected === 'function') return expected;
    throw new TypeError(`${api}: urlOrPredicate must be a string, RegExp, or function`);
  }

  function wait(page, expected, options = {}, apiStack) {
    const api = 'page.waitForRequest';
    try {
      validateOptions(options, ['timeout'], api);
      const session = page._context._browser._session;
      if (typeof session.nextRequestNotice !== 'function')
        throw new UnsupportedOperationError(`${api} (select request-events native profile)`);
      const timeout = options.timeout ?? (typeof session.pageDefaultTimeout === 'function'
        ? session.pageDefaultTimeout(page._id, false)
        : 30_000);
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      const predicate = prepareMatcher(page, expected, api);
      // waitForEvent attaches the listener before returning its Promise. No
      // asynchronous native arm step may race the user's Promise.all action.
      return waitForEvent(
        page,
        'request',
        { predicate, timeout },
        api,
        timeout,
        { apiStack, logLine: matcherLog(expected) },
      );
    } catch (error) {
      return Promise.reject(error);
    }
  }

  function knownRequest(page, id) {
    return page._networkRequests?.get(id) ?? page._networkRequestRefs?.get(id)?.deref() ?? null;
  }

  function rememberRequest(page, id, request) {
    page._networkRequestRefs ??= new Map();
    page._networkRequestFinalizer ??= new FinalizationRegistry(key => {
      if (!page._networkRequestRefs.get(key)?.deref()) page._networkRequestRefs.delete(key);
    });
    page._networkRequestRefs.set(id, new WeakRef(request));
    page._networkRequestFinalizer.register(request, id);
  }

  class NetworkRequest extends Request {
    redirectedFrom() {
      if (this._redirectedFrom) return this._redirectedFrom;
      if (typeof this._call.redirectedFrom !== 'string' || this._historyOwner == null)
        return null;
      const session = this._page._context._browser._session;
      if (typeof session.requestRedirectedFrom !== 'function') return null;
      let predecessor;
      try { predecessor = session.requestRedirectedFrom(this._historyOwner); }
      catch (error) { throw mapError(error, 'request.redirectedFrom'); }
      if (predecessor == null) return null;
      const previous = canonicalize(this._page, predecessor.request, predecessor.historyOwner);
      if (previous !== this) this._linkRedirectedFrom(previous);
      return this._redirectedFrom;
    }
  }

  function canonicalize(page, metadata, historyOwner = null) {
    const id = metadata.id;
    if (typeof id !== 'string') return new NetworkRequest(page, metadata, historyOwner);
    let request = knownRequest(page, id);
    if (request) request._adoptNetworkNotice(metadata, historyOwner);
    else {
      request = new NetworkRequest(page, metadata, historyOwner);
      rememberRequest(page, id, request);
    }
    const predecessorId = metadata.redirectedFrom;
    if (typeof predecessorId === 'string' && predecessorId !== id) {
      const predecessor = knownRequest(page, predecessorId);
      if (predecessor && request._redirectedFrom !== predecessor)
        request._linkRedirectedFrom(predecessor);
    }
    return request;
  }

  function dispatch(browser, notice) {
    const page = [...browser._contexts.values()]
      .map(context => context._pages.get(notice.pageId))
      .find(Boolean);
    if (!page) return;
    const id = notice.request.id;
    const request = canonicalize(page, notice.request, notice.historyOwner);
    if (!['request', 'requestfinished', 'requestfailed'].includes(notice.kind))
      throw new Error(`Unsupported native request notice kind: ${notice.kind}`);
    if (notice.kind === 'request') page._networkRequests.set(id, request);
    if (notice.kind === 'requestfailed') request._setFailure(notice.failureText ?? null);
    try {
      page._context.emit?.(notice.kind, request);
      page.emit(notice.kind, request);
    } finally {
      if (notice.kind !== 'request') page._networkRequests.delete(id);
    }
  }

  function watchCombined(session, browserRef, dispatchResponse) {
    if (typeof session.nextNetworkNotice !== 'function') return false;
    const next = () => {
      Promise.resolve().then(() => session.nextNetworkNotice()).then(value => {
        if (!value) return;
        const browser = browserRef.deref();
        if (!browser) return;
        if (value.kind === 'request' && value.requestNotice)
          dispatch(browser, value.requestNotice);
        else if (value.kind === 'response' && value.responseNotice)
          dispatchResponse(browser, value.responseNotice);
        else
          throw new Error(`Unsupported native network notice kind: ${value.kind}`);
        next();
      }).catch(error => {
        const browser = browserRef.deref();
        if (browser?.isConnected()) browser._didWatchFail(error);
      });
    };
    next();
    return true;
  }

  function watch(session, browserRef) {
    if (typeof session.nextRequestNotice !== 'function') return;
    const next = () => {
      Promise.resolve().then(() => session.nextRequestNotice()).then(notice => {
        if (!notice) return;
        const browser = browserRef.deref();
        if (!browser) return;
        dispatch(browser, notice);
        next();
      }).catch(error => {
        const browser = browserRef.deref();
        if (browser?.isConnected()) browser._didWatchFail(error);
      });
    };
    next();
  }

  return { canonicalize, dispatch, prepareMatcher, wait, watch, watchCombined };
};
