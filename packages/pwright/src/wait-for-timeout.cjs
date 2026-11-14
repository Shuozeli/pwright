'use strict';

// Validation and API-name adaptation only. Rust owns the timer, cancellation,
// fixed admission and completion publication.
module.exports = function waitForTimeout({ mapError, UnsupportedOperationError }) {
  async function wait(frame, timeout, api) {
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    const session = frame._page._context._browser._session;
    if (typeof session.waitForTimeout !== 'function')
      throw new UnsupportedOperationError(`${api} (select wait-for-timeout native profile)`);
    try {
      await session.waitForTimeout(timeout);
    } catch (error) {
      throw mapError(error, api);
    }
  }

  return { wait };
};
