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
const marker = 'about:blank#node094-';

// Hold one known default-Context Page initialization while two controller-owned
// Contexts appear and disappear. Only native event/command timing is controlled.
class QueuedContextGate extends EventEmitter {
  #keeper;
  #held;
  #released = false;
  #targets = new Map();
  #detached = new Set();
  #commands = new Map();
  #rejectedTemporaryPages = 0;
  #error;
  command(raw, _client, upstream) {
    const message = JSON.parse(raw.toString());
    const detachedSession = message.method === 'Target.detachFromTarget' ? message.params.sessionId : undefined;
    const owned = [...this.#targets.values()].includes(detachedSession ?? message.sessionId);
    if (detachedSession !== undefined && message.sessionId === undefined && owned && detachedSession !== this.#keeper)
      this.#rejectedTemporaryPages++;
    assert.ok(this.#commands.size < 128, 'bounded pending native command inventory');
    this.#commands.set(message.id, { method: message.method, session: message.sessionId,
      detachedSession, owned });
    if (!this.#released && this.#keeper !== undefined && message.sessionId === this.#keeper && message.method === 'Page.enable') {
      assert.equal(this.#held, undefined, 'one held keeper initialization command');
      this.#held = { raw, upstream };
      this.emit('change');
    } else upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    if (message.method === 'Target.attachedToTarget' && message.params.targetInfo.url.startsWith(marker)) {
      assert.ok(this.#targets.size < 3, 'only three test-owned target identities');
      this.#targets.set(message.params.targetInfo.targetId, message.params.sessionId);
      if (message.params.targetInfo.url === `${marker}keeper`) this.#keeper = message.params.sessionId;
      this.emit('change');
    }
    if (message.method === 'Target.detachedFromTarget' && [...this.#targets.values()].includes(message.params.sessionId)) {
      this.#detached.add(message.params.sessionId);
      this.emit('change');
    }
    if (message.id !== undefined) {
      const command = this.#commands.get(message.id);
      this.#commands.delete(message.id);
      if (message.error) process.stdout.write(`# queued-context native receipt ${JSON.stringify({ ...command, error: message.error })}\n`);
    }
    client.send(raw, { binary: false });
  }
  async waitFor(predicate, label) {
    await diagnosticDeadline((async () => {
      while (!predicate()) { this.check(); await once(this, 'change'); }
      this.check();
    })(), label);
  }
  held() { return this.waitFor(() => this.#held !== undefined, 'keeper initialization held'); }
  attached(target) { return this.waitFor(() => this.#targets.has(target), 'owned temporary Page attached'); }
  detached(target) { return this.waitFor(() => this.#detached.has(this.#targets.get(target)), 'owned temporary Page detached'); }
  release() {
    this.#released = true;
    if (this.#held) {
      const { raw, upstream } = this.#held;
      this.#held = undefined;
      if (upstream.readyState === WebSocket.OPEN) upstream.send(raw, { binary: false });
    }
  }
  fail(error) { this.#error ??= error; this.emit('change'); }
  check() { if (this.#error) throw this.#error; }
  acceptedTemporaryPages() { assert.equal(this.#rejectedTemporaryPages, 0, 'no root detach rejects these borrowed Pages'); }
}

async function exercise(type, retired) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  let keeper;
  let native;
  const contexts = new Map();
  try {
    keeper = await controller.contexts()[0].newPage();
    native = await controller.newBrowserCDPSession();
    await keeper.goto(`${marker}keeper`);
    const result = await withCdpCommandRelay(endpoint, new QueuedContextGate(), async (relay, gate) => {
      const connecting = type.connectOverCDP(relay, { timeout: 5000 })
        .then(browser => ({ browser }), error => ({ error: error.message.split('\n')[0] }));
      let connection;
      try {
        await gate.held();
        // Act: dispose only Contexts created by this controller, after each real
        // attachment reaches the blocked test connection's transport.
        for (let index = 0; index < 2; index++) {
          const { browserContextId } = await native.send('Target.createBrowserContext', { disposeOnDetach: true });
          contexts.set(browserContextId, undefined);
          const { targetId } = await native.send('Target.createTarget', { browserContextId, url: `${marker}temporary-${index}` });
          contexts.set(browserContextId, targetId);
          await gate.attached(targetId);
          if (retired) {
            await native.send('Target.disposeBrowserContext', { browserContextId });
            contexts.delete(browserContextId);
            await gate.detached(targetId);
          }
        }
        gate.release();
        connection = await connecting;
        if (!connection.browser) return { connected: false, error: connection.error };
        if (type === candidate) gate.acceptedTemporaryPages();
        const fresh = await connection.browser.newContext();
        try {
          const page = await fresh.newPage();
          const temporary = connection.browser.contexts().flatMap(context => context.pages())
            .filter(page => page.url().startsWith(`${marker}temporary-`));
          return { connected: true, value: await page.evaluate(() => 42),
            temporaryValues: await Promise.all(temporary.map(page => page.evaluate(() => 42))) };
        } finally { await fresh.close(); }
      } finally {
        gate.release();
        connection ??= await connecting;
        if (connection.browser) {
          await diagnosticDeadline(connection.browser.close(), 'Browser disconnect');
          if (connection.browser.pwrightMetrics) {
            assert.equal(connection.browser.pwrightMetrics().activeLeases, 0);
            assert.equal(connection.browser.pwrightMetrics().cleanupInFlight, 0);
            for (let turn = 0; turn < 100 && connection.browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
            assert.equal(connection.browser.pwrightFfiMetrics().pending, 0);
            assert.equal(connection.browser.pwrightFfiMetrics().reservedBytes, 0);
          }
        }
      }
    });
    assert.equal(await keeper.evaluate(() => 42), 42, 'borrowed default-Context Page survives disconnect');
    for (const [browserContextId, targetId] of contexts) {
      const { targetInfo } = await native.send('Target.getTargetInfo', { targetId });
      assert.equal(targetInfo.browserContextId, browserContextId, 'live foreign Context survives test-browser disconnect');
    }
    return result;
  } finally {
    try {
      for (const browserContextId of contexts.keys()) await native.send('Target.disposeBrowserContext', { browserContextId });
    } finally {
      try { await keeper?.close(); } finally { await controller.close(); }
    }
  }
}

for (const retired of [true, false]) test(retired
  ? 'connection survives queued attachments from already disposed foreign Contexts'
  : 'connection discovers and borrows late live foreign Context Pages', { timeout: 30000 }, async () => {
  // Arrange
  const expected = await exercise(oracle, retired);
  assert.deepEqual(expected, { connected: true, value: 42, temporaryValues: retired ? [] : [42, 42] });
  // Act
  const actual = await exercise(candidate, retired);
  // Assert
  assert.deepEqual(actual, expected);
});
