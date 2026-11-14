'use strict';

// Playwright-shaped URL matching only. Rust owns URL observation, lifecycle
// ordering, timeout enforcement, Page/Frame closure and bounded concurrency.
module.exports = function waitForUrlAdapter({ mapError, validateOptions, UnsupportedOperationError }) {
  const states = ['load', 'domcontentloaded', 'networkidle', 'commit'];
  const isRegExp = value => value instanceof RegExp
    || Object.prototype.toString.call(value) === '[object RegExp]';

  function prepareMatcher(session, pageId, expected, api) {
    if (typeof expected === 'string') {
      if (expected === '') return () => true;
      if (typeof session.pageUrlGlob !== 'function')
        throw new UnsupportedOperationError(`${api} (select wait-for-url native profile)`);
      let pattern;
      try { pattern = session.pageUrlGlob(pageId, expected); }
      catch (error) { throw mapError(error, api); }
      const regex = new RegExp(pattern);
      return candidate => regex.test(candidate);
    }
    if (isRegExp(expected)) return candidate => expected.test(candidate);
    if (typeof expected === 'function') {
      return candidate => {
        let parsed;
        try { parsed = new URL(candidate); }
        catch (_) { return false; }
        return Boolean(expected(parsed));
      };
    }
    throw new TypeError(`${api}: url must be a string, RegExp, or function`);
  }

  async function wait(frame, expected, options = {}, api = 'frame.waitForURL') {
    validateOptions(options, ['waitUntil', 'timeout'], api);
    const waitUntil = options.waitUntil ?? 'load';
    if (typeof waitUntil !== 'string' || !states.includes(waitUntil))
      throw new TypeError(`${api}: waitUntil: expected one of (load|domcontentloaded|networkidle|commit)`);
    const page = frame._page;
    const session = page._context._browser._session;
    if (typeof session.frameWaitForUrl !== 'function')
      throw new UnsupportedOperationError(`${api} (select wait-for-url native profile)`);
    const timeout = options.timeout ?? (typeof session.pageDefaultTimeout === 'function'
      ? session.pageDefaultTimeout(page._id, true)
      : 30_000);
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    const matcher = prepareMatcher(session, page._id, expected, api);
    try {
      await session.frameWaitForUrl(page._id, frame._id, matcher, waitUntil, timeout);
    } catch (error) {
      const mapped = mapError(error, api);
      if (mapped?.code === 'PWRIGHT_TIMEOUT' && !/[.!?]$/.test(mapped.message))
        mapped.message += '.';
      throw mapped;
    }
  }

  return { prepareMatcher, wait };
};
