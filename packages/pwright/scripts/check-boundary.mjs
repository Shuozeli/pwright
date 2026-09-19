import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import './check-native-class-boundary.mjs';

const workspace = fileURLToPath(new URL('../../../', import.meta.url));
const retainedReferenceLeaves = [
  'pwright-composition-chromium-handle-evaluation',
  'pwright-composition-chromium-locator-resolution',
  'pwright-backend-cdp-locator-resolution',
];
const retainedPreflightLeaves = [
  'pwright-backend-cdp-file-input',
  'pwright-composition-chromium-file-input',
];
function effectiveSelection(feature) {
  const selected = new Set(feature.split(',').filter(Boolean));
  if (selected.has('retained-serialized-payload-mutation')) selected.add('retained-serialized-payload-import');
  if (selected.has('retained-serialized-payload-import')) selected.add('retained-call-argument-serialization');
  if (selected.has('retained-call-argument-serialization')) selected.add('retained-input-instrumentation');
  if (selected.has('retained-input-instrumentation')) selected.add('retained-target-prevalidation');
  if (selected.has('retained-target-prevalidation')) selected.add('retained-target-marking');
  if (selected.has('retained-target-marking')) selected.add('retained-file-input');
  if (selected.has('retained-file-input')) {
    selected.add('handles');
    selected.add('file-input');
  }
  return [...selected];
}
function checkSerializedPayloadMutation(lines, enabled) {
  for (const name of ['pwright-node-addon', 'pwright-node-host',
    'pwright-composition-chromium-file-input', 'pwright-backend-cdp-file-input']) {
    const line = lines.find(value => value.startsWith(`${name} v`));
    if (!line) {
      assert.equal(enabled, false, `${name} must be present for serialized payload mutation`);
      continue;
    }
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('retained-serialized-payload-mutation'), enabled,
      `${name}/retained-serialized-payload-mutation requires explicit selection`);
  }
}
function checkSerializedPayloadImport(lines, enabled) {
  for (const name of ['pwright-node-addon', 'pwright-node-host']) {
    const line = lines.find(value => value.startsWith(`${name} v`));
    if (!line) {
      assert.equal(enabled, false, `${name} must be present for serialized payload import`);
      continue;
    }
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('retained-serialized-payload-import'), enabled,
      `${name}/retained-serialized-payload-import requires explicit selection`);
  }
}
function checkCallArgumentSerialization(lines, enabled) {
  const line = lines.find(value => value.startsWith('pwright-node-addon v'));
  if (!line) {
    assert.equal(enabled, false, 'call-argument serialization is an addon-only leaf');
    return;
  }
  const features = line.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(features.includes('retained-call-argument-serialization'), enabled,
    'addon retained-call-argument-serialization requires explicit selection');
}
function checkInputInstrumentation(lines, enabled) {
  const line = lines.find(value => value.startsWith('pwright-node-addon v'));
  if (!line) {
    assert.equal(enabled, false, 'input instrumentation is an addon-only leaf');
    return;
  }
  const features = line.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(features.includes('retained-input-instrumentation'), enabled,
    'addon retained-input-instrumentation requires explicit selection');
}
function checkTargetPrevalidation(lines, enabled) {
  const expected = new Map([
    ['pwright-node-host', ['retained-target-prevalidation']],
    ['pwright-composition-chromium-file-input', ['retained-preflight', 'retained-target-prevalidation']],
    ['pwright-backend-cdp-file-input', ['retained-preflight']],
  ]);
  for (const [name, leaves] of expected) {
    const line = lines.find(value => value.startsWith(`${name} v`));
    if (!line) {
      assert.equal(enabled, false, `${name} must be present for target prevalidation`);
      continue;
    }
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    for (const leaf of leaves)
      assert.equal(features.includes(leaf), enabled,
        `${name}/${leaf} requires explicit retained-target-prevalidation selection`);
  }
}
function checkTargetMarking(lines, enabled) {
  for (const name of ['pwright-node-host', 'pwright-backend-cdp-file-input',
    'pwright-composition-chromium-file-input']) {
    const line = lines.find(value => value.startsWith(`${name} v`));
    if (!line) {
      assert.equal(enabled, false, `${name} must be present for target marking`);
      continue;
    }
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('retained-target-marking'), enabled,
      `${name}/retained-target-marking requires the explicit marker selection`);
  }
}
function tree(feature, target = 'pwright-node-host') {
  const args = ['tree', '--locked', '-p', target, '--edges', 'normal', '--prefix', 'none', '--format', '{p} features:{f}'];
  if (feature) args.push('--features', feature);
  const result = spawnSync('cargo', args, { cwd: workspace, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
}
function checkCloseCoordinator(lines, selected) {
  for (const [name, forbidden] of [
    ['pwright-composition-chromium', ['close-capacity', 'native-close-authority', 'native-close-scheduler', 'close-route-retirement', 'native-close-retirement']],
    ['pwright-runtime', ['disposal-reservations']],
    ['pwright-backend-cdp-topology', ['transition-reservations']],
  ]) {
    const line = lines.find(value => value.startsWith(`${name} v`));
    assert.ok(line, `${name} must be present`);
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    for (const feature of forbidden)
      assert.equal(features.includes(feature), selected.includes('concurrent-close') || (name === 'pwright-runtime' && selected.includes('handles')), `${name}/${feature} requires its explicit close or Handle selection`);
  }
}
function checkNativeHandleRelease(lines, selected) {
  const line = lines.find(value => value.startsWith('pwright-composition-chromium-handle-lifecycle v'));
  assert.ok(line, 'existing Handle lifecycle leaf must be present');
  assert.equal(line.split(' features:')[1].split(' ')[0].split(',').includes('native-release'), selected.includes('handles'),
    'native-release requires explicit host handles selection');
  const creation = lines.find(value => value.startsWith('pwright-composition-chromium-handle-evaluation v'));
  assert.ok(creation, 'existing Handle evaluation leaf must be present');
  assert.equal(creation.split(' features:')[1].split(' ')[0].split(',').includes('native-creation'), selected.includes('handles'),
    'native-creation requires explicit host handles selection');
  assert.equal(creation.split(' features:')[1].split(' ')[0].split(',').includes('frame-value'), selected.includes('handles'),
    'Frame value evaluation with Handle arguments requires explicit host handles selection');
  assert.equal(creation.split(' features:')[1].split(' ')[0].split(',').includes('handle-value'), selected.includes('handles'),
    'Native Handle value programs require explicit host handles selection');
}
function checkRetainedReference(lines, enabled, prevalidationEnabled) {
  const selectedLeaves = new Set();
  const selectedPreflightLeaves = new Set();
  for (const line of lines) {
    const name = line.split(' ')[0];
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    if (features.includes('retained-preflight')) selectedPreflightLeaves.add(name);
    if (features.includes('retained-reference')) selectedLeaves.add(name);
    if (name === 'pwright-node-host')
      assert.equal(features.includes('retained-file-input'), enabled,
        'retained-file-input must remain an explicit host/addon selection');
  }
  assert.deepEqual([...selectedLeaves].sort(), enabled ? [...retainedReferenceLeaves].sort() : [],
    'retained-reference may select exactly the three retained-upload leaves, only for the retained-upload opt-in');
  assert.deepEqual([...selectedPreflightLeaves].sort(),
    prevalidationEnabled ? [...retainedPreflightLeaves].sort() : [],
    'retained-preflight may select exactly its backend/composition leaves, only for target prevalidation');
}
// These leaves already occur transitively in the aggregate composition. Handle
// selection adds no packages and only its five explicit Handle/lifecycle features below
// the host. Check both ordinary and upload-composed graphs exactly.
for (const base of ['', 'file-input']) {
  const fingerprint = feature => [...new Set(tree(feature).trim().split('\n')
    .filter(line => !line.startsWith('pwright-node-host v'))
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      if (!feature.split(',').includes('handles')) return line;
      const [pkg, selected] = line.split(' features:');
      const removed = pkg.startsWith('pwright-runtime v') ? ['disposal-reservations']
        : pkg.startsWith('pwright-composition-chromium-handle-lifecycle v') ? ['native-release']
        : pkg.startsWith('pwright-composition-chromium-handle-evaluation v') ? ['native-creation', 'frame-value', 'handle-value'] : [];
      return removed.length ? `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).join(',')}` : line;
    }))].sort();
  assert.deepEqual(fingerprint(base ? `handles,${base}` : 'handles'), fingerprint(base),
    'host handles selection may add only its four declared Handle/lifecycle features');
}
// Preserve every normal package and feature (including host features) except
// the host/addon opt-ins and their three leaf flags. This is stronger than comparing
// package counts and also catches unrelated feature widening in combined builds.
for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
    const fingerprint = (feature, stripRetained) => [...new Set(tree(feature, target).trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = !stripRetained ? [] : ['pwright-node-host', 'pwright-node-addon'].includes(name) ? ['retained-file-input']
        : retainedReferenceLeaves.includes(name) ? ['retained-reference'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
    assert.deepEqual(fingerprint(`retained-file-input${other}`, true), fingerprint(`handles,file-input${other}`, false),
      `${target} retained-file-input may add no normal packages or features beyond its host/addon flags and three retained-reference flags`);
  }
  for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
    const fingerprint = (feature, stripMarking) => [...new Set(tree(feature, target).trim().split('\n')
      .map(line => line.replace(/ \(\*\)$/, ''))
      .map(line => {
        const [pkg, selected] = line.split(' features:');
        const name = pkg.split(' ')[0];
        const removed = !stripMarking ? []
          : ['pwright-node-host', 'pwright-node-addon', 'pwright-backend-cdp-file-input',
              'pwright-composition-chromium-file-input'].includes(name)
            ? ['retained-target-marking'] : [];
        return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
      }))].sort();
    assert.deepEqual(fingerprint(`retained-target-marking${other}`, true),
      fingerprint(`retained-file-input${other}`, false),
      `${target} retained-target-marking may add only its host/addon/backend/composition feature flags`);
  }
  for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
    const fingerprint = (feature, stripPrevalidation) => [...new Set(tree(feature, target).trim().split('\n')
      .map(line => line.replace(/ \(\*\)$/, ''))
      .map(line => {
        const [pkg, selected] = line.split(' features:');
        const name = pkg.split(' ')[0];
        const removed = !stripPrevalidation ? []
          : ['pwright-node-host', 'pwright-node-addon'].includes(name)
            ? ['retained-target-prevalidation']
            : name === 'pwright-composition-chromium-file-input'
              ? ['retained-preflight', 'retained-target-prevalidation']
              : name === 'pwright-backend-cdp-file-input' ? ['retained-preflight'] : [];
        return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
      }))].sort();
    assert.deepEqual(fingerprint(`retained-target-prevalidation${other}`, true),
      fingerprint(`retained-target-marking${other}`, false),
      `${target} retained-target-prevalidation may add only its host/addon/backend/composition feature flags`);
  }
}
for (const feature of ['', 'drag-source', 'drag-runtime', 'frame-navigation', 'drag-runtime,frame-navigation', 'extra-headers', 'extra-headers,drag-runtime,frame-navigation', 'concurrent-close', 'concurrent-close,extra-headers,drag-runtime,frame-navigation', 'file-input', 'file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'handles', 'handles,file-input', 'handles,file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'retained-file-input', 'retained-file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'retained-target-marking', 'retained-target-marking,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'retained-target-prevalidation', 'retained-target-prevalidation,concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
  const selected = effectiveSelection(feature);
  const lines = tree(feature).trim().split('\n');
  checkCloseCoordinator(lines, selected);
  checkNativeHandleRelease(lines, selected);
  checkRetainedReference(lines, selected.includes('retained-file-input'),
    selected.includes('retained-target-prevalidation'));
  checkTargetMarking(lines, selected.includes('retained-target-marking'));
  checkTargetPrevalidation(lines, selected.includes('retained-target-prevalidation'));
  const packages = new Set(lines.map(line => line.split(' ')[0]));
  for (const upload of ['pwright-file-input-model', 'pwright-file-input-source', 'pwright-file-input-fs', 'pwright-file-input-tokio', 'pwright-backend-cdp-file-input', 'pwright-composition-chromium-file-input'])
    assert.equal(packages.has(upload), selected.includes('file-input'), `${upload} requires file-input selection`);
  const cdp = lines.find(line => line.startsWith('pwright-cdp v'));
  assert.equal(cdp.split(' features:')[1].split(' ')[0].split(',').includes('large-commands'), selected.includes('file-input'));
  assert.equal(lines.some(line => line.startsWith('pwright-network-model v') && line.split(' features:')[1].split(' ')[0].split(',').includes('extra-headers')), selected.includes('extra-headers'));
  for (const forbidden of ['napi', 'napi-derive', 'pwright-node-addon', 'pwright-server', 'pwright-bridge', 'pwright-cli', 'pwright-script'])
    assert.equal(packages.has(forbidden), false, `safe host must not depend on ${forbidden}`);
  assert.equal(packages.has('pwright-backend-cdp-drag'), selected.some(value => value.startsWith('drag-')));
  const runtime = lines.find(line => line.startsWith('pwright-runtime v'));
  assert.ok(runtime, 'runtime package must be present');
  const handoff = runtime.split(' features:')[1].split(' ')[0].split(',').includes('lifetime-handoff');
  assert.equal(handoff, selected.includes('drag-runtime'), `${feature || 'default'} lifetime-handoff selection`);
  assert.equal(lines.some(line => line.startsWith('pwright-cap-navigation v') && line.split(' features:')[1].split(' ')[0].split(',').includes('frame-navigation')), selected.includes('frame-navigation'));
  console.log(`Safe Rust host ${feature || 'default'} boundary verified (${packages.size} packages including host); lifetime-handoff=${handoff}.`);
}

const addonManifest = readFileSync(new URL('../../../crates/pwright-node-addon/Cargo.toml', import.meta.url), 'utf8');
const retainedAddonFeature = /^retained-file-input\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(retainedAddonFeature, 'the addon retained-file-input feature must be declared explicitly');
assert.deepEqual([...retainedAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['file-input', 'handles', 'pwright-node-host/retained-file-input'],
  'addon retained-file-input must select only its two implied addon features and the host opt-in');
const markingAddonFeature = /^retained-target-marking\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(markingAddonFeature, 'the addon retained-target-marking feature must be declared explicitly');
assert.deepEqual([...markingAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-node-host/retained-target-marking', 'retained-file-input'],
  'addon retained-target-marking must select only retained preparation and the host marker leaf');
const prevalidationAddonFeature = /^retained-target-prevalidation\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(prevalidationAddonFeature, 'the addon retained-target-prevalidation feature must be declared explicitly');
assert.deepEqual([...prevalidationAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-node-host/retained-target-prevalidation', 'retained-target-marking'],
  'addon retained-target-prevalidation must select only marking and the host prevalidation leaf');
const instrumentationAddonFeature = /^retained-input-instrumentation\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(instrumentationAddonFeature, 'the addon retained-input-instrumentation feature must be declared explicitly');
assert.deepEqual([...instrumentationAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['retained-target-prevalidation'],
  'addon retained-input-instrumentation must select only target prevalidation');
const serializationAddonFeature = /^retained-call-argument-serialization\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(serializationAddonFeature, 'the addon retained-call-argument-serialization feature must be declared explicitly');
assert.deepEqual([...serializationAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['retained-input-instrumentation'],
  'addon retained-call-argument-serialization must select only input instrumentation');
const importAddonFeature = /^retained-serialized-payload-import\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(importAddonFeature, 'the addon retained-serialized-payload-import feature must be declared explicitly');
assert.deepEqual([...importAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['dep:serde_json', 'pwright-node-host/retained-serialized-payload-import',
    'retained-call-argument-serialization'].sort(),
  'addon retained-serialized-payload-import must select only serialization, JSON and the host owner leaf');
const mutationAddonFeature = /^retained-serialized-payload-mutation\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(mutationAddonFeature, 'the addon retained-serialized-payload-mutation feature must be declared explicitly');
assert.deepEqual([...mutationAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-node-host/retained-serialized-payload-mutation',
    'retained-serialized-payload-import'].sort(),
  'addon retained-serialized-payload-mutation must select only import and the host mutation leaf');
const hostManifest = readFileSync(new URL('../../../crates/pwright-node-host/Cargo.toml', import.meta.url), 'utf8');
const prevalidationHostFeature = /^retained-target-prevalidation\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(prevalidationHostFeature, 'the host retained-target-prevalidation feature must be declared explicitly');
assert.deepEqual([...prevalidationHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-composition-chromium-file-input/retained-target-prevalidation', 'retained-target-marking'],
  'host retained-target-prevalidation must select only marking and composition prevalidation');
const importHostFeature = /^retained-serialized-payload-import\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(importHostFeature, 'the host retained-serialized-payload-import feature must be declared explicitly');
assert.deepEqual([...importHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['retained-target-prevalidation'],
  'host retained-serialized-payload-import must select only target prevalidation');
const mutationHostFeature = /^retained-serialized-payload-mutation\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(mutationHostFeature, 'the host retained-serialized-payload-mutation feature must be declared explicitly');
assert.deepEqual([...mutationHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-composition-chromium-file-input/retained-serialized-payload-mutation',
    'retained-serialized-payload-import'].sort(),
  'host retained-serialized-payload-mutation must select only import and the composition mutation leaf');
for (const feature of ['', 'handles', 'drag-runtime', 'frame-navigation', 'drag-runtime,frame-navigation',
  'extra-headers', 'drag-runtime,extra-headers', 'frame-navigation,extra-headers', 'drag-runtime,frame-navigation,extra-headers', 'concurrent-close', 'concurrent-close,drag-runtime,frame-navigation,extra-headers', 'file-input', 'file-input,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'handles,file-input', 'handles,file-input,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-file-input', 'retained-file-input,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-target-marking', 'retained-target-marking,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-target-prevalidation', 'retained-target-prevalidation,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-input-instrumentation', 'retained-input-instrumentation,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-call-argument-serialization', 'retained-call-argument-serialization,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-serialized-payload-import', 'retained-serialized-payload-import,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-serialized-payload-mutation', 'retained-serialized-payload-mutation,concurrent-close,drag-runtime,frame-navigation,extra-headers']) {
  const selected = effectiveSelection(feature);
  const lines = tree(feature, 'pwright-node-addon').trim().split('\n');
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(host, 'addon must use the safe host');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('handles'), selected.includes('handles'),
    'Node Handle ownership requires explicit handles selection');
  checkCloseCoordinator(lines, selected);
  checkNativeHandleRelease(lines, selected);
  checkRetainedReference(lines, selected.includes('retained-file-input'),
    selected.includes('retained-target-prevalidation'));
  checkTargetMarking(lines, selected.includes('retained-target-marking'));
  checkTargetPrevalidation(lines, selected.includes('retained-target-prevalidation'));
  checkInputInstrumentation(lines, selected.includes('retained-input-instrumentation'));
  checkCallArgumentSerialization(lines, selected.includes('retained-call-argument-serialization'));
  checkSerializedPayloadImport(lines, selected.includes('retained-serialized-payload-import'));
  checkSerializedPayloadMutation(lines, selected.includes('retained-serialized-payload-mutation'));
  const packages = new Set(lines.map(line => line.split(' ')[0]));
  for (const upload of ['pwright-file-input-model', 'pwright-file-input-source', 'pwright-file-input-fs', 'pwright-file-input-tokio', 'pwright-backend-cdp-file-input', 'pwright-composition-chromium-file-input'])
    assert.equal(packages.has(upload), selected.includes('file-input'), `${upload} requires addon file-input selection`);
  const cdp = lines.find(line => line.startsWith('pwright-cdp v'));
  assert.equal(cdp.split(' features:')[1].split(' ')[0].split(',').includes('large-commands'), selected.includes('file-input'));
  assert.equal(lines.some(line => line.startsWith('pwright-network-model v') && line.split(' features:')[1].split(' ')[0].split(',').includes('extra-headers')), selected.includes('extra-headers'));
  assert.equal(packages.has('pwright-backend-cdp-drag'), selected.includes('drag-runtime'));
  const runtime = lines.find(line => line.startsWith('pwright-runtime v'));
  assert.ok(runtime);
  assert.equal(runtime.split(' features:')[1].split(' ')[0].split(',').includes('lifetime-handoff'), selected.includes('drag-runtime'));
  assert.equal(lines.some(line => line.startsWith('pwright-cap-navigation v') && line.split(' features:')[1].split(' ')[0].split(',').includes('frame-navigation')), selected.includes('frame-navigation'));
  console.log(`Node addon ${feature || 'default'} resolved selection verified (${packages.size} packages including addon).`);
}

for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
  const fingerprint = (feature, stripMutation) => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripMutation && ['pwright-node-addon', 'pwright-node-host',
        'pwright-composition-chromium-file-input', 'pwright-backend-cdp-file-input'].includes(name)
        ? ['retained-serialized-payload-mutation'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint(`retained-serialized-payload-mutation${other}`, true),
    fingerprint(`retained-serialized-payload-import${other}`, false),
    'serialized payload mutation may add only its exact addon, host, composition and backend flags');
}

for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
  const fingerprint = (feature, stripImport) => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripImport && ['pwright-node-addon', 'pwright-node-host'].includes(name)
        ? ['retained-serialized-payload-import'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint(`retained-serialized-payload-import${other}`, true),
    fingerprint(`retained-call-argument-serialization${other}`, false),
    'serialized payload import may add only its exact addon and host owner flags');
}

for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
  const fingerprint = (feature, stripSerialization) => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripSerialization && name === 'pwright-node-addon'
        ? ['retained-call-argument-serialization'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint(`retained-call-argument-serialization${other}`, true),
    fingerprint(`retained-input-instrumentation${other}`, false),
    'addon retained-call-argument-serialization may add only its own phase flag');
}

for (const other of ['', ',concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
  const fingerprint = (feature, stripInstrumentation) => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripInstrumentation && name === 'pwright-node-addon'
        ? ['retained-input-instrumentation'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint(`retained-input-instrumentation${other}`, true),
    fingerprint(`retained-target-prevalidation${other}`, false),
    'addon retained-input-instrumentation may add only its own phase flag');
}

const source = new URL('../src/', import.meta.url);
for (const file of readdirSync(source).filter(file => /\.(cjs|mjs)$/.test(file))) {
  const text = readFileSync(new URL(file, source), 'utf8');
  assert.doesNotMatch(text, /(?:require\(['"](?:playwright(?:-core)?|(?:node:)?(?:child_process|net|http|https)|ws)['"]\)|from\s+['"]playwright(?:-core)?['"])/);
}
console.log('Runtime JS contains no driver, CDP transport, or process launcher imports.');
