import assert from 'node:assert/strict';

export const fixtureTitle = '__pwright_native_case__';

export async function findFixturePage(pages, fixtureUrl) {
  assert.equal(typeof fixtureUrl, 'string', 'the fixture owner must provide its current URL');
  assert.ok(fixtureUrl.length > 0, 'the fixture URL must not be empty');
  // Read only Pages at the URL supplied by their fixture owner. Shared Chrome
  // can contain unrelated stalled Pages; even a failed title probe there can
  // consume a complete API timeout. The actual fixture must still return its
  // marker exactly once, without retries or normalization of a failed read.
  const candidates = pages.filter(page => page.url() === fixtureUrl);
  const probes = await Promise.allSettled(candidates.map(page => page.title()));
  const matches = candidates.filter((_page, index) =>
    probes[index].status === 'fulfilled' && probes[index].value === fixtureTitle);
  assert.equal(matches.length, 1,
    'the test-owned Page must be readable and unique; stale or unreadable fixture Pages fail setup');
  return matches[0];
}
