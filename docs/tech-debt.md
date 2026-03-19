# Tech Debt

Remaining structural issues. Updated 2026-03-17.

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

### ~~gRPC error conversion~~ (resolved)
Replaced 40+ `map_err` closures with `cdp_to_status()` helper that maps
error variants to correct gRPC codes.

### Unit test gaps
These modules have zero unit tests (covered only by integration tests):
- `navigate.rs` — 5 wait strategies
- `tab.rs` — create/close/reattach/resolve
- `touchscreen.rs` — tap dispatch

## Resolved

| Item | Resolution |
|------|-----------|
| Root node ID extraction (4 locations) | Extracted `root_node_id()` helper |
| on_request/on_response duplication | Generic `subscribe_network_event<T>()` |
| `GotoOptions.wait_until: String` | `WaitUntil` enum |
| `ScreenshotOptions.format: String` | `ImageFormat` enum |
| `on_error: String` / `param_type: String` | `OnError` / `ParamType` enums |
| go_back/go_forward duplication | `navigate_history(offset)` helper |
| `block_media` else-if bug | Independent `if` statements |
| `ChromeHttpClient::new()` panic | Returns `CdpResult<Self>` |
| State file corruption silent | Warns with error message |
| hover/drag coords before scroll | Swap order: scroll first |
| `WaitStrategy::None` waits | Returns immediately |
| `on_error: continue` wrong counter | Increments `failed` |
| CSS escaping missing `]` | Escapes `]`, `\n`, `\0` |
| unwrap_or(0.0) in get_element_center_js | Returns error |
| Inline JS in locator | Moved to pwright-js constants |
| Duplicate poll_ready_state | CLI reload uses shared function |
| Non-deterministic RefCache HashMap | Switched to BTreeMap |
| CLI type_text 3x overhead | Uses insert_text |
| TabHandle no Drop warning | Added Drop impl with tracing::warn |
| Emoji in CLI output | Replaced with [ok]/[error] |
| json!() cookie construction | Typed Cookie struct |
