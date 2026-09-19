import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";
import type { RunContext, TestCase } from "../src/harness.js";
import { externalFixtureServer } from "../src/external-fixture-server.js";
import { API_SUITES } from "../src/shared-suites.js";
import { validateCatalog } from "../src/shared-catalog-validation.js";
import { sharedInputResolver } from "../src/shared-input-file.js";
import { closeAll, rejectAfterCleanup, type SharedSession } from "../src/shared-lifecycle.js";
import {
  classify, dataEvidence, observeCase, summarize, UnsupportedRunOperation,
  type SideObservation,
} from "../src/shared-differential.js";

const ok = (): SideObservation => ({ operations: ["title"], unsupported: [], browserVersion: "152.0", data: { title: "A" } });
const issue = { name: "Error", message: "fixture failure" };

function fakeSession(title: RunContext["title"], cleanup: () => Promise<void>): SharedSession<RunContext> {
  // A deliberately narrow fake: unexpected browser operations fail, never approximate results.
  const ctx = new Proxy({ title }, {
    get(target, key) {
      if (key === "title") return target.title;
      throw new Error(`Unexpected fake operation: ${String(key)}`);
    },
  }) as RunContext;
  return { ctx, browserVersion: "152.0", cleanup };
}

test("identical successful values match without requiring identical operation traces", () => {
  // Arrange
  const oracle = ok(), candidate = { ...ok(), operations: [] };
  // Act
  const status = classify(oracle, candidate);
  // Assert
  assert.equal(status, "match");
});

for (const [name, oracle, candidate, expected] of [
  ["different values", ok(), { ...ok(), data: { title: "B" } }, "value-mismatch"],
  ["different Chrome versions", ok(), { ...ok(), browserVersion: "153.0" }, "value-mismatch"],
  ["missing observation", ok(), { ...ok(), data: undefined }, "setup-error"],
  ["missing version", ok(), { ...ok(), browserVersion: undefined }, "setup-error"],
  ["both sides fail", { ...ok(), actionError: issue }, { ...ok(), actionError: issue }, "oracle-error"],
  ["oracle error precedes unsupported", { ...ok(), actionError: issue }, { ...ok(), unsupported: ["title"] }, "oracle-error"],
  ["unsupported operation", ok(), { ...ok(), unsupported: ["title"] }, "unsupported"],
  ["candidate failure", ok(), { ...ok(), actionError: issue }, "candidate-error"],
  ["setup failure", { ...ok(), setupError: issue }, ok(), "setup-error"],
  ["cleanup failure precedes equal data", ok(), { ...ok(), cleanupError: issue }, "cleanup-error"],
] as const) {
  test(`${name} cannot be reported as a match`, () => {
    // Arrange
    const left = { ...oracle, operations: [...oracle.operations], unsupported: [...oracle.unsupported] };
    const right = { ...candidate, operations: [...candidate.operations], unsupported: [...candidate.unsupported] };
    // Act
    const status = classify(left, right);
    // Assert
    assert.equal(status, expected);
  });
}

test("caught unsupported calls remain non-passing and cleanup still runs", async () => {
  // Arrange
  let closed = 0;
  const session = fakeSession(async () => { throw new UnsupportedRunOperation("title"); }, async () => { closed++; });
  const fixture: TestCase = { name: "caught", fn: async ctx => {
    try { await ctx.title(); } catch { /* Test the fixture swallowing an unsupported rejection. */ }
    return { title: "A" };
  } };
  // Act
  const result = await observeCase(fixture, async () => session);
  // Assert
  assert.equal(closed, 1);
  assert.deepEqual(result.operations, ["title"]);
  assert.deepEqual(result.unsupported, ["title"]);
  assert.equal(classify(ok(), result), "unsupported");
});

test("action and cleanup failures are both retained", async () => {
  // Arrange
  const session = fakeSession(async () => { throw new Error("action"); }, async () => { throw new Error("cleanup"); });
  const fixture: TestCase = { name: "failure", fn: async ctx => ({ title: await ctx.title() }) };
  // Act
  const result = await observeCase(fixture, async () => session);
  // Assert
  assert.equal(result.actionError?.message, "action");
  assert.equal(result.cleanupError?.message, "cleanup");
  assert.equal(classify(ok(), result), "cleanup-error");
});

test("factory failure is retained as setup error", async () => {
  // Arrange
  const fixture: TestCase = { name: "setup", fn: async () => { throw new Error("must not run"); } };
  // Act
  const result = await observeCase(fixture, async () => { throw new Error("setup"); });
  // Assert
  assert.equal(result.setupError?.message, "setup");
  assert.deepEqual(result.operations, []);
});

test("cleanup attempts later resources after an earlier failure", async () => {
  // Arrange
  const closed: string[] = [];
  const actions = [
    async () => { closed.push("page"); throw new Error("page failure"); },
    async () => { closed.push("context"); throw new Error("context failure"); },
    async () => { closed.push("browser"); },
  ];
  // Act
  const outcome = await closeAll(actions).then(() => undefined, (error: unknown) => error);
  // Assert
  assert.deepEqual(closed, ["page", "context", "browser"]);
  assert.ok(outcome instanceof AggregateError);
  assert.equal(outcome.errors.length, 2);
});

