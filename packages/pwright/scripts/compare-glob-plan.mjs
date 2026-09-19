// Offline comparison of the real Rust planner and the pinned oracle compiler.
// This proves pattern planning only, not URL/baseURL matching or browser lookup.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, join, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const { globToRegexPattern } = requireCore(join(core, 'lib/utils/isomorphic/urlMatch.js'));
const cases = [''];
let layer = [''];
for (let depth = 1; depth <= 4; depth++) {
  layer = layer.flatMap(prefix => ['a', '/', '*', '\\', '{', '}', ','].map(c => prefix + c));
  cases.push(...layer);
}
cases.push('?', 'a?q=1', '\\?', '$^+.()|[]', '你好/🦀', '\\🦀/**/你好', 'a/***/b', '{{a,b},c}', '*a'.repeat(8192));
assert.ok(cases.length <= 4096);
const input = JSON.stringify(cases);
assert.ok(Buffer.byteLength(input) <= 512 * 1024);
const result = spawnSync('cargo', ['run', '--locked', '--release', '-p', 'pwright-node-host', '--example', 'compile_frame_globs'], {
  cwd: fileURLToPath(new URL('../../../', import.meta.url)),
  env: { ...process.env, CARGO_INCREMENTAL: '0' },
  input, encoding: 'utf8', timeout: 120_000, maxBuffer: 8 * 1024 * 1024,
});
if (result.stderr) process.stderr.write(result.stderr);
if (result.error) throw result.error;
assert.equal(result.status, 0, 'Rust glob planner bridge must finish successfully');
const patterns = JSON.parse(result.stdout);
assert.equal(patterns.length, cases.length);
const mismatches = [];
for (const [index, glob] of cases.entries()) {
  const oracle = globToRegexPattern(glob);
  if (patterns[index] !== oracle) mismatches.push({ glob, oracle, rust: patterns[index] });
}
const report = { oracle: 'playwright@1.58.2', scope: 'resolved-glob-pattern-only', total: cases.length, matched: cases.length - mismatches.length, mismatches };
if (process.env.PWRIGHT_GLOB_PLAN_REPORT) {
  assert.ok(isAbsolute(process.env.PWRIGHT_GLOB_PLAN_REPORT));
  await writeFile(process.env.PWRIGHT_GLOB_PLAN_REPORT, JSON.stringify(report, null, 2), { flag: 'wx' });
}
console.log(`Rust glob planning: ${report.matched}/${report.total}; no browser or candidate Node lookup claim`);
assert.equal(mismatches.length, 0, JSON.stringify(mismatches.slice(0, 5)));
