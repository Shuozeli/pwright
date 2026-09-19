// Pinned upstream policy/ordering oracle, followed by separately labelled
// backend-script descriptor comparisons. No candidate addon, browser, native
// Handle, DOM, filesystem upload access or upload success exists in this file.
// Actual ElementHandle prototype functions supply projection/precheck/marking
// code. Explicit local adapters intercept evaluation: captured precheck code
// may run against plain descriptors, never a simulated browser DOM. Successful
// prechecks throw STOP at the late utility boundary before invoking browser
// injectedScript.setInputFiles. The actual utility argument serializer is
// exercised only by separately labelled late-serialization adapters.
// Sources: Playwright 1.58.2 lib/server/dom.js:428–434,544–584;
// server/javascript.js:183–219; utils/isomorphic/utilityScriptSerializers.js:147–244.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const load = path => requireCore(join(core, 'lib', path));
const { ElementHandle } = load('server/dom.js');
const { serializeAsCallArgument } = load('utils/isomorphic/utilityScriptSerializers.js');
const limits = { timeout: 5000 };

async function rejection(promise) {
  return promise.then(() => assert.fail('fixture must stop before upload success'), error => error);
}
function rawPayload() {
  return { name: 'one.bin', mimeType: 'application/octet-stream', buffer: 'AQ==' };
}
function progress(id = '') {
  return { metadata: { id }, race: value => Promise.resolve(value) };
}
function projection(items, trace = []) {
  const stop = new Error('STOP: first native utility evaluation boundary');
  const state = { flags: undefined, stop, trace, evaluations: 0 };
  const receiver = {
    evaluateHandleInUtility(callback, flags) {
      assert.equal(typeof callback, 'function');
      trace.push('evaluateHandleInUtility'); state.flags = flags; state.callback = callback; state.evaluations++;
      throw stop;
    },
    instrumentation: { onBeforeInputAction() { assert.fail('must not reach input instrumentation'); } },
  };
  return { state, promise: Reflect.apply(ElementHandle.prototype._setInputFiles, receiver, [progress(), items]) };
}

test('pinned DOM projection: all three carrier getters run before multiplicity and native evaluation', limits, async () => {
  // Arrange
  const trace = [];
  const items = {
    get filePayloads() { trace.push('filePayloads'); return { get length() { trace.push('payloads:length'); return 0; } }; },
    get localPaths() { trace.push('localPaths'); return { get length() { trace.push('paths:length'); return 0; } }; },
    get localDirectory() { trace.push('localDirectory'); return ''; },
  };
  // Act
  const run = projection(items, trace);
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.deepEqual(trace, ['filePayloads', 'localPaths', 'localDirectory', 'payloads:length', 'paths:length', 'evaluateHandleInUtility']);
  assert.deepEqual(run.state.flags, { multiple: false, directoryUpload: false });
});

for (const [label, paths] of [['undefined', undefined], ['null', null], ['zero', 0], ['empty string', '']]) {
  test('pinned DOM projection: multiple preserves raw falsy ' + label + ' localPaths', limits, async () => {
    // Arrange / Act
    const run = projection({ filePayloads: [], localPaths: paths });
    const error = await rejection(run.promise);
    // Assert: no Boolean normalization is present in the original expression.
    assert.equal(error, run.state.stop);
    assert.equal(Object.is(run.state.flags.multiple, paths), true);
    assert.equal(run.state.flags.directoryUpload, false);
  });
}

test('pinned DOM projection: greater-than multiplicity uses number-hint coercion', limits, async () => {
  // Arrange
  const trace = [];
  const length = { [Symbol.toPrimitive](hint) { trace.push(hint); return '2'; } };
  // Act
  const run = projection({ filePayloads: { length } }, trace);
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.deepEqual(trace, ['number', 'evaluateHandleInUtility']);
  assert.equal(run.state.flags.multiple, true);
});

test('pinned DOM projection: payload multiplicity short-circuits path length and directory coercion', limits, async () => {
  // Arrange
  const directory = { [Symbol.toPrimitive]() { assert.fail('truthiness must not coerce directory object'); } };
  const items = { filePayloads: { length: 2 },
    localPaths: { get length() { assert.fail('true payload multiplicity skips paths.length'); } },
    localDirectory: directory };
  // Act
  const run = projection(items);
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.deepEqual(run.state.flags, { multiple: true, directoryUpload: true });
});

