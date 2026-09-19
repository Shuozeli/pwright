<!-- agent-updated: 2026-09-19T20:42:42Z -->

NODE-191 installs two default-off, capacity-one mutation scopes. The backend
reserves one ActiveCommand plus 64 KiB of BackendStateBytes for the sole large
CDP request. The composition owner reserves one LiveObject plus 16 KiB, keeps
512 bytes of fixed state, and pre-reserves one cleanup continuation before the
operation can start. Cleanup failure closes later mutation admission instead of
returning a reusable slot without native settlement evidence.

The existing NODE-190 96 MiB retained-graph allowance stays live until the
graph has moved into the started mutation and is dropped only after group
settlement. The existing validated target/marker/preflight owners, Node timer
and one 1 MiB FFI permit remain independently held. Direct N-API roots stay at
zero. Protocol validation caps the narrow graph at 64 files, 4 KiB per name,
1 KiB per MIME string, 64 KiB aggregate metadata and 70 MiB aggregate base64;
the administrative allowances are not measured RSS and do not bound Chrome's
`File`/`DataTransfer` allocations. [Canonical NODE-191 resource
proof](knowledge/pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190 adds one explicitly selected host scope. Installation claims 4 KiB of
fixed control storage plus one ActiveCommand and 96 MiB of BackendStateBytes
for the sole retained graph. The complete owner lease is acquired before the
addon copies a value from V8. Per-graph limits are depth 64, 8,192 visited
nodes, 8,192 array/object entries and 72 MiB total UTF-8 string bytes. A second
graph is rejected before exclusive ticket checkout; dropping/consuming the
first immediately restores admission.

Successful import changes direct Node roots from eleven to zero but does not
claim zero-copy behavior: V8 and Rust representations coexist during the
synchronous traversal until the phase commit drops the old root container.
The 96 MiB lease is an administrative worst-case allowance, not measured RSS,
allocator footprint or browser memory. The existing 1 MiB FFI permit, validated
target/marker/preflight leases and Node timer remain independently owned. No
CDP command allowance is acquired. [Canonical NODE-190 resource proof](knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: Late call-argument serialization keeps the same one-command, 1 MiB
FFI admission and validated Rust owner. It adds one logical persistent Node root
(ten to eleven) for the serialized wrapper graph, but copies no payload string
or binary into Rust and starts no CDP command. Synchronous getter/`toJSON`
execution can reenter; exact-session validation occurs before checkout and the
ticket is rechecked before the new root is installed. Reachable V8 graph bytes
are not claimed by the fixed Rust allowance, just as the preceding raw carrier
graph was not; the compatibility leaf records that limit honestly rather than
claiming a root count is a byte bound. [Canonical NODE-189 resource
proof](knowledge/pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: Awaited input instrumentation adds no Rust resource allowance. The
selected call keeps the same 32-preparation registry, ten logical roots, 16 KiB
registry/phase allowances, one parked 1 MiB FFI command admission and the exact
validated owner/cleanup reservations from NODE-187. Begin/finish are synchronous
Node-thread phase gates and do not resume the FFI admission or acquire a native
lease.

The restricted progress view, hook Promise and listener-owned values are V8
allocations. They have no Rust byte bound and are not added to native lease or
RSS totals. The one existing Node timer still owns timeout delivery and aborts
the exact ticket. [Canonical NODE-188 resource proof](knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: Selected target prevalidation keeps the existing one-command/1 MiB
FFI permit, 32 preparation slots and ten Node roots. The retargeted native object
ID is held only by Rust. The pre-existing eight-call marker admission remains
nested in each validated owner until the original group settles; no release and
re-adopt interval exists.

Two new scopes are selected only by `retained-target-prevalidation`. Backend
capacity is eight admissions, each with two RemoteHandle units and 19,546
BackendStateBytes. Composition capacity is eight LiveObjects with 16 KiB owner
storage each plus 512 bytes of fixed service state (131,584 bytes total), and a
separate executor reserves eight cleanup continuations. The fixed service state
adds one selected installation lease, making the idle retained-upload/marker/
prevalidation test baseline exactly seven rather than the old marker baseline
of six. These are Rust administrative bounds, not V8/DOM/browser RSS claims.
[Canonical NODE-187 resource proof](knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Selected successful marking now parks one concrete
`RetainedMarkedFileInput` inside the existing typed FFI result. Its fixed
completion layout remains compile-checked beneath the existing 1 MiB command
admission; no second command, permit or runtime lane is added. The existing
eight-call target-marker admission bounds simultaneously retained marked owners,
while the persistent per-world marker cache remains independently admitted.

Only selected preparation roots grow from six to ten: four additional roots
retain raw `filePayloads`, `localPaths`, `localDirectory` and `multiple` values.
Older profiles retain the six-root maximum. Ten is a logical-reference bound,
not a V8 or Buffer byte bound. Two truth flags live in each selected Rust record
and remain covered by the unchanged 32-entry/16 KiB registry and synchronous
phase checks. Abort/drop uses the owner's pre-admitted cleanup continuation.
[Canonical NODE-186 resource proof](knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: The selected call/deadline envelope adds no FFI command, native lane,
root or preparation slot. A per-scope `Cell<u64>` supplies protocol identity and
each selected record/snapshot carries `Option<u64>`; concrete checks still place
all 32 entries and synchronous replacement overlap inside the unchanged 16 KiB
registry/phase allowances. Materialization and marking reuse the existing one
1 MiB preparation admission.

A live truthy-timeout call owns one Node timer, one Promise and one Error until
its coordinator settles; zero/NaN schedules none. Those V8 allocations are not
byte-bounded by Rust and must not be added to native lease totals or presented as
an RSS bound. `abortDeadline` revokes the exact existing ticket rather than
allocating another cancellation owner. Exact verification and cleanup metrics
belong to the [canonical NODE-185 checkpoint](knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: The selected addon reuses each preparation's existing one-command,
1 MiB FFI admission for native target marking. It does not admit a second slot
or widen the 32-preparation, six-root, 16 KiB registry-administration or 16 KiB
synchronous-phase limits. The enlarged concrete registry layout remains checked
at construction. Native progress/state/outcome/fence, the 128-byte call ID and
fixed overlap are compile-checked below 1 MiB; the actual boxed future body is
separately reserved against the existing 128 KiB command-future limit.

Running ownership adds one weak revoker in the Node entry, while parked ownership
returns the original typed FFI preparation to that entry. The exact host source
registration is owned by typed progress until commit or abort. These values do
not add a native Handle pin, Node root or browser-memory bound. Existing marker
cache and registration ledgers remain separately admitted. Same-permit live
metrics and final zero-resource cleanup are recorded in the
[canonical NODE-184 checkpoint](knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: The default-off host `retained-target-marking` cache admits 64
current/creating worlds plus eight retiring predecessors, and independently
limits lookup/call leases to eight. Cache bookkeeping declares 128 KiB fixed
administration, 72 * 16 KiB world storage and 8 * 4 KiB waiter/lease storage:
1,312 KiB in one scope with 81 lease slots. These are additional selected
allowances, not a reuse of the existing Handle or upload budgets.

The installation independently selects 72 persistent marker owners and eight
mark-call owners, their backend native/byte allowances, 16 KiB composition
owner allowances, 72 untimed creation-driver reservations and 80 marker-cleanup
reservations. It also adds 72 existing 8 KiB world-snapshot slots and their
retirement-registry accounting. The persistent snapshot/receipt is independent
of every per-call paired snapshot; its owner contains no source Handle or work
token. Browser DOM graphs retained by the native Set and transport/runtime
allocation are not bounded by these administrative figures.

Stale-world pruning never frees a slot before terminal group ownership and
snapshot release. Open replacement uses eight retiring slots; once sealed, all
occupied slots may drain without new admission. Failed cleanup can return active
allowances but still poisons the cache; counters are not a fabricated native
ACK. Bounded generation receipts prevent stale completion from clearing reused
slots, and wakeups/drops run outside locks. No addon selection, Node root/budget
change, typed preparation promotion or public upload is implied. Exact evidence
and remaining gates are tracked in the [canonical NODE-183 checkpoint](knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache),
not a new live-pass claim here. Earlier resource checkpoints retain their scope.

NODE-182: Default-off `retained-target-marking` now implements an independently
owned private marker object in an exact utility world. Its persistent group is
not the temporary adopted target group. The backend admits state creation
synchronously and keeps the state/init allowance through acknowledged group
release or authoritative exact-world/session retirement. Each one-shot mark
uses the target's settlement-owned group for possible result/exception wrappers
and retains its independent allowance until that entire group is released.
Dynamic utility-realm Set/CustomEvent/dispatch behavior can retain or mutate DOM
graphs; two native-wrapper units do not bound those graphs or browser RSS.

For positive caller-selected state capacity `S` and call capacity `C`, the
backend declares `RemoteHandle = 2 * (S + C)` and
`BackendStateBytes = 512 + 20_020 * S + 22_930 * C`, with one scope and
`1 + S + C` lease slots. Independent counters enforce the two capacities even
when the shared ledger has spare resources. The fixed 512-byte administration
outlives the service while any state/call remains. Bounded identifiers, script
and typed/JSON overlap, primary/cleanup diagnostics and checked future/container
storage have explicit backing. Transport envelopes, supplying route/destruction
receipts and caller-retained returned diagnostics have their own ownership.

The composition selects separate state/call UploadProfiles and cleanup
reservations; it retains persistent state/route backing through in-flight marks
and the adopted target plus call admission through its terminal cleanup. Those
resources are additional, not paid for by the backend formula or the existing
eight retained-upload snapshot slots. Failed or abandoned cleanup poisons
admission before backing returns. An executor that drops promised jobs still
cannot establish native cleanup, and transport errors are not retirement proof.
Any destruction receipt with a single release-fence waiter must not be shared
by concurrent group releases; supply independent receipts or multi-waiter proof.

This is an owned-world primitive, not a production world registry or host/addon
selection. Connection-wide call IDs, the original action deadline,
instrumentation, same-ticket native handoff and public retained upload remain
open. Exact offline and isolated-browser evidence belongs in the
[canonical NODE-182 checkpoint](knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking);
none proves the missing Node integration. Earlier checkpoint notes below retain
their historical scope.

NODE-181: Preparation records and synchronous snapshots now carry the original
native wrapper's Weak state identity. Downgrade/clone add no backing allocation,
native owner, task token or JavaScript root. The actual Registry<InputRecord>
layout gate checks the enlarged 32-entry table against its unchanged 16 KiB
administrative allowance. The same concrete record/snapshot overlap has two
compile-time checks: the existing 1 MiB call admission and the independent
16 KiB phase scratch, which remains charged after reentrant FFI revocation.
PublicationPhaseStorage::CAPACITY_BYTES derives from the reservation constant;
the host lifetime test compares it with the actual held lease. No allowance,
six-root or 32-preparation limit increases. Two actual-layout admission/close
tests exercise preparation capacity, not native-wrapper lifetime or browser
cleanup. Actual-ticket browser evidence belongs in the canonical checkpoint;
final retirement observations require zero native-host, FFI and preparation
resources, not merely stable command-idle samples with a remaining watch.
These metrics exclude separately retained timeout-policy metadata ledgers.

Stale Weak references may retain dead Arc backing after active leases return;
neither zero counters nor these size checks establish global heap/RSS bounds.
Future marking requires separately admitted persistent exact-world remote
state, snapshot/native ownership and cleanup capacity; the existing eight
in-flight retained-upload snapshot slots do not pay for persistent marker
worlds. No such service or resource profile is implemented by this checkpoint.
Same-entry native progress and its original call identity/deadline remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: The existing retained-file-input selection adds eight weak invocation
slots under a 4 KiB BackendStateBytes admission, one scope and one lease. The
size proof includes the index/Arc/Mutex, detached batch, eight complete source
registration receipts and bounded control storage. A detached armed batch keeps
that allowance through revocation wakeups and result destruction. No registry
entry owns the FFI budget/result, native receiver or work token. Stale Weak
receipts can retain dead allocation backing after the active lease returns;
zero ledger counts do not prove tombstone RAM or browser memory reclamation.

Registration belongs immediately before native work, not at initial language
preparation: the 32-entry preparation limit is not replaced by an early eight-
entry limit. The FFI lane/base/future allowances remain independently owned.
Caller verification of the exact Node preparation scope and continued receipt
retention are required; metadata provenance alone grants no native-use right.

Host shutdown cancels outside locks, crosses the shared state/admission barrier,
detaches the weak batch, closes the operation tracker outside locks and revokes
before both operation and Handle drains. Owners seal before waking outside
their own lock. These ordering guarantees do not implement actual Node handoff,
marking, deadline/DOM/utility serialization or browser cleanup. Final evidence
belongs in the canonical checkpoint; no new browser pass is asserted.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Selected same-admission FFI parking retains the typed successful
result, lane and base-byte permit in the original entry. It returns only the
completed phase's future-body lease after the actual future is destroyed;
no task token spans a parked foreign await. Resume registers work before
extracting the result under shutdown's registry-to-entry lock order. Its
`(T, FfiExecution<T>)` return is a trusted internal boundary: immediately move
both into the next admitted continuation, never retain the owner after dropping
execution or carry an unstarted work token across arbitrary JavaScript waiting.
Weak per-entry revocation keeps Running storage through future destruction and
drops idle/late results before their permits, outside locks. Selected phase and
cancellation bookkeeping shares existing lane capacities and declared budgets;
this is not a measured RSS/global-allocation bound or browser-release proof.
Global shutdown now seals admission under the registry lock before invoking
either cancellation wakeups or tracker closure outside locks. All admission,
promotion, parking and inspection entrypoints reject the sealed interval before
the cancellation notification arrives; no late work token can cross that gate.
Default-capable and selected lifecycle regressions have separate evidence in
the canonical checkpoint; passing bookkeeping probes is not native cleanup.

The raw-carrier projection diagnostic retains language references and exact
short-circuit results without payload traversal, byte copying or serialization.
It adds no ticket stage or persistent root; actual preparation still ends at
Materialized with `FfiPreparation<()>`. Future projected-reference retention
needs explicit root admission, not hidden objects under the old six-root count.
Exact-session revoker registration and invocation before `Owners::wait`, native
marking/progress ownership, NODE-178 selection, utility serialization and public
upload remain open. FFI budget identity is not session/native-use authority.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Retained DOM prevalidation separately admits two extra RemoteHandle
units plus 19,546 BackendStateBytes per backend admission, without altering
existing upload/host budgets. Its composition claims a caller-selected
UploadProfile and independent cleanup slot; actual validation/release future
and wrapper sizes are checked before boxing. The one-shot allowance, original
group and exact retargeted receiver survive waiter cancellation and parked-owner
release through settlement. Known cleanup failure/abandonment poisons admission
before extra backing is returned. An executor discarding jobs cannot establish
native reclamation. Transport/V8/browser memory remains outside these Rust
administrative bounds. No host/addon or implicit runtime selection is added.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Two synchronous registration gates reuse exact owner-ticket and
creation-Page metadata; they do not acquire native pins, task tokens, roots or
new budget capacity. Current world/Handle/Frame runtime retirement is not an
upstream dispatcher-registration proof. Existing six-root/scratch/metadata
limits remain unchanged across setImmediate and the empty instrumentation await.
The internal Materialized driver returns undefined, avoiding Promise assimilation
of a user-controlled ticket; caller ownership persists until abort/promotion.
A foreign direct gate does not abort someone else's preparation; the orchestration
driver owns abort on its failures. Arbitrary callback/V8 memory remains outside
administrative bounds. Native ownership must be admitted at the correct later
phase, not by eagerly projecting PreparedFileInput before target/DOM/utility
serialization boundaries. [Canonical contract and verification](knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Server validation/materialization retain the same six-root ticket,
32-entry and 512 KiB phase limits. Begin/finish do not retain phase guards or
execution tokens across JS awaits. MIME lookup copies only a table-bounded
extension into Rust stack scratch; serialized data/strings and user callback
V8 graphs remain outside administrative byte claims. No native byte owner yet.
[Canonical contract and current evidence](knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Raw-buffer channel validation reuses existing
synchronous phase admission. ArgumentsReady becomes ChannelReady by replacing
root six: six roots, 32 entries, 512 KiB phase scratch and the NODE-174 784 KiB
timeout-metadata ceiling do not increase. No FFI work token, native-use pin,
feature, dependency or raw class is added. Buffer identity remains unchanged;
no bytes are copied or serialized. Observable map/raw valueOf results and
ordinary assignment remain language values, not byte-bounded V8 graphs.
User callback allocation, execution time and opaque allocator backing remain
outside these administrative storage claims.

Rust visits payloads (name, optional mimeType, buffer), localDirectory,
directoryStream, localPaths, streams and timeout in schema order. Trusted
frozen ports perform language primitives. Three no-capture/ZST native callback
specializations use static typed dispatch, with current Env/path/ports supplied
on invocation and no captured scoped N-API value, lease or native authority.
This avoids new Rust callback-Box allocation and its construction-failure leak.
N-API/V8 callback-owned backing and arbitrary escaped JS graphs are not bounded
by phase admission or the administrative RSS claim. Existing guards retain
scratch through synchronous revocation/replacement. Without a genuine
WritableStream producer/brand, the first `_object` read remains observable
but validation fails closed; valid-channel support is not inferred from shape.
Normal non-isUnderTest mode only is selected. Offline verification is recorded
in the canonical checkpoint; actual-ticket/browser/public gates remain open,
with no new browser pass. Deferred server bytes/path storage, float
deadlines and typed move-only FfiPreparation promotion/native submission need
separate proofs. [Canonical scope and exact evidence](knowledge/pwright-node-handles.md)
own those gates; prior accounting below is historical.

NODE-174 (2026-09-17): Selected timeout metadata now admits sixteen canonical
session indexes at 16 KiB each. Combined with 16 KiB factory administration and
1024 shared 512-byte slots for both cells and fresh scoped publication bindings,
the ceiling is 784 KiB across two ledger scopes and 1041 lease slots. This is
an increase over NODE-173's 528 KiB; the 1024 slots are not cells plus a second
unbounded binding pool. Each Context/Page lookup admits a fresh binding before
publication, while canonical cell identity remains shared. Binding clones share
their lease; copies inside Handle/FFI records remain under those records'
existing admission. Arbitrary V8 graphs/allocator backing are not byte-bounded.
The facade holds no timeout cell or binding and makes no metadata queries; it
forwards creation Page ID only. Each selected Frame producer obtains a fresh
binding in Rust while canonical Context/Page cells remain shared. After FFI
admission Rust retains either the policy or its lookup error through normal
source/argument materialization; policy failure is raised before host execution,
not by a JS lookup that could skip conversion. Idle lifecycle wrappers allocate
no policy bindings before native membership commit.

Indexes use bounded published Context/Page membership and exact Weak<Owners>
identity. Close drops cached cell ownership, but a retained closed index still
consumes its session slot until its last clone drops. Scoped outputs retain
only cell/binding/factory metadata and immutable weak creation-Page provenance,
not the index, HostSession, transport, native owner or running task. Handle
descendants and preparation inherit the creation binding in Rust; argument
assembly cannot substitute late settings. This does not grant closed/disposed
Handles native-use authority or remove normal source revalidation.

The facade captures raw debug values once; Rust owns undefined-only fallback
and inspector exclusions. Existing preparation/root/synchronous-phase limits
remain unchanged. No new feature/profile/dependency/class is introduced.
Public setters and arbitrary JS defaults, channel/native upload integration,
deferred materialization, deadline mapping and browser gates remain open.
[Canonical accounting and exact verification](knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding)
are authoritative; earlier accounting below is historical.

NODE-173 (2026-09-17): Selected `retained-file-input` adds an independent
client-timeout metadata budget: 1024 cells at 512 bytes each plus 16 KiB fixed
administration, 528 KiB total, with one scope and 1025 lease slots. Admission
precedes each Context/Page cell allocation. Cells retain their factory Arc so
administration survives until the final cell lease drops. They hold no Node
roots, HostSession, native-use authority, transport or running-task token.
Immutable parent shape allows only the normal Page-to-Context chain, not
arbitrary user-defined hierarchies. Typed `Option<f64>` defaults and an injected
debug bool are client metadata, not validated native deadline values.

Raw `assembleArguments` reuses the existing synchronous phase admission and
atomically replaces root six (converted value) with assembled params when
Ready becomes ArgumentsReady. Six roots per record, 32 preparation entries,
the shared FFI pool and 512 KiB synchronous scratch ceiling remain unchanged;
the separate 528 KiB metadata budget increases selected resources. Converted
and options spreads create scoped intermediate objects, not persistent roots.
Their allowance does not byte-bound arbitrary V8 graphs, copied user-property
sets, scoped handles or opaque allocator backing.

Rust timeout resolution invokes getters without holding metadata locks, then
reads live defaults; mutation during getters affects the same operation.
Creation-Frame/settings association remains caller-selected and unvalidated.
Public binding/setter/debug integration, channel/error conversion, Buffer/path
materialization, native timeout-domain mapping, promotion/submission and browser
evidence remain separate gates. No new feature/profile/dependency is implied.
[Canonical accounting and verification](knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
are authoritative. Earlier checkpoints below retain their historical scope.

NODE-172 (2026-09-17): The selected registry adds a separate scratch allowance
for at most 32 simultaneously active synchronous construction/replacement
phases, 16 KiB each (512 KiB aggregate). This is an explicit increase in selected
administrative resources, not an unchanged total budget. The original 32 entry
slots, 16 live session scopes, checked 16 KiB table reservation, 1 MiB FFI Command
allowance per preparation and shared 64 MiB FFI pool remain unchanged. Scratch
holds no FFI Command reservation, running-task token or native-use authority.

Initial `creation_phase` and subsequent phases pre-admit scratch before N-API
allocation. A lease survives callback-triggered abort/finalization until its raw
synchronous phase returns, independently of the revoked entry's permit; it does
not span arbitrary Promise waits. This bounds admitted old/new root-array
construction overlap even when reentry revokes and reuses preparation slots.
Callbacks are followed by liveness checks before further allocation, and
replacement rejects revocation. Registered private arrays still grow through
3/4/5/6 values, at most 192 across live entries. That count and the scratch
allowance are not byte bounds on arbitrary V8 graphs, callback-scoped handles,
opaque allocator backing, retained closed wrappers or weak tombstones.

A phase guard holds weak scope/registry identity plus its synchronous scratch
lease, not the preparation record or permit. Pure `RootSnapshot` inspection
clones weak source metadata and an `Rc<NodeReference>` under the borrow;
reference lookup occurs after it ends, and the temporary Rc drops before user
JavaScript. No N-API call occurs inside a registry borrow. Abort/close can remove
registered roots and return the preparation permit while a guard and its
independent scratch remain held until the synchronous return. Replacement and
rejected-record destruction occur outside borrows. No native work token,
byte copy or native submission is introduced. User JavaScript allocations and
synchronous execution are not bounded by the Rust size-policy check. Stateless
kernel diagnostics retain no persistent roots and do not verify real ticket
N-API behavior or browser cleanup. Raw closed/cancelled phase ordering remains
a public-alignment task. [Canonical accounting, contract and evidence](knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases).
Earlier checkpoints below retain their historical scope.

NODE-171 (2026-09-17): Selected Node preparation has 32 environment-wide
entry slots and 16 live session scopes. A fixed 16 KiB administrative
reservation covers the checked registry/live-scope structure; each entry also
holds 1 MiB from the existing FFI Command allowance through `FfiPreparation`,
without a running-task token. Admission precedes allocation of one private
three-value root array and its Node reference. This bounds admitted entries
and explicit root handles, not the arbitrary V8 graphs reachable from them.

The 16 KiB is not a bound on arbitrarily retained closed wrappers or weak
tombstone/control allocations. Tickets and Handle source identities are weak,
session-specific metadata, not native-use pins. Scope close, NativeSession
Drop and environment cleanup remove entries and drop roots on the Node thread
outside RefCell borrows before FFI draining. Terminal publication revokes when
delivered; raw sessions without a watcher can retain staging until later
publication, explicit close, wrapper Drop or environment shutdown. No immediate
peer-close reclamation is promised. Public conversion/promotion/submission and
browser validation remain open. [Canonical limits and evidence](knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).
Earlier checkpoints below retain their historical scope.

NODE-170 (2026-09-17): FfiPreparation shares the existing FFI lane and byte ceilings, with no work token until atomic promotion and no second permit at promotion. Shutdown revokes held preparation cells; a surviving receipt keeps only weak budget identity. Future Node roots require separate admission, thread-affine cleanup and session revocation. No new resource ceiling or Node-reference cleanup proof is claimed.
[Canonical preparation contract](knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution).

NODE-169 (2026-09-17): Retained uploads share the existing eight Handle calls and explicitly track native work through shutdown without reserving another persistent result. A separately selected resolver admits sixteen native units and eight temporary attempts; its eight-slot untimed cleanup queue is independent of the single upload queue. Only the new selection adds eight paired-world snapshots and retirement-registry slots for acquisitions that outlive runtime lanes (81 total, or 1105 with drag runtime). Descriptor Arcs are not native-use pins. The generic resolver profile declares selector capacity, but this path never reserves SelectorBytes.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): Compiling retained-reference alone does not install its
resource allowance. ResolutionProfile::with_retained_reference explicitly adds
one attempt scratch ceiling per two native-owner units. Each attempt reserves
two RemoteHandles, their existing owner bytes and a separate BackendStateBytes
scratch lease (bounded ID/JSON copies, two 1024-byte diagnostics and checked
future storage). Scratch is retained through failed-acquisition compensation
and returned after successful owner publication; the two native units remain
with that owner through group release. SelectorBytes, scope and lease ceilings
are unchanged. Decoded transport envelopes remain a separate transport bound.

Utility numeric metadata adds two i64 values per selected context route for
current/replacement storage, separate from existing native-string ceilings.
The snapshot's existing allowance covers its additional inline numeric field.
Old-world destruction cannot discharge unverified numeric-context adoption;
only after native unique-world validation may that proof cover the new group.
Local lease return after cleanup failure is not proof of native release.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress).

NODE-167 (2026-09-17): ownerFrame shares the existing eight Handle calls and
8 MiB call allowance but consumes no additional persistent result slot.
Atomic tracked-read admission adds two temporary RemoteHandle units per read
(result plus possible exception) at scope 154/1, sixteen aggregate units and
eight additional resource leases. Call/work/native ownership and a frozen
world snapshot survive started cancellation and terminal cleanup. Existing
112 cleanup slots suffice; unpolled cancellation does no native work. The
source descriptor is not a native-use pin, and generic Cleanup is not erased
by an unrelated or stale retirement snapshot. Scalar and batch descriptors
retain only the bounded creation-world fingerprint (Page, numeric context,
unique context); its identity bytes fit the existing 16 KiB owner allowance.
It retains no native client, receipt, snapshot permit or route lease and is
used only to classify an observed world replacement. [Canonical contract and evidence](knowledge/pwright-node-handles.md).

NODE-166 (2026-09-17): The selected host now admits a property batch's available
result owners, one shared call allowance, N child cleanup tickets, one aggregate
compensation ticket and the started-call continuation before native work.
Cleanup capacity becomes 112 (32 + 64 + 8 + 8). Batch inbox/detached storage is
explicitly charged at scope 166/1; no task or unbounded queue is added. Names
retain the shared call lease through language conversion; siblings use exact
ObjectId ownership and independent descriptor leases. The backend retains ONE
evaluation lease across carrier acquisition, raw property read and all release
settlement. N+2 temporary-wrapper admission and bounded-depth structural
staging checks precede parser copies. These are not browser Object.keys/RSS
bounds or a pre-read bound on malformed decoded CDP allocations. Ordinary
profiles remain unchanged. [Canonical contract and evidence](knowledge/pwright-node-handles.md).

NODE-165 (2026-09-17): Raw retained-result acquisition keeps its existing
evaluation lease through native settlement and failure compensation. At most
two IDs are borrowed from the already-owned response (result and exception),
deduplicated, and each addressable ID is attempted once; no ID cloning,
detached cleanup task or new admission class is added. Bounded diagnostic
writers use the existing error-buffer allowance. Empty, non-string, oversized
or missing retained-object IDs cannot be truncated into valid cleanup targets
and remain an explicit Cleanup failure.

Composition cancellation transfers the started future to its pre-admitted
continuation; persistent route admission remains held until cleanup settles.
This does not establish evaluation admission across a successful carrier's
later Runtime.getProperties/read/release pipeline. That pipeline, late
property-read settlement and a pre-owned object-group budget for unaddressable
IDs remain separate design work. See [canonical evidence and scope](knowledge/pwright-node-handles.md);
these accounting guarantees are not browser-RSS measurements.

NODE-165 shutdown ownership: untimed acquisition cannot return its existing
lease merely because the public caller stopped waiting. Handle-enabled
shutdown therefore revokes publication and retires this connection's owned
inspector sessions before draining Handle work; detach failure is retained
and forces transport disconnect before that drain. The non-Handle order is
unchanged, and no shared-browser or borrowed-Page ownership is broadened.
Externally retained descriptors can still own their admitted storage after
native retirement. Compensation racing detach can conservatively report
Cleanup even after acknowledgement: the host's pre-call snapshot is not the
actual acquisition identity. Matching a typed frozen route/client failure to
an exact retirement receipt remains future work, not permission to discard
cleanup failures. [Canonical resource and shutdown evidence](knowledge/pwright-node-handles.md#node-165-retained-acquisition-owns-failure-cleanup-and-shutdown-settlement)
records that residual without treating local lease release as native proof.

NODE-164 (2026-09-17): All raw native classes now derive module-local identity through the crate-wide N-API facade; these tags must reject foreign Rust layouts before any typed cast. Portable property reads reserve their persistent result route before snapshot I/O and reuse the single-result publication/compensation path. Abandoned operations reporting Cleanup are no longer treated as successful cleanup. These implementations address the prior priority audit gap; [exact verification, remaining resource/isolation gates and batch budgets](knowledge/pwright-node-handles.md) are recorded canonically; Node getProperties is not implemented.

NODE-163: The largest single-result creation future now has an 8 KiB compile-time ceiling explicitly included in the existing eight 8 MiB call allowances; the former 4 KiB runtime guard rejected all creation after producer composition grew. Native-class publication keeps its Box until construction/tagging succeed and reclaims failed wrapping. Failed JS wrapper construction requests native disposal immediately and awaits its acknowledgement. Batch result admission and sibling-specific cleanup identities are still required before getProperties can be exposed.

The same call allowance is shared through caller reply/topology completion and native continuation/publication; queued delivery no longer drops the charge while a suspended caller can retain its input reference vector. Sharing adds no lease or call slot. A single-property read can transiently own one carrier plus its result; carrier cleanup precedes portable publication. The 64 result-route limit is not a count of all transient CDP objects or browser RAM.

NODE-162: Native expression programs retain the original bounded source/argument graph for validation and admission even when the browser ignores argument values. No wrapper-source expansion, extra graph nesting, worker, queue or owner is added. Disposed-target errors occupy the existing bounded conversion/FFI call until materialization, with no target descriptor.

NODE-161: At most 4096 input nodes bound Handle slots and retained Arc owners. Their combined vector backing is below 128 KiB and is included in the existing 8 MiB FFI peak proof; owner/call capacities remain 64/8. Native sidecars share, rather than duplicate, the admitted live descriptor. FFI metrics are environment-shared: nested-session tests compare entry baselines and require zero only after all participating sessions close.

NODE-160: Selected Handle owner allowance rises from 8 KiB to 16 KiB (64 owners) to cover descriptor, accepted cleanup and completion backing/diagnostics. Reads and creation share eight 8 MiB call allowances. Disposed Node wrappers drop the descriptor synchronously after accepted cleanup and keep only a completion receipt, not service admission. Foreign-language-owned wrapper/receipt storage and JS heap are not asserted to be zero or bounded by SessionMetrics.

NODE-159: Selected host cleanup capacity is 104 slots: 32 ordinary, 64 result/recovery owners and 8 creation continuations. Publication inbox plus detached batch backing is charged at scope 159/1. Call/result allowances and frozen-world witnesses survive queueing and cleanup; existing addon profiles remain feature-cold.

NODE-158: Unpublished native Handles retain the existing route reservation and maximum native/preview allowance until publish or release. Busy commits retain that reservation and add no retry I/O. Callers must pre-admit retained future, inbox and abandonment/compensation storage; the raw leaf does not provide those host budgets or promise drop-triggered native cleanup.

NODE-157: Selected release state and eight queue-free deferred reservations use scopes 157/1 and 157/2. Handles add 64 existing world-access snapshot slots (8192 bytes each plus registry accounting); source-world witnesses live with native work, never portable descriptors. The aggregate route profile now follows leaf ownership charges. Native-close outbox capacity/backing storage is removed.

NODE-156: Native-release selects Arc-owned route leases, adding size_of(ResourceLease) plus two usize fields per route. Old requests/receipts retain native-ID/preview and RemoteHandle admission after route retirement. Deferred runtime capacity survives busy local commits. Feature-off representation/budgets are unchanged; caller-owned future/continuation storage remains separate.

NODE-155: Queue-free disposal reservations retain bounded order storage and pending permits without EventItem leases or control-lease arrays. Deferred preparation preserves all admission; terminal errors/drop release it. Optional deferred commit retains its existing notification allowance. Native operation storage remains separately admitted; these charges are not RSS.

NODE-154: Selected pending Handles admit 64 result owners at 8 KiB each, eight inputs/futures at 8 MiB each and 16 KiB table state (scope 154/1). The same cleanup executor has 96 slots only when handles is selected. Descriptor storage survives native close if callers retain it; its storage lease is independent of the native work fence and is not zeroed by a closed SessionMetrics snapshot. Defaults and native route limits are unchanged.

NODE-153: An abandoned Handle batch now attempts later disposals after a native release failure. Successful releases reclaim their existing route/object/RemoteHandle leases; failed releases remain visibly owned for explicit retry or parent retirement. One boolean aggregates failure with no new queue, budget or dependency. Node pending-result and retained-wrapper admission remain unfinished.

NODE-152: Optional native Handle installation admits 64 shared routes, 1024-byte native IDs/previews and eight evaluations under separate scopes. It fits the existing 256-object runtime; ordinary limits are unchanged. Pending result/wrapper/abandonment admission remains a required prerequisite to Node exposure.

NODE-151: PayloadTarget borrows bounded IDs/client without copying or adding a pool; existing payload admission is shared. Its caller must retain separately admitted native object/world/group owners. The future Node Handle path still needs explicit persistent-result, publication, adopted-wrapper and cleanup admission; these are not paid for by the upload source budget.

NODE-150: Selector-taking uploads reuse existing query admission and scoped Rust temporaries, avoiding JS-finalized intermediate Locators. Source, FFI, operation and native cleanup budgets are unchanged.
[Page/Frame upload contract and verification](knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149: Upload ingress admits 1 MiB metadata/future storage before getters, then independently admits actual bytes. No borrowed pointer crosses JS reentry or async work; source/transport/FFI ownership remains separate.
[Selected Node upload boundary and verification](knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Selected host uploads have independent two-source/eight-call counters, one filesystem job and one native upload. The source counter survives submitted-waiter cancellation via prepare_retaining; ordinary connection and FFI limits remain unchanged. Caller Node backing still needs explicit ingress admission.
Explicit transport disconnection fails pending/new commands, but queued writer bytes keep their leases until I/O retirement. It is a terminal teardown fallback after failed native retirement, not proof of successful group cleanup.
[Selected safe-host upload contract](knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: LargeCommandProfile adds independent count/wire and retained framing storage, held through queued writes, pending settlement and unconsumed replies. Source, encoded input, FFI and browser allocations remain separate owners; declared accounting is not an exact physical-memory measurement.
[Large outbound admission and remaining gates](knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: UploadProfile admits independent operation count/control storage before boxing the continuation, and keeps source, route and resolution owners through cleanup. The caller separately budgets backend, source and executor storage. Lost cleanup poisons future admission; transport closure is not browser destruction proof, and a dropped public wait cannot reclaim pending native command admission.
[Upload continuation and remaining gates](knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: PayloadProfile admits ActiveCommand and BackendStateBytes before base64/value construction, bounding call count, item count, base64 and metadata. Native object/world/group names are bounded to 1,024 bytes each and included in scratch allowance; by-value dispatch assigns exception wrappers to the borrowed resolution group. Original payload bytes, resolution groups, serialized wire envelopes and browser File/DOM allocations remain independently owned. Whole large-message/FFI peak accounting and started-call cancellation composition are still open.
[Payload ownership and capacity limits](knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: Executor WorkProfile declares queue ActiveCommand and BackendStateBytes requirements separately from file-source admission. One shared task owner spans queueing, reads and unconsumed completion; cancellation never releases its slot before the worker exits. Runtime/channel overhead is explicitly provisioned, while runtime thread stacks and allocator internals are not claimed as exact byte accounting.
[Queue/source ownership and limits](knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: PreparedPaths admits one batch plus per-path potential descriptor slots before stat, and storage for retained/temporary paths, MIME, reader values and source views. Payload source leases overlap explicitly during materialization; partial open/read errors drop readers before path admission. Provider-private heaps, OS descriptor memory, static MIME data and executor queues are distinct accounting boundaries.
[Filesystem ownership and limitations](knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: File-input content admits one LiveObject, planned ArtifactBytes and BackendStateBytes for entries, strings and owner bookkeeping before copying or reading. Length drift rejects after at most one extra stack byte; partial batches release their lease. Source handles, input backing stores, executor queues, transport/base64 and browser allocations are explicitly outside this leaf's claim.
[Accounting and cancellation evidence](knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: File-input preflight computes exclusive 50 MiB transfer and per-file base64 sizes under explicit metadata/count bounds without allocation. These counters are not leases or whole-operation peak admission. Future selected owners must account for Buffer/file/encoding/transport overlap; ordinary limits are unchanged.
[File-input policy, evidence and remaining work](knowledge/pwright-file-input.md).

NODE-140: The shared corpus records native lifecycle RequestCapacity during candidate newPage initialization. Current selection admits 256 request slots per Page, but the report does not identify the exhausted owner or prove missing reclamation. Do not increase capacity or drop events as a presumed fix. Production profiles and artifact hashes are unchanged.
[Expanded verification and retained failures](knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: Page lifecycle and registry terminal owners add inline Notify bodies, automatically included by their existing size_of-based profile admission. The borrowed close future adds no transport, cursor, task, registry retention or navigation slot. Its inline waiter storage is caller-admitted within the existing bounded Frame navigation operation allowance; this is not complete allocator/RSS accounting.
[Page lifetime publication contract and verification](knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Concurrent-close selection declares eight native slots, eight original topology/runtime/publication reservations, fixed 256-ID scratch per payload, a fixed completion ring and an explicitly preallocated 2304-message embedded outbox. Ordinary runtime selection keeps its outbox disabled. This is bounded admission, not complete allocator/RSS accounting.
[Node close-driver integration and final verification](knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: Close retirement borrows caller-admitted fixed disposal-ID scratch and Handle selections rather than allocating additional ID Vecs. It retains existing capacity through the returned notification owner. No default pool/feature is selected; complete graph/container peaks remain a separate outstanding requirement.
[Native local retirement and post-lock notification contract](knowledge/pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: The close-capacity bundle owns existing reservations, not a new heap/pool or a complete budget. Scheduler profiles account for its inline payload; component and callback heaps remain separately admitted. Completion consumption retains native evidence/slot during the callback, with ancestor admission retained through joined children.
[Joined close capacity and native completion contract](knowledge/pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: The guarded publication callback uses existing deferred-disposal admission; its successful result retains that slot/lease, and failure explicitly drops bytes before returning the slot. Callback/output/error heaps and stable disposal-ID scratch remain separately caller-admitted. No new pool, worker or default resource selection.
[Runtime-locked publication/disposal contract and evidence](knowledge/pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: ControlOutboxStorageProfile explicitly admits old/new message and lease buffer payloads, preallocates to the exact logical queue limit and retains its peak allowance for the RuntimeStore lifetime. Count/EventItem limits remain separate. Queue clear and transport close do not release backing storage; graph/table nodes and other commit scratch remain outstanding.
[Preallocated runtime outbox contract and evidence](knowledge/pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Selected world receipts add one owning-Page target bounded to 1024 bytes. The existing 8192-byte snapshot permit covers six bounded IDs plus headers; installed registry admission automatically includes the larger inline receipt. Native-retirement-off budgets are unchanged; no RSS bound is claimed.
[Native Page world-retirement contract and evidence](knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: Native close commit scope borrows sealed evidence and native IDs while retaining the completion's existing admission. It adds no success-path collection, ID clone, subscription or lease pool. Callback captures/output/scratch and deferred notification owners require their existing independent admission; this scope is not a total commit-memory guarantee.
[Native close commit-scope contract and evidence](knowledge/pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: with_deferred_commit validates the whole runtime pending-call limit and adds collection-payload peak admission for traversal, removed records, affected calls, signal references and saved wakers. Returned records/wakers retain the original slot and bytes after notify, until drop. Allocator metadata, user-waker heaps and caller diagnostic heaps remain separate; this is not a total RSS bound.
[Deferred runtime close commit contract and evidence](knowledge/pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: Deletion-only route retirement owns borrowed route/network guards and a borrowed disposal slice, without new route collections, IDs, provider allocations or subscriptions. Its inline guard/wake layouts still belong in the eventual coordinator envelope; caller-owned disposal/topology/runtime scratch remains separately admitted. No new standalone lease pool or default budget increase.
[Deletion-only close route contract and evidence](knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Native scheduling declares a measured fixed vector/state envelope plus N retained completion/Arc/error envelopes, in addition to independent Page-command, authority and CDP fanout budgets. Retained ancestor joins and returned completions keep their original permits; payload-owned heaps require separate admission. Full retirement/publication scratch peak remains outstanding.
[Bounded native scheduler contract and evidence](knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Native ancestor authority declares one scope, N+1 leases and 2048 + N*16384 BackendStateBytes. The shared nonzero N limit includes retained Page bindings and Context receipts, not only running commands. IDs are bounded at 1024 bytes; byte/slot rollback and all-resource teardown are tested. Other native/runtime/topology/publication budgets remain separate.
[Native ancestor authority contract and evidence](knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Split native close preparation checks both future bodies, two bounded native-ID copies, wrapper/owner storage and bounded interruption/error allowances against the existing 16 KiB call envelope before native mutation. Pending/cursor limits and source budget are unchanged. Phase/drop/transport tests prove exact lease reclamation; full coordinator peak budgeting remains separate.
[Native close phase contract and evidence](knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred Page/Context publication reserves 1/17 lifecycle groups plus an envelope covering both owner boxes and vector buffers. Existing metadata/factory ledgers cap concurrent admission; no extra queue or task is installed. Full native coordinator peak accounting remains outstanding.
[Deferred publication contract and evidence](knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Runtime disposal reservations add retained pool/owner/vector byte admission, an explicit shared pending limit and logical outbox slots. EventItem leases remain within the existing runtime queue budget and use the selected extension scope through FIFO drain. Ordinary mutation admission subtracts reserved slots; failure releases leases before capacity. Weak waiting owners do not retain the runtime. Feature-off layouts remain unchanged; commit scratch/native/publication peak ownership still needs composition-level derivation.
[Runtime reservation contract and evidence](knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Opt-in topology reservations declare a separate extension scope, N pending slots and an additional committed-shadow allowance: 512 + (N + 1) * (M + 128) BackendStateBytes and N + 2 leases, where M is the existing native-string shadow maximum. Pool clones share the same atomic limit. Layout, exact-capacity, overlap, failure rollback and complete-drain tests pass. This does not account for future runtime/publication storage or represent RSS; ordinary/Node profiles are unchanged.
[Topology reservation contract and evidence](knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Context terminal settlement and reply-producer/request-queue-loss sealing reuse existing operation/FFI/publication owners and stop tokens without a new production allowance, task or dependency. Queue-full backpressure remains unchanged. Diagnostic traces filter unrelated commands before consuming 32 pending slots, record eight Context identities and can hold eight retirement packets of at most 4096 bytes each. Future concurrent close intents require a separately derived reservation plan; the current one-close confirmation allowance does not permit unbounded parallel waits.
[Close cascade contracts and evidence](knowledge/pwright-close-cascade.md).

NODE-122: Selected close confirmation pre-admits one exact-target cursor, a 4 KiB source allowance plus measured wrapper/Arc ownership, and a 16 KiB call owner with a 1 KiB injected interruption-future allowance. The measured declaration totals 24,840 bytes, excluding separately derived connection fanout storage. Error formatting itself is bounded to 1024 bytes per field, without an unbounded intermediate allocation. Target IDs cap at 128 bytes. A separate atomic pending-slot limit prevents borrowing unused shared-ledger capacity. Failed preparations and abandoned continuations release RAII leases; owner loss is not a native-success receipt. Both final held-close and Worker gates pass with zero live host/FFI ownership after cleanup. Admission bytes are not RSS measurements.
[Confirmation resource and verification contract](knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Test-only close tracing explicitly caps pending commands/session mappings at 32, created targets at eight, close receipts/recent completions at 128 and native retirement receipts at 64. Original-scenario reports cap child streams at 64 KiB and total output at 4 MiB. Production resource owners are unchanged; the next native confirmation owner requires independent pre-admission and teardown proofs.
[Transparent trace and next confirmation contract](knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Native driver attribution adds failure-only bounded backend text, not a runtime journal or resource owner. Prefixes share the existing 4096-byte UTF-8 diagnostic cap. Test-only relay reports cap pending identities, receipts, targets and output; immediate queued FFI watches are measured separately from bounded environment-turn drain.
[Close attribution contract and evidence](knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: The selected ready source pre-admits 32 groups and fixed vector storage before initialization, replenishes before creation/discovery publication and rolls back abandoned creation. A separately charged per-Page acknowledgement Arc is admitted before local activation; staging only clones it. Ready, external-close, explicit-close and terminal capacity remain distinct. Cold fixed layouts include new optional fields; no zero-byte opt-out claim is made.
[Native ready-source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Creation conversion reuses the existing fixed 32-KiB port allowance; its compile-time bound now includes creation DTOs and six possible ID copies. Missing Context/root/tree bindings have independent last-reference metadata leases. Reused wrappers acquire no new lasting binding; no runtime owner, queue or registry slot was added.
[Creation-conversion contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: The selected external-close source pre-admits 32 lifecycle reservations plus its fixed pool storage before initialization. Replenishment precedes creation/discovery publication and rolls back on abandoned creation or partial admission failure. The pool is independent of explicit and terminal close capacity; the new optional pointer increases fixed cold-owner layout by 8 logical bytes on this artifact.
[External-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: The close source admits its observer Arc, boxed native transaction receipt and fixed 32-notice vector through the metadata factory before allocation; each notice also owns an existing-queue lifecycle reservation. Failure rolls back admission before native close. Partial postcommit publication failure forces native reconciliation to retire the Session, retaining the last staged mask.
[Explicit-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: The selected Node port reserves 32 KiB plus FrameConversion bookkeeping from the Session metadata factory, lasting until its final strong owner. Closed retained Browsers/Locators therefore retain one port revision lease, not a native runtime. The separately admitted fixed 16-slot weak cleanup registry also covers empty Rc allocations retained by weak slots, and has no FFI task token.
[Terminal dispatcher contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Pwright v2 resource ownership and limits

NODE-114: Selecting the terminal source admits 32 group slots, 33 factory revision leases and 13,448 logical bytes on this aarch64 build before native initialization. The collection owner adds one optional box field, automatically included in the fixed index size allowance. No teardown allocation or new queue/task is added.
[Terminal source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Each selected lifecycle group reserves an EventItem and its boxed body before native work, using existing ledgers. The box remains in the fixed queue until acknowledgement, so discard releases it even with a retained drain. No extra queue/task or runtime owner is introduced.
[Group contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Only the selected close decorator owns an observer Arc. Its implementation must
admit the reservation box and every retained payload before native close; drop
returns unused capacity, including failure and continued cancellation paths.
Ordinary service layout is unchanged; existing async allocations are not zero.
[NODE-112 close receipt contract and verification](knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The new committed driver view borrows topology and an existing effect slice;
it adds no retained storage, ledger allowance or subscriber. Future retained
cause groups require admission before native mutation, not allocation in the
under-lock observer. Test-only receipts are capped at 32 effects / 32 records.
[NODE-111 source contract and verification](knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 adds a scoped exception port with no retained storage, reference, queue
or new ledger allowance. It forwards a borrowed JS value without serialization;
Node owns its error-processing allocations. Diagnostic children separately cap
32 trace records, four errors, 64 KiB output streams and 14/15-second lifetimes.
[NODE-110 contracts, fix and verification](knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 adds size_of(DrainOwnership) plus one size_of(FramePublicationDrain)
to the selected fixed queue allowance before queue allocation. Successful claims
and advances allocate no queue/container and admit no extra lease. Only one
active guard retains the metadata endpoint; completed guards retain none.
Foreign wrappers and notice journals still require separate future admission.
[NODE-109 contract and verification](knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 adds TRACE-only borrowed detach/destruction identities after event
admission. Native IDs are checked against 256 bytes before logging, malformed
values are flagged, and no payload copy, runtime queue, lease or subscriber is
added. Logging sinks own any formatting storage. The test-only recorder separately
caps 64 receipts, 128-byte fields and 32 unanswered IDs each, failing on overflow.
These diagnostic bounds do not change production resource admissions or measure
browser/process RAM.
[NODE-108 receipts and verification](knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 retains the existing metadata ledger handle as a revision domain proof.
Measured revision allowance is now 1256 bytes (+8); the 32-retirement pool plus
vector storage is 42496 bytes (+256), still 33 revision items. Fixed publication
storage remains 31504 bytes. Rebasing makes bounded scans and clones an admitted
revision without new heap storage/admission; it does not allocate a union of old
and new members. Event cursor/delivery budgets remain separate and unimplemented.
[NODE-107 contracts and evidence](knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 measures the immutable view body at 16 bytes, borrowed collection and
iterator bodies at 24 bytes each. Mask transitions allocate nothing and retain the
existing revision lease. Conservative revision allowance is now 1248 bytes;
fixed publication storage 31504 bytes; the 32-retirement pool plus vector storage
42240 bytes / 33 revision items. These supersede the NODE-103 layout numbers,
not its capacities. Hidden descriptors remain charged until final release.
Future event cursors/delivery captures need separate admission; this is not RSS.
[Layout evidence and ownership](knowledge/pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 adds no production allocation or budget. Its proposed event publication
owner must separately admit typed notices, per-step retained metadata, cursor
storage and foreign callback/conversion ownership before activation. Existing
32-retirement capacity alone is not proof for a new per-event representation.
Terminal publication must remain allocation-free and Worker discard must not call
foreign code. Measure actual layouts before implementing the selected integration.
[Design requirements and red evidence](knowledge/pwright-node-lifecycle-events.md).

NODE-103 wires the selected collection producer and Node readers. A revision now
charges 1,232 bytes/one revision slot; the preallocated 32-retirement pool and
vector charge 41,728 bytes/33 revision slots plus 32 metadata-queue items.
Fixed index/queue storage is 31,496 bytes/one state. Creation replenishes before
native activation and rolls back on failure; removal uses admitted capacity.
Terminal/discard releases producer ownership. These are logical Rust budgets,
not process RAM; older checkpoint numbers below are historical.
[Current contract and verification](knowledge/pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).

NODE-102 implements pre-admitted collection revision storage in the existing
Rust publication queue. Each bounded 16-Context/16-Page revision charges 1,144
bytes and one existing Frame revision slot until its final reader drops; a
separate metadata queue slot is reserved before activation and released on
delivery. Fixed selected index/queue storage is now 31,256 bytes/one state.
Terminal publication and discard release current ownership without allocating.
Native producers/Node getters are not connected yet; this is not a browser RAM
improvement or a fix for NODE-101's red gate.
[Exact ownership, tests and remaining integration](knowledge/pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 adds test-only collection publication characterization, not a new
production budget. The proposed Rust view must charge retained descriptors and
pending membership changes before native activation, preserve conversion proofs
and pre-admit saturated teardown. Native active capacity alone cannot bound
unpublished retirements plus replacement creation. No new storage is implemented.
[Evidence and required resource proofs](knowledge/pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 embeds Browser connection publication in existing Rust metadata.
There is no new task, queue, cursor, dependency or per-call lease; native
cancellation/admission remains independent of the public flag. Combined Page
enumeration still measures 31,248 fixed tree/index/queue bytes and one state,
with all host ownership and pending metadata counters reclaimed.
[Scope and evidence](knowledge/pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 pre-admits one metadata EventItem per Page for eventual closure. The
single existing queue obeys queued items + live close permits <= 256; 16 active
Pages leave 240 ordinary slots. Its entire selected VecDeque capacity is admitted
before allocation, using the existing Frame ledger. The shared fixed index/queue
charge is 31,248 bytes and one state on this Mac; close tokens retain charged
metadata until publication/discard, never a host or transport. MAX sequence is
reserved for the allocation-free terminal batch. No new task/cursor/dependency.
These are logical native budgets, not per-chat RAM or total V8 memory.
[Derivation and tests](knowledge/pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-095 adds bounded identity storage to selected pending CDP calls: 256 bytes
plus Option<Box<str>> metadata per existing command lease in the Node profile.
There is no session history, new lease or cursor; unsent wire is released on
retirement. Event admission/fanout budgets remain unchanged.
[Contract and evidence](knowledge/pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 changes only selected Context classification. It adds no production
storage owner, task, queue, cursor, feature or dependency, and avoids redundant
Context-list refreshes in that mode. The test relay has explicit bounded target,
Context and pending-command inventories; these are not production allocations.
[Bounds and evidence](knowledge/pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-092 only changes terminal classification of an existing second-stage detach
receipt. No retries, new queues/tasks/cursors, dependency or allocation owner is
added. Tests require zero pending native registrations and candidate disconnect
owners; this is not a native-waker/Node Worker shutdown proof.
[Boundary](knowledge/pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-091 reserves a fixed 720-slot weak watch registry (nine requests x 80 routes).
Each request watch owns three <=256-byte native IDs and two cleared-on-drop waker
slots. Primary/compensation share watches within the existing 128-KiB plan and
992-KiB request allowances; weak-slot storage is charged separately. No extra
cursor/task or unbounded session history. [Derivation](knowledge/pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-090 explicitly discards lifecycle storage reserved for a Page that disappears
before publication. Published owners are preserved; escaped unpublished handles
are sealed and retain their lease until drop. The existing bounded registry is
reused without a new pool/task/cursor. Ledger reservations are not RAM estimates.
[Contract](knowledge/pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 retains the 992-KiB request and 419-owner bounds while making Page
snapshot-cell/Arc and sequential-group storage explicit inside the 128-KiB plan
sub-budget. First-Page snapshots are shared; later Pages materialize one owner
at dispatch. Test-only CDP receipt gates are bounded and loopback-only, not
production runtime owners or resident-RAM estimates.
[Resource derivation](knowledge/pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 keeps nine request owners through primary/compensation drainage. Each
reserves 992 KiB including two 80-member fanouts and a checked 64-KiB job body;
each native future is checked <=4 KiB before boxing. An 80-permit semaphore
bounds native JSON scratch with backpressure. The header pool admits 419 owners
for nine possible compensation plans. No extra runtime task is spawned.
[Derivation and shutdown exclusions](knowledge/pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 selects nine metadata writers and a derived 291-slot header pool for
sparse settings, prior/input owners, nine 16-Page frozen plans, initializers and
compensation. Existing request allowances cover two route/plan copies; no lock
crosses native awaits. Prepared navigation retains its slot even before polling.
[Derivation and cancellation limits](knowledge/pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 adds an explicit writer count to HeaderSettingsProfile, default one.
Pending Vec storage is charged before allocation; prior snapshots retain their
existing pool owners. Outstanding writers keep empty tombstones until resolution;
retirement removes all affected records. Concurrent native uncertainty remains
conservative and does not block desired-state reads. Host concurrency is pending.
[Profile and reclamation evidence](knowledge/pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 reserves eight independent 32-KiB navigation preparation allowances in
the selected header installation. Implicit referrer copying is admitted first and
capped at 16 KiB; explicit strings move without another copy. The slot guard stays
alive through native completion and drops on conflict/cancellation. Repeated
public conflicts followed by navigation exercise reclamation.
[Selected bounds and exclusions](knowledge/pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 creation headers share the existing nine request permits and 128-KiB
request allowance. Prepared bounded Rust strings precede native Context creation;
FFI reserves 512 KiB plus 32 KiB for baseURL/publication. No new runtime owner or
cursor is introduced. Oversized creation rejection and eighteen Context retirements
pass publicly; injected post-create cleanup failure remains unverified.
[Ownership and exclusions](knowledge/pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 adds sixteen pre-admitted `(ObjectId, HeaderSyntaxError)` slots to the
selected header installation body. Records contain no strings or runtime owners;
first failure is latched, duplicate sessions do not grow storage, and verified
Page retirement reclaims slots. Desired strings remain within existing snapshot
bounds; final syntax validation is borrowed and allocation-free. Failure cleanup
reuses the completed creation call ID for sequential exact-Page compensation.
[Bounds and recovery evidence](knowledge/pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 admits a 512-KiB FFI ticket before Rust header copying, bounds input
to 256 enumerable keys / 64 KiB copied strings, and retains lexical JS handles
only through synchronous collection/conversion. Async work owns Rust data only;
settlement and failed-input paths release their tickets. V8 enumeration arrays
are created before their length is visible and remain external VM allocations,
as do arbitrary getters/valueOf work. No hard process-RAM bound is claimed.
[Allowance, exclusions and live cleanup](knowledge/pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 reserves header adapter/native scratch before allocation, independently
caps nine request owners and 80 native applications, and tracks at most 160
modified session IDs. It clears obsolete native header state before forgetting
IDs; topology replacement alone is not native retirement. Queued requests drop
before I/O shutdown, preventing a retained-connection wait cycle. Fixed budget
reservations are not RSS. Two live runs end with zero host leases/operations.
Ordinary event reconciliation has no stale-session scratch allocation when
all modified routes remain current, or when no headers have been configured.
[Exact allowances and exclusions](knowledge/pwright-extra-http-headers.md#admission-and-remaining-compatibility-work).

NODE-080 pre-admits the sparse Context/Page index and shared transaction body.
Original/proposed/prior/effective headers retain separate HeaderPool admissions.
A single writer prevents overlapping metadata transactions; cancellation retains
bounded uncertainty markers, while native retirement and final drop reclaim
owners. [Limits and exclusions](knowledge/pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-079 adds borrowed IDs/kind to the existing optional initializer call.
No settings table, task, cursor or queue is added. The callback's retained
inputs/future and eventual native/FFI copies remain caller-admitted costs;
portable IDs do not themselves retain a Page or prove public readiness.
[Scoped ownership contract](knowledge/pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-078 introduces caller-configured header owners/count/byte bounds. Snapshot
copies and merge planning require admission; clones retain one charge through
final drop, and a local owner cap cannot borrow unrelated shared capacity.
Native/FFI storage remains a separate unfinished integration requirement.
[Exact allowance and exclusions](knowledge/pwright-extra-http-headers.md#rust-ownership-contract).

NODE-077 exercises the existing Frame FFI/result owners during Worker teardown:
pending owned navigation and queued conversion both remove their exact native
Context/target, while borrowed loading survives. The queued test observes one
Command and at least 512 KiB of FFI reservation before termination. No budget or
production owner changes; post-destruction ledger/RAM cannot be read from that
Worker. Public timeout continuation separately confirms waiter cancellation is
not browser stop-loading at three tested wait policies.
[Ownership evidence](knowledge/pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 Frame.goto reserves a 512-KiB existing Command-lane FFI allowance
before bounded native input copies. Its typed host output retains the separate
384-KiB result charge through N-API conversion; the FFI ticket covers cloned
strings, lowercased header map and JS string-conversion overlap. Async bodies,
native arenas and Context URL-value copies keep their own admissions. No Rust
response pointer is handed to JS or made dependent on JS GC after delivery.
[Owner/copy boundary](knowledge/pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075 adds namespace 126/3 for 32 active-or-escaped typed navigation results
at 384 KiB each, and 126/4 for their independently leased Arc counter. The
explicit local count cannot borrow other features' spare plan capacity. JSON
projection admits before dispatch and bounds source/headers/strings; returned
owners expose only borrowed outcomes and retain no native arena or host.
The executor admission/TaskTracker/deadline covers projection. Native/foreign
copies, allocator metadata and caller stack remain separate bounds.
[Proof and native escaped-result test](knowledge/pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074's selected navigation 1.2 codecs borrow input/result values during
encoding and traverse bounded headers in canonical order without a copied index
or response. Existing explicit string/header/count/node/depth and aggregate
header-byte limits bound wire allocation. Decoding rejects attachments and
forged duplicated route IDs. No runtime lease owner, native cursor, browser or
background task is installed by the contract feature. FFI copies and mapping
from the host's charged response arena remain separately unfinished admission.
[Boundary](knowledge/pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073 selects a 128-Frame/eight-snapshot retained route owner and eight
48-KiB host executor allowances. A separately admitted Arc counter enforces
eight operations before publication waiting: shared-plan spare capacity does
not expand this local count. URL/referrer capacity is bounded before retention.
Pending native cancellation reclaims its call slot but keeps queued wire bytes
charged until writer retirement; it does not stop browser loading.
[Detailed ownership](knowledge/pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 adds separately admitted fixed Page registration indexes and per-call
Arc/Mutex/notification bodies. Ticket drop reclaims a slot on the next checkpoint;
moved results keep their core leases after all Page/driver owners drop. The
host's separate `frame-navigation` feature declares 128 registration owners,
allocating response storage only on registration. Readiness-only has no such
budget. Caller executor/task storage, native requests and FFI copies are separate.
[Bounds and reclamation](knowledge/pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 admits one navigation invocation's fixed event/request slots, response
and error arena, receipt ID and owner body before allocation. Escaped response
bytes borrow the owner; the lease stays until drop. Redirects reuse chain slots;
capacity failure seals instead of truncating or growing. Caller Arc/Mutex,
envelopes, native pending calls and FFI copies remain separate admissions.
[Contract](knowledge/pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-070 admits a fixed lifecycle registry index and distinct per-slot Page
owners. Active and escaped-retired capacities are separate; a retained handle
prevents slot reuse. Handles retain a shared terminal body whose lease also
conservatively keeps the index allowance until final drop. The selected host
separately admits its shared clock/readiness-sink allocation. These are declared
capacities, not measured process-RAM guarantees.
Registry tests and the real-host gate verify retained leases remain after close
while escaped handles live, then return to zero after final drop. Concurrent
shutdown rejects queries even while a synchronous notice callback holds its lock.
[Bounds, reclamation and exclusions](knowledge/pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 explicitly admits one Page lifecycle owner's body, Mutex and Arc
overhead in addition to the fixed native/reducer capacities. Clock/sink pointees
and any retained response/error payload remain caller-owned admissions. The
observer borrows the existing cursor's envelope; it adds no queue or task.
Sealing retains storage charges until final drop, not until Page retirement.
[Boundary and verification](knowledge/pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-068 reuses the existing topology shadow transaction and descendant-removal
storage; it adds no persistent loader/history map or new resource profile.
Document commit disposes old descendants only after runtime/service-route
publication succeeds. Retained snapshots remain admitted after retirement;
renderer handoff can retire a route without destroying its portable Frame.
[Ownership and verification](knowledge/pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

NODE-067 pre-admits native Frame/request rows and a fixed native-ID arena in a
separate scope before allocation. Transitions cannot grow that storage. Borrowed
response/error notices do not retain envelope payloads; future escaping values,
event queues, authoritative topology, native sessions and FFI promises retain
separate admission obligations. Unit and live tests verify lease/byte release.
[Formula and exclusions](knowledge/pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

NODE-066 separately pre-admits two Frame-navigation route tables and bounded
escaped snapshots. Snapshots keep old route/service/ID storage admitted after
retirement; clones share capacity and last drop releases it. Route allocations
drop before the final table allowance; accessor clones share one admitted
service-owner Arc. Exact Vec/String
capacities are checked. Client factory internals, transport, caller input/results,
event delivery and process RAM remain separate boundaries. No event queue or
snapshot storage is installed by default.
[Formula, exclusions and tests](knowledge/pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

NODE-065 reserves the full lifecycle store body and fixed Frame/request slots
before construction. Later transitions allocate nothing; errors/loss seal
snapshot queries, and drop releases the lease. Native event normalization,
delivery queues, retained routes and FFI promises need separate ownership proofs.
[Exact charge, exclusions and tests](knowledge/pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 Frame initiation shares existing navigation operation/byte admission
and borrows the native Frame ID; topology navigation routes borrow existing maps
without an added allocation or index. These are not owned asynchronous routing
leases. Future retained routes, lifecycle history, subtree requests and FFI
delivery require their own admission and retirement proof before public exposure.
[Current bound and pending ownership](knowledge/pwright-frame-navigation.md#resource-and-dependency-contract).

NODE-063 lookup admits 32 KiB before cache-ID conversion/callbacks, with a
compile-time proof for 128 selected-path DTOs and 256 numeric known IDs.
Pinned reads and persistent native bindings retain separate existing leases;
glob planning keeps its own reservations. The native ancestry test binds only
the selected nested Frame and missing parent; callback recursion rejects and
unwinds. Last GC returns to 1 state / 0 revisions / 7672 fixed-index bytes.
Node language values and process RAM remain outside this logical quota claim.
[Proof and evidence](knowledge/pwright-frame-lookup.md#selected-path-ownership-and-admission).

NODE-062 Context scope 121/4 is independently bounded: 16 registry slots, 256
settings/bindings, 128 conversions, 16 KiB/baseURL and 2 MiB logical bytes.
Real Node pressure retains 127 closed configured Contexts (255 states/36629 bytes),
rejects another configured creation, and returns to 0 states/560 fixed-index bytes
after GC. Foreign callback reentry exhausts admission and unwinds all conversions.
These are native logical quotas, not process RSS or Node URL/parser memory bounds.
[Proof and verified evidence](knowledge/pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-060 admits N-API glob input validation/copy as 2*n+1024, and URL-output
copies as s+o+2*max(s,o)+1024, each plus a guard body. Five read slots can overlap
with preparation/restoration/final plan. Actual conversion and failures release
all temporary charges; Node-owned URL/parser allocation is outside this logical
Rust ledger and is not claimed as a bounded process/RSS total.
[Proof and evidence](knowledge/pwright-frame-lookup.md#admitted-native-copies-and-retention).

NODE-059 adds preprocessing admission: input/final glob 16 KiB, relative-path
capacity at most 10*n, two fixed restoration buffers each capped at 256 KiB,
and conservative 24*n+32 lowercase temporary storage plus owner bodies. Peak
is three existing read slots; completion retains only the final GlobPlan lease.
Borrowed URL parsing is separately owned and still needs production admission;
the diagnostic parser is not covered by these logical-storage bounds.
[Derivation and rollback gates](knowledge/pwright-frame-lookup.md#preprocessing-storage-contract).

NODE-058 shares existing Frame read/byte admission for lookup and glob plans.
Lookup adds no body beyond its charged immutable read; callbacks run unlocked.
For resolved glob length n <= 16 KiB, reserve 7*n+2 output bytes and owner bodies
before allocation. Plans hold their own read slot through conversion/drop.
This does not bound the still-unimplemented baseURL preprocessing stage.
[Derivation and rollback tests](knowledge/pwright-frame-lookup.md#proven-planner-storage-bound).

NODE-056 explicitly selects a separate metadata-only name ledger in the Node
host: 16 KiB/name, 2 MiB total, 512 leases. Nonempty names reserve text, body and
Arc counters before allocation; revisions share identical names and retain old
charges through publication/conversion. URL-only books allocate no name ledger.
Public `nameBytes` is logical storage, not V8/RSS.
[Accounting and evidence](knowledge/pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 reserves four Page-creation states including both root and tree
bindings. Synchronous descriptors hold a 32 KiB metadata conversion lease through
actual Node conversion, with a compile-time DTO-size proof; retained native tree
wrappers keep separate leases until finalization. Cached enumeration emits only
IDs. Explicit value-evaluation activation adds no resource requirement or task.
[Accounting and public pressure evidence](knowledge/pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 includes the main-Frame binding in pre-native Page admission: three
state slots (node/tree/binding), not two, plus the initial revision and bytes.
Activation transfers the existing binding lease to N-API without re-admission.
Real public pressure at 254/256 states now rejects without a native target or
metadata growth; exact-limit activation is separately unit-tested. This shares
existing caps and adds no ledger, task or larger capacity.
[Reproduction and accounting](knowledge/pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 admits native Frame wrappers until finalization within the existing
256-state/8 MiB ledger. Synchronous read guards share the 256 revision slots,
reserve descriptor/ID conversion storage before copy and survive actual N-API
conversion. No FfiBudget/HostSession is captured by retained native metadata.
Repeated public enumeration does not grow native charges; post-finalization
returns to the existing one-state/7672-byte empty host index, not zero RSS.
[Full accounting](knowledge/pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 installs the Frame index: 16 Page/128 Frame index slots, 256 retained
states, 256 revisions and 8 MiB logical storage at scope 121/2. Page metadata is
reserved before native creation. The fixed empty index is 7672 bytes/one state
on this Mac and survives session close until its owner drops. The selected
paused-OOPIF world/resume join uses two existing slots, after the four-command
prerequisite join; it adds no task or larger queue/deadline.
[Full accounting and native evidence](knowledge/pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-051 adds an explicitly constructed Frame-tree metadata ledger at scope
121/2: retained states, immutable revisions and storage bytes are separately
bounded through last-reference release. Retirement copying is admitted without
scratch allocation. Mixed URL/tree delivery shares the existing bounded queue;
no watcher or runtime owner is added. Host installation and FFI output-vector
admission are still pending. [Exact boundary](knowledge/pwright-frame-publication.md#resource-contract).

NODE-049 adds default-off runtime `lifetime-handoff`. Selected profiles admit
one additional BackendStateBytes lease per PendingCall in its existing Call
scope: the whole cancellation signal plus Arc counters (152 bytes on this Mac,
2432 bytes for sixteen pending calls). The lease survives until the last signal
reference, including retained tokens after command completion, is dropped.
Original pending/target/connection admission is unchanged. Token/provider storage
fits the existing Locator invocation's 16 KiB fixed allowance. No new registry,
task, event cursor or controller capacity is added.
[Accounting and exhaustion proof](knowledge/pwright-chromium-drag.md#resource-and-verification-boundary).

NODE-047 admits 16 retained Locator keyboard invocations at scope 122/10,
with 1104 KiB each (17.25 MiB logical allowance), while the existing Locator
factory admits eight executing actions. Exact Frame/Page route leases survive
cancellation. Controller slots are now 536/272; provider allowance is 86 MiB
plus fixed storage. [Accounting](knowledge/pwright-chromium-drag.md#node-047-resources-and-evidence).

NODE-046 extends selected drag admission for eight Keyboard cancellation owners:
528 dispatcher slots and 264 Page commands per controller. Pending/write CDP
limits remain 552/32. Started canceled keydown work remains owned by Page
quarantine until native retirement. [Details](knowledge/pwright-chromium-drag.md#node-046-selection-and-resource-boundary).

NODE-045 derives host Mouse admission from eight native operations and 65
children per maximum click: 520 selected dispatcher slots, 256 Page drag command
slots and 552 shared pending CDP commands. Write backpressure remains 32.
The enlarged logical wire/provider allowances are not eager allocation or RSS
claims. [Exact accounting and limits](knowledge/pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).

Latest Keyboard host budgets: 16 retained invocations at scope 120/18, each
with argument bytes plus 8192 call/native-binding bytes, alongside 4096 state
bytes and separate route leases. Scope 120/19 admits eight executing operations.
The original cancellation Arc retains every invocation lease. N-API separately
admits conversion/settlement before copying. Public 24-case verification ends
with zero host leases/operations/cleanup. Earlier pending budgets below refer
to the preceding primitive/adapter checkpoints.

Raw Keyboard adds a caller-polled press helper over at most 16 borrowed token
slices and the existing Page key/event state. No key-expression copy, delay task
or autonomous cleanup is introduced. Keyboard 1.0 now validates owned argument
capacity (128 key bytes / 1 MiB text) and bounded wire fields. The optional native
backend reserves factory-wide operation slots and argument capacity plus 8192
diagnostic bytes before returning a service future. Admission persists through
finish and releases on future drop; it does not account for caller-owned Page
bindings, transport buffers or retained replies. Pre-conversion host leases and
native retirement remain open; local budget release is not native retirement.
The optional wire recipe now adds backend-plus-route-plus-frame requirements.
Page/handler Arcs retain their route storage lease; native service handles cannot
escape through the route API. Admission revocation rejects new/unpolled commands
without prematurely releasing already-started operation ownership.
The opt-in controlled adapter retains that exact route through the existing
runtime's abandoned-cleanup Arc. Seven adapter tests verify target checks,
permit-delayed revocation, disposal, both timeout policies and cleanup-retained
route storage. Runtime scheduling resources are declared separately; host queued
arguments and retained native-client storage are not covered by this route lease.
An optional input-state factory now separately bounds retained Page states and
fixed policy-specific storage. Every coordinator clone and transaction retains
the state lease. Node host source selects 32 retained states for its 16-Page
topology at scope `extension(120,17)`; old/candidate states share this quota.
Legacy profiles remain opt-in and do not retroactively claim those state budgets.
[Raw Keyboard ownership and verification](knowledge/pwright-raw-keyboard.md).

The native reload primitive shares the existing navigation operation/byte budget
and owns one bounded event cursor. No replay queue or background task is added.
Dropping the raw future frees local admission, not proof of browser stop-loading.
Typed Reload now uses controlled cancellation via Page.stopLoading and Node's
existing navigation FFI allowance. Timeout resource release passes, but Page close
during a held response still fails native disappearance confirmation. That proof
is not weakened or replaced by CDP command acknowledgement.
[Reload resource contract and tests](knowledge/pwright-page-reload.md).

Sequential typing adds a borrowed input view with a 1 MiB UTF-8 length limit and
four-byte inline scalar iteration under the existing Page input gate. This is
not an owned-capacity or lifetime/RSS bound. The public path now separately
rejects String capacity over 1 MiB in codec/host/backend; the v2 codec checks
before cloning, the existing Locator lease covers retained input, and N-API
admits its existing 6 MiB action ticket before conversion under the 64 MiB
environment ceiling. Delay, deadline, queue admission and cleanup remain caller
owned, with no new task or clock in the input leaf. Normal dependency closure
remains five Pwright packages. [Contract and pending gates](knowledge/pwright-sequential-typing.md).

The opt-in interleaved Page policy now resolves the earlier delay/cleanup
contention without enlarging queues or timeouts. It owns one fixed 16-code key
set, coordinates and three-button mask per Page; repeated downs allocate no
slot and new codes beyond 16 fail before dispatch. Individual native events
remain serialized, but no gate survives a caller delay. Logical state changes
before dispatch and persists after timeout/failure; there is no command-local
release-all obligation. Runtime native retirement and Page/Context disposal
remain separate requirements. No CallId map, task, clock or Cargo edge is added;
existing action tickets/text leases are unchanged. This finite-state/count
bound is not a total-heap or RSS measurement. Default exclusive behavior remains.

The Node-selected recursive-attachment join requires four pending CDP slots and
adds `4 * max_response_bytes` beyond the three-way prerequisite profile: 16 MiB
of declared driver response-expansion capacity in this recipe, not a preallocated
buffer or RSS claim. Repeated/older opt-in selection cannot double-charge it.
Driver event limits, metadata ownership, dependency closure and connection
deadlines do not increase. [Contract](node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

New screenshot Worker tests exercise the existing four-result admitted-copy
budget at queued settlement and post-publication teardown. Both now pass in the
12/12 recursive-attachment artifact; the preceding trace-only artifact failed
before those assertions. This proves the checked lifecycle/copy behavior, not RSS.
Diagnostic session/target correlation retains at most 64 pairs of 128-byte IDs
only in an explicitly installed test subscriber; production adds a TRACE event,
not retained global state. The selected OOPIF utility-world test declares four
world slots for two Frames; its original main-only variant keeps two. Node's
production topology budget is unchanged. [Checkpoint](node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

Node Page screenshot selects a 2 MiB decoded-image ceiling, two active captures,
and eight admitted queue entries. Queue metadata reserves 4 KiB plus 4 KiB per
entry; nine scopes/leases cover retained state and queue metadata has its own
extension scope `(120, 16)`. One deadline spans queue wait
and capture. Each FFI screenshot reserves 4 MiB + 4096 bytes under the existing
64 MiB environment limit for successive native-to-host and host-to-Node copies.
The core lease ends before the next queued capture; the FFI ticket protects the
host output until Node settlement. Published Buffer lifetime belongs to the user,
not the native ledger. [Evidence and incomplete compatibility](node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

The Node Locator evaluation boundary now admits raw conversion callbacks under
the existing 8 MiB-per-evaluation / 64 MiB-environment FFI budget before taking
Node roots. Its environment-local registry and nonblocking Node-API notification
queue each have capacity 32; IDs are monotonic and never reused. Original JS
object graphs and exceptions are bounded in root count, not deep heap size.
Source/tree/binary conversion has the existing checked limits; no V8/RSS bound
is implied. Pending entries are removed on settlement/error and before native
environment teardown. Running browser-Promise cancellation now retires the exact
owned inspector session before cleanup (NODE-027); zero JS roots or a removed Context do not
prove native release. See [the current FFI contract](node-ffi-compatibility-design.md#public-locatorevaluate-ffi-checkpoint-2026-09-07).

The independent Locator leaf now optionally retains one caller-budgeted native
retirement receipt in its existing cleanup continuation. Its inline future fits
the existing 8192-byte owner allowance; the receipt's dynamic allocations remain
the composing caller's responsibility. The Node host now charges an additional
`size_of::<Retirement>() + 1024 + 64` bytes per admitted call for its boxed
projection and unique-ID copy. The existing bounded registry charges actual
receipt/weak-entry sizes including waiter/flags; it retains one snapshot permit
across native detach without another inventory vector. No task, channel, timer
or dependency is added. Native-wait retirement does not release the group owner
without separate evidence, and failed detach never creates a receipt. Non-Node
default APIs remain unchanged. See [the host checkpoint](node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

This document is the DT-060 allocation inventory, limit contract, eviction
policy, and internal memory-metrics schema. It covers memory retained by the
Pwright client/runtime process. Browser parent, renderer, GPU, and utility
process memory is measured separately; a Pwright allocation ledger cannot and
must not pretend to cap Chromium's own RSS.

## Rules

1. Every retained collection has a count limit. Every retained variable-size
   payload has both a local structural limit and a connection-wide byte budget.
2. A connection owns one explicitly constructed `ResourceLedger`. There is no
   process-global ledger and no hidden default budget.
3. `ResourceLimits::deny_all` begins with zero allowance for every class. The
   composition root enables only the classes installed capabilities require.
4. A reservation checks its global and per-scope limits atomically before the
   producer retains data. Multi-class reservations either all succeed or make
   no counter change.
5. A non-cloneable `ResourceLease` stays beside the retained state. Explicit
   release, normal return, cancellation, panic unwinding, and future drop all
   return its units through RAII.
6. Correctness state is never silently evicted. A component must apply the
   class policy below: backpressure, a declared lossy queue policy, spill, or a
   typed `ResourceExhausted` failure.
7. Browser acquisition and process ownership are unrelated to memory
   accounting. Creating a ledger never launches, downloads, bundles, closes,
   or kills a browser.

## Units and ownership scopes

Count classes use one unit per item. Byte classes use one unit per byte of
retained application data. Rust allocator and map-node overhead is not guessed;
the separate hard limits on objects, scopes, leases, queues, calls, and topology
records bound that metadata.

One ledger is connection-local and aggregates all participating components.
Each reservation is additionally attributed to one scope:

| Scope | Use |
| --- | --- |
| `Connection` | Handshake/control state and data with no narrower owner |
| `Object(ObjectId)` | Page, Frame, BrowserContext, request, response, or handle state |
| `Call(CallId)` | Command parameters/results and transient operation state |
| `Extension { namespace, id }` | Adapter/plugin state whose identity is intentionally not portable |

An extension namespace is assigned by the composition root. It is not a backend
protocol ID and never appears in the public Playwright-shaped API.

## Resource classes

The core ledger defines these stable accounting classes:

| Class | Unit | Intended owner | Release boundary |
| --- | ---: | --- | --- |
| `LiveObject` | item | runtime connection/object graph | child-first Dispose or transport close |
| `PendingCall` | item | call correlator/runtime | result, cancellation, deadline, send failure, or close |
| `ActiveCommand` | item | connection scheduler active permit | operation completion, cancelled admission, or future drop |
| `EventItem` | item | subscribed object/event route | dequeue, declared eviction, unsubscribe, or close |
| `EventBytes` | byte | queued typed event | with its event item |
| `WireBytes` | byte | encoded/incoming frame | final frame clone/decoder/transport release |
| `ResponseBodyBytes` | byte | response/call retaining a body | body consumer/dispose/stream cancellation |
| `LogBytes` | byte | opt-in log capability | dequeue, ring eviction, or capability stop |
| `TraceBytes` | byte | opt-in trace recorder | bounded chunk flush/spill or trace stop |
| `ArtifactBytes` | byte | optional Artifact source and Stream read result | explicit Stream/Artifact disposal or returned read-result drop |
| `ScreenshotBytes` | byte | screenshot call/artifact; retained value type is owned by command-free `pwright-screenshot-model` | result consumption or artifact dispose |
| `DomContentBytes` | byte | Frame DOM serialization result | returned budgeted UTF-8 value drop |
| `SelectorBytes` | byte | operation carrying a `pwright-selector-model` recipe or backend cache | operation end or bounded cache eviction |
| `RemoteHandle` | item | Page/Frame execution world | explicit handle dispose, world destruction, or page close |
| `BackendStateBytes` | byte | adapter extension scope | target/session detach or backend close |

`ResourceLimits` supplies a global and per-scope hard limit for every enabled
class, plus independent maximum counts for live scope records and active
leases. Zero is a valid limit and rejects the first reservation.

## Allocation inventory

| Retained state | Owner | Existing local bound | Shared-ledger class | Overflow/eviction policy | Implementation status |
| --- | --- | --- | --- | --- | --- |
| Runtime object graph | runtime connection | `RuntimeLimits.live_objects` | `LiveObject` in object scope | reject Create before graph/outbox mutation; parent Dispose is child-first | shared count implemented when injected |
| Client live-handle tokens and unclaimed announcements | client connection | `ClientLimits.live_objects` shared by announced, claimed, and disposed-before-claim entries | `LiveObject` | reject initializer; remote Result must claim exact prior Create; Dispose invalidates clones; late Result consumes its tombstone and fails | local bound implemented |
| Pending runtime calls | runtime dispatcher | `RuntimeLimits.pending_calls` | `PendingCall` in call scope | reject before scheduler/backend; completion/future drop releases; cancellation alone retains | shared count implemented when injected |
| Pending encoded client calls | client-wire channel | pending-call/tombstone limits | `PendingCall` in call scope | reject before state/send mutation; result, cancellation terminal result, send failure, or close releases | shared count implemented when injected |
| Target lanes and queued commands | target and connection schedulers | lane/waiter limits; one active target permit by default, explicit nonzero controlled-runtime concurrency optional | already retained by call-scoped `PendingCall`; no `ActiveCommand` while waiting | bounded FIFO admission, exact-call cancellation, or typed exhaustion | local bounds plus shared pending-call count implemented |
| Active backend commands | connection scheduler | `RuntimeLimits.active_commands` | `ActiveCommand` in call scope | reserve before grant; atomically transfer the lease on FIFO promotion; operation/cancellation/future drop releases | shared count implemented when injected |
| Typed event queues | client event extension | capacity plus declared maximum decoded bytes per event | `EventItem` + `EventBytes` in source-object scope | RejectNewest does not reserve; DropOldest atomically replaces the old lease; local oversize rejects before ledger mutation; dequeue/clear/drop releases | shared budget and production profile implemented |
| Lifecycle/control outbox | runtime control | explicit item capacity | `EventItem` in object/call scope | reserve the whole lifecycle batch before graph mutation; claim rollback retains, while pop/commit/close releases | shared count implemented when injected |
| Abandoned-operation continuation | injected cleanup executor | fixed queue or semaphore capacity; optional caller-selected nonzero Tokio job and drain deadlines; finite explicit retry attempt count | one non-cloneable reservation or active permit per admitted logical job; retry keeps the same permit | reserve before irreversible backend work; shutdown closes admission then drains accepted reservations/jobs; job deadline drops the opaque job so its own cancellation-safe guard escalates; drain timeout observes but does not cancel; only a caller factory can create retry attempts | standard-library caller-driven queue plus opt-in Tokio reserved execution, application drain, explicit retry factories, reporting events, and detailed metrics implemented |
| Owned-browser shutdown diagnostics | optional launch-composition report sink | one report per successfully terminated owned Browser; no portable result payload | provider-owned report moves synchronously after runtime disposal; default sink is zero-state discard | no queue, task, clone, or wire encoding in the composition; a sink needing I/O must perform its own bounded nonblocking admission | generic sink and discard default implemented; encoded unit-result and exact-PID/profile live proof pass |
| Prepared object disposal | controlled runtime | at most one active transaction; exact existing subtree and lifecycle-outbox capacity | reserved `EventItem` leases in object scope; no `WireBytes` | reject before backend work; Drop releases every reservation; commit transfers leases child-first; transport close invalidates the token | implemented and coordinated with Chromium topology close |
| Prepared object creation | controlled runtime | at most one active transaction; one object or a finite caller-supplied related-object batch with validated graph insertions and exact lifecycle-outbox slots | reserved `LiveObject` + optional `EventItem` in each new object scope | validate the full candidate before reserve; keep every object absent before commit; Drop releases; commit publishes the graph and ordered Creates together; transport close invalidates the token | implemented for single Context and Page+main-Frame publication |
| Graceful-close machines | unique client driver / server control decorator | one fixed-size phase value and counters per endpoint | `Connection`; no dynamic units | no queue or allocation; cancelled Ack wait retains only phase; preceding frames are validated and dropped one at a time | implemented |
| Tailnet endpoint authentication | caller-owned endpoint plus one caller-polled accept/connect future | one zeroized 32-byte PSK; 32-byte nonce; two 32-byte proof buffers; exactly 105 transcript bytes | fixed stack/endpoint state, no variable shared-ledger units | reject non-Tailscale addresses before I/O; reject invalid proof before frame transport; caller bounds concurrent futures and injects deadline | implemented |
| Mutual-TLS endpoint authentication | caller-owned endpoint; semaphore-bounded accepted handshakes; one caller-polled client connect | rustls cap: 65,535-byte handshake message and 65,536-byte certificate; explicit outbound TLS buffer limit `max(65,536, stream envelope + 8)`; compile-time pin count | third-party TLS state is documented separately; authenticated Pwright frames enter the existing `WireBytes` ledger | reject wildcard/config/ALPN/certificate/pin failures before frame transport; handshake permit releases on success/error/cancellation; caller injects deadline and PKI policy | implemented |
| Caller-driven Chromium endpoint server | deployment composition | one caller-selected pre-bound listener, stateless attach factory, and external CDP endpoint; exactly one accept future per call | endpoint-specific pre-authentication resources; each successful peer gets a fresh connection ledger | endpoint rejection precedes CDP; no loop, task, retry, admission policy, or deadline; runtime/application installers remain explicit per connection | Unix UID plus dedicated-Chrome live proof; Tailnet and mTLS static adapters implemented; operational admission/deadline policy remains caller-owned |
| Complete current-capability Chromium recipe | one accepted Chromium connection | one controlled Page runtime; one static command chain; one activated control receiver; lifecycle plus three typed-event outbound lanes; topology and PageLoad backend cursors; caller-sized cleanup queue; four supervisor/runtime command slots in the full profile | existing per-capability ledger claims; topology driver owns one fixed backend-state lease; PageLoad adds one selected event cursor and one retained future; each typed event owns its declared queue leases | reject any unselected or unnegotiated capability/control/event component; derive reclaiming subscriber capacity from selected plugins; keep supervisor ingress and runtime active-command limits equal; terminate on cursor lag, mapping failure, or event overflow; Page-close poll and cleanup draining remain injected | all eight current browser capabilities plus control and three Page event lanes compile as one concrete application; unit cursor proof, dedicated-Chrome assembly/reattach proof, and four-command navigation/Locator overlap proof pass |
| Typed result/command to encoded wire frame | protocol connection | per-payload, attachment, attachment-count, aggregate frame, and source representability limits | `WireBytes` | reserve source + maximum output before codec allocation; atomically shrink/move to exact route-owned frame; failure releases | shared budget and production protocol claim implemented |
| Outgoing stream envelope and pending write | Tokio stream transport | exact encoded and length-prefixed sizes | `WireBytes` in connection scope | reserve before each allocation; pending buffer owns lease across cancellation; production rejects a frame from another/no ledger | shared budget and production transport claim implemented |
| Incoming stream envelope | Tokio stream transport | outer declared length checked before allocation | `WireBytes` in connection scope | reject before body read; cancellation/parse failure releases through RAII | shared budget and production transport claim implemented |
| Decoded incoming frame | stream codec/transport | payload, attachment-count, attachment, and aggregate limits | `WireBytes` in route-derived scope | reserve before copying; no truncation; final frame clone owns lease | shared budget implemented when injected |
| Generic fragment assembly | transport utility | declared length checked before exact reserve | `WireBytes` in caller-selected scope | reserve full declaration before buffer allocation; reject declaration/overrun/incomplete stream; finished bytes retain lease | shared budget implemented when injected |
| Raw Chromium CDP connection | CDP WebSocket connection | pending/write/event counts plus command/response/event/error/WebSocket byte limits; event capacity plus one incoming allocation-before-admission item; optional fixed subscriber-slot bookkeeping | `WireBytes` + `EventItem` + `EventBytes` + optional `BackendStateBytes` in connection-selected scope | synchronous command admission; bounded serialization; pending/queue clones share leases; legacy v1 uses Tokio broadcast; opt-in v2 fanout reclaims after every current cursor advances, drops events with no cursor, and reports bounded overwrite as lag | shared budget, legacy compatibility path, task-free reclaiming path, derived composition preflight, and production connector implemented |
| Native CDP liveness | one attached Chromium composition | one cancellation-token clone and one boxed caller-polled future; no task, queue, Browser owner, or replacement connection | fixed backend state only; the concrete connection retains its existing resource leases | reader/writer termination makes the signal sticky; liveness backend returns terminal failure; supervisor cleanup drops sibling backend turns and shuts down the remote transport; old native sessions are never rebound | implemented independently of topology-events; fresh remote-client snapshot/event-view bootstrap pending |
| Reusable Chromium attach recipe | process/application configuration | immutable capability profile, `ResourceLimits`, and Browser root ID only; no retained ledger, connection, topology, ObjectId source, event receiver, runtime, or browser lease | none while stored; every `prepare` allocates a new connection-local `ProductionResourceLedger` | validate profile/limits/root namespace before I/O; preparation or attachment failure drops only fresh state; repeated calls never share ledgers or rebind Browser graphs | `ChromiumAttachFactory` and ledger-identity tests implemented; complete remote session unit pending |
| Fresh Chromium remote connection unit | one accepted remote connection | exactly one fresh CDP attachment, production remote transport/supervisor, runtime graph, and optional topology-event driver; no hidden task/listener/executor | every component must prove the fresh connection ledger; transport requirements must exactly equal the handshake profile | reject missing handshake before CDP; reject builder error, foreign ledger, plan drift, duplicate topology driver, installer that returns without installing runtime, unnegotiated recipe capability, missing discovery/control component, or an unconsumed topology driver; topology updates fence through their final lifecycle `Create` even without portable event queues; failed staging drops the new unit rather than exposing a partial session | consuming runtime/handshake/application stages, mandatory liveness wrapping, exact shutdown, pre-provisioned navigation execution, client-observed automatic discovery, packaged Unix-listener admission, and complete current-capability assembly have dedicated-Chrome proof |
| CDP context/target/session/frame/world maps | Chromium backend | explicit object/context/page/frame/world/native-ID limits; at most one owned async-spanning transaction | `BackendStateBytes` in one adapter extension scope | production mutation pre-reserves a maximum shadow; ordinary candidates commit exact bytes; BrowserContext close keeps the conservative shadow lease until the next mutation so post-CDP commit cannot fail; drop rolls back | shared budget, production profile, and owned close transaction implemented |
| Explicit Chromium BrowserContext close | composed lifecycle service + ordered runtime control | one topology transaction, one runtime transaction, and at most supervisor in-flight commands plus one outbound writer slot | topology `BackendStateBytes` + runtime child-first `EventItem`; command futures are already bounded by supervisor `PendingCall`; raw transports retain existing `WireBytes` | reject non-Owned before CDP; backend failure drops both preparations; success commits topology/runtime, sends through target Dispose, then sends Result; barriers precede ordinary queued writers | local, remote-stream, client-application, and abandonment ordering implemented |
| Explicit Chromium BrowserContext creation | composed creation service + ordered runtime control | one monotonic ID, one topology candidate, one runtime creation transaction, and one synchronously reserved continuation | topology `BackendStateBytes` + runtime `LiveObject`/`EventItem`; existing CDP/wire command budgets cover native and remote traffic | reject invalid target/metadata/capacity before CDP; burn failed ID; compensate post-CDP publication failure by disposing the exact native context; remote sends exact Create before Result | local, remote-stream, client-application, abandonment, compensation, and dedicated-Chrome proofs implemented |
| Explicit Chromium Page creation | composed creation service + ordered runtime control, optionally joined to the single-cursor network driver | two monotonic IDs, one topology candidate, one Page-route candidate, one two-object runtime creation batch, at most one provisional Page/session registration, bounded exact-target cleanup checks, and one synchronously reserved continuation | topology/routes/network-registry `BackendStateBytes` + two runtime `LiveObject`/`EventItem` leases; existing CDP/wire command budgets cover native and remote traffic | reject invalid Context/metadata/capacity before CDP; create `about:blank`, flatten-attach, require a bounded root frame; under inherited Context interest register and enable the exact session before publication, hold a racing provisional event, then commit the lane with runtime/topology/routes; on failure disable/retire before closing and confirming target absence; remote sends Page Create, Frame Create, then Result | local, remote-stream, client atomic-claim, abandonment, ordinary compensation, provisional-network success/publication-failure/cancellation recovery, and dedicated-Chrome proofs implemented |
| Explicit Chromium Page close | composed lifecycle service + ordered runtime control | one topology transaction, one runtime transaction, one exact Page-route replacement, bounded target-absence checks, and the existing ordered-writer limit | topology/routes `BackendStateBytes` + runtime child-first `EventItem`; each `Target.getTargets` response is bounded by the CDP response-byte limit and is not retained after a check | reject before CDP on invalid/lifecycle capacity; close exact target; commit only after exact absence; publish topology/routes together, then runtime Frame→Page Dispose, then Result | local, remote-stream, client-application, abandonment, and dedicated-Chrome proofs implemented |
| Chromium Page before-unload initiation | exact Page-session route | one command and existing CDP pending/write response bounds | existing route `BackendStateBytes`, CDP `WireBytes`, and supervisor/runtime `PendingCall`/`ActiveCommand` | send page-session `Page.close`; return without topology, route, or runtime disposal; later dialog/target lifecycle owns resolution | implemented with fake-CDP and owner-liveness proof |
| Abandoned lifecycle continuation | caller-owned reserved cleanup executor | one synchronously reserved slot per admitted create/close; queued jobs plus reservations never exceed executor capacity | existing future retains its already-accounted topology/runtime/wire owners; queue adds fixed bookkeeping only | reject before CDP if reservation fails; unpolled Drop releases; started pending Drop transfers the exact future without a second capacity check; job completion/failure is metered | implemented for BrowserContext creation/close, Page creation, and ordinary Page close |
| Attached Chromium topology/runtime owner | attached composition | one shared topology and at most one installed production runtime | topology/runtime use their already-declared exact ledger claims; the `Arc`/mutex are fixed attachment state | topology mutation requires a synchronous guard; runtime factory rejects a second owner; Page-runtime close factories reuse exact CDP/topology/route identities and require the continuation owner | implemented and covered by dedicated-Chrome Context/Page proofs |
| Portable Page to CDP session routes | Chromium composition | topology page/native-ID limits; four target/session slots plus two overlapping utility-context slots per Page | `BackendStateBytes` in a distinct composition extension scope | fixed six-slot maximum lease before retention; unchanged routes reuse services; utility-world replacement rebuilds only the affected locator route; detach/reconnect drops retired entries | shared budget and production profile implemented |
| Chromium utility-world bootstrap | optional topology-event infrastructure profile | one fixed 34-byte per-Page name, one browser-retained empty new-document registration per live target session, and sequential commands over at most the bounded frame tree | no separate Rust retention class; command/response bytes use the existing CDP `WireBytes` plan, world unique IDs use topology and Page-route `BackendStateBytes` | no task or queue; only a creation failure for a frame absent from a fresh authoritative tree is tolerated; target/session teardown owns Chromium-side registration disposal | implemented for Page and OOPIF sessions with fake-race and dedicated-Chrome isolation proof |
| Locator query/runtime source | locator capability/Chromium backend | depth, node, aggregate string, value-capacity, generated-script, and concurrent-action limits | `SelectorBytes` in composition-selected scope | production call reserves its complete transient peak synchronously before returning a future; completion/cancellation/unpolled future drop releases | shared budget and production profile implemented |
| Active Chromium `Page.goto` tracking | navigation backend action | URL/native-ID/inflight-request/header/status limits plus concurrent-operation bound | `BackendStateBytes` in composition-selected scope | production call reserves complete peak synchronously; reject before request insertion/header copy; completion/cancellation/drop releases | shared budget and production profile implemented |
| HTTP Request/Response lifecycle metadata | optional `pwright.network-events` model/contract, Chromium normalizer, single-cursor attachment driver, subscription registry, and one cross-event remote stream | active request count; Page-lane and BrowserContext counts; one fixed parent Context ID per Page driver lane; separate four-bit Page/Context masks; at most one in-flight Context plan containing bounded Page IDs, compact session IDs, and actions; at most one provisional Page transaction containing bounded session/target identity and fixed client/bookkeeping ownership; one unpublished-batch Page ID; two temporary runtime children per active request; compact in-flight Request/optional-Response metadata; bounded terminal snapshots and ordered queue items/bytes | pre-reserved `BackendStateBytes` for native correlation plus the maximum compact Request/Response snapshot of every active request, exact-session Page state and its parent Context ID, one replacement-only shadow adapter beyond the public Page-lane count, empty-Context masks, the maximum Context transition plan, and one maximum provisional Page transaction; `EventItem`/`EventBytes` count complete terminal snapshot capacity for selected queues; `LiveObject` and lifecycle `EventItem` cover only in-flight Request/Response children | register one connection cursor and exact Page adapters before enable; use direct/inherited union for native ownership; Page edges commit after exact-session CDP success; Context edges snapshot before I/O and commit only after the whole batch, reverse-compensate definite failure, and preserve cancelled enable as idempotent disable cleanup; explicit `newPage` and automatic new-Page discovery hold provisional-session events until atomic publication and reverse enable before target/session cleanup on failure; replace a discovered session through the pre-enabled shadow while preserving masks and disposing old active Request roots in the runtime rebind; Page/session detach and explicit Page/Context close hold network state across topology/routes/runtime publication, preserving adapters on failure and retiring them only after parent-subtree commit; Context close filters adapters by their retained parent identity without another Page-ID allocation or a subscription lock across publication; require exact Page acknowledgment after every normalized batch before another receive, replacement, or retirement; the owned driver turn combines receive, lifecycle/control/event publication, and acknowledgment; fan out Page then Context with shared identities; lifecycle Create is fenced before first exposure; terminal Request/optional-Response values are self-contained and one Request-root Dispose commits only after every selected queue accepts the batch; queue loss or retirement failure is connection-terminal; application install requires unique-driver consumption and the retained backend drops it on terminal connection notification; no completed history or body read | portable model, object kinds, complete terminal snapshot codec/client contract, production normalizer, bounded single-cursor driver/registry, transactional Page activation, resumable multi-session Context activation, explicit-newPage and automatic-discovery inheritance/rollback, atomic session replacement/rollback/recovery, atomic parent retirement, unpublished-batch barrier, owned publish turn/backend, explicit Page/Context close adapter retirement, Page/BrowserContext client helpers, empty Context retention and dual-scope publication, ordered codec/client path, all-capabilities application wiring, 4,096-iteration bounds, overflow proof, unit terminal cleanup, dedicated-Chrome install/drop/external-browser-survival proofs, a persistent-Page Chrome workload with stable `[4, 4, 4]` live objects, and two consecutive three-run RSS series implemented; cross-host series and reviewed budgets pending |
| Response bodies | future independent body capability | per-call result or chunk bytes and concurrent consumers | `ResponseBodyBytes` | reserve before allocation; bounded result or stream/backpressure; never read or retain all traffic implicitly | capability not yet implemented |
| Console and browser logs | future opt-in log capability | must declare item and byte limits | `LogBytes` | event-specific ring/drop policy with loss metrics | capability not yet implemented |
| Trace metadata chunks | optional BrowserContext tracing capability plus injected sink | session/chunk name/title bytes; group depth; entries per chunk; bytes per entry; total in-memory chunk bytes; exactly one detached chunk awaiting sink; preallocated registered-Context count | `TraceBytes` plus independent entry and Context count bounds | reserve before copy; group location and correlated call-ID entries reserve exact encoded size before one allocation; `stopChunk` atomically transfers or discards; a detached chunk gates the next chunk/session and drops payload leases/vector storage before reopening the gate; Context teardown releases recorder first and then requires sink cleanup; sink-cleanup failure cannot retain base state; wire control accepts no attachments; no core I/O/task | core-only bounded recorder, exact production profile, six-command 1.0 schema/capability, bounded binary codec, opt-in client facade, generated server chain, task-free shared recorder service, injected clock/sink, full typed duplex proof, optional navigation contributor, dedicated-Chrome `goto` proof, first DT-003 tracing workload, and bounded atomic file sink implemented; rich contributors, remote streaming, repeated series, and reviewed budgets pending |
| PWRTRACE encoding and atomic trace files | shared `pwright-trace-format` plus optional `pwright-trace-sink-file` leaf | encoded Artifact byte ceiling; encoded directory-path bytes; distinct Contexts; committed chunks per Context; one encoded file; total committed bytes per Context; exactly one synchronous staging file under the state lock | Artifact path reserves exact `ArtifactBytes` before allocation and returns that owned buffer; file path retains recorder `TraceBytes` only until synchronous handoff and accounts disk separately | one shared exact size/writer contract; Artifact oversize rejects before lease/allocation and makes no staging copy; file sink requires a fresh directory, admits before path creation, then create-new temporary file, stream, sync, and atomic rename; failed publication cleans staging best-effort | two format AAA tests prove exact budget/bytes and oversize pre-admission; six file-sink tests preserve exact tracing-service bytes and Context/chunk/aggregate/directory gates; producer command, directory recovery, archive/ZIP, serving, and remote delivery remain separate |
| Pullable trace chunks | optional `pwright-trace-sink-pull` leaf | fallibly preallocated global FIFO slots; queued chunks per Context; retained name/title/payload bytes per chunk and globally | original chunk-owned `TraceBytes` leases plus independent queue item/byte admission; peaks and accept/reject totals are sink-local | reject before queue mutation; queued chunk keeps recorder gate closed; `pop` releases queue admission but transfers the non-cloneable chunk and all leases to caller; Context/all abort drops only still-queued chunks; caller must bound and clean any post-pop async send | six AAA tests cover tracing-service handoff, ownership after pop, item/byte/per-Context rejection, label accounting, selective abort, and connection-wide abort; concrete remote transport, cancellation/retry, and delivery acknowledgment remain separate |
| Artifact byte stream | optional command-free model, `pwright.artifact` 1.0 contract/codec/server, opt-in client facade, task-free registry/service, and separate runtime lifecycle adapter | fixed preallocated Artifact and Stream tables; one immutable source shared by bounded Streams; one cursor/prepared read per Stream; source, read, failure, retained-result, Artifact-count, and Stream-count limits; one successful-result attachment; controlled-runtime live-object/control capacity and ordered-writer waiters | `ArtifactBytes` in source-object and result-call scopes for sources, failure results, and read results; production profile covers maximum sources plus retained-result pool and worst-case one-byte result scopes; a prebudgeted source may transfer the same owned lease/buffer into the Artifact; wire attachment remains separately owned until exact decode copy succeeds; runtime owns its existing `LiveObject` and control-message leases | reserve/copy result before cursor mutation; commit advances, prepared-read drop rolls back; explicit close rejects during preparation; Artifact/parent/connection teardown aborts child Streams first; registry Create stays unpublished until runtime commit; disposal holds and compares stable child-first plans; exact Create/Dispose is fenced before command success; failed Stream Create publication is compensated locally; codec requires index zero, one attachment, exact declared length, and explicit EOF including empty artifacts | stable handles/object vocabulary, eight model tests, frozen generated five-command schema, six codec tests, three client tests, ten registry/service tests including owned-source scope/no-copy and complete typed duplex pull, opt-in/absent manifest proofs, and five runtime transaction/order/failure/trace-ownership tests implemented; tracing producer command pending |
| DOM/ARIA snapshots, screencast, network bodies, HAR, sources, video | independent future trace contributors/artifact capabilities | each plugin declares its own acquisition, retained bytes, staging count, redaction, and sink limits | contributor-specific bytes; never borrowed implicitly from base trace budget | independently selectable; shared chunk sequence only after explicit composition; failure attribution remains per contributor | deliberately not bundled with base tracing |
| OS process-memory attribution | optional `pwright-process-metrics` diagnostic; optional `ledger` feature depends only on `pwright-core` | process-table snapshot bytes, process count, inspected command bytes, explicit stable runtime/browser root PIDs, total/phase observation counts, exact bounded series run count, and one typed owned-process exit proof | sampler-owned transient bytes plus one pre-reserved bounded observation vector per run and one pre-reserved bounded report vector per series; results correlate fixed-size OS counters, `ResourceMetrics`, and browser object counts and are not a `ResourceLedger` limit | read at most limit plus one sentinel byte; kill/reap oversized `ps`; discard command text after role classification; reject malformed/duplicate/cyclic PID graphs, overflow, phase/time/root changes, incomplete runs, teardown deadline violations, series plan mismatch, unclean logical state, or measured teardown above the stable warm process count; owned teardown requires the same snapshot to contain the runtime and omit the exact Browser PID and never fabricates zero browser RSS; reusable variable byte payload requires an explicit per-class plan while owner/lease/scope baselines remain strict | default standard-library-only crate and CLI, opt-in caller-driven external/owned recorders and bounded series, AAA tests, dedicated-Chrome idle/detach, Page/Context, retained-Handle, first tracing run, one three-run Playwright-shaped `goto` series, two consecutive three-run network-lifecycle series, and one three-run real owned-browser teardown series implemented; other repeated/cross-host series, Linux PSS, and budget thresholds remain DT-003 work |
| Page screenshots | optional `pwright.content` 1.0 | decoded-byte ceiling, one active capture per Page, configured connection concurrency, one attachment | `ScreenshotBytes` for backend/client result; `WireBytes` for frame copy | reserve maximum before decode, reject encoded oversize, shrink to exact retained bytes; drop releases admission | production Chromium adapter and remote/client transfer implemented |
| Locator screenshots | optional `pwright.locator_screenshot` 1.0 | selector depth/node/string ceilings, bounded Frame topology and native-ID length, one active capture per Page, configured selector/capture concurrency, decoded-byte ceiling, one attachment | `SelectorBytes` during resolution; `ScreenshotBytes` for backend/client result; `BackendStateBytes` for independent Frame routes and shared owner-translation chain; `WireBytes` for frame copy | acquire capture before resolution; drop releases selector/capture admission; scroll then re-resolve geometry; translate each ancestor via `DOM.getFrameOwner` plus native border box; shared attachment helper validates metadata before exact retained copy | independent schema/capability/codec/client/server, Page/main/local-child/nested-OOPIF routing, negotiated remote transfer, dedicated-Chrome main and same-process child proofs implemented; cross-site OOPIF live and decoded pixel fidelity remain conformance work |
| Frame DOM content | optional `pwright.dom_content` 1.1 | UTF-8 byte ceiling, one active read per Frame by default with explicit nonzero concurrency opt-in, configured shared connection concurrency, one attachment, shared utility-route count/native-ID ceiling | `DomContentBytes` for backend/client result; `WireBytes` for frame copy; one `BackendStateBytes` route reservation shared with locator | reserve maximum before native read/copy, shrink to exact retained text; no navigation retry; drop releases both Frame and shared admission; exact Frame route and utility world only | capability, codec, client/server, Chromium adapter, direct/remote composition, shared route, and dedicated-Chrome proof implemented |
| PDF and other artifacts | future content/artifact capabilities | command and artifact byte limits | `ScreenshotBytes` or a reviewed plugin class | fail before retaining oversized result; no truncation | capability not yet implemented |
| JS/element handles | future DOM capability | per-world/page and connection counts | `RemoteHandle` | explicit release; world/page teardown releases descendants | capability not yet implemented |

The independent Locator-screenshot capture composition owns only the
capture/result/wire portion of the table entry. Its injected
selector-to-geometry plugin owns and claims `SelectorBytes` plus any persistent
route state; the application must combine both profiles concurrently before
constructing the shared production ledger. The complete Chromium aggregate
continues to supply and budget the full-Locator resolver for compatibility.

“Not yet implemented” means the feature is absent, not that it may retain data
without a limit. Its future capability must reserve the listed class before it
can be advertised by a production composition.

## Implemented shared ledger

`pwright-core::ResourceLedger` contains only standard-library synchronization
and collections. Cloning the ledger shares one connection budget; cloning does
not create capacity. It exposes:

- atomic `reserve` and `reserve_many` operations;
- global and per-scope checks with checked arithmetic;
- independent scope-record and lease-count caps;
- `ResourceLease::try_resize` for bounded stream growth without release/re-add
  races;
- `ResourceLease::try_replace` for atomic lossy-queue replacement without
  temporarily oversubscribing or discarding the old admitted value on failure;
- current, peak, and rejected counters per class, plus live/peak scopes and
  active/peak leases;
- non-cloneable leases and non-cloneable `BudgetedBytes` for immutable retained
  byte payloads.

Malformed requests such as zero units or duplicate classes are programming
errors and do not consume scope metadata. Admission failures increment the
aggregate rejection counter; class-specific capacity failures also increment
that class's rejection counter.

Production composition has a separate, zero-dependency validation layer. A
`ResourceRequirements` value declares the minimum global and per-scope
capacity for every class a component may reserve, plus the minimum scope and
lease metadata capacity. `merge` takes the maximum for mutually exclusive
requirements or two views of the same peak. `combine_concurrent` saturating-adds
every dimension for independently live components. A composition root must use
the operation matching actual lifetime overlap and must never silently lower a
component declaration.

`ProductionResourceLedger::new` checks that the merged declared plan fits the
configured `ResourceLimits`; `from_pristine_ledger` additionally rejects a
ledger with current or historical use. Components then call `claim` with their
own requirements. A claim returns another handle to that same ledger only when
the component was declared. Failures distinguish an insufficient configured
limit from an omitted component in the declared production plan, so a remote
composition cannot accidentally turn an unbudgeted constructor into its
production path. Constructing or claiming this token has no browser, provider,
transport, executor, or process side effect.

`pwright-composition-chromium::PreparedChromiumAttach` now uses additive
concurrent composition for the raw CDP connection, production topology,
page-session routes, runtime, each selected backend service profile, and each
selected wire protocol. Navigation-only composition does not advertise locator
or include `SelectorBytes`; selecting locator against a plan that omitted its
profile fails before any endpoint is available. The root also rejects
non-extension service scopes, duplicate component scopes, an empty capability
set, and a runtime object bound smaller than the topology it must represent.
The page-route profile reserves six bounded native-ID slots per Page, covering
its stable route key pair, the pair owned by `CdpSession`, and old/new
utility-context values during locator-route replacement; unchanged routes
share their service instances instead of duplicating them. Navigation
and locator factories claim their profiles once and share profile-local
concurrency counters across every Page, so an overprovisioned common ledger
cannot bypass a selected capability's local limit. The server supervisor now
declares one global and one call-scoped `PendingCall` unit per retained command
future. This is intentionally additive with the runtime's own pending-call
lease: the supervisor owns the outer handler/send future while the runtime owns
correlation, cancellation, and cleanup state. Duplicate active call IDs fail
the per-call bound before another future is retained, and clearing active work
releases every supervisor lease.

The optional Tokio/Chromium adapter computes stream transport retention from
the codec before any socket exists. An authenticated accepted connection can
enter the Chromium root only after its production transport claims that exact
declaration from the attachment's fresh ledger; the existing root check then
rejects plan drift or foreign-ledger identity. Pairing the transport with its
endpoint-derived authorization gate adds no queue, lease, task, or second
ledger. Runtime and handshake stages move that same ownership forward.

`ChromiumAttachFactory` does not clone this ledger-bearing preparation. It
stores only immutable recipe inputs, validates them at construction, and calls
`PreparedChromiumAttach::new` for every preparation. Two calls therefore have
different `ResourceLedger` identities even with identical limits and plans.
The Browser root ID can repeat because every resulting remote connection owns
an independent wire namespace; its first child source and topology epoch are
also created anew only after native attachment.

Remote server resources remain optional. Selecting
`ChromiumRemoteHandshakeProfile` adds the caller-supplied production transport
requirements and twice the maximum admitted peer receive-frame size for
server-hello source/output overlap. A larger untrusted ClientHello offer is
rejected before authorization or response encoding; the emitted ServerHello
is tagged with the same ledger. Selecting `ChromiumRemoteControlProfile` adds
one control encode plus one object-scoped `EventItem` lease per subscription.
The two profiles must share the same peer frame ceiling. Without these options,
an in-process Chromium attach does not pay their byte or subscription costs.
Remote navigation and locator handler protocols use the accepted directional
limits, while preparation proves those limits cannot exceed or contradict each
selected capability's declared wire profile.

Subscription activation is a two-phase resource transition. Enable reserves
its `EventItem` lease before awaiting the backend but does not publish the route
until success; failure drops the reservation. Disable keeps both route and
lease while awaiting deactivation and releases them only after success. The
receiver stores that one in-flight edge, so cancellation cannot erase an
uncertain backend side effect or its resource owner. Another control is rejected
until cleanup resolves it.

The supervisor first polls the ordered control. Only handlers that certify the
input frame was released before returning Pending allow a subsequent receive;
the supervisor may then retain exactly one budgeted next frame, never dispatch
it early, and stops receiving again. This preserves the transport's one decoded-
frame bound instead of assuming arbitrary handlers dropped their input. EOF
cancels the control and transfers its retained in-flight lease to close
compensation. No unbounded control or prefetch queue exists.

Object disposal walks matching routes directly in the bounded map, awaiting
and releasing one successful edge at a time; failure retains the current and
remaining leases for retry. Every explicit terminal supervisor path signals
runtime closure and then removes one route before each best-effort backend
disable. It attempts all routes, reports cleanup failures, and leaves zero
subscription leases because retry is impossible after peer ownership ends.
Neither path copies the subscription set or allocates a second cleanup queue.
The async close GAT reaches one completed attempt before the supervisor returns
or drops command futures; receive, protocol, command, control, observer, outbound,
capacity, and resource failures therefore have the same release boundary as
clean EOF. A dropped run future releases Rust-owned leases synchronously but
requires a later owner-supplied async close call for backend deactivation. If
that future interrupts an in-progress disable, the current subscription route
and lease remain retained, and the supervisor leaves the barrier incomplete so
the next owner call can retry it.

Cross-component topology mutation uses a prepare/commit protocol rather than
three independent fallible updates. The production topology candidate owns its
shadow-state lease and rolls back on drop. Page routes build a bounded candidate
under their route lock. The runtime lifecycle batch simulates the complete
graph, reserves the full outbox and all new `LiveObject` leases, and validates
pending-call cancellation before replacing state. The composition publishes
route and topology candidates only after runtime admission succeeds, leaving no
fallible step after the first live mutation. Failed admission therefore retains
the previous topology bytes, object leases, lifecycle queue, and route table.

`pwright-transport::WireFrame::new_budgeted` reserves its aggregate payload and
attachment length as `WireBytes`. A budgeted `WireProtocol` first reserves
twice the maximum structurally attainable frame size in connection scope before
calling its codec. Production codecs must bound variable-size typed source data
by that same frame ceiling, so this covers source and output buffers
simultaneously. After encoding, `WireFrame::new_reserved_up_to` validates the
parts, atomically shrinks the reservation to exact retained bytes, and moves it
to `FrameRoute::resource_scope`: Command/Result use the call scope, Event uses
the source-object scope, and Handshake/Control use the connection scope.
`WireFrame::clone` shares one immutable storage allocation and the same lease,
so cloning a queued frame does not duplicate its byte buffer or accounting. The
final clone returns the budget. Codec and frame failures release the staging
reservation without retaining output.

`BoundedBytesAssembler::new_budgeted` gives non-Tokio and noncanonical-stream
adapters the same rule. It validates the local declaration, reserves the full
declared `WireBytes`, and only then allocates the fragment buffer. Successful
`finish` transfers both the exact bytes and lease into non-cloneable
`AssembledBytes`; incomplete input, construction failure, or assembler/output
drop releases it. The unbudgeted constructor remains for isolated adapters.

For inbound byte streams, `TokioStreamTransport::with_resource_ledger` reserves
the declared envelope in connection scope before allocating or reading its
body. `StreamFrameCodec::decode_budgeted` validates the complete header and all
declared lengths, then obtains an opaque `WireFrameReservation` before copying
payload or attachments. The envelope and retained frame reservations overlap
during the copy because both allocations really exist then; this makes the
reported peak honest instead of disguising the copy as a zero-cost transfer.
After decode, the envelope lease is released and only the route-attributed
frame lease remains. Cancellation while reading, allocation failure, malformed
input, decode failure, and reservation-size mismatch all drop their exclusive
leases without a cleanup gap.

The same Tokio injection covers outbound staging. `encoded_len` validates and
computes the exact envelope size without allocating; the transport reserves it
before encoding. It then reserves the length-prefixed pending-write buffer
before copying. Both leases overlap only while both buffers exist, after which
the encoded envelope and its lease are explicitly dropped. The pending-write
lease stays inside `WriterState` across cancellation, so a later send can finish
the partial frame without creating unaccounted retained memory. Successful
flush or terminal write failure drops that buffer and returns its budget.

`ShutdownFrameTransport` allocates no second frame or shutdown queue. Tokio
shutdown takes the existing writer lock, finishes the one already-accounted
pending buffer, flushes it, and closes the write half. The unique client driver
uses one already-bounded priority send-gate waiter, so only the active writer
can precede shutdown and queued command writers cannot follow it. Cancelling
shutdown retains only the same pending-write state and lease needed by the next
retry. Server shutdown likewise runs after its bounded supervisor work and
control cleanup; preserving two close errors adds fixed result state, not a
retained history.

The optional in-memory transport has a constructor-fixed frame count per
direction. It retains no encoded byte copy: queued `WireFrame` values keep their
existing immutable storage and resource leases. Each direction adds one
`VecDeque` bounded by that count, two closure flags, and at most one replaceable
receive waker. An unpolled send retains its caller-owned frame only inside the
future and never enters the queue. Full queues reject the new frame; receiver
drop clears all queued frames and releases their leases; sender shutdown lets
the peer drain existing frames and then returns EOF. This adapter is not marked
`ProductionFrameTransport`: a future production installation must separately
declare its queue-item metadata rather than treating already-accounted frame
bytes as complete queue accounting.

`TypedEventHandler::new_with_resource_ledger` makes decoded event retention a
separate opt-in composition. A capability supplies a type-specific size
function for variable decoded payload data; every admitted entry reserves
`EventItem` and, for nonempty payloads, `EventBytes` as one transaction in its
source-object scope. Reliable `RejectNewest` overflow returns the unqueued event
without consuming ledger capacity. Lossy `DropOldest` atomically replaces the
old entry's lease, including when the new event belongs to another object, so a
failed replacement preserves the old queue entry. Dequeue, explicit `clear`,
handler destruction, and conversion back into the protocol all release queued
leases. The codec remains responsible for bounding transient decode work; the
event lease measures data retained by the queue after decoding.

`TypedEventProductionProfile` multiplies the queue capacity by its declared
maximum decoded bytes per event, while reserving one multi-class lease and up
to one source scope per slot. `new_with_production_resources` must claim that
complete `EventItem`/`EventBytes`/metadata profile before construction. The
runtime sizing callback is checked against the declared per-event maximum
before touching the ledger, so spare capacity contributed by another event
plugin cannot let this handler exceed its own contract. A zero-byte event still
owns its `EventItem`; zero queue capacity claims and retains nothing.

Event sequencing adds no history buffer or source-indexed map. Each installed
typed server channel retains one `Option<EventSequence>`, and each client
handler retains one last-received scalar plus fixed counters. The eight-byte
sequence travels inside the already bounded wire payload and is covered by
`WireBytes`; the queued wrapper is covered by the existing fixed `EventItem`
slot. Server assignment happens before logical or resource admission, so a
failed admission consumes a sequence but no extra retained allocation. Client
observation happens before local queue admission; a locally rejected event is
therefore counted as received and its returned value remains the explicit
evidence of local loss. Connection-epoch reset clears queued items and their
leases before accepting a new baseline.

`CdpTopologyProductionProfile` derives a native-string storage ceiling from
the topology's context/page/frame/world counts and per-string byte limit. The
formula counts every retained duplicate across forward/reverse indexes,
pending or materialized records, session keys and owner values, crash status,
and execution-world keys. Production topology inputs are compacted before
retention so a short caller string with large spare capacity cannot bypass the
bound; the live lease measures actual retained `String::capacity()`, not an
estimate. Fixed records and map nodes remain bounded by the independent count
limits described above. The composition root must also assign an explicit
`ResourceScope::Extension`; generic connection/object/call scopes are rejected,
so separately installed adapters cannot silently collide in one scope.

Each production mutation first reserves one maximum-size `BackendStateBytes`
shadow lease, then clones and evaluates the pure state transition. A topology
or resource failure drops the shadow and leaves both live topology and its
exact lease unchanged. Success shrinks the shadow lease to the new exact byte
count, swaps it with the live state, and releases the old state and lease. The
production requirement consequently covers at most two topology copies, one
extension scope, and two leases. This deliberate bounded overlap makes atomic
failure semantics and the transient clone peak visible instead of hiding it.

`CdpLocatorService` itself has no variable-size retained service state: its
client handle, retry duration, one `OnceCell<()>`, and atomic counters are fixed
size. Locator memory instead belongs to each active action. A
`CdpLocatorProductionProfile` multiplies an explicit concurrent-action bound by
a conservative per-action `SelectorBytes` peak covering selector/value input
capacity, formatted selector labels, generated click/fill/query JavaScript,
selector runtime installation, a bounded text-content result, and intermediate
string-allocation overlap. Depth, node, aggregate string, fill-value, and
text-content limits constrain the derivation.

Production `call` validates logical size and spare string capacity, obtains a
clone-shared local action slot, and reserves the full byte peak synchronously
before returning the future. This means an unpolled future is already charged;
rejection captures neither command data nor browser work. Completion,
cancellation, or future drop releases bytes before returning the local slot.
The local counter prevents this locator plugin from borrowing another plugin's
unused global ledger allowance. Raw constructors remain explicit unbudgeted
test/compatibility paths.

The command-free selector compiler/runtime is now a separate leaf with the
same depth, node, aggregate string, retained-capacity, generated-script, and
runtime-install bounds used by the complete Locator backend. The first-party
selector-geometry plugin adds its own `CdpSelectorGeometryProductionProfile`.
It reserves one conservative `SelectorBytes` peak before an async resolution
can retain selector labels, geometry and scroll scripts, or installation
source. Its clone-shared local gate prevents borrowing unrelated ledger
headroom. Optional Frame utility-world routes are separate persistent
`BackendStateBytes` leases, bounded by route count and native-ID bytes and
released only after the last service clone drops. No resolved DOM or protocol
handle survives an evaluation attempt.

Frame locator routing has a separate persistent `BackendStateBytes` boundary.
`with_context_routes` derives its route count and native-ID ceiling from the
selected topology profile. Each route may retain its target ID, session ID,
selected-world `uniqueContextId`, and the target-local CDP client's copies; the
reservation doubles that five-string bound so current and replacement tables
can overlap during atomic synchronization. The route lease is acquired before
any Frame strings can be retained. Route count, every ID, and actual compacted
string capacities are checked before commit. Page routes therefore use only
their four-string target/session bound, and Page locator calls reuse the main
Frame route instead of retaining another utility-context copy.

Locator 1.3's negative-text, relative `has`/`hasNot`, and visibility filters
remain inside the same selector depth, node-count, aggregate string, generated
JavaScript, and transient-overlap bounds. They add no retained DOM handle,
server object, cache entry, route, or per-filter resource class. The semantic
runtime is versioned and installed once per execution context; extending it
cannot silently reuse an older runtime lacking the new visibility primitive.

Locator 1.4's regex source and canonical flags are charged to the same
aggregate selector-string and retained-capacity limits on construction,
encoding, decoding, and backend admission. Regex objects exist only during one
browser-side selector evaluation; no compiled expression, match result, or DOM
identity is retained across retries. Runtime v3 prevents a context containing
the older string-only runtime from being reused. Invalid ECMAScript source
terminates as typed `InvalidSelector` rather than entering an unbounded retry.

Locator 1.5's placeholder, alt, title, and test-ID values remain inside the
same per-selector node, aggregate string-byte, retained-capacity, escaped
JavaScript, and transient-overlap bounds. `TestIdAttributes` additionally caps
one immutable comma-separated name list at 16 entries and rejects empty or
unsafe names before encoding or backend admission. Expanding those names into
a temporary JSON array is covered by the existing per-node generated-script
allowance. Runtime v5 retains neither configured names, compiled regexes, nor
DOM matches after one evaluation.

Locator 1.6 `count` uses the same selector depth, node, string-capacity,
generated-script, concurrent-operation, and `SelectorBytes` admission bounds.
It evaluates one non-strict snapshot and copies only a checked `u32`; the
temporary browser-side match array and DOM nodes are not retained. A replaced
runtime permits exactly one reinstall plus one further evaluation, preventing
navigation recovery from becoming an unbounded loop. Snapshot evaluations are
reported separately from actionability evaluations.

Locator 1.7 visibility shares those same bounded snapshot resources and copies
only one boolean. It retains no node, geometry, style, or hidden-state cache.
The derived `is_hidden` future owns only the in-flight `is_visible` future and
negates its result; it creates no additional wire frame or browser operation.
Runtime replacement recovery remains bounded to one reinstall and one second
evaluation.

Locator 1.8 checked/editable/enabled queries reuse the same selector-storage,
generated-script, concurrency, and `SelectorBytes` admission peak. Each
evaluation copies one boolean or one bounded typed error and retains no DOM
handle, state history, or result cache. Missing and detached targets may retry,
but only inside the runtime-owned call deadline; every attempt increments the
separate `state_evaluations` metric and holds the same single operation lease.
The derived `is_disabled` future retains only one in-flight `is_enabled`
future and creates no second wire frame or browser evaluation.

Locator 1.9 `wait_for` reuses that same selector storage, generated-script,
concurrency, `SelectorBytes`, and one-operation-lease boundary. Each attempt
copies only the state outcome or a bounded typed error; it retains no DOM node,
ElementHandle, geometry, style, or match history. Unmet attached/visible and
detached/hidden conditions retry only within the runtime-owned caller
deadline, and every attempt increments the separate `wait_evaluations` metric.
The default visible state is represented by a small enum value rather than a
second command or backend allocation.

Locator 1.10 `text_content` uses the same single operation lease and retry
deadline while adding an explicit 1 MiB UTF-8 output ceiling to the per-action
`SelectorBytes` peak. The backend moves the validated JSON string out of the
CDP response, compacts spare capacity, and returns `Option<String>`; null stays
null and exact whitespace is preserved. Strictness, invalid selectors,
malformed non-string values, and oversized results are terminal. Each attempt
increments `content_evaluations`, and no DOM node, ElementHandle, result
history, or cache survives the call.

Locator 1.12 `inner_html` is a third mutually exclusive content-result branch.
It moves and compacts at most 1 MiB of browser-serialized UTF-8, reusing the
same operation lease, retry deadline, selector storage, and
`content_evaluations` counter. The production peak remains the maximum of raw
text, rendered text, and serialized markup—not their sum—and the call retains
no node, markup history, or cache after completion.

Locator 1.13 `get_attribute` adds a mutually exclusive 64 KiB attribute-name
input branch and a nullable 1 MiB attribute-result branch. The input peak is
`max(fill value, attribute name)`, the generated-script peak is
`max(fill script, attribute script)`, and the copied-result peak is the maximum
of text content, inner text, inner HTML, and attribute value—not their sum.
The backend rejects an oversized name before CDP, compacts a present result,
and records retries in `attribute_evaluations`; it retains no element identity,
attribute history, or cache after completion.

Locator 1.14 `input_value` adds no variable-size command input. Its non-null
UTF-8 result is capped at 1 MiB, moved out of the CDP response, compacted, and
accounted as another mutually exclusive locator-result branch rather than
being summed with text, markup, or attribute output. Retries increment the
independent `input_value_evaluations` counter. The shared runtime retargets an
associated label at evaluation time and no DOM identity, value history, or
cache survives the call.

Locator 1.15 `all_text_contents` adds the first retained collection result.
The wire and backend reject more than 4096 strings, any string above 1 MiB, or
more than 4 MiB of aggregate UTF-8. The production peak adds that aggregate to
the simultaneous `Vec<Value>` and `Vec<String>` container slots used while
validated JSON strings are moved and compacted. This collection branch is the
maximum against scalar locator results, not their sum. It is an immediate
non-strict snapshot, increments `all_text_contents_evaluations`, permits only
one runtime-reinstall retry, and retains neither the match array nor DOM nodes
after completion.

Locator 1.16 `all_inner_texts` uses the same collection ceilings and transient
`Vec<Value>` plus compacted `Vec<String>` shape, so it does not increase the
per-call maximum. A call can return either raw text contents or rendered inner
texts, never both; production therefore takes the maximum of the two equal
collection peaks rather than their sum. Its independent
`all_inner_texts_evaluations` counter distinguishes layout-sensitive reads,
while the immediate snapshot, one runtime-reinstall retry, and handle-free
completion keep the lifetime boundary identical.

Locator 1.17 `hover` adds no variable-size result and no DOM-handle lifetime.
Its command owns one already-bounded selector, fixed scalar flags, an optional
fixed-size modifier bitset with at most five members, and an optional fixed-size
pair of finite `f64` coordinates. Modifier presence itself is retained because
omission and explicit empty have different Page-key semantics.

Locator 1.18 `focus` and `blur` add no variable-size result, input transaction,
or DOM-handle lifetime. Each owns one already-bounded selector and one compact
evaluation script, shares the existing Locator action admission and selector
byte lease, and returns only a fixed status record. The two operations are
mutually exclusive call branches, so production resource requirements retain
the existing maximum instead of summing focus and blur peaks.

Locator 1.19/2.2 Press adds one
128-byte key expression, at most 16 stack-resolved key descriptors, and one
fixed 16-slot pressed-key record per selected Page. Events remain sequential;
delay shares the deadline scheduler and navigation waiting shares the existing
Page event cursor. The existing mutually exclusive Fill input peak is larger
than the Press expression, so the selector-action reservation does not grow.

Each selected Page owns exactly one fixed-size input coordinator, bounded by
the existing `max_pages` topology limit and shared by all of that Page's main,
local-frame, and OOPIF Locator routes. It contains one mutex, two finite pointer
coordinates, at most one pressed mouse button, a pressed-modifier bitset, one
fixed 16-slot physical-key record, and
at most one pending-restore bitset; it owns no queue, task, map, DOM handle, or
selector. One transaction holds the gate across key transitions and the pointer
action. Completion restores immediately. Controlled runtime cancellation
releases a pressed button and restores a transaction whose action future was
dropped; if a backend future is dropped outside that control path, the next
transaction performs the same bounded repair before accepting new input. Trial
may dispatch temporary modifier key events but terminates before mouse dispatch.

The existing Click command adds no wire or allocation peak at this checkpoint.
It now uses the same fixed Page state to emit mouse move, press, and release.
The pressed-button record is updated only after successful CDP acknowledgement;
a failed release stays pending for the cleanup retry. Cleanup failure remains a
typed terminal error instead of pretending that browser input state is known.

The extensible-record prototype bounds forward-compatible option bags before
reading any unknown data. Every schema must publish an unchanged maximum for
the complete record body; the decoder checks that `u32` length before taking
the slice. Field iteration is zero-copy and retains only a borrowed reader
cursor plus the previous `u16` tag. Skipping an unknown field allocates
nothing. Encoding
uses one output `Vec` and backpatches record and field lengths in place, so it
does not temporarily duplicate the option bag. Per-field string, byte,
repeated-item, nesting, and message-node limits still apply inside known field
bodies. Locator 2.0 now assigns Click options a 128-byte maximum record body;
its empty default occupies only the four-byte record-length prefix and the
complete current field set stays under the declared bound. The generated wire
model changes no locator 1.x production budget. Locator 2.x maps the public
option bag without an extra retained copy and caps `steps` at 10,000. The CDP
adapter emits those mouse moves sequentially from fixed Page input state, so
the count bounds work but does not multiply retained memory. Its navigation
barrier adds one fixed selected event cursor per active waiting Click and shares the
connection's already-budgeted immutable event payloads; it creates no second
reader or event queue. Locator 2.x can now enter a production profile only
through the explicit `with_v2_track` opt-in. The advertised support range is
built from a separate v2 inventory, and the accepted handshake installs only
the selected major's exact codec and handler tree. Enabling the track adds no
backend-state, queue, wire-byte, or retained-allocation requirement; disabling
it restores the v1-only manifest with identical resource requirements.

Locator 2.1 Focus and Blur reuse the same selector, call, retry, and utility-
world evaluation budgets already declared for locator 1.18. They add no input
transaction, retained DOM handle, queue, or second mutually exclusive peak.
The exact selected 2.x minor is a fixed codec value, not a per-call allocation.

Locator 2.2 Press reuses the same Page input owner and selector-action lease as
1.19. Its key record is fixed-size, every acknowledged down/up mutates only
that record, and failure or controlled cancellation releases remaining keys in
reverse order before later input can acquire the Page gate.

Locator 1.20/2.3 SetChecked adds one boolean and a bounded 64-byte extensible
option record to the already-bounded selector command. Pre-state,
actionability, and post-state results are fixed records; Check and Uncheck are
client composition and add no routes. The real action uses the existing Page
mouse transaction and selector-action lease. It retains no DOM handle, result
string, queue, map, task, or navigation cursor, so production takes the
existing Click maximum rather than summing another concurrent branch.

Locator 1.21/2.4 SelectOption bounds both request and result to 1,024 items,
64 KiB per string, and 1 MiB aggregate string bytes. The request retains only
descriptors and the generated script for one mutually exclusive locator
attempt; the production profile reserves their larger maximum rather than
summing them with Click, Press, or SetChecked. Each retry returns a fixed
status until every descriptor has an enabled match. Successful mutation
returns bounded selected values in DOM order and retains no element handle,
queue, map, pointer state, or navigation cursor. ElementHandle selection is
not hidden in this budget: it remains outside this slice pending a separately
negotiated cross-world Handle adoption contract.

Locator 1.22/2.5 ScrollIntoViewIfNeeded reuses the existing locator operation,
selector, utility-world installation, and action-script branches. Its request
adds only one already-bounded selector and its result is unit. The asynchronous
precommit script is dropped before the synchronous commit script is built, so
only one script branch is retained at a time. Every retry returns a fixed
status after releasing renderer-local references; no DOM
handle, point, observer, input transaction, navigation cursor, queue, map, or
task survives the evaluation. The generated script is below the existing
focus-script bound, which is already below the SelectOption-dominated action
maximum, so this mutually exclusive command adds no resource class and does
not increase the declared production peak. Cancellation before the scroll
commit is stateless; cancellation after commit cannot undo the scroll and must
never replay it.

The generated 22-command locator handler tables are split into two balanced
typed groups.
This is a build-resource boundary rather than a runtime heap allowance: it
prevents dual-major compilation from expanding a pathological left-nested
error/event type. It does not add a runtime collection or retain the unselected
major's handler tree.

The generated action script remains below the existing maximum action-script
branch, and each attempt uses the same locator operation lease and caller
deadline. Retries increment `hover_evaluations`; completion retains neither the
element, point, nor platform marker.

Client `Locator.all()` does not allocate a vector proportional to the match
count. `LocatorList` retains one moved base Selector plus a `u32` snapshot
length; each `get` or iterator step creates only the requested `nth` Locator.
This keeps hidden retention constant in N while making any caller-chosen
collection of yielded Locators explicit. The Count command still owns its
normal bounded call/selector state. A count above `i32::MAX`, which the current
portable nth AST cannot encode, fails before list construction. Client
`clear()` adds no state at all beyond the existing empty-value Fill call.

`CdpScreenshotProductionProfile` declares the decoded-byte maximum and maximum
simultaneous captures. Its shared factory allows one active capture per Page,
enforces the connection-wide count with atomics, and accumulates no waiting
requests. Admission reserves a full per-call `ScreenshotBytes` lease before CDP
response decoding. The exact padded-base64 decoded bound is checked first; an
oversized response fails without allocating the decoded vector. Successful
decode moves the vector into `BudgetedBytes::from_reserved`, which shrinks the
existing lease without a release/reacquire gap. Completion, decoding failure,
and dropping a pending capture release both Page/global admission and unused
bytes.

Remote encoding retains the backend `ScreenshotData` while separately
accounting the attachment copy as `WireBytes`. The client codec validates one
and only one attachment, its declared index and byte length, and its own
configured ceiling before copying it into a call-scoped `ScreenshotBytes`
lease. Dropping the client result releases that lease. The CDP base64 string is
bounded by the connection response limit and is not misreported as decoded
artifact retention.

Locator 1.11 `inner_text` shares that operation lease, retry deadline, and
`content_evaluations` counter, but its result is a non-null rendered string.
The backend moves and compacts at most 1 MiB of UTF-8 after proving the target
is an HTML element. Because a call can return either text-content or inner-text
but never both, the production content-result peak is the maximum of the two
1 MiB branches rather than their sum; no DOM handle, layout object, result
history, or cache survives the call.

`CdpNavigationLimits` bounds every variable-size value retained by an active
Chromium goto action: input and committed URLs, frame/loader/request identifiers,
simultaneous request IDs, response-header count and aggregate name/value bytes,
and status/error text. `CdpNavigationProductionProfile` multiplies a
conservative per-operation `BackendStateBytes` peak by an explicit concurrent
operation bound and requires a composition-selected resource scope.

Production `call` acquires a clone-shared local slot and reserves the full peak
synchronously before returning its future. Thus an unpolled future is charged,
and unused capacity in the shared ledger cannot let this plugin exceed its own
concurrency contract. Event processing checks request identifiers and the
inflight count before mutation. Response parsing checks header count, JSON value
type, and aggregate bytes before creating the owned vector. Retained strings
are copied into compact allocations. Completion, cancellation, and both
unpolled and polled future drop release the resource lease before returning the
local slot.

This profile ends when the backend future yields its portable result. Retention
by application code after an in-process return is caller-owned. Remote response
encoding and pending protocol transport writes have separate owners and carry
their own leases. The selected CDP event delivery path is separately bounded as
described below.

Legacy/v1 callers retain Tokio's broadcast receiver API and its bounded replay
ring. A v2 production profile may instead select the task-free
`ReclaimingEventFanout`. It preallocates bounded event and subscriber slots,
retains no event published without a subscriber, and removes a slot as soon as
every subscriber that existed at publication advances beyond it. A slow cursor
that crosses the event bound receives an explicit lag count; connection close
allows queued items to drain before returning `Closed`. Dropping the final
cursor reclaims every unread event.

The composition profile derives the required subscriber capacity from selected
plugins and their declared concurrency: topology and PageLoad each add one,
network adds its maximum Page lanes, and navigation plus Locator add their
maximum active operation counts. An explicitly smaller fanout fails production
validation before CDP I/O. Legacy delivery remains valid for preserved v1 and
non-opt-in profiles.

The raw CDP layer has independent hard limits for pending commands, write and
event channel items, serialized command/response/event/error bytes, and the
WebSocket frame/message ceiling. Pending admission is synchronous and
clone-shared. Its RAII registration removes an unpolled or cancelled command;
the shutdown failsafe drains the table even when the reader future is dropped.
Serialization writes into a bounded buffer rather than allocating an
arbitrarily large command and checking afterward.

`CdpConnectionProductionProfile` accounts the reader's maximum raw-message and
parsed-JSON overlap as a fixed `WireBytes` lease. Every admitted command owns a
full command/result peak shared by its pending entry and outgoing queue item;
the lease therefore survives caller cancellation while the writer still owns
the bytes. Each unique incoming event owns exact `EventItem` and `EventBytes`
units. `CdpEvent` clones share its immutable payload and lease, eliminating
subscriber-count deep copies. The profile reserves event capacity plus one
maximum event because the incoming value is allocated before either delivery
strategy performs bounded admission. Reclaiming mode additionally reserves one
fixed `BackendStateBytes` lease for event/subscriber bookkeeping. Release tests
prove that delivered values keep the shared payload lease alive and that it
returns to zero after all cursors and values drop. Event oversize or budget
exhaustion terminates the connection instead of silently omitting protocol
evidence. The production attach connector rejects an omitted declaration,
config/profile mismatch, or undersized subscriber plan before network access.
Fanout metrics expose both current and peak subscriber counts. The complete
profile derives a safe bound of 10, while the canonical shared-network recipe's
real Chrome overlap proof observed 6 or 7 cursors at peak and returned to three
steady cursors, zero queued events, and zero lag.

`DirectDispatcher::with_resource_ledger` and
`ControlledDispatcher::with_resource_ledger` install the ledger only while the
runtime is pristine. Each successful Create reserves one `LiveObject` in the
new object's scope after graph/outbox validation and before graph mutation.
Adopt changes only parent ownership and keeps the same object-scoped lease.
Child-first Dispose and transport-close drain remove the exact corresponding
leases; a rejected Create leaves both the graph and lifecycle outbox unchanged.

When the lifecycle outbox is enabled, Create, Adopt, and every child-first
Dispose message also own an `EventItem`. A multi-message Dispose reserves the
entire ordered batch before changing the graph, so exhaustion cannot leave a
half-disposed tree. Merely claiming the next message retains its lease; rolling
the claim back therefore keeps both message and capacity, while pop, claim
commit, queue clear, and transport close release the exact front lease.

`PreparedObjectDisposal` reserves that same complete child-first batch without
yet changing the object graph. Its single active transaction token excludes
competing lifecycle mutations while an owner awaits backend cleanup. Drop
returns all reserved leases; commit transfers them to the outbox and removes
the objects; transport close drains the runtime and invalidates the token. No
wire payload is allocated by preparation. This bounds the otherwise dangerous
gap between successful native cleanup and local lifecycle publication, but the
composition layer also serializes its native-ID topology before invoking the
backend.

For remote close, `OrderedRuntimeControl` bounds its active and waiting writers
to the supervisor's declared in-flight commands plus the singleton outbound
lane. These are not a second command queue: each waiting barrier remains inside
an already-accounted supervisor command future, and the one ordinary sender is
the fixed outbound future. The priority gate is allocation-bounded at
construction. It is acquired before CDP work, excludes ordinary claims, and
drains already-budgeted outbox entries through the exact context Dispose.
Dropping a waiting/acquired barrier removes or releases its slot and wakes the
next barrier before ordinary output. Result encoding and its existing
`WireBytes` reservation begin only after that control sequence finishes.

An explicit close also reserves one slot from an injected
`ReservedCleanupExecutor` before constructing browser work. The reservation is
counted together with queued cleanup jobs, so direct submission cannot steal
capacity from an admitted close. Normal completion or unpolled Drop releases
the slot. If a polled future is abandoned while pending, consuming the
reservation moves that exact boxed future into the queue without rechecking
capacity; the existing topology, runtime, CDP, and wire leases move with it.
The queue counts accepted, completed, and failed continuations. It does not
spawn, poll, retry, or choose shutdown policy for the owner.

The same runtime injection reserves `PendingCall` in call scope before inserting
the call table, cancellation signal, or scheduler admission. The lease covers
target/connection queueing, backend execution, cancellation wait, and an
abandoned operation. Completion and future drop release it. Parent or transport
close deliberately retains the call lease after releasing affected object
leases, because the cancelled call remains owned until its cleanup path reaches
a terminal state.

Only a connection-wide execution permit reserves `ActiveCommand`; a target head
waiting for that permit remains solely a `PendingCall`. Immediate admission
reserves before scheduler state changes. Releasing an active permit atomically
moves its lease to the oldest connection waiter and changes the call scope
without a release/re-reserve window, so current usage and peak usage remain
truthful at capacity. Cancellation, failed admission, completion, and dropping
either an unpolled granted acquisition or its permit release the exact lease.

`FrameLimits::max_frame_bytes` derives the largest structurally attainable
frame body rather than assuming every independent limit can be reached at
once. `WireProtocol::production_resource_requirements` declares twice that
capacity for the typed-source/encoded-output overlap, and
`with_production_resources` refuses to encode unless that class was present in
the plan. Every budgeted
`WireFrame` records the exact ledger identity, including a zero-byte frame;
equal numeric limits from another ledger do not satisfy that proof.

`StreamFrameLimits::retention_bounds` solves the attainable envelope/body
maximum while attachment headers grow and remaining body room shrinks. Its
outbound peak includes the input frame, encoded envelope, and length-prefixed
pending write. Its inbound peak includes the envelope and decoded frame; the
full-duplex requirement is the sum because one reader and one writer may run
concurrently. `TokioStreamTransport::with_production_resources` claims that
complete byte/scope/lease profile. Production send then rejects any outbound
frame not budgeted by the very same ledger before encoding or allocating
staging memory. Extra concurrent send attempts still contend against the hard
ledger rather than gaining hidden capacity.

`EncodedClientChannel::new_with_resource_ledger` and its control-enabled and
post-handshake variants apply the same `PendingCall` class on the client side.
Registration reserves before correlation, pending-map, send-gate, or transport
mutation. A normally delivered result releases the count when correlation ends,
even if its separately wire-budgeted frame awaits application consumption. If a
possibly sent future is dropped, the lease moves with the bounded tombstone;
sending a cancellation is not terminal and does not release it early. The late
result, send/receive failure, EOF, explicit close, or receive-driver drop clears
the exact remaining leases. Unbudgeted constructors remain available for
isolated and compatibility compositions.

The production client path additionally requires a `ProductionFrameTransport`.
That small codec-neutral trait exposes the transport's retained-resource
profile and exact connection ledger; Tokio is only one possible implementation.
`ClientWireProductionProfile` combines it with the bounded pending-call
backlog. Its wire-byte maximum covers both attainable peaks: full-duplex I/O
plus the other pending slots' frames, and a full pending table plus the next
frame encoded before its registration is rejected. Its metadata bound includes
pending leases, frame leases, and transport staging leases. Direct,
control-enabled, and post-handshake production constructors claim this complete
profile and reject a production transport from another ledger.

The server's outbound lifecycle adapter uses the same budgeted control protocol
and transport handle as its inbound receiver. The supervisor retains at most
one outbound attempt and rotates it with receive and active command lanes. An
idle result is parked on the plugin's readiness future, whose default never
wakes; receive or command progress also retries it. An empty runtime outbox
therefore cannot consume CPU in a ready-loop. A successful send
commits the runtime's existing bounded outbox claim; encoding or transport
failure leaves that claim in place and terminates the supervisor distinctly.

When negotiated graceful close is installed, the Chromium control profile adds
one further protocol-encode peak. The ordinary lifecycle outbound future may
already retain its maximum frame while the ordered inbound Close barrier must
encode CloseAck, so counting only one peak would make a valid shutdown depend
on scheduler timing. Both protocols claim the same connection ledger; actual
frames reserve and release exact bytes. The reserved `pwright.control`
handshake entry also consumes one explicitly bounded capability slot and fails
composition before I/O when that slot or its ID-byte bound is unavailable.

`pwright-server-events` supplies the matching typed event-output boundary. One
production profile adds a bounded `EventItem`/`EventBytes` queue to one maximum
protocol encode, while the separately declared transport covers its own
retained I/O state. A claimed front event remains logically inside capacity
during an outbound attempt. New producers therefore cannot evict it under
`DropOldest`; a failed or cancelled attempt restores it without a second
enqueue count for ownership accounting. Such an attempt is still terminal for
that connection because the transport may own a partial write. A successful
send commits the claim and releases the decoded payload lease while the
budgeted `WireFrame` continues to own its exact `WireBytes` until the transport
releases its final clone. Chromium remote event
profiles are optional and additive, and their accepted-handshake factory
rejects a production transport that does not share the composition ledger.
Each typed queue stores at most one outbound waker; accepted enqueue takes and
wakes it, while static chains race only readiness futures and keep actual sends
serialized.

The concrete `PageLoad` event adds no variable decoded payload bytes: its
portable payload is zero-sized and its canonical encoded payload is exactly 17
bytes (kind, sequence, and duplicated source Page identity). The
`BrowserContextPage` and `PagePopup` payloads each retain one `ObjectId` and
encode to 25 bytes by appending the related Page identity. Their client queues
use `RejectNewest`; capacity, `EventItem`, overlapping encode `WireBytes`, and
transport staging remain declared by the existing per-event production
profiles. CDP lifecycle activation owns no retained queue or background task
and reuses the exact synchronized Page route and CDP client retained by
`CdpPageServices`. Topology-derived subscriptions activate no CDP stream, and
their explicit publisher owns no queue beyond the independently installed
typed senders.

The topology connection pump retains only its driver, two optional typed
sender handles, a nonzero reconstruction limit, counters, and terminal state;
it adds no collection, queue, task, timer, or transport reader. Each successful
same-connection recovery consumes one bounded reconstruction attempt and one
strictly increasing `ConnectionEpoch`; epoch exhaustion is terminal rather
than wrapping. Portable discovery lanes prohibit replay and convert CDP loss,
publication overflow, driver failure, or a dropped polled turn into a terminal
connection state. Supervisor termination records `BackendEventFailed`, runs
the reasoned control cleanup once, and attempts transport shutdown even if
cleanup fails.

The server backend lane adds no event queue and no concurrency multiplier. Its
Chromium adapter retains at most one boxed pump future, moves the single pump
into that future, and restores it before reporting the outcome. Receive,
commands, outbound delivery, and backend work are polled with rotating
priority, but only the existing bounded command vector can contain more than
one operation. Dropping another lane's ready future cannot drop the backend
turn. Dropping the entire caller-owned connection run remains terminal; its
backend termination hook drops that one retained turn synchronously and marks
the adapter closed. No browser work is orphaned in a task.
Leaving the Page domain enabled after the final listener is intentional shared-
domain safety; the lifecycle-event toggle itself is disabled.

Fresh remote assembly does not introduce another budget or another owner. The
transport builder must prove both the declared transport plan and the identity
of the attachment's fresh ledger before a supervisor exists. Runtime install,
handshake acceptance, and application install are consuming stages; an error
drops the attachment, supervisor, runtime, authorization, topology driver, and
all their leases together. The application context cannot be independently
constructed and exposes the supervisor's exact shared transport. A selected
topology driver must leave that context exactly once. The root adds the
task-free CDP liveness lane around every caller backend without adding a queue
or capacity multiplier, and the final complete run invokes shutdown on that
same transport after terminal cleanup.

The original unbudgeted constructors remain for isolated codecs, tests, and v1
compatibility. `RuntimeLimits::resource_requirements` now declares its exact
`LiveObject`, `PendingCall`, realizable `ActiveCommand`, lifecycle `EventItem`,
scope, and lease minima. `DirectDispatcher::with_production_resources` and
`ControlledDispatcher::with_production_resources` claim that declaration and
reject an omitted or undersized runtime plan before state exists. The common
wire protocol and Tokio stream transport now enforce the same production claim
and exact-ledger identity. The encoded client now composes through the same
proof. Typed event queues, Chromium topology, selected navigation/locator
services, the server supervisor, handshake, and control subscriptions enforce
their own production profiles as well. A complete deployed remote server must
still add its concrete transport requirements to the optional handshake
component; each installed typed event output now contributes its own queue and
encode profile. Preserving v1 does not authorize an unbounded v2 server profile.

## Eviction matrix

| Data category | May drop? | Required behavior at limit |
| --- | --- | --- |
| Object lifecycle, command result, cancellation | No | backpressure/reject before mutation or terminate an invalid connection |
| Pending calls and active commands | No | reject admission with `ResourceExhausted` |
| Page close/crash and protocol diagnostics | No | reserve protected capacity or close the affected connection |
| High-rate telemetry/event samples | Yes, by declared event class | RejectNewest or DropOldest and increment loss metrics |
| Response body requested by user | No silent truncation | bounded streaming or `ResourceExhausted` |
| Logs | Only when configured lossy | bounded ring with dropped item/byte metrics |
| Trace/HAR/video | Not silently | bounded asynchronous spill; stop recorder on sustained pressure |
| Screenshots/PDF | No truncation | fail the command before retaining the result |
| Selector/query/cache | Cache entries only | reject oversized query; LRU/epoch-evict bounded cache entries |
| Remote JS/element handles | No implicit live-handle eviction | explicit release or parent/world disposal |

## Metrics schema

Each connection exposes a `ResourceMetrics` snapshot. For every class it
contains:

```text
global_limit
per_scope_limit
current
peak
rejected
```

The snapshot also contains `live_scopes`, `peak_scopes`, `active_leases`,
`peak_leases`, and `rejected_reservations`. Component-local metrics remain
necessary: queue drop counts, object counts, pending calls, scheduler waiters,
topology records, selector-runtime installations, and cleanup failures explain
which subsystem owns the shared units.

`scope_metrics(scope)` returns current units and active leases for one known
connection, object, call, or extension scope without cloning or exporting the
whole bounded scope table. Diagnostics can therefore attribute pressure to a
specific Page/call/plugin while global metrics remain allocation-free.

OS measurements must report the Pwright client/runtime process separately from
the browser parent, renderer, GPU, and utility processes. Ledger values explain
retained logical data; RSS/allocator measurements capture allocator overhead,
code, stacks, and fragmentation. Neither number substitutes for the other.

## Optional Node FFI boundary

The experimental Node host has an additional environment-owned production
ledger, separate from each attached Chromium connection's ledger. It owns
`PendingCall` and `BackendStateBytes` in one explicit extension scope. Its
fixed profile admits at most 16 connect, 32 command, 16 close and 16 session-signal
settlements, with 64 MiB of reserved native payload/future bytes and 160
lease records. There is no process-global registry or budget.

Admission precedes copying borrowed JS strings and scheduling native work.
Per-call payload allowances are 32 KiB for control, 1 MiB + 4 KiB for DOM reads,
256 KiB for navigation, 6 MiB for Locator reads, and 8 MiB for by-value evaluation. Actual boxed-future body bytes are also reserved,
with lane-specific ceilings of 1 MiB / 128 KiB / 16 KiB / 16 KiB. Independent
close slots and the combined byte ceilings preserve control headroom when
command slots are full. Bounded backend data and the FFI-owned copy keep
separate leases during their overlap.

Success and error outcomes retain their ticket through JS conversion, not
merely through Rust completion. The bounded registry lets environment teardown
cancel/join execution and reclaim queued native values without running JS.
Foreign callbacks can then retain only empty ticket cells. A failed argument
check, future-size reservation, conversion, or dropped ticket releases admission.
The `pwrightFfiMetrics()` extension exposes environment-wide running, queued,
pending, byte and per-lane counters independently from live-host counters.
Its `queuedCommands` counter excludes queued notifications/control results from
command-pressure measurements; `queued` still counts all settlement lanes.

Unit tests cover reclamation with retained foreign tickets and release after
conversion/admission failure. Real-Node tests block the JS loop until 32 outcomes
are queued, exercise close under pressure and terminate a Worker before result
delivery. These bounds cover the current async exports, not future event/Buffer
APIs, all user-owned V8 values, historic Frame metadata or process RSS. Full
profile and evidence are in [the Node FFI design](node-ffi-compatibility-design.md).

URL publication additionally uses the per-host metadata ledger, not the FFI
result allowance. Its 128 record, 16 KiB per URL and 2 MiB aggregate limits cover
native, published and queued values together; shared `Arc` values are charged
once. At most 256 pending `EventItem` leases represent ordered URL changes.
The profile permits 769 lease records for entries, URL allocations, event items
and replacement overlap. Overflow rejects before metadata mutation and the host
terminates on observer failure rather than losing a navigation silently.

One already-budgeted session-signal wait carries a fixed sequence watermark;
only Node callbacks publish through it. Command/error/close settlement uses the
same ordered boundary, so stale callbacks cannot regress URL state. The
`pendingPublications` metadata metric counts changes awaiting publication.
Frame-local `pwrightMetadataMetrics().unpublishedUrl` identifies a native URL
revision not yet published on that exact Frame, without inspecting another Page.
Environment teardown discards that queue and disables further enqueueing,
including updates arriving while native shutdown drains. URL snapshots and
publication endpoints retain no native transport/runtime. General event payloads
and subscriptions still require their own admission and ordering contracts.

The attachment diagnostic's command timeline is not a production queue. A
test-only tracing subscriber admits at most 32 pending records and 128 recent
completed records, with method/session strings limited to 128 bytes each.
Overflow is explicit; no response body, console content or script is copied.
The existing production command trace includes an opaque session ID but installs
no subscriber, collector, task or Node dependency. The independent Rust probe
retains the same five-second attachment budget and bounds cleanup separately.
Auxiliary Node diagnostics cap process/target inventories and subprocess logs;
they do not define browser RSS limits or weaken any conformance deadline.

The Node host explicitly selects three-way session prerequisites: Frame Tree,
Runtime enable and empty utility-world script registration on the existing
caller-driven task. The production profile rejects pending-command capacity
below three and declares three `4 * max_response_bytes` expansions, adding
32 MiB over the serial Node profile's reserved backend ceiling (16 MiB over
the earlier paired-only option, not actual allocation or RSS). Default v2
consumers stay serial with the original declaration; the paired-only option
remains available and unchanged. No new worker, subscriber, timer,
unbounded queue or cross-session concurrency is installed. All requests use
existing command admission; cancellation/error drops pending registrations and
already queued bytes keep their independent transport leases until released.
Repeated option selection is idempotent. World creation still awaits all three
replies; Context defaults remain earlier, recursive attachment/resume later.

The Node profile separately opts into eight controlled-runtime target permits
and eight DOM reads per Frame (NODE-020). This does not increase the existing
eight connection-active / sixteen pending-call bounds, shared DOM-read byte
ceiling, or 64 MiB FFI settlement budget. Target-admitted calls waiting for the
connection still own only pending-call admission. Serial consumers keep an
inline active identity; only the opt-in scheduler retains a bounded active-ID
set. DOM Frame and shared counters roll back on rejection and drop. No task,
timer, worker, object identity alias, or capability is added by this selection.
Input operations retain their separate Page-owned transaction admission.

Node evaluation additionally bounds source at 32 KiB, depth at 64, nodes/entries
at 4,096, and aggregate strings/binary payloads at 256 KiB each. The callback
converts only after FFI admission; callback-scoped JS handles never cross a Rust
await. Ordinary object/array structure is snapshotted synchronously. Binary
leaves retain fixed byte-range views in one non-Send environment registry, then
an injected Node `setImmediate` callback materializes owned bytes before Rust
execution. No borrowed byte pointer or Node reference enters a Rust future.
The native pointer is reread at dispatch; detachment rejects, an out-of-bounds
resizable view becomes empty, and no JS call occurs during the byte copy.

Prepared calls retain their existing 8 MiB FFI admission. In addition to the
256 KiB binary payload cap, each evaluation admits at most 1 MiB of aggregate
backing capacity, counting every captured buffer's full `maxByteLength`, even
for a tiny subarray. Aliased backings are conservatively charged per view.
The registry has at most 32 pending entries, further constrained by the shared
64 MiB allowance. One native callback per environment owns only a weak registry
and call IDs. The Node timers scheduler and ArrayBuffer capacity getter are
captured at package import; pre-import replacement of intrinsics is not a
security boundary this package defends.

`pwrightConversionMetrics()` exposes environment-wide `pending`,
`binaryReferences` and `backingBytes` for the prepared phase separately from
queued native outcomes and active browser operations. Getter/conversion failure
drops every captured reference. FFI `running` includes admitted prepared calls;
it is not a count of browser operations or proof that Rust has started execution.
Environment cleanup clears the prepared registry
and its three intrinsic/callback references before joining Rust's FFI tracker,
so unstarted task tokens cannot deadlock teardown. Output uses fresh Node-owned
buffers while its FFI ticket remains charged. Worker tests cover prepared,
running and queued evaluation states and require exact native Context/Page
removal. The copy-timing comparisons now match; NODE-013 still tracks intermittent
attachment timeouts that can fail setup before Worker-state assertions run.

The explicit Node evaluation backend admits eight operations with its existing
peak/route reservation model, a 2 MiB generated-program/wire ceiling and 4 KiB
error fields. This requires the Node connection's CDP command ceiling to rise
from 64 KiB to 4 MiB; response and WebSocket ceilings remain 4 MiB. Derived
connection/backend requirements, rather than uncharged extra buffers, cover
the change. V8 allocations for property enumeration/construction, user-created
JS values and overall process/browser RSS are outside these Rust byte metrics.

### Native Locator values and reads

Each Node environment explicitly owns a separate immutable-query budget:
1,024 recipes/copies and 8 MiB of declared storage. Each recipe admits at most
64 KiB of aggregate strings, 256 selector nodes and depth 32. Admission occurs
before Node string conversion, AST cloning or composition. The declaration
covers selector node storage, fixed owner metadata and twice the input string
capacity for the UTF-8 conversion/copy overlap. Abandoned drafts, failed
construction and native value finalization release their reservations.

Locators retain a weak Rust host identity and portable Frame ID, not a DOM
handle, native session or cached match. Cross-connection/Frame composition
rejects before copying a query. Browser close need not destroy retained pure
query values; `pwrightLocatorMetrics()` reports their environment-wide recipe
and byte counts separately from active-session metrics. JavaScript wrappers
retain their parent wrappers; this is not a claim about V8 heap or process RSS.

A read reserves a separate command-owned selector copy until the operation
finishes. The existing Locator backend independently admits eight operations
and its selector/program/result resources. Every read uses a conservative
6 MiB FFI payload allowance through Node conversion or environment reclamation,
even when the result is a boolean or count; the shared 64 MiB cap further
limits concurrent settlements. The Node profile selects Locator 1.x with
1 MiB codec strings, 2 MiB command and 6 MiB result frames. It remains an
explicit broad reference profile, not a minimal distribution package.

GC tests force finalization only in the test child (`--expose-gc`); production
does not trigger GC. Separate count/byte pressure cases require no browser
commands before rejection. Worker teardown must cancel a pending query and
remove its exact owned native Context/Page without a JS callback. This does
not yet prove arbitrary query/operation concurrency or lower memory usage.

Native Locator actions use that same independent command-query admission and
6 MiB FFI settlement allowance. Borrowed Node strings/objects are validated on
the callback thread before conversion: Fill at 1 MiB UTF-8, Press at 128 bytes,
and selection at 1,024 descriptors with 64 KiB per string / 1 MiB total.
The allowance includes overlapping UTF-16/UTF-8 conversion and owned input/result
vectors; unpaired UTF-16 rejects explicitly. Safe Rust host entry points also
validate retained capacities. No Node reference crosses an await, and no new
runtime task, queue, browser resource or capability dependency is installed.

Fill now acquires the existing Page input transaction for focus/selection and
the final insertion. It releases the gate before a retry delay or selector
runtime reinstall. Empty Fill uses the existing tracked keyboard path for
Delete; key-up failure triggers repair and both Locator majors route cancellation
to shared input cleanup. This reuses the existing input state/lease peak and does
not serialize unrelated evaluation or Locator reads behind a missing fill target.
The Node deadline comparison intentionally still detects Playwright's retained
Shift versus Rust's released Shift (NODE-022); this resource guarantee is not
evidence of complete compatibility.

## Remaining DT-060 verification

Context creation/disposal now use that same bounded Node host driver and the
existing typed Context services. Each native Context is owned with
`disposeOnDetach: true`; tests resolve its exact ID and Page target before
disconnect or Worker termination and require both native records to disappear.
Default-Context close selects attachment shutdown without holding an operation
token across its own drain. The optional session-initializer port is awaited
by the existing driver before resume/readiness and adds no task, queue or event
subscriber. Its Node profile stores only one connection reference and fixed
window settings; fixed typed CDP requests use existing command/byte limits.
Download artifacts, headless fonts and general Context options need additional
explicit initialization/resource contracts before being exposed as compatible.

Command completion does not enqueue another topology fence after authoritative
`TargetClosed`, or after the host is already disconnected. Ten simultaneous
Page-close cancellations previously overflowed the eight-entry fence queue and
overwrote one closed result with ResourceExhausted (NODE-023). The shared Rust
completion policy preserves that terminal result without another allocation or
queue entry. Success, timeout, browser exceptions and other nonterminal errors
retain publication; live fence exhaustion still propagates. This does not expand
the queue or implement execution-world readiness waiting (NODE-021).

### Semantic Locator values

The independent temporary-element resolver declares one native group/Handle,
2,368 owner bytes and a source-derived selector scratch peak per capacity unit.
It keeps two leases during a query and one while the selected node is retained.
Its separate composition holds one pre-admitted caller cleanup reservation
through resolution and release. Failed or abandoned group cleanup terminally
closes admission across resolver clones; reclaiming local leases after failure
does not establish remote cleanup. Neither leaf is currently installed by the
Node host or ordinary Locator reads. See the exact formulas and cancellation
proofs in [Locator evaluation](knowledge/pwright-locator-evaluation.md).

The optional strict waiter adds a 256-byte installed state lease and a shared
atomic concurrency limit; the aggregate production ledger alone cannot enforce
component-local capacity. One active waiter holds a lease for 1,024 internal
state bytes and `2 * (256 * size_of::<Selector>() + 65536)` selector bytes.
Both aggregate text length and retained string capacity are bounded. Profile
metadata is one scope and `capacity + 1` leases. Injected route storage,
deadline/pacing future state, native group/compiler work and cleanup executor
capacity retain separate owners. Terminating the wait never transfers its
retry loop into cleanup: only the already-dispatched query or release survives.
After an operation completes the fixed state lease remains until the last
installation clone is dropped. These are accounting bounds, not measured RSS.

The separate main-world element-evaluation leaf pre-admits one **additional**
native Handle for the adopted wrapper, plus an owner allowance that covers
bounded parameter copies and returned values through cleanup. The allowance is
`L.maximum_retained_bytes_per_operation() + 8192 + 4 * L.value.max_nodes()
* (size_of::<EvaluationValue>() + size_of::<HandleEvaluationValue>())`.
Its installed admission/health state owns 256 bytes. The existing value backend
separately reserves its execution peak; the profile checks the simultaneous sum
and conservatively combines two scope declarations and `2 * capacity + 1` leases.
Two caller cleanup reservations span source selection and adoption/evaluation.
`DOM.resolveNode` explicitly joins the original group, so a missing returned
object ID still has compensation. Group release is acknowledged before retiring
the adopted wrapper or its lease. Unpolled evaluation never executes user code;
started evaluation continues the same call exactly once. Failed or discarded
cleanup closes admission, not merely returns local counters. Node now selects
this leaf, with opt-in destruction receipts fixing the reproduced NODE-026 gate.

The opt-in `ChromiumFrameWorldAccess` port reuses the atomic service route table
without another topology copy. Its profile adds `size_of::<State>() + 256`
installed backend bytes and 8,192 bytes per pending or retained snapshot,
one scope and `capacity + 1` leases. The owner type includes the retained
service value; its byte allowance must not be guessed independently of that
type. A shared atomic counter enforces accessor-local capacity across clones
before readiness or route copying. Snapshot admission lasts after readiness
returns and through any use of cloned native IDs/client, including replacement
of the table's current worlds. The accessor retains the service's existing
route leases until the last snapshot is dropped. Invalidation waiting reuses
the independently budgeted readiness waiters. Route retirement, including
renderer handoff, is not native-world destruction or group-cleanup proof.

The explicitly selected native-retirement registry adds checked installed bytes
`size_of::<Registry>() + 256 + capacity * (size_of::<Weak<Receipt>>() +
size_of::<Receipt>() + 32)`. This covers expired Weak allocations until pruning,
not only currently live receipts. It shares the installed-state lease with the
route table and retains at most snapshot capacity entries. Receipt clones retain
the original 8 KiB permit; the allowance covers the receipt/permit and up to five
bounded native-ID allocations. Thus dropping a snapshot cannot admit another
one while native cleanup retains its proof. No extra scope, lease, task or
subscription is installed; the default profile remains unselected. Exact native
destruction is distinct from the existing route invalidation boolean.

The optional selection/adoption preparation profile adds 256 installed backend
bytes, 8,192 bytes per active preparation, one scope and `capacity + 1` leases.
Each preparation separately retains the original selector under
`256 * size_of::<Selector>() + 65536` bytes; the strict waiter owns its two
coexisting clones. The concrete generated operation plus result wrapper and
1,024-byte route-adapter allowance is checked before boxing against the 8,192-
byte ceiling. The private route future is also checked before boxing against
1,024 bytes. Oversized injected future types reject rather than silently
outgrowing the declared allocation. The caller owns pair/error/notification
storage and all existing native/cleanup service budgets separately.

`ResolutionRouteSource::retain_route` and the continuation service's
`call_retaining` keep a caller-budgeted paired snapshot alive through native
resolution, returned elements, adoption/evaluation and acknowledged group
release. Dropping a started query or preparation transfers that exact reference
with its cleanup job; it does not free the snapshot budget early. Stateless
resolution clients still retain no extra owner. A discarded/failed cleanup job
still poisons native admission: this reference is a lifetime guard, not proof
that a retired CDP session released native objects. Node now installs the phase
with explicit additional resource declarations below.

`ControlledDispatcher::dispatch_with` shares the existing graph, pending-call
and target/connection admission pools, deadlines and abandoned cleanup executor
with installed commands. It creates no extra queue, task or resource allowance.
Tests prove actual contention, not just equal configuration. Handler/query/owner
storage and native continuation reservations remain caller-budgeted; returning
a prepared owner does not keep it governed by a completed runtime call.

The host now retains selection/adoption, delayed conversion, evaluation and
release inside one controlled Frame command. The native owner never escapes to
a JS token registry. Six profile scope IDs select eight executing leaves and
sixteen pending calls; 8,192 installed bytes plus a formula-derived 2,490,368
bytes per pending call cover host state on this target. Callback/future inline
bounds are 4 KiB each; command futures are checked against 16 KiB before boxing.
External Node references/heap captures remain separately admitted. Cleanup has
32 slots (the previous 16 plus two per eight new executions). No CDP event cap
changes. Runtime cancellation is not native-reclamation evidence: the Context
close gate (NODE-026) now passes only with acknowledged native destruction.
Full resource derivation and
evidence: [host continuation](knowledge/pwright-locator-evaluation.md#host-owned-evaluation-continuation-2026-09-07).

NODE-025 now reproduces attachment failure at the connection's bounded
`event_channel_capacity + 1` item ceiling (1,025), with Runtime.enable pending.
A bounded diagnostic records 1,012 execution-context creation events. Review
active event retention, initialization consumption and world attribution before
changing the resource plan. No ceiling was changed, and this is not RSS evidence.

The Node profile now explicitly selects an immutable CDP admission callback
for schema-valid foreign isolated Frame-world creation. Classification occurs
after raw/individual payload limits but before shared event leases and fanout;
responses, needed events and malformed/unknown shapes retain their contracts.
Default connections remain unfiltered. The callback adds no queue, task, mutable
registry or allowance; the same 1,024 slots / 1,025 item ceiling apply. The
composing caller must cover all subscribers' interests. A live probe omits
1,440 irrelevant events and closes with zero recorded host resources, but full
Node gates still contain attach timeouts and a lifecycle watchdog kill. This
does not prove browser-native world reclamation or RSS bounds. See
[the policy contract](knowledge/pwright-cdp-event-admission.md).

Semantic Node Locator construction separately reuses the existing query-value
budget: 1,024 recipes / 8 MiB, depth 32, 256 nodes, 64 KiB aggregate strings.
It admits source, regex flags and the implicit eleven-byte test-id attribute
before conversion. UTF-16 validation and UTF-8 copying occur in separate phases
within the existing two-copy allowance; role names have a fixed 32-byte input
limit and options are fixed scalars plus an admitted matcher. Invalid lazy role
issues retain only an enum, not an unbounded error string. No command slot,
remote handle or browser work is needed to construct a query. Command cloning
and result settlement keep their independent existing reservations. Runtime-7
installation bytes remain derived from actual source length rather than a
stale literal; count/queue limits and dependency closures are unchanged.

### Execution-world readiness

`ChromiumExecutionWorldReadinessProfile` separately declares one resource
scope and at most one state lease plus one lease per waiting command. It is
absent by default, requires the topology-event driver when selected, and must
use that topology's exact Frame limit. A borrowed Frame-to-Page iterator also
represents live Frames without execution worlds; the composition retains only
bounded sorted ObjectId pairs, with no native strings or hidden subscriptions.

The declared BackendStateBytes ceiling is
`256 + (max_frames * 128 * 2) + (max_waiters * 512)`. The two Frame allowances
cover simultaneously retained current/prepared route arrays. The Node profile
selects 64 Frames and 8 waiters: **20,736 bytes**, at most **9 leases**, including
a 16,640-byte state reservation. These are conservative declared accounting
allowances, not measured allocation sizes, V8 usage or process RSS.

Atomic count admission precedes each waiter's 512-byte reservation. Ready
routes need no waiter slot; full admission rejects ResourceExhausted. Drop,
original command timeout, Page/Context retirement and transport shutdown
reclaim waiter leases through the existing controlled-runtime cancellation
path. Defaults allocate no readiness Frame table. Eight active commands,
sixteen pending calls and the existing FFI budgets are unchanged. No new
executor, timer, task, event queue or dependency is introduced.

Node Page creation and ordinary close now share the eight-entry typed
lifecycle/topology request queue with command-return fences. The existing
single driver executes one explicit lifecycle transaction at a time, with no
second topology consumer. Creation admits its metadata entry and initial URL
bytes before native target creation, then activates that exact reservation.
Unused preparation is reclaimed without publication; retained closed wrappers
continue to charge the 128-entry metadata limit. Creation uses a normal FFI
command slot; Page close uses the independent close lane. The explicit native
close profile has 20 absence checks and injected 10 ms pauses. Both services
use the existing bounded continuation executor during shutdown. Future public
Page events and shutdown-during-creation ownership still require dedicated
evidence; these additions do not prove whole-process RSS bounds.

The common accounting contract and bidirectional retained-wire integration are
complete, fresh staged remote assembly preserves one ledger identity through
transport shutdown, typed client and server event queues can share item/byte
budgets, and Page screenshot bytes now retain exact backend, wire, and client
leases across their distinct lifetimes. DT-060
also covers runtime live objects, client/runtime pending calls, and active
commands. It remains open until all advertised v2 producers use the appropriate
lease for retained payloads; backend and capability production factories
reject missing declarations as runtime, wire, encoded client, and typed events
now do; and the DT-003 memory workloads verify teardown and steady-state RSS
separately for runtime and browser processes. The optional process-metrics
probe now makes that OS split executable with bounded snapshot, record, and
command inspection. Its opt-in recorder correlates those samples with internal
ledger and object snapshots, enforces a cleaned warm comparison baseline, and
has real-Chrome idle external-attachment, Page/Context, Playwright-shaped
navigation, and retained-Handle teardown proofs. Handle observations correlate
native routes, runtime objects, and `RemoteHandle` units, including explicit and
parent retirement. Idle teardown separately proves runtime invalidation, CDP
connection-owner release, and external-browser survival through reattachment.
The drivers also distinguish immediate logical cleanup from bounded Chrome
process-tree settlement before post-teardown RSS sampling. A bounded exact-run
series now rejects mismatched or unclean reports and records diagnostic RSS
ranges; the navigation driver has one and the network driver has two consecutive
three-run Chrome series. Other
workload series and reviewed thresholds remain open.
