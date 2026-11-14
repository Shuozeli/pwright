'use strict';

const { readFile } = require('node:fs/promises');

module.exports = ({ getNativeEnvironment, mapError, validateOptions, readSelectorFile = readFile }) => {
  const registrationApi = 'selectors.register';

  async function source(script) {
    if (typeof script === 'function') return `(${script.toString()})(undefined)`;
    if (typeof script === 'string') return script;
    if (!script || typeof script !== 'object' || Array.isArray(script))
      throw new TypeError(`${registrationApi}: script must be a function, string, or object`);
    if (script.content !== undefined) {
      if (typeof script.content !== 'string')
        throw new TypeError(`${registrationApi}: content must be a string`);
      return script.content;
    }
    if (script.path !== undefined) {
      if (typeof script.path !== 'string')
        throw new TypeError(`${registrationApi}: path must be a string`);
      return readSelectorFile(script.path, 'utf8');
    }
    throw new Error('Either path or content property must be present');
  }

  function registrationError(error) {
    const mapped = mapError(error, registrationApi);
    const duplicatePrefix = `${registrationApi}: Selectors.register: `;
    if (mapped?.message?.startsWith(duplicatePrefix))
      mapped.message = `${registrationApi}: ${mapped.message.slice(duplicatePrefix.length)}`;
    else if (mapped?.message?.startsWith('Selectors.register: '))
      mapped.message = `${registrationApi}: ${mapped.message.slice('Selectors.register: '.length)}`;
    return mapped;
  }

  return Object.freeze({
    async register(name, script, options = {}) {
      options = { ...options };
      validateOptions(options, ['contentScript'], registrationApi);
      if (typeof name !== 'string')
        throw new TypeError(`${registrationApi}: name must be a string`);
      if (options.contentScript !== undefined && typeof options.contentScript !== 'boolean')
        throw new TypeError(`${registrationApi}: contentScript must be a boolean`);
      const selectorSource = await source(script);
      try {
        getNativeEnvironment().registerSelectorEngine(
          name,
          selectorSource,
          options.contentScript ?? false,
        );
      } catch (error) {
        throw registrationError(error);
      }
    },

    setTestIdAttribute(attributeName) {
      const api = 'selectors.setTestIdAttribute';
      if (typeof attributeName !== 'string')
        throw new TypeError(`${api}: attributeName must be a string`);
      try {
        getNativeEnvironment().setTestIdAttribute(attributeName);
      } catch (error) {
        throw mapError(error, api);
      }
    },
  });
};