test('pinned DOM projection: length getter failure follows carrier destructuring and precedes native evaluation', limits, async () => {
  // Arrange
  const trace = [], sentinel = new Error('payload length getter');
  const items = {
    get filePayloads() { trace.push('payloads'); return { get length() { trace.push('length'); throw sentinel; } }; },
    get localPaths() { trace.push('paths'); return undefined; },
    get localDirectory() { trace.push('directory'); return undefined; },
  };
  // Act
  const run = projection(items, trace);
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, sentinel);
  assert.equal(run.state.evaluations, 0);
  assert.deepEqual(trace, ['payloads', 'paths', 'directory', 'length']);
});

test('pinned DOM projection: payload indices and metadata remain unread at native precheck entry', limits, async () => {
  // Arrange
  let reads = 0;
  const files = [];
  Object.defineProperty(files, 0, { get() { reads++; throw new Error('payload must remain deferred'); } });
  // Act
  const run = projection({ filePayloads: files });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.equal(reads, 0);
  assert.equal(run.state.evaluations, 1);
});

test('pinned DOM projection: null carrier rejects without evaluation', limits, async () => {
  // Arrange / Act
  const run = projection(null);
  const error = await rejection(run.promise);
  // Assert
  assert.ok(error instanceof TypeError);
  assert.equal(run.state.evaluations, 0);
});

function precheckFixture(items, config = {}) {
  const trace = [];
  const stop = new Error('STOP: late utility boundary, no browser call');
  const incomingNode = {};
  const element = config.element ?? { tagName: 'INPUT', multiple: false, webkitdirectory: false };
  const state = { trace, stop, serialized: undefined, flags: undefined, asElementCalls: 0,
    lateCalls: 0, createdErrors: [] };
  const injected = {
    retarget(node, behavior) {
      trace.push('retarget'); assert.equal(node, incomingNode); assert.equal(behavior, 'follow-label');
      return config.missing ? undefined : element;
    },
    createStacklessError(message) {
      trace.push('dom-error'); const error = new Error(message); state.createdErrors.push(error); return error;
    },
  };
  const retargetedAdapter = {
    evaluateInUtility(callback, files) {
      trace.push('late-utility'); state.lateCalls++;
      assert.equal(typeof callback, 'function');
      config.late?.(files, state);
      throw stop;
    },
  };
  const receiver = {
    evaluateHandleInUtility(callback, flags) {
      trace.push('precheck'); state.flags = flags;
      // Actual upstream callback, plain descriptor fixtures. This does not
      // execute browser retargeting, utility transport or native Handle lookup.
      const selected = callback([injected, incomingNode, flags]);
      return { asElement() {
        trace.push('asElement'); state.asElementCalls++;
        return selected ? retargetedAdapter : null;
      } };
    },
    instrumentation: { async onBeforeInputAction(actualReceiver, metadata) {
      assert.equal(actualReceiver, receiver); assert.equal(metadata.id, 'oracle-call');
      trace.push('before-input'); await config.before?.(state);
    } },
  };
  return { state, promise: Reflect.apply(ElementHandle.prototype._setInputFiles, receiver, [progress('oracle-call'), items]) };
}

for (const [label, element, multiple, directory, message] of [
  ['non-INPUT tag', { tagName: 'DIV' }, false, false, 'Node is not an HTMLInputElement'],
  ['non-multiple input', { tagName: 'INPUT', multiple: false, webkitdirectory: false }, true, false,
    'Non-multiple file input can only accept single file'],
  ['directory on ordinary input', { tagName: 'INPUT', multiple: true, webkitdirectory: false }, false, true,
    'File input does not support directories, pass individual files instead'],
  ['files on directory input', { tagName: 'INPUT', multiple: true, webkitdirectory: true }, false, false,
    '[webkitdirectory] input requires passing a path to a directory'],
]) {
  test('pinned precheck callback policy: ' + label + ' wins before payload reads', limits, async () => {
    // Arrange
    let reads = 0;
    const files = [];
    files.length = multiple ? 2 : 1;
    Object.defineProperty(files, 0, { get() { reads++; throw new Error('late payload getter'); } });
    // Act
    const run = precheckFixture({ filePayloads: files, localDirectory: directory ? '/never-accessed' : undefined }, { element });
    const error = await rejection(run.promise);
    // Assert
    assert.equal(error, run.state.createdErrors[0]);
    assert.equal(error.message, message);
    assert.equal(reads, 0);
    assert.equal(run.state.asElementCalls, 0);
    assert.equal(run.state.lateCalls, 0);
    assert.deepEqual(run.state.trace, ['precheck', 'retarget', 'dom-error']);
  });
}

