<!-- agent-updated: 2026-09-17T06:49:16Z -->

## NODE-166: confirmed Page close preserves retained-world retirement proof

`ChromiumPageCloseCommandService` now records
`Destruction::PageTarget(exact_target)` through its existing prepared route
transaction after native confirmation succeeds and before publishing the
replacement topology/routes. The selected registry matches the frozen owning
Page target and connection epoch, so retained Handle/world snapshots keep the
same native destruction fact even after their routes disappear. Other Pages'
receipts remain unchanged; profiles without this registry install no new owner.

This promotes an existing confirmation, not a command acknowledgement: ordinary
bounded polling must still confirm exact target absence, while the selected
native-confirmation policy must still receive its exact destruction evidence.
A still-present target, failed close, pending confirmation or inspector detach
alone cannot mark the receipt. Before-unload initiation, event publication,
command deadlines and unrelated close policies are unchanged.

The source audit found that the post-close topology snapshot only drains
currently queued events. It cannot guarantee a later `targetDestroyed` has been
consumed before Handle-enabled session shutdown tries to detach retained owned
sessions. Recording already-confirmed Page destruction at the close transaction
avoids an unnecessary detach attempt on that destroyed session. It does not
suppress failed detach or generic cleanup errors, authorize disposal of borrowed
Pages, or solve the separate compensation-versus-detach failure-identity gap.

Deterministic composition cases retain a world snapshot while withholding all
destruction events, gate absence confirmation after command acknowledgement,
check failed/still-present targets, preserve a second Page, and exercise owned
session retirement after confirmed Page closure. These are transaction/receipt
tests, not an attribution of NODE-165's first unclassified live timeout or a
claim of complete shutdown reliability. Exact executed results and remaining
gates live in [the canonical Handle checkpoint](pwright-node-handles.md).
The independently owned property-batch cleanup contract is recorded in
[Handle lifecycle](pwright-handle-lifecycle.md).

