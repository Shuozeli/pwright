'use strict';

// Rust selects the event and commits membership/closed state. This adapter
// only prepares wrapper identity and returns the EventEmitter invocation.
// Creation factories receive native descriptors, never browser commands.
module.exports = function lifecycleEvents(browserRef, create) {
  return (kind, contextId, pageId, openerId) => {
    if (![0, 1, 2, 3].includes(kind)) throw new TypeError('Unknown native lifecycle notice');
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
      return () => opener.emit('popup', popup);
    }
    if (kind === 0) {
      const page = context._pages.get(pageId);
      if (!page) return undefined;
      return () => {
        context._pages.delete(pageId);
        page.emit('close', page);
      };
    }
    return () => {
      browser._contexts.delete(contextId);
      context.emit('close', context);
    };
  };
};
