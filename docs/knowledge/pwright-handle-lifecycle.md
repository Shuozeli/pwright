<!-- agent-updated: 2026-09-19T19:03:44Z -->

NODE-190: Import begins with non-exclusive exact-session and `Serialized` stage
inspection. The host admission is acquired before phase checkout or graph
access, so foreign-session and capacity failures cannot revoke the legitimate
ticket. Once checked out, the addon copies the graph synchronously and rechecks
the phase before replacing the record. A getter cannot be introduced through
the current package-owned graph, but the final fence prevents future reentrant
adapters from resurrecting a revoked entry.

Success commits `PayloadImported`, drops all direct V8 roots and retains the
Rust graph plus its sole owner lease. Import errors after checkout revoke the
phase; pre-check admission errors leave `Serialized` retryable. Explicit abort,
deadline, scope/environment teardown or session close drops the graph and
returns its admission. NODE-191 may instead call the move-once `take`; repeated
consumption fails closed. [Canonical NODE-190 lifecycle](pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: Late serialization accepts only the exact owning session and
`Instrumented` stage. Its synchronous JavaScript traversal runs while the
registry phase is checked out; reentrant abort/retirement removes that exact
entry, and the mandatory post-callback recheck prevents a serialized root from
resurrecting it. Successful completion replaces the root container once and
commits `Serialized`; abort drops both raw and serialized graphs plus the same
validated native owner. [Canonical NODE-189 lifecycle](pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: Instrumentation begins only from `Prevalidated`. The begin gate checks
the exact owning session before checking out the phase, reads the original
receiver from root zero, and commits `WaitingInputAction` while the validated
owner remains parked. The finish gate accepts only that same session/ticket and
stage, then commits `Instrumented`. Neither gate moves the retained Rust owner or
FFI preparation.

During the Node await, timeout, source retirement, session close or explicit
abort can remove the entry and revoke its existing owner. A late finish then
observes closure and cannot resurrect it. Listener rejection uses the same abort
path. Foreign-session rejection occurs before exclusive checkout, so it cannot
destroy the rightful owner's preparation. [Canonical NODE-188 lifecycle](pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: Prevalidation checks out the exact parked mark completion, moves its
owner into one asynchronous Rust future and parks the validated owner back on
the same FFI ticket. The phase is rechecked before native work and before
`Projected -> Prevalidated` commit. The existing Node progress races the await;
abort, timeout, scope/environment close or drop reaches either the running
revoker or the re-parked nested owner.

`PrevalidationError::MarkedCleanup` preserves marker-specific cleanup failure
instead of pretending it is a locator-resolution cleanup. Successful terminal
drop releases derived preflight admission and the original marker/adopted group
as one chain. The persistent marker world may remain cached by design; only
per-call and preflight owners must return to the exact warm baseline.
[Canonical NODE-187 lifecycle](pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Ordered mark completion now separates the consumable error outcome
from the retained successful owner. `finish_outcome` clears failed completions,
but a successful `RetainedMarkedFileInput` remains move-only inside the same
parked FFI state across Node carrier projection. Projection callbacks can abort
or replace nothing silently: the phase is rechecked before new roots and before
the transition to `Projected`.

Ticket abort, scope/environment close and ordinary drop all reach the owner's
pre-admitted group cleanup. A persistent marker world may remain cached by
design, so exact live baselines are sampled only after the temporary warm-up
mark call settles. The owner is not yet consumed by native DOM prevalidation;
the lifecycle is therefore internal and incomplete.
[Canonical NODE-186 lifecycle](pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: The selected preparation now retains its protocol call identity from
post-schema allocation through Materialized and Marked. The post-instrumentation
deadline gate rechecks source registration before any materialization getter or
serializer. One Node progress then owns both language materialization and native
marking; timer delivery removes the exact registry entry and reaches either its
running weak revoker or parked FFI owner. The timer is cleared on every exit and
no second deadline or command admission is created.

This does not yet cover later projection, prevalidation, mutation or every
shutdown/publication race. The N-API deadline method identifies the cancellation
edge, but native cancellation still carries no typed timeout reason; public error
identity is Node-owned. Targeted standalone/combined browser cases prove baseline
reclamation for the new path, and both rebuilt artifacts pass the final internal
26/26 lane. This is still not public upload or differential API parity.
[Canonical NODE-185 contract and evidence](pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: The selected Node handoff upgrades the original NativeHandle Weak only
inside admitted Rust work after all current language materialization. Explicit
disposal before that upgrade produces the existing disposed-Handle error and
aborts the preparation; it cannot be replaced by an arbitrary JS receiver.
During native work the Node registry keeps a weak FFI revoker, not another
Handle owner. Scope/environment/host close removes the entry outside its borrow
and cancels running work; successful completion returns the same ticket to a
parked entry before publication callbacks can reenter.

Live coverage includes late disposal, synchronous running/queued cancellation
and foreign-session rejection without owner-ticket loss. Pending-shutdown and
publication-triggered revocation races remain explicit gates. See the
[canonical NODE-184 checkpoint](pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: The default-off host `retained-target-marking` cache now owns persistent
exact-utility-world marker state separately from source Handles and temporary
adoption groups. It admits 64 current/creating worlds, eight retiring worlds
and eight lookup/call leases. Same-world waiters share one admitted initialization;
canceling one does not cancel the others. Last-waiter abandonment or sealing
rejects late publication while its admitted continuation still cleans up.

Persistent backing contains only its separately admitted utility snapshot and
independent retirement receipt. A temporary call owns the Handle/source, paired
snapshot, tracked admission and marker lease through target-group settlement.
Topology-boundary pruning handles replaced worlds, removed Frames and closed
Pages without another acquisition. Currentness is rechecked before publishing
or marking; stale leases cannot start new marks, and generation checks prevent
old receipts from clearing a reused slot. A still-retiring exact world cannot
be revived as a second marker state.

Admission seals before shutdown; owner detachment and both creation/cleanup
drains occur outside host locks after owned native-session retirement. Route
callbacks, owner drops and wakeups also occur outside cache locks. Snapshot
release precedes slot reuse; failed/abandoned cleanup keeps admission closed,
regardless of returned counters. Node still stops at Materialized: its Weak
upgrade, typed promotion, original deadline/call-ID policy and full retained
upload are not integrated. Exact verification and remaining gates belong to the
[canonical NODE-183 checkpoint](pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache);
no new live-test pass is claimed here. Older notes below retain historical scope.

NODE-182: The selected owned-world marker has unique persistent group ownership,
separate from each adopted target's temporary group. State/init and one-shot
call allowances are admitted independently; exception wrappers stay with the
target group. A successful marking result retains the target plus call admission
until its group is released, without resetting persistent marker state.

The composition reserves cleanup before creation/marking. Unpolled creation
performs no browser work; dropped started work continues the same admitted call
and cleanup instead of replaying it. Persistent state/route backing remains
retained across in-flight marks. Explicit marker release while busy returns the
same owner unchanged; dropping that owner defers release until in-flight
references finish. Ready-target and explicit-release Drop preserve the reserved
cleanup path and allowances through group settlement.

Only acknowledged group release or authoritative retirement of the exact native
world/session discharges ownership. Route disappearance, transport failure and
error text are not proof. A single-waiter release-fence receipt must not be shared
by concurrent group releases. Failed or abandoned cleanup poisons new admission
before backing returns; an executor dropping promised work does not establish
native reclamation. Raw backend callers must uphold these continuation contracts.

This is not yet a production world registry, NativeHandle Weak upgrade or
same-ticket Node execution phase. Materialized still holds a unit FFI receipt;
deadline/call-ID/instrumentation integration and public retained upload remain
open. Exact verification belongs to the [canonical NODE-182 checkpoint](pwright-node-handles.md#node-182-persistent-native-target-marking),
with isolated native-owner evidence distinct from missing Node integration.
Historical NODE-181 and earlier descriptions below are unchanged.

NODE-181: The selected InputRecord now binds the actual original NativeHandle
state through a private Weak-only value. Capture follows admission; snapshot,
dispatch and language-phase replacements preserve it without upgrading,
locking or checking disposal. The JavaScript receiver can still participate in
observable language behavior but cannot replace this inert original identity.
There is no new strong native pin, authority or early liveness check.
Concrete replacement/snapshot overlap is checked against both unchanged call
and independent synchronous-phase allowances. The latter's 16 KiB lease
survives reentrant revocation until those temporaries are destroyed; its public
capacity constant and actual held reservation are checked by the host test.

Future native execution must upgrade and revalidate only at its proper boundary,
then keep typed progress, the exact session revocation receipt and cleanup
ownership together. The preparation still stops at Materialized with a unit
FFI result. Persistent exact-utility-world marker state, connection-wide call
identity, the deadline before materialization and distinct recoverable/fatal
marking outcomes are future requirements, not implemented capabilities. Such
marker state must outlive a call's temporary group and must not be cleared on
ordinary completion, timeout or source disposal. No marking, native promotion,
late mutation or public upload is claimed. Actual-ticket browser results and
their cleanup-observation scope are recorded canonically; neither those cases
nor actual-layout tests prove later Weak upgrade, wrapper GC or future native
group settlement. Final fixture retirement waits for zero native-host, FFI and
preparation resources, including watch admissions, without extending deadlines.
[Canonical contract and verification](pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: Exact-session retained invocation hooks now connect safe-host teardown
to weak FFI revokers before either operation or Handle-owner waits. Registration
checks weak owner provenance and binds target/creation Frame/Page metadata plus
exact registry generation; it deliberately does not introduce a disposal,
execution-world or DOM check before the proper native boundary. The caller must
first verify the Node preparation scope of its supplied revoker. The move-only
receipt must survive every native/parked phase through cleanup; dropping it
unregisters the hook rather than canceling the FFI operation.

Shutdown cancels outside locks, takes the admission-state barrier and detaches
hooks, then closes the tracker outside locks and revokes before both drains.
Raw peer termination and teardown fallback do not depend on a Node callback.
Detached batches have armed Drop and retain their administrative lease through
callouts. Handle closure is outside the outer session lock; Owners::close seals
and detaches under its own lock, then closes its tracker outside that lock.
Running FFI work still retains admission until actual future destruction;
revocation alone is not an acknowledged native release.
On failed native retirement, both Handle and file-input-only shutdown branches
now clone the connection under the session lock, then request disconnect after
unlocking. Synchronous CDP cancellation/reply wakeups cannot reenter that held
lock; the original retirement error and cleanup ordering remain unchanged.
This is a source-reviewed correction with recompilation tracked canonically,
not an assembled HostSession retirement-failure regression or native ACK proof.

Eight weak slots consume a selected 4 KiB active allowance, not a bound on stale
weak tombstones, process RAM or native DOM graphs. Register only at the first
native handoff, without changing the earlier 32-entry language-preparation cap.
Actual Node promotion remains unwired: Materialized still owns a unit receipt,
and marking, NODE-178 selection, late serialization/mutation and public upload
remain open. Final verification belongs in the canonical checkpoint; no browser
proof is added. [Canonical contract and verification](pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Optional FFI handoff now follows Initial -> Running -> Queued ->
Parked -> Running under the same typed entry and permit. Fresh start cannot
overwrite a parked result. Parking requires settled success, removes queued
counts and returns only the previous future-body lease; no running token spans
the parked language wait. Resume atomically registers work before extracting
the owner. Its `(T, FfiExecution<T>)` tuple is trusted internal plumbing: both
must immediately enter the next native continuation, with fresh future-size
admission, or be dropped together. It is not permission to keep native ownership
after dropping its execution admission.

A weak Send/Sync revoker targets one exact entry, not a HostSession. Per-entry
revocation wakes outside locks; Running storage survives actual future Drop,
while idle/late results are destroyed before their permits outside locks.
Future-before-execution ownership covers unpolled Drop; an inner future scope
ends before settlement. Pure parked inspection releases the registry guard
before its callback, but holds the entry guard and permits neither reentry nor
N-API/native-authority escape. These are bookkeeping guarantees, not measured
allocation or browser group-release guarantees.
Global shutdown separately seals its registry before cancellation and tracker
closure, both of which can synchronously wake Rust code and now run outside
locks. Admission/start/resume/inspect/park reject the seal-before-notification
gap. Generic eager-execution Drop and shutdown-wakeup probes also run without
the retained-file-input feature; exact final evidence belongs in the canonical
checkpoint, not inferences from selected-only tests.

Production integration remains absent: the Node ticket still stops at
Materialized with a unit preparation, and raw projection is diagnostic-only.
Bounded exact-session revokers must be installed before parked owners can hold
route/work tokens and invoked on explicit close or raw peer termination before
`Owners::wait`; Node callback/environment cleanup alone is insufficient. Native
marking, one operation deadline, projected-reference root admission, NODE-178
prevalidation selection, later utility serialization/mutation and public upload
remain separate gates. No new browser result is claimed.
[Canonical contract and verification](pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Standalone retained file-input prevalidation now transfers an adopted
settlement-owned group into a validated receiver owner, not a public Handle.
Extra result/exception capacity and route ownership stay retained through a
late query and group release after cancellation. Ready-owner Drop and explicit
release cancellation reuse a separately reserved cleanup continuation; no
mutation follows a canceled precheck. Cleanup errors close admission before
returning extra backing. Dropping a broken executor's armed work poisons the
service but cannot prove native reclamation. Future Node handoff must separately
design revocation of this parked owner, not hold an unstarted task token during
an arbitrary language await. Host/addon selection remains off.
[Canonical contract and verification](pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: The selected preparation has two Rust-owned registration transitions:
ChannelReady -> ServerDispatchReady before server validation, and
ServerParamsReady -> ServerMaterializationReady after the empty instrumentation
await. Exact owner-ticket plus creation-Page membership is checked, not
world/Handle/Frame runtime presence; navigation retirement is not explicit
dispatcher disposal. These checks create no task token/native pin or extra
roots/budget. A foreign direct gate cannot abort the originating ticket;
the orchestration driver owns failure abort. Its undefined Materialized return
avoids assimilating ticket.then, leaving the caller's ownership intact.
Full instrumentation, late utility serialization, native promotion/submission
and public upload remain separate lifecycle gates.
[Canonical contract and verification](pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: New server-schema/begin-materialization/finish-materialization
phases recheck revocation before replacing roots; no registry borrow crosses
user code and no phase guard crosses await. Six roots retain params/timeout
and the raw awaited carrier. Native source revalidation and same-permit
execution promotion remain separate, unimplemented Node integration gates.
[Canonical contract and current evidence](pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): The selected raw preparation gains a synchronous
ArgumentsReady to ChannelReady transition. Rust validates arguments and replaces
root six under the existing revocable phase guard; no new root, FFI execution
token, scratch allowance or native-use authority is created. Abort/close and
failed replacement retain existing ownership behavior. No registry borrow or
metadata lock may span user getters, array methods, boxed valueOf or inherited
setters.

Validation follows payloads (name, optional mimeType, buffer), localDirectory,
directoryStream, localPaths, streams and timeout, including optional fields,
observable map/raw results and raw Buffer identity. Ordinary assignment stays
observable; this is not serializer or byte-copy work. Three no-capture/ZST
native callback specializations use static typed dispatch and current
Env/path/ports per invocation, never capturing scoped N-API handles, preparation
leases or native owners. This avoids new Rust callback-Box allocation and its
construction-failure leak; N-API/V8 callback backing and arbitrary escaped JS
graphs remain outside administrative RSS bounds. Normal non-isUnderTest mode
only is selected. Without a genuine WritableStream
producer/brand, observe the first `_object` read and fail closed rather than
inferring channel authority from object shape.

Preparation/scratch/metadata, feature, dependency and class budgets remain
unchanged. Offline verification is recorded in the canonical checkpoint;
actual-ticket/browser/public gates remain open. Shared CDP is blocked without
restart permission and no new browser pass is claimed. Public error prefix/
stack, deferred server bytes/path preparation, float deadlines, typed move-only
FfiPreparation promotion and renewed native-source authority, plus public
ElementHandle.setInputFiles, remain separate gates. [Canonical contract and exact evidence](pwright-node-handles.md)
own completion claims; earlier lifecycle notes remain historical.

NODE-174 (2026-09-17): A Native Handle's timeout binding is immutable metadata
outside its Live/Disposed state. Production publication checks exact weak
Handle-owner identity and frozen creation-Page proof; Rust-derived producers,
including generic descendants and property batches, inherit that binding.
Preparation captures it before observable language phases, and argument
assembly accepts no replacement settings. Metadata checks do not infer current
DOM ownership or grant disposed/closed Handles native-use permission; normal
source admission/revalidation remains separate. Local retained Frame/Page
wrappers keep their creation relationship after membership removal.
The facade forwards creation Page ID only, with no JS cell/cache/lookup.
Rust captures canonical policy or lookup error after FFI admission and defers
the error through normal source/argument materialization before host execution.
Neither pre-commit lifecycle construction nor a closed Page performs an earlier
JS metadata query that could skip conversion. Each selected Frame producer
issues its own binding to shared cells; only Native Handles retain published
policy, which their descendants reuse independently of later membership.

Canonical Context/Page cells come from bounded published session membership.
Close drops index-held cell references; the final index clone releases its
16 KiB admission, so retaining a closed index is not a free slot. Fresh scoped
publications consume binding leases shared by their clones, in the same
1024-slot pool as cells. Those clones retain cell/binding/factory metadata and
Weak<Owners> proof, not the index, HostSession, transport, native owner or task.
Factory administration and sixteen bounded indexes bring the total to 784 KiB;
Handle/FFI embedded clones remain covered by their existing record admission.

The facade freezes raw debug inputs at import; Rust owns undefined-only
fallback and inspector exclusions. No feature/profile/dependency/class or
shared-browser ownership expands. Public default setters/arbitrary JS state,
channel/error phases, deferred bytes/paths, native deadline/submission
integration and public retained upload remain open. [Canonical contract and
exact verification](pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding)
own remaining gates; no new browser pass is claimed. Earlier lifecycle notes
below preserve their historical scope.

NODE-173 (2026-09-17): The raw preparation gains a synchronous argument phase
after conversion assimilation, without another await. Rust spreads converted
input then options, resolves live timeout metadata, and atomically moves Ready
to ArgumentsReady. Params replace the converted value in root six: no extra
registered root, preparation slot, FFI command or native task is added. Existing
32-preparation and 512 KiB synchronous-phase limits and revocation checks stay
in force; no registry/metadata lock spans user getters.

Separate selected timeout cells own only client metadata: a Context root or
Page child, typed optional numeric defaults and inherited injected debug bool.
The 1024-cell budget admits 512 bytes per cell plus 16 KiB factory administration
(528 KiB total, 1025 leases, one scope). Cells keep the factory Arc alive until
the final cell lease drops, but retain no Node roots, HostSession, native-use
authority, browser object or running task. Module-local branding and safe
publication protect the internal settings class, not creation-Frame identity.
The current caller chooses that association; it is not automatically bound
or validated and cannot substitute for native source revalidation.

Live defaults are read after each options getter; parent fallback repeats
the original options read. The model is limited to the normal two-level
Page/Context chain, not arbitrary custom default state. Production facade,
public setters/default/debug configuration, channel/error phases, deferred
Buffer/path work, native deadline-domain mapping, promotion/submission and
public retained upload remain open. No shared-browser ownership, feature,
profile, dependency package or server expands.
[Canonical contract, evidence and remaining gates](pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
are authoritative. Earlier notes preserve their historical scope.

NODE-172 (2026-09-17): Preparation language phases use weak scope/registry
identity for one exact entry. The record and FFI permit stay in revocable
storage; the guard owns independent synchronous scratch, not roots or native
authority. Pure `RootSnapshot` inspection clones weak source metadata and an
`Rc<NodeReference>`. Reference lookup occurs after the registry borrow ends;
that temporary Rc drops before user JavaScript, leaving callback-scoped handles.
No N-API call occurs inside the borrow. Initial `creation_phase` and later phases
pre-admit scratch before N-API allocation and recheck liveness after callbacks
to avoid revoked allocations. Replacement rejects revocation, and old/rejected
records drop outside borrows. Abort, close or an unfinished guard's drop removes
the entry; a surviving guard cannot pin registered roots. Its scratch lease
survives callback abort/finalization until the raw synchronous phase ends, not
arbitrary Promise waits. A completed guard cannot abort a reentrant successor.

Private roots grow through receiver/files/options, context, items and the
awaited conversion carrier (3/4/5/6 values), at most 192 registered values across
32 entries. The separate registry-wide scratch ceiling is 32 synchronous
construction/replacement phases at 16 KiB each (512 KiB aggregate); no FFI
Command slot/task or native authority is acquired. This raises selected resource
totals while preserving 32 entries, 16 scopes, the checked 16 KiB table, 1 MiB
per preparation and shared 64 MiB FFI pool. Arbitrary V8 graphs, scoped handles,
opaque allocator backing and retained closed/weak wrappers are not byte-bounded
by these reservations. NODE-171 watcher-delivery and raw-session
revocation limits still apply; raw closed/cancelled phase ordering requires
public alignment.

Rust preflight and the ordinary `{ payloads }` async carrier grant no native-use
pin, promotion or submission authority. Paths are explicitly unsupported after
classification, and public `ElementHandle.setInputFiles` stays unexposed.
Stateless kernel diagnostics do not prove actual-ticket N-API or browser
lifecycle behavior. No shared browser/Page ownership, feature, dependency or
artifact profile expands. [Canonical contract, evidence and remaining gates](pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases)
are authoritative. Earlier notes preserve their historical scope.

NODE-171 (2026-09-17): Preparation now uses a weak exact-session Node ticket
and fixed-size `HandleSourceIdentity`: weak Owners provenance, target,
creation Frame and Element kind. Identity can survive explicit disposal and
descriptor drop without retaining a native service, work token or resource
lease. It deliberately proves neither ticket liveness nor current native/world
ownership. NODE-169 submission remains responsible for those checks.

Only admitted preparation entries retain receiver/files/options roots. Scope
close, NativeSession Drop and environment cleanup revoke entries independently
of ticket GC; roots drop on the Node thread outside registry borrows before
the FFI drain. Delivered terminal publication also revokes. A raw session
without a watcher can require later publication, explicit close, wrapper Drop
or environment shutdown; host validation failure at an arbitrary entrypoint
alone does not revoke the scope. The fixed 16 KiB administration covers the
bounded registry/live scopes, not arbitrary retained closed wrappers, weak
tombstone allocations or V8 graphs. No shared-browser/Page ownership expands.

The new retained-file-input addon/artifact selection exposes only this internal
preparation boundary, not public `ElementHandle.setInputFiles`, conversion,
promotion or native submission. [Canonical contract, evidence and remaining gates](pwright-node-handles.md#node-171-session-bound-node-preparation-registry)
are authoritative; no browser verification is claimed here. Earlier notes
below preserve their historical scope.

NODE-169 (2026-09-17): Safe-host `retained-file-input` is a separate default-off
selection; existing host/addon profiles remain unchanged. Retaining the source
descriptor and borrowed native identity does not pin its native lifetime against
explicit disposal. The host checks the original creation Frame/world, adopts
into that Frame's utility world, and revalidates before submitting mutation.

Adoption and upload retain independently admitted, untimed phase cleanup.
Cancellation during adoption continues only native acquisition and owned-group
release, never a previously unsubmitted upload; a started upload has its own
settlement owner. Shutdown drains both phases and preserves cleanup failures.
The addon/public Node retained-upload API and ingress rendezvous remain absent.
[Canonical NODE-169 contract, exact verification and remaining gates](pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle)
own the evidence; no live-pass claim is made here. NODE-168 and earlier entries
below remain historical checkpoints.

NODE-168 (2026-09-17): The default-off `retained-reference` resolver entry point
adopts an exact retained source into a distinct admitted object group and returns
the real `ResolvedElement`/`OwnedElement` owner, without selector evaluation.
The caller retains source/route admission; the composition reserves cleanup
before acquisition and continues started work through group-release settlement.
A Frame snapshot supplies coherent utility numeric and unique context IDs.

The new group's destruction proof remains disabled until a constant probe
validates its adopted argument in the exact unique world. Numeric-context reuse
or a failed probe therefore cannot discharge cleanup using the expected old
world's receipt. A started group acquisition that fails before validation
explicitly releases its group. The legacy Handle-adoption abandonment branch
now reports Cleanup as continuation
failure, not successful abandonment; ordinary non-cleanup failures keep their
existing behavior. Public Node retained upload is not yet exposed.
[Canonical NODE-168 integration and verification](pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress)
owns exact evidence and remaining gates; this source checkpoint makes no
live-pass claim. NODE-167 and earlier entries below retain their historical scope.

NODE-167 implementation contract (2026-09-17): `ownerFrame` is a temporary
native-owner query, not persistent Handle creation. It follows the retained
node's current DOM document rather than its creation Frame, retains the document
element for direct raw `DOM.describeNode`, and holds evaluation/native admission
through release settlement. The raw caller must pre-own continuation capacity
and retain the source and whole future after cancellation; dropping that future
does not acknowledge cleanup. The host supplies this started-call continuation.

OwnerFrame acquisition alone validates and copies one protected receiver ID
after evaluation admission. It rejects source-alias success and excludes that
borrowed ID from malformed/exception compensation while attempting every
distinct new owner. This closes that narrow compensation hazard, not generic
borrowed-ID safety: existing acquisition paths are unchanged. Retaining a Rust
descriptor also does not prevent concurrent explicit native source disposal.

Scalar and batch publication retain only an immutable source-world fingerprint:
Page ID, numeric context ID and `Arc<str>` unique context ID within the existing
16 KiB descriptor allowance, not a client, receipt, permit or native lease.
The host can report the canonical navigation error before lookup or refine an
ordinary TargetClosed only for the same Ready Page with an observed replacement
main world (changed numeric or unique ID). A missing world, crash or close is
not navigation proof; failed Cleanup is never replaced by this diagnostic.
Exact verification of this refinement remains in the canonical record.

Each query shares one of the eight existing call slots and reserves two
transient `RemoteHandle` units, sixteen aggregate at pending scope
`extension(154, 1)`. The 64 persistent-result owner slots are unchanged and are
not consumed by owner lookup. After topology publication, Rust maps the native
Frame within the source BrowserContext; the handles-only Node method returns
its canonical cached wrapper or null. Descendant Handle producers preserve the
resolver. This does not add retained upload. Unaddressable raw IDs/pre-owned
groups and conservative Cleanup-versus-detach failures remain unresolved;
neither an ordinary detach error nor descriptor drop proves native release.
[Exact verification and remaining gates](pwright-node-handles.md) are canonical;
no live-pass claim is made here. NODE-166 and earlier entries below are historical.

NODE-166 (2026-09-17): The selected property-batch path now admits sibling
routes, pending-result owners and individual cleanup tickets before native
enumeration, plus one whole-batch recovery reservation. All siblings share the
original call allowance; unused maximum admission is trimmed downward without
reacquisition. Property names retain that shared allowance until consumed by
the language boundary. Route-batch trimming never creates an object or enlarges
the original reservation.

Native property reads use explicit raw settlement ownership, preserving
descriptor and exception RemoteObjects for attempt-all compensation. The staged
batch publishes runtime objects and routes all-or-none after revalidating the
source Handle and exact world. A busy lifecycle transaction parks the same
batch for local-only retry, without another browser evaluation or property
read. Each returned Handle is independently owned under its Frame, not under
the source Handle; disposing one sibling does not dispose the others.

Receiver loss before commit compensates the whole unpublished batch. An unread
published reply drops independently registered sibling owners. If owner close
rejects retention after native/local publication, every sibling ticket must
still be registered or submitted, including siblings after the first error.
Refused compensation remains a failed cleanup job; neither a dropped descriptor
nor a generic TargetClosed/Cleanup result proves native release. Existing exact
native-lifetime checks are not broadened or replaced with the earlier host
snapshot.

Page.close now also records its already-confirmed exact target disappearance
in the selected retained-world registry before route publication. This removes
dependence on a later topology destruction event for that receipt, without
treating command acknowledgement or inspector loss alone as destruction.
See [the Page confirmation contract](pwright-page-close.md) and
[canonical NODE-166 verification and remaining gates](pwright-node-handles.md).
New deterministic cases exercise batch ordering, unused admission, lost replies,
busy/stale/disposed sources, independent disposal, empty batches, refused
cleanup and owner-close-before-retention. That last case verifies attempt-all
after retention fails; it is not injected mid-loop thread scheduling. Exact
executed counts, live results and outstanding compatibility work belong in the
canonical checkpoint, not inferred from this source-coverage inventory.

NODE-165 (2026-09-17): Raw retained-result acquisition owns both result and
exception RemoteObject identities before parsing can fail. Its explicit
release-to-settlement path attempts every distinct addressable ID and retains
evaluation admission until acknowledgement or native failure. Cancellation
transfers the started future to existing admitted continuation ownership;
a refused release or unaddressable retained ID reports Cleanup and therefore
failed continuation health. A local drop or ordinary error is not release
proof. Shared parsing, ordinary CDP release methods and the existing exact-world
retirement rules remain unchanged.

[Canonical verification and remaining lifecycle gates](pwright-node-handles.md)
separate this acquisition boundary from Runtime.getProperties late settlement,
full property-pipeline admission and the unimplemented pre-owned-group design
for unaddressable identities. Node batch cleanup/publication and full-corpus
compatibility remain open; no new package, feature, schema or public API is added.

NODE-165 shutdown ordering: with Handles selected, shutdown seals admissions,
stops/drains caller operations, revokes publication and retires only this
connection's owned inspector sessions before waiting for Handle work. Detach
failure remains an error and requests transport disconnect before the drain;
non-Handle shutdown ordering is unchanged. This broadens neither shared-browser
nor borrowed-Page ownership. A compensation-versus-detach race can still report
Cleanup conservatively even after acknowledged session retirement. Do not
suppress it using the host's earlier world snapshot: compensation failure must
carry the actual frozen route/client identity and match an exact retirement
receipt first. [The canonical shutdown record](pwright-node-handles.md#node-165-retained-acquisition-owns-failure-cleanup-and-shutdown-settlement)
owns the evidence and this explicit residual.

NODE-164 (2026-09-17): Portable HandleGetProperty now admits its persistent route before native snapshot creation and reuses single-result publication/compensation. An abandoned portable operation that reports Cleanup also reports failure to its continuation owner instead of silently succeeding. All raw N-API classes now have module-local identity, addressing the earlier priority class-layout audit gap without changing disposal authority. [Exact verification and remaining gates](pwright-node-handles.md) are recorded canonically; Node getProperties and its sibling-specific batch cleanup are still open.

NODE-163: New single-result producers retain the existing abandonment/recovery path and exact world-retirement witness. Native class construction/tagging failures drop the still-owned descriptor; failed wrapping reclaims its Box. JS classification/binding/marker failures synchronously request disposal and await completion, preserving both publication and cleanup errors. Unit fault tests are not live N-API failure injection.

NODE-162: Disposed Handle evaluation targets no longer preempt ordinary argument traversal; their errors are carried through the existing bounded conversion path before execution. This owns no disposed descriptor and leaves accepted cleanup receipts unchanged. Full concurrent-disposal and Worker/failure parity remain separate gates.

NODE-161: Argument conversion can temporarily retain an Arc to an admitted descriptor through execution, but the wrapper and native sidecar share one Live/Disposed state. Explicit disposal still revokes the existing ticket once and future argument admission validates that ticket state. A disposed argument is remembered during ordinary getter traversal and rejected during materialization, matching the pinned client phase.

NODE-160: Accepted retained cleanup now publishes a shared completion acknowledgement. Explicit dispose is idempotent, cancellation of a waiter does not cancel cleanup, and executor cancellation/deadline yields a missing-ack failure instead of success or an endless wait. Completion-only receipts survive descriptor drop and session shutdown without retaining service admission.

NODE-159: The existing host release coordinator also accepts typed failed unpublished compensation. Unpublished results require exact native-world retirement proof but have no local object to retire. Published results retain both native-world and local-object checks; unproved detach and generic failure never count as cleanup success.

# Pwright Handle lifecycle checkpoint

## NODE-157: selected host release scheduling

The safe host now selects native-release and queue-free deferred disposal
reservations. Existing cleanup workers perform native I/O; the existing topology
driver commits acknowledged receipts through an eight-slot inbox. Busy local
commits park without resending native commands or self-waking. A TargetClosed
release only succeeds via parent destruction when the exact frozen world has
native retirement evidence and the old runtime object is no longer live.
Otherwise cleanup remains a failure, including evidence-free TargetClosed.
Shutdown keeps pumping native events while owned cleanup drains. This does not
yet coordinate Handle creation publication or expose public Node Handle APIs.
See [host ownership, budgets and verification](pwright-node-handles.md#node-157-host-release-coordinator-and-parent-retirement-evidence).

## NODE-156: native release separated from exact-route retirement

The optional native-release feature adds a consuming request and successful
native acknowledgement receipt. No lifecycle transaction or route lock spans
native I/O. The short local commit compares the retained route resource owner,
so an old completion cannot retire an object that reused the same portable ID,
payload or disposer. Busy commits retain admission for local-only retry;
successful commits return post-lock runtime notification ownership. Primitive
routes require no browser command. Caller-owned continuation and shutdown
coordination remain explicit and are not installed by this leaf.

Selected route leases stay charged while a native request/receipt survives
parent retirement. Default representation/budgets and legacy disposal service
behavior are unchanged. See the [canonical ownership and verification record](pwright-node-handles.md#node-156-optional-native-release-and-exact-route-commit-receipts)
for resource formulas, 15 deterministic tests, real Chrome evidence and remaining
host/public Node integration.

## NODE-154: host cleanup sequencing

The selected safe host now owns pending creation results through a revocable
ticket table. Real Chrome shutdown testing exposed the legacy disposer's global
prepared-runtime transaction: concurrent Handle releases are rejected while
another native release awaits its acknowledgement. A bounded host-local turn
sequences its result cleanup calls; it does not change the lifecycle leaf or
coordinate other Page/Context/topology transactions. Mixed lifecycle ordering
and public Node wrapper disposal remain open. See the
[native evidence and pending-owner contract](pwright-node-handles.md#node-154-session-owned-pending-results).

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
