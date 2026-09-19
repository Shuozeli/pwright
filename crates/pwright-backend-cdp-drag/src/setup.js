function(token) {
  const name = '__pwrightDragObserversV1';
  let table = globalThis[name];
  if (table === undefined) {
    table = new Map();
    Object.defineProperty(globalThis, name, { value: table, configurable: true });
  }
  if (!(table instanceof Map)) throw new Error('drag observer registry was replaced');
  // Explicit no-install result: cleanup must never delete another owner's token.
  if (table.has(token) || table.size >= 32) return false;
  let timer;
  let dragEvent = null;
  let settle;
  let observation = Promise.resolve(false);
  const drag = event => { dragEvent = event; };
  const mouse = () => {
    observation = new Promise(resolve => { settle = resolve; });
    addEventListener('dragstart', drag, { capture: true, once: true });
    timer = setTimeout(() => {
      timer = undefined;
      const started = dragEvent !== null && !dragEvent.defaultPrevented;
      dragEvent = null;
      settle(started);
      settle = undefined;
    }, 0);
  };
  const cleanup = async observe => {
    try {
      if (observe) return await observation;
      return false;
    } finally {
      removeEventListener('mousemove', mouse, { capture: true });
      removeEventListener('dragstart', drag, { capture: true });
      if (timer !== undefined) clearTimeout(timer);
      if (settle) settle(false);
      dragEvent = null;
      table.delete(token);
      if (table.size === 0 && globalThis[name] === table) delete globalThis[name];
    }
  };
  table.set(token, cleanup);
  addEventListener('mousemove', mouse, { capture: true, once: true });
  return true;
}
