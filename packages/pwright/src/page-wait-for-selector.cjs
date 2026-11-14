'use strict';

// Only public argument validation and the Page/Frame timeout snapshot live in
// JS. The selected native Locator owns one wait and optional Handle publication.
module.exports = function pageWaitForSelector({ validateOptions, mapError }) {
  async function wait(frame, selector, options = {}, api = 'page.waitForSelector') {
    validateOptions(options, ['state', 'strict', 'timeout', 'visibility', 'waitFor'], api);
    if (options.visibility)
      throw new Error('options.visibility is not supported, did you mean options.state?');
    if (options.waitFor && options.waitFor !== 'visible')
      throw new Error('options.waitFor is not supported, did you mean options.state?');
    if (typeof selector !== 'string') throw new TypeError(`${api}: selector must be a string`);
    const state = options.state ?? 'visible';
    if (!['attached', 'detached', 'visible', 'hidden'].includes(state))
      throw new TypeError(`${api}: state: expected one of (attached|detached|visible|hidden)`);
    const strict = options.strict ?? false;
    if (typeof strict !== 'boolean') throw new TypeError(`${api}: strict must be a boolean`);
    const page = frame.page();
    const session = page.context()._browser._session;
    let timeout;
    try {
      timeout = options.timeout ?? (typeof session.pageDefaultTimeout === 'function'
        ? session.pageDefaultTimeout(page._id, false)
        : 30_000);
    } catch (error) { throw mapError(error, api); }
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    const locator = frame.locator(selector);
    try {
      return await locator._waitForSelectorElement(state, timeout, strict, api);
    } catch (error) {
      let mapped = api.startsWith('frame.') && typeof frame._mapEvaluationError === 'function'
        ? frame._mapEvaluationError(error, api)
        : error;
      const lostContext = mapped?.code === 'PWRIGHT_BACKEND' && [
        'Execution context was destroyed',
        'uniqueContextId not found',
        'Cannot find context with specified id',
        'CDP error -32000: Inspected target navigated or closed',
      ].some(message => mapped.message.includes(message));
      if (api.startsWith('frame.') && lostContext) {
        let frameDetached = false;
        try { frameDetached = frame._metadata?.isDetached?.() === true; }
        catch (_) { /* An unavailable detachment signal cannot replace the original error. */ }
        if (frameDetached) {
          const detached = new Error(`${api}: Frame was detached`);
          detached.code = mapped.code;
          mapped = detached;
        }
      }
      if (['PWRIGHT_TIMEOUT', 'PWRIGHT_TARGET_CLOSED', 'PWRIGHT_BACKEND', 'PWRIGHT_JAVASCRIPT'].includes(mapped?.code)
        && !mapped.message.includes('waiting for locator(')) {
        const escaped = selector.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
        mapped.message += `\nCall log:\n  - waiting for locator('${escaped}')${state === 'attached' ? '' : ` to be ${state}`}`;
      }
      throw mapped;
    }
  }

  return { wait };
};
