'use strict';

// Page/main-Frame and opt-in browser-root Chromium CDP sessions share one
// bounded lazy Browser watch; OOPIF child sessions are unsupported.

const { EventEmitter } = require('node:events');

const MAX_SESSIONS = 8;
const MAX_LISTENERS = 64;
const MAX_PENDING_DELIVERIES = 32;
const BATCH_SIZE = 64;
const MAX_BATCHES_PER_FLUSH = 8;
const MAX_PARAMS_BYTES = 1024 * 1024;
const TARGET_CLOSED = 'Target page, context or browser has been closed';
const CHILD_FRAME_MESSAGE = 'This frame does not have a separate CDP session, it is a part of the parent frame\'s session';

module.exports = ({ mapError, UnsupportedOperationError }) => {
  function hasSessions(browser) {
    if (browser._browserCdpSessions?.size) return true;
    for (const context of browser._contexts.values()) {
      if (context._cdpSessions?.size) return true;
    }
    return false;
  }

  function failWatch(state, error) {
    if (!state.active || state.terminal) return;
    state.active = false;
    const browser = state.browserRef.deref();
    if (browser?.isConnected()) browser._didWatchFail(mapError(error, 'cdpSession.on'));
  }

  function armWatch(state) {
    if (state.pending || !state.active || state.terminal) return;
    const browser = state.browserRef.deref();
    if (!browser?.isConnected() || !hasSessions(browser)) {
      state.active = false;
      return;
    }
    const native = state.nativeRef.deref();
    if (!native) {
      state.active = false;
      return;
    }
    // Exactly one native Watch ticket per Browser. A pending Promise cannot be
    // canceled from JS, so stop/close merely forbids re-arming after settlement.
    state.pending = true;
    let next;
    try { next = native.nextCdpSessionEventGeneration(state.generation); }
    catch (error) {
      state.pending = false;
      failWatch(state, error);
      return;
    }
    Promise.resolve(next).then(async generation => {
      if (!state.active || state.terminal) return;
      const owner = state.browserRef.deref();
      if (!owner?.isConnected() || !hasSessions(owner)) {
        state.active = false;
        return;
      }
      if (typeof generation !== 'bigint' || generation === state.generation)
        throw new TypeError('CDPSession native event generation is invalid');
      state.generation = generation;
      for (const session of [...(owner._browserCdpSessions?.values() ?? [])]) {
        if (!state.active || state.terminal || !owner.isConnected()) return;
        try { await session._flush(); }
        catch (error) {
          if (error?.code === 'PWRIGHT_TARGET_CLOSED') session._retire();
          else throw error;
        }
      }
      for (const context of owner._contexts.values()) {
        for (const session of [...(context._cdpSessions?.values() ?? [])]) {
          if (!state.active || state.terminal || !owner.isConnected()) return;
          try { await session._flush(); }
          catch (error) {
            // Native Page/child retirement may lead the JS lifecycle notice.
            if (error?.code === 'PWRIGHT_TARGET_CLOSED') session._retire();
            else throw error;
          }
        }
      }
    }).then(() => {
      state.pending = false;
      armWatch(state);
    }, error => {
      state.pending = false;
      failWatch(state, error);
    });
  }

  function startWatch(browser) {
    let state = browser._cdpSessionWatchState;
    if (!state) {
      state = {
        browserRef: new WeakRef(browser),
        nativeRef: new WeakRef(browser._session),
        generation: 0n,
        active: false,
        pending: false,
        terminal: false,
      };
      browser._cdpSessionWatchState = state;
    }
    if (state.terminal || browser._cdpSessionWatchPaused) return;
    state.active = true;
    armWatch(state);
  }

  function pauseBrowser(browser) {
    browser._cdpSessionWatchPaused = true;
    if (browser._cdpSessionWatchState) browser._cdpSessionWatchState.active = false;
  }

  function resumeBrowser(browser) {
    browser._cdpSessionWatchPaused = false;
    if (hasSessions(browser)) startWatch(browser);
  }

  function stopBrowser(browser) {
    browser._cdpSessionWatchPaused = true;
    const state = browser._cdpSessionWatchState;
    if (!state) return;
    state.active = false;
    state.terminal = true;
  }

  function sessionRemoved(browser) {
    if (!hasSessions(browser)) pauseBrowser(browser);
  }

  class CDPSession extends EventEmitter {
    #browser;
    #sessions;
    #native;
    #token;
    #pageId;
    #closed = false;
    #detaching = false;
    #pending = 0;
    #tail = Promise.resolve();

    constructor(browser, sessions, pageId, token) {
      super();
      this.#browser = browser;
      this.#sessions = sessions;
      this.#native = browser._session;
      this.#pageId = pageId;
      this.#token = token;
      this.setMaxListeners(MAX_LISTENERS);
    }

    get _pageId() { return this.#pageId; }

    #emitListenerEvent(event, value) {
      try { this.emit(event, value); }
      catch (error) {
        // User callbacks are not native command/watch failures. Node owns the
        // original exception channel; the event is still consumed exactly once.
        this.#native.reportCallbackException(error);
      }
    }

    #checkListenerCapacity() {
      const count = this.eventNames().reduce((total, name) => total + this.listenerCount(name), 0);
      if (count >= MAX_LISTENERS)
        throw new Error(`CDPSession event handler limit exceeded (${MAX_LISTENERS})`);
    }

    on(event, listener) {
      this.#checkListenerCapacity();
      return super.on(event, listener);
    }

    addListener(event, listener) { return this.on(event, listener); }

    prependListener(event, listener) {
      this.#checkListenerCapacity();
      return super.prependListener(event, listener);
    }

    #serialize(operation) {
      if (this.#pending >= MAX_PENDING_DELIVERIES)
        throw new Error(`CDPSession pending delivery capacity exceeded (${MAX_PENDING_DELIVERIES})`);
      this.#pending += 1;
      const result = this.#tail.then(operation);
      this.#tail = result.catch(() => {});
      return result.finally(() => { this.#pending -= 1; });
    }

    async _flush() {
      if (this.#closed) return;
      return this.#serialize(async () => {
        if (this.#closed) return;
        for (let pass = 0; pass < MAX_BATCHES_PER_FLUSH; pass += 1) {
          let batch;
          try { batch = await this.#native.cdpSessionClaim(this.#token, BATCH_SIZE); }
          catch (error) { throw mapError(error, 'cdpSession.on'); }
          if (!batch || typeof batch.peek !== 'function' || typeof batch.ackNext !== 'function'
              || typeof batch.rollback !== 'function') {
            batch?.rollback?.();
            throw new TypeError('CDPSession native event batch is invalid');
          }
          let delivered = 0;
          try {
            for (;;) {
              if (this.#closed) return;
              const event = batch.peek();
              if (event === null) break;
              if (!Array.isArray(event) || event.length !== 2 || typeof event[0] !== 'string'
                  || !event[1] || typeof event[1] !== 'object' || Array.isArray(event[1]))
                throw new TypeError('CDPSession native event is invalid');
              const [method, params] = event;
              this.#emitListenerEvent(method, params);
              this.#emitListenerEvent('event', { method, params });
              batch.ackNext();
              delivered += 1;
            }
          } finally { batch.rollback(); }
          if (delivered < BATCH_SIZE) return;
        }
      });
    }

    async send(method, params = {}) {
      if (this.#closed || this.#detaching) throw new Error(TARGET_CLOSED);
      if (typeof method !== 'string' || !method)
        throw new TypeError('cdpSession.send: method must be a nonempty string');
      if (!params || typeof params !== 'object' || Array.isArray(params))
        throw new TypeError('cdpSession.send: params must be an object');
      const paramsJson = JSON.stringify(params, (_key, value) => {
        if (typeof value === 'number' && !Number.isFinite(value))
          throw new TypeError('cdpSession.send: params must contain finite numbers');
        return value;
      });
      if (typeof paramsJson !== 'string')
        throw new TypeError('cdpSession.send: params must be JSON-serializable');
      if (Buffer.byteLength(paramsJson, 'utf8') > MAX_PARAMS_BYTES)
        throw new TypeError('cdpSession.send: params exceed 1 MiB');
      let result;
      try { result = await this.#native.cdpSessionSend(this.#token, method, paramsJson); }
      catch (error) { throw mapError(error, 'cdpSession.send'); }
      await this._flush();
      return result;
    }

    async detach() {
      if (this.#closed || this.#detaching) throw new Error(TARGET_CLOSED);
      this.#detaching = true;
      try {
        await this.#serialize(async () => {
          if (this.#closed) throw new Error(TARGET_CLOSED);
          try { await this.#native.cdpSessionDetach(this.#token); }
          catch (error) { throw mapError(error, 'cdpSession.detach'); }
          this._retire();
        });
      } catch (error) {
        this.#detaching = false;
        throw error;
      }
    }

    _retire() {
      if (this.#closed) return;
      this.#closed = true;
      this.#sessions.delete(this.#token);
      sessionRemoved(this.#browser);
      this.#emitListenerEvent('close', this);
    }

    _abandon() { this.#closed = true; }
  }

  async function create(context, target, Page, Frame) {
    const page = target instanceof Page ? target : target instanceof Frame ? target.page() : null;
    if (!page)
      throw new TypeError('browserContext.newCDPSession: page: expected Page or Frame');
    if (page.context() !== context)
      throw new TypeError('browserContext.newCDPSession: Page or Frame belongs to a different BrowserContext');
    const browser = context.browser();
    const live = () => !browser._cdpClosePending && !context._cdpClosePending
      && !page._cdpClosePending && !browser._cdpClosed && !context._cdpClosed
      && !page._cdpClosed && !browser._disconnected && browser.isConnected()
      && browser._contexts.get(context._id) === context && context._pages.get(page._id) === page;
    if (!live()) throw new Error(TARGET_CLOSED);
    const frameId = target instanceof Frame ? target._id : null;
    if (frameId !== null && frameId !== page._mainFrame._id)
      throw new Error(CHILD_FRAME_MESSAGE);
    const native = browser._session;
    if (['cdpSessionCreate', 'cdpSessionSend', 'cdpSessionClaim', 'cdpSessionDetach',
      'nextCdpSessionEventGeneration', 'reportCallbackException']
      .some(method => typeof native[method] !== 'function'))
      throw new UnsupportedOperationError('browserContext.newCDPSession (select cdp-session native profile)');
    const sessions = context._cdpSessions ??= new Map();
    if (sessions.size >= MAX_SESSIONS)
      throw new Error(`CDPSession capacity exceeded (${MAX_SESSIONS})`);
    let lease;
    try { lease = await native.cdpSessionCreate(context._id, page._id, frameId); }
    catch (error) { throw mapError(error, 'browserContext.newCDPSession'); }
    let token;
    let session;
    let claimed = false;
    try {
      if (!lease || typeof lease.token !== 'function' || typeof lease.claim !== 'function'
          || typeof lease.abort !== 'function')
        throw new TypeError('CDPSession native delivery lease is invalid');
      // A close may win after native attachment but before the Promise result
      // is observed. Abort now; waiting for JS GC would retain native capacity.
      if (!live()) throw new Error(TARGET_CLOSED);
      token = lease.token();
      if (typeof token !== 'bigint' || token < 0n || sessions.has(token))
        throw new TypeError('CDPSession native token is invalid');
      session = new CDPSession(browser, sessions, page._id, token);
      sessions.set(token, session);
      lease.claim();
      claimed = true;
    }
    catch (error) {
      if (session && sessions.get(token) === session) sessions.delete(token);
      session?._abandon();
      if (!claimed && typeof lease?.abort === 'function') {
        try { lease.abort(); }
        catch (cleanup) {
          throw new AggregateError([error, cleanup], 'CDPSession attachment and cleanup failed', { cause: error });
        }
      }
      throw mapError(error, 'browserContext.newCDPSession');
    }
    startWatch(browser);
    return session;
  }

  async function createBrowser(browser) {
    const api = 'browser.newBrowserCDPSession';
    const live = () => !browser._cdpClosePending && !browser._cdpClosed
      && !browser._disconnected && browser.isConnected();
    if (!live()) throw new Error(TARGET_CLOSED);
    const native = browser._session;
    if (['cdpBrowserSessionCreate', 'cdpSessionSend', 'cdpSessionClaim', 'cdpSessionDetach',
      'nextCdpSessionEventGeneration', 'reportCallbackException']
      .some(method => typeof native[method] !== 'function'))
      throw new UnsupportedOperationError(`${api} (select browser-cdp-session native profile)`);
    const sessions = browser._browserCdpSessions;
    if (sessions.size >= MAX_SESSIONS)
      throw new Error(`CDPSession capacity exceeded (${MAX_SESSIONS})`);
    let lease;
    try { lease = await native.cdpBrowserSessionCreate(); }
    catch (error) { throw mapError(error, api); }
    let token;
    let session;
    let claimed = false;
    try {
      if (!lease || typeof lease.token !== 'function' || typeof lease.claim !== 'function'
          || typeof lease.abort !== 'function')
        throw new TypeError('CDPSession native delivery lease is invalid');
      if (!live()) throw new Error(TARGET_CLOSED);
      token = lease.token();
      if (typeof token !== 'bigint' || token < 0n || sessions.has(token))
        throw new TypeError('CDPSession native token is invalid');
      session = new CDPSession(browser, sessions, null, token);
      sessions.set(token, session);
      lease.claim();
      claimed = true;
    } catch (error) {
      if (session && sessions.get(token) === session) sessions.delete(token);
      session?._abandon();
      if (!claimed && typeof lease?.abort === 'function') {
        try { lease.abort(); }
        catch (cleanup) {
          throw new AggregateError([error, cleanup], 'CDPSession attachment and cleanup failed', { cause: error });
        }
      }
      throw mapError(error, api);
    }
    startWatch(browser);
    return session;
  }

  async function flushBrowser(browser) {
    for (const session of [...browser._browserCdpSessions.values()]) {
      try { await session._flush(); }
      catch (error) {
        if (error?.code === 'PWRIGHT_TARGET_CLOSED') session._retire();
        else throw error;
      }
    }
  }

  function retireBrowser(browser) {
    for (const session of [...browser._browserCdpSessions.values()]) session._retire();
  }

  async function flushPage(context, pageId) {
    for (const session of [...(context._cdpSessions?.values() ?? [])]) {
      if (session._pageId === pageId) await session._flush();
    }
  }

  function retirePage(context, pageId) {
    for (const session of [...(context._cdpSessions?.values() ?? [])]) {
      if (session._pageId === pageId) session._retire();
    }
  }

  function retireContext(context) {
    for (const session of [...(context._cdpSessions?.values() ?? [])]) session._retire();
  }

  return {
    create, createBrowser, flushBrowser, flushPage, retireBrowser, retirePage, retireContext,
    pauseBrowser, resumeBrowser, stopBrowser,
  };
};
