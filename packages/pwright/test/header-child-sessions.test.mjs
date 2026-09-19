import assert from 'node:assert/strict';
import { EventEmitter, once } from 'node:events';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { test, before, after } from 'node:test';
import { setImmediate } from 'node:timers/promises';
import { chromium as candidate } from '@shuozeli/pwright';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';
import { withCdpCommandRelay, WebSocket } from './cdp-command-relay.mjs';

const oracleRequire = createRequire(new URL('../../../tests/package.json', import.meta.url));
assert.equal(oracleRequire('playwright/package.json').version, '1.58.2');
const { chromium: oracle } = oracleRequire('playwright');
const endpoint = process.env.PWRIGHT_TEST_CDP;
const bind = process.env.PWRIGHT_TEST_BIND;
const base = process.env.PWRIGHT_TEST_BASE;
assert.equal(endpoint, 'http://127.0.0.1:9222');
assert.ok(bind, 'explicit Tailscale bind required');
assert.ok(base && new URL(base).hostname.endsWith('.ts.net'));
assert.ok(process.env.PWRIGHT_NATIVE_PROFILE?.includes('extra-headers'));
const missing = { code: -32001, message: 'Session with given id not found.' };
const fault = { code: -32000, message: 'NODE091 child header failure' };
let url;
const server = createServer({ maxHeaderSize: 8192 }, (request, response) => {
  const path = new URL(request.url, url);
  if (path.pathname === '/echo') {
    response.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store',
      'access-control-allow-origin': '*', 'access-control-allow-headers': '*',
      'access-control-allow-methods': 'GET, OPTIONS', 'access-control-allow-private-network': 'true' });
    response.end(JSON.stringify({ layer: request.headers['x-layer'] ?? null,
      context: request.headers['x-context'] ?? null }));
  } else {
    response.writeHead(200, { 'content-type': 'text/html', 'cache-control': 'no-store' });
    response.end(path.pathname === '/parent'
      ? '<title>header parent</title><iframe sandbox="allow-scripts" src="/child"></iframe>'
      : '<title>header child</title>');
  }
});
server.maxConnections = 8;
server.requestTimeout = 5000;
before(async () => {
  server.listen(0, bind);
  await once(server, 'listening');
  url = new URL('/parent', base);
  url.port = String(server.address().port);
  process.stdout.write(`# child header fixture ${url.href}\n`);
});
after(async () => {
  const closed = once(server, 'close');
  server.close();
  server.closeAllConnections();
  await closed;
});

