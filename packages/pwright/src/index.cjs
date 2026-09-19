'use strict';

const { EventEmitter } = require('node:events');
const FrameCache = require('./frame-cache.cjs');
const lifecycleEvents = require('./lifecycle-events.cjs');
const { existsSync } = require('node:fs');
const { nativeArtifact, verifyNativeArtifact } = require('./native-artifact.cjs');
const { setImmediate: scheduleConversion } = require('node:timers');
const fileInputDebug = require('./file-input-debug.cjs');
const arrayBufferMaximum = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength')?.get;

class TimeoutError extends Error {
  constructor(message) { super(message); this.name = 'TimeoutError'; }
}

class UnsupportedOperationError extends Error {
  constructor(api) {
    super(`${api} is not supported by the current Pwright native profile`);
    this.name = 'UnsupportedOperationError';
    this.code = 'PWRIGHT_UNSUPPORTED';
  }
}

const compatibility = Object.freeze({
  oracle: 'playwright@1.58.2',
  profile: 'chromium-attach-navigation-read-evaluation-locator-actions',
  browserOwnership: 'external',
  apiCoverage: 'experimental-subset',
  nativeProfile: nativeArtifact.profile,
});

let addon;
let nativeEnvironment;
function nativeAddon() {
  if (addon) return addon;
  const artifact = nativeArtifact.path;
  if (!existsSync(artifact)) {
    throw new Error(`Pwright native artifact is missing (${process.platform}-${process.arch}, profile=${nativeArtifact.profile}). Run PWRIGHT_NATIVE_PROFILE=${nativeArtifact.profile} pnpm build in packages/pwright; no browser or binary is downloaded at import time.`);
  }
  const loaded = require(artifact);
  verifyNativeArtifact(loaded, nativeArtifact);
  addon = loaded;
  return addon;
}

function mapError(error, api) {
  if (error instanceof UnsupportedOperationError) return error;
  const match = /^(PWRIGHT_[A-Z_]+): ([\s\S]*)$/.exec(error.message);
  if (!match) return error;
  if (match[1] === 'PWRIGHT_UNSUPPORTED') return new UnsupportedOperationError(`${api}: ${match[2]}`);
  const mapped = match[1] === 'PWRIGHT_TIMEOUT'
    ? new TimeoutError(`${api}: ${match[2]}`)
    : match[1] === 'PWRIGHT_INVALID_ARGUMENT'
      ? new TypeError(`${api}: ${match[2]}`)
      : new Error(`${api}: ${match[2]}`);
  mapped.code = match[1];
  return mapped;
}

// Language exception port only. Rust chooses the channel-validation phase;
// strict JS assignment preserves setters, read-only properties and primitive
// throws without serializing or replacing the user's exception object.
function headerChannelError(error, api) {
  error.message = api + ': ' + error.message;
  throw error;
}

function validateOptions(options, supported, api) {
  if (!options || typeof options !== 'object' || Array.isArray(options))
    throw new TypeError(`${api}: options must be an object`);
  for (const key of Object.keys(options)) {
    if (!supported.includes(key)) throw new UnsupportedOperationError(`${api} option ${key}`);
  }
}

const locatorApi = require('./locator.cjs')({ mapError, validateOptions, UnsupportedOperationError });
const fileInput = require('./file-input.cjs')({ mapError, validateOptions, UnsupportedOperationError });
const screenshotOptions = require('./screenshot.cjs')({ validateOptions });
const Keyboard = require('./keyboard.cjs')({ mapError, validateOptions });
const Mouse = require('./mouse.cjs')({ mapError, validateOptions });
const handles = require('./handles.cjs')({ mapError, UnsupportedOperationError });

// This continuation captures only a WeakRef, never Browser or NativeSession.
// GC is fallback cleanup; explicit close remains the deterministic contract.
function watchSession(signal, browserRef) {
  signal.wait().then(connected => {
    if (!connected) browserRef.deref()?._didDisconnect();
    else if (browserRef.deref()) watchSession(signal, browserRef);
  });
}

class BrowserType {
  name() { return 'chromium'; }

