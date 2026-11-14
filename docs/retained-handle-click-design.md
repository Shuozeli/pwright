<!-- agent-updated: 2026-10-11T22:41:54Z -->

# Retained ElementHandle click: composable design and implementation status

Status: implemented in source and in an installed opt-in ABI108 wheel as a
top-level backend, host, PyO3 and async/sync Python path. The post-repair wheel
has official Chromium click evidence, including two paired detached-handle
passes, but the path is not production-ready: broader child-Frame geometry, modifier
rollback on cancellation and broader lifetime stress remain. A newer complete
wheel now passes both unchanged official timeout-detail cases 2/2, with
source-bound provenance and strict candidate credit. Fresh oracle reruns are
currently blocked at CDP setup by an extension service worker, though the
earlier full70 oracle run passed both cases.
The earlier frozen ABI108 reload wheel lacked `ElementHandle.click`; its
missing-method failures are historical, not the current wheel result. The
post-repair full 70-case click report is 65 passed / four failed / one
fail-closed report error.
The corrected upstream manifest tags nine exact `element-handle.click` cases,
including iframe, detachment, force, hit-target and device-scale behavior.

## Mutable page `Node` global (2026-10-11)

The pinned Python `test_click_with_node_removed` case exposed a second
main-world dependency after the shared selector runtime stopped reading
`Node.ELEMENT_NODE`: the opt-in retained ElementHandle click actionability and
route-check scripts still read that mutable page global. Both scripts now use
a local numeric `ELEMENT_NODE = 1` constant. A feature-gated source invariant
covers top-level and one-link child paths, including force route checking;
the complete compiled retained-click release subset passes 21/21.
This is source-only until a new wheel is built and the unchanged official
case reruns; no Python or Core conformance row is credited by this edit.

## One-link child-Frame source checkpoint (2026-10-11)

The opt-in retained-click source now admits only a direct, same-session child
of the owning Page's main Frame. The host keeps the original `PendingHandle`,
its source client and unique creation world, plus current child and parent
world snapshots. The backend checks the exact child/parent edge, translates
the target's Frame-local CSS point through the iframe **content** quad, and
checks the iframe owner is the parent hit target before Page-owned pointer
dispatch. After mouse movement it rechecks the exact native Element and
geometry before `mousePressed`. Invalid quads, replaced edges and parent
overlays fail without pressing. The default feature path is unchanged.

This is a source-only repair, not an updated wheel or official iframe pass.
The opt-in backend release check and four focused fake-CDP child-click tests
pass; the opt-in host release check passes. A complete fake `HostSession`
handle/world routing test has not been assembled, and no live CDP/browser
test ran for this checkpoint. Nested Frames, OOPIFs and cross-origin parent
hit checks remain unsupported; the source rejects them rather than claiming
coordinate parity. No strict-ledger row is promoted by these gates.

## Full-profile wheel baseline and detached-target repair (2026-10-10)

The frozen **pre-repair** current-source wheel at
`.work/official-conformance/builds/python-retained-click-source-20261010.ra00oP/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `79f4e2eaec461dd4dd8f492d0ad9fc6c472825fbb989b6938806cb122e6f67c4`)
was built with the declared full feature union plus opt-in
`element-handle-click`; a second build had identical normalized payload.
Installed Python 3.9.6 and 3.12.14 both report ABI 108, 97 selected runtime
features and a native retained-click method. Each passes 4/4 facade tests and
422/422 package units with one explicitly deselected default-profile-only
ABI-107 assertion; unfiltered runs each have that one known failure. The
release PyO3 option parser passes 3/3 after using the installed Command Line
Tools Python link path. These are installation/unit gates, not live-browser or
official upstream cases.

After freezing that wheel, source review identified a definite error
classification mismatch: ordinary exact Element detachment had been folded
into `TargetClosed`, which the host renders as Page/context/browser closure.
The opt-in backend now emits `FailureKind::Backend` with the official
`Element is not attached to the DOM` message for DOM detach, while
Frame/world retirement remains `TargetClosed`. Only this typed detached
status gets exact-message treatment in the opt-in host; generic Backend
mapping is unchanged. Feature-on backend release tests pass 132/132, and
focused host release tests pass 4/4. This repair **postdates the wheel above**.
An independently built **post-repair** full-profile wheel at
`.work/official-conformance/builds/python-retained-click-detachedfix-20261010.Zw4S2S/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `3a820d641bf4ffd41b1fb9386f6203a639207a38b3ee9ac34a83f062c0e86258`)
now passes installed-wheel gates on Python 3.9.6 and 3.12.14. Both load the
same native `.so` (SHA-256
`e597806f4c505b524ca1d7f439b175cbc113dcb28f4c8eb44dada70b0cf20482`),
report ABI 108 and 97 selected features including `element-handle-click`,
expose the native method, pass 4/4 facade tests, and pass 422 package units
with one known default-profile-only assertion explicitly deselected. Build
source hashes stayed frozen, and a second wheel has the same normalized
payload apart from CycloneDX identity fields. The unchanged pinned official
two-case detached-handle Chromium selection subsequently passed 2/2 in both
candidate and independent oracle lanes using this exact wheel. Both schema-5
source-bound sidecars verify, and the candidate strict 6,526-row successor
changes only those two `not-run` rows to `passed` (SHA-256
`5015b54bd58c16f0f345af5ae6ef034f6999194031471b1c2e7fbe60941677d6`).
See `docs/python-official-conformance.md` for the reports and hashes. This
proves these two top-level Chrome cases only; child-Frame geometry, lifetime
stress and the rest of the upstream corpus remain open.
At this older wheel checkpoint, stable-position and hit-target timeout
diagnostics remained open; the newer repair and official run below supersede
that source-only status.

