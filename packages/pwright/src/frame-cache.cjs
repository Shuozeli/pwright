'use strict';

const MAX_PENDING_DETACHED_FRAMES = 128;
const MAX_LIVE_EVENT_FRAMES = 128;

// Wrapper identity only. Membership, parent IDs and detach state are native.
module.exports = class FrameCache {
  constructor(tree, root, create) {
    this._tree = tree;
    this._root = root;
    this._create = create;
    this._frames = new Map([[root._id, new WeakRef(root)]]);
    this._retired = new Map();
  }
  knownFrame(id) {
    if (id === this._root._id) return this._root;
    return this._frames.get(id)?.deref() ?? this._retired.get(id) ?? null;
  }
  _prune(live) {
    for (const [id, reference] of this._frames) {
      if (live.has(id)) continue;
      this._frames.delete(id);
      const frame = reference.deref();
      if (!this._tree.retiredMetadata || !frame) continue;
      if (!this._retired.has(id) && this._retired.size >= MAX_PENDING_DETACHED_FRAMES)
        throw new Error('Pending detached Frame identity capacity exceeded');
      this._retired.set(id, frame);
    }
  }
  lookup(name, url, settings) {
    const live = this._tree.retiredMetadata
      ? new Set(this._tree.frames().map(identity => identity.id)) : null;
    if (live) this._prune(live);
    // Keep known wrappers alive across foreign callback reentrancy and GC.
    const known = new Map([[this._root._id, this._root]]);
    for (const [id, reference] of this._frames) {
      const frame = reference.deref();
      if (frame) known.set(id, frame);
      else this._frames.delete(id);
    }
    if (live) for (const [id, frame] of this._retired) if (live.has(id)) known.set(id, frame);
    const selected = this._tree.lookup(name, url, settings, [...known.keys()]);
    if (!selected) return null;
    for (const value of selected.newFrames) {
      // A predicate may have called frames()/frame() and populated this cache.
      let frame = known.get(value.id) ?? this._frames.get(value.id)?.deref() ?? this._retired.get(value.id);
      if (!frame) {
        const parent = value.parentId == null ? null
          : known.get(value.parentId) ?? this._frames.get(value.parentId)?.deref() ?? this._retired.get(value.parentId);
        if (value.parentId != null && !parent) throw new Error('Invalid native Frame parent identity');
        frame = this._create(value.id, value.binding, parent);
        this._frames.set(value.id, new WeakRef(frame));
      }
      known.set(value.id, frame);
    }
    const frame = known.get(selected.id);
    if (!frame) throw new Error('Invalid native Frame selection identity');
    return frame;
  }
  frames() {
    const identities = this._tree.frames();
    const live = new Map(identities.map(identity => [identity.id, identity]));
    // The opt-in path strongly retains only bounded event ancestry; normal
    // live Frame wrappers remain weak until materialized by callers.
    this._prune(live);
    const resolved = new Map([[this._root._id, this._root]]);
    const resolve = id => {
      if (resolved.has(id)) return resolved.get(id);
      const identity = live.get(id);
      if (!identity) throw new Error('Invalid native Frame parent identity');
      let frame = this._frames.get(id)?.deref() ?? this._retired.get(id);
      if (!frame) {
        const parent = identity.parentId == null ? null : resolve(identity.parentId);
        frame = this._create(id, this._tree.metadata(id), parent);
        this._frames.set(id, new WeakRef(frame));
      }
      resolved.set(id, frame);
      return frame;
    };
    return identities.map(identity => resolve(identity.id));
  }
  forEvent(id) {
    if (typeof id !== 'string' || !/^[1-9]\d{0,19}$/.test(id))
      throw new TypeError('Invalid native Frame event identity');
    const identities = this._tree.frames();
    if (!Array.isArray(identities) || identities.length > MAX_LIVE_EVENT_FRAMES)
      throw new Error('Frame event projection exceeds identity capacity');
    const live = new Map();
    for (const identity of identities) {
      if (typeof identity.id !== 'string' || live.has(identity.id))
        throw new Error('Invalid native Frame event projection');
      live.set(identity.id, identity);
    }
    if (!live.has(id)) throw new Error('Native Frame event identity is not live');
    this._prune(live);
    const resolve = (frameId, depth) => {
      if (frameId === this._root._id) return this._root;
      if (depth >= MAX_LIVE_EVENT_FRAMES)
        throw new Error('Invalid native Frame event ancestry');
      const identity = live.get(frameId);
      if (!identity || typeof identity.parentId !== 'string')
        throw new Error('Invalid native Frame event parent identity');
      let frame = this._frames.get(frameId)?.deref() ?? this._retired.get(frameId);
      if (!frame) {
        const parent = resolve(identity.parentId, depth + 1);
        frame = this._create(frameId, this._tree.metadata(frameId), parent);
        this._frames.set(frameId, new WeakRef(frame));
      }
      return frame;
    };
    return resolve(id, 0);
  }
  detached(id) {
    if (!this._tree.retiredMetadata) throw new Error('Frame detach publication was not selected');
    const retain = (frameId, frame) => {
      if (!this._retired.has(frameId) && this._retired.size >= MAX_PENDING_DETACHED_FRAMES)
        throw new Error('Pending detached Frame identity capacity exceeded');
      this._frames.delete(frameId);
      this._retired.set(frameId, frame);
      return frame;
    };
    const resolve = (frameId, depth, target) => {
      if (frameId === this._root._id) return this._root;
      if (depth >= MAX_PENDING_DETACHED_FRAMES) throw new Error('Invalid detached Frame ancestry');
      const known = this._retired.get(frameId) ?? this._frames.get(frameId)?.deref();
      if (known) return retain(frameId, known);
      if (this._retired.size >= MAX_PENDING_DETACHED_FRAMES)
        throw new Error('Pending detached Frame identity capacity exceeded');
      const binding = target ? this._tree.retiredMetadata(frameId) : this._tree.eventMetadata(frameId);
      const parentId = binding.parentId();
      if (typeof parentId !== 'string') throw new Error('Detached main Frame is not publishable');
      const parent = resolve(parentId, depth + 1, false);
      const frame = this._create(frameId, binding, parent);
      return retain(frameId, frame);
    };
    return resolve(id, 0, true);
  }
  finishDetached(id) {
    try { this._tree.finishDetached?.(id); }
    finally { this._retired.delete(id); }
  }
  clearRetired() { this._retired.clear(); }
};
