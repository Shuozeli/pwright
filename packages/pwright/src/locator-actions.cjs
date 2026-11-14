'use strict';

// Public argument shapes only. Native code constructs typed action requests;
// this file performs no selector resolution, input, waiting or navigation.
module.exports = function locatorActions({ mapError, validateOptions, handleKind = () => null }) {
  const fields = {
    click: ['button', 'clickCount', 'delay', 'force', 'modifiers', 'noWaitAfter', 'position', 'steps', 'trial'],
    dblclick: ['button', 'delay', 'force', 'modifiers', 'noWaitAfter', 'position', 'trial'],
    tap: ['force', 'modifiers', 'noWaitAfter', 'position', 'trial'],
    fill: ['force', 'noWaitAfter'], clear: ['force', 'noWaitAfter'],
    press: ['delay', 'noWaitAfter'],
    type: ['delay', 'noWaitAfter'], pressSequentially: ['delay', 'noWaitAfter'],
    check: ['force', 'position', 'trial', 'noWaitAfter'],
    uncheck: ['force', 'position', 'trial', 'noWaitAfter'],
    setChecked: ['force', 'position', 'trial', 'noWaitAfter'],
    selectOption: ['force', 'noWaitAfter'],
    hover: ['force', 'modifiers', 'position', 'trial', 'noWaitAfter'],
    focus: [], blur: [], scrollIntoViewIfNeeded: [],
  };
  return async function act(locator, operation, options, value, publicApi = `locator.${operation}`) {
    const api = publicApi;
    validateOptions(options, ['timeout', ...fields[operation]], api);
    const snapshot = {};
    for (const field of ['timeout', ...fields[operation]]) {
      // Keep the existing option shape when the opt-in click path is not used.
      if (field === 'steps' && options[field] === undefined) continue;
      snapshot[field] = options[field];
    }
    const timeout = snapshot.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    if (snapshot.position !== undefined) validateOptions(snapshot.position, ['x', 'y'], `${api} position`);
    if (operation === 'click' && snapshot.steps !== undefined &&
        (!Number.isInteger(snapshot.steps) || snapshot.steps < 1 || snapshot.steps > 10_000))
      throw new TypeError(`${api}: steps must be an integer in [1, 10000]`);
    if (operation === 'fill') {
      // Playwright's protocol accepts boxed strings but rejects other values
      // before an action reaches the browser-owned input transaction.
      if (value instanceof String) value = value.valueOf();
      if (typeof value !== 'string')
        throw new TypeError(`${api}: value: expected string, got ${typeof value}`);
    }
    if (operation === 'selectOption') {
      const descriptors = Array.isArray(value) ? value : value === null ? [] : [value];
      if (descriptors.length > 1024) throw new TypeError(`${api}: at most 1024 descriptors`);
      for (const [index, item] of descriptors.entries()) {
        if (typeof item === 'string') continue;
        const path = `options[${index}]`;
        const kind = handleKind(item);
        if (kind === 'element') {
          if (Array.isArray(value))
            throw new TypeError(`${api}: ${path}: ElementHandle arrays are not supported`);
          continue;
        }
        if (kind)
          throw new TypeError(`${api}: ${path}: expected ElementHandle, got ${kind === 'forged' ? 'unbound wrapper' : 'JSHandle'}`);
        if (item !== null && typeof item === 'object' &&
            Object.prototype.toString.call(item) === '[object PwrightJSHandle]')
          throw new TypeError(`${api}: ${path}: ElementHandle belongs to a different native module`);
        if (item === null || typeof item !== 'object')
          throw new TypeError(`${api}: ${path}: expected object, got ${item === null ? 'null' : typeof item}`);
        validateOptions(item, ['value', 'label', 'index'], `${api} descriptor`);
        for (const field of ['value', 'label']) {
          if (item[field] !== undefined && typeof item[field] !== 'string')
            throw new TypeError(`${api}: ${path}.${field}: expected string, got ${typeof item[field]}`);
        }
        if (item.index !== undefined) {
          if (typeof item.index !== 'number')
            throw new TypeError(`${api}: ${path}.index: expected integer, got ${typeof item.index}`);
          if (!Number.isInteger(item.index))
            throw new TypeError(`${api}: ${path}.index: expected integer, got float ${item.index}`);
        }
      }
    }
    try { return await locator.act(operation, timeout, snapshot, value); }
    catch (error) { throw mapError(error, api); }
  };
};