test('pinned precheck callback policy: missing retarget returns disconnected without later phases', limits, async () => {
  // Arrange
  const files = [rawPayload()];
  // Act
  const run = precheckFixture({ filePayloads: files }, { missing: true });
  const result = await run.promise;
  // Assert: this is an error sentinel, never a successful upload.
  assert.equal(result, 'error:notconnected');
  assert.equal(run.state.asElementCalls, 1);
  assert.equal(run.state.lateCalls, 0);
  assert.deepEqual(run.state.trace, ['precheck', 'retarget', 'asElement']);
});

test('pinned precheck callback policy: type=file is not checked here and asElement is read twice', limits, async () => {
  // Arrange
  const element = { tagName: 'INPUT', multiple: false, webkitdirectory: false,
    get type() { assert.fail('early callback does not inspect input.type'); },
    getAttribute() { assert.fail('input type is checked only by later browser injected code'); } };
  // Act
  const run = precheckFixture({ filePayloads: [rawPayload()] }, { element });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.equal(run.state.asElementCalls, 2);
  assert.deepEqual(run.state.trace, ['precheck', 'retarget', 'asElement', 'asElement', 'before-input', 'late-utility']);
});

test('pinned precheck boundary: input instrumentation rejection wins before late payload serialization', limits, async () => {
  // Arrange
  let reads = 0;
  const sentinel = new Error('before input action');
  const files = [];
  Object.defineProperty(files, 0, { get() { reads++; throw new Error('late array getter'); } });
  // Act
  const run = precheckFixture({ filePayloads: files }, { before() { throw sentinel; } });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, sentinel);
  assert.equal(reads, 0);
  assert.equal(run.state.lateCalls, 0);
});

test('pinned late serializer adapter: instrumentation microtask mutation is visible to payload getters', limits, async () => {
  // Arrange
  const trace = [];
  let name = 'before.bin';
  const files = [{ ...rawPayload(), get name() { trace.push('name:' + name); return name; } }];
  // Act
  const run = precheckFixture({ filePayloads: files }, {
    before() { queueMicrotask(() => { name = 'after.bin'; trace.push('microtask'); }); },
    late(value, state) { state.serialized = serializeAsCallArgument(value, item => ({ fallThrough: item })); },
  });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.deepEqual(trace, ['microtask', 'name:after.bin']);
  assert.equal(run.state.serialized.a[0].o.find(entry => entry.k === 'name').v, 'after.bin');
});

test('pinned late serializer adapter: throwing object fields are omitted rather than eagerly rejected', limits, async () => {
  // Arrange
  let reads = 0;
  const files = [{ ...rawPayload(), get name() { reads++; throw new Error('object name getter'); } }];
  // Act
  const run = precheckFixture({ filePayloads: files }, {
    late(value, state) { state.serialized = serializeAsCallArgument(value, item => ({ fallThrough: item })); },
  });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.state.stop);
  assert.equal(reads, 1);
  assert.equal(run.state.serialized.a[0].o.some(entry => entry.k === 'name'), false);
  assert.equal(run.state.serialized.a[0].o.some(entry => entry.k === 'buffer'), true);
  assert.equal(run.state.trace.indexOf('before-input') < run.state.trace.indexOf('late-utility'), true);
});

test('pinned late serializer adapter: throwing array index propagates only after input instrumentation', limits, async () => {
  // Arrange
  const sentinel = new Error('late array index');
  let reads = 0;
  const files = [];
  Object.defineProperty(files, 0, { get() { reads++; throw sentinel; } });
  // Act
  const run = precheckFixture({ filePayloads: files }, {
    late(value) { serializeAsCallArgument(value, item => ({ fallThrough: item })); },
  });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, sentinel);
  assert.equal(reads, 1);
  assert.deepEqual(run.state.trace, ['precheck', 'retarget', 'asElement', 'asElement', 'before-input', 'late-utility']);
});

