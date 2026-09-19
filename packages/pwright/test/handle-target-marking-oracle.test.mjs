// Pinned Playwright 1.58.2 descriptor/language oracle, NOT DOM/browser evidence.
// The actual server _markAsTargetElement invokes its actual extracted injected
// method in a VM; the candidate uses the two checked-in backend scripts. Plain
// node descriptors and a recording CustomEvent stand-in expose ordering only.
// No native owner, CDP, preparation ticket, upload or real event propagation is
// created. Real-DOM resource/marking gates remain a separate integration suite.
// Sources under pinned playwright-core/lib: server/dom.js:428–434;
// generated/injectedScriptSource.js exported source:7680–7705.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { createContext, runInContext } from 'node:vm';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const core = dirname(requireCore.resolve('playwright-core/package.json'));
const load = path => requireCore(join(core, 'lib', path));
const { ElementHandle } = load('server/dom.js');
const { source } = load('generated/injectedScriptSource.js');
const start = source.indexOf('  markTargetElements(markedElements, callId) {');
const end = source.indexOf('  _setupGlobalListenersRemovalDetection()', start);
assert.ok(start >= 0 && end > start, 'pinned marking source boundaries must exist');
const pinnedMethod = source.slice(start, end);
const createSource = readFileSync(new URL('../../../crates/pwright-backend-cdp-file-input/src/target_marker_create.js', import.meta.url), 'utf8');
const markSource = readFileSync(new URL('../../../crates/pwright-backend-cdp-file-input/src/target_marker_mark.js', import.meta.url), 'utf8');
const limits = { timeout: 5000 };
const MARK = '__playwright_mark_target__';
const UNMARK = '__playwright_unmark_target__';

function fixture(mode) {
  const context = createContext({ mode });
  const run = expression => runInContext(expression, context, { timeout: 1000 });
  run(`
    globalThis.candidateCreate = (${createSource});
    globalThis.candidateMark = (${markSource});
    globalThis.injected = ({ ${pinnedMethod} });
    globalThis.marker = candidateCreate();
    globalThis.trace = [];
    globalThis.events = [];
    globalThis.sentinel = new Error('controlled utility method failure');
    globalThis.fault = '';
    globalThis.dispatchReturn = true;
    globalThis.currentState = () => mode === 'pinned' ? injected._markedElements : marker.state;
    globalThis.stateSummary = () => {
      const state = currentState();
      return state === undefined ? null : { callId: state.callId, nodes: Array.from(state.elements, node => node.name) };
    };
    globalThis.CustomEvent = class {
      constructor(type, options) {
        trace.push(['construct', type, stateSummary()]);
        if (fault === 'construct:' + type) throw sentinel;
        this.type = type;
        this.detail = options.detail;
        this.bubbles = options.bubbles;
        this.cancelable = options.cancelable;
        this.composed = options.composed;
        this.optionKeys = Object.keys(options);
      }
    };
    globalThis.makeNode = (name, type = 1) => ({
      name,
      get nodeType() { trace.push(['nodeType', name]); if (fault === 'nodeType:' + name) throw sentinel; return type; },
      get isConnected() { throw new Error('marking must not inspect connectivity'); },
      get tagName() { throw new Error('marking must not inspect input type'); },
      get disabled() { throw new Error('marking must not inspect actionability'); },
      get parentElement() { throw new Error('marking must not retarget'); },
      dispatchEvent(event) {
        trace.push(['dispatch', name, event.type, stateSummary()]);
        events.push({ node: name, type: event.type, detail: event.detail, bubbles: event.bubbles,
          cancelable: event.cancelable, composed: event.composed, optionKeys: event.optionKeys });
        if (fault === 'dispatch:' + name + ':' + event.type) throw sentinel;
        return dispatchReturn;
      }
    });
    globalThis.nodes = { A: makeNode('A'), B: makeNode('B'), C: makeNode('C'),
      text: makeNode('text', 3), document: makeNode('document', 9), stringType: makeNode('stringType', '1') };
  `);
  return {
    run,
    async invoke(node, callId) {
      context.currentNode = context.nodes[node];
      context.currentCallId = callId;
      try {
        let result;
        if (mode === 'pinned') {
          result = await Reflect.apply(ElementHandle.prototype._markAsTargetElement, {
            evaluateInUtility(callback, arg) {
              assert.equal(arg, callId);
              // The actual pinned callback supplies its node guard and Set.
              context.callbackSource = String(callback);
              return run('eval("(" + callbackSource + ")")([injected, currentNode, currentCallId])');
            },
          }, [{ metadata: { id: callId }, race: value => Promise.resolve(value) }]);
        } else result = run('candidateMark(marker, currentNode, currentCallId)');
        assert.equal(result, undefined);
        return { returnedUndefined: true };
      } catch (error) {
        return { thrownSentinel: error === context.sentinel, name: error.name, message: error.message };
      }
    },
    snapshot() {
      return JSON.parse(run('JSON.stringify({ trace, events, state: stateSummary() })'));
    },
  };
}

