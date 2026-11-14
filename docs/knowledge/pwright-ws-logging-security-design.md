<!-- agent-updated: 2026-10-12T09:34:00Z -->

# Secret-safe WebSocket logging boundary

Status: local source mitigation implemented for the opt-in
`pwright-transport-ws/live-tokio` connector. Its namespaced, pinned
`pwright-tokio-tungstenite` and `pwright-tungstenite` 0.26.2 forks remove
secret-bearing dependency log records. Release transport tests pass 33/33,
including an isolated real WebSocket TRACE sentinel test and three native TLS
certificate tests; strict release Clippy, default-off dependency closure, and
workspace formatting checks pass. This is **not yet a production release
claim**: the forks are local/unpublished, the full `LiveWsConnector` WSS path
and Rustls are not covered by the TLS fixture, and unrelated CDP paths can
still use stock Tungstenite. Frozen ABI123 wheels predate this mitigation.

The optional transport now retains at most 8 KiB of an HTTP upgrade refusal
body in its selected fork. With one valid `Content-Length`, the fork reads a
bounded body across split TCP reads under the existing connection deadline;
without a valid length it keeps only the same-read tail so a keepalive
response cannot stall. The cap applies separately to the raw body buffer and
final lossy UTF-8 string, not to total handshake memory or header buffering.
Its Rust `Debug` representation redacts the body, but `Display`
deliberately includes it for the caller-visible connection error, matching
the official 401/body diagnostic. Callers must not log that error as if it
were secret-free. Both a real in-memory 401 handshake and an explicitly
selected full-MagicDNS/Tailscale-bound live connector test pass; this does not
extend the fork-path no-TRACE-leak proof to other transports. The fork-path
TRACE sentinel now also checks that a rejected HTTP upgrade body reaches the
caller but not the captured dependency logs.

Independent source review found a concrete process-wide gap: the default Node
addon still reaches stock Tungstenite through `pwright-cdp`, and
`crates/pwright-cdp/src/connection.rs:959` records its full `ws_url` at DEBUG.
The stock crate can also log incoming messages and outgoing frames at TRACE.
The selected Server feature therefore loads both the sanitized Server stack
and the unsanitized CDP stack; do not describe the entire process as
secret-safe. A separate CDP logging migration and regression gate are needed.

## Pinned exposure

The original `tokio-tungstenite` 0.26.2 and `tungstenite` 0.26.2 have these
exposure sites; the selected transport path now uses the namespaced forks.
`crates/pwright-transport-ws/src/live.rs` passes the full endpoint and
caller headers to `connect_async_with_config`. The local `WsEndpoint` and
`WsHeader` debug implementations redact values, but they do not control
dependency log records:

| Pinned dependency source | Emitted data |
| --- | --- |
| `tungstenite-0.26.2/src/handshake/client.rs:183` | TRACE prints the complete serialized HTTP upgrade request, including path/query, Authorization, Cookie and other caller headers. |
| `tungstenite-0.26.2/src/protocol/mod.rs:470,768` | TRACE prints received messages and sent frames. |
| `tungstenite-0.26.2/src/protocol/frame/mod.rs:220,249` | TRACE prints received and written frames; `frame/frame.rs:372-399` formats payload bytes as hexadecimal. |
| `tungstenite-0.26.2/src/protocol/mod.rs:719,736` | DEBUG prints Close frames, including their reasons. |
| `tungstenite-0.26.2/src/client.rs:98,132` | The synchronous client path can DEBUG-log a URI with query; current `LiveWsConnector` uses the Tokio path, but a safe fork should cover this path too. |

This is a data-leak boundary for both handshake secrets and subsequent
Playwright protocol payloads. Suppressing only the handshake call is
insufficient. `RUST_LOG`, `log::set_max_level`, and a process-wide logger
filter cannot be changed by an embeddable library without affecting unrelated
threads or applications. A custom HTTP upgrade followed by the stock
Tungstenite frame parser would still leak protocol payloads at TRACE.

## Implemented composable boundary

The local source uses a pinned, namespaced no-secret-logging variant of **both** Tungstenite
0.26.2 and Tokio-Tungstenite 0.26.2 for the optional
`pwright-transport-ws/live-tokio` connector. The Tokio variant must depend on
the namespaced Tungstenite variant; changing only one crate would leave the
stock implementation on the live path. Remove payload-bearing log expressions
or replace them with safe metadata such as opcode, byte count and status.
Never log endpoint path/query, header values, message or frame contents, or
Close reasons. Preserve the upstream WebSocket frame parser, size limits and
`native-tls` handshake/certificate-validation path; the bounded rejected-HTTP
body reader is the separate, documented handshake change. No `danger_*` TLS option
or verification bypass is acceptable. Keep the default, runtime-free
transport feature free of either live connector, and leave unrelated CDP
users on their existing dependency unless separately migrated.

A workspace-root `[patch.crates-io]` is not a sufficient published-library
solution: Cargo patches are honored by the consuming root, not transitively
by downstream users of `pwright-transport-ws`. It would also replace the
dependency for unrelated workspace consumers. The current path dependencies
pin `0.26.2-pwright.1` and retain original licenses, crates.io SHA-256
provenance, and reviewable semantic-source diffs in each fork's `UPSTREAM.md`.
Upstream sources were mechanically rustfmt'd for workspace CI; the provenance
notes explain how to normalize upstream before comparing. The forks must be
published before a downstream release can consume them. A binary also using
CDP may contain duplicate WebSocket code; that cost remains unmeasured.

## Verification state before enabling sensitive headers

1. **Passed at the fork path.** In a dedicated test process with a TRACE logger, perform an actual WS
   upgrade and bidirectional exchange using distinct sentinel strings in a
   URL query, Authorization/Cookie/custom headers, outbound and inbound
   messages, and a Close reason. Assert the fixture received the exact
   request and payloads, while captured logs contain none of the sentinels.
   Do not make the product set a global logger; the isolated test process may
   install one solely to inspect its own logs.
2. **Passed at the fork path, not full connector E2E.** Exercise `wss` with a trusted test CA and matching hostname, and verify
   unknown-issuer and hostname-mismatch rejection. The production connector
   must keep normal certificate verification and SNI; no insecure fallback.
3. **Source/dependency gates passed locally; packaging open.** Review every log call in both pinned variants on each upgrade. Gate the
   source diff, TRACE sentinel test, feature/dependency closure, release
   tests and Node/Python packaging. The selected connector must not bring
   the stock payload-logging Tungstenite into its own type path.
4. **Still open.** Only then run unchanged, source-pinned official `BrowserType.connect`
   close and header cases in isolated candidate/oracle lanes. A passing
   failure-path test does not prove a successful remote Browser connection.
