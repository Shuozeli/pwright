<!-- agent-updated: 2026-09-10T19:29:31Z -->

NODE-139: A pending Frame.goto publication fence is subordinate to the exact Page lifetime, not close intent or renderer route currency. Page or registry sealing wakes the borrowed wait; synchronous result projection rechecks Page/connection liveness. Node close event production and serialized publication remain separate owners.
[Page lifetime publication contract and verification](pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Selected explicit closes now separate native progress from serialized local publication. Jobs retain replies/capacity even if callers disappear, and completion notification runs outside the host-state lock. Native-success/local-publication failure is terminal. Teardown drains written close commands before dropping their owners.
[Node close-driver integration and final verification](pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: The new native retirement API returns CommittedNativeClose so the driver can release LiveSession before publication and cancellation wakes. A sole logical publication owner is still required, and post-commit publication failure is terminal. Existing Node close sources remain serial until driver wiring.
[Native local retirement and post-lock notification contract](pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: Deferred close-publication capacity can travel with runtime/topology reservations through native scheduling. The selected consuming callback is synchronous and does not itself publish a Node group or run listeners. Full host serialization and post-lock event settlement remain separate.
[Joined close capacity and native completion contract](pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: A new runtime-locked local-publication callback supplies part of the short close transaction. It must never invoke Node listeners or other user callbacks under the lock; close groups and FFI settlement remain separate after native/local commit. The serial Node service is not yet switched.
[Runtime-locked publication/disposal contract and evidence](pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Runtime control outboxes can now preallocate message/lease backing before native retirement, preserving queued lifecycle messages and metrics. This produces no callback or public close group and does not select concurrent Node close scheduling.
[Preallocated runtime outbox contract and evidence](pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Actual native Page target destruction now marks selected retained-world lifetimes independently of local topology removal. This is cleanup evidence only: it neither emits a lifecycle callback nor completes the pending close coordinator/Node scheduler integration.
[Native Page world-retirement contract and evidence](pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: The new same-topology native commit scope supplies the serialization point for future close transactions. Deferred route/runtime notification owners must be returned from its callback and notified after the outer topology lock is released. Node request scheduling and public event publication remain unchanged.
[Native close commit-scope contract and evidence](pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Runtime commit now has a separately selected post-lock notification port, complementing route readiness deferral. The future close coordinator must release all outer locks before notifying, including post-commit error/teardown paths. This emits no Node close group and does not change the currently selected host.
[Deferred runtime close commit contract and evidence](pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: The new local routing phase returns readiness notification as a separate post-commit port, so the future coordinator can release topology/runtime/handle locks before reentrant wakers. It emits no Page/Context close group and changes no Node public settlement or event behavior yet.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Native scheduling now independently drives Page/Context commands, but does not mutate topology/runtime or emit close groups. Native completions are not public close results. The Node driver still needs short serialized retirement/publication commits and existing terminal fences.
[Bounded native scheduler contract and evidence](pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Exact native Context receipts can now certify preexisting descendant bindings without treating already-closed errors as destruction. They do not stage runtime/topology or Node lifecycle events. The driver still uses serial close execution; the coordinator and existing publication layer must be composed before Node behavior changes.
[Native ancestor authority contract and evidence](pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: The new Rust acknowledgement phase is not an event-publication receipt or public close success. Existing exact destruction waits and Node event staging remain in use; concurrent native scheduling/ancestor joins must be composed before changing lifecycle behavior. No new JS or addon artifact.
[Native close phase contract and evidence](pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred close planning reuses the existing lifecycle staging/drain and preserves Page-before-Context notification order. It is not yet called by Node's native close scheduler; current JS event behavior and binary evidence remain unchanged.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Adds unselected runtime outbox/disposal reservation with FIFO item-lease transfer and latest-state child-first removal. Parent/child ordering, pending-call cancellation and typed failure tests pass, including lifetime-handoff interaction. These runtime control messages are not Node close-event publication; the native coordinator and publication-group rebasing remain required. Node event behavior/artifacts are unchanged.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-123: Queued Context.close retains TargetClosed through terminal staging on host interruption, preserving Context/Page emitters. A later peer repeat exposed reply-producer loss before the normal stop block; producer/queue loss now seals admission before settlement, without treating ordinary closed-object replies or queue fullness as connection loss. Rebuilt profiles pass repeated queued-Context owner/peer gates and both event/listener corpora. Native ancestor-close dispatch remains unimplemented.
[Close cascade contracts and evidence](pwright-close-cascade.md).

NODE-122: Selected explicit Page.close now confirms exact native destruction before transaction commit. Cancellation/peer loss retire unconfirmed work without a fake close commit; terminal groups remain the event authority. A new Rust settlement fence fixes lost Page close listeners during that handoff. Both final artifacts pass events 7/7, listeners 18/18 and controlled close/Worker cases 4/4. This does not resolve the separate Frame arrival-boundary close stall.
[Selected confirmation](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: No production source/dispatcher changed. Transparent original-scenario tracing passes eight mutation and eight close-reentry samples per profile, and both complete listener corpora pass 18/18. Retain earlier failures. The next selected close confirmation must keep native commit before event publication and avoid waiting on the blocked topology loop.
[Transparent trace and next confirmation contract](pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: The production event source is unchanged. Full listener corpora pass 18/18 in both rebuilt profiles; a prior filtered close-reentry run failed in native target-absence confirmation. New Rust driver-phase diagnostics and a controlled initialization timeout prevent treating every browser.close failure as a close-command failure. Natural stability gaps remain open.
[Close attribution contract and evidence](pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

# Node lifecycle events and stepwise Rust publication

## NODE-119: Native ready and popup source

The selected Node Events composition now produces Context.page and Page.popup
from the native driver's committed PageReady and PopupLinked effects, using the
NODE-118 wrapper converter. Explicit, external and terminal close sources remain
selected. Ordinary safe-host collections and the terminal-only constructor retain
their narrower behavior. No Node driver, extra transport, task or subscription is
installed.

### Discovery is not readiness

CollectionBatch.commit_view accepts an exact visibility projection of its own
pre-admitted revision. Its selector runs outside the queue mutex while the existing
batch publication gate remains closed. Returning another revision, even from the
same resource domain with identical descriptors, fails without opening the gate.
The ordinary commit method still selects a fully visible revision.

The ready composition now retains a native descriptor revision and a separate
exact planned view. Reconciliation rebases existing visibility onto the new
revision, preserves Page masks, and reveals new Context membership independently.
New Page descriptors stay hidden until their PageReady group commits. This applies
to explicit creation, external discovery, and snapshot-only refresh. Missing
visible identities fail rebase; close causes must hide them first. A subsequent
full snapshot cannot publish unready Pages or re-show a previously closed Page.

The borrowed per-transaction observer first stages external disposals, reconciles
the admitted descriptor projection, then stages ready/linkage effects in their
native order. PageReady validates its actual owning Context and reveals only that
Page. An already-visible duplicate ready effect spends no group and emits nothing.
The source does not reconstruct readiness from a collection difference.

### Dependencies across native transactions

Every selected native Page has a separately admitted metadata-only creation
acknowledgement token. PageReady and later PopupLinked groups share this token;
they never retain a Host or transport. Pending, synchronously delivered and
synchronously failed delivery are distinct atomic states.

This preserves native effect order without looking ahead and moving a popup ahead
of intervening effects. A Context.page notice may be in an earlier transaction than
its PopupLinked cause when the opener becomes ready later. Successful synchronous
delivery permits that popup; a synchronous throw or interrupted unacknowledged
dispatch suppresses it before foreign conversion. The drain consumes a suppressed
dependency as progress and continues unrelated groups. A popup preceding its
creation acknowledgement is an internal error, not permission to emit early.

Async listener Promises are still not awaited. Their synchronous return acknowledges
delivery, so later rejection cannot retroactively suppress a popup. Existing Node
uncaughtException/unhandledRejection behavior and same-emitter listener semantics
remain EventEmitter-owned, while the dependency decision remains in Rust.

The source uses separate ordered groups for ready and linkage. The existing
single-group PageReady-with-dependent-popup primitive remains available to other
producers; both forms retain the same synchronous acknowledgement contract.

### Capacity and retirement

ReadySource pre-admits 32 lifecycle groups plus its fixed vector/box storage before
initialization: up to 16 Page-ready causes and 16 later popup links without another
creation. Explicit creation/discovery replenishes this pool before adding visible
objects. An abandoned creation restores its previous length; partial replenishment
also rolls back newly acquired external-close and legacy-retirement capacity.
Terminal, explicit close, external close and ready capacity are independent.

Each Page's acknowledgement Arc is charged before allocation and local activation;
explicit newPage reserves it before the native command. External discovery admits
it before indexing/public visibility, and failure propagates through existing
Session retirement. Staging native effects only moves existing reservations and
clones charged tokens. It performs no late admission for a ready notification.

Native index retirement releases its token owner. Pending groups may keep the token
until dispatch, but JS Page/Frame wrappers do not retain it. Terminal teardown uses
the exact planned masks, so queued ready commits before its close while never-ready
Pages remain unannounced. New optional index fields and group fields are included
in size_of-based admission even in colder compositions; no zero-byte opt-out or
unbounded resource promise is made.

### Verification

Four new masked-batch tests cover retained hidden descriptors, exact ready
predecessors, rejecting another admitted revision and preserving the failure gate.
Ten source tests cover deferred visibility, ordered ready/linkage, delayed opener,
throwing/interrupted dispatch, duplicate ready, exhausted fresh group admission,
abandoned creation capacity, and ready-before-close terminal handoff.

Current default host tests pass 390 with 8 ignored; combined tests pass 438 with
10 ignored. Strict combined all-target Clippy, 67 Node unit tests, TypeScript, all
seven host/eight addon dependency selections and formatting pass.

Both final native profiles pass the entire lifecycle corpus (7/7) and listener
corpus (18/18), including the formerly red explicit newPage, popup, mutation,
async-gate, popup-throw and async-reject cases. Their existing 39 collection,
Page/Browser close, Worker cleanup and Page-enumeration regressions also pass.
The corpus is no longer filtered around missing creation events.

Extra Frame-tree/lookup workloads exposed stale pre-NODE-115 cleanup expectations,
not an event-order discrepancy: a retained closed Browser now owns exactly one
fixed publication-port lease. The tests now require exactly one state and one
revision, keeping native runtime cleanup assertions intact. Frame lookup's obsolete
7,672-byte baseline is replaced with the measured current 64,600-byte fixed index,
queue and port baseline, also independently observed in Page enumeration. Initial
failed diagnostics are preserved. Full Frame results and final artifact identities
are recorded below.

Evidence is under `.work/node119-ready-source/`. The initial default addon
`068ea2736edca915b8a04d7854cf032b97958d18ebd7246ff6075a6dec763a5e`
already passed 7/7 and 18/18 before the source was simplified to preserve each
individual native effect's order; final reports must refer to the rebuilt artifacts.

Final artifact SHA-256 values:

- default: `45dd5974f66a75903425d397bf6829bbb537c86696e1fbd6fde209fe0aaf0cb5`
- combined: `25c5d5c2dce96fd43e003864b9615c63ce71637d103184e075e513f85f60bbb4`

Both profiles additionally pass public Frame-tree 18/18 and Frame lookup 41/41
with three boundary checks. The six Page/Frame JSON reports were checked against
those exact binaries and contain no mismatching cases. Page and Frame cleanup
report one fixed state, one port revision and 64,600 logical bytes, a 136-byte
fixed-index increase from NODE-118. All host runtime/resource/operation and
URL/pending-publication metrics return to zero; no ready acknowledgement remains.
The Frame lookup gate also requires that exact byte baseline rather than a range.

Explicit newPage and popup each pass three consecutive focused runs in each
profile. The six-case listener repetition subset passes all three default runs.
Combined repetitions report 5/6, 6/6 and 6/6: the first mutation case fails in
finally cleanup with browser.close: Timeout waiting for response, not a recorded
event-trace mismatch. That run exits 7 and remains in
repeat-listeners-drag-frame-extra-headers-1.log. The other two planned runs were
completed without restarting the shared browser or relaxing any assertion.
This is an unresolved native close/driver-response stability failure; full-corpus
18/18 results do not erase it, and it is not proven to have NODE-118's exact cause.

The extra Frame fixture directory was temporarily registered on the existing
serve-lib daemon at the verified Tailscale address, route /pwright-node119,
port 9398, with a 30-minute timeout. The default Frame-tree rerun initially refused
to overwrite an existing immutable report (EEXIST); the successful original report
was retained, and subsequent runs used new report names. Neither diagnostic was
treated as proof of a browser semantic mismatch.

Handoff: all NODE-119 build/test processes are terminal. The exact preflight Page
0A1F19465E13DB2CA99B78B040E157C4 was closed and verified absent. The temporary
fixture registration was removed; repository fixtures were not deleted. The two
remaining Chrome Pages belong to the XLQY preview and were left untouched.
Shared serve-lib PID 54590 remains available at 7,632 KiB RSS; no new daemon was
created. At 10:45:09Z shared backend PID 53189 RSS was 1,728,816 KiB, down from
5,166,208 KiB at 10:44:33Z, with zero disk swap. These are shared process readings,
not exact chat RAM. No staging, commit or push was made.

This completes the joined creation/close event corpus, not the full Node API
compatibility goal. Broader API coverage and NODE-118's intermittent native close
confirmation issue remain open. Passing these scenarios does not prove arbitrary
browser behavior, unlimited resources, all transports, or complete Playwright parity.

## NODE-118: Precommit creation-wrapper conversion

The Node publication port now understands ContextPage and Popup notices from an
immutable LifecycleNoticeView. This is the missing conversion half of ready/linkage
integration, not a newly selected producer. Current native compositions still
produce explicit/external/terminal close notifications only; creation/popup parity
must not be claimed until the authoritative ready/linkage source is joined.

### Prepare identity, then commit state

For ContextPage, the private JS callback returns a bounded descriptor request:
whether Context and Page identities are missing, and a synchronous preparation
function. It does not mutate caches or emit an event. Rust binds only the requested
Context settings, Page tree and root descriptors from the notice's immutable
revision. A cached Context/Page requests no new lasting binding. A request for a
new Context but a cached Page is invalid and fails before state publication.

The addon passes the admitted descriptors to the request's preparation function.
Injected factories construct the actual BrowserContext/Page/Frame wrappers; they
neither issue commands nor decide readiness. The resulting closure captures those
identities without installing them into Browser/Context caches. Only after the
exclusive native drain commits the notice does the closure insert the exact
objects and emit Context.page. A conversion failure cannot install a partially
prepared Context; a listener exception occurs after committed identity and retains
the existing acknowledge/original-exception handling.

Popup conversion resolves the Context from native metadata and reuses its already
announced popup Page and acquired opener wrapper. It does not synthesize an emitter
that the user never acquired. Missing popup identity with an acquired opener is an
internal ordering error, not permission to invent a Page or emit early. Rust still
owns which dependent notice follows successful Context.page delivery; JavaScript
does not implement a dependency queue or decide whether a listener throw suppresses
that native dependency.

Page root binding uses the immutable admitted CollectionPage descriptor rather than
a live Host route or a lookup in currently published membership. It therefore also
works for a not-yet-public ready Page or a retained descriptor whose native Page has
already retired. Both root/tree reservations precede wrapper conversion, and partial
failure releases earlier Rust admission without committing the pending notice.

### Resource boundary and tests

The existing 32-KiB port conversion allowance covers the extra request/call frames,
DTOs and bounded ID copies; its compile-time layout proof was extended. Context,
root and tree bindings separately own their lasting metadata-only leases. No new
queue, subscription, runtime owner, task, registry slot or dependency was added.
V8 allocation/GC remains distinct from logical Rust resource accounting.

Ten new Node unit cases cover empty and cached identity requests, hidden preparation,
postcommit identity/receiver, failed Page conversion without partial Context caching,
throwing Context.page listeners, exact popup identity and missing/unknown popup
ordering. Four native tests cover hidden descriptor binding, exhausted fresh binding
capacity preserving the uncommitted notice, binding after native/metadata teardown,
and releasing abandoned root/tree bindings. The initial teardown test cleared only
FrameTrees but expected the separate FrameMetadata active index to be empty; its
Arrange now follows the real Host teardown sequence and clears both owners.

Verification output is retained in `.work/node118-creation-conversion/`.
Node units pass 67; TypeScript and the seven host/eight addon dependency selections
pass. Host suites pass 376 / 8 ignored (default) and 424 / 10 ignored (combined).
Strict combined all-target Clippy, formatting and diff checks pass. Clippy's new
complex-function-type warning was resolved with a named CreationConverter type,
not an allow attribute; both final profiles were rebuilt afterward.

Initial default live regressions passed all 39 collection/close/cleanup cases,
but the full listener/event corpora additionally hit native close-confirmation
failures in close-reentry and owned Page close. Those runs report 13/18 and 4/7
respectively, and remain in default-lifecycle-listeners.log and
default-lifecycle-events.log. The errors precede foreign conversion:
Target.closeTarget returned, but the target remained listed for the existing
20 absence checks (injected 10-ms pauses between checks). This is evidence of
an unresolved close-confirmation timing case, not proof that the converter caused
it or that build load alone explains it. Final-artifact repetitions and full-corpus
reruns must remain separate from these original failures; no timeout/assertion is
relaxed to turn them green.

Both final artifacts pass all 39 collection/close/Worker-cleanup regressions and
return to the same complete-corpus results as NODE-117: 14/18 listeners and 5/7
lifecycle events. The remaining listener failures are mutation, async-gate,
popup-throw and async-reject; lifecycle failures are explicit newPage and popup.
Page close and close-reentry each pass three consecutive focused runs per profile.
These repetitions do not resolve or erase the earlier intermittent native-close
failure. No new production Page-ready conversion has been exercised by these
existing close-only sources.

Final native addon SHA-256 values, matched by the final Page-enumeration reports:

- default: `2b6fab7f121f288ef8f4b28e1f4410792a319c43e7eb2ede30b611a460284665`
- combined: `e26f5fa336c4a00ea32834ae28349ce2f6f4371576f94149509e5ff6e5707c70`

The retained closed Browser still has one metadata state, one fixed port revision
and 64,464 logical bytes. All host runtime/resource/operation and URL/publication
metrics are zero. These measurements verify unchanged close-path cleanup, not
unselected creation-path V8 reclamation. The four native tests separately verify
binding lease ownership; real creation-source integration remains required.

Handoff: NODE-118 build/test jobs are terminal. The exact owned preflight target
5502A97DA8FECE0CD6749281859E84B0 was closed and verified absent; one other Page
and shared Chrome were left untouched. No server/route was created or left running.
At 10:14:52Z shared backend PID 53189 RSS was 1,064,704 KiB, versus 5,664,928 KiB
at the preceding checkpoint; swap remained zero. The earlier targeted physical
footprint was 5.5 GiB with a process-lifetime peak of 13.6 GiB. These shared-process
readings are not exact per-chat RAM. No staging, commit or push was performed;
the full compatibility goal remains active.

### Next native source work

Consume PageReady and PopupLinked from the borrowed per-transaction driver hook;
do not infer readiness from collection differences. Reconcile admitted descriptors
without publishing the full new Page set ahead of its group. Preserve planned
visibility through snapshot-only refresh, external close and terminal teardown.
Handle deferred opener readiness as a later linkage cause, not a second PageReady.
Admit source storage before public/native activation and preserve failure/rollback
and dependent-listener semantics. Then verify real Node conversion together with
production source delivery; isolated adapter/native tests do not establish this.

Current source audit: ChromiumFrameOwnershipView.collection_pages delegates to
page_session_routes, which checks attached session/context ownership but does not
filter on PageReady. It cannot be used as an event-ready set. The topology's
mark-ready path emits PageReady once using ready_reported, then collects eligible
PopupLinked effects; opener readiness can cause those links in a later transaction.
The creation source must preserve that distinction across ordinary snapshots.

## NODE-117: Pre-admitted external-close source

The selected Node lifecycle composition now joins external Page/Context disposal
to the existing close dispatcher. FrameTrees consumes NODE-111's borrowed committed
driver effects before reconciling the resulting ownership snapshot. A snapshot-only
refresh does not replay causes. Ordinary safe-host connections and the narrower
terminal-only constructor do not select this source. No new queue, task,
subscription, dependency or JavaScript browser policy is introduced.

### Admission and native ownership

External disposal cannot be prevented by a late admission failure. The selected
source therefore reserves 32 lifecycle notices, plus one fixed pool-storage lease,
before initialization. The bound covers the collection's 16 Contexts and 16 Pages.
Each notice uses the existing publication queue's reservation and charged body.
The boxed pool's admission covers both its own layout and the vector backing
storage before either allocation. It is separate from explicit-close receipts,
legacy collection-retirement reservations and terminal-only capacity.

Before explicit creation or external discovery can add visible objects, the owner
replenishes the external pool. An abandoned creation restores its previous length;
partial replenishment releases newly acquired groups and newly replenished legacy
collection reservations. Successful creation retains the replenishment. The host's
existing serial topology/lifecycle driver owns this sequence; this is not a second
concurrent native writer.

During disposal the source only consumes existing reservations. It derives the
retired set from actual committed Dispose effects and the exact planned collection
view. Compact Context disposal means its visible Page subtree, followed by Context
close; a Page disposal means that Page alone. No allocation or new admission is
required on this path. Native tests verify unchanged factory metrics even when
fresh lifecycle admission is exhausted.

### Stepwise publication and failure

External and explicit sources share the same notice-staging operation. Each Page
group receives the native close token; its obsolete raw flag-publication permit
is consumed only after successful staging. The planned view is updated after every
group. Duplicate external effects, or a follow-up effect after explicit close,
cannot spend another token or emit another close for the removed member.

The existing Node dispatcher converts identity, commits native public metadata,
then emits and acknowledges without holding native locks. Page flags and collection
membership remain stepwise; a throwing first Page listener does not block sibling
Page/Context groups. Subsequent terminal teardown uses the surviving planned masks,
so it does not re-show or re-close previously staged objects.

If an invariant fails after the native external event, the source closes admission
and propagates the observer error into existing Session retirement. It cannot roll
back the browser event or silently omit notification. The separate terminal pool
remains available for cleanup. The insufficient-pool test injects an invalid state
deliberately and verifies no partial publication before fail-closed teardown.

### Verification and remaining work

Nine new native tests cover stepwise Context cascade, exhausted fresh admission,
duplicate receipts, explicit/external non-replay, abandoned and successful creation,
partial pool selection rollback, terminal mask handoff and fail-closed exhaustion.
Default host tests pass 372 with 8 ignored; combined host tests pass 420 with 10
ignored. Strict combined all-target Clippy, formatting, 57 Node unit tests,
TypeScript and all seven host/eight addon dependency selections pass.

Both native artifacts pass the same 39 real-Chrome regression cases: collection
publication (7), creation (7), lifecycle (1), Page.isClosed (8), Browser connection
(5), terminal dispatch/Worker cleanup (4), and public Page enumeration (7).
The complete lifecycle-event corpus advances from 4/7 to 5/7 in both profiles:
Worker-confirmed external Page close now matches Playwright 1.58.2. Its native
closure completes while Node is deliberately blocked, while public flags,
membership and listener count remain unchanged until Node resumes. The surviving
native sibling remains usable. Context cascade has native unit coverage; no new
dedicated external-Context live differential scenario is claimed.

The complete listener corpus remains 14/18. Its red cases are mutation, async-gate,
popup-throw and async-reject. The lifecycle corpus still fails explicit newPage
and popup creation. Full red outputs are retained and are not replaced by focused
green runs. Ready/creation and linkage sources remain unimplemented: wrapper
conversion must be admitted before publication, with Context.page preceding its
dependent popup notification and exact masks surviving partial delivery.

Evidence is under `.work/node117-external-close/`. The final native SHA-256
values match both Page-enumeration JSON reports:

- default: `b7a4fed81e6e5f31bf7e9b2510565e6fa21f449f499dd6b0336268ccf9103934`
- combined: `f5b2a96f4227806c0bedd75c76c55a5204524bc6bd220bd40a51df0c95220c00`

Each retained closed Browser reports one fixed metadata state, one port revision
lease and 64,464 logical bytes, an 8-byte fixed-layout increase from NODE-116.
This is admitted metadata, not RSS. All host runtime/resource/operation and
URL/pending-publication metrics return to zero; external pool leases do not remain.
No staging, commit or push was made, and the overall compatibility goal is active.

The external Page-close case additionally passes three consecutive focused runs
per profile, without replacing either full-corpus result. All NODE-117 build/test
jobs are terminal. The exact preflight target ABF08DB14FCF8E2BC010195CD7786FC0 was
closed and verified absent; one other task's Page remains untouched. No preview
server or route was created. Shared Chrome and other task services remain running.
At 09:55:30Z the shared Codex backend PID 53189 had RSS 1,803,376 KiB, down from
7,343,936 KiB at 09:45:55Z, with zero disk swap. Its targeted earlier physical
footprint was 7.6 GiB; neither reading establishes per-chat memory attribution.
This task leaves no build, test worker, server or owned browser target running.

## NODE-116: Explicit-close source integration

The Node composition now selects HostSession.connect_over_cdp_with_lifecycle_events:
terminal production plus native explicit Page/Context close receipts. The ordinary
safe-host connection remains collection-only, and the experimental terminal-only
entry point retains its narrower behavior. Creation, popup and external disposal
sources are not implied by this new selection.

### Native cause and cancellation boundary

Two observed attachment factories wrap the actual raw Page/Context close services
with NODE-112's observer, then place that wrapper **inside** the existing Continuable
close service. They preserve the attachment's exact browser client, topology,
runtime, Page routes, Handle/Handle-leaf route selection, and Page-close poll.
The host selects a typed ordinary/observed service variant; the ordinary composition
services are unchanged. No new transport, queue, task, subscription or dependency
is installed. These factories match the currently selected non-network Page-runtime
close factories; they do not silently replace the separate network-aware factories.

The observer receives the actual prepared topology effects before native CDP close.
It snapshots the exact planned collection view, validates descendant visibility,
and reserves every notice/body before allocation or native side effects. Dropping
an abandoned guard releases that admission without publishing state. Once native
topology/runtime/routes commit, the receipt consumes the real committed effects.
No operation result or foreign collection difference is used to invent an event.

A critical protocol detail: native Context disposal is a **parent-subtree**
operation. Production topology emits one Dispose(Context), not an obligatory list
of Dispose(Page) records. The source expands the actual parent cause using the
already admitted native ownership view. Page disposals are therefore selected by
their own effect or by their owning Context's effect. Context-close groups follow
their visible Page-close groups, including the empty-Context case.

The first live candidate rejected Context closure because it incorrectly required
each child disposal to appear separately. Production topology's own
prepare_browser_context_close test explicitly proves the compact parent receipt.
That assumption was corrected; the native incremental-flag test now uses only
Dispose(Context), not a synthetic expanded effect list. Initial failed diagnostics
remain in default-listeners.log, separate from corrected artifact evidence.

### Publication ownership and failure

Each successfully staged Page-close group takes ownership of that Page's raw
closed-flag permit. Subsequent native snapshot reconciliation skips this transferred
permit, so it cannot publish a duplicate raw closed flag before another Page's
callback. Native closed state and public closed state remain separate.

CollectionState keeps an exact planned CollectionView, updating it after every
successfully staged group. A subsequent complete native snapshot appends its
coherent collection revision after the groups and records that revision's full
view. If Session teardown happens before that refresh, it consumes the existing
planned masks and cannot re-show/re-close an already-staged Page.

Committed effects must match the admitted visible retirement set and the planned
view must still be identical. Postcommit failure cannot roll native close back:
the source closes the FrameTrees admission gate so the following reconciliation
retires the Session. The last successfully staged mask survives for terminal
cleanup. Precommit admission failure does not close that gate or mutate Page flags.
This source still relies on the host's existing serial topology/lifecycle ownership;
it is not a second concurrent topology writer.

All Node callbacks use NODE-115's existing conversion/commit/emit/ack protocol.
Synchronous listener throws preserve the original Node exception and stop only
same-emitter listeners. Independent Page/Context groups continue; operation
Promises are not rejected with listener exceptions. The JS adapter was not given
new close-state authority, a diff algorithm, or a browser command implementation.

### Resource proof and verification

The metadata factory admits each observer Arc and each boxed native receipt.
The receipt's storage covers its fixed vector of at most 32 notice/reservation
pairs; each notice separately owns the existing queue's lifecycle reservation.
The count follows the immutable collection's 16 Context / 16 Page bounds.
Partial reservation failure releases earlier groups. Terminal-only reservations
are not consumed by ordinary close. The planned-view field changes fixed owner
layout, automatically included by size_of-based index admission even in the cold
composition; selecting close events allocates the observer storage separately.

Seven new native tests cover abandonment, compact Context cascade admission,
stepwise closed flags with a throwing first listener, partial group-capacity
rollback, mismatched committed effects and fail-closed teardown, raw-permit
transfer during reconciliation, and terminal handoff of unpublished close masks.
Host suites pass 363 / 8 ignored (default) and 411 / 10 ignored (combined).
Strict combined all-target Clippy, 57 Node unit tests, TypeScript and the seven
host/eight addon dependency selections pass. No Node runtime driver dependency
is introduced into the safe host.

Default real-Chrome evidence advances the listener corpus from 12/18 to 14/18:
close-reentry and context-close-page-throw now match Playwright 1.58.2. Four
listener cases remain red: mutation, async-gate, popup-throw and async-reject,
all waiting for ordinary creation/linkage sources. The separate lifecycle corpus
passes owned Page/Context/Browser close and borrowed Browser disconnect (4/7);
explicit newPage, popup and Worker-confirmed external Page close remain red.
These full-corpus runs intentionally exit nonzero; red cases are not filtered out.

Evidence is under `.work/node116-explicit-close/`. Corrected default regressions
pass collection publication (7), collection creation (7), collection lifecycle (1),
Page.isClosed (8), Browser connection (5), terminal dispatch/cleanup (4), and public
Page enumeration (7). Both final profiles pass all 39 of these regression/cleanup
cases, plus the same 14/18 listener and 4/7 lifecycle-event results. The newly
joined close-reentry and context-close-page-throw scenarios each pass three
consecutive runs per profile; their focused repetition does not replace the
separately retained whole-corpus red results.

Final native addon SHA-256 values:

- default: `b2c6e9894c6940a494dfc70b4bd4836816e35d02c57842a80209e9d409c41501`
- combined: `cfb4b619a501e3d66ff5e060948ca478532024513cf1358383585523c3276785`

The default/combined Page-enumeration JSON reports match those exact artifact
hashes. Each retained closed Browser reports one fixed metadata state, one port
revision lease and 64,456 logical bytes: a 24-byte fixed-layout increase over
NODE-115, not an RSS measurement. All host runtime/resource/operation metrics
and URL/pending-publication metrics return to zero. Source observer/receipt
leases do not remain after retirement. Strict final Clippy, formatting, type,
unit and dependency outputs are retained with the two host test logs and full
corrected/default and combined live logs. No commit, staging or push was made.

Handoff: NODE-116 build/test processes are terminal; the exact preflight target
74F22437926F85F24F27AD951DECC9FC was closed and verified absent. No route/server
was created and shared Chrome remains available. At 09:33:43Z the shared Codex
backend PID 53189 had RSS 1,228,432 KiB, with zero disk swap. The concurrent large
rustc PID 43170 belongs to the XLQY server checkout, verified by its Cargo parent's
working directory, not this task. No exact per-chat memory attribution is claimed
and no shared service was stopped.

### Remaining source work

Join the borrowed committed driver-effect source for external Page disposal and
ready/linkage. External native events cannot be prevented by failed late admission,
so their notification storage must be admitted before their objects can retire.
Creation/popup conversion must admit new JS/native wrapper identities before state
commit and preserve the Context.page -> popup dependency rule. Keep terminal
capacity independent and continue carrying exact planned masks; do not revert
to full visibility when some ready notifications are still pending.

## NODE-115: Environment-thread terminal dispatch

The Node addon now selects NODE-114's terminal source at connection setup.
Every asynchronous publication path uses an explicit PublicationPort: connection
conversion, Session methods, signal watches, Locator reads/actions/evaluation,
and deferred binary evaluation. Rust worker futures capture only FramePublications
and fixed-watermark fences, never Rc, Node references, callbacks or JS values.
The standalone safe-host connection still does not opt into terminal events.
This is terminal event support, not completion of ordinary lifecycle parity.

### Dispatch and exception contract

The environment thread acquires the exclusive native drain, advances ordinary
metadata, then prepares each pending notice by resolving existing JS wrapper
identity. The internal JS adapter returns an emit closure or undefined; it does
not derive event causes, diff collections, wait for readiness, or own native
state. Unacquired wrappers have no user listeners and are not synthesized at
shutdown. Creation/popup notices are explicitly unsupported by this adapter
until the corresponding source/conversion composition is implemented.

Rust commits the notice's collection view and closed flag immediately before
invoking the emit closure. No native mutex or RefCell borrow spans foreign code.
The closure removes the exact wrapper cache entry and calls the existing Node
EventEmitter with the same receiver and event argument as Playwright.

Raw Node-API invocation retrieves and clears a synchronous thrown value without
converting it into napi::Error. Rust acknowledges ListenerThrew before calling
napi_fatal_exception with that exact value. Same-emitter listeners stop according
to EventEmitter semantics; independent Page/Context groups still progress.
Returned Promises are not awaited, so async rejection stays an unhandledRejection,
not a rejected browser.close operation. A listener can synchronously start another
browser.close without acquiring a second drain inside the current callback.
Internal preparation/drain failure instead discards queued publication, releases
the callback and requests native shutdown through a Weak Host capability.

### Admission, retained wrappers and environment cleanup

The environment owns one fixed sixteen-slot weak port registry. Its separate
administrative lease is bounded by 16 KiB and covers the Rc/RefCell registry,
shutdown's temporary slot array, and empty Rc allocations that weak slots may
retain after the last strong port is gone. It owns no FFI task token and cannot
make FfiBudget.shutdown wait for itself.

Each live port admits 32 KiB plus FrameConversion bookkeeping from its Session's
metadata factory before allocation. A compile-time bound covers the fixed body,
reference holder, IDs and synchronous conversion/dispatch allowance. These are
logical reservation bounds, not V8 accounting or measured RSS. The port owns no
transport/runtime; its host reference is Weak. Node callbacks capture only a
WeakRef to Browser. Extra strong port clones in pending calls remain covered by
the existing per-operation future/result allowances.

Terminal completion drops the Node callback synchronously. Metadata-only port
storage lasts with the last retained Browser/Locator/watch/conversion owner:
a closed Browser therefore retains exactly one revision lease for this fixed
body, whereas NODE-114's collection-only addon expected zero. The Page enumeration
test now asserts that exact lease rather than weakening the assertion to an
upper bound. Native resource leases, FFI settlements and pending publications
must still reach zero. Finished weak slots are reusable even while their closed
Browser wrappers remain retained.

Environment cleanup clears publication callbacks before shutting down deferred
conversions or joining native I/O. It performs no JS event dispatch. The existing
native owner discards pending groups and waits for runtime retirement. If Node
connection conversion cannot admit/publish its port, it discards that endpoint
and requests shutdown rather than leaving an undrainable Session record.

### Verification checkpoint

The first live artifact advanced the original listener corpus from 6/14 to 8/14:
Browser-close Page-listener throw and Context-listener throw now match the pinned
Playwright 1.58.2 oracle. Four new paired cases also match: a Page listener throwing
an object, throwing a number, asynchronously rejecting, and synchronously starting
another Browser.close. The expanded corpus is 12/18, with the original six
ordinary-source failures still explicit. Whole-corpus runs intentionally remain
nonzero until those missing capabilities are implemented.

Ten pure JS adapter tests cover preparation versus emission, original wrapper
identity/receiver, exact cache removal timing, three thrown-value types, listener
mutation/once semantics, missing wrappers, collected Browser and unknown notices.
Three Rust registry-bound tests bring host suites to 356 passing / 8 ignored
(default) and 404 passing / 10 ignored (combined features). Strict combined
all-target Clippy, 57 Node unit tests and public TypeScript checks pass.

The new terminal-dispatch gate terminates a Worker with live native ownership,
then another with an already-produced terminal cut while JS is synchronously
blocked. Cleanup must not invoke any registered listener, leak its owned Context,
or close external Pages. A third test retains twenty sequentially closed Browsers
to exceed the sixteen active-slot bound; fixed metadata stays constant, all
native resources/FFI results/publications retire, and subsequent admission works.
The fourth case drops an actual CDP relay connection and compares Page/Context
state at their close callbacks and Browser disconnected against Playwright;
the externally owned browser/Page remain usable after the client connection dies.
Existing collection-publication (7), collection-creation (7), Page.isClosed (8),
Browser connection (5), and public Page enumeration (7) regressions also pass in
the initial default build. Final profile receipts are recorded below.

Evidence directory: `.work/node115-terminal-dispatch/`. Preliminary logs are
kept separately from final artifact receipts. No commit or push is made here.

Repeated execution exposed a test synchronization defect in
`default-terminal-3-final.log`: native connected=false closes the admission gate
before teardown necessarily produces its terminal cut. The queued-Worker test
now waits, while Node stays blocked, for native driver/lease retirement **and**
pending publication, retaining its original five-second bound and final closed
flag/queue assertions. It does not add a blind delay or weaken the assertions.
Both profiles then passed all four terminal cases on three consecutive runs:
`{default,drag-frame-extra-headers}-terminal-{1,2,3}-verified.log`.

Final receipts (both artifacts rebuilt from the final Rust dispatch code):

| Gate | Default | drag-frame-extra-headers |
| --- | --- | --- |
| Collection publication / creation / Page closed / Browser connection | 27/27 | 27/27 |
| Public Page enumeration | 7/7 | 7/7 |
| Terminal cleanup, slot reuse and actual transport loss | 4/4, three runs | 4/4, three runs |
| Expanded paired listener corpus | 12/18; six known failures | 12/18; six known failures |

The reference-only listener corpus passes 18/18. Missing paired cases remain
mutation, async-gate, close-reentry, popup-throw, async-reject, and
context-close-page-throw. The first five require ordinary creation/close sources;
the sixth requires explicit Context-close Page/Context production. They are not
new terminal-dispatch regressions. `*-lifecycle-listeners-final.log` preserve the
complete failing comparisons rather than filtering them out of the gate.

Final addon SHA-256 values:

- default: `c5243ba10eb0866abba74735ef5384d57c41b1699338bcaac18b7d9919f76093`
- combined: `2f3b24ae5cb33a2ddf9f89cf42f1fbe7b902199de620f87ff6d7927536c9b606`

`*-pages-final.json` records the exact corresponding artifact. In both profiles,
the retained closed Browser sample reports one metadata state, one revision and
64,432 logical bytes (existing fixed index plus the new retained port allowance);
all host resource/operation metrics and all URL/publication metrics are zero.
This is not a process-memory measurement. `clippy-final.log`, host default/combined
logs, `unit-verified.log`, `types-verified.log`, `boundary.log`, and `fmt-final.log`
contain the remaining native, language and dependency verification receipts.

Handoff: all NODE-115 build/test/relay/Worker processes exited. The exact owned
preflight target ABC152DCB2C976859EA956D2714B3855 was closed and verified absent;
the dedicated Chrome process remains available with no Page targets. No static
route or server was created. Shared services were not restarted or stopped.
At 09:09:48Z the shared Codex backend PID 53189 had RSS 3,781,936 KiB; at
09:10:01Z vmmap reported 4.1G physical footprint and a process-lifetime peak of
13.6G. These non-simultaneous measures differ and cannot be assigned to this
chat. System free-memory percentage was 71%, disk swap zero; the turn observed
substantial shared-backend fluctuation and performed read-only follow-up checks.

### Remaining work

Join ordinary committed ready/linkage and prepared/committed explicit-close
receipts to exact planned visibility, including dependent Context.page -> popup
groups. Preserve the current terminal source's reserved capacity and pass its
exact planned masks once ordinary sources can leave partially visible revisions.
Do not claim Context.page, Page.popup, or explicit Page/Context close event parity
from these terminal results. Internal callback-conversion failure paths are
fail-closed but do not yet have an injected Node-allocation-failure live test.

## NODE-114: Pre-admitted native terminal source

The actual SessionTeardown -> FrameTrees.clear path can now produce terminal
Page/Context lifecycle groups. This is a selected native source, not a synthetic
collection-diff journal and not a JS callback. The new experimental Rust
HostSession.connect_over_cdp_with_terminal_events entry point selects it before
topology initialization. Ordinary connect_over_cdp and the Node addon remain on
the previous collection-only path. This partial composition does not replace
the goal of ordinary lifecycle events and full Node FFI delivery.

### Admission and ownership

TerminalLifecycle reserves all 32 possible Page/Context groups before native
initialization, in addition to the existing collection retirement reservations.
Neither ordinary creation nor explicit close can spend this terminal-only pool.
There are at most 16 Contexts and 16 Pages in an admitted collection view, so a
terminal cut needs no more than 32 groups regardless of earlier churn. Each
group already owns its queue EventItem and charged body/storage Arc. A separate
factory lease covers the fixed reservation vector and its owner; partial
selection failure releases all earlier admissions and fails connection setup.

On the aarch64 test build, selection adds 33 factory revision leases and 13,448
logical bytes. The collection owner gains one optional box field; its fixed
index admission is derived from size_of and includes that field even when this
source is unselected. The default path allocates no terminal pool. These are
layout/ledger allowances, not RSS or allocator measurements. No dependency,
feature, protocol tag, queue, worker or subscription is added.

### Actual terminal production

After stopping the serial native driver, FrameTrees takes the terminal owner
and its last coherent collection revision before clearing native indexes. For
the currently selected collection-only producer this is a full visibility view.
It passes that metadata-only view to the terminal source. Future ordinary event
producers must pass their **exact planned visibility masks**, not construct a
new full view that reveals not-yet-ready Pages. The low-level source accepts a
CollectionView for this reason; that future join is explicitly unfinished.

The source marks native Page trees closed using the existing poison-recovering
teardown operation, and consumes their already reserved legacy close permits.
Visible Pages receive group close tokens instead of duplicate raw close entries.
Native Pages absent from the coherent visible collection keep a raw close token
but receive no invented Page event. The source then appends one Page-close group
per visible Page and one Context-close group after that Context's visible Pages,
all at the reserved MAX watermark. It makes no new allocation or reservation.
Validated immutable collection identities and caps bound every mask transition
and reservation pop. The cleanup path recovers poisoned locks as before; this
does not make later ordinary reads of a poisoned endpoint safe or retryable.

Each group independently uses NODE-113 conversion/commit/acknowledgement. A
ListenerThrew acknowledgement does not drop sibling/Context groups. Old fences
cannot cross the terminal watermark. Complete publishes disconnected readiness
only after all groups have been consumed; publish_disconnect remains the existing
one-shot state transition, not a newly manufactured callback. Repeated host close
does not reconstruct another terminal pool or replay events. Discard executes no
foreign code and releases pending group bodies. Failure before initial collection
projection falls back to the existing metadata-only native teardown, since there
is no established public membership to notify.

Review identified a failed-reconciliation edge: a collection batch can stage
some raw Page-close tokens and fail before committing its projection. Its batch
gate has never made those tokens publishable. At terminal handoff, raw close
tokens *after that failed batch's start* for Pages in the terminal visible cut
are removed, transferring those flag publications to the independent terminal
groups. Otherwise the first Page callback could already observe several closed
Pages. Earlier committed entries and unrelated metadata are preserved. A focused
test stages two closes inside an abandoned batch and verifies three terminal
groups with incremental flags and no publication through the old fence.

### Evidence and remaining work

Ten deterministic tests cover maximum 16+16 membership, a throwing first Page
acknowledgement, ordinary queue saturation, old fences, pre-staging discard,
partial admission rollback, empty membership, pre-initialization selection and
release on early clear, and abandoned-batch handoff. The real-Chrome test uses the experimental HostSession
entry point, creates one owned Context/two Pages, and calls actual host.close.
It observes incremental closed flags and membership through the public drain,
simulates the first owned Page's ListenerThrew result, checks both Page groups and
their Context group, retained membership, one-shot disconnected state and a second
close with no replay. It does not execute an actual JS listener.

Cleanup resolves the exact owned native Context solely for cleanup identity,
stops the same host even on observation timeout, and removes that Context only
if it still exists. It preserves external Chrome and unrelated Pages. Final
source verification after the handoff fix:

- Host release tests: 353 default passes (8 live tests ignored), 401 combined
  drag-runtime/frame-navigation/extra-headers passes (10 live tests ignored).
- Real native terminal test: three consecutive runs per final compiled profile
  pass, each reporting two owned Page groups and one owned Context group. Earlier
  default/combined passes and repetitions are preserved separately, not counted
  as tests of the final handoff fix.
- Strict combined composition/host/addon all-target Clippy passes. Node unit
  tests pass 47/47; types, 7 host and 8 addon dependency proofs, workspace fmt
  and diff checks pass. No Clippy warning is suppressed.
- Both final rebuilt Node artifacts pass collection-publication 7/7,
  collection-creation 7/7, page-is-closed 8/8 and browser-connection 5/5: 27/27
  per profile. Both listener gates remain 6/14, with the same eight missing
  Page/Context notification scenarios and six passing Browser exception cases.
  This is expected while Node still selects the ordinary metadata-only source;
  it is not evidence that terminal groups are dispatched through Node.

Final addon SHA-256 values:

- default: `b32a9e428fd333d43ac26b0d17c0c4a29223efa21365b336e90d1023f4bd31a4`
- drag-frame-extra-headers: `ae0e249ae1b95bdc183368a2208bbf8cfe6039ed1569d7ff89858953db08eff6`

Evidence resides in `.work/node114-terminal-source/`: host-*-verified.log,
live-*-verified-{1,2,3}.log, clippy-verified.log, build-*-verified.log, layout.log,
default/combined collection-publication, collection-creation, page-is-closed,
browser-connection and listeners logs, boundary.log, node-unit.log, node-types.log
and fmt-verified.log. Earlier intermediate builds/tests remain separately named.

All build/test jobs are terminal. The exact owned preflight target
0B6BDF196D44D209D53EC9E4A4872344 is verified absent; the unrelated XLQY target
and shared Chrome/serve-lib services are preserved. No task-owned background
process, new commit or push is left by this checkpoint.

Memory observations concern shared backend PID 53189, not this chat alone. RSS
was 3,276,288 KiB at 08:08:26Z, 1,375,936 KiB at 08:25:32Z, 4,872,448 KiB at
08:34:24Z and 2,424,320 KiB at 08:38:48Z. Targeted vmmap snapshots reported a
7.9G physical footprint at 08:34:35Z and 320.5M at 08:37:57Z; its 13.6G peak is
since process launch, not a per-turn peak. These samples are not simultaneous
and RSS/physical footprint/compression are not interchangeable. System disk
swap remained zero; memory-pressure free readings were 59% then 70%. The large
fluctuation was reported and checked read-only. No per-chat attribution or
causal memory reduction is established, and no shared process was restarted.

Strict Clippy initially rejected an unnecessary boxed self parameter in terminal
staging. The caller now moves the admitted body out of its box; its storage lease
continues through staging. No warning suppression or new allocation was added.

Remaining: join NODE-111 ready/linkage and NODE-112 explicit-close receipts to
planned visibility, pre-admit ordinary group retention, add bounded environment-
owned foreign dispatch for every FFI fence path, and verify the resulting Node
behavior against the pinned Playwright oracle. The new terminal constructor is
not selected by Node and must not be advertised as public Node event support.

## NODE-113: Bounded dependent publication groups

The isolated Playwright 1.58.2 listener reference now passes 14 scenarios.
Three additions close a two-Page Context or Browser with a throwing first Page
listener, or throw from Context close during Browser close. A synchronous throw
skips remaining listeners of that event, but independent sibling Page, Context
and Browser disconnected events continue. Each callback observes incremental
closed flags and collection membership, with Browser connected until its own
disconnected event. The close Promise resolves. Only the diagnostic Page array
is normalized to the first native close; no sibling close order is imposed and
no wrapper/Context collection is reordered. The original error object reaches
uncaughtException. Existing dependent popup and async-listener contracts remain.

### Implemented Rust primitive

FrameChangeValue now holds an optional, boxed lifecycle group in the **existing**
fixed-capacity queue. No second queue, background loop, subscription or runtime
owner is introduced. LifecycleReservation admits a queue EventItem plus the
concrete group/reservation/conversion-body allowance before its Box allocation
and native work. A pre-admitted storage Arc keeps the temporary conversion
allowance alive even if conversion synchronously discards the queue; it is
released when conversion returns or unwinds. CollectionReservation.into_revision retains already charged
descriptors without publishing a full-visible collection replacement, releasing
its unused queue slot. Only explicit callers select these operations.

LifecycleCause supplies PageReady, deferred PopupLinked, PageClosed or
ContextClosed from an authoritative native producer. PageReady rebases the
previous view, preserves visible identities/order, reveals only the new Page
and its parent if needed, then provides context.page and an optional dependent
popup. PopupLinked validates two distinct visible Pages in one Context and
does not manufacture another context.page. PageClosed validates the close token
against the exact retained tree and hides that Page. ContextClosed rejects a
Context with visible Pages. Each independent close is a separate group.

Staging returns a planned view for the producer's next ordered group, not a
public collection. The queue domain and reservation are checked at staging;
the exact predecessor revision identity and membership masks are checked again
at publication. These checks do not establish native chronology: the producer
still owns source ordering and must not enqueue duplicate legacy close changes.

The exclusive fixed-watermark drain now has three phases:

1. with_next_notice borrows charged descriptors for wrapper admission/conversion
   outside all native locks. Failure leaves the notice and public state intact.
2. commit_notice installs the Page closed flag and collection visibility before
   foreign emit, advances the notice once and requires acknowledgement. It
   returns without native locks. Stale predecessors or metadata commit errors
   discard the endpoint instead of replaying/regressing partially published state.
3. acknowledge_notice accepts the synchronous Delivered or ListenerThrew result.
   Delivered continues a dependent popup. ListenerThrew drops only the remaining
   dependent notices, never later independent groups. An async listener's returned
   Promise is Delivered here; rejection handling stays on Node's async channel.

The group remains in the queue through acknowledgement, retaining its slot and
body lease. is_drained therefore cannot report completion while a callback is
outstanding. Dropping before commit preserves the notice; dropping after a
successful acknowledgement preserves the next uncommitted popup. Dropping with
an outstanding acknowledgement abandons only that group, since an interrupted
foreign emit must not be replayed. Drop wakes waiters. Explicit discard releases
queued bodies even if the empty drain handle survives; no callback is executed.
Legacy whole-fence publication refuses an encountered group **before popping it**.
Ordinary metadata advancement returns NoticePending at a group and
AwaitingAcknowledgement after commit, rather than silently discarding notices.

### Verification and remaining integration

All 23 new deterministic group tests pass. Final full host release suites pass
343 default tests (7 explicit live tests ignored) and 391 combined
drag-runtime/frame-navigation/extra-headers tests (9 live tests ignored).
Strict combined composition/host/addon all-target Clippy passes without warning
suppression. Node unit tests pass 47/47; types, 7 host and 8 addon dependency
proofs, JS syntax, workspace fmt and diff checks pass. Native probes/real Chrome
tests marked ignored were not run by these host suites.

The initial compile used a nonexistent test-only is_published_closed name; it
was corrected to the existing FrameTree.is_closed getter. An intermediate full
run had 342 passes and one test-only layout failure: a guessed FrameChange size
of 120 disagreed with the measured 88 bytes. The assertion was replaced with
the actual invariants (unchanged enum/guard layout), and the final suites above
pass. Do not confuse either harness correction with a production behavior fix.

On this aarch64 build the group admission is 344 logical bytes, including its
body, reservation, storage Arc and one temporary conversion allowance. The
FrameChange variant remains 24 bytes, queue item 88 bytes and exclusive drain
24 bytes; the eight-slot fixture queue allowance remains 904 bytes. No selected
group means no new group allocation. These are ledger/layout allowances, not
process RSS or allocator measurements. The callback-discard test verifies that
conversion's allowance remains charged until its last temporary reader returns.

Logs are under `.work/node113-publication-groups/`: reference-listeners.log
(14/14), host-default-verified.log, host-combined.log, clippy.log, layout.log,
node-unit.log, node-types.log, boundary.log and fmt-final.log. The expanded
paired listener gate was also run with the unchanged NODE-112 default addon
SHA-256 `e93e688a6f2514bed14bb7ab97a46244f9d2abd321330b16eae0b2c8e4682861`:
6/14 pass, 8 fail for missing Page/Context events. That artifact was **not rebuilt
in NODE-113**; this is red harness coverage, not an end-to-end run of the new
Rust primitive. The three new failures join the five previously known event
failures; Browser exception cases still pass. Full public lifecycle parity
remains unproven.

At 08:07:01Z all test/build jobs have terminated and the exact owned preflight
target B6020F2306FEA6A5234FA62DDE5B52E4 is verified absent. The unrelated XLQY
target and shared Chrome/serve-lib services are preserved; this checkpoint
leaves no task-owned background process. Shared backend PID 53189 RSS is
1,886,560 KiB, down from 3,664,272 KiB at 07:50:20Z, with zero swap throughout.
A targeted vmmap check and 71% memory-pressure free reading did not establish
a per-chat culprit; shared-process RSS cannot be attributed to this chat.

This is **not yet selected by the native host producers or Node callback port**.
No additional Page/Context JS compatibility is claimed. Remaining work is to
join NODE-111 driver and NODE-112 explicit-close inputs with planned views,
reserve all terminal group bodies before Page/Context creation, and replace all
FFI full-fence delivery paths with one environment-owned, weakly referenced
dispatcher. Terminal staging cannot use the fallible ordinary stage method after
teardown; saturation-safe terminal publication remains separate work. Do not
wire a producer before its foreign consumer can handle NoticePending.

## NODE-112: Pre-admitted explicit close receipts

ObservedChromiumCloseCommandService is an opt-in decorator for the raw Chromium
Page and BrowserContext close services. Ordinary services keep their existing
field layout; they select no observer, receipt box, journal, subscription or
extra task. The decorator composes inside the existing Continuable*Close service
so abandoned started commands keep the same native future and admission.

The native close services now preserve the CdpTopologyUpdate returned by their
prepared topology commit until the runtime/routes and optional network
retirement finish. Previously that output was discarded. No disposal order,
native confirmation rule, ownership rule or lock acquisition order is changed.
Existing network cleanup messages/counts retain their previous treatment.

### Two-phase native contract

1. ChromiumCloseObserver.prepare receives a borrowed ChromiumClosePlan containing
   the typed Page/BrowserContext target and exact prepared topology effects.
   Topology/runtime transaction reservations are already active, but no native
   close has been sent and their mutexes are not held. This is not a success
   receipt. The selected implementation must admit its returned box and every
   retained payload/metadata byte before allocating or starting native work.
2. Native close/absence confirmation and local publication use existing services.
   Errors drop the returned non-cloneable reservation; they do not manufacture
   a committed receipt. Default/external Contexts do not enter this destruction
   observer. Already closed Pages fail before a second observer admission.
3. The consumed ChromiumCloseReservation.committed receives the actual committed
   effect slice, after topology/runtime/routes/handle and selected network
   retirement, outside their locks. No state snapshot or JS collection diff is
   used to reconstruct the cause. Tests also check that the effect slice keeps
   its original prepared allocation, rather than a new cloned vector.

Preparation and completion are synchronous **native journal inputs**, never
foreign callbacks. Prepare must not reenter lifecycle while transaction
reservations are active. Completion does not promise that the following driver
metadata snapshot has already been projected. A later bounded journal must join
these causes to the coherent publication batch and retain the identities it
needs before retirement; it must not emit JS here.

An internal observer error is propagated with its FailureKind. Failure after
native commit cannot roll back the closed object. This is distinct from a JS
listener throwing: NODE-110's user-listener exception contract must not be
implemented by returning an observer error or rejecting the close Promise.
Admission failure or raw-future cancellation releases the guard. Abandoning a
started Continuable close transfers it to the already reserved cleanup owner,
without a second prepare call. The raw service alone is not advertised as
cancellation-safe.

No new dependency, Cargo feature, protocol tag or event capability is added.
The selected decorator holds an observer Arc; its caller owns that selection's
fixed storage. The reservation body and retained payload are implementation-owned
and require their own bounded admission. Tests use a separate ledger before the
box/vector allocations and verify return to zero. Test diagnostics that inspect
runtime owners are not a model for production metadata retention. The ordinary
async close still owns its existing future/transaction allocations; this
checkpoint does not claim zero process-memory overhead.

### Verification and remaining integration

Fourteen new deterministic tests passed: Page/Context success; both admission
rejections before CDP; native Context failure and unconfirmed Page absence;
external Context rejection; post-commit internal failure; unpolled laziness;
already closed idempotence; both started cleanup continuations; and Page/Context
network retirement before unlocked notification. The network tests use try_lock
on the exact driver state at completion, not only final lane counts.

An initial compile check exposed the existing network commit tuple shapes
(update plus cleanup messages/count), which are now unpacked while preserving
their previous cleanup treatment. The first test compile also used two wrong
enum forms (ResourceScope::Connection is unit-like; borrowed Context ownership
is External); these test-only names were corrected. No enum or ownership policy
was changed.

A new ignored real-Chrome test selects the observer around actual Page and
Context close services. It creates one owned Context with two Pages, expects
Page retirement while its sibling survives, then Context retirement of the
sibling, and checks two exact receipts and zero observer bytes. Cleanup addresses
only that native Context and preserves the external Chrome process.

The first default live run passed, but the first frame-lifecycle live run failed
because its fake ImmediatePageClosePoll made two immediate absence checks and
still saw the closing native target. The failure was preserved, and exact
Context cleanup ran before the assertion. The live fixture now injects the
actual Node host policy: CdpPageCloseLimits(128, 20) with 10 ms between checks,
as configured in host profile.rs and lifecycle.rs. No production close policy,
timeout or success condition changed. Unconfirmed absence still fails, as the
deterministic failure test verifies. Recovery with real asynchronous polling is
observed; the original browser completion timing is not independently traced.

Final composition release suites pass 367 default and 400 frame-lifecycle,
with 16 explicit live tests ignored in each ordinary run. The new selected
observer live test passes in both configurations, followed by three more
successful runs of each exact compiled test executable: four consecutive
final-policy observations per configuration. The repeats start no Cargo builds.
The initial immediate-poll result is not counted as final-policy verification.

Strict final composition/host/addon all-target Clippy passes with combined
drag-runtime, frame-navigation and extra-headers selection. Node units 47,
strict types, all 7 host / 8 addon dependency selections, formatting and diff
whitespace checks pass. The host unit suites were not rerun; the dependency
change is covered by native suites, host/addon checks and fresh Node artifacts.

### Rebuilt Node artifacts and differential evidence

Chrome 152.0.7977.83, pinned Playwright 1.58.2, and newly rebuilt release addons:

| Paired corpus | Default | drag-frame-extra-headers |
| --- | --- | --- |
| Collection publication | 7/7 | 7/7 |
| Collection creation visibility | 7/7 | 7/7 |
| Page.isClosed | 8/8 | 8/8 |
| Browser connection/disconnect | 5/5 | 5/5 |
| Lifecycle listeners | 6/11 | 6/11 |

The five failures are still missing Page/Context notices: mutation, async-gate,
close-reentry, popup-throw and async-reject. In close-reentry the sibling stays
open because its close is initiated by the missing callback; do not claim final
state parity for that case. All six supported Browser exception cases remain
green. This is a regression check of the ordinary Node services, which have not
selected the new observer, not proof of Node lifecycle callback implementation.
The selected native observer itself is covered by the separate real-Chrome test.

Default addon SHA-256:
`e93e688a6f2514bed14bb7ab97a46244f9d2abd321330b16eae0b2c8e4682861`;
combined SHA-256:
`a01fcccab5600a2937d513d8cc4e693d7b3cfae5573be5ebfcca5dce2c3fb2bc`.

All build/test children finished, including expected failing listener lanes.
The exact preflight anchor `AEBE0C36171F9B70D6B26C515CC9AA51` stayed alive
through native/Node runs, then was verified and closed with Pwright. Final
inventory was empty. No task-owned server route or background runtime remains;
shared Chrome/serve-lib and separately identified XLQY builds were not stopped.
Read-only shared-backend PID 53189 RSS: 756096 KiB at 07:03:53Z, 1580784 KiB at
07:14:18Z, 2993344 KiB at 07:19:10Z, 998416 KiB at 07:27:12Z, 2246288 KiB at
07:34:01Z and 1965648 KiB at 07:36:18Z; swap stayed zero. These shared-process
values are not exact per-chat RAM. Rust builds in this task stayed serial with
one Cargo job; live probes/Node comparisons did not launch another Rust build.

The Node host has not selected this decorator or a native cause journal yet.
Terminal Browser teardown remains a separate pre-admitted source. Native
ready/linkage and close receipts must next feed one bounded dependent-group
representation, intermediate visibility publication and unlocked foreign
delivery. The five missing Page/Context listener scenarios are not solved by
adding an input port.

The next producer must also avoid duplicate notice generation when an explicit
commit is followed by a native detach or terminal teardown. Use actual native
effects and single-consumption publication ownership, not one synthetic event
per API invocation. Retain admitted Page/Context metadata before retirement;
environment discard must release it without calling JS.

Evidence directory: `.work/node112-close-receipts/`.

## NODE-111: Borrowed committed driver effects

The selected ChromiumFrameUrlObserver now has a default reconcile_committed
method receiving ChromiumCommittedFrameOwnership: the exact effect slice of one
committed driver transaction plus its coherent post-commit ownership view. The
old observer implementation remains valid: the default delegates once to
reconcile_ownership. FrameTrees still uses that fallback; no host journal or
Node callback dispatcher is installed in this checkpoint.

The hook runs after runtime/topology commit, live-Frame pruning and that
transaction's URL/tree observations, while the topology mutex is held. It is a
native metadata projection, **not** a foreign callback boundary. It must not
reenter topology or invoke JS. An observer error propagates after commit and
does not roll back native state. Earlier projection failure can prevent this
hook; this is not a durable or replayable commit log.

Both the view and effects are borrowed. There is no added retained field,
queue, driver task, subscription, capability tag or dependency. Retaining a
future cause/group requires selected caller-owned capacity admitted before
native mutation; this observer must not allocate an unbudgeted event journal.
Any later JS delivery remains outside native locks and must use the exclusive
publication owner, fixed watermark and separately admitted foreign resources.

### Source coverage and group boundaries

| Source | Current committed observer coverage |
| --- | --- |
| Discovered Page/popup initialization | Driver transaction effects after successful initialization |
| Pre-registered Page initialization | Existing initialize_frame_tree marks Page ready during synchronized initialization |
| Deferred opener readiness | Original later PopupLinked effect, not a second popup PageReady |
| Exact Target.detachedFromTarget | Driver disposal transaction and post-disposal ownership |
| Snapshot refresh | Old ownership callback only; no replayed or synthesized effects |
| Explicit Page/Context close | Direct service commits bypass this hook; separate input integration required |
| Terminal Browser/transport teardown | Not established by this driver hook |

receive_once and initialize_snapshot can combine several transaction outputs.
This observer runs **before** that aggregation. The direct-transaction test
checks pointer identity with its returned effect slice; the aggregate may own a
different allocation. A native transaction is still not automatically one
Playwright dependent notification group. In particular, a ready popup whose
opener is not ready can link in the opener's later readiness transaction. Do
not assume every PopupLinked immediately follows PageReady for the same popup,
or emit Context page for that popup twice.

Source audit: ChromiumPageCloseCommandService::close and
ChromiumBrowserContextCloseCommandService::close discard the CdpTopologyUpdate
returned by their prepared topology commit, then commit runtime retirement.
initialize_snapshot later refreshes the final metadata even with no next event.
That preserves current getters, but cannot recover the lost cause ordering.
PreparedCdpPageClose and PreparedCdpBrowserContextClose already expose their
prepared update; the next integration must reserve from that native plan and
publish only after the owning transaction completes, not synthesize a notice
from the API method or final collection difference.

Terminal audit: SessionTeardown::drop retires the LiveSession and clears native
Frame/Context/URL indexes. FrameTrees::clear uses already reserved Page-close
permits through close_for_teardown before clearing its index. That path does
not call the new observer. Future terminal notices must preserve saturation-safe
retirement and retain only admitted metadata before those indexes disappear;
environment discard must still release storage without calling JS.

### Tests and verification scope

Eleven new composition tests cover exact popup ordering/membership, deferred
linkage, native detach disposal, snapshot non-replay, failed initialization,
post-commit observer failure, legacy fallback, idempotent readiness, borrowed
slice identity, pre-registered initialization and ignored target destruction.
The latter does not change driver behavior: Target.targetDestroyed alone is
not this driver's Page disposal input.

The initial targeted run had two incorrect test assumptions: equality of the
transaction slice address with the outer aggregated vector, and using target
destruction rather than exact session detach. Both were corrected with explicit
regression cases; the original failing log is retained. No production behavior
was changed to satisfy those assumptions.

Composition release tests: default 353 passed / 15 intentionally ignored;
frame-lifecycle 386 passed / 15 ignored. Strict combined composition/host/addon
all-target Clippy, 47 Node units, types, all 7 host / 8 addon dependency
selections, formatting and whitespace checks pass. The final Clippy rerun also
covers the documentation/comment and test-readability edits after the initial
run. No new host unit-suite run is claimed; both addon profiles were freshly
rebuilt against the changed composition boundary.

### Fresh browser evidence and setup failure

The first five lanes in each profile failed during pinned upstream Playwright
1.58.2 attachment: Browser.setDownloadBehavior returned "Browser context
management is not supported." No paired behavior comparison was reached; these
are setup failures, not missing-event or candidate-only regression evidence.
At that point the dedicated Chrome process had no tabs. Opening a task-owned
about:blank through Pwright in that same process restored the Browser lane to
5/5. The remaining lanes were then rerun while retaining that tab. No shared
Chrome restart or production/test semantic change was used. This establishes
the observed recovery, not Chromium's underlying cause or a universal zero-tab
rule. The initial logs remain intact.

Both new release artifacts, Chrome 152.0.7977.83 and pinned Playwright 1.58.2:

| Paired corpus | Default | drag-frame-extra-headers |
| --- | --- | --- |
| Collection publication | 7/7 | 7/7 |
| Collection creation visibility | 7/7 | 7/7 |
| Page.isClosed | 8/8 | 8/8 |
| Browser connection/disconnect | 5/5 | 5/5 |
| Lifecycle listeners | 6/11 | 6/11 |

The five listener failures remain mutation, async-gate, close-reentry,
popup-throw and async-reject: missing Page/Context notices. The reentrant case
also leaves the sibling open because its close was initiated only by the
missing first close callback; do not claim all final states match. All six
Browser exception cases pass, including object/primitive identity, error-handler
reentry, async rejection and expected unhandled child exit. These are bounded
corpus results, not full lifecycle compatibility. Reopened logs have the
*-reopened.log suffix (default Browser uses default-browser-reopened.log).

Default addon SHA-256:
`8ba20370498dec789bbbea55867496b1181a96e4417b6b64528137d45342ca87`;
combined SHA-256:
`84d2c2e2b1fc6ab05511cb65c63cf026f9afda8d99eca74356728dd30383136c`.

All task build/test children finished. The original preflight target
`1D66C1F0A12D1FF8477CDAD417E3F50E` and later anchor
`41952F0CB0F88972912DC45B9DF97D9E` were individually verified and closed with
Pwright; final inventory was empty. No task-owned HTTP route or background
runtime remains. Shared Chrome/serve-lib and unrelated XLQY work were untouched.
Subsequent browser runs must revalidate attachment, not assume an empty browser
still supports the upstream Context-management setup.

Read-only shared-backend PID 53189 RSS: 3002976 KiB at 06:52:48Z,
1144080 KiB at 06:57:43Z, 2347552 KiB at 07:02:38Z; swap stayed zero.
This is shared process memory, not exact per-chat RAM. The prior targeted rustc
peak check reached 8.1 GiB physical footprint (vmmap warned detailed allocator
inspection was incomplete). Builds in this task were serial with one Cargo job;
separately identified XLQY Cargo work was not owned or stopped by this task.

Evidence directory: `.work/node111-committed-effects/`. Missing Page/Context
callbacks, native dependent groups, explicit-close/terminal cause inputs,
per-group failure receipts and foreign callback delivery remain incomplete.

## NODE-110: Listener failure boundaries and scoped exception reporting

The new `test:lifecycle-listeners` lane characterizes actual browser events in
isolated Node children, not manual EventEmitter.emit calls. Pinned Playwright
1.58.2 passes all eleven reference cases. Its installed client EventEmitter
copies the current listener list; BrowserContext._onPage adds membership, emits
Context page, then emits opener popup in the same dispatch call. The in-process
transport invokes client dispatch from setImmediate. Local source evidence:
`tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/`
`client/{eventEmitter,browserContext,page}.js` and `inProcessFactory.js`.

Observed contracts:

- Removing a later listener during delivery does not remove it from the current
  copied listener list; an added listener starts on the next event. Once runs
  once. Original Page arguments and callback receivers are preserved.
- An async listener blocked on an explicit Promise gate does not block the next
  listener or newPage completion. The gate is released after command completion.
- A first Page close listener can initiate sibling.close(). The first callback
  and synchronous nested call return see exactly one closed/removed Page; the
  second close callback sees two. Both commands complete; the sibling close
  callback does not synchronously interleave into the first callback.
- A once Context page listener throwing synchronously on popup discovery stops
  both the rest of that emit and the dependent opener popup notification.
  Already published membership remains. In the diagnostic child, which captures
  the expected uncaught exception, a later independent newPage still delivers
  Context page and resolves normally. This does not claim an unhandled process
  survives: it has an explicit error observer only for characterization.
- An async rejecting listener does not stop the next listener or command. Its
  original rejection appears through unhandledRejection, not uncaughtException.
- Browser disconnected synchronous errors are uncaughtException receipts, not
  close()/watch Promise rejections. Error, frozen object and number identities
  are preserved. An error handler reentering browser.close() completes without
  duplicate disconnected delivery. Async rejection keeps its separate channel.
- Without a process error handler, the isolated disconnected-throw child exits
  with status 1 and reports the original error rather than a success document.

### Implemented fix, not the missing lifecycle dispatcher

The initial default candidate run passed 0/6. Five failures were missing
Page/Context events; the sixth found a separate real bug in the already supported
Browser disconnected event: a synchronous listener exception escaped watchSession's
Promise continuation as unhandledRejection. The close Promise itself resolved,
and final metadata matched; the exception channel was still incompatible.
The pre-fix evidence remains in `.work/node110-listener-semantics/default.log`.

`callback_exception.rs` now exposes a scoped NativeSession.reportCallbackException
language port. The JS disconnected adapter catches only synchronous emit failures
and forwards the original value. Rust invokes
[Node-API napi_fatal_exception](https://nodejs.org/api/n-api.html#napi_fatal_exception)
with that borrowed handle, unconditionally in release builds. It does not turn
the value into a new Error, stringify it, schedule another task or reject an API
Promise. Async listener promises are not awaited or rerouted by this catch.
Older addons without the new method retain their old exception behavior and
must be rebuilt for this fix; no compatibility claim is made for those artifacts.

No host/publication state, native lifecycle decision or browser command moved
into JS. Rust still commits/suppresses the one-shot disconnected state before
language dispatch. The new port holds no host mutex, resource guard, persistent
Node reference, callback registry or queue; it borrows one live Node value only
for the synchronous call. Node's own exception machinery may allocate, but no
new Rust retained storage or logical budget is introduced. Calling user process
error handlers is outside native locks. Status failure has a fixed diagnostic.

The child has its own unref'ed 14-second hard limit, the parent enforces 15
seconds per child and 64 KiB each of stdout/stderr, and each paired case has a
35-second limit. The existing outer lane keeps 180 seconds, 4 MiB stdout and
1 MiB stderr bounds. Reports cap 32 trace observations and four expected error
receipts. Only the deliberately thrown original value is intercepted; unrelated
errors remain fatal. Handlers/listeners are removed before normal cleanup, and
candidate native/FFI resource counters must reach zero. The no-handler case
intentionally exits and does not claim execution of its JS finally block.

### Consequences for the next Rust publication layer

Native causes need **dependent notification groups**, not independent emit calls
after every queue item. Commit the Page membership/state before its Context page
notification, but suppress the dependent popup when that synchronous dispatch
throws; do not roll back membership or retry the consumed event. Later independent
groups remain available. The Rust owner must release/advance authority and charge
or retire every skipped notice explicitly, without discarding unrelated work.
Async listener promises are language-owned activity and must not hold the native
drain until they settle. Conversion/admission errors are a separate failure class;
do not automatically treat them as user listener exceptions.

This checkpoint does not implement Page/Context events, native cause groups or
their FFI cursor. Further work includes intermediate visibility transactions,
group failure receipts and bounded notice/callback admission. removeAllListeners
async behavior, async-hooks timing and failures thrown by process error handlers
themselves are not characterized by this lane.

### Verification checkpoint

Evidence is in `.work/node110-listener-semantics/`. Standalone final reference
passes 11/11 (`reference-final.log`). Both rebuilt profiles pass all six Browser
exception scenarios and fail the five missing Page/Context event cases:

| Profile | Full listener lane | Browser exception cases | Normal Browser connection regression |
| --- | --- | --- | --- |
| default | 6/11 | 6/6 | 5/5 |
| drag-frame-extra-headers | 6/11 | 6/6 | 5/5 |

Final logs are `default-complete.log`, `combined-complete.log`,
`browser-default.log` and `browser-combined.log`. Every handled candidate case
completed its native/FFI zero-resource cleanup assertions; the deliberately
unhandled case instead proved exit status 1. Missing close callbacks have a real
behavioral effect: the sibling close initiated by a listener never runs, leaving
one Page open in the candidate observation. Do not classify all five failures
as harmless trace-only discrepancies or claim their final states all match.

Default addon SHA-256:
`d228ce530117d642d86ed83ba48117c89204d0b407c93f61ebf91e323cd4a869`;
combined SHA-256:
`e072bbac9cbd2cc7fed484f19a7480881a71ebc30c6f7b193a1e44fa34cae30b`.
Both are release builds; the core host was not modified in NODE-110. Strict
combined host/addon all-target release Clippy, 47 Node unit tests, types, all 7
host / 8 addon dependency selections, JS syntax, Rust formatting and whitespace
checks pass. No fresh host unit-suite rerun is claimed for this addon-only fix.

All build/test children ended, including the expected nonzero exits. The exact
task preflight target `1DD6D558493192ECECC123610ACC5485` was verified and closed
through Pwright; the final inventory was empty. A setup preflight temporarily
created from the prior task directory (`7B5C9741B3A1955B9BA60839E4182A80`) had
already been explicitly closed before opening the isolated NODE-110 directory.
No file-server route, Node listener or task-owned background runtime remained.
Shared Chrome/serve-lib were not restarted or stopped; unrelated work was not
closed to obtain the result.

Read-only shared-backend PID 53189 snapshots: RSS 3743248 KiB at 06:13:20Z,
3112960 KiB at 06:19:21Z, 1875200 KiB at 06:23:29Z, 2889696 KiB at 06:31:21Z
and 1763568 KiB at 06:33:00Z; swap remained zero. The earlier observed XLQY
cargo processes were not launched or stopped by this task. These shared-process
figures do not establish per-chat RAM or a causal effect of this small port.

The full compatibility goal remains active. No new staging, commit or push.

## NODE-109: Exclusive fixed-watermark publication owner

The Rust host now exposes `FramePublicationFence::try_begin(&self)` and a
non-cloneable `FramePublicationDrain::publish_next(&mut self)`. Acquisition
requires the already pre-admitted publication queue and only one active owner.
Busy acquisition leaves the original borrowed fence available for exact-watermark
retry. The owner sequence uses checked monotonic nonzero IDs, never wraparound.
Ordinary whole-fence `publish()` refuses to bypass an active exclusive owner.

`Published` installs exactly one existing internal metadata change; `Complete`
means the captured watermark is exhausted. Changes staged later remain pending,
and delayed old fences cannot regress or replay a consumed change. For MAX
terminal fences, only Complete clears collections and sets DisconnectReady;
the existing separate publish_disconnect transition owns the public connected
flag. Complete releases the endpoint Arc even if the finished guard is retained.
Drop releases only the claim, without publishing or discarding remaining work.
Environment discard revokes authority and abandons pending changes. A metadata
application error discards the endpoint, preventing a retry after partial failure;
mutex poisoning is fatal and requires teardown. No method invokes foreign code,
and every method returns with native locks released.

This is a queue-ownership primitive, **not an event dispatcher**. In particular,
Page Close and Collection can be separate queue entries, so Published is not an
atomic Playwright callback-state transaction. No queue-difference event synthesis,
callback, cursor, waiter queue, native cause journal or Node event subscription
was added. Node still uses full-fence publication. Before FFI integration, native
cause transactions must compose collection visibility/rebasing and metadata,
then resolve existing wrapper identities and dispatch outside all native locks.
Busy foreign completions must retain their original fences rather than silently
replace them with newer watermarks. Listener throw/reentry, terminal notice
capacity and foreign wrapper/port admission remain open design and test work.

The selected queue allowance now additionally covers size_of(DrainOwnership)
and one size_of(FramePublicationDrain), admitted before allocating the queue.
The aarch64 release layout measures these at 16 and 24 bytes respectively:
fixed storage is now 31544 bytes (+40), with one fixed state. Collection revision
storage remains 1256 bytes and one revision item. These are logical ledger
allowances, not process resident memory or allocator-size measurements.
Claims/advances allocate no new queue, container or lease. The guard holds only
the metadata endpoint and its existing fixed storage, never Host/runtime/CDP;
retained collection revisions remain independent. Finished guard bodies or
foreign containers need caller-owned storage admission, not unbounded runtime
retention under the single active-guard allowance.

Sixteen added Rust tests cover selection, exclusive claim, whole-fence bypass,
one-step visibility/unlocked reads, fixed/old watermarks, partial-drop recovery,
finished-owner rejection, discard, nonce exhaustion, terminal close/readiness,
held collection batches, poisoned-target failure, unchanged queue capacity and
final metadata/storage release. Default host release tests pass 320 with 7
ignored; combined profile passes 368 with 9 ignored. Node unit tests pass 47 and
strict types pass. Evidence: `.work/node109-publication-owner/`.
Strict release host/addon all-target Clippy passes with the combined features;
all 7 host / 8 addon dependency selections, formatting and whitespace checks pass.
Both Node profiles were rebuilt from this checkpoint. Default SHA-256:
`f6289d7168c78912731817faf108a2a7113b43525a624563d1d7ca921cdb9d15`;
combined SHA-256:
`92dd494b5e096ef6437f9432378dca5e651819e708b1833ce532b7eaef5eb63c`.
The shared dedicated Chrome remained running with its existing profile and
five-second connection budget. Paired upstream Playwright 1.58.2 results:

| Rebuilt profile | Collection publication | Page closed | Browser connection | Creation visibility | Lifecycle events |
| --- | --- | --- | --- | --- | --- |
| default | 7/7 | 8/8 | 5/5 | 7/7 | 0/7 |
| drag-frame-extra-headers | 7/7 | 8/8 | 5/5 | 7/7 | 0/7 |

Both event lanes have seven complete reference/candidate recordings, all seven
final states matching and no initialization timeout. Failures remain missing
Context page / popup / Page close / Context close notifications; candidate
Browser disconnected delivery still exists. The rebuilt regressions exercise
the existing whole-fence path, not the unconnected exclusive API. These results
do not prove full lifecycle compatibility or fix the earlier attachment stalls.

All build/test jobs ended. The exact preflight about:blank target
`EE9EEF91410DECFC67FBB4BD146AF7DE` was verified and closed through Pwright;
the remaining unrelated XLQY target was untouched. No HTTP fixture mount was
created and no Node relay listener remained. Shared Chrome and serve-lib were
not restarted. A separately observed cargo build was attributed by cwd to
`scratch/xlqy-hotkeys-chat/xlqy-game/server`, not this task.

Read-only memory snapshots identified shared backend PID 53189: RSS 1132976 KiB
at 05:48:02Z, 5786048 KiB at 06:03:22Z, then 1097856 KiB at 06:06:34Z without
cleanup. vmmap measured physical footprint 6.1 GiB at 06:03:30Z; its reported
lifetime peak is not a per-task measurement. At 06:11:19Z RSS was 1758736 KiB;
swap remained zero. Shared-process memory cannot be attributed to this chat.

Next: design/admit native cause transactions and characterize listener reentry
and thrown callbacks before integrating this owner across every FFI fence.
The overall Node compatibility goal remains active; no new commit or push.

### Earlier checkpoints

NODE-108 adds native TRACE diagnostics, not Node EventEmitter notifications.
Exact session-retirement handling already exists and passes controlled paired
cases (6/6 default, 10/10 combined). Fresh native layouts pass without reproducing
the earlier timeout; bounded identity receipts now support more precise analysis.
The admitted publication step owner/journal and foreign dispatch remain open.
[NODE-108 receipts and verification](pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

## NODE-107: Validated cross-revision visibility rebasing

`CollectionView::try_rebase(&CollectionRevision)` now moves a view to a separately
admitted revision while preserving exactly its visible membership and order.
Additions remain hidden. Missing visible members cause a typed error: retire them
on the old view first, then rebase, then explicitly reveal additions in the
native cause order. This is not a collection-difference event generator or an
assumption that native creation/closure notices may be reordered.

The input view and revision remain unchanged on success and failure. Validation
is bounded by the two existing 16-member slices (quadratic scans, no temporary
union, heap allocation or additional resource admission). A complete replacement
of 16 old Pages by 16 new Pages needs only their separately admitted revisions;
the transition never constructs a 32-member revision. Old reader-held descriptors
remain charged and immutable in membership. This test models metadata overlap,
not authorization for 32 simultaneously active native Pages.

Each revision now retains a clone of its publication domain's existing metadata
ResourceLedger. Pointer identity rejects foreign domains even for empty/hidden
views. It allocates no new domain object and retains neither the publication
queue nor Host/runtime/transport. A Weak queue-liveness test verifies that views
can outlive the queue and still rebase/release metadata. Shared ledger identity
does not prove chronology or grant a one-shot publication authorization.

Overlapping descriptors, including hidden ones, cannot change object kind,
Page parent Context, main Frame identity or Context metadata identity. Configured
Context settings require the same settings Arc, not merely equal baseURL text;
unconfigured borrowed settings require the same metadata ledger. Numeric Context
identity remains the native owner's responsibility, particularly for unconfigured
settings that intentionally have no per-Context allocation. Missing members and
visible-order changes are also typed errors. No native closed flag is changed.

Twenty new Rust tests cover domain rejection, unchanged membership/order and
budgets, identity/kind/parent replacement, hidden additions, independent equal
settings, configured/unconfigured transitions, full-capacity replacement and
final lease release after queue destruction. Default host release tests pass
304 with 7 ignored; combined passes 352 with 9 ignored. Strict host/addon Clippy,
47 Node unit tests, strict types, all 7 host / 8 addon dependency selections,
formatting and whitespace checks pass. Evidence is under
`.work/node107-collection-rebase/`.

Release layout checks measure revision storage at 1256 bytes (+8 from NODE-106)
and the selected 32-retirement pool plus vector at 42496 bytes (+256), still
33 revision-ledger items. Fixed publication storage stays 31504 bytes. The new
domain handle is covered by size_of(Projection) in admission; no uncharged new
Arc allocation is introduced. These are logical budgets, not process RAM.

Both profiles were rebuilt. Default addon SHA-256 is
`bda2e892c3a05131e232e6f15da54e9b67be71f43a19c50ddb379caba77b878b`;
combined is `4332d193c9578b0fdb6dd6dbddacb09b13f851392dc1616e5d5cf01aa5edfb0f`.
The initial default collection-creation run passed 6/7: candidate connectOverCDP
timed out after 5000 ms during topology initialization in the borrowed-discovery
case, before the candidate was available or any rebasing could run. The unchanged
full repeat passed 7/7. Preserve both logs; this is a different observation from
NODE-106's post-creation native activeFrames readiness timeout, not a root-cause
diagnosis or proof of a fix. The five-second budget and shared Chrome remain
unchanged. A fresh default Rust attachment command-trace probe was built, and
its 12 recorder unit tests passed (1 live diagnostic ignored by default).

### Attachment diagnostics, not a timeout fix

The first combined Browser connection lane passed 4/5; its fifth case failed in
the upstream reference's connectOverCDP through the diagnostic relay, also at
5000 ms. The candidate for that case had not run. Preserve
`drag-frame-extra-headers-browser-connection.log` separately from later results.
These observations do not establish either a new Rust regression or a shared
browser scheduling cause; no retry was added to the production API or test cases.

After the test runner exited, `probe-attachment-commands.mjs` ran against the
fresh default `attach_probe-4831ca089a83d681` binary (SHA-256
`55c8d96bdb394f85eefc05a406d115dd223cbf5671a46085888403277b5dcedb`).
All six observations connected within the original five-second deadline, each
with 38 commands / 38 responses, zero pending commands and no recorder overflow:

| Observer layout | Round 1 attachment ms | Round 2 attachment ms |
| --- | ---: | ---: |
| Rust host alone | 52 | 280 |
| Upstream observer | 2138 | 2215 |
| Upstream observer plus browser CDP session | 1879 | 2073 |

Retained response maxima were 21/234 ms alone and 340–392 ms with the observers.
This experiment observes slower attachment in these observer layouts but does
not reproduce the original timeout or identify its cause. It exercises the real
default Rust HostSession, not the Node FFI bridge, and is not an additional parity
pass. The bounded JSON and log are `attach-commands.json` / `attach-commands.log`;
the probe and all its controller connections ended before regressions resumed.

### Combined-profile reproduction and incomplete live gate

The fresh combined probe `attach_probe-cc7e7cd1ff1e3669` (SHA-256
`2189c545d19c898e05c8edae9e14809b23e3781742dc97781c1bdbd56a9fa7c2`)
then reproduced four topology-initialization timeouts without the Node addon.
Its bounded recorder had no overflow. `attach-commands-combined.json` retains
the complete six observations:

| Layout / round | Outcome | Command at the deadline |
| --- | --- | --- |
| Alone / 1 | Connected, 2039 ms; 50/50 responses | None |
| Observer / 1 | Timeout, 5003 ms; 16/17 responses | Page.enable, unanswered 4994 ms |
| Observer plus browser session / 1 | Timeout, 5003 ms; 16/17 responses | Page.enable, unanswered 4993 ms |
| Alone / 2 | Connected, 3483 ms; 50/50 responses | None |
| Observer / 2 | Timeout, 5003 ms; 17/18 responses | Network.enable, unanswered 701 ms; earlier Page.enable took 4294 ms |
| Observer plus browser session / 2 | Timeout, 5003 ms; 26/27 responses | Page.enable, unanswered 843 ms after earlier initialization consumed the deadline |

The first three timed-out targets map to `529C7FCFACF62452D1B2BF31CF6FA297`,
previously an unrelated XLQY Page; the fourth maps to
`5A6DD1B90BE624C8D945A3C9EAF16575`, also unrelated XLQY. The later Pwright
inventory no longer contains the first target and includes a newly created
unrelated Page. No action here closed or modified either target. This is evidence
that the browser population changed, not proof that target retirement caused
the timeout or that the observer layout alone caused it. `initialize_snapshot_inner`
awaits session synchronization/event application in sequence; a targeted test of
retirement while a command is pending is the next diagnostic, not a justified fix
from these observations alone. Do not extend public deadlines or restart shared
Chrome to turn the gate green.

The actual NODE-107 live matrix is **partial**, not all-green:

| Lane | Default | Combined |
| --- | --- | --- |
| Collection publication | 7/7 | 7/7 |
| Creation/discovery | Initial 6/7, repeat 7/7 | 7/7 |
| Empty-Context lifecycle | 1/1 | 1/1 |
| Page isClosed | 8/8 | 8/8 |
| Browser connection | 5/5 | 4/5 twice: first reference connect timeout, then candidate connect timeout |
| Page enumeration | 7/7 | Candidate initialization timeout; no completed comparison |
| Context baseURL | 16/16 plus 4 boundaries | Candidate initialization timeout; JSON has error and zero paired cases |
| Context input boundaries | 2 passed, 3 feature-skipped | 5/5 |
| Worker destruction subset | 3/3 | 3/3 |
| Lifecycle event contract | 0/7, all seven final states match but notices missing | 0/7: four missing-notice failures, three candidate initialization timeouts |

The final standalone reference lifecycle attempt also fails all seven during
connectOverCDP (about 35 seconds total). It establishes no new event semantics
and does not invalidate the earlier completed paired reference traces. Keep
`reference-lifecycle.log`; do not relabel those setup failures as expected missing
events. Remaining combined evidence uses `combined-final-*.log`, with the earlier
failures preserved separately. Neither repeated connection attempts nor the
default-profile trace establish combined-profile stability or a completed goal.

### Cleanup and next work

The exact NODE-107 preflight target `05B2DFE1A5985EC77777F938EA419ACF` was
verified as about:blank and closed. Both serve-lib routes `/pwright-node107` and
`/pwright-node107-context` were deregistered and absent; all 23 recorded diagnostic
relay ports have no listener. All build/test/probe processes ended. No task-owned
runtime remains; shared Chrome, serve-lib and other tasks were not interrupted.
HEAD remains f76e3f2, staging empty; no new commit or push.

Shared codex PID 53189 RSS: 3617264 KiB at 05:07:59 UTC, 990928 at 05:15,
8202672 at 05:22:31, and 1810528 at 05:29:31. Targeted vmmap near the spike
reported physical footprint 7.2 GiB; later RSS fell without intervention. System
swap stayed zero; memory_pressure went from 62% free near the spike to 71% at
handoff. A separate large cargo process was verified by cwd as XLQY work and
left untouched. These are shared-process observations, not this chat's exact RAM
or evidence that revision storage caused the spike.

Before claiming the live gate recovered, characterize pending initialization
commands during target/session retirement and concurrent observers with bounded
receipts, then validate the same rebuilt profiles. Event work still requires
the admitted fixed-watermark step owner, typed native cause journal, foreign
dispatch, and listener-failure/reentry rules. The rebase primitive alone does not
implement these lifecycle semantics.

The selected Node publisher still installs full views at a whole fence. This
rebasing primitive is not yet used to dispatch callbacks. Next implement the
admitted, one-shot, fixed-watermark step owner and native cause journal; validate
listener failure/reentry before integrating all foreign publication paths. The
NODE-106 intermittent discovery timeout also remains an unresolved diagnostic.

## NODE-106: Immutable single-revision visibility foundation

Implemented in `pwright-node-host::frame_metadata::collections::visibility`:
`CollectionView` retains an already-admitted immutable `CollectionRevision` and
two private 16-bit membership masks. `full` and `hidden` select initial visibility;
`with_visibility` returns a new value without modifying older retained views.
Typed errors reject unknown members, duplicate transitions, hiding a Context with
visible Pages, and revealing a Page before its parent Context. Hiding a Page does
not set its closed flag or change native ownership. Descriptor identity and order
remain those of the retained revision; hidden descriptors remain charged until
the final revision owner drops.

`published_collections()` now returns this view. Existing whole-fence publication
installs a full view, and selected Node collection readers consume borrowed
`CollectionMembers` (`len`, `is_empty`, `iter`, `get`, indexing). Iteration is
ordered, double-ended, exact-sized and fused. Native HostSession collection APIs
remain separate. No JS lifecycle state, dependency, feature, subscriber, transport
or foreign callback was added. This is preparation for event pacing, not an event
dispatcher: no partial view is installed between callbacks yet.

### Measured representation and admission

On this Mac's release build, view body is 16 bytes; borrowed member and iterator
bodies are each 24 bytes, with no heap allocation. A visibility transition clones
only the retained Arc and value masks, requiring no new logical admission. The
revision allowance conservatively includes one view body. Additional captured
cursor or foreign delivery bodies will still need explicit admission.

| Logical storage | NODE-103 bytes | NODE-106 bytes |
| --- | ---: | ---: |
| Collection revision reservation | 1232 | 1248 |
| Fixed publication storage | 31496 | 31504 |
| Selected 32-retirement pool plus vector storage | 41728 | 42240 |

The pool still accounts for 33 revision-ledger items, not a new event capacity.
These are Rust layout/admission measurements, not RSS or browser memory savings.
Evidence: `.work/node106-collection-visibility/{layout,pool-layout,view-layout}.log`.

Fourteen new tests cover immutable old membership and descriptor identity,
parent/child ordering, duplicate/unknown transitions, all 16 Page and Context bits,
empty revisions, exact/double-ended iteration and final hidden-view lease release.
Default host release tests pass 284 with 7 ignored; combined
`drag-runtime,frame-navigation,extra-headers` passes 332 with 9 ignored. Strict
host/addon Clippy passes; Node unit tests pass 47, strict types and all 7 host / 8
addon dependency selections pass. Both addon profiles were rebuilt successfully.

### Rebuilt-artifact live verification

Dedicated external Chrome 152.0.7977.83 over loopback CDP, pinned Playwright
1.58.2; no shared browser restart. Evidence remains under
`.work/node106-collection-visibility/`. Release artifact SHA-256:

- Default (`packages/pwright/native/pwright.node`):
  `cc0bdbf9a12ed0857fbb554d4863061da3cc224307afb1f5abcfc7b81fe5303f`.
- Combined (`native/drag-frame-extra-headers/pwright.node`):
  `711e35cc80318c92c8bd0ecf94eaa698f8cf18fbd9b5d15db9e1a0df41ede856`.

| Lane | Default | Combined |
| --- | --- | --- |
| Collection publication | 7/7 | 7/7 |
| Creation/discovery | 7/7 | Initial 6/7; repeat 7/7; targeted discovery 1/1 |
| Empty-Context lifecycle | 1/1 | 1/1 |
| Page isClosed | 8/8 | 8/8 |
| Browser connection | 5/5 | 5/5 |
| Page enumeration | 7/7 | 7/7 |
| Context baseURL | 16/16 plus 4 boundaries | 16/16 plus 4 boundaries |
| Context input boundaries | 2 passed, 3 feature-skipped | 5/5 |
| Worker destruction subset | 3/3 | 3/3 |
| Lifecycle event contract | RED 0/7 | RED 0/7 |

The initial combined creation run failed only Worker-confirmed borrowed discovery:
Target.createTarget was acknowledged, but candidate native activeFrames did not
exceed its sampled baseline within five seconds. FFI counters were all zero at
failure. The unchanged full repeat and targeted case passed. Preserve
`drag-frame-extra-headers-collection-creation.log`,
`combined-collection-creation-repeat.log` and `combined-discovery-confirm.log`:
the intermittent readiness timeout is not diagnosed or fixed. Aggregate Frame
count alone does not establish a target-specific cause; neither scheduling nor
shared browser activity is proven responsible. No timeout/assertion was weakened.

The reference-only lifecycle contract passes 7/7 in 5.509 seconds
(`reference-lifecycle.log`). Both candidates fail all seven on missing notices;
all final states still match the reference, with 14 missing notices per profile.
`*-lifecycle-red.log` preserves failures rather than converting them into green
expectations. Both artifact-specific Page/Context JSON reports record their new
hashes. After Page wrapper reclamation, host ownership counters and URL metadata
are zero; retained Browser metadata still owns fixed tree storage (1 state,
0 revisions, 31504 bytes). This is not a claim of zero process RAM.

The NODE-106 preflight target `5F44017B95B5A888B9E5CF1CD42B5FEB` was verified
as about:blank and closed. Both temporary serve-lib routes `/pwright-node106`
and `/pwright-node106-context` on port 9384 were deregistered and absent from
the mount list. All 20 recorded diagnostic relay ports have no listener; build
and test processes exited. No task-owned background runtime remains. Shared
Chrome, serve-lib and other tasks were preserved.

Read-only memory snapshots: shared codex PID 53189 RSS was 2277744 KiB at
05:00:09 UTC, 2799216 KiB at 05:05:58, then 2089424 KiB at 05:06:45.
A targeted vmmap near handoff reported physical footprint 1.8 GiB; system swap
remained zero and memory_pressure reported 72% free. These differently timed
process/OS measures are not interchangeable or attributable to this chat alone.
HEAD remains f76e3f2, staging empty; no commit or push in NODE-106.

### Remaining event integration

This view represents one revision only. It does not solve old/new/pending identity
overlap, cross-revision rebasing, a one-shot admitted step cursor, queued typed
notices, wrapper retention through dispatch, listener failure/reentry, or routing
every NAPI publication fence consistently. The proposed boundary below remains
open except for this measured immutable-view building block. Collection getter
regressions and the intentionally RED lifecycle-event gate are recorded above
separately; getter parity is not event parity. Carry forward the intermittent
borrowed-discovery readiness timeout as an unresolved diagnostic, not a fixed bug.

## NODE-105 status: characterized, candidate RED

The Node facade currently inherits EventEmitter but emits only Browser
`disconnected`. Context `page`, Page/Context `close` and opener `popup` listeners
can be registered yet receive no notification. NODE-103/104 collection/scalar
success does not establish this event contract. This checkpoint adds actual
reference/candidate evidence and the next implementation design; no production
Rust/JS behavior, native binary, dependency or feature selection changes here.

## Observed pinned reference contract

Source: the installed `playwright-core@1.58.2/lib/client/browserContext.js`
(`_onPage`, `_onClose`, `pages`), `page.js` (`_onClose`) and `browser.js`
(`_didClose`), verified against real dedicated Chrome 152.0.7977.83. The separate
upstream checkout is not the pinned oracle. `packages/pwright/test/lifecycle-events.test.mjs`
records event arguments, callback-visible state and Promise resolution boundaries.

| Operation | Event/settlement order | State inside callbacks |
| --- | --- | --- |
| Explicit Context.newPage | Context page, then resolution | New Page already listed; exact returned/cached Page and main Frame |
| Page.close with a sibling | Page close, then resolution | Argument is closing Page; isClosed true; Page absent; sibling and Context remain |
| Context.close with two Pages | Page close, Page close, Context close, resolution | Page counts 1 then 0; closed counts 1 then 2; Context removed only before its own callback |
| Owned Browser.close | Both Page closes, Context close, disconnected, resolution | Page/Context callbacks still see connected true; disconnected callback sees false |
| Borrowed Browser disconnect | Both tracked Page closes, Context close, disconnected, resolution | Same connection-local sequence; both controller-owned native Pages survive |
| Worker-confirmed external Page closure | Page close | No event while Node is blocked despite confirmed native closure; sibling remains usable |
| Popup via window.open | Context page, then opener popup | Both receive the same Page, already in the Context collection |

Sibling Page identity order is diagnostic, not a cross-run guarantee. The trace
compares counts and exact per-handler argument identity without asserting which
independent sibling must close first. Retained arrays are actual earlier
Context.pages() results and must not be mutated. Repeating owned close emits no
additional event. Popup ordering relative to evaluate() completion is deliberately
not inferred: the test checks Context-page before popup and the observed identities.

The external-close Worker is reused from NODE-098: it resolves an exact marker,
closes that native Page and acknowledges through SharedArrayBuffer independently
of the blocked subject. Candidate native Frame retirement is observed before
sampling zero callbacks/unchanged public state. Public isClosed then provides a
bounded publication observation; this is not a fabricated callback or CDP event.

## Reproducible evidence

Run `PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:lifecycle-events` after the
dedicated external Chrome/Pwright preflight. No HTTP fixture is required.
`PWRIGHT_LIFECYCLE_REFERENCE=1` explicitly selects reference-only characterization.
Cases have 30-second deadlines; the existing lane supervisor caps the process at
180 seconds, stdout at 4 MiB and stderr at 1 MiB. Each trace is capped at 32 entries.

Evidence directory: `.work/node105-lifecycle-events/`.

- Reference: 7/7 initially and on repeat; final retained-array version 7/7 in
  5.393 seconds (`reference-final.log`).
- Default: 0/7 twice; final 11.946 seconds (`default-final-red.log`).
- Combined: 0/7 twice; final 12.288 seconds (`combined-final-red.log`).
- Every final collection/closed state agrees. Candidate lacks all new event
  kinds; Browser disconnected remains present and correctly precedes resolution.
  Creation/popup `samePage` is false because no event argument was received,
  not because the Page's native/main-Frame identity is incorrect.
- Unchanged default SHA-256:
  `cf8c6555ca051d587831181225fe7f09a23d48f14f9f763103ff72896682c4e5`.
- Unchanged combined SHA-256:
  `2896109c45a7a2fd7af25a79671bcbe62f1bcda7e595df0c19999d5984b891ba`.

The final test also asserts the reference's complete per-callback state sequence,
not only event names. This strengthened `reference-contract.log` passes 7/7 in
6.701 seconds; `default-contract-red.log` and `combined-contract-red.log` each
pass 0/7 in 14.228 and 14.060 seconds. Parsing both final receipts confirms all
seven final states (and the blocked external-close snapshot) match, with exactly
14 missing notifications per profile across the seven cases. No native panic or
environment cleanup failure is present. These failures are the expected red
baseline, not a claim of implementation success.

Node unit tests pass 47/47; strict types, seven host/eight addon resolved dependency
selections, syntax and whitespace checks pass. No Rust test/build is claimed in
this test/design-only checkpoint. All test processes/Workers exited. The final
browser inventory has no node105 marker Page; exact about:blank preflight target
`67165A4F5CE872885810EE019A5525E4` was closed. No HTTP route, relay or background
server was created, and no task-owned runtime remains. Shared Chrome/XLQY work
was preserved. Working tree remains unstaged/uncommitted beyond the foundation.

Read-only memory snapshots identified shared backend PID 53189 dynamically:
966,832 KiB RSS at 04:12:00 UTC; 1,827,008 KiB at the 04:30:42 continuation;
2,277,888 KiB at 04:42:16. vmmap at 04:42:40 reported 2.8 GiB physical footprint;
swap remained zero and memory_pressure reported 70% free. Larger Node/cargo jobs
were attributed by cwd to XLQY deployment/server work and left running. These
shared-process measurements do not establish this chat's RAM or a test-caused leak.

## Why adding emit calls after the existing fence is insufficient

FramePublicationFence::publish currently drains every change through a fixed
watermark under its Rust queue lock. Page-close tokens advance flags, then a
collection revision replaces membership; terminal publication clears Contexts
before marking disconnect ready. This correctly preserves blocked-loop getters,
but afterward both Pages are closed and the Context is gone. Emitting two Page
events at that point would make the first callback see the wrong state.

Likewise a JS collection-difference loop cannot reconstruct suppressed/transient
creations, exact opener identity or native ordering. It would introduce a second
lifecycle state machine and still miss callback pacing. Do not patch public
close()/newPage() methods with synthetic events: external closure/discovery and
borrowed disconnect must go through the same Rust-owned publication mechanism.

## Proposed implementation boundary (not implemented)

1. **Reuse typed native causes.** The repo already has pwright-cap-page-events
   types for BrowserContextPage/PagePopup, CDP topology opener mapping and a
   ChromiumTopologyPageEventPublisher with independently optional senders. Reuse
   these portable identities/effects; do not infer opener in JS or install the
   Page-load cursor merely to obtain topology notifications. Audit capability
   registration for missing Page/Context close event definitions before allocating
   IDs/tags. Existing lifecycle command capabilities do not define these events.
2. **Own publication steps in Rust.** Introduce an explicit selected lifecycle
   publication owner/cursor on the existing metadata watermark. Ordered steps
   combine a state transition with one typed notice: add Page then notify Context;
   notify opener afterward; remove/close one Page then notify it; remove Context
   after its Page steps; publish disconnected last. Derive transitions from
   committed, ready native lifecycle effects, not only a final topology diff.
   Preserve transient-event and initialization-failure semantics; bootstrap
   snapshots must not replay historical creation events to later listeners.
3. **Keep retained views immutable.** Prototype a charged published-view cursor
   over immutable revisions plus small visibility masks, or pre-admitted per-step
   revisions if that proves simpler. A reader-retained old view cannot change
   underneath it. Masks/cursors must represent overlap of old/current/pending
   identities, not silently reuse the active 16-Page limit as a pending-event bound.
   Measure layouts and choose the representation before native integration.
4. **Resolve wrappers before retirement dispatch.** The language port must reuse
   existing Context/Page wrappers, even if an intervening getter evicts a closed
   member from the facade cache. A typed delivery retains charged metadata and
   an opaque one-shot step identity. Rust validates/advances the step immediately
   before synchronous EventEmitter dispatch; JS only resolves identity and forwards
   the typed notice. No native lock may be held while constructing foreign values
   or calling user code. Do not retain Browser/Host/runtime through event payloads.
5. **Route every fence consistently.** Audit command/error settlements,
   NativeSessionSignal.wait, deferred/Locator conversions, navigation and initial
   connect. A command cannot resolve past an undelivered earlier lifecycle step.
   Retain fixed-watermark behavior: an older completion cannot publish future
   native changes. The no-event Rust path and v1 compositions remain opt-out;
   Node event selection must be explicit and dependency-proved, not a broad
   default-feature dependency added to core.
6. **Admit teardown and foreign delivery up front.** Charge notice count/bytes,
   metadata retained through callbacks, cursor/queue bodies, native wrapper
   conversions and the weak environment callback/registration. Reserve owned
   creation/closure work before native activation and borrowed discovery before
   local admission. Saturated teardown must still deliver its admitted close
   steps without allocating a fresh revision/queue slot. Do not drop reliable
   lifecycle notices on overflow; fail the affected admission explicitly.
7. **Specify reentry and failure before wiring.** Establish one active drain
   owner, one-shot step consumption and deterministic progress after a callback
   invokes more APIs. Characterize throwing listeners before choosing whether
   subsequent notices/Promise settlements continue or fail; do not invent this
   behavior from EventEmitter alone. Env/Worker destruction must discard pending
   foreign callbacks and reclaim leases without invoking JS or retaining host
   execution ownership. Verify callback failure and conversion exhaustion, not
   only successful listeners.

## Required next verification

- Rust tests for intermediate membership/closed flags, fixed/old fences, ordered
  one-shot notices, retained metadata, per-step reentry, failed conversion,
  admission rollback and saturated terminal/discard paths, with measured bytes.
- Rebuild only selected artifacts and make this unchanged seven-case red gate
  green. Keep NODE-101/103/104 scalar/collection/creation regressions green too.
- Extend the oracle before broad claims: listeners throwing/removing/adding
  listeners, callback-created work, no-listener behavior, initial attachment,
  popup opener closure/first requests, initialization failure and crash semantics.
- Rerun Worker destruction, strict Clippy, Node units/types and exact dependency
  closures. Record native artifact identities and final resource reclamation.

This is an active implementation gate, not a completed event subsystem or full
Playwright-compatible Node release. No new commit, staging or push is authorized
by this checkpoint; the foundation remains the previously committed f76e3f2.
