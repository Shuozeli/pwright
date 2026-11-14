'use strict';

const { types: { isRegExp } } = require('node:util');

// Only the opt-in Playwright Server Page-notice profile loads this adapter.
// Native owns wire validation, metadata and queue bounds; JS owns language
// predicates, waiter lifetimes and EventEmitter delivery.
module.exports = ({ TimeoutError, UnsupportedOperationError, mapError }) => {
  const TARGET_CLOSED = 'Target page, context or browser has been closed';
  const MAX_WAITERS = 32;
  const MAX_REQUEST_REFS = 4096;
  const EVENTS = new Set(['request', 'response', 'download', 'close', 'crash']);

  function objectKey(value) {
    if (!value || typeof value.guid !== 'string' || !value.guid
        || typeof value.generation !== 'string' || !/^[1-9]\d*$/.test(value.generation))
      throw new Error('Invalid native Page notice object identity');
    return JSON.stringify([value.guid, value.generation]);
  }

  function nativeIdentity(native) {
    if (typeof native.createdSequence !== 'function')
      throw new Error('Selected Page-notice addon lacks a creation-sequence fence');
    const value = { guid: native.guid(), generation: native.createdSequence() };
    objectKey(value);
    return value;
  }

  function targetClosed(api) {
    const error = new Error(`${api}: ${TARGET_CLOSED}`);
    error.name = 'TargetClosedError';
    error.code = 'PWRIGHT_TARGET_CLOSED';
    return error;
  }

  function checkTimeout(options, api) {
    if (!options || typeof options !== 'object' || Array.isArray(options))
      throw new TypeError(`${api}: options must be an object`);
    for (const name of Object.keys(options)) {
      if (name !== 'timeout' && name !== 'predicate')
        throw new UnsupportedOperationError(`${api} option ${name}`);
    }
    const timeout = options.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    return timeout;
  }

  function register(page, event, predicate, timeout, api) {
    if (page._terminalClosed || !page._native.isOpen()) return Promise.reject(targetClosed(api));
    page._remoteWaiters ??= new Set();
    if (page._remoteWaiters.size >= MAX_WAITERS)
      return Promise.reject(new Error(`${api}: waiter capacity exceeds ${MAX_WAITERS}`));
    return new Promise((resolve, reject) => {
      const waiter = {
        api, event, predicate, settled: false, timer: null,
        finish(error, value) {
          if (this.settled) return;
          this.settled = true;
          if (this.timer) clearTimeout(this.timer);
          page._remoteWaiters.delete(this);
          if (error) reject(error);
          else resolve(value);
        },
      };
      page._remoteWaiters.add(waiter);
      if (timeout) waiter.timer = setTimeout(() => {
        waiter.finish(new TimeoutError(`${api}: Timeout ${timeout}ms exceeded while waiting for event "${event}"`));
      }, timeout);
    });
  }

  function onEvent(page, event, value) {
    for (const waiter of [...(page._remoteWaiters ?? [])]) {
      if (waiter.event !== event) continue;
      try {
        const accepted = waiter.predicate ? waiter.predicate(value) : true;
        // Settle synchronous matches before EventEmitter listeners can close
        // the Page. A pending async predicate still loses to a terminal notice.
        if (accepted && typeof accepted.then === 'function') {
          Promise.resolve(accepted).then(
            matches => { if (matches) waiter.finish(null, value); },
            error => waiter.finish(error),
          );
        } else if (accepted) waiter.finish(null, value);
      } catch (error) { waiter.finish(error); }
    }
  }

  function onPageClosed(page) {
    // Pinned Playwright Waiter applies the predicate to the terminal event
    // itself. A rejected match stays pending (and may later time out).
    onEvent(page, 'close', page);
    for (const waiter of [...(page._remoteWaiters ?? [])]) {
      if (waiter.event !== 'close') waiter.finish(targetClosed(waiter.api));
    }
  }

  function onPageCrashed(page) {
    onEvent(page, 'crash', page);
    for (const waiter of [...(page._remoteWaiters ?? [])]) {
      if (waiter.event !== 'crash') waiter.finish(new Error('Page crashed'));
    }
  }

  function waitForEvent(page, event, optionsOrPredicate = {}) {
    const api = 'page.waitForEvent';
    try {
      if (typeof event !== 'string' || !EVENTS.has(event))
        throw new UnsupportedOperationError(`${api} event ${String(event)}`);
      const options = typeof optionsOrPredicate === 'function'
        ? { predicate: optionsOrPredicate } : optionsOrPredicate;
      const timeout = checkTimeout(options, api);
      if (options.predicate !== undefined && typeof options.predicate !== 'function')
        throw new TypeError(`${api}: predicate must be a function`);
      return register(page, event, options.predicate, timeout, api);
    } catch (error) { return Promise.reject(error); }
  }

  function urlMatcher(page, expected, api) {
    if (typeof expected === 'string') {
      if (typeof page._native.urlGlob !== 'function')
        throw new UnsupportedOperationError(`${api} (select Page-notice native profile)`);
      let pattern;
      try { pattern = page._native.urlGlob(expected); }
      catch (error) { throw mapError(error, api); }
      if (typeof pattern !== 'string') throw new Error(`${api}: invalid native URL glob`);
      const regex = new RegExp(pattern);
      return value => regex.test(value.url());
    }
    if (isRegExp(expected)) return value => expected.test(value.url());
    if (typeof expected === 'function') return expected;
    throw new TypeError(`${api}: urlOrPredicate must be a string, RegExp, or function`);
  }

  function waitForNetwork(page, event, expected, options = {}) {
    const api = event === 'request' ? 'page.waitForRequest' : 'page.waitForResponse';
    try {
      const timeout = checkTimeout(options, api);
      if (Object.hasOwn(options, 'predicate'))
        throw new UnsupportedOperationError(`${api} option predicate`);
      return register(page, event, urlMatcher(page, expected, api), timeout, api);
    } catch (error) { return Promise.reject(error); }
  }

  function headers(value) {
    if (!Array.isArray(value)) throw new Error('Invalid native Page notice headers');
    return Object.freeze(value.map(header => {
      if (!header || typeof header.name !== 'string' || typeof header.value !== 'string')
        throw new Error('Invalid native Page notice header');
      return Object.freeze({ name: header.name, value: header.value });
    }));
  }

  class RemoteRequest {
    constructor(id, snapshot) {
      this._id = id;
      this._snapshot = Object.freeze({ ...snapshot, headers: headers(snapshot.headers) });
    }
    url() { return this._snapshot.url; }
    method() { return this._snapshot.method; }
    resourceType() { return this._snapshot.resourceType; }
    isNavigationRequest() { return this._snapshot.isNavigationRequest; }
    headersArray() { return this._snapshot.headers.map(header => ({ ...header })); }
    headers() { return Object.fromEntries(this._snapshot.headers.map(({ name, value }) => [name.toLowerCase(), value])); }
    allHeaders() { return Promise.resolve(this.headers()); }
    response() { return Promise.reject(new UnsupportedOperationError('request.response on remote Page notice')); }
    postDataBuffer() { throw new UnsupportedOperationError('request.postDataBuffer on remote Page notice'); }
    postData() { throw new UnsupportedOperationError('request.postData on remote Page notice'); }
  }

  class RemoteResponse {
    constructor(id, snapshot, request) {
      this._id = id;
      this._snapshot = Object.freeze({ ...snapshot, headers: headers(snapshot.headers) });
      this._request = request;
    }
    url() { return this._snapshot.url; }
    status() { return this._snapshot.status; }
    statusText() { return this._snapshot.statusText; }
    ok() { return this.status() >= 200 && this.status() <= 299; }
    fromServiceWorker() { return this._snapshot.fromServiceWorker; }
    headers() { return Object.fromEntries(this._snapshot.headers.map(({ name, value }) => [name.toLowerCase(), value])); }
    allHeaders() { return Promise.resolve(this.headers()); }
    request() { return this._request; }
    body() { return Promise.reject(new UnsupportedOperationError('response.body on remote Page notice')); }
    text() { return Promise.reject(new UnsupportedOperationError('response.text on remote Page notice')); }
    json() { return Promise.reject(new UnsupportedOperationError('response.json on remote Page notice')); }
  }

  class RemoteDownload {
    constructor(id, url, suggestedFilename) {
      this._id = id;
      this._url = url;
      this._suggestedFilename = suggestedFilename;
    }
    url() { return this._url; }
    suggestedFilename() { return this._suggestedFilename; }
    path() { return Promise.reject(new UnsupportedOperationError('download.path on remote Page notice')); }
    saveAs() { return Promise.reject(new UnsupportedOperationError('download.saveAs on remote Page notice')); }
  }

  function rememberRequest(page, id, snapshot) {
    const key = objectKey(id);
    page._remoteRequestRefs ??= new Map();
    let request = page._remoteRequestRefs.get(key)?.deref();
    if (!request) request = new RemoteRequest(id, snapshot);
    page._remoteRequestRefs.delete(key);
    page._remoteRequestRefs.set(key, new WeakRef(request));
    if (page._remoteRequestRefs.size > MAX_REQUEST_REFS)
      page._remoteRequestRefs.delete(page._remoteRequestRefs.keys().next().value);
    return request;
  }

  function materialize(page, notice) {
    if (notice.kind === 'request')
      return rememberRequest(page, notice.object, notice.request);
    if (notice.kind === 'response') {
      const request = rememberRequest(page, notice.response.request, notice.requestSnapshot);
      return new RemoteResponse(notice.object, notice.response, request);
    }
    if (notice.kind === 'download')
      return new RemoteDownload(notice.object, notice.url, notice.suggestedFilename);
    throw new Error('Invalid native Page notice kind');
  }

  return {
    objectKey, nativeIdentity, targetClosed, onEvent, onPageClosed, onPageCrashed,
    waitForEvent, waitForNetwork, materialize,
  };
};
