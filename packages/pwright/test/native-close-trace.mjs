import assert from 'node:assert/strict';

// Reusable bounded observer, not a browser controller. The caller supplies time.
export class CloseTrace {
  #pending = new Map();
  #error;
  #now;
  #started;
  #track;
  created = [];
  contexts = [];
  receipts = [];
  completed = [];
  sessions = new Map();
  lifecycle = [];
  navigation = [];
  constructor(now, track = () => true) { this.#now = now; this.#started = now(); this.#track = track; }
  #time() { return this.#now() - this.#started; }
  #identity(value) {
    assert.ok(value === undefined || (typeof value === 'string' && value.length <= 128));
    return value;
  }
  #errorText(value) {
    assert.ok(value === undefined || (typeof value === 'string' && value.length <= 4096));
    return value;
  }
  #attached(session, target) {
    assert.ok(session && target, 'attachment must contain exact session and target identities');
    assert.ok(this.sessions.has(session) || this.sessions.size < 32);
    this.sessions.set(this.#identity(session), this.#identity(target));
  }
  command(raw, _client, upstream) {
    const message = JSON.parse(raw.toString());
    assert.equal(typeof message.method, 'string');
    if (!this.#track(message.method)) { upstream.send(raw, { binary: false }); return; }
    assert.ok(this.#pending.size < 32);
    this.#pending.set(message.id, { id: message.id, method: this.#identity(message.method),
      target: this.#identity(message.params?.targetId), context: this.#identity(message.params?.browserContextId),
      session: this.#identity(message.sessionId), atMs: this.#time() });
    upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    const command = this.#pending.get(message.id);
    this.#pending.delete(message.id);
    if (command?.method === 'Target.createBrowserContext' && message.result?.browserContextId) {
      assert.ok(this.contexts.length < 8);
      this.contexts.push(this.#identity(message.result.browserContextId));
    }
    if (command?.method === 'Target.createTarget' && message.result?.targetId) {
      assert.ok(this.created.length < 8);
      this.created.push(this.#identity(message.result.targetId));
    }
    if (message.method === 'Target.attachedToTarget') {
      this.#attached(message.params.sessionId, message.params.targetInfo.targetId);
    }
    if (command?.method === 'Target.attachToTarget' && message.result?.sessionId)
      this.#attached(message.result.sessionId, command.target);
    if (this.created.includes(this.sessions.get(message.sessionId)) &&
      ['Page.frameNavigated', 'Page.lifecycleEvent', 'Network.loadingFailed'].includes(message.method)) {
      assert.ok(this.navigation.length < 64, 'bounded owned-Page navigation close trace');
      this.navigation.push({ method: message.method, atMs: this.#time(),
        session: this.#identity(message.sessionId),
        frame: this.#identity(message.params.frameId ?? message.params.frame?.id),
        loader: this.#identity(message.params.loaderId ?? message.params.frame?.loaderId),
        request: this.#identity(message.params.requestId),
        name: this.#identity(message.params.name), type: this.#identity(message.params.type),
        errorText: this.#errorText(message.params.errorText), canceled: message.params.canceled });
    }
    if (['Target.detachedFromTarget', 'Target.targetDestroyed'].includes(message.method)) {
      assert.ok(this.lifecycle.length < 64);
      this.lifecycle.push({ method: message.method, atMs: this.#time(),
        session: this.#identity(message.params.sessionId), target: this.#identity(message.params.targetId),
        unanswered: [...this.#pending.values()].filter(value => value.session !== undefined &&
          (message.method === 'Target.detachedFromTarget' ? value.session === message.params.sessionId :
            this.sessions.get(value.session) === message.params.targetId)).map(value => value.id) });
    }
    if (command) {
      const receipt = { ...command, elapsedMs: this.#time() - command.atMs, success: message.result?.success,
        errorCode: message.error?.code,
        navigationError: command.method === 'Page.navigate' ? this.#errorText(message.result?.errorText) : undefined,
        navigationLoader: command.method === 'Page.navigate' ? this.#identity(message.result?.loaderId) : undefined,
        ownedStillPresent: message.result?.targetInfos?.filter(value => this.created.includes(value.targetId)).map(value => value.targetId) };
      if (this.completed.length === 128) this.completed.shift();
      this.completed.push(receipt);
      if (['Target.closeTarget', 'Target.getTargets', 'Target.disposeBrowserContext'].includes(command.method)) {
        assert.ok(this.receipts.length < 128);
        this.receipts.push(receipt);
      }
    }
    client.send(raw, { binary: false });
  }
  fail(error) { this.#error ??= error; }
  check() { if (this.#error) throw this.#error; }
  release() {}
  unanswered() { return [...this.#pending.values()]; }
  report() {
    return { created: this.created, contexts: this.contexts, receipts: this.receipts, recentCompleted: this.completed,
      sessions: Object.fromEntries(this.sessions), lifecycle: this.lifecycle, navigation: this.navigation, unanswered: this.unanswered() };
  }
}
