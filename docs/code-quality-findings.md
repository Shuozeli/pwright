# Code Quality Findings

Audit performed 2026-03-19 against commit d8fbf98.

## 1. Duplication

### gRPC error conversion boilerplate -- DONE
- **Location:** `crates/pwright-server/src/service/*.rs` (40+ occurrences)
- **Problem:** Every CDP error was mapped to `Status::internal` regardless of error kind.
- **Fix applied:** Added `cdp_to_status()` helper in `service.rs` that maps `CdpError` variants to correct gRPC codes (`Timeout` -> `DEADLINE_EXCEEDED`, `Closed` -> `UNAVAILABLE`, `ElementNotFound` -> `NOT_FOUND`, etc.). All 40+ `map_err` closures replaced with `.map_err(cdp_to_status)`.

### Node ID validation repeated in execute_action match arms -- DONE
- **Location:** `crates/pwright-server/src/service/actions.rs`
- **Problem:** 10 match arms repeated `node_id.ok_or_else(|| Status::invalid_argument("ref required for ACTION"))`.
- **Fix applied:** Extracted `require_node_id()` helper in `service.rs`, used across all 10 match arms.

### Touch tap implemented three times -- PARTIAL
- **Location:** `crates/pwright-bridge/src/playwright/touchscreen.rs:18-26`, `page.rs:464-479`, `crates/pwright-server/src/service/actions.rs`
- **Fix applied:** `Page::tap()` now delegates to `self.touchscreen().tap(x, y)` instead of inlining the CDP calls. Server handler still inlines (acceptable since it operates at a different abstraction level).

### Keyboard down/up branching -- DONE
- **Location:** `crates/pwright-bridge/src/playwright/keyboard.rs`
- **Fix applied:** Extracted private `dispatch_key()` helper. `down()` and `up()` delegate to it.

### Executor error emission near-duplicate -- DONE
- **Location:** `crates/pwright-script/src/executor.rs`
- **Fix applied:** Unified the two error branches into one `sink.emit()` call followed by a conditional `return`.

## 2. Stringly-Typed APIs

### CDP event type strings (mouse/keyboard/touch)
- **Location:** 30+ hardcoded string literals across `actions.rs`, `mouse.rs`, `keyboard.rs`, `touchscreen.rs`, `page.rs`, server `actions.rs`
- **Problem:** Typos compile fine but produce silent runtime misbehavior.
- **Fix:** Define `MouseEventType`, `KeyEventType`, `TouchEventType` enums in pwright-cdp.
- **Status:** Deferred -- large cross-crate change, requires updating CdpClient trait signatures.

### Mouse button as Option<String>
- **Location:** `crates/pwright-bridge/src/playwright/mouse.rs:12`
- **Status:** Deferred -- low priority, contained to one module.

### ExecutionResult.status as String
- **Location:** `crates/pwright-script/src/executor.rs:326`
- **Status:** Deferred -- low priority, internal to script runner.

## 3. Silent Failures

### Reload ignores poll_ready_state result -- DONE
- **Location:** `crates/pwright-server/src/service/navigation.rs`
- **Fix applied:** Changed `let _ =` to `if let Err(e) = ... { tracing::warn!(...) }`.

### JsonlSink drops serialization and write errors
- **Location:** `crates/pwright-script/src/output.rs:67-69,75-77`
- **Status:** Deferred -- low priority, would require changing `OutputSink` trait signature.

### State file permission failure silenced
- **Location:** `crates/pwright-cli/src/state.rs:75`
- **Status:** Deferred -- very low priority.

## 4. Missing Abstractions

### Proto conversion boilerplate (no From impls)
- **Location:** `crates/pwright-server/src/service/{content,data,tabs}.rs`
- **Status:** Deferred -- medium effort, 4 impl blocks needed. Worth doing when adding new proto fields.

### set_cookies uses json!() instead of typed struct
- **Location:** `crates/pwright-server/src/service/data.rs:76-91`
- **Status:** Deferred -- requires changing bridge's `set_cookies` signature to accept typed cookies.

## 5. Unsafe Patterns

### #[allow(unused_imports)] removed -- DONE
- **Location:** `crates/pwright-server/src/service/content.rs:3`
- **Fix applied:** Removed the `#[allow(unused_imports)]` line. The `use base64::Engine as _` import remains (needed for method resolution).

### root_node_id uses unwrap_or(1) fallback
- **Location:** `crates/pwright-bridge/src/playwright/selectors.rs:15-20`
- **Status:** Deferred -- low-medium priority, would require updating 3 call sites to handle Result.

### Numeric cast: f64 to u64 for timeout
- **Location:** `crates/pwright-server/src/service/navigation.rs:52`
- **Status:** Deferred -- low priority, proto field is always non-negative in practice.

## 6. Dead Code / Stubs

### Placeholder test file
- **Location:** `crates/pwright-cli/tests/download_command.rs:1-6`
- **Status:** Deferred -- intentional placeholder.

### MockCdpClient missing setters
- **Location:** `crates/pwright-bridge/src/test_utils.rs`
- **Status:** Deferred -- add when tests need these responses.

## 7. Noise

### Duplicate doc comment -- DONE
- **Location:** `crates/pwright-script/src/executor.rs`
- **Fix applied:** Removed the duplicate line, kept the more informative version.
