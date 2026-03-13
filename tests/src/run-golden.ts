/**
 * Run pwright against golden files.
 *
 * Usage: pnpm test [spec-name]
 *
 * Requires:
 *   1. Chrome with --remote-debugging-port=9222
 *   2. pwright-server running on gRPC port 50051
 *   3. Golden files recorded (pnpm golden:record)
 */

import { createTestServer } from "./test-server.js";
import { createPwrightContext } from "./pwright-adapter.js";
import { runSuite, matchGolden, printResults, printMatchResults, type Suite } from "./harness.js";

import pageGoto from "../specs/page-goto.spec.js";
import pageEvaluate from "../specs/page-evaluate.spec.js";
import pageContent from "../specs/page-content.spec.js";
import locatorQuery from "../specs/locator-query.spec.js";
import locatorClick from "../specs/locator-click.spec.js";
import locatorFill from "../specs/locator-fill.spec.js";
import keyboard from "../specs/keyboard.spec.js";
import mouse from "../specs/mouse.spec.js";
import screenshot from "../specs/screenshot.spec.js";
import download from "../specs/download.spec.js";
import fileUpload from "../specs/file-upload.spec.js";
import locatorText from "../specs/locator-text.spec.js";
import locatorLabel from "../specs/locator-label.spec.js";
import locatorRole from "../specs/locator-role.spec.js";
import locatorFilter from "../specs/locator-filter.spec.js";
import locatorCompose from "../specs/locator-compose.spec.js";
import touchscreen from "../specs/touchscreen.spec.js";
import navigationExtras from "../specs/navigation-extras.spec.js";
import actionExtras from "../specs/action-extras.spec.js";

const ALL_SUITES: Suite[] = [
  pageGoto, pageEvaluate, pageContent,
  locatorQuery, locatorClick, locatorFill,
  keyboard, mouse, screenshot, download, fileUpload,
  locatorText, locatorLabel, locatorRole,
  locatorFilter, locatorCompose, touchscreen,
  navigationExtras, actionExtras,
];

async function main() {
  const specFilter = process.argv[2];
  const grpcAddress = process.env.PWRIGHT_GRPC || "localhost:50051";
  const cdpUrl = process.env.CDP_URL || "http://localhost:9222";

  const suites = specFilter
    ? ALL_SUITES.filter((s) => s.name === specFilter)
    : ALL_SUITES;

  if (suites.length === 0) {
    console.error(`No suite matching "${specFilter}"`);
    console.error(`Available: ${ALL_SUITES.map((s) => s.name).join(", ")}`);
    process.exit(1);
  }

  console.log(`\n🔧 Testing pwright against golden files`);
  console.log(`   gRPC: ${grpcAddress}`);
  console.log(`   CDP: ${cdpUrl}`);
  console.log(`   Suites: ${suites.map((s) => s.name).join(", ")}\n`);

  const server = await createTestServer();
  console.log(`   Test server: ${server.PREFIX}\n`);

  const { ctx, cleanup } = await createPwrightContext(grpcAddress, cdpUrl, server);

  let totalMatched = 0;
  let totalTests = 0;
  let anyFailed = false;

  try {
    for (const suite of suites) {
      const results = await runSuite(suite, ctx);
      printResults(suite.name, results, "🔧 pwright");

      const { matches, goldenFile } = matchGolden(suite, results);
      printMatchResults(suite.name, matches, goldenFile);

      const matched = matches.filter((m) => m.match).length;
      totalMatched += matched;
      totalTests += matches.length;

      if (matched < matches.length) anyFailed = true;
    }
  } finally {
    await cleanup();
    await server.close();
  }

  const pct = totalTests > 0 ? ((totalMatched / totalTests) * 100).toFixed(1) : "0";
  console.log(`\n  🎯 Golden Match: ${totalMatched}/${totalTests} (${pct}%)\n`);

  if (anyFailed) process.exit(1);
  process.exit(0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
