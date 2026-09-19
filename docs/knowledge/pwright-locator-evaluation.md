<!-- agent-updated: 2026-09-17T15:52:46Z -->

NODE-183: Exact `existing_frame_utility_world` selection now obtains one
independently admitted utility-only snapshot under the authoritative route lock.
A registered Frame with no observed utility world returns None immediately;
missing Frame/Page registration or a crashed Page is an error. A Page ID is
never substituted with its main Frame. No main world, browser I/O or readiness
wait is required. Identity validation precedes cloning, and each selection owns
its own existing 8 KiB snapshot permit and independent retirement receipt.

The default-off host `retained-target-marking` feature adds 72 snapshot slots
for its 64-current/eight-retiring marker cache; it does not borrow the eight
per-call retained-upload slots. Marker backing retains only that utility
snapshot, while each mark's adopted target separately retains its paired
snapshot, source Handle, tracked call and short-lived marker lease. The two
receipts therefore do not share single-waiter release-fence authority.

The host matches exact Page/Frame, client allocation and utility unique identity,
rechecks currentness before publication/marking, and prunes stale routes at its
serialized topology boundary. Cleanup acknowledgement precedes capacity reuse;
failed cleanup closes admission. Existing Locator policy is unchanged. This is
not Node preparation promotion or public retained upload, and no new live pass
is claimed here. See the [canonical NODE-183 contract and verification](pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache)
for exact evidence and remaining gates; earlier checkpoints stay historical.

NODE-169 (2026-09-17): The default-off host-only `retained-file-input` feature
reuses the retained-reference resolver without selector evaluation. Its coherent
utility snapshot belongs to the retained Handle's original creation Frame,
not whichever Frame currently owns the DOM node. Borrowed source identity and
descriptor retention do not pin the native object; the host revalidates the
source/world before adoption and before submitting the upload.

Adoption cleanup has its own admitted, untimed continuation, separate from the
upload continuation. Abandoned acquisition can finish only acquisition and
group cleanup, never start an unsubmitted mutation. Existing Locator selection
is unchanged; no addon feature or public Node retained-upload API is exposed.
[Canonical NODE-169 contract, exact verification and remaining gates](pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle)
own the evidence. Offline ingress characterization is not browser validation;
no live-pass claim is made here. Earlier entries remain historical checkpoints.

NODE-168 (2026-09-17): Default-off `retained-reference` on the backend and
composition resolver leaves adds exact retained-node adoption without selector
evaluation. It returns the existing group-owned `ResolvedElement`/`OwnedElement`
types and reuses pre-admitted continuation and route-owner retention. A Frame
snapshot now exposes utility numeric and unique IDs captured under the same
route-table lock; the snapshot is not a lock against later native navigation.

After numeric-context resolution, the adopted argument must pass a constant
probe in that exact unique world before the group can accept the supplied
destruction proof. An unverified group instead requires explicit settled
release, so reused numeric contexts cannot borrow an old world's death proof.
The legacy Handle-adoption abandonment path separately preserves Cleanup as
failed continuation health. Public Node retained upload is still absent.
[The canonical NODE-168 record](pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress)
owns integration gaps and exact verification; no live-pass claim is made here.
Earlier checkpoints below remain historical evidence within their stated scope.

NODE-159: Host Handle creation matches its frozen source identity before enqueueing an unpublished native owner. The existing world witness follows unpublished compensation and published-result cleanup. Locator policy and unselected snapshot capacity remain unchanged.

