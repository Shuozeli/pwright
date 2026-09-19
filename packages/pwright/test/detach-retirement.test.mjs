import assert from 'node:assert/strict';
import { EventEmitter, once } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { withCdpCommandRelay, WebSocket } from './cdp-command-relay.mjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');

// Hold a successful resume ACK, not a fabricated error. The controller then
// terminates its own Worker before Rust can send the second cleanup command.
class ResumeReceiptGate extends EventEmitter {
  #url;
  #child;
  #parent;
  #resumeId;
  #detachId;
  #held;
  #released = false;
  #detached = false;
  #receipt;
  #error;
  #holdDetach;
  #heldDetach;
  constructor(url, holdDetach) { super(); this.#url = url; this.#holdDetach = holdDetach; }
  command(raw, _client, upstream) {
    const message = JSON.parse(raw.toString());
    if (this.#child !== undefined && message.sessionId === this.#child &&
        message.method === 'Runtime.runIfWaitingForDebugger' && this.#resumeId === undefined)
      this.#resumeId = message.id;
    if (this.#child !== undefined && message.method === 'Target.detachFromTarget' &&
        message.params.sessionId === this.#child) {
      assert.equal(message.sessionId, this.#parent, 'detach addresses the announcing parent');
      assert.equal(this.#detachId, undefined, 'no repeated native detach attempts');
      this.#detachId = message.id;
    }
    upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    if (message.method === 'Target.attachedToTarget' && message.params.targetInfo.type === 'worker' &&
        message.params.targetInfo.url === this.#url) {
      assert.equal(this.#child, undefined, 'one test-owned native Worker session');
      this.#child = message.params.sessionId;
      this.#parent = message.sessionId;
      assert.ok(this.#parent);
    }
    if (message.id !== undefined && message.id === this.#resumeId && !this.#released) {
      assert.equal(message.error, undefined, 'Chrome successfully resumed the exact Worker');
      assert.equal(this.#held, undefined, 'one held receipt');
      this.#held = { raw, client };
      this.emit('change');
      return;
    }
    if (message.method === 'Target.detachedFromTarget' && message.params.sessionId === this.#child) {
      this.#detached = true;
      this.emit('change');
      if (this.#holdDetach) {
        assert.equal(this.#heldDetach, undefined, 'one exact held detach event');
        this.#heldDetach = { raw, client };
        return;
      }
    }
    if (message.id !== undefined && message.id === this.#detachId) {
      this.#receipt = message.error ?? null;
      process.stdout.write(`# exact child detach receipt ${JSON.stringify(this.#receipt)}\n`);
      this.emit('change');
    }
    client.send(raw, { binary: false });
    if (message.id !== undefined && message.id === this.#detachId) this.releaseDetach();
  }
  async waitFor(predicate, label) {
    await diagnosticDeadline((async () => {
      while (!predicate()) { this.check(); await once(this, 'change'); }
      this.check();
    })(), label);
  }
  held() { return this.waitFor(() => this.#held !== undefined, 'held successful Worker resume'); }
  detached() { return this.waitFor(() => this.#detached, 'real Worker detach event'); }
  async detachReceipt(retired) {
    await this.waitFor(() => this.#receipt !== undefined, 'second-stage native detach receipt');
    assert.deepEqual(this.#receipt, retired ? { code: -32602, message: 'No session with given id' } : null);
  }
  noDetach() { assert.equal(this.#detachId, undefined, 'native detach already retired the pending resume'); }
  releaseDetach() {
    if (this.#heldDetach) {
      const { raw, client } = this.#heldDetach;
      this.#heldDetach = undefined;
      if (client.readyState === WebSocket.OPEN) client.send(raw, { binary: false });
    }
  }
  release() {
    this.#released = true;
    if (this.#held) {
      const { raw, client } = this.#held;
      this.#held = undefined;
      if (client.readyState === WebSocket.OPEN) client.send(raw, { binary: false });
    }
  }
  fail(error) { this.#error ??= error; this.emit('change'); }
  check() { if (this.#error) throw this.#error; }
}

async function exercise(type, retired, receiptFirst) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  const context = await controller.newContext();
  try {
    const page = await context.newPage();
    const workerURL = await page.evaluate(() => new Promise(resolve => {
      const url = URL.createObjectURL(new Blob(['postMessage(42); onmessage = event => postMessage(event.data);'], { type: 'text/javascript' }));
      globalThis.node092Worker = new Worker(url);
      globalThis.node092Worker.onmessage = () => resolve(url);
    }));
    // Rust releases unsupported Worker inspector sessions; upstream owns Worker
    // initialization. Delay only Rust's native detach notification to preserve
    // the second-stage receipt path as a distinct controlled ordering test.
    const observation = await withCdpCommandRelay(endpoint,
      new ResumeReceiptGate(workerURL, type === candidate && retired && receiptFirst), async (relay, gate) => {
      const connecting = type.connectOverCDP(relay, { timeout: 5000 })
        .then(browser => ({ browser }), error => ({ error: error.message.split('\n')[0] }));
      let result;
      try {
        await gate.held();
        // Act: terminate only this Worker, or preserve it as a detach control.
        if (retired) {
          await page.evaluate(() => globalThis.node092Worker.terminate());
          await gate.detached();
        }
        if (retired && !receiptFirst) {
          result = await diagnosticDeadline(connecting, 'connection settles before held resume ACK');
          if (!result.browser) return { connected: false, error: result.error };
          if (type === candidate) gate.noDetach();
        }
        gate.release();
        if (type === candidate && (!retired || receiptFirst)) await gate.detachReceipt(retired);
        result = await diagnosticDeadline(connecting, 'connection completion');
        if (!result.browser) return { connected: false, error: result.error };
        const fresh = await result.browser.newContext();
        try {
          const healthy = await fresh.newPage();
          return { connected: true, value: await healthy.evaluate(() => 42),
            borrowedPage: await page.evaluate(() => 6 * 7),
            workerReply: retired ? null : await diagnosticDeadline(page.evaluate(() => new Promise(resolve => {
              globalThis.node092Worker.onmessage = event => resolve(event.data);
              globalThis.node092Worker.postMessage(7);
            })), 'live Worker survives inspector detach') };
        } finally { await fresh.close(); }
      } finally {
        gate.release();
        gate.releaseDetach();
        result ??= await connecting;
        if (result.browser) {
          await diagnosticDeadline(result.browser.close(), 'Browser disconnect');
          if (result.browser.pwrightMetrics) {
            assert.equal(result.browser.pwrightMetrics().activeLeases, 0);
            assert.equal(result.browser.pwrightMetrics().cleanupInFlight, 0);
            for (let turn = 0; turn < 100 && result.browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
            assert.equal(result.browser.pwrightFfiMetrics().pending, 0);
            assert.equal(result.browser.pwrightFfiMetrics().reservedBytes, 0);
          }
        }
      }
    });
    assert.equal(await page.evaluate(() => 42), 42, 'borrowed Page survives candidate disconnect');
    return observation;
  } finally {
    try { await context.close(); } finally { await controller.close(); }
  }
}

for (const [name, retired, receiptFirst] of [
  ['connection survives Worker retirement when the second-stage receipt precedes the detach notification', true, true],
  ['connection settles from Worker detach before the held resume ACK without another detach command', true, false],
  ['connection cleanup detaches its inspector without terminating a live Worker', false, false],
]) {
  test(name, { timeout: 30000 }, async () => {
    // Arrange: public lifecycle baseline, not a claim that Worker APIs are equal.
    const expected = await exercise(oracle, retired, receiptFirst);
    assert.deepEqual(expected, { connected: true, value: 42, borrowedPage: 42, workerReply: retired ? null : 7 });
    // Act
    const actual = await exercise(candidate, retired, receiptFirst);
    // Assert
    assert.deepEqual(actual, expected);
  });
}
