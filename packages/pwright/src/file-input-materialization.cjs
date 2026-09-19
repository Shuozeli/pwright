'use strict';

// Language intrinsics only. Rust selects the regex sources, traversal,
// accepted input mode, MIME table, fallback and serialization encoding.
// Capturing RegExp matches literals not consulting a later global override.
// Primordial constructor/Reflect tampering before this module loads is outside
// this selected boundary; ordinary String/replace/lowercase overrides remain
// observable at each invocation.
const RegExpConstructor = RegExp;
const ownKeys = Reflect.ownKeys;

const materializationPorts = Object.freeze({
  read: (value, key) => value[key],
  filter_truthy: values => values.filter(Boolean),
  equal: (left, right) => left === right,
  map: (values, callback) => values.map(callback),
  // Fresh unary arrow; escaped callbacks get this invocation's value/ports.
  mapper: (native, ports) => payload => native(payload, ports),
  string: value => String(value),
  replace: (value, pattern, replacement) => value.replace(new RegExpConstructor(pattern), replacement),
  lower_case: value => value.toLowerCase(),
  less_than: (left, right) => left < right,
  subtract: (left, right) => left - right,
  // Computed own keys perform ToPropertyKey, including an object's string-hint
  // conversion and Symbol preservation. No MIME lookup or selection lives here.
  property_key: value => ownKeys({ [value]: 0 })[0],
  serialize: (value, encoding) => value.toString(encoding),
  error: message => new Error(message),
});

async function asyncMaterialize(preparation) {
  // Assimilate the OUTER ordinary carrier only. Nested serializer/map values
  // are deliberately neither awaited nor normalized by this language wrapper.
  return preparation.beginMaterialization(materializationPorts);
}

module.exports = { materializationPorts, asyncMaterialize };
