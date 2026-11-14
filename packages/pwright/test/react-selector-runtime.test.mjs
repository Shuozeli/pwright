import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const source = readFileSync(fileURLToPath(new URL('../../../crates/pwright-node-host/src/react_selector.rs', import.meta.url)), 'utf8');
const runtime = source.match(/const RUNTIME: &str = r#"([\s\S]*?)"#;/)?.[1];
assert.ok(runtime, 'Rust-owned opt-in React runtime source is present');
const selectorSource = readFileSync(fileURLToPath(new URL('../../../crates/pwright-backend-cdp-selector-runtime/src/lib.rs', import.meta.url)), 'utf8');
const selectorRuntime = selectorSource.match(/pub const SELECTOR_RUNTIME: &str = r#"([\s\S]*?)"#;/)?.[1];
assert.ok(selectorRuntime, 'Rust-owned web-platform selector runtime source is present');

class FakeElement {
  constructor(parent = null) {
    this.parent = parent;
    this.children = [];
    this.shadowRoot = null;
    this.attributes = new Set();
    if (parent) parent.children.push(this);
  }

  contains(other) {
    for (let node = other; node; node = node.parent)
      if (node === this) return true;
    return false;
  }

  getRootNode() {
    let node = this;
    while (node.parent) node = node.parent;
    return node;
  }

  hasAttribute(name) {
    return this.attributes.has(name);
  }
}

class FakeShadowRoot extends FakeElement {
  constructor(host, document) {
    super();
    this.host = host;
    this.ownerDocument = document;
  }
}

class FakeDocument extends FakeElement {
  constructor() {
    super();
    this.ownerDocument = null;
  }

  createTreeWalker(scope) {
    const nodes = [];
    const walk = node => {
      nodes.push(node);
      for (const child of node.children) walk(child);
    };
    walk(scope);
    let offset = 0;
    return {
      get currentNode() { return nodes[offset]; },
      nextNode() { if (offset + 1 >= nodes.length) return false; ++offset; return true; },
    };
  }
}

function fixture() {
  const document = new FakeDocument();
  const container = new FakeElement(document);
  const list = new FakeElement(container);
  const first = new FakeElement(list);
  const second = new FakeElement(list);
  const button = new FakeElement(container);
  const outside = new FakeElement(document);
  for (const node of [container, list, first, second, button, outside]) node.ownerDocument = document;
  function BookList() {}
  function BookItem() {}
  function ColorButton() {}
  const firstItem = { type: BookItem, memoizedProps: { name: 'The Great Gatsby' }, child: { type: 'li', stateNode: first } };
  const secondItem = { type: BookItem, memoizedProps: { name: 'Other book' }, child: { type: 'li', stateNode: second } };
  firstItem.sibling = secondItem;
  const bookList = { type: BookList, child: { type: 'ol', stateNode: list, child: firstItem } };
  bookList.sibling = {
    type: ColorButton,
    memoizedProps: { color: 'red', enabled: true, nested: { index: 0, value: 4.1 } },
    child: { type: 'button', stateNode: button },
  };
  const root = { type: null, child: bookList };
  container.__reactContainer$test = { stateNode: { current: root } };
  const engine = runInNewContext(runtime, {
    document,
    Element: FakeElement,
    ShadowRoot: FakeShadowRoot,
    NodeFilter: { SHOW_ELEMENT: 1 },
  });
  return { engine, document, container, list, first, second, button, outside };
}

test('opt-in React runtime counts live components and scopes their DOM roots', () => {
  // Arrange
  const { engine, document, list, first, second, outside } = fixture();

  // Act
  const all = Array.from(engine.query(document, 'BookItem'));
  const listMatch = Array.from(engine.query(document, 'BookList'));
  const scoped = Array.from(engine.query(list, 'BookItem'));
  const unrelated = Array.from(engine.query(outside, 'BookItem'));

  // Assert
  assert.deepEqual(all, [first, second]);
  assert.deepEqual(listMatch, [list]);
  assert.deepEqual(scoped, [first, second]);
  assert.deepEqual(unrelated, []);
});

test('opt-in React runtime matches nested property and string/regex operators', () => {
  // Arrange
  const { engine, document, first } = fixture();

  // Act
  const partial = Array.from(engine.query(document, 'BookItem[name *= "Gatsby"]'));
  const insensitive = Array.from(engine.query(document, 'BookItem[name = "the great gatsby" i]'));
  const regex = Array.from(engine.query(document, 'BookItem[name = /gat.by/i]'));
  const missing = Array.from(engine.query(document, 'BookItem[nested.index = 0]'));

  // Assert
  assert.deepEqual(partial, [first]);
  assert.deepEqual(insensitive, [first]);
  assert.deepEqual(regex, [first]);
  assert.deepEqual(missing, []);
});

test('opt-in React runtime traverses React 15 legacy roots', () => {
  // Arrange
  const document = new FakeDocument();
  const root = new FakeElement(document);
  root.ownerDocument = document;
  root.attributes.add('data-reactroot');
  function BookItem() {}
  root.__reactInternalInstance$test = {
    _currentElement: { type: BookItem, props: { name: 'Legacy book' } },
    _renderedComponent: {
      _currentElement: { type: 'li', props: {} },
      _hostNode: root,
    },
  };
  const engine = runInNewContext(runtime, {
    document,
    Element: FakeElement,
    ShadowRoot: FakeShadowRoot,
    NodeFilter: { SHOW_ELEMENT: 1 },
  });

  // Act
  const matches = Array.from(engine.query(document, 'BookItem[name="Legacy book"]'));

  // Assert
  assert.deepEqual(matches, [root]);
});

test('opt-in React runtime parses regex character classes and typed nested props', () => {
  // Arrange
  const { engine, document, button } = fixture();

  // Act
  const regex = Array.from(engine.query(document, 'ColorButton[color = /[pqr]ed/]'));
  const typed = Array.from(engine.query(document, 'ColorButton[nested.index = 0][enabled = true]'));
  const truthy = Array.from(engine.query(document, 'ColorButton[enabled]'));
  const absent = Array.from(engine.query(document, 'ColorButton[nested.missing.index = 0]'));

  // Assert
  assert.deepEqual(regex, [button]);
  assert.deepEqual(typed, [button]);
  assert.deepEqual(truthy, [button]);
  assert.deepEqual(absent, []);
});

test('opt-in React runtime accepts trailing whitespace after a property selector', () => {
  // Arrange
  const { engine, document, button } = fixture();

  // Act
  const matches = Array.from(engine.query(document, 'ColorButton[enabled = true] '));

  // Assert
  assert.deepEqual(matches, [button]);
});

test('opt-in React runtime rejects empty and malformed component selectors', () => {
  // Arrange
  const { engine, document } = fixture();

  // Act
  const empty = () => engine.query(document, '');
  const malformed = () => engine.query(document, 'BookItem!');

  // Assert
  assert.throws(empty, /selector cannot be empty/);
  assert.throws(malformed, /unexpected symbol "!" at position 8/);
});

test('opt-in React runtime discovers another root inside open shadow DOM', () => {
  // Arrange
  const { engine, document, outside } = fixture();
  const shadow = new FakeShadowRoot(outside, document);
  outside.shadowRoot = shadow;
  const item = new FakeElement(shadow);
  item.ownerDocument = document;
  function BookItem() {}
  shadow.__reactContainer$shadow = {
    stateNode: { current: { type: null, child: { type: BookItem, child: { type: 'li', stateNode: item } } } },
  };

  // Act
  const matches = Array.from(engine.query(document, 'BookItem'));
  const shadowMatches = Array.from(engine.query(shadow, 'BookItem'));

  // Assert
  assert.equal(matches.length, 3);
  assert.deepEqual(shadowMatches, [item]);
});

test('opt-in React runtime returns parent components before nested components', () => {
  // Arrange
  const document = new FakeDocument();
  const container = new FakeElement(document);
  const outer = new FakeElement(container);
  const inner = new FakeElement(outer);
  for (const node of [container, outer, inner]) node.ownerDocument = document;
  function Card() {}
  container.__reactContainer$order = {
    stateNode: {
      current: {
        type: null,
        child: {
          type: Card,
          child: {
            type: 'div',
            stateNode: outer,
            child: { type: Card, child: { type: 'span', stateNode: inner } },
          },
        },
      },
    },
  };
  const engine = runInNewContext(runtime, {
    document,
    Element: FakeElement,
    ShadowRoot: FakeShadowRoot,
    NodeFilter: { SHOW_ELEMENT: 1 },
  });

  // Act
  const matches = Array.from(engine.query(document, 'Card'));

  // Assert
  assert.deepEqual(matches, [outer, inner]);
});

test('opt-in React runtime visits a shadow root before a later light-DOM root', () => {
  // Arrange
  const document = new FakeDocument();
  const host = new FakeElement(document);
  const later = new FakeElement(document);
  const shadow = new FakeShadowRoot(host, document);
  host.shadowRoot = shadow;
  const shadowItem = new FakeElement(shadow);
  const lightItem = new FakeElement(later);
  for (const node of [host, later, shadowItem, lightItem]) node.ownerDocument = document;
  function Marker() {}
  shadow.__reactContainer$shadowOrder = {
    stateNode: { current: { type: null, child: { type: Marker, child: { type: 'i', stateNode: shadowItem } } } },
  };
  later.__reactContainer$lightOrder = {
    stateNode: { current: { type: null, child: { type: Marker, child: { type: 'i', stateNode: lightItem } } } },
  };
  const engine = runInNewContext(runtime, {
    document,
    Element: FakeElement,
    ShadowRoot: FakeShadowRoot,
    NodeFilter: { SHOW_ELEMENT: 1 },
  });

  // Act
  const matches = Array.from(engine.query(document, 'Marker'));

  // Assert
  assert.deepEqual(matches, [shadowItem, lightItem]);
});

test('scoped CSS includes its own element but plain CSS remains descendant-only', () => {
  // Arrange
  const root = {
    nodeType: 1,
    shadowRoot: null,
    querySelectorAll: () => [],
    matches: selector => selector === ':scope.target',
  };
  const engine = runInNewContext(selectorRuntime, { Node: { ELEMENT_NODE: 1 } });

  // Act
  const scoped = Array.from(engine.css(root, ':scope.target'));
  const plain = Array.from(engine.css(root, '.target'));
  const unmatched = Array.from(engine.css(root, ':scope.other'));

  // Assert
  assert.deepEqual(scoped, [root]);
  assert.deepEqual(plain, []);
  assert.deepEqual(unmatched, []);
});
