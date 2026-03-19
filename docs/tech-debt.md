# Tech Debt

Remaining structural issues. Updated 2026-03-19.

## Open

### Selector encoding: `__pw_` string prefixes
31 occurrences across `selectors.rs`, `locator.rs`, `page.rs`. Selector
metadata encoded as string prefixes (`__pw_text=`, `__pw_label=`, etc.),
parsed back with `strip_prefix`. Should be a `SelectorKind` enum.
**Effort:** Large (architectural change, touches Locator internals).

### CdpClient trait delegation (185 lines)
`client_trait.rs` — 50+ methods that all delegate to CdpSession.
`macro_rules!` doesn't work due to `#[async_trait]` lifetime expansion.
Needs a proc macro crate.

### MockCdpClient (722 lines)
`test_utils.rs` — 15 setter methods + 60+ trait impls, all identical
patterns. Same `#[async_trait]` macro limitation.

### CLI command boilerplate
`commands.rs` — 10+ ref-based action commands follow the same
`connect -> resolve_ref -> resolve_tab -> action -> output::ok` pattern.
Extract a `with_ref()` closure helper.

### Unit test gaps
These modules have zero unit tests (covered only by integration tests):
- `navigate.rs` — 5 wait strategies
- `tab.rs` — create/close/reattach/resolve
- `touchscreen.rs` — tap dispatch

## Resolved

| Item | Resolution |
|------|-----------|
| **Stringly-typed APIs** | |
| CDP input event types (30+ strings) | `MouseEventType`, `KeyEventType`, `TouchEventType` enums |
| Mouse button `Option<String>` | `MouseButton` enum |
| `ExecutionResult.status: String` | `ExecutionStatus` enum |
| `GotoOptions.wait_until: String` | `WaitUntil` enum |
| `ScreenshotOptions.format: String` | `ImageFormat` enum |
| `on_error: String` / `param_type: String` | `OnError` / `ParamType` enums |
| **Duplication** | |
| gRPC error conversion (40+ closures) | `cdp_to_status()` with proper status codes |
| Node ID validation (10 match arms) | `require_node_id!()` macro |
| go_back/go_forward duplication | `navigate_history(offset)` helper |
| on_request/on_response duplication | Generic `subscribe_network_event<T>()` |
| Root node ID extraction (4 locations) | `root_node_id()` helper returning `CdpResult` |
| Duplicate poll_ready_state | CLI reload uses shared function |
| Touch tap 3x duplication | `Page::tap()` delegates to `Touchscreen::tap()` |
| Keyboard down/up branching | `dispatch_key()` helper |
| Executor error emission duplicate | Single emit + conditional return |
| Proto conversion boilerplate | `From` impls in `conversions.rs` |
| **Bugs** | |
| `block_media` else-if bug | Independent `if` statements |
| `on_error: continue` wrong counter | Increments `failed` |
| hover/drag coords before scroll | Scroll first, then get coords |
| `WaitStrategy::None` waits | Returns immediately |
| CSS escaping missing `]` | Escapes `]`, `\n`, `\0` |
| **Silent failures** | |
| `ChromeHttpClient::new()` panic | Returns `CdpResult<Self>` |
| State file corruption silent | Warns with error message |
| `unwrap_or(0.0)` in get_element_center_js | Returns error |
| `root_node_id` unwrap_or(1) | Returns `CdpResult<i64>` |
| Reload poll_ready_state silenced | Logs `tracing::warn!` |
| JsonlSink silent error drops | Logs `tracing::warn!` |
| State file permission silenced | Logs `tracing::debug!` |
| f64-to-u64 timeout cast | `.max(0.0)` guard |
| **Cleanup** | |
| `json!()` cookie construction | Typed `Cookie` struct, `&[Cookie]` API |
| `network_set_cookies(Vec<Value>)` | `network_set_cookies(&[Cookie])` |
| Inline JS in locator | Moved to pwright-js constants |
| Non-deterministic RefCache HashMap | Switched to BTreeMap |
| CLI type_text 3x overhead | Uses insert_text |
| TabHandle no Drop warning | Added Drop impl with `tracing::warn` |
| Emoji in CLI output | Replaced with [ok]/[error] |
| Unnecessary `#[allow(unused_imports)]` | Removed |
| Duplicate doc comment | Removed |
| Empty placeholder test file | Deleted |
| MockCdpClient missing setters | Added `set_targets_response()`, `set_describe_node_response()` |
