# Critical Review Findings - 2026-03-29

Adversarial review of codebase and test methodology. Ordered by priority.

## P0 — Fix Now

### 1. Tautological server action tests
**File:** `pwright-server/src/service/tests.rs:463-863`
**Status:** Fixed
**Issue:** 11 action tests used `if let Some` guards that silently passed if ref cache was empty.
**Fix applied:** Replaced with `.expect()` calls. Also enhanced FakeCdpClient to return DOM-derived accessibility tree from `accessibility_get_full_tree()` so snapshot produces real ref cache entries.

### 2. Hover/drag coordinate bug (real production bug)
**File:** `pwright-bridge/src/actions.rs` — `hover_by_node_id`, `drag_by_node_id`
**Status:** Fixed
**Issue:** Used `DOM.getBoxModel` content coords (page-absolute). `dispatchMouseEvent` expects viewport-relative coords.
**Fix applied:** Changed `hover_by_node_id` and `drag_by_node_id` to use `get_element_center_js` (JS getBoundingClientRect, viewport-relative) — consistent with `click_by_node_id`.

### 3. Template injection in script runner
**File:** `pwright-script/src/executor.rs:105,127-128,197-199`
**Status:** Documented (by design)
**Issue:** User params interpolated directly into CSS selectors, JS expressions, and URLs.
**Resolution:** Script runner is a trusted-input tool (user authors both script and params). Added `// SAFETY:` comments at every callsite. Added detailed doc comment on `resolve_template` explaining the design decision and recommending `args` (typed CDP arguments) over template interpolation for untrusted values. Aligns with CLAUDE.md rule against heuristic preprocessing.

## P1 — Fix Soon

### 4. Wait tests verify mock, not behavior
**File:** `pwright-bridge/tests/fake_browser.rs:344-449`
**Status:** Open
**Issue:** `wait_for_text_finds_existing_text` pre-sets evaluate_response to true — HTML content is irrelevant. Test passes with empty HTML. Same for `wait_for_text_in_selector_succeeds` and `wait_until_true_expression`.
**Fix:** Don't pre-set evaluate_response. Either enhance FakeCdpClient to evaluate text presence against the DOM, or test with integration tests.

### 5. Navigation timeout is misleading
**File:** `pwright-bridge/src/navigate.rs:78`
**Status:** Fixed
**Issue:** `page_navigate` used CDP default timeout (30s) regardless of `opts.timeout`.
**Fix applied:** Wrapped `page_navigate` in `tokio::time::timeout(opts.timeout, ...)` so the user's timeout applies to the entire navigation, not just the post-nav wait.

### 6. Stale nodeIds after navigation
**File:** `pwright-server/src/service/navigation.rs`
**Status:** Fixed
**Issue:** Ref cache not invalidated on navigation. Stale nodeIds caused confusing errors.
**Fix applied:** Added `browser.delete_ref_cache(&tab.tab_id)` after navigate, reload, go_back, go_forward in the server handlers.

### 7. No WebSocket reconnection
**File:** `pwright-cdp/src/connection.rs`
**Status:** Accepted (known limitation)
**Issue:** Chrome restart = permanent failure until manual `ConnectBrowser`.
**Resolution:** Automatic reconnection is a feature with its own failure modes (session state loss, in-flight command handling, tab map invalidation). The current behavior is correct — the server returns `FailedPrecondition` and the client retries `ConnectBrowser`. Documented as a known limitation.

### 8. Tab leak on failed close
**File:** `pwright-bridge/src/tab.rs:53-74`
**Status:** Fixed
**Issue:** `close_tab` removed tab from map before calling `target_close`. If close failed, tab leaked.
**Fix applied:** Reordered — read target_id first, call `target_close`, only remove from map on success. On failure, tab stays in map for retry.

## P2 — Cleanup

### 9. `assert!(true)` in list_tabs test
**File:** `pwright-server/src/service/tests.rs` — `list_tabs_returns_page_targets`
**Status:** Fixed
**Fix applied:** Replaced with meaningful assertion.

### 10. FakeCdpClient eval_property_check is fragile
**File:** `pwright-fake/src/client.rs:105-131`
**Status:** Fixed
**Issue:** Matched on JS string substrings — changing JS formatting broke fake silently.
**Fix applied:** Changed to exact equality checks against `pwright_js::element` constants (IS_CHECKED, IS_DISABLED, GET_TEXT_CONTENT, GET_INNER_TEXT, GET_INPUT_VALUE). Added pwright-js as dependency. blur/focus kept as substring match since they're called via various patterns.

### 11. Browser is a god object
**File:** `pwright-bridge/src/browser.rs`
**Status:** Documented (deferred)
**Issue:** 9 fields of state, 5 responsibilities.
**Fix applied:** Added doc comment noting potential extraction points (RefCacheStore, TabPool) for when the struct grows further. Not worth refactoring now.

### 12. executor.rs has zero inline tests
**File:** `pwright-script/src/executor.rs`
**Status:** Fixed
**Issue:** Core execution logic with no unit tests.
**Fix applied:** Added 18 inline tests covering: resolve_template (basic substitution, multiple/adjacent vars, empty string, unknown vars, empty var name, unclosed braces, triple braces, no-reprocessing, integer/boolean/null values, whitespace handling), json_value_to_string (string/number/bool/null/missing), ExecutionStatus serialization.

### 13. Locking order not enforced in server
**File:** `pwright-server/src/service.rs:111-138`
**Status:** Fixed (documented)
**Issue:** Semaphore → tab_lock order not documented.
**Fix applied:** Added doc comment on `resolve_tab_locked` documenting the locking order and stating it's the canonical locking point — do not duplicate.
