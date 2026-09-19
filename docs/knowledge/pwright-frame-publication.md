<!-- agent-updated: 2026-09-10T15:40:39Z -->

NODE-130: Local route retirement exposes an explicit readiness wake port after commit. The caller must release all enclosing lifecycle locks before notifying. This does not drain metadata, mutate collection visibility or emit lifecycle callbacks; existing publication reservations and fences still need composition.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-126: Publication capacity can now be admitted before native work without freezing visibility. Latest-state preparation consumes existing groups, includes later visible descendants and omits previously closed members; commit still rejects any intervening view change.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Runtime disposal/outbox reservation now complements topology reservation. Reserved outbox slots and EventItem leases transfer to current-state child-first disposal, but this does not reserve Node lifecycle groups or rebase frozen observer views. Publication admission, peak storage and native ancestor receipts must be composed before enabling concurrent close work in Node.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: The new opt-in topology reservation primitive permits latest-state commits without holding a gate during native waits. It does not reserve publication groups or rebase existing frozen observer views. Those are required next alongside runtime disposal reservations and typed ancestor receipt ownership; current Node publication selection and binary evidence are unchanged.
[Topology reservation contract and evidence](pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Context-close interruption waits for terminal staging outside its operation token and preserves the upstream-style closed error. Reply-producer/request-queue loss now seals host admission before close settlement can obtain a pre-terminal fence. This preserves Page identities for terminal groups. Metadata layout, publication capacity and native notice dispatch are unchanged; repeated Context/Page owner/peer gates verify original emitters and state at successful Page settlement.
[Close cascade contracts and evidence](pwright-close-cascade.md).

NODE-122: Interrupted Page.close no longer settles on a pre-terminal fence. Rust waits for terminal staging after releasing its operation token, then the existing FFI dispatcher publishes close state and the original emitter before successful JS settlement. No new Frame-publication queue/layout or JS callback policy is added. Default held-receipt, owner-close and peer-loss tests assert this ordering explicitly.
[Selected confirmation](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-119: CollectionBatch can now commit an exact masked view of its own admitted revision. The selector runs outside the queue lock while the batch gate stays closed; foreign revisions fail. Ready/linkage causes stage in native order. A failed earlier creation acknowledgement suppresses its later popup before conversion, not unrelated groups.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: The existing lock-free with_next_notice boundary now passes its immutable revision into foreign conversion. A ready request prepares only missing wrappers without mutating caches; the exclusive native drain still commits before emit and acknowledgement. Internal conversion failure retains the established fail-closed port behavior.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Committed external Dispose effects stage pre-admitted close groups before collection reconciliation. Each group updates the exact planned visibility mask and transfers its raw close permit. Duplicate or post-explicit disposal receipts cannot replay an already-staged notice.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Explicit-close groups transfer each Page's raw close permit after successful group staging. Snapshot reconciliation skips that transferred permit and appends its new collection revision after the groups. Native teardown consumes the exact planned mask, preventing replay of a Page already staged closed.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Environment-thread dispatch prepares existing JS identity, commits the native notice, emits without native locks, acknowledges delivery, then reports original synchronous listener exceptions through Node. Old fences still cannot consume terminal groups; the watcher and operation completions share the same exclusive drain.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Retained Frame identity and event-loop publication

NODE-114: Native terminal production appends Page/Context groups at MAX using pre-admitted slots/bodies. Earlier fences cannot publish them, and disconnect readiness follows every group acknowledgement. The current collection-only source passes its full native revision; future masked producers must pass their exact planned view instead.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Lifecycle groups now share the fixed-watermark queue. Legacy whole-fence publication refuses a group without consuming it; the exclusive drain requires conversion, state commit and acknowledgement. Committed listener failures never roll back visible metadata.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Selected explicit closes can now reserve native cause retention from the exact
prepared plan and receive committed effects. This does not publish the following
Frame/collection snapshot or install callback groups. Join receipts to the same
admitted metadata batch before foreign delivery.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The native ownership observer can now receive an exact committed driver-effect
slice, before output aggregation. FrameTrees still uses snapshot fallback; no
journal or callback-state transaction is wired. Snapshot refresh does not replay
causes and cannot recover direct close-service effects.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 proves the next publication layer needs dependent notice groups: a
synchronous Context page listener failure suppresses its dependent opener popup,
without rolling back membership or preventing later independent notifications
when the diagnostic process handles the error. Async listeners must not hold the
Rust drain until their promises settle. No group cursor is implemented yet.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 implements exclusive fixed-watermark Rust drains. try_begin borrows a
fence for exact-watermark retry; publish_next installs at most one queue change.
Legacy whole-fence publication cannot bypass an active owner. Locks are released
before return, and no foreign callback is invoked. This is not yet a native
notice journal or callback visibility transaction.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-107 adds try_rebase: preserve visible identities/order across an admitted
revision boundary, keep additions hidden, reject missing visible members and
foreign/changed identities. Retire missing members before rebasing; native event
cause order is not inferred. The current full-fence publisher is unchanged.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 implements immutable CollectionView membership over an admitted revision.
published_collections now returns a view; selected readers borrow its ordered
members. Existing fences install full views, preserving whole-fence behavior.
Typed single-member transitions preserve prior views and reject invalid parent /
child visibility. They do not publish closed flags or dispatch lifecycle callbacks.
[Implementation, measured budgets and remaining event cursor](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 establishes a RED lifecycle-event gate: reference 7/7, both candidates
0/7 despite matching final scalar/collection states. Callback-visible closure is
stepwise (one Page removed at a time, Context afterward), so draining the current
whole fence and then emitting notices cannot match upstream. No production
behavior changed; a bounded Rust stepwise publication design is now recorded.
[Evidence and implementation gate](pwright-node-lifecycle-events.md).

## NODE-104: Creation and discovery publication characterization

`test:collection-creation` now compares seven public observations against pinned
Playwright 1.58.2: one/two explicit Context creations, one/two explicit Page
creations, Context/Page retirement immediately followed by replacement, and an
independently created borrowed Page. It verifies unchanged membership during a
blocked Node turn, fresh arrays without mutating retained copies, surviving old
object order, stable repeated enumeration and exact returned/cached wrapper,
Context and main-Frame identities after publication.

The candidate waits synchronously for its native creation results to reach
`pwrightFfiMetrics().queuedCommands`, without running JS promise callbacks or
polling public collections under test. The reference uses a blocked language
observation; its JS-based driver cannot independently progress during that turn,
so this is not described as a reference native-completion receipt. For discovery,
a separate Worker locates an exact anchor URL inside a controller-owned native
Context, creates a Page there, and acknowledges the actual Target.createTarget
response via SharedArrayBuffer. The subject remains blocked; candidate Frame
admission is also observed before the collection read. No event is fabricated.

The Worker cleans up only its exact returned target. The parent closes only its
owned anchor Context, verifies borrowed disconnect preserved the anchor, awaits
Worker exit and asserts zero runtime cleanup/host leases and FFI reservations
after candidate disconnect. Other tasks' Pages are not cleanup targets. The
test uses no HTTP fixture, new production runtime or native addon rebuild.
Existing 30-second cases and 180-second/4 MiB stdout/1 MiB stderr lane supervision
bound the diagnostic; per-wait controller deadlines remain 10 seconds.

### Two harness corrections, not product fixes

The first reference run passed 6/7: two concurrent Page creations appeared in
the reverse of invocation order. The pinned client inserts Pages into a Set when
its channel `page` event arrives (`playwright-core@1.58.2/lib/client/browserContext.js`,
`_onPage`/`pages`), rather than reserving collection positions at invocation.
The test now records invocation indices as diagnostic evidence while asserting
within-connection stable order, original-object relative order and membership.
It does not impose deterministic ordering on independent concurrent completion.
The separately checked upstream source checkout is 1.63.0-next, not the oracle;
only the installed 1.58.2 package and actual reference runs establish this gate.

The first candidate run passed 5/7 because replacement readiness incorrectly
counted Close-lane results as Command-lane results. The corrected diagnostic
waits for the number of creations only; close completion is awaited afterward.
NODE-101 independently proves blocked native retirement. Queue snapshots retain
the distinct command/close/watch/running counts, so this correction does not
pretend every close had completed at the blocked read. No Rust behavior changed.

### Verified scope and remaining work

Evidence: `.work/node104-collection-creation/`.

- Corrected reference: 7/7 twice, 3.119 s and 3.156 s.
- Default: 7/7 twice, 6.523 s and 6.769 s.
- Combined: 7/7 twice, 6.301 s and 10.172 s.
- Node units: 47/47; strict types, seven host/eight addon resolved dependency
  selections, JS syntax and whitespace checks pass. Rust tests/builds were not
  repeated because no Rust source changed in this checkpoint.
- `reference-initial.log` and `default-initial.log` preserve the harness failures;
  use `reference-final-1/2.log`, `default-final/repeat.log` and
  `combined-final/repeat.log` for final results.
- Artifacts remain NODE-103 default
  `cf8c6555ca051d587831181225fe7f09a23d48f14f9f763103ff72896682c4e5`
  and combined `2896109c45a7a2fd7af25a79671bcbe62f1bcda7e595df0c19999d5984b891ba`.
  No Rust source, dependency, feature selection or logical budget changed.

This closes the seven specified creation/discovery collection observations, not
the full event/API contract. Inspection of the Node facade still finds no Context
`page`, Page/Context `close` or `popup` event emission (only Browser disconnect).
Next characterize those events and their handler-visible state/order, then design
their bounded Rust-owned publication and language dispatch. Do not implement them
by polling collection differences in JavaScript or infer them from getters alone.

All test/Worker processes exited, and the final dedicated-Chrome inventory has
no node104 marker Page. Exact preflight Page `AC355059CB41BC92D8089217EA359228`
was verified as about:blank and closed. No HTTP route, server or relay was created;
no task-owned background runtime remains. Shared browser/services were preserved.
No staging, new commit or push; overall goal remains active.

Read-only memory snapshots: at 03:58:29 UTC the verified shared backend PID 53189
had 4,830,272 KiB RSS; targeted vmmap at 03:58:55 reported 2.2 GiB physical
footprint. At 04:11:02 RSS was 655,088 KiB, down 4,175,184 KiB from the first
snapshot, with swap still zero. Another cargo process was identified by cwd as
XLQY work and left alone. Fluctuating shared-process RSS/footprint does not prove
per-chat use or attribute that reduction to these tests or their cleanup.

## NODE-103: Rust collection producers and published Node reads

The NODE-102 storage is now connected to selected Node host lifecycle producers
and addon collection reads. Historical NODE-101/102 red/unwired statements below
describe those checkpoints, not this implementation. The unchanged seven-case
collection gate passes on both rebuilt artifacts. This is a bounded lifecycle
subset, not full parity.

### Producer and publication contract

- `ChromiumFrameOwnershipView` exposes borrowed, typed Context/Page membership
  iterators with portable ObjectIds and Context ownership. No native CDP identity,
  extra domain, subscriber, runtime or dependency enters the public projection.
- `FrameTrees::with_collections` explicitly selects the producer. Native
  `HostSession::contexts/pages/page_snapshot` keep their native semantics; the
  addon instead calls `published_contexts/published_pages/published_page_snapshot`.
  These read immutable Rust membership, including retained settings/main-Frame/
  tree metadata after native retirement. Existing cache identity and 32 KiB
  conversion admission remain; JavaScript holds no new lifecycle state.
- Explicit Context/Page creation reserves a revision and replenishes retirement
  capacity before native creation. A `CollectionCreation` guard restores the
  prior pool length on later admission/native failure; existing exact-object
  compensation remains responsible for a native object created before failure.
  Borrowed discovery reserves before local Frame-tree activation. Unchanged
  membership reuses the previous staged signature without a new revision.
- Selected startup preallocates 32 retirement revisions: at most 16 Contexts
  and 16 Pages can retire without intervening creation. Every creation replenishes
  that pool before proceeding. Pure membership removal consumes a reservation,
  so closure does not depend on fresh queue/count/byte admission. Selection and
  partial replenishment failures release their exact leases.
- `begin_collection` freezes the publishable watermark before Frame reconciliation
  can stage Page-close changes. `commit` stages the complete membership revision
  and opens the gate in one queue lock. Previously captured fences cannot expose
  half the update. A watcher stays Pending, rather than spinning on held changes.
  An abandoned batch deliberately keeps the gate closed: the producer propagates
  failure and retires the session. Terminal MAX publication clears the gate,
  publishes admitted Page closure and releases current membership before Browser
  disconnect readiness. Discard drops ownership without delivering callbacks.
- Published revisions have no strong path back to the publication book, host,
  driver or transport. The producer's last-staged signature, pending creation and
  retirement pool are released by `FrameTrees::clear`; external retained metadata
  remains independently charged until its last owner drops.

### Logical resource measurements

On this Mac, one revision now charges **1,232 bytes/one revision slot**, including
the conservative batch-body allowance. It separately reserves one existing
metadata-queue item until publication. The 32-reservation retirement pool plus
its vector storage charges **41,728 bytes/33 revision slots** and 32 queue items.
Selected fixed index/queue state is **31,496 bytes/one state**. These are additive
logical admissions; active/published/retained revisions and object metadata still
carry their own charges. They are not process RSS or a browser RAM reduction.

Four new batch tests cover atomic close publication, a genuinely Pending watch,
abandoned-batch terminal delivery and discard. Three new producer-pool tests cover
selection/clear, partial admission rollback, replenishment exhaustion and creation
guard abandonment. Host release tests pass: default 270 passed/7 ignored;
combined 318 passed/9 ignored. Strict combined Clippy, 47 Node units, strict types,
and resolved dependency checks for seven host/eight addon selections pass.

### Synchronous Context input regression

The Context/baseURL regression exposed an existing mismatch: oversized primitive
baseURL failed synchronously before the Promise error mapper, yielding ordinary
Error instead of the required TypeError. Rust `context_settings::read_base_url`
now measures UTF-8 length before copying and throws the native TypeError through
N-API PendingException in both plain and selected-header creation paths. The JS
catch scope is unchanged; arbitrary user header getter exceptions retain identity.

`test:context-input-boundary` is a candidate-specific resource/exception gate,
not an upstream limit-parity claim. It checks ASCII/multibyte input, selected-header
input and getter identity, no extra Context/command admission and zero FFI/host
leases after disconnect. Cache binding is included before measuring the baseline;
connected watch reservations are compared against baseline, not incorrectly zeroed.

Evidence directory: `.work/node103-collection-integration/`. The initial Context
regression used the wrong fixture directory (404 pages) and still found the real
TypeError mismatch; it is not passing fixture evidence. Final runs use verified
`tests/fixtures` through temporary serve-lib route `/pwright-node103-context/`.
Creation/discovery blocked-event-loop characterization, broader close/crash event
ordering and complete Playwright API compatibility remain open.

### Final rebuilt artifact verification

Pinned oracle: Playwright 1.58.2; dedicated external Chrome 152.0.7977.83 over
loopback CDP. Only default and `drag-frame-extra-headers` were rebuilt here.

| Lane | Default | Combined |
| --- | --- | --- |
| Unchanged collection publication | 7/7 | 7/7 |
| Empty Context publication | 1/1 | 1/1 |
| Browser connection publication | 5/5 | 5/5 |
| Page.isClosed publication | 8/8 | 8/8 |
| Public Page enumeration | 7/7 | 7/7 |
| Context/baseURL | 16/16 + 4 boundary checks | 16/16 + 4 boundary checks |
| Worker destruction subset | 3/3 | 3/3 |
| Candidate input boundary | 2 passed, 3 unselected skips | 5/5 |
| Selected header creation/getter comparisons | Not selected | 3/3 |

Final SHA-256 identities (also in Page/Context JSON reports):

- default: `cf8c6555ca051d587831181225fe7f09a23d48f14f9f763103ff72896682c4e5`
- combined: `2896109c45a7a2fd7af25a79671bcbe62f1bcda7e595df0c19999d5984b891ba`

Use `*-verified.log/json`, `input-default-pass.log`,
`header-creation-combined.log` and final build/Clippy/unit/type/boundary logs in
the evidence directory. `input-default-verified.log` and `input-default-final.log`
are earlier test-baseline failures, not final failures. `contexts-default.json`
records the pre-fix TypeError regression. Earlier pre-boundary-fix artifact
`2e46580c...` and initial green runs are intermediate evidence only.

After Page enumeration, both profiles report zero live host objects/scopes/leases,
pending calls/operations/cleanup, URL entries/bytes/names/active Frames/publications,
and Frame revisions. The retained disconnected wrapper still owns fixed Frame
storage (31,496 bytes/one state); Context settings return to their fixed 560-byte
baseline. This is not a claim that a retained JS wrapper consumes zero memory.

All test/build processes exited. All 35 recorded diagnostic relay/header-fixture
ports have no listener. Exact task preflight Page
`6E8C76B445DC9408B3D8134AE2B4D94E` was verified as about:blank and closed. The
`/pwright-node103` registration expired by its timeout; `/pwright-node103-context`
was deregistered, and neither remains in serve-lib list. Shared Chrome, XLQY
Pages/services and the shared serve-lib daemon were not stopped. No task-owned
background runtime remains; no staging, new commit or push was performed.

At 03:47:39 UTC the dynamically verified shared Codex backend PID 53189 had
2,412,400 KiB RSS; at 03:56:38 it had 3,531,152 KiB, then 2,933,456 KiB at
03:57:10 with vmmap physical footprint 2.7 GiB. Swap remained zero and
memory_pressure reported 73% free. These fluctuating shared-process measurements
cannot attribute memory to this chat; they are separate from the Rust lease
reclamation assertions above. No shared process was killed or restarted.

## NODE-102: Bounded Rust collection revision storage

The first implementation part of NODE-101 is present in
crates/pwright-node-host/src/frame_metadata/collections.rs. It is integrated
with the existing Rust metadata queue/fences, but native lifecycle producers
and public Node collection getters are NOT wired to it yet. The NODE-101 red
browser results remain the current compatibility status; there is no rebuilt
Node artifact or claim that the six discrepancies are fixed.

### Reservation, ownership and publication

FramePublications::reserve_collection takes an explicit FrameTreeFactory.
It requires the preallocated metadata queue, reserves its EventItem slot,
reserves the revision's count/bytes from the existing Frame tree ledger, and
allocates both fixed-capacity vectors plus their Arc body before returning.
The caller must do this before native activation. There is no topology query
or new runtime, task, subscriber, dependency, resource scope or JS state.

CollectionReservation has exclusive access to its unpublished Arc and can
fill at most 16 Context records and 16 Page records without vector growth.
Context records contain portable identity and ContextSnapshot settings.
Page records contain portable identity, owning Context, retained main FrameNode
and FrameTree. Invalid/duplicate identities, absent Contexts and reused root
identities are rejected without mutating the projection. The main Frame is
cloned directly under its retained tree lock, without allocating a Frame list.

The reservation carries a Weak publication-book identity and a pre-admitted
queue slot. stage_collection validates the exact book and rejects discarded or
terminal state, then moves the already allocated revision into the existing
FrameChange queue. Successful filling/staging performs no new heap allocation;
error diagnostic Strings are outside this successful-path claim. Abandonment
and admission failure release both budgets through RAII.

A fixed FramePublicationFence publishes only revisions at or below its watermark.
published_collections clones the current Arc without allocating or reading native
topology. The membership vectors are immutable; their retained object metadata
continues to follow normal URL/tree/closed publication semantics. Holding an
older revision keeps its original membership and its explicit lease until the
last reader drops it. It does not keep the queue, host or browser alive.

The terminal MAX fence releases the current projection before marking Browser
disconnect ready. It needs no new collection allocation, including when ordinary
queue admission is full. Environment discard drops the queue's and current-view
ownership without advancing discarded callbacks; caller-retained revisions keep
their own charged metadata. Older fences cannot republish a popped/discarded
revision. Reserve/stage after terminal state is rejected.

### Measured logical budgets on this Mac

- One revision pre-reserves 1,144 bytes in the existing Frame tree ledger,
  covering Arc body/control, both full vectors and a conservative reservation-body
  allowance. It consumes one of that ledger's shared revision/EventItem slots.
- It also reserves one metadata-queue EventItem while reserved/queued. That queue
  slot is released on delivery; the revision's count/bytes remain charged while
  published or retained. These are two different ledgers, not one double-counted
  item. Active Page close permits still share the 256-item metadata queue.
- The new optional current-revision owner adds eight charged fixed bytes to the
  selected index/queue layout: 31,256 bytes and one state, formerly 31,248.
  No published revision is allocated unless a producer explicitly reserves one.
- Replacement transient overlap is measured as two times 1,144 bytes and two
  revision slots while an old reader remains. Last-reader drop returns the exact
  delta. Native active-object limits do not hide retained-version consumption.

These are layout-dependent logical Rust admissions, not total allocator overhead,
process RSS, per-chat RAM or a measured browser memory improvement. Existing
addon conversion bounds have not changed because no Node read path changed.

### Verification and remaining integration

Eighteen deterministic Rust tests cover fixed/old fences, replacement overlap,
queue-slot pre-admission, byte rejection rollback, abandoned/foreign reservations,
metadata survival after native retirement, saturated terminal publication,
discard with retained readers, absence of a strong path back to the book,
identity validation, both 16-entry capacities, shared revision count pressure,
explicit queue selection, post-discard/terminal rejection and measured layout.

Evidence: .work/node102-collection-storage/.

- Initial default run: 258 passed, 7 ignored (first 13 new tests).
- Final default: 263 passed, 7 ignored; combined: 311 passed, 9 ignored.
- Dedicated combined layout test: 1/1; revision_bytes=1144,
  fixed_bytes=31256, fixed_states=1.
- Strict combined host/addon all-targets Clippy passed with -D warnings.
- Node units: 47/47; strict types and resolved dependency checks for seven host
  and eight addon selections passed. Formatting and whitespace checks passed.
- No Node addon build/copy, browser run or new test fixture was performed.
  Installed default/combined SHA-256 remain NODE-100's
  2bf67640c21ebcf951d842c71965224ff3e8501526aa500c2d0c5871bd470c0d and
  df23f52e832149381f644a0d401dd08c19f0940b81f753a6f2e69acffc141d6f.

Next wire the same pre-admission through explicit Context/Page creation and
borrowed discovery, and project their committed ownership/settings/root metadata.
Creation compensation must release a failed reservation. Ordinary Page closure
and membership removal must share one publication boundary; merely enqueueing a
new collection after a previously publishable close token would leave a race.
Context removal must follow its children. Do not silently read newest topology
when building an older fence or change native admission to use the public view.
The current module does not itself solve these producer-ordering requirements.

Then replace HostSession::contexts / page_snapshot's direct topology reads for
the selected Node path, while preserving cached identity, 32 KiB conversion
proofs and uncached metadata binding after native retirement. Initial connection
must deliver its first collection before returning the Browser. The code locations
and compatibility matrix remain in NODE-101 below. Rebuild both native profiles
and rerun that unchanged red gate plus enumeration, pressure, Worker and scalar
lifecycle regressions. Creation/discovery timing and public close-event ordering
remain additional open gates. The full goal is not complete.

All launched test/build/check commands reached terminal status. No task-owned
background process was left running; shared Chrome/services were not changed.
At 03:14:30 UTC shared backend PID 53189 RSS was 1,762,320 KiB and shared Chrome
PID 54462 RSS was 352,512 KiB; swap was zero. These are process-level readings,
not session attribution. HEAD remains f76e3f2; no staging, new commit or push.
The final 03:16:52 UTC backend sample rose to 4,990,880 KiB RSS, with vmmap
reporting 4.7 GiB physical footprint. memory_pressure reported 72% free and the
preceding swap check remained zero. Task build/test processes had already exited;
this shared-process growth is unexplained, not attributed to this chat or the
1,144-byte Rust revision storage. Continue read-only sampling; no restart/kill.

## NODE-101: Context and Page collection publication discrepancy

This checkpoint adds a reproducible RED compatibility gate, not a production
fix. Browser.contexts() and BrowserContext.pages() still consult native topology.
When native cleanup finishes while Node is blocked, the collections lose members
even though Browser.isConnected remains true and Page.isClosed remains false.
NODE-099/100 fixed those scalar getters; their bounded results do not establish
collection publication compatibility.

### Pinned reference and observed differences

The installed Playwright 1.58.2 client returns copies of its local sets:
browser.js:100 and browserContext.js:261, under
tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/client/.
Page._onClose (page.js:175) sets closed and removes the Page from its Context
before emitting close; Context._onClose (browserContext.js:429) removes the
Context from its Browser before emitting close. These are Node-delivered local
transitions, not live queries against Chrome routes.

The seven-case test:collection-publication lane retains a Context and two exact
Page identities, and compares initial, blocked and completed states. It checks
membership, surviving relative order, independent returned arrays, retained
array contents/order, Browser.isConnected and both Page.isClosed values.
Borrowed Pages are selected by exact task-specific about:blank markers; other
tasks' Pages are excluded, not closed. Initial discovery order is not required
to be identical between separate connections.

| Case | Upstream during blocked Node | Candidate discrepancy |
| --- | --- | --- |
| Owned Page close | Both Pages remain listed | Closed native Page removed early |
| Owned Context close | Context and both Pages remain listed | Context and Pages removed early |
| Owned Browser close | Context and both Pages remain listed | Context and Pages removed early |
| Borrowed Browser disconnect | Borrowed Context/Pages remain listed | Context and Pages removed early |
| Worker-confirmed transport drop | Borrowed Context/Pages remain listed | Context and Pages removed early |
| Worker live-transport control | Collections remain intact | Matches |
| Worker-confirmed external Page close | Both Pages remain listed | Exact closed Page removed early |

Both candidate profiles match every initial and final observation; a separate
log audit asserts these equalities and identifies only blocked registered/pages
fields as different. After Page-only closure the sibling remains usable; borrowed
disconnect leaves both controller-owned native Pages evaluable. Copies returned
before closure still retain the original identities, and subsequent calls return
fresh arrays in all observations. No new Page/Context close event support or
event-handler collection ordering is claimed.

### Mechanism and bounded evidence

The transport cases reuse NODE-100's actual WebSocket forwarding Worker, with
independent confirmation of both socket closures while the subject Node thread
waits. External Page closure reuses NODE-098's exact-marker controller Worker,
which confirms native Page.close completion. These Workers are test infrastructure,
not a production driver. The candidate additionally waits synchronously for
native host retirement, or an activeFrames decrease for Page/Context closure.
That aggregate counter alone does not prove which target retired; the external
Page Worker independently confirms the exact target. Ordinary upstream close
probes block for 100 ms; their stronger native-confirmation controls are separate.

No public collection getter is used as the native-progress predicate. The Node
thread cannot execute completion/publication callbacks during Atomics.wait.
Each test is bounded at 30 seconds, Worker confirmation and diagnostic awaits at
10 seconds, native metric polling at 5 seconds, and the existing lane supervisor
at 180 seconds with 4 MiB stdout/1 MiB stderr. There is no HTTP fixture.

Evidence directory: .work/node101-collection-publication/.

- Initial reference.log: 5/7. Two failures were the test's incorrect assumption
  that separately attached borrowed Pages always enumerate in creation order.
  Retained as harness-development evidence, not a product discrepancy.
- Corrected reference-verified.log: 7/7, 5.073 seconds; reference-repeat.log:
  7/7, 4.251 seconds. Membership labels are normalized, but within-connection
  surviving order and retained-copy order are explicitly checked, not discarded.
- default-baseline.log: 1/7, six collection-publication failures, 10.772 seconds.
- combined-baseline.log: 1/7, the same six failures, 14.619 seconds.
- Node unit tests 47/47 and strict types passed; new test syntax and whitespace
  checks passed. No production Rust change or native rebuild occurred.
- Default tested SHA-256:
  2bf67640c21ebcf951d842c71965224ff3e8501526aa500c2d0c5871bd470c0d.
- Combined tested SHA-256:
  df23f52e832149381f644a0d401dd08c19f0940b81f753a6f2e69acffc141d6f.
  These remain NODE-100 artifacts; they do not fix NODE-101.

### Rust implementation gate

Root cause locations are HostSession::contexts and HostSession::page_snapshot
in crates/pwright-node-host/src/lib.rs. They enumerate the current live
composition, or return empty after teardown. The addon context_settings.rs and
page_snapshot.rs then serialize that result; the JS facade prunes identity
caches immediately. ContextMetadata::visible's name refers to native creation
commit visibility, not Node event-loop publication. Context removal/clear and
FrameTrees retirement discard native index entries before the existing Node fence.

The next implementation must introduce a Rust-owned published membership view,
not make JS cached IDs authoritative and not delay native cancellation/cleanup.
The design requirements are:

1. One coherent projection associates published Context identities/settings with
   published Page identities, Context ownership, root metadata and FrameTree.
   Uncached descriptors must still be constructible after native retirement but
   before publication. Retain only charged metadata, never LiveSession, runtime,
   transport, lifecycle services or task handles. Native topology remains the
   operation-authority source; stale public visibility cannot authorize a call.
2. Stage creation, discovery and removal through the existing publication
   watermark. Page removal and its published closed flag must become visible
   together; Context removal follows child retirement in the same delivered
   boundary. Terminal collection removal must be published before the existing
   Rust Browser disconnect event port. Ordinary Page close must not hide Contexts.
   Initial attachment must install its bounded view before connect resolves.
3. Admit published storage, pending revisions/deltas, descriptor ownership and
   terminal capacity before native creation/discovery. Account for an old visible
   member while the native slot is reused: a 16-live-object bound is not a bound
   on all retained/backlogged metadata. Preserve bounded 32 KiB conversions and
   their compile-time layout proofs, or explicitly revise/prove them if the
   chosen representation changes maximum simultaneously published descriptors.
4. Reuse the single metadata watch and fixed completion fences. Do not expose
   latest native state through an older fence, coalesce away observable revisions,
   or introduce an unbounded history/retry queue. Lock ordering must not invert
   the current publication-book -> retained-view ordering. No Arc cycle from
   a retained projection back to the publication queue is permitted.
5. Teardown under full ordinary admission must require no new allocation.
   Discard must release pending and published ownership without foreign callbacks;
   native cleanup stays prompt. Prove exact bytes, revision/item capacity, slot
   reuse, duplicate retirement, old/terminal fences and last-reference reclamation
   in deterministic Rust tests before integrating the N-API read path.

These are implementation requirements, not a completed storage implementation or
a measured new memory budget. Existing terminal Page permits cannot silently be
reused for extra uncharged collection work. Creation/discovery timing while Node
is blocked and public close-event ordering still require additional oracle cases.
Keep the unchanged seven-case red lane for the fix, then rerun Page/Context
enumeration, metadata-pressure, Worker destruction and scalar lifecycle gates
on rebuilt default/combined artifacts, followed by dependency/type/resource proofs.

### Cleanup and handoff

All launched commands reached terminal status (the two baseline failures are
expected assertions, not hangs). All 14 recorded temporary relay ports had no
listeners after completion. The exact preflight Page
3EA2EAC508486A82A7012EF816B4EE04 was closed; unrelated XLQY Pages and shared
Chrome/services were preserved. No HTTP fixture, server or native build was
started by NODE-101.

At 02:58:15 UTC, shared Codex backend PID 53189 RSS was 1,670,112 KiB and shared
Chrome PID 54462 RSS was 344,864 KiB, with zero swap. RSS fluctuated during the
turn; a separate 02:57:34 vmmap sample reported 2.1 GiB physical footprint for
the backend. These measures are neither equivalent nor per-chat attribution.
No memory cleanup/restart was performed. No staging, new commit or push.
Final 03:00:30 UTC read-only check: backend RSS 3,331,712 KiB, reported physical
footprint 3.2 GiB; Chrome RSS 347,024 KiB. System memory_pressure reported 70%
free and the immediately preceding swap check remained zero. The changing shared
backend footprint warrants continued sampling but does not identify this chat
as the owner. Final syntax, cargo formatting and whitespace checks passed;
no test/Worker processes or native panic/cleanup-failure diagnostics remained.
The active goal remains incomplete; the next action is the Rust publication
implementation and its bounded ownership tests, not another status-only audit.

NODE-100 separates terminal metadata publication from Browser event dispatch.
A MAX fence publishes Page closures and marks disconnect ready; the existing
Node event continuation invokes a Rust one-shot port before emission. Older
fences and discard cannot authorize or regress public disconnect state.
[Contract and tests](pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 adds Page-close tokens to the existing URL/tree publication queue.
Every Page reserves its eventual close slot before activation; full ordinary
admission cannot suppress closure. A reserved terminal sequence publishes the
whole session-retirement batch before disconnect delivery. Discard releases
metadata without changing the visible closed flag. The selected preallocated
queue and index now share a 31,248-byte/one-state charge on this Mac, held through
the publication endpoint lifetime; NODE-052's old 7,672-byte figure is historical.
Retained close tokens keep their tree/URL leases until delivery or discard.
[Contract and proof](pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-098 confirms that Page.isClosed also needs event-loop publication: an
upstream Page remains locally open while Node is blocked even after a controller
Worker has confirmed native close. Existing Tree.closed is native-only and must
not be exposed directly. Bounded terminal publication, including queue saturation
and environment discard, is the next implementation gate; current Rust is unchanged.
[Reference contract and proposal](pwright-page-close.md#node-098-pageisclosed-reference-contract-and-failing-candidate-baseline).

NODE-095 makes actual native detach independently terminate selected pending
initialization. Topology still processes its original ordered detach event;
vanished unpublished Pages are discarded without sealing the shared driver.
[Contract and evidence](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 routes unregistered Context attachments through the selected default
view before native initialization. Vanished sessions still fail the exact
native check and publish no stale Page/Frame; live Contexts remain borrowed.
[Contract](pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-092 prevents an already-gone unsupported child inspector session from
aborting Page connection initialization after resume succeeds. It does not change
Frame publication semantics or imply Worker API coverage.
[Evidence](pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-091 composes header session-close observation with selected Frame lifecycle
callbacks, including prepare, discard, initialize, reconcile, event and seal.
Frame publication itself is unchanged. Header-only observation does not opt into
Frame lifecycle domains. [Contract](pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-090 handles an exact missing-session receipt before discovered-Page
publication/network registration without publishing a ready Page or Frame.
Already-routed initialization uses the existing exact-session detach transaction.
Both paths explicitly discard abandoned, never-initialized observer storage;
queued detach events remain idempotent and unrelated live owners survive.
[Scope](pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-088 selected header jobs progress alongside topology/lifecycle waits without
restarting an observation. Primary error delivery can precede compensation drain;
completion wakes obsolete-session reconciliation. Full attachment-handoff and
public event-order equivalence remain unverified.
[Publication scope](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-084 installs initial Context headers on the existing Rust topology driver
before Context descriptor publication. Later Page initialization reads that state
through the existing scoped initializer; no new event cursor is added. Expected
invalid syntax retains the NODE-083 exact-Page recovery path.
[Creation publication boundary](pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 no longer propagates expected header syntax rejection through the
shared topology observer's fatal-error path. It records a typed Page-local
failure, finishes ordinary initialization, closes the explicit created Page,
reconciles retirement and then rejects its creation result. Full transient Page
event parity and other initializer/transport failures remain separate work.
[Publication and recovery contract](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 public header futures use the existing publication-aware settlement
boundary. Synchronous language values never survive into async work; the FFI
ticket remains held through Rust result publication and JS conversion. The live
header lane verifies zero pending/reserved FFI state after owned Context close;
this does not prove OOPIF/worker or cancellation/failure timing compatibility.
[Selected FFI evidence](pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 initializes selected native headers before resume and reconciles stale
session header state before replying to driver-queued operations. Existing
utility-world/publication fences remain. A replaced route is not proof its old
CDP session ended; clearing current headers must not reveal stale session values.
[Live regression and remaining OOPIF gate](pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-079 passes the owning Page/Context identity to pre-resume initializers.
A new Page's ID is reserved but not yet published; it must not create a ready
foreign wrapper. OOPIF callbacks receive the owning Page, not a fabricated new
Page. Existing publication/utility-world fences remain in place.
[Scoped initialization proof](pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-076 delivers Frame.goto through the existing spawn_published settlement
fence. The native result lease remains alive across publication and complete
N-API conversion. Immediate Frame.url/Page.url reads and exact child Response.frame
identity are now exercised through the actual public package, with no additional
cursor or duplicate JS topology owner.
[Selected public boundary](pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075 projects the settled native arena after the existing final host
publication fence while retaining operation admission/deadline. A typed OOPIF
result carries the selected portable Frame ID; the live test confirms its URL
and unchanged parent/sibling. Foreign event-loop delivery/Node wrapper mapping
is still a separate boundary, not implied by the Rust host result.
[Checkpoint](pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-073 host native navigation registers before dispatch, then runs the existing
host final-publication fence before returning its Rust result. Its absolute
deadline includes both preparation and publication. Node event-loop watermark
delivery remains the foreign boundary; the live Rust child test explicitly
publishes that watermark before reading the retained child URL.
[Execution contract](pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 applies native notices and operation readiness under the Page lock;
advancing elapsed timers publishes readiness before a later commit clears it.
Receipt delivery uses the same lock and current snapshot. Rust host registration
performs the existing initial fence, but the full Frame.goto executor still
needs its final public-completion fence and native-call lifetime integration.
[Ordering contract](pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-070 optionally installs the lifecycle registry in the Rust host before
initial topology observation. Rust readiness-handle retrieval first uses the
existing host publication fence. This is not a new JS event stream or Frame.goto
completion fence, and the default Node artifact does not select it.
[Contract](pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 observes lifecycle after ordered topology application, using current
Frame/session ownership and initial tree document seeding. This is a synchronous
Rust observer under existing locks, not a JavaScript event emitter or a complete
FFI command-return fence. Its host installation remains open.
[Contract](pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-068 puts document-commit descendant disposal through the existing
topology/runtime/service-route transaction before URL/name observation and
ownership reconciliation. Child/OOPIF and unchanged-native-ID main commits
now retire old children there; hash navigation does not. Both old renderer
retirement and retained portable Frame identity are verified in a live handoff.
This does not add public Frame.goto or a second metadata publication channel.
[Contract and evidence](pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

NODE-064 adds a borrowed navigation route, not a second metadata publication
path. Frame.goto must use the existing publication fence before FFI completion
and retain the selected response Frame. Its six native settled-state comparisons
do not establish event ordering or expose a new Node method.
[Navigation design and remaining public gates](pwright-frame-navigation.md).

NODE-063 binds selected ancestry from an immutable FrameTreeRead, even after
reentrant publication retires a selected node. The weak wrapper cache rechecks
identity after callbacks. Public lookup passes new/already-open nested ordering
cases; this is not exhaustive OOPIF or event-timing parity. Frame publication
regressions pass 18/18 and Page enumeration 7/7 on the rebuilt addon.
[Ownership and evidence](pwright-frame-lookup.md#node-063-public-lookup-over-the-rust-kernel).

NODE-062 keeps Context metadata separate from Frame publication. Descriptor
conversion proofs cover Page and Context reads independently; neither changes
the fixed 7672-byte Frame index. The final addon passes public Page enumeration
7/7 and shared native glob conversion 1647/1647 plus seven boundary checks.
Earlier attachment failures are preserved and do not have a production fix.
[Artifact-specific evidence](pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061 refreshes public Page enumeration to 7/7 and the original-layout native
glob boundary to 1647/1647 plus seven checks on the unchanged selected artifact.
Final native metadata returns to the fixed host-index baseline. This updates
the last-run status only; intermittent connection failures remain undiagnosed.
[Diagnostic and regression evidence](pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 adds an internal glob conversion primitive borrowing the existing
Frame-tree metadata ledger. Actual native tests verify retained closed-tree use
and last-wrapper release; it does not add public lookup or change publication
semantics. The rebuilt artifact's original attachment/Page regressions remain
failing, so earlier public passes cannot be carried forward automatically.
[Scope and evidence](pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-057 re-ran Mouse on this unchanged artifact: both experimental background
setup and the original default setup pass 44/44. That does not explain the
earlier extra events. See [the setup evidence](pwright-raw-mouse.md#node-057-setup-experiment-not-a-causal-fix).
Full Frame lookup now has a [version-pinned design and 14 oracle characterization tests](pwright-frame-lookup.md);
candidate lookup is not implemented yet.

## NODE-056: bounded navigation names and public Frame.name

`Frame.name()` is now a synchronous read of the Rust-published navigation
snapshot, not a DOM lookup or an evaluation. Full navigation publishes URL and
name together in one immutable revision at the existing event-loop fence.
Same-document URL changes preserve the previous name allocation. Initial
attachment, a new Page, and navigation without a CDP name use the empty string;
a name-only change still creates a publication. Retained detached/closed Frames
keep their last published name without keeping a HostSession, FfiBudget,
execution registry, transport or task alive.

The optional `ChromiumFrameUrlObserver::update_navigation` callback defaults
to the original URL-only callback. Initial Frame trees and full-navigation
events use it only after the existing route/identity checks; same-document
events still call `update`. URL-only observers remain valid and no extra
CDP subscription or task is installed. The Node host explicitly selects
`FrameMetadata::with_names`; a URL-only metadata book has no name ledger
and rejects name reads as unsupported.

### Name admission and foreign conversion

Names use a separate metadata-only ledger at extension scope 121/3:
16 KiB maximum UTF-8 name, 2 MiB total logical storage and 512 lease slots
(`2 * 128 entries + 256 publications`). Each nonempty name reserves its
UTF-8 bytes, `BudgetedName` body and two Arc counters **before** copying.
Empty names require no allocation. Identical names share their charged Arc
across URL revisions. Replacement retains the old charge until all old views,
queued publications and in-flight borrowed conversions release it. Exhaustion
of name bytes, URL bytes or publication items leaves both metadata views and
the pending queue unchanged; temporary reservations roll back.

`nameBytes` reports this separate logical charge, not URL bytes, V8 allocation,
allocator overhead or process RSS. It is optional in the public diagnostic
TypeScript type because the historical default addon predates the field.
N-API converts a borrowed Rust name directly to a JS string: the immutable
revision stays leased during conversion, and no second native string copy or
callback under the metadata mutex is needed. Older addons fail `Frame.name()`
explicitly with the rebuild/unsupported error.

### Upstream comparison and current evidence

The local pinned upstream client initializes `_name` from the Frame initializer,
updates it on navigation, and returns `_name || ''`. The public workload now
also proves a less obvious detail: after setting `window.name = 'window-你好'`
and then the iframe attribute to `attribute-你好`, both implementations retain
`keyboard-frame` through the same-document navigation, then report
`window-你好` after full navigation. The attribute is not the authoritative
published name.

Evidence is under `.work/node056-frame-names/`. `frames-final.json` records
**18/18** shared public cases against Playwright 1.58.2 in Chrome 152, on local
and sandbox/OOPIF fixtures. It covers initial names, window/attribute mutation,
same/full navigation, nested empty names, retained detach/close values, stable
Frame identity, repeated reads with unchanged ledgers and final native cleanup.
All owned Contexts/targets were absent afterward; name bytes, URL entries and
host operations/leases were zero, with only the fixed 7672-byte Frame index.
`pages.json` additionally passes the existing **7/7** public enumeration gate.

Selected addon SHA-256:
`244a7b104e706185cbfdb0585845a91664282d4a4a9a1d2839755efda6d03d6f`.
The default historical addon remains
`8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.
Fifteen new native metadata tests cover name publication, idempotence, empty
and disabled selection, UTF-8/per-book limits, old/new overlap, rollback,
discard, last-reference release and unlocked borrowed conversion. Three new
composition tests cover initial-tree names, omitted-name clearing and
same-document callback selection; existing URL-only observer tests still pass.

Release verification: selected host **183 passed / 9 ignored**, default host
**181 / 7**, Chromium composition **315 / 15**; strict all-target Clippy for
host/addon/composition passes. Node unit tests **42** and strict TypeScript pass.
The import/dependency boundary gate retains its existing closure counts.

The pressure gate's first attempt (`binding.log`) stopped before creation:
ambient Frames made the initial state count even, so two-state retained-child
increments reached 254 instead of the required 253. The test now conditionally
adds an owned auxiliary Page with an unenumerated child (four Page states plus
one child node), preserving the exact 253-state boundary instead of weakening
the assertion. `binding-final.json` passes: 253 states, five revisions and
46256 logical bytes before/after rejection, 118 retained Frames and zero
unexpected targets. The auxiliary Page is owned and cleaned by the test Context;
unrelated Pages remain untouched. This adjustment is test-only.

### Open Mouse regression diagnostic

The same artifact does **not** have a passing current Mouse regression gate.
Unmodified full comparisons matched only 7/44 (`mouse.json`) and 4/44
(`mouse-repeat.json`). In the first down/up-only case, the first run's oracle
also received pointer/mouse moves near (169, 461); in the repeat those extra
moves appeared in the candidate near (171, 460). These were not requested by
the workload. Do not filter them away or attribute every discrepancy to them
without checking individual cases.

`probe-idle-mouse.mjs` is a bounded, non-conformance diagnostic: it creates
only owned Contexts/Pages, sends no Mouse/Keyboard commands, records two
consecutive two-second windows on each provider, verifies cleanup, and writes
at most a 256 KiB report with a 30-second overall deadline. It uses the existing
raw Mouse fixture; supply `PWRIGHT_IDLE_MOUSE_REPORT` as an unused absolute path,
the selected native profile, loopback CDP endpoint and `PWRIGHT_MOUSE_FIXTURE`.
The first single-window experiment (`mouse-idle.json`) recorded two events in
each provider with no input requests. The final two-window source records
oracle 0/0 and candidate 2/0 (`mouse-idle-windows.json`). This is consistent
with a late initial activation/pointer event, but does not prove its source or
rule out concurrent input. No production Mouse behavior or conformance event
filter was changed. Diagnose this and establish a valid input-isolation/setup
boundary before claiming the current artifact passes the Mouse regression.

The first live attempt (`frames.log`) ran before a failed fixture registration
was corrected: its URL was 404, so its initial-child timeout is not conformance
evidence. `frames-verified.log` then exposed a wrong test assumption on the
oracle: full navigation does not necessarily publish the iframe attribute.
The final workload waits for a real navigation URL and compares observed names
without asserting that incorrect value. Both failed logs remain available.
CDP was initially unresponsive, later responded with a different browser-instance
ID without a restart by this task. The old NODE-055 preflight target was absent
from the recovered inventory; the NODE-056 preflight was explicitly closed.
This recovery does not explain or fix the intermittent attachment issue.

### Remaining Frame design

This completes the name slice, not overall Node compatibility. Next design and
implement `page.frame(string | { name?, url? })`, including name precedence,
empty input validation, URL glob/RegExp/predicate forms and stable first-match
collection semantics. Do not call a name-only subset full lookup compatibility.
Frame event delivery/order, complex initial-attachment collection ordering,
Locator/Handle evaluation activation and the attachment investigation remain
open. No browser is bundled or launched by the Node package.

## NODE-055: admitted Page descriptors and cache-aware enumeration

The new addon no longer emits unaccounted `NativeFrameMetadata` boxes in Page
descriptors. `NativeFrameTree` owns a `FrameTreeBinding` with its own state/byte
leases until native finalization, retaining only metadata and its ledger. It
does not retain HostSession, FfiBudget, an execution registry or transport.
The host's original URL/tree fields remain available to Rust consumers, while
the addon transfers only the admitted tree/root bindings into native wrappers.

New-Page admission now reserves **four states** before native creation:
root node, Page tree, root binding and tree binding. The root binding is moved
through a Box in the host descriptor, using its already-reserved wrapper-body
storage; it is unboxed when transferred to N-API. This avoids inflating the
lifecycle reply enum, without suppressing Clippy or adding an unbudgeted body.
The exact-limit unit gate accounts for all four states. A real public pressure
test at 253/256 states rejects with no target and unchanged resources:
253 states, three revisions and 45712 logical bytes before and after this run.
The previous three-state checkpoint below is historical.

### Read boundary and identity cache

Synchronous `contexts`, legacy `pages` and the new `pageSnapshot` reserve a
metadata-only `FrameConversion` guard before copying arguments or native DTOs.
The guard shares the existing revision/EventItem cap and reserves 32 KiB plus
its own body. It remains live through actual `ToNapiValue`, including failure.
A compile-time size assertion covers host/addon DTO layouts, at most 16 live
Pages/Contexts, decimal IDs, conversion overlap and up to 256 cached Page IDs.
That larger cache-input bound is intentional: externally closed Pages can stay
in the JS cache until enumeration, so a 16-ID limit would prevent recovery when
the cache contains more entries than the live browser Page cap.

`HostSession::page_snapshot` reads IDs and uncached descriptors under one topology
lock. JS sends its known IDs, prunes retired entries and reuses existing Page
objects in the returned native order. Cached enumeration allocates no new
native tree/root wrappers. Missing descriptors are admitted before being boxed;
partial failures retain no uncharged metadata. Existing binary compatibility is
explicit: a historical addon without `pageSnapshot` uses the earlier facade
path, and is not claimed to have the new admission guarantees. The default
artifact is not replaced. V8/N-API allocator overhead and process RSS remain
outside these logical byte claims.

### Public popup discrepancy and activation policy

The new shared test first failed at popup discovery. A return-value diagnostic
proved the earlier failure was before discovery: `Boolean(window.open(url))`
was true upstream and false in the candidate. Upstream Chromium
`crExecutionContext.evaluateWithArguments` sends `userGesture: true`; the
candidate's value evaluator omitted it.

`CdpEvaluationProductionProfile::with_user_gesture(true)` now explicitly selects
transient activation for the Node host's Page/Frame value evaluator. Its new
optional CDP client port preserves exact unique-context routing, awaitPromise,
by-value output and exception handling. The native implementation sends typed
`Runtime.evaluate` parameters with `userGesture: true`. Default backend profiles
and ordinary internal CDP reads are unchanged; no global activation override,
retry, new connection or dependency is added. Resource requirements are unchanged.
Locator/Handle user-script activation remains a separate, unverified parity gate.

### Evidence

Selected addon SHA-256:
`327f3809d98f202ef8b77a0fadec0ffc2c6248ffb4f56eb791aebd7c4f9c9911`.
Evidence lives in `.work/node055-page-descriptors/`:

- `pages.log` / `pages-diagnostic.log`: original discovery deadline and explicit
  oracle-true/candidate-false popup result; these failures are preserved.
- `pages-final.json/log`: **7/7** public comparisons, including created/cached
  Page identity, function/expression activation, uncached popup discovery,
  removal, 20 externally closed retained Pages and closed-Context enumeration.
  Two 2000-iteration cache loops do not grow native metadata charges.
- `binding-final.json/log`: creation rejects at the four-state boundary without
  a target or ledger delta. Exact owned native Context/targets are cleaned up.
- Public enumeration finalization returns to the fixed empty index: one state,
  zero revisions, 7672 bytes; URL entries and active host owners reach zero.
- Release tests: value-evaluation leaf 10, CDP 75 and selected host 168 pass;
  strict all-target Clippy, JS contracts, TypeScript and dependency checks pass.
- The same final binary passes Frame 10/10, Mouse 44/44, Keyboard 24/24 and
  drag/Locator 10/10. Reports contain the selected SHA-256 above. Default host
  release tests pass 166/7 ignored, and Chromium composition passes 312/15
  ignored. The 42 JS contracts pass; normal host/addon closures stay 234/235
  and 249/250 respectively.

`pnpm test:pages` runs the shared pinned-oracle workload under the existing
bounded supervisor with GC enabled. Set selected `PWRIGHT_NATIVE_PROFILE`, the
dedicated loopback `PWRIGHT_TEST_CDP` and an unused absolute
`PWRIGHT_PAGE_ENUMERATION_REPORT`; no static fixture is required. Polling is an
explicit settled-collection barrier, not proof of popup/Frame event timing.
Frame names/lookup/events, broader evaluation/Handle parity and intermittent
attachment-timeout root cause remain open.

After all gates passed, cleanup encountered a separate endpoint stall: a bounded
`/json/version` HTTP request received no bytes in 5 s, while the task's CLI close
and `/json/list` read also stalled. Only those two task-owned client processes
were cancelled; Chrome and unrelated Pages were not restarted or terminated.
The `/pwright-node055` serve-lib route was deregistered and confirmed absent.
All test-owned Context/target cleanup was verified inside the completed gates,
but closure of the separate preflight target
`4FD5F7F3EE273C4FF154202F84590C2C` remains **unverified**. Recheck that exact target
when CDP responds; do not close other Pages. This broad HTTP stall is additional
diagnostic evidence, not an identified cause of the earlier attach timeouts.

## NODE-054: creation-time root binding admission

The public quota stress probe reproduced a native Page leak in NODE-053. It
retained 121 detached public Frame wrappers until the tree ledger had 254/256
states. `context.newPage()` admitted a root node and Page tree, created the
native target, then failed when the JS constructor requested the third state
for its main-Frame binding. The promise rejected with resource exhaustion but
the native target and two states remained until Context cleanup.

`FrameTrees::reserve_page` now reserves all three states and their bytes before
the lifecycle service calls native Page creation. `ReservedFrameBinding` is
activated without another reservation and transferred through a move-only
host `PageDescriptor.main_frame_binding`. The addon moves it into its native
Frame wrapper; JS consumes `mainFrameBinding` directly, without a second lookup
or quota claim. Existing-Page enumeration has no creation side effect and keeps
its normal published-node lookup. No lifecycle compensation or retry was added
to JS. Abandoned reservations and failed native creation drop all pre-admitted
metadata through Rust ownership.

The exact same public stress workload now rejects before creating a target:
states **254 -> 254**, revisions **5 -> 5**, bytes **46416 -> 46416**, and zero
unexpected owned native targets. Both old and new runs explicitly dispose and
verify the owned native Context and all its targets. `binding-before.json/log`
preserve the original failure; `binding-after.json/log` preserve the repaired
behavior. Two Rust tests cover full rollback with only two free slots and
successful activation at the exact 256-state limit without double charging.
Existing reservation/activation tests now include the root binding's third
state. The empty host index remains one state/7672 bytes.

### Artifact and regression results

Selected addon SHA-256:
`bdf626196f65e9135e2d60eed546737288c24be8540a388038ad7900162711c7`.
The default addon remains unchanged. Evidence is in `.work/node054-attachment/`:

- Public Frame collections/lifetime: 10/10 on the selected new binary, with
  final native wrapper release and exact owned Context/target cleanup.
- Public Mouse: 44/44 and drag/Keyboard/Locator: 10/10 on the same binary.
- Public Keyboard: 24/24 on the same binary, with zero host owners after close.
- Release host: 163 passed/9 ignored selected; 161 passed/7 ignored default.
- Strict all-target Clippy, 42 JS contracts, strict TypeScript and unchanged
  dependency boundaries pass. No Cargo dependency or background task is added.

`pnpm test:page-binding` runs the bounded public resource regression (no static
fixture required). Supply the dedicated loopback `PWRIGHT_TEST_CDP`, selected
`PWRIGHT_NATIVE_PROFILE` and an unused absolute `PWRIGHT_PAGE_BINDING_REPORT`.
It intentionally expects resource rejection, fails if a native target was
created, and keeps retained wrappers live through the attempt. It does not use
private JS/native handles to manufacture pressure.

### Attachment investigation remains distinct

Before rebuilding, the unchanged NODE-053 artifact passed Mouse 44/44 and drag
10/10 on another run. A bounded two-round attachment matrix also passed all ten
attempts: oracle/candidate alone, oracle with oracle observer, candidate with
oracle observer, and candidate with candidate observer. Thus the earlier
5000 ms timeouts are not a stable reproduction and have **not** been fixed or
attributed to observer concurrency. Chrome was not restarted, unrelated Pages
were untouched, and no deadline was increased. Original failures remain in the
NODE-053 logs; green retries do not erase that unresolved intermittent issue.

The final rebuilt-artifact matrix also passes 10/10. Its phase timings place the
observed mixed-client delay in subject connection (1311/1467 ms with an oracle
observer versus 158/159 ms alone), not subject shutdown (0 ms at this timer
resolution). This is a measurement in the shared live browser, not a causal or
stable performance claim; inspect a reproducible protocol trace before changing
initialization ordering or deadlines.

`pnpm probe:attachment` records bounded per-phase timing, failure phase and
candidate shutdown owner counts. It creates/navigates/closes no Pages. Supply
the loopback endpoint, selected profile and unused absolute
`PWRIGHT_ATTACHMENT_REPORT`. Failures are diagnostic data, so this command is
not a conformance test or evidence that attachment must always succeed.

Remaining work includes the general existing-Page descriptor conversion/boxing
budget, Frame names/lookup/events and event timing/order parity. The root-binding
fix does not claim to bound all V8/N-API bookkeeping or establish full API parity.

## NODE-053: public Node Frame collections and retained wrappers

The rebuilt addon now exposes `page.frames()`, `frame.parentFrame()`,
`frame.childFrames()` and `frame.isDetached()`. Root and child wrappers share
Rust's published metadata; no topology or detach logic is implemented in JS.
The existing async-completion/event-loop watermark still publishes both URL
and tree revisions. Synchronous collection reads never query native topology.

### Ownership and conversion

`FrameTree::read(bytes_per_frame)` borrows an immutable published revision and
admits temporary conversion bytes before any descriptor/ID vector copy. Reads
share the existing 256 `EventItem` slots with revisions; metrics `revisions`
therefore include in-progress reads. The N-API method performs `ToNapiValue`
while its guard is alive, releasing temporary native storage before returning
the already-converted JS array. The separate metadata lookup admits its bounded
20-byte ID input before UTF-8 copying.

`FrameTree::bind` only finds published nodes. Each `FrameBinding` admits its own
native wrapper storage and `LiveObject` slot until native finalization, sharing
the existing 256-state/8 MiB ledger. It retains only its Frame node and resource
leases, never HostSession, FfiBudget, task registry or transport. Retained user
wrappers can intentionally exhaust this quota; GC timing is not assumed and
admission errors are not silently retried or converted into missing Frames.

The Page owns its main Frame strongly. Its child cache uses bounded-current-set
weak references, pruned at each enumeration. A child wrapper retains its original
parent and Page, including after subtree removal. A temporary resolver constructs
parents by exact portable ID without changing the Rust collection's order.
Repeated enumeration creates fresh JS arrays but reuses observable Frame objects
and native bindings. JS/V8 heap bytes are not claimed as Rust-ledger or RSS bytes.
Page close preserves its last published collection and does not mark live Frames
detached; actual child removal updates the retained native detach flag.

The historical default addon remains untouched. On that older ABI, existing main
Frame operations continue working, but the new collection/detach methods fail
explicitly with `UnsupportedOperationError` asking for a native rebuild. There
is no fake root-only implementation. Both current Rust profiles compile the new
metadata API; only the selected `drag-runtime` binary is rebuilt and tested here.

### Public paired gate

`pnpm test:frames` runs `scripts/compare-frame-tree.mjs` under the existing bounded
process supervisor with `--expose-gc`. The same public workload runs serially
against pinned `playwright@1.58.2` and the selected addon on Chrome 152. Both local
and sandbox/OOPIF fixtures cover initial identity, nested attachment, subtree
removal, replacement and Page close: **10/10 observations match**. Each fixture
also performs 1000 repeated Page/child enumerations with stable identities and
unchanged native ledger usage. Exact owned native Contexts and targets are
confirmed absent after cleanup. Post-close finalization leaves zero URL entries,
publications, active host leases and operations, plus the known retained empty
tree index: **one state, zero revisions, 7672 bytes** until its host owner drops.

This is settled-state metadata parity: both sides use an explicit bounded
publication-condition barrier. It does **not** establish immediate post-command
timing or Frame event ordering parity. Frame names, `page.frame()` lookup, Frame
events, complex initial-attachment collection order and full lifecycle/error
parity remain open. Six new Rust admission/lifetime tests and four JS cache
tests cover exhaustion, rollback, last-reference release, retained parents and
identity reuse. Evidence is under `.work/node053-frame-api/`; later checkpoints
below are historical, not the current API coverage.

### Verified artifact and remaining regression gate

Final selected `native/drag-runtime/pwright.node` SHA-256 is
`8e53500bbbc1167a1f84fc9140b59ff2ddc436478f8517fec8d68d1994cb04c2`.
`frames-final.json` records 10/10, and `keyboard.json` records 24/24 on that
exact binary. Release host tests pass 161/9 ignored selected and 159/7 ignored
default; strict all-target Clippy, 42 JS contracts, strict TypeScript, formatting
and dependency-boundary checks pass. A further real Rust Frame gate passes both
modes in 0.61 s (`native-diagnostic.log`). Default artifact SHA-256 remains
`8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.

The first intermediate artifact passed drag/Locator 10/10, but this result is
not transferred to the final artifact. Final Mouse/drag runs and retries failed
at `connectOverCDP` initialization, before candidate input actions. Logs record
both upstream Playwright 5000 ms connection timeouts and Rust 5000 ms topology
initialization timeouts (`mouse*.log`, `drag-keyboard-{final,retry,repeat}.log`).
The shared Chrome process was neither restarted nor given a longer deadline;
unrelated Pages were left intact. Root cause remains undiagnosed, so these final
input regressions are **not green**. Next investigate bounded attachment traces
and observer concurrency before extending parity claims.

Further admission gates must cover wrapper exhaustion during public new-Page
construction and the existing Page-descriptor conversion/boxing path. The new
Frame-node binding and collection-read accounting is not proof that all legacy
context/Page descriptor conversions or V8/native wrapper bookkeeping are bounded.

## NODE-052: live host projection and paused OOPIF attachment

The Rust host now installs the metadata projection and returns a retained
`frame_tree` in its Page descriptors. The public Node facade still lacks
`page.frames()` and child-Frame wrappers; Rust metadata ownership is not proof
of JS `Frame.page()` or wrapper-cache identity.

### Projection and admission

The optional `ChromiumFrameUrlObserver::reconcile_ownership` callback receives
a borrowed `ChromiumFrameOwnershipView` after committed URL observations. It
enumerates portable Frame/Page/parent identities without requiring execution
worlds. Final snapshot drain also projects direct lifecycle-service mutations,
including closing a Page when no later event is queued. URL-only observers keep
their default behavior, and absent observers create no projection/index/task.

Host `FrameTrees` installs bounded active indexes (16 Pages, 128 Frame entries)
and a factory with 256 combined retained states, 256 revisions and 8 MiB logical
storage. The live topology itself remains bounded at 64 materialized Frames.
Index vectors and the two serialized reconciliation scratch vectors are admitted
before allocation. The fixed empty host index charges one state and **7672 bytes**
on this Mac; that charge lasts until the index owner drops, not merely session
close. URLs retain their separate 128-entry/2 MiB admission.

Unchanged ownership consumes no revision/publication slot. A real child removal
stages a detach; whole-Page retirement freezes the last collection without
detaching its Frames. Session teardown clears active indexes and closes tree
admission without capturing the HostSession or transport in retained views.

Page creation reserves the root node, tree and initial revision before native
`Target.createTarget`. Activation consumes those existing leases without double
charging; failed activation follows existing native Page compensation. The serial
lifecycle driver prevents another projection from taking the reserved active
index slots during creation. Abandoned reservations release all their admission.

### Two native attachment defects exposed by the new gate

The local lifetime case passed first, but creating an owned Page and navigating
to its sandbox/OOPIF fixture terminated the session. The unchanged NODE-050
selected addon (SHA-256 `375c27ea32fe97e347533ad1279f28bb113c1cdc4ac39ff36fc21947157f2db7`)
also reproduced the failure through public `browser.newContext()`,
`context.newPage()` and `page.goto()`. This established a pre-existing path,
not a failure attributable only to the new metadata index.

1. An attachment announced by a parent outside the current route table was
   released with browser-root `Target.detachFromTarget`. Chrome rejected that
   session address. The new explicit CDP port resumes the announced child and
   detaches it through its exact announcing parent envelope, without constructing
   a fake target binding, opening another connection or ignoring errors.
   Upstream `crConnection.ts::CRSession.detach` performs the same resume-before-
   parent-detach sequence because Chromium does not reliably resume on detach.
2. An accepted, debugger-paused OOPIF could await `Page.createIsolatedWorld`
   before receiving `Runtime.runIfWaitingForDebugger`. That deadlocked the
   initializer until the original request timeout. The already-opted-in parallel
   recursive attachment profile now joins world initialization with resume,
   **after** settings, script registration and recursive attachment prerequisites.
   It consumes two of its existing four command slots, adds no task and does
   not enlarge deadlines or queues. Legacy serial profiles are unchanged.

The initial scoped-detach-only change replaced the session error with a
navigation timeout. Adding child resume exposed the second initializer ordering
bug. All intermediate failures remain in the evidence directory; none were
normalized into passing metadata observations.

### Evidence and remaining work

Directory: `.work/node052-frame-projection/`.

- `oracle.json` / `oracle.log`: fresh pinned Playwright 1.58.2 local and sandbox
  observations on Chrome 152.0.7977.76.
- `native-resume-worlds.log` and `native-final-repeat.log`: both Rust host lifetime cases match the stable
  oracle snapshots. Child title routes through the real Rust Frame service;
  native target inspection proves the sandbox OOPIF boundary. Owned Context,
  Page and OOPIF target IDs disappear after cleanup. Host runtime owners and URL
  entries/revisions return to zero; only the fixed empty index remains.
- `old-addon-oopif.log`: unchanged NODE-050 binary fails public owned-OOPIF goto.
  `new-addon-oopif.log`: rebuilt selected addon succeeds, with SHA-256
  `7f6f11a73cada576e5ddcaa96edc7116fa44aa667d012bf8dfe01d3491e0606c`.
  The default binary is not replaced.
- Ten new projection/reservation tests, a real serialized child/parent CDP
  envelope test and two driver regressions cover the new boundaries. CDP tests:
  **74 passed**. Composition tests: **312 passed**, 15 native tests ignored.
- All-target release Clippy for addon/host/composition/CDP/fake passes with
  `-D warnings`. Selected host unit tests: **155 passed**, nine native tests
  ignored; default host: **153 passed**, seven native tests ignored. The new
  native gate was run separately twice as recorded above.
- Actual selected Node input regressions: **Mouse 44/44**, **Keyboard 24/24**,
  **drag/Keyboard/Locator 10/10**. Reports `mouse.json`, `keyboard.json` and
  `drag-keyboard.json` record the new artifact hash, native ownership checks
  and zero host runtime owners after close. Node contracts: **38 passed**;
  strict types and dependency/import boundaries pass (host 234/235, addon 249/250).

Next: pre-admitted N-API metadata conversion and stable public JS Frame wrappers;
then run the same public lifetime actions against the actual addon. Frame names,
lookup/events, Page collection ordering across nested dynamic Frames and races
where a child disappears during initialization/release remain explicit gates.
No full compatibility, universal race tolerance or RSS improvement is claimed.

## NODE-051: metadata kernel and shared publication fence

Historical NODE-051 status: the Rust metadata kernel and shared URL/tree publication endpoint are
implemented and unit-tested. The host lifecycle driver does **not** yet install
or update Page trees, and the Node addon/facade does **not** expose child-Frame
enumeration. This checkpoint is not public Frame conformance or goal completion.

### Upstream observations

`packages/pwright/scripts/probe-frame-identity.mjs` probes pinned Playwright
1.58.2 against the dedicated Chrome 152 CDP process. Both local and sandboxed
child fixtures establish these public behaviors:

| Operation | Page collection | Retained child metadata |
| --- | --- | --- |
| Initial child | Main Frame plus child; stable object identity | Same Page and parent, not detached |
| iframe removal after `framedetached` | Child absent; main has no children | Detached, but original parent and URL retained |
| Replacement iframe | Different child Frame identity | Old child remains detached |
| Page close after replacement | Last two-Frame collection retained | Main/replacement remain not detached; parent and Page retained |

The immediate snapshot after `page.evaluate(() => iframe.remove())` is not a
detach barrier. In the recorded sandbox run, it still showed two Frames and
`isDetached() === false`; the subsequent `framedetached` observation showed one
Frame and `true`. The probe registers the event wait before removing the iframe.
Do not normalize away this publication boundary or misclassify it as a discrepancy.
The sandbox fixture's process boundary was verified separately by NODE-050;
this public-identity probe does not independently inspect renderer sessions.

### Rust ownership and publication

`pwright_node_host::frame_tree` is an explicitly constructed, caller-driven
metadata component. It imports no browser, transport, runtime or task owner.

- `FrameTreeFactory` owns a resource ledger with caller-supplied limits. Clones
  share exact budget identity; foreign-budget roots/children are rejected.
- `FrameNode` retains portable identity, immutable parent ID, the existing URL
  snapshot, detach state and its own storage leases. It has no strong parent,
  Page, tree, HostSession or CDP reference. A root can back only one Page tree.
- `FrameTree` holds native and published immutable revisions. Staging validates
  a single root, bounded count, present/acyclic ancestry, unique identities and
  no rebinding of retained IDs before admitting the next revision.
- Publication tokens capture an exact revision. Older tokens cannot regress a
  newer published collection. Pending retirements carry forward even if an
  attachment/removal token was dropped or delivered out of order.
- `close()` stops staging without marking Frames detached. Revisions staged
  before close can still publish. A retained Frame keeps its URL and parent ID
  after tree drop; all node admission releases at its last reference.

Portable IDs must come from the runtime's non-reusing identity source. This
component is not an ID allocator and intentionally has no unbounded historical
ID registry. The caller must reuse the exact node for a live portable identity.

`FramePublications::stage_tree` adds a revision to the existing publication queue,
alongside URL changes. It reserves the shared queue item before mutating the native
tree. The same fixed watermark then publishes both kinds in order; a captured
fence cannot publish later updates. No second watcher, task or event cursor is
introduced. Environment discard abandons queued tokens without advancing visible
state and rejects later tree staging. `pending_publications` counts both kinds
when a caller explicitly stages trees; existing host usage still queues only URLs.

This establishes a common delivery mechanism, not an atomic multi-event topology
transaction. Future lifecycle integration must enqueue URL and ownership changes
in the correct order and preserve Page-close versus Frame-detach semantics.

### Resource contract

Scope `extension(121,2)` separately admits:

- combined retained Page-tree and Frame-node states (`LiveObject`);
- immutable revisions (`EventItem`), including native, published and queued/held
  references; shared Arcs charge once, and last-reference drop releases admission;
- storage bytes: fixed state/revision layouts plus two Arc counters, plus each
  frame/retirement array element. URL allocations remain in the existing URL book.

Each state/revision uses two leases; metadata capacity is exactly
`2 * (states + revisions)`. A two-pass borrowed retirement iterator counts and
admits the final array before copying, with no full-capacity scratch allocation
or extra scratch lease. Replacement overlap must fit both count and byte limits.
Queue admission is separately bounded by scope `extension(121,1)`.

`FrameTree::frames()` returns a caller-owned vector: its allocation and foreign
conversion need separate admission at the eventual FFI boundary. The factory
limits are logical storage bounds, not an allocator/RSS measurement. No production
HostSession tree limits or new installed capability are claimed at this checkpoint.

### Verification

Evidence directory: `.work/node051-frame-api.COOmxt/` (local, not committed).

- `oracle-settled.json` / `.log`: local and sandbox public identity observations
  with a detach-event barrier. Earlier `oracle-final.json` records why the
  immediate post-evaluate snapshot is not that barrier. Failed endpoint/service
  attempts remain in their original logs rather than being replaced by success.
- 21 tree tests plus six mixed-publication tests cover delayed/old tokens,
  carried retirements, subtree removal, close, identity/budget isolation, queue/
  revision/byte exhaustion rollback, exact-capacity operation and last-reference
  release. The initial byte test incorrectly consumed its test-held child and
  was corrected to retain that reference while asserting unchanged admission.
- Default release host: **143 passed, six native tests ignored**. Selected
  `drag-runtime` release host: **145 passed, eight native tests ignored**.
- Selected all-target release Clippy with `-D warnings`: passed.
- Node contracts: **38 passed**; strict TypeScript and dependency/import boundary
  checks passed. Closures remain host **234/235**, addon **249/250** packages.
- Neither addon was rebuilt. Default SHA-256 remains
  `8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`;
  selected remains
  `375c27ea32fe97e347533ad1279f28bb113c1cdc4ac39ff36fc21947157f2db7`.
  NODE-050 public input parity is prior evidence, not a rerun for this checkpoint.

### Remaining integration gates

1. Project committed Frame/Page/parent topology after URL observations through
   an optional borrowed callback, including new Page initialization, OOPIF
   replacement, actual detach and Page/Context/transport closure.
2. Admit the host tree factory and active-index storage; keep retained Page views
   usable without retaining HostSession. Define teardown of unpublished revisions.
3. Pass Page-tree/Frame-node snapshots through N-API with pre-conversion admission.
   Preserve JS wrapper identity without an unbounded strong retired-Frame map.
4. Expose `page.frames()`, `frame.parentFrame()`, `frame.childFrames()` and
   `frame.isDetached()` using published metadata. Names, lookup and Frame events
   need their own correctly sourced metadata/contracts, not hardcoded values.
5. Run the same public Node lifetime cases against the actual rebuilt artifact,
   plus child/OOPIF command and native owner-removal regressions. Record the exact
   binary hash, owned-target retirement and post-close resource metrics.
