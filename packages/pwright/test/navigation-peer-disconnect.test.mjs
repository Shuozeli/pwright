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
assert.ok(bind);
assert.ok(base && new URL(base).hostname.endsWith('.ts.net'));
let url;
const server = createServer({ maxHeaderSize: 8192 }, (request, response) => {
  response.writeHead(200, { 'content-type': 'text/html', 'cache-control': 'no-store' });
  response.end(request.url === '/parent'
    ? '<title>peer parent</title><iframe sandbox="allow-scripts" src="/child"></iframe>'
    : '<title>peer child</title>');
});
server.maxConnections = 8;
server.requestTimeout = 5000;
before(async () => {
  server.listen(0, bind);
  await once(server, 'listening');
  url = new URL('/parent', base);
  url.port = String(server.address().port);
  process.stdout.write(`# peer disconnect fixture ${url.href}\n`);
});
after(async () => {
  const closed = once(server, 'close');
  server.close();
  server.closeAllConnections();
  await closed;
});

// Test transport only: hold one genuine child resume command, then disconnect
// this borrowing client. Never close the owner's Context, Page or browser.
class PeerResumeGate extends EventEmitter {
  #marker;
  #page;
  #child;
  #held;
  #error;
  constructor(marker) { super(); this.#marker = marker; }
  command(raw, _client, upstream) {
    const message = JSON.parse(raw.toString());
    if (this.#child !== undefined && message.sessionId === this.#child &&
        message.method === 'Runtime.runIfWaitingForDebugger') {
      assert.equal(this.#held, undefined, 'one held resume command');
      this.#held = message.id;
      this.emit('change');
    } else upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    if (message.method === 'Target.attachedToTarget') {
      if (message.params.targetInfo.url === this.#marker) {
        assert.equal(this.#page, undefined, 'one borrowed Page attachment');
        this.#page = message.params.sessionId;
      }
      if (this.#page !== undefined && message.sessionId === this.#page &&
          message.params.targetInfo.type === 'iframe') {
        assert.equal(this.#child, undefined, 'one owned fixture child');
        assert.equal(message.params.waitingForDebugger, true);
        this.#child = message.params.sessionId;
      }
    }
    client.send(raw, { binary: false });
  }
  async held() {
    await diagnosticDeadline((async () => {
      while (this.#held === undefined) { this.check(); await once(this, 'change'); }
      this.check();
    })(), 'borrowing peer child resume');
  }
  release() {} // The discarded command belongs only to the disconnected peer.
  fail(error) { this.#error ??= error; this.emit('change'); }
  check() { if (this.#error) throw this.#error; }
}

async function closeBrowser(browser) {
  if (!browser) return;
  await diagnosticDeadline(browser.close(), 'Browser disconnect');
  if (browser.pwrightMetrics) {
    assert.equal(browser.pwrightMetrics().activeLeases, 0);
    assert.equal(browser.pwrightMetrics().cleanupInFlight, 0);
  }
}

// One exact owner child resume is held. The peer only observes; its Page-level
// auto-attachment may retire that child before any peer child session exists.
class MidAttachmentGate extends EventEmitter {
  #page;
  #child;
  #expectedChild;
  #method;
  #held;
  #released = false;
  #error;
  #trace = [];
  #attachments = [];
  childRetired = false;
  constructor(method, expectedChild) {
    super(); this.#method = method; this.#expectedChild = expectedChild;
  }
  record(value) {
    if (this.#trace.length === 128) this.#trace.shift();
    this.#trace.push({ at: performance.now(), ...value });
  }
  command(raw, _client, upstream) {
    const message = JSON.parse(raw.toString());
    if (message.method === 'Page.navigate' && message.params.url === url.href)
      this.#page = message.sessionId;
    this.record({ direction: 'command', id: message.id, method: message.method,
      session: message.sessionId, child: message.params?.sessionId });
    if (!this.#released && this.#child !== undefined && message.sessionId === this.#child &&
        message.method === this.#method) {
      assert.equal(this.#held, undefined, 'one held child initialization command');
      assert.ok(raw.length <= 1024, 'bounded held initialization command');
      this.#held = { raw, upstream };
      this.emit('change');
    } else upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    this.record({ direction: 'native', id: message.id, method: message.method,
      session: message.sessionId, child: message.params?.sessionId,
      target: message.params?.targetInfo?.targetId, waiting: message.params?.waitingForDebugger,
      type: message.params?.targetInfo?.type,
      url: (message.params?.targetInfo?.url ?? message.params?.frame?.url)?.slice(0, 256),
      reason: message.params?.reason?.slice(0, 256), error: message.error && {
        code: message.error.code, message: message.error.message?.slice(0, 256) } });
    if (message.method === 'Target.attachedToTarget') {
      assert.ok(this.#attachments.length < 64, 'bounded native attachment evidence');
      this.#attachments.push({ at: performance.now(), session: message.sessionId,
        child: message.params.sessionId, target: message.params.targetInfo.targetId,
        type: message.params.targetInfo.type, url: message.params.targetInfo.url.slice(0, 256) });
      if (this.#expectedChild !== undefined && message.params.targetInfo.url === url.href)
        this.#page = message.params.sessionId;
      if (this.#page !== undefined && message.sessionId === this.#page &&
          message.params.targetInfo.type === 'iframe') {
        if (this.#expectedChild !== undefined)
          assert.equal(message.params.targetInfo.targetId, this.#expectedChild, 'same native child, not another Page');
        if (this.#child === undefined) {
          if (this.#expectedChild === undefined) assert.equal(message.params.waitingForDebugger, true);
          this.#child = message.params.sessionId;
          this.childTarget = message.params.targetInfo.targetId;
        }
      }
    }
    if (message.method === 'Target.detachedFromTarget' && message.params.sessionId === this.#child)
      this.childRetired = true;
    client.send(raw, { binary: false });
  }
  async held() {
    await diagnosticDeadline((async () => {
      while (!this.#held) { this.check(); await once(this, 'change'); }
      this.check();
    })(), `held ${this.#method} during peer attachment`);
  }
  release() {
    if (this.#released) return;
    this.#released = true;
    if (this.#held?.upstream.readyState === WebSocket.OPEN)
      this.#held.upstream.send(this.#held.raw, { binary: false });
    this.#held = undefined;
  }
  diagnose(side, error) {
    process.stdout.write(`# mid-attachment ${JSON.stringify({ side, error: String(error).slice(0, 512),
      gateError: this.#error && String(this.#error).slice(0, 512), attachments: this.#attachments,
      trace: this.#trace })}\n`);
  }
  fail(error) { this.#error ??= error; this.emit('change'); }
  check() { if (this.#error) throw this.#error; }
}

function rawPeer(method, pageScoped = false) {
  return { async connectOverCDP(relay) {
    const socket = new WebSocket(relay);
    let socketFailure;
    socket.on('error', error => { socketFailure ??= error; });
    try {
      await diagnosticDeadline(once(socket, 'open'), 'raw peer transport');
      let nextId = 1;
      let pageSession;
      const attachment = raw => {
        const message = JSON.parse(raw.toString());
        if (message.method === 'Target.attachedToTarget' && message.params.targetInfo.url === url.href) {
          pageSession = message.params.sessionId;
        }
      };
      socket.on('message', attachment);
      async function send(method, params, sessionId) {
        if (socketFailure) throw socketFailure;
        assert.ok(nextId <= 2, 'at most two serial native control commands');
        const id = nextId++;
        let handler;
        let failed;
        let closed;
        const receipt = new Promise((resolve, reject) => {
          handler = raw => {
            const message = JSON.parse(raw.toString());
            if (message.id !== id) return;
            socket.off('message', handler);
            if (message.error) reject(new Error(JSON.stringify(message.error)));
            else resolve(message.result);
          };
          failed = reject;
          closed = () => reject(new Error('Raw control connection closed before receipt'));
          socket.on('message', handler);
          socket.once('error', failed);
          socket.once('close', closed);
        });
        socket.send(JSON.stringify({ id, method, params, sessionId }));
        try { return await diagnosticDeadline(receipt, 'raw peer command receipt'); }
        finally {
          socket.off('message', handler);
          socket.off('error', failed);
          socket.off('close', closed);
        }
      }
      const autoAttach = { autoAttach: true, waitForDebuggerOnStart: true, flatten: true };
      if (pageScoped) {
        await send('Target.setAutoAttach', autoAttach);
        assert.ok(pageSession, 'raw borrowing session for exact fixture URL');
      }
      if (method) {
        const params = method === 'Target.setAutoAttach' ? autoAttach : {};
        await send(method, params, pageScoped ? pageSession : undefined);
      }
      socket.off('message', attachment);
      return { async close() {
        if (socketFailure) { socket.terminate(); throw socketFailure; }
        if (socket.readyState === WebSocket.CLOSED) return;
        const closed = once(socket, 'close');
        socket.close();
        await diagnosticDeadline(closed, 'raw peer disconnect');
      } };
    } catch (error) { socket.terminate(); throw error; }
  } };
}

for (const [ownerName, ownerType] of [['oracle', oracle], ['candidate', candidate]]) {
  for (const [peerName, peerType] of [['oracle', oracle], ['candidate', candidate]]) {
    test(`${ownerName} navigation survives ${peerName} peer disconnect before child resume`,
      { timeout: 30000 }, async () => {
        // Arrange
        const browser = await ownerType.connectOverCDP(endpoint, { timeout: 5000 });
        let context;
        let ffiBrowser = browser.pwrightFfiMetrics ? browser : undefined;
        try {
          context = await browser.newContext();
          const page = await context.newPage();
          const marker = `data:text/html,<title>node096-${ownerName}-${peerName}</title>`;
          await page.goto(marker, { timeout: 5000 });
          await withCdpCommandRelay(endpoint, new PeerResumeGate(marker), async (relay, gate) => {
            let peer;
            let navigating;
            try {
              peer = await peerType.connectOverCDP(relay, { timeout: 5000 });
              if (peer.pwrightFfiMetrics) ffiBrowser = peer;
              navigating = page.goto(url.href, { timeout: 5000 }).then(
                response => ({ response }), error => ({ error }));
              await gate.held();
              // Act
              await closeBrowser(peer);
              peer = undefined;
              // Assert
              const result = await diagnosticDeadline(navigating, 'owner navigation after peer disconnect');
              assert.equal(result.error, undefined);
              assert.equal(result.response.status(), 200);
              assert.equal(await page.evaluate(() => document.title), 'peer parent');
              const child = page.frames().find(frame => frame.url() === new URL('/child', url).href);
              assert.ok(child, 'live child is still published by its owner');
              assert.equal(await child.evaluate(() => document.title), 'peer child');
            } finally {
              await closeBrowser(peer);
              if (navigating) await diagnosticDeadline(navigating, 'navigation cleanup');
            }
          });
        } finally {
          try { await diagnosticDeadline(context?.close(), 'owned Context cleanup'); }
          finally {
            await closeBrowser(browser);
            // FFI admission belongs to the Node environment, not one Browser.
            // Only assert zero after BOTH connections and navigation are drained.
            if (ffiBrowser) {
              for (let turn = 0; turn < 100 && ffiBrowser.pwrightFfiMetrics().pending; turn++) await setImmediate();
              assert.equal(ffiBrowser.pwrightFfiMetrics().pending, 0);
              assert.equal(ffiBrowser.pwrightFfiMetrics().reservedBytes, 0);
            }
          }
        }
      });
  }
}

async function observeMidAttachment(ownerType, peerType, peerName) {
  const ownerGate = new MidAttachmentGate('Runtime.runIfWaitingForDebugger');
  return withCdpCommandRelay(endpoint, ownerGate, async ownerRelay => {
    const owner = await ownerType.connectOverCDP(ownerRelay, { timeout: 5000 });
    let context;
    let navigating;
    let ffiBrowser = owner.pwrightFfiMetrics ? owner : undefined;
    try {
      context = await owner.newContext();
      const page = await context.newPage();
      navigating = page.goto(url.href, { timeout: 5000 }).then(
        response => ({ response }), error => ({ error }));
      await ownerGate.held();
      const peerGate = new MidAttachmentGate(undefined, ownerGate.childTarget);
      return await withCdpCommandRelay(endpoint, peerGate, async peerRelay => {
        let peer;
        try {
          // The actual second connection is the operation under comparison.
          peer = await peerType.connectOverCDP(peerRelay, { timeout: 5000 });
          if (peer.pwrightFfiMetrics) ffiBrowser = peer;
          ownerGate.release();
          const result = await diagnosticDeadline(navigating, 'owner navigation completion');
          let navigation;
          if (result.error) {
            assert.equal(result.error.name, 'TimeoutError');
            assert.match(result.error.message, /^page\.goto: Timeout 5000ms exceeded\.?(?:\n|$)/);
            navigation = { status: 'rejected', name: result.error.name, timeout: 5000 };
          } else {
            assert.equal(result.response.status(), 200);
            navigation = { status: 'fulfilled', httpStatus: result.response.status() };
          }
          const initialChildSessionRetired = ownerGate.childRetired;
          await closeBrowser(peer);
          peer = undefined;
          const mainTitle = await page.evaluate(() => document.title);
          assert.equal(mainTitle, 'peer parent');
          // An explicit SECOND navigation tests recovery; never retry or erase
          // the first result inside production code or this observation.
          const recovery = await page.goto(url.href, { timeout: 5000 });
          assert.equal(recovery.status(), 200);
          const child = page.frames().find(frame => frame.url() === new URL('/child', url).href);
          assert.ok(child, 'recovery publishes a usable child');
          const childTitle = await child.evaluate(() => document.title);
          assert.equal(childTitle, 'peer child');
          return { navigation, initialChildSessionRetired, mainTitle,
            recoveryStatus: recovery.status(), childTitle };
        } catch (error) {
          peerGate.diagnose(peerName, error);
          throw error;
        } finally {
          ownerGate.release();
          await closeBrowser(peer);
        }
      });
    } catch (error) {
      ownerGate.diagnose(ownerType === oracle ? 'oracle' : 'candidate', error);
      throw error;
    } finally {
      ownerGate.release();
      if (navigating) await diagnosticDeadline(navigating, 'owner navigation cleanup');
      try { await diagnosticDeadline(context?.close(), 'owned Context cleanup'); }
      finally {
        await closeBrowser(owner);
        if (ffiBrowser) {
          for (let turn = 0; turn < 100 && ffiBrowser.pwrightFfiMetrics().pending; turn++) await setImmediate();
          assert.equal(ffiBrowser.pwrightFfiMetrics().pending, 0);
          assert.equal(ffiBrowser.pwrightFfiMetrics().reservedBytes, 0);
        }
      }
    }
  });
}

for (const [peerName, peerType] of [['oracle', oracle], ['candidate', candidate],
  ['raw-open', rawPeer()], ['raw-version', rawPeer('Browser.getVersion')],
  ['raw-auto-attach', rawPeer('Target.setAutoAttach')],
  ['raw-page-enable', rawPeer('Page.enable', true)],
  ['raw-page-auto-attach', rawPeer('Target.setAutoAttach', true)]]) {
  test(`mid-navigation peer attachment and recovery: ${peerName}`, { timeout: 30000 }, async () => {
    // Arrange: characterize the actual pinned oracle, including native failure.
    const expected = await observeMidAttachment(oracle, peerType, peerName);
    // Act
    const actual = await observeMidAttachment(candidate, peerType, peerName);
    // Assert
    assert.deepEqual(actual, expected);
    process.stdout.write(`# peer attachment comparison ${JSON.stringify({ peer: peerName, expected, actual })}\n`);
  });
}