  async connectOverCDP(endpointURL, options = {}) {
    const api = 'browserType.connectOverCDP';
    if (endpointURL !== null && typeof endpointURL === 'object') {
      validateOptions(endpointURL, ['endpointURL', 'timeout'], api);
      options = { ...endpointURL };
      endpointURL = options.endpointURL;
      delete options.endpointURL;
    }
    validateOptions(options, ['timeout'], api);
    if (typeof endpointURL !== 'string') throw new TypeError(`${api}: endpointURL must be a string`);
    const timeout = options.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    try {
      const { NativeEnvironment } = nativeAddon();
      nativeEnvironment ??= new NativeEnvironment(scheduleConversion, arrayBufferMaximum);
      const session = await nativeEnvironment.connectOverCDP(endpointURL, timeout);
      try {
        if (typeof session.initializeFileInputTimeouts === 'function')
          session.initializeFileInputTimeouts(...fileInputDebug);
      } catch (primary) {
        // No public Browser exists yet. The acquired session still owns its
        // resources, including when selected metadata initialization fails.
        try { await session.close(); }
        catch (cleanup) {
          throw new AggregateError([primary, cleanup], 'File-input timeout initialization and session cleanup failed', { cause: primary });
        }
        throw primary;
      }
      return new Browser(this, session);
    } catch (error) { throw mapError(error, api); }
  }

  async launch() { throw new UnsupportedOperationError('browserType.launch'); }
  async launchPersistentContext() { throw new UnsupportedOperationError('browserType.launchPersistentContext'); }
  async connect() { throw new UnsupportedOperationError('browserType.connect'); }
}

class Browser extends EventEmitter {
  constructor(type, session) {
    super();
    this._type = type;
    this._session = session;
    this._contexts = new Map();
    // Legacy artifacts predate the Rust-owned one-shot event publication port.
    if (!session.publishDisconnect) this._disconnected = false;
    const browserRef = new WeakRef(this);
    if (session.setLifecycleCallback) session.setLifecycleCallback(lifecycleEvents(browserRef, {
      context: (browser, value) => new BrowserContext(browser, value.id, value.settings),
      page: (context, value) => new Page(context, value),
    }));
    watchSession(session.sessionSignal(), browserRef);
  }

  browserType() { return this._type; }
  isConnected() { return this._session.isConnected(); }
  version() { return this._session.version(); }
  contexts() {
    if (this._session.contextSnapshot) {
      try {
        const snapshot = this._session.contextSnapshot([...this._contexts.keys()]);
        const active = new Set(snapshot.ids);
        for (const id of this._contexts.keys()) if (!active.has(id)) this._contexts.delete(id);
        for (const value of snapshot.newContexts) this._wrapContext(value);
        return snapshot.ids.map(id => this._contexts.get(id));
      } catch (error) { throw mapError(error, 'browser.contexts'); }
    }
    const descriptors = this._session.contexts();
    const active = new Set(descriptors.map(value => value.id));
    for (const id of this._contexts.keys()) if (!active.has(id)) this._contexts.delete(id);
    return descriptors.map(value => this._wrapContext(value));
  }
  _wrapContext(value) {
    if (!this._contexts.has(value.id)) this._contexts.set(value.id, new BrowserContext(this, value.id, value.settings));
    return this._contexts.get(value.id);
  }

  async close(options = {}) {
    validateOptions(options, [], 'browser.close');
    try { await this._session.close(); }
    catch (error) { throw mapError(error, 'browser.close'); }
    this._didDisconnect();
  }

  _didDisconnect() {
    if (this._session.publishDisconnect) {
      if (!this._session.publishDisconnect()) return;
    } else {
      if (this._disconnected) return;
      this._disconnected = true;
    }
    this._contexts.clear();
    try { this.emit('disconnected', this); }
    catch (error) {
      // A native notification's listener error is not an API/watch rejection.
      // Rust reports the original value through Node's exception channel with
      // no retained callback or queue. Older artifacts retain their old behavior.
      if (!this._session.reportCallbackException) throw error;
      this._session.reportCallbackException(error);
    }
  }

