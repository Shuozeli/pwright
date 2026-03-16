# Design: Network Capture CLI

## Problem

The CLI is stateless — each invocation connects, acts, disconnects. Network
events are transient CDP events that fire on the WebSocket. If nobody is
listening when a request happens, it's gone. Users need to capture network
traffic while interacting with a site across multiple CLI invocations.

## Solution: Second CDP Session Listener

CDP supports multiple sessions attached to the same target. Each session
receives its own copy of domain events. `pwright network-listen` creates a
second session on the active tab, enables the Network domain on it, and
streams events as JSONL to stdout until Ctrl+C or `--duration` expires.

The user's interaction commands (`click`, `fill`, `press`) use the primary
session. The listener uses a separate session. Both are attached to the
same tab — no interference.

```
Chrome Tab (target_id: ABC123)
  |
  +-- Session 1 (from pwright click/fill/press)
  |     Used for DOM, Input, Runtime commands
  |
  +-- Session 2 (from pwright network-listen)
        Network.enable -> streams events to stdout
```

## Commands

### `pwright network-listen`

Long-running command. Attaches a second CDP session to the active tab,
enables Network domain, prints request/response events as JSONL to stdout.

```bash
# Terminal 1: start listener
pwright network-listen
pwright network-listen --duration 60          # stop after 60s
pwright network-listen --filter "/api/"       # only URLs containing /api/
pwright network-listen --type XHR             # only XHR requests
pwright network-listen --type Fetch,XHR       # XHR and Fetch
```

Output (one JSON object per line):
```jsonl
{"event":"request","reqid":"1001.2","method":"GET","url":"https://example.com/api/items","type":"XHR","timestamp":1710000000.123}
{"event":"response","reqid":"1001.2","status":200,"mime":"application/json","size":1234,"url":"https://example.com/api/items","timestamp":1710000000.456}
{"event":"request","reqid":"1001.3","method":"POST","url":"https://example.com/api/search","type":"Fetch","post_data":"{\"q\":\"test\"}","timestamp":1710000001.789}
{"event":"response","reqid":"1001.3","status":200,"mime":"application/json","size":567,"url":"https://example.com/api/search","timestamp":1710000002.012}
```

Exits on:
- Ctrl+C (SIGINT)
- `--duration` timeout
- Tab closed
- Chrome disconnected

### `pwright network-get <reqid>`

Fetches the response body for a request ID from the LIVE Chrome session.
Chrome keeps response bodies in memory for the current page load. Works
as long as the page hasn't navigated away.

```bash
pwright network-get 1001.2                   # print body to stdout
pwright network-get 1001.2 --output data.json   # save to file
```

The `reqid` comes from the `network-listen` output. It's Chrome's internal
`Network.requestId` which is stable for the lifetime of the page.

### `pwright network-list` (bonus, retroactive)

Quick one-shot query using `performance.getEntriesByType('resource')`.
Works after the fact without needing a listener running. Returns less
detail (no headers, no bodies, no requestIds) but useful for quick checks.

```bash
pwright network-list                         # all resources
pwright network-list --filter ".js"          # only JS files
```

Output:
```
  #  Type       Status  Size     Duration  URL
  1  script     200     45.2KB   120ms     https://example.com/app.js
  2  fetch      200     1.2KB    45ms      https://example.com/api/data
  3  img        200     234KB    89ms      https://example.com/logo.png
```

## Implementation

### `network-listen` internals

```
1. Load CLI state (ws_url, target_id)
2. Connect to Chrome via WebSocket
3. Attach NEW session to target_id -> session_id_2
4. Send Network.enable on session_id_2
5. Subscribe to events on session_id_2
6. Loop:
   a. Receive CDP event
   b. If Network.requestWillBeSent -> print request JSONL
   c. If Network.responseReceived -> print response JSONL
   d. If Network.loadingFailed -> print error JSONL
   e. If Ctrl+C or timeout -> break
7. Send Network.disable on session_id_2
8. Detach session_id_2
```

Key: step 3 creates a SEPARATE session from the one stored in CLI state.
This avoids interfering with the user's primary session.

### `network-get` internals

```
1. Load CLI state (ws_url, active_tab)
2. Connect and resolve tab (uses primary session)
3. Call Network.getResponseBody({ requestId }) on primary session
   (Network domain may need to be enabled first)
4. Print body to stdout or save to file
5. If base64Encoded, decode before saving
```

Note: `Network.getResponseBody` works on the primary session even if
Network was enabled on the listener session, because Chrome stores
response bodies per-target, not per-session.

### `network-list` internals

```
1. Connect and resolve tab
2. Call page.evaluate("JSON.stringify(performance.getEntriesByType('resource'))")
3. Parse and format as table
```

No CDP Network domain needed. Pure JS query.

## File changes

```
crates/pwright-cli/src/main.rs        Add NetworkListen, NetworkList, NetworkGet commands
crates/pwright-cli/src/commands.rs    Add network_listen, network_list, network_get handlers
crates/pwright-cli/Cargo.toml         Add ctrlc or tokio signal for Ctrl+C handling
```

No changes needed in pwright-cdp or pwright-bridge — all the CDP primitives
(`target_attach`, `network_enable`, `subscribe_events`, `network_get_response_body`)
already exist.

## Workflow Example

```bash
# 1. Open a site
pwright open https://example.com

# 2. Start network listener in background (or separate terminal)
pwright network-listen --filter "/api/" > traffic.jsonl &

# 3. Interact with the site
pwright snapshot
pwright click e5
pwright fill e3 "search query"
pwright press Enter

# 4. Check captured traffic
cat traffic.jsonl
# {"event":"request","reqid":"1001.5","method":"POST","url":"https://example.com/api/search",...}
# {"event":"response","reqid":"1001.5","status":200,"mime":"application/json",...}

# 5. Grab the response body
pwright network-get 1001.5
# {"results": [{"title": "Result 1", ...}, ...]}

# 6. Stop listener
kill %1
```

## Edge Cases

- **Page navigation resets requestIds** — Chrome clears response body cache
  on navigation. `network-get` only works for the current page load.
- **Listener starts after page load** — Requests that happened before
  `network-listen` started are not captured. Start listener before navigating.
- **Multiple listeners** — Multiple `network-listen` processes can run
  simultaneously (each gets its own session). No conflict.
- **Large response bodies** — `network-get` returns the full body. For
  binary content (images, etc.), use `--output` to save to file.
- **Session cleanup on crash** — If `network-listen` crashes, the second
  session stays attached in Chrome. It will be garbage collected when the
  tab closes. No leak risk for typical usage.

## Not in Scope

- Request modification/interception (use Fetch domain separately)
- WebSocket frame capture (different CDP domain)
- HAR export format (can be built as a post-processor over JSONL)
- Persistent capture across navigations (by design, each page load is independent)
