import assert from 'node:assert/strict';
import { test } from 'node:test';
import locatorFactory from '../src/locator.cjs';

function locatorWithCount(count) {
  const calls = [];
  const makeNative = index => ({
    nth(value) { calls.push(['nth', value]); return makeNative(value); },
    async read(operation) {
      calls.push(['read', operation, index]);
      if (operation === 'count') return count;
      if (operation === 'textContent') return String.fromCharCode(65 + index);
      throw new Error(`unexpected read ${operation}`);
    },
    async countWithHook(hook) {
      calls.push(['countWithHook:start', index]);
      await hook();
      calls.push(['countWithHook:end', index]);
      return count;
    },
  });
  const session = { locator: () => makeNative(-1) };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = { _id: 'frame', page: () => page };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  return { calls, locator: api.create(frame, 'p') };
}

function locatorWithEvaluation() {
  const calls = [];
  const native = {
    async evaluate(timeout, converter) {
      calls.push(['evaluate', timeout, await converter()]);
      return 'one';
    },
    async evaluateFirst(timeout, converter) {
      calls.push(['evaluateFirst', timeout, await converter()]);
      return 'first';
    },
    async evaluateAll(converter) {
      calls.push(['evaluateAll', await converter()]);
      return ['one', 'two'];
    },
    async evaluateHandle(timeout, converter) {
      calls.push(['evaluateHandle', timeout, await converter()]);
      return { id: 'native-result' };
    },
    async waitFor(state, timeout) {
      calls.push(['waitFor', state, timeout]);
    },
    async boundingBox(timeout) {
      calls.push(['boundingBox', timeout]);
      return { x: 1, y: 2, width: 3, height: 4 };
    },
    async highlight(description) {
      calls.push(['highlight', description]);
    },
    async expectVisibility(timeout, visible, isNot) {
      calls.push(['expectVisibility', timeout, visible, isNot]);
      return { matches: visible, received: visible, timedOut: false };
    },
    async expectState(timeout, state, isNot) {
      calls.push(['expectState', timeout, state, isNot]);
      return state === 'empty'
        ? { matches: true, received: 'empty', timedOut: false }
        : state === 'enabled'
          ? { matches: false, received: 'disabled', preview: '<button disabled>Text</button>', timedOut: true }
        : { matches: false, received: undefined, timedOut: true };
    },
    async expectChecked(timeout, checked, indeterminate, isNot) {
      calls.push(['expectChecked', timeout, checked, indeterminate, isNot]);
      return { matches: true, received: indeterminate ? 'mixed' : checked === false ? 'unchecked' : 'checked', timedOut: false };
    },
    async expectProperty(timeout, name, expected, isNot) {
      calls.push(['expectProperty', timeout, name, expected, isNot]);
      return name === 'missing'
        ? { matches: isNot, missing: true, timedOut: true }
        : { matches: true, received: expected, missing: false, timedOut: false };
    },
    async expectCount(timeout, expected, isNot) {
      calls.push(['expectCount', timeout, expected, isNot]);
      return { matches: true, received: expected, timedOut: false };
    },
    async expectTextRegex(timeout, source, flags, ignoreCase, useInnerText, isNot) {
      calls.push(['expectTextRegex', timeout, source, flags, ignoreCase, useInnerText, isNot]);
      return { matches: false, received: undefined, timedOut: true };
    },
    async expectTexts(timeout, expected, substring, normalizeWhitespace, ignoreCase, useInnerText, isNot) {
      calls.push(['expectTexts', timeout, expected, substring, normalizeWhitespace, ignoreCase, useInnerText, isNot]);
      return { matches: true, received: expected, timedOut: false };
    },
    async expectString(timeout, kind, expected, flags, isNot) {
      calls.push(['expectString', timeout, kind, expected, flags, isNot]);
      return { matches: true, received: expected, missing: false, timedOut: false };
    },
    async expectClassMatchers(timeout, expected, isNot) {
      calls.push(['expectClassMatchers', timeout, expected, isNot]);
      return { matches: true, received: expected.map(value => value.expected), timedOut: false };
    },
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = {
    _id: 'frame',
    page: () => page,
    async _wrapNativeHandle(value) {
      calls.push(['wrapHandle', value]);
      return { id: 'public-result' };
    },
  };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  return { calls, locator: api.create(frame, 'p') };
}

test('Locator all derives one bounded nth recipe per Rust count result', async () => {
  // Arrange
  const value = locatorWithCount(3);

  // Act
  const locators = await value.locator.all();
  const texts = await Promise.all(locators.map(locator => locator.textContent()));

  // Assert
  assert.deepEqual(texts, ['A', 'B', 'C']);
  assert.deepEqual(value.calls, [
    ['read', 'count', -1], ['nth', 0], ['nth', 1], ['nth', 2],
    ['read', 'textContent', 0], ['read', 'textContent', 1], ['read', 'textContent', 2],
  ]);
});

test('Locator all rejects an oversized result before deriving any nth recipe', async () => {
  // Arrange
  const value = locatorWithCount(1025);

  // Act
  const operation = value.locator.all();

  // Assert
  await assert.rejects(operation, /fixed 1024-item boundary/);
  assert.deepEqual(value.calls, [['read', 'count', -1]]);
});

test('Locator private count hook is awaited inside the native query boundary', async () => {
  // Arrange
  const value = locatorWithCount(0);

  // Act
  const count = await value.locator.count({
    __testHookBeforeQuery: async () => {
      value.calls.push(['hook:start']);
      await Promise.resolve();
      value.calls.push(['hook:end']);
    },
  });

  // Assert
  assert.equal(count, 0);
  assert.deepEqual(value.calls, [
    ['countWithHook:start', -1],
    ['hook:start'],
    ['hook:end'],
    ['countWithHook:end', -1],
  ]);
});

test('Locator evaluateFunction forces a string declaration through the Rust function lane', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._evaluateFunction('node => node.textContent');

  // Assert
  assert.equal(result, 'one');
  assert.deepEqual(value.calls, [[
    'evaluate', 30_000,
    { source: 'node => node.textContent', isFunction: true, argument: undefined },
  ]]);
});

