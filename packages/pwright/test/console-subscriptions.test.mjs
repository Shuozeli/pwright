import assert from 'node:assert/strict';
import { test } from 'node:test';
import NativeConsoleEventEmitter from '../src/console-subscriptions.cjs';

function fixture() {
  const transitions = [];
  const session = {
    setPageConsoleListener(id, enabled) { transitions.push([id, enabled]); },
  };
  return {
    emitter: new NativeConsoleEventEmitter(session, '7', 'setPageConsoleListener'),
    transitions,
  };
}

test('console listener registration mirrors only zero-to-one and one-to-zero transitions', () => {
  // Arrange
  const { emitter, transitions } = fixture();
  const first = () => {};
  const second = () => {};

  // Act
  emitter.on('console', first);
  emitter.prependListener('console', second);
  emitter.removeListener('console', first);
  emitter.off('console', second);

  // Assert
  assert.deepEqual(transitions, [['7', true], ['7', false]]);
});

test('once and removeAllListeners release and permit a later native subscription', () => {
  // Arrange
  const { emitter, transitions } = fixture();
  emitter.once('console', () => {});

  // Act
  emitter.emit('console', { text: 'first' });
  emitter.on('console', () => {});
  emitter.removeAllListeners();
  emitter.on('console', () => {});

  // Assert
  assert.deepEqual(transitions, [
    ['7', true],
    ['7', false],
    ['7', true],
    ['7', false],
    ['7', true],
  ]);
});

test('profiles without console-events retain ordinary EventEmitter behavior', () => {
  // Arrange
  const emitter = new NativeConsoleEventEmitter({}, '7', 'setPageConsoleListener');
  const calls = [];

  // Act
  emitter.on('console', value => calls.push(value));
  emitter.emit('console', 'message');

  // Assert
  assert.deepEqual(calls, ['message']);
});
