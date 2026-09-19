// Execute upstream's setters, not a reimplementation of their transaction flow.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, resolve, isAbsolute } from 'node:path';
import { spawnSync } from 'node:child_process';
const requireTests = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireTests('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireTests.resolve('playwright/package.json'));
assert.equal(requireCore('playwright-core/package.json').version, '1.58.2');
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { Page } = requireCore(resolve(core, 'lib/server/page.js'));
const { BrowserContext } = requireCore(resolve(core, 'lib/server/browserContext.js'));
const { mergeHeaders } = requireCore(resolve(core, 'lib/server/network.js'));
// Desired metadata may contain invalid HTTP syntax; native installation is the
// separate failure port, including the no-session and shadowed-value cases.
const samples = [[], [{ name: 'X-A', value: 'one' }], [{ name: 'X-A', value: 'first' }, { name: 'x-a', value: 'last' }], [{ name: 'X-A', value: 'bad\nvalue' }]];
const cases = [], expected = [];
for (const context of [true, false]) for (const prior of samples) for (const next of samples) {
  for (const [fail, compensation_fails] of [[false, false], [true, false], [true, true]]) {
    // Arrange: callbacks are the native outcome port. Metadata methods are real.
    const during = [];
    const primary = new Error('native update failed');
    const compensation = new Error('native compensation failed');
    const owner = context ? { _options: { extraHTTPHeaders: prior } } : { _extraHTTPHeaders: prior };
    const headers = () => context ? owner._options.extraHTTPHeaders : owner._extraHTTPHeaders;
    const apply = async () => {
      during.push(mergeHeaders([headers()]));
      if (during.length === 1 && fail) throw primary;
      if (during.length === 2 && compensation_fails) throw compensation;
    };
    if (context) owner.doUpdateExtraHTTPHeaders = apply;
    else owner.delegate = { updateExtraHTTPHeaders: apply };
    let failed = false;
    // Act
    try {
      const setter = context ? BrowserContext.prototype.setExtraHTTPHeaders : Page.prototype.setExtraHTTPHeaders;
      await setter.call(owner, { race: promise => promise }, next);
    } catch (error) { assert.equal(error, primary); failed = true; }
    // Assert source outcomes before comparing the Rust process.
    assert.equal(failed, fail);
    assert.equal(during.length, fail ? 2 : 1);
    cases.push({ context, prior, next, fail, compensation_fails });
    expected.push({ during, final_headers: mergeHeaders([headers()]), failed });
  }
}
const binary = process.env.PWRIGHT_HEADER_SETTINGS_RUST_ORACLE;
assert.ok(binary && isAbsolute(binary), 'set PWRIGHT_HEADER_SETTINGS_RUST_ORACLE to the built example');
const result = spawnSync(binary, [], { input: JSON.stringify(cases), encoding: 'utf8', timeout: 30_000, maxBuffer: 256 * 1024, killSignal: 'SIGKILL' });
assert.ifError(result.error);
assert.equal(result.status, 0, result.stderr);
const actual = JSON.parse(result.stdout);
assert.equal(actual.length, expected.length);
for (let index = 0; index < actual.length; index++) {
  const { uncertain, ...metadata } = actual[index];
  assert.deepEqual(metadata, expected[index], `metadata transaction ${index}`);
  // Rust-only status records observed native uncertainty without changing the
  // Playwright-compatible desired metadata. Upstream has no equivalent field.
  assert.equal(uncertain, cases[index].fail && cases[index].compensation_fails);
}
console.log(`Playwright 1.58.2 internal header setter metadata: ${cases.length}/${cases.length} matched; native uncertainty tracked separately`);
