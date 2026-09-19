import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createRequire } from 'node:module';
import { parentPort, workerData } from 'node:worker_threads';

// Test controller only. Native creation continues while the subject JS thread
// is blocked. Cleanup is limited to the exact returned target in the anchor's
// independently owned Context; shared Chrome Pages are never cleanup targets.
const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const { chromium } = require('playwright');
assert.equal(workerData.endpoint, 'http://127.0.0.1:9222');
assert.ok(workerData.marker.startsWith('about:blank#node104-'));
const state = new Int32Array(workerData.state);
let browser, session, targetId;
try {
  browser = await chromium.connectOverCDP(workerData.endpoint, { timeout: 5000 });
  session = await browser.newBrowserCDPSession();
  const targets = (await session.send('Target.getTargets')).targetInfos
    .filter(target => target.url === workerData.anchor);
  assert.equal(targets.length, 1, 'exactly one independently owned anchor');
  assert.ok(targets[0].browserContextId, 'never create in the shared default Context');
  const finish = once(parentPort, 'message');
  parentPort.postMessage('ready');
  Atomics.wait(state, 0, 0, 10000);
  assert.equal(Atomics.load(state, 0), 1, 'parent explicitly requests creation');
  ({ targetId } = await session.send('Target.createTarget', {
    browserContextId: targets[0].browserContextId, url: workerData.marker,
  }));
  assert.equal(typeof targetId, 'string');
  Atomics.store(state, 0, 2);
  Atomics.notify(state, 0);
  const [message] = await finish;
  assert.equal(message, 'finish');
} catch (error) {
  Atomics.store(state, 0, -1);
  Atomics.notify(state, 0);
  throw error;
} finally {
  try { if (targetId) await session.send('Target.closeTarget', { targetId }); }
  finally {
    try { await session?.detach(); }
    finally { await browser?.close(); }
  }
}
