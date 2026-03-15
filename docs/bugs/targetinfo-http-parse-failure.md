<!-- agent-updated: 2026-03-15T21:10:00Z -->

# Bug: TargetInfo struct fails to parse Chrome HTTP /json/list response

## Status: Fixed (2fa7b2e, 2026-03-15)
## Severity: High
## Component: pwright-cdp / ChromeHttpClient

## Symptom

`ChromeHttpClient::list_targets()` fails with:
```
HTTP list_targets JSON parse failed: error decoding response body
```

Parsing succeeds only when Chrome returns an empty target list (`[]`). Any downstream service reusing `TargetInfo` for HTTP endpoints hits this when Chrome has active tabs.

## Root Cause

`TargetInfo` in `crates/pwright-cdp/src/domains/target.rs` was designed for CDP `Target.getTargets` responses, but `ChromeHttpClient::list_targets()` reuses it to parse Chrome's HTTP `/json/list` response. The two endpoints return different JSON schemas:

### CDP `Target.getTargets` response (what TargetInfo expects):
```json
{
  "targetInfos": [
    {
      "targetId": "ABC123",
      "type": "page",
      "title": "Google",
      "url": "https://google.com",
      "attached": false
    }
  ]
}
```

### Chrome HTTP `/json/list` response (what list_targets actually gets):
```json
[
  {
    "description": "",
    "devtoolsFrontendUrl": "/devtools/inspector.html?ws=...",
    "id": "ABC123",
    "title": "Google",
    "type": "page",
    "url": "https://google.com",
    "webSocketDebuggerUrl": "ws://localhost:9222/devtools/page/ABC123"
  }
]
```

### Field mismatches:
| Field | CDP (Target.getTargets) | HTTP (/json/list) | TargetInfo expects |
|-------|------------------------|--------------------|--------------------|
| ID | `targetId` | `id` | `targetId` (via camelCase rename) -- MISMATCH |
| attached | `attached` (bool) | not present | `attached` (required bool) -- MISSING |
| extra fields | none | `description`, `devtoolsFrontendUrl`, `webSocketDebuggerUrl` | ignored (serde default) -- OK |

When Chrome has 0 targets, `/json/list` returns `[]` which deserializes to `Vec<TargetInfo>` successfully (empty vec). Once targets exist, the missing `targetId` and required `attached` fields cause deserialization failure.

## Impact

- Any consumer using `ChromeHttpClient::list_targets()` fails when Chrome has active tabs
- Tab cleanup logic that depends on `list_targets()` is completely non-functional
- `create_target()` also fails on Chrome 134+ (requires PUT, not GET)

## Proposed Fix

Add serde aliases and defaults to `TargetInfo` to handle both response formats:

```rust
/// Target metadata returned by both CDP `Target.getTargets` and Chrome HTTP `/json/list`.
///
/// CDP uses `targetId` + `attached`; Chrome HTTP uses `id` and omits `attached`.
/// The serde aliases handle both formats transparently.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    #[serde(alias = "id")]
    pub target_id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub attached: bool,
}
```

Changes:
- `#[serde(alias = "id")]` on `target_id`: accepts both `targetId` (CDP) and `id` (HTTP)
- `#[serde(default)]` on `attached`: defaults to `false` when absent (HTTP response)

This is backward-compatible -- CDP `Target.getTargets` responses still parse correctly.

## Verification

1. `cargo test --workspace` should pass (existing tests unaffected)
2. Integration test `http_client_list_targets` should pass against real Chrome
3. All 8 `chrome_http` integration tests pass

## Discovered

2026-03-15, during integration testing of ChromeHttpClient against real Chrome.
