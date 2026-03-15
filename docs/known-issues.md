# Known Issues

## Open

### `tab_locks` DashMap unbounded growth (LOW)
**Location:** `crates/pwright-bridge/src/browser.rs`

Tabs closed externally (Chrome crash, user closes tab) leave orphaned entries
in `tab_locks`. Very slow growth, only matters for long-running servers.

**Fix:** Periodic cleanup or bounded LRU map.

### Snapshot depth computation O(n*d) (LOW)
**Location:** `crates/pwright-bridge/src/snapshot.rs:80-88`

`depth_of` walks parent chain per node. Acceptable for typical DOM depth (<20).

**Fix:** Precompute depths in single BFS pass (optimization, not bug).

---

## Not Issues (Reviewed and Rejected)

| Proposal | Why it's not an issue |
|----------|----------------------|
| gRPC needs authentication | Binds localhost by default. `--bind-all` requires user-managed firewall. Auth is a future feature. |
| `with_page` should register in tab map | Ephemeral tabs with auto-close by design. |
| Ref cache staleness between CLI invocations | Inherent to stateless CLI. gRPC server uses tab locks. |
| Download file race condition | Chrome's filename is unpredictable. Not practical to exploit. |
| Snapshot depth should use BFS | O(n*d) with d<20 is fast enough. |

---

## Fixed

| Issue | Severity | Fix |
|-------|----------|-----|
| JS injection in selector helpers | HIGH | `serde_json::to_string()` via `js_escape()` |
| CSS injection in `get_by_*` attribute selectors | HIGH | `css_escape_attr()` for `\` and `"` |
| Page Drop leaks listener tasks | MEDIUM | `Drop for Page` aborts handles via `try_lock` |
| Template variable injection in scripts | MEDIUM | Single-pass `resolve_template` |
| gRPC handlers skip tab locks | MEDIUM | `resolve_tab_locked()` with semaphore + mutex |
| File path traversal via `set_input_files` | MEDIUM | `upload_dir` validation with `canonicalize()` |
| Broadcast `Lagged` handling | LOW | Match `RecvError::Lagged` and continue |
| `select_by_node_id` wrong for `<select>` | LOW | `Runtime.callFunctionOn` with `selectedIndex` |
| `fetch_ws_url` TCP truncation | LOW | Replaced raw TCP with `reqwest` |
| CLI state file world-readable | LOW | `0600` permissions on Unix |
| Click doesn't trigger `<a>` navigation | CRITICAL | v3: scroll first, viewport coords via getBoundingClientRect, buttons field |
| Eval can't await Promises | HIGH | Auto-detect `await` keyword, use `evaluate_async` |
| Opaque wait_for timeout errors | MEDIUM | Include selector and timeout in error message |
| Opaque JS eval errors | MEDIUM | Extract line number and column from `exceptionDetails` |
| No `pwright script run` CLI command | HIGH | Added `Script` subcommand with `Run`/`Validate` |
| No `wait`/sleep step in scripts | MEDIUM | Added `wait: <ms>` step type |
| `with_page` silently ignores tab close errors | HIGH | Refactored to `TabHandle` + `Browser::new_tab` with explicit lifecycle; `with_page` propagates all errors via `CdpError::Compound` |
