---
name: pwright
description: Automates browser interactions via Chrome CDP for web testing, form filling, screenshots, data extraction, and scripted automation. Use when the user needs to navigate websites, interact with web pages, fill forms, take screenshots, test web applications, extract information from web pages, or run multi-step browser workflows. Requires a running Chrome with --remote-debugging-port=9222.
allowed-tools: Bash(pwright:*)
---

# Browser Automation with pwright

## Quick Start

```bash
# 1. Make sure Chrome is running with remote debugging
# Linux
google-chrome --headless=new --remote-debugging-port=9222 &
# macOS
/Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome --headless=new --remote-debugging-port=9222 &

# 2. Navigate to a page (auto-connects to Chrome)
pwright open https://example.com

# 3. Get the accessibility snapshot (shows refs like e0, e1, e5)
pwright snapshot

# 4. Interact using refs from the snapshot
pwright click e1
pwright fill e3 "search query"
pwright press Enter

# 5. Snapshot again to see results
pwright snapshot

# 6. Close when done
pwright close
```

## Core Workflow

The agent loop for browser tasks:

1. **Navigate** to a URL with `pwright open <url>` or `pwright goto <url>`
2. **Snapshot** the accessibility tree (`pwright snapshot`) -- get refs like `e0`, `e5`, `e12`
3. **Act** on refs -- `click e5`, `fill e3 "text"`, `press Enter`
4. **Snapshot** again to see results
5. **Repeat** step 3-4 until done

Refs are stable between snapshots -- no need to re-snapshot before every action. Only snapshot after the page changes significantly (navigation, form submission, etc.).

## Commands

### Navigation

```bash
pwright open https://example.com   # Connect + navigate (first command)
pwright goto https://other.com     # Navigate active tab to URL
pwright reload                     # Reload current page
pwright go-back                    # Browser back button
pwright go-forward                 # Browser forward button
```

### Actions (use refs from snapshot)

```bash
pwright click e5                   # Click element
pwright dblclick e5                # Double-click element
pwright fill e3 "user@example.com" # Set input value
pwright type "search text"         # Type character by character
pwright press Enter                # Press keyboard key
pwright press Tab                  # Tab to next field
pwright hover e4                   # Hover over element
pwright select e9 "option-value"   # Select dropdown option
pwright focus e2                   # Focus an element
pwright check e6                   # Check checkbox
pwright uncheck e6                 # Uncheck checkbox
pwright drag e7 --dx 100 --dy 0   # Drag element by offset
pwright upload e8 ./file.pdf       # Upload file to file input
pwright click-at 282 293           # Click at viewport coordinates (real CDP input)
pwright click-at 282 293 --button right  # Right-click at coordinates
pwright click-at 282 293 --click-count 2 # Double-click at coordinates
pwright hover-at 282 293           # Hover at viewport coordinates
```

> **Coordinate-based commands** (`click-at`, `hover-at`) send real CDP input events
> that work on SPAs with empty accessibility trees (e.g. Gmail, Google Docs). Use
> `pwright eval` to get element coordinates via `getBoundingClientRect()`, then
> `click-at` to interact.

### Inspection

```bash
pwright snapshot                   # Accessibility tree with refs
pwright eval "document.title"      # Run JavaScript
pwright eval "document.querySelectorAll('a').length"
```

### Screenshots & Export

```bash
pwright screenshot                 # Screenshot -> screenshot.png
pwright screenshot --filename=page.png
pwright pdf                        # PDF export -> page.pdf
pwright pdf --filename=report.pdf
```

### Tabs

```bash
pwright tab-list                   # List all tabs
pwright tab-new https://other.com  # Open new tab
pwright tab-select <tab_id>        # Switch active tab
pwright tab-close                  # Close active tab
pwright tab-close <tab_id>         # Close specific tab
```

### Cookies

```bash
pwright cookie-list                # List cookies
pwright cookie-set --name session --value abc123 --domain example.com
```

### Download

```bash
pwright download e5                # Click e5 and capture download
pwright download e5 --dest ./file.pdf  # Save to specific path
```

### Script Runner

For multi-step automation, use YAML scripts instead of chaining commands:

```bash
# Run a script with parameters
pwright script run scraper.yaml --param url=https://example.com

# Validate without executing
pwright script validate scraper.yaml --param url=https://example.com

# Load params from file (for credentials)
pwright script run scraper.yaml --param-file secrets.yaml
```

