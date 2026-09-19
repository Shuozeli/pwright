<!-- agent-updated: 2026-09-19T17:30:22Z -->

# Chromium file-input payload backend

Optional exact-element buffered-file mutation. Composes the group-owned Locator
resolution reference and prepared source owner; no filesystem, executor or Node
dependency is selected by this backend.

`PayloadTarget::new(client, world, object_id, group)` also accepts an exact
borrowed receiver independently of Locator resolution; identifiers must be
nonempty and at most 1024 UTF-8 bytes. `apply_on_target(target, payloads)` reuses
all policy, encoding, scratch admission and transport selection. It performs no
selector retry, ownership admission, cross-world adoption or native release.
The caller must retain separately admitted object/world/group owners through
settlement and cleanup. Do not destroy a persistent Handle by releasing its
temporary upload group. The original `apply(&ResolvedElement, payloads)` API
delegates to this shared path and remains compatible.

The owned upload continuation still accepts the Locator-owned element; a
borrowed target does not itself provide cancellation-safe continuation. See
[Node Handle integration and retained-node evidence](../../docs/knowledge/pwright-node-handles.md).

For exceptional outgoing payloads, select the optional `large-commands`
feature, explicitly configure the connection's LargeCommandProfile and select
`PayloadProfile::with_large_commands`. This adds no package dependency and
does not widen ordinary commands or inbound messages. Raw source, backend
encoding and transport backing each require their own resource plan.

