<!-- agent-updated: 2026-10-12T08:41:30Z -->

# Pwright Tokio-Tungstenite logging fork

This is the `tokio-tungstenite` 0.26.2 crates.io source package, checksum
`7a9daff607c6d2bf6c16fd681ccb7eecc83e4e2cdc1ca067ffaadfca5de7f084`.
The upstream project is <https://github.com/snapview/tokio-tungstenite> and
its original MIT license file is retained here.

The package was renamed to `pwright-tokio-tungstenite` and pinned to
`0.26.2-pwright.1`. Its library name remains `tokio_tungstenite` for source
compatibility. The `tungstenite` dependency key resolves exclusively to the
matching `pwright-tungstenite` fork. Source changes remove untrusted error
details and native root-loader error details from `log` records; the async
adapter, native TLS connector, SNI, certificate verification, and frame
behavior are otherwise unchanged. Upstream examples and archive metadata are
omitted. Source was mechanically formatted with `rustfmt 1.9.0-stable` to
satisfy the host workspace's formatting gate.

To review the semantic source diff, verify the upstream `.crate` archive
against the checksum above, extract its `src/` into a temporary directory,
run `rg --files -0 -g '*.rs' <temporary-src> | xargs -0 rustfmt --edition 2018`,
then `diff -ru <temporary-src> src`. The only differing Rust sources are
`lib.rs` and `tls.rs`. The manifest also changes package identity, library
name declaration, the `tungstenite` package alias, packaging scope, and
omits example targets.

Do not upgrade this snapshot without repeating the complete dependency/log
audit and isolated TRACE sentinel/TLS tests.
