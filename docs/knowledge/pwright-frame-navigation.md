<!-- agent-updated: 2026-10-06T06:59:02Z -->

## NODE-366: post-commit client redirect keeps the original response

The navigation owner now separates the response selected by the original
`Page.navigate` document from the lifecycle document that must satisfy the
public `waitUntil` contract. If page script starts a same-Frame client redirect
after the first document commits, Rust adopts the next navigation in the
already admitted Page lifecycle registry, waits for its requested state and
still returns the original response. The original request identity stays in
the bounded navigation arena for the independently selected response-body
capability; Node does not retain a second response or run a lifecycle loop.

The thin Node error adapter adds only Playwright's timeout call-log rendering
after Rust has selected the typed timeout. It does not change cancellation,
deadline or lifecycle semantics. The unchanged four-case
`tests/page/frame-goto.spec.ts` selection passes 4/4 candidate and 4/4 oracle.
Candidate/oracle report SHA-256 values are
`4ab237b0a8a1d47cf9f03b24a49d6dd0d10f2e96f794aba857683e74fa01be86`
and `1dd1411a3aa357c80f43ddf3bf895f6085a796453f5700e732619e6f03058ee9`.
The broader Core and Python corpora remain incomplete.

NODE-140: The complete 29-case Frame corpus still passes only 27 cases; both candidate and upstream response-arrival Page.close observations exceed the unchanged diagnostic deadline. A separate oracle control remains pending even after body EOF and goto(load) completion. Committed-document controls pass; they do not replace the failing arrival gate.
[Expanded verification and retained failures](testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

## NODE-139: Page lifetime through navigation publication

The selected Rust executor now keeps the exact PageLifecycleHandle after dropping
its native route and navigation registration. A Page-scoped action is not finished
while its topology-publication fence is pending. Actual Page or registry sealing
interrupts that fence, and the final synchronous bounded result projection rechecks
Page/connection liveness. A live Page preserves its native navigation error or
publication capacity failure. Close intent alone is not closure, a renderer route
swap is not Page death, and ERR_ABORTED is not blanket-mapped to TargetClosed.

The borrowed Page closed() future registers both existing-owner Notify waiters
before checking terminal flags, so closure before first poll or during registration
cannot be lost. It retains no registry, transport, native route or navigation slot.
Notify bodies are included in the existing Page/registry size_of-based resource
profiles; the embedding bounded Frame operation admits the inline future. There is
no new worker, cursor, unbounded waiter pool or production JavaScript automation.
Other embedders must bound their concurrent operations and admit their waiter state.

Pinned source is Playwright **1.58.2**, tag commit
`ce480a952553175eae75342aad2c5e86cdf2cbba`, not the newer upstream checkout HEAD.
In that revision, server/frames.ts raceNavigationAction keeps the Page openScope
through the action, while requestFailed aborts only a matching pending document.
After commit, a loading failure does not replace the lifecycle wait with a new
navigation error. Page._didClose seals openScope with TargetClosed. These distinct
boundaries do not justify treating every Context close request as immediate death.

### Corrected interpretation of the prior natural comparison

The bounded native journal in
`.work/node139-navigation-close/{candidate,oracle}-before-lifetime.json` used the
unchanged NODE-138 artifact `02e4d9be871525903927cb6e534627831ba082659cda380d691c18b621f8872f`.
In the Context-first case, candidate loader `06C...` had no matching frameNavigated
before loadingFailed; its navigation error settled before Target.detachedFromTarget.
Oracle loader `974...` had init and frameNavigated before loadingFailed, then closed
while waiting for load. The runs were in **different native document phases**.
Their error-type difference is therefore not a proven same-phase semantic defect.
The existing Rust reducer already distinguishes pending and committed documents.

The new frame-navigation-close lane deliberately waits for published page.url()
to equal the streaming document URL while goto(load) remains pending. It compares
Context close, Page close, an already-delivered network error, sibling isolation,
and an already-delivered commit-only success. All **five paired cases passed on
the old artifact** (`stable-phases-baseline.log`); they are regression controls,
not a red/green demonstration of the publication-lifetime fix. The new deterministic
Rust tests exercise the actual pending-fence boundary separately. The diagnostic
journal records at most 64 owned-Page phase events and bounded navigation reply
identities/errors without altering the forwarded wire buffers.

### Rebuilt artifact and final verification

Only the experimental `concurrent-close-drag-frame-extra-headers` artifact was
rebuilt, atomically replacing its own native file. SHA-256:
`1b18f3cca8fa45e351c50bed6d3dc42be25a1a67c77d2895bf4638d052d90efc`.
The default artifact remains
`d7d70cb36b0870858e2109ba69c67c6eb06da855dc2a8ea65158a297f8c45d95`;
the earlier drag/frame/header combined artifact remains
`6406b660904da3fbbf51bb40ad383c4d18b9a0591472de7b7b5d4e184dd45525`.
Other frame-navigation profiles were not rebuilt by this checkpoint.

Rust release evidence under `.work/`:

- `node139-composition-final.log`: 521 passed, 16 existing ignored, with
  native-close-retirement and frame-lifecycle selected. The six new lifetime
  cases cover all waiters, pre-poll retirement, sibling isolation, registry seal,
  cancelled-wait replacement/zero final leases, and ordinary document progress.
- `node139-lifetime-wakes-final.log`: all six pass again after strengthening the
  multi-waiter case to assert actual wake counters, not just ready polling. The
  borrowed future fits the tested 256-byte inline allowance.
- `node139-host-final.log`: 482 passed, 10 existing ignored, on the full
  concurrent-close/drag/frame/header selection. Nine new publication-lifetime
  tests cover closed-before-publication, closure during success/error/fence
  failure, a permanently pending fence, simultaneous readiness, live-error
  preservation, live capacity errors and ready success.
- Combined host/addon, selected composition and ordinary host strict Clippy pass
  (`node139-{host,addon,composition,cold}-clippy.log`). JS contracts pass 89;
  nine host/ten addon dependency selections pass. Formatting, diff whitespace
  and all ten new documentation routes were checked.

Final Chrome evidence is under `.work/node139-navigation-close/`, using pinned
Playwright 1.58.2 and the dedicated loopback CDP browser. The supervised lanes
pass frame-navigation-close 5/5, native-close-confirmation 6/6, lifecycle-events
7/7, browser-connection 5/5, collection-lifecycle 1/1 and page-is-closed 8/8:
**32 test cases**, with original deadline and panic-failure checks intact.
They cover scoped lifecycle and teardown regressions, not the complete Frame
navigation/package compatibility corpus.

The four `{oracle,candidate}-{transparent,hold-retirement}-final.json` reports
pass both close orders: **eight additional case executions**. In the controlled
candidate Page-first case, three retirement packets remain held and Page.close
remains unsettled when the owned Context's native disposal is dispatched, with
two host/FFI close operations admitted. Both closes then fulfill, lifecycle notices
retain their exact ordering/identity, all candidate host/FFI counters return to
zero, and no probe-owned native target remains.

The final transparent Context-first **oracle also returns net::ERR_ABORTED**:
loader `6CD6E8787382B939AEB090C305DA4864` has loadingFailed but no matching
frameNavigated. The candidate likewise retains its precommit ERR_ABORTED, while
both Page-first cases return TargetClosed. This is consistent with phase-sensitive
settlement, not evidence that every close must rewrite a previously ready network
error. Exact native-error formatting is not certified by these category controls.

No commit or push occurred; foundation commit f76e3f2 remains the local baseline.
No new persistent server, browser or worker was left running. At 19:29:31Z the
verified shared backend PID 53189 had RSS 2,866,960 KiB (about 2.73 GiB), down
from 3,531,392 KiB at 19:17:41Z, with zero swap. This is shared-process RSS,
not a per-chat measurement or a unique-RAM sum. Complete compatibility and
allocator/peak accounting remain open.

NODE-130: Selected deletion-only route retirement preserves exact surviving navigation entries and revokes only disposed Frames on commit. Preparation/drop does not revoke snapshots. A changed surviving navigation identity is rejected instead of reconstructing a provider during retirement.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-123: Transparent traces isolate a candidate-only ancestor-close scheduling discrepancy at the held-response boundary. A successful repeat settled Page.close before the Context request, so it is not a fix. A completed-document/held-retirement control proves Context dispatch remains blocked with two admitted host/FFI operations, unlike pinned upstream. Context terminal semantics and the later producer/queue-loss settlement window are separately fixed and tested; the full Frame corpus is not reclassified as green.
[Close cascade contracts and evidence](pwright-close-cascade.md).

## NODE-122: Close-confirmation regression checkpoint

The Frame-navigation harness now accepts all four native profiles that actually
contain `frame-navigation`, including `drag-frame-extra-headers`. Its fixture
directory is `tests/fixtures`, not the package's smaller action-fixture directory.
Both required static documents were verified HTTP 200 through a temporary
Tailscale-bound serve-lib registration before the valid run.

`combined-frame-navigation-verified-fixture.log` records **27 passed, two failed**
under `.work/node122-native-confirmation/`. The candidate-only arrival-boundary
close case exceeded its Context-cleanup diagnostic deadline after Page.close
did not settle. The final paired arrival-boundary reproducer failed in the
**oracle** Page.close observation, before reaching the candidate. The separately
instrumented document-execution close boundary and timeout-continuation cases
passed. NODE-077 already records this unresolved arrival-boundary close stall;
these observations do not prove a new isolated browser cause or a complete fix.

The selected Rust confirmation now waits for native destruction rather than
declaring failure after twenty absence checks. No stopLoading, retry, synthetic
receipt, ignored assertion or longer diagnostic deadline was added to make this
suite green. Browser/Worker interruption is separately verified by the controlled
held-native-receipt lane. Earlier unconfigured/incorrect-fixture runs are retained
as setup failures, not counted as behavioral passes.

[Selected close-confirmation contract](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

# Frame navigation: Rust ownership and Playwright compatibility

NODE-097 reproduces the paused-child retirement/5-second timeout signature with
an upstream owner too. A second client's Page-scoped auto-attach is sufficient;
the direct matrix compares original success/failure and a separate recovery
navigation, passing 11/11 combined/default. This is not a Rust navigation fix,
a production retry, complete call-log parity or attribution of all old failures.
[Evidence](pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 corrects untimed failure-snapshot interpretation: the measured matching
load follows the 5-second deadline and Page.stopLoading. There is no demonstrated
timely-load matcher failure in that trace. A controlled borrowing-peer shutdown
matrix passes 4/4 on the combined artifact; native child retirement under churn
still needs attribution. No navigation production code or deadline was changed.
[Checkpoint](pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 distinguishes a pending initialization call whose exact native session
has detached from a navigation timeout with no native call pending. Selected
reader-level retirement addresses the former; the latter still needs lifecycle
evidence. The child-header diagnostic now retains a separately bounded initial
navigation trace keyed by the actual command session/target and reply loader.
[Evidence and limits](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-091 preserves selected Frame lifecycle observation through a delegating
header observer. Headers alone do not select lifecycle/navigation domains.
Native OOPIF header-close comparisons are distinct from full Frame navigation
parity. [Contract](pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-090 adds explicit abandonment of a lifecycle registry's prepared but never
published Page. Unlike ordinary reconciliation this seals an unseen owner;
escaped handles retain their lease, and live replacement owners are unaffected.
It does not change navigation dispatch/wait semantics or prove OOPIF retirement.
[Initialization cleanup](pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-087 selected Frame.goto captures Page header policy synchronously in Rust
before N-API scheduling. Tests cover both setter-before-goto and goto-before-
setter invocation orders, within a passing combined 30-test public lane. Native
fanout, broader Frame/Worker navigation and NODE-077 close behavior remain open.
[Preparation and scope](pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 reproduces Frame.goto reading stale Page headers after an unawaited setter:
two candidate requests were sent where upstream rejected before navigation. The
optional Rust overlapping model is implemented, but ordered host preparation and
concurrent native settlement remain pending. Sequential NODE-085 passes do not
prove this concurrent behavior.
[Race and integration requirements](pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 integrates Page-only referrer reconciliation into public `frame_goto`
when the header feature is also selected. It runs before native dispatch; the
raw `navigate_frame_native` port remains lower-level. The thirteen-case Frame
comparison passes within the combined 24/24 header regression. This does not
resolve the independent NODE-077 close-at-server-write failures or prove full
Frame navigation, pending setter races, child-frame or OOPIF parity.
[Referrer contract and gates](pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-078 verifies the separate Rust Page-referrer/Context-header merge policy
against 484 pinned internal cases and repeats 23 public upstream observations.
The Frame executor has not yet installed that settings owner: native session
fanout, transactional setters and FFI creation options remain required before
the existing public `referer` option has full extra-header reconciliation.
[Implementation boundary and next integration](pwright-extra-http-headers.md).

## NODE-077 causal close, timeout continuation and Worker teardown

This checkpoint changes test infrastructure, not production Rust/JS behavior.
Both NODE-076 addon hashes are unchanged. The earlier independent Rust close
success was not a stable guarantee: the new standalone-profile run reproduces
`TargetStillOpen` for exact target `947A7E675C51494B5A41AE393098895E` at the
original server-write boundary. The combined run passes that Rust case; both
still fail the upstream Page-close deadline. Keep all runs, not only the pass.

### Causal boundaries without a production workaround

The retained arrival case signals when the server queues its response fragment;
it does **not** prove browser response delivery or document commit. The added
case holds the body open but sends a beacon from script execution in the new
document before calling Page.close. Both implementations then close and reject
pending Frame.goto with a closed error. Since the beacon adds content and
scheduling, this establishes a passing execution boundary, not an isolated
Chromium buffering cause. No close retry, extended confirmation count,
unconditional stopLoading or synthesized success was added.

Current source inspection confirms ordinary upstream close delegates to browser
Target.closeTarget and waits for its closed promise; Rust sends the same command
and requires bounded target-absence confirmation. The older
[raw-CDP reload isolation](pwright-page-reload.md#minimal-raw-cdp-isolation)
already reproduced acknowledgement without removal without host/addon/runtime.
That historical diagnostic is not re-labelled as a new Frame-native trace.

Three new paired timeout-continuation cases use DOMContentLoaded, load and
networkidle. Each waits for execution of the new document, observes the original
2-second API timeout and readyState=loading, then ends the held response. Both
implementations execute its final script and finish loading, with exactly one
document request. Thus API timeout releases the waiter without stopping/retrying
the browser load at these tested boundaries. This does not prove every redirect,
cross-process handoff or arbitrary cancellation policy.

### Worker environment ownership

`pnpm test:frame-navigation-workers` runs three candidate-specific native
integration cases under the same bounded supervisor. Upstream is only a native
identity/borrowed-Page observer here, not a substitute candidate driver:

1. **Owned running:** terminate after the held new document executes while
   Frame.goto is pending; the exact native Context and target must disappear.
2. **Borrowed running:** terminate at the same boundary on an explicitly
   test-owned external Page; that Page must remain live/loading. Ending the
   server response must still execute its final script. Only the test owner
   closes it afterward; the shared default Context is never disposed.
3. **Owned queued:** block only the Worker JS event loop until Rust reports one
   queued Command and at least the 512-KiB FFI reservation. Termination then
   exercises cleanup while the typed Frame result is awaiting N-API conversion;
   its exact native Context and target must disappear.

Each case uses one Worker, one dynamic Tailscale server and fixed control
messages. Queue observation is bounded to 10 seconds; Worker blocking to 30;
case to 25 and supervisor to 180 seconds. Native panic/cleanup diagnostics fail
the lane. Cleanup targets only recorded native identities. Post-termination
environment ledger values cannot be read: these tests prove native ownership
and clean Worker exit, not an exact post-destruction RAM or ledger measurement.

### Fresh evidence

Directory: `.work/node077-frame-close-boundaries/`.

| Profile / lane | Result | Duration |
| --- | --- | --- |
| frame-navigation / public | 27 paired matches; Rust arrival-close and upstream arrival-close fail | 59.798 s |
| drag-frame-navigation / public | 27 paired matches + independent Rust close; upstream arrival-close fails | 65.001 s |
| frame-navigation / Worker ownership | 3/3 pass | 1.704 s |
| drag-frame-navigation / Worker ownership | 3/3 pass | 1.743 s |

All four processes exited normally, with nonzero status for both public lanes;
no cases were skipped or cancelled. Failed-close fixture cleanup still completes
and candidate runtime/FFI checks drain, but cleanup is not a successful Page.close.
Node contract units 47/47, strict types, dependency boundaries, JS syntax and
diff checks pass. No addon rebuild, new production dependency or push occurred.
Header/referer reconciliation, exact errors/redirect adoption, broader Worker
interleavings and full Playwright coverage remain open.

Handoff: all NODE-077 runners are terminal, temporary serve-lib route
`/pwright-node077-navigation` is removed, and preflight target
`79BC151A276EE7FDE6A0B65AA4DD1E12` is absent. No task-owned test runtime remains.
Shared Chrome/XLQY services are untouched. The 09:27:34 UTC snapshot records
shared backend RSS about 1.67 GiB and zero disk swap, not per-chat RAM.

## NODE-076 public Node Frame navigation

`Frame.goto(url, { waitUntil?, timeout?, referer? })` now calls the selected
Rust host through N-API. The existing Context-owned URL-value port resolves
baseURL first. JS performs language argument checks and constructs the public
Response wrapper; dispatch, lifecycle matching, deadlines, cancellation and
response parsing remain in Rust. A Response stores the exact navigated Frame
wrapper, including a child Frame; existing Page.goto/reload retain mainFrame
identity. The addon result retains the selected portable Frame ID.

### Independently selected packaging

The addon `frame-navigation` feature forwards the host feature and selects the
existing navigation 1.2 value types. Four fixed native profiles are accepted:
`default`, `drag-runtime`, `frame-navigation`, `drag-frame-navigation`. The last
one combines both independent Cargo features. Each nondefault profile builds
to `native/<profile>/pwright.node`; import-time selection verifies both compiled
capability flags. Unknown/nonstring names, missing files and mismatched flags
fail instead of falling back. Default and earlier drag artifacts are untouched.
This adds no Node driver, browser launcher, transport or new normal dependency
package. The embedded host calls its typed executor directly; generated remote
FrameGoto service installation/negotiation remains separate unfinished work.

### FFI ownership

The invocation admits a 512-KiB Command-lane FFI ticket before copying borrowed
JS inputs. The target pair is a borrowed two-element JS array with bounded
Page/Frame ID strings, not an eagerly deserialized Vec. URL/referrer are each
at most 16 KiB; waitUntil is at most 32 bytes. The ticket covers inputs, Rust
conversion copies/header map and foreign string conversion overlap. Native
response arenas, the 384-KiB typed output owner and boxed async-body storage
retain their independent existing charges.

`FrameGotoOutput::value()` projects a borrowed result or bounded HostError copy.
The output wrapper retains its result lease through the N-API settlement
callback. The existing publication fence publishes the Frame URL before foreign
delivery; the output is dropped only after conversion. No external pointer,
finalizer-owned Rust result or new event cursor is introduced. JS Response
metadata is a language-owned copy after delivery.

### Public verification and open discrepancies

Evidence: `.work/node076-public-frame-navigation/`. The new supervised
`pnpm test:frame-navigation` lane compares against pinned Playwright 1.58.2 on
dedicated Chrome 152 over loopback CDP. Fixtures come from repository
`tests/fixtures` via serve-lib; dynamic held-response/referer servers use the
explicit Tailscale bind and close in finally. Each candidate disconnect asserts
zero active runtime leases, cleanup tasks and pending/reserved FFI bytes.

The first run passed 22 cases and timed out inside the upstream held-response
Page-close case before it reached the candidate. This is retained as a failed
run, not a complete conformance pass. The harness now applies an independent
diagnostic close deadline and runs the Rust close case independently before the
paired reproducer; API navigation timeouts remain unchanged.

Final supervised results (both exited nonzero, not killed):

| Artifact / log | Passed | Failed | Duration |
| --- | --- | --- | --- |
| frame-navigation / `live-bounded.log` | 23 paired cases + independent Rust close | upstream held-response Page.close | 47.847 s |
| drag-frame-navigation / `live-combined.log` | 23 paired cases + independent Rust close | same upstream Page.close | 48.248 s |

Both failures report `Diagnostic Page close during held Frame navigation
exceeded 10000ms` before the candidate phase. The Rust-only close case passes
and drains runtime/FFI metrics. This does not establish close parity or diagnose
the upstream/Chrome stall; preserve the original `live.log` supervisor kill too.
Successful pairs cover four waits on main/local/sandbox child Frames, synchronous
URL publication, parent/sibling isolation, about:blank/data/null, hash document
preservation, 404, baseURL, referer, held commit/timeout, detached/invalid-wait
rejection and retained Response identity after Page close. Sandbox child coverage
is not proof of every cross-site process handoff or Worker lifecycle.

The combined artifact additionally passes the existing drag/Keyboard/Locator
10/10 public comparison (`combined-drag.json`, `combined-drag.log`), including
owned Context/target removal and zero host owners. Other final checks:
239 Rust host units pass (seven native tests ignored), 47 Node contract units,
strict TypeScript, nine selected dependency boundaries, default addon release
check, selected host/addon and combined addon strict Clippy, fmt and diff checks.
CI steps were added but no remote CI run/push is claimed.

Artifact SHA-256:

- Frame navigation: `2a43ee309e2e4a41bd73252536d1a69732d6b7e0bdab6a35a3e664e7ea77547e`.
- Combined: `e47ff341041eeb92f7bc041cd741dc4761d0b60b7142aedfb33625ba3c4f23d1`.
- Unchanged default: `8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.
- Unchanged drag: `1b627f190a51b5e174fdb0b357eb2e8a37901c23d215f9483ba554b99888e3c0`.

All test/build handles are terminal. The exact preflight target
`795E4CFEFFDB7BF31FEFAFF868B7F2F2` was closed and only the temporary
`/pwright-node076-navigation` serve-lib route was deregistered. Shared Chrome,
XLQY processes and other registrations remain untouched. At 09:16:43 UTC the
shared Codex backend RSS was about 898 MiB and disk swap remained zero;
these are process/system figures, not a per-chat memory measurement.

BaseURL and explicit referer are exposed. Context/Page extraHTTPHeaders and
referer conflict reconciliation remain unsupported. Exact error text,
redirected-navigation adoption, download/interruption parity, Worker teardown
and broader Playwright API coverage are not proved by this selected lane.

## NODE-075 typed Rust host result ownership

The selected host now exposes `HostSession::frame_goto(page_id, frame_id,
v1_2::GotoParams, timeout_ms) -> FrameGotoOutput`. It moves URL/referrer inputs
into the existing native executor and maps all four selected wait policies in
Rust. The URL must already be resolved. This is host integration of the typed
values, **not yet** installation of a `Service<Command<FrameGoto>>` in the
generated server chain or a public Node method.

`FrameGotoOutput::outcome()` borrows either the generated `FrameGotoResult` or
typed navigation error. There is no consuming extraction/Clone API that would
drop the owner's charge while its returned allocations remain live. A caller
copying borrowed values must admit a separate copy. The output retains no
HostSession, Page registration or native response arena; it remains usable after
host close. Native initiation/host errors remain outer HostError, while settled
wait errors map to typed failure kinds/details. These diagnostic messages have
not yet passed public Playwright error-string parity.

### Projection and resource bounds

- New namespace 126/3 admits 32 active-or-escaped result owners, each with a
  384-KiB allowance. A separate Arc counter at 126/4 enforces the local count
  even when a combined resource plan has spare budget. Failed reserve rolls the
  count back. Admission occurs before native dispatch, not after navigation.
- Native JSON input is at most 64 KiB, URLs at most 16 KiB, status text at most
  1 KiB, and the header map at most 256 entries. A custom bounded header visitor
  avoids building a general JSON tree; unknown response fields are ignored.
  Escaped values are decoded and headers canonically ordered in Rust. Malformed
  fields/oversized header maps become InvalidBackendResponse details; source or
  URL/status limits report ResourceExhausted. HTTP 404 is a successful response.
- The allowance covers parser scratch, borrowed/escaped intermediate values,
  final strings, both bounded header arrays and owner/error metadata. A
  compile-time inequality checks the documented four-source-size + two-header-
  array + metadata model; a maximum-header/large-string test checks actual
  retained capacities. Native arena, allocator bookkeeping, caller task stack
  and future foreign copies retain their separate boundaries.
- The existing executor's TaskTracker token, eight-operation admission and
  absolute deadline now remain alive through bounded synchronous projection.
  Connection/deadline checks precede returning the final owner. No new task or
  CDP cursor is created; dropping a pending call still does not stop loading.

### Verification

Evidence directory: `.work/node075-typed-host-navigation/`.
The actual selected Rust host test passed in 5.66 s: raw native regression,
typed HTTP 200/headers, hash/networkidle/null, HTTP 404, typed OOPIF child response
and published URL with unchanged parent/sibling, dropped-call/timeout/host-close
cancellation, and a typed result retained past host close. After releasing Page
handles, the escaped result still held leases; dropping it restored zero leases.
Context `BC4D9756360CBE3D2410BC1FF8D6A10D` was disposed and its absence asserted.
The later compile-time budget assertion/capacity-only unit addition does not
change native behavior; no second browser run is claimed for those additions.

The final selected host run passes 238 units with seven ignored native tests,
including the maximum-capacity case. Selected all-target strict release Clippy
passes in 2.42 s; readiness-only strict Clippy passes in 4.45 s and the default
addon release source check passes in 4.54 s. Default and selected host normal
dependency closures are identical at 105 Pwright packages. Fmt/diff checks pass
after formatting the final test/assertion; retained default/drag addon hashes
are unchanged. The fixture registration and exact preflight tab were removed;
unrelated XLQY mounts/Chrome stayed alive. All task-launched checks completed.
Final 08:50:55 UTC memory snapshot: shared backend RSS about 1.24 GiB, zero swap;
this is not per-chat memory attribution. No task-owned server/browser remains.

Next: Context baseURL/referrer/header reconciliation, budgeted foreign response
and error conversion, correct Node Response.frame identity, generated-service
selection where required, and pinned public Frame.goto differential gates.
Redirected-navigation adoption and earlier attachment reliability remain open.

## NODE-074 opt-in typed navigation 1.2 contract

`navigation-1.2.yml` adds `FrameGoto` at method 65540, with command/result
tags 7/8. Its only target kind is `pwright.browser.frame`; existing Begin/Goto/
Reload remain Page-only with unchanged methods, tags and payload fields. A
success carries a nonzero portable `frame_id: u64` and optional HTTP response,
not a CDP Frame ID string. HTTP error statuses remain successful responses;
same-document/no-response success uses `None`. Errors retain typed navigation
details and the common failure kind.

The root `navigation.yml` and root APIs remain 1.1, with an exact frozen 1.1
baseline. Capability, codec, client and server crates expose default-off
`frame-navigation` features. Selected models/codecs/server bindings are under
`v1_2`; `FrameGotoExt` applies only to Rust Frame handles. Enabling a Cargo
feature does not replace a root server or install native navigation. Embedders
must explicitly choose `v1_2::NavigationCodec` and supply all four services to
the selected server chain. The root chain still advertises 1.1 even in a build
where features are unified; the selected complete chain advertises 1.2.

The schema's `rust_minor_module: true` is development-time Rust placement only.
It scopes generated public-model imports and server imports without changing
wire identity. Library/protocol schemas reject this option. Capability models,
raw wire models and server chains remain generated; typed wire adapters and
client extension dispatch are handwritten. The generated server is now source
under `pwright-server-navigation`, not merely the earlier `.work` preview.

All four selected command/result codecs use borrowed encoding. Headers are
ordered by `(name, value, original index)` without copying/sorting the public
response. Encoding/decoding enforce explicit string, count, aggregate header,
depth and value-node limits; attachments are unsupported. Selected decoding
checks method/envelope kind and duplicated call/target identities before
exposing typed values. `WireProtocol` itself validates version/frame limits,
not this inner identity equality. Existing root typed behavior is not rewritten.
The codec allocates only bounded payload/decoded value storage and starts no
task, browser, event cursor or runtime owner. Future FFI result copies require
their own admission; these codec limits do not cover them.

Verification in `.work/node074-frame-contract/`:

- 109 selected Rust tests pass: generator 62, capability 2, client 7, codec
  36 (22 units, 12 schema/typed cases, two stream integrations), server 2.
- All eight selected typed command/result adapters preserve retained Page
  payload bytes where applicable; new tests exercise Frame identity, timeout,
  cancellation metadata, null/HTTP-404 results, typed errors, canonical headers,
  aggregate limits, forged call/target IDs and forged Page envelopes.
- A Rust Frame extension round-trips through the actual encoded client,
  DirectDispatcher with the browser object catalog, typed server and Tokio
  duplex stream. Its deterministic service is not a browser implementation.
- Strict selected all-target release Clippy, generated-output `--check` against
  frozen 1.1, and the existing 15-package navigation-only dependency proof pass.
  CI now includes selected generation/tests/Clippy; no remote CI run is claimed.
- A separate default-feature run passes 32 tests. Selected and default server
  normal closures contain the same nine Pwright packages. Formatting/diff
  checks pass; retained default/drag addon SHA-256 values are unchanged.

All test/build handles started for this checkpoint completed. No task-owned
browser/server process was left running. Final memory snapshot at 08:36:40 UTC
reported shared backend RSS about 1.61 GiB and zero swap; per-chat attribution
is unavailable. Concurrent XLQY builds/services were identified by cwd and left
untouched.

The production Rust host executor is **not yet connected** to this typed
contract. Context baseURL/header reconciliation, response/error conversion with
explicit foreign-copy budgets, selected Node wrappers, negotiation integration,
redirected-navigation adoption and public pinned Node differential gates remain
open. No browser test or addon binary rebuild was performed for this wire-only
checkpoint; NODE-073's native evidence and attachment reliability issue remain
separate historical results.

## NODE-073 Rust host dispatch, scheduling and cancellation

The separate Rust host `frame-navigation` feature now installs retained native
routes before initial topology publication. `HostSession::navigate_frame_native`
combines the existing Page-locked registration, exact Frame route dispatch,
native receipt, notification/deadline scheduling, and final host publication
fence. The function takes an already-resolved URL and returns the original
bounded `FrameNavigationWait` owner, not a copied JSON value or Node wrapper.
Read `outcome()` for either its completed response or terminal navigation error.
Native initiation/host failures remain outer `HostError` values. This is a Rust
execution port, not the completed generated/public Frame.goto contract.

The host deadline starts before the initial publication fence. The remaining
duration is translated into the Page's injected clock origin; an outer absolute
deadline also covers native receipt and final publication. Zero disables that
deadline. Existing host stop/disconnect signals race the entire operation and
the operation participates in the host TaskTracker, so close waits for its
future/registration/native-call owners to retire. Page observation closure maps
to TargetClosed. Document-level errors remain local to the waiter.

Route currency and navigation completion are different facts. The old
`begin_navigation` primitive retains its pre/post-reply route checks. New
`dispatch_navigation` checks currency before first-poll dispatch, then returns
the exact native receipt even if a renderer handoff retired that route while
the portable Frame survived. The embedding ordered waiter decides whether the
Frame/document completed, detached or was interrupted. Neither primitive retries,
falls back to Page navigation, creates a world or sends Page.stopLoading.

Cancellation drops the native future. Existing CDP PendingRegistration RAII
removes its pending map entry and capacity slot; queued wire bytes remain
charged until the writer consumes/drops them. Cancellation cannot unsend a
command already queued or delivered, and does not cancel browser loading.
The host adds no detached navigation task or new event cursor.

### Bounds and verification

- Route namespace 126/0: 128 current/prepared Frame slots, eight retained route
  snapshots, 256-byte native IDs, using the existing route profile accounting.
- Namespace 126/1: eight admitted executor allowances of 48 KiB, covering two
  owned URL/referrer strings (each capacity, not merely length, <=16 KiB) and
  bounded executor/timer state. Native copies, registration arenas and foreign
  values retain their separate profiles. Embedding task stack/allocator metadata
  are outside the logical bound.
- Namespace 126/2: an admitted shared counter body plus Arc overhead. Its
  explicit eight-operation admission runs before the publication wait. A
  combined resource plan is not a feature-local concurrency limiter; the counter
  enforces this ceiling even with other features' spare budget. An outstanding
  permit retains the counter's lease after Installation drop.
- Existing four registrations per Page and escaped-result arena ceilings remain
  independent limits. Admission failures occur before native dispatch.

The first real-host run passed HTTP 200, hash/networkidle/null, HTTP 404, and
OOPIF child response/URL publication with unchanged parent/sibling. It disposed
Context `B12D304630AD4ACA1AB8471B0884BE56`. That run preceded the final admission
counter and streaming cancellation extension. Evidence directory:
`.work/node073-host-frame-navigation/`.

Final-source verification:

- 361 composition + 232 selected host + 77 CDP units pass (670 total);
  composition has 15 ignored and host has nine ignored native tests. New route
  checks cover pre-poll retirement, receipt across renderer handoff, and dropped
  dispatch lease reclamation without Page.stopLoading. Counter tests deliberately
  use a larger combined resource plan to prove the explicit count boundary.
- Strict selected all-target release Clippy and readiness-only all-target Clippy
  pass. Default and selected addon source checks pass; fmt/diff checks pass.
- The rebuilt pinned `playwright-core@1.58.2` internal navigation oracle matches
  all 29 traces / 131 observations. This remains internal, not public FFI parity.
- `live-diagnostic.log`: the final native suite passes in 5.09 s, including
  dropped-call loading continuation, the 1000-ms timeout and host-close drain.
  All streaming tasks are joined on success. Context
  `F2EC1507AB5FF4641890A20C46E6D178` is disposed and verified absent by the test.
- `live-final.log` and `live-repeat.log` retain two preceding attachment failures
  (`Topology initialization failed: Connection closed`), before any navigation
  case. Their exact Contexts `3FFEBBB072ABC17E50728EFC8283BE27` and
  `1ACBD3F030265C328AAA50AA9A9F2A87` were also disposed. Adding WARN tracing to
  the same test changed timing; the next run passed but did not establish or fix
  the attachment cause. Reliability remains open, not silently reclassified green.
- Default/selected normal dependency graphs contain the same 105 unique Pwright
  packages. Existing default and drag addon artifact hashes remain unchanged.
- Removed only preflight target `5B54EB41920BD6421E3DD941600039BE` and the
  `/pwright-node073-navigation` serve-lib route. Verified no matching target or
  route remains; left all unrelated Chrome targets and five XLQY mounts intact.
  No task-owned streaming server or test process remains. Final memory check has
  zero disk swap; the release compiler's observed peak RSS was about 5.7 GiB and
  that process completed. Shared application memory is not attributed per chat.

The extended live test uses three bounded dynamic chunked HTTP responses on
ephemeral Tailscale-bound ports. It holds each body open to test future drop,
absolute timeout and host close after observed HTTP dispatch. It also checks
the original document remains loading after abandonment/timeout and can finish
after the body is released. Its listener/stream task has abort-on-drop cleanup;
the existing test supervisor bounds the entire run and independently disposes
the exact test Context. These are test application responses, not static-file
servers; static fixtures still use serve-lib.

Still required: generated Frame command/wire adapters, Node argument/result
admission and Promise/error mapping, baseURL/header reconciliation, Response.frame
binding, redirectNavigation adoption, Worker ownership, explicit native
cross-renderer response-handoff coverage and public pinned differential gates.
Do not infer full Playwright compatibility from this Rust host test.

## NODE-072 ordered operation registration and actual Rust host observation

The lifecycle owner now has explicitly selected, fixed-capacity navigation
registration slots. `NavigationRegistrationProfile` separately admits the
operation core and its Arc/Mutex/notification owner before allocation.
`PageLifecycleHandle::register_navigation` validates that the selected portable
Frame is observed by that Page and returns one non-cloneable
`NavigationWaitHandle`. Registration sends no native navigation command.

The waiter is updated under the existing Page lifecycle lock, not from a later
snapshot-polling task. At every owner checkpoint, elapsed timers are advanced
and active waiters see readiness **before** a subsequent commit/request can
erase it. Native notices and their resulting snapshots are delivered together
under that lock. A native receipt is applied under the same lock and then sees
current readiness: load observed and cleared before the receipt is not reused
as though the lifecycle wait had already started. Existing external notice
sinks retain their prior borrowed-notice contract and cannot reenter the owner.

Operation errors, including deadline/capacity/interruption, remain terminal
results of that invocation, not failures of the shared Page driver. A failed
registration does not seal the Page. A true shared stream/normalizer failure
still seals observation. Normal Page close seals/wakes that Page's handles,
without closing other Pages. Sealing never blocks on a callback's Page lock;
the lock holder also wakes consumers when it exits after a concurrent seal.

The ticket supports receipt delivery, local abort, an async changed notification,
the combined Page/operation next deadline, and moving out a settled core owner.
`notify_one` retains an early change permit; `changed(&mut self)` permits
only one outstanding consumer wait. Native changes wake even pending operations:
a commit may introduce an idle deadline earlier than the consumer's current
timer. Plain snapshot/deadline reads do not self-notify into a busy loop.
Notifications do not create a task or timer. The embedding executor must
schedule the returned deadline and call a
checkpoint afterward, including for idle-only progress. The live test uses that
notified/deadline loop, not a test-side event matcher.

Moving a settled result copies neither response nor error bytes. The returned
`FrameNavigationWait::outcome()` borrows them from the original admitted arena.
Its lease remains live after the ticket, Page owner and driver drop. A consumed
ticket rejects further receipt/result operations without poisoning the Page.
Dropped tickets release registration slots on the next Page checkpoint; no
Weak references or uncharged weak-Arc tails are used. Holding a ticket retains
only its Page/terminal owners and registration, not the host or native transport.

Host selection is deliberately separate:

- Default: no lifecycle registry or navigation registration selection.
- `frame-lifecycle`: prior readiness-only selection, without registration slots
  or response-storage budgets.
- `frame-navigation`: includes lifecycle plus four registration slots per Page.
  The 32 active-or-escaped Page owners imply 128 budgeted registration owners.
  Each operation permits 16 journal events, eight document-request chains,
  256-byte IDs, 1024-byte error text and 64 KiB serialized response per chain.
  These are declared ceilings; response arenas allocate on registration, not
  eagerly for every possible operation. Escaped outcomes keep their charges
  and can exhaust the shared ceiling even after a Page slot becomes reusable.

The Rust-only `HostSession::register_frame_navigation` performs the existing
publication fence, validates Page/Frame observation and admits registration.
It uses the Page's injected monotonic origin; timeout zero/None disables its
registration deadline. This is not a full command deadline spanning earlier
preparation. A future Frame.goto executor must establish its deadline before
the publication/route/dispatch phases and preserve native-call retirement.
No public Node API, generated protocol command or addon feature was added.

Verification under `.work/node072-navigation-registration/`:

- `unit-composition-final.log`: 358 pass / 15 ignored, including ten new production
  driver tests for transient load, native response retention, pre-receipt
  lifecycle ordering, idle-before-commit, isolated timeout, admission/drop
  reclamation, early wake permits, rescheduling newly introduced idle deadlines
  without busy polling, Page-close wake and moved-result leases.
- `unit-host.log`: 226 pass / seven ignored with navigation selected.
- `unit-navigation.log`: 67 pass. The pinned offline oracle remains 29 traces /
  131 matching observations (`oracle.json`).
- `clippy-final.log`: strict release all-targets host/composition Clippy
  passes with navigation and drag-runtime. `readiness-clippy.log` separately
  passes for readiness-only selection.
- `live-final.log`: actual selected Rust host receives an HTTP 200 response
  from its production CDP cursor, returns the original Frame, then waits for
  hash/subtree idle and returns null without changing document generation.
  Native initiation is deliberately supplied by an independent test session
  through the real Rust Frame-begin service; host-owned dispatch is still open.
  The existing multiple-Page, explicitly verified sandbox OOPIF, dynamic Page
  discovery, close and escaped-lease checks also pass. A pending registration
  rejects after host close; final handle drops return host leases to zero.
- `default-addon.log` and `selected-addon.log`: source checks pass for default
  addon and drag-runtime plus host navigation selection. Retained addon hashes
  remain unchanged.
  Default and navigation-selected normal closures contain the same 105 unique
  Pwright packages; the already-optional composition-to-navigation edge is
  enabled in the selected graph.

Both live Contexts `61AC5A04A41C1EEC3F53B42E5D721319` and final-source
`24FC5426F13DDE0AE4B5D5F3FC1E975D` were disposed and their absence verified by
the tests. Exact preflight targets `A2947D14A914C826122ED51967F5F4AC` and
`3DD163214720C6D0D3AFBF584D1183D8` were closed; the target-list query returned
zero matching preflight/fixture targets. Route `/pwright-node072-navigation`
was deregistered from port 9384 on the Tailscale bind; unrelated mounts stayed
registered. No task-owned compiler, test or persistent service remained.
The first final deregistration encountered a transient daemon-unavailable
response; a fresh list confirmed the route still existed, and the same exact
bind/port/route retry succeeded. No shared daemon or unrelated mount was stopped.

This is real host observation/registration evidence, not public Node Frame.goto
parity or a complete native executor. Next integrate the retained Frame route,
registration, bounded native receipt lifetime and deadline/close cancellation
as one host operation; then complete redirected-navigation adoption, generated
Frame commands, N-API output ownership, final publication fencing and public
Node differential gates including selected child/OOPIF/Worker behavior.

## NODE-071 bounded navigation matching and response retention

The selected navigation leaf now exports `frame_lifecycle::navigation` with
`FrameNavigationWait`, its explicit profile, borrowed `NavigationPoll` result,
and terminal error reasons. This is the per-invocation Rust state machine, not
yet an asynchronous host operation or public Node `Frame.goto` implementation.
It consumes the existing native lifecycle notices; it owns no native client,
event subscription, thread, task, timer, execution world or browser process.

Pinned `playwright-core@1.58.2/lib/server/frames.js:gotoImpl` establishes:

- Collect navigation events before dispatch; after the receipt, select the
  earliest successful new-document commit or matching-document failure. An
  earlier successful foreign commit interrupts even if the expected commit
  follows. An unrelated failed document does not interrupt this invocation.
- Same-document receipts require a same-document event and still wait for the
  requested existing Frame lifecycle. New-document events cannot substitute.
- Once the navigation event matches, wait on the Frame's **current** lifecycle;
  a later document does not retroactively replace the selected event/response
  or automatically turn that already-matched operation into an interruption.
- After lifecycle readiness, obtain the final redirect request's response.
  Response availability is not body/stream completion; HTTP 404/500 is response
  data. Failure after commit does not synthesize a pending-navigation abort.

The implementation keeps a bounded pre-reply event journal, document-request
slots and one preallocated byte arena. Redirect legs reuse their request-chain
slot, discard the earlier response and preserve final-request association.
Completed requests remain retained for the invocation, independently of the
native normalizer's inflight request removal. Response JSON is serialized into
the fixed arena, not cloned into an unbounded `Value`. Completion borrows those
bytes and the original portable Frame ID; decoding/escaping a result requires
separate caller admission. No response bodies are fetched.

The profile checks arithmetic and reserves `BackendStateBytes` before allocating
the event/request vectors and arena. It charges their logical storage plus the
owner body, with the lease declared last. It does not charge allocator metadata,
an enclosing Arc/Mutex, native envelopes, external snapshots or foreign output
copies. Event/request/ID/error/response limits fail explicitly; they do not grow,
drop history or truncate a successful response. Errors are terminal. Dropping
the owner releases its lease; early cancellation does not release storage while
the owner or a borrowed response is still live.

The executor must install this owner before dispatch, deliver ordered notices,
deliver the native receipt exactly once, and poll the selected Frame snapshot
after each relevant Page transition and timer checkpoint **before later events
can erase a transient lifecycle observation**. Snapshot identity, epoch/session
validation and subtree readiness belong to the existing topology/lifecycle
owner. The current readiness-only host sink has not yet been replaced with that
serialized operation integration. External detach/close/disconnect/event-loss,
download and cancellation reasons seal an active wait without `Page.stopLoading`.
The injected absolute deadline is checked before processing an input at its
deadline; this alone is not native pending-call cancellation/retirement proof.

Verification (local logs under `.work/node071-navigation-wait/`):

- `unit.log`: 67 navigation-leaf unit tests pass, including 22 new operation
  tests for ordering, lifecycle/response phases, redirect reuse, post-commit
  failures, terminal reasons, byte/slot exhaustion and lease reclamation.
- `clippy.log`: release all-targets strict Clippy passes without suppressions.
- `oracle.json`: 29 traces / 131 step observations match the installed pinned
  **actual `Frame.gotoImpl` and `Request` response/redirect promises**. The
  adapter controls native receipt and FrameManager-style notices and drains
  promise reactions at event-loop checkpoints. It is an offline internal oracle,
  not a browser, network normalizer, production driver or public FFI gate.
- `default-addon.log` and `selected-addon.log`: default addon and combined
  drag-runtime/lifecycle Rust source checks pass. Both retained addon hashes
  remain unchanged; no addon build or Node API was added.

Reproduce from the repository root after installing the existing test dependencies:

```sh
CARGO_INCREMENTAL=0 cargo test --locked --release -p pwright-backend-cdp-navigation --features frame-lifecycle --lib
CARGO_INCREMENTAL=0 cargo build --locked --release -p pwright-backend-cdp-navigation --features frame-lifecycle --example frame_navigation_trace
node tests/src/frame-navigation-wait-oracle.mjs target/release/examples/frame_navigation_trace
```

The bridge caps input at 256 KiB, 64 cases and 128 steps/case, output at 1 MiB,
and checks owner lease release per case. The Node adapter pins the installed
package version, bounds child output and applies a 30-second child timeout.
This turn started no browser, preview service or other persistent process.

Still required for the actual objective: wire an admitted operation registry
and wake/scheduling path into the production host without callback reentry;
retain/revalidate native Frame routes and retire pending native calls; implement
Playwright `redirectNavigation` adoption (distinct from HTTP redirect chains),
native initiation error formatting and lifecycle/terminal race semantics; add
the generated distinct Frame command, N-API response ownership and actual
`Response.frame()` identity; verify public Node behavior against pinned
Playwright with real Chrome, OOPIF/Worker, cancellation and shutdown gates.

## NODE-070 bounded multi-Page registry and opt-in Rust host selection

`FrameLifecycleRegistry` composes Page owners on the existing driver cursor.
Its profile explicitly bounds active Pages, all retained owner slots, and each
Page's Frames/requests/native-ID bytes. A dedicated namespace supplies one index
scope and three disjoint scopes per slot. Checked size/scope arithmetic and the
complete resource plan are validated before index allocation; the fixed slot
vector cannot grow.

The observer port now has `prepare_page(topology, page)`, called before enabling
Network/lifecycle domains. Newly discovered portable Page IDs may be prepared
before their topology publication. Capacity failure is fail-closed; this hook
does **not** claim admission before native Target creation, which belongs to the
Page-creation/operation layer. Current topology reconciles retired owners before
new admission. One Page's close does not seal another Page.

Escaped `PageLifecycleHandle` values hide the owner Arc and retain no registry,
other Page or browser transport. They keep their Page storage and a small shared
terminal body alive. The terminal body conservatively retains the index allowance
until the final handle drops, even if the index itself has already been freed.
Retired slots with no escaped handles are reclaimed at the next registry
checkpoint; slots cannot be reused while a handle retains an old owner. A shared
atomic terminal flag rejects reads immediately even if shutdown cannot acquire
the callback lock. No new queue, cursor, task, timer or JavaScript callback is
introduced by this registry.

The Rust `pwright-node-host/frame-lifecycle` feature is default-off. When selected,
the existing host profile admits 16 active Pages and 32 total owners, each with
64 Frames, 256 request slots and 256-byte native IDs. An admitted shared support
allocation provides a monotonic clock and an explicitly readiness-only sink.
The existing driver owns the registry; the live session retains it for Rust
`HostSession::frame_lifecycle(page)` access after the normal publication fence.
The support sink intentionally retains no response/error envelope. Installing
the operation-specific admitted response sink remains necessary for Frame.goto.

Verification in `.work/node070-host-lifecycle/`:

- `unit-composition.log`: 348 pass, 15 unrelated ignored. Eight new registry
  tests cover independent Page readiness/close, active and retained limits,
  pre-domain rejection, slot reclamation, epoch failure, escaped leases and
  shutdown while a notice callback holds the registry lock. Retained reads
  reject immediately through the shared terminal flag.
- `unit-host.log`: 226 pass, seven unrelated ignored. Initial test-only missing
  import and unused-fixture warnings were corrected; no Clippy suppression added.
- `clippy-final.log`: strict release all-targets passes with host lifecycle and
  drag-runtime selected together. Formatting and diff checks are also run.
- `live-host-final.log`: the selected actual Rust host passes against Chrome
  152, using independent imported Pages, real sandbox iframe renderer creation,
  subsequent Page navigation, Page retirement, dynamic third-Page admission and
  escaped shutdown. The test verifies one topology driver, zero live objects
  after close, retained leases while handles live and zero leases after their
  final drop. This is not a public Node Frame.goto differential.
- Default and selected normal host closures have the same 105 Pwright packages.
  Neither retained addon was rebuilt (hashes remain `8bb3c7ff...`/`1b627f19...`).
- Both native runs dispose their exact test Context and verify absence. The
  preflight target `6D5A1F4B0CE0EAF68580729123232968` was closed with absence
  checked, and only `/pwright-node070-host-lifecycle` was deregistered. Shared
  Chrome, XLQY targets/routes/servers were not stopped. Swap remains zero.

No addon or public Node API was changed.
Worker ownership, Frame command/response/waiter/cancellation semantics and public
FFI discrepancy gates remain open; readiness integration does not complete them.

## NODE-069 opt-in production ordered lifecycle observation

The production topology driver now accepts a synchronous
`ChromiumFrameEventObserver` before initialization. The new, default-off
composition `frame-lifecycle` feature supplies `PageLifecycleObservation`, an
explicit one-Page/one-epoch owner over the native normalizer and reducer. This
is still a building block for Frame.goto, not its public implementation.

- The existing ordered cursor applies topology first, then passes the borrowed
  envelope and authoritative routing to the observer. There is no additional
  subscriber, queue or spawned task. Default construction selects no observer
  and enables no additional native domains.
- Selected renderer sessions enable Network and Page lifecycle events before
  completing initialization/resuming paused targets. Initial Frame trees seed
  loader IDs parent-first. Repeated same-loader snapshots are idempotent. A
  changed loader in an authoritative current-renderer snapshot is a document
  commit: an already-navigated replacement may not emit a later native commit
  event. Stale-renderer snapshots are rejected by current topology routing.
  Snapshot commits also deliver a borrowed Committed notice to the sink.
- Reconciliation removes retired Frames/requests and attaches missing Frames
  using bounded repeated scans rather than a second owned Frame map. Routing
  requires the selected Page and current renderer session; document handoff
  uses the existing topology proof, not request-ID coincidence.
- The caller injects a monotonic clock and synchronous borrowed-notice sink.
  Callbacks run under topology/owner locks: no reentry or JavaScript callback.
  Any response/failure data retained by the sink requires its own admission.
- The owner body/Mutex/Arc allowance is pre-admitted separately from fixed
  native rows, IDs and reducer state. Its lease is last to drop. Clock/sink
  pointees, outer multi-Page registries, envelopes and native transport are
  separate resources, not covered by that allowance.
- Driver error, cancelled in-progress driver future, event loss, resync and
  driver drop seal readiness. Normal Page retirement seals just that Page
  owner. Retained queries reject a sealed owner; storage remains admitted until
  the last owner reference is dropped. Use the driver's idle-only signal for
  timer wakeups instead of cancelling an in-progress receive/transaction.

Initialization accounts for observed Network traffic; it cannot reconstruct
requests preceding attachment. Worker ownership is not yet supplied by the
production topology route port. Multi-Page installation, response/waiter state,
native pending-reply cancellation, Node host selection, generated Frame commands
and public Frame.goto differential verification remain open.

Verification records are under `.work/node069-ordered-lifecycle-owner/`:

- `live-reconciled.log`: two real Chrome 152 cases pass through the production
  driver. Same-origin HTTP navigation observes response delivery, a new document
  and load/idle. Cross-origin data navigation replaces the renderer, imports
  its changed snapshot loader, reaches load/idle and preserves the portable
  Frame while retiring the old retained renderer route. Both dispose and verify
  absence of their exact owned Contexts. This is not cross-renderer HTTP response
  handoff coverage: the cross-origin destination is a data URL.
- Earlier `live.log` accepted old readiness and failed the generation assertion;
  `live-document-matched.log` then timed out waiting for a new generation.
  `live-trace.log` identified the changed replacement snapshot with no native
  commit event. These failures are retained, not counted as passing cases.
- Final post-reconciliation unit runs pass 340 composition tests (15 unrelated
  ignored) and 45 navigation tests. The driver suite includes explicit stale
  snapshot rejection and paused Page initialization order. A test-only temporary
  loader borrow compile error was corrected before the final navigation run.
  `comparison.json`/`comparison.log` pass all 93 normalized lifecycle snapshots
  across 11 traces against pinned Playwright 1.58.2; this internal differential
  is not a public Frame.goto oracle. Final strict Clippy, formatting and diff
  checks pass (`clippy-final.log`).
- Node host/addon default and drag-runtime source checks pass. Neither addon
  was rebuilt; retained hashes remain `8bb3c7ff...` and `1b627f19...`.
- Default and selected normal closures contain the same 90 Pwright packages;
  the selected feature activates the existing navigation implementation through
  the new optional direct edge. Navigation-only proof passes at 15 packages.
- Every test-owned Context was disposed, the preflight target was closed and
  absence verified, and only `/pwright-node069-ordered-lifecycle` was deregistered.
  Shared Chrome, three XLQY routes and the XLQY servers remain untouched.
  At the 06:23 UTC handoff no Pwright compiler/test process remains; swap is zero.

## NODE-068 production document-commit topology retirement

The production topology event driver now calls
`CdpTopology::frame_document_committed` for current-session native new-document
events, before observing the new URL/name and reconciling published ownership.
It retires the selected Frame's old descendants, their worlds/renderer routes
and pending descendants, but preserves the selected Frame itself. The unchanged
main-native-ID case now also retires children; previously only main-ID rebinding
did so. Child/OOPIF commits previously only updated navigation metadata.

The primitive requires the exact current Frame/session route and connection
epoch. Unknown, stale-session, wrong-renderer and crashed routes cannot retire
another Frame's children. Hash navigation does not call this primitive. Main
Frame rebinding and descendant retirement share the existing production
topology/runtime/service-route transaction. A rejected runtime disposal batch
must leave old topology and retained navigation snapshots valid. There is no
new document-history table, native-ID copy, dependency or feature selection:
the operation reuses bounded topology removal and shadow-transaction machinery.
It consumes ordered new-document commits; it is not a loader deduplicator or
document waiter.

This follows the installed Playwright 1.58.2
`FrameManager.frameCommittedNewDocumentNavigation`, which removes children before
updating URL/name and clearing lifecycle. It resolves a prerequisite disagreement
between topology and NODE-067's reducer. It does **not** yet install that reducer
into the production driver or Node host.

`live_frame_document` now exercises the real production composition, runtime,
event driver and NODE-066 retained navigation access. Two live Chrome 152 cases
pass in `.work/node068-frame-document-topology/live-final.log`:

1. Same-origin HTTP child navigation retires the nested Frame/runtime object and
   its retained route, while preserving the selected Frame, parent, sibling and
   same-session selected route.
2. HTTP-to-data cross-origin navigation preserves the selected portable Frame but
   changes its actual renderer session and permanently retires its old snapshot.
   Parent and sibling runtime objects/routes survive. The test waits for the
   new renderer route instead of interpreting old-route retirement as Frame
   destruction.

An initial data-URL fixture incorrectly asserted that the selected route must
survive every document navigation. Its assertion failure is retained in
`live-selected.log`; the new cross-origin case directly verifies the session
change and distinguishes this expected handoff from a lost Frame. The first
test setup also omitted capability selection and was rejected before creating
a Context (`live.log`). All later owned Contexts were explicitly disposed even
on assertion panic. The preflight tab and this run's served fixture route have
been removed; shared Chrome, serve-lib and unrelated XLQY resources remain intact.

Unit coverage adds exact-session local/remote retirement, old-epoch exclusion,
production child/main document publication, unchanged hash/stale-session state,
and rejected runtime-batch route preservation. Final-source verification:

- `unit-final.log`: 49 topology and 329 composition tests pass (378 total);
  fifteen separately ignored native unit tests remain unexecuted.
- `clippy-final.log`: strict release all-target Clippy passes for both crates.
- `node-default-check.log` / `node-selected-check.log`: Rust Node host and addon
  source checks pass in default and drag-runtime configurations. Neither addon
  binary was rebuilt; both retained artifact hashes remain unchanged.
- `dependencies.log`: the topology-only proof still has exactly five Pwright
  packages. Formatting and diff-whitespace checks pass.

Reproduce both native tests after dedicated Chrome/Pwright preflight and a
verified serve-lib registration of `tests/fixtures/`:

```sh
PWRIGHT_CDP_WS_URL="$browser_ws" \
PWRIGHT_FRAME_DOCUMENT_FIXTURE="$fixture_base" \
CARGO_INCREMENTAL=0 cargo test --locked --release \
  -p pwright-composition-chromium --test live_frame_document \
  -- --ignored --nocapture --test-threads=1
```

Release test compilation temporarily used about 7.8 GiB RSS; it completed and
no task-owned compiler/server/browser process remains. At 05:42:15 UTC, the
shared backend was about 733 MiB RSS and swap remained zero. These are process
measurements, not exact per-chat usage or new lifecycle memory-budget claims.

Remaining work still includes installing and seeding bounded lifecycle observation
before enabling native events, event-loss sealing, document/request/response
handoff, timeout/close/disconnect handling, Frame command generation, Rust
host/N-API completion and paired public `Frame.goto` verification. A renderer
snapshot is an initiation route, not the lifetime of the portable Frame or its
event-driven navigation operation.

## NODE-067 bounded native lifecycle normalization

The default-off navigation `frame-lifecycle` feature now exports
`CdpFrameLifecycle`, composing the production reducer with bounded CDP identity
normalization. It adds no dependency, transport, task, JavaScript algorithm or
public Node method. `CdpLifecycleRoutes` is an injected borrowed routing port,
not a second production topology owner. The embedding owner must publish Frame
attachments/removals in event order and prove session ownership and connection
epoch before events can affect a Frame. Worker request ownership can be supplied
explicitly by this port; no missing-Frame fallback targets the main Frame.

Ordinary requests are identified by session plus request ID. Redirect legs reuse
the native request ID but receive distinct normalized request identities.
Document requests additionally require CDP type Document and requestId=loaderId.
Cross-session document completion requires explicit current-Frame handoff proof;
ambiguous matches fail. A new document commit retains only its selected document
request, removes descendant state and resets lifecycle. Load and DOMContentLoaded
require the current loader; hash navigation preserves the existing document.
Data URLs are ignored for network accounting. Favicon exclusion survives redirects;
EventSource is not silently excluded from inflight accounting.

These choices are based on the locally installed Playwright 1.58.2 Chromium
adapter/network manager and the previously characterized FrameManager reducer.
The adapter returns borrowed commit, response and failure notices, including
cancellation and redirect identity, but does not yet consume those notices in a
full navigation operation. Response metadata and bodies are not copied or
retained here; an escaping response needs its own admitted owner.

The profile requires distinct native/reducer resource scopes, Frame/request
capacities and a maximum UTF-8 native-ID byte length. Native storage consists of
fixed optional rows plus an arena of `(frames + 2 * requests) * id_bytes`:
one current loader per Frame and one request/session pair per request. Checked
arithmetic additionally charges actual row types and the owner body. It
conservatively counts the inline reducer body again. Both allowances are claimed
before allocation; capacities are checked; transition storage does not grow.
Drop releases allocations before the native lease. Capacity, malformed identities,
clock reversal or observer loss must not turn into successful readiness:
normalization failures seal the reducer. Pruning propagates errors explicitly.

Verification under `.work/node067-native-frame-lifecycle/`:

- `unit-final.log`: 41 navigation unit tests pass, including ten native adapter
  cases covering redirects, loader/epoch filtering, document-session handoff,
  worker/EventSource accounting, duplicate per-session request IDs, favicon
  redirects, descendant pruning, borrowed response/error notices and resource
  admission/release.
- `clippy-final.log`: strict release Clippy passes for all targets with the
  feature selected.
- `live-final.log`: a real Chrome 152 test passes through production CDP
  transport, native initiation, the native normalizer and the production reducer.
  It observes HTTP response/finish and load/DOMContentLoaded/idle for the root,
  two children and a nested Frame, then selected-child document navigation and
  a hash change. It verifies descendant removal, unchanged parent/sibling
  snapshots, hash document-generation preservation and zero remaining observer
  leases/bytes. This harness supplies a bounded single-renderer routing port;
  it does not prove production topology-driver integration or public FFI parity.
- Initial fixture registration used an invalid timeout format and produced an
  HTTP 404 assertion failure (`live.log`). The test now catches assertion panics
  with a joined task, aborts timed-out work, and disposes its exact Context before
  propagating failure. The original Context was confirmed absent after transport
  disconnection; Context creation uses dispose-on-detach. Recovery never targets
  another Context without exact fixture/target validation.
- `expected-failure-cleanup.log`: an intentional missing-route HTTP 404 raises
  an assertion panic; the joined test verifies exact Context disposal before
  returning the expected failed-test status. This is cleanup evidence, not a
  passing navigation case.
- `default-unit.log`: 21 default-feature units pass; `comparison.json` /
  `comparison.log`: the production reducer still matches Playwright 1.58.2 on
  93/93 snapshots across 11 traces. `dependencies.log` passes the exact
  15-Pwright-package navigation closure. Both existing addon hashes are unchanged.

Reproduce the native gate after the required dedicated Chrome/Pwright preflight
and verified serve-lib fixture registration:

```sh
# Supply the current loopback browser WebSocket URL and verified fixture base URL.
PWRIGHT_CDP_WS_URL="$browser_ws" \
PWRIGHT_FRAME_LIFECYCLE_FIXTURE="$fixture_base" \
CARGO_INCREMENTAL=0 cargo test --locked --release \
  -p pwright-backend-cdp-navigation --features frame-lifecycle \
  --test live_frame_lifecycle -- --ignored --nocapture
```

This run's preflight target and `/pwright-node067-native-frame-lifecycle` route
were removed and absence verified; all test Contexts were disposed or confirmed
absent. No new background process is left running. Existing shared Chrome,
serve-lib and unrelated XLQY servers/routes were not stopped. At 05:26:41 UTC,
the shared backend RSS was about 730 MiB and swap remained zero; this is not
a per-chat memory measurement or a bound on browser/runtime memory.

Remaining gates: production event-driver selection and initialization fence,
coherent topology/descendant removal during renderer swaps, real OOPIF/worker
handoff verification, pending/current-document response and redirect matching,
timeout/close/disconnect cancellation, additive Frame command generation,
Rust host/N-API admission and publication fences, and paired public
`Frame.goto` discrepancy tests. Installation must precede event enabling; this
adapter cannot reconstruct pre-observation or lost events. Receiving a current
snapshot alone does not identify the document requested by a navigation.

## NODE-066 retained routes in the production topology transaction

`ChromiumFrameNavigationProfile` / `ChromiumFrameNavigationAccess` now install
an optional Frame navigation table inside `CdpPageServices`. Installation requires
explicit navigation selection and its additional declared resource requirements.
It allocates no table entries until the next successful topology synchronization.
No evaluation world, world-readiness service, public network subscription, Node
driver, background task or additional Cargo dependency is selected.

The table uses NODE-064's borrowed topology resolver, preserving the exact
connection epoch, portable Frame/Page, native Frame, target and renderer session.
Local Frames reuse the existing Page client; remote Frames and their local
descendants share a renderer client within the selected table. The aggregate
backend delegates native initiation to the existing isolated navigation adapter,
with the existing shared operation budget; it does not duplicate the algorithm.

`snapshot(frame)` admits a non-cloneable retained route before cloning ownership.
It exports borrowed IDs and `begin_navigation`, but no escapable native-client or
service Arc. Initiation checks observed retirement before first-poll dispatch and
again after the native reply, without retrying or calling `Page.stopLoading`.
This remains initiation, not document-load completion or native cancellation.
A pending native reply is not itself woken by route retirement; the final
operation still needs the host's timeout/close/event-delivery control.

Publication participates in the existing prepared topology/runtime/route
transaction. Only the infallible commit retires replaced route entries. Discarded
candidate tables do not mutate live entries. Unrelated world publications reuse
the route, while detach, crash, reconnect and renderer-swap gaps retire affected
snapshots permanently, even if the same native IDs later return. Last service-table
drop retires outstanding snapshots. Explicit `seal()` is a terminal event-loss /
shutdown fence; the embedding owner must invoke it, and later publications cannot
reopen the installation. The Node event driver does not install this access yet.

Storage is admitted before table/client/ID allocation. The owner allowance covers
two simultaneous bounded route tables and their bodies; each retained snapshot
claims an additional old-route/body allowance until drop, including after its
route leaves the current table. Capacity is shared across accessor clones.
The checked formula derives from the actual owner/entry/service/snapshot types,
five bounded native-ID copies per entry and explicit fixed overhead. Vec and
String capacities are checked against their admitted sizes. Accessor clones share
one additionally admitted service-owner Arc. Table fields drop route allocations
before their final allowance owner. The route allowance
does not bound arbitrary injected SessionFactory internals, shared transport,
caller-owned request/receipt values, event queues, native sessions or process RAM.
It is not a native destruction receipt or proof that Chrome has freed a resource.

The production-composition tests exercise exact local/remote/nested initiation,
no Page fallback, duplicate installation, overflow, retained admission and release,
swap-away/back without an intervening snapshot read, unrelated world updates,
crash/reconnect, teardown and terminal sealing. They also cover rejected route
preparation and a rejected runtime lifecycle batch after route preparation; neither
may publish retirement. A gated fake native reply tests retirement while dispatch
is already in flight. These are production Rust state/transaction tests, not new
real-browser or public Node `Frame.goto` conformance evidence.

Final-source verification under `.work/node066-frame-navigation-routes/`:

- `unit-verified.log`: aggregate backend 18, navigation leaf 21, composition 326
  and fake client 23 unit tests pass (388 total). The composition count includes
  all eleven new route tests; its fifteen ignored tests remain unexecuted native
  lanes, not additional passes.
- `clippy-verified.log`: strict all-target Clippy passes for composition,
  aggregate backend and fake client. `fmt-final.log` is clean, as is diff checking.
- `node-check.log` / `node-selected-check.log`: default and drag-runtime Rust
  host/addon source configurations compile. These do not rebuild an addon.
- `node-unit.log`: 46 Node tests pass; `types.log`: strict TypeScript passes.
  `node-boundary-retry.log`: host closures stay 234/235/235, addon closures stay
  249/250, with no runtime JS driver/CDP/process-launcher imports. The earlier
  `node-boundary.log` records an incorrect script name, corrected to test:boundary.
- Navigation-only and topology-only dependency scripts pass at 15 and five
  Pwright packages (`navigation-dependencies.log`, `topology-dependencies.log`).

The wider backend regression exposed a stale command-count assertion:
`unit-drop-order.log` records expected 41 versus the existing 42 declarations.
The manifest already included reload and sequential typing. The corrected test
expects 42 and explicitly checks reload; no new Frame command is advertised.
`unit-manifest-fixed.log` preserves an intermediate missing test-module qualifier,
fixed before the final successful run. Earlier results are retained as history.

No addon was rebuilt. Verified SHA-256 remains `8bb3c7ff...` for the default
artifact and `1b627f19...` for drag-runtime. No new commit/push or browser/server
launch was performed. All checkpoint build/check processes completed; other
tasks' XLQY servers were left untouched. The foundation remains `f76e3f2`.
Next connect this retained owner to bounded native lifecycle normalization,
loader/response matching and controlled cancellation, then expose the generated
Frame command through FFI and verify the public API against pinned Playwright.

## NODE-065 optional normalized lifecycle reducer

`pwright-backend-cdp-navigation` now offers the default-off `frame-lifecycle`
feature. Its `FrameLifecycleStore` keeps current-document lifecycle and the
selected Frame subtree's idle state in Rust, independently of transport, Node,
execution worlds or public network-event subscription. It is not yet selected
by the Node host and does not itself execute or complete `Frame.goto`.

The reducer consumes explicit Attach/Commit/DocumentEvent/RequestStarted/
RequestFinished/Detach inputs. `DocumentGeneration` is a caller-normalized,
increasing document identity; it is not a native loader string. Request ObjectIds
must be unique for the observer's lifetime. A Rust adapter must resolve exact
connection/session/frame/loader identities before supplying these inputs. This
does not require installing a public Request event stream, but normalization,
identity admission and any pending-event storage are still unimplemented owners.

Current behavior and evidence:

- Persistent snapshots distinguish commit, DOMContentLoaded, load and network-idle;
  old-generation events cannot set a replacement document's load bit.
- Idle state includes descendants and excludes unrelated siblings, regardless of
  how a future adapter maps their renderer sessions. There is no CDP-session-local
  request heuristic in the reducer. Favicon requests are excluded; request IDs
  are deduplicated. EventSource/stream completion policy belongs to normalization,
  not an implicit exemption from this request accounting.
- A document commit removes the old child subtree and keeps only its explicit
  current-document request. Finished requests from retired documents are ignored.
  Hash navigation does not clear an existing lifecycle snapshot.
- All time is supplied by the embedding Rust owner as monotonic Duration values.
  Elapsed timers are drained before applying an event. The owner schedules the
  next deadline; the reducer owns no timer task or clock. Backward time seals it.
- Fixed Frame/request slots and the store body are reserved before allocation.
  The checked charge is `size_of(Store) + frames * size_of(Option<FrameState>) +
  requests * size_of(Option<RequestState>)`. Slot capacities are checked against
  their admitted counts. Transitions and copy-out snapshots allocate nothing;
  there are no retained strings, callback roots or dynamically growing indexes.
  The lease lasts until store drop. Allocation metadata, external queues,
  normalization and whole-process RAM remain outside this logical-storage proof.
- Capacity/identity errors and explicit event-loss invalidation seal the observer:
  snapshot/deadline queries then fail, preventing stale readiness from becoming a
  false navigation success. This is not a recovery/replay mechanism.

The oracle executes the installed Playwright 1.58.2 `FrameManager` with a small
fake Page boundary and deterministic timer port. It observes the manager's
lifecycle events instead of replacing its algorithm. It agrees with the Rust
reducer on **93/93 snapshots across 11 traces**, including request activity after
network-idle publication and lifecycle-triggered recalculation. This is internal
normalized-state differential evidence, not public API or browser conformance.
The pinned source was also checked against the
[upstream v1.58.2 Frame implementation](https://github.com/microsoft/playwright/blob/v1.58.2/packages/playwright-core/src/server/frames.ts).

Reproduce from the repository root, with an unused absolute report path:

```sh
CARGO_INCREMENTAL=0 cargo build --locked --release \
  -p pwright-backend-cdp-navigation --features frame-lifecycle \
  --example frame_lifecycle_trace
PWRIGHT_FRAME_LIFECYCLE_BINARY=<absolute-path-to-target/release/examples/frame_lifecycle_trace> \
PWRIGHT_FRAME_LIFECYCLE_REPORT=<unused-absolute-report-path> \
node packages/pwright/scripts/compare-frame-lifecycle.mjs
```

The Node package also exposes `pnpm test:frame-lifecycle` with the same two
environment variables. It runs a prebuilt binary with a five-second deadline,
512 KiB input and 1 MiB output limits. The bridge caps traces, steps, Frame slots
and request slots and verifies lease release after each trace. The fake timer
port is scoped to a synchronous oracle run and restored in `finally`; this
script is development-only and starts no browser or server.

Reports under `.work/node065-frame-lifecycle/`:

- `comparison.json` / `comparison-retry.log`: 93/93, no mismatches.
- `comparison-cleanup.json` / `comparison-cleanup.log`: final-source 93/93 after
  bounded parent-first subtree cleanup replaced repeated ancestry walking.
- `unit-cleanup.log`, `clippy-cleanup.log`: final-source 31 tests / strict checks.
- `node-units.log`, `types.log`, `node-boundary.log`: 46 Node unit tests, strict
  TypeScript and unchanged 234/235/235 host / 249/250 addon package boundaries.
- `unit-final.log`: 31 navigation tests, including ten new lifecycle tests.
- `clippy-final.log`, `fmt.log`: strict all-target checks with the feature selected.
- `dependencies-default.txt` / `dependencies-selected.txt`: identical normal
  dependency sets; `navigation-dependencies.log`: existing 15-package proof passes.
- The first `comparison.log` records a missing `isClosed` method on the fake Page;
  it failed before producing comparison evidence. The corrected harness is not a
  production behavior change.

NODE-066 supplies the admitted retained route owner. Next integrate a loss-aware
native event normalizer with this reducer, then document/response correlation, the distinct
portable Frame command and public FFI. Prove native timing, session swaps,
same-document waits and cancellation against a real browser before enabling it.
The Node artifacts and the broad compatibility goal remain unchanged/incomplete.

## NODE-064 native initiation checkpoint

The full target is public `Frame.goto(url, options): Promise<Response | null>`
with Playwright behavior and Rust-owned routing, navigation state, waits and
cancellation. It is **not implemented yet**. This checkpoint implements and
verifies two prerequisites; it does not advertise a new portable command or
public Node method:

- `CdpTopology::frame_navigation_route` borrows the exact Frame, owning Page,
  renderer target/session and connection epoch without requiring a world.
- `CdpNavigationService::begin_frame_navigation` initiates navigation on an
  injected session with an explicit native Frame ID. Its receipt does not mean
  load completion. It does not subscribe, retry, create a world or stop a Page.

Pinned Playwright 1.58.2 and actual Rust initiation agree on six settled-state
observations: local child and nested child, plus a verified sandbox OOPIF, each
with new-document then hash navigation. Parent/root/sibling isolation, retained
Frame identity, URL, referrer and receipt kind agree. Invalid native Frame IDs
reject without navigating the Page. This is **native initiation evidence**,
not a public FFI comparison or lifecycle/timeout parity proof.

## Authoritative behavior and existing gaps

The oracle is the installed `tests/node_modules/.pnpm/playwright-core@1.58.2/`
package, not an unpinned upstream checkout. Relevant source locations inside
`node_modules/playwright-core/lib/server/` are:

- `frames.js`: `raceNavigationAction`, `goto`, `gotoImpl` and
  `_recalculateNetworkIdle`.
- `chromium/crPage.js`: `navigateFrame`, `_sessionForFrame` and `_navigate`.

The Chromium delegate selects the Frame's renderer session and sends
`Page.navigate` with `frameId`, `url`, optional `referrer` and
`referrerPolicy: unsafeUrl`. The server collects navigation events before native
dispatch, matches the new document (or same-document event), then waits for the
requested lifecycle and returns the final redirect request's response or null.
Its lifecycle wait also applies to hash navigation. The operation races the
Frame's detach scope and Page's open scope. Network-idle combines the selected
Frame and descendants, not unrelated siblings.

Current Pwright Page navigation cannot simply be called with a Frame ID:

1. Portable `Goto` targets `pwright.browser.page`. Its CDP request has no
   explicit `frameId`; its cancellation calls Page-wide `stopLoading`.
2. The existing new-document request set observes its CDP session, rather than
   an authoritative selected-Frame subtree across renderer sessions.
3. Its same-document branch waits for a matching hash-navigation event, but
   does not additionally wait for the requested existing-document lifecycle.
4. Composition's Frame evaluation routes depend on execution contexts. A
   navigation route must remain available when the Frame exists but its world
   does not. Locator/utility-world selection is not a navigation prerequisite.
5. Node `Response.frame()` currently returns `page.mainFrame()`. A child-frame
   navigation response must retain the actual selected Frame wrapper.

The native Frame initiation path additionally rejects CDP
`net::ERR_HTTP_RESPONSE_CODE_FAILURE`, which the legacy Page begin parser permits.
This is an explicit native initiation error, not an ordinary HTTP 404/500
response; ordinary HTTP statuses remain response data. Page behavior is unchanged.
Download receipts remain distinguishable from success; the future full operation
must reject downloads according to the pinned contract rather than returning a
successful loaded document.

## Design decisions and remaining implementation

### 1. A distinct portable Frame command

Add a generated Frame-targeted command instead of broadening Page `Goto`'s
target kinds or interpreting one object kind as another. The proposed additive
navigation revision is 1.2, retaining the published 1.1 compatibility baseline.
Regenerate contracts, codecs, client/server adapters and references together;
prove old method IDs and Page behavior unchanged. This schema change is pending.

Keep the richer lifecycle/routing implementation explicitly selectable. A
navigation-only installation must not gain Locator, input, artifacts, launch or
Node driver dependencies. A shared type definition is not permission to install
an unrelated runtime service or to advertise an unsupported backend command.

### 2. Exact native route and dispatch lifetime

The new borrowed route follows the topology's nearest Main/remote renderer,
while preserving the selected descendant's native Frame ID. It has no map,
string copies, world allocation or I/O. Missing Frames, Page IDs passed as
Frames, crash, reconnect and ambiguous renderer-swap gaps yield no route.
The route is scoped to its immutable topology borrow: it is **not** an owned
dispatch lease. Absence alone does not distinguish detach from a recoverable
swap; the higher layer must consult authoritative lifecycle state.

Next implement an optional admitted owned invocation retaining the original
Frame/Page/runtime lifetime, client and routing identities. Revalidate the
epoch and exact route immediately before first native dispatch. Waiting for a
route before dispatch can be allowed by the command deadline; changing session
and retrying after a possibly sent navigation is not allowed. Rebinding an
existing remote Frame preserves that Frame but retires its old child subtree;
stale child identities must not become routes into the replacement document.

### 3. Rust lifecycle and response state

Install bounded observation before dispatch. Join it with topology updates so
Frame/loader/request/session identity, redirects, same-document events and
renderer swaps are correlated explicitly. A persistent per-Frame lifecycle
snapshot is needed to observe an already-fired lifecycle after hash navigation;
a new command-local subscription alone cannot recover an earlier event.

The Rust state machine must handle `commit`, `domcontentloaded`, `load` and
`networkidle`, retaining only admitted state. Network-idle must account for the
selected subtree, including remote descendants, and ignore sibling activity.
Inject timing/scheduling policy; do not move retries, navigation waits or
network-idle timers into Node. Event loss must fail explicitly, not synthesize a
successful lifecycle or repeat a side effect.

Keep final document response identity separate from a load event. Verify redirects,
HTTP errors, network failures, downloads, data/about URLs and same-document null
responses. Do not infer body/stream completion from navigation completion.

### 4. Cancellation, publication and Node value glue

Before implementing cancellation, characterize timeout/detach/close/disconnect
against the pinned oracle. Do not reuse Page-wide `stopLoading` for child Frame
cancellation. Prove sibling/ancestor navigation survives a child deadline and
that pending native requests/leases are retired without retaining the whole host.

Reuse Rust-owned Context baseURL with its admitted foreign URL-value conversion.
Node should contribute only language-value validation, promises and existing
weak-wrapper identity; no JavaScript automation driver or CDP client is part of
the runtime package. Preserve the selected Frame in `Response.frame()` and flush
the existing Rust publication fence before public promise completion. Frame
events and their ordering must be verified separately, not inferred from a final
tree snapshot.

## Resource and dependency contract

Native initiation validates the borrowed Frame ID and URL/referrer limits before
I/O. Successful admission uses the same production operation counter and
`BackendStateBytes` lease as Page navigation, acquired before boxing the active
future. An unpolled future holds admission and releases it on drop. No second
route owner or event/request index is added by this primitive. Its at-most-four
overlapping native-ID copies fit the existing eight-ID allowance; caller input
and transport envelope ownership remain separate. Unit tests exercise shared
exhaustion, pre-dispatch validation, error propagation and drop/re-admission.

The borrowed topology route allocates no owned state. A future retained route,
lifecycle history/subtree request index, callback/promise storage and FFI result
copies need explicit pre-allocation admission and release tests. These existing
primitive bounds do **not** prove the future full navigation operation bounded,
nor are they a whole-process Rust/Node/Chrome RAM limit.

No Cargo dependency or runtime JS driver import was added. Navigation-only stays
at 15 Pwright packages; topology-only stays at five. The Node addon was not
rebuilt: NODE-063 selected artifact `1b627f19...` and its earlier public results
remain separate from this newly compiled Rust test binary.

## Verification and reproduction

Current reports are under `.work/node064-frame-navigation/` (local evidence,
not a committed public artifact):

- `oracle-oopif.json` / `oracle-oopif.log`: six pinned public observations.
- `live-oopif.log` and final-source `live-final.log`: six native matches,
  verified owned OOPIF session, invalid target isolation and confirmed Context disposal.
- `topology-tests-final.log`: 46 tests, including six borrowed-route checks.
- `backend-final.log`: 21 navigation tests including strict Frame initiation error
  handling; `cdp-tests.log`: both generated-parameter serialization checks.
- `leaf-tests-final.log`: navigation/CDP/fake release regressions.
- `clippy-final-all.log`, `fmt.log`, `dependencies.log` and
  `topology-dependencies.log`: strict code/dependency checks.
- `host-addon-default.log` / `host-addon-selected.log`: both current-source
  Rust host/addon configurations compile. `node-units.log` has 46 passes;
  `types.log` and `node-boundary.log` pass strict types and exact dependency/import
  checks. No addon binary was rebuilt by these checks.

The final native Context was confirmed disposed, the exact task preflight target
was closed and verified absent, and `/pwright-node064-frame-navigation` was
deregistered. Other Chrome targets and XLQY routes/servers were left intact.
No task-owned build or test process remains. No new commit or push was made;
the foundation commit remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`.

Retain the earlier reports: `oracle.json` mislabeled the nested hash-only case
as a new document; `oracle-document.json` and `live.log` correct that four-case
coverage. The final fixture uses a distinct query for new-document navigation,
and the oracle asserts response-vs-null before recording. The initial topology
test incorrectly expected a remote rebind to preserve its old nested document;
`topology-tests.log` retains that failure, and the corrected test explicitly
checks retirement rather than changing existing topology policy.

From a task-specific directory, start/verify dedicated `chrome-cdp`, then use
`pwright --cdp http://127.0.0.1:9222 open about:blank`. Register `tests/fixtures`
with serve-lib on a unique route bound to the Tailscale IP, and verify the exact
MagicDNS URL. Do not reuse another task's preflight target or restart shared
Chrome. Set the following for the oracle (all output paths must be unused):

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_FRAME_NAVIGATION_FIXTURE=<verified-MagicDNS-fixture-directory-URL> \
PWRIGHT_FRAME_NAVIGATION_ORACLE=<absolute-oracle-path> \
node packages/pwright/scripts/probe-frame-navigation.mjs
```

Then run the Rust gate using the current verified browser WebSocket URL:

```sh
PWRIGHT_CDP_WS_URL=<verified-loopback-browser-WebSocket-URL> \
PWRIGHT_FRAME_NAVIGATION_ORACLE=<absolute-oracle-path> \
CARGO_INCREMENTAL=0 cargo test --locked --release \
  -p pwright-backend-cdp-navigation --test live_frame_begin -- --ignored --nocapture
```

The oracle is capped at 64 KiB. Rust exercise timeout is 30 seconds; explicit
diagnostic settlement is capped at five seconds, outside the initiation API.
The test returns comparison failures through Context cleanup instead of panicking
before cleanup, and OOPIF selection requires the exact owned Context, type and
fixture URL. Dispose only owned Contexts/targets and deregister the exact route.

## Completion gates still open

- [x] Opt-in retained route admission and transaction-coupled revocation (NODE-066).
- [ ] Real retained-owner session-swap/detach races and native cancellation control.
- [x] Optional fixed-capacity normalized lifecycle reducer and pinned internal traces (NODE-065).
- [ ] Portable Frame command and explicitly selected Rust lifecycle service with native normalization.
- [ ] All wait policies, including existing lifecycle on same-document navigation.
- [ ] Subtree network-idle across OOPIFs; busy siblings must not block it.
- [ ] Redirects, downloads, network failures, no-response URLs and interruption.
- [ ] Timeout/close/disconnect with exact native-request/lease retirement.
- [ ] Host/N-API/public `Frame.goto`, baseURL, actual `Response.frame` identity.
- [ ] Publication/event-order differential tests, reentry/GC/Worker shutdown.
- [ ] Rebuilt selected artifact, full regression corpus and dependency/resource proofs.

These gates remain part of the original Node compatibility goal, alongside the
other API families and the open attachment-reliability investigation.
