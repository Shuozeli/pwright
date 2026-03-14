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

### P1: Publish Docker image to ghcr.io

Eliminate the `--build` step from `docker compose up`. CI pipeline builds
and pushes `ghcr.io/shuozeli/pwright-server:latest` on each release.

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

### P2: Script runner Phase 2 (for_each, retry, if)

See `docs/knowledge/script-runner-design.md` Phase 2.

---

### P2: `page.reload()` should wait for page load

Currently `Page::reload()` fires `Page.reload` CDP command and returns
immediately. The JS execution context resets, so subsequent `evaluate()`
calls may return empty. Should poll `readyState` like `goto()` does.

---

### P3: Script runner Phase 3 (paginate, screenshot, debug mode)

See `docs/knowledge/script-runner-design.md` Phase 3.

---

## Bugs from Field Testing

Track bugs reported from production usage (dragb integration).

| # | Bug | Severity | Status |
|---|-----|----------|--------|
| 1 | `click` didn't trigger `<a>` navigation | Critical | FIXED (v3: scroll first, then viewport coords via getBoundingClientRect) |
| 2 | eval can't await Promises | High | FIXED (auto-detect await, use evaluate_async) |
| 3 | JS eval IIFE vs function duality | Medium | Open - consider auto-wrapping |
| 4 | Opaque error messages | Medium | FIXED (selector in timeout, line:col in JS errors) |
| 5 | No wait/sleep step | Medium | FIXED (wait step added) |
| 6 | No for_each loop | Low | Open (Phase 2) |
| 7 | No `pwright script run` CLI | High | FIXED |
