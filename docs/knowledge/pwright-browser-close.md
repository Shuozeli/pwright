<!-- agent-updated: 2026-10-10T08:05:07Z -->

Owned host shutdown one-flight checkpoint (2026-10-10): on first
`HostSession::close`, the unique process lease moves to one detached Tokio
shutdown task. Concurrent callers await the same retained provider report,
and canceling one caller cannot abandon the started shutdown. Two fake-future
release tests, all-feature release check, rustfmt and diff checks pass. The
retained report proves only the direct child waited/reaped; it is not yet
exposed to Node/Python or conformance provenance. Chrome descendants,
pre-close-drop reaping and temporary-profile deletion remain unproven.

Owned Chromium launch lease guard (2026-10-10): the launch provider still
returns a raw successful lease with kill-on-drop disabled. The optional launch
composition now arms that lease synchronously as it takes ownership, before
binding the production CDP connection. Dropping an unbound or abandoned
composition lease requests process termination; cancelling a pending
bind-failure cleanup retains the same guard. Two fake-process release tests
cover takeover and cleanup cancellation without starting a browser. The guard
targets the spawned child; descendant process-tree supervision remains a
separate launcher prerequisite.

NODE-130: Deletion-only Context routing can prune exact local Page/Frame/provider and subscription membership while preserving unrelated Contexts. It does not mint native world destruction or implement terminal shutdown; exact native receipt marking and full close/terminal publication remain coordinator responsibilities.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Owner interruption or peer loss does not discard scheduler-owned written Page/Context commands. They drain through reply/transport failure; stop prevents successful evidence validation. Host/Worker teardown integration remains outstanding and no installed Node terminal behavior changes yet.
[Bounded native scheduler contract and evidence](pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Native ancestor authority retains written Context-command ownership through interruption/peer loss and refuses proof after owner/epoch invalidation. Certification is bound to one attachment domain and cannot reuse another connection's matching native strings. Node's existing terminal/event behavior is unchanged.
[Native ancestor authority contract and evidence](pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Split native command work preserves written-reply ownership through caller interruption. Stop during discovery prevents later target mutation; stop after target write is observed after its reply. Transport loss in either phase fails without claiming destruction and drains all tested native ownership. Node's serial scheduler remains unchanged.
[Native close phase contract and evidence](pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred publication admission/preparation rejects a terminated owner; dropping capacity returns unconsumed groups. Short commit failures still fail closed and preserve terminal handoff. This is storage behavior, not the missing concurrent native ancestor-close dispatch.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: The optional runtime reservation layer checks transport termination before pending/byte admission, invalidates waiting/prepared disposal on shutdown and reclaims outbox/lease ownership without retaining a dropped runtime. This was verified with a deterministic full-pool/closed-transport red/green gate. It is not selected in Node yet and does not resolve the pending Page/ancestor Context dispatch gap.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-123: An in-flight owned/external Context.close interrupted by host termination waits outside its operation token for terminal staging and preserves TargetClosed, unlike Page.close suppression. Already-retired Contexts remain idempotent. Reply-producer/request-queue loss now seals admission immediately, closing a later reproduced pre-stop settlement window. Ordinary closed-object replies and full queues leave live connections intact. Final owner/peer repeats pass in both profiles; ancestor Context dispatch behind a waiting Page close remains a controlled red gate.
[Close cascade contracts and evidence](pwright-close-cascade.md).

NODE-122: A selected native Page-close receipt wait observes the same host stop token used by Browser/Worker teardown. Written command responses remain owned; subsequent owner loss rolls back the unconfirmed transaction and releases cleanup ownership. Interrupted public Page.close waits outside its operation token for terminal staging, preventing successful settlement from deleting the emitter before Page.close delivery. Both final artifacts pass paired owner-close/peer-loss, held-receipt Worker and retained-driver attribution cases. Frame arrival-boundary close still has an explicit unresolved diagnostic failure.
[Native confirmation contract and evidence](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

## NODE-121: Transparent original-scenario tracing

The NODE-120 synthetic response hold proved attribution, not natural failure
causality. The new `scripts/probe-listener-native-close.mjs` runs the **original**
isolated mutation and close-reentry scenarios through the existing diagnostic
loopback relay. Their event handlers, 5000 ms connection/command budget,
14-second child limit and cleanup sequence are unchanged. No response or event
is delayed, suppressed or manufactured.

A test-only `listenerEndpoint` selector accepts either the normal canonical CDP
endpoint or an explicit bare `ws://127.0.0.1:<port>` relay, while still requiring
the canonical `PWRIGHT_TEST_CDP=http://127.0.0.1:9222`. Non-loopback addresses,
credentials, paths, alternate protocols and invalid ports reject. Production
package endpoint selection is not changed.

The shared `test/native-close-trace.mjs` observer now serves both diagnostic
scripts. Its caller supplies monotonic time. It retains at most 32 pending
commands, 32 session-to-target identities, eight created targets, 128 close
receipts, a ring of the newest 128 completed commands and 64 native
detach/destroy receipts per connection. Detach correlates exact inner session
identity; target destruction uses known session-to-target mappings. Neither
receipt fabricates a command response. Overflow fails explicitly. Wire buffers
are forwarded unchanged and browser event payloads/URLs are not retained.

Each child has bounded 64 KiB stdout/stderr and a separate 15-second supervisor.
The first failed child ends the experiment, retaining its stderr and all pending
method/session identities before starting another connection. Owned-target
absence is checked after every child, without closing unrelated Pages. Each
exclusive-create report is capped at 4 MiB and records the native artifact hash.
This is a diagnostic transport, not the production architecture.

### Current evidence

Evidence: `.work/node121-natural-close/`.
Both final profiles complete 16 original-scenario samples (eight mutation and
eight close-reentry), with zero failed children, unanswered native commands or
remaining owned targets. Both full paired listener corpora pass 18/18. The
final Node suite passes 78/78, including eleven new endpoint/identity/boundary
tests; strict types and all package dependency/runtime boundaries pass.

Final reports are `default-original-final.json` and
`drag-frame-extra-headers-original-final.json`; corresponding paired results are
`*-listeners-final.log`. After extracting the observer, the older direct close
probe also passes eight rounds per profile (`*-extracted-trace.json`), retaining
zero settled FFI calls/bytes and no owned target leaks.

No native artifact was rebuilt or changed:
default `91fb3ddbb963d1071ccc81497f5d3e48445f1d4b1dbd11dbe5f773a8d88dd696`;
combined `5e8a35af014451f78aaa3010d57d64b005fe6c3221e5eff37daaf63aa6b852a8`.
The earlier natural failures remain unresolved. Transparent instrumentation can
still affect scheduling, so successful traces do not prove absence of a race.

### Next action changed by pinned-source inspection

The actual installed oracle is Playwright **1.58.2**. Its
`playwright-core/lib/server/page.js:578-590` waits for `_closedPromise` after
ordinary delegate close. Its client `lib/client/page.js:505-517` suppresses
target-closed errors for ordinary close and closes an owned Context when one
exists. The separate upstream clone has newer close code and is **not** used
as the pinned oracle for this conclusion.

The current Rust Node selection instead exhausts 20 target-absence checks with
10 ms inter-check pauses. That is a known contract mismatch, already documented
under NODE-012, irrespective of whether the latest intermittent failure can be
reproduced. Therefore the next implementation is the selected native receipt
confirmation design in [Page close](pwright-page-close.md#node-121-native-close-confirmation-plan),
not another series of unchanged repeats or an arbitrary attempt-count increase.
Keep the independent retained-driver timeout investigation open.

Handoff at 2026-09-10T11:17:51Z: all task-owned test/probe children and relay
listeners have exited. Only the owned preflight target
`7ADBBC9C9275F2890D61C96E23C3C5A6` was explicitly closed; its absence was verified.
One unrelated XLQY Page remains, and the shared Chrome/services were not stopped.
Shared backend PID 53189 RSS changed from 3835392 to 4497536 KiB during this
turn; a later targeted vmmap reports physical footprint 5.0G, lifetime peak
13.6G. These are different measurements of the shared process, not per-chat
attribution. Swap stays zero and memory_pressure reports 70% free. No new commit
or push was made.

## NODE-120: Attribute retained native driver failures

`HostSession::close()` joins the background driver and returns its retained
result. Consequently, a `browser.close` stack frame does **not** prove a close
command timed out. The driver previously returned the same unqualified
`Timeout waiting for response` for failures in unrelated phases.

The private Rust `driver_diagnostics::Phase` now annotates backend failures at
the topology receive/snapshot boundary and driver/peer transport/mouse
retirement/owned-session retirement/transport-close aggregation boundaries.
Existing error precedence, complete teardown, CDP deadlines, native close
confirmation attempts and error codes are unchanged. Non-backend variants keep
their structured values and messages. Backend diagnostics remain UTF-8-valid
and capped at the existing 4096-byte limit, including phase prefixes.
This adds no steady-state logger, event queue, callback, dependency or runtime.

### Deterministic attribution evidence

`packages/pwright/test/driver-close-attribution.test.mjs`, available through
`pnpm --dir packages/pwright run test:driver-close-attribution`, uses the
existing loopback-only diagnostic CDP relay. After a real owned Page is created,
it withholds exactly one **successful Chrome Page.enable response** during the
driver's initialization snapshot. It does not change production timeouts or
pretend that Chrome failed to execute that command.

The existing 5000 ms response deadline expires. The pending newPage call loses
its driver reply and reports TargetClosed; joining the driver subsequently
reports:
```
browser.close: native driver: topology snapshot: Timeout waiting for response
```
Both rebuilt profiles pass this check, including zero active host leases,
cleanup-in-flight, settled FFI pending calls and reserved bytes. This proves
failure attribution and cleanup for the injected scenario, **not the cause or
repair of NODE-119's naturally occurring mutation cleanup timeout**.

### Independent native close-confirmation failure

The initial filtered default listener run in this turn reproduced
`page.close: Backend: CDP target ... remained open after Target.closeTarget`
in close-reentry. Its evidence remains in
`.work/node120-close-attribution/default-listeners-1.log`.
The Page-close service issues Target.closeTarget, then at most 20 GetTargets
checks with the selected 10 ms pause between checks. That bounded absence
check can fail independently of a retained driver-response timeout.
No attempt count, polling pause, response deadline or cleanup policy was relaxed.

`scripts/probe-native-close.mjs` captures only bounded command identities,
close/disposal/getTargets receipts and exact owned-target absence. The test
relay forwards all traffic unchanged. There are at most 32 pending identities,
128 close receipts, eight created targets per round and eight sequential
rounds. Reports are exclusive-create, at most 1 MiB and include native artifact
SHA-256. Only task-created targets are inventoried for cleanup; shared browser
targets are not closed. Reports distinguish the immediate FFI watch snapshot
from its bounded environment-turn drain. This diagnostic is not a parity test
or a claim that intermittent failure is repaired.

Initial trace runs passed eight reentrant-close rounds per profile, with no
unanswered commands or remaining owned targets. Full listener corpora then
passed 18/18 in both profiles. These successes do not erase the earlier failure
or establish a universally safe close-confirmation budget.

### Verification and remaining work

Evidence directory: `.work/node120-close-attribution/`.
Default native artifact:
`91fb3ddbb963d1071ccc81497f5d3e48445f1d4b1dbd11dbe5f773a8d88dd696`.
Combined drag/frame-navigation/extra-headers artifact:
`5e8a35af014451f78aaa3010d57d64b005fe6c3221e5eff37daaf63aa6b852a8`.

Rust host tests: default 393 passed / 8 ignored; combined 441 passed / 10
ignored. Three new deterministic unit tests cover phase, classification and
UTF-8 byte bounds. Strict host/addon all-target combined Clippy, Node 67 unit
tests, strict types and package dependency/runtime boundary checks pass.
Development harness failures remain in the evidence directory: an incorrect
initial assumption that disposal snapshots always issue GetTargets, an
incorrect pendingCalls metric name, and a combined diagnostic started before
the rebuilt addon was installed. They are not counted as final verification.

Final supervised gates additionally pass lifecycle events 7/7 and injected
attribution 1/1 per profile. Final unchanged-traffic close reports are
`default-close-final.json` and `drag-frame-extra-headers-close-final.json`:
eight rounds each, no errors, no unanswered commands, no owned targets left,
zero active host leases/cleanup and settled FFI pending/reserved bytes. Earlier
immediate snapshots sometimes contained one queued FFI watch; final probes
retain that pre-drain observation and separately assert the actual drained state.

Handoff at 2026-09-10T11:03:49Z: all task build/test/probe processes have exited;
the exact preflight target `264349911C0B321BE78661008861EB0C` was closed and its
absence verified. No new daemon/server remains. Shared Chrome, its three XLQY
Pages and existing services are untouched. The shared Codex backend PID 53189
RSS was 519856 KiB (start 3184256 KiB), with swap still zero; this is shared
process RSS, not exact per-chat memory or a physical-footprint measurement.

Next: obtain a bounded identity trace of the **natural** mutation cleanup
timeout, and an isolated native close-confirmation exhaustion trace. Identify
the unanswered method/session or the target's actual disappearance timing
before changing ownership, event handling or the confirmation contract.
Broader Node compatibility remains unfinished. No new commit or push.

NODE-119: Ready-source state composes with the existing terminal watermark and planned masks: queued ready precedes its Page close, invisible unready Pages are not announced during teardown, and dependent popup suppression cannot block independent closes. Browser/Worker cleanup regression evidence remains required; external Chrome is never owned.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: No new Browser/runtime ownership is retained by creation conversion. The same weak Browser callback, exclusive publication port and fail-closed conversion handling remain in use. Close callbacks retain their existing identity-only path; terminal/Worker regression coverage is retained.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Terminal capacity remains independent of external-close reservations. Terminal handoff consumes exact masks left by external groups, without re-showing or re-closing previously staged Pages. Browser connection and terminal Worker/transport cleanup regressions pass in both profiles; shared Chrome is not shut down.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Terminal teardown preserves the exact planned view left by an explicit-close transaction, including groups not yet dispatched. It neither replays already-staged Page-close groups nor restores their visibility before disconnected readiness.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Actual Node Browser.close/session termination now publishes Page and Context close listeners before disconnected readiness. Native state commits precede callbacks, and each independent notice is acknowledged even when a synchronous listener throws. The attached external Chrome process is not closed.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Playwright-aligned Browser close and disconnect design

NODE-114: The experimental native terminal path now produces independent Page/Context groups before disconnected readiness, without issuing Browser.close or restarting external Chrome. Real-Chrome tests validate native group publication; Node EventEmitter delivery remains separate work.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Reference tests confirm that a throwing Page close listener does not suppress sibling/Context/disconnected events; throwing Context close also leaves disconnected intact. The Rust group skip policy matches this distinction. Saturation-safe terminal producer integration remains pending.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Explicit native Page/Context close can now report pre-admitted commit receipts.
Browser terminal teardown is not routed through that decorator and still needs
its own admitted cause input. Preserve the existing one-shot disconnected and
listener exception contracts.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The borrowed driver-effect hook does not establish terminal Browser teardown
cause coverage. Keep existing disconnected publication unchanged; a future
terminal event group needs its own pre-admitted native input and unlocked
delivery rather than final collection differencing.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 fixes the already supported disconnected event's exception channel:
synchronous listener failures report the same thrown value as uncaughtException,
not an API/watch rejection; Browser.close still resolves. Error-handler reentry
retains one-shot delivery, and async rejecting listeners keep their separate
unhandledRejection behavior. Normal-disconnect regressions are recorded below.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109's terminal drain publishes admitted metadata changes first, and only an
explicit Complete clears collections and sets DisconnectReady. Published is not
a terminal-completion signal. The existing publish_disconnect transition still
owns the final public connected flag; Node has not switched to stepped delivery.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-107 enables a view with retired members hidden to rebase to an admitted
empty revision without dropping old reader-held metadata. Queue/domain ownership
is checked independently of connection liveness. Terminal full-fence behavior
and missing Page/Context close notifications remain unchanged.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 supplies immutable masked collection views for future stepwise close
publication, but terminal fences still publish their complete state. Hiding a
Context is rejected while any of its Pages is visible; this transition does not
emit close/disconnected or alter native ownership. Event dispatch remains open.
[Representation and verification](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 verifies Page-close then Context-close callbacks before Browser
disconnected for owned close and borrowed disconnect. Earlier callbacks still
see connected true; disconnected sees false. Candidate keeps the correct
disconnected notification/final states but misses Page/Context callbacks.
Native borrowed Pages survive. No production behavior changed in this red gate.
[Stepwise publication requirements](pwright-node-lifecycle-events.md).

NODE-103 connects Rust collection membership to the existing terminal fence.
Ordinary batches hold Page-close publication until coherent membership commits;
terminal teardown opens an abandoned gate and clears current membership before
Browser disconnect readiness. Borrowed native Pages remain outside cleanup ownership.
Default/combined Browser gates pass 5/5 and collection gates 7/7, resolving the
specific early-removal cases recorded below, not all close/crash event ordering.
[Producer contract and evidence](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).

NODE-102's Rust collection storage releases its current revision at terminal
MAX publication before Browser disconnect readiness, without new allocation.
Discard and old-fence tests pass. This does not yet connect native collection
producers/getters or fix NODE-101's six early-removal observations.
[Storage scope and remaining integration](pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 confirms the next collection audit is RED: contexts/pages disappear
before Node publication in six default/combined cases even while the NODE-100
Browser getter remains true. Final states agree. The Rust scalar fix is retained;
collection membership needs a coherent Rust-published projection.
[Evidence and implementation gate](pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

## NODE-100: Rust-owned Browser disconnect publication

The pinned Playwright 1.58.2 client sets its local connected flag to false in
Browser._didClose immediately before emitting disconnected (installed
tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/client/browser.js, getter at line 115,
event handler at line 153). Native transport retirement alone is not public
event dispatch. The previous Rust-backed getter exposed native cancellation
early: the new five-case differential baseline passed 2/5, failing pending close,
concurrent close and Worker-confirmed transport drop while Node was blocked.

The existing Rust metadata state now owns one monotonic ConnectionPublication:
Open -> DisconnectReady -> Disconnected. A terminal MAX publication fence first
publishes retained Page closures and marks DisconnectReady. It must not itself
set the public Browser flag: an unrelated operation completion, or a watch that
still returns connected=true, can deliver that fence. Immediately before the
JavaScript facade emits disconnected, NativeSession.publishDisconnect calls the
Rust event port. Rust validates readiness, transitions once, and returns false
on duplicate dispatch. An older fence cannot authorize dispatch or reopen state.
Environment discard rejects dispatch and never advances readiness.

NativeSession.isConnected reads the published state synchronously. Native
HostSession.is_connected, command admission, cleanup and pwrightMetrics().connected
continue to reflect native termination promptly. Thus the diagnostic metric may
be false while the public getter remains true until Node dispatch. No JavaScript
lifecycle flag is installed for rebuilt artifacts; JavaScript only forwards the
one-shot port and emits the existing event. Legacy artifacts retain the previous
JS duplicate guard and premature native getter behavior: rebuild to obtain this
fix. Default and drag-frame-extra-headers were rebuilt; other local profiles are
not claimed current.

### Resource and verification boundary

One enum is embedded in existing startup metadata; there is no new queue,
subscriber, task, cursor, per-call lease, transport, feature or dependency.
The existing terminal Page permits and fixed publication fences are unchanged.
The combined Page enumeration report still shows the fixed tree/index/queue
charge of 31,248 bytes and one state, with host active leases, cleanup, operations,
objects, scopes, pending calls, topology drivers and retained event bytes all zero.
URL entries/bytes, active Frames and pending publications are also zero.
These are logical Rust resource counters, not total process RAM.

Seven deterministic Rust cases cover native-only teardown, terminal Page
publication without Browser dispatch, older empty fences, duplicate/old-fence
monotonicity, ordinary Page close, ready-state discard and discarded terminal
fences. The new test:browser-connection lane compares upstream first against
explicit expected observations for pending close, concurrent close, Context-only
close, independent Worker transport drop and live-transport control.
A test-only Worker forwards real WebSocket traffic, closes only its relay,
confirms both socket closures, and signals a shared integer while Node is blocked.
The candidate separately polls native cleanup metrics with bounded synchronous
waits; this is not the getter under test. Borrowed native Pages remain evaluable
after subject disconnect. The live control evaluates through the relay.
The ordinary upstream close probe blocks 100 ms; only the Worker probe independently
confirms transport retirement. These are not exhaustive lifecycle interleavings.

Each case has a 30-second limit; Worker confirmation and diagnostic awaits are
bounded at 10 seconds, native polling at 5 seconds. The existing supervisor caps
the lane at 180 seconds with 4 MiB stdout and 1 MiB stderr. No HTTP fixture or
production driver is added.

Evidence: .work/node100-browser-connection/ (local, not release artifacts).

- Baseline: 2/5, three premature false observations; 17.257 seconds.
- Final default: Browser connection 5/5, 8.086 seconds; Page.isClosed 8/8,
  13.892 seconds.
- Final combined: Browser connection 5/5, 8.181 seconds; Page.isClosed 8/8,
  15.119 seconds; Page enumeration/reclamation 7/7.
- Rust host default: 245 passed, 7 ignored; combined: 293 passed, 9 ignored.
- Strict combined host/addon Clippy, Node units 47/47, strict types, and resolved
  dependency boundary checks (7 host and 8 addon selections) passed.
- Final cargo fmt --all --check, git diff --check and both new Node script
  syntax checks passed. HEAD remains f76e3f2 and the staging index is empty.
- Default SHA-256: 2bf67640c21ebcf951d842c71965224ff3e8501526aa500c2d0c5871bd470c0d.
- Combined SHA-256: df23f52e832149381f644a0d401dd08c19f0940b81f753a6f2e69acffc141d6f;
  page-enumeration.json names this exact artifact.

Both test command chains terminated successfully. All 16 recorded relay ports
had no listeners after completion; no task test/build/Worker processes remained.
Only the exact task preflight target FCE5F650C973E030B9413716AD8EC925 was closed.
Shared Chrome and unrelated XLQY Pages/services were left running; no fixture
registration was created. At 02:50:06 UTC the shared Codex backend PID 53189 RSS
was 2,014,176 KiB and shared Chrome PID 54462 RSS was 984,336 KiB; swap was zero.
The backend fluctuated from 4,031,376 KiB at 02:49:11; a separate vmmap sample
reported 1.6 GiB physical footprint, which is not the same metric or instant.
This does not establish per-chat RAM or a leak.

Next characterize Browser.contexts() and BrowserContext.pages() at the same
publication boundary. Their native-index reads are a source-based suspicion,
not a reproduced discrepancy. Page close/crash events, before-unload true,
close reasons and broader Node API parity remain open. No staging, new commit
or push; the full goal remains active.

The NODE-099 Browser getter audit request below is historical and is resolved
only within the bounded NODE-100 scope above.

NODE-099 stages all retained Page-close publications before the host disconnect
signal. Existing Node completion/watch fences publish the terminal batch, including
when normal metadata admission is full; environment destruction discards it.
Page.isClosed therefore closes the old borrowed wrapper without destroying the
external Page. The existing Browser.isConnected native-state getter still needs
its own blocked-event-loop audit; this checkpoint does not claim all Browser
state/event ordering is compatible.
[Scope and verification](pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-097 separates an already connected peer's shutdown (four passing controls)
from a new peer attaching during paused child initialization. Page-level native
auto-attach can retire that child even for Playwright itself; both owners retain
a usable main Page and recover after the peer disconnects. The direct matrix
passes 11/11 combined/default, with final per-Browser and environment cleanup.
This does not change close authority or prove full shutdown/Worker semantics.
[Evidence](pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-095 attributes some disconnect timeouts to initialization calls still
awaiting replies after their exact session detached. Selected transport retirement
and typed target-local errors address this boundary; full disconnect parity is open.
[Contract and evidence](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 reproduces a root-detach failure from queued disposed-Context Pages.
The fix removes erroneous Context rejection in selected Playwright mode; it
does not relax raw detach errors or native-world retirement receipts. The
live control checks borrowed Context survival after test-browser disconnect.
[Evidence](pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-092 accepts an exact already-detached child receipt in the existing Rust
resume/detach helper. Test-browser disconnect must preserve its controller's
borrowed Page and release candidate owners. Other disconnect failures, including
the NODE-091 concurrent timeout, remain open. [Scope](pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-088 header jobs own primary siblings and compensation after early public
failure. Normal completion drains both; host shutdown/peer loss drops jobs before
transport retirement and records unresolved native uncertainty. This must not be
reported as proof that already-written native commands were canceled or undone.
Controlled native/foreign-waker retirement during shutdown remains required.
[Current boundary](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

> Status: the owned launch/composition/shutdown boundary, portable remote
> `Browser.close`, bounded close-reason propagation, disconnected-event
> contract, ordered composition wiring, and exact-session authorization are
> implemented.

## Upstream behavior

The source baseline is Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`.

Playwright presents one public `browser.close({ reason })` method, but its
effect depends on how the Browser was acquired:

- a normally launched Browser calls the server Browser's process `close`,
  which attempts graceful browser shutdown, waits for disconnection, and may
  force-kill the owned process after a timeout;
- Chromium `connectOverCDP` installs a synthetic browser-process adapter whose
  `close` only closes and waits for that CDP transport, so it does not terminate
  the externally owned Chrome process;
- Playwright's remote `connect` client marks the Browser so local close closes
  the client connection. The server-side pre-launched Browser dispatcher has
  `ignoreStopAndKill`, preventing that client from terminating the shared
  browser;
- browser disconnection closes descendant contexts/pages and emits the public
  disconnected event whether shutdown or transport loss caused it.

The detailed terminal order is also observable. The client stores the supplied
close reason before sending `close`, then awaits its disconnected promise. On
the server, the first non-empty reason becomes the Browser's terminal reason.
`Browser.didClose()` closes Page descendants and Contexts before emitting the
Browser disconnected event. Dispatchers publish each close event before
disposing that channel object, ending with Browser close followed by Browser
dispose. The reason is diagnostic context inherited by descendant operations;
it is not an argument to the native Chromium shutdown provider.

Relevant anchors are:

- `packages/playwright-core/src/client/browser.ts` (`Browser.close`,
  `_shouldCloseConnectionOnClose`, `_didClose`);
- `packages/playwright-core/src/client/connect.ts` (remote connection close);
- `packages/playwright-core/src/server/browser.ts` (`Browser._close`,
  `didClose`);
- `packages/playwright-core/src/server/browserType.ts` (`browserProcess`,
  graceful-close/kill escalation);
- `packages/playwright-core/src/server/chromium/chromium.ts`
  (`_connectOverCDPImpl` synthetic process adapter);
- `packages/playwright-core/src/server/dispatchers/browserDispatcher.ts`
  (`ignoreStopAndKill`);
- `packages/protocol/spec/browser.yml` (`Browser.close`).

## Pwright decision

The public outcome should remain Playwright-shaped—after `close().await`, that
Browser handle and every descendant are stale—but process authority must remain
structural rather than inferred from a flag.

Pwright therefore separates two mechanisms:

1. **Disconnect an external attachment.** This is connection lifecycle, not a
   browser capability and never sends CDP `Browser.close`. It closes the
   Pwright/CDP connection and invalidates its complete portable object tree.
   Existing graceful connection-control machinery owns remote Close/Ack and
   transport shutdown.
2. **Close an owned launched browser.** This is an optional Browser lifecycle
   capability installed only by a launched-browser composition that has moved
   the non-cloneable `BrowserProcessLease` into one shutdown owner. It consumes
   that authority exactly once and uses the provider's bounded graceful
   close/process-exit/force-kill coordinator. An attach-only composition cannot
   construct or advertise this command.

A later ergonomic facade may expose both as `Browser::close`, selecting the
mechanism from the acquisition type, as Playwright does. The lower layers must
not erase the distinction. There is no generic CDP backend feature called
`browser.close`, because process ownership and escalation belong to the
provider/composition boundary, not to Chromium command mapping.

## Local and remote completion

For an owned launched browser whose Pwright protocol transport is independent
of CDP, the required sequence is:

```text
reserve caller-owned continuation
  -> validate the exact Browser target
  -> prepare complete Browser runtime disposal
  -> claim the session-authorized unique process lease
  -> provider shutdown (Browser.close, observe exit, bounded force escalation)
  -> commit complete Browser runtime disposal
  -> publish descendant Dispose controls through Browser Dispose
  -> publish Result
```

Process exit—not CDP acknowledgement—is the native completion proof. Once the
lease has been claimed, cancellation or future Drop cannot put it back; the
exact shutdown future must transfer to a previously reserved owner. A failure
that leaves the provider kill-on-drop guard active is terminal cleanup state,
not a retry using duplicate authority.

For external attachment disconnect, there is no command Result on the closing
connection. Transport closure is authoritative and the client's unique reader
invalidates the whole Browser tree. If a separate remote Pwright transport
controls an external CDP attachment, closing that client session may dispose
its session-owned contexts before disconnecting, but it still cannot send
CDP `Browser.close`.

Remote authorization is mandatory for owned-process close. The launch leaf
adds `pwright.browser_lifecycle` to its own handshake manifest only when the
caller selects it; the attach-only manifest remains unchanged. After the real
server handshake accepts that capability for one session, converting the owner
into a non-cloneable close grant additionally requires the accepting
supervisor and launched composition to share the same production resource
ledger identity. This is a pointer-identity proof already present at the
connection root—not a random token or global registry. A session with the same
capability, limits, plan, or numeric Browser ID but a different ledger is
foreign; a supervisor without production resources is unbound. A rejected
conversion returns the untouched owner, so none of these failures can orphan
the process. Shared/prelaunched and external-attach services cannot construct
the grant, matching Playwright's `ignoreStopAndKill` safety behavior.

## Application cleanup policy boundary

Graceful application-wide cleanup drain, retry, and reporting remain deployment
policy rather than Browser protocol behavior. The optional Tokio adapter now
implements that policy without changing `Browser.close`:

- shutdown rejects new cleanup admission but honors every reservation accepted
  before the transition and waits for reserved plus running work;
- a drain timeout reports remaining work without cancelling it, allowing a
  later completion or a deployment-selected runtime stop;
- ordinary and reserved cleanup futures are never retried automatically;
- retry requires an explicit finite factory for a fresh attempt, so it is not
  available to the consuming owned-Browser close continuation by accident;
- one capacity permit spans every retry attempt and its delay;
- a synchronous reporter observes admission, retry, job, and shutdown events;
  callback panics are counted and cannot orphan a cleanup job.

A composition test polls owned close until the unique process authority has
moved, drops the command future, starts executor shutdown, then proves the
shutdown wait includes that exact continuation and resolves only after the
Browser runtime tree is empty.

## Implemented ownership and remote-close checkpoint

The optional `pwright-composition-chromium-launch` leaf accepts an explicit
installed-browser launch request and returns an `OwnedChromiumComposition` that
retains the only process lease. Its CDP connection carries a production plan
and ledger-identity proof until the prepared composition consumes it; a foreign
plan or ledger is rejected. The ordinary attach composition cannot call owned
shutdown or construct a remote-close grant, both enforced by compile-fail
tests. Composition takeover arms kill-on-drop immediately after provider
success, before binding. The provider's standalone successful raw lease keeps
its existing drop behavior. On binding failure, the termination future holds
the armed lease across an await, so dropping that future still requests kill.

Owned shutdown borrows the owner rather than consuming it when the future is
created. Dropping an unpolled future therefore leaves process authority intact.
Once polled, it consumes the lease into the provider's bounded graceful-close,
exit-observation, and force-escalation coordinator, whose cancellation guard
prevents an admitted shutdown from orphaning the process.

Abandoned close execution now has two explicit adapters. The standard-library
`CleanupQueue` remains bounded and caller-driven. The optional
`TokioCleanupExecutor` implements the same reservation contract and can apply a
caller-selected nonzero deadline to every submitted job. Reservation capacity
cannot be stolen between command construction and irreversible shutdown. A
deadline drops a stalled shutdown, which activates the owned process
kill-on-drop guard, releases the executor permit, and increments `timed_out`;
Tokio runtime shutdown instead increments `cancelled`. Because dropping the
pending close rolls back its prepared runtime transaction, the native CDP
disconnect lane remains responsible for final descendant-first disposal.

Shutdown diagnostics are now independently pluggable without widening the
portable API. `BrowserCloseResult` remains `()` and the capability, codec,
client, and server crates retain no provider dependency. The launch composition
defaults to `DiscardOwnedBrowserShutdownReport`, but an assembly root may
select an `OwnedBrowserShutdownReportSink<Report>` through
`OwnedBrowserCloseExecution`. It receives the concrete provider report only
after runtime disposal commits. Encoded testing sends a provider-shaped report
to the sink while decoding unit success on the wire; the live Chrome proof
matches the received report's PID and user-data directory to the exact process
that was reaped.

The portable protocol is independently packaged as
`pwright-cap-browser-lifecycle`, `pwright-codec-browser-lifecycle`,
`pwright-client-browser-lifecycle`, and
`pwright-server-browser-lifecycle`. It contains one `browser.close` command
with an optional UTF-8 `reason` and typed authorization/shutdown/cleanup failures.
Timeout and cancellation metadata and wire attachments are rejected because
shutdown continuation is owned by the server rather than a client deadline.
Client close treats an already-stale Browser as idempotent success.

The reason is bounded by the selected Browser-lifecycle codec's string budget.
An empty reason is treated as absent. After native shutdown, one shared
`Arc<str>` terminal failure is installed into every still-pending operation in
the Browser subtree before its child-first disposal wakes those calls. This
avoids a string clone per descendant. Backend cleanup deliberately continues
to receive the structural `ParentClosed` cancellation reason, while callers
receive `TargetClosed` with the supplied diagnostic. Client-side persistence
uses a separate bounded root-family state: `ClientLimits` caps terminal-reason
bytes, every descendant shares one `Arc<str>`, and first non-empty reason wins.
The Rust close future installs it only when first polled and before dispatch;
oversize fails before the channel is touched. After child-first Dispose, a new
operation reports the same reason without keeping the client connection alive.
Production client recipes must set this bound no larger than the negotiated
Browser-lifecycle codec string bound.

## Disconnected-event foundation

`pwright-cap-browser-events`, `pwright-codec-browser-events`, and
`pwright-client-browser-events` define the independently selectable public
surface. The event carries no payload, matching Playwright's callback; its
route carries Browser identity and the fixed-size codec carries a monotonic
sequence. The reliable client handler uses `RejectNewest`, because replacing a
terminal state notification would make the client view invalid.

`pwright-server-browser-events` supplies a separate queue-free, task-free
publisher for the one possible disconnected event. Its first polled send owns
the attempt; failure or Drop makes publication terminal so a possibly partial
stream frame is never retried. The ordered runtime-control barrier can now send
all child-first lifecycle messages before Browser Dispose while leaving that
final Dispose claimed by nobody. The eventual composition can therefore hold
one writer permit across:

```text
Page/Frame/Context Dispose -> Browser disconnected -> Browser Dispose
```

`BrowserEventsInstallationProfile` now makes that wiring explicit. It cannot
prepare without the bounded remote handshake, ordered control, and a send
frame large enough for the fixed 17-byte payload. When accepted, the attach
composition builds one publisher from the negotiated directional protocol and
the connection's production ledger, without adding a queue. Its returned
bundle shares that publisher with an event-aware CDP liveness backend and a
connection-cleanup decorator. The owned-close handler takes another clone of
the same publisher. Thus native loss and explicit close race through one
at-most-once state instead of emitting independently.

The owned command service validates the Browser ID and prepares complete
runtime disposal before it takes the grant. An unpolled command is lazy. Once
started, dropping it transfers the exact future into capacity reserved from a
caller-owned cleanup queue; no hidden task or executor is introduced. A
terminal provider success or failure commits descendant-first disposal, and
the ordered control barrier sends Page, Context, and Browser Dispose messages
before the typed Result.

Unit and encoded-memory tests prove authority is consumed once, wrong targets
do not consume it, accepted sessions preserve exact supervisor-ledger identity,
shutdown failure still invalidates the tree, a dropped started call continues,
and the client cannot complete before all Dispose messages make its handles
stale. A real Chrome 152 test creates actual accepted unprivileged, unbound,
foreign-ledger, and exact-owner sessions. Each rejected grant returns the same
live process owner; only the exact-owner session proceeds. It then observes
Page Dispose, Context Dispose, disconnected, Browser Dispose, and Result in
order while reaping the exact launched process. Polling the shared native
liveness lane afterward coalesces the same CDP closure without a duplicate
event. The unrelated dedicated Chrome remains reachable.

## Acceptance evidence

- compile-time proof prevents `AttachedChromiumComposition` from terminating a
  process or constructing the owned-close grant;
- one non-cloneable lease and one active shutdown transaction exist per
  launched Browser;
- unpolled close is lazy and admitted started close survives caller Drop via a
  pre-reserved continuation;
- injected provider parts expose graceful/forced completion, lost
  acknowledgement, and shutdown failure;
- report-sink tests prove provider diagnostics remain launch-local while the
  encoded portable success remains unit-valued; the live report identifies the
  exact reaped PID and profile;
- encoded E2E proves Page/Context Dispose, disconnected, Browser Dispose, then
  Result;
- native-disconnect cleanup proves the same lifecycle/event order and still
  delegates ordinary connection cleanup, including on publication failure;
- profile and negotiation tests prove the capability is absent unless its full
  queue-free remote path is selected and uses the shared production ledger;
- accepted-handshake tests prove production connection identity matches only
  the originating supervisor ledger, while unbound and equal-plan foreign
  ledgers cannot authorize the owned close grant;
- optional-Tokio tests prove reservation exclusivity, automatic transferred
  close execution, deadline-driven future Drop, permit release, lifecycle
  rollback, and distinct timeout/runtime-cancellation metrics;
- codec, client, and runtime tests prove the optional close reason is bounded
  on the wire and shared across already-pending descendant failures without
  changing backend cleanup semantics;
- client tests prove unpolled close is reason-lazy, the root family shares one
  bounded allocation, and post-disposal operations report the stored reason;
- live tests prove external Chrome remains reachable, owned remote close emits
  the complete terminal order, and only its dedicated process is reaped;
- dependency checks keep Node.js, bundled browsers, implicit download, hidden
  tasks, and unbounded queues out of this leaf and the minimal attach profile.