Run selected checks with
`cargo test --release --locked -p pwright-backend-cdp-file-input --features large-commands`.
The separate ignored `live_large_payload` test is memory-heavy and must be
invoked explicitly after dedicated Chrome preflight. See the
[large transfer contract](../../docs/knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

Run `cargo test --release --locked -p pwright-backend-cdp-file-input`.
The ignored live integration test requires dedicated Chrome preflight and the
verified loopback PWRIGHT_CDP_WS_URL. See the
[contract, browser evidence and unfinished public integration](../../docs/knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

Callers own started-command continuation and source-group cleanup, including
exceptions from the explicit group-owned by-value CDP call. Native path
and directory upload are separate unfinished branches, not silently implemented
as payload transfer.

The mutation uses explicit protocol settlement, not the ordinary command timer.
Select pwright-composition-chromium-file-input to retain started operations after
a public waiter is dropped. The caller must still drive its cleanup executor;
transport loss is not group destruction proof.

## NODE-178: selected retained-input prevalidation

The default-off `retained-preflight` feature adds
`CdpFileInputPreflightService`, `RetainedPreflightProfile` and one-shot
`PreflightAdmission`. It retargets the caller-owned utility receiver and checks
INPUT/multiple/directory policy, returning the exact retargeted native ID.
This is not a selector requery, payload read, byte decoder or upload mutation.
It adds no enabled/visible/`isConnected` check or eager `type=file` rejection.

The explicit grouped CDP acquisition preserves its raw result and exception
envelope through settlement, with `returnByValue=false`, `awaitPromise=false`
and `userGesture=true`. Separate admission covers two possible derived native
wrappers and bounded command/result/diagnostic backing. A raw caller must keep
that admission and the original group owner through acquisition and group
cleanup; dropping a Rust value alone is not release proof. Prefer the selected
composition owner for cancellation-safe lifetime management.

Transport failures preserve typed CDP kinds and protocol codes with fresh
bounded diagnostics; opaque JSON/recursive Compound errors normalize to Other.
Neither error text nor transport termination proves native group destruction.

Run selected offline checks with
`CARGO_INCREMENTAL=0 cargo test --release --locked -p pwright-backend-cdp-file-input --features retained-preflight --lib`.
No Node preparation phase or public API selects this backend yet, and this
checkpoint claims no browser pass. Marking, raw carrier projection,
instrumentation/deadline integration, late utility serialization and subsequent
mutation remain separate work. See the
[canonical NODE-178 contract and verification](../../docs/knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

## NODE-182: selected persistent utility-world target marking

The default-off `retained-target-marking` feature adds
`CdpTargetMarkingService`, `TargetMarkingProfile`, `MarkerWorld`, unique
`MarkerState` and one-shot `MarkerAdmission`. It also selects the Locator
backend's `retained-reference` ownership check. It does not enable marking in
the host, addon, Node preparation driver or public `ElementHandle.setInputFiles`.

Creation synchronously admits storage before returning its future, then creates
a private plain `{ state: undefined }` in a uniquely named persistent group.
Group names use the injected never-reused `ObjectIdSource`; installations on one
native session must share that source. `MarkerWorld` retains the exact client,
bounded unique utility-context ID and optional caller-admitted destruction
receipt. State is not stored in the temporary adopted target group or reset at
the end of each marking call. A destruction receipt with only one release-fence
waiter must not be shared by concurrently releasing groups; callers supply
independent receipts or a genuinely multi-waiter proof.

Marking accepts only a settlement-owned `ResolvedElement` from the same client
and unique world. It dispatches the original adopted node, not a selector
requery or retargeted input. The grouped raw CDP call uses
`returnByValue=true`, `awaitPromise=true`, `userGesture=true`, preserves raw
exceptions and requires an undefined result. Its possible result/exception
wrappers belong to the adopted target group, never the persistent state group.
There is no early connected, enabled, visible or INPUT/type check. Empty call
IDs perform no marking I/O; nonempty IDs are bounded to 128 UTF-8 bytes.

For selected state capacity `S` and call capacity `C`, both positive, the backend
declares `RemoteHandle = 2 * (S + C)` and
`BackendStateBytes = 512 + 20_020 * S + 22_930 * C`, with one scope and
`1 + S + C` lease slots. The 512-byte shared administration survives the service
while any admission remains. Separate counters prevent state/call capacity from
borrowing each other's slots. State/init and per-call allowances each cover two
native wrappers, bounded diagnostics and explicitly checked storage. Transport
buffers, caller-supplied route/proof backing and browser DOM graphs retained by
the marker Set have separate ownership; these are not process-RSS bounds.

`MarkerState::release` retains its allowance until group-release acknowledgement
or authoritative exact-world/session retirement. Error text and transport loss
are not retirement proof. A call admission remains held through adopted-group
cleanup; `complete_cleanup()` is only a receipt supplied after that cleanup
succeeds, not a native release itself. Failed or abandoned cleanup poisons
further admission before resources return. Raw Drop cannot perform asynchronous
cleanup: use the selected composition's pre-admitted continuations and drive its
cleanup executor. Dropping an unpolled creation creates no native object.

Run selected offline checks with
`CARGO_INCREMENTAL=0 cargo test --release --locked -p pwright-backend-cdp-file-input --features retained-target-marking --lib`.
The first selected backend run passed 37 tests, including 21 marker tests.
Final offline and isolated owned-world browser evidence is recorded in the
[canonical NODE-182 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).
This primitive supplies no production world registry, connection-wide call-ID
issuance, original action deadline, instrumentation or same-ticket Node native
handoff. Their ordering before raw carrier reads and the complete upload path
remain integration work; an offline ownership test is not browser proof.

## NODE-187: selected same-owner prevalidation

The new host/addon `retained-target-prevalidation` profile reaches this existing
`retained-preflight` backend only through the composition owner. The backend API
and package graph are unchanged: it still accepts one exact group-owned target
and native Boolean flags, then returns the retargeted INPUT ID in that same
group. For host capacity eight, its independent declaration is 16 RemoteHandle
units and 156,368 BackendStateBytes total. Marker-call and adopted-group lifetime
remain composition responsibilities; the backend never releases or re-adopts
them.

This integration is read-only and internal. It adds no instrumentation, payload
serialization, mutation or public API. Exact same-ticket/cleanup and browser
evidence belongs in the
[canonical NODE-187 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

Rust code is MIT; the DOM helper adapts Playwright 1.58.2 under Apache-2.0.
See [LICENSE.playwright](LICENSE.playwright).
