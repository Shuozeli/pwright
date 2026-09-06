<!-- agent-updated: 2026-09-05T13:26:15Z -->

# pwright

A lightweight Rust CLI for browser automation via Chrome DevTools Protocol.

pwright connects to Chrome over CDP and lets you control it from the terminal.
The core workflow is **snapshot-act-snapshot**: take an accessibility snapshot,
interact with elements by ref, then snapshot again to verify.

```
Terminal                  pwright              Chrome
   │                        │                    │
   │  pwright open <url>    │   CDP/WebSocket    │
   │ ─────────────────────> │ ─────────────────> │
   │  pwright snapshot      │   AX tree query    │
   │ ─────────────────────> │ ─────────────────> │
   │  [e0] heading "Home"  │                    │
   │  [e1] link "Sign in"  │                    │
   │  pwright click e1      │   DOM + Input      │
   │ ─────────────────────> │ ─────────────────> │
```

## Quick Start

### 1. Start Chrome

```bash
# Linux
google-chrome --remote-debugging-port=9222

# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --remote-debugging-port=9222

# Windows (PowerShell)
& "C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222
```

> Append `&` (Linux/macOS) or use `Start-Process` (Windows) to run Chrome in the background.

Add `--headless=new` for headless mode (no visible window).

### 2. Install

```bash
# Install from GitHub
cargo install --git https://github.com/Shuozeli/pwright.git --locked pwright-cli

# Or build from source
cargo build --release
# Binary: target/release/pwright
```

### 3. First Command

```bash
pwright open https://example.com
```

This auto-discovers Chrome's WebSocket URL, creates a tab, navigates, and
saves session state to `.pwright/state.json`. All subsequent commands reuse
this session -- no need to re-specify connection details.

## Experimental composable Rust v2

The existing CLI above remains the supported v1 path. The parallel Rust v2
library now has a no-default-features umbrella and never downloads or launches
a browser unless a separate explicit component is selected and invoked:

```toml
# Portable navigation types only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["navigation"] }

# Portable by-value evaluation types only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["evaluation"] }

# Portable Page screenshot types only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["content"] }

# Portable Locator screenshot types only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["locator-screenshot"] }

# Portable read-only Frame DOM-content types only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["dom-content"] }

# Client-side Page navigation without CDP/server code
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-navigation"] }

# Client-side Frame/Page evaluation without CDP/server code
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-evaluation"] }

# Client-side Page screenshot without CDP/server code
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-content"] }

# Client-side Locator screenshot without CDP/server code
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-locator-screenshot"] }

# Client-side Frame/Page content without CDP/server code
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-dom-content"] }

# Attach-only Chromium navigation server without client/provider/installer code
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-navigation"] }

# Attach-only one-Frame Chromium evaluation server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-evaluation"] }

# Attach-only one-Page Chromium screenshot server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-content"] }

# Attach-only Chromium Locator screenshot composition with the first-party
# selector-geometry plugin; the underlying composition still accepts alternatives
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-locator-screenshot"] }

# Attach-only one-Frame Chromium DOM-content server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-dom-content"] }
```

See [the v2 dependency topology](docs/knowledge/pwright-dependency-topology.md)
for the exact compile-time dependency proofs and current limitations.

## The Snapshot Workflow

The snapshot is the core of how pwright works. It renders the page's
accessibility tree with stable element refs:

```bash
$ pwright snapshot
  [e0] heading "Example Domain"
  [e1] paragraph "This domain is for use in ..."
  [e2] link "More information..."
```

Use refs in action commands:

```bash
pwright click e2          # Click the link
pwright fill e5 "hello"   # Fill a text input
pwright press Enter       # Press a key
```

After navigation or form submission, take a fresh snapshot to get updated refs.

## CLI Commands

### Navigation

