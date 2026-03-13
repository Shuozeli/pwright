/**
 * Golden test harness.
 *
 * Provides suite/test primitives, golden recording, and golden matching.
 *
 * Usage in spec files:
 *   export default defineSuite('page-goto', (t) => {
 *     t.test('should navigate', async (ctx) => {
 *       await ctx.goto(ctx.server.EMPTY_PAGE);
 *       return { url: await ctx.url() };
 *     });
 *   });
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { resolve } from "path";
import { fileURLToPath } from "url";
import { dirname } from "path";
import type { TestServer } from "./test-server.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const GOLDEN_DIR = resolve(__dirname, "../golden");

// ─── Types ───

export interface RunContext {
  /** Test server instance */
  server: TestServer;
  goto(url: string): Promise<void>;
  reload(): Promise<void>;
  goBack(): Promise<void>;
  goForward(): Promise<void>;
  url(): Promise<string>;
  title(): Promise<string>;
  content(): Promise<string>;
  evaluate(expr: string): Promise<unknown>;
  screenshot(format?: string): Promise<Buffer>;
  textContent(selector: string): Promise<string | null>;
  getAttribute(selector: string, name: string): Promise<string | null>;
  innerText(selector: string): Promise<string>;
  innerHTML(selector: string): Promise<string>;
  inputValue(selector: string): Promise<string>;
  click(selector: string): Promise<void>;
  dblclick(selector: string): Promise<void>;
  fill(selector: string, value: string): Promise<void>;
  type(selector: string, text: string): Promise<void>;
  press(key: string): Promise<void>;
  hover(selector: string): Promise<void>;
  focus(selector: string): Promise<void>;
  blur(selector: string): Promise<void>;
  check(selector: string): Promise<void>;
  uncheck(selector: string): Promise<void>;
  locatorCount(selector: string): Promise<number>;
  isVisible(selector: string): Promise<boolean>;
  isChecked(selector: string): Promise<boolean>;
  mouseClick(x: number, y: number): Promise<void>;
  mouseDblclick(x: number, y: number): Promise<void>;
  mouseMove(x: number, y: number): Promise<void>;
  keyboardPress(key: string): Promise<void>;
  keyboardType(text: string): Promise<void>;
  setInputFiles(selector: string, files: string[]): Promise<void>;
  getByText(text: string, options?: { exact?: boolean }): Promise<string | null>;
  getByLabel(text: string): Promise<string | null>;
  getByRole(role: string, options?: { name?: string }): Promise<string | null>;
  waitForDownload(action: () => Promise<void>): Promise<string>;
}

export interface TestCase {
  name: string;
  fn: (ctx: RunContext) => Promise<Record<string, unknown>>;
}

export interface TestResult {
  name: string;
  status: "pass" | "fail" | "skip";
  data?: Record<string, unknown>;
  error?: string;
  durationMs: number;
}

export interface GoldenFile {
  suite: string;
  recorded_with: string;
  recorded_at: string;
  tests: Record<string, { status: string; data: Record<string, unknown> }>;
}

export interface Suite {
  name: string;
  tests: TestCase[];
}

// ─── Suite Builder ───

export function defineSuite(
  name: string,
  setup: (t: SuiteBuilder) => void
): Suite {
  const builder = new SuiteBuilder(name);
  setup(builder);
  return builder.build();
}

class SuiteBuilder {
  private tests: TestCase[] = [];
  constructor(private name: string) {}

  test(name: string, fn: (ctx: RunContext) => Promise<Record<string, unknown>>) {
    this.tests.push({ name, fn });
  }

  build(): Suite {
    return { name: this.name, tests: this.tests };
  }
}

// ─── Runner ───

