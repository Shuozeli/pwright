'use strict';

// API-shaped values only. Native Locators own the query graph, Frame identity,
// storage admission and DOM resolution; selector strings are never composed here.
module.exports = function locatorApi({ mapError, validateOptions, UnsupportedOperationError }) {
  const act = require('./locator-actions.cjs')({ mapError, validateOptions });
  const setInputFiles = require('./file-input.cjs')({ mapError, validateOptions, UnsupportedOperationError });
  function native(api, action) {
    try { return action(); } catch (error) { throw mapError(error, api); }
  }
  function filters(options) {
    validateOptions(options, ['hasText', 'hasNotText', 'has', 'hasNot', 'visible'], 'locator.filter');
    const value = { hasText: options.hasText, hasNotText: options.hasNotText,
      has: options.has, hasNot: options.hasNot, visible: options.visible };
    for (const key of ['hasText', 'hasNotText']) {
      if (value[key] instanceof RegExp) throw new UnsupportedOperationError(`locator.filter ${key} RegExp`);
      if (value[key] !== undefined && typeof value[key] !== 'string') throw new TypeError(`locator.filter: ${key} must be a string`);
    }
    for (const key of ['has', 'hasNot']) {
      if (value[key] !== undefined && !(value[key] instanceof Locator)) throw new TypeError(`locator.filter: ${key} must be a Locator`);
    }
    if (value.visible !== undefined && typeof value.visible !== 'boolean') throw new TypeError('locator.filter: visible must be a boolean');
    return value;
  }

  class Locator {
    #frame;
    #native;
    constructor(frame, value) { this.#frame = frame; this.#native = value; }
    static create(frame, value, options) { return new Locator(frame, value).#applyFilter(options); }
    page() { return this.#frame.page(); }
    #derive(api, action) { return new Locator(this.#frame, native(api, action)); }
    first() { return this.nth(0); }
    last() { return this.nth(-1); }
    nth(index) {
      if (!Number.isInteger(index) || index < -2147483648 || index > 2147483647)
        throw new TypeError('locator.nth: index must be an int32');
      return this.#derive('locator.nth', () => this.#native.nth(index));
    }
    #combine(other, kind) {
      if (!(other instanceof Locator)) throw new TypeError(`locator.${kind}: expected a Locator`);
      return this.#derive(`locator.${kind}`, () => this.#native.combine(other.#native, kind));
    }
    and(other) { return this.#combine(other, 'and'); }
    or(other) { return this.#combine(other, 'or'); }
    getByText(value, options) { return this.#combine(semantic(this.#frame, 'text', value, options), 'within'); }
    getByLabel(value, options) { return this.#combine(semantic(this.#frame, 'label', value, options), 'within'); }
    getByPlaceholder(value, options) { return this.#combine(semantic(this.#frame, 'placeholder', value, options), 'within'); }
    getByAltText(value, options) { return this.#combine(semantic(this.#frame, 'altText', value, options), 'within'); }
    getByTitle(value, options) { return this.#combine(semantic(this.#frame, 'title', value, options), 'within'); }
    getByTestId(value) { return this.#combine(semantic(this.#frame, 'testId', value), 'within'); }
    getByRole(value, options) { return this.#combine(role(this.#frame, value, options), 'within'); }
    locator(selector, options = {}) {
      const optionsValue = filters(options);
      const other = typeof selector === 'string' ? create(this.#frame, selector) : selector;
      return this.#combine(other, 'within').#applyFilter(optionsValue);
    }
    filter(options = {}) {
      const optionsValue = filters(options);
      return this.#derive('locator.filter', () => this.#native.copy()).#applyFilter(optionsValue);
    }
    #applyFilter(options) {
      let value = this;
      for (const [key, exclude] of [['hasText', false], ['hasNotText', true]]) {
        if (options[key] === undefined) continue;
        value = value.#derive('locator.filter', () => value.#native.filterText(options[key], exclude));
      }
      if (options.has !== undefined) value = value.#combine(options.has, 'has');
      if (options.hasNot !== undefined) value = value.#combine(options.hasNot, 'hasNot');
      if (options.visible !== undefined) {
        value = value.#derive('locator.filter', () => value.#native.visible(options.visible));
      }
      return value;
    }
    async #read(operation, options, attribute) {
      const api = `locator.${operation}`;
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      try { return await this.#native.read(operation, timeout, attribute); }
      catch (error) { throw mapError(error, api); }
    }
    count() { return this.#read('count', {}); }
    async evaluate(pageFunction, arg, options = {}) {
      const api = 'locator.evaluate';
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      try {
        return await this.#native.evaluate(timeout, () => ({
          source: String(pageFunction), isFunction: typeof pageFunction === 'function', argument: arg,
        }));
      } catch (error) {
        const mapped = mapError(error, api);
        // Playwright decorates the original conversion exception in its API
        // boundary. Preserve object identity and the native JS TypeError when
        // a thrown primitive cannot carry a message property.
        if (mapped === error) error.message = `${api}: ${error.message}`;
        throw mapped;
      }
    }
    allTextContents() { return this.#read('allTextContents', {}); }
    allInnerTexts() { return this.#read('allInnerTexts', {}); }
    textContent(options = {}) { return this.#read('textContent', options); }
    innerText(options = {}) { return this.#read('innerText', options); }
    innerHTML(options = {}) { return this.#read('innerHTML', options); }
    inputValue(options = {}) { return this.#read('inputValue', options); }
    isVisible(options = {}) { return this.#read('isVisible', options); }
    isHidden(options = {}) { return this.#read('isHidden', options); }
    isEnabled(options = {}) { return this.#read('isEnabled', options); }
    isDisabled(options = {}) { return this.#read('isDisabled', options); }
    isEditable(options = {}) { return this.#read('isEditable', options); }
    isChecked(options = {}) { return this.#read('isChecked', options); }
    async getAttribute(name, options = {}) {
      if (typeof name !== 'string') throw new TypeError('locator.getAttribute: name must be a string');
      return this.#read('getAttribute', options, name);
    }
    click(options = {}) { return act(this.#native, 'click', options); }
    dblclick(options = {}) { return act(this.#native, 'dblclick', options); }
    fill(value, options = {}) { return act(this.#native, 'fill', options, value); }
    clear(options = {}) { return act(this.#native, 'clear', options); }
    press(key, options = {}) { return act(this.#native, 'press', options, key); }
    pressSequentially(text, options = {}) { return act(this.#native, 'pressSequentially', options, text); }
    type(text, options = {}) { return act(this.#native, 'type', options, text); }
    check(options = {}) { return act(this.#native, 'check', options); }
    uncheck(options = {}) { return act(this.#native, 'uncheck', options); }
    setChecked(checked, options = {}) { return act(this.#native, 'setChecked', options, checked); }
    selectOption(values, options = {}) { return act(this.#native, 'selectOption', options, values); }
    setInputFiles(files, options = {}) { return setInputFiles(this.#native, files, options); }
    hover(options = {}) { return act(this.#native, 'hover', options); }
    focus(options = {}) { return act(this.#native, 'focus', options); }
    blur(options = {}) { return act(this.#native, 'blur', options); }
    scrollIntoViewIfNeeded(options = {}) { return act(this.#native, 'scrollIntoViewIfNeeded', options); }
  }

  function create(frame, selector, options = {}) {
    if (typeof selector !== 'string') throw new TypeError('frame.locator: selector must be a string');
    const optionsValue = filters(options);
    const value = native('frame.locator', () => frame.page().context().browser()._session.locator(frame._id, selector));
    return Locator.create(frame, value, optionsValue);
  }
  function matcher(value, api) {
    if (typeof value === 'string') return { source: value, flags: undefined };
    if (Object.prototype.toString.call(value) === '[object RegExp]')
      return { source: value.source, flags: value.flags };
    throw new TypeError(`${api}: expected a string or RegExp`);
  }
  function semantic(frame, kind, value, options = {}) {
    const api = `getBy${kind[0].toUpperCase()}${kind.slice(1)}`;
    validateOptions(options, ['exact'], api);
    if (options.exact !== undefined && typeof options.exact !== 'boolean') throw new TypeError(`${api}: exact must be a boolean`);
    const input = matcher(value, api);
    const query = native(api, () => frame.page().context().browser()._session.semanticLocator(frame._id, kind, input.source, options.exact ?? false, input.flags));
    return Locator.create(frame, query, {});
  }
  function role(frame, value, options = {}) {
    const api = 'getByRole';
    if (typeof value !== 'string') throw new TypeError(`${api}: role must be a string`);
    validateOptions(options, ['name', 'exact', 'checked', 'disabled', 'expanded', 'includeHidden', 'level', 'pressed', 'selected'], api);
    const input = options.name === undefined ? {} : matcher(options.name, api);
    for (const key of ['exact', 'checked', 'disabled', 'expanded', 'includeHidden', 'pressed', 'selected'])
      if (options[key] !== undefined && typeof options[key] !== 'boolean') throw new TypeError(`${api}: ${key} must be a boolean`);
    if (options.level !== undefined && (!Number.isInteger(options.level) || options.level < 0 || options.level > 0xffff_ffff))
      throw new TypeError(`${api}: level must be a uint32`);
    const converted = { ...options, name: input.source, nameFlags: input.flags, exact: options.exact ?? false, includeHidden: options.includeHidden ?? false };
    const query = native(api, () => frame.page().context().browser()._session.roleLocator(frame._id, value, converted));
    return Locator.create(frame, query, {});
  }
  return { create, semantic, role };
};
