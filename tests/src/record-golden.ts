/**
 * Record golden files from Playwright.
 *
 * Usage: pnpm golden:record [spec-name]
 *
 * Requires Chrome with --remote-debugging-port=9222
 */

import { createTestServer } from "./test-server.js";
import { createPlaywrightContext } from "./playwright-adapter.js";
import { runSuite, recordGolden, printResults, type Suite } from "./harness.js";

// Import all spec files
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
  const cdpUrl = process.env.CDP_URL || "http://localhost:9222";

  const suites = specFilter
    ? ALL_SUITES.filter((s) => s.name === specFilter)
    : ALL_SUITES;

  if (suites.length === 0) {
    console.error(`No suite matching "${specFilter}"`);
    console.error(`Available: ${ALL_SUITES.map((s) => s.name).join(", ")}`);
    process.exit(1);
  }

  console.log(`\n🎭 Recording golden files from Playwright`);
  console.log(`   CDP: ${cdpUrl}`);
  console.log(`   Suites: ${suites.map((s) => s.name).join(", ")}\n`);

  const server = await createTestServer();
  console.log(`   Test server: ${server.PREFIX}\n`);

  const { ctx, cleanup } = await createPlaywrightContext(cdpUrl, server);

  let totalTests = 0;
  let totalPassed = 0;

  try {
    for (const suite of suites) {
      const results = await runSuite(suite, ctx);
      printResults(suite.name, results, "🎭 Playwright");

      const passed = results.filter((r) => r.status === "pass").length;
      totalTests += results.length;
      totalPassed += passed;

      recordGolden(suite, results, "playwright@1.58.2");
      console.log(`   → Saved golden/${suite.name}.golden.json`);
    }
  } finally {
    await cleanup();
    await server.close();
  }

  console.log(`\n   📦 ${totalPassed}/${totalTests} recorded across ${suites.length} suites\n`);

  if (totalPassed < totalTests) process.exit(1);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
