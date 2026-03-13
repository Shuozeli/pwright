/**
 * Unit tests for cli-harness normalizers and parsers.
 * Targeting 100% branch coverage.
 */

import { describe, it, assert } from "./test-utils.js";
import {
  stripAnsi,
  normalizeEvalOutput,
  normalizeOutput,
  parseSnapshotRefs,
  parseSnapshotLines,
  defineCliSuite,
  runCliSuite,
} from "../src/cli-harness.js";

// ═══════════════════════════════════════════
// stripAnsi
// ═══════════════════════════════════════════

describe("stripAnsi", () => {
  it("removes single ANSI code", () => {
    assert(stripAnsi("\u001b[32mgreen\u001b[0m") === "green");
  });

  it("removes multi-param ANSI codes", () => {
    assert(stripAnsi("\u001b[1;31mbold red\u001b[0m") === "bold red");
  });

  it("returns plain text unchanged", () => {
    assert(stripAnsi("hello world") === "hello world");
  });

  it("handles empty string", () => {
    assert(stripAnsi("") === "");
  });

  it("handles string with only ANSI codes", () => {
    assert(stripAnsi("\u001b[32m\u001b[0m") === "");
  });

  it("handles multiple adjacent codes", () => {
    const input = "\u001b[32m INFO\u001b[0m \u001b[3murl\u001b[0m\u001b[2m=\u001b[0m\"ws://localhost\"";
    const result = stripAnsi(input);
    assert(!result.includes("\u001b"), `still has ANSI: ${result}`);
    assert(result.includes("INFO"), `missing INFO: ${result}`);
    assert(result.includes("ws://localhost"), `missing url: ${result}`);
  });
});

// ═══════════════════════════════════════════
// parseSnapshotRefs
// ═══════════════════════════════════════════

describe("parseSnapshotRefs", () => {
  it("parses pwright refs [eN]", () => {
    const refs = parseSnapshotRefs("[e0] RootWebArea\n  [e1] heading\n  [e2] button");
    assert(refs.length === 3);
    assert(refs[0] === "e0" && refs[1] === "e1" && refs[2] === "e2");
  });

  it("parses playwright-cli refs [ref=eN]", () => {
    const refs = parseSnapshotRefs('- generic [active] [ref=e1]:\n  - button "Click" [ref=e2]');
    assert(refs.length === 2);
    assert(refs[0] === "e1" && refs[1] === "e2");
  });

  it("returns empty for no refs", () => {
    assert(parseSnapshotRefs("no refs here").length === 0);
  });

  it("returns empty for empty string", () => {
    assert(parseSnapshotRefs("").length === 0);
  });

  it("handles mixed pwright and playwright-cli lines", () => {
    // [eN] takes priority — if line has [eN] it won't check [ref=eN]
    const refs = parseSnapshotRefs("[e0] heading\n- button [ref=e5]");
    assert(refs.length === 2);
    assert(refs[0] === "e0");
    assert(refs[1] === "e5");
  });

  it("skips lines with only brackets but no ref pattern", () => {
    const refs = parseSnapshotRefs("[active] generic\n[focused] textbox");
    assert(refs.length === 0, `expected 0, got ${refs.length}`);
  });
});

// ═══════════════════════════════════════════
// parseSnapshotLines
// ═══════════════════════════════════════════