The complete post-repair 70-case Chromium click report now records 65 passes,
four failures and one fail-closed report error. Two detached-handle cases pass
in both candidate and oracle lanes; child-Frame retained click remains an
explicit unsupported-geometry failure, and the device-scale-factor case's
duplicate isolated JUnit records leave the full-file row fail-closed. A later
raw-preserving diagnostic identifies the body failure as the same explicit
child-Frame guard, before any pointer dispatch, not DPR arithmetic. The body
failure skips its explicit `context.close()`; subsequent Browser and
environment teardown expose a sealed registry and residual leases/scopes.
The first sealing trigger remains unproven, so that cleanup path needs its own
regression rather than reclassifying the official row.
The original-fixture favicon 404 is shared with the oracle. Exact reports,
source hashes and ledger treatment are in `docs/python-official-conformance.md`.

The pinned `test_click.py` stable-position and hit-target timeout assertions
require the waiting/retry phrases and, for hit-target, the exact bounded
`<div id="blocker"></div> intercepts pointer events` preview. Source audit
locates the loss before Python: retained click writes only its latest status
to a service-wide progress slot, cancellation clears that slot, and generic
host timeout mapping discards the detail. The narrow repair is now
**source-implemented**: a fixed eight-observation ring belongs to each
opt-in click invocation, and only a validated 24-byte tag / 64-byte ID
blocker preview enters its log. A ready target clears old waiting
observations. After successful owned-press cancellation, only a
deadline-expired invocation with recorded progress maps to a distinct bounded
host message preserving `PWRIGHT_TIMEOUT`; empty-progress and cleanup-failure
cases keep their existing classifications. Default Locator/v1 paths remain
unchanged. Focused concurrency, sanitization, cleanup-order and error-code
tests pass; feature-on backend release tests pass 136/136, default backend
123/123, host release 643 passed / 14 ignored, strict release Clippy and
Python-addon release check pass. A later full-profile wheel (SHA-256
`3d6f18613e34234d22a6cc2e1b2f10228ecd3f195a72d1e41a51c8a6d54c8de8`)
reproduces its normalized payload, loads ABI 108 and 97 selected features,
and passes 4/4 facade plus 422 package units with one old default-profile
assertion excluded. It passes the two unchanged pinned timeout tests 2/2 in
the candidate lane; source-bound JUnit SHA-256 is
`8129b8012671ea788d0a8a742f50122ec3764838b945c474c406a176af191e78`.
The 6,526-row strict candidate successor is now **1,374 passed / five failed /
one error / 11 skipped / 5,135 not run** (SHA-256
`5dff8e00888dad238e40fb4167a1a8a3976111ad197a6450b89d8e49b652b9a1`).
The earlier independent oracle full70 passed both exact IDs; a fresh two-case
oracle retry failed before test bodies on a Chrome-extension service worker,
so it is diagnostic and not a contradictory timeout assertion. Neither the
mixed-build ledger nor two-case browser result is a full-corpus verification.

## Implemented source slices (2026-10-10)

`pwright-backend-cdp-locator` now has a `retained-element-click` feature and a
top-level-only `click_retained_top_level_element` entry point. It passes the
original CDP object ID into its unique execution context, rechecks that same
object after navigation barriers, and shares v2 actionability plus the pointer
and navigation transaction. It never re-queries a selector. Child Frame/OOPIF
routes are rejected rather than misclicked; the frame-to-top-level CSS geometry
step below is still open. The host must retain the handle lease, apply a
bounded runtime deadline, and invoke cancellation cleanup on timeout or
caller cancellation. A review found that `trial=True` still dispatched
pointer input; the backend now returns after exact-element actionability and
before navigation preparation or input. The current guarded feature-on backend
release suite passes **132/132**; its focused retained tests pass **9/9**.
The current default-feature release suite also passes **123/123**; neither
local suite exercises installed-wheel or official browser behavior.

