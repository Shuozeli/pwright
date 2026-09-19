<!-- agent-updated: 2026-09-10T20:47:00Z -->

# Admitted file-input execution

Optional Tokio adapter for Rust filesystem preparation. Inject the runtime
Handle, filesystem/MIME services and explicit queue/source resource plans.
Submission admits task and byte capacity before copying paths or spawning work.

Queued, running and completed-unconsumed work occupies bounded slots. Cancelling
a waiter does not release a still-running reader's buffers or descriptors.
Close admission, consume/drop Jobs and await close_and_drain before declaring
work drained. OS calls are not forcibly interruptible; delivered results retain
their separate source leases.

Run `cargo test --release --locked -p pwright-file-input-tokio`.
See the [contract, evidence and remaining Node/browser work](../../docs/knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).
