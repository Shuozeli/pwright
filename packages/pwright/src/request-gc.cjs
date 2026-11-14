'use strict';

// Thin Playwright-shaped facade. Page/session lookup and the exact Chromium
// command remain in the optional Rust leaf.
module.exports = function requestGcAdapter({ mapError, UnsupportedOperationError }) {
  return async function requestGC(session, pageId) {
    if (typeof session.pageRequestGc !== 'function')
      throw new UnsupportedOperationError('page.requestGC (select request-gc native profile)');
    try { await session.pageRequestGc(pageId); }
    catch (error) { throw mapError(error, 'page.requestGC'); }
  };
};
