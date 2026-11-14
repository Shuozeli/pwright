'use strict';

// Rust owns navigation, deadline and cancellation semantics. This adapter adds
// only the Playwright language-facing call-log presentation to a typed native
// timeout or backend failure after the public URL has been resolved.
module.exports = ({ mapError, TimeoutError }) => function navigationError(error, api, url, waitUntil) {
  const mapped = mapError(error, api);
  if ((mapped instanceof TimeoutError && mapped.code === 'PWRIGHT_TIMEOUT')
    || mapped.code === 'PWRIGHT_BACKEND')
    mapped.message += `\nCall log:\n  - navigating to "${url}", waiting until "${waitUntil}"`;
  return mapped;
};
