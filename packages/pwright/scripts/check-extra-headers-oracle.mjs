// Internal differential only. Executes pinned upstream production functions
// with a rejecting native-navigation port; does not claim browser integration.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { EventEmitter } from 'node:events';
import { spawnSync } from 'node:child_process';

const root = fileURLToPath(new URL('../../..', import.meta.url));
const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requirePlaywright = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requirePlaywright.resolve('playwright-core/package.json'));
assert.equal(requirePlaywright('playwright-core/package.json').version, '1.58.2');
const { mergeHeaders } = requirePlaywright(resolve(core, 'lib/server/network.js'));
const { Frame } = requirePlaywright(resolve(core, 'lib/server/frames.js'));
const header = (name, value) => ({ name, value });
const lists = [
  [], [header('X-A', 'a')], [header('x-a', 'b')],
  [header('X-A', 'a'), header('x-a', 'b')],
  [header('X-B', 'b'), header('X-A', 'a')],
  [header('X-A', 'a'), header('X-B', 'b'), header('x-a', 'last')],
  [header('Referer', 'https://example.test/first')],
  [header('referer', '')],
  [header('Referer', 'https://example.test/first'), header('referer', 'https://example.test/last')],
  [header('referer', ''), header('REFERER', 'https://example.test/last')],
  [header('X-UTF8', 'é\tvalue'), header('X-Empty', '')],
];
const cases = [];
const expected = [];
const nativeBoundary = new Error('intentional native boundary');
for (const context of lists) for (const page of lists) {
  for (const option of [null, '', 'https://example.test/first', 'https://example.test/last']) {
    // Arrange: only the native port and progress/event plumbing are fakes.
    const frame = new EventEmitter();
    let referer = null, conflict = false, dispatched = false;
    frame._page = {
      extraHTTPHeaders: () => page,
      delegate: { navigateFrame: (_frame, _url, value) => {
        dispatched = true;
        referer = value ?? null;
        return Promise.reject(nativeBoundary);
      } },
    };
    const progress = { log() {}, race: promise => promise };
    const options = { waitUntil: 'commit', ...(option === null ? {} : { referer: option }) };
    // Act: execute upstream's own first-header selection/conflict check.
    try {
      await Frame.prototype.gotoImpl.call(frame, progress, 'https://example.test/document', options);
      assert.fail('native port must stop navigation');
    } catch (error) {
      if (error !== nativeBoundary) {
        assert.equal(error.message, '"referer" is already specified as extra HTTP header');
        assert.equal(dispatched, false);
        conflict = true;
      }
    }
    cases.push({ context, page, option });
    expected.push({ headers: mergeHeaders([context, page]), referer, conflict });
  }
}
const binary = process.env.PWRIGHT_HEADERS_RUST_ORACLE;
assert.ok(binary && binary.startsWith('/'), 'set PWRIGHT_HEADERS_RUST_ORACLE to the built absolute example path');
const result = spawnSync(binary, [], {
  cwd: root, input: JSON.stringify(cases), encoding: 'utf8',
  timeout: 30_000, maxBuffer: 1024 * 1024, killSignal: 'SIGKILL',
});
assert.ifError(result.error);
assert.equal(result.status, 0, result.stderr);
const actual = JSON.parse(result.stdout);
// Assert: exact ordering, spelling, values, referrer and conflict result.
assert.equal(actual.length, expected.length);
for (let index = 0; index < expected.length; index++) {
  assert.deepEqual(actual[index], expected[index], `header policy case ${index}`);
}
console.log(`Playwright 1.58.2 internal header merge/navigation policy: ${cases.length}/${cases.length} matched`);
