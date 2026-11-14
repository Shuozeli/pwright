import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const createWait = createRequire(import.meta.url)('../src/page-wait-for-selector.cjs');

function fixture(defaultTimeout = 700) {
  const calls = [];
  const handle = { id: 'retained-element' };
  const session = {
    pageDefaultTimeout(pageId, navigation) {
      calls.push(['defaultTimeout', pageId, navigation]);
      return defaultTimeout;
    },
  };
  const page = { _id: 'page-1', context: () => ({ _browser: { _session: session } }) };
  const query = {
    _waitForSelectorElement(state, timeout, strict, api) {
      calls.push(['wait', state, timeout, strict, api]);
      return Promise.resolve(state === 'hidden' || state === 'detached' ? null : handle);
    },
    first() { throw new Error('Page waits must not derive a first Locator'); },
  };
  const frame = {
    page: () => page,
    locator(selector) { calls.push(['locator', selector]); return query; },
  };
  const adapter = createWait({
    validateOptions(options, supported, api) {
      if (!options || typeof options !== 'object' || Array.isArray(options))
        throw new TypeError(`${api}: options must be an object`);
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new Error(`${api}: unsupported ${key}`);
      }
    },
    mapError(error, api) { return new Error(`${api}: ${error.message}`); },
  });
  return { adapter, calls, frame, handle, session };
}

test('Page waitForSelector snapshots inherited timeout and uses one non-strict native wait', async () => {
  // Arrange
  const { adapter, calls, frame, handle } = fixture(250);

  // Act
  const result = await adapter.wait(frame, '#target');

  // Assert
  assert.equal(result, handle);
  assert.deepEqual(calls, [
    ['defaultTimeout', 'page-1', false], ['locator', '#target'],
    ['wait', 'visible', 250, false, 'page.waitForSelector'],
  ]);
});

test('Frame waitForSelector keeps strictness and explicit zero timeout', async () => {
  // Arrange
  const { adapter, calls, frame, handle } = fixture();

  // Act
  const result = await adapter.wait(frame, 'main > div',
    { state: 'attached', strict: true, timeout: 0 }, 'frame.waitForSelector');

  // Assert
  assert.equal(result, handle);
  assert.deepEqual(calls, [
    ['locator', 'main > div'],
    ['wait', 'attached', 0, true, 'frame.waitForSelector'],
  ]);
});

test('waitForSelector forwards hidden completion as null without a second query', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const result = await adapter.wait(frame, '.gone', { state: 'hidden', timeout: 50 });

  // Assert
  assert.equal(result, null);
  assert.deepEqual(calls, [
    ['locator', '.gone'], ['wait', 'hidden', 50, false, 'page.waitForSelector'],
  ]);
});

test('waitForSelector accepts the upstream legacy waitFor visible alias', async () => {
  // Arrange
  const { adapter, calls, frame, handle } = fixture();

  // Act
  const result = await adapter.wait(frame, '*', { waitFor: 'visible', timeout: 5 });

  // Assert
  assert.equal(result, handle);
  assert.deepEqual(calls, [
    ['locator', '*'], ['wait', 'visible', 5, false, 'page.waitForSelector'],
  ]);
});

test('waitForSelector sends XPath unchanged to one non-strict native wait', async () => {
  // Arrange
  const { adapter, calls, frame, handle } = fixture();

  // Act
  const result = await adapter.wait(frame, '//p[normalize-space(.)="hello world"]');

  // Assert
  assert.equal(result, handle);
  assert.deepEqual(calls, [
    ['defaultTimeout', 'page-1', false],
    ['locator', '//p[normalize-space(.)="hello world"]'],
    ['wait', 'visible', 700, false, 'page.waitForSelector'],
  ]);
});

test('waitForSelector forwards detached and hidden states as native null completions', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const detached = await adapter.wait(frame, '//div', { state: 'detached', timeout: 10 });
  const hidden = await adapter.wait(frame, '//div', { state: 'hidden', timeout: 10 });

  // Assert
  assert.equal(detached, null);
  assert.equal(hidden, null);
  assert.deepEqual(calls, [
    ['locator', '//div'], ['wait', 'detached', 10, false, 'page.waitForSelector'],
    ['locator', '//div'], ['wait', 'hidden', 10, false, 'page.waitForSelector'],
  ]);
});

