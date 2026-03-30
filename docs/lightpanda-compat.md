<!-- agent-updated: 2026-03-30T06:00:00Z -->

# Lightpanda Browser Compatibility

## Background

[Lightpanda](https://github.com/lightpanda-io/browser) is a headless browser written in Zig from scratch (not a Chromium fork). It claims 9x less memory and 11x faster than Chrome. It exposes a CDP (Chrome DevTools Protocol) server, making it theoretically compatible with any CDP client, including pwright.

This doc tracks what works and what doesn't when connecting pwright to Lightpanda.

## Connection Design

### Unified `Browser::connect()`

`Browser::connect(config)` handles both backends:

1. If `cdp_url` is HTTP → fetch `/json/version`, rewrite the returned WS URL to use the caller's host:port, connect WS
2. If `cdp_url` is WS → connect directly
3. Always use `CdpTabCloser` (`Target.closeTarget` over WS) — never HTTP for tab management

The URL rewrite is an implementation detail. Chrome returns `ws://127.0.0.1:9225/devtools/browser/...` even through a proxy — we graft the caller's host:port onto the path. Lightpanda returns `ws://0.0.0.0:9222/` — same rewrite, path is `/`.

```rust
// Chrome behind proxy — HTTP discovery, rewrite, WS tab management
let config = BrowserConfig {
    cdp_url: "http://chrome-proxy:9225".to_string(),
    ..BrowserConfig::default()
};

// Lightpanda — direct WS, no discovery needed
let config = BrowserConfig {
    cdp_url: "ws://127.0.0.1:9333/".to_string(),
    ..BrowserConfig::default()
};

// Same method for both
let browser = Browser::connect(config).await?;
```

Reverse proxies (Caddy, nginx) work — all CDP traffic flows over the single WS connection.

## Lightpanda Constraints

- **1 browser context per WebSocket connection** — `Target.createBrowserContext` errors on second call
- **1 tab per browser context** — `Target.createTarget` errors if a tab exists
- **For parallel tabs:** open multiple `Browser::connect()` connections
- **No HTTP debug endpoints** except `/json/version`
- **No `data:` URL support** — returns `UrlMalformat`
- **Target IDs are per-connection** — two connections both get `FID-0000000001`

## Side-by-Side: pwright Feature Compatibility

Tested with 32 integration tests (`tests/integration/tests/lightpanda.rs`).

### Connection & Tab Lifecycle

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| Connect via WS URL | PASS | PASS | |
| Connect via HTTP URL | PASS | N/A | LP has no HTTP discovery except `/json/version` |
| `Browser.getVersion` | PASS | PASS | |
| `Target.createTarget` | PASS | PASS | |
| `Target.closeTarget` | PASS | PASS | |
| `Target.getTargets` | PASS | PASS | |
| Tab close idempotent | PASS | PASS | |
| Multiple tabs, one connection | PASS | FAIL | LP: 1 tab per connection |
| Multiple tabs, multiple connections | PASS | PASS | |
| Close via HTTP (`/json/close`) | PASS | FAIL (404) | Not needed — WS close works |

### Navigation

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `page.goto(url)` | PASS | PASS | |
| `page.title()` | PASS | PASS | |
| `page.url()` | PASS | PASS | |
| `page.content()` (outerHTML) | PASS | PASS | |
| `page.body_text()` | PASS | PASS | |
| `page.reload()` | PASS | PASS | |
| `page.go_back()` / `go_forward()` | PASS | Not tested | |
| Navigate between pages | PASS | PASS | |
| Click link triggers navigation | PASS | PASS | Full lifecycle events fire |
| `data:` URLs | PASS | FAIL | LP returns `UrlMalformat` |
| Page lifecycle events | PASS | PASS | DOMContentLoaded, load, networkIdle |

### JavaScript Evaluation

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `evaluate_sync` — numbers | PASS | PASS | |
| `evaluate_sync` — strings | PASS | PASS | |
| `evaluate_sync` — booleans | PASS | PASS | |
| `evaluate_sync` — null | PASS | PASS | |
| `evaluate_sync` — objects | PASS | PASS | |
| `evaluate_sync` — arrays | PASS | PASS | |
| `evaluate_sync_into<T>` typed | PASS | PASS | |
| DOM queries via JS | PASS | PASS | |
| DOM create element via JS | PASS | PASS | |
| `Runtime.callFunctionOn` | PASS | PASS | |
| `evaluate_async` (Promises) | PASS | Not tested | |
| `evaluate_with_arg` | PASS | Not tested | |

### DOM & Selectors

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `DOM.getDocument` | PASS | PASS | |
| `DOM.querySelector` | PASS | PASS | |
| `DOM.performSearch` / `getSearchResults` | PASS | PASS | |
| `DOM.getBoxModel` | PASS | PASS | padding/border/margin are zeros in LP |
| `DOM.resolveNode` | PASS | PASS | |
| `DOM.getOuterHTML` | PASS | PASS | |
| `DOM.describeNode` | PASS | **FAIL** | LP returns `UnknownMethod` |
| `page.text_content(selector)` | PASS | PASS | |
| `page.inner_html(selector)` | PASS | PASS | |
| `page.get_attribute(selector, attr)` | PASS | **FAIL** | Uses `DOM.describeNode` internally |
| `page.is_visible(selector)` | PASS | PASS | |

### Locator API

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `locator.text_content()` | PASS | PASS | |
| `locator.count()` | PASS | PASS | |
| `locator.click()` | PASS | PASS | |
| `locator.get_attribute()` | PASS | **FAIL** | Uses `DOM.describeNode` |
| `locator.fill()` | PASS | **FAIL** | Uses `DOM.describeNode` |
| `locator.first()` / `last()` / `nth()` | PASS | Not tested | |
| `get_by_text()` / `get_by_label()` / `get_by_role()` | PASS | Not tested | |

### Input & Actions

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `Input.dispatchMouseEvent` (click) | PASS | PASS | Only `mousePressed` processed |
| `Input.dispatchKeyEvent` | PASS | PASS | Only `keyDown` processed |
| `Input.insertText` | PASS | PASS | |
| Click button (onclick handler) | PASS | PASS | Tested via JS-created element |
| `page.fill()` | PASS | **FAIL** | Requires `DOM.describeNode` |
| `page.type_text()` | PASS | Not tested | |
| Double-click | PASS | Not tested | |
| Right-click | PASS | Not tested | |
| Hover | PASS | Not tested | |
| Coordinate-based click | PASS | Not tested | |

### Accessibility

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `Accessibility.getFullAXTree` | PASS | PASS* | *`nodeId` is integer (Chrome: string) — pwright's typed deserializer fails, raw CDP works |

### Screenshot & Content

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `Page.captureScreenshot` (PNG) | PASS | PASS | |
| Screenshot (JPEG) | PASS | FAIL | LP only supports PNG |
| PDF export | PASS | Not tested | |

### Cookies & Network

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| `Network.getCookies` | PASS | PASS | |
| `Network.setCookie` | PASS | PASS | |
| `Network.setExtraHTTPHeaders` | PASS | PASS | |
| `Security.setIgnoreCertificateErrors` | PASS | PASS | |
| Network event listeners (`on_response`) | PASS | Not tested | |
| Request interception (`Fetch` domain) | PASS | Not tested | |

### Emulation

| Feature | Chrome | Lightpanda | Notes |
|---------|--------|------------|-------|
| Device metrics override | PASS | FAIL | Stub (no-op) |
| Touch emulation | PASS | FAIL | Stub (no-op) |
| Media emulation | PASS | FAIL | Stub (no-op) |
| User agent override | PASS | FAIL | Stub (no-op) |

## Key Gaps Summary

### Blockers for Full pwright Compatibility

| Gap | Impact | Workaround |
|-----|--------|------------|
| **`DOM.describeNode` not implemented** | `get_attribute()`, `fill()`, locator attribute queries all fail | Use JS eval: `document.querySelector(sel).getAttribute(name)` |
| **No `data:` URL support** | Can't use inline HTML for tests | Use real URLs or JS-created DOM |
| **1 tab per connection** | No multi-tab on single connection | Open separate connections |
| **AX tree `nodeId` is integer** | pwright's `RawAXNode` deserializer fails | Use raw `session.send()` instead of typed API |

### Non-Blockers (Stubs / Partial)

- Emulation domain: all stubs (no-ops)
- Input events: only `mousePressed` and `keyDown` processed; others return success but are ignored
- `DOM.getBoxModel`: padding/border/margin always zeros
- Screenshot: PNG only (no JPEG/WebP)
- CSS inspection: only `CSS.enable` (no-op)

## Test Infrastructure

Integration tests: `tests/integration/tests/lightpanda.rs` (32 tests)

```bash
# Start Lightpanda
docker compose -f tests/integration/docker-compose.local.yml up -d

# Run tests
cargo test -p pwright-integration-tests --test lightpanda -- --ignored --test-threads=1
```

Tests use `LIGHTPANDA_HOST` / `LIGHTPANDA_PORT` env vars (default: `127.0.0.1:9333`).

## Remaining Items

- [ ] Test `evaluate_async` (Promise resolution) on Lightpanda
- [ ] Test `evaluate_with_arg` (Runtime.callFunctionOn with args)
- [ ] Test `go_back()` / `go_forward()` history navigation
- [ ] Test advanced locators (`get_by_text`, `get_by_role`, `get_by_label`)
- [ ] Test network event listeners (`on_response`, `on_request`)
- [ ] Test `Fetch` domain (request interception)
- [ ] Evaluate multi-connection pooling for parallel tab support
- [ ] File upstream issue for `DOM.describeNode` support
- [ ] File upstream issue for `data:` URL support
- [ ] File upstream issue for AX tree `nodeId` type (integer vs string)
