<!-- agent-updated: 2026-09-10T20:16:00Z -->

# Pwright file-input content ownership

Optional Rust leaf depending only on core and the allocation-free file-input
model. Copies actual byte slices or reads finite opened readers into a
non-cloneable, ledger-admitted batch. No Node, CDP, executor or browser action.

See the [ownership contract, tests and remaining tasks](../../docs/knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

Run `cargo test --release --locked -p pwright-file-input-source`.
Synchronous reads belong in an explicitly admitted embedding executor, not
on a Node/event-loop thread. This is not yet public setInputFiles support.
