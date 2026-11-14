<!-- agent-updated: 2026-09-22T06:58:47Z -->

# Code Quality Findings

Audit dates: 2026-03-26 (historical audit), 2026-09-21 (current audit)

---

## Current status (2026-09-21)

The March audit below is retained as historical evidence, but its global
conclusions are no longer current. In particular, “no new issues,” “all
workspace tests pass,” and “clean dependency graph” must not be used as present
completion evidence after the Rust v2 and Node-FFI additions.

The current audit found and repaired these concrete defects:

- rejected or synchronously throwing Node session watches could escape as
  unhandled rejections and leave connection state published as live;
- a stale or wrong `.node` binary could satisfy the feature-marker checks
  without any explicit JS/Rust ABI identity;
- `connectOverCDP({ timeout: 0 })` disabled the outer attach timeout but
  silently retained a 30-second per-command timeout;
- Page, Frame and JSHandle evaluation rejected runtime values that the matching
  Locator path stringified;
- CI omitted the package's supported Node versions, the real default addon load
  and the terminal retained-payload profiles;
- the native smoke test used a Worker that could keep the test runner alive
  after a macOS native-load stall; it now uses a bounded child process;
- the two CDP test clients duplicated call recording and persistent response
  storage, while the stateful fake repeated Runtime scripting and the canned
  mock implemented only a partial strict policy;
- `Clock::deadline_from_now` could truncate or wrap large durations, the fake
  clock did not implement a real wait, and all three navigation polling loops
  bypassed clock injection. Deadlines now saturate, `FakeClock::sleep` waits for
  explicit advancement, and DOM/network-idle/selector polling shares the
  injected clock boundary.

Current focused evidence is 172/172 Node package tests, real ABI loading for
default and both terminal retained-payload artifacts, 167/167 bridge library
tests, 30/30 fake library tests, strict release Clippy, formatting, dependency
gates and CI-YAML parsing. This is not a claim that the entire workspace is
currently green: an unrelated pre-existing workspace test process was still
live during the audit, so the new CI definition remains the authoritative full
gate once committed and run.

### Structural findings and resolutions

