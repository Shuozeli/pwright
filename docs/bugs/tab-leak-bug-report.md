<!-- agent-updated: 2026-03-15 -->
# Bug Report: `with_page` Tab Leak Under CDP Connection Failure

## Status: FIXED (root cause eliminated)

**What was fixed:**

1. `with_page` removed entirely. Callers manage tab lifecycle explicitly.
2. `TabCloser` trait abstracts the close transport. `Browser::connect_http`
   auto-selects `HttpTabCloser` which uses Chrome's HTTP debug endpoint
   (`GET /json/close/{targetId}`) -- this works even when the CDP WebSocket
   is dead under Chrome memory pressure.
3. `ChromeHttpClient` provides HTTP-based `list_targets`, `close_target`,
   `create_target`, and `version` for direct Chrome management.

## Original Symptom

When using `Browser::with_page()` in a long-running server, tabs accumulate
in Chrome and are never closed. After ~60 minutes of serving requests,
Chrome instances reach 1.4GB+ memory and get OOMKilled.

Example tab state after sustained use:

```
instance-0: 3 tabs leaked
instance-1: 3 tabs leaked
instance-2: 2 tabs leaked
instance-3: 5 tabs leaked
```

Leaked tabs are never navigated to `about:blank` or closed.

## Root Cause

The original `with_page` silently swallowed close errors with
`let _ = self.browser_session.target_close(&target_id).await`.

This was a two-part problem:

1. **Error swallowing:** `with_page` discarded the close error. Fixed by
   removing `with_page` -- callers now use `TabHandle::close()` which
   always returns errors.

2. **WebSocket unreliable under pressure:** CDP `Target.closeTarget` goes
   over the same WebSocket as DOM operations. Under Chrome memory pressure,
   the WebSocket dies and close commands never reach Chrome. Fixed by
   adding `HttpTabCloser` which uses Chrome's HTTP debug endpoint
   (`GET /json/close/{targetId}`), a simpler code path that stays alive
   when the WebSocket is dead.

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
3. In a loop, call `browser.with_page(|page| { page.goto("https://example.com/", None).await; ... })`
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

The caller would run this periodically.

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

### Key files

- `crates/pwright-bridge/src/browser.rs` -- `with_page`, `TabHandle`, `Browser::new_tab`, `combine_results`
- `crates/pwright-bridge/src/tab.rs` -- `close_tab` (used by gRPC server, not `with_page`)
- `crates/pwright-cdp/src/connection.rs` -- `CdpError::Compound`, Drop/shutdown (CancellationToken)

## Impact

- **OOMKilled**: Chrome instances hit memory limit after 30-60 min of sustained use
- **Auto-reconnect masks the problem**: pool reconnects to restarted Chrome, but leaked tabs repeat
- **Workaround**: Increase Chrome memory limit, but this just delays OOM
