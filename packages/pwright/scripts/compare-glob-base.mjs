// Real Rust preprocessing + diagnostic URL parser versus pinned URL-glob policy.
// Does not claim parser allocation admission or public Node lookup integration.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { globBaseCases } from './glob-base-cases.mjs';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { resolveGlobToRegexPattern } = requireCore(join(core, 'lib/utils/isomorphic/urlMatch.js'));
const cases = globBaseCases();
const input = JSON.stringify(cases);
assert.ok(cases.length <= 4096 && Buffer.byteLength(input) <= 512 * 1024);
const result = spawnSync('cargo', ['run', '--locked', '--release', '-p', 'pwright-node-host', '--example', 'compile_frame_url_globs'], {
  cwd: fileURLToPath(new URL('../../../', import.meta.url)), env: { ...process.env, CARGO_INCREMENTAL: '0' },
  input, encoding: 'utf8', timeout: 120_000, maxBuffer: 8 * 1024 * 1024,
});
if (result.stderr) process.stderr.write(result.stderr);
if (result.error) throw result.error;
assert.equal(result.status, 0, 'Rust URL-glob bridge must finish successfully');
const patterns = JSON.parse(result.stdout);
assert.equal(patterns.length, cases.length);
const mismatches = [];
for (const [index, sample] of cases.entries()) {
  const oracle = resolveGlobToRegexPattern(sample.baseURL, sample.glob);
  if (patterns[index] !== oracle) mismatches.push({ ...sample, oracle, rust: patterns[index] });
}
const report = { oracle: 'playwright@1.58.2', scope: 'URL-glob-policy-with-diagnostic-Rust-URL-parser', total: cases.length, matched: cases.length - mismatches.length, mismatches };
if (process.env.PWRIGHT_GLOB_BASE_REPORT) {
  assert.ok(isAbsolute(process.env.PWRIGHT_GLOB_BASE_REPORT));
  await writeFile(process.env.PWRIGHT_GLOB_BASE_REPORT, JSON.stringify(report, null, 2), { flag: 'wx' });
}
console.log(`Rust URL glob policy: ${report.matched}/${report.total}`);
assert.equal(mismatches.length, 0, JSON.stringify(mismatches.slice(0, 5)));
