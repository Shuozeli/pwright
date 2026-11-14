'use strict';

// Public numeric validation only; Page input state, event ordering and native
// retirement are owned by the selected Rust capability.
module.exports = function touchscreenApi({ mapError, UnsupportedOperationError }) {
  return class Touchscreen {
    #session;
    #page;
    constructor(session, page) { this.#session = session; this.#page = page; }
    async tap(x, y) {
      if (!Number.isFinite(x) || !Number.isFinite(y))
        throw new TypeError('touchscreen.tap: coordinates must be finite numbers');
      if (typeof this.#session.touchscreenTap !== 'function')
        throw new UnsupportedOperationError('touchscreen.tap requires the touchscreen-input native capability');
      try { await this.#session.touchscreenTap(this.#page, x, y); }
      catch (error) { throw mapError(error, 'touchscreen.tap'); }
    }
  };
};
