import assert from 'node:assert/strict';
import { EventEmitter, once } from 'node:events';
import { createRequire } from 'node:module';
import { test } from 'node:test';
import { setImmediate, setTimeout } from 'node:timers/promises';
import { Worker } from 'node:worker_threads';
import { chromium as candidate } from '@shuozeli/pwright';
import { withCdpCommandRelay, WebSocket } from './cdp-command-relay.mjs';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const require = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(require('playwright/package.json').version, '1.58.2');
const oracle = require('playwright').chromium;
const endpoint = process.env.PWRIGHT_TEST_CDP;
assert.equal(endpoint, 'http://127.0.0.1:9222');

// Hold only the owned Page's native retirement events, not command replies or
// authoritative target enumeration. Chrome still closes the Page normally.
class RetirementGate extends EventEmitter {
  #commands = new Map();
  #sessions = new Map();
  #held = [];
  #error;
  #client;
  #released = false;
  #holdContext;
  contextReplyHeld = false;
  target;
  context;
  acknowledged = false;
  destruction = false;
  constructor(holdContext = false) { super(); this.#holdContext = holdContext; }
  command(raw, _client, upstream) {
    const value = JSON.parse(raw.toString());
    // Retain only correlation needed by this gate. Domain-initialization
    // commands on borrowed Pages are forwarded without consuming its journal.
    if (['Target.createBrowserContext', 'Target.createTarget', 'Target.attachToTarget', 'Target.closeTarget', 'Target.disposeBrowserContext'].includes(value.method)) {
      assert.ok(this.#commands.size < 32);
      this.#commands.set(value.id, { method: value.method, target: value.params?.targetId, context: value.params?.browserContextId });
    }
    upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    this.#client = client;
    const value = JSON.parse(raw.toString());
    const command = this.#commands.get(value.id);
    this.#commands.delete(value.id);
    if (command?.method === 'Target.createBrowserContext' && value.result?.browserContextId) {
      assert.equal(this.context, undefined, 'exactly one owned Context');
      this.context = value.result.browserContextId;
    }
    if (command?.method === 'Target.createTarget' && value.result?.targetId) {
      assert.equal(this.target, undefined, 'exactly one owned Page');
      this.target = value.result.targetId;
    }
    if (value.method === 'Target.attachedToTarget') {
      assert.ok(this.#sessions.size < 32);
      this.#sessions.set(value.params.sessionId, value.params.targetInfo.targetId);
    }
    if (command?.method === 'Target.attachToTarget' && value.result?.sessionId) {
      assert.ok(this.#sessions.has(value.result.sessionId) || this.#sessions.size < 32);
      this.#sessions.set(value.result.sessionId, command.target);
    }
    const retired = value.method === 'Target.targetDestroyed' ? value.params.targetId :
      value.method === 'Target.detachedFromTarget' ? value.params.targetId ?? this.#sessions.get(value.params.sessionId) : undefined;
    const contextReply = this.#holdContext && command?.method === 'Target.disposeBrowserContext' && command.context === this.context;
    if (!this.#released && ((this.target && retired === this.target) || contextReply)) {
      assert.ok(this.#held.length < 8);
      this.#held.push({ raw, client });
      if (value.method === 'Target.targetDestroyed') this.destruction = true;
      if (contextReply) this.contextReplyHeld = true;
      this.emit('change');
      return;
    }
    client.send(raw, { binary: false });
    if (command?.method === 'Target.closeTarget' && command.target === this.target && !value.error) {
      this.acknowledged = true;
      this.emit('change');
    }
  }
  async held(requireDestruction = false) {
    await diagnosticDeadline((async () => {
      while (!this.acknowledged || this.#held.length === 0 || (requireDestruction && !this.destruction)) {
        this.check();
        await once(this, 'change');
      }
      this.check();
    })(), 'owned native close acknowledged and retirement held');
  }
  cut() { this.#client.close(); }
  release() {
    this.#released = true;
    for (const { raw, client } of this.#held)
      if (client.readyState === WebSocket.OPEN) client.send(raw, { binary: false });
    this.#held.length = 0;
  }
  fail(error) {
    if (!this.#error) process.stderr.write(`# retirement gate failure: ${String(error).slice(0, 4096)}\n`);
    this.#error ??= error;
    this.emit('change');
  }
  check() { if (this.#error) throw this.#error; }
}

async function exercise(type, mode) {
  const queuedContext = mode.startsWith('queued-context-');
  const gate = new RetirementGate(queuedContext);
  const result = await withCdpCommandRelay(endpoint, gate, async relay => {
    const browser = await type.connectOverCDP(relay, { timeout: 5000 });
    let context, closing, contextClosing;
    try {
      context = await browser.newContext();
      const page = await context.newPage();
      const notices = [];
      const contextNotices = [];
      context.on('close', function(value) { contextNotices.push({ sameArgument: value === context, sameReceiver: this === context, pageClosed: page.isClosed() }); });
      page.on('close', function(value) { notices.push({ sameArgument: value === page, sameReceiver: this === page, closed: page.isClosed() }); });
      let settled = false;
      let atSettlement;
      closing = page.close().then(() => {
        settled = true;
        atSettlement = { closed: page.isClosed(), notices: notices.length };
        return { ok: true };
      }, error => { settled = true; return { error: String(error), code: error.code }; });
      await gate.held();
      // A controlled observation window, not a replacement API deadline. This
      // exceeds the old 20 x 10 ms selection while wire acknowledgements flow.
      await setTimeout(300);
      const held = { settled, closed: page.isClosed(), listed: context.pages().includes(page), notices: notices.length };
      assert.deepEqual(held, { settled: false, closed: false, listed: true, notices: 0 });
      if (type === candidate) assert.equal(gate.destruction, true, 'selected native discovery source delivered exact destruction');
      if (queuedContext) {
        contextClosing = context.close().then(() => ({ ok: true }), error =>
          /Target page, context or browser has been closed/.test(String(error)) ? { closed: true } : { error: String(error) });
        await diagnosticDeadline((async () => {
          while (type === candidate ? browser.pwrightMetrics().activeOperations < 2 : !gate.contextReplyHeld) {
            gate.check(); await setTimeout(10);
          }
        })(), 'Context close admitted while Page confirmation is pending');
      }
      if (mode === 'release') gate.release();
      else if (mode === 'owner-close' || mode === 'queued-context-owner') await diagnosticDeadline(browser.close(), 'owner shutdown during confirmation');
      else gate.cut();
      const close = await diagnosticDeadline(closing, 'pending close settles after confirmation or owner retirement');
      assert.deepEqual(close, { ok: true });
      assert.deepEqual(atSettlement, { closed: true, notices: 1 }, 'Page close publication precedes successful settlement');
      await diagnosticDeadline(browser.close(), 'confirmation browser cleanup');
      const contextOutcome = contextClosing ? await diagnosticDeadline(contextClosing, 'queued Context close settlement') : undefined;
      if (browser.pwrightMetrics) {
        for (let i = 0; i < 100 && browser.pwrightFfiMetrics().pending; i++) await setImmediate();
        assert.equal(browser.pwrightMetrics().activeLeases, 0);
        assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
        assert.equal(browser.pwrightFfiMetrics().pending, 0);
        assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      }
      return { held, close, atSettlement, notices, contextNotices, contextOutcome, connected: browser.isConnected() };
    } finally {
      gate.release();
      if (closing) await closing;
      if (contextClosing) await contextClosing;
      try { if (browser.isConnected()) await context?.close(); }
      finally { await browser.close(); }
    }
  });
  const targets = await fetch(`${endpoint}/json/list`, { signal: AbortSignal.timeout(5000) }).then(response => response.json());
  assert.ok(targets.every(value => value.id !== gate.target), 'the exact owned Page is absent');
  return result;
}

for (const mode of ['release', 'owner-close', 'peer-loss', 'queued-context-owner', 'queued-context-peer']) test(`native close confirmation: ${mode}`, { timeout: 30000 }, async () => {
  // Arrange
  const expected = await exercise(oracle, mode);
  // Act
  const actual = await exercise(candidate, mode);
  // Assert
  assert.deepEqual(actual, expected);
  process.stdout.write(`# close confirmation ${mode} ${JSON.stringify(actual)}\n`);
});

test('Worker teardown retires an admitted close with its native receipt held', { timeout: 30000 }, async () => {
  // Arrange
  const controller = await oracle.connectOverCDP(endpoint, { timeout: 5000 });
  const cdp = await controller.newBrowserCDPSession();
  const gate = new RetirementGate();
  try {
    await withCdpCommandRelay(endpoint, gate, async relay => {
      const worker = new Worker(new URL('./native-close-confirmation-worker.mjs', import.meta.url), { workerData: { endpoint: relay } });
      try {
        const [ready] = await diagnosticDeadline(once(worker, 'message'), 'native confirmation Worker ready');
        assert.equal(ready, 'ready');
        await gate.held(true);
        assert.equal(gate.destruction, true);
        assert.equal(typeof gate.context, 'string');
        const before = (await cdp.send('Target.getBrowserContexts')).browserContextIds;
        assert.ok(before.includes(gate.context));
        // Act
        await diagnosticDeadline(worker.terminate(), 'Worker cleanup during native confirmation');
        // Assert
        const after = (await cdp.send('Target.getBrowserContexts')).browserContextIds;
        assert.equal(after.includes(gate.context), false, 'exact Worker-owned Context retired');
      } finally {
        await worker.terminate();
      }
    });
  } finally {
    await cdp.detach();
    await controller.close();
  }
});
