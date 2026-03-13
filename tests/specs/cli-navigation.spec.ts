/**
 * CLI comparison: navigation commands
 */
import { defineCliSuite, normalizeEvalOutput } from "../src/cli-harness.js";

export default defineCliSuite("cli-navigation", (t) => {
  t.test("should open and navigate", async (cli, serverPrefix) => {
    const open = await cli.exec("open", serverPrefix + "/content.html");
    const evalResult = await cli.exec("eval", "document.title");
    return {
      openOk: open.exitCode === 0,
      title: normalizeEvalOutput(evalResult.stdout),
    };
  });

  t.test("should goto new URL", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/empty.html");
    await cli.exec("goto", serverPrefix + "/content.html");
    const evalResult = await cli.exec("eval", "document.title");
    return {
      title: normalizeEvalOutput(evalResult.stdout),
    };
  });

  t.test("should reload page", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    await cli.exec("reload");
    const evalResult = await cli.exec("eval", "document.title");
    return {
      title: normalizeEvalOutput(evalResult.stdout),
    };
  });
});
