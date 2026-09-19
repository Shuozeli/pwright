<!-- agent-updated: 2026-09-19T21:23:32Z -->

# Node retained handles: integration contract and verification

## NODE-191: Terminal Rust-owned serialized-payload mutation

NODE-191 consumes the exact two owners deliberately retained by earlier
checkpoints: NODE-187's already-retargeted, prevalidated INPUT and NODE-190's
move-once serialized graph. The standalone and combined
`retained-serialized-payload-mutation` profiles compose every prerequisite but
add only one backend/composition/host/addon leaf. Existing profile identities
and ordinary feature closures remain unchanged.

The backend accepts only the package-produced file-payload subset. The root is
one serialized array with IDs 1..65 and at most 64 exact four-field objects;
names are at most 4 KiB, MIME strings 1 KiB, aggregate metadata 64 KiB and
aggregate padded base64 70 MiB. IDs may be Rust integers or finite positive
integral doubles because N-API's JSON import represents ordinary JavaScript
numbers as `f64`; fractional, non-finite, zero, out-of-range, reordered or
unknown fields fail before native I/O. This is a protocol proof after the
JavaScript serializer, not a second public argument validator or a generic
evaluation codec.

One large `Runtime.callFunctionOn` runs in the retained creation Frame utility
world and object group, with the retained retargeted object ID and
`userGesture: true`. The request contains neither selector/retarget/adoption
logic nor `DOM.setFileInputFiles`. Its narrow parser reconstructs the serialized
array, decodes each base64 string with `atob`, constructs `File` objects, assigns
one `DataTransfer.files`, dispatches `input` then `change` with the pinned flags,
and ignores the injected helper's returned error string. A late non-file target
is consequently a successful terminal no-op, matching Playwright's ignored
return rather than inventing a protocol failure.

Ownership remains explicit through settlement. The backend admits one
ActiveCommand and 64 KiB of control/scratch storage. The composition admits one
LiveObject and 16 KiB owner plus 512 bytes fixed state and a reserved cleanup
continuation. The existing 96 MiB graph owner, validated target chain, Node
deadline and 1 MiB FFI ticket stay live until mutation and acknowledged group
release settle; direct V8 roots remain zero. Dropping an unpolled future issues
no command. Dropping a started waiter transfers the future to cleanup, and a
failed group release poisons subsequent mutation admission.

The addon never admits a second FFI command. It atomically takes both owners at
`PayloadImported`, parks the same typed result, then the Node-thread callback
rechecks exact session/ticket authority, commits `Mutated` and immediately uses
the terminal registry path. Success therefore removes the entry and releases
the FFI permit without `abort()` or GC. Foreign-session and out-of-order calls
cannot acquire or revoke the rightful owner.

Final checkpoint verification passes 5/5 focused backend tests, 4/4 focused
composition tests and 9/9 selected host file-input tests. Both rebuilt
artifacts pass the 392/392 offline retained-input/oracle matrix, 167/167 package
suite and complete 42/42 ordinary real-Chrome raw binding lane; the combined
artifact also passes the 59/59 focused profile/dispatch/Worker batch. TypeScript, exact
dependency and package-boundary checks, formatting, diff checks and strict
release Clippy for backend, composition, host, standalone addon and combined
addon are green.

The completed live matrix uses Playwright 1.58.2's in-process server
instrumentation to pause both implementations after prevalidation. Empty,
one-file and multiple-file bytes/name/MIME/call-time modified values, exact
event flags, listener exceptions, late type change, deadline cancellation and
Frame detach agree. A separate same-turn abort after native admission proves
no side effect and complete continuation cleanup. Each artifact also admits and
mutates exactly 50 MiB minus one byte: Node RSS grows by about 333--334 MiB at
the sample point, the source ArrayBuffer returns exactly to baseline after GC,
and all native owners are retired. RSS itself remains allocator-resident until
process exit, so this is not reported as reclaimed physical memory.

The matrix exposes two deliberate open compatibility decisions. Playwright
resolves when the main document navigates during the instrumentation pause,
whereas NODE-191 rejects fail-closed after reclaiming every owner. The pinned
channel also strips caller `lastModifiedMs`; ordinary payloads still match the
browser creation-time behavior. No public `ElementHandle.setInputFiles` or
public error/stack rewrite is exposed, so neither difference is hidden behind a
compatibility claim.

Run the ordinary live matrix under each profile:

```sh
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 \
PWRIGHT_NATIVE_PROFILE=retained-serialized-payload-mutation \
pnpm run test:handle-file-input-binding
```

Repeat with the `-combined` profile. Run the opt-in memory case with
`PWRIGHT_MAX_PAYLOAD_GATE=1`, `PWDEBUG=0` and `node --expose-gc --test
--test-name-pattern='maximum admitted payload'
test/handle-file-input-binding.test.mjs` under each profile. These commands
assume the externally managed, loopback-only dedicated Chrome required by the
repository policy; the tests create and close only isolated Contexts.

## NODE-190: Bounded Rust ownership for the serialized payload graph

NODE-189 deliberately left the Playwright-compatible serialized graph rooted
in V8 because JavaScript owns getter, `toJSON`, realm, cycle and special-value
semantics. NODE-190 adds the next separate boundary: after those semantics have
already run, copy the package-owned graph into a bounded Rust owner before any
transport or browser mutation. This avoids an unaccounted 50--70 MiB V8 graph
surviving into a future asynchronous CDP command and makes the extra memory
capability explicit and opt-in.

The standalone and combined `retained-serialized-payload-import` profiles add
an addon leaf over `retained-call-argument-serialization` and a host leaf over
`retained-target-prevalidation`. The host installation reserves 4 KiB of
control state. Each session admits at most one live import owner and reserves
one ActiveCommand plus the full 96 MiB BackendStateBytes allowance before the
first graph value is copied. Capacity is checked after exact-session/stage
inspection but before exclusive phase checkout, so a rejected second
`Serialized` ticket remains valid and retryable. Public dispatch still aborts
its exact ticket when any phase throws.

The addon importer visits own enumerable string keys and JSON-compatible
arrays/scalars under fixed bounds: maximum depth 64, 8,192 nodes, 8,192 entries
and 72 MiB aggregate UTF-8 string bytes. Undefined/function/symbol values are
omitted from objects and become null in arrays; non-finite numbers become null.
Raw BigInt, native object cycles and unsupported native values are rejected.
Playwright's serializer has already converted logical cycles to `{ ref }`
records, so those remain ordinary acyclic data. This walker handles an
unexposed graph created immediately beforehand by this package; it is not a
replacement for the JavaScript serializer or a claim of generic hostile-object
`JSON.stringify` equivalence. Exact transport of unpaired UTF-16 code units and
wire-key ordering remains a NODE-191 protocol-design gate; the current
N-API-UTF-8/`serde_json::Value` owner is the ordinary value-payload subset.

After the copy, Rust rechecks the ticket and atomically commits `Serialized ->
PayloadImported`. The private root container is replaced with an empty array,
reducing direct V8 roots from eleven to zero, while the exact validated target,
`call@N`, Node deadline and parked 1 MiB FFI admission remain unchanged. A
`RetainedSerializedPayload` stores `serde_json::Value` plus metrics and the
dedicated lease. Its mutex-protected graph may be taken exactly once by the
next Rust phase; abort, session close or owner drop releases it and the sole
admission without JavaScript GC.

This is a bounded copy, not zero-copy: both V8 and Rust forms may coexist during
the synchronous walk until commit drops the old roots. The 96 MiB reservation
is a declared administrative allowance, not measured process RSS or a claim
about Chrome memory. No CDP command, element retarget, INPUT mutation,
`input`/`change` event, terminal publication, Playwright error rewrite or public
`ElementHandle.setInputFiles` is implemented. NODE-191 must consume this exact
owner in a non-retargeting Rust/CDP mutation and then prove settlement.

Final verification passes the 482/482 extended offline file-input matrix and
166/166 package cases under both artifacts, plus the 36/36 focused serializer/
dispatch lane, 3/3 focused Rust owner tests, Worker identity/teardown,
TypeScript, exact dependency/boundary checks, formatting, diff checks and
strict release Clippy. Both rebuilt
artifacts pass the complete 34/34 real-Chrome binding lane. The live cases prove
foreign-session rejection, eleven-to-zero root transfer, unchanged parked FFI
admission, added native owner leases, exact abort reclamation and single-slot
capacity with a retryable rejected ticket.

## NODE-189: Late call-argument serialization on the retained owner

Pinned Playwright does not pass the already materialized `filePayloads` object
directly to Chromium. After DOM prevalidation and the awaited
`onBeforeInputAction`, `evaluateInUtility` synchronously runs
`serializeAsCallArgument`. Object-field getters that throw are omitted, array
index getters that throw propagate, empty objects may fall back to `toJSON`,
cycles use numeric references, and special JavaScript values receive explicit
tags. Instrumentation microtasks therefore settle before any of these late
reads. The earlier `Buffer.toString('base64')` remains materialization, not this
late phase.

The addon-only `retained-call-argument-serialization` leaf composes directly on
`retained-input-instrumentation`. Rust checks the exact NativeSession before
checkout, requires `Instrumented`, invokes the package's pinned synchronous
language adapter on root six, rechecks after every possible JavaScript reentry,
then stores its result as root ten and commits `Serialized`. Thus the same
validated Rust target, public `call@N`, Node deadline, 1 MiB FFI admission and
root container survive `Instrumented -> Serialized`; there is no second target
resolution, native command or progress object.

The adapter is deliberately JavaScript because getter, proxy, realm, `toJSON`
and cycle behavior belong to the Node language boundary. Browser/target
authority and phase policy remain in Rust. For ordinary payloads it creates
only structural wrappers and reuses the existing base64 string leaves, so this
phase does not copy a 50--70 MiB payload into Rust. The logical root count grows
from ten to eleven. Reachable V8 graph bytes remain outside the fixed Rust FFI
allowance, exactly as the raw carrier did; a future strict-memory profile may
add a declared compatibility limit, but the Playwright-compatible leaf does not
silently truncate the graph.

This is not yet the full generic server evaluation serializer. In particular,
nested live Playwright `JSHandle` values require a later Rust-owned adoption
table before `{ h }` slots can be emitted. Normal file payloads contain values,
not handles, and are differentially pinned here. DOM mutation, CDP transport,
`input`/`change` events, terminal cleanup/publication, error rewriting and the
public `ElementHandle.setInputFiles` entry point remain separate leaves.

The standalone/combined artifact identities and binary marker prove explicit
selection. Differential serializer tests cover ordinary payloads, cycles,
special values, throwing object/array getters, `toJSON` fallback and thenable
non-assimilation. Offline dispatch tests prove post-instrumentation ordering and
rejection cleanup; the real binding lane proves exact-session rejection, the
eleventh root, unchanged FFI admission, retained native leases and terminal
reclamation.

Final verification passes 398/398 selected offline file-input cases, 165/165
package cases, and 34/34 focused serializer/dispatch cases under each selected
artifact. Both standalone and combined artifacts pass Worker loading and their
full 32/32 real-Chrome binding lanes. Types, exact dependency/boundary scripts,
formatting, diff checks and strict release Clippy for standalone and combined
selections are also green. Two earlier live runs observed shared-Chrome timing
and sibling-watch sampling fluctuations; unchanged focused cases and both full
reruns passed without widening deadlines or closing the unrelated XLQY tab.

## NODE-188: Awaited input-action instrumentation boundary

Pinned Playwright calls `instrumentation.onBeforeInputAction(progress, this,
undefined, box)` only after `_setInputFiles` has retargeted the original node,
proved that the target is an INPUT and validated multiple/directory policy. For
the public `ElementHandle.setInputFiles` path, `box` is also undefined. Local
path checks or the late utility-world payload serializer and DOM mutation happen
after every normal listener and then every `order: 'last'` listener has settled.

The explicit addon-only `retained-input-instrumentation` leaf preserves that
ordering. `handleBeginFileInputInstrumentation` checks the exact NativeSession
before exclusive checkout, obtains root zero from the private root container and
commits `Prevalidated -> WaitingInputAction`. Root zero is the original
JavaScript receiver. It is not the Rust retargeted object, an object ID supplied
by JavaScript or new authority. After the awaited port resolves,
`handleFinishFileInputInstrumentation` repeats exact-session/stage validation
and commits `WaitingInputAction -> Instrumented`.

Node owns the internal port. Its frozen default is an awaited no-op because this
package does not yet implement Playwright tracing, debugger, recorder or
screencast listeners. The port shape is deliberately narrow: a frozen progress
view with the retained `call@N` and `race`, the original receiver, and undefined
point/box. This is a composition seam for later optional plugins, not a public
listener API and not permission for arbitrary user callbacks. The receiver is a
Node-facade identity token, not yet Playwright's server `SdkObject` with
page/context attribution. A concrete listener plugin must add that Rust-backed
adapter and normal-then-last listener ordering explicitly; it must not treat the
current token as full listener compatibility.

The outer coordinator races the hook with the one existing Node deadline.
Listener rejection, timeout or target/session retirement reaches the existing
catch and aborts the exact preparation. A close during the await removes the
entry, so the finish gate observes TargetClosed rather than reviving state. A
foreign session fails before phase checkout and cannot cancel the rightful
owner. The validated Rust owner, original marker/adoption group and source
registration remain nested in the same parked FFI result throughout.

This leaf adds no host/backend/composition feature, package, FFI command, permit,
Node root or native lease. The selected record still owns ten logical roots and
one 1 MiB admission. Its short-lived progress-view object and Promise are V8
allocations without a Rust byte bound and must not be counted as native memory.
Resolved dependency fingerprints prove that stripping only
`pwright-node-addon/retained-input-instrumentation` yields the exact
prevalidation graph.

Verification passes the selected 275-case offline lifecycle/Worker batch,
164 package cases, types and exact boundary/dependency scripts, strict release
Clippy for standalone and combined selections, artifact identity/Worker checks,
and 30/30 real-Chrome binding cases under each artifact. The live cases prove
the no-listener checkpoint leaves ten roots, the same permit and validated owner
parked until abort, and that a sibling session cannot cancel the ticket. This is
still an internal stop point: late serialization, mutation/events, terminal
publication/error mapping and public `ElementHandle.setInputFiles` remain open.

## NODE-187: Same-owner retained DOM prevalidation

The new explicit `retained-target-prevalidation` leaf composes the existing
NODE-178 DOM preflight with the exact NODE-186 marked owner. A private
`PreflightTarget` abstraction preserves the old `OwnedElement` behavior and
adds `OwnedMarkedElement` without erasing its cleanup type. The resulting
`OwnedValidatedMarkedFileInput` nests the marked owner, derived preflight
admission and retargeted INPUT ID. Marker-call admission, the original adopted
object group and preflight wrappers therefore survive together until one
terminal cleanup; the implementation never releases and re-adopts the target.

The selected internal order is now:

`server schema -> call ID -> instrumentation checkpoint -> deadline -> materialize -> mark -> project -> prevalidate`

`preparePrevalidatedTarget` retains the existing Node progress through the new
async step. `handlePrevalidateFileInputTarget` requires `Projected`, reads only
the native projection flags, resumes the original `FfiPreparation`, moves out
the marked owner and rechecks the exact registration before the Rust preflight.
Success parks the validated owner on that same ticket and commits
`Prevalidated`. The preflight retargets in the exact utility world and checks
INPUT/multiplicity/directory policy; it reads no payload bytes. Its returned
object ID remains Rust-only, so the ten existing Node roots do not grow.

### Cleanup, errors and resource proof

`PrevalidationError::MarkedCleanup` preserves a `MarkerOperationError` cleanup
surface when the nested marked owner fails to settle. An unpolled or cancelled
future, Node timeout, explicit abort, session close and ordinary drop all retain
the same nested owner until pre-admitted cleanup completes. The raw internal
host mapper currently exposes other prevalidation failures through its bounded
backend error; the final Playwright language-error mapping is intentionally not
claimed yet.

There is still one 1 MiB FFI permit, 32 preparation slots and ten Node roots.
The original eight-call marker admission remains occupied inside the validated
owner. The selected preflight adds two independent capacity-eight scopes:

- backend admission: 16 RemoteHandle units and 156,368 BackendStateBytes total
  (two handles plus 19,546 bytes per active operation);
- composition admission: eight LiveObjects, 16 KiB per owner plus 512 bytes of
  fixed service state (131,584 BackendStateBytes total), with eight separate
  cleanup reservations.

The fixed service state contributes one selected installation lease. Thus the
shared host teardown assertion remains exactly six leases for the old marking
profile and exactly seven for prevalidation. These are administrative
admissions, not bounds on V8, DOM or browser process memory. No new package,
transport, executor dependency or runtime lane is introduced.

### Verification checkpoint

The composition suite passes 52 cases with one real-Chrome primitive ignored;
the selected host passes 666 with 24 live cases ignored, and the addon passes
68/68. Offline retained lifecycle passes 271/271 and package contracts pass
163/163. Types, exact dependency/boundary checks, old-marker/default-addon
checks, formatting and whitespace checks pass. Strict release Clippy passes
standalone and combined selections, and both rebuilt artifacts pass Worker
construction/teardown.

Against the dedicated loopback Chrome, the focused prevalidation case passes
under both artifacts. Each complete raw binding lane passes 28/28, proving a
real marked owner becomes a validated owner on the same parked permit while ten
roots remain, then explicit abort restores the exact warm resource baseline.
The initial selected-host run passed 657 and failed nine shared upload tests
because their idle assertion still required the old six-lease marker baseline;
the implementation intentionally added a seventh fixed preflight service lease.
The feature-gated six/seven assertion and full rerun pass.

### Remaining integration gates

- NODE-188 now runs the awaited `onBeforeInputAction` boundary without losing
  the validated owner; optional concrete listeners remain separately gated.
- Serialize bytes late in the utility world, then mutate and settle the same
  group with complete cancellation/publication coverage.
- Map the exact Playwright language errors and differentially verify public
  `ElementHandle.setInputFiles`; the internal 28-case lane is not API parity.

## NODE-186: Retained marked owner and same-ticket carrier projection

Successful native target marking now returns a move-only
`RetainedMarkedFileInput` rather than immediately releasing its adopted object
group. It contains the concrete `OwnedMarkedElement` and its already-reserved
creation/cleanup reservations; it is neither a public Handle nor authority to
bypass later source/world checks. `NativeMarkCompletion` parks that owner beside
the original preparation result. Ordered publication still consumes the mark
outcome first, but success leaves the owner in place for the next native phase.
Failure clears the completion, while abort, scope/environment close or ordinary
drop follows the pre-admitted cleanup continuation.

The selected internal order is now:

`server schema -> call ID -> instrumentation checkpoint -> deadline -> materialize -> mark -> project`

`prepareProjectedTarget` keeps the same progress and Node timer alive through
the new synchronous step. `projectAfterMarking` invokes the actual Rust-hosted
language projection kernel on the raw materialized carrier. Its observable order
is `filePayloads`, `localPaths`, `localDirectory`, then multiplicity
(`payloads.length` only when required). It performs no byte serialization or
await. Because getters and numeric coercion can synchronously reenter and abort,
the exact phase is rechecked before replacement roots are allocated and before
commit. Success retains the original six roots plus exact raw
`filePayloads`/`localPaths`/`localDirectory`/`multiple` references, stores native
`multiple` and `directoryUpload` booleans, and advances `Marked -> Projected`.

### Resource and boundary proof

The selected logical root maximum grows from six to ten; every older profile
remains at six. Those roots may reference arbitrarily large V8 objects or
Buffers, so this is not a JS heap/RSS byte bound. `ProjectionFlags` remains in
the existing 32-entry registry and its unchanged 16 KiB administration and
synchronous-overlap checks. The larger `NativeMarkCompletion` is included in the
existing fixed native-size assertion beneath the one 1 MiB FFI command permit.
No command, lane, package, feature or artifact path is added.

The target-marker service's existing eight-call admission bounds simultaneously
retained marked owners. Its persistent exact-world marker cache is independent:
warming the first call intentionally leaves its world scope/leases resident,
while the temporary call/group owner must retire before an exact warm baseline
is sampled. Dropping a projected ticket releases its temporary owner but does
not pretend that the persistent cache was destroyed.

### Verification checkpoint

The selected addon library passes 68/68 release tests and the selected host
passes 666 with 24 ignored. The six-file offline retained lifecycle suite passes
270/270, including the new exact projection-order contract. Package contracts
pass 162/162; types, exact dependency/boundary checks, default addon check, fmt
and whitespace checks pass. Strict release Clippy passes standalone and combined
selections. Rebuilt standalone and combined artifacts both pass Worker teardown.

Against the dedicated loopback Chrome, the NODE-186 focused live case passes
twice standalone and once combined. Both complete raw binding lanes pass 27/27.
They prove that a real marked owner and ten roots remain on the same parked
ticket with one additional 1 MiB FFI admission until abort returns every
temporary owned metric to its exact warm baseline. The first standalone full
run passed 26/27: the new assertion captured its baseline while the warm-up mark
call still held asynchronous cleanup leases. The corrected test waits for that
temporary lease count to drop, while retaining the persistent cache by design;
the two final full runs pass without widening deadlines or closing the three
unrelated XLQY tabs.

### Remaining integration gates

- Consume this exact `RetainedMarkedFileInput` in native DOM prevalidation;
  releasing and re-adopting the original would reopen an ownership race.
- Reconcile prevalidation's `ResolutionError` cleanup surface with the retained
  marked owner's `MarkerOperationError` without flattening typed cleanup failure.
- Continue the same progress through `onBeforeInputAction`, late utility-world
  byte serialization, mutation and terminal cleanup.
- Add the remaining shutdown/publication races and public Playwright
  differential suite before exposing `ElementHandle.setInputFiles`.

## NODE-185: Session call identity and one Node deadline

The selected marker profile now establishes the public-call envelope before
materialization. After the server-direction schema succeeds,
`handleBeginFileInputCall` allocates `call@N` from a monotonic counter owned by
that exact `NativeSession` preparation scope. It is independent of preparation
receipts and the Rust command runtime's private call IDs; sibling session scopes
start at one, failed later phases may leave gaps, and overflow fails explicitly.
The ID stays in the Rust record through every remaining phase. Target marking no
longer accepts any caller-supplied JavaScript ID.

The ordered selected path is now:

`server schema -> call ID -> onBefore checkpoint -> source recheck / deadline -> materialize -> mark`

The empty `onBefore` checkpoint remains an awaited language boundary. After it,
`handleStartFileInputDeadline` revalidates the exact retained source and returns
the already server-validated raw timeout without coercion. The JS coordinator
then creates one Node `setTimeout` deadline immediately before materialization.
The same progress races both outer materialization and native marking; it is not
restarted at the Rust boundary. Node therefore preserves event-loop delivery:
synchronous getters or serializers may run past nominal time until the timer can
execute. Rust starts no competing clock.

When the timer wins, Node emits the exact internal `PWRIGHT_TIMEOUT` Error and
calls the feature-gated `abortDeadline` edge to revoke the exact preparation.
That edge is distinct at the N-API boundary, but the current Rust revoker still
models ordinary ticket cancellation; no typed timeout reason is transported
through the FFI. Every failure aborts the preparation and the coordinator clears
its timer in `finally`. The old raw `prepareServerPayloads` path may still stop at
`Materialized`; the internal `prepareMarkedTarget` path composes materialization
and marking under this one progress owner.

### Resource and boundary proof

There is still one 1 MiB FFI command admission and no second native slot. The
per-session `Cell<u64>` is included in the existing concrete 16 KiB registry
layout check, and the `Option<u64>` carried by all 32 records/snapshots is included
in the unchanged 16 KiB administration and synchronous-overlap checks. A live
truthy-timeout call adds one Node timer, one deadline Promise and one Error; those
V8 objects are scoped to the coordinator but do not constitute a byte-bounded JS
heap claim. Timeout zero/NaN creates no timer. Existing marker-cache, host
registration and browser-DOM ownership remain independently admitted.

### Verification checkpoint

The selected addon library passes 68/68 release tests, including monotonic,
sibling-scope, overflow and closed-scope call-ID cases. Strict release Clippy
passes both standalone and combined selections. The retained offline lifecycle
suite passes 269/269 (the prior 267 plus two coordinator/deadline contracts),
package contracts pass 162/162, and types plus boundary/dependency checks pass.
Standalone Worker smoke passes. Combined Worker smoke had one retained failure
where exit and forced termination both exceeded their bounds, followed by a
clean 26 ms pass; the failed execution remains evidence rather than being erased.

Against the dedicated loopback Chrome, the full internal raw binding lane passes
26/26 with both rebuilt standalone and combined marker artifacts. The three new
cases also pass 3/3 in focused runs. They prove one coordinator from
Materialized to Marked, `call@1`/`call@2` session sequencing independent of
preparation identity, and a real timer revoking an outer materialization await,
with owned resources returning to baseline. Before the final two cases were
added, the full standalone lane passed 24/24. A later 25-case run passed the new
deadline case but failed one existing connection topology initialization
(24/25); a retry under three unrelated XLQY Chrome tabs passed 20/25 and failed
five only at that same 5-second topology initialization boundary. No timeout was
widened and no unrelated tab or shared Chrome process was closed. Both later
full runs passed with the same three unrelated tabs still present, and a final
tab inventory showed no task-owned Page left behind.

### Remaining integration gates

- Project the materialized carrier, run retained DOM prevalidation and preserve
  target-marking order on this same ticket and deadline.
- Add `onBeforeInputAction`, late utility-realm byte serialization, mutation and
  cleanup without restarting progress or moving synchronous language errors.
- Carry a typed deadline reason through native cancellation only if public error
  attribution requires it; current timeout classification is deliberately owned
  by Node.
- Exercise publication-callback revocation, pending shutdown and Worker deadline
  races, then make the pinned public Playwright differential corpus green.
- Keep `ElementHandle.setInputFiles` absent until that composition and public
  error wrapping exist; this remains an internal raw coordinator.

## NODE-184: Same-admission Node target-marking handoff

The addon now has an explicit default-off `retained-target-marking` feature and
matching standalone/combined artifact profiles. The internal
`NativeSession::handleMarkFileInputTarget` advances an actual preparation from
`Materialized` through Rust target marking to `Marked`. It is intentionally not
called by the public file-input driver yet: callers still lack connection-wide
protocol call identity and the single public deadline that must begin before
materialization. Public `ElementHandle.setInputFiles` therefore remains gated.

### Exact ownership transition

Selected preparation admission creates `FfiPreparation<NativeMarkProgress>`
from the start; it does not retype a unit result or admit another FFI command.
The 32-entry Node registry keeps that move-only preparation while language work
owns six thread-affine roots. At the first native boundary it verifies the exact
Node session before exclusive checkout, takes the preparation, records its weak
revoker in the entry and leaves the roots revocable in place. Only then does the
host register the exact source/revoker and the worker upgrade the inert original
`NativeHandle` Weak. No `Rc`, `NodeReference`, N-API value or registry borrow
crosses the Rust future.

Native completion is deliberately an outer successful FFI result containing an
inner typed mark outcome and a move-only `FramePublicationFence`. This permits
`FfiTicket::park` to reinstall the same command slot and 1 MiB byte lease. The
Node callback installs that parked preparation under a fresh 16 KiB phase-scratch
receipt before publishing. It temporarily upgrades completion state only to
take the fence, drops that strong owner before any callback can reenter, then
revalidates the exact busy entry after publication. Only successful typed mark
completion replaces the record at `Marked`; error classification is flattened
at this final language boundary after preceding lifecycle publication.

Abort, scope close, environment shutdown and host revocation all converge on
the same entry. A running entry owns only a weak `FfiRevoker`; its `Drop` invokes
revocation after removal from the registry borrow. A parked entry instead owns
the original `FfiPreparation`, whose drop reclaims its typed result and retained
session registration. Failure to register, start, reserve future storage, park,
publish or revalidate drops an unfinished native/phase guard and removes the
entry. A foreign session is rejected before checkout and cannot cancel the
owner's valid ticket.

### Resource and feature boundary

No default, `retained-file-input` or previously named artifact changes feature
selection. `retained-target-marking` adds only four same-named flags: addon,
host, file-input composition and file-input backend; it adds no package. The
Node preparation still has 32 entries, six roots, one existing 1 MiB command
admission per entry and 16 KiB synchronous phase scratch. Registry construction
continues to check the concrete enlarged table against its 16 KiB administrative
allowance. Native progress/state/outcome/fence plus bounded call ID and overlap
have a selected compile-time check below that existing 1 MiB allowance. The
actual future body is separately charged by the command lane's existing 128 KiB
limit before boxing. Existing host registration and target-marker budgets remain
independent; these logical limits do not bound V8, DOM Set or browser RSS.

### Verification checkpoint

Current-source checks use locked release builds, one Cargo job and disabled
incremental compilation. The selected addon and host pass strict all-target
Clippy. Exact standalone/combined dependency fingerprints pass. Package
contracts pass 162 cases and types pass. The standalone artifact loads and
retires its gated classes in a real Worker; the combined artifact does too.
The retained lifecycle contracts pass 267/267 under the marker profile. The
selected host library passes 666 tests with 24 ignored browser lanes.

Against the dedicated loopback Chrome, the actual raw binding lane passes
24/24 cases for both the standalone and combined artifacts. Four
marker-specific cases prove successful `Materialized` to `Marked` parking with
one 1 MiB admission, synchronous abort of running/queued work, disposal after
materialization rejecting the bound original wrapper, and foreign-session
rejection without canceling the owner ticket. Every fixture explicitly retires
its Contexts, connections, Handles and preparations; final
host/FFI/preparation counters reach zero. Each task preflight tab was closed and
`pwright tab-list` was empty afterward. This is real Node/native/browser
evidence, but still an internal raw method rather than a public upload pass.

### Remaining integration gates

- Establish public connection-wide call identity and one action deadline before
  materialization, including faithful Node event-loop timer delivery and typed
  cancellation reason; the internal caller-supplied ID starts no new timer.
- Automatically compose marking with raw-carrier projection, retained DOM
  prevalidation, input instrumentation, late utility serialization and the
  mutation/cleanup phase on the same admission.
- Add deterministic publication-callback revocation, pending native shutdown,
  Worker teardown and deadline race coverage, then run the pinned public
  Playwright differential corpus. Current synchronous abort is not every race.
- Keep the raw marker method internal until that sequence and public error
  wrapping are complete; do not claim `ElementHandle.setInputFiles` parity.

## NODE-183: Host-owned utility-world marker cache

The safe Rust host now has a default-off `retained-target-marking` feature.
It selects `retained-file-input` and the matching composition/backend marker
feature, without selecting any new package. Existing host `retained-file-input`
and all addon/Node profiles remain unchanged. The Rust-only
`HostSession::handle_mark_target` runs original-target adoption, marking and
temporary-group cleanup; it is not public Node `ElementHandle.setInputFiles`.
NODE-182 below remains the historical primitive-only checkpoint.

### Independent persistent and per-call owners

`ChromiumFrameWorldAccess::existing_frame_utility_world(frame)` selects only
the exact registered Frame under the authoritative route lock. It performs no
browser I/O, readiness wait or main-world substitution. A live Frame with no
utility world returns `None`; missing registration and crashes remain errors.
Each successful selection reserves one existing 8192-byte snapshot permit and
an independent native-retirement receipt. It does not clone another receipt's
single-waiter authority. Main-world replacement does not invalidate an unchanged
utility world; an observed utility retirement remains sticky.

The host cache keys Page, Frame, client Arc identity and unique utility-world
ID. It admits 64 current/creating entries, eight retiring generations and eight
lookup/call leases independently. Concurrent same-world lookups share creation
and reuse the persistent marker after calls finish. Generation checks prevent
old completion from clearing a replacement. Failed or abandoned native cleanup
closes further admission rather than reporting successful release.

Only the independently admitted utility snapshot is retained by the persistent
world owner: no `PendingHandle`, Handle-call allowance, upload lease or active
operation token. The temporary adopted target's route owner instead retains
the source Handle, paired snapshot, tracked-call admission and marker lease
through acknowledged target cleanup. Caller cancellation cannot return these
allowances while native work still owns them.

At every serialized host topology boundary, bounded pruning examines exact
route currentness, including removed Frames and closed Pages. A retained wakeup
permit makes cleanup completion observable even before the next driver select.
Pruning, factory callbacks, route callbacks, native-owner drops and wakeups run
outside the cache lock. Currentness is checked again before creation publication,
lookup delivery and marking. If retirement headroom is exhausted, stale slots
remain charged and unusable until actual cleanup; route disappearance alone
does not return capacity. Sealing happens before host work drains; persistent
native cleanup follows owned inspector-session retirement.

### Resource and failure contract

| Selected component | Scope | BackendStateBytes | Other capacity |
| --- | --- | ---: | --- |
| Fixed cache, 72 world/driver allowances and eight lookup/call allowances | 183/1 | 1,343,488 | 81 maximum leases |
| Native marker backend | 183/2 | 1,625,392 | 160 RemoteHandle units |
| Persistent composition state owners | 183/3 | 1,180,160 | 72 LiveObject units |
| Temporary composition call owners | 183/4 | 131,584 | 8 LiveObject units |

These four components total 4,280,624 declared backend-state bytes across four
scopes and 244 maximum leases. They are additional to 72 shared snapshot slots
and expanded receipt-registry metadata. The selected snapshot capacity becomes
153, or 1177 with drag-runtime. Creation has 72 injected untimed continuation
reservations; marker state/call cleanup has 80, independent of the existing
eight adoption continuations. Fixed table/control storage has compile-time size
checks, while generic cache-future storage has checked static-size assertions.
Retained host diagnostics are copied into fresh,
total-at-most-1024-byte string backing; truncating an oversized String in place
would not satisfy this bound. Native creation errors keep their typed structure.

