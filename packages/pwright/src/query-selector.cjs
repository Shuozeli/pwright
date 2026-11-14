'use strict';

// Playwright-shaped query facade. Selector execution, world selection,
// ElementHandle adoption and ownership stay in Rust.
module.exports = function querySelector() {
  async function one(frame, selector, api) {
    if (typeof selector !== 'string')
      throw new TypeError(`${api}: selector: expected string, got ${typeof selector}`);
    // Native Locator construction owns Playwright selector parsing, including
    // registered-engine name case and unknown-engine diagnostics.
    // Page/Frame `$` are intentionally non-strict: unlike Locator's Handle
    // operation they return the first match when several elements resolve.
    // Rust resolves first-or-null and publishes the optional ElementHandle in
    // one browser read, so DOM mutation cannot split a count/read pair.
    return await frame.locator(selector)._queryElementHandle();
  }

  async function all(frame, selector, api) {
    if (typeof selector !== 'string')
      throw new TypeError(`${api}: selector: expected string, got ${typeof selector}`);
    return await frame.locator(selector)._queryElementHandles();
  }

  return { one, all };
};