| Command | Description |
|---------|-------------|
| `pwright open <url>` | Connect to Chrome, open tab, navigate |
| `pwright goto <url>` | Navigate active tab |
| `pwright reload` | Reload current page |
| `pwright go-back` | Browser back |
| `pwright go-forward` | Browser forward |
| `pwright close` | Close active tab |

### Actions

All actions use **refs** from `pwright snapshot` (e.g., `e0`, `e1`, `e5`).

| Command | Description |
|---------|-------------|
| `pwright click <ref>` | Click element |
| `pwright dblclick <ref>` | Double-click element |
| `pwright fill <ref> "value"` | Set input value (clears first) |
| `pwright type "text"` | Type character-by-character |
| `pwright press <Key>` | Press key (Enter, Tab, Escape, ArrowDown...) |
| `pwright hover <ref>` | Hover over element |
| `pwright focus <ref>` | Focus element |
| `pwright select <ref> "value"` | Select dropdown option |
| `pwright scroll <ref>` | Scroll element into view |
| `pwright drag <ref> --dx N --dy N` | Drag element by offset |
| `pwright check <ref>` | Check checkbox |
| `pwright uncheck <ref>` | Uncheck checkbox |
| `pwright upload <ref> <files...>` | Upload files to file input |
| `pwright download <ref> [--dest path]` | Click and capture download |

### Inspection

| Command | Description |
|---------|-------------|
| `pwright snapshot` | Accessibility tree with element refs |
| `pwright text` | Extract page text content |
| `pwright eval "expression"` | Execute JavaScript, print result |
| `pwright health` | Check Chrome connectivity |

### Waiting

| Command | Description |
|---------|-------------|
| `pwright wait-for-text "text"` | Wait until page body contains text |
| `pwright wait-for "selector"` | Wait until CSS selector appears |
| `pwright wait-until "expr"` | Wait until JS expression is truthy |

### Export

| Command | Description |
|---------|-------------|
| `pwright screenshot [--filename name]` | Capture page as PNG |
| `pwright pdf [--filename name]` | Export page as PDF |
| `pwright capture-url <url> --dest path` | Save raw browser response bytes with status, headers, redirects, and a byte cap |

`capture-url` is intended for documents and endpoints that must pass through
Chrome but cannot be read with page JavaScript because of CORS or Chrome's PDF
viewer. Add `--context-url <issuer-page>` to establish cookies and referrer
context first, `--max-bytes N` to bound the decoded body, and `--json` for a
machine-readable result. Existing destination files are never overwritten.
Once the exact body and metadata are durably written, a transient
post-capture Fetch-disable or exact-tab-close failure is emitted as a
structured warning rather than changing the acquisition to failure. A failed
tab close retains its exact lease so the next same-endpoint invocation can
recover only that tab.

For shared Chrome processes with an unhealthy or unsuitable process-wide
proxy, set `PWRIGHT_EPHEMERAL_PROXY_SERVER`. `script run` and `capture-url`
then create their transient tab in an isolated browser context using that
proxy and dispose the context during exact-tab cleanup. Persistent
`open`/`goto` sessions and other profiles are unchanged.

### Tabs

| Command | Description |
|---------|-------------|
| `pwright tab-list` | List all open tabs |
| `pwright tab-new [url]` | Open new tab |
| `pwright tab-select <id>` | Switch active tab |
| `pwright tab-close [id]` | Close tab |

### Cookies

| Command | Description |
|---------|-------------|
| `pwright cookie-list` | List cookies |
| `pwright cookie-set --name N --value V --domain D` | Set a cookie |

### Options

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--cdp <url>` | `PWRIGHT_CDP` | `http://localhost:9222` | Chrome CDP endpoint |
| `--ephemeral-proxy-server <url>` | `PWRIGHT_EPHEMERAL_PROXY_SERVER` | unset | Proxy override for isolated `script run` and `capture-url` tabs |
| `--ephemeral-proxy-bypass-list <list>` | `PWRIGHT_EPHEMERAL_PROXY_BYPASS_LIST` | unset | Chromium-format bypass list for the isolated proxy |