describe("parseSnapshotLines", () => {
  // pwright format
  it("parses pwright with name", () => {
    const nodes = parseSnapshotLines('[e0] RootWebArea "Example Domain"');
    assert(nodes.length === 1);
    assert(nodes[0].ref === "e0");
    assert(nodes[0].role === "RootWebArea");
    assert(nodes[0].name === "Example Domain");
  });

  it("parses pwright without name", () => {
    const nodes = parseSnapshotLines("[e1] textbox");
    assert(nodes.length === 1);
    assert(nodes[0].ref === "e1");
    assert(nodes[0].role === "textbox");
    assert(nodes[0].name === "");
  });

  it("parses multiple pwright lines with indentation", () => {
    const input = '[e0] RootWebArea "Page"\n  [e1] heading "Title"\n  [e2] button "Submit"';
    const nodes = parseSnapshotLines(input);
    assert(nodes.length === 3);
    assert(nodes[2].name === "Submit");
  });

  // playwright-cli format
  it("parses playwright-cli with name", () => {
    const nodes = parseSnapshotLines('- button "Click Me" [ref=e2]');
    assert(nodes.length === 1);
    assert(nodes[0].ref === "e2");
    assert(nodes[0].role === "button");
    assert(nodes[0].name === "Click Me");
  });

  it("parses playwright-cli without name", () => {
    const nodes = parseSnapshotLines("- textbox [ref=e3]");
    assert(nodes.length === 1);
    assert(nodes[0].ref === "e3");
    assert(nodes[0].role === "textbox");
    assert(nodes[0].name === "");
  });

  it("parses playwright-cli with [active] modifier", () => {
    const nodes = parseSnapshotLines("- generic [active] [ref=e1]:");
    assert(nodes.length === 1);
    assert(nodes[0].ref === "e1");
    assert(nodes[0].role === "generic");
  });

  // Edge cases
  it("skips empty lines", () => {
    const nodes = parseSnapshotLines("\n\n[e0] heading\n\n");
    assert(nodes.length === 1);
  });

  it("skips whitespace-only lines", () => {
    const nodes = parseSnapshotLines("   \n  \t  \n[e0] heading");
    assert(nodes.length === 1);
  });

  it("skips lines matching neither format", () => {
    const nodes = parseSnapshotLines("some random text\n### Snapshot\n[e0] heading\nmore text");
    assert(nodes.length === 1);
    assert(nodes[0].ref === "e0");
  });

  it("handles empty input", () => {
    assert(parseSnapshotLines("").length === 0);
  });
});

// ═══════════════════════════════════════════
// normalizeEvalOutput
// ═══════════════════════════════════════════

describe("normalizeEvalOutput", () => {
  // playwright-cli branches
  it("extracts from playwright-cli ### Result block", () => {
    const input = '### Result\n"Content Page"\n### Ran Playwright code\n```js\nawait page.evaluate(...);\n```';
    assert(normalizeEvalOutput(input) === '"Content Page"');
  });

  it("extracts number from playwright-cli", () => {
    assert(normalizeEvalOutput("### Result\n6\n### Ran Playwright code") === "6");
  });

  it("extracts null from playwright-cli", () => {
    assert(normalizeEvalOutput("### Result\nnull\n### Ran Playwright code") === "null");
  });

  it("handles ### Result as last line (no value after)", () => {
    // resultIdx >= 0 but resultIdx + 1 >= lines.length → falls to pwright branch
    // "### Result" doesn't match any pwright noise filter, so it's returned
    const result = normalizeEvalOutput("### Result");
    assert(result === "### Result", `got: "${result}"`);
  });

  // pwright branches — each noise filter
  it("strips INFO lines from pwright", () => {
    assert(normalizeEvalOutput("INFO something\n42") === "42");
  });

  it("strips WARN lines from pwright", () => {
    assert(normalizeEvalOutput("WARN something\n42") === "42");
  });

  it("strips Connected line", () => {
    assert(normalizeEvalOutput("Connected to Chrome\n42") === "42");
  });

  it("strips Tab: line", () => {
    assert(normalizeEvalOutput("Tab: tab_00000000\n42") === "42");
  });

  it("strips URL: line", () => {
    assert(normalizeEvalOutput("URL: https://example.com\n42") === "42");
  });

  it("strips 'connecting to Chrome' line", () => {
    assert(normalizeEvalOutput("connecting to Chrome url=ws://...\n42") === "42");
  });

  it("strips 'connected to Chrome' line", () => {
    assert(normalizeEvalOutput("connected to Chrome successfully\n42") === "42");
  });

  it("strips 'tab re-attached' line", () => {
    assert(normalizeEvalOutput('tab re-attached tab_id="tab_00000000"\n42') === "42");
  });

  it("strips 'tab created' line", () => {
    assert(normalizeEvalOutput('tab created tab_id="tab_00000000"\n42') === "42");
  });

  it("strips ✅ line", () => {
    assert(normalizeEvalOutput("✅ Clicked [e1]\n42") === "42");
  });

  it("strips 📸 line", () => {
    assert(normalizeEvalOutput("📸 screenshot.png (1234 bytes)\n42") === "42");
  });

  it("strips 📄 line", () => {
    assert(normalizeEvalOutput("📄 page.pdf (5678 bytes)\n42") === "42");
  });

  it("returns empty for empty input", () => {
    assert(normalizeEvalOutput("") === "");
  });

  it("returns empty when all lines are noise", () => {
    const input = "INFO foo\nWARN bar\nConnected to Chrome\nTab: xxx\nURL: yyy";
    assert(normalizeEvalOutput(input) === "");
  });

  it("extracts last meaningful line from multiple results", () => {
    const result = normalizeEvalOutput("INFO noise\nfirst\nsecond");
    assert(result === "second", `expected "second", got "${result}"`);
  });

  it("handles full pwright output with ANSI codes", () => {
    const input = '\u001b[32m INFO\u001b[0m connecting to Chrome \u001b[3murl\u001b[0m\u001b[2m=\u001b[0m"ws://localhost"\n\u001b[32m INFO\u001b[0m connected to Chrome successfully\n\u001b[32m INFO\u001b[0m tab re-attached\n"hello"';
    assert(normalizeEvalOutput(input) === '"hello"');
  });
});

