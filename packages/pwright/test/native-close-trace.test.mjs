import assert from 'node:assert/strict';
import { test } from 'node:test';
import { CloseTrace } from './native-close-trace.mjs';
import { HeldRetirementTrace } from './held-retirement-trace.mjs';
import { listenerEndpoint } from '../scripts/listener-endpoint.mjs';

function fixture() {
  const upstream = [], downstream = [];
  const trace = new CloseTrace(() => 10);
  return { trace, upstream, downstream,
    command: value => trace.command(Buffer.from(JSON.stringify(value)), undefined, { send: raw => upstream.push(raw) }),
    response: value => trace.response(Buffer.from(JSON.stringify(value)), { send: raw => downstream.push(raw) }) };
}

function failureOf(action) {
  try { action(); }
  catch (error) { return error; }
  return undefined;
}

test('navigation trace records document phase only for owned Page sessions', () => {
  // Arrange
  const f = fixture();
  f.trace.created.push('owned');
  f.trace.sessions.set('s', 'owned');
  const event = { method: 'Page.frameNavigated', sessionId: 's', params: { frame: { id: 'frame', loaderId: 'loader' } } };
  f.response({ ...event, sessionId: 'foreign' });
  // Act
  f.response(event);
  // Assert
  assert.equal(f.trace.report().navigation.length, 1);
  assert.equal(f.trace.navigation[0].loader, 'loader');
  assert.equal(f.trace.navigation[0].frame, 'frame');
  assert.equal(f.downstream.length, 2);
  assert.deepEqual(JSON.parse(f.downstream[1]), event);
});

test('navigation replies preserve loader and native error without inventing retirement', () => {
  // Arrange
  const f = fixture();
  f.command({ id: 1, method: 'Page.navigate', sessionId: 's' });
  // Act
  f.response({ id: 1, result: { loaderId: 'loader', errorText: 'net::ERR_ABORTED' } });
  // Assert
  assert.equal(f.trace.completed[0].navigationLoader, 'loader');
  assert.equal(f.trace.completed[0].navigationError, 'net::ERR_ABORTED');
  assert.equal(f.trace.lifecycle.length, 0);
});

test('owned navigation journal rejects overflow before retaining the sixty-fifth event', () => {
  // Arrange
  const f = fixture();
  f.trace.created.push('owned');
  f.trace.sessions.set('s', 'owned');
  const event = { method: 'Page.lifecycleEvent', sessionId: 's', params: { name: 'load' } };
  for (let index = 0; index < 64; index++) f.response(event);
  // Act
  const error = failureOf(() => f.response(event));
  // Assert
  assert.ok(error instanceof Error);
  assert.equal(f.trace.navigation.length, 64);
  assert.equal(f.downstream.length, 64);
});

test('owned navigation journal rejects oversized errors before retention', () => {
  // Arrange
  const f = fixture();
  f.trace.created.push('owned');
  f.trace.sessions.set('s', 'owned');
  // Act
  const error = failureOf(() => f.response({ method: 'Network.loadingFailed', sessionId: 's',
    params: { requestId: 'request', errorText: 'x'.repeat(4097) } }));
  // Assert
  assert.ok(error instanceof Error);
  assert.equal(f.trace.navigation.length, 0);
});

test('controlled trace withholds only exact owned retirement and releases original buffers', () => {
  // Arrange
  const trace = new HeldRetirementTrace(() => 0);
  const forwarded = [];
  const client = { readyState: 1, send: value => forwarded.push(value) };
  trace.created.push('owned');
  trace.holding = true;
  const owned = Buffer.from('{"method":"Target.targetDestroyed","params":{"targetId":"owned"}}');
  const foreign = Buffer.from('{"method":"Target.targetDestroyed","params":{"targetId":"other"}}');
  trace.response(owned, client);
  trace.response(foreign, client);
  assert.deepEqual(forwarded, [foreign]);
  assert.equal(trace.heldCount(), 1);
  assert.equal(trace.lifecycle.length, 2, 'both arrival receipts remain recorded');
  // Act
  trace.release();
  // Assert
  assert.equal(forwarded[1], owned);
  assert.equal(trace.heldCount(), 0);
});

