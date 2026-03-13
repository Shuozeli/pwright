/**
 * Record CLI golden files using playwright-cli.
 *
 * Usage: pnpm golden:record:cli [spec-name]
 */

import { createTestServer } from "./test-server.js";
import { createPlaywrightCliRunner } from "./playwright-cli-adapter.js";
import { runCliSuite, type CliSuite } from "./cli-harness.js";
import { recordGolden, printResults } from "./harness.js";

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

  const suites = specFilter
    ? ALL_SUITES.filter((s) => s.name === specFilter)
    : ALL_SUITES;

  if (suites.length === 0) {
    console.error(`No CLI suite matching "${specFilter}"`);
    console.error(`Available: ${ALL_SUITES.map((s) => s.name).join(", ")}`);
    process.exit(1);
  }

  console.log(`\n🎭 Recording CLI golden files (playwright-cli)`);
  console.log(`   Suites: ${suites.map((s) => s.name).join(", ")}\n`);

  const server = await createTestServer();
  console.log(`   Test server: ${server.PREFIX}\n`);

  const cli = createPlaywrightCliRunner();

  let totalTests = 0;
  let totalPassed = 0;

  try {
    for (const suite of suites) {
      // Close any previous session before each suite
      await cli.cleanup().catch(() => {});

      const results = await runCliSuite(suite, cli, server.PREFIX);
      printResults(suite.name, results, "🎭 playwright-cli");

      const passed = results.filter((r) => r.status === "pass").length;
      totalPassed += passed;
      totalTests += results.length;

      // Record golden with suite adapter
      recordGolden(
        { name: suite.name, tests: suite.tests.map((t) => ({ name: t.name, fn: async () => ({}) })) },
        results,
        "playwright-cli"
      );
    }
  } finally {
    await cli.cleanup().catch(() => {});
    await server.close();
  }

  console.log(`\n  🎯 Recorded: ${totalPassed}/${totalTests} passed\n`);
  if (totalPassed < totalTests) process.exit(1);
  process.exit(0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
