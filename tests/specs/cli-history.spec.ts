/**
 * CLI comparison: browser history (go-back, go-forward)
 */
import { defineCliSuite, normalizeEvalOutput } from "../src/cli-harness.js";

export default defineCliSuite("cli-history", (t) => {
  t.test("should go back after navigation", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    await cli.exec("goto", serverPrefix + "/input.html");
    await cli.exec("go-back");
    // Small delay for navigation
    await new Promise((r) => setTimeout(r, 500));
    const result = await cli.exec("eval", "document.title");
    return { title: normalizeEvalOutput(result.stdout) };
  });

  t.test("should go forward after going back", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    await cli.exec("goto", serverPrefix + "/input.html");
    await cli.exec("go-back");
    await new Promise((r) => setTimeout(r, 500));
    await cli.exec("go-forward");
    await new Promise((r) => setTimeout(r, 500));
    const result = await cli.exec("eval", "document.title");
    return { title: normalizeEvalOutput(result.stdout) };
  });

  t.test("should survive go-back at start of history", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const back = await cli.exec("go-back");
    // Should not crash
    return { ok: back.exitCode === 0 };
  });
});