Script example:

```yaml
name: "Extract title"
params:
  url: { type: string, required: true }
scripts:
  get_links: |
    JSON.stringify([...document.querySelectorAll('a')].map(a => a.href))
steps:
  - goto: "{{ url }}"
    wait_for: "h1"
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
  - eval: { ref: get_links, save_as: links }
  - wait: 2000
  - output:
      title: "{{ title }}"
      links: "{{ links }}"
```

Scripts support: `goto`, `click`, `fill`, `press`, `extract`, `eval` (with JS registry),
`output`, `wait` (sleep), and `on_error: continue` for graceful error handling.

### System

```bash
pwright health                     # Check Chrome connectivity
pwright close                      # Close active tab
```

## Keyboard Keys

Common key names for `pwright press`:

| Key | Name |
|-----|------|
| Enter | `Enter` |
| Tab | `Tab` |
| Escape | `Escape` |
| Backspace | `Backspace` |
| Delete | `Delete` |
| Space | `Space` |
| Arrow keys | `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight` |
| Modifiers | `Shift`, `Control`, `Alt`, `Meta` |
| Function keys | `F1` through `F12` |

## What a Snapshot Looks Like

```
pwright snapshot
```

Output:

```
  [e0] heading "Example Domain"
  [e1] link "More information..."
  [e2] textbox "Search" value=""
  [e3] button "Submit"
```

Each `[eN]` is a ref you can use in action commands: `pwright click e1`, `pwright fill e2 "hello"`.

## CDP Connection

pwright connects to Chrome via the Chrome DevTools Protocol (CDP). Chrome must be running with remote debugging enabled.

```bash
# Default: connects to http://localhost:9222
pwright open https://example.com

# Custom CDP endpoint
pwright --cdp http://192.168.1.100:9222 open https://example.com

# Or via environment variable
PWRIGHT_CDP=http://remote-host:9222 pwright open https://example.com
```

### Docker Deployment

For a one-command setup with Chrome + gRPC server:

```bash
cd deploy
docker compose up --build -d
# gRPC available at localhost:50051
```

### State Persistence

After `pwright open`, the CLI saves session state to `.pwright/state.json` in the current directory. This stores the WebSocket URL, active tab ID, and target ID -- so subsequent commands don't need to reconnect.

Add `.pwright/` to your `.gitignore`.

### Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| "cannot connect to Chrome" | Chrome not running or wrong port | Run `google-chrome --headless=new --remote-debugging-port=9222 &` |
| "no webSocketDebuggerUrl" | Chrome started without debug flag | Restart Chrome with `--remote-debugging-port=9222` |
| "no active tab" | No `pwright open` was run | Run `pwright open <url>` first |
| "failed to reattach tab" | Chrome restarted since last session | Run `pwright open <url>` again |

Verify Chrome is reachable:

```bash
curl -s http://localhost:9222/json/version | jq .webSocketDebuggerUrl
# Should output: "ws://localhost:9222/devtools/browser/..."
```

## Example: Form Submission

```bash
pwright open https://example.com/login
pwright snapshot
pwright fill e1 "user@example.com"
pwright fill e2 "password123"
pwright click e3
pwright snapshot
pwright close
```

## Example: Multi-Tab Research

```bash
pwright open https://docs.example.com
pwright snapshot
pwright tab-new https://api.example.com/reference
pwright snapshot
pwright tab-list
pwright tab-select <first_tab_id>
pwright close
```

## Example: Data Extraction

```bash
pwright open https://example.com/data
pwright eval "JSON.stringify([...document.querySelectorAll('tr')].map(r => r.textContent))"
pwright screenshot --filename=data-page.png
pwright close
```

## Example: Scripted Scraping

```bash
# Create a script
cat > scrape.yaml << 'EOF'
name: "Scrape prices"
params:
  url: { type: string, required: true }
scripts:
  get_prices: |
    JSON.stringify([...document.querySelectorAll('.price')].map(e => e.textContent))
steps:
  - goto: "{{ url }}"
    wait_for: ".price"
  - eval: { ref: get_prices, save_as: prices }
  - output: { prices: "{{ prices }}" }
EOF

# Run it
pwright script run scrape.yaml --param url=https://example.com/products
```
