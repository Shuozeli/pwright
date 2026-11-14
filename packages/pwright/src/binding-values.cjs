'use strict';

const MAX_DEPTH = 64;
const MAX_NODES = 4096;
const MAX_ENTRIES = 4096;
const MAX_STRING_BYTES = 64 * 1024;

const typedArrays = Object.freeze({
  i8: Int8Array,
  ui8: Uint8Array,
  ui8c: Uint8ClampedArray,
  i16: Int16Array,
  ui16: Uint16Array,
  i32: Int32Array,
  ui32: Uint32Array,
  f32: Float32Array,
  f64: Float64Array,
  ...(typeof BigInt64Array === 'undefined' ? {} : { bi64: BigInt64Array }),
  ...(typeof BigUint64Array === 'undefined' ? {} : { bui64: BigUint64Array }),
});

function state() {
  return { nodes: 0, entries: 0, stringBytes: 0 };
}

function claimNode(limits, depth) {
  if (depth > MAX_DEPTH) throw new Error('binding value depth limit exceeded');
  if (++limits.nodes > MAX_NODES) throw new Error('binding value node limit exceeded');
}

function claimEntry(limits) {
  if (++limits.entries > MAX_ENTRIES) throw new Error('binding value entry limit exceeded');
}

function claimString(limits, value) {
  limits.stringBytes += Buffer.byteLength(value, 'utf8');
  if (limits.stringBytes > MAX_STRING_BYTES) throw new Error('binding value string-byte limit exceeded');
}

function parseValue(value, refs = new Map(), limits = state(), depth = 0) {
  claimNode(limits, depth);
  if (value === undefined || typeof value === 'boolean' || typeof value === 'number') return value;
  if (typeof value === 'string') {
    claimString(limits, value);
    return value;
  }
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new TypeError('invalid serialized binding value');
  if (Object.hasOwn(value, 'ref')) {
    if (!Number.isInteger(value.ref) || !refs.has(value.ref))
      throw new TypeError('unknown serialized binding reference');
    return refs.get(value.ref);
  }
  if (Object.hasOwn(value, 'v')) {
    if (value.v === 'undefined') return undefined;
    if (value.v === 'null') return null;
    if (value.v === 'NaN') return NaN;
    if (value.v === 'Infinity') return Infinity;
    if (value.v === '-Infinity') return -Infinity;
    if (value.v === '-0') return -0;
    throw new TypeError('unknown serialized binding primitive');
  }
  if (Object.hasOwn(value, 'd')) return new Date(value.d);
  if (Object.hasOwn(value, 'u')) return new URL(value.u);
  if (Object.hasOwn(value, 'bi')) return BigInt(value.bi);
  if (Object.hasOwn(value, 'e')) {
    const error = new Error(value.e?.m);
    error.name = value.e?.n;
    error.stack = value.e?.s;
    return error;
  }
  if (Object.hasOwn(value, 'r')) return new RegExp(value.r?.p, value.r?.f);
  if (Object.hasOwn(value, 'ta')) {
    const Constructor = typedArrays[value.ta?.k];
    if (!Constructor) throw new TypeError('unknown serialized typed array');
    const bytes = Buffer.from(value.ta.b, 'base64');
    if (bytes.byteLength % Constructor.BYTES_PER_ELEMENT)
      throw new TypeError('misaligned serialized typed array');
    const copy = Uint8Array.from(bytes);
    return new Constructor(copy.buffer, copy.byteOffset, copy.byteLength / Constructor.BYTES_PER_ELEMENT);
  }
  if (Object.hasOwn(value, 'a')) {
    if (!Array.isArray(value.a) || !Number.isInteger(value.id))
      throw new TypeError('invalid serialized binding array');
    if (refs.has(value.id)) throw new TypeError('duplicate serialized binding reference');
    const result = [];
    refs.set(value.id, result);
    for (const item of value.a) {
      claimEntry(limits);
      result.push(parseValue(item, refs, limits, depth + 1));
    }
    return result;
  }
  if (Object.hasOwn(value, 'o')) {
    if (!Array.isArray(value.o) || !Number.isInteger(value.id))
      throw new TypeError('invalid serialized binding object');
    if (refs.has(value.id)) throw new TypeError('duplicate serialized binding reference');
    const result = {};
    refs.set(value.id, result);
    for (const property of value.o) {
      claimEntry(limits);
      if (!property || typeof property.k !== 'string' || !Object.hasOwn(property, 'v'))
        throw new TypeError('invalid serialized binding property');
      claimString(limits, property.k);
      if (property.k !== '__proto__')
        result[property.k] = parseValue(property.v, refs, limits, depth + 1);
    }
    return result;
  }
  throw new TypeError('unknown serialized binding value');
}

function parseArguments(serialized) {
  if (!Array.isArray(serialized))
    throw new Error('serializedArgs is not an array. This can happen when Array.prototype.toJSON is defined incorrectly');
  const limits = state();
  return serialized.map(value => parseValue(value, new Map(), limits));
}

module.exports = { parseArguments, parseValue };
