'use strict';

// Language operations only. Rust selects carrier field order, short-circuiting
// and native truth values. No payload traversal, scheduler or native work lives
// here, and these ports do not confer a native target-marking receipt.
const projectionPorts = Object.freeze({
  read: (value, key) => value[key],
  greater_than: (left, right) => left > right,
  destructure_error: items => {
    // Preserve the engine's exact nullish destructuring TypeError. Rust calls
    // this only for null/undefined; it never substitutes for carrier projection.
    const { filePayloads } = items;
    return filePayloads;
  },
});

module.exports = { projectionPorts };
