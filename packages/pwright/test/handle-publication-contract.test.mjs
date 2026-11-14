import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const handleApi = createRequire(import.meta.url)('../src/handles.cjs');

function api(mapError = error => error) {
  return handleApi({
    mapError,
    validateOptions(options, supported, name) {
      for (const key of Object.keys(options)) {
        if (!supported.includes(key)) throw new Error(`${name}: unsupported ${key}`);
      }
    },
    UnsupportedOperationError: Error,
  });
}

test('JSHandle toString returns the native creation preview synchronously and caches it', async () => {
  // Arrange
  let reads = 0;
  const native = {
    isElement: () => false,
    preview() { reads++; return 'Window'; },
    bindWrapper() {},
  };
  const handle = await api().create({}, native);

  // Act
  const first = handle.toString();
  const second = String(handle);

  // Assert
  assert.equal(first, 'Window');
  assert.equal(second, 'Window');
  assert.equal(reads, 1);
});

test('ElementHandle toString normalizes the initial CDP node-name preview synchronously', async () => {
  // Arrange
  const native = {
    isElement: () => true,
    preview: () => 'body',
    bindWrapper() {},
  };
  const handle = await api().create({}, native);

  // Act
  const preview = handle.toString();

  // Assert
  assert.equal(preview, 'JSHandle@<body></body>');
});

test('ElementHandle asynchronously caches the rich preview computed by Rust', async () => {
  // Arrange
  const refresh = deferred();
  const native = {
    isElement: () => true,
    preview: () => 'div',
    bindWrapper() {},
  };
  const calls = [];
  const session = {
    elementHandlePreview(...args) {
      calls.push(args);
      return refresh.promise;
    },
  };
  const handle = await api().create(session, native);

  // Act
  const initial = handle.toString();
  refresh.resolve('JSHandle@<div id="target">Text</div>');
  await new Promise(resolve => setImmediate(resolve));
  const updated = handle.toString();

  // Assert
  assert.equal(initial, 'JSHandle@<div></div>');
  assert.equal(updated, 'JSHandle@<div id="target">Text</div>');
  assert.deepEqual(calls, [[native]]);
});

test('ElementHandle preview refresh failures are isolated from publication', async () => {
  // Arrange
  const native = {
    isElement: () => true,
    preview: () => 'span',
    bindWrapper() {},
  };
  const session = {
    async elementHandlePreview() { throw new Error('preview failed'); },
  };
  const handle = await api().create(session, native);

  // Act
  await new Promise(resolve => setImmediate(resolve));
  const preview = handle.toString();

  // Assert
  assert.equal(preview, 'JSHandle@<span></span>');
});

function deferred() {
  let resolve, reject;
  const promise = new Promise((done, failed) => { resolve = done; reject = failed; });
  return { promise, resolve, reject };
}

function failingNative(phase, primary) {
  return {
    isElement() {
      if (phase === 'classification') throw primary;
      return false;
    },
    bindWrapper(wrapper) {
      if (phase === 'binding') throw primary;
      if (phase === 'marker') Object.preventExtensions(wrapper);
    },
  };
}

test('ElementHandle innerText delegates its browser semantics to the Rust DOM capability', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleInnerText(...args) {
      calls.push(args);
      return 'Click me';
    },
    async handleEvaluate() { throw new Error('generic evaluation must not implement innerText'); },
  };
  const handle = await api().create(session, native);

  // Act
  const value = await handle.innerText();

  // Assert
  assert.equal(value, 'Click me');
  assert.deepEqual(calls, [[native]]);
});

test('ElementHandle textContent delegates its browser semantics to the Rust DOM capability', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleTextContent(...args) {
      calls.push(args);
      return 'Hello';
    },
    async handleEvaluate() { throw new Error('generic evaluation must not implement textContent'); },
  };
  const handle = await api().create(session, native);

  // Act
  const value = await handle.textContent();

  // Assert
  assert.equal(value, 'Hello');
  assert.deepEqual(calls, [[native]]);
});

test('ElementHandle focus forwards the exact retained owner to Rust', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleFocus(...args) { calls.push(args); },
    async handleEvaluate() { throw new Error('generic evaluation must not implement focus'); },
  };
  const handle = await api().create(session, native);

  // Act
  await handle.focus();

  // Assert
  assert.deepEqual(calls, [[native]]);
});

test('ElementHandle focus fails closed without its selected native DOM capability', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const handle = await api().create({}, native);

  // Act
  const result = handle.focus();

  // Assert
  await assert.rejects(result, /select element-handle-dom native profile/);
});

test('ElementHandle focus rejects unimplemented options rather than silently ignoring them', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleFocus(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  const result = handle.focus({ timeout: 100 });

  // Assert
  await assert.rejects(result, /elementHandle\.focus options/);
  assert.deepEqual(calls, []);
});