These are declared Rust/native-wrapper allowances, not measurements or bounds
on browser RSS, allocator overhead or DOM graphs held by the persistent Set.
Persistent marker state intentionally survives disposal of the source Handle.

The caller supplies the protocol call ID; the internal runtime command ID is
not substituted. Empty metadata skips marking before source validation or world
admission, including for a disposed Handle. This phase creates no new deadline.
Native adoption, JavaScript/protocol and cleanup details remain typed; failure
classification must not infer recoverability from strings.

### Verification checkpoint

Final-source verification passed under `.work/node183-host-marking/`. All Cargo
checks use release/locked dependencies, one build job and disabled incremental
compilation. The first selected host run had 648 passes, nine failures and 24 ignored tests:
the nine pre-existing upload fixtures expected two fixed installation leases,
but selected marking adds four. Only the still-installed fixed baseline was
updated; final zero-resource and zero-lease checks remain intact. This initial
failure is not recorded as a passing gate. The next run had 661 passes and one
new fixture failure: retained adoption requires two native-owner units, not one.
Its profile was corrected without weakening the real-adoption/stale-mark or
cleanup assertions. Those failed runs remain in `host-selected.log` and
`host-selected-final.log`; only the later verified source is counted below.

| Final-source gate | Observed result | Evidence |
| --- | --- | --- |
| Default host | 429 passed, 8 ignored | `host-default-regression.log` |
| Existing retained-file-input host | 637 passed, 23 ignored | `host-retained-regression.log` |
| Standalone marker host | 666 passed, 24 ignored | `host-selected-verified.log` |
| Marker + concurrent-close + extra-headers + drag-runtime + frame-navigation host | 728 passed, 26 ignored | `host-combined.log` |
| Full Chromium composition | 395 passed, 16 ignored; includes 14 new exact-Frame utility cases | `composition-full.log`, `utility-single.log` |
| Strict all-targets Clippy | Default/standalone/combined host and Chromium composition pass | `clippy-default-host.log`, `clippy-selected-final.log`, `clippy-combined.log`, `clippy-composition.log` |
| Exact marker dependency closure | Standalone and combined add only three marker flags; existing host/addon selections unchanged | `dependencies-final.log` |
| Runtime JS and resolved addon boundaries | Pass | `boundary.log` |
| Package contracts / types | 161 passed / pass | `package-contracts.log`, `types.log` |
| Pinned marker / DOM-policy offline comparisons | 21 / 35 passed | `target-marking-oracle.log`, `dom-oracle.log` |
| Workspace formatting / whitespace | Pass | `fmt-final.log`, final `git diff --check` |
| Assembled real Chrome host marker | One distinct case passed in both final host selections | `live-selected-final.log`, `live-combined-final.log` |

The host adds 29 deterministic checks: 23 cache lifecycle/reentry cases, two
bounded diagnostic cases, three typed failure-classification cases and one
selected snapshot-capacity proof. The cache tests include 73 successive Frame
generations, stale creation without a pruning callback, stale held-lease refusal,
independent capacity limits, cancellation and lock-free wakeup/factory reentry.
The three failure-classification tests preserve the original typed Arc payload.

The assembled Chrome case uses real host-produced A/B ElementHandles and host
APIs, not raw-CDP substitutes for marking. It verifies repeated-ID reuse,
different-ID behavior, an empty-ID no-op after source disposal, marker state
surviving both source disposals without Handle descriptors or operation tokens,
navigation to a new utility world and completed predecessor retirement. Its
owned Context and attachment are always cleaned up, and all declared resource
classes, active leases and SessionMetrics reach zero. The same case also passed
an earlier source snapshot in `live-selected-initial.log`; that is not another
distinct case. The browser was reused without restart through dedicated
loopback CDP; this task closed only its own preflight tab and owned Contexts.
These results do not prove Node ticket promotion, pending-native host shutdown
races, arbitrary DOM graph reclamation or completed retained upload.

