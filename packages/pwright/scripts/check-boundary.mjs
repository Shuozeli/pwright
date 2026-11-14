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
  if (selected.has('response-request')) selected.add('frame-navigation');
  if (selected.has('request-post-data-json')) selected.add('request-events');
  if (selected.has('request-events')) selected.add('url-glob');
  if (selected.has('response-finished')) selected.add('response-events');
  if (selected.has('response-events')) selected.add('url-glob');
  if (selected.has('touchscreen-input')) selected.add('device-emulation');
  if (selected.has('element-handle-dom')) selected.add('handles');
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
function checkRequestEvents(lines, eventsEnabled, jsonEnabled, networkEventsEnabled = eventsEnabled || jsonEnabled) {
  const addon = lines.find(value => value.startsWith('pwright-node-addon v'));
  if (addon) {
    const features = addon.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('request-events'), eventsEnabled,
      'addon request-events requires explicit selection');
    assert.equal(features.includes('request-post-data-json'), jsonEnabled,
      'addon request-post-data-json requires explicit selection');
    if (eventsEnabled) assert.equal(features.includes('url-glob'), true,
      'addon request events require the shared URL-glob compiler');
  }
  const host = lines.find(value => value.startsWith('pwright-node-host v'));
  assert.ok(host, 'request event boundary requires the safe host');
  const features = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(features.includes('request-events'), eventsEnabled,
    'host request-events must follow addon selection');
  assert.equal(features.includes('request-post-data-json'), jsonEnabled,
    'host request-post-data-json must follow addon selection');
  assert.equal(features.includes('network-events'), networkEventsEnabled,
    'request observation or parsing requires the bounded Rust network lifecycle');
  if (addon && eventsEnabled) assert.equal(features.includes('url-glob'), true,
    'the addon request-event boundary composes the shared Rust URL-glob compiler');
  if (eventsEnabled || jsonEnabled) {
    for (const forbidden of ['page-routes', 'response-body', 'request-history'])
      assert.equal(features.includes(forbidden), false,
        `request events must not select ${forbidden}`);
  }
}
function checkResponseEvents(lines, enabled, networkEventsEnabled = enabled) {
  const addon = lines.find(value => value.startsWith('pwright-node-addon v'));
  if (addon) {
    const features = addon.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('response-events'), enabled,
      'addon response-events requires explicit selection');
    if (enabled) assert.equal(features.includes('url-glob'), true,
      'addon response events require the shared URL-glob compiler');
  }
  const host = lines.find(value => value.startsWith('pwright-node-host v'));
  assert.ok(host, 'response event boundary requires the safe host');
  const features = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(features.includes('response-events'), enabled,
    'host response-events must follow addon selection');
  assert.equal(features.includes('network-events'), networkEventsEnabled,
    'response observation requires the bounded Rust network lifecycle');
  if (addon && enabled) assert.equal(features.includes('url-glob'), true,
    'the addon response-event boundary composes the shared Rust URL-glob compiler');
  if (enabled) {
    for (const forbidden of ['page-routes', 'response-body', 'request-history'])
      assert.equal(features.includes(forbidden), false,
        `response events must not select ${forbidden}`);
  }
}
function checkResponseFinished(lines, enabled) {
  const addon = lines.find(value => value.startsWith('pwright-node-addon v'));
  if (addon) {
    const features = addon.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('response-finished'), enabled,
      'addon response-finished requires explicit selection');
  }
  const host = lines.find(value => value.startsWith('pwright-node-host v'));
  assert.ok(host, 'response-finished boundary requires the safe host');
  const features = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(features.includes('response-finished'), enabled,
    'host response-finished must follow addon selection');
  if (enabled) {
    assert.equal(features.includes('response-events'), true,
      'response-finished must extend bounded response publication');
    for (const forbidden of ['page-routes', 'response-body', 'request-history'])
      assert.equal(features.includes(forbidden), false,
        `response-finished must not select ${forbidden}`);
  }
}
function checkResponseRequest(lines, enabled, networkEventsEnabled = enabled) {
  const addon = lines.find(value => value.startsWith('pwright-node-addon v'));
  if (addon) {
    const features = addon.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('response-request'), enabled,
      'addon response-request requires explicit selection');
    if (enabled) assert.equal(features.includes('page-routes'), false,
      'addon response-request must not select Page routing');
  }
  const host = lines.find(value => value.startsWith('pwright-node-host v'));
  assert.ok(host, 'response-request boundary requires the safe host');
  const features = host.split(' features:')[1].split(' ')[0].split(',');
  for (const feature of ['response-request', 'request-history'])
    assert.equal(features.includes(feature), enabled,
      `host ${feature} must follow response-request selection`);
  assert.equal(features.includes('network-events'), networkEventsEnabled,
    'host network-events must follow a selected network consumer');
  if (enabled) assert.equal(features.includes('page-routes'), false,
    'host response-request must not select Page routing');
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
function checkHistoryNavigation(lines, enabled) {
  for (const name of ['pwright-node-addon', 'pwright-node-host', 'pwright-backend-cdp',
    'pwright-backend-cdp-navigation', 'pwright-composition-chromium']) {
    const line = lines.find(value => value.startsWith(`${name} v`));
    assert.ok(line, `${name} must be present`);
    const features = line.split(' features:')[1].split(' ')[0].split(',');
    assert.equal(features.includes('history-navigation'), enabled,
      `${name}/history-navigation requires explicit selection`);
  }
  const packages = new Set(lines.map(line => line.split(' ')[0]));
  assert.equal(packages.has('pwright-cap-history-navigation'), enabled,
    'history capability crate requires explicit history-navigation selection');
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
// ElementHandle DOM operations reuse the existing Handle/query graph. Its
// retained bounding-box read deliberately selects the host's geometry leaf;
// no other transitive feature widening is allowed.
for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  const selectedTree = tree('element-handle-dom', target).trim().split('\n');
  const hostLine = selectedTree.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(hostLine?.split(' features:')[1]?.split(',').includes('locator-bounding-box'),
    `${target} ElementHandle DOM must select its explicit host geometry leaf`);
  const fingerprint = (feature, stripDom) => [...new Set(tree(feature, target).trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = !stripDom ? []
        : name === 'pwright-node-host' ? ['element-handle-dom', 'locator-bounding-box']
          : name === 'pwright-node-addon' ? ['element-handle-dom'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint('element-handle-dom', true), fingerprint('handles', false),
    `${target} element-handle-dom may add only its explicit host/addon feature flags`);
}
// Scoped selector waits add only their own host/addon method and the shared
// timeout-settings owner. No DOM-action or geometry leaf is implied.
for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  const fingerprint = (feature, stripWait) => [...new Set(tree(feature, target).trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripWait && ['pwright-node-host', 'pwright-node-addon'].includes(name)
        ? ['element-handle-wait-for-selector'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint('element-handle-wait-for-selector', true),
    fingerprint('handles,timeout-settings', false),
    `${target} scoped ElementHandle wait may add only its explicit host/addon feature flags`);
}
// Value-string selectOption reuses exact Handle evaluation and timeout settings.
// The new host/addon method must not pull in DOM geometry or Locator actions.
for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  const fingerprint = (feature, stripSelect) => [...new Set(tree(feature, target).trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripSelect && ['pwright-node-host', 'pwright-node-addon'].includes(name)
        ? ['element-handle-select-option-value'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint('element-handle-select-option-value', true),
    fingerprint('handles,timeout-settings', false),
    `${target} retained value selection may add only its explicit host/addon feature flags`);
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
for (const feature of ['', 'bypass-csp', 'device-emulation', 'touchscreen-input', 'element-handle-dom,touchscreen-input', 'selector-registration', 'request-events', 'response-events', 'response-finished', 'request-post-data-json', 'drag-source', 'drag-runtime', 'frame-navigation', 'drag-runtime,frame-navigation', 'extra-headers', 'extra-headers,drag-runtime,frame-navigation', 'concurrent-close', 'concurrent-close,extra-headers,drag-runtime,frame-navigation', 'file-input', 'file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'handles', 'element-handle-dom', 'handles,file-input', 'handles,file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'retained-file-input', 'retained-file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'retained-target-marking', 'retained-target-marking,concurrent-close,extra-headers,drag-runtime,frame-navigation', 'retained-target-prevalidation', 'retained-target-prevalidation,concurrent-close,extra-headers,drag-runtime,frame-navigation']) {
  const selected = effectiveSelection(feature);
  const lines = tree(feature).trim().split('\n');
  checkCloseCoordinator(lines, selected);
  checkNativeHandleRelease(lines, selected);
  checkRetainedReference(lines, selected.includes('retained-file-input'),
    selected.includes('retained-target-prevalidation'));
  checkTargetMarking(lines, selected.includes('retained-target-marking'));
  checkTargetPrevalidation(lines, selected.includes('retained-target-prevalidation'));
  checkResponseRequest(lines, selected.includes('response-request'),
    selected.includes('response-request') || selected.includes('request-events')
      || selected.includes('response-events') || selected.includes('request-post-data-json'));
  checkRequestEvents(lines, feature.split(',').includes('request-events'),
    selected.includes('request-post-data-json'), selected.includes('request-events')
      || selected.includes('response-events') || selected.includes('request-post-data-json'));
  checkResponseEvents(lines, selected.includes('response-events'),
    selected.includes('request-events') || selected.includes('response-events')
      || selected.includes('request-post-data-json'));
  checkResponseFinished(lines, selected.includes('response-finished'));
  const packages = new Set(lines.map(line => line.split(' ')[0]));
  for (const touchscreen of ['pwright-cap-touchscreen', 'pwright-backend-cdp-touchscreen', 'pwright-composition-chromium-touchscreen'])
    assert.equal(packages.has(touchscreen), selected.includes('touchscreen-input'), `${touchscreen} requires touchscreen-input selection`);
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
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('bypass-csp'), selected.includes('bypass-csp'),
    'CSP policy owner requires explicit bypass-csp selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('device-emulation'), selected.includes('device-emulation'),
    'device settings owner requires explicit device-emulation selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('touchscreen-input'), selected.includes('touchscreen-input'),
    'Touchscreen input owner requires explicit touchscreen-input selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('selector-registration'), selected.includes('selector-registration'),
    'selector registry requires explicit selector-registration selection');
  console.log(`Safe Rust host ${feature || 'default'} boundary verified (${packages.size} packages including host); lifetime-handoff=${handoff}.`);
}

const addonManifest = readFileSync(new URL('../../../crates/pwright-node-addon/Cargo.toml', import.meta.url), 'utf8');
const routeOverrideFeature = /^route-header-url-overrides\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(routeOverrideFeature, 'the addon Route override feature must be declared explicitly');
assert.deepEqual([...routeOverrideFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['page-routes', 'pwright-node-host/route-redirects'],
  'Route header/URL overrides must select only Page routing and the Rust redirect owner');
for (const [selection, expected] of [
  ['', false],
  ['page-routes', false],
  ['route-header-url-overrides', true],
]) {
  const lines = tree(selection, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(addon && host, 'Route override boundary requires the addon and safe host');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const hostFeatures = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(addonFeatures.includes('route-header-url-overrides'), expected,
    'Route override conversion must require its explicit addon selection');
  assert.equal(hostFeatures.includes('route-redirects'), expected,
    'Route redirect header retention must require the same opt-in selection');
  assert.equal(hostFeatures.includes('request-history'), expected,
    'Route redirects must compose bounded Rust request history');
  assert.equal(hostFeatures.includes('page-routes'), selection !== '',
    'Route override conversion must not alter the default Page routing boundary');
}
const pageUnrouteFeature = /^page-unroute\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(pageUnrouteFeature, 'the addon Page.unroute feature must be declared explicitly');
assert.deepEqual([...pageUnrouteFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['page-routes'],
  'Page.unroute must compose only the already-existing Rust Page routing owner');
for (const [selection, expected] of [
  ['', false],
  ['page-routes', false],
  ['route-header-url-overrides', false],
  ['page-unroute', true],
]) {
  const lines = tree(selection, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(addon && host, 'Page.unroute boundary requires the addon and safe host');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const hostFeatures = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(addonFeatures.includes('page-unroute'), expected,
    'Page.unroute conversion must require its distinct opt-in selection');
  assert.equal(hostFeatures.includes('page-routes'), selection !== '',
    'Page.unroute must not broaden the default Rust host boundary');
  assert.equal(hostFeatures.includes('route-redirects'), selection === 'route-header-url-overrides',
    'Page.unroute alone must not select route redirect support');
}
const routeRequestHistoryFeature = /^route-request-history\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(routeRequestHistoryFeature, 'the addon route Request history feature must be explicit');
assert.deepEqual([...routeRequestHistoryFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['page-routes', 'request-history', 'pwright-node-host/route-request-history'],
  'route Request history must compose the existing Rust route and bounded request-history owners');
for (const [selection, expected] of [
  ['', false],
  ['page-routes', false],
  ['request-history', false],
  ['route-request-history', true],
]) {
  const lines = tree(selection, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(addon && host, 'route Request history boundary requires the addon and safe host');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const hostFeatures = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(addonFeatures.includes('route-request-history'), expected);
  assert.equal(hostFeatures.includes('route-request-history'), expected);
  assert.equal(hostFeatures.includes('page-routes'), selection === 'page-routes' || expected);
  assert.equal(hostFeatures.includes('request-history'), selection === 'request-history' || expected);
}
const contextCookiesFeature = /^context-cookies\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(contextCookiesFeature, 'the addon Context cookie feature must be declared explicitly');
assert.deepEqual([...contextCookiesFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['pwright-node-host/context-cookies'],
  'Context cookies must select only the existing Rust host cookie owner');
for (const [selection, addonExpected, hostExpected] of [
  ['', false, false],
  ['context-cookies', true, true],
  ['context-api-request', false, true],
]) {
  const lines = tree(selection, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(addon && host, 'Context cookie boundary requires the addon and safe host');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const hostFeatures = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(addonFeatures.includes('context-cookies'), addonExpected);
  assert.equal(hostFeatures.includes('context-cookies'), hostExpected);
}
const largeEvaluationInputsFeature = /^large-evaluation-inputs\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(largeEvaluationInputsFeature, 'the addon large evaluation input leaf must be explicit');
assert.deepEqual([...largeEvaluationInputsFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['pwright-node-host/large-evaluation-inputs']);
const largeRoutePostDataFeature = /^large-route-post-data\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(largeRoutePostDataFeature, 'the addon large Route POST leaf must be explicit');
assert.deepEqual([...largeRoutePostDataFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['page-routes', 'pwright-node-host/large-route-post-data']);
for (const [selection, expectedEvaluation, expectedRoute] of [
  ['', false, false],
  ['page-routes', false, false],
  ['large-evaluation-inputs', true, false],
  ['large-route-post-data', false, true],
  ['large-evaluation-inputs,large-route-post-data', true, true],
  ['frame-navigation,large-evaluation-inputs,large-route-post-data', true, true],
]) {
  const lines = tree(selection, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(addon && host, 'large Route input boundary requires addon and Rust host');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const hostFeatures = host.split(' features:')[1].split(' ')[0].split(',');
  assert.equal(addonFeatures.includes('large-evaluation-inputs'), expectedEvaluation);
  assert.equal(hostFeatures.includes('large-evaluation-inputs'), expectedEvaluation);
  assert.equal(addonFeatures.includes('large-route-post-data'), expectedRoute);
  assert.equal(hostFeatures.includes('large-route-post-data'), expectedRoute);
  assert.equal(hostFeatures.includes('page-routes'), selection.includes('page-routes') || expectedRoute);
  assert.equal(hostFeatures.includes('frame-navigation'), selection.includes('frame-navigation'));
  assert.equal(hostFeatures.includes('network-events'), false,
    'standalone large Route input proof must not select the unrelated network-events body cap');
}
const responseRequestAddonFeature = /^response-request\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(responseRequestAddonFeature, 'the addon response-request feature must be declared explicitly');
assert.deepEqual([...responseRequestAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['frame-navigation', 'request-history', 'pwright-node-host/response-request'],
  'addon response-request must select navigation and the Rust-owned request history');
const requestEventsAddonFeature = /^request-events\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(requestEventsAddonFeature, 'the addon request-events feature must be declared explicitly');
assert.deepEqual([...requestEventsAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-node-host/request-events', 'url-glob'].sort(),
  'addon request-events must select only URL matching and the Rust notice owner');
const responseEventsAddonFeature = /^response-events\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(responseEventsAddonFeature, 'the addon response-events feature must be declared explicitly');
assert.deepEqual([...responseEventsAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-node-host/response-events', 'url-glob'].sort(),
  'addon response-events must select only URL matching and the Rust notice owner');
const responseFinishedAddonFeature = /^response-finished\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(responseFinishedAddonFeature, 'the addon response-finished feature must be declared explicitly');
assert.deepEqual([...responseFinishedAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['pwright-node-host/response-finished', 'response-events'].sort(),
  'addon response-finished must add only terminal ownership to response publication');
const browserRegistryAddonFeature = /^browser-registry\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(browserRegistryAddonFeature, 'the addon browser-registry feature must be declared explicitly');
assert.deepEqual([...browserRegistryAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['dep:pwright-browser-registry'],
  'addon browser-registry must select only the pure Rust registry leaf');
const persistentContextPreflightFeature = /^persistent-context-preflight\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(persistentContextPreflightFeature, 'the addon persistent-context-preflight feature must be declared explicitly');
assert.deepEqual([...persistentContextPreflightFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['dep:pwright-persistent-context-preflight'],
  'addon persistent-context-preflight must select only the pure Rust decision leaf');
const requestPostDataJsonAddonFeature = /^request-post-data-json\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(requestPostDataJsonAddonFeature, 'the addon request-post-data-json feature must be declared explicitly');
assert.deepEqual([...requestPostDataJsonAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['dep:serde_json', 'napi_runtime/serde-json', 'pwright-node-host/request-post-data-json',
    'request-events'].sort(),
  'addon request-post-data-json must add only Rust parsing and JSON conversion');
const bypassCspAddonFeature = /^bypass-csp\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(bypassCspAddonFeature, 'the addon bypass-csp feature must be declared explicitly');
assert.deepEqual([...bypassCspAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['pwright-node-host/bypass-csp'],
  'addon bypass-csp must select only the Rust host policy owner');
const deviceEmulationAddonFeature = /^device-emulation\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(deviceEmulationAddonFeature, 'the addon device-emulation feature must be declared explicitly');
assert.deepEqual([...deviceEmulationAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['pwright-node-host/device-emulation'],
  'addon device-emulation must select only the Rust host device owner');
const touchscreenInputAddonFeature = /^touchscreen-input\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(touchscreenInputAddonFeature, 'the addon touchscreen-input feature must be declared explicitly');
assert.deepEqual([...touchscreenInputAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['device-emulation', 'pwright-node-host/touchscreen-input'],
  'addon touchscreen-input must select touch Context creation and the Rust host Touchscreen owner');
const pageViewportAddonFeature = /^page-viewport\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(pageViewportAddonFeature, 'the addon page-viewport feature must be declared explicitly');
assert.deepEqual([...pageViewportAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['device-emulation', 'pwright-node-host/page-viewport'].sort(),
  'addon page-viewport must declare its device-emulation prerequisite');
const contextApiRequestAddonFeature = /^context-api-request\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(contextApiRequestAddonFeature, 'the addon context-api-request feature must be declared explicitly');
assert.deepEqual([...contextApiRequestAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['api-request', 'dep:async-trait', 'dep:url', 'device-emulation',
    'pwright-node-host/context-api-request'].sort(),
  'addon context-api-request must declare its API and device-emulation prerequisites');
const selectorRegistrationAddonFeature = /^selector-registration\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(selectorRegistrationAddonFeature, 'the addon selector-registration feature must be declared explicitly');
assert.deepEqual([...selectorRegistrationAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['pwright-node-host/selector-registration'],
  'addon selector-registration must select only the Rust host registry owner');
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
const bidiStartupFailureAddonFeature = /^bidi-startup-failure\s*=\s*\[([^\]]*)\]/m.exec(addonManifest);
assert.ok(bidiStartupFailureAddonFeature, 'the addon BiDi startup-failure feature must be declared explicitly');
assert.deepEqual([...bidiStartupFailureAddonFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['playwright-trace', 'dep:pwright-owned-process-startup'].sort(),
  'BiDi startup failure must select only the existing trace profile and neutral process-startup leaf');
const ownedProcessStartupManifest = readFileSync(new URL('../../../crates/pwright-owned-process-startup/Cargo.toml', import.meta.url), 'utf8');
let inStartupDependencySection = false;
const ownedProcessStartupDependencies = [];
for (const line of ownedProcessStartupManifest.split('\n')) {
  if (line.startsWith('['))
    inStartupDependencySection = /^\[(?:dependencies|target\..+\.dependencies)\]$/.test(line);
  else if (inStartupDependencySection) {
    const declaration = /^([A-Za-z0-9_-]+)\s*=/.exec(line);
    if (declaration) ownedProcessStartupDependencies.push(declaration[1]);
  }
}
assert.deepEqual(ownedProcessStartupDependencies.sort(), ['libc', 'tokio'],
  'the owned-process-startup leaf must depend only on process/runtime primitives, never browser engines or protocols');
const hostManifest = readFileSync(new URL('../../../crates/pwright-node-host/Cargo.toml', import.meta.url), 'utf8');
const responseRequestHostFeature = /^response-request\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(responseRequestHostFeature, 'the host response-request feature must be declared explicitly');
assert.deepEqual([...responseRequestHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['frame-navigation', 'request-history'],
  'host response-request must select only navigation and bounded request history');
const requestEventsHostFeature = /^request-events\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(requestEventsHostFeature, 'the host request-events feature must be declared explicitly');
assert.deepEqual([...requestEventsHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['network-events'],
  'host request-events must select only the bounded network lifecycle');
const responseEventsHostFeature = /^response-events\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(responseEventsHostFeature, 'the host response-events feature must be declared explicitly');
assert.deepEqual([...responseEventsHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['network-events'],
  'host response-events must select only the bounded network lifecycle');
const responseFinishedHostFeature = /^response-finished\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(responseFinishedHostFeature, 'the host response-finished feature must be declared explicitly');
assert.deepEqual([...responseFinishedHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['response-events'],
  'host response-finished must extend only bounded response publication');
const requestPostDataJsonHostFeature = /^request-post-data-json\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(requestPostDataJsonHostFeature, 'the host request-post-data-json feature must be declared explicitly');
assert.deepEqual([...requestPostDataJsonHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]),
  ['network-events'],
  'host request-post-data-json must select only the bounded network lifecycle');
const urlGlobHostFeature = /^url-glob\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(urlGlobHostFeature, 'the host URL-glob feature must be declared explicitly');
assert.deepEqual([...urlGlobHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]), [],
  'host URL-glob compilation must remain metadata-only');
const touchscreenInputHostFeature = /^touchscreen-input\s*=\s*\[([^\]]*)\]/m.exec(hostManifest);
assert.ok(touchscreenInputHostFeature, 'the host touchscreen-input feature must be declared explicitly');
assert.deepEqual([...touchscreenInputHostFeature[1].matchAll(/"([^"]+)"/g)].map(match => match[1]).sort(),
  ['device-emulation', 'dep:pwright-backend-cdp-input', 'dep:pwright-backend-cdp-touchscreen',
    'dep:pwright-cap-touchscreen', 'dep:pwright-composition-chromium-touchscreen',
    'pwright-backend-cdp-locator/touchscreen-input',
    'pwright-composition-chromium/touchscreen-input'].sort(),
  'host touchscreen-input must select Context touch state plus its Rust input, capability, backend and composition leaves');
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
for (const feature of ['', 'bypass-csp', 'device-emulation', 'touchscreen-input', 'element-handle-dom,touchscreen-input', 'selector-registration', 'request-events', 'response-events', 'response-finished', 'request-post-data-json', 'handles', 'drag-runtime', 'frame-navigation', 'history-navigation', 'frame-navigation,history-navigation', 'drag-runtime,frame-navigation',
  'extra-headers', 'drag-runtime,extra-headers', 'frame-navigation,extra-headers', 'drag-runtime,frame-navigation,extra-headers', 'concurrent-close', 'concurrent-close,drag-runtime,frame-navigation,extra-headers', 'file-input', 'file-input,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'handles,file-input', 'handles,file-input,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-file-input', 'retained-file-input,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-target-marking', 'retained-target-marking,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-target-prevalidation', 'retained-target-prevalidation,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-input-instrumentation', 'retained-input-instrumentation,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-call-argument-serialization', 'retained-call-argument-serialization,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-serialized-payload-import', 'retained-serialized-payload-import,concurrent-close,drag-runtime,frame-navigation,extra-headers', 'retained-serialized-payload-mutation', 'retained-serialized-payload-mutation,concurrent-close,drag-runtime,frame-navigation,extra-headers']) {
  const selected = effectiveSelection(feature);
  const lines = tree(feature, 'pwright-node-addon').trim().split('\n');
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(host, 'addon must use the safe host');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('handles'), selected.includes('handles'),
    'Node Handle ownership requires explicit handles selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('bypass-csp'), selected.includes('bypass-csp'),
    'Node CSP policy requires explicit bypass-csp selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('device-emulation'), selected.includes('device-emulation'),
    'Node device settings require explicit device-emulation selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('touchscreen-input'), selected.includes('touchscreen-input'),
    'Node Touchscreen input requires explicit touchscreen-input selection');
  assert.equal(host.split(' features:')[1].split(' ')[0].split(',').includes('selector-registration'), selected.includes('selector-registration'),
    'Node selector registry requires explicit selector-registration selection');
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
  checkHistoryNavigation(lines, selected.includes('history-navigation'));
  checkResponseRequest(lines, selected.includes('response-request'),
    selected.includes('response-request') || selected.includes('request-events')
      || selected.includes('response-events') || selected.includes('request-post-data-json'));
  checkRequestEvents(lines, selected.includes('request-events'),
    selected.includes('request-post-data-json'), selected.includes('request-events')
      || selected.includes('response-events') || selected.includes('request-post-data-json'));
  checkResponseEvents(lines, selected.includes('response-events'),
    selected.includes('request-events') || selected.includes('response-events')
      || selected.includes('request-post-data-json'));
  checkResponseFinished(lines, selected.includes('response-finished'));
  const packages = new Set(lines.map(line => line.split(' ')[0]));
  for (const touchscreen of ['pwright-cap-touchscreen', 'pwright-backend-cdp-touchscreen', 'pwright-composition-chromium-touchscreen'])
    assert.equal(packages.has(touchscreen), selected.includes('touchscreen-input'), `${touchscreen} requires addon touchscreen-input selection`);
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

for (const feature of ['', 'owned-launch', 'browser-registry']) {
  const lines = tree(feature, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  assert.ok(addon, 'browser registry boundary requires the Node addon');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const selected = feature === 'browser-registry';
  assert.equal(addonFeatures.includes('browser-registry'), selected,
    'addon browser-registry requires explicit selection');
  assert.equal(lines.some(line => line.startsWith('pwright-browser-registry v')), selected,
    'pure Rust registry leaf requires explicit browser-registry selection');
  assert.equal(addonFeatures.includes('owned-launch'), feature === 'owned-launch',
    'browser-registry and owned-launch must remain independent');
  console.log(`Node addon ${feature || 'default'} browser-registry boundary verified.`);
}
const withoutRegistryLeaf = feature => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
  .map(line => line.replace(/ \(\*\)$/, ''))
  .filter(line => !line.startsWith('pwright-browser-registry v'))
  .map(line => {
    if (!line.startsWith('pwright-node-addon v')) return line;
    const [pkg, selected] = line.split(' features:');
    return `${pkg} features:${selected.split(',').filter(value => value !== 'browser-registry').sort().join(',')}`;
  }))].sort();
assert.deepEqual(withoutRegistryLeaf('browser-registry'), withoutRegistryLeaf(''),
  'browser-registry may add only its addon marker and dependency-free Rust leaf, not owned launch or new browser providers/installers');

for (const feature of ['', 'owned-launch', 'playwright-trace', 'bidi-startup-failure']) {
  const lines = tree(feature, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  const host = lines.find(line => line.startsWith('pwright-node-host v'));
  assert.ok(addon && host, 'Playwright trace boundary requires the Node addon and host');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const hostFeatures = host.split(' features:')[1].split(' ')[0].split(',');
  const bidiStartupSelected = feature === 'bidi-startup-failure';
  const traceSelected = feature === 'playwright-trace' || bidiStartupSelected;
  assert.equal(addonFeatures.includes('playwright-trace'), traceSelected,
    'addon trace capability requires explicit selection');
  assert.equal(hostFeatures.includes('playwright-trace'), traceSelected,
    'host trace capability requires explicit selection');
  assert.equal(lines.some(line => line.startsWith('pwright-trace-playwright-v8 v')), traceSelected,
    'pure Rust Playwright trace leaf requires explicit selection');
  assert.equal(addonFeatures.includes('owned-launch'), feature !== '',
    'Playwright trace capability composes owned launch without changing its original profile');
  assert.equal(addonFeatures.includes('bidi-startup-failure'), bidiStartupSelected,
    'addon BiDi startup failure requires its own explicit selection');
  assert.equal(lines.some(line => line.startsWith('pwright-owned-process-startup v')), bidiStartupSelected,
    'neutral process-startup leaf requires BiDi startup-failure selection');
  console.log(`Node addon ${feature || 'default'} Playwright trace boundary verified.`);
}
const withoutBidiStartupFailureLeaf = (feature, stripSelected) => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
  .map(line => line.replace(/ \(\*\)$/, ''))
  .filter(line => !stripSelected || !line.startsWith('pwright-owned-process-startup v'))
  .map(line => {
    if (!stripSelected || !line.startsWith('pwright-node-addon v')) return line;
    const [pkg, selected] = line.split(' features:');
    return `${pkg} features:${selected.split(',').filter(value => value !== 'bidi-startup-failure').sort().join(',')}`;
  }))].sort();
assert.deepEqual(withoutBidiStartupFailureLeaf('bidi-startup-failure', true),
  withoutBidiStartupFailureLeaf('playwright-trace', false),
  'BiDi startup failure may add only its addon marker and neutral process-startup leaf, not browser engine or protocol activation');

for (const feature of ['', 'owned-launch', 'browser-registry', 'persistent-context-preflight']) {
  const lines = tree(feature, 'pwright-node-addon').trim().split('\n');
  const addon = lines.find(line => line.startsWith('pwright-node-addon v'));
  assert.ok(addon, 'persistent-Context preflight boundary requires the Node addon');
  const addonFeatures = addon.split(' features:')[1].split(' ')[0].split(',');
  const selected = feature === 'persistent-context-preflight';
  assert.equal(addonFeatures.includes('persistent-context-preflight'), selected,
    'addon persistent-context-preflight requires explicit selection');
  assert.equal(lines.some(line => line.startsWith('pwright-persistent-context-preflight v')), selected,
    'pure Rust preflight leaf requires explicit selection');
  assert.equal(addonFeatures.includes('owned-launch'), feature === 'owned-launch',
    'persistent-Context preflight and owned launch must remain independent');
  console.log(`Node addon ${feature || 'default'} persistent-Context preflight boundary verified.`);
}
const withoutPreflightLeaf = feature => [...new Set(tree(feature, 'pwright-node-addon').trim().split('\n')
  .map(line => line.replace(/ \(\*\)$/, ''))
  .filter(line => !line.startsWith('pwright-persistent-context-preflight v'))
  .map(line => {
    if (!line.startsWith('pwright-node-addon v')) return line;
    const [pkg, selected] = line.split(' features:');
    return `${pkg} features:${selected.split(',').filter(value => value !== 'persistent-context-preflight').sort().join(',')}`;
  }))].sort();
assert.deepEqual(withoutPreflightLeaf('persistent-context-preflight'), withoutPreflightLeaf(''),
  'persistent-Context preflight may add only its addon marker and dependency-free Rust leaf');

for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  for (const feature of ['response-request', 'response-request,extra-headers']) {
    const lines = tree(feature, target).trim().split('\n');
    checkResponseRequest(lines, true);
    const navigation = lines.find(line => line.startsWith('pwright-cap-navigation v'));
    assert.ok(navigation, `${target} response-request requires the navigation capability`);
    assert.equal(navigation.split(' features:')[1].split(' ')[0].split(',').includes('frame-navigation'), true,
      `${target} response-request must compose frame-navigation`);
    console.log(`${target} ${feature} response-request closure verified.`);
  }
}

for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  const fingerprint = (feature, stripResponseRequest) => [...new Set(tree(feature, target).trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = !stripResponseRequest ? []
        : name === 'pwright-node-addon' ? ['request-history', 'response-request']
          : name === 'pwright-node-host'
            ? ['network-events', 'request-history', 'response-request'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint('response-request', true), fingerprint('frame-navigation', false),
    `${target} response-request may add only its addon marker and Rust request-history owner flags`);
}

for (const target of ['pwright-node-host', 'pwright-node-addon']) {
  const fingerprint = (feature, stripFinished) => [...new Set(tree(feature, target).trim().split('\n')
    .map(line => line.replace(/ \(\*\)$/, ''))
    .map(line => {
      const [pkg, selected] = line.split(' features:');
      const name = pkg.split(' ')[0];
      const removed = stripFinished && ['pwright-node-addon', 'pwright-node-host'].includes(name)
        ? ['response-finished'] : [];
      return `${pkg} features:${selected.split(',').filter(value => !removed.includes(value)).sort().join(',')}`;
    }))].sort();
  assert.deepEqual(fingerprint('response-finished', true), fingerprint('response-events', false),
    `${target} response-finished may add only its addon and host terminal-owner flags`);
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
