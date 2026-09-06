<!-- agent-updated: 2026-07-29T18:36:00Z -->

# Architecture

## Overview

pwright is a thin Rust bridge between downstream services and Chrome. It connects to Chrome via the Chrome DevTools Protocol (CDP) over WebSocket and exposes a gRPC API.

```
┌──────────────┐       gRPC        ┌──────────────┐      CDP/WS       ┌─────────┐
│  Downstream  │ ◄───────────────► │   pwright    │ ◄───────────────► │  Chrome  │
│  Service     │  (protobuf)       │  (5.9 MB)    │  HTTP /json/*     │  :9222   │
└──────────────┘                   └──────────────┘ ◄───────────────► └─────────┘
```

CDP WebSocket handles DOM, Runtime, Input, and all page interactions.
Chrome HTTP endpoints (`/json/close`, `/json/list`) handle tab lifecycle
and are used as a reliable fallback when the WebSocket is under pressure.
Pwright does not consider an HTTP close complete until `/json/list` confirms
that the exact leased target disappeared.
`PWRIGHT_EPHEMERAL_LEASE_ROOT` lets sibling recipe working directories share
the exact-target recovery ledger for one browser endpoint.
Script runs persist that exact target immediately after `createTarget`, before
`attachToTarget` or page-domain setup. If attach fails, Pwright closes the
created target; if cleanup also fails, the already-persisted lease makes the
target recoverable by the next invocation. Recovery is fail-closed: a script
does not create another tab while any same-endpoint leased target cannot be
closed and verified. A lease whose owner process is still running represents
active sibling work and is preserved, allowing bounded concurrent recipe
processes without one invocation closing another invocation's tab. Lease
removal is idempotent because concurrent stale-lease recovery may observe that
another process has already removed the same file.
Script recipes configure endpoint discovery, WebSocket connection, and CDP
command latency separately from page-readiness waits through
`config.connection_timeout_ms`; raising it for a remote browser does not
lengthen explicit page challenge or selector waits.
When `PWRIGHT_EPHEMERAL_PROXY_SERVER` or its bypass-list companion is set,
recipe and raw-capture tabs are created inside a CDP browser context with
`disposeOnDetach`. `TabHandle::close` closes the exact target and disposes
that context. This overrides a process-wide Chrome proxy for transient work
without changing persistent tabs or other profiles, while process failure
also tears down the isolated context.

## Crate Structure

### `pwright-cdp` — Raw CDP Client

Async WebSocket client for Chrome DevTools Protocol.

- **`connection.rs`** — WebSocket transport with JSON-RPC framing. Each outbound command gets a unique `id`; responses are routed back via `oneshot` channels. Events are broadcast to all subscribers. Incoming frames/messages are bounded at 128 MiB so a 50 MiB raw response can survive CDP's base64 and JSON expansion without hitting Tungstenite's smaller defaults.
- **`session.rs`** — Wraps a connection with an optional `sessionId` for tab-scoped commands. Browser-level commands use `sessionId = None`.
- **`events.rs`** — `CdpEvent` type for event dispatch.
- **`domains/`** — Typed wrappers for 9 CDP domains:

| Domain | Key Methods |
|--------|-------------|
| `target` | `createBrowserContext`, `createTarget`, `disposeBrowserContext`, `closeTarget`, `getTargets`, `attachToTarget` |
| `page` | `navigate`, `captureScreenshot`, `printToPDF`, `addScriptToEvaluateOnNewDocument` |
| `dom` | `focus`, `scrollIntoViewIfNeeded`, `getBoxModel`, `resolveNode`, `setFileInputFiles` |
| `input` | `dispatchMouseEvent`, `dispatchKeyEvent`, `insertText`, `dispatchTouchEvent` |
| `runtime` | `evaluate`, `callFunctionOn` |
| `accessibility` | `getFullAXTree` |
| `network` | `setBlockedURLs`, `getCookies`, `setCookies` |
| `fetch` | `enable`, response-stage interception, `getResponseBody`, `continueRequest`, `failRequest` |
| `browser` | `getVersion`, `downloadProgress` |

### `pwright-bridge` — High-Level Operations

Translates user-intent into CDP command sequences.

