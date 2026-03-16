# pwright CLI vs chrome-devtools-mcp CLI

Feature comparison against Google's official Chrome DevTools MCP CLI.

**Repo:** https://github.com/ChromeDevTools/chrome-devtools-mcp

## Architecture

| | pwright CLI | chrome-devtools CLI |
|---|---|---|
| Language | Rust (~6MB binary) | Node.js (npm) |
| Protocol | Direct CDP WebSocket | Puppeteer (wraps CDP) |
| Browser mgmt | Attach-only | Launches + manages Chrome |
| State | File-based (`.pwright/state.json`) | Background daemon (Unix socket) |
| MCP support | No | Yes (MCP server + CLI client) |
| Slim mode | N/A | 3-tool minimal mode |

## Tool Coverage Comparison

### Input Automation

| Tool | pwright | CD-MCP | Notes |
|------|:-------:|:------:|-------|
| click | `click eN` | `click uid` | Both use a11y tree refs |
| click-at | `click-at x y` | -- | **pwright only**: coordinate-based real CDP input |
| dblclick | `dblclick eN` | `click uid --dblClick` | CD-MCP uses flag |
| fill | `fill eN "val"` | `fill uid "val"` | Equivalent |
| fill_form (batch) | -- | `fill_form [...]` | **CD-MCP only**: fills multiple form elements at once |
| type | `type "text"` | `type_text "text"` | Equivalent |
| press key | `press Enter` | `press_key Enter` | Equivalent |
| hover | `hover eN` | `hover uid` | Equivalent |
| hover-at | `hover-at x y` | -- | **pwright only**: coordinate-based hover |
| drag | `drag eN --dx --dy` | `drag from to` | pwright uses offset, CD-MCP uses target element |
| upload | `upload eN ./file` | `upload_file uid path` | Equivalent |
| select | `select eN "val"` | -- | **pwright only** |
| focus | `focus eN` | -- | **pwright only** |
| check/uncheck | `check eN` / `uncheck eN` | -- | **pwright only** |
| handle_dialog | -- | `handle_dialog accept/dismiss` | **CD-MCP only**: alert/confirm/prompt |

### Navigation

| Tool | pwright | CD-MCP | Notes |
|------|:-------:|:------:|-------|
| navigate | `open url` / `goto url` | `navigate_page url` | Equivalent |
| reload | `reload` | `navigate_page --type reload` | pwright has dedicated command |
| back/forward | `go-back` / `go-forward` | `navigate_page --type back/forward` | pwright has dedicated commands |
| new tab | `tab-new url` | `new_page url` | CD-MCP adds `--background`, `--isolatedContext` |
| list tabs | `tab-list` | `list_pages` | Equivalent |
| select tab | `tab-select id` | `select_page id` | Equivalent |
| close tab | `tab-close id` | `close_page id` | Equivalent |
| wait_for (text) | -- | `wait_for "text1" "text2"` | **CD-MCP only** (planned P1 for pwright) |
| initScript | -- | `navigate_page --initScript "..."` | **CD-MCP only**: inject JS before page scripts |

### Content Extraction

| Tool | pwright | CD-MCP | Notes |
|------|:-------:|:------:|-------|
| a11y snapshot | `snapshot` | `take_snapshot` | Equivalent |
| screenshot | `screenshot` | `take_screenshot` | CD-MCP adds element screenshot by uid |
| PDF export | `pdf` | -- | **pwright only** |
| JS evaluate | `eval "expr"` | `evaluate_script "fn"` | CD-MCP requires function syntax |
| typed evaluate | `evaluate_into::<T>()` (Rust) | -- | **pwright only** |
| cookies | `cookie-list` / `cookie-set` | -- | **pwright only** |
| download | `download eN` | -- | **pwright only** |

### Debugging & Performance (CD-MCP only)

| Tool | pwright | CD-MCP |
|------|:-------:|:------:|
| performance_start_trace | -- | Yes |
| performance_stop_trace | -- | Yes |
| performance_analyze_insight | -- | Yes |
| take_memory_snapshot | -- | Yes |
| list_network_requests | `network-listen` / `network-list` | Yes |
| get_network_request | `network-get <reqid>` | Yes |
| list_console_messages | -- | Yes |
| get_console_message | -- | Yes |
| lighthouse_audit | -- | Yes |

### Emulation (CD-MCP only)

| Tool | pwright | CD-MCP |
|------|:-------:|:------:|
| emulate (viewport, network, CPU, geo) | -- | Yes |
| resize_page | -- | Yes |

### Automation (pwright only)

| Tool | pwright | CD-MCP |
|------|:-------:|:------:|
| YAML script runner | `script run` / `script validate` | -- |
| Param files | `--param-file secrets.yaml` | -- |
| JSONL output | Yes (structured per-step) | -- |
| Script JS registry | Yes | -- |

## Actionable Gaps (pwright should consider)

### High Priority

1. **`wait_for` (text-based)** - Already planned P1. CD-MCP takes an array of texts
   and resolves when any appears. Implement similarly.

2. **`handle_dialog`** - Alert/confirm/prompt handling. CD-MCP uses
   `Page.javascriptDialogOpening` CDP event. Common need for form automation.

### Medium Priority

3. **`fill_form` (batch fill)** - Fill multiple form elements in one call.
   Reduces round-trips for complex forms. Could be a recipe pattern instead.

4. **Element screenshot by uid** - CD-MCP's `take_screenshot --uid` captures
   a single element. pwright only does full-page/viewport.

5. **`initScript`** - Inject JS before page scripts load (via
   `Page.addScriptToEvaluateOnNewDocument`). Useful for mocking APIs.

### Out of Scope (intentional)

- **Performance tracing** - DevTools profiling, not automation
- **Network inspection** - Debugging tool, pwright has `on_request`/`on_response` for automation
- **Console messages** - Debugging tool
- **Lighthouse** - Auditing tool
- **Emulation** - Device emulation, network throttling
- **MCP server** - pwright is a library/CLI, not an MCP server (could be built on top)
- **Browser launch/management** - pwright is attach-only by design
