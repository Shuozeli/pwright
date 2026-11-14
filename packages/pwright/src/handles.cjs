'use strict';

// Object identity and language forwarding only. Native owners perform every
// browser read and release; no selector re-query substitutes for a Handle.
module.exports = function handleApi({
  mapError,
  validateOptions = (options, supported, api) => {
    if (!options || typeof options !== 'object' || Array.isArray(options))
      throw new TypeError(`${api}: options must be an object`);
    for (const key of Object.keys(options)) {
      if (!supported.includes(key)) throw new UnsupportedOperationError(`${api} option ${key}`);
    }
  },
  UnsupportedOperationError,
}) {
  const handleWrappers = new WeakSet();
  const elementHandleWrappers = new WeakSet();
  function validateSelector(selector, api) {
    if (typeof selector !== 'string') throw new TypeError(`${api}: selector must be a string`);
  }

  async function withTemporaryNative(session, native, operation) {
    let result;
    try {
      result = await operation();
    } catch (primary) {
      try { await session.handleDispose(native); }
      catch (cleanup) {
        throw new AggregateError(
          [primary, cleanup],
          'Temporary Handle operation and cleanup failed',
          { cause: primary },
        );
      }
      throw primary;
    }
    await session.handleDispose(native);
    return result;
  }

  class JSHandle {
    #session;
    #native;
    #resolveFrame;
    #preview;
    constructor(session, native, resolveFrame, isElement = false) {
      this.#session = session;
      this.#native = native;
      this.#resolveFrame = resolveFrame;
      const preview = typeof native.preview === 'function' ? native.preview() : undefined;
      this.#preview = isElement ? elementPreview(preview) : preview;
      if (isElement && typeof session.elementHandlePreview === 'function') {
        // Playwright refreshes ElementHandle previews asynchronously. Rust
        // owns the DOM algorithm; this facade only caches its bounded result.
        void Promise.resolve()
          .then(() => session.elementHandlePreview(native))
          .then(value => {
            if (typeof value === 'string' && value.startsWith('JSHandle@')) this.#preview = value;
          })
          .catch(() => {});
      }
      if (typeof native.bindWrapper === 'function') {
        native.bindWrapper(this);
        handleWrappers.add(this);
      }
      // Legacy/unselected converters can reject this reserved marker. Only the
      // native type-tagged binding grants reference authority; this tag cannot.
      Object.defineProperty(this, Symbol.toStringTag, { value: 'PwrightJSHandle' });
    }
    asElement() { return null; }
    async jsonValue() {
      try { return await this.#session.handleJsonValue(this.#native); }
      catch (error) { throw mapError(error, 'jsHandle.jsonValue'); }
    }
    async evaluate(pageFunction, arg) {
      if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
      try { return await this.#session.handleEvaluate(this.#native, String(pageFunction), typeof pageFunction === 'function', arg); }
      catch (error) { throw mapError(error, 'jsHandle.evaluate'); }
    }
    async dispose() {
      try { await this.#session.handleDispose(this.#native); }
      catch (error) { throw mapError(error, 'jsHandle.dispose'); }
    }
    async evaluateHandle(pageFunction, arg) {
      if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
      try { return await create(this.#session, await this.#session.handleEvaluateHandle(this.#native, String(pageFunction), typeof pageFunction === 'function', arg), this.#resolveFrame); }
      catch (error) { throw mapError(error, 'jsHandle.evaluateHandle'); }
    }
    async getProperty(name) {
      if (typeof name !== 'string') throw new TypeError('jsHandle.getProperty: property name must be a string');
      try { return await create(this.#session, await this.#session.handleGetProperty(this.#native, name), this.#resolveFrame); }
      catch (error) { throw mapError(error, 'jsHandle.getProperty'); }
    }
    async getProperties() {
      try { return await createBatch(this.#session, await this.#session.handleGetProperties(this.#native), this.#resolveFrame); }
      catch (error) { throw mapError(error, 'jsHandle.getProperties'); }
    }
    toString() { return this.#preview ?? 'JSHandle@object'; }
  }
  class ElementHandle extends JSHandle {
    #session;
    #native;
    #resolveFrame;
    constructor(session, native, resolveFrame) {
      super(session, native, resolveFrame, true);
      if (handleWrappers.has(this)) elementHandleWrappers.add(this);
      this.#session = session;
      this.#native = native;
      this.#resolveFrame = resolveFrame;
    }
    asElement() { return this; }
    async innerText() {
      try {
        return await this.#session.elementHandleInnerText(this.#native);
      } catch (error) { throw mapError(error, 'elementHandle.innerText'); }
    }
    async textContent() {
      try {
        return await this.#session.elementHandleTextContent(this.#native);
      } catch (error) { throw mapError(error, 'elementHandle.textContent'); }
    }
    async focus() {
      const api = 'elementHandle.focus';
      if (arguments.length !== 0) throw new UnsupportedOperationError(`${api} options`);
      if (typeof this.#session.elementHandleFocus !== 'function')
        throw new UnsupportedOperationError(`${api} (select element-handle-dom native profile)`);
      try { await this.#session.elementHandleFocus(this.#native); }
      catch (error) { throw mapError(error, api); }
    }
    async selectOption(value, options = {}) {
      const api = 'elementHandle.selectOption';
      if (typeof value !== 'string')
        throw new UnsupportedOperationError(`${api} currently supports only a string value or label`);
      validateOptions(options, ['timeout'], api);
      if (Buffer.byteLength(value, 'utf8') > 64 * 1024)
        throw new TypeError(`${api}: value exceeds 65536 UTF-8 bytes`);
      const timeout = options.timeout ?? undefined;
      if (timeout !== undefined && (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff))
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (typeof this.#session.elementHandleSelectOptionValue !== 'function')
        throw new UnsupportedOperationError(`${api} (select element-handle-select-option-value native profile)`);
      try { return await this.#session.elementHandleSelectOptionValue(this.#native, value, timeout); }
      catch (error) { throw mapError(error, api); }
    }
    async scrollIntoViewIfNeeded(options = {}) {
      const api = 'elementHandle.scrollIntoViewIfNeeded';
      validateOptions(options, ['timeout'], api);
      const timeout = options.timeout ?? undefined;
      if (timeout !== undefined && (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff))
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (typeof this.#session.elementHandleScrollIntoViewIfNeeded !== 'function')
        throw new UnsupportedOperationError(`${api} (select element-handle-scroll-into-view native profile)`);
      try { await this.#session.elementHandleScrollIntoViewIfNeeded(this.#native, timeout); }
      catch (error) { throw mapError(error, api); }
    }
    async boundingBox() {
      const api = 'elementHandle.boundingBox';
      if (typeof this.#session.elementHandleBoundingBox !== 'function')
        throw new UnsupportedOperationError(`${api} (select element-handle-dom and locator-bounding-box native profiles)`);
      try { return await this.#session.elementHandleBoundingBox(this.#native); }
      catch (error) { throw mapError(error, api); }
    }
    async dispatchEvent(type, eventInit = {}) {
      const api = 'elementHandle.dispatchEvent';
      if (typeof type !== 'string') throw new TypeError(`${api}: type must be a string`);
      if (typeof this.#session.elementHandleDispatchEvent !== 'function')
        throw new UnsupportedOperationError(`${api} (select element-handle-dom and locator-dom-actions native profiles)`);
      try { await this.#session.elementHandleDispatchEvent(this.#native, type, eventInit); }
      catch (error) { throw mapError(error, api); }
    }
    async tap(options = {}) {
      const api = 'elementHandle.tap';
      validateOptions(options, ['force', 'modifiers', 'noWaitAfter', 'position', 'timeout', 'trial'], api);
      const timeout = options.timeout ?? 30_000;
      if (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff)
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (options.position !== undefined)
        validateOptions(options.position, ['x', 'y'], `${api} position`);
      if (typeof this.#session.elementHandleTap !== 'function')
        throw new UnsupportedOperationError(`${api} (select touchscreen-input native profile)`);
      const snapshot = {
        force: options.force,
        modifiers: options.modifiers,
        noWaitAfter: options.noWaitAfter,
        position: options.position,
        trial: options.trial,
      };
      try { await this.#session.elementHandleTap(this.#native, timeout, snapshot); }
      catch (error) { throw mapError(error, api); }
    }
    async #domRead(operation, attribute) {
      try {
        return await this.#session.elementHandleDomRead(this.#native, operation, attribute);
      } catch (error) { throw mapError(error, `elementHandle.${operation}`); }
    }
    async getAttribute(name) {
      if (typeof name !== 'string') throw new TypeError('elementHandle.getAttribute: name must be a string');
      return this.#domRead('getAttribute', name);
    }
    async innerHTML() { return this.#domRead('innerHTML'); }
    async inputValue() { return this.#domRead('inputValue'); }
    async isChecked() { return this.#domRead('isChecked'); }
    async isDisabled() { return this.#domRead('isDisabled'); }
    async isEditable() { return this.#domRead('isEditable'); }
    async isEnabled() { return this.#domRead('isEnabled'); }
    async isHidden() { return this.#domRead('isHidden'); }
    async isVisible() { return this.#domRead('isVisible'); }
    async ownerFrame() {
      if (typeof this.#resolveFrame !== 'function') throw new Error('ElementHandle owner Frame resolver is missing');
      try {
        const id = await this.#session.handleOwnerFrame(this.#native);
        return id == null ? null : this.#resolveFrame(id);
      } catch (error) { throw mapError(error, 'elementHandle.ownerFrame'); }
    }
    async contentFrame() {
      if (typeof this.#resolveFrame !== 'function') throw new Error('ElementHandle content Frame resolver is missing');
      try {
        const id = await this.#session.handleContentFrame(this.#native);
        return id == null ? null : this.#resolveFrame(id);
      } catch (error) { throw mapError(error, 'elementHandle.contentFrame'); }
    }
    async $(selector) {
      const api = 'elementHandle.$';
      validateSelector(selector, api);
      try {
        const native = await this.#session.elementHandleQuerySelector(this.#native, selector);
        return native == null ? null : await create(this.#session, native, this.#resolveFrame);
      } catch (error) { throw mapError(error, api); }
    }
    async $$(selector) {
      const api = 'elementHandle.$$';
      validateSelector(selector, api);
      try {
        return await createList(
          this.#session,
          await this.#session.elementHandleQuerySelectorAll(this.#native, selector),
          this.#resolveFrame,
        );
      } catch (error) { throw mapError(error, api); }
    }
    async waitForSelector(selector, options = {}) {
      const api = 'elementHandle.waitForSelector';
      validateOptions(options, ['state', 'strict', 'timeout', 'visibility', 'waitFor'], api);
      if (options.visibility)
        throw new Error('options.visibility is not supported, did you mean options.state?');
      if (options.waitFor && options.waitFor !== 'visible')
        throw new Error('options.waitFor is not supported, did you mean options.state?');
      validateSelector(selector, api);
      const state = options.state ?? 'visible';
      if (!['attached', 'detached', 'visible', 'hidden'].includes(state))
        throw new TypeError(`${api}: state: expected one of (attached|detached|visible|hidden)`);
      const strict = options.strict ?? false;
      if (typeof strict !== 'boolean') throw new TypeError(`${api}: strict must be a boolean`);
      const timeout = options.timeout ?? undefined;
      if (timeout !== undefined && (!Number.isInteger(timeout) || timeout < 0 || timeout > 0xffff_ffff))
        throw new TypeError(`${api}: timeout must be an integer in [0, 4294967295]`);
      if (typeof this.#session.elementHandleWaitForSelector !== 'function')
        throw new UnsupportedOperationError(`${api} (select element-handle-wait-for-selector native profile)`);
      try {
        const native = await this.#session.elementHandleWaitForSelector(
          this.#native, selector, state, strict, timeout,
        );
        return native == null ? null : await create(this.#session, native, this.#resolveFrame);
      } catch (error) {
        const mapped = mapError(error, api);
        if (['PWRIGHT_TIMEOUT', 'PWRIGHT_TARGET_CLOSED', 'PWRIGHT_BACKEND', 'PWRIGHT_JAVASCRIPT'].includes(mapped?.code)
          && !mapped.message.includes('waiting for locator(')) {
          const escaped = selector.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
          mapped.message += `\nCall log:\n  - waiting for locator('${escaped}')${state === 'attached' ? '' : ` to be ${state}`}`;
        }
        throw mapped;
      }
    }
    async $eval(selector, pageFunction, arg) {
      const api = 'elementHandle.$eval';
      if (arguments.length > 3) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
      validateSelector(selector, api);
      const source = String(pageFunction);
      try {
        const target = await this.#session.elementHandleQuerySelector(this.#native, selector);
        if (target == null) {
          throw new Error(`${api}: Failed to find element matching selector "${selector}"`);
        }
        return await withTemporaryNative(
          this.#session,
          target,
          () => this.#session.handleEvaluate(
            target,
            source,
            typeof pageFunction === 'function',
            arg,
          ),
        );
      } catch (error) { throw mapError(error, api); }
    }
    async $$eval(selector, pageFunction, arg) {
      const api = 'elementHandle.$$eval';
      if (arguments.length > 3) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
      validateSelector(selector, api);
      const source = String(pageFunction);
      try {
        return await this.#session.elementHandleEvalOnSelectorAll(
          this.#native,
          selector,
          source,
          typeof pageFunction === 'function',
          arg,
        );
      } catch (error) { throw mapError(error, api); }
    }
  }
  function elementPreview(preview) {
    if (typeof preview !== 'string' || preview.length === 0) return 'JSHandle@node';
    if (preview.startsWith('JSHandle@')) return preview;
    // CDP's initial node description is only its lower-case node name. Rust
    // retains that bounded value with the Handle; the language facade must
    // keep toString synchronous just like Playwright's cached preview.
    if (/^[a-z][a-z0-9:-]*$/.test(preview)) {
      const voidElements = new Set([
        'area', 'base', 'br', 'col', 'command', 'embed', 'hr', 'img', 'input',
        'keygen', 'link', 'menuitem', 'meta', 'param', 'source', 'track', 'wbr',
      ]);
      return voidElements.has(preview)
        ? `JSHandle@<${preview}/>`
        : `JSHandle@<${preview}></${preview}>`;
    }
    return `JSHandle@${preview}`;
  }
  function wrap(session, native, resolveFrame) {
    return native.isElement() ? new ElementHandle(session, native, resolveFrame) : new JSHandle(session, native, resolveFrame);
  }
  async function create(session, native, resolveFrame) {
    try {
      return wrap(session, native, resolveFrame);
    } catch (primary) {
      // Native publication already transferred one owned result. Request its
      // release immediately, even if classification or wrapper binding failed;
      // do not wait for an unreachable wrapper to be garbage-collected.
      try { await session.handleDispose(native); }
      catch (cleanup) {
        throw new AggregateError([primary, cleanup], 'Handle wrapper publication and cleanup failed', { cause: primary });
      }
      throw primary;
    }
  }
  async function createBatch(session, namedHandles, resolveFrame) {
    try {
      const properties = new Map();
      // Rust supplies the validated, ordered snapshot and one owner per entry,
      // including aliases. This layer only classifies and publishes wrappers.
      for (const { name, handle } of namedHandles) properties.set(name, wrap(session, handle, resolveFrame));
      return properties;
    } catch (primary) {
      const failures = [primary];
      // Do not use create() above: the batch owns compensation for every entry,
      // including already wrapped and not yet visited children, exactly once.
      // Sequential disposal respects the bounded native call/queue allowance;
      // a cleanup failure must not prevent attempts for subsequent owners.
      for (const { handle } of namedHandles) {
        try { await session.handleDispose(handle); }
        catch (cleanup) { failures.push(cleanup); }
      }
      if (failures.length > 1) {
        throw new AggregateError(failures, 'Handle batch wrapper publication and cleanup failed', { cause: primary });
      }
      throw primary;
    }
  }
  async function createList(session, handles, resolveFrame) {
    try {
      return handles.map(handle => wrap(session, handle, resolveFrame));
    } catch (primary) {
      const failures = [primary];
      for (const handle of handles) {
        try { await session.handleDispose(handle); }
        catch (cleanup) { failures.push(cleanup); }
      }
      if (failures.length > 1) {
        throw new AggregateError(failures, 'Handle list wrapper publication and cleanup failed', { cause: primary });
      }
      throw primary;
    }
  }
  function handleKind(value) {
    if (elementHandleWrappers.has(value)) return 'element';
    if (handleWrappers.has(value)) return 'other';
    // A copied prototype is not a retained native Handle. Reject it rather
    // than letting selectOption interpret its empty own keys as a descriptor.
    if (value instanceof ElementHandle || value instanceof JSHandle) return 'forged';
    return null;
  }
  return { create, createBatch, createList, handleKind };
};
