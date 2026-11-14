'use strict';

// Rust owns the lazy iframe-owner recipe whenever the selected native profile
// provides it. The compatibility fallback retains the earlier handle-based
// resolver for older artifacts, but current profiles never pin a child Frame
// identity across retries.
module.exports = function frameLocatorApi(locatorApi) {
  class FrameLocator {
    #frame;
    #owner;
    #native;
    #description;
    constructor(frame, owner, native, description) {
      this.#frame = frame;
      this.#owner = owner;
      this.#native = native;
      this.#description = description;
    }
    locator(selectorOrLocator, options = {}) {
      if (this.#native) {
        const locator = typeof selectorOrLocator === 'string'
          ? locatorApi.create(this.#frame, selectorOrLocator)
          : selectorOrLocator;
        const description = `${this.#description}.${locator?.toString?.() ?? 'locator(...)'}`;
        return locatorApi._fromFrameNative(
          this.#frame, this.#native, locator, options, description,
        );
      }
      if (typeof selectorOrLocator !== 'string')
        throw new TypeError('frameLocator.locator: selector must be a string or Locator');
      const description = `${this.#description}.${locatorApi._selectorDescription(selectorOrLocator)}`;
      return locatorApi.deferred(this.#frame, async () => {
        const owner = await this.#owner.elementHandle();
        try {
          if (typeof owner.contentFrame !== 'function')
            throw new TypeError('frameLocator: owner did not resolve to an iframe element');
          const child = await owner.contentFrame();
          if (!child) throw new TypeError('frameLocator: iframe has no attached Frame');
          return locatorApi.create(child, selectorOrLocator, options);
        } finally {
          await owner.dispose();
        }
      }, description);
    }
    getByText(value, options) { return this.locator(locatorApi.semantic(this.#frame, 'text', value, options)); }
    getByLabel(value, options) { return this.locator(locatorApi.semantic(this.#frame, 'label', value, options)); }
    getByPlaceholder(value, options) { return this.locator(locatorApi.semantic(this.#frame, 'placeholder', value, options)); }
    getByAltText(value, options) { return this.locator(locatorApi.semantic(this.#frame, 'altText', value, options)); }
    getByTitle(value, options) { return this.locator(locatorApi.semantic(this.#frame, 'title', value, options)); }
    getByTestId(value) { return this.locator(locatorApi.semantic(this.#frame, 'testId', value)); }
    getByRole(value, options) { return this.locator(locatorApi.role(this.#frame, value, options)); }
    owner() { return this.#owner; }
    frameLocator(selector) {
      if (typeof selector !== 'string')
        throw new TypeError('frameLocator.frameLocator: selector must be a string');
      return this.locator(selector).contentFrame();
    }
    first() { return this.#owner.first().contentFrame(); }
    last() { return this.#owner.last().contentFrame(); }
    nth(index) { return this.#owner.nth(index).contentFrame(); }
  }
  locatorApi._setFrameLocatorFactory(
    (frame, owner, native, description) => new FrameLocator(frame, owner, native, description),
  );
  function create(frame, selector) {
    if (typeof selector !== 'string') throw new TypeError('frame.frameLocator: selector must be a string');
    return locatorApi.create(frame, selector).contentFrame();
  }
  return { create };
};
