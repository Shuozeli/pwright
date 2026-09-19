import assert from 'node:assert/strict';
import { CloseTrace } from './native-close-trace.mjs';

// Diagnostic-only source control. The parent trace records native arrival time;
// forwarding of only exact owned-Page retirement events is separately withheld.
export class HeldRetirementTrace extends CloseTrace {
  #held = [];
  holding = false;
  response(raw, client) {
    const value = JSON.parse(raw.toString());
    const retired = value.method === 'Target.targetDestroyed' ? value.params.targetId :
      value.method === 'Target.detachedFromTarget' ? value.params.targetId ?? this.sessions.get(value.params.sessionId) : undefined;
    if (this.holding && this.created.includes(retired)) {
      assert.ok(this.#held.length < 8);
      assert.ok(raw.length <= 4096);
      super.response(raw, { send: buffer => this.#held.push({ buffer, client }) });
    } else super.response(raw, client);
  }
  heldCount() { return this.#held.length; }
  release() {
    this.holding = false;
    for (const { buffer, client } of this.#held) {
      if (client.readyState === 1) client.send(buffer, { binary: false });
    }
    this.#held.length = 0;
  }
}
