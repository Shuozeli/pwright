/**
 * Minimal test utilities — no external deps needed.
 */

type TestFn = () => void | Promise<void>;

interface Suite {
  name: string;
  tests: { name: string; fn: TestFn }[];
}

const suites: Suite[] = [];
let currentSuite: Suite | null = null;

export function describe(name: string, fn: () => void) {
  const suite: Suite = { name, tests: [] };
  currentSuite = suite;
  fn();
  suites.push(suite);
  currentSuite = null;
}

export function it(name: string, fn: TestFn) {
  if (!currentSuite) throw new Error("it() must be called inside describe()");
  currentSuite.tests.push({ name, fn });
}

export function assert(condition: boolean, message?: string) {
  if (!condition) throw new Error(message ?? "Assertion failed");
}

export function runAll(): { passed: number; failed: number; errors: string[] } {
  let passed = 0;
  let failed = 0;
  const errors: string[] = [];

  for (const suite of suites) {
    console.log(`\n  ${suite.name}`);
    for (const test of suite.tests) {
      try {
        const result = test.fn();
        if (result instanceof Promise) {
          throw new Error("Async tests not supported in sync runner");
        }
        console.log(`    ✅ ${test.name}`);
        passed++;
      } catch (err: any) {
        console.log(`    ❌ ${test.name}: ${err.message}`);
        failed++;
        errors.push(`${suite.name} > ${test.name}: ${err.message}`);
      }
    }
  }

  return { passed, failed, errors };
}

export function report(results: { passed: number; failed: number; errors: string[] }) {
  const total = results.passed + results.failed;
  console.log(`\n  🎯 ${results.passed}/${total} passed`);
  if (results.failed > 0) {
    console.log(`  ❌ ${results.failed} failed`);
  }
}