test('controlled retirement storage rejects a ninth event before retaining it', () => {
  // Arrange
  const trace = new HeldRetirementTrace(() => 0);
  trace.created.push('owned');
  trace.holding = true;
  const raw = Buffer.from('{"method":"Target.targetDestroyed","params":{"targetId":"owned"}}');
  const client = { readyState: 1, send() {} };
  for (let index = 0; index < 8; index++) trace.response(raw, client);
  // Act
  const error = failureOf(() => trace.response(raw, client));
  // Assert
  assert.ok(error instanceof Error);
  assert.equal(trace.heldCount(), 8);
});

test('controlled retirement storage rejects oversized native packets', () => {
  // Arrange
  const trace = new HeldRetirementTrace(() => 0);
  trace.created.push('owned');
  trace.holding = true;
  const raw = Buffer.from(JSON.stringify({ method: 'Target.targetDestroyed', params: { targetId: 'owned', padding: 'x'.repeat(4096) } }));
  // Act
  const error = failureOf(() => trace.response(raw, { readyState: 1, send() {} }));
  // Assert
  assert.ok(error instanceof Error);
  assert.equal(trace.heldCount(), 0);
});

test('controlled release reclaims buffers after the client has closed', () => {
  // Arrange
  const trace = new HeldRetirementTrace(() => 0);
  trace.created.push('owned');
  trace.holding = true;
  let sent = false;
  trace.response(Buffer.from('{"method":"Target.targetDestroyed","params":{"targetId":"owned"}}'), { readyState: 3, send() { sent = true; } });
  // Act
  trace.release();
  // Assert
  assert.equal(sent, false);
  assert.equal(trace.heldCount(), 0);
});

test('selected trace forwards unrelated setup commands without spending pending slots', () => {
  // Arrange
  const trace = new CloseTrace(() => 0, method => method.startsWith('Target.'));
  const forwarded = [];
  const raw = Buffer.from('{"id":1,"method":"Page.enable"}');
  // Act
  for (let index = 0; index < 40; index++) trace.command(raw, undefined, { send: value => forwarded.push(value) });
  // Assert
  assert.equal(forwarded.length, 40);
  assert.ok(forwarded.every(value => value === raw));
  assert.deepEqual(trace.unanswered(), []);
});

test('trace correlates exact owned Context creation and disposal', () => {
  // Arrange
  const f = fixture();
  f.command({ id: 1, method: 'Target.createBrowserContext' });
  f.response({ id: 1, result: { browserContextId: 'context-owned' } });
  f.command({ id: 2, method: 'Target.disposeBrowserContext', params: { browserContextId: 'context-owned' } });
  // Act
  f.response({ id: 2, result: {} });
  // Assert
  assert.deepEqual(f.trace.report().contexts, ['context-owned']);
  assert.equal(f.trace.receipts[0].context, 'context-owned');
});

test('diagnostic listener selection leaves the normal endpoint unchanged', () => {
  // Arrange
  const endpoint = 'http://127.0.0.1:9222';
  // Act
  const selected = listenerEndpoint(endpoint, undefined);
  // Assert
  assert.equal(selected, endpoint);
});

test('diagnostic listener selection rejects non-loopback or decorated URLs', () => {
  for (const relay of ['ws://0.0.0.0:1234', 'ws://100.78.97.63:1234', 'ws://localhost:1234',
    'ws://127.0.0.1:0', 'ws://127.0.0.1:65536', 'ws://127.0.0.1:1234/path',
    'ws://user@127.0.0.1:1234', 'wss://127.0.0.1:1234']) {
    // Arrange
    const endpoint = 'http://127.0.0.1:9222';
    // Act
    const error = failureOf(() => listenerEndpoint(endpoint, relay));
    // Assert
    assert.ok(error instanceof Error);
  }
});

test('diagnostic listener selection accepts the dedicated relay', () => {
  // Arrange
  const relay = 'ws://127.0.0.1:54321';
  // Act
  const selected = listenerEndpoint('http://127.0.0.1:9222', relay);
  // Assert
  assert.equal(selected, relay);
});

