<!-- agent-updated: 2026-07-28T08:06:00Z -->

# Raw URL Capture Design

## Purpose

`pwright capture-url` saves the exact body of an HTTP(S) response acquired
through Chrome. It is for browser-bound acquisition where page JavaScript is
the wrong transport:

- cross-origin issuer documents can reject JavaScript `fetch()` through CORS;
- direct PDF navigation can expose Chrome's small PDF embedder HTML through
  `Network.getResponseBody`, rather than the original PDF;
- downstream processors need status, headers, final URL, redirects, an exact
  byte count, and a hard size limit.

This command does not decide whether a URL is trustworthy. Discovery,
ownership, provenance, redistribution, and redirect policy stay with the
caller.

## Interface

```text
pwright capture-url <url>
  --dest <path>
  [--context-url <url>]
  [--max-bytes <bytes>]
  [--timeout <milliseconds>]
  [--json]
```

Both target and context URLs must use HTTP(S). The default decoded-body limit
is 50 MiB. The destination is created only when capture succeeds, and an
existing file is never overwritten.

When `--context-url` is present, Pwright first loads that page in the same tab
and waits for DOM readiness. This preserves the browser profile's cookies and
establishes issuer/referrer context before target navigation.

## CDP Sequence

```text
connect to existing Chrome
  -> recover any same-endpoint leased ephemeral target
  -> create target and persist its exact target ID
  -> optionally navigate to context URL
  -> Fetch.enable(pattern="*", stage=Response)
  -> Page.navigate(target URL)
  -> Fetch.requestPaused(main Document)
       -> for redirect: record hop, continueRequest, await next same-frame Document
       -> for final: validate Content-Length
          -> Fetch.getResponseBody
          -> continueRequest
          -> decode and enforce exact byte limit
  -> create destination without overwrite
       [durable capture commit point]
  -> Fetch.disable
  -> close and verify the exact leased target
  -> remove lease
```

Response-stage interception occurs after Chrome receives response headers but
before the renderer or PDF viewer consumes the body. The event's frame and
resource type identify the main document. Redirects keep the claimed main
frame, so unrelated subresources and other tabs cannot satisfy the capture.
Because Chrome returns binary bodies as base64 inside one CDP WebSocket
message, Pwright permits bounded 128 MiB WebSocket frames/messages. This is
large enough for the default 50 MiB decoded-body limit plus base64 and JSON
overhead. The prior Tungstenite 16 MiB frame default closed the connection on
a live 15,084,952-byte issuer deck.

## Result

With `--json`, stdout contains one object:

```json
{
  "requested_url": "https://cdn.example/q2.pdf",
  "context_url": "https://investors.example/results",
  "final_url": "https://cdn.example/q2.pdf",
  "status": 200,
  "status_text": "OK",
  "mime_type": "application/pdf",
  "response_headers": {"content-length": "145284"},
  "redirect_chain": [],
  "network_request_id": null,
  "bytes_written": 145284,
  "destination": "/private/run/q2.pdf",
  "transport": "cdp_fetch_response_body"
}
```

Tracing and diagnostics remain on stderr so automation can parse stdout.

## Failure and Recovery

- A declared or decoded body over `--max-bytes` fails without retaining a
  destination.
- A pre-existing destination fails closed.
- CDP disconnection, command timeout, malformed response events, and body
  decoding errors are explicit failures.
- After the exact body has been durably written, Fetch-disable or tab-close
  failures are structured warnings rather than acquisition failures. A failed
  close retains the exact target lease for later recovery.
- WebSocket frame/message limits remain bounded and exceed the maximum
  base64-encoded default capture; oversized decoded content is still rejected
  by `--max-bytes`.
- The target ID is leased before attach. If the process dies, the next
  same-endpoint Pwright invocation recovers that exact target. Pwright never
  closes unrelated pre-existing tabs.
- The caller should classify endpoint failures as infrastructure state and
  avoid recording them as source-quality failures.