test('ElementHandle value selection forwards only its retained owner and bounded string', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleSelectOptionValue(...args) { calls.push(args); return ['blue']; },
    async handleEvaluate() { throw new Error('generic evaluation must not select an option'); },
  };
  const handle = await api().create(session, native);

  // Act
  const result = await handle.selectOption('blue', { timeout: 91 });

  // Assert
  assert.deepEqual(result, ['blue']);
  assert.deepEqual(calls, [[native, 'blue', 91]]);
});

test('ElementHandle value selection rejects unselected and unsupported forms before native work', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleSelectOptionValue(...args) { calls.push(args); } };
  const selected = await api().create(session, native);
  const unselected = await api().create({}, native);

  // Act
  const missing = unselected.selectOption('blue');
  const descriptor = selected.selectOption({ value: 'blue' });
  const force = selected.selectOption('blue', { force: true });
  const oversized = selected.selectOption('x'.repeat(65537));

  // Assert
  await assert.rejects(missing, /select element-handle-select-option-value native profile/);
  await assert.rejects(descriptor, /only a string value or label/);
  await assert.rejects(force, /unsupported force/);
  await assert.rejects(oversized, /exceeds 65536 UTF-8 bytes/);
  assert.deepEqual(calls, []);
});

test('ElementHandle scroll forwards the exact retained owner and explicit deadline to Rust', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleScrollIntoViewIfNeeded(...args) { calls.push(args); },
    async handleEvaluate() { throw new Error('generic evaluation must not implement scrolling'); },
  };
  const handle = await api().create(session, native);

  // Act
  await handle.scrollIntoViewIfNeeded({ timeout: 125 });

  // Assert
  assert.deepEqual(calls, [[native, 125]]);
});

test('ElementHandle scroll delegates default timeout selection to the Rust owner Page', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleScrollIntoViewIfNeeded(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  await handle.scrollIntoViewIfNeeded();

  // Assert
  assert.deepEqual(calls, [[native, undefined]]);
});

test('ElementHandle scroll preserves an explicit zero timeout without inheriting defaults', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleScrollIntoViewIfNeeded(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  await handle.scrollIntoViewIfNeeded({ timeout: 0 });

  // Assert
  assert.deepEqual(calls, [[native, 0]]);
});

test('ElementHandle scroll treats null timeout like an omitted timeout', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleScrollIntoViewIfNeeded(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  await handle.scrollIntoViewIfNeeded({ timeout: null });

  // Assert
  assert.deepEqual(calls, [[native, undefined]]);
});

test('ElementHandle scroll fails closed without its selected native leaf', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const handle = await api().create({}, native);

  // Act
  const result = handle.scrollIntoViewIfNeeded();

  // Assert
  await assert.rejects(result, /select element-handle-scroll-into-view native profile/);
});

test('ElementHandle waitForSelector forwards the original native root and inherits timeout in Rust', async () => {
  // Arrange
  const root = { isElement: () => true, bindWrapper() {} };
  const matched = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleWaitForSelector(...args) { calls.push(args); return matched; },
  };
  const handle = await api().create(session, root);

  // Act
  const result = await handle.waitForSelector('span', { state: 'attached' });

  // Assert
  assert.equal(result.asElement(), result);
  assert.deepEqual(calls, [[root, 'span', 'attached', false, undefined]]);
});

test('ElementHandle waitForSelector forwards strict selection and explicit zero timeout', async () => {
  // Arrange
  const root = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleWaitForSelector(...args) { calls.push(args); return null; },
  };
  const handle = await api().create(session, root);

  // Act
  const result = await handle.waitForSelector('xpath=.//span', {
    state: 'hidden', strict: true, timeout: 0,
  });

  // Assert
  assert.equal(result, null);
  assert.deepEqual(calls, [[root, 'xpath=.//span', 'hidden', true, 0]]);
});

test('ElementHandle waitForSelector adds the upstream scoped wait call log on navigation errors', async () => {
  // Arrange
  const root = { isElement: () => true, bindWrapper() {} };
  const error = Object.assign(new Error('elementHandle.waitForSelector: Execution context was destroyed.'),
    { code: 'PWRIGHT_BACKEND' });
  const session = {
    async elementHandleWaitForSelector() { throw error; },
  };
  const handle = await api().create(session, root);

  // Act
  const result = handle.waitForSelector('span').catch(caught => caught);

  // Assert
  assert.equal(await result, error);
  assert.match(error.message, /waiting for locator\('span'\) to be visible/);
});

test('ElementHandle waitForSelector keeps the explicit timeout and scoped call log', async () => {
  // Arrange
  const root = { isElement: () => true, bindWrapper() {} };
  const error = Object.assign(new Error('elementHandle.waitForSelector: Timeout 100ms exceeded.'),
    { code: 'PWRIGHT_TIMEOUT' });
  const session = {
    async elementHandleWaitForSelector() { throw error; },
  };
  const handle = await api().create(session, root);

  // Act
  const result = handle.waitForSelector('span', { timeout: 100 }).catch(caught => caught);

  // Assert
  assert.equal(await result, error);
  assert.match(error.message, /Timeout 100ms exceeded/);
  assert.match(error.message, /waiting for locator\('span'\) to be visible/);
});

