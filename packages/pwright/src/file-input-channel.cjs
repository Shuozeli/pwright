'use strict';

class ValidationError extends Error {}

// Single language operations only. Rust owns the schema, traversal order,
// optional-field policy and validation phase; no transport or upload runs here.
const channelPorts = Object.freeze({
  read: (object, key) => object[key],
  assign: (target, key, value) => { target[key] = value; },
  is_undefined: value => Object.is(value, undefined),
  is_null: value => Object.is(value, null),
  is_array: value => Array.isArray(value),
  is_string_object: value => value instanceof String,
  is_number_object: value => value instanceof Number,
  is_buffer: value => value instanceof Buffer,
  value_of: arg => arg.valueOf(),
  map: (arg, callback) => arg.map(callback),
  add: (left, right) => left + right,
  // Escaped callbacks retain JS ports/path, never scoped native Env/values or
  // a preparation ticket. Every invocation forwards its current language data.
  mapper: (native, path, ports) => (value, index) => native(value, index, path, ports),
  error: message => new ValidationError(message),
});

// Server-only interpolation primitive. Rust still owns GUID read order and
// fail-closed registry policy; this performs no lookup or channel validation.
const serverValidationPorts = Object.freeze({
  ...channelPorts,
  template: value => `${value}`,
});

module.exports = { channelPorts, serverValidationPorts };
