# CLI Guide

pwright provides a command-line interface for browser automation. It connects to a running Chrome via CDP and persists session state so you can chain commands across terminal invocations.

## Setup

### 1. Start Chrome

```bash
# Linux
google-chrome --headless=new --remote-debugging-port=9222 &

# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --headless=new --remote-debugging-port=9222 &

# Windows (PowerShell)
Start-Process "C:\Program Files\Google\Chrome\Application\chrome.exe" "--headless=new","--remote-debugging-port=9222"
```

> Remove `--headless=new` for a visible Chrome window (useful for debugging).

### 2. Install pwright

```bash
cd /path/to/pwright
cargo build --release
# Binary: target/release/pwright
```

### 3. First Use

```bash
pwright open https://example.com
```

This auto-discovers Chrome's WebSocket URL, creates a tab, navigates, and saves session state to `.pwright/state.json`.

---

## Command Reference

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
| `pwright fill <ref> "value"` | Set input value |
| `pwright type "text"` | Type character-by-character |
| `pwright press <Key>` | Press keyboard key (Enter, Tab, Escape...) |
| `pwright hover <ref>` | Hover over element |
| `pwright select <ref> "value"` | Select dropdown option |
| `pwright focus <ref>` | Focus element |
| `pwright check <ref>` | Check checkbox |
| `pwright uncheck <ref>` | Uncheck checkbox |
| `pwright scroll <ref>` | Scroll element into view |
| `pwright drag <ref> --dx N --dy N` | Drag element by offset |
| `pwright upload <ref> <files...>` | Upload files to file input |
| `pwright download <ref> [--dest path]` | Click + capture download |

### Coordinate-Based Actions

For pages with empty accessibility trees (Gmail, Google Docs, heavy SPAs), use coordinate-based commands that send real CDP input events:

| Command | Description |
|---------|-------------|
| `pwright click-at <x> <y>` | Click at viewport coordinates |
| `pwright click-at <x> <y> --button right` | Right-click at coordinates |
| `pwright click-at <x> <y> --click-count 2` | Double-click at coordinates |
| `pwright hover-at <x> <y>` | Hover at viewport coordinates |

Use `pwright eval` to find element coordinates via `getBoundingClientRect()`, then `click-at` to interact.

### Inspection

| Command | Description |
|---------|-------------|
| `pwright snapshot` | Accessibility tree with element refs |
| `pwright text` | Extract visible page text content |
| `pwright eval "expression"` | Execute JavaScript |
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

### Scripts

| Command | Description |
|---------|-------------|
| `pwright script run <script.yaml>` | Execute a YAML automation script |
| `pwright script run <script.yaml> --param key=val` | Pass parameters |
| `pwright script run <script.yaml> --param-file secrets.yaml` | Load params from file |
| `pwright script validate <script.yaml>` | Validate script without executing |

### Network Capture

| Command | Description |
|---------|-------------|
| `pwright network-listen` | Stream network traffic as JSONL |
| `pwright network-listen --filter "/api/"` | Filter by URL substring |
| `pwright network-listen --type XHR` | Filter by resource type |
| `pwright network-listen --duration 30` | Stop after N seconds |
| `pwright network-list` | List resources on current page (retroactive) |
| `pwright network-list --filter ".js"` | Filter resource list |
| `pwright network-get <reqid>` | Get response body by request ID |
| `pwright network-get <reqid> --output data.json` | Save body to file |

> Run `network-listen` in a separate terminal while interacting with the site. It uses a second CDP session so it does not interfere with your commands. The `reqid` in listener output can be used with `network-get`.

### Options

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--cdp <url>` | `PWRIGHT_CDP` | `http://localhost:9222` | Chrome CDP HTTP endpoint |

---

## Working with Snapshots

The snapshot is the key to interacting with pages. It shows the accessibility tree with stable refs:

```bash
$ pwright snapshot
  [e0] heading "Example Domain"
  [e1] paragraph "This domain is for use in ..."
  [e2] link "More information..."
```

Use ref values in action commands:

