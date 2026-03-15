<!-- agent-updated: 2026-03-15 -->
# Bug Report: `with_page` Tab Leak Under CDP Connection Failure

## Status: FIXED (root cause eliminated)

**What was fixed:** `with_page` has been removed entirely. Callers now
use `Browser::new_tab` / `TabHandle::close` for explicit tab lifecycle
management. Close errors are always visible to the caller, and
`TabHandle::target_id()` is available for HTTP-based fallback cleanup.

**What remains open:** When the CDP WebSocket is dead (Chrome under memory
pressure, network hiccup), the `target_close` CDP command still cannot
reach Chrome. The error is now surfaced to the caller, but the tab still
leaks in Chrome. An HTTP-based fallback (`/json/close/{targetId}`) would
fix this -- see proposed fixes below.

## Original Symptom

When using `Browser::with_page()` in a long-running server (llm-web-proxy),
tabs accumulate in Chrome and are never closed. After ~60 minutes of serving
requests, Chrome instances reach 1.4GB+ memory and get OOMKilled.

Observed tab state after 61 minutes (16 ok requests served):

```
chrome-0: 3 tabs  [page] https://gemini.google.com/app
chrome-1: 3 tabs  [page] https://gemini.google.com/app
chrome-4: 2 tabs  [page] https://gemini.google.com/app
chrome-6: 1 tab   [page] https://gemini.google.com/app
chrome-7: 5 tabs  [page] https://gemini.google.com/app
chrome-9: 1 tab   [page] https://gemini.google.com/app
```

15 leaked tabs across 6 Chrome instances. None navigated to `about:blank`.
None closed. Chrome instances that hit 1536MB limit were OOMKilled.

## Root Cause

The original `with_page` silently swallowed close errors with
`let _ = self.browser_session.target_close(&target_id).await`.

This has been refactored: `with_page` now delegates to `TabHandle::close()`
which calls `self.browser_client.target_close()` via the `CdpClient` trait,
and errors are propagated via `combine_results()` / `CdpError::Compound`.

However, when the WebSocket is dead, the CDP command still cannot reach Chrome,
so the tab leaks regardless of error propagation. The caller now knows the
close failed (via the error) and has access to `TabHandle::target_id()` for
HTTP-based fallback cleanup.

## Why This Doesn't Affect Short-Lived Usage

- CLI commands: process exits after one operation, Chrome GC's everything
- Tests: fresh Chrome per test, tabs don't accumulate
- gRPC server (pwright-server): uses tab map + `close_tab()` which also calls
  `target_close`, but the server creates tabs differently (persistent, not ephemeral)

The bug only manifests in **long-running servers** that create many ephemeral
tabs via `with_page` over minutes/hours, where some CDP connections degrade.

## Reproduction

1. Start `chromedp/headless-shell` with `--remote-debugging-port=9222`
2. Connect via `Browser::connect_http("http://localhost:9222")`
3. In a loop, call `browser.with_page(|page| { page.goto("https://gemini.google.com/", None).await; ... })`
4. After ~10-20 iterations, query `http://localhost:9222/json/list`
5. Observe tabs accumulating (should be 0 or 1, will be 5+)

## Proposed Fix

### Option A: HTTP-based tab cleanup (recommended)

After `with_page` completes (success or failure), use Chrome's HTTP DevTools
API to force-close the tab, bypassing the potentially-dead WebSocket:

```rust
// In Browser or as a helper
async fn force_close_tab(cdp_http_url: &str, target_id: &str) {
    let url = format!("{}/json/close/{}", cdp_http_url, target_id);
    let _ = reqwest::get(&url).await; // fire and forget
}
```

This works because Chrome's `/json/close/{targetId}` HTTP endpoint is
independent of the CDP WebSocket -- it always works as long as Chrome's
HTTP debug server is up.

Changes needed:
- `Browser` needs to store the original HTTP CDP URL (currently only stores
  the WebSocket URL after connection)
- `with_page` calls `force_close_tab` after `target_close` as a safety net
- Or: replace `target_close` entirely with the HTTP endpoint

### Option B: Periodic tab garbage collector

Add a method `Browser::close_orphaned_tabs()` that:
1. Queries `/json/list` to get all open tabs
2. Compares against known active tabs (would need a tracking set)
3. Closes any tabs not in the active set

The caller (llm-web-proxy) would call this periodically.

### Option C: Navigate to about:blank in `with_page` before close

Currently the caller does this, but it fails when CDP is dead. Move it into
`with_page` itself, with error handling:

```rust
// Before target_close:
match page.goto("about:blank", None).await {
    Ok(_) => {},
    Err(_) => {
        // CDP dead -- use HTTP fallback
        force_close_tab(&self.cdp_http_url, &target_id).await;
        return result;
    }
}
let _ = self.browser_session.target_close(&target_id).await;
```

## Evidence

### Memory growth timeline (per-Chrome, 10 instances)

```
12:25  1494 MB (peak from load test)
12:30  1013 MB
13:30  3800 MB (10 Chrome total)
14:15  3805 MB
14:40   881 MB (after OOM restarts)
```

### Key files

- `crates/pwright-bridge/src/browser.rs` -- `with_page`, `TabHandle`, `Browser::new_tab`, `combine_results`
- `crates/pwright-bridge/src/tab.rs` -- `close_tab` (used by gRPC server, not `with_page`)
- `crates/pwright-cdp/src/connection.rs` -- `CdpError::Compound`, Drop/shutdown (CancellationToken)

### Consumer code (llm-web-proxy)

- `libservers/experimentation/llm-web-proxy/src/service.rs` -- gRPC handler using `with_page`
- `libservers/experimentation/llm-web-proxy/src/http.rs` -- HTTP handler using `with_page`
- Both navigate to `about:blank` before returning, but this fails when CDP is dead

## Impact

- **OOMKilled**: Chrome instances hit memory limit after 30-60 min of sustained use
- **Auto-reconnect masks the problem**: pool reconnects to restarted Chrome, but leaked tabs repeat
- **Workaround**: Increase Chrome memory limit (current: 1.5GB), but this just delays OOM