The subsequent opt-in `element-handle-click` host feature retains the original
`PendingHandle`, validates owner and native identity, selects the existing
Page input route, and dispatches through the backend's controlled command and
cancellation path. The PyO3 method converts bounded `ClickOptionsV2`; thin
async/sync facades expose `ElementHandle.click`. Child/OOPIF Frames are
deliberately rejected because their frame-local coordinates cannot yet be
safely sent to Page input. At that source checkpoint, the frozen ABI108 v2
wheel and official browser results were unchanged. The later installed-wheel
gate is recorded above; official browser verification remains separate.
The new async/sync façade tests pass 4/4 in a read-only source overlay over
the frozen ABI108 v2 environment, using a fake native session. Focused
feature-on release tests pass 2/2 in the host and 3/3 in the PyO3 option
parser (using the frozen Python 3.12 interpreter for the latter). These
checks confirm types, options and forwarding only; they do not exercise a
live native click, timeout cleanup, iframe geometry or Chrome.
The earlier host source passed strict release Clippy; the new world/press
changes have not yet rerun that gate. Addon strict Clippy stops at an
unmodified `locator_read` eight-argument lint; allowing only that existing
lint checks the addon successfully. Focused typechecking of the new Python
tests passes; whole-file typechecking retains three pre-existing Page.click
diagnostics. Do not report either as a clean whole-addon/whole-file gate.
The earlier source-world ownership audit found a concrete missing guard. The
current host now retains a `ChromiumFrameWorldSnapshot` with the source,
original Handle and tracked read admission through cancellation. Its check
compares Frame/Page, numeric and unique main-world identity, client and
observed topology epoch. The backend requires that guard on each retry,
after navigation preparation/evaluation and after pointer movement before
each press. Its opt-in checked mouse-down additionally validates **after**
acquiring the Page input dispatch lock, closing a queued-lock navigation
window before mutating or sending `mousePressed`; ordinary input is unchanged.
It also re-evaluates the exact original object after hover and retries a
moved point without pressing. Eight independent world-snapshot
slots are budgeted for opt-in retained clicks. Focused host release tests
pass 2/2, host and Python-addon release checks compile, and the input crate's
owned-press tests pass 7/7; queued-lock retirement and current-world press
regressions pass 2/2. These are source/unit gates only. There is not
yet a real Chrome click, installed wheel, or official nine-case rerun.

The new Page-bound press token releases a button on cancellation only if the
same invocation still owns that button; a later same-button action invalidates
the claim and other buttons are left alone. It does **not** roll back
interleaved Page keyboard modifiers if cancellation drops the action before
its normal `finish()`, and raw Mouse actions intentionally retain their own
concurrency semantics. Multi-step pointer movement can also send a later
`mouseMoved` after an observed world replacement before its post-move guard,
though the checked-down gate prevents a subsequent press. These require
explicit lifecycle validation before a production-readiness claim.

## Creation-world gap found and source repair

- `crates/pwright-node-host/src/handles/pending.rs:117-123,270-289`
  (`ReadAdmission`, `admit_tracked_read`) admits a call, tracked work and a remote-native-handle
  resource lease. None is a world/epoch permit. Likewise, the retained source
  descriptor explicitly is not a native-use or route-lease pin
  (`crates/pwright-composition-chromium-handle-evaluation/src/retained_reference.rs:5-15`).
- Before this repair, `element_handle_click.rs` validated the retained source
  once and `click_retained_top_level_element` could await retries and mouse
  movement without a world check. The current `ClickOwner` and mandatory
  backend validation callback close that **observed-state** gap; they do not
  create an atomic browser-document lock. The backend also rechecks the exact
  object in its original unique context after movement and validates under
  the Page input lock immediately before pressing.
- The retained file-input implementation already demonstrates the needed
  ownership pattern: `RouteOwner` holds a
  `ChromiumFrameWorldSnapshot` alongside the source and Handle, and
  `check_world` compares Page, Frame, main-world IDs and client, then calls
  `is_current()` (`crates/pwright-node-host/src/handles/file_input.rs:323-349,380-439`).
  `ChromiumFrameWorldSnapshot::is_current` also compares topology epoch and
  utility-world identity (`crates/pwright-composition-chromium/src/frame_world_access.rs:408-469`).
  It is a bounded, observed-world guard, **not** an atomic browser-document
  lock; checks must be repeated across asynchronous boundaries.

