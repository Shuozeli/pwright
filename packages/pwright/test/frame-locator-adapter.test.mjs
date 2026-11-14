import assert from 'node:assert/strict';
import { test } from 'node:test';
import locatorFactory from '../src/locator.cjs';
import frameLocatorFactory from '../src/frame-locator.cjs';

function fixture() {
  const calls = [];
  const native = {
    expectText: async (...args) => {
      calls.push(['expectText', ...args]);
      return { matches: true, received: 'request count: 1', timedOut: false };
    },
  };
  const nativeFrameLocator = {
    locatorFrom(locator) {
      calls.push(['frameLocator.locatorFrom', locator]);
      return native;
    },
  };
  native.contentFrame = () => {
    calls.push(['locator.contentFrame']);
    return nativeFrameLocator;
  };
  native.nth = index => {
    calls.push(['locator.nth', index]);
    return native;
  };
  const session = {
    locator(frame, selector) {
      calls.push(['locator', frame, selector]);
      return native;
    },
    semanticLocator(frame, kind, source, exact, flags) {
      calls.push(['semanticLocator', frame, kind, source, exact, flags]);
      return native;
    },
    roleLocator(frame, role, options) {
      calls.push(['getByRole', frame, role, options]);
      return native;
    },
  };
  const page = { context: () => ({ browser: () => ({ _session: session }) }) };
  const child = { _id: 'child', page: () => page };
  let disposed = false;
  const owner = {
    contentFrame: async () => child,
    dispose: async () => { disposed = true; calls.push(['dispose']); },
  };
  const frame = {
    _id: 'root',
    page: () => page,
    _wrapNativeHandle: async value => {
      calls.push(['wrapNativeHandle', value]);
      return owner;
    },
    waitForFunction: async (fn, selector) => {
      calls.push(['waitForFunction', typeof fn, selector]);
      return owner;
    },
  };
  const locatorApi = locatorFactory({
    mapError: error => error,
    validateOptions(options, allowed) {
      for (const key of Object.keys(options)) assert.ok(allowed.includes(key));
    },
    UnsupportedOperationError: class UnsupportedOperationError extends Error {},
  });
  return {
    calls,
    frame,
    session,
    native,
    nativeFrameLocator,
    locatorApi,
    disposed: () => disposed,
    frameLocatorApi: frameLocatorFactory(locatorApi),
  };
}

test('FrameLocator retains one Rust-owned lazy iframe recipe across the native expectation', async () => {
  // Arrange
  const value = fixture();
  const locator = value.frameLocatorApi.create(value.frame, '[name=tracker]').locator(':scope');

  // Act
  const result = await locator._expect('to.have.text', {
    expectedText: [{ string: 'request count: 1', matchSubstring: true, normalizeWhiteSpace: true }],
    isNot: false,
    timeout: 5000,
  });

  // Assert
  assert.deepEqual(result, { matches: true, received: 'request count: 1', timedOut: false });
  assert.equal(value.disposed(), false);
  assert.deepEqual(value.calls, [
    ['locator', 'root', '[name=tracker]'],
    ['locator.contentFrame'],
    ['locator', 'root', ':scope'],
    ['frameLocator.locatorFrom', value.native],
    ['expectText', 5000, 'request count: 1', true, true, false, false, false],
  ]);
});

test('FrameLocator releases its owner Handle when no attached Frame exists', async () => {
  // Arrange
  const value = fixture();
  value.native.contentFrame = undefined;
  value.native.elementHandle = async timeout => {
    value.calls.push(['elementHandle', timeout]);
    return 'native-owner';
  };
  const fallbackOwner = {
    contentFrame: async () => null,
    dispose: async () => { value.calls.push(['dispose']); },
  };
  value.frame._wrapNativeHandle = async nativeOwner => {
    value.calls.push(['wrapNativeHandle', nativeOwner]);
    return fallbackOwner;
  };
  const locator = value.frameLocatorApi.create(value.frame, 'iframe').locator('body');

  // Act
  const operation = locator.textContent();

  // Assert
  await assert.rejects(operation, /iframe has no attached Frame/);
  assert.deepEqual(value.calls, [
    ['locator', 'root', 'iframe'],
    ['elementHandle', 30000],
    ['wrapNativeHandle', 'native-owner'],
    ['dispose'],
  ]);
});

test('FrameLocator composes nested, indexed, semantic, and owner recipes without resolving a Frame', () => {
  // Arrange
  const value = fixture();
  const frameLocator = value.frameLocatorApi.create(value.frame, 'iframe');

  // Act
  const nested = frameLocator.frameLocator('iframe').getByRole('button').toString();
  const indexed = frameLocator.nth(2).locator('span').toString();
  const owner = frameLocator.owner();

  // Assert
  assert.equal(nested, "locator('iframe').contentFrame().locator('iframe').contentFrame().getByRole('button')");
  assert.equal(indexed, "locator('iframe').nth(2).contentFrame().locator('span')");
  assert.equal(owner.toString(), "locator('iframe')");
  assert.deepEqual(value.calls.map(call => call[0]), [
    'locator', 'locator.contentFrame',
    'locator', 'frameLocator.locatorFrom', 'locator.contentFrame',
    'getByRole', 'frameLocator.locatorFrom',
    'locator.nth', 'locator.contentFrame',
    'locator', 'frameLocator.locatorFrom',
  ]);
});

test('FrameLocator rejects a non-string owner selector before retaining state', () => {
  // Arrange
  const value = fixture();

  // Act
  const create = () => value.frameLocatorApi.create(value.frame, 7);

  // Assert
  assert.throws(create, /selector must be a string/);
  assert.deepEqual(value.calls, []);
});
