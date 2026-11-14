import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import vm from 'node:vm';

function controllerSource() {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
  const rust = fs.readFileSync(path.join(root, 'crates/pwright-node-host/src/page_bindings.rs'), 'utf8');
  const match = rust.match(/const CONTROLLER_SOURCE: &str = r#"([\s\S]*?)"#;/);
  assert.ok(match, 'Rust page-bindings capability must own the browser controller source');
  return match[1];
}

function harness() {
  const calls = [];
  const context = vm.createContext({
    URL,
    atob,
    btoa,
    __calls: calls,
    __pwright_binding__: payload => calls.push(payload),
  });
  vm.runInContext(`
    class Window {}
    class Document {}
    class Node {}
    globalThis.Window = Window;
    globalThis.Document = Document;
    globalThis.Node = Node;
    globalThis.__window = new Window();
    globalThis.__document = new Document();
    globalThis.__node = new Node();
    ${controllerSource()}
  `, context);
  return { calls, context };
}

test('Rust-owned binding controller serializes aliases and cycles and decodes a cyclic result', async () => {
  // Arrange
  const { calls, context } = harness();
  vm.runInContext(`globalThis.__pwright_binding_controller__.add('roundtrip')`, context);

  // Act
  const promise = vm.runInContext(`
    (() => {
      const cyclic = {};
      cyclic.self = cyclic;
      return globalThis.roundtrip(cyclic, globalThis.__window, globalThis.__document, globalThis.__node);
    })()
  `, context);
  const payload = JSON.parse(calls[0]);
  const delivered = {};
  delivered.self = delivered;
  context.__delivery = {
    name: 'roundtrip',
    seq: payload.seq,
    result: delivered,
  };
  vm.runInContext(`globalThis.__pwright_binding_controller__.deliver(globalThis.__delivery)`, context);
  const result = await promise;

  // Assert
  assert.equal(payload.schema, 'pwright.page-binding.v1');
  assert.deepEqual(payload.serializedArgs[0], { o: [{ k: 'self', v: { ref: 1 } }], id: 1 });
  assert.deepEqual(payload.serializedArgs.slice(1), ['ref: <Window>', 'ref: <Document>', 'ref: <Node>']);
  assert.equal(result.self, result);
});

test('Rust-owned binding controller retains exactly one handle argument for native take', async () => {
  // Arrange
  const { calls, context } = harness();
  vm.runInContext(`globalThis.__pwright_binding_controller__.add('handle', true)`, context);
  context.__argument = { answer: 42 };

  // Act
  const promise = vm.runInContext(`globalThis.handle(globalThis.__argument, undefined, undefined)`, context);
  const payload = JSON.parse(calls[0]);
  context.__take = payload;
  const taken = vm.runInContext(`globalThis.__pwright_binding_controller__.take(globalThis.__take)`, context);
  context.__delivery = { name: 'handle', seq: payload.seq, result: 17 };
  vm.runInContext(`globalThis.__pwright_binding_controller__.deliver(globalThis.__delivery)`, context);

  // Assert
  assert.equal(Object.hasOwn(payload, 'serializedArgs'), false);
  assert.equal(taken, context.__argument);
  assert.equal(await promise, 17);
  assert.throws(
    () => vm.runInContext(`globalThis.handle(1, 2)`, context),
    /exposeBindingHandle supports a single argument, 2 received/,
  );
});

test('Rust-owned binding controller does not depend on Array map or push', () => {
  // Arrange
  const { calls, context } = harness();
  vm.runInContext(`globalThis.__pwright_binding_controller__.add('add')`, context);

  // Act
  vm.runInContext(`Array.prototype.map = null; Array.prototype.push = null; globalThis.add(5, 6)`, context);

  // Assert
  assert.deepEqual(JSON.parse(calls[0]).serializedArgs, [5, 6]);
});

test('Rust-owned binding controller preserves the official broken Array toJSON signal', () => {
  // Arrange
  const { calls, context } = harness();
  vm.runInContext(`globalThis.__pwright_binding_controller__.add('add')`, context);

  // Act
  vm.runInContext(`Array.prototype.toJSON = () => '[]'; globalThis.add(5, 6)`, context);

  // Assert
  assert.equal(JSON.parse(calls[0]).serializedArgs, '[]');
});
