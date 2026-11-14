<!-- agent-updated: 2026-10-10T11:25:08Z -->

# pwright

A lightweight Rust CLI for browser automation via Chrome DevTools Protocol.

The ongoing Rust-backed Playwright compatibility work has pinned the complete
48,477-invocation Core JS/TS and 6,526-invocation Python v1.58.0 inventories.
Recent browserless BrowserType evidence adds 15 Core and four Python outcomes;
an additional 12-row Core launch-option preflight matrix is paired-green,
including 11 newly run rows. Valid browser/server launch remains unsupported.
Full-corpus execution and several compatibility/lifecycle gates remain open.
Core455 separately accounts for 18 upstream-private, no-I/O source-only
invocations in each lane; these do not attest Pwright public API parity. The
opt-in Rust persistent-Context preflight matches 14/14 browserless pinned
public-method comparisons without adding an official corpus outcome.
See the [Core conformance ledger](docs/upstream-playwright-conformance.md)
and [Python conformance ledger](docs/python-official-conformance.md).

NODE-432 completes outcome parity for the unchanged nine-case Chromium Page
response-wait file: candidate and Playwright 1.58.0 oracle each record eight
expected outcomes plus the same Chrome-154 cached-image unexpected result.
`waitForResponse()` and generic Response events compose over one bounded Rust
network observer. A separate `response-finished` leaf makes body predicates
wait for terminal readiness while keeping the minimal `response-events`
artifact free of body, routing and request-history capabilities. The complete
Core and Python corpora remain open.

NODE-431 completes the unchanged eight-case Chromium Page request-wait file.
The shared JavaScript event waiter now reproduces Playwright timeout identity,
quoted event diagnostics, caller stacks, long-matcher logs and `timeout: 0`,
while Page-default timeout state, request observation, bounded queues and URL
glob compilation remain in Rust. Candidate and independent oracle pass 8/8;
the complete Core and Python corpora remain open.

NODE-430 adds independently selectable Node request-event and post-data parser
profiles over the existing bounded Rust network lifecycle. `waitForRequest`
arms synchronously in JavaScript but request observation, lifecycle storage,
URL-glob compilation, post-data decoding and resource limits remain in Rust.
The two minimal native profiles now have explicit build gates so composability
is tested independently from the full official-conformance binary.
The complete unchanged Chromium request post-data file passes 6/6 in both
candidate and Playwright oracle lanes; the full Core and Python corpora remain
open.

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

NODE-429 adds an independent `response-request` composition over frame
navigation and bounded request history. `Response.request()` now exposes the
actual Rust-captured navigation Request snapshot without selecting Page routes
or rebuilding sent headers in JavaScript. Candidate and Playwright 1.58.0
oracle agree on all seven complete Chromium extra-header cases; Chrome 154
turns the upstream Referer expected-failure into the same unexpected pass in
both lanes. Full-corpus conformance remains open.

NODE-362 composes target-origin TLS over already-established HTTP CONNECT and
SOCKS5 tunnels. The generic transport port takes ownership of an existing
connection; Rustls implements SNI, certificate verification and exact-origin
client-certificate selection without opening another socket, while both proxy
leaves remain independent of the TLS crate. Two exact-union Node profiles add
certificate plus HTTP or SOCKS selection. The unchanged official proxy+mTLS
rows pass 18/18 candidate and 18/18 oracle across Library and Bidi, without a
browser or CDP. The Core ledger advances to candidate 1,309 expected / one
skipped / 31 unexpected / 47,136 not run and oracle 992 expected / 47,485 not
run. Full-corpus execution, proxy bypass and authenticated SOCKS remain open.

NODE-361 executes the eight already-supported global-request client-
certificate rows through all six Bidi projects. The input-validation row
passes 6/6 and the seven direct-request behaviors pass 42/42 in both candidate
and oracle lanes. A freshly built release profile loads before the candidate
run; no browser or CDP is used. The Core ledger advances to candidate 1,291
expected / one skipped / 31 unexpected / 47,154 not run and oracle 974
expected / 47,503 not run.

