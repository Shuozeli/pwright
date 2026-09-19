<!-- agent-updated: 2026-09-19T21:23:32Z -->

NODE-191 adds opt-in `retained-serialized-payload-mutation` and combined native
artifacts. The internal coordinator consumes the Rust-owned serialized graph
and retained prevalidated INPUT in one non-retargeting Rust/CDP operation,
performs the pinned `File`/`DataTransfer` plus `input`/`change` mutation, waits
for group cleanup, then terminally removes the same FFI ticket. It does not use
`DOM.setFileInputFiles`, add a generic evaluator or expose public
`ElementHandle.setInputFiles`.

Both rebuilt profiles pass 392 offline retained-input/oracle cases, 167 package
cases, all 42 ordinary real-Chrome raw binding cases and a separately enabled 50 MiB
minus one byte maximum-payload gate, plus focused Rust, Worker, type,
dependency, formatting and strict Clippy gates. The direct Playwright 1.58.2
matrix covers empty/multiple files, events, listener exceptions, late type
change, cancellation and lifecycle races. It records that Playwright resolves
post-instrumentation main-document navigation while Pwright rejects fail-closed;
the channel-stripped `lastModifiedMs` remains a second explicit discrepancy.
See the [canonical
NODE-191 contract](../../docs/knowledge/pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190 adds opt-in `retained-serialized-payload-import` and combined native
artifacts. After NODE-189 performs JavaScript-compatible serialization, the
selected host reserves one bounded 96 MiB Rust owner before the addon copies the
graph. Success releases all eleven direct V8 roots and retains a move-once Rust
value on the same validated-target ticket. This remains internal transport
staging: it sends no CDP command, mutates no INPUT, emits no DOM event and does
not expose public `ElementHandle.setInputFiles`.

Both rebuilt profiles pass the 482-case extended offline matrix, 166 package
cases, 36 focused serializer/dispatch cases, 3 Rust owner cases, Worker, type,
exact boundary, formatting and strict Clippy gates, plus both complete 34/34
real-Chrome raw binding lanes. See the [canonical NODE-190 contract](../../docs/knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189 adds opt-in `retained-call-argument-serialization` and combined native
artifacts. The internal coordinator can now preserve one validated target from
instrumentation through Playwright-compatible value serialization while Rust
owns session/phase/root authority. It stops before CDP mutation, events,
terminal error transport and public `ElementHandle.setInputFiles`; normal users
should not treat this internal profile as upload API completion.

The selected checkpoint passes 398/398 offline file-input cases, 165/165
package cases, both 34/34 focused artifact lanes and both 32/32 real-Chrome
binding lanes, plus Worker, type, boundary, formatting and strict Clippy gates.

NODE-188: The `retained-input-instrumentation` and combined artifacts extend the
internal path through Playwright's awaited `onBeforeInputAction` position. Rust
gates `Prevalidated -> WaitingInputAction -> Instrumented`; Node awaits a frozen
internal port under the existing deadline with the original receiver and
retained `call@N`. The production port is an awaited no-op until optional
tracing/debugger/recorder/screencast capabilities are implemented.

This remains internal and adds no public listener API. Late utility-world
serialization, DOM mutation/events, terminal cleanup/error transport and public
`ElementHandle.setInputFiles` remain absent. Both rebuilt profiles pass Worker
identity checks and 30/30 real-Chrome raw binding cases; offline lifecycle,
package, boundary, type and strict Clippy gates are green. See the
[canonical NODE-188 contract](../../docs/knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: The new `retained-target-prevalidation` and combined artifacts add an
internal `preparePrevalidatedTarget` path. It consumes the exact Rust marked
owner after projection, runs read-only DOM retarget/policy validation, and parks
the validated owner on the same FFI ticket under the same Node deadline. The
ticket remains at ten roots and one 1 MiB permit; there is no release/re-adopt
gap, selector requery or payload-byte serialization.

This remains internal. Input instrumentation, late utility serialization,
mutation/terminal publication, Playwright error mapping and public
`ElementHandle.setInputFiles` are not implemented. Current gates pass 52
composition cases with one ignored, 666 host cases with 24 ignored, 68 addon
cases, 271 offline lifecycle cases, 163 package cases, both Workers/artifacts,
types/exact boundaries, strict standalone/combined Clippy and both 28/28
real-Chrome raw binding lanes. See the
[canonical NODE-187 contract](../../docs/knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Successful selected marking now retains the exact move-only native
target owner in the same parked FFI result. The internal
`prepareProjectedTarget` helper then runs the real carrier-projection kernel,
keeps four raw projected values as roots, stores native multiplicity/directory
flags and advances the same ticket from Marked to Projected. Selected roots grow
from six to ten; earlier profiles remain at six and no JS-byte bound is claimed.

This remains internal. Retained DOM prevalidation has not yet consumed the
owner, and input instrumentation, late serialization, mutation/cleanup, public
error mapping and `ElementHandle.setInputFiles` remain absent. Current gates
pass 68 addon tests, 666 host tests with 24 ignored, 270 offline lifecycle cases,
162 package cases, both Worker/artifact profiles, types/boundaries, strict
standalone/combined Clippy and both 27/27 real-Chrome raw binding lanes.
See the [canonical NODE-186 contract](../../docs/knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: The two `retained-target-marking` artifacts now allocate monotonic
`call@N` identities per native session after server validation, then start one
Node-event-loop deadline after the instrumentation checkpoint and before
materialization. That same progress covers materialization and native target
marking; the marker reads the retained Rust ID rather than accepting JS input.
The internal helper is `prepareMarkedTarget`; ordinary raw preparation can still
stop at Materialized.

This remains internal. Public `ElementHandle.setInputFiles`, carrier projection,
retained DOM prevalidation, input instrumentation, late utility serialization,
mutation and public error wrapping are not implemented. Current gates pass 68
selected addon tests, 269 offline lifecycle cases, 162 package cases, types,
boundaries and strict standalone/combined Clippy. The full internal real-Chrome
binding lane passes 26/26 under both rebuilt artifacts; this is not public API parity.
See the [canonical NODE-185 contract](../../docs/knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: The package can build `retained-target-marking` and
`retained-target-marking-combined` artifacts. They expose an internal raw
Session method that takes an actual Materialized preparation through Rust
original-target marking and parks the same FFI ticket at Marked. The method is
feature-identity checked and absent from earlier profiles. It is not called by
the public file-input facade, so public `ElementHandle.setInputFiles` behavior
is unchanged and incomplete.

Both marker artifacts pass Worker loading and the real-Chrome raw binding lane
passes 24/24 under each, including same-permit accounting, synchronous abort,
late original disposal and foreign-session isolation. The retained lifecycle
contracts pass 267/267 under the marker profile. Public connection-wide call
IDs, the pre-materialization deadline, later native phases and full differential
parity remain gated. See the
[canonical NODE-184 contract](../../docs/knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: Rust host consumers can explicitly select the default-off
`retained-target-marking` feature for original-Handle marking with a bounded
persistent utility-world cache. Same-world calls reuse private state; native
retirement is independent of disposing source Handle wrappers. This is a Rust
host capability only: **no addon feature or Node artifact selection changes**,
and public `ElementHandle.setInputFiles` remains unimplemented. Node preparation
still stops at Materialized; typed handoff, original-target Weak upgrade, the
single operation deadline and later native phases remain integration work.
Selected Rust tests and strict Clippy pass. One assembled-host Chrome case
passes both final host selections with all resources retired; this does not
verify a Node artifact rebuild or public handoff. Default/older retained
regressions and dependency/boundary gates also pass. See the
[canonical NODE-183 contract and verification](../../docs/knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).
Earlier checkpoints below retain their historical scope and execution status.

NODE-182: The default-off `retained-target-marking` backend/composition primitive
owns private persistent utility-world marker state separately from temporary
target groups. The caller still owns world-lifetime routing; this feature is
**not selected by host/addon profiles** and exposes no public retained upload.
Run `pnpm test:handle-target-marking-oracle`: **21 offline cases pass**, comparing
actual extracted Playwright 1.58.2 marking policy with the checked-in scripts
in plain-object VMs (19 paired cases plus two creation contracts), not real DOM.
Coverage includes same/different-call state, event order/flags, dynamic utility
methods and incoming Set-construction failure before old-state reset. Separate
Rust backend/composition suites pass 37/36 standalone and 55/51 with preflight
and large commands. One real-DOM native primitive case passes in both selections
with nine marking calls, eight events and zero remaining
owned cleanup resources. These are primitive-level results, not public Node
call-ID/deadline, weak-target promotion or complete setInputFiles parity.
[Canonical NODE-182 contract and evidence](../../docs/knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).
Earlier checkpoints below retain their historical scope and execution status.

NODE-181: Selected preparation now carries the original native wrapper's inert
weak identity through every language phase. Capture/clone does not lock,
upgrade, check disposal or pin native resources; copied metadata and an
overridden JavaScript receiver cannot replace this binding. The existing
32-entry capacity and independent call/synchronous-phase storage bounds remain
unchanged. Preparation still stops at Materialized: native Weak upgrade,
marking, typed native handoff, retained prevalidation and public upload are open.

Both rebuilt profiles pass **268 selected offline checks each**; package tests
pass **161**, including ten sampling/diagnostic tests, with types and the
35-case descriptor/language DOM oracle passing. Both selected addon suites pass
65 tests; the focused host phase-storage test, five strict Clippy selections,
boundary/dependency gates and formatting pass. After the final-retirement fix,
standalone and combined actual-ticket live binding each passed **20/20**. A
standalone repeat then passed 19 and failed one active-baseline comparison:
queued watch delivery became armed watch work with unchanged ownership totals.
The harness now also requires no queued FFI and exact running/pending watch
accounting for active baselines. Final frozen-source standalone, combined and
standalone-repeat runs each pass **20/20**, with zero failures/cancellations/skips
and fully retired owned cleanup: **20 distinct cases × three runs = 60 successful
executions**, not 60 distinct cases. Initial live evidence is retained: five
passes preceded a case-six cleanup failure and fourteen blocked cases; the
diagnostic rerun failed its first cleanup on `ffiPending = 1`, blocking nineteen.
The harness now waits for all final resources, including watches, to reach
zero instead of accepting command-idle samples; deadlines were not increased.
These cases do not prove later Weak upgrade, wrapper GC or upload success.
Exact evidence and final live results belong in the
[canonical NODE-181 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).
Earlier checkpoint paragraphs below retain their historical execution status.

NODE-180: The selected host adds eight source-bound weak FFI revocation slots
with a separate 4 KiB administrative allowance. Registration belongs after
Materialized, immediately before the first native phase; it must not reduce
the 32 language-preparation slots or introduce early disposal/DOM checks.
Explicit close, peer termination and failure teardown revoke detached hooks
outside locks before native work drains. The shutdown admission barrier and
Owners close also keep cancellation/tracker wakeups outside their state locks.
Source review also moves retirement-failure transport disconnect outside the
session lock; this is not an assembled-session failure-path test.
This is host bookkeeping, not actual Node promotion: preparations still hold
a unit FFI result and stop at Materialized. Exact original-wrapper binding,
typed native progress, marking, retained prevalidation, late serialization and
public upload remain open. Rebuilt standalone and combined selected artifacts
each pass **268 cases across seven offline lanes**. Package contracts pass 151,
types pass and the descriptor/language DOM oracle passes 35; these are offline regressions, not
proof of actual Node ticket promotion or DOM/browser execution. The 17
actual-ticket live cases remain unexecuted; host/component results and remaining
gates
belong in the [canonical NODE-180 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Rust now provides raw carrier projection plus same-admission FFI
park/resume prerequisites. Parking retains the original result/permit without
a running task token across foreign waits; neither prerequisite is wired into
the actual preparation, which still stops at Materialized. There is no marking
receipt or public upload API. Run
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-projection`
and repeat with `retained-file-input-combined`. Both final artifacts, rebuilt
after the shutdown-lock/waker fix, pass **47 projection cases and 268 cases
across all seven selected offline lanes each**. Package contracts pass 151,
type checks pass, and the separate DOM oracle passes 35 descriptor/language
cases. These diagnostics do not prove actual ticket ownership, DOM/browser
behavior or upload success. The 17 actual-ticket cases remain unexecuted.
Final artifact and native evidence
belong in the [canonical NODE-179 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: The default-off native `retained-preflight` leaf owns the retargeted
input after a payload-free DOM precheck; it is not selected by host/addon
profiles and does not expose public ElementHandle.setInputFiles. Run
`pnpm test:handle-file-input-dom-oracle` from this package: **35 offline cases
pass**, comprising 24 pinned upstream cases and 11 comparisons of the actual
candidate script against pinned callbacks on plain descriptors, not DOM/browser
execution. Current package contracts pass 151 cases and type checks pass. No
addon was rebuilt for these regressions; selected artifacts remain the NODE-177
builds. The 17-case actual-ticket lane remains unexecuted. Native test evidence
and remaining integration gates are maintained in the
[canonical NODE-178 checkpoint](../../docs/knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Selected retained preparation adds Rust registration checks before
server validation and after a thin empty-instrumentation await, with setImmediate
delivery from the client continuation. Checks use exact owner-ticket and creation
Page membership, not world/Handle/Frame runtime liveness. The internal driver
resolves undefined at Materialized; the caller retains its preparation. It adds
no roots, dependencies, classes, budgets or native pins and does not expose public
ElementHandle.setInputFiles. Twenty new offline dispatch cases pass per artifact;
canonical verification distinguishes those adapters from actual tickets/browser
work. Full instrumentation, error transport and correctly ordered late utility
serialization/native submission remain open. No fresh browser pass is claimed.
[Canonical contract and verification](../../docs/knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

After rebuilding, run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-dispatch`
from this package and repeat with `retained-file-input-combined`. Both versions
pass 221 cases across dispatch, materialization, channel, arguments, preflight
and Worker lanes. The 17-case live binding lane remains unexecuted while shared
Chrome CDP is unavailable.

NODE-176: Selected retained-upload builds add Rust server-direction validation
and raw payload materialization, preserving custom serializers and outer async
assimilation. Public ElementHandle.setInputFiles and native submission remain
unimplemented. Run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-materialization`
after building, then repeat for
`retained-file-input-combined`. This is offline kernel evidence only.
Both rebuilt profiles pass 76 new cases plus regressions; the 12-case live
ticket gate remains unexecuted while shared Chrome CDP is unresponsive.
[Canonical contract and current evidence](../../docs/knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Selected retained-upload preparation adds Rust-owned
raw-buffer channel validation and an internal ChannelReady stop point. This
preserves observable map/valueOf/property assignment and Buffer identity without
copying bytes. It does not yet expose public ElementHandle.setInputFiles, valid
WritableStream authority, API error wrapping or native submission. No feature,
profile, dependency, class or preparation-root limit is added.
[Contract, verification and remaining gates](../../docs/knowledge/pwright-node-handles.md#node-175-rust-owned-raw-buffer-channel-validation).

After rebuilding the selected artifact, run
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-channel`
from this package; repeat with `retained-file-input-combined`. This offline
diagnostic lane compares actual native schema behavior with pinned Playwright,
not actual preparation tickets or browser upload conformance.

The final standalone and combined artifacts each pass **61 channel cases**,
plus the existing argument/preflight/Worker checks. The live binding lane now
has eight authored cases, including ChannelReady and channel-failure cleanup;
these browser-dependent cases remain unexecuted.

NODE-174 (2026-09-17): Retained-upload profiles now bind Handle creation to a
Rust-owned canonical Page/Context timeout policy. JS forwards the creation Page
ID; Rust carries the immutable binding through descendant Handles and preparation,
without using current DOM ownerFrame or caller-selected late settings. Session
initialization consumes import-cached raw debug inputs; Rust chooses the policy.
Metadata admission now covers cells, repeated scoped wrappers and session indexes
under a 784 KiB administrative ceiling, not a process-RAM limit. `pnpm test`
includes offline facade forwarding/lifecycle contracts. Public default setters
and ElementHandle.setInputFiles remain unimplemented; actual native/browser
binding verification remains separate. [Contract, evidence and remaining work](../../docs/knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding).

The new live gate is **written and syntax-checked, not yet run**. Once the
dedicated Chrome CDP endpoint is recovered, run from this package:

```sh
PWRIGHT_NATIVE_PROFILE=retained-file-input PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:handle-file-input-binding
```

Repeat with `retained-file-input-combined`. The script sets `PWDEBUG=0` before
import. These six cases require actual session/Handle/preparation objects and
verify ArgumentsReady plus abort/reclamation; they do not prove public uploads.

NODE-173 (2026-09-17): Selected retained-upload staging now merges arguments
and resolves live numeric Context/Page timeout settings in Rust immediately
after conversion. The internal metadata factory adds a bounded 528 KiB allowance
and one opaque class, not browser or task ownership. Public creation-Frame
binding, default-timeout setters and `ElementHandle.setInputFiles` remain
unimplemented; no public timeout policy is replaced by this internal component.
After building, run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-arguments`
from this package; repeat with `retained-file-input-combined`. This is offline
kernel/metadata verification, not actual-ticket or browser conformance.
[Exact contract, evidence and limits](../../docs/knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings).

NODE-172 (2026-09-17): The retained-file-input selections add a Rust preflight
kernel and reentrant phase guard. A narrow language driver preserves public
owner lookup, synchronous Context resolution and async conversion of the exact
ordinary payload carrier. Root counts now reflect 3/4/5/6 retained values by
stage. Initial construction and replacement separately reserve bounded
synchronous scratch (32 phases x 16 KiB), surviving callback revocation without
holding an FFI task across awaits. These are administrative allowances, not
arbitrary V8 graph byte bounds. This is still internal staging:
public `ElementHandle.setInputFiles`, options/channel validation, path work,
materialization and native submission are absent. Selected diagnostic exports
exercise the same kernel offline but create no real ticket/session/browser.
[Canonical contract, verification and limitations](../../docs/knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases)
distinguish kernel comparisons from public API compatibility.

After building either selected profile, run
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:handle-file-input-preflight`
from this package (substitute `retained-file-input-combined` for the combined
artifact). This runs the shared Rust kernel against pinned Playwright 1.58.2,
not the public upload method or actual rooted tickets.

NODE-171 (2026-09-17): Two new opt-in artifacts, `retained-file-input` and
`retained-file-input-combined`, expose an internal session-bound preparation
registry. Older profiles are unchanged. The new raw native ticket admits and
roots arguments, supports explicit abort, and is revoked during session close
and environment cleanup. Public `ElementHandle.setInputFiles` is still absent;
no owner lookup, conversion, serialized payload or native submission is wired
to this ticket yet. Its fixed entry/root limits do not bound arbitrary V8 input
graphs or user-retained closed-wrapper/tombstone allocations.

Build and run the offline selected-artifact check from this package:

```sh
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm build
PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:preparation-boundary
```

Use `retained-file-input-combined` for both commands to select the combined
profile. The check validates real class loading/type tags and environment
Worker exit, with zero actual tickets or browser connections. It does not
establish rooted-ticket cleanup or upload compatibility. See the
[canonical NODE-171 contract and evidence](../../docs/knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).

NODE-170 (2026-09-17): The Rust host adds revocable `FfiPreparation<T>` under
its existing `retained-file-input` selection. Preparation shares the existing
FFI lane/byte budget without holding a running-task token until atomic start.
Exact-budget provenance binds the environment, not the session: the Node
registry, session revocation, type-tagged preparation ticket and full retained
upload ingress remain missing. This checkpoint adds no addon feature, public
Node API, artifact, dependency or feature.

`pnpm test:handle-file-input-oracle` characterizes upstream-only offline behavior.
The additions cover owner `page()`/`context()` evaluation, error precedence,
empty-input microtasks, actual client metadata disposal during getter reentry,
and custom Buffer serialization with per-item snapshot order.
They do not exercise native DOM disposal, a browser, or candidate upload parity.
[Canonical NODE-170 contract and current verification](../../docs/knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution)
records Rust gates still in progress and remaining integration. Earlier entries
below retain their historical scope.

NODE-169 (2026-09-17): Run `pnpm test:handle-file-input-oracle` from this package to characterize Playwright 1.58.2 retained-upload ingress offline. It imports the pinned dependency from tests/node_modules and never connects to Chrome. The new retained-file-input selection is Rust-host-only; no Node artifact or public ElementHandle.setInputFiles support is added.
[Canonical contract and evidence](../../docs/knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-167 implementation contract (2026-09-17): The explicit `handles` artifact
now exposes `ElementHandle.ownerFrame(): Promise<Frame | null>`. Rust queries the
retained node's current DOM owner, not its creation Frame, using a temporary
document-element Handle, direct raw `DOM.describeNode` and settled native
cleanup. After topology publication, JavaScript returns the canonical Frame
wrapper; `evaluateHandle`, `getProperty` and `getProperties` descendants preserve
the context-scoped resolver. Removed nodes may retain an owner; orphan documents
return null. Retained ElementHandle upload is still not implemented.

OwnerFrame acquisition validates/copies one protected receiver ID only after
evaluation admission. It rejects source-alias success and skips that borrowed
ID during malformed/exception compensation, still attempting every distinct
new owner. Other acquisition paths are unchanged; this is not generic
borrowed-ID protection or a release pin against concurrent source disposal.

To distinguish navigation from close, scalar and batch publication retain an
immutable Page/numeric-context/`Arc<str>` unique-context fingerprint inside the
existing 16 KiB descriptor allowance, with no client, receipt, permit or native
lease. Only an observed replacement main world on the same Ready Page permits
the canonical navigation error before lookup or refinement of ordinary
TargetClosed; missing worlds, crashes, closes and Cleanup are not reclassified.
The focused lane also covers full persistent capacity, same-document navigation
and navigation followed by close; canonical verification, not this case list,
records what actually passed on each artifact.

The host pre-owns the started-call continuation and shares eight call slots,
with two transient `RemoteHandle` units per query, sixteen aggregate at pending
scope `extension(154, 1)`. No slot from the 64 persistent-result owners is used.
Raw callers still owe source/future retention through cleanup settlement;
unaddressable raw IDs/pre-owned groups and conservative Cleanup-versus-detach
outcomes remain open. Run `pnpm test:handle-owner-frame` after dedicated
Chrome/Pwright preflight with `PWRIGHT_NATIVE_PROFILE=handles` and
`PWRIGHT_TEST_CDP=http://127.0.0.1:9222`; the optional OOPIF case also needs the
existing served `PWRIGHT_DRAG_FIXTURE`. [Exact verification and remaining gates](../../docs/knowledge/pwright-node-handles.md)
are canonical; this checkpoint makes no live-pass claim. NODE-166 and earlier
entries below are historical checkpoints, not current aggregate pass counts.

NODE-166 (2026-09-17): The handles artifact implements JSHandle.getProperties(),
returning an ordered Map with independently disposable JSHandle/ElementHandle
values. Its Rust core admits a whole batch before browser work and compensates
failed publication without waiting for GC. Selected session capacity remains
64 retained results total (including the source); concurrent calls share eight
slots. Run the focused comparison with `pnpm test:handle-properties` after the
required dedicated Chrome/Pwright preflight and with PWRIGHT_NATIVE_PROFILE=handles
and PWRIGHT_TEST_CDP=http://127.0.0.1:9222. [Exact contract and verification](../../docs/knowledge/pwright-node-handles.md)
remain distinct from the broader unfinished Playwright surface.

NODE-165 (2026-09-17): Rust Handle acquisition now preserves raw CDP result
and exception identities through settlement and compensates addressable IDs
before reporting failure. Existing admission stays held during that cleanup;
unaddressable IDs or refused releases remain visible Cleanup failures.
This adds no public Node API, package, feature or portable schema.
[The canonical checkpoint](../../docs/knowledge/pwright-node-handles.md)
records the selected artifact and exact verification; historical passes do
not verify a newly built artifact. Late property-read settlement, full property-
pipeline admission, pre-owned groups for unaddressable IDs, Node getProperties
batch ownership and broader compatibility remain unfinished.

NODE-164 (2026-09-17): All exported raw native classes now receive module-local TypeTag identities through the actual N-API conversion checks, addressing the earlier priority cross-build implementation gap. The napi_runtime dependency key still refers to the same napi package/version/features. Portable property extraction also preserves snapshot identity and pre-admitted cleanup ownership. [The canonical checkpoint](../../docs/knowledge/pwright-node-handles.md) records exact old-peer/same-build module isolation verification and remaining regression gates. Node getProperties and the broader compatibility surface remain unfinished.

NODE-163: The handles profile adds JSHandle.evaluateHandle (function/string and owned arguments) and getProperty. Browser policy and ownership remain Rust-side; failed native/JS publication compensates the owned result explicitly. getProperties remains unimplemented, and its five paired tests are retained as failures. Run pnpm test:handle-producers with the selected handles artifact; PWRIGHT_HANDLE_PHASE_TRACE=1 enables bounded oracle/candidate diagnostics. Exact evidence and property Promise-identity semantics are in ../../docs/knowledge/pwright-node-handles.md.

NODE-162: The handles profile now supports string-form JSHandle.evaluate and string Frame/Page evaluation with owned Handle arguments. Arguments are still converted and validated even though expressions ignore their values. Rebuild this profile for the updated native Handle call signature. Handle-targeted producers, previews, ownerFrame/adoption and retained ElementHandle actions/uploads remain unsupported.

NODE-161: The handles profile now accepts real Handle arguments in function-form Page/Frame.evaluate, Page/Frame.evaluateHandle and JSHandle.evaluate, including nested arrays/objects, aliases and cycles. TypeScript unwraps Handle argument types. String expressions with Handle arguments, cross-Frame adoption, additional Handle producers, toString and ElementHandle actions/uploads remain unsupported. Run PWRIGHT_NATIVE_PROFILE=handles pnpm build and the matching pnpm test:handles lane after dedicated Chrome preflight.

NODE-160: An independent handles native profile now supports Page/Frame.evaluateHandle, JSHandle.jsonValue, function-form JSHandle.evaluate, dispose and ElementHandle.asElement identity. Build with PWRIGHT_NATIVE_PROFILE=handles pnpm build; select that same environment value when running. Reference arguments, string Handle evaluation, toString, additional Handle producers and ElementHandle actions/uploads are not implemented. See ../../docs/knowledge/pwright-node-handles.md for exact verification and limitations.

NODE-159: The Rust safe host now integrates staged Frame Handle creation, driver publication and owned orphan-result cleanup. This Node package still does not select Handles or expose JSHandle/ElementHandle. No addon artifact was rebuilt or new public parity score established; see the canonical Node Handle integration record.

NODE-158: A separately opt-in Rust leaf now supports staged native Handle creation. This package still has no public JSHandle/ElementHandle API and does not select native-creation. No addon artifact was rebuilt; the last paired Node close evidence remains NODE-157.

NODE-157: Rust host Handle cleanup now uses bounded native-release receipts and
driver-owned local commits. Public JSHandle/ElementHandle APIs remain absent;
the addon still does not select handles. The existing combined profile
`file-input-concurrent-close-drag-frame-extra-headers` was rebuilt because
in-process concurrent close no longer allocates an unused control outbox.
Its SHA-256 is `93c0527d02bbcb66d8082e4ddda6f673b1b66528d784e82445e5ec122de2cd6a`.
Paired native-close-confirmation passes 6/6, paired lifecycle-events 7/7,
Node contracts 100/100 and strict TypeScript validation passes. Further validation and
remaining gaps are recorded in the [Node Handle checkpoint](../../docs/knowledge/pwright-node-handles.md#node-157-host-release-coordinator-and-parent-retirement-evidence);
this is not a rerun or improvement claim for the complete 57-case shared corpus.

NODE-154: The Rust host is adding owned pre-publication Handle results and session-driven cleanup. This Node package still has no Handle producer or selected handles artifact. Do not infer JSHandle/ElementHandle compatibility from the internal Rust owner tests.

NODE-152: The Rust host has a handles-only native installation prerequisite. This Node package still does not select it or expose JSHandle/ElementHandle. No native package artifact is rebuilt; use the Node Handle integration record for pending publication/cleanup work.

NODE-151: test:handle-file-input is a new explicit experimental conformance lane. PWRIGHT_HANDLE_PROVIDER=oracle characterizes 18 passing upstream cases; default paired mode fails on missing Node Handle APIs. This does not add ElementHandle support or change the prior shared score. Rust has a new borrowed-node payload primitive; no native package artifact was rebuilt.

NODE-150: Rebuilt file-input profiles support Page.setInputFiles and Frame.setInputFiles as well as Locator. Selector APIs choose the first match by default; strict: true opts into strictness. Upload verification passes 43/43 x3 standalone and 43/43 combined; full shared corpus remains 53/57 matching. ElementHandle and native directory uploads remain unsupported.
[Page/Frame upload contract and verification](../../docs/knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 verified checkpoint: upload suites pass 32/32 on standalone and combined profiles. The complete shared API corpus is 53/57 matching; overall Playwright compatibility is not complete. Default and previous native artifacts remain unchanged.

NODE-149: Locator.setInputFiles is now available only with PWRIGHT_NATIVE_PROFILE=file-input (build/select it explicitly). Supports memory payloads and regular paths; SharedArrayBuffer, native directories and Page/Frame/handle upload methods remain unsupported. Earlier NODE-148 and prior entries below are historical checkpoints.
[Selected Node upload boundary and verification](../../docs/knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Selected safe Rust host APIs now support buffered/path upload composition with existing Frame/Locator ownership. This JS package and all existing native artifacts remain unchanged: public setInputFiles is not yet implemented. Node ingress, selected addon build and differential verification are next.
[Selected safe-host upload contract](../../docs/knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: The optional Rust transport can now admit large buffered uploads independently and fragment their writes. This Node package still does not expose setInputFiles or select the upload leaves; no native artifact was rebuilt. Public lifecycle integration and differential verification remain pending.
[Large outbound admission and remaining gates](../../docs/knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: Rust has an optional owned payload-upload continuation with admitted cleanup and explicit protocol settlement. This package still does not expose setInputFiles or select the upload leaves. No native addon was rebuilt; full public cancellation, native path/directory behavior and differential verification remain pending.
[Upload continuation and remaining gates](../../docs/knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: Optional Rust buffered-file mutation now has actual Chrome fixture evidence, including an end-to-end Rust file-reading path. This Node package still does not select the backend or expose setInputFiles. Native path/directory completion, operation cancellation and public FFI integration remain pending; no addon was rebuilt.
[Low-level payload backend checkpoint](../../docs/knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: Optional Rust file-input execution now has bounded task admission and cancellation-safe filesystem workers. This Node package still does not select it: public setInputFiles remains unsupported until native action, lifecycle integration, message budgets and wrapper wiring are implemented. Native artifacts remain unchanged.
[Executor checkpoint](../../docs/knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Optional Rust filesystem preparation and a pinned MIME provider now cover path sources, without selecting them in this Node package. setInputFiles is still unsupported; the executor, native backend and FFI wrapper remain to be implemented. Existing native artifacts remain unchanged.
[Filesystem/MIME checkpoint](../../docs/knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: A separate Rust file-input content leaf now has ledger-admitted bounded copying/reading and ownership tests. It is not selected by this package; setInputFiles remains unsupported pending path preparation, native actions and Node integration. No native artifact was rebuilt.
[Content ownership checkpoint](../../docs/knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: A separate Rust file-input preparation model and `pnpm run probe:file-input-oracle` now document pinned upload behavior. This does not add Locator.setInputFiles or an upload native artifact; the public API remains unsupported until resource-admitted IO, native actions and FFI wiring are complete.
[File-input policy, evidence and remaining work](../../docs/knowledge/pwright-file-input.md).

NODE-140: Broader verification is not green: Frame navigation 27/29 and shared API equality 51/57. The existing experimental artifact is unchanged. New bounded diagnostics are available as `pnpm run probe:page-close-body` and `pnpm run probe:shared-click`; their observed failures are retained, not counted as passing compatibility gates.
[Expanded verification and retained failures](../../docs/knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: The selected Frame navigation core retains Page lifetime through pending publication and final result conversion. Run the new paired committed-document close controls with `pnpm run test:frame-navigation-close`, selecting a frame-navigation native profile and setting PWRIGHT_TEST_CDP, PWRIGHT_TEST_BIND and the full PWRIGHT_TEST_HOST. This is a focused regression lane, not a claim of complete Playwright parity.
[Page lifetime publication contract and verification](../../docs/knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Experimental `concurrent-close` and
`concurrent-close-drag-frame-extra-headers` profiles now wire bounded owned
Page/Context native close scheduling and local retirement in Rust. They install
into separate native directories and require a matching binary marker; the
default artifact is unchanged. Build explicitly with
`PWRIGHT_NATIVE_PROFILE=concurrent-close-drag-frame-extra-headers pnpm run build`,
then use the same environment selection when testing or loading the package.
The final combined artifact passes controlled/transparent close gates and 27
focused lifecycle regressions. This is not full compatibility: interrupted
Frame.goto error classification still differs, and default/external Context
paths retain the old scheduler.
[Integration scope, retained failure and browser evidence](../../docs/knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-133: Rust world receipts retain owning-Page identity and process real Page destruction even after local route retirement. Default and drag/frame/extra-headers addons were rebuilt and pass 28 focused real-Chrome regression cases; hashes and exact scope are recorded below. The serial Node close scheduler and controlled ancestor-dispatch gap remain unchanged. Earlier NODE-123 broad matrices cover older artifacts, not these builds.
[Native Page world-retirement contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-130: Rust adds an optional deletion-only local route commit for upcoming concurrent close integration. Node still uses its existing serial lifecycle path; no installed addon rebuild, public behavior change or new browser differential result is claimed.
[Deletion-only close route contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Rust gains optional concurrent native close scheduling with bounded receipts and guarded route validation. The Node driver/addon still use serial close execution: installed artifacts and browser evidence remain NODE-123. Retirement/publication and host integration remain required.
[Bounded native scheduler contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Rust gains optional exact native Context ancestor receipts and sealed Page bindings, verified at the composition layer. The Node driver/addon do not select native-close-authority yet; default/combined installed hashes and browser evidence are unchanged. Concurrent native coordination remains next.
[Native ancestor authority contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: The Rust leaf now supports separately owned native close acknowledgement and destruction waiting. This additive API is not selected by the package's serial close scheduler yet. Installed default/combined addon hashes and browser evidence are unchanged; exact ancestor receipts and coordinator integration remain next.
[Native close phase contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Rust close observers now support deferred publication admission and current-view preparation. This prerequisite is not wired into the Node close scheduler; installed native artifacts and browser compatibility evidence are unchanged.
[Deferred publication contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Optional Rust runtime disposal/outbox reservations now complement topology reservations. Neither is selected by this package yet; Node binaries and browser evidence are unchanged. The next work is bounded native close scheduling, exact ancestor receipts, publication rebasing and complete peak ownership budgets. These storage primitives alone do not fix ancestor close dispatch.
[Runtime reservation contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Rust now has an optional topology transition-reservation building block for concurrent close work. It is not selected by this Node package yet and does not fix ancestor close dispatch alone. Runtime disposal reservations, native scheduling and publication rebasing remain next; the NODE-123 addon artifacts and verified browser matrix are unchanged.
[Topology reservation contract and evidence](../../docs/knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: In-flight Context.close interrupted by Browser/peer termination reports TargetClosed after terminal staging; retired Contexts remain idempotent. Driver producer/queue loss now seals admission before close settlement, preserving original emitters. Both final profiles pass 50 close/event/connection cases plus two extra six-case confirmation repeats. `pnpm probe:close-cascade` records transparent or explicitly controlled diagnostics; the candidate ancestor-dispatch gate remains red, outside that matrix. All scheduling policy remains in Rust.
[Design, bounded probe configuration and evidence](../../docs/knowledge/pwright-close-cascade.md).

NODE-122: The Node Events composition selects Rust native destruction confirmation for Page.close, including ownership-safe interruption and terminal event settlement. Run `PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:native-close-confirmation` against the dedicated Chrome; the four cases include paired held-receipt behavior and Worker teardown. Both final artifacts pass 48 close/lifecycle/connection tests. Frame navigation remains 27/29 at the unresolved arrival-boundary close scenario; use the workspace `tests/fixtures` directory for that separate lane. No Node driver, browser launcher or full Playwright-parity claim is added.
[Contract and evidence](../../docs/knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Adds transparent original-listener close diagnostics and eleven ordinary unit tests. Both existing native profiles pass their 16 diagnostic samples and full 18-case listener corpora; natural failures remain unproven as fixed. Pinned source confirms the ordinary-close confirmation policy still needs a native composable implementation.
[Transparent trace and next confirmation contract](../../docs/knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Native backend errors retained by Browser.close now include driver/retirement phase context. The new test:driver-close-attribution lane verifies an injected Page.enable response timeout on both rebuilt profiles. Full listener corpora pass 18/18, but naturally occurring driver timeouts and close-confirmation exhaustion remain unresolved; this is not a stability or complete compatibility claim.
[Close attribution contract and evidence](../../docs/knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Native Context.page and Page.popup notifications are connected to the Rust publication source and precommit wrapper converter, alongside explicit/external/terminal close events. Both final profiles pass full lifecycle and listener corpora 7/7 and 18/18. One combined listener repetition timed out during browser.close cleanup and remains unresolved. This is still an experimental API subset, not full Playwright compatibility or a stability guarantee.
[Native ready-source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Internal ready/popup wrapper conversion is prepared, with metadata admission before native publication and wrapper cache installation afterwards. Native creation/linkage source wiring is still pending: newPage/popup event parity remains incomplete. Existing close-event support is unchanged.
[Creation-conversion contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: External Page-close notifications now match Playwright in the Worker-confirmed differential case, alongside explicit and terminal close sources. Both profiles pass 5/7 lifecycle-event cases and 14/18 listener cases. newPage/popup notifications remain incomplete; this package is not yet fully Playwright-compatible.
[External-close source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Explicit owned Page/Context close events now reach Node using Rust native transaction receipts, including synchronous listener failure and close reentry. Terminal events remain supported. Page creation, popup and externally initiated Page-close event parity are still incomplete.
[Explicit-close source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: The package now selects Rust terminal lifecycle delivery: Session termination dispatches close on existing Page/Context wrappers before Browser disconnected. Ordinary Page/Context explicit-close events and creation/popup notifications remain incomplete; do not treat the package as fully Playwright-compatible.
[Terminal dispatcher contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Pwright Node FFI (experimental subset)

NODE-114: The Rust host now offers an experimental terminal-event connection path, verified through actual native shutdown. This package does not select it yet; Page/Context event coverage still awaits native ordinary-source joining and FFI delivery.
[Terminal source contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Rust now provides a bounded lifecycle publication group/cursor primitive; it is not yet wired into this package's Page/Context events. The listener gate adds Context/Browser close exception-isolation cases. Browser disconnected support is unchanged.
[Group contract and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: The Rust composition offers opt-in explicit close receipts. The Node host still
uses its existing services without that observer; this does not add public
Page/Context events. Native selected-path and fresh addon regression evidence
are tracked separately.
[NODE-112 close receipt contract and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: The native composition gains a borrowed driver-effect input. No Node API or
new lifecycle callback support is exposed by this checkpoint; the existing
metadata fallback remains in use. Rebuilt artifact regression evidence is
recorded with the source coverage limits.
[NODE-111 source contract and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 fixes the synchronous Browser disconnected listener error channel in
rebuilt addons. test:lifecycle-listeners adds eleven isolated reference/paired
cases; Page/Context event cases remain incomplete. Older addons keep the old
exception behavior until rebuilt. No runtime queue or browser logic is added to
JS by this scoped exception-forwarding boundary.
[NODE-110 contracts, fix and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 adds an exclusive Rust metadata drain foundation, not a new Node API or
lifecycle callback implementation. The package still publishes whole fences.
Page/Context/popup lifecycle notifications remain incomplete; internal queue
steps must not be treated as supported Playwright events.
[NODE-109 contract and verification](../../docs/knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108's attachment-command diagnostic requires a freshly rebuilt Rust probe
with bounded native lifecycle identity receipts. Old probe binaries lack this
field and fail evidence validation. This TRACE data does not implement public
Page/Context lifecycle callbacks. Controlled retirement comparisons pass on the
unchanged NODE-107 Node addons; earlier intermittent initialization failures are
not claimed fixed.
[NODE-108 receipts and verification](../../docs/knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107's rebuilt combined-profile live gate is not fully verified: attachment
timeouts interrupt Page enumeration/baseURL and some other cases. The Rust-only
combined probe reproduces initialization stalls outside the Node bridge; no fix
or deadline change is included. Default regressions do not prove combined stability.
[Known failures and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#combined-profile-reproduction-and-incomplete-live-gate).

NODE-107 adds a Rust-only primitive for rebasing visible collection membership
across admitted revisions. Existing Node collection getters and full-fence
publication remain unchanged. This does not implement lifecycle callbacks.
[NODE-107 contracts and evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 rebuilds both addon profiles with Rust immutable collection visibility
views. Existing getters still expose full published membership; the new view
foundation does not add Context page, Page/Context close or popup event support.
No JavaScript API, dependency or feature-selection change is introduced.
[Implementation and measured evidence](../../docs/knowledge/pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 adds the intentionally RED test:lifecycle-events lane. Dedicated-Chrome
preflight plus PWRIGHT_TEST_CDP is required; no HTTP fixture is used.
PWRIGHT_LIFECYCLE_REFERENCE=1 runs the pinned reference only (7/7). Default and
drag-frame-extra-headers each fail all seven cases: Context page, Page/Context
close and popup callbacks are missing, although final collections are correct.
Do not infer these events are supported merely because on() accepts a listener.
[Evidence and Rust implementation design](../../docs/knowledge/pwright-node-lifecycle-events.md).

NODE-104 adds test:collection-creation (7/7 twice on both NODE-103 artifacts).
It covers blocked-loop single/concurrent creation, retirement/replacement and
independently confirmed borrowed Page discovery; no HTTP fixture is needed.
Use the dedicated Chrome/Pwright preflight and PWRIGHT_TEST_CDP explicitly.
PWRIGHT_COLLECTION_REFERENCE=1 runs only the pinned reference, not candidate parity.
Context page, Page/Context close and popup events remain unimplemented.
[Exact scope and evidence](../../docs/knowledge/pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-103 connects collection lifecycle publication and reads in Rust. Rebuilt
default and drag-frame-extra-headers pass test:collection-publication (7/7),
test:collection-lifecycle (1/1), Browser/Page scalar and enumeration regressions.
test:context-input-boundary checks Pwright's own bounded-input TypeError contract
and selected-header getter identity; test:context-base compares 16 upstream cases.
No browser is downloaded or launched by the package; use the dedicated Chrome
Pwright preflight and explicitly select/rebuild the desired native profile.
[Implementation, artifact hashes and remaining limits](../../docs/knowledge/pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
The earlier red/unwired checkpoints below are historical, not current test status.

NODE-102 implements the Rust collection storage/fence layer, but does not yet
wire native lifecycle producers or replace Node collection getters. Installed
artifacts are unchanged and test:collection-publication remains RED as recorded
in NODE-101. Do not treat the new Rust unit results as a public compatibility fix.
[Storage implementation and pending cutover](../../docs/knowledge/pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 adds the intentionally RED collection publication gate:
```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:collection-publication
```
Use the dedicated-Chrome/Pwright preflight; no HTTP fixture is required.
PWRIGHT_COLLECTION_REFERENCE=1 explicitly runs reference-only characterization
(7/7 twice), not candidate conformance. Default and drag-frame-extra-headers
currently pass only 1/7: collections remove members too early while Node is
blocked. No native fix/rebuild is included in this checkpoint; NODE-100's
scalar getter fix remains valid within its tested scope.
[Evidence and Rust implementation gate](../../docs/knowledge/pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 fixes Browser.isConnected publication timing in Rust and removes the
JS disconnect flag from rebuilt artifacts. After dedicated-Chrome/Pwright
preflight, run:
```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:browser-connection
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 PWRIGHT_NATIVE_PROFILE=drag-frame-extra-headers pnpm test:browser-connection
```
No HTTP fixture is needed. Both rebuilt profiles pass 5/5 and Page.isClosed
8/8; the combined Page enumeration lane passes 7/7. Rebuild each desired native
profile: older binaries keep the premature getter behavior through a legacy
fallback. Broad lifecycle/event/API compatibility is not yet complete.
[Contract and current artifact hashes](../../docs/knowledge/pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 adds Rust-backed `page.isClosed(): boolean`. Rebuild the desired native
profile; old artifacts fail this method explicitly. The existing
`PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:page-is-closed` lane now passes
8/8 on final rebuilt default/combined profiles, with Page enumeration 7/7 and
Worker teardown 3/3 each. Final artifact hashes and resource checks are in the
checkpoint below; other profiles have not been rebuilt in this turn.
No browser launch, JS lifecycle copy or new dependency is introduced. Page
close/crash events, before-unload true and close reasons remain unsupported.
[Contract and verification](../../docs/knowledge/pwright-page-close.md#node-099-rust-owned-page-close-publication).

The NODE-098 red results below are the preserved pre-implementation baseline.

NODE-098 adds `PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:page-is-closed`
(after the dedicated-Chrome/Pwright preflight; no HTTP fixture). This lane is
currently RED: Page.isClosed is not implemented. Eight upstream cases pass;
default and combined candidate runs fail 0/8 at the missing method. Explicit
`PWRIGHT_IS_CLOSED_REFERENCE=1` runs labelled reference-only characterization,
not candidate conformance. No native artifact or supported API changed.
[Contract and next implementation gate](../../docs/knowledge/pwright-page-close.md#node-098-pageisclosed-reference-contract-and-failing-candidate-baseline).

NODE-097 expands `pnpm test:navigation-peer-disconnect` to 11 cases: four existing
shutdown controls plus seven paired mid-navigation peer/recovery observations.
Combined and default pass 11/11. Three modes deliberately reproduce native
navigation failure in BOTH implementations; passing means matching the original
outcome and healthy explicit recovery, not pretending every navigation succeeds.
The same dedicated-CDP and Tailscale fixture variables below apply. No production
API or native binary changed. [Evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 adds `pnpm test:navigation-peer-disconnect`: four owner/peer combinations
hold the borrowing client's native child resume command, disconnect that client,
and verify the owner Page/child survive. Use the selected native profile plus
PWRIGHT_TEST_CDP=http://127.0.0.1:9222, explicit PWRIGHT_TEST_BIND (Tailscale IP)
and PWRIGHT_TEST_BASE (full MagicDNS HTTP origin), as with the header fixture.
The combined control passes 4/4; continuous connection churn remains failing.
This test-only checkpoint does not change production Rust or increase deadlines.
[Evidence and limits](../../docs/knowledge/pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 extends test:attachment-retirement to require connection settlement
from native detach BEFORE a held initialization response. The Node host selects
Rust pending-session retirement with a 256-byte native ID bound; no JS driver is
added. Current combined/default attachment cases pass 10/10 and 6/6;
test:detach-retirement now tests three native receipt/event/live-Worker orderings
and passes 3/3 on both artifacts. Child-header overlap remains a failed gate.
[Contract and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 adds `pnpm test:queued-context-retirement`, using the dedicated Chrome
CDP endpoint and no HTTP fixture. It tests late disposed/live Context attachment
while a known Page's native initialization is held. Build the selected native
profile before running; this lane is not complete Playwright conformance.
[Checkpoint](../../docs/knowledge/pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-093 expands test:attachment-retirement to cover header initialization:
8 combined cases and 6 headers-only cases pass on rebuilt artifacts. The gate
selects the header-specific Network.enable occurrence, while the pinned oracle
uses its shared network manager. Broader concurrency/disconnect parity is still
unfinished. [Checkpoint](../../docs/knowledge/pwright-extra-http-headers.md#node-093-typed-header-initialization-retirement).

NODE-092 adds test:detach-retirement: a real native Worker-close race between
resume and detach, plus a live-worker preservation control. The fix is entirely
Rust; no extra feature is required. Full Worker/Node compatibility remains open.
[Build, command and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-092 targeted default/combined comparisons pass 2/2 each (combined repeated).
Serial child-header/attachment regressions pass 8/8 and 7/7. The overlapping gate
still fails at 4/8 child-header cases alongside 35/35 headers. Native tests pass
79 CDP, 373 composition and 274 host; this is not complete compatibility.

NODE-091 adds test:header-child-sessions, comparing native OOPIF and main-session
closure during public header setters with Playwright 1.58.2. Session policy and
bounded watches live in Rust; the relay is test-only. This remains an experimental
subset, and Page.isClosed is not implemented by this checkpoint.
[Build, command, evidence and exclusions](../../docs/knowledge/pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

NODE-091 final gate: 8/8 in both `drag-frame-extra-headers` and `extra-headers`,
plus one combined repeat. Existing headers pass 35/35 and attachment 7/7; Rust
host tests pass 274. Concurrent attachment/cleanup failures remain open, so
these artifact-specific passes do not establish full compatibility.

NODE-090 adds `test:attachment-retirement`, a real-Chrome public differential lane
for a Page disappearing during native connection initialization. Select and build
the desired native artifact first; the combined profile also covers lifecycle
Network/Page enabling. The rebuilt combined artifact passes 7/7 plus three 7/7
repeats, and headers pass 35/35. Rust CDP/composition/host tests pass 78/373/269
respectively. This remains an experimental subset, not complete compatibility.
[Command and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 final gate passes 35/35, with three further 4/4 receipt-gate repetitions.
An earlier attachment failure remains unresolved under target churn. Host units
pass 269; isolated native close reports zero owners. No test runtime/relay is
left running, and the existing foundation commit remains local and unchanged.

NODE-089 corrects Context Page-by-Page header dispatch and native protocol-error
messages in Rust. Four public success/failure comparisons hold real Chrome ACKs
while a newer setter proceeds, including deferred later-Page values. These run
inside the existing `test:extra-headers` lane; its diagnostic relay is test-only
and loopback-only. Full Node/cross-browser compatibility remains unfinished.
[Current evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 selected header builds use bounded Rust native fanout/jobs. A setter's
first failure settles before its retained compensation drains; the existing driver
continues polling jobs and lifecycle work. Combined public tests pass 31/31.
Controlled real-CDP timing, Worker/OOPIF handoff and shutdown retirement remain
unfinished; this is still an experimental subset.
[Current evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 selected header profiles now prepare setters and Page/Frame navigation
synchronously in Rust before scheduling native completion. Frozen native plans
preserve each invocation's values; combined release tests pass 30/30 and five
focused repetitions pass 6/6 each. Unpolled Rust future ordering and reclamation
also pass live. This fixes
the demonstrated logical ordering path, not native concurrent fanout or early
failure settlement. The package remains an experimental subset.
[Current behavior and evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 identifies an unresolved ordering bug: navigation immediately following
an unawaited header setter can see stale Page headers. The expanded combined lane
has 26 tests and retains this regression. Rust's optional overlapping model matches
128 internal upstream traces, but is not yet integrated into the host scheduler.
The rebuilt full lane passed 26/26 once; three focused repeats were pass/fail/pass.
The Frame ordering discrepancy remains unresolved in the current artifact.
[Current discrepancy and next work](../../docs/knowledge/pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 selected header builds reconcile `goto({ referer })` against the first
original Page Referer in Rust; Context headers do not cause option conflicts.
Combined header/drag/frame tests pass 24/24 and header-only passes 23/23.
The implicit referrer copy has a
16-KiB selected bound and eight admitted concurrent navigation owners.
[Current behavior and limitations](../../docs/knowledge/pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 adds `browser.newContext({ extraHTTPHeaders: { 'X-Example': 'value' } })`
to selected header profiles, composable with `baseURL`. Rust owns the complete
creation/settings transaction. Header-only and combined drag/frame/header releases
both pass 21/21 live tests, now
including creation language semantics, deferred syntax, recovery and admission.
Earlier notes below retain their historical scope; full compatibility is not claimed.
[Current evidence and remaining work](../../docs/knowledge/pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 corrects deferred HTTP-syntax validation and Page-initialization failure
isolation in selected Rust header builds. `pnpm test:extra-headers` now has 15
tests, including Context masking/clearing, recoverable failed Page creation and
18 failed-Page retirements. Fresh header-only and drag/frame/header artifacts
both pass 15/15; the latter checks Frame.goto after failure recovery. Creation's
`extraHTTPHeaders` option remains unsupported; this fixes prerequisites rather
than claiming that option or full failure/cancellation compatibility.
[Current evidence](../../docs/knowledge/pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 adds `Page.setExtraHTTPHeaders` and `BrowserContext.setExtraHTTPHeaders`
in independently selected native header profiles. Build and run with
`PWRIGHT_NATIVE_PROFILE=extra-headers`; `pnpm test:extra-headers` runs nine paired
public groups and one input-budget test (10/10 passing). The lane requires the
dedicated Chrome preflight and explicit CDP/Tailscale bind/base variables; it
owns a dynamic fixture. Rust handles merge, ownership, routing and native work.
Context creation's header option, navigation referer policy, native failure
fanout/timing, OOPIF/workers and full compatibility remain unfinished.
[Scope, budgets and commands](../../docs/knowledge/pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

Header combinations are `extra-headers`, `drag-extra-headers`,
`frame-extra-headers`, and `drag-frame-extra-headers`, in addition to the four
existing profiles. Each has a separate native path and all three compiled flags
must match. Only header-only was rebuilt/live-tested in this checkpoint; combined
Clippy and dependency gates do not establish combined live parity. Earlier NODE
entries below are historical checkpoints, not current profile coverage.

NODE-081 adds opt-in native Rust host header application, not public Node
`extraHTTPHeaders` setters or a packaged addon feature. Existing addon binaries
are unchanged. The dependency check now also verifies header-only/combined Rust
host selections and keeps headers absent from existing addon selections.
[Native tests and remaining public work](../../docs/knowledge/pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080 adds `scripts/check-header-settings-oracle.mjs`, a development-only
54-case comparison of Rust metadata transactions with pinned upstream setters.
It is not a public Node extraHTTPHeaders implementation; package API and retained
addon binaries are unchanged.
[Build/run instructions](../../docs/knowledge/pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-078 adds development-only header characterization and Rust policy
comparison scripts. Public `setExtraHTTPHeaders` and Context creation's
`extraHTTPHeaders` option are still unsupported; no addon was rebuilt.
[Commands, evidence and remaining integration](../../docs/knowledge/pwright-extra-http-headers.md).

NODE-077 expands `pnpm test:frame-navigation` with post-execution close and
timeout continuation: 27 paired cases match in both profiles, but the original
arrival-close case remains red (Rust also fails it intermittently). New
`pnpm test:frame-navigation-workers` passes 3/3 in each profile for owned pending,
borrowed pending and queued-result environment teardown. It uses the same
profile/CDP/base/bind variables and a diagnostic observer, not an upstream driver
in production. Run browser lanes serially; retained addon files are unchanged.
[Current evidence and limits](../../docs/knowledge/pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 exposes `frame.goto(url, { waitUntil?, timeout?, referer? })` in the
explicit `frame-navigation` native profile. It uses Rust navigation/lifecycle
logic and returns a Response whose frame() is the exact navigated wrapper.
Context baseURL is supported; extraHTTPHeaders/referer conflicts, exact error
text and the full compatibility goal remain unfinished. Both new profiles pass
23 paired Frame cases and independent Rust close, but fail an upstream
held-response Page-close diagnostic deadline; the full lane is not green.
The combined profile also passes the existing drag/Keyboard/Locator 10/10 lane.
[Current scope, resource bounds and verification](../../docs/knowledge/pwright-frame-navigation.md#node-076-public-node-frame-navigation).
Earlier NODE entries below describe their historical artifacts, not current
selected-profile coverage.

NODE-065 adds development-only `pnpm test:frame-lifecycle`. Build the Rust
`frame_lifecycle_trace` example with `frame-lifecycle`, then supply absolute
`PWRIGHT_FRAME_LIFECYCLE_BINARY` and optional unused absolute
`PWRIGHT_FRAME_LIFECYCLE_REPORT`. The 93/93 normalized-state comparison does not
expose public Frame.goto or rebuild the addon.
[Build command and verification scope](../../docs/knowledge/pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 adds the development-only `scripts/probe-frame-navigation.mjs` oracle
and Rust native-initiation comparison. Six local/nested/OOPIF observations pass;
this does not add public Frame.goto or rebuild either Node addon. Full navigation
still requires Rust lifecycle/cancellation and Frame-specific response identity.
[Setup and design](../../docs/knowledge/pwright-frame-navigation.md).

NODE-063 adds `page.frame(name | { name?, url? })`, including globs, caller RegExp
and URL predicates, backed by Rust selection. Rebuild the selected addon first;
the historical default binary does not implement it. `pnpm test:frame-lookup`
passes 41/41 observations and three native groups. Configure loopback
`PWRIGHT_TEST_CDP`, verified serve-lib `PWRIGHT_FRAME_LOOKUP_FIXTURE` directory
and unused absolute `PWRIGHT_FRAME_LOOKUP_REPORT`. Glob text is capped at 16 KiB;
native bindings/read bytes remain explicitly bounded. This is not full Playwright
parity; Frame.goto, Frame events and connection reliability remain open.
Same-addon sequential typing passes 38/38 using sequential-typing.html.
[API scope and verification](../../docs/knowledge/pwright-frame-lookup.md#node-063-public-lookup-over-the-rust-kernel).

NODE-062 browser.newContext({ baseURL }) and Page.goto relative URL resolution
now pass 16/16 Playwright 1.58.2 observations plus four native boundary groups.
Context enumeration is cache-aware; pwrightContextMetadataMetrics() exposes its
separate ledger. Final-addon Page 7/7 and native glob 1647/1647 plus seven checks
also pass. Other Context options, Frame.goto and public page.frame remain
unsupported; connection reliability is still under investigation.
[API scope, quotas and verification](../../docs/knowledge/pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061: pnpm probe:attachment-commands runs a prebuilt attach_probe test binary
asynchronously across observer layouts. Supply its absolute
PWRIGHT_ATTACHMENT_TRACE_BINARY, loopback PWRIGHT_TEST_CDP and an unused absolute
PWRIGHT_ATTACHMENT_COMMAND_REPORT. The unchanged addon now passes original-layout
glob 1647/1647 and Page 7/7; this is not a fix for intermittent connection failures.
[Build command, limits and evidence](../../docs/knowledge/pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 adds internal native glob conversion and pnpm test:glob-ffi. It requires
PWRIGHT_CDP_ENDPOINT, an unused absolute PWRIGHT_GLOB_FFI_REPORT, and the rebuilt
selected addon. A labelled PWRIGHT_GLOB_OBSERVER_SESSION=after run passes 1647
patterns and seven boundary checks. Default before-layout and Page regressions
still fail during attachment; page.frame and Context baseURL remain unfinished.
[Boundary, setup and evidence](../../docs/knowledge/pwright-frame-lookup.md#node-060-actual-n-api-url-value-conversion-port).

NODE-059: `pnpm test:glob-base` compares 1647 Rust URL-glob patterns against
Playwright 1.58.2. Optional `PWRIGHT_GLOB_BASE_REPORT` must be an unused absolute
path. This offline helper verifies preprocessing with a diagnostic Rust parser,
not public `page.frame` or production parser allocation admission. Native
binaries remain unchanged.
[Contract and remaining gates](../../docs/knowledge/pwright-frame-lookup.md#node-059-bounded-glob-preprocessing-and-borrowed-url-resolution).

NODE-058: `pnpm test:glob-plan` builds/runs a bounded Rust-only helper and
compares 2810 resolved-glob patterns against Playwright 1.58.2. This is not public
`page.frame` support; N-API lookup, baseURL and identity integration remain open.
Future native builds include host MIT/Apache license texts and upstream notices
alongside the selected binary. Existing binary bytes were not changed here.
[Implementation and attribution](../../docs/knowledge/pwright-frame-lookup.md#node-058-rust-traversal-and-bounded-resolved-glob-compiler).

NODE-057: `pnpm test:lookup-oracle` runs 14 pinned-client characterization tests;
`page.frame(...)` is still unimplemented. [Design and remaining gates](../../docs/knowledge/pwright-frame-lookup.md).
Mouse now passes 44/44 on the unchanged addon in both original and experimental
`PWRIGHT_MOUSE_SETUP=background` modes. Original remains the default; the
earlier intermittent extra events are not explained by these successful reruns.
[Diagnostic scope and evidence](../../docs/knowledge/pwright-raw-mouse.md#node-057-setup-experiment-not-a-causal-fix).

NODE-056 adds `frame.name()` as a Rust-published navigation snapshot. Rebuild
and select `PWRIGHT_NATIVE_PROFILE=drag-runtime`; historical addons return an
explicit unsupported/rebuild error. `pnpm test:frames` now passes 18/18 shared
local/OOPIF cases, including names retained after detach/close. `page.frame`
lookup and Frame events are not yet supported.
Current Mouse regression has unexplained extra movement events and is not green;
the successful Frame gate must not be read as whole-package parity.
[Setup, bounds and evidence](../../docs/knowledge/pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 adds admitted, cache-aware Page enumeration and explicit Rust-backed
transient activation for Page/Frame `evaluate`. The shared `pnpm test:pages`
gate passes 7/7, including popup discovery and retained closed Pages; repeated
enumeration does not accumulate native wrappers. The creation-pressure gate
now includes four states. Default historical addon bytes remain unchanged.
[Setup, bounds and evidence](../../docs/knowledge/pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 pre-admits new-Page root bindings in Rust, fixing a reproduced native
target leak under retained-Frame pressure. `pnpm test:page-binding` checks that
resource rejection creates no target and restores metadata charges. The rebuilt
selected addon passes Frame 10/10, Mouse 44/44 and drag/Locator 10/10.
`pnpm probe:attachment` is a bounded diagnostic matrix, not a passing parity
gate; earlier intermittent connection timeouts remain under investigation.
[Setup, artifacts and evidence](../../docs/knowledge/pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 adds `page.frames()`, `frame.parentFrame()`, `frame.childFrames()` and
`frame.isDetached()`. Rebuild with `PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm build`
and select that same profile when running. The old default binary remains
untouched and explicitly rejects these new methods until rebuilt. The shared
`pnpm test:frames` gate passes 10/10 local/OOPIF lifecycle observations; names,
lookup and Frame events are still unsupported. `browser.pwrightFrameTreeMetrics()`
reports retained native states, revisions/read guards and logical bytes.
[Current API and verification](../../docs/knowledge/pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 fixes owned-Page OOPIF attachment in Rust and rebuilds only the selected
`drag-runtime` artifact. Public owned-OOPIF navigation now succeeds; Mouse 44/44,
Keyboard 24/24 and drag/Locator 10/10 pass on that exact binary. The host now
maintains bounded Frame trees, but `page.frames()` and public child-Frame wrappers
remain unsupported. The default addon is unchanged.
[Artifact and native evidence](../../docs/knowledge/pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-050 adds the opt-in `PWRIGHT_NATIVE_PROFILE=drag-runtime` local build and
loader. Build with that environment variable; the default binary is preserved.
A Rust capability export verifies the selected artifact on first native use.
Actual Node results: Mouse 44/44, Keyboard 24/24, drag/Locator 10/10. The profile
remains experimental, and public child-Frame APIs are still unsupported.
[Selection, evidence and remaining work](../../docs/knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 fixes Rust-host Frame input lifetime and verifies 12/12 native cases.
The package addon remains unchanged and does not select `drag-runtime`; these
are not new public Node child-Frame or drag-compatibility passes.
`pnpm test:boundary` now checks default, drag-source and drag-runtime closures
and their resolved runtime feature selection, as well as the thin JS boundary.
[Contract, evidence and remaining gates](../../docs/knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 adds internal Rust-host native child/OOPIF tests, not public child-Frame
APIs. Eight of ten cases match; post-focus iframe removal still cancels Rust
input too early. The addon remains unchanged and does not select drag.
[Open gate](../../docs/knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 verifies ten selected Rust-host drag/Keyboard/Locator scenarios.
This package's addon is unchanged and still does NOT enable `drag-runtime`.
The NODE-045 public Mouse result remains 39/44, not ten new public Node passes.
[Selection gates](../../docs/knowledge/pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).

NODE-046 adds selected Rust-host Keyboard Escape cancellation, verified against
five native oracle cases. This package's addon still does NOT select drag and
was not rebuilt; NODE-045 public Mouse evidence remains 39/44.
[Remaining selection gates](../../docs/knowledge/pwright-chromium-drag.md#still-required-before-public-drag-selection).

NODE-045 rebuilds this package's default addon with 552 pending CDP commands,
derived from admitted Mouse batches plus the existing non-Mouse allowance.
Public Mouse is 39/44; all 25 raw cases match. `drag-runtime` is still unselected.
[Exact artifact and remaining gates](../../docs/knowledge/pwright-chromium-drag.md#node-045-verification-and-actual-node-artifact).

NODE-044 implements an opt-in `pwright-node-host/drag-runtime` integration, but
this package's addon still uses the default host. Do not infer Node drag support
from host-native tests; public Mouse evidence remains **36/41**.
[Host selection gates](../../docs/knowledge/pwright-chromium-drag.md#remaining-compatibility-gates).

NODE-043 adds an opt-in host `drag-source`: fresh Page utility snapshots now
remain owned through movement results, retirement and quarantine. Exact world
destruction drains started listener requests without issuing new cleanup there.
Default/selected host closures are **234/235**; only the drag leaf is added.
HostSession Mouse/close/Escape selection and public differential checks remain
pending; the addon and public **36/41** baseline are unchanged.
[Production source and evidence](../../docs/knowledge/pwright-chromium-drag.md#node-043-retained-production-page-drag-source).

NODE-042 adds immediate, budgeted snapshots of a Page's existing utility worlds,
including child Frames/OOPIFs without waiting for a main world. Utility-only
native destruction receipts retain snapshot admission and remain distinct from
route invalidation and owned-session retirement. Production drag-source/host
integration is still pending; no addon rebuild or public **36/41** change.
[Snapshot ownership and verification](../../docs/knowledge/pwright-chromium-drag.md#node-042-existing-page-utility-world-snapshots).

NODE-041 implements the explicit `mouse-dispatch` feature and Page-owned cleanup
driver: **82** unit tests and **4** selected-Mouse native cases pass, alongside
the prior **10** native cases. Default/feature closures are **3/5** Pwright
packages. Host Frame-source/lifecycle and Keyboard Escape integration remain
pending; no addon rebuild or public **36/41** change.
[Selected dispatcher and evidence](../../docs/knowledge/pwright-chromium-drag.md#node-041-selected-mouse-dispatcher-and-page-cleanup-driver).

NODE-040 adds Page-owned drag state and retained native enter/over/drop/cancel.
**70** unit tests and **10** native scenarios pass, including actual drop and
cancel with dragend checks. The optional leaf remains **3** Pwright packages;
Mouse dispatch/forClick/down suppression, Keyboard Escape and Node host selection
are still pending. The addon/public **36/41** evidence is unchanged.
[Page state and verification](../../docs/knowledge/pwright-chromium-drag.md#node-040-page-drag-state-and-native-event-owners).

NODE-039 adds composed intercepted movement to the optional Rust leaf (**57**
unit tests; **9** native cases). It is not selected by this Node package. Page
drag state, Mouse adaptation and host teardown remain pending; the addon and
public **36/41** result are unchanged.
[Source checkpoint](../../docs/knowledge/pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 adds retained subscription/interception ownership in the optional Rust
drag leaf, with **44** unit tests and **6** native listener/capture cases passing.
It is not selected by this package; the addon and public **36/41** evidence are
unchanged. [Source checkpoint](../../docs/knowledge/pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 adds bounded typed drag-data storage to the optional Rust leaf, not this
package's active host. Unit and native data-flow checks pass; the addon is not
rebuilt and public Mouse remains **36/41**. Capture lifecycle and Page-scoped
integration are still pending.
[Source checkpoint](../../docs/knowledge/pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036 adds an optional Rust-owned drag-listener helper, verified on real
Chrome. It is not installed in this package; the addon is unchanged and HTML
drag/public concurrency compatibility remains incomplete.
[Native helper and remaining integration](../../docs/knowledge/pwright-chromium-drag.md).

The native factory and Page route now support a selected Mouse dispatcher
(NODE-035), but the host still uses its direct binding. A real Page-scoped drag
provider and host teardown integration remain pending. No addon is rebuilt or
public discrepancy resolved by this checkpoint.
[Current source integration](../../docs/knowledge/pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

The Rust input scheduler now exposes an optional cancellation-aware Mouse
dispatch seam (NODE-034). It is not selected by the public host and no addon
is rebuilt for that source checkpoint. It does not resolve the drag differences
below. [Ownership and remaining integration](../../docs/knowledge/pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

The expanded same-turn Mouse corpus is **not green**: **33/41** on the prior
addon, **36/41** after Rust first-poll ordering. Five left-held-move cases still
differ because the optional Chromium drag path is missing. The original **26/26**
below covers only the earlier corpus. Do not serialize the whole click or add JS
delays to make these cases pass. [Current concurrency checkpoint](../../docs/knowledge/pwright-raw-mouse.md#same-turn-input-ordering-node-033).

Final ordering addon `e65e6d85...17b246`: Mouse **36/41**, Mouse Worker/profile
**3/3**, Keyboard **24/24**, typing **30/38**, actions **24/27**, screenshot
**12/12**, lifecycle **45/45**. The typing/action failures are attachment timeouts
on both providers; do not carry over the intermediate artifact's green results.
Reload/shared were not rerun on this final artifact. [Exact gates](../../docs/knowledge/pwright-raw-mouse.md#final-artifact-gate).

Earlier release addon `e9d9e543...109535` introduced stable `page.mouse` with
move/down/up/click/dblclick/wheel; all input state and scheduling remain in Rust.
Public comparison: **26/26**. Profile rejection and owned/borrowed Worker teardown:
**3/3**, including no synthetic release on the surviving borrowed Page.
Run `pnpm test:mouse` with explicit dedicated `PWRIGHT_TEST_CDP`, verified serve-lib
`PWRIGHT_MOUSE_FIXTURE` and a new absolute `PWRIGHT_MOUSE_REPORT`; use
`pnpm test:mouse-native` for the profile/Worker lane.
[Limits, artifact and reproduction](../../docs/knowledge/pwright-raw-mouse.md#public-node-mouse-checkpoint).
Host dependency closure remains **234**. Nested allocation audit, HTML drag,
exact error fidelity and full artifact-specific regression remain open. This
does not imply real Touchscreen support.

On this Mouse addon: Keyboard **24/24**, typing **38/38**, actions **27/27**,
screenshot **12/12**, lifecycle **45/45**, reload **16/18**. Shared 57 cases:
**53 match / 2 unsupported / 1 oracle-error / 1 setup-error**. The renewed 5 s
topology-attach timeout and both original reload-close failures remain open;
overall compatibility is red. [Exact evidence](../../docs/knowledge/pwright-raw-mouse.md#same-artifact-regression).

Full rerun on the Keyboard addon: **38/38 typing**, **27/27 actions**,
**12/12 screenshot**, **45/45 lifecycle**, **16/18 reload**. Held-modifier typing
entries are no longer filtered out. Two original reload-close failures remain;
intermittent attachment/window issues were not reproduced or proven fixed.
[Exact evidence](../../docs/knowledge/pwright-raw-keyboard.md#same-artifact-post-keyboard-regression).

Previous Keyboard addon `1319fa02...ded25dc1` exposes `page.keyboard.down/up/press/
insertText/type` with Rust-owned Page state, routing, delay and cancellation.
Public comparison is **24/24** and shared coverage **52 match / 3 unsupported /
2 oracle-error**, no other errors. Run `pnpm test:keyboard` with dedicated
`PWRIGHT_TEST_CDP`, verified serve-lib `PWRIGHT_INPUT_FIXTURE`, and an absolute
unused `PWRIGHT_KEYBOARD_REPORT` file. Key/text limits are 128 bytes/1 MiB,
physical key capacity 16, and unpaired UTF-16 rejects explicitly.
[Artifact and budgets](../../docs/knowledge/pwright-raw-keyboard.md).
Earlier artifact-specific reload/lifecycle/screenshot results below remain
historical; the full gates were not rerun by this Keyboard checkpoint.

Playwright-shaped ESM/CommonJS API backed by the Pwright Rust v2 host through
Node-API. No Playwright runtime dependency, Node driver subprocess, browser
download, or browser launch. This package is private and not published.

`page.reload({ waitUntil, timeout })` now invokes Rust navigation 1.1, with a
separate native Reload command and the existing FFI admission/publication path.
Both original shared reload scenes match: corpus **48 match / 7 unsupported /
2 oracle-error**. The new public reload lane is **16/18** (response-arrival close
fails on both sides; the separate post-commit close passes), and lifecycle is
**44/45** (window geometry).
The same artifact's screenshot gate is **0/12**, failing at attachment before
image comparison; earlier green screenshot runs do not supersede this result.
Ordinary/POST/timeout success is not complete lifecycle compatibility.
Run `pnpm test:reload` with CDP/base and `PWRIGHT_TEST_BIND` as for `test:live`.
[Contract, exact artifact and red evidence](../../docs/knowledge/pwright-page-reload.md).
The separate `scripts/probe-reload-boundary.mjs` launches a test-only Rust host
diagnostic with explicit CDP/Tailscale settings. Its four native controls track
bounded request/loader events and target presence; a successful diagnostic run
does not mean reload-close compatibility passes.
[Commands and causal findings](../../docs/knowledge/pwright-page-reload.md#causal-native-close-diagnostic).
Pass `raw` to run the standalone CDP reproducer with no host or addon, additionally
setting the verified loopback `PWRIGHT_CDP_WS_URL`. It retains the original fixture
and separately samples native target presence; it is not a workaround or a parity
pass. [Raw isolation](../../docs/knowledge/pwright-page-reload.md#minimal-raw-cdp-isolation).

`Locator.pressSequentially(text, { delay, timeout })` and deprecated
`Locator.type()` call the Rust typing capability. Node explicitly selects
interleaved Page input state; the new artifact passes **30/30 typing + 27/27
action** tests, including overlap and the prior delayed-Press Shift mismatch.
Physical state survives command timeout instead of automatically releasing all
keys. It remains fixed at 16 physical codes per Page; unpaired UTF-16 rejects.
These APIs do not claim full keyboard, native cancellation or string compatibility.
[Typed contract, limits and evidence](../../docs/knowledge/pwright-sequential-typing.md).

Current screenshot checkpoint: `page.screenshot()` returns a Rust-captured,
Node-owned Buffer. PNG/JPEG, quality, fullPage, clip, scale and timeout are exposed;
the recursive-attachment artifact now passes all 12 decoded-pixel/resource/Worker
tests and all 45 lifecycle tests. The host has an eight-call finite
queue and a 2 MiB image ceiling. Path, rendering-state options, combined fullPage/
clip and default font/caret fidelity remain incomplete. After the dedicated
Chrome/Pwright preflight and fixture setup, run `pnpm --dir packages/pwright
test:screenshot` with the same `PWRIGHT_TEST_CDP` and `PWRIGHT_TEST_BASE` variables
as the other live lanes. [Screenshot evidence and limits](../../docs/node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

Prior checkpoint: owned-session retirement is wired into host shutdown.
The full 23-case Locator evaluation lane and native diagnostic supervisor pass,
including preservation of a borrowed Page and its running Promise when a Worker
terminates (NODE-027). Compatibility is still incomplete: NODE-013 attach timing,
NODE-014 window dimensions, NODE-022 keyboard behavior and unsupported APIs remain.
The latest lifecycle attempt failed a window comparison and reached its unchanged
total deadline. [Current evidence and resource contract](../../docs/node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

## Previous checkpoints (historical)

Complete compatibility is **not green**. Previous host-owned rebuild `8cc20b9e...6aad0` passes
Locator 32/32, evaluation 38/38 and lifecycle 45/45; actions are 25/26 (NODE-022).
All 57 shared cases finish with 41 matches / 14 unsupported / 2 oracle errors,
and no setup/value/candidate/cleanup errors. The safe host now owns Locator
selection/adoption and delayed conversion inside one controlled command, but
its initial native Context-close cleanup failure (NODE-026) is now fixed with
optional native-destruction receipts. Two earlier safe-host Chrome runs pass 5/5;
the final epoch-guarded run is 4/5 due to a 5 s attach setup failure before queue
assertions. Context cleanup still passes; an isolated queue diagnostic passes
without replacing that failed gate. Connection timing remains unresolved.
Public Node `Locator.evaluate` is now implemented over that Rust host, including
delayed source/getter conversion and binary staging. Its dedicated lane passes
22 assertions but still fails the native diagnostic gate when terminating a
Worker during a never-resolving browser evaluation (NODE-027). It is experimental,
not a fully verified compatibility claim. No silent retry or Chrome restart was used.
[Current FFI evidence](../../docs/node-ffi-compatibility-design.md#public-locatorevaluate-ffi-checkpoint-2026-09-07).

On this new addon, existing evaluation/Locator/lifecycle regressions pass
38/38, 32/32 and 45/45. All 57 shared cases now finish with 43 matches /
12 unsupported / 2 oracle errors; both label observations newly match. That
shared run has no native cleanup failure, but it does not exercise or clear
NODE-027. No complete-compatibility or memory-improvement claim is made.
[Evidence and next gates](../../docs/node-ffi-compatibility-design.md#native-destruction-receipt-checkpoint-2026-09-07).

## Build and test

Final native-retirement rebuild `930469ad...a7a4a3` passes 442 Rust release
tests, but its public lanes remain non-green: Locator 29/32, evaluation 17/38,
lifecycle watchdog after 11 passes / 5 failures, actions 25/26 (NODE-022).
The full shared corpus is 41 matches / 14 unsupported / 2 oracle errors, with
zero value/candidate/setup/cleanup errors. Attachment failures affect both
implementations and remain unclassified; the shared subset does not replace
the failed public lanes. That historical rebuild predates public Locator.evaluate.

From this directory, with Rust and Node installed:

```sh
pnpm install --frozen-lockfile
pnpm build
pnpm test
pnpm test:types
pnpm test:boundary
```

The default build compiles a release addon and atomically replaces
`native/pwright.node` (gitignored). To build the experimental selected profile
without replacing that file:

```sh
PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm build
PWRIGHT_NATIVE_PROFILE=drag-runtime node your-script.mjs
```

For Frame navigation without drag, use `PWRIGHT_NATIVE_PROFILE=frame-navigation`
for both build and execution. For both capabilities, use
`PWRIGHT_NATIVE_PROFILE=drag-frame-navigation`. Each profile has a separate
`native/<profile>/pwright.node`, and both compiled flags must match the selected
profile. Default and drag-only files are not overwritten by these builds.

`pnpm test:frame-navigation` uses repository `../../tests/fixtures` (not this
package's fixture directory), registered via serve-lib. Set the selected native
profile, loopback `PWRIGHT_TEST_CDP`, verified full-MagicDNS `PWRIGHT_TEST_BASE`
and explicit Tailscale `PWRIGHT_TEST_BIND`. The pinned oracle is development-only.

This writes `native/drag-runtime/pwright.node`; the same environment selection is
required when running it. Selection is fixed when the package is first imported.
Unknown profiles, missing selected binaries and mismatched compiled capabilities
fail; there is no download or fallback to another profile. With the documented
dedicated Chrome preflight and verified serve-lib fixture/report variables set,
`PWRIGHT_NATIVE_PROFILE=drag-runtime pnpm test:drag-keyboard` runs the new shared
native drag/Keyboard/Locator Node comparison. It needs `PWRIGHT_TEST_CDP`,
`PWRIGHT_DRAG_FIXTURE` and a new absolute `PWRIGHT_DRAG_KEYBOARD_REPORT` path.

Local validation used Node 26.7.0 on macOS arm64. The intended
Node >=20 range and other platforms are not yet verified.

For the real-browser comparison, install the repository's pinned oracle first:

```sh
pnpm --dir ../../tests install --frozen-lockfile --ignore-scripts
chrome-cdp start
chrome-cdp status
```

Register `test/fixtures` with the existing `serve-lib` daemon using an explicit
Tailscale bind IP, route, port and lifetime. Verify the exact full-MagicDNS URL
with `serve-lib list` and `curl -I`. Static HTML must not use another server.
For example, this Mac's verified fixture route is
`http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-node-actions` while its registration
is active. `PWRIGHT_TEST_BIND` is the verified Tailscale IP; it is used only for
ephemeral dynamic stalled-response test endpoints, which are closed afterward.

In a task-specific working directory, run the mandatory initial Pwright open
against that fixture URL:

```sh
pwright --cdp http://127.0.0.1:9222 open http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-node-actions/index.html
```

Then return to this package and run:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_TEST_BASE=http://macbookpro-yuacx.tail8f3b66.ts.net:9384/pwright-node-actions \
PWRIGHT_TEST_BIND="$TAILSCALE_IP" pnpm test:live
```

The same CDP/base configuration also runs `pnpm test:evaluation`,
`pnpm test:locator`, `pnpm test:locator-evaluation` and `pnpm test:actions`
(no dynamic bind variable needed). Locator evaluation covers delayed conversion,
timeout scope, error identity, binary copying and four Worker teardown phases.
Its supervisor rejects native cleanup diagnostics even if every TAP assertion passes.
The Locator supervisor enables explicit GC only in its test child. All package
lanes use TAP so failure details survive a later supervisor deadline kill.
The lifecycle lane has a 600-second aggregate watchdog and an independent
60-second bound for each case (setup/action/cleanup), startup and process exit.
Other lanes retain 180 seconds total. Original API deadlines stay unchanged.
Bounded start/end IPC drives the case watchdog; fixture-phase TAP comments cannot
renew it. At most 128 cases, 4096 phase records and 4 MiB stdout are accepted;
stderr diagnostics and native panic/cleanup failures remain supervised.
The unchanged interleaved-input addon now completes all **45/45 lifecycle cases
in 192.715 seconds** with this policy; the prior traced 180-second run was killed
after 44 passes. Earlier attachment/window failures remain open.
[Evidence](../../docs/node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).
Run all package lanes and the shared corpus serially against one CDP endpoint;
separate working directories isolate Pwright state, not browser-wide target
discovery. Overlapping supervisors produced missing-session/timeout failures
while attaching, recorded in the paired-world access checkpoint. Serialization
is the test protocol, not a fix or a compatibility claim for concurrent clients.
The preceding concurrency addon passed Locator 22/22, lifecycle 44/44 and
evaluation 38/38. The action-stage addon then reproduced setup timeouts and a
ten-read close/fence overflow (NODE-023); the latter now has a scoped Rust fix
and completed Locator 22/22, evaluation 38/38 and lifecycle 44/44 reruns on
`e5076407...65d48`. Actions remain 22/24 with NODE-021/022. These are supported subsets, not full
Playwright gates. Artifact-specific evidence and retained failures are in the
[Node design record](../../docs/node-ffi-compatibility-design.md).
No failures are skipped or silently retried.

The preceding topology rebuild `72644cf9...b9ddcd` completes a serial run of Locator **32/32**,
evaluation **38/38**, lifecycle **44/45** (NODE-014) and actions **25/26** (NODE-022).
The unchanged shared corpus completes **41 matches / 14 unsupported / 2 oracle
errors**, with zero value/candidate/setup/cleanup errors and no supervisor
timeout or native failure. Lifecycle, action and shared gates remain nonzero.
An earlier overlapping package/shared run on this same binary failed with
additional attachment errors: Locator 27/32, evaluation 34/38, lifecycle 43/45
and shared 11 setup errors. Both runs are retained; a serial pass does not
resolve concurrent attachment or the older navigation discrepancies.
See the [preceding artifact evidence](../../docs/node-ffi-compatibility-design.md#production-paired-world-access-checkpoint-2026-09-07).
The independent Rust strict-wait and main-world adoption/evaluation leaves are
now installed by the safe host, whose normal closure is 224 packages. Node
`Locator.evaluate` was unavailable at that checkpoint; it is now exposed with
correctly timed FFI conversion and an open running-cancellation cleanup gate.

At the earlier Rust-only checkpoint, paired preparation passed release and real Chrome
gates, including retained route ownership through cancelled native cleanup.
It then remained outside this package's 221-package normal closure. That
checkpoint did not rebuild the addon or rerun the five full browser lanes; it
is historical evidence, not the current installation. See
[the preparation record](../../docs/node-ffi-compatibility-design.md#paired-element-preparation-checkpoint-2026-09-07).

This explicitly authorized loopback CDP endpoint controls the dedicated Chrome
automation profile, never a normal user profile. Tests do not close preexisting
pages. Test cases create and close their own fixture pages. Close the preflight
page using `pwright --cdp http://127.0.0.1:9222 close` in its unchanged task
working directory after verifying its active target is the owned preflight. Verify
that the target is absent from CDP discovery afterward. Do not reset
the shared automation browser. The oracle is pinned to Playwright 1.58.2 by the
existing `tests/pnpm-lock.yaml`, and a different version fails the lane.

## Supported today

### Attachment diagnostics

After the same dedicated Chrome/Pwright preflight, run from the repository root:

```sh
CARGO_INCREMENTAL=0 cargo test --release -p pwright-node-host --test attach_probe --no-run
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm --dir packages/pwright exec node scripts/probe-attach.mjs
```

For the separate retained-world investigation:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm --dir packages/pwright exec node scripts/probe-retained-worlds.mjs
```

This probe creates, reloads and closes one owned blank Page, then disconnects its
own connections. It observes bounded context metadata on that Page across six
fixed candidate/oracle attachments; browser-wide attachment can still initialize
other existing targets. It stops on failure without retry and does not change
the five-second connection timeout. A completion record proves that reload
created a different main world. Context counts do not measure heap/RSS or prove
init-script retention. The current result shows one reused Pwright world and
three added oracle worlds across three reconnects each, with no NODE-013 root
cause claim. See [the evidence and limits](../../docs/node-ffi-compatibility-design.md#node-013-retained-world-diagnostic-2026-09-07).

The original attachment diagnostic compares Node main-thread, Worker and independent Rust
host attachment while one pinned Playwright connection remains open. It does
not create or close Pages, retry a failed attach, raise its five-second budget,
or change the conformance suite. Cleanup and auxiliary CDP observations have
explicit diagnostic deadlines. A failure keeps the command nonzero.

The Rust probe records at most 32 unanswered commands and the latest 128 replies,
with method/session/target names capped at 128 bytes, plus counts for at most 64
event methods and 64 deduplicated session-to-target bindings. A separate counter records pre-admission omissions
without mixing them into delivered event counts. It retains no console, script
or page-content payloads. Its subscriber is test-only; ten offline tests verify
recording bounds, conflicting bindings, method counts and omission separation.

A separate prerequisite-only ABBA diagnostic compares widths 1/2/2/1 with no
production scheduling change. After prebuilding `--test attach_pair_probe
--no-run`, supply `PWRIGHT_TEST_CDP` and `PWRIGHT_TEST_WS` (the exact dedicated
browser WebSocket reported by `chrome-cdp status`) to
`pnpm --dir packages/pwright exec node scripts/probe-attach-pairs.mjs` from the
repository root. It only enables session-local Page/Runtime domains and reads
frame metadata on two to eight existing Pages; it detaches its own sessions and
never creates worlds, navigates or closes Pages. It is not a full-host/Node
benchmark. The first fixed experiment showed no improvement with one slow Page:
serial 555–567 ms, paired 587–591 ms. [Evidence and limits](../../docs/node-ffi-compatibility-design.md#node-013-target-correlation-and-bounded-pair-experiment-2026-09-07).

NODE-013 reproduced outside Node-API too. The first standalone host probe took
64 ms; with the oracle connected, both Node lanes and the independent Rust host
hit five seconds. A controlled TCP_NODELAY experiment showed no consistent
improvement and was reverted. The latest paired-prerequisite diagnostic reports
roughly 1.7–1.9-second overlapping attachments with three Pages. Command traces
prove the two prerequisites overlap; changing live activity prevents a controlled
performance attribution. The full regressions below were independently rerun;
neither a passing probe nor those tests proves full compatibility or root cause.
See the [diagnostic record](../../docs/node-ffi-compatibility-design.md#node-013-attachment-diagnostic-checkpoint-2026-09-06).

### Compatibility checkpoint

The recursive-attachment addon (`31333abc…45cdd9`) passes **12/12 screenshots**,
including both Worker settlement/Buffer cases, and **45/45 lifecycle tests**.
The full shared corpus completes with **45 match / 10 unsupported / 2 oracle-error**
and no value/candidate/setup/cleanup errors; its supervisor remains non-passing.
Earlier 3/12, 8/10 and 10/10 results belong to different artifacts and remain in
the record. [Current verification](../../docs/node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

This host now explicitly joins recursive auto-attach with Frame Tree, Runtime
enable and empty utility-world registration. Context settings/Page enable remain
earlier; all replies precede world creation, resume and Page publication. Other
Rust profiles keep their previous order. The extra declared response expansion
is 16 MiB, not a measured RSS increase or an allocated buffer. Three-Page live
traces prove command overlap but changing browser activity prevents a controlled
speedup claim or closure of intermittent NODE-013.

An earlier Node profile opted into a bounded Frame Tree/Runtime initialization
pair. Default Rust consumers remain serial; utility worlds, Context defaults and
PageReady retain their fences. This adds one declared response expansion, not a
Node driver or task. Test fixtures now pass their owner's URL to Workers and
probe titles only at that URL, retaining the unique/readable marker assertion.
See the latest Node design record for verification; prior full-gate failures
remain evidence until explicitly superseded by a completed rerun.

The package-local lifecycle/resource lane above is separate from the broader
[shared API corpus](../../tests/README.md). From `tests/`, `pnpm test:node-ffi`
runs all 19 existing API suites / 57 cases on fresh owned Contexts, using this
package's public API without a Playwright or JavaScript browser-logic fallback.
The preceding action-stage addon (`eae32ce9...41af9e`) completed the shared run with
36 matches, 19 unsupported cases and 2 official oracle history-navigation
errors (NODE-015), with no value mismatch or candidate/setup/cleanup errors.
Locator reads added ten matches over the earlier evaluation 12/43/2 checkpoint;
public actions add another fourteen over the read/concurrency 22/33/2 result.
The final terminal-completion addon (`e5076407...65d48`) completes all 57 cases
with 31 matches, 19 unsupported, two oracle errors and five attachment setup
errors. No current setup failure inherits an earlier artifact's passing result.
It intentionally exits nonzero; unsupported and oracle-error
cases are not passes. Artifact hashes and prior failures remain in the design record.
Earlier package-local passes do not mean all of Playwright is supported.

## Public subset

With the explicit handles native profile, Page/Frame.evaluateHandle and
JSHandle.evaluate/evaluateHandle/jsonValue/getProperty/dispose preserve native
object identity; ElementHandle.asElement distinguishes DOM results. getProperty
preserves the property's type, including a Promise, unlike evaluateHandle's
awaited evaluation result. The new producer lane is intentionally non-green
until getProperties has an atomically owned batch path. Preview/toString,
ownerFrame/adoption and retained ElementHandle actions/uploads remain absent.

```js
import { chromium } from '@shuozeli/pwright';
// CommonJS: const { chromium } = require('@shuozeli/pwright');

const browser = await chromium.connectOverCDP('http://127.0.0.1:9222');
try {
  console.log(browser.version());
  const context = browser.contexts()[0];
  const page = await context.newPage();
  try { console.log(page.mainFrame().page() === page); } // true
  finally { await page.close(); } // close exactly the Page this code created
} finally {
  await browser.close(); // disconnects; does not terminate external Chrome
}
console.log(browser.pwrightMetrics()); // Pwright extension: teardown counters
```

The endpoint-object overload (`{ endpointURL, timeout }`) is also supported.
Browser, Context, Page and main Frame identity are stable while live. Repeated
or concurrent browser close is idempotent and emits `disconnected` once.

No-options `BrowserContext.newPage()` and ordinary `Page.close()` are supported,
including `runBeforeUnload: false`. Creation, metadata admission, native target
initialization and close all execute through existing Rust services. Explicit
lifecycle calls share an eight-entry serial queue with topology fences. A new
Page reserves its main Frame's metadata entry and URL bytes before creating a
native target. Retained closed wrappers still consume that metadata admission.
Close suppresses only target-closed errors, not capacity/backend failures.

`Browser.newContext()` / `newContext({})` now create an isolated owned Context;
`context.close()` disposes its native Context and descendants. Repeated and
concurrent close are idempotent. Closing the borrowed default Context instead
disconnects this attachment, matching Playwright CDP without closing external
Chrome. Browser disconnect and forced Worker teardown remove the exact owned
native Context/Page, verified through Chrome's inventory.

Owned Contexts select a Rust pre-resume initializer for the default 1280x720
viewport/screen, desktop scale, native window size, focus and media preferences.
The initial document records these values before an observer could alter them.
Non-empty Context options, accepted-download/artifact initialization and
headless-font defaults are not implemented; see NODE-014. This is not full
default/options parity merely because `newContext()` returns successfully.

Like Playwright's CDP attachment, this facade attributes Pages from Contexts
unknown to the connection to its default Context. This does not acquire or own
those foreign Contexts. Do not use that view as evidence of a private Context
boundary. The reusable Rust topology driver retains strict exclusion by default;
only this compatibility host explicitly enables the fallback behavior.

`Page.goto` now supports `commit`, `domcontentloaded`, `load`, and `networkidle`;
the returned response exposes URL/status/status text/headers/ok/frame metadata,
not bodies, request objects or completion of body streams. `Page.title/content`
and main Frame `title/content` run in Rust. Timeouts and browser-close cancellation
use the controlled v2 dispatcher and bounded cleanup executor.
Worker teardown joins Rust cleanup before Node destroys its runtime. The live
suite verifies that terminating a Worker during navigation cancels its pending
HTTP request while leaving the external browser alive. Native cleanup failures
fail the runner even when Node assertions pass. A prior rebuilt 44-case lane
passed all cases, including native URL metadata, prototype interference and
concurrent DOM reads, blocked-JS native-result pressure (over 800,000 UTF-8
bytes), independent close admission, undelivered-result Worker teardown and
URL visibility while the JS event loop is blocked, plus queued-URL reclamation
when a Worker is forcibly terminated. Lifecycle cases additionally compare
initialized new Pages, concurrent creation, idempotent close after external
closure/disconnection, Page-close navigation cancellation and rejection of
unsupported options before mutation. It also verifies Context isolation,
descendant navigation cancellation and native cleanup on disconnect/Worker exit.
An earlier lifecycle rerun reported 17 passes and one Worker failure before the
180-second supervisor killed the incomplete lane; it is not a 44-case pass.
Earlier 30-second attachment failures and subsequent 5-second topology-phase
timeouts remain NODE-013. Later passing runs do not establish their cause or
resolution. The diagnostic record preserves this incomplete run and its exact
test-owned Page cleanup without restarting shared Chrome.

Synchronous `Page.url()` and main `Frame.url()` read a Rust-owned published view,
including external navigation, history/hash changes and the last URL after
close. The separate `browser.pwrightMetadataMetrics()` counts retained metadata
even when live-session counters are zero. Each host admits 128 metadata records,
16 KiB per URL and 2 MiB total URL bytes, including native/published/queued
overlap, with at most 256 unpublished changes. `pendingPublications` reports
that queue independently from FFI task counters. Fixed watermarks publish only
on Node callbacks, so native receipt while JS is blocked does not change a
synchronous URL getter within the same turn. This behavior now matches the
paired Playwright fixture; ordering with future public events remains open.
`frame.pwrightMetadataMetrics().unpublishedUrl` identifies a native URL revision not
yet published for that exact Frame; these metrics are Pwright extensions.
Title uses the Rust
`FrameTitle` fixed read in the utility world, sharing bounded DOM-read routes
and result ownership with content. The NODE-010 discrepancy is resolved and
remains a mandatory regression test. Title does not depend on public evaluation;
the host now installs that capability explicitly for `evaluate`. Reads have no action timeout; native CDP command budgets
still apply and remain an explicitly documented compatibility boundary.

`Page.evaluate` and main `Frame.evaluate` support string expressions or functions
with one argument, Promise results, special scalars, cyclic/shared objects and
arrays, Date/URL/RegExp/Error values and eleven standard typed-array kinds. Browser
execution uses the existing Rust capability in the main world; the facade does
not execute user functions in Node or delegate them to Playwright. Argument and
result trees have explicit admission, including 32 KiB source, depth 64, 4,096
nodes/entries and separate 256 KiB aggregate string/binary limits. Each evaluation
reserves 8 MiB of FFI payload allowance through JS conversion, within the same
64 MiB environment cap. A two-phase Node-thread conversion snapshots ordinary
object structure immediately and copies typed-array bytes at the next injected
`setImmediate`, before any Rust browser execution. Captured fixed views have a
1 MiB aggregate backing-capacity ceiling per evaluation, including the full
buffer `maxByteLength` behind small subarrays. Shared backing aliases are charged
conservatively per view. No JS reference or borrowed pointer crosses a Rust await.

`browser.pwrightConversionMetrics()` reports environment-wide `pending`,
`binaryReferences` and `backingBytes` for this prepared phase. They are distinct
from active browser work or queued Rust results; FFI `running` already includes
admitted prepared calls. Getter failures release captured
views; Worker cleanup clears unstarted conversions before draining Rust tasks.
Blocked-JS and forced-Worker tests cover prepared input and running/queued output.

This is not full evaluation parity. The NODE-017 comparisons now match immediate,
microtask and later-immediate mutation, later-getter mutation, detachment,
resizable buffer shrink/growth and fixed subarray behavior. Unpaired
UTF-16 strings, SharedArrayBuffer-backed arguments and invalid Date arguments
explicitly reject; complete serialization/error-stack edge cases remain NODE-018
and NODE-003 work. Handles and callbacks are not accepted as evaluation arguments.

### Locator queries and reads

`Page.locator` and main `Frame.locator` return lazy Rust-owned queries.
Supported composition includes `first`, `last`, `nth`, `and`, `or`, nested
`locator`, and string/has/hasNot/visible filters. `locator.page()` preserves
Page identity. Ordinary CSS pierces open shadow roots; every operation resolves
the current DOM again. This is not the full Playwright selector language:
engine prefixes, XPath, extended selector syntax and RegExp filters are not
supported yet.

Page, main Frame and Locator also expose `getByText`, `getByLabel`,
`getByRole`, `getByPlaceholder`, `getByAltText`, `getByTitle` and
`getByTestId`, with string or RegExp matchers. Exact text options and pinned
Playwright 1.58.2 role-name/state/level/hidden options are forwarded to existing
Rust selector values. Test-id uses data-testid; mutable selector registration
is not exposed. Unpaired UTF-16 explicitly rejects. Recipe limits below include
regex flags and the implicit test-id attribute, before native copying.
The expanded 32-case Locator lane passes on `edd8ba96...b0c9b7`, including
label associations, relative/Shadow DOM lookup, role-name/title priority and
global role-name regex. This is not complete selector or accessible-name parity.

Reads include `count`, `allTextContents`, `allInnerTexts`, `textContent`,
`innerText`, `innerHTML`, `inputValue`, `getAttribute`, `isVisible`, `isHidden`,
`isEnabled`, `isDisabled`, `isEditable` and `isChecked`. Scalar reads are strict;
count/collections are not. Missing attributes return null. For example:

```js
const enabledSave = page.locator('button').filter({ hasText: 'Save', visible: true });
console.log(await enabledSave.isEnabled());
console.log(await page.locator('li').allTextContents());
```

Query values and command-owned copies have their own environment-wide limit:
1,024 entries / 8 MiB, with depth 32, 256 nodes and 64 KiB aggregate strings per
query. `browser.pwrightLocatorMetrics()` exposes `{recipes, bytes}`. Retained
pure values may keep this counter nonzero after browser close without keeping
Rust browser I/O alive. Each read also reserves 6 MiB in the existing FFI
settlement budget. These declarations are not V8 heap or RSS measurements.
Page close/Worker termination cancel reads; count/byte pressure and finalization
are tested independently. The Node profile opts into eight target permits and
eight DOM reads per Frame, within the unchanged eight connection-active /
sixteen pending-call and shared byte limits. Default Rust consumers stay
serial. Independent title/Locator/evaluation work can progress while a Locator
waits (NODE-020); arbitrary navigation/input ordering remains unverified.

### Locator actions

`click`, `dblclick`, `fill`, `clear`, `press`, `pressSequentially`, `type`, `check`, `uncheck`,
`setChecked`, `selectOption`, `hover`, `focus`, `blur` and
`scrollIntoViewIfNeeded` invoke existing Rust services. Options include timeout,
force/trial where applicable, pointer position/modifiers, click button/count
and delay, and Press delay/navigation waiting. Sequential typing supports delay
and timeout; its accepted `noWaitAfter` boolean has no effect. Double-click, hover, checked
state and selection do not wait for navigation, following the pinned reference.
SelectOption accepts one string/descriptor, a homogeneous array of either, or
null for deselection; ElementHandle values are not supported.

Each action retains a 6 MiB FFI allowance through settlement. Fill inputs are
bounded at 1 MiB UTF-8, as are sequential text inputs (owned capacity is checked
separately), key expressions at 128 bytes, and selection at 1,024
descriptors / 64 KiB per string / 1 MiB aggregate. Unpaired UTF-16 input
explicitly rejects. Backend script/wire limits are independent additional
bounds. Fill/clear now share the Rust Page input coordinator with pointer and
key actions, including Delete key-up repair and cancellation cleanup.

After the dedicated Chrome/Pwright preflight, set `PWRIGHT_INPUT_FIXTURE` to the
verified serve-lib URL for `tests/fixtures/sequential-typing.html` and run
`pnpm --dir packages/pwright test:typing` with `PWRIGHT_TEST_CDP`. This supervised
lane includes native cancellation/Worker checks; it is distinct from the older
actions lane and the low-level Rust input oracle.

The action gate is not green: the latest semantic Locator addon passes **25/26**,
with the timed-out Shift-state discrepancy (NODE-022) still failing. Pinned
Playwright can retain Shift after this timeout; Rust releases it under its
existing cleanup contract. Original post-Enter reads and added consecutive
document-replacement comparisons now pass (NODE-021's entry precondition).
Do not treat ordinary action matches as full action/error/keyboard parity.

The Node profile explicitly selects bounded Rust execution-world readiness for
DOM reads, evaluation and ordinary Locator operations. It waits under existing
command admission/deadlines/cancellation; no JS sleeps, input replay, extra
driver or browser launch is added. Eight waiters and 64 Frame ownership pairs
declare an additional 20,736-byte backend-state ceiling and at most nine leases,
not a measured RSS bound. Default Rust profiles remain non-waiting. This does
not make later document replacement during backend dispatch atomic.

On the previous world-readiness addon `1c8ee9ea...d9df65`, Locator passes 22/22 and evaluation 38/38.
Lifecycle completes **42/44**, retaining a native-window initialization
discrepancy and a post-disconnect observer navigation interruption. Shared
results are 36 matches / 19 unsupported / 2 oracle errors. The ordinary failing
gates remain visible in the [artifact record](../../docs/node-ffi-compatibility-design.md#world-readiness-artifact-evidence-2026-09-07).

Context options, `Browser.newPage()`,
`Page.isClosed()`, before-unload close, child frames, advanced screenshot options, browser launch,
and general events (including Page/Context close events) are not exposed
yet. Declared unsupported async methods reject with
`errors.UnsupportedOperationError`; unsupported sync methods throw it. Unknown
options are rejected instead of ignored. This is not a complete replacement
for Playwright or Playwright Test.

See [the design and discrepancy register](../../docs/node-ffi-compatibility-design.md)
for bounded-resource settings, verification scope, and pending gates. In
particular, the initial host still uses an aggregate Rust composition; smaller
feature-specific native packages and RSS comparison are future work.

The current async exports also have an environment-wide FFI budget: 16 connect,
32 command, 16 close and 16 session-signal settlements; 64 MiB of reserved
native payload/future bytes. Slots remain charged after Rust finishes until JS
conversion, drop, or environment shutdown reclaims the outcome, including
errors. Close has independent admission. Oversized UTF-8 inputs are rejected
before native copying or browser work. Exhaustion rejects with
`PWRIGHT_RESOURCE_EXHAUSTED`; it never silently drops a result.

`browser.pwrightFfiMetrics()` reports `pending`, `queued`, `queuedCommands`, `running`,
`reservedBytes`, `connects`, `commands`, `closes` and `watches`. `queuedCommands`
excludes metadata/disconnect notifications; `queued` counts all lanes. These counters
cover the shared native environment, not just that Browser. They reach zero
after all sessions close and their callbacks settle; other active sessions can
keep them nonzero. The blocked-JS and forced-Worker cases test undelivered
result reclamation. This does not measure user-owned V8 objects, all historic
metadata, allocator overhead or process RSS. New events and binary APIs still
need explicit budgets before being exposed.