NODE-133: Actual Page target destruction now marks selected paired/utility world receipts using their frozen owning-Page target, including after local routes disappear. This does not turn an acknowledged close command or generic detach into destruction proof, publish close events or resolve the queued ancestor-dispatch discrepancy.
[Native Page world-retirement contract and evidence](pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-130: A selected deletion-only route transaction validates the survivor view, removes exact disposed Page/Frame keys, revokes only removed navigation entries and keeps unrelated provider identities. Its preparation is reversible; it is local routing only, not native-death proof or public close completion.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Optional native scheduling replaces an acknowledged destruction wait only with exact successful ancestor evidence. A still-written child command must finish first. Route validation runs before initial native polling and again after discovery before Target.closeTarget initiation; the ordinary Node service is unchanged.
[Bounded native scheduler contract and evidence](pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: The new opt-in Context authority can certify exact preexisting Page ancestry after successful native Context disposal, including parent-first topology retirement. Different sources, epochs, ancestors, rebound routes and later bindings are rejected. This does not replace Page's written-command ownership or wire Node concurrent scheduling.
[Native ancestor authority contract and evidence](pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Adds an explicit prepare_command path returning command acknowledgement separately from exact destruction waiting. Both futures are prebuilt within existing bounds; an early destruction event cannot skip the command reply. This enables later ancestor composition without abandoning written commands, but Node still uses ordinary native confirmation.
[Native close phase contract and evidence](pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred Page publication after an earlier Context publication produces no duplicate notice, while Page-first preparation lets the Context retire remaining descendants. These plans do not prove native Page destruction or ancestor success; driver integration is still pending.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Runtime latest-state disposal reservation now supports either parent/child storage commit order without duplicate child control messages. Existing descendant cancellation and transferred lifetime ownership remain intact. This does not authorize native Page-close success or replace exact destruction/ancestor receipts, and is not wired into host scheduling. NODE-123 browser dispatch remains unresolved.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-123: Pending Page.close followed by Context.close exposes an ancestor-dispatch gap: FFI admits both, but the host and exclusive transactions block native Context disposal. Terminal settlement preserves queued Context errors and original emitters; reply-producer/request-queue loss now seals admission before Page success can use a pre-terminal fence. Final profiles each pass 50 close-related cases plus two extra six-case confirmation repeats. The separate ancestor-dispatch probe remains red; native Page confirmation was not weakened.
[Close cascade contracts and evidence](pwright-close-cascade.md).

## NODE-122: Selected native destruction confirmation

The Node Events composition now selects `NativePageCloseConfirmation` through
the additive `CdpPageCloseConfirmation` / `CdpPreparedPageClose` contracts and
`ConfirmedAttachedPageCloseService`. Ordinary/v1 bounded polling remains the
default outside this selection. This supersedes NODE-121's implementation-pending
status, not its unproven attribution of historical natural failures.

Admission precedes mutation: validate the exact target, claim one live pending
slot, reserve call storage and subscribe to the same connection's reclaiming
fanout before enabling discovery or sending `Target.closeTarget`. The first
selected close enables page-only `Target.setDiscoverTargets` on that connection.
Its bounded command response still has the existing CDP timeout. The independent
event cursor consumes an exact, unscoped `Target.targetDestroyed`; a successful
command acknowledgement, inspector detach, unrelated destruction or repeated
`Target.getTargets` absence is not its confirmation. No 20-check receipt deadline
or unbounded event journal is introduced. Discovery lasts until connection
retirement; the ordinary topology driver does not turn these discovery notices
into premature disposal commits.

Prepared topology/runtime disposal and observer reservations survive the wait.
Actual confirmation precedes commit and close-event staging. Event lag or a
malformed destruction receipt fails; source/transport loss and owner cancellation
retire the failed operation without manufacturing native success. The selected
continuation retirement policy accepts terminal ownership loss as cleaned up;
the default continuation still requires an actual successful result. A written
close command remains polled through its bounded response before cancellation
can interrupt its subsequent receipt wait.

The new held-receipt test exposed a separate terminal-settlement race: host
stop previously returned ordinary Page.close success before terminal events
were staged, allowing the existing JS close facade to drop its emitter identity.
Rust now waits for `disconnected` on interrupted Page.close **after** the admitted
operation token has left. This avoids teardown's operation-tracker deadlock and
ensures the FFI fence includes terminal publication before successful settlement.
JavaScript gains no close policy, native wait, transport or driver.

Resource contract: one selected pending close, target identity at most 128 bytes,
4 KiB source allowance plus measured selecting-wrapper/Arc bodies, and 16 KiB
per prepared call (including a 1 KiB bounded injected interruption future and
4 KiB diagnostic allowance). One additional reclaiming subscriber is derived
into the connection plan. An explicit atomic slot cap prevents unused shared
ledger allowance from admitting extra concurrent closes. All reservations are
RAII-owned, including admission failures and abandoned continuations.

The measured host declaration is **24,840 bytes**: 4096 source + 4360 selecting
wrapper/Arc bodies + 16,384 call bytes, excluding the separately derived CDP
fanout storage. These are admission allowances, not process-RSS estimates.
Native command diagnostics format directly into two independent 1024-byte
UTF-8 buffers, preserving failure classification; resource diagnostics are also
bounded before copying. There is no unbounded temporary `error.to_string()` in
the selected path. Ordinary/v1 formatting is unchanged.

Verification evidence is retained under `.work/node122-native-confirmation/`:

- Backend: 21 selected unit tests, including the six unchanged bounded-polling
  tests; admission, exact identity, cancellation, lag, malformed/scoped/unrelated
  receipts and installation accounting. Composition tests prove pre-mutation
  rejection, rollback, retained cleanup ownership and terminal-loss retirement.
  Four final tests cover bounded formatting, UTF-8, fragmented writes and error
  classification. The ordinary feature-cold backend also passes its six tests.
- Both final Node artifacts: four held-receipt/Worker tests, lifecycle events 7/7, listeners
  18/18, Page.isClosed 8/8, terminal dispatcher 4/4, Browser connection 5/5 and
  collection lifecycle 1/1, plus driver-close attribution 1/1: **48 per profile**.
  Final logs use `<lane>-bounded-default.log` and `<lane>-bounded-combined.log`.
  A 300 ms diagnostic observation window is not a
  production deadline. Both settlement state and original emitter identity are
  asserted against pinned Playwright 1.58.2 in the three paired close scenarios.
- Rust logic gates: host default 397 passed/eight ignored; combined host 445
  passed/ten ignored; composition 371 passed/16 ignored. These ran before the
  final diagnostic-only leaf formatting patch; its 21-test leaf gate and the
  final native browser matrix were rerun afterward. Strict final all-target
  Clippy (`-D warnings`), fmt and diff checks pass.
- Node units 78/78, types and all seven host/eight addon dependency selections
  pass again on final source. Runtime JS still contains no driver, CDP transport
  or process launcher imports.
- Additional Frame-navigation corpus: **27/29**, not a green gate. Candidate
  arrival-boundary close/Context cleanup and the upstream arrival-boundary
  close observation still time out. The document-execution close case passes.
  See [the unresolved NODE-077 scenario](pwright-frame-navigation.md#node-122-close-confirmation-regression-checkpoint).
- Earlier failed evidence remains: fixed 1024-byte wrapper admission was too
  small and was replaced by measured ownership; owner/peer terminal events were
  initially lost; a test proxy incorrectly retained unrelated setup commands;
  the Worker initially checked destruction before waiting for that exact receipt.
  These are distinct causes, not evidence of a historical natural-timeout fix.

Final local artifact SHA-256:

| Profile | SHA-256 |
| --- | --- |
| default | `00e785606f3b6cd8277196a489812f64867649c8da49c4eecd20fb71bb3f55de` |
| drag-frame-extra-headers | `b0407dd5920b195bcfad11daab11338e30187efd848592b3f01ca84191b3c8cd` |

Both temporary serve-lib routes were deregistered. Only the exact remaining
NODE-122 anchor was closed; shared Chrome/serve-lib were not restarted or stopped.
No new task-owned background server is left running.

No push or additional commit. Overall Node compatibility remains in progress;
this selected Page-close policy does not establish complete Playwright parity.

## NODE-121: Native close confirmation plan

Status: **next implementation; not yet selected or implemented**. Pinned
Playwright 1.58.2 waits for actual Page closure, rather than declaring failure
after a fixed number of absence checks. Rust already has an isolated
`CdpPageCloseService` and a transactional composition close path; preserve both
boundaries when adding an opt-in confirmation policy.

1. Keep ordinary/v1 bounded polling constructors and dependency closures
   unchanged. Add a separately selected native confirmation capability for the
   Node composition; do not globally replace a finite-attempt policy or inflate
   its default count.
2. Prepare/admit an exact target-identity confirmation owner **before**
   Target.closeTarget can be sent. Charge its receiver, retained identity and
   escaped future storage; subscription/capacity failure must precede mutation.
3. A successful close command response alone is not target disappearance.
   Confirm with an authoritative exact-target destruction receipt or a
   specifically designed authoritative absence check for the already-closed
   race. A detached inspector session alone is not enough: borrowed targets can
   outlive their inspector sessions.
4. Native receipt consumption must advance independently of the host topology
   loop. That loop currently awaits lifecycle service execution before its next
   receive/snapshot; waiting on a notification produced only by that same loop
   would deadlock. Use a bounded selected native source, not JS listeners.
5. Do not add an implicit overall 20-check deadline to ordinary Node Page.close.
   Native command response deadlines, caller/session cancellation and transport
   termination are separate contracts; preserve their explicit error paths.
6. Retain existing prepared topology/runtime disposal and close-observer
   reservations until confirmation succeeds. Commit actual disposal before
   publication. Failed/lost confirmation must not invent a successful close
   receipt or prematurely mark the Page closed.
7. Integrate cancellation with the existing Continuable/cleanup ownership:
   dropping a waiting public call must not abandon written native work, retain
   unbounded subscriptions or prevent Browser/Worker teardown. Prove admission
   closure, cleanup retirement and disconnected sequencing explicitly.
8. Define lag, malformed event, target identity mismatch, stream termination,
   already-closed target and command-error races. Resource bounds do not imply
   a fake semantic close deadline; a bounded live receiver can wait without
   growing a journal.
9. Verification gates: deterministic fake-source ordering/race tests; no
   mutation on admission failure; unchanged ordinary bounded-polling tests;
   selected dependency/resource proofs; cancellation/Worker/transport closure;
   actual Node Page/Context/Browser event corpus and paired pinned-oracle close
   behavior. Delay/absence confirmation tests must identify exactly which native
   evidence is controlled and must not claim that injected timing is the cause
   of NODE-118/119 natural failures.

The mutation cleanup timeout remains independently unresolved. The selected
policy must improve the pinned close contract even if no natural timeout occurs
during its tests; do not describe it as a proven fix for an unidentified cause.
[Original-scenario trace and pinned-source evidence](pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: The initial default close-reentry probe again exhausted native target-absence checks. Later complete listener corpora and eight diagnostic reentry rounds per profile passed. Preserve both observations: target-still-open and retained driver timeout are separate paths, and the production 20-check/10 ms selection is unchanged.
[Close attribution contract and evidence](pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: A queued ready notice can precede Page close even after native retirement. Planned masks retain the ready Page for ordered terminal closure while unready Pages stay hidden. Existing close admission/continuation contracts remain intact; NODE-118's intermittent native absence-check exhaustion remains a separate unresolved stability item.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Ready conversion can bind a retained root descriptor after native Page retirement, without consulting public membership or live routes. This supports queued ready-before-close delivery but does not itself select that source; existing explicit/external/terminal close publication remains unchanged.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: External Page disposal now reaches the existing Node Page wrapper through pre-admitted native groups. The Worker-confirmed real-Chrome case proves native closure while Node is blocked does not publish JS flags or listeners early; after unblocking its trace matches upstream and the sibling Page remains usable.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Successful explicit native Page.close now supplies its Page-close group through the selected observer. The callback sees that Page removed/closed before later sibling close publication, and reentrant close progresses. External Page disposal still awaits the committed driver-effect source.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Terminal native Page-close groups now emit on the existing Page wrapper after its Rust closed flag and membership commit. Original synchronous exceptions stop same-emitter listeners but not independent sibling groups. Ordinary explicit Page.close event production is still unjoined.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Playwright-aligned Page close design

NODE-114: Actual native session teardown can now supply independent Page-close groups using pre-admitted bodies. Ordinary explicit Page close still uses the earlier collection-only path; its prepared/committed observer is not joined to this producer yet.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: The Rust group primitive can publish a Page closed flag and membership removal at one event-loop commit. A synchronous listener throw skips only that event group, not sibling Page or Context close groups. Native producers and Node dispatch are still pending.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: The optional native observer reserves from the prepared close plan before CDP,
then receives actual effects only after confirmed absence and all local/network
retirement. Failure returns admission; a continued close keeps the same guard.
This is not a Page close callback or a replacement for foreign publication.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: Source audit confirms explicit Page close commits prepared topology directly
and discards its effect output. The new driver observer sees native detach
transactions but does not recover that explicit-close cause from later snapshot
refresh. Reserve from the native prepared close plan before adding callbacks.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-109 can publish a queued Page close under an exclusive Rust owner, but the
following collection revision can be a different queue change. Therefore no
Page close callback is emitted at this lower-level boundary. Event-visible
membership and close flags still need one native-cause transaction.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-107 requires a retiring Page to be hidden on its old view before rebasing
to a revision that omits it. Earlier reader-held views keep their members. This
primitive does not change closed flags or emit close callbacks; sequencing those
with the native causes still belongs to the future admitted step owner.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 adds an immutable single-revision visibility transition: one Page can
be hidden while siblings and earlier retained views preserve their membership.
This does not mutate the Page closed flag. The existing whole-fence path remains
in use; combining visibility, closed-state publication and close notification in
an admitted one-shot step is still required.
[Tests and remaining integration](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 reveals the next missing layer: Page close events, including external
native closure. In a two-Page Context close, the first callback sees one Page
remaining and the second sees none; Context membership remains until its own
close callback. Candidate final states match but notifications are absent.
The existing atomic whole-fence publisher needs a stepwise event boundary, not
emit calls appended to close methods.
[Reference states, red gate and design](pwright-node-lifecycle-events.md).

NODE-103 makes ordinary Page-close and collection removal one published Rust batch.
The watcher cannot deliver a close token before the complete membership revision;
uncached reads still use retained main-Frame/tree metadata after native retirement.
Removal consumes pre-admitted retirement capacity, including an empty Context with
no Page-close token. Both rebuilt profiles pass Page.isClosed 8/8, collection
publication 7/7 and the empty-Context case 1/1. Earlier red/unwired notes are historical.
[Implementation and remaining scope](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).

NODE-102 retains the main Frame/tree metadata in charged Rust collection
revisions after native Page index retirement. The storage/fence tests pass, but
ordinary Page-close and membership-removal producer ordering is not wired yet.
Keep that change atomic at publication; adding a later revision after an already
publishable close token would leave a race.
[Implementation and outstanding gate](pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 reproduces Page collection removal before published Page.isClosed
changes. Owned and Worker-confirmed external closure both fail the blocked-Node
collection gate; after publication the sibling/closed state matches upstream.
This does not undo NODE-099, but shows its scalar gate did not cover collections.
[Evidence and Rust integration requirements](pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 verifies the unchanged Page.isClosed gate at 8/8 on both rebuilt
profiles while fixing Browser.isConnected publication. Terminal Page fences make
Browser disconnect ready; only the Rust one-shot event port flips its public
flag immediately before disconnected emission. Page closure alone cannot
authorize Browser disconnect. The older next-audit note below is now historical.
[Contract and current artifacts](pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

## NODE-099: Rust-owned Page close publication

`Page.isClosed(): boolean` is now implemented through `NativeFrameTree.isClosed`
and the retained Rust `FrameTree`. The JavaScript facade only checks artifact
availability and maps errors; it stores no new lifecycle flag and sends no CDP
request. Older artifacts fail explicitly with `PWRIGHT_UNSUPPORTED` until rebuilt.
Native retirement and published closure are separate monotonic fields in the
existing, byte-admitted tree state. Crash remains a live Page state, and closure
does not manufacture Frame-detached notifications.

### One queue, with pre-admitted terminal notifications

Each admitted Page owns one `FrameClosePermit` in its existing ownership entry.
Explicit creation reserves the permit before `Target.createTarget`, alongside
the existing root/tree/binding reservations; discovery reserves before admitting
the Page tree. Failed reservation releases everything without Page activation.
The permit owns an existing metadata `EventItem` lease, not a task or subscriber.

`FramePublications::stage_close` transfers the permit and a `FrameClosePublication`
into the existing URL/tree queue. The close token retains the charged metadata
tree until delivery or discard; it never retains a HostSession, runtime, execution
world or browser transport. This deliberate retention replaces the old assumption
that dropping the last external tree reference releases it before a pending
close publication is consumed.

The invariant is **queued notifications + reserved Page-close permits <= 256**.
At the active 16-Page limit, at most 240 remaining notifications can be admitted.
A full ordinary queue can reject another Page/event, but cannot evict a Page's
already-reserved close. Closure itself needs no additional admission or allocation.
This is a finite host admission limit, not a Playwright unlimited-capacity claim.

The selected queue is allocated at startup with `try_reserve_exact(256)`, after
reserving its complete logical storage in the existing 8-MiB Frame ledger.
Selection rejects a previously allocated legacy queue, even if currently empty,
because its retained capacity could exceed the new explicit allowance. URL-only
callers do not select this preallocation or Page-close ownership.

The fixed charge includes `FrameTrees`, Page/Frame vectors and reconciliation
scratch, `256 * size_of::<FrameChange>()`, and the shared `Arc<Storage>` payload
plus control words. It measures **31,248 bytes and one state** on this Mac after
Page enumeration/GC. The guard is shared with the metadata publication endpoint,
so its lease cannot disappear while the queue allocation remains observable.
Per-Page permit storage is included in `size_of::<PageEntry>()`; retained tree
state uses its existing `size_of` admission. These are logical native budgets,
not RSS, allocator overhead or V8 memory estimates.

### Fixed fences, disconnect and environment destruction

Normal close uses the same strictly increasing sequence as URL/tree changes.
A previously captured fence cannot expose a later close; delivering an older
fence after a newer one cannot reopen the Page. Existing queued tree retirements
and URL changes still publish in order, preserving their Frame semantics.

The sequence `u64::MAX` is reserved exclusively for session teardown. Ordinary
publication stops before consuming that terminal value. `FrameTrees::clear`
closes all active native trees and queues their reserved notifications as one
mutex-serialized terminal batch, then drops the active index entries. Only after
this does `SessionTeardown` signal disconnect. Even sequence exhaustion or full
event admission leaves the terminal path available. The existing Node watch and
async-completion fences publish these tokens; no extra event loop is introduced.

Environment destruction first calls `discard`: queued tokens and permits are
released without executing foreign callbacks. Subsequent native teardown drops
remaining permits without queueing. A retained published view is not advanced
by discarded work. Repeated clear does not enqueue duplicate closure events.

The unchanged NODE-098 eight-case gate passes on both final rebuilt profiles,
and Page enumeration/reclamation passes 7/7 on both.
The original red baseline remains historical evidence, not current API status.
Public Page close/crash events, before-unload true and close reasons remain open.
Next audit the existing Browser.isConnected getter against the same Node
publication boundary; broad Node API compatibility and the full goal remain open.

### NODE-099 verification and cleanup

Final source and artifact checks are in
`.work/node099-page-close-publication/`:

| Gate | Default | Combined |
| --- | --- | --- |
| Rust host release unit suite (`host-*-verified.log`) | 238 pass, 7 ignored | 286 pass, 9 ignored |
| Unchanged Page.isClosed paired gate (`*-live-final.log`) | 8/8, 8.558 s | 8/8, 7.455 s |
| Page enumeration/GC (`pages-*-final.log/json`) | 7/7 | 7/7 |
| Existing Worker destruction subset (`workers-*.log`) | 3/3 | 3/3 |

The twelve added Rust cases cover publication, old fences, admission saturation,
terminal sequence exhaustion, discard, last-reference leases, close-only watch
delivery, foreign permits and startup queue selection. Existing Node units pass
47/47; strict boolean typing, formatting, syntax, all seven host/eight addon
dependency selections and the no-JS-driver boundary pass. Strict combined
host/addon all-target Clippy passes with no suppression (`clippy-verified.log`).
Ignored live Rust cases are not counted as passes.

Final native SHA-256:

- default: `5d279b3afcceaf19385c4f56f0562b59ed8a68b8ab9822629f1bf6648110b027`;
- combined: `f69732227e1c795cf0e2248c1e0bb0f9bc635e59e0ee0d99cf346b07a1af8d6f`.

Both Page enumeration JSON reports match their current binary hash and end with
zero host leases/objects/operations/cleanup, zero URL entries/bytes/publications,
and only the fixed 31,248-byte/one-state index/queue allowance. Other native
profiles were not rebuilt. These results do not establish all selector/input,
network, Page-event or concurrent-browser behavior.

The Worker subset is the existing borrowed-session teardown, exact owned
Context/Page destruction, and queued URL-publication discard tests. It is not a
new live queued-close-token test. Each selected case has a 30-second test bound;
both runs exited successfully with no native panic or environment-cleanup errors.
The static fixture used the already-running shared serve-lib daemon, through its
verified Tailscale control address, with a temporary 15-minute /pwright-node099
registration. The initial default-control refusal and old fixture URL 404 were
configuration/registration discovery, not an application failure. The exact new
MagicDNS URL returned 200 before use; registration was removed after tests and
then returned 404. No daemon was started or stopped.

The initial Rust frame-only run (under the prior
`.work/node098-page-is-closed/node099-host-initial.log`) passed 115 and failed
one old immediate-release assertion. The retained-close contract now requires
consuming the publication fence before expecting metadata release; that assertion
was corrected and both full suites pass. This is intentional charged retention,
not a waived leak. Final test-only Arrange/Act/Assert cleanup did not change the
production artifact hashes.

At 02:21:52 UTC all eight recorded relay ports had no listener, and exact
preflight target 5FF349BB4A32389229DD6435684FBFFA was closed. An unrelated XLQY
Page FC690BFDB9AFE712298FCC710090A3A9 was preserved. No task fixture/relay/Worker
remains. At 02:22:54 the shared backend RSS was 584,064 KiB (opening 2,336,432 KiB),
the existing Chrome RSS 1,056,416 KiB (opening 1,031,104 KiB), with swap zero.
These are process measurements, not this chat's RAM. Chrome and shared services
remain running; no process cleanup outside this task was performed.

Foundation HEAD remains f76e3f2. No changes were staged, committed or pushed.
The full Node FFI compatibility goal remains active.

## NODE-098: Page.isClosed reference contract and failing candidate baseline

This checkpoint adds characterization and a red differential gate, **not** a
production `Page.isClosed` implementation. The installed comparison oracle is
Playwright **1.58.2**; its packaged source, rather than a newer upstream checkout,
is the reference for these observations.

| Actual native scenario | Retained public Page state |
| --- | --- |
| Ordinary Page.close, including a second close | false immediately after starting close; true after completion |
| Context.close | false immediately; true after completion |
| Browser.close with an owned Context | false immediately; true after completion |
| Borrowing Browser.close | true after disconnect; controller's native Page still evaluates to 42 |
| Loss of only the borrowing CDP transport | true after disconnect; native Page still evaluates to 42 |
| Another connection closes the exact Page | eventually true without a subject-side Page command |
| Real Page renderer crash, followed by close | false after crash; true only after close |
| Worker confirms native close while subject JS is blocked | false before Node yields; true after publication |

Ordinary Page/Context/Browser closure preserves the main Frame object and does
not turn `mainFrame().isDetached()` true. Page closure therefore cannot be derived
from Frame-detach state. A borrowed native Page surviving disconnect does not
mean that its old connection-local Page wrapper remains open.

Pinned source anchors: `lib/client/page.js:78` initializes from the channel's
`isClosed`; `_onClose` at 175 sets `_closed`, `_onCrash` at 181 does not, and
`isClosed` at 520 reads the flag. `lib/server/chromium/crBrowser.js:194`
closes its connection-local Page representations on disconnect. These are
installed 1.58.2 file locations, not line claims about current upstream HEAD.

### Test mechanics and evidence

`packages/pwright/test/page-is-closed.test.mjs` and its controller Worker use
fresh, exact-owned Pages/Contexts in the existing dedicated Chrome. No browser
process is launched or killed. The transport case terminates only its own
loopback relay; the crash case targets only its fresh Page renderer. The Worker
uses one four-byte SharedArrayBuffer: parent starts the close, worker awaits
actual Page.close completion, then signals the blocked parent. This establishes
the reference event-loop observation without substituting a sleep for closure.
It does not independently prove when the candidate Rust topology driver has
consumed that close; deterministic host publication tests are still required.

Run `PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:page-is-closed` from the
Node package after the usual task-specific Pwright preflight. No HTTP fixture
is needed. `PWRIGHT_IS_CLOSED_REFERENCE=1` is explicitly labelled **reference
only**; it is not a candidate compatibility pass. The normal lane never silently
skips a missing candidate API and uses the existing 180-second/4-MiB supervisor.

Evidence under `.work/node098-page-is-closed/`:

- `oracle.log`: reference 8/8, 2.288 s.
- `oracle-supervised.log`: repeated supervised reference 8/8, 6.959 s.
- `baseline.log`: default candidate 0/8, 9.269 s; every failure is
  `page.isClosed is not a function`, after successful upstream characterization.
- `combined-baseline.log`: combined candidate 0/8, 13.194 s; same missing API.
- `unit.log`: existing Node units 47/47; strict types and dependency/no-JS-driver
  boundaries pass. Rust production files and both NODE-095 artifacts are unchanged.
- `final-worker.log`: final Worker/reference case passes 1/1 after endpoint
  validation and Context-setup cleanup hardening. Syntax and whitespace checks pass.

At 01:55:06 UTC all six recorded relay ports have no listener and the exact
Pwright preflight Page has been closed; Chrome's target inventory is empty. No
test Worker, fixture server or relay remains. The pre-existing dedicated Chrome
process remains running at RSS 1,039,952 KiB. Shared backend RSS fluctuated from
2,048,752 KiB at 01:45:10 to 6,687,664 KiB at 01:51:57 (physical footprint 6.2 GiB
at that sample), then 1,108,016 KiB at 01:55:06; swap stayed zero. This does not
establish per-chat RAM or a leak. The separately observed Rust compiler's cwd was
the XLQY server, not Pwright. No shared process was killed/restarted, and no Rust
build was started in this checkpoint. No changes were staged, committed or pushed.

### Implementation contract and next design gate

1. Keep lifecycle truth in the retained Rust Page-tree metadata, not a JS
   `_closed` flag, a Frame-detach flag, or a transport liveness query.
2. Separate native retirement from the JS-visible close publication. Current
   `FrameTree::close` and `close_for_teardown` only stop native updates; exposing
   that flag directly would bypass the established Node publication boundary.
3. Reuse the existing `FramePublications` fixed-watermark mechanism. A fence
   captured before close must not expose close even if delivered later. An old
   fence must not reopen state published by a newer fence. Close must not erase
   retained Frame metadata or already-staged Frame retirement publications.
4. Teardown must publish closure even when normal metadata-event admission is
   full; environment destruction instead discards pending publication without
   executing foreign code. `SessionTeardown` currently clears the tree index
   before cancelling its disconnect signal, so this needs an explicit owner.
5. Proposed bounded route: pre-admit a terminal close-publication permit for each
   Page before native Page creation, carry it in the existing Page ownership
   entry, and consume it through the existing publication queue on retirement.
   Prove queue storage, lease transfer, sequence exhaustion and no-allocation
   teardown before adopting this route; do not add an unbounded emergency queue.
6. Add Rust tests for old/new fences, repeated close, saturation, discard,
   teardown with retained Page/Frame bindings, and released storage. Then add
   the native synchronous getter, the thin Node method and strict boolean type,
   rebuild default/combined, and turn this unchanged red gate green.

The getter is a host metadata surface; it does not require installing a new CDP
domain or an always-on browser feature. Public Page close/crash events, reasons
and before-unload semantics remain separate unfinished work. The goal is active.

NODE-083 reuses the existing exact-Page close service for header initialization
compensation, preserving the original error after successful cleanup. Only the
failed newly created Page closes; Context and attachment remain alive. This does
not resolve the independent NODE-077 close-at-server-write investigation.
[Failure recovery](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-077 retains the public Frame.goto close-at-server-write failure and adds a
separate post-document-execution boundary. The latter passes on both profiles
against Playwright; the former stalls upstream and intermittently reports Rust
TargetStillOpen. Prior raw-CDP reload evidence already reproduces command
acknowledgement without target removal. No production close policy, retry,
confirmation count or stopLoading behavior changes in this checkpoint.
[Current close/cancellation evidence](pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

> Status: implemented experimental v2 checkpoint; it is not yet a stabilized
> compatibility promise.

## Node FFI integration checkpoint

The experimental Node facade exposes ordinary `Page.close()`, including
explicit `runBeforeUnload: false`, through this Rust capability. Repeated,
concurrent, externally completed and post-disconnect close suppress only
target-closed errors. The serial native lifecycle/topology driver reconciles
retirement before resolving; close has independent FFI admission from commands.
Its explicit profile uses at most 20 absence checks with injected 10 ms pauses.
The native CDP command timeout remains separately bounded.

Commands cancelled by authoritative Page/Context retirement now retain their
TargetClosed result without requesting another completion-publication fence.
Ten simultaneous Locator cancellations previously overflowed the eight-entry
driver queue and replaced a closed error with ResourceExhausted (NODE-023).
The shared host completion policy removes only this redundant terminal step;
the close owner still publishes retirement, and successful/nonterminal command
outcomes still require their normal fence. It adds no queue, task or capacity.
The rebuilt Locator lane passes 22/22, including the unchanged ten-read close
assertion; earlier failed runs remain in the Node evidence record.

NODE-099 adds synchronous `Page.isClosed()` through Rust-published metadata.
Before-unload true and close reasons are rejected before mutation. Public Page
close events are not exposed yet, so this is not complete
Playwright Page-close API or event-order parity. See
[NODE-012 and the lifecycle checkpoint](../node-ffi-compatibility-design.md#explicit-page-creation-and-ordinary-close-checkpoint).

## Upstream behavior being matched

The source baseline is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`. Its public
`Page.close({ runBeforeUnload, reason })` deliberately maps to two protocol
operations:

- ordinary close sends protocol `Page.close`, suppresses a racing
  target-closed error, and waits for the server Page's closed promise;
- `runBeforeUnload: true` sends protocol `Page.runBeforeUnload`, does not wait
  for Page closure, and does not suppress a racing target-closed error.

On Chromium, ordinary close uses browser-session `Target.closeTarget`; the
before-unload form uses page-session `Page.close`. The latter may leave the
Page open while a before-unload dialog awaits a user decision. Relevant
anchors are:

- `packages/playwright-core/src/client/page.ts` (`Page.close`);
- `packages/playwright-core/src/server/page.ts` (`close`, `runBeforeUnload`);
- `packages/playwright-core/src/server/chromium/crPage.ts` (`closePage`);
- `packages/playwright-core/src/server/chromium/crBrowser.ts` (`_closePage`);
- `packages/protocol/spec/page.yml` (`close`, `runBeforeUnload`).

## Capability boundary

Pwright adds a separate optional `pwright.page_lifecycle` capability. It
does not become part of `pwright-client`, navigation, locators, browser-context
lifecycle, CDP, or the runtime by default. Version 1.0 owns two typed commands:

1. `page.close`: ordinary close with termination semantics;
2. `page.run_before_unload`: initiation-only before-unload semantics.

Using two commands, as Playwright's own channel does, prevents one boolean from
hiding two incompatible completion contracts. Both target only Page and carry
no action timeout or cancellation metadata. The first slice exposes an
argument-free ordinary close and an explicit before-unload method. Playwright's
optional close `reason`, and a convenience options facade that selects between
the commands, are a later additive contract after close-reason propagation to
interrupted operations is designed end to end.

Ordinary close is idempotent at the client boundary: a locally stale Page or a
racing `TargetClosed` result is success. Before-unload initiation is not
idempotent in this way because reaching the live Page is itself the requested
effect.

## Chromium ownership and completion

Ordinary Chromium Page close is a four-owner transaction:

```text
prepare topology Page removal
  -> prepare runtime Page subtree disposal
  -> Target.closeTarget(exact target)
  -> confirm exact target is absent
  -> record selected exact-target world-retirement receipt
  -> commit topology and Page-session route table together
  -> commit runtime child-first disposal
  -> publish every Dispose through Page
  -> publish Result
```

`Target.closeTarget` acknowledgement alone is not termination proof. The CDP
adapter must confirm that the exact target no longer appears before local
commit. Confirmation/retry timing is injected and bounded; the core service
does not read a clock, sleep, spawn, or choose Tokio. Failure leaves both local
owners live so the caller can retry or reconcile.

The ordinary close future uses the same synchronously reserved, caller-owned
continuation rule as BrowserContext close. Once native work starts, dropping
the caller future transfers the exact transaction to that bounded owner.
Remote completion uses the same priority control barrier so Frame and Page
Dispose messages are applied by the client before `close().await` wakes.

Before-unload initiation is intentionally different:

```text
Page.close on the exact page session
  -> return initiation Result
  -> keep Page and descendants live
  -> later dialog/target events decide whether disposal occurs
```

It does not prepare or commit local disposal. A future dialog capability will
own accept/dismiss; dismissing before-unload may preserve the Page. This avoids
falsely making an initiated close look completed.

## Implemented composition

The capability, codec, client extension, and generated server handler are four
independent crates: `pwright-cap-page-lifecycle`,
`pwright-codec-page-lifecycle`, `pwright-client-page-lifecycle`, and
`pwright-server-page-lifecycle`. Selecting the capability installs both command
descriptors and the two native Chromium features; omitting it installs none of
them.

`ChromiumPageCloseCommandService` holds the browser-session client, shared
production topology, exact `CdpPageServices` route table, runtime lifecycle
handle, bounded confirmation count, and an injected polling policy. After
native absence is proven, it prepares the route-table replacement from the
topology candidate, records that exact target's destruction in any selected
retained-world registry, and publishes topology plus routes under one
synchronous lock window before committing runtime disposal. `Page.run_before_unload`
routes through the exact Page session and performs no owner mutation.

The attached factory constructs both services from one controlled Page
runtime. BrowserContext close has a target-aware factory as well, so closing a
context retires descendant Page routes in the same topology publication
window. Continuation admission is synchronous and caller-owned for ordinary
Page close, matching the BrowserContext close abandonment contract.

## Acceptance gates

- schema/codegen proves both commands are independently typed and Page-only;
- client tests prove ordinary-close idempotence and before-unload strictness;
- fake-CDP tests prove exact browser-session versus page-session methods;
- failed native close or failed absence confirmation rolls back both owners;
- future Drop before polling is lazy, and Drop after native start transfers to
  an already-reserved bounded continuation;
- remote frames dispose Frame before Page and before ordinary close Result;
- before-unload Result does not invalidate the Page;
- dedicated Chrome proves ordinary close removes only the selected Page and
  preserves its context and browser.

All listed gates have executable release-mode coverage. The live proof ran
against the dedicated Chrome 152 CDP profile and also re-ran the target-aware
BrowserContext close proof.
