<!-- agent-updated: 2026-09-10T23:46:00Z -->

# Raw Page Mouse: primitives, click scheduling and Node integration tasks

## NODE-150: fresh-document click delivery and explicit scheduling controls

The NODE-149 complete shared corpus retained `touched` versus `No touch`.
This investigation adds a separate `raw-click-observation.html` fixture that
preserves the target/handlers and records at most 64 document-capture events.
It reads event fields only, never geometry, focus or hit testing during input.
The original `touchscreen.html` and shared workload remain unchanged.

`probe:shared-click` keeps original goto -> raw click -> result read as its
default. Event snapshots and geometry are read only afterward. The bounded
transparent relay now includes force alongside pointer fields and records
exact session/target identity and command completion. Direct and relayed cases
remain separate. All reports are under `.work/node150-click/`.

`observed-drained.json` contains 20 completed original-mode attempts: upstream
0/10 intended successes, candidate 2/10. Every missed-click sample contains
only pointerover/pointermove/mousemove on the correct target, with no
pointerdown/mousedown/pointerup/mouseup/click. In traced missed samples all
three expected move/down/up commands address the exact owned Page session and
receive non-error responses. Coordinates, force, masks and counts match.
Correct protocol acknowledgement therefore does not prove the DOM click.

Explicit experimental controls, with 5 direct and 5 relayed attempts per side:

| Diagnostic mode | What changes from original | Upstream | Candidate |
| --- | --- | --- | --- |
| original | Nothing; no added pre-click wait or action | 0/10 | 2/10 |
| serial-buttons | Separately await public move, down and up | 10/10 | 10/10 |
| delayed-click | Public click with explicit delay: 1 | 10/10 | 10/10 |
| painted-batch | Await two animation frames, then original zero-delay click | 10/10 | 10/10 |

The last control preserves move/down/up batching but changes document readiness.
This supports a fresh-document/render-state timing sensitivity shared with
upstream; it does **not** identify the precise Chromium implementation branch,
prove a Rust defect, or establish a production fix. None of these controls is
silently enabled by the package or shared corpus. No timeout, golden assertion,
default batch order, browser flags, activation or browser restart is changed.
Playwright 1.58.2's pinned server/input.ts deliberately sends zero-delay clicks
as a same-turn move/down/up batch, so serializing the default would change its
specified scheduling and is not adopted just to make this fixture pass.

As supporting—not exact-build causal—source evidence, Chromium main's
[InputInjector](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/content/browser/devtools/protocol/input_handler.cc)
can acknowledge an event when it was not queued; a successful CDP reply alone
is not sufficient proof of renderer delivery. The exact local Chrome 152
source revision has not been verified against this moving source.

The first original/instrumented probes stopped on a queued terminal watch
after Browser.close: host resources were zero, but one 32,968-byte FFI watch
settlement remained undelivered. Both failed reports are retained. The probe
now records `ffiAtClose`, then uses the same bounded 100-event-loop-turn delivery
drain as the shared/public gates before asserting all FFI counters are zero.
This is a diagnostic cleanup-boundary correction, after action observations;
it does not delay or hide the click result. All four complete control reports
have zero operation/cleanup errors and no diagnostic capture failure, while
original-mode missed clicks still cause a nonzero exit.

The final unchanged shared workload (`.work/node150-click/shared/report.json`)
still has 53/57 matching cases, but its click mismatch reverses NODE-149:
upstream returns `No touch` and candidate returns `touched`. This independently
retains evidence of missed intended clicks on both providers, not a consistently
one-sided Rust failure. The report has zero candidate/setup/cleanup errors;
download remains unsupported and both upstream history cases time out.
The temporary fixture server and preflight target were verified absent after
cleanup; shared Chrome was neither restarted nor stopped.

Remaining work: exact native discard/renderer-delivery attribution and full
shared raw-click stability. Equal missed clicks are not intended-behavior
success. Do not mark the NODE-149 complete corpus green from these controls.

