'use strict';

// Playwright-shaped language validation only. Rust owns current-document
// lifecycle state, ordered observation, timeout enforcement and Page closure.
module.exports = function loadState({ mapError, validateOptions, UnsupportedOperationError }) {
  const states = ['load', 'domcontentloaded', 'networkidle', 'commit'];

  async function wait(frame, state = 'load', options = {}, api = 'frame.waitForLoadState', apiStack) {
    validateOptions(options, ['timeout'], api);
    if (typeof state !== 'string' || !states.includes(state))
      throw new TypeError(`${api}: state: expected one of (load|domcontentloaded|networkidle|commit)`);
    const page = frame._page;
    const session = page._context._browser._session;
    if (typeof session.frameWaitForLoadState !== 'function')
      throw new UnsupportedOperationError(`${api} (select frame-navigation native profile)`);
    const timeout = options.timeout ?? (typeof session.pageDefaultTimeout === 'function'
      ? session.pageDefaultTimeout(page._id, true)
      : 30_000);
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    try {
      await session.frameWaitForLoadState(page._id, frame._id, state, timeout);
    } catch (error) {
      const mapped = mapError(error, api);
      if (mapped?.code === 'PWRIGHT_TIMEOUT' && !/[.!?]$/.test(mapped.message))
        mapped.message += '.';
      if (apiStack?.stack) {
        const callers = apiStack.stack.slice(apiStack.stack.indexOf('\n'));
        mapped.stack = `${mapped.name}: ${mapped.message}${callers}`;
      }
      throw mapped;
    }
  }

  return { wait };
};
