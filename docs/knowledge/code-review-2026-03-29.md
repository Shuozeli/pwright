# Code Review Findings - 2026-03-29

Full AI-generated codebase review. Issues ordered by severity.

## High Severity

### 1. CDP timeout race condition
**File:** `pwright-cdp/src/connection.rs`
**Status:** Not a bug (standard timeout semantics)
**Issue:** Response discarded after timeout fires. Once the caller gets `Timeout`, the late response has nowhere to go — this is how timeouts work.
**Fix applied:** Added warn log when reader receives response for timed-out/unknown command. Reader loop now cancels shutdown token on exit so pending commands fail fast with `Closed` instead of hanging.

### 2. Navigation error hiding
**File:** `pwright-bridge/src/navigate.rs:76-84`
**Status:** Not a bug (matches Playwright)
**Issue:** HTTP 404/500 treated as successful navigation. This is correct — matches Playwright behavior where the page loaded and callers check response status separately.
**Fix applied:** Added documentation comment explaining Playwright compatibility.

### 3. Network listener resource leak
**File:** `pwright-bridge/src/playwright/page.rs`
**Status:** Fixed
**Issue:** `Page::drop()` used `try_lock()`. tokio Mutex doesn't poison (original concern was wrong), but `get_mut()` is more correct since Drop has `&mut self`.
**Fix applied:** Changed from `try_lock()` to `get_mut()`.

### 4. Script async detection is fragile
**File:** `pwright-script/src/parser.rs`
**Status:** Fixed
**Issue:** Auto-detected async by scanning JS body text. Any heuristic is fragile — smarter text matching just creates different edge cases.
**Fix applied:** Removed auto-detection entirely. Scripts default to sync. Users declare `async: true` explicitly via object form: `my_script: { body: "await fetch(...)", async: true }`. Added CLAUDE.md rule against preprocessing user-supplied JavaScript.

## Medium Severity

### 5. Ref cache not cleaned on Page::close()
**File:** `pwright-bridge/src/playwright/page.rs`
**Status:** Documented (design constraint)
**Issue:** Page doesn't have access to Browser, can't clean ref cache. Stale entries are harmless.
**Fix applied:** Added doc comment directing users to `Browser::close_tab()` for full cleanup.

### 6. Silent error swallowing in navigation
**File:** `pwright-bridge/src/navigate.rs:91-101`
**Status:** Accepted behavior
**Issue:** navigate() returns empty URL/title when JS eval fails. Navigation itself succeeded — the metadata is best-effort. Warn logs already present. Failing the whole call because title extraction failed would be worse.

### 7. Broadcast channel drops events silently
**File:** `pwright-cdp/src/connection.rs`
**Status:** Fixed
**Fix applied:** Changed `let _ = event_tx.send(event)` to log at warn level when send fails.

### 8. Spawned task panics unobserved
**File:** `pwright-cdp/src/connection.rs`
**Status:** Fixed
**Fix applied:** Reader loop now cancels the shutdown token on exit, so pending commands fail fast with `Closed` instead of hanging until timeout.

### 9. Silent template variable substitution
**File:** `pwright-script/src/executor.rs`
**Status:** Fixed
**Fix applied:** Added `tracing::warn!` log for unresolved template variables.

### 10. Coordinate zero-check footgun
**File:** `pwright-bridge/src/actions.rs`
**Status:** Fixed
**Fix applied:** Replaced `unwrap_or(0.0)` sentinel pattern with `Option`-based coordinate extraction. Falls back to JS only when box model data is genuinely missing, not when element is at (0,0).

## Low Severity

### 11. Hard-coded channel capacities
**File:** `pwright-cdp/src/connection.rs`
**Status:** Fixed
**Fix applied:** Added `ConnectionConfig` struct with configurable `write_channel_capacity` and `event_channel_capacity`. Added `connect_with_config()` method. Existing `connect()` and `connect_with_timeout()` use defaults.

### 12. Fake box model returns identical coordinates
**File:** `pwright-fake/src/client.rs`
**Status:** Deferred
**Reason:** Test infrastructure improvement, not a production issue.

### 13. Fake can't update input values after creation
**File:** `pwright-fake/src/client.rs`
**Status:** Deferred
**Reason:** Test infrastructure improvement, not a production issue.

### 14. Inconsistent domain method patterns
**File:** `pwright-cdp/src/domains/`
**Status:** Deferred
**Reason:** Refactor risk outweighs benefit. Working code with no bugs.

### 15. Code duplication
**Status:** Deferred
**Reason:** Premature abstraction risk. Three similar lines are better than a wrong abstraction.

### 16. Screenshot quality not validated
**File:** `pwright-server/src/service/content.rs`
**Status:** Fixed
**Fix applied:** Quality values now clamped to 0-100 range.
