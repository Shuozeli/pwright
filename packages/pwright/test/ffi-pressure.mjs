import assert from 'node:assert/strict';

export const zeroFfiMetrics = Object.freeze({
  pending: 0, queued: 0, queuedCommands: 0, running: 0, reservedBytes: 0,
  connects: 0, commands: 0, closes: 0, watches: 0,
});

export function queueReadsWithoutDelivering(page, browser, count = 64) {
  const outcomes = Array.from({ length: count }, () => page.content().then(
    value => ({ ok: true, bytes: Buffer.byteLength(value) }),
    error => ({ ok: false, code: error.code }),
  ));
  const admitted = browser.pwrightFfiMetrics();
  assert.equal(admitted.commands, 32, 'only 32 command settlements can be admitted');
  assert.ok(admitted.reservedBytes <= 64 * 1024 * 1024);
  return outcomes;
}

export function blockUntilResultsAreQueued(browser) {
  const clock = new Int32Array(new SharedArrayBuffer(4));
  const deadline = Date.now() + 10_000;
  let metrics = browser.pwrightFfiMetrics();
  while (metrics.queuedCommands < 32) {
    assert.ok(Date.now() < deadline, `native results did not accumulate while JS was blocked: ${JSON.stringify(metrics)}`);
    // No JS event loop turn, Promise continuation or Node-API resolver runs.
    Atomics.wait(clock, 0, 0, 10);
    metrics = browser.pwrightFfiMetrics();
  }
  assert.equal(metrics.commands, 32);
  assert.ok(metrics.reservedBytes >= 32 * (1024 * 1024 + 4096));
  assert.equal(browser.pwrightMetrics().activeOperations, 0);
  return metrics;
}
