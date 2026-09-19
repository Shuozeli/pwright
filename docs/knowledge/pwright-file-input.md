<!-- agent-updated: 2026-09-19T20:42:42Z -->

NODE-191: The selected internal path now continues `PayloadImported -> Mutated`
and then terminally removes the preparation. Rust validates the exact
package-produced array/object wrapper shape, moves it with the retained
already-retargeted INPUT into one cancellation-safe operation, and executes one
large utility-world `Runtime.callFunctionOn` with `userGesture`. There is no
selector lookup, second retarget/adoption or `DOM.setFileInputFiles` command.

The embedded helper decodes base64 in the browser, creates `File` values,
assigns one `DataTransfer.files`, then synchronously dispatches bubbling,
composed `input` followed by bubbling, non-composed `change`. A target changed
late to a non-file element returns the pinned helper string, which the caller
ignores as a successful no-op. The current pinned channel projection strips a
caller-provided `lastModifiedMs`, so the end-to-end path observes the browser's
File-creation timestamp; the mutation helper still handles the optional
serialized field when present. This discrepancy is recorded rather than
misreported as full upstream parity.

The real-Chrome lane proves one file's exact name, MIME and bytes, event order
and flags, terminal same-ticket cleanup and the late non-file no-op under both
artifacts. Public `ElementHandle.setInputFiles`, public Playwright error/stack
rewriting and the broader empty/multiple/race/maximum-payload differential
matrix remain later gates. [Canonical NODE-191
contract](pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190: The selected internal path now continues `Serialized ->
PayloadImported`. Rust admits one worst-case graph owner before the addon walks
the inaccessible serialized root. The walker preserves ordinary
JSON-compatible object/array/scalar data, turns unsupported array entries and
non-finite numbers into null, omits unsupported object values, and rejects raw
BigInt, native cycles and values outside its explicit limits. Playwright cycle
references are already serialized `{ ref }` records and therefore remain
ordinary data.

On success the preparation retains the same validated INPUT owner but no direct
V8 roots; the imported graph can be moved exactly once by the next native phase.
This checkpoint does not execute Playwright's utility-world `selectInputFiles`
body, set files, dispatch `input`/`change`, settle public errors or expose
`ElementHandle.setInputFiles`. [Canonical NODE-190 contract](pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: The selected internal path now continues `Instrumented -> Serialized`
through Playwright's late synchronous call-argument traversal. A pinned Node
language adapter preserves getter omission/propagation, `toJSON`, cycles and
special tags while Rust owns the exact session, phase and root. Its output is
root eleven; existing base64 strings are shared rather than copied into Rust.
This remains a value-payload checkpoint: generic live-handle adoption,
non-retargeting mutation/events, terminal cleanup/error transport and public
`ElementHandle.setInputFiles` are not implemented. [Canonical NODE-189
contract](pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: The selected internal path now continues `Prevalidated ->
WaitingInputAction -> Instrumented` without releasing the validated Rust owner
or resuming its parked FFI admission. Playwright's `onBeforeInputAction` point is
after INPUT/multiple/directory prevalidation and before path access, payload
serialization or DOM mutation. The Node port sees the original receiver root,
retained `call@N` and undefined point/box; the retargeted target remains private
Rust authority.

The production port is an awaited no-op until tracing/debugger/recorder/
screencast become separately selected plugins. Rejection, timeout or close
aborts the exact ticket. Late utility serialization, event-producing mutation,
terminal cleanup/publication, public errors and `ElementHandle.setInputFiles`
remain open. [Canonical NODE-188 contract](pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: The selected path now continues `Projected -> Prevalidated` by moving
the exact `RetainedMarkedFileInput` into the existing retained DOM-preflight
primitive. Its policy uses the already-projected native `multiple` and
`directoryUpload` truth values, retargets labels in the exact utility world and
retains the resulting INPUT ID in the original object group. It does not read
or serialize payload bytes, query a selector, release/re-adopt the target or
add a Node root.

The resulting Rust-only `OwnedValidatedMarkedFileInput` keeps marker-call,
adopted-group and preflight ownership together through terminal cleanup. This
is still an internal stop point: `onBeforeInputAction`, late utility-world byte
serialization, browser mutation, public error mapping and
`ElementHandle.setInputFiles` remain absent.
[Canonical NODE-187 contract](pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Target marking no longer releases the adopted element on successful
completion. The exact move-only marked owner stays in the selected preparation's
parked FFI result while `prepareProjectedTarget` projects the materialized
carrier. The actual projection reads `filePayloads`, `localPaths`,
`localDirectory`, then multiplicity, preserves four raw values as new roots,
stores native `multiple`/`directoryUpload` flags and commits `Projected` only
after an exact phase recheck.

The next prevalidation phase must consume this retained owner directly. Current
code intentionally stops before DOM validation, input instrumentation, late
utility-world byte serialization and mutation/cleanup, so public
`ElementHandle.setInputFiles` remains absent.
[Canonical NODE-186 contract](pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: After server schema validation, the selected raw driver now creates a
session-scoped `call@N`, awaits the instrumentation checkpoint, revalidates the
exact retained source and starts one Node timer immediately before
materialization. That one progress races materialization and target marking;
marking reads the Rust-retained ID and no longer accepts a JS-supplied value.
Timeout delivery revokes the exact preparation, while Node owns event-loop
timing and the timeout Error and Rust starts no duplicate clock.

The internal `prepareMarkedTarget` helper reaches Marked under this envelope;
the older raw helper can intentionally stop at Materialized. Carrier projection,
retained DOM prevalidation, real input instrumentation, late utility-world byte
serialization, mutation/cleanup and public error wrapping are still missing.
Therefore public `ElementHandle.setInputFiles` remains unavailable. Exact tests,
resource proof and full-lane qualifications are in the
[canonical NODE-185 checkpoint](pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: An explicit addon `retained-target-marking` profile advances the
internal raw preparation from `Materialized` to `Marked`. It upgrades the bound
original wrapper only after exact-session registration, performs host marking in
Rust, parks the same FFI ticket before lifecycle publication and rechecks the
entry after possible callback reentry. A running abort reaches the weak revoker;
a parked abort drops the typed result and registration. No Node roots enter the
worker and a foreign session cannot acquire or cancel the owner phase.

The public driver still stops at Materialized. Connection-wide call identity,
one deadline begun before materialization, projection, retained DOM prevalidation,
instrumentation, late serialization and mutation must be composed before this
raw method can become public `setInputFiles`. The 24-case real-Chrome lane and
remaining races are recorded in the
[canonical NODE-184 checkpoint](pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: New default-off host `retained-target-marking` selects the existing
retained-upload prerequisites and marker composition. Its Rust-only mark phase
uses the retained original node without a selector, forwards the caller's
call ID, and settles the temporary target group. No fresh timeout or runtime
command-ID substitution is added; this phase does not itself perform upload.

The host now caches persistent marker state for 64 current/creating exact
Page/Frame/client/utility-world keys, with eight retiring predecessors and eight
lookup/call leases. Each persistent entry owns a separately admitted utility-only
snapshot/receipt, never the source Handle or operation token. Shared creation,
late-result compensation, topology-driven pruning, generation-protected reuse
and fail-closed cleanup preserve ownership through cancellation and close.
Temporary adopted targets retain their own paired snapshot and marker lease
through group acknowledgement. Currentness is checked before publication/marking.

This host feature is not selected by addon profiles and does not connect Node's
Materialized ticket to native work. Same-ticket Weak upgrade/promotion, public
call identity and original deadline/instrumentation policy, carrier projection,
retained DOM prevalidation, late utility serialization/mutation and public
ElementHandle.setInputFiles remain separate integration work. Exact verification
and remaining gates are in the [canonical NODE-183 checkpoint](pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache);
no new live-test pass is asserted here. Prior checkpoints remain historical.

NODE-182: New default-off `retained-target-marking` features implement native
original-node marking with persistent private utility-world state. Calls own
only their adopted temporary target group and extra result/exception allowance;
ordinary completion/cancellation must not clear the persistent state. Creation,
mark and release use pre-admitted cancellation-safe continuations, exact native
retirement evidence and fail-closed cleanup. Real Chrome checks repeated/new
call IDs, no-op text/empty IDs, label/shadow events and all-class final cleanup.

This primitive is independent of retained DOM preflight and large commands.
The host/addon do not select it, Node still stops at raw Materialized, and no
public retained upload is claimed. Exact-world cache ownership, typed progress,
connection-wide identity, one pre-materialization deadline with faithful Node
timer delivery, carrier projection, prevalidation, instrumentation and late
serialization/mutation remain integration work.
[Canonical contract and verification](pwright-node-handles.md#node-182-persistent-native-target-marking).

NODE-181: Original native-wrapper weak capture is implemented. After existing
preparation admission, Rust downgrades the actual supplied NativeHandle state
and preserves that identity through every InputRecord/snapshot replacement.
The arbitrary JS receiver cannot rebind it. Capture/clone never upgrade, read
live/disposed state, add getters or move disposal errors before language work.
There is no new strong native pin, root, capacity, feature or budget. The actual
16 KiB registry-layout check includes the fields. The same transient overlap sum
is compile-checked against both the existing 1 MiB entry and independent 16 KiB
phase scratch, which remains admitted after a callback revokes the entry.

Native use remains absent: the ticket is still unit-result Materialized, without
Weak upgrade, marking or a native phase. Final frozen-source live runs pass
20/20 standalone, 20/20 combined and 20/20 standalone repeat: twenty distinct
cases, sixty passes, with owned cleanup fully retired and no failures,
cancellations or skips. Their scope is preparation through Materialized and
abort, not later native Weak upgrade, GC, marking or upload. Exact completed
offline/Rust/dependency gates and test-only watch-sampling corrections belong to
the canonical record. Next design must establish the operation deadline before
materialization, connection-wide protocol identity and separately admitted
persistent exact-world remote-marker ownership before the typed handoff.
[Canonical contract and verification](pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: Safe-host retained uploads now have exact-session, source-bound weak
FFI registration/check under the HostSession state lock. The eight-slot registry
adds selected-only 4 KiB BackendStateBytes and one fixed scope/lease. Generation
checks prevent stale/foreign receipt reuse; metadata provenance deliberately
does not add dispatcher, disposal or current-world validation. Registration is
intended only at the future first native handoff, never initial language setup.

Close/peer teardown seal and detach hooks, then revoke outside locks before both
operation and Handle drains. Default session shutdown uses cancel-first plus a
state barrier, and Handle owner closure also moves tracker wakeups outside its
lock. These host hooks are implemented; their actual Node/native assembly is
not. Preparation still stops at Materialized with unit-result admission and
the unchanged 32-entry language limit. Original NativeHandle weak binding,
typed progress/promotion, persistent marking, one-deadline native sequencing,
public upload and browser verification remain open. A returned ledger lease
does not prove stale Weak Arc backing was freed.
[Canonical contract and verification](pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Raw carrier projection is now a Rust-owned diagnostic kernel: read
filePayloads/localPaths/localDirectory first, then preserve the exact
short-circuit multiplicity result and derive native Boolean flags. No payload
traversal, byte copy, serialization, assimilation or persistent root is added.
It is not a production marking receipt or a new preparation phase.

The existing selected FFI budget can park successful typed results in their
original entry and resume the same permit. No running token spans the parked
foreign await; weak revocation retains Running storage until its future drops,
and idle owner destruction happens outside locks. This generic mechanism is
not yet connected to Node preparation or the native precheck. Preparation still
stops at Materialized with `FfiPreparation<()>`; public retained upload is absent.

Next integration must register exact-session weak revokers before owner drain,
including raw peer termination without Node callbacks. Native marking requires
separately admitted persistent utility-world state, not temporary group cleanup.
Preserve mark-original-target before carrier reads, then native retarget/checks,
instrumentation and late utility serialization against the exact validated
receiver. Establish the single deadline before materialization. The eager typed
upload helper is not an equivalent continuation; offline evidence is not native
marking, actual-ticket or browser verification.
[Canonical contract and verification](pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: The default-off retained-preflight leaves validate/retarget an already
adopted settlement-owned input group without reading payloads or mutating files.
The exact result ID, two extra native wrapper units and admitted backing survive
query cancellation and ready/release Drop until group settlement. Ordinary
selector cleanup cannot substitute for this settlement contract. Input type,
DOM connectivity, visibility and enabled checks are intentionally absent from
this precheck, matching its pinned upstream phase. No host/addon selection or
public retained upload is added. Observable marking/carrier projection and the
late custom utility serializer remain separate future work; do not reuse the
eager typed upload helper as their equivalent.
[Canonical contract and verification](pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Retained preparation now checks exact owner-ticket and creation-Page
registration before server schema and after an empty instrumentation await.
Do not substitute world/Handle/Frame runtime membership: navigation or detach
can retire native objects without removing the upstream dispatcher. The thin
driver schedules setImmediate in the client-validation continuation and resolves
undefined at internal Materialized; public ElementHandle.setInputFiles remains
absent. It adds no roots, dependencies, classes, budgets or native pins.

The next integration must retain the order mark target -> carrier projection ->
DOM checks/retarget -> onBeforeInputAction -> late utility serializer -> browser
atob/File. Preparing a typed PreparedFileInput eagerly would move observable
conversion too early. Actual submission, full instrumentation/error transport,
deadlines and browser verification remain open.
[Canonical contract and verification](pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: The optional retained-upload preparation now models a second
server-direction schema and raw in-memory payload serialization. Own-property
output, observable serializers/MIME and outer async assimilation are preserved;
path/stream materialization remains explicitly unsupported. Manual ticket
phases grant no dispatcher/native authority or public upload success.
[Canonical contract and current evidence](pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Add synchronous Rust channel validation immediately
after ArgumentsReady, replacing root six and advancing to ChannelReady. Visit
payloads (name, optional mimeType, buffer), localDirectory, directoryStream,
localPaths, streams and timeout in order, preserving optional-field behavior.
Keep observable map/raw results, raw boxed valueOf results, inherited setters
from ordinary assignment and raw Buffer identity. This phase neither copies
file bytes nor performs deferred server byte/path preparation or serialization.

Trusted frozen ports contain language primitives only. Rust drives normal
non-isUnderTest validation. Three no-capture/ZST native callback specializations
use static typed dispatch and current Env/path/ports per invocation; scoped
N-API values, registry leases and native owners do not escape in closures.
This avoids new Rust callback-Box allocation and its construction-failure leak;
N-API/V8 callback backing and escaped JS graphs remain outside administrative
RSS bounds. Existing roots,
FFI/synchronous scratch, metadata, feature, dependency and class budgets remain
unchanged. The preparation is not promoted to native work.

No WritableStream producer/brand exists. Observe the first `_object` read,
then fail closed with the expected-channel error; valid-stream support remains
explicitly missing rather than accepted through duck typing. Public API error
prefix/stack, deferred server bytes/path work, float deadlines, typed move-only
FfiPreparation promotion/native authority and public ElementHandle.setInputFiles
remain open. Offline verification is recorded in the canonical checkpoint;
actual-ticket/browser/public gates remain open. The binding lane has eight
authored, unexecuted cases, including two channel cases. Shared CDP is blocked
without restart permission and no new browser pass is claimed. [Canonical scope and exact evidence](pwright-node-handles.md)
own verification; earlier entries below retain their historical scope.

# File input: Rust preparation policy and Node FFI implementation plan

NODE-174 (2026-09-17): Retained-upload preparation now captures the Handle's
immutable creation-Page timeout binding, not its current DOM owner Frame and
not a settings value supplied at argument assembly. Selected sessions issue
canonical Context/Page cells from published membership with exact Weak<Owners>
provenance; Native Handle publication checks frozen creation-Page proof. Rust
propagates the binding through generic JSHandle/ElementHandle evaluateHandle,
getProperty and every getProperties child. Raw `assembleArguments(ports)` uses
the captured policy and still follows conversion assimilation without an extra
await. This supersedes the caller-selected association limitation in NODE-173.

The facade sends the creation Page ID from every Frame; it never acquires or
caches timeout settings in JS. Rust captures the canonical policy Result after
FFI admission and defers lookup errors past normal source/argument materialization
before host execution. This preserves conversion order on closed Pages and
avoids metadata queries during pre-commit lifecycle preparation. Each selected
Frame producer admits its own binding to shared canonical cells; Native Handle
publication, not the JS Page wrapper, retains it. JavaScript freezes raw
PWDEBUG/npm_config_pwdebug/npm_package_config_pwdebug at module import; Rust
selects the first defined value and treats empty, `0`, `false` and `console`
as non-inspector mode. Selected initialization precedes Browser publication.
Cells and fresh scoped bindings share 1024 admitted slots; bounded session
indexes increase total metadata to 784 KiB without JS roots, HostSession,
native-use pin or task retention. No feature/profile/dependency/class is added.

Public setDefaultTimeout is deliberately absent: typed numeric metadata does
not model arbitrary JavaScript default state. Channel validation/error phases,
deferred Buffer serialization/path work, native float deadline mapping,
promotion/submission integration and public ElementHandle.setInputFiles remain
open. [Canonical NODE-174 scope and exact verification](pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding)
own actual-ticket/browser/discrepancy gates; no new browser pass is claimed.
Earlier entries remain historical checkpoints.

NODE-173 (2026-09-17): Post-conversion argument assembly is now Rust-owned:
spread the actual awaited converted value, spread options, then resolve
timeout. The only new JavaScript language ports perform spread and a member
read. Assembly follows conversion assimilation in the same continuation, with
no added await. Raw `assembleArguments(typedSettings, ports)` moves Ready to
ArgumentsReady and replaces the sixth root (converted) with params; six roots,
32 preparations and 512 KiB synchronous scratch remain the existing ceilings.

Rust uses live Context/Page timeout metadata. Falsy options normalize to
`{}`; a numeric first timeout read causes a second raw return, without
coercing a value that changes type. Otherwise inherited injected debug mode
precedes the live default, then the parent repeats the options getter, finally
falling back to 30000 ms. Defaults are read after getters, preserving updates
during those callbacks. Only typed `Option<f64>` defaults and the normal
two-level Page/Context shape are modeled; public setters and arbitrary
JavaScript default state or custom hierarchies are not implemented.

The selected environment safely publishes the internal module-local,
no-public-constructor settings class. Its independent metadata budget is
1024 cells at 512 bytes plus 16 KiB administration (528 KiB total, one scope,
1025 leases), retained through the final cell lease without JS roots, native
authority, session or task. Creation-Frame association is currently chosen by
the caller, not automatically bound or validated. Production facade binding,
default/debug configuration, channel validation/errors, deferred Buffer
serialization and path work, native timeout-domain mapping, promotion,
submission and public retained upload remain open. No new feature/profile,
package, dependency or server is added.
[Canonical NODE-173 contract and exact evidence](pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
own verification; offline language kernels do not establish browser parity.
Earlier entries remain historical checkpoints.

NODE-172 (2026-09-17): The selected boundary now implements the synchronous
Rust owner/context/array-size preflight kernel. One-expression JavaScript ports
preserve member-call receivers, observable array methods and actual JavaScript
truthiness/addition/comparison. Rust selects call order, fresh callbacks,
classification, the 50 MiB size threshold and preflight error messages. It does
not yet read names, MIME types, options or raw bytes.

Owner lookup is awaited; `page().context()` then runs synchronously outside
the separate async converter. That converter returns an ordinary `{ payloads }`
carrier, preserving inherited `then` assimilation. Paths receive a typed
classification, then an explicit unsupported error at the parent boundary;
no stat or file reading is performed. Options/channel validation, deferred
Buffer materialization, native promotion/submission and public
`ElementHandle.setInputFiles` remain unwired.

The phase-checked guard has weak scope/registry identity. Pure `RootSnapshot`
inspection clones weak source metadata and an `Rc<NodeReference>`; reference
lookup happens after the registry borrow ends, and the temporary Rc drops
before user JavaScript. No N-API call occurs inside a registry borrow. Initial
`creation_phase` and later phases pre-admit independent scratch before N-API
allocation and recheck liveness after callbacks; replacement rejects revocation.
The selected registry shares at most 32 synchronous construction/replacement
phases with a 16 KiB administrative allowance each (512 KiB aggregate), holding
no FFI Command/task or native authority. Scratch survives callback-triggered
abort/finalization until synchronous return, not arbitrary Promise waits.

Records remain revocable and grow through private 3/4/5/6-value arrays, at most
192 registered values. Existing 32 entries, 16 scopes, the 16 KiB table, 1 MiB
per preparation and shared 64 MiB FFI pool are unchanged; the separate scratch
allowance increases selected resource totals. Arbitrary V8 graphs, scoped
handles and opaque allocator backing are not byte-bounded. Abort/close removes
registered roots even while the guard and scratch survive. Raw closed/cancelled phase ordering still
needs public alignment. Stateless selected diagnostics test the same kernel
without persistent roots or session/native authority, not actual-ticket N-API
or browser behavior. No new feature, dependency or artifact profile is added.
[Canonical NODE-172 contract and exact evidence](pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases)
own verification. Earlier entries remain historical checkpoints.

NODE-171 (2026-09-17): The selected addon now owns a bounded preparation
registry, not the public retained upload operation. A module-local
`NativeFileInputPreparation` carries weak exact-session authority; admitted
entries root receiver/files/options in one private three-element array without
getters, Buffer copying, filesystem access or native I/O. Handle provenance is
weak metadata that survives disposal without granting native-use authority.
Submission must still use the full source and world revalidation from NODE-169.

The registry has 32 entries and 16 live session scopes, a checked 16 KiB fixed
administrative reservation and 1 MiB per entry from the existing FFI Command
allowance. These are not bounds on arbitrary V8 graphs, retained closed
wrappers or weak tombstone allocations. Scope close, NativeSession Drop and
environment cleanup revoke entries; Node roots drop outside RefCell borrows
before FFI drain. Terminal publication revokes on delivery. A raw session
without a watcher may retain staging until later publication, explicit close,
wrapper Drop or environment shutdown, not merely any later entrypoint.

The new `retained-file-input` and `retained-file-input-combined` profiles select
the addon feature (Handles + file-input + host retained selection); thirteen
older artifact profiles are unchanged. Public `ElementHandle.setInputFiles`,
phase-accurate language conversion, preparation promotion and native submission
remain unwired, and browser verification is still absent.
[Canonical NODE-171 contract and exact evidence](pwright-node-handles.md#node-171-session-bound-node-preparation-registry)
own verification. Earlier entries below remain historical checkpoints.

NODE-169 (2026-09-17): The default-off safe-host `retained-file-input` feature
combines Handles, file input and the retained-reference leaves. It accepts
already-materialized input for the exact retained ElementHandle, without a
selector. The borrowed source descriptor is not a native-use pin; source/world
identity is revalidated, and adoption targets the original creation Frame's
utility world, not the current DOM owner's Frame.

Adoption and submitted upload have separate admitted, untimed cleanup owners.
Abandoned adoption settles acquisition and group cleanup only; it cannot start
an unsubmitted mutation. No addon feature or public Node retained-upload API is
exposed yet. The pinned offline ingress oracle characterizes getter/error order
and deferred materialization, not browser behavior or candidate parity.
[Canonical NODE-169 contract, exact verification and remaining gates](pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle)
own the evidence; no live-pass claim is made here. Earlier checkpoints below
retain their historical scope.

NODE-168 (2026-09-17): The optional `retained-reference` resolver path now
produces a real group-owned `ResolvedElement`/`OwnedElement` from the exact
retained receiver without a selector. This is a lower-level upload prerequisite,
not public Node `ElementHandle.setInputFiles`. Its temporary group is admitted
before native acquisition and retained through settled cleanup by the existing
continuation owner. The source itself never becomes the temporary release owner.

Target utility numeric and unique IDs come from the same Frame snapshot.
A unique-world argument-validation probe must succeed before the resolved
group accepts that world's destruction proof. Once group acquisition starts,
failed/unverified adoption must release it explicitly through settlement. The
legacy Handle-adoption abandonment path now preserves Cleanup failures. The Node preparation ticket,
getter ordering, deferred Buffer materialization and split path phases remain
integration work, with public retained upload still absent.
[Canonical NODE-168 contract and verification](pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress)
owns exact results and remaining gates; this routing checkpoint claims no live
pass. NODE-167 and earlier entries below remain historical checkpoints.

NODE-167: Current DOM ownerFrame is being integrated as the retained-upload
prerequisite. The full preparation-ticket, getter-order, independent adopted
group and single-mutation continuation design is recorded in the
[canonical Handle checkpoint](pwright-node-handles.md). Public retained upload
and the eighteen-case paired gate remain open; the old Locator ingress has a
different observable conversion order and must not be reused unchanged.
Handle-enabled shutdown now retires owned inspector sessions before Handle
drain (NODE-165), superseding the earlier order in the dated NODE-154 note below.

Historical NODE-154 regression: all eight native safe-host upload tests pass with
handles+file-input after adding pending Handle result ownership. At that checkpoint,
Handle cleanup drained before Locator inspector-session retirement; the source/executor/upload
budgets and public Node artifacts are unchanged. Evidence is in
`.work/node154-handle-owners/handle-upload-live.log`. This is not public
ElementHandle upload support or a new Node differential score.

## NODE-151: retained-node backend seam and public Handle red baseline

The backend now exposes a bounded borrowed `PayloadTarget` and
`apply_on_target`; the existing resolved-Locator method delegates to the same
implementation. This separates target origin from mutation without adding a
resource pool, copying native identifiers, adopting objects or owning native
disposal. The caller must retain/admit the exact object/world/group; the view
does not certify ownership and is not the cancellation-safe owned continuation.

Real Chrome verifies both a retained attached input and a removed input whose
owner document remains live: only the original node receives the bytes, a
same-selector replacement is untouched, the receiver survives mutation and
explicit group release destroys it. The first run's fixture redeclaration
failure is retained; wrapping setup in an IIFE produces the passing 16-scenario
run. No production DOM behavior or assertion was relaxed.

The new `test:handle-file-input` public lane passes 18/18 on pinned Playwright
1.58.2 but fails 18/18 in paired mode because Node Handle producers are absent.
It is not added to the prior 53/57 shared score. The source preflight also proves
that retirement errors precede payload getters, and distinguishes a removed
node from a node with no browsing-context owner. A Locator facade or generic
isConnected rejection would implement the wrong semantics.

[Node Handle integration contract, exact errors and outstanding steps](pwright-node-handles.md)
records the required selection, publication, current ownerFrame, adoption and
cleanup work. No Node artifact was rebuilt or Handle API advertised in this
checkpoint. The entire compatibility goal remains active.

Final NODE-151 checks under `.work/node151-handles/`:

- Backend unit tests **16/16**, owned upload continuation tests **17/17**.
- Real Chrome `backend-live-second.log`: **16 scenarios** pass in one test,
  including two exact retained-node cases. Earlier fixture failure is retained.
- Oracle **18/18** in both expanded runs; the final run pins exact retirement
  error text. Paired red baseline **0/18**, all missing candidate producers.
- Package contracts **100/100**; backend/continuation all-feature and selected
  addon strict all-target Clippy pass. Exact dependency-closure proof passes;
  ordinary host/addon exclude the six upload leaves. Format/diff checks pass.
- Default, standalone file-input and combined native artifact hashes still
  match NODE-150. No rebuild, commit or push is performed.

At 2026-09-10T23:58:11Z no NODE-151 tests/builds remain running and its preflight
target is closed (absence independently checked). No static service was started;
no task-owned background process remains. Shared backend PID 53189 RSS is about
1.03 GiB, renderer 53192 about 1.15 GiB, swap zero. Backend RSS declined from
about 3.43 GiB at this resumed turn's start; this is shared-process variability,
not a measured per-chat allocation or an attributed benefit of cleanup.

## NODE-150: Page and Frame selector-taking upload APIs

The selected file-input addon now also exposes `Page.setInputFiles(selector,
files, options)` and `Frame.setInputFiles(selector, files, options)`. These
reuse the same Rust upload source, resolver, operation, cleanup and publication
pipeline as Locator.setInputFiles. They add no filesystem, selector evaluation,
CDP logic or source copying in the JS facade. No new crate or resource pool is
introduced; ordinary builds still exclude file-input.

Unlike Locator, selector-taking methods select the first match by default.
Explicit `strict: true` preserves strict resolution. The native boundary
constructs an existing budget-admitted Rust query and, for non-strict calls,
its `nth(0)` projection. Neither intermediate becomes a JS NativeLocator:
query owners survive the operation and drop in Rust after completion rather
than waiting for V8 GC. The page method supplies the existing main Frame ID;
the Frame method supplies that exact Frame's ID. The wrapper validates public
options and attributes HostError timeout/native failures to the correct API.

The public test lane now includes default-first and strict duplicate behavior,
label/path delegation and timeout attribution on both surfaces, actual child
Frame isolation, already-closed Page rejection, and repeated Page/Frame calls
whose live query recipe/byte metrics return to baseline without explicit GC.
Earlier Buffer, ownership, large-payload and Worker gates remain in this lane.

Build the selected or combined profile again for these new native entry points;
an earlier file-input artifact correctly reports an explicit unsupported error
for missing frameSetInputFiles. The default and pre-upload artifacts remain
unchanged. ElementHandle upload, native directory mutation, exact slow-input
preparation timing, additional OOPIF/borrowed-Worker/native-interruption gates
and physical-memory parity remain open. This entry supersedes older historical
statements below that Page/Frame upload APIs are absent.

The same turn further characterizes the unrelated raw-click discrepancy; see
[fresh-document click observations](pwright-raw-mouse.md#node-150-fresh-document-click-delivery-and-explicit-scheduling-controls).
No raw Mouse production behavior or full-corpus expected value was changed.

Selected verification: `selector-upload-{first,second,third}.log` each pass **43/43**,
including the existing 32 upload cases and 11 selector-surface/resource cases.
`contracts.log` passes **100/100** public package contracts and `types.log`
passes strict package TypeScript. `addon-clippy.log`, `combined-clippy.log` and
`default-clippy.log` pass all-target strict Clippy. `combined-upload.log` also
passes **43/43** on the combined artifact. `shared-unit.log` passes **30/30**
and `shared-types.log` passes shared TypeScript. Standalone/combined/default
feature matrices and exact closure proofs pass; the new methods add no dependency.
The complete shared corpus (`shared/report.json`) still reports **53/57 matches**:
one click-result mismatch, one unsupported download and two upstream history
errors; candidate, setup and cleanup error counts are all zero. This run's click
values reverse NODE-149: upstream returns `No touch`, candidate returns `touched`.
Both histories still time out upstream while waiting for load after goBack;
candidate goBack remains unsupported. The shared upload case matches. The
supervisor exits 1 as expected for this non-green report. No shared assertion,
production Mouse scheduling or timeout was changed to improve these counts.

Rebuilt artifact SHA-256 values for this checkpoint:

- file-input: `e3df3f39eb3e875ee0be02a1afa892d87be864f2397801d596ac8f91f66e2882`
- file-input-concurrent-close-drag-frame-extra-headers:
  `b7424be406adf01c3f82e4f219df236845a2e442febd1b82617cf89cbd674b73`

Evidence is under `.work/node150-click/`; no commit/push is performed.

Cleanup was independently rechecked at 2026-09-10T23:42:48Z: this turn's
serve-lib listeners on 19340/19349 and preflight target
`EEE977AF5A2CAE853098F7395A747000` are absent. No NODE-150 background process is
intentionally retained; shared Chrome and unrelated XLQY services are untouched.
At that checkpoint the shared Codex backend (PID 53189) has RSS about 3.65 GiB;
the following targeted vmmap reports a 3.3 GiB physical footprint. Renderer
53192 has RSS about 1.21 GiB, swap is zero, and memory_pressure reports 68%
system-wide free. These are distinct shared-process/system measurements, not
per-chat memory or proof of Pwright's memory parity.

## NODE-149: selected Node Buffer ingress and public Locator uploads

`pwright-node-addon/file-input` now forwards the selected safe-host feature.
The JS package exposes `Locator.setInputFiles` for paths, homogeneous path or
payload arrays, single `{ name, mimeType, buffer }` payloads and empty clearing
arrays. Options are bounded integer `timeout` and the deprecated/no-op boolean
`noWaitAfter`. Page/Frame/ElementHandle upload methods are still absent.

Build the isolated artifact with
`PWRIGHT_NATIVE_PROFILE=file-input pnpm --dir packages/pwright build`; select
that same profile when loading the package. The combined
`file-input-concurrent-close-drag-frame-extra-headers` profile is also built.
Neither selection overwrites the default or previous selected artifacts, and
the loader verifies the compiled `fileInputEnabled` marker. Ordinary builds
exclude all six upload leaves and `pwright-cdp/large-commands`.

### Boundary and ownership

Each call admits **1 MiB of FFI metadata/conversion/future storage**, not raw
file contents, before reading payload fields. Rust bounds array counts and
metadata, captures all potentially reentrant getters, and invokes a
callback-scoped Buffer language-type port. The tiny JS facade passes the
captured `node:buffer` `Buffer.isBuffer` function without reading payload
properties or copying bytes. This port never survives the callback.

Do not use `napi_is_buffer` as proof of the public Buffer type: the tested Node
26.7.0 implementation accepts ordinary ArrayBuffer views too. This was a real
red differential (`Uint8Array` accepted by candidate but rejected by upstream),
not an inferred issue. The
[Node implementation](https://github.com/nodejs/node/blob/v26.7.0/src/node_buffer.cc#L222-L228)
also confirms the broader check. Rust still independently checks actual
uint8 view/backing metadata, rejects shared or detached backing and reads
actual native lengths. Only **after all JS calls finish** does it construct
borrowed byte slices and ask the host to admit/copy them into the move-only
source. No raw pointer, JS reference, filesystem operation, base64 encoder or
CDP implementation is put in the JS wrapper or allowed to escape that phase.

SharedArrayBuffer-backed Buffers and unpaired UTF-16 metadata are explicit
unsupported boundaries. Wrong name/MIME language types and non-Buffer
payloads report ordinary `Error`, matching the pinned channel validator;
detached backing reports `TypeError`. Getter exceptions preserve their identity.
The host's exact 50 MiB source-policy rejection also maps to `Error`.

Rust snapshots the caller cwd for relative paths at the N-API boundary and
passes it explicitly to the safe host. The selected source/worker/upload/CDP
reservations remain independent of the ordinary 64 MiB FFI budget. Public
completion uses the existing publication fence and result owner. No extra
Tokio runtime, child driver, server, browser download, GC-dependent source
release or default evaluation BinaryView expansion is introduced.

The current public deadline starts at N-API entry and passes the remaining
budget into the host; timeout errors retain the original requested duration.
**Remaining timing qualification:** upstream client-side file conversion is
before channel timeout computation. Slow getter/path-preparation parity has
not been established; do not claim complete timing compatibility from the
missing-selector timeout fixture.

Selected binary packaging additionally carries the MIME data MIT license and
the adapted Playwright helper's Apache-2.0 license/NOTICE. Default notices are
unchanged. Exact closure checks prove selected addon = ordinary addon union
selected safe host; no new N-API/runtime dependency is installed.

### Verification and retained failures

The new `test:file-input` lane uses pinned Playwright **1.58.2**, independently
owned Contexts and identical fixture setup through public `evaluate`. It reads
actual browser File bytes/metadata and events; it is not a mocked API check.
The initial run failed because the fixture used the unimplemented candidate
Page.setContent; the corrected fixture does not claim setContent support.
An intermediate 24-case run passed. Expansion then exposed the Uint8Array
acceptance bug and two Error/TypeError mismatches; their red logs are retained
alongside final verification rather than replaced.

Tests cover binary and offset/empty Buffers, filename MIME fallback,
absolute/relative Rust-read paths, multiple/clear/label/hidden/text-input
semantics, validation order, strictness, timeout, 8 MiB uploads, exact 50 MiB
rejection, getter reentry/detach/throw, shared backing rejection, two-source
admission for empty batches, source reuse without GC, and Worker termination
with either a resolving operation or a queued unpublished result. Worker
tests identify and assert removal of exact owned native Page/Context IDs;
they do not restart or close the shared browser. All ordinary test cleanup
asserts zero host leases, cleanup work and FFI reservations after disconnect.

The shared corpus adapter now invokes the public Locator method and applies
the same owned-file alias resolver on both sides. Unselected profiles still
classify uploads as explicitly unsupported. Full shared verification has now
been rerun below; it remains non-green and does not erase NODE-140 history.
Native directory mutation, Page/Frame/handle methods, borrowed Worker teardown,
forced mid-submitted-upload retirement and complete timing/memory parity remain
open. Component cleanup tests do not substitute for those public browser gates.

Final selected artifact verification (Node 26.7.0, Chrome 152.0.7977.83):

- `node149-file-input-final-{1,2,3}.log`: **32/32** each, three consecutive
  passes. `node149-file-input-final-worker-bytes.log`: **32/32** after additionally
  asserting all queued Worker's 1 MiB bytes through an independent browser
  observer before termination. These counts overlap, not distinct feature cases.
- `node149-contracts-final.log`: **98/98** ordinary package contracts;
  `node149-shared-unit.log`: **30/30** shared-harness unit tests. Package and
  shared TypeScript checks pass. Source contains no JS driver/transport/process
  imports. Default, selected and combined strict Clippy and exact dependency
  closure checks pass (`node149-default-clippy.log`,
  `node149-addon-final-clippy.log`, `node149-combined-clippy.log`).
- `node149-combined-file-input.log`: **32/32** on the combined artifact, including
  the strengthened queued-Worker native byte assertion.
- Standalone artifact SHA-256:
  `3caa7eebd32e3ceb5eda1c650a39cdb6760c88dc975e6bd0b8f673221f468781`.
  Combined artifact SHA-256:
  `c64782a522dc141fcef21a75ba3d925878da1c75fff43396419726c857c87203`.
  Existing default and previous concurrent-close combined artifact hashes
  remain unchanged. Foundation HEAD remains `f76e3f2`; no commit or push.

All logs above are under `.work/`. Error fixtures compare error families and
relevant message text, not byte-identical upstream call logs or stacks.

### Complete shared API corpus: still non-green

`.work/node149-shared/report.json` records all **19 suites / 57 cases** without
filters: **53 match, 1 value-mismatch, 1 unsupported, 2 oracle-error; zero
candidate-error, setup-error or cleanup-error**. Supervisor exits 1 correctly;
no native panic, output overflow or supervisor timeout occurred. Source, native
artifact and HTTP fixture hashes are retained in `metadata.json`.

The unchanged shared `file-upload` case now matches using the same newly owned
file on both sides. The harness deletes only that owned temporary fixture after
the run. Previously failing Page-content setup did not recur in this run; no
root-cause fix for that older intermittent result is claimed.

Remaining full-corpus rows:

- Download: candidate `waitForDownload` remains explicitly unsupported.
- Touchscreen-labelled case: oracle `touched`, candidate `No touch`. This corpus
  uses `mouseClick`; do not claim an implemented touchscreen API or change its
  expectation to conceal the discrepancy.
- Both history cases: upstream `page.goBack` exceeds its 5000 ms load wait;
  candidate `goBack` remains unsupported. An upstream timeout is not parity.

The task-specific serve-lib daemon used the verified Tailscale fixture route,
was deregistered/stopped after testing, and its separate machine-local config
and event DB remain outside the repository for diagnostics. The task-owned
preflight blank tab was closed. No user/shared Chrome process was restarted or
terminated. No task-owned server or Worker is intentionally left running.

The 23:22:57Z read-only handoff snapshot shows the shared Codex backend at
2,033,824 KiB RSS (about 1.94 GiB), renderer about 1.15 GiB and system swap zero.
Earlier backend RSS transiently reached 4.5 GiB and then fell without cleanup;
targeted vmmap showed about 2 GiB of MALLOC_LARGE (empty) resident storage. This
is process-level evidence, not per-chat attribution or exact upload RAM. The
remaining observed large rustc process belongs to the separate XLQY checkout,
not this completed Pwright build. No unrelated process was stopped.


## NODE-148: selected safe-host upload integration

The optional `pwright-node-host/file-input` feature now composes the existing
source/filesystem/executor/backend/continuation leaves and large outbound CDP
selection inside the real host. This is not yet an addon artifact or public JS
method: the ordinary Node host/addon dependency closure is unchanged.

The safe Rust host exposes synchronous `HostSession::prepare_file_payloads`
for borrowed `MemoryFile` slices, producing a move-only `PreparedFileInput`.
Missing/empty MIME uses the pinned Rust provider, memory mtime remains unset,
and actual slice lengths are admitted before copying. Metadata is bounded before
MIME lookup. A prepared batch is tied to its originating connection; it cannot
be submitted to another host. Caller-owned bytes and any Node backing store are
outside this source allocation and must have their own boundary admission.

`HostLocator::set_input_files` consumes a prepared batch.
`HostLocator::set_input_files_paths` takes an explicit base and borrowed paths;
the existing admitted Tokio adapter copies the bounded request and performs
filesystem work off the event loop. The base is not read from a process global
inside the host. connectOverCDP explicitly uses Separate locality even on a
loopback endpoint. Native directories return a clear unsupported error; they
are never silently turned into an empty buffered upload or traversed in JS.

### One controlled Frame command and shared resolver

Host-local typed method 65002 uses the existing Frame-targeted runtime, not a
new transport/server or portable wire method. Preparation precedes strict
resolution. The same prepared source survives retries; after resolution the
upload is submitted once. One deadline covers host admission, queue, path work,
selection, mutation and release. This host deadline does not yet include the
future N-API conversion phase, which must preserve the public call's original
deadline when it is integrated.

The operation borrows the existing Locator installation's resolver, waiter,
unique object-group ID source and authoritative native Frame-world snapshots.
Its utility-world route adapter retains the exact snapshot through group release
and verifies client/world identity. It never starts an independent ID counter
or falls back to a replacement Frame. Only selected host builds add one extra
resolution/world-snapshot allowance for a cancelled upload that outlives its
runtime lane; ordinary Locator budgets remain unchanged.

### Admission and shutdown

The selected host permits two source batches (including empty/prepared batches),
eight admitted host calls, one filesystem job and one submitted upload. The raw
source plan contributes a 50 MiB aggregate ceiling, shared between preparing and
in-flight batches. Each batch still follows the strict less-than-50-MiB semantic
limit. Inputs allow 64 entries, 4096-byte paths/names, 1024-byte MIME values and
64 KiB aggregate metadata. These are explicit implementation limits, not a claim
that upstream has these count/metadata caps. Encoding/wire/framing budgets remain
separate from raw bytes and from browser allocations.

Independent atomic call/source admission supplements the shared production
ledger: unrelated components cannot lend their count capacity to upload calls.
`prepare_retaining` on the existing upload composition retains the caller's
source-slot owner through native completion AND acknowledged group cleanup,
including after the public waiter is dropped. Its inline size participates in
the existing checked owner-byte bound; the original `prepare` API delegates
with no extra retained object. An unpolled upload drops the source slot without
submitting a mutation. This is not proof of browser cleanup if the executor is
discarded; the existing fail-closed continuation contract still applies.

Submitted upload continuations use a separately admitted one-slot Tokio cleanup
executor with no arbitrary job deadline. Host shutdown first stops admission and
waits for public operations, then uses the existing exact native session
retirement/transport handling, drains file jobs and upload continuations, and
finally drains shared Locator cleanup. Thus the ordinary five-second cleanup
deadline does not prematurely destroy an upload's source owner. The host does
not kill Chrome, create another runtime or close unrelated browser Contexts.

Failure handling also matters: if native session retirement itself fails, an
untimed upload cannot be left waiting before the transport is torn down. That
selected shutdown branch calls `CdpConnection::request_disconnect` for this
host's connection before draining uploads. This cancels local I/O and fails
pending/new sends without a Browser.close command. Already queued bytes keep
their admission until the writer actually retires; `io_finished` is distinct
from the stopping signal. The original retirement failure remains an error,
and connection loss is never treated as proof of browser group destruction.
This is teardown failure handling, not a generic public upload-timeout policy.

### NODE-148 verification

- `.work/node148-disconnect-default-tests.log`: 534 default-selection library
  tests pass (97 CDP, 17 upload-continuation, 420 host).
  `.work/node148-final-settlement-tests.log`: 545 selected library tests pass
  (106 CDP, 17 continuation, 422 host). These suites overlap and are not additive.
  Both strict Clippy runs pass in `.work/node148-disconnect-default-clippy.log`
  and `.work/node148-final-settlement-clippy.log`.
- `.work/node148-final-settlement-live.log`: all eight real Chrome safe-host
  tests pass after the final transport/shutdown changes. They cover Rust MIME
  and exact buffered bytes/events, actual local-path preparation, missing-target
  timeout, strictness/no mutation, Context closure during selection and host
  drain, source-slot admission/reuse even for empty inputs, an 8 MiB upload
  beyond the ordinary command ceiling, and a concurrent Locator evaluation
  whose native object remains valid after upload group cleanup. Fixture Contexts
  are disposed and the tests check final host metrics; the ordinary successful
  cases also check every production resource class returns to zero.
- The two new continuation tests retain a caller source-admission owner through
  separately gated native response and group release, and return it without
  mutation when unpolled. New transport tests cover terminal disconnect with
  unpolled/new calls and queued large backing. They do not invent successful
  browser destruction after transport loss. Deliberately forcing real Chrome
  detach failure during a submitted upload remains a broader fault-injection
  gap; the host fallback branch has code-path and transport-component evidence.
- `.work/node148-final-boundaries.log` and
  `.work/node148-node-boundaries.log` verify exact default/selected closures and
  standalone/combined host feature selections. The selected host has 240 normal
  packages (241 with concurrent-close, drag, frame navigation and headers).
  Existing addon profiles do not select upload. `.work/node148-node-contracts.log`
  records 89 passing JS package contract tests. No native artifact was rebuilt;
  all three prior artifact hashes and the foundation commit remain unchanged.
- `.work/node148-combined-final-check.log`: the final file-input + concurrent
  close + drag + Frame navigation + extra-headers host selection compiles.
  Format/diff checks pass. The task's preflight blank tab was closed after
  testing; no task-owned test process or server remains. Final shared backend
  RSS was about 2.16 GiB and system swap was zero; this is not per-chat RAM.

These are safe-host/leaf tests, not public Node setInputFiles differential
acceptance. Started-upload cancellation ownership has deterministic component
tests; the Context-close host case cancels while still selecting the target,
not while Chrome is already applying the file mutation. Keep that distinction
when adding public Worker/Page/Frame and forced native-error scenarios.

Still open: N-API Buffer/backing-store validation and ingress admission, thin
public JS/TS wrappers, isolated addon artifact selection, original public deadline
and error publication, worker-thread verification, native directories and the
full upstream differential gate. The generic evaluation BinaryView's 1 MiB
backing ceiling is not a reusable large-file ingress policy. Do not enlarge that
ordinary limit or the 64 MiB general FFI settlement budget to expose upload.

## NODE-147: independently admitted large outbound commands

The optional `pwright-cdp/large-commands` feature supplies
`LargeCommandProfile` and
`CdpConnectionProductionProfile::with_large_commands`. Compile-time selection
alone does not enable it: the production connection must select the profile,
and the caller must use the explicit large settlement API. The file-input
backend forwards this feature and requires `PayloadProfile::with_large_commands`.
Unselected paths fail explicitly; there is no automatic large-command fallback.

The ordinary Node command ceiling is 4 MiB. A source just below 50 MiB becomes
69,905,068 base64 bytes before JSON, so changing only the source policy cannot
make that upload work. The selected pool adds a separate maximum command size,
capacity and fragment size; it does not widen ordinary commands, responses,
events or inbound WebSocket messages. Large commands also consume existing
ordinary pending/write slots. Profile selection composes in either order with
reclaiming event fanout and exact-session retirement, preserving all resource
requirements. Invalid, duplicate, incompatible and overflowing selections fail.

### Ownership and peak accounting

- Admit a large slot and its WireBytes before raw JSON serialization. Per-slot
  admission is `3 * max_bytes + 1024`, conservatively covering geometric Vec
  growth and overlapping old/new storage. Caller-owned source and input Value
  allocations are not included and must already have their own admission.
- Retain that admission through prepared bytes, queued/writing messages,
  protocol settlement and an unconsumed response. Dropping an unpolled future
  queues no command; dropping a submitted waiter does not release queued/native
  work. Free wire backing before making its independent slot available again.
- Fragment text messages with shared Bytes slices, Text then Continuation
  opcodes, flushing each fragment. Do not interleave two data messages.
  UTF-8 code points may cross frame boundaries; the receiver reconstructs the
  text message before decoding. This avoids retaining a full large-frame copy
  in tungstenite's writer buffer.
- Admit connection-lifetime framing/control storage as
  `512 + 4 * (fragment_bytes + 14)`. The pool exposes lightweight active count,
  written-fragment count and largest-fragment metrics, not a transactional
  memory snapshot or exact allocator/physical-footprint proof.
- Backend base64/JSON input scratch, raw source bytes, FFI references/copies,
  transport raw bytes and browser File/JS storage are distinct. A capacity of
  one limits concurrency; it does not mean a 50 MiB source costs only 50 MiB RAM.
  Fragmentation is not streaming source materialization: the complete admitted
  source, encoded input and serialized command still exist.

### Browser verification and remaining public integration

The memory-heavy ignored `live_large_payload` test selects one 72 MiB outgoing
slot with 64 KiB fragments while retaining ordinary 4 MiB commands and 64 KiB
responses. It uploads 52,428,799 real bytes, checks every browser File byte,
metadata and input/change events, releases the object group, and checks source,
backend and transport ownership after disposing its own Context. A second
Unicode payload checks text reconstruction across fragment boundaries.

Final local evidence:

- `.work/node147-default-tests.log`: 246 default-selection release tests pass.
  An initial default-only compile failure exposed an unqualified CdpError in
  the disabled-feature branch; it was fixed and the entire default set rerun.
- `.work/node147-selected-tests.log`: 130 selected release tests pass, including
  eight new transport admission/ownership cases. This overlaps the default
  suite; the counts are not additive. Default and selected strict Clippy pass
  in `.work/node147-default-clippy.log` and
  `.work/node147-selected-clippy-final.log`.
- `.work/node147-large-live-final.log`: the final nearly-50-MiB upload and
  Unicode reconstruction pass on Chrome 152.0.7977.83. The encoded transfer
  uses 1,067 fragments, with maximum payload 65,536 bytes. The test takes
  4.23 seconds; the surrounding cargo command also includes build/lock time.
  Its `/usr/bin/time` process-tree statistics do not isolate test, browser or
  chat memory and are not a per-upload physical-memory measurement.
- `.work/node147-ordinary-live-final.log`: all 14 prior browser payload
  scenarios plus three continuation/abandonment scenarios pass. Test-owned
  Contexts are disposed; unrelated browser pages are not touched.
- `.work/node147-boundaries-final.log`: exact default/selected closures pass.
  `.work/node147-node-contracts.log` and `.work/node147-node-boundaries.log`:
  89 Node contract tests and the existing package boundary matrix pass. The
  three existing native artifact hashes remain unchanged. Format and diff
  whitespace checks pass. CI has explicit default and selected Rust checks;
  live browser tests remain opt-in and are not claimed as CI runs.

This is a transport/backend prerequisite, not public Node setInputFiles.
The next integration must reuse the connection's existing resolver/unique group
ID source and exact native-world routes; it must not create a second resolver
whose IDs restart at one. Buffer ingress requires validated fixed backing storage
and bounded copies before async escape. Keep ordinary FFI limits unchanged and
budget source ownership separately. Started upload cleanup must survive public
timeout/cancellation and must drain during Page/Frame/Worker shutdown; the
existing five-second generic cleanup timeout is not upload settlement evidence.
Native directory completion and the public differential gate remain open.

## NODE-146: single-use upload continuations

The optional `pwright-composition-chromium-file-input` leaf supplies
`FilePayloadOperationService::prepare`, consuming one `OwnedElement` and one
`PreparedPayloads`. It sends no mutation until the returned UploadFuture is
polled. It borrows the existing backend, retains the original route/group and
source, and never repeats selector resolution or a submitted mutation.

- Drop before first poll: discard the source and submit only the resolved
  element's already-admitted group cleanup. No upload command runs.
- Drop after first poll: transfer the entire in-flight operation to its own
  pre-reserved cleanup executor slot. Source, backend command, route and group
  owners survive the public waiter; completing cleanup does not reissue upload.
- Drop while group release is pending: continue that same release Future;
  retain the raw source and operation admission until it settles.
- Await normally: publish the result only after group release or its existing
  authoritative destruction proof. Preserve an action error and a cleanup error
  together. A dropped public result does not hide cleanup failure from the runner.
  Its executor diagnostic is a fixed, bounded failure summary, rather than a
  duplicate of large browser-controlled primary/cleanup messages.
  Awaited callers still receive the structured original errors.

UploadProfile declares a positive operation count and owner-byte allowance.
Construction admits shared health/count storage. Preparation atomically admits
one operation and its control bytes, then reserves continuation capacity before
boxing. Concrete Future/wrapper sizes plus control allowance must fit. Source,
backend encoding, route/resolution and executor-private backing remain separate
plans. The source and operation stay admitted through cleanup, while backend
command scratch can retire after its response. Completed Future objects do not
retain their admission. Dropping the service does not discard started work.

Each active operation needs one executor reservation in addition to the
resolved element's existing reservation. The caller must budget and drive its
executor through shutdown; this crate starts no task or runtime. Cleanup failure
or discarding an armed continuation poisons new service admission. Zero Rust
leases after an error is not proof of zero browser objects. An executor that
abandons jobs has violated the drain contract; fail-closed health is not a
replacement for actually executing cleanup.

### Explicit CDP settlement, not an internal timeout

Inspection found that ordinary `send_command_with_session` removes response
registration at its configured timeout. Keeping that Future alive would not
preserve a pending upload. The new explicit
`send_command_to_settlement_with_session`/`CdpSession::send_to_settlement` path
instead retains the response registration until protocol settlement. The
payload-only group method is named
`runtime_call_function_by_value_in_group_to_settlement`; ordinary CDP APIs and
their deadlines are unchanged. Public cancellation/deadline publication belongs
outside this native Future, using the continuation owner above.

Protocol response, transport termination and explicitly selected exact-session
retirement can settle this wait. A generic cancellation, error string or timeout
is not native destruction proof. Group cleanup separately requires acknowledgement
or the existing authoritative proof. An unreachable/hung browser can retain a
bounded operation until actual settlement/retirement; this change does not invent
a forced stop or silently release admission. The embedding Node host must still
wire exact Page/Frame retirement, cancellation and worker shutdown/drain.

Evidence:

- `.work/node146-final-tests.log`: **246 release tests pass**, including 15 new
  ownership tests and 5 new CDP settlement tests. Paused-clock tests distinguish
  external timeout from native completion and preserve ordinary command deadlines.
  Tests also cover queue/storage/ledger rejection, lost-executor health and
  selected matching-session retirement. The first in-memory peer fixture omitted
  production's post-reader pending settlement; that test harness was corrected,
  not production's close behavior. The failed initial log is retained.
- `.work/node146-final-clippy.log`: strict release Clippy passes.
  `.work/node146-owner-bounded-tests.log` and
  `.work/node146-owner-bounded-clippy.log` recheck the final 15 owner tests and
  strict Clippy after bounding abandoned-result diagnostics, including a large
  fake cleanup message that must not be copied into the executor's result.
  `.work/node146-boundaries.log`: exact closure passes. The new leaf selects the
  union of buffered backend and Locator-resolution composition dependencies,
  with no filesystem, filesystem executor or Node production dependency.
  Ordinary host/addon exclude all six upload leaves. Tokio test-util is dev-only.
- `.work/node146-final-live.log`: **17 real Chrome fixture scenarios pass**:
  the 14 existing payload cases and 3 owned-operation cases (complete,
  drop-started, drop-unpolled). Real File bytes/metadata/events prove one mutation
  for a started upload and none for an unpolled upload; group IDs fail after
  release and operation/source/group leases drain. Both live tests remain
  explicitly ignored by default; they are not automatic CI browser acceptance.
  `.work/node146-owner-bounded-live.log` reruns all three owner scenarios after
  the final diagnostic change. Every test-owned Context and the preflight blank
  tab were disposed; no task-owned server or worker remains.
- `.work/node146-node-contracts.log` and `.work/node146-node-boundaries.log`:
  89 Node package contract tests and ordinary/selected host/addon boundaries pass.

This is not the public Node upload feature or a complete Locator action. Still
open: memory-payload preparation and public codec/capability, preparation and
resolution cancellation, Node/Page/Frame/Worker lifecycle integration, exact
timeout/error publication, native paths/directories and their input-event wait,
large-message/FFI peak admission, and public differential cases. Existing native
artifacts and NODE-140's non-green broader gates remain unchanged.

## NODE-145: Chromium payload mutation

`pwright-backend-cdp-file-input` now implements an actual browser mutation for
prepared buffered files. `CdpFilePayloadService::apply` borrows a group-owned
`ResolvedElement` and `PreparedPayloads`, using the exact original client,
utility world and native object ID. It never polls/retries a selector, launches a
browser, reads a path, or implements a Node wrapper. **Public setInputFiles is
still unsupported; native paths/directories are not implemented by this backend.**

The DOM helper retargets labels with the pinned control/button exceptions,
checks input/multiple/directory flags, creates browser File and DataTransfer
objects from base64, assigns input.files and dispatches input then change.
Hidden/disabled inputs work without pointer actionability. The text-input
success/no-op quirk and directory-target rejection (including empty payloads)
are preserved. Rust validates the returned property snapshot with UploadPlan
and rejects inconsistent status/property combinations as protocol errors.
Unexpected status strings are bounded before serde can echo browser text.
Prepared MIME must be resolved and nonempty; nonfinite modification time rejects
before dispatch. Empty/unknown MIME resolution belongs to source preparation,
not an invented browser fallback.

Retarget/validation/mutation run in **one synchronous utility-world call**,
retaining the same DOM element without a second Inspector wrapper. This differs
from upstream's internal multi-call arrangement; private wire identity and
same-race timing parity are not claimed. Returned values are small by-value
status snapshots, not additional remote handles. The exact source group remains
caller-owned and is released through the existing resolution composition.
The new explicit CDP by-value-in-group primitive also assigns exception wrappers
to that group; it never silently falls back to the ungrouped method. Existing
ungrouped callers retain their old contract. Native object, world and group
names each have a 1,024-byte backend limit and explicit scratch allowance.
The adapted helper retains Microsoft's notice and the complete Apache-2.0
license in LICENSE.playwright; the backend crate declares MIT AND Apache-2.0.

PayloadProfile supplies explicit call/item/base64/metadata ceilings. It reserves
ActiveCommand and BackendStateBytes before constructing encoded payloads and
typed CDP argument values. Accounting includes two overlapping base64/value
representations and conservative metadata/control allowances. Original raw
bytes are covered by the source owner. Transport command/response retention,
allocator internals and browser File/DataTransfer storage are separate, not
counted as exact backend heap. A nearly 50 MiB source and the ordinary 64 MiB
FFI ceiling still require a dedicated connection/large-message plan. No default
transport or ordinary Locator allowance was expanded.

This is a low-level borrowed operation, not cancellation-safe public action
composition. The caller must retain a started call and its owners until native
completion, then release the source group; dropping an await is not proof of
no browser mutation. Page/Frame/worker cancellation, timeout publication,
no-double-mutation retry policy and pending-command drain must be wired in the
embedding layer before public exposure. Native directory completion additionally
needs its pre-installed cancellable input-event wait, not just a native command
acknowledgement. These gates remain open.

Evidence:

- `.work/node145-grouped-regression.log`: **226 release tests pass**, including
  12 backend + 86 filesystem/source/model/executor + 105 CDP + 23 Fake tests.
  The browser integration
  test is ignored by default and separately invoked below; do not report it as
  an automatically running CI browser gate.
- `.work/node145-grouped-clippy.log` and
  `.work/node145-grouped-boundaries.log`: strict release Clippy and exact
  dependency closure pass. Backend normal dependencies add model/source/base64
  to Locator resolution's CDP closure, not filesystem, executor or Node. Ordinary
  host/addon exclude all five file-input leaves. CI tests/lints the backend too.
  `.work/node145-grouped-final-clippy.log` rechecks the final corrected live
  fixture; formatting and git whitespace checks also pass.
- `.work/node145-grouped-live-fixed.log`: **14 actual Chrome scenarios pass**.
  They cover binary and multiple files, clearing existing files, label retarget,
  hidden/disabled input, Shadow DOM, text-input no-op, invalid targets, button
  versus label retargeting, multiplicity, directory and directory-clear rejection,
  plus an actual file read through the admitted Rust executor and pinned MIME
  provider before locator resolution and browser mutation. A deliberately
  throwing utility-world files setter preserves its JavaScript exception,
  leaves files/events untouched and releases the root and Rust leases.
- `.work/node145-package-contracts.log` and `.work/node145-node-boundaries.log`:
  89 Node contract tests and ordinary/selected dependency boundaries pass.

The initial new exception fixture failed before dispatch because its setup
returned HTMLInputElement.prototype by value (CDP -32603). Evaluating that
prototype alone through Pwright reproduced the error; ending the setup with
void 0 fixed the fixture without changing upload behavior or weakening assertions.
The failed run remains in `.work/node145-grouped-live.log`.

The live test inspects actual File bytes/names/MIME/mtime and input/change
bubbling/composed/trusted flags. It checks Inspector object IDs fail after group
release and source/backend/resolution leases return to zero. It does not inspect
the discarded exception RemoteObject ID or prove exact browser heap usage.
Each run creates
and disposes only its own Context; the preflight blank tab was also closed.
Its connection explicitly bounds commands to 2 MiB, responses to 1 MiB and
events to 64 KiB. These small fixture checks are not large-upload peak-memory
proof or the public Node/shared-corpus differential acceptance gate.

Run the ignored integration test only after the dedicated Chrome/Pwright
preflight, with the verified loopback PWRIGHT_CDP_WS_URL:
`cargo test --release --locked -p pwright-backend-cdp-file-input --test live_payloads -- --ignored --nocapture`.
No static server is used. No native addon rebuild, commit or push occurred.
NODE-140 broader failures and the public upload gap remain unchanged.

## NODE-144: admitted blocking executor

`pwright-file-input-tokio` now runs the synchronous path pipeline through an
explicitly selected, bounded adapter. It receives an existing Tokio Handle and
injected Send/Sync filesystem/MIME providers; it creates no runtime and no
unbounded application queue. The ordinary Node host/addon still exclude this
adapter and the other file-input leaves. **setInputFiles is not public yet.**

`WorkProfile` explicitly supplies task capacity, source limits, scope and
runtime/channel overhead bytes. Its production requirements admit queue state
in ActiveCommand and BackendStateBytes. Source/path/content admission is supplied
separately; the composition root can combine both requirements and share one
production ledger, as tested. Queue planning is not a substitute for source
planning. The overhead parameter is declared embedding policy, not an exact
measurement of Tokio internals, allocator padding or runtime thread stacks.

`submit_paths` checks input shape/length bounds, acquires a task slot and byte
lease, then copies only bounded base/path strings and creates the worker/channel.
It never queues before admission. Capacity includes queued tasks, running tasks
and completed but unconsumed results. A shared caller/worker task owner keeps the
slot and request/channel allowance until neither side needs them. Consuming the
Future releases its side even if the completed Future object itself is retained.
Delivered prepared content/native paths have independent source leases.

Dropping a Job, explicitly cancelling it, closing the executor, or dropping the
executor marks unpublished work cancelled. Workers check before preparation,
before each stat/open, and before each read of at most 64 KiB. This does not
interrupt an already executing system call. The current read/descriptor/content
owner remains live until the call actually returns. A cancelled queued worker
exits before touching the filesystem; a cancelled publication discards its
prepared output before releasing its task slot. A closed runtime is reported
as WorkerStopped, and a caught worker panic as WorkerPanicked.

`close_and_drain` closes admission and waits on the actual outstanding-owner
count with notification registration before the empty check. Callers must also
consume/drop their Jobs; close does not seize an unconsumed receiver. Prepared
results already delivered to callers remain independently owned and are not
destroyed by draining work. OS-stalled calls may delay drain indefinitely:
embedding operation deadlines must not claim that dropping an await killed IO
or reclaimed its resources. A Job's has_result observation reports a buffered
value only; false is not evidence that a runtime is still alive.

The cancellation-aware filesystem wrapper retains the task owner with each
opened reader. Ordinary file preparation remains the NODE-143 pipeline,
including raw-path stat ordering, native directory/local routing, exclusive
transfer limits, pinned MIME and post-read mtime. No tokio::fs nested task or
per-file detached runtime is introduced.

Verification:

- `.work/node144-file-input-all-tests.log`: **16 executor + 19 filesystem +
  22 source + 29 model tests pass**, zero ignored. Includes real file bytes,
  task/byte capacity rejection, blocked-reader cancellation with bytes/descriptor
  still admitted, explicit cancellation, close/drop behavior, bounded read
  chunks, queued cancellation under a single blocking worker, stopped runtime,
  worker panic, buffered-result capacity and one shared production plan.
- `.work/node144-file-input-tokio-clippy.log`: strict release Clippy passes.
  Test gates release blocked workers on fixture teardown, including assertion
  failure; readiness/final-release polling has a bounded diagnostic watchdog.
- `.work/node144-file-input-boundaries.log`: exact normal dependency closure
  adds only Tokio and pin-project-lite to filesystem/core/model/source (plus Unix
  libc). Ordinary host/addon exclude all four leaves. CI now tests/lints all four.

Final checks retain all 89 package contracts and nine host/ten addon boundary
selections in `.work/node144-package-contracts.log` and
`.work/node144-node-boundaries.log`. Formatting and diff checks pass. The combined
addon hash remains `1b18f3cca8fa45e351c50bed6d3dc42be25a1a67c77d2895bf4638d052d90efc`.
The original foundation commit `f76e3f2701a8ccde0474b5accf40dc328aaa511c` was
reverified locally; this checkpoint makes no new commit or push.
No task-owned background process remains. Shared backend PID 53189 RSS changed
from 3,375,456 KiB at 20:32:03Z to 1,600,240 KiB at 20:44:35Z and 1,253,632 KiB
at 20:48:46Z; swap stayed zero. This is a shared-process observation, not exact
per-chat RAM or a claim that these code changes reclaimed application memory.

Remaining implementation is not narrowed: prepare/copy memory payloads with MIME
and N-API backing-store admission; implement Chromium input mutation and directory
input-event completion; admit overlapping base64/JSON/wire storage; attach this
executor to Node operation cancellation and Page/Frame/worker shutdown; expose
the Playwright-shaped APIs and run differential browser tests. This checkpoint
does not claim those integration or upload gates, and NODE-140 failures remain.
No native addon rebuild, browser process, server, commit or push was performed.

## NODE-143: filesystem preparation and pinned MIME

`pwright-file-input-fs` now composes the content/model leaves into actual path
preparation. `PathServices` injects an absolute UTF-8 base directory,
`FileSystem` and `MimeLookup`. `PreparedPaths` owns admitted path state and
selects native paths/directory or bounded payload materialization. This remains
an optional synchronous leaf; **public Node upload is still unsupported**.

Pinned source inspection found that client elementHandle.ts first stats the
original item, then resolves it lexically. The implementation preserves this
order: eliminating missing components or symlink/parent traversal before the
initial stat would change behavior. A second directory fails immediately, even
before a later missing path. The final mixed-directory/file check precedes
separate-browser size admission. Separate-browser files receive a second size
stat on their resolved paths, then bounded reading and a post-read path mtime
observation. Native directories remain paths even with a separate browser;
colocated native files are not buffered or subject to the transfer size limit.
Empty inputs clear with an empty payload owner. Native input-event waiting and
final access checks belong to the still-unimplemented browser backend.

The production `SystemFileSystem` uses ordinary stat and Unix O_NONBLOCK open,
then checks the opened descriptor is a regular file. This avoids hanging on a
FIFO substituted between stat and open; devices/pipes reject explicitly rather
than pretending to support them. Symlinks are followed. Non-Unix opens return
UnsupportedPlatform until a corresponding provider is implemented. File growth,
truncation and type drift remain explicit SourceChanged failures; exact upstream
error parity for concurrently modified sources is not claimed.

Before stat, the path owner reserves one batch plus one potential descriptor
slot per item in LiveObject. Its BackendStateBytes allowance covers the retained
path array, temporary join/normalization allocations, MIME strings, reader values,
borrowed source-view array and bookkeeping. Generic providers must separately
admit any private reader heaps; the production File reader has no such owned
content buffer. OS/kernel descriptor memory and allocator overhead are external.
Payload materialization acquires its separate content lease while this path
lease is still held. Failed opens close previously opened readers; failed reads
or post-read stats release both stages. Native routes retain path admission
through the future browser action, without opening or traversing directories.

`PinnedMime` vendors the **1,173 extension mappings** from Playwright 1.58.2's
bundled mime@3.0.0. The original MIT text is retained in the filesystem crate's
MIME-LICENSE. The optional static table is read-only binary data, not per-call
heap or a production Node/Playwright dependency. Lookup is allocation-free and
preserves bare extensions, dotfiles, slash/backslash stripping, case conversion
and unknown-type behavior. Unknown path MIME falls back to
application/octet-stream. The provider is injectable; no alternative MIME
database is silently substituted. Memory-payload MIME preparation remains part
of the upcoming embedding boundary.

Evidence:

- `.work/node143-file-input-tests.log`: **19 filesystem + 22 source + 29 model
  release tests pass**, zero ignored. Includes actual file bytes through the
  production provider, original missing-component behavior, native directory and
  large-local-file routing, separate-browser size rejection, post-read mtime,
  byte growth, device rejection, admission-before-stat and partial-open cleanup.
- `.work/node143-file-input-clippy.log` and
  `.work/node143-file-input-boundaries.log`: strict release Clippy and exact
  normal dependency closure pass. Filesystem selects core/model/source and Unix
  libc only. Ordinary host/addon exclude all three file-input leaves.
- `.work/node143-mime-oracle.log`: **9,393 native Rust lookup results match
  Playwright 1.58.2**, covering every table extension, uppercase/bare/dotfile/path
  variants and additional Unicode/unknown inputs. The command
  `node scripts/check-file-input-mime-oracle.mjs` verifies the static table,
  builds a release diagnostic example and compares bounded 128-argument batches.
  This is a direct value differential, not a browser upload E2E.

CI now includes all three leaves' release tests and strict Clippy. The MIME
oracle comparison requires the existing pinned tests/ Playwright installation
and was run locally; it is not falsely represented as a CI browser gate.

Final regression checks retain 89 passing package contracts and nine host/ten
addon boundary selections in `.work/node143-package-contracts.log` and
`.work/node143-node-boundaries.log`. Formatting/diff checks pass; the combined
native artifact remains SHA-256
`1b18f3cca8fa45e351c50bed6d3dc42be25a1a67c77d2895bf4638d052d90efc`.
No task-owned background process remains. The verified shared backend PID 53189
had RSS 3,194,880 KiB at 20:17:39Z, 5,060,496 KiB at 20:27:36Z and 2,866,848 KiB
at 20:31:23Z; swap remained zero. Targeted investigation during the high sample
reported physical footprint 3.2 GiB and system-wide memory free 69%. These are
distinct measures of a shared process, not per-chat attribution or leak proof.

Next work remains explicit: admit/cancel worker scheduling and source-handle
lifetimes before async escape, prepare memory payload MIME, implement the
Chromium file-input capability and native-directory event wait, admit overlapping
base64/JSON/wire bytes, then expose and differentially verify the Node API.
No executor was introduced in this checkpoint and synchronous methods must not
run on a Node/event-loop thread. No native addon was rebuilt; no browser process,
server, commit or push was started. NODE-140 failures remain unresolved.

## NODE-142: ledger-admitted content ownership

The new optional `pwright-file-input-source` leaf implements actual content
copying/reading, not just a metadata plan. Its only normal dependencies are core
and the existing model. Ordinary host/addon profiles still exclude both leaves.
**Public Node setInputFiles remains unsupported.**

`PreparedPayloads::materialize` takes explicit limits, a ledger/scope, and
borrowed `PayloadSource` entries. Memory sources use actual slice lengths.
Reader sources use an observed remaining length and an injected finite `Read`
implementation; a real `std::fs::File` is exercised against a checked-in fixture.
The caller supplies already resolved names/MIME and observed optional mtime.
No MIME fallback, path resolution, opening, stat, directory traversal or clock
query is hidden in this content layer.

Before any content read or copy, allocation-free model iteration validates
aggregate lengths and metadata, then one atomic lease reserves a LiveObject,
source ArtifactBytes, and BackendStateBytes for the entry array, strings, batch
owner and a fixed 512-byte bookkeeping allowance. Allocations use fallible exact
capacity requests. Source bytes and the lease remain together in a private,
non-cloneable owner, exposing only borrowed views. No unleased extraction API
exists; storage drops before its lease. Input backing stores, file descriptors,
ledger setup, allocator rounding and outer executor resources are not counted
as owned storage by this leaf.

Reads fill only the pre-admitted length and use one extra stack byte to detect
growth, including growth from a zero-byte observation. Truncation or growth
returns SourceChanged; IO errors propagate and partial batches are discarded.
This is an explicit source-drift policy, **not a proved Playwright error parity
claim**. Equal-length concurrent rewrites are not detected, and a file can change
after the EOF observation. The content layer is not a filesystem snapshot.
The enclosing path layer must preserve upstream remote-path size/error ordering
before using this payload-oriented primitive.

There is no worker spawn or async API. The method does synchronous IO; do not run
it on a Node/event-loop thread. The embedding layer must admit worker/queue
capacity before scheduling, own opened handles until reads exit, reject
non-finite device/pipe inputs and stop new work on cancellation. Dropping an
await alone does not stop filesystem IO. The current tests prove that discarding
a result receiver during a gated read leaves the worker's content lease live
until actual completion; they do not establish a production executor or bounded
cancellation latency. Native paths/directories never require this content copy.

Verification: `.work/node142-file-input-tests.log` records **20 source tests and
29 model tests passed**, zero ignored. Cases cover binary/metadata preservation,
real-file reading, clear/empty content, per-file base64 planning, growth,
truncation, read/probe errors, interrupted probe, atomic count/byte/metadata
admission before IO, partial-failure release, ownership transfer, and abandoned
results during active reads. Strict release Clippy passes in
`.work/node142-file-input-clippy.log`; the exact core/model/source closure and
ordinary-host exclusion proof passes in `.work/node142-file-input-boundaries.log`.
CI now includes both crates' release tests and strict Clippy.

The existing 89 package contracts and nine host/ten addon boundary selections
also pass in `.work/node142-package-contracts.log` and
`.work/node142-node-boundaries.log`. Formatting and diff checks pass. The combined
native artifact remains SHA-256
`1b18f3cca8fa45e351c50bed6d3dc42be25a1a67c77d2895bf4638d052d90efc`.
No task-owned background process was launched or left running. At 20:16:52Z the
verified shared Codex backend PID 53189 had RSS 3,929,280 KiB, versus 3,601,808
KiB at turn start and 1,966,480 KiB at an intermediate checkpoint. These large
fluctuations do not establish per-chat ownership or monotonic leaking; swap was
zero throughout. Shared services and other tasks were left untouched.

Next: complete admitted path/stat/MIME and source-handle preparation plus an
explicit embedding executor, then implement native action/transport and Node
wiring. Existing 64 MiB ordinary FFI limits must not be silently expanded.
No browser upload test, addon rebuild, commit or push is part of this checkpoint.
NODE-140 broader compatibility failures remain open.

## NODE-141: pinned behavior and allocation-free preparation

`setInputFiles` is an actual missing public surface in the NODE-140 shared corpus.
This checkpoint begins its Rust implementation with `pwright-file-input-model`.
The crate has zero dependencies and performs no allocation, filesystem access,
browser action, process launch or JavaScript conversion. It is not selected by
the existing host/addon profiles. **The Node API remains unsupported**: this
preparation model is not a complete upload implementation or release gate.

### Authoritative behavior

The oracle is Playwright 1.58.2, tag commit
`ce480a952553175eae75342aad2c5e86cdf2cbba`. Read that revision rather than the newer
upstream checkout HEAD. Relevant files are client/elementHandle.ts,
server/fileUploadUtils.ts, server/dom.ts, server/chromium/chromium.ts and crPage.ts
under packages/playwright-core/src, plus packages/injected/src/injectedScript.ts.

The original input can be a path, path array, memory FilePayload or payload array.
Mixed paths and payloads reject. Empty arrays select payload clearing. Path
resolution/stats happen before selector resolution; files mixed with a directory
or multiple directories reject. Relative path resolution belongs to an injected
filesystem/environment boundary, not the portable model.

There are **three different locality boundaries**, not one inferred local flag:

- Client-to-Playwright-server remote connections can stream files to the server.
  This wire protocol is not copied into the in-process Rust FFI architecture.
- `connectOverCDP` defaults to a browser **not collocated with the server**, even
  for loopback Chrome. Ordinary paths are read locally and transferred as memory
  payloads; directly sending those paths to DOM.setFileInputFiles would diverge.
- Directories remain native paths in the pinned implementation even with that
  separate-browser flag. This does not guarantee remote Chrome shares the path;
  the future backend must preserve the explicit filesystem/locality distinction.

Both explicit payload totals and files buffered for a separate browser reject
at **greater than or equal to 50 * 1024 * 1024 bytes**. The public message says
50Mb, but the actual boundary is exclusive 50 MiB. Collocated native paths are
not subject to that transfer buffer limit. Payload encoding pads **each file**
independently: two one-byte files need eight base64 bytes, not four.

Target validation follows labels, requires an INPUT element, and checks the
multiple/directory combinations before mutation. This is not a pointer action:
hidden/disabled file inputs work without actionability waits. A directory requires
webkitdirectory, and that target rejects ordinary files **and empty []**.
Do not invent one universal type=file error: the pinned injected-payload branch
returns a string for a text input, which its caller ignores, producing success
without mutation; the native-path branch is a separate browser operation.

Payload File objects preserve raw bytes and declared names. Empty MIME falls back
to filename lookup and finally application/octet-stream. Path-derived buffers
carry filesystem mtime; memory File construction has browser time semantics.
Time/filesystem/MIME inputs must be injected or treated as external observations,
not queried from portable core globals. The injected payload path dispatches
input (bubbling, composed) then change (bubbling, non-composed), both untrusted.
Native directory input/change events are trusted and the action awaits input to
observe completed asynchronous directory traversal.

### Implemented model boundary

`FileInputLimits` explicitly bounds item count, per-path/name/MIME byte lengths
and cumulative metadata. No implicit fallback profile is created. The planner
classifies homogeneous inputs, validates exclusive aggregate transfer limits and
selects Payloads, ReadFiles, NativePaths or NativeDirectory. `UploadPlan` exposes
item count, source bytes, individually padded base64 bytes and metadata bytes.
It also implements the post-retarget target validation order.

Metadata is **not an ownership/admission certificate**. The model borrows
caller-supplied sizes/strings and does not retain them or prove their relationship
to real Buffers/files. Its source/base64 counters do not cover vector capacity,
JSON strings, filesystem buffers, N-API backing stores, pending native calls,
browser objects or directory contents. These require separate owners before
materialization. A path read must detect growth beyond the admitted limit rather
than trusting an earlier stat or using an unbounded read-to-end allocation.

### Remaining implementation tasks (updated through NODE-150)

1. Source materialization, filesystem/MIME, admitted execution, safe-host
   memory preparation and selected N-API/Locator input boundaries are implemented.
   Extend error/getter ordering and slow-preparation timing characterization;
   keep deliberately unsupported Buffer forms explicit.
2. The borrowed Chromium payload backend, owned continuation and selected host
   Frame command and public publication are implemented. Retain source, route
   and group ownership through settlement and cleanup. Retain one
   prepared source through selector retries. Once a file mutation is submitted,
   do not blindly repeat it after detach/timeout and duplicate input/change events.
3. Keep payload transfer separate from native path/directory commands. Directory
   completion needs a pre-installed, cancellable input-event wait. Timeout,
   element/frame/Page retirement, browser disconnect and worker teardown must
   reclaim every listener, handle, file, buffer and native-command owner.
4. The opt-in large outgoing CDP pool and bounded fragments are implemented
   and selected by the safe host/addon, with independently admitted Node ingress.
   Keep source/base64/transport admission separate and the 64 MiB ordinary FFI settlement budget
   unchanged rather than increasing every default command allowance.
   Directory traversal's browser allocations are not bounded by a one-path Rust
   input count. Keep native/browser resource limits distinct from host admission.
5. The isolated file-input artifact, Buffer type/storage checks, TypeScript
   declarations, packaging notices and feature/dependency gates are implemented.
   Extend worker tests to borrowed Contexts and controlled interruption of an
   already submitted native mutation; queued-result/selection teardown does not
   prove every native phase. Measure physical memory separately from admission.
6. Locator/Page/Frame.setInputFiles are public under selection. Implement the
   handle surface with its correct ownership and timeout semantics;
   finish directory differential fixtures and rerun complete shared gates.
   Source/model or focused Locator success must not mark those tasks done.

### Evidence and limits

`node141-file-input-model-tests.log` under `.work/` records **29 release tests
passed**, with no ignored tests. Coverage includes exact/aggregate/overflow size
limits without allocating giant test buffers, per-file base64 padding, metadata
bounds, input mixing, separate/collocated routing, directory validation order,
empty clearing and target rules. Strict Clippy passes. The dedicated dependency
proof finds zero normal dependencies and excludes the model from ordinary host
and addon closures; CI now includes these checks. Existing nine host/ten addon
dependency selections and all 89 package contracts remain passing.

`packages/pwright/scripts/probe-file-input-oracle.mjs` exercises **21 actual
upstream cases** on the dedicated Chrome. All pass in
`.work/node141-file-input/oracle-first.json`, including raw binary content,
directory relative paths, events, label retargeting, hidden/disabled inputs, MIME
fallback, source/target errors, missing-path-before-selector behavior, exact
50 MiB rejection and the text-input success/no-op quirk. Its transparent observer
records **zero native file-path commands for regular paths** and exactly one for
the directory, confirming routing rather than assuming it from endpoint locality.
The relay/report have fixed bounds and no candidate core is invoked by this
oracle characterization; do not count these as candidate upload conformance.

Fixtures are task-owned local files under `.work/node141-file-input/fixtures`,
not publicly served. Run the probe with the loopback PWRIGHT_TEST_CDP plus absolute
PWRIGHT_FILE_INPUT_FIXTURES and an unused PWRIGHT_FILE_INPUT_REPORT path. It writes
reports exclusively. The package command is `pnpm run probe:file-input-oracle`.
Existing native artifacts are unchanged. No commit or push occurred, and all
NODE-140 non-green verification results remain open.

Final checks: both dev and release strict model Clippy pass, formatting and diff
whitespace checks pass, and all eight new documentation routes resolve. The
experimental addon still hashes to
`1b18f3cca8fa45e351c50bed6d3dc42be25a1a67c77d2895bf4638d052d90efc`.
No task-owned background process remains. At 20:05:21Z the verified shared backend
PID 53189 had RSS 2,653,984 KiB (about 2.53 GiB), versus 2,202,944 KiB at turn start;
swap is zero. A checkpoint in between was much lower, so this is not evidence of
monotonic growth, a leak or per-chat ownership. The 50 MiB oracle rejection input
existed only in the completed diagnostic process.
