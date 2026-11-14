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

test('known Frame lookup reuses a wrapper without enumerating native frames', () => {
  // Arrange
  const { root, source, cache } = fixture();
  source.rows.push({ id: '2', parentId: '1' });
  const child = cache.frames()[1];
  const binds = source.binds.length;
  source.rows = null;

  // Act
  const knownRoot = cache.knownFrame('1');
  const knownChild = cache.knownFrame('2');

  // Assert
  assert.equal(knownRoot, root);
  assert.equal(knownChild, child);
  assert.equal(source.binds.length, binds);
  assert.equal(cache.knownFrame('3'), null);
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

test('Frame event resolves only its ancestry and reuses the enumerated canonical Frame', () => {
  // Arrange
  const { root, source, cache } = fixture();
  source.rows.push({ id: '2', parentId: '1' }, { id: '3', parentId: '2' }, { id: '4', parentId: '1' });

  // Act
  const nested = cache.forEvent('3');

  // Assert
  assert.equal(nested.parent.parent, root);
  assert.deepEqual(source.binds, ['2', '3']);
  assert.equal(cache.frames()[2], nested);
});

test('Frame event rejects an oversized projection before binding or retaining wrappers', () => {
  // Arrange
  const { source, cache } = fixture();
  source.rows.push(...Array.from({ length: 128 }, (_, index) => ({ id: String(index + 2), parentId: '1' })));

  // Act
  const result = () => cache.forEvent('2');

  // Assert
  assert.throws(result, /identity capacity/);
  assert.deepEqual(source.binds, []);
  assert.deepEqual([...cache._frames.keys()], ['1']);
});

test('Frame event rejects missing or cyclic ancestry without retaining new wrappers', () => {
  // Arrange
  const { source, cache } = fixture();
  source.rows.push({ id: '2', parentId: '3' }, { id: '3', parentId: '2' });

  // Act
  const result = () => cache.forEvent('2');

  // Assert
  assert.throws(result, /ancestry/);
  assert.deepEqual(source.binds, []);
  assert.deepEqual([...cache._frames.keys()], ['1']);
});

test('retired Frame event identities cannot be replayed after projection cleanup', () => {
  // Arrange
  const { source, cache } = fixture();
  source.rows.push({ id: '2', parentId: '1' });
  const old = cache.forEvent('2');
  source.rows = [{ id: '1', parentId: null }, { id: '3', parentId: '1' }];

  // Act
  const next = cache.forEvent('3');

  // Assert
  assert.notEqual(next, old);
  assert.deepEqual([...cache._frames.keys()], ['1', '3']);
  assert.throws(() => cache.forEvent('2'), /not live/);
  assert.equal(cache._retired.size, 0);
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

test('detached child-first notices reuse previously observed canonical Frames', () => {
  // Arrange
  const root = { _id: '1' };
  const completed = [];
  const rows = [{ id: '1', parentId: null }, { id: '2', parentId: '1' }, { id: '3', parentId: '2' }];
  const tree = {
    frames: () => rows.slice(),
    metadata: id => ({ parentId: () => id === '2' ? '1' : '2' }),
    retiredMetadata: () => { throw new Error('cached Frame must not consume a native binding'); },
    finishDetached: id => completed.push(id),
  };
  const cache = new FrameCache(tree, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));
  const [, child, grandchild] = cache.frames();
  rows.splice(1);
  cache.frames();

  // Act
  const first = cache.detached('3');
  cache.finishDetached('3');
  const second = cache.detached('2');
  cache.finishDetached('2');

  // Assert
  assert.equal(first, grandchild);
  assert.equal(second, child);
  assert.equal(first.parent, second);
  assert.equal(second.parent, root);
  assert.deepEqual([...cache._retired.keys()], []);
  assert.deepEqual(completed, ['3', '2']);
});

test('unobserved detached Frame materializes from retained native parent identity', () => {
  // Arrange
  const root = { _id: '1' };
  const remaining = new Set(['2', '3']);
  const bindings = [];
  const bind = id => {
    if (!remaining.delete(id)) throw new Error(`Frame ${id} binding consumed twice`);
    bindings.push(id);
    return { parentId: () => id === '3' ? '2' : '1', isDetached: () => true };
  };
  const tree = {
    frames: () => [{ id: '1', parentId: null }],
    retiredMetadata: bind,
    eventMetadata: bind,
  };
  const cache = new FrameCache(tree, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));

  // Act
  const grandchild = cache.detached('3');
  const child = cache.detached('2');

  // Assert
  assert.equal(grandchild.parent, child);
  assert.equal(child.parent, root);
  assert.equal(grandchild.metadata.isDetached(), true);
  assert.deepEqual(bindings, ['3', '2']);
  cache.finishDetached('3');
  cache.finishDetached('2');
  assert.equal(cache._retired.size, 0);
});

test('unobserved live ancestor uses one reserved event binding and remains canonical', () => {
  // Arrange
  const root = { _id: '1' };
  const calls = [];
  const tree = {
    frames: () => [{ id: '1', parentId: null }, { id: '2', parentId: '1' }],
    metadata: () => { throw new Error('event ancestor must not use a dynamic binding'); },
    retiredMetadata(id) { calls.push(['retired', id]); return { parentId: () => '2' }; },
    eventMetadata(id) { calls.push(['ancestor', id]); return { parentId: () => '1' }; },
  };
  const cache = new FrameCache(tree, root, (id, metadata, parent) => ({ _id: id, metadata, parent }));

  // Act
  const child = cache.detached('3');
  const parent = cache.frames()[1];
  cache.finishDetached('3');

  // Assert
  assert.equal(child.parent, parent);
  assert.equal(parent.parent, root);
  assert.deepEqual(calls, [['retired', '3'], ['ancestor', '2']]);
  cache.clearRetired();
  assert.equal(cache._retired.size, 0);
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
