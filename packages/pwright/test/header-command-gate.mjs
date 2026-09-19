import assert from 'node:assert/strict';
import { EventEmitter, once } from 'node:events';
import { WebSocket, withCdpCommandRelay } from './cdp-command-relay.mjs';
import { diagnosticDeadline } from '../scripts/probe-deadline.mjs';

const marker = 'X-Node089';
const fault = 'NODE089 injected header failure';
const MAX_HELD = 8;

class HeaderCommandGate extends EventEmitter {
  #armed = false;
  #failed = false;
  #pending = new Map();
  #held = [];
  #error;
  #primary = 0;
  #compensation = 0;
  #observed = new Map();
  #failPrimary = true;

  arm({ failPrimary = true } = {}) {
    assert.equal(this.#armed, false);
    this.#failPrimary = failPrimary;
    this.#armed = true;
  }
  command(raw, client, upstream) {
    const message = JSON.parse(raw.toString());
    const value = message.method === 'Network.setExtraHTTPHeaders'
      ? Object.entries(message.params.headers).find(([name]) => name.toLowerCase() === marker.toLowerCase())?.[1]
      : undefined;
    if (value !== undefined) {
      assert.ok(this.#observed.size < 32, 'bounded header receipt observations');
      this.#observed.set(message.id, value);
    }
    if (this.#armed && (value === 'proposal' || value === 'old')) {
      if (value === 'proposal') {
        this.#primary++;
        if (!this.#failed && this.#failPrimary) {
          this.#failed = true;
          this.#observed.delete(message.id);
          client.send(JSON.stringify({ id: message.id, sessionId: message.sessionId,
            error: { code: -32000, message: fault } }));
          return;
        }
      } else {
        assert.equal(this.#failed, true, 'compensation follows the primary failure');
        this.#compensation++;
      }
      assert.ok(this.#pending.size + this.#held.length < MAX_HELD, 'bounded held native receipts');
      this.#pending.set(message.id, value);
    }
    upstream.send(raw, { binary: false });
  }
  response(raw, client) {
    const message = JSON.parse(raw.toString());
    if (this.#observed.has(message.id)) {
      const value = this.#observed.get(message.id);
      this.#observed.delete(message.id);
      this.emit('receipt', { value, error: message.error });
    }
    if (this.#pending.has(message.id)) {
      assert.equal(message.error, undefined, 'Chrome must acknowledge the forwarded command successfully');
      this.#pending.delete(message.id);
      this.#held.push({ raw, client });
      this.emit('change');
    } else {
      client.send(raw, { binary: false });
    }
  }
  fail(error) {
    this.#error ??= error;
    this.emit('change');
  }
  async held(count) {
    await diagnosticDeadline((async () => {
      while (this.#held.length < count) {
        if (this.#error) throw this.#error;
        await once(this, 'change');
      }
    })(), `wait for ${count} held header receipts`);
    this.check();
    assert.equal(this.#held.length, count);
  }
  release() {
    this.#armed = false;
    for (const { raw, client } of this.#held.splice(0)) {
      if (client.readyState === WebSocket.OPEN) client.send(raw, { binary: false });
    }
    // Teardown can release a response that has not arrived yet by removing its
    // gate entry. The connection is still kept alive for Context cleanup.
    this.#pending.clear();
  }
  check() { if (this.#error) throw this.#error; }
  counts() {
    return { primary: this.#primary, compensation: this.#compensation, held: this.#held.length };
  }
  nextReceipt() {
    return diagnosticDeadline(once(this, 'receipt').then(([receipt]) => receipt), 'next header receipt');
  }
}

export async function withHeaderCommandGate(endpoint, action) {
  return withCdpCommandRelay(endpoint, new HeaderCommandGate(), action);
}

export { marker, fault };