NODE-360 executes the same three unchanged global-request proxy rows through
the six-project Bidi matrix. Authenticated HTTP CONNECT, HTTPS proxy-server
ALPN and SOCKS5 each pass 6/6 candidate and 6/6 oracle without CDP, adding 18
verified invocations to each ledger without a runtime change. The Core ledger
is now candidate 1,243 expected / one skipped / 31 unexpected / 47,202 not run
and oracle 926 expected / 47,551 not run.

NODE-359 adds a separate `api-request-socks5-proxy` profile and Rust SOCKS5
connector. The leaf performs a bounded no-auth SOCKS5 handshake, sends domain
names to the proxy for remote resolution and supports IPv4/IPv6 target forms;
the base request and HTTP CONNECT profiles do not select it. The unchanged
official Playwright 1.58.0 SOCKS row passes 3/3 candidate and 3/3 oracle across
the Library projects without CDP. The Core ledger advances to candidate 1,225
expected / one skipped / 31 unexpected / 47,220 not run and oracle 908
expected / 47,569 not run. HTTPS target tunneling, authenticated SOCKS and
proxy bypass remain open.

NODE-358 composes HTTPS proxy-server transport into the same optional HTTP
CONNECT leaf. The target origin is still plain HTTP; the injected Rustls
connector now owns TLS to an `https:` proxy and forwards the request's
`ignoreHTTPSErrors` policy. The unchanged official ALPN row passes 3/3
candidate and 3/3 oracle across the Library projects. The Core ledger advances
to candidate 1,222 expected / one skipped / 31 unexpected / 47,223 not run and
oracle 905 expected / 47,572 not run. HTTPS target tunneling, SOCKS and bypass
remain open.

NODE-357 adds the separately selectable Node `api-request-http-proxy` profile.
Its Rust-only HTTP CONNECT leaf owns bounded proxy parsing, redacted Basic
credentials, the CONNECT handshake and prefetched tunnel bytes; the base
`api-request` profile still does not select proxy code. The unchanged official
Playwright 1.58.0 global-request proxy row passes 3/3 candidate and 3/3 oracle
across Chromium, WebKit and Firefox Library projects without CDP. The Core
ledger advances to candidate 1,219 expected / one skipped / 31 unexpected /
47,226 not run and oracle 902 expected / 47,575 not run. HTTPS-over-proxy,
SOCKS, bypass and BrowserContext proxy composition remain open.

NODE-356 keeps ABI 96 and closes the complete async/sync Chromium request-
fallback files. Request headers now expose Playwright's lowercase public map
plus `all_headers()` / case-insensitive `header_value()`. A bounded 64-entry
facade identity cache preserves Rust's effective fallback URL, method, headers
and body until the terminal request event, and sync route handlers run off the
bridge loop so nested sync Page calls cannot deadlock it. Source candidate and
independent Playwright 1.58.0 oracle each pass all 55 rows (26 async, 29 sync),
advancing the provisional ledger to 1,070 passed, zero failed, seven skipped
and 5,449 not run. Two release-wheel payloads reproduce after CycloneDX
normalization and installed-wheel units pass 207/207. Promotion to final
installed-wheel browser evidence remains pending because the shared dedicated
Chrome CDP endpoint became unresponsive during another concurrent task; it was
not restarted, so unknown tabs remain intact.

NODE-355 advances Python to ABI 96 with opt-in `context-api-request` and
`route-fetch` compositions. Rust owns BrowserContext cookie sharing, browser
user agent, TLS/credential/base-URL policy, typed query and body preparation,
request lifetime, `Route.fetch()` and response-backed fulfillment; Python is a
thin async/sync facade and does not serialize HTTP wire bytes. Final installed
candidate and independent Playwright 1.58.0 oracle both pass all 108 selected
Chromium rows: context request async 51/51, sync 47/47, route fetch async 6/6
and sync 4/4. The authoritative ledger is now 1,015 passed, zero failed, seven
skipped and 5,504 not run. Full Python and Core JS/TS execution remain open.