test('diagnostic listener selection rejects a noncanonical browser even with a valid relay', () => {
  // Arrange
  const relay = 'ws://127.0.0.1:54321';
  // Act
  const error = failureOf(() => listenerEndpoint('http://example.invalid:9222', relay));
  // Assert
  assert.ok(error instanceof Error);
});

test('trace forwards the exact command wire buffer without rewriting it', () => {
  // Arrange
  const trace = new CloseTrace(() => 0);
  const command = Buffer.from('{"id":1,"method":"Page.enable","sessionId":"s"}');
  const forwarded = [];
  // Act
  trace.command(command, undefined, { send: raw => forwarded.push(raw) });
  // Assert
  assert.equal(forwarded[0], command);
  assert.equal(trace.unanswered().length, 1);
});

test('trace forwards the exact response wire buffer without rewriting it', () => {
  // Arrange
  const trace = new CloseTrace(() => 0);
  const command = Buffer.from('{"id":1,"method":"Page.enable","sessionId":"s"}');
  const response = Buffer.from('{"id":1,"result":{}}');
  trace.command(command, undefined, { send() {} });
  const forwarded = [];
  // Act
  trace.response(response, { send: raw => forwarded.push(raw) });
  // Assert
  assert.equal(forwarded[0], response);
  assert.deepEqual(trace.unanswered(), []);
});

test('detach receipt records only unanswered commands in that exact session', () => {
  // Arrange
  const f = fixture();
  f.command({ id: 1, method: 'Page.enable', sessionId: 'child' });
  f.command({ id: 2, method: 'Page.enable', sessionId: 'other' });
  // Act
  f.response({ method: 'Target.detachedFromTarget', params: { sessionId: 'child' } });
  // Assert
  assert.deepEqual(f.trace.lifecycle[0].unanswered, [1]);
  assert.equal(f.trace.unanswered().length, 2, 'native detach is not a fabricated response');
});

test('pending command inventory rejects overflow before forwarding the command', () => {
  // Arrange
  const f = fixture();
  for (let id = 1; id <= 32; id++) f.command({ id, method: 'Page.enable' });
  // Act
  const error = failureOf(() => f.command({ id: 33, method: 'Page.enable' }));
  // Assert
  assert.ok(error instanceof Error);
  assert.equal(f.upstream.length, 32);
  assert.equal(f.trace.unanswered().length, 32);
});

test('target destruction correlates pending commands through the recorded attachment identity', () => {
  // Arrange
  const f = fixture();
  f.command({ id: 1, method: 'Target.attachToTarget', params: { targetId: 'owned' } });
  f.response({ id: 1, result: { sessionId: 'child' } });
  f.command({ id: 2, method: 'Page.enable', sessionId: 'child' });
  f.command({ id: 3, method: 'Page.enable', sessionId: 'other' });
  // Act
  f.response({ method: 'Target.targetDestroyed', params: { targetId: 'owned' } });
  // Assert
  assert.deepEqual(f.trace.lifecycle[0].unanswered, [2]);
  assert.equal(f.trace.sessions.get('child'), 'owned');
});

test('recent completion ring retains its newest 128 identities only', () => {
  // Arrange
  const f = fixture();
  // Act
  for (let id = 1; id <= 130; id++) {
    f.command({ id, method: 'Page.enable' });
    f.response({ id, result: {} });
  }
  // Assert
  assert.equal(f.trace.completed.length, 128);
  assert.equal(f.trace.completed[0].id, 3);
  assert.equal(f.trace.completed.at(-1).id, 130);
});

test('diagnostic lifecycle overflow is an explicit failure, not a silent omission', () => {
  // Arrange
  const f = fixture();
  for (let i = 0; i < 64; i++) f.response({ method: 'Target.targetDestroyed', params: { targetId: `t${i}` } });
  // Act
  const error = failureOf(() => f.response({ method: 'Target.targetDestroyed', params: { targetId: 'overflow' } }));
  // Assert
  assert.ok(error instanceof Error);
  assert.equal(f.trace.lifecycle.length, 64);
});