// ═══════════════════════════════════════════
// normalizeOutput
// ═══════════════════════════════════════════

describe("normalizeOutput", () => {
  // pwright noise
  it("strips INFO lines", () => {
    assert(normalizeOutput("INFO foo\nresult") === "result");
  });

  it("strips WARN lines", () => {
    assert(normalizeOutput("WARN bar\nresult") === "result");
  });

  it("strips 'connecting to Chrome'", () => {
    assert(normalizeOutput("connecting to Chrome url=...\nresult") === "result");
  });

  it("strips 'connected to Chrome'", () => {
    assert(normalizeOutput("connected to Chrome successfully\nresult") === "result");
  });

  it("strips 'tab re-attached'", () => {
    assert(normalizeOutput("tab re-attached tab_id=xxx\nresult") === "result");
  });

  it("strips 'tab created'", () => {
    assert(normalizeOutput("tab created tab_id=xxx\nresult") === "result");
  });

  // playwright-cli noise
  it("strips ### Browser line", () => {
    assert(normalizeOutput("### Browser `default` opened\nresult") === "result");
  });

  it("strips - default: line", () => {
    assert(normalizeOutput("- default:\nresult") === "result");
  });

  it("strips - browser-type: line", () => {
    assert(normalizeOutput("- browser-type: chrome\nresult") === "result");
  });

  it("strips - user-data-dir: line", () => {
    assert(normalizeOutput("- user-data-dir: <in-memory>\nresult") === "result");
  });

  it("strips - headed: line", () => {
    assert(normalizeOutput("- headed: false\nresult") === "result");
  });

  it("strips --- separator", () => {
    assert(normalizeOutput("---\nresult") === "result");
  });

  it("strips ### Ran Playwright line", () => {
    assert(normalizeOutput("### Ran Playwright code\nresult") === "result");
  });

  it("strips ``` code fence", () => {
    assert(normalizeOutput("```js\nresult\n```") === "result");
  });

  it("handles ANSI codes in input", () => {
    assert(normalizeOutput("\u001b[32mINFO\u001b[0m foo\nresult") === "result");
  });

  it("returns empty for all-noise input", () => {
    const input = "INFO foo\nWARN bar\n### Browser opened\n- default:";
    assert(normalizeOutput(input) === "");
  });

  it("preserves multiple meaningful lines joined by newline", () => {
    const result = normalizeOutput("INFO noise\nline1\nline2");
    assert(result === "line1\nline2", `got: ${result}`);
  });

  it("strips empty and whitespace-only lines", () => {
    const result = normalizeOutput("   \n\nresult\n   ");
    assert(result === "result");
  });
});

