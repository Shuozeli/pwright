/**
 * CLI test harness — subprocess execution and output comparison.
 *
 * Extends the golden test framework with CLI-level testing.
 * Both CLIs run as subprocesses; stdout is captured and normalized
 * into structured data for golden comparison.
 */

import { execFile } from "child_process";

// ─── Types ───

export interface CliResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export interface CliRunner {
  /** Execute a CLI command, return raw output */
  exec(command: string, ...args: string[]): Promise<CliResult>;
  /** Cleanup (close browser, etc.) */
  cleanup(): Promise<void>;
}

export interface CliTestCase {
  name: string;
  fn: (cli: CliRunner, serverPrefix: string) => Promise<Record<string, unknown>>;
}

export interface CliSuite {
  name: string;
  tests: CliTestCase[];
}

// ─── Suite Builder ───

export function defineCliSuite(
  name: string,
  setup: (t: CliSuiteBuilder) => void
): CliSuite {
  const builder = new CliSuiteBuilder(name);
  setup(builder);
  return builder.build();
}

class CliSuiteBuilder {
  private tests: CliTestCase[] = [];
  constructor(private name: string) {}

  test(name: string, fn: (cli: CliRunner, serverPrefix: string) => Promise<Record<string, unknown>>) {
    this.tests.push({ name, fn });
  }

  build(): CliSuite {
    return { name: this.name, tests: this.tests };
  }
}

// ─── Runner ───

import type { TestResult } from "./harness.js";

export async function runCliSuite(
  suite: CliSuite,
  cli: CliRunner,
  serverPrefix: string
): Promise<TestResult[]> {
  const results: TestResult[] = [];

  for (const tc of suite.tests) {
    const start = Date.now();
    try {
      const data = await tc.fn(cli, serverPrefix);
      results.push({
        name: tc.name,
        status: "pass",
        data,
        durationMs: Date.now() - start,
      });
    } catch (err: any) {
      results.push({
        name: tc.name,
        status: "fail",
        error: err.message,
        durationMs: Date.now() - start,
      });
    }
  }

  return results;
}

// ─── Subprocess Helper ───

export function execCli(
  bin: string,
  args: string[],
  options?: { cwd?: string; env?: Record<string, string>; timeout?: number }
): Promise<CliResult> {
  return new Promise((resolve) => {
    const child = execFile(
      bin, args,
      {
        cwd: options?.cwd ?? process.cwd(),
        env: { ...process.env, ...options?.env },
        timeout: options?.timeout ?? 15000,
        maxBuffer: 1024 * 1024,
      },
      (err, stdout, stderr) => {
        resolve({
          stdout: stdout?.toString() ?? "",
          stderr: stderr?.toString() ?? "",
          exitCode: err ? (err as any).code ?? 1 : 0,
        });
      }
    );
  });
}

// ─── Output Normalizers ───

/** Extract snapshot refs from CLI output */
export function parseSnapshotRefs(stdout: string): string[] {
  const refs: string[] = [];
  for (const line of stdout.split("\n")) {
    // pwright format: [e1] role "name"
    const m1 = line.match(/\[(e\d+)\]/);
    if (m1) { refs.push(m1[1]); continue; }
    // playwright-cli format: - role "name" [ref=e1]
    const m2 = line.match(/\[ref=(e\d+)\]/);
    if (m2) refs.push(m2[1]);
  }
  return refs;
}

/**
 * Parse snapshot lines from either format:
 *   pwright:        [e0] role "name"
 *   playwright-cli: - role "name" [ref=e0]
 */
export function parseSnapshotLines(stdout: string): { ref: string; role: string; name: string }[] {
  const nodes: { ref: string; role: string; name: string }[] = [];
  for (const line of stdout.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // pwright format: [e0] role "name"
    const m1 = trimmed.match(/^\[(e\d+)\]\s+(\w+)(?:\s+"([^"]*)")?/);
    if (m1) {
      nodes.push({ ref: m1[1], role: m1[2], name: m1[3] ?? "" });
      continue;
    }

    // playwright-cli YAML format: - role "name" [ref=e1]
    const m2 = trimmed.match(/^-\s+(\w+)(?:\s+"([^"]*)")?.+\[ref=(e\d+)\]/);
    if (m2) {
      nodes.push({ ref: m2[3], role: m2[1], name: m2[2] ?? "" });
      continue;
    }
  }
  return nodes;
}

// ─── Stdout Normalization ───

/** Strip ANSI escape codes */
export function stripAnsi(s: string): string {
  return s.replace(/\u001b\[[0-9;]*m/g, "");
}

/**
 * Normalize CLI output to extract just the result value.
 *
 * playwright-cli outputs:
 *   ### Result\n"Content Page"\n### Ran Playwright code\n```js\n...\n```
 *
 * pwright outputs:
 *   INFO connecting to Chrome...\n"Content Page"
 *
 * This extracts the last meaningful line — the actual result.
 */
export function normalizeEvalOutput(stdout: string): string {
  const clean = stripAnsi(stdout);
  const lines = clean.split("\n").map((l) => l.trim()).filter(Boolean);

  // For playwright-cli: find line after "### Result"
  const resultIdx = lines.findIndex((l) => l === "### Result");
  if (resultIdx >= 0 && resultIdx + 1 < lines.length) {
    return lines[resultIdx + 1];
  }

  // For pwright: skip INFO lines, take the last non-empty line
  const meaningful = lines.filter(
    (l) => !l.startsWith("INFO") && !l.startsWith("WARN") && !l.startsWith("Connected") && !l.startsWith("Tab:") && !l.startsWith("URL:")
      && !l.includes("connecting to Chrome") && !l.includes("connected to Chrome") && !l.includes("tab re-attached") && !l.includes("tab created")
      && !l.startsWith("✅") && !l.startsWith("📸") && !l.startsWith("📄")
  );

  return meaningful.length > 0 ? meaningful[meaningful.length - 1] : "";
}

/** Normalize any CLI stdout — strip noise, return meaningful content */
export function normalizeOutput(stdout: string): string {
  const clean = stripAnsi(stdout);
  const lines = clean.split("\n").map((l) => l.trim()).filter(Boolean);

  // Strip noise from both CLIs
  const meaningful = lines.filter(
    (l) => !l.startsWith("INFO") && !l.startsWith("WARN")
      && !l.includes("connecting to Chrome") && !l.includes("connected to Chrome")
      && !l.includes("tab re-attached") && !l.includes("tab created")
      && !l.startsWith("### Browser") && !l.startsWith("- default:") && !l.startsWith("- browser-type:")
      && !l.startsWith("- user-data-dir:") && !l.startsWith("- headed:") && !l.startsWith("---")
      && !l.startsWith("### Ran Playwright") && !l.startsWith("```")
  );

  return meaningful.join("\n");
}
