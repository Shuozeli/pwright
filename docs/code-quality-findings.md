# Code Quality Findings

Second audit performed 2026-03-19 against commit 05818de (post-cleanup).

## 1. Duplication

### FromEvalResult trait impls are copy-paste with accessor name changed
- **Location:** `crates/pwright-bridge/src/evaluate.rs:22-67`
- **Problem:** Four impls (`String`, `bool`, `i64`, `f64`) repeat the same pattern: `remote_object["value"].as_TYPE().ok_or_else(|| CdpError::Other(format!("expected TYPE ...")))`. Only the accessor method name and type name differ.
- **Fix:** Extract a helper:
  ```rust
  fn extract_value<T>(remote_object: &Value, extractor: fn(&Value) -> Option<T>, type_name: &str) -> CdpResult<T> {
      extractor(&remote_object["value"])
          .ok_or_else(|| CdpError::Other(format!("expected {type_name} from evaluate, got: {remote_object}")))
  }
  ```
  Then each impl becomes a one-liner: `extract_value(remote_object, Value::as_str, "string").map(String::from)`.

### RPC handler preamble repeated 15+ times
- **Location:** `crates/pwright-server/src/service/data.rs:18-21,39-42,59-62`, `navigation.rs:80-82,100-103,116-119`, `content.rs:17-20,50-53,78-80,100-102`, `tabs.rs:56-58`, `actions.rs:12-15`
- **Problem:** Every handler that operates on a tab repeats:
  ```rust
  let browser = svc.get_browser().await?;
  let req = request.into_inner();
  let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;
  ```
  Already tracked in `tech-debt.md` as "CLI command boilerplate" (same pattern, different crate). The `_permit` and `_lock` bindings are required to hold the RAII guards, so this can't be trivially extracted into a function without changing ownership semantics.
- **Fix:** A macro would work but adds indirection. Best addressed when the handler count grows further. Mark as **low priority** — the pattern is correct, just verbose.
- **Priority:** Low.

## 2. Missing Abstractions

### Magic numbers for timeouts and poll intervals
- **Location:** `crates/pwright-bridge/src/navigate.rs:116,147,182` (200ms, 250ms poll intervals), `crates/pwright-bridge/src/playwright/locator.rs:359` (200ms), `crates/pwright-bridge/src/playwright/page.rs:148` (30_000ms default timeout), `crates/pwright-bridge/src/actions.rs:235` (drag step: 10.0, 5.0, 40.0)
- **Problem:** Timing constants are scattered as raw literals. Changing the default poll interval requires finding all occurrences.
- **Fix:** Define named constants in each module:
  ```rust
  // navigate.rs
  const READY_STATE_POLL_MS: u64 = 200;
  const NETWORK_IDLE_POLL_MS: u64 = 250;
  // locator.rs
  const WAIT_POLL_MS: u64 = 200;
  // actions.rs
  const DRAG_STEP_PX: f64 = 10.0;
  const DRAG_MIN_STEPS: f64 = 5.0;
  const DRAG_MAX_STEPS: f64 = 40.0;
  ```
- **Priority:** Low.

### Function key virtual code magic number
- **Location:** `crates/pwright-bridge/src/keys.rs:89`
- **Problem:** `111 + num as i64` uses a magic number for the base virtual key code of F-keys.
- **Fix:** `const VK_F1_BASE: i64 = 112; // Windows virtual key code for F1` then `VK_F1_BASE + (num - 1) as i64`.
- **Priority:** Low.

## 3. Over-Architecture

No issues found. Crate decomposition and abstraction levels are appropriate.

## 4. Silent Failures

### YAML parser defaults silently on missing optional fields
- **Location:** `crates/pwright-script/src/parser.rs:17-19` (`unwrap_or("unnamed")`, `unwrap_or("")`, `unwrap_or(1)`)
- **Problem:** Missing `name`, `description`, `version` fields default silently. This is intentional for optional schema fields but `version` defaulting to 1 could mask a schema version mismatch.
- **Fix:** No change needed — this is correct YAML parsing behavior. The validator catches truly invalid scripts.
- **Priority:** Not actionable.

## 5. Unsafe Patterns

### Narrowing casts in parser (i64 -> i32, i64 -> u64)
- **Location:** `crates/pwright-script/src/parser.rs:19` (`as i32`), `:115,167` (`as u64`)
- **Problem:** `version` field cast from i64 to i32 without bounds check. Timeout values cast from i64 to u64 without negative check.
- **Fix:** Use `i32::try_from(n).unwrap_or(1)` for version. For timeouts, use `.max(0) as u64`.
- **Priority:** Low — these are YAML values from controlled input.

### tabs.len() as i32 could overflow on 32-bit
- **Location:** `crates/pwright-server/src/service/lifecycle.rs:46`
- **Problem:** `tabs.len() as i32` wraps silently if there are >2B tabs (impossible in practice, but the cast is sloppy).
- **Fix:** Use `i32::try_from(tabs.len()).unwrap_or(i32::MAX)`.
- **Priority:** Very low.