async function paired(steps, setup = '') {
  const results = [];
  for (const mode of ['pinned', 'candidate']) {
    const target = fixture(mode);
    if (setup) target.run(setup);
    const outcomes = [];
    for (const step of steps) {
      if (step.before) target.run(step.before);
      const result = await target.invoke(step.node ?? 'A', step.id ?? 'call@1');
      outcomes.push({ result, ...target.snapshot() });
    }
    results.push(outcomes);
  }
  assert.deepEqual(results[1], results[0]);
  return results[1];
}

test('marker creation contract: separate ordinary private containers define own undefined state', limits, () => {
  // Arrange
  const target = fixture('candidate');
  // Act
  const shape = JSON.parse(target.run(`JSON.stringify((() => {
    const a = candidateCreate(), b = candidateCreate();
    const property = Object.getOwnPropertyDescriptor(a, 'state');
    return { separate: a !== b, ordinary: Object.getPrototypeOf(a) === Object.prototype,
      keys: Object.keys(a), undefinedState: property.value === undefined,
      writable: property.writable, enumerable: property.enumerable, configurable: property.configurable };
  })())`));
  // Assert: candidate ownership shape, not a pinned constructor comparison.
  assert.deepEqual(shape, { separate: true, ordinary: true, keys: ['state'], undefinedState: true,
    writable: true, enumerable: true, configurable: true });
});

test('marker creation contract: own state bypasses inherited setters', limits, () => {
  // Arrange
  const target = fixture('candidate');
  target.run(`Object.defineProperty(Object.prototype, 'state', { set() { throw sentinel; }, configurable: true });`);
  // Act
  const result = target.run(`Object.hasOwn(candidateCreate(), 'state')`);
  // Assert
  assert.equal(result, true);
});

test('pinned marker differential: first mark has exact flags/detail and commits after dispatch', limits, async () => {
  // Arrange / Act
  const [value] = await paired([{ node: 'A' }]);
  // Assert
  assert.deepEqual(value.events, [{ node: 'A', type: MARK, detail: 'call@1', bubbles: true,
    cancelable: true, composed: true, optionKeys: ['bubbles', 'cancelable', 'detail', 'composed'] }]);
  assert.deepEqual(value.trace.at(-1), ['dispatch', 'A', MARK, null]);
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
});

test('pinned marker differential: persistent same-call same-node state suppresses duplicate events', limits, async () => {
  // Arrange / Act
  const [, value] = await paired([{ node: 'A' }, { node: 'A' }]);
  // Assert: constructors still execute even when no dispatch is necessary.
  assert.equal(value.events.length, 1);
  assert.equal(value.trace.filter(entry => entry[0] === 'construct').length, 4);
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
});

test('pinned marker differential: same-call replacement unmarks then marks before committing', limits, async () => {
  // Arrange / Act
  const [, value] = await paired([{ node: 'A' }, { node: 'B' }]);
  // Assert
  assert.deepEqual(value.events.map(event => [event.type, event.node]), [[MARK, 'A'], [UNMARK, 'A'], [MARK, 'B']]);
  assert.deepEqual(value.trace.filter(entry => entry[0] === 'dispatch').slice(-2), [
    ['dispatch', 'A', UNMARK, { callId: 'call@1', nodes: ['A'] }],
    ['dispatch', 'B', MARK, { callId: 'call@1', nodes: ['A'] }],
  ]);
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['B'] });
});

for (const node of ['A', 'B']) {
  test(`pinned marker differential: different call marks ${node} without unmarking the old target`, limits, async () => {
    // Arrange / Act
    const [, value] = await paired([{ node: 'A' }, { node, id: 'call@2' }]);
    // Assert: previous state is reset before event construction/dispatch.
    assert.deepEqual(value.events.map(event => [event.type, event.node, event.detail]), [[MARK, 'A', 'call@1'], [MARK, node, 'call@2']]);
    assert.deepEqual(value.trace.filter(entry => entry[0] === 'construct').slice(-2).map(entry => entry[2]), [null, null]);
  });
}

