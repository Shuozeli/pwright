'use strict';

module.exports = function responseRequest({ Request, UnsupportedOperationError, canonicalizeRequest }) {
  function create(page, metadata, historyOwner = null) {
    if (metadata === undefined || metadata === null) return null;
    return canonicalizeRequest
      ? canonicalizeRequest(page, metadata, historyOwner)
      : new Request(page, metadata, historyOwner);
  }

  function get(request) {
    if (request === null)
      throw new UnsupportedOperationError('response.request (select response-request native profile)');
    return request;
  }

  return { create, get };
};
