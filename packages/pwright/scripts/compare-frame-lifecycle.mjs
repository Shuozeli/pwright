// Internal FrameManager differential: no public Frame.goto or browser claim.
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { createRequire } from 'node:module';
import { dirname, join, isAbsolute } from 'node:path';
import { spawnSync } from 'node:child_process';
import { writeFile } from 'node:fs/promises';
import { cases } from './frame-lifecycle-cases.mjs';

const requireOracle = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(requireOracle('playwright/package.json').version, '1.58.2');
const requireCore = createRequire(requireOracle.resolve('playwright/package.json'));
const root = dirname(requireCore.resolve('playwright-core/package.json'));
const { FrameManager, Frame } = requireCore(join(root, 'lib/server/frames.js'));
const { createRootSdkObject } = requireCore(join(root, 'lib/server/instrumentation.js'));

class FakePage extends EventEmitter {
  constructor() {
    super();
    const sdk = createRootSdkObject();
    this.attribution = sdk.attribution;
    this.instrumentation = sdk.instrumentation;
    this.frameManager = new FrameManager(this);
  }
  mainFrame() { return this.frameManager.mainFrame(); }
  isClosed() { return false; }
  addNetworkRequest() {}
  emitOnContext() {}
}

function observe(steps) {
  const originalSet = globalThis.setTimeout, originalClear = globalThis.clearTimeout;
  let now = 0, nextTimer = 1;
  const timers = new Map();
  const page = new FakePage(), manager = page.frameManager;
  const states = new Map(), requests = new Map();
  globalThis.setTimeout = (callback, delay) => {
    assert.equal(delay, 500, 'only the pinned idle timer is replaced');
    assert.ok(timers.size < 32);
    const id = nextTimer++;
    timers.set(id, { at: now + delay, callback });
    return id;
  };
  globalThis.clearTimeout = id => { timers.delete(id); };
  const observations = [];
  try {
    for (const step of steps) {
      switch (step.type) {
        case 'attach': {
          const frame = manager.frameAttached(String(step.frame), step.parent === null ? null : String(step.parent));
          // Constructor emits no lifecycle event; its initial commit bit is
          // specified by the pinned constructor. All later bits use events.
          const state = { document: null, fired: new Set(['commit']) };
          states.set(step.frame, state);
          frame.on(Frame.Events.AddLifecycle, event => state.fired.add(event));
          frame.on(Frame.Events.RemoveLifecycle, event => state.fired.delete(event));
          frame.on(Frame.Events.InternalNavigation, event => {
            if (event.newDocument) state.document = Number(event.newDocument.documentId);
          });
          break;
        }
        case 'commit': manager.frameCommittedNewDocumentNavigation(String(step.frame), `https://fixture.invalid/${step.document}`, '', String(step.document), true); break;
        case 'hash': manager.frameCommittedSameDocumentNavigation(String(step.frame), 'https://fixture.invalid/#hash'); break;
        case 'lifecycle': manager.frameLifecycleEvent(String(step.frame), step.event); break;
        case 'start': {
          let request = requests.get(step.request);
          if (!request) {
            const frame = manager.frame(String(step.frame));
            request = { frame: () => frame, _documentId: step.document === null ? undefined : String(step.document), _isFavicon: step.favicon };
            requests.set(step.request, request);
          }
          manager.requestStarted(request);
          break;
        }
        case 'finish': manager.reportRequestFinished(requests.get(step.request), null); break;
        case 'detach': manager.frameDetached(String(step.frame)); break;
        case 'tick': {
          assert.ok(step.ms >= now);
          for (;;) {
            const due = [...timers].filter(([, timer]) => timer.at <= step.ms).sort((a, b) => a[1].at - b[1].at || a[0] - b[0])[0];
            if (!due) break;
            timers.delete(due[0]); now = due[1].at; due[1].callback();
          }
          now = step.ms;
          break;
        }
        default: throw new Error(`unknown trace step ${step.type}`);
      }
      const snapshot = [];
      for (let id = 1; id <= 8; id++) {
        if (!manager.frame(String(id))) continue;
        const { document, fired } = states.get(id);
        snapshot.push([id, document, ...['commit', 'domcontentloaded', 'load', 'networkidle'].map(event => fired.has(event))]);
      }
      observations.push(snapshot);
    }
  } finally {
    try { manager.dispose(); assert.equal(timers.size, 0); }
    finally { globalThis.setTimeout = originalSet; globalThis.clearTimeout = originalClear; }
  }
  return observations;
}

const binary = process.env.PWRIGHT_FRAME_LIFECYCLE_BINARY;
assert.ok(typeof binary === 'string' && isAbsolute(binary));
const input = JSON.stringify(cases);
assert.ok(Buffer.byteLength(input) <= 512 * 1024);
const expected = cases.map(({ steps }) => observe(steps));
const result = spawnSync(binary, [], { input, encoding: 'utf8', timeout: 5000, maxBuffer: 1024 * 1024 });
if (result.error) throw result.error;
assert.equal(result.status, 0, result.stderr);
const actual = JSON.parse(result.stdout);
assert.equal(actual.length, cases.length);
const mismatches = [];
let total = 0;
for (const [index, spec] of cases.entries()) {
  assert.equal(actual[index].length, spec.steps.length);
  for (const [step, operation] of spec.steps.entries()) {
    total++;
    if (JSON.stringify(actual[index][step]) !== JSON.stringify(expected[index][step]))
      mismatches.push({ case: spec.name, step, operation, rust: actual[index][step], oracle: expected[index][step] });
  }
}
const report = { oracle: 'playwright@1.58.2 internal FrameManager', scope: 'normalized lifecycle state only; no browser or Frame.goto conformance claim', cases: cases.length, total, matched: total - mismatches.length, mismatches };
if (process.env.PWRIGHT_FRAME_LIFECYCLE_REPORT) {
  assert.ok(isAbsolute(process.env.PWRIGHT_FRAME_LIFECYCLE_REPORT));
  await writeFile(process.env.PWRIGHT_FRAME_LIFECYCLE_REPORT, JSON.stringify(report, null, 2), { flag: 'wx' });
}
console.log(`Rust Frame lifecycle: ${report.matched}/${total} snapshots across ${cases.length} traces`);
assert.equal(mismatches.length, 0, JSON.stringify(mismatches.slice(0, 5)));
