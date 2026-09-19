'use strict';
const { isBuffer } = require('node:buffer').Buffer;

// Public shape/feature selection only. Rust owns Buffer conversion, filesystem
// preparation, MIME, selector resolution, timing, mutation and cleanup.
module.exports = function fileInputApi({ mapError, validateOptions, UnsupportedOperationError }) {
  function readOptions(options, api, selector) {
    validateOptions(options, selector ? ['timeout', 'noWaitAfter', 'strict'] : ['timeout', 'noWaitAfter'], api);
    const timeout = options.timeout ?? 30_000;
    const noWaitAfter = options.noWaitAfter;
    if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
      throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
    if (noWaitAfter !== undefined && typeof noWaitAfter !== 'boolean')
      throw new TypeError(`${api}: noWaitAfter must be a boolean`);
    if (selector && options.strict !== undefined && typeof options.strict !== 'boolean')
      throw new TypeError(`${api}: strict must be a boolean`);
    return timeout;
  }
  async function setInputFiles(locator, files, options = {}) {
    const api = 'locator.setInputFiles';
    const timeout = readOptions(options, api, false);
    if (typeof locator.setInputFiles !== 'function')
      throw new UnsupportedOperationError(`${api} requires the file-input native profile`);
    try { return await locator.setInputFiles(files, timeout, isBuffer); }
    catch (error) { throw mapError(error, api); }
  }
  setInputFiles.forFrame = async (session, frame, selector, files, options, api) => {
    const timeoutMs = readOptions(options, api, true);
    if (typeof selector !== 'string') throw new TypeError(`${api}: selector must be a string`);
    if (typeof session.frameSetInputFiles !== 'function')
      throw new UnsupportedOperationError(`${api} requires a rebuilt file-input native profile`);
    try {
      return await session.frameSetInputFiles(frame, selector, files,
        { timeoutMs, strict: options.strict ?? false }, isBuffer);
    } catch (error) { throw mapError(error, api); }
  };
  return setInputFiles;
};
