# TODO

## Planned Features

### P0: `pwr` CLI - gRPC client for remote pwright

A lightweight CLI binary that connects to a pwright gRPC server instead of
directly to Chrome. For users who have a deployed pwright-server and want
CLI access without SSH or Docker exec.

```bash
# Instead of:
pwright --cdp http://remote:9222 snapshot

# This:
pwr --server remote:50051 snapshot
pwr --server remote:50051 click e1
pwr --server remote:50051 script run scraper.yaml --param url=https://example.com
```

**Crate:** `crates/pwr/` (new, depends only on tonic client + clap)

**Why separate binary:**
- Thin client (~500 lines), no CDP/WebSocket dependencies
- Can be distributed as a standalone binary without Rust toolchain
- Clear separation: `pwright` = direct CDP, `pwr` = remote gRPC

**Commands to support:**
- All snapshot/action/navigation commands (map to existing gRPC RPCs)
- `pwr script run` (needs new `RunScript` gRPC RPC on server)
- `pwr health` (maps to existing Health RPC)

**Implementation:**
1. Add `RunScript` RPC to pwright-server proto + service
2. Create `crates/pwr/` with tonic gRPC client
3. Mirror pwright CLI command structure
4. JSONL output for script results

**Open questions:**
- Should `pwr` maintain state like pwright does (`.pwright/state.json`)?
- Should script YAML files be sent to the server or parsed client-side?
- Auth: bearer token via `--token` flag or `PWR_TOKEN` env?

---

### P1: `--capabilities` structured self-description

Agents need to discover what a CLI can do. A `pwright --capabilities` command
that outputs JSON describing all tools, params, and examples would make pwright
usable by any agent without a skill file.

```bash
pwright --capabilities  # JSON output of all commands with params and descriptions
```

**Why:** quotemstr and yammosk on HN describe the need for structured CLI
help consumable by LLMs and shell completion. Current `--help` is human-readable
but not machine-optimal. This is the CLI equivalent of MCP's tool discovery.

**Implementation:** Auto-generate from clap's command metadata. One new flag.

---

### P1: Publish Docker image to ghcr.io

Eliminate the `--build` step from `docker compose up`. CI pipeline builds
and pushes a pre-built image on each release.

---

### P1: `RunScript` gRPC RPC

New RPC on the gRPC server to accept a YAML script + params and execute it:

```protobuf
rpc RunScript(RunScriptRequest) returns (stream RunScriptResponse);

message RunScriptRequest {
  string yaml = 1;
  map<string, string> params = 2;
}

message RunScriptResponse {
  oneof result {
    StepResult step = 1;
    ScriptResult summary = 2;
  }
}
```

Server-side streaming returns step results as they complete. The `pwr` CLI
consumes the stream and prints JSONL.

---

### P2: `page.reload()` should wait for page load (bridge layer)

The CLI reload command already waits via `poll_ready_state`, but the bridge
`Page::reload()` still fires `Page.reload` and returns immediately. The JS
execution context resets, so subsequent `evaluate()` calls may return empty.

---

### P2: `handle_dialog` - Alert/confirm/prompt handling

Chrome fires `Page.javascriptDialogOpening` when alert/confirm/prompt appears.
pwright currently has no way to handle these, causing automation to hang.

```rust
page.on_dialog(|dialog| async {
    dialog.accept(Some("response")).await
});
// or for CLI:
// pwright handle-dialog accept
// pwright handle-dialog dismiss
```

Uses CDP `Page.handleJavaScriptDialog`. chrome-devtools-mcp has this.

---

### P2: Connection health check

`Browser::connect_http()` succeeds once but the connection can die silently.
Add a lightweight ping:

```rust
if !browser.is_alive().await {
    browser = Browser::connect_http(url).await?;
}
```

Implement via `ChromeHttpClient::version()` (HTTP) or a no-op CDP call
(WebSocket). Return `bool`, not `Result` -- this is a health probe.

**Location:** `crates/pwright-bridge/src/browser.rs`

---

### P2: Script runner Phase 2 (for_each, retry, if)

See `docs/knowledge/script-runner-design.md` Phase 2.

---

### P2: Request blocking CLI

Block images/ads/tracking to reduce page weight during agent browsing.
Already have `network_set_blocked_urls` in the API, just needs CLI exposure.

```bash
pwright block-urls "*.jpg" "*.png" "*.gif" "*.mp4"
pwright block-urls "google-analytics.com/*" "facebook.net/*"
```

**Why:** Token efficiency is the #1 complaint about browser automation tools.
Blocking unnecessary resources cuts page load time and snapshot size.

---

### P2: Viewport / device emulation CLI

Capture responsive behavior by adjusting viewport. Useful for testing
responsive layouts and mobile-specific behavior.

```bash
pwright emulate --viewport 375x812         # iPhone-sized
pwright emulate --viewport 1920x1080       # Desktop
pwright resize 375 812                     # Simple resize
```

**Why:** Requires CDP `Emulation` domain (not yet generated in codegen).

---

### P2: Console message capture CLI

Access browser console messages for debugging.

```bash
pwright console-list                       # List console messages
pwright console-get <msgid>                # Get full message with stack trace
```

**Why:** chrome-devtools-mcp has this. Useful for debugging JS errors during
automation without needing to look at Chrome directly.

---

### P3: Script runner Phase 3 (paginate, screenshot, debug mode)

See `docs/knowledge/script-runner-design.md` Phase 3.

---

### P3: CDP codegen remaining

Phases 1-4 done (types, params, returns, domain migration).
Remaining: CDP event dispatcher, add new domains (Emulation, Console, Log).
See `docs/knowledge/cdp-codegen-design.md`.

---

### P3: Cookie import/export

Save and restore authenticated sessions across runs.

```bash
pwright cookie-export cookies.json
pwright cookie-import cookies.json
```

---

### P3: Split CdpClient trait

Current `CdpClient` trait has 45+ methods. Consider splitting into
domain-specific sub-traits for cleaner mock/fake implementations.

---

## Completed

| Feature | Commit |
|---------|--------|
| CDP codegen phases 1-4 (types, params, returns, migration) | `448e430` |
| `FromEvalResult` / `evaluate_into::<T>()` | `c73702c` |
| Network capture CLI (`network-listen`, `network-list`, `network-get`) | `7157788` |
| `click-at` / `dblclick` / `hover-at` coordinate CLI commands | `03b3de9` |
| Chrome HTTP integration test fixes (TargetInfo alias, PUT /json/new) | `2b038ca` |
| RECIPE.md skill doc for recipe authoring | `c73702c` |
| Chrome-devtools-mcp feature comparison | `448e430` |
| Code review: 12 code improvements + 16 review items fixed | `fbbd82d` |
| `page.wait_for_text()` / `page.wait_until()` + CLI `wait-for-text`, `wait-for`, `wait-until` | — |

## Bugs from Field Testing

| # | Bug | Severity | Status |
|---|-----|----------|--------|
| 1 | `click` didn't trigger `<a>` navigation | Critical | FIXED (v3: scroll first, then viewport coords via getBoundingClientRect) |
| 2 | eval can't await Promises | High | FIXED (auto-detect await, use evaluate_async) |
| 3 | JS eval IIFE vs function duality | Medium | Open - consider auto-wrapping |
| 4 | Opaque error messages | Medium | FIXED (selector in timeout, line:col in JS errors) |
| 5 | No wait/sleep step | Medium | FIXED (wait step added) |
| 6 | No for_each loop | Low | Open (Phase 2) |
| 7 | No `pwright script run` CLI | High | FIXED |