test('pinned marker differential: empty call ID skips even the node getter and retains prior state', limits, async () => {
  // Arrange / Act
  const [before, after] = await paired([{ node: 'A' }, { node: 'B', id: '', before: "fault = 'nodeType:B';" }]);
  // Assert
  assert.deepEqual(after, before);
});

for (const node of ['text', 'document', 'stringType']) {
  test(`pinned marker differential: ${node} node type skips marking without retargeting`, limits, async () => {
    // Arrange / Act
    const [, value] = await paired([{ node: 'A' }, { node, id: 'call@2' }]);
    // Assert
    assert.equal(value.events.length, 1);
    assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
    assert.deepEqual(value.trace.at(-1), ['nodeType', node]);
  });
}

for (const id of ['call@1', 'call@2']) {
  for (const fault of [`construct:${UNMARK}`, `dispatch:B:${MARK}`]) {
    test(`pinned marker differential: ${fault} preserves the correct partial state for ${id}`, limits, async () => {
      // Arrange / Act
      const [, value] = await paired([{ node: 'A' }, { node: 'B', id, before: `fault = ${JSON.stringify(fault)};` }]);
      // Assert: same-ID old state survives; changed-ID state was already cleared.
      assert.equal(value.result.thrownSentinel, true);
      assert.deepEqual(value.state, id === 'call@1' ? { callId: 'call@1', nodes: ['A'] } : null);
    });
  }
}

test('pinned marker differential: throwing nodeType leaves even different-call state untouched', limits, async () => {
  // Arrange / Act
  const [, value] = await paired([{ node: 'A' }, { node: 'B', id: 'call@2', before: "fault = 'nodeType:B';" }]);
  // Assert
  assert.equal(value.result.thrownSentinel, true);
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
});

test('pinned marker differential: false dispatch return does not cancel state publication', limits, async () => {
  // Arrange / Act
  const [value] = await paired([{ node: 'A' }], 'dispatchReturn = false;');
  // Assert: descriptor return-value test, not real DOM preventDefault coverage.
  assert.deepEqual(value.result, { returnedUndefined: true });
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
});

test('pinned marker differential: utility Set constructor is looked up on every invocation', limits, async () => {
  // Arrange / Act
  const [, value] = await paired([{ node: 'A' }, { node: 'B', before: `
    const NativeSet = Set;
    globalThis.Set = class extends NativeSet { constructor(values) { trace.push(['new-Set', !!values]); super(values); } };
  ` }]);
  // Assert: same ID keeps the earlier set, while the incoming set is new.
  assert.deepEqual(value.trace.filter(entry => entry[0] === 'new-Set'), [['new-Set', true]]);
});

test('pinned marker differential: incoming Set construction fails before different-call state reset', limits, async () => {
  // Arrange / Act: construction precedes markTargetElements itself upstream,
  // so even a new call ID must not clear the previous state on this failure.
  const [, value] = await paired([{ node: 'A' }, { node: 'B', id: 'call@2', before: `
    globalThis.Set = class { constructor() { trace.push(['new-Set-throws']); throw sentinel; } };
  ` }]);
  // Assert
  assert.equal(value.result.thrownSentinel, true);
  assert.deepEqual(value.trace.at(-1), ['new-Set-throws']);
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
  assert.equal(value.events.length, 1);
});

test('pinned marker differential: utility CustomEvent and target method remain dynamic after creation', limits, async () => {
  // Arrange / Act
  const [, value] = await paired([{ node: 'A' }, { node: 'B', before: `
    const PreviousEvent = CustomEvent;
    globalThis.CustomEvent = class extends PreviousEvent { constructor(type, options) { trace.push(['replacement-event', type]); super(type, options); } };
    const previousDispatch = nodes.B.dispatchEvent;
    nodes.B.dispatchEvent = function(event) { trace.push(['replacement-dispatch', this.name]); return previousDispatch.call(this, event); };
  ` }]);
  // Assert
  assert.equal(value.trace.filter(entry => entry[0] === 'replacement-event').length, 2);
  assert.deepEqual(value.trace.filter(entry => entry[0] === 'replacement-dispatch'), [['replacement-dispatch', 'B']]);
});

test('pinned marker differential: Set has is not a captured intrinsic', limits, async () => {
  // Arrange / Act
  const [, value] = await paired([{ node: 'A' }, { node: 'B', before: `Set.prototype.has = function() { throw sentinel; };` }]);
  // Assert
  assert.equal(value.result.thrownSentinel, true);
  assert.deepEqual(value.state, { callId: 'call@1', nodes: ['A'] });
});
