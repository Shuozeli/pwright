<!-- agent-updated: 2026-09-09T09:16:43Z -->

# Chromium drag: listener ownership and remaining integration

NODE-076 adds the separate `drag-frame-navigation` native artifact, combining
independent drag-runtime and frame-navigation Cargo features. It preserves the
existing drag-only/default files, verifies both compiled flags, passes strict
combined Clippy/dependency checks, and passes the existing public
drag/Keyboard/Locator 10/10 comparison. Its Frame lane has 23 paired matches,
one passing independent Rust-close check and an upstream close failure; this
is not whole-package compatibility or a resolution of the close discrepancy.
[Artifact hashes and evidence](pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-052 rebuilds the selected addon after Frame projection and owned-OOPIF
attachment fixes. Actual Mouse 44/44, Keyboard 24/24 and drag/Locator 10/10
comparisons pass again, with SHA-256
`7f6f11a73cada576e5ddcaa96edc7116fa44aa667d012bf8dfe01d3491e0606c`.
The earlier NODE-050 hash/results below remain historical. Public child-Frame
wrappers are not yet exposed.
[Native failures, fixes and artifact evidence](pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

## NODE-050: native lifetime gates and selected Node addon

The selected release **Node FFI** now matches pinned Playwright 1.58.2 for
**44/44 public Mouse**, **24/24 public Keyboard**, and **10/10 native
drag/Keyboard/Locator** scenarios on Chrome 152.0.7977.76. These are actual
public Node calls into Rust, not the earlier internal HostLocator-only proof.
They do not establish full Playwright compatibility, public child-Frame APIs,
all navigation races, or memory/RSS improvements.

### Expanded real Frame lifecycle evidence

The shared Frame catalog expands from twelve to eighteen cases. Both local and
OOPIF variants now cover:

- Removing the Frame before focus: an absent selector makes focus impossible.
  Rust explicitly polls the request and verifies one admitted active operation
  before removal; both sides verify no focus/key events. Frame removal rejects
  the call, the Frame route disappears, and later Page keydown is not a repeat
  from input incorrectly started by that rejected Locator.
- Closing Page during the post-focus Page key delay, after native `dragend` but
  before keyup; the owning Context survives.
- Closing Context at the same phase; both Context and Page disappear.

All eighteen cases match the DOM input/drag observations and specified terminal
categories. Rust asserts `HostError::TargetClosed` for closure; upstream Node
asserts `name === 'Error'` plus the correct Page/Context-close or detached-Frame
message. The initial assumption that the error name was `TargetClosedError`
failed against the real oracle. Pinned `lib/client/errors.js` confirms that its
TargetClosedError constructor inherits Error.name rather than assigning a new
name. This is recorded evidence, not renaming a Rust error to pass a test.
Public child-Frame error-message fidelity is still ungated because those Node
wrappers are not implemented.

Before host shutdown the harness checks no active operations remain. After
shutdown it checks zero host leases/operations and the expected borrowed Page
lifetime, then verifies disposal of only its own native Context/targets. These
tests close the owner during a key delay AFTER Escape cancellation has already
completed; they do not prove Page destruction while the native cancellation
request itself is still in flight.

### Explicit local Node artifact selection

The addon has default-off `drag-runtime`, forwarding to the existing safe Rust
host feature. Node remains a conversion/object-facade boundary: it implements
no CDP transport, drag state, input ordering, lifecycle scheduler or browser
launcher. Two local build profiles coexist:

| Selection | Artifact | Compiled drag |
| --- | --- | --- |
| unset / `PWRIGHT_NATIVE_PROFILE=default` | `native/pwright.node` | no |
| `PWRIGHT_NATIVE_PROFILE=drag-runtime` | `native/drag-runtime/pwright.node` | yes |

From `packages/pwright`, use `PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm build`,
then the same environment selection when running Node or comparison scripts.
The build atomically installs only that profile's binary. It does not replace
the default artifact, download a browser, or launch one. These are experimental
local build profiles, not published per-feature npm binaries or verified
cross-platform packages.

`native-artifact.cjs` captures the requested profile once per process; the loader,
build script and artifact reporters share its fixed paths. Unknown profiles and
path injection fail. Missing selected binaries do not fall back to default.
Before accepting a newly loaded binary, the loader checks the exported Rust
`dragRuntimeEnabled()` capability. A selected binary must prove true; selected
code that declares that capability in the default path is rejected. Legacy
binaries without the export cannot self-report their features; the known older
default binary remains usable only through the default selection, and its
unchanged hash is checked separately in this checkpoint. Failed validation does not
cache the mismatched binary as an accepted addon. `compatibility.nativeProfile`
reports the requested profile, validated on first native use.

Mouse, Keyboard, shared-corpus and retained-world reporters hash the selected
artifact and record its profile, so they cannot label selected execution with
the old default file's hash. The public drag comparison shares its exact action
executor with the existing Rust-host oracle recorder. It compares every native
DOM snapshot before close, verifies owned fixture targets are absent, and
requires zero Node host leases, operations and cleanup work after disconnect.

### NODE-050 evidence and remaining work

Evidence directory: `.work/node050-frame-lifecycle.14wlF2/`.

- `oracle.log`: failed initial error-name assumption; `oracle-verified.json`
  / `live-16.log`: all sixteen cases after adding owner closure.
- `oracle-18.json` / `oracle-18.log` and `live-18.log`: all eighteen Frame
  lifecycle cases, including pre-focus detach, pass with owned cleanup.
- `host-unit.log`: 118 pass / 8 native ignores. `clippy.log` and
  `addon-clippy.log`: strict release all-target checks pass.
- `node-contract-final.log`: 38 tests including fixed profile paths, invalid
  selection and binary-capability mismatch. Node and shared TypeScript checks
  pass; `shared-unit.log` has 30 passing harness tests.
- `boundary.log`: host default/source/runtime remain 234/235/235 normal packages
  including host; addon default/selected are 249/250 including addon. Only the
  selected profiles enable drag and lifetime handoff.
- `addon-build-verified.log` produces selected SHA256
  `375c27ea32fe97e347533ad1279f28bb113c1cdc4ac39ff36fc21947157f2db7`.
  `mouse-selected-final.json`, `keyboard-selected.json` and
  `drag-keyboard-selected-final.json` all refer to that binary and pass 44/44, 24/24
  and 10/10 respectively. The first Mouse run also passed, on the preceding
  selected binary before the capability export; final evidence reruns it.
- `keyboard-oracle-refactored.json` / `native-regression.log`: the extracted
  shared action runner still drives all ten original Rust-host oracle cases;
  those and native drop/host-close cancellation pass serially. The final public
  drag report additionally verifies exact native Context and target disappearance
  with at most one retained observer ownership record.
- The default addon remains byte-for-byte the NODE-045 artifact
  `8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.
  Its historical 39/44 Mouse result is not a fresh same-source default benchmark.
- `targets-final.json` and cleanup logs prove removal of only preflight target
  `E6993EFE9C2CC815AE53C50D4C094330` and `/pwright-node050` serve-lib registration.
  Fixture files, unrelated Pages/registrations and the existing dedicated Chrome
  remain intact. Both native artifacts are local/gitignored; neither was pushed.

Still open: Page/Context destruction during pending native cancellation, richer
navigation/removal and interleaved-move races, public child-Frame APIs and their
error details, the broader shared Node compatibility corpus, optional capability
distribution and process-memory attribution. The selected profile is opt-in and
experimental; these results do not make it the default or mark the overall
Node-compatibility goal complete. No new commit or push.

## NODE-049: optional Frame-to-Page lifetime handoff

The two NODE-048 post-focus detach discrepancies are fixed in the selected Rust
host. The shared Frame catalog now has twelve cases: the original ten plus
local/OOPIF removal during a finite 1500 ms Escape delay with navigation waiting
enabled. All twelve match pinned Playwright 1.58.2 on Chrome 152.0.7977.76.
These are real HostLocator tests, not new public Node child-Frame APIs or a
rebuilt addon. The older NODE-048 failing results below remain historical evidence.

### Phase transition and cancellation identity

`pwright-runtime/lifetime-handoff` is default-off and adds no Cargo dependency.
Only host `drag-runtime` selects it. An admitted executing command prepares one
caller-budgeted `CallLifetimeHandoff`; successful Locator focus commits it
synchronously, before logical key state or native input changes. Press and
sequential typing share the optional `LocatorInputHandoff` leaf port. The leaf
does not depend on runtime, drag, Node, or a new transport. Already Page-targeted
host calls do not need a handoff.

Preparation and commit validate the original pending call, executing admission,
live source and strict live ancestor, and absence of prior cancellation. Commit
rechecks all of these under the runtime store lock used by lifecycle disposal.
The original cancellation-signal Arc fences CallId reuse; a weak store reference
avoids a runtime/backend cleanup cycle. One OnceLock changes the lifecycle owner
exactly once. Prepared disposal recalculates affected calls when it commits, so
a handoff may safely linearize between disposal preparation and commit.

Original Frame command metadata, target scheduler permit, connection permit,
timeout budget, caller cancellation identity and retained service binding stay
unchanged. All seven subtree cancellation/idle-selection paths now use the
effective owner; they do not keep a detached Frame alive or mutate the graph.
Source removal after commit no longer cancels input. Page/ancestor closure,
transport closure, typed Page failure and deadline still terminate it through
the existing cleanup path. Source disposal or caller cancellation before commit
wins and prevents input. No replay, deadline reset, background worker or new
registry is introduced.

### Navigation correction exposed by the stronger gate

The first expanded run matched eleven cases but failed `sandbox-detach-finish`.
The test originally joined a failed press with a `dragend` observer, hiding the
operation error behind its 25-second supervisor. It now uses fail-fast try-join
and bounded phase labels, while retaining failed-case cleanup and aggregate
nonzero exit. The exact error was `Page.getFrameTree returned a root frame with
a parent`, before focus/input: an OOPIF renderer root legitimately has a parent.

Locator navigation preparation now reads the session's frame tree directly,
without applying Page creation's parentless-root assertion. The Page creation
assertion itself is unchanged. Press keeps its Frame event subscription but
sends the input epilogue `Page.enable` to the retained Page keyboard client,
so successful keyup does not require sending to the deleted OOPIF session.
The finite-delay oracle and host also reject a child keyup already present in
the pre-removal snapshot. This does not establish complete navigation parity
when input itself requests navigation and that Frame is subsequently removed.

### Resource and verification boundary

Selected runtime requirements charge the entire cancellation-signal allocation
plus Arc counters: `size_of::<CancellationSignal>() + 2 * size_of::<usize>()`.
It is **152 bytes on this Mac**, **2432 bytes** for the host's sixteen pending
calls, and one additional lease per call in its existing Call scope. This is
explicit `BackendStateBytes` admission before signal allocation. The signal owns
the lease until its last reference drops, including a retained handoff token
after PendingCall completion. Tests prove exhausted retained-signal storage
rejects a new call without leaking its pending lease, then becomes available
after the last token drops. Token/provider storage stays inside the existing
16 KiB fixed allowance of each bounded Locator invocation. No new Page slots,
event cursors, native command capacity or tasks are introduced; 536/272 controller
slots and 552/32 CDP pending/write bounds remain unchanged. This is logical
allocation accounting, not RSS measurement.

Evidence: `.work/node049-lifetime-handoff.cWsdw3/`.

- `oracle-final.json` / `oracle-final.log`: twelve pinned native observations.
- `live-final.log` and `live-staged.log`: eleven matches and the OOPIF finite
  failure; `live-diagnostic.log` exposes the actual parented-root error. These
  failed runs remain failures; neither timeout nor expected result was relaxed.
- `live-verified.log`: all twelve native matches after the real fix, including
  post-focus detach ending by deadline and by successful finite completion.
  Each case checks input/drag snapshots before close, zero host owners, borrowed
  Page survival, and removal of its own Context/targets.
- `runtime-accounted-final.log`: 129 selected runtime tests, including sixteen
  new tests for handoff ordering, identity, deadline, lifecycle paths, permits
  and last-reference byte accounting. `runtime-default.log`: 113 default tests.
- `locator-verified.log`: 112 Locator tests, including handoff rejection before
  any key/text input, original Page epilogue and parented session-root handling.
- `host-unit.log`: 118 pass / 8 explicit native ignores. The ignored lanes are
  run separately with dedicated Chrome and exact oracle inputs.
- `oracle-guarded.json` / `oracle-guarded.log` and `live-guarded.log`: fresh
  twelve-case rerun with the additional pre-removal keyup assertion, all match.
- `composition-unit.log`: 310 pass / 15 explicit native ignores;
  `clippy.log`: strict release all-target checks pass across runtime, Locator,
  composition and selected host. `node-contract.log` / `node-types.log`: 34
  package contract tests and strict TypeScript check pass; not native parity.
- `keyboard-oracle.json` / `native-regression-serial.log`: ten original
  Keyboard/Locator oracle cases plus native drop and host-close cancellation
  pass, with the two native test functions explicitly serialized.
- `boundary-verified.log`: default/source/runtime closures are 234/235/235
  packages including host. Only `drag-runtime` enables lifetime handoff. The
  first boundary-script attempt incorrectly searched Cargo feature-edge text;
  the final check asserts resolved `{f}` features on the actual runtime package.
  `input-deps.log` / `locator-deps.log` pass the five/ten-package leaf proofs.
- `targets-final.json` and cleanup logs verify removal of only the task's
  preflight target `7983CCA39EBA53107EF295C4DA189D61` and `/pwright-node049`
  serve-lib registration. Fixture files and unrelated Pages/registrations remain;
  the dedicated Chrome was not restarted or closed.

Remaining gates: native pre-focus detach, Page/Context destruction while the
transferred command or native Escape cancellation is pending, and richer
navigation/removal races. Runtime deterministic tests do not replace these
native receipt checks. Public child-Frame APIs, selected addon/public Node
differential coverage and the other known Mouse discrepancies remain open.
The NODE-045 default addon hash remains
`8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`;
its prior 39/44 public Mouse baseline was not rerun by these Rust-host tests.
No new commit or push; the overall Node compatibility goal remains active.

## NODE-048: native Frame gate exposes a lifetime discrepancy

The new shared `tests/data/raw-drag-frame-cases.json` drives pinned Playwright
and actual Rust HostLocator comparisons. `raw-drag.html?frame-mode=local` embeds
a same-process child; `frame-mode=sandbox` uses `sandbox="allow-scripts"` with
the same verified Tailscale fixture URL. On the dedicated Chrome 152.0.7977.76,
the sandbox child has its own native iframe target. Both oracle and host reject
a missing/unexpected process boundary: the oracle checks native Target inventory,
and the host additionally checks Frame parent/Page ownership and distinct versus
shared renderer-session identity. No Chrome flags/profile restart, alternate
hostname, public server or browser download was needed.

Eight scenarios match: local/OOPIF Escape, Control+Escape, sequential `a中`, and
a 60-second delayed Escape that reaches its 2-second timeout. Native key events,
modifier/repeat state, focus, input value and parent drag events are compared
before host close. The borrowed Page/Frame survives close unchanged; host owners
return to zero, and each test-owned Context/target is independently removed.

The expanded ten-case gate is **failing, 8/10**, not a full Frame pass. Removing
the child AFTER Escape has canceled the native drag but BEFORE the delayed
press ends exposes the same discrepancy for local and OOPIF Frames:

| Stage | Pinned Playwright 1.58.2 | Current selected Rust host |
| --- | --- | --- |
| Focus and Escape cancel | Frame focus, Page keyboard/drag | Matching native behavior |
| Remove iframe during 60-second key delay | Page-owned press remains pending | Frame-bound runtime call ends with TargetClosed |
| Five-second command deadline | TimeoutError; Page logical key survives | Already terminated; parity gate fails |

The first oracle assertion incorrectly expected a detached-Frame error. Actual
Playwright returned TimeoutError. The test now asserts that observed behavior;
it does NOT rename TargetClosed, accept either error, skip either case, or delay
the Rust error to manufacture a timing match. All cases run, audit owned cleanup
even on failure, and the aggregate test exits nonzero for discrepancies.

### Evidence and scope

Evidence directory: `.work/node048-frame-keyboard.spTN9z/`.

- `oracle.json` / `live-frames-second.log`: initial eight-case oracle/host match.
  The first host run (`live-frames.log`) hit an existing five-second
  TopologyInitialization connect timeout on the second case. It remains an
  unresolved connection-stability failure, not a parity success or fixed bug.
  No connection timeout or assertion was relaxed; the terminal failed run was
  followed by a complete successful run.
- `oracle-detach.log`: records the mistaken oracle expectation failure.
  `oracle-detach-verified.json` records all ten pinned native observations.
- `live-all.log`: eight native matches and explicit failures for `local-detach`
  and `sandbox-detach`, with failed-case host-owner and Context/target cleanup.
- Final shared-catalog runs `oracle-final.json` / `oracle-final.log` and
  `live-final.log` repeat all ten cases: 8 match, the same two detach failures,
  aggregate exit 101. No native failure is reclassified as a successful test.
- `host-unit-final.log`: 118 pass / 8 explicit native ignores.
  `clippy-verified.log`: strict release all-target checks pass.
  `keyboard-oracle.json` / `keyboard-regression.log`: all ten original
  non-Frame Keyboard/Locator cases plus drop/close-cancel regressions pass
  against the modified fixture. Dependency boundary checks remain 234/235/235.
- `probe-drag-frames.mjs` and `drag/live_frames.rs` use the shared catalog.
  The native Rust test is an explicit ignored browser lane like the existing
  harnesses, not a disabled assertion; running it exits nonzero on discrepancies.
- The Rust test enumerates existing portable topology internally only to select
  the real HostLocator. Public Node child-Frame enumeration/wrappers are still
  missing. This verification does not implement `Page.frames()` or `childFrames()`.
- Production dispatch and resource profiles, dependency closures and the built
  NODE-045 addon are unchanged in this checkpoint. No commit or push.
- `targets-final.json` verifies task preflight target
  `CE461EF194BF50D95E68D9A5FA6AEF4B` was closed. The task-only `/pwright-node048`
  serve-lib registration was removed after validation; fixture files remain.
  Unrelated registrations/Pages and the Chrome process were preserved.

Reproduction requires the standard dedicated Chrome/Pwright preflight and a
verified serve-lib URL for `raw-drag.html`. Set `PWRIGHT_TEST_CDP` to the dedicated
loopback HTTP endpoint, `PWRIGHT_DRAG_FIXTURE` to that fixture URL, and
`PWRIGHT_DRAG_FRAME_ORACLE` to a new absolute output path, then run
`node packages/pwright/scripts/probe-drag-frames.mjs`. With the current verified
browser WebSocket in `PWRIGHT_CDP_WS_URL` and the same fixture/oracle variables,
run `CARGO_INCREMENTAL=0 cargo test --release -p pwright-node-host --features
drag-runtime --lib host_locator_keyboard_matches_native_local_and_oopif_frames
-- --ignored --nocapture`. It is expected to fail until the lifetime defect is
fixed; do not filter out the detach catalog entries.

### Next implementation: explicit Frame-to-Page lifetime transition

Pinned `server/dom.js::_press` focuses the element, then awaits
`this._page.keyboard.press`; `server/input.js::press` waits on command progress
between Page keydown and keyup. The observed Frame removal does not cancel that
Page input phase. In contrast, selected host `locator/keyboard.rs` dispatches one
Frame-targeted controlled command; runtime Frame retirement cancels it even
after focus and Page input have begun.

The fix must preserve these boundaries, rather than merely change error text:

1. Selector resolution/focus stays Frame-owned. Detachment before the handoff
   must still fail; no input may start after that Frame stage has been canceled.
2. After successful focus, admit an explicit Page-owned input continuation
   atomically with lifecycle validation. Retain the exact Page native client,
   input/controller and pending native owners; never re-resolve a replacement
   Frame/Page or replay input.
3. Preserve original command/Frame metadata and bounded command/target admission
   while transferring cancellation ownership. Do not globally keep detached
   Frames alive, ignore lifecycle controls, or dispatch the entire operation on
   Page from the outset (that changes pre-focus cancellation and concurrency).
4. Page/Context closure, disconnect, timeout and host shutdown must still stop
   the Page continuation and join native retirement before transport teardown.
5. Add deterministic handoff-race/lease tests and native local/OOPIF tests for
   pre-focus detach, post-focus detach with successful finite delay, deadline,
   Page destruction, and native-cancel interruption. Current zero owner counters
   do not prove all of those continuation/destruction receipts.

This requires an explicit optional lifetime-handoff contract between the
controlled runtime and the Locator input phase; no such contract is implemented
by this checkpoint. Keep the ten-case discrepancy gate mandatory and failing
until the real behavior matches. The overall Node compatibility goal is active.

## NODE-047: Locator keyboard selects the owning Page

The opt-in Rust `drag-runtime` host now sends Locator `press` and
`pressSequentially` through the same Page/native-client drag controller as raw
Keyboard and Mouse. The Frame utility client still performs selector resolution,
focus and the existing Press navigation barrier. Keyboard and insertText use
the original owning Page client. Pointer Locator actions are unchanged.

`ChromiumLocatorKeyboardRoute` waits through existing readiness when selected,
then takes the Frame service and owning Page client/input under one route lock.
Ownership comes from the Frame service's retained Page input identity, not the
optional readiness inventory; prepared-only compositions therefore work too.
It retains both route-storage leases, creates no new native session/world or
task, and never reconstructs native identity from a session-ID string. Selection
rejects a foreign input owner, a second provider, or exclusive input policy.

The host admits a per-command invocation before taking this snapshot. Its
cancellation handler retains the exact selected Frame service and Page
controller before input starts, never re-resolving a replacement route. It runs
service cleanup and drains Page quarantine even if cleanup fails. The provider
is transaction-local, not stored in shared Page input, avoiding an ownership
cycle through the controller's Frame-world access.

### NODE-047 resources and evidence

- Scope 122/10 admits 16 retained invocations, each with 1 MiB text, 64 KiB
  selector and 16 KiB call/snapshot allowance (17.25 MiB logical total).
  The existing Locator factory still admits eight executing actions. Native
  provider futures and retained route leases keep their separate owners.
- Eight additional Locator owners bring each selected controller to 536
  dispatcher slots / 272 Page commands and an 86 MiB logical provider allowance
  plus fixed storage. Shared CDP pending/write bounds remain 552/32. These are
  admission allowances, not eager allocation or measured RSS improvements.
- `.work/node047-locator-keyboard.OclAGR/locator-unit.log`: 108 release tests,
  including five new Frame/Page routing, typing and rejection tests.
- `oracle-final.json` and `live-drag-current.log`: all 10 pinned Playwright 1.58.2
  native snapshots match on Chrome 152.0.7977.76 BEFORE host close. Five original
  Keyboard cases are joined by idle/drag Locator Escape, Control+Escape,
  sequential text and a 60-second delayed press canceled by a 2-second timeout.
  The timeout case verifies the subsequent repeated Escape and keyup, not just
  local timeout delivery. Existing drop/close-cancel cases also pass; borrowed
  Pages survive and host leases/operations return to zero before fixture removal.
- `composition-unit.log`: 310 pass / 15 explicit ignores, including prepared-only
  Page/main/local-child/OOPIF snapshots. Selector execution keeps its exact
  utility-world identity; every key dispatch uses the root Page client, never
  the OOPIF client. `host-unit.log`: 118 pass / 7 explicit ignores.
  The final `units-verified.log` repeats all three unit suites successfully
  after the readiness-race recheck change: 536 pass / 22 explicit ignores.
- `mouse-oracle.json` / `live-mouse.log`: 25/25 native Mouse comparisons and the
  host-close-during-click-delay retirement regression pass. Strict release
  Clippy (`clippy-current.log`), default host check, formatting and dependency
  checks also pass.
- Default/drag-source/drag-runtime host closures remain 234/235/235; drag stays
  3/5, input-only stays 5. Locator gains no drag dependency.
- `targets-final.json` confirms removal of this task's preflight target
  `6C46663350D18FB4A4063C5AE107F0EC`. Each native harness independently removed
  its own Context/target; unrelated targets and the Chrome process were kept.

The first native test attempt rejected the fixture's one-recipe query budget:
lazy query plus command conversion require two admitted recipes. The test now
explicitly admits both; production admission was not relaxed. The first
composition boundary attempt exposed the optional-readiness ownership bug
described above. Failed logs are preserved with subsequent evidence.

Real child/OOPIF Locator keyboard, native destruction during retained commands,
and a pending-native-cancel interruption through the Locator host remain further
gates. Lower-layer gated Escape retirement tests do not establish those host
cases. The Node addon is unchanged (NODE-045 hash below); this is Rust-host
integration evidence, not a new public Node conformance result. No commit/push.

## NODE-046: optional Rust Keyboard Escape interception

The selected `drag-runtime` host now shares its exact Page/native-client
controller between Mouse and Keyboard. A transaction-local
`CdpKeyDownInterceptor` port lives in the input leaf; Keyboard backend/Page
bindings optionally inject it. Default bindings remain unintercepted. It is not
installed into shared Page state, avoiding a Page-input/controller ownership
cycle and accidental replacement of other transactions' providers.

The hook runs after US key resolution and logical pressed-key/modifier updates,
but before native keydown. `None` passes through without a provider allocation.
The drag dispatcher selects only code `Escape`, validates the original native
client Arc, and admits a lazy owner from its existing registry before boxing it.
An unpolled owner performs no cancellation or native IO. On first poll it starts
the existing DragPage cancel operation: false continues normal keydown; true
suppresses only that keydown. Native cancel errors propagate and preserve drag
state. Keyup and insertText never invoke the hook; consumed Escape still counts
as logically pressed for later autoRepeat.

Dropping a started Escape future transfers its native cancel owner to the same
Page quarantine used by Mouse. The unique driver drains its existing native
reply and cancel continuation; it neither sends a second cancel nor synthesizes
keyup/mouseup. This intentionally preserves upstream's cancelDrag continuation
after a progress-race cancellation. Keyboard's exact host cancellation handler
retains the controller and drives quarantine after route cleanup, including if
route cleanup errors. Host shutdown also drains those owners before inspector
session/transport teardown. A stopped/quarantined dispatcher rejects new handled
work rather than pretending dirty native state was cleared.

### NODE-046 selection and resource boundary

- Hook storage/operation trees remain provider-owned; the input/Keyboard leaves
  gain no drag dependency, executor, task or native-client wrapper. Selecting a
  second hook, or selecting one on the legacy exclusive transaction policy,
  fails explicitly without native input effects.
- The opt-in host adds eight Keyboard cancellation owners to NODE-045's Page
  budgets: 528 dispatcher slots and 264 DragPage command slots per controller.
  The logical native-provider allowance is now 83.5 MiB plus fixed storage per
  controller, not eager allocation or measured RSS. Existing non-Mouse CDP
  headroom covers these Keyboard commands; pending/write limits stay 552/32.
- Host default/source/runtime closures remain 234/235/235; drag remains 3/5.
  Default v1/Keyboard behavior and default addon feature selection are unchanged.

### NODE-046 verification

Evidence directory: `.work/node046-escape.hdwLaG/`.

- `drag-unit-final.log`: **99** release tests pass. Eight added regressions
  cover idle Escape pass-through, consumed keydown plus repeated logical state
  and unconsumed keyup, pending-cancel quarantine, native cancel failure/retry,
  unpolled cancellation, exact client identity/double selection, ordinary-key
  pass-through under full drag admission, and legacy-policy rejection.
- `host-unit.log`: **118** selected-host unit tests pass with **7** explicit
  ignores. `keyboard-layers-unit.log`: **85** input, **18** Keyboard backend and
  **18** runtime-selected Keyboard composition unit tests pass.
- `oracle-final.json` / `oracle-final.log`: five fresh pinned Playwright 1.58.2
  drag/Keyboard scenarios on Chrome 152.0.7977.76. The fixture records bounded
  native DOM key events (code, repeat and modifier flags) alongside drag events.
  Cases cover idle press, drag press, repeated down, Control+Escape chord and
  Escape keyup without keydown. Observations happen before Context close.
- `live-drag-final.log` passes both existing drop/close cases and all **5/5** exact
  host-versus-oracle snapshots. The host compares BEFORE closing so close-time
  cancel cannot hide missing Escape behavior. Borrowed Page survival, zero host
  leases/operations and exact test-owned Context/target removal are checked too.
- `live-mouse.log`: **25/25** ordinary Mouse comparisons still match the fresh
  pinned `mouse-oracle.json`, and host close during click delay still returns
  TargetClosed without synthetic mouseup/click or retained host owners.
- `clippy-verified.log`: strict release all-target drag/host checks pass.
  `default-check.log`: unselected host release check passes. Dependency,
  formatting and diff checks preserve the optional boundaries.
  Keyboard backend/composition closures remain **8/16** Pwright packages;
  selected runtime adds only `pwright-runtime` (**17**). The input-only
  release dependency proof still passes with **5** Pwright packages.
- `targets-final.json` verifies task preflight target
  `E58DDA9BEFBA7C670DF33AB98E1C6963` and fixture targets are absent. The existing
  serve-lib fixture route was renewed for one hour; unrelated Pages and the
  dedicated Chrome process were not closed/restarted.

### Still required before public drag selection

NODE-047 wires Locator press/sequential keyboard transactions into the same
controller. Other Locator pointer actions are still on their existing routes;
real child/OOPIF and native interruption cases remain required.
Exact Page/Context native destruction retirement, overlapping intercepted moves,
non-stalling world loss and post-movement Frame enumeration remain open.

The Node addon was not rebuilt for this checkpoint; its NODE-045 SHA256 remains
`8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.
Public Mouse evidence remains **39/44**, not a new Node drag pass count.
No commit or push; the overall Node compatibility goal remains incomplete.

## NODE-045: maximum click batches and matching host admission

The 25-case raw Mouse corpus now includes click counts 15, 16 and 32. Before
the fix, the selected HostSession matched 15 but rejected 16 and 32 at the drag
dispatcher. A no-delay click initiates `2 * count + 1` children before awaiting
their replies; serializing those calls to fit a small queue would change the
pinned upstream behavior.

The dispatcher now supports explicit capacities 1..=1024 using fixed, admitted
16-word bitsets for occupancy, quarantine and per-round completion. Slots beyond
31/63/511 remain independent; the driver still polls outside its registry lock,
retains dirty work and performs no allocation or automatic retry per round.
DragPage's independently admitted command capacity accepts the same 1..=1024
range. These are maximum configurable bounds, not new default leaf installs.

The host derives capacity from the existing eight native Mouse-operation leases
and the shared input-model maximum of 32 click pairs:

- At most 65 child commands per click and 520 dispatcher slots per Page/native
  controller. Native-operation leases stay owned through retirement.
- 256 retained Page drag command slots per controller cover eight sets of up
  to 32 drop commands. The controller's native-provider allowance correspondingly
  changes from 11 MiB to 81 MiB plus fixed storage; payload/source/capture leases
  remain separately accounted. Ordinary drag source/capture concurrency is still
  one per Page; this does not prove overlapping intercepted-move parity.
- Both default and selected host CDP profiles admit 552 pending commands:
  520 Mouse children plus the previous 32-command non-Mouse allowance. The write
  queue remains 32 with its existing backpressure. No JS scheduler, hidden queue
  or serial-click fallback was introduced.
- `pwright-input-model` becomes a direct normal host dependency to share the
  accepted click bound; it was already in the transitive normal closure.
  Default/source/runtime closure counts remain 234/235/235; drag leaf 3/5.

These are conservative logical resource allowances, not eager allocations or an
RSS improvement. The unchanged CDP 4 MiB command/response ceilings make the
connection's declared worst-case wire allowance about 4.32 GiB at 552 pending
commands. Typed small-input wire allowances and allocation/RSS measurement remain
separate work; neither a zero lease count nor passing tiny fixtures proves a
whole-process memory ceiling. Full interleaved native behavior under eight
simultaneous maximum clicks is not inferred from the gated fake-client test.

### NODE-045 verification and actual Node artifact

Evidence directory: `.work/node045-mouse-capacity.IAxfj0/`.

- `before.log`: 16/32 click batches fail; the 15-click boundary passes, using
  the same fresh pinned 1.58.2 oracle recorded in `oracle.json` / `oracle.log`.
- `unit-drag-final.log`: **91** drag release unit tests pass. New regressions
  cover bitset word boundaries, invalid capacities, 65-command click initiation,
  520 concurrently held commands moved to quarantine, independent retirement of
  the last high slot, exact lease return, and 256 unpolled Page command owners.
- `unit-host-verified.log`: **118** selected-host release unit tests pass,
  **7** explicitly ignored. The new profile test checks the 65/520/552 derivation
  and unchanged write backpressure. `clippy-final.log`: strict release all-target
  drag/host Clippy passes. Formatting, diff and dependency checks pass.
- `after-final.log`: both native HostSession Mouse tests pass: **25/25** exact
  DOM-event comparisons, plus cancellation during a 60-second click delay with
  no synthetic release and no retained host owners. `live-drag.log`: actual
  drop and close-cancel still pass with borrowed Page survival and zero leases.
- `native-layers.log` / `native-runtime.log`: all five lower-layer native
  comparisons pass the same **25/25** corpus: raw input, typed backend,
  selected passthrough dispatcher, binary-wire composition and controlled-runtime
  composition. The test corpus was expanded without removing prior cases.
- `build-addon.log` rebuilds the default, non-drag Node addon. New SHA256:
  `8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.
  The previous binary is retained only in the ignored evidence directory.
  `public-mouse.json` / `public-mouse.log`: **39/44**, including all 25 raw
  cases. The same five `same-turn-move-down-move-up-{1..5}` discrepancies remain;
  the gate correctly exits nonzero. Its post-close host metrics are all zero.
- `node-mouse-native.log`: **3/3** Node validation/Worker termination tests;
  `keyboard.log` / `keyboard.json`: **24/24** public Keyboard comparisons;
  `screenshot.log`: **12/12** screenshot tests; `node-contract.log`: **34/34**
  contract/diagnostic tests; `types.log`: strict TypeScript check passes.
  `lifecycle.log`: **45/45** Browser/Context/FFI lifecycle regressions pass on
  this rebuilt artifact, under the existing per-case and aggregate supervisor.
- `targets-final.json` confirms the task preflight target
  `17E1FFA9776B437675EBD27EE2234F3D` and task fixture targets are absent.
  Only the task-created `/pwright-node045` serve-lib registration was removed;
  the pre-existing shared fixture route was retained. No unrelated Page was
  closed and Chrome was not restarted.

The addon still does not select `drag-runtime`. Escape cancellation, exact
Page/Context destruction integration, overlapping captured movements and the
remaining public discrepancies are still required; this is not goal completion.
No commit or push was performed.

## NODE-044: opt-in HostSession Mouse integration

`pwright-node-host/drag-runtime` selects the production source and dispatcher
inside actual HostSession Mouse calls. It is deliberately separate from
`drag-source`: default hosts and the Node addon do not select it yet. All
dispatch, captured data, Page state and retirement remain Rust-owned.

The host inventory admits at most 32 Page/native-client controllers. Reuse
requires both the Page object ID and the exact native client Arc. Each controller
owns its `DragPage`, dispatcher, source and unique retirement driver. The listener
namespace includes the native session ID to separate independent sessions sharing
a utility world. Stale controllers are stopped; only drained, non-dragging ones
are removed. Dirty or active state stays retained rather than treating route loss
as native cleanup proof.

Mouse binds the selected dispatcher through the existing Page input route.
Invocation workers retain their controller and drive quarantine after native
retirement receipts and once more before exiting. Host close seals admission,
joins Mouse workers, drains each controller and awaits native cancel for an active
drag before inspector-session and transport teardown. The existing topology
driver continues polling during Mouse shutdown so exact world destruction
receipts can advance. No new background task is introduced in a reusable leaf.

### Selected resource profile

- The existing Locator world-access installation is shared, not duplicated.
  The selected snapshot allowance is `32 * 32 + 8 = 1032`; default/source-only
  hosts retain the previous 8-slot allowance.
- Each controller selects 32 worlds, one retained source movement/capture/data
  record, 32 dispatcher/Page operations, and 256 KiB aggregate drag text.
- Reusable wrapper/data leases are separate from a logical native-provider
  allowance of `32 * 32 KiB + 32 * 320 KiB = 11 MiB` per controller plus fixed
  storage. This allowance is not an eager allocation or an RSS guarantee;
  connection buffers, serialization and allocator behavior remain distinct.
- `ChromiumAttachProductionProfile::with_extension_event_subscribers` adds
  checked, explicit extension cursor demand. Drag selects 32 additional capture
  cursors before deriving fanout. An already-derived undersized profile is
  rejected at preparation; adding extensions does not create a subscriber.

### Remaining compatibility gates

The Node addon is unchanged: public Mouse evidence remains **36/41**, not a new
pass count. Keyboard Escape is not wired to cancelDrag. Page/Context destruction
still needs its own exact native cleanup integration: a stale active controller
may report an unclean shutdown when its session is already gone. High-overlap
behavior, non-stalling world-loss handling and fresh post-movement enumeration
remain open. The dispatcher's 32-slot mask also needs reconciliation with the
public maximum of 32 click pairs (65 child commands) before addon selection;
the 22-case ordinary Mouse corpus does not cover that maximum.

### NODE-044 verification

Evidence directory: `.work/node044-host-drag.ZP83hv/`.

- `host-unit.log`: **117** selected-host release unit tests pass, with **7**
  native/source tests explicitly ignored in this unit-only invocation.
- `composition-verified.log`: **310** Chromium composition release unit tests
  pass, with **15** native tests explicitly ignored. New checks cover extension
  cursor demand, stale fanout rejection and checked extension-count overflow.
  `clippy-final-verified.log`: strict release all-target Clippy passes for both
  the Chromium composition and drag-runtime host. `fmt.log` and
  `diff-check.log`: workspace formatting and diff whitespace checks pass.
  `default-host-check.log`: the unselected default host release check also passes.
- `live-host-final.log`: the actual HostSession integration test passes both
  drag/drop and close-cancel cases on Chrome **152.0.7977.76**. It checks exact
  dropped text or absence of a drop, native dragend, borrowed Page survival,
  zero host leases/operations, and test-owned Context/target removal.
- `oracle.log` / `raw-mouse-oracle.json`: fresh pinned Playwright **1.58.2**
  observations for **22** raw Mouse cases, with owned Context cleanup.
  `live-mouse-final.log`: both ignored host tests pass with `drag-runtime`:
  all **22** exact DOM-event comparisons and host close during a 60-second
  click delay. The latter cancels promptly with TargetClosed, does not synthesize
  mouseup/click, and leaves no host leases/tasks; it does not wait out the delay.
- `deps-source.log`: default/source/runtime closures **234/235/235**, exactly
  one optional drag leaf. `deps-drag.log`: default/selected leaf **3/5** Pwright
  packages. `deps-host.log`: unchanged default host has no Node-API dependency;
  runtime JS has no driver/CDP/process imports.
- Addon SHA256 is unchanged:
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
  No commit or push. `targets-final.json` verifies the exact preflight target
  `6B27745B5D9E9CBACA4F82CEA913EFB7` and task fixture targets are absent;
  unrelated Chrome Pages and the browser process remain untouched.

The initial oracle command ran from the package-less workspace root and failed
before browser attachment; rerunning from `packages/pwright` used the same
script successfully. The new composition tests initially exposed a missing
test-only qualified error type, then an invalid test assumption that fanout can
be derived twice. Configuration intentionally remains single-install: the final
test checks late-extension rejection separately from extensions-before-derivation.
No production behavior was relaxed to make these tests pass.

## NODE-043: retained production Page drag source

The explicit `pwright-node-host/drag-source` feature supplies
`PageDragMoveSource`, implementing the Mouse dispatcher's `DragMoveSource` port.
It takes a new NODE-042 utility snapshot for each movement, bound to one Page
and the capture factory's exact native client. Construction and each start
validate the input binding; a replacement session cannot silently reuse the
source. It performs no world creation, subscription, browser command or task
spawn during construction/start. Native work begins only when its returned
`DragMove` is polled.

`CdpClient::retained_native_session(Arc<Self>)` is an optional concrete native
extension view. Real `CdpSession` returns its original Arc allocation, without
reconstructing the session or attaching again. Trait-only clients return `None`
unless explicitly implemented. The source checks client allocation identity
against its capture binding; equal native-ID strings are insufficient.

### Ownership and destruction

`DragMove::start_with_world_lifetime` retains a caller-provided
`DragWorldLifetime` owner through result, retirement and dispatcher quarantine,
until the entire move is dropped. Existing `start` callers keep the old manual
acknowledgment path. The host source's owner holds the immutable utility snapshot
and matches both client identity and unique-world ID before consulting its
native destruction receipt. Route replacement/crash, transport loss and owned
session detach are not world destruction.

When retirement observes an exact destroyed world, it drains any already-started
listener request before declaring that listener retired; it does not issue a
new cleanup command into the destroyed world. This proof can supersede the
listener request's result, but never its pending native ownership. Independent
live worlds still run normal cleanup. A source snapshot/result remains admitted
after successful retirement until the movement owner actually drops.

`PageDragSourceProfile` requires distinct source/movement scopes and explicit
1..=32 world/move ceilings. It admits the source owner, retained Arc wrapper and
temporary borrowed-world vector before allocation, and includes the bounded
`DragMove` owner requirements. Snapshot, listener/capture/data factories, native
provider futures, wire payloads and error allocation trees have separate owners
and must also be declared by the embedding host. Source clone admission is
shared; partial failure releases the reservation. No general RSS bound is claimed.

The default host still excludes the drag leaf: dependency checks show **234**
normal packages by default and **235** with `drag-source`, with exactly that
one extra crate. Default/Mouse-selected drag closures remain **3/5** Pwright
packages. `scripts/check-v2-drag-source.mjs` enforces this selection boundary.

This source is not yet installed by `HostSession` or the Node addon. Remaining
work is Page/session controller ownership, retaining/driving dispatcher quarantine
through host close, Mouse route selection, Keyboard Escape and public differential
verification. The Node addon/public Mouse baseline remains unchanged at **36/41**.
The movement still observes its prepared world list; upstream's non-stalling
world-loss handling and fresh post-movement Frame enumeration remain open too.

### NODE-043 verification

Evidence directory: `.work/node043-drag-source.cJWLq1/`.

- `unit-final.log`: **86** drag, **73** CDP and **117** selected-host release
  unit tests pass; the host run separately lists **7** ignored native tests.
  Four new movement regressions cover retained-owner drop timing, destruction
  during pending setup, foreign-client proof rejection and independent live-world
  cleanup. Host tests cover source bounds and the absent native view on fakes.
- `default-drag.log`: **74** default-feature drag unit tests pass.
  `clippy-final.log`: strict release all-target checks pass for the selected
  host and drag leaf. Workspace formatting/diff checks pass.
- `live-source-ready.log`: one expanded production-driver/native source scenario
  passes on the existing Chrome **152.0.7977.76**. It verifies original-Arc
  extraction, unpolled cancellation/overlap admission, exact delivered drag text,
  zero retained source/snapshot admission after move drop, fresh inclusion of a
  newly created child Frame, and exact child-world destruction while movement
  is pending. Retirement stays pending until the existing movement future is
  released, then cleans the live main world and retires the destroyed child.
  The test-owned Context is disposed and its target absence checked.
- `live-source.log` passed the earlier single-world drag case. The expanded
  attempt in `live-source-final.log` failed at the fixture-ready precondition:
  `Target.createTarget` can return before page scripts run. The final test waits
  for observed fixture readiness before ANY input, while driving topology events;
  no input sequence, outcome, timeout classification or application code was
  altered to mask that setup race.
- `live-mouse.log`: all **4** existing selected-Mouse native cases pass.
  `live-primitives.log`: all **10** existing native listener/capture/Page cases
  pass. These remain Rust/native checks, not paired public Node comparisons.
- `deps-source.log`: default/selected host **234/235**, only the drag leaf added.
  `deps-drag.log`: default/Mouse-selected leaf **3/5**. `deps-host.log` confirms
  no runtime JS driver/CDP/process imports in the unchanged default host.
- Addon SHA256 remains
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
  No commit or push. `targets-final.json` confirms the task's preflight target
  `6ED4901C185FB66FBD178C3D41E8B6B5` and fixture targets are absent; unrelated
  Chrome targets and the browser process were not closed/restarted.

## NODE-042: existing Page utility-world snapshots

`ChromiumFrameWorldAccess::existing_page_utility_worlds(page, maximum)` now
provides the immediate world selection needed by the future host drag source.
It reads the existing authoritative route publication under one lock: only the
requested Page's current utility routes, including local child Frames and OOPIF
sessions. Missing worlds are omitted without readiness waits or world creation;
an existing utility world does not require a corresponding main world. Frame
IDs are not silently converted to Pages. Crashed/closed Pages fail explicitly.

This differs intentionally from the existing paired-world accessor used by
Locator evaluation, which must wait for main AND utility worlds of one Frame.
Pinned Playwright 1.58.2 `crDragDrop.js` uses non-stalling existing-context
evaluation across Frames; reusing the paired wait would stall otherwise usable
drag movement on missing worlds. The new API is selection only: it does not yet
implement upstream's tolerance of worlds disappearing during listener evaluation,
nor upstream's fresh post-movement Frame enumeration.

### Admission and retained identity

- Maximum selected worlds is explicitly 1..=32. Exceeding it fails the whole
  batch, never truncates. All per-world permits are reserved before result-vector
  allocation and native-ID clones; partial admission failure rolls back.
- It shares the accessor's existing 8192-byte per-world allowance and capacity
  with pending and retained paired snapshots. The immutable batch does not expose
  vector draining/resizing, so callers cannot release its element permits while
  retaining uncharged vector capacity. Empty batches allocate no vector storage.
- Each utility route now retains its original `Arc<dyn CdpClient>` directly;
  snapshots clone that same client instead of creating a fresh session. Snapshot
  allowances cover escaped client/world references, while route native-ID and
  composition storage retain their existing owners. This does not add a drag,
  Mouse, executor or Node dependency to the composition.
- `is_current` checks observed Page ownership, connection epoch, unique world ID
  and client identity. An observed stale world stays stale, even if the same
  identity later reappears. It is NOT a native destruction acknowledgment.

When native retirement is explicitly selected, a separate
`ChromiumNativeUtilityWorldLifetime` records one exact utility world's lifetime.
Main-world destruction and foreign-session events cannot retire it. The existing
registry now represents paired receipts with `Some(main)` and utility-only
receipts with `None`, preserving the old two-world proof for paired access.
Acknowledged owned-session detach retires native wrappers without claiming the
browser world was destroyed. Lifetime clones retain their original snapshot
permit after the batch is dropped. No new polling task/subscription is installed.

### Remaining integration

The Node host still does not select the drag dispatcher. The production
`DragMoveSource` must retain these snapshots through listener cleanup/quarantine,
connect exact native destruction receipts, and account for child allocations.
The host also needs Page/session ownership and teardown plus Keyboard Escape
cancellation. No addon rebuild or new public differential result is claimed;
the existing public Mouse baseline remains **36/41**.

### NODE-042 verification

Evidence directory: `.work/node042-utility-snapshot.4q6lsY/`.

- **13** new utility snapshot regressions cover missing/mainless worlds, local
  children, OOPIF client identity, foreign Page exclusion, whole-batch overflow,
  shared paired admission/rollback, retained destruction proofs, stale routes,
  session acknowledgment versus destruction, crash and payload bounds.
- `composition-final.log`: **308** release unit tests pass, **15** live tests
  ignored by the ordinary run. `host-final.log`: **115** pass, **6** ignored.
- `clippy-final.log`: strict release all-target Clippy passes. Workspace format
  and diff-whitespace checks also pass.
- `live-worlds-final.log`: the expanded production-driver Chrome test passes.
  Both main-Frame and child-Frame utility snapshots perform real evaluations;
  their global writes are absent in the corresponding main worlds. Child removal
  invalidates the retained utility snapshot; the owned Context is disposed and
  snapshot bytes return to baseline. This is one multi-step native scenario,
  not thirteen native tests or public Node differential evidence.
- The initial `live-worlds.log` failed during attachment at the old **8-world**
  limit, before exercising the snapshot. The shared browser contained other
  task Pages. The live fixture now explicitly budgets **8 Frames / 16 worlds**
  and selects the Node host's main/Pwright utility event policy; it does not
  alter production host limits. Only this task's preflight Page was closed.
  Unrelated Pages and the existing Chrome process were retained.
- `deps-drag.log`: default/selected drag closures remain **3/5** Pwright packages.
  `deps-host.log`: **234** normal packages; no runtime JS driver/CDP/process imports.
- Addon SHA256 is unchanged:
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
  No commit/push. `targets-final.json` confirms absence of the owned preflight
  target `545AF9CA47FB280DF31A34B804F9A420` and fixture Page; the two other task
  Pages remain present.

## NODE-041: selected Mouse dispatcher and Page cleanup driver

The explicit `pwright-backend-cdp-drag/mouse-dispatch` feature now implements
`CdpMouseDispatcher` using the existing input, movement and Page drag owners.
`MouseDragDispatcher` is verified through the Rust input facade on real Chrome.
**12** new deterministic tests pass (**82** with the feature; **70** without).
Four new selected-dispatch native cases and the ten existing native primitive/
composition cases pass. The Node host still does not select this feature; the
public addon remains unchanged at **36/41** Mouse comparisons.

Source: `src/mouse.rs`, `src/mouse/tests.rs`, `tests/live_mouse.rs` within the
drag crate. Evidence: `.work/node041-mouse-dispatch.CV5zjF/`.

### Explicit selection and routing

The constructor binds one native client, one `DragPage`, one `DragMoveSource`,
explicit resources/scope and operation capacity. The input client must be the
same native object as the Page's drag binding, and the source must use the same
capture factory. Each dispatch checks the exact input client Arc again. This
requires host Page bindings to be reused, not reconstructed as unrelated client
objects for every call. No production consumer has been changed to select it.

`DragMoveSource` supplies bounded existing-world composition and fresh unpolled
children. It owns snapshot/lifetime allocation and navigation policy; no default
Frame discovery is hidden in the dispatcher. The native fixture supplies one
known main-frame world. The future host source must use current admitted Frame
snapshots and handle navigation/OOPIF replacement.

Branch behavior on first result poll:

- forClick movement always sends actual pointer movement, bypassing capture and
  Page drag-position updates, including while dragging.
- Ordinary movement updates the Page's last position. Active drag selects over;
  otherwise a non-left last button selects direct movement, and left selects
  listener/capture/movement composition.
- Actual pointer movement samples live buttons/modifiers/force after any delayed
  preparation. Position and last button remain captured in the request.
- Captured data selects enter, with modifiers freshly read after observation.
  Enter's first poll installs Page data before the completed capture is released.
- Active drag suppresses physical down and maps up to drop; inactive down/up
  remain direct. Independent logical Mouse state is still mutated by input.
- Wheel remains outside this dispatcher, as in the existing native Mouse leaf.

Keyboard Escape is **not yet connected**. The native cancellation case invokes
the explicit Rust Page cancel operation and must not be counted as Escape parity.

### Terminal retirement without losing native ownership

Construction returns the dispatcher and a unique, non-cloneable
`MouseDragRetirementDriver`. The Page owner must retain both. Dispatch slots are
preallocated; each admitted outer operation keeps its slot/storage through
result completion until drop or actual retirement.

If cleanup returns an error with a dirty movement still present, the outer
operation transfers that work and its memory lease into the Page's quarantine
**before** returning the terminal retirement error required by the Mouse
interface. Dropping a started, unretired outer operation also transfers its work;
unstarted or actually retired work can release normally. No native reply owner
is intentionally discarded as a cleanup acknowledgement.

While quarantine is nonempty, new dispatch and further result planning reject.
A separate quarantine bit remains set while the cleanup driver temporarily
takes a work item out of its slot, avoiding a false clean window during polling.
The driver polls children outside registry locks, fairly across slots. It keeps
dirty work after errors and retries only on a later explicit cleanup round.
There is no background task or automatic retry loop.

`stop()` prevents new result work but does not retire active outer invocations.
`is_drained()` means dispatch slots are released, **not that Page drag data is
empty**. The host must retire active calls, drain quarantine, cancel active Page
drag and only then close transport. Destruction-based resolution of uncertain
quarantine and host lifecycle wiring still need integration; dropping the Page
driver early is not native cleanup.

### Budgets and compatibility limits

Operation capacity is **1..=32**, with fixed registry/quarantine/driver/outer
owner/native-wrapper payloads charged before allocation. Retained request
type/pointer-type string capacities total at most **32 bytes**. Rejection uses
a zero-sized owner rather than allocating a rejected-operation box. Shared
Mouse readers retain their input admission until work releases them.

Source/movement/Page/data allocations keep their own budgets. Native client and
drag-event future allocation trees, wire replies/errors and allocator overhead
are still outside this outer measurement. This is not a complete RSS proof.
The current capture factory permits one interval at a time; overlapping capture
requests and high-count Mouse batches can hit explicit limits. No serialized
fallback or hidden retry is added to make those cases appear compatible. Their
full differential/concurrency coverage remains open.

Default drag dependency closure remains **3** Pwright packages. Selecting Mouse
policy adds only input and input-model, for **5** total. It does not install the
Mouse service crate, Locator, runtime, Node host or another executor. Existing
Mouse/native/runtime/host closures remain unchanged.

### NODE-041 evidence

- `unit-final.log`: **82** pass. New cases cover branch selection, physical down
  suppression with logical state preserved, forClick bypass/cancel position,
  delayed buttons/modifiers/force, late enter modifiers, stop/binding rejection,
  capacity, dropped native ownership and failed-cleanup quarantine transfer.
- `unit-default.log`: **70** pass; `clippy-default.log` and `clippy-final.log`
  pass strict release all-target checks for both feature profiles.
- `live-mouse-final.log`: **4/4** selected Mouse cases pass on Chrome
  **152.0.7977.76**: direct, drop, explicit Page cancel and click bypass. Drop and
  cancel verify native dragend; drop checks exact delivered text. Suppression
  asserts the original native mousedown count is one and an extra logical down
  does not increment it. Source count, Page state, registry cleanup, Context/
  target absence and zero resource leases are also checked. These are native
  Rust-input cases, not Node/FFI or paired upstream comparisons.
- `live-primitives.log`: prior **10/10** native listener/capture/composition/Page
  cases still pass. `mouse-regression.log`: adjacent **85/17/38** tests pass.
- `deps-drag.log` checks default/feature **3/5**; other proofs retain native
  Mouse/default Mouse/runtime Mouse **8/16/17**, host **234** normal packages
  with no runtime JS driver/CDP/process imports.
- The addon hash stays
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
  No commit or push was made. Preflight target
  `B6AA08E299B556E793DE57AB1D9B4D1C` and fixture targets are absent in
  `targets-final.json`; unrelated Chrome targets/processes were left alone.

Next connect current bounded Frame snapshots, exact Page client reuse, Escape
and shutdown/quarantine ownership in the host. Then build a fresh addon and run
the unchanged 41-case Mouse gate plus HTML drag/Frame/cancellation/Worker cases.
No public discrepancy is resolved until that artifact-specific evidence exists.

## NODE-040: Page drag state and native event owners

`DragPage` now owns persistent Page drag data and the last ordinary-move position.
`DragPageOperation` retains native enter/over/drop/cancel requests through their
actual receipts. **13** new deterministic tests pass (**70** drag tests total).
The native suite passes **10** scenarios: six listener cases and four composed
Page cases, including a new native cancellation path. Composed enter/over/drop
and cancel now use the Rust Page object rather than direct test commands. This
is still not installed as the Mouse dispatcher or Node host; public **36/41**
and the existing addon remain unchanged.

Source: `crates/pwright-backend-cdp-drag/src/page.rs` and `page/tests.rs`.
Evidence: `.work/node040-drag-page.CVqT1Q/`.

### Page identity and state transitions

`DragEventClient` extends the fixed interception binding, with a production
`CdpSession` implementation. Page construction requires the identical native
client Arc as its capture factory. Enter accepts only the exact shared data
record retained by a retired `DragMove` from that factory. A matching session
string from another capture/Page is insufficient. The record is shared, not
deep-cloned into a second persistent data tree.

Command construction admits storage/count but does not change state or send IO.
First polling enter installs Page data **before** sending dragEnter, matching
upstream's `_dragState` assignment before native dispatch. Over retains data and
updates the last ordinary-move position. Drop keeps state while its reply is
pending and clears it after observed success; a native error preserves it.
Cancel with no active drag returns false without a native command. Active cancel
sends the last ordinary-move position, empty items, operations mask **65535**,
and no modifiers property; successful native completion clears Page state.

The caller must call `record_position` when an ordinary move first polls,
including moves that select direct pointer input. A forClick bypass must **not**
update it. Enter deliberately does not rewind this position after a later move
has updated it. The forthcoming Mouse adapter still needs to enforce these
branches, consult `is_dragging()` for physical down suppression, and select over
or drop for active dragging. Logical Mouse state remains owned by input, not
duplicated here. Keyboard Escape must select cancel before native keydown.

### Cancellation and lifetime

An abandoned borrowed result future leaves the native owner intact. Explicit
retirement drops an unstarted plan without mutation/IO, or drains an already
started request. For drop, retirement does not run the abandoned result's
success continuation, so Page data remains. For cancel, successful native drain
still clears state: upstream `cancelDrag` has no inner progress race, unlike
drop. These rules follow pinned `crInput.js` / `crDragDrop.js` at Playwright
**1.58.2**; they are source-aligned unit coverage, not paired cancellation timing
conformance. Clearing follows the upstream shared-state behavior; overlapping
command/new-drag races still need their own differential verification.

Command retirement is not whole-Page drag cleanup: enter/over/error can leave
active data on the Page. The host must drain commands and cancel an active drag
before transport teardown, or independently confirm native Page destruction.
Dropping an undrained command poisons that Page against further command creation;
this contains misuse but does not perform native cleanup. A command's count and
memory lease remain held until its owner drops, even after result completion.

### Budgets and verification

Command capacity is explicit (**1..=32**). Page/fixed operation payloads and Arc
counters are charged before allocations/state mutation. Capture/data and shared
native state retain their existing admission. Injected command futures,
outgoing/reply/error JSON trees and allocator overhead need separate budgets.
The native adapter serializes borrowed generated data directly into its outgoing
JSON tree without first cloning all strings/vectors. This remains partial
retained-payload accounting, not a complete allocation-tree or RSS bound.

- `unit-final.log`: **70** pass, including first-poll enter state, unpolled
  cancellation, exact capture/client provenance, pending/successful/failed drop,
  different abandoned drop/cancel continuations, cancel payload selection,
  no-drag cancellation, operation capacity, dirty-drop poisoning and nonfinite
  coordinate rejection.
- `clippy-final.log`: strict release all-target checks pass. `clippy.log`
  records the earlier test mutex-lifetime diagnostic, fixed with lexical scope.
- `live-restored.log`: **10/10** Chrome **152.0.7977.76** scenarios pass. Native
  composed drop checks exact delivered text and actual dragend; native cancel
  checks no drop delivery and actual dragend. Both check Page state clears after
  replies, listener restoration, Context/target absence and zero resource leases.
  Main-frame-only cases do not prove OOPIF/navigation, real Mouse branch
  selection, Keyboard Escape or public Node conformance.
- `live.log` is a fixture setup failure, not a drag result: the temporary route
  had expired (HTTP 404). The test exited with owned Context cleanup. The same
  exact fixture directory/route was re-registered with serve-lib on verified
  Tailscale IP **100.78.97.63**, port **9384**, timeout **2h**, and the full
  MagicDNS fixture URL returned HTTP **200** before the successful run.
- Adjacent input/native/controlled Mouse tests pass **85/17/38**. Dependency
  closures stay **3/8/16/17**, host **234** normal packages; no new dependencies
  or runtime JS driver/CDP/process imports. Formatting and tracked diff checks
  pass. The addon SHA-256 remains
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
- `targets-final.json` confirms exact preflight target
  `79D880824C007CA16B333777281F1A9B` and fixture targets absent. No browser restart,
  unrelated target closure, commit or push occurred.

Next implement and select the actual `CdpMouseDispatcher`, connecting ordinary
move/forClick/down/up branches, live-state reads, bounded Frame snapshots and
Page-owned cleanup quarantine. Then connect Keyboard Escape and host teardown,
rebuild the addon and rerun public discrepancy gates. The five current Mouse
failures remain open until that public evidence exists.

## NODE-039: composed intercepted movement

`DragMove` now composes the temporary stages in Rust: prepare listeners in an
existing-world snapshot, enable capture, poll a lazy native movement callback,
observe/remove listeners, collect admitted data if any world expects drag, then
disable interception. Its result is `Option<AdmittedDragData>`, not a completed
Page drag gesture. **13** new deterministic tests pass (**57** total drag tests).
The native suite now passes **9** scenarios: the earlier six plus composed
ordinary movement, native dragging and prevented dragstart. Node/public Mouse
remains **36/41**; this helper is not yet a selected Mouse dispatcher.

Source: `crates/pwright-backend-cdp-drag/src/movement.rs` and `movement/tests.rs`.
Evidence: `.work/node039-drag-move.XAZj4a/`.

### Composition and ownership

Construction admits the outer owner/vector before allocation and creates fresh,
unpolled listener/capture children. Failed construction releases those children
without IO. The snapshot has an explicit maximum of **32** already selected
utility worlds. Setup and observation poll independent worlds fairly, without
waiting for the first pending world before starting the next. Completion flags
prevent re-polling a finished child's result. The caller still owns world
discovery, lifetime evidence and navigation/non-stalling policy; this snapshot
consumer neither creates worlds nor substitutes replacement contexts.

The movement callback is an injected, fresh lazy future. It is first polled only
after listener preparation and capture enable succeed. Its eventual Mouse adapter
must sample live buttons/modifiers at that point, retaining captured position
and last-button semantics. The current native test callback uses fixture-specific
button values, not the production `MouseStateReader` adapter.

The owner exposes caller-driven result/retirement polling and borrowed async
convenience methods. Borrowed child futures can be dropped after each poll because
their native/read progress already lives in the child owner. No task, executor,
timer or new dependency is installed. If observation expects no drag, it skips
waiting for a data event and disables interception. If data is expected but has
not arrived, result polling stays pending and can be cancelled by retirement.

### Failure and cancellation

Any result failure stops further action planning but retains all child owners.
Retirement drops an unstarted movement callback; a started callback is drained
before interception is disabled. Listener cleanup is still driven fairly while
other children are pending. A failed child's cleanup does not abandon other
worlds. Each retirement round observes each child's cleanup once, retains the
first error while finishing the other children, and returns it separately from
`is_retired()`. An explicit later retirement call can retry dirty children.
It does not repeatedly retry a failed child in the same polling round.

Destruction acknowledgements require exact native client Arc identity as well as
the unique world ID, or the exact capture session binding. Pending native work
must still be drained. Acknowledgement stops new action planning; the caller must
independently obtain actual destruction evidence. A matching string or Arc alone
is not proof. Drop is not cleanup; the eventual Page provider must retain or
quarantine a move whose cleanup remains uncertain. This helper does **not** yet
implement `MouseDispatchOperation`, whose terminal retirement-error contract
requires persistent uncertain children to have a separate Page owner.

### Accounting and evidence

The outer requirements charge `DragMove` and the bounded world-slot vector;
listener, capture and data factories retain their separate admissions. The
caller-supplied movement future and its allocation tree, shared client/native
lifetimes, CDP arguments/results/errors and allocator overhead require their own
budgets. Outer/child struct storage may be conservatively double-charged. This
remains partial retained-payload accounting, not a full allocation-tree/RSS proof.

- `unit-final.log`: **57** pass. New coverage includes stage ordering, any-world
  observation, parallel setup/cleanup, cancellation before setup/during setup/
  during movement/while waiting for data, failed setup/input/cleanup, explicit
  cleanup retry, failed construction and exact-world acknowledgement. The earlier
  `unit.log` failed one fixture configuration that cleared its own boolean reply;
  correcting that setup, not production behavior, fixed the test.
- `clippy-final.log`: strict release all-target checks pass.
- `live-final.log`: **9/9** scenarios pass on Chrome **152.0.7977.76**, including
  three composed cases. NativeDrag's callback triggers a real intercepted drag;
  the returned typed data is then sent by test scaffolding through
  dragEnter/dragOver/drop and exact delivered text is checked. All cases verify
  native listener restoration, Context/target absence and zero resource leases.
  The new cases use a single main-frame world; multi-Frame/OOPIF/navigation,
  live Mouse-state sampling and public Node conformance remain unverified.
- `mouse-regression.log`: input/native/controlled Mouse **85/17/38** pass.
  Dependency proofs remain drag/native Mouse/Mouse/runtime Mouse **3/8/16/17**;
  host remains **234** normal packages with no JS driver/CDP/process imports.
- `targets-final.json` records exact preflight target
  `F9A0BDAFA89C4FCF0E0175241AAF94CA` and raw-drag fixture targets absent after
  cleanup. The dedicated Chrome process and unrelated targets were left running.

Next implement the Page-scoped drag state and actual Mouse dispatcher: adapt
live-state movement, forClick bypass, down suppression, enter/over/drop/cancel,
Keyboard Escape and Page-owned uncertain cleanup. Then supply bounded current
Frame snapshots and select one provider per host Page before rebuilding and
rerunning the unchanged public gates. No public failure is closed by this helper.

## NODE-038: retained capture lifecycle

`DragCaptureFactory` / `DragCapture` now own one interception interval on a
fixed native session. This composes NODE-037 typed storage with a fresh event
subscription and retained `Input.setInterceptDrags` enable/disable receipts.
The leaf still has **3** normal Pwright packages and is not installed by the
Node host. **17** new deterministic tests pass (**44** drag tests total).
The existing **6** real Chrome cases now use this owner instead of direct
interception toggles/event collection. Public Mouse remains **36/41**, with no
addon rebuild or new public discrepancy claim.

Source: `crates/pwright-backend-cdp-drag/src/capture.rs` and `capture/tests.rs`.
Evidence: `.work/node038-drag-capture.EEDZn5/`.

### Binding and sequencing

`DragInterceptionClient` is an injected fixed connection/session binding. Its
production implementation uses the exact `CdpSession` for both subscriptions and
typed toggle parameters; it does not reconnect, migrate sessions or discover
targets. One factory must be installed per host Page/session. Cloned factories
share exclusive admission: a second capture rejects until the existing owner
is retired **and dropped**. Independent factories are not a global session lock;
the future Page composition must prevent duplicate installations.

`start()` admits the owner without subscription or native IO. First polling
`enable()` subscribes before polling the enable command. A fresh subscription
does not replay already queued events from earlier intervals. Factory-local
checked `u64` epochs are never reused; exhaustion rejects without IO. These are
ownership identifiers, **not wire event epochs**: CDP does not tag interceptions
with our interval. Late native events still require correct Page/input ordering;
matching session strings or local epochs alone cannot authenticate freshness.

`receive()` is caller-polled only when utility-world listeners expect drag. It
filters exact session/method and passes the first matching event through the
admitted typed data store. First matching data wins, like upstream's resolved
Promise; repeated receive returns a shared reference to that same record. Its
cursor is removed after capture, and later queued events do not replace it.
Lag, closure and data rejection are terminal read failures, not silent retries
that select a later event. There is a **32-event per poll** work bound followed
by a wake/yield under unrelated-event traffic. No executor, deadline or timer
is installed. The enclosing operation must supervise missing expected events.

### Cancellation and cleanup

The owner retains its native toggle future and owning event-read future across
abandoned borrowed `enable`, `receive` or `retire` futures. `retire()` is sticky:
it drops the subscription, drains any started enable request, then sends disable.
An enable error cannot prove interception was never enabled, so disable still
runs. A failed disable leaves the owner dirty/admitted and can be explicitly
retried; an abandoned pending disable resumes the same request, not a duplicate.

Successful disable is an interception cleanup receipt, **not active drag
cancellation**; native dragCancel/drop and Page drag state are separate work.
Dropping a dirty owner cannot perform async cleanup. It permanently poisons that
factory against reuse rather than pretending the session is clean. The host
must retain dirty owners for actual cleanup/quarantine; poisoning is misuse
containment, not a replacement for that integration. The factory's own memory
admission persists while the factory lives; abandoned owners no longer retain
their individual memory leases because their Rust allocations are gone.

An explicit destruction acknowledgement requires the identical client Arc and
no pending native receipt. A replacement binding with the same session string
is rejected. The caller must independently verify native session/target
destruction; passing a matching Arc is not itself destruction evidence.

### Accounting and verification

Capture requirements charge fixed factory/owner payloads, Arc counters, bounded
session storage and the concrete retained event-read wrapper future, measured
without creating a subscription. The NODE-037 store separately charges data.
Shared client state, connection subscriber/queue admission, provider toggle
future internals, JSON command/reply/error trees and allocator overhead need
their own budgets; this is not a complete allocation-tree or RSS proof. The
legacy broadcast receiver may retain source ring data after cursor removal;
reclaiming-fanout unit tests verify prompt cursor-specific reclamation.

- `unit-final.log`: **44** pass. New coverage includes subscription-before-enable,
  unpolled cancellation, first-event/session filtering, escaped data, pending
  enable/disable drain, failed enable/disable, abandoned receive, lag/closure,
  malformed data, subscription capacity failure, fresh intervals, dirty-drop
  poisoning, exact-binding/pending-receipt acknowledgement, bounded polling,
  unread cursor reclamation and epoch exhaustion.
- `clippy-final.log`: strict release all-target checks pass. `clippy.log` records
  three earlier collapsible-if diagnostics in the live test, subsequently fixed.
- `live.log`: **6/6** existing Chrome **152.0.7977.76** scenarios pass. Every case
  uses the production capture owner for interception enable/disable. NativeDrag
  receives actual data through it, sends dragEnter/over/drop, checks exact text,
  drops all record owners and observes the data lease released. Each case
  independently confirms native Context/target absence and zero resource leases.
  The test still dispatches movement/drop directly; it is not a Mouse provider.
- Adjacent input/native/controlled-Mouse tests pass **85/17/38**; drag/native
  Mouse/default-runtime closures remain **3/8/16/17** packages.
- `deps-host.log`: host remains **234** normal packages with no runtime JS
  driver/CDP/process-launch imports. Workspace formatting and tracked diff checks
  pass. The addon hash is unchanged at
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
  No commit or push is made this turn.
- `targets-final.json` confirms exact preflight target
  `B51448F4AC8787CE4BD1907D3A495DCC` and raw-drag fixture targets absent. Unrelated
  Chrome targets were not closed and the browser was not restarted.

Next compose the live-state move callback, bounded Frame-world snapshot,
Page drag state and result/retirement dispatcher. The host must own one shared
provider across calls and retire it before transport teardown. Capturing data
alone does not close the five public same-turn Mouse discrepancies.

## NODE-037: admitted intercepted data

`DragDataStore` now validates and retains an independent typed copy of
`Input.dragIntercepted` data. It remains in the optional **3-Pwright-package**
drag leaf and is not installed by the Node host. **14** new tests pass (**27**
total), and the six-case native listener suite passes with its native-drag case
now routing captured data through this store before dragEnter/over/drop. Public
Mouse remains the prior addon's **36/41**; no addon is rebuilt.

Evidence: `.work/node037-drag-data.flvmab/`; implementation:
`crates/pwright-backend-cdp-drag/src/data.rs`.

### Validation before retention

`DragDataProfile` declares item count, file count, aggregate UTF-8 text bytes and
maximum retained records. Arithmetic overflow rejects. MIME types, contents,
titles, base URLs and file paths all count toward the byte limit; count limits
also reject oversized arrays of empty strings. Paths remain opaque strings and
are never opened by this component.

`admit` checks method and exact session before inspecting data; unrelated events
return `None` without validating or retaining their trees. Expected session IDs
must be **1..=128 bytes**. Borrowed JSON is checked for the known event/data/item
shape, required strings, nullable optional strings, arrays and a signed-64-bit
operations mask. Unknown fields reject explicitly rather than silently disappear
in copying. No extra mask-bit restriction is invented. Array order, empty strings
and empty versus absent file arrays are preserved; optional null becomes `None`
as in the generated CDP type.

Validation creates no owned decoding, payload clone or heap-building error
message. Only after it succeeds does the store reserve count/byte admission and
copy known fields. Failed storage admission rolls back the count. The record
does **not** retain the original `CdpEvent` or JSON tree. It preserves its origin
session ID and exposes borrowed generated `DragData`.

### Ownership and accounting

`AdmittedDragData::clone` shares one immutable Arc allocation/admission. Escaped
clones keep record and store state admitted after original handle/store drop;
capacity returns only after the last clone drops. Record storage includes Arc
payload/control counters, vector elements, maximum string bytes and bounded
session copy. Tests compare requested capacities against the declared lease.
This is a requested retained-payload bound, **not allocator rounding/headers or
process RSS**.

Source connection JSON/event buffers, caller-created deep clones, serialization
copies and pending native request buffers need their own admission. The live test
borrows the record during dispatch; it does not certify the outgoing-command
allocation tree. This store does not subscribe, manage capture epochs, reject
duplicate/stale interceptions or own Page drag state. Session comparison is
within the caller's connection; the future provider must enforce connection
generation and Page identity. A matching session string is not freshness proof.

### NODE-037 verification

- `unit-final.log`: **27** pass, including **14** data tests for full-field copy,
  shared retention, count release, malformed/unknown fields, aggregate UTF-8
  boundaries, empty/null distinctions, rollback, overflow and payload size.
- `clippy-final.log`: strict release all-target checks pass.
- `live-final.log`: all **6** listener scenarios pass on Chrome **152.0.7977.76**.
  NativeDrag now admits actual captured data, checks its origin, sends it through
  dragEnter/over/drop, verifies exact text and observes record count return to
  zero. Each scenario still checks listener restoration and native Context/target
  absence. This is one native data-flow case, not six new data comparisons.
- `deps-*.log`: drag **3**, native Mouse **8**, Mouse default/runtime **16/17**
  Pwright packages and host **234** normal packages. No dependency was added.
- `mouse-regression.log`: adjacent input/native/controlled-route release tests
  pass **85/17/38**. Workspace formatting and `git diff --check` pass.
- `targets-final.json`: the exact task preflight target and all raw-drag fixture
  targets are absent after cleanup; unrelated browser targets were left alone.
- The addon SHA-256 remains
  `e65e6d85a6f91e2d07ccde82cf27cf7472e2b15b330b534aa9cb79d27d17b246`.
  This turn adds no public compatibility result and makes no commit or push.

## NODE-036 status

`pwright-backend-cdp-drag` implements a caller-owned **single utility-world drag
observer**, not the complete Mouse drag provider. It adds no dependency to Mouse,
the Node host or the production addon. **13** deterministic tests and **6** real
Chrome cases pass. Public Mouse remains the previous addon's **36/41**; no addon
is rebuilt and no public discrepancy is claimed resolved by this checkpoint.

Evidence: `.work/node036-drag-listeners.4cD9j7/`.
Source: `crates/pwright-backend-cdp-drag/src/{lib.rs,setup.js,cleanup.js}`.
The scripts are Rust-owned browser injection payloads, not Node-side scheduling
or automation. The Rust leaf installs no host timer, background task, browser
process, frame discovery, event subscription or input dispatcher.

## Upstream semantics and implemented behavior

Pinned source is `playwright-core@1.58.2/lib/server/chromium/crDragDrop.js`,
particularly `DragManager.interceptDragCausedByMove`. For an ordinary left-held
move with no active drag, upstream installs utility-world listeners, subscribes
to `Input.dragIntercepted`, enables interception, sends native movement, reads
whether dragstart was prevented, removes listeners and disables interception.
It then retains drag data and sends dragEnter if a drag actually started.
Click-bypass moves do not follow this preparation path.

The new observer implements only the listener portion. A one-shot mousemove
listener installs a one-shot capturing dragstart listener. A browser-owned
zero-delay task observes the final `defaultPrevented` value after event handlers
have run. This is browser event ordering, **not an artificial host delay** added
to make the five failing same-turn Mouse cases match. The observer never sends
native pointer or drag events by itself.

Each observer owns a distinct token in a utility-world registry. At most **32**
entries may exist in that world's registry, including across factories. Cleanup
removes only that token's listeners, clears its timer, settles any pending
observation and releases the captured event. The registry property is removed
only when empty and still identical to that registry, so delayed cleanup cannot
delete a replacement registry.

## Admission, results and retirement

`DragListenerFactory::new` requires explicit production resources, capacity
**1..=32**, a resource scope and a caller-supplied **1..=64-byte ASCII namespace**.
Namespaces should be unique among live factories in a utility world. Tokens use
a checked monotonic factory-local counter; IDs are not reused when a slot is
reclaimed, and exhaustion rejects without IO. There is no global ID generator.

`start` validates an exact existing unique context ID (**1..=128 bytes**) and
reserves a listener slot/lease before copying IDs or allocating the native
wrapper. It does not create a world or send CDP commands. The caller supplies the
already selected utility-world identity; this API alone does not authenticate
that the context is a utility world or belongs to the intended Page.

`prepare()` first-polls the retained setup RPC. `observe()` then awaits the
browser observation while removing listeners and returns its boolean outcome.
`retire()` without observation removes listeners without planning another input
action. Cancellation during setup drains the already-started setup RPC before
cleaning its potentially installed token. Cancellation during observation drains
the already-started cleanup RPC, not a duplicate request. Borrowed prepare,
observe or retirement futures may be abandoned while the owner retains work;
once retirement has been polled, prepare/observe cannot resume action work.

An ambiguous setup error still requires cleanup: a failed reply does not prove
the script never ran. In contrast, a successful **explicit false** setup reply
means a collision/capacity rejection with no installation. That owner is already
retired and **must not clean the token**, which may belong to another observer.
Both a unit test and a real same-world namespace-collision test pin this case.

Cleanup failure or a malformed cleanup reply leaves `is_retired() == false` and
the owner admitted. A later explicit `retire()` can retry; this is not a silent
automatic retry or a successful retirement receipt. Independently confirmed
destruction of the exact unique world may be acknowledged only with no retained
native call. The acknowledgement accepts caller-supplied authority; matching a
string is not itself destruction evidence. Losing a transport or observing a
reused numeric context is not sufficient.

Dropping the owner is not native cleanup proof. Owners retain their factory and
both admission leases even if the factory handle has escaped/dropped; all leases
are released when retired owners are finally dropped. The eventual Mouse
provider must keep uncertain listener owners in a Page-owned cleanup/quarantine
collection instead of turning their failed cleanup into an unqualified terminal
`MouseDispatchOperation` retirement acknowledgement.

### Partial storage accounting

The declared requirements include fixed factory/observer payloads, namespace,
context/token storage, the concrete retained wrapper future and outer argument
vector. This is **not a complete allocation-tree or RSS bound**. JSON map-node
layout, allocator overhead, shared client/world state, injected CDP future
internals, result/error trees and browser-side engine allocations remain outside
the measurement. Browser entries/listeners/timers have explicit count limits,
but their engine byte size is not measured. The observer does not budget retained
drag data; NODE-037's separate store now provides that admission.

## Verification

- `unit-final.log`: **13** pass. Includes admission/ID exhaustion, unpolled cancel,
  setup drain, abandoned observation, separate cleanup failures, malformed
  envelopes, exact-world acknowledgement checks and collision/no-install behavior.
- `clippy-final.log`: strict release all-target checks pass.
- `mouse-regression.log`: unchanged input **85**, native Mouse **17** and
  route/runtime **38** tests pass; formatting and tracked diff checks also pass.
- `live-final.log`: **6/6** Chrome **152.0.7977.76** scenarios pass inside one
  ignored test: no drag, native draggable, prevented dragstart, retire before
  movement, two simultaneous owners, and a duplicate-namespace/token collision.
  Each scenario owns its own Context/target and verifies their absence after
  disposal. Native DOMDebugger listener counts return to the baseline, not just
  an empty bookkeeping flag. Resource leases are zero after owner teardown.
- In the native draggable case, trusted native mouse input causes
  `Input.dragIntercepted`; the test observes the text payload and sends native
  dragEnter/dragOver/drop to verify exact delivered data. These direct commands
  are diagnostic scaffolding, **not a production drag dispatcher**.
- `live-first.log` records the earlier five cases before adding collision
  coverage. `live-initial.log` is a compile failure, not a browser outcome.
- `scripts/check-v2-drag-backend.mjs` proves exactly **3 Pwright packages**:
  this leaf, `pwright-cdp`, `pwright-core`. Existing Mouse default/runtime
  closures remain **16/17**, native Mouse **8**, host **234** normal packages.

Fixture: `tests/fixtures/raw-drag.html`, served through the existing serve-lib
route `/pwright-mouse-concurrency/raw-drag.html` on
`macbookpro-yuacx.tail8f3b66.ts.net:9384`. Preflight used dedicated Chrome/Pwright
with task-local state and loopback CDP 9222, never the user's Chrome profile.
`targets-final.json` confirms preflight target
`50B3875ECEB7550C137CEE42BE1F202F` and drag fixture targets absent after cleanup.
Chrome was not restarted and unrelated Pages were not selected for close.

These cases validate a single main-frame utility world and are not paired public
Playwright comparisons, multi-frame/OOPIF coverage, drag payload limits or Node
Worker/host-close conformance. The production addon remains `e65e6d85...17b246`.

## Next implementation requirements

1. Compose a bounded snapshot of existing per-Frame utility worlds, respecting
   navigation/replacement and upstream non-stalling behavior. Do not redirect
   old-world cleanup into a newly created world.
2. Typed data admission (NODE-037), capture lifecycle (NODE-038), and composed
   listener/capture/lazy movement (NODE-039) are implemented. Adapt the callback
   to live Mouse state and complete interval/navigation supervision.
   The currently selected Frame-world pre-admission policy retains non-context
   events, including Input events; an additional filter change is not yet shown
   necessary. Event identity, lag and buffer ownership still require integration.
3. NODE-041 implements opt-in Mouse branching, physical down suppression and
   forClick bypass over NODE-040 Page state. Connect and verify Keyboard Escape.
4. Select the provider through native/Page bindings from NODE-035 in the host,
   retaining the NODE-041 cleanup driver and uncertain work after call results.
5. Install one correctly owned provider per host Page across per-call routes;
   shut it down before transport teardown. Then rebuild the addon and rerun the
   unchanged 41-case public Mouse gate plus real HTML drag, navigation, multi-Frame,
   failure/cancellation and Worker teardown cases. Keep the five current failures,
   NODE-013 attachment and native reload-close issues open until their own gates pass.