test('ElementHandle waitForSelector rejects invalid options without native submission', async () => {
  // Arrange
  const root = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleWaitForSelector(...args) { calls.push(args); return null; },
  };
  const handle = await api().create(session, root);

  // Act
  const invalidState = handle.waitForSelector('span', { state: 'ready' });
  const invalidStrict = handle.waitForSelector('span', { strict: 'yes' });
  const invalidTimeout = handle.waitForSelector('span', { timeout: -1 });

  // Assert
  await assert.rejects(invalidState, /state: expected one of/);
  await assert.rejects(invalidStrict, /strict must be a boolean/);
  await assert.rejects(invalidTimeout, /timeout must be an integer/);
  assert.deepEqual(calls, []);
});

test('ElementHandle waitForSelector fails closed outside the opt-in native profile', async () => {
  // Arrange
  const root = { isElement: () => true, bindWrapper() {} };
  const handle = await api().create({}, root);

  // Act
  const result = handle.waitForSelector('span');

  // Assert
  await assert.rejects(result, /select element-handle-wait-for-selector native profile/);
});

test('ElementHandle scroll rejects unsupported options before native submission', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleScrollIntoViewIfNeeded(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  const unsupported = handle.scrollIntoViewIfNeeded({ force: true });

  // Assert
  await assert.rejects(unsupported, /unsupported force/);
  assert.deepEqual(calls, []);
});

test('ElementHandle scroll rejects invalid timeout before native submission', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleScrollIntoViewIfNeeded(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  const invalidTimeout = handle.scrollIntoViewIfNeeded({ timeout: -1 });

  // Assert
  await assert.rejects(invalidTimeout, /timeout must be an integer/);
  assert.deepEqual(calls, []);
});

test('ElementHandle boundingBox forwards its retained owner to the native geometry capability', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const bounds = { x: 1, y: 2, width: 3, height: 4 };
  const calls = [];
  const session = {
    async elementHandleBoundingBox(...args) { calls.push(args); return bounds; },
    async handleEvaluate() { throw new Error('generic evaluation must not implement boundingBox'); },
  };
  const handle = await api().create(session, native);

  // Act
  const value = await handle.boundingBox();

  // Assert
  assert.equal(value, bounds);
  assert.deepEqual(calls, [[native]]);
});

test('ElementHandle boundingBox preserves a native null for an element without a layout box', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const session = { async elementHandleBoundingBox() { return null; } };
  const handle = await api().create(session, native);

  // Act
  const value = await handle.boundingBox();

  // Assert
  assert.equal(value, null);
});

test('ElementHandle boundingBox requires its selected native geometry composition', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const handle = await api().create({}, native);

  // Act
  const result = handle.boundingBox();

  // Assert
  await assert.rejects(result, /select element-handle-dom and locator-bounding-box native profiles/);
});

test('ElementHandle boundingBox maps a native geometry failure without replacing its owner', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const failure = new Error('geometry failed');
  const calls = [];
  const session = {
    async elementHandleBoundingBox(...args) { calls.push(args); throw failure; },
  };
  const handle = await api((error, name) => new Error(`${name}: ${error.message}`)).create(session, native);

  // Act
  const result = handle.boundingBox();

  // Assert
  await assert.rejects(result, /elementHandle\.boundingBox: geometry failed/);
  assert.deepEqual(calls, [[native]]);
});

test('ElementHandle dispatchEvent forwards the exact owner and initializer to Rust', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const eventInit = { bubbles: false, detail: 2 };
  const calls = [];
  const session = {
    async elementHandleDispatchEvent(...args) { calls.push(args); },
    async handleEvaluate() { throw new Error('generic evaluation must not implement dispatchEvent'); },
  };
  const handle = await api().create(session, native);

  // Act
  await handle.dispatchEvent('custom', eventInit);

  // Assert
  assert.deepEqual(calls, [[native, 'custom', eventInit]]);
  assert.equal(calls[0][2], eventInit);
});

test('ElementHandle dispatchEvent supplies an empty initializer when omitted', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async elementHandleDispatchEvent(...args) { calls.push(args); } };
  const handle = await api().create(session, native);

  // Act
  await handle.dispatchEvent('click');

  // Assert
  assert.deepEqual(calls, [[native, 'click', {}]]);
});

test('ElementHandle dispatchEvent fails explicitly when its native composition is not selected', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const handle = await api().create({}, native);

  // Act
  const result = handle.dispatchEvent('click');

  // Assert
  await assert.rejects(result, /select element-handle-dom and locator-dom-actions native profiles/);
});

