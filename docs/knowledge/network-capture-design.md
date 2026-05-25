<!-- agent-updated: 2026-05-25T02:30:00Z -->

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
pwright network-listen --duration 60               # stop after 60s
pwright network-listen --filter "/api/"            # only URLs containing /api/
pwright network-listen --resource-type XHR         # only XHR requests
pwright network-listen --resource-type Fetch,XHR   # XHR and Fetch
pwright network-listen --filter "/api/" --include-body  # inline response bodies
```

Output (one JSON object per line; tracing logs and the start/stop banner
go to stderr so stdout is pure JSONL):
```jsonl
{"event":"request","reqid":"1001.2","method":"GET","url":"https://example.com/api/items","type":"XHR","post_data":null}
{"event":"response","reqid":"1001.2","status":200,"mime":"application/json","url":"https://example.com/api/items"}
{"event":"request","reqid":"1001.3","method":"POST","url":"https://example.com/api/search","type":"Fetch","post_data":"{\"q\":\"test\"}"}
{"event":"response","reqid":"1001.3","status":200,"mime":"application/json","url":"https://example.com/api/search"}
```

With `--include-body`, response events are deferred until
`Network.loadingFinished` for the same request, and the body is fetched
on the **same listener session** (the only session Chrome will release
the body to, see "Edge Cases" below) and inlined:

```jsonl
{"event":"response","reqid":"1001.2","status":200,"mime":"application/json","url":"...","body":"{\"results\":[...]}","base64_encoded":false}
```

If the body fetch fails (page navigated away, request canceled, etc.),
the event is emitted with a `body_error` field instead of `body`.

Exits on:
- Ctrl+C (SIGINT)
- `--duration` timeout
- Tab closed
- Chrome disconnected

### `pwright network-get <reqid>`

Fetches the response body for a request ID **using a fresh CDP session**.
This only works in narrow cases (e.g., when the body was captured by a
prior call on the same primary tab session and is still in this session's
cache). In modern Chrome (M110+), `Network.getResponseBody` is scoped to
the session that observed the response — so `network-get` from a separate
process generally cannot retrieve a body captured by a separate
`network-listen` process. Prefer `network-listen --include-body` for the
listen-and-fetch case.

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

Note: in modern Chrome (M110+), `Network.getResponseBody` is scoped to
the CDP session that captured the request — bodies seen by the
`network-listen` listener session are **not** retrievable from a fresh
`network-get` session, even on the same target. The `--include-body`
flag on `network-listen` is the supported way to retrieve those bodies.

### `network-listen --include-body` internals

```
1. Same setup as network-listen above (attach listener session,
   Network.enable, subscribe to events).
2. Maintain a `pending: HashMap<requestId, NetworkResponse>` of matched
   responses awaiting their body.
3. On Network.requestWillBeSent (matches filter/type):
   - Record requestId -> resource_type so later events can re-check the
     --resource-type filter (CDP only emits type on the request event).
   - Emit "request" JSONL.
4. On Network.responseReceived (matches filter/type):
   - Insert into `pending` (do NOT emit yet — body isn't fetchable until
     loadingFinished).
5. On Network.loadingFinished:
   - If requestId is in `pending`, call
     Network.getResponseBody on the listener session, then emit
     "response" JSONL with `body` + `base64_encoded` (or `body_error`).
6. On Network.loadingFailed:
   - Emit "failed" JSONL. Drop any pending entry.
```

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
  binary content (images, etc.), use `--output` to save to file. With
  `network-listen --include-body`, binary bodies are emitted as
  base64-encoded strings (`base64_encoded: true`); the consumer is
  expected to decode them. There is no size cap — extremely large bodies
  will inflate the JSONL line and the listener's memory footprint.

- **Per-session getResponseBody scoping (Chrome M110+)** —
  `Network.getResponseBody` only returns bodies that were observed by
  the calling session. The listener session sees responses via its own
  `Network.enable`; another session attached to the same target does not
  inherit that cache. This is why `network-get` from a fresh process is
  unreliable for retrieving listener-captured bodies, and why
  `--include-body` exists.
- **Session cleanup on crash** — If `network-listen` crashes, the second
  session stays attached in Chrome. It will be garbage collected when the
  tab closes. No leak risk for typical usage.

## Not in Scope

- Request modification/interception (use Fetch domain separately)
- WebSocket frame capture (different CDP domain)
- HAR export format (can be built as a post-processor over JSONL)
- Persistent capture across navigations (by design, each page load is independent)
