'use strict';

// Rust owns response observation, bounded queues and URL-glob compilation.
// This adapter only publishes immutable snapshots and evaluates public
// JavaScript matchers on the Page EventEmitter.
module.exports = function responseEvents({
  createResponse,
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
    if (typeof expected === 'string') return `waiting for response "${trimWithEllipsis(expected)}"`;
    if (isRegExp(expected))
      return `waiting for response /${trimWithEllipsis(expected.source)}/${expected.flags}`;
    return undefined;
  }

  function prepareMatcher(page, expected, api) {
    if (typeof expected === 'string') {
      const session = page._context._browser._session;
      if (typeof session.pageUrlGlob !== 'function')
        throw new UnsupportedOperationError(`${api} (select response-events native profile)`);
      let pattern;
      try { pattern = session.pageUrlGlob(page._id, expected); }
      catch (error) { throw mapError(error, api); }
      const regex = new RegExp(pattern);
      return response => regex.test(response.url());
    }
    if (isRegExp(expected)) return response => expected.test(response.url());
    if (typeof expected === 'function') return expected;
    throw new TypeError(`${api}: urlOrPredicate must be a string, RegExp, or function`);
  }

  function wait(page, expected, options = {}, apiStack) {
    const api = 'page.waitForResponse';
    try {
      validateOptions(options, ['timeout'], api);
      const session = page._context._browser._session;
      if (typeof session.nextResponseNotice !== 'function')
        throw new UnsupportedOperationError(`${api} (select response-events native profile)`);
      const timeout = options.timeout ?? (typeof session.pageDefaultTimeout === 'function'
        ? session.pageDefaultTimeout(page._id, false)
        : 30_000);
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      const predicate = prepareMatcher(page, expected, api);
      return waitForEvent(
        page,
        'response',
        { predicate, timeout },
        api,
        timeout,
        { apiStack, logLine: matcherLog(expected) },
      );
    } catch (error) {
      return Promise.reject(error);
    }
  }

  function fromNotice(page, notice, request) {
    return createResponse(page, {
      ...notice.response,
      nativeRequestId: notice.nativeRequestId,
      finishedOwner: notice.finishedOwner,
      historyOwner: notice.historyOwner,
    }, undefined, request);
  }

  function dispatch(browser, notice) {
    const page = [...browser._contexts.values()]
      .map(context => context._pages.get(notice.pageId))
      .find(Boolean);
    if (!page) return;
    const response = fromNotice(page, notice);
    page._context.emit('response', response);
    page.emit('response', response);
  }

  function watch(session, browserRef) {
    if (typeof session.nextResponseNotice !== 'function') return;
    const next = () => {
      Promise.resolve().then(() => session.nextResponseNotice()).then(notice => {
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

  return { dispatch, fromNotice, prepareMatcher, wait, watch };
};
