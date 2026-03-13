/**
 * CLI comparison: keyboard commands (type, press)
 */
import { defineCliSuite, normalizeEvalOutput } from "../src/cli-harness.js";

export default defineCliSuite("cli-keyboard", (t) => {
  t.test("should type text into focused input", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/input.html");
    // Focus the text input via eval
    await cli.exec("eval", "document.getElementById('text').focus()");
    await cli.exec("type", "hello");
    const result = await cli.exec("eval", "document.getElementById('text').value");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should press Enter key", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/interactive.html");
    await cli.exec("eval", "document.getElementById('focus-input').focus()");
    await cli.exec("press", "Enter");
    // Verify key was dispatched via keyboard-output
    const result = await cli.exec("eval", "document.getElementById('keyboard-output').textContent");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should press Tab key", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/input.html");
    await cli.exec("eval", "document.getElementById('text').focus()");
    await cli.exec("press", "Tab");
    // After Tab, focus should move to next input
    const result = await cli.exec("eval", "document.activeElement?.id ?? 'none'");
    return { focusedId: normalizeEvalOutput(result.stdout) };
  });

  t.test("should press Escape key", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/interactive.html");
    await cli.exec("press", "Escape");
    const result = await cli.exec("eval", "document.getElementById('keyboard-output').textContent");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should type multiple characters sequentially", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/input.html");
    await cli.exec("eval", "document.getElementById('text').focus()");
    await cli.exec("type", "abc123");
    const result = await cli.exec("eval", "document.getElementById('text').value");
    return { value: normalizeEvalOutput(result.stdout) };
  });
});