NODE-354 advances Python to ABI 95 with an opt-in
`service-worker-policy` feature layered only on `init-scripts`. Rust owns the
exact Playwright block script, transactional installation and Context-scoped
cleanup; the Python async/sync facades only validate `"allow"` / `"block"`
and forward the choice. The final installed wheel passes all four source-pinned
async/sync Chromium rows with the required secure fixture, plus independent
20/20 async and 20/20 sync stress runs. Official Playwright 1.58.0 cannot
initialize its oracle Context against Chrome 154 because that driver issues
`Browser.setDownloadBehavior`, which Chrome rejects before test assertions.
Accordingly, the four rows remain explicitly `not-run` and the authoritative
ledger remains 907 passed, zero failed, seven skipped and 5,612 not run.

NODE-353 keeps Python ABI 94 and fixes paused-popup event composition in Rust.
An opening dialog may publish the provisional popup Page before world creation
settles, immediate-close console Handles finish before detach compensation,
and host teardown revokes retained Dialog actions before closing the CDP
transport. Final candidate and official Playwright agree on 11 applicable
BrowserContext-event passes; one shared Chrome 154 source/oracle drift remains
explicitly `not-run`. The cumulative Python ledger reaches 907 passed, zero
failed, seven skipped and 5,612 not run.

NODE-346 advances Python to ABI 93 with a separately selectable
`route-redirects` composition, exactly the dependency union of `page-routes`
and `request-history`. Rust auto-continues Chromium redirect pauses without a
second user callback or finite-use decrement, retains at most 64 explicit
header overrides, and strips `Cookie` before reapplying those headers. The
existing Rust request-history owner supplies exact Request identity; thin
Python links `redirected_from` / `redirected_to`. The two unchanged official
redirect rows move from candidate 0/2 to final installed candidate and
independent oracle 2/2. The complete Page-route file reaches 43 passed / 1
failed and the cumulative ledger reaches 853 passed, zero failed, seven
skipped and 5,666 not run. Service-worker interception remains the sole
Page-route discrepancy; the broader Python and Core JS/TS corpora remain open.

NODE-345 advances Python to ABI 92 with a separately selectable
`route-fulfill-api-response` adapter over Page routing and the Rust API request
client. Rust inherits status, headers and content type from the retained
`APIResponse`, rejects a body above the existing 1 MiB route bound before
cloning, and fulfills the route without moving response bytes through Python.
The unchanged official global-fetch row moves from candidate 0/1 to final
installed candidate and independent oracle 1/1. The complete Page-route file
improves to 41 passed / 3 failed, and cumulative Python execution reaches 851
passed, zero failed, seven skipped and 5,668 not run.

NODE-344 keeps Python at ABI 91 and aligns browser-generated CORS preflights
with Playwright. When a registered route matches, the Rust route owner answers
the bounded `OPTIONS` preflight directly with a 204 CORS response, without
publishing it to Python or consuming the user route's finite-use budget;
ordinary `OPTIONS` requests remain routable. The two unchanged official rows
move from candidate 0/2 to final installed candidate and independent oracle
2/2. The complete Page-route file improves to 40 passed / 4 failed, and
cumulative Python execution reaches 850 passed, zero failed, seven skipped and
5,669 not run.

NODE-343 advances Python to ABI 91. A routed `Request` carries Chromium's
bounded `Fetch.networkId` as an internal key, and the Rust request-history
owner waits for the exact matching network record before returning its settled
`Response`; Python neither guesses by URL nor owns a second request registry.
The unchanged official encoded-URL row moves from an ABI 90 candidate failure
to final installed candidate and independent oracle 1/1. The complete
Page-route file improves to 38 passed / 6 failed, and cumulative Python
execution reaches 848 passed, zero failed, seven skipped and 5,671 not run.