test('Locator evaluateFirstFunction delegates non-strict selector evaluation to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();
  const callback = node => node.nodeName;

  // Act
  const result = await value.locator._evaluateFirstFunction(callback, { suffix: '!' });

  // Assert
  assert.equal(result, 'first');
  assert.deepEqual(value.calls, [[
    'evaluateFirst', 30_000,
    { source: String(callback), isFunction: true, argument: { suffix: '!' } },
  ]]);
});

test('Locator evaluateHandle delegates strict resolution and Handle creation to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();
  const callback = node => node.firstChild;

  // Act
  const result = await value.locator.evaluateHandle(callback, undefined, { timeout: 73 });

  // Assert
  assert.deepEqual(result, { id: 'public-result' });
  assert.deepEqual(value.calls, [
    ['evaluateHandle', 73, { source: String(callback), isFunction: true, argument: undefined }],
    ['wrapHandle', { id: 'native-result' }],
  ]);
});

test('Locator descriptions compose structurally and custom descriptions reset after chaining', () => {
  // Arrange
  const native = {
    combine() { return native; },
  };
  const session = {
    locator: () => native,
    roleLocator: () => native,
  };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = { _id: 'frame', page: () => page };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });

  // Act
  const nested = api.create(frame, '#outer').locator('#inner');
  const plain = api.create(frame, 'button');
  const described = plain.describe('Submit button');
  const chained = described.locator('span');
  const role = api.role(frame, 'button', { name: 'Submit' });

  // Assert
  assert.equal(nested.toString(), "locator('#outer').locator('#inner')");
  assert.equal(plain.description(), null);
  assert.equal(described.description(), 'Submit button');
  assert.equal(described.toString(), 'Submit button');
  assert.equal(chained.description(), null);
  assert.equal(chained.toString(), "locator('button').locator('span')");
  assert.equal(role.toString(), "getByRole('button', { name: 'Submit' })");
});