No addon artifact rebuild, additional commit or push was performed. The local
foundation HEAD remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`. All build/test
processes launched for this checkpoint exited, with no task-owned background
service left running. A remaining Cargo process was verified to belong to the
separate `/Users/cyuan/projects/xlqy-mobile` checkout and was not interrupted.

Read-only memory observations at 2026-09-17T15:51:11Z and 16:09:39Z reverified
the shared backend PID 12747: RSS 392,400 to 468,336 KiB (about +74 MiB), with an
intermediate 518,096 KiB sample at 16:08:04Z that subsequently fell about 49 MiB.
Targeted `vmmap -summary` reported physical footprint 396.7M and lifetime peak
1.2G; these are separate measurements, not RSS totals or per-chat attribution.
Shared renderer PID 12750 changed from 884,544 to 873,776 KiB RSS, and ChatGPT
PID 12352 from 411,472 to 397,120 KiB. System swap remained zero. No heap dump,
full-transcript loading, service restart or memory cleanup was performed. The
observed backend increase remains unattributed across the shared tasks; neither
these snapshots nor the resource ledger establish an application memory leak.

### Remaining integration gates

- Promote the actual Node preparation registry from unit results to typed
  progress while preserving the same move-only FFI ticket, exact revoker and
  publication fence. This Rust phase alone does not publish a Node completion.
- Establish connection-wide protocol call identity and the original deadline
  before materialization, preserving Node event-loop timer delivery and explicit
  upstream race boundaries rather than resetting a native timer on entry.
- Compose marking with raw-carrier projection, retained DOM prevalidation,
  input instrumentation, late utility serialization and mutation. Public
  retained upload and complete differential parity are still absent.
- Add assembled pending-creation/adoption shutdown and language-handoff races;
  deterministic cache cancellation tests do not prove those end-to-end paths.

### Source-backed next slice, implemented by NODE-184

The addon preparation now stores typed progress, upgrades the original wrapper
only at native dispatch and retains the exact-session registration through
running and parked states. All six Node roots remain thread-affine. The ownership
constraints below are retained as the rationale for the implemented bridge.

Do not reuse `boundary.rs::spawn_with_env` unchanged: it calls `ticket.deliver`
and consumes the permit. A parking bridge instead needs to return the owned
ticket and move-only `FramePublicationFence`, park the ticket before invoking
publication callbacks under separately admitted phase scratch, and revalidate
after reentry. Neither a second FFI admission nor a dummy resume to extract the
fence preserves the intended ownership boundary. Abort/close must still reach
the running weak revoker outside registry borrows.

Automatic public chaining remains gated on operation control established before
`file-input-dispatch.cjs` materialization: connection-wide identity, original
deadline, Node-event-loop timer delivery and typed cancellation reason. A
revoker reporting TargetClosed is not a timeout implementation. Required tests
include same-permit accounting, foreign-session rejection without canceling a
valid ticket, late disposal, publication-triggered revocation, queued completion
after close, and actual marking before carrier getters. This is a next-step
design constraint, not implemented Node behavior or a public upload claim.

## NODE-182: Persistent native target marking

The existing file-input backend and composition now expose a separately
selectable `retained-target-marking` feature. It implements actual native
marking, but no safe-host or addon profile selects it yet. Public
`ElementHandle.setInputFiles`, typed Node/native handoff and the final upload
pipeline remain incomplete. NODE-181's selected Node artifacts are unchanged.

### Persistent world state versus temporary call ownership

`CdpTargetMarkingService` creates a private remote `{state: undefined}` object
in its own uniquely named group, bound to the exact client identity and unique
utility-world ID. Injected group IDs must never be reused within the same native
session. The caller owns world selection, caching, replacement and shutdown;
this primitive creates no world registry, timer, task, driver or browser.

`TargetMarkingOperationService` separately admits persistent states and calls.
Its `OwnedTargetMarker` survives normal call completion and cancellation. A
mark consumes a settlement-owned adopted target, verifies the exact
service/client/world before native work, and produces `OwnedMarkedElement`.
That owner retains both the target and the mark call's extra result/exception
allowance until acknowledged group cleanup. It cannot return a bare target
while prematurely discarding the additional allowance. The original node is
marked, not a selector requery or a label-retargeted input.

Started creation, marking and explicit release all carry pre-reserved cleanup
continuations. Dropping a waiter continues native settlement and cleanup;
dropping the persistent owner defers its release until in-flight marks settle.
An explicit busy release returns the same owner unchanged. Cleanup failure or
abandoned executor work poisons admission before returning its allowances.
Cleanup requires an acknowledgement or an exact authoritative retirement
receipt, never a changed route or matching error text. A caller-supplied
single-waiter world receipt must not be concurrently polled by multiple group
releases; future host integration needs independently admitted receipts or
serialized terminal release ownership.

The backend declares `RemoteHandle = 2 * (states + calls)` and
`BackendStateBytes = 512 + 20,020 * states + 22,930 * calls` for these scripts.
Separate atomic quotas prevent spare combined-ledger capacity from exceeding
either selected pool. State/call composition owners, route receipts, cleanup
reservations and transport messages have separate admission. Bounded wrapper
counts and Rust storage do not bound DOM graphs retained by the marker's Set,
browser memory, allocator overhead or application RSS.

### Native semantics and protocol boundary

The scripts follow pinned Playwright 1.58.2's original-element guard and
persistent target-marking policy: repeated call IDs unmark removed nodes before
marking new nodes; a new ID clears the previous state without unmarking its
old set. State commits after dispatch. An empty ID or non-element does not
change state. Dynamic realm `Set`, `CustomEvent`, membership, iteration and
`dispatchEvent` behavior are preserved, including thrown exceptions.

A new CDP call explicitly requests `returnByValue: true`, `awaitPromise: true`
and the supplied `userGesture`. It returns the raw result/exception envelope,
owns transient wrappers in the adopted target group and waits through native
settlement instead of the ordinary response timeout. Existing grouped helpers
are unchanged. Typed transport, protocol, JavaScript and cleanup errors remain
distinct; eventual host recoverability policy must not turn admission or
unconfirmed cleanup failures into `error:notconnected`.

### Verification checkpoint

Evidence is under `.work/node182-target-marking/`. Cargo uses release, locked
dependencies, disabled incremental compilation and one build job.

| Final-source gate | Observed result | Evidence |
| --- | --- | --- |
| Complete CDP library | 124 passed, including four new raw-settlement wire checks | `cdp-full.log` |
| Default backend/composition | 16 / 17 passed | `default-final.log` |
| Standalone marker backend/composition | 37 / 36 passed; one explicitly ignored live test | `marker-final.log` |
| Marker + preflight + large-command backend/composition | 55 / 51 passed; one explicitly ignored live test | `marker-combined-final.log` |
| Strict all-targets marker Clippy | Standalone and combined pass | `clippy-marker-{standalone,combined}.log` |
| Strict all-targets ordinary CDP/addon and retained addon Clippy | Pass | `clippy-default.log`, `clippy-addon-retained.log` |
| Resolved independent feature/package boundaries | Pass; host/addon marker selection remains absent | `dependencies.log` |
| Marker script versus pinned policy | 21 passed: 19 paired descriptor/language cases and two creation contracts | `target-marking-oracle-final.log` |
| Package contracts / existing DOM-policy oracle / types | 161 passed / 35 passed / pass | `package-contracts.log`, `dom-oracle.log`, `types.log` |
| Runtime JS and resolved addon boundaries / workspace formatting | Pass / pass | `boundary.log`, `fmt.log` |
| Real Chrome marker primitive | One distinct case passes standalone and combined | `live-marker-{standalone,combined}-final.log` |

The new deterministic coverage comprises 21 backend and 19 composition cases;
the table includes existing tests too. Four added composition cases check
cross-service rejection on the same client/world and independent state/call
capacity without extra I/O. The offline Set-constructor regression preserves
prior state when incoming `new Set([node])` throws before changed-ID reset.
Offline descriptors are not real browser evidence. No addon artifact rebuild or
new Node/native behavior is inferred from these primitive checks.

The actual Chrome test
`real_chrome_marker_persists_across_calls_and_releases_owned_groups` passed in
both final selections (and once earlier during implementation).
It makes nine marking calls against real adopted DOM nodes and checks eight
ordered events, persistent same/different-ID behavior, no-op text/empty-ID
calls, label identity, shadow composed propagation, event flags/detail and
unchanged file contents. It releases temporary groups after each call, then
the persistent group, fixture group, owned Context and transport; all declared
resource classes reach zero. It does not test world replacement, the Node
handoff, DOM prevalidation, timeouts or completed upload. Chrome was reused
without restart through the dedicated loopback CDP profile; only the task's
preflight tab and fresh Context were closed.

All task-owned build/test processes have exited; no task-owned background
service remains. No additional commit or push was made. At
2026-09-17T15:19:12Z, the dynamically reverified shared backend PID 12747 had
RSS 429,328 KiB (about 419 MiB), versus 423,232 KiB at 15:09:27Z (about +6 MiB).
ChatGPT PID 12352 had RSS 426,464 KiB; dedicated shared Chrome PID 32415 had
RSS 411,840 KiB. System swap remained zero. These shared-process RSS measures
cannot be attributed to this chat or added as unique physical memory. No heap
dump, transcript loading, browser restart or memory cleanup was performed.

### Next integration gates

- Install a bounded exact-world marker owner in the selected safe host, with
  separately admitted persistent state and exact retirement/route ownership.
- Change the actual preparation registry from a unit FFI result to typed
  progress at initial admission. Running/parked transitions must preserve the
  same move-only ticket, revoker registration and publication fence; late NAPI
  completion must not resurrect a revoked entry.
- Establish connection-wide call identity and one deadline before
  materialization. Pinned `server/progress.js` uses Node timer delivery and
  explicit `progress.race`; it does not globally race the entire task. A native
  timer firing while synchronous JavaScript blocks its event loop can reorder
  observable errors. Preserve language-event-loop delivery at the relevant
  boundaries rather than adding eager native elapsed-time checks or restarting
  an integer timeout on native entry.
- Only then compose original-target acquisition/adoption/marking, raw carrier
  projection, retained DOM prevalidation, input instrumentation, late utility
  serialization and mutation. Preserve recoverable-versus-fatal utility errors
  and retain native/registration owners through cleanup and publication.

### Source-backed host follow-up, not yet implemented

The Chromium composition's `page_utility_worlds.rs` already owns utility-only
snapshots and independently admitted exact-world retirement receipts. Its
existing accessor returns an immutable Page-wide batch (up to 32), not a
transferable single-Frame owner. A narrow `existing_frame_utility_world(frame)`
accessor is the next reusable building block: select under the authoritative
route lock, reserve the existing 8192-byte snapshot allowance before cloning,
and return an independent receipt without browser I/O or readiness waits.
Later integration must compare this snapshot with the per-call paired snapshot
using Page, Frame, client identity and unique utility-world ID.

Do not put the current host `handles/file_input.rs::RouteOwner` into persistent
marker storage: it contains a PendingHandle, Handle call allowance, operation
tracker token and upload call lease. Retaining it per world would consume the
finite call pool and could prevent shutdown drains. Persistent world ownership
needs only its independent utility snapshot, bounded metadata and a
generation-safe cache/retirement lease. Stop cache admissions at the existing
cancel/seal boundary before operation drains, retain native receipts through
owned inspector-session retirement, and only then finish persistent cleanup.
Actual cleanup completion, not marker Drop or route disappearance, retires a
cache slot. Failure/fallback teardown must follow the same ownership discipline.

The current host limits Frames to 64 and Handle calls to eight. A candidate
cache policy is 64 current/creating worlds plus eight separately admitted
retiring generations, with eight call slots. The retiring bound is a proposed
new backpressure policy, not a consequence of the existing call count: the
ninth blocked retirement must wait or refuse before creating a successor.
If selected, 72 state slots plus eight calls require 160 backend native units,
72 extra snapshot slots and 80 separate marker cleanup reservations, in
addition to existing adoption capacity. Derive and check metadata, native
receipt and future storage before accepting those numbers into the host;
none of these proposed capacities is installed by NODE-182.

## NODE-181: Original native wrapper binding

Selected preparation now preserves the actual `NativeHandle` supplied at
ingress, independently of the arbitrary JavaScript receiver and copied source
metadata. A private `OriginalHandle(Weak<Mutex<State>>)` is captured after FFI
and synchronous-phase admission, before the first NAPI root allocation. Every
`InputRecord`, `RootSnapshot`, scoped snapshot and record replacement carries
that same binding. Capture/clone perform only `Arc::downgrade`/`Weak::clone`:
there is no state lock, upgrade, disposal check, native-use authority or strong
native pin. In particular, this does not move native errors before observable
owner lookup, files/options getters or conversion.

The binding is inert identity plumbing. Preparation still uses
`FfiPreparation<()>` and stops at raw Materialized; it does not yet register the
NODE-180 native revoker, acquire the original native target, mark an element,
invoke retained prevalidation or implement public `ElementHandle.setInputFiles`.
Stale Weak references can retain dead Arc backing after active admission ends;
zero ledger metrics do not establish heap/GC reclamation or a per-chat RAM bound.

### Unchanged admission, with independent scratch proof

The actual `Registry<InputRecord>` layout check covers all 32 enlarged entries
inside its existing 16 KiB administrative allowance. Two new addon tests use
that concrete registry: exactly 32 initial admissions succeed, the 33rd refuses
without changing metrics, and closing reclaims admission while weak receipts
survive. They construct no NativeHandle, NAPI roots or remote objects.

One concrete record/snapshot overlap sum is now checked at compile time against
both the existing 1 MiB call allowance and the independent 16 KiB synchronous
phase allowance. The second proof matters because a reentrant callback can
revoke the FFI entry while temporary Rust values are still on the stack.
`PublicationPhaseStorage::CAPACITY_BYTES` derives from the same constant used
by actual reservation; the held-after-budget-drop regression compares its real
ledger charge with that constant and then checks release. No capacity, class,
dependency, feature or public method is added or enlarged.

### Actual-ticket coverage and observation boundaries

Three new browser cases cover substituting a different live native receiver,
explicitly disposing the original source during deferred owner lookup, and a
later files getter retaining its exception identity/order after that disposal.
They keep real raw wrappers/tickets alive through cleanup. Existing copied
metadata could satisfy these cases; they do not test a later Weak upgrade,
wrapper collection, target marking or completed upload.

The owner-wait observation explicitly permits its one pending preparation
Command admission while requiring native work to be idle and existing watch
work unchanged. Pure numeric-sample tests exercise this observation predicate;
they are not substitute browser evidence. Final close observations must instead
wait for fully reclaimed host, FFI and preparation resources. Stable samples
with a remaining watch are not final retirement. Failure diagnostics retain
the original thrown values and label bounded nested cleanup causes.

Active-session baselines additionally require no queued FFI result, with all
pending/running work accounted for by armed watches. The owner-wait case adds
exactly its one unstarted preparation admission. Full before/after snapshots
are still compared; no counter is discarded and no deadline is increased.

### Verification checkpoint

Evidence is under `.work/node181-original-handle/`. Cargo commands use release,
locked dependencies, disabled incremental compilation and one build job.

| Final-source gate | Observed result | Evidence |
| --- | --- | --- |
| Selected addon explicit `--lib` suites | 65 passed per profile | `addon-{standalone,combined}-final.log` |
| Actual phase-storage lifetime/charge regression | 1 passed | `phase-storage-final.log` |
| Rebuilt addon offline matrix | 268 passed per profile | `{projection,dispatch,materialization,channel,arguments,preflight,smoke}-{standalone,combined}.log` |
| Package regressions | 161 passed, including 10 observation/diagnostic tests | `package-contracts-active-watch.log`, `binding-observation-active-watch.log` |
| Type check and descriptor/language DOM oracle | Pass; 35 oracle cases | `types-final.log`, `dom-oracle-final.log` |
| Strict host/addon all-targets Clippy | Default, file-input, handles+file-input, retained standalone and combined pass | `clippy-*.log` |
| Formatting and resolved boundaries | Pass | `fmt.log`, `boundary.log`, `dependencies.log` |
| Actual-ticket real Chrome binding | 20/20 standalone, 20/20 combined, 20/20 standalone repeat | `live-binding-{standalone,combined,standalone-repeat}-watch-settled.log` |

The live gate contains **20 distinct cases executed three times**, not 60
distinct cases. Every final run has zero failures, cancellations or skips and
observes fully retired native-host/FFI/preparation resources. The seven offline
lanes contain projection 47, dispatch 20, materialization 76, channel 61,
arguments 36, preflight 27 and Worker smoke 1 per artifact. The prior NODE-180
full host results are historical; NODE-181 reruns the focused changed host
storage regression, not the entire host suite.

Keep earlier failures as evidence, not overwritten green results:

- `live-binding-standalone.log`: five passed, case six failed in cleanup with
  an unclassified AggregateError; fourteen subsequent fixture blocks were not
  independent behavioral failures.
- `live-binding-standalone-diagnostic.log`: the first cleanup observed
  `ffiPending = 1` after accepting two command-idle samples; nineteen later
  cases were blocked. Bounded nested diagnostics exposed the premature final
  retirement observation. Requiring all final resources to reach zero fixed
  this class of sampling error.
- `live-binding-{standalone,combined}-final.log`: 20/20 each after that fix.
  `live-binding-standalone-repeat.log` then passed 19/20: case seven compared an
  active baseline with one queued watch against the same watch after rearming.
  All twenty cleanup phases succeeded. Requiring armed-watch accounting fixed
  that second sampling error; the final three runs above followed this change.

The fixture retains its 20-second setup/exercise bound, four three-second
cleanup phases, 35-second case limit and independent process supervisor.
Diagnostic timeout cannot satisfy an expected native rejection. Neither fix
changes production behavior, ignores cleanup failure or weakens final counters.

Final rebuilt artifacts, unchanged by the subsequent test-only fixes:

| Profile | Bytes | SHA-256 |
| --- | ---: | --- |
| `retained-file-input` | 17,797,248 | `5b9b4fe02731a74bcae6a328fcc4f3849926fdd462a2fee55415dcc2b7927b0b` |
| `retained-file-input-combined` | 19,470,224 | `a047516b2832cb80635b487173339eb72d65d18cdbd7b63976bfa259d6b8184a` |

The shared dedicated Chrome had recovered externally before this gate. The
task verified loopback-only CDP, used its own Pwright working directory and
closed only its own blank preflight tab and owned test contexts. The two
pre-existing pages remain; no shared browser restart, additional commit or push
was performed. All task test/build children have exited and no task-owned
background service remains.

At 2026-09-17T14:43:48Z, dynamically verified shared backend PID 12747 had RSS
402,784 KiB (about 393 MiB), versus 379,568 KiB at the resumed 14:34:33Z snapshot
(about +23 MiB). Targeted vmmap reported a separate physical footprint of
335.8 MiB, versus 308.8 MiB at 14:28:38Z. These shared-process measures cannot
be attributed to this chat, and are not added together. System swap remained
zero; artifact sizes and resource allowances do not establish application RAM.

### Next native integration: preserve upstream observable boundaries

The pinned Playwright 1.58.2 implementation starts its progress deadline before
materialization, marks the original target before reading the materialized
carrier, then retargets/checks DOM state and performs late utility serialization.
Do not reuse the eager host upload path, substitute a preparation counter for
connection-wide protocol call identity, or start a fresh integer timeout only
at native entry. Metadata/deadline preparation must not hold an unstarted native
work token across JavaScript awaits.

Marking retains a per-utility-world `{callId, elements}` state across calls and
operation abort/disposal. A persistent private remote object/group needs exact
connection/session/Frame/context identity and its own storage, snapshot and
cleanup admission; an attempt-scoped group or the existing in-flight world
allowance cannot own it. Context replacement alone is not release acknowledgement.
Preserve same-call element event differences, different-call replacement and
the upstream `userGesture` behavior without adding early file-input/attachment
checks. Typed recoverable not-connected outcomes may continue to carrier
getters; fatal errors and unconfirmed cleanup must not be flattened into them.

At the actual native boundary, acquire/init world state, validate and borrow the
original weak target, adopt it with retained group ownership, revalidate, mark,
and settle temporary group cleanup before advancing. The same FFI admission
needs typed prepared/running/parked continuation ownership, a move-only
publication fence and exact originating-scope validation. Register the existing
session revoker immediately before native work and retain its receipt through
cleanup, not during language-only preparation. These are remaining design and
implementation requirements, not completed NODE-181 behavior.

## NODE-180: Session-bound revocation before native drains

The selected safe host now owns a bounded registry of weak FFI revokers, with
source-bound registration/check APIs and explicit close/raw-peer/failure teardown
hooks. This implements the host-side prerequisite identified by NODE-179.
It does **not** promote the actual Node preparation: that still stores a unit
FFI result and stops at Materialized. Native marking, typed progress, the original
native-wrapper weak binding, retained prevalidation selection, late utility
serialization and public retained upload remain required.

### Register at the native boundary, not initial language preparation

`HostSession::register_retained_file_input` verifies connected state and exact
Element source provenance while holding the same session lock as admission,
then installs a weak FFI hook. The move-only `RetainedFileInputRegistration`
contains copied source metadata and a generation-safe weak registry receipt.
`check_retained_file_input_registration` verifies the exact Owners origin,
target, creation Frame/Page and registry generation. Neither operation checks
dispatcher disposal, runtime/native liveness, current world or DOM ownership.
Those checks cannot be moved ahead of their proper observable native boundary.
The receipt grants cancellation registration, not native-use authority.

The host does not infer a Node preparation scope from an arbitrary Rust
`FfiRevoker`. Eventual Node integration must verify the exact originating scope
and bind registration to the initially admitted typed progress entry before
native execution. Keep the receipt with the native continuation through group
cleanup; dropping it unregisters its hook without cancelling the FFI entry.
Register only after Materialized, immediately before the first native phase,
not during owner lookup, argument conversion, dispatch/schema or materialization.
The existing 32 language-only preparation slots are unchanged; the new eight
native-stage slots must not prematurely reduce that capacity or reorder errors.

### Exact fixed storage and weak ownership

The registry lives in selected `RetainedUploads`: eight slots, monotonically
checked generation IDs and fail-closed admission. Slot reuse cannot let an old
receipt remove or validate a new entry, even when indices/generations match in
another session. The separately claimed `BackendStateBytes` allowance is
**4 KiB**, one scope `extension(169, 2)` and one fixed lease. It is combined
through the existing selected file-input requirements; no package or feature
is added, and no ordinary Node/transport/payload limits are enlarged.

Compile/runtime size checks include the index Mutex/state, Arc header, detached
batch, all eight full source-bound receipt values and bounded control storage.
This is an active administrative allowance, not measured RSS or a global heap
bound. Stale external Weak receipts may retain dead Arc backing after Inner and
its lease have dropped; zero ledger metrics do not prove that heap is reclaimed.
Native/FFI results, task tokens and their separate budgets remain elsewhere.

`seal_and_take` seals and moves only weak hooks; it invokes no wakeup or native
destructor under the caller's lock. The detached batch retains the fixed lease
through its callouts. Its consuming `revoke` and armed Drop drain hooks outside
locks, including unwind/forgotten-explicit-revoke fallback. The batch must never
be dropped while a caller admission lock is held. Poison recovery is cleanup-only.
Running FFI revocation still retains native future storage until actual Drop;
parked results are reclaimed without requiring a Node callback.

### Stop admission before either drain, with wakeups outside locks

The actual driver now uses `shutdown_admission::cancel_then_seal`: cancel the
session stop token outside locks, acquire the session-state admission barrier
and detach weak revocations, release the guard, then close the operation tracker
outside locks. Both existing token issuers check the same stop token and publish
under that lock. Thus an admission which read the earlier connected state must
finish publication before the barrier; later admissions reject cancellation.
No new shared atomic close flag or broad admission rewrite is needed.

The driver invokes the detached batch **before** `task_operations.wait()`
and the later Handle drain, on both explicit stop and raw peer termination.
`SessionTeardown::drop` first removes live state under its lock, then revokes
outside that lock before closing Handles. `RetainedUploads::shutdown` provides
an additional idempotent fallback, not a substitute for the earlier hook.
The driver also clones its Handle installation before calling `close`, rather
than invoking it under the session-state mutex.

Independent review reproduced a second wake-under-lock defect in
`Owners::close`. Owner admission is now sealed and tickets detached under its
own mutex; tracker close/wakeups occur afterward, before ticket submission.
The regression's old implementation fails a nonblocking lock observation rather
than hanging. These bookkeeping fixes do not by themselves prove browser group
release or the assembled session's native-retirement result.

The two native-retirement failure branches also previously requested transport
disconnect under the session lock. `CdpConnection::request_disconnect` cancels
the transport and rejects pending replies synchronously, so their wakeups can
reenter host admission. Both branches now clone the exact connection under the
guard and disconnect after releasing it, preserving the retirement error and
drain order. This correction is source-reviewed and recompiled in the selected,
combined and file-input-only configurations; an assembled HostSession failure-
path regression is still absent.

### Verification checkpoint

Evidence is recorded under `.work/node180-session-revocation/`. Release commands
use locked dependencies, disabled incremental compilation and one build job.

| Gate | Observed result |
| --- | --- |
| Default host full unit suite | 429 passed, 8 ignored |
| Handles-only host full unit suite | 498 passed, 14 ignored |
| Final retained-file-input host full unit suite | 637 passed, 23 ignored |
| Final combined host full unit suite | 699 passed, 25 ignored |
| Selected addon explicit `--lib` unit suites | 63 passed per profile |
| Strict all-target host/addon Clippy | Default, file-input-only, handles+file-input, retained standalone and combined all pass |
| Formatting and dependency gates | `cargo fmt --all --check`, Node boundary and file-input dependency checks pass |
| Rebuilt standalone/combined addon offline checks | 268 passed per artifact, zero failed/cancelled/skipped |
| Package regressions | 151 contracts, type check and 35 descriptor/language DOM-oracle cases pass |

Default and Handles-only tests preceded the final disconnect-only correction;
the final selected/combined suites and five Clippy configurations followed it.
The combined selection adds concurrent-close, extra-headers, drag-runtime and
frame-navigation to retained-file-input. No browser-dependent ignored test was
converted into an offline success.

The selected host includes **28 new regressions**: 15 bounded-registry tests,
eight actual registration/FFI/Owners tests, four shutdown-admission helper tests
and one Owners-close wakeup regression. The latter first failed against the old
lock order (`owners-close-red.log`) and then passed (`owners-close-green.log`).
The eight integration fixtures use real Owners work tokens but no remote Handle
allocation. They prove parked-result reclamation without a Node callback and
running-future storage/admission retention until actual destruction, not native
object-group acknowledgement. The helper separately proves lock-free token/
tracker wakeups and a deterministic in-flight admission race; it does not
instantiate HostSession.

Each rebuilt artifact passes projection 47, dispatch 20, materialization 76,
channel 61, arguments 36, preflight 27 and Worker smoke 1. These are existing
offline regressions against the rebuilt Rust boundary, not new actual-ticket
or browser cases. All 17 authored live binding cases remain unexecuted.

Final artifacts (bytes and SHA-256):

| Profile | Bytes | SHA-256 |
| --- | ---: | --- |
| `retained-file-input` | 17,796,688 | `57cae4edc87fdd57f36ff77f62775c1a6645e615983454a66b7be187522677ca` |
| `retained-file-input-combined` | 19,481,488 | `e2d60a10293f8961b270e02baef7df01d3d599d0745e482826418861f7a75c85` |

Artifact size and the 4 KiB administrative lease are different measurements;
neither establishes process RSS or a reduction in global/browser memory.
No shared Chrome restart, new browser result, public upload, additional commit
or push occurred. No task-owned background service was started. The overall
Node FFI goal remains active.

### Next concrete Node integration boundary

Capture the original `NativeHandle` state weakly at existing preparation
admission, alongside source metadata; do not upgrade it, check disposal or add
a strong native pin during language phases. Carry that exact weak identity
through record/snapshot replacement and recheck fixed storage proofs. Only the
proper native boundary may upgrade it and acquire native-use authority.

Actual typed progress also needs a selected completion path: the generic
`boundary::spawn_with_env` always delivers its ticket, whereas intermediate
progress must park the same entry before foreign publication callbacks. The
current `FramePublicationFence` is move-only and cannot be extracted through
read-only `inspect_parked`. Resolve and account its completion ownership without
holding an unstarted execution token across JS waiting. Keep a weak revoker
available while a phase is running, verify the exact Node scope before host
registration, and recheck phase/session state after publication reentry.
Native marking, persistent utility-world storage, the original operation
deadline and late serialization/mutation remain separate required work; a
synthetic Marked success or the eager upload helper cannot substitute for them.

## NODE-179: Same-admission FFI handoff and raw carrier projection

This checkpoint implements two prerequisites for the real Node/native upload
handoff: revocable same-permit FFI parking/resumption and a Rust-owned raw carrier
projection kernel. It does **not** connect them to `NativeFileInputPreparation`,
select the NODE-178 precheck in the host/addon, implement native marking, or
publish `ElementHandle.setInputFiles`. That preparation still stops at
Materialized and holds `FfiPreparation<()>`; no caller-provided marking flag or
skip-to-projection transition is accepted. The full Node FFI objective remains
unchanged and incomplete.

### Keep native ownership inside the original FFI entry

Under the existing `retained-file-input` selection, a completed successful
`FfiTicket<T>` can `park` in its original budget. The result stays in the original
entry, as does its lane/base-byte permit. No second call is admitted and its
result type cannot change. Parking removes queued-result counts and releases
only the prior phase's separately reserved future-body bytes, after that future
has been destroyed. No running task token spans the parked foreign await.

The lifecycle is `Initial -> Running -> Queued -> Parked -> Running ...` before
final delivery or revocation. `start` is fresh-only; `resume` requires a parked
successful result and exact budget identity. Task registration and result
extraction occur under shutdown's registry-to-entry lock order. The caller must
immediately move both returned values into the next native continuation, reserve
its actual future body and run/drop it. The Rust tuple API is a trusted internal
boundary, not permission to keep its extracted owner after dropping execution
or to carry an unstarted execution token across JS waiting.

Pure Rust `inspect_parked` can copy phase/publication metadata without extracting
native ownership. It releases the registry guard before invoking its callback,
so callback panic cannot poison global shutdown; the entry guard remains held.
Do not call NAPI, await, reenter the entry/budget, or return native-use authority
from that callback.

`FfiRevoker` is a weak Send/Sync receipt for an exact entry, not session identity.
Revoking an initial, queued or parked entry drops its result before its permit,
outside locks. Revoking Running requests per-entry cancellation and keeps its
allowance until execution drops the actual future. Late results are discarded
outside locks while still charged. Cancellation wakeups also happen outside
locks. `ExecutionWork` has future-before-execution field order for unpolled Drop;
the actual running future has an inner lexical lifetime ending before settlement.
These rules protect bookkeeping, not independently prove browser group release.

Independent review also reproduced a pre-existing global-budget shutdown
reentry hazard: `stopped.cancel()` woke Rust callbacks while the registry lock
was held. The NODE-179 red regression fails on the lock observation without
hanging; its actual admission probe runs only when the lock is available. The
repair separates a registry-locked admission seal from lock-free cancellation
and task-tracker close. Every admission/promotion must reject the seal even
before cancellation fires. Default-build shutdown/drop tests and selected
parked-phase seal tests are required in addition to the per-entry revoker tests;
this does not establish that unrelated HostSession cancellation is reentry-safe.

Selected per-entry phase/cancellation bookkeeping is bounded by existing active
lane capacities; no new lane or declared payload budget is introduced. This is
not a new measured RSS/global allocation bound. Retained empty receipts and
caller-copied weak tombstones remain outside active-entry byte claims.

### Preserve the language values until their proper native boundary

The synchronous Rust kernel reads `filePayloads`, `localPaths`, `localDirectory`
before either multiplicity read. It then evaluates the exact short-circuit
expression, retaining raw falsy results including undefined, null, -0, NaN, 0n
and empty string. Relational comparison uses a single language `>` port, so
number-hint coercion, BigInt and thrown Symbols behave like the engine.
`coerce_to_bool` supplies native truth values without consulting the replaceable
global Boolean. No payload indexing, traversal, Buffer copy, serialization or
then assimilation occurs. The nullish error port performs literal destructuring
to preserve the engine's error type/message instead of inventing property-read
wording.

`inspectFileInputProjection` exposes this exact Rust kernel for offline checks.
Its own-data diagnostic contains the three original references, `rawMultiple`,
`multiple` and `directoryUpload`; it stores no persistent root or native owner.
This is not a production marking receipt or a new ticket stage. Eventual ticket
integration must account all retained raw references explicitly: preserving all
current client roots plus projected values needs a larger stated root bound;
hiding them in a nested carrier cannot preserve the old six-root accounting.

### Native integration contracts established from upstream

- Mark the original node after materialization and before any carrier read.
  The mark operation is in the original creation Frame's utility world, not the
  later retargeted input's world or the client ownerFrame conversion Context.
- Marker state belongs to the persistent injected utility instance, not a
  temporary per-call group. A different call ID replaces its prior Set without
  unmark events; the same ID unmarks removed nodes and marks new nodes. Mark and
  unmark CustomEvents carry call-ID detail and bubble, compose and are
  cancelable. State is assigned after dispatch. There is no automatic
  operation-end reset. Native handle limits do not bound the retained DOM graph.
- Recoverable utility/adoption errors on an attached Frame become
  `error:notconnected`; `_markAsTargetElement` ignores that return and carrier
  getters still run. Abort/timeout, session closure, detached Frame and actual
  evaluation-JS errors remain non-retriable. Do not swallow all marker errors or
  make all of them fatal before projection.
- Upstream metadata uses connection-wide request IDs to form `call@<id>`.
  Empty internal call IDs intentionally skip marking. Preparation IDs, a fresh
  per-phase ID or empty metadata cannot silently substitute for that contract.
- A retained group can keep temporary wrappers through these boundaries, but
  it cannot bypass disposal/current-world revalidation of the original Handle
  before subsequent utility calls. Bind the original native wrapper weakly at
  admission and acquire actual native authority at execution, not through an
  arbitrary JS receiver root or an added strong pin over initial foreign waits.
- The progress deadline begins before server materialization, not after
  Materialized or separately at each native phase. Progress races must follow
  actual upstream boundaries; a timer alone does not race every foreign value.

Pinned sources: `lib/server/dispatchers/dispatcher.js:87,277`,
`lib/server/dom.js:121,428,544`, `lib/server/frames.js:899`,
`lib/server/javascript.js:194`, `lib/client/connection.js:107`, and
`markTargetElements` in `lib/generated/injectedScriptSource.js` (extracted
logical lines 7680–7705), all Playwright 1.58.2.

### Required next integration, not fulfilled by generic parking

The initial admission must select one native progress/publication result type;
parked owners stay in its FFI entry, never in a Node payload across callbacks.
Publish copied phase/fence information outside locks. The current addon registry
still uses the old unit result and must be adapted, not retyped after admission.

Register bounded weak revokers with the exact HostSession **before** any parked
owner can hold a Handle route/work token. A fixed eight-slot registry matching
the existing Handle/upload call capacities is **proposed, not implemented**;
it needs its own explicit fixed-storage proof and generation-safe weak receipts.
The actual host budget route is
`crates/pwright-node-host/src/file_input/profile.rs`: `requirements()` combines
the selected `RetainedUploads::requirements()` from `handles/file_input.rs`.
The installation is `crates/pwright-node-host/src/file_input/mod.rs`; its existing
32 KiB `STATE_BYTES` allowance and `size_of::<Self>() + size_of::<Slots>()` check
do not by themselves cover a newly allocated registry's backing.

Seal and take the session's revocation entries under its admission lock, then
drop all host/registry locks before invoking revokers or destroying results.
Both explicit close and raw peer termination must revoke before
`task_operations.wait()` **and** the later Handle/`Owners::wait`, with an
idempotent teardown fallback. Node scope/environment cleanup alone does not
cover a peer closing while no Node callback resumes. The next change must also
audit the existing `lib.rs` shutdown block: `task_stop.cancel()` and
`task_operations.close()` currently run under `task_state`'s admission mutex
and can wake reentrant code. Merely moving new revoker calls outside that mutex
would not prove the whole host shutdown reentry-safe; NODE-179's FFI-budget
shutdown work does not fix this separate host boundary.

FFI budget identity is environment-wide, not proof of session/Page authority.
Page/Frame retirement, the operation deadline and public error publication must
likewise be coordinated with these parked phases. No existing service may be
shut down by trusting an unrelated or foreign receipt.

Persistent exact-world marker ownership/retirement, actual native-mark completion,
raw projection root admission, NODE-178 host selection, onBeforeInputAction,
late custom utility serialization, same-retargeted-receiver mutation and live
differential validation are still required. The old eager typed
`handle_set_input_files` path remains separate and is not a compatibility shortcut.

### Verification checkpoint

Final evidence is under `.work/node179-native-handoff/`. Cargo used release,
locked resolution, no incremental compilation and one build job throughout.

| Check | Final result | Evidence |
| --- | --- | --- |
| Standalone selected host | 609 passed, 23 ignored; includes all 61 FFI cases | `host-standalone-final.log` |
| Combined selected host | 671 passed, 25 ignored | `host-combined.log` |
| Selected addons, both profiles | 63 passed each, zero ignored | `addon-{standalone,combined}-final.log` |
| Default host FFI filter | 18 passed, zero ignored; 415 unrelated cases filtered | `ffi-default.log` |
| Strict host/addon all-targets Clippy | Default, handles+file-input, standalone and combined passed with `-D warnings` | `clippy-default.log`, `clippy-handles-file-input.log`, `clippy-standalone.log`, `clippy-combined-final.log` |
| Formatting and dependency gates | Passed; host 240/241 and addon 255/256 selected closures unchanged | `fmt-final.log`, `boundary.log`, `file-input-dependencies.log` |

The independently captured shutdown regression failed on the old locked
wakeup (`shutdown-wake-red.log`, one failed test); the relocated generic
regression and seal-gap/drop probes now pass in their applicable profiles.
There are 29 new Rust ownership/shutdown tests: twelve parking, eight initial
independent lifecycle, five selected seal-gap and four default-capable cases.
The initial all-targets Clippy pass also exposed a missing Rust crate re-export
of the diagnostic. It was fixed consistently with the existing diagnostics,
without lint suppressions, then both addon test suites and artifacts were rebuilt.

Both final artifacts pass **268 selected offline cases each**:
47 projection, 20 dispatch, 76 materialization, 61 channel, 36 arguments,
27 preflight and one Worker smoke, with zero failures, cancellations or skips.
The 47 projection cases comprise 44 paired scenarios plus three diagnostic/port
contracts. They invoke the actual Rust kernel and actual pinned upstream
`_setInputFiles`, stopped deliberately at its first native evaluation boundary.
The smoke additionally checks the actual projection export; its ten classes,
five selected session methods, six preparation methods, zero real tickets and
zero browser connections are unchanged. Logs are
`{projection,dispatch,materialization,channel,arguments,preflight,smoke}-{standalone,combined}-final.log`.
Package contracts **151/151**, types and the existing DOM-logic offline oracle
**35/35** also pass (`package-contracts-final.log`, `types-final.log`,
`dom-oracle-final.log`).

| Final darwin-arm64 artifact | Bytes on disk | SHA-256 |
| --- | ---: | --- |
| `native/retained-file-input/pwright.node` | 17,768,704 | `aa17c6c2d5e933766c390719ff6a5511fdf0b7e6942c0bf230d5e03b916980b3` |
| `native/retained-file-input-combined/pwright.node` | 19,456,512 | `040430da4a6341e6d6f3ee0268d8c4e92362c7d199062863226f195547b30b44` |

These sizes are not resident-memory measurements. Offline language checks do
not synthesize upload success, execute browser DOM, exercise an actual marking
receipt or prove ticket-root reclamation. FFI probes observe real budget
entries/destructors/wakeups, not native browser cleanup. The 17 existing
actual-ticket cases remain unexecuted; no new browser result is claimed and
the shared Chrome was not restarted. No task-owned server/browser remains.
Foundation HEAD remains `f76e3f2701a8ccde0474b5accf40dc328aaa511c`; this
checkpoint adds no commit or push, and the overall Node FFI goal remains active.

## NODE-178: Owned native file-input prevalidation

The new default-off `retained-preflight` capability separates a native DOM
precheck from payload conversion and mutation. It is implemented in the existing
file-input backend and composition crates, **not selected by the host/addon**.
This checkpoint does not add public `ElementHandle.setInputFiles`, marking,
carrier projection, Node ticket promotion, late utility serialization or upload.
The existing typed payload path is unchanged and is not an equivalent late
continuation: it rechecks/retargets and prepares payload bytes too early.

### Exact owner and one-shot native acquisition

`FileInputPreflightOperationService::prepare` consumes an already-adopted
`OwnedElement` with settlement-mode group release. The read-only
`uses_settlement_release` predicate rejects ordinary selector-owned groups
before any derived native query; it neither upgrades nor readmits those owners.
The selected backend accepts native Boolean flags, not arbitrary JS carrier
values, and executes the checked-in synchronous `retained_preflight.js` once
in the exact utility world. It follows label retargeting, checks INPUT,
multiplicity and directory policy, and retains the returned node ID in the
original object group. It does not check input type=file, visibility,
enabled/stability or DOM `isConnected`, and reads no payload.

The new explicit CDP method preserves raw result and exception RemoteObjects,
uses `returnByValue: false`, `awaitPromise: false`, an explicit object group and
caller-selected `userGesture`. Prevalidation selects `userGesture: true`, as
pinned upstream `CRExecutionContext.evaluateWithArguments` does. It uses native
settlement rather than the ordinary response deadline. This is a synchronous
precheck-specific primitive, not a general replacement for upstream asynchronous
utility evaluation. Custom clients must implement it explicitly; its default
returns unsupported rather than silently using a weaker ordinary method.

The admission is consumed when `validate` is called, even if the returned future
is never polled or fails. No reuse can create additional wrappers under one
allowance. Successful composition returns an `OwnedValidatedFileInput`: its
`object_id()` is the exact retargeted receiver; `element()` preserves the
original adopted receiver/world/group/route. A future mutation must use that
returned ID without retargeting again. This owner is neither a public Handle,
a DOM snapshot nor a navigation-resistant native-use pin.

### Admission, cancellation and cleanup

Backend capacity independently admits two extra RemoteHandle units (result and
exception) and **19,546 BackendStateBytes per admission**. The byte formula is
seven bounded native IDs plus two script copies, two 1 KiB diagnostic buffers
and 8 KiB control storage. A compile-time check bounds the actual validation
future/admission/error inline storage within that control allowance. Serialized
wire frames and decoded reply envelopes retain transport-owned bounds; this is
not an estimate of browser or arbitrary JS heap memory.

The composition separately claims its caller-selected `UploadProfile`, one
LiveObject per active operation, declared owner bytes and an independent cleanup
reservation in addition to the adopted element's existing reservation. Before
boxing it checks the actual validation and release future sizes, public wrapper
sizes and control allowances. Capacity remains occupied while a successful
owner is parked and during release, not just during the query.

- Dropping an unpolled precheck sends no precheck command; the original adopted
  owner follows its existing reserved cleanup path.
- Dropping a started query transfers the same future to its reserved executor.
  Late success or exception leads only to whole-group release, never upload.
- Dropping a ready owner or an explicit release future, before or after its
  first poll, preserves the extra allowance and route through group settlement.
- Cleanup failure closes service admission before returning backing capacity.
  Owner-first drop ordering also poisons admission when an executor discards
  an armed continuation. Such a broken executor still cannot prove native
  reclamation; do not equate returned Rust leases with browser cleanup.

The embedding application must drive its injected cleanup executor. No runtime,
worker, server or browser is launched by these leaves, and there is no new
implicit timeout. The eventual public operation needs one deadline across all
server phases, plus cancellation-safe ownership at each language/native handoff.

Backend errors retain typed CDP terminal/timeout/resource variants and protocol
codes. Fresh UTF-8-bounded strings retain at most 1 KiB total diagnostic backing;
navigation URL/reason share that allowance. Opaque JSON errors normalize through
a bounded formatter, and recursive Compound errors become a fixed Other message
without formatting their tree. Text such as "Target closed" never upgrades a
Protocol/Other error into a terminal kind or establishes group destruction.
Future host error mapping remains separate from this type-preserving boundary.

### Pinned upstream ordering that remains to integrate

The required order is materialize -> mark the original target -> project raw
carrier fields -> precheck/retain retargeted input -> await onBeforeInputAction
-> late custom utility serialization -> browser atob/File and mutation.
`_markAsTargetElement` can dispatch the observable bubbling/composed/cancelable
`__playwright_mark_target__` event with call-ID detail and alter the DOM before
carrier reads. It is not safely replaceable by an unconditional no-op.

Projection reads all three carrier fields before multiplicity. The upstream
`multiple` expression can preserve raw falsy values; these Rust Boolean flags
only represent its later native truth values. Detached/disabled INPUTs are not
rejected by this precheck. A text INPUT still reaches late serialization before
the injected upload helper's no-op, so eager type=file rejection is not parity.
The custom utility serializer is not JSON.stringify: object getter failures,
array access, cycles and special values have distinct behavior. None of these
language boundaries is implemented by this native precheck leaf.

Source anchors under pinned Playwright 1.58.2: `lib/server/dom.js:428,544`,
`lib/server/javascript.js:185`,
`lib/server/chromium/crExecutionContext.js:66`, and
`lib/utils/isomorphic/utilityScriptSerializers.js:147`.

Before Node selection, design the parked owner and cancellation handshake across
the instrumentation/late serializer await. Do not hold an unstarted FFI execution
token indefinitely, release/readmit the original permit, or change an already
admitted result type. Public error transport, terminal native error mapping and
browser activation/timing remain separate conformance work.

### Verification checkpoint

Completed offline evidence is under `.work/node178-prevalidation/`. The new DOM
oracle passes **35 offline cases**: 24 pinned upstream prototype/serialization
cases and 11 comparisons of the actual candidate script against the captured
upstream callback and generated retarget method on plain descriptors. These
are not DOM/browser tests. Successful prechecks deliberately stop at the late
utility boundary; they never synthesize upload success. Native lifecycle tests
likewise use controlled CDP fakes/transport peers, not a live Chrome result.

All Rust commands use release, locked dependencies, `CARGO_INCREMENTAL=0` and
`CARGO_BUILD_JOBS=1`. Completed results:

| Check | Result | Evidence log |
| --- | --- | --- |
| Preflight backend/composition `--lib` | 34 / 32 passed, including 18 / 15 new cases | `preflight-tests-final.log` |
| Same leaves with large commands | 34 / 32 passed | `preflight-tests-large.log` |
| Unselected/default backend/composition | 16 / 17 passed | `default-leaf-tests.log` |
| CDP default and Fake `--lib` after userGesture change | 120 / 23 passed | `cdp-fake-gesture.log` |
| CDP large commands | 129 passed | `cdp-large-gesture.log` |
| Fake grouped-resolution integration | 5 passed | `fake-grouped-gesture.log` |
| Retained-resolution backend | 49 passed | `resolution-retained-final.log` |
| Pinned DOM/serialization oracle | 35 passed | `dom-oracle.log` |
| Package contracts / TypeScript | 151 passed / exit 0 | `package-contracts.log`, `types.log` |

The four new CDP tests exercise actual serialization/decoder and pending-command
ownership through delayed replies, Promise non-assimilation flags, invalid input
and transport termination. They are not browser execution. Two initial resource
assertions were corrected: a refused reservation increments the rejection count,
and ordinary selector resolution legitimately has a nonzero historical selector
peak. Final tests preserve exact current lease/resource invariants; the initial
failure logs remain available as `preflight-tests.log` and `resolution-retained.log`.

Strict all-target Clippy with `-D warnings` passes for default leaves,
preflight leaves, preflight plus large commands, and CDP/Fake plus large commands
(`clippy-{default-leaves,preflight,preflight-large,cdp-fake-final}.log`).
Combined retained-file-input host/addon all-target `cargo check` passes, while
still excluding preflight (`check-host-addon-unselected-preflight.log`).
Feature-fingerprint gates verify exactly one backend flag or four composition/
backend/adoption flags, no new normal packages, and no implicit host/addon
selection. Existing selected graphs remain **240/241 host** and **255/256 addon**
packages (`file-input-dependencies.log`, `boundary.log`). Formatting, JavaScript
syntax and whitespace checks pass (`fmt-final.log`, `whitespace-final.log`).

No Node artifact was rebuilt for this standalone leaf. Both sizes/SHA-256 hashes
were rechecked and exactly match the NODE-177 fingerprints below; they do not
include this new preflight implementation. **17 authored actual-ticket cases**
and public retained-upload/browser gates remain unexecuted. No new live browser
pass is claimed and no shared browser/service was restarted. There is no new
task-owned background runtime. Memory checkpoints remained stable with zero
swap; shared app/backend RSS is not exact per-chat RAM. The overall Node FFI
goal stays active; no additional commit or push was made.

## NODE-177: Deferred registration gates before materialization

The selected internal preparation now reaches raw Materialized through the
production scheduling helper, with Rust-owned registration and phase checks.
This is still **not** public `ElementHandle.setInputFiles`, a native byte owner,
or browser mutation. The helper resolves `undefined`; the caller retains its
original preparation and must abort it after this internal stop point. Returning
the ticket through an async return would assimilate a user-controlled `then`
outside cleanup protection, so the driver never returns or awaits the ticket.

### Registration is distinct from execution-context validity

The safe host checks current exact `Owners` provenance, Element kind and live
owner-ticket membership, then asks the runtime only whether the immutable
creation **Page** exists. Explicit disposal/abandonment removes the owner ticket;
session close/disconnect or missing Page rejects with structured TargetClosed.
A foreign session is not authorized by equal numeric object IDs. A missing
creation-Page proof fails explicitly rather than falling back to another Page.

In the actual HostSession composition, all Handle producers share one private
`AtomicObjectIdSource`; it only increments with checked exhaustion and never
reclaims IDs. Failed scalar work and trimmed/abandoned batches still consume
their allocated IDs. Exact owner installation plus live ID membership therefore
cannot accept a replacement Handle with a recycled key in that session. This
is a property of the concrete host allocator, not the generic `ObjectIdSource`
trait. See `pwright-core/src/object_id.rs` and the composition's
`handle_leaf_services` wiring.

Do not ask whether the Handle or Frame runtime object, execution world or native
receiver is still live here. Pinned upstream Handle dispatchers survive
navigation and Frame detach, although their execution contexts become invalid.
Those failures belong to later native execution. Checking them before schema
validation would change the winning error and observable getter order. The
Page check is a point-in-time gate, not a lifetime pin; complete Page-close event
timing versus the upstream dispatcher remains a separate live verification gate.

The addon checks the exact preparation scope **before** acquiring its exclusive
phase. A foreign raw-session check therefore cannot abort the rightful ticket.
The selected phase transitions are:

`ChannelReady -> ServerDispatchReady -> ServerParamsReady -> ServerMaterializationReady -> WaitingMaterialization -> Materialized`.

The two Server*Ready registration transitions are performed only by
`handleCheckFileInputDispatch`; `validateServerParams` and
`beginMaterialization` require their respective preceding gate. This prevents
manual skipping of either check, but does not pretend that the Rust phase enum
enforces an event-loop turn. The thin JS driver supplies that scheduling.

### Deferred driver and cleanup

`continueAfterChannel` schedules delivery directly in the client-validation
continuation, without first awaiting a receipt or validated params.
`prepareServerPayloads` uses captured Node `setImmediate`, performs the first
registration check, validates server Params, awaits the empty-instrumentation
checkpoint, performs the second check, and only then invokes raw materialization
and captures its awaited outer carrier. Params and nested helper values remain
unawaited. This preserves second-schema microtasks before Buffer serialization.

The await models the no-listener `onBeforeCall` boundary only. It is not an
instrumentation API, metadata validator, dispatcher error transport, public API
stack rewriting, progress deadline or native upload. Driver failure aborts the
supplied preparation, including a caller supplying the wrong session. The
non-destructive foreign-scope guarantee applies to the raw addon gate, not to
the driver that owns failure cleanup for its supplied ticket.

Both gates clone only the existing Rust record/Rc root container. They create
no N-API reference, native-use lease, operation, worker or execution token. No
registry borrow spans host calls or language operations; old records drop
outside registry borrows. The six-root, 32-entry, 16-scope, 512 KiB synchronous
phase and 784 KiB timeout-metadata limits are unchanged. Arbitrary V8 graphs,
callback allocations and serialized strings remain outside administrative Rust
byte claims. No new feature, package, profile or raw class is introduced.

### Native continuation must preserve late conversion

The pinned server source rules out eagerly projecting the raw carrier into
the existing `PreparedFileInput`. After helper materialization, upstream marks
the target, reads `filePayloads`, `localPaths`, `localDirectory` and multiplicity,
then performs utility-world retargeting/INPUT/multiple/directory checks. Only
after the `onBeforeInputAction` await does its custom utility argument serializer
visit the raw payload values. Browser-side injected code subsequently performs
`atob` and `File` coercion. Earlier DOM failures must win over later raw-payload
getters, malformed base64 and filename/MIME coercion.

The current typed Rust payload backend normalizes strings/bytes beforehand and
combines DOM validation with mutation. It cannot faithfully consume arbitrary
raw materialization results at this boundary. Next work must split owned native
DOM validation from late bounded utility-value serialization/mutation, retaining
the exact retargeted owner and cleanup across the handoff. Do not substitute
`JSON.stringify`, permissive `Buffer.from(base64)`, eager Rust decoding or
premature string conversion. Do not hold an unstarted FFI execution token across
an arbitrary JS wait: that would delay shutdown. Any later execution promotion
must preserve the original admitted permit and a result type chosen at admission,
not release/readmit or retype the existing preparation.

Source anchors: pinned Playwright 1.58.2
`lib/server/dispatchers/dispatcher.js:252`, `lib/server/dom.js:544`,
`lib/server/javascript.js:185`, and
`lib/utils/isomorphic/utilityScriptSerializers.js:147` under
`tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/`.
The materialization kernel is not a substitute for that complete native sequence.

### Verification status

Completed offline evidence is under `.work/node177-dispatch/`. Both selected
release artifacts were rebuilt with `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`.
Each passes the following native-kernel/driver-adapter lanes:

| Lane | Passed per artifact |
| --- | ---: |
| New deferred dispatch | 20 |
| Materialization/server schema | 76 |
| Client channel | 61 |
| Argument assembly | 36 |
| Preflight | 27 |
| Worker publication/teardown smoke | 1 |
| Total | 221 |

The 20 new cases distinguish nine real pinned dispatcher metadata cases from
eleven candidate driver adapters using actual Rust diagnostics. The upstream
fixture always throws at command entry: it never reports synthetic upload
success. Candidate adapter cleanup counters are not actual native-ticket/root
proofs. The two hostile ticket-`then` cases confirm no getter/callback invocation
and retain the Materialized adapter until explicit abort. The Worker creates no
ticket or browser connection; it checks ten raw classes, five selected session
methods and six selected preparation methods, then real environment teardown.

Additional gates:

- Host unit tests pass **580 / 642** for standalone / combined, with **23 / 25
  ignored** respectively. Both include the 20 source-identity cases, of which
  13 are new exact-ticket and injected creation-Page reader checks. These prove
  host predicate/resource behavior, not browser Page-close timing.
- Explicit addon `cargo test --release --locked --lib` runs pass **63 per
  profile**, rather than relying on the crate's default `test = false` target.
- Package contracts **151**, pinned ingress oracle **51**, TypeScript and
  syntax checks pass. The actual-ticket binding file is syntax-checked only.
- Strict Clippy passes for standalone, combined, default and the older
  `handles,file-input` selection, for both host and addon with all targets.
- Dependency checks retain **240/241 host** and **255/256 addon** packages.
  Runtime JS still imports no browser driver, CDP transport or process launcher.
  Workspace formatting and whitespace checks pass.

Final artifacts (paths relative to `packages/pwright`; bytes; SHA-256):

- `native/retained-file-input/pwright.node`: **17,735,408**;
  `2d12c320b5edb165740fe65b372b0ec71f0f976385eb760e7c0444a658aab3ac`.
- `native/retained-file-input-combined/pwright.node`: **19,406,640**;
  `1cd32bd8db4d0efc8a783c407434346658d50ba8aaf513b1bfa62309f0fcca11`.

Logs: `build-{standalone,combined}.log`, `host-{standalone,combined}.log`,
`addon-lib-tests-{standalone,combined}.log`,
`{dispatch,materialization,channel,arguments,preflight,smoke}-{standalone,combined}.log`,
`clippy-{standalone,combined,default,old-selection}.log`, `package-contracts.log`,
`types.log`, `oracle.log`, `boundary.log` and `fmt-final.log`.

The binding lane now contains **17 authored, unexecuted actual-ticket cases**.
Its five new cases cover driver Materialized/abort, serializer failure cleanup,
disposed-source client conversion before dispatch rejection, raw sibling-session
isolation, and second-schema microtask disposal before serialization. Existing
manual sequences now include both registration gates. Their source review
confirms that `handleDispose` unregisters ownership synchronously before returning
its completion Promise; this is not a substitute for executing the live cases.

The read-only CDP probe at **2026-09-17T12:24:56Z** still timed out after three
seconds, receiving zero bytes (curl exit 28, HTTP 000). This establishes endpoint
unavailability, not its cause or that a restart would fix it. Shared Chrome was
not restarted without permission. No task-owned server/browser was started and
no native upload or new browser pass is claimed. The overall Node FFI goal
remains active; public upload and the late native continuation remain incomplete.

## NODE-176: Server-direction validation and raw payload materialization

The selected addon now separates a second, server-direction schema pass from
the raw in-memory payload helper. These are language kernels, not a dispatcher
or complete upload implementation. Rust selects schema/order, supported input,
MIME policy and output fields; frozen JavaScript ports perform individual
language operations. No additional dependency, feature, profile or opaque class
is introduced. Public `ElementHandle.setInputFiles` remains gated.

The server validator preserves raw Buffers and arbitrary map/valueOf results.
Its channel branch follows wire GUID checks rather than client `_object`
branding: read `guid` for the string check, reread it for the registry key, then
perform template interpolation for a missing-owner diagnostic. There is no
WritableStream dispatcher registry in this capability; even a well-shaped GUID
cannot grant authority. The existing client branch is unchanged. Both directions
use static zero-capture native callback specializations.

### Materialization boundary

The payload helper reads `payloads`, `streams`, `directoryStream`, `localPaths`,
then `localDirectory`. Exactly-one truthy selection precedes supported-route
selection. Paths/directories/streams fail explicitly as unimplemented here;
they do not trigger file I/O or claim native stream ownership.

An observable payload map reads each `name`, then `mimeType`; falsy MIME rereads
`name` and performs pinned MIME coercion/lookup. It then calls the original
buffer's `toString("base64")` with its original receiver and reads
`lastModifiedMs`. Serializer results remain raw: strings, undefined, objects
and nested Promises are not eagerly decoded or awaited. All ordinary payload
map callbacks complete before microtasks queued by an earlier serializer.
Later mutation cannot retroactively change an earlier returned base64 string.

Payloads and the final carrier use own writable/enumerable/configurable data
properties, including undefined values. They bypass inherited setters, unlike
schema output assignment. `asyncMaterialize` supplies a real JavaScript async
return: the final carrier's inherited `then` is observable and can replace the
carrier; nested serializer/map thenables are not assimilated. Error identity
is local to this helper, before server serialization and public API wrapping.

MIME data stays in the existing pinned Rust leaf. Exact extension lookup is
distinct from filename lookup, so overridden normalization results are not
normalized again. The table derives its maximum extension byte length for
fixed Rust scratch; it does not require copying the whole filename into Rust.
Language-created strings, serialized base64 and arbitrary callback graphs are
V8-owned and are not bounded by administrative Rust admission.
The longest pinned extension is 12 bytes, so the native copy uses a 13-byte
stack array including its terminator. JavaScript retains the RegExp constructor
and Reflect.ownKeys intrinsic at module initialization; pre-import primordial
tampering and mutation of the pinned MIME dictionary are outside this boundary.
String/replace/lowercase callbacks and their raw returns remain observable.

### Ticket phases and authority

`validateServerParams` moves ChannelReady to ServerParamsReady, replacing the
sixth root. `beginMaterialization` returns the raw carrier and moves to
WaitingMaterialization while retaining validated params. Its phase guard ends
before the JS await. `finishMaterialization` moves the saved params to root
five and the awaited carrier to root six, then marks Materialized. All phases
recheck revocation before replacement-root allocation; failures/abort use the
existing out-of-borrow destruction path. The limit stays six roots per ticket,
32 entries, 16 scopes and 512 KiB independently admitted synchronous scratch.
Timeout metadata remains separately admitted at 784 KiB. No FFI execution is
promoted and no native-use pin is acquired by these methods.

These internal steps may be invoked manually for testing, but are deliberately
not wired into an automatic public dispatcher. Upstream first checks dispatcher
existence after `setImmediate`, validates parameters and metadata, awaits
instrumentation, rechecks identity, and runs a progress-controlled command.
Those target/metadata/instrumentation/deadline/error-transport responsibilities
are not replaced by the kernels. A source descriptor is still not native-use
authority. Typed native conversion, safe decoding/storage of raw results,
same-permit execution promotion and final submission remain separate work.
The inaccessible trusted server Frame/collocation read is not reproduced by
this payload-only helper; no stand-in Frame is treated as target authority.

### Verification status

Final evidence is recorded under `.work/node176-materialization/`:

- Both rebuilt selected artifacts pass **76 new offline cases each**. Cases
  distinguish direct helper, server schema, deferred two-validator seam and
  four candidate-only unsupported-route contracts. They include 512 KiB names,
  repeated MIME coercion and exact-key lookup, custom serializers/raw returns,
  own-property construction, outer assimilation and escaped callbacks.
- Both artifacts also pass **61 channel**, **36 argument**, **27 preflight**
  cases and **one Worker smoke** each. The Worker checks ten opaque classes,
  four session methods and six preparation methods; it creates zero actual
  tickets/connections. No new actual-ticket execution is inferred from it.
- Explicit `cargo test --release --locked --lib` addon runs pass **63 tests per
  profile**. The earlier logs without `--lib` contain compile completion only
  (`test = false`), not executed tests. Strict release/locked Clippy passes
  standalone, combined, default, older `handles,file-input`, and the MIME leaf.
- The MIME leaf passes **21** release/locked tests. Its existing full-name
  oracle verifies **1,173** table entries and **9,393** filename cases.
- Package contracts pass **151**, upstream ingress oracle **51**, and type
  checks pass. Dependency checks retain **240/241 host** and **255/256 addon**
  packages with no JS browser driver/CDP/process-launcher imports. Workspace
  formatting and whitespace checks pass after fixing initial formatting only.

Final artifacts (paths relative to `packages/pwright`; bytes; SHA-256):

- `native/retained-file-input/pwright.node`: **17,739,664**;
  `72ec6025f16cb751f96fee876a6a2772dde8b1ef3211346e98c665a63f27e604`.
- `native/retained-file-input-combined/pwright.node`: **19,380,176**;
  `f55429c6aca8064d6ae5ad8d7ac18ea5d610d43622e1819c0af47188e792aef9`.

Builds use `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`. Logs include
`build-{standalone,combined}.log`,
`{materialization,channel,arguments,preflight,smoke}-{standalone,combined}.log`,
`addon-lib-tests-{standalone,combined}.log`, `clippy-*.log`, `mime-tests.log`,
`mime-oracle.log`, `package-contracts.log`, `types.log`, `oracle.log`,
`boundary.log` and `fmt-final.log`. Host runtime logic did not change; its MIME
re-export does not imply a fresh host-test run.

The new offline lane distinguishes direct helper inputs from a scheduled
client-schema → setImmediate → server-schema → helper comparison. It uses the
actual pinned Playwright 1.58.2 validator/helper, without a browser, real ticket,
synthetic upload success, dispatcher target admission or error transport.
The live binding lane now has **12 authored, unexecuted cases**: four new cases
cover raw Materialized/abort, serializer error, callback revocation and abort
between begin/finish. Shared Chrome has not been restarted without permission.
The latest read-only CDP probe at **2026-09-17T11:53:30Z** still timed out after
three seconds without a response. No new browser pass, server process or native
upload is claimed. The overall Node FFI compatibility goal remains active.

## NODE-175: Rust-owned raw-buffer channel validation

The selected retained-upload preparation gains a synchronous channel-validation
phase after argument assembly. Rust owns the pinned schema, traversal, optional
fields, type decisions and diagnostics. Frozen JavaScript ports perform only
language operations: property read/assignment, intrinsic predicates, valueOf,
map, addition, callback adaptation and ValidationError construction. This is the
normal in-process **raw Buffer** path with `isUnderTest` false, not a wire/base64
transport, public upload API or native mutation.

### Schema and observable behavior

Validation order is `payloads` (each item's `name`, optional `mimeType`, then
`buffer`), `localDirectory`, `directoryStream`, `localPaths`, `streams`, then
required `timeout`. Every field except timeout is optional. Unknown fields,
including `noWaitAfter`, are omitted here after their earlier spread/getter
effects. The validator does not impose exactly-one-input, native string/array
limits, nonnegative/integer timeout rules, or server file checks: those are
distinct subsequent phases.

Boxed String/Number values invoke their observable `valueOf` and preserve its
raw return, including objects or undefined. Undefined validated fields are
omitted even when their original schema entry is required. Buffer validation
uses `instanceof Buffer` and retains the exact object without calling its
serializer or copying bytes. Payload/path arrays use their observable `map`,
including sparse/species behavior and arbitrary overridden return values.
Results remain raw language values, not eager Rust DTOs.

The result object uses ordinary strict property assignment, including inherited
setters; this is intentionally different from the private root container's own
data-property definitions. A fresh nonconstructible two-argument JS map callback
forwards each invocation's value, index, path and ports to one of three native
callback specializations with static typed dispatch and no captures. A generic
compile-time zero-size assertion rejects future captured closures or function
pointers. This avoids a new Rust callback-box allocation, including leaks on the
pinned napi helper's fallible creation/finalizer paths. No scoped Env/Unknown,
preparation receipt, session or phase guard escapes. Index path concatenation uses
the real JS addition operation rather than early index coercion or a u32 cast.
Escaped callbacks remain language validators, not native-use authority.

This local composition has no WritableStream producer or channel-owner brand.
A present directoryStream/stream element performs its first observable `_object`
read and then fails closed as an unrecognized channel. It never trusts a
`_type`/`_guid` shape, borrows upstream Playwright objects as authority, or claims
valid-stream parity. Real branded stream support remains an explicit missing
capability; future integration must preserve repeated property-read ordering
and separately enforce exact native provenance before submission.

### Integration and ownership

`NativeFileInputPreparation.validateChannel(ports)` transitions
ArgumentsReady to ChannelReady by replacing the sixth root with validated
params. A synchronous phase guard protects the original snapshot; no registry
borrow spans getters, map callbacks, valueOf or output setters. The entry is
rechecked before replacement-root construction and commit, so callback revocation
cannot publish new persistent roots. No task, browser work, Buffer pointer or
async native call is introduced.

The existing internal `preflight` still stops at ArgumentsReady. The new
`prepareChannel` stop point calls validation in the same continuation directly
after assembly, with no added await or Promise assimilation of validated params.
Both abort on failure. These helpers are not the public ElementHandle method:
they do not yet implement ChannelOwner API-zone attribution, error.message/stack
mutation, instrumentation, server scheduling or final submission semantics.

The six-root/192-total-root, 32-entry preparation, 16-scope registry, independent
32-phase/512 KiB synchronous scratch and existing FFI limits do not increase.
Timeout metadata remains separately admitted at 784 KiB. These administrative
limits do not bound arbitrary V8 graphs, user-retained escaped callbacks or their
callback-owned backing. No feature, profile, dependency or opaque class is added.

### Verification and remaining gates

Final verification is recorded under `.work/node175-channel/`:

- Both final zero-capture native artifacts rebuilt successfully. Each passes
  **61 new offline channel cases**, including three explicit stateless driver
  adapters exercising assembly-to-validation continuation, unknown `then`
  handling and channel getter exception/abort behavior. Direct cases run the
  actual selected Rust diagnostic against the actual pinned Playwright 1.58.2
  raw-buffer validator, not a rewritten oracle.
- Both profiles also pass **36 argument cases**, **27 preflight cases** and
  **one offline Worker test**. The Worker verifies ten opaque classes, four
  selected session methods and three preparation methods including
  validateChannel, but creates **zero real tickets/browser connections**.
- Release/locked addon unit tests pass **63 cases per profile**. Strict Clippy
  passes for standalone/combined selections, default and older
  `handles,file-input`. Workspace formatting passes. The initial diagnostic
  re-export omission was repaired without suppressing dead-code diagnostics;
  final logs supersede that intermediate failure.
- Package contracts pass **151 cases**; TypeScript checks pass. The
  upstream-only offline ingress oracle passes **51 cases**. Dependency/native
  boundary checks retain **240/241 host packages**, **255/256 addon packages**,
  and no JS browser-driver/CDP/process-launcher imports. No host implementation
  changed in this checkpoint; NODE-174 host tests are historical, not a fresh run.

Final artifacts (paths relative to `packages/pwright`; bytes; SHA-256):

- `native/retained-file-input/pwright.node`: **17,670,736**;
  `1cc2526dbfbdb81875d9ffd785ea4f00021d51d3faccc1d9ed9f57c24d05ea5d`.
- `native/retained-file-input-combined/pwright.node`: **19,333,824**;
  `c4e7af38b53e3c04b587e87906ca7972133a0c18455c78fa498ab79559379371`.

Native builds use `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`, with release/locked
Cargo checks. Evidence includes `addon-tests-{standalone,combined}-zst.log`,
`clippy-{standalone,combined}-zst.log`,
`clippy-{default,old-selection}-final.log`,
`build-{standalone,combined}-final.log`,
`channel-{standalone,combined}-final.log`,
`{arguments,preflight,smoke}-{standalone,combined}.log`,
`package-contracts.log`, `types.log`, `boundary.log`, `oracle.log` and
`fmt-workspace-final.log`. The earlier initial channel pass is not the final
post-resource-fix artifact evidence.

These are raw-kernel and labelled driver-seam comparisons, not actual-ticket
authority, public error wrapping, server materialization or browser behavior.
They do not prove arbitrary replacement of upstream's internal schema
`Object.entries` iteration or test-hook copying; Rust keeps its schema fixed
and normal non-isUnderTest mode remains explicit.

The live binding lane now has eight authored cases: the previous six plus an
actual-ticket ChannelReady/abort case and a channel-getter rejection/root-drain
case. They retain bounded/poison-on-timeout cleanup and are syntax-checked only,
**not executed**. Adding these cases does not close the native-ticket/browser
gate. The Worker additionally checks actual validateChannel registration and
forged-receiver rejection while still creating zero real tickets/connections.

Open work includes public default setters/arbitrary JS default state, valid
stream capability, public channel error/stack phases, deferred per-item serializer
and Buffer handling, two-phase paths, fractional/nonfinite native deadline mapping,
correctly typed consuming FFI preparation promotion, retained native source
authority, public ElementHandle.setInputFiles, and live/differential browser gates.
The current registry's FfiPreparation<()> and immutable source receipt cannot be
silently reused as a submitted native operation or replaced by a second FFI slot.
Chrome remains unavailable; no restart, new server or browser is authorized by
this checkpoint. Earlier checkpoints below retain their historical scope.

## NODE-174: canonical creation-Page timeout binding

Selected production Handle creation now acquires its timeout policy inside
Rust, using the Frame wrapper's creation Page ID. The session issues a canonical
Page cell with a live canonical Context parent from the authoritative published
membership. Repeated factory calls for one exact session reuse the same index;
conflicting debug configuration is rejected. Current DOM ownerFrame is never
used to select this client policy. Unscoped diagnostic cells cannot substitute
for session-issued bindings.

### Capture, inheritance and failure order

`frameEvaluateHandleWithPageTimeout` captures the lookup result at FFI admission
before argument callbacks or an await. A failed lookup remains error data:
normal source/argument conversion and deferred binary materialization still run
before that error is returned, without starting host execution. JS performs its
usual function-to-string conversion before entering this native method. Both
ordinary and Handle-containing evaluation paths preserve this order. Older
profiles keep the original four-argument, unbound native producer.

Native publication compares the binding's exact weak Handle-owner identity and
Page ID with the result's frozen creation-Page proof. Missing proof, wrong session
or wrong Page rejects publication and requests disposal of the owned result.
The source proof comes from the captured creation-world snapshot, not a later
topology query. It works for generic JSHandles as well as ElementHandles and
survives native close/disposal without granting native use.

The immutable policy lives beside, not inside, the Handle's Live/Disposed state.
Rust propagates it through evaluateHandle, getProperty and every getProperties
child, including generic-to-Element transitions. Batch failure retains existing
partial-publication rollback. There is no public rebind operation. Preparation
rejects unbound Handles and copies the bound live settings before creating
input roots. Every subsequent stage retains that metadata; production
`assembleArguments(ports)` no longer accepts late caller-selected settings.

Context/Page/Frame wrapper constructors perform no timeout lookup. Lifecycle
creation factories run before membership commits, so eager constructor lookup
would reject a valid new Page. The native producer acquires metadata only when
Handle creation is requested. There is no JS policy cache or second default
configuration store. This also avoids an early JS lookup error skipping user
conversion on an already-closed Page.

### Admission, debug configuration and teardown

Cells and independently issued scoped bindings share **1,024 slots at 512 bytes**.
Every fresh lookup reserves its binding before returning another raw wrapper or
committing new cells, including repeated lookup of an existing canonical Page.
Clones passed into already-admitted Handle/FFI storage share that binding lease.
An additional ledger admits at most **16 session indexes, 16 KiB each**, each
with fixed sixteen-Context/sixteen-Page arrays. Together with the existing
**16 KiB administration**, the declared metadata allowance is **784 KiB**, with
1,041 leases across two ledgers/scopes. The original preparation/synchronous
scratch/FFI limits and ten-class selected inventory do not increase.

Lookup prunes retired index entries; held settings retain their live parent.
Session close discards index-owned cells; returned policy bindings retain no
strong HostSession/index, transport, native-use pin or task. A held index clone
keeps its index lease until final drop. Cells/bindings keep shared administration
admitted through their last lease. These are administrative bounds, not a bound
on V8 graphs, retained JS tombstones, weak native backing or process RSS.

The JS import captures a frozen tuple of raw PWDEBUG, npm_config_pwdebug and
npm_package_config_pwdebug values. Rust selects the first defined value, including
an empty string; empty, `0`, `false` and `console` disable inspector timeout
behavior. Other nonempty strings enable it. Initialization occurs once per
selected session before Browser publication, with disconnect compensation if
setup fails. No environment reads occur during timeout resolution.

Scope close seals policy lookup before deleting Node references. Environment
cleanup first seals **all** preparation scopes, then retires publication scopes
and policy indexes, then drops remaining preparation roots. Reentry cannot use
another still-open policy slot during that drain. Existing metadata holders
remain client-only objects; public cancellation/getter precedence is not thereby
proved.

### Verification and remaining gates

Final verification is recorded under `.work/node174-bindings/`:

- Release/locked selected host tests: **567 passed, 23 ignored**. This includes
  twenty new canonical-policy tests; scalar/batch publication tests additionally
  assert the captured creation Page. The ignored browser tests remain unrun.
- Release/locked addon tests: **63 passed** for each standalone and combined
  retained-file-input selection. Strict Clippy passes for the selected host and
  standalone/combined addons, plus the default addon and older `handles,file-input`
  selection. Final logs supersede the repaired intermediate test-module ordering
  and feature-gated N-API registration failures.
- Both selected native artifacts rebuilt successfully. Each passes **36 argument
  comparisons**, **27 preflight comparisons** and **one offline Worker test**.
  The Worker checks ten opaque classes and registration/forged-receiver rejection
  for four selected session methods. It creates two unscoped timeout metadata
  objects, but **zero actual preparation tickets or browser connections**.
- Package contracts: **151 passed**, including seventeen new timeout-facade
  contracts. Type checks and dependency/native boundary checks pass. Selected
  normal dependency closures remain **240/241 host packages** and **255/256 addon
  packages** for standalone/combined; JS still imports no browser driver, CDP
  transport or process launcher. The upstream-only offline oracle passes **51**
  cases, not candidate/native browser conformance. Workspace formatting and
  whitespace checks also pass.

Final artifact fingerprints (bytes; SHA-256):

- `native/retained-file-input/pwright.node`: **17,654,832**;
  `69486d4803558639b9265c7bd30591d12ad1e1b794e60d6673ee69812bab9826`.
- `native/retained-file-input-combined/pwright.node`: **19,291,248**;
  `8a48e65fb790bb945f23645333628272e7ce287171e629f7eefd3dde1fd3aae5`.

Artifact paths are relative to `packages/pwright`. Builds use
`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`; native tests use release/locked mode.
See `host-tests.log`, `addon-tests-final.log`, `addon-tests-combined.log`,
`clippy-*-final.log`, `clippy-host.log`, `clippy-default.log`,
`clippy-combined.log`, `{build,arguments,preflight,smoke}-{standalone,combined}.log`,
`package-contracts-final.log`, `types-final.log`, `boundary-final.log` and
`oracle.log` in the evidence directory.

The metadata unit fixtures exercise exact-origin/Page proof, canonical identity,
live inheritance, capacity/churn, close and lease lifetime. They do not execute
an assembled HostSession membership lookup against a browser. Offline facade
tests execute the real facade with a fake addon loader, proving forwarding and
pre-commit wrapper behavior, not native authority or browser conformance.

The new six-case `handle-file-input-binding.test.mjs` live gate is authored and
syntax-checked, **not executed**. It is intended to cover actual source policy,
current-owner override, generic-to-Element descendants and property aliases,
disposed/closed-Page metadata, wrong-Page publication compensation, and rejection
of a same-environment sibling session's source. Actual tickets must reach
ArgumentsReady with six rooted values, then abort back to the captured native-host,
FFI and preparation baseline. Those metrics exclude the separate timeout-policy
ledgers; retained metadata is not thereby proved to consume zero bytes. The owner
override case changes the Page default after Handle publication to exercise live
canonical sharing, not just an initial value snapshot.

The lane retains wrappers through cleanup. Each case has a 20-second setup/exercise
bound, followed by four aggregate cleanup phases capped at three seconds each:
abort/disposal, owned Context retirement, all-connection retirement, and metrics.
Every phase is attempted even if a previous phase fails. Retirement guards prevent
late exercise continuations from starting more work; late acquired owners are
retired explicitly with separately tracked three-second attempts. An exercise
deadline immediately blocks subsequent fixtures, even if currently known-owner
metrics become zero: late acquisition cleanup is then best effort, not a complete
reclamation proof. A cleanup failure also prevents subsequent fixtures from opening
connections. The outer case limit is 35 seconds, diagnostic observations have a
10-second limit, and the process supervisor remains a 180-second last resort.
These safeguards have been reviewed and syntax-checked, not exercised against
a browser. The gate does not upload files.
After the dedicated Chrome endpoint is recovered and verified, run from
`packages/pwright` for each already-built selected profile:

```sh
PWRIGHT_NATIVE_PROFILE=retained-file-input PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:handle-file-input-binding
PWRIGHT_NATIVE_PROFILE=retained-file-input-combined PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:handle-file-input-binding
```

The script injects `PWDEBUG=0` before imports to make default traversal observable.
This is a future actual-ticket integration gate, not an additional offline pass.

The shared Chrome endpoint still times out; no restart or new browser/server was
performed. Actual selected N-API producer/descendant/closed-Page ordering,
publication-mismatch cleanup, preparation-ticket integration and adopted-node
browser cases remain unverified. Public default setters still need arbitrary
JavaScript default-state semantics (upstream stores values without numeric
validation). Public ElementHandle.setInputFiles, channel field/error phases,
deferred Buffer serialization, paths, native deadline mapping, promotion/submission
and the full discrepancy corpus remain open. No feature/profile/dependency is
added and no whole-API compatibility pass is claimed.

## NODE-173: Rust-owned argument assembly and live timeout settings

The selected preparation now continues immediately after conversion assimilation
into Rust-owned argument construction. Rust copies converted fields, then
options, then computes and defines the own `timeout` field. The two JavaScript
ports are only the native language spread operation and `options.timeout` member
access. Own keys, symbols, descriptors, getters, primitive/null spreading and
options overriding converted payloads remain observable; inherited setters are
not invoked. The intermediate spread objects contain own data properties, so
their reuse does not re-read the original source getters. No additional await,
channel validation, Buffer copying, path work or browser command is inserted.

### Rust client timeout policy

An explicitly selected metadata component stores a Context/root cell and Page
children with live numeric defaults. Clones retain identity rather than taking
a value snapshot. Rust applies Frame's falsy-options normalization to `{}`,
reads the original options' timeout and, if its JS type is Number, reads it
again and returns that second raw language value. A getter may change the value
or its type between reads; there is no early number conversion or u32 cast.
Otherwise Rust checks the injected cached debug classification, then the live
local default, then repeats the option/debug/default sequence at the parent,
finally selecting upstream's 30,000 ms default. No settings lock spans user
JavaScript, so an options getter can update the default used in this resolution.
Zero, signed zero, fractions, negative values, NaN and infinities are preserved
at this boundary; this is not acceptance by a native deadline implementation.

The internal `NativeFileInputTimeoutSettings` class is produced by the selected
NativeEnvironment factory and has no public constructor. It supports a Page
child and typed numeric set/unset operations, holds only Rust metadata, and uses
failure-safe construction/tag/wrap publication. The selected class inventory is
now ten; older profiles stay unchanged. This is not public Page/Context
`setDefaultTimeout`, arbitrary-JavaScript-value setter compatibility, mutable
debug-mode policy or support for arbitrary monkeypatched parent graphs. The
normal Context -> Page shape is immutable and limited to two levels.

### Ownership, admission and integration scope

Each selected NativeEnvironment separately admits at most 1,024 settings cells
at 512 bytes each, plus 16 KiB of shared administration: **528 KiB**, one scope,
1,025 leases. Factory clones share that limit; each cell keeps administration
alive until its own lease has dropped. Page cells may retain their Context's
client metadata after the factory is gone, but never a HostSession, topology,
browser connection, JS root, FFI command permit or task token. These declared
Rust administrative allowances are not total V8 allocator or process RSS bounds.

`assembleArguments` uses the existing revocation-safe synchronous phase scratch,
rechecks before replacing persistent roots, and transitions Ready to
ArgumentsReady. The sixth slot now stores assembled arguments instead of the
assimilated converted value; the six-value/192-total-root limit does not grow.
The existing 32-entry/16-scope preparation limits, 1 MiB per-entry FFI admission,
64 MiB shared FFI pool and independent 512 KiB synchronous scratch are unchanged.

The raw method currently receives typed settings from its caller. It does NOT
automatically bind or validate the source Handle's creation-Frame settings;
same-module metadata from another environment is not native authority.
Production facade binding, inherited public configuration and debug-mode
classification still need integration. Never substitute the current owner
Frame's settings or a prematurely flattened default-value snapshot. Public
`ElementHandle.setInputFiles` remains unexposed. Raw revocation fences establish
accounting, not public cancellation/error precedence: a kernel can finish its
language callbacks before its final live-entry check rejects a revoked phase.

### Verification scope and remaining gates

The offline argument lane combines real selected Rust timeout metadata and
stateless N-API kernels with the existing language continuation. Its oracle
runs the pinned Playwright 1.58.2 ElementHandle and Frame/TimeoutSettings bodies,
capturing arguments before channel validation. One explicitly labelled
direct-expression case exercises the assembly kernel without the method's await
flow. These tests do not create actual preparation tickets or browser connections.
Rust metadata tests separately cover capacity, reuse, shared live state and
factory/captured-cell lifetime; Worker checks cover real class loading and exit.

Final source verification is recorded under `.work/node173-arguments/`:

- Selected host release tests: **547 passed, 23 ignored**, including 14 timeout
  metadata cases (`host-tests.log`). Ignored browser cases are not passes.
- Standalone and combined addon release `--lib` tests: **57 passed each**
  (`addon-tests.log`, `addon-tests-combined.log`).
- Strict release Clippy passes for the selected host and four addon selections:
  default, existing `handles,file-input`, retained standalone and retained
  combined (`clippy-host.log`, `clippy-default.log`, `clippy-old-selection.log`,
  `clippy-addon.log`, `clippy-combined.log`). Builds were serialized with
  `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`; Cargo used `--release --locked`.
- Both rebuilt native artifacts pass **36 offline argument comparisons**, the
  **27 existing preflight comparisons**, and **one Worker smoke** each
  (`arguments-{standalone,combined}.log`, `preflight-{standalone,combined}.log`,
  `smoke-{standalone,combined}.log`). The Worker loads ten selected classes and
  creates actual Context/Page metadata objects, but zero preparation tickets
  or browser connections.
- Package contracts: **134 passed**; TypeScript and source/dependency boundary
  checks pass (`package-contracts.log`, `types.log`, `boundary.log`). Retained
  standalone/combined closures remain 240/241 packages including the host and
  255/256 including the addon; older selections remain unchanged. Runtime JS
  still imports no Playwright driver, CDP transport or process launcher.
- The expanded pinned upstream-only ingress oracle passes **51 cases**
  (`oracle.log`). Its channel-validation observations are not evidence that
  candidate channel validation has been implemented.

Reproduce each selected native lane from the repository root with
`PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm --dir packages/pwright build`,
then the same profile with `test:handle-file-input-arguments`,
`test:handle-file-input-preflight` and `test:preparation-boundary`. Repeat using
`retained-file-input-combined`. The latter selects
`retained-file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation`.

Final artifact byte sizes and SHA-256 digests (not process-memory measurements):

- `native/retained-file-input/pwright.node`: **17,579,216 bytes**,
  `42c24835f6f2df668eb4349873d4824d0539b65e88293fee04224d3fafb6da16`.
- `native/retained-file-input-combined/pwright.node`: **19,229,536 bytes**,
  `844b24a009e65bf0511ec23549f57563b37ae04060100421fc376f4cbc710d75`.

The shared dedicated Chrome endpoint remained unreachable: read-only status
failed and `/json/version` timed out after three seconds. No restart was
authorized or performed; this checkpoint has no new browser evidence and
started no background browser or server.

Keep production creation-Frame association, public setter semantics,
actual-ticket N-API cleanup, channel field order/error-prefix behavior,
deferred observable Buffer serialization, path handling, fractional native
deadlines, promotion/submission and browser/full discrepancy gates open.
Argument construction must be followed by channel validation in the same
continuation, not by awaiting this internal helper again.

## NODE-172: Rust-owned preflight and reentrant language phases

The selected preparation now implements owner lookup, owner Context resolution,
array classification and Buffer size preflight. Browser ownership and admission
remain Rust-owned. Public `ElementHandle.setInputFiles` is still unexposed;
there is no options/channel validation, path stat/read, byte materialization,
FFI execution promotion or native submission in this checkpoint. No feature,
dependency package, artifact profile, driver or browser process is added.

### Observable language order

Rust calls public `receiver.ownerFrame()` through an injected member-call port.
The thin language continuation awaits its raw result, then synchronously runs
Rust's falsy-owner check and `frame.page().context()`. Context resolution must
stay **outside** the async input converter: moving it inside changes when a
context exception reaches the caller. The following async converter invokes
Rust's array/size kernel and returns an ordinary own-data `{ payloads }` object,
not an internal ticket or metadata DTO. Even empty conversion is awaited.
Inherited `then` can participate in async-return assimilation and replace that
carrier; `finishConversion` preserves the actual awaited value for later phases.

The kernel observes `Array.isArray`, `files.slice`, and the returned items'
public `some`/`every`/`reduce` methods. Their results remain JavaScript values:
branching uses ToBoolean, addition and threshold comparison use actual JS
operators, not f64 coercion. A truthy first `item.buffer` read causes a second
read and then `byteLength`. All size reads precede names, MIME, options and
byte copying, none of which run yet. The 50 MiB threshold and fixed policy
errors are selected in Rust. Paths return a typed classification in the kernel;
the current parent explicitly reports unsupported before filesystem work.

One-expression CJS ports implement only member calls, getters, operators and
Error construction. Fresh arrow adapters preserve callback arity and lack of
constructibility. Native reducer callbacks capture no Env, scoped value,
registry or session: each invocation receives its ports from the adapter, so
an overridden array method may safely retain and invoke a callback later.
Pending user exceptions are neither stringified nor mapped to host errors.
Callback Function.toString/native stack shapes differ from upstream; helper
parameter names can also appear in generated TypeErrors. These are not claimed
as exact source/stack compatibility.

### Revocable phases and roots

`Ticket::phase` marks one populated entry busy, rejects concurrent same-ticket
reentry and returns a weak phase guard with a separate synchronous scratch
lease. `creation_phase` protects initial empty-entry construction in the same
way. Payloads stay in the registry rather than moving into the guard. Snapshot
inspection clones only Rust metadata and an Rc to the private root reference.
Reference lookup and all other N-API operations run after releasing the registry
borrow; the temporary Rc is dropped before user JavaScript runs.
Replacement is atomic, rejects a revoked entry, and drops old/rejected payloads
outside RefCell borrows. An unfinished guard aborts its exact entry. Completed
guards cannot abort a later phase started by an old payload's destructor.
Abort, scope close and environment shutdown reclaim registry roots while a
guard is held. Post-callback liveness checks avoid replacement allocation after
observable revocation; atomic replacement still checks again after allocation.

The production stages are Initial, WaitingOwner, ContextReady,
WaitingConversion and Ready. Direct rooted-value counts are respectively
3, 3, 4, 5 and 6: receiver/files/options, then Context, sliced items and the
awaited converted value. Checked metrics sum actual counts, at most 192 across
the 32 entries. The existing 1 MiB per-entry FFI preparation allowance, 64 MiB
shared FFI pool and 16 KiB fixed registry check remain in force.

Each registry additionally owns a selected-only scratch ledger: at most 32
simultaneous synchronous phases, each reserving 16 KiB (512 KiB aggregate),
with explicit scope/lease metadata limits. Admission precedes initial or
replacement root allocation. The lease covers bounded administrative overlap
and stays held through callback revocation, allocation-time finalizer reentry,
and old/rejected payload destruction. Reusing a revoked registry entry cannot
bypass the independent phase limit. Guards can keep this small ledger alive,
but hold no strong registry/session, native authority, FFI Command permit or
running-task token. They end synchronously and never span a Promise await.

These are declared administrative allowances, not measured V8 allocator bytes
or a bound on arbitrary reachable V8 graphs, scoped handles surviving until
the current callback returns, or escaped callbacks/user-held tombstones.

### Verification scope and open gates

Stateless selected-addon diagnostic exports call the same Rust kernel without
creating a NativeSession, preparation ticket, persistent Node root, native
authority or browser connection. The new offline differential lane targets
that kernel and the language driver against pinned Playwright 1.58.2; it is not
evidence of actual rooted-ticket integration, public channel/server behavior
or native upload parity. Registry tests separately exercise phase cancellation,
replacement, destructor reentry, weak lifetimes and checked root counts.

Final scratch-admitted source verification is recorded under
`.work/node172-preflight/`; `*-scratch.log` files supersede earlier intermediate
builds in that directory:

- Selected host release tests: **533 passed, 23 ignored**, including four new
  shared phase-budget cases (`host-tests-scratch.log`). The focused publication
  budget group separately passes all seven cases (`host-phase-tests.log`).
- Standalone and combined addon release `--lib` tests: **57 passed each**,
  including 53 registry cases, three publication cases and one class-identity
  case (`addon-tests-scratch.log`, `addon-tests-combined-scratch.log`). Initial
  construction, 32 revoked-entry guards, capacity reuse and scratch retention
  through old/rejected/closing payload destruction are exercised.
- Strict release Clippy passes for the selected host and four addon selections:
  default, existing `handles,file-input`, retained standalone and retained
  combined (`clippy-*-scratch.log`).
- Rebuilt standalone and combined native artifacts each pass **27 offline
  preflight differential cases** and **one Worker class/environment smoke**
  (`preflight-{standalone,combined}-scratch.log`,
  `smoke-{standalone,combined}-scratch.log`). These still create zero actual
  preparation tickets or browser connections.
- Node package contracts **134 passed**, pinned upstream-only ingress oracle
  **24 passed**, strict TypeScript and boundary checks pass
  (`package-contracts-scratch.log`, `oracle-scratch.log`, `types-scratch.log`,
  `boundary-scratch.log`). Exact selected normal closures remain 240/241 host
  packages and 255/256 addon packages; runtime JS imports no driver, CDP
  transport or process launcher.

All Cargo gates used `--release --locked`, `CARGO_INCREMENTAL=0` and
`CARGO_BUILD_JOBS=1`. Combined selection adds
`concurrent-close,extra-headers,drag-runtime,frame-navigation` to
`retained-file-input`. From `packages/pwright`, run `pnpm build`,
`pnpm test:handle-file-input-preflight` and `pnpm test:preparation-boundary` with
`PWRIGHT_NATIVE_PROFILE=retained-file-input`; repeat with
`PWRIGHT_NATIVE_PROFILE=retained-file-input-combined`.

Final darwin-arm64 artifacts, SHA-256 and byte sizes:

- `packages/pwright/native/retained-file-input/pwright.node`: 17,573,792 bytes,
  `75c6da834d22180e9b4734a789c136db6a49e074dea974dfd494ea1d94f76050`.
- `packages/pwright/native/retained-file-input-combined/pwright.node`:
  19,209,808 bytes,
  `15c18b674ec8adfa36c14c400dfb38ec3d0df8f37e8ddeb79c6fe49826e5f67b`.

The shared Chrome readiness check still reported unreachable and
`/json/version` timed out after three seconds. No restart was attempted.
Public close/cancellation error-phase
ordering, real ticket/N-API cleanup, paths, options, timeout domains, deferred
serialization, submission and real-browser comparisons remain open. In
particular raw preparation revocation can fail before later user getters;
that cannot be presented as verified public Playwright closed-target ordering.

The next bounded slice is post-conversion argument construction, before channel
validation: spread converted fields, then options, then resolve timeout from the
Handle's creation Frame. Preserve getter order, options overriding payloads and
fractional timeout values. It must run in the current continuation without an
extra await. Channel validation/error prefixes and later observable Buffer
serialization remain separate gates; no premature byte copying or native
execution promotion is authorized by successful preflight.

No new commit/push or background browser/server was created. Read-only memory
checks remained at zero swap; app/backend RSS is shared process evidence, not
per-chat memory attribution.

## NODE-171: session-bound Node preparation registry

The addon now has a separately selected `retained-file-input` feature and two
new artifact profiles: `retained-file-input` and `retained-file-input-combined`.
The former selects Handles, file input and retained upload; the latter also
selects concurrent close, drag runtime, Frame navigation and extra headers.
All thirteen older profiles retain their existing feature sets. This is an
internal preparation boundary, **not** public `ElementHandle.setInputFiles`.
No portable protocol, dependency package, browser command or worker is added.

### Exact provenance without premature native authority

The selected `NativeHandle` keeps an immutable `HandleSourceIdentity`: weak
Owners identity, target ID, creation Frame ID and Element kind. Capturing it
does not acquire a ticket, native lease, snapshot, task or strong Owners
reference. Host validation checks the exact live session and Element kind,
but deliberately does not reject a disposed descriptor at this metadata phase.
Pinned public ownerFrame overrides and later conversion getters must not be
skipped by premature disposal validation. Actual upload still needs the full
source/world checks at native submission; this metadata cannot authorize use.

`NativeSession.handlePrepareFileInput` validates that provenance, admits a
revocable FFI preparation, then roots receiver/files/options in a private
three-element Node array. Own data-property definitions avoid invoking an
inherited Array prototype setter. No input getter, Buffer pointer, filesystem
operation or browser operation runs here. Primitive inputs can be rooted
without treating them as independently referenceable N-API objects.

The returned module-local, type-tagged `NativeFileInputPreparation` has no
public constructor or exposed numeric authority. It contains a weak exact-
session identity and never-reused registry ID. `abort()` releases its entry
explicitly and `isPending()` reports revocation. Raw creation-Frame inspection
checks that exact session again; another session in the same environment cannot
substitute its shared FFI budget as authority. Generic native publication tags
the instance before wrapping it, compensating construction/tag/wrap failures
by dropping the untransferred ticket. Existing NativeHandle publication uses
the same failure-safe implementation.

### Admission and shutdown boundaries

The thread-affine registry has 32 environment entries and 16 live session
scopes, with a checked 16 KiB fixed administrative reservation. Each entry
shares the existing FFI Command lane and reserves 1 MiB within the environment's
existing 64 MiB FFI pool; it owns no TaskTracker token during JavaScript waits.
Up to three directly rooted argument values per populated entry are counted.
This is not a byte bound on arbitrary reachable V8 graphs. User-retained closed
wrappers and weak ticket tombstones can also outlive the active registry slot;
the administrative reservation is not a bound on all their backing allocations
or on process RSS. No copied payload exists yet.

Explicit abort, session close, session wrapper Drop and environment shutdown
remove entries and drop their roots on the Node thread, outside the registry's
RefCell borrow. Payload destruction precedes release of its FFI allowance.
The newly registered environment hook runs before the older hook's synchronous
FFI/native drain. A held weak ticket cannot keep its scope, registry, Node
arguments or native session alive after revocation.

Terminal publication revokes its session scope before invoking lifecycle
callbacks; normal Page/Context notices do not revoke the whole session.
The public facade installs the existing session watcher. A raw NativeSession
without that watcher has no new thread-safe callback: after peer disconnect,
root cleanup can wait for a later publication, explicit close, wrapper Drop
or environment shutdown. A host validation error alone does not revoke roots.
Immediate peer-close cleanup for such an unobserved raw session is not claimed.

### Verification scope and remaining work

The Rust registry tests exercise real admission with deterministic fake payload
destructors, including reentry, exact-session rejection, overflow, shutdown and
held-ticket lifetimes. Host tests separately exercise weak source provenance
and disposal. The selected-artifact Worker smoke loads the actual addon,
checks the nine-class inventory and constructor/type tags, constructs the real
environment, then exits through its cleanup hooks. It deliberately creates
**zero actual preparation tickets and zero browser connections**: clean Worker
exit does not establish actual rooted-ticket teardown or listener ordering.

Current logs are under `.work/node171-preparation-registry/`. The selected
release/locked host suite passes **529 tests**, with **23 ignored**, including
seven new source-identity tests. Both standalone and combined addon suites
pass **32 tests** each: 28 registry, three generic publication, and one class-tag
inventory test. Strict all-targets Clippy passes for both new selections.
Package contracts pass **134**, the pinned upstream-only ingress oracle passes
**24**, strict TypeScript declarations pass, and the exact dependency/feature
fingerprint gate passes. No running browser is used by those checks.

Both darwin-arm64 artifacts were built explicitly and passed the offline
Worker smoke described above, including direct method calls with forged and
foreign-class receivers. Their SHA-256 values are:

- `retained-file-input`: `62b3d66c18a7430a57df02837c5a4d7c124c6cbbca2ac4e0a1cd3d744e32744f`
- `retained-file-input-combined`: `4be2757cc74e558a4c5bab15f71802886822d66df83e541df755f3856f24e6d9`

The initial check found a nonexistent N-API property-attribute constant, fixed
by composing the supported writable/enumerable/configurable flags; the first
Clippy run found an identity `map` in a test, fixed with `inspect`. The original
failure logs are preserved separately from the successful `*-final.log` runs.
A combined build-script sample showed a temporary pre-main `_dyld_start` wait
and 96 KiB reported footprint; it subsequently completed without intervention.
That observation does not establish the cause of the launch delay.

CI now explicitly selects addon library tests with `--lib` (the default library
disables automatic tests), plus strict all-targets Clippy, for standalone and
combined retained-upload selections. Workflow YAML/command structure checks
pass; editing those commands is not evidence of a remote CI run. Strict
all-targets Clippy also passes for default and old `handles,file-input`
selections. Formatting and diff whitespace checks pass. The two new generated
native binaries are explicitly ignored by Git; no new commit or push was made.

At 09:27:30 UTC, all task-launched checks/builds had exited and no new background
server or browser remained. Shared-process RSS in KiB: app PID 12352 = 407056,
backend 12747 = 290928, renderer 12750 = 637232, dedicated Chrome 1654 = 835136.
Changes since 09:15:53 were +912, 0, +5808 and +176 KiB respectively. Swap was
zero; the system compressor occupied 102270 16-KiB pages. These observations
are not additive unique RAM or exact per-chat attribution. Unrelated cargo
PID 66696 remained running and was not changed.

Public owner/context lookup, phase-correct conversion, deferred serialization,
timeout semantics, path handling, FFI promotion and native upload submission
are still unwired. Real rooted-ticket cleanup, cross-artifact rejection and
public browser/differential verification remain separate gates. The shared
Chrome is not restarted and historical live passes do not validate this build.

### Next bounded implementation slice (not implemented)

Implement only the owner/Buffer pre-options phase before expanding the public
API. Its observed sequence is public `receiver.ownerFrame()` and await, null
owner rejection, `frame.page().context()`, observable array classification,
then the complete size pass, followed by an await even for empty input. Truthy
payload buffers are read twice before their byteLength; name, MIME, options
and byte materialization belong to later phases. Paths require an explicit
deferred path-preparation state, not a claim of supported native upload.

Keep orchestration in Rust, with narrow injected ports for JavaScript await,
operators/coercion and calls that N-API cannot reproduce directly. Before any
user code, check out a phase and release every registry borrow. The current
`Scope::inspect` is intentionally unsuitable for invoking JavaScript: it holds
a RefCell borrow. A phase guard must reject replay/reentry, observe abort/close
and never reinstall state after revocation. Extra owner/context/item roots need
explicit accounting; the current three-value metric cannot silently expand.
No FFI running-task token may survive an arbitrary owner Promise wait.

An offline selected-addon harness should exercise that same production kernel
against pinned oracle traces, rather than duplicate the candidate in JavaScript.
Until that exists, pure Rust state tests are not N-API ingress parity. Options
spread, channel validation, creation-Frame fractional timeout/domain behavior,
nested owner error prefixes, custom Buffer serialization/server error identity,
paths, promotion, submission and live browser gates remain later work.

## NODE-170: revocable FFI preparation before execution

The host-only `retained-file-input` selection now exposes a move-only
`FfiPreparation<T>`. It is a prerequisite for staged language conversion, not
the Node upload preparation registry or public `ElementHandle.setInputFiles`.
No addon feature/artifact, Node class, dependency, browser command or additional
budget is introduced.

### Admission without waiting for arbitrary JavaScript

`FfiBudget::prepare` uses the same lane, PendingCall and caller-specified byte
allowance as ordinary `admit`, but does not create a TaskTracker token. A user
overriding ownerFrame with a Promise that never settles must not become Rust
work that environment shutdown waits to join. Preparations and existing eager
commands share finite capacity; they cannot bypass each other's count or byte
ceilings. Dropping a preparation reclaims its existing permit once.

`start` consumes the preparation, checks exact budget identity and live permit,
and registers execution under the same registry lock as shutdown. Promotion
does not reserve a second slot or byte lease. Its lock order is registry then
entry; reclamation removes permit/value under the entry lock and drops them
after unlocking. A preparation that loses to shutdown cannot register late
work. One that wins becomes ordinary cancellable FFI execution, which must be
run or dropped immediately: no further arbitrary JavaScript wait may retain
its work token.

Shutdown reclaims preparations without requiring their holders to resume. A
still-held receipt retains only an empty ticket cell and weak budget identity,
not the registry/ledger, native service, task or Node reference. This is an
admission/ownership guarantee, not a measurement of process RSS or a promise
to reclaim graphs retained by the user's own unresolved JavaScript Promises.

### Budget identity is not session or Node authority

The addon shares one FfiBudget across sessions in an environment. Exact budget
identity therefore cannot authorize a sibling session's Handle. The future
Node-thread registry must separately bind the originating session/source,
retain bounded roots only after admission, support explicit abort and per-session
revocation, and drop roots on their owning thread before environment FFI drain.
Module-local class tags must precede raw class casts if a ticket class is exposed.
An independently supplied numeric ID is not authority.

Do not hold `Owners::admit_tracked_call`, a world snapshot, native service or
FfiExecution across owner lookup/conversion. `Owners::admit_call` is also not a
replacement for revocable preparation: its allowance can outlive close while
held by a descriptor. Re-enter HostSession/source validation at submission;
an old file-input Installation has no independent session-liveness authority.
This refines NODE-168's proposed preparation owner: admission is phase-local,
and native work/cleanup tracking is acquired only when that phase starts.

### Additional upstream-only observations

The offline oracle now also checks public overridden owner Frame `page()` and
Page `context()` calls before either Buffer or empty-input inspection, their
exception identity/precedence, an async conversion boundary even for no files,
and disposal reentry during the size pass followed by later channel validation.
The disposal case executes actual client JSHandle.dispose/Connection metadata
retirement using a dispatcher-shaped fixture event; it proves no native DOM
disposal or candidate behavior. The earlier 16-case NODE-169 evidence remains
a historical checkpoint, not the current oracle count.

Pinned server preparation calls each raw Buffer's public `toString('base64')`
in order. The existing mutable-Buffer oracle already observes this call. Native
raw-byte copying must not silently claim equivalence for custom serializer
getters/functions, their exceptions or their mutations of another payload.
Three additional executed cases establish that a custom serializer can replace
raw `[1, 2]` with serialized `[9, 8]`; a thrown serializer Error is copied by
the server/client error transport (unlike client-side getter identity); and a
later serializer mutating an earlier Buffer cannot change that earlier item's
already-produced snapshot. These are observations of actual pinned client and
server helpers through the bounded offline transport, not a DOM mutation pass.
Any future fast path must preserve applicable language observations before
borrowing backing-store pointers; no JavaScript invocation may occur while a
Rust byte slice is live. This remains an ingress design/verification gate.

### Verification scope and remaining work

Fourteen deterministic preparation tests cover shared admission, exact-origin
checks, drop/revocation, both shutdown/promotion orders, held-receipt shutdown
without a timer or callback, and absence of a strong budget owner after revoke.
Root reran the selected release/locked host suite: **522 passed**, **23 ignored**;
strict all-targets Clippy passes. The ordinary `handles,file-input` selection
passes its **14 existing FFI tests**, without compiling the new preparation API.
The offline oracle passes **24 tests**, package contracts pass **132**, and strict
TypeScript declarations pass. Logs are under `.work/node170-preparation/`, with
the final 24-case oracle in `oracle-final.log`; earlier 21-case logs are retained
as earlier checkpoints. These gates use no running browser or addon artifact.

The final all-features host suite passes **584 tests**, with **25 ignored**;
the full ordinary `handles,file-input` host suite still passes **491**, with
**22 ignored**. Both the combined host and old addon all-features/all-targets
strict Clippy gates pass. The exact dependency/feature fingerprint gate,
formatting and diff whitespace checks pass. The current Node artifact was not
rebuilt or browser-tested, and no new commit or push was made.

At 08:58:17 UTC, the serialized builds had exited and this checkpoint left no
new background server/browser. A read-only process snapshot reported RSS in
KiB: app PID 12352 = 403520, shared backend 12747 = 290720, renderer 12750 =
667680, shared dedicated Chrome 1654 = 834672. Relative to 08:45:03 these changed
by +1040, +672, +40448 and +208 KiB respectively. Swap remained zero; the system
compressor occupied 102567 16-KiB pages. These are separate shared-process RSS
and system compression observations, not additive unique RAM or per-chat
attribution. No service was stopped or restarted to obtain these numbers.

The full Node registry, source/session revocation, language phases, timeout-domain
compatibility, two-phase paths, addon integration and real browser/differential
verification remain unfinished. A successful pure-Rust preparation test does
not prove Node-reference cleanup, Worker teardown or public API parity.

## NODE-169: safe-host retained upload and pinned ingress oracle

The new **Rust-host-only** `retained-file-input` feature implies `handles` and
`file-input`, plus `retained-reference` on the Handle evaluation leaf and the
Locator-resolution composition/backend. Existing selections remain unchanged.
`HostSession::handle_set_input_files` consumes already-materialized,
installation-bound payloads and an exact retained ElementHandle; it is not the
public Node API or its staged language conversion. No addon feature/artifact,
portable schema, selector producer or Node method is added in this checkpoint.

### Source identity and original-Frame routing

The Handle leaf exposes a read-only `ChromiumRetainedElementSource` receipt.
Borrowing and revalidation are synchronous and perform no browser I/O. They
check live runtime target/parents, the exact route payload/client/native ID,
Element kind and bounded ID, creation Frame/Page and numeric/unique main-world
identity. A same-field foreign receipt cannot substitute for that exact source.
IDs are shared from existing bounded storage, not copied into another registry.
The receipt grants no native release authority and is **not** a native-use pin.
Explicit disposal can still win after validation and make acquisition fail.

The host uses the original Handle creation Frame's paired main/utility snapshot,
not the current ownerFrame's utility world. Pinned upstream server `dom.js`
`evaluateHandleInUtility` uses `this._frame._utilityContext()`; client ownerFrame
instead selects the Context for file conversion. Before adoption the host
validates the persistent ticket and receipt against the snapshot. After adoption
it revalidates the persistent source and exact observed world before mutation;
failure explicitly releases the temporary group. This conservative concurrent
disposal/replacement behavior still requires live differential characterization.

### Phase-local cancellation and shutdown admission

Atomic tracked-call admission shares the existing eight Handle calls, checks
the source session/live ticket under the same lock as close, and registers
shutdown work without reserving another persistent Handle or native unit. The
new resolver independently admits sixteen native units, up to eight adopted
attempts, and its explicit scratch. The generic resolver profile also declares
ordinary selector capacity, but this path never reserves SelectorBytes or runs
a selector. Only the new selection also adds eight paired-world snapshot and
native-retirement registry slots, because abandoned retained acquisitions can
outlive their runtime lanes independently of normal Locator work. Selected
capacity is 81 snapshots (1105 with drag runtime), including existing Handle
and continuing-upload allowances. Default/older profiles and ordinary Locator
resolution budgets/deadlines are unchanged.

An independent eight-slot **untimed** adoption cleanup executor is installed
only by the new host feature. It must not reuse the ordinary five-second
cleanup executor, which could discard a still-pending native acquisition. It
also must not share the upload executor's sole slot: OwnedElement retains the
adoption reservation while the submitted upload reserves its own continuation.
Existing Handle call/file-input call allowances cover retained source metadata,
route/proof projections and checked inline future storage; bytes remain owned
by the independent file source lease.

Acquisition and submitted mutation keep separate continuations. Dropping an
acquisition waiter finishes only that acquisition and releases the resulting
group; it **never** advances into a previously unsubmitted upload. Once mutation
has started, its existing UploadFuture retains bytes, source slot, exact group,
snapshot and tracked Handle work through native settlement and cleanup. Upload
preparation rejection still transfers the OwnedElement's reserved cleanup.
No workflow-wide continuation silently submits new work after cancellation.

The global HostSession operation token belongs only to the public waiter,
because shutdown waits it before inspector-session retirement. The tracked
Handle work token belongs to the native route owner and drains after retirement.
Shutdown subsequently drains both independent upload/adoption executors and
reports cleanup health; simultaneous adoption and mutation cleanup failures
retain both diagnostics. A group not yet validated in its unique world cannot
borrow an old-world proof; close during that stage can conservatively report
Cleanup even after detach. Such errors must not be rewritten as success or
hidden by a later publication error. No graceful-close parity is claimed yet.

### Executed offline oracle, not candidate parity

`pnpm --dir packages/pwright test:handle-file-input-oracle` executes the actual
local pinned Playwright 1.58.2 ElementHandle/Frame/Connection/channel validators
and server `prepareFilesForUpload`. Only metadata, owner responses and an
explicitly bounded deferred raw-Buffer transport are fixtures. No Browser,
CDP connection, native addon, Dispatcher ProgressController or DOM mutation is
created. All channel callbacks must settle before fixture teardown.

The **16 passing cases** establish owner-before-getter behavior, the complete
size pass before options and channel fields, public ownerFrame override use,
creation-Frame timeout (including fractional timeout acceptance), owner-Context
path observation, path errors before option reads, microtasks at conversion and
dispatch, later-getter Buffer mutation, protocol-field options overrides, and
phase-dependent Error identity/prefix and primitive-throw behavior. The raw
Buffer remains mutable until actual server preparation snapshots it. These
results strengthen the NODE-168 ingress contract; they are not a candidate
score and do not establish browser/native behavior.

### Executed host and boundary checks

Local release/locked results are retained under
`.work/node169-retained-upload/`; Cargo runs use one build job and no incremental
artifacts. The selected host suite passes **508 tests**, with **23 ignored**
browser/diagnostic tests. The old `handles,file-input` host combination passes
**491 tests**, with **22 ignored**. The source-receipt suite passes **12 tests**;
both its leaf and the selected host pass strict all-targets Clippy after fixing
a nested cleanup conditional in the ignored live test. The latter now compiles,
but compilation does not establish a browser result. The all-features host's
focused snapshot/retirement-capacity test also passes (1105 declared slots),
and the old addon all-features/all-targets strict Clippy selection passes.
The capacity test checks requirements, not eight simultaneous assembled-host
cancellations. CI configuration now includes the selected source/host and
combined-capacity gates; those CI jobs have not been run remotely.

The exact normal package/feature fingerprint gate passes for selected and
ordinary host/addon combinations. Package contracts pass **132 tests**, and
strict TypeScript declarations pass. The assembled host test is deliberately
ignored: the shared dedicated Chrome's loopback CDP endpoint timed out again,
and restarting that browser has not been authorized. No addon artifact, native
upload result, candidate differential pass, browser cleanup or CI run is claimed.

### Remaining integration gates

The Node layer still needs a bounded preparation owner with phase-correct
root/exception rendezvous, public ownerFrame invocation, separate size/channel
passes, delayed backing-store validation and Rust byte copy, options/timeout
semantics, and path stat/read separation. The current safe-host submission
accepts already-owned data and a u32 timeout; it does not itself reproduce those
observable client phases or upstream's broader timeout domain. No early byte
snapshot, current-owner utility substitution or selector re-query may be used
as a shortcut. An explicit addon selection, public producers, real browser
upload/cancellation/close gates and the full differential corpus remain open.

## NODE-168: group-owned retained adoption and deferred upload ingress

The resolver backend and composition now have an explicit default-off
`retained-reference` feature. This is a lower-level retained-upload prerequisite,
not completed `ElementHandle.setInputFiles` support or a new Node artifact.
The new request uses the exact borrowed native receiver, independently adopted
into a pre-admitted temporary group, never a fabricated selector or the
persistent source's release group. An Arc of PendingHandle is not a native-use
pin. NODE-167's artifact-specific evidence below remains unchanged.

### Independent group ownership and native-world validation

The backend validates exact source/target client identity, bounded native IDs
and a positive utility numeric ID before I/O. It admits a never-reused group
name, native owner and attempt scratch before describing the source. A describe
failure has created no native object. Grouped DOM.resolveNode then uses a new
explicit settlement method; ordinary timed resolve methods remain unchanged.
Malformed, missing, oversized or source-alias IDs are compensated by releasing
the entire independently owned group, never by releasing the borrowed source.

Numeric context identity alone is insufficient: Chromium may reuse it before
replacement events reach topology. After resolve, a constant by-value probe
passes the new object as an argument in the expected unique utility world.
There is no DOM mutation, selector query or user getter. Only a successful
native validation permits attaching the supplied world-destruction proof to
the owner. Before validation, failures require actual group release and cannot
be discharged by an old world's receipt. The raw resolve envelope is dropped
before waiting on the probe. Browser verification of this new pipeline remains
pending; deterministic tests establish the boundary and fault handling.

Success returns an actual ResolvedElement, wrapped by the existing continuation
as an OwnedElement. Its group release selects terminal settlement; other
resolver owners retain their ordinary release behavior. Pre-reserved cleanup
outlives abandoned acquisition, returned owners and explicit release futures,
including their caller-budgeted route/source references. Unpolled work performs
no browser I/O. Failed cleanup seals further admission and remains visible to
the executor; returning local leases does not assert native release.

### Explicit resource selection and bounded retained diagnostics

Compilation alone does not admit retained-reference work. The caller must also
select `ResolutionProfile::with_retained_reference()`. Each attempt uses two
existing RemoteHandle units (adopted node plus possible probe exception),
4736 owner bytes and a separate 14336-byte BackendStateBytes scratch lease.
The profile adds `floor(native_units / 2) * scratch_bytes`, without changing
scope/lease ceilings or SelectorBytes. The scratch calculation includes eight
bounded native-ID copies, two 1024-byte diagnostics and 4096 bytes of checked
control/future overhead. A compile-time assertion covers inline attempt storage.
Scratch survives failed-acquisition group release; successful publication drops
scratch while the owner retains both native units until release.

Native string diagnostics are copied into compact bounded UTF-8 allocations;
truncating a String in place would retain its old capacity. Protocol codes and
terminal/resource variants remain typed. Opaque JSON/compound errors become a
bounded streaming-formatted diagnostic. Returned errors belong to the consumer
after the call, not to a forever-retained native owner. These bounds do not
prove the transport decoder's raw-envelope peak or the transient exception
string formatted by the existing CDP by-value helper before normalization.
Those incoming/transient allocations remain a distinct CDP-boundary concern.

The authoritative utility route and paired snapshot now retain numeric and
unique context identity together. Numeric-only replacement invalidates both
the cached route and snapshot. Dom-content and Locator selected route plans
add `2 * size_of::<i64>()` per route for current/replacement numeric metadata;
their native-string budgets and lease counts remain distinct and unchanged.
The older Handle-adoption continuation also now reports typed Cleanup failures
instead of turning an abandoned failed compensation into success. That narrow
repair does not make its ordinary timed acquisition equivalent to the new
group-owned retained-reference path.

### Pinned caller-side order

The local oracle is Playwright 1.58.2. Its
[client ElementHandle implementation](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/client/elementHandle.js)
(`setInputFiles`, lines 118–123; conversion, lines 206–265) requires this order:

1. Await public `this.ownerFrame()` before accessing files or options. A null
   owner rejects with `Cannot set input files to detached element`.
2. Convert files using the current owner's Context. Arrays use observable
   `.slice()`, then `.some()`/`.every()`; mixed paths and payloads reject first.
   Memory payloads run the complete size pass before name/MIME validation:
   `item.buffer ? item.buffer.byteLength : 0`. A truthy buffer getter is read
   twice per item; aggregate size `>= 50 MiB` rejects. Paths instead undergo
   sequential `stat` and normalization, with directory/mixed-input checks.
3. Await conversion, including a microtask boundary for memory payloads.
4. Spread options, invoking enumerable getters including unknown fields, then
   compute timeout using the original creation Frame, not the current owner.
   An own enumerable numeric timeout normally has three reads: the spread,
   numeric check and return.
5. Validate channel fields: each payload's `name`, optional `mimeType`, then
   `buffer`, all payloads before the final timeout field. Only afterward send
   the retained-handle operation for server preparation and mutation.

Sources for the last two phases are
[TimeoutSettings](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/client/timeoutSettings.js)
(lines 55–64),
[ElementHandleSetInputFilesParams](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/protocol/validator.js)
(lines 2197–2207), and
[validator primitives](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/protocol/validatorPrimitives.js)
(lines 84–151). `noWaitAfter` is spread/read but absent from that schema;
unknown options are not rejected, and protocol-field names can overwrite
converted fields. Boxed strings are accepted by string validation; timeout is
not the existing ingress's unsigned-integer-only contract.

Exception behavior is phase-dependent. File conversion and options getter
throws occur outside channel wrapping. Channel validation runs inside
[`ChannelOwner._wrapApiCall`](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/client/channelOwner.js)
(lines 143–175), which assigns to `error.message`; original exception identity,
setters and primitive-throw behavior cannot be replaced by generic HostError
stringification. Owner preflight failures need the outer setInputFiles API
attribution, not a double-prefixed ownerFrame error. Upstream's public
`this.ownerFrame()` override behavior must be preserved or explicitly scoped out.

### Deferred bytes and two-phase paths

The pinned local
[in-process factory](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/inProcessFactory.js)
(lines 30–47) enables raw Buffer references and deferred `setImmediate` dispatch.
Client channel validation does not copy those bytes. Server
[`prepareFilesForUpload`](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/server/fileUploadUtils.js)
(lines 44–78) later snapshots payload bytes with `buffer.toString('base64')`.
Later metadata/options getters and microtasks before dispatch can therefore
change earlier buffers. Paths have distinct phases: client stat/normalization
before options, then server-side reads/MIME/encoding afterward. The action
timeout begins in
[`Dispatcher._runCommand`](../../tests/node_modules/.pnpm/playwright-core@1.58.2/node_modules/playwright-core/lib/server/dispatchers/dispatcher.js)
(lines 87–94), not before ownerFrame or client conversion.

The current [JS upload adapter](../../packages/pwright/src/file-input.cjs) reads
options first; [native Locator ingress](../../crates/pwright-node-addon/src/locator/file_input.rs)
captures name/MIME/buffer and copies synchronously, without the earlier size
pass or deferred-copy boundary. Its timing and the combined path-read worker
cannot be reused unchanged for exact retained-upload ingress parity.

### Minimum typed preparation owner and Node-thread rendezvous

Use one bounded, move-only, session-bound Rust preparation ticket, not a new
persistent Handle result. Pre-admit it before async owner lookup. Proposed
phases are owner-checked, payload-references-sized or paths-observed,
channel-validated, deferred source-materialization, and submitted/aborted.
The ticket holds target identity, originating installation, original timeout
scope, operation admission and pre-owned continuation/cleanup capacity.

The Node-thread rendezvous may retain bounded rooted language values and
original exceptions; Rust worker futures receive only owned typed data.
No callback-scoped `Unknown`, raw Buffer pointer, Mutex guard or RefCell borrow
may cross an await or user getter. After the deferred copy phase begins,
perform every backing-store check and admitted Rust copy without invoking JS.
The existing [deferred conversion registry](../../crates/pwright-node-addon/src/deferred_conversion.rs)
(lines 244–345) supplies the structural model: remove an entry before callbacks,
consume it once, handle scheduler failure, and drain pending roots during
environment shutdown. A new raw ticket class requires module-local TypeTag
checks; an unscoped numeric ID is not authority.

Path observation must remain separate from later byte materialization. Getter
reentry can dispose/navigate/close the target: complete the applicable client
validation phase, then revalidate before retained-target submission. Preserve
the selected phase's primary exception while explicitly aborting unsubmitted
tickets. After submission, cancellation transfers the same owned future and
source/group admission through native settlement and cleanup; GC is not the
primary release mechanism. The implemented lower-level group owner still needs
Node preparation integration and real-browser failure/abandonment verification.

### Verification and artifact boundaries

Current local release/locked evidence is retained under
`.work/node168-retained-adoption/`. Cargo runs were serialized with
`CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`:

- Selected resolver backend: **47 passed**; selected continuation composition:
  **43 passed** (`retained-budget-final.log`). This includes explicit profile
  selection, scratch admission, malformed/alias native results, native-world
  proof ordering, cancellation and failed group cleanup.
- Unselected resolver backend/composition: **23 + 37 passed**
  (`default-regression.log`). Full CDP library: **116 passed**
  (`cdp-regression.log`); ordinary timed command behavior remains covered.
- Raw CDP grouped-resolve and grouped-release tests: **3 + 3 passed**; Fake
  integration: **3 passed** (`cdp-resolve.log`, `cdp-release.log`, `fake.log`).
  These distinguish late native settlement from ordinary command deadlines.
- Legacy adoption failed-compensation regression: **1 passed**; utility numeric
  snapshot regressions: **3 passed**; Dom-content and Locator numeric route
  budget tests: **1 + 1 passed**. Complete Chromium composition library:
  **381 passed, 16 ignored** (`composition-worlds-final.log`). Focused results
  overlap full suites and are not an additive test total.
- Handles host library: **489 passed, 14 ignored** (`host-regression.log`).
  Strict all-target Clippy passes for the selected resolver leaves and the
  all-feature Node addon (`clippy-retained-final.log`, `clippy-addon.log`).
- The resolver dependency proof passes both unselected and selected profiles:
  backend **5**, composition **7** Pwright packages, with identical complete
  normal dependency closures (`dependencies.log`). `pnpm --dir packages/pwright
  test:boundary` also passes (`node-boundary.log`). No new normal dependency,
  Node artifact profile, public upload method or protocol schema is introduced.

The new ignored
[real-Chrome test](../../crates/pwright-backend-cdp-locator-resolution/tests/retained_chrome.rs)
compiles with `--features retained-reference --test retained_chrome --no-run`
(`chrome-test-compile-final.log`).
It is **not executed or a live pass**. Once the shared browser is responsive
and dedicated-profile preflight is complete, it checks matched numeric/unique
world adoption, physical invalidation of the released temporary, continued
usability of the borrowed source, and mismatched-world rejection. The mismatch
case relies on the acknowledged release contract; zero local leases alone is
not native-retirement evidence. The test uses only its own dispose-on-detach
Context, Page, groups and connection, with bounded cleanup attempts even after
exercise failure, panic or timeout. It never starts or closes Chrome.

Chrome's loopback version endpoint still timed out during this checkpoint.
No restart was authorized or performed. NODE-167's last cleanup artifact and
its unfinished repeat remain unchanged: later Rust tests do not promote that
artifact to a live ownerFrame pass. No addon was rebuilt in this checkpoint;
the retained-upload lane and full public corpus have not been rerun.

At 2026-09-17T08:22:06Z, the read-only process snapshot reported RSS of
392.5 MiB for ChatGPT PID 12352, 282.5 MiB for shared Codex backend PID 12747,
610.1 MiB for renderer PID 12750 and 814.5 MiB for dedicated Chrome PID 1654.
Compared with 08:18:31Z those changed by approximately +1.5, +0.1, +8.3 and
+0.1 MiB respectively. Swap remained zero. These are process RSS observations,
not unique physical footprint or per-chat attribution. No preview server was
started; the former task port 9392 has no listener. Earlier NODE-167 native
page cleanup remains unverified while Chrome is unavailable.

### Required gates and explicit gaps

The unchanged eighteen-case retained-upload lane does not establish full
getter/copy ordering. Add bounded comparisons for complete getter traces,
owner failure before options, later-getter and microtask Buffer mutation,
detach/resize before deferred copy, reentrant disposal plus later getter
failure, bad paths versus options errors, and path preparation versus timeout.
Ticket replay/foreign session, admission exhaustion, cancellation, Worker
shutdown and capacity reuse must prove deterministic cleanup without GC.

An explicit combined handles+file-input artifact/dependency proof and missing
Page/Frame `$`/Locator.elementHandle producers remain separate prerequisites.
Overridden array methods, boxed values, overridden Buffer serialization,
protocol-field options overrides, remote Playwright streaming and native
directory/path behavior need explicit characterization or declared limits.
Selected resource bounds must not be silently relaxed to mimic unrestricted
input behavior. No new live result or compatibility score is claimed here.

## NODE-167: current DOM owner Frame and retained-upload preparation design

The Handles profile adds `ElementHandle.ownerFrame()`. The Rust core queries
the retained node's current owner document (or a Document itself), obtains a
temporary document-element receiver, reads its native Frame ID, releases that
receiver, and maps the ID through current topology in the source BrowserContext.
The Node layer only returns the canonical cached Frame wrapper. Descendants
from evaluateHandle, getProperty and getProperties retain the same Context
resolver. There is no selector query, cached creation-parent answer, new browser
driver, package, feature or portable protocol version. Host-local method 65006
is selected only by the existing handles profile/handle-value leaf selection.

Removed nodes can still have a live owner Frame. A document without a browsing
context returns null. A node created in the main evaluation world but owned by
a child document must resolve to that child; moving the node between documents
changes its answer. Native Frame IDs never escape the Rust host as public IDs.
The topology fence precedes a single bounded current topology lookup restricted
to the original BrowserContext. A vanished destination returns null, not the
creation Frame. JS retrieves existing wrappers from the Context/Page caches.

### Temporary ownership, admission and cancellation

One atomic tracked-read admission checks the exact session/live ticket and
registers work under the same lock as owner shutdown. It shares the existing
eight call slots and 8 MiB call allowance; it does not consume a new slot from
the 64 persistent results. Extension scope 154/1 additionally reserves two
temporary RemoteHandle units per read: one successful document element, or
distinct result and exception receivers during failed acquisition. The
aggregate selected limit is sixteen temporary units and eight additional
resource leases. Existing cleanup capacity remains 112; no worker or queue
is introduced.

Before native work the host reserves a continuation and retains a source-world
snapshot so shutdown can retire its inspector session before draining work.
The snapshot is not proof that a generic cleanup error succeeded. Started
cancellation transfers the same future, call/native admission and work token;
unpolled cancellation performs no native work. The caller's shared call charge
survives reply/topology mapping. Explicit disposal can win before acquisition;
a descriptor Arc is not a native-use pin. Once acquired, the temporary root has
its own ID and is released without consuming the persistent source Handle.

The backend keeps one evaluation lease through acquisition, direct raw
object-ID DOM.describeNode and terminal release-to-settlement. The describe
read keeps its ordinary deadline because it creates no RemoteObject. Custom
CDP clients must implement that explicit raw method; no lossy backend-node-ID
detour or empty default response is used. Frame-ID copies and diagnostics are
bounded. Invalid/missing acquired IDs still require a pre-owned group and stay
honest Cleanup failures. Generic compensation-versus-detach remains conservative;
this checkpoint does not suppress it using a stale host snapshot.

OwnerFrame selects a narrow protected-receiver acquisition path: one bounded
source ID is copied only after evaluation admission. A successful response
aliasing the source is rejected as Cleanup. Malformed/exception compensation
also excludes that exact borrowed ID while attempting all distinct new owners.
Other producers retain their previous acquisition behavior; this is not a
claim of generic borrowed-ID protection across all Handle operations.

### Navigation error identity without retaining native services

The first built paired run passed fifteen cases and failed navigation error
parity: the candidate returns TargetClosed while upstream reports execution
context destruction. The topology transaction deliberately disposes runtime
Handle objects and their native routes on main-world removal/replacement,
although the Page and portable Frame can remain alive. Looking up the already
retired route cannot recover the original reason.

Scalar and batch publication therefore preserve only immutable creation-world
metadata on each PendingHandle: Page ID, numeric execution context and a
bounded shared unique-context string. This descriptor metadata holds no native
client, lifetime receipt, snapshot permit or route lease. Its maximum identity
bytes are explicitly included in the existing 16 KiB owner allowance.
Before ownerFrame dispatch, and when refining an ordinary TargetClosed result,
Rust compares that fingerprint with an observed newer main world on the same
still-ready source Page. Only that explicit replacement yields the pinned
execution-context-destroyed message. Missing-world gaps, crashed/closed Pages
and absent Frames are not inferred as navigation. Cleanup failures are never
reclassified by this check. This is public error classification, not proof of
native object release.

A final review identified a separate error-precedence edge: the shared
publication helper can replace an already-failed command with a later fence
failure. OwnerFrame now captures the structured Cleanup kind before converting
to HostError, still attempts the connected publication fence, and preserves
the original native compensation failure if that fence fails. This narrow
helper does not change other commands' publication policy. Ordinary successful
reads still report failed publication, and ordinary TargetClosed does not
reenter a full publication queue. Five deterministic tests cover these paths.

### Retained upload is a separate, still-open vertical slice

Pinned Playwright 1.58.2 first awaits ownerFrame and rejects a null owner before
reading payload or options getters. File conversion uses the current owner's
Context, while the server still evaluates against the retained Handle and its
creation Frame's utility world. Current ownerFrame is preflight information,
not permission to change the receiver or requery a selector.

The next upload design must provide these boundaries together:

1. Pre-admit a bounded, single-use preparation ticket before async owner lookup;
   preserve failure precedence before user Buffer/options getters.
2. Resume conversion on the Node thread through an owned rendezvous. Complete
   all getters and language type callbacks before borrowing Buffer backing;
   no raw pointer or callback-scoped handle crosses an await. Revalidate after
   reentrant getters and synchronously copy into admitted Rust-owned sources.
3. Adopt the exact retained receiver into an independently owned temporary
   group, admitted before DOM.resolveNode. An Arc of PendingHandle or a bare
   PayloadTarget is not independent native ownership. Do not put the persistent
   source into the group released by upload. Expose coherent utility numeric
   and unique context identity if that adoption seam needs both.
4. Produce an actual OwnedElement through a retained-reference adoption entry
   point, without fabricating a selector. Reuse the existing single-mutation
   upload continuation and retain source/group/route admission through native
   settlement and cleanup. Avoid any late publication dependent on an already
   closed Handle-release coordinator during shutdown.
5. Add explicit combined handles+file-input artifact/dependency proof, Page/Frame
   `$` and Locator.elementHandle producers, then pass the unchanged eighteen-case
   retained-upload lane plus getter-order, reentry, Worker, OOPIF and gated
   dispose/close/cancellation cases. Directory/native-path limitations remain
   separately declared.

The existing Locator ingress cannot be copied unchanged: it reads options and
payload fields in a different observable order. Upstream payload conversion
performs its buffer-size pass before channel name/MIME/buffer validation; option
spread also invokes unknown enumerable getters. Existing adoption cleanup's
abandoned Err-to-success branch and ordinary timed acquisition need repair or
a stronger independently owned group before upload reuse. Neither this design
nor ownerFrame is claimed as completed ElementHandle.setInputFiles support.

### Verification status

Work evidence is in `.work/node167-owner-frame/`. The initial three-case run
against the unchanged NODE-166 native artifact fails all three with the missing
handleOwnerFrame ABI; this is a wiring red baseline, not a semantic comparison
against a built ownerFrame implementation. Direct raw DOM description tests
pass both exact request/metadata and ordinary-deadline cases.

The pre-review Handles artifact
`1c49f57bc3b616419bf3aa0a29af0c187033138a19ec0b551d143b83b1b1297d`
passes the traced paired Playwright 1.58.2 ownerFrame lane **19/19** with no
skips. This includes a verified sandbox OOPIF target, navigation versus close,
same-document navigation, descendants after source disposal, owner adoption,
orphan/detached documents, and eight reads while all 64 persistent Handle slots
are occupied. Each candidate case explicitly checks zero host/FFI ownership
after cleanup with wrappers still strongly reachable, without requesting GC.
The exact artifact is retained under `verified-pre-review/`; this pass predates
the final cleanup-versus-publication precedence patch above.

The untraced repeat must **not** be reported as green: the first nine cases
pass, the tenth times out, and subsequent upstream connectOverCDP calls cannot
retrieve the endpoint URL. Independent curl and chrome-cdp status checks confirm
that shared Chrome PID 1654 still listens on loopback 9222 but is unresponsive.
The lane supervisor terminates the remaining test process; the sequential
properties/producers/handles/acquisition/module regression commands therefore
never start. Their older checkpoint results are not refreshed evidence.

A one-second native sample observes Chrome's main thread waiting inside macOS
LocalAuthentication `canEvaluatePolicy`, with the dispatch thread soft limit
reached in every sample. This locates the observed stall but does not establish
its initiating cause. vmmap reports physical footprint 652.9 MiB (peak 655.2),
not its 99.4 GiB virtual mapping; ps RSS is 833664 KiB. Swap is zero and the
system memory-pressure query reports 89% free. Shared Chrome was not restarted;
recovery and final browser reruns require explicit restart approval or recovery
of the existing endpoint.

Final post-review Handles artifact:
`0d45e693b6353c2c9ddb6118f1d4b0038febc1a2f36fd616c08525aae01d30bd`.
The release build completes; this artifact has **not** passed the browser lane
because the shared endpoint remains unavailable. Completed gates:

| Gate | Result and evidence |
| --- | --- |
| CDP raw object-ID description | 2 passed; `cdp-description.log` |
| Handle evaluation backend | 80 passed, 3 ignored; `backend-final.log` |
| Handle evaluation composition leaf | 57 passed, 1 ignored; `leaf.log` |
| Final selected Rust host | 489 passed, 14 ignored; `host-cleanup-final.log` |
| Final package contracts | 132 passed; `package-contracts-cleanup-final.log` |
| Final TypeScript/dependency boundaries | Pass; `types-cleanup-final.log`, `boundary-cleanup-final.log` |
| Final selected host/addon strict Clippy | Pass, all targets, warnings denied; `clippy-cleanup-final.log` |
| Backend/default addon strict Clippy | Pass; `clippy-backend.log`, `clippy-default-final.log` |
| Final all-features addon check | Pass; `check-all-features-cleanup-final.log` |
| Formatting and whitespace | `cargo fmt --all -- --check` and `git diff --check` pass |

Resource handoff: the task-owned serve-lib daemon PID 44957 and its explicit
9392 `/pwright-node167` registration were stopped; the test supervisor and all
task-owned builds exited. No new task-owned background process is left running.
Shared Chrome was left untouched. Its unresponsive endpoint prevents verifying
or closing the preflight target `9D50DF6B6054F8BFFA524BE88C24BA5B` and any
resources left by the aborted case. Their cleanup is explicitly pending, not
inferred from the earlier lane's zero-ledger assertions.

At the final measurement, ps reports shared ChatGPT PID 12352 RSS 398464 KiB,
backend 12747 RSS 283520 KiB, renderer 12750 RSS 623008 KiB, and dedicated shared
Chrome 1654 RSS 833648 KiB; swap remains zero. Relative to 07:14:01Z these are
approximately +2.9, +10.3, +17.5 and +78.9 MiB respectively. These are process
RSS deltas, not unique RAM totals or per-chat attribution. An independently
running build under `projects/xlqy-mobile` belongs to other work and was not
interrupted or included in this task's process ownership.

The full compatibility goal remains active. Historical NODE-150 full-corpus
53/57 and NODE-151 retained-upload 0/18 candidate results remain historical,
not refreshed scores. The following NODE-166 and earlier sections are dated
checkpoints rather than statements that ownerFrame is still unimplemented.

## NODE-166: owned property batches across the Rust and Node boundary

The opt-in Handles profile now implements `JSHandle.getProperties()` as one
Rust-owned command returning an ordered `Map<string, JSHandle>`. DOM values
use the existing ElementHandle wrapper. JavaScript only publishes language
wrappers; snapshot semantics, native identities, admission, publication and
disposal remain in Rust. No package, feature, browser launcher, driver process
or portable protocol version is added. Verification is recorded below;
this checkpoint is not completion of the full compatibility goal.

### Admit the entire result before browser work

One batch shares one of the existing eight call allowances and pre-admits all
currently available slots in the existing 64-result owner limit. Each sibling
receives its own runtime ObjectId and revocable cleanup ticket; CallId is no
longer a result ownership key because one call can now return many siblings.
The active source also consumes an owner, so a normal batch can return at most
63 children, fewer while other descriptors are retained. This is an explicit
selected-profile limit, not an upstream Playwright limit.

The host reserves the started-call continuation, one whole-batch compensation
job and every possible child ticket before Frame snapshot or native I/O. The
staged leaf reserves persistent routes and IDs before enumeration and only
trims downward after learning the actual result count. A primitive may return
an empty batch with no result slots, but still owns its call/work lifetime.
The legacy portable batch method is not claimed to have this new pre-I/O
admission contract.

The selected cleanup executor grows from 104 to 112 slots: 32 ordinary, 64
child/result, eight started-call continuations and eight unpublished-batch
compensations. A fixed eight-entry batch inbox, plus detached driver storage,
is explicitly charged at extension scope 166/1. It shares the existing call
limit with scalar creation; it adds no worker or unbounded queue. Named results
retain the shared call allowance until names are consumed at the language
boundary. Per-child owner allowances survive independently through disposal.

### Preserve native owners until settlement or publication

The CDP boundary gains an explicit raw `Runtime.getProperties` settlement
method; ordinary timed calls and exception conversion remain unchanged.
Carrier acquisition, raw property read, carrier release and failure cleanup
retain the same evaluation lease. Temporary native admission includes one
carrier and up to N+1 overflow-detection values, distinct from N persistent
result slots. Before parser copies, bounded-depth structural accounting checks
String/Vec capacities, sparse map nodes and simultaneous staging against that
same allowance. These are host bounds, not a browser heap/RSS claim: the
browser snapshot still materializes `Object.keys`, and decoded malformed CDP
responses remain subject to the transport's own limits.

Failure cleanup borrows and deduplicates every addressable descriptor,
accessor, symbol, internal/private and exception ID, attempts all releases,
and preserves bounded diagnostics. A repeated ID, including an alias of the
carrier ID, is rejected rather than published after its native owner was
released. Missing, invalid or oversized IDs still require a future pre-owned
object-group design and report honest Cleanup failure.

Local batch publication is atomic and revalidates source/world identity. A
busy lifecycle transaction parks the same owner without repeating browser I/O.
Receiver loss, rejected publication and owner shutdown compensate the entire
unpublished batch. After commit, every sibling is retained or submitted for
cleanup even if the first retain fails. No cleanup path relies on JS GC.

N-API conversion defines own data properties on result rows and the result
array. A recovery owner tombstones every already-wrapped NativeHandle if a
later conversion fails; unvisited Rust owners drop into their existing cleanup
tickets. JS Map/wrapper failure disposes all returned native children in order,
attempts subsequent children after an error, and reports primary plus cleanup
failures. Source review confirms these conversion ownership paths; it is not
fault-injection proof of every Node-API allocation/status failure.

### Exact Page-close native receipt

Confirmed Page target absence now marks the retained native-world receipt
inside the same prepared route/topology close transaction, matching the
existing Context-close rule. A command ACK alone, failed close or still-present
target never promotes the receipt; another Page's receipt remains untouched.
This fixes retained Handles incorrectly awaiting an inspector session already
destroyed by confirmed Page.close, without treating generic detach or a stale
snapshot as proof. The conservative acquisition-compensation-versus-detach
residual from NODE-165 remains separate.

### Verification status

Evidence directory: `.work/node166-handle-properties/`. The Page-receipt red
run has two passes and four semantic failures (after correcting fixture
imports). The first host run has 472 passes, one assertion failure and 14
ignored tests: the owner-close test expected a top-level TargetClosed whereas
the existing scalar boundary wraps Backend with typed HostError::TargetClosed.
Its assertion was corrected to require both existing layers, not a production
error suppression. The first leaf test build also exposed a private shared
fixture helper; widening that helper only to its parent test module fixed the
compile error without changing production behavior.

| Gate | Verified result |
| --- | --- |
| Selected host unit tests | 473 passed, 14 live/diagnostic tests ignored |
| Exact Page-close receipt | 6 passed after the four-failure semantic red |
| Raw CDP retained result/settlement | 11 passed |
| Handle backend unit tests | 65 passed, 3 live tests ignored in the ordinary run |
| Backend live acquisition tests, explicitly selected | All 3 passed on dedicated Chrome |
| Downward lifecycle batch trim | 4 passed |
| Native-creation leaf integration | 57 passed, 1 live test ignored |
| Default leaf integration | 32 passed |
| Native addon unit tests | 4 passed |
| Package contracts / TypeScript | 124 passed / passed |
| Package and native-creation dependency boundaries | Passed; no new package or transitive feature |
| Strict Clippy | Addon selected/default/all-features and affected host/CDP/backend/leaf test targets passed |

The first Handles artifact is SHA-256
`2aef5886f672972ef18f88ccc2b44c401dab8dea4df9e282012b78d5a4ff6894`;
its independent same-build copy is retained under this checkpoint's evidence
directory. The existing NODE-161 peer remains unchanged. On that artifact,
producer comparison passes 21/21 with phase tracing and again without tracing;
the five formerly absent getProperties cases now match pinned Playwright
1.58.2. Existing Handles pass 30/30, acquisition passes 7/7, wrapper-module
isolation passes 1/1 and the eight-class native isolation lane passes 3/3.
The new property lane passes 8/8 both traced and plain: ordered/independent
results, detached DOM identity, alias lifetime, Page-close disposal, poisoned
Object/Array prototype setters, full 63-child capacity reuse and live partial
Map-publication rollback. The latter retains all raw children, including the
unvisited child, and verifies disposed state and restored live ownership with
no GC request; fallback cleanup runs only after the verdict.

Do not discard the initial untraced producer failure: it reports 20 passes and
one cancelled first evaluateHandle case after the 25-second case timeout.
The cause remains unclassified, like NODE-165's first-run anomaly. Later
traced/plain passes do not prove that timeout solved or a cold-start cause.
The final post-format rebuild is byte-for-byte identical (same SHA-256 above),
with another independent copy under `final-same-build/`; its focused property
lane passes 8/8 again. Formatting and whitespace checks pass. The only temporary
Page created for this checkpoint's Pwright preflight,
`8AFEAF9D1D62B128E504D755ABCC68B2`, was closed through its task-local Pwright
state; the subsequent target inventory contains zero Page targets. No new
server or Chrome process was started, and no existing peer artifact was
overwritten. The final host rerun also passes 473 tests with 14 ignored. At
handoff no checkpoint-owned compiler, test runner, worker or server remains
running. The original foundation commit remains `f76e3f2`; this checkpoint
creates no additional commit and performs no push.

Memory snapshots are process observations, not per-chat attribution. At
07:04:08Z, ChatGPT PID 12352 RSS is 387,968 KiB, shared backend PID 12747
267,952 KiB and reused dedicated Chrome PID 1654 752,752 KiB; swap remains zero.
Compared with the 06:43:35Z snapshot, app/backend RSS is lower and Chrome RSS
is 170,816 KiB higher after the browser batches. The transient Rust compiler
peaked near 7.6 GiB RSS during the serial composition test build, then exited.
At 07:04:21Z vmmap reports Chrome physical footprint 574.6M, peak 603.2M,
but warns its PartitionAlloc-zone analysis is incomplete. Do not sum RSS and
footprint, infer a leak from this pair of snapshots, or attribute the shared
app/backend to this chat. Zero Page targets and released Handle ledgers do
not imply a zero-RAM Chrome process. The shared Chrome remains running;
no browser/app/service was restarted to manufacture a lower measurement.

Remaining broader work includes preview/toString, ownerFrame/adoption,
retained ElementHandle actions/uploads, expanded fault/concurrency coverage
and the complete shared discrepancy corpus. Historical NODE-150 remains
53/57 matching; this checkpoint has not rerun or cleared those four rows.

## NODE-165: retained acquisition owns failure cleanup and shutdown settlement

This checkpoint addresses NODE-164's raw acquisition gap for addressable
RemoteObject IDs. It does not complete Node getProperties, the property
pipeline's settlement/admission contract, or the entire compatibility goal.
No package, feature, portable schema or public Node API is added.

### Keep raw results until their ownership is decided

Three explicit CdpClient/CdpSession methods select retained evaluation,
unique-world retained function calls and release-to-settlement. Acquisition
returns the raw response, including exceptionDetails, and uses the existing
untimed send_to_settlement path. Ordinary evaluation, exception conversion,
release methods and their deadlines remain unchanged. Custom CdpClient
implementations fail closed unless they implement the new contract; silently
falling back to timed ordinary methods would discard late native owners.

The acquisition helper borrows at most two IDs from the original response
(result and exception). Successful parsing transfers only the result owner.
Failure releases every distinct addressable ID once, including when the first
release fails. Both raw response and the caller's existing evaluation lease
remain alive through settlement and compensation. No native-ID clone, new
queue, detached task or destructor I/O is introduced. The shared parser stays
pure because property/adoption callers already own its failure cleanup.

An empty, non-string, oversized or missing required objectId cannot be safely
addressed by this bounded path. It reports Cleanup/unresolved ownership, never
claims that dropping the JSON released the browser object. A pre-owned object
group is still a separate design requirement for those cases. Error formatting
streams through a UTF-8 bounded writer, attempts all releases even after its
diagnostic capacity is exhausted, and drops redundant primary detail copies
before allocating final Cleanup fields. Normal exception detail text retains
the prior formatting; the outer prefix-inclusive message is now bounded too.

### Untimed acquisition requires native retirement before drain

The first NODE-165 addon exposed an inherited shutdown-order problem: host
shutdown waited for Handle creation continuations before detaching the owned
inspector sessions that could settle them. A never-resolving browser Promise
therefore exhausted the existing five-second cleanup deadline. The paired
lane reproduced six passing cases and one Browser.close failure; the focused
red run reports `browser.close: native command cancellation cleanup failed`.
It is not evidence that the external Chrome process should be terminated.

Handle-enabled shutdown now seals admissions, stops/drains caller operations,
revokes Handle publication and retires only this connection's owned inspector
sessions before waiting for Handle work. Native retirement is serialized with
topology mutation. Detach failure preserves its error and requests transport
disconnect before draining untimed operations. The non-Handle order remains
unchanged. Ready Handle releases still need both the exact native retirement
receipt and local object retirement; generic TargetClosed is not proof.

One real host regression passes with an observer-owned borrowed Page, a Ready
Handle and a second never-resolving acquisition: close settles without cleanup
expiry, the acquisition rejects, the Page survives, and only the externally
retained descriptor/table leases remain until that descriptor is dropped.
The new paired Node fixture independently observes the exact task-owned
target/Context. A pending expression is resolved only for failure cleanup,
after the verdict/deadline, never to turn a hanging close into a passing test.

### Verified evidence and remaining boundaries

Evidence is under .work/node165-handle-acquisition/. The initial addon (before
the shutdown reorder) has SHA-256
25efe39ee343487f7edf19e85d5b85b7996fac6e70fd82ac750212e88dfc22bc.
acquisition-before-host-fix.log records six pass/one fail/zero cancelled;
browser-close-before-host-fix.log retains the exact cleanup failure. The
initial producer lane remains 16 pass/five fail/zero cancelled, all five for
the still-absent Node getProperties API.

Eight real CDP decoder tests prove raw result/exception preservation after
ordinary timeout, pending-slot retention, delayed release acknowledgement,
transport failure and unchanged ordinary timeout/exception semantics. The
backend's first run passes 50 tests with three live tests ignored;
19 are new service-level acquisition cases. All three ignored tests pass when
explicitly run on dedicated Chrome: exact formerly usable native IDs become
unusable with Chrome's object-not-found protocol error after malformed-result
or thrown-Error compensation; successful acquisition stays usable until its
explicit release. Transport failure alone does not satisfy those assertions.

The selected composition integration suite passes 47 tests with one live test
ignored. Three new cancellation cases retain leases/routes while acquisition
or release is gated, compensate late results, report failed continuation
health on refused cleanup, revalidate replaced worlds and never publish an
abandoned owner. These are not proof that the whole getProperties pipeline
retains the evaluation lease: its ordinary carrier/property phases remain
separately scoped. Raw service futures still require caller-owned completion;
the existing creation/ordered wrapper provides dropped-waiter recovery.

The final addon and its independently copied same-build image both have
SHA-256 ea746b3c91e101a99bbc4036bd2654d58da7b5221f557060691878592d2d3a73.
The preserved NODE-161 and NODE-164 peer images are unchanged. On the final
artifact, acquisition-traced.log and acquisition-untraced-{1,2,3}.log each
pass all seven pairs with zero cancellations and no relaxed deadlines. These
cover pending Page/Handle creation interrupted by owner close, Error/primitive/
object throws, later Frame usability, independent owned-Context recovery and
zero host/FFI accounting while wrappers remain alive. They compare rejection
classification/name and recovery, not exact upstream stack/message text.

The first post-rebuild run is NOT discarded: acquisition-final.log records one
pass, five failures and one cancellation. The first Page.close pair exceeded
its 25-second test deadline, followed by five AggregateError cleanup failures
whose nested details were not then emitted. Its cause remains unclassified;
the later passing runs do not establish a cold-start explanation or prove
absence of a race. Diagnostic-only UTC phase and bounded nested error-tree
output is now available through PWRIGHT_HANDLE_ACQUISITION_TRACE=1, without
changing API deadlines/assertions. A tree is limited to depth three/eight
nodes, six KiB serialized text and eight KiB per complete record. Preserve this
failure as a reliability follow-up rather than relabeling the entire lane
unconditionally green.

Other final-artifact gates pass 30 Handle pairs, one public-wrapper module
case and three raw-class module isolation cases. producers-final.log remains
16 pass/five fail/zero cancelled, each failure explicitly the missing
getProperties method. The 110 package contracts, TypeScript check and exact
source/dependency boundary checks pass. The final backend run passes 55 tests
(three live tests ignored): five additional tiny-limit tests exercise caps
0, 1, 4 and 8 bytes, retaining typed failure/attempt-all behavior even with no
diagnostic room. This bounds new acquisition-specific exception/identity
messages, not every inherited parser or transport error in the library.
Selected composition checks pass four unit tests plus 47 integration tests
(one live test ignored); selected host passes 451 with 14 live tests ignored,
and selected addon passes four. Strict selected/default/all-feature Clippy
passes with no warning suppression. The final separate host shutdown regression
passes again (one test), and all three backend native-identity live checks pass
again. Formatting, JS syntax and final diff whitespace checks pass. Ignored
generic-run entries are not counted as executed live tests.

All task-owned build/test processes have finished; no fixture server was
started. The exact preflight target 02F7E72ED0DFA80E8CEA5D6DC8B345CA was closed
through its local Pwright tab ID, and final CDP inventory contains no Pages.
The shared dedicated Chrome remains running, without restart. At 06:20:44Z,
verified app PID 12352 RSS is 634,736 KiB, shared backend PID 12747 is
536,128 KiB (+400 since 06:04:52Z), and shared Chrome PID 1654 is 709,232 KiB
(+96,960 over that interval). A subsequent identity-checked renderer PID12750
sample is 928,896 KiB. These are process RSS values, not unique physical RAM
or per-chat memory; shared mappings are not summed. Targeted Chrome vmmap
reports physical footprint 462.7M (peak 501.4M), a distinct metric. Swap is
zero and memory_pressure reports 92% free. Chrome's RSS growth during the
browser-heavy verification is recorded, not asserted to be a proven leak or
attributed to this chat. No shared process or other task's build was stopped.

Remaining boundaries are explicit:

- Runtime.getProperties still has ordinary timed result handling; late
  property results, successful-carrier release and full-pipeline admission
  require their own ownership design.
- The compensation-versus-detach race can conservatively report Cleanup even
  after an acknowledged owned-session detach has retired the objects. Do not
  suppress generic Cleanup using the host's pre-call world snapshot: the
  staging service resolves its actual route later, and failure currently
  carries no proven route identity. A future typed compensation failure must
  retain the actual frozen route/client identity and match its exact native
  receipt before accepting retirement as cleanup, preserving the primary
  error and unrelated cleanup failures. Existing tests intentionally retain
  failed continuation health when compensation cannot prove success.
- A second source-confirmed shutdown gap must be tested before claiming a
  complete destruction fence: ordinary/confirmed Page close proves exact
  target absence but does not promote Destruction::PageTarget in its prepared
  route transaction, unlike Context close. initialize_snapshot drains queued
  events, not a future targetDestroyed event. With a retained world snapshot,
  early session retirement can therefore try detaching an already-destroyed
  session and retain a conservative failure before the event arrives. This
  is not established as the first post-build timeout's cause. Next tests must
  withhold destruction events, gate native confirmation, preserve failure
  when the target remains present, and prove another Page is unaffected.
  Only successful native confirmation may promote the exact Page receipt.
- Node getProperties still needs sibling ticket identity, batch admission,
  atomic route/runtime publication and native/JS Map rollback. Preview,
  adoption, retained ElementHandle actions/uploads, Worker/fault cases and the
  broader differential corpus remain unfinished.

## NODE-164: module-local raw classes and portable property identity

The raw-class audit gap described in NODE-163 is now addressed for all eight
exported classes: Environment, Session, SessionSignal, FrameTree, FrameNode,
ContextSettings, Locator and Handle. The public-wrapper sidecar guard remains
separate. A stable package/version/class string alone cannot prove a Rust
layout belongs to the currently loaded addon.

### One identity boundary for generated native conversions

The addon aliases its own crate as napi and reexports the pinned runtime plus
a local bindgen_prelude. Only type_tag_from_ident changes: the lower word is
the address of a non-zero-sized, non-mergeable AtomicU8 in this loaded image;
the upper word is the pinned per-class hash. No pointer is reconstructed from
a tag or exposed to JavaScript. The real TypeTag trait remains unchanged, so
generated constructors, borrowed receivers, raw class arguments and the custom
Handle publisher all stamp/check the same identity. A Rust test proves every
selected raw class uses the module word and all class words are distinct.

The Cargo dependency key changes to napi_runtime, still package napi 3.12.2
with exactly the same features. No package or feature edge is added. This
removes a competing extern-prelude name: an initial per-module alias passed
unit compilation but failed production-generated registration modules with
E0659. The final crate-wide facade builds the production addon successfully.
An explicit ctor reexport avoids macro-resolution ambiguity; the one ordinary
root Result collection is explicitly std::result::Result.

This is a reviewed generated-code seam, not an advertised napi extension API.
The source-drift gate pins napi 3.12.2, napi-derive 3.6.3 and its 6.1.2 backend,
requires napi8, checks all raw-class declarations and both typed registries,
and verifies validation/error propagation precedes the reviewed casts in
borrowed arguments, receivers, accessors, ClassInstance and Reference paths.
It is not a general control-flow proof. Dependency upgrades must re-audit it.

The raw addon exports nativeClassNames and nativeClassMatches for diagnostics;
the public Playwright-shaped facade does not expose these. The latter accepts
Unknown, bounds the class name to 64 UTF-8 bytes, and compares opaque tags only:
it never unwraps or casts native storage. Each mixed-module live test asserts
foreign-tag rejection before invoking a method with a foreign argument or
receiver. No old addon's method is invoked on a new incompatible object.
Borrowed receiver checks may also be rejected by V8 before Rust, so their
success alone is not evidence of Rust conversion order.

### Portable getProperty follows the same snapshot semantics

The legacy portable HandleGetProperty command now uses the already verified
single-property snapshot backend rather than awaited evaluateHandle. It
reserves a persistent route before native I/O and shares publication with the
ordinary producer, preserving exact source/target revalidation, atomic runtime
and route commit, and rejected-publication compensation. Its result remains
Frame-owned after source Handle disposal. Wire 1.2, schemas and opt-in feature
selection are unchanged. Abandoned creation now reports a backend Cleanup
failure to its continuation instead of silently classifying it as ordinary.

Eight new deterministic tests cover snapshot-only publication, native identity,
independent lifetime, capacity rejection before I/O, world-replacement
compensation, backend shape failure, abandoned delivery and refused cleanup.
They pause the initial carrier request, not every getProperties/release await.
Raw service futures still require caller-owned completion; only the existing
creation/ordered continuation provides dropped-waiter recovery.

### Verification and remaining work

Evidence is under .work/node164-native-class-identity/. The selected handles
artifact and independently copied same-build artifact both have SHA-256
74dcf0e51459bef72cfeef61eedd875cbe294dc13993f65afb6353d9f0e9b909.
The unchanged NODE-161 peer has SHA-256
45d63983b2b08a74cfa8a793d62e425c6b142dae8c8805822f418476df0194b8.

native-class.log passes three bounded cases: current rejects legacy, current
rejects the independent same-build image, and that image rejects current.
All eight brands, primitive/proxy/prototype spoofing, five Handle parameters,
Locator.combine, FrameTree.lookup and eight borrowed receivers are covered.
Same-module positive controls remain usable; rejected calls leave accounting
unchanged, and owner-module cleanup returns host leases and FFI bytes to zero
while wrappers are retained. Binary digests, realpaths and distinct constructor
identities establish the tested module relationship.

The same artifact passes 30 Handle pairs, the public-wrapper module case,
110 package contracts and TypeScript checks. Exact dependency/source boundary
checks pass. The producer lane remains 16 pass / five fail / zero cancelled;
every failure is the absent getProperties method, with none skipped.
The portable real-Chrome regression passes independently through encoded/decoded
1.2 commands and creation_service, without native-creation selected. Both
Promise and thenable properties retain original identity; the then getter is
not read, derived Handles survive target disposal, and all three routes plus
resource/continuation metrics return to their baseline. Its 30-second exercise
has outer Context cleanup on success, panic or timeout; it checks that the
externally owned browser remains alive. Tests run on macOS arm64, Node 26.7.0
and the dedicated Chrome 152.0.7977.83.

Final Rust checks: default leaf 32 pass, selected leaf 48 pass, selected addon
four pass, default addon one pass and selected host 451 pass. Explicit live/OOPIF tests remain ignored
in generic runs (four default leaf, five selected leaf, 14 host); the new
portable live test is executed separately rather than claimed from an ignored
entry. Strict selected, default and all-feature host/addon Clippy passes, with
no warnings suppressed. Formatting and diff whitespace checks pass. Fresh Rust
test executables again showed a verified pre-main startup delay (32 KiB RSS,
zero CPU); their original processes completed without restart.

All task-owned build/test processes have finished. No fixture server was
started this round. The preflight target CAF777B43DFCCADDB8EFD86B2DFB524D was
closed using its Pwright local tab ID; final CDP inventory has no page targets.
The shared dedicated Chrome process remains running and was not restarted.
At 05:46:24Z, verified shared backend PID 12747 RSS is 531,312 KiB
(+2,704 from this turn's first sample), renderer PID 12750 is 926,592 KiB
(+2,032), and shared Chrome PID 1654 is 563,936 KiB. Swap is zero and
memory_pressure reports 92% free. These are RSS observations, not an exact
per-chat physical footprint; shared mappings are not summed as unique RAM.

An independent source audit also found an inherited acquisition failure gap:
handle_evaluation.rs passes a raw RemoteObject into parse_response without an
error cleanup branch. For example, a result containing objectId but no type
is rejected before a native owner exists, so a property carrier can be leaked
before read_snapshot is reached. Overlong IDs require a bounded cleanup policy
as well. This is source-confirmed, not a malformed-response reproduction in
this checkpoint; fix it before claiming comprehensive native compensation.

The next acquisition design must preserve raw retained-result responses before
the CDP helper turns JavaScript exceptions into strings, and must account for
late replies discarded by ordinary command timeouts. Keep the shared parser
pure: property-snapshot and element-adoption callers already own compensation,
so adding release inside parse_response would double-clean those paths.
An explicit retained-result path should capture at most two distinct IDs
(result and exception), retain admission through settlement/cleanup, transfer
only the successful owner and attempt all failure releases once. Empty,
non-string or oversized IDs cannot be silently truncated or claimed reclaimed;
bounded unresolved-cleanup reporting and a separately admitted object-group
design need consideration. These are next-slice proposals, not implemented
transport or cancellation guarantees. Tests must include the real in-memory
CDP decoder: configured fake responses bypass the current exception stripping.

getProperties batch admission/sibling identities/atomic publication, previews,
adoption, retained actions/uploads and Handle Worker/fault/concurrent-disposal
gates remain open. No full shared-corpus score is refreshed. No additional
commit or push is made.

## NODE-163: Handle-targeted producers and failure-safe publication

The selected handles profile adds JSHandle.evaluateHandle (function/string,
including owned nested arguments) and getProperty. Rust owns browser execution,
source-world routing, result staging and cleanup. A derived result belongs to
the source Frame and remains independently owned after its source Handle is
disposed. Publication revalidates both source target and argument Handles,
including an expression that ignores its target. The existing eight-entry
driver inbox, dual recovery reservations and exact native-world witness remain
the publication/abandonment boundary; no package or feature edge is added.

### Storage and publication failures

The first rebuilt artifact rejected existing creation because the composed
future exceeded its former 4 KiB runtime guard (29 failures in handles.log).
The concrete future now has an 8 KiB compile-time assertion. These bytes are
explicitly included in the existing 8 MiB call-allowance proof, with unchanged
eight-call and 64-result limits. No unbudgeted nested box hides the growth.

Review also found that the host dropped its call allowance before its result
receiver resumed, although that receiver could still own an input-reference
vector and creation state. Admission now shares the same allowance through
caller completion, native continuation and publication. A delayed consumer or
cancelled caller cannot leave suspended state uncharged; sharing adds no lease
or capacity slot. Three deterministic tests cover delayed consumption at the
eight-call cap, cancellation with retained continuation and lost result reply.
The outer owned-argument methods have no later suspension after the guarded
inner await, so their vectors are dropped in that same completion poll.

Locked napi-derive-backend 6.1.2 releases its Box before fallible class instance
construction; locked napi 3.12.2 can fail before native wrapping. A local
publication adapter now constructs and tags the class before exposing its Box,
then makes wrapping the final fallible ownership transfer. Failed wrapping
reclaims the allocation. The factory flag is restored by RAII, including nested
factory calls. This class exposes borrowed native methods, not napi-rs
Reference<Class>, so no native reference-map entry is needed.

JS wrapper construction is a distinct publication boundary. Classification,
sidecar binding or marker-definition failure immediately requests native
disposal and waits for its acknowledgement. If cleanup also fails, an
AggregateError preserves both errors. Internal factory callers now await the
wrapper; public APIs remain asynchronous. Three native binding/factory tests
and nine JS publication contract tests cover deterministic failure paths.
They are not live N-API allocation-failure injection or proof for Worker exit.

### Property identity is not evaluation-result awaiting

Pinned Playwright 1.58.2 javascript.js wraps a property in a null-prototype
temporary object, then reads that object's own property descriptors. Thus a
Promise-valued property remains a Promise Handle; neither it nor another
returned object's then getter is inspected by awaiting. The initial Rust
target[name] evaluation violated both behaviors. property-identity-red.log
records two paired identity/then-access failures, alongside the five still
unimplemented getProperties cases (13 pass, seven fail, no cancellations).
The repair must preserve one property result through bounded snapshot/read
cleanup without publishing its temporary wrapper as a portable Handle.

The repaired Rust path creates one null-prototype carrier, extracts exactly one
descriptor with the requested name, and releases the carrier before exposing
the property. Malformed/empty/mismatched responses compensate every returned
remote value plus the carrier. Failed carrier release is always a Cleanup
failure, even when the returned property is primitive; otherwise abandoned
creation could silently classify failed compensation as an ordinary error.
At most one transient carrier and one result coexist per admitted property
call. The 64 limit counts persistent result routes, not all transient native
CDP objects or browser RAM. The existing eight-call allowance bounds these
additional carriers; no extra portable object or queue is installed.

The snapshot uses the requested name, matching the pinned upstream algorithm.
A callable property literally named then can therefore make the carrier itself
thenable. A separate bounded oracle/candidate characterization covers this
edge; ordinary Promise/thenable-valued properties must not be confused with it.

### Direct native addon classes remain an audit gap

The verified cross-module guard covers JS wrapper sidecars used as public
evaluation arguments. It does not cover every raw native class. napi-rs derives
NativeHandle's class tag from package/version/module/class text, which can
remain identical when another compiled addon has a different Rust layout.
Passing that raw class directly into another addon's exported NativeSession
method can pass the derived tag before casting. The existing module-isolation
test does not exercise this route and must not be cited as proof for it.
Do not intentionally pass an incompatible-layout object just to demonstrate a
crash. Design a module-local native-class guard before claiming this boundary
safe; account for NativeSession and other derived native classes too. The
ordinary public facade keeps these raw objects private, while direct mixed-build
native-addon use is outside the presently verified boundary.

### getProperties batch design remains unimplemented

Do not loop the single-result producer: it would execute getters differently,
consume separate calls and lose atomic failure cleanup. One command must admit
up to N result owners and all continuation/name/vector storage before native
I/O. The backend's property ceiling can exceed the Node profile's 64 owners;
source/other live owners reduce the remaining result capacity. Unknown result
count needs a downward-only route-reservation trim and a separate empty case.
The backend's N+1 overflow sentinel and snapshot also need transient ownership.

Current cleanup tickets identify a result by CallId. Batch siblings sharing a
call would collide: introduce a distinct result identity, such as
(CallId, result_index), throughout owner validation, revocation and release,
then add atomic retain_batch. Stage one owned native batch, revalidate exact
target/world and publish all runtime objects/routes at one serialized boundary.
Busy publication defers the entire batch without rerunning JavaScript; rejected
or abandoned publication attempts cleanup of every result despite individual
release failures.

N-API batch conversion and JS Map assembly require explicit rollback guards:
failure after earlier elements converted must synchronously request cleanup of
already-published and still-unconverted owners. Automatic Vec conversion or
eventual GC is insufficient. Keep names/vector allowance alive through both
language-publication phases. Characterize Proxy/symbol/modified-intrinsic
behavior against upstream before treating the existing Object.keys snapshot
algorithm as fully equivalent to Runtime.getProperties.

### Verification checkpoint

Evidence is under .work/node163-handle-producers/. The producer lane contains
21 pairs and does not skip the five missing getProperties cases. A first
18-case run timed out in its first pair; phase diagnostics were added without
raising deadlines. The traced 20-case red rerun did not reproduce the timeout;
the original timeout's phase/cause remains unattributed.

Before the property identity repair, the selected artifact
aea5158949df8febd4864fed27a6c9351f445aca92d9c0decab45e271d3c9433 passed the
existing 30-case Handle lane, module-isolation case, 38 ordinary evaluation
pairs and 110 package contracts. Types and exact dependency boundaries passed.
These are intermediate evidence, not the final repaired-artifact gate.

Final rebuilt handles artifact SHA-256:
f2edecc2fafd3680ef6195c7d15f899ce5b0d3d384ef929b130e1186501d1b8d.
producers-repaired.log reports 16 pass / five fail / zero cancelled out of
21 pairs. All five failures are missing getProperties, not hidden or skipped;
Promise identity, then-getter reads and the literal-then edge now match upstream.
On that same artifact, handles-repaired.log passes 30/30,
module-repaired.log 1/1 using the independently loaded NODE-161 peer, and
evaluation-repaired.log 38/38. Package contracts pass 110, TypeScript passes
(including a Promise-preserving getProperty type) and dependency closure checks
pass. Production JS still has no browser driver, CDP transport or process
launcher imports.

Final Rust checks: backend 31 pass; selected Handle leaf 40 pass / five explicit
live/OOPIF tests ignored; selected host 451 pass / 14 ignored; addon three pass.
Strict Clippy passes for the selected backend/leaf/host/addon and default
host/addon. The earlier default host pass was 421 with eight ignored.
Final all-feature host tests pass 515 with 24 ignored. Strict all-feature
addon/host Clippy also passes. Its freshly generated build script had a
verified pre-main macOS startup delay; the original process eventually
completed without restart. Formatting and diff whitespace checks pass.
No full shared-corpus score is refreshed. All task-owned Cargo/test processes
have finished.

The exact preflight target C9DFB3F2340808C5B32194579246A387 and
/pwright-node163 registration are removed. Browser inventory reports no tabs.
The temporary task-owned serve-lib daemon (PID 19969; last RSS 5,296 KiB) was
stopped only after its mount list was empty; it was not a pre-existing service.
No other task's server or the shared dedicated Chrome process was stopped.

At 05:18:52Z the verified shared backend PID 12747 RSS is 526,064 KiB and
renderer PID 12750 RSS is 923,376 KiB, compared with 516,048/962,176 KiB at
04:54:47Z. Swap remains zero; memory_pressure reports 92% free. These shared
process measurements are not this chat's exact RAM, and are not summed as
unique physical memory. The stopped fixture daemon leaves no listener on
7878/9386. No recurring monitoring automation was created.

getProperties, previews/toString, ownerFrame/adoption, retained ElementHandle
actions/uploads and Handle-specific Worker/fault/concurrent-disposal coverage
remain open. The unchanged retained-upload lane and last full shared corpus
remain non-green. No additional commit or push has been made.

## NODE-162: native expression programs and global evaluation scope

The selected handles artifact now supports string-form JSHandle.evaluate and
string Frame/Page evaluation with owned Handle arguments. Expressions return
their result without invoking a returned function, and await Promise results.
Arguments still pass through ordinary getters, deferred binary conversion,
bounded graph validation and native reference/world checks before execution.
Ignoring an argument value is not permission to ignore its lifetime or ownership.

### Native program versus portable projection

CdpHandleEvaluationProgram owns the original source, argument graph and actual
execution mode. Its portable-shaped projection serves source/graph validation
and admission only. A native expression with arguments uses the function-shaped
projection to retain the complete graph without changing any source bytes,
wrapping user code or adding synthetic graph nesting. The CDP invocation selects
the program's actual mode. CdpHandleEvaluationRequest.params() remains the
backward-compatible validation projection; program() exposes execution semantics.
This native type has no wire codec. Existing EvaluateHandleParams,
HandleEvaluateParams and Handle evaluation 1.2 codecs remain unchanged.

The existing native-creation and frame-value entrypoints now accept native
programs through added methods, preserving their old portable entrypoints.
Independent default-off handle-value adds the Handle-targeted native value
entrypoint. It reuses exact target/reference resolution, adoption ownership
and the value backend, with no temporary result owner. The host selects this
explicitly: five lower-level feature differences, no new package dependency.
Existing call, source, graph, FFI, result and cleanup allowances are unchanged.

### Two observed semantic fixes

The Handle creation runtime used direct eval inside its strict helper function.
Paired tests proved that user expressions could see internal c/handles/decode/
argument bindings, functions inherited the helper's strict scope, and var
declarations did not persist globally. Both expression and function creation
now use indirect globalThis.eval in the selected execution world, consistent
with the existing by-value runtime and the pinned upstream utility script.

A disposed Handle target used to fail before reading its argument. The pinned
Playwright 1.58.2 client serializes that argument first. NativeHandle now captures
the target as either an owned live descriptor or HostError after ordinary
conversion; the existing EvaluationCall carries this result through deferred
binary materialization before returning the target error. No disposed descriptor
is retained and no new queue or task is introduced. This fixes the verified
getter ordering without claiming complete concurrent-disposal/error precedence.

### Evidence and remaining work

Evidence directory: .work/node162-handle-expressions/. The initial red log has
six failures on the original artifact. First-built.log isolates the remaining
disposed-target phase mismatch: upstream read the getter once, candidate zero.
The final handles.log passes 30 tests: 29 paired workloads plus the native-class
prototype guard. This includes full 32,768-byte source and 4,096-node argument
limits together, proving the native path does not spend user limits on wrappers.
Backend tests pass 21, including four new native-program/invocation tests.
Node contracts pass 101 and TypeScript plus exact dependency checks pass.

The final selected artifact SHA-256 is
172fd91b793b1976bef7b124695ea5a1efe08f36ea8578a5b7f523fccde10f90.
An intermediate phase-red.log ran before the rebuilt binary was published and
used the new facade with the old ABI; it is retained but excluded from semantic
evidence. A unit-test draft used a nonexistent Default constructor and omitted
an import; both compile errors were fixed before the final backend test pass.
Final ordinary evaluation passes 38/38 on this artifact. The module-isolation
test also passes using the independently loaded NODE-161 addon as its peer
(SHA-256 45d63983b2b08a74cfa8a793d62e425c6b142dae8c8805822f418476df0194b8),
so the guard rejects a different build before reading its sidecar layout.
Ordinary Rust host tests pass 448 (14 ignored); combined host tests pass 512
(24 ignored). Portable codec tests pass nine, including the 1.2 command family.
Strict Clippy passes for selected/default host and addon, combined host,
backend, independent handle-value, and frame-value/native-creation. Formatting
and diff whitespace checks pass. CI changes remain local and were not pushed.

The /pwright-node162 registration and exact preflight target
16E492C6FB8D9B2947BE87660B625240 are removed. All task-owned build/test processes
are finished; no new background server remains. The pre-existing shared
serve-lib daemon stays running (PID 54590, RSS 7,360 KiB). At 04:27:53Z the shared
backend/renderer RSS values are 3,608,688/1,379,328 KiB, versus
2,265,296/1,455,664 KiB at turn entry and 810,880/1,349,056 KiB at 04:25:53Z.
Swap remains 111.44 MiB; the last system query reports 66% free. These shared
process fluctuations do not identify this chat's exact RAM or prove a leak.
A subsequent targeted vmmap reports 2.3 GiB physical footprint (13.6 GiB
process-lifetime peak); it must not be added to RSS or attributed to this chat.
No shared service or other task was stopped.

Handle-targeted evaluateHandle/getProperty/getProperties, toString previews,
ownerFrame/adoption and retained ElementHandle actions/uploads remain open.
Handle-specific Worker/failure/concurrent-disposal gates and the unchanged
18-case retained-upload lane remain separate. The last full shared corpus
remains non-green; none of its previous discrepancy rows is closed by this work.
No new commit or push has been made.

## NODE-161: owned nested Handle arguments

The selected Node artifact now accepts real Handle arguments in function-form
Frame/Page value evaluation, Frame Handle creation and Handle value evaluation.
Nested arrays/objects, repeated native references and cycles retain their graph
identity. TypeScript argument unboxing mirrors those supported shapes without
changing the browser-side target value to a JS wrapper type.

### Native identity and retained ownership

A private module-local 128-bit N-API type tag, installed after successful native
wrapping, identifies a wrapper sidecar. A private non-mergeable AtomicU8 static's
address supplies module identity only, never a portable ID or a pointer to
dereference from JavaScript. A constant shared by multiple loaded addon versions
would incorrectly authorize reading another version's unstable Rust layout.
The sidecar and NativeHandle share one
Arc<Mutex<Live/Disposed>> state; this is not another descriptor lease or a JS
back-reference/cycle. Only a matching native binding can produce an owned
argument reference. Visible ID fields and the legacy Symbol.toStringTag marker
cannot grant native authority. Unsupported/legacy Handle markers reject instead
of being silently serialized as empty objects, including unselected profiles.

Wrapping failure drops the new sidecar box. If type-tag installation fails after
wrapping, remove_wrap returns ownership for cleanup; failed rollback leaves the
box Node-owned for its finalizer and reports the combined error. The pointer is
unwrapped and dereferenced only after checking the native type tag and non-null
result. Finalizers drop Rust state only and perform no Node API calls. Dedicated
N-API failure injection remains a verification gap; source-level ownership and
the public ordinary/explicit-disposal path are covered separately.

The bounded reader emits ordinary Undefined placeholders plus owned Handle slots
at increasing preorder positions. After the existing late binary copier, Rust
validates the ordinary graph, slot count/order and exact placeholder positions,
then substitutes typed Handle leaves. IDs originate only from retained Rust
descriptors; no user-supplied ID creates an owner. A plain Undefined remains a
plain value unless the converter supplied a validated owned slot.

Every argument is checked against the host's owner table before native dispatch.
Owned descriptors stay alive through execution/publication, including across
deferred Node conversion, and disposal revocation is still authoritative. Slot
and Arc-vector backing is below 128 KiB at 4096 input nodes and is included in
the existing 8 MiB FFI peak proof. Native Handle owner/call limits remain 64/8;
the NODE-160 16 KiB per-owner allowance also covers shared-state/sidecar backing.

### Direct Frame values and conversion phase

The independent, default-off frame-value leaf adds evaluate_value_for_frame.
It uses the existing Frame/world resolution, native argument/adoption machinery
and value-returning backend, without reserving or publishing a temporary result
Handle. No extra package, runtime, worker or wire protocol is installed. Exact
host dependency fingerprints now permit only four lower-level features:
frame-value, native-creation, native-release and disposal-reservations.

Pinned Playwright 1.58.2 client source showed that serializing a disposed Handle
does not itself stop traversal of later ordinary getters. The first design's
early error would have been wrong. Conversion now retains a disposed marker,
finishes ordinary getter traversal and deferred binary copying, then rejects
before browser execution. The paired getter-order test verifies this phase.
This does not claim complete target-disposal or concurrent-call ordering parity.

### Verification and remaining scope

Evidence is in .work/node161-handle-arguments/. Argument ownership checks pass
15 selected unit tests, including three new nested-graph, invalid-slot and
descriptor-retention tests. The complete ordinary host passes 448 (14 ignored)
and combined host passes 512 (24 ignored). Node contracts pass 101 and expanded
type checks pass. The final reference/identity Handle lane passes 20 paired
cases and one candidate-only native-class prototype-spoofing guard (21 total).
The full-capacity paired case occupies all 64 Handle slots and still performs
Frame by-value evaluation, proving it needs no temporary result Handle slot.
An additional test loads an independent copy of the final addon and verifies
that the primary module rejects its peer's Handle before reading a sidecar
layout; the peer's Handle remains usable. This is module isolation evidence,
not a claim of testing every cross-version ABI. The native-class guard also
checks that a wrapper cannot be re-prototyped into a napi-rs NativeHandle.
The locked napi-derive-backend 6.1.2 validates its class type tag before casting.

The final selected artifact SHA-256 is
45d63983b2b08a74cfa8a793d62e425c6b142dae8c8805822f418476df0194b8.
The existing combined artifact remains unchanged at
93c0527d02bbcb66d8082e4ddda6f673b1b66528d784e82445e5ec122de2cd6a.
Strict Clippy passes for the final selected host/addon; default host/addon,
combined host and standalone frame-value leaf checks also pass. Dependency
boundary, formatting and whitespace checks pass. Remote CI has not run.

The final module-local-tag artifact also passes the ordinary evaluation lane:
38 tests, zero failures or skips (final-evaluation.log). The independent-module
test passes after its directional title correction (module-isolation.log).
It can be repeated with PWRIGHT_NATIVE_PROFILE=handles, PWRIGHT_TEST_CDP set
to the dedicated loopback endpoint and PWRIGHT_PEER_ADDON set to an absolute
path for a separately copied matching addon, then pnpm test:handle-module.
The test checks distinct NativeHandle constructors before exercising the boundary.

At 04:09:05Z, the shared backend/renderer RSS values were 4,110,256/1,364,048
KiB; the backend had been 1,611,264 KiB at 04:06:54Z. A targeted vmmap at
04:09:24Z reports 2.6 GiB physical footprint (13.6 GiB process-lifetime peak),
and the next RSS sample falls to 2,475,392 KiB. These asynchronous measurements
are not additive and do not identify this chat's exact RAM or establish a leak.
Swap remains 111.44 MiB; the system query reports 66% free. No shared process
was killed or restarted.

The task's /pwright-node161 registration and exact preflight tab are removed.
No task-owned test/build/server process remains. The pre-existing shared
serve-lib daemon remains running (PID 54590, RSS 7,344 KiB). Evidence logs and
the independent copied addon fixture remain on disk; those are not resident
background workloads. Foundation commit f76e3f2 is still local; no new commit
or push was made during NODE-161.

The added foreign-session workload initially failed an incorrectly scoped
fixture assertion: an outer session was still using 32,968 shared FFI bytes.
The red log is retained. The corrected nested fixture returns to its measured
entry baseline, while the outermost fixture still requires zero shared FFI
bytes; per-session host leases must always reach zero. The browser program
does not execute with the foreign Handle.

String expressions carrying Handle arguments are explicitly unsupported; the
current typed expression model does not accept an argument. Cross-Frame adoption
is not silently enabled. Additional Handle/property producers, canonical
previews, ownerFrame, retained ElementHandle actions/uploads, proxy-wrapped
Handle behavior and Handle-specific Worker/failure/concurrent-disposal gates
remain open. The original retained-upload 18-case lane is unchanged and is not
the expanded basic/reference Handle lane. The full shared compatibility gate
and its prior discrepancies are not superseded by these focused passes.

## NODE-160: first public opt-in Handle vertical slice

The independent `handles` Node artifact now exposes Page/Frame.evaluateHandle
(function or expression), JSHandle.jsonValue, function-form JSHandle.evaluate,
explicit dispose, and the JSHandle/ElementHandle asElement distinction.
Creation, retained-object value reads, disposal and lifecycle validation all
remain in Rust. JS contains only wrapping, input-shape validation and delegation;
there is no selector replacement for a retained node.

### Ownership and conversion

The safe host registers a completion sender in the already accepted cleanup
job before returning its PendingHandle. request_dispose submits that ticket
exactly once and returns a completion-only HandleRelease receipt. Concurrent
waiters share the result; dropping a waiter does not cancel native cleanup.
An executor deadline or cancellation drops the sender, which is a failed
acknowledgement rather than a successful release or an endless wait. Native
cleanup failures remain visible to both disposal and session shutdown.

The N-API wrapper owns either an Arc<PendingHandle> or a HandleRelease receipt.
Explicit disposal synchronously accepts cleanup and drops its descriptor before
awaiting completion. In-flight reads may retain their own descriptor borrow.
Already disposed JS wrappers can remain alive without consuming native result
slots or retaining the host owner table. Their foreign-language-owned completion
backing is not included in a claim of zero JS/native heap allocation.

The selected descriptor/cleanup allowance increases from 8 KiB to 16 KiB per
owner, still capped at 64. Eight 8 MiB call allowances are now shared by creation
and retained reads. Source/value limits and the existing controlled object graph,
pending-call table, topology publication fence and cleanup executor are reused.
Foreign-session or already-disposing read targets are rejected before leaf
dispatch. The default profile does not select any of this ownership machinery.

DeferredConversions now discriminates Frame by-value, Frame by-reference and
Handle by-value calls. All three reuse the existing bounded PreparedInput and
late binary materialization; no duplicate serializer or early binary copy is
introduced. Owned PendingHandle results cross FFI settlement, not naked IDs.

### Verification and open work

Evidence directory: `.work/node160-handle-operations/`.
The focused public lane passes nine paired workloads against Playwright 1.58.2
plus one candidate-only unsupported-reference assertion (10 tests total).
It covers primitives, mutation identity, removed-node identity, Frame expression
creation, concurrent/repeated disposal, 70 retained-but-disposed wrappers,
Page retirement, disposal after native session shutdown, and deferred binary
copy timing. Every fixture checks zero service leases after explicit disposal
and shutdown while retaining its JS wrappers.

The ordinary evaluation lane also passes all 38 tests on the new handles
artifact. Node contracts pass 101 and TypeScript checks pass. An initial
evaluation attempt stopped at its fixture-URL precondition before executing
tests; rerun used a verified full-MagicDNS serve-lib route. A new paused-time
unit test initially asserted failed instead of the executor's separate timed_out
metric; its red log is retained and the assertion is corrected without changing
runtime timeout behavior. Final ordinary Rust host tests pass 445 (14 ignored),
and the combined host passes 509 (24 ignored). The six real-Chrome Handle tests
pass in both ordinary and combined hosts. Selected host/addon, default
host/addon, and combined-host strict Clippy checks pass. Dependency boundary,
formatting and diff-whitespace checks pass. The new CI check is only edited
locally; no push or remote CI execution occurred.

Read-only memory snapshots show the shared Codex backend RSS varying from
3,988,688 KiB at 02:55:45Z to 1,739,584 KiB at 03:10:03Z and 3,066,432 KiB at
03:17:16Z. The last targeted vmmap reports 2.9 GiB physical footprint; these
different measurements must not be summed. Swap remains 111.44 MiB and the
03:16:23Z system query reports 64% free. These shared-process measurements do
not identify this chat's exact memory or prove a per-chat leak.
At handoff (03:19:19Z), backend RSS is 2,049,952 KiB and renderer RSS is
1,200,432 KiB; swap is unchanged. The task's temporary fixture registration and
preflight tab are removed. No task-owned background processes remain; the
pre-existing shared serve-lib daemon remains running at 5,344 KiB RSS.

Run the new focused lane from packages/pwright after the required dedicated
Chrome/Pwright preflight:

```sh
PWRIGHT_NATIVE_PROFILE=handles pnpm build
PWRIGHT_NATIVE_PROFILE=handles PWRIGHT_TEST_CDP=http://127.0.0.1:9222 pnpm test:handles
```

The fixed artifact is native/handles/pwright.node; a compiled handlesEnabled
marker must match this selection. Default and other named artifacts reject a
mismatching binary. The new artifact SHA-256 is
95ee64765a8571a81dd1becb56010b7db6e0cad4c7922bf02eae0b0810496b8c.
The previous combined file-input/close/drag/frame/headers artifact remains
unchanged at 93c0527d02bbcb66d8082e4ddda6f673b1b66528d784e82445e5ec122de2cd6a.

This is not the complete Handle API or a green full compatibility gate.
Owned nested Handle arguments, Handle-targeted evaluateHandle/getProperty/
getProperties, string-form Handle evaluation, canonical toString previews,
ownerFrame/adoption and ElementHandle actions/uploads remain unfinished.
Nested wrapper arguments currently reject explicitly in the Rust converter
instead of becoming empty ordinary objects. The original 18-case retained-upload
lane remains unchanged and was not reclassified as passing. Handle-specific
N-API conversion-failure and Worker-termination fault injection remain to be added.
The prior full shared corpus discrepancies are not resolved by this focused lane.

## NODE-159: safe-host staged creation and owned publication queue

The selected safe host now uses `stage_evaluate_handle`, not the raw immediate
creation command. Its eight-call admission remains owned while native creation
and local publication are pending. The caller freezes its existing Frame/world
snapshot; Page and both execution-context identities must match the unpublished
native owner before that witness follows cleanup. No script is retried.

The existing topology driver processes a fixed eight-entry publication inbox at
its local boundary, before native-release commits. Busy commits return the same
native owner to the inbox without self-waking or sending another native command.
The driver revalidates the source through the leaf on each attempt. No extra
task, runtime, browser, current-world registry or unbounded queue is introduced.

Every call reserves two existing cleanup-executor slots before native work:
one for abandoning the creation future, and one for unpublished compensation
or the eventual retained result. The latter moves with the result; it is never
reacquired after native creation. Selected executor capacity is now
`32 + 64 + 8 = 104` slots, preserving ordinary capacity, 64 result owners and
eight in-flight creation continuations. The selected publication owner/dual
inbox backing uses scope 159/1. Existing per-call/result allowances retain
future and cleanup ownership, with the 4096-byte creation-future size check.

There is no bare initializer channel. On successful local commit, the driver
first registers the cleanup ticket with Owners and only then sends a
PendingHandle. Losing an unread reply drops that descriptor and submits its
ticket. Losing a receiver before publication, closing the queue, a late arrival
to a closed queue or a failed enqueue drops a recoverable native envelope:
its already-reserved cleanup job releases the unpublished result. A rejected
publication also submits compensation before reporting its primary error.
Combined primary/compensation failures remain visible; executor failures reach
session shutdown even when no caller remains to receive an error.

Native creation exposes typed compensation separately from the original local
failure. A TargetClosed native release is not ignored: the existing release
coordinator can await the exact native-world retirement witness. An unpublished
result has no runtime object, so only that world proof is required; published
results still require native retirement and removal of their old local object.
Missing evidence and other errors remain failures, with the existing cleanup
deadline authoritative. Closing the installation revokes result owners and
closes the publication queue before draining owned work/native events.

### Verification and remaining public integration

Evidence lives in `.work/node159-host-handle-publication/`. Initial host checks
pass 431 tests and strict Clippy. Seven final queue tests pass for receiver loss
before publication, unread successful replies, busy retry without self-wake,
pending/late queue close, stale-source cleanup before failure delivery, and
cleanup failure reporting, and preservation of both primary and compensation
diagnostics. Final ordinary host checks pass 438 tests (13 ignored), combined
checks pass 502 (23 ignored). The initial six-case queue run passed before the
additional diagnostic test; its earlier count is not the final gate.
Final combined strict Clippy passes. The native-creation/native-release leaf
regression passes 4 unit + 29 integration tests (5 native checks ignored), its
dependency proof passes, and formatting/whitespace checks are clean.

Both ordinary and combined Handles pass five real Chrome tests. The new safe-host scenario
concurrently creates Handles and closes unrelated Pages eight times; the script
counter is exactly eight and cleanup drains. The retained-64 descriptor scenario
still passes: exactly 65 descriptor-storage leases survive the closed host,
then descriptor drop returns them. The queue tests deterministically force busy commit;
the native interleaving test does not claim every close ordering was exercised.

Host/addon dependency matrices now permit native-creation in addition to the
two earlier release features only when host handles is selected. No new package
is added, and existing Node addon profiles still do not select Handles. No addon
artifact is rebuilt and no new Node API parity score is claimed.

CI now selects combined host unit verification as well as ordinary tests and
strict Clippy. This is a local CI configuration change, not a remote CI run.
The test-owned preflight tab was closed after both native suites; no preview
server or task-owned background worker was started.
The shared backend's final 02:48:30 UTC snapshot was about 3.2 GiB RSS, with
swap unchanged at 111.44 MiB. No per-chat RAM attribution or cleanup action is
implied by these shared-process readings.

Next: retained wrapper ownership and public Node producers/operations, including
JSHandle/ElementHandle disposal, arguments, property batches and conversion
abandonment. Extend staged ownership to Handle-targeted/batch producers before
exposing them. The 18-case public Handle discrepancy lane remains a required
gate, not evidence supplied by these internal host tests.

## NODE-158: staged native creation before portable publication

The evaluation leaf now offers optional `native-creation` with
`stage_evaluate_handle`. It shares Frame validation, argument resolution and
adoption with the ordinary command, admits an ID/route, executes native creation
once and returns `PendingNativeHandleCreation`. That owner retains native data,
the frozen execution-world identity and route capacity; it is not yet a portable
object. Its `world_identity` is available before a route has been published.

`commit_or_defer` performs no browser I/O. At a caller-owned local boundary it
revalidates the Frame world and referenced Handles, obtains the short runtime
creation transaction and publishes the route/object atomically. A busy gate
returns `Deferred` with the original native result and reservation. No automatic
loop, sleep or repeated evaluation is introduced. Other local failures return
`Rejected`, which still owns native compensation; explicit `release` preserves
both the primary and any cleanup failure. Unpublished abandonment also requires
explicit asynchronous release; primitives send no native release command.

This is a raw leaf, not an abandonment-safe language API. Callers must pre-admit
retained future/inbox/cleanup ownership before native I/O, keep the creation
future through settlement and keep every returned owner through publish or
release. Merely dropping a native owner performs no browser cleanup. In
particular, a host queue must recover an unpublished result when its receiver
disappears, including cleanup deadline and shutdown paths. Existing route
reservations pay native/preview bounds while unpublished; caller queues/futures
need their own admitted storage. No host worker or scheduler is installed here.

The default raw service still uses its previous immediate publication path.
Only common Frame-creation resolution was extracted into a private helper.
The feature is not selected by the safe host or Node addon; batch properties,
Handle.evaluateHandle/getProperty staging and public Node wrappers remain open.
No addon artifact or Node discrepancy score changes in this checkpoint.

### Verification and next integration

Evidence: `.work/node158-native-handle-creation/`.

- Selected leaf: 4 unit tests and 29 integration tests pass, including nine new
  staged cases. Five native checks are ignored in this ordinary run.
- Combined native-creation/native-release passes the same 4 + 29 checks and
  strict Clippy. The unselected leaf remains 4 + 20 passing tests. Default
  host/addon feature matrices and the locked standalone composition example
  proof pass, without selecting native-creation in any existing host profile.
- New cases cover unpublished graph state, busy retention/no repeated CDP,
  surviving an unrelated committed close, stale-world rejection/cleanup,
  explicit abandonment, route exhaustion before a second script, primitive
  cleanup, terminal transport close and combined publication/cleanup failure.
- The new real Chrome test passes: two local commit attempts defer while an
  unrelated local Page lifecycle transaction is held; after releasing that gate,
  the Handle evaluates correctly and the browser counter is exactly one.
  Explicit disposal returns route capacity; the test-owned Context is removed.
  This uses a deterministic local gate, not a real native Page-close race.
- Strict selected Clippy passes. The new dependency proof confirms no package
  or transitive-feature changes. CI now executes the selected tests, Clippy and
  proof; this is local verification, not a claim that remote CI was run.
- Initial test compile failures were incorrect fixture accessor/value/dispose
  API names; corrected tests use the existing public APIs. The final native
  run has no retries or hidden reruns of the user script.

The test-owned preflight tab was closed and no task-owned server/worker remains.
The shared backend measured about 2.6 GiB RSS at 02:27:16 UTC, with swap unchanged
at 111.44 MiB; this is not a per-chat memory attribution.

Next: reserve bounded host publication and orphan-compensation capacity before
starting native work; process commits in the existing driver; preserve native
world-retirement evidence for failed compensation; drain pending owners during
shutdown. Only then switch safe-host Frame creation to this path, extend other
producers and wire persistent Node Handles plus the 18-case discrepancy lane.

## NODE-157: host release coordinator and parent-retirement evidence

Selected host Handles now use NODE-156 native release. The existing cleanup
executor polls native I/O; a bounded inbox gives completed receipts to the
existing topology driver for short local commits. No new task/runtime/browser
is created. Eight native releases/parked completions share eight semaphore
permits and eight queue-free deferred runtime reservations (256-object,
16-call bounds). Remaining waiters are already bounded by the 64 result owners.
The temporary host-local async cleanup mutex is removed.

The driver commits between topology/explicit lifecycle transactions, outside
the host state lock. A busy receipt parks without self-waking or resending the
native command. Handle creation publishes within a synchronous future poll;
the host signals after that poll, including abandoned-creation continuations,
so its short transaction cannot strand a parked receipt. Native event/explicit
transaction boundaries also provide commit opportunities. Shutdown revokes
Ready-result tickets, continues processing native destruction events and local
release completions, then detaches inspector sessions and drains the existing
executor. Descriptors still do not own that native-work fence.

The first combined native run passed three checks but failed the new mixed
lifecycle test: Context destruction detached the session while eight
`Runtime.releaseObject` calls were in flight. Treating every failed call as a
failed cleanup incorrectly made host close fail. The correction is evidence-
based, not string matching or blanket suppression:

- A creation retains a budgeted snapshot from the existing Frame/world accessor.
  The evaluation leaf exposes the Handle's frozen source-world identity. Page,
  numeric context ID and unique context ID must match the snapshot before its
  native-lifetime receipt can follow the cleanup ticket. A mismatched/missing
  identity grants no parent-retirement proof.
- The evaluation leaf reuses the common typed CDP failure classifier, retaining
  TargetClosed for a detached session. This adds a direct edge to an already
  transitive package, not a new package; root and two affected example lockfiles
  record the edge.
- A TargetClosed release with a matching native-lifetime witness can wait in
  the bounded inbox. Success requires both acknowledged/observed native-world
  retirement and absence of the old local runtime object. Generic errors and
  TargetClosed without evidence remain failures. Merely losing a local route
  or receiving a detach event is insufficient. The existing five-second cleanup
  deadline remains an explicit failure if proof never arrives.

The host adds 64 retained world-snapshot slots only when Handles are selected.
Each uses the accessor's existing 8192-byte allowance plus its bounded native
receipt registry accounting. Release-inbox state is charged at scope 157/1;
runtime reservations at 157/2. The aggregate Handle profile now delegates its
requirements to the selected lifecycle leaf, including 72 bytes per route on
this target for the retained ownership allocation (4608 bytes for 64 routes).
These are logical budgets, not physical RAM measurements.

The in-process concurrent-close profile also stops creating and draining an
unused control outbox: no wire peer consumes it. It now uses queue-free deferred
runtime reservations just like Handle releases. Ordinary and native-close
attachments both retain their 256-object/16-call limits without a control queue;
actual browser events and their separate budgets are unchanged.

### Verification and scope

Evidence lives in `.work/node157-host-handle-release/`. The first combined unit
run passed 493/494; the old profile assertion omitted the newly selected 4608
ownership bytes. Final combined host validation passes 495 unit tests with 22
native tests ignored; ordinary Handles pass 431 with 12 ignored. Five inbox
tests verify eight-call admission, local-only deferred retry without self-waking,
terminal close, generic native failure and TargetClosed without proof. The
evaluation leaf passes 4 unit and 20 integration tests (4 native ignored).
Its first source-world test failed because the fixture rebound a Frame without
unbinding the old route; the corrected fixture verifies that Handle identity
still reports the creation world after replacement, without new CDP calls.

After adding parent-retirement evidence, all four selected native tests pass
in both combined and ordinary Handle builds. The new scenario interleaves
release with an unrelated Page close, world replacement and Context close,
then retains eight descriptors through session shutdown. The existing borrowed-
Page test still retains 64 descriptors across close: exactly 65 storage leases
remain, and descriptor drop returns the final leases. These tests do not assert
every possible native ordering or establish public Node Handle parity.

The final combined native run (`live-latest.log`) again passes 4/4 and records
eight failed native releases subsequently confirmed by both exact native-world
and local-object retirement. This proves the evidence branch ran in that
scenario; it is not merely inferred from a passing close. Strict combined host
Clippy also passes with warnings denied.

Final default-host regression passes 421 unit tests (8 native ignored), with
strict Clippy clean. The final host/addon dependency matrices and locked
Handle-evaluation composition example proof pass. Existing CI workspace tests
cover the source-world regression, and selected host CI covers the inbox tests.

Host/addon boundary matrices allow only native-release and runtime
disposal-reservations as new Handle features; addon Handles remain unselected.
The combined Node artifact was rebuilt because the concurrent-close runtime
profile changed. Its SHA-256 is
`93c0527d02bbcb66d8082e4ddda6f673b1b66528d784e82445e5ec122de2cd6a`.
On this artifact, paired native-close-confirmation passes 6/6 and paired
lifecycle-events passes 7/7 against the pinned Playwright oracle. Node contract
tests pass 100/100 and strict TypeScript validation passes. The full shared
57-case corpus was not rerun; its previous discrepancy score is unchanged.

The task-owned preflight tab was closed after native verification; test-owned
Contexts were disposed and no preview server was started. Shared backend RSS
rose from about 2.2 GiB at 02:09 UTC to 5.4 GiB at 02:13, then returned to about
1.0 GiB at 02:14:34. A targeted snapshot measured 5.0 GiB physical footprint
during the rise, while system swap stayed at 111.44 MiB. These are shared
process observations, not per-chat RAM or proof of a Pwright leak. Other tasks'
processes and the dedicated shared Chrome process were left untouched.

Remaining: raw Handle creation still performs its own short publication and
can be rejected when unrelated legacy lifecycle work owns the global gate.
Wakeup coordination fixes release deferral, not that creation scheduling gap.
Coordinate/stage creation publication, then add persistent wrapper ownership,
N-API producers/operations and reopen the 18-case Handle discrepancy lane.

## NODE-156: optional native release and exact-route commit receipts

`pwright-composition-chromium-handle-lifecycle/native-release` now adds a phased
API on the existing raw disposal service. `prepare_native_release` first checks
that the supplied disposal reservation belongs to this runtime, that transport
is still open and that deferred commit storage was selected. Only then does it
capture the native disposer and exact route resource owner. An already retired
runtime object returns no request, not a claim of native success.

The consuming request's `release` performs only injected native disposal. It
holds neither a runtime lifecycle transaction nor a route mutex across I/O.
Success returns a move-only receipt; primitives without a native object follow
the same local ownership path without sending a command. Native failure leaves
the current route untouched. Dropping an unpolled request sends nothing. This
leaf does **not** install an executor or automatically continue a dropped
written command: callers must supply native-future storage and an admitted
continuation. NODE-154's host result owners remain the intended caller.

The receipt's `commit_or_defer` locks Handle routes, compares their exact
resource-owner allocation, then prepares/commits runtime retirement under the
runtime mutex. A busy lifecycle gate returns the original receipt plus its
unchanged reservation; retry only this short local phase, never native release.
Successful local retirement returns the existing deferred notification owner;
notify only after all outer coordinator locks are released. Non-leaf runtime
subtrees fail explicitly instead of silently retiring unrelated descendants.
Terminal errors after native success require coordinator recovery.

The selected route lease is held in an Arc whose allocation is unique to each
publication, including batch publication. The request/receipt retains that
allocation, preventing address reuse while an old completion exists. Object ID,
payload and disposer reuse cannot make an old receipt match a replacement route.
A missing/replaced route returns `Superseded` without touching the current runtime
object. This is not a second native route registry or a growing tombstone list.

Selected route budgets add `size_of(ResourceLease) + 2 * size_of(usize)` bytes
per route for the retained Arc payload/header. Native-ID/preview bytes and the
RemoteHandle lease remain charged while an old request/receipt survives parent
retirement, even though route metrics already show no current entry. Thus route
counts and the production ledger may intentionally differ. Receipt/continuation
future storage and allocator overhead are separate, not claimed as RSS coverage.
The ordinary feature-off representation and budgets remain unchanged.

### Verification and remaining host work

Evidence: `.work/node156-native-handle-release/`. Fifteen new public-API tests
pass alongside the existing 14 lifecycle tests. Coverage includes native wait
without a gate, native failure, deferred retry with exactly one release,
parent-retired native storage, reused ID with the same payload/disposer,
pre-I/O foreign/incomplete/closed admission, primitive retirement, unpolled
abandonment and unexpected runtime descendants. Default lifecycle tests pass
14/14; combined runtime tests pass 210 plus one doctest.

A real Chrome 152 test passes: the native object is readable before release;
after the release acknowledgement `Runtime.getProperties` rejects that exact
ID while a sibling native object in the same session remains readable and the
local runtime/route are still live. A deliberately occupied local
transaction produces deferral, then local retry retires both owners and returns
all release/RemoteHandle admission. The owned Context is disposed and Chrome
remains alive. The preflight tab is also closed. This is native receipt evidence,
not public Node differential parity.

Selected and default lifecycle all-target strict Clippy pass, as does all-feature
runtime Clippy. Downstream evaluation/ordered-delivery tests pass 4 + 19 + 6;
their four browser tests are not rerun. Workspace formatting and diff checks
pass. No compile/test processes or task-owned browser contexts remain running.

Shared-app memory varied during this turn: backend RSS was 1,660,912 KiB at
01:21:04Z, 1,504,272 KiB at 01:30:44Z, and 4,728,368 KiB near 01:34Z. At
01:35:42Z targeted vmmap reported 4.6 GiB physical footprint with 4.4 GiB in
`MALLOC_LARGE (empty)` regions; system pressure reported 66% free and swap stayed
111.44 MiB. This is a shared-process observation, not exact chat attribution or
proof of a leak. No process was restarted or killed.

The feature adds only the existing runtime `disposal-reservations` selection,
with no new package. CI checks this exact closure and exercises the optional
tests/strict Clippy. Node host/addon matrices explicitly reject selecting this
feature until integration is complete. No Node artifact was rebuilt or API
opened. The old raw disposal path remains for existing callers.

Next: select this capability in the host with explicit release reservation
budgets for ordinary and concurrent-close profiles; park native completions in
the existing driver; commit at its serialized boundaries; continue servicing
pending Handle completions during shutdown before detaching inspector sessions.
Remove the host-local cleanup mutex only after that integration and mixed
Page/Context/topology/late-creation tests pass. Persistent wrapper publication
and the 18-case public Handle parity lane remain required.

## NODE-155: queue-free disposal admission and recoverable commit boundaries

The existing optional runtime `disposal-reservations` feature now supports
`DisposalReservationProfile::new_in_process`. It requires a budgeted runtime
with its control outbox disabled, reserves disposal-order storage and bounded
pending permits, and allocates no control-lease array or EventItem leases.
The old `new` constructor still requires the outbox and preserves its queue
admission semantics. Neither selection enables a transport or executor.

In-process admission can overlap an existing prepared lifecycle transaction:
it reserves storage, not a graph snapshot or exclusive gate. The selected
object bound must still equal the runtime's entire live-object limit. Both
profiles support the existing deferred-publication/notification owner.

The new consuming `prepare_or_defer` returns either a short exclusive prepared
disposal or the original reservation when the lifecycle boundary is busy.
Deferral retains the same storage, pending permit and optional queue admission;
it does not mutate the graph, add a worker/waker, or retry native work. A caller
must park the owner and retry only local preparation at a serialized boundary.
Preparation then reads the latest subtree, including children added since
admission. Foreign-runtime and closed-transport errors remain terminal and
consume admission; closed transport takes precedence over a busy boundary.
The older `prepare` API still consumes admission on every error, including busy.

Logical reservation storage is 256 bytes plus twice the ObjectId size per
configured object; the pool adds 512 bytes. Outbox mode additionally charges a
ResourceLease and ResourceRequest per object. Deferred commits add their existing
peak collection/notification allowance. These are logical capacity charges,
not RSS, native operation storage or allocator-overhead measurements.

This is a runtime prerequisite, **not** host integration. NODE-154's host-local
cleanup mutex and legacy native-await lifecycle transaction remain unchanged.
Next: retain an exact native release/identity receipt, perform native I/O outside
the global lifecycle transaction, and integrate the short latest-state commit
with the shared host driver, Page/Context close and topology publication. Do not
select a control outbox or concurrent-close merely to enable in-process Handles.
Public Node Handle producers remain gated; no addon artifact is rebuilt here.

### Verification

Evidence is retained under `.work/node155-in-process-disposal/`. Nine initial
in-process tests pass. Five additional boundary tests each exercise both queue
modes: retained admission on busy, retry against a new child, dropped deferral,
terminal transport close and foreign-runtime rejection. The first full run
passed 191/192: the failed test incorrectly expected an unrelated prepared
transaction's two EventItem leases to disappear before that owner was dropped.
The corrected test checks those leases remain, then return on owner drop; no
production change was needed for that failure. The first combined-feature run
also exposed a test baseline taken after admitting a pending call: optional
lifetime-handoff storage (152 bytes) correctly disappeared when that call
settled. Measuring the baseline before call admission fixes the assertion in
both profiles. The final all-features gate passes 210 unit tests and one doctest;
the default gate passes 113 tests. The final selected-feature gate passes 192
unit tests plus one doctest. All-target/all-feature runtime Clippy with
`-D warnings`, workspace formatting and `git diff --check` pass. The downstream host combination
`handles,file-input,concurrent-close,extra-headers,drag-runtime,frame-navigation`
passes 490 tests with 21 browser tests ignored. Exact host/addon dependency and
feature matrices pass with unchanged selection boundaries. No browser or server
is started for this runtime change.

Memory checkpoint: shared backend PID 53189 RSS rose from 1,512,304 KiB at
01:13:30Z to 3,454,048 KiB near 01:19Z. Targeted vmmap reported 3.3 GiB physical
footprint, including roughly 3.0 GiB in `MALLOC_LARGE (empty)` regions; these
measurements are not additive or exact per-chat attribution. System pressure
reported 66% free, with swap unchanged at 111.44 MiB. No shared process was
interrupted. At 01:20:23Z the shared backend RSS was 4,453,072 KiB (about
4.25 GiB), so the increase remains unexplained at per-task level despite stable
swap. All task build/test processes have exited. This run creates no long-running
background service.

## NODE-154: session-owned pending results

The selected safe host now has `frame_prepare_handle`, returning a move-only
`PendingHandle`. This is an owned pre-publication result, **not** Node's public
`evaluateHandle`/JSHandle API. It exposes only a borrowed portable initializer;
there is no release/unwrap into an unowned native ID and no JS wrapper yet.

The host-local typed command uses the existing runtime's target/connection
admission and lifecycle graph via `dispatch_with`, not a second runtime. The
existing independent raw Handle service performs native creation and atomic
route publication. Before native work starts, the host admits input/future
storage, result storage and one cleanup reservation. Dropping an unpolled call
does not execute native work; dropping a started call moves its future and all
admission into the existing cleanup executor. Successful Ready results transfer
the same reservation into a bounded session-owned ticket table synchronously.
Dropping the returned result removes/submits its ticket exactly once. A result
that completes after the table closes is rejected and its cleanup is submitted.

Shutdown seals commands, waits for admitted operations to exit, revokes every
Ready-result ticket, and waits for native result work before detaching Locator
inspector sessions or shutting down the cleanup executor. Returned descriptors
do **not** own that work fence or a native service/connection. They can remain
alive after close, retaining only their separately charged portable descriptor
storage, which remains visible in SessionMetrics active leases/scopes after
close. Descriptor storage is reclaimed on descriptor
drop, without delaying native shutdown. No additional long-running worker,
executor, browser, native
route table, topology copy or dependency is selected.

Explicit selected allowances:

- 64 result owners, including creation, Ready values and dropped-result cleanup;
- 8 separately admitted in-flight creation inputs/futures, 8 MiB each;
- 8 KiB per result owner, shared between its descriptor and cleanup lifetime;
- 16 KiB fixed table/accounting state, under resource scope 154/1;
- 96 slots in the **same** cleanup executor (ordinary 32 plus 64 Handle tickets).

The existing five-second cleanup-job deadline is unchanged; cleanup failures
and timeouts remain failures, not native-release evidence. The totals above
are logical resource allowances, not RSS. Ordinary host limits and all Node
artifacts are unchanged. Persistent wrapper delivery/disposal, publication
commit, current ownerFrame/adoption and long-running/concurrent lifecycle
semantics remain required before exposing Node Handle producers.

### Verification and discovered lifecycle constraint

Five deterministic owner tests pass: retained descriptor across shutdown,
drop-Ready exactly-once cleanup, close-before-result compensation, 64-owner
capacity, and independent eight-call capacity. The first native run passes the
two NODE-152 checks but fails the new retained-result shutdown scenario with
`native command cancellation cleanup failed`. `live-diagnostic.log` establishes
the cause: concurrent cleanup calls fail before native release because the
legacy runtime allows only one prepared lifecycle transaction across an await.

One bounded async turn now sequences this installation's Handle cleanup calls.
All waiters already own one of the 64 admitted result allowances; there is no
new worker, timer, retry or unbounded queue. **It does not coordinate with other
Page/Context/topology transactions or late creation publication.** Those mixed
lifecycle cases remain unverified and must be coordinated before opening the
public Node Handle API. The initial failures are retained, not suppressed.

The next run (`live-serialized.log`) finishes cleanup but exposes a test mistake:
it expected all storage leases to vanish while deliberately retaining portable
descriptors. The final assertion checks exact separation instead: after native
close, live objects/calls/drivers/operations/cleanup/event bytes are zero, with
**65 storage leases in one scope** (64 descriptors plus table state). Dropping
the descriptors returns those leases too, producing an entirely empty metrics
snapshot. No production metric was zeroed or filtered to make the test pass.

`live-final.log` passes **3/3** real Chrome tests, including unpolled, started
and Ready-result abandonment; 64 retained results; 65th rejection without
browser counter effects; and shutdown while all descriptors remain held. The
subject host borrows an independently owned Context, and the observer proves
its Page survives host close. `host-tests.log` passes **426 tests, 11 ignored**;
the three Chrome tests above are explicit runs, not implied by that unit gate.
`clippy-final.log` passes strict all-target checks for the full combined host
profile. `boundary.log` passes the exact dependency/feature matrix and
`contracts-final.log` passes **100 Node package contracts**. The initial attempt
used a nonexistent `test:contract` script; the actual package `test` script is
the successful contract gate. No Node differential score or artifact changes.

Final regressions: `default-tests.log` passes **421 tests, 8 ignored** with
handles unselected. `handle-upload-live.log` passes **8/8** existing native host
uploads with handles+file-input, including large payloads and shutdown cleanup.
Formatting and whitespace checks pass. No new commit or push was made, and
all three Node artifact SHA-256 values remain unchanged from NODE-150.

Cleanup: all NODE-154 build/test processes exited. Only this task's preflight
target `865CB94AAB0B33D3794EE44F8124E1B0` was closed; independent target-list
verification finds neither it nor the test Page. No new server remains, and
the shared Chrome process was not restarted. At 00:57:58Z the shared Codex
backend PID 53189 was 2.13 GiB RSS and renderer PID 53192 was 1.14 GiB RSS.
System swap was 111.44 MiB (up from zero earlier), while memory_pressure
reported 64% system-wide free. Other active Cargo/rustc work was verified as
XLQY, not this task, and left untouched. These are process/system observations,
not a per-chat RAM estimate or evidence that this change caused the swap use.

## NODE-153: do not abandon the rest of a cleanup batch

Reviewing the pending-result prerequisite uncovered an existing defect in
`ContinuableHandleEvaluationService`: after a dropped, started `getProperties`
finished creating its results, the first failed disposal returned immediately.
Later results remained registered and unobserved even when their release would
have succeeded. The separate ordered-delivery plugin already attempts every
result; this fix hardens the older in-process continuation, not that plugin.

Cleanup now attempts the entire bounded batch and returns the same failure if
any release failed. A single boolean records failure; no new error collection,
queue, worker, retry policy or dependency is introduced. Successful releases
return runtime objects, routes and RemoteHandle leases. A refused native
release keeps its existing visible route and lease for explicit retry or parent
retirement; this change does not pretend the failed resource was reclaimed.

Four public-service regression scenarios use real creation/lifecycle/runtime
and production-ledger implementations with `FakeCdpClient` fault injection:
first release fails, middle release fails, two releases fail, and all succeed.
They inspect the exact native release attempts, remaining portable identities,
route/RemoteHandle counts, empty continuation queue, and successful explicit
retry of the refused releases. The pre-fix run (`.work/node153-handle-cleanup/red.log`)
has **three failures and one pass**: first failure skipped both later objects;
middle failure skipped the final object. These are deterministic ownership
regressions, not new Chrome or public Node conformance evidence.

Post-fix `green.log` passes **43 tests** across the evaluation composition
(4 unit + 19 integration), lifecycle (14) and ordered delivery (6). Five live
tests remain ignored and were not run at NODE-153. `clippy.log` passes strict
all-target checks for those three crates; workspace formatting and
`git diff --check` pass. The Node artifacts were not rebuilt and no new
browser/server was launched. All NODE-153 build/test processes have exited.

Read-only memory checks: shared Codex backend PID 53189 RSS was 4.44 GiB at
00:21:17Z, 5.40 GiB at 00:27:08Z, then 2.23 GiB at 00:27:45Z. The intervening
vmmap snapshot reported 1.9G physical footprint (historical peak 13.6G); it is
a different metric and timestamp, not additive to RSS. Swap stayed zero and
memory_pressure reported 67% system-wide free. Shared renderer PID 53192 was
1.18 GiB RSS at the final check. No process was killed or restarted, and no
per-chat RAM attribution is established by these shared-process measurements.

Pending-result delivery and retained-wrapper shutdown remain unimplemented.
The remote plugin's Result-send tracker is useful prior art, but its completion
fence is transport delivery, not N-API conversion/publication or environment
close. Do not select a transport merely to borrow that tracker. The next Node
owner must separately cover creation abandonment, Ready-result abandonment,
publication transfer and authoritative retirement while a JS wrapper survives.

## NODE-152: opt-in safe-host native installation

`pwright-node-host/handles` now selects the existing independent lifecycle and
evaluation leaves using `new_controlled_page_runtime_with_handle_leaf`. The
host's live topology remains the only Frame/world source. The feature adds
`HostSession::handle_metrics` for native route accounting, not Handle creation.
No second driver, executor, registry or Node process is introduced.

The explicit native profile admits **64 total Handle routes**, **1024 UTF-8
bytes per native ID**, **1024 preview bytes** and **8 evaluation calls**. Handle
lifecycle/evaluation use separate resource scopes 152/1 and 152/2. The lifecycle
profile declares 64 RemoteHandle units and 128 KiB maximum retained ID/preview
bytes; backend evaluation and wire framing have their own existing plans.
Browser + 16 Contexts + 16 Pages + 64 Frames + 64 Handles totals 161 objects,
within the ordinary 256-object runtime limit. No default runtime limit grows.
These are declared logical allowances, not a physical-RAM measurement or a
budget for future JS wrappers/abandoned result owners. Same-session adoption
is deliberately not selected yet.

The ordinary host still installs neither Handle profile. The Handle packages
already occurred transitively through the aggregate Chromium composition:
selection adds direct optional dependency edges but no additional package or
downstream feature. The boundary checker compares exact dependency/feature
fingerprints for ordinary versus handles, and file-input versus
file-input+handles, and covers the complete combined host selection. It also
asserts that existing Node addon profiles do **not** select handles.

### What remains gated

The Node addon has no handles feature, no artifact was rebuilt, and the Node
package still lacks JSHandle/ElementHandle producers and methods. No raw native
service or disposable handle ID is exposed through the host API. In particular,
the native test below does not establish public HostSession Handle creation,
N-API Create-before-Result, abandoned-result cleanup, retained-wrapper shutdown,
current ownerFrame, property-batch or cross-world parity.

Before exposing creation, add an admitted pending-result owner that compensates
both an abandoned started creation and an already-created result abandoned
during JS conversion/publication. Explicitly solve outstanding cleanup
reservations when JS retains wrappers across environment close: merely storing
one Tokio reservation in every wrapper can make shutdown wait for GC. Use the
existing runtime retirement as authority and retire native owners without
waiting for user-held wrapper objects to disappear. Do not import raw services
into a thin JS wrapper to bypass this ownership boundary.

### Native assembly verification

The new ignored host tests exercise (a) actual selected HostSession connect and
shutdown with an empty Handle installation, and (b) the **exact host profile**
using the composition's public raw Handle services. The latter is a native
assembly test, not a public Node test. It creates and reads an object Handle,
explicitly disposes it, fills the 64-route capacity with ElementHandles, checks
overflow has no browser-side counter effect, and closes the Page to return
all routes and RemoteHandle leases. Only its owned Context is closed.

The first native assembly run failed with a zero numeric execution-context ID:
the test had not installed/driven the topology subscriber. The corrected test
installs that existing driver and consumes the same snapshot/publication fence
used by the host after Page creation. This is a test assembly correction, not
a production sleep, retry or relaxation of exact execution-world identity.
The error and corrected runs are retained under `.work/node152-handles/`.

The initial `--locked` check correctly refused the new dependency edges;
`cargo check --release --offline` updated the workspace lock without a fetch.
Subsequent verification uses `--locked`, release artifacts and one build job.

Completed verification under `.work/node152-handles/`:

- `live-final.log`: both dedicated Chrome tests pass, including the 64-element
  capacity/counter gate and Page retirement. `live.log` retains the initial
  missing-driver failure; `live-second.log` is the earlier smaller corrected run.
- `host-all.log`: handles-only host **421 passed, 10 ignored**; the two new
  native tests are run explicitly above. Other ignored lanes are not implied.
- `default-tests.log`: ordinary host **421 passed, 8 ignored**, including the
  negative assertion that neither Handle profile is installed.
- `handle-upload-live.log`: **8/8** existing actual safe-host upload tests pass
  with **handles,file-input**, including cleanup and the large outgoing path.
  This is not the all-feature concurrent-close native combination.
- `clippy.log` and `combined-clippy.log`: strict all-target checks pass for
  handles-only and handles,file-input,concurrent-close,extra-headers,drag-runtime,
  frame-navigation. `boundary.log` passes every default/selected/combined graph
  check; `contracts.log` retains **100/100** package contracts.

CI now includes the selected host unit and strict single/combined Handle checks;
real Chrome tests remain explicit local lanes. No CI run or push is claimed.
Final profile assertions derive the conservative 161-object sum from the actual
topology fields; worlds are not counted as portable Frames.

The final profile assertion passes after that correction; strict handles+
file-input Clippy, workspace formatting and `git diff --check` also pass
(`final-profile-test.log`, `final-clippy.log`). The preflight target
`059989FBCC1DB24E3CD109F54E271D82` was closed and independently verified absent.
No NODE-152 server remains. All three existing Node artifacts retain their
NODE-150 hashes; the foundation commit remains `f76e3f2` with no push.

## NODE-151 status

Node JSHandle/ElementHandle creation, public lifecycle and upload are still
**unimplemented**. This checkpoint adds an executable public conformance lane
and a reusable Rust payload target boundary, not a selector-backed substitute
for a Handle and not a new public compatibility claim.

At NODE-151 the existing Rust Handle evaluation/lifecycle leaves already implemented retained
object identity, optional same-session ElementHandle adoption, bounded native
routes and explicit disposal. The Node host did not yet select a Handle profile;
the JS package lacks `evaluateHandle`, `$` and `Locator.elementHandle`. The
new default paired lane consequently fails all 18 cases on missing candidate
entry points. That red baseline is intentional evidence, not an ignored test.

## Pinned public observations

The oracle is Playwright **1.58.2**, source commit
`ce480a952553175eae75342aad2c5e86cdf2cbba`. Local source inspection covers
[client ElementHandle](https://github.com/microsoft/playwright/blob/ce480a952553175eae75342aad2c5e86cdf2cbba/packages/playwright-core/src/client/elementHandle.ts),
[server DOM operations](https://github.com/microsoft/playwright/blob/ce480a952553175eae75342aad2c5e86cdf2cbba/packages/playwright-core/src/server/dom.ts),
and [injected payload mutation](https://github.com/microsoft/playwright/blob/ce480a952553175eae75342aad2c5e86cdf2cbba/packages/injected/src/injectedScript.ts).
The installed oracle version is asserted by the executable lane.

Actual Chrome observations, not inferred Locator behavior:

| Situation | Required observed behavior |
| --- | --- |
| Original input gets a new ID; a replacement gets the original selector | Upload changes the original node only |
| Original input is removed, but its owner document remains live | ownerFrame is still the main Frame; upload succeeds on the removed node |
| Retained label changes its `for` attribute | Upload follows that label's current control |
| Same-process DOM adoption moves the input into a child document | ownerFrame changes to the child; Handle identity and upload receiver remain the original node |
| Element lives in createHTMLDocument without a browsing context | ownerFrame is null; upload rejects before payload getters |
| Explicit dispose or Page close precedes upload | Target-closed error before payload getters |
| Navigation destroys the Handle's execution context | Execution-context-destroyed error before payload getters |
| Owning child Frame is detached while the creating world survives | Detached-element error before payload getters |

The distinction between a removed DOM node and a destroyed owning document is
essential. Do not reject every `isConnected === false` node, re-resolve the old
selector, or use a Handle's original Frame parent as a permanent ownerFrame
answer. Runtime identity, source execution world and current DOM owner document
are separate facts. The observed source preflight runs **before** Buffer getter
conversion; reusing synchronous Locator N-API ingress first would change error
ordering and run getters after retirement when upstream does not.

The lane also proves binary bytes, ordinary caller paths, clearing, hidden and
disabled inputs, text-input success/no-op, non-input rejection, Page/Frame `$`,
Locator.elementHandle and primitive JSHandle identity/idempotent disposal.
It does not yet cover OOPIF adoption, concurrent disposal after native mutation
submission, property batches, directory uploads or slow-preparation deadlines.

## Backend implemented here

`pwright-backend-cdp-file-input::PayloadTarget<'a>` borrows one exact CDP client,
execution-world ID, object ID and cleanup-group ID. Empty identifiers and IDs
over 1024 UTF-8 bytes reject without I/O or copying. `apply_on_target` shares
the original payload policy, byte accounting, encoding, ordinary/large transport
choice and native error parser. The existing `apply(&ResolvedElement, ...)`
entry delegates to it; no dependency, resource pool or ordinary bound grows.

