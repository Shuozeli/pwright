import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createLifecycleTrace } from './lifecycle-trace.mjs';

test('progress is emitted immediately in bounded ordered TAP comments', () => {
  // Arrange
  const lines = [];
  const trace = createLifecycleTrace(line => lines.push(line), () => 17);
  // Act
  trace('test:start', 'close under pressure');
  trace('browser-close:start');
  // Assert
  const records = lines.map(line => JSON.parse(line.slice(2)));
  assert.deepEqual(records.map(record => [record.sequence, record.phase, record.test]), [
    [1, 'test:start', 'close under pressure'], [2, 'browser-close:start', 'close under pressure'],
  ]);
  assert.equal(records[1].elapsedMs, 17);
});

test('trace exhaustion fails before another record is emitted', () => {
  // Arrange
  const lines = [];
  const trace = createLifecycleTrace(line => lines.push(line), () => 0, 1);
  trace('first');
  // Act
  const emit = () => trace('second');
  // Assert
  assert.throws(emit, /budget exhausted/);
  assert.equal(lines.length, 1);
});

test('oversized trace labels fail without output', () => {
  // Arrange
  const lines = [];
  const trace = createLifecycleTrace(line => lines.push(line), () => 0);
  // Act
  const emit = () => trace('test:start', 'x'.repeat(513));
  // Assert
  assert.throws(emit, /name exceeds bound/);
  assert.equal(lines.length, 0);
});
