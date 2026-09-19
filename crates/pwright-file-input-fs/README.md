<!-- agent-updated: 2026-09-17T11:56:08Z -->

NODE-176: `PinnedMime::lookup_extension` supplies exact case-sensitive table
lookup after a language adapter has normalized a key. `MAX_EXTENSION_BYTES`
is derived from the pinned table for bounded FFI scratch; whole filename lookup
is unchanged. The leaf remains independent of Node and CDP.
[Canonical contract and current evidence](../../docs/knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

# Pwright file-input filesystem preparation

Optional injected filesystem/MIME services with admitted path and descriptor
ownership. Composes the model and content-source leaves; no Node, CDP or executor.
The production file-opening provider currently requires Unix. Native directory
routes retain paths; separate-browser ordinary files become bounded payloads.

See the [design, evidence and open work](../../docs/knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

Run `cargo test --release --locked -p pwright-file-input-fs`.
With the pinned oracle installed under tests/, run
`node scripts/check-file-input-mime-oracle.mjs` from the workspace root.

Pinned MIME data derives from mime@3.0.0 bundled with Playwright 1.58.2.
See [MIME-LICENSE](MIME-LICENSE). The lookup table is optional static data, not
a runtime dependency on Playwright. Do not run synchronous filesystem preparation
on the Node/event-loop thread.
