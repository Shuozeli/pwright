# pwright - Project Guide

This is the authoritative reference for understanding pwright. Read this
before proposing changes.

## What is pwright?

A lightweight Rust CDP (Chrome DevTools Protocol) bridge. It connects to an
already-running Chrome instance and provides browser automation via:

- **Rust library** (`pwright-bridge`): Playwright-compatible Page/Locator API
- **gRPC server** (`pwright-server`): 21 RPCs for tabs, navigation, actions, content
- **CLI** (`pwright-cli`): snapshot-act-snapshot workflow for terminal/agent use
- **Script runner** (`pwright-script`): declarative YAML automation scripts

## Crate Map

```
pwright-cdp          Raw WebSocket CDP client
                     Connection, Session, domain wrappers (DOM, Runtime, etc.)
                     CancellationToken for clean shutdown

pwright-js           Centralized JS snippets (string constants, no runtime)

pwright-bridge       High-level browser operations
                     Page, Locator, Keyboard, Mouse, Touchscreen
                     Actions, navigation, snapshot, cookies, evaluate
                     Clock trait for DI in time-dependent code
                     TabCloser trait + ChromeHttpClient for HTTP tab lifecycle

pwright-server       gRPC server wrapping pwright-bridge
                     21 RPCs, protobuf schemas

pwright-cli          CLI wrapping pwright-bridge
                     Stateless: persists session in .pwright/state.json

pwright-script       Declarative YAML script runner (Phase 1 done)
                     Parser, validator, executor, JSONL output
                     Protobuf schema for script input/output

pwright-fake         In-memory browser fake for testing
                     DOM tree, CSS selector engine, HTML parser
                     FakeCdpClient implementing CdpClient trait

pwright-cdp-gen      CDP protocol code generator (standalone binary)
                     Generates typed Rust structs from protocol JSON spec
```

## Architecture Decisions

These are intentional. Do NOT propose changing them.

### 1. Attach-only, no browser launch
pwright does NOT download, install, or manage Chrome. It connects to an
already-running instance. Keeps binary at ~6MB.

### 2. No gRPC authentication
Binds `127.0.0.1` by default. Documented. `--bind-all` requires firewall.
Auth is a future feature request, not a missing security fix.

### 3. Tab lifecycle is caller-managed
`Browser::new_tab` returns a `TabHandle` with explicit `close()`. Callers
own the tab lifecycle. There is no auto-close convenience wrapper --
callers must close tabs themselves to avoid leaks.

### 4. Ref cache staleness is expected
CLI's snapshot-act workflow stores refs in `.pwright/state.json`. DOM changes
between invocations invalidate refs. This is the documented design. The gRPC
server uses per-tab locks for concurrent access.

### 5. `Page::close()` takes `&self`
Uses `AtomicBool` for thread-safe close from multiple tasks sharing `Arc<Page>`.

### 6. JS selectors use `returnByValue: false`
`get_by_text/label/role` and `filter_by_text` evaluate JS to find elements,
then `DOM.requestNode` for nodeIds. Must call `DOM.getDocument` first to
enable the DOM domain. Uses `runtime_evaluate_as_object` (not `runtime_evaluate`).

### 7. CdpConnection uses CancellationToken
On drop, cancels reader/writer tasks via `tokio::select!`. Writer sends
WebSocket Close frame. Do not revert to `drop(writer_handle)`.

### 8. DOM methods use `nodeId` not `backendNodeId`
Callers pass nodeIds from `querySelector`. These are NOT backendNodeIds.
CDP accepts both as separate parameters - they're different ID spaces.

### 9. Download race condition is acceptable
Chrome's `Browser.downloadProgress` provides the filename. Predicting it
for a symlink attack is not practical.

### 10. Snapshot depth O(n*d) is acceptable
Typical DOM depth <20. BFS optimization not worth the complexity.

## How to Learn This Project

1. Read this file first
2. Read `docs/architecture.md` for crate internals
3. Read `docs/known-issues.md` for current bugs and their status
4. Read `docs/knowledge/testing-strategy.md` for test architecture
5. Run `cargo test --workspace` to verify your environment
6. Run integration tests against real Chrome to see CDP in action

## Testing

### Tier 1: MockCdpClient (call sequence verification)
Records which CDP methods are called and in what order.
Use for: "click dispatches mousePressed then mouseReleased"

### Tier 2: FakeCdpClient (behavior verification)
In-memory DOM with real CSS selector matching.
Use for: "is_checked returns true for checked checkbox"

### Tier 3: Docker integration (end-to-end)
Real Chrome via `chromedp/headless-shell` with in-process test server.
96 tests across 15 files. Concurrency tests with 5 parallel tabs.

**Integration test gotchas:**
- Chrome rejects non-IP `Host` headers (resolve hostname to IP)
- Chrome returns internal WS URL (use `rewrite_ws_url`)
- Test server/CDP connection need dedicated threads (survive `#[tokio::test]` runtime boundaries)
- Share one CDP WebSocket per test binary via `OnceLock`

## CDP Protocol Notes

- `DOM.getDocument` must be called before `DOM.requestNode` (enables DOM domain)
- `Target.attachToTarget` with `flatten: true` multiplexes sessions on one WS
- `Runtime.evaluate` with `returnByValue: true` serializes DOM nodes as null
- `Runtime.evaluate` with `returnByValue: false` returns objectId for DOM nodes
- Chrome's HTTP endpoint rejects Host headers that aren't localhost or an IP

## Key Files

| File | Purpose |
|------|---------|
| `CLAUDE.md` | Slim rules for agents (loaded every conversation) |
| `docs/index.md` | This file (full project guide) |
| `docs/known-issues.md` | Prioritized bugs and improvements |
| `docs/architecture.md` | Crate internals and design principles |
| `docs/knowledge/script-runner-design.md` | Script runner 5-phase plan |
| `docs/knowledge/testing-strategy.md` | Fake + Docker test architecture |
| `docs/knowledge/playwright-api.md` | Playwright API compatibility matrix |
| `docs/knowledge/feature-comparison.md` | pwright vs Playwright vs PinchTab |
| `docs/knowledge/chrome-devtools-mcp-comparison.md` | pwright CLI vs Google's chrome-devtools-mcp CLI |
| `docs/knowledge/cdp-codegen-design.md` | CDP protocol codegen crate design |
| `docs/knowledge/network-capture-design.md` | Network capture CLI design (second CDP session) |
| `docs/codelabs.md` | Usage examples (Rust, CLI, gRPC, scripts) |
