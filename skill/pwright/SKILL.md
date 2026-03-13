---
name: pwright
description: Automates browser interactions via Chrome CDP for web testing, form filling, screenshots, and data extraction. Use when the user needs to navigate websites, interact with web pages, fill forms, take screenshots, test web applications, or extract information from web pages. Requires a running Chrome with --remote-debugging-port=9222.
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
# Windows (PowerShell)
# Start-Process "C:\Program Files\Google\Chrome\Application\chrome.exe" "--headless=new","--remote-debugging-port=9222"

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
2. **Snapshot** the accessibility tree (`pwright snapshot`) — get refs like `e0`, `e5`, `e12`
3. **Act** on refs — `click e5`, `fill e3 "text"`, `press Enter`
4. **Snapshot** again to see results
5. **Repeat** step 3-4 until done

Refs are stable between snapshots — no need to re-snapshot before every action. Only snapshot after the page changes significantly (navigation, form submission, etc.).

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
pwright fill e3 "user@example.com" # Set input value
pwright type "search text"         # Type character by character
pwright press Enter                # Press keyboard key
pwright press Tab                  # Tab to next field
pwright hover e4                   # Hover over element
pwright select e9 "option-value"   # Select dropdown option
pwright focus e2                   # Focus an element
pwright drag e7 --dx 100 --dy 0   # Drag element by offset
pwright upload e8 ./file.pdf       # Upload file to file input
```

### Inspection

```bash
pwright snapshot                   # Accessibility tree with refs
pwright eval "document.title"      # Run JavaScript
pwright eval "document.querySelectorAll('a').length"
```

### Screenshots & Export

```bash
pwright screenshot                 # Screenshot → screenshot.png
pwright screenshot --filename=page.png
pwright pdf                        # PDF export → page.pdf
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

### State Persistence

After `pwright open`, the CLI saves session state to `.pwright/state.json` in the current directory. This stores the WebSocket URL, active tab ID, and target ID — so subsequent commands don't need to reconnect.

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
