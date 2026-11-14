'use strict';

module.exports = ({ mapError, UnsupportedOperationError }) => Object.freeze({
  async add(context, cookies) {
    const session = context.browser()._session;
    if (typeof session.contextAddCookies !== 'function')
      throw new UnsupportedOperationError('browserContext.addCookies (select context-cookies native profile)');
    try { await session.contextAddCookies(context._id, cookies); }
    catch (error) { throw mapError(error, 'browserContext.addCookies'); }
  },
});