test('Locator text RegExp filters preserve source, flags and exclusion in native recipes', () => {
  // Arrange
  const calls = [];
  const native = {
    copy() { return this; },
    filterRegex(source, flags, exclude) {
      calls.push(['filterRegex', source, flags, exclude]);
      return this;
    },
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = { _id: 'frame', page: () => page };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });

  // Act
  api.create(frame, 'div', { hasText: /^hello$/iu }).filter({ hasNotText: /world/m });

  // Assert
  assert.deepEqual(calls, [
    ['filterRegex', '^hello$', 'iu', false],
    ['filterRegex', 'world', 'm', true],
  ]);
});

test('Locator read timeout includes the immutable Locator call-log description', async () => {
  // Arrange
  const timeout = Object.assign(new Error('locator.innerText: Timeout 10ms exceeded.'), {
    name: 'TimeoutError',
  });
  const native = { async read() { throw timeout; } };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = { _id: 'frame', page: () => page };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });

  // Act
  const operation = api.create(frame, 'span').innerText({ timeout: 10 });

  // Assert
  await assert.rejects(operation, error => {
    assert.match(error.message, /waiting for locator\('span'\)/);
    return true;
  });
});

test('Locator public elementHandle publishes the Rust-owned native handle through its Frame', async () => {
  // Arrange
  const calls = [];
  const nativeHandle = { id: 'native-element' };
  const element = { id: 'public-element' };
  const native = {
    async elementHandle(timeout) {
      calls.push(['elementHandle', timeout]);
      return nativeHandle;
    },
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = {
    _id: 'frame',
    page: () => page,
    async _wrapNativeHandle(value) {
      calls.push(['wrap', value]);
      return element;
    },
  };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  const locator = api.create(frame, 'custom=value');

  // Act
  const result = await locator.elementHandle({ timeout: 73 });

  // Assert
  assert.equal(result, element);
  assert.deepEqual(calls, [
    ['elementHandle', 73],
    ['wrap', nativeHandle],
  ]);
});

test('Locator atomic query wraps the one Rust-owned optional ElementHandle', async () => {
  // Arrange
  const calls = [];
  const nativeHandle = { id: 'native-element' };
  const element = { id: 'public-element' };
  const native = {
    async queryElementHandle() {
      calls.push(['queryElementHandle']);
      return nativeHandle;
    },
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = {
    _id: 'frame',
    page: () => page,
    async _wrapNativeHandle(value) {
      calls.push(['wrap', value]);
      return element;
    },
  };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  const locator = api.create(frame, 'div');

  // Act
  const result = await locator._queryElementHandle();

  // Assert
  assert.equal(result, element);
  assert.deepEqual(calls, [['queryElementHandle'], ['wrap', nativeHandle]]);
});

test('Locator atomic query preserves native null without wrapper publication', async () => {
  // Arrange
  let wrapped = false;
  const native = { async queryElementHandle() { return null; } };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = {
    _id: 'frame',
    page: () => page,
    async _wrapNativeHandle() { wrapped = true; },
  };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  const locator = api.create(frame, '.missing');

  // Act
  const result = await locator._queryElementHandle();

  // Assert
  assert.equal(result, null);
  assert.equal(wrapped, false);
});

test('Locator public elementHandles publishes one Rust-owned ElementHandle batch through its Frame', async () => {
  // Arrange
  const calls = [];
  const nativeHandles = [{ id: 'native-a' }, { id: 'native-b' }];
  const elements = [{ id: 'public-a' }, { id: 'public-b' }];
  const native = {
    async queryElementHandles() {
      calls.push(['queryElementHandles']);
      return nativeHandles;
    },
  };
  const session = { locator: () => native };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const frame = {
    _id: 'frame',
    page: () => page,
    async _wrapNativeHandles(values) {
      calls.push(['wrapBatch', values]);
      return elements;
    },
  };
  const api = locatorFactory({
    mapError: error => error,
    validateOptions() {},
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  const locator = api.create(frame, 'div');

  // Act
  const result = await locator.elementHandles();

  // Assert
  assert.equal(result, elements);
  assert.deepEqual(calls, [
    ['queryElementHandles'],
    ['wrapBatch', nativeHandles],
  ]);
});

test('Locator evaluateAll delegates one opaque conversion to the Rust collection lane', async () => {
  // Arrange
  const value = locatorWithEvaluation();
  const callback = nodes => nodes.map(node => node.textContent);

  // Act
  const result = await value.locator.evaluateAll(callback, { suffix: '!' });

  // Assert
  assert.deepEqual(result, ['one', 'two']);
  assert.deepEqual(value.calls, [[
    'evaluateAll',
    { source: String(callback), isFunction: true, argument: { suffix: '!' } },
  ]]);
});

test('Locator waitFor validates public options and delegates state waiting to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  await value.locator.waitFor({ state: 'hidden', timeout: 73 });

  // Assert
  assert.deepEqual(value.calls, [['waitFor', 'hidden', 73]]);
});

test('Locator geometry delegates its bounded timeout to the selected Rust leaf', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator.boundingBox({ timeout: 74 });

  // Assert
  assert.deepEqual(result, { x: 1, y: 2, width: 3, height: 4 });
  assert.deepEqual(value.calls, [['boundingBox', 74]]);
});

test('Locator highlight passes only the public description into the Rust browser operation', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  await value.locator.highlight();

  // Assert
  assert.deepEqual(value.calls, [['highlight', "locator('p')"]]);
});

test('Locator visibility expectation delegates polling and negation to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.be.visible', { timeout: 91, isNot: true });

  // Assert
  assert.deepEqual(result, { matches: true, received: true, timedOut: false });
  assert.deepEqual(value.calls, [['expectVisibility', 91, true, true]]);
});