test('ElementHandle dispatchEvent rejects a non-string event type before native forwarding', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  let called = false;
  const session = { async elementHandleDispatchEvent() { called = true; } };
  const handle = await api().create(session, native);

  // Act
  const result = handle.dispatchEvent(42);

  // Assert
  await assert.rejects(result, /type must be a string/);
  assert.equal(called, false);
});

test('ElementHandle tap delegates exact-owner actionability options to Rust', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleTap(...args) { calls.push(args); },
  };
  const handle = await api().create(session, native);
  const options = {
    force: true,
    modifiers: ['Alt'],
    position: { x: 2.5, y: 3.25 },
    timeout: 91,
    trial: true,
  };

  // Act
  await handle.tap(options);

  // Assert
  assert.deepEqual(calls, [[native, 91, {
    force: true,
    modifiers: ['Alt'],
    noWaitAfter: undefined,
    position: options.position,
    trial: true,
  }]]);
});

for (const [method, expected] of [
  ['innerHTML', '<span>hello</span>'],
  ['inputValue', 'typed'],
  ['isChecked', true],
  ['isDisabled', false],
  ['isEditable', true],
  ['isEnabled', true],
  ['isHidden', false],
  ['isVisible', true],
]) {
  test(`ElementHandle ${method} delegates its exact-owner semantics to the Rust DOM capability`, async () => {
    // Arrange
    const native = { isElement: () => true, bindWrapper() {} };
    const calls = [];
    const session = {
      async elementHandleDomRead(...args) { calls.push(args); return expected; },
      async handleEvaluate() { throw new Error(`generic evaluation must not implement ${method}`); },
    };
    const handle = await api().create(session, native);

    // Act
    const value = await handle[method]();

    // Assert
    assert.equal(value, expected);
    assert.deepEqual(calls, [[native, method, undefined]]);
  });
}

test('ElementHandle getAttribute delegates the bounded name to the Rust DOM capability', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleDomRead(...args) { calls.push(args); return 'value'; },
    async handleEvaluate() { throw new Error('generic evaluation must not implement getAttribute'); },
  };
  const handle = await api().create(session, native);

  // Act
  const value = await handle.getAttribute('data-name');

  // Assert
  assert.equal(value, 'value');
  assert.deepEqual(calls, [[native, 'getAttribute', 'data-name']]);
});

test('ElementHandle dollar query publishes Rust-owned matches and preserves native null', async () => {
  // Arrange
  const source = { isElement: () => true, bindWrapper() {} };
  const child = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleQuerySelector(...args) {
      calls.push(args);
      return args[1] === '.present' ? child : null;
    },
    async handleEvaluateHandle() { throw new Error('generic evaluation must not implement querySelector'); },
  };
  const handle = await api().create(session, source);

  // Act
  const present = await handle.$('.present');
  const missing = await handle.$('.missing');

  // Assert
  assert.equal(present.asElement(), present);
  assert.equal(missing, null);
  assert.deepEqual(calls, [[source, '.present'], [source, '.missing']]);
});

test('ElementHandle double-dollar query publishes the Rust-owned ordered element batch', async () => {
  // Arrange
  const source = { isElement: () => true, bindWrapper() {} };
  const first = { isElement: () => true, bindWrapper() {} };
  const second = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleQuerySelectorAll(...args) {
      calls.push(args);
      return [first, second];
    },
    async handleEvaluateHandle() { throw new Error('generic evaluation must not query elements'); },
    async handleGetProperties() { throw new Error('Node must not classify query properties'); },
  };
  const handle = await api().create(session, source);

  // Act
  const elements = await handle.$$('xpath=.//span');

  // Assert
  assert.equal(elements.length, 2);
  assert.equal(elements[0].asElement(), elements[0]);
  assert.equal(elements[1].asElement(), elements[1]);
  assert.deepEqual(calls, [[source, 'xpath=.//span']]);
});

test('ElementHandle dollar-eval evaluates the exact queried owner and releases it', async () => {
  // Arrange
  const source = { isElement: () => true, bindWrapper() {} };
  const child = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleQuerySelector(...args) { calls.push(['query', ...args]); return child; },
    async handleEvaluate(...args) { calls.push(['evaluate', ...args]); return 'hello'; },
    async handleDispose(value) { calls.push(['dispose', value]); },
  };
  const handle = await api().create(session, source);
  const callback = element => element.textContent;

  // Act
  const result = await handle.$eval('span', callback);

  // Assert
  assert.equal(result, 'hello');
  assert.deepEqual(calls[1], ['evaluate', child, String(callback), true, undefined]);
  assert.deepEqual(calls[2], ['dispose', child]);
});

test('ElementHandle dollar-eval prefixes a native missing-selector error without fabricating an owner', async () => {
  // Arrange
  const source = { isElement: () => true, bindWrapper() {} };
  const disposals = [];
  const session = {
    async elementHandleQuerySelector() { return null; },
    async handleDispose(value) { disposals.push(value); },
  };
  const handle = await api().create(session, source);

  // Act
  const operation = handle.$eval('.missing', element => element.textContent);

  // Assert
  await assert.rejects(operation, /elementHandle\.\$eval: Failed to find element matching selector "\.missing"/);
  assert.deepEqual(disposals, []);
});