This view is **not an ownership/admission certificate**. The embedding must
retain the native receiver, exact world route and separately admitted group
through settlement and cleanup. The primitive performs neither adoption nor
Handle disposal, and dropping a primitive future does not provide the owned
continuation guarantee. A persistent Handle must not be put into a temporary
upload group whose cleanup destroys it. Keep the original Handle lease and any
adopted wrapper/group lease distinct.

The existing owned upload continuation still consumes `OwnedElement`; it has
not been generalized to arbitrary borrowed targets. The Node bridge must use
an admitted owned target/continuation, not call this primitive from a JS wrapper
and hope GC supplies the missing lifetime.

## Required integration sequence

1. **Select and publish retained handles.** Add a narrow Node host/addon Handle
   selection. Reuse `new_controlled_page_runtime_with_handle_leaf` and
   `handle_leaf_services` over the existing live topology, object-ID source and
   controlled runtime; do not install a second route registry or executor.
   Reserve native route, preview/ID bytes, JS-conversion result owner and
   abandonment cleanup before native creation. Publish a typed JSHandle or
   ElementHandle only after conversion succeeds. Failed conversion, abandoned
   results and Worker teardown must compensate native results.
2. **Expose lifecycle and exact target access.** Add public `evaluateHandle`,
   Handle evaluation/jsonValue, `asElement`, `dispose` and `ownerFrame`, then
   Page/Frame `$` and Locator.elementHandle producers. Explicit disposal must be
   idempotent without hiding backend failures. Bind browser/Context/Page/Frame
   retirement and execution-world replacement to the same Handle route owner.
   `ownerFrame` must query the current DOM owner, not cache the creation parent.
