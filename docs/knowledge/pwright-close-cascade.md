<!-- agent-updated: 2026-09-11T02:46:34Z -->

NODE-159: Safe-host Frame Handle creation now publishes through the same driver boundary used for release commits. A busy close transaction retains the unpublished native result instead of rejecting or rerunning the script. Queue close/receiver loss submits pre-admitted compensation; shutdown revokes publication before draining native work. See the Node Handle checkpoint for exact scope and verification.

NODE-157: Embedded native closes now use queue-free deferred runtime reservations. Handle completions commit only at the driver's local boundary and continue through shutdown. Failed release during parent destruction requires exact native-world retirement plus local object retirement; a detach alone cannot produce a successful cleanup claim.

NODE-156: Optional native Handle disposal can now complete browser I/O without a runtime gate and return exact-route evidence for a later short commit. Parent retirement leaves old receipt-owned resource leases charged; superseded completion never touches the replacement runtime object. Host driver/shutdown coordination is still pending.

NODE-155: Runtime disposal reservations can now explicitly select a queue-free runtime and return unchanged admission on a busy prepare boundary. The legacy outbox profile and consuming prepare behavior are preserved. Native Handle release receipts and shared driver integration remain pending; Page/Context close coordination is not claimed fixed.

NODE-154: Pending Handle cleanup now has a host-local bounded turn after a real-browser failure showed the legacy global prepared lifecycle transaction rejects concurrent disposals. This is not a shared lifecycle coordinator: overlap with Page/Context/topology work and late Handle creation publication must still be verified/coordinated. Session close drains pending Handle native work before detaching Locator inspector sessions; descriptors may survive without retaining that work fence.
[Pending Handle evidence and remaining mixed-lifecycle gate](pwright-node-handles.md#node-154-session-owned-pending-results).

NODE-140: Concurrent ancestor dispatch remains distinct from single-Page close progress. Full arrival-boundary close tests fail on candidate and upstream; an oracle body-release control still requires later Context cleanup to settle Page.close. A passing candidate repeat does not establish a fix. No close retry, synthetic native death or new timeout policy was added.
[Expanded verification and retained failures](testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

# Native close cascades: intent, receipt and commit ownership

NODE-139 correction: the previous natural Frame.goto error comparison used
different native document commit phases and is not a proved same-phase semantic
discrepancy. A separate Rust fix retains actual Page lifetime through pending
navigation publication; close intent alone does not rewrite a live error.
[Page lifetime publication contract and verification](pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

## NODE-138: opt-in Node close-driver integration

The Node host now actually selects the native scheduler and local retirement
composition through `concurrent-close`. The one existing topology task polls
eight native jobs; no per-close task or JavaScript automation core is added.
Admission reserves original topology/runtime/publication capacity before native
dispatch. Each payload owns a fixed 256-ID scratch array and its caller reply.
A fixed eight-entry FIFO parks completions while creation, initialization or
header reconciliation owns a local transaction. Every relevant driver await
cooperates with native progress without cancelling/restarting the awaited work.

At a transaction boundary, a completion consumes its original capacity under
sealed attachment-bound evidence. Local retirement runs inside the host state
lock; its returned notification owner leaves that lock before publication and
cancellation wakes. Native-success/local-publication failure stops the driver.
Dropped callers do not abandon admitted native work. On terminal shutdown, the
driver stops admission and drains written close commands before releasing their
owners or waiting for the host operation tracker; existing native world/Mouse
retirement and final disconnected fencing remain in place.

Selection deliberately covers only owned Contexts and their Pages, because the
ancestor authority requires owned native Context identity. Default/external
Context and borrowed Page paths retain their previous services, including
creation compensation. Compiling the feature without selecting lifecycle events
keeps the scheduler absent and the ordinary runtime outbox disabled.

The selected embedded runtime has a 2304-message control outbox: eight worst-case
256-object close reservations plus another graph of non-close traffic. Storage
is preallocated/admitted before close reservations. The host consumes its graph
directly rather than through a wire client, so the driver drains committed control
messages at admission/reconciliation boundaries; FrameTrees separately owns Node
lifecycle publication. This does not claim complete graph/container allocator
accounting or an RSS ceiling. Scheduler/payload/parked completion storage, native
sources, topology/runtime capacity and outbox backing have explicit profiles.

Two isolated package profiles are available: `concurrent-close` and
`concurrent-close-drag-frame-extra-headers`. Their `concurrentCloseEnabled` binary
marker must match the selected package profile. Default and the earlier combined
artifact are not replaced. Dependency checks now cover nine host and ten addon
selections and still require ordinary profiles to exclude all close coordinator
features. CI includes selected and combined host tests plus addon Clippy.

The first combined artifact (`54d7f8d45324bd88f7485beb7e68d8b434df9e54a229f560c1da6b23fce05bd7`)
passed the original controlled ancestor-dispatch probe: Page.close was pending
with three held native retirement packets, two host/FFI close operations were
admitted, and Context disposal was dispatched before releasing those packets.
Both public closes completed and all host/FFI counters and owned targets returned
to zero. Upstream Playwright 1.58.2 passed the same controlled cases. This artifact
also passed all six native-close-confirmation/interruption/Worker cases.

Do not omit the next failure: `.work/node138-close-driver/candidate-transparent.json`
reproduced a natural Page-first race. A native event had already retired local
Page state, so the delayed native completion had an empty latest topology delta.
The real deferred publication observer incorrectly required a second Dispose and
stopped the attachment (`deferred close plan has no disposal of its admitted target`).
Context.close then failed; this first artifact is not the final passing build.

The correction permits only an empty publication no-op when the admitted target
is no longer visible in the latest planned collection. It does not synthesize a
Dispose, hide a visible target, accept unrelated effects or certify native death;
the composition still validates sealed native proof independently. Regression
tests cover Page-after-Page, Page-after-Context, Context-after-Context empty deltas,
and unrelated disposal rejection, retaining the visible-target empty-plan rejection.
Final rebuilt artifact and browser verification follow.

Final combined addon SHA256:

`02e4d9be871525903927cb6e534627831ba082659cda380d691c18b621f8872f`

The strengthened original probe now asserts successful public Context/Page
settlement, exactly one Page/Context/Browser notice in that order, argument/receiver
identity and closed/connected state. Both upstream and candidate pass both orders
in held-retirement and transparent modes (eight case executions across the pair).
The candidate additionally passes three full held-retirement repeats (six cases).
All candidate reports retain zero host/FFI counters and no owned native targets.
Files: `{oracle,candidate}-{hold-retirement,transparent}-final.json` and
`candidate-held-repeat-{1,2,3}.json` beneath the evidence directory.

Final-artifact differential lanes pass native-close-confirmation 6/6,
lifecycle-events 7/7, browser-connection 5/5, collection-lifecycle 1/1 and
page-is-closed 8/8: 27 test cases, distinct from the close-probe executions.
This covers peer/interruption/Worker teardown and explicit lifecycle regression,
not the entire package compatibility corpus. Earlier failed reports remain intact.

Final Rust release tests pass 473 combined (10 existing ignored) in
`node138-host-rebased.log`, and 425 minimal concurrent-close (8 ignored) in
`node138-host-selected.log`. Host combined/cold and combined addon strict Clippy
pass. JS contracts pass 85; dependency boundaries pass nine host/ten addon
selections. Formatting and diff checks pass. Only the experimental combined
addon was rebuilt; the newly declared minimal addon profile was not built here.
Default and prior combined hashes remain NODE-133's. No commit or push occurred.

One separate error-type difference was visible in the final transparent report:
Context-first interruption returns `PWRIGHT_BACKEND / net::ERR_ABORTED` from
`Frame.goto` in the candidate, whereas upstream returns TargetClosed. The close
gate intentionally checks close semantics, not equality of that navigation error.
NODE-139 subsequently established different native commit phases in a fresh
bounded trace; this is not sufficient evidence of incorrect cancellation
precedence. The independent publication-lifetime boundary is addressed there
without treating local absence or close intent as native death.
Creation/borrowed-context paths remain serial and broader compatibility is open.

At 18:57:09Z the dynamically identified shared backend PID 53189 had RSS
2,079,440 KiB (about 1.98 GiB), versus 3,600,992 KiB at 18:37:29Z; swap remained
zero. A mid-turn observation reached about 4.6 GiB resident and later declined.
These are shared process observations, not per-chat RAM or a leak conclusion.
All task-owned builds, probes, relays and HTTP fixtures finished; the one temporary
Pwright blank tab was closed. Other tasks' shared Chrome/apps/servers were untouched.

Evidence lives in `.work/node138-*.log` and `.work/node138-close-driver/`.
The goal remains active: selection is experimental, ordinary paths are unchanged,
and broader Playwright compatibility and complete peak accounting remain open.

## NODE-137: native-proven local retirement and post-session-lock notification

Inspection of the actual Node driver confirms the remaining serial point in
`lifecycle_services.execute(...).await`. Making those existing services parallel
is insufficient: they prepare topology/runtime transactions before native waits,
and reusing them after scheduler evidence would send another native command.
The new opt-in `native-close-retirement` composition implements the local half
of a completed native close. Node driver admission/polling has not been switched.

`AttachedChromiumComposition::retire_native_close` consumes the original joined
capacity and validates sealed scheduler evidence through the attachment-bound
topology scope. The native Page commit target now borrows its frozen session ID
as well as target ID, avoiding a current-route lookup or copied native string.
The admitted portable target must match the native receipt. Runtime reservation
preparation verifies its exact runtime owner; callers must use the Page runtime
installed on this attachment, not a foreign compatible-looking runtime.

The caller supplies stable, pre-admitted `&mut [ObjectId]` scratch for the latest
runtime subtree, normally a fixed array inside the scheduler's admitted payload.
Insufficient scratch fails before any local retirement. There is no extra ID
Vec, scratch pool or Handle-selection Vec in this path. The compatibility Handle
path reuses the existing retirement/metrics helper with borrowed IDs; the
independent Handle lifecycle leaf adds `borrow_parent_retirement`, leaving its
old owned preparation and default layout intact. Existing topology/runtime
preparation and diagnostic allocation contracts still apply; this does not
complete the outstanding graph/container peak accounting or prove a RAM ceiling.

The selected lock order is network driver, native/topology scope, Handle routes,
Page routes/subscriptions, then runtime state. All fallible local preparation
precedes topology/runtime mutation. Runtime's guarded publication callback swaps
topology, deletes the exact routes and retires Handles before completing its
child-first disposal under the same runtime mutex. Current transport failure and
native authority errors preserve their failure kinds. Exact native world death
is recorded from the validated Page/Context receipt, not from local absence;
this native fact is independent of whether local retirement can finish.

A selected network registry requires the exact matching driver registration.
Unpublished batches and pending adapter transitions reject before the local
callback. After local success, adapters for the exact Page/Context are removed
without allocating/replaying another Request cleanup Vec: their runtime subtree
already supplied Dispose messages. An absent child adapter is valid after a
parent-first commit. Foreign registries and silently omitted registration ports
are rejected. Unrelated adapters, subscriptions and service Arc identities stay
in place; no route factories or native close commands run during retirement.

The API returns `CommittedNativeClose`, not a completed public close. Its lifetime
depends on runtime/scratch, **not the borrowed attachment**, so a Node caller can
leave its outer `LiveSession` mutex scope before notifying. `notify` commits the
Rust publication reservation, then wakes route-readiness/runtime cancellation
waiters. It must run outside every caller lock and under the driver's logical
single-publication ownership. Even publication failure releases already committed
notifications; the selecting host must treat that failure as terminal. Dropping
this must-use owner without notification does not roll back committed state.

The local integration tests use production topology, runtime reservations,
network adapters, Handle leaf and retained world/navigation owners. They invoke
the local phase with explicit native keys and a bounded test observer, not a
Playwright oracle or real Node lifecycle queue. Existing bounded-CDP scheduler
tests continue to validate sealed evidence. New cases cover parent-first joins,
single Request disposal, surviving sibling identities, Handle release, native
world flags/navigation revocation, scratch bounds, target/registry mismatch,
publication/network preflight failure, transport classification and an explicit
post-outer-lock notification owner. Final verification is recorded below.

Evidence is retained in `.work/node137-*.log`. Preserve the first missing-test-
imports build failure and the seven duplicate registry-lane fixture failures;
the fixture already had registry-only lanes before installing real driver
adapters, and that duplicate registration was corrected. The earlier combined
run passed all 12 then-present tests (514 total), before adding the explicit
notification-owner case and sharing the compatibility Handle helper.

Final source verification: `node137-combined-final.log` passes 515 release tests
(16 existing ignored), including all 13 new local integration cases and the
bounded native scheduler/authority cases. The feature set is native retirement,
Frame lifecycle, runtime lifetime handoff and extra headers. The independent
Handle lifecycle regression passes 14 tests with no ignored cases. Final combined
and cold strict Clippy pass, along with exact dependency closure, all seven
host/eight addon boundaries, formatting and diff checks. The earlier standalone
retirement run passed 481 before the final notification-owner test; do not
substitute that earlier artifact for the final combined evidence.

All task-owned verification and bounded peer processes finished. No browser or
persistent server was started, and both installed addon hashes remain NODE-133's.
At 18:27:10Z the shared Codex backend RSS was about 1.00 GiB, system free percentage
67% and swap zero; this is process-level observation, not this chat's unique RAM.

The feature is exactly the existing capacity, route-retirement and native
scheduler prerequisites; an exact dependency proof checks that it adds no other
packages/features. Current Node host/addon profiles explicitly keep it unselected.
CI now includes a dedicated selected test/lint lane. No addon rebuild, browser
result, commit or push is claimed. NODE-123's controlled ancestor-dispatch gate
and full Node-compatible goal remain open.

Next host integration is concrete: declare bounded scheduler/capacity and inline
scratch payload requirements in `profile.rs`; install one caller-polled close
owner after runtime/topology initialization; intercept ClosePage/CloseContext
requests in the driver instead of awaiting their old services. Native jobs must
continue progressing during other driver awaits while completed intents retain
their admission until a short commit is permitted. Consume evidence through this
retirement API, release the outer session lock, notify, reconcile metadata/events
and only then send DriverReply. Stop must drain written native jobs before
`task_operations.wait()`. Keep creation compensation, normal closed-Page
suppression, Context interruption errors and terminal/publication fences intact,
then rebuild both addons and rerun the controlled Page-to-Context dispatch gate.

## NODE-136: joined close capacity and consuming native completions

`ChromiumCloseCapacityBundle`, selected by composition feature `close-capacity`,
reserves topology, runtime/outbox and close-publication capacity in that order.
Failure of a later admission drops all earlier owners. Waiting owns no topology
snapshot or lifecycle mutation gate; descendants can still arrive. `into_parts`
moves the original reservations into latest-state preparation without acquiring
replacement capacity. The original portable target is retained, but neither
capacity admission nor parts extraction is evidence of native death.

This is an optional composition of existing owners, not a new queue, heap, pool
or complete budget. The scheduler profile accounts for the inline bundle as its
payload; reservation-owned heaps retain their existing component admission.
Caller callback/output/error heaps, stable disposal-ID scratch and full
topology/Handle/network/graph peak storage remain independently admitted. The
exact topology owner is checked by its reservation pool and runtime reservations
are bound to their runtime. Cross-component matching and native authority remain
the selecting coordinator's responsibility.

`NativeCloseCompletion::consume` solves a move-only composition gap: a caller can
move its payload into a synchronous callback while borrowing the native result.
Native evidence and the scheduler slot remain owned until the callback returns
or unwinds. Evidence is dropped before the original slot; joined child evidence
can retain an ancestor's original admission longer. Returned payloads keep their
own admission, not the consumed scheduler slot. The callback cannot return a
borrow of the evidence. The API does not validate proof, mutate local state,
recover callback panics or publish an event. Use the attachment-bound native
topology scope inside it, perform all fallible preflight before local mutation,
and notify only after all enclosing locks are released.

Ten local tests pass for sequential admission failures and rollback, exact owner
rejection, no waiting gate, capacity movement, late descendants, Page/Context
retirement and rejected latest publication preparation. Further tests exercise
move-only completion payloads, real native success/error evidence, callback
error/panic and parent admission retained by a joined child. The joined CDP test
moves the exact reservation-bound topology into native authority before command
admission, withholds Page destruction, requires Context dispatch, then consumes
parent-first native proof into topology/runtime retirement and a no-op child
join. It uses real bounded CDP I/O and production local owners, but a bounded
test publication observer: not Node lifecycle groups, routes, Handles, network
retirement or a browser/Playwright oracle.

The new feature adds only the existing topology `transition-reservations` and
runtime `disposal-reservations` capabilities, with no new normal packages. Its
direct optional topology dependency required an offline Cargo.lock refresh;
subsequent checks are locked. Exact feature-closure and the strengthened seven
host/eight addon boundary checks pass. Current Node profiles explicitly exclude
these unfinished close capabilities. CI now selects the capacity and joined
native scheduler test lanes. Evidence is retained under
`.work/node136-close-capacity-bundle/`.

Final release suites pass 382 with capacity only, 444 with capacity plus native
scheduling, and 502 with those plus route retirement, Frame lifecycle, runtime
lifetime handoff and extra headers. Each suite has 16 previously ignored tests;
all 16 new tests execute in the joined/combined configurations. Cold and combined
strict Clippy pass, as does the compile-fail doctest rejecting escaped native
evidence. Native scheduler dependency closure, formatting and diff checks pass.
The first locked check's expected lock-refresh error is retained separately
from the successful offline check and subsequent locked verification.

All task-owned verification processes and bounded peer I/O finished. No browser
or persistent server was started. At the 17:56:31Z handoff snapshot, the shared
Codex backend RSS was about 676 MiB, system free percentage 68% and swap zero;
this is not per-chat attribution. The release compiler's earlier roughly
7.6 GiB RSS was transient and its process has exited. Installed NODE-133 addon
hashes were verified unchanged.

Full native/local/publication coordinator and Node scheduling remain unwired.
NODE-123's controlled ancestor-dispatch discrepancy is still open. No addon
rebuild, new browser result, commit or push is claimed by this step.

## NODE-135: runtime-locked local publication and disposal

Composition review exposed a commit window: the existing Page/Context service
publishes topology/routes before calling runtime disposal, but transport close
can invalidate even a prepared runtime transaction. NODE-132's topology mutex
alone cannot prevent that independent runtime mutation. The new selected
`ReservedPreparedDisposal::commit_deferred_with_publication` validates disposal
and runs a short local-publication callback plus runtime retirement under one
runtime-state mutex. It is a building block for the new coordinator; the old
Node service has not been switched to it.

Before invoking the callback, runtime verifies its transaction and live
transport, checks the latest child-first order and resolves every affected
call's cancellation signal. A stopped/stale runtime or inconsistent signal
inventory rejects without invoking publication. After a successful callback,
the same lock protects graph retirement, control messages and cancellation
installation. Pending calls cannot disappear under that lock; the former
post-mutation fallible missing-call lookup is now an explicit invariant, so it
cannot return a recoverable error after publication has begun. Ordinary disposal
uses this implementation with an infallible no-op callback and retains its
existing immediate notification behavior.

The outer Result reports runtime/admission rejection; the inner Result reports
the callback's caller-defined failure. A callback error returns runtime capacity
without disposing its graph, queuing Dispose messages or notifying cancellation.
It does **not** undo external state already mutated by that callback. All
fallible local preparation must precede the callback's first external mutation.
The failure path explicitly releases bytes before the pending slot rather than
depending on closure-capture destructor order. A callback panic poisons runtime;
this API provides no panic recovery and does not report successful disposal.

Successful output is `(caller_output, DeferredDisposalCommit)`. The latter
retains the original bytes/slot, and no user waker runs until explicit notify
after **all** enclosing locks are released. The caller must hold its already
validated native/topology/route/Handle guards in the chosen global lock order.
No runtime reentry, native await, JavaScript/event callback or user-waker
invocation is permitted within publication, including capture destructors.
This callback publishes local prepared state, not user-facing close events.

The future coordinator still needs stable, independently admitted disposal-ID
scratch for borrowed route/Handle preparations; it cannot move a runtime
preparation while those guards borrow its disposal-order Vec. Native receipt
validation, network retirement, lifecycle-group publication and terminal/FFI
settlement remain separate. NODE-134 supplies outbox backing, while complete
graph/table and topology/Handle/network peak ownership remains unfinished.
Callback capture/output/error heaps and typed diagnostics are caller-admitted;
the method adds no queue, pool, worker, feature or normal dependency.

Ten new tests exercise a separately running transport closer that observes the
held runtime mutex and can retire only the remaining root after the callback
and child disposal finish. Other cases cover publication rejection/identity,
stopped/unselected rejection before callback, post-lock reentrant waking, a late
call with typed failure, missing-signal preflight, retained admission/output,
external-mutation non-rollback and panic poisoning. Runtime release suites pass
113 ordinary, 178 selected and 196 combined with lifetime handoff; no ignored
tests. Default host regression passes 416 (8 ignored), preserving its existing
serial path. Cold/combined strict lint, the updated public API doctest, exact
runtime dependency closure and seven host/eight addon boundaries pass. Evidence
is in `.work/node135-runtime-publication-scope/`.

All task-owned verification processes finished, including the transport-race
test thread. Formatting and diff checks pass. No server or browser process was
started in this turn; installed addon hashes were rechecked unchanged.

These are runtime synchronization and regression tests, not a native multi-owner
commit or Node/browser compatibility gate. No addon rebuild/browser run or new
commit/push is claimed. NODE-133 artifacts remain installed; the controlled
ancestor-dispatch failure and full close coordinator/host integration stay open.

## NODE-134: preallocated runtime control outbox storage

`ControlOutboxStorageProfile`, independently selected within the existing
`pwright-runtime/disposal-reservations` feature, now covers the long-lived
control-message and EventItem-lease queue buffers. Count admission alone did
not prevent these buffers from growing during the post-native close commit.
Installation reserves byte admission first and preallocates both buffers to
the runtime's exact logical outbox capacity, before native close reservations.
The queue's existing messages, FIFO order, metrics and paired EventItem leases
are preserved, including when installation follows earlier creation messages.

For capacity Q, let `G(Q) = max(8, 2Q)`. The declared payload envelope is
`2 * G(Q) * (sizeof(ControlMessage) + sizeof(ResourceLease)) +
sizeof(ResourceLease) + 128`, with checked arithmetic and addressable-size
validation. Two generations allow old grown buffers to coexist with newly
reserved storage during installation. The whole peak allowance remains owned
by `RuntimeStore`, not the close pool: queue drain/clear, transport shutdown,
pool drop and dispatcher drop do not release it while another lifecycle handle
still retains the store and its backing arrays. The lease is dropped after
the queue fields. Allocator bookkeeping/rounding is separate; this is not RSS.

Installation rejects a non-extension scope, overflow, unbudgeted runtime,
missing/mismatched outbox, mismatched queue/lease inventory, duplicate selection,
outstanding message claims or disposal reservations, active lifecycle mutation,
stopped runtime, undeclared plan and insufficient byte admission. Both allocations
are fallible. Replacement lease storage is allocated first; queue reservation
is the last fallible step before the resource-buffer swap and lease installation.
Admission/allocation failure preserves installed messages and resource ownership.

Core `BoundedQueue::try_reserve_full_capacity` explicitly preallocates without
changing logical capacity, contents, counters or overflow policy; its new
`allocated_capacity` reports backing element slots, not bytes or physical RAM.
Existing constructors remain lazy. The runtime's extra lease field is compiled
only with disposal reservations, and no Node profile selects that feature.
There is no new dependency, transitive feature, worker, global/default budget
increase or browser operation.

Fourteen new runtime tests and two core tests cover existing messages/leases,
full creation-plus-disposal queue capacity without growth, retained storage
lifetime, duplicate/mismatched selections, outstanding owners, closed runtime,
budget failure, overflow and default queue behavior. Final runtime release
results: 113 ordinary, 168 selected and 186 with lifetime handoff; all have zero
ignored/failing tests. Core release passes 75 tests. Selected/combined/cold
strict all-target Clippy and the updated public API doctest pass. Node boundary
checks retain all seven host/eight addon selections. Exact normal runtime
dependency/feature closure, formatting and diff checks pass. All verification
processes finished; no task-owned server/browser process was launched. Final
shared-backend RSS was approximately 1.9 GiB with zero swap, not a per-chat
attribution. Evidence is in
`.work/node134-preallocated-outbox/`.

The intermediate `runtime-combined-final.log` has one test-fixture failure:
its simulated budget consumer incorrectly requested the global allowance from
one scope. The corrected fixture consumes the permitted per-scope allowance,
then observes actual installation rejection with preserved buffers/messages.
`runtime-combined-green.log` is the final 186-test pass; the red log is retained.

This completes the outbox-backing portion of NODE-131's outstanding ownership,
not long-lived graph/table nodes or complete topology/Handle/network commit
scratch. Those remain separately owned/admitted, followed by the full latest-state
close transaction, Node scheduler selection and written-command shutdown drain.
No addon was rebuilt or browser test rerun for this opt-in runtime change;
NODE-133 hashes and their scoped browser evidence remain unchanged. The
controlled ancestor-dispatch discrepancy remains open. No commit or push.

## NODE-133: retained worlds follow exact native Page destruction

Selected native-world receipts now freeze the owning Page's native target ID,
alongside their connection epoch, Context, session and unique world IDs. Both
paired Frame snapshots and utility-only Page batches capture the Page target,
not an OOPIF's separate native target. The topology driver consumes a real
`Target.targetDestroyed` fact and marks only matching retained receipts from
its current epoch. This remains useful after local Page/OOPIF routes are gone;
unrelated Pages and an OOPIF target destruction alone do not match the owning
Page identity. Generic `Target.detachedFromTarget`, route deletion and a close
command acknowledgement are still not proof that borrowed worlds were destroyed.

The handler holds topology, route and registry locks in that order, validates
the driver's epoch under the topology lock, and updates monotonic atomic bits.
It does not rebuild routes, send commands, notify user wakers, retire runtime
objects or publish Page/Context close events. Its outcome remains `Ignored`
because there is no topology delta. With native-world retirement unselected,
the handler skips decoding and preserves prior ignored-event behavior. Source
authority remains the topology driver's existing connection/runtime assembly;
this adds no public receipt-forging or general cross-attachment authority API.

The additional retained Page target is bounded to 1024 bytes before copying.
The paired-snapshot layout proof now covers six bounded IDs plus its headers
inside the existing 8192-byte permit; utility-only snapshots share this bound.
Installed registry admission already derives its inline/expired-Weak envelope
from `size_of::<Receipt>()`, so it also covers the larger receipt. No extra pool,
dependency, worker or native-retirement-off budget is introduced. These are
logical retained-payload limits, not a process RSS or allocator-overhead bound.

The selected release suite passes 397 tests (16 ignored), including 12 new
cases for Page/local/OOPIF and utility-only ownership, late native destruction,
sibling isolation, duplicate events, stale epochs, malformed/unknown events,
unselected behavior and oversize rollback. Selected strict all-target Clippy
passes, as does feature-cold strict all-target Clippy. Tests invoke the real decoded driver branch with fake native clients
and real topology/routes; they are not raw-CDP or browser-oracle evidence.
The first compilation failure (missing async snapshot awaits) is retained
separately from corrected successful runs in
`.work/node133-page-world-retirement/`. The combined composition suite passes
486 tests (16 ignored); default Node host passes 416 (8 ignored). Node unit
tests pass 84, and TypeScript, seven host/eight addon dependency boundaries,
workspace formatting and diff checks pass. These overlapping suites are not
summed as distinct tests.

Both release addons were rebuilt and installed by atomic rename. SHA-256:

- Default: `d7d70cb36b0870858e2109ba69c67c6eb06da855dc2a8ea65158a297f8c45d95`.
- Drag/frame/extra-headers: `6406b660904da3fbbf51bb40ad383c4d18b9a0591472de7b7b5d4e184dd45525`.

On the existing dedicated Chrome 152.0.7977.83 over loopback CDP, both rebuilt
profiles pass six native-close-confirmation cases. Attachment-retirement passes
six default and ten combined cases: 28 browser test cases total, including
upstream 1.58.2 comparisons and candidate Worker cleanup assertions. Final
logs are `browser-{default,combined}-{confirmation,attachment}.log`. These are
focused regression lanes, not browser proof of every new retained-receipt
case, and not a rerun/pass of the controlled ancestor-dispatch probe. Earlier
NODE-123 broad browser matrices describe the earlier artifacts, not these hashes.

All task-owned build/test/relay processes finished; no new fixture server was
started. The task's initial exact blank target was closed before the tests.
The final Chrome inventory contained one unattributed blank target, which was
left untouched rather than assumed task-owned. Shared-backend RSS fluctuated
during the run; the final targeted physical-footprint reading was about 1.8 GiB
with zero swap. Shared process measurements cannot identify this chat's RAM.

This corrects retained native-world cleanup evidence only. It does not connect
NODE-124 through NODE-132 into a complete close transaction or change the
serial Node request scheduler. NODE-123's controlled ancestor-dispatch failure
remains unresolved; no full Playwright compatibility claim follows.

## NODE-132: attachment-bound native close commit scope

`AttachedChromiumComposition::with_native_close_topology` connects sealed
`NativeCloseEvidence` to a short synchronous current-topology callback. Exact
connection and shared-topology Arc identity must match the attachment before
locking. After acquiring that exact topology mutex it rechecks native owner
liveness, validates the receipt's epoch/native identities against the protected
view, and only then invokes the callback. Context ancestor validation has an
internal already-locked form so this path never recursively locks topology.
Ordinary public certification retains its early domain checks.

The callback receives borrowed `NativeCloseCommitTarget::Page` or `Context`
keys from the sealed receipt plus the mutable production topology. It does not
substitute a current lookup for the original native identity. A valid
parent-first local retirement can leave the child's Page/Context route absent;
the captured exact ancestor remains valid for a local already-retired join.
Epoch change or native/portable target, session or Context rebinding rejects
before callback invocation, as do owner stop, native disconnect and poisoned
topology. This is native authority plus topology serialization, not proof that
runtime/route/Handle/publication owners belong to that same domain.

The selecting coordinator must enter with all reservation/scratch admission,
perform only synchronous local preparation/commit, release other guards and
return NODE-130/NODE-131 deferred notification owners. It invokes their notify
ports only **after** this outer method returns and releases the topology lock.
The outer Result describes authority/scope acquisition; the callback's Output
can independently contain a local preflight/commit Result. A callback error is
not rollback of state it already mutated, and no automatic notification or
terminal/public-close settlement is inferred. Native transport can terminate
after validation; runtime terminal checks and post-commit teardown remain the
coordinator's responsibility rather than an impossible lock over the browser.

Successful scope entry allocates no collection, native ID, subscriber, worker
or new lease pool. Borrowed proof stays inside its completion admission;
callback capture/output, diagnostic and commit scratch storage remain separately
owned/admitted. The API is part of existing opt-in `native-close-scheduler`,
with no new dependency/feature/default profile selection and no host wiring.

Verification in `.work/node132-native-close-commit-scope/` passes 474 combined
composition tests (native-close-scheduler/close-route-retirement/frame-lifecycle),
with 16 ignored, including all 12 new scope cases. The first selected run passed
426 tests (16 ignored) before the last disconnect/poison cases were added.
Combined, authority-only and feature-cold all-target strict Clippy pass. Exact
scheduler dependency and all seven host/eight addon boundary checks pass.
The authority-only release regression passes 394 tests (16 ignored), preserving
the non-scheduler certification path. Workspace formatting and diff checks pass;
all task-owned verification processes finished and native peer I/O was joined.

Tests use production CDP and bounded Tailscale-bound native peers, then real
Page/Context topology preparation/commit through the same implementation used
by the public attachment method. They are not full multi-component or Node
browser discrepancy tests. Full peak ownership, retirement/publication and
host scheduling/drain remain unfinished. Default/combined installed addon
hashes are unchanged from NODE-123; no browser run, addon rebuild, additional
commit or push is claimed.

## NODE-131: deferred runtime disposal notifications

Short close commits cannot call arbitrary wakers while the coordinator still
holds topology, route, Handle or network-driver locks. The existing runtime
commit drops its own lock before waking, but cannot release its caller's locks.
`DisposalReservationProfile::with_deferred_commit(pending_calls)` now explicitly
selects `ReservedPreparedDisposal::commit_deferred` and its typed-failure variant.
The ordinary reservation profile and immediate commit behavior are preserved.
This selection remains inside the existing `disposal-reservations` feature and
adds no dependency package, transitive feature, default selection or native work.

The selected bound must equal the whole runtime pending-call limit, just as
the object bound already equals its whole graph limit. Admission still precedes
native work and retains no lifecycle gate or stale graph. Preparation uses the
latest subtree; commit installs parent-closed cancellation or the caller's typed
failure under the runtime lock without invoking any user waker. Existing caller
cancellation and transferred lifetime owners retain their ordinary precedence.
The selected implementation avoids the additional subtree BTreeSet by using the
already admitted disposal order for membership checks.

The new `DeferredDisposalCommit` exposes removed child-first records only by
borrow. Its idempotent `notify(&mut self)` drains saved wakers after the selecting
coordinator has released **all** outer locks. Calling notify does not return the
pending permit or byte lease: these stay with the retained record/waker owner
until drop, even if the runtime and reservation pool have already been dropped.
Drop does not invoke wake; the coordinator must notify on successful retirement
and on every subsequent error/teardown path after releasing its locks. Local
unknown-ID no-op behavior remains local runtime behavior, not native-death proof.

The added per-slot collection-payload envelope uses whole graph limit M and
pending-call limit C. With `G(n) = max(8, 2*n)`, it charges concurrent current-order
verification, graph-disposal order and traversal stack, removed record storage,
affected call IDs, temporary signal references and saved wakers:

```text
G(M) * (2*sizeof(ObjectId) + sizeof((ObjectId, bool)))
  + M * sizeof(ObjectRecord<ObjectKindTag>)
  + G(C) * (sizeof(CallId) + sizeof(Arc<CancellationSignal>) + sizeof(Waker))
  + sizeof(DeferredDisposalCommit) + 256
```

This is additional to the reservation's existing original disposal order,
control leases and waiting-owner budget. Checked arithmetic rejects overflow;
reselection and runtime-limit mismatch reject before installation/mutation.
Long-lived runtime graph/table storage and the control outbox's backing arrays
are not charged by this transient-result envelope: they can remain allocated
after a result/pool is dropped and need owner-lifetime admission in the selecting
composition. Existing outbox EventItem/lease counts are not a byte-storage proof.
The envelope describes collection payloads with small-vector/growth headroom,
not allocator metadata, arbitrary user-waker-owned heaps or total physical RAM.
Typed diagnostic heap storage remains independently admitted by the caller and
may outlive this result through affected call signals. Existing Rust allocation
failure behavior is unchanged; budget admission is not a recoverable allocator.

Verification is recorded in `.work/node131-deferred-runtime-close/`. Runtime
release tests pass 113 ordinary, 154 with reservations, and 172 with reservations
and lifetime handoff; the latter includes all 13 new deferred cases. Strict
ordinary/combined all-target Clippy and the updated public API doctest pass.
Existing Node host release regressions pass 416 ordinary (8 ignored) and 464
with drag-runtime/frame-navigation/extra-headers (10 ignored). These host runs
exercise the preserved immediate commit path, not selection of the new port.
Runtime dependency comparison and all seven host/eight addon dependency
boundaries pass; runtime JavaScript still imports no driver/CDP/process launcher.
Workspace formatting and diff checks pass. CI now explicitly retains ordinary
runtime tests/lint alongside selected tests; no remote CI run is claimed.

Installed default/combined addon hashes are unchanged from NODE-123. No addon
was rebuilt, no browser gate was run and no additional commit/push was made.
All task-owned verification processes finished. This is runtime/component
evidence, not a Node browser discrepancy pass.

Remaining: the full latest-state coordinator still needs topology/container and
Handle/network/native-world scratch admission, exact ancestor/source validation,
serialized multi-component retirement and publication, already-retired joins,
host request scheduling and written-command shutdown drain. Then rebuild both
addons and rerun the NODE-123 controlled and original transparent browser gates.

## NODE-130: deletion-only close route retirement

The independently selected `close-route-retirement` feature adds
`CdpPageServices::prepare_close_route_retirement`. It is a local routing
transaction, not native-death authority, topology/runtime disposal or public
event publication. It has no dependency prerequisites and is not selected by
the Node host/addon. Ordinary synchronization and existing close services remain
unchanged; the future concurrent-close commit explicitly selects this primitive.

Preparation borrows the caller's already-bounded child-first runtime disposal
slice and validates the prepared topology candidate against the current route
view. Epoch must match. Every survivor must retain its exact identity; every
candidate route must already exist and must not be in the disposal set. Checks
cover Context/Page membership, Page target/session/context/main-frame/crash
identity, main-world evaluation and Handle-evaluation routes, main/utility
Locator/DOM routes, Locator screenshot native-frame/parent/world identity,
optional navigation entries and readiness Frame-to-Page membership. This is not
a general route refresh: new routes, changed surviving worlds, incomplete
disposal sets or removal of a still-present candidate route fail before mutation.

The prepared owner retains the existing route guard followed by the optional
network-subscription guard, matching the existing lock order. Pending Context
subscription transitions and missing active survivor lanes reject preparation.
Keeping the network lock means commit does not reacquire a fallible lock or
silently discard a poisoning error after another state component has committed.
Dropping preparation releases locks with no local state change.

Commit removes only disposed route keys, preserving all surviving service Arcs,
input ownership and the native-world receipt registry. Removed navigation
entries are marked non-current; unrelated retained snapshots remain current.
It prunes subscription lanes/Contexts from that survivor view without inserting
or repairing them. It does not create service factories, new native clients,
route tables, IDs, subscriptions, queues or native commands. In particular,
removing local routes does not mark borrowed execution worlds destroyed: the
coordinator must independently record exact native receipts before declaring
native object retirement.

The returned `CloseRouteRetirementWake` is an explicit borrowed post-commit port.
Its `notify` runs only after route/network locks have been dropped. The selecting
coordinator must also release enclosing topology/runtime/handle locks first:
arbitrary user wakers may reenter the host. The port is not a metadata visibility
or lifecycle-event publication receipt, and notifying alone cannot satisfy a
world-readiness lookup.

There is no additional heap-owning pool or default resource-profile increase:
preparation/commit manage borrowed guards and delete from existing collections.
The eventual coordinator still includes these inline guard/wake layouts in its
own envelope and separately admits disposal IDs, topology/runtime/handle
scratch and event groups. Normal allocator metadata and arbitrary injected value
destructors are not turned into an RSS guarantee by this local no-new-collection
contract. Topology and runtime gates/native evidence must be held by the caller
across this short synchronous phase; this primitive neither validates a native
connection/source nor authorizes a caller to commit an unrelated candidate.

Release verification in `.work/node130-close-route-retirement/` passes 385
selected tests (`routes-verified.log`) and 462 with native close scheduling and
Frame lifecycle (`routes-combined.log`), each with 16 ignored. All 13 new route
tests pass. They use real topology/routes/ledgers and the existing deterministic
session factory; they are not a browser/Node oracle.

Retained intermediate evidence includes initial missing test imports and one
incorrect screenshot identity expectation (`routes-selected.log`: 384 passed,
one failed). Ordinary synchronization copies the screenshot route wrapper when
rebuilding clip translators. The corrected comparison checks the retained inner
service against ordinary synchronization, then separately verifies that this
deletion-only path preserves the original outer wrapper too. This was a test
oracle correction, not a production behavior fix or a finding about ChatGPT RAM.

The exact dependency comparison adds no normal packages or transitive features;
all seven host and eight addon selection-boundary checks pass, with no driver,
CDP transport or process-launch imports in runtime JavaScript. Installed default
and combined addon hashes remain unchanged from NODE-123. No browser/server was
started and no additional commit or push was made for this route primitive.
Strict all-target Clippy passes for feature-cold, selected and combined
configurations (`clippy-ordinary.log`, `clippy-first.log`,
`clippy-verified.log`); workspace formatting and `git diff --check` also pass.

The remaining close work is still substantive: full commit-scratch peak
admission, topology/runtime/handle/network-driver and native-world retirement,
latest-view close-group publication, exact already-retired joins, host driver
scheduling and shutdown drain, then rebuilt default/combined Node artifacts
and the original controlled/transparent browser discrepancy gates.

## NODE-129: bounded native close scheduler

The new opt-in composition feature `native-close-scheduler` selects the existing
native authority and Page confirmation features. Its attachment factory creates
one concrete connection/topology authority and Page source, sharing the same
interruption owner. Callers cannot assemble a scheduler from foreign sources.
Ordinary composition fields, Node host selections and installed addons are
unchanged. This implements native coordination, not the retirement transaction.

`NativeCloseScheduler<T>` has a fixed N-entry vector. Page/Context admission
returns T unchanged on failure and sends no native command. It admits its slot,
exact Page ancestry or Context command, and any Page command/receiver futures
before insertion. T is the caller's retained admission/reply payload: inline
storage is measured, while payload-owned heaps and topology/runtime/publication
reservations must be independently admitted before native polling. N bounds
retained intents, not merely executing commands. Default/external Context
disposal is rejected; the future host adapter must preserve its existing special
public close policy instead of routing those cases through owned-Context proof.

`poll_next` drives all native entries before yielding one completion. It polls
Context work independently of Page acknowledgement/destruction waits; it does
not hold any topology/runtime mutation gate. Cancelling a surrounding poll
future does not remove native work. A successful Context response is certified
against each pending Page's exact domain/epoch/ancestry and retained as a shared
receipt only for covered descendants. A failed or unrelated Context cannot
complete the Page wait. Already-complete entries are never repolled. A measured
round-robin completion cursor prevents newly refilled low slots from starving an
older ready result; no extra completion queue is allocated.

An acknowledged Page can use the exact ancestor receipt instead of waiting for
Target.targetDestroyed. A still-written Page command is never dropped or reported
complete solely because its Context completed: the scheduler waits for its
response/transport termination, then can use the retained ancestor even if the
native child command failed because its target was destroyed. An unpolled child
has no written command to retain and may use valid ancestor proof immediately.
Parent-first topology retirement does not invalidate captured exact ancestry.
Owner stop and epoch/route rebinding prevent evidence validation; native errors
do not themselves mint death proof. Page and Context public error/idempotence
rules still belong to the future host adapter.

Page binding validation occurs before its first native poll and, through the
additive leaf `prepare_command_with_validation` API, again after discovery and
immediately before Target.closeTarget initiation. The validator is synchronous;
its concrete captured body is measured in the existing command-future envelope.
Its separately admitted shared Page binding outlives both command and scheduler
entry. The unguarded split API remains available with a zero-sized validator;
the ordinary confirmation service is unchanged.

Completions keep their scheduler permit and byte lease. Native evidence has
private constructors, is exposed by borrow, and must be revalidated immediately
before serialized current-state preparation/commit. Consuming the completion to
retrieve its reply payload drops the evidence. Shared Context receipts retain
the parent's original slot even after that Context completion is consumed, until
the last descendant joins release it. This prevents repeatedly freeing parent
slots while accumulating retained receipts. No worker, unbounded result queue,
or asynchronous Drop mechanism is installed. Once native work is polled, the
owner must be driven to completion/transport termination before it is dropped;
host/Worker shutdown still needs to integrate this explicit drain contract.

The scheduler profile is generic over T and declares one scope and N+1 leases.
Its state envelope includes the scheduler, shared admission owner/Arc headers
and N fixed Option<Entry<T>> cells. Each retained slot additionally admits the
completion layout, Slot, shared Context receipt, shared Page binding, Arc headers
and an 8192-byte temporary/retained diagnostic envelope. This covers the bounded
leaf error while converting to a maximum 1024-byte UTF-8 scheduler diagnostic.
Storage drops before its permit becomes reusable. Authority and native Page
profiles must each cover N; their budgets remain independent. The connection
must declare the corresponding native Page subscriber and transport capacity.
These are logical admitted allocations, not measured RSS, and do not yet derive
the complete runtime/topology/route/handle/network/publication commit peak.

Evidence is retained in `.work/node129-native-close-scheduler/`. The initial
compile-only log contains a test helper API mistake, subsequently corrected.
`scheduler-red.log` has eleven native tests passing and the stale pre-first-poll
Page regression failing. `composition-selected.log` then passes 413 tests with
16 ignored, including nineteen scheduler cases; `composition-final.log` adds
guarded discovery and maximum identity for 415 passes. These intermediate runs
remain separate from the final corpus:

- `composition-verified.log`: 416 pass, 16 ignored, including 22 new scheduler
  tests. Cases cover withheld destruction/independent ancestor dispatch, both
  command/receipt orders, exact versus unrelated/failed ancestors, written child
  retention, parent-first topology retirement, owner interruption/peer loss,
  admission/cursor rollback, returned payload ownership, absent/rebound routes,
  pre-first-poll and post-discovery epoch checks, maximum identities and fair
  completion delivery. Each fixture joins its peer/native I/O and asserts zero
  in every resource class and zero active leases at teardown.
- `leaf-verified.log`: 38 pass, including two new validator admission/rejection
  tests. `leaf-ordinary.log`: six feature-cold tests pass. Earlier
  `leaf-final.log` retains the previous 36-test run.
- `clippy-combined-verified.log` and `clippy-leaf-verified.log` pass strict
  all-targets Clippy with scheduler plus Frame lifecycle and the selected native
  leaf. `clippy-ordinary.log` passes the feature-cold composition check;
  earlier selected-only checks also pass. No lint suppression was added.
- `dependencies-final.log` and `dependencies-leaf.log` prove no new normal
  packages or unintended transitive features. `node-boundary-final.log` passes
  all seven host and eight addon selections, with no JS driver/CDP/process
  imports. `node-boundary.log` retains an initial wrong script-path diagnostic;
  the final command uses the package's existing pnpm boundary script.
- CI includes the selected scheduler tests, strict lint and parameterized
  dependency proof; no remote CI run is claimed. Formatting and whitespace checks
  pass. Both installed Node binary hashes remain NODE-123's values. No browser
  differential, new addon build, commit or push was performed, and no task-owned
  protocol peer or background server remains.

Next compose the NODE-124/125/126 capacity owners and latest-state short commit
with this scheduler, select it in the host with full peak admission, preserve
Page/Context public error and terminal-publication rules, then rebuild the default
and combined addons and rerun the original controlled and transparent close
probes. The NODE-123 installed-addon ancestor-dispatch discrepancy and the full
compatibility goal remain open.

## NODE-128: exact native Context ancestor authority

The opt-in composition feature `native-close-authority` adds
`NativeCloseAuthorityProfile` and the attachment's `native_close_authority`
factory. Production installation derives its concrete connection and shared
topology from the same attachment; it does not accept a caller-assembled pair.
The domain is bound to its installation epoch. Clones share an identity, a
bounded retained-intent limit and a checked monotonic binding sequence. No
worker, event cursor, mutation gate or scheduler is installed.

`bind_page` reads one current Page route and its owned native Context, validates
all bounded identities and captures portable Page/Context IDs, target/session/
native-Context IDs, epoch and admission generation. Its sealed result has no
public constructor. Default/external Context ancestry cannot mint this owned
Context-disposal authority. Unknown routes, changed installation epoch, closed
connection/owner, exhausted capacity and sequence overflow reject before native
mutation and return admitted resources on failure.

`prepare_context` captures an owned route, admits its retained ticket/identity
and boxes the actual native-command future before mutation. Starting revalidates
the exact current route/epoch before issuing Target.disposeBrowserContext. The
future retains written-reply ownership even after caller interruption. Only an
actual successful native response followed by live-owner/epoch validation yields
`NativeContextCloseReceipt`; an error classified as already closed is not a
destruction receipt. Native error classification is retained, with formatting
bounded to 1024 UTF-8 bytes. The ordinary public Context-close idempotence policy
is unchanged; this authority API is not yet its replacement.

`certify_page` requires the same domain, epoch and captured Context identities,
and a Page binding admitted no later than receipt creation. Under one topology
lock it validates current epoch/ownership, Context ID reverse mapping, Page target
reverse mapping, session ownership and any still-materialized Page route.
Previously retired routes are allowed, so a parent-first topology commit does
not erase genuine captured ancestry. Rebound IDs/routes and bindings created
after the receipt are rejected. The returned borrowed `NativeAncestorRetirement`
retains both underlying tickets via its references; certification allocates no
new storage. It is native evidence, not a transaction guard or event receipt.
The coordinator must serialize certification and latest-state preparation/commit,
revalidating after any await or unrelated lifecycle mutation, and retain any
still-written child command independently using NODE-127's phase ownership.

The profile declares one independent scope, `N + 1` lease records and
`2048 + N * 16384` BackendStateBytes. N is an explicit nonzero u16 limit shared
by Page bindings, prepared Context commands and retained Context receipts, not
just currently executing commands. Native IDs have an explicit nonzero bound
at most 1024 bytes. The state/erased interruption wrapper layout is checked
before installation allocation. Page-binding layout plus three maximum IDs is
compile-checked; Context preparation checks the actual future body, receipt,
three identity copies, 1024-byte diagnostic and 4096-byte command bookkeeping
against its 16 KiB envelope. Ticket drop releases byte storage before making
its pending slot reusable. CDP wire/error transport ownership, Page confirmation
and all topology/runtime/publication reservations remain separate components;
this is not a complete coordinator peak budget or an RSS estimate.

Verification in `.work/node128-native-ancestor-authority/`:

- `composition-selected.log`: 394 pass, 16 ignored, including 22 new tests.
  The bounded production-CDP peer tests cover exact acknowledgement, current
  ancestry, independent domain/Context rejection, generation cut/overflow,
  epoch changes before and after native reply, route/ID rebinding, missing and
  unowned routes, interruption, peer loss, shared capacity, failed byte
  admission, drop/reuse and maximum 1024-byte IDs. Native topology has no
  active transaction during the pending command. Every fixture joins its own
  peer and native I/O and verifies all resource classes/leases return to zero.
- Selected, feature-cold and combined Frame lifecycle strict all-targets Clippy
  pass. `authority-first.log` retains
  the earlier ten-test focused run, separate from the final expanded corpus.
- Dependency proof allows only the root feature and the explicit existing
  native-confirmation prerequisite: no new normal packages or other transitive
  features. The common CDP error classifier becomes a direct optional dependency;
  protocol-peer futures/WebSocket dependencies are dev-only. Node boundary
  checks pass seven host/eight addon selections. CI includes selected tests,
  strict lint and the dependency proof; no remote run is claimed.
- No installed Node binary was rebuilt: default/combined hashes still match
  NODE-123. No browser differential was rerun or declared fixed. Formatting and
  whitespace checks pass; no test peer or task-owned background server remains.

Next wire bounded native scheduling and short retirement commits using this
authority, NODE-127 command phases and NODE-124/125/126 storage reservations.
The full compatibility goal and controlled ancestor-dispatch discrepancy remain
open; this feature is not selected by the Node driver/addon.

## NODE-127: native command and destruction-wait phases

The selected `native-confirmation` leaf now exposes an additive
`NativePageCloseConfirmation::prepare_command` path. It returns a
`PreparedNativePageCloseCommand` with both native-command and destruction-wait
futures already admitted and boxed. Its `start` transfers the command future
to the future coordinator; the coordinator must retain it through the written
command's response or native transport termination, independently of caller or
ancestor completion. No scheduler is installed here.

Only a successful native `Target.closeTarget` reply returns an
`AcknowledgedNativePageClose`. This value retains the source identity, bounded
target ID and original pre-admitted receiver/slot/storage. `belongs_to` checks
the exact installed source (and hence its bound concrete connection), not only
a matching target string. It does not encode topology epoch, Context ancestry
or ancestor destruction. `wait_for_destruction` transfers the prebuilt future
without another command, allocation or subscription. It uses the existing exact
unscoped `Target.targetDestroyed` validation; early events remain buffered in
the original cursor. Command acknowledgement alone is never public success.

An acknowledged wait can be dropped without abandoning an in-flight command,
but only the eventual coordinator can authorize public completion from an exact
independently validated ancestor receipt. Dropping a started command future is
still forbidden by the continuation ownership contract; the leaf does not
spawn a replacement task to conceal such a drop. Owner interruption during
discovery waits for that written reply, then prevents `Target.closeTarget`.
Interruption after Target.closeTarget is sent waits for the command reply before
the destruction phase reports Cancelled. Native failure, false/malformed success
responses, discovery failure or transport loss never mint an acknowledgement.

No profile allowance or default pending count changes. Both paths share the
explicit pending cap, source state and independently budgeted fanout cursor.
Split admission checks the actual command/receipt future body sizes plus
`size_of(Prepared) + 2 * target_bytes + ERROR_BYTES (4096) +
INTERRUPTION_FUTURE_BYTES (1024)` and wrapper storage against the existing
16384-byte call envelope. Receipt storage is checked before boxing; the final
combined bound is checked before boxing the command future. The maximum
1024-byte native identity is tested. No native mutation precedes these checks.
CDP command-wire/event admission remains the connection profile's separate
responsibility; full coordinator peak budgeting is still outstanding.

Ordinary `CdpPageCloseConfirmation::prepare/start` keeps its existing source and
behavior, sharing only the extracted receipt-wait loop. The Node host still
selects that ordinary confirmation interface and its one-pending-close profile;
it does not call `prepare_command`. The new code remains behind the existing
native-confirmation feature. `futures-util` and `tokio-tungstenite` are added only
as dev-dependencies for a bounded test-only protocol peer, not a browser or oracle.

Verification in `.work/node127-native-close-phases/`:

- `native-verified.log`: 36 tests pass, including 15 new phase tests through
  the real production CDP connection and bounded WebSocket peer. Tests cover
  both event/reply orders, exact-source identity, pending/byte/cursor reuse,
  maximum IDs, concurrent independent commands, discovery/command failures,
  interruption before/after mutation and transport loss in both phases.
  Every fixture joins its peer and native I/O tasks and checks all resource
  classes and live leases return to zero. Mac runs bind the verified Tailscale IP;
  the test harness uses the documented wildcard local-dev fallback in CI without
  Tailscale. Its 10-second transport/peer watchdog is not a browser close policy.
- `ordinary.log`: all six feature-cold leaf tests pass. Strict leaf Clippy and
  default/combined Node-addon all-targets Clippy pass. The first lint log retains
  a test-only single-match diagnostic, fixed without suppression before rerun.
- Existing Page-lifecycle-only dependency proof passes. The new selected proof
  finds no normal package/version/transitive-feature delta after stripping only
  Cargo's duplicate `(*)` display marker; the initial marker-only diff is retained.
  Node boundary checks pass seven host/eight addon selections. CI now explicitly
  includes selected native tests, strict lint and the dependency proof; no remote
  CI execution is claimed. Formatting and whitespace checks pass.
- No Node binary rebuild or browser run: the installed default/combined addon
  hashes remain NODE-123's values. No test peer or other task-owned server remains.

Next implement exact native Context/epoch/ancestry receipt authority and compose
these phases with topology, runtime and publication reservations in the bounded
native close coordinator. The original controlled ancestor-dispatch discrepancy
remains unresolved; this is not an end-to-end compatibility pass.

## NODE-126: deferred close publication capacity

The existing composition observer offers an explicit `reserve_capacity(target)`
method returning `ChromiumCloseCapacity`. Its default implementation returns
UnsupportedCapability without calling the immediate observer. Ordinary close
services, constructors, fields and scheduling remain unchanged. The selected
Node host observer implements the method, but the native driver does not call
it yet. No Cargo dependency or feature selection was added.

Admission retains metadata ownership and reserves capacity, not a visibility
snapshot: one lifecycle group for Page, seventeen for Context (the existing
sixteen-Page bound plus its Context). This includes possible later descendants,
not only currently visible children. Groups use the existing bounded publication
and FrameTreeFactory ledgers. Each group consumes its ordinary publication
EventItem and factory storage reservation; an additional factory envelope covers
`size_of(Capacity) + size_of(Prepared) + maximum *
(size_of(Notice) + size_of(LifecycleReservation))`. Both box layouts and
simultaneously allocated vector buffers are admitted. Group storage already
accounts for its group/Arc/temporary descriptor allowances. This is a logical
storage contract, not a per-call RSS estimate or a complete coordinator budget.
No new pending queue, worker, native future or subscription is installed.
The future coordinator must admit its required concurrent close count alongside
existing publication/terminal/creation pools; the primitive does not promise that
every FFI lane can simultaneously reserve a maximum-size Context cascade.

After native work, the caller serializes a short preparation/commit phase.
Preparation consumes the capacity, validates the exact admitted target and root
disposal, rejects unrelated disposal effects, and builds notices from the latest
planned collection revision and visibility mask. It can include Pages added
since admission and skips members already hidden by earlier close commits.
It allocates the Prepared box against the existing envelope, but reserves no
new group or view storage. Unused groups are released immediately; drop/error
releases the remainder and envelope. It reuses the ordinary Page-before-Context
staging and publication drain.

The existing strict Prepared check is retained: any view change between
preparation and commit fails closed, rather than applying stale state.
Preparation failure after native mutation is the caller's teardown
responsibility. A capacity/borrowed `ChromiumClosePlan::new` is not proof of
native success, object incarnation or acknowledged ancestor destruction.
In particular, an already-hidden Page does not itself authorize public success.

Final verification in `.work/node126-deferred-publication/`:

- `host-default-final.log`: 416 passed, 8 ignored; combined drag-runtime,
  frame-navigation and extra-headers: 464 passed, 10 ignored in
  `host-combined-final.log`. Thirteen new deterministic host tests cover both
  commit orders, visibility changes, a new revision growing from two to sixteen
  Pages, full group exhaustion, partial rollback, drop, invalid plans, exact
  group counts, strict interleaving rejection and terminal handoff.
- `legacy-observer.log`: the composition compatibility test passes; a legacy
  observer opts out without invoking its immediate preparation method.
- `clippy-default-final.log` and `clippy-combined.log`: strict all-targets
  Clippy passes. The initial `clippy-default.log` retains the boxed-local
  diagnostic; consuming the Box before the internal planning function fixes it
  without suppressions. Both full host suites were rerun after that change.
- `node-unit.log`: 84 passed; `node-types.log` passes. `node-boundary.log`
  passes seven safe-host and eight addon selections, with no JS driver, CDP
  transport or process-launcher imports. Formatting and diff-whitespace checks
  pass. Existing CI host/workspace tests pick up these new cases; no remote
  CI run is claimed.

No Node binary rebuild or browser run occurred; no new server remains running.
Native intent scheduling, exact ancestor receipts, cancellation/command ownership
and whole-composition peak budgets remain required. The NODE-123 controlled
ancestor-dispatch discrepancy is unchanged.

## NODE-125: opt-in runtime disposal and outbox reservations

Implemented `pwright-runtime/disposal-reservations`, without selecting it in
the host/addon. `DisposalReservationProfile` requires an explicit nonzero
pending limit N, the runtime's exact whole-object limit M, a budgeted outbox
and an extension scope. Binding rejects an outbox that cannot fit N * M
reserved messages. The eventual recipe also needs headroom for ordinary
creation/native-event bursts; this minimum is not a complete coordinator plan.

`DisposalReservations::reserve` checks transport closure under the runtime
lock before slot/byte admission. It reserves one shared pending permit,
retained owner/lease-vector storage, M outbox slots and M EventItem leases,
but no graph snapshot or lifecycle gate. Ordinary mutation admission subtracts
all outstanding reserved slots. Existing exclusive transactions reject new
reservations because their uncommitted messages are protected by their gate.
The control-item leases use the selected extension scope, not guessed future
Object IDs; emitted messages retain their actual Object IDs and normal FIFO
claim/drain behavior. No extra EventItem allowance is added: both reserved
and queued items consume the runtime's existing bounded outbox budget.

`ReservedDisposalCapacity::prepare` validates the exact runtime handle and
transport, reads the latest child-first subtree and enters the ordinary
exclusive disposal transaction. It trims unused item leases and transfers
queue-slot ownership into that transaction. Commit reuses existing disposal,
call cancellation, typed-failure propagation and current lifetime-owner rules.
It converts reserved slots to actual messages under the same lock. Drop/error
releases item leases before freeing slots; transport closure cannot double
release them. Ledger replacement is rejected while capacity remains reserved,
even if the object graph is empty. Pool/waiting owners retain only a Weak store
reference and do not keep a dropped runtime alive.

The feature adds one reservation counter to RuntimeState and one slot count to
PreparedObjectDisposal; both fields are absent when the feature is off. Additional
BackendStateBytes are `512 + N * (256 + M * (size_of(ResourceLease) +
size_of(ResourceRequest) + 2 * size_of(ObjectId)))`, plus one scope and N + 1
lease records. The 512/256 allowances have compiled-layout checks; the order
vector includes growth headroom. This is retained waiting-owner/outbox storage,
not a complete peak-heap guarantee. Existing commit scratch is bounded by
runtime object/pending-call limits; its composition-level byte declaration,
returned records, native futures and publication storage remain separate work.

Verification in `.work/node125-runtime-reservations/`:

- Final selected runtime tests: 142 passed (113 existing plus 29 new).
  With lifetime-handoff: 159 passed, including a new test that transferred
  lifetime ownership survives disposal of the original subtree. Feature-off:
  113 passed. New tests cover later descendants, both parent/child commit
  orders, outbox transfer/drain, pending-call cancellation
  ownership, typed errors, clones/concurrent admission, rollback, partial
  EventItem/metadata exhaustion, overflow, foreign runtimes and teardown.
- `closed-priority-red.log` records the initial full-pool/closed-transport
  precedence failure. Moving termination validation before admission makes
  that deterministic regression green without extra retries or deadlines.
- Strict Clippy passes selected, combined and ordinary builds. One public API
  documentation example passes. The dependency proof compares normal packages,
  versions and transitive features in both selected variants against ordinary
  runtime: no delta. Node dependency/boundary checks remain green.
- CI is configured for the selected/combined tests, documentation example,
  Clippy and dependency proof; no remote run or browser improvement is claimed.

Neither storage reservation proves object incarnation or native ancestor
retirement. Unknown/already-retired runtime IDs keep ordinary no-op semantics;
the coordinator must validate exact native ownership before choosing the ID
and deciding a public close outcome. Preparation consumes its reservation on
failure, so the coordinator must serialize short commits rather than polling
prepare against an occupied gate. Next compose both reservation layers with
bounded native intents, exact ancestor receipts, publication rebasing and full
peak ownership budgets, then rerun the still-red NODE-123 dispatch probe.
Node artifacts are unchanged; JavaScript remains language adaptation only.

## NODE-124: opt-in topology transition reservations

Implemented `pwright-backend-cdp-topology/transition-reservations` as an
executor-free, command-free optional module. No existing host/addon selects it.
The ordinary topology constructors, profile and exclusive transaction layouts
are unchanged. This is the topology-storage prerequisite for concurrent native
close intents, **not** a completed ancestor-close implementation.

`TransitionReservationProfile` binds one declared topology profile to an
independent extension scope and explicit nonzero pending limit. The caller
combines both requirements concurrently in its production plan. A bound
`TransitionReservations` pool shares an atomic limit across clones; it checks
exact owner identity before admission and releases the pending slot if byte or
lease-metadata admission fails. It does not borrow extra slots from spare bytes.
The shared pool itself is byte-admitted before allocating its Arc.

`ReservedTopologyTransition` retains a lease, pool permit and connection epoch,
but no shadow snapshot or mutation gate. At `prepare`, it validates the exact
owner/epoch, respects any existing exclusive transaction, and clones **current**
state using already-reserved capacity. It rejects a callback that changes epoch.
Successful preparation enters the existing exclusive owned transaction; commit
reuses its owner/gate validation and native-state lease transfer. Dropping or
failing preparation/commit releases the gate and reservation without changing
live state. A committed lease may outlive the pool until the next ordinary
topology mutation or owner drop. Slot ownership lasts through prepared commit;
abandoned reservations drop bytes before publishing a free slot.

Let M be the existing topology profile's maximum retained native-string bytes
and N the selected pending limit. Additional admission is
`512 + (N + 1) * (M + 128)` BackendStateBytes, one independent extension scope
and `N + 2` leases. The extra shadow covers a previously committed pool lease
while all N pending slots refill. The 512/128-byte allowances cover retained
pool/Arc and reservation/lease metadata; compiled-layout tests enforce them.
These are logical resource declarations, not RSS or complete heap accounting:
topology structural limits remain unchanged, and arbitrary callback/output,
runtime disposal and publication storage need separate caller admission.

Verification in `.work/node124-topology-reservations/`:

- Selected topology: 76 passing unit tests (49 existing plus 27 new); ordinary
  feature-off topology: 49 passing. New gates cover latest-state sibling
  preservation, children created after parent reservation, both sequential
  sibling retirements, concurrent slot contention, fresh/stale epochs, foreign
  owners/commits, preparation rollback, overflow, byte/metadata exhaustion,
  committed-shadow overlap and complete lease drain.
- Preparation succeeds even when the rest of the global byte budget is held;
  it does not obtain another byte lease after reservation.
- Strict selected Clippy passes. The new dependency proof compares all normal
  transitive packages, versions and features against feature-off topology;
  there is no delta. The original topology-only example still passes.
- One public-API Rust documentation example passes. All seven safe-host and
  eight addon resolved selections still pass the Node dependency boundary;
  the runtime JavaScript check finds no driver, CDP transport or process launcher.
- CI now includes the selected tests, strict Clippy and dependency proof.
  This records local verification plus CI configuration, not an executed
  remote CI run.

The Node binaries retain their NODE-123 hashes below. Its controlled ancestor
dispatch gate therefore remains unresolved; no new browser result is claimed.
Next implement runtime disposal/control-message capacity reservations and their
latest-state commit rules, then compose typed native close receipts, bounded
concurrent intents and publication rebasing. The coordinator must serialize
the short preparation/commit phase and preserve written native command owners.
This generic storage reservation is not an ancestor receipt, does not decide
already-retired child success, and consumes itself on failed preparation.

## NODE-123 evidence and current status

The overall Node compatibility goal remains active. This document records a
newly isolated discrepancy and the next composable implementation boundary; the
concurrent close coordinator described below is **not implemented**.

`scripts/probe-close-cascade.mjs` in its default transparent mode uses an unchanged-wire loopback relay and a
temporary Tailscale-bound dynamic response server. It records exact native
Target/Context identities, command acknowledgements, event identities, public
settlement times, and cleanup separately. It never ends the held response to
make close succeed. Reports are exclusive-created and limited to 1 MiB, with
32 selected pending commands/session mappings, eight created Targets/Contexts,
128 retained close/recent receipts, 64 native retirement events and 16 phases.
The original CloseTrace default still observes every command; an injected
selection now excludes unrelated initialization without spending pending slots.

Evidence: `.work/node123-close-cascade/oracle.json` and `candidate.json`.

| Response is still open | Playwright 1.58.2 | Rust Node candidate |
| --- | --- | --- |
| Context.close directly | Native Context disposal, Page/Context close events, navigation rejects closed | Same tested outcome; all live host/FFI ownership drains |
| Page.close, then Context.close after Page command acknowledgement | Sends Context disposal while Page.close is pending; disposal responds in about 5 ms; Page/Context close complete | Page command acknowledges; no Context-disposal command appears during the 10 s observation window; navigation times out at its unchanged 5 s API deadline |
| Browser shutdown after that candidate stall | Separate interrupted-operation contract | Releases pending work and native ownership, but initially lost Page/Context events |

The final row exposed a second defect. Rust previously suppressed in-flight
Context-close TargetClosed into success. The JS success facade then removed
Context/Page emitter identities before terminal publication. Pinned client
`BrowserContext.close` instead awaits the channel close without the Page-close
TargetClosed suppression. The host now preserves the in-flight Context error
and waits outside its operation token for terminal staging. Missing/already
retired Contexts remain idempotent, and the default CDP Context still closes only
the attachment. Controlled queued-Context owner/peer shutdown tests prove the
different Context/Page outcomes and original emitter identity. This fix does
**not** dispatch the blocked ancestor disposal.

### Reply-producer and queue-loss handoff

The first Context fix was insufficient: a later combined-profile repeat in
`combined-confirmation-final.log` failed `queued-context-peer`. Page.close had
already succeeded while `Page.isClosed` was false and no close listener had run.
The earlier 50-case green runs are retained, but do not establish race freedom.

Source inspection identified a shutdown window: the driver can drop its active
reply sender, or close/drain its request queue, before its normal stop block.
Mapping that producer/queue loss directly to TargetClosed previously allowed
the caller to treat it as an ordinary closed object on a still-live connection.
`complete_driver_reply` and `failed_driver_admission` now seal the existing stop
token at those loss boundaries. Page/Context settlement releases its operation
token before waiting for terminal staging. Actual driver replies are unchanged;
an ordinary TargetClosed reply does not terminate a live connection, and a full
queue still reports ResourceExhausted without stopping it. Four deterministic
unit tests cover those distinctions. This ordering fix adds no allocation,
subscription, task, dependency or resource allowance.

Producer-only addon repeats (`producer-*.log`) passed three six-case runs per
profile before the queue-loss addition. They are intermediate evidence, not the
final artifacts below. A successful stress repeat cannot prove all possible
schedules, and these fixes do not change the exclusive close transaction gates.

### Controlled dispatch proof and repeat interpretation

The post-fix transparent repeat (`candidate-terminal-fixed.json`) did not
exercise the pending-close boundary: Page.close had already settled before
Context.close began. Its success is not evidence that ancestor progress changed.

`PWRIGHT_CLOSE_CONTROL=hold-retirement` instead fully loads the response first,
then holds only exact owned-Page retirement packets (eight packets, 4096 bytes
each). It records native arrival separately from delivery, waits until Page.close
is demonstrably pending, asks for Context.close, and requires the disposal
command to reach the wire **before** releasing retirement packets. This is an
explicit synthetic isolation, not a transparent replay of the open-response
scenario or a production timeout.

`oracle-controlled.json` passes this ordering. `candidate-controlled.json` exits
1: during the 10 s dispatch observation, two host operations and two FFI closes
are active, three retirement packets are held, and no Context-disposal command
is sent. Release then allows both closes to settle and all owned targets/leases
to drain. Thus the remaining red gate proves head-of-line blocking independently
of a browser-side open-response stall, and is not an FFI admission shortage.

Final NODE-123 artifacts:

| Profile | SHA-256 |
| --- | --- |
| default | `b5365c92df52f91b1c8ba4c4e092872c69ab9e2b336aa9934610a374a4c8586f` |
| drag-frame-extra-headers | `6c6f9bb95296bd5a47cd271d2453feff56961b7a60bec03007cb34ae8d4681aa` |

Both artifacts pass the close/connection/event matrix: confirmation 6, events 7,
listeners 18, Page.isClosed 8, terminal dispatch 4, Browser connection 5,
collection lifecycle 1 and driver attribution 1 (**50 each**). Each profile also
passes two extra full six-case confirmation repeats: 124 total final-profile
test executions, not 124 unique behaviors. Final logs use `queue-default-*`
and `queue-combined-*`. Both builds include producer-loss and closed-queue
sealing. Host Rust tests pass 403/eight ignored in default and 451/ten ignored
in combined (`host-*-queue-sealed.log`). Node units pass 84, with types,
boundary/dependency checks, fmt and strict Clippy in both profiles also passing.
This green matrix does not include or supersede the deliberately red dispatch
probe above. No new production worker, subscription, dependency or resource
allowance is introduced by the terminal-settlement or driver-loss handoff fixes.

The final combined artifact was also checked with the controlled probe again:
`oracle-queue-sealed-controlled.json` passes, while
`candidate-queue-sealed-controlled.json` exits 1 only for ancestor dispatch.
At the candidate observation deadline Page.close is still pending, two host
operations/two FFI closes are admitted, three retirement packets are held and
no Context-disposal command has been sent. The oracle dispatches before release.
Both direct-Context cases pass; after releasing the held packets both candidate
cases reclaim all owned Targets, host leases and pending FFI operations. This
confirms the final binary's remaining discrepancy, not just an older artifact.

Run the diagnostic with explicit `PWRIGHT_TEST_CDP`, verified Tailscale
`PWRIGHT_TEST_BIND` and full-MagicDNS `PWRIGHT_TEST_HOST`, an absolute new
`PWRIGHT_CLOSE_REPORT` path, `PWRIGHT_CLOSE_SIDE=oracle|candidate`, and
`PWRIGHT_NATIVE_PROFILE=drag-frame-extra-headers` for the candidate:
`pnpm --dir packages/pwright probe:close-cascade`. Set the control variable above
only for the synthetic gate. Observation errors produce a nonzero exit and
remain separate from Browser/operation cleanup errors in the report.

## Why an extra spawned close task is insufficient

FFI is not a single-close scheduler: the environment admits 16 close tickets and
`FfiExecution::run` polls each admitted future. The host lifecycle loop instead
awaits `LifecycleServices::execute` before reading its next request. Selected
Page.close holds both `PreparedCdpPageClose` and `PreparedObjectDisposal` while
awaiting destruction. Both topology and runtime reject another lifecycle
transaction while those owners exist. Thus moving Context.close onto another
Tokio task alone would exchange a wait for a transaction-in-progress failure.

## Chosen implementation direction

Separate bounded **native close intent/receipt ownership** from a short-lived
**serialized retirement commit**, only in an explicitly selected composition.
Preserve the existing ordinary/v1 constructors. JavaScript remains wrappers and
EventEmitter behavior; it must not run the queue, signal native retirement or
repair collection membership.

| Phase | Retained authority | Must not do |
| --- | --- | --- |
| Admit intent | Exact connection epoch, portable/native target and ancestor identities; bounded command, receiver, future and maximum retirement/publication reservations | Send a close command before all required capacity is admitted |
| Await native work | Independent owned command/receipt futures, with bounded concurrent jobs | Hold the global topology/runtime mutation gate while waiting, or drop written command ownership |
| Commit retirement | Exclusive transaction against current authoritative state, using already-admitted storage | Commit an old whole-topology snapshot over concurrent retirement/creation |
| Publish and settle | Existing ordered lifecycle groups and FFI fences | Emit duplicate child close, expose all flags before callbacks, or settle against a pre-terminal fence |

An owned Context-disposal success is a potential **typed ancestor retirement
receipt**, not an arbitrary substitute for Page destruction. It must bind the
same connection epoch, exact native Context and verified Page ancestry. A
different Context, inspector detach or Context admission failure is not proof.
If the parent commits first, it publishes descendant groups in native ownership
order; pending child close intents join that committed retirement rather than
replaying it. Any still-written child command retains its response/termination
ownership independently. Native failure classification must remain visible.

Required additive pieces, before wiring this selection:

1. **Retirement capacity reservation without an exclusive mutation gate.**
   NODE-124 supplies topology native-shadow reservation; NODE-125 adds runtime
   waiting/outbox reservation and latest-state disposal preparation. NODE-126
   supplies deferred publication capacity. Complete commit-scratch/intent byte
   derivation and coordinator integration remain. These reservations must
   support admitted storage that can later prepare a
   latest-state commit without allocating beyond the reservation. Retaining an
   old cloned topology and clearing its gate is not sufficient: it can overwrite
   a concurrently committed parent/sibling change. Prove epoch/identity checks,
   reserve failure before native mutation, maximum control-message/observer
   capacity, and rollback on every failed preparation.
2. **Bounded close-intent coordinator.** Native waits run independently; commit
   ownership remains serialized. Explicit profile limits cover pending intents,
   command/receipt futures, extra fanout cursors, ancestor joins and parked caller
   replies. Derive exact global/per-scope requirements before choosing defaults;
   do not merely reuse unused ledger allowance or expand an unbounded queue.
3. **Ancestor receipt and publication composition.** Rebase close publication
   from the latest committed visibility, preserving already-published groups and
   creation acknowledgement semantics. Existing frozen observer views cannot
   simply be reused after another intent commits. Duplicate/already-retired
   child intents need a typed successful join only when retirement is proved.
4. **Cancellation and failed-parent continuation.** Failed Context admission or
   native disposal must not strand the still-pending Page close or fabricate its
   success. Browser/Worker stop must retire all written commands and bounded
   owners, stage terminal groups, and preserve Page versus Context error rules.

## Verification requirements

- Deterministic unit tests for both receipt orders, both commit orders, stale
  epochs/identities, different ancestors, exact capacity exhaustion, failed
  parent admission/native response, dropped callers, event loss and teardown.
- Prove no lost sibling/object state when retirement commits after another
  lifecycle operation, and no post-mutation unadmitted allocation.
- Controlled CDP gates for pending Page.close plus Context.close: Context
  disposal must reach the wire before Page destruction is released, without
  caller-issued Browser.close. Cover multiple Pages and unrelated Contexts.
- Preserve the six NODE-123 close-confirmation cases, both complete event/listener
  corpora, default-context/connection behavior, owned/borrowed cleanup and every
  selected dependency closure. Verify zero live native/FFI ownership afterward.
- Rerun the original Frame response-arrival scenario independently from the
  document-execution boundary. Preserve oracle failures and observer overhead;
  no stopLoading, HTTP-response completion, retry or expanded timeout workaround.

Until these contracts and gates are implemented, ancestor-close progress is an
explicit compatibility gap, even when Browser shutdown successfully reclaims it.
