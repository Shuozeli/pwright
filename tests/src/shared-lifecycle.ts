export interface SharedSession<Context> {
  ctx: Context;
  browserVersion: string;
  cleanup(): Promise<void>;
}

// All owned resources are attempted even when an earlier cleanup fails.
export async function closeAll(actions: readonly (() => Promise<void>)[]): Promise<void> {
  const errors: unknown[] = [];
  for (const close of actions) {
    try { await close(); } catch (error) { errors.push(error); }
  }
  if (errors.length) throw new AggregateError(errors, "fixture cleanup failed");
}

export async function rejectAfterCleanup(error: unknown, cleanup: () => Promise<void>): Promise<never> {
  try { await cleanup(); }
  catch (cleanupError) { throw new AggregateError([error, cleanupError], "fixture setup and cleanup failed"); }
  throw error;
}
