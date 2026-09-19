import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { isAbsolute } from 'node:path';
import { test } from 'node:test';
import { setImmediate } from 'node:timers';
import { chromium } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(import.meta.url);
const { nativeArtifact } = require('../src/native-artifact.cjs');
const peerPath = process.env.PWRIGHT_PEER_ADDON;
assert.ok(peerPath && isAbsolute(peerPath), 'provide an independently copied addon fixture');
assert.notEqual(peerPath, nativeArtifact.path);
const primary = require(nativeArtifact.path);
const peer = require(peerPath);
assert.notEqual(primary.NativeHandle, peer.NativeHandle, 'fixture must load an independent native module');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');

test('a native module rejects a peer module Handle before reading its sidecar layout', { timeout: 20000 }, async () => {
  // Arrange
  const browser = await chromium.connectOverCDP(endpoint, { timeout: 5000 });
  const maximum = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'maxByteLength').get;
  const environment = new peer.NativeEnvironment(setImmediate, maximum);
  let context, other, otherContext, nativeHandle;
  try {
    context = await browser.newContext();
    const page = await context.newPage();
    other = await environment.connectOverCDP(endpoint, 5000);
    otherContext = await other.newContext();
    const otherPage = await other.newPage(otherContext.id);
    nativeHandle = await other.frameEvaluateHandle(otherPage.mainFrameId, '({ answer: 42 })', false, undefined);
    const wrappers = require('../src/handles.cjs')({ mapError: error => error, UnsupportedOperationError: Error });
    const wrapper = await wrappers.create(other, nativeHandle);

    // Act: a global constant brand would get past conversion into the foreign
    // owner check; a module-local brand rejects before any Rust layout cast.
    const result = page.evaluate(value => value.answer, wrapper);

    // Assert
    await assert.rejects(result, { code: 'PWRIGHT_UNSUPPORTED' });
    assert.deepEqual(await wrapper.jsonValue(), { answer: 42 });
  } finally {
    const failures = [];
    for (const close of [
      () => nativeHandle && other.handleDispose(nativeHandle),
      () => otherContext && other.closeContext(otherContext.id),
      () => other?.close(),
      () => context?.close(),
      () => browser.close(),
    ]) {
      try { await diagnosticDeadline(Promise.resolve(close()), 'module isolation cleanup'); }
      catch (error) { failures.push(error); }
    }
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    if (other) assert.equal(other.metrics().activeLeases, 0);
    if (failures.length) throw new AggregateError(failures, 'module isolation cleanup failed');
  }
});
