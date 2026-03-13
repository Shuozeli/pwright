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
| `pwright scroll <ref> --dx N --dy N` | Scroll element |
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

### Export

| Command | Description |
|---------|-------------|
| `pwright screenshot [--filename name]` | Capture page as PNG |
| `pwright pdf [--filename name]` | Export page as PDF |

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

## Agent Integration

pwright is designed for AI agent workflows. The agent loop is:

```
snapshot -> read refs -> act on refs -> snapshot -> verify -> repeat
```

A skill definition for agent frameworks is available at `skill/pwright/SKILL.md`.

## gRPC API

pwright also exposes a gRPC server for programmatic access (20 RPCs covering
tabs, navigation, actions, content, cookies, downloads, and JS evaluation).

```bash
# Start the gRPC server (binds to 127.0.0.1:50051 by default)
pwright-server --cdp-url http://localhost:9222
```

See [gRPC API Reference](docs/grpc-api.md) for full request/response schemas.

## Documentation

- [Architecture](docs/architecture.md) -- crate structure, layer design, concurrency model
- [Getting Started](docs/getting-started.md) -- prerequisites, Chrome setup, first use
- [CLI Guide](docs/cli-guide.md) -- full command reference
- [gRPC API Reference](docs/grpc-api.md) -- all RPCs with request/response schemas
- [Codelabs](docs/codelabs.md) -- usage examples

## Prerequisites

- Rust 2024 edition (1.85+)
- Google Chrome or Chromium
- `protoc` (protocol buffer compiler) -- for building from source

## Security

By default, the gRPC server binds to `127.0.0.1` only. Use `--bind-all` with caution
and always firewall both the CDP (9222) and gRPC (50051) ports.

## License

MIT
