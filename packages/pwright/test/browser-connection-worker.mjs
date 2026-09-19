import assert from 'node:assert/strict';
import { once } from 'node:events';
import { parentPort, workerData } from 'node:worker_threads';
import { withCdpCommandRelay, WebSocket } from './cdp-command-relay.mjs';

// A real transport controller on a different JS thread. No synthetic native
// event or production API result is injected into the subject connection.
class Gate {
  client;
  upstream;
  error;
  command(raw, client, upstream) {
    this.client = client; this.upstream = upstream;
    upstream.send(raw, { binary: false });
  }
  response(raw, client) { client.send(raw, { binary: false }); }
  fail(error) { this.error ??= error; }
  check() { if (this.error) throw this.error; }
  release() {}
  async disconnect() {
    assert.equal(this.client.readyState, WebSocket.OPEN);
    const closed = Promise.all([once(this.client, 'close'), once(this.upstream, 'close')]);
    this.client.terminate();
    await closed;
  }
}
const state = new Int32Array(workerData.state);
try {
  await withCdpCommandRelay(workerData.endpoint, new Gate(), async (url, gate) => {
    parentPort.postMessage({ url });
    const [command] = await once(parentPort, 'message');
    assert.ok(command === 'drop' || command === 'keep');
    if (command === 'drop') await gate.disconnect();
    else assert.equal(gate.client.readyState, WebSocket.OPEN);
    gate.check();
    Atomics.store(state, 0, 1);
    Atomics.notify(state, 0);
    const [finish] = await once(parentPort, 'message');
    assert.equal(finish, 'finish');
  });
} catch (error) {
  Atomics.store(state, 0, -1);
  Atomics.notify(state, 0);
  throw error;
}