Remaining proof: add deterministic host-level snapshot replacement/retirement,
capacity-release and timeout/caller-cancellation tests. The focused backend
test already proves an observed retirement after mouse movement produces no
`mouseDown`; the input tests prove the owned-press token's local races and
queued-lock world retirement. A late browser navigation between locked
validation and CDP dispatch remains possible, so no atomic lease is claimed.
Modifier rollback, raw-Mouse overlap, per-step move validation and child-Frame
geometry remain separate implementation gaps. These gates precede live Chrome and
official upstream pass credit.

## Boundary and invariants

- Keep the backend's opt-in `retained-element-click` feature and the matching
  opt-in `element-handle-click` composition in `pwright-node-host` and
  `pwright-python-addon`; add it to a future full Python wheel only after
  validation. It
  must not pull in screenshot or touchscreen capabilities, alter default/v1
  wire schemas, or create a second browser lifecycle.
- The native input is the existing retained `PendingHandle`, not a selector or
  element preview. Validate that it is live, an Element, and owned by the
  expected Page/Frame. A detached, disposed, replaced or retired target fails;
  it must never be reacquired by selector.
- Retain a bounded snapshot of the original creation-world pair until the
  click and its cleanup settle, revalidating it at every specified boundary;
  fail closed after observed retirement. The existing Handle read admission
  and Page/Frame input-route lease do not establish this guarantee.
- Share the existing locator click input transaction, navigation barrier,
  cancellation cleanup and `ClickOptionsV2` semantics. A DOM `.click()` or a
  host-only copy of mouse dispatch is not a compatible substitute.
- The pointer point must be expressed in top-level viewport CSS pixels before
  Page input dispatch. Frame-local points from retained evaluation cannot be
  sent directly to Page input, and device scale must not be multiplied in.

## Proposed composition

1. Extract the selector-independent part of
   `pwright-backend-cdp-locator`'s v2 click into a feature-gated executor. Its
   typed input is a finite top-level CSS point, bounded `ClickOptionsV2`,
   Page client/input coordinator, and navigation observer. The existing
   locator path obtains a point by selector and calls the same executor.
2. Add an exact-handle actionability evaluator using the borrowed retained
   object in its owning Frame. Reuse the locator status mapping where possible,
   but never re-query a matching DOM node. Preserve one deadline for
   actionability, pointer input and optional navigation wait. `trial` runs
   actionability only; cancellation and any partial mouse-down must release
   input state.
3. Translate the Frame-local CSS point through a bounded current parent chain.
   For each parent, use CDP's frame-owner and box-model geometry in the parent
   session, scroll the owner into view when needed, and recompute geometry.
   Use the iframe **content** quad, not its border origin, so border/padding
   and supported affine transforms are handled. Reject malformed, non-finite
   or unsupported perspective geometry instead of misclicking. Revalidate the
   target route immediately before dispatch. This helper must work for OOPIF
   and must not depend on the optional screenshot leaf.
4. The host now keeps the bounded handle lease and creation-world snapshot
   owner, invokes the guarded backend capability, and holds the Page-bound
   press token for cancellation. PyO3 checks `NativeHandle.read_frame()` and
   forwards the existing options. Thin async/sync Python facades expose the
   official signature and use the Page's effective timeout.

The existing host `element_handle_tap` proves exact-handle evaluation is
possible, but its Frame-local coordinate and touchscreen dispatch are not a
mouse-click implementation. The screenshot frame translator is a geometry
precedent, not a drop-in dependency: it currently uses a border-based origin.

## Verification before pass credit

- Focused fake-CDP tests: retained identity across DOM replacement;
  creation main/utility-world replacement and topology epoch change at each
  retry/barrier/pointer checkpoint; disposed/detached/retired failures;
  snapshot-capacity exhaustion and release on success, timeout and caller
  cancellation; force, obscured, stability, trial, modifiers, no-wait-after,
  and failed mouse-up cleanup. Verify no `mouse_down` follows an observed
  source-world replacement.
- Frame geometry tests: border/padding, nested same-process and cross-origin
  OOPIF, parent scrolling/overlay, CSS scale/rotation, malformed quad, and
  device-scale factor 2 with CSS-pixel dispatch.
- Release feature/dependency proof, Python 3.9 and 3.12 installed-wheel units,
  a real dedicated-Chrome iframe click, and then the exact nine tagged
  upstream official cases in independently verified candidate/oracle lanes.
  Do not promote any strict outcome from source-only or synthetic tests.

The frame translation above remains a separate proposed boundary. The source
path is top-level-only. Next are host-level world/cancel race proofs, modifier
rollback design, a current-source isolated Python 3.9/3.12 wheel build, a
real dedicated-Chrome top-level proof, and bounded Frame geometry before
iframe pass credit. No official row has changed because of this source-only
work.