test('ElementHandle double-dollar-eval delegates query array ownership and evaluation to Rust', async () => {
  // Arrange
  const source = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = {
    async elementHandleEvalOnSelectorAll(...args) { calls.push(args); return 2; },
    async handleEvaluateHandle() { throw new Error('generic evaluation must not create the query array'); },
    async handleEvaluate() { throw new Error('Node must not evaluate the query array'); },
  };
  const handle = await api().create(session, source);
  const callback = elements => elements.length;

  // Act
  const result = await handle.$$eval('div', callback);

  // Assert
  assert.equal(result, 2);
  assert.deepEqual(calls, [[source, 'div', String(callback), true, undefined]]);
});

test('ElementHandle double-dollar wrapper failure releases every Rust-owned result', async () => {
  // Arrange
  const primary = new Error('second wrapper failed');
  const source = { isElement: () => true, bindWrapper() {} };
  const first = { isElement: () => true, bindWrapper() {} };
  const second = { isElement: () => true, bindWrapper() { throw primary; } };
  const disposals = [];
  const session = {
    async elementHandleQuerySelectorAll() { return [first, second]; },
    async handleDispose(value) { disposals.push(value); },
  };
  const handle = await api().create(session, source);

  // Act
  const operation = handle.$$('div');

  // Assert
  await assert.rejects(operation, error => error === primary);
  assert.deepEqual(disposals, [first, second]);
});

test('ElementHandle ownerFrame forwards only its native receiver and returns the canonical wrapper', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const frame = { identity: 'canonical Frame' };
  const calls = [];
  const session = { async handleOwnerFrame(value) { calls.push(['native', value]); return '42'; } };
  const handle = await api().create(session, native, id => { calls.push(['resolve', id]); return frame; });

  // Act: surplus arguments are ignored, not inspected or forwarded.
  const ignored = { get selector() { throw new Error('surplus argument inspected'); } };
  const owner = await handle.ownerFrame(ignored);

  // Assert
  assert.equal(owner, frame);
  assert.deepEqual(calls, [['native', native], ['resolve', '42']]);
});

test('ElementHandle ownerFrame returns native null without resolver lookup or extra native calls', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const calls = [];
  const session = { async handleOwnerFrame(value) { calls.push(value); return null; } };
  const handle = await api().create(session, native, () => { throw new Error('null must not resolve a Frame'); });

  // Act
  const owner = await handle.ownerFrame();

  // Assert
  assert.equal(owner, null);
  assert.deepEqual(calls, [native]);
});

test('ElementHandle contentFrame forwards its retained native receiver and resolves the child Frame', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const frame = { identity: 'child Frame' };
  const calls = [];
  const session = {
    async handleContentFrame(value) {
      calls.push(['native', value]);
      return '84';
    },
  };
  const handle = await api().create(session, native, id => {
    calls.push(['resolve', id]);
    return frame;
  });

  // Act
  const content = await handle.contentFrame();

  // Assert
  assert.equal(content, frame);
  assert.deepEqual(calls, [['native', native], ['resolve', '84']]);
});

test('ElementHandle contentFrame maps an absent child to null without resolver lookup', async () => {
  // Arrange
  const native = { isElement: () => true, bindWrapper() {} };
  const session = { async handleContentFrame() { return null; } };
  const handle = await api().create(session, native, () => {
    throw new Error('null must not resolve a Frame');
  });

  // Act
  const content = await handle.contentFrame();

  // Assert
  assert.equal(content, null);
});

test('ElementHandle ownerFrame exposes missing resolver wiring instead of silently returning null', async () => {
  // Arrange: only legacy unit fixtures may omit the production resolver.
  const native = { isElement: () => true, bindWrapper() {} };
  let nativeCalls = 0;
  const session = { async handleOwnerFrame() { nativeCalls++; return '42'; } };
  const handle = await api().create(session, native);

  // Act
  const result = handle.ownerFrame();

  // Assert
  await assert.rejects(result, /ElementHandle owner Frame resolver is missing/);
  assert.equal(nativeCalls, 0);
});

test('JSHandle evaluation stringifies non-function runtime values before native forwarding', async () => {
  // Arrange
  const native = { isElement: () => false, bindWrapper() {} };
  const child = { isElement: () => false, bindWrapper() {} };
  const calls = [];
  const session = {
    async handleEvaluate(...args) { calls.push(['value', ...args]); return 'evaluated'; },
    async handleEvaluateHandle(...args) { calls.push(['handle', ...args]); return child; },
  };
  const handle = await api().create(session, native);
  // Act
  const value = await handle.evaluate(42, 'argument');
  const descendant = await handle.evaluateHandle(null);
  // Assert
  assert.equal(value, 'evaluated');
  assert.deepEqual(calls, [
    ['value', native, '42', false, 'argument'],
    ['handle', native, 'null', false, undefined],
  ]);
  assert.equal(descendant.asElement(), null);
});

