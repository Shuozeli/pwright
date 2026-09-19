import assert from 'node:assert/strict';
import { test } from 'node:test';
import { findFixturePage, fixtureTitle } from './fixture-selection.mjs';

const fixtureUrl = 'https://fixture.invalid/test-owned';

test('fixture discovery does not probe unrelated Page titles', async () => {
  // Arrange
  let unrelatedReads = 0;
  const unrelated = { url() { return 'https://unrelated.invalid/'; },
    async title() { unrelatedReads += 1; throw new Error('execution context destroyed'); } };
  const fixture = { url() { return fixtureUrl; }, async title() { return fixtureTitle; } };
  // Act
  const selected = await findFixturePage([unrelated, fixture], fixtureUrl);
  // Assert
  assert.equal(selected, fixture);
  assert.equal(unrelatedReads, 0);
});

test('fixture discovery never hides a failed title read on the actual fixture', async () => {
  // Arrange
  const fixture = { url() { return fixtureUrl; }, async title() { throw new Error('fixture read failed'); } };
  // Act
  const selected = findFixturePage([fixture], fixtureUrl);
  // Assert
  await assert.rejects(selected, { code: 'ERR_ASSERTION' });
});

test('fixture discovery rejects stale duplicate fixture Pages', async () => {
  // Arrange
  const stale = { url() { return fixtureUrl; }, async title() { return fixtureTitle; } };
  const fixture = { url() { return fixtureUrl; }, async title() { return fixtureTitle; } };
  // Act
  const selected = findFixturePage([stale, fixture], fixtureUrl);
  // Assert
  await assert.rejects(selected, { code: 'ERR_ASSERTION' });
});

test('fixture discovery rejects a marker at the wrong URL', async () => {
  // Arrange
  const stale = { url() { return 'https://fixture.invalid/stale'; },
    async title() { return fixtureTitle; } };
  // Act
  const selected = findFixturePage([stale], fixtureUrl);
  // Assert
  await assert.rejects(selected, { code: 'ERR_ASSERTION' });
});

test('fixture discovery requires an explicit owner URL', async () => {
  // Arrange
  const pages = [];
  // Act
  const selected = findFixturePage(pages);
  // Assert
  await assert.rejects(selected, { code: 'ERR_ASSERTION' });
});
