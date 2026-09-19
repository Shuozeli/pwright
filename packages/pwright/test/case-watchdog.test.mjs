import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createCaseWatchdog } from '../scripts/case-watchdog.mjs';

function setup() {
  const timers = new Map();
  const errors = [];
  let next = 0;
  const watchdog = createCaseWatchdog({
    schedule: (callback, delay) => { const id = ++next; timers.set(id, { callback, delay }); return id; },
    cancel: id => timers.delete(id), fail: reason => errors.push(reason),
  });
  return { timers, errors, watchdog };
}

test('each case gets an independent bound and progress phases cannot extend it', () => {
  // Arrange
  const { timers, errors, watchdog } = setup();
  // Act
  watchdog.message({ event: 'pwright-case', phase: 'start', name: 'close' });
  const [timer] = timers.values();
  timer.callback();
  watchdog.close();
  // Assert
  assert.equal(timer.delay, 60_000);
  assert.deepEqual(errors, ['case watchdog expired: close']);
  assert.equal(timers.size, 0);
});

test('normal completion replaces the case deadline with the next-case exit bound', () => {
  // Arrange
  const { timers, errors, watchdog } = setup();
  watchdog.message({ event: 'pwright-case', phase: 'start', name: 'read' });
  // Act
  watchdog.message({ event: 'pwright-case', phase: 'end', name: 'read' });
  const count = timers.size;
  watchdog.close();
  // Assert
  assert.equal(count, 1);
  assert.equal(timers.size, 0);
  assert.deepEqual(errors, []);
});

test('out-of-order and oversized case announcements fail explicitly', () => {
  // Arrange
  const { errors, watchdog } = setup();
  // Act
  watchdog.message({ event: 'pwright-case', phase: 'end', name: 'never started' });
  watchdog.message({ event: 'pwright-case', phase: 'start', name: 'x'.repeat(513) });
  watchdog.close();
  // Assert
  assert.deepEqual(errors, ['case end does not match active case', 'invalid case watchdog message']);
});

test('startup is bounded even if the child never announces a case', () => {
  // Arrange
  const { timers, errors, watchdog } = setup();
  const [timer] = timers.values();
  // Act
  timer.callback();
  watchdog.close();
  // Assert
  assert.equal(timer.delay, 60_000);
  assert.deepEqual(errors, ['case watchdog expired: startup']);
  assert.equal(timers.size, 0);
});

test('a second start cannot renew an active case deadline', () => {
  // Arrange
  const { timers, errors, watchdog } = setup();
  watchdog.message({ event: 'pwright-case', phase: 'start', name: 'active' });
  const [original] = timers.values();
  // Act
  watchdog.message({ event: 'pwright-case', phase: 'start', name: 'active' });
  const [remaining] = timers.values();
  watchdog.close();
  // Assert
  assert.equal(remaining, original);
  assert.deepEqual(errors, ['invalid case start order or capacity']);
});

test('the fixed case capacity rejects the next announcement', () => {
  // Arrange
  const { errors, watchdog } = setup();
  for (let index = 0; index < 128; index++) {
    const name = `case-${index}`;
    watchdog.message({ event: 'pwright-case', phase: 'start', name });
    watchdog.message({ event: 'pwright-case', phase: 'end', name });
  }
  assert.deepEqual(errors, []);
  // Act
  watchdog.message({ event: 'pwright-case', phase: 'start', name: 'over-capacity' });
  watchdog.close();
  // Assert
  assert.deepEqual(errors, ['invalid case start order or capacity']);
});
