<!-- agent-updated: 2026-09-10T16:47:04Z -->

NODE-133: The driver handles Target.targetDestroyed as a selected native-world receipt update under topology/route/registry locks. It validates the current epoch and needs no surviving route. This returns Ignored without synthesizing topology/public events; unselected registries preserve prior ignored-event behavior.
[Native Page world-retirement contract and evidence](pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-124: The opt-in transition-reservations module pre-admits native-shadow storage without a mutation gate. Later preparation validates owner/epoch and clones current topology, so intermediate sibling/child changes are not overwritten. The driver does not yet use it; native receipt authority, runtime disposal and publication remain separate. Selected tests pass 76, ordinary 49; no new dependency or transitive feature.
[Topology reservation contract and evidence](pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-122: Selected Page.close cannot wait for the host topology loop that is itself awaiting the lifecycle service. An independently admitted cursor on the same connection now consumes exact native destruction; Page-only discovery is enabled before the first selected close. Target discovery alone still does not commit topology disposal. Unconfirmed owner-loss rollback is followed by independent terminal publication, not a fabricated close receipt.
[Selected confirmation](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

# Chromium topology-event driver

NODE-112: Direct Page/Context close services now expose selected prepared/committed effects
through a separate native decorator. This does not replay them through the
under-lock driver ownership observer. The host must join both inputs explicitly;
terminal teardown still uses its independent saturation-safe path.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The selected Frame ownership observer now receives per-transaction committed
effects before driver aggregation, with default snapshot compatibility. Exact
session detach, Page readiness and deferred popup links are covered; direct
lifecycle-service commits and terminal teardown are not an exhaustive source.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

## NODE-108: Bounded native lifecycle identity receipts

The selected Node CDP profile already enables `with_pending_session_retirement(256)`.
On an exact `Target.detachedFromTarget` event, the reader removes matching pending
envelopes and delivers `CdpError::SessionDetached` before event fanout. Both
discovered and synchronized session initialization recognize that typed outcome
and retire/abandon the old route without pretending unrelated errors succeeded.
Existing reader tests cover detach before first poll, write backpressure, late
native responses, unrelated sessions, opt-out and malformed events. This mechanism
must not be reimplemented merely because a later browser inventory lost a target.

NODE-107's traces counted detach events but did not identify their sessions. They
therefore could not establish whether a detach concerned the initialization
command still lacking a response. NODE-108 adds diagnostic visibility, not a
behavioral timeout fix:

- `pwright-cdp::connection::lifecycle_trace` emits TRACE-only borrowed identities
  for `Target.detachedFromTarget` and `Target.targetDestroyed` after event admission
  and before fanout. Detach uses the inner params.sessionId, not the outer parent
  session. Target destruction supplies only its target identity, not an invented
  session. IDs must be nonempty UTF-8 strings of at most 256 bytes; malformed
  identities produce an explicit malformed flag without retaining/printing them.
  No URL or complete event payload is logged. The disabled path returns before
  inspection. No new queue, runtime, subscription, dependency or resource lease
  is installed; publication and pending-command retirement semantics are unchanged.
- The test-only attach probe records at most 64 lifecycle receipts, with a 128-byte
  per-field diagnostic limit and up to 32 unanswered command IDs per receipt.
  It correlates detach by exact session and destruction by an already known
  session-to-target binding. Earlier native responses are excluded. Receipts do
  not synthesize responses or remove entries from the diagnostic unanswered map:
  an absent CDP response does not prove the native future remains pending after
  SessionDetached was delivered. Capture freezes before cleanup.
- Overflow, malformed records and oversized fields fail diagnostic completeness;
  earlier receipts are not evicted to hide overflow. The report adds `lifecycle`
  to the existing v1 probe schema. `probe-attachment-commands.mjs` now requires a
  rebuilt probe with this bounded array and annotates unanswered commands with
  matching native-retirement receipts. Old binary reports are not silently
  accepted as if they contained identity evidence. Existing 30-second child,
  256 KiB stdout, 64 KiB stderr and 2 MiB aggregate bounds remain unchanged.

Five new CDP identity tests and seven recorder tests pass. The CDP release suite
passes 91; both default and combined attach-probe suites pass 19 (one explicit
live test ignored by default). Strict CDP/host all-target Clippy, Node unit 47,
strict types, 7 host / 8 addon dependency selections, formatting, script syntax
and whitespace checks pass. Evidence: `.work/node108-attachment-retirement-trace/`.

### Fresh native probes: no reproduced timeout

Dedicated external Chrome 152.0.7977.83, same loopback CDP endpoint and pinned
Playwright 1.58.2 observers. Each profile ran two rounds of alone, observer and
observer-plus-browser-session layouts, serially with no other test runner from
this task. Both profiles connected in all six observations, had no unanswered
commands at completion and no recorder overflow. Every attempt saw three detach
receipts; none matched an unanswered command at its receipt, and no target-
destruction receipt was captured. These successful attempts do not disprove the
earlier timeouts or diagnose their cause.

| Profile / layout | Round 1 ms | Round 2 ms |
| --- | ---: | ---: |
| Default alone | 12 | 82 |
| Default observer | 243 | 205 |
| Default observer plus browser session | 237 | 277 |
| Combined alone | 18 | 78 |
| Combined observer | 554 | 464 |
| Combined observer plus browser session | 444 | 491 |

Native probe SHA-256: default
`e0209b2c505d86991234df4559e7f637dff4fa6957e3c6df995d18829ae96a4d`;
combined `481edd4e88fec10ebc451a7b5abf40ae7562f9e0b22927e481c0484498327baf`.
The original NODE-107 failed traces remain intact. No native deadline, retry or
Page-initialization order was changed. Future reproduced stalls should use the
new exact receipts instead of equating any detach count with a matching session.

### Controlled retirement comparison and handoff

The existing `test:attachment-retirement` lane passed 6/6 default and 10/10
combined against pinned upstream Playwright. It holds one exact command for a
fresh controller-owned Page, closes that Page, and verifies that connection
completes without publishing the stale Page and that a fresh Page evaluates to
42. The stronger detach-before-response cases keep the command held until
connect completes, proving no later missing-session response or enlarged timeout
is needed. Cases include Page.enable and, in the combined profile, Network.enable
at both lifecycle/header initialization positions. All connection/FFI cleanup
counters asserted by the lane return to zero.

These are NODE-107 addon artifacts, not newly rebuilt TRACE-enabled Node addons:
default SHA-256 `bda2e892c3a05131e232e6f15da54e9b67be71f43a19c50ddb379caba77b878b`;
combined `4332d193c9578b0fdb6dd6dbddacb09b13f851392dc1616e5d5cf01aa5edfb0f`.
The fresh native probe binaries above carry the new diagnostics. No Node API,
publication logic, event dispatch or deadline changed. Retirement comparison
success supports the existing mechanism in those controlled cases, not a fix
for the earlier intermittent Page.enable/Network.enable stalls on other targets.
The previous partial combined live matrix and missing Node lifecycle callbacks
remain open. Continue the Rust admitted event-step owner; use the new receipts
when investigating another reproduced attachment stall rather than patching
retirement behavior without a matching causal record.

Evidence: `retirement-default.log` (6/6, 6.499 seconds),
`retirement-drag-frame-extra-headers.log` (10/10, 13.863 seconds),
`addon-hashes.log`, and the per-profile native JSON/logs. All test/probe/build
processes ended. Exact preflight target `13C4357E5994BDA639D5F47473E5A3CA`
was verified as about:blank and closed; all 32 diagnostic relay ports have no
listener. No HTTP fixture/serve-lib mount was created and no task-owned runtime
remains. Shared Chrome and other work were preserved; HEAD is f76e3f2 with empty
staging and no new commit/push.

Read-only shared codex PID 53189 RSS snapshots: 2539040 KiB at 05:31:52 UTC,
2464544 at 05:38:46, 1741344 before the final live batch, 1131600 at 05:45:01.
Swap remained zero. These process readings are not exact per-chat memory or an
allocation benchmark of the new TRACE helper.

NODE-107 reproduces combined-profile initial-attachment timeouts in the real Rust
host with upstream observers: four of six bounded observations time out, with
Page.enable or Network.enable lacking a response at the deadline. Alone attempts
pass. Trace target identities point to unrelated XLQY Pages; the later inventory
shows target churn. This narrows the diagnostic but does not prove a retirement
race or justify deadline inflation. The new collection rebase primitive is not
called during these failures. Characterize pending initialization versus target /
session retirement and observer coexistence before implementing a fix.
[Exact receipts, partial live matrix and cleanup](pwright-node-lifecycle-events.md#combined-profile-reproduction-and-incomplete-live-gate).

NODE-104 adds independent Worker-confirmed native Page creation in an exact
controller-owned Context, then checks blocked and delivered collection views.
Default/combined pass this seven-case creation/discovery lane twice. It adds no
native domain, driver or subscription and does not establish popup event parity.
[Receipt, ownership and observed scope](pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

## NODE-103 Selected collection projection

The existing ChromiumFrameOwnershipView now exposes borrowed typed Context and
Page membership iterators (portable ObjectId, owned Context flag, Page Context).
The selected Node FrameTrees producer consumes them in the serial lifecycle owner.
It reserves before local activation, stages Page-tree changes behind a collection
watermark, and commits complete membership before permitting Node publication.
Unselected compositions keep ordinary Frame reconciliation. No extra CDP domain,
subscription or transport is created. Projection errors propagate to retirement;
an abandoned batch cannot leak partially published closure.
[Ownership, budgets and actual Chrome tests](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).

## NODE-097 Page auto-attachment from a second client

The controlled reproduction now identifies a native cross-client interference
case, not a Rust-only navigation discrepancy. Evidence is in
`.work/node097-peer-attachment/`. The same dedicated Chrome 152.0.7977.83 process
is used; no Chrome restart, native addon rebuild or production change was needed.
The previous NODE-096 turn was progress: its timing evidence changed this turn's
experiment instead of motivating an unsupported navigation-waiter patch.

Hold the OWNER's genuine Runtime.runIfWaitingForDebugger command for a paused
iframe under the exact Page.navigate session. Connect a second client while
that command is held, wait for the peer's connection/command receipt, then
release the original command. Both owner/peer relays use the same monotonic
clock and record native identity. No target, error or detach event is fabricated.

The initial experiment incorrectly expected the peer to initialize that iframe
before the owner's command could be released. All four new combinations fail
that diagnostic gate (the four pre-existing disconnect controls pass): the
owner's native child detaches and the peer never receives a child attachment.
The second experiment waits for peer connection instead; its "navigation must
succeed" assertion also fails for ALL four owner/peer combinations, including
Playwright owning the Page while another Playwright connects. All four produce
the original 5-second navigation timeout. Those initial 4/8 logs remain, but
neither is evidence of a candidate-only regression.

Minimal raw-CDP controls isolate the trigger on this Chrome build:

| Second client's operation while the owner resume is held | Original navigation | Owner's exact initial child session |
| --- | --- | --- |
| Open WebSocket only | HTTP 200 | Stays attached |
| Browser.getVersion only | HTTP 200 | Stays attached |
| Browser-root Target.setAutoAttach | HTTP 200 | Stays attached |
| Browser-root auto-attach, then Page.enable on the borrowed Page | HTTP 200 | Stays attached |
| Browser-root auto-attach, then Target.setAutoAttach on that Page | TimeoutError, 5000 ms | Native detach observed |
| Full Playwright peer connection | TimeoutError, 5000 ms | Native detach observed |
| Full Rust peer connection | TimeoutError, 5000 ms | Native detach observed |

Additional one-off upstream controls for Runtime.enable and main/child isolated
world creation succeed. The raw Page-auto-attach trace contains NO peer iframe
attachment, so this failing case is not simply a new peer failing to resume its
own paused iframe. It needs only two acknowledged native commands on the peer;
the owner still receives exact native child retirement. This establishes the
minimal operation-level trigger, not Chromium's internal implementation cause.
Pinned installed Playwright 1.58.2 sends the same Page-scoped command from
`lib/server/chromium/crPage.js:406`; its browser-root initialization is separately
at `crBrowser.js:91`. Rust's recursive session initialization uses the same
autoAttach=true, waitForDebuggerOnStart=true, flatten=true settings.

`test:navigation-peer-disconnect` now retains its four earlier shutdown controls
and adds seven DIRECT paired observations, not seven always-success assertions.
For each peer mode, run pinned upstream as owner and then Rust as owner; compare
the original HTTP response or TimeoutError/deadline, exact child-session retirement,
surviving main title and explicit recovery. Close only the peer, make a SECOND
navigation, and require HTTP 200 plus a usable child with the expected title.
The first result is never retried, rewritten or omitted. Timeout comparison
accepts the existing optional trailing period and excludes call-log text; it is
not a claim of complete error-string/call-log fidelity. Unexpected errors fail.
Browser-local leases must be zero after close; Node-environment FFI counters
must be zero after both connections and all operations drain.

The test-only evidence owner has a 128-entry projection ring, 64 attachment
records and at most one held command of 1024 bytes. Raw controls allow at most
two serial outstanding-at-most-one commands. Fields omit payloads/scripts/headers;
URLs, reasons and native error messages are truncated to 256 characters. The
existing 30-second test, 180-second suite and bounded-output supervisor remain.
No production queue, event cursor, dependency, feature, retry or timeout changes.

The combined direct matrix passes 11/11 in 44.043 s; default passes 11/11 in
37.511 s. All seven paired outcomes agree, including the three failing-native-
navigation modes and successful recovery. Native hashes remain NODE-095's
combined c135bb8e...72384 and default 3e062f6d...ddb5. Node units pass 47/47,
strict TypeScript, 7 host / 8 addon dependency selections and the no-JS-driver
boundary pass. The Rust regression was not rerun because Rust sources/binaries
were unchanged.

The final combined repeat also passes 11/11 (34.877 s), including strengthened
positive assertions for HTTP status and main/child titles in both recovery
branches. The default run's recorded outcomes independently contain the same
healthy values. The original failure is preserved in every failing-native mode;
the conformance claim is equivalence, not navigation success.

Do NOT silently turn the old uncoordinated header/child overlap into a green gate
or assert that every historical failure is explained. Its prior 6/8 result is
unchanged. The reproduced child-retirement/timeout signature is also upstream-
native, so timing under a second client is not by itself a Rust discrepancy.
Use isolated serial lanes for capability conformance and keep shared-browser
stress failures separately classified; use this controlled peer matrix for the
demonstrated interference boundary. Earlier typed-retirement and Context fixes
retain their independent failing-baseline evidence.

Next return to an explicit API gap, Page.isClosed, with Rust-owned state and
pinned live lifecycle/close/disconnect comparisons. Other API/Worker work and
unmatched stress signatures remain open. This checkpoint advances differential
verification but does not complete the full Node FFI goal.

Final isolated regressions pass child headers 8/8 (6.020 s) and headers 35/35
(27.522 s), without overlapping clients from another test lane. Syntax and
tracked/untracked whitespace checks pass. All test handles are terminal; the
exact preflight Page 51927ADAD511E29E2E095C191CFAFA1A is closed, target inventory
is empty, and none of 169 recorded fixture/relay ports remains listening.
No task-owned background process remains; shared Chrome was not restarted.

At 01:40:13 UTC the dynamically verified shared backend PID 53189 has RSS
2,608,672 KiB (opening 625,392 KiB), and Chrome PID 54462 has RSS 1,056,160 KiB
(opening 1,030,048 KiB). Swap remains zero. A targeted subsequent vmmap reports
backend physical footprint 2.3 GiB with historical peak 13.6 GiB. This growth is
reported as shared-process memory, not assigned to this chat. The concurrently
observed Rust compiler belonged to XLQY, verified by its cwd; this task launched
no Rust compiler. No shared app/service was killed, restarted or cleaned up.
Foundation commit f76e3f2 is unchanged; nothing was staged, committed or pushed.

## NODE-096 Navigation timing and borrowing-peer control

This checkpoint changes diagnostics/tests, not the Rust navigation or topology
implementation. Evidence is under `.work/node096-navigation-ordering/`; the
combined/default native hashes remain the NODE-095 hashes recorded below. No
native rebuild, production dependency, feature or resource contract changes.

Correction to NODE-095: seeing load in a failure snapshot does not prove that
load preceded the deadline. The snapshot is taken after command cancellation and
the topology publication fence. In `command-child-overlap.log`, one candidate
navigation sends Page.stopLoading at +5002.855 ms, receives its reply at
+5007.888 ms, and receives the matching main-frame/loader load at +5009.504 ms.
Its final failure snapshot is +5025.164 ms. This trace does NOT establish that
the Rust matcher ignored a timely load. Another timed failure has no load at all.

The same trace records a successful main document request/response/commit well
before the deadline, two paused native iframe attachments under duplicate Page
sessions, and both child detach events around +71 ms. Rust resumes the child on
the superseded parent, then attempts parent-scoped detach; its real second-stage
receipt is -32602. A later trace initializes the current child through headers,
then loses its session during the prerequisite batch. The reason BOTH native
child sessions retire remains unproven. No captured Inspector crash notice proves
a renderer crash, and temporal proximity is not proof that the resume/detach
command or another client caused retirement. Do not hide this with a longer
navigation timeout, a retry, or a blanket missing-session exception.

Diagnostics now record monotonic API-start/command-send/command-completion/event
times, bounded main/child request and response projections, stopLoading and
initialization command order, native frame-detach reasons and Inspector/target
retirement notices. Existing limits remain: 128 unanswered command entries,
128 active attachments, 64 global event projections, 64 navigation projections,
and eight failure snapshots. A new 128-entry navigation command ring retains only
method/identity/timing/error projections, never scripts, headers or response
bodies. URLs/reasons/errors are truncated. These are test-only bounds, not a new
production cursor, history or metric. An unanswered wire command is still not
proof of a live Rust pending owner.

`retirement-child-overlap.log` also exposes an ORACLE fixture-observation failure:
the same native iframe detaches, reattaches with waitingForDebugger=false, then
loads, while the gate waits for a fresh Page.frameNavigated that never arrives.
The gate now accepts the real Target.attachedToTarget URL snapshot as another
source of committed-child identity. It still requires an iframe native target
under the exact Page.navigate session, permits a session replacement only after the prior
session retired, and never retargets an armed/held header operation. This fixes
an observation assumption, not a demonstrated Rust navigation failure. A distinct
earlier oracle Frame.evaluate rejection remains recorded, not relabelled Rust.

The new `pnpm test:navigation-peer-disconnect` is a controlled four-combination
comparison: oracle/candidate own the Page and oracle/candidate borrow it. The
peer first connects to the marked Page; during sandboxed child navigation its
genuine Runtime.runIfWaitingForDebugger command is held at the relay. Disconnect
only that peer, then assert owner navigation response 200, main and child titles,
live child publication and final cleanup. No fake native event/error is supplied.
The gate retains one Page ID, one child ID and one command ID, not the command
body; ordinary supervisor/time/output limits apply. Its HTTP fixture uses the
explicit Tailscale bind and its CDP relay stays loopback-only.

An initial 3/4 probe had a TEST attribution error: it required environment-wide
FFI pending=0 while a second candidate Browser was still navigating. Browser-local
lease checks remain immediate; environment-wide pending/reserved-byte assertions
now run only after BOTH Browsers and navigation are drained. The corrected
combined probe passes 4/4 in 3.109 s. This rules out failure in this controlled
already-connected-peer shutdown case, not all peer attachment/churn races.

Preserved overlap evidence (all using the same combined artifact): initial child
7/8 and repeat 6/8 alongside headers 35/35; timed child 7/8 alongside headers
35/35; command-traced child 5/8 alongside headers 35/35; retirement-traced child
6/8 alongside headers 35/35. The last child run includes one candidate navigation
failure and one oracle observation failure described above. These failed gates
must remain distinct from serial or controlled success.

Next isolate peer attachment DURING child initialization, rather than assuming
that an already-connected peer's disconnect reproduces continuous connection
churn. Correlate native target/session replacement with the owning and borrowing
connections; preserve both real oracle failures and candidate failures. Full
Playwright compatibility, broader Worker APIs and the active Node FFI goal are
not completed by this checkpoint.

Final verification: exact-session child headers pass serial 8/8 (6.946 s).
The controlled peer matrix passes combined 4/4 (1.500 s) and default 4/4
(1.471 s), with final zero Browser leases and drained environment FFI counters.
The final overlap still fails child headers 6/8 (24.918 s), both candidate
fixture-navigation timeouts, alongside headers 35/35 (37.995 s). The gate reports
no observer error for these two failures; both recorded main loads arrive after
the 5-second deadline. No production fix is claimed. Node units pass 47/47,
strict TypeScript, 7 host and 8 addon dependency selections pass, and runtime JS
has no driver/CDP/process-launcher imports. Syntax and whitespace checks pass.

Preserve intermediate failed probes: `snapshot-child-overlap.log` passes 0/8
while its broad target-only observer can select a child from either of the two
Page sessions and disconnects on conflicting live identities. The corrected
observer requires the exact navigation parent session, not merely the Page
target, and failure snapshots now include the original gateError instead of
only the resulting connection-closed error. This intermediate test regression
is distinct from the final original 5-second navigation failures. Its companion
header lane passes 35/35; it must not be presented as Rust worsening to 0/8.

All build/test handles are terminal (no Rust build was started this checkpoint).
The exact owned preflight Page 98CA853E092071130ED6E64945C3A820 is closed; none
of 208 recorded fixture/relay ports is listening. Shared Chrome and extension
services are not killed or restarted. At 01:21:09 UTC, shared backend PID 53189
RSS is 2,299,856 KiB, down 1,343,120 KiB from the opening measurement;
shared Chrome PID 54462 RSS is 1,052,288 KiB, up 171,504 KiB. Swap is zero.
These are shared-process RSS samples, not exact per-chat RAM or unique summed
physical memory. No task-owned background runtime remains. Foundation commit
f76e3f2 remains unchanged; no staging, new commit or push occurred.

A subsequent targeted vmmap sample reports backend footprint 360.9 MiB with a
historical 13.6 GiB peak; neither is this chat's allocation. Chrome's vmmap sample
warns it cannot examine the PartitionAlloc malloc zone, so its 330.5 MiB reported
footprint is not a complete allocator analysis. After closing the preflight,
CDP target inventory is empty; no shared process was terminated by this task.

## NODE-095 Native detach before an initialization response

The new bounded failure snapshot separates NODE-094's residual failures. In
`.work/node095-concurrent-diagnostics/child-overlap.log`, a navigation timeout
has no pending native command. Other failures retain Runtime.runIfWaitingForDebugger
or Page.enable for an iframe session whose exact Target.detachedFromTarget event
has already arrived. Two connect failures similarly retain Network.enable after
the exact Page session detached. These are native receipts and events, not
guessed ownership or an inferred browser crash. The overlap fails child headers
3/8 and headers 34/35 with no Pwright build running during that test pair.

A controlled public comparison now keeps Page.enable or the selected header
Network.enable command held until AFTER native detach and connection settlement.
Pinned Playwright 1.58.2 connects, omits the vanished Page and evaluates a fresh
Page to 42 before the held command receives any response. Old Rust times out.
The expanded baseline is 8/10: the eight earlier receipt-based cases pass and
both new detach-before-receipt cases fail only in Rust. The transport relay does
not fabricate a protocol error; after successful settlement it releases the
command and verifies Chrome's real missing-session receipt independently.

Pinned source: crConnection.js:146-155 rejects pending callbacks when a session
is disposed; crBrowser.js:179-185 closes the Page on native detach. Rust's ordered
topology driver cannot wait for an initialization response while relying on a
later queued detach event to stop that same wait.

The Rust production connection therefore offers explicitly selected
with_pending_session_retirement(max_native_id_bytes). The existing reader, after
the raw event-size check and before optional event filtering, recognizes typed
Target.detachedFromTarget parameters and settles only pending calls whose CDP
ENVELOPE session ID matches. It returns CdpError::SessionDetached, not a fabricated
Protocol error or a connection-wide Closed result. Parent/root commands are not
cancelled merely because their parameters name that child. The original detach
event still follows ordinary admission/fanout; late command replies cannot revive
the removed pending registration. Pending and unsent/write-backpressured calls
settle without waiting for another response. Default/v1 connections do not select
this behavior or copy session identities into their pending entries.

The Node Rust host selects a 256-byte ID bound. Topology initialization can now
discard an unpublished Page, or retire a stale synchronized route, from this
typed proof. Child resume cleanup accepts that proof only for its child-envelope
resume stage; raw root detach and parent-envelope second-stage failures remain
strict. Backend/server error projection classifies it as target-local, not a lost
transport. Optional header application preserves typed errors through initialization;
fanout maps this proof to child success or main TargetClosed without waiting for
the topology observer, and forgets its applied-session entry. Ordinary protocol
errors still use the existing public formatting and are not blanket-ignored.

No persistent session registry, tombstone history, task, cursor, retry, feature,
dependency or disposal authority is added. Each selected pending entry owns a
compact ID through its existing command lease. The per-command WireBytes bound
increases by max_native_id_bytes + size_of(Option<Box<str>>) (272 bytes in the
Node profile on this Mac), including overlap with unsent wire or retained response.
The fixed inline Option also enlarges the pending-entry layout for unselected
connections, but they allocate no ID payload. Native detach matching performs at
most the pending count observed at entry, with no temporary command-ID vector.
Future calls admitted after detach still rely on native validation; this is not
the complete persistent CRSession state machine or recursive session disposal.

Diagnostics remain test-only: 128 pending command records, 128 active attachments,
64 recent global event projections, 64 initial-navigation-scoped projections,
and at most eight failure snapshots per exercise. The scoped ring retains the
exact navigation command session/target, reply frame/loader and corresponding
Page/child events so foreign Page churn cannot evict all fixture evidence.
URLs/errors are truncated to 256/512 characters; existing supervisor output and
time limits are unchanged. The no-pending-command navigation timeout remains a
separate lifecycle/publication investigation.

Rust regression passes 86 CDP, 3 shared backend, 375 composition (15 existing
ignored) and 274 host tests (9 existing ignored). Actual-reader tests cover
matching child versus parent/root envelopes, malformed/irrelevant events, opt-out,
late-response order, native success before detach, unpolled and write-backpressured
calls, ID admission and final zero leases. Node units pass 47/47, strict types
and all 7 host / 8 addon dependency selections pass. The first rebuilt combined
addon passes attachment retirement 10/10. Its hash is
195ec33f431da11e61283847be57030fcaefff44cb9e7593464c2b8bbbec128b;
this intermediate artifact predates the final unsent/write-backpressure change.
Current-source combined addon SHA-256 is
c135bb8e80e886ebc9c4c8e14e7ea3259b1e457e3b39ba950314af38e2172384.
It passes attachment retirement 10/10, child headers serial 8/8 and queued Contexts
2/2. Selected all-target Clippy with warnings denied passes; the updated legacy
server error-mapping test passes (one selected test, 49 filtered).

The old two-case Worker test initially passes 1/2 because it always demands a
second detach command after native closure. This new mechanism correctly skips
that command when detach retires the pending resume. Keep BOTH orderings: the
expanded three-case test delays Rust's actual detach notification until after
the second-stage native receipt to retain NODE-092's real -32602 proof; separately,
event-first closure must connect before releasing the held resume ACK and send
no second detach. The live Worker control still requires a successful detach
receipt and an echo of 7. All three pass, with borrowed Page survival and zero
candidate ownership metrics. No native error or event is fabricated.

An initial current-source test batch failed before candidate execution: after
closing the final preflight Page, this Chrome instance rejected even upstream
Browser.setDownloadBehavior with Browser context management is not supported.
Reopening a task-owned about:blank through Pwright on the SAME browser restored
the precondition; no shared Chrome restart was used. Preserve current-*.log as
environment failures, distinct from the subsequent final-*.log results.
Default artifact SHA-256 is
3e062f6dc3e21a7905477238308a56770e2d7a220375f0e0fba7aa496389ddb5.
Default public checks pass attachment 6/6, Worker orderings 3/3 and queued
Contexts 2/2. Other optional artifact profiles were not rebuilt in this checkpoint.

Final no-build overlap: headers pass 35/35 (29.995 s); child headers pass 6/8
(21.773 s), and a second child run passes 6/8 (18.005 s). The second run starts
while headers are still live but finishes after them, so it is partially
overlapping, not a fully overlapping independent pair. There were no compiler
processes at dispatch. Neither child run reports connect/Browser.close failure;
the first fails candidate evaluate and navigation, the second fails two candidate
navigations. This is still a failed concurrency gate, not proof of full reliability.

The scoped second-run snapshot contains main-Frame load for the exact reply loader,
with no unanswered wire command and zero host pendingCalls. It has no event
timestamps and therefore cannot establish whether load preceded the deadline;
NODE-096 demonstrates why cancellation-induced load must be distinguished.
Another navigation lacks that main load.
Relay records are now labelled unansweredWireCommands: a missing wire reply is
NOT proof that a corresponding Rust pending owner still exists. Cancellation or
timeout can have already removed it. At all four failure snapshots the host
reports zero pendingCalls. Preserve this distinction when interpreting earlier
logs whose same diagnostic field was named pending. More precise exercise phase
labels were added for the next investigation; no API deadline was increased.

Next work: reproduce loader/session matching and Frame publication for the
no-pending-call navigation timeout, plus replacement iframe/evaluate progress.
Queued events on a superseded duplicate Page attachment are a hypothesis, not
a demonstrated cause. Retaining complete CRSession closed state, recursive
parent/session disposal, other pending operation lifetimes and full API/Worker
compatibility remain open.

Final strengthened diagnostic fixture passes serial child headers 8/8 in
handoff-child-serial.log (7.191 s). All NODE-095 build/test handles are terminal;
formatting, Node syntax and git whitespace checks pass. Initial compile attempts
exposed pending-entry test initializers/patterns, a test borrow lifetime and
exhaustive error mappings; these were corrected before the successful full run.
The server is a binary, so the initial --lib test selection was corrected to
--bin pwright-server. Failed logs remain available separately from final results.

At 01:00:39 UTC both owned preflight Pages are closed, no normal Page remains,
and none of the 208 non-CDP ports recorded in this checkpoint is listening.
Only a pre-existing Chrome extension service-worker target remains; it was not
stopped. No task-owned background runtime remains, and shared Chrome/XLQY services
were not restarted or killed. Shared backend PID 53189 reports RSS 1,128,368 KiB
and a separately sampled physical footprint of 993.1 MiB; shared Chrome PID 54462
reports RSS 889,808 KiB. Swap is zero. The owned Rust compiler earlier peaked at
8.1 GiB footprint, distinct from the concurrently observed XLQY compiler and
shared app process. These are process measurements, not per-chat RAM attribution.
Foundation commit f76e3f2 remains unchanged; no staging, new commit or push occurred.

## NODE-094 Queued attachments from disposed Contexts

A deterministic public comparison reproduces the remaining browser-root -32602
failure without overlapping suites. Hold Page.enable for one controller-owned
default-Context Page; through the controller's native CDP session, create two
temporary Contexts and their Pages. Wait for each real attachment to reach the
test connection, dispose only that Context, observe its actual detach, then
release the held keeper command. The old Rust driver rejects connectOverCDP;
pinned Playwright 1.58.2 stays connected and usable. A live control retains the
two temporary Contexts, evaluates both borrowed Pages and verifies their native
Context identities survive test-browser disconnect.

The bug is Context classification, not a new native error-normalization rule.
The getBrowserContexts snapshot can predate an attachment; a later refresh can
also omit a Context already disposed while its event was queued. The old driver
treated that unknown ID as a reason to reject/detach the Page. Chrome then
returned No session with given id for an attachment that was already gone.
In the existing selected Playwright fallback mode, every unregistered native
Context now maps directly to this connection's default view, as in pinned
crBrowser.js _onAttachedToTarget (Context lookup followed by default fallback).
Initialization remains responsible for validating the exact native session:
NODE-090/093's typed missing-session handling abandons a vanished Page without
publishing it. This avoids the erroneous root-detach branch rather than
suppressing its error.

Registered Context lookup still takes precedence. Native ID validation is
unchanged. Strict foreign-Context exclusion remains the default for other Rust
callers; raw target_detach, subtype rejection, native-world retirement, retries
and general protocol-error behavior are untouched. No extra native owner,
Context disposal authority, task, queue, event cursor, feature or dependency is
introduced. The selected mode also avoids Context-list refreshes whose purpose
was distinguishing a default identity it does not need to infer.

Evidence directory: `.work/node094-queued-context-retirement/`.
The first baseline is 0/1; the expanded baseline is 1/2 (disposed case fails,
live control passes). Native receipts are from Chrome, not injected error text.
The new Rust test feeds two Context IDs absent from discovery's snapshot and
requires all Pages in the default view, no native Context ownership, no detach
and only the initial Context-list query. Existing strict-mode rejection tests
remain regression controls. Its first draft exceeded the fixture's two-Page
capacity; the corrected fixture tests the necessary pair without raising any
production limit. Rebuilt combined public cases pass 2/2, with zero root detach
commands rejecting either temporary Page, attachment regression 8/8 and child
detach regression 2/2. Default public cases pass 2/2 on its rebuilt addon.
Final Rust regression passes 374 composition tests (15 existing ignored) and
274 host tests (9 existing ignored). Strict-mode exclusion and borrowed Context
ownership controls pass. Node unit tests pass 47/47, strict types and all 7 host /
8 addon dependency selections pass, and selected composition/host all-target
Clippy with warnings denied passes. The first overlap, while a Pwright compiler
was live, passes headers 35/35 but child headers 6/8: one candidate Page.goto
timeout and one upstream Page.goto timeout, not two Rust assertion failures.
The earlier root-detach failure was not reproduced in that run. Repeating after
all Pwright builds finish again passes headers 35/35 but child headers 6/8:
one candidate Page.goto timeout and one candidate Browser.close native-response
timeout. Therefore the timeouts cannot be dismissed as only Pwright build load.
A separate XLQY compiler was active at the repeat's start (gone by its end), so
this is not an idle-machine benchmark. The root-detach reproducer is corrected;
remaining native pending-command/event ordering still needs investigation.
Full compatibility is open.

Final serial checks on the strengthened cleanup fixture pass 2/2
(`final-targeted.log`) and child headers pass 8/8 (`final-child-serial.log`).
These do not override either failed overlapping run. All NODE-094 build/test
handles are terminal. At 00:31 UTC the dedicated Chrome target list is empty,
including the owned preflight target; none of the 106 non-CDP ports recorded in
this checkpoint's logs remain listening. This checkpoint leaves no test-owned
background runtime. Shared Chrome and the separately owned XLQY compiler were
not stopped or restarted.

The read-only handoff memory snapshot reports shared Codex backend PID 53189
RSS 3,797,328 KiB, versus 1,071,936 KiB at 00:27 UTC. This is process-level
growth, not attribution to this chat. A subsequent vmmap summary reports about
3.2 GiB resident mapping total; that different-time measurement is not a
physical-footprint estimate. System swap remains zero and memory_pressure
reports 74% free. The concurrent compiler's working directory identifies it as
XLQY, not Pwright; no cleanup of another task was attempted.
A subsequent handoff sample rises to 4,922,176 KiB RSS, with vmmap reporting
4.6 GiB physical footprint (13.6 GiB process-lifetime peak). Swap is still zero.
The increase remains unexplained and cannot establish per-chat ownership.

Combined addon SHA-256:
`07b042deb4706ccbb538cf7231f20017ea1b93149cbe48db8a7d909cda9f9971`.
Default addon SHA-256:
`04d49a27f4cb17ab55996b2bcd3c341852cfe4c0d8cdea561e5b76ab67847f14`.
Other optional artifact profiles were not rebuilt in this checkpoint.

The test-only relay retains at most three target/session identities, three
retired session IDs, two owned temporary Contexts, 128 pending command records
and one held command. It uses the existing 4-MiB frame/output bounds and
10-second diagnostic / 30-second per-test / 180-second supervisor deadlines.
It starts no HTTP fixture server. Both successful candidate disconnects check
zero host/cleanup/FFI owners; the controller later disposes its own live Contexts
and closes its keeper Page.

NODE-093 preserves typed Network.enable errors through the optional header
initializer so the existing exact-session retirement check can recognize them.
The new second-occurrence attachment test fails before the change and passes
after it; headers-only exercises the same boundary without Frame lifecycle.
Browser-root detach and wider concurrency failures remain separate.
[Evidence and exclusions](pwright-extra-http-headers.md#node-093-typed-header-initialization-retirement).

## NODE-092 Child disappears between resume and detach

The NODE-091 overlapping public lanes exposed a native `-32602: No session with
given id` connection-initialization failure. A targeted NODE-092 test now
reproduces one source of that error without concurrent test lanes: a controller creates one
dedicated browser Worker in its own Page, then the test connection attaches.
A loopback relay holds Chrome's **successful response** to
Runtime.runIfWaitingForDebugger for that exact Worker session. The controller
terminates the Worker, the relay observes its real detach event, then releases
the successful resume response. Rust's subsequent Target.detachFromTarget,
addressed to the announcing parent with the exact child ID in its parameters,
receives Chrome's actual -32602 receipt. Before the fix Rust rejected connection
initialization; Playwright 1.58.2 remained usable.

The Rust cleanup helper now accepts only the exact second-stage native receipt
`code: -32602, message: "No session with given id"` as already detached. The first
stage retains NODE-090's separate exact missing-child rule. No arbitrary
invalid-parameter error, similar message, parent-session error, timeout, channel
loss or connection closure is normalized to success. No retry, additional
command, task, queue, subscription, allocation owner, feature or dependency is
added. This changes the native cleanup helper's terminal-result classification,
not general CDP error handling or a public Node CDPSession.detach API.

| Stage | Native envelope/parameters | Accepted retirement receipt |
| --- | --- | --- |
| Resume paused child | child session envelope | NODE-090: -32001 / Session with given id not found. |
| Release child attachment | announcing parent envelope, child sessionId parameter | NODE-092: -32602 / No session with given id |

The real public lifecycle comparison is intentionally not a Worker API
equivalence claim. Pinned Playwright's CRPage handles dedicated Workers, while
the current Rust host releases unsupported child inspector sessions. Both must
survive the same controller-owned Worker's death and keep the borrowed Page
usable. A second control leaves the Worker alive: Chrome must acknowledge Rust's
detach normally, and the Worker must still echo a message afterward. Both tests
create/evaluate/close a fresh owned Page, disconnect the test browser, check the
borrowed controller Page again, and assert zero candidate host/cleanup/FFI owners.
Browser Worker retirement is distinct from Node worker_threads shutdown.

Pinned source evidence: Playwright 1.58.2
`lib/server/chromium/crConnection.js:137-144` implements resume then parent-scoped
detach; `crPage.js:561-605` distinguishes iframe, Worker and unsupported targets,
and catches unsupported-target detach errors. Rust deliberately does not
blanket-catch native errors; only the receipt whose meaning is verified above
is accepted.

Evidence directory: `.work/node092-detach-retirement/`.
`baseline.log` contains the actual -32602 receipt and public connection failure
against the NODE-091 combined artifact. After the Rust change,
`live-combined.log` passes both retired/live Worker cases. The later test
strengthening additionally asserts borrowed-Page survival after disconnect.
Native `cdp-tests.log` passes 79 tests. Its new eight-case test runs both real
serialized commands and pending-call registrations, checks parent/child scope,
rejects seven near-miss/transport failures, and checks zero pending registrations
and no third command. Final rebuilt artifact and regression results follow below.

Verified public results on the rebuilt artifacts:

- Combined: `live-combined.log` 2/2; strengthened `repeat-combined.log` 2/2,
  including borrowed Page evaluation after test-browser disconnect.
- Default: `live-default.log` 2/2 with the same strengthened test, no optional
  capability selection.
- Combined serial regressions: `child-headers.log` 8/8, `attachment.log` 7/7.
- Deliberate overlap: `concurrent-headers.log` 35/35, but
  `concurrent-child.log` only 4/8. Two cases fail Browser.close with -32602;
  one fails connect with -32602 and one with Connection closed. This is still a
  failing concurrency gate, not a fully resolved attachment/cleanup issue.

Default SHA-256:
`3c2310da022c9464c7949822bfa5cf4505ad173536268db3920188420af05a02`.
Combined SHA-256:
`b07f43b344dc63398ffa5bcde2d10139ae171d230179bf9550c9baf7e818b358`.
Other optional profile binaries were not rebuilt by NODE-092.

The raw `CdpSession::target_detach` API remains strict. Source inspection finds
its unchanged callers in subtype/rejected Page attachment and cleanup branches
of topology_events.rs and in native_world_retirement.rs. These need their own
native receipt/ownership probes before changing their semantics; do not blanket
accept matching strings or assume this two-stage helper covers those paths.
Final terminal Rust regressions pass: 79 CDP tests, 373 composition tests
(15 existing ignored), and 274 host tests (9 existing ignored). The complete
composition/host release build took 5m 03s with one build job. Node tests pass
47/47, strict types pass, and all 7 host / 8 addon dependency selections pass.
CDP/selected host all-target Clippy with warnings denied, formatting, Node syntax
and git whitespace checks pass. Every recorded build/test handle is terminal.

Browser cleanup is complete: all 71 recorded diagnostic/fixture ports have no
listeners, and owned preflight target C2BEA2D68CE07ECD2E7686ED4CF56F63 was closed.
Both XLQY Pages visible in final-tabs.log were preserved. No shared Chrome/server
was restarted or killed. During verification the shared backend footprint rose
to 8.4 GiB, then a fresh check returned 2.8 GiB with zero swap. These are process
measurements, not per-chat attribution; verification builds after the spike were
limited to one job. At 14:45:19Z the shared backend PID 53189 had RSS 2,313,456 KiB
(versus 2,091,504 KiB at 14:31:01Z); swap was zero. No task-owned build, Worker,
server or relay remains running. Foundation commit f76e3f2 is unchanged and local;
no new commit or push was made.

Run after explicitly building the desired native artifact:

```sh
PWRIGHT_NATIVE_PROFILE=default CARGO_BUILD_JOBS=1 pnpm build
PWRIGHT_NATIVE_PROFILE=default PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
pnpm test:detach-retirement
```

Use the dedicated Chrome profile and task-specific Pwright state directory for
preflight; do not restart shared Chrome to make the comparison pass. The relay
is test-only and loopback-only. The fixture uses controller-owned blob Workers;
it starts no static/dynamic HTTP server. At most one resume response is held and
one child/parent pair is retained per gate, with the existing 4-MiB relay frame
limit and diagnostic/public deadlines.

Still open: the repeated NODE-092 concurrent -32602/Connection closed failures,
the NODE-091 concurrent disconnect timeout, other initialization or
parent-session failure paths, response/detach ordering behind blocked topology
work, crashes/replacements, Node Worker teardown/native-waker ownership,
Page.isClosed and full Node/Firefox/WebKit compatibility. This checkpoint does
not turn an experimental API subset into complete Playwright compatibility.

NODE-091 composes a selected Rust header observer with the Frame observer on this
driver's existing ordered cursor. Actual detach/crash events signal bounded
request watches after topology application; topology route disappearance alone
is not treated as native session closure. No second driver/cursor is added.
[Header contract and ordering exclusions](pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

## NODE-090: missing-session retirement before Page publication

A real loopback CDP gate holds a command for one controller-owned Page, closes
that exact Page through another pinned Playwright connection, and releases the
command. Chrome itself returns `-32001: Session with given id not found.`.
The original Page.enable and Runtime.enable cases both succeeded upstream and
failed candidate connectOverCDP before this change; evidence is retained in
`.work/node090-attachment-retirement/initial.log`.

Rust now separates native discovered-Page initialization from topology/network
publication. Only the exact typed missing-session receipt in that phase abandons
the attempt. No new Page/Frame/ready route is published, no command is retried,
and no replacement session is detached. Direct CdpError and an initializer's
boxed CdpError retain their type. Different codes/messages, timeout, connection
loss, channel loss, Compound errors and string lookalikes still propagate.

The first fix alone passed only 1/7 expanded public cases. Initial connection
also synchronizes already-routed targets. That path now uses the same exact
receipt classification and the existing detach transaction (including network
retirement), rather than retrying a stale route. A borrowed identity check skips
children removed by an earlier parent retirement without another route copy.
Cleanup also discards never-initialized owners after routed Page removal.

A subsequent 3/7 public run exposed a second native failure from child cleanup:
`Runtime.runIfWaitingForDebugger` addressed an already-vanished child session.
The actual CDP child-detach sequence now accepts only that exact child-missing
receipt as already retired and sends no redundant detach. Other resume errors
and errors from the subsequent detach still propagate. This is not a generic
transport-error suppression or implementation of Worker APIs.

The optional synchronous `discard_prepared_page` observer callback reclaims a
reservation that ordinary reconciliation cannot retire because the Page was
never published. The lifecycle registry seals only that unpublished owner;
published owners survive replacement abandonment. Escaped retired handles retain
their existing storage lease until their last reference drops. No new allocation
pool, queue, cursor, runtime task, feature or production dependency is introduced.

The rebuilt combined artifact passes all seven public cases and three further
7/7 repetitions. The existing header regression passes 35/35. The public lane is
`PWRIGHT_NATIVE_PROFILE=drag-frame-extra-headers PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm --dir packages/pwright test:attachment-retirement`.
Five native phases run for every selected artifact; frame-navigation artifacts
also test Network.enable and Page.setLifecycleEventsEnabled. All use real Chrome
receipts and verify a fresh Page can evaluate after connection plus zero candidate
leases/cleanup/pending FFI bytes after disconnect. The shared test-only relay was
extracted from the existing header gate; its listener stays loopback-only.

### Verification evidence

Evidence root: `.work/node090-attachment-retirement/`.

- `initial.log`: original 0/2 paired gate; `live-expanded.log`: first fix 1/7;
  `live-routed.log`: routed fix 3/7; `native-errors.log`: exact child-resume
  receipt establishing the additional failure. None is replaced by the passes.
- `build-cleanup.log`: release combined addon; SHA-256
  `663fa0298b81ca1cedc4e4f7ce64fd71f6a20a72b0b18d34200896624206f5e8`.
- `live-cleanup.log`, `repeat-1.log`, `repeat-2.log`, `repeat-3.log`:
  7/7 each on that artifact. `headers-regression.log`: 35/35.
- `rust-verified.log`: CDP 78/78 and composition 373 passed / 15 ignored.
  Nine new composition cases cover unpublished/routed/OOPIF session retirement,
  unrelated failures, no stale retry, live-owner preservation and escaped-owner
  leases. One CDP case exercises five exact/nonmatching native failure classes
  through real pending-call serialization; the successful two-command detach
  sequence remains covered separately.
- Earlier Rust attempts caught an extraction compile error, a never-initialized
  routed lifecycle owner, and a test fixture retaining its own pending-command
  slot. These were corrected; final checks use the updated sources.
- `clippy-verified.log`: CDP/composition all-target Clippy passes with warnings
  denied. `default-verified.log`: default addon release check passes.
  `host-verified.log`: 269 passed / 9 ignored. Node units 47/47, strict types
  and all seven host/eight addon dependency profiles pass. Only the combined
  artifact was rebuilt/live-tested here.
- All 99 recorded fixture/relay ports have no remaining listeners. Both owned
  preflight Pages were closed through Pwright; final dedicated tab inventory
  is empty. The shared Chrome process and other tasks' services were not stopped.

Final memory checkpoint at 2026-09-09T14:04:33Z: shared backend 53189 RSS
1,380,304 KiB, down from 3,883,008 KiB at 13:56:16Z; disk swap remains zero.
These are shared-process observations, not per-chat RAM or a causal cleanup
claim. All task-owned build/test handles are terminal, with no task-owned
background runtime left running. The remaining compiler was verified by cwd
as the separate XLQY client build. The foundation commit
`f76e3f2701a8ccde0474b5accf40dc328aaa511c` remains unchanged and local;
this checkpoint stages, commits and pushes nothing.

Remaining: real OOPIF/replacement/Worker retirement, other Page-local
initialization failures, later detach-command races, and shutdown/native-waker
ownership. This checkpoint does not prove all earlier attachment stalls resolved
or complete Playwright compatibility.


NODE-088 the selected Node host polls header jobs alongside pinned topology
transactions rather than canceling/restarting receive/initialization. Job completion
wakes idle reconciliation; the same driver still owns topology. Native semaphore
backpressure keeps header jobs progressing while an initializer waits for capacity.
[Driver cooperation and limits](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-083 stores expected header-syntax initialization errors in a bounded
Page-local table instead of failing the shared topology observation attempt.
Explicit failed creation closes its exact Page and reconciles again before
reply publication. Unexpected transport/initializer errors still propagate;
no generic error is silently converted to an expected syntax rejection.
[Scope and verification](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-081's optional host header feature uses the existing driver queue and scoped
initializer. Following route reconciliation it clears obsolete header state on
this feature's exact modified sessions. It introduces no event cursor and does
not detach sessions retained by other capabilities. Native Page wire tests pass;
OOPIF/worker header verification remains separate unfinished work.
[Integration and stale-session failure](pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-079 adds borrowed identity-aware session initialization. Discovery passes
reserved Page/Context IDs before publication; existing/OOPIF sessions inherit
committed Page ownership. Callbacks run outside the topology lock and before
resume. Existing initializers delegate unchanged; worker attachment remains
unsupported, and no settings owner or additional event cursor is installed.
[Contract and tests](pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-073 installs the selected host's retained navigation table before initial
topology synchronization. Its executor consumes the existing Page registration
notices and idle deadlines; it adds no event cursor or detached driver task.
An in-flight route handoff returns its native receipt to the ordered waiter
instead of retrying the browser side effect.
[Route and host contract](pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 adds selected navigation slots inside the existing Page lifecycle
owner. No new cursor/task is introduced. Timer-before-transition, native-notice
and post-transition checkpoints advance registered waiters without reentry.
Operation failures remain local; shared stream failures still seal observation.
[Driver tests and real host evidence](pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-070 adds selected Page admission before native lifecycle/Network domain
enabling. A bounded multi-Page registry uses that hook and the existing ordered
callbacks; no extra cursor/task is added. Native Target creation remains a
separate earlier boundary. The opt-in Rust host selects the registry before its
initial snapshot. [Contract and evidence](pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 adds an optional pre-initialization synchronous observer to this
driver's existing ordered cursor. Selected sessions enable lifecycle/Network
domains; tree snapshots seed document identity. Error/cancellation/loss/drop
seals retained lifecycle queries. No extra queue or subscriber is installed.
[Owner contract and current verification](pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

## NODE-068: commit descendant retirement in the existing transaction

Current-session new-document events now retire old child Frames and reconcile
their runtime/service routes before publishing URL/name ownership. This includes
child/OOPIF commits and same-native-ID main navigation. Wrong-renderer events
and hash changes do not retire descendants. The selected Frame survives; its
renderer route can nevertheless retire during a real cross-origin handoff.
The new native tests use the actual driver, not NODE-067's test-only routing port.
Bounded lifecycle observation itself still needs installation/initialization.
[Contract, live results and remaining integration](pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

## NODE-067: native normalization port, not yet installed in this driver

The navigation leaf now accepts borrowed authoritative session/Frame routing,
explicit Frame attach/remove and a connection epoch. Its live local-frame harness
prunes the external test route mapping immediately after document commits or
removal, before processing another envelope. Production integration must preserve
this coherence and install observation before enabling events. Renderer swaps
are not ordinary Frame removal; real OOPIF/worker initialization and handoff are
still required. Do not treat the test port as production topology integration.
[Normalizer contract and evidence](pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

## NODE-066: optional retained navigation route publication

When Frame navigation access is explicitly installed, prepared service routes
also stage its bounded native routes from the candidate topology. The existing
runtime lifecycle transaction must succeed before commit marks replaced entries
retired; dropping a failed candidate leaves old snapshots valid. Commit adds no
native calls or waker invocation under the route lock. Installation itself does
not start or alter the native event driver. Its future embedding must seal access
on terminal event loss/disconnect and separately control pending native calls.
[Contract and tests](pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

## NODE-062: current attachment delay is target-correlated

Rebuilt the existing attach_probe against the Context metadata host changes;
`.work/node062-context-base/attachment-commands.json` preserves six layout
observations on Chrome 152.0.7977.83. Both standalone Rust attempts time out;
the four observer layouts fail earlier in Playwright 1.58.2's own connection.

In both standalone traces, existing `about:blank` target
`08178DA3BC37B01AAEABFBE835562EEC` has slow `Page.enable`: 4760 ms and 3984 ms.
Those are observed command-preparation-to-response intervals, not proof of
Chrome-only execution time. In round zero the next target's initialization
starts around 4769 ms and three commands remain unanswered at snapshot; in
round one four follow-up commands remain unanswered. The original five-second
deadline is unchanged. No target was excluded from initialization and no shared
browser restart or unrelated target close was performed.

This is evidence for the location of the current delay, not yet its cause or
a fix. Other work owns live application Pages, and the unknown blank target is
not ours to remove. The Node Context and Page acceptance lanes also fail at
attachment; historical NODE-061 passing samples do not certify this build.
[Context implementation and remaining gates](pwright-context-creation.md#verification-status-and-remaining-gates).

## NODE-061: reuse the bounded attachment collector

The existing `crates/pwright-node-host/tests/attach_probe.rs` remains the single
command/session collector. It now ignores the second id/code trace emitted for
a protocol error, rather than counting it as another response, and freezes the
connect snapshot before cleanup. Two regressions bring its offline tests to
12 passing / one explicitly ignored live diagnostic. This fixes diagnostic
accounting only; no browser protocol, scheduler, timeout or addon changed.

`pnpm probe:attachment-commands` runs a prebuilt copy of that existing test
binary under six explicit layouts (two rounds of alone, one oracle observer,
and observer plus browser CDP session). It uses asynchronous child I/O so the
Node observer can keep processing its own protocol; blocking spawn would change
the experiment. A preliminary duplicate collector/example was removed after
the historical implementation was found. Early `.work/node061-attachment-trace/`
prototype output is preserved but is not the canonical reproduction path.

The collector retains at most 32 unanswered commands, 128 completed commands,
64 bounded session/target bindings and 64 event-method counters; text identities
are checked at 128 UTF-8 bytes before copying. It records no parameters, response
bodies, script source or page content. Its sticky overflow fails verification.
The wrapper caps each child's stdout at 256 KiB and stderr at 64 KiB, with a
30-second child deadline and 2 MiB aggregate report. Each attach keeps the
existing five-second deadline. Observer CDP-session creation and cleanup calls
have ten-second diagnostic deadlines, including the existing attachment matrix.
Cleanup failure ends the experiment instead of accumulating observer owners.

Build with `CARGO_INCREMENTAL=0 cargo test --locked --release -p pwright-node-host
--features drag-runtime --test attach_probe --no-run --message-format=json`.
Read the matching `compiler-artifact.executable` from that output; pass its
absolute path as `PWRIGHT_ATTACHMENT_TRACE_BINARY`, the dedicated loopback
`PWRIGHT_TEST_CDP`, and an unused absolute `PWRIGHT_ATTACHMENT_COMMAND_REPORT`.
This is the real Rust host on the existing diagnostic's current-thread runtime,
not a claim about N-API scheduling. Reports retain all probe data and additionally
summarize session/target-correlated slow responses and prepared commands with no
observed response. A prepared trace is not proof that a command reached Chrome.

`commands-existing.json` passes all six layouts with 24 commands / 24 responses
each, no pending commands and no overflow. Alone samples take 150/237 ms; observer
samples 892/874 ms; observer-browser-session samples 899/930 ms. This refresh is
consistent with the previously documented multi-target latency, but it does not
reproduce the five-second failure or establish its cause. External Page membership
changed between the preliminary and reused-probe runs; their timings are not a
controlled A/B comparison. Preserve NODE-060's failures and keep reliability open.
The final wrapper run, `commands-final.json`, also passes all six layouts with
24/24 commands/responses, no pending entries or overflow, at 125–926 ms. Strict
all-target host/addon Clippy, 43 Node unit checks, syntax/format/diff checks and
the unchanged dependency/import boundary pass. The timeout was not reproduced
in either final reused-collector round, so no production fix is justified yet.

The **unchanged** selected addon now also passes the original-layout native glob
gate (1647 patterns / seven boundary checks) and public Page enumeration 7/7 in
`glob-original.json` and `pages-original.json`. No experimental observer ordering,
longer timeout or production change was needed for those runs. This is current
artifact-specific evidence, not a causal fix or repository-wide compatibility.
Final native metadata returns to one index state / zero revisions / 7672 bytes,
with zero URL/name bytes, active native leases and operations. Process-memory
samples mix test/oracle/Node/addon storage and do not establish a parser/RSS budget.

Native artifact hashes and foundation commit are unchanged. No push occurred;
the exact owned preflight target was closed and unrelated Chrome Pages preserved.

The Node recipe now opts into `with_parallel_recursive_attachment()`: after Page
enable and Context initialization, one fixed join awaits frame-tree acquisition,
Runtime enable, optional empty script registration and recursive auto-attach.
All replies precede world creation, resume and publication. The option requires
four pending slots and another four-times response expansion; default and older
profiles remain unchanged. Earlier target events stay queued in the same bounded
inbox; no cross-session scheduler or task is added. [Contract and verification](../node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

Current evidence includes 287 offline composition checks, a live four-command
overlap trace, Node screenshot 12/12 and lifecycle 45/45, and both serial and
recursive-join OOPIF evaluation cases. The latter's selected utility profile
requires four world slots for its two Frames; the original two-slot main-only
case is unchanged. These results do not close intermittent NODE-013 or public
API compatibility; the shared corpus still has unsupported/oracle-error cases.

Initial discovery still initializes Pages serially inside `attach_discovered_page`
as `initialize_snapshot` applies each announcement. Existing-session synchronization
is not the only initialization path; parallelizing that loop alone cannot fix
initial attachment latency. Bounded session/target traces now identify successive
slow targets. A separate width-two prerequisite experiment showed no improvement
with one slow Page and does not run production discovery/world/network/publication
fences. No cross-session scheduling change was adopted. Any future opt-in requires
bounded preparation and response storage plus ordered publication/rollback proofs.
[Diagnostic evidence and limits](../node-ffi-compatibility-design.md#node-013-target-correlation-and-bounded-pair-experiment-2026-09-07).

## Optional pre-admission of foreign world events

The Chromium composition exports a stateless connection policy that omits only
well-formed, explicitly non-default foreign Frame-world creation before retained
CDP event admission. Node selects it explicitly; default connections do not.
Main/Pwright/unknown worlds and every other event remain subject to the existing
exact session, name, epoch and route validation. The policy does not destroy
worlds, alter topology ownership or authorize a prefix as a valid route. Every
consumer on the selected connection must tolerate omitted foreign inventory.
See [classification, resource ordering and evidence](pwright-cdp-event-admission.md).

## Authoritative paired-world access

The optional `ChromiumFrameWorldAccess` port reads the existing synchronized
Frame routes under one lock and waits for both main and utility worlds using
the existing bounded readiness notification. Its non-cloneable snapshots have
separate admission and retained-byte budgets; replacing either world retires
the pair. Snapshot invalidation is sticky once observed. The port adds no
driver, subscription or polling task. The Node host now explicitly installs it
for its single-command Locator evaluation continuation, not every consumer.
See [the ownership contract](pwright-locator-evaluation.md#production-paired-world-access-2026-09-07).

The independently selected `with_native_retirement()` adds bounded weak receipts
for those retained snapshots. Exact `executionContextDestroyed` unique IDs and
session `executionContextsCleared` events update both-world destruction state
inside the same route transaction that removes the worlds. Successful native
Context disposal marks receipts before runtime cancellation; idempotent errors
do not. Default access profiles still install no registry. Detach, crash and
route loss are not destruction proof. Proof matching and snapshot validity
include the connection epoch, preventing old acknowledgements from affecting
reused identities after reconnect. The receipt retains its snapshot permit
and adds no driver, subscription or task. The reproduced NODE-026 host close
failure now passes; see [the opt-in contract](pwright-locator-evaluation.md#optional-native-destruction-receipts-2026-09-07).

The topology's world-route iterators now compare each observed world's session
with `frame_session_id(FrameId)`. A local-to-OOPIF handoff or an unbound swap
gap cannot export the previous session's worlds as command routes. Late
execution-context creation from the previous renderer is an ignored no-op,
not permission to overwrite the current renderer's world. Old world records
remain accounted until their actual retirement; absent routes are not evidence
that native object groups have been released. This preserves the distinction
between route invalidation and native cleanup.

The real-browser port test uses the production topology driver. Its one native
Context is explicitly registered as test-owned before discovery; unrelated
non-default Contexts remain unclaimed by default. Main-world navigation and
same-process child detach are exercised live; cross-process handoff/swap and
late-event ownership are covered by deterministic production-table tests.

The independent Locator preparation phase now has a real-driver integration
test with this port. The source's owning paired reference follows native query,
adoption/evaluation and cleanup continuations, rather than expiring when a
caller drops its readiness/preparation future. Production Node host assembly
now keeps the pair inside one controlled command through delayed conversion;
authoritative session-retirement cleanup remains open (NODE-026). No new
topology subscription or driver is added by the phase itself.

## Optional command-entry world readiness

The Node composition now selects bounded execution-world readiness for DOM
reads, by-value evaluation and ordinary Locator routes. The topology driver
remains the only event consumer: it publishes route changes, not a second
readiness task. A prepared route transaction never wakes commands until commit;
commit releases the route mutex before notification. Waiters register before
rechecking the current exact execution-world route, so a wakeup alone is not
evidence of a usable world and an intermediate replacement is not reused.

The topology's borrowed `frame_page_routes()` enumerates live ownership even
during execution-world gaps, without allocating or cloning native identifiers.
The optional composition retains bounded sorted pairs; detached Frames are
removed. Page crashes and retirement terminate waits. The controlled runtime
owns deadlines and cancellation, including transport loss without another
topology event. Default compositions retain their original immediate lookup.
See the [NODE-021 contract](../node-ffi-compatibility-design.md#node-021-bounded-execution-world-readiness)
for budgets, test evidence and the post-dispatch navigation-race boundary.

> Status: implemented and verified for optional browser-root discovery,
> default Context, existing/new Pages, popup ordering, complete initial frame
> trees, OOPIF sessions, opt-in utility-world creation and locator routing,
> world replacement, detach, crash, sticky lag, and same-connection
> authoritative reconstruction.

## Decision

`with_session_initializer()` optionally selects a caller-owned
`ChromiumSessionInitializer` port. The driver awaits it after Page enable and
before debugger resume/readiness, supplying the exact native session/target,
registered Context ownership and Page-versus-OOPIF role. It covers both existing
route synchronization and discovery. Failure prevents readiness publication;
fake-CDP tests verify ordering and fail-closed behavior. No initializer or
settings are installed by default, and the port adds no task or subscription.
The Node host uses it for owned Context defaults; see
[the Context checkpoint](../node-ffi-compatibility-design.md#owned-context-lifecycle-and-initialization-checkpoint).

The default still excludes Pages from unknown non-default browser contexts.
An explicit `with_playwright_context_fallback()` builder option attributes those
Pages to this connection's default Context, matching Playwright 1.58.2's
Chromium CDP view. The Node compatibility host selects it; other callers are
unchanged. It neither registers nor acquires ownership of the foreign Context
and cannot authorize disposing it. Unit tests prove both default exclusion and
opt-in Page attribution without foreign Context registration; the paired Node
fixture uses a marker Page created by an independent Context owner.

Chromium topology ingestion is optional infrastructure in
`pwright-composition-chromium`, not a portable capability and not a hidden
runtime task. Selecting `ChromiumTopologyEventDriverProductionProfile` adds its
fixed memory declaration to the production plan. After attachment,
`topology_event_driver()` claims the attachment's single event subscription;
a second claim is rejected instead of manufacturing two competing event
baselines. The owner explicitly calls `receive_once(runtime)` whenever its
scheduler permits one event to advance.

This keeps executor, cancellation, fairness, shutdown, and retry policy outside
the browser adapter. A composition that does not select topology events has no
driver, subscription table, or retained driver budget.

Browser-root discovery is a second explicit selection:
`ChromiumPageDiscoveryProductionProfile`. It requires topology events but adds
no portable capability and no task. Without it, the driver can maintain Pages
that another component explicitly created or admitted without claiming every
browser target.

## Browser-root discovery

On its first caller-owned receive, selected discovery performs:

1. browser-root `Target.setAutoAttach(autoAttach=true,
   waitForDebuggerOnStart=true, flatten=true)`;
2. `Target.getTargetInfo` as the Chromium ordering barrier used by Playwright;
3. bounded `Target.getBrowserContexts` enumeration; and
4. transactional creation of one portable default BrowserContext.

The event subscription exists before these calls, so attachments for targets
that predate the driver remain queued. Chrome 152 demonstrated that default
Pages can carry an opaque `browserContextId`; absence is not a safe default
test. The driver therefore records the bounded set returned by
`Target.getBrowserContexts` (contexts created through CDP). A Page whose exact
ID is already registered uses that context. An unregistered ID in the
non-default set is detached. The one remaining stable ID, or a missing ID, maps
to the unique default Context. A later unfamiliar ID triggers a fresh bounded
enumeration before it can be classified. Unsupported root targets are detached
so an auto-attached worker cannot remain paused.

The topology represents the default Context with an internal empty native key,
not a fabricated CDP ID. Ownership is `Default`, close remains forbidden, and
at most one such Context may be registered.

## Session initialization

The Node-FFI attachment diagnostic now records bounded method/session timings
without adding an observer or task to this driver. With an overlapping upstream
Playwright connection, both native host and Node entrypoints reproduced the
five-second initialization deadline. Slow requests span multiple initialization
steps; this is not evidence of one deadlocked FFI callback. A TCP_NODELAY ABBA
experiment did not establish an improvement and was reverted. Existing command
ordering/readiness was unchanged in that diagnostic checkpoint. The subsequent
opt-in paired-prerequisite implementation below retains the readiness fence. See
[NODE-013](../node-ffi-compatibility-design.md#node-013-attachment-diagnostic-checkpoint-2026-09-06).

Before every receive, the driver snapshots the topology's bounded target-local
session routes. Each newly observed Page or OOPIF session receives:

1. `Page.enable`;
2. complete `Page.getFrameTree` acquisition;
3. `Runtime.enable`;
4. when `ChromiumUtilityWorldProfile` is selected, one empty
   `Page.addScriptToEvaluateOnNewDocument` registration with a Page-unique
   world name followed by `Page.createIsolatedWorld` for every current frame;
5. recursive `Target.setAutoAttach(autoAttach=true, flatten=true)`; and
6. when discovery owns paused targets,
   `Runtime.runIfWaitingForDebugger`.

The default sequence remains serial. Selecting the production profile's
`with_parallel_session_prerequisites()` pairs only steps 2 and 3 after Page
enable and the optional Context initializer. Both must succeed before utility
worlds, recursive attachment, debugger resume or readiness publication. This
uses a fixed two-future join on the caller's task, never concurrent sessions or
a spawned task. The Node host initially selected it. Pinned Playwright 1.58.2's
`crPage.js::_initialize` likewise issues these prerequisites independently;
Pwright retains its stronger awaited utility-world/readiness fence.

The opt-in requires at least two configured pending CDP slots. Both requests use
the existing connection admission/leases; no waiting admission or retry is
introduced. The driver reserves a second four-times response expansion for
overlapping decode/retention (an additional 16 MiB declared ceiling in the Node
profile, not a preallocated buffer or RSS claim). Failure drops the other future
and its pending registration; already queued bytes retain their writer lease.
Parent cancellation drops both. Deterministic gated futures cover both pending
orders, both error directions, cancellation and serial-default behavior.

The Node-selected profile passes complete real-Chrome lifecycle 44/44 and
evaluation 38/38 regressions. The independent host trace records both commands
prepared together and all replies; see the
[paired-prerequisite checkpoint](../node-ffi-compatibility-design.md#node-013-paired-prerequisite-checkpoint-2026-09-06).
These results do not establish that intermittent browser scheduling latency is
resolved or that the remaining Playwright API is implemented.

The Node host now explicitly selects
`with_parallel_utility_world_registration()`: a fixed three-future join adds
the empty world-script registration alongside Frame Tree and Runtime enable.
Page enable and the Context initializer remain earlier; all three replies and
tree validation precede actual isolated-world creation. Recursive auto-attach,
debugger resume and publication remain later and sequential. Without a utility
world capability this option performs only the paired prerequisites and sends
no script/world commands. Default and paired-only consumers are unchanged.

This stronger option requires at least three pending CDP slots and reserves a
third `4 * max_response_bytes` expansion. It is idempotent and includes the
paired prerequisite allowance exactly once. Invalid trees can now have an empty
script registration sent before validation, but cannot create actual worlds or
publish a discovered Page. The existing exact-frame disappearance check and
fail-closed handling of world creation errors remain intact. No cross-session
concurrency, spawned task, hidden retry or timeout increase is introduced.
Deterministic tests cover each pending/error branch, owner cancellation,
profile accounting and real driver discovery/maintenance paths. See the
[registration checkpoint](../node-ffi-compatibility-design.md#node-013-three-way-registration-checkpoint-2026-09-07)
for separately retained browser evidence; this is not a NODE-013 closure claim.

Discovery uses `waitForDebuggerOnStart=true`; maintenance-only use keeps it
false. Existing local descendants from the returned frame tree are normalized
parent-first. Page root identity reuses the reserved main-Frame ObjectId, while
OOPIF roots preserve their already-established Frame identity. Duplicate IDs,
an invalid root parent, and contradictory tree parents fail explicitly.
The utility-world name is `__pwright_utility_` plus the fixed-width hexadecimal
portable Page ID. A creation failure triggers one fresh frame-tree read. It is
tolerated only if that exact frame is now absent, after which worlds are
created for the refreshed tree; a still-present frame or any retry failure
preserves the CDP error. The profile starts no task and retains no name map.

The same synchronization runs after applying an event. For a discovered Page,
the driver uses the event's session to finish every CDP initialization command
first, then admits the Page, complete frame tree, and ready transition in one
topology/runtime/route transaction. Thus an initialization or tree-validation
failure creates no internal or externally visible Page object and emits no
`PageReady`. Explicitly admitted Page and OOPIF routes use the before/after
synchronization pass and are initialized before `receive_once` returns. A later
caller retry repeats idempotent setup where a route still exists. Detached or
rebound sessions are removed from the retained table.

All updates committed before, during, and after one received event are combined
in order into its single return value. Callers therefore cannot lose
`PageReady` or `PopupLinked` merely because those effects arose during session
synchronization rather than direct event decoding.

Recursive auto-attach is required because Chromium reports cross-process child
Frames on their own target-local sessions. It does not imply browser discovery,
browser launch, or ownership of Chrome.

## Event normalization

### Optional URL projection

`with_frame_url_observer` selects a synchronous, caller-budgeted
`ChromiumFrameUrlObserver` before initial snapshot initialization. It receives
initial Frame-tree URLs, new-document URLs plus fragments, and same-document
URLs on this driver's existing ordered event cursor. It installs no task,
subscription or default URL table. Callbacks must not reenter topology.

Projection and retirement run under the topology lock after each applicable
commit. `CdpTopology::frame_session_id` resolves the authoritative renderer even
before worlds exist, following local descendants to their Main/remote ancestor
and returning no session during a swap gap. Stale or different-session URL
events are ignored. Observer errors propagate after commit; the caller must
treat them as a failed snapshot, not assume rollback. The Node host closes and
drains on this error. Retained wrapper metadata has an independent budget and
does not retain live routes (see the Node FFI design).

`receive_once_or_signal(runtime, signal)` allows a caller-owned wakeup only
while waiting for the next event. It does not cancel in-progress initialization.
The caller may then use the bounded `initialize_snapshot` drain as a
command-return fence. A signal alone neither consumes a queued event nor
asserts that the snapshot is fresh. Ordinary `receive_once` uses a never-ready
signal and retains its original behavior.

Release tests cover projection from initial fragments, stable portable identity
on main-Frame replacement, signal/drain ordering, retired-session rejection,
active-entry retirement and observer error propagation. The pure topology's
renderer-session lookup also has a local-under-OOPIF/swap-gap regression. The
current composition suite passes 194 tests with fourteen existing live tests
left opt-in; the separate Node differential lane passed 26 cases in three runs.
The topology-only package closure remains five Pwright crates.

### Native events

Generated CDP event types are the decode boundary. The driver recognizes:

| CDP event | Topology transition |
| --- | --- |
| `Page.frameAttached` | attach or rebind a local child Frame under the reporting Page |
| `Page.frameDetached` | apply exact `remove` or `swap` semantics |
| root `Page.frameNavigated` | rebind the stable main-Frame object to its new native ID |
| `Page.frameNavigated` / `Page.navigatedWithinDocument` with URL observer | project the committed URL from the authoritative Frame session |
| `Runtime.executionContextCreated` | attach only a main or named Pwright utility world |
| `Runtime.executionContextDestroyed` | remove the exact session/context pair |
| `Runtime.executionContextsCleared` | remove every world owned by that exact session |
| iframe `Target.attachedToTarget` | attach or rebind an OOPIF Frame to its child session |
| browser-root Page `Target.attachedToTarget` | admit an existing/new Page into its exact or default Context |
| `Target.detachedFromTarget` | retire a Page session or put an OOPIF into detach synchronization |
| Page `Target.targetCrashed` | publish the typed Page crash effect |

Unknown event methods and events from already-retired sessions are ignored.
Malformed recognized events fail explicitly. A child-Frame navigation event is
not treated as a second attachment source: local attachment and OOPIF target
events own host transitions, while execution-context events own document-world
replacement.

Main-world and utility-world routes are enumerated separately. Portable
`Frame.evaluate` binds only the main world. When the utility profile is
selected, locator service publication waits for each Frame's matching utility
event, then selector installation and actionability evaluation use that
Frame's exact target-local session and `uniqueContextId`. Page calls delegate
to the main Frame route; local child Frames and OOPIF Frames keep independent
routes. Page-authored isolated worlds with any other name are ignored.

## Atomicity and races

Every mutation uses `transact_topology_update`. It stages a complete production
topology shadow, derives Page plus Frame evaluation and locator routes from that
candidate, applies one runtime lifecycle batch, and only then publishes
topology and routes. A failure before runtime admission leaves all live views
unchanged. A destroyed selected world removes its locator route rather than
falling back to another document; its replacement receives a fresh
selector-runtime installation cell.

Local-to-OOPIF swaps preserve the portable Frame `ObjectId`. The new main world
replaces the previous same-kind route even though it belongs to a different
session. A late destruction event for the old session/context pair therefore
cannot remove the new route. OOPIF target detach removes its worlds and requests
Frame-detach synchronization; the later Page `frameDetached(remove)` owns final
portable disposal.

The driver is bound to one `ConnectionEpoch`. Once topology begins reconnecting
under another epoch, the old driver rejects work before consuming another
event. A new connection must construct a new driver and perform explicit state
resynchronization.

Cross-WebSocket rebinding is deliberately forbidden. The local Playwright
source establishes native disconnect as terminal: Chromium
`CRConnection._onClose` disposes the root protocol session,
`CRBrowser._didDisconnect` closes all Pages and workers, the shared Browser
base closes Contexts and emits `disconnected`, and `BrowserDispatcher` closes
then disposes the public channel object. Pwright retains each `CdpSession` on
its exact concrete `CdpConnection` and follows the same rule.

Because topology-events are optional, closure detection cannot live only in
the topology driver. `CdpConnection::closed()` returns an owned future over its
shutdown token without retaining the connection. The attached composition can
create `ChromiumCdpLivenessBackend`, a task-free supervisor backend lane that
turns closure into a sticky terminal error. `ConcurrentServerBackendChain`
combines it fairly with the optional topology backend and forwards the final
termination reason to both, which drops any retained topology turn.
`AttachedChromiumComposition::server_backend` is the canonical constructor, so
native liveness is present even when the supplied plugin is `NoServerBackend`.
A new CDP attachment gets a new Browser graph and new object identities.

For event loss on a still-live CDP transport, the same driver exposes
`resynchronize_after_event_loss(runtime, new_epoch)`. It is valid only after a
reported lag and requires browser-root discovery. The operation disables
auto-attach, waits for Chromium to detach the old sessions, replaces the
broadcast subscription, begins a strictly newer topology epoch, and re-enables
auto-attach. Chromium then republishes all existing targets. The browser-root
`Target.getTargetInfo` response is the snapshot barrier: every earlier
attachment is drained and initialized before `finish_reconnect` disposes
Contexts, Pages, and non-main Frames absent from the snapshot. Surviving native
target identities retain their portable ObjectIds while receiving new CDP
sessions. Main and utility worlds are rebuilt from new Runtime events.

The reconstruction remains caller-driven and returns one ordered update from
`ReconnectStarted` through the replayed `PageReady`/`PopupLinked` effects to
`ReconnectFinished`. A failed reconstruction remains in the sticky resync
state and can only be retried with another newer epoch; ordinary receive cannot
silently resume from a partial snapshot.

## Backpressure and memory

The underlying CDP connection owns a bounded broadcast ring and exact event
item/byte leases. Its production profile includes one extra transient event:
Tokio allocates the incoming event before a full ring evicts its oldest slot.
Broadcast lag is never interpreted as "skip and continue": `receive_once`
returns `ResyncRequired { skipped }` without applying the next visible event,
and repeats that error without consuming anything until explicit
reconstruction succeeds. A missing detach or world-clear event can therefore
never be silently skipped.

The driver profile is derived from the exact connection and topology profiles
and is rejected if those profiles drift. Its one fixed `BackendStateBytes`
lease covers:

- the initialized-session table;
- bounded non-default Context identities and one observed default native ID;
- one complete Page/OOPIF session-route snapshot;
- normalized frame attachments and duplicate-detection identities;
- one temporary target-local client during initialization; and
- conservative generated-event and frame-tree-response decoding overlap based
  on the connection's maximum event and response bytes.

Session count, native-ID bytes, event bytes, resource scope, and profile
identity are all validated before state is retained.

## Complete application recipe

`ChromiumAllCapabilitiesApplicationInstaller` now chooses and statically chains
every currently implemented command, activated control, typed portable event,
topology, PageLoad, and native-liveness plugin after the consuming
same-supervisor handshake. It requires every component to have been selected
and negotiated rather than degrading to a smaller recipe. PageLoad has an
independent broadcast cursor so it cannot steal native events from topology
ingestion, and both backends remain caller-polled without spawning.

The Page-event portion is no longer constructed inside the aggregate. The
independent `ChromiumPageEventsInstallation` accepts the caller's cursor,
adapter, shared topology, transport, exact profile, and production ledger, then
returns the topology publisher, caller-polled PageLoad backend, and three
separately bounded portable outbounds. Its ordered creation fence remains the
only edge between discovery mutation and publication. The aggregate converts
to this profile only when all three Page-event lanes are selected with the same
wire profile, and compatibility-reexports the leaf types without becoming an
owner of their implementation. Release tests cover exact retained-byte bounds,
lane independence, incomplete selections, and mixed-wire rejection; the
complete recipe's four-Page Chrome 152 test passed three consecutive runs after
the migration.

## Explicit remaining work

- Firefox/WebKit or WebDriver BiDi event adapters; and
- memory/latency soak measurements under navigation and iframe churn.

These remain separate design and capability work. The event driver reconstructs
the selected Page topology on its current live Chromium connection; it does not
yet claim transparent transport reconnection.

## Portable discovery-event bridge

`pwright.page-events` version 1.2 now defines fixed-size portable
`BrowserContextPage` and `PagePopup` events. The former maps `PageReady` to its
Context source; the latter maps `PopupLinked` to its opener source. Both carry
the discovered Page `ObjectId`, preserving the stable topology identity rather
than leaking a CDP target or session ID.

The composition root can install either bounded typed sender independently and
pass it to `ChromiumTopologyPageEventPublisher`. No sender means no event queue.
RejectNewest and DropOldest loss are both terminal publication results that
require the caller to close or reconstruct its event view. This is deliberately
not hidden inside the topology driver: topology mutation, capability
installation, codec choice, transport I/O, and executor ownership remain
separate components.

For remote delivery, composition rejects these discovery-event profiles unless
ordered lifecycle control and Page discovery are present. The connection owner
uses `publish_after_creation_fence` to scan the same update, send ordered
control through the final required initializer, and only then enqueue the
events. Existing objects reconstructed without a new `Create` bypass that
fence. Invoking the bridge and selecting connection shutdown/resync behavior
remain explicit composition choices.

`ChromiumTopologyConnectionPump` now combines one driver turn, the ordered
creation fence, publication, and loss policy without spawning. Same-connection
reconstruction has a nonzero maximum-attempt bound and advances via checked
epochs. It is available only for an inert portable publisher. When discovery
callbacks are installed, CDP loss requires a new client connection so replay
cannot duplicate callbacks whose prior delivery is unknowable. Publication
loss and a dropped polled pump future are likewise terminal. The pump can then
terminate the server supervisor under `BackendEventFailed`, which performs
reasoned control cleanup and transport shutdown.

`ChromiumTopologyServerBackend` retains one pending pump turn and implements
the supervisor's optional backend lane. The same caller-owned supervisor now
rotates its unique remote transport reader, bounded command futures, serial
outbound delivery, and this backend lane. A ready lane advances the starting
priority to the next lane, preventing a target-event flood from starving
commands or lifecycle output. Terminal backend failure uses
`BackendEventFailed`; the complete application-run method then shuts down the
write half and reports both the primary and shutdown failures if necessary.
No `select` outside the supervisor can accidentally cancel its reader or the
topology turn. EOF, failure, or dropping the complete run invokes the backend
termination hook and releases the retained turn; no task is spawned.

## Evidence

Release-profile fake tests prove default Context uniqueness, complete initial
frame-tree publication, existing/new Page admission, popup ready/link ordering,
foreign-context rejection, failed-initialization gating, local Frame/world publication, recursive OOPIF
session initialization, local↔OOPIF identity preservation, harmless late world
destruction, two-stage OOPIF removal, sticky broadcast-lag failure,
authoritative Context/Page/Frame reconciliation, portable-ID preservation,
post-recovery event consumption, and stale-epoch rejection. Strict Clippy
passes with warnings denied.

A dedicated Chrome 152 test uses a `serve-lib` cross-site fixture and the
production `AttachedChromiumComposition`. No test code parses or manually
applies the child target/world events. Repeated caller-owned `receive_once`
calls discover the OOPIF, initialize its exact session, publish its main-world
`uniqueContextId`, and successfully evaluate `location.hostname` through the
portable child `Frame` service. Cleanup removes the temporary context while the
externally owned Chrome remains alive.

Current Chrome reports the root of an OOPIF session's `Page.getFrameTree` with
the embedding Frame as `parentId`. Validation accepts this only when the route
is a non-Page target session; a Page session root with any parent remains a
typed failure. Positive OOPIF and negative Page-root AAA tests freeze that
distinction, and the cross-site Chrome proof exercises the real parent-bearing
shape.

The utility-world Chrome 152 proof additionally creates a same-process iframe
whose `#inside` button does not exist in the main document. It waits for the
child utility route and completes a trial click through the child Frame's
portable object ID. Unit proofs cover shared Page/main-Frame routing, distinct
local-frame and OOPIF sessions, fail-closed world removal, and fresh runtime
installation after world replacement.

A second repeated Chrome 152 test selects browser-root discovery, creates a
default-context target before auto-attach, and performs no manual topology
setup. It proves the opaque default-context ID classification, existing Page
initialization, caller-owned concurrent popup unpause, ordered popup linkage,
and then produces 1,100 Runtime events against a 1,024-slot ring. The resulting
lag is recovered twice by real auto-attach re-publication: opener and popup keep
their ObjectIds, receive new sessions, clean up exactly, and leave the external
browser alive.

The fresh-remote-connection Chrome 152 checks use the reusable attachment
factory and a production-ledger-aware transport builder. One transition
installs a new runtime containing only its Browser root while the supervisor
remains handshake-first. A second selected topology profile is claimed by the
remote seed, and the attached composition rejects another driver rather than
creating a competing event baseline. A third check proves a runtime-installer
plugin that returns without constructing the graph cannot enter the final
connection state. Four post-handshake checks then prove that acceptance comes
from the stored supervisor, a negotiated navigation handler can be built from
the accepted context, an unconsumed selected topology driver rejects assembly,
the transferred driver remains the unique baseline, mandatory CDP liveness
wraps even a no-op backend, and EOF shuts down exactly the transport stored in
that connection once.
