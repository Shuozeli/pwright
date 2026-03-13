/**
 * CLI comparison: snapshot commands
 */
import { defineCliSuite, parseSnapshotLines } from "../src/cli-harness.js";

export default defineCliSuite("cli-snapshot", (t) => {
  t.test("should return snapshot with refs", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/content.html");
    const snap = await cli.exec("snapshot");
    const nodes = parseSnapshotLines(snap.stdout);

    return {
      hasNodes: nodes.length > 0,
      // First node should be the root
      firstRole: nodes[0]?.role ?? "none",
      refCount: nodes.length,
    };
  });

  t.test("should snapshot interactive page", async (cli, serverPrefix) => {
    await cli.exec("open", serverPrefix + "/input.html");
    const snap = await cli.exec("snapshot");
    const nodes = parseSnapshotLines(snap.stdout);

    // Should contain interactive elements
    const hasTextbox = nodes.some((n) => n.role === "textbox");
    const hasButton = nodes.some((n) => n.role === "button");

    return { hasTextbox, hasButton };
  });
});
