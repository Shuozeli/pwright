'use strict';

const pending = Symbol('pwright.pendingEventWaits');
const maximum = 32;

// EventEmitter plumbing only. Rust decides which browser events exist and
// publishes their Page identity; this adapter owns no browser observation.
module.exports = function waitForEventApi({ TimeoutError, validateOptions }) {
  function restoreApiStack(error, apiStack) {
    if (!apiStack?.stack) return error;
    const firstFrame = apiStack.stack.indexOf('\n');
    if (firstFrame !== -1)
      error.stack = `${error.name}: ${error.message}${apiStack.stack.slice(firstFrame)}`;
    return error;
  }

  function timeoutError(api, event, timeout, { apiStack, logLine } = {}) {
    let message = `${api}: Timeout ${timeout}ms exceeded while waiting for event "${event}"`;
    if (logLine) {
      const header = ' logs ';
      const width = 60;
      const left = (width - header.length) / 2;
      const right = width - header.length - left;
      message += `\n${'='.repeat(left)}${header}${'='.repeat(right)}\n${logLine}\n${'='.repeat(width)}`;
    }
    return restoreApiStack(new TimeoutError(message), apiStack);
  }

  return function waitForEvent(
    emitter,
    event,
    optionsOrPredicate = {},
    api = 'page.waitForEvent',
    defaultTimeout = 30_000,
    details = {},
  ) {
    if (typeof event !== 'string') return Promise.reject(new TypeError(`${api}: event must be a string`));
    const options = typeof optionsOrPredicate === 'function'
      ? { predicate: optionsOrPredicate }
      : optionsOrPredicate;
    try { validateOptions(options, ['predicate', 'timeout'], api); }
    catch (error) { return Promise.reject(error); }
    if (options.predicate !== undefined && typeof options.predicate !== 'function')
      return Promise.reject(new TypeError(`${api}: predicate must be a function`));
    const timeout = options.timeout ?? defaultTimeout;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      return Promise.reject(new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`));
    const count = emitter[pending] ?? 0;
    if (count >= maximum) return Promise.reject(new Error(`${api}: waiter capacity exceeds ${maximum}`));
    emitter[pending] = count + 1;
    const deadlineError = timeoutError(api, event, timeout, details);
    return new Promise((resolve, reject) => {
      let timer;
      let settled = false;
      const cleanup = () => {
        emitter.removeListener(event, listener);
        if (event !== 'close') emitter.removeListener('close', closed);
        if (timer) clearTimeout(timer);
        emitter[pending] -= 1;
      };
      const finish = (callback, value) => {
        if (settled) return;
        settled = true;
        cleanup();
        callback(value);
      };
      const listener = value => {
        Promise.resolve().then(() => options.predicate?.(value) ?? true).then(
          accepted => { if (accepted) finish(resolve, value); },
          error => finish(reject, error),
        );
      };
      const closed = () => finish(reject, new Error(`${api}: Page closed while waiting for ${event}`));
      emitter.on(event, listener);
      if (event !== 'close') emitter.on('close', closed);
      if (timeout) timer = setTimeout(() => finish(reject, deadlineError), timeout);
    });
  };
};
