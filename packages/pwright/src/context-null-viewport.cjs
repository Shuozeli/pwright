'use strict';

// Option conflicts are observable before Context admission. Null-viewport
// execution itself remains outside the currently selected native profiles.
module.exports = ({ UnsupportedOperationError }) => ({
  validate(options) {
    const api = 'browser.newContext';
    if (options.noViewport !== undefined && typeof options.noViewport !== 'boolean')
      throw new TypeError(`${api}: noViewport must be a boolean`);
    if (options.noViewport && options.deviceScaleFactor !== undefined)
      throw new Error(`${api}: "deviceScaleFactor" option is not supported with null "viewport"`);
    if (options.noViewport && options.isMobile === true)
      throw new Error(`${api}: "isMobile" option is not supported with null "viewport"`);
    if (options.noViewport)
      throw new UnsupportedOperationError(`${api} noViewport=true (select null-viewport native profile)`);
    if (options.deviceScaleFactor !== undefined)
      throw new UnsupportedOperationError(`${api} deviceScaleFactor (select device-emulation native profile)`);
  },
});