function markingFixture({ id = 'mark-call', serializerError, markError } = {}) {
  const trace = [];
  const stop = new Error('STOP: native precheck after marking');
  const bytes = Buffer.from([1]);
  bytes.toString = function (encoding) {
    trace.push('serialize:' + encoding);
    if (serializerError) throw serializerError;
    return 'AQ==';
  };
  const node = { nodeType: 1 };
  const receiver = {
    _frame: { _page: { browserContext: { _browser: { _isCollocatedWithServer: false } } } },
    _markAsTargetElement: ElementHandle.prototype._markAsTargetElement,
    _setInputFiles: ElementHandle.prototype._setInputFiles,
    evaluateInUtility(callback, callId) {
      trace.push('mark-evaluation');
      if (markError) throw markError;
      callback([{ markTargetElements(nodes, actualId) {
        trace.push('mark-target'); assert.deepEqual([...nodes], [node]); assert.equal(actualId, id);
      } }, node, callId]);
    },
    evaluateHandleInUtility() { trace.push('native-precheck'); throw stop; },
  };
  const params = { payloads: [{ name: 'one.bin', mimeType: 'application/octet-stream', buffer: bytes }] };
  return { trace, stop, promise: Reflect.apply(ElementHandle.prototype.setInputFiles, receiver, [progress(id), params]) };
}

test('pinned target marking: actual materialization precedes actual marking and then precheck', limits, async () => {
  // Arrange / Act
  const run = markingFixture();
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.stop);
  assert.deepEqual(run.trace, ['serialize:base64', 'mark-evaluation', 'mark-target', 'native-precheck']);
});

test('pinned target marking: empty call metadata skips marking but not subsequent precheck', limits, async () => {
  // Arrange / Act
  const run = markingFixture({ id: '' });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, run.stop);
  assert.deepEqual(run.trace, ['serialize:base64', 'native-precheck']);
});

test('pinned target marking: materialization failure precedes any marking or precheck', limits, async () => {
  // Arrange
  const sentinel = new Error('materialization failed');
  // Act
  const run = markingFixture({ serializerError: sentinel });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, sentinel);
  assert.deepEqual(run.trace, ['serialize:base64']);
});

test('pinned target marking: marking failure prevents DOM precheck', limits, async () => {
  // Arrange
  const sentinel = new Error('target marking failed');
  // Act
  const run = markingFixture({ markError: sentinel });
  const error = await rejection(run.promise);
  // Assert
  assert.equal(error, sentinel);
  assert.deepEqual(run.trace, ['serialize:base64', 'mark-evaluation']);
});

// These are source-policy comparisons, NOT DOM or browser tests. The oracle
// combines the actual captured dom.js callback with the actual pinned generated
// retarget method. Both implementations receive independently created plain
// descriptors. matches/closest return fixture-selected values, not DOM queries.
// Only ordinary boolean flags cross this boundary: caller-side raw getters and
// multiplicity coercion belong to the separately tested projection above.
const { source: injectedSource } = load('generated/injectedScriptSource.js');
const retargetStart = injectedSource.indexOf('retarget(node, behavior) {');
const retargetEnd = injectedSource.indexOf('  async checkElementStates(', retargetStart);
assert.ok(retargetStart >= 0 && retargetEnd > retargetStart, 'pinned retarget source markers must exist');
const pinnedRetarget = runInNewContext(
  '({' + injectedSource.slice(retargetStart, retargetEnd) + '}).retarget',
  { Node: Object.freeze({ ELEMENT_NODE: 1 }) }, { timeout: 1000 },
);
const backendPrecheck = runInNewContext(
  '(' + readFileSync(new URL('../../../crates/pwright-backend-cdp-file-input/src/retained_preflight.js', import.meta.url), 'utf8') + ')',
  { Node: Object.freeze({ ELEMENT_NODE: 1 }) }, { timeout: 1000 },
);
const policyMessages = [undefined,
  'Node is not an HTMLInputElement',
  'Non-multiple file input can only accept single file',
  'File input does not support directories, pass individual files instead',
  '[webkitdirectory] input requires passing a path to a directory',
];

