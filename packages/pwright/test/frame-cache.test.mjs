import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createRequire } from 'node:module';
const FrameCache = createRequire(import.meta.url)('../src/frame-cache.cjs');

function fixture() {
  const root = { _id: '1', parent: null };
  const source = { rows: [{ id: '1', parentId: null }], binds: [] };
  const cache = new FrameCache({
    frames: () => source.rows,
    metadata: id => { source.binds.push(id); return { id }; },
  }, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));
  return { root, source, cache };
}

test('enumeration preserves root and child identities without rebinding native metadata', () => {
  // Arrange
  const { root, source, cache } = fixture();
  source.rows.push({ id: '2', parentId: '1' });
  const initial = cache.frames();
  // Act
  const again = cache.frames();
  // Assert
  assert.notEqual(initial, again);
  assert.equal(again[0], root);
  assert.equal(again[1], initial[1]);
  assert.equal(again[1].parent, root);
  assert.deepEqual(source.binds, ['2']);
});

test('nested parents are resolved by identity without reordering native collection', () => {
  // Arrange
  const { root, source, cache } = fixture();
  source.rows.push({ id: '3', parentId: '2' }, { id: '2', parentId: '1' });
  // Act
  const frames = cache.frames();
  // Assert
  assert.deepEqual(frames.map(frame => frame._id), ['1', '3', '2']);
  assert.equal(frames[1].parent, frames[2]);
  assert.equal(frames[2].parent, root);
  assert.deepEqual(source.binds, ['2', '3']);
});

test('retiring a subtree prunes weak indexes without destroying retained parent identity', () => {
  // Arrange
  const { root, source, cache } = fixture();
  source.rows.push({ id: '2', parentId: '1' }, { id: '3', parentId: '2' });
  const [, child, nested] = cache.frames();
  source.rows = [{ id: '1', parentId: null }, { id: '4', parentId: '1' }];
  // Act
  const after = cache.frames();
  // Assert
  assert.equal(after[0], root);
  assert.notEqual(after[1], child);
  assert.equal(nested.parent, child);
  assert.equal(child.parent, root);
  assert.deepEqual([...cache._frames.keys()], ['1', '4']);
  assert.ok([...cache._frames.values()].every(value => value instanceof WeakRef));
});

test('native admission errors propagate and a later enumeration can retry', () => {
  // Arrange
  const root = { _id: '1' };
  let exhausted = true;
  const error = new Error('PWRIGHT_RESOURCE_EXHAUSTED');
  const cache = new FrameCache({ frames: () => [{ id: '1' }, { id: '2', parentId: '1' }],
    metadata: () => { if (exhausted) throw error; return {}; },
  }, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));
  // Act / Assert
  assert.throws(() => cache.frames(), value => value === error);
  exhausted = false;
  assert.equal(cache.frames()[1].parent, root);
});

test('lookup binds only the native-selected ancestry and reuses it on repeat', () => {
  // Arrange
  const root = { _id: '1' };
  const created = [];
  const requests = [];
  const cache = new FrameCache({ lookup: (name, url, settings, known) => {
    requests.push({ name, url, settings, known });
    return { id: '3', newFrames: known.includes('3') ? [] : [
      { id: '2', parentId: '1', binding: { id: '2' } },
      { id: '3', parentId: '2', binding: { id: '3' } },
    ] };
  } }, root, (id, binding, parent) => { created.push(id); return { _id: id, binding, parent }; });
  const settings = {};
  const first = cache.lookup('selected', undefined, settings);
  // Act
  const repeated = cache.lookup('selected', undefined, settings);
  // Assert
  assert.equal(first, repeated);
  assert.equal(first.parent.parent, root);
  assert.deepEqual(created, ['2', '3']);
  assert.deepEqual(requests[0], { name: 'selected', url: undefined, settings, known: ['1'] });
  assert.deepEqual(requests[1].known, ['1', '2', '3']);
});

test('lookup reuses wrappers created by a reentrant enumeration', () => {
  // Arrange
  const { cache, source } = fixture();
  source.rows.push({ id: '2', parentId: '1' });
  let inner;
  cache._tree.lookup = () => {
    inner = cache.frames()[1];
    return { id: '2', newFrames: [{ id: '2', parentId: '1', binding: { unused: true } }] };
  };
  // Act
  const outer = cache.lookup(undefined, () => true, {});
  // Assert
  assert.equal(outer, inner);
  assert.deepEqual(source.binds, ['2']);
});

test('lookup preserves arbitrary callback throws and no-match allocates no wrappers', () => {
  // Arrange
  const { cache, source } = fixture();
  const thrown = { marker: true };
  cache._tree.lookup = () => { throw thrown; };
  // Act / Assert
  assert.throws(() => cache.lookup(undefined, () => false, {}), error => error === thrown);
  cache._tree.lookup = () => null;
  assert.equal(cache.lookup(undefined, () => false, {}), null);
  assert.deepEqual(source.binds, []);
});
