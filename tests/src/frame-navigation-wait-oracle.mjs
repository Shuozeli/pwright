// Offline internal-oracle gate, not a public Node facade or browser test.
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const require = createRequire(import.meta.url);
const playwrightRequire = createRequire(require.resolve('playwright/package.json'));
const packagePath = playwrightRequire.resolve('playwright-core/package.json');
assert.equal(require(packagePath).version, '1.58.2');
const { Frame, NavigationAbortedError } = require(resolve(dirname(packagePath), 'lib/server/frames.js'));
const { Request } = require(resolve(dirname(packagePath), 'lib/server/network.js'));
const rustBinary = process.argv[2];
assert.ok(rustBinary, 'provide the release frame_navigation_trace binary');
const receipt = loader => ({ type: 'receipt', loader });
const commit = loader => ({ type: 'commit', loader });
const start = (request, loader, from) => ({ type: 'start', request, loader, ...(from ? { from } : {}) });
const response = (request, status) => ({ type: 'response', request, response: { status } });
const lifecycle = event => ({ type: 'lifecycle', event });
const hash = { type: 'hash' };
const fail = request => ({ type: 'fail', request });
const cases = [];
for (const wait of ['commit', 'domcontentloaded', 'load', 'networkidle']) {
  const ready = wait === 'commit' ? [] : [lifecycle(wait)];
  cases.push(
    { name: `${wait}: early response and commit`, wait, steps: [start(1, 'a'), response(1, 500), commit('a'), ...ready, receipt('a')] },
    { name: `${wait}: late response`, wait, steps: [receipt('a'), start(1, 'a'), commit('a'), ...ready, response(1, 200)] },
    { name: `${wait}: hash waits lifecycle`, wait, steps: [receipt(null), hash, ...ready] },
    { name: `${wait}: hash uses existing lifecycle`, wait, steps: [...ready, hash, receipt(null)] },
    { name: `${wait}: redirect response`, wait, steps: [start(1, 'a'), response(1, 302), start(2, 'a', 1), response(2, 404), commit('a'), receipt('a'), ...ready] },
  );
}
cases.push(
  { name: 'earlier foreign commit interrupts', wait: 'load', steps: [commit('other'), commit('a'), receipt('a'), lifecycle('load')] },
  { name: 'matching earlier commit is not retroactively interrupted', wait: 'load', steps: [commit('a'), commit('other'), receipt('a'), lifecycle('load')] },
  { name: 'unrelated failure ignored', wait: 'load', steps: [start(1, 'other'), fail(1), commit('a'), receipt('a'), lifecycle('load')] },
  { name: 'matching pending failure', wait: 'load', steps: [start(1, 'a'), fail(1), receipt('a')] },
  { name: 'old request failure ignored', wait: 'load', steps: [start(1, 'old'), start(2, 'a'), fail(1), response(2, 200), commit('a'), receipt('a'), lifecycle('load')] },
  { name: 'failure after commit with response', wait: 'load', steps: [start(1, 'a'), response(1, 200), commit('a'), receipt('a'), fail(1), lifecycle('load')] },
  { name: 'failure after commit without response', wait: 'load', steps: [start(1, 'a'), commit('a'), receipt('a'), fail(1), lifecycle('load')] },
  { name: 'hash ignores new document events', wait: 'load', steps: [commit('other'), start(1, 'bad'), fail(1), hash, receipt(null), lifecycle('load')] },
  { name: 'commit without matching pending request has null response', wait: 'load', steps: [start(1, 'other'), commit('a'), receipt('a'), lifecycle('load')] },
);

async function oracle(test) {
  const frame = new EventEmitter();
  frame.attribution = {};
  frame.instrumentation = {};
  frame._firedLifecycleEvents = new Set(['commit']);
  let resolveReceipt;
  const nativeReceipt = new Promise(resolve => { resolveReceipt = resolve; });
  frame._page = { extraHTTPHeaders: () => [], delegate: { navigateFrame: () => nativeReceipt } };
  const requests = new Map();
  let pending;
  let result = { state: 'pending' };
  // Real gotoImpl and real Request redirect/response promises; only native
  // dispatch and ordered FrameManager notices are controlled by this adapter.
  const running = Frame.prototype.gotoImpl.call(frame, { race: promise => promise, log() {} }, 'http://example.test/a', { waitUntil: test.wait });
  running.then(value => { result = { state: 'complete', response: value }; }, error => {
    if (!(error instanceof NavigationAbortedError)) throw error;
    result = { state: error.message.includes('interrupted by another navigation') ? 'interrupted' : 'network' };
  });
  const snapshots = [];
  for (const step of test.steps) {
    switch (step.type) {
      case 'receipt': resolveReceipt({ newDocumentId: step.loader ?? undefined }); break;
      case 'start': {
        const request = new Request({}, frame, null, requests.get(step.from) ?? null, step.loader, 'http://example.test/a', 'document', 'GET', null, []);
        requests.set(step.request, request);
        pending = request;
        break;
      }
      case 'commit': {
        const request = pending?._documentId === step.loader ? pending : undefined;
        if (request) pending = undefined;
        frame._firedLifecycleEvents.clear();
        frame._firedLifecycleEvents.add('commit');
        frame.emit(Frame.Events.InternalNavigation, { url: `http://example.test/${step.loader}`, newDocument: { documentId: step.loader, request } });
        break;
      }
      case 'hash': frame.emit(Frame.Events.InternalNavigation, { url: 'http://example.test/a#hash' }); break;
      case 'response': requests.get(step.request)._setResponse(step.response); break;
      case 'fail': {
        const request = requests.get(step.request);
        request._setFailureText('net::ERR_ABORTED');
        if (pending === request) {
          pending = undefined;
          frame.emit(Frame.Events.InternalNavigation, { newDocument: { documentId: request._documentId, request }, error: new NavigationAbortedError(request._documentId, 'net::ERR_ABORTED') });
        }
        break;
      }
      case 'lifecycle': frame._firedLifecycleEvents.add(step.event); frame.emit(Frame.Events.AddLifecycle, step.event); break;
      default: throw new Error(`unknown step ${step.type}`);
    }
    // Drain promise reactions at a real event-loop checkpoint, not a guessed
    // fixed number of Promise.resolve microtasks.
    await new Promise(resolve => setImmediate(resolve));
    snapshots.push(result);
  }
  assert.notEqual(result.state, 'pending', `${test.name}: unresolved oracle`);
  await running.catch(() => {});
  assert.equal(frame.listenerCount(Frame.Events.InternalNavigation), 0);
  assert.equal(frame.listenerCount(Frame.Events.AddLifecycle), 0);
  return snapshots;
}
const expected = [];
for (const test of cases) expected.push(await oracle(test));
const result = spawnSync(resolve(rustBinary), [], { input: JSON.stringify(cases), encoding: 'utf8', timeout: 30_000, maxBuffer: 1024 * 1024 });
assert.ifError(result.error);
assert.equal(result.status, 0, result.stderr);
const actual = JSON.parse(result.stdout);
assert.equal(actual.length, expected.length);
for (let i = 0; i < cases.length; i++) assert.deepEqual(actual[i], expected[i], cases[i].name);
console.log(JSON.stringify({ oracle: 'playwright-core@1.58.2 Frame.gotoImpl + Request', kind: 'offline-internal-not-public-ffi', cases: cases.length, observations: expected.reduce((n, trace) => n + trace.length, 0), matched: true, adapter: fileURLToPath(import.meta.url) }));
