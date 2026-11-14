<!-- agent-updated: 2026-10-12T09:33:00Z -->

# Pwright Tungstenite logging fork

This is the `tungstenite` 0.26.2 crates.io source package, checksum
`4793cb5e56680ecbb1d843515b23b6de9a75eb04b66643e256a396d43be33c13`.
The upstream project is <https://github.com/snapview/tungstenite-rs> and its
original MIT and Apache-2.0 license files are retained here.

The package was renamed to `pwright-tungstenite` and pinned to
`0.26.2-pwright.1`. Its library name remains `tungstenite` for source
compatibility. The source changes remove sensitive values from `log` records:
serialized upgrade requests, URIs, message and frame payloads, close reasons,
and native root-loader error details. A separate bounded client-handshake
change retains at most 8 KiB of a rejected HTTP upgrade response body when
one valid `Content-Length` is present, including bytes arriving after the
headers. Without a valid length it retains only the already-buffered tail
and does not wait on a keepalive connection. A declared body that stalls
remains subject to the caller's connection deadline. Other header parsing,
frame behavior, protocol limits, and TLS selection are unchanged.
The original examples, benches, and archive metadata are omitted. Source was
mechanically formatted with `rustfmt 1.9.0-stable` to satisfy the host
workspace's formatting gate.

To review the semantic source diff, verify the upstream `.crate` archive
against the checksum above, extract its `src/` into a temporary directory,
run `rg --files -0 -g '*.rs' <temporary-src> | xargs -0 rustfmt --edition 2021`,
then `diff -ru <temporary-src> src`. The only differing Rust sources are
`client.rs`, `handshake/client.rs`, `handshake/machine.rs`, `protocol/frame/mod.rs`,
`protocol/mod.rs`, and `tls.rs`. The manifest also changes package identity,
library name declaration, packaging scope, and omits example/bench targets.

Do not upgrade this snapshot by copying a newer release without repeating a
complete log-call audit and the isolated TRACE sentinel/TLS tests.