export async function runSuite(
  suite: Suite,
  ctx: RunContext
): Promise<TestResult[]> {
  const results: TestResult[] = [];

  for (const tc of suite.tests) {
    const start = Date.now();
    try {
      const data = await tc.fn(ctx);
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

// ─── Golden Recording ───

export function recordGolden(
  suite: Suite,
  results: TestResult[],
  recordedWith: string
): void {
  if (!existsSync(GOLDEN_DIR)) mkdirSync(GOLDEN_DIR, { recursive: true });

  const golden: GoldenFile = {
    suite: suite.name,
    recorded_with: recordedWith,
    recorded_at: new Date().toISOString(),
    tests: {},
  };

  for (const r of results) {
    golden.tests[r.name] = {
      status: r.status,
      data: r.data || {},
    };
  }

  const path = resolve(GOLDEN_DIR, `${suite.name}.golden.json`);
  writeFileSync(path, JSON.stringify(golden, null, 2) + "\n");
}

// ─── Golden Matching ───

export interface MatchResult {
  name: string;
  match: boolean;
  expected?: Record<string, unknown>;
  actual?: Record<string, unknown>;
  error?: string;
}

export function matchGolden(
  suite: Suite,
  results: TestResult[]
): { matches: MatchResult[]; goldenFile: GoldenFile | null } {
  const path = resolve(GOLDEN_DIR, `${suite.name}.golden.json`);

  if (!existsSync(path)) {
    return {
      matches: results.map((r) => ({
        name: r.name,
        match: false,
        error: "No golden file found. Run `pnpm golden:record` first.",
      })),
      goldenFile: null,
    };
  }

  const golden: GoldenFile = JSON.parse(readFileSync(path, "utf-8"));
  const matches: MatchResult[] = [];

  for (const r of results) {
    const expected = golden.tests[r.name];

    if (!expected) {
      matches.push({
        name: r.name,
        match: false,
        error: "Test not found in golden file",
      });
      continue;
    }

    if (r.status === "fail") {
      matches.push({
        name: r.name,
        match: false,
        error: `Test failed: ${r.error}`,
      });
      continue;
    }

    if (expected.status !== "pass") {
      // Both failed or both skipped — match
      matches.push({ name: r.name, match: true });
      continue;
    }

    const isMatch = deepEqual(expected.data, r.data || {});
    matches.push({
      name: r.name,
      match: isMatch,
      expected: isMatch ? undefined : expected.data,
      actual: isMatch ? undefined : r.data,
    });
  }

  return { matches, goldenFile: golden };
}

// ─── Utilities ───

function deepEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== typeof b) return false;
  if (a === null || b === null) return a === b;
  if (typeof a !== "object") return false;

  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    return a.every((v, i) => deepEqual(v, b[i]));
  }

  const aObj = a as Record<string, unknown>;
  const bObj = b as Record<string, unknown>;
  const keysA = Object.keys(aObj).sort();
  const keysB = Object.keys(bObj).sort();

  if (keysA.length !== keysB.length) return false;
  return keysA.every((key, i) => keysB[i] === key && deepEqual(aObj[key], bObj[key]));
}

// ─── Display ───

export function printResults(
  suiteName: string,
  results: TestResult[],
  label: string
): void {
  console.log(`\n  ${label}: ${suiteName}`);
  for (const r of results) {
    if (r.status === "pass") {
      console.log(`    ✅ ${r.name} (${r.durationMs}ms)`);
    } else if (r.status === "skip") {
      console.log(`    ⏭️  ${r.name} (skipped)`);
    } else {
      console.log(`    ❌ ${r.name}: ${r.error}`);
    }
  }
}

export function printMatchResults(
  suiteName: string,
  matches: MatchResult[],
  goldenFile: GoldenFile | null
): void {
  const total = matches.length;
  const passed = matches.filter((m) => m.match).length;

  console.log(`\n  📋 ${suiteName} — ${passed}/${total} match golden`);
  if (goldenFile) {
    console.log(`     (golden recorded with ${goldenFile.recorded_with})`);
  }

  for (const m of matches) {
    if (m.match) {
      console.log(`    ✅ ${m.name}`);
    } else if (m.error) {
      console.log(`    ❌ ${m.name}: ${m.error}`);
    } else {
      console.log(`    ❌ ${m.name}`);
      console.log(`       expected: ${JSON.stringify(m.expected)}`);
      console.log(`       actual:   ${JSON.stringify(m.actual)}`);
    }
  }
}