// ═══════════════════════════════════════════
// defineCliSuite
// ═══════════════════════════════════════════

describe("defineCliSuite", () => {
  it("creates suite with correct name", () => {
    const suite = defineCliSuite("my-suite", () => {});
    assert(suite.name === "my-suite");
  });

  it("creates suite with tests", () => {
    const suite = defineCliSuite("test-suite", (t) => {
      t.test("test1", async () => ({ a: 1 }));
      t.test("test2", async () => ({ b: 2 }));
    });
    assert(suite.tests.length === 2);
    assert(suite.tests[0].name === "test1");
    assert(suite.tests[1].name === "test2");
  });

  it("creates empty suite", () => {
    const suite = defineCliSuite("empty", () => {});
    assert(suite.tests.length === 0);
  });
});

// ═══════════════════════════════════════════
// runCliSuite
// ═══════════════════════════════════════════

describe("runCliSuite (async)", () => {
  // These are async tests, handled in main()
});

// ─── Runner ───

async function runTests() {
  const { runAll, report } = await import("./test-utils.js");
  let results = runAll();

  // Run async tests separately
  console.log("\n  runCliSuite (async)");

  // Test: passing test
  {
    const suite = defineCliSuite("pass-suite", (t) => {
      t.test("passes", async () => ({ ok: true }));
    });
    const mockCli = {
      exec: async () => ({ stdout: "", stderr: "", exitCode: 0 }),
      cleanup: async () => {},
    };
    const res = await runCliSuite(suite, mockCli, "http://localhost");
    assert(res.length === 1);
    assert(res[0].status === "pass");
    assert((res[0].data as any).ok === true);
    console.log("    ✅ handles passing test");
    results.passed++;
  }

  // Test: failing test
  {
    const suite = defineCliSuite("fail-suite", (t) => {
      t.test("fails", async () => { throw new Error("boom"); });
    });
    const mockCli = {
      exec: async () => ({ stdout: "", stderr: "", exitCode: 0 }),
      cleanup: async () => {},
    };
    const res = await runCliSuite(suite, mockCli, "http://localhost");
    assert(res.length === 1);
    assert(res[0].status === "fail");
    assert(res[0].error === "boom");
    console.log("    ✅ handles failing test");
    results.passed++;
  }

  // Test: empty suite
  {
    const suite = defineCliSuite("empty", () => {});
    const mockCli = {
      exec: async () => ({ stdout: "", stderr: "", exitCode: 0 }),
      cleanup: async () => {},
    };
    const res = await runCliSuite(suite, mockCli, "http://localhost");
    assert(res.length === 0);
    console.log("    ✅ handles empty suite");
    results.passed++;
  }

  // Test: multiple tests in suite
  {
    const suite = defineCliSuite("multi", (t) => {
      t.test("t1", async () => ({ n: 1 }));
      t.test("t2", async () => ({ n: 2 }));
      t.test("t3", async () => { throw new Error("oops"); });
    });
    const mockCli = {
      exec: async () => ({ stdout: "", stderr: "", exitCode: 0 }),
      cleanup: async () => {},
    };
    const res = await runCliSuite(suite, mockCli, "http://localhost");
    assert(res.length === 3);
    assert(res[0].status === "pass" && res[1].status === "pass" && res[2].status === "fail");
    assert(res[0].durationMs >= 0);
    console.log("    ✅ handles mix of pass/fail");
    results.passed++;
  }

  report(results);
  process.exit(results.failed > 0 ? 1 : 0);
}

runTests();
