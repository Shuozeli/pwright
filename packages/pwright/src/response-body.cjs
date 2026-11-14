'use strict';

module.exports = ({ mapError, UnsupportedOperationError }) => Object.freeze({
  create(session, pageId, frameId, requestId, beforeRead = () => Promise.resolve()) {
    let pending;
    const body = () => {
      if (!pending) pending = (async () => {
        if (typeof session.frameResponseBody !== 'function')
          throw new UnsupportedOperationError('response.body (select response-body native profile)');
        if (typeof requestId !== 'string' || requestId.length === 0)
          throw new UnsupportedOperationError('response.body (navigation request identity unavailable)');
        try {
          await beforeRead();
          return await session.frameResponseBody(pageId, frameId, requestId);
        } catch (error) {
          throw mapError(error, 'response.body');
        }
      })();
      return pending;
    };
    const text = async () => (await body()).toString('utf8');
    const json = async () => JSON.parse(await text());
    return Object.freeze({ body, text, json });
  },
});
