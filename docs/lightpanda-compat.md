<!-- agent-updated: 2026-03-30T01:00:00Z -->

# Lightpanda Browser Compatibility

## Background

[Lightpanda](https://github.com/lightpanda-io/browser) is a headless browser written in Zig from scratch (not a Chromium fork). It claims 9x less memory and 11x faster than Chrome. It exposes a CDP (Chrome DevTools Protocol) server, making it theoretically compatible with any CDP client, including pwright.

This doc tracks what works and what doesn't when connecting pwright to Lightpanda.

## Connection Design

### The Problem

Chrome exposes two interfaces: HTTP debug endpoints and WebSocket CDP. Lightpanda only implements `/json/version` over HTTP — everything else is WebSocket-only. pwright's `connect_http()` bundles HTTP discovery with HTTP tab management (`HttpTabCloser`), which breaks with Lightpanda since `/json/close/{id}` returns 404.

### The Solution: Unified `Browser::connect()`

**Drop `connect_http` as a separate method. Unify into a single `Browser::connect(config)` that:**

1. If `cdp_url` is HTTP → fetch `/json/version`, rewrite the returned WS URL to use the caller's host:port, connect WS
2. If `cdp_url` is WS → connect directly
3. Always use `CdpTabCloser` (`Target.closeTarget` over WS) — never HTTP for tab management

The URL rewrite is an implementation detail, not a user-facing choice. Chrome returns `ws://127.0.0.1:9225/devtools/browser/...` even when reached through a proxy — we take the path and graft it onto the caller's host:port. Lightpanda returns `ws://0.0.0.0:9222/` — same rewrite, just path is `/`.

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

### Validated: `Target.closeTarget` Works Through Proxies

Tested 2026-03-30 against Chrome at `chrome-proxy:9225` (reverse proxy):

| Test | Direct (localhost:9222) | Through proxy (chrome-proxy:9225) |
|------|-------------------------|-------------------------------------|
| Single tab lifecycle | PASS | PASS |
| `Target.closeTarget` | PASS | PASS |
| 10 rapid create/close | 10/10 | 10/10 |
| 5 concurrent tabs close | 5/5 | — |
| URL rewrite | N/A | `ws://127.0.0.1:9225/...` → `ws://chrome-proxy:9225/...` PASS |

`HttpTabCloser` can be removed. `CdpTabCloser` is reliable with both Chrome and Lightpanda, through direct connections and reverse proxies.

### Reverse Proxy Support

WebSocket-only works behind Caddy and nginx. All CDP traffic flows over the single WS connection — no HTTP endpoints needed beyond the optional `/json/version` discovery.

Caddy auto-detects WebSocket upgrades. nginx needs `proxy_set_header Upgrade` and `Connection "upgrade"` configured (see `docs/guides/remote-cdp.md`).

## Lightpanda Constraints

- **1 browser context per WebSocket connection** — `Target.createBrowserContext` returns error on second call
- **1 tab per browser context** — `Target.createTarget` returns error if a tab already exists
- **For parallel tabs:** open multiple `Browser::connect()` connections, each with its own tab
- **No HTTP debug endpoints** except `/json/version` — `/json/list`, `/json/new`, `/json/close` all return 404

## Test Setup

- Lightpanda: Docker `lightpanda/browser:nightly` on port 9333
- Chrome: `chromedp/headless-shell:latest` on port 9222 (local) and `chrome-proxy:9225` (proxy)
- Test transport: direct WebSocket
- Test dates: 2026-03-29 to 2026-03-30

## Compatibility Results

### Core Protocol

| Operation | CDP Method | Lightpanda | Chrome |
|-----------|------------|------------|--------|
| Get browser version | `Browser.getVersion` | PASS | PASS |
| Create browser context | `Target.createBrowserContext` | PASS | PASS |
| Create target/tab | `Target.createTarget` | PASS | PASS |
| Attach to target | `Target.attachToTarget` | PASS | PASS |
| List targets | `Target.getTargets` | PASS | PASS |
| Close target/tab | `Target.closeTarget` | PASS | PASS |
| Navigate | `Page.navigate` | PASS | PASS |
| Reload | `Page.reload` | PASS | PASS |
| Page lifecycle events | `Page.setLifecycleEventsEnabled` | PASS | PASS |
| Get frame tree | `Page.getFrameTree` | PASS | PASS |
| Screenshot | `Page.captureScreenshot` | PASS (PNG only) | PASS |
| Create isolated world | `Page.createIsolatedWorld` | PASS | PASS |

### JavaScript Evaluation

| Operation | Lightpanda | Chrome |
|-----------|------------|--------|
| Simple math (`1+1`) | PASS | PASS |
| `document.title` | PASS | PASS |
| `document.documentElement.outerHTML` | PASS | PASS |
| `document.documentElement.innerHTML` | PASS | PASS |
| `Runtime.callFunctionOn` with objectId | PASS | PASS |

### DOM

| Operation | CDP Method | Lightpanda |
|-----------|------------|------------|
| Get document tree | `DOM.getDocument` | PASS |
| Query selector | `DOM.querySelector` | PASS |
| Search DOM | `DOM.performSearch` | PASS |
| Get search results | `DOM.getSearchResults` | PASS |
| Get box model | `DOM.getBoxModel` | PASS (padding/border/margin are zeros) |
| Resolve node to JS object | `DOM.resolveNode` | PASS |
| Get outer HTML | `DOM.getOuterHTML` | PASS |
| Accessibility tree | `Accessibility.getFullAXTree` | PASS |

### Input & Navigation

| Operation | CDP Method | Lightpanda | Notes |
|-----------|------------|------------|-------|
| Mouse click | `Input.dispatchMouseEvent` (mousePressed) | PASS | |
| Mouse release | `Input.dispatchMouseEvent` (mouseReleased) | PASS | |
| Key press | `Input.dispatchKeyEvent` (keyDown) | PASS | |
| Insert text | `Input.insertText` | PASS | |
| Click-to-navigate (`<a>` tag) | Input + Page events | PASS | Full navigation lifecycle fires |

**Input caveat:** Lightpanda only processes `mousePressed` (ignores `mouseMoved`, `mouseWheel`) and `keyDown` (ignores `keyUp`, `rawKeyDown`, `char`). Commands return success for ignored types. End-to-end form interactions (fill + submit) need more testing.

### Cookies & Network

| Operation | CDP Method | Lightpanda |
|-----------|------------|------------|
| Get cookies | `Network.getCookies` | PASS |
| Set cookie | `Network.setCookie` | PASS |
| Enable network tracking | `Network.enable` | PASS |
| Set extra HTTP headers | `Network.setExtraHTTPHeaders` | PASS |
| Ignore certificate errors | `Security.setIgnoreCertificateErrors` | PASS |

## What Does NOT Work (Lightpanda)

| Feature | Why |
|---------|-----|
| Multiple tabs per connection | 1 context, 1 tab per WS connection |
| Device/touch emulation | All Emulation domain methods are stubs |
| CSS inspection | Only `CSS.enable` (no-op) |
| Screenshot formats besides PNG | Only PNG supported |

## Implementation Plan: Unified `Browser::connect()`

Breaking change to pwright-bridge:

1. **Remove `connect_http()`** — merge its logic into `connect()`
2. **Detect scheme in `connect()`** — HTTP triggers `/json/version` discovery + URL rewrite; WS connects directly
3. **Remove `HttpTabCloser`** — always use `CdpTabCloser`
4. **Remove `ChromeHttpClient`** — no longer needed for tab management (keep `reqwest` for `/json/version` fetch only, or inline the single HTTP call)
5. **Update callers** — CLI, gRPC server, tests, examples
6. **Test** — Chrome direct, Chrome through proxy, Lightpanda direct

## Remaining Items to Investigate

- [ ] End-to-end test: unified `connect()` from pwright Rust code against both backends
- [ ] Test actual form interactions (fill input + click submit + verify result) on Lightpanda
- [ ] Test wait-for-selector / wait-for-text patterns on Lightpanda
- [ ] Evaluate multi-connection pooling for parallel tab support with Lightpanda
