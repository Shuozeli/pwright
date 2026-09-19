<!-- agent-updated: 2026-09-19T21:23:32Z -->

NODE-191 separates protocol, ownership, packaging and browser claims. Five fake
CDP tests prove exact world/group/object identity, one large user-gesture call,
no retarget or `DOM.setFileInputFiles`, integral N-API IDs, pre-I/O rejection,
ignored late-target return, native failure propagation and released admission.
Four composition AAA tests prove success settlement, unpolled no-I/O drop,
started-waiter cleanup transfer and cleanup-failure poisoning. The selected
host file-input slice remains 9/9.

Both final artifacts pass 392/392 offline retained-input/oracle cases, 167/167
package cases and the complete 42/42 ordinary real-Chrome raw binding lane; the
combined artifact also passes the focused 59/59 profile/dispatch/Worker batch.
Types, dependency
fingerprints, package boundaries, formatting and strict release Clippy pass.
The earlier 306-case retained-input subset is now run together with its pinned
DOM/target-marker oracle files as one 392-case offline gate; the live binding
module remains separate because it requires `PWRIGHT_TEST_CDP`.

The live gate now separates three claims. A direct pinned-oracle matrix compares
empty/multiple payloads, listener throws, late type changes, timeout
cancellation, Frame detach and main-document navigation at the same awaited
instrumentation boundary. Candidate-only same-turn native abort proves
continuation cleanup. An explicitly enabled maximum gate mutates 50 MiB minus
one byte under both artifacts, bounds sampled Node RSS growth below 768 MiB and
requires ArrayBuffer/native-owner reclamation. Preserve the observed navigation
settlement discrepancy: Playwright resolves while Pwright rejects fail-closed.
[Canonical NODE-191 evidence](pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190 separates four claims. Offline coordinator tests prove import ordering
and exact abort on rejection. Worker/profile checks prove only selected binaries
register the method and marker. The real-binding lane proves exact-session
provenance, eleven-to-zero root transfer, unchanged FFI permit, separately
charged Rust ownership, retryable capacity rejection and exact abort cleanup.
None of these is DOM mutation or public `setInputFiles` parity.

Final results under both standalone and combined artifacts are 482/482 in the
extended offline file-input matrix, 166/166 package cases, 36/36 focused
serializer/dispatch cases, 3/3 Rust owner cases, Worker loading and 34/34
real-Chrome binding cases.
Types, exact dependency/boundary scripts, formatting, diff checks and strict
release Clippy pass. Preserve the first standalone live run: its two failures
were test-contract defects, not accepted product behavior. One idle-only sampler
was applied to a deliberately pending ticket, and Node 26's `assert.throws`
return was treated as an Error. The corrected lane establishes its baseline
before the ticket, keeps capacity failure retryable before exclusive checkout,
and proves exact reclamation. Freshly rebuilt ad-hoc-signed addons also incurred
one slow first `dlopen` in macOS policy validation; warm reruns passed without
widening the Worker or case deadlines.

NODE-189 adds three deliberately separate lanes: direct differential comparison
with Playwright 1.58.2's `serializeAsCallArgument`; offline coordinator tests for
post-instrumentation ordering and exact exception cleanup; and the selected
real-binding lane for N-API session provenance, the eleventh root, unchanged FFI
admission/native owner and full abort reclamation. None is counted as DOM
mutation or public `setInputFiles` conformance.

Final NODE-189 results are 398/398 selected offline file-input cases, 165/165
package cases and 34/34 focused serializer/dispatch cases under both standalone
and combined artifacts. Both artifacts pass Worker loading and full 32/32
real-Chrome binding lanes; types, exact dependency/boundary checks, formatting,
diff checks and strict release Clippy also pass.

NODE-188: Test the instrumentation seam without conflating it with future
listeners. Offline adapter cases prove `prevalidate -> begin -> awaited hook ->
finish`, original-receiver/call-ID arguments, listener rejection cleanup and the
shared deadline. Worker smoke proves both new N-API gates and the compiled marker
are profile-gated. Dependency tests prove an addon-only single-flag delta.

Real Chrome uses a file INPUT and actual NativeHandle/preparation. It proves the
default awaited no-listener path retains ten roots, the one 1 MiB permit and the
validated owner until explicit abort, and separately proves foreign-session
begin cannot cancel the owner. Final results: selected offline lifecycle/Worker
275/275, package 164/164, types and boundaries green, strict standalone/combined
release Clippy green, and full live binding 30/30 under each rebuilt artifact.
[Canonical NODE-188 evidence](pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: Test the new leaf at four boundaries. Composition tests prove that a
validated owner retains the exact marker call and original group until one
release. Dependency/package tests prove explicit profile identity and an exact
marking-to-prevalidation graph delta. Offline dispatch proves the same progress
orders projection before prevalidation. Live Chrome uses a real file INPUT and
one payload, warms the persistent marker cache, then checks ten roots, the same
1 MiB parked permit, extra native leases and exact abort reclamation.

Final results: composition 52 passed/1 ignored; selected host 666 passed/24
ignored; selected addon 68/68; offline lifecycle 271/271; package 163/163;
types, dependency/boundary, old-marker/default-addon, fmt and diff checks pass;
strict standalone/combined Clippy passes. Both rebuilt artifacts pass Worker
teardown, the focused live case passes under each, and both full live lanes pass
28/28. Preserve the initial selected-host 657-pass/9-fail run: all failures were
the old six-lease idle assertion observing the intentional seventh preflight
service lease; the feature-gated exact baseline and full rerun pass.
[Canonical NODE-187 evidence](pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Test the retained owner and carrier projection as one ownership slice.
Offline dispatch records `mark -> filePayloads -> localPaths -> localDirectory
-> payloads.length`; addon tests retain existing fixed-layout/phase coverage;
Worker smoke proves both selected artifacts register and retire the new raw
method without browser tickets. Live Chrome warms the independent persistent
marker cache, waits for its temporary call leases to settle, then proves one
ticket owns ten roots, one 1 MiB FFI permit and a temporary marked owner until
explicit abort restores the exact warm baseline.

Final results: selected addon 68/68; selected host 666/24 ignored; offline
lifecycle 270/270; package 162/162; types, boundary, default addon check, fmt and
diff checks pass; strict standalone/combined Clippy passes. Both rebuilt Worker
profiles pass, and full live binding passes 27/27 standalone and 27/27 combined.
Preserve the earlier standalone 26/27 run: its sole failure sampled the baseline
before a warm-up marker call's asynchronous cleanup settled. The harness now
waits for that temporary lease reduction; it does not require the deliberately
persistent marker cache to disappear or close unrelated XLQY tabs.
[Canonical NODE-186 evidence](pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: Verify ordering and ownership at separate layers. Rust unit tests
prove per-session monotonic `call@N`, sibling independence, overflow and closed
scope behavior. Offline dispatcher tests prove `schema -> ID -> instrumentation
-> deadline -> materialize -> mark`, one progress owner and timeout revocation.
Worker smoke proves gated N-API registration/teardown without browser tickets.
Live raw cases use actual Handles/preparations for ID sequence, same-ticket mark
and outer-materialization timeout, then compare owned resource baselines.

Current results: selected addon 68/68; offline lifecycle 269/269; package
162/162; types and boundary checks pass; strict standalone/combined Clippy
passes; both rebuilt artifacts pass the final internal live lane 26/26. Preserve one
combined Worker exit/termination timeout followed by a clean pass. Also preserve
the standalone 24/25 and 20/25 full-lane runs whose failures occurred only at
existing five-second topology initialization while unrelated XLQY tabs loaded
shared Chrome. No deadline was widened and no unrelated tab was closed; both
later full runs passed with the same three tabs present. Public differential
parity remains a later gate.
[Canonical evidence](pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: Verify the selected Node marker bridge at three distinct boundaries.
Exact dependency fingerprints must show only addon/host/backend/composition
`retained-target-marking` flags and no package addition. Offline Worker loading
must prove the gated feature marker and raw method registration without creating
a browser ticket. Real Chrome must use actual NativeHandles and preparation
tickets through Materialized, then assert one parked 1 MiB admission at Marked,
running/queued abort, late original disposal and foreign-session isolation.

The current raw binding run passes 24/24 under both standalone and combined
marker artifacts with final owned host/FFI/preparation retirement. Both
artifacts pass Worker smoke loading; retained lifecycle contracts pass 267/267,
package contracts pass 162 and strict selected Clippy passes. These gates do
not prove the absent public deadline/call-ID source, automatic later phases or
public differential parity. Add deterministic publication-callback,
pending-shutdown, Worker teardown and timeout races before widening the claim.
[Canonical evidence](pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: Gate the default-off Rust host marker cache separately from Node
language preparation and public upload. Deterministic tests cover exact-Frame
utility selection, shared admission, independent retirement waiters, coalesced
initialization, capacity/replacement, cancellation, cleanup failure and stale
generation ownership. Route invalidation alone must never certify native
destruction; failed release must remain observable and close admission.

The ignored assembled-host Chrome case uses real A/B ElementHandles
and only host APIs for marking. It checks repeated-ID A/A/B events, new-ID
behavior, state surviving source disposal, fresh-world navigation and final
resource retirement. Its finite spawned body captures panic/error/timeout and
always attempts owned Context and attachment cleanup, preserving shared Chrome.
The same one distinct case passes both final host selections with all owned
resources zero; an earlier initial execution is recorded separately. It is not
a Node ticket/Weak-upgrade/deadline or retained upload test.

Final-source host tests pass 666/24 ignored standalone and 728/26 ignored
combined, with strict Clippy passing both. Full composition passes 395/16
ignored and strict Clippy; the older retained host regression passes 637/23
ignored. Package tests (161), marker oracle (21), DOM oracle (35) and types pass.
Default host tests pass 429/8 ignored with strict Clippy. Independent standalone/
combined marker dependency proofs, runtime-JS/addon boundary checks, formatting
and whitespace checks pass. No Node artifact rebuild is claimed. Exact outcomes belong in the
[canonical NODE-183 checkpoint](pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).
Earlier checkpoints retain their historical scope and execution status.

NODE-182: Keep script policy, native ownership and actual DOM evidence separate.
`pnpm test:handle-target-marking-oracle` passes **21 offline cases**: 19 paired
comparisons use the actual pinned Playwright 1.58.2 `_markAsTargetElement` and
extracted injected marker method against the checked-in scripts; two cases
check private-container creation. Plain descriptors/recording event stand-ins
prove language/state ordering, not DOM propagation, utility-world identity or
native resource release. The added Set-constructor error case requires old
state to survive because construction precedes different-call reset.

Separate backend/composition Rust suites pass 37/36 standalone, 55/51 combined
with preflight/large commands, and 16/17 with defaults. One distinct real-DOM
native primitive case passes in both final selections, exercising nine marking
calls and eight observable events per execution, with
owned cleanup resources returning to zero. The `retained-target-marking`
feature remains default-off and absent from host/addon selection. These gates
do not prove public Node call-ID/deadline behavior, original weak-target
promotion, GC or complete retained upload. Final routing/integration evidence
belongs in the [canonical NODE-182 checkpoint](pwright-node-handles.md#node-182-persistent-native-target-marking).
Earlier checkpoints below retain their historical scope and execution status.

NODE-181: Preserve real-ticket original-source/receiver distinction, disposal
during owner lookup, explicit native resource release and later getter-error
identity/order. The weak binding itself is inert: these cases can also pass
using existing copied metadata, so they do not prove later Weak upgrade, wrapper
collection, marking or upload. Keep concrete 32-entry registry and independent
16 KiB synchronous-phase accounting proofs separate from browser observations.

Final rebuilt artifacts pass **268 offline checks each**, selected addon suites
pass **65 each**, and the focused host phase-storage test passes. Package tests
pass **161**, including ten pure observation/diagnostic tests; types, the
35-case descriptor/language DOM oracle, five strict Clippy selections,
boundary/dependency gates and formatting pass. Standalone and combined
real-ticket live binding each passed **20/20** after the final-retirement fix.
A standalone repeat passed 19 and failed one full active-baseline comparison
when a queued watch became armed (same pending/watch count and bytes); all
cleanup passed and no fixture poisoning followed. Active baselines now require
no queued FFI and exact pending/running watch accounting, preserving every
snapshot counter comparison. Final frozen-source standalone, combined and
standalone-repeat runs each pass **20/20**, with no failures/cancellations/skips
and fully retired owned cleanup. Count this as **20 distinct cases run three
times (60 successful executions)**, not 60 distinct cases. Preserve earlier
failed runs: the initial five passes were followed by case-six cleanup failure and fourteen fixture
blocks; the diagnostic run failed its first case on remaining `ffiPending = 1`
and blocked nineteen. Bounded nested diagnostics exposed this sampling race.
Active-session baselines still permit watch work; the final-retirement predicate
now requires all host/FFI/preparation zeros before accepting stable samples,
without changing deadlines or suppressing cleanup failures. Public upload and
future typed native/Weak-upgrade behavior remain separate gates. Exact evidence
belongs in the [canonical NODE-181 checkpoint](pwright-node-handles.md#node-181-original-native-wrapper-binding).
Earlier checkpoint paragraphs below retain their historical execution status.

NODE-180: Gate the eight-slot host revocation registry with actual FFI entries
and Owners work tokens: exact source/registry provenance, generation reuse,
disposed metadata without native-use authority, parked-result reclamation and
running-future Drop before accounting release. Separately exercise real stop
and tracker wakeups against the shutdown helper's admission barrier; an
in-flight token must publish before the barrier completes, and later admission
must observe cancellation. Keep wakeups/destructors outside locks, including
Owners close. Source review additionally moves retirement-failure transport
disconnect outside the session lock; do not present that as a full-session
failure-path test. Root has recorded the red-then-green Owners close regression
and host results. Both rebuilt standalone and combined selected artifacts pass
**268 cases across seven offline lanes each**; independent regressions pass 151
package contracts, type checks and 35 descriptor/language DOM-oracle cases. These
component fixtures do not prove assembled HostSession shutdown, actual Node
preparation promotion, DOM/browser cleanup or upload. The Node preparation
still stops at Materialized; original-wrapper binding, typed progress, marking,
retained prevalidation and late serialization remain integration gates. The
17 actual-ticket live cases remain unexecuted.
Exact evidence belongs in the
[canonical NODE-180 checkpoint](pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Compare actual Rust raw-carrier projection with pinned `_setInputFiles`
stopped before native evaluation. Cover ordered getters, raw falsy multiplicity,
relational coercion, thrown-value identity, no assimilation/traversal and
stateless reentry; label diagnostic output-shape contracts separately. Both
final standalone and combined artifacts, rebuilt after the shutdown-lock/waker
fix, pass **47 new projection cases and 268 selected offline cases each**.
Package contracts pass 151, type checks pass, and the separate descriptor/language
DOM oracle passes 35. Keep same-permit FFI park/resume Rust ownership,
cancellation and wakeup evidence separate: no language fixture proves actual
ticket reclamation, marking, DOM/browser behavior or upload success. The 17
actual-ticket cases remain unexecuted. Final evidence and remaining integration
gates belong in the
[canonical NODE-179 checkpoint](pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: `pnpm test:handle-file-input-dom-oracle` passes **35 offline cases**:
24 actual pinned upstream prototype/serializer cases plus 11 actual candidate
script comparisons against captured upstream precheck and generated retarget
code. Evaluation adapters stop at explicit boundaries; plain node descriptors
do not prove DOM/browser behavior, native ownership or upload success. Current
package regressions pass 151 contracts and type checks. No addon rebuild was
performed; selected artifacts remain the unchanged NODE-177 builds, and all
17 actual-ticket live cases remain unexecuted. Keep native acquisition,
cancellation/cleanup and dependency verification distinct from these language
oracles; their final evidence belongs in the
[canonical NODE-178 checkpoint](pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Both rebuilt selected artifacts pass twenty new offline dispatch
cases each; the 17-case actual-ticket lane remains unexecuted. Full evidence
and exact results belong in the canonical checkpoint. Real
upstream DispatcherConnection/Dispatcher/SdkObject cases stop by deliberately
throwing at command entry, never reporting upload success. Separate candidate
driver adapters call actual Rust kernels but do not prove native ticket/session
authority or native-root reclamation. Cover initial registration, schema-error
precedence, instrumentation-await/identity recheck, second-schema microtasks,
abort forwarding and unobserved hostile ticket.then. Gate actual owner-ticket
plus creation-Page checks separately from world/Handle/Frame runtime retirement.
No fresh browser pass is claimed; the public upload, late utility serializer,
native submission and full instrumentation/error-transport gates remain open.
[Canonical contract and verification](pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Add separately labelled direct payload-helper and deferred
two-validator comparisons against actual Playwright 1.58.2. Do not equate raw
kernel equality with dispatcher/error transport, actual ticket or browser
conformance. The live binding lane has 12 authored unexecuted cases; four new
cases exercise raw materialization and revocation. Both selected artifacts pass
76 new offline cases, 61 channel, 36 argument, 27 preflight and one Worker test.
Explicit addon `--lib` tests pass 63 per profile; other exact verification is
tracked in the canonical checkpoint. The overall goal remains incomplete.
[Canonical contract and current evidence](pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Compare the selected native raw-buffer channel kernel
against pinned Playwright's actual validator in normal non-isUnderTest mode.
Cover schema traversal, optional/unknown fields, boxed valueOf raw returns,
Buffer identity without serialization/copy, observable sparse/custom map and
escaped callbacks, index coercion, inherited output setters and errors. Test
the internal driver continuation separately, without claiming real ticket or
public API attribution. Keep unsupported genuine WritableStream success,
server materialization, deadline/submission and browser gates open. Current
execution evidence belongs to the [canonical NODE-175 record](pwright-node-handles.md#node-175-rust-owned-raw-buffer-channel-validation).

Both final zero-capture selected artifacts pass 61 channel cases, 36 argument
cases, 27 preflight cases and the Worker smoke. The new channel lane includes
three explicitly labelled driver-adapter cases, not real tickets. The live
binding lane now contains eight authored cases, adding ChannelReady/abort and
channel-getter failure/root drain; it remains **syntax-checked but unexecuted**.
No browser gate is closed by these offline checks.

NODE-174 (2026-09-17): Exercise canonical timeout indexes with real host ledger
and source-provenance fixtures: same-ID foreign sessions, wrong/missing Page
proof, generic/disposed origins, live parent identity, repeated binding admission,
close/churn and retained lease reclamation. Scalar/batch publication asserts the
frozen creation Page. New package contracts execute actual index.cjs with a fake
addon loader: cached raw debug tuple, Page-ID forwarding, legacy profile fallback,
conversion-before-native entry and pre-commit lifecycle wrappers with no policy
queries. These are not native producer or browser tests. Independently verify
Rust policy error deferral past both materialization paths, descendant propagation,
and all-scope sealing before environment root deletion; code review does not close
the actual N-API/browser gates. [Canonical outcomes and open gates](pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding).

`test:handle-file-input-binding` adds six live actual-ticket cases under a
180-second supervisor. They target creation-Page policy versus current-owner
override, generic Handle descendants, disposed/closed-Page metadata, wrong-Page
publication compensation and sibling-session rejection. Run separately for both
selected artifacts with the dedicated loopback CDP endpoint after recovery. The
script fixes `PWDEBUG=0` before import; the assertions stop at ArgumentsReady and
abort/reclamation, not upload. **Authored and syntax-checked only; unexecuted**
at this checkpoint. A 20-second exercise bound precedes four three-second
aggregate cleanup phases, with a 35-second outer case limit. All owned Contexts
and connections receive retirement attempts even after another phase fails;
retirement guards reject late exercise work, and an exercise deadline or cleanup
failure prevents later fixtures from starting. Late acquisition retirement is
tracked and bounded separately; a timed-out exercise does not prove complete
reclamation merely from earlier zero metrics. Resource assertions cover native-host/FFI/preparation
metrics, not the separate timeout-policy ledgers. Existing offline Worker checks now verify four selected
session methods but still create no actual ticket or browser connection.

NODE-173 (2026-09-17): Compare post-conversion spread/getter/timeout order using
real selected Rust client metadata and stateless N-API kernels against pinned
ElementHandle, Frame and TimeoutSettings bodies. Capture before channel
validation; label direct-expression kernel cases separately. Exercise live
default mutation during getters, repeated raw option reads, falsy normalization,
fractional/nonfinite numbers, symbol/proxy order, exception identity and no-extra-
await continuation. Rust tests prove metadata admission/lifetimes, while Worker
tests audit the tenth selected opaque class. None creates a real preparation
ticket or proves production creation-Frame binding/public setter/native deadline
parity. [Canonical NODE-173 verification](pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings)
records exact outcomes and outstanding gates.

NODE-172 (2026-09-17): Compare the actual selected Rust/N-API preflight kernel
with pinned Playwright, preserving owner/context/async conversion boundaries,
custom array-method observations, coercion and thrown-value identity. Stateless
diagnostic entrypoints grant no session/native authority and hold no persistent
roots; this lane does not test real preparation tickets, channels, server
materialization or the DOM. Test weak phase ownership, revocation/replacement
and checked root counting independently in Rust. Cover initial construction,
the independent 32-phase scratch limit despite revoked-entry reuse, and scratch
retention through replacement/rejection/destructor reentry without FFI work.
Keep source/stack shape and
public closed-target ordering limitations explicit. Exact commands, results
and remaining gates are in the [canonical NODE-172 record](pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases).

NODE-171 (2026-09-17): Verify selected Node preparation with deterministic
Rust registry/source-provenance tests, explicit addon `cargo test --lib`, exact
normal dependency/feature fingerprints and separate selected-artifact Worker
smokes. Run `PWRIGHT_NATIVE_PROFILE=retained-file-input pnpm test:preparation-boundary`
from packages/pwright after building that artifact; repeat with
`retained-file-input-combined`. These offline Workers load real N-API classes
and exit through real environment hooks but create zero tickets/connections.
Fake payload destruction is not actual Node-root deletion, immediate raw
peer-close cleanup, listener ordering or public upload parity. Standalone and
combined addon test/Clippy commands are added to CI; current execution evidence
and remaining live gates belong to the
[canonical NODE-171 record](pwright-node-handles.md#node-171-session-bound-node-preparation-registry).

NODE-170 (2026-09-17): The existing host `retained-file-input` selection now
exposes `FfiPreparation<T>`: preparation shares the ordinary lane/byte budget,
but creates no running-task token until atomic promotion. Tests target capacity,
revocation, start/shutdown ordering, foreign-budget rejection and weak-owner
lifetime. Exact budget origin is environment authority, not session authority;
the Node registry, session revocation, type-tagged ticket and full ingress remain
unimplemented. Rust verification is still in progress; current results belong
in the [canonical NODE-170 record](pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution).

The pinned offline ingress oracle includes dedicated cases for
overridden owner `page()`/`context()` evaluation and errors, empty-input microtask
ordering, and disposal reentry. Disposal exercises real client metadata retirement
from a dispatcher-shaped event, not native DOM disposal. Custom Buffer
serialization and per-item snapshot order are also characterized. Exact results
belong in the canonical record. These upstream-only results prove neither
browser behavior nor native candidate parity. No addon, public Node API,
artifact, dependency or feature is added by this checkpoint; earlier
entries below retain their historical scope.

NODE-169 (2026-09-17): Retained-upload checks combine synchronous source identity/disposal tests, atomic tracked-call accounting, gated real-service/Fake cancellation and cleanup, and a pinned offline client/channel/server-preparation oracle. An assembled host browser test remains ignored pending dedicated Chrome recovery. Upstream-only oracle success is not candidate API or native parity.
[Canonical contract and evidence](pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): Add deterministic retained-reference tests for untimed
grouped resolve/release, exact unique-world verification, malformed response
group cleanup, borrowed-source protection, independently selected scratch,
route retention after caller cancellation, cleanup poisoning, numeric snapshot
replacement and legacy abandonment error propagation. CI selects both optional
resolver leaves for tests/strict Clippy and checks unchanged dependency closure.
An ignored native test compiles and covers actual temporary invalidation,
borrowed-source survival and mismatched-world rejection within its own Context;
it has not run. The shared Chrome endpoint remains unavailable; these gates do
not substitute for live Node upload or final ownerFrame differential verification.
[Canonical commands and evidence](pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress).

NODE-167 (2026-09-17): Current-owner Frame verification combines exact CDP
request/deadline tests, backend cleanup/identity bounds, tracked-read capacity
and cancellation tests, wrapper-resolver propagation and a paired live lane.
The live lane distinguishes removed nodes, document adoption, orphan owners,
retirement and canonical Frame identity; optional sandbox coverage requires
native OOPIF target evidence. Keep the old-artifact ABI red baseline separate
from built semantic results. [Exact current evidence](pwright-node-handles.md)
remains canonical; no retained-upload or full-corpus pass is implied.

NODE-166 (2026-09-17): Property-batch gates combine deterministic raw-CDP,
backend compensation, route trimming, staged publication, host admission and
receiver-loss tests with real Chromium differential lanes. The focused
handle-properties lane also covers partial JS Map failure, prototype-safe
native publication and explicit capacity reuse without GC. Keep native class
isolation and existing producer/acquisition regressions separate. Unit counts,
artifact hashes and live results belong in [the canonical checkpoint](pwright-node-handles.md).
Source review of N-API recovery is not fault injection of every allocation
failure; focused passing lanes do not clear the historical full-corpus gaps.

NODE-165 (2026-09-17): Verify the retained-result acquisition boundary with
both the real in-memory CDP decoder and deterministic backend/composition
tests. Decoder tests must preserve result/exception IDs and late settlement,
because configured fake responses bypass ordinary exception stripping.
Backend cases cover malformed/throwing responses, duplicate and unaddressable
IDs, all-ID release attempts, bounded diagnostics and honest Cleanup errors.
Composition gates pause acquisition or release, drop started operations,
check admitted route/native-call resources, replace a source world, and
observe failed continuation health rather than silent success.

[The canonical checkpoint](pwright-node-handles.md) owns exact results,
pending live checks, artifact identities and log paths; older passing counts
below are not NODE-165 verification. Runtime.getProperties late replies and
full property-pipeline admission, pre-owned cleanup groups for unaddressable
IDs, Node getProperties batch publication and the full compatibility corpus
remain unverified or unfinished at this boundary.

NODE-164 (2026-09-17): The priority raw-class audit now has an implementation and a dedicated native-class module lane: safe no-cast probes precede foreign argument/receiver calls, covering an earlier addon and an independently loaded same-build copy. Borrowed methods can reject in V8 before Rust, so the locked-source validation-order gate and Rust all-class TypeTag proof are also required. Portable property identity, admission and abandoned Cleanup reporting have additional regression coverage. [The canonical checkpoint](pwright-node-handles.md) records exact verification and remaining gates; Node getProperties and the full-corpus non-green gates remain open.

NODE-164 selected gates: native-class module isolation 3/3, Handles 30/30,
wrapper-module isolation 1/1, 110 package contracts, TypeScript and the full
source/dependency boundary gate pass. Rust checks pass 32 default leaf tests,
48 selected leaf tests, four selected addon tests, one default addon test and
451 selected host tests. The new portable property real-Chrome test passes
separately; strict selected/default/all-feature Clippy passes. Explicit ignored
tests, artifact identities and cleanup evidence are retained in
[the canonical checkpoint](pwright-node-handles.md). The producer lane still
has five missing getProperties failures, and the full compatibility corpus is
not rerun or claimed complete.

NODE-163: The producer lane retains all 21 paired cases: 16 pass and five getProperties failures remain. Red/green evidence includes the creation-future storage regression, Promise/thenable property identity, native/JS publication rollback and safe-host allowance retention. Existing 30 Handle tests, 38 ordinary evaluation tests, one independent-module wrapper test and 110 package contracts pass on the final selected artifact. Raw native-class cross-build validation remains a distinct audit gap. Exact Rust counts, artifact hash, one unreproduced initial timeout and unchanged full-corpus failures are in pwright-node-handles.md.

NODE-162: New paired Handle tests reproduce internal eval-scope leakage, incorrect global declaration lifetime, unsupported string paths, and premature disposed-target rejection. Preserve the old-artifact red and built-artifact phase failure separately. Native-program units and source/node boundary cases verify unchanged admission without expression rewriting; canonical final counts live in pwright-node-handles.md.

NODE-161: Expanded Handle tests now cover nested/aliased/cyclic arguments, primitive and DOM references, Handle-accepting producers/reads, binary timing, disposed getter phase, foreign-session rejection and full 64-slot capacity. The final artifact passes 20 paired cases plus one native-class prototype guard, an independent-module rejection test, and 38 ordinary evaluation cases. A nested-fixture red check incorrectly demanded zero environment-shared FFI bytes while an outer watcher was alive; the corrected test compares its measured entry baseline and still requires complete zero at the outermost close. Final artifact and remaining gates live in the canonical Handle record.

NODE-160: The new focused public Handle lane compares actual Node APIs against pinned Playwright 1.58.2: nine paired cases plus one explicit unsupported-reference check. It does not replace the retained-upload 18-case lane. Ordinary evaluation also passes 38 paired/contract cases on the selected artifact; evidence and remaining fault-injection gaps are in the canonical Handle record.

NODE-159: Verify driver publication with deterministic receiver-loss, unread reply, deferred commit, stale source, queue-close/late-arrival and compensation-failure tests, plus real safe-host creation/unrelated-close interleaving and retained-descriptor shutdown. Keep these separate from the unfinished public Node Handle lane.

NODE-158: Staged creation adds nine deterministic integration checks and a real Chrome single-execution check. The native test holds a deterministic local unrelated-Page lifecycle gate, not a real Chrome Page-close acknowledgement race. Selected/default leaf, strict Clippy and dependency evidence remain separate from host/Node integration.

NODE-157: Preserve the initial 3/4 native mixed-lifecycle failure (SessionDetached during Context close) separately from the four passing tests after exact native-world evidence integration. Both ordinary and combined Handle builds pass native checks; release scheduling is not evidence that raw creation scheduling or public Node Handle parity is complete.

NODE-156: Fifteen phased-release public-API tests pass, including same-ID/payload/disposer replacement and retained storage after parent retirement. A real Chrome object becomes unreadable after native acknowledgement while local state remains live until deferred commit. Keep this native evidence distinct from still-absent public Node Handle parity.

NODE-155: Runtime tests cover queue-free admission plus retained busy-boundary ownership in both outbox modes, latest-subtree retry, terminal failures and post-commit notification storage. Record default, selected and combined feature gates separately in pwright-node-handles.md; this runtime-only change supplies no new browser or public Node parity evidence.

NODE-154: Five owner tests and three native checks pass. The Chrome scenario uses an independent Context owner, abandons unpolled/started/Ready results, rejects a 65th creation before browser effects and retains 64 descriptors across subject-host close. The borrowed Page survives; exactly 65 storage leases remain until descriptor drop. Preserve the initial lifecycle-transaction conflict and later test-metric correction separately from the final gate; mixed lifecycle coordination remains open.

NODE-153: Four deterministic Handle batch abandonment regressions inject first/middle/multiple native release refusals or success through FakeCdpClient while retaining real services/runtime/ledger. Preserve the pre-fix three failures; verify every release attempt, failed-route visibility, lease counts and explicit retry. These tests do not advance the Chrome or Node differential score.

NODE-152: New native Handle assembly tests use the exact host profile and public composition services, alongside a real selected-host connect/close test. Preserve the initial missing-topology-driver failure; corrected assembly must use the normal publication fence. These native gates do not replace the still-red Node Handle lane.

NODE-151: New explicit test:handle-file-input lane characterizes 18 public Handle cases. Oracle-only passes do not count as candidate conformance; paired mode is red because producers are missing. Backend live checks retain original/removed-node identity and explicit release. The previous shared score is not updated by this separate lane.

NODE-150: Page/Frame upload coverage passes 43/43 x3 standalone and 43/43 combined; 100 contracts, 30 shared units, both TypeScript checks, strict Clippy and dependency gates pass. Full corpus remains 53/57 matching with zero candidate/setup/cleanup errors; click values reverse the prior run. Separate raw-click controls retain the original workload and expectations. Temporary service and preflight target cleanup is verified.
[Page/Frame upload contract and verification](pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 final verification: 98 JS contracts, 30 shared-harness unit tests, both TypeScript checks, default/selected/combined strict Clippy and dependency gates pass. Standalone and combined upload suites pass 32/32; full shared corpus reports 53 match, 1 value-mismatch, 1 unsupported, 2 oracle-error and zero candidate/setup/cleanup errors.

NODE-149: Added a public file-input differential lane and bounded Worker teardown gates. Preserve initial fixture failures, the Uint8Array mismatch and Error/TypeError red tests separately from corrected passes.
[Selected Node upload boundary and verification](pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Added selected safe-host profile checks and real Chrome upload/lifecycle cases, plus retained-source admission tests across native response and group cleanup. These validate host assembly, not the still-unwired N-API/JS differential surface.
[Selected safe-host upload contract](pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: The selected transport tests exercise independent admission, builder composition, ordinary-limit preservation and queued/unconsumed ownership. An explicitly ignored memory-heavy Chrome test validates near-limit bytes plus cross-fragment UTF-8. This does not replace public Node differential acceptance.
[Large outbound admission and remaining gates](pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: New continuation tests cover unpolled/started/release-phase abandonment, external deadlines, storage/queue admission and cleanup failure. CDP paused-clock tests distinguish retained settlement from ordinary timeout. The separate ignored Chrome continuation test verifies complete, drop-started and drop-unpolled flows using actual File bytes, events and root cleanup; public Node differential acceptance remains open.
[Upload continuation and remaining gates](pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: 226 release tests pass across the file-input, CDP and Fake packages; 89 Node contract tests, strict Clippy and dependency boundaries pass. The explicitly invoked Chrome integration test passes 14 scenarios, including actual file preparation through the Rust executor, browser byte/MIME/mtime/event inspection, native group cleanup and a throwing files setter. Its default ignore is retained for CI environments without Chrome; these are fixture checks, not the public Node differential gate.
[Payload backend verification](pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: 16 executor tests and 70 earlier filesystem/source/model tests pass in release mode, with strict Clippy and exact dependency proof. Coverage includes actual file preparation, blocked-reader cancellation, queued cancellation, unconsumed completion capacity, stopped runtime, worker panic and one shared production ledger. This is not a Node/browser upload gate.
[Executor verification evidence](pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: 19 filesystem, 22 content-source and 29 model release tests pass with strict Clippy and exact dependency proof. The native Rust MIME lookup matches Playwright 1.58.2 on 9,393 cases across all 1,173 table mappings and filename variants. These checks cover filesystem/value preparation, not browser injection, worker cancellation or public upload support.
[Filesystem/MIME evidence](pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: All 20 source-content release tests and 29 model tests pass, with strict release Clippy and dependency proof passing. Tests include real checked-in file bytes, growth/truncation, admission failure before IO, owner transfer and abandoning a result during a gated read. These are Rust content ownership checks, not browser upload conformance or a production executor cancellation proof.
[Content ownership evidence](pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: File-input preparation has 29 Rust release tests and 21 upstream-only browser observations, including binary bytes, path versus directory native dispatch, validation order and exclusive 50 MiB rejection. Preserve these as model/oracle evidence, not candidate upload conformance. NODE-140 full-corpus failures are unchanged.
[File-input policy, evidence and remaining work](pwright-file-input.md).

## NODE-140: broadened Node FFI verification remains non-green

This checkpoint expands beyond NODE-139's focused close gates. It changes test
infrastructure, not production Rust semantics, deadlines or resource capacities.
The tested experimental `concurrent-close-drag-frame-extra-headers` addon remains
`1b18f3cca8fa45e351c50bed6d3dc42be25a1a67c77d2895bf4638d052d90efc`.
Playwright 1.58.2 and the existing dedicated Chrome 152 are unchanged. No Chrome
restart, foreign Page cleanup, browser download, JS automation fallback, retry,
new production wait or golden rewrite was used.

All evidence below is in `.work/node140-frame-corpus/`. Static fixtures were
served from the exact repository `tests/fixtures` via an owned `/node140` mount
on the existing Tailscale-bound serve-lib daemon. The shared runner verified
every served fixture's bytes against disk and recorded source/native hashes.
The temporary mount is removed after verification; recorded fixture URLs are
historical evidence, not a currently promised preview.

### Whole-corpus outcomes

| Gate | Result | Remaining evidence |
| --- | --- | --- |
| Full Frame navigation | 27 passed / 2 failed | Candidate-independent and final upstream arrival-boundary Page.close each exceed the unchanged 10-second diagnostic bound |
| All 19 shared suites / 57 unchanged cases | 51 match / 1 value mismatch / 2 unsupported / 2 oracle errors / 1 setup error | No candidate-action or cleanup classification errors; incomplete compatibility, not a passing report |

`full-frame-first.log` retains both failures, including their exact stacks. The
29-case suite was not filtered or weakened. It still verifies main/local/sandbox
navigation, four lifecycle waits, response/Frame identity, same-document and
data/about URLs, 404, baseURL/referer, detach/invalid-option behavior, retained
Response ownership and loading continuation after API timeout. NODE-139's focused
passes do not supersede this non-green full corpus.

`shared-before/report.json` and its exclusive supervisor/source metadata retain:

- `page-content / should get text content of nested element`: candidate setup
  fails during newPage. Browser cleanup reports the original native driver
  topology-snapshot failure: `native Frame lifecycle: Lifecycle(RequestCapacity)`.
  The selected profile declares 256 lifecycle request slots per Page. This report
  does not identify which Page filled them, prove a leak, or justify increasing
  limits/ignoring events. Add bounded Page/request ownership attribution before
  changing lifecycle admission or reclamation.
- `touchscreen / should dispatch touch tap`: oracle returns `touched`, candidate
  `No touch`. The historical fixture calls raw mouseClick, **not touchscreen.tap**.
  Preserve the value mismatch; this is not proof of a touch API implementation.
- Download event and file upload are still explicit unsupported operations.
- Both history cases remain oracle TimeoutErrors after back navigation. Candidate
  goBack is separately recorded unsupported. Do not classify oracle failure as a
  match or silently alter the externally owned browser's BFCache behavior.

### Controlled Page-close/body diagnostic

`probe:page-close-body` adds two owned-page observations per provider: the original
response-arrival boundary and an explicitly published document-commit boundary.
It transparently records native close/navigation receipts, observes 200 ms after
the close reply, then ends only its fixture's held HTTP body. That interval is a
diagnostic control, not an API deadline or a success threshold. No ancestor close
is sent before body release; Context/Browser cleanup occurs afterward and is
reported separately. Reports are bounded to 512 KiB and written exclusively.

`oracle-body-release.json` retains a **failed arrival case**: native close replies,
the new document commits, body release lets goto(load) succeed, but Page.close
still exceeds its 10-second diagnostic bound. It fulfills only during subsequent
owned Context cleanup. Thus ending the body alone did not release this observed
close. The committed oracle control closes before body release. Both candidate
controls close before body release in this run; that repeat does not erase the
candidate failure in the full corpus or demonstrate a production fix.
All four controls finish cleanup without leaving an owned native target. Exact
native/browser cause and timing-sensitive cross-provider behavior remain open.

### Original raw-click sequence with after-action inspection

`probe:shared-click` preserves goto -> mouse.click(100,100) -> result evaluation.
It adds no pre-click evaluation, focus operation, reset, paint wait or retry.
Only after the original result is captured does it read geometry/hit target,
readyState, focus, viewport, scale and the result again. Five pairs run directly
and five through the bounded transparent CDP observer; all use fresh owned
Contexts. Native input journal capacity is eight commands per case, report
capacity 1 MiB, and failures remain nonzero exits.

`shared-click-probe.json` records **20 attempts, only two intended successes**:
direct oracle 0/5, direct candidate 0/5, traced oracle 0/5, traced candidate 2/5.
There are no setup/cleanup errors in this diagnostic. Both reads remain unchanged
when a click is missed; geometry is 8,8,200,200, hit target is `target`, viewport
1280x720/scale 1 and readyState complete. Captured move/down/up coordinates,
button masks, click counts and modifier fields agree across providers. This
reproduces an upstream-visible failure too; it does not identify browser focus,
paint, event contamination or Rust as the cause. A relay's passing samples do
not validate a relay workaround.

Equal returned values alone do not prove the named fixture's intended behavior:
both sides returning `No touch` would be equal but would still miss the click.
Keep explicit expected-behavior diagnostics separate from corpus equality counts.

Next implementation work must retain these non-green gates, attribute lifecycle
request capacity before modifying resource policy, and complete missing Rust-
backed public surfaces such as file upload/download. Broad compatibility and
three consecutive fully passing release comparisons remain unproved.

Final tooling verification: 89 package contracts and 30 shared-runner unit tests
pass; package and shared-runner TypeScript checks pass; both new diagnostic
scripts pass Node syntax checks. All eight new documentation routes resolve and
git diff whitespace checks pass. No Rust source or addon was rebuilt in NODE-140.
The exact `/node140` mount was deregistered and port 19340 has no listener; the
preexisting shared serve-lib daemon remains untouched. No probe/test process is
left running and no commit/push occurred. At 19:45:13Z shared backend PID 53189
had RSS 3,479,616 KiB (about 3.32 GiB), versus 3,440,992 KiB at turn start, with
zero swap. These are shared-process observations, not per-chat RAM attribution.

NODE-139: Compare navigation errors only after establishing the same native document phase. Preserve baseline and rebuilt artifact evidence separately. Add deterministic publication-race and real lifecycle-owner wake tests, bounded native phase tracing, and five committed-document/close regression controls paired with pinned Playwright 1.58.2.
[Page lifetime publication contract and verification](pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: Test the selected Node coordinator with the original held-retirement ancestor-dispatch probe, then ordinary close/terminal/resource differential lanes. Keep compile/unit tests, actual command dispatch, public settlement and leak checks distinct; do not substitute the earlier isolated composition evidence.
[Node close-driver integration and final verification](pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: Local close integration tests cover actual runtime/routes/network/Handle/world owners, single Request disposal, parent-first joins and preflight failures. They use explicit native keys and a bounded test observer; bounded-CDP evidence validation is separate. Do not report them as Node lifecycle or browser parity.
[Native local retirement and post-lock notification contract](pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: Joined close tests cover staged admission rollback, move-only ownership, latest descendants and local retirement. Native completion tests cover proof/slot lifetime, error and panic, plus a real-CDP bounded-peer composition test with a test publication observer. This is not Node event publication or a browser parity result.
[Joined close capacity and native completion contract](pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: Ten guarded runtime-publication cases cover a separately running transport closer blocked on the state mutex, callback failure and output identity, preflight rejection, deferred reentrant wakes, late-call typed failure, retained capacity, external non-rollback and panic poisoning. Runtime suites pass 113/178/196; default host 416 (8 ignored). These are synchronization/regression tests, not native or Node/browser close parity.
[Runtime-locked publication/disposal contract and evidence](pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Outbox tests verify full creation-plus-disposal queue capacity without growth, preserved FIFO messages/metrics/leases, retained storage lifetime and rejected capacity/claim/gate/budget/transport states. Runtime ordinary/selected/combined passes 113/168/186; core passes 75. Retain the intermediate incorrect global-versus-scope exhaustion fixture separately from the final green run. No new browser result is claimed.
[Preallocated runtime outbox contract and evidence](pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Twelve new retained-world decoded-driver tests pass: selected/combined composition totals are 397/486 (16 ignored each), default host 416 (8 ignored), Node unit 84. Both rebuilt profiles pass six native-close-confirmation cases, plus six default/ten combined attachment-retirement cases: 28 focused Chrome regressions. Keep those distinct from fake-client receipt tests and the unresolved controlled ancestor-dispatch gate. Missing-await compilation evidence is retained; strict lint, types, dependency boundaries and formatting pass.
[Native Page world-retirement contract and evidence](pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: All 12 new native commit-scope cases pass in the 474-test combined suite (16 ignored); authority-only regression passes 394 (16 ignored). Cases use production CDP with bounded peers and real topology commits, covering exact lock ownership/latest state, foreign connection/topology, epoch/native ID rebindings, parent-first joins, stop while acquiring the lock, disconnect, poison and callback-result/admission ownership. Three strict lint configurations, dependency/Node boundaries, formatting and diff checks pass. The initial 426-test selected run predates the last two cases; no browser result is inferred.
[Native close commit-scope contract and evidence](pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Deferred runtime tests cover post-lock reentrant waking, immediate cancellation state, retained result admission, late descendants/calls, existing caller cancellation, lifetime handoff, typed failures, overflow/mismatched profiles, capacity rollback and transport invalidation. Runtime suites pass 113 ordinary, 154 selected and 172 combined; host regressions pass 416 ordinary (8 ignored) and 464 combined (10 ignored). Strict lint, the updated public API doctest, dependency/Node boundaries, formatting and diff checks pass. Host runs preserve existing immediate behavior; no rebuilt addon or browser discrepancy result is claimed.
[Deferred runtime close commit contract and evidence](pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: All 13 new route-retirement tests pass; selected/combined release suites pass 385/462 tests (16 ignored each). Tests compare deletion-only commit with ordinary synchronization and exercise rollback, OOPIF navigation snapshots, native-world proof separation, rejected non-deletion views, subscription state and deferred readiness wake. Retain the intermediate incorrect screenshot-wrapper identity expectation and its corrected inner-service comparison plus original-wrapper retention assertion. Dependency/Node selection boundaries pass; no browser result is inferred.
[Deletion-only close route contract and evidence](pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Final selected composition passes 416 tests (16 ignored), including 22 new scheduler cases; the native leaf passes 38 and the ordinary leaf six. Production-CDP peers cover withheld Page retirement with independent Context dispatch, retained child commands, exact/foreign ancestors, stale routes before/after discovery, capacity rollback and teardown. A ready-slot test verifies fair delivery. Strict Clippy and dependency/Node boundaries pass. The pre-first-poll epoch regression is retained as red evidence. These are native scheduler tests, not a Node/browser compatibility pass.
[Bounded native scheduler contract and evidence](pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Selected composition passes 394 tests with 16 ignored, including 22 new native authority/resource cases using production CDP and bounded peers. Exact domain/epoch/ancestry, generation, rebindings, failures, maximum IDs and complete lease teardown are covered. The real-browser NODE-123 dispatch gate remains unresolved.
[Native ancestor authority contract and evidence](pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Selected native close tests pass 36 (15 new real-CDP/bounded-peer phase cases); feature-cold tests pass six. Leaf and default/combined addon Clippy, dependency proofs, formatting and Node boundary checks pass. These protocol-peer tests are not browser differential evidence; NODE-123 remains the unresolved ancestor-dispatch gate.
[Native close phase contract and evidence](pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Deferred-publication tests exercise both close orders, later visibility/revisions, maximum descendants, group exhaustion, partial rollback, invalid plans, terminal handoff and retained strict commit checks. See close-cascade for final verification evidence; retain NODE-123 as the unresolved browser gate.
[Deferred publication contract and evidence](pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Runtime reservations pass 142 selected tests, 159 with lifetime-handoff and 113 feature-off tests, plus one public API example and strict Clippy/dependency checks. Coverage includes partial admission failure, queue ownership, weak-store teardown, later descendants, both parent/child orders, cancellation/typed errors and transport-closed precedence (retained red/green evidence). This storage verification is not native close or Node event compatibility; preserve the unchanged NODE-123 red dispatch gate.
[Runtime reservation contract and evidence](pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Opt-in topology reservations pass 76 unit tests, including 27 new tests for concurrent admission, latest-state commits, epochs/owners, overflow, byte/metadata failure, rollback and full drain; ordinary topology passes 49. Strict Clippy and selected/ordinary dependency proofs pass. These deterministic storage tests do not replace the still-red NODE-123 browser ancestor-dispatch gate. CI configuration is updated but no remote CI run is claimed.
[Topology reservation contract and evidence](pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Retain transparent/controlled close-cascade reports, the initial Context mismatch and the later combined-profile queued-peer failure separately. Four deterministic tests cover producer/queue loss versus ordinary closed-object replies and queue fullness. Final profiles each pass the 50-case close/event/connection matrix plus two extra six-case repeats; Node units pass 84 and host tests pass 403/451 (8/10 ignored). The candidate ancestor-dispatch control remains red despite successful cleanup. A repeat without a pending Page close does not cover that boundary; these results are not full compatibility.
[Close cascade evidence](pwright-close-cascade.md).

NODE-122: Adds supervised `test:native-close-confirmation`: a loopback relay holds only exact owned-Page retirement events while forwarding command responses. Paired release, owner-close and peer-loss scenarios assert no early settlement and original close emitter/state before success. A Worker case verifies exact owned-Context retirement while its close receipt is held. The gate keeps at most 32 relevant commands/session mappings and eight held receipts; unrelated domain setup is not journaled. Both final artifacts pass 48 close/lifecycle/connection tests; the extra Frame-navigation corpus remains 27/29. Native leaf formatting, unit/type/dependency checks and strict final Clippy pass. Preserve failed setup runs and arrival-boundary failures separately; do not turn diagnostic deadlines into production close policy.
[Selected confirmation evidence](pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Extracts a shared bounded close observer and runs original isolated listener scenarios through unchanged wire forwarding. Eleven endpoint, buffer identity, correlation and overflow tests join the ordinary suite (78 total). Both final 16-sample diagnostic sets and 18-case listener corpora pass; transparent tracing is not scheduling-neutral or proof of a natural-timeout fix.
[Transparent trace and next confirmation contract](pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Adds a supervised real-addon driver-close-attribution lane with one withheld successful Page.enable response, plus a bounded no-delay native close receipt probe. Tests distinguish native execution, response delivery, retained driver failure, exact owned-target cleanup and settled FFI metrics. Do not equate injected failure or later successful repeats with a natural-timeout root-cause fix.
[Close attribution contract and evidence](pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Four masked-batch and ten native ready-source tests cover hidden descriptors, exact predecessors, rejected revisions, failure gates, dependent order, delayed/throwing/interrupted dispatch, duplicate ready, exhausted fresh admission, creation rollback and terminal handoff. Full real-Chrome lifecycle/listener corpora now pass initially. Frame-tree/lookup finalization assertions are updated from obsolete zero revisions to exactly one retained NODE-115 port lease.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Four native tests cover hidden ready-descriptor admission, exhausted binding capacity preserving the notice, binding after complete metadata teardown, and abandoned root/tree lease cleanup. Ten Node tests cover missing/cached identity requests, precommit cache isolation, conversion/listener failures and popup identity/order. Live creation-source evidence remains required.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Nine native external-close tests cover pre-admission exhaustion, stepwise cascade, duplicate receipts, explicit/external non-replay, creation rollback/replenishment, partial admission rollback, terminal masks and fail-closed invariants. Both profiles retain full red corpora: 5/7 lifecycle events and 14/18 listeners, with all 39 close/collection regressions passing.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Added native close-source tests for abandonment, compact Context cascade, incremental flags with listener throw, partial capacity rollback, mismatched commit, raw-flag transfer and terminal-mask handoff. Real Node runs preserve the full red creation/popup/external-close corpus rather than hiding missing behavior.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Added pure JS identity/exception adapter tests, real native terminal listener variants, bounded Worker teardown with queued terminal groups, and twenty retained closed Browsers to prove active port-slot reuse. Page enumeration now expects exactly one retained fixed-port metadata lease after close rather than zero; native runtime and pending publication counts still drain to zero.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Testing Strategy

NODE-114: Added bounded terminal-source tests plus an explicit real-Chrome HostSession.close test. It validates incremental callback-visible state and simulates a first-listener throw through the native acknowledgement API; it does not execute a JS listener. Native cleanup addresses one exact owned Context only.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Three independent-close exception cases extend the isolated listener reference to 14 scenarios. Native group tests cover conversion/commit/ack ordering, failure scope, drop/discard, stale/foreign views, queue saturation and unchanged queue-slot layout. Reference-only success does not establish candidate lifecycle support.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Fourteen native close observer tests cover admission, native failure, exact
borrowed effects, unlock ordering, continued cancellation and network retirement.
An ignored dedicated-Chrome test exercises the selected decorator on an owned
Page and Context, with exact-target cleanup and separate artifact evidence.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: Eleven native observer tests distinguish exact transaction effects from outer
aggregation, deferred popup linkage, snapshot refresh and post-commit failure.
Direct-service close and terminal effects are not covered by this hook. Preserve
the initial test-assumption failures separately from final verification.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 adds isolated lifecycle-listener comparisons, including expected
uncaught exceptions, unhandled rejections and unhandled process exit. Child and
parent bounds prevent a test runner's termination from orphaning an unbounded
child. These tests separate missing callbacks from a real exception-channel bug
and do not claim full lifecycle or async-hooks compatibility.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 tests exclusive acquisition, legacy bypass rejection, exact watermarks,
partial-drop recovery, stale/finished authority, discard, nonce exhaustion,
terminal readiness, held collection batches, application errors and queue lease
release. Rebuilt Node profiles must separately revalidate the existing full-fence
path; they do not exercise unimplemented lifecycle callback dispatch.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 adds bounded native detach/destruction identities to attachment traces:
64 receipts, 128-byte diagnostic fields and at most 32 unanswered IDs per receipt.
Overflow/malformed data fails completeness; no response is fabricated and capture
freezes before cleanup. CDP 91, both probe suites 19, strict checks and Node unit
47 pass. Fresh default/combined probes connect 6/6 each; controlled attachment
retirement pairs pass 6/6 and 10/10 on unchanged NODE-107 addons. This does not
resolve the prior intermittent startup stalls or complete lifecycle event parity.
[NODE-108 receipts and verification](pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 preserves every setup failure separately from behavioral discrepancies.
Default live regressions pass after one recorded creation-connect timeout;
combined Page enumeration/baseURL cannot complete and Browser connection is 4/5.
Its lifecycle lane has four missing-notice failures plus three connect failures;
the final standalone oracle attempt fails all seven during connect. Fresh default
Rust attachment traces pass 6/6; combined traces reproduce four of six timeouts.
Do not treat the diagnostic script's successful evidence capture as six passed
attachments or conflate setup timeouts with the intentional event RED gate.
[Full matrix and bounded command receipts](pwright-node-lifecycle-events.md#combined-profile-reproduction-and-incomplete-live-gate).

NODE-107 adds 20 Rust rebase tests: domain and descriptor identity, partial-view
membership/order, missing/hidden members, equal-but-independent settings,
configured/unconfigured transitions, full-capacity replacement and release after
queue destruction. Tests do not assert a fabricated event order or claim Node
callbacks are wired. Artifact-specific live evidence remains a separate gate.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 adds 14 Rust visibility tests covering immutability, object identity,
parent/child ordering, duplicate/unknown transitions, 16-bit boundaries, empty
views, ordered exact/fused iteration and last-owner resource release. Default
host passes 284 (7 ignored), combined 332 (9 ignored), strict Clippy passes.
Node unit 47, types and 7 host / 8 addon selection checks pass. Rebuilt artifact
regressions and the intentionally red lifecycle gate are recorded separately.
[Detailed evidence](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 adds test:lifecycle-events: seven paired bounded traces of callback
arguments, handler-visible state and settlement order. Final reference passes
7/7; both unchanged artifacts pass 0/7 (all seven fail), despite identical final
state. The external-close Worker supplies independent native confirmation.
Preserve these red receipts; earlier getter tests are not event compatibility.
[Exact cases, limits and next tests](pwright-node-lifecycle-events.md).

NODE-104 adds test:collection-creation with seven bounded paired cases and
a native-creation controller Worker. Corrected reference/default/combined each
pass twice. It distinguishes queued Command results from the Close lane and
within-connection collection order from nondeterministic concurrent invocation
order. Diagnostic observations are separate from public equality assertions.
No production change or Rust retest/rebuild is implied by this test-only checkpoint.
[Evidence, artifact identities and remaining events](pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-103 turns the unchanged seven-case collection gate green on rebuilt default
and combined profiles. It adds four deterministic batch tests, three retirement
pool/rollback tests, paired empty-Context publication and candidate-only
test:context-input-boundary. The latter is an explicit resource/exception boundary,
not a claim that upstream imposes Pwright's byte limits. Initial test baselines
were corrected for cached bindings and connected watch reservations.
Both profiles pass scalar/enumeration/Context/Worker regressions; selected header
creation/getter comparisons also pass. Keep creation/discovery blocked-loop timing open.
[Counts, artifacts and harness corrections](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
NODE-101 red and NODE-102 unrebuilt notes below remain historical evidence.

NODE-102 adds eighteen deterministic Rust collection revision tests for
pre-admission, rollback, fixed/old fences, retained-version bytes, strong-path
absence, identity/count bounds and saturated terminal/discard behavior.
Default/combined host suites pass 263/311; strict Clippy, Node units/types and
dependency boundaries pass. The browser red gate was not rerun because installed
artifacts and public Node collection reads are unchanged; no parity claim.
[Exact results and logical layout measurements](pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 adds test:collection-publication (seven paired cases, exact Page
identities, retained/fresh arrays, surviving order and independent Worker
transport/Page closure controls). Corrected oracle passes 7/7 twice; both
candidate profiles fail the same six blocked-Node removals and pass the live
control. Initial/final observations match; do not report this red lane as
compatibility coverage. Native retirement diagnostics do not poll getters under test.
[Harness correction, bounds and evidence](pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-100 adds test:browser-connection: five pinned Playwright 1.58.2 paired
observations with explicit expected values, real Worker-controlled transport
drop, live and Context-close controls, and exactly-once event assertions.
The red baseline is 2/5; rebuilt default/combined pass 5/5, Page.isClosed 8/8
each, and combined Page enumeration 7/7. Seven deterministic Rust publication
tests complement the bounded browser probes, not exhaustive lifecycle coverage.
[Limits, cleanup and artifact hashes](pwright-browser-close.md#node-100-rust-owned-browser-disconnect-publication).

NODE-099 turns the original eight-case Page.isClosed gate green on final
rebuilt default/combined artifacts, with Page enumeration/reclamation 7/7 each
and the existing Worker teardown subset 3/3 each.
Twelve new Rust cases cover fixed/old fences, full ordinary and terminal queues,
sequence exhaustion, discard, lease retention, foreign permits and startup queue
admission. Final fresh-queue hardening is covered by 238 default/286 combined host
tests and strict Clippy; artifact hashes and full results are in the checkpoint.
Do not equate these tests with Page event ordering or full Worker/API parity.
[Evidence and scope](pwright-page-close.md#node-099-rust-owned-page-close-publication).

NODE-098 adds `test:page-is-closed`: eight pinned real-Chrome observations,
including exact-target renderer crash, borrowing-transport loss and a separate
controller Worker confirming native closure while the subject Node loop waits.
Explicit reference-only mode passes 8/8 twice; both candidate profiles fail 0/8
at the missing method. Do not count this as API coverage. Add deterministic Rust
fence/saturation/discard tests before converting the red gate to conformance.
[Mechanics and limits](pwright-page-close.md#node-098-pageisclosed-reference-contract-and-failing-candidate-baseline).

NODE-097 distinguishes candidate discrepancy from native co-client interference.
The expanded peer lane passes 11/11 combined/default: four shutdown controls plus
seven direct owner comparisons, including original timeout AND explicit recovery.
A raw second client's Page-scoped Target.setAutoAttach reproduces paused-child
retirement even for an upstream owner; root-only and Page.enable controls pass.
Do not replace the old stress assertions with blanket timeout acceptance. Run
capability conformance serially and classify shared-browser stress separately;
retain unmatched failures and the original failed logs.
[Controlled trigger and evidence](pwright-chromium-topology-events.md#node-097-page-auto-attachment-from-a-second-client).

NODE-096 timestamps navigation prerequisites, cancellation and native retirement;
a load observed only after stopLoading must not be called a timely missed event.
The new navigation-peer-disconnect lane compares both owner/peer implementations
with a held native child resume command and exact-peer shutdown. Its combined
control passes 4/4; churn remains failing. Check environment-wide FFI zero only
after ALL candidate Browsers/operations drain, while retaining per-Browser lease
checks. A native OOPIF reattachment may report its already committed URL in
TargetInfo without another frameNavigated; the header gate now accepts that
exact target snapshot, without weakening native iframe ownership checks.
[Evidence, limits and remaining work](pwright-chromium-topology-events.md#node-096-navigation-timing-and-borrowing-peer-control).

NODE-095 extends attachment-retirement with two detach-before-response cases
and bounded child-header failure snapshots. The old addon passes the eight
receipt-based cases but fails both new cases. Overlap baseline is 3/8 child
headers and 34/35 headers. Do not equate receipt-only coverage with detach handling.
Current combined/default attachment gates pass 10/10 and 6/6; expanded Worker
orderings pass 3/3 each. Final overlap passes headers 35/35 but child headers
6/8 twice; final serial child regression passes 8/8. Keep the task-owned preflight
Page through the browser batch: this Chrome instance rejects upstream Context
management after its last normal Page closes, until a new Pwright Page is opened.
Unanswered wire diagnostics are not active Rust pending-call counts.
[Contract and evidence](pwright-chromium-topology-events.md#node-095-native-detach-before-an-initialization-response).

NODE-094 adds a real queued-Context attachment gate: hold a known Page's
initialization, create two controller-owned Contexts/Pages, dispose or retain
them, then release the command. Compare public connection, Page enumeration,
evaluation and borrowed Context survival; do not inject missing-session errors.
[Bounds and evidence](pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-093 extends the real attachment-retirement gate with a per-session command
occurrence, distinguishing lifecycle Network.enable from header initialization.
A 7/8 combined baseline becomes 8/8 after retaining typed native errors;
headers-only passes 6/6. Child-header diagnostic command and active-attachment
maps are capped independently at 128 and record native identity at dispatch.
[Detailed evidence and exclusions](pwright-extra-http-headers.md#node-093-typed-header-initialization-retirement).

NODE-092 adds test:detach-retirement. Hold a real successful Worker resume ACK,
terminate only that Worker, observe its native detach event, then release the ACK
and require the real second-command -32602 receipt. A live-worker control verifies
ordinary inspector detach without Worker termination. The public comparison
checks connection usability, fresh owned Pages and borrowed Page survival, not
full Worker API equivalence. [Evidence](pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach).

NODE-092 targeted default/combined cases pass 2/2 each, combined repeats 2/2,
serial child headers 8/8 and attachment 7/7. Deliberate overlap still fails:
35/35 headers but 4/8 child-header cases, including disconnect/native -32602 and
connection-closed failures. Preserve this failing gate independently from the
verified two-stage cleanup fix.

NODE-091 adds test:header-child-sessions with native OOPIF proof, closure-before-
receipt assertions, actual missing-session receipts, live-error lookalike controls,
main-session failures, wire state and replacement inheritance. The test-only
relay remains loopback-only; the dynamic echo fixture binds Tailscale. Keep
fixture failures, baseline mismatches and final artifact results separate.
[Evidence](pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout).

Final NODE-091 results: combined 8/8 plus repeat 8/8, headers-only 8/8, header
regression 35/35, attachment 7/7, host 274. An overlapping header/child-session
run passed 6/8 and exposed disconnect timeout/native -32602 initialization errors.
Run lanes serially for an isolated baseline, but retain those concurrent failures
as separate reliability evidence rather than dismissing them as fixture noise.

NODE-090 adds `test:attachment-retirement`: hold a native initialization command,
close only the controller-owned Page, release the command and require Chrome's
real missing-session error. Compare connect success, absence of stale Pages,
fresh Page evaluation and candidate disconnect ownership against Playwright
1.58.2. Public gate passes 7/7 plus three 7/7 repeats; headers pass 35/35.
CDP tests pass 78, composition 373 (15 ignored), host 269 (9 ignored), Node 47;
strict types, Clippy, default addon compile and all dependency profiles pass.
The original and intermediate failing runs remain recorded.
[Evidence](pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication).

NODE-089 final expanded gate passes 35/35 and three additional receipt-gated
runs pass 4/4 each. Isolated live Rust cleanup passes; concurrent native attach
and an earlier 34/35 public run failed on a missing session. These failures remain
in the evidence and are not erased by the later pass. Node units pass 47/47;
strict types, Clippy, default/header-only compile and dependency profiles pass.

NODE-089 adds four public Page/Context receipt-gated success/failure comparisons
against actual pinned Playwright and dedicated Chrome. They reproduce native
error-message and stale later-Page discrepancies before the Rust corrections;
all four pass on the rebuilt artifact. Host tests pass 269. Synthetic protocol
failure plus real held ACKs is distinguished from organic crash/close coverage.
[Evidence and exclusions](pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing).

NODE-088 latest release host tests pass 266 (nine ignored), including actual
production early-rejection/drain coordination under held native gates. Combined
public tests pass 31/31, adding two-Page partial failure and subsequent recovery.
This is not yet a controlled real-CDP timing or native shutdown-retirement proof.
[Artifact-specific evidence](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-087 combined release passes 30 public header tests: both Page/main-Frame
invocation orders, overlapping setters and existing conversion/wire/retirement
coverage. Host units pass 257 (nine ignored), Node units 47; strict types,
default/header-only/frame-only compile, Clippy and dependency boundaries pass.
Five focused repetitions pass 6/6 each. The Rust live host verifies both unpolled
invocation orders, eight-slot exhaustion/drop reclamation, native wire behavior,
eighteen Page retirements and zero owners after close. These do not establish
concurrent native fanout or early public rejection during compensation.
[Evidence and open native concurrency](pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch).

NODE-086 retains a failing public regression for unawaited header setter followed
by Frame.goto (two unexpected navigations in eight iterations). The model-only
128-case controlled native-completion comparison passes, as does the old 96-case
sequential comparison. Selected model tests pass 53; eight unselected base tests
are not header proof. Never infer public ordering from internal trace agreement.
The rebuilt combined lane passes 26/26 once, but focused repeats are pass/fail/pass:
the current Frame race remains reproducible. Host tests pass 251 (seven ignored),
Node unit tests 47, type/default/dependency and model/host/addon Clippy gates pass.
[Reproduction and gates](pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model).

NODE-085 combined header/drag/frame tests pass 24/24, including two thirteen-case
Page/Frame referrer comparisons and seventeen repeated pre-dispatch conflicts.
The fixture checks actual Referer values, zero conflicting document requests and
unchanged URLs. Error comparison covers class/first line, not full call logs.
Header-only release also passes 23/23; its only omitted group is the optional
Frame matrix. Host header+Frame tests pass 251 (seven existing ignored), Node
unit tests 47, strict types/default checks and full host/addon Clippy pass.
[Failures, gates and remaining scope](pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation).

NODE-084 expands the header lane to 21 tests: eighteen paired groups and three
candidate-only admission/retirement groups. Both header-only and combined
drag/frame/header release artifacts pass all 21;
235 host tests and 47 Node unit tests pass. Preserve the initial upstream attach
failure with no Page window separately from the successful same-process rerun.
[Detailed evidence and limitations](pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 expands public header coverage with masked invalid strings, empty
Context storage, failed-Page cleanup and recovery, plus 18 repeated failed
creations. Header-only and drag/frame/header lanes each pass 15/15; the combined
case verifies Frame.goto after recovery. Host tests pass 235 (seven existing
ignored), model tests 47, and all-feature Clippy passes. Preserve early-syntax,
attachment-shutdown and incorrect test-assumption failures separately. Upstream
public characterization has 29 cases; internal metadata comparison has 96.
[Commands, artifacts and scope](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 adds supervised `pnpm test:extra-headers`: nine paired public groups
against Playwright 1.58.2 plus one Rust-copy admission/recovery test, all passing
on the rebuilt header-only addon. Preserve the initial two ordering failures and
two conversion-error failures; their corrected pass must not erase that evidence.
The dynamic Tailscale fixture and API-owned Contexts are closed, with host/FFI
zero-owner assertions. Unit 47/47, types, six host tests, selected/combined Clippy
and eight addon dependency/profile combinations also pass.
[Scope, commands and evidence](pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 has two successful native Rust-host Chrome runs: nine header-wire
observations, wrong-kind rejection, 18 Page retirement cycles and zero-owner
shutdown. Preserve `live.log`'s original stale-session failure separately from
the corrected runs. Host tests pass 231 (seven existing ignored), model tests
46, strict selected Clippy and seven host/four addon dependency selections pass.
The test owns its native Context and bounds dynamic HTTP peers and operation
deadlines. These are not paired Node API or live worker/OOPIF gates.
[Evidence and supervisor scope](pwright-extra-http-headers.md#verification).

NODE-080 passes 45 Rust tests and strict selected Clippy. New internal comparison
executes actual pinned upstream setters with injected outcomes: 54/54 desired
metadata/error traces match, including failed compensation; 484 merge/referrer
cases still match. Native state confidence is a separate Rust assertion, not an
upstream field or live-browser result.
[Reproduction](pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-079's three scoped-initialization tests pass, and the full composition
library passes 335 tests with 15 existing tests ignored. They cover reserved
identity/publication, initializer failure and OOPIF ownership using the real
Rust driver with fake native ports. They are not real-browser header tests.
[Evidence and remaining gates](pwright-extra-http-headers.md#node-079-identity-aware-native-initialization).

NODE-078 separates 25 Rust crate tests, 484 exact internal comparisons using
upstream production functions, and 23 public upstream-only Chrome observations
reproduced twice. None is a paired public Pwright extra-header gate. The internal
Rust subprocess is bounded; the public characterization has per-operation
deadlines, not an aggregate supervisor. Native installation remains untested.
[Reproduction and limits](pwright-extra-http-headers.md#reproduction-and-evidence).

NODE-077 expands the Frame lane to 29 cases without removing its failing
server-write close reproducer. Standalone is 27 pass / 2 fail; combined is
28 pass / 1 fail. Both have 27 paired matches, including executed-document
close and three timeout-continuation policies. The differing independent Rust
close outcome is retained as timing-sensitive native evidence, not profile
causation. A new supervised candidate Worker lane passes 3/3 in both profiles,
covering owned pending, borrowed pending and owned queued-result teardown.
It proves exact native identities/borrowed continuity, not a post-Worker memory
measurement. All runners terminate; Node units 47/47, types and boundaries pass.
[Evidence and limitations](pwright-frame-navigation.md#node-077-causal-close-timeout-continuation-and-worker-teardown).

NODE-076 adds public Frame.goto comparison against Playwright 1.58.2, plus
independent compiled-profile combinations and TypeScript coverage. Final single
and combined profiles each pass 23 paired cases plus independent Rust close,
and fail the same upstream Page-close diagnostic deadline. Both exit nonzero;
the initial 22-pass run required a supervisor kill. Neither is a green full
conformance gate. Combined drag/Keyboard/Locator passes 10/10. Host units 239,
Node units 47, TypeScript, selected dependency and strict Clippy gates pass.
The supervisor retains 180 seconds total, bounded
output and native panic/cleanup detection. Diagnostic close deadlines only
bound the harness; they do not alter either implementation's navigation API.
[Current evidence and remaining gates](pwright-frame-navigation.md#node-076-public-node-frame-navigation).

NODE-075 adds projection/admission/lifetime units and extends the actual host
Chrome gate with typed HTTP/hash/404/OOPIF responses. A result is retained past
host close, then dropped to prove complete lease release. The live run passes
in 5.66 s and disposes its exact test Context. This is native Rust evidence, not
a public Node differential gate or proof of exact Playwright error messages.
Final selected host units: 238 pass, seven native tests ignored. Selected and
readiness-only strict Clippy, default addon release check, dependency comparison
and fmt/diff checks pass. Retained addon hashes are unchanged.
[Verification](pwright-frame-navigation.md#node-075-typed-rust-host-result-ownership).

NODE-074 adds navigation 1.2 selected-contract tests: 109 selected Rust tests,
strict all-target Clippy, generated-output freshness, and the retained
15-package navigation-only proof pass. Checks include legacy typed Page bytes,
new Frame envelopes/identity/error/header bounds, both server manifests and a
Frame client/runtime/server/stream roundtrip. CI selects these opt-in features
explicitly. No Chrome or public Node differential gate ran for this checkpoint.
A separate default-feature run passes 32 tests; selected/default server normal
closures match at nine Pwright packages. Fmt/diff checks pass and retained addon
hashes are unchanged.
[Evidence and remaining integration](pwright-frame-navigation.md#node-074-opt-in-typed-navigation-12-contract).

NODE-073 extends the actual Rust host gate with host-owned HTTP/hash/404 and
OOPIF child navigation, plus bounded unfinished HTTP responses for dropped-call,
timeout and host-close tests. Route/counter/deadline units are distinct from
native tests; existing CDP pending/writer cancellation checks cover transport
retirement. 670 Rust units, strict selected/readiness checks and the 131-step
internal oracle pass. The extended final live run passes in 5.09 s, but two
preceding pre-navigation attachment failures remain unexplained; WARN logging
changed timing, not a proven attachment fix. All exact test Contexts were disposed.
[Evidence](pwright-frame-navigation.md#node-073-rust-host-dispatch-scheduling-and-cancellation).

NODE-072 verifies production Page-locked registrations: 358 composition, 226
host and 67 navigation units pass, plus the pinned 131-observation oracle.
Selected/readiness strict Clippy passes. The actual host live test observes
HTTP and hash navigation from its production cursor, and verifies shutdown
with an escaped pending registration. Native initiation is test-owned, so this
is not host-owned command execution or public Node Frame.goto conformance.
The final-source suite also proves pending native changes re-plan earlier idle
deadlines, without self-notifications from snapshot reads. The final live repeat
passes after this fix and disposes its exact Context.
[Evidence and cleanup](pwright-frame-navigation.md#node-072-ordered-operation-registration-and-actual-rust-host-observation).

NODE-071 adds 22 operation units (67 leaf units pass) and an offline pinned
`Frame.gotoImpl`/`Request` differential: 29 traces, 131 step observations match.
The oracle controls native receipts and navigation/lifecycle notices; it does
not prove native normalization, production host integration or public FFI.
Strict release Clippy and default/selected addon source checks pass.
[Reproduction and exclusions](pwright-frame-navigation.md#node-071-bounded-navigation-matching-and-response-retention).

NODE-070 registry gates cover independent Pages, active/retired owner limits,
pre-domain admission failure, reclaimed slots, epoch failure and escaped leases.
The feature-selected host integration test passes with multiple Pages, an
explicitly verified sandbox iframe renderer, dynamic discovery and shutdown
through the actual Rust host. It checks retained leases before final handle drop
and zero leases afterward. Rust suites pass 348 composition plus 226 host tests;
strict all-targets Clippy passes with lifecycle and drag-runtime selected. This
is not a public Node differential. Owned native resources were cleaned up.
[Evidence record](pwright-frame-navigation.md#node-070-bounded-multi-page-registry-and-opt-in-rust-host-selection).

NODE-069 adds selected production-driver tests for initial document seeding,
Page isolation, stale loader/session rejection, sink error, receive cancellation,
event loss, Page close and last-owner lease release, plus native seed/retention
tests. Feature-selected live document tests now also wait for the production
reducer's load/idle and changed document generation. Both native cases pass
after fixing changed-loader snapshot reconciliation on renderer replacement.
Earlier assertion/timeout failures are retained. Final post-fix runs pass 340
composition and 45 navigation units plus 93/93 pinned internal snapshots across
11 traces. Final strict Clippy, formatting and cleanup pass. Successful topology-only runs do not
prove the lifecycle assertion, and none of these gates proves public Frame.goto.
[Verification record](pwright-frame-navigation.md#node-069-opt-in-production-ordered-lifecycle-observation).

NODE-068 adds production driver and topology document-commit tests, including
wrong renderer/epoch, child/main retirement, hash preservation and runtime
rollback. Two live `live_frame_document` tests pass with exact Context cleanup:
same-origin retained-route stability and actual cross-origin session replacement.
The original data-URL assertion failure is retained, not counted as conformance.
Final-source checks pass 49 topology plus 329 composition units, strict Clippy,
default/drag-runtime host/addon compilation and the five-package topology proof.
Fifteen other ignored native unit tests were not run. Addon artifacts are unchanged.
[Verification files and scope](pwright-frame-navigation.md#node-068-production-document-commit-topology-retirement).

NODE-067 adds ten native normalizer unit cases (41 total navigation units) and an
ignored real-Chromium `live_frame_lifecycle` integration test. It exercises actual
CDP HTTP/document/hash events through the production Rust normalizer/reducer with
a test-owned single-renderer route port. The live test has passed; OOPIF/worker
handoff and public Frame.goto differential verification remain separate gates.
Assertion panics are joined and timeouts aborted before exact Context disposal.
An intentionally failing 404 case confirms disposal on assertion panic. Default
navigation units (21), the exact dependency proof (15 Pwright packages) and the
pinned reducer regression (93/93 snapshots) also pass. Temporary native-test
Contexts, preflight target and fixture registration have been cleaned up.
[Commands, retained failures and scope](pwright-frame-navigation.md#node-067-bounded-native-lifecycle-normalization).

NODE-066 adds production Frame-navigation route tests: exact renderer selection,
retained budget, revocation, swap-away/back, pre-poll and in-flight initiation,
terminal seal and topology/runtime rollback. These use the real composition and
resource ledger with an injected fake native endpoint; no new browser/public API
conformance is claimed. The fake Frame navigation reply gate is test-only.
Final verification passes 388 Rust units across four affected crates (including
eleven new route tests), with fifteen native tests still ignored. Strict Clippy,
both host/addon source configurations, 46 Node units, TypeScript and dependency
checks pass. A stale backend-manifest assertion was corrected from 41 to its
existing 42 commands and now explicitly checks reload; no Frame API was advertised.
[Tests and verification status](pwright-frame-navigation.md#node-066-retained-routes-in-the-production-topology-transaction).

NODE-065 adds `pnpm test:frame-lifecycle`, comparing a prebuilt bounded Rust trace
bridge to the installed Playwright 1.58.2 FrameManager with fake Page/time ports.
All 93 snapshots across 11 traces agree. This exercises the real upstream reducer,
not public Frame.goto or native CDP timing. Ten new Rust tests cover document,
subtree, resource, time and invalidation boundaries. No browser was started.
[Commands, report bounds and scope](pwright-frame-navigation.md#node-065-optional-normalized-lifecycle-reducer).

NODE-064 compares six pinned final-state observations against Rust Frame-native
initiation: local/nested children and verified OOPIF, document/hash, referrer and
ancestor/sibling isolation. Diagnostic settlement is outside the tested API;
this does not prove Frame.goto waits. Both oracle and Rust dispose owned Contexts.
An initial mislabeled nested hash case and incorrect topology-rebind expectation
remain in the evidence history, with nonvacuous corrected cases now passing.
[Commands, limits, reports and remaining public gates](pwright-frame-navigation.md#verification-and-reproduction).

NODE-063 adds supervised test:frame-lookup: 41 version-pinned public observations
plus three internal binding/admission/GC groups. It verifies both imported and
new nested topology, true exception identity, reentry and closed/disconnected
lookup. The first GC baseline assertion was incorrect and is retained as failed
evidence; corrected/expanded runs pass. Same-addon Page 7/7, Frame 18/18 and
glob 1647/1647 plus seven checks pass without widening any browser deadline.
Typing passes 38/38 after correcting the fixture from input.html to
sequential-typing.html; the earlier missing-resetTyping failures remain recorded.
[Reproduction and scope](pwright-frame-lookup.md#node-063-evidence-and-remaining-work).

NODE-062 now passes supervised Context baseURL comparison: 16 public observations
and four internal groups covering invalid options, race identity, retained-state
pressure, foreign URL reentry/fallback and last GC. Final-addon Page 7/7, glob
1647/1647 plus seven checks, Node 43 units and strict types pass. Tests run
serially against dedicated Chrome; old failed attachment reports remain intact.
[Reproduction and artifact evidence](pwright-context-creation.md#verification-status-and-remaining-gates).

NODE-061 reuses attach_probe rather than keeping a duplicate collector. Twelve
offline checks cover its bounds, protocol-error response counting and frozen
snapshot. The async layout wrapper caps child output/time and preserves failures.
Original-layout native glob 1647/1647 and Page enumeration 7/7 now pass without
changing the addon; earlier intermittent failures are not erased by that refresh.
[Reproduction and scope](pwright-chromium-topology-events.md#node-061-reuse-the-bounded-attachment-collector).

NODE-060 adds the real native glob conversion gate and a shared 1647-case corpus.
Seven boundary/lifetime checks pass in a labelled late-observer-session run.
Default attachment and public Page regressions remain failing; the expanded
attachment matrix includes extra observer browser-session cases and reproduces
both oracle/candidate timeouts. Passing experimental layout is not a causal fix.
[Commands, limits and failure evidence](pwright-frame-lookup.md#artifact-specific-evidence-and-open-connection-regression).

NODE-059 adds `pnpm test:glob-base`: 1647 exact Rust pattern comparisons with
Playwright 1.58.2 across baseURL, Unicode and exhaustive short token interactions.
Sixteen additional native tests exercise helper semantics and resource rollback.
The diagnostic Rust URL parser has bounded test input but no production parser
allocation proof; this suite is not public Node/browser lookup conformance.
[Reproduction and scope](pwright-frame-lookup.md#node-059-evidence).

NODE-058 compares 2810 real Rust resolved-glob outputs to the pinned compiler,
separate from the 14 oracle-only lookup tests. Native kernel tests cover
membership, callback/lifetime behavior and resource failure; neither suite
establishes public Node lookup or full URL/baseURL parity. The native notice-copy
helper has an exact-byte packaging test.
[Commands and scope](pwright-frame-lookup.md#evidence-and-scope).

NODE-057's optional Mouse background setup and original control both pass 44/44
on the unchanged addon. This does not prove a causal fix for the previous extra
movement. Preserve both failing and passing traces and label setup in reports.
The new focus probe preserves partial samples on normal failures.
[Setup evidence](pwright-raw-mouse.md#node-057-setup-experiment-not-a-causal-fix).
Frame lookup's 14 offline [oracle characterization tests](pwright-frame-lookup.md)
pin client algorithm behavior, not candidate or real-browser conformance.

NODE-056 extends the shared public Frame gate to 18 local/OOPIF cases, including
name mutation versus navigation publication and retained values after close.
Native tests independently cover name/URL/event quota rollback and last-reference
release. Keep the failed fixture attempt and rejected oracle assumption visible;
the final paired result does not establish Frame-event timing or all API parity.
Mouse regression remains failing on this artifact. The new zero-input idle
diagnostic records possible contamination without filtering the conformance
trace, changing production input code or declaring a retry a fix.
[Evidence and scope](pwright-frame-publication.md#node-056-bounded-navigation-names-and-public-framename).

NODE-055 adds shared public Page enumeration with uncached popup discovery and
stale-cache pressure, plus four-state pre-creation admission and last-wrapper
release. A failing popup return value identified missing activation rather than
justifying removal of the discovery case. Typed CDP and evaluation-profile tests
cover the opt-in policy; logical accounting is not claimed as V8/RSS measurement.
[Evidence and scope](pwright-frame-publication.md#node-055-admitted-page-descriptors-and-cache-aware-enumeration).

NODE-054 preserves a real public resource-failure reproduction before fixing
admission ordering: root binding exhaustion must reject before target creation.
The same bounded workload now has zero target/ledger delta, with exact Context
cleanup. Two native tests cover rollback and exact-limit activation. Keep
diagnostic attachment matrices distinct from paired conformance and do not
treat successful retries as a fix for intermittent connection timeouts.
[Evidence and commands](pwright-frame-publication.md#node-054-creation-time-root-binding-admission).

NODE-053 tests the same public Frame workload on the pinned oracle and actual
selected addon: ten settled local/OOPIF observations, nested removal and close,
1000 repeated enumeration loops, exact owned Context/target absence and native
metadata finalization. Six Rust tests exercise admission and last-reference
release; four JS tests cover weak cache identity and parent retention. Polling
barriers are explicit and do not count as Frame-event-order conformance.
[Contract and evidence](pwright-frame-publication.md#node-053-public-node-frame-collections-and-retained-wrappers).

NODE-052 keeps the old-addon owned-OOPIF failure as provenance, then verifies
the rebuilt selected artifact and real Rust Frame projection. Native tests
compare stable publication points, inspect renderer boundaries and audit exact
owned Context/target removal. A pending-world fake gate and serialized CDP
child/parent envelope test prevent the two discovered ordering/routing defects
from being hidden behind successful metadata-only unit tests.
[Evidence and scope](pwright-frame-publication.md#node-052-live-host-projection-and-paused-oopif-attachment).

NODE-050 separates 18 internal native Frame lifecycle cases from actual selected
Node API results (Mouse 44/44, Keyboard 24/24, drag/Locator 10/10). Artifact
reporters share the loader's fixed profile selection and hash the selected binary.
The new drag runner and Rust oracle share one public-action implementation.
[Selection, evidence and remaining work](pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049 expands the Frame catalog to 12 cases, including finite completion
after detach with navigation waiting enabled. Both sides reject pre-removal
keyup; fail-fast Rust try-join exposes action errors instead of masking them
behind the dragend observer. The fixed native gate matches 12/12; earlier failed
runs remain recorded, not relabeled as successes.
[Contract, evidence and remaining gates](pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048 adds a shared ten-case Frame catalog and mandatory native process proof.
The local/OOPIF detach cases remain failing; the harness runs all cases, audits
owned cleanup even on failure, and exits nonzero (8/10 matches). Oracle behavior
is measured before asserting the Rust error contract, never normalized to it.
[Evidence](pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).

NODE-047 extends the shared drag/Keyboard catalog to ten cases, adding Locator
Escape/chord, sequential typing and delayed-press timeout/repeat. Full native
snapshots are compared before host close. Controlled Frame/OOPIF routing tests
must not be reported as real child/OOPIF browser proof.
[Evidence](pwright-chromium-drag.md#node-047-resources-and-evidence).

NODE-046 adds five pinned drag/Keyboard cases, with bounded DOM key recording.
The actual host compares full snapshots BEFORE close, then audits borrowed Page
survival and fixture removal. Gated tests separately prove interrupted native
cancel retention; a passing close-cancel test alone cannot prove Escape behavior.
[Evidence](pwright-chromium-drag.md#node-046-verification).

NODE-045 expands the shared raw Mouse corpus from 22 to 25 with count 15/16/32
boundaries. The selected host passes 25/25; the rebuilt default public Node gate
correctly fails at 39/44, retaining all five same-turn discrepancies. Gated fake
clients prove 520-owner admission/retirement, not full native concurrency parity.
[Results](pwright-chromium-drag.md#node-045-verification-and-actual-node-artifact).

NODE-044 adds `live_drag.rs`: actual HostSession drag/drop and close-during-drag
checks, borrowed-Page survival, exact test-owned Context cleanup and zero host
leases. Ordinary Mouse is compared against a fresh pinned 1.58.2 oracle;
these Rust-host checks do not substitute for an addon rebuild/public Node run.
[Checkpoint evidence](pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration).

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

NODE-039: **13** new composition tests (**57** drag total), strict release
Clippy, **85/17/38** adjacent regressions and **9** native scenarios pass. Three
new native cases exercise composed ordinary/native/prevented movement; multi-world
fairness and failure/cancellation coverage are deterministic tests, not native
multi-Frame conformance. Public Node gates are not rerun.
[Evidence and limits](pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038: **17** new capture tests (**44** drag total), strict release Clippy,
adjacent input/native/controlled-Mouse **85/17/38**, and the **6** real Chrome
listener cases pass. Native cases now enable/receive/disable through the capture
owner; movement/drop remain direct scaffolding. Cancellation, lag and cleanup
failure coverage is deterministic fake-session coverage, not native fault
injection or public Node parity.
[Evidence and limits](pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037: **14** new data-admission tests (**27** total drag tests), strict
release Clippy and the existing **6** native listener scenarios pass. NativeDrag
now delivers actual captured data through bounded typed storage, verifies text
and checks record release. This is one data-flow case, not six additional
comparisons; no public addon gate was rerun.
[Evidence and accounting limits](pwright-chromium-drag.md#node-037-verification).

NODE-036: drag observer **13** deterministic tests plus **6** main-frame native
Chrome cases, including prevented/native dragstart, actual intercepted/drop data,
listener-count restoration, parallel tokens and collision-safe no-install
receipts. These are not full public Mouse differential tests; the addon is
unchanged. [Evidence and limits](pwright-chromium-drag.md#verification).

NODE-035 adds **13** tests for typed Page selection, standalone button ownership,
physical suppression/logical state, deferred cleanup and consumed-receipt
cancellation acknowledgement. Input/native/route totals are **85/17/38**.
Fresh Chrome lanes pass default **22/22** and explicitly selected passthrough
**22/22**; counters verify all four delegate request kinds ran. This is not
real drag interception or a rebuilt public Node artifact.
[Evidence](pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

NODE-034 uses explicit setup/cleanup gates to test optional Mouse dispatch
cancellation, separate error receipts, slot reuse/wakeups and retained Page
readers. Input **81**, native **13**, route/runtime **33**, host **115 / 6 ignored**
and strict release Clippy pass. These tests use a fake multi-phase provider,
not Chromium drag interception. No new addon artifact/public gate is claimed.
The changed Rust source also passes fresh **22/22** HostSession browser
comparisons and close-during-click-delay cleanup on dedicated Chrome 152.
Those browser tests use the default direct dispatcher, not the fake delegate.
[Checkpoint evidence](pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

Expanded Mouse gate (NODE-033): **41** cases, including five required repetitions
of each same-JS-turn submission sequence. The prior addon is **33/41**, Rust
first-poll ordering reaches **36/41**, and five drag-path mismatches remain red.
Never insert awaits, sort native events or select only matching repetitions.
The seven new deterministic host tests force reverse polling and check queued
body cancellation, ownership/capacity and wakeups; host total **115 / 6 ignored**.
[Exact scope](pwright-raw-mouse.md#same-turn-input-ordering-node-033).

Final NODE-033 artifact: **36/41** Mouse; **3/3** Mouse profile/Worker;
**24/24** Keyboard; **30/38** typing; **24/27** actions; **12/12** screenshot;
**45/45** lifecycle. All failed typing/action cases stop during connection setup
(5 candidate topology-init, 6 oracle connect). Intermediate green results are not
substitutes. Final reload/shared were not rerun; prior failures stay open.

Public Mouse addon `e9d9e543...109535`: **26/26** paired cases, **3/3** native
profile/Worker tests, **34/34** offline tests and strict types/Clippy pass. The
paired lane verifies owned native Context/target absence; Worker termination
must follow confirmed mousedown, preserve a borrowed Page without synthetic
mouseup/click, and remove a worker-owned Context/Page. Keep exact error fidelity,
wider concurrency, nested allocation audit and full artifact regressions separate.
[Public evidence](pwright-raw-mouse.md#public-node-mouse-checkpoint).

Same Mouse artifact: Keyboard **24/24**, typing **38/38**, actions **27/27**,
screenshot **12/12**, lifecycle **45/45**, reload **16/18**. All 57 shared cases
finish: **53 match / 2 unsupported / 1 oracle-error / 1 setup-error**. The shared
supervisor correctly fails on the candidate 5 s topology-init timeout (NODE-013),
without native panic, output overflow or supervisor timeout. Preserve this result;
do not substitute a later narrow pass for the failed full gate.

Earlier safe host Mouse gate: **9** new tests (**108 pass / 6 ignored** host total),
**22/22** real HostSession event/state comparisons, and a close-during-delay
probe verifying TargetClosed, no synthetic release and zero host leases/tasks.
Use an observer-owned borrowed Context for post-close DOM assertions: a
host-created Context can disappear automatically on detach and is not evidence
of no mouseup. The host closure is **234**; addon compile-check, strict Clippy,
offline Node **31** and types pass without rebuilding the addon.
[Host evidence and remaining FFI scope](pwright-raw-mouse.md#host-checkpoint-evidence).

Optional Mouse runtime gate: **15** additional deterministic tests (**33** with
the default route tests) prove deadline/cancellation receipt waits, abandonment,
transport loss, delayed-click cancellation without synthetic release, resumed
and competing waiters, exact identity and all six typed routes. Fresh controlled
Chrome comparison passes **22/22**, with reply/receipt identity and runtime/route
ownership checks. It is separate from binary-wire and Node API coverage.
Default/runtime closures are **16/17**, with no new executor or full host.
[Controlled evidence and embedding requirements](pwright-raw-mouse.md#controlled-checkpoint-evidence).

Mouse Page-route gate: **18** release tests cover generated wire handlers,
cancelled/abandoned responses, exact identity, retained receipt quotas, fair
retirement, independent driver ownership and separate cleanup failures. A fresh
**22/22** Chrome comparison now exercises binary wire -> Page route -> native
Mouse and checks reply/receipt identity and released records. It does not prove
controlled runtime, Node API, concurrent error timing or HTML drag support.
Strict all-target Clippy and the new **16-package** composition proof pass.
[Route evidence and ownership limits](pwright-raw-mouse.md#page-route-checkpoint-evidence).

Typed native Mouse factory gate: **13** release tests and **22/22** fresh browser
observations pass through the six starter APIs. Tests preserve the native owner
across cancellation/error, retire before Context cleanup and check admission
release. Existing input/Keyboard/Locator/root/host regressions remain green.
This does not cover wire/FFI route retirement, provider allocation bounds or
concurrent first-error ordering. [Native evidence](pwright-raw-mouse.md#native-checkpoint-evidence).

Mouse contract gate: **3 capability / 2 client / 18 codec / 1 server** tests
pass in release mode. The new `scripts/check-v2-mouse-contracts.mjs` checks exact
normal closures, global method ownership and generated-output freshness in CI.
It validates the portable layer, not native/FFI Mouse conformance.

Raw Mouse gate: **22/22** exact event/state comparisons on fresh Playwright
1.58.2/Chrome 152, now including click/dblclick, click counts and positive delays.
Input **71**, model **6**, fake **23** and strict Clippy pass. Deterministic tests
check pending-tail retirement separately from DOM snapshots. Native commands,
fixture waits and click result polling are bounded individually; no outer case
timeout may discard a retained click owner. This is not Node Mouse coverage;
the existing shared `touchscreen` case uses mouseClick, not true touch input.
[Evidence, limits and next gates](pwright-raw-mouse.md).

The raw CDP isolation lane now reproduces the close/commit symptom in two runs
without host, navigation service, runtime or addon. Its response-boundary target
survives both 200 ms and 1000 ms samples; these are observation windows, not API
timeouts. CDP **73 + 11 + 3** offline tests and strict release Clippy pass.
[Minimal recipe and limits](pwright-page-reload.md#minimal-raw-cdp-isolation).

New test-only reload diagnostic correlates bounded loader/request events and
native target presence. Direct `Target.closeTarget` can acknowledge while the
target survives reload timeout even when host Page close is bypassed. Earlier
successful controls and stopLoading failures remain in the evidence; diagnostic
integrity passing is **not** parity passing. Host **99 pass / 6 ignored**, strict
release Clippy and Node **31/31** pass without a new addon build.
[Controls, bounds and reproduction](pwright-page-reload.md#causal-native-close-diagnostic).

Same-artifact full rerun: typing **38/38** (all held-modifier catalog entries
restored for both APIs), actions **27/27**, screenshots **12/12**, lifecycle
**45/45**, reload **16/18**. Evidence: `.work/node031-regression.iGmeff/` on
`1319fa02...ded25dc1`. Reload's two original close-boundary failures remain;
intermittent attachment/window causes remain open despite this run passing.

Latest public Keyboard gate: **24/24** against Playwright 1.58.2 on addon
`1319fa02...ded25dc1`, including exact errors, navigation, Locator state sharing,
Page isolation and active close. Shared 57-case coverage is **52/3/2**, no other
errors. Host **95**, root **295**, Node contracts **31**, shared verifier **30**
pass. Evidence: `.work/node031-keyboard-routing.oTrp98/`. Earlier primitive
checkpoints below are historical; full reload/lifecycle/screenshot gates were
not rerun and cannot be inferred green from shared subsets.

## Raw Keyboard primitive gate

`probe-raw-keyboard.mjs` and the ignored Rust `live_keyboard` target match
**14/14** pinned raw Keyboard event/state cases. Input tests pass **42/42** and
Locator regression **103/103**. This is a native primitive result, not public
Node Keyboard support; the shared corpus/addon remain unchanged. Owned Context
cleanup and native absence are required before comparison failures return.
The subsequent typed contract gate passes **3 capability / 2 client / 13 codec /
1 server** tests. Generated five-command dispatch and exact dependency/freshness
checks are covered. The optional native backend subsequently passes **18** tests
and **14/14** fresh typed Chrome comparisons with zero post-case operation leases.
The optional single-Page composition subsequently passes **11** tests and **14/14**
full wire-to-Chrome cases with no retained route/operation/frame leases after each
attempt. Aggregate host lifecycle routing and public Keyboard FFI remain unverified.
The opt-in controlled adapter adds seven deterministic tests (**18** composition
tests total), including exact Page validation, disposal, queued revocation,
abandoned cleanup and both deadline policies. A fresh regenerated-wire Chrome
comparison remains **14/14**; this does not test the runtime adapter on Chrome.
Both default/runtime closures (**16/17**) and release adapter tests now have CI
gates. Evidence: `.work/node031-keyboard-routing.oTrp98/`.
Independent Page-state admission adds four input tests (**46** total) and seven
aggregate tests (**294** passing, 15 ignored), with **89** host tests (5 ignored).
A real-host attach/close test verifies zero leases/operations/cleanup after close.
It does not replace Keyboard command lifecycle or same-addon differential gates.
[Scope, evidence and reproduction](pwright-raw-keyboard.md).

## Native reload primitive gate

The subsequent public `test:reload` lane now invokes the typed 1.1 Rust command
through FFI. Current **16/18** retains both oracle Page.close waiting and native
TargetStillOpen failures for a held-response arrival boundary. A separate close
case passes after a beacon proves the replacement document executed, with the
body still held; it does not replace the original failing gate. POST method/body,
ordinary reload, history, document replacement and timeout pass. A diagnostic
close bound ensures failure reaches cleanup; it is not an API timeout override.
Shared results improve to **48/7/2**, while the same artifact's full lifecycle
gate is **44/45**, NODE-014. Its screenshot gate is **0/12**, failing during
attachment before image comparison (62.496 seconds, clean failing exit).
Preserve these red gates separately from old green artifacts. Final
backend/client/codec tests are **14/6/22**.

The separate `probe-reload.mjs` and ignored Rust `live_reload` target compare
12 blank/data/HTTP reload observations under all four wait policies against
Playwright 1.58.2. They pass, alongside 13 navigation-leaf unit tests and the
unchanged 15-package navigation proof. This is not public Node FFI verification;
These primitive results do not substitute for the public gate above.
[Commands, bounded ownership and evidence](pwright-page-reload.md).

## Sequential input primitive gate

The separate public `pnpm --dir packages/pwright test:typing` lane now invokes
both Locator aliases through the addon. The opt-in interleaved artifact now
passes 30/30: exact same/different-key overlap events, missing-target timeout
during another held-key delay, and Page/Worker cleanup are required checks.
The same artifact passes 27/27 actions, including NODE-022 and explicit
literal-versus-physical Shift cases. Preserve the older 27/28 exclusive-mode
failure and initial interleaved 25/26 action result; neither an observation
timer nor a successful cleanup count is proof of native retirement or complete
compatibility. See [artifact-specific results](pwright-sequential-typing.md).

The same artifact's earlier broad lifecycle run is **incomplete**, killed at its
180-second supervisor with 18 passed / 3 failed completed cases of 45. The
following screenshot gate fails 12/12 in attachment, then passes a 12/12 recheck
without artifact change, browser restart or successful intervening cleanup.
Preserve both outcomes; do not replace the failed full lifecycle gate with a
green input subset. An exact orphaned fixture Page was cleaned manually only
after all runners ended, which does not satisfy automated teardown coverage.

The subsequent phase-traced run completes 44 passing cases before the same
180-second aggregate limit kills the final candidate connect after about 3.4
seconds, before its 30-second API deadline. The lifecycle supervisor therefore
now has 600 seconds total and an independent 60-second case/setup/cleanup bound;
startup, between-case and exit waits are also bounded. Only validated case
start/end IPC can replace that timer; phase output cannot renew it. Other lanes
retain 180 seconds, all API deadlines/assertions are unchanged, and stdout is
bounded to 4 MiB. The resulting same-artifact full rerun passes **45/45 in
192.715 seconds**, with clean supervisor exit, alongside **29/29** offline tests,
TypeScript and the unchanged 224-package safe-host boundary. Preserve earlier
red evidence; a complete rerun does not establish an attachment/window fix.
[Logs and limits](../node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).

`packages/pwright/scripts/probe-sequential-typing.mjs` records a fresh pinned
Playwright 1.58.2 oracle from the serve-lib `sequential-typing.html` fixture.
The input backend's ignored `live_typing` target compares 13 identical text,
modifier and delay cases on dedicated Chrome, asserting exact event fields,
order and final values after owned-context cleanup. Final result is 13/13;
retain the first modifier-location failures and the later test-only version
guard failure. Release coverage is 27 input/model tests plus 103 Locator
regressions. This is **not** a public Node API gate or a timing/RSS proof.
[Commands, ownership and remaining gates](pwright-sequential-typing.md).

## Node screenshot gate

`pnpm --dir packages/pwright test:screenshot` uses the same dedicated Chrome and
serve-lib fixture environment as the other supervised live lanes. Twelve tests
decode PNG/JPEG and compare dimensions/pixels, edge clips, concurrent buffers,
post-close Buffer ownership, queued Page-close settlement and Worker destruction
at queued-native/publication boundaries. The recursive-attachment artifact now
passes **12/12**, including both Worker cases; the preceding trace-only artifact
failed 3/12 before the Worker assertions. This is broader in scope
than the unchanged shared corpus's two PNG-header observations, but does not
cover fonts/caret/animation transactions. Keep the first 7/9 and subsequent 9/10
failures alongside the handoff-stage 10/10 result and final-artifact 8/10
attachment failures. Current shared verification is 45 match / 10 unsupported /
2 oracle-error with zero setup errors, and lifecycle passes 45/45 on the same
artifact. The previous 21-setup-error shared report is retained separately;
none of these results establishes full compatibility. [Current evidence](../node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

The `live_frame_evaluation` integration target contains serial/main-world-only
and recursive-join/utility-world OOPIF cases. Run with its existing explicit CDP
and served fixture variables, `--ignored --test-threads=1`, after the dedicated
Chrome/Pwright preflight. Both pass; the selected utility fixture needs four
world slots instead of the original main-only two. Preserve the first 1/2
budget failure and corrected 2/2 run; no production Node limit was raised.

## Retained-world diagnostic

`packages/pwright/scripts/probe-retained-worlds.mjs` is a separate six-attachment
diagnostic on one owned blank Page, with fixed five-second connection deadlines,
bounded main-frame metadata, sticky overflow failure and explicit fixture cleanup.
Five collector tests are included in the Node package's 14-test offline lane.
Its current completed result establishes context counts and a changed main-world
identity after reload, not a memory budget, timeout fix or conformance pass.
See [recorded evidence and limitations](../node-ffi-compatibility-design.md#node-013-retained-world-diagnostic-2026-09-07).

## Public Locator evaluation gate

`pnpm --dir packages/pwright test:locator-evaluation` compares delayed source/
getter conversion, missing/strict failure, timeout scope, selected element/main
world, binary microtasks, exceptions and rich values against Playwright 1.58.2.
It also verifies bounded Node roots and Worker teardown in raw, binary, running
and queued-result phases, including exact owned Context/target removal.
The final host-retirement build passes 23 assertions and its supervisor. The
added case terminates a Worker while it evaluates on a borrowed default-Context
Page, verifies Page/Promise survival, then resumes that Promise independently.
A preceding full run remains 22/23 on NODE-013 attachment timeout; lifecycle
records NODE-014 and its unchanged total-deadline kill. Keep those failures and
all unchanged watchdogs. Passing TAP alone never overrides a native diagnostic.
See [current evidence](../node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

## Native cancellation boundary verification (2026-09-07)

NODE-027 native boundary checks are separate from the still-red Worker lane:
`cargo test --release --locked -p pwright-composition-chromium-locator-evaluation
--lib live::cancellation -- --ignored --test-threads=1` explicitly runs both
Chrome tests after the required dedicated-profile preflight and with
`PWRIGHT_CDP_WS_URL` set to the verified loopback endpoint. They use causal
Promise gates, not elapsed-time readiness. One proves late native exceptions
can outlive an early group release; the other verifies Page/Promise survival
after exact owned-session detachment. The leaf's deterministic cancellation
test retains native accounting until settlement. Current evidence is under
`.work/node027-native-proof.HY4spM/`: 42 offline tests and 2 live tests pass.
These passing boundary tests must not be substituted for the failing Node
cleanup supervisor or interpreted as an implemented cancellation fix.

## Receipt-driven evaluation verification (2026-09-07)

Receipt implementation follow-up: `.work/node027-retirement-leaf.VvZ61y/`
records 21 raw-resolution and 51 evaluation/preparation release tests, plus all
six explicit Chrome tests (`--lib live:: -- --ignored --test-threads=1`). The new
native test drains cancelled Locator cleanup after exact owned-session detach,
then resumes the unchanged borrowed Page's user Promise through another session.
This exercises the production leaves but uses a fixture-owned receipt producer;
it must not substitute for the pending actual Node-host/Worker integration.
Strict all-target Clippy and the unchanged 15-package dependency proof pass.

## Three-way session registration verification (2026-09-07)

The optional Rust initialization join is tested independently from browser
timing: deterministic gates cover each pending/error branch, caller cancellation
and exact production-profile accounting. Real driver tests preserve Context,
world-creation, recursive-attach/resume and ready-publication ordering, including
world failure, frame disappearance and absent utility-world capability.

Fresh release/browser evidence is retained under
`.work/node-attach-phase.j2k3aV/`: 272 Chromium composition + 77 host offline
tests, strict Clippy, 9 Node offline tests, TypeScript and the unchanged
224-package dependency boundary pass. The native host live lane passes 5/5;
complete Node lifecycle/evaluation/Locator lanes pass 45/45, 38/38 and 32/32.
The command-level overlap probe passes separately after the first attempt's
Rust subprocess timed out on a Cargo build lock (its failure log is retained).
Do not run Cargo-based timed probes concurrently with compilation or Clippy.
All conformance deadlines remain unchanged. Actions/shared were not rerun here;
no full compatibility, RSS or NODE-013 root-cause resolution is implied. See
[the canonical checkpoint](../node-ffi-compatibility-design.md#node-013-three-way-registration-checkpoint-2026-09-07).

## Rust v2 Node-FFI verification

The native-retirement checkpoint fixes the reproduced NODE-026 Context-close
cleanup failure. Final epoch-guarded sources pass 442 release tests (23 explicit
live cases ignored), including thirteen new proof/budget/reconnect tests. The
pre-guard real safe-host cases pass twice (3.75 s and 1.61 s), plus the Context-close case passes alone.
These cases verify authoritative cleanup and shutdown, not just TargetClosed.
Strict Clippy, workspace all-target check, Node offline 9, shared harness 30,
both type checks, 5/7/15 closures and 30 umbrella profiles pass. Default receipt
selection is absent; the safe-host boundary remains 224 normal packages.
Final addon: `930469ad...a7a4a3`. Its real-host run is **4/5**: the queue case
fails during 5 s TopologyInitialization, before queue assertions, while Context
cleanup passes. A separately recorded isolated queue diagnostic passes; do not
replace the failed gate with that diagnostic. Logs:
`.work/node-native-retirement.bA2K7k/`; [current complete record](../node-ffi-compatibility-design.md#native-destruction-receipt-checkpoint-2026-09-07).

Final public lanes are Locator 29/32, evaluation 17/38, lifecycle watchdog after
11 passes / 5 failures (remaining 29 not completed), actions 25/26 (NODE-022),
shared 41 matches / 14 unsupported / 2 oracle errors. Candidate and oracle
initialization failures are retained, not attributed to a guessed cause.
The shared run completes all 57 pairs with no value/candidate/setup/cleanup
errors; its supervisor exits 1 without timeout/signal/overflow/native failure.
All runners ended. Exact owned preflight and watchdog fixture targets were
closed; unrelated Pages remain. Legacy CLI `tab-close` expects its saved alias,
not a native target ID: verify target disappearance rather than its success text.

Previous host-owned checkpoint (`8cc20b9e...6aad0`) passes 77 host units, strict
Clippy, workspace check, 5/7/15 closures, 30 umbrella proofs, both type checks,
Node offline 9, shared harness 30 and the 224-package safe-host boundary.
Expanded real-host Chrome tests are **4/5**: delayed conversion/identity,
missing/strict/error cleanup, real eight-lane contention and caller cancellation
pass; native Context-close cleanup fails NODE-026. The ordinary unit run ignores
these five live tests; it must not be reported as a full green gate.

All five unchanged Node lanes run serially once: Locator 32/32, evaluation 38/38,
lifecycle 45/45, actions 25/26 (NODE-022), shared 41 matches / 14 unsupported /
2 oracle errors. Shared has zero value/candidate/setup/cleanup errors, supervisor
status 1 with no timeout/signal/overflow/native failure. Node does not expose
Locator.evaluate, so existing public lanes do not cover the new host cleanup gap.
Logs: `.work/node-owned-locator-evaluation.AbFw0k/`; [full record](../node-ffi-compatibility-design.md#host-owned-locator-evaluation-checkpoint-2026-09-07).

Preceding event-admission checkpoint (`ab1a0857...6f7d9a`) remains **not green**:
Locator 32/32, evaluation 38/38, lifecycle 39 reported passes before its
180-second supervisor kill, actions 17/26, shared 33 matches / 11 unsupported /
2 oracle errors / 11 setup errors. All 57 shared pairs completed under their
separate ten-minute supervisor (exit 1, no timeout/signal/overflow/native failure).
Both oracle and candidate attachments time out; no silent retry replaces these
reports. Shared's lower unsupported count is not increased capability coverage.

Release CDP 73, aggregate composition 254, resolution/evaluation 37/41, probe 6,
Node offline 9 and shared harness 30 cases pass, alongside strict Clippy,
workspace release check, both type checks, 5/7/15 leaf closures, 30 umbrella
profiles and the 221-package Node boundary. The first separate live Rust probe
attaches in 381 ms with 1,440 pre-admission omissions, 32 retained world-created
events and zero recorded host resources after close. This scoped result does
not resolve native-world retention, RSS or general attachment delays. Logs:
`.work/node-world-event-admission.SCODws/`; [full artifact record](../node-ffi-compatibility-design.md#optional-world-event-admission-checkpoint-2026-09-07).

Preceding controlled-dispatch checkpoint: 113 runtime, 247 aggregate composition,
31 runtime-control, 16 Tokio and 41 evaluation-composition unit cases pass.
All three leaf Chrome tests run serially and pass, including real shared Frame
queuing/timeout and physical temporary-group reclamation. Five bounded attach-
probe tests pass. Logs: `.work/node-controlled-preparation.vZZhDf/`.

The rebuilt addon `3ff763b2...7594ba` completes all five serial full gates:
Locator 32/32, evaluation 38/38, lifecycle 25/45, actions 0/26 and shared 57 setup
errors (zero matches). Lifecycle has NODE-014 and 19 attachment failures;
all action/shared candidates fail setup. Two independent Rust-host diagnostics
reproduce event-item exhaustion during Runtime.enable; the bounded method-count
probe records 1,012 execution-context creation events. Both diagnostics fail,
not pass. No retry replaces the gate and no browser restart/cap increase occurs.
The earlier green-subset evidence below is historical, not this artifact's
result. See [the full NODE-025 record](../node-ffi-compatibility-design.md#controlled-preparation-dispatch-and-attachment-pressure-2026-09-07).

The next Rust-only preparation phase passes 37 resolution-composition and 41
evaluation-composition unit cases, including five new route-retention and
seventeen preparation cases. Both separately invoked real Chrome cases pass
serially: the new one composes the actual paired-world access port and driver,
waits for a delayed hidden element, preserves detached identity, evaluates once
in main world and checks native references before/after group release. Strict
Clippy, workspace release check, 5/7/15 leaf closures, 30 umbrella profiles,
Node offline 9, types and the 221-package Node boundary pass. Logs are in
`.work/node-element-preparation.YxRNYz/`. Node does not select these leaves;
its addon and preceding five-lane evidence below are unchanged, not new runs.
See [the scoped checkpoint](../node-ffi-compatibility-design.md#paired-element-preparation-checkpoint-2026-09-07).

The preceding paired-world topology rebuild uses addon
`72644cf9eb2285380af2546833f18796dae7b5ee407864b83ee5cb4471b9ddcd`.
The first overlapping package/shared run failed: Locator 27/32, evaluation
34/38, lifecycle 43/45, actions 25/26, shared 34 matches / 10 unsupported /
2 oracle errors / 11 setup errors. Missing native sessions and initialization
timeouts remain NODE-013 evidence, including a nested cleanup error inside one
shared setup failure. The lower unsupported count is not more API coverage.

A separately recorded serial run on the same binary/source/fixtures completes
Locator 32/32, evaluation 38/38, lifecycle 44/45 (NODE-014), actions 25/26
(NODE-022), shared 41 matches / 14 unsupported / 2 oracle errors. Serial shared
value/candidate/setup/cleanup counts are zero; supervisor exits 1 without a
signal, timeout, overflow or native failure. Logs are in
`.work/node-frame-worlds.qDq1XE/`, with `serial-*` and `shared-serial/` kept
separate from the first failing evidence. Run browser supervisors serially;
this is workload isolation, not a production attachment-race fix.

The new independently installed paired-world access port passes 20 new unit
cases, plus three authoritative-renderer topology cases. Full affected units
pass topology 40 and composition 247 (15 browser cases ignored). One separate
real Chrome test verifies the production driver, world replacement, local-child
detach and owned Context cleanup; live OOPIF coverage is not claimed. Release
Clippy, workspace check, 30 umbrella profiles, 5/7/15 leaf closures, Node offline
9, shared unit 30, both type checks and the 221-package Node boundary pass.
Node does not install the port or expose Locator.evaluate yet. See
[the full artifact record](../node-ffi-compatibility-design.md#production-paired-world-access-checkpoint-2026-09-07).

Previous CDP/Handle regression uses addon
`42a26fd0b5325c393b930a89f18dc628959c11bfa884e293756fb19c9d734e94`.
All five gates complete: Locator 32/32, evaluation 38/38, lifecycle 44/45
(NODE-014: Rust outer window 1282x800 versus Playwright 0x0), actions 25/26
(NODE-022), shared 41 matches / 14 unsupported / 2 oracle errors. Shared
value/candidate/setup/cleanup errors are zero; supervisor exit 1 has no signal,
timeout, overflow or native failure. Logs/report are in
`.work/node-element-evaluation.D9og5u/`. No shared specs/fixtures/goldens changed.

The new independently selected main-world adoption/evaluation leaf passes 24
release unit cases, strict Clippy and one real Chrome test. It verifies exact
selected-node identity after replacement before evaluation, main-world access,
typed values, structured errors, one dispatched invocation after cancellation,
no invocation on unpolled drop, and physical invalidation of both native
wrappers after group release. Its exact normal closure is 15 Pwright packages.
Node still does not install it; public Locator.evaluate remains unavailable.
Workspace release check, 30 umbrella profiles, both types, Node offline 9,
shared unit 30 and the 221-package Node boundary pass. See
[the complete checkpoint](../node-ffi-compatibility-design.md#locator-main-world-evaluation-checkpoint-2026-09-07).

Previous CDP-primitive regression uses addon
`db8f7337a628d4424e4ed252d8e2e27ed465b07a17878f72fcd6b0f6e6b7ea84`.
All five gates complete: Locator 32/32, evaluation 38/38, lifecycle 45/45,
actions 25/26 (NODE-022), shared 41 matches / 14 unsupported / 2 oracle errors.
Shared value/candidate/setup/cleanup errors are zero; supervisor exit 1 has no
signal, timeout, overflow or native failure. Reports and logs are in
`.work/node-locator-resolution.yiEN25/`. This does not resolve intermittent
NODE-014/024 or add Node Locator.evaluate coverage.

The independent resolution/continuation leaves now have 15/32 release unit
tests and two real Chrome tests proving semantic utility-world lookup, delayed
hidden/disabled elements, document replacement, one resolution deadline,
strict/close outcomes, detached-node identity and native group invalidation.
The new waiting API has 22 unit cases. It uses an injected single-Frame driver
in the live test, not a completed production topology/Node integration.
Strict-wait leaf evidence is `.work/node-locator-wait.HOnILm/units-complete.log` and
`live-final.log`; that checkpoint's Node artifact was not rebuilt/re-run for
these unselected leaf changes. Cancellation and discarded executor
jobs either complete cleanup or close admission; no zero-counter-only cleanup
claim is accepted. Exact dependency closures are five/seven Pwright packages.
Node does not install these leaves yet. Its unchanged 221-package boundary and
30 umbrella profiles pass. See [the phase and resource contract](pwright-locator-evaluation.md).

Previous semantic Locator checkpoint uses addon
`edd8ba969f29904184da9cc025397638cad99bd00860090c6ece267bf2b0c9b7`:
Locator **32/32**, actions **25/26**, evaluation **38/38**, lifecycle **45/45**,
shared **41 matches / 14 unsupported / 2 oracle errors**. Logs and completed
report are `.work/node-navigation-order.KIOgvf/*-final.log` and `shared-final/`.
Shared value/candidate/setup/cleanup errors are zero; supervisor exit 1 has no
signal, timeout, overflow or native failure. NODE-022 remains an ordinary
action failure. NODE-014 and NODE-024 did not reproduce and are not resolved.

Ten added Locator cases cover seven semantic constructors, exact/regex input,
role options, labels, relative/Shadow DOM composition, re-resolution/input,
lazy role errors, lossless-input rejection, name/title priority and global
role-name regex. The lifecycle lane adds a caller-released dynamic response:
independent reads complete while goto load remains pending until body release.
Static fixtures remain serve-lib-hosted; the dynamic gate is Tailscale-bound
and owned by its test. No assertion is skipped or normalized.

Final release tests pass Locator backend 100, selector runtime 4 and safe host
68, including six new typed-query/budget tests. Strict Clippy, workspace release
check, format/diff checks, offline 9, shared unit 30, legacy CLI harness 67,
both type checks, 221-package boundary, 10/18-package Locator dependency proofs
and 30 umbrella profiles pass. Intermediate failures are retained: Shadow DOM
name priority, obsolete runtime-version guards (explicitly stopped once
diagnosed), and 2-versus-3 global regex matches. The final artifact passes
their mandatory comparisons without changing text/label regex behavior.

Previous world-readiness checkpoint: addon
`1c8ee9ea2544c1c6e8f03ea5a74b4b29d390eb8cac4ca30b718a260224d9df65`
completes actions **25/26**, Locator **22/22**, evaluation **38/38**, lifecycle
**42/44**, and shared **36 matches / 19 unsupported / 2 oracle errors**.
Logs are `.work/node-world-readiness.YcU7iD/*-final.log` and
`shared-final/`; shared value/candidate/setup/cleanup errors are zero.
The shared gate still exits 1 without signal, timeout, overflow or native
failure. NODE-022 remains an ordinary action failure. Lifecycle records
NODE-014 outer dimensions (Rust 1282x800 versus oracle 0x0) and NODE-024's
independent-controller navigation interruption after candidate disconnect.
A three-pair bounded ordering probe passes but does not replace the failed gate.

The original Enter regression and new concurrent post-Enter reads and repeated
commit-navigation/action replacements pass. Nineteen new Rust readiness tests
cover exact contexts, bounded waiters/leases, unreset deadlines, close/transport
and future-drop cancellation, lossless notifications, preparation rollback and
unchanged default call-construction snapshots. Composition release tests pass
227, with 14 pre-existing opt-in browser tests ignored; topology 37 and host 62
also pass. Final strict Clippy, workspace release check, offline 9, shared-unit
30, both type checks, 221-package boundary and 30 umbrella profiles pass.
No dependency or JS automation behavior is added. These are entry-readiness
proofs, not complete navigation, Handle, selector, distribution or RSS parity.
The first readiness artifact's 44/44 lifecycle pass is retained separately and
does not make the final artifact green. Historical checkpoints follow.

The experimental Node package has five distinct live lanes: package-local
lifecycle/resource regressions, by-value evaluation/FFI pressure comparisons,
Locator query/read/resource comparisons, Locator action/input comparisons,
and the unchanged shared API corpus
in `tests/`.
The latter runs all 19 API suites / 57 cases on independently owned oracle and
candidate Contexts, with Playwright 1.58.2 as the reference. It never rewrites
goldens, skips unsupported operations, or counts oracle errors as parity.
The first full runs reported 6 matches, 49 unsupported cases and 2 oracle history
errors. Adding Rust-backed evaluation moves the complete matrix to 12/43/2, with
no value/candidate/setup/cleanup errors. The full gate intentionally fails.
The earlier paired-prerequisite addon reran the then-current three lanes: lifecycle 44/44,
evaluation 38/38 and shared 12/43/2, with no candidate/setup/cleanup errors in
the shared report. Earlier lifecycle 17 passes plus one failure before its
180-second supervisor and evaluation 35/38 topology-setup failures remain
recorded (NODE-013). TypedArray timing comparisons match (NODE-017). Preserve
failed and partial runs alongside passes; do not skip, normalize or silently
retry them. The shared gate still fails; these are not full API/RSS claims.

The new Locator lane runs `pnpm --dir packages/pwright test:locator` with the
same verified CDP/base variables. Its test-only child uses `--expose-gc` to prove
query finalization; production never invokes GC. Current expanded coverage has
22 original ordinary cases (32 after semantic coverage): composition/read/error/re-resolution comparisons, same-native
Page cross-connection rejection, Page-close cancellation, independent recipe
count/byte pressure and exact Worker Context removal. The earlier 18/19 result
retains NODE-020's original failure: an independent title read was serialized
behind a missing-element Locator on the same Frame. The explicit bounded Node
concurrency profile now passes 22/22, preserving that comparison and adding
evaluate-to-create, independent read overlap, and close under ten-read pressure.
No skip or timeout normalization hides a failure. Earlier expanded failures exposed a closed-session error-classification
bug, now fixed in the Rust Locator leaf using the shared command-free classifier.
Two additional leaf tests cover closure versus navigation-context loss.

The default-Context-close lifecycle test now creates and retains its exact own
Page rather than assuming the preflight uses `/index.html`; its title and live
identity must survive both implementations' default-Context close. An unrelated
or differently named preflight cannot satisfy the assertion. The original failed
run also captured a real initial outer-window-size discrepancy (NODE-014); this
fixture ownership correction does not resolve that separate native behavior.
See the latest [artifact-specific record](../node-ffi-compatibility-design.md)
for complete reruns; do not transfer passes across different addon hashes.

The Locator-stage rebuilt addon (`748c976f...e07838`) completes lifecycle 44/44,
evaluation 38/38 and the unchanged shared matrix at 22 matches / 33 unsupported
/ 2 oracle errors. Shared value/candidate/setup/cleanup error counts are zero;
the supervisor records exit 1 without timeout, overflow or native failure.
Together with that artifact's Locator 18/19, these remain historical scoped
results, not a compatibility certificate.

The concurrency-stage addon (`9b2415d7...e77452`) completes all four lanes:
lifecycle 44/44, evaluation 38/38, Locator 22/22, and shared 22/33/2 with no
value/candidate/setup/cleanup errors. Logs and the completed shared report are
in `.work/node-concurrency.TLMFjf/`. The shared supervisor still records exit 1,
without timeout, overflow or native failure. Its first invocation rejected a
relative output path before spawning the runner; that command error remains in
`shared.log`, separately from `shared-live.log` and the actual report.
Package-local passes do not close the shared unsupported gate or prove complete
operation ordering, distribution, or RSS behavior.

The action-stage addon (`eae32ce9...41af9e`) expands shared coverage to
36 matches / 19 unsupported / 2 oracle errors, with zero value/candidate/setup/
cleanup errors. The complete 57-case report is in
`.work/node-actions.5JbLoA/shared/`; its supervisor still exits 1 without
timeout, overflow or native failure. Public action adaptation adds fourteen
matches over the prior 22/33/2 artifact without changing specs or goldens.

`pnpm --dir packages/pwright test:actions` is an additional ordinary gate,
using the same CDP/fixture configuration. Its action-stage final 24-case run passes 22:
NODE-021's post-Enter missing utility route and NODE-022's timeout Shift-state
difference remain failures. The first 21-case run passed 17, including one
candidate and one oracle attachment timeout; the invalid heterogeneous
selection fixture was corrected to homogeneous forms and retained separately as
an expected-rejection comparison. That rejection initially exposed Error versus
TypeError, now fixed without normalizing assertions. Both failed artifacts/logs
remain alongside final evidence. Exact event/keyboard state is compared; a
cleanup policy difference is not silently accepted as parity.

The same `eae32ce9...41af9e` regression then passed evaluation 36/38 (two oracle
attach timeouts), Locator 21/22 (NODE-023 fence exhaustion), and reported 36
lifecycle passes plus one failure before its 180-second supervisor killed the
child. These replace no older artifact's successful evidence and are not green
gates. A stale exact fixture left by the killed child correctly failed duplicate
selection in a filtered diagnostic; only that verified owned target was closed.

NODE-023 was reproduced with the exact native eight-entry topology/lifecycle
queue error. The scoped terminal-completion fix skips redundant publication
only after TargetClosed/disconnection; it does not hide live capacity failures
or solve NODE-021. The replacement `e5076407...65d48` passes 62 safe-host tests,
Locator 22/22, evaluation 38/38 and lifecycle 44/44; actions remain 22/24
(NODE-021/022). Its shared corpus completes 31 matches / 19 unsupported /
2 oracle errors / 5 attach setup errors, not the preceding artifact's 36/19/2.
The initial expired-route HTTP 404 is retained separately from this completed
run. Full artifact-specific evidence belongs in the Node design record. Package
supervision now selects TAP so failure details are emitted before a possible
later kill, without increasing deadlines or changing failure classification.

Static fixtures must be registered with `serve-lib` and verified using their
full Tailscale MagicDNS URL. The Node runner checks served bytes against disk.
Use the dedicated Chrome profile and Pwright preflight described in the
[comparison guide](../../tests/README.md), not the legacy static server or the
historical Docker browser setup below. Test-owned Contexts/Pages are closed;
the external Chrome process and unrelated pages remain owned by their callers.

Pure runner tests cover classification, caught unsupported calls, cleanup
failures, bounded evidence and complete suite registration. Candidate teardown
checks Rust host, FFI and Node-thread prepared-conversion counters. Reports preserve both sides' observations,
versions and hashes; a separate supervisor rejects native panic or cleanup
diagnostics even if assertions passed. The history/BFCache diagnostic is a
separate oracle-only probe, never a modification to the paired conformance run.
See [NODE-005 and NODE-015](../node-ffi-compatibility-design.md) for open gates.

NODE-013 additionally has a separate bounded attachment probe. It traces only
command identity/method/session/timing, caps unanswered commands at 32 and recent
completed history at 128, and fails visibly on recording overflow. Three offline
tests prove those bounds. The live diagnostic is explicitly selected with
`--ignored`; it is not an ignored compatibility assertion. The Node orchestration
compares main-thread, Worker and independent Rust-process behavior while holding
one oracle connection. A transport ABBA helper accepts separately built release
probes, records artifact hashes, and rejects a changed target set as inconclusive.
It is not imported by the conformance lanes. No production TCP_NODELAY switch
remains after that experiment failed to demonstrate a useful difference.

Initial oracle-overlap probes reproduced the same five-second topology timeout
without Node-API, so an FFI-only explanation is insufficient. The latest opt-in
pair probe records overlapping requests and roughly 1.7–1.9-second attachments
with the oracle held open. Changing live browser activity prevents treating
timings as a controlled comparison or a resolved issue. Full-suite reruns above
are recorded independently of the probe. Commands and limits are in the
[package guide](../../packages/pwright/README.md#attachment-diagnostics).

The subsequent fixture-selection regression now requires the exact current URL
from the fixture creator, including across Workers. Only matching Pages receive
title probes; the unique marker must still be readable. Five offline tests cover
no unrelated title calls, unreadable fixtures, duplicate markers, wrong URLs and
missing owner URLs. This does not change capability assertions or the shared
corpus. The subsequent opt-in Rust prerequisite pair has separate deterministic
concurrency/cancellation/profile tests; browser reruns are recorded in the Node
design checkpoint, not inferred from unit passes.

## Historical v1 testing strategy

The following describes the existing v1 in-process fakes and Docker integration
infrastructure. Historical completion counts are not freshly rerun v2 evidence,
and its browser/server commands are not the prescribed local Mac workflow.

---

## Tier 1: FakeCdpClient (in-process, no Chrome)

An in-memory browser simulation that CDP operations work against. Tests set up
pages with real HTML, and `Locator`/`Page` methods interact with a real DOM tree.

### Architecture

```
crates/pwright-fake/src/
  lib.rs           Crate root, re-exports
  client.rs        FakeCdpClient struct + CdpClient impl
  dom.rs           In-memory DOM tree (nodes, attributes, text)
  selector.rs      CSS selector matching against the DOM
  builder.rs       HTML parser for test setup
```

### Usage

```rust
let fake = FakeCdpClient::from_html(r#"
    <div id="app">
        <h1>Welcome</h1>
        <input type="checkbox" id="agree" checked />
        <button class="submit" disabled>Submit</button>
        <ul>
            <li>Item 1</li>
            <li>Item 2</li>
        </ul>
    </div>
"#);

let page = Page::new(Arc::new(fake));
assert!(page.is_visible("button.submit").await?);
assert!(page.locator("#agree").is_checked().await?);
assert_eq!(page.text_content("h1").await?, Some("Welcome".into()));
```

### What the fake simulates

| CDP method | Fake behavior |
|-----------|--------------|
| `dom_get_document` | Return root node ID from in-memory tree |
| `dom_query_selector` | Match CSS against in-memory DOM |
| `dom_query_selector_all` | Return all matching node IDs |
| `dom_get_attributes` | Return node's attributes from tree |
| `dom_get_outer_html` | Serialize node to HTML string |
| `dom_get_box_model` | Succeed if node is visible, fail if hidden |
| `dom_resolve_node` | Map nodeId to objectId |
| `dom_request_node` | Map objectId to nodeId |
| `runtime_call_function_on` | Evaluate simple property checks (checked, disabled, textContent) |
| `runtime_evaluate` | Configurable (for complex JS, selector resolution) |

### CSS selector matching scope

- Tag: `button`, `div`, `li`
- Class: `.submit`, `.toggle`
- ID: `#agree`
- Attribute presence: `[checked]`, `[disabled]`
- Attribute value: `[type="checkbox"]`, `[data-testid="x"]`
- Descendant: `div span`, `ul li`
- Compound: `button.submit`, `input[type="checkbox"]`

### What stays configurable

- `runtime_evaluate` for arbitrary JS (page queries, selector resolution)
- Screenshots, PDF, cookies (don't interact with DOM)
- Event injection (`send_event`)
- Navigation responses

### Coexistence with MockCdpClient

`MockCdpClient` remains for call-sequence verification tests (e.g. "clicking
calls mousePressed then mouseReleased"). `FakeCdpClient` is for behavior
verification (e.g. "clicking a checkbox changes checked state").

| Test type | Use |
|-----------|-----|
| "Does click dispatch the right CDP calls?" | MockCdpClient |
| "Does is_checked() return true for a checked checkbox?" | FakeCdpClient |
| "Does first() pick the first of 3 elements?" | FakeCdpClient |
| "Does wait_for_response capture network events?" | MockCdpClient (event injection) |
| "Does create_tab call Target.createTarget?" | MockCdpClient + FakeSessionFactory |

### SessionFactory for Browser-level tests

`Browser` stores `Arc<dyn CdpClient>` (browser session) and `Arc<dyn SessionFactory>`
(tab session creator) instead of concrete `CdpSession`/`CdpConnection`. This enables
unit testing of `tab.rs` methods (create_tab, close_tab, resolve_tab, list_tabs) without
a real WebSocket server.

```rust
use pwright_bridge::test_utils::{MockCdpClient, FakeSessionFactory};

let mock = Arc::new(MockCdpClient::new());
let browser = Browser::new_for_test(mock.clone(), Arc::new(FakeSessionFactory));

let tab = browser.create_tab("https://example.com").await?;
assert_eq!(mock.calls_for("Target.createTarget").len(), 1);
```

`FakeSessionFactory` returns fresh `MockCdpClient` instances for each tab session.
`Browser::new_for_test()` is `#[cfg(test)]` only.

---

## Tier 2: Docker Integration Tests (real Chrome, no internet)

End-to-end tests against a real Chrome headless instance. A fake HTTP server
provides test pages so no internet connection is needed.

### Architecture

```
tests/integration/
  docker-compose.yml           Chrome headless (CI)
  docker-compose.local.yml     Chrome headless (local dev)
  Dockerfile                   Test runner container
  Cargo.toml                   Integration test crate
  src/lib.rs                   Shared helpers (connect, navigate)
  pages/
    login.html                 Login form test page
    todo.html                  TodoMVC-like test page
    api-demo.html              SPA that makes API calls
  tests/
    actions.rs                 Click, fill, type, press, hover, select, drag
    click_navigation.rs        Click triggers <a> navigation
    concurrency.rs             5 parallel tabs, session isolation
    connection.rs              Connect, reconnect, health
    locator.rs                 Selector resolution + element queries
    locator_advanced.rs        getBy*, filter, composition, nth
    login.rs                   Full login flow
    navigation.rs              goto, reload, back/forward
    navigation_advanced.rs     Wait strategies, block images/media
    network.rs                 wait_for_response, response_body
    recipes.rs                 YAML recipe execution
    script_execution.rs        Script runner end-to-end
    chrome_http.rs             ChromeHttpClient + HTTP tab lifecycle
    coordinate_actions.rs      click-at, hover-at, dblclick coordinate commands
    network_capture.rs         network-listen, network-get, second CDP session
```

### Docker Compose

```yaml
services:
  chrome:
    image: chromedp/headless-shell:latest
    ports: ["9222:9222"]

  test-runner:
    build: .
    depends_on: [chrome]
```

### Test pages

| Page | Purpose | Key scenarios |
|------|---------|---------------|
| `login.html` | Form with email/password/submit | fill, click, is_disabled, wait_for |
| `todo.html` | Todo list with add/check/delete | first/last/nth, count, is_checked |
| `api-demo.html` | SPA making fetch() calls on load | wait_for_response, response_body |

### Running

```bash
# Local: start Chrome, run tests from host
docker compose -f tests/integration/docker-compose.local.yml up -d
cargo test -p pwright-integration-tests -- --ignored --test-threads=1

# CI: Dockerized test runner (matches CI)
cd tests/integration && docker compose up --build --abort-on-container-exit --exit-code-from test-runner
```

---

## Implementation Status

### Phase 1: FakeCdpClient -- DONE
1. In-memory DOM tree with HTML parser (`pwright-fake`)
2. CSS selector matching engine (tag, class, id, attribute, descendant, compound)
3. FakeCdpClient implementing CdpClient with real DOM operations
4. 19 behavior verification tests (`fake_browser.rs`)
5. 3 examples migrated to FakeCdpClient (todomvc, svgomg, mock-filesystem)
6. MockCdpClient retained for CDP call-sequence verification

### Phase 2: Docker Integration -- DONE
1. Docker compose with Chrome (`chromedp/headless-shell`) + in-process test server
2. 96 integration tests across 15 test files
3. Concurrency tests (6 Docker + 6 FakeCdpClient)
4. Coordinate action tests (click-at, hover-at, dblclick)
5. Network capture tests (network-listen, network-get)
6. CI: Dockerized test runner (`docker compose up --abort-on-container-exit`)
7. Two compose files: `docker-compose.yml` (CI) + `docker-compose.local.yml` (dev)
