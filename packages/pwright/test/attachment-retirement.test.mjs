import assert from 'node:assert/strict';
import { EventEmitter, once } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { withCdpCommandRelay, WebSocket } from './cdp-command-relay.mjs';
import artifactConfig from '../src/native-artifact.cjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');

class RetirementGate extends EventEmitter {
  #url;
  #method;
  #session;
  #pending;
  #released = false;
  #error;
  #receipt;
  #detached = false;
  #commands = new Map();
  #occurrence;
  #seen = 0;
  constructor(url, method, occurrence) {
    super(); this.#url = url; this.#method = method; this.#occurrence = occurrence;
  }
  command(raw, _client, upstream) {
    const command = JSON.parse(raw.toString());
    assert.ok(this.#commands.size < 128, 'bounded diagnostic command inventory');
    this.#commands.set(command.id, { method: command.method, session: command.sessionId });
    const selected = this.#session !== undefined && command.sessionId === this.#session && command.method === this.#method;
    if (selected) this.#seen++;
    if (!this.#released && selected && this.#seen === this.#occurrence) {
      assert.equal(this.#pending, undefined, 'exactly one native command is held');
      this.#pending = { raw, upstream, id: command.id };
      this.emit('held');
    } else {
      upstream.send(raw, { binary: false });
    }
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    if (message.id !== undefined) {
      const command = this.#commands.get(message.id);
      this.#commands.delete(message.id);
      if (message.error) process.stdout.write(`# native retirement diagnostic ${JSON.stringify({
        method: command?.method, ownedSession: command?.session === this.#session, error: message.error,
      })}\n`);
    }
    if (message.method === 'Target.attachedToTarget' && message.params.targetInfo.url === this.#url) {
      assert.equal(this.#session, undefined, 'exactly one test-owned Page session');
      this.#session = message.params.sessionId;
    }
    if (message.method === 'Target.detachedFromTarget' && message.params.sessionId === this.#session) {
      this.#detached = true;
      this.emit('detached');
    }
    if (this.#pending && message.id === this.#pending.id) {
      this.#receipt = message.error;
      this.emit('receipt');
    }
    client.send(raw, { binary: false });
  }
  async held() {
    if (!this.#pending) await diagnosticDeadline(once(this, 'held'), 'held attachment command');
    this.check();
  }
  async detached() {
    if (!this.#detached) await diagnosticDeadline(once(this, 'detached'), 'exact attachment detach');
    this.check();
  }
  release() {
    if (!this.#released && this.#pending) {
      this.#released = true;
      if (this.#pending.upstream.readyState === WebSocket.OPEN)
        this.#pending.upstream.send(this.#pending.raw, { binary: false });
    }
  }
  async missingReceipt() {
    if (!this.#receipt) await diagnosticDeadline(once(this, 'receipt'), 'native missing-session receipt');
    assert.deepEqual(this.#receipt, { code: -32001, message: 'Session with given id not found.' });
  }
  fail(error) { this.#error ??= error; this.emit('held'); this.emit('receipt'); this.emit('detached'); }
  check() { if (this.#error) throw this.#error; }
}

async function closeBrowser(browser) {
  await diagnosticDeadline(browser.close(), 'Browser disconnect');
  if (browser.pwrightMetrics) {
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
    for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
    assert.equal(browser.pwrightFfiMetrics().pending, 0);
    assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
  }
}

async function retireDuringAttachment(type, method, occurrence, detachBeforeReceipt = false) {
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  const context = await controller.newContext();
  try {
    const page = await context.newPage();
    const marker = `about:blank#node090-${method}`;
    await page.goto(marker);
    // The combined Rust profile enables Network once for lifecycle events and
    // again for header initialization. Upstream uses one shared network manager.
    const nativeOccurrence = type === oracle ? 1 : occurrence;
    return await withCdpCommandRelay(endpoint, new RetirementGate(marker, method, nativeOccurrence), async (relay, gate) => {
      const connecting = type.connectOverCDP(relay, { timeout: 5000 })
        .then(browser => ({ browser }), error => ({ error: error.message.split('\n')[0] }));
      let result;
      try {
        await gate.held();
        // Act: close only the controller's fresh Page while native init waits.
        await page.close();
        if (detachBeforeReceipt) {
          // Keep the exact command held: native detach must settle initialization
          // without relying on any later error response or a longer timeout.
          await gate.detached();
          result = await connecting;
          if (!result.browser) return { connected: false, error: result.error };
        }
        gate.release();
        await gate.missingReceipt();
        result = await connecting;
        if (!result.browser) return { connected: false, error: result.error };
        // Assert: the vanished Page is not published; a fresh owned Page works.
        const stale = result.browser.contexts().flatMap(context => context.pages())
          .some(page => page.url() === marker);
        const fresh = await result.browser.newContext();
        try {
          const healthy = await fresh.newPage();
          return { connected: true, stale, value: await healthy.evaluate(() => 42) };
        } finally { await fresh.close(); }
      } finally {
        gate.release();
        result ??= await connecting;
        if (result.browser) await closeBrowser(result.browser);
      }
    });
  } finally {
    try { await context.close(); } finally { await controller.close(); }
  }
}

const features = artifactConfig.nativeFeatures(artifactConfig.nativeArtifact.profile);
const methods = ['Page.enable', 'Page.getFrameTree', 'Runtime.enable',
  'Target.setAutoAttach', 'Runtime.runIfWaitingForDebugger'].map(method => [method, 1]);
if (features.includes('frame-navigation'))
  methods.push(['Network.enable', 1], ['Page.setLifecycleEventsEnabled', 1]);
if (features.includes('extra-headers'))
  methods.push(['Network.enable', features.includes('frame-navigation') ? 2 : 1]);
for (const [method, occurrence] of methods) {
  test(`connection survives a test-owned Page retiring during ${method} occurrence ${occurrence}`, { timeout: 30000 }, async () => {
    // Arrange
    const expected = await retireDuringAttachment(oracle, method, occurrence);
    assert.deepEqual(expected, { connected: true, stale: false, value: 42 });
    // Act
    const actual = await retireDuringAttachment(candidate, method, occurrence);
    // Assert
    assert.deepEqual(actual, expected);
  });
}

const detachedMethods = [['Page.enable', 1]];
if (features.includes('extra-headers'))
  detachedMethods.push(['Network.enable', features.includes('frame-navigation') ? 2 : 1]);
for (const [method, occurrence] of detachedMethods) {
  test(`native detach settles ${method} initialization before the held command receives a response`, { timeout: 30000 }, async () => {
    // Arrange
    const expected = await retireDuringAttachment(oracle, method, occurrence, true);
    assert.deepEqual(expected, { connected: true, stale: false, value: 42 });
    // Act
    const actual = await retireDuringAttachment(candidate, method, occurrence, true);
    // Assert
    assert.deepEqual(actual, expected);
  });
}
