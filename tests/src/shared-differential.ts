import { inspect, isDeepStrictEqual } from "node:util";
import type { RunContext, TestCase } from "./harness.js";
import type { SharedSession } from "./shared-lifecycle.js";

export type RunOperation = Exclude<keyof RunContext, "server">;

export class UnsupportedRunOperation extends Error {
  readonly code = "PWRIGHT_SHARED_UNSUPPORTED";
  constructor(readonly operation: RunOperation) {
    super(`Node FFI adapter does not implement ${operation}`);
    this.name = "UnsupportedRunOperation";
  }
}

export interface Diagnostic {
  name: string;
  message: string;
  code?: string;
  causes?: Diagnostic[];
}

export interface SideObservation {
  operations: RunOperation[];
  unsupported: RunOperation[];
  browserVersion?: string;
  data?: Record<string, unknown>;
  setupError?: Diagnostic;
  actionError?: Diagnostic;
  cleanupError?: Diagnostic;
}

export type ComparisonStatus = "match" | "value-mismatch" | "unsupported" |
  "oracle-error" | "candidate-error" | "setup-error" | "cleanup-error";

export interface SharedComparison {
  suite: string;
  test: string;
  status: ComparisonStatus;
  oracle: SideObservation;
  candidate: SideObservation;
}

export function diagnostic(error: unknown, depth = 0): Diagnostic {
  const value = error instanceof Error ? error : new Error(String(error));
  return {
    name: value.name.slice(0, 128), message: value.message.slice(0, 4096),
    ...("code" in value && typeof value.code === "string" ? { code: value.code.slice(0, 128) } : {}),
    ...(value instanceof AggregateError && depth < 2 ? {
      causes: value.errors.slice(0, 4).map((cause: unknown) => diagnostic(cause, depth + 1)),
    } : {}),
  };
}

export async function observeCase(
  test: TestCase,
  create: () => Promise<SharedSession<RunContext>>,
): Promise<SideObservation> {
  const observation: SideObservation = { operations: [], unsupported: [] };
  let session: SharedSession<RunContext>;
  try { session = await create(); observation.browserVersion = session.browserVersion; }
  catch (error) { observation.setupError = diagnostic(error); return observation; }
  const tracked = { ...session.ctx };
  for (const key of Object.keys(session.ctx)) {
    if (key === "server") continue;
    const operation = key as RunOperation;
    const method = session.ctx[operation];
    Object.defineProperty(tracked, key, { value: async (...args: unknown[]) => {
      if (observation.operations.length >= 256) {
        const error = new Error("fixture exceeded its 256-operation budget");
        observation.actionError = diagnostic(error);
        throw error;
      }
      observation.operations.push(operation);
      try { return await Reflect.apply(method, session.ctx, args); }
      catch (error) {
        if (error instanceof UnsupportedRunOperation && !observation.unsupported.includes(error.operation))
          observation.unsupported.push(error.operation);
        throw error;
      }
    } });
  }
  try {
    observation.data = await test.fn(tracked);
    if (!observation.data || typeof observation.data !== "object" || Array.isArray(observation.data))
      throw new Error("fixture must return an observation record");
    dataEvidence(observation.data);
  }
  catch (error) { observation.data = undefined; observation.actionError = diagnostic(error); }
  finally {
    try { await session.cleanup(); }
    catch (error) { observation.cleanupError = diagnostic(error); }
  }
  return observation;
}

export function classify(oracle: SideObservation, candidate: SideObservation): ComparisonStatus {
  if (oracle.cleanupError || candidate.cleanupError) return "cleanup-error";
  if (oracle.setupError || candidate.setupError) return "setup-error";
  if (oracle.actionError || oracle.unsupported.length) return "oracle-error";
  if (candidate.unsupported.length) return "unsupported";
  if (candidate.actionError) return "candidate-error";
  if (!oracle.browserVersion || !candidate.browserVersion || !oracle.data || !candidate.data)
    return "setup-error";
  if (oracle.browserVersion !== candidate.browserVersion) return "value-mismatch";
  return isDeepStrictEqual(oracle.data, candidate.data) ? "match" : "value-mismatch";
}

// Report values without JSON's lossy undefined/NaN/-0 coercion. Comparison itself
// still uses the original values, never this display string or a normalized golden.
export function dataEvidence(data: Record<string, unknown>): string {
  const result = inspect(data, {
    depth: null, maxArrayLength: null, maxStringLength: null,
    customInspect: false, getters: false, colors: false, compact: true,
  });
  if (Buffer.byteLength(result) > 64 * 1024) throw new Error("fixture evidence exceeded its 64 KiB budget");
  return result;
}

export function summarize(rows: readonly SharedComparison[]): Record<ComparisonStatus, number> {
  const counts: Record<ComparisonStatus, number> = {
    match: 0, "value-mismatch": 0, unsupported: 0, "oracle-error": 0,
    "candidate-error": 0, "setup-error": 0, "cleanup-error": 0,
  };
  for (const row of rows) counts[row.status]++;
  return counts;
}
