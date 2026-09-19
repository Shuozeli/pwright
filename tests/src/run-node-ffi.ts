/** Run the unchanged shared API corpus on two independent, owned Contexts per case. */
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { appendFile, open, readFile, readdir, unlink, writeFile } from "node:fs/promises";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { API_SUITES } from "./shared-suites.js";
import { validateCatalog } from "./shared-catalog-validation.js";
import { externalFixtureServer } from "./external-fixture-server.js";
import { sharedInputResolver } from "./shared-input-file.js";
import { createPlaywrightContext } from "./playwright-adapter.js";
import { createNodeFfiContext } from "./node-ffi-adapter.js";
import { closeAll } from "./shared-lifecycle.js";
import {
  classify, dataEvidence, diagnostic, observeCase, summarize,
  type SharedComparison, type SideObservation,
} from "./shared-differential.js";

const testsRoot = fileURLToPath(new URL("../", import.meta.url));
const repoRoot = resolve(testsRoot, "..");
const require = createRequire(import.meta.url);
const { nativeArtifact }: { nativeArtifact: { path: string; profile: string } } =
  require("../../packages/pwright/src/native-artifact.cjs");
const sha256 = (value: Uint8Array) => createHash("sha256").update(value).digest("hex");

function required(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`Required configuration: ${name}`);
  return value;
}

function reportSide(side: SideObservation) {
  const { data, ...rest } = side;
  return { ...rest, ...(data ? { dataInspect: dataEvidence(data) } : {}) };
}