NODE-140: The unchanged shared fixture reports a candidate click value mismatch.
An isolated original-sequence diagnostic then misses the expected click on all
10 upstream attempts and 8 of 10 candidate attempts, with no setup/cleanup errors.
Captured native input fields agree; after-action geometry, hit target, viewport
and readyState do not explain the cause. No pre-click wait, Locator substitution,
retry or production Mouse change was used. Equal missed-click values are not
proof that an intended click succeeded.
[Expanded verification and retained failures](testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

## NODE-057: setup experiment, not a causal fix

The unchanged NODE-056 addon now passes **44/44** both with the new explicit
`PWRIGHT_MOUSE_SETUP=background` diagnostic mode and, immediately afterward,
with `PWRIGHT_MOUSE_SETUP=original`. Both reports have SHA-256
`244a7b104e706185cbfdb0585845a91664282d4a4a9a1d2839755efda6d03d6f`,
zero host leases/operations/cleanup, and confirmed removal of owned targets and
Contexts. The original mode remains the default. The earlier 7/44 and 4/44
failures below remain valid evidence; a passing control does **not** show that
the new setup fixed those failures or identify the source of extra movement.

Background setup creates one additional owned blank guard Page/Context via the
test observer and activates that exact target before resetting each fixture.
It also does so after the two-Page case creates its second Page and after the
navigation case reloads. It changes neither the 44 input workloads nor captured
events, and does not insert waits between same-turn input calls. The guard's
exact Context and target are checked absent in cleanup. This is optional test
infrastructure, not Node package browser behavior or a required runtime feature.
Reports now label `setup` explicitly; no event is filtered from comparison.

`scripts/probe-mouse-focus.mjs` is a separate bounded diagnostic, run from
`packages/pwright` with the selected profile, loopback endpoint,
`PWRIGHT_MOUSE_FIXTURE` and unused absolute `PWRIGHT_MOUSE_FOCUS_REPORT`.
For each provider it explicitly activates its own fixture or blank Page, records
two seconds with no input, then records a down/up pair. It uses a 30-second
overall bound and 256 KiB report bound. Normal failures are written with any
completed samples and still exit nonzero; failures are not passing comparisons.

Evidence: `.work/node057-mouse-isolation/`.
`focus.log` first failed in the second upstream connection (after the observer
was connected); `focus-repeat.log` later failed in candidate topology
initialization. Both retained the original 5-second connection bound and
WebSocket/HTTP stayed reachable. The final `focus-capture.json` completed all
four provider/focus combinations: zero idle events and the expected five
down/up/click events in each. It did not reproduce an activation difference.
`mouse-background.json` and `mouse-original.json` then each pass 44/44.
No Chrome restart, production input edit, timeout increase or assertion
weakening occurred. Initial activation, concurrent native input and intermittent
attachment remain investigations, not diagnosed/fixed causes.

NODE-056's current addon has an open Mouse regression: two full comparisons
match 7/44 and 4/44 with extra, unrequested movement events appearing on either
provider. A separate no-input diagnostic observes possible initial-event
contamination; it does not prove a source or validate production Mouse parity.
No event filtering or Mouse implementation change was made. Historical passing
artifact results below must not be presented as this artifact's current result.
[Failure evidence and bounded diagnostic](pwright-frame-publication.md#open-mouse-regression-diagnostic).

NODE-050's selected Node addon now matches all 44 public Mouse comparisons,
including the five same-turn move/down/move/up cases that failed in the earlier
historical default artifact. This is not a fresh same-source default-versus-
selected causal benchmark. The default binary is unchanged.
[Selection, evidence and remaining work](pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 preserves the existing Page drag controller while Locator lifetime
moves from Frame to Page after focus. Native Frame gate matches 12/12; Mouse
capacity and the historical default-addon 39/44 baseline are unchanged.
[Contract, evidence and remaining gates](pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 verifies native drag cancellation by child/OOPIF Locator Escape, with
no controller/budget change. Post-focus Frame removal still ends the Rust
Locator command too early; the expanded Frame gate is 8/10, not full parity.
[Evidence](pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 also shares the Page controller with Locator keyboard transactions.
Controller admission is now 536 dispatcher / 272 Page command slots; CDP bounds
remain 552/32. Locator pointer actions are unchanged.
[Ownership and evidence](pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 shares the Page drag controller/retirement driver with selected Keyboard
Escape cancellation. Pending cancels use the same quarantine; 528 dispatcher and
264 Page command slots include eight Keyboard owners. No synthetic release or
new leaf task is introduced. [Details](pwright-chromium-drag.md#node-046-optional-rust-keyboard-escape-interception).

NODE-045 fixes maximum click-batch admission: 25/25 actual host raw cases pass.
The rebuilt default Node addon scores 39/44 (all three added count boundaries
match; the same five same-turn discrepancies remain). See
[capacity and artifact evidence](pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).

NODE-044 adds opt-in HostSession Mouse selection, per-native-session drag state
and joined quarantine/cancel shutdown. Default Node behavior is unchanged;
Escape, Page destruction and the 32-slot versus 65-command click bound remain
open. [Host integration](pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration).

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

## Status

NODE-041 implements the explicit `mouse-dispatch` feature and Page-owned cleanup
driver: **82** unit tests and **4** selected-Mouse native cases pass, alongside
the prior **10** native cases. Default/feature closures are **3/5** Pwright
packages. Host Frame-source/lifecycle and Keyboard Escape integration remain
pending; no addon rebuild or public **36/41** change.
[Selected dispatcher and evidence](pwright-chromium-drag.md#node-041-selected-mouse-dispatcher-and-page-cleanup-driver).

NODE-040 adds Page-owned drag state and retained native enter/over/drop/cancel.
**70** unit tests and **10** native scenarios pass, including actual drop and
cancel with dragend checks. The optional leaf remains **3** Pwright packages;
Mouse dispatch/forClick/down suppression, Keyboard Escape and Node host selection
are still pending. The addon/public **36/41** evidence is unchanged.
[Page state and verification](pwright-chromium-drag.md#node-040-page-drag-state-and-native-event-owners).

NODE-039 composes temporary drag listeners, capture and lazy native movement in
one Rust owner. **13** new tests (**57** drag total) and **9** native cases pass,
including three composed paths. The production live-state Mouse adapter, Page
drag state and host selection remain pending; public **36/41** is unchanged.
[Composed movement](pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 adds a fixed-session capture owner: fresh subscription before enabling,
retained enable/disable receipts across cancellation, first matching admitted
data and explicit dirty-state handling. **17** new tests (**44** total drag)
and **6** native listener/capture cases pass. The live-state move callback and
full Page dispatcher remain pending; public **36/41** is unchanged.
[Capture lifecycle](pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 adds bounded typed intercepted-data retention, with **14** new tests
(drag total **27**) and the existing native-drag case passing through that store.
Session provenance, count and aggregate UTF-8 limits are explicit; capture epochs,
subscriptions and the production dispatcher remain pending. Public **36/41** is
unchanged. [Data ownership](pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036 adds an optional single-world drag-listener owner in a separate
**3-Pwright-package** leaf. **13** unit tests and **6** real Chrome cases pass,
including actual drag interception/drop in diagnostic scaffolding and exact
listener removal. The production drag dispatcher, multi-Frame composition and
host integration remain unimplemented; no addon rebuild or public gate change.
[Listener design and evidence](pwright-chromium-drag.md).

NODE-035 now carries the optional dispatcher through typed native Page bindings
and the Page route, including standalone down/up. Default and explicitly selected
test-only passthrough bindings each match **22/22** fresh browser cases. This is
selection/retirement evidence, not Chromium drag behavior. The host still selects
the default binding and no addon is rebuilt; the latest public result stays
NODE-033's **36/41**, with all five drag-path differences open.

NODE-034 adds an optional cancellation-aware dispatch interface to the Rust
click/move scheduler. It is infrastructure, **not an implemented Chromium drag
provider**; no Node addon is rebuilt for this checkpoint. The latest public
41-case result below therefore remains the previous artifact's **36/41**.

**Latest expanded gate is red (NODE-033).** Adding same-JS-turn concurrent
submissions expands public Mouse from 26 to **41** cases. The previous addon
matches **33/41**. An explicitly selected Rust first-poll ordering boundary
improves this to **36/41**, leaving all five left-held-move cases red. These are
not discarded or changed to sequential awaits. See the new checkpoint below;
the earlier 26/26 result remains evidence only for its original smaller corpus.

NODE-032 now implements move/down/up/wheel and bounded click/dblclick scheduling
in Rust. They share the existing Page-owned input coordinator with Keyboard and
Locator and match **22/22** fresh Playwright 1.58.2 event/state cases on Chrome
152. A rebuilt release addon now exposes all six public Node Mouse methods;
**26/26** public comparisons and **3/3** profile/Worker tests pass. Concurrent
first-error ordering, HTML drag interception and final nested allocation audit
remain open.
Mouse 1.0 contracts, admitted native starters and a bounded Page route with
caller-polled retirement are implemented. The generated wire chain now matches
**22/22** fresh browser cases through that route; this is not Node API coverage.
An explicit `runtime` feature now adds a receipt-aware single-call adapter;
**15** additional tests and **22/22** controlled-runtime browser cases pass.
The safe Rust Node host now selects all six methods and bounded independent
retirement workers. Its **22/22** host-path comparison and real close-during-delay
probe pass. JavaScript/N-API exposure is now implemented; see the artifact-specific
public checkpoint below. The earlier **52/3/2** shared result is historical.

## Page-scoped dispatcher selection (NODE-035)

Evidence: `.work/node035-mouse-page-dispatch.i7TGJZ/`.

`CdpMouseServiceFactory::bind_with_dispatcher` selects one dispatcher for the
returned Page's move/down/up/click/dblclick services. The factory itself is not
mutated; later ordinary bindings do not inherit the selection. Wheel remains
direct. `ChromiumMousePage::bind_with_dispatcher` exposes the same explicit
selection through the optional Page route, preserving its resource admission,
exact command/cancellation identity and caller-polled retirement driver. The
existing `bind` APIs retain their direct behavior.

Standalone buttons now use `MouseButtonExecution` when a provider is selected.
`raw_mouse_down_with_dispatcher` and `raw_mouse_up_with_dispatcher` construct
effect-free owners. At first result poll they apply logical Page button state,
capture the event, then start/poll the provider. This allows a future drag
provider to suppress a physical down/up or replace up with drop **without
skipping logical button-state changes**. Cancellation before first poll does
neither; cancellation after setup begins does not roll back the already applied
logical state or emit a synthetic release.

Button result observation is separate from cleanup. A successful or failed
provider result may leave an operation retained until `retire()` finishes; an
abandoned borrowed retirement future can resume using the same owner. The native
starter retains its existing operation admission until its owner drops, and the
Page driver retains native/record ownership through actual receipt consumption.
The controlled invocation adapter does not acknowledge cancellation merely on
provider-result completion, cleanup start, or unconsumed receipt production.
Action errors remain Backend errors; cleanup errors remain Cleanup errors with
the original operation identity.

### Storage and integration boundary

The concrete native-operation measurement now includes the standalone button
owner as a possible execution variant. Shared Page-binding measurement includes
the optional dispatcher Arc. Existing host **32 KiB** native allowances are
unchanged and its admission/layout tests pass. Injected provider boxes, event
payloads, subcalls, errors and persistent drag state still need separate admission
and ownership. Neither this outer measurement nor test fakes certify their nested
storage or whole-process RSS.

No production drag provider, background worker, new crate, package dependency,
clock or JS behavior is introduced. The Node host's per-call Page route still
uses ordinary `bind`; a real drag integration must instead supply the same
properly owned **Page-scoped** provider across those calls, retire persistent
interception state during Page/host teardown, and budget all listener/world/event
payload resources. Merely selecting a fresh provider per command would lose drag
state and is not a completed integration.

### Verification

Thirteen new deterministic tests cover button first-poll/cancel boundaries,
result versus cleanup errors, physical button suppression with preserved logical
state, all five typed service selections and direct wheel, binding isolation,
admission rejection, response drop, cleanup receipts and controlled cancellation.
The controlled setup/cleanup provider is a fake, not real drag interception.

- `component-tests.log`: input **85**, native **17**, route/runtime **38** passed.
- `host-adjacent-tests.log`: host **115 / 6 ignored**, Keyboard **18**, Locator
  **103** passed.
- `clippy.log`: strict release all-target/all-feature input/native/route/host/addon
  checks pass; dependency proofs in `deps-*.log` retain input-only **5**, native **8**,
  Mouse default/runtime **16/17** Pwright packages and host **234** normal packages.
- `oracle.json` and `oracle.log`: fresh **22** Playwright 1.58.2 observations on
  dedicated Chrome **152.0.7977.76**.
- `native-live.log`: default **22/22**, selected test-only passthrough **22/22**,
  with owned Context/target absence checked after each case. Four provider counters
  assert that ordinary move, forClick move, down and up actually entered the
  selected path. The passthrough forwards captured native events unchanged; it
  does not intercept drag or establish error/continuation timing parity.

Formatting and tracked whitespace checks pass (`fmt.log`, `diff-check.log`).
Preflight used task-local Pwright state and the verified serve-lib Mouse fixture
on `macbookpro-yuacx.tail8f3b66.ts.net:9384`; CDP remained loopback-only on 9222.
`targets-final.json` confirms preflight target
`D776915F326F46F7B8C59BCEE09BFCD0` and Mouse fixture targets absent after cleanup.
Chrome was not restarted and unrelated Pages were not selected for close.

The addon remains `e65e6d85...17b246` and is not rebuilt here. No new public
41-case or Worker/FFI result is claimed; keep NODE-013 attachment, native
reload-close and the broader compatibility/allocation gates open.

## Optional mouse dispatch ownership (NODE-034)

Evidence: `.work/node034-mouse-dispatch.l4hwM5/`. This source checkpoint adds
`CdpMouseDispatcher`, `MouseDispatchOperation`, `MouseDispatchRequest`,
`MouseDispatchKind`, `MouseStateReader` and `DirectMouseDispatch` to the existing
command-free `pwright-backend-cdp-input` leaf. No dependency, executor, clock,
browser launch, JS behavior or native drag interception is added.

### Selection and lifetime

- `raw_mouse_click_with_dispatcher` selects one provider at execution construction;
  it requires the existing Interleaved Page policy. The unchanged
  `raw_mouse_click` uses direct native dispatch. Unpolled retirement neither calls
  the provider factory nor changes logical pointer state.
- Requests distinguish ordinary move, click-bypass move, down and up. A zero-count
  click remains `for_click: true`; the typed `CdpMouseMoveService` marks its
  count-zero sequence `false`. This flag alone changes no native behavior.
- Request coordinates, last-button and initial event payload are captured.
  `MouseStateReader` provides **live buttons/modifiers only**, for a provider that
  sends an event after asynchronous preparation. It retains the Page-state
  admission even if cloned/escaped past the coordinator and execution lifetime.
  Reading live state does not itself rewrite or dispatch the captured event.
- Provider construction must be effect-free. `poll_result` drives action work;
  `poll_retire` stops planning pointer actions, drains started work and performs
  owned cleanup. Pending polls must install a waker. Terminal retirement, including
  an error receipt, requires no further cleanup from that owner. Persistent drag
  state needs a separate Page-scoped owner and lease; dropping a result future or
  owner is not a retirement acknowledgement.

### Scheduling and errors

The existing fixed **65 slots** now retain direct or delegated dispatch owners.
A delegate may own multiple separately budgeted native subcalls, so
`pending_native_calls()` now counts **retained dispatch owners**, not nested RPCs.
No-delay batches still construct and first-poll every submitted owner before
observing an immediate operation failure. This is not proof of upstream
first-rejected-Promise fidelity.

An early operation error returns while retaining unfinished cleanup. Calling
`retire()` subsequently returns its separate cleanup error, if any. Successful
children are retired before later interpolation steps reuse their slot; cleanup
is polled immediately so it registers a wakeup. Thus successful result observation
does not accumulate unbounded successful-but-unretired moves. It also means a
provider's successful cleanup participates in scheduling; exact Chromium timing
still needs the real provider and a browser oracle.

A cleanup failure stops later planning with a bounded action diagnostic and keeps
the original cleanup error for the retirement receipt. Already submitted batch
owners still receive their first poll. Retiring during preparation never resumes
that operation's pointer-action phase. A borrowed retirement future may be dropped
and recreated while the execution retains its owners. The direct fallback drops
an unstarted RPC without sending it, and drains a started RPC without resubmission.

### Resource boundary and current scope

The sequence's payload measurement includes its larger fixed slot array, direct
outer futures/event type strings and delay boxes. Delegated boxes, native
subcalls, setup listeners, interception payloads, provider errors and shared drag
state are **not** certified by that measurement: an injected provider must admit
and bound them separately. Allocator headers and nested client/executor storage
remain outside this partial measurement. The host's existing **32 KiB** native
profile is unchanged; host admission/layout tests still pass with the larger
sequence. This is not a process RSS bound.

Only the low-level scheduler exposes injection in this checkpoint. Typed native
factories, Page/runtime composition and the Node host still select the direct
path; standalone down/up/wheel do not yet pass through this interface. A later
Page-scoped selection must cover drag-related down/up suppression/drop, temporary
listeners, utility worlds, event interception and shutdown before any drag parity
claim. Wheel has no drag-dispatch requirement established here.

### Verification

Ten additional deterministic tests cover unpolled cancellation, cancellation
during gated setup and cleanup, abandoned/resumed retirement, early result versus
cleanup error, whole-batch first polling, cleanup-waker registration, more than
65 interpolated moves with slot reuse, cleanup failure stopping later actions,
live state versus captured event fields, request kinds, direct fallback, and
escaped reader lease retention. They use controlled fakes, not real interception.

- `input-tests.log`: **81** passed (previously 71).
- `mouse-tests.log`: native **13**, route/runtime **33** passed.
- `host-tests.log`: **115** passed / **6** ignored.
- `adjacent-tests.log`: Keyboard **18** and Locator **103** passed.
- `clippy.log`: strict release all-target/all-feature input/native/route/host/addon
  Clippy passed. No warning suppression was added.
- `deps-input.log`, `deps-backend.log`, `deps-composition.log`, `deps-host.log`:
  input-only **5**, native **8**, Mouse default/runtime **16/17** Pwright packages;
  host **234** normal packages, unchanged.
- `fmt.log`, `diff-check.log`: formatting and tracked diff whitespace checks pass.

`oracle.json`/`oracle.log` record **22** fresh Playwright 1.58.2 cases on the
existing dedicated Chrome **152.0.7977.76**. `host-live.log` runs the changed Rust
source through HostSession: **22/22** exact comparisons plus the close-during-
60-second-click-delay probe pass (two test functions). The close probe observes
real mousedown, then verifies `TargetClosed`, no synthetic release and drained
host tasks/leases. The paired lane checks each owned native Context/target absent
and final host metrics drained. This exercises the default direct path, not an
injected Chromium drag provider or N-API first-poll tickets.

Preflight used `chrome-cdp start/status`, task-local Pwright state and verified
HTTP 200 from the existing serve-lib `/pwright-mouse-concurrency/raw-mouse.html`
route on `macbookpro-yuacx.tail8f3b66.ts.net:9384`. CDP stayed on loopback 9222;
Chrome was not restarted. `targets-final.json` confirms owned preflight target
`81EEAA8F20C3A6BCF3E293E47AD7BDDB` and Mouse fixture targets absent. Unrelated
targets were not selected for close.

The production addon is still `e65e6d85...17b246`; prior artifact-specific results
must not be described as tests of this source change. NODE-013 attachment,
reload-close, broader continuation/error ordering and allocation issues remain
open alongside the missing drag provider.

## Same-turn input ordering (NODE-033)

Evidence: `.work/node033-mouse-concurrency.zwN8wl/`.
`before.json` records the expanded corpus on the earlier `e9d9e543...109535`
artifact: **33/41**. `after.json` records **36/41** on intermediate
`3c7561fe...1bbf3ed`. The production source also adds owner-drop wakeup coverage;
the final release artifact is
`e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
Final artifact results are recorded separately below.

### Final artifact gate

The final **e65e6d85...17b246** build includes installation-owner-drop wakeup.
All rows below use that artifact; intermediate green results do not replace them.

| Evidence in NODE-033 directory | Final result |
| --- | --- |
| `final.json`, `final.log` | **36/41**, exactly the five left-held-move cases remain red |
| `mouse-native-final.log` | **3/3**, including owned/borrowed Worker retirement |
| `keyboard-final.json`, `keyboard-final.log` | **24/24** |
| `typing-final.log` | **30/38**; 3 candidate topology-init and 5 oracle connect timeouts |
| `actions-final.log` | **24/27**; 2 candidate topology-init and 1 oracle connect timeout |
| `screenshot-final.log` | **12/12** |
| `lifecycle-final.log` | **45/45** |

The input/order and browser-setup failures are distinct observations. The failed
typing/action cases stop at attachment, before their input actions; the oracle
reports only its connect timeout, not a verified internal topology phase. The
candidate's new ordering installation is constructed after topology initialization.
This does not rule out indirect interactions or establish the intermittent
failure's root cause. Keep NODE-013 open and do not retry it into a passing result.

The intermediate `3c7561fe...1bbf3ed` artifact's `typing.log`, `actions.log`,
`screenshot.log`, `lifecycle.log` passed **38/27/12/45** respectively, and its
Keyboard/Mouse-Worker gates passed **24/3**. Those are historical artifact-specific
facts, not the final result. The final artifact did **not** rerun reload or the
57-case shared corpus; their prior failures remain open, not silently green.

Final public Mouse metrics are all zero after disconnect; every case checks its
owned native Context/target absence. `targets-final.json` also confirms preflight
`FDE86C7AA794FF6C92043EF3D41893D3` and test fixture targets absent. The unrelated
shared Chrome target identities changed during this run (two Pages both before
and after); no stable whole-browser target-set claim is made. Chrome was not
restarted and unrelated targets were not selected for close.

`node-tests-final.log` passes **34/34**, `types-final.log` and final formatting/
diff checks pass, and `boundary-final.log` retains host **234**. Fixtures for the
final runs use the verified serve-lib route `/pwright-mouse-concurrency` to
`tests/fixtures`; the older Mouse route was not replaced after a register conflict.

The 15 new cases are five independently required repetitions each of:
`move/down/move/up`, `Shift down/click/Shift up`, and a left/right button-state
sequence. All calls are submitted in one JS turn before awaiting settlement.
Failures preserve the exact DOM records. Success is not selected by retrying
until a repetition happens to match.

### Two independent differences

1. The earlier addon sometimes starts later Rust input futures first. The
   double-button sequence then reports wrong button masks, press ordering or
   release coordinates. A reverse-executor-poll unit test now pins the required
   synchronous prefix order. The new start boundary makes all five two-button
   repetitions match in the intermediate artifact's fresh run.
2. Pinned Chromium `RawMouseImpl.move` bypasses drag handling for click moves,
   but routes ordinary moves through `DragManager.interceptDragCausedByMove`.
   When the last button is left, this awaits utility-world listener setup and
   `Input.setInterceptDrags` before sending the move, even on this non-draggable
   fixture. Thus the same-turn up can reach the DOM first; the observed pressure
   also differs. The current pure Rust move primitive has no such delegate.
   All five corresponding cases remain failures. Do not add a timer or reorder
   expected events to imitate one observed trace: implement the optional drag
   delegate, interception lifecycle and forClick bypass with their own budgets.

The source evidence is pinned `playwright-core@1.58.2`:
`lib/server/input.js`, `lib/server/chromium/crInput.js`, and
`lib/server/chromium/crDragDrop.js`. DOM observations alone do not prove exact
CDP payloads or a universal ordering rule for every browser/event-loop schedule.

### Bounded Rust first-poll boundary

`pwright-node-host::input_start` owns a **32-slot**, session-local ordering array.
Raw Mouse and Keyboard N-API calls admit a ticket synchronously, **after foreign
argument conversion and before spawning their Rust future**. No JS state or
Promise queue implements this order. The selected host installs it explicitly;
the reusable Mouse/Keyboard/input leaves acquire no dependency on it.

A later future cannot perform its first poll until earlier tickets have first-
polled or been dropped. The ticket remains held for the entire first poll, with
no queue mutex held while polling the future, and then releases even if the
future returns Pending. Native replies, timers and subsequent continuations
therefore still overlap; this is not a whole-operation serial lock. It does
**not** prove ordering once runtime admission queues, backpressure, Locator
preparation or later continuations interleave. Those need separate tests and
possibly a broader explicitly selected input execution scheduler.

One production lease covers the measured concrete array/Mutex/lease/Arc-control
storage in extension scope **120/22**. Tickets and the generic `run` future are
inline in the already measured/budgeted FFI future; `run` adds no heap box or
task. Injected waker storage, allocator headers and the broader nested runtime
allocation audit remain outside this local measurement. The lease stays alive
while any ticket retains the array, even if its installation owner disappears.

Host admission publishes a shutdown tracker token under the same state lock as
close. Shutdown closes/wakes the start order before joining admitted operations;
first-poll completion or ticket drop releases its tracker token. Dropping either
installation owner also wakes queued tickets, including topology-driver failure
teardown. A caller retaining an unrun ticket must run or drop it; runtime idle is
not proof that these ownership guards or native Mouse receipts have retired.

`host-tests-final.log`: **115 passed / 6 ignored**, including seven deterministic
start-order tests: reverse polling, pending-result overlap, dropped-head wakeup,
close rejection/wakeup of multiple queued bodies, bounded capacity/reuse, retained
lease lifetime, shutdown tracker release, and owner-drop wakeup (some assertions
share a test). `clippy-final.log` passes strict release host/addon all-targets.
The optional Mouse closures remain **16/17** and host closure **234**.

## Initial public Node Mouse checkpoint (historical corpus)

Evidence directory: `.work/node032-mouse-ffi.dx1AXD/`.
Release addon SHA-256:
`e9d9e543e84bc12327ec33ec8e84559a590d480d062528b8813171ba86109535`.

`Page.mouse` is one stable facade per Page. Its move/down/up/click/dblclick/wheel
methods only validate JS shapes, adapt defaults and forward to Node-API. The
addon converts to the six existing typed Rust host methods. Coordinates, buttons,
Keyboard modifiers, native scheduling, delays and receipt workers remain in Rust.
No JS timers, CDP transport, Playwright runtime, subprocess or browser download
is added to the production package.

The private native options object contains four `f64` fields, preventing integer
truncation and owned string/object graphs before FFI admission. The addon admits
the existing Command lane with **32 KiB** argument/result allowance before copying
the bounded Page ID (20 bytes) and operation name (16 bytes). Typed parameters
contain no heap-owned data. The publication/settlement boundary separately
measures and charges the concrete outer future before boxing it and retains the
ticket through delivery or environment teardown. Shared host/native/route storage
keeps its own previously described reservations. This does **not** complete the
nested runtime/executor allocation or allocator-header audit, nor prove an RSS cap.

Current profile restrictions are explicit, not full upstream numeric parity:
finite coordinates/deltas; integral move steps at most 10000; click count at most
32 pairs (nonpositive integral counts produce no pairs); down/up counts must be
signed 32-bit integers; finite delays between 0 and 4294967295 ms. Only left/right/
middle buttons are accepted. Negative delays, unknown options, HTML drag handling
and arbitrary high-work batches are not silently emulated. The native connection's
32-pending-command bound still applies independently of the 65-slot scheduler.

- `mouse.json` / `mouse.log`: **26/26** against Playwright 1.58.2 on Chrome 152.
  The original 22 cases are joined by facade identity and shared state across
  navigation, Page isolation, closed-Page rejection, and Page close after native
  mousedown during a **60-second** click delay. Fractional-step rejection is
  normalized to its category; exact error wording/class parity is not claimed.
  Every owned native Context and its targets are checked absent after cleanup.
- `mouse-native.log`: **3/3**, including 12 invalid numeric/button inputs plus an
  unsupported option without state change or retained FFI commands. Two real
  Worker termination cases start only after native mousedown: owned Context/Page
  disappears; borrowed Context/Page survives with no synthetic mouseup/click.
  The observer finally closes its exact Context and verifies native absence.
- `node-tests.log`: **34/34** offline facade/contract tests; `types.log`: strict
  TypeScript passes. `clippy.log`: strict release addon all-targets passes.
  `host-tests.log`: **108 passed / 6 ignored**. `host-boundary.log` and
  `mouse-boundary.log`: host **234**, optional Mouse **16/17** remain independent.

Run the public lane with `pnpm --dir packages/pwright test:mouse`, setting dedicated
`PWRIGHT_TEST_CDP`, verified serve-lib `PWRIGHT_MOUSE_FIXTURE`, and a new absolute
`PWRIGHT_MOUSE_REPORT` path. `test:mouse-native` uses the first two settings and
the existing native panic/Worker diagnostic supervisor. The shared adapter now
forwards Mouse click/dblclick/move through these public methods; the unchanged
shared corpus and all new-artifact regression outcomes are recorded separately.

### Same-artifact regression

The same `e9d9e543...109535` release addon was used without rebuilding between
these lanes. This is the named input/lifecycle/shared regression set, not every
historical evaluation/Locator gate or full Playwright compatibility.

| Evidence in the directory above | Result |
| --- | --- |
| `keyboard.json`, `keyboard.log` | 24/24 public Keyboard |
| `typing.log` | 38/38, including held modifiers and typing Worker teardown |
| `actions.log` | 27/27 |
| `screenshot.log` | 12/12 |
| `lifecycle.log` | 45/45 |
| `reload.log` | 16/18; both original response-boundary close failures persist |
| `shared/report.json` | 53 match / 2 unsupported / 1 oracle-error / 1 setup-error |

The complete **57-case** shared corpus has no value mismatch, candidate-action
error or cleanup error, but its supervisor correctly exits **1**. There is no
panic, output overflow or supervisor timeout. Upload and download are still
unsupported. The old mouse-backed `touchscreen` case now matches; it remains
**not real Touchscreen evidence**.

The shared `go back after navigation` case records an upstream `page.goBack`
timeout, followed by a candidate **5000 ms topology-initialization timeout before
any candidate operation**. Classification stays `setup-error`, not a hidden
second oracle-error. `go forward after going back` remains `oracle-error`, with
candidate goBack unsupported. This is renewed NODE-013 evidence, not proof that
Mouse caused the intermittent attachment problem. No deadlines, native absence
checks or readiness fences were weakened; no silent retry or Chrome reset was
used. Overall compatibility remains **red**.

`shared-types.log` and `shared-unit.log` pass (30 shared-harness unit tests).
`node-tests-final.log` / `types-final.log` retain 34 offline passes and strict types.

The supervised public rerun (`mouse-supervised.json` / `.log`) also passes
**26/26**, with all final host metrics zero and the same artifact hash.
`targets-final.json` verifies the exact preflight target
`CE5A13D2C40E596A9B649A16C6F6BD46` and Mouse fixture targets absent. The shared
dedicated Chrome target inventory changed during the run: one of two unrelated
XLQY Pages disappeared; this work did not issue a close for its ID, and the cause
was not observed. Do not claim the entire shared target set was stable, or
attribute the topology timeout to that change without causal evidence.

## Verified upstream behavior

The installed pinned `lib/server/input.js::Mouse` retains logical destination,
last button and a set of pressed buttons on Page. Keyboard modifiers come from
the same Page. Down/up in separate calls must use that state; an up resets the
last button to none even when other buttons remain held. Up dispatches even if
the specified button was not held. This is not command-local button ownership.

`Mouse.move` assigns its logical destination before the native move sequence.
Zero and negative integer steps emit no moves but still update that destination.
The internal loop could interpolate fractional steps, but the **public protocol
rejects them as non-integers before dispatch**. The initial oracle failed on
2.5; the model and test were corrected to cover rejection without a changed
position. Do not infer the public contract solely from the server method body.

`lib/server/chromium/crInput.js::RawMouseImpl` sends native mouse events. Move
includes the last button, the complete button mask and pressure 0.5 when any
button is pressed, otherwise 0. Down includes that pressure; up omits it. Wheel
uses the current pointer/modifiers and native deltaX/deltaY, **without a button
mask**, and does not wait for scrolling to finish. JS `scrollBy` is not wheel
input and is not an acceptable fallback.

The existing v1 bridge Mouse is not reused: it does not retain proper down/up
coordinates, and its wheel evaluates JavaScript. Those legacy APIs are left
unchanged. Also, the shared suite called `touchscreen` actually invokes
`mouseClick`; its fixture accepts click as well as touchstart. Passing that one
case later cannot establish Touchscreen support.

## Rust boundary and resource ownership

- `pwright-cdp` adds `input_dispatch_pointer_event` using the existing generated
  `DispatchMouseEventParams`, including pressure and wheel fields. The old
  convenience signature remains unchanged. The DI trait's default fails
  explicitly for clients that do not implement the complete payload; it must
  not silently drop pressure/deltas. The real client and recording fake implement
  it, with no JavaScript evaluation.
- `pwright-input-model::MouseMoveSteps` validates finite integral numeric values
  and declares a maximum of **10,000 move events**. Fractional, nonfinite and
  excessive-work failures have distinct typed variants. Nonpositive integral
  values retain their public semantics. The work cap is an explicit Pwright
  resource restriction, not evidence of unbounded Playwright parity.
- `CdpInputTransaction::raw_mouse_move/down/up/wheel` require the explicitly
  selected **Interleaved** Page input policy. Exclusive Locator transactions
  retain their old cleanup policy and reject raw Mouse before dispatch.
- The fixed Page record adds only last-button state to its existing pointer,
  button-mask and keyboard record. Existing Locator down/up update that field
  too. Interleaved Locator interpolation now assigns its logical destination
  before dispatch as well, preserving a consistent shared position after native
  failure; its exclusive policy and native event sequence remain unchanged.
  Production state-byte accounting derives from the concrete allocation
  size, and escaped coordinator/transaction clones retain their existing lease.
  The click execution additionally retains this same admission while it escapes
  a transaction; it does not create another Page record.
- Raw Mouse captures logical state before its native await, without awaiting
  previous raw Mouse replies. Keyboard/Locator retain their existing event gate;
  their full mixed-concurrency semantics are a separate unverified gate.
  Logical state is retained across calls and changes before native dispatch,
  including failure.
  Dropping an action or running interleaved cleanup is not a synthetic release.
  A failed interpolated move retains its destination, not just its last emitted
  intermediate position. Nonfinite coordinates/deltas reject before mutation.
- The standalone input dependency proof remains **5 Pwright packages including
  the example**, with no new normal dependency. Raw Mouse services must later
  obtain the same admitted `page_input_binding`, not make an independent state
  record per API call or JS wrapper.

These primitives do not yet implement upstream move's drag-manager interception.
Event parity for the catalog below does not establish HTML drag behavior.

## Optional Mouse 1.0 contracts

Canonical source: `crates/pwright-cap-mouse/schema/mouse.yml`. Generated
[protocol reference](../../crates/pwright-cap-mouse/schema/protocol-reference.md),
capability descriptors, binary models and server chain all come from this source.
This is an initial unpublished contract, not a released ABI or a new Playwright
driver process. It imports the existing common call metadata and failure model.

| Command | Method | Command/result tags | Parameters |
|---|---:|---|---|
| mouse.move | 917505 | 1 / 2 | x, y, steps |
| mouse.down | 917506 | 3 / 4 | button, click_count |
| mouse.up | 917507 | 5 / 6 | button, click_count |
| mouse.click | 917508 | 7 / 8 | x, y, button, click_count, optional delay |
| mouse.dblclick | 917509 | 9 / 10 | x, y, button, optional delay |
| mouse.wheel | 917510 | 11 / 12 | delta_x, delta_y |

All six return unit and target Page only. The client has one independent
extension trait per command; a down-only channel compiles without the other five
services and needs no main Frame. The server injects one service per command.
Call IDs, target identity, timeout and cancellation remain in the existing
command/metadata model, separate from Mouse delay. There is no new lifetime,
transport, selector, input state or browser owner in these packages.

Parameters are fixed-size, Copy types. `MouseNumber` rejects nonfinite numbers
but preserves negative/fractional coordinates and signed zero. Move steps reuse
the existing finite integral 10,000-work limit. Click batch count reuses the
0–32 type; down/up preserve their distinct signed 32-bit native click-count
payload rather than interpreting it as a batch. Dblclick has a distinct type
with no count override. No viewport clamping, scrolling or JS event synthesis
is implied. Public JS option defaults and conversion remain adapter work.

The handwritten semantic adapter validates finite numbers and step invariants
when turning decoded wire models into typed commands. The generated schema also
bounds click batches to 32 and error message/detail strings to 4096 bytes each;
a codec may select a stricter string budget. No attachments are accepted.
Command frames verify method, payload variant, inner/outer call ID and Page ID;
results verify operation variant and call identity even without a header method.
Every operation failure kind and typed detail is preserved. These bounds do not
constitute production resource leases or native retirement proof.

Evidence: `.work/node032-mouse-contract.mafoFW/`.

- `tests-final.log`: capability **3**, client **2**, codec **18**, server **1**
  tests pass in release mode. Includes all six parameter/result roundtrips,
  separate delay/control metadata, six-command server dispatch, independent
  client extension, forged identity/method/variant rejection, extra attachments,
  nonfinite/fractional/excess-work wire values and malicious oversized batches.
- `clippy-final.log`: strict release all-target Clippy passes with no bypasses.
- `contracts.log`: exact normal closures **3 cap / 6 client / 7 codec / 10 server**
  Pwright packages; none loads Tokio, Node, a native backend or the generator.
  All canonical schemas are scanned for method conflicts, and generated outputs
  and reference pass the freshness check. This check is added to CI, while the
  standard workspace test job includes the four packages.
- `keyboard-contracts.log`, `input-boundary.log`, `host-boundary.log`: existing
  Keyboard contracts remain fresh, input-only remains **5** Pwright packages and
  the current Rust host remains **229** normal-dependency packages. Mouse was not
  silently added to the existing host or umbrella installation.
- No native code or Node facade changed in this contract checkpoint. The earlier
  **22/22** browser observations validate the primitive scheduler only; they do
  not establish end-to-end wire/FFI Mouse compatibility. No browser was started,
  addon rebuilt, additional commit made or push performed in this checkpoint.

## Native factory and retained operation contract

`pwright-backend-cdp-mouse` adds six independent typed starters around the same
injected Page input coordinator. Binding rejects Exclusive input before native
work. `begin_raw_mouse_transaction` constructs a raw input owner synchronously
without modifier restoration, state mutation or native dispatch. There is no
selector resolution, new Page input state or browser/driver startup.

The services expose `start(params) -> CdpMouseOperation`, **not** a unit-returning
`Service<Params>` future. The operation is deliberately not itself a Future:
`result(&mut self)` and `retire(&mut self)` borrow its retained state separately.
Cancelling result polling leaves native work inside the owner. Retirement stops
future interpolation/delays, drains started native calls and never synthesizes
mouseup. Retiring an unpolled operation sends no input. Both result polling and
retirement can be interrupted without dropping the owner; result observation
after retirement is a protocol error. Cleanup reply failures remain separate
from the earlier operation result.

This is an explicit ownership contract, not automatic abandonment protection.
Dropping a started `CdpMouseOperation` is **not** retirement proof. The Page route
below now keeps it outside its cancellable response future in bounded,
caller-polled retirement storage. The opt-in controlled adapter below preserves
that ownership on error, timeout, close or abandonment. The host must keep its
unique driver through shutdown. Copying the current Keyboard unit-future adapter
would discard Mouse tails; waiting for retirement before every error result
would instead change upstream early-error timing. Neither is the chosen design.

Move currently reuses the click scheduler's interpolation with zero click pairs;
its distinct portable/native entry remains available for the future non-click
HTML drag delegate. This only reproduces native pointer movement, not HTML drag
interception/drop. Click and dblclick run entirely in Rust with an injected,
owned delay future. Down/up/wheel each retain one native request. Delay providers
are supplied explicitly and no runtime task or timer is created by this leaf.

### Admission accounting

`CdpMouseProductionProfile` declares an operation count and byte allowance.
Binding rejects a profile smaller than `required_operation_bytes`; start reserves
before creating the execution owner. Resource-ledger rejection rolls back the
count. The owner retains its admission through result and retirement, releasing
it only when dropped; an escaped click also retains the Page input admission.
Profile multiplication rejects overflow. Diagnostic messages/details are bounded
to 4096 bytes each, and excessive native diagnostics become resource failures.

The byte helper derives concrete wrapper, single/sequence future, callback box
and outer native-future payload sizes plus event type strings and diagnostic
allowances. It does **not** cover allocator headers, the shared Page binding,
input coordinator/native client state, or allocations/errors inside injected
CDP and delay providers. Binding storage has a separate `shared_binding_bytes`
helper. The embedding installation must account for those provider/route owners
and retained cleanup storage; this local profile is not a whole-process memory
bound or a completed production FFI installation proof.

### Native checkpoint evidence

Directory: `.work/node032-mouse-native.ZIxhdC/`.

- `oracle.log` / `oracle.json`: fresh **22** Playwright 1.58.2 observations.
  `native.log`: the **typed native starter** path matches **22/22** exact DOM
  sequences/outcomes on Chrome 152, **5.52 seconds**. Every Mouse action uses
  the new factory, checks no pending native calls after retirement and verifies
  operation admission is released after owner drop. Shared Keyboard actions use
  the same Page input coordinator. This is not a wire or Node API comparison.
- `regression.log`: native Mouse **13**, input **71**, Keyboard backend **18**,
  Locator **103**, root **295 pass / 15 ignored**, host **99 pass / 6 ignored**.
  Mouse tests include effect-free admission, resumed retirement, pending replies
  after error, cancellation during delay/interpolation, escaped input admission,
  profile under-sizing/overflow, ledger rollback and bounded diagnostics.
- `clippy-final.log`: strict release all-target Clippy passes. The earlier
  unused import was removed, not suppressed. Format and diff checks pass.
- `backend-boundary.log`: exactly **8** normal Pwright packages; no runtime,
  Locator, Node host or driver. New CI proof: `check-v2-mouse-backend.mjs`.
  `contracts.log`, `input-boundary.log`, `host-boundary.log`: portable Mouse
  **3/6/7/10**, input **5**, host **229** boundaries remain intact.
- The expired shared fixture registration returned 404; this task registered
  `/pwright-mouse-native` via serve-lib on the verified Tailscale interface.
  The exact MagicDNS fixture URL returned 200 before comparison. Result polling,
  native CDP calls and fixture waits remain bounded individually; native owners
  are retired before test-owned Context cleanup and absence checks.
- Only preflight `31E2393F94F9D25C006BA60420DEACD6` was explicitly closed using
  its verified local tab ID. `targets-final.json` proves absence and preserves
  two other tasks' Pages. All test processes exited; Chrome was not restarted.
  The addon is unchanged at `1319fa02...ded25dc1`; no additional commit/push.

To reproduce after the standard dedicated Chrome/Pwright preflight and fresh
oracle creation, use the same environment variables as the primitive gate but
select `cargo test --release --locked -p pwright-backend-cdp-mouse --test live_mouse
-- --ignored --nocapture`.

## Bounded Page route and retirement receipts

`pwright-composition-chromium-mouse` connects all six generated binary commands
to the typed native starters. It binds one exact Page and the injected shared
input coordinator; wrong targets and duplicate retained CallIds fail before
native input. Admission is eager and effect-free. The optional leaf has exactly
**16 normal Pwright dependencies**, enforced in CI by
`scripts/check-v2-mouse-composition.mjs`; it installs neither a controlled
runtime nor the complete Chromium/Node host.

The Page owns fixed-capacity slots containing native operation owners. Response
futures borrow those owners while polling, never across a mutex-held native poll.
Success or error queues retirement; dropping an unfinished response requests it.
Early errors can therefore return with native tails still retained. A unique,
caller-owned `MouseRetirementDriver` polls eligible records fairly without
head-of-line blocking, creates no task, and returns one identity-bearing receipt
per operation. Cleanup failures remain separate receipts, not swallowed behind
an earlier operation error. Abandonment wakes an idle driver.

Capacity counts live records, not just occupied queue slots. A retired receipt
retains both its record and native admission until consumed/dropped; an unpolled
cancelled response can also retain record admission after the driver finishes.
Holding either cannot create an unbounded stream of detached receipts. Native
admission failure rolls back tentative record admission. Route, record and
native execution payloads have separate checked accounting; provider/client,
delay, caller response/executor allocations and allocator overhead still need
embedding-specific accounting. This is not a whole-process memory bound.

Cancellation requests validate the complete CallId/Page/method/cancellation-ID
tuple. The first reason wins, including when transport closure follows a
deadline. Request acknowledgement is **not** native completion: the embedding
must consume the driver's actual receipt. `revoke_admission` rejects new and
not-yet-polled work; it does not silently cancel already-polled operations.
Active Page shutdown must explicitly request cancellation and drain receipts.

The default is an **inert wire recipe**. Command timeout metadata does not start
a deadline engine in the plain generated handlers. The optional invocation
adapter below uses the embedding's existing controlled runtime; enabling its
feature does not silently change those plain wire handlers. Adapting request
acknowledgement as completed cleanup would be incorrect.
The embedding must continuously poll the unique driver and retain
it through shutdown. Dropping every route/driver owner is not retirement proof;
panic/unwind recovery is not established. Call IDs must not be reused for later
operations; bounded storage does not retain permanent identity tombstones.

### Page-route checkpoint evidence

Directory: `.work/node032-mouse-route.p09rjP/`.

- `tests-final.log` / `regression.log`: **18** Page-route tests, including all six
  generated wire handlers, exact cancellation identity, first-reason stability,
  duplicate IDs, early error/pending tails, dropped or unpolled responses,
  revocation, independent driver lifetime, fair retirement, separate cleanup
  failures, retained receipt/response quota, admission rollback and overflow.
  Native Mouse **13** and input **71** release regressions also pass.
- `oracle.json` / `oracle.log`: **22** fresh pinned Playwright 1.58.2 cases.
  `live-wire-final.log`: **22/22** match via full binary wire -> generated server
  -> Page route -> Rust native Mouse on Chrome 152, **6.52 seconds**. Each valid
  Mouse action checks reply and retirement identity and zero retained records.
  Fractional-step rejection remains pre-wire typed validation, not a native
  request. Shared Keyboard actions use the same input coordinator.
- Native calls, fixture waits and wire results are individually bounded. The
  installation survives a dropped wire response and drains native retirement
  before owned Context disposal; exact target/Context absence is checked for
  every case. No outer case timeout discards the retained Page owner.
- `clippy-all.log`: strict release all-target Clippy passes. Dependency proofs
  pass for composition **16**, native **8**, portable **3/6/7/10**, input **5**,
  and unchanged safe host **229**; generated schemas remain fresh.
- Only task preflight `DA666432705DA14FAB3BC8C31B380437` was explicitly closed.
  `targets-final.json` verifies it and Mouse fixture Pages absent, with one
  unrelated Page left untouched. Chrome was not restarted. No addon rebuild,
  additional commit or push; previous public-API evidence remains historical
  and artifact-specific. Sequential DOM parity does not prove concurrent
  first-error timing, HTML drag behavior, controlled runtime or public Node API.

Reproduce with the standard dedicated Chrome/Pwright preflight, fresh oracle and
the same environment as the native gate, selecting
`cargo test --release --locked -p pwright-composition-chromium-mouse --test live_mouse -- --ignored --nocapture`.

## Optional controlled runtime invocation

Feature `pwright-composition-chromium-mouse/runtime` adds exactly
`pwright-runtime`: default/opt-in normal closures are **16/17** Pwright packages.
It neither selects an executor nor starts a timer, browser, driver task or a new
runtime. Use `ChromiumMouseInvocation::<C>::for_command(page, &command)` and pass
that same command/invocation to the existing runtime's `dispatch_with`. Six
distinct typed `TargetedService` implementations preserve command identity.

The invocation is single-use and retains the full CallId/Page/method/cancellation
tuple, since the runtime's parameter handler receives only target and params.
It does not admit native/route work while queued: only `call_for` creates the
route response and its retirement ticket. Exact-context cancellation before
start prevents later admission. Wrong cancellation identities have no effect.
The runtime remains authoritative for deadline/lifecycle terminal classification.

Cancellation borrows the same route's retained record and waits for its receipt
to be **consumed via `into_outcome`**, not merely created or dropped. Consumption
releases native admission before waking the cleanup waiter and delivers one
bounded diagnostic copy. The consumer still receives the original cleanup
result and must handle it, including after an already-returned operation error.
The runtime-enabled route profile includes **2 * 4096 diagnostic bytes per
record** for that copy plus the concrete completion state. Retained invocation
tickets continue holding record quota after a receipt frees its native owner.
Invocation/Arc payload has an `allocation_bytes` helper; its caller-owned boxed
futures, queued invocations, returned diagnostics and executor/provider storage
still require a host admission plan. This is not complete FFI memory accounting.

An interrupted cleanup wait can resume; a competing or already-completed waiter
is rejected without replacing the original waker or stealing another receipt.
No mutex spans native polling or an explicit wake. Success/early error returns
normally while the independent driver retains any native tail. Runtime idle
therefore does **not** prove Mouse retirement. Transport closure intentionally
skips the runtime's backend-cancellation callbacks and can return immediately;
response abandonment still leaves the native owner in the Page queue. The host
must revoke/request shutdown as appropriate and drain the unique driver even
after runtime/transport close or cleanup-queue rejection.

For deadline or Page cancellation, execution itself can await the receipt. The
embedding must poll execution/cleanup and the driver concurrently; awaiting only
execution and then starting the driver would deadlock. Enabling `runtime` alone
does not supply this pump. Dropping every driver/route owner, or dropping a
receipt instead of consuming it, is not an acknowledged cleanup protocol.

### Controlled checkpoint evidence

Directory: `.work/node032-mouse-runtime.DAsMP3/`.

- `tests-final.log`: **33** tests pass (**18** route + **15** controlled). New
  coverage includes all six commands, deadline waiting through produced-but-not-
  consumed receipts, caller cancellation, revoked Page disposal, identical
  cleanup errors at both consumers, early click errors with retained tails,
  abandoned cleanup jobs, transport-close retention, queued deadline without
  native admission, resumable/competing waiters, exact context, pre-start cancel,
  delayed-click cancellation without synthetic release and receipt wakeups.
- `oracle.json` / `oracle.log`: fresh **22** pinned Playwright 1.58.2 observations.
  `live-controlled.log`: **22/22** exact event/state cases on Chrome 152 via the
  injected controlled runtime and explicit concurrent receipt pump, **6.17 s**.
  This mode does not exercise generated wire dispatch or Node. It shares one
  runtime and Page input coordinator per case, verifies reply/receipt identity,
  empty runtime calls/cleanup queue and released route records before Context
  disposal, then checks exact native target/Context absence.
- `live-wire.log`: the refactored runner also rechecks the default binary-wire
  path against the same fresh oracle: **22/22**, **6.28 s**. `default-tests.log`
  passes **18** without the feature; `regression.log` passes Mouse **33**,
  controlled Keyboard **18** and shared runtime **113** tests. Strict all-target
  Clippy passes in both modes (`clippy-complete.log`, `clippy-default.log`).
- `boundary-final.log`, `contracts.log` and `host-boundary.log`: **16/17** route
  closures, portable **3/6/7/10** generation/dependency checks and unchanged
  **229-package** host boundary pass. The optional runtime test is now in CI.
- Only task preflight `A67A1CD44B096B8A366EC134F507BD36` was explicitly closed.
  `targets-final.json` verifies it and Mouse fixture Pages absent, with one
  unrelated Page untouched. All test processes exited; Chrome was not restarted.
  The addon remains `1319fa02...ded25dc1`; no additional commit or push.
- Deterministic fakes, not the DOM oracle, prove cancellation/error scheduling.
  Full public Node behavior, first-rejected-Promise parity, HTML drag support and
  host Worker/shutdown integration remain unverified by this checkpoint.

Use the existing fresh-oracle/browser preflight procedure. The same live runner
uses generated binary wire by default, or the controlled invocation path with
`--features runtime`; it prints the selected mode. Both retain the installation
through native retirement before owned Context cleanup, with bounded CDP and
fixture waits and no outer timeout that could discard the native owner.

## Safe Node host installation

`pwright-node-host::mouse` now provides six typed `HostSession::mouse_*` methods.
Calls enter the existing controlled runtime and snapshot the matching native
session plus shared Page input coordinator. Each invocation has an exact route
and a one-record retirement driver; creating that route does not create/reset
Page pointer state. The host's normal dependency closure is now **234**, adding
the five Mouse capability/backend/codec/server/composition packages to **229**.
The portable/default/runtime Mouse leaves retain their independent boundaries.

This host explicitly selects Tokio retirement tasks; the reusable Mouse leaf
still spawns none. There are at most **16 retained invocations/workers** and
**8 admitted native executions**. Host storage admission precedes the native
session/ID snapshot and worker allocation. Bind/spawn shares a mutex with
shutdown's close/wait boundary, preventing a newly registered task after shutdown
has observed an empty tracker. Normal resource failures roll back admission.

An invocation owns a completion guard that drops only after its inner runtime
ticket. The worker separately retains the call lease, exact route and unique
driver until both the invocation has finished and all native receipts have been
consumed. API success/error, caller abandonment and cleanup-job completion can
therefore have different lifetimes without discarding native tails. Workers
always consume receipts before recording cleanup failures, so error reporting
cannot strand a runtime waiter. Failure count and the first bounded diagnostic
remain observable through `mouse_metrics` and host shutdown. An incomplete
started worker records failure instead of silently reporting successful cleanup;
panic/unwind recovery of native work is still not established.

Host teardown first stops new host calls and waits for admitted call futures to
leave. It then closes Mouse admission and joins the independent receipt workers
while the existing runtime cleanup executor remains active, before inspector
retirement, cleanup-executor shutdown and transport release. This ordering avoids
deadlocking cleanup behind an undriven Mouse receipt. Dropping the Mouse
installation signals its workers; it does not abort their native futures. There
is no outer retirement timeout that discards owners: individual native commands
retain the selected CDP timeout. Borrowed Pages/Contexts are not closed by Mouse.

### Host resource plan and limits

- Native wrapper/scheduler allowance is **32 KiB per admitted execution**,
  validated against the concrete backend requirement when binding.
- Per retained call reserves **16 KiB** host overhead plus **65 * (4096 + 8192)**
  bytes for native provider frames, pointer JSON and bounded native diagnostics.
  The actual unpolled CDP provider frame is checked against **4096**; concrete
  retirement/outer-dispatch frames have guards against their host allowance.
  Selected session/ID, invocation, fixed state and delay payloads have layout
  tests. Shared fixed state reserves **16 KiB**, including the first diagnostic.
- Route record/completion leases, shared Page input and native connection
  buffers/pending replies keep their separate production plans. These are
  admission allowances, not eagerly allocated buffers or RSS measurements.
  Nested runtime/executor allocation and allocator-header audit, foreign
  invocation/result storage and artifact-specific N-API validation remain part
  of the final FFI budget gate. This is not a whole-process memory proof.
- The existing connection still has **32 pending native commands**. Mouse's
  scheduler cap does not override that shared limit; extreme click batches or
  overlapping workloads can fail resource admission. Current browser evidence
  covers the catalog's counts/delays, not unrestricted Playwright capacity.

### Host checkpoint evidence

Directory: `.work/node032-mouse-host.icJ1jR/`.

- `host-tests-final.log`: safe host **108 pass / 6 ignored**, including **9** new
  Mouse tests for zero-effect unpolled calls, independent pending-tail retirement,
  cleanup/worker rendezvous, retained cleanup failures, finite count, shutdown
  rejection, rollback, selected layout sizes and installation-drop retention.
- `oracle.json` / `oracle.log`: fresh **22** pinned Playwright 1.58.2 cases.
  `live-host.log`: **22/22** exact event/state matches via the actual HostSession
  Mouse/Keyboard methods, **7.30 s**, using host-created Context/Page routes.
  Each case checks worker/quota drain and native target/Context absence; final
  host close checks zero active leases, operations and cleanup jobs.
- `live-close-borrowed.log`: real mousedown is observed before host shutdown
  during a **60-second click delay**. The API returns TargetClosed, no mouseup or
  click appears in the surviving borrowed Page, and host leases/operations/
  cleanup are zero, **0.32 s**. The observer then disposes only its exact owned
  native Context and verifies target/Context absence.
- `live-close.log` is a failed test-design attempt, not a passing observation:
  the host-created Context was disposed automatically on detach, making a
  post-close DOM snapshot impossible; redundant disposal also failed. The
  corrected probe keeps an observer-owned borrowed Context alive, preserving
  the no-synthetic-release assertion rather than counting disappearance as proof.
- `addon-check.log`: release addon compile-check passes; `clippy-complete.log`:
  strict host all-target Clippy passes; Node offline **31** and strict types pass.
  `host-boundary.log` / `mouse-boundary.log`: host **234**, Mouse **16/17**.
  No new addon was linked/copied and the public package still exposes no Mouse.
- Final-source `host-tests-complete.log` and `clippy-last.log` also cover the
  admission-before-snapshot assertion. `targets-final.json` confirms task
  preflight `C0BCA133EE1432D2A673AF557516C0F2` and Mouse fixture Pages absent,
  leaving two unrelated Pages untouched. All test handles completed; Chrome
  was not restarted. The addon hash remains `1319fa02...ded25dc1`, with no
  additional commit or push.

Reproduce the host gate with the same fresh oracle and dedicated Chrome/Pwright
preflight, selecting `cargo test --release --locked -p pwright-node-host --test
live_mouse -- --ignored --nocapture --test-threads=1`. No new browser process or
driver is launched by the Rust library; the test attaches to the selected
dedicated loopback Chrome and uses serve-lib fixtures.

## Click result versus native retirement

`MouseClickOptions` validates coordinates and composes a button, bounded count,
move steps and injected delay. `MouseClickCount` accepts integral finite counts,
normalizes nonpositive counts to zero, and caps one execution at **32 pairs**.
The cap is explicit Pwright resource policy, not unrestricted upstream parity.
Double-click uses count two. The scheduler owns at most **65 native future slots**
(one move plus 64 button events), with no spawned task or internally chosen clock.
The caller supplies its delay future. The native factory above now accounts for
local execution payload and concurrent admission; provider allocations remain
an explicit embedding requirement.

- Without delay, move (at most one step), down and up are submitted in one poll
  without waiting for earlier replies; every submitted future is first-polled
  even if an earlier call immediately fails. More than one move step is awaited
  before the button batch, matching the pinned server implementation.
- With positive delay, move/down/wait/up are ordered, with an additional wait
  between click pairs: `2 * count - 1` waits. Zero count moves only.
- The caller awaits `&mut execution`, then **always** awaits `execution.retire()`,
  including after error or stopping result polling. Early operation failure can
  leave native calls in flight. Retirement stops future planning, drops the
  injected wait, and drains retained replies; it sends no synthetic release and
  does not retry. Dropping the execution is not native-retirement proof.
- An execution retains Page input admission and the native client owner after
  its originating coordinator/transaction drop, through result and retirement.
  The Page route and controlled invocation preserve this owner under response
  cancellation; the host's independent workers now retain it through shutdown.
- Multiple errors becoming ready between polls currently use slot observation
  order. Equivalence to Playwright's first rejected Promise timing is **not
  established**. The public layer must not claim concurrent error parity yet.

## Click checkpoint evidence

Directory: `.work/node032-click.5E0hZb/`.

- `oracle.log` / `oracle.json`: fresh **22** pinned cases, written after owned
  Context cleanup; `native.log`: **22/22** exact DOM event sequences/outcomes,
  **6.77 seconds**. The original 12 cases remain, plus single/double/triple clicks,
  integer/fractional positive delay, middle/right clicks, zero/negative counts
  and a double-click with shared Shift. These snapshots do not assert wall-clock
  delay precision, concurrent error timing or HTML drag interception.
- `unit-final.log`: input **71**, model **6**, fake **23** pass. The 12 click tests
  cover batch submission before replies, multi-step ordering, delayed prefixes,
  early error with a pending tail, unpolled/cancelled retirement without extra
  events, fixed capacity and raw action state capture before previous replies.
  A production ledger test verifies the escaped execution retains input admission.
- `clippy-final.log`: strict release all-target Clippy passes. The initial
  collapsible-if lint was corrected, not suppressed.
- `regression.log`: native Keyboard **18**, Locator **103**, root composition
  **295 pass / 15 ignored**, safe host **99 pass / 6 ignored**. `dependencies.log`
  retains the **5-package** input-only proof. `node.log`, `types.log` and
  `boundary.log` pass **31** offline Node tests, strict types and the **229-package**
  host/JS boundary. Format and diff checks pass. The addon hash remains
  `1319fa027f4bd5ee0d3e2f3932dce1c01ad5b8de86381b6817970ee0ded25dc1`;
  earlier public-browser results remain specific to that unchanged artifact.
- Native commands and fixture waits have five-second bounds. Click result
  polling is bounded separately; its owner is retained while native replies
  retire under their own CDP deadlines. There is intentionally no outer case
  timeout that could drop that owner. The catalog/action/work limits bound the
  complete sequence. Exact target and Context absence is checked after cleanup.
- The CLI accepts its local tab ID, not the raw target ID: the first close
  attempt returned success without removing the preflight, as preserved in
  `targets-final.json` and `targets-latest.json`. Closing its verified local
  `tab_00000000` then removed exactly target `63B976AE4B30922385EDE3265A422AC5`;
  `targets-after-local-id.json` proves absence. Two other tasks' Pages remain
  untouched. Chrome was not restarted; all test processes exited, with no addon
  rebuild, additional commit or push.

## Earlier primitive checkpoint evidence

Directory: `.work/node032-raw-mouse.9xHyGb/`.

- `oracle.log`: initial failure on public non-integer `steps`; no oracle file
  was published from that incomplete attempt.
- `oracle-final.log` / `oracle.json`: fresh pinned oracle with all **12** cases.
  Unexpected errors fail the oracle; only the catalog's explicit fractional-step
  validation error is classified. Evidence is written exclusively after owned
  Context and connection cleanup.
- `native.log`: all **12/12** exact DOM event sequences and classified outcomes
  match, **3.87 seconds**. The final-source rerun `native-final.log` also matches
  **12/12**, **3.60 seconds**, after the shared Locator interpolation correction.
  Cases cover origin down/up, fractional coordinates,
  integer interpolation, held drag pressure, multiple buttons, middle/up-unheld,
  zero/negative steps, fractional-step rejection, click-count payloads, shared
  keyboard modifiers and native wheel with Shift/a held button.
- Wheel observations wait for a delivered fixture wheel event before reading
  the snapshot; they do not assert API settlement or scroll completion timing.
  Each side uses fresh owned Context/Page state. The native runner checks exact
  target and Context absence after every case, including failed comparisons.
- `final-regression.log`: input **58** (12 new pointer tests), input model **4**,
  Locator **103**, native Keyboard **18**, root composition **295 pass / 15 ignored**,
  safe host **99 pass / 6 ignored**. Ignored browser tests are not inferred green.
  `unit-complete.log` additionally records CDP **73** and fake **23** passing.
- Strict release all-target Clippy passes (`clippy-last.log`); input-only
  dependency proof passes (`dependencies.log`). No new addon was built, so the
  previous public Keyboard **24/24**, full lanes and reload **16/18** remain
  artifact-specific historical evidence, not validation of newly linked code.
- The task preflight `0FEB820EEB04092CA6769E62642EEFB4` is absent from
  `targets-final.json`; no Mouse fixture Page remains. Only that exact preflight
  was explicitly closed outside test-owned Context cleanup; Chrome was not
  restarted. The final inventory contains zero Pages.
- The final-source rerun used a separately recorded preflight
  `795A6184F207999F01C193C474AE9C5D`; `targets-latest.json` verifies both preflight
  IDs absent and zero fixture/other Pages. All test processes have exited.
  Offline Node **31/31**, TypeScript, **229-package** host/JS boundary, fmt and
  diff checks pass. The addon remains `1319fa02...ded25dc1`; no additional commit
  or push was performed.

After dedicated Chrome/Pwright preflight and verified serve-lib registration:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_MOUSE_FIXTURE='<verified full MagicDNS URL>/raw-mouse.html' \
PWRIGHT_MOUSE_ORACLE='/absolute/new/task-directory/oracle.json' \
node packages/pwright/scripts/probe-raw-mouse.mjs

PWRIGHT_CDP_WS_URL='<verified loopback browser WebSocket>' \
PWRIGHT_MOUSE_FIXTURE='<same verified full MagicDNS URL>/raw-mouse.html' \
PWRIGHT_MOUSE_ORACLE='/absolute/new/task-directory/oracle.json' \
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-backend-cdp-input --test live_mouse -- --ignored --nocapture
```

The fixture caps records at 256; the runner caps oracle input at 1 MiB and action
count at 12 per case. The current runner bounds commands, fixture waits and click
result polling individually as described above, preserving native retirement
before Context cleanup. Diagnostic timeout or unexpected protocol error is
failure, not a match.

## Remaining design and implementation tasks

1. Complete concurrent first-error ordering and mixed Keyboard/Locator/Mouse
   state-capture verification. Click scheduling is implemented above; do not
   move the sequence into JavaScript or equate sequential DOM snapshots with
   all overlapping-command behavior. Enforce result/retirement separation in
   the admitted production owner, including native future-storage accounting.
2. The cancellation-aware dispatch seam and native factory/Page selection are
   implemented (NODE-034/035). A budgeted Chromium provider and shared Page-owned
   host selection/teardown remain to implement.
   Cover native
   HTML drag interception/drop and forClick bypass independently from pointer
   dragging. The current fixture deliberately contains no HTML draggable node.
3. Contract layer is implemented above: keep its generated outputs, method
   ownership, six independent services and exact dependency proofs intact while
   integrating the native backend. Do not add a runtime or browser dependency
   to the portable Mouse capability/client/codec/server packages.
4. Native admission, Page-route retirement, optional controlled invocation and
   host-owned receipt workers are implemented above. Finish the FFI storage/
   nested runtime allocation audit and verify foreign Worker/transport teardown
   with the newly linked addon. Runtime idle must not imply native retirement.
5. N-API and the stable `page.mouse` facade now expose move/down/up/click/
   dblclick/wheel. Keep JS limited to validation/forwarding and result wrapping, not automation,
   inject delays, own pointer state, launch a driver or bundle Chrome.
6. Public subset, navigation/state sharing, multiple Pages, Worker teardown and
   delayed-click close now pass on the hashed addon above. Complete exact error
   fidelity, wider concurrency/transport-loss cases, full artifact-specific
   regressions. The unchanged shared corpus now completes as recorded above but
   retains a topology setup failure; do not count it as a green gate.
   Download, upload, true Touchscreen and native reload-close remain separate
   open work; none is waived by this primitive checkpoint.
