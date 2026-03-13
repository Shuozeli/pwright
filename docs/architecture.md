# Architecture

## Overview

pwright is a thin Rust bridge between downstream services and Chrome. It connects to Chrome via the Chrome DevTools Protocol (CDP) over WebSocket and exposes a gRPC API.

```
┌──────────────┐       gRPC        ┌──────────────┐      CDP/WS       ┌─────────┐
│  Downstream  │ ◄───────────────► │   pwright    │ ◄───────────────► │  Chrome  │
│  Service     │  (protobuf)       │  (5.9 MB)    │  (JSON-RPC/WS)   │          │
└──────────────┘                   └──────────────┘                   └─────────┘
```

## Crate Structure

### `pwright-cdp` — Raw CDP Client

Async WebSocket client for Chrome DevTools Protocol.

- **`connection.rs`** — WebSocket transport with JSON-RPC framing. Each outbound command gets a unique `id`; responses are routed back via `oneshot` channels. Events are broadcast to all subscribers.
- **`session.rs`** — Wraps a connection with an optional `sessionId` for tab-scoped commands. Browser-level commands use `sessionId = None`.
- **`events.rs`** — `CdpEvent` type for event dispatch.
- **`domains/`** — Typed wrappers for 8 CDP domains:

| Domain | Key Methods |
|--------|-------------|
| `target` | `createTarget`, `closeTarget`, `getTargets`, `attachToTarget` |
| `page` | `navigate`, `captureScreenshot`, `printToPDF`, `addScriptToEvaluateOnNewDocument` |
| `dom` | `focus`, `scrollIntoViewIfNeeded`, `getBoxModel`, `resolveNode`, `setFileInputFiles` |
| `input` | `dispatchMouseEvent`, `dispatchKeyEvent`, `insertText`, `dispatchTouchEvent` |
| `runtime` | `evaluate`, `callFunctionOn` |
| `accessibility` | `getFullAXTree` |
| `network` | `setBlockedURLs`, `getCookies`, `setCookies` |
| `fetch` | `enable`, `disable`, `continueRequest`, `failRequest` |

### `pwright-bridge` — High-Level Operations

Translates user-intent into CDP command sequences.

- **`browser.rs`** — Central controller. Manages the CDP connection, tab map, ref caches, and concurrency (per-tab `Mutex` + cross-tab `Semaphore`).
- **`tab.rs`** — Tab lifecycle: create (via `Target.createTarget` + `attachToTarget`), close, list, resolve.
- **`navigate.rs`** — Navigation with 4 wait strategies: None, DOM ready, NetworkIdle, Selector.
- **`actions.rs`** — 9 browser actions: click, type, fill, press, focus, hover, select, scroll, drag. Each follows the CDP pattern learned from PinchTab (e.g., `scrollIntoView → getBoxModel → dispatchMouseEvent`).
- **`snapshot.rs`** — Builds a flat accessibility tree from `Accessibility.getFullAXTree`. Assigns refs (`e0`, `e1`, ...) mapped to `backendDOMNodeId` for stable element references.
- **`content.rs`** — Screenshot (PNG/JPEG/WebP), PDF, text extraction.
- **`evaluate.rs`** — JavaScript evaluation.
- **`cookies.rs`** — Cookie get/set.
- **`keys.rs`** — Named key definitions (Enter, Tab, Escape, Arrow keys, F1-F12).
- **`playwright/`** — Playwright-compatible high-level API:
  - **`page.rs`** — `Page` struct: `goto()`, `locator()`, `get_by_text/label/role()`, `keyboard()`, `mouse()`, `touchscreen()`, etc.
  - **`locator.rs`** — `Locator` struct: `click()`, `fill()`, `type_text()`, `filter_by_text()`, `and()`, `or()`, `set_input_files()`, etc.
  - **`keyboard.rs`** — `Keyboard` struct: `press()`, `type_text()`, `down()`, `up()`, `insert_text()`.
  - **`mouse.rs`** — `Mouse` struct: `click()`, `dblclick()`, `move_to()`, `wheel()`, `down()`, `up()`.
  - **`touchscreen.rs`** — `Touchscreen` struct: `tap(x, y)` via `Input.dispatchTouchEvent`.
  - **`selectors.rs`** — Selector resolution engine supporting CSS and JS-based selectors (`__pw_text=`, `__pw_label=`, `__pw_role=`, `__pw_filter_text=`).

### `pwright-server` — gRPC Server

- **`main.rs`** — CLI entrypoint with `clap`. Configurable via flags or env vars (`CDP_URL`, `GRPC_ADDR`, etc.).
- **`service.rs`** — Implements all 20 gRPC RPCs, mapping each to bridge layer operations.
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
5. **JS-only-when-necessary** — Pure CDP domains preferred (`DOM.getAttributes`, `DOM.getBoxModel`, AX tree). JS evaluation is used only for `innerText`, `value`, `scrollBy`, text/label/role locator resolution.
