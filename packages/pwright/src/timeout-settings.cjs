'use strict';

// Language validation only. Rust owns fixed Context/Page state, inheritance,
// lifecycle reconciliation and effective timeout selection.
module.exports = function timeoutSettings({ mapError, UnsupportedOperationError }) {
  function validate(timeout, api) {
    if (timeout === undefined) return;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
  }

  function setPage(page, timeout, navigation, api) {
    validate(timeout, api);
    const session = page._context._browser._session;
    if (typeof session.setPageDefaultTimeout !== 'function')
      throw new UnsupportedOperationError(`${api} (select timeout-settings native profile)`);
    try {
      session.setPageDefaultTimeout(page._id, navigation, timeout);
    } catch (error) {
      throw mapError(error, api);
    }
  }

  function setContext(context, timeout, navigation, api) {
    validate(timeout, api);
    const session = context._browser._session;
    if (typeof session.setContextDefaultTimeout !== 'function')
      throw new UnsupportedOperationError(`${api} (select timeout-settings native profile)`);
    try {
      session.setContextDefaultTimeout(context._id, navigation, timeout);
    } catch (error) {
      throw mapError(error, api);
    }
  }

  return { setPage, setContext };
};
