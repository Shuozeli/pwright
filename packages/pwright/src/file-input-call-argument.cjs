'use strict';

/*
 * Adapted from Playwright 1.58.2 utilityScriptSerializers.ts.
 * Copyright (c) Microsoft Corporation.
 * SPDX-License-Identifier: Apache-2.0
 *
 * This is a synchronous Node-language adapter, not browser authority. Rust
 * owns the retained target, phase ordering and persistent root. Keeping this
 * code in JavaScript preserves the engine's exact getter, toJSON, realm and
 * cycle semantics while avoiding a second copy of large base64 string leaves.
 */

const functionBindingPrefix = '__pw_fn_';

function isRegExp(object) {
  try {
    return object instanceof RegExp || Object.prototype.toString.call(object) === '[object RegExp]';
  } catch {
    return false;
  }
}

function isDate(object) {
  try {
    return object instanceof Date || Object.prototype.toString.call(object) === '[object Date]';
  } catch {
    return false;
  }
}

function isURL(object) {
  try {
    return object instanceof URL || Object.prototype.toString.call(object) === '[object URL]';
  } catch {
    return false;
  }
}

function isError(object) {
  try {
    return object instanceof Error || (object && Object.getPrototypeOf(object)?.name === 'Error');
  } catch {
    return false;
  }
}

function isTypedArray(object, constructor) {
  try {
    return object instanceof constructor ||
      Object.prototype.toString.call(object) === `[object ${constructor.name}]`;
  } catch {
    return false;
  }
}

function isArrayBuffer(object) {
  try {
    return object instanceof ArrayBuffer ||
      Object.prototype.toString.call(object) === '[object ArrayBuffer]';
  } catch {
    return false;
  }
}

const typedArrayConstructors = Object.freeze({
  i8: Int8Array,
  ui8: Uint8Array,
  ui8c: Uint8ClampedArray,
  i16: Int16Array,
  ui16: Uint16Array,
  i32: Int32Array,
  ui32: Uint32Array,
  f32: Float32Array,
  f64: Float64Array,
  bi64: BigInt64Array,
  bui64: BigUint64Array,
});

function typedArrayToBase64(array) {
  if ('toBase64' in array)
    return array.toBase64();
  const binary = Array.from(new Uint8Array(array.buffer, array.byteOffset, array.byteLength))
    .map(byte => String.fromCharCode(byte)).join('');
  return btoa(binary);
}

function serializeFileInputCallArgument(value) {
  return serialize(value, { visited: new Map(), lastId: 0 });
}

function serialize(value, visitorInfo) {
  if (value && typeof value === 'object') {
    if (typeof globalThis.Window === 'function' && value instanceof globalThis.Window)
      return 'ref: <Window>';
    if (typeof globalThis.Document === 'function' && value instanceof globalThis.Document)
      return 'ref: <Document>';
    if (typeof globalThis.Node === 'function' && value instanceof globalThis.Node)
      return 'ref: <Node>';
  }
  return innerSerialize(value, visitorInfo);
}

function innerSerialize(value, visitorInfo) {
  if (typeof value === 'symbol' || Object.is(value, undefined))
    return { v: 'undefined' };
  if (Object.is(value, null))
    return { v: 'null' };
  if (Object.is(value, NaN))
    return { v: 'NaN' };
  if (Object.is(value, Infinity))
    return { v: 'Infinity' };
  if (Object.is(value, -Infinity))
    return { v: '-Infinity' };
  if (Object.is(value, -0))
    return { v: '-0' };
  if (typeof value === 'boolean' || typeof value === 'number' || typeof value === 'string')
    return value;
  if (typeof value === 'bigint')
    return { bi: value.toString() };

  if (isError(value)) {
    let stack;
    if (value.stack?.startsWith(value.name + ': ' + value.message))
      stack = value.stack;
    else
      stack = `${value.name}: ${value.message}\n${value.stack}`;
    return { e: { n: value.name, m: value.message, s: stack } };
  }
  if (isDate(value))
    return { d: value.toJSON() };
  if (isURL(value))
    return { u: value.toJSON() };
  if (isRegExp(value))
    return { r: { p: value.source, f: value.flags } };
  for (const [kind, constructor] of Object.entries(typedArrayConstructors)) {
    if (isTypedArray(value, constructor))
      return { ta: { b: typedArrayToBase64(value), k: kind } };
  }
  if (isArrayBuffer(value))
    return { ab: { b: typedArrayToBase64(new Uint8Array(value)) } };

  const previous = visitorInfo.visited.get(value);
  if (previous)
    return { ref: previous };

  if (Array.isArray(value)) {
    const items = [];
    const id = ++visitorInfo.lastId;
    visitorInfo.visited.set(value, id);
    for (let index = 0; index < value.length; ++index)
      items.push(serialize(value[index], visitorInfo));
    return { a: items, id };
  }

  if (typeof value === 'object') {
    const properties = [];
    const id = ++visitorInfo.lastId;
    visitorInfo.visited.set(value, id);
    for (const name of Object.keys(value)) {
      let item;
      try {
        item = value[name];
      } catch {
        continue;
      }
      if (name === 'toJSON' && typeof item === 'function')
        properties.push({ k: name, v: { o: [], id: 0 } });
      else
        properties.push({ k: name, v: serialize(item, visitorInfo) });
    }

    let jsonWrapper;
    try {
      if (properties.length === 0 && value.toJSON && typeof value.toJSON === 'function')
        jsonWrapper = { value: value.toJSON() };
    } catch {
      // Pinned Playwright ignores a throwing fallback toJSON lookup/call.
    }
    if (jsonWrapper)
      return innerSerialize(jsonWrapper.value, visitorInfo);
    return { o: properties, id };
  }

  if (typeof value === 'function' && value.name.startsWith(functionBindingPrefix))
    return { fn: value.name };
  return undefined;
}

Object.freeze(serializeFileInputCallArgument);
module.exports = { serializeFileInputCallArgument };
