# Getting Started

## Prerequisites

- Rust 2024 edition (1.85+)
- Google Chrome or Chromium
- `protoc` (protocol buffer compiler) — for building from source

## Build

```bash
cd pwright
cargo build --release
```

The binary is at `target/release/pwright-server` (~5.9 MB).

---

## What is CDP?

**CDP** (Chrome DevTools Protocol) is the same protocol that Chrome DevTools uses to inspect and control the browser. When you open Chrome DevTools (F12), it communicates with Chrome over CDP.

pwright connects to Chrome via CDP to perform browser automation — navigating pages, clicking elements, taking screenshots, etc. — without needing a GUI. Think of it as a programmatic remote control for Chrome.

### How it works

```
Your Code → gRPC → pwright → CDP (WebSocket) → Chrome
```

1. Chrome exposes a **WebSocket endpoint** when started with `--remote-debugging-port`
2. pwright connects to that WebSocket and sends JSON-RPC commands
3. Your code talks to pwright via gRPC (or the CLI)

---

## Start Chrome with CDP

Chrome must be running with remote debugging enabled:

```bash
# Linux — headless
google-chrome --headless=new --remote-debugging-port=9222 &
# Linux — headed (visible window)
google-chrome --remote-debugging-port=9222 &

# macOS — headless
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --headless=new --remote-debugging-port=9222 &
# macOS — headed
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222 &

# Windows (PowerShell) — headless
Start-Process "C:\Program Files\Google\Chrome\Application\chrome.exe" "--headless=new","--remote-debugging-port=9222"
# Windows (PowerShell) — headed
Start-Process "C:\Program Files\Google\Chrome\Application\chrome.exe" "--remote-debugging-port=9222"
```

Verify Chrome is listening:

```bash
curl -s http://127.0.0.1:9222/json/version | jq .
```

Expected output:

```json
{
  "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/browser/xxxxxxxx-...",
  "Browser": "Chrome/131.0.6778.139",
  ...
}
```

> The `webSocketDebuggerUrl` is the CDP WebSocket URL that pwright connects to. You'll see it stored in `.pwright/state.json` after your first `pwright open` command.

---

## The `.pwright` Folder

When you run `pwright open`, the CLI creates a `.pwright/` directory in your current working directory to persist session state across commands.

### `.pwright/state.json`

```json
{
  "cdp_url": "http://localhost:9222",
  "ws_url": "ws://localhost:9222/devtools/browser/0760709d-...",
  "active_tab": "tab_00000000",
  "target_id": "5C2B2F5BA250506DB3351C1D0376B41C"
}
```

| Field | What it stores |
|-------|---------------|
| `cdp_url` | The Chrome HTTP endpoint (e.g. `http://localhost:9222`) used for WebSocket URL discovery |
| `ws_url` | The WebSocket URL auto-discovered from `cdp_url` via `/json/version` |
| `active_tab` | The internal tab ID for the currently active tab, so subsequent commands know which tab to operate on |
| `target_id` | Chrome's target ID for re-attaching to the tab after reconnection |

### How state works

- `pwright open` — Creates the directory, connects to Chrome, opens a tab, saves state
- `pwright snapshot`, `pwright click e1`, etc. — Read state, reconnect, perform the action
- `pwright close` — Clears the active tab from state
- `pwright tab-select <id>` — Updates the active tab

This means you can run CLI commands across separate terminal invocations without passing `--tab-id` every time.

### Gitignore

Add `.pwright/` to your `.gitignore` — it contains session-specific data:

```gitignore
.pwright/
```

---

## Start pwright Server (gRPC mode)

```bash
# Localhost only (default — recommended)
./target/release/pwright-server \
  --cdp-url ws://127.0.0.1:9222/devtools/browser/...

# Bind to all interfaces (use with caution)
./target/release/pwright-server \
  --cdp-url ws://127.0.0.1:9222/devtools/browser/... \
  --bind-all

# Or via environment variables
CDP_URL=ws://127.0.0.1:9222/devtools/browser/... \
./target/release/pwright-server
```

### Server Options

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--cdp-url` | `CDP_URL` | — | Chrome DevTools WebSocket URL |
| `--addr` | `GRPC_ADDR` | `127.0.0.1:50051` | gRPC listen address |
| `--bind-all` / `-B` | — | off | Bind to all network interfaces (0.0.0.0) |
| `--disable-eval` | `PWRIGHT_DISABLE_EVAL` | off | Block the Evaluate RPC |
| `--max-parallel-tabs` | `MAX_PARALLEL_TABS` | `4` | Max concurrent tab operations |
| `--nav-timeout-ms` | `NAV_TIMEOUT_MS` | `30000` | Default navigation timeout |

---

## CLI Quick Start

```bash
# 1. Connect to Chrome and open a tab
pwright open https://example.com

