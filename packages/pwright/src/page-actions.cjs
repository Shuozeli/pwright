'use strict';

// Playwright-shaped argument validation only. Rust owns document replacement,
// lifecycle waiting, timeout enforcement, selector resolution and input.
module.exports = function pageActions({ mapError, validateOptions, UnsupportedOperationError, locatorAction }) {
  const invalidFillTarget = 'Element is not an <input>, <textarea> or [contenteditable] element';
  async function setContent(session, pageId, html, options = {}) {
    const api = 'page.setContent';
    validateOptions(options, ['waitUntil', 'timeout'], api);
    if (typeof html !== 'string') throw new TypeError(`${api}: html must be a string`);
    const timeout = options.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    const waitUntil = options.waitUntil ?? 'load';
    if (typeof waitUntil !== 'string') throw new TypeError(`${api}: waitUntil must be a string`);
    if (typeof session.pageSetContent !== 'function')
      throw new UnsupportedOperationError(`${api} (select set-content native profile)`);
    try {
      await session.pageSetContent(pageId, html, waitUntil, timeout);
    } catch (error) { throw mapError(error, api); }
  }

  function press(frame, selector, key, options = {}, api = 'page.press') {
    return locatorAction(frame.locator(selector), 'press', options, key, api);
  }

  function type(frame, selector, text, options = {}, api = 'page.type') {
    return locatorAction(frame.locator(selector), 'type', options, text, api);
  }

  async function fill(frame, selector, value, options = {}, api = 'page.fill') {
    try {
      return await locatorAction(frame.locator(selector), 'fill', options, value, api);
    } catch (error) {
      // Only this exact native invalid-target message has the stackless Playwright
      // presentation; preserve every other backend error unchanged.
      if (error.code === 'PWRIGHT_BACKEND'
          && error.message === `${api}: Backend: ${invalidFillTarget}`) {
        const escapedSelector = selector.replaceAll('\\', '\\\\').replaceAll("'", "\\'");
        error.message = `${api}: Error: ${invalidFillTarget}\nCall log:\n  - waiting for locator('${escapedSelector}')`;
      }
      throw error;
    }
  }

  function focus(frame, selector, options = {}, api = 'page.focus') {
    return locatorAction(frame.locator(selector), 'focus', options, undefined, api);
  }

  async function pointerAction(frame, selector, operation, options, api) {
    const supported = [
      'timeout', 'strict', 'button', 'delay', 'force', 'modifiers',
      'noWaitAfter', 'position', 'trial',
    ];
    if (operation === 'click') supported.push('clickCount');
    validateOptions(options, supported, api);
    if (options.strict !== undefined && typeof options.strict !== 'boolean')
      throw new TypeError(`${api}: strict must be a boolean`);
    const { strict, ...locatorOptions } = options;
    const locator = frame.locator(selector);
    // Selector actions choose the first match unless strict is explicitly on;
    // Locator actions themselves remain strict and Rust owns the action.
    return locatorAction(strict ? locator : locator.first(), operation, locatorOptions, undefined, api);
  }

  function click(frame, selector, options = {}, api = 'page.click') {
    return pointerAction(frame, selector, 'click', options, api);
  }

  function dblclick(frame, selector, options = {}, api = 'page.dblclick') {
    return pointerAction(frame, selector, 'dblclick', options, api);
  }

  function tap(frame, selector, options = {}, api = 'page.tap') {
    return locatorAction(frame.locator(selector), 'tap', options, undefined, api);
  }

  function checked(frame, selector, operation, options, value, api) {
    validateOptions(options, ['timeout', 'force', 'position', 'trial', 'noWaitAfter', 'strict'], api);
    if (options.strict !== undefined && typeof options.strict !== 'boolean')
      throw new TypeError(`${api}: strict must be a boolean`);
    const { strict, ...locatorOptions } = options;
    // Selector-based Page actions match the first element unless strict is set.
    // The Locator/Rust path still owns resolution, waiting and input.
    const locator = frame.locator(selector);
    return locatorAction(strict ? locator : locator.first(), operation, locatorOptions, value, api);
  }

  async function check(frame, selector, options = {}, api = 'page.check') {
    return checked(frame, selector, 'check', options, undefined, api);
  }

  async function uncheck(frame, selector, options = {}, api = 'page.uncheck') {
    return checked(frame, selector, 'uncheck', options, undefined, api);
  }

  async function setChecked(frame, selector, value, options = {}, api = 'page.setChecked') {
    return checked(frame, selector, 'setChecked', options, value, api);
  }

  async function dispatchEvent(frame, selector, type, eventInit, options = {}, api = 'page.dispatchEvent') {
    validateOptions(options, ['timeout', 'strict'], api);
    if (options.strict !== undefined && typeof options.strict !== 'boolean')
      throw new TypeError(`${api}: strict must be a boolean`);
    if (typeof type !== 'string') throw new TypeError(`${api}: type must be a string`);
    const { strict, ...locatorOptions } = options;
    const locator = frame.locator(selector);
    return (strict ? locator : locator.first()).dispatchEvent(type, eventInit, locatorOptions);
  }

  return { setContent, press, type, fill, focus, click, dblclick, tap, check, uncheck, setChecked, dispatchEvent };
};
