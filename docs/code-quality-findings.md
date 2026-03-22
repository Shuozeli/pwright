# Code Quality Findings

Audit date: 2026-03-22

All 22 issues resolved: 21 fixed, 1 skipped (codegen unwraps -- dev-only tool).

---

## 1. Duplication (High Impact) -- ALL DONE

### 1.1 Repeated `resolve_node -> extract objectId` pattern -- DONE
- **Fix:** Extracted `resolve_to_object_id()` helper in `actions.rs`. Removed unused `selectors::resolve_object_id`.

### 1.2 Repeated CDP result value extraction in Page -- DONE
- **Fix:** Extracted `eval_page_string()` in `page.rs`.

### 1.3 Duplicated click dispatch sequence -- DONE
- **Fix:** Extracted `dispatch_click_at()` in `actions.rs`.

### 1.4 Near-duplicate generated CDP code in integration tests -- DONE
- **Fix:** Removed orphaned `tests/integration/crates/pwright-cdp/` (10K+ lines). Integration tests already depended on workspace crate.

---

## 2. Silent Failures (High Impact) -- ALL DONE

### 2.1 `unwrap_or_default()` swallowing deserialization failures -- DONE
- **Fix:** Replaced with `.map_err(CdpError::Json)?` in `network.rs`, `target.rs`, `accessibility.rs`.

### 2.2 `unwrap_or_default()` in `runtime_call_function_on` -- DONE
- **Fix:** Proper error propagation via `map_err(CdpError::Json)`.

### 2.3 Navigate swallows URL/title extraction errors -- DONE
- **Fix:** Added `tracing::warn!()` on extraction failure.

---

## 3. Inconsistency / Potential Bugs (High Impact) -- ALL DONE

### 3.1 Server check/uncheck is not idempotent -- DONE
- **Fix:** Added `is_checked()` guard. Added `actions::is_checked()` helper.

### 3.2 `go_back` / `go_forward` implemented differently -- DONE
- **Fix:** Server now delegates to `Page::go_back()` / `Page::go_forward()` (CDP history API).

### 3.3 `Mouse::dblclick` dispatches wrong event sequence -- DONE
- **Fix:** Now sends correct 4-event sequence matching `dblclick_by_node_id`.

### 3.4 `Page::dblclick` bypasses Locator pattern -- DONE
- **Fix:** Added `Locator::dblclick()`, `Page::dblclick()` delegates to it.

### 3.5 `set_input_files` uses `unwrap_or(1)` for root node ID -- DONE
- **Fix:** Returns proper error via `Status::internal()`.

---

## 4. Missing Abstractions (Medium Impact)

### 4.1 Stringly-typed `behavior` in `browser_set_download_behavior` -- SKIPPED
- Low risk, only called internally with correct strings. Requires trait signature change.

### 4.2 Stringly-typed `reason` in `fetch_fail_request` -- SKIPPED
- Same reasoning as 4.1.

### 4.3 Duplicate wait strategy / screenshot format enums -- DONE
- **Fix:** Removed `WaitUntil` and `ImageFormat` from `page.rs`. Now uses `WaitStrategy` and `ScreenshotFormat` directly. Updated all callers across CLI, script executor, and integration tests.

---

## 5. Dead / Unused Code (Medium Impact) -- ALL DONE

### 5.1 `test_utils` module compiled in release builds -- DONE
- **Fix:** Gated behind `#[cfg(any(test, feature = "test-utils"))]`. Example crates enable the feature.

### 5.2 `BrowserConfig::max_tabs` is never enforced -- DONE
- **Fix:** Removed the field.

### 5.3 `Tab.created_at` and `Tab.last_used` not meaningfully used -- DONE
- **Fix:** Removed `created_at`. Added TODO on `last_used`.

---

## 6. Unsafe Patterns (Medium Impact)

### 6.1 `unwrap()` in production script parser -- DONE
- **Fix:** Replaced with `.ok_or_else(|| ScriptError::Parse(...))?`.

### 6.2 `unwrap()` calls in codegen binary -- DONE
- **Fix:** Changed `generate_domain_module` and all internal functions to return `fmt::Result`. Replaced ~30 `.unwrap()` calls with `?`. Caller uses `.expect()` with descriptive message.

---

## 7. Error Handling (Medium Impact) -- DONE

### 7.1 `CdpError::Other` used as catch-all -- DONE
- **Fix:** Added `PageClosed`, `TabNotFound(String)`, `HttpFailed(String)`, `JsException(String)` variants. Updated ~17 call sites and gRPC status mapper.

---

## 8. Noise / Low Priority -- DONE

### 8.1 Redundant doc comments -- DONE
- **Fix:** Removed redundant doc comments across all CDP domain files. Kept comments with genuine edge-case documentation.

### 8.2 Trivial `cookies.rs` delegation -- SKIPPED
- Left as-is. Maintains API symmetry with other bridge modules.

### 8.3 `.take()` on local variable -- SKIPPED
- Cosmetic. No behavior impact.
