'use strict';

// Public argument shapes only. Native code constructs typed action requests;
// this file performs no selector resolution, input, waiting or navigation.
module.exports = function locatorActions({ mapError, validateOptions }) {
  const fields = {
    click: ['button', 'clickCount', 'delay', 'force', 'modifiers', 'noWaitAfter', 'position', 'trial'],
    dblclick: ['button', 'delay', 'force', 'modifiers', 'noWaitAfter', 'position', 'trial'],
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
  return async function act(locator, operation, options, value) {
    const api = `locator.${operation}`;
    validateOptions(options, ['timeout', ...fields[operation]], api);
    const snapshot = {};
    for (const field of ['timeout', ...fields[operation]]) snapshot[field] = options[field];
    const timeout = snapshot.timeout ?? 30_000;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    if (snapshot.position !== undefined) validateOptions(snapshot.position, ['x', 'y'], `${api} position`);
    if (operation === 'selectOption') {
      const descriptors = Array.isArray(value) ? value : value === null ? [] : [value];
      if (descriptors.length > 1024) throw new TypeError(`${api}: at most 1024 descriptors`);
      for (const item of descriptors) {
        if (typeof item !== 'string') validateOptions(item, ['value', 'label', 'index'], `${api} descriptor`);
      }
    }
    try { return await locator.act(operation, timeout, snapshot, value); }
    catch (error) { throw mapError(error, api); }
  };
};
