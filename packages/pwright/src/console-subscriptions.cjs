'use strict';

const { EventEmitter } = require('node:events');

// EventEmitter owns language listener identity and ordering. Rust owns the
// bounded native subscription; this adapter only mirrors the zero-to-one and
// one-to-zero listener transitions across the FFI boundary.
class NativeConsoleEventEmitter extends EventEmitter {
  #targetId;
  #setConsoleListener;
  #consoleRegistered = false;

  constructor(session, targetId, method) {
    super();
    this.#targetId = targetId;
    this.#setConsoleListener = typeof session?.[method] === 'function'
      ? session[method].bind(session)
      : undefined;
  }

  #register() {
    if (this.#consoleRegistered || !this.#setConsoleListener) return;
    this.#setConsoleListener(this.#targetId, true);
    this.#consoleRegistered = true;
  }

  #release() {
    if (!this.#consoleRegistered) return;
    this.#consoleRegistered = false;
    try { this.#setConsoleListener(this.#targetId, false); }
    catch { /* A closed native owner no longer retains a live subscription. */ }
  }

  #add(method, event, listener) {
    if (event !== 'console' || typeof listener !== 'function')
      return EventEmitter.prototype[method].call(this, event, listener);
    if (this.listenerCount('console') === 0) this.#register();
    return EventEmitter.prototype[method].call(this, event, listener);
  }

  addListener(event, listener) { return this.#add('addListener', event, listener); }
  on(event, listener) { return this.addListener(event, listener); }
  prependListener(event, listener) { return this.#add('prependListener', event, listener); }

  removeListener(event, listener) {
    const result = EventEmitter.prototype.removeListener.call(this, event, listener);
    if (event === 'console' && this.listenerCount('console') === 0) this.#release();
    return result;
  }

  off(event, listener) { return this.removeListener(event, listener); }

  removeAllListeners(event) {
    const result = event === undefined
      ? EventEmitter.prototype.removeAllListeners.call(this)
      : EventEmitter.prototype.removeAllListeners.call(this, event);
    if ((event === undefined || event === 'console') && this.listenerCount('console') === 0)
      this.#release();
    return result;
  }
}

module.exports = NativeConsoleEventEmitter;
