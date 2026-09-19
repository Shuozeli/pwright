<!-- agent-updated: 2026-09-19T17:30:22Z -->

# Owned Chromium file-input operations

An optional single-use composition of prepared payloads, an exact resolved
element, its retained route and caller-driven cleanup. It neither resolves
selectors nor starts a runtime, reads files or implements Node APIs.

`prepare_retaining` additionally consumes a caller-owned source-admission guard
and retains it through native settlement and group cleanup after waiter drop.
Its inline storage is included in the declared owner-byte limit; caller-owned
external allocations still need their own budget. The original `prepare` API
is unchanged and delegates with no extra retained object. The selected Node host
uses this to bound both prepared and already-submitted source batches.

Declare UploadProfile and source/backend/resolution/executor budgets separately.
One live operation needs its own cleanup reservation in addition to the resolved
element's existing reservation. Owner byte admission checks the concrete async
state and wrapper sizes before boxing. Embedding/allocator overhead is separate.

Awaiting publishes only after group release. Dropping before first poll sends no
mutation; dropping afterwards transfers the operation and all its input owners
to the reserved executor. Drive that executor through settlement and cleanup
before shutdown. Cleanup failure fails admission closed; it is not native
destruction proof. A broken executor that discards jobs cannot promise cleanup.

Run `cargo test --release --locked -p pwright-composition-chromium-file-input`.
The ignored live_continuation test requires dedicated Chrome/Pwright preflight
and the verified loopback PWRIGHT_CDP_WS_URL. See the
[contract and remaining public gates](../../docs/knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

## NODE-178: selected retained-input prevalidation

The default-off `retained-preflight` feature adds
`FileInputPreflightOperationService` and `OwnedValidatedFileInput`. It also
selects the resolver's `retained-reference` ownership path. The read-only
`ResolvedElement::uses_settlement_release()` check rejects an ordinary timed
group owner before any derived query; it does not upgrade that owner's policy.

Unlike payload mutation, successful prevalidation returns a parked owner of
the exact retargeted input. Its original adopted element, route owner, separate
two-RemoteHandle admission and extra cleanup reservation remain owned through
explicit release or ready-owner Drop until group release settles. Capacity is
not returned merely because the query finished. Dropping a started query
continues only acquisition and cleanup, never payload processing or mutation.
Unpolled drop performs no prevalidation query but still releases the existing
adopted group. Failed cleanup closes admission; an executor that discards or
times out its own jobs cannot establish native reclamation.

Run the selected offline suite with
`CARGO_INCREMENTAL=0 cargo test --release --locked -p pwright-composition-chromium-file-input --features retained-preflight --lib`.
The NODE-178 run recorded **32 passed**, including 15 new ownership tests.
There is no new Node integration or browser pass: target marking, raw carrier
projection, instrumentation/deadline integration, late utility serialization
and public retained upload remain separate gates. See the
[canonical NODE-178 contract and verification](../../docs/knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

## NODE-182: persistent private utility-world target marking

The independent, default-off `retained-target-marking` feature adds
`TargetMarkingOperationService`, `OwnedTargetMarker` and `OwnedMarkedElement`.
It selects the backend marker primitive and settlement-owned retained resolver;
it does not select Node host/addon behavior or the prevalidation feature.

Use separate `UploadProfile` scopes and capacities for persistent world owners
and per-call owners, plus the backend's independent state/call native budgets.
The service checks actual future, release and wrapper storage before boxing.
Caller-owned route/snapshot storage and executor capacity are separately admitted.
Each persistent state and each admitted call reserves its cleanup continuation
before native work starts. The persistent state owns a private remote group,
not a global property. The caller must retain exactly one chosen owner for the
intended utility-world lifetime; this crate does not supply a world registry,
session shutdown hook or world-replacement policy.

Marking borrows that private state and consumes the original adopted target.
Exact service, client, unique utility world and settlement cleanup mode are
checked; marking neither queries selectors nor retargets a label to its input.
The call's result/exception wrappers belong to the temporary target group, not
the persistent group. `OwnedMarkedElement` keeps the additional two-native-unit
admission through target-group cleanup; it deliberately has no conversion that
could separate the target from that allowance early. Chaining this owner into
the later prevalidation/native pipeline remains a separate integration gate.

Normal target cleanup and a canceled mark leave the persistent state intact.
A dropped started waiter continues that same native request through settlement
and target cleanup. Dropping the persistent owner defers its group release until
all in-flight marks relinquish it; explicit `release()` returns the same owner
unchanged while busy. Ready Drop and dropped explicit-release futures retain
their original reservations and resource leases through group acknowledgement
or authoritative destruction proof. Failed or abandoned cleanup closes admission;
an executor that discards work cannot establish native reclamation. A shared
destruction receipt with one release waiter must not be polled concurrently by
independent group releases. Native-wrapper bounds do not bound the DOM graph
reachable through the marker's retained Set.

Run the selected offline suite with
`CARGO_INCREMENTAL=0 cargo test --release --locked -p pwright-composition-chromium-file-input --features retained-target-marking --lib`.
The ignored real-Chrome ownership test requires dedicated Chrome/Pwright
preflight and the explicit loopback `PWRIGHT_TEST_CDP_WS` endpoint. Exact results
belong in the [canonical Handle checkpoint](../../docs/knowledge/pwright-node-handles.md).
This primitive does not establish actual Node ticket promotion, operation
deadline/call-ID integration, recoverable-versus-fatal host error classification,
late utility serialization or public retained upload parity.

## NODE-187: same-owner target prevalidation

The explicit `retained-target-prevalidation` feature composes both prior leaves.
`prepare_marked` consumes `OwnedMarkedElement` directly and returns
`OwnedValidatedMarkedFileInput`; the original marker-call admission, adopted
group and derived preflight admission remain nested until terminal cleanup. No
release/re-adopt conversion is exposed. The existing `prepare` and
`OwnedValidatedFileInput` APIs remain unchanged for independent consumers.

The selected host supplies capacity eight, 16 KiB owner storage per operation
and eight independent cleanup reservations. Run
`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo test --release --locked -p pwright-composition-chromium-file-input --features retained-target-prevalidation --lib`.
The final run passes 52 tests with one dedicated-Chrome primitive ignored. This
composition still performs no instrumentation, payload serialization or upload
mutation. See the
[canonical NODE-187 contract](../../docs/knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).
