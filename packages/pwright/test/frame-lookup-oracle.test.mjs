// Characterize the pinned client algorithm before implementing its Rust port.
// Synthetic retained Frame views exercise pure lookup, not browser conformance.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { Page } = requireCore(join(core, 'lib/client/page.js'));
const { urlMatches, globToRegexPattern } = requireCore(join(core, 'lib/utils/isomorphic/urlMatch.js'));
function frame(name, url) { return { name: () => name, url: () => url }; }
function lookup(frames, selector, baseURL) {
  return Page.prototype.frame.call({ frames: () => [...frames], _browserContext: { _options: { baseURL } } }, selector);
}

test('lookup returns the first matching Frame identity', () => {
  // Arrange
  const first = frame('same', 'https://a.test/');
  const second = frame('same', 'https://b.test/');
  // Act
  const result = lookup([first, second], 'same');
  // Assert
  assert.equal(result, first);
});

test('truthy name takes precedence over URL evaluation', () => {
  // Arrange
  const selected = frame('named', 'about:blank');
  let calls = 0;
  // Act
  const result = lookup([selected], { name: 'named', url: () => { calls++; return false; } });
  // Assert
  assert.equal(result, selected);
  assert.equal(calls, 0);
});

test('an empty name with a URL selector uses the URL branch', () => {
  // Arrange
  const selected = frame('', 'about:blank');
  // Act
  const result = lookup([selected], { name: '', url: 'about:blank' });
  // Assert
  assert.equal(result, selected);
});

test('empty selectors reject before enumerating Frames', () => {
  // Arrange
  const selectors = ['', {}, { name: '' }, { url: '' }];
  // Act / Assert
  for (const selector of selectors) assert.throws(() => lookup([], selector), /Either name or url matcher should be specified/);
});

test('missing names return null even when URL would match', () => {
  // Arrange
  const selected = frame('other', 'about:blank');
  // Act
  const result = lookup([selected], { name: 'missing', url: 'about:blank' });
  // Assert
  assert.equal(result, null);
});

test('the URL property getter is read even when name takes precedence', () => {
  // Arrange
  const failure = new Error('getter failure');
  const selector = { name: 'named', get url() { throw failure; } };
  // Act / Assert
  assert.throws(() => lookup([frame('named', 'about:blank')], selector), error => error === failure);
});

test('predicate receives a WHATWG URL and skips invalid URLs', () => {
  // Arrange
  const invalid = frame('', '');
  const valid = frame('', 'https://EXAMPLE.test/path?q=1');
  const seen = [];
  // Act
  const result = lookup([invalid, valid], { url: value => { seen.push(value); return value.pathname === '/path'; } });
  // Assert
  assert.equal(result, valid);
  assert.equal(seen.length, 1);
  assert.ok(seen[0] instanceof URL);
  assert.equal(seen[0].hostname, 'example.test');
});

test('synchronous lookup uses callback truthiness without awaiting a Promise', () => {
  // Arrange
  const selected = frame('', 'about:blank');
  // Act
  const result = lookup([selected], { url: () => Promise.resolve(false) });
  // Assert
  assert.equal(result, selected);
});

test('predicate exceptions retain their identity', () => {
  // Arrange
  const failure = new Error('predicate failure');
  // Act / Assert
  assert.throws(() => lookup([frame('', 'about:blank')], { url: () => { throw failure; } }), error => error === failure);
});

test('the pinned oracle preserves RegExp lastIndex between calls', () => {
  // Arrange
  const selected = frame('', 'about:blank');
  const pattern = /about/g;
  // Act
  const results = [lookup([selected], { url: pattern }), lookup([selected], { url: pattern }), lookup([selected], { url: pattern })];
  // Assert
  assert.deepEqual(results, [selected, null, selected]);
  assert.equal(pattern.lastIndex, 5);
});

test('single-star and deep-star URL globs have different slash behavior', () => {
  // Arrange
  const url = 'https://example.test/a/b';
  // Act
  const values = [urlMatches(undefined, url, 'https://example.test/*'), urlMatches(undefined, url, 'https://example.test/**')];
  // Assert
  assert.deepEqual(values, [false, true]);
});

test('query question marks are literal and relative globs resolve against baseURL', () => {
  // Arrange
  const url = 'https://example.test/root/a?q=1';
  // Act
  const values = [urlMatches('https://example.test/root/', url, './a?q=1'), urlMatches(undefined, 'https://example.test/axq=1', 'https://example.test/a?q=1')];
  // Assert
  assert.deepEqual(values, [true, false]);
});

test('pinned glob conversion does not use the newer nested-brace validation', () => {
  // Arrange
  const glob = '{{a,b},c}';
  // Act
  const pattern = globToRegexPattern(glob);
  // Assert
  assert.equal(pattern, '^((a|b)\\,c)$');
});

test('unsupported URL matcher types are checked only for a valid Frame URL', () => {
  // Arrange
  const invalid = frame('', '');
  const valid = frame('', 'about:blank');
  // Act / Assert
  assert.equal(lookup([invalid], { url: {} }), null);
  assert.throws(() => lookup([valid], { url: {} }), /url parameter should be string, RegExp or function/);
});