- **`browser.rs`** — Central controller. Manages the CDP connection, tab map, ref caches, and concurrency (per-tab `Mutex` + cross-tab `Semaphore`). `TabHandle` provides explicit ephemeral tab lifecycle (`new_tab` / `close`), and `new_tab_with_created_target` lets callers persist ownership before attach. Any attach failure attempts exact-target cleanup. `TabCloser` abstracts the close transport: `CdpTabCloser` (WebSocket), `HttpTabCloser` (Chrome HTTP endpoint), or a context-aware closer for isolated proxy overrides. HTTP-configured close verifies exact target removal and preserves failure state if neither transport can prove that the target disappeared.
- **`chrome_http.rs`** — HTTP client for Chrome's debug endpoints (`/json/list`, `/json/close`, `/json/new`, `/json/version`). More reliable than CDP WebSocket under memory pressure.
- **`tab.rs`** — Tab lifecycle: create (via `Target.createTarget` + `attachToTarget`), close, list, resolve.
- **`navigate.rs`** — Navigation with 4 wait strategies: None, DOM ready, NetworkIdle, Selector.
- **`actions.rs`** — 12 browser actions: click, dblclick, type, fill, press, hover, focus, select, scroll (element + page), drag, check, uncheck. Each follows the CDP pattern (e.g., `scrollIntoView → getBoxModel → dispatchMouseEvent`). Focus uses `DOM.focus` directly. Check/uncheck are in `Locator` (check state + click).
- **`snapshot.rs`** — Builds a flat accessibility tree from `Accessibility.getFullAXTree`. Assigns refs (`e0`, `e1`, ...) mapped to `backendDOMNodeId` for stable element references.
- **`content.rs`** — Screenshot (PNG/JPEG/WebP), PDF, text extraction.
- **`evaluate.rs`** — JavaScript evaluation. `FromEvalResult` trait for typed `evaluate_into::<T>()` (String, bool, i64, f64, Value, `FromEvalJson<T>` for JSON deserialization).
- **`cookies.rs`** — Cookie get/set.
- **`clock.rs`** — `Clock` trait for dependency-injected time operations. `TokioClock` (real) for production, `FakeClock` (deterministic) for tests.
- **`keys.rs`** — Named key definitions (Enter, Tab, Escape, Arrow keys, F1-F12).
- **`playwright/`** — Playwright-compatible high-level API:
  - **`page.rs`** — `Page` struct: `goto()`, `locator()`, `get_by_text/label/role()`, `keyboard()`, `mouse()`, `touchscreen()`, etc.
  - **`locator.rs`** — `Locator` struct: `click()`, `fill()`, `type_text()`, `filter_by_text()`, `and()`, `or()`, `set_input_files()`, etc.
  - **`keyboard.rs`** — `Keyboard` struct: `press()`, `type_text()`, `down()`, `up()`, `insert_text()`.
  - **`mouse.rs`** — `Mouse` struct: `click()`, `dblclick()`, `move_to()`, `wheel()`, `down()`, `up()`.
  - **`touchscreen.rs`** — `Touchscreen` struct: `tap(x, y)` via `Input.dispatchTouchEvent`.
  - **`selectors.rs`** — Selector resolution engine. Uses `SelectorKind` enum (Css, Text, TextExact, Label, Role, Nth, FilterText) dispatching to CSS queries or JS-based element finding.
- **`network.rs`** — Network event types (`NetworkRequest`, `NetworkResponse`) and CDP event parsing helpers for `Page::on_request()`/`on_response()`.

The CLI's `capture-url` path combines these layers: a durably leased
`TabHandle`, optional context navigation, `Fetch.requestPaused` at the
response stage, and `Fetch.getResponseBody`. Interception happens before
Chrome's PDF viewer substitutes its HTML embedder, so binary documents retain
their original bytes. The caller receives HTTP metadata and redirects, while
destination non-overwrite and decoded-byte limits are enforced locally.
The CDP transport limit is deliberately larger than the decoded capture limit:
Chrome may place the complete base64 response in one WebSocket frame. A live
15,084,952-byte issuer deck validated this boundary end to end.
The durable destination write is the capture commit point. Fetch-disable and
tab-close operations run afterward. If either teardown operation fails after
that commit, the CLI emits a structured warning without invalidating the
captured bytes. An unverified tab close keeps its exact persisted target lease,
allowing the next same-endpoint command to recover that tab without touching
unrelated browser state.

### `pwright-script` — Declarative Script Runner

Parses and executes YAML automation scripts against `pwright-bridge`.

- **`proto/script.proto`** — Protobuf schema (source of truth) for Script, Step types, and output format.
- **`parser.rs`** — YAML parser: converts YAML into internal Script model with template resolution.
- **`validator.rs`** — Validates params, template refs, JS registry refs, and step-specific rules.
- **`executor.rs`** — Executes steps sequentially against a `Page`, manages variables and output collection.
- **`output.rs`** — JSONL streaming output (`JsonlSink`) and in-memory output (`VecSink`) for testing.
- **`model.rs`** — Internal types: `Script`, `Step`, `StepKind`, `ParamDef`, `JsFunction`.

### `pwright-fake` — In-Memory Browser Fake

Test-only crate providing `FakeCdpClient` with an in-memory DOM tree.

- **`pwright.rs`** — Consumer-facing `FakePwright` harness. Wraps the in-memory
  CDP fake and gives downstream projects a stable testing component for their
  pwright-facing adapters.
- **`dom.rs`** — DOM node tree with text_content, attributes, find, serialize.
- **`selector.rs`** — CSS selector matching (tag, class, id, attribute, descendant, compound).
- **`builder.rs`** — HTML parser that builds DomNode trees from strings.
- **`client.rs`** — `FakeCdpClient` implementing `CdpClient` with real DOM operations.

### `pwright-server` — gRPC Server

- **`main.rs`** — CLI entrypoint with `clap`. Configurable via flags or env vars (`CDP_URL`, `GRPC_ADDR`, etc.).
- **`service.rs`** — Implements all 21 gRPC RPCs, mapping each to bridge layer operations.
- **`build.rs`** — Protobuf codegen via `tonic-build`.

## Concurrency Model

- **Per-tab sequential**: Each tab has a `tokio::sync::Mutex` ensuring only one CDP command sequence runs at a time per tab.
- **Cross-tab parallel**: A `tokio::sync::Semaphore` bounds concurrent tab operations (default: 4).
- gRPC requests flow through this executor so concurrent clients don't corrupt tab state.

## Design Principles

1. **Thin bridge** — No profiles, dashboard, orchestration, stealth, or IDPI. Downstream handles its own policy.
2. **gRPC-native** — Protobuf over HTTP/2. No REST API.
3. **Attach-first** — Connects to an already-running Chrome. Launch mode can be added later.
4. **Minimal memory** — No GC, no embedded JS runtime. Target: <15MB resident.
5. **JS-only-when-necessary** — Pure CDP domains preferred (`DOM.getAttributes`, `DOM.getBoxModel`). JS evaluation via `Runtime.callFunctionOn` is used for `innerText`, `value`, `scrollBy`, `is_checked`/`is_disabled` (DOM properties), text/label/role locator resolution, and per-element `evaluate()`.
