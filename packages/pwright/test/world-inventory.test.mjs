import assert from 'node:assert/strict';
import { test } from 'node:test';
import { WorldInventory } from '../scripts/probe-world-inventory.mjs';

const event = (id, name, isDefault = false, frameId = 'owned') =>
  ({ context: { id, name, auxData: { frameId, isDefault } } });

test('world inventory classifies only the owned frame and deduplicates context IDs', () => {
  // Arrange
  const inventory = new WorldInventory('owned');
  const events = [event(1, '', true), event(2, '__pwright_utility_0001'),
    event(2, '__pwright_utility_0001'), event(3, '__playwright_utility_world_page1'),
    event(4, 'extension'), event(5, '', true, 'foreign')];
  // Act
  events.forEach(value => inventory.created(value));
  // Assert
  assert.deepEqual(inventory.snapshot(), { counts: { main: 1, pwright: 1, playwright: 1, other: 1 },
    pwrightNames: ['__pwright_utility_0001'], mainContextIds: [1], overflow: false });
});

test('world inventory rejects oversized identities before retaining them', () => {
  // Arrange
  const inventory = new WorldInventory('owned');
  // Act
  inventory.created(event(1, 'x'.repeat(129)));
  // Assert
  assert.deepEqual(inventory.snapshot(), { counts: { main: 0, pwright: 0, playwright: 0, other: 0 },
    pwrightNames: [], mainContextIds: [], overflow: true });
});

test('world inventory rejects invalid context IDs', () => {
  // Arrange
  const inventory = new WorldInventory('owned');
  // Act
  inventory.created(event(Number.MAX_SAFE_INTEGER + 1, ''));
  // Assert
  assert.equal(inventory.snapshot().overflow, true);
  assert.equal(inventory.snapshot().counts.other, 0);
});

test('world inventory stops retaining new contexts at the fixed capacity', () => {
  // Arrange
  const inventory = new WorldInventory('owned');
  for (let id = 0; id < 512; id++) inventory.created(event(id, 'extension'));
  // Act
  inventory.created(event(512, 'extension'));
  // Assert
  assert.equal(inventory.snapshot().overflow, true);
  assert.equal(inventory.snapshot().counts.other, 512);
});

test('reset clears context storage but cannot hide an earlier diagnostic overflow', () => {
  // Arrange
  const inventory = new WorldInventory('owned');
  inventory.created(event(1, '', true));
  inventory.created(event(2, 'x'.repeat(129)));
  // Act
  inventory.reset();
  // Assert
  assert.equal(inventory.snapshot().counts.main, 0);
  assert.equal(inventory.snapshot().overflow, true);
});