1. **NODE-004 aggregate Node assembly:** the extracted session-init and Frame
   observation/projection plugins each have an exact five-Pwright-package
   closure; the extracted Frame lifecycle/navigation-wait plugin has an exact
   ten-package closure. Together they reduce aggregate-coupled host files from
   55 to 39. During this migration the default host/addon resolve 236/251
   packages and the aggregate
   has 69 direct normal dependencies; no feature-minimal claim is made.
   Remaining contracts/services still require leaf ownership and a narrow Node
   assembly. See
   [node-ffi-compatibility-design.md](node-ffi-compatibility-design.md#discrepancies-and-unfinished-gates).
2. **Distinct CDP test clients — compatible core resolved:**
   `MockCdpClient` and `FakeCdpClient` currently occupy 979 and 1,967 source lines
   respectively. Their identical call-recording/filtering path has one
   owner in `pwright-fake::CdpCallLog`, selected by
   `pwright-bridge/test-utils` without entering the production bridge closure.
   All 15 canned response fields plus the fake navigation override share
   `CdpResponseSlot` for persistent optional storage.
   The stateful fake's 11 configurable Runtime entry points now also share
   `CdpResponseScript` for error/FIFO/fallback precedence while retaining
   method-local default JSON and gates. One lock now owns the complete script;
   the previous three-lock representation could expose a partial
   reconfiguration to concurrent calls. The canned mock's per-method optional
   defaults and strict behavior remain distinct. `strict()` now rejects every
   absent configurable response slot through one helper, accepts configured
   responses, and leaves fixed built-in responses/no-op methods available.
   Three AAA regressions lock that policy. Method defaults, gates, FIFO/error
   scripts and the in-memory DOM remain intentionally separate; the clients are
   not interchangeable, but no known identical response mechanism remains
   duplicated.
3. **Distribution boundary:** `@shuozeli/pwright` remains private and
   source-built. No prebuilt-platform or npm publication claim is made.

## Historical summary (2026-03-26)

The codebase is in **excellent shape**. The initial audit found 8 categories of
issues. Most were fixed in the same session. The re-audit found no new issues --
the fixes were clean and did not introduce regressions. The remaining items are
either cosmetic, deferred by design, or acceptable for test-only code.

**Scorecard:**
- Fixed: 6 items
- Deferred (intentional): 2 items (MockCdpClient/FakeCdpClient consolidation, Clock inconsistency)
- No action needed: 5 items (cosmetic or test-only)

---

## 1. Unused Dependencies (Medium Impact) -- DONE

### pwright-bridge declares four crates it never imports

- **Location:** `crates/pwright-bridge/Cargo.toml:14-18` (dependencies section)
- **Problem:** `anyhow`, `base64`, `rand`, and `thiserror` are listed as dependencies but no source file under `crates/pwright-bridge/src/` imports or references any of them. This inflates compile times and the dependency graph for no benefit. The crate uses `pwright_cdp::connection::CdpError` for error handling (not `anyhow` or `thiserror`), never decodes base64 (just passes strings through), and has no randomness logic.
- **Fix:** Remove these four lines from `crates/pwright-bridge/Cargo.toml`:
  ```toml
  # Remove:
  anyhow = { workspace = true }
  base64 = { workspace = true }
  rand = { workspace = true }
  thiserror = { workspace = true }
  ```
  Run `cargo check -p pwright-bridge` to verify nothing breaks.
- **Resolution:** Removed all four from `[dependencies]`. Moved `anyhow` to `[dev-dependencies]` since `tests/download_spike.rs` uses it. All tests pass.

---

## 2. Duplication (Medium Impact) -- PARTIALLY DONE

### MockCdpClient and FakeCdpClient duplicate ~1300 lines of CdpClient boilerplate

- **Location:** `crates/pwright-bridge/src/test_utils.rs` (746 lines) and `crates/pwright-fake/src/client.rs` (584 lines)
- **Problem:** Both files implement `CdpClient` with nearly identical boilerplate for every trait method (recording calls, returning canned values). The same ~45 async method signatures are hand-duplicated. When a new method is added to `CdpClient`, both files must be updated in lockstep. This is the largest source of maintenance drag in the codebase.
- **Fix:** Consider one of:
  1. Make `FakeCdpClient` the single test implementation and delete `MockCdpClient` from pwright-bridge. Add the call-recording and canned-response features to `FakeCdpClient`.
  2. Extract a macro or trait-blanket pattern that generates the recording+delegation boilerplate so both impls share the same template.

  Option 1 is simpler and already partially done -- `FakeCdpClient` has `calls()` and `method_names()`.
- **Status:** PARTIAL -- Call recording and the stateful fake's ordered Runtime
  response script are centralized and release-tested. Full client consolidation
  remains a large structural refactor; preserve the canned mock's strict/default
  semantics instead of forcing it through the DOM fake.

### Duplicate click dispatch pattern in actions.rs and mouse.rs -- DONE

- **Location:** `crates/pwright-bridge/src/actions.rs:91-118` (`dispatch_click_at`) and `crates/pwright-bridge/src/playwright/mouse.rs:37-68` (`Mouse::click`)
- **Problem:** Both functions send the same mousePressed/mouseReleased pair with the same parameter structure. `dispatch_click_at` is the low-level helper; `Mouse::click` reimplements it with optional delay support.
- **Fix:** Have `Mouse::click` call `dispatch_click_at` for the actual dispatch, adding only the delay logic around it. This eliminates the duplicated CDP call sequences.
- **Resolution:** Generalized `dispatch_click_at` to accept `MouseButton` (was hardcoded to Left) and made it `pub(crate)`. `Mouse::click` now calls `dispatch_click_at` for the no-delay path; the delay path must inline events since the delay goes between press and release.

---

## 3. Stringly-Typed APIs (Medium Impact) -- DONE

### Browser.setDownloadBehavior uses raw string for behavior parameter -- DONE

- **Location:** `crates/pwright-cdp/src/client_trait.rs:32` and `crates/pwright-cdp/src/domains/browser.rs:8-12`
- **Problem:** The `behavior` parameter is `&str` but CDP only accepts `"deny"`, `"allow"`, `"allowAndName"`, or `"default"`. Passing any other string silently produces a CDP error at runtime. This is a stringly-typed API.
- **Fix:** Define a `DownloadBehavior` enum in `domains/browser.rs`:
  ```rust
  pub enum DownloadBehavior { Deny, Allow, AllowAndName, Default }
  ```
  Change the trait method to accept `DownloadBehavior` instead of `&str`.
- **Resolution:** Added `DownloadBehavior` enum with `as_str()` method. Updated `CdpClient` trait, `CdpSession` impl, macro delegation, `MockCdpClient`, `FakeCdpClient`, and the caller in `Page::wait_for_download`. Re-exported from `pwright_cdp` crate root.

### WaitStrategy variants use string matching for block patterns

- **Location:** `crates/pwright-bridge/src/navigate.rs:222-229`
- **Problem:** `IMAGE_BLOCK_PATTERNS` and `MEDIA_BLOCK_PATTERNS` are `&[&str]` constants. This is fine for now but the media-is-superset-of-images relationship is encoded only in code comments and test assertions, not in types. Low priority.
- **Fix:** No immediate action needed. Mark as cosmetic.
- **Status:** NO ACTION -- cosmetic, as noted in the finding.

---

## 4. Unsafe Patterns -- unwrap() in Non-Test Code (Medium Impact) -- DONE

### unwrap() on tracing directive parse in CLI main -- DONE

- **Location:** `crates/pwright-cli/src/main.rs:367`
- **Problem:** `.add_directive("pwright=info".parse().unwrap())` -- this unwrap will never fail on a hardcoded string, but it violates the fail-fast-via-Result principle and triggers clippy warnings in strict mode.
- **Fix:** Replace with `.add_directive("pwright=info".parse().expect("hardcoded directive"))` or use `unwrap_or_else` with a log message.
- **Resolution:** Replaced `unwrap()` with `expect("hardcoded directive")`.

### unwrap() in MockCdpClient Mutex locks

- **Location:** `crates/pwright-bridge/src/test_utils.rs` (many lines, e.g., 94, 113, 120, 131, 139, ...)
- **Problem:** Every `Mutex::lock().unwrap()` in `MockCdpClient` will panic if the mutex is poisoned (e.g., a test panic while holding the lock). This is test-only code, so the impact is limited -- a poisoned mutex in tests usually indicates a real bug. Low priority.
- **Fix:** Acceptable as-is for test code. No action required.
- **Status:** NO ACTION -- test-only code, acceptable.

### unwrap() on build.rs tonic codegen

- **Location:** `crates/pwright-server/build.rs:4,6`
- **Problem:** `unwrap()` in build scripts. Build scripts are dev-only tools; panicking is the correct behavior if codegen fails.
- **Fix:** Acceptable as-is. No action required.
- **Status:** NO ACTION -- build scripts, acceptable.

---

## 5. Missing Abstraction (Low-Medium Impact) -- DONE

### Repeated `result["result"]["value"]` JSON navigation pattern -- DONE

- **Location:** Throughout the codebase:
  - `crates/pwright-bridge/src/navigate.rs:135-138` (`poll_ready_state`)
  - `crates/pwright-bridge/src/navigate.rs:213-219` (`eval_string`)
  - `crates/pwright-bridge/src/content.rs:47-51` (`get_text`)
  - `crates/pwright-bridge/src/playwright/page.rs:190-195` (`eval_page_string`)
  - `crates/pwright-bridge/src/playwright/locator.rs:228-232` (`text_content`)
  - `crates/pwright-bridge/src/playwright/locator.rs:245-248` (`inner_text`)
  - `crates/pwright-bridge/src/playwright/locator.rs:277-289` (`input_value`)
  - `crates/pwright-bridge/src/playwright/locator.rs:319-323` (`is_disabled`)
  - `crates/pwright-bridge/src/playwright/locator.rs:333-338` (`is_checked`)
- **Problem:** The pattern `result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str()).unwrap_or_default()` (or slight variations) is copy-pasted at least 9 times. Each site manually navigates the CDP `RemoteObject` shape.
- **Fix:** Extract a helper function:
  ```rust
  fn extract_result_value(result: &Value) -> Option<&Value> {
      result.get("result").and_then(|r| r.get("value"))
  }
  ```
  Use it across all call sites. The `FromEvalResult` trait in `evaluate.rs` partially addresses this but isn't used in the low-level bridge functions.
- **Resolution:** Added `pub fn extract_result_value` to `evaluate.rs` and replaced all 9 call sites in `navigate.rs`, `content.rs`, `page.rs`, and `locator.rs`.

---

## 6. Dead / Redundant Code (Low Impact) -- DEFERRED

### `Clock` trait and `FakeClock` are only used by `Locator::wait_for`

- **Location:** `crates/pwright-bridge/src/clock.rs` (154 lines)
- **Problem:** The entire `Clock` abstraction (trait, `TokioClock`, `FakeClock`) exists solely to make `Locator::wait_for` testable with deterministic time. The `navigate.rs` polling loops (`poll_ready_state`, `wait_network_idle`, `wait_selector_visible`) do not use the Clock abstraction -- they use `tokio::time::interval` and `tokio::time::Instant` directly.
- **Fix:** Either:
  1. Migrate the navigate.rs polling loops to use the `Clock` trait for consistency and testability (preferred).
  2. Or document that Clock is Locator-specific and not a general abstraction.

  Currently the inconsistency is confusing -- some waits are time-injectable, others are not.
- **Status:** DEFERRED -- Migrating navigate.rs to use Clock requires threading a Clock parameter through multiple public APIs. Low impact, better as a dedicated task.

### `cookies.rs` is a thin passthrough module

- **Location:** `crates/pwright-bridge/src/cookies.rs` (61 lines)
- **Problem:** `get_cookies` and `set_cookies` are one-line wrappers that delegate directly to `session.network_get_cookies()` / `session.network_set_cookies()`. They add no logic, no error handling, no transformation. Callers could use the `CdpClient` methods directly.
- **Fix:** Mark as cosmetic / low priority. These wrappers exist for API symmetry with `content.rs`, `navigate.rs`, etc. If the module is kept, at least remove the test file which only tests that the passthrough works (it does by construction).
- **Status:** NO ACTION -- cosmetic, kept for API symmetry.

---

## 7. Noted TODO Debt (Informational) -- DONE

### Tab.last_used is never updated -- DONE

- **Location:** `crates/pwright-bridge/src/tab.rs:17-20`
- **Problem:** Already marked with `// TODO(refactor)`. The `last_used` field is set at creation time but never updated when the tab is used. This means `current_tab()` returns the most recently *created* tab, not the most recently *used* one. For the current use case (CLI with one active tab) this is not a bug, but it will cause surprising behavior if multi-tab workflows are added.
- **Fix:** Either rename to `created_at` (accurate) or update on each tab operation (useful).
- **Resolution:** Renamed `last_used` to `created_at` and replaced the TODO with an accurate doc comment.

### max_tabs enforcement removed

- **Location:** `crates/pwright-bridge/src/browser.rs:95-97`
- **Problem:** Already marked with `// TODO(feature)`. Documented decision, not a bug.
- **Fix:** No action needed.
- **Status:** NO ACTION -- documented decision.

---

## 8. Noise / Cosmetic (Low Priority) -- NO ACTION

### `cdp_delegate_impl` macro is used only once

- **Location:** `crates/pwright-cdp/src/client_trait.rs:143-214`
- **Problem:** The `cdp_delegate_impl!` macro exists to generate the `CdpClient for CdpSession` impl. It's invoked exactly once. This is slightly over-abstracted for a single use, but it does make the delegation boilerplate more readable. Borderline cosmetic.
- **Fix:** No action needed -- the macro adds clarity by separating the method-list from the dispatch logic.
- **Status:** NO ACTION.

### `Cow<'static, str>` in KeyDef.code is unnecessary for static keys

- **Location:** `crates/pwright-bridge/src/keys.rs:7`
- **Problem:** For all named keys (Enter, Tab, Escape, etc.), `KeyDef.code` is `Cow::Borrowed(...)`. The `Cow::Owned` variant is used only for F-keys (`F1`-`F12`). This is technically correct but the F-key string could be `&'static str` using a lookup table, avoiding the Cow indirection.
- **Fix:** Cosmetic. The current approach works fine. No action required.
- **Status:** NO ACTION.

---

## Historical re-audit notes (2026-03-26)

A full re-audit of the entire codebase confirmed:

1. **No `#[allow(...)]` directives anywhere** -- all warnings are fixed at the source.
2. **No unwrap() in production code** -- all unwrap calls are in test code, test_utils, or build scripts.
3. **No unused imports or dead code** -- clippy passes clean with zero warnings.
4. **No silent error swallowing** -- all error paths either propagate via `?` or use explicit `map_err`.
5. **Formatting is clean** -- `cargo fmt --check` passes.
6. **All tests pass** -- `cargo test --workspace` green.
7. **Consistent error handling** -- `CdpError` enum covers all error categories with proper `thiserror` derives and Display impls.
8. **Good test coverage** -- every module has unit tests, with fake/mock infrastructure for CDP client injection.
9. **Clean dependency graph** -- no cross-repo path dependencies, no unused workspace dependencies.
10. **Type safety** -- `DownloadBehavior` enum, `SelectorKind` enum, `WaitStrategy` enum, `MouseButton`/`MouseEventType` enums all prevent stringly-typed APIs.