for (const producer of ['evaluateHandle', 'getProperty', 'getProperties']) {
  test(`${producer} forwards the source resolver to ElementHandle descendants`, async () => {
    // Arrange
    const source = { isElement: () => false, bindWrapper() {} };
    const child = { isElement: () => true, bindWrapper() {} };
    const frame = { identity: 'existing child Frame' };
    const calls = [];
    const session = {
      async handleEvaluateHandle(value) { assert.equal(value, source); return child; },
      async handleGetProperty(value) { assert.equal(value, source); return child; },
      async handleGetProperties(value) { assert.equal(value, source); return [{ name: 'node', handle: child }]; },
      async handleOwnerFrame(value) { calls.push(['native', value]); return '84'; },
    };
    const wrapper = await api().create(session, source, id => { calls.push(['resolve', id]); return frame; });
    const descendant = producer === 'evaluateHandle' ? await wrapper.evaluateHandle(value => value.node)
      : producer === 'getProperty' ? await wrapper.getProperty('node') : (await wrapper.getProperties()).get('node');

    // Act
    const owner = await descendant.ownerFrame();

    // Assert
    assert.equal(typeof wrapper.ownerFrame, 'undefined');
    assert.equal(descendant.asElement(), descendant);
    assert.equal(owner, frame);
    assert.deepEqual(calls, [['native', child], ['resolve', '84']]);
  });
}

for (const rejection of ['synchronous', 'asynchronous']) {
  test(`ElementHandle ownerFrame maps ${rejection} native rejection without disposing its source`, async () => {
    // Arrange
    const native = { isElement: () => true, bindWrapper() {} };
    const primary = new Error('native owner lookup failed');
    const mapped = new Error('mapped owner lookup failure');
    const mappings = [];
    const session = { handleOwnerFrame(value) {
      assert.equal(value, native);
      if (rejection === 'synchronous') throw primary;
      return Promise.reject(primary);
    } };
    const handle = await api((error, name) => { mappings.push({ error, name }); return mapped; })
      .create(session, native, () => { throw new Error('failed native lookup must not resolve a Frame'); });

    // Act
    const result = handle.ownerFrame();

    // Assert
    await assert.rejects(result, error => error === mapped);
    assert.deepEqual(mappings, [{ error: primary, name: 'elementHandle.ownerFrame' }]);
  });
}

for (const phase of ['classification', 'binding', 'marker']) {
  test(`wrapper ${phase} failure requests disposal immediately and awaits its acknowledgement`, async () => {
    // Arrange
    const primary = new Error(`${phase} failed`);
    const native = failingNative(phase, primary);
    const completion = deferred();
    const disposals = [];
    const session = { handleDispose(value) { disposals.push(value); return completion.promise; } };
    let settled = false;

    // Act
    const result = api().create(session, native);
    const observed = result.then(() => { settled = true; }, error => { settled = true; return error; });

    // Assert: invoking the async factory must already have requested cleanup.
    assert.deepEqual(disposals, [native]);
    await Promise.resolve();
    assert.equal(settled, false);
    completion.resolve();
    const error = await observed;
    if (phase === 'marker') assert.ok(error instanceof TypeError);
    else assert.equal(error, primary);
    assert.equal(settled, true);
    assert.equal(disposals.length, 1);
  });
}

for (const rejection of ['synchronous', 'asynchronous']) {
  test(`wrapper failure preserves both publication and ${rejection} disposal failures`, async () => {
    // Arrange
    const primary = new Error('binding failed');
    const cleanup = new Error('cleanup failed');
    const native = failingNative('binding', primary);
    const disposals = [];
    const session = {
      handleDispose(value) {
        disposals.push(value);
        if (rejection === 'synchronous') throw cleanup;
        return Promise.reject(cleanup);
      },
    };

    // Act
    const result = api().create(session, native);

    // Assert
    await assert.rejects(result, error => {
      assert.ok(error instanceof AggregateError);
      assert.equal(error.cause, primary);
      assert.deepEqual(error.errors, [primary, cleanup]);
      return true;
    });
    assert.deepEqual(disposals, [native]);
  });
}

for (const element of [false, true]) {
  test(`successful ${element ? 'ElementHandle' : 'JSHandle'} wrapping retains the native owner`, async () => {
    // Arrange
    const wrappers = [];
    const native = { isElement: () => element, bindWrapper: wrapper => wrappers.push(wrapper) };
    const disposals = [];
    const session = { handleDispose: value => disposals.push(value) };

    // Act
    const wrapper = await api().create(session, native);

    // Assert
    assert.deepEqual(wrappers, [wrapper]);
    assert.equal(wrapper.asElement(), element ? wrapper : null);
    assert.equal(Object.prototype.toString.call(wrapper), '[object PwrightJSHandle]');
    assert.deepEqual(disposals, []);
  });
}

