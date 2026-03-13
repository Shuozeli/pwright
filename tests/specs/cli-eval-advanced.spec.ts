/**
 * CLI comparison: eval edge cases
 */
import { defineCliSuite, normalizeEvalOutput } from "../src/cli-harness.js";

export default defineCliSuite("cli-eval-advanced", (t) => {
  t.test("should evaluate boolean true", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "true");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate boolean false", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "false");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate undefined", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "undefined");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate array", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "JSON.stringify([1, 2, 3])");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate DOM query", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "document.getElementById('heading').textContent");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate window dimensions", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "typeof window.innerWidth");
    return { type: normalizeEvalOutput(result.stdout) };
  });

  t.test("should handle eval with quotes", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "'single quotes'");
    return { value: normalizeEvalOutput(result.stdout) };
  });
});
