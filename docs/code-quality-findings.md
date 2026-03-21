# Code Quality Findings

Audit performed 2026-03-20 on the pwright workspace.

## Summary

The codebase is well-structured with clean separation of concerns, good test coverage, and consistent patterns. The issues found are minor.

## Findings

### 1. Dead Code: `runtime_evaluate_typed` and `runtime_call_function_typed` [FIXED]

**File:** `crates/pwright-cdp/src/domains/runtime.rs`, lines 115-131
**Severity:** Low
**Category:** Dead code

Two public methods on `CdpSession` -- `runtime_evaluate_typed` and `runtime_call_function_typed` -- are never called anywhere in the codebase. They wrap `runtime_evaluate` / `runtime_call_function_on` with typed deserialization into `EvaluateResult` / `CallFunctionResult`, but no code uses them.

**Fix:** Remove the dead methods. The hand-written `RemoteObject`, `EvaluateResult`, and `CallFunctionResult` types (lines 11-42) exist solely to support these methods and their tests, but should be kept since they have their own unit tests validating CDP type deserialization which serves as documentation. Only the two unused methods need removal.

---

### 2. Dead Code: Hand-written types `RemoteObject`, `EvaluateResult`, `CallFunctionResult` only used in tests [FIXED]

**File:** `crates/pwright-cdp/src/domains/runtime.rs`, lines 11-42
**Severity:** Low
**Category:** Dead code / over-architecture

The three structs `RemoteObject`, `EvaluateResult`, and `CallFunctionResult` are defined but never used outside of this file's own tests and the dead `_typed` methods. The generated `runtime.rs` already has equivalent types. After removing the `_typed` methods, these types only serve their own unit tests.

**Fix:** Remove the dead types and their tests. The generated equivalents in `generated/runtime.rs` can be used if typed deserialization is needed in the future.

---

### 3. Duplication: `print_snapshot` and `format_snapshot_node` in CLI output [FIXED]

**File:** `crates/pwright-cli/src/output.rs`, lines 5-33 and 60-83
**Severity:** Low
**Category:** Duplication

`print_snapshot` formats and prints each node. `format_snapshot_node` (test-only) formats a node to a string using identical logic. The formatting logic is duplicated.

**Fix:** Refactor `print_snapshot` to use `format_snapshot_node` (make it non-test-only), then call `println!` on its result.

---

### 4. Redundant Block Patterns: `MEDIA_BLOCK_PATTERNS` is superset of `IMAGE_BLOCK_PATTERNS` [FIXED]

**File:** `crates/pwright-bridge/src/navigate.rs`, lines 216-223
**Severity:** Low
**Category:** Duplication / Redundant data

`MEDIA_BLOCK_PATTERNS` is a strict superset of `IMAGE_BLOCK_PATTERNS` -- it contains all 7 image patterns plus 7 audio/video patterns. When both `block_images` and `block_media` are true (lines 63-72), the image patterns are sent to Chrome twice. While functionally harmless, it is confusing and duplicative.

**Fix:** Define `IMAGE_BLOCK_PATTERNS` and `MEDIA_ONLY_PATTERNS` (audio/video only), then compose `MEDIA_BLOCK_PATTERNS` from both at the usage site. Alternatively, when `block_media` is true, skip adding image patterns since media already includes them.

---

### 5. Silent `unwrap_or_default()` on deserialization in CDP domains [INFO]

**Files:**
- `crates/pwright-cdp/src/domains/network.rs`, line 57: `network_get_cookies`
- `crates/pwright-cdp/src/domains/target.rs`, line 57: `target_get_targets`
- `crates/pwright-cdp/src/domains/accessibility.rs`, line 72: `accessibility_get_full_tree`

**Severity:** Info
**Category:** Silent failures

These methods use `serde_json::from_value(result[key].take()).unwrap_or_default()` which silently returns an empty collection if deserialization fails. This masks protocol bugs. However, per the CLAUDE.md rules, this is a known pattern in the project and changing it would alter error behavior.

**Fix:** No fix applied. Documented for awareness.

---

### 6. `Cow<'static, str>` in `KeyDef::code` is unnecessary for most cases [INFO]

**File:** `crates/pwright-bridge/src/keys.rs`, lines 6-10
**Severity:** Info
**Category:** Over-engineering

`KeyDef::code` uses `Cow<'static, str>` but only the F-key branch allocates (`Cow::Owned`). All other cases use `Cow::Borrowed`. This is actually correct and idiomatic for this pattern.

**Fix:** No fix needed. The current approach is optimal.

---

### 7. Fully Qualified Path Where Import Already Exists [FIXED]

**File:** `crates/pwright-bridge/src/actions.rs`, line 329
**Severity:** Low
**Category:** Noise / Style

`select_by_node_id` uses `pwright_cdp::connection::CdpError::Other(...)` despite `CdpError` being imported at line 5 via `use pwright_cdp::connection::{CdpError, Result as CdpResult}`. All other call sites in the file use the short form.

**Fix:** Replace with `CdpError::Other(...)`.

---

### 8. Formatting Issues From Previous Fixes [FIXED]

**File:** `crates/pwright-bridge/src/navigate.rs`, line 361
**Severity:** Low
**Category:** Style

`cargo fmt --check` reported a formatting inconsistency in a test assertion added by the previous fix for finding 4.

**Fix:** Ran `cargo fmt`.