test('Locator state expectation delegates semantics to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.be.empty', { timeout: 92, isNot: false });

  // Assert
  assert.deepEqual(result, { matches: true, received: 'empty', timedOut: false });
  assert.deepEqual(value.calls, [['expectState', 92, 'empty', false]]);
});

test('Locator boolean expectations delegate state and checked options to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const enabled = await value.locator._expect('to.be.enabled', { timeout: 92, isNot: false });
  const checked = await value.locator._expect('to.be.checked', {
    timeout: 93,
    expectedValue: { checked: false, indeterminate: undefined },
  });

  // Assert
  assert.deepEqual(enabled, {
    matches: false,
    received: 'disabled',
    timedOut: true,
    log: [
      '  - Expect "toBeEnabled" with timeout 92ms',
      "  - waiting for locator('p')",
      '  - locator resolved to <button disabled>Text</button>',
      '  - unexpected value "disabled"',
    ],
  });
  assert.deepEqual(checked, { matches: true, received: 'unchecked', timedOut: false });
  assert.deepEqual(value.calls, [
    ['expectState', 92, 'enabled', false],
    ['expectChecked', 93, false, undefined, false],
  ]);
});

test('Locator missing expectation reports the exact public description', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.be.attached', { timeout: 10 });

  // Assert
  assert.equal(value.locator.toString(), "locator('p')");
  assert.deepEqual(result, {
    matches: false,
    received: undefined,
    timedOut: true,
    errorMessage: 'Error: element(s) not found',
    log: [
      '  - Expect "toBeAttached" with timeout 10ms',
      "  - waiting for locator('p')",
    ],
  });
  assert.deepEqual(value.calls, [['expectState', 10, 'attached', false]]);
});

