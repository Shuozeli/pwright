'use strict';

const ConsoleMessage = require('./console-message.cjs');

// Rust selects the event and commits membership/closed state. This adapter
// only prepares wrapper identity and returns the EventEmitter invocation.
// Creation factories receive native descriptors, never browser commands.
module.exports = function lifecycleEvents(browserRef, create, frameEventsEnabled = false) {
  return (kind, contextId, pageId, openerId, messageType, text, argumentsJSON, url, lineNumber, columnNumber,
    nativeDialog, dialogType, dialogMessage, dialogDefaultValue) => {
    if (![0, 1, 2, 3, 4, 5, 6, 7, 9, 10].includes(kind)
        && !(frameEventsEnabled && (kind === 11 || kind === 12)))
      throw new TypeError('Unknown native lifecycle notice');
    const browser = browserRef.deref();
    if (!browser) return undefined;
    const context = browser._contexts.get(contextId);
    if (kind === 2) {
      const page = context?._pages.get(pageId);
      return {
        contextNeeded: !context,
        pageNeeded: !page,
        prepare(contextDescriptor, pageDescriptor) {
          // No membership/cache mutation until Rust commits the notice.
          const preparedContext = context ?? create.context(browser, contextDescriptor);
          const preparedPage = page ?? create.page(preparedContext, pageDescriptor);
          return () => {
            browser._contexts.set(contextId, preparedContext);
            preparedContext._pages.set(pageId, preparedPage);
            preparedContext.emit('page', preparedPage);
          };
        },
      };
    }
    if (!context) return undefined;
    if (kind === 3) {
      const opener = context._pages.get(openerId);
      if (!opener) return undefined; // No acquired emitter means no listeners.
      const popup = context._pages.get(pageId);
      if (!popup) throw new TypeError('Native popup notice precedes Page identity');
      return () => {
        popup._opener = opener;
        opener.emit('popup', popup);
      };
    }
    if (kind === 4 || kind === 5) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      const event = kind === 4 ? 'domcontentloaded' : 'load';
      return () => page.emit(event, page);
    }
    if (kind === 7) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      return () => page.emit('framenavigated', page.mainFrame());
    }
    if (kind === 11 || kind === 12) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      if (typeof openerId !== 'string' || !page._frameCache?.forEvent)
        throw new TypeError('Native Frame event notice is missing Frame identity');
      const event = kind === 11 ? 'frameattached' : 'framenavigated';
      if (page.listenerCount(event) === 0) return undefined;
      return () => page.emit(event, page._frameCache.forEvent(openerId));
    }
    if (kind === 10) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      if (typeof openerId !== 'string' || !page._frameCache?.detached)
        throw new TypeError('Native Frame detach notice is missing Frame identity');
      return () => {
        try {
          const frame = page._frameCache.detached(openerId);
          page.emit('framedetached', frame);
        }
        finally { page._frameCache.finishDetached(openerId); }
      };
    }
    if (kind === 6) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      if (typeof messageType !== 'string' || typeof text !== 'string')
        throw new TypeError('Native console notice is missing message data');
      if (typeof argumentsJSON !== 'string' || typeof url !== 'string'
          || !Number.isInteger(lineNumber) || !Number.isInteger(columnNumber))
        throw new TypeError('Native console notice is missing argument or location data');
      const message = new ConsoleMessage(page, messageType, text, argumentsJSON, {
        url, lineNumber, columnNumber,
      });
      return () => {
        page._consoleMessages.push(message);
        if (page._consoleMessages.length > 200) page._consoleMessages.splice(0, page._consoleMessages.length - 200);
        page.emit('console', message);
        context.emit('console', message);
      };
    }
    if (kind === 9) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      if (!nativeDialog || typeof dialogType !== 'string' || typeof dialogMessage !== 'string'
          || typeof dialogDefaultValue !== 'string' || typeof create?.dialog !== 'function')
        throw new TypeError('Native dialog notice is missing dialog data');
      const dialog = create.dialog(page, nativeDialog, dialogType, dialogMessage, dialogDefaultValue);
      const hasSubscribers = page.listenerCount('dialog') > 0 || context.listenerCount('dialog') > 0;
      return () => {
        page.emit('dialog', dialog);
        context.emit('dialog', dialog);
        if (!hasSubscribers) void dialog._autoHandle();
      };
    }
    if (kind === 0) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      return () => {
        context._pages.delete(pageId);
        context._retireCdpSessions?.(pageId);
        page._retireNetworkRequestCache();
        page.emit('close', page);
      };
    }
    return () => {
      context._retireAllCdpSessions?.();
      for (const page of context._pages.values()) page._retireNetworkRequestCache();
      browser._contexts.delete(contextId);
      if (browser._freshOwned && typeof context._emitCloseOnce === 'function') context._emitCloseOnce();
      else context.emit('close', context);
    };
  };
};
