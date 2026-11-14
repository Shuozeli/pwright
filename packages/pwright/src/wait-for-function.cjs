'use strict';

// Language overload/type adaptation only. Rust owns admission, one-shot poll
// attempts, deadline/cancellation, falsy disposal and the exact truthy Handle.
module.exports = function waitForFunction({
  mapError,
  validateOptions,
  UnsupportedOperationError,
  createHandle,
}) {
  async function wait(frame, pageFunction, arg, options = {}, api) {
    validateOptions(options, ['polling', 'timeout'], api);
    const source = String(pageFunction);
    const isFunction = typeof pageFunction === 'function';
    const timeout = options.timeout;
    if (timeout !== undefined && (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff))
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    const polling = options.polling ?? 'raf';
    if (typeof polling === 'string' && polling !== 'raf')
      throw new TypeError(`${api}: Unknown polling option: ${polling}`);
    const pollingMs = polling === 'raf' ? 0 : polling;
    if (polling !== 'raf' && typeof pollingMs === 'number' && pollingMs <= 0)
      throw new TypeError(`${api}: Cannot poll with non-positive interval: ${pollingMs}`);
    if (polling !== 'raf' && (!Number.isInteger(pollingMs) || pollingMs > 0xffff_ffff))
      throw new TypeError(`${api}: polling must be 'raf' or a positive integer in [1, 4294967295]`);
    const session = frame._page._context._browser._session;
    if (typeof session.frameWaitForFunction !== 'function')
      throw new UnsupportedOperationError(`${api} (select wait-for-function native profile)`);
    try {
      const native = await session.frameWaitForFunction(
        frame._id,
        frame._page._id,
        source,
        arg,
        { isFunction, pollingMs, timeoutMs: timeout },
      );
      return await createHandle(
        session,
        native,
        id => frame._page._context._resolveHandleFrame(id),
      );
    } catch (error) {
      throw mapError(error, api);
    }
  }

  return { wait };
};
