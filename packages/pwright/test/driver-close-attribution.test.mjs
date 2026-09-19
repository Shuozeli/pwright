import assert from 'node:assert/strict';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium } from '@shuozeli/pwright';
import { withCdpCommandRelay, WebSocket } from './cdp-command-relay.mjs';

const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');

// Diagnostic transport only: Chrome still executes every command. Hold exactly
// one successful Page initialization response, not a close acknowledgement.
class SnapshotGate {
  #pending = new Map();
  #armed = false;
  #held;
  #error;
  created = false;
  arm() { this.#armed = true; }
  command(raw, _client, upstream) {
    const message = JSON.parse(raw.toString());
    assert.ok(this.#pending.size < 32);
    this.#pending.set(message.id, message.method);
    upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    const method = this.#pending.get(message.id);
    this.#pending.delete(message.id);
    if (method === 'Target.createTarget' && !message.error) this.created = true;
    if (this.#armed && !this.#held && method === 'Page.enable') {
      assert.equal(this.created, true, 'native creation precedes the withheld initialization');
      assert.equal(message.error, undefined, 'Chrome has already answered initialization successfully');
      this.#held = { raw, client, id: message.id };
    } else client.send(raw, { binary: false });
  }
  release() {
    if (this.#held?.client.readyState === WebSocket.OPEN)
      this.#held.client.send(this.#held.raw, { binary: false });
  }
  fail(error) { this.#error ??= error; }
  check() { if (this.#error) throw this.#error; }
  heldIdentity() { return this.#held && { id: this.#held.id, method: 'Page.enable' }; }
}

test('browser.close attributes a retained initialization failure, not a close command', { timeout: 20000 }, async () => {
  // Arrange
  await withCdpCommandRelay(endpoint, new SnapshotGate(), async (relay, gate) => {
    const browser = await chromium.connectOverCDP(relay, { timeout: 5000 });
    try {
      const context = await browser.newContext();
      gate.arm();
      // Act: native creation succeeds, then the existing response deadline
      // expires in its initialization snapshot. Joining the driver reports it.
      const requestError = await context.newPage().then(() => undefined, error => error);
      const closeError = await browser.close().then(() => undefined, error => error);
      // Assert
      assert.equal(gate.created, true);
      assert.ok(gate.heldIdentity());
      assert.equal(requestError?.code, 'PWRIGHT_TARGET_CLOSED');
      assert.equal(closeError?.code, 'PWRIGHT_BACKEND');
      assert.match(closeError.message, /native driver: topology snapshot: Timeout waiting for response/);
      assert.equal(browser.isConnected(), false);
      assert.equal(browser.pwrightMetrics().activeLeases, 0);
      assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
      for (let index = 0; index < 100 && browser.pwrightFfiMetrics().pending; index++) await setImmediate();
      assert.equal(browser.pwrightFfiMetrics().pending, 0);
      assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      process.stdout.write(`# retained driver failure ${JSON.stringify({ request: requestError.message,
        close: closeError.message, held: gate.heldIdentity(), created: gate.created })}\n`);
    } finally {
      await browser.close();
    }
  });
});