NODE-157: Optional host Handles reuse this Frame/world accessor and its native
retirement registry, adding 64 admitted snapshots only when handles is selected.
Snapshots retain the existing 8192-byte allowance and registry accounting; no
second world registry is installed. The exact Handle source-world identity must
match before its native lifetime may justify release cleanup after parent
destruction. Locator behavior and unselected snapshot budgets are unchanged.
[Host release integration and verification](pwright-node-handles.md#node-157-host-release-coordinator-and-parent-retirement-evidence).

NODE-133: Retained paired/utility-world cleanup now observes actual destruction of its frozen owning Page, including after routes have disappeared. A generic detach is still not world-death evidence, and destruction updates do not wake user code or replace native command settlement. Snapshot and registry storage remain admitted.
[Native Page world-retirement contract and evidence](pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

# Locator evaluation and temporary element ownership

NODE-044 shares the existing world-access installation with opt-in host drag.
Only `drag-runtime` increases snapshot admission to 1032 (32 Pages * 32 worlds
plus 8 Locator executions). Host shutdown continues topology receipt processing
while Mouse owners retire, before the existing Locator session teardown.
[Integration](pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration).

NODE-043 adds an opt-in host `drag-source`: fresh Page utility snapshots now
remain owned through movement results, retirement and quarantine. Exact world
destruction drains started listener requests without issuing new cleanup there.
Default/selected host closures are **234/235**; only the drag leaf is added.
HostSession Mouse/close/Escape selection and public differential checks remain
pending; the addon and public **36/41** baseline are unchanged.
[Production source and evidence](pwright-chromium-drag.md#node-043-retained-production-page-drag-source).

NODE-042 adds immediate, budgeted snapshots of a Page's existing utility worlds,
including child Frames/OOPIFs without waiting for a main world. Utility-only
native destruction receipts retain snapshot admission and remain distinct from
route invalidation and owned-session retirement. Production drag-source/host
integration is still pending; no addon rebuild or public **36/41** change.
[Snapshot ownership and verification](pwright-chromium-drag.md#node-042-existing-page-utility-world-snapshots).

## Current Node FFI status

Public `Locator.evaluate` now uses the host-owned Rust command. The Node-only
conversion registry retains an admitted callback until Rust preparation, sends
only an integer notification across threads, preserves later binary copying,
and retains conversion exceptions until native cleanup completes. The host now
seals admissions and retires its exact owned inspector sessions before draining
abandoned Locator work. The final 23-case lane passes with supervisor exit 0,
including Worker termination with a borrowed default-Context Page whose user
Promise survives and can be resumed. NODE-027's reproduced cleanup failure is
fixed; NODE-013 attachment timeouts and NODE-014 window dimensions still prevent
a broad compatibility claim. Exact Context disappearance alone is insufficient.
The following older Rust-only checkpoints are historical; see the current
[FFI ownership, resources and evidence](../node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

## Native cancellation boundary evidence

NODE-027 now has explicit native boundary regressions: acknowledged group
release does not settle an outstanding by-value evaluation and a late native
exception can create a new RemoteObject. Detaching an owned debugger session
preserves a borrowed Page and its user Promise. These tests do not supply a
production retirement receipt or prove physical heap reclamation; see
[the evidence and remaining receipt obligations](../node-ffi-compatibility-design.md#node-027-native-cancellation-boundary-evidence-2026-09-07).

## Receipt-driven evaluation implementation

The independent leaf now has opt-in `evaluate_with_retirement` and a
single-consumer `NativeEvaluationRetirement` receipt. Acknowledged native
retirement ends the outstanding wait but still requires independent group
cleanup evidence. The group proof has a defaulted object-retirement predicate,
so session retirement does not claim world destruction. A real native test
drains cancelled Locator cleanup after an exact owned-session detach while
preserving and subsequently resuming the borrowed Page's user Promise. Host
shutdown now selects this path, with an in-flight release fence for detach-ack
races; [host integration and bounds](../node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

## Pinned upstream contract

The installed Playwright 1.58.2 oracle, not the newer upstream checkout, is the
reference. `lib/client/locator.js` implements `evaluate` through `_withElement`:

1. wait for a strict, attached element using the Locator timeout;
2. evaluate through that exact ElementHandle in the main world;
3. dispose the temporary Handle in `finally`.

The timeout passed to `Locator.evaluate` limits element resolution. It is not
forwarded to `ElementHandle.evaluate`, so it is not a deadline for arbitrary
user JavaScript after resolution. Resolution must not require visibility,
stability or input actionability. A selected node may become detached before
the user function runs; re-resolving would change the selected object and must
not replay arbitrary JavaScript.

Function-source and argument serialization happen when `h.evaluate` runs,
after the asynchronous resolution, not at the initial Locator call. Reusing
the Node Page.evaluate entry's eager structural snapshot would therefore change
observable getter/mutation timing. The existing later binary-copy phase must
also remain aligned with the actual evaluation dispatch. This needs dedicated
paired tests before exposing the Node method.

## Chosen composition

Locator resolution is independent of ordinary Locator reads/actions and value
evaluation. It must not make a count/text-only consumer install retained Handle
support. Two new opt-in leaves provide the first required connection:

- `pwright-backend-cdp-locator-resolution`: one exact-world selector attempt;
- `pwright-composition-chromium-locator-resolution`: pre-admitted continuation
  cleanup around that attempt and its temporary owner.

The backend reuses the typed selector model and shared selector compiler/runtime.
It returns no element for zero matches, a strict error for multiple matches,
and a non-cloneable `ResolvedElement` for one match. It neither waits/retries nor
evaluates user code. The supplied route must be the current utility world;
main-world adoption and topology revalidation remain separate composition work.
Runtime installation is idempotent, but its bounded source is currently sent
with each attempt; no hidden installation cache or polling task is introduced.

The new CDP primitive calls `Runtime.callFunctionOn` with `uniqueContextId`,
`returnByValue: false`, `awaitPromise: false` and a caller-owned `objectGroup`.
Groups use a never-reused injected ObjectId source. Every resolver sharing a
native session must share that source; replacing it with a fresh counter while
the session survives is invalid. No wall clock, randomness or process ID is
used by core logic. Results/exception objects belong to that group, allowing
`Runtime.releaseObjectGroup` even when response parsing fails or no usable
object ID arrives. Generic CDP clients fail explicitly if these methods are
not implemented; existing evaluation methods retain their previous behavior.

## Resources and cancellation

`ResolutionProfile` requires a positive capacity and a declared production
ledger. Each active attempt first reserves one `RemoteHandle`, owner bytes,
and selector scratch; compilation and native I/O happen only afterward.
Existing selector bounds remain depth 32, 256 nodes and 64 KiB aggregate strings.
Native IDs and utility-world identities are bounded at 1,024 bytes.

Owner bytes are `2 * 1024 + 64 + 256` (2,368). Scratch bytes are derived from
actual selector-runtime/compiler maxima and the native Selector type size:
`8 * (MAX_COMPILED_SELECTOR_BYTES + SELECTOR_RUNTIME.len() + 1024)
+ 256 * size_of::<Selector>() + 2 * 65536`.
The factor covers simultaneous compiler, install, JSON-escaped and request
copies. One capacity unit has two leases during resolution and one after
success; scratch returns before the caller receives the selected element.
CDP response envelopes and caller-owned routes have their separate transport/
topology budgets. These declarations are not a renderer-heap or process-RSS proof.

The composition reserves one caller-owned cleanup slot before invoking the raw
backend. No task or executor is selected by either leaf. Its ownership rules are:

- unpolled resolution drops without browser work;
- dropped started resolution finishes that same attempt and releases its group;
- a returned owner retains its original cleanup reservation;
- dropping the owner, or an unpolled/started release future, transfers cleanup
  without reissuing the native request;
- failed cleanup is reported to the executor rather than swallowed.

An armed group guard also closes admission if a raw owner/started attempt is
abandoned, the cleanup executor discards its job, or native release fails.
The terminal health flag is shared by resolver clones and checked again before
dispatch. There is no reset method. The host must abandon that native session
before installing a fresh resolver; zero local leases after a failed release
is not proof that Chrome released the remote object. Concurrent operations
already dispatched remain bounded by the declared capacity.

## Verification checkpoint

Release tests pass 15 backend and 10 composition cases, including count/strict
results, capacity-before-I/O, invalid setup/selector/native identity, group
cleanup on query/response failures, non-reused IDs, dropped futures/owners,
failed cleanup and discarded continuation jobs. Strict Clippy passes.

The final dedicated Chrome 152 test passes with a test-owned Context. It locates
an associated label and a Shadow DOM role despite a main-world querySelectorAll
override, retains the exact element after detachment, proves both explicit and
drop-driven release invalidate the actual native object ID, and verifies zero
local leases/reservations after successful cleanup. Missing and strict cases
also pass. The externally owned browser survives. Logs are in
`.work/node-locator-resolution.yiEN25/guard-{units,clippy,live}.log`.
An initial live-test compilation selected the wrong event-receiver type; that
failed log is retained separately, not counted as a browser test result.

`bash scripts/check-v2-locator-resolution-dependencies.sh` proves exact normal
closures of five backend and seven composition Pwright packages. They contain
no aggregate Chromium root, provider, capability/codec/client/server family,
input service, evaluation service or Node addon. Node does not select these
new leaves yet; its existing closure remains 221 normal packages.

Shared CDP/fake-client release tests pass 67/23, and workspace release check,
format/diff checks and all 30 existing umbrella profiles pass. Rebuilt Node
regressions retain 32/32 Locator, 38/38 evaluation, 45/45 lifecycle, 25/26 actions
and shared 41/14/2; the new leaves are not installed by that Node profile.
See the [artifact record](../node-ffi-compatibility-design.md#locator-evaluation-ownership-prerequisite-2026-09-07).

## Remaining gates before Node Locator.evaluate

- retain the now-passing NODE-026 acknowledged Context-close regression while
  validating broader unacknowledged context/session-loss paths; route invalidation
  or cancellation acknowledgement alone is still not native cleanup;
- delayed Node source/argument conversion, admitted references, exceptions,
  Promise completion and Worker teardown;
- paired detached-node, timeout-phase, mutation/getter, close/navigation and
  error comparisons; only then enable the unchanged shared label observations.

The query, waiting and adopted-element evaluation primitives are implemented;
the safe host now installs them in one controlled command as described below.
Node `Locator.evaluate`, `evaluateAll`, public
ElementHandle/JSHandle and the broad compatibility goal remain incomplete.

## Strict waiting checkpoint (2026-09-07)

`StrictResolutionWaiter` is now an optional API of the existing composition
leaf. It does not install another crate, event driver, timer runtime or task.
The caller supplies a bounded `ResolutionRouteSource`, a `DeadlineScheduler`
for pacing, the continuation-safe attempt service, and one termination future.
The termination future represents the original command deadline and lifecycle
signals; it is polled before work on every wake and never recreated per retry.
The caller may lend a pinned future so that adoption can keep that same deadline.
Returning a selected element does not make that deadline govern user evaluation.

The route source must represent the same Frame and exact utility world, wait
for replacement-world readiness, and report closure/crash explicitly. Before
dispatch and after a result the waiter checks authoritative route validity.
A successful stale-world response is explicitly released before retry. A CDP
failure is retried only when the source proves that exact session/world retired;
strict, selector, malformed-response, admission and cleanup failures stay
terminal. The source is not a generic retry predicate based on error strings.
An invalidation error after success drops the unpublished owner into its
reserved cleanup slot. No result is silently changed to a different node after
the waiter returns it.

Pacing is injected as six durations; the tested profile uses
`[0, 20, 50, 100, 100, 500]` ms after the initial immediate attempt, matching
the pinned oracle's initial retry and capped backoff. Only the first retry
may be zero. An unavailable scheduler fails explicitly when pacing is needed.
There is no visibility, stability or enabled-state requirement for attachment.
Timeout/drop during a native query or stale-owner release continues just that
already-dispatched operation in its reserved cleanup slot, not the wait loop.

Each installation holds a 256-byte state lease for its shared admission counter.
Each waiter reserves one lease covering 1,024 bytes of internal state plus
`2 * (256 * size_of::<Selector>() + 65536)` selector bytes for the original and
one command clone. Aggregate retained string capacity, not just text length,
must fit 64 KiB. Failed admission drops the owned input before returning its
future. Profile requirements contain one scope and `capacity + 1` leases.
Caller-owned route storage and injected future state retain their own budgets;
the backend's compilation/group leases and executor slots remain separate.
No process-RSS claim follows from these accounting bounds.

An independent atomic capacity limit is necessary in addition to the shared
production ledger: its per-class aggregate ceilings include other installed
components and do not enforce a particular waiter's concurrency limit. The
counter is shared across clones, rejects before route lookup, and rolls back
on failure/drop. A capacity test caught the initial ledger-only implementation;
it now verifies both clone rejection and subsequent admission after release.

Evidence in `.work/node-locator-wait.HOnILm/`:

- `units-complete.log`: 15 backend and 32 composition release tests pass
  (22 new waiter tests, including deadline, route replacement, close, exact
  capacity, retained allocation size and borrowed deadline ownership).
- `live-final.log`: both dedicated Chrome 152 tests pass. The original group
  test takes 0.21 s and the new waiter test 0.48 s. The latter uses one owned
  Context, delayed hidden/disabled content, a real document navigation with
  CDP-observed world replacement, absent-selector timeout, strict violation,
  target close, detached-object identity and physical group invalidation.
  Both dispose their test Contexts, verify zero remaining leases/slots, and
  leave the external browser alive. The test's single-Frame route driver is
  not the pending production topology adapter.
- `clippy-complete.log`, `dependencies.log`, `workspace.log`, `umbrella.log`:
  strict Clippy, five/seven-package closure proofs, workspace release check
  and 30 umbrella profiles pass. Node boundary remains 221 packages;
  offline 9 tests and type check pass.

The first type-check passes needed type/lifetime annotations. A mechanical
test-helper replacement then accidentally made the lease helper recursive;
that test process was terminated and the helper corrected (`units.log`).
These intermediate failures are not passing evidence. Later complete unit,
Clippy and real-browser logs above are authoritative.

Node does not select this API yet. No addon rebuild or new five-lane Node
differential result is claimed here: its SHA-256 remains
`db8f7337a628d4424e4ed252d8e2e27ed465b07a17878f72fcd6b0f6e6b7ea84`.
The previous 41/14/2 shared result and NODE-022 action discrepancy remain open.

## Main-world adoption and evaluation checkpoint (2026-09-07)

The new opt-in `pwright-composition-chromium-locator-evaluation` leaf consumes
an existing `OwnedElement` and a caller-supplied `MainWorldRoute`. Its
`LocatorElementEvaluationService::adopt` validates a positive numeric context,
a nonempty unique context ID of at most 1,024 bytes, and the exact shared CDP
client before native adoption. It returns a non-cloneable `MainWorldElement`;
adoption does not receive function source or execute user code. The caller must
cover this phase with the original resolution deadline and topology lifetime.

Adoption reuses `CdpElementHandleAdoptionService`: describe the already selected
node, then resolve its backend node into main world. The new optional
`objectGroup` parameter puts the adopted wrapper in the **original** temporary
group. Both wrappers and unknown objects from malformed/lost resolve responses
are therefore covered by one group release. This requires another pre-admitted
native Handle unit, rather than pretending two wrappers count as one. The raw
adoption service's default path remains unchanged and does not select groups.

`MainWorldElement::evaluate(&EvaluateParams)` consumes the owner and validates
borrowed caller input before copying it into the native future. This is the
point where the future Node binding can convert JS source/arguments after
resolution/adoption. The implementation uses the existing
`CdpHandleValueEvaluationService` and Rust value serializer, with a native-only
receiver rather than an invented portable Handle ID. Numeric context IDs are
used for DOM adoption; the actual user Runtime call uses the stable unique ID.
No selector is sent again, no main-world DOM-query fallback exists, and the
method cannot replay the user function after a navigation or other failure.

The temporary group is retained through successful values, JavaScript failures,
Promise completion and cancellation. An unpolled evaluation is discarded with
cleanup only; a started evaluation transfers that exact future, not a new
invocation, to its reserved caller executor. A prepared owner and unpolled or
started explicit release have the same cleanup protection. Failed group release
retains the primary failure, closes admission across installation clones, and
does not claim native reclamation from zero local counters. Discarding the
executor's continuation also closes admission. The host must abandon the native
session before replacing a poisoned installation; production integration still
needs authoritative context/session-retirement proofs so already-destroyed
worlds are not mistaken for unproven cleanup. No error-string heuristic is added.

Resource profile for capacity `N`:

- fixed installed state: 256 backend bytes and one lease;
- each prepared/evaluating owner: one additional `RemoteHandle` and an owner
  byte allowance of `L.maximum_retained_bytes_per_operation() + 8192
  + 4 * L.value.max_nodes() * (size_of::<EvaluationValue>()
  + size_of::<HandleEvaluationValue>())`;
- the shared value-evaluation factory separately declares its existing
  `N * L.maximum_retained_bytes_per_operation()` execution peak;
- profile requirements conservatively combine the two scope declarations and
  `2 * N + 1` leases, although both use the supplied resource scope;
- original resolver ownership, route-source storage and two reserved cleanup
  slots remain separately owned. The extra native unit and owner allowance
  survive until group release completes, including an unknown adopted ID.

All arithmetic must fit the declared ceilings, including the simultaneous owner
and evaluator sum. Input source is bounded before cloning; value validation
precedes argument copying. The owner allowance covers the retained value while
group cleanup is pending. These are explicit host-side accounting bounds, not
a Chrome renderer-heap or process-RSS measurement.

Release tests pass 24 new cases, with strict Clippy. Real Chrome 152 proves
main-world marker access despite a main-world querySelectorAll override,
label-selected identity after detachment/replacement **before** evaluation,
BigInt/TypedArray transport, structured TypeError, no execution from an unpolled
future, and one execution from a cancelled already-dispatched Promise. Both the
utility and main native IDs become physically invalid after group cleanup.
The test owns and disposes one Context; its cleanup wait is bounded and the
external browser survives. Final evidence is
`.work/node-element-evaluation.D9og5u/{units-guard,clippy-guard,live-final}.log`;
the final live case takes 0.34 s. Early live-test compilation errors (helper
visibility and a Poll/Result return mismatch) are retained separately.

`bash scripts/check-v2-locator-evaluation-dependencies.sh` proves an exact
15-package normal Pwright closure, with no aggregate root, provider, portable
Handle lifecycle, codec/server or Node. Resolution-only closures remain five/
seven packages; ordinary Locator consumers still do not install evaluation.
Shared backend Handle tests (17), CDP tests (67), fake tests (23), resolution
tests (15/32), workspace release check and all 30 umbrella profiles pass.

Node does not install the new composition yet. Its shared CDP/Handle changes
were rebuilt and all five differential lanes rerun with addon
`42a26fd0b5325c393b930a89f18dc628959c11bfa884e293756fb19c9d734e94`:
Locator 32/32, evaluation 38/38, lifecycle 44/45 (NODE-014 reproduces), actions
25/26 (NODE-022), and unchanged shared 41 matches / 14 unsupported / 2 oracle
errors. No additional shared label match or public Locator.evaluate support is
claimed. See [the complete artifact record](../node-ffi-compatibility-design.md#locator-main-world-evaluation-checkpoint-2026-09-07).

## Production paired-world access (2026-09-07)

`ChromiumFrameWorldAccess` now provides an independently installed, bounded
access port over the production `CdpPageServices` route table. It requires
explicit world readiness, value-evaluation routes and utility-world Locator
routes. A caller merges `ChromiumFrameWorldAccessProfile::requirements()` into
the attach profile's extension requirements and installs the accessor with
that same production ledger. Default profiles do not install an accessor.

`snapshot(FrameId)` admits synchronously, then waits for both observed worlds
under the existing topology commit notification. It accepts only the exact
Frame: a Page ID is rejected, a local child uses its own world IDs, and an
OOPIF pair must have matching target/session ownership. The returned
`ChromiumFrameWorldSnapshot` is non-cloneable. Its one client must be reused
for both utility selection and main-world adoption, even when independent
capability factories originally constructed different client wrappers for
the same native session. No second topology table, event subscription, driver,
timer or polling task is introduced.

`is_current()` reads both world identities and their client in one route-table
transaction. Either world changing retires the pair. Once observed retired,
the snapshot cannot become current again if a prior route reappears. The
caller-polled `wait_invalidated(&mut self)` uses the existing bounded readiness
waiters, checks before waiting, and registers notification before rechecking;
one mutable borrow prevents duplicate invalidation waits on a single snapshot.
Frame/Page removal and crash remain terminal errors, not successful readiness.
The host must still supply its transport/Worker shutdown signal.

The production topology now exports world command routes only when their
session equals `frame_session_id(FrameId)`. During renderer handoff or a swap
gap, old world records may remain in the native inventory, but they cannot
be selected as current routes. A late execution-context announcement from the
previous renderer is ignored without overwriting the current world's record.
This is not native destruction evidence: group cleanup must not be skipped
just because an accessor reports a route as retired.

Resource contract for snapshot capacity `N`:

- installed state is `size_of::<State>() + 256` backend bytes, derived from the
  actual owner type, including the retained service factories/profiles;
- every pending or returned snapshot reserves 8,192 backend bytes, covering
  snapshot/future state and up to four 1,024-byte native routing identities
  retained beyond current table replacement;
- the profile declares one scope, `N + 1` leases, and the simultaneous sum;
- one installation-wide atomic counter bounds both unpolled and returned
  snapshots across accessor clones; a larger shared ledger cannot bypass it;
- readiness waiting separately uses its existing 512-byte waiter allowance;
  cancellation releases that waiter but preserves the returned snapshot's
  allowance until the snapshot itself is dropped;
- the accessor and surviving snapshots retain the original service owner and
  all of its route leases. Dropping the last accessor cannot leave a snapshot
  holding an unaccounted route table. Cloned native routing values remain
  borrowed in ownership terms: keep the snapshot alive while using them.

This is a coherent snapshot of **observed worlds**, not an atomic browser
document lease. Chrome can navigate before its event reaches the driver.
The selection/adoption phase must revalidate on both sides of adoption,
handle native failures under the original deadline, and retain explicit group
cleanup. A finished adoption must not cause later user code to be reselected
or replayed. The accessor is not yet wired into the Node preparation/settlement
pipeline; controlled-runtime integration, deferred conversion and the public
Locator.evaluate conformance gate remain open.

Release verification passes 40 topology and 247 Chromium composition unit
cases (15 browser cases remain explicitly ignored in the ordinary unit run).
This increment adds 20 access-port cases and three renderer-route cases. One
separately invoked real Chrome test uses the production topology driver to
verify world isolation, real navigation/invalidation, exact local-child
pairing/detachment, rejection of old native worlds, resource return and owned
Context cleanup. OOPIF handoff is deterministic unit evidence, not live OOPIF
coverage. Strict Clippy, workspace check, all 30 umbrella profiles and exact
five/seven/fifteen-package leaf dependency proofs pass. Full Node results,
including concurrent-run setup failures and a separate serial experiment, are
kept in [the artifact checkpoint](../node-ffi-compatibility-design.md#production-paired-world-access-checkpoint-2026-09-07).

## Paired selection and adoption phase (2026-09-07)

`preparation::LocatorElementPreparationService` now composes the strict waiter
and main-world adoption inside the existing independent evaluation leaf. Its
`PreparationInputs` injects the existing attempt service, strict waiter,
adoption service, `PairedWorldSource` and retry scheduler. The source supplies
an owning reference to one coherent `PairedWorld`: exact utility/main routes
must share a native client, have distinct valid worlds and represent one Frame.
Both-world invalidation and route storage remain source responsibilities.
The normal leaf closure remains fifteen packages, without the aggregate root.

The operation preserves these phase boundaries:

1. Admit one preparation and validate the original selector before cloning it.
2. Strictly wait in the exact utility world, retaining that pair through every
   native attempt. A private one-slot adapter bridges the pair to the existing
   strict waiter; it is not another topology table or event subscription.
3. Verify the selected element's client/world against the retained pair and
   revalidate both worlds before adoption. Adopt into that pair's main world.
4. Revalidate after adoption. A stale successful result is explicitly released
   before retry; only a native Backend failure with independently observed
   retirement may retry after the adoption leaf acknowledged group cleanup.
   Strict, malformed-protocol, setup, admission and cleanup errors stay terminal.
5. Return one `MainWorldElement`. Resolution is over; subsequent source/argument
   conversion and single-use evaluation are separate. The resolver never
   reselects the element or replays user JavaScript after this return.

One caller-owned stop future is polled first throughout route readiness,
selection, adoption, stale cleanup and retry pacing. It is never replaced or
restarted. Native cancellation transfers only the dispatched query/adoption/
release to its pre-admitted continuation, not the retry loop. The same deadline
does not govern later arbitrary user evaluation. Transport/Worker and runtime
target-lifecycle policies still belong to the eventual host integration.

### Route ownership through cleanup

A cancellation audit exposed an ownership gap: keeping a paired snapshot only
in the awaiting preparation could return its budget while the cancelled native
query was still running in a cleanup continuation. `ResolutionRouteSource`
therefore has an optional `retain_route` hook, and the continuation composition
has `call_retaining`. Stateless existing sources keep their old behavior.

For a budgeted pair, the owning Arc follows the unpolled/started attempt, the
returned `OwnedElement`, explicit release and every cleanup continuation. The
adoption/evaluation leaf already retains that source owner, so its pair now
survives through both native wrappers, user evaluation and group release too.
Cloning the Arc does not admit another snapshot. Dropping the preparation,
its route adapter, or the caller's accessor cannot return the original route
allowance early. If an executor discards cleanup or native release fails, the
existing raw-owner health guard still closes admission; zero local counters
are not proof of native cleanup. Typed route-source errors can now propagate
as `ResolutionWaitError::Route` rather than being mislabeled as CDP failures.

### Preparation resource contract

For capacity `N`, the separately selected profile declares one scope and `N+1`
leases, 256 installed backend bytes plus `N * 8192` preparation backend bytes,
and `N * (256 * size_of::<Selector>() + 65536)` selector bytes. That last
allowance retains the original retry selector; the strict waiter separately
covers its input and attempt clones. Clones share one atomic admission count.
The profile requires a positive injected retry delay, rejects overflow, and
does not change any default capability profile.

Concrete async state is checked before heap allocation: the generated operation
plus its result wrapper and a 1 KiB private route-adapter allowance must fit
8,192 bytes. The private route future is separately checked against that 1 KiB
before boxing. An oversized injected stop or inline source future explicitly
rejects before route/browser work; rejection does not allocate the large box.
The first 4 KiB assumption failed a test at 4,304 bytes and was replaced with
these executable guards, not merely an unchecked larger estimate. Source-owned
pair/notification/error storage, strict waiting, native resolution/adoption/
evaluation and cleanup execution retain their independent budgets.

Release tests now pass 37 resolution-composition cases and 41 evaluation-
composition cases: five new retention cases and seventeen preparation cases.
Both ignored Chrome cases run separately, including a new real production-
driver/paired-port integration. It selects a delayed hidden label target,
adopts before evaluating, preserves the detached original despite replacement,
reads main-world state, executes once, proves both native IDs valid before and
invalid after cleanup, returns the snapshot allowance and times out a missing
selector. The assembler adapter is currently in the test fixture; the Node
host has not installed the phase or its typed controlled-runtime commands.
See [the complete checkpoint](../node-ffi-compatibility-design.md#paired-element-preparation-checkpoint-2026-09-07).

Remaining integration is still material: production host assembly, controlled
command/target admission and lifecycle retirement, bounded delayed Node-API
references/settlement, explicit disposal on conversion exceptions and paired
public behavior/error tests. In particular, a destroyed CDP session can make
group release fail; route invalidation alone must not turn that error into
successful cleanup. Node Locator.evaluate is not exposed by this checkpoint.

## Shared controlled dispatch prerequisite (2026-09-07)

The runtime's new `dispatch_with` accepts an explicitly installed typed handler
and cancellation implementation while preserving the same graph, pending calls,
Frame/connection scheduling, deadlines and abandoned cleanup executor as ordinary
commands. Nine new deterministic tests cover actual sharing and lifecycle effects.
It does not register a wire capability or provide another resource pool.

A real Chrome preparation fixture uses the production controlled runtime and
paired-world port. It checks both native references before/after explicit group
release, a queue deadline while an existing value-evaluation holds the Frame
lane (no preparation backend or snapshot admission), and a started missing-
selector deadline followed by native cleanup drainage and health checks. All
three leaf Chrome cases pass serially. The normal closure remains 15 packages.

The command adapter remains test-only: its escaped owner is explicitly released,
not governed by host lifecycle after return. Production error mapping, native
retirement and deferred FFI remain required. Node does not install this phase;
its shared-runtime rebuild's full regression fails from independently reproduced
attachment event pressure (NODE-025). See [the complete evidence](../node-ffi-compatibility-design.md#controlled-preparation-dispatch-and-attachment-pressure-2026-09-07).

The subsequent Node rebuild explicitly selects conservative world-event
admission, avoiding retained leases for well-formed foreign worlds. Its live
Rust attach probe passes, but five-lane Node regression still contains attach
timeouts, a lifecycle watchdog kill and NODE-022. This prerequisite does not
install the prepared-owner phase or expose public Locator.evaluate. Production
runtime ownership, native retirement and deferred FFI conversion remain open.
See [the scoped event-admission checkpoint](../node-ffi-compatibility-design.md#optional-world-event-admission-checkpoint-2026-09-07).

## Host-owned evaluation continuation (2026-09-07)

`HostLocator::evaluate_with(timeout_ms, convert)` now installs the real paired
world accessor, strict selection/adoption and single-use evaluation in the Node
host profile. The Rust `FnOnce` conversion future produces owned EvaluateParams
only after successful preparation. No Node-API value enters the host. Public
Node bindings and their admitted reference/exception/binary staging were pending
at this checkpoint; the current FFI integration and open cleanup gate are above.

The earlier escaped prepared-owner handoff is replaced by **one controlled Frame
command** containing all phases. The selected element never escapes to a JS
token or outlives a completed runtime call. Existing graph, target/connection
admission and lifecycle signals cover conversion, user evaluation and release.
No portable/wire capability, task or prepared-owner registry is added.

One host-side phase deadline starts before polling controlled dispatch, covering
queueing and selection/adoption. It ends exactly when preparation succeeds;
conversion/evaluation retain lifecycle cancellation but not the Locator timeout.
The callback runs once and is never called for missing/strict failures. The
native CDP command timeout remains a separate limit, not unlimited execution.
Conversion failure explicitly awaits group release before returning its typed
error; cleanup failure remains visible. Cancellation transfers only the actual
native continuation to pre-reserved cleanup, checked again at host shutdown.

The explicit profile has eight executing slots and sixteen pending calls. Six
scope IDs (120,10–15) cover resolution, adoption, waiting, preparation, paired
access and host state. Fixed host state is 8,192 bytes. Per-call allowance is
2,490,368 bytes on this target, derived from two bounded value trees, source,
two 4 KiB conversion states, a 16 KiB command future and 8 KiB metadata. Inline
callback/future sizes are checked before dispatch and command state before
boxing. External Node references and caller-owned heap captures stay separately
admitted; query/native/route ownership has its existing independent leases.
The initial guessed 2 MiB allowance failed at 2,478,080 bytes. Its replacement
is formula-derived; the failed bound assertion was retained.

Host cleanup capacity is explicitly 32 instead of 16: its previous runtime/
lifecycle allowance plus two native reservations per eight simultaneous
evaluations. No CDP event capacity changes. Three new normal packages enter the
host closure (now 224): resolution backend, resolution composition and evaluation
composition. Independent 5/7/15 closures and all 30 umbrella profiles still pass.

Evidence: `.work/node-owned-locator-evaluation.AbFw0k/`. Nine new deterministic
host cases pass (77 total, five live ignored): phase timing, shutdown/drop,
input/JS errors, admission and rollback. Strict Clippy and workspace check pass.
The first live run passed 4/4 in 1.55 s; its cancellation case was then renamed
to accurately say caller cancellation rather than host shutdown. The expanded
real Chrome run is **4/5, not green**, in 1.60 s:

- Utility selection despite poisoned main-world querySelectorAll; original
  detached identity/main world after conversion exceeds the resolution timeout;
  runtime call stays live until evaluation and release finish.
- Missing/strict never invoke conversion; conversion/structured JS errors clean
  up before returning.
- Eight prepared commands occupy the existing lanes; a ninth times out before
  conversion, then all eight finish and release their groups.
- Caller cancellation releases ownership without callback completion.
- **NODE-026 (initial failure, fixed below):** Context close cancels a prepared command with TargetClosed,
  but subsequent host shutdown reports native cancellation cleanup failure.

The production Context-close service awaits native Target.disposeBrowserContext
before committing route/runtime disposal. A selected group can therefore lose
its native session before drop-driven release. Cancellation acknowledgement is
not physical cleanup. Next work must connect temporary groups to authoritative
parent/world/session destruction or correctly ordered bounded cleanup. No
session-error string is accepted as native destruction proof, no failure is
suppressed, and this failing test remains a public Node-method release gate.
See [the rebuilt artifact record](../node-ffi-compatibility-design.md#host-owned-locator-evaluation-checkpoint-2026-09-07).
This describes the initial failure; the following checkpoint fixes that gate.

## Optional native-destruction receipts (2026-09-07)

The Node host explicitly selects `with_native_retirement()` on its paired-world
access profile. Default profiles have no registry or receipt. The registry holds
at most `capacity` Weak receipts; live receipt clones retain the original
snapshot permit even after the snapshot is dropped. It adds neither a task nor
a subscription. Lookup/registration and native-destruction marking are ordered
by the existing prepared route transaction, before cancellation publication.

Each identity also retains its connection epoch. Paired snapshot validity checks
that epoch, and destruction uses the prepared topology candidate's epoch; an old
close acknowledgement cannot retire a newly reused identity after reconnect.

Accepted evidence is an acknowledged owned BrowserContext disposal, an exact
session/world unique-ID destruction, or that session's execution-context clear.
The two-world receipt is complete only when both selection and adoption worlds
are dead. Numeric IDs, route replacement, detach/crash, failed transport and
error text are not evidence. Late events ignored by the current driver remain
conservative cleanup failures; this is not general disconnected-session recovery.
The underlying protocol defines
[world destruction/clearing](https://chromedevtools.github.io/devtools-protocol/tot/Runtime/#event-executionContextDestroyed)
and [Context disposal](https://chromedevtools.github.io/devtools-protocol/tot/Target/#method-disposeBrowserContext).

An optional caller-budgeted `ObjectGroupDestructionProof` travels from the
resolution route through requests, temporary owners and cleanup continuations.
The raw resolver skips release with confirmed proof and rechecks after a failed
in-flight release. Unconfirmed failure still poisons admission. The Context
adapter retains its idempotent public close behavior but exposes separate
AcknowledgedDestruction / AlreadyClosed outcomes internally; only the former
can issue a receipt. No browser/backend semantics move into JavaScript.

Extra installed bytes are `size_of::<Registry>() + 256 + capacity *
(size_of::<Weak<Receipt>>() + size_of::<Receipt>() + 32)`, with checked arithmetic.
Expired Weak allocation metadata remains charged to installed state until the
next registration prunes it. The route table retains that installed-state lease.
The existing 8 KiB snapshot allowance covers its permit, receipt and up to five
bounded native-ID allocations; its retained permit enforces count/byte limits
through the last proof clone. No extra scope/lease/task count is added.

The former NODE-026 real Context-close failure now passes in two full 5/5 host
runs plus an isolated repeat. Final epoch-guarded sources pass 442 release cases;
their host run is 4/5 because the queue fixture fails during 5 s connection
initialization, before any queue assertions. Context cleanup passes again, and
a separately recorded isolated queue diagnostic succeeds. This does not resolve
NODE-013 attachment timing. Deterministic tests cover exact proof identity,
partial destruction, stale routes, retained permit/cycle-free release and a
native release that fails while destruction is observed. See
[the artifact and regression record](../node-ffi-compatibility-design.md#native-destruction-receipt-checkpoint-2026-09-07).
Public Node source/argument staging, settlement and Worker teardown remain next.