async function main(): Promise<void> {
  assert.equal(process.argv.length, 2, "this gate runs the entire API corpus; filters are not accepted");
  const cdp = required("PWRIGHT_TEST_CDP");
  assert.equal(cdp, "http://127.0.0.1:9222", "use only the dedicated loopback Chrome profile");
  const server = externalFixtureServer(required("PWRIGHT_SHARED_FIXTURES"));
  const output = required("PWRIGHT_SHARED_OUTPUT");
  assert.ok(isAbsolute(output), "PWRIGHT_SHARED_OUTPUT must be an absolute, new directory");
  const oraclePackage: unknown = require("playwright/package.json");
  assert.ok(oraclePackage && typeof oraclePackage === "object" && "version" in oraclePackage);
  assert.equal(oraclePackage.version, "1.58.2", "shared oracle version must match the reviewed pin");
  const expectedCases = validateCatalog(API_SUITES, await readdir(join(testsRoot, "specs")));
  const startedAt = new Date().toISOString();
  const fixtureHashes: Record<string, string> = {};
  // Exact bytes prevent an expired/misdirected serve-lib route from looking like parity.
  for (const name of (await readdir(join(testsRoot, "fixtures"))).sort()) {
    const local = await readFile(join(testsRoot, "fixtures", name));
    const response = await fetch(`${server.PREFIX}/${encodeURIComponent(name)}`, {
      signal: AbortSignal.timeout(5000), redirect: "error",
    });
    assert.equal(response.status, 200, `fixture status: ${name}`);
    assert.match(response.headers.get("content-type") ?? "", /text\/html/i, `fixture MIME: ${name}`);
    assert.equal(sha256(new Uint8Array(await response.arrayBuffer())), sha256(local), `fixture bytes: ${name}`);
    fixtureHashes[name] = sha256(local);
  }
  const sourceHashes: Record<string, string> = {};
  for (const dir of ["src", "specs"]) {
    for (const name of (await readdir(join(testsRoot, dir))).sort()) {
      if (name.endsWith(".ts")) sourceHashes[`${dir}/${name}`] = sha256(await readFile(join(testsRoot, dir, name)));
    }
  }
  const metadata = {
    schema: "pwright.shared-node-ffi.v1", startedAt, cdp, fixtures: server.PREFIX,
    oracle: `playwright@${oraclePackage.version}`, candidate: "@shuozeli/pwright local release addon",
    node: process.version, platform: process.platform, arch: process.arch,
    gitHead: execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot, encoding: "utf8" }).trim(),
    gitStatus: execFileSync("git", ["status", "--short"], { cwd: repoRoot, encoding: "utf8", maxBuffer: 1024 * 1024 }),
    nativeSha256: sha256(await readFile(nativeArtifact.path)), nativeProfile: nativeArtifact.profile,
    fixtureHashes, sourceHashes, expectedSuites: API_SUITES.length, expectedCases,
    limits: {
      connectTimeoutMs: 5000, oracleActionTimeoutMs: 5000, candidateGotoTimeoutMs: 5000,
      candidateReadTimeout: "native CDP command budget; no public read timeout option",
      operationsPerSide: 256, evidenceBytesPerSide: 65536, cases: 256,
      supervisorTimeoutMs: 600000, supervisorLogBytes: 2097152,
    },
    limitations: [
      "Test corpus observations are not a complete Playwright compatibility specification.",
      "Download fixture observes an event and returns its historical placeholder, not artifact contents.",
      "Upload alias /tmp/test-file.txt maps to a new owned file, never an existing machine file.",
      "Touchscreen corpus currently uses mouseClick, not the Playwright touchscreen API.",
      "getByLabel adapter observes a tag name; screenshots observe headers/size, not pixel parity.",
      "Native hash identifies the tested binary; git status alone does not prove it matches all source edits.",
    ],
  };
  await writeFile(join(output, "metadata.json"), JSON.stringify(metadata, null, 2) + "\n", { flag: "wx" });
  const rows: SharedComparison[] = [];
  const upload = join(output, "test-file.txt");
  const casesFile = await open(join(output, "cases.jsonl"), "wx");
  let uploadCreated = false;
  try {
    await writeFile(upload, "Pwright shared upload fixture\n", { flag: "wx", mode: 0o600 });
    uploadCreated = true;
    for (const suite of API_SUITES) {
      for (const test of suite.tests) {
        console.log(JSON.stringify({ event: "case-start", suite: suite.name, test: test.name }));
        const oracle = await observeCase(test, () => createPlaywrightContext(cdp, server, {
          connectTimeoutMs: 5000, actionTimeoutMs: 5000, resolveInputFile: sharedInputResolver(upload),
        }));
        const candidate = await observeCase(test, () => createNodeFfiContext(cdp, server, {
          resolveInputFile: sharedInputResolver(upload),
        }));
        const row = { suite: suite.name, test: test.name, status: classify(oracle, candidate), oracle, candidate };
        rows.push(row);
        await casesFile.write(JSON.stringify({ ...row, oracle: reportSide(oracle), candidate: reportSide(candidate) }) + "\n");
        console.log(JSON.stringify({ event: "case-end", suite: row.suite, test: row.test, status: row.status,
          unsupported: candidate.unsupported }));
      }
    }
  } finally {
    await closeAll([
      () => casesFile.close(),
      async () => { if (uploadCreated) await unlink(upload); },
    ]);
  }
  assert.equal(rows.length, expectedCases);
  const summary = summarize(rows);
  const report = {
    ...metadata, completedAt: new Date().toISOString(), complete: true,
    summary, passed: summary.match === expectedCases,
    cases: rows.map(row => ({ ...row, oracle: reportSide(row.oracle), candidate: reportSide(row.candidate) })),
  };
  await writeFile(join(output, "report.json"), JSON.stringify(report, null, 2) + "\n", { flag: "wx" });
  console.log(JSON.stringify({ event: "summary", expectedCases, summary, report: join(output, "report.json") }));
  if (!report.passed) process.exitCode = 1;
}

main().catch(async (error: unknown) => {
  const failure = diagnostic(error);
  console.error(JSON.stringify({ event: "runner-error", error: failure }));
  const output = process.env.PWRIGHT_SHARED_OUTPUT;
  if (output) await appendFile(join(output, "fatal.jsonl"), JSON.stringify(failure) + "\n").catch((writeError: unknown) => {
    console.error("Could not write failure evidence", writeError);
  });
  process.exitCode = 2;
});