  pwrightMetrics() { return this._session.metrics(); }
  pwrightMetadataMetrics() { return this._session.frameMetadataMetrics(); }
  pwrightFrameTreeMetrics() {
    if (!this._session.frameTreeMetrics) throw new UnsupportedOperationError('pwrightFrameTreeMetrics (rebuild native addon)');
    return this._session.frameTreeMetrics();
  }
  pwrightContextMetadataMetrics() {
    if (!this._session.contextMetadataMetrics) throw new UnsupportedOperationError('pwrightContextMetadataMetrics (rebuild native addon)');
    return this._session.contextMetadataMetrics();
  }
  pwrightFfiMetrics() { return this._session.ffiMetrics(); }
  pwrightConversionMetrics() { return this._session.conversionMetrics(); }
  pwrightLocatorConversionMetrics() { return this._session.locatorConversionMetrics(); }
  pwrightLocatorMetrics() { return this._session.locatorMetrics(); }
  async newContext(options = {}) {
    // Language-level copy mirrors the pinned selector-options adapter: read
    // own enumerable option getters once, without inheriting option fields.
    options = { ...options };
    validateOptions(options, ['baseURL', 'extraHTTPHeaders'], 'browser.newContext');
    if (options.baseURL !== undefined && typeof options.baseURL !== 'string')
      throw new TypeError('browser.newContext: baseURL must be a string');
    if (options.baseURL !== undefined && !this._session.contextSnapshot)
      throw new UnsupportedOperationError('browser.newContext baseURL (rebuild native addon)');
    if (options.extraHTTPHeaders && !this._session.newContextWithExtraHttpHeaders)
      throw new UnsupportedOperationError('browser.newContext extraHTTPHeaders (select extra-headers native profile)');
    // Do not map arbitrary synchronous user getter/conversion exceptions.
    const pending = options.extraHTTPHeaders
      ? this._session.newContextWithExtraHttpHeaders(options.extraHTTPHeaders, headerChannelError, options.baseURL)
      : this._session.newContext(options.baseURL);
    try { return this._wrapContext(await pending); }
    catch (error) { throw mapError(error, 'browser.newContext'); }
  }
  async newPage() { throw new UnsupportedOperationError('browser.newPage'); }
}

class BrowserContext extends EventEmitter {
  constructor(browser, id, settings) { super(); this._browser = browser; this._id = id; this._settings = settings; this._pages = new Map(); }
  browser() { return this._browser; }
  async setExtraHTTPHeaders(headers) {
    const session = this._browser._session;
    if (!session.setContextExtraHttpHeaders) throw new UnsupportedOperationError('browserContext.setExtraHTTPHeaders (select extra-headers native profile)');
    // Synchronous language/getter exceptions must propagate unchanged.
    const pending = session.setContextExtraHttpHeaders(this._id, headers, headerChannelError);
    try { await pending; }
    catch (error) { throw mapError(error, 'browserContext.setExtraHTTPHeaders'); }
  }
  pages() {
    if (this._browser._session.pageSnapshot) {
      try {
        const snapshot = this._browser._session.pageSnapshot(this._id, [...this._pages.keys()]);
        const active = new Set(snapshot.ids);
        for (const id of this._pages.keys()) if (!active.has(id)) this._pages.delete(id);
        for (const value of snapshot.newPages) this._wrapPage(value);
        return snapshot.ids.map(id => this._pages.get(id));
      } catch (error) { throw mapError(error, 'browserContext.pages'); }
    }
    const descriptors = this._browser._session.pages(this._id);
    const active = new Set(descriptors.map(value => value.id));
    for (const id of this._pages.keys()) if (!active.has(id)) this._pages.delete(id);
    return descriptors.map(value => this._wrapPage(value));
  }
  _wrapPage(value) {
    if (!this._pages.has(value.id)) this._pages.set(value.id, new Page(this, value));
    return this._pages.get(value.id);
  }
  _resolveHandleFrame(id) {
    // Rust selects a portable Frame in this Context and publishes topology
    // first. This layer only retrieves its canonical cached wrapper. A Frame
    // retired between native settlement and lookup has no current owner.
    for (const page of this.pages()) {
      const frame = page.frames().find(value => value._id === id);
      if (frame) return frame;
    }
    return null;
  }
  async newPage(...args) {
    if (args.length) throw new UnsupportedOperationError('browserContext.newPage arguments');
    try { return this._wrapPage(await this._browser._session.newPage(this._id)); }
    catch (error) { throw mapError(error, 'browserContext.newPage'); }
  }
  async close(options = {}) {
    validateOptions(options, [], 'browserContext.close');
    try {
      await this._browser._session.closeContext(this._id);
      this._pages.clear();
      this._browser._contexts.delete(this._id);
      if (!this._browser.isConnected()) this._browser._didDisconnect();
    } catch (error) { throw mapError(error, 'browserContext.close'); }
  }
}

