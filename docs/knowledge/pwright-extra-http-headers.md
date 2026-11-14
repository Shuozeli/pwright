<!-- agent-updated: 2026-10-06T07:11:26Z -->

# Extra HTTP headers: Rust policy and native installation design

## NODE-369 complete official Context header file

The complete unchanged Playwright v1.58.0
`tests/library/browsercontext-set-extra-http-headers.spec.ts` file passes 2/2
candidate and 2/2 independent oracle. It proves a Context creation header plus
Page override reaches the fixture with case-insensitive replacement while the
other Context header survives, and that a non-string creation value retains the
public error. Candidate/oracle report SHA-256 values are
`c5694f1ef1c40cbb50b0b397bff35a45fdc70d678747cfd230de31f0149fb099`
and `a9cdcafcc44f8c8da97d7420f7f4e6a15a3cde000c8082707ccfc7757e31131f`.
No header implementation changed in this checkpoint; it is official coverage
of the Rust-owned NODE-084 creation/override path, not full header conformance.

NODE-097's controlled raw peer isolates a native Page-auto-attach interference
signature seen during shared-browser header churn. Both upstream and Rust owners
timeout and recover identically; the combined/default peer matrices pass 11/11.
Header implementation and the earlier failed uncoordinated stress log are unchanged.
This does not explain every historical header/Frame failure.
[Attribution and limits](pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 adds bounded command/network/event timing to the child-header test,
separates oracle fixture-observation failures from Rust failures, and recognizes
an already committed native iframe reattachment from its TargetInfo URL. Armed
operations retain their original session. Header production logic is unchanged;
churn overlap remains a failed gate despite a passing controlled peer shutdown
matrix. [Evidence](pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 preserves typed native errors through nonempty header initialization
and handles actual pending-call SessionDetached proof during fanout. Child
closure succeeds; main closure rejects TargetClosed; neither waits for the
topology observer before settling. Live protocol errors retain their existing
formatting. The reader's new retirement option is separately budgeted and
selected by the Node host. [Contract and evidence](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 follows up on the root-detach failure with a deterministic queued
disposed-Context reproducer and a selected Context-classification fix. Targeted
default/combined tests pass, but overlapping child-header navigation/disconnect
timeouts remain after Pwright builds finish.
[Current evidence](pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

## NODE-093 Typed header-initialization retirement

Overlapping public lanes reproduced two distinct failures on the NODE-092
artifact: header initialization failed with a real Network.enable missing-session
receipt, and Browser.close failed on a browser-root Target.detachFromTarget.
The bounded child-header relay now records method/envelope/detached-session
identity, native attachment type/context and oracle/candidate labels. Pending
commands and active attachments are separately capped at 128; attachment records
are removed on detach, and a command retains only its selected identity until
the response. This is test-only instrumentation, not production session history.

Network.enable in the header initializer previously converted CdpError into a
formatted HostError before the topology driver's exact typed retirement check.
Preserve the original CdpError in the existing boxed initialization error
boundary instead. The driver can now discard precisely the vanished session.
No error is blanket-ignored, no command retried, and no new production task,
queue, subscription, resource owner, feature or dependency is introduced.
Ordinary header setters and their public error formatting are unchanged.

The attachment gate now selects a command occurrence for one exact native Page
session. The combined profile enables Network for lifecycle events and then
again for headers; the old first-occurrence test missed the header boundary.
Pinned Playwright 1.58.2 uses one shared network manager, so the paired oracle
holds its first enable while Rust holds its header enable. Both receive Chrome's
actual -32001 / Session with given id not found. after the controller closes
only its own Page. Baseline: 7/8, with the new second-occurrence case failing only
in Rust. Rebuilt combined: 8/8. Headers-only, with no Frame-navigation dependency:
6/6. Each successful candidate disconnect checks zero host/cleanup/FFI owners.

Evidence: `.work/node093-disconnect-attribution/`.
Combined SHA-256:
`520687ebece460d1a7bd90cd597e194cdffe549351e312e8b5ef5a4dcb971426`.
Headers-only SHA-256:
`f444bdf27ce4728090879cdca2bce2dd9f5763caf1b1dc77307a06daabe8eeee`.
Default and other optional binaries were not rebuilt in this checkpoint.

Serial child-header regression passes 8/8; host release tests pass 274
(9 existing ignored), Node unit tests 47/47, strict types and all 7 host / 8
addon dependency selections pass. Selected host all-target Clippy with warnings
denied passes. The post-fix overlapping run still fails: child headers 5/8
(one disconnect timeout and two initialization timeouts), headers 34/35
(one initialization timeout). No root -32602 appeared in that run, which does
not prove its separate path fixed. All five selected composition retirement tests
pass, including wrong-code/message, timeout, transport, Compound and untyped
lookalike rejection controls. This is a filtered test run (350 tests filtered),
not a claim to have rerun the complete composition suite. The release build took
3m 40s; every recorded build/test handle is terminal. Formatting, Node syntax
and git whitespace checks pass. Foundation commit f76e3f2 remains unchanged;
no staging, new commit or push was performed.

All browser test processes are terminal. The task's preflight target
BC4D15EEA0C99AB33323180EEFD6DBDD was closed, Chrome's target list is empty, and
none of the 113 recorded diagnostic/fixture ports has a listener. No shared
browser or game server was restarted or killed. The Rust build uses one job;
the composition-test compiler reached 6,975,904 KiB RSS at 00:08:43Z. This is
an identified build process, distinct from the shared backend's 3,499,248 KiB
RSS at that time. At 00:05Z the shared backend footprint was 1.5 GiB and swap
was zero; its historical peak is not per-chat attribution. The compiler's
reported physical-footprint peak was 8.1 GiB, falling to 1.4 GiB before exit.
No task-owned background process remains after verification.

Still open: root-detach ownership/late-Page ordering and disconnect timeouts,
header initialization's nonempty Network.setExtraHTTPHeaders typed error path,
blocked-driver response/event ordering, Worker shutdown and full API parity.
The pre-fix overlap failed 6/8 child cases and 33/35 header cases; the attachment
baseline also overlapped the tail of that header run. The targeted failure
includes a native receipt for the exact owned Page, not a guessed string.

NODE-092 reproduces one source of -32602 attachment errors: a second-stage child
cleanup race, and fixes that native path. Serial child-header tests pass 8/8,
but overlapping lanes still yield 4/8 child-header cases versus 35/35 headers,
with connection/disconnect failures. Historical failing logs remain preserved.
[Reproduction and exclusions](pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

## NODE-091 Native session closure during header fanout

Pinned Playwright 1.58.2 `CRNetworkManager._forEachSession` ignores only
closed/crashed child-session errors. `CRSession.dispose` rejects pending
callbacks on closure; it does not wait for a later command response. Main-session
closure still rejects. A native-looking `-32001` error on a live session is not
a closure signal. The baseline public comparison reproduced two failures:
Page/Context setters rejected a removed OOPIF and rolled the parent back, while
Playwright fulfilled and retained the proposed headers. Four live-child error
controls already agreed.

Rust now creates bounded request-scoped native session watches during synchronous
route capture. The existing ordered topology observer feeds actual detach/crash
events into these watches. Closing a main session also closes watches for its
children. Each watch has separate primary/compensation waker slots; every waiter
clears its slot on drop. A closed child settles successfully before a held native
command response; a closed main returns TargetClosed. Ordinary protocol errors,
including a missing-session lookalike without closure, still propagate unchanged.
Header merge, deferred Page snapshots and compensating metadata policy stay Rust
owned. No native error-message matching was added to the setter.

The optional host observer composes with, and delegates every callback to, the
existing selected Frame lifecycle observer. Headers alone return false from
`observes_page`: they do not implicitly enable lifecycle domains or select the
Frame-navigation feature. No second event cursor, runtime task, Node automation
driver, production dependency or new public header API is introduced.

Resource ownership: at most nine admitted requests times 80 routes = 720 weak
watch slots. A watch owns three native IDs (each <=256 bytes), one close flag,
two AtomicWakers, and Arc bookkeeping. Primary and compensation plans share the
same watch; their route vectors remain separate. Storage is checked inside the
existing 128-KiB plan / 992-KiB request allowance; the installation additionally
reserves the fixed weak-slot vector. Dead entries are pruned before admission,
so completed requests do not accumulate session history. This is a reservation
proof, not a resident-memory estimate. Closing a native session justifies retiring
its pending command; arbitrary shutdown/cancellation still is not a native ACK.

The new `test:header-child-sessions` lane compares real public APIs using the
existing test-only loopback CDP relay and a dynamic Tailscale-bound HTTP fixture.
A sandboxed `allow-scripts` iframe without same-origin permission is verified as
an actual native iframe session. Chromium initially announces an empty URL, so
the gate correlates its subsequent Page.frameNavigated URL on that same session.
The fixture supplies CORS/preflight headers for sandboxed wire-header inspection.
One owned command is held; native closure must settle the setter before release.
After release, Chrome must independently return its actual missing-session error.
Controls inject ordinary and missing-session-lookalike errors into a live child.
Replacement-child inheritance, main-session rejection, parent wire state and a
fresh usable Page are also checked. Candidate disconnect asserts zero active
leases, cleanup, FFI pending entries and reserved FFI bytes.

Evidence directory: `.work/node091-header-child-session/`. The initial fixture
attempts failed on empty attachment URLs and missing CORS preflight permissions;
these are retained separately from the reproduced Rust mismatch in
`baseline.log` (4/6). The first resource test incorrectly counted shared Watch
allocations twice; corrected accounting retains both independent route vectors.
The first main-close comparison called the not-yet-supported Page.isClosed API;
the header-specific check now uses public Context Page enumeration. Page.isClosed
itself remains an API coverage gap, not a claimed implementation.

Final terminal verification:

- `live-serial.log`: 8/8 combined-profile public cases, 22.368 seconds;
  `repeat-combined.log`: another 8/8, 15.931 seconds.
- `live-headers-only.log`: 8/8 on the separately built `extra-headers` artifact,
  14.905 seconds. No drag or Frame-navigation selection is needed for this lane.
- `headers-regression.log`: 35/35 combined public header cases;
  `attachment-regression.log`: 7/7 combined attachment-retirement cases.
- `host-final.log`: 274 release host tests pass, 9 existing live tests ignored.
  Five session-watch tests cover scoped closure, two independent task wakeups,
  clearing the registered waker on drop, admission and capacity reuse.
- `clippy-final.log`: selected host all-target Clippy with warnings denied passes.
  Default and headers-only addon release checks also pass. Node 47 tests, strict
  types and all 7 host / 8 addon dependency boundary selections pass. Formatting,
  Node syntax and git whitespace checks pass.

Combined native SHA-256:
`1d159de7a06891fb04892b5de31eb2cb3f96b3c40d60b9a9354a41e5140901c7`.
Headers-only native SHA-256:
`41599e5603109750081efe3e3fef5e5dfd4070a263842b1b7b2605de1e8f9079`.
The later Rust-only test strengthening does not change production behavior.

Preserved failing concurrency evidence: `live-verified.log` passed 6/8 while
overlapping the header regression. One case failed disconnect with `Timeout
waiting for response`; another failed connection initialization with native
`-32602: No session with given id`. Serial success does not fix or disprove these
attachment/cleanup races. The main-close first attempt in `live-main.log` instead
failed only because this package does not yet implement Page.isClosed.

Cleanup: all 148 recorded test fixture/relay ports have no listeners. The exact
owned preflight target `01AFFD90D2B81B3CC2EDAD7715FEAFC7` was closed; the remaining
XLQY Page `4C5FBEFD6A0ABB83847C2DB825825AAE` was preserved. The first inventory from
the repo root used an old cached WebSocket URL and returned 404; the task-owned
state directory succeeded, so no shared Chrome restart was needed. All test/build
handles are terminal. No task-owned server/worker remains, and no shared server
was stopped. At 14:29:08Z the shared backend PID 53189 had RSS 1,421,376 KiB
(down from 2,872,800 KiB at 14:12:01Z); swap remained zero. This is process-level
RSS, not this chat's RAM. Foundation `f76e3f2` remains local; no push/new commit.

Run from `packages/pwright` after explicitly building the selected artifact:

```sh
PWRIGHT_NATIVE_PROFILE=drag-frame-extra-headers CARGO_BUILD_JOBS=2 pnpm build
PWRIGHT_NATIVE_PROFILE=drag-frame-extra-headers \
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_TEST_BIND=100.78.97.63 \
PWRIGHT_TEST_BASE=http://macbookpro-yuacx.tail8f3b66.ts.net \
pnpm test:header-child-sessions
```

The bind/hostname above were verified on this Mac; resolve them again elsewhere.
Remaining fidelity work includes the observed concurrent attachment/cleanup races,
response-versus-detach ordering under a blocked
topology transaction, crashed/replaced-session native comparisons, Worker/host
shutdown and native-waker retirement, header-only attachment failures whose native
error is currently wrapped by the initializer, Page.isClosed, and full Node /
cross-browser compatibility. These tests do not establish those broader claims.

NODE-090 extracts the test-only loopback relay for shared header and native
attachment-retirement tests. Header regression passes 35/35 on the rebuilt
combined artifact; host tests pass 269. The attachment fix is Rust-owned and
does not change setter ordering, desired-header policy or compensation budgets.
[Attachment evidence and exclusions](pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

## NODE-089 Native receipt gates and Context Page sequencing

This checkpoint corrects NODE-088's assumption that a Context starts every Page
concurrently. Pinned Playwright 1.58.2 `chromium/crBrowser.js:420` awaits each
Page's `updateExtraHTTPHeaders` in order. Only a Page's CRNetworkManager fans out
concurrently across its native sessions. Both primary and compensation must
preserve these two levels. The Rust Fanout now has bounded sequential groups:
all sessions in the current Page start eagerly, later Pages start only after
that Page succeeds, and failure drains started siblings without starting later
Pages. A retained failed job still permits a newer setter to progress.

Freezing every Page's values was also incorrect. Context's first Page reads
headers at synchronous call preparation; later Pages read current desired
headers when their turn starts. The dispatch plan now freezes only the first
Page and records later Page identities. Each Page group has one shared snapshot
cell, materialized once and cloned across its sessions. After a held first-Page
compensation ACK, the next Page observes a newer setter instead of replaying
old headers. No lock crosses a native await. Frozen route identities remain;
route handoff/retirement equivalence still needs separate work.

Native `Network.setExtraHTTPHeaders` protocol errors now retain the method name
in the public first line, matching upstream's `Protocol error (...)` instead of
exposing the raw `CDP error -32000` representation. Non-protocol failures retain
their prior classifications. The existing UTF-8-safe 4096-byte diagnostic bound
still applies; call-log/stack equivalence is not established.

### Controlled public verification, not a simulated setter oracle

`packages/pwright/test/header-command-gate.mjs` is test-only transport plumbing.
It connects only to the dedicated `127.0.0.1:9222` Chrome and exposes one
ephemeral **loopback-only** relay. It never proxies CDP to Tailscale/LAN. It uses
the pinned oracle's already-installed WebSocket implementation, not an added
runtime dependency. A marker is armed only after a fresh API-owned Context/Page
is ready; unrelated targets are not modified. It retains at most eight gated
receipts and 32 marker-command observations, with 4-MiB WebSocket message caps.
Relay/listener/socket teardown is scoped to the test. Dynamic HTTP wire fixtures
still bind to the explicit Tailscale IP and use the full MagicDNS hostname.

Four paired public cases cover Page/Context and primary success/failure. The
success case holds Chrome's actual first-Page ACK and proves the API remains
pending. The failure case substitutes one protocol-error response and holds
Chrome's successful compensation ACK, proving public rejection arrives before
that ACK is released. Context cases also assert the second Page has not started.
A following public setter and real navigation succeed while the old ACK is held.
Releasing it makes the later Context Page use the new desired value, verified
from the actual native command's Chrome receipt. Final public navigation and
Context/Browser cleanup verify wire values and zero candidate host/FFI owners.
This establishes the tested native receipt ordering, not OOPIF crash suppression,
compensation-failure parity or shutdown cancellation semantics.

Evidence root: `.work/node089-native-header-fault/`. `initial.log` demonstrates
the Page native-error message mismatch; its Context case also exposed the test's
incorrect all-Page fanout assumption by timing out on upstream. After correcting
the Page grouping and fixture, `intermediate.log` passes Page but demonstrates
the candidate's stale `old` value on the later Context Page (upstream: `after`).
The final rebuilt combined artifact passes all four focused cases
(`focused-final.log`, 6.35 s). Release host tests pass **269**, nine existing
ignored; new tests cover sequential group advancement, failed-group drain without
later dispatch, and bounded native protocol-error projection. All-target Clippy
passes.

Final expanded public gate: **35/35**, `live-expanded.log` (79.57 s), including
post-release wire checks before a final setter can mask stale installation.
Three further focused repetitions each pass **4/4** (`receipt-repeat-1.log`
through `receipt-repeat-3.log`, 6.41/6.87/6.39 s). The initial full run remains
**34/35** in `live-final.log`: candidate Context-success setup failed during
connectOverCDP with `Topology initialization failed: CDP error -32001: Session
with given id not found.` This occurred before its fault gate was armed. The
later pass does not resolve native attachment reliability under target churn.

The Rust live host integration passes **1/1** in `host-live-serial.log` (13.29 s),
including synchronous navigation preparation, wire inheritance, 18 retired Pages
and zero leases/operations/topology drivers at close. The first invocation in
`host-live.log` failed before execution because the wrong environment variable
names were supplied; this is an invocation error, not a code regression. The
corrected invocation in `host-live-corrected.log`, run alongside the public lane,
failed during topology initialization with the same missing-session message.
The final isolated run keeps the original test timeout and Chrome process.
Parallel-attachment reliability remains open rather than silently retrying
inside tests or treating the serial pass as proof of concurrency safety.

Final default and header-only addon checks, all seven host/eight addon dependency
profiles, Node **47/47** unit tests, strict TypeScript checks, JavaScript syntax,
formatting and `git diff --check` pass. The combined release addon was rebuilt;
default/header-only checks are compile evidence, not rebuilt live artifacts.

### Resource accounting and unfinished scope

The existing **992-KiB** request reservation, **419** HeaderPool owners, nine
request slots and 80 native semaphore permits remain unchanged. Snapshot-cell
and group-queue bytes are now explicitly counted inside the existing 128-KiB
plan allowance; its test checks route copies, optional first snapshots, cells,
group indices, input and wrapper/scratch together against that sub-budget, not
against the larger total containing future-body reservations. First-Page clones
share their existing pool owner; each later Page acquires at most one new owner.
These are admission reservations, not per-chat/process resident-memory bounds.

Open work includes non-main session-close suppression, actual OOPIF/Worker
coverage, Page-order/attachment handoff under churn, native compensation failure,
host/Worker shutdown with outstanding receipts and foreign wakers, generated
header contracts, NODE-077 navigation-close reliability and the broader Node API
and cross-browser goal. None is marked complete by these four controlled cases.

### NODE-088 verification closed out on resume

The prior five focused processes were confirmed terminal, not restarted:
`race-repeat-1.log` through `race-repeat-5.log` each pass **7/7**. The Rust live
host test passes **1/1** and reports zero leases/operations/topology drivers.
Header-only and frame-only addon checks, default compile, Node unit/type checks,
all seven host/eight addon boundary profiles and formatting completed. Its nine
dynamic fixture ports (55027, 55481, 55816, 56461, 58207, 58382, 58550, 58649,
58694) were verified absent. Preflight Page cleanup is recorded with this turn's
final cleanup; no shared XLQY server/browser was stopped.

### Cleanup and memory checkpoint

All test/build handles are terminal. All fixture/relay ports recorded in NODE-089
logs were checked against listening sockets and are absent. The current exact
preflight Page `6EB132417AA67DA535D88ED3167335CC` was closed through its isolated
Pwright state; the earlier NODE-088 Page `FF9154A17A500CBA23FD7F441D464F10` was
already absent from the live inventory. Unrelated Gemini and XLQY Pages and the
shared dedicated Chrome process remain open. No task-owned runtime/server is left
running, no shared server was stopped, and nothing was staged, committed or pushed.
Foundation HEAD remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`.

At 13:25:19Z, shared Codex backend PID 53189 RSS is 931,280 KiB, down from
1,970,464 KiB at 13:08:17Z. Swap remains zero. These are shared-process snapshots,
not per-chat attribution or proof that this code caused the change. The separate
XLQY cleanup proposal was not authorized and remains untouched.

## NODE-088 Caller-polled native jobs and early primary rejection

The selected host now owns up to nine header jobs inside its existing topology
task. It polls those jobs alongside idle receive, native initialization, explicit
lifecycle transactions and obsolete-session reconciliation. The in-progress
topology future stays pinned; header progress never cancels/restarts a partially
applied topology observation. Completed jobs notify the idle driver so obsolete
native effects are reconciled even if no further browser event/request arrives.
No extra Tokio task, Node driver or event cursor is spawned.

Each job owns the NODE-087 frozen primary plan and an optional frozen compensation
plan. A bounded Fanout starts every member before exposing an immediate failure;
pending calls are retained in FuturesUnordered, not canceled by a try-join shortcut.
On first failure, Rust restores desired metadata and captures/starts compensation,
then sends the original error to the public completion channel. The same job
continues polling both primary siblings and compensation. Its request permit is
held until both drain. Compensation success resolves the update; failed or
unconfirmed compensation retains the model's native uncertainty. A later setter
can proceed while this drain remains active, and late acknowledgement/drop does
not repeat restoration over the later desired value.

Pinned 1.58.2 source references are Page.setExtraHTTPHeaders and
BrowserContext.setExtraHTTPHeaders (pre-await assignment, detached compensation),
CRNetworkManager._forEachSession (Promise.all fanout) and addSession (Network.enable
at attachment). The host now enables Network during scoped initialization, not
before every setter. Waiting for a per-setter enable ACK would introduce another
dispatch-order race absent from upstream's normal header update path.

### Backpressure and allocation boundaries

The existing 80-native-operation bound is now a semaphore shared by application,
initialization and obsolete-session clearing. Native JSON copies are constructed
only after admission. Saturation waits instead of turning an attachment into a
capacity failure that seals the topology observer. Because header jobs keep being
polled while initialization/reconciliation awaits, their native receipts release
capacity without a driver deadlock. Already admitted waiting future bodies remain
bounded by the nine request owners and two 80-route fanouts per request.

Each native future is checked <=4 KiB before boxing, with a 1-KiB per-member
queue/waker/control allowance. Each header-job future is checked <=64 KiB before
boxing; a unit test measures the actual production future type. Per-request
reservation is **992 KiB** = 128 KiB existing input/routes/plans + 64 KiB job body
+ 2 * 80 * (4 KiB future + 1 KiB control). The fixed installation allowance covers
the job-queue header and coalescing completion notification. Native scratch remains
80 * 160 KiB. HeaderPool now admits **419 owners**, adding eight more possible
16-Page compensation plans to NODE-087's 291-owner derivation. The exact pool
capacity and overflow/reclamation test has been updated. These are resource
reservations, not measurements of resident RAM or a bound on Chromium/Node/allocator
internals. `futures-util` is an optional direct dependency only with extra-headers;
it already existed transitively in the resolved host graph.

### Evidence and unfinished fidelity

Evidence is in `.work/node088-header-jobs/`. The latest release host suite passes
**266 tests**, with nine existing ignored tests. Controlled tests cover eager
sibling start, later-member failure without waiting for the first member, retained
drain after primary failure, future/count admission, exact production job-body
size, job-drop reclamation, idle reconciliation wakeup, and full native capacity
while a cooperating initializer waits. The actual production reject-and-drain
helper is tested with held primary and compensation gates for both compensation
success and failure: public error is available first, a later desired setting
survives late resolution, and all ledger charges return to zero.

The rebuilt combined artifact passes **31/31** public differential tests
(`live-complete.log`, 45.55 s), including a new two-Page partial-failure/recovery
case. One Page override masks an invalid Context value while its sibling rejects;
a subsequent setter must recover both actual wire values. Existing invocation
ordering, conversion, creation and retirement groups remain included. Earlier
in-turn artifacts passed 30/30 (`live-initial.log`, `live-final.log`); these do not
substitute for the final 31-test artifact. All-target Clippy and default addon
compile pass after fixing test-module placement and a cfg-statement syntax error.

Important limits: the held-compensation test is a Rust production-helper test,
not yet a controlled real-CDP/public-Node timing oracle. OOPIF/Worker session-close
error suppression, attachment handoff with old frozen routes, native failure
injection and exact public event ordering still require verification. On host
shutdown/peer loss, outstanding job futures and queue entries are dropped before
transport retirement; unresolved updates record uncertainty. This is NOT proof
that a written native command was canceled or compensated, nor a complete native
retirement/foreign-waker lifetime audit. The existing NODE-077 navigation-close
discrepancy and broader Node API/cross-browser scope remain open. The full goal
is unchanged and is not complete.

## NODE-087 Synchronous Rust preparation and frozen dispatch

The selected N-API setter now calls `start_set_extra_http_headers_native` before
spawning its completion future. Rust reserves lifecycle queue capacity first,
admits one of nine request/writer slots, resolves portable topology, publishes
desired metadata and freezes native dispatch values. Node still only converts
language values and settles results; it owns no header/navigation policy or CDP
transport. Owned input buffers and borrowed HeaderRef arrays do not escape the
synchronous setter preparation. Its returned future owns completion and the
operation tracker, not a deferred metadata mutation.

Selected Page/Frame goto similarly calls `start_page_goto` / `start_frame_goto`
before scheduling. Rust captures the first original Page Referer and any conflict
then; it does not read a later setter's values when Tokio eventually polls the
future. The existing eight navigation permits remain alive through completion,
including while a prepared future has not been polled. Header-free builds keep
their existing async path; ordinary Rust async methods prepare on first poll.

### Atomic admission and explicit owners

The host wraps HeaderSettings in an Arc/Mutex preparation boundary. Every host
metadata reader (navigation, initializer, creation, prune and compensation) uses
it. A setter's publication and effective-plan admission share that lock. If plan
admission fails, rollback happens before any host reader can observe the proposal;
no native operation was dispatched. The lock is never held across native awaits.
Queue capacity is reserved before publication, so queue-full rejection cannot
leave a published update behind. After publication, cancellation is different:
an initializer may have consumed the proposal. PreparedUpdate Drop restores
desired metadata but records native uncertainty; it does not claim compensation
or absence of native effects. Late drop after restoration never assigns again.

Each primary dispatch freezes at most 80 route IDs and one merged owner per Page
(at most 16), shared by that Page's main/OOPIF routes. The driver uses this plan,
not whatever latest settings exist at execution time. On failure, restoration
and compensation-plan capture share the preparation lock; later setters cannot
mutate that frozen compensation. Native primary and compensation still execute
serially on the existing driver, so native concurrency/early settlement is NOT
implemented by this change.

The selected model now admits nine overlapping writers. HeaderPool grows from
160 to a derived **291 owner slots**: 32 sparse settings + 9 * (16 per-Page
dispatches + one prior + one input) + 80 initializer snapshots + 16 compensation
snapshots + one temporary proposal. This deliberately overcounts shared originals
and setter/creation request overlap. The existing 128-KiB per-request allowance
covers both bounded route copies, both per-Page plan vectors, borrowed input and
future/wrapper scratch; a test checks the sum. Each effective value still has its
own pool byte lease. Header strings remain <=64 KiB / 256 entries per owner;
navigation keeps its separate <=16-KiB copy and eight 32-KiB allowances. These are
admission limits, not resident-RAM estimates; allocator, JS/Chromium and task-stack
memory remain external. No dependency, extra driver or browser process is added.

### Verification and remaining scope

Evidence is under `.work/node087-header-preparation/`. Four added host unit tests
exercise frozen per-Page deduplication across newer setters, frozen compensation
and late abandonment, partial-plan reclamation, and all 291 pool slots plus
overflow. The existing request-body arithmetic test now covers two plans.
Release host tests with header/frame/drag selected pass **257**, with nine
existing ignored tests. Clippy all-target checks pass. Default addon compile,
47 Node unit tests, strict types, seven host/eight addon dependency profiles pass.

The first freshly rebuilt combined public lane passes **28/28** (`live.log`),
including the original eight-iteration setter-before-navigation race and the
new eight-iteration reverse order for both Page and main Frame. The expanded
lane adds three overlapping setters at Page and Context scope and passes
**30/30** (`live-expanded.log`). Each paired group uses the actual pinned 1.58.2
public API and wire fixture; public errors are compared by first line, not full
call log. Five consecutive focused repetitions pass **6/6 each**
(`race-repeat-1.log` through `race-repeat-5.log`), including both directions for
Page/main Frame and both overlapping setter scopes. Unlike NODE-086's
pass/fail/pass repeats, this artifact has no failure in those five repetitions;
this is evidence for the tested race, not all concurrent browser behavior.

The Rust live host test also passes (`host-live.log`, 13.33 s after compilation).
It prepares a setter without polling its completion and observes the navigation
conflict; it prepares a conflicting navigation, changes desired headers, and
only then polls that navigation. Both prove policy capture at the synchronous
Rust boundary. Eight prepared, unpolled navigations exhaust the ninth slot;
dropping all eight allows a fresh real navigation. Existing current/future Page
wire checks and eighteen Page retirements pass, and host close reports zero
leases, operations and topology drivers. Final all-target Clippy passes after
these test additions. Separate header-only and frame-only addon compile checks
also pass; only the combined native artifact was rebuilt/live-tested this turn.

Cleanup: all eight dynamic fixture ports (53483, 54080, 54612, 54705, 54742,
54777, 54812, 54854) are absent after terminal test handles. The exact task-owned
preflight Page F2D492E516F503F91634A28F3BFC67E0 was closed through its isolated
Pwright working directory. The unrelated XLQY Page/server and shared Chrome
remain running. No task-owned test/build server is intentionally left behind.
At 12:36:14Z the shared Codex backend PID 53189 has RSS 2,733,136 KiB, up from
1,243,552 KiB at 12:21:11Z; this is shared-process fluctuation, not per-chat
attribution. Swap remains zero; compressor storage is about 4.76 GiB. No memory
cleanup, server restart, commit or push was performed. Foundation HEAD remains
f76e3f2701a8ccde0474b5accf40dc328aaa511c; the cumulative FFI work stays unstaged.

Remaining: concurrent native fanout/attachment handoff, primary rejection before
tracked compensation drainage, cancellation/native failure injection, child
Frame/OOPIF/Worker wire coverage, typed capability/service/codec exposure, and
creation failure ordering. The independent NODE-077 navigation-close discrepancy
is not fixed here. The complete Node/Playwright compatibility goal remains active.
Earlier NODE entries below describe their historical artifacts and limitations.

## NODE-086 Pending setter race and bounded overlapping model

The new public test starts a Page header setter, then Page/Frame navigation
without awaiting the setter. Both promises are immediately observed with
Promise.allSettled. Against Playwright 1.58.2, eight Frame navigations reject
the old explicit referer without wire requests. The candidate instead navigated
in two iterations (`.work/node086-header-ordering/before.log`: Page group passed,
Frame group failed). This is an actual ordering discrepancy, not a missing await
in the test. Sequential NODE-085 passes do not cover this behavior.

Source inspection identifies two boundaries requiring work. N-API currently
spawns independent host futures; the setter's desired metadata is published only
when its request reaches the topology driver. Navigation can read the older
metadata first. The driver also awaits an entire setter application/compensation
before processing its next lifecycle request. Upstream setters assign desired
headers before awaiting native work and allow another setter to overlap. Adding
a fence that waits for the first setter's native ACK would not reproduce that
contract, especially early rejection while compensation remains in flight.

### Implemented model prerequisite

`HeaderSettingsProfile::with_concurrent_writers(n)` explicitly selects a bounded
pending-writer table; `new` still defaults to one writer, preserving existing
ordered embeddings. The profile admits `n * size_of::<Pending>()` in addition
to sparse entries, Shared/Arc metadata and padding before either Vec is allocated.
Zero and overflowing counts reject. Each pending record owns its captured prior
snapshot and generation. Copying original values stays in the existing HeaderPool
budget. Empty entries remain as tombstones while an outstanding writer can still
restore them; retirement removes every affected pending record without resurrection.

An older failure must restore its captured value even if a newer proposal is
currently visible. Conversely, success does not republish its old proposal, and
late compensation acknowledgement/failure must not repeat an earlier rollback
assignment over newer desired metadata. The model now separates restoration from
resolution. A per-entry writer identity controls acknowledgement bookkeeping;
unresolved native uncertainty stays separate from readable desired state.
Concurrent Context acknowledgements conservatively avoid healing unrelated Page
uncertainty, unlike the explicitly ordered one-writer profile.

Six added unit tests cover late failure/success, non-repeated compensation,
independent Context/Page writes, captured-prior behavior, writer-capacity admission,
byte-accounting overflow and retirement/lease reclamation. The selected model
suite passes **53** (`model-final.log`). The initial unselected model commands
ran only eight base tests (`model-initial.log`, `model.log`); they are not header
evidence. The corrected selected runs use `--features extra-headers`.

`check-header-overlap-oracle.mjs` calls the actual pinned Page/BrowserContext
setter methods with controlled native-completion promises. It compares **128/128**
traces against `header_overlap_oracle`: Page/Context owners, absent/present prior
values, both primary completion orders, all success/failure combinations, and
held compensation resolved/rejected in reverse order. Desired snapshots at each
native dispatch and settlement match. Server setter promises reject while their
compensation is held. This is an internal model comparison, not native fanout or
FFI ordering proof. The existing sequential oracle also remains **96/96**.

Build the development examples with `cargo build --release --locked -p
pwright-network-model --features extra-headers --examples -j2`, then run the new
script with `PWRIGHT_HEADER_OVERLAP_RUST_ORACLE` set to the absolute built example
path. The bridge caps input/output at 256 KiB, cases at 128 and execution at
30 seconds. The example is feature-gated; no production dependency is added.

### Required host integration, not implemented here

1. Admit invocation order in Rust at the language-call boundary, before independent
   task scheduling can reorder setter publication and navigation preparation.
   Capture navigation policy at its ordered preparation phase; waiting for a
   setter ACK or reading whatever latest value exists later is not equivalent.
2. Select a bounded overlapping settings profile and freeze each native dispatch's
   effective values/routes under explicit owner budgets. Applying a queued older
   request using the latest settings instead of its dispatch snapshot is not parity.
3. Let the existing driver progress native writes, attachment and later requests
   without awaiting one whole setter transaction. Track bounded in-flight native
   application and compensation owners; do not add an untracked background task.
4. Separate public primary settlement from compensation completion while preserving
   teardown drainage, exact native retirement, resource reclamation and Context/Page
   failure visibility. Verify this with controlled native outcomes and public races.

The host still selects one writer and the public race remains unresolved. Do not
promote the 128-case model gate or an occasional passing race run to public ordering
compatibility. Full goal completion remains unproven.

### Final NODE-086 gates and reproduced remaining failure

The freshly rebuilt combined artifact passes 26/26 once (`live.log`, 19.26 s).
Three focused repetitions of the same artifact then produce pass, **Frame race
failure**, pass (`race-repeat-1.log` through `race-repeat-3.log`). This confirms
the public discrepancy remains intermittent in the current build. It is not
fixed by the model extension, nor hidden by the single full-lane pass.
Selected host header+Frame tests pass 251 with seven existing ignored tests;
Node unit tests pass 47, strict types/default addon/dependency gates pass.
Model and complete selected host/addon all-target Clippy pass. Logs include
`host.log`, `unit.log`, `types.log`, `default-check.log`, `boundary.log`,
`clippy.log` and `model-clippy-final.log`. Formatting/JS syntax/whitespace checks
pass. Foundation commit remains local and unchanged; no staging or push occurred.
All five dynamic fixture listeners and the task-owned blank Page are gone. No
task-owned test/build process is intentionally retained. Final shared-backend RSS
is about 2.23 GiB with zero disk swap; this is not per-chat memory attribution.

## NODE-085 Navigation referrer reconciliation

The selected Rust Page.goto and public Frame.goto host paths now invoke the
existing `PageHeaders::navigation_referrer` policy before native dispatch. This
reads the first original case-insensitive Page Referer, not the last-wins merged
network header list. Context headers do not participate in option conflicts.
An unequal explicitly supplied referer rejects even when the Page header is an
empty string; an equal option is retained without copying. Omitting the option
copies the selected Page value. Clearing Page headers removes that policy input,
while native Context header inheritance continues independently.

The raw `navigate_frame_native` port remains a lower-level dispatch interface;
public `frame_goto` performs reconciliation above it. No JS browser policy, new
dependency, cursor or driver task is added. A prepared referrer guard admits one
of eight independent navigation slots before copying. The selected installation
reserves 8 x 32 KiB for these owners in its existing body lease, independently
of the setter/creation request slots. The 16-KiB navigation-string bound applies
to implicit Page referrers too; a larger stored header produces explicit resource
exhaustion before copying. This is a selected limit, not a claim that Playwright
has the same cap. Supplied owned strings are moved, not duplicated. The guard
survives moving its value into the command and remains held through completion;
rejection/drop releases it. Header snapshots are only borrowed during preparation.

The pre-fix public differential reproduced missing conflict rejection in both
Page and Frame (`.work/node085-navigation-referrer/before.log`). Each new paired
group covers thirteen sequences, including Context-only/explicit behavior,
Page defaults, duplicate-first precedence, equal/unequal/empty options and
clear-to-Context inheritance. A dynamic fixture verifies actual Referer values;
conflict cases assert zero document requests and an unchanged Page URL. Error
class and first line are compared; exact call-log/stack formatting is not claimed.
A candidate case repeats seventeen conflicts, then successfully navigates, to
check rejection-slot reclamation. Existing final-close resource assertions remain.

The freshly rebuilt combined drag/frame/header artifact passes **24/24** tests
(`combined-live.log`, 63.76 s): twenty paired groups and four candidate groups.
Host header+Frame tests pass 251 with seven existing ignored tests (`host.log`).
Both host/addon all-target all-feature Clippy passes (`clippy-all.log`); the first
Clippy run caught an unused re-export, removed without suppression. Build logs
are `build.log` and `header-build.log`. Header-only full regression passes
**23/23** (`header-live.log`, 16.82 s); only the independently selected Frame
matrix is absent. Node unit tests pass 47, strict types/default addon checks
pass, and `boundary.log` verifies seven host/eight addon selections. Formatting,
JS syntax and whitespace checks pass. All three fixture listeners and the
task-owned blank Page are gone; no test/build process is intentionally retained.
The final shared backend RSS is about 522 MiB with zero disk swap, not per-chat
RAM. Foundation commit remains local and unchanged; no staging, new commit or
push occurred.

These tests await header setters. Racing a pending setter/rollback against
navigation, native fanout/early-rejection/cancellation, child-frame/OOPIF/worker
wire behavior, injected creation compensation failure and full navigation/event
parity remain unproven. No full-goal completion is claimed. Historical checkpoint
statements below about missing referrer integration refer to those earlier states.

## NODE-084 Context creation headers

The selected Node profile now accepts `browser.newContext({ extraHTTPHeaders })`.
The JS facade copies own enumerable options once, like pinned 1.58.2
`Selectors._withSelectorOptions`, and skips falsy header values, like
`prepareBrowserContextParams`. Rust's scoped language port retains the setter's
own-validation / inherited-value collection / boxed conversion phases, using
the creation API label and `extraHTTPHeaders[index].value` channel path.
Arbitrary synchronous getter throws bypass the host-error mapper unchanged.
TypeScript advertises string-valued records. Header-free profiles remain
unsupported for truthy header options; omitted/falsy options use ordinary creation.

Creation is not implemented as a JS create-then-set sequence. The selected Rust
host admits and copies a `CreationHeaders` owner before native creation, sharing
the installation's nine request permits and 128-KiB per-request scratch allowance
with setters. It retains bounded original strings, not JS values. The existing
topology driver creates the native Context, commits desired headers synchronously,
then commits baseURL metadata and publishes the descriptor. Post-create header
or metadata failure invokes exact-Context close compensation. HTTP syntax is
deferred until a Page installs the effective headers, so invalid creation syntax
can be cleared, or fail a Page locally without invalidating its Context.

The creation FFI ticket reserves 512 KiB plus the existing 32-KiB small-call
allowance for baseURL conversion and descriptor publication. Input stays within
256 keys / 64 KiB total UTF-8 header strings and 16 KiB baseURL. No dependency,
driver task, event cursor, Node driver or browser download is added. Dropped queued
requests release their prepared owner/permit; reconciled Context retirement
reclaims settings. These are logical reservations, not resident-memory estimates.
V8 enumeration/getter allocations and allocator overhead remain external costs.

The header-only release artifact passes **21/21** public tests: eighteen paired
groups and three candidate admission/retirement groups. New coverage includes
first-Page inheritance, baseURL composition, caller mutation after creation,
Page override/clear, falsy/inherited options, getter reads, channel error paths,
thrown identity, invalid-syntax Page recovery, rejected oversized creation input,
and eighteen successive header-bearing Context retirements. Final close checks
zero host leases, cleanup-in-flight, pending FFI results and reserved bytes.

Evidence: `.work/node084-context-headers/live-window.log` (21/21, 34.64 s),
`build.log`, `host.log` (235 pass, seven existing ignored), `unit.log` (47 pass),
`types.log`, `clippy.log` (all three features/all targets), and
`default-check.log`. The initial `live.log` failed eighteen paired groups in the
upstream attachment before exercising candidate behavior: no Page window remained,
and Browser.setDownloadBehavior reported unsupported Context management. Opening
a task-owned blank Page fixed the rerun on the same Chrome process; no browser
restart or oracle behavior bypass was used. Keep this environmental failure
separate from compatibility results.

The freshly rebuilt `drag-frame-extra-headers` artifact also passes **21/21**
(`combined-build.log`, `combined-live.log`, 16.83 s), including Frame.goto after
Page-initialization recovery. `boundary.log` verifies seven host/eight addon
dependency selections and the absence of JS driver/CDP/process-launch imports.
Formatting, JS syntax and whitespace checks pass. The task-owned blank Page and
all three dynamic fixture listeners are gone. No task-owned background runtime
is retained. Final shared-backend RSS is about 2.11 GiB with zero disk swap;
this is not per-chat RAM. Foundation commit remains local and unchanged at
`f76e3f2701a8ccde0474b5accf40dc328aaa511c`; no staging, new commit or push occurred.

This checkpoint does not prove post-create compensation under injected native
failure, pending creation cancellation, exact conflicting-invalid-option ordering,
exotic Proxy/String behavior or full Page-event ordering. Navigation referer
policy, typed header contracts and native fanout/early-rejection/worker/OOPIF
coverage remain goal work. Historical NODE sections below retain their original
scope; their statements that creation is unsupported describe those checkpoints.

## NODE-083 deferred syntax and Page-local initialization failure

Context creation characterization revealed that original desired headers must
not be HTTP-syntax validated. The extended Playwright 1.58.2 public oracle has
29 cases. An invalid creation option and an invalid setter on an empty Context
both succeed at the Context phase, then fail at `browserContext.newPage`.
An empty Context can also clear the invalid value before creating a Page.
A Page override can mask an invalid Context value; a later duplicate can mask
an invalid earlier Page value. The old candidate rejected all three patterns
before merge/installation (`before.log`: 10 pass, three fail).

`HeaderPool` now validates allocation bounds only when copying desired settings
and materializing merged values. `EffectiveHeaders::validate_http_syntax` is a
borrowed, allocation-free check of the final merged values at installation.
Its copyable `HeaderSyntaxError` is distinct from allocation/settings
`HeaderError`. Context/Page spelling, ordering and invalid strings remain in
admitted snapshots; masked syntax does not reject a setter. A later failing
unmask restores the prior Page override through the existing transaction.

The next recovery case caught a separate lifecycle bug: Page initialization's
header error escaped through the shared topology loop, terminating the attachment
and sealing an optional Frame observer (`recovery.log`: 13 pass, one fail).
Expected syntax rejection is now recorded as a Page-keyed typed failure while
ordinary topology initialization completes. The explicit `newPage` transaction
then closes only that newly created Page, reconciles its retirement, and rejects
with the header error. The Context and shared topology/Frame observer survive.
Other transport/initializer failures are not silently classified as syntax errors.

It is important to follow the full upstream lifecycle, not just Page.reportAsNew:
the server emits a failed Page internally, but BrowserContext.newPage's catch
closes it before its public promise rejects. An intermediate test assertion
that expected a surviving Page failed on the oracle (`recovery-final.log`).
The corrected public contract checks zero Pages after rejection, successful
Context clearing and new Page creation. The candidate does not yet claim full
Page-event ordering parity for that transient internal publication.

The selected installation pre-admits storage for sixteen `(ObjectId,
HeaderSyntaxError)` entries in its existing body reservation, before constructing
the Vec. First failure is latched per Page; repeated session initialization does
not add owners. No error string, task or event cursor is retained. Reconciled
Page retirement reclaims slots, and an explicit capacity rejection prevents
unbounded growth. The original FFI/snapshot bounds are unchanged. An 18-Page
rejection/retirement test exceeds the table's lifetime capacity; combined profile
testing additionally exercises Frame.goto after recovery.

This is a prerequisite correction, not an implementation of
`newContext({ extraHTTPHeaders })`. That option still needs a creation-scoped
admitted settings owner, installation before descriptor publication, and exact
Context compensation for post-creation failure. Creation-option language
truthiness/order must follow pinned prepareBrowserContextParams rather than
reusing setter conversion blindly. Navigation referer policy, native transport
failure/cancellation/fanout timing, OOPIF/workers and Page events remain open.

Evidence is in `.work/node083-header-validation/`: `oracle.json`/`oracle.log`
hold the 29 public upstream cases; `before.log` preserves the three early-syntax
failures; `after.log` is the initial 13/13 correction; `recovery.log` preserves
the shared-attachment failure. `settings-oracle.log` reports 96/96 internal
metadata comparisons, now including invalid desired strings. These internal
results alone are not native initialization or public API proof.

### Final NODE-083 gates

Both freshly rebuilt macOS arm64 profiles pass **15/15** live tests:
`extra-headers` (`final.log`, 38.80 s) and `drag-frame-extra-headers`
(`combined-final.log`, 38.97 s). Each has thirteen paired public groups and two
candidate-only admission/retirement checks. The combined recovery case also
successfully executes Frame.goto after the failed Page has been closed.
Each run includes eighteen failed creations, zero Pages after every rejection,
subsequent successful creation, and zero host/FFI owners on final close.
This proves expected syntax failure isolation, not arbitrary native failures,
popup event ordering or full combined drag/navigation parity.

Host library tests pass 235 with seven existing ignored tests; network model
tests pass 47. All-three-feature all-target Clippy, default addon check, 47 Node
unit tests, strict types, and the seven-host/eight-addon dependency gates pass.
Logs: `host-final.log`, `model-final.log`, `final-clippy.log`,
`default-check.log`, `unit.log`, `types.log`, `boundary.log`.
Freshly rebuilt examples retain 96/96 metadata and 484/484 merge/referrer
matches (`settings-oracle-final.log`, `merge-oracle-final.log`); these remain
internal comparisons separate from the two public fifteen-test runs.
No test fixture listener remains. The foundation commit is unchanged and local;
no staging, new commit or push occurred. No test-owned background runtime is
intentionally retained. Context creation header options remain the next work.

## NODE-082 public selected setters and language-phase differential

The experimental Node facade now exposes `page.setExtraHTTPHeaders(headers)`
and `browserContext.setExtraHTTPHeaders(headers)` through the independently
selected `pwright-node-addon/extra-headers` feature. Header ownership, merge,
native routing, initialization and clearing remain in the Rust host. No Node
driver, browser download, launch process or new runtime dependency is added.
The non-header profiles reject these APIs as unsupported.

The N-API language port mirrors the pinned client phases: own enumerable
validation, prototype-inclusive value collection, then channel string conversion.
Undefined values are skipped and boxed strings use their actual `valueOf`, not
ToString. A live regression caught conversion happening too early: an earlier
boxed value changed a later property before the candidate captured it. The
corrected port retains bounded callback-scoped key/value handles until collection
finishes, then converts them. No JS handle escapes into the Rust async future.

Another live regression distinguishes property-getter exceptions from boxed
conversion exceptions. The former propagate unchanged; the latter pass through
a synchronous JS language-exception port selected by Rust's conversion phase.
That small port performs strict message-prefix assignment and rethrows, preserving
JS setters, frozen objects and primitive/null error behavior. Header policy is
not implemented in this port. Exact stack rewriting, exotic Proxy/enumeration
traps and custom String instanceof behavior remain outside the demonstrated gate.

Each call admits a 512-KiB FFI ticket before Rust input copying. At most 256
enumerable keys and 64 KiB of aggregate copied name/value bytes are accepted.
The allowance covers both lexical handle vectors, UTF conversion overlap,
owned strings, borrowed host projections, async storage and settlement errors.
It is a logical reservation, not RSS. N-API/V8 materializes enumeration arrays
before their length can be checked; those arrays, arbitrary getters/valueOf
allocations, allocator padding and VM stacks remain explicit external costs.
The future owns only Rust data and its host Arc. The ticket remains through
result publication/JS settlement; failed input also releases admission.

Native input syntax failures now preserve Chromium's header error wording;
count/byte/owner/budget failures stay resource failures. This does not prove
native transport-error formatting or partial-failure timing compatibility.

### Selected build and verification

From `packages/pwright`, build with
`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 PWRIGHT_NATIVE_PROFILE=extra-headers pnpm build`.
Run `PWRIGHT_NATIVE_PROFILE=extra-headers pnpm test:extra-headers` after the
dedicated Chrome/Pwright preflight, with `PWRIGHT_TEST_CDP=http://127.0.0.1:9222`,
an explicit Tailscale `PWRIGHT_TEST_BIND`, and a full MagicDNS
`PWRIGHT_TEST_BASE`. This lane starts its own dynamic HTTP header-echo fixture
on that interface and an ephemeral port; it does not serve static files.
It creates/closes only API-owned Contexts, then checks zero host leases,
cleanup-in-flight, FFI pending results and reserved bytes. The existing supervisor
retains its 180-second aggregate/output/panic bounds; cases have 25-second
diagnostic deadlines. The fixture caps connections at eight.

The rebuilt macOS arm64 selected addon passes **10/10** live tests: nine paired
groups against Playwright 1.58.2 and one candidate resource-admission group.
Coverage includes existing/new Page inheritance, duplicate-case precedence,
both clears, syntax/primitive/inherited/boxed values, getter identity, conversion
ordering, prototype shadowing/deletion, conversion exceptions and post-rejection
operation. This is not full Playwright compatibility. Package unit tests pass
47/47; strict TypeScript checks, six selected host module tests, selected and
all-three-feature addon Clippy pass. Resolved dependency checks cover seven host
selections and all eight addon feature combinations; loader tests verify the
eight-by-eight compiled-flag matrix. Only the header-only addon was rebuilt here;
combined resolution/compilation is not combined live verification.

Evidence: `.work/node082-header-ffi/`. `ordering-before.log` preserves both
captured-value failures; `live-expanded.log` preserves both conversion-error
prefix failures; `live-final.log` is the corrected ten-test pass.
`build-final.log`, `clippy-fixed.log`, `combined-clippy.log`, `host-tests.log`,
`unit-final.log`, `types-final.log` and `boundary-final.log` record local gates.

The unchanged selected artifact repeats 10/10 in `live-repeat.log` (14.69 s,
versus 15.10 s in `live-final.log`). The full selected host library suite passes
233 with seven existing ignored tests (`host-all.log`); the default addon also
checks without headers (`default-check.log`). All five fixture listener ports
are absent after the runs. No task-owned test/build process is intentionally
left running. Foundation commit remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`;
no new commit, staging or push occurred.

Remaining required work includes Context creation's `extraHTTPHeaders` option,
Page-only navigation referer reconciliation, typed capability/service exposure,
tracked concurrent native fanout and early rejection/compensation semantics,
cancellation/partial failure, OOPIF/worker wire tests and combined live profiles.
The public setters remain experimental while those paths are incomplete.
Earlier NODE sections below describe their historical artifacts and boundaries.

## NODE-081 selected Rust host and native session reconciliation

`pwright-node-host/extra-headers` now installs the model, a native adapter and
the scoped initializer. The provisional Rust ports are
`set_context_extra_http_headers_native` and
`set_page_extra_http_headers_native`. They copy borrowed input through the
admitted HeaderPool, enter the existing eight-slot driver queue, validate the
live portable target, and apply typed `Network.setExtraHTTPHeaders` parameters
to current Page/OOPIF routes. Context and Page writes cannot interleave with
attachment. New session initialization reads the owning Context/Page's current
desired settings before resume. Empty fresh settings do not erase an existing
borrowed attachment's headers; explicit clearing still sends an empty object.

There is no new event cursor, process or detached worker. The selected host
enables the native Network domain when applying headers. The default host and
all currently packaged addon profiles leave this feature off. No public Node
setter, addon export or selected header artifact is delivered by this step.

The first live run caught stale native state: after updating then clearing a
Context, a newly created Page sent its original Context value again. The driver
can replace a manually attached Page route with its auto-attached session
without detaching the old session. Dropping an old portable route therefore
does not clear its Chromium header side effect. The header adapter now retains
at most 160 exact session IDs that it has modified. Driver reconciliation clears
headers on obsolete sessions before forgetting them; it does not detach sessions
or revoke other capability owners. Only an acknowledged clear or the exact CDP
`-32001 / Session with given id not found.` response confirms retirement. Timeout
and unrelated protocol errors propagate. Successful explicit empty application
also removes that session's applied-state record.
Ordinary event reconciliation allocates no stale-session scratch when all
applied routes remain current, including the never-configured header case.

The initial fixture also incorrectly used an externally created Context that
the Playwright-compatible fallback maps into the default portable Context.
Its host-created Page consequently outlived disposal of the fixture's native
Context. That exact test Page was closed and verified absent. The corrected
test creates a host-owned Context, resolves its exact native cleanup identity,
and closes/disposes only that Context even after test failure. It does not use
the fallback Context containing shared Pages.

`HeaderSettings::retain_live` prunes sparse metadata only against reconciled
native topology; retirement supersedes pending generations. Shutdown seals and
drains queued requests before awaiting connection I/O, because a queued header
request can retain the installation and its connection. The live close check
confirms zero host leases, operations and topology drivers after owners drop.

### Admission and remaining compatibility work

The selected value profile allows 160 immutable owners, each with at most 256
headers and 64 KiB of name/value bytes. Desired metadata has 16 Context and 16
Page slots. A fixed pre-allocation body reservation covers wrapper/Arc metadata,
nine request scratch areas of 128 KiB each, 80 native scratch areas of 160 KiB
each, 160 retained session strings of at most 256 bytes plus Vec elements, and
another 128 KiB for driver reconciliation. Atomic admissions enforce nine
requests and 80 simultaneous native applications independently of spare ledger
capacity. The reservation is a logical peak allowance, not resident RAM.
Native scratch includes one typed-to-JSON map/string copy and a 32-KiB
future/error allowance; wire/response storage uses the existing CDP profile.
Allocator padding, task stacks and Node values remain external boundaries.

Native application currently walks routes sequentially and drains compensation
on the same driver before returning the original application error. Desired
metadata is restored before compensation; unsuccessful compensation leaves
bounded native uncertainty. This intentionally provisional port does **not**
claim Playwright's public Promise fanout/rejection timing. Before public exposure:

- Implement tracked concurrent fanout and upstream-compatible early rejection /
  compensation timing, with real cancellation and partial-failure tests.
- Add Context creation options, typed capability/service and admitted N-API
  enumeration/conversion; reconcile Page-only navigation referer selection.
- Support worker sessions and verify OOPIF headers, attachment races and ownership.
- Build explicit addon/package selections and paired public Node browser gates.

### Verification

Two completed isolated live Chrome runs each pass nine response-header
observations (existing/new Page inheritance, duplicate-case Page precedence,
Context update across two Pages, both clears, rejected syntax preserving state),
wrong-object-kind rejection, 18 Page create/set/close cycles and final host
retirement. This is real Rust-host wire evidence, not a paired public Node test
or live OOPIF/worker result. The first corrected run took 12.41 seconds; the repeat
took 12.45 seconds. The fixture is dynamic Rust HTTP on the explicit Tailscale IP,
with eight peer slots, 16-KiB request buffers and three-second peer deadlines.
The exercised setup/sequence has a 90-second deadline and Context/host cleanup
have 15-second deadlines; initial host attachment has its own 15-second deadline.

Selected host tests pass 231 with seven existing ignored tests; five new tests
cover typed native parameters, slot bounds, allocation allowances, exact native
retirement errors and final-owner release. The model passes 46 tests, including
the new predicate-based retirement regression. Strict selected all-target
Clippy passes. Evidence is under `.work/node081-native-headers/`:
`live.log` preserves the original failure; `live-reconciled-3.log` and
`live-repeat.log` are successful corrected runs; `host-tests.log`,
`model-tests.log`, `clippy-final.log` and `boundary.log` record local gates.
The foundation commit remains local; no new commit or push was made.

Additional gates: default host tests pass 226 with seven existing ignored;
combined header/drag/navigation host check and default addon check pass. Rebuilt
development oracles retain 54/54 pinned setter-metadata and 484/484 merge/referrer
matches. These remain internal comparisons, not the native/public timing gate.
Logs: `default-tests.log`, `combined-check.log`, `addon-default-check.log`,
`settings-oracle.log` and `merge-oracle.log`. Formatting, Node syntax and
whitespace checks pass. The final no-stale-route allocation adjustment is covered
by `host-tests-final.log` and `clippy-final-2.log` after the live runs above.

## NODE-080 bounded desired-settings transactions

`pwright-network-model/extra-headers` now includes `settings::HeaderSettings`:
a sparse, bounded Rust Context/Page settings registry and one ordered writer.
The registry is implemented, but **not installed by the Node host yet**. It does
not send CDP commands, start tasks, or expose public JavaScript setters.

The transaction flow preserves the pinned upstream setter's metadata ordering:

| Transition | Desired metadata | Recorded update status |
| --- | --- | --- |
| `begin_context` / `begin_page` | Proposal visible | Applying |
| `commit` after successful native fanout | Proposal retained | Stable |
| `restore_prior` before native compensation | Prior values visible | Restoring |
| `rollback` after compensation or known absence of side effects | Prior values retained | Stable |
| Unresolved handle drop, including failed compensation | Prior values restored | Indeterminate |

`Stable` means no unresolved recorded update for that scope, not proof that a
new native session has already been initialized. Desired values remain readable
even when native status is indeterminate, and another update may reconcile them.
This is deliberate compatibility: upstream restores its metadata and starts
best-effort compensation without awaiting it; compensation failure does not
replace the original setter error or make its metadata getters unavailable.
An early fail-closed prototype was corrected before this checkpoint.

`prior_effective` returns a separately admitted rollback header plan without
mutating metadata. `restore_prior` clones only admitted Arc snapshots; a first
setting uses an allocation-free absent tombstone. A restoring update cannot be
committed as a successful proposal. Confirmed Context-wide application or
compensation also resolves recorded child uncertainty, provided the embedding
host really included all eligible Page settings in the native fanout.

Context and Page slot limits are independent; an active writer holds its slot
until resolved. New settings are admitted through the injected HeaderPool,
never accepted as arbitrary unbudgeted foreign snapshots. Capacity, syntax and
snapshot-admission failures leave old metadata and writer availability intact.
Commit of empty settings removes the sparse entry. Indeterminate tombstones
remain bounded entries until an update or verified native retirement resolves
them. Page entries bind their Context ID and reject a mismatched parent.
Confirmed Context-wide reconciliation also removes resolved absent child
tombstones, so their sparse Page slots become reusable; a regression covers it.

The registry reserves BackendStateBytes before allocating its fixed Entry Vec,
Shared/Mutex body, Arc counters and a 512-byte metadata allowance. One declared
scope/lease covers that body; the HeaderPool separately covers original,
proposed, prior and escaped effective snapshots. Portable IDs, native target
validation, native session descriptors/requests, executor/future boxes and FFI
values are separate embedding responsibilities. No timing, IDs or native state
are discovered implicitly inside the model.

Verified native retirement methods remove Page or Context/children and supersede
an affected pending generation, so a stale completion/drop cannot restore it
or corrupt a reused slot. `close` seals metadata; pending handles keep the
registry body charged until dropped, while escaped Page snapshots independently
keep their original header lease. Final-drop tests return every ledger class
to zero. Metadata retirement does not authorize closing a shared browser.

Verification: 45 crate tests pass (20 new transaction tests, 17 earlier header
tests and eight network tests), with strict all-target Clippy. The previous
484 internal merge/referrer comparisons still match. A new internal bridge
executes Playwright 1.58.2's actual Page/BrowserContext setter methods against
injected native outcomes: 54/54 cases match during-apply, during-compensation,
final desired headers and primary-error behavior. It separately verifies Rust's
uncertainty flag; Playwright exposes no corresponding flag. The native ports
are fakes, so this is not a public/browser fanout or cancellation-timing gate.

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release -j 2 -p pwright-network-model --features extra-headers --example header_settings_oracle
PWRIGHT_HEADER_SETTINGS_RUST_ORACLE="$PWD/target/release/examples/header_settings_oracle" node packages/pwright/scripts/check-header-settings-oracle.mjs
```

The example limits JSON input/output to 256 KiB and cases to 128; the Node
supervisor bounds its child to 30 seconds and 256 KiB of output. Serde and oracle
JS remain development-only. Local evidence is under
`.work/node080-header-transactions/`: `tests-final.log`, `clippy-final.log`,
`settings-oracle.log`, `merge-oracle.log` and `build-oracles.log`.

Next: select this registry in the Rust host, integrate its writes/retirement
with the existing driver queue, apply headers through the scoped initializer,
and implement admitted current-session fanout plus tracked compensation without
changing upstream-visible primary errors. Then add typed/FFI creation options
and setters, worker/session support, and actual paired public browser gates.
No native addon was rebuilt or browser task started in NODE-080.
The eight default-feature crate tests, formatting, Node syntax, whitespace and
the existing host/addon dependency-boundary profiles also pass. All test/example
processes are terminal; no background runtime was introduced.

## NODE-079 identity-aware native initialization

The production topology driver now calls
`ChromiumSessionInitializer::initialize_scoped` with a borrowed
`ChromiumSessionInitializationTarget`: native session/target/owning-Page target,
portable Page/Context IDs, Context ownership and a typed Page/OOPIF distinction.
Newly discovered Pages pass their reserved Page ID before topology publication;
reconciliation of existing and OOPIF sessions obtains owning IDs from the
committed Page session route. The topology mutex is released before invoking
or awaiting the initializer. Reserved IDs do not imply public readiness and
must not be exposed as ready Page wrappers or treated as live capabilities.

The new method defaults to the original `initialize` entry point, so existing
implementations (including the Node host's owned-Context defaults) require no
source changes and are not invoked twice. No initializer is installed by
default. This adds no settings map, event cursor, queue or background task;
implementations still own admission for retained routing/headers and futures.
The target enum intentionally covers only Page and out-of-process Frame. The
driver's missing worker attachment must not be disguised as a Frame session.

Three focused tests pass: reserved identity before Page publication, failed
scoped initialization leaving the Page unpublished/unresumed, and an OOPIF
receiving its owning Page/Context without reinitializing the parent. The full
composition library suite passes 335 tests with 15 existing tests ignored.
These exercise the real Rust driver with fake native ports, not live browser
header transmission or Node setter behavior.

Strict composition all-target Clippy, default addon check, combined
drag-runtime/frame-navigation addon check, formatting, whitespace and all
existing host/addon dependency-boundary profiles pass. No addon binary was
rebuilt and no live browser run was performed for this port-only change.
The focused release compile took 1m49s; observed compiler RSS reached about
7.6 GiB. All build/check handles are terminal at handoff, with no new background
runtime left running and zero system swap. Shared backend RSS at the final
snapshot was about 3.0 GiB, not attributable exclusively to this task.

Still next: admitted Context/Page registry, ordered setter transactions and
session fanout/rollback, typed native/FFI conversion, worker handling and public
paired comparisons. `initialize_scoped` is the ownership port needed for that
implementation; it does not itself send `Network.setExtraHTTPHeaders`.
Evidence: `.work/node079-scoped-session-settings/scoped-tests.log` and
`composition-tests.log`, plus `clippy.log`, `addon-default-check.log`,
`addon-check.log` and `boundary.log`. The foundation commit remains local and
unchanged. Commands used release mode, `--locked`, `CARGO_INCREMENTAL=0` and
two build jobs; full library tests used `cargo test -p
pwright-composition-chromium --lib`, and strict Clippy used the same package
with `--all-targets -- -D warnings`.

## NODE-078 status

Implemented: default-off `pwright-network-model/extra-headers`, immutable
Context/Page/effective snapshots, admitted merge storage and navigation-referrer
selection. No browser policy runs in the production JavaScript facade.

Verified locally: 25 Rust tests (17 new header tests plus eight existing network
tests), strict selected all-target Clippy, and 484/484 internal comparisons with
Playwright 1.58.2. The public upstream-only characterization has 23 observations,
reproduced in a second Chrome run after replacing the ephemeral origin.
These are **not** 23 or 484 public Pwright API comparisons.

Still required: native Context/Page/session installation, rollback and concurrent
attach ordering, schema/service/FFI exposure, foreign-value admission, selected
artifact packaging and real paired public tests. The Node addon binaries and
public supported-method set have not changed. The overall FFI goal remains open.

## Pinned behavior

Authoritative local upstream sources under Playwright core 1.58.2:

- `lib/server/network.js::mergeHeaders`: Context first, Page second; names compare
  case-insensitively; the last matching spelling/value wins without moving the
  first insertion position.
- `lib/server/frames.js::Frame.gotoImpl`: conflict checking uses only the **first
  original Page** `referer`, not Context headers or the effective last-wins map.
  An explicit differing navigation option throws before native dispatch. An
  empty Page referer is present, not absent.
- `lib/server/chromium/crPage.js::updateExtraHTTPHeaders` and
  `crNetworkManager.js::_setExtraHTTPHeadersForSession`: effective network headers
  are applied to current sessions and reused during new-session initialization.
- `lib/server/page.js::setExtraHTTPHeaders` and
  `browserContext.js::setExtraHTTPHeaders`: restore prior metadata on update
  failure and attempt compensating native update. Context updates additionally
  reach service workers through the Chromium Context implementation.
- `lib/client/network.js::validateHeaders` and `utils/isomorphic/headers.js`:
  JavaScript value validation and enumeration are separate from native header
  syntax validation. `undefined` values are omitted; numbers reject. Own-key
  validation and `for...in` conversion differ for inherited properties.

The live oracle confirms Page precedence, Context updates retaining Page
overrides, clearing Page revealing Context, clearing Context retaining Page,
and duplicate-case last-wins transmission. In this Chrome build, Context referer
does not cause an option conflict but still wins on the wire over a differing
navigation option. Duplicate Page referers can accept an option matching the
first value while sending the last value. Do not collapse those two policies.

The live invalid-input cases cover a non-ASCII name, space/empty name, newline
value, undefined value and numeric value. They distinguish client validation,
Page setter protocol errors and navigation conflicts. The effective-value wire check
validates ASCII token names and rejects control characters except HTAB in values;
it is **not** a proof of all Chromium value encoding, error wording or timing.

## Rust ownership contract

The caller injects `HeaderProfile(scope, owners, headers, bytes)` and a production
resource ledger. Limits must be positive and arithmetic must not overflow.
Context, Page and effective headers have distinct types, so a merged Context
header set cannot accidentally be used for the Page-only referer check.

Each original snapshot preserves duplicate spelling/order and owns copied
strings. Clones share an Arc and one charged owner. A snapshot may outlive the
pool handle; all pool/snapshot charges release after the final reference drops.
Debug formatting reports counts/lengths, never header names or secret values.

The pool declares one scope and `owners + 1` concurrent leases, `owners`
LiveObjects, and BackendStateBytes for its body plus `owners` full snapshot
allowances. A snapshot allowance covers the maximum owned Entry array, borrowed
merge-planning array, copied name/value bytes, snapshot/Arc metadata and a
conservative 512-byte metadata allowance. Pool body/Arc metadata has a separate
256-byte allowance. Allocator padding, caller inputs, native wire buffers and
foreign/runtime objects remain separate costs, not a process-RAM guarantee.

An atomic local owner ceiling prevents borrowing spare slots from unrelated
components in a larger shared ledger. Ledger reservation failure rolls that
local admission back. Input bounds validate before snapshot allocation; HTTP
syntax is deferred until final effective values reach installation. Merging admits
before allocating its bounded borrowed planning array. Selected final bytes are
checked after replacements, so an intermediate borrowed union may exceed the
string byte limit if later Page replacement shrinks it. Failure releases the
temporary result admission and preserves original snapshots.

Production normal dependency closure is exactly `pwright-network-model` and
`pwright-core`, both with and without the feature. Serde/JSON dependencies belong
only to the development example; no Tokio, CDP, browser, Node driver, task or
event cursor is introduced by selecting this model.

## Native integration plan and required evidence

1. Install an independently selected Rust settings registry keyed by portable
   Context/Page identity. Keep original Page snapshots distinct from merged
   session snapshots. Do not extend immutable baseURL metadata with a mutable
   JavaScript shadow map. Admit settings, pending transactions, fanout descriptors
   and all native/foreign copies before retaining or dispatching them.
2. Use NODE-079's identity-aware session-initialization port for direct portable
   settings lookup. Page/OOPIF ownership is implemented; worker ownership and
   settings selection/application are still missing. Resolve the
   race between a settings update and newly attached Page/OOPIF/worker sessions
   using the existing ordered host/driver integration, without a second cursor.
3. Connect NODE-080's desired-settings transaction to native effects: retain
   prior/proposed snapshots, expose the proposal before application, restore
   prior desired metadata before compensation, and track native uncertainty
   separately. Preserve the primary public error when compensation fails. Define concurrent
   setters, Context disposal, detach during fanout, and cancellation explicitly.
   If compensation fails, record indeterminate native status without making
   restored desired metadata unreadable; do not report native restoration merely
   because local metadata was restored.
4. Supply the original owning Page settings to Page/Frame navigation's Rust
   pre-dispatch referrer check, but use effective headers for Network commands.
   Context creation must seed settings before any owned Page can issue requests;
   borrowed/default Context behavior must also have an explicit contract.
5. Add typed optional operations/services and Node `extraHTTPHeaders` creation
   option plus Context/Page setters. The N-API boundary owns bounded JavaScript
   enumeration/string conversion (including exceptional getters and inherited
   properties); merging, retention, native fanout and rollback remain Rust.
6. Compare actual wire requests and errors against pinned public Playwright:
   existing/new Pages, OOPIF attach/replacement, workers, clear/update, rejected
   setters preserving old settings, concurrent navigation/setters, close and
   Worker teardown. Rebuild and test independent/combined artifacts; keep default
   and existing features opt-in. Internal policy success is not a substitute.

## Reproduction and evidence

From the repository root:

```sh
CARGO_INCREMENTAL=0 cargo test --locked --release -j 2 -p pwright-network-model --features extra-headers
CARGO_INCREMENTAL=0 cargo clippy --locked --release -j 2 -p pwright-network-model --all-targets --features extra-headers -- -D warnings
CARGO_INCREMENTAL=0 cargo build --locked --release -j 2 -p pwright-network-model --features extra-headers --example extra_headers_oracle
PWRIGHT_HEADERS_RUST_ORACLE="$PWD/target/release/examples/extra_headers_oracle" node packages/pwright/scripts/check-extra-headers-oracle.mjs
```

The internal comparator executes upstream `mergeHeaders` and `Frame.gotoImpl`
directly with a deliberately rejecting native-navigation port, not a copied JS
algorithm. Eleven ordered header lists crossed with each other and four options
produce 484 cases. The Rust example bounds input/output at 1 MiB and cases at
2048; its child process has a 30-second timeout and 1-MiB output limit. Its JSON
input/output ownership is development-only and not charged to production pools.

`packages/pwright/scripts/probe-extra-headers.mjs` is the separate public oracle.
It requires `PWRIGHT_TEST_CDP`, `PWRIGHT_TEST_BIND`, `PWRIGHT_TEST_HOST`, and an
unused absolute `PWRIGHT_HEADERS_ORACLE` report path. Use the dedicated loopback
CDP instance and a task-specific Pwright preflight directory. Its dynamic HTTP
server binds to the verified Tailscale IP on an ephemeral port; URLs use the
full MagicDNS hostname. It bounds document observations to four per case,
error text to 4096 characters and the report to 128 KiB; navigation timeout is
five seconds and diagnostic creation/setter/cleanup deadlines are ten seconds.
These per-operation deadlines are not an aggregate process supervisor. Server
cleanup now also runs if initial CDP connection or browser disconnect fails.

Local evidence: `.work/node078-extra-headers/{unit-final.log,clippy.log,
internal-oracle.log,oracle.json,oracle-recheck.json,oracle-repeat-check.log}`.
Both public runs finished normally. No native addon was rebuilt, no shared
browser/server was stopped, and no commit or push was added by this checkpoint.
Both task-owned about:blank preflight targets were closed and confirmed absent
through a fresh target inventory. The unrelated XLQY Page was left untouched.
The CLI's native-ID `tab-close` success message did not remove these targets;
cleanup required reattaching the known task-owned target as the active logical
tab and using `close`. This diagnostic does not establish a general CLI fix.
