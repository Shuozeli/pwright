async function(token, observe) {
  const table = globalThis.__pwrightDragObserversV1;
  if (table === undefined) return false;
  if (!(table instanceof Map)) throw new Error('drag observer registry was replaced');
  const cleanup = table.get(token);
  return cleanup === undefined ? false : await cleanup(observe);
}