for (const producer of ['evaluateHandle', 'getProperty', 'getProperties']) {
  test(`${producer} maps a child publication failure only after child cleanup completes`, async () => {
    // Arrange
    const primary = new Error('child publication failed');
    const mapped = new Error('mapped publication failure');
    const completion = deferred();
    const target = { isElement: () => false, bindWrapper() {} };
    const child = failingNative('binding', primary);
    const disposals = [];
    const mappings = [];
    const session = {
      async handleEvaluateHandle(value) { assert.equal(value, target); return child; },
      async handleGetProperty(value) { assert.equal(value, target); return child; },
      async handleGetProperties(value) { assert.equal(value, target); return [{ name: 'child', handle: child }]; },
      handleDispose(value) { disposals.push(value); return completion.promise; },
    };
    const wrapper = await api((error, name) => { mappings.push({ error, name }); return mapped; }).create(session, target);

    // Act
    const result = producer === 'evaluateHandle'
      ? wrapper.evaluateHandle(value => value)
      : producer === 'getProperty' ? wrapper.getProperty('child') : wrapper.getProperties();
    const observed = result.catch(error => error);

    // Assert: producer has delivered the child; cleanup, not GC, owns release.
    await Promise.resolve();
    assert.deepEqual(disposals, [child]);
    assert.deepEqual(mappings, []);
    completion.resolve();
    assert.equal(await observed, mapped);
    assert.deepEqual(mappings, [{ error: primary, name: `jsHandle.${producer}` }]);
    assert.deepEqual(disposals, [child]);
  });
}

function batchNative(element = false) {
  const state = { classifications: 0, wrappers: [] };
  return {
    state,
    isElement() { state.classifications++; return element; },
    bindWrapper(wrapper) { state.wrappers.push(wrapper); },
  };
}

test('empty native property snapshot publishes an empty Map without disposal', async () => {
  // Arrange
  const disposals = [];
  const session = { handleDispose: value => disposals.push(value) };
  // Act
  const properties = await api().createBatch(session, []);
  // Assert
  assert.ok(properties instanceof Map);
  assert.equal(properties.size, 0);
  assert.deepEqual(disposals, []);
});

test('batch publication preserves Rust order, classifies once, and owns aliases independently', async () => {
  // Arrange
  const target = batchNative();
  const children = [batchNative(), batchNative(true), batchNative(), batchNative(), batchNative()];
  const names = ['10', '2', '__proto__', 'then', ''];
  const entries = children.map((handle, index) => ({ name: names[index], handle }));
  const sharedValue = { answer: 42 };
  const disposals = [];
  const calls = [];
  const session = {
    handleGetProperties(value) { calls.push(value); return entries; },
    handleJsonValue(value) { assert.ok(!disposals.includes(value)); return sharedValue; },
    handleDispose(value) { disposals.push(value); },
  };
  const wrapper = await api().create(session, target);
  // Act
  const properties = await wrapper.getProperties();
  // Assert
  assert.deepEqual(calls, [target]);
  assert.deepEqual([...properties.keys()], names);
  assert.deepEqual(disposals, []);
  for (const [index, handle] of children.entries()) {
    const result = properties.get(names[index]);
    assert.equal(handle.state.classifications, 1);
    assert.deepEqual(handle.state.wrappers, [result]);
    assert.equal(result.asElement(), index === 1 ? result : null);
    assert.equal(Object.prototype.toString.call(result), '[object PwrightJSHandle]');
  }
  const first = properties.get('10');
  const alias = properties.get('__proto__');
  assert.notEqual(first, alias);
  // Independent owner release is a subsequent lifecycle observation.
  await first.dispose();
  assert.equal(await alias.jsonValue(), sharedValue);
  assert.deepEqual(disposals, [children[0]]);
  await alias.dispose();
  assert.deepEqual(disposals, [children[0], children[2]]);
});

for (const phase of ['classification', 'binding', 'marker']) {
  test(`batch ${phase} failure releases previous, failing and unvisited owners exactly once`, async () => {
    // Arrange
    const primary = new Error(`${phase} failed`);
    const before = batchNative();
    const failing = failingNative(phase, primary);
    const after = batchNative(true);
    const entries = [before, failing, after].map((handle, index) => ({ name: String(index), handle }));
    const disposals = [];
    const session = { handleDispose(value) { disposals.push(value); } };
    // Act
    const observed = await api().createBatch(session, entries).catch(error => error);
    // Assert
    if (phase === 'marker') assert.ok(observed instanceof TypeError);
    else assert.equal(observed, primary);
    assert.deepEqual(disposals, [before, failing, after]);
    assert.equal(before.state.wrappers.length, 1);
    assert.equal(after.state.classifications, 0);
    assert.equal(after.state.wrappers.length, 0);
  });
}

