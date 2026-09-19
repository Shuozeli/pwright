const attach = (frame, parent = null) => ({ type: 'attach', frame, parent });
const commit = (frame, document, request = null) => ({ type: 'commit', frame, document, request });
const lifecycle = (frame, document, event) => ({ type: 'lifecycle', frame, document, event });
const start = (frame, request, favicon = false, document = null) => ({ type: 'start', frame, request, favicon, document });
const finish = request => ({ type: 'finish', request });
const tick = ms => ({ type: 'tick', ms });
const detach = frame => ({ type: 'detach', frame });

export const cases = [
  { name: 'initial and document lifecycle', steps: [attach(1), commit(1, 1), lifecycle(1, 1, 'domcontentloaded'), lifecycle(1, 1, 'load'), tick(499), tick(500), commit(1, 2), tick(999), tick(1000)] },
  { name: 'busy sibling and idle nested subtree', steps: [attach(1), attach(2, 1), attach(3, 1), attach(4, 2), start(3, 10), tick(500), finish(10), tick(999), tick(1000)] },
  { name: 'nested request blocks ancestor', steps: [attach(1), attach(2, 1), attach(3, 2), start(3, 10), tick(500), finish(10), tick(1000)] },
  { name: 'commit keeps document request only', steps: [attach(1), attach(2, 1), start(1, 10, false, 1), start(1, 11), start(2, 12), commit(1, 1, 10), tick(500), finish(11), finish(12), finish(10), tick(1000)] },
  { name: 'detached busy subtree unblocks parent', steps: [attach(1), attach(2, 1), attach(3, 2), start(3, 10), tick(500), detach(2), finish(10), attach(4, 1), tick(1000)] },
  { name: 'favicon and request deduplication', steps: [attach(1), start(1, 10), start(1, 10), start(1, 11, true), tick(500), finish(10), finish(10), tick(1000)] },
  { name: 'request after published idle', steps: [attach(1), commit(1, 1), tick(500), start(1, 10), tick(600), finish(10), tick(1200), lifecycle(1, 1, 'load'), tick(1800)] },
  { name: 'lifecycle recalculation during late request', steps: [attach(1), commit(1, 1), tick(500), start(1, 10), lifecycle(1, 1, 'load'), finish(10), tick(1000)] },
  { name: 'new child after published parent idle', steps: [attach(1), commit(1, 1), tick(500), attach(2, 1), start(2, 10), lifecycle(1, 1, 'load'), tick(1000), finish(10), tick(1500)] },
  { name: 'child document invalidates parent idle', steps: [attach(1), attach(2, 1), tick(500), start(2, 10, false, 1), commit(2, 1, 10), finish(10), tick(1000)] },
  { name: 'same-document preserves existing lifecycle', steps: [attach(1), commit(1, 1), lifecycle(1, 1, 'load'), tick(500), { type: 'hash', frame: 1 }, start(1, 10), { type: 'hash', frame: 1 }, finish(10)] },
];