test("partial setup failure retains both the primary and cleanup errors", async () => {
  // Arrange
  const primary = new Error("primary"), cleanup = new Error("cleanup");
  // Act
  const outcome = await rejectAfterCleanup(primary, async () => { throw cleanup; })
    .then(() => undefined, (error: unknown) => error);
  // Assert
  assert.ok(outcome instanceof AggregateError);
  assert.deepEqual(outcome.errors, [primary, cleanup]);
});

test("caught operation budget failures cannot become passing data", async () => {
  // Arrange
  const session = fakeSession(async () => "A", async () => {});
  const fixture: TestCase = { name: "budget", fn: async ctx => {
    for (let index = 0; index < 257; index++) await ctx.title().catch(() => "caught");
    return { title: "A" };
  } };
  // Act
  const result = await observeCase(fixture, async () => session);
  // Assert
  assert.equal(result.operations.length, 256);
  assert.match(result.actionError?.message ?? "", /256-operation budget/);
  assert.equal(classify(ok(), result), "candidate-error");
});

test("evidence preserves undefined, NaN and negative zero for review", () => {
  // Arrange
  const values = { missing: undefined, number: NaN, zero: -0 };
  // Act
  const evidence = dataEvidence(values);
  // Assert
  assert.match(evidence, /undefined/);
  assert.match(evidence, /NaN/);
  assert.match(evidence, /-0/);
});

test("oversize evidence fails the case and still cleans its session", async () => {
  // Arrange
  let closed = false;
  const session = fakeSession(async () => "x".repeat(65536), async () => { closed = true; });
  const fixture: TestCase = { name: "large", fn: async ctx => ({ title: await ctx.title() }) };
  // Act
  const result = await observeCase(fixture, async () => session);
  // Assert
  assert.equal(closed, true);
  assert.match(result.actionError?.message ?? "", /64 KiB budget/);
  assert.equal(result.data, undefined);
});

test("catalog includes every existing API spec exactly once", async () => {
  // Arrange
  const files = await readdir(new URL("../specs", import.meta.url));
  // Act
  const total = validateCatalog(API_SUITES, files);
  // Assert
  assert.equal(total, API_SUITES.reduce((count, suite) => count + suite.tests.length, 0));
});

test("catalog rejects omitted suites", () => {
  // Arrange
  const suites = [{ name: "one", tests: [{ name: "case", fn: async () => ({}) }] }];
  // Act
  const validate = () => validateCatalog(suites, ["one.spec.ts", "two.spec.ts"]);
  // Assert
  assert.throws(validate, /every non-CLI spec/);
});

test("catalog rejects duplicate case names", () => {
  // Arrange
  const fixture = { name: "case", fn: async () => ({}) };
  // Act
  const validate = () => validateCatalog([{ name: "one", tests: [fixture, fixture] }], ["one.spec.ts"]);
  // Assert
  assert.throws(validate, /duplicate test name/);
});

test("external fixtures use their exact registered prefix without a local listener", async () => {
  // Arrange
  const base = "http://fixture.example.ts.net:9384/shared/";
  // Act
  const server = externalFixtureServer(base);
  // Assert
  assert.equal(server.EMPTY_PAGE, "http://fixture.example.ts.net:9384/shared/empty.html");
  assert.throws(() => server.setRoute("/x", () => {}), /dynamic routes/);
  assert.throws(() => server.setRedirect("/x", "/y"), /dynamic redirects/);
  await server.close();
});

for (const url of ["http://127.0.0.1:1234", "http://fixture", "http://user:password@fixture.example.ts.net", "http://fixture.example.ts.net/?query=1"]) {
  test(`external fixture rejects unreviewed URL ${new URL(url).hostname}`, () => {
    // Arrange
    const base = url;
    // Act
    const create = () => externalFixtureServer(base);
    // Assert
    assert.throws(create);
  });
}

test("upload resolution uses an owned file and rejects arbitrary paths", () => {
  // Arrange
  const resolve = sharedInputResolver("/task-owned/test-file.txt");
  // Act
  const file = resolve("/tmp/test-file.txt");
  // Assert
  assert.equal(file, "/task-owned/test-file.txt");
  assert.throws(() => resolve("/tmp/another-file.txt"), /Unregistered/);
});

test("summary retains non-passing statuses instead of counting them as skipped", () => {
  // Arrange
  const rows = ["match", "unsupported", "oracle-error", "cleanup-error"] as const;
  // Act
  const result = summarize(rows.map(status => ({ suite: "one", test: status, status, oracle: ok(), candidate: ok() })));
  // Assert
  assert.equal(result.match, 1);
  assert.equal(result.unsupported, 1);
  assert.equal(result["oracle-error"], 1);
  assert.equal(result["cleanup-error"], 1);
  assert.equal(Object.values(result).reduce((sum, count) => sum + count, 0), 4);
});

test("candidate fixture adapter imports only the public package and test support", async () => {
  // Arrange
  const source = await readFile(new URL("../src/node-ffi-adapter.ts", import.meta.url), "utf8");
  // Act
  const imports = [...source.matchAll(/from\s+"([^"]+)"/g)].map(match => match[1]).sort();
  // Assert
  assert.deepEqual(imports, [
    "./harness.js", "./shared-differential.js", "./shared-lifecycle.js", "./test-server.js",
    "@shuozeli/pwright", "node:assert/strict", "node:timers/promises",
  ].sort());
  assert.doesNotMatch(source, /\bfetch\s*\(|\bWebSocket\b|\bcreateTestServer\s*\(/);
});