## Examples

### Login Flow

```bash
pwright open https://example.com/login
pwright snapshot
# [e3] textbox "Email"
# [e4] textbox "Password"
# [e5] button "Sign in"
pwright fill e3 "user@example.com"
pwright fill e4 "password123"
pwright click e5
pwright snapshot   # verify login succeeded
```

### Data Extraction

```bash
pwright open https://example.com/pricing
pwright eval "JSON.stringify([...document.querySelectorAll('.price')].map(e => e.textContent))"
pwright screenshot --filename=pricing.png
```

### Multi-Tab

```bash
pwright open https://docs.example.com
pwright tab-new https://api.example.com
pwright tab-list
pwright tab-select <tab_id>
pwright snapshot
```

### Cookie-Based Auth

```bash
pwright open https://example.com
pwright cookie-set --name session_token --value abc123 --domain example.com
pwright reload
pwright snapshot   # now authenticated
```

## Session State

pwright persists connection state in `.pwright/state.json` so commands can be
chained across terminal invocations:

```bash
pwright open https://example.com   # saves state
pwright snapshot                   # reconnects automatically
pwright click e1                   # same session
pwright screenshot                 # same session
```

Add `.pwright/` to your `.gitignore`.

## Script Runner

pwright supports declarative YAML scripts for multi-step automation:

```yaml
# hello.yaml
name: "Extract title"
params:
  url: { type: string, required: true }
steps:
  - goto: "{{ url }}"
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
  - output:
      title: "{{ title }}"
```

```bash
pwright script run hello.yaml --param url=https://example.com

# Validate without executing
pwright script validate hello.yaml --param url=https://example.com
```

Scripts support parameters (`--param`, `--param-file`), a central JS registry,
error handling (`on_error: continue`), and produce structured JSONL output.
See [examples/scripts/](examples/scripts/) for more.

## Agent Integration

pwright is designed for AI agent workflows. The agent loop is:

```
snapshot -> read refs -> act on refs -> snapshot -> verify -> repeat
```

A skill definition for agent frameworks is available at `skill/pwright/SKILL.md`.

## gRPC API

pwright also exposes a gRPC server for programmatic access (21 RPCs covering
tabs, navigation, actions, content, cookies, downloads, and JS evaluation).

```bash
# Start the gRPC server (binds to 127.0.0.1:50051 by default)
pwright-server --cdp-url http://localhost:9222
```

See [gRPC API Reference](docs/grpc-api.md) for full request/response schemas.

## Docker Deployment

One-command setup with Chrome + gRPC server:

```bash
cd deploy
docker compose up --build -d
# gRPC available at localhost:50051
```

See [deploy/README.md](deploy/README.md) for details.

## Documentation

- [Architecture](docs/architecture.md) -- crate structure, layer design, concurrency model
- [Composable Architecture v2](docs/architecture-v2.md) -- isolated pluggable rewrite foundation, implementation boundaries, and attach-versus-launch ownership
- [Pwright Rust v2 Design Tasks](docs/design-tasks-v2.md) -- design gates and verified implementation checkpoints
- [Getting Started](docs/getting-started.md) -- prerequisites, Chrome setup, first use
- [CLI Guide](docs/cli-guide.md) -- full command reference
- [gRPC API Reference](docs/grpc-api.md) -- all RPCs with request/response schemas
- [Codelabs](docs/codelabs.md) -- usage examples
- [TODO](docs/todo.md) -- planned features and roadmap
- [Tech Debt](docs/tech-debt.md) -- tracked structural issues

## Prerequisites

- Rust 2024 edition (1.85+)
- Google Chrome or Chromium
- `protoc` (protocol buffer compiler) -- for building from source

## Security

By default, the gRPC server binds to `127.0.0.1` only. Use `--bind-all` with caution
and always firewall both the CDP (9222) and gRPC (50051) ports.

## License

MIT
