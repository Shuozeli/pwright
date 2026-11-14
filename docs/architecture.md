<!-- agent-updated: 2026-10-20T18:30:17Z -->

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

## Composable response-wait boundary

The Node compatibility layer does not create a second CDP network observer.
The optional `response-events` feature exposes immutable notices from the
existing bounded Rust network lifecycle. JavaScript installs the public Page
listener synchronously and evaluates language predicates, but Rust owns
subscription activation, queue admission, URL-glob compilation, response
identity and foreign-copy accounting.

Terminal readiness is a second optional capability. `response-finished`
attaches one opaque Rust owner to an event Response; `Response.finished()` and
body reads share its single cached wait. `response-body` remains independent
and uses the browser-native request identity only after readiness. This keeps
the minimal event artifact free of body reads, Page routes and request history
while allowing the official profile to compose all three leaves.

## Experimental Playwright Server client boundary

Playwright Server's dispatcher protocol is distinct from Chromium CDP. The
default-off `pwright-playwright-server-client-core` crate owns only bounded
command IDs, pending replies, GUID creation/adoption/disposal, events, and
terminal cleanup. It accepts caller-supplied text transport; it has no socket,
async runtime, browser executable, CDP, or language-binding dependency. The
separate opt-in `playwright-server-initialize` path currently sends only
`Root.initialize` and exposes Python async/sync `BrowserType.connect` failure
behavior. The new default-off `pwright-playwright-server-session` crate
composes the protocol core with the bounded WebSocket transport and returns a
connection-owned Browser lease only after validating the upstream
`Root.initialize` reply and referenced GUID graph. Pre-reply lifecycle and
event ingress is replayed in wire order through a queue bounded to 1,024
messages and 1 MiB of original frame bytes; a pre-reply Browser close or
disposal fails the connection. Its 19 fake-duplex release tests pass with both
default and `live-tokio` features. An optional `supervisor` actor owns a
continuously polled lease with 128 in-flight command slots and an ordered event
queue capped at 128 entries and approximately 4 MiB of serialized payload by
default; overflow fails closed and aborted actors report `Stopped`. An explicit
drop policy for bindings without event listeners retains protocol validation,
GUID graph projection and terminal observation without queuing ordinary events.
The supervisor
reports its pre-reply ingress count, projects a bounded GUID initializer graph
before each command reply, and offers a terminal watcher that does not retain
the socket owner. The current all-feature suite passes 40/40 with strict
release Clippy. Two selected
Tailscale-bound real-WebSocket fixtures pass for raw lease and supervised
command/event/close paths against a synthetic Playwright Server, but are not
official conformance evidence. A separate default-off
`pwright-playwright-server-endpoint-discovery` crate converts HTTP(S) `/json`
responses to checked WS(S) endpoints using an injected fetcher; `live-reqwest`
is opt-in and follows any valid-Location 3xx up to five HTTP(S) redirects under
one cumulative deadline, retaining the original
endpoint as the WebSocket base. Its default and live-feature release suites pass
12/12 and 15/15 respectively; three explicitly selected Tailscale-bound live
HTTP fixture tests also pass. An explicit default-off
`pwright-playwright-server-connect` crate composes endpoint discovery,
caller-selected WebSocket headers, validated bootstrap and the supervisor;
its real HTTP/WS connectors are opt-in. Both default and all-feature release
suites pass 9/9, plus one explicitly selected synthetic Tailscale-bound
HTTP-to-WebSocket fixture. For HTTP(S), header syntax and bounds are validated
before discovery, while the WebSocket-reserved-name check runs after discovery
and before upgrade; direct WS(S) retains immediate rejection. The nine
fixture-adapted upstream redirect-timeout assertions now pass in both oracle
and candidate lanes without ledger promotion. A separate default-off
browser-ops crate passes 19/19 release tests for empty-options `newContext`,
`newPage`, and Context
`close` operations. Close accepts a correlated empty reply even if the server
disposed the Context first; stale/recycled handles are rejected before send.
Its handles retain Browser/Context/Page creation-sequence lineage and recheck
the parent generation after each correlated creation reply. This prevents a
recycled GUID from publishing a handle but cannot undo an in-flight wire-side
effect at a compatible server.
A separate, default-off `pwright-playwright-server-page-events` Rust leaf
consumes validated, ordered protocol ingress and projects bounded
Page-attributed Request, Response and Download notices plus Page/Context/Browser
terminal notices. Its 16 release tests and strict Clippy pass; its normal
dependency closure contains only the protocol core and `serde_json`, not WS,
Tokio, CDP or Node. The session now composes that leaf under the distinct
`ProjectPageNotices` policy: it projects at the wire ingress position, drops
unrelated validated events without filling the raw queue, serializes notice
readers and synthesizes child closes before `Disconnected`. Its selected suite
passes 50/50 release tests; the default queue and existing Node remote profile
still retain their prior behavior. A separate default-off Node profile exposes
typed notices and acknowledges Page network subscriptions before publishing a
new Page. Actor-side generation admission blocks commands aimed at a GUID
recycle it has already observed, without claiming an atomic server-side
compare-and-send. The selected addon builds and loads. In the independently
verified [v5 fixture-adapted diagnostic](../.work/official-conformance/results/core-connect-page-events-synthetic-v2-green-20261020-v5/attestation.json),
three unchanged upstream teardown assertions pass 3/3 in each oracle and
real-addon candidate lane, and a separate positive probe delivers
Request→Response→Download to waiters and Page listeners in each lane. This
does not establish real Playwright Server/browser or full public-API parity,
and it promotes no official ledger rows.
The Node opt-in session profile returns a Browser and composes the Rust
browser-ops crate for empty-option Context creation and Context Page creation.
Four earlier synthetic Tailscale-bound native tests and 18 adapter tests pass,
including a 160-event unsupported-event burst and ordered terminal
Page→Context→Browser close notifications;
two additional selected Tailscale-bound real-addon tests exercise explicit
Context close and implicit-Context `Browser.newPage()`/`Page.close()` without
disconnecting Browser. The thin wrapper keeps unsupported actions explicit and rejects recycled GUID
wrappers. Arbitrary non-terminal child-close events are not yet exposed.
Python ABI125 separately opts into a remote Browser lifecycle plus
empty-option Context/Page creation, with graph-backed inventories that reject
unknown or recycled object generations. Its fresh installed-wheel focused
tests passed 49/49 on both CPython 3.9.6 and 3.12.14, including four Tailscale-bound synthetic HTTP-to-WebSocket
variants with a 160-event burst; current binding source builds and passes
strict release Clippy, while direct addon unit testing is blocked by this Mac's
missing Python 3.9 link library. A newer ABI125 wheel with explicit
empty-option Context close and implicit-Context `Browser.new_page()` passes a
focused subset of 65/65 tests on each of those Python versions with Tailscale
fixtures enabled, including 6/6 compiled-extension WebSocket cases. It does
not support arbitrary externally initiated
child-close notifications or Context-owned `Page.close`; the protocol has no
compare-and-close generation field for stronger wire-side reuse guarantees.
The Python binding synthesizes Page→Context→Browser terminal close events;
sync callbacks without a caller dispatcher drain at the next caller-thread
API boundary, not during a completely idle caller. Arbitrary non-terminal
child-close events remain unsupported.
Neither binding has real-server E2E proof. This split allows later
transport and object adapters to remain
independently selectable without inflating the default installation. The live
WebSocket dependency's logging
and packaging boundary is tracked in the
[security design](knowledge/pwright-ws-logging-security-design.md).
Independent Node opt-in profiles expose bounded native `BrowserType.connect`
DNS/WebSocket-error, `Root.initialize` Close-error, and request-header
diagnostics. The default-off `playwright-server-connect-headers` profile uses
the `pwright-playwright-connect-headers` Rust leaf for default User-Agent,
`x-playwright-browser`, and empty proxy-header composition with case-insensitive
explicit-header precedence. The transport's bounded `WsRequest` applies the
same field limits and reserved-name policy to both explicit and synthesized
headers; Node only supplies header pairs and runtime identity. The current
native port receives runtime identity in a bounded three-string tuple and
exports a header-profile-specific ABI-shape marker, so the old addon cannot
be mistaken for this signature. The post-refactor v2, fixture-host-adapted
Core diagnostic passes four original assertions in each candidate/oracle lane;
its four socket receipts per lane have only ordinal per-spec attribution.
Historical v1 evidence is preserved. Neither version is an unadapted official
pass, public API parity, or a Core ledger promotion. Exact artifact hashes and
fixture limits are in the [connect evidence scope](knowledge/pwright-playwright-server-connect.md).
Those earlier Node error/header diagnostic profiles do not construct a remote
Browser on a successful connection; the newer opt-in session profile is
consumed by both language bindings as described above.
The optional live transport separately classifies HTTP upgrade rejection into
a status and at most 8 KiB of body. A selected Node failure-path adapter has
passed two fixture-host-adapted Core HTTP 401 assertions against an independent
oracle, without creating a remote Browser or promoting ledger rows. The
transport explicitly flushes a received WebSocket Close reply. A subsequent
host-adapted strict diagnostic verifies two correct Close replies in each
candidate/oracle lane, without proving an unadapted official run or a remote
Browser.

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
- **`navigate.rs`** — Navigation with 4 wait strategies: None, DOM ready, NetworkIdle, Selector. The three polling strategies perform an immediate probe and then wait through the injected `Clock`; production uses `TokioClock` while deterministic tests advance `FakeClock` explicitly.
- **`actions.rs`** — 12 browser actions: click, dblclick, type, fill, press, hover, focus, select, scroll (element + page), drag, check, uncheck. Each follows the CDP pattern (e.g., `scrollIntoView → getBoxModel → dispatchMouseEvent`). Focus uses `DOM.focus` directly. Check/uncheck are in `Locator` (check state + click).
- **`snapshot.rs`** — Builds a flat accessibility tree from `Accessibility.getFullAXTree`. Assigns refs (`e0`, `e1`, ...) mapped to `backendDOMNodeId` for stable element references.
- **`content.rs`** — Screenshot (PNG/JPEG/WebP), PDF, text extraction.
- **`evaluate.rs`** — JavaScript evaluation. `FromEvalResult` trait for typed `evaluate_into::<T>()` (String, bool, i64, f64, Value, `FromEvalJson<T>` for JSON deserialization).
- **`cookies.rs`** — Cookie get/set.
- **`clock.rs`** — `Clock` trait for dependency-injected monotonic time. `TokioClock` uses real Tokio time. `FakeClock` sleepers remain pending until `advance`/`set_ms` reaches their deadline, waking without real delays. Duration conversion and deadline addition saturate instead of wrapping.
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
- **`call_log.rs`** — Shared thread-safe CDP call recorder used by both the
  stateful fake and bridge's opt-in canned mock. The bridge reaches it only
  through `test-utils`; the production bridge closure does not include
  `pwright-fake`.
- **`response_script.rs`** — Reusable FIFO error/one-shot-response/persistent-
  fallback precedence with no method defaults, strict policy, or async gate.
  One mutex owns the complete script so response reconfiguration and
  consumption cannot observe a partial cross-queue update. `FakeCdpClient`
  uses it for all 11 configurable Runtime entry points while each method
  continues to own its protocol-shaped default.
- **`response_slot.rs`** — Shared thread-safe optional persistent response used
  by all 15 canned-mock response fields and the stateful fake's navigation
  override. It owns no default, strict policy, queue, error, or async gate.
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
