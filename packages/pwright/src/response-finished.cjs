'use strict';

// The opaque owner is retained by the native runtime until Chromium publishes
// the terminal request event. This adapter never polls or recreates lifecycle
// state in JavaScript.
module.exports = ({ mapError, UnsupportedOperationError }) => Object.freeze({
  create(session, owner) {
    let pending;
    const wait = async api => {
      if (!owner)
        throw new UnsupportedOperationError(`${api} (select response-finished native profile)`);
      if (typeof session.responseFinished !== 'function')
        throw new UnsupportedOperationError(`${api} (select response-finished native profile)`);
      pending ??= Promise.resolve().then(() => session.responseFinished(owner));
      try { await pending; }
      catch (error) { throw mapError(error, api); }
    };
    return Object.freeze({
      finished: async () => { await wait('response.finished'); return null; },
      beforeBody: () => owner ? wait('response.body') : Promise.resolve(),
    });
  },
});
