/**
 * CLI comparison: eval commands
 */
import { defineCliSuite, normalizeEvalOutput } from "../src/cli-harness.js";

export default defineCliSuite("cli-eval", (t) => {
  t.test("should evaluate arithmetic", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "1 + 2 + 3");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate string", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const result = await cli.exec("eval", "document.title");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate null", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "null");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should evaluate object", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    const result = await cli.exec("eval", "JSON.stringify({a:1,b:2})");
    return { value: normalizeEvalOutput(result.stdout) };
  });
});
