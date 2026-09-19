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
import { runSuite, matchGolden, printResults, printMatchResults } from "./harness.js";

import { API_SUITES as ALL_SUITES } from "./shared-suites.js";

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
