import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const require = createRequire(join(root, 'tests/package.json'));
const playwrightRequire = createRequire(require.resolve('playwright/package.json'));
const oracleRoot = dirname(playwrightRequire.resolve('playwright-core/package.json'));
assert.equal(require(join(oracleRoot, 'package.json')).version, '1.58.2');
const { mime } = require(join(oracleRoot, 'lib/utilsBundle.js'));
const source = readFileSync(join(root, 'crates/pwright-file-input-fs/src/mime/table.rs'), 'utf8');
const rows = [...source.matchAll(/\(\s*"([^"]+)",\s*"([^"]+)",?\s*\)/g)].map(match => [match[1], match[2]]);
assert.equal(rows.length, 1173);
assert.deepEqual(Object.fromEntries(rows), { ...mime._types });
const build = spawnSync('cargo', ['build', '--release', '--locked', '-p', 'pwright-file-input-fs', '--example', 'mime-lookup'], {
  cwd: root, encoding: 'utf8', maxBuffer: 1024 * 1024,
  env: { ...process.env, CARGO_INCREMENTAL: '0', CARGO_BUILD_JOBS: '1' },
});
if (build.error) throw build.error;
assert.equal(build.status, 0, build.stderr);
const cases = rows.flatMap(([ext]) => [ext, `.${ext}`, `a.${ext}`, `a.${ext.toUpperCase()}`, `a/${ext}`, `a/.${ext}`, `a/b.${ext}`, `a\\b.${ext}`]);
cases.push('', '.', '..', '/', 'a.', 'a.unknown-pwright', 'a.KML', 'a.İ', 'a/b\\.txt');
for (let offset = 0; offset < cases.length; offset += 128) {
  const batch = cases.slice(offset, offset + 128);
  const result = spawnSync(join(root, 'target/release/examples/mime-lookup'), batch, {
    cwd: root, encoding: 'utf8', maxBuffer: 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, result.stderr);
  const lines = result.stdout.trimEnd().split('\n');
  assert.equal(lines.length, batch.length);
  for (let index = 0; index < batch.length; index += 1) {
    assert.equal(lines[index], mime.getType(batch[index]) ?? '-', batch[index]);
  }
}
process.stdout.write(`Pinned MIME table: ${rows.length} mappings; native Rust lookup matches Playwright 1.58.2 on ${cases.length} cases.\n`);