# 2. Get the accessibility snapshot (shows element refs like e0, e1, ...)
pwright snapshot

# 3. Click an element by its ref
pwright click e1

# 4. Take a screenshot
pwright screenshot

# 5. Close the tab
pwright close
```

---

## First gRPC Call

Using [grpcurl](https://github.com/fullstorydev/grpcurl):

```bash
# 1. Connect to Chrome
grpcurl -plaintext -d '{"cdp_url": "ws://127.0.0.1:9222/devtools/browser/..."}' \
  localhost:50051 pwright.v1.BrowserService/ConnectBrowser

# 2. Navigate to a page
grpcurl -plaintext -d '{"url": "https://example.com", "new_tab": true}' \
  localhost:50051 pwright.v1.BrowserService/Navigate

# 3. Get accessibility snapshot
grpcurl -plaintext -d '{"tab_id": "tab_00000000", "filter": "FILTER_INTERACTIVE"}' \
  localhost:50051 pwright.v1.BrowserService/GetSnapshot

# 4. Click an element by ref
grpcurl -plaintext -d '{"tab_id": "tab_00000000", "kind": "CLICK", "ref": "e5"}' \
  localhost:50051 pwright.v1.BrowserService/ExecuteAction
```

---

## Using from Code

Generate client stubs from `proto/pwright/v1/browser.proto` using your language's protobuf toolchain:

- **Python**: `grpcio-tools` / `betterproto`
- **Go**: `protoc-gen-go-grpc`
- **Rust**: `tonic-build` (client codegen is already enabled)
- **TypeScript**: `@grpc/proto-loader` or `ts-proto`

---

## Troubleshooting

### "cannot connect to Chrome" / Connection refused

```
Error: cannot connect to Chrome

  Caused by: Connection refused (os error 111)
```

**Chrome is not running** or is not listening on the expected port.

**Fix:**
1. Check if Chrome is running: `ps aux | grep chrome`
2. Start Chrome with remote debugging: `google-chrome --headless=new --remote-debugging-port=9222 &`
3. Verify it's listening: `curl -s http://localhost:9222/json/version`

### "no webSocketDebuggerUrl in response"

Chrome is reachable but returned an unexpected response.

**Possible causes:**
- The URL is pointing to something other than Chrome (e.g., a different web server on port 9222)
- Chrome was started without `--remote-debugging-port`

**Fix:** Ensure Chrome was started with `--remote-debugging-port=9222` and try `curl http://localhost:9222/json/version` — you should see `webSocketDebuggerUrl` in the JSON response.

### "No connection. Run `pwright open` first."

The CLI doesn't have a saved session.

**Fix:** Run `pwright open` (or `pwright open https://example.com`) to establish a connection. This creates `.pwright/state.json`.

### "failed to reattach tab"

Chrome restarted since the last `pwright open`, so the saved tab ID is stale.

**Fix:** Run `pwright open` again to create a fresh connection and tab.

### Wrong port or remote Chrome

If Chrome is on a different host or port:

```bash
# CLI: specify the CDP HTTP endpoint
pwright --cdp http://192.168.1.100:9222 open https://example.com

# Server: specify the CDP WebSocket URL
pwright-server --cdp-url ws://192.168.1.100:9222/devtools/browser/...
```

The `--cdp` flag can also be set via the `PWRIGHT_CDP` environment variable.

---

## Security Considerations

pwright provides direct browser control — treat both the CDP and gRPC ports as privileged.

| Port | Risk | Mitigation |
|------|------|------------|
| CDP (9222) | Full browser control for anyone who can reach it | Firewall to localhost; never expose to the network |
| gRPC (50051) | Arbitrary JS execution, screenshots, cookie access | Defaults to localhost; use `--bind-all` only with firewall rules |

### Key defaults

- **gRPC binds `127.0.0.1`** — only local clients can connect. Use `--bind-all` to override.
- **`--disable-eval`** — blocks the `Evaluate` RPC, preventing arbitrary JavaScript execution via gRPC.
- **No TLS** — gRPC runs over plaintext HTTP/2. If you need encryption, front it with a TLS-terminating proxy.
- **No auth** — all RPCs are open. In multi-tenant environments, run separate pwright instances with network isolation.
