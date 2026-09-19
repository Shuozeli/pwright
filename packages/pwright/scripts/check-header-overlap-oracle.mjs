// Pinned real setter methods with controlled native completion promises.
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
const headers = value => value === null ? [] : [{ name: 'X-A', value }];
const cases = [], expected = [];
for (const context of [false, true]) for (const initial_empty of [false, true])
for (const order of [[0, 1], [1, 0]]) for (const fail of [[false, false], [false, true], [true, false], [true, true]])
for (const compensation_fail of [[false, false], [false, true], [true, false], [true, true]]) {
  // Arrange: hold native promises; do not replace upstream metadata logic.
  const initial = headers(initial_empty ? null : 'old');
  const owner = context ? { _options: { extraHTTPHeaders: initial } } : { _extraHTTPHeaders: initial };
  const current = () => (context ? owner._options.extraHTTPHeaders : owner._extraHTTPHeaders)[0]?.value ?? null;
  const native = [], during = [], phases = [];
  const apply = () => {
    during.push(current());
    const completion = Promise.withResolvers();
    native.push(completion);
    return completion.promise;
  };
  if (context) owner.doUpdateExtraHTTPHeaders = apply;
  else owner.delegate = { updateExtraHTTPHeaders: apply };
  const setter = context ? BrowserContext.prototype.setExtraHTTPHeaders : Page.prototype.setExtraHTTPHeaders;
  const primary = [new Error('first native failure'), new Error('second native failure')];
  const calls = ['first', 'second'].map(value => setter.call(owner, { race: promise => promise }, headers(value))
    .then(() => ({ failed: false }), error => ({ failed: true, error })));
  const compensations = [];
  // Act: server-method rejection happens while compensation remains unresolved.
  for (const index of order) {
    if (fail[index]) native[index].reject(primary[index]); else native[index].resolve();
    const result = await calls[index];
    assert.equal(result.failed, fail[index]);
    if (fail[index]) {
      assert.equal(result.error, primary[index]);
      compensations.push({ index, completion: native.at(-1) });
    }
    phases.push(current());
  }
  for (const { index, completion } of compensations.reverse()) {
    if (compensation_fail[index]) completion.reject(new Error('compensation failure'));
    else completion.resolve();
    await Promise.resolve();
    phases.push(current());
  }
  // Assert against the actual Rust model, not a copied JS policy implementation.
  cases.push({ context, initial_empty, order, fail, compensation_fail });
  expected.push({ during, phases, final_value: current() });
}
assert.equal(cases.length, 128);
const binary = process.env.PWRIGHT_HEADER_OVERLAP_RUST_ORACLE;
assert.ok(binary && isAbsolute(binary));
const result = spawnSync(binary, [], { input: JSON.stringify(cases), encoding: 'utf8', timeout: 30_000, maxBuffer: 256 * 1024, killSignal: 'SIGKILL' });
assert.ifError(result.error);
assert.equal(result.status, 0, result.stderr);
assert.deepEqual(JSON.parse(result.stdout), expected);
console.log(`Playwright 1.58.2 internal overlapping header setters: ${cases.length}/${cases.length} matched; server rejection precedes held compensation`);