NODE-342 advances Python to ABI 90. Python regex and predicate URL filters are
thin language-owned matchers over a bounded Rust catch-all route; Rust still
owns LIFO fallback, settlement and native retirement. The new opt-in
`route-fulfill-path` capability performs bounded async file I/O and MIME
selection in Rust. Final installed candidate/oracle pass the two unchanged
official rows 2/2, the complete Page-route file improves to 37 passed / 7
failed, and cumulative Python execution is 847 passed, zero failed, seven
skipped and 5,672 not run.

NODE-340 advances Python to ABI 89 with exact `Request.frame` identity for
ordinary network events and Page routes. Rust owns bounded native Frame
descriptors and cursor-relative availability; Python reuses the existing Page
Frame registry for Request, Response and ElementHandle facades. Final
candidate/oracle pass request-frame 6/6 plus route-frame 1/1, the complete
Page-route file improves to 35 passed / 9 failed and cumulative Python
execution is 845 passed, zero failed, seven skipped and 5,674 not run.

NODE-339 advances Python to ABI 87 and completes the Page request lifecycle
listener trio: `request`, `requestfinished` and `requestfailed`. Rust publishes
all three through one bounded request-event queue, including the failure text,
and retains Chromium's native request ID only for exact navigation
Response-to-Request correlation; Python continues to expose the portable Rust
identity. The two unchanged abort rows pass installed candidate/oracle 2/2
and repeat 10/10. The complete Page-route file improves to 34 passed / 10
failed, and cumulative Python execution is 839 passed, zero failed, seven
skipped and 5,680 not run.

NODE-338 keeps Python ABI 86 and makes Rust serialize Playwright's fixed HTTP
reason phrase into every `Fetch.fulfillRequest`, instead of inheriting the
current Chrome default. The official 422 row moves from candidate failure to
installed candidate/oracle 1/1 and repeat 5/5. The complete isolated
Page-route file is 32 passed / 12 failed with no regression. Cumulative Python
execution is 837 passed, zero failed, seven skipped and 5,682 not run.

NODE-337 keeps Python ABI 86 and promotes two additional public-header-shape
rows already fixed by NODE-336's Rust normalization. The unchanged rows move
from an ABI 84 candidate 0/2 baseline to installed candidate/oracle 2/2 and
repeat 10/10. A complete isolated Page-route rerun records 31 passed and 13
failed; the known unstable data-URL/equal-request rows remain outside the
ledger. Cumulative Python execution is 836 passed, zero failed, seven skipped
and 5,683 not run.

The optional Python package has reached ABI 93 for the current conformance
checkpoint. Its ABI 86 async/sync Page and BrowserContext
`set_extra_http_headers()` remain thin facades over the existing bounded Rust
header installation. Page headers override Context headers natively; an
implicit Page `referer` is shared by
navigation and the route request snapshot, whose CDP names are normalized to
Playwright's lowercase public shape. The three unchanged official Chromium
rows move from ABI 84 candidate 0/3 to installed candidate/oracle 3/3, and five
candidate repeats pass 15/15. The cumulative ledger is 834 passed, zero
failed, seven skipped and 5,685 not run. The all-feature Rust host passes
1,018 library tests plus 22 integration probes, with 26 plus 10 explicit live
ignores; the Python addon passes 2/2 and both strict Clippy gates pass.

NODE-335's ABI 85 checkpoint added Page and BrowserContext handler-specific
`unroute()` and `Route.fallback()` through the bounded Rust route owner. Rust
preserves Page-before-Context and newest-before-oldest traversal, rematches
dynamic URL overrides, merges fallback overrides field by field and settles
the request once. The unchanged eight-row official Chromium manifest passes
installed candidate/oracle 8/8 and five candidate repeats 40/40. The
checkpoint ledger was 831 passed, zero failed, seven skipped and 5,688 not run.
Regex/predicate matchers and `unroute_all(behavior="ignoreErrors")` remain
separate gaps.

