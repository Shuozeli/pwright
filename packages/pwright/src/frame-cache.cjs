'use strict';

// Wrapper identity only. Membership, parent IDs and detach state are native.
module.exports = class FrameCache {
  constructor(tree, root, create) {
    this._tree = tree;
    this._root = root;
    this._create = create;
    this._frames = new Map([[root._id, new WeakRef(root)]]);
  }
  lookup(name, url, settings) {
    // Keep known wrappers alive across foreign callback reentrancy and GC.
    const known = new Map([[this._root._id, this._root]]);
    for (const [id, reference] of this._frames) {
      const frame = reference.deref();
      if (frame) known.set(id, frame);
      else this._frames.delete(id);
    }
    const selected = this._tree.lookup(name, url, settings, [...known.keys()]);
    if (!selected) return null;
    for (const value of selected.newFrames) {
      // A predicate may have called frames()/frame() and populated this cache.
      let frame = known.get(value.id) ?? this._frames.get(value.id)?.deref();
      if (!frame) {
        const parent = value.parentId == null ? null
          : known.get(value.parentId) ?? this._frames.get(value.parentId)?.deref();
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
    // Retired wrappers may be retained by users, never by this cache.
    for (const id of this._frames.keys()) if (!live.has(id)) this._frames.delete(id);
    const resolved = new Map([[this._root._id, this._root]]);
    const resolve = id => {
      if (resolved.has(id)) return resolved.get(id);
      const identity = live.get(id);
      if (!identity) throw new Error('Invalid native Frame parent identity');
      let frame = this._frames.get(id)?.deref();
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
};
