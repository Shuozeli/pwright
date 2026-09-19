// Test-only, bounded progress evidence survives an outer supervisor kill.
export function createLifecycleTrace(write, now, limit = 4096) {
  let records = 0;
  let test = '';
  return (phase, name) => {
    if (name !== undefined) {
      if (typeof name !== 'string' || name.length > 512) throw new Error('lifecycle trace name exceeds bound');
      test = name;
    }
    if (typeof phase !== 'string' || phase.length > 128) throw new Error('lifecycle trace phase exceeds bound');
    if (records >= limit) throw new Error('lifecycle trace record budget exhausted');
    records++;
    write(`# ${JSON.stringify({ event: 'lifecycle-progress', sequence: records, elapsedMs: now(), test, phase })}\n`);
  };
}
