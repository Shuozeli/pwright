# pwright Golden Tests

Golden test framework that records **Playwright's** output as snapshots, then validates **pwright** produces identical results.

## Quick Start

```bash
# Prerequisites: Chrome + pwright-server running
google-chrome --headless --no-sandbox --remote-debugging-port=9222 &
cargo run --bin pwright-server &

# Install deps
pnpm install

# Record golden files from Playwright
pnpm golden:record

# Test pwright against goldens
pnpm test
```

## Commands

| Command | Description |
|---------|-------------|
| `pnpm golden:record` | Run all specs with Playwright, save results → `golden/` |
| `pnpm golden:record <suite>` | Re-record a single suite (e.g. `page-goto`) |
| `pnpm test` | Run all specs with pwright, compare against goldens |
| `pnpm test <suite>` | Test a single suite against its golden |
| `pnpm test:unit` | Run unit tests for harness utilities |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CDP_URL` | `http://localhost:9222` | Chrome CDP endpoint |
| `PWRIGHT_GRPC` | `localhost:50051` | pwright gRPC server address |

## Project Structure

```
tests/
├── specs/                      # Golden test suites (17 files)
│   ├── page-goto.spec.ts            Navigation tests
│   ├── page-evaluate.spec.ts        JS evaluation tests
│   ├── page-content.spec.ts         Content extraction tests
│   ├── locator-query.spec.ts        Locator query tests
│   ├── locator-click.spec.ts        Click action tests
│   ├── locator-fill.spec.ts         Fill/type action tests
│   ├── keyboard.spec.ts             Keyboard input tests
│   ├── mouse.spec.ts                Mouse input tests
│   ├── screenshot.spec.ts           Screenshot tests
│   └── cli-*.spec.ts                CLI comparison tests (8 files)
│
├── golden/                     # Recorded Playwright snapshots (committed)
│   └── *.golden.json                One per spec suite
│
├── fixtures/                   # Static HTML test pages
│   ├── empty.html
│   ├── input.html
│   ├── content.html
│   └── interactive.html
│
├── src/                        # Framework internals
│   ├── harness.ts                   Suite/test builders, golden record/match
│   ├── cli-harness.ts               CLI-specific harness
│   ├── test-server.ts               HTTP server for fixtures
│   ├── playwright-adapter.ts        RunContext → Playwright APIs
│   ├── playwright-cli-adapter.ts    RunContext → Playwright CLI
│   ├── pwright-adapter.ts           RunContext → pwright gRPC
│   ├── pwright-cli-adapter.ts       RunContext → pwright CLI
│   ├── record-golden.ts             CLI: pnpm golden:record
│   ├── record-golden-cli.ts         CLI: pnpm golden:record:cli
│   ├── run-golden.ts                CLI: pnpm test
│   └── run-golden-cli.ts            CLI: pnpm test:cli
│
└── unit/                       # Unit tests for harness utilities
    ├── test-utils.ts                Minimal test runner (no deps)
    └── cli-harness.test.ts          Parser/normalizer unit tests
```

## How It Works

### 1. Golden Recording (`pnpm golden:record`)

Runs each spec suite through **Playwright** and saves the results as JSON.

### 2. Golden Testing (`pnpm test`)

Runs the same specs through **pwright** (gRPC), then deep-compares each test's `data` against the golden snapshot.

### 3. Adding Tests

Create a new spec file in `specs/`:

```typescript
import { defineSuite } from "../src/harness.js";

export default defineSuite("my-feature", (t) => {
  t.test("should do something", async (ctx) => {
    await ctx.goto(ctx.server.PREFIX + "/content.html");
    const title = await ctx.title();
    return { title };
  });
});
```

Then register it in both `src/record-golden.ts` and `src/run-golden.ts`.

## Current Coverage

**38 tests across 9 API suites — 100% golden match** ✅

```
📋 page-goto        — 5/5  ✅
📋 page-evaluate    — 5/5  ✅
📋 page-content     — 4/4  ✅
📋 locator-query    — 6/6  ✅
📋 locator-click    — 4/4  ✅
📋 locator-fill     — 4/4  ✅
📋 keyboard         — 4/4  ✅
📋 mouse            — 4/4  ✅
📋 screenshot       — 2/2  ✅
```
