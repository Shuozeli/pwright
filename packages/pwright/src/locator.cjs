'use strict';

// API-shaped values only. Native Locators own the query graph, Frame identity,
// storage admission and DOM resolution; selector strings are never composed here.
module.exports = function locatorApi({ mapError, validateOptions, UnsupportedOperationError, handleKind }) {
  const act = require('./locator-actions.cjs')({ mapError, validateOptions, handleKind });
  const setInputFiles = require('./file-input.cjs')({ mapError, validateOptions, UnsupportedOperationError });
  const screenshotOptions = require('./screenshot.cjs')({ validateOptions });
  let createFrameLocator;
  function native(api, action) {
    try { return action(); } catch (error) { throw mapError(error, api); }
  }
  function quoteSelector(selector) {
    return `'${selector.replaceAll('\\', '\\\\').replaceAll("'", "\\'")}'`;
  }
  function formatValue(value) {
    if (typeof value === 'string') return quoteSelector(value);
    if (Object.prototype.toString.call(value) === '[object RegExp]') return String(value);
    return String(value);
  }
  function isRegExp(value) {
    return Object.prototype.toString.call(value) === '[object RegExp]';
  }
  function formatCall(name, value, options, keys) {
    const entries = [];
    for (const key of keys) {
      if (options[key] !== undefined) entries.push(`${key}: ${formatValue(options[key])}`);
    }
    return `${name}(${formatValue(value)}${entries.length ? `, { ${entries.join(', ')} }` : ''})`;
  }
  function filters(options) {
    validateOptions(options, ['hasText', 'hasNotText', 'has', 'hasNot', 'visible'], 'locator.filter');
    const value = { hasText: options.hasText, hasNotText: options.hasNotText,
      has: options.has, hasNot: options.hasNot, visible: options.visible };
    for (const key of ['hasText', 'hasNotText']) {
      if (value[key] !== undefined && typeof value[key] !== 'string' && !isRegExp(value[key]))
        throw new TypeError(`locator.filter: ${key} must be a string or RegExp`);
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
    #resolver;
    #description;
    #customDescription;
    constructor(frame, value, resolver, description = 'locator(...)', customDescription = null) {
      this.#frame = frame;
      this.#native = value;
      this.#resolver = resolver;
      this.#description = description;
      this.#customDescription = customDescription;
    }
    static create(frame, value, options, description) {
      return new Locator(frame, value, undefined, description).#applyFilter(options);
    }
    static deferred(frame, resolver, description) {
      return new Locator(frame, undefined, resolver, description);
    }
    static action(locator, operation, options, value, publicApi) {
      if (!(locator instanceof Locator)) throw new TypeError(`${publicApi}: expected a Locator`);
      return locator.#action(operation, options, value, publicApi);
    }
    static inFrame(frame, frameLocator, locator, options, description) {
      if (!(locator instanceof Locator))
        throw new TypeError('frameLocator.locator: selector must be a string or Locator');
      if (!locator.#native)
        return Locator.deferred(frame, async () => Locator.inFrame(
          frame, frameLocator, await locator.#resolved(), options, description,
        ), description);
      const value = native(
        'frameLocator.locator',
        () => frameLocator.locatorFrom(locator.#native),
      );
      return Locator.create(frame, value, filters(options), description);
    }
    page() { return this.#frame.page(); }
    async #resolved() {
      if (this.#native) return this;
      const locator = await this.#resolver();
      if (!(locator instanceof Locator)) throw new TypeError('Deferred Locator resolver did not return a Locator');
      const resolved = await locator.#resolved();
      return new Locator(
        this.#frame, resolved.#native, undefined, this.#description, this.#customDescription,
      );
    }
    #derive(api, action, description = this.#description) {
      if (this.#native)
        return new Locator(this.#frame, native(api, () => action(this.#native)), undefined, description);
      return Locator.deferred(
        this.#frame,
        async () => (await this.#resolved()).#derive(api, action, description),
        description,
      );
    }
    first() { return this.#derive('locator.first', value => value.nth(0), `${this.#description}.first()`); }
    last() { return this.#derive('locator.last', value => value.nth(-1), `${this.#description}.last()`); }
    nth(index) {
      if (!Number.isInteger(index) || index < -2147483648 || index > 2147483647)
        throw new TypeError('locator.nth: index must be an int32');
      return this.#derive('locator.nth', value => value.nth(index), `${this.#description}.nth(${index})`);
    }
    contentFrame() {
      if (typeof createFrameLocator !== 'function')
        throw new UnsupportedOperationError('locator.contentFrame (FrameLocator facade unavailable)');
      const description = `${this.#description}.contentFrame()`;
      if (!this.#native)
        return createFrameLocator(this.#frame, this, undefined, description);
      if (typeof this.#native.contentFrame !== 'function')
        return createFrameLocator(this.#frame, this, undefined, description);
      const value = native('locator.contentFrame', () => this.#native.contentFrame());
      return createFrameLocator(this.#frame, this, value, description);
    }
    frameLocator(selector) {
      if (typeof selector !== 'string')
        throw new TypeError('locator.frameLocator: selector must be a string');
      return this.locator(selector).contentFrame();
    }
    #combine(other, kind, description = `${this.#description}.${kind}(${other.#description})`) {
      if (!(other instanceof Locator)) throw new TypeError(`locator.${kind}: expected a Locator`);
      if (!this.#native || !other.#native) {
        return Locator.deferred(this.#frame, async () =>
          (await this.#resolved()).#combine(await other.#resolved(), kind), description);
      }
      return this.#derive(`locator.${kind}`, value => value.combine(other.#native, kind), description);
    }
    and(other) { return this.#combine(other, 'and'); }
    or(other) { return this.#combine(other, 'or'); }
    #descendant(other) {
      return this.#combine(other, 'within', `${this.#description}.${other.#description}`);
    }
    getByText(value, options) { return this.#descendant(semantic(this.#frame, 'text', value, options)); }
    getByLabel(value, options) { return this.#descendant(semantic(this.#frame, 'label', value, options)); }
    getByPlaceholder(value, options) { return this.#descendant(semantic(this.#frame, 'placeholder', value, options)); }
    getByAltText(value, options) { return this.#descendant(semantic(this.#frame, 'altText', value, options)); }
    getByTitle(value, options) { return this.#descendant(semantic(this.#frame, 'title', value, options)); }
    getByTestId(value) { return this.#descendant(semantic(this.#frame, 'testId', value)); }
    getByRole(value, options) { return this.#descendant(role(this.#frame, value, options)); }
    locator(selector, options = {}) {
      const optionsValue = filters(options);
      const other = typeof selector === 'string' ? create(this.#frame, selector) : selector;
      if (!(other instanceof Locator)) throw new TypeError('locator.locator: selector must be a string or Locator');
      const description = typeof selector === 'string'
        ? `${this.#description}.locator(${quoteSelector(selector)})`
        : `${this.#description}.locator(${other.#description})`;
      return this.#combine(other, 'within', description).#applyFilter(optionsValue);
    }
    filter(options = {}) {
      const optionsValue = filters(options);
      return this.#derive('locator.filter', value => value.copy()).#applyFilter(optionsValue);
    }
    #applyFilter(options) {
      let value = this;
      for (const [key, exclude] of [['hasText', false], ['hasNotText', true]]) {
        if (options[key] === undefined) continue;
        value = value.#derive('locator.filter', nativeValue => isRegExp(options[key])
          ? nativeValue.filterRegex(options[key].source, options[key].flags, exclude)
          : nativeValue.filterText(options[key], exclude));
      }
      if (options.has !== undefined) value = value.#combine(options.has, 'has');
      if (options.hasNot !== undefined) value = value.#combine(options.hasNot, 'hasNot');
      if (options.visible !== undefined) {
        value = value.#derive('locator.filter', nativeValue => nativeValue.visible(options.visible));
      }
      return value;
    }
    async #read(operation, options, attribute) {
      const api = `locator.${operation}`;
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native) return (await this.#resolved()).#read(operation, options, attribute);
      try { return await this.#native.read(operation, timeout, attribute); }
      catch (error) {
        const mapped = mapError(error, api);
        if (mapped?.name === 'TimeoutError' && !mapped.message.includes('waiting for '))
          mapped.message += `\nCall log:\n  - waiting for ${this.toString()}`;
        throw mapped;
      }
    }
    async count(options = {}) {
      validateOptions(options, ['__testHookBeforeQuery'], 'locator.count');
      const hook = options.__testHookBeforeQuery;
      if (hook === undefined) return this.#read('count', {});
      if (typeof hook !== 'function')
        throw new TypeError('locator.count: __testHookBeforeQuery must be a function');
      if (!this.#native) return (await this.#resolved()).count(options);
      if (typeof this.#native.countWithHook !== 'function')
        throw new UnsupportedOperationError('locator.count __testHookBeforeQuery (rebuild native addon)');
      try {
        return await this.#native.countWithHook(async () => { await hook(); });
      } catch (error) { throw mapError(error, 'locator.count'); }
    }
    async boundingBox(options = {}) {
      const api = 'locator.boundingBox';
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native) return (await this.#resolved()).boundingBox(options);
      if (typeof this.#native.boundingBox !== 'function')
        throw new UnsupportedOperationError(`${api} (select locator-bounding-box native profile)`);
      try { return await this.#native.boundingBox(timeout); }
      catch (error) { throw mapError(error, api); }
    }
    async highlight() {
      const api = 'locator.highlight';
      if (!this.#native) return (await this.#resolved()).highlight();
      if (typeof this.#native.highlight !== 'function')
        throw new UnsupportedOperationError(`${api} (select locator-highlight native profile)`);
      try { await this.#native.highlight(this.toString()); }
      catch (error) { throw mapError(error, api); }
    }
    async dispatchEvent(type, eventInit = {}, options = {}) {
      const api = 'locator.dispatchEvent';
      if (typeof type !== 'string') throw new TypeError(`${api}: type must be a string`);
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native) return (await this.#resolved()).dispatchEvent(type, eventInit, options);
      if (typeof this.#native.dispatchEvent !== 'function')
        throw new UnsupportedOperationError(`${api} (select locator-dom-actions native profile)`);
      try { await this.#native.dispatchEvent(type, eventInit, timeout); }
      catch (error) { throw mapError(error, api); }
    }
    async selectText(options = {}) {
      const api = 'locator.selectText';
      validateOptions(options, ['force', 'timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      const force = options.force ?? false;
      if (typeof force !== 'boolean') throw new TypeError(`${api}: force must be a boolean`);
      if (!this.#native) return (await this.#resolved()).selectText(options);
      if (typeof this.#native.selectText !== 'function')
        throw new UnsupportedOperationError(`${api} (select locator-dom-actions native profile)`);
      try { await this.#native.selectText(timeout, force); }
      catch (error) { throw mapError(error, api); }
    }
    async screenshot(options = {}) {
      const api = 'locator.screenshot';
      const params = screenshotOptions(options, api, true);
      if (!this.#native) return (await this.#resolved()).screenshot(options);
      if (typeof this.#native.screenshot !== 'function')
        throw new UnsupportedOperationError(`${api} (select locator-screenshot native profile)`);
      try { return await this.#native.screenshot(params); }
      catch (error) { throw mapError(error, api); }
    }
    async all() {
      const count = await this.count();
      if (count > 1024) throw new Error('locator.all: result exceeds the fixed 1024-item boundary');
      return Array.from({ length: count }, (_, index) => this.nth(index));
    }
    async #evaluate(pageFunction, arg, options, forceFunction, api, first = false) {
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native)
        return (await this.#resolved()).#evaluate(pageFunction, arg, options, forceFunction, api, first);
      try {
        const converter = () => ({
          source: String(pageFunction), isFunction: forceFunction || typeof pageFunction === 'function', argument: arg,
        });
        return first
          ? await this.#native.evaluateFirst(timeout, converter)
          : await this.#native.evaluate(timeout, converter);
      } catch (error) {
        const mapped = mapError(error, api);
        // Playwright decorates the original conversion exception in its API
        // boundary. Preserve object identity and the native JS TypeError when
        // a thrown primitive cannot carry a message property.
        if (mapped === error) error.message = `${api}: ${error.message}`;
        throw mapped;
      }
    }
    evaluate(pageFunction, arg, options = {}) {
      return this.#evaluate(pageFunction, arg, options, false, 'locator.evaluate');
    }
    async evaluateHandle(pageFunction, arg, options = {}) {
      const api = 'locator.evaluateHandle';
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native) return (await this.#resolved()).evaluateHandle(pageFunction, arg, options);
      if (typeof this.#native.evaluateHandle !== 'function')
        throw new UnsupportedOperationError(`${api} (select handles native profile)`);
      try {
        const native = await this.#native.evaluateHandle(timeout, () => ({
          source: String(pageFunction), isFunction: typeof pageFunction === 'function', argument: arg,
        }));
        return await this.#frame._wrapNativeHandle(native);
      } catch (error) { throw mapError(error, api); }
    }
    _evaluateFunction(functionDeclaration, options = {}) {
      return this.#evaluate(functionDeclaration, undefined, options, true, 'locator._evaluateFunction');
    }
    _evaluateFirstFunction(functionDeclaration, arg, options = {}) {
      return this.#evaluate(functionDeclaration, arg, options, true, 'locator._evaluateFirstFunction', true);
    }
    elementHandle(options = {}) { return this._elementHandle(options); }
    async _elementHandle(options = {}) {
      const api = 'locator.elementHandle';
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native) return (await this.#resolved())._elementHandle(options);
      if (typeof this.#native.elementHandle !== 'function')
        throw new UnsupportedOperationError(`${api} (select handles native profile)`);
      try {
        const native = await this.#native.elementHandle(timeout);
        return await this.#frame._wrapNativeHandle(native);
      } catch (error) { throw mapError(error, api); }
    }
    async _waitForElement(state, timeout, api) {
      if (!this.#native) return (await this.#resolved())._waitForElement(state, timeout, api);
      if (typeof this.#native.waitForElement !== 'function')
        throw new UnsupportedOperationError(`${api} (select a handles-capable native profile)`);
      try {
        const native = await this.#native.waitForElement(state, timeout);
        return native === null ? null : await this.#frame._wrapNativeHandle(native);
      } catch (error) { throw mapError(error, api); }
    }
    async _waitForSelectorElement(state, timeout, strict, api) {
      if (!this.#native)
        return (await this.#resolved())._waitForSelectorElement(state, timeout, strict, api);
      if (typeof this.#native.waitForSelectorElement !== 'function')
        throw new UnsupportedOperationError(`${api} (select a handles-capable native profile)`);
      try {
        const native = await this.#native.waitForSelectorElement(state, timeout, strict);
        return native === null ? null : await this.#frame._wrapNativeHandle(native);
      } catch (error) { throw mapError(error, api); }
    }
    async _queryElementHandle() {
      const api = 'locator._queryElementHandle';
      if (!this.#native) return (await this.#resolved())._queryElementHandle();
      if (typeof this.#native.queryElementHandle !== 'function')
        throw new UnsupportedOperationError(`${api} (select handles native profile)`);
      try {
        const native = await this.#native.queryElementHandle();
        return native === null ? null : await this.#frame._wrapNativeHandle(native);
      } catch (error) { throw mapError(error, api); }
    }
    elementHandles() { return this.#elementHandles('locator.elementHandles'); }
    _queryElementHandles() { return this.#elementHandles('locator._queryElementHandles'); }
    async #elementHandles(api) {
      if (!this.#native) return (await this.#resolved()).#elementHandles(api);
      if (typeof this.#native.queryElementHandles !== 'function')
        throw new UnsupportedOperationError(`${api} (select handles native profile)`);
      try {
        return await this.#frame._wrapNativeHandles(await this.#native.queryElementHandles());
      } catch (error) { throw mapError(error, api); }
    }
    async evaluateAll(pageFunction, arg) {
      const api = 'locator.evaluateAll';
      if (!this.#native) return (await this.#resolved()).evaluateAll(pageFunction, arg);
      try {
        return await this.#native.evaluateAll(() => ({
          source: String(pageFunction), isFunction: true, argument: arg,
        }));
      } catch (error) {
        const mapped = mapError(error, api);
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
    async waitFor(options = {}) {
      const api = 'locator.waitFor';
      validateOptions(options, ['state', 'timeout'], api);
      const state = options.state ?? 'visible';
      if (!['attached', 'detached', 'visible', 'hidden'].includes(state))
        throw new TypeError(`${api}: state must be attached, detached, visible, or hidden`);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (!this.#native) return (await this.#resolved()).waitFor(options);
      try { await this.#native.waitFor(state, timeout); }
      catch (error) { throw mapError(error, api); }
    }
    async getAttribute(name, options = {}) {
      if (typeof name !== 'string') throw new TypeError('locator.getAttribute: name must be a string');
      return this.#read('getAttribute', options, name);
    }
    #action(operation, options, value, publicApi) {
      if (this.#native) return act(this.#native, operation, options, value, publicApi);
      return this.#resolved().then(locator => locator.#action(operation, options, value, publicApi));
    }
    click(options = {}) { return this.#action('click', options); }
    dblclick(options = {}) { return this.#action('dblclick', options); }
    tap(options = {}) { return this.#action('tap', options); }
    fill(value, options = {}) { return this.#action('fill', options, value); }
    clear(options = {}) { return this.#action('clear', options); }
    press(key, options = {}) { return this.#action('press', options, key); }
    pressSequentially(text, options = {}) { return this.#action('pressSequentially', options, text); }
    type(text, options = {}) { return this.#action('type', options, text); }
    check(options = {}) { return this.#action('check', options); }
    uncheck(options = {}) { return this.#action('uncheck', options); }
    setChecked(checked, options = {}) { return this.#action('setChecked', options, checked); }
    selectOption(values, options = {}) { return this.#action('selectOption', options, values); }
    setInputFiles(files, options = {}) {
      if (this.#native) return setInputFiles(this.#native, files, options);
      return this.#resolved().then(locator => locator.setInputFiles(files, options));
    }
    hover(options = {}) { return this.#action('hover', options); }
    focus(options = {}) { return this.#action('focus', options); }
    blur(options = {}) { return this.#action('blur', options); }
    scrollIntoViewIfNeeded(options = {}) { return this.#action('scrollIntoViewIfNeeded', options); }
    async _expect(expression, options = {}) {
      if (!this.#native) return (await this.#resolved())._expect(expression, options);
      validateOptions(options, ['expectedNumber', 'expectedText', 'expectedValue', 'expressionArg', 'isNot', 'useInnerText', 'timeout'], 'locator._expect');
      const timeout = options.timeout ?? 5_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError('locator._expect: timeout must be an integer in [0, 4294967295]');
      const isNot = options.isNot ?? false;
      try {
        if (expression === 'to.be.visible' || expression === 'to.be.hidden') {
          const result = await this.#native.expectVisibility(
            timeout, expression === 'to.be.visible', isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        if (expression === 'to.be.attached' || expression === 'to.be.detached' || expression === 'to.be.empty') {
          const result = await this.#native.expectState(
            timeout, expression.slice('to.be.'.length), isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        if (expression === 'to.be.checked') {
          const expected = options.expectedValue;
          if (!expected || typeof expected !== 'object' || Array.isArray(expected))
            throw new TypeError('locator._expect: expectedValue must be an object');
          for (const key of ['checked', 'indeterminate']) {
            if (expected[key] !== undefined && typeof expected[key] !== 'boolean')
              throw new TypeError(`locator._expect: expectedValue.${key} must be a boolean`);
          }
          const result = await this.#native.expectChecked(
            timeout, expected.checked, expected.indeterminate, isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        const booleanStates = {
          'to.be.editable': 'editable',
          'to.be.readonly': 'readonly',
          'to.be.enabled': 'enabled',
          'to.be.disabled': 'disabled',
          'to.be.focused': 'focused',
        };
        if (booleanStates[expression]) {
          const result = await this.#native.expectState(
            timeout, booleanStates[expression], isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        if (expression === 'to.have.count') {
          if (!Number.isInteger(options.expectedNumber) || options.expectedNumber < 0 || options.expectedNumber > 0xffff_ffff)
            throw new TypeError('locator._expect: expectedNumber must be a uint32');
          const result = await this.#native.expectCount(timeout, options.expectedNumber, isNot);
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        if (expression === 'to.have.property') {
          if (typeof options.expressionArg !== 'string')
            throw new TypeError('locator._expect: expressionArg must be a string');
          const result = await this.#native.expectProperty(
            timeout, options.expressionArg, options.expectedValue, isNot,
          );
          return this.#withExpectationDiagnostic(
            result, expression, timeout, isNot, result.missing,
          );
        }
        if (expression === 'to.have.class' || expression === 'to.have.value' || expression === 'to.have.class.array') {
          if (!Array.isArray(options.expectedText))
            throw new TypeError('locator._expect: expectedText must be an array');
          const matchers = options.expectedText.map(expected => {
            if (!expected || (typeof expected.string !== 'string' && typeof expected.regexSource !== 'string'))
              throw new TypeError('locator._expect: expectedText value must contain string or regexSource');
            if (expected.regexFlags !== undefined && typeof expected.regexFlags !== 'string')
              throw new TypeError('locator._expect: regexFlags must be a string');
            return typeof expected.regexSource === 'string'
              ? { expected: expected.regexSource, flags: expected.regexFlags ?? '' }
              : { expected: expected.string };
          });
          if (expression === 'to.have.class.array') {
            const result = await this.#native.expectClassMatchers(timeout, matchers, isNot);
            return this.#withExpectationDiagnostic(result, expression, timeout, isNot, false);
          }
          if (matchers.length !== 1)
            throw new TypeError('locator._expect: expectedText must contain exactly one value');
          const matcher = matchers[0];
          const result = await this.#native.expectString(
            timeout, expression === 'to.have.class' ? 'class' : 'value',
            matcher.expected, matcher.flags, isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        if (expression === 'to.have.text.array') {
          if (!Array.isArray(options.expectedText))
            throw new TypeError('locator._expect: expectedText must be an array');
          const expected = options.expectedText.map(value => {
            if (!value || typeof value.string !== 'string') {
              if (value && typeof value.regexSource === 'string')
                throw new UnsupportedOperationError('locator._expect to.have.text.array RegExp');
              throw new TypeError('locator._expect: expectedText value must contain string');
            }
            return value.string;
          });
          const first = options.expectedText[0] ?? {};
          const result = await this.#native.expectTexts(
            timeout, expected, first.matchSubstring ?? false,
            first.normalizeWhiteSpace ?? false, first.ignoreCase ?? false,
            options.useInnerText ?? false, isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot, false);
        }
        if (expression !== 'to.have.text') throw new UnsupportedOperationError(`locator._expect ${expression}`);
        if (!Array.isArray(options.expectedText) || options.expectedText.length !== 1)
          throw new TypeError('locator._expect: expectedText must contain exactly one value');
        const expected = options.expectedText[0];
        if (!expected || (typeof expected.string !== 'string' && typeof expected.regexSource !== 'string'))
          throw new TypeError('locator._expect: expectedText value must contain string or regexSource');
        if (typeof expected.regexSource === 'string') {
          if (expected.regexFlags !== undefined && typeof expected.regexFlags !== 'string')
            throw new TypeError('locator._expect: regexFlags must be a string');
          const result = await this.#native.expectTextRegex(
            timeout, expected.regexSource, expected.regexFlags ?? '', expected.ignoreCase,
            options.useInnerText ?? false, isNot,
          );
          return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
        }
        const result = await this.#native.expectText(timeout, expected.string,
          expected.matchSubstring ?? false, expected.normalizeWhiteSpace ?? false,
          expected.ignoreCase ?? false, options.useInnerText ?? false, isNot);
        return this.#withExpectationDiagnostic(result, expression, timeout, isNot);
      } catch (error) { throw mapError(error, 'locator._expect'); }
    }
    #withExpectationDiagnostic(
      result, expression, timeout, isNot,
      missing = result.missing ?? (result.preview === undefined && result.received === undefined),
    ) {
      const { missing: _missing, preview, ...publicResult } = result;
      if (!result.timedOut) return publicResult;
      const matcherName = {
        'to.be.attached': 'toBeAttached',
        'to.be.detached': 'toBeAttached',
        'to.be.checked': 'toBeChecked',
        'to.be.disabled': 'toBeDisabled',
        'to.be.editable': 'toBeEditable',
        'to.be.empty': 'toBeEmpty',
        'to.be.enabled': 'toBeEnabled',
        'to.be.focused': 'toBeFocused',
        'to.be.hidden': 'toBeHidden',
        'to.be.readonly': 'toBeEditable',
        'to.be.visible': 'toBeVisible',
        'to.have.class': 'toHaveClass',
        'to.have.class.array': 'toHaveClass',
        'to.have.property': 'toHaveJSProperty',
        'to.have.count': 'toHaveCount',
        'to.have.text': 'toHaveText',
        'to.have.text.array': 'toHaveText',
        'to.have.value': 'toHaveValue',
      }[expression] ?? expression;
      const log = [
        `  - Expect "${isNot ? 'not ' : ''}${matcherName}" with timeout ${timeout}ms`,
        `  - waiting for ${this.toString()}`,
      ];
      if (preview !== undefined) log.push(`  - locator resolved to ${preview}`);
      if (!missing && result.received !== undefined)
        log.push(`  - unexpected value "${String(result.received)}"`);
      return {
        ...publicResult,
        ...(missing ? { errorMessage: 'Error: element(s) not found' } : {}),
        log,
      };
    }
    describe(description) {
      if (typeof description !== 'string') throw new TypeError('locator.describe: description must be a string');
      if (Buffer.byteLength(description) > 64 * 1024)
        throw new TypeError('locator.describe: description exceeds 65536 UTF-8 bytes');
      return new Locator(this.#frame, this.#native, this.#resolver, this.#description, description);
    }
    description() { return this.#customDescription; }
    toString() { return this.#customDescription ?? this.#description; }
  }

  function create(frame, selector, options = {}) {
    if (typeof selector !== 'string') throw new TypeError('frame.locator: selector must be a string');
    const optionsValue = filters(options);
    const value = native('frame.locator', () => frame.page().context().browser()._session.locator(frame._id, selector));
    return Locator.create(frame, value, optionsValue, `locator(${quoteSelector(selector)})`);
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
    return Locator.create(frame, query, {}, formatCall(api, value, options, ['exact']));
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
    return Locator.create(frame, query, {}, formatCall(api, value, options,
      ['name', 'exact', 'checked', 'disabled', 'expanded', 'includeHidden', 'level', 'pressed', 'selected']));
  }
  function deferred(frame, resolver, description) {
    return Locator.deferred(frame, resolver, description);
  }
  function fromNative(frame, value, options = {}, description) {
    return Locator.create(frame, value, filters(options), description);
  }
  function fromFrameNative(frame, frameLocator, locator, options = {}, description) {
    return Locator.inFrame(frame, frameLocator, locator, options, description);
  }
  function setFrameLocatorFactory(factory) {
    if (typeof factory !== 'function')
      throw new TypeError('FrameLocator factory must be a function');
    createFrameLocator = factory;
  }
  function selectorDescription(selector) {
    if (typeof selector !== 'string')
      throw new TypeError('selector description requires a string');
    return `locator(${quoteSelector(selector)})`;
  }
  return {
    create,
    semantic,
    role,
    deferred,
    action: Locator.action,
    _fromNative: fromNative,
    _fromFrameNative: fromFrameNative,
    _native: native,
    _selectorDescription: selectorDescription,
    _setFrameLocatorFactory: setFrameLocatorFactory,
  };
};