class Page extends EventEmitter {
  constructor(context, descriptor) {
    super(); this._context = context; this._id = descriptor.id;
    const metadata = descriptor.mainFrameBinding ?? (descriptor.frameTree
      ? descriptor.frameTree.metadata(descriptor.mainFrameId) : descriptor.mainFrameMetadata);
    this._mainFrame = new Frame(this, descriptor.mainFrameId, metadata, null);
    this._frameCache = descriptor.frameTree ? new FrameCache(descriptor.frameTree, this._mainFrame,
      (id, node, parent) => new Frame(this, id, node, parent)) : null;
    this.keyboard = new Keyboard(context._browser._session, this._id);
    this.mouse = new Mouse(context._browser._session, this._id);
  }
  context() { return this._context; }
  isClosed() {
    if (!this._frameCache?._tree.isClosed) throw new UnsupportedOperationError('page.isClosed (rebuild native addon)');
    try { return this._frameCache._tree.isClosed(); }
    catch (error) { throw mapError(error, 'page.isClosed'); }
  }
  async setExtraHTTPHeaders(headers) {
    const session = this._context._browser._session;
    if (!session.setPageExtraHttpHeaders) throw new UnsupportedOperationError('page.setExtraHTTPHeaders (select extra-headers native profile)');
    const pending = session.setPageExtraHttpHeaders(this._id, headers, headerChannelError);
    try { await pending; }
    catch (error) { throw mapError(error, 'page.setExtraHTTPHeaders'); }
  }
  mainFrame() { return this._mainFrame; }
  setInputFiles(selector, files, options = {}) {
    return fileInput.forFrame(this._context._browser._session, this._mainFrame._id,
      selector, files, options, 'page.setInputFiles');
  }
  frame(selector) {
    const name = typeof selector === 'string' || selector instanceof String ? selector : selector.name;
    const url = typeof selector === 'object' && selector !== null ? selector.url : undefined;
    if (!this._frameCache || !this._frameCache._tree.lookup || !this._context._settings)
      throw new UnsupportedOperationError('page.frame (rebuild native addon)');
    // Do not map arbitrary user-thrown values through the HostError adapter.
    return this._frameCache.lookup(name, url, this._context._settings);
  }
  frames() {
    if (!this._frameCache) throw new UnsupportedOperationError('page.frames (rebuild native addon)');
    try { return this._frameCache.frames(); }
    catch (error) { throw mapError(error, 'page.frames'); }
  }
  async screenshot(options = {}) {
    const params = screenshotOptions(options);
    try { return await this._context._browser._session.pageScreenshot(this._id, params); }
    catch (error) { throw mapError(error, 'page.screenshot'); }
  }
  async goto(url, options = {}) {
    validateOptions(options, ['waitUntil', 'timeout', 'referer'], 'page.goto');
    if (typeof url !== 'string') throw new TypeError('page.goto: url must be a string');
    const timeout = options.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError('page.goto: timeout must be an integer in [0, 4294967295]');
    if (options.referer !== undefined && typeof options.referer !== 'string')
      throw new TypeError('page.goto: referer must be a string');
    if (options.waitUntil !== undefined && typeof options.waitUntil !== 'string')
      throw new TypeError('page.goto: waitUntil must be a string');
    try {
      if (this._context._settings) url = this._context._settings.resolveUrl(url);
      const result = await this._context._browser._session.pageGoto(this._id, url, options.waitUntil ?? 'load', timeout, options.referer);
      return result.response ? new Response(this, result.response) : null;
    } catch (error) { throw mapError(error, 'page.goto'); }
  }
  async title() { return this._mainFrame._read('frameTitle', 'page.title'); }
  async reload(options = {}) {
    validateOptions(options, ['waitUntil', 'timeout'], 'page.reload');
    const timeout = options.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError('page.reload: timeout must be an integer in [0, 4294967295]');
    if (options.waitUntil !== undefined && typeof options.waitUntil !== 'string')
      throw new TypeError('page.reload: waitUntil must be a string');
    try {
      const result = await this._context._browser._session.pageReload(this._id, options.waitUntil ?? 'load', timeout);
      return result.response ? new Response(this, result.response) : null;
    } catch (error) { throw mapError(error, 'page.reload'); }
  }
  async content() { return this._mainFrame._read('frameContent', 'page.content'); }
  url() { return this._mainFrame.url(); }
  async evaluate(pageFunction, arg) {
    if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
    return this._mainFrame._evaluate(pageFunction, arg, 'page.evaluate');
  }
  async evaluateHandle(pageFunction, arg) {
    if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
    return this._mainFrame._evaluateHandle(pageFunction, arg, 'page.evaluateHandle');
  }
  async close(options = {}) {
    validateOptions(options, ['runBeforeUnload'], 'page.close');
    if (options.runBeforeUnload !== undefined && typeof options.runBeforeUnload !== 'boolean')
      throw new TypeError('page.close: runBeforeUnload must be a boolean');
    if (options.runBeforeUnload) throw new UnsupportedOperationError('page.close runBeforeUnload');
    try {
      await this._context._browser._session.closePage(this._id);
      this._context._pages.delete(this._id);
    } catch (error) { throw mapError(error, 'page.close'); }
  }
  locator(selector, options = {}) { return this._mainFrame.locator(selector, options); }
  getByText(value, options) { return this._mainFrame.getByText(value, options); }
  getByLabel(value, options) { return this._mainFrame.getByLabel(value, options); }
  getByPlaceholder(value, options) { return this._mainFrame.getByPlaceholder(value, options); }
  getByAltText(value, options) { return this._mainFrame.getByAltText(value, options); }
  getByTitle(value, options) { return this._mainFrame.getByTitle(value, options); }
  getByTestId(value) { return this._mainFrame.getByTestId(value); }
  getByRole(value, options) { return this._mainFrame.getByRole(value, options); }
}