// Transport timing/fault injection only, never an implementation of headers.
class ChildHeaderGate extends EventEmitter {
  #session;
  #mainSession;
  #armed = false;
  #pending;
  #released = false;
  #receipt;
  #detached = false;
  #error;
  #injected;
  #commands = new Map();
  #attachments = new Map();
  #iframeSessions = new Set();
  #childTarget;
  #events = [];
  #navigation;
  #navigationStartedAt;
  beginNavigation() { this.#navigationStartedAt = performance.now(); }
  #diagnostics = 0;
  observeChild(session, committedUrl) {
    if (committedUrl !== new URL('/child', url).href || this.#armed || this.#pending) return;
    const attachment = this.#attachments.get(session);
    if (attachment?.type !== 'iframe' || !this.#iframeSessions.has(session) ||
        attachment.parent !== this.#navigation?.session) return;
    this.#childTarget ??= attachment.id;
    assert.equal(attachment.id, this.#childTarget, 'one initial native child target');
    if (this.#session !== undefined && this.#session !== session)
      assert.equal(this.#iframeSessions.has(this.#session), false, 'old child session retired before replacement');
    this.#session = session;
    this.#detached = false;
    this.emit('change');
  }
  diagnose(phase, error, browser) {
    if (this.#diagnostics++ >= 8) return;
    const now = performance.now();
    process.stdout.write(`# child-header failure snapshot ${JSON.stringify({
      phase, error: String(error).slice(0, 512), observedAt: now,
      gateError: this.#error && String(this.#error).slice(0, 512),
      navigationStartedAt: this.#navigationStartedAt,
      heldCommand: this.#pending?.id, released: this.#released,
      unansweredWireCommands: [...this.#commands].map(([id, { started, ...command }]) => ({
        id, ...command, elapsedMs: Math.round(now - started),
      })),
      attachments: [...this.#attachments], events: this.#events, navigation: this.#navigation,
      host: browser?.pwrightMetrics?.(), ffi: browser?.pwrightFfiMetrics?.(),
    })}\n`);
  }
  arm(injected, main = false) {
    assert.ok(this.#session, 'native iframe attachment proves an OOPIF, not just a Frame wrapper');
    if (main) {
      assert.ok(this.#mainSession, 'owning main-frame session was observed');
      this.#session = this.#mainSession;
    }
    this.#armed = true;
    this.#injected = injected;
  }
  command(raw, client, upstream) {
    const command = JSON.parse(raw.toString());
    assert.ok(this.#commands.size < 128, 'bounded diagnostic command inventory');
    const detachedSession = command.method === 'Target.detachFromTarget' ? command.params.sessionId : undefined;
    this.#commands.set(command.id, { started: performance.now(), method: command.method, session: command.sessionId,
      detachedSession, target: this.#attachments.get(detachedSession ?? command.sessionId) });
    if (command.method === 'Page.navigate' && command.params.url === url.href) {
      assert.equal(this.#navigation, undefined, 'one initial fixture navigation');
      this.#navigation = { id: command.id, session: command.sessionId,
        target: this.#attachments.get(command.sessionId)?.id, sentAt: performance.now(), events: [], commands: [] };
    }
    const attachment = this.#attachments.get(detachedSession ?? command.sessionId);
    const parent = this.#attachments.get(attachment?.parent);
    if (this.#navigation && (command.sessionId === this.#navigation.session ||
        (this.#navigation.target !== undefined &&
          (attachment?.id === this.#navigation.target || parent?.id === this.#navigation.target)))) {
      if (this.#navigation.commands.length === 128) this.#navigation.commands.shift();
      this.#navigation.commands.push({ id: command.id, ...this.#commands.get(command.id) });
    }
    if (this.#armed && command.sessionId === this.#session &&
        command.method === 'Network.setExtraHTTPHeaders' &&
        Object.entries(command.params.headers).some(([name, value]) => name.toLowerCase() === 'x-layer' && value === 'proposal')) {
      assert.equal(this.#pending, undefined, 'one test-owned child header command');
      this.#pending = { raw, client, upstream, id: command.id, sessionId: command.sessionId };
      this.#armed = false;
      this.emit('change');
    } else upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    if (['Target.attachedToTarget', 'Target.detachedFromTarget', 'Page.frameNavigated',
      'Page.lifecycleEvent', 'Page.frameStartedLoading', 'Page.frameStoppedLoading', 'Page.frameDetached',
      'Inspector.detached', 'Inspector.targetCrashed', 'Target.targetCrashed', 'Target.targetDestroyed',
      'Network.requestWillBeSent', 'Network.responseReceived', 'Network.loadingFinished',
      'Network.loadingFailed'].includes(message.method)) {
      if (this.#events.length === 64) this.#events.shift();
      const event = { method: message.method, session: message.sessionId, receivedAt: performance.now(),
        childSession: message.params.sessionId, frame: message.params.frameId ?? message.params.frame?.id,
        target: message.params.targetInfo?.targetId ?? message.params.targetId,
        reason: message.params.reason?.slice(0, 256),
        name: message.params.name, waitingForDebugger: message.params.waitingForDebugger,
        loader: message.params.loaderId ?? message.params.frame?.loaderId,
        request: message.params.requestId, resourceType: message.params.type,
        status: message.params.response?.status, error: message.params.errorText?.slice(0, 256),
        url: (message.params.frame?.url ?? message.params.request?.url ?? message.params.response?.url ??
          message.params.targetInfo?.url)?.slice(0, 256) };
      this.#events.push(event);
      const attachment = this.#attachments.get(message.sessionId);
      const parent = this.#attachments.get(attachment?.parent);
      if (this.#navigation && (message.sessionId === this.#navigation.session ||
          (this.#navigation.target !== undefined &&
            (attachment?.id === this.#navigation.target || parent?.id === this.#navigation.target ||
              (event.target !== undefined && (event.target === this.#navigation.target ||
                this.#navigation.events.some(previous => previous.target === event.target))))))) {
        if (this.#navigation.events.length === 64) this.#navigation.events.shift();
        this.#navigation.events.push(event);
      }
    }
    if (message.id !== undefined && message.id === this.#navigation?.id)
      this.#navigation.receipt = { frame: message.result?.frameId, loader: message.result?.loaderId,
        error: message.error, errorText: message.result?.errorText?.slice(0, 512) };
    if (message.method === 'Target.attachedToTarget') {
      assert.ok(this.#attachments.size < 128, 'bounded active native attachment inventory');
      const info = message.params.targetInfo;
      this.#attachments.set(message.params.sessionId, { id: info.targetId,
        type: info.type, subtype: info.subtype, context: info.browserContextId, parent: message.sessionId });
    }
    if (message.method === 'Target.detachedFromTarget') this.#attachments.delete(message.params.sessionId);
    if (message.id !== undefined) {
      const command = this.#commands.get(message.id);
      this.#commands.delete(message.id);
      const navigationCommand = this.#navigation?.commands.find(command => command.id === message.id);
      if (navigationCommand) {
        navigationCommand.completedAt = performance.now();
        navigationCommand.error = message.error && { code: message.error.code,
          message: message.error.message?.slice(0, 256) };
      }
      if (message.error) process.stdout.write(`# native child-header diagnostic ${JSON.stringify({
        ...command, elapsedMs: command && Math.round(performance.now() - command.started), error: message.error,
      })}\n`);
    }
    if (message.method === 'Page.frameNavigated' && !message.params.frame.parentId &&
        message.params.frame.url === url.href) this.#mainSession = message.sessionId;
    if (message.method === 'Target.attachedToTarget' && message.params.targetInfo.type === 'iframe') {
      assert.ok(this.#iframeSessions.size < 64, 'bounded native iframe inventory');
      this.#iframeSessions.add(message.params.sessionId);
      // An already committed OOPIF can reattach without another frameNavigated.
      // The native target snapshot is also authoritative URL/identity evidence.
      this.observeChild(message.params.sessionId, message.params.targetInfo.url);
    }
    // Chromium announces a paused OOPIF with an empty initial URL. Correlate
    // its committed URL through the same native session, not a guessed target.
    if (message.method === 'Page.frameNavigated')
      this.observeChild(message.sessionId, message.params.frame.url);
    if (message.method === 'Target.detachedFromTarget') this.#iframeSessions.delete(message.params.sessionId);
    if (message.method === 'Target.detachedFromTarget' && message.params.sessionId === this.#session) {
      this.#detached = true;
      this.emit('change');
    }
    if (this.#pending && message.id === this.#pending.id) {
      this.#receipt = message.error;
      this.emit('change');
    }
    client.send(raw, { binary: false });
  }
  async waitFor(predicate, label) {
    await diagnosticDeadline((async () => {
      while (!predicate()) { this.check(); await once(this, 'change'); }
      this.check();
    })(), label);
  }
  attached() { return this.waitFor(() => this.#session !== undefined, 'native iframe attachment'); }
  held() { return this.waitFor(() => this.#pending !== undefined, 'held child header command'); }
  detached() { return this.waitFor(() => this.#detached, 'native iframe detach'); }
  async missingReceipt() {
    await this.waitFor(() => this.#receipt !== undefined, 'native child missing-session receipt');
    assert.deepEqual(this.#receipt, missing);
  }
  release() {
    this.#armed = false;
    if (!this.#released && this.#pending) {
      this.#released = true;
      const { raw, client, upstream, id, sessionId } = this.#pending;
      if (this.#injected) {
        this.#commands.delete(id);
        if (client.readyState === WebSocket.OPEN)
          client.send(JSON.stringify({ id, sessionId, error: this.#injected }));
      } else if (upstream.readyState === WebSocket.OPEN) upstream.send(raw, { binary: false });
    }
  }
  fail(error) { this.#error ??= error; this.emit('change'); }
  check() { if (this.#error) throw this.#error; }
}

async function wire(frame) {
  return frame.evaluate(async () => (await fetch('/echo')).json());
}
async function exercise(type, scope, injected, main = false) {
  return withCdpCommandRelay(endpoint, new ChildHeaderGate(), async (relay, gate) => {
    process.stdout.write(`# child-header implementation ${type === candidate ? 'candidate' : 'oracle'}\n`);
    let browser;
    try { browser = await type.connectOverCDP(relay, { timeout: 5000 }); }
    catch (error) { gate.diagnose('connect', error); throw error; }
    let context;
    let pending;
    let phase = 'new Context';
    try {
      context = await browser.newContext();
      phase = 'initial Context headers';
      await context.setExtraHTTPHeaders({ 'X-Context': 'inherited' });
      phase = 'new Page';
      const page = await context.newPage();
      phase = 'fixture navigation';
      gate.beginNavigation();
      await page.goto(url.href, { timeout: 5000 });
      phase = 'native child attachment';
      await gate.attached();
      phase = 'header exercise';
      const child = page.frames().find(frame => frame.url() === new URL('/child', url).href);
      assert.ok(child);
      const owner = scope === 'Page' ? page : context;
      await owner.setExtraHTTPHeaders({ 'X-Layer': 'old', 'X-Context': 'inherited' });
      phase = 'initial child wire read';
      assert.deepEqual(await wire(child), { layer: 'old', context: 'inherited' });
      phase = 'held header setter';
      gate.arm(injected, main);
      // Act: hold the child's native command, then close the child or inject an
      // ordinary protocol error while it is still alive (including a lookalike).
      pending = owner.setExtraHTTPHeaders({ 'X-Layer': 'proposal', 'X-Context': 'inherited' })
        .then(() => ({ status: 'fulfilled' }), error => ({ status: 'rejected',
          error: error.message.split('\n')[0] }));
      await gate.held();
      if (main) {
        await page.close();
        await gate.detached();
        const result = await diagnosticDeadline(pending, 'setter rejects on main close');
        assert.equal(result.status, 'rejected');
        gate.release();
        await gate.missingReceipt();
        const fresh = await context.newPage();
        await fresh.goto(new URL('/echo', url).href, { timeout: 5000 });
        return { result, stillPublished: context.pages().includes(page), fresh: await wire(fresh) };
      }
      if (!injected) {
        phase = 'remove child';
        await page.evaluate(() => document.querySelector('iframe').remove());
        await gate.detached();
        // Closure settles the public operation without waiting for the held
        // command to reach Chrome or receive a native error response.
        assert.deepEqual(await diagnosticDeadline(pending, 'setter settles on child close'), { status: 'fulfilled' });
      }
      gate.release();
      if (!injected) await gate.missingReceipt();
      const result = await diagnosticDeadline(pending, 'child header setter');
      phase = 'parent wire after setter';
      const parent = await wire(page);
      phase = 'healthy header setter';
      await owner.setExtraHTTPHeaders({ 'X-Layer': 'healthy', 'X-Context': 'inherited' });
      let replacement = null;
      if (!injected) {
        phase = 'replacement iframe load';
        await page.evaluate(() => new Promise(resolve => {
          const frame = document.createElement('iframe');
          frame.setAttribute('sandbox', 'allow-scripts');
          frame.src = '/child?replacement=1';
          frame.onload = () => resolve();
          document.body.appendChild(frame);
        }));
        const fresh = page.frames().find(frame => frame.url() === new URL('/child?replacement=1', url).href);
        assert.ok(fresh, 'a new child is published after native session retirement');
        phase = 'replacement child wire read';
        replacement = await wire(fresh);
      }
      phase = 'final wire reads';
      return { result, parent, recovered: await wire(page),
        child: injected ? await wire(child) : null, replacement };
    } catch (error) {
      gate.diagnose(phase, error, browser);
      throw error;
    } finally {
      gate.release();
      if (pending) await diagnosticDeadline(pending, 'drain child header setter');
      try { await diagnosticDeadline(context?.close(), 'owned Context cleanup'); }
      catch (error) { gate.diagnose('Context cleanup', error, browser); throw error; }
      finally {
        try { await diagnosticDeadline(browser.close(), 'Browser disconnect'); }
        catch (error) { gate.diagnose('Browser disconnect', error, browser); throw error; }
      }
      if (browser.pwrightMetrics) {
        assert.equal(browser.pwrightMetrics().activeLeases, 0);
        assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
        for (let turn = 0; turn < 100 && browser.pwrightFfiMetrics().pending; turn++) await setImmediate();
        assert.equal(browser.pwrightFfiMetrics().pending, 0);
        assert.equal(browser.pwrightFfiMetrics().reservedBytes, 0);
      }
    }
  });
}

for (const scope of ['Page', 'Context']) {
  test(`${scope} header fanout rejects when its main session closes`, { timeout: 30000 }, async () => {
    // Arrange
    const expected = await exercise(oracle, scope, undefined, true);
    // Act
    const actual = await exercise(candidate, scope, undefined, true);
    // Assert
    assert.deepEqual(actual, expected);
  });
  for (const [name, injected] of [['native child closes', undefined], ['live child protocol failure', fault],
    ['live child missing-session lookalike', missing]]) {
    test(`${scope} header fanout: ${name}`, { timeout: 30000 }, async () => {
      // Arrange
      const expected = await exercise(oracle, scope, injected);
      assert.equal(expected.result.status, injected ? 'rejected' : 'fulfilled');
      // Act
      const actual = await exercise(candidate, scope, injected);
      // Assert
      assert.deepEqual(actual, expected);
    });
  }
}