```bash
pwright click e2      # Click "More information..." link
pwright fill e5 "hi"  # Fill a text input
```

Refs remain valid until the page changes significantly. After navigation or form submission, take a new snapshot.

---

## Session State (`.pwright/`)

After `pwright open`, the CLI creates `.pwright/state.json`:

```json
{
  "cdp_url": "http://localhost:9222",
  "ws_url": "ws://localhost:9222/devtools/browser/...",
  "active_tab": "tab_00000000",
  "target_id": "5C2B2F5BA250506DB3351C1D0376B41C"
}
```

This enables chaining commands without re-specifying connection details:

```bash
pwright open https://example.com  # Saves state
pwright snapshot                  # Reads state, reconnects
pwright click e1                  # Same session
pwright screenshot                # Same session
```

> Add `.pwright/` to your `.gitignore`.

---

## Agent Setup

To use pwright as a tool for AI agents, the agent needs:

1. **Chrome running** with `--remote-debugging-port=9222`
2. **pwright binary** available on `$PATH`
3. **The snap-act loop**: snapshot → act on refs → snapshot again

### Agent Workflow Pattern

```
Agent receives task → pwright open <url> → pwright snapshot → 
read refs → pwright click/fill/press → pwright snapshot → 
verify result → repeat or close
```

### Best Practices for Agents

- **Always snapshot before acting** on a new page
- **Use `pwright eval`** for complex data extraction rather than parsing snapshot text
- **Chain commands** — state persists across calls
- **After form submission or navigation**, take a fresh snapshot
- **Use `--cdp`** flag if Chrome is on a non-default port

### Skill File

For AI agent frameworks that support skill definitions, pwright provides a skill file at:

```
skill/pwright/SKILL.md
```

This follows the same pattern as `playwright-cli` and `pinchtab` skill definitions.

---

## Examples

### Login Flow

```bash
pwright open https://example.com/login
pwright snapshot

# Find email/password fields and submit button from snapshot
pwright fill e1 "user@example.com"
pwright fill e2 "password123"
pwright click e3

pwright snapshot  # Verify login succeeded
pwright close
```

### Data Extraction

```bash
pwright open https://example.com/pricing
pwright eval "JSON.stringify([...document.querySelectorAll('.price')].map(e => e.textContent))"
pwright screenshot --filename=pricing.png
pwright close
```

### Multi-Tab Workflow

```bash
pwright open https://docs.example.com
pwright tab-new https://api.example.com
pwright tab-list

# Switch between tabs
pwright tab-select <tab_id_from_list>
pwright snapshot
pwright close
```

### Cookie-Based Session

```bash
pwright open https://example.com
pwright cookie-set --name session_token --value abc123 --domain example.com
pwright reload
pwright snapshot  # Now authenticated
pwright close
```

### Script Automation

For multi-step workflows, use YAML scripts instead of chaining CLI commands:

```yaml
# scrape.yaml
name: "Scrape prices"
params:
  url: { type: string, required: true }

scripts:
  get_prices: |
    [...document.querySelectorAll('.price')]
      .map(el => el.textContent.trim())

steps:
  - goto: "{{ url }}"
    wait_for: ".price"
  - eval:
      ref: get_prices
      save_as: prices
  - output:
      prices: "{{ prices }}"
```

```bash
pwright script run scrape.yaml --param url=https://example.com/products
```

Output (JSONL):
```jsonl
{"step_index":0,"step_type":"goto","status":"ok","duration_ms":1200}
{"step_index":1,"step_type":"eval","status":"ok","duration_ms":15}
{"step_index":2,"step_type":"output","status":"ok","details":{"prices":"[...]"}}
{"summary":true,"status":"ok","total_steps":3,"succeeded":3}
```

Scripts support:
- **Parameters**: `--param key=val` or `--param-file secrets.yaml`
- **JS registry**: Complex JS defined once in `scripts:`, referenced by `eval.ref`
- **Error handling**: Per-step `on_error: fail | continue`
- **Validation**: `--validate` checks params, template refs, JS refs without executing

See [examples/scripts/](../examples/scripts/) for more examples.
