'use strict';

// Object identity and language forwarding only. Native owners perform every
// browser read and release; no selector re-query substitutes for a Handle.
module.exports = function handleApi({ mapError, UnsupportedOperationError }) {
  class JSHandle {
    #session;
    #native;
    #resolveFrame;
    constructor(session, native, resolveFrame) {
      this.#session = session;
      this.#native = native;
      this.#resolveFrame = resolveFrame;
      native.bindWrapper?.(this);
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
      if (typeof pageFunction !== 'function' && typeof pageFunction !== 'string') throw new TypeError('jsHandle.evaluate: pageFunction must be a function or string');
      try { return await this.#session.handleEvaluate(this.#native, String(pageFunction), typeof pageFunction === 'function', arg); }
      catch (error) { throw mapError(error, 'jsHandle.evaluate'); }
    }
    async dispose() {
      try { await this.#session.handleDispose(this.#native); }
      catch (error) { throw mapError(error, 'jsHandle.dispose'); }
    }
    async evaluateHandle(pageFunction, arg) {
      if (arguments.length > 2) throw new Error('Too many arguments. If you need to pass more than 1 argument to the function wrap them in an object.');
      if (typeof pageFunction !== 'function' && typeof pageFunction !== 'string') throw new TypeError('jsHandle.evaluateHandle: pageFunction must be a function or string');
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
    toString() { throw new UnsupportedOperationError('jsHandle.toString'); }
  }
  class ElementHandle extends JSHandle {
    #session;
    #native;
    #resolveFrame;
    constructor(session, native, resolveFrame) {
      super(session, native, resolveFrame);
      this.#session = session;
      this.#native = native;
      this.#resolveFrame = resolveFrame;
    }
    asElement() { return this; }
    async ownerFrame() {
      if (typeof this.#resolveFrame !== 'function') throw new Error('ElementHandle owner Frame resolver is missing');
      try {
        const id = await this.#session.handleOwnerFrame(this.#native);
        return id == null ? null : this.#resolveFrame(id);
      } catch (error) { throw mapError(error, 'elementHandle.ownerFrame'); }
    }
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
  return { create, createBatch };
};
