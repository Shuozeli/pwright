'use strict';

// Rust retains the per-request response and charges the FFI copy. This leaf
// only applies Playwright's public header folding to the ordered native pairs.
module.exports = ({ mapError, UnsupportedOperationError }) => {
  async function groupedHeaders(session, historyOwner, api) {
    if (historyOwner == null || typeof session.responseRawHeaders !== 'function')
      throw new UnsupportedOperationError(`${api} (select route-request-history native profile)`);
    let pairs;
    try { pairs = await session.responseRawHeaders(historyOwner); }
    catch (error) { throw mapError(error, api); }
    const grouped = new Map();
    for (const { name, value } of pairs) {
      const key = name.toLowerCase();
      const values = grouped.get(key);
      if (values) values.push(value);
      else grouped.set(key, [value]);
    }
    return grouped;
  }

  const folded = (name, values) => values.join(name === 'set-cookie' ? '\n' : ', ');
  return Object.freeze({
    async all(session, historyOwner) {
      const grouped = await groupedHeaders(session, historyOwner, 'response.allHeaders');
      return Object.fromEntries([...grouped].map(([name, values]) => [name, folded(name, values)]));
    },
    async value(session, historyOwner, name) {
      const api = 'response.headerValue';
      if (historyOwner == null || typeof session.responseRawHeaders !== 'function')
        throw new UnsupportedOperationError(`${api} (select route-request-history native profile)`);
      if (typeof name !== 'string') throw new TypeError(`${api}: name must be a string`);
      const key = name.toLowerCase();
      const grouped = await groupedHeaders(session, historyOwner, api);
      const values = grouped.get(key);
      return values ? folded(key, values) : null;
    },
  });
};
