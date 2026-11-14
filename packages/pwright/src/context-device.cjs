'use strict';

// Public option validation and capability selection only. Rust owns device
// option retention, Context installation, CDP emulation and teardown.
module.exports = ({ UnsupportedOperationError }) => ({
  validate(options) {
    if (options.hasTouch !== undefined && typeof options.hasTouch !== 'boolean')
      throw new TypeError('browser.newContext: hasTouch must be a boolean');
    if (options.hasTouch && (options.bypassCSP || options.extraHTTPHeaders !== undefined || options.proxy !== undefined))
      throw new UnsupportedOperationError('browser.newContext hasTouch=true with bypassCSP, extraHTTPHeaders or proxy');
  },

  create(session, options, baseURL) {
    if (!options.hasTouch) return undefined;
    if (!session.newContextWithHasTouch)
      throw new UnsupportedOperationError('browser.newContext hasTouch=true (select device-emulation native profile)');
    return session.newContextWithHasTouch(baseURL, true);
  },
});
