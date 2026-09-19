// Explicit offline contract proof; no browser or production runtime starts.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
function cargo(args) {
  const result = spawnSync('cargo', args, { cwd: root, encoding: 'utf8',
    env: { ...process.env, CARGO_INCREMENTAL: '0' }, timeout: 120000, maxBuffer: 4 * 1024 * 1024 });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
}
const closures = {
  cap: ['pwright-cap-mouse', 'pwright-core', 'pwright-input-model'],
  client: ['pwright-browser-model', 'pwright-cap-mouse', 'pwright-client', 'pwright-client-mouse', 'pwright-core', 'pwright-input-model'],
  codec: ['pwright-cap-mouse', 'pwright-codec-binary', 'pwright-codec-common', 'pwright-codec-mouse', 'pwright-core', 'pwright-input-model', 'pwright-transport'],
  server: ['pwright-cap-mouse', 'pwright-codec-binary', 'pwright-codec-common', 'pwright-codec-mouse', 'pwright-connection-control', 'pwright-core', 'pwright-input-model', 'pwright-server-mouse', 'pwright-server-wire', 'pwright-transport'],
};
for (const [kind, expected] of Object.entries(closures)) {
  const tree = cargo(['tree', '--locked', '-p', `pwright-${kind}-mouse`, '--edges', 'normal', '--prefix', 'none', '--format', '{p}']);
  const actual = [...new Set(tree.split('\n').filter(line => line.startsWith('pwright-')).map(line => line.split(' ')[0]))].sort();
  const allPackages = new Set(tree.split('\n').map(line => line.split(' ')[0]));
  for (const forbidden of ['tokio', 'napi', 'napi-derive', 'serde_yaml', 'pwright-schema-gen'])
    assert.equal(allPackages.has(forbidden), false, `${kind} mouse must not require ${forbidden}`);
  assert.deepEqual(actual, expected, `${kind} mouse dependency boundary changed`);
  console.log(`Mouse ${kind} normal closure: ${actual.length} Pwright packages`);
}

// Inventory top-level canonical capability schemas, not frozen old baselines.
// Full YAML/model validation and output freshness belong to the generator below.
const owners = new Map();
for (const directory of readdirSync(join(root, 'crates')).filter(name => name.startsWith('pwright-cap-'))) {
  const schemaDirectory = join(root, 'crates', directory, 'schema');
  let entries;
  try { entries = readdirSync(schemaDirectory); }
  catch (error) { if (error.code === 'ENOENT') continue; throw error; }
  for (const entry of entries.filter(name => name.endsWith('.yml'))) {
    const source = readFileSync(join(schemaDirectory, entry), 'utf8');
    const capability = source.match(/^capability: (.+)$/m)?.[1];
    if (!capability) continue;
    let command;
    for (const line of source.split('\n')) {
      const id = line.match(/^  - id: (.+)$/);
      if (id) command = id[1];
      const method = line.match(/^    method: (\d+)$/);
      if (!method) continue;
      assert.ok(command, `${entry}: command method without identity`);
      const owner = `${capability}/${command}`;
      const previous = owners.get(method[1]);
      assert.ok(previous === undefined || previous === owner, `method ${method[1]} conflicts: ${previous} vs ${owner}`);
      owners.set(method[1], owner);
    }
  }
}
for (const [offset, name] of ['move', 'down', 'up', 'click', 'dblclick', 'wheel'].entries())
  assert.equal(owners.get(String(917505 + offset)), `pwright.mouse/mouse.${name}`);
console.log('Mouse method range ownership verified against canonical capability schemas');

cargo(['run', '--release', '--locked', '-p', 'pwright-schema-gen', '--',
  '--input', 'crates/pwright-cap-mouse/schema/mouse.yml',
  '--output', 'crates/pwright-codec-mouse/src/generated_schema.rs',
  '--capability-output', 'crates/pwright-cap-mouse/src/generated_capability.rs',
  '--server-output', 'crates/pwright-server-mouse/src/generated_server.rs',
  '--reference-output', 'crates/pwright-cap-mouse/schema/protocol-reference.md',
  '--import', 'crates/pwright-codec-common/schema/common.yml', '--check']);
console.log('Mouse schema, capability, codec model, server chain and reference are fresh');