NODE-334's ABI 84 checkpoint added finite route `times` budgets. The three
unchanged official Chromium rows moved from candidate 0/3 to installed
candidate/oracle 3/3; five more candidate runs passed 15/15. Its cumulative
ledger was 823 passed, zero failed, seven skipped and 5,696 not run.

Route glob `?` is also literal in both the Rust selector and the compiled
Chromium Fetch pattern; the unchanged official row passes final installed
candidate/oracle 1/1 and candidate repeat 5/5. The prior cumulative ledger was
820 passed, zero failed, seven skipped and 5,699 not run. The final 44-row
Page-route diagnostic records 22 pass / 22 fail; two apparent losses from
NODE332 reproduce on its unchanged wheel and remain explicit unstable
diagnostics rather than attributed regressions.

`Route.continue_()` composes bounded URL, method, header and
POST-data overrides in the Rust route owner, including same-protocol URL
validation and one typed `Fetch.continueRequest` command. The unchanged
Playwright Python 1.58.0 request-continue file passes installed candidate and
independent oracle 10/10 after the ABI 82 red lane passed 2/10. The cumulative
ledger reached 819 passed, zero failed, seven skipped and 5,700 not run out of
6,526. The complete async Page-route file improves from 18 pass / 25 fail / 1
error to 23 pass / 21 fail / 0 errors; those 21 rows remain explicit work.

Async/sync Page and BrowserContext `unroute_all()` remove
their exact Rust registration scope, update Fetch interception on every
affected session and optionally wait for already-started route actions to
settle. The navigation driver also correlates a late Chromium error-document
commit with the newest started loader so an aborted HAR navigation cannot
spuriously interrupt the next `goto`. The two unchanged Playwright Python
1.58.0 rows pass installed candidate and independent oracle 2/2, plus a
candidate 20/20 repeat lane. The cumulative ledger reached 785 passed, zero
failed, seven skipped and 5,734 not run out of 6,526. NODE-330 additionally
promotes six unchanged default-removal and in-flight-wait rows on that same
wheel: installed candidate and independent oracle both pass 6/6. The
cumulative ledger is therefore 791 passed, zero failed, seven skipped and
5,728 not run. NODE-331 then executes the complete 44-case async Page-route
Chromium file in isolated subprocesses: 18 existing behaviors pass candidate
and independent oracle 18/18 and advance the ledger to 809 passed, zero
failed, seven skipped and 5,710 not run. The preserved remaining baseline is
25 failures plus one setup/error row, grouped for subsequent implementation.

An independently selectable `har-replay` leaf parses bounded
plain HAR archives and composes with the existing Rust-owned route Fetch
driver; Python only forwards public values and invokes callbacks. Page- and
Context-scoped registrations share that owner: Page registrations take
precedence, Context registrations cover existing and future Pages/OOPIFs, and
a bounded 64-session initializer bridges requests paused before new-target
topology is published. The unchanged Playwright Python 1.58.0 Context HAR URL-
filter/ordinary-route selection passes installed candidate and independent
oracle 4/4, with separate async 20/20 and sync 20/20 stress lanes.
Regex/predicate route matchers, `fallback()`, handler-specific unroute,
`ignoreErrors`, external/ZIP HAR, update recording, the remaining Python
corpus and the complete Core JS/TS corpus remain open. This is an incremental
checkpoint, not a complete compatibility claim. See
[the Python conformance record](docs/python-official-conformance.md).

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
- [Node FFI Compatibility](docs/node-ffi-compatibility-design.md) -- composable Playwright-compatible Node capabilities and official conformance evidence
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
