import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

const require = createRequire(import.meta.url);
const { parseArguments, parseValue } = require('../src/binding-values.cjs');

test('binding value codec reconstructs aliases and cycles', () => {
  // Arrange
  const encoded = {
    o: [
      { k: 'name', v: 'root' },
      { k: 'self', v: { ref: 1 } },
    ],
    id: 1,
  };

  // Act
  const decoded = parseValue(encoded);

  // Assert
  assert.equal(decoded.name, 'root');
  assert.equal(decoded.self, decoded);
});

test('binding argument decoder keeps a shared limit but independent reference tables', () => {
  // Arrange
  const encoded = [
    { o: [{ k: 'self', v: { ref: 1 } }], id: 1 },
    { o: [{ k: 'self', v: { ref: 1 } }], id: 1 },
  ];

  // Act
  const [left, right] = parseArguments(encoded);

  // Assert
  assert.equal(left.self, left);
  assert.equal(right.self, right);
  assert.notEqual(left, right);
});
