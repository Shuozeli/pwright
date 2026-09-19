'use strict';

// Public API shape only. The Page input state and every input action live in Rust.
module.exports = function keyboardApi({ mapError, validateOptions }) {
  return class Keyboard {
    #session;
    #page;
    constructor(session, page) { this.#session = session; this.#page = page; }
    async #call(operation, value, options) {
      const api = `keyboard.${operation}`;
      if (typeof value !== 'string') throw new TypeError(`${api}: input must be a string`);
      validateOptions(options, ['delay'], api);
      const delay = options.delay ?? 0;
      if (!Number.isFinite(delay) || delay < 0 || delay > 0xffff_ffff)
        throw new TypeError(`${api}: delay must be finite and between 0 and 4294967295 ms`);
      try { await this.#session.keyboard(this.#page, operation, value, delay); }
      catch (error) { throw mapError(error, api); }
    }
    down(key) { return this.#call('down', key, {}); }
    up(key) { return this.#call('up', key, {}); }
    press(key, options = {}) { return this.#call('press', key, options); }
    insertText(text) { return this.#call('insertText', text, {}); }
    type(text, options = {}) { return this.#call('type', text, options); }
  };
};
