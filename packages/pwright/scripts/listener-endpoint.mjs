import assert from 'node:assert/strict';

// Test-only override for the existing loopback CDP relay, never a production
// endpoint fallback or a way to select a different browser/profile remotely.
export function listenerEndpoint(endpoint, relay) {
  assert.equal(endpoint, 'http://127.0.0.1:9222');
  if (relay === undefined) return endpoint;
  assert.equal(typeof relay, 'string');
  const match = /^ws:\/\/127\.0\.0\.1:([1-9][0-9]{0,4})$/.exec(relay);
  assert.ok(match && Number(match[1]) <= 65535, 'diagnostic relay must be a loopback-only WebSocket listener');
  return relay;
}