3. **Compose Handle uploads.** Validate Handle ownership/ownerFrame before JS
   getters, using a bounded native preparation ticket if the boundary requires
   an async preflight followed by callback-scoped conversion. Do not retain raw
   Buffer pointers across that wait. Revalidate ticket liveness after reentrant
   getters. Bind exact same-session utility-world adoption to a temporary owner,
   preserve the persistent Handle and original error/timeout semantics, then
   enter the common one-mutation upload continuation. Public cancellation must
   transfer the retained target/source/adoption to pre-admitted cleanup.
4. **Verify the complete surface.** Make the default paired lane green, add
   cross-world/OOPIF and borrowed-Worker cases, creation/publication rejection,
   source-getter disposal races and mid-upload retirement. Assert native release
   and ledger baselines, not only JS GC. Keep directory/event completion and
   slow-input timeout differences as separate outstanding gates. Rebuild only
   selected artifacts and rerun feature-closure and full shared regressions.

Numeric Handle route/call/continuation and publication capacities must be derived
from the selected producer/retirement overlap and explicitly tested before
installation. The current source/FFI/transport budgets do not implicitly pay
for persistent Handles, adopted wrappers, listener waits or browser allocations.
The full Node compatibility goal remains unchanged.

## Executable evidence

After dedicated Chrome preflight, from the repository:

