'use strict';

module.exports = function pageViewportApi({ mapError, validateOptions, UnsupportedOperationError }) {
  return {
    async set(session, pageId, viewport) {
      const api = 'page.setViewportSize';
      validateOptions(viewport, ['width', 'height'], api);
      const { width, height } = viewport;
      if (!Number.isInteger(width) || !Number.isInteger(height))
        throw new TypeError(`${api}: width and height must be integers`);
      if (width < 1 || height < 1 || width > 10_000_000 || height > 10_000_000)
        throw new RangeError(`${api}: width and height must be between 1 and 10000000`);
      if (typeof session.pageSetViewportSize !== 'function')
        throw new UnsupportedOperationError(`${api} (select page-viewport native profile)`);
      try { await session.pageSetViewportSize(pageId, width, height); }
      catch (error) { throw mapError(error, api); }
    },
  };
};