for (const phase of ['construction', 'first insertion', 'later insertion']) {
  test(`Map ${phase} failure compensates the complete native batch`, async () => {
    // Arrange
    const primary = new Error(`Map ${phase} failed`);
    const children = [batchNative(), batchNative(true), batchNative()];
    const entries = children.map((handle, index) => ({ name: String(index), handle }));
    const disposals = [];
    const session = { handleDispose(value) { disposals.push(value); } };
    const factory = api();
    const OriginalMap = globalThis.Map;
    let insertions = 0;
    class FailingMap extends OriginalMap {
      constructor() {
        if (phase === 'construction') throw primary;
        super();
      }
      set(name, value) {
        insertions++;
        if (insertions === (phase === 'first insertion' ? 1 : 2)) throw primary;
        return super.set(name, value);
      }
    }
    // Act: restore the global synchronously before awaiting any cleanup work.
    let result;
    try {
      globalThis.Map = FailingMap;
      result = factory.createBatch(session, entries);
    } finally { globalThis.Map = OriginalMap; }
    const observed = await result.catch(error => error);
    // Assert
    assert.equal(observed, primary);
    assert.deepEqual(disposals, children);
    const visited = phase === 'construction' ? 0 : phase === 'first insertion' ? 1 : 2;
    assert.deepEqual(children.map(child => child.state.classifications), children.map((_child, index) => index < visited ? 1 : 0));
  });
}

test('batch rollback preserves every cleanup failure and still attempts subsequent owners', async () => {
  // Arrange
  const primary = new Error('classification failed');
  const synchronous = new Error('first synchronous cleanup failure');
  const asynchronous = new Error('second asynchronous cleanup failure');
  const children = [failingNative('classification', primary), batchNative(), batchNative()];
  const entries = children.map((handle, index) => ({ name: String(index), handle }));
  const disposals = [];
  const session = {
    handleDispose(value) {
      disposals.push(value);
      if (value === children[0]) throw synchronous;
      if (value === children[1]) return Promise.reject(asynchronous);
    },
  };
  // Act
  const observed = await api().createBatch(session, entries).catch(error => error);
  // Assert
  assert.ok(observed instanceof AggregateError);
  assert.equal(observed.cause, primary);
  assert.deepEqual(observed.errors, [primary, synchronous, asynchronous]);
  assert.deepEqual(disposals, children);
});

test('batch rollback admits disposal sequentially and awaits the last acknowledgement', async () => {
  // Arrange
  const primary = new Error('binding failed');
  const children = [failingNative('binding', primary), batchNative()];
  const entries = children.map((handle, index) => ({ name: String(index), handle }));
  const first = deferred();
  const last = deferred();
  const lastStarted = deferred();
  const disposals = [];
  const session = {
    handleDispose(value) {
      disposals.push(value);
      if (value === children[0]) return first.promise;
      lastStarted.resolve();
      return last.promise;
    },
  };
  let settled = false;
  // Act
  const observed = api().createBatch(session, entries).catch(error => { settled = true; return error; });
  // Assert
  assert.deepEqual(disposals, [children[0]]);
  await Promise.resolve();
  assert.equal(settled, false);
  first.resolve();
  await lastStarted.promise;
  assert.deepEqual(disposals, children);
  assert.equal(settled, false);
  last.resolve();
  assert.equal(await observed, primary);
  assert.equal(settled, true);
});

test('getProperties forwards only its native target without inspecting surplus arguments', async () => {
  // Arrange
  const target = batchNative();
  const calls = [];
  let reads = 0;
  const ignored = { get value() { reads++; throw new Error('ignored argument getter'); } };
  const session = { handleGetProperties(...args) { calls.push(args); return []; } };
  const wrapper = await api().create(session, target);
  // Act
  const properties = await wrapper.getProperties(ignored);
  // Assert
  assert.equal(properties.size, 0);
  assert.deepEqual(calls, [[target]]);
  assert.equal(reads, 0);
});

for (const rejection of ['synchronous', 'asynchronous']) {
  test(`getProperties maps ${rejection} native rejection without releasing its source`, async () => {
    // Arrange
    const primary = new Error('native target is disposed');
    const mapped = new Error('mapped target error');
    const target = batchNative();
    const disposals = [];
    const mappings = [];
    const session = {
      handleGetProperties(value) {
        assert.equal(value, target);
        if (rejection === 'synchronous') throw primary;
        return Promise.reject(primary);
      },
      handleDispose(value) { disposals.push(value); },
    };
    const wrapper = await api((error, name) => { mappings.push({ error, name }); return mapped; }).create(session, target);
    // Act
    const observed = await wrapper.getProperties().catch(error => error);
    // Assert
    assert.equal(observed, mapped);
    assert.deepEqual(mappings, [{ error: primary, name: 'jsHandle.getProperties' }]);
    assert.deepEqual(disposals, []);
  });
}