```sh
# Characterization only; this is not candidate conformance.
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 PWRIGHT_HANDLE_PROVIDER=oracle \
  pnpm --dir packages/pwright test:handle-file-input

# Default is both providers. Missing candidate APIs must fail, never skip.
PWRIGHT_TEST_CDP=http://127.0.0.1:9222 PWRIGHT_NATIVE_PROFILE=file-input \
  pnpm --dir packages/pwright test:handle-file-input
```

The supervisor retains its 180-second process/output bounds; each case has a
25-second test bound. Contexts are independent, Handle/Context/connection cleanup
is attempted on failure, and the candidate's host/FFI counters must drain.
These tests create no static server and stop no shared browser process.

Evidence lives in `.work/node151-handles/`. `oracle-first.log` passes the first
14 cases; `oracle-second.log` passes the expanded **18/18** and
`oracle-third.log` passes **18/18** with exact retirement errors pinned.
`paired-red.log`
fails **18/18** on candidate missing methods, after exercising the oracle for
each case. It is not added to the existing green 57-case shared report.

`backend-live.log` retains a fixture setup failure: the second retained-node
case redeclared a top-level `const replacement`. An IIFE fixes that fixture
scope, without changing production behavior or assertions.
`backend-live-second.log` then passes all 16 browser scenarios, including
both retained-node states, exact binary content and explicit native release.
Final strict checks, cleanup and artifact status are recorded in the file-input
checkpoint; oracle passes do not mean public Node Handles are implemented.