test('Locator property expectation delegates value comparison to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();
  const expected = { answer: 42 };

  // Act
  const result = await value.locator._expect('to.have.property', {
    expressionArg: 'model.value', expectedValue: expected, timeout: 93,
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: expected, timedOut: false });
  assert.deepEqual(value.calls, [['expectProperty', 93, 'model.value', expected, false]]);
});

test('Locator count expectation delegates collection polling to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.have.count', {
    expectedNumber: 2, timeout: 94, isNot: false,
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: 2, timedOut: false });
  assert.deepEqual(value.calls, [['expectCount', 94, 2, false]]);
});

test('Locator property missing result is distinguished from an undefined property', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.have.property', {
    expressionArg: 'missing', expectedValue: undefined, timeout: 10,
  });

  // Assert
  assert.deepEqual(result, {
    matches: false,
    timedOut: true,
    errorMessage: 'Error: element(s) not found',
    log: [
      '  - Expect "toHaveJSProperty" with timeout 10ms',
      "  - waiting for locator('p')",
    ],
  });
  assert.deepEqual(value.calls, [['expectProperty', 10, 'missing', undefined, false]]);
});

test('Locator text RegExp delegates JavaScript matching semantics to Rust browser evaluation', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.have.text', {
    expectedText: [{ regexSource: 'a.c', regexFlags: 'u', ignoreCase: true }],
    isNot: true,
    useInnerText: true,
    timeout: 11,
  });

  // Assert
  assert.deepEqual(result, {
    matches: false,
    received: undefined,
    timedOut: true,
    errorMessage: 'Error: element(s) not found',
    log: [
      '  - Expect "not toHaveText" with timeout 11ms',
      "  - waiting for locator('p')",
    ],
  });
  assert.deepEqual(value.calls, [[
    'expectTextRegex', 11, 'a.c', 'u', true, true, true,
  ]]);
});

test('Locator ordered text collection delegates polling and normalization flags to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const result = await value.locator._expect('to.have.text.array', {
    expectedText: [
      { string: 'first', normalizeWhiteSpace: true, ignoreCase: true },
      { string: 'second', normalizeWhiteSpace: true, ignoreCase: true },
    ],
    timeout: 15,
    useInnerText: true,
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: ['first', 'second'], timedOut: false });
  assert.deepEqual(value.calls, [[
    'expectTexts', 15, ['first', 'second'], false, true, true, true, false,
  ]]);
});

test('Locator class and value expectations delegate exact and RegExp matching to Rust', async () => {
  // Arrange
  const value = locatorWithEvaluation();

  // Act
  const classResult = await value.locator._expect('to.have.class', {
    expectedText: [{ regexSource: '^active', regexFlags: 'i' }], timeout: 12,
  });
  const valueResult = await value.locator._expect('to.have.value', {
    expectedText: [{ string: 'ready' }], timeout: 13, isNot: true,
  });

  // Assert
  assert.deepEqual(classResult, { matches: true, received: '^active', timedOut: false });
  assert.deepEqual(valueResult, { matches: true, received: 'ready', timedOut: false });
  assert.deepEqual(value.calls, [
    ['expectString', 12, 'class', '^active', 'i', false],
    ['expectString', 13, 'value', 'ready', undefined, true],
  ]);
});

test('Locator class collection preserves ordered mixed exact and RegExp matchers', async () => {
  // Arrange
  const value = locatorWithEvaluation();
  const expectedText = [
    { string: 'first' },
    { regexSource: '^sec', regexFlags: 'm' },
  ];

  // Act
  const result = await value.locator._expect('to.have.class.array', {
    expectedText, timeout: 14, isNot: false,
  });

  // Assert
  assert.deepEqual(result, {
    matches: true,
    received: ['first', '^sec'],
    timedOut: false,
  });
  assert.deepEqual(value.calls, [[
    'expectClassMatchers', 14,
    [{ expected: 'first' }, { expected: '^sec', flags: 'm' }], false,
  ]]);
});
