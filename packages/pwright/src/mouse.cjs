'use strict';

// Shape and defaults only; coordinates, button state and scheduling live in Rust.
module.exports = function mouseApi({ mapError, validateOptions }) {
  return class Mouse {
    #session;
    #page;
    constructor(session, page) { this.#session = session; this.#page = page; }
    async #call(operation, x, y, options, supported) {
      const api = `mouse.${operation}`;
      validateOptions(options, supported, api);
      if (!Number.isFinite(x) || !Number.isFinite(y))
        throw new TypeError(`${api}: coordinates and deltas must be finite numbers`);
      const button = options.button ?? 'left';
      if (!['left', 'right', 'middle'].includes(button))
        throw new TypeError(`${api}: button must be left, right or middle`);
      const native = {
        button: ['left', 'right', 'middle'].indexOf(button),
        steps: options.steps ?? 1,
        clickCount: options.clickCount ?? 1,
        delayMs: options.delay ?? 0,
      };
      for (const [key, value] of Object.entries(native)) {
        if (typeof value !== 'number') throw new TypeError(`${api}: ${key} must be a number`);
      }
      try { await this.#session.mouse(this.#page, operation, x, y, native); }
      catch (error) { throw mapError(error, api); }
    }
    move(x, y, options = {}) { return this.#call('move', x, y, options, ['steps']); }
    down(options = {}) { return this.#call('down', 0, 0, options, ['button', 'clickCount']); }
    up(options = {}) { return this.#call('up', 0, 0, options, ['button', 'clickCount']); }
    click(x, y, options = {}) { return this.#call('click', x, y, options, ['button', 'clickCount', 'delay']); }
    dblclick(x, y, options = {}) { return this.#call('dblclick', x, y, options, ['button', 'delay']); }
    wheel(deltaX, deltaY) { return this.#call('wheel', deltaX, deltaY, {}, []); }
  };
};