function descriptorFixture(config) {
  const trace = [], sentinel = new Error('descriptor getter failure');
  const forbidden = new Set(['type', 'getAttribute', 'disabled', 'isConnected', 'getBoundingClientRect', 'getClientRects', 'offsetWidth']);
  function element(label, values = {}, match = true, nearest = {}) {
    const raw = { nodeType: 1, tagName: 'INPUT', multiple: false, webkitdirectory: false,
      isContentEditable: false, ...values,
      matches(selector) { trace.push(label + ':matches:' + selector); return match; },
      closest(selector) { trace.push(label + ':closest:' + selector); return nearest[selector] ?? null; },
    };
    return new Proxy(raw, { get(target, key) {
      trace.push(label + '.' + String(key));
      assert.equal(forbidden.has(key), false, 'early precheck must not inspect ' + String(key));
      if (key === config.throwKey) throw sentinel;
      return Reflect.get(target, key);
    } });
  }
  const selected = element('selected', config.values, config.match ?? true);
  let incoming = selected;
  if (config.missing || config.parent) {
    incoming = new Proxy({ nodeType: 3, parentElement: config.missing ? null : selected }, {
      get(target, key) { trace.push('text.' + String(key)); return Reflect.get(target, key); },
    });
  }
  if (config.label) {
    const label = { get control() { trace.push('label.control'); return selected; } };
    incoming = element('labelChild', { tagName: 'SPAN' }, false, { label });
  }
  return { trace, sentinel, incoming, selected };
}

async function descriptorComparison(config) {
  // Capture rather than reimplement the actual upstream DOM precheck callback.
  const captured = projection({ filePayloads: [] });
  await rejection(captured.promise);
  const flags = { multiple: config.multiple ?? false, directoryUpload: config.directory ?? false };
  const outcomes = [];
  for (const mode of ['pinned', 'backend-script']) {
    const fixture = descriptorFixture(config);
    let code;
    try {
      const result = mode === 'pinned'
        ? captured.state.callback([{
          retarget: pinnedRetarget,
          createStacklessError(message) {
            const error = new Error(message);
            error.policyCode = policyMessages.indexOf(message);
            assert.ok(error.policyCode > 0, 'unexpected upstream policy error');
            return error;
          },
        }, fixture.incoming, flags])
        : backendPrecheck(fixture.incoming, flags);
      // Internal numeric codes intentionally differ from upstream thrown
      // errors; normalize only here, not as a native error-shape parity claim.
      if (result === fixture.selected) code = 'selected-identity';
      else if (mode === 'pinned' && result === undefined) code = 0;
      else { assert.equal(typeof result, 'number'); code = result; }
    } catch (error) {
      if (error === fixture.sentinel) code = 'getter-identity';
      else if (mode === 'pinned' && error.policyCode > 0) code = error.policyCode;
      else throw error;
    }
    outcomes.push({ code, trace: fixture.trace });
  }
  return outcomes;
}

for (const [label, config, expected] of [
  ['missing parent has no policy reads', { missing: true }, 0],
  ['non-INPUT fails before multiplicity', { values: { tagName: 'DIV' }, multiple: true, directory: true }, 1],
  ['multiple failure wins over directory failure', { multiple: true, directory: true }, 2],
  ['ordinary input rejects directory', { directory: true }, 3],
  ['directory input rejects files', { values: { webkitdirectory: true } }, 4],
  ['ordinary input returns exact node without type, disabled, connected or layout reads', {}, 'selected-identity'],
  ['multiple input short-circuits the first directory check', { multiple: true, values: { multiple: true } }, 'selected-identity'],
  ['directory input accepts multiple selection', { multiple: true, directory: true, values: { webkitdirectory: true } }, 'selected-identity'],
  ['text descriptor retargets to exact parent', { parent: true }, 'selected-identity'],
  ['label descriptor retargets to exact control', { label: true }, 'selected-identity'],
  ['unexpected node getter error preserves identity', { throwKey: 'webkitdirectory', directory: true }, 'getter-identity'],
]) {
  test('descriptor-only script differential: ' + label, limits, async () => {
    // Arrange: each side constructs its own descriptors; no DOM exists here.
    const scenario = { ...config };
    // Act
    const [oracle, candidate] = await descriptorComparison(scenario);
    // Assert: compare policy result/identity and exact node-side getter order.
    assert.equal(oracle.code, expected);
    assert.deepEqual(candidate, oracle);
    if (config.label) assert.equal(oracle.trace.filter(entry => entry === 'label.control').length, 2);
  });
}