test('waitForSelector rejects invalid options before creating a native query', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const outcomes = await Promise.all([
    adapter.wait(frame, 'button', { state: 'ready' }).catch(error => error),
    adapter.wait(frame, 'button', { strict: 'yes' }).catch(error => error),
    adapter.wait(frame, 'button', { timeout: -1 }).catch(error => error),
    adapter.wait(frame, 'button', { force: true }).catch(error => error),
    adapter.wait(frame, 'button', { waitFor: 'attached' }).catch(error => error),
    adapter.wait(frame, 'button', { visibility: 'visible' }).catch(error => error),
  ]);

  // Assert
  for (const outcome of outcomes) assert.ok(outcome instanceof Error);
  assert.equal(outcomes[0].message,
    'page.waitForSelector: state: expected one of (attached|detached|visible|hidden)');
  assert.equal(outcomes[4].message,
    'options.waitFor is not supported, did you mean options.state?');
  assert.equal(outcomes[5].message,
    'options.visibility is not supported, did you mean options.state?');
  assert.deepEqual(calls, []);
});

test('waitForSelector reports the official state diagnostic for boolean values', async () => {
  // Arrange
  const { adapter, calls, frame } = fixture();

  // Act
  const truthy = await adapter.wait(frame, 'section', { state: true }).catch(error => error);
  const falsy = await adapter.wait(frame, 'section', { state: false }).catch(error => error);

  // Assert
  for (const error of [truthy, falsy]) {
    assert.equal(error.message,
      'page.waitForSelector: state: expected one of (attached|detached|visible|hidden)');
  }
  assert.deepEqual(calls, []);
});

test('waitForSelector timeout names the selector and state in its call log', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const timeout = Object.assign(new Error('page.waitForSelector: Timeout 5ms exceeded.'),
    { code: 'PWRIGHT_TIMEOUT' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw timeout; } });

  // Act
  const result = await adapter.wait(frame, "div[data-name='test']",
    { state: 'visible', timeout: 5 }).catch(error => error);

  // Assert
  assert.equal(result, timeout);
  assert.equal(result.message,
    "page.waitForSelector: Timeout 5ms exceeded.\nCall log:\n  - waiting for locator('div[data-name=\\'test\\']') to be visible");
});

test('waitForSelector attached timeout omits a redundant state suffix', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const timeout = Object.assign(new Error('page.waitForSelector: Timeout 5ms exceeded.'),
    { code: 'PWRIGHT_TIMEOUT' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw timeout; } });

  // Act
  const result = await adapter.wait(frame, '//div',
    { state: 'attached', timeout: 5 }).catch(error => error);

  // Assert
  assert.equal(result.message,
    "page.waitForSelector: Timeout 5ms exceeded.\nCall log:\n  - waiting for locator('//div')");
});

test('waitForSelector strict JavaScript failure retains the selector call log', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const violation = Object.assign(new Error('page.waitForSelector: strict mode violation'),
    { code: 'PWRIGHT_JAVASCRIPT' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw violation; } });

  // Act
  const result = await adapter.wait(frame, '//div',
    { strict: true, timeout: 5 }).catch(error => error);

  // Assert
  assert.equal(result.message,
    "page.waitForSelector: strict mode violation\nCall log:\n  - waiting for locator('//div') to be visible");
});

test('Frame waitForSelector maps a detached Frame before adding its call log', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const closed = Object.assign(new Error('frame.waitForSelector: Target closed'),
    { code: 'PWRIGHT_TARGET_CLOSED' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw closed; } });
  frame._mapEvaluationError = () => Object.assign(
    new Error('frame.waitForSelector: Frame was detached'), { code: 'PWRIGHT_TARGET_CLOSED' });

  // Act
  const result = await adapter.wait(frame, 'span', { timeout: 5 },
    'frame.waitForSelector').catch(error => error);

  // Assert
  assert.equal(result.message,
    "frame.waitForSelector: Frame was detached\nCall log:\n  - waiting for locator('span') to be visible");
});

test('Frame waitForSelector maps a navigation backend error after Frame detachment', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const navigated = Object.assign(new Error('frame.waitForSelector: Execution context was destroyed.'),
    { code: 'PWRIGHT_BACKEND' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw navigated; } });
  frame._metadata = { isDetached: () => true };

  // Act
  const result = await adapter.wait(frame, 'span', { timeout: 5 },
    'frame.waitForSelector').catch(error => error);

  // Assert
  assert.equal(result.message,
    "frame.waitForSelector: Frame was detached\nCall log:\n  - waiting for locator('span') to be visible");
});

