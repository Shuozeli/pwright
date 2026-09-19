// Native Keyboard stays optional; generated contract proofs run separately.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const result = spawnSync('cargo', ['tree', '--locked', '-p', 'pwright-backend-cdp-keyboard',
  '--edges', 'normal', '--prefix', 'none', '--format', '{p}'], {
  cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 4 * 1024 * 1024,
});
if (result.error) throw result.error;
assert.equal(result.status, 0, result.stderr);
const packages = [...new Set(result.stdout.split('\n').map(line => line.split(' ')[0]))].sort();
assert.deepEqual(packages.filter(name => name.startsWith('pwright-')), [
  'pwright-backend-cdp-common', 'pwright-backend-cdp-input', 'pwright-backend-cdp-keyboard',
  'pwright-browser-backend', 'pwright-cap-keyboard', 'pwright-cdp', 'pwright-core', 'pwright-input-model',
]);
for (const forbidden of ['napi', 'napi-derive', 'serde_yaml', 'pwright-schema-gen',
  'pwright-runtime', 'pwright-backend-cdp-locator', 'pwright-node-host']) {
  assert.equal(packages.includes(forbidden), false, `Keyboard backend must not require ${forbidden}`);
}
console.log('Keyboard backend normal closure: exactly 8 Pwright packages, no runtime/Locator/Node driver');
