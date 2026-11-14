'use strict';

// Public option validation and capability selection only. Rust owns policy
// retention, Context creation, per-session installation and teardown.
module.exports = ({ UnsupportedOperationError }) => ({
  validate(options) {
    if (options.bypassCSP !== undefined && typeof options.bypassCSP !== 'boolean')
      throw new TypeError('browser.newContext: bypassCSP must be a boolean');
    if (options.offline !== undefined && typeof options.offline !== 'boolean')
      throw new TypeError('browser.newContext: offline must be a boolean');
    if (options.offline && options.hasTouch)
      throw new UnsupportedOperationError('browser.newContext offline with hasTouch=true');
  },

  create(session, options, baseURL) {
    if (options.bypassCSP && options.offline)
      throw new UnsupportedOperationError('browser.newContext bypassCSP with offline');
    if (options.bypassCSP) {
      if (options.extraHTTPHeaders !== undefined || options.proxy !== undefined)
        throw new UnsupportedOperationError('browser.newContext bypassCSP with extraHTTPHeaders or proxy');
      if (!session.newContextWithBypassCsp)
        throw new UnsupportedOperationError('browser.newContext bypassCSP (select bypass-csp native profile)');
      return session.newContextWithBypassCsp(baseURL, true);
    }
    if (options.offline) {
      if (options.extraHTTPHeaders !== undefined || options.proxy !== undefined)
        throw new UnsupportedOperationError('browser.newContext offline with extraHTTPHeaders or proxy');
      if (!session.newContextWithOffline)
        throw new UnsupportedOperationError('browser.newContext offline (select context-offline native profile)');
      return session.newContextWithOffline(baseURL, true);
    }
    return undefined;
  },
});
