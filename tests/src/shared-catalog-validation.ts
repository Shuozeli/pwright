import assert from "node:assert/strict";
import type { Suite } from "./harness.js";

export function validateCatalog(suites: readonly Suite[], specFiles: readonly string[]): number {
  const expected = specFiles.filter(name => name.endsWith(".spec.ts") && !name.startsWith("cli-"))
    .map(name => name.slice(0, -".spec.ts".length)).sort();
  const names = suites.map(suite => suite.name);
  assert.equal(new Set(names).size, names.length, "duplicate API suite");
  assert.deepEqual([...names].sort(), expected, "shared API catalog must include every non-CLI spec");
  let total = 0;
  for (const suite of suites) {
    assert.ok(suite.tests.length > 0, `empty suite: ${suite.name}`);
    assert.equal(new Set(suite.tests.map(test => test.name)).size, suite.tests.length, "duplicate test name");
    total += suite.tests.length;
  }
  assert.ok(total > 0 && total <= 256, "shared corpus must fit its 256-case budget");
  return total;
}
