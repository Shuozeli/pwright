import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { setImmediate as defer } from 'node:timers/promises';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import lifecycleEvents from '../src/lifecycle-events.cjs';

const require = createRequire(import.meta.url);
const Dialog = require('../src/dialog.cjs')({
  mapError(error, api) {
    const mapped = new Error(`${api}: ${error.message}`);
    mapped.cause = error;
    return mapped;
  },
});

function fixture() {
  const calls = [];
  const session = {
    async handleDialog(...args) { calls.push(args); },
  };
  const browser = { _session: session, _contexts: new Map() };
  const context = Object.assign(new EventEmitter(), { _browser: browser, _pages: new Map() });
  const page = Object.assign(new EventEmitter(), { _context: context });
  context._pages.set('2', page);
  browser._contexts.set('1', context);
  const prepare = lifecycleEvents(new WeakRef(browser), {
    dialog: (...args) => new Dialog(...args),
  });
  return { calls, browser, context, page, prepare };
}

test('Dialog preserves metadata and delegates each action to its native Rust owner', async () => {
  // Arrange
  const { calls, page } = fixture();
  const native = { owner: 7 };
  const dialog = new Dialog(page, native, 'prompt', 'name?', 'Ada');

  // Act
  await dialog.accept('Grace');

  // Assert
  assert.equal(dialog.page(), page);
  assert.equal(dialog.type(), 'prompt');
  assert.equal(dialog.message(), 'name?');
  assert.equal(dialog.defaultValue(), 'Ada');
  assert.deepEqual(calls, [[native, true, 'Grace']]);
});

test('Dialog validates prompt text and maps native dismiss failures with the public API name', async () => {
  // Arrange
  const { page } = fixture();
  page._context._browser._session.handleDialog = async () => { throw new Error('closed'); };
  const dialog = new Dialog(page, {}, 'alert', 'stop', '');

  // Act
  const invalid = dialog.accept(42);
  const failed = dialog.dismiss();

  // Assert
  await assert.rejects(invalid, /dialog\.accept: promptText must be a string/);
  await assert.rejects(failed, /dialog\.dismiss: closed/);
});

test('Dialog lifecycle notice fans one identity out to Page and Context without auto handling', () => {
  // Arrange
  const { calls, context, page, prepare } = fixture();
  const received = [];
  page.on('dialog', value => received.push(['page', value]));
  context.on('dialog', value => received.push(['context', value]));
  const native = { owner: 8 };
  const emit = prepare(9, '1', '2', undefined, undefined, undefined, undefined,
    undefined, undefined, undefined, native, 'confirm', 'continue?', '');

  // Act
  emit();

  // Assert
  assert.equal(received.length, 2);
  assert.equal(received[0][1], received[1][1]);
  assert.equal(received[0][1].page(), page);
  assert.equal(received[0][1].type(), 'confirm');
  assert.deepEqual(calls, []);
});

test('Unobserved dialog is auto-dismissed after ordered lifecycle publication', async () => {
  // Arrange
  const { calls, prepare } = fixture();
  const native = { owner: 9 };
  const emit = prepare(9, '1', '2', undefined, undefined, undefined, undefined,
    undefined, undefined, undefined, native, 'alert', 'stop', '');

  // Act
  emit();
  await defer();

  // Assert
  assert.deepEqual(calls, [[native, false, undefined]]);
});

test('Unobserved beforeunload dialog is auto-accepted', async () => {
  // Arrange
  const { calls, prepare } = fixture();
  const native = { owner: 10 };
  const emit = prepare(9, '1', '2', undefined, undefined, undefined, undefined,
    undefined, undefined, undefined, native, 'beforeunload', 'leave?', '');

  // Act
  emit();
  await defer();

  // Assert
  assert.deepEqual(calls, [[native, true, undefined]]);
});
