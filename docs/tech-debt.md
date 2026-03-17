# Tech Debt

Structural issues from AI-generated code that need human-directed refactoring.
These are not bugs — the code works. They are maintainability and design issues
that accumulate when code is generated function-by-function without cross-cutting
awareness.

## Boilerplate Explosion

### CdpClient trait delegation (185 lines)
`client_trait.rs:128-313` — 50+ methods that all just delegate to CdpSession.
Cannot use `macro_rules!` due to `#[async_trait]` lifetime expansion conflict.
Would need a proc macro crate to solve properly.

### MockCdpClient (722 lines)
`test_utils.rs` — 15 setter methods and 60+ trait impls that are all identical
patterns. Should be macro-generated.

### gRPC handlers (21 handlers)
`service/*.rs` — All start with the same `get_browser -> resolve_tab_locked`
boilerplate. Should extract a helper or macro.

### CLI commands (40+ functions)
`commands.rs` — All follow `connect -> resolve_tab -> do_thing -> output::ok`.
A macro or closure could eliminate the repetition.

## Copy-Paste Patterns

### ~~Root node ID extraction~~ FIXED
Extracted `root_node_id()` helper in selectors.rs, used by all call sites.

### ~~on_request/on_response~~ FIXED
Extracted generic `subscribe_network_event<T>()` method.

## Stringly-Typed APIs (partially fixed)

### Fixed
- [x] `GotoOptions.wait_until` -> `WaitUntil` enum
- [x] `ScreenshotOptions.format` -> `ImageFormat` enum
- [x] Script model: `on_error: String` -> `OnError` enum, `param_type: String` -> `ParamType` enum

### Remaining
- [ ] Selector encoding: `__pw_text=`, `__pw_label=` etc. Should be `SelectorKind` enum (31 occurrences across 3 files — architectural change)

## Test Coverage Illusion

### Well-tested (easy to test)
- String manipulation, mock passthrough, locator resolution (151+ tests)

### Not tested (hard to test)
- `navigate.rs` (363 lines): all 5 wait strategies — zero unit tests
- `tab.rs` (149 lines): create/close/reattach/resolve — zero unit tests
- `chrome_http.rs` (164 lines): HTTP endpoints — zero unit tests
- `touchscreen.rs`: entire module — zero unit tests

These are only covered by integration tests which run against real Chrome.

## Over-engineering

### pwright-fake (1,643 lines)
Full HTML parser + DOM tree + CSS selector engine, used by exactly one test
file (`pwright-script/tests/execute_script.rs`). The investment is
disproportionate to the test surface it enables.

## Priority

Focus refactoring on the copy-paste patterns first — they cause the most
maintenance burden. The boilerplate explosion is annoying but stable (it
doesn't change often). The test coverage gap is addressed by integration tests.
