'use strict';

const { EventEmitter } = require('node:events');

// A finalized JS facade must not retain a server connection indefinitely.
// Explicit close is still the deterministic lifecycle contract.
const remoteFinalizers = new FinalizationRegistry(native => {
  Promise.resolve().then(() => native.close()).catch(() => {});
});
const MAX_EARLY_TERMINALS = 128;

module.exports = ({ mapError, UnsupportedOperationError, TimeoutError, pageEventsSelected }) => {
  const pageEvents = pageEventsSelected
    ? require('./remote-page-events.cjs')({ mapError, UnsupportedOperationError, TimeoutError })
    : null;
  function requireEmptyOptions(options, api) {
    if (options === null || typeof options !== 'object' || Array.isArray(options)
      || Object.keys(options).length)
      throw new UnsupportedOperationError(`${api} options on remote session`);
  }

  function notifyTerminal(target, event, native) {
    try { target.emit(event, target); }
    catch (error) {
      try { native.reportCallbackException(error); }
      catch (reportError) { process.nextTick(() => { throw reportError; }); }
    }
  }

  function notifyPayload(target, event, value, native) {
    try { target.emit(event, value); }
    catch (error) {
      try { native.reportCallbackException(error); }
      catch (reportError) { process.nextTick(() => { throw reportError; }); }
    }
  }

  class RemotePage extends EventEmitter {
    constructor(context, native) {
      super();
      this._context = context;
      this._native = native;
      this._terminalClosed = false;
      this._ownsContext = false;
      this._noticeKey = context._browser._pageEventsEnabled
        ? pageEvents.objectKey(pageEvents.nativeIdentity(native)) : null;
    }
    context() { return this._context; }
    isClosed() { return this._terminalClosed || !this._native.isOpen(); }
    async close(options = {}) {
      requireEmptyOptions(options, 'page.close');
      if (!this._ownsContext)
        throw new UnsupportedOperationError('page.close on remote Context-owned Page');
      await this._context.close();
    }
    goto() { throw new UnsupportedOperationError('page.goto on remote session'); }
    evaluate() { throw new UnsupportedOperationError('page.evaluate on remote session'); }
    _didClose() {
      if (this._terminalClosed) return false;
      this._terminalClosed = true;
      if (this._context._browser._pageEventsEnabled) {
        pageEvents.onPageClosed(this);
        this._remoteRequestRefs?.clear();
      }
      this._context._pages.delete(this._noticeKey ?? this._native.guid());
      notifyTerminal(this, 'close', this._context._browser._native);
      return true;
    }
  }

  class EventRemotePage extends RemotePage {
    waitForEvent(event, optionsOrPredicate = {}) {
      return pageEvents.waitForEvent(this, event, optionsOrPredicate);
    }
    waitForRequest(urlOrPredicate, options = {}) {
      return pageEvents.waitForNetwork(this, 'request', urlOrPredicate, options);
    }
    waitForResponse(urlOrPredicate, options = {}) {
      return pageEvents.waitForNetwork(this, 'response', urlOrPredicate, options);
    }
  }

  class RemoteBrowserContext extends EventEmitter {
    constructor(browser, native) {
      super();
      this._browser = browser;
      this._native = native;
      this._pages = new Map();
      this._terminalClosed = false;
      this._noticeKey = browser._pageEventsEnabled
        ? pageEvents.objectKey(pageEvents.nativeIdentity(native)) : null;
      if (browser._pageEventsEnabled)
        this._closeApplied = new Promise(resolve => { this._resolveCloseApplied = resolve; });
    }
    browser() { return this._browser; }
    _prunePages(guids = this._native.pageGuids()) {
      if (this._browser._pageEventsEnabled) return guids;
      const live = new Set(guids);
      for (const [guid, page] of this._pages) {
        if (!live.has(guid) || page.isClosed()) this._pages.delete(guid);
      }
      return guids;
    }
    pages() {
      if (this._terminalClosed || !this._browser._native.isConnected()) return [];
      const guids = this._prunePages();
      return guids.map(guid => {
        const page = this._browser._pageEventsEnabled
          ? [...this._pages.values()].find(value => value._native.guid() === guid
            && !value._terminalClosed && value._native.isOpen())
          : this._pages.get(guid);
        if (!page) throw new UnsupportedOperationError('context.pages with externally created remote Page');
        return page;
      });
    }
    async newPage() {
      const browser = this._browser;
      const fence = browser._creationFences?.pages;
      if (fence) fence.inFlight += 1;
      try {
        const native = await this._native.newPage();
        if (!fence) this._prunePages();
        const page = browser._pageEventsEnabled
          ? new EventRemotePage(this, native) : new RemotePage(this, native);
        if (fence) {
          browser._throwWatchFailure('browserContext.newPage');
          const early = fence.terminals.get(page._noticeKey);
          if (early === 'pageCrashed')
            throw new Error('browserContext.newPage: Page crashed before publication');
          if (early || this._terminalClosed || browser._disconnected || !native.isOpen())
            throw pageEvents.targetClosed('browserContext.newPage');
        }
        this._pages.set(page._noticeKey ?? native.guid(), page);
        return page;
      } catch (error) { throw mapError(error, 'browserContext.newPage'); }
      finally {
        if (fence && --fence.inFlight === 0) fence.terminals.clear();
      }
    }
    async close(options = {}) {
      requireEmptyOptions(options, 'browserContext.close');
      const browser = this._browser;
      if (this._terminalClosed) {
        if (browser._pageEventsEnabled && browser._watchFailure) {
          await browser._terminalApplied;
          browser._throwWatchFailure('browserContext.close');
        }
        return;
      }
      if (this._native.isOpen()) {
        if (typeof this._native.close !== 'function')
          throw new UnsupportedOperationError('browserContext.close (rebuild native addon)');
        try {
          const closing = Promise.resolve().then(() => this._native.close());
          if (browser._pageEventsEnabled) await Promise.race([closing, this._closeApplied]);
          else await closing;
        }
        catch (error) {
          if (!this._terminalClosed) throw mapError(error, 'browserContext.close');
        }
      }
      if (browser._pageEventsEnabled) {
        await this._closeApplied;
        browser._throwWatchFailure('browserContext.close');
      }
      else this._didClose();
    }
    _didClose() {
      if (this._terminalClosed) return false;
      this._terminalClosed = true;
      for (const page of [...this._pages.values()]) page._didClose();
      this._pages.clear();
      const key = this._noticeKey ?? this._native.guid();
      if (this._browser._contexts.get(key) === this) this._browser._contexts.delete(key);
      notifyTerminal(this, 'close', this._browser._native);
      this._resolveCloseApplied?.();
      return true;
    }
  }

  return class RemoteBrowser extends EventEmitter {
  constructor(type, native, pageEventsEnabled = false) {
    super();
    this._type = type;
    this._native = native;
    this._disconnected = false;
    this._contexts = new Map();
    this._pageEventsEnabled = pageEventsEnabled;
    if (pageEventsEnabled) {
      if (!pageEvents)
        throw new Error('Selected Page-notice addon lacks its JS adapter');
      if (typeof native.nextPageNotice !== 'function')
        throw new Error('Selected Page-notice addon lacks nextPageNotice');
      this._terminalApplied = new Promise(resolve => { this._resolveTerminalApplied = resolve; });
      this._creationFences = {
        contexts: { inFlight: 0, terminals: new Map() },
        pages: { inFlight: 0, terminals: new Map() },
      };
    }
    const browserRef = new WeakRef(this);
    remoteFinalizers.register(this, native, this);
    if (pageEventsEnabled) this._startPageNoticePump(native, browserRef);
    else Promise.resolve().then(() => native.waitTerminal()).then(
      () => browserRef.deref()?._didDisconnect(),
      error => browserRef.deref()?._didWatchFail(error),
    ).catch(error => process.nextTick(() => { throw error; }));
  }

  _startPageNoticePump(native, browserRef) {
    const pull = () => {
      Promise.resolve().then(() => native.nextPageNotice()).then(async notice => {
        const browser = browserRef.deref();
        if (!browser) return;
        if (!(await browser._applyPageNotice(notice))) pull();
      }).catch(error => browserRef.deref()?._didWatchFail(error));
    };
    pull();
  }

  async _applyPageNotice(notice) {
    if (!notice || typeof notice !== 'object') throw new Error('Invalid native Page notice');
    const kind = notice.kind;
    if (kind === 'disconnected') {
      if (!this._browserClosedNoticed)
        throw new Error('Native Page notice disconnected before browserClosed');
      await this._didDisconnect();
      return true;
    }
    if (kind === 'browserClosed') {
      pageEvents.objectKey(notice.object);
      this._browserClosedNoticed = true;
      return false;
    }
    if (kind === 'contextClosed') {
      const key = pageEvents.objectKey(notice.object);
      const context = this._contexts.get(key);
      if (context) context._didClose();
      else this._recordEarlyTerminal(this._creationFences.contexts, key, kind);
      await Promise.resolve();
      return false;
    }
    if (kind === 'pageClosed' || kind === 'pageCrashed') {
      const key = pageEvents.objectKey(notice.object);
      const page = [...this._contexts.values()].map(context => context._pages.get(key)).find(Boolean);
      if (page) {
        if (kind === 'pageClosed') page._didClose();
        else {
          pageEvents.onPageCrashed(page);
          notifyTerminal(page, 'crash', this._native);
        }
      } else this._recordEarlyTerminal(this._creationFences.pages, key, kind);
      // Give rejected waiter reactions a checkpoint before disconnected.
      await Promise.resolve();
      return false;
    }
    if (kind === 'request' || kind === 'response' || kind === 'download') {
      const key = pageEvents.objectKey(notice.page);
      const page = [...this._contexts.values()].map(context => context._pages.get(key)).find(Boolean);
      if (!page || page._terminalClosed) return false;
      const value = pageEvents.materialize(page, notice);
      pageEvents.onEvent(page, kind, value);
      notifyPayload(page._context, kind, value, this._native);
      notifyPayload(page, kind, value, this._native);
      return false;
    }
    throw new Error(`Invalid native Page notice kind: ${String(kind)}`);
  }

  browserType() { return this._type; }
  version() { return this._native.version(); }
  isConnected() { return !this._disconnected && this._native.isConnected(); }

  contexts() {
    if (this._disconnected || !this._native.isConnected()) return [];
    const guids = this._pruneContexts();
    return guids.map(guid => {
      const context = this._pageEventsEnabled
        ? [...this._contexts.values()].find(value => value._native.guid() === guid
          && !value._terminalClosed && value._native.isOpen())
        : this._contexts.get(guid);
      if (!context) throw new UnsupportedOperationError('browser.contexts with externally created remote Context');
      return context;
    });
  }

  async newContext(options = {}) {
    requireEmptyOptions(options, 'browser.newContext');
    const fence = this._creationFences?.contexts;
    if (fence) fence.inFlight += 1;
    try {
      const native = await this._native.newContext();
      if (!fence) this._pruneContexts();
      const context = new RemoteBrowserContext(this, native);
      if (fence) {
        this._throwWatchFailure('browser.newContext');
        if (fence.terminals.has(context._noticeKey) || this._disconnected || !native.isOpen())
          throw pageEvents.targetClosed('browser.newContext');
      }
      this._contexts.set(context._noticeKey ?? native.guid(), context);
      return context;
    } catch (error) { throw mapError(error, 'browser.newContext'); }
    finally {
      if (fence && --fence.inFlight === 0) fence.terminals.clear();
    }
  }

  _recordEarlyTerminal(fence, key, kind) {
    if (!fence?.inFlight) return;
    if (!fence.terminals.has(key) && fence.terminals.size >= MAX_EARLY_TERMINALS)
      throw new Error(`Native Page notice early-terminal capacity exceeds ${MAX_EARLY_TERMINALS}`);
    fence.terminals.set(key, kind);
  }

  _pruneContexts(guids = this._native.contextGuids()) {
    if (this._pageEventsEnabled) return guids;
    const live = new Set(guids);
    for (const [guid, context] of this._contexts) {
      if (!live.has(guid) || !context._native.isOpen()) {
        context._pages.clear();
        this._contexts.delete(guid);
      }
    }
    return guids;
  }

  async close(options = {}) {
    if (options === null || typeof options !== 'object' || Object.keys(options).length)
      throw new TypeError('browser.close: options must be an empty object');
    if (this._disconnected) {
      if (this._pageEventsEnabled) {
        await this._terminalApplied;
        this._throwWatchFailure('browser.close');
      }
      return;
    }
    try {
      const closing = Promise.resolve().then(() => this._native.close());
      if (this._pageEventsEnabled) await Promise.race([closing, this._terminalApplied]);
      else await closing;
    }
    catch (error) {
      if (!this._disconnected) throw mapError(error, 'browser.close');
    }
    if (this._pageEventsEnabled) {
      await this._terminalApplied;
      this._throwWatchFailure('browser.close');
    } else this._didDisconnect();
  }

  _throwWatchFailure(api) {
    if (this._watchFailure) throw mapError(this._watchFailure, api);
  }

  async _didDisconnect() {
    if (this._disconnected) return false;
    this._disconnected = true;
    remoteFinalizers.unregister(this);
    // On a projection failure, no child notices may remain. Reject Page
    // waiters before publishing Context and Browser terminal callbacks.
    if (this._pageEventsEnabled) {
      this._creationFences.contexts.terminals.clear();
      this._creationFences.pages.terminals.clear();
      for (const context of this._contexts.values())
        for (const page of [...context._pages.values()]) page._didClose();
      await Promise.resolve();
    }
    for (const context of [...this._contexts.values()]) {
      context._didClose();
    }
    this._contexts.clear();
    notifyTerminal(this, 'disconnected', this._native);
    this._resolveTerminalApplied?.();
    return true;
  }

  _didWatchFail(error) {
    this._watchFailure = error;
    this._didDisconnect().catch(failure => process.nextTick(() => { throw failure; }));
    if (this._pageEventsEnabled)
      Promise.resolve().then(() => this._native.close()).catch(() => {});
  }

  async newPage(options = {}) {
    requireEmptyOptions(options, 'browser.newPage');
    if (typeof this._native.supportsContextClose !== 'function'
        || !this._native.supportsContextClose())
      throw new UnsupportedOperationError('browser.newPage (rebuild native addon for implicit Context cleanup)');
    const context = await this.newContext();
    try {
      const page = await context.newPage();
      page._ownsContext = true;
      return page;
    } catch (error) {
      try { await context.close(); }
      catch (cleanupError) {
        throw new AggregateError([error, cleanupError], 'browser.newPage failed and implicit Context cleanup failed');
      }
      throw error;
    }
  }
  newBrowserCDPSession() { throw new UnsupportedOperationError('browser.newBrowserCDPSession on remote session'); }
  pwrightMetrics() { throw new UnsupportedOperationError('browser.pwrightMetrics on remote session'); }
  pwrightMetadataMetrics() { throw new UnsupportedOperationError('browser.pwrightMetadataMetrics on remote session'); }
  pwrightFrameTreeMetrics() { throw new UnsupportedOperationError('browser.pwrightFrameTreeMetrics on remote session'); }
  pwrightContextMetadataMetrics() { throw new UnsupportedOperationError('browser.pwrightContextMetadataMetrics on remote session'); }
  pwrightFfiMetrics() { throw new UnsupportedOperationError('browser.pwrightFfiMetrics on remote session'); }
  pwrightConversionMetrics() { throw new UnsupportedOperationError('browser.pwrightConversionMetrics on remote session'); }
  pwrightLocatorConversionMetrics() { throw new UnsupportedOperationError('browser.pwrightLocatorConversionMetrics on remote session'); }
  pwrightLocatorMetrics() { throw new UnsupportedOperationError('browser.pwrightLocatorMetrics on remote session'); }
  };
};
