// Keyboard-only wire recipe must not pull in the full Chromium/Node installation.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const result = spawnSync('cargo', ['tree', '--locked', '-p', 'pwright-composition-chromium-keyboard',
  '--edges', 'normal', '--prefix', 'none', '--format', '{p}'], {
  cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 4 * 1024 * 1024,
});
if (result.error) throw result.error;
assert.equal(result.status, 0, result.stderr);
const packages = [...new Set(result.stdout.split('\n').map(line => line.split(' ')[0]))].sort();
assert.deepEqual(packages.filter(name => name.startsWith('pwright-')), [
  'pwright-backend-cdp-common', 'pwright-backend-cdp-input', 'pwright-backend-cdp-keyboard',
  'pwright-browser-backend', 'pwright-cap-keyboard', 'pwright-cdp', 'pwright-codec-binary',
  'pwright-codec-common', 'pwright-codec-keyboard', 'pwright-composition-chromium-keyboard',
  'pwright-connection-control', 'pwright-core', 'pwright-input-model', 'pwright-server-keyboard',
  'pwright-server-wire', 'pwright-transport',
]);
for (const forbidden of ['napi', 'napi-derive', 'serde_yaml', 'pwright-schema-gen',
  'pwright-runtime', 'pwright-backend-cdp-locator', 'pwright-node-host', 'pwright-composition-chromium']) {
  assert.equal(packages.includes(forbidden), false, `Keyboard composition must not require ${forbidden}`);
}
console.log('Keyboard composition normal closure: exactly 16 Pwright packages, no full Chromium/runtime/Locator/Node driver');

const controlled = spawnSync('cargo', ['tree', '--locked', '-p', 'pwright-composition-chromium-keyboard',
  '--features', 'runtime', '--edges', 'normal', '--prefix', 'none', '--format', '{p}'], {
  cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 4 * 1024 * 1024,
});
if (controlled.error) throw controlled.error;
assert.equal(controlled.status, 0, controlled.stderr);
const controlledPackages = [...new Set(controlled.stdout.split('\n').map(line => line.split(' ')[0]))].sort();
assert.deepEqual(controlledPackages, [...packages, 'pwright-runtime'].sort());
console.log('Keyboard runtime opt-in adds exactly pwright-runtime: 17 Pwright packages, no new executor or full host');
