'use strict';

const { channelPorts } = require('./file-input-channel.cjs');

// JavaScript language primitives only. Rust selects the calls, order, branches,
// size limit and errors; no upload, file-system or browser policy lives here.
const ports = Object.freeze({
  owner_frame: receiver => receiver.ownerFrame(),
  page: owner => owner.page(),
  context: page => page.context(),
  is_array: files => Array.isArray(files),
  slice: files => files.slice(),
  some: (items, predicate) => items.some(predicate),
  every: (items, predicate) => items.every(predicate),
  reduce: (items, reducer, initial) => items.reduce(reducer, initial),
  buffer: item => item.buffer,
  byte_length: buffer => buffer.byteLength,
  add: (left, right) => left + right,
  at_least: (left, right) => left >= right,
  unary: native => value => native(value),
  // A user-supplied array method can retain a callback. JavaScript owns its
  // ports and forwards them on every invocation; Rust captures no scoped Env.
  binary: (native, buffer, byteLength, add) => (size, item) => native(size, item, buffer, byteLength, add),
  error: message => new Error(message),
});

// CopyDataProperties and one member read only. Rust controls merge order and
// the live client-settings traversal; JavaScript does not choose a timeout.
const argumentPorts = Object.freeze({
  spread: (target, source) => ({ ...target, ...source }),
  timeout: options => options.timeout,
});

async function asyncConvert(preparation) {
  // Async-return assimilation applies to the ordinary converted carrier, not
  // to a ticket or private metadata record. Even empty input crosses this await.
  return preparation.convertInput(ports);
}

async function preflight(preparation, includeChannel = false, continuation) {
  try {
    const owner = await preparation.beginOwnerLookup(ports);
    // Upstream evaluates frame.page().context() outside async convertInputFiles.
    preparation.prepareContext(owner, ports);
    const converted = await asyncConvert(preparation);
    preparation.finishConversion(converted);
    preparation.assembleArguments(argumentPorts);
    // Both internal stop points share the same conversion continuation. There
    // is no await/Promise assimilation of channel params between these phases.
    if (includeChannel) preparation.validateChannel(channelPorts);
    // A selected server continuation schedules delivery in THIS continuation,
    // without first awaiting a Promise that contains validated params/ticket.
    if (continuation) return continuation(preparation);
    // Neither stop point is public setInputFiles: channel API error wrapping,
    // deferred materialization and native submission are still separate gates.
    return preparation;
  } catch (error) {
    preparation.abort();
    throw error;
  }
}

function prepareChannel(preparation) {
  return preflight(preparation, true);
}

function continueAfterChannel(preparation, continuation) {
  return preflight(preparation, true, continuation);
}

module.exports = { ports, argumentPorts, preflight, prepareChannel, continueAfterChannel };
