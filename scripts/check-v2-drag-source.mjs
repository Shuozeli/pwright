// The optional host source must not change the default host/drag dependency sets.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
function packages(feature) {
  const result = spawnSync('cargo', ['tree', '--locked', '-p', 'pwright-node-host',
    '--no-default-features', ...(feature ? ['--features', feature] : []),
    '--edges', 'normal', '--prefix', 'none', '--format', '{p}'], {
    cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 4 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, result.stderr);
  return [...new Set(result.stdout.trim().split('\n').map(line => line.split(' ')[0]))].sort();
}
const base = packages(null);
const selected = packages('drag-source');
assert.deepEqual(packages('drag-runtime'), selected);
assert.equal(base.includes('pwright-backend-cdp-drag'), false);
assert.deepEqual(selected.filter(name => !base.includes(name)), ['pwright-backend-cdp-drag']);
assert.deepEqual(base.filter(name => !selected.includes(name)), []);
for (const name of ['napi', 'napi-derive', 'pwright-node-addon']) {
  assert.equal(selected.includes(name), false, `source must not pull ${name}`);
}
console.log(`Host default/drag-source/drag-runtime closures: ${base.length}/${selected.length}/${selected.length}; exactly the drag leaf is opt-in, no Node-API dependency`);