class Frame {
  constructor(page, id, metadata, parent) {
    this._page = page; this._id = id; this._metadata = metadata; this._parent = parent;
  }
  page() { return this._page; }
  setInputFiles(selector, files, options = {}) {
    return fileInput.forFrame(this._page._context._browser._session, this._id,
      selector, files, options, 'frame.setInputFiles');
  }
  async goto(url, options = {}) {
    validateOptions(options, ['waitUntil', 'timeout', 'referer'], 'frame.goto');
    if (typeof url !== 'string') throw new TypeError('frame.goto: url must be a string');
    const timeout = options.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError('frame.goto: timeout must be an integer in [0, 4294967295]');
    if (options.referer !== undefined && typeof options.referer !== 'string')
      throw new TypeError('frame.goto: referer must be a string');
    if (options.waitUntil !== undefined && typeof options.waitUntil !== 'string')
      throw new TypeError('frame.goto: waitUntil must be a string');
    const context = this._page._context;
    const session = context._browser._session;
    if (!session.frameGoto) throw new UnsupportedOperationError('frame.goto (select frame-navigation native profile)');
    try {
      if (context._settings) url = context._settings.resolveUrl(url);
      const result = await session.frameGoto([this._page._id, this._id], url, options.waitUntil ?? 'load', timeout, options.referer);
      return result.response ? new Response(this._page, result.response, this) : null;
    } catch (error) { throw mapError(error, 'frame.goto'); }
  }
  name() {
    if (!this._metadata.name) throw new UnsupportedOperationError('frame.name (rebuild native addon)');
    try { return this._metadata.name(); }
    catch (error) { throw mapError(error, 'frame.name'); }
  }
  parentFrame() { return this._parent; }
  childFrames() { return this._page.frames().filter(frame => frame.parentFrame() === this); }
  isDetached() {
    if (!this._metadata.isDetached) throw new UnsupportedOperationError('frame.isDetached (rebuild native addon)');
    return this._metadata.isDetached();
  }
  locator(selector, options = {}) { return locatorApi.create(this, selector, options); }
  getByText(value, options) { return locatorApi.semantic(this, 'text', value, options); }
  getByLabel(value, options) { return locatorApi.semantic(this, 'label', value, options); }
  getByPlaceholder(value, options) { return locatorApi.semantic(this, 'placeholder', value, options); }
  getByAltText(value, options) { return locatorApi.semantic(this, 'altText', value, options); }
  getByTitle(value, options) { return locatorApi.semantic(this, 'title', value, options); }
  getByTestId(value) { return locatorApi.semantic(this, 'testId', value); }
  getByRole(value, options) { return locatorApi.role(this, value, options); }
  url() { return this._metadata.url(); }
  pwrightMetadataMetrics() { return { unpublishedUrl: this._metadata.hasUnpublishedUrl() }; }
  async _read(method, api) {
    try { return await this._page._context._browser._session[method](this._id); }
    catch (error) { throw mapError(error, api); }
  }
  async title() { return this._read('frameTitle', 'frame.title'); }
  async content() { return this._read('frameContent', 'frame.content'); }
  async evaluate(pageFunction, arg) {
    if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
    return this._evaluate(pageFunction, arg, 'frame.evaluate');
  }
  async evaluateHandle(pageFunction, arg) {
    if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
    return this._evaluateHandle(pageFunction, arg, 'frame.evaluateHandle');
  }
  async _evaluateHandle(pageFunction, arg, api) {
    if (typeof pageFunction !== 'string' && typeof pageFunction !== 'function')
      throw new TypeError(`${api}: pageFunction must be a string or function`);
    const context = this._page._context;
    const session = context._browser._session;
    const scoped = typeof session.frameEvaluateHandleWithPageTimeout === 'function';
    if (!scoped && typeof session.frameEvaluateHandle !== 'function') throw new UnsupportedOperationError(api);
    try {
      // Forward identity only. Rust selects canonical Page policy and orders
      // any lookup failure after normal argument materialization.
      const native = scoped
        ? await session.frameEvaluateHandleWithPageTimeout(this._id, String(pageFunction), typeof pageFunction === 'function', arg, this._page._id)
        : await session.frameEvaluateHandle(this._id, String(pageFunction), typeof pageFunction === 'function', arg);
      return await handles.create(session, native, id => context._resolveHandleFrame(id));
    } catch (error) { throw mapError(error, api); }
  }
  async _evaluate(pageFunction, arg, api) {
    if (typeof pageFunction !== 'string' && typeof pageFunction !== 'function')
      throw new TypeError(`${api}: pageFunction must be a string or function`);
    try {
      return await this._page._context._browser._session.frameEvaluate(this._id, String(pageFunction), typeof pageFunction === 'function', arg);
    } catch (error) { throw mapError(error, api); }
  }
}

class Response {
  constructor(page, metadata, frame = page.mainFrame()) { this._page = page; this._metadata = metadata; this._frame = frame; }
  url() { return this._metadata.url; }
  status() { return this._metadata.status; }
  statusText() { return this._metadata.statusText; }
  ok() { return this._metadata.ok; }
  headers() { return { ...this._metadata.headers }; }
  frame() { return this._frame; }
  async body() { throw new UnsupportedOperationError('response.body'); }
  async text() { throw new UnsupportedOperationError('response.text'); }
  async json() { throw new UnsupportedOperationError('response.json'); }
  async finished() { throw new UnsupportedOperationError('response.finished'); }
}

const chromium = new BrowserType();
module.exports = { chromium, errors: Object.freeze({ TimeoutError, UnsupportedOperationError }), compatibility };
