/**
 * Run pwright CLI against CLI golden files.
 *
 * Usage: pnpm test:cli [spec-name]
 */

import { createTestServer } from "./test-server.js";
import { createPwrightCliRunner } from "./pwright-cli-adapter.js";
import { runCliSuite, type CliSuite } from "./cli-harness.js";
import { matchGolden, printResults, printMatchResults } from "./harness.js";

import cliNavigation from "../specs/cli-navigation.spec.js";
import cliSnapshot from "../specs/cli-snapshot.spec.js";
import cliEval from "../specs/cli-eval.spec.js";
import cliActions from "../specs/cli-actions.spec.js";
import cliKeyboard from "../specs/cli-keyboard.spec.js";
import cliHistory from "../specs/cli-history.spec.js";
import cliEvalAdvanced from "../specs/cli-eval-advanced.spec.js";
import cliContent from "../specs/cli-content.spec.js";

const ALL_SUITES: CliSuite[] = [
  cliNavigation, cliSnapshot, cliEval, cliActions,
  cliKeyboard, cliHistory, cliEvalAdvanced, cliContent,
];

async function main() {
  const specFilter = process.argv[2];
  const cdpUrl = process.env.CDP_URL || "http://localhost:9222";

  const suites = specFilter
    ? ALL_SUITES.filter((s) => s.name === specFilter)
    : ALL_SUITES;

  if (suites.length === 0) {
    console.error(`No CLI suite matching "${specFilter}"`);
    console.error(`Available: ${ALL_SUITES.map((s) => s.name).join(", ")}`);
    process.exit(1);
  }

  console.log(`\n🔧 Testing pwright CLI against golden files`);
  console.log(`   CDP: ${cdpUrl}`);
  console.log(`   Suites: ${suites.map((s) => s.name).join(", ")}\n`);

  const server = await createTestServer();
  console.log(`   Test server: ${server.PREFIX}\n`);

  const cli = createPwrightCliRunner(cdpUrl);

  let totalMatched = 0;
  let totalTests = 0;
  let anyFailed = false;

  try {
    for (const suite of suites) {
      // Clean state before each suite
      await cli.cleanup().catch(() => {});

      const results = await runCliSuite(suite, cli, server.PREFIX);
      printResults(suite.name, results, "🔧 pwright");

      // Match against golden — use a compatible Suite object
      const suiteLike = {
        name: suite.name,
        tests: suite.tests.map((t) => ({ name: t.name, fn: async () => ({}) })),
      };
      const { matches, goldenFile } = matchGolden(suiteLike, results);
      printMatchResults(suite.name, matches, goldenFile);

      const matched = matches.filter((m) => m.match).length;
      totalMatched += matched;
      totalTests += matches.length;

      if (matched < matches.length) anyFailed = true;
    }
  } finally {
    await cli.cleanup().catch(() => {});
    await server.close();
  }

  const pct = totalTests > 0 ? ((totalMatched / totalTests) * 100).toFixed(1) : "0";
  console.log(`\n  🎯 CLI Golden Match: ${totalMatched}/${totalTests} (${pct}%)\n`);

  if (anyFailed) process.exit(1);
  process.exit(0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
