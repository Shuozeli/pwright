<!-- agent-updated: 2026-09-05T20:53:09Z -->

# Pwright Handle lifecycle checkpoint

## Decision boundary

This checkpoint defines how an already-created JavaScript handle is identified
and explicitly released. It does not add `evaluateHandle`, create browser-side
objects, or change the existing value-only evaluation capability.

Keeping those operations separate is important for composition: a user who
only installs value evaluation retains no live JavaScript objects and needs no
Handle lifecycle machinery.

## Portable object model

The core-only browser vocabulary now contains two additional object kinds:

- `pwright.browser.js_handle`, tag 5, exposed in Rust as `JsHandle`;
- `pwright.browser.element_handle`, tag 6, exposed as `ElementHandle`.

Both kinds have `Frame` as their allowed parent. An ElementHandle is distinct
at the protocol-object level even though Playwright models it as a specialized
JavaScript handle. This gives dispatch and capability admission an exact kind
without depending on CDP terminology.

The client can materialize either typed Handle from a Frame initializer. All
clones share the same bounded liveness token. Runtime child-first Dispose
notifications make those clones stale; the client never recomputes the
ownership subtree itself.

## Optional capability and wire contract

`pwright.handle_lifecycle` version 1.0 owns one command:

| Command | Method | Targets | Controls | Result |
|---|---:|---|---|---|
| `handle.dispose` | `0x0008_0001` | JsHandle, ElementHandle | none | unit |

The schema is canonical input for generated capability metadata, binary types,
the protocol reference, and the typed concurrent server handler. The codec
rejects attachments, mismatched inner/outer identities, wrong routes or method
tags, and timeout/cancellation metadata. Errors retain their portable failure
kind plus optional not-owned, backend, or compound-cleanup detail.

The opt-in client extension exposes async `dispose` on both Handle types. It is
idempotent when the local Handle is already stale or the server reports a
racing `TargetClosed`. Backend, transport, protocol, and resource failures are
not hidden. Rust `Drop` sends no protocol message and never blocks.

## Chromium backend leaf

The injected `CdpClient` boundary now includes `runtime_release_object`; the
real session implementation serializes it as `Runtime.releaseObject`. The fake
records the same method and supports deterministic failure injection.

`CdpHandleDisposeService` owns a compact, non-empty native object ID whose byte
length is checked at construction. The service is bound to the exact CDP
session passed by its caller, sends one release request, performs no hidden
retry, and maps the native error without erasing its failure classification.
The Chromium backend manifest advertises this leaf as a native implementation
of `handle.dispose`.

## Verified behavior

Release-profile tests prove:

- object kind IDs/tags are unique and both Handle kinds allow Frame parents;
- both client Handle types dispatch the same no-controls command;
- child-first family invalidation makes both dispose calls local no-ops;
- a racing `TargetClosed` is success while a backend failure remains visible;
- command/result round trips preserve route identity and structured errors;
- timeout metadata and tampered call IDs are rejected;
- the backend issues only `Runtime.releaseObject`, enforces native-ID bounds,
  and preserves native failure visibility;
- the backend manifest includes the optional Handle lifecycle feature.

## Relationship to Handle creation

The independently selectable
`pwright-composition-chromium-handle-lifecycle` now makes lifecycle a complete
server installation unit without importing Handle evaluation. It owns the
bounded portable route registry and generated 1.0 handler. Producers reserve a
route plus its maximum `RemoteHandle`/retained-byte budget before native I/O,
then publish exact bytes and an opaque typed payload. The only native behavior
injected into lifecycle is an object-local disposer; the first-party CDP form
is the isolated `CdpHandleDisposeService`.

Explicit disposal reserves caller-owned continuation capacity before native
release, preserves both runtime and route on visible backend failure, and
commits runtime plus route retirement after success. A started future that is
dropped transfers the exact transaction to the caller's bounded queue. A
primitive Handle has no disposer and performs no CDP request. Batch admission
reserves all routes before native creation and publishes every route under one
registry lock around an injected runtime commit; commit failure publishes none
and releases every reservation. Twelve AAA tests cover these transactions and a
dedicated Chrome 152 proof releases one real
RemoteObject while leaving the external browser alive. Its standalone normal
Cargo closure is exactly 15 Pwright packages and excludes Handle evaluation,
value evaluation, topology, Page/Context capabilities, aggregate crates,
clients, providers, launchers, installers, Tokio, Node.js, and browser bytes.

The dependent [Handle evaluation checkpoint](pwright-handle-evaluation.md) now
has an independent 1.2 composition consumer. It reuses this exact registry for
all six creation/value/property commands, including atomic property batches and
abandoned-result disposal, while owning a separate bounded Frame route table.
Its dedicated-Chrome proof reaches a real execution world without making this
lifecycle leaf or value-only evaluation depend on Handle evaluation. The
installation additionally exposes its raw disposer so the optional ordered
remote-delivery plugin can own the complete Create/Result/Dispose fence without
forcing that policy into local callers.

Connection teardown now has an explicit two-phase leaf transaction. Preparing
locks and snapshots the exact live routes; dropping it preserves them, while
commit retires that snapshot and its leases. The aggregate
`ChromiumHandleAwareRuntime` is generic over this retirement owner, so it can
wrap either the compatibility registry or the independent lifecycle leaf
without deleting routes before the inner runtime accepts transport close.
The same registry now prepares parent-subtree and topology-owner invalidation from plain
Frame/Page IDs, emits the exact Handle `Dispose` batch, incorporates any
additional runtime removals, and commits under one lock. This keeps topology
types out of the lifecycle leaf while removing the last retirement behavior
that was structurally available only in the aggregate registry.

Production route accounting and explicit Page/owned-Context retirement are now
implemented in the dependent checkpoint. The same route transaction now
handles main-world replacement, topology Frame disposal, Page crash, reconnect,
and an opt-in Handle-aware connection-close target. Direct Handle creation now
transfers a dropped started operation to bounded continuation and disposes any
orphan result. The production profile now installs this lifecycle capability
independently, and its negotiated remote handler fences native disposal behind
the shared ordered Dispose stream. Handle evaluation may depend on it, but
value evaluation does not. The remote evaluation handler carries the same
pre-reserved cleanup ownership through Result delivery, so cancellation after
Create also disposes through this ordered service. The complete-capability
recipe now makes `ChromiumHandleAwareRuntime` over the lifecycle leaf its
runtime type, delegates the shared control source through it, and therefore
cannot omit route retirement on transport close. Its Page and owned-Context
close services and topology transaction use the same leaf owner; the older
aggregate registry remains only for compatibility constructors.