for (const diagnostic of [
  'CDP error -32602: uniqueContextId not found',
  'uniqueContextId not found',
  'CDP error -32000: Cannot find context with specified id',
  'Cannot find context with specified id',
  'CDP error -32000: Inspected target navigated or closed',
]) {
  test(`Frame waitForSelector maps detached context loss: ${diagnostic}`, async () => {
    // Arrange
    const { adapter, frame } = fixture();
    const backend = Object.assign(new Error(`frame.waitForSelector: Backend: ${diagnostic}`),
      { code: 'PWRIGHT_BACKEND' });
    frame.locator = () => ({ _waitForSelectorElement: async () => { throw backend; } });
    frame._metadata = { isDetached: () => true };

    // Act
    const result = await adapter.wait(frame, 'span', { timeout: 5 },
      'frame.waitForSelector').catch(error => error);

    // Assert
    assert.equal(result.code, 'PWRIGHT_BACKEND');
    assert.equal(result.message,
      "frame.waitForSelector: Frame was detached\nCall log:\n  - waiting for locator('span') to be visible");
  });

  test(`Frame waitForSelector preserves attached context loss: ${diagnostic}`, async () => {
    // Arrange
    const { adapter, frame } = fixture();
    const backend = Object.assign(new Error(`frame.waitForSelector: Backend: ${diagnostic}`),
      { code: 'PWRIGHT_BACKEND' });
    frame.locator = () => ({ _waitForSelectorElement: async () => { throw backend; } });
    frame._metadata = { isDetached: () => false };

    // Act
    const result = await adapter.wait(frame, 'span', { timeout: 5 },
      'frame.waitForSelector').catch(error => error);

    // Assert
    assert.equal(result, backend);
    assert.equal(result.message,
      `frame.waitForSelector: Backend: ${diagnostic}\nCall log:\n  - waiting for locator('span') to be visible`);
  });
}

test('Frame waitForSelector does not remap an unrecognized backend error after detachment', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const backend = Object.assign(new Error('frame.waitForSelector: Backend: CDP error -32000: Other error'),
    { code: 'PWRIGHT_BACKEND' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw backend; } });
  frame._metadata = { isDetached: () => true };

  // Act
  const result = await adapter.wait(frame, 'span', { timeout: 5 },
    'frame.waitForSelector').catch(error => error);

  // Assert
  assert.equal(result, backend);
  assert.equal(result.message,
    "frame.waitForSelector: Backend: CDP error -32000: Other error\nCall log:\n  - waiting for locator('span') to be visible");
});

test('Page waitForSelector preserves context loss even when its Frame was detached', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const backend = Object.assign(new Error('page.waitForSelector: Backend: CDP error -32602: uniqueContextId not found'),
    { code: 'PWRIGHT_BACKEND' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw backend; } });
  let detachmentChecks = 0;
  frame._metadata = { isDetached: () => { detachmentChecks++; return true; } };

  // Act
  const result = await adapter.wait(frame, 'span', { timeout: 5 }).catch(error => error);

  // Assert
  assert.equal(result, backend);
  assert.equal(detachmentChecks, 0);
  assert.equal(result.message,
    "page.waitForSelector: Backend: CDP error -32602: uniqueContextId not found\nCall log:\n  - waiting for locator('span') to be visible");
});

test('Frame waitForSelector preserves context loss when detachment cannot be determined', async () => {
  // Arrange
  const { adapter, frame } = fixture();
  const backend = Object.assign(new Error('frame.waitForSelector: Backend: CDP error -32000: Cannot find context with specified id'),
    { code: 'PWRIGHT_BACKEND' });
  frame.locator = () => ({ _waitForSelectorElement: async () => { throw backend; } });
  frame._metadata = { isDetached: () => { throw new Error('metadata unavailable'); } };

  // Act
  const result = await adapter.wait(frame, 'span', { timeout: 5 },
    'frame.waitForSelector').catch(error => error);

  // Assert
  assert.equal(result, backend);
  assert.equal(result.message,
    "frame.waitForSelector: Backend: CDP error -32000: Cannot find context with specified id\nCall log:\n  - waiting for locator('span') to be visible");
});

test('waitForSelector maps a native inherited-timeout read failure', async () => {
  // Arrange
  const { adapter, calls, frame, session } = fixture();
  session.pageDefaultTimeout = () => { throw new Error('closed'); };

  // Act
  const result = await adapter.wait(frame, 'button').catch(error => error);

  // Assert
  assert.equal(result.message, 'page.waitForSelector: closed');
  assert.deepEqual(calls, []);
});
