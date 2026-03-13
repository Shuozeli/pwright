/**
 * CLI comparison: action commands (click, fill, screenshot)
 */
import { defineCliSuite, parseSnapshotLines, normalizeEvalOutput, stripAnsi } from "../src/cli-harness.js";

export default defineCliSuite("cli-actions", (t) => {
  t.test("should click button via ref", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/interactive.html");
    const snap = await cli.exec("snapshot");
    const nodes = parseSnapshotLines(snap.stdout);

    const btn = nodes.find((n) => n.role === "button");
    if (!btn) throw new Error("no button found in snapshot");

    await cli.exec("click", btn.ref);

    const result = await cli.exec("eval", "document.getElementById('click-count')?.textContent ?? 'missing'");
    return { clicked: normalizeEvalOutput(result.stdout) };
  });

  t.test("should fill input via ref", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/input.html");
    const snap = await cli.exec("snapshot");
    const nodes = parseSnapshotLines(snap.stdout);

    const input = nodes.find((n) => n.role === "textbox");
    if (!input) throw new Error("no textbox found in snapshot");

    await cli.exec("fill", input.ref, "hello world");

    const result = await cli.exec("eval", "document.querySelector('input[type=text], input[type=email], input:not([type])')?.value ?? ''");
    return { value: normalizeEvalOutput(result.stdout) };
  });

  t.test("should screenshot to file", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");

    const filename = `/tmp/cli-test-${Date.now()}.png`;
    await cli.exec("screenshot", `--filename=${filename}`);

    const { existsSync, statSync } = await import("fs");
    const exists = existsSync(filename);
    const size = exists ? statSync(filename).size : 0;

    return { exists, sizeAboveZero: size > 0 };
  });
});
