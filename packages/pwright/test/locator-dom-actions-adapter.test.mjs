import assert from 'node:assert/strict';
import { test } from 'node:test';
import handleFactory from '../src/handles.cjs';
import locatorFactory from '../src/locator.cjs';

function locatorWithDispatch(nativeOverrides = {}, handleKind) {
  const calls = [];
  const native = {
    async act(...args) {
      calls.push(['act', ...args]);
    },
    async dispatchEvent(type, eventInit, timeout) {
      calls.push(['dispatchEvent', type, eventInit, timeout]);
    },
    async selectText(timeout, force) {
      calls.push(['selectText', timeout, force]);
    },
    ...nativeOverrides,
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = { _id: 'frame', page: () => page };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions(options, allowed, name) {
      calls.push(['validateOptions', options, allowed, name]);
      for (const key of Object.keys(options)) {
        if (!allowed.includes(key)) throw new Error(`${name} option ${key} is not supported`);
      }
    },
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
    handleKind,
  });
  const locator = api.create(frame, 'button');
  calls.length = 0;
  return { calls, locator };
}

function retainedHandle(isElement = true) {
  const handles = handleFactory({
    mapError: error => error,
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  const native = {
    isElement: () => isElement,
    preview: () => isElement ? 'OPTION' : 'object',
    bindWrapper() {},
  };
  return { handles, native };
}

test('Locator dispatchEvent delegates the bounded event payload and timeout to Rust', async () => {
  // Arrange
  const value = locatorWithDispatch();
  const eventInit = { detail: 2 };
  const options = { timeout: 73 };

  // Act
  await value.locator.dispatchEvent('click', eventInit, options);

  // Assert
  assert.deepEqual(value.calls, [
    ['validateOptions', options, ['timeout'], 'locator.dispatchEvent'],
    ['dispatchEvent', 'click', eventInit, 73],
  ]);
});

test('Locator dispatchEvent supplies Playwright-shaped defaults to Rust', async () => {
  // Arrange
  const value = locatorWithDispatch();

  // Act
  await value.locator.dispatchEvent('focus');

  // Assert
  assert.deepEqual(value.calls, [
    ['validateOptions', {}, ['timeout'], 'locator.dispatchEvent'],
    ['dispatchEvent', 'focus', {}, 30_000],
  ]);
});

test('Locator dispatchEvent rejects a non-string event type before native execution', async () => {
  // Arrange
  const value = locatorWithDispatch();

  // Act
  const operation = value.locator.dispatchEvent(42);

  // Assert
  await assert.rejects(operation, /type must be a string/);
  assert.deepEqual(value.calls, []);
});

test('Locator selectText delegates force and timeout to the same Rust DOM leaf', async () => {
  // Arrange
  const value = locatorWithDispatch();
  const options = { force: true, timeout: 91 };

  // Act
  await value.locator.selectText(options);

  // Assert
  assert.deepEqual(value.calls, [
    ['validateOptions', options, ['force', 'timeout'], 'locator.selectText'],
    ['selectText', 91, true],
  ]);
});

test('Locator tap forwards the complete Playwright option snapshot to Rust', async () => {
  // Arrange
  const value = locatorWithDispatch();
  const options = {
    force: true,
    modifiers: ['Alt'],
    noWaitAfter: true,
    position: { x: 2.5, y: 3.25 },
    timeout: 91,
    trial: true,
  };

  // Act
  await value.locator.tap(options);

  // Assert
  assert.deepEqual(value.calls[0], [
    'validateOptions',
    options,
    ['timeout', 'force', 'modifiers', 'noWaitAfter', 'position', 'trial'],
    'locator.tap',
  ]);
  assert.deepEqual(value.calls[1], [
    'validateOptions',
    options.position,
    ['x', 'y'],
    'locator.tap position',
  ]);
  assert.deepEqual(value.calls[2], [
    'act',
    'tap',
    91,
    options,
    undefined,
  ]);
});

test('Locator click forwards bounded mouse-movement steps to the native action', async () => {
  // Arrange
  const value = locatorWithDispatch();

  // Act
  for (const steps of [1, 5, 10_000])
    await value.locator.click({ steps, timeout: 91 });

  // Assert
  const actions = value.calls.filter(call => call[0] === 'act');
  assert.equal(actions.length, 3);
  assert.deepEqual(actions.map(call => [call[1], call[2], call[3].steps]), [
    ['click', 91, 1],
    ['click', 91, 5],
    ['click', 91, 10_000],
  ]);
});

test('Locator click preserves the original native option shape when steps are omitted', async () => {
  // Arrange
  const value = locatorWithDispatch();

  // Act
  await value.locator.click({ timeout: 91 });

  // Assert
  const action = value.calls.find(call => call[0] === 'act');
  assert.equal(action[1], 'click');
  assert.equal(action[2], 91);
  assert.equal(Object.hasOwn(action[3], 'steps'), false);
});

test('Locator click rejects invalid steps before native admission', async () => {
  // Arrange
  const value = locatorWithDispatch();
  const invalid = [0, -1, 1.5, 10_001, NaN, Infinity, '5', null];

  // Act and assert
  for (const steps of invalid)
    await assert.rejects(value.locator.click({ steps }), /locator\.click: steps must be an integer in \[1, 10000\]/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});

test('Locator dblclick does not advertise click-only steps', async () => {
  // Arrange
  const value = locatorWithDispatch();

  // Act
  const operation = value.locator.dblclick({ steps: 5 });

  // Assert
  await assert.rejects(operation, /locator\.dblclick option steps is not supported/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});

for (const { name, input, diagnostic } of [
  {
    name: 'null array item',
    input: ['blue', null, 'black'],
    diagnostic: 'options[1]: expected object, got null',
  },
  {
    name: 'number descriptor',
    input: 12,
    diagnostic: 'options[0]: expected object, got number',
  },
  {
    name: 'number value field',
    input: { value: 12 },
    diagnostic: 'options[0].value: expected string, got number',
  },
  {
    name: 'number label field',
    input: { label: 12 },
    diagnostic: 'options[0].label: expected string, got number',
  },
  {
    name: 'string index field',
    input: { index: '12' },
    diagnostic: 'options[0].index: expected integer, got string',
  },
]) {
  test(`Locator selectOption rejects ${name} with a protocol-shaped diagnostic before native admission`, async () => {
    // Arrange
    const value = locatorWithDispatch();

    // Act
    const operation = value.locator.selectOption(input);

    // Assert
    await assert.rejects(operation, error => {
      assert.equal(error.constructor, TypeError);
      assert.equal(error.message, `locator.selectOption: ${diagnostic}`);
      return true;
    });
    assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
  });
}

test('Locator selectOption keeps top-level null as the deselect-all value', async () => {
  // Arrange
  const value = locatorWithDispatch();

  // Act
  await value.locator.selectOption(null);

  // Assert
  assert.deepEqual(value.calls.filter(call => call[0] === 'act').map(call => [call[1], call[4]]), [
    ['selectOption', null],
  ]);
});

test('Locator selectOption forwards the exact retained ElementHandle wrapper to native', async () => {
  // Arrange
  const { handles, native } = retainedHandle();
  const option = await handles.create({}, native);
  const value = locatorWithDispatch({}, handles.handleKind);

  // Act
  await value.locator.selectOption(option);

  // Assert
  const action = value.calls.find(call => call[0] === 'act');
  assert.equal(action[1], 'selectOption');
  assert.strictEqual(action[4], option);
  assert.equal(value.calls.filter(call => call[3] === 'locator.selectOption descriptor').length, 0);
});

test('Locator selectOption rejects a copied ElementHandle prototype before native admission', async () => {
  // Arrange
  const { handles, native } = retainedHandle();
  const option = await handles.create({}, native);
  const forged = Object.create(Object.getPrototypeOf(option));
  const value = locatorWithDispatch({}, handles.handleKind);

  // Act
  const operation = value.locator.selectOption(forged);

  // Assert
  await assert.rejects(operation, /options\[0\]: expected ElementHandle, got unbound wrapper/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});

test('Locator selectOption rejects an ElementHandle without a native wrapper binding', async () => {
  // Arrange
  const { handles, native } = retainedHandle();
  delete native.bindWrapper;
  const option = await handles.create({}, native);
  const value = locatorWithDispatch({}, handles.handleKind);

  // Act
  const operation = value.locator.selectOption(option);

  // Assert
  await assert.rejects(operation, /options\[0\]: expected ElementHandle, got unbound wrapper/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});

test('Locator selectOption rejects a non-element JSHandle before native admission', async () => {
  // Arrange
  const { handles, native } = retainedHandle(false);
  const other = await handles.create({}, native);
  const value = locatorWithDispatch({}, handles.handleKind);

  // Act
  const operation = value.locator.selectOption(other);

  // Assert
  await assert.rejects(operation, /options\[0\]: expected ElementHandle, got JSHandle/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});

test('Locator selectOption rejects a handle from another native module before native admission', async () => {
  // Arrange
  const foreign = Object.defineProperty({}, Symbol.toStringTag, { value: 'PwrightJSHandle' });
  const value = locatorWithDispatch();

  // Act
  const operation = value.locator.selectOption(foreign);

  // Assert
  await assert.rejects(operation, /ElementHandle belongs to a different native module/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});

test('Locator selectOption rejects ElementHandle arrays instead of treating them as descriptors', async () => {
  // Arrange
  const { handles, native } = retainedHandle();
  const option = await handles.create({}, native);
  const value = locatorWithDispatch({}, handles.handleKind);

  // Act
  const operation = value.locator.selectOption([option]);

  // Assert
  await assert.rejects(operation, /options\[0\]: ElementHandle arrays are not supported/);
  assert.deepEqual(value.calls.filter(call => call[0] === 'act'), []);
});
