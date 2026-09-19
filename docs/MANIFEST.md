<!-- agent-updated: 2026-09-19T21:23:32Z -->

NODE-191: Add explicit standalone/combined
`retained-serialized-payload-mutation` profiles. They consume NODE-190's
move-only graph and the retained, already-retargeted INPUT in one Rust-owned
operation, validate the package-produced payload subset, issue exactly one
large utility-world `Runtime.callFunctionOn` with `userGesture`, reproduce the
pinned `File`/`DataTransfer` plus `input` then `change` behavior, settle the
object group and terminally remove the same FFI ticket. There is no selector,
second retarget/adoption, `DOM.setFileInputFiles`, extra N-API root or public
`ElementHandle.setInputFiles`. The completed upstream differential matrix
covers payload/event semantics, late type change, listener exceptions,
cancellation, navigation/detach races and the maximum admitted payload. It
records, rather than hides, the navigation-settlement and `lastModifiedMs`
differences that the later public API must decide. [Canonical contract and
verification](knowledge/pwright-node-handles.md#node-191-terminal-rust-owned-serialized-payload-mutation).

NODE-190: Add explicit standalone/combined
`retained-serialized-payload-import` profiles after NODE-189 serialization.
The host reserves one 96 MiB Rust graph owner before the addon copies any V8
value, and the addon enforces depth/node/entry/string-byte limits while
importing package-owned JSON-compatible data. Success advances `Serialized ->
PayloadImported`, drops all eleven direct V8 roots and retains a move-once Rust
graph on the same preparation. No CDP command, browser mutation, DOM event,
public API or generic live-handle transport is added. [Canonical contract and
verification](knowledge/pwright-node-handles.md#node-190-bounded-rust-ownership-for-the-serialized-payload-graph).

NODE-189: Add the addon-only `retained-call-argument-serialization` leaf/profile
after awaited input instrumentation. Rust preserves exact session/ticket/target
authority while the pinned Node language adapter implements synchronous getter,
`toJSON`, cycle and special-value behavior. The serialized graph becomes an
eleventh root; base64 string leaves are reused and never copied into Rust by
this phase. Generic live-handle adoption, DOM mutation/events, terminal
publication/error transport and the public method remain open. [Canonical
contract and verification](knowledge/pwright-node-handles.md#node-189-late-call-argument-serialization-on-the-retained-owner).

NODE-188: Add the addon-only `retained-input-instrumentation` leaf/profile and
internal `prepareInstrumentedTarget` coordinator. Rust gates exact-session
`Prevalidated -> WaitingInputAction -> Instrumented`; Node awaits a frozen,
default-no-op `onBeforeInputAction` port with the original receiver and retained
call identity under the same deadline. No host/backend/composition feature,
package edge, FFI admission, root or native lease is added. Late serialization,
mutation/publication, error mapping and public `ElementHandle.setInputFiles`
remain open. [Canonical contract and verification](knowledge/pwright-node-handles.md#node-188-awaited-input-action-instrumentation-boundary).

NODE-187: Add an explicit `retained-target-prevalidation` leaf/profile that
consumes the exact move-only marked owner after carrier projection. Route the
existing read-only DOM preflight through the same parked FFI ticket and Node
deadline, retain marker-call/adopted-group ownership inside the new validated
owner, and commit `Projected -> Prevalidated` without release/re-adopt, new JS
roots or a second command. Record independent eight-operation backend/owner/
cleanup budgets, exact dependency fingerprints, the selected seven-lease idle
baseline and final 28/28 real-Chrome results for both artifacts. Instrumentation,
late serialization, mutation, public error mapping and
`ElementHandle.setInputFiles` remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-187-same-owner-retained-dom-prevalidation).

NODE-186: Keep the successful native marking result move-only and parked on the
same FFI ticket instead of releasing it at the marker boundary. Route the
selected post-mark carrier projection, its observable getter/coercion order,
post-callback phase recheck, ten-root replacement and Rust-owned truth flags to
the canonical Handle record. Record unchanged one-command/1 MiB admission,
selected-only root growth from six to ten, the independent eight-call native
owner bound and the persistent marker-cache baseline caveat. Public retained
prevalidation, instrumentation, serialization, mutation and
`ElementHandle.setInputFiles` remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-186-retained-marked-owner-and-same-ticket-carrier-projection).

NODE-185: Route the selected per-session `call@N` source, post-schema/pre-
instrumentation identity transition, post-instrumentation source recheck and one
Node-event-loop deadline to the canonical Handle record. The same progress owns
materialization and marking; Rust starts no clock and marking accepts no JS ID.
Keep the unchanged FFI/registry budgets distinct from the small unbounded-V8
timer objects, preserve the one combined Worker timeout followed by a clean
pass, and record final 26/26 live evidence for both rebuilt artifacts while
retaining earlier overloaded-Chrome failures. Public projection, prevalidation,
instrumentation, serialization, mutation and `ElementHandle.setInputFiles`
remain open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-185-session-call-identity-and-one-node-deadline).

NODE-184: Route the default-off addon target-marking profile, typed initial FFI
progress, exact running revoker, original-Handle late upgrade, same-ticket park,
publication-before-error ordering and `Materialized` to `Marked` transition to
the canonical Handle record. Update architecture/resources, file-input/lifecycle,
dependency/testing, Node design/tasks and package mirrors. Keep the raw internal
method distinct from absent pre-materialization deadline/call identity, later
native phases and public retained upload. Record 24-case real Chrome evidence,
including cancellation, late disposal and foreign-session ownership, without
turning logical admission into an RSS claim.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-184-same-admission-node-target-marking-handoff).

NODE-183: Route the default-off Rust host target-marker cache, independent exact
Frame utility snapshot, 64-current/eight-retiring/eight-call admission, bounded
diagnostics, topology pruning and completion wakeups to the canonical Handle
record. Update architecture/resources, file-input/lifecycle/Locator evaluation,
dependency/testing, Node design/tasks and package mirrors. Keep Rust assembled
evidence distinct from unchanged addon artifacts, absent typed Node handoff,
deadline/call-ID authority and public retained upload. Preserve initial fixture
failures and final-source results, without treating native Set retention as an
RSS bound or kernel cancellation coverage as assembled pending-work shutdown.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-183-host-owned-utility-world-marker-cache).

NODE-182: Route default-off persistent exact-utility-world target marking,
separate state/call admission, raw awaited CDP settlement, cancellation-safe
group ownership and fail-closed cleanup to the canonical Handle record. Update
backend/composition READMEs, architecture/resources/lifecycle, dependency and CI
selection, Node design/tasks, file-input, package and testing mirrors. Distinguish
offline pinned-policy comparisons from real DOM marker evidence and from the
still-absent host cache, typed Node handoff, deadline/call identity and public
retained upload. Keep native Set retention and single-waiter proof limitations
explicit; Node timer delivery is not interchangeable with eager Rust expiry.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-182-persistent-native-target-marking).

NODE-181: Route inert original-NativeHandle weak binding through all selected
preparation records/snapshots, unchanged 32-entry registry layout and independent
1 MiB/16 KiB transient proofs to the canonical Handle record. Update architecture,
resources, Node design/tasks, file-input/lifecycle, dependency/testing and package
mirrors. Keep actual-ticket disposal/error-order evidence distinct from future
Weak upgrade, persistent utility-world marking, typed native handoff and public
upload. Record watch-aware active baselines/final cleanup observations and
bounded failure diagnostics separately from production behavior. Preserve both
failed sampling runs, final 20-case live passes across three executions,
268 offline checks per artifact, 161 package tests and final artifact hashes.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-181-original-native-wrapper-binding).

NODE-180: Route bounded exact-session weak FFI revocation, source/generation
binding, four-KiB selected registry storage, cancel-first admission barrier,
lock-free owner wakeups and revoke-before-drains teardown to the canonical
Handle record. Update architecture/resources, Node design/tasks, file-input/
lifecycle, dependency/testing and package mirrors. Keep initial language-only
capacity unchanged, and distinguish kernel ownership evidence from absent
actual Node promotion, marking, public upload and assembled/browser proof.
Record final selected/combined host and addon results, rebuilt artifact hashes,
268 offline checks per artifact, five strict Clippy selections and dependency
gates. Distinguish source-reviewed lock-free disconnect on retirement failure
from an unexecuted assembled-session failure-path regression.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-180-session-bound-revocation-before-native-drains).

NODE-179: Route same-admission FFI park/resume, weak revocation, actual-future
destruction ordering and the Rust raw-carrier projection kernel to the canonical
Handle record. Update architecture/resources, Node design/tasks, file-input/
lifecycle, dependency/testing and package mirrors. Keep exact HostSession
revoker registration, persistent utility-world marker ownership, real Node
ticket integration and public retained upload open. Offline projection and
budget tests do not prove browser behavior or native cleanup settlement.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-179-same-admission-ffi-handoff-and-raw-carrier-projection).

NODE-178: Route default-off retained native file-input prevalidation, exact
retargeted receiver/group ownership, one-shot result/exception allowances,
explicit userGesture/raw settlement, cancellation and fail-closed cleanup to
the canonical Handle record. Update file-input backend/composition READMEs,
adoption/lifecycle, architecture/resources, dependency/testing, Node design/tasks
and package records. Keep the 35-case offline oracle distinct from DOM/browser
proof, and host/addon selection, marking/raw carrier projection, late utility
serialization, deadlines/promotion and public retained upload open.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-178-owned-native-file-input-prevalidation).

NODE-177: Route exact owner-ticket/creation-Page registration, two Rust phase
gates around server validation, setImmediate/empty-instrumentation scheduling,
undefined Materialized return and abort ownership to the canonical Handle
checkpoint. Architecture/resources, Node design/tasks, file-input/lifecycle,
dependency/testing and package notes preserve the distinction from
world/Handle/Frame runtime retirement and the unchanged roots/dependencies/
classes/budgets/native pins. Both rebuilt selected artifacts pass twenty new
offline dispatch cases each; 17 live binding cases remain unexecuted. Exact
evidence belongs in the canonical record. Keep target marking, carrier projection,
DOM retarget/checks, onBeforeInputAction, late utility serializer and browser
atob/File ordering explicit; eager PreparedFileInput conversion and public upload
are not completed. No new browser result is claimed.
[Canonical contract and verification](knowledge/pwright-node-handles.md#node-177-deferred-registration-gates-before-materialization).

NODE-176: Route server-direction schema, raw payload materialization, exact pinned
MIME extension lookup, outer async assimilation and revocable six-root ticket
phases to the canonical Handle record and linked resource/architecture/Node,
dependency, file-input/lifecycle and testing records. Both rebuilt selected
artifacts pass 76 new offline cases; 12 actual-ticket live cases remain unexecuted. Real dispatcher
authority, error transport, native storage/promotion and public upload stay open.
[Canonical contract](knowledge/pwright-node-handles.md#node-176-server-direction-validation-and-raw-payload-materialization).

NODE-175 (2026-09-17): Route Rust raw-buffer channel schema/order, observable map/valueOf/ordinary assignment, zero-capture escaped callback safety and the ArgumentsReady-to-ChannelReady sixth-root replacement to the canonical Handle record. Record 61 channel cases per rebuilt profile, final artifact fingerprints and regression gates separately from the eight unexecuted live binding/channel cases. Keep valid-stream authority, public error wrapping, deferred materialization/submission and browser gates distinct from offline native-kernel comparisons.
[Canonical channel validation contract](knowledge/pwright-node-handles.md#node-175-rust-owned-raw-buffer-channel-validation).

NODE-174 (2026-09-17): Route canonical session/Page timeout issuance, frozen creation-Page proof, Rust producer/descendant binding, deferred lookup errors, bounded scoped-wrapper admission, cached raw debug inputs and seal-before-root-drop teardown to the canonical Handle record. Record rebuilt standalone/combined artifact fingerprints and completed offline checks separately from the new unexecuted six-case live binding gate; retain public setter/upload/channel gates.
[Canonical creation-Page policy contract](knowledge/pwright-node-handles.md#node-174-canonical-creation-page-timeout-binding).

NODE-173 (2026-09-17): Route Rust post-conversion argument assembly, live typed Context/Page timeout policy, selected metadata admission/class publication and offline argument comparisons to the canonical Handle record. Keep caller-supplied settings distinct from unimplemented production creation-Frame binding, public setters, channel validation, native deadlines and complete upload parity.
[Canonical argument and timeout contract](knowledge/pwright-node-handles.md#node-173-rust-owned-argument-assembly-and-live-timeout-settings).

NODE-172 (2026-09-17): Route Rust preflight policy, exact owner/context/conversion boundaries, narrow language ports, reentry-safe construction/replacement guards, independent synchronous scratch admission, variable root accounting and selected-addon offline kernel comparison to the canonical Handle record. Architecture/resources, Node design/tasks, dependency, file-input/lifecycle, package and testing records retain the missing public upload, native integration and browser gates.
[Canonical preflight contract](knowledge/pwright-node-handles.md#node-172-rust-owned-preflight-and-reentrant-language-phases).

NODE-171 (2026-09-17): Route exact-session Node preparation tickets, weak source metadata, pre-root admission, cleanup-hook/publication revocation, selected artifact profiles and offline Worker checks to the canonical Handle record. Architecture/resources, Node design/tasks, dependency, upload/lifecycle, package and testing records retain the unimplemented public conversion/submission and browser gates. Root counts are not arbitrary V8 graph or retained-tombstone byte bounds.
[Canonical Node preparation contract](knowledge/pwright-node-handles.md#node-171-session-bound-node-preparation-registry).

NODE-170 (2026-09-17): Route revocable FFI preparation, atomic execution promotion, environment-versus-session authority, shutdown/byte admission and additional upstream-only ingress observations to the canonical Handle record. Update architecture/resources, Node design/tasks, dependency/testing and package status. No Node registry, public upload method or addon artifact is implied.
[Canonical preparation contract](knowledge/pwright-node-handles.md#node-170-revocable-ffi-preparation-before-execution).

NODE-169 (2026-09-17): Route the host-only retained-file-input submission, read-only source receipt, atomic tracked-call admission, independent untimed phase cleanup and pinned offline ingress oracle to the canonical Handle record. Architecture/resources, dependency/Node design, upload/adoption/lifecycle/evaluation, package/test and task records preserve the explicit missing public Node integration and browser gates.
[Canonical contract and evidence](knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle).

NODE-168 (2026-09-17): Route optional retained-reference group adoption,
unique-world verification before accepting destruction receipts, explicit
attempt scratch/diagnostic budgets, utility numeric snapshot coherence,
numeric route storage and legacy adoption cleanup failures to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress).
Adoption, Locator evaluation, file-input, lifecycle, architecture/resources,
dependency and test records describe the boundaries. Node retained uploads and
the final browser gates remain incomplete.

NODE-167 (2026-09-17): Route current-DOM ElementHandle.ownerFrame, canonical
Frame-cache lookup, tracked read/native admission, raw object-ID description,
cancellation/cleanup and retained-upload preparation design to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md). The
Handle leaf/evaluation/operations/lifecycle, Node/package, architecture,
resources, dependency, testing and task records describe their boundaries.
Retained upload and full-corpus parity remain open.

NODE-166 (2026-09-17): Route Node getProperties, pre-admitted staged native
batches, per-ObjectId cleanup tickets, raw property settlement, complete
addressable-owner compensation, structural staging bounds, failure-safe native
and JS publication, and confirmed Page-close world receipts to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md).
Selected cleanup capacity is now 112; scalar/batch calls share the existing
eight-call and 64-result budgets. No package/feature/protocol version is added.
The Handle evaluation/operations/lifecycle, Page-close, architecture/resource,
dependency, Node/package and task/testing records describe their boundaries.
Keep the full corpus and unaddressable-owner design open; historical missing
getProperties notes below describe their original checkpoints.

NODE-165 (2026-09-17): Route explicit raw retained-result CDP acquisition/release
selection, exception-ID preservation, settlement ownership, borrowed two-ID
compensation, bounded diagnostics and cancellation/failure evidence to
[the canonical Handle checkpoint](knowledge/pwright-node-handles.md).
The Handle evaluation/operations/lifecycle records and leaf README describe
the ownership boundary; resource and dependency records retain the unchanged
package/feature/schema/API scope. Exact verification belongs in the canonical
checkpoint, not inferred from older counts. Runtime.getProperties settlement,
full property-pipeline admission, a pre-owned group for unaddressable IDs,
Node batch publication and the full compatibility corpus remain open.

Also route NODE-165's Handle-enabled owned-inspector retirement before Handle
drain, error-preserving detach/disconnect fallback, and conservative
compensation-versus-detach residual to [the canonical shutdown record](knowledge/pwright-node-handles.md#node-165-retained-acquisition-owns-failure-cleanup-and-shutdown-settlement)
and the architecture/resource/lifecycle records. Non-Handle ordering is
unchanged; no shared-browser or borrowed-Page ownership is broadened. Generic
Cleanup and an earlier host snapshot cannot substitute for the actual frozen
route/client identity plus exact native retirement receipt.

NODE-164 (2026-09-17): Route the crate-wide N-API module-local class identity, locked-source validation-order gate, native-class isolation lane, dependency-key-only rename, portable property snapshot/publication repair and abandoned cleanup failure reporting to [the canonical Handle checkpoint](knowledge/pwright-node-handles.md). The priority raw-class audit now has an implementation; exact verification and remaining gates are recorded in that canonical checkpoint. Preserve historical evidence and keep Node getProperties and broader compatibility gaps open.

NODE-163: Route Handle-targeted producers, staged source revalidation, native and JS publication rollback, compile-time future storage bounds, property Promise identity, paired producer evidence and the remaining batch ownership design to knowledge/pwright-node-handles.md and the Handle leaf README.

NODE-162: Route native-only expression programs, unchanged portable 1.2 codecs, indirect global Handle creation, ignored-argument validation/admission, deferred disposed-target errors, handle-value selection and red/green verification to knowledge/pwright-node-handles.md and the Handle leaf README.

NODE-161: Route module-local native-branded owned nested arguments, independent-addon/type-spoofing guards, Frame by-value reference evaluation, conversion/error phases, shared FFI baseline, resource proofs, type unboxing and final artifact verification to knowledge/pwright-node-handles.md and the Handle evaluation leaf README. String-expression reference arguments, adoption and other producer/action families remain unfinished.

NODE-160: Route NODE-160 public opt-in Handle creation, value reads, completion-only disposal, conversion reuse, resource accounting, artifact selection and differential evidence to knowledge/pwright-node-handles.md. The full retained-upload lane and remaining Handle producers are still open.

NODE-159: Route safe-host native-creation selection, eight-entry driver publication, pre-admitted receiver-loss/late-result compensation, typed world-retirement proof, queue/resource budgets and final test evidence to knowledge/pwright-node-handles.md. Public Node Handles and other producer families remain unfinished.

NODE-158: Route opt-in staged native Handle creation, deferred local publication, explicit compensation ownership, exact source-world revalidation, dependency/CI checks and real Chrome single-execution evidence to knowledge/pwright-node-handles.md and knowledge/pwright-handle-evaluation.md. Host admission/queue/recovery and public Node Handle APIs remain unfinished.

NODE-157: Route host-native Handle release scheduling, queue-free concurrent close, source-world retirement witnesses, bounded shutdown draining and red/green browser evidence to knowledge/pwright-node-handles.md. Creation scheduling and public Node Handle parity remain open.

NODE-156: Route optional native Handle release, exact route-owner receipts, pre-I/O commit admission, live Chrome invalidation and dependency/CI evidence to knowledge/pwright-node-handles.md and knowledge/pwright-handle-lifecycle.md. Host scheduling and public Node Handle integration remain pending.

NODE-155: Route queue-free runtime disposal admission, recoverable busy commit boundaries and retained capacity tests to knowledge/pwright-node-handles.md. Native release receipts, host scheduling and public Node Handle publication remain pending.

NODE-154: Route safe-host pending Handle results, revocable cleanup tickets, descriptor-versus-native work lifetimes, explicit admission and native shutdown evidence to knowledge/pwright-node-handles.md. Public N-API Handle producers remain gated.

NODE-153: Route abandoned Handle batch partial-failure cleanup, red/green service regression evidence and still-pending Node delivery ownership to knowledge/pwright-node-handles.md and knowledge/pwright-handle-evaluation.md. No public Node API or artifact changes.

NODE-152: Route the optional safe-host handles profile, shared leaf ownership, native capacity/retirement tests and still-gated Node publication to knowledge/pwright-node-handles.md. Existing addon artifacts must not select handles.

NODE-151: Route Node Handle prerequisites, 18-case oracle/paired-red evidence and the borrowed payload-target boundary to knowledge/pwright-node-handles.md and the file-input checkpoint. Public Node Handles remain absent.

NODE-150: Route Page/Frame upload APIs and Rust-scoped query ownership to the file-input record: standalone 43/43 x3, combined 43/43, 100 package contracts and 30 shared units pass. Full corpus remains 53/57 matching. Raw-click capture and its reversed mismatch are recorded separately in pwright-raw-mouse.
[Page/Frame upload contract and verification](knowledge/pwright-file-input.md#node-150-page-and-frame-selector-taking-upload-apis).

NODE-149 verification: both selected artifacts pass 32 upload cases; full shared corpus is 53/57 matching and remains non-green. See the canonical file-input record for exact remaining rows.

NODE-149: Route opt-in addon profiles, Buffer ingress/type validation, public Locator uploads, packaging notices, Worker teardown and differential verification to the file-input record.
[Selected Node upload boundary and verification](knowledge/pwright-file-input.md#node-149-selected-node-buffer-ingress-and-public-locator-uploads).

NODE-148: Route selected safe-host upload APIs, shared resolver/Frame lifetime, source admission and shutdown drain changes to the file-input record. Public addon/JS integration remains pending.
[Selected safe-host upload contract](knowledge/pwright-file-input.md#node-148-selected-safe-host-upload-integration).

NODE-147: Route optional large outbound CDP admission, fragmented transport, backend selection and near-limit browser evidence to the file-input record. Ordinary command/inbound ceilings and public Node artifacts remain unchanged.
[Large outbound admission and remaining gates](knowledge/pwright-file-input.md#node-147-independently-admitted-large-outbound-commands).

NODE-146: Adds admitted exact-element upload continuations and an explicit CDP settlement wait. Route dropped-waiter, timeout, source/group/route ownership, cleanup health, selected dependency and test changes to the file-input record. Public Node upload and native directory completion remain open.
[Upload continuation and remaining gates](knowledge/pwright-file-input.md#node-146-single-use-upload-continuations).

NODE-145: Adds the opt-in Chromium payload mutation backend and real-browser source-to-File evidence. Route DOM behavior, encoding/scratch admission, group-owned by-value CDP calls and exception cleanup, native ownership and upload validation changes to the file-input record. Native path/directory completion, cancellation composition and public Node wiring remain open.
[Chromium payload backend and evidence](knowledge/pwright-file-input.md#node-145-chromium-payload-mutation).

NODE-144: Adds the optional admitted Tokio file-input executor. Route worker admission, cancellation, drain and queue/source lifetime changes to the file-input knowledge record. Browser backend, large-message transport and public Node upload are still open.
[Admitted executor contract and evidence](knowledge/pwright-file-input.md#node-144-admitted-blocking-executor).

NODE-143: Adds optional filesystem path preparation, admitted descriptor/path ownership and a pinned MIME provider. Route filesystem/MIME changes to the file-input knowledge record; executor admission, browser actions and public Node upload remain open.
[Filesystem preparation and evidence](knowledge/pwright-file-input.md#node-143-filesystem-preparation-and-pinned-mime).

NODE-142: Adds the optional ledger-admitted file-input content owner, selected only as a standalone leaf. Update source ownership, failure/cancellation evidence and dependency routing. Path preparation, executor admission, browser upload and public Node wiring remain open.
[File-input content ownership and remaining work](knowledge/pwright-file-input.md#node-142-ledger-admitted-content-ownership).

NODE-141: Begins optional Rust file-input preparation with a zero-dependency model and pinned real-browser oracle characterization. Update file-input semantics, ownership/resource and dependency boundaries, architecture, Node/package status and implementation tasks. No public upload API or native addon selection is implemented yet.
[File-input policy, evidence and remaining work](knowledge/pwright-file-input.md).

NODE-140: Reopens the complete Frame and shared API corpora and adds bounded Page-close/body and raw-click diagnostics. Update testing strategy, Frame/close and raw-Mouse evidence, request-capacity status, Node/package scope and design tasks. Preserve non-green full gates separately from prior focused passes.
[Expanded verification and retained failures](knowledge/testing-strategy.md#node-140-broadened-node-ffi-verification-remains-non-green).

NODE-139: Retain Page lifetime through Frame.goto publication and result projection, with a borrowed close wait on existing lifecycle owners. Update Frame navigation, close-cascade interpretation, lifecycle/ownership, resource admission, architecture, Node/package status and testing/tasks. The earlier natural error comparison used different native commit phases; do not label it a proved semantic discrepancy.
[Page lifetime publication contract and verification](knowledge/pwright-frame-navigation.md#node-139-page-lifetime-through-navigation-publication).

NODE-138: The opt-in Node driver now owns concurrent native closes and latest-state local retirement. Update close-cascade, lifecycle/ownership, resources/architecture, package profiles and verification status. Default/external Context paths and ordinary builds are unchanged.
[Node close-driver integration and final verification](knowledge/pwright-close-cascade.md#node-138-opt-in-node-close-driver-integration).

NODE-137: Adds native-proven synchronous local close retirement and a returned post-lock notification owner. Update close-cascade, architecture/resources, object ownership, Node lifecycle/compatibility status and tasks/testing. The Node driver and controlled browser gate remain unwired.
[Native local retirement and post-lock notification contract](knowledge/pwright-close-cascade.md#node-137-native-proven-local-retirement-and-post-session-lock-notification).

NODE-136: Joined close-capacity admission and consuming native completion now compose move-only reservations with borrowed evidence. Update close-cascade, architecture/resource/object ownership, Node lifecycle/compatibility status and tests/tasks. Keep the full Node coordinator and browser discrepancy open.
[Joined close capacity and native completion contract](knowledge/pwright-close-cascade.md#node-136-joined-close-capacity-and-consuming-native-completions).

NODE-135: Adds runtime-locked local publication plus deferred disposal, preventing transport mutation between the two phases of a selected close commit. Update close-cascade, architecture/resource/object ownership, Node lifecycle/compatibility status and testing/tasks. This does not wire the full native close coordinator.
[Runtime-locked publication/disposal contract and evidence](knowledge/pwright-close-cascade.md#node-135-runtime-locked-local-publication-and-disposal).

NODE-134: Adds optional runtime-owned control outbox backing admission and fallible preallocation through the existing disposal-reservations feature. Update close-cascade, runtime/resource ownership, architecture/dependencies, Node lifecycle/compatibility status and testing/tasks. Graph/container/commit scratch and the full close coordinator remain unfinished.
[Preallocated runtime outbox contract and evidence](knowledge/pwright-close-cascade.md#node-134-preallocated-runtime-control-outbox-storage).

NODE-133: Selected native-world receipts retain the owning Page target and the driver marks them on actual native target destruction, even after local routes disappear. Update close-cascade, topology/Locator/native ownership, resource and lifecycle contracts, architecture, Node/package status and verification tasks. Public close scheduling remains separate.
[Native Page world-retirement contract and evidence](knowledge/pwright-close-cascade.md#node-133-retained-worlds-follow-exact-native-page-destruction).

NODE-132: Adds attachment-bound native evidence validation and latest-topology callback serialization. Update close-cascade, architecture, native ownership/resource contracts, Node lifecycle/compatibility status, tasks and tests. It is not the complete retirement coordinator.
[Native close commit-scope contract and evidence](knowledge/pwright-close-cascade.md#node-132-attachment-bound-native-close-commit-scope).

NODE-131: Adds selected deferred runtime commits, collection-payload peak admission and explicit post-lock notification ownership. Update close-cascade, runtime resources/object ownership, lifecycle/architecture, Node status, tasks and testing. Full close coordinator and host integration remain unfinished.
[Deferred runtime close commit contract and evidence](knowledge/pwright-close-cascade.md#node-131-deferred-runtime-disposal-notifications).

NODE-130: Adds independent deletion-only route retirement, strict survivor validation, atomic subscription pruning and a separate post-commit wake port. Update close-cascade, route/resource ownership, Frame navigation/publication, lifecycle integration status, architecture/dependencies, Node/package and tasks/testing. Full close transaction and Node scheduler integration remain unfinished.
[Deletion-only close route contract and evidence](knowledge/pwright-close-cascade.md#node-130-deletion-only-close-route-retirement).

NODE-129: Adds opt-in bounded native close scheduling and guarded Page command initiation. Update close-cascade, native ownership/resources, lifecycle/publication integration status, dependency/architecture, Node/package and testing/tasks. The Node host and retirement commit are still unwired.
[Bounded native scheduler contract and evidence](knowledge/pwright-close-cascade.md#node-129-bounded-native-close-scheduler).

NODE-128: Adds opt-in exact native Context ancestor authority, sealed Page bindings, bounded command receipts and current-route certification. Update close-cascade, Page/Browser lifecycle ownership, resources, architecture/dependencies, Node/package status and testing/tasks. Keep native scheduler/publication integration and the browser gate unfinished.
[Native ancestor authority contract and evidence](knowledge/pwright-close-cascade.md#node-128-exact-native-context-ancestor-authority).

NODE-127: Adds split native command acknowledgement and destruction-wait ownership. Update close-cascade, Page/Browser close, native/lifecycle ownership/resources, architecture/dependencies, Node/package status and testing/tasks. Keep the coordinator and ancestor-dispatch gate explicitly unfinished.
[Native close phase contract and evidence](knowledge/pwright-close-cascade.md#node-127-native-command-and-destruction-wait-phases).

NODE-126: Adds optional deferred close publication admission and latest-view preparation. Update close-cascade, publication/lifecycle/ownership/resources, architecture/dependencies, Node/package status and testing/tasks. Native scheduling and ancestor receipts remain unwired.
[Deferred publication contract and evidence](knowledge/pwright-close-cascade.md#node-126-deferred-close-publication-capacity).

NODE-125: Adds opt-in runtime disposal/outbox reservations, queue-slot accounting and latest-state preparation. Update close-cascade, lifecycle/object/resource contracts, architecture/dependencies, Node/package status and testing/tasks. Selected/combined/ordinary tests pass 142/159/113; native intent scheduling and publication rebasing remain unwired. Preserve closed-transport precedence red/green evidence separately from the unchanged browser discrepancy.
[Runtime reservation contract and evidence](knowledge/pwright-close-cascade.md#node-125-opt-in-runtime-disposal-and-outbox-reservations).

NODE-124: Adds opt-in topology transition reservations with an independent budget/pending limit and latest-state exclusive commit. Update close-cascade/topology contracts, resources/ownership, architecture/dependencies, Node/package status and design/testing tasks. Keep the feature unwired in host/addon and retain the ancestor-dispatch gap. Selected topology passes 76 tests; ordinary passes 49; normal dependency/features are unchanged.
[Topology reservation contract and evidence](knowledge/pwright-close-cascade.md#node-124-opt-in-topology-transition-reservations).

NODE-123: Adds close-cascade evidence/design and bounded transparent/controlled probes. Fixes Context interruption semantics and seals admission when driver reply producers or request queues disappear before the normal stop block. Update Browser/Page/Frame close, lifecycle/publication, object/resource ownership, architecture, Node/package status and design/testing tasks. Preserve the intermediate peer-repeat failure and distinguish final 50-case matrices plus repeated confirmation gates from the red ancestor-dispatch probe. Concurrent intents and reservation/commit separation remain unimplemented.
[Close cascade contracts and evidence](knowledge/pwright-close-cascade.md).

NODE-122: Adds selected native Page destruction confirmation, bounded source/call/error ownership, an independent reclaiming cursor and terminal-settlement fencing. Update Page/Browser close, topology, lifecycle/publication, dependency/resource/object ownership, architecture, Node/package status, design tasks and verification evidence together. Also update Frame navigation for the combined-profile fixture harness and retained 27/29 arrival-boundary failures. Both final artifacts pass 48 close/lifecycle/connection cases; historical failed runs remain distinct from final gates and overall compatibility remains unfinished.
[Native confirmation contract and evidence](knowledge/pwright-page-close.md#node-122-selected-native-destruction-confirmation).

NODE-121: Adds bounded original-scenario close tracing and a pinned-source-based native close confirmation plan. Update Browser/Page close contracts, lifecycle/testing evidence, resource boundaries, Node/package status and design tasks. Diagnostic code changes do not imply a native behavior fix or new addon artifact.
[Transparent trace and next confirmation contract](knowledge/pwright-browser-close.md#node-121-transparent-original-scenario-tracing).

NODE-120: Adds bounded Rust native-driver error attribution and diagnostic-only close probes. Update Browser/Page close, lifecycle evidence, architecture/resource boundaries, Node/package status, testing and design tasks. Preserve natural failure evidence separately from controlled injection and later successful repeats.
[Close attribution contract and evidence](knowledge/pwright-browser-close.md#node-120-attribute-retained-native-driver-failures).

NODE-119: Connects native ready/linkage production through masked collection commits and per-Page Rust acknowledgement state. Update lifecycle/publication, creation/close, object/resource ownership, architecture/dependencies, Node/package status, design tasks and tests. Record both complete event corpora and exact Frame/cleanup resource assertions; keep the overall compatibility/stability gaps explicit.
[Native ready-source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: Adds precommit ready/popup descriptor conversion, missing-wrapper requests and metadata-root binding. Update publication/lifecycle, object/resource ownership, creation/close, Node/package, architecture/dependencies, tasks and tests. Keep conversion evidence separate from pending production native creation sources.
[Creation-conversion contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Joins committed external disposal effects with independently pre-admitted close capacity. Update lifecycle/publication, creation/close, object/resource ownership, architecture/dependencies, Node/package status, tasks and testing. Preserve the two creation-event gaps and distinguish native Context-cascade tests from live external Page-close proof.
[External-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Wires explicit-close attachment factories and pre-admitted native receipts into the existing Node dispatcher. Update lifecycle/publication, close/creation, ownership/resources, architecture/dependencies, Node/package, tasks and tests; preserve compact Context-cascade semantics and the explicit external-close/creation gaps.
[Explicit-close source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Selects native terminal production in the Node addon and adds one bounded environment-thread dispatcher across all publication paths. Update lifecycle/publication, ownership/resources, architecture/dependencies, Node/package status, close/creation, tasks and testing; distinguish working terminal events from unfinished ordinary sources.
[Terminal dispatcher contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Documentation manifest

NODE-114: Integrates an opt-in terminal native source with pre-initialization group admission and adds a real host-shutdown gate. Update lifecycle/publication, resource/ownership, architecture/dependencies, Node/package status, Page/Context/Browser boundaries, tasks and verification. Preserve the distinction between native source completion and pending Node event delivery.
[Terminal source contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: Adds pre-admitted groups to the existing Rust publication queue and extends listener characterization. Update lifecycle/publication, object/resource ownership, architecture/dependencies, Node/package status, creation/close, tasks and testing. No selected Node event support is claimed.
[Group contract and evidence](knowledge/pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Adds the optional prepared/committed native close observer, preserving ordinary
service fields and existing native lock order. Update lifecycle/publication,
Page/Context/Browser source coverage, ownership/resources, architecture and
dependencies, Node/package status, tasks and verification evidence.
[NODE-112 close receipt contract and verification](knowledge/pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: Adds the borrowed committed driver-effect observer, without a host event journal
or Node callback integration. Update topology events, lifecycle/publication,
creation/close source coverage, ownership/resources, architecture/dependencies,
Node/package status, tasks and testing; distinguish native input from delivery.
[NODE-111 source contract and verification](knowledge/pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 characterizes listener failure/reentry and adds scoped Rust exception
reporting for existing Browser disconnected dispatch. Update lifecycle events,
Browser close, Frame publication, Context/Page creation, ownership/resources,
architecture/dependencies, Node/package, tasks and testing. Preserve pre-fix
error-channel evidence and distinguish the fixed Browser cases from missing
Page/Context notifications and unimplemented dependent native cause groups.
[NODE-110 contracts, fix and verification](knowledge/pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 introduces the exclusive Rust metadata drain and its fixed storage
allowance, without wiring Node callback delivery. Update lifecycle-event status,
Frame publication, Context/Page creation, Page/Browser close, object ownership, resources, architecture,
dependencies, Node/package, design tasks and testing. Distinguish raw queue
advancement from atomic event-state publication; record rebuilt artifact evidence.
[NODE-109 contract and verification](knowledge/pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-108 adds bounded native lifecycle TRACE identities and an additive attach-
probe report field. Update topology diagnostics, event admission trace ordering,
resources/dependencies, Node/package, lifecycle implementation status, tasks and
testing. Distinguish freshly rebuilt native probes from unchanged Node addons;
preserve prior initialization failures and do not claim a behavioral timeout fix.
[NODE-108 receipts and verification](knowledge/pwright-chromium-topology-events.md#node-108-bounded-native-lifecycle-identity-receipts).

NODE-107 also records newly reproduced initial-attachment failures and bounded
command receipts on the rebuilt profiles. Update topology-event driver diagnostics,
Node/package, tasks and testing to mark the live gate partial. Do not attribute
setup timeouts to missing lifecycle notifications or erase earlier failures.

NODE-107 adds cross-revision visibility rebasing, metadata-domain identity and
Context settings identity validation. Update lifecycle-event status, Frame
publication, ownership/resources, architecture/dependencies, Context creation,
Page/Browser close, Node/package, tasks and testing. Record the new layout and
rebuilt artifacts without claiming the step owner or callbacks are implemented.
[NODE-107 contracts and evidence](knowledge/pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 introduces immutable Rust collection visibility and changes the selected
published-collection return type and layout allowances. Update lifecycle-event
status, Frame publication, resources, object ownership, architecture/dependencies,
Page/Browser close, Context creation, Node/package, tasks and testing. Record
rebuilt artifacts; distinguish this single-revision foundation from the still
unimplemented cross-revision step cursor and lifecycle callback dispatch.

NODE-105 adds a RED Node lifecycle-event gate and stepwise publication design.
Update lifecycle-event design, Frame publication, Page/Context creation and close,
Browser close, ownership/resource/dependency design requirements, Node/package,
tasks and testing. This introduces no production behavior, budget or dependency;
keep callback-state requirements separate from already-green collection getters.

NODE-104 adds paired creation/discovery collection publication characterization.
Update Frame publication, Context/Page creation, topology, Node/package, tasks
and testing. Preserve the concurrent-order and FFI-lane harness corrections;
record unchanged artifacts and separate collection coverage from missing events.
No production resource, dependency or architecture contract changed.

NODE-103 wires selected Rust collection producers and Node getters, atomic batch
publication, pre-admitted retirement pools/creation rollback, and Rust baseURL
TypeError validation. Update Frame publication, topology, resources, ownership,
architecture/dependencies, Page/Browser close, Context creation, Node/package,
tasks and testing. Preserve older red/unwired checkpoints as historical evidence;
record rebuilt artifact hashes and final matrices, not broad parity claims.

NODE-102 implements bounded Rust collection revision storage on the existing
publication queue. Update Frame publication, resources, object ownership,
architecture/dependencies, Page/Browser close, Context creation, Node/package,
tasks and testing. Record measured logical budgets and distinguish storage
implementation from still-unwired producers/getters and the unchanged red gate.

NODE-101 adds Context/Page collection publication characterization and a RED
candidate lane. Update Frame publication, Page/Browser close, Context creation,
resource-design requirements, Node/package, tasks and testing. No production
capability or new byte budget is implemented; preserve artifact-specific failures
and distinguish an oracle harness-order correction from a product discrepancy.

NODE-100 changes Browser connection publication timing and Rust-owned event
dispatch suppression. Update Browser/Page close, Frame publication, object and
resource ownership, architecture/dependencies, Node/package, tasks and testing.
Keep native admission separate from public state, preserve the red baseline,
and identify rebuilt artifacts; do not imply complete lifecycle compatibility.

NODE-099 implements Rust-published Page.isClosed and pre-admitted terminal queue
ownership. Update Page/Browser close, Frame publication, object/resource contracts,
architecture/dependencies, Node/package, tasks and testing. Preserve the earlier
red baseline as historical evidence; only rebuilt artifacts establish support.

NODE-098 adds Page.isClosed reference characterization, a red candidate lane and
a terminal-publication design gate. Update Page-close/publication, Node/package,
tasks and testing. This changes no production API, resource contract or artifact;
reference-only success must not be reported as candidate compatibility.

NODE-097 isolates a native cross-client Page auto-attachment trigger and expands
the peer lane to direct failure/recovery comparisons. Update topology/navigation,
Browser-close and header stress attribution, Node/package, tasks and testing.
No production API/resource/architecture changes or broad compatibility claims.

NODE-096 adds bounded navigation timing/retirement diagnostics and the controlled
navigation-peer-disconnect test. Update topology/navigation evidence, header
fixture observation rules, Node/package, tasks and testing. Correct the earlier
untimed load inference; no production architecture/resource contract changes.

NODE-095 adds selected reader-level pending-session retirement and typed detach
handling, with bounded command identity storage. Update topology/close/publication,
headers, ownership/resources, architecture/dependencies, Node/package, tasks and
testing evidence. Preserve default/v1 opt-out and the distinct no-pending-call
navigation timeout; do not claim full CRSession or API parity.

NODE-094 changes selected Context fallback for stale discovery snapshots and
adds queued-context-retirement. Update topology, Context/object/publication/close,
architecture/resources, Node/package, tasks and testing records. It does not
normalize root-detach errors or prove full concurrent/API compatibility.

NODE-093 preserves typed header initialization errors and expands attachment
occurrence/child-header command diagnostics. Update header/topology, Node/package,
tasks and testing evidence. Resource/feature/dependency contracts are unchanged;
do not promote targeted retirement coverage to full concurrent disconnect parity.

NODE-092 corrects exact second-stage native child-detach retirement and adds a
controller-owned browser Worker race/control lane. Update topology/close and
publication, Node/package, ownership/resources, architecture/dependencies, tasks
and testing. Do not generalize this receipt into all cleanup/Worker parity.

NODE-091 adds request-scoped native session-close watches and a composed optional
header observer, plus the public header-child-sessions differential lane. Update
headers, Node/package, topology/publication/navigation, ownership/resources,
architecture/dependencies, tasks and testing. Broader shutdown and API parity
remain unproven.

NODE-090 changes discovered-Page initialization failure handling and prepared
lifecycle-owner retirement. Update topology, publication/navigation, ownership,
resources, architecture/dependencies, Node/package, tasks and testing records.
The attachment-retirement lane is not proof of full connection/API parity.

NODE-089 corrects Context Page sequencing, defers later-Page header snapshots,
and adds real-Chrome receipt-gated public success/failure comparisons. Update
header, Node/package, architecture, ownership/resource, dependency, task and
testing records. Preserve NODE-088 as historical evidence, not current dispatch
semantics; shutdown, OOPIF/Worker and broader compatibility remain open.

NODE-088 adds caller-polled header jobs, bounded native fanout, early primary
rejection with retained compensation, and native-capacity backpressure. Update
headers, topology/publication/creation/close, Node/package, architecture,
resources/ownership, dependencies, tasks and testing. Shutdown retirement and
real-CDP failure-timing fidelity remain unproven.

NODE-087 moves selected header/navigation preparation to the synchronous Rust
call boundary, freezes native dispatch and selects nine metadata writers. Update
Node/package, header/navigation, ownership/resource, architecture, dependency,
task and testing records. Native concurrency/early settlement remains unfinished.

NODE-086 reproduces a pending header-setter/navigation ordering discrepancy and
adds explicitly bounded overlapping metadata writers. Update header, navigation,
Node/package, ownership/resource, dependency, task and testing records. Internal
128-case agreement must not be presented as fixed public scheduling.

NODE-085 wires Page-only referrer policy into public Rust Page/Frame navigation.
Update header/navigation, Node/package, ownership/resources, dependency, task and
testing records. Paired first-line/wire checks do not establish call-log fidelity,
pending setter races, worker/OOPIF routing or complete navigation compatibility.

NODE-084 adds selected Rust-owned Context creation headers and public differential
coverage. Update header/Context creation, Node/package, ownership/resources,
publication/dependencies, tasks and testing records; creation fault injection and
broader header/navigation parity remain open.

NODE-083 separates bounded desired-header storage from native syntax rejection,
and isolates failed explicit Page initialization through exact-Page compensation.
Update headers, Context/Page creation and close, topology/publication, ownership,
resources, Node/package, dependency, task and test records.

NODE-082 exposes experimental selected Node header setters and phase-aware
language conversion. Update header, Node/package, resources/ownership,
Context/publication, dependency, tasks and testing records. Nine paired groups
and one admission test do not establish failure timing, workers or full parity.

NODE-081 selects the Rust native header host and fixes obsolete-session header
state. Update header, topology/publication, Context, ownership/resources,
dependencies, Node/package, tasks and testing records. Native wire tests are
not public Node/worker coverage or upstream failure-timing compatibility.

NODE-080 implements bounded Rust desired-header transactions and a 54-case
pinned internal setter comparison. Update header, ownership/resources, Context,
Node/tasks, dependency, package and testing records. Native installation and
public setters remain open; uncertainty tracking must not block desired reads.

NODE-079 extends native session initialization with borrowed portable ownership
while preserving legacy callbacks. Update topology, header, publication/object,
resource/dependency, Node/task and testing records. This is not a settings
registry, native header update or public FFI implementation.

NODE-078 adds optional Rust header snapshots/merge policy and pinned internal
and public-oracle characterization. Update header, Context/navigation, resource,
dependency, Node/task, package and testing records. Native installation and
public setters are still open; internal agreement is not full API compatibility.

NODE-077 extends public navigation with executed-document close, timeout
continuation and candidate Worker ownership/queued-result tests. Update Frame
navigation, Page close/reload relationship, object/resource ownership, Node/task
and package/testing records. Preserve the intermittent native close failure;
clean Worker exit is not a post-destruction RAM measurement or full parity.

NODE-076 exposes optional public Node Frame.goto and composable native profiles.
Update navigation, Node/package, dependency, object/publication/resource and
task/testing records. Keep failed oracle close runs distinct from independently
verified Rust cleanup and do not promote subset results to full compatibility.

NODE-075 connects typed navigation values to the Rust host executor with
independent result leases. Update navigation, ownership/resources, publication,
dependency, Node/task and test records. Host result projection is distinct from
generated server installation and public Node differential completion.

NODE-074 adds separately selected navigation 1.2 contracts, typed codecs and
server bindings. Update navigation, generator/IDL, dependencies, object/resource
ownership, Node design, tasks and testing records. Typed stream verification is
not production host integration or a public Node/browser differential result.

NODE-073 joins retained native routes and ordered navigation into the selected
Rust host executor. Update navigation, ownership/resources, publication/topology,
dependency, Node design, tasks and testing records. Dynamic streaming cancellation
tests remain distinct from generated/public Node differential evidence.

NODE-072 adds ordered Page navigation registrations and separate Rust host
selection. Update navigation, topology/publication, ownership/resources,
dependency, Node design, tasks, testing and shared comparison records. Actual
host observation is separate from host-owned dispatch and public Frame.goto.

NODE-071 adds an optional per-invocation navigation matcher/response owner and
pinned internal oracle. Update navigation, ownership/resources, dependencies,
Node design, tasks, testing and shared comparison records. Do not treat the
standalone state machine as a production host or public Frame.goto gate.

NODE-070 adds bounded multi-Page lifecycle composition and opt-in Rust host
installation. Update navigation, topology, publication, resource/ownership,
dependency, Node, task and test records. Readiness selection is not a navigation
response/operation implementation or public Node compatibility gate.

NODE-069 adds opt-in production ordered lifecycle observation and a one-Page
resource owner. Update navigation, topology, publication, resource/ownership,
dependency, Node, task and test records. Driver integration is not completion of
multi-Page/Worker ownership, navigation waiters or public Frame.goto.

NODE-068 changes production document-commit descendant retirement and its
runtime/route publication. Update navigation, topology, Frame publication,
object/resource ownership, Node design, task and test records. Live renderer
handoff does not establish full lifecycle or public Frame.goto parity.

NODE-067 adds bounded native Frame lifecycle normalization, borrowed notices and
live single-renderer CDP verification. Update navigation, topology, resource,
ownership, dependency, Node, task and test records. Native-adapter success is not
production event-driver integration, OOPIF/worker parity or public Frame.goto.

NODE-066 adds an optional transaction-published retained Frame navigation table.
Update navigation, topology publication, resources/ownership, dependencies, Node
and task/testing records. Rust route tests do not establish public Frame.goto or
native cancellation; the host has not selected this owner yet.

NODE-065 adds the default-off normalized Frame lifecycle reducer and pinned
internal differential traces. Update navigation, resource/ownership/dependency,
Node/task and testing records. Neither native normalization nor public Frame.goto
is completed by this state-machine gate.

NODE-064 adds world-independent borrowed Frame navigation routes, explicit
native Frame initiation and six pinned local/OOPIF observations. Update the
navigation design, ownership/resources/dependencies, Node/task and testing docs.
Native initiation and final-state probes do not establish public Frame.goto parity.

NODE-063 public Frame lookup binds Rust selection to scoped Node language values
and cache-aware selected ancestry. Update lookup/publication, resource/ownership,
dependency, Node/task and testing records; distinguish the 41-case public gate
from broad compatibility and still-open Frame/event/connection work.

NODE-062 Context baseURL ownership/navigation and its original-layout live gates
now pass: 16 public observations, four boundary groups, Page 7/7 and glob
1647/1647 plus seven checks. Update Context creation, resources, object ownership,
dependency, Frame lookup/publication, Node/task and testing records. Keep earlier
attachment failures and the unresolved reliability investigation visible.

NODE-061 reuses and corrects the existing attachment collector, adds an async
layout wrapper and refreshes unchanged-artifact original-layout results. Update
topology diagnostics, lookup, Node/task and testing records; preserve the open
connection reliability investigation separately from the newly passing runs.

NODE-060 adds the actual N-API URL-value port, native-copy admission and a
labelled browser diagnostic. Update lookup/publication, resource, ownership,
dependency, Node/task and testing records. Preserve failed original-layout
attachment/Page regressions separately from the successful late-observer run.

NODE-059 adds Rust URL-glob preprocessing/restoration and a separately owned
URL-resolution port. Update the lookup, resource, ownership, dependency, Node,
task and test/package records; distinguish diagnostic parser behavior from its
still-required production allocation admission and public API integration.

NODE-058 implements caller-admitted Rust lookup traversal and resolved-glob
planning. Its resource proof, offline oracle bridge and source attribution update
the Frame lookup, ownership, resource, dependency, Node/task and testing records.

NODE-057 adds a version-pinned Frame lookup design/characterization record and
optional Mouse focus/setup diagnostics. Keep diagnostic experiments distinct
from candidate conformance and artifact-specific regression results.

NODE-056 navigation names update the Frame publication, object model, resource,
dependency, Node FFI, task and test/package records. Name storage is an explicit
optional ledger; public lookup and Frame events remain separate unfinished work.
The zero-input Mouse diagnostic and failing regression are additionally tracked
in `knowledge/pwright-raw-mouse.md`; historical passes are artifact-specific.

Update the matching document and its freshness timestamp when a trigger changes.
Existing documentation is indexed here; listing does not imply every legacy page
has been re-reviewed. Canonical technical designs remain Markdown.

Retained Frame host projection, pre-native metadata/root-binding admission, parent-scoped
child release and paused-OOPIF initialization are tracked in
`knowledge/pwright-frame-publication.md`, with linked resource, dependency,
Node package, input-regression and testing records.

Cache-aware descriptor reads, native Page-tree boxing, four-state pre-creation
admission and Node user-script activation additionally update the value-evaluation
contract and the public Page-enumeration comparison records.

Optional runtime lifetime handoff, its last-reference signal-byte budget and
Frame-to-Page input phase contract are recorded in `knowledge/pwright-chromium-drag.md`,
with resource/dependency summaries and linked Locator, typing, Node and test docs.

Selected Node artifact build/loading, compiled-capability verification, shared
artifact provenance and native/public input results also update the Node package
README, Node FFI design, dependency topology and comparison/testing guides.

| File | Covers | Update When |
| --- | --- | --- |
| [README.md](README.md) | pwright Documentation | Crate/package layout, capabilities, or documentation navigation changes |
| [architecture-v2.md](architecture-v2.md) | Composable architecture v2 (discussion draft) | Behavior, API, or architecture described in this document changes |
| [architecture.md](architecture.md) | Architecture | Behavior, API, or architecture described in this document changes |
| [bugs/gmail-cdp-click-limitation.md](bugs/gmail-cdp-click-limitation.md) | Gmail (and similar SPAs) cannot be controlled via pwright click | Behavior, API, or architecture described in this document changes |
| [bugs/index.md](bugs/index.md) | Bug Reports | Behavior, API, or architecture described in this document changes |
| [bugs/tab-leak-bug-report.md](bugs/tab-leak-bug-report.md) | Bug Report: `with_page` Tab Leak Under CDP Connection Failure | Behavior, API, or architecture described in this document changes |
| [bugs/targetinfo-http-parse-failure.md](bugs/targetinfo-http-parse-failure.md) | Bug: TargetInfo struct fails to parse Chrome HTTP /json/list response | Behavior, API, or architecture described in this document changes |
| [cli-guide.md](cli-guide.md) | CLI Guide | Behavior, API, or architecture described in this document changes |
| [code-quality-findings.md](code-quality-findings.md) | Code Quality Findings | Behavior, API, or architecture described in this document changes |
| [codelabs.md](codelabs.md) | Codelabs: pwright Usage Guide | Behavior, API, or architecture described in this document changes |
| [design-tasks-v2.md](design-tasks-v2.md) | Pwright Rust v2 design tasks | Design decisions, implementation checkpoints, or verification gates change |
| [exploration/github-trending-structure.md](exploration/github-trending-structure.md) | GitHub Trending HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/reddit-structure.md](exploration/reddit-structure.md) | Reddit HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/weibo-structure.md](exploration/weibo-structure.md) | Weibo HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/wikipedia-structure.md](exploration/wikipedia-structure.md) | Wikipedia HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/x-structure.md](exploration/x-structure.md) | X.com (Twitter) HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/xueqiu-structure.md](exploration/xueqiu-structure.md) | Xueqiu HTML Structure | The investigated site structure or automation recipe changes |
| [exploration/zhihu-structure.md](exploration/zhihu-structure.md) | Zhihu HTML Structure | The investigated site structure or automation recipe changes |
| [feedback-from-gemini-pdf-extractor.md](feedback-from-gemini-pdf-extractor.md) | docs/feedback-from-gemini-pdf-extractor.md | Behavior, API, or architecture described in this document changes |
| [getting-started.md](getting-started.md) | Getting Started | Behavior, API, or architecture described in this document changes |
| [grpc-api.md](grpc-api.md) | gRPC API Reference | Behavior, API, or architecture described in this document changes |
| [guides/remote-cdp.md](guides/remote-cdp.md) | Connecting to Chrome on a Remote Machine | Behavior, API, or architecture described in this document changes |
| [index.md](index.md) | pwright - Project Guide | Crate/package layout, capabilities, or documentation navigation changes |
| [knowledge/api-parity.md](knowledge/api-parity.md) | API Parity: gRPC vs CLI vs Playwright Bridge | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/cdp-codegen-design.md](knowledge/cdp-codegen-design.md) | Design: CDP Protocol Codegen Crate (`pwright-cdp-gen`) | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/chrome-devtools-mcp-comparison.md](knowledge/chrome-devtools-mcp-comparison.md) | pwright CLI vs chrome-devtools-mcp CLI | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/code-review-2026-03-29.md](knowledge/code-review-2026-03-29.md) | Code Review Findings - 2026-03-29 | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/critical-review-2026-03-29.md](knowledge/critical-review-2026-03-29.md) | Critical Review Findings - 2026-03-29 | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/feature-comparison.md](knowledge/feature-comparison.md) | Feature Comparison: Playwright vs PinchTab vs pwright | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/network-capture-design.md](knowledge/network-capture-design.md) | Design: Network Capture CLI | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/playwright-api.md](knowledge/playwright-api.md) | Playwright API Compatibility | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/playwright-design-course.md](knowledge/playwright-design-course.md) | Playwright design and capability course | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-artifacts-and-streams.md](knowledge/pwright-artifacts-and-streams.md) | Pwright Artifact and Stream boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-browser-close.md](knowledge/pwright-browser-close.md) | Playwright-aligned Browser close and disconnect design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-chromium-topology-events.md](knowledge/pwright-chromium-topology-events.md) | Chromium topology-event driver | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-chromium-drag.md](knowledge/pwright-chromium-drag.md) | Drag primitives, opt-in Mouse/Keyboard/Locator host selection, Page quarantine and retained utility-world snapshots | Feature selection/dependencies, original-Arc native extension views, Frame source/keyboard snapshot selection and retention, single-world destruction receipts, live-state reads, Mouse branches, Page drag state, capacity/admission, terminal retirement/quarantine/driver lifetime, host integration or native/differential evidence changes |
| [knowledge/pwright-cdp-event-admission.md](knowledge/pwright-cdp-event-admission.md) | Optional pre-admission of CDP events, selected Frame-world policy and extension cursor accounting | Event classification, profile selection, fanout derivation, resource/validation ordering or verification changes |
| [knowledge/pwright-context-creation.md](knowledge/pwright-context-creation.md) | Playwright-aligned BrowserContext creation design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-dependency-topology.md](knowledge/pwright-dependency-topology.md) | Pwright v2 dependency topology | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-element-handle-adoption.md](knowledge/pwright-element-handle-adoption.md) | ElementHandle adoption | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-generator-output-ownership.md](knowledge/pwright-generator-output-ownership.md) | Pwright generator output ownership | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-handle-evaluation.md](knowledge/pwright-handle-evaluation.md) | Handle evaluation, raw retained-result acquisition and bounded failure compensation | Capability, native acquisition/settlement, parser ownership, resource contract or verification changes |
| [../crates/pwright-composition-chromium-handle-evaluation/README.md](../crates/pwright-composition-chromium-handle-evaluation/README.md) | Independent Handle evaluation leaf and optional staged native creation | Public leaf APIs, selection, acquisition/settlement, native/publication ownership or continuation obligations change |
| [knowledge/pwright-node-handles.md](knowledge/pwright-node-handles.md) | Node retained-Handle integration, upload prerequisites and public conformance lane | Node Handle producers, publication, ownerFrame/disposal/adoption, borrowed payload target, budgets or verification changes |
| [knowledge/pwright-handle-lifecycle.md](knowledge/pwright-handle-lifecycle.md) | Handle lifecycle, native compensation, shutdown ordering and continuation failure reporting | Native ownership, release settlement, shutdown-before-drain order, retirement proof, resource contract or verification changes |
| [knowledge/pwright-handle-operations.md](knowledge/pwright-handle-operations.md) | Handle-local operations, property snapshots and acquisition ownership | Property/result semantics, acquisition/cleanup, batch ownership, resource contract or verification changes |
| [knowledge/pwright-improvements.md](knowledge/pwright-improvements.md) | pwright Improvements | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-focus.md](knowledge/pwright-locator-focus.md) | Pwright Locator focus and blur design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-evaluation.md](knowledge/pwright-locator-evaluation.md) | Locator evaluation, paired preparation, host-owned continuation, Node conversion rendezvous and native retirement receipts | Resolution/adoption deadlines, route/proof retention, world destruction versus session-object retirement, host lifecycle, async storage bounds, Node roots/exception/binary timing or verification changes |
| [knowledge/pwright-locator-press.md](knowledge/pwright-locator-press.md) | Locator press, retained Frame focus / Page keyboard selection, cancellation and native detach lifetime discrepancy | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-sequential-typing.md](knowledge/pwright-sequential-typing.md) | Public Rust-backed typing, opt-in Page input policy and artifact-specific regression evidence | Typing semantics, bounds, cancellation, modifier fields, wire/FFI integration or oracle evidence changes |
| [knowledge/pwright-raw-keyboard.md](knowledge/pwright-raw-keyboard.md) | Keyboard contracts, bounded services, controlled dispatch, retained Page input and public Rust host/Node API | Named key resolution, schemas/generated adapters, Page routing/revocation, shared input state, FFI exposure, budgets, dependency proofs or oracle evidence changes |
| [knowledge/pwright-raw-mouse.md](knowledge/pwright-raw-mouse.md) | Raw Mouse, six-command protocol, Page-selected multi-phase dispatch, standalone button retirement, retained Page/runtime invocations, host retirement, bounded first-poll ordering and public Node facade | Mouse semantics, drag delegation, Page provider selection/teardown, live-state reader retention, schema/adapter ownership, validation, concurrency, scheduling, native/route/provider/FFI accounting, cancellation identity, receipt delivery, host/Worker shutdown, Node API or artifact evidence changes |
| [knowledge/pwright-locator-screenshot.md](knowledge/pwright-locator-screenshot.md) | Playwright Locator screenshot composition | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-file-input.md](knowledge/pwright-file-input.md) | Rust file-input preparation, native/payload locality and Node upload implementation | File-input source/target semantics, ownership, resource policy, selected dependency closure or verification changes |
| [../crates/pwright-file-input-fs/README.md](../crates/pwright-file-input-fs/README.md) | Optional file-input filesystem provider and pinned MIME data | Filesystem API, supported platform, MIME licensing or verification commands change |
| [../crates/pwright-file-input-tokio/README.md](../crates/pwright-file-input-tokio/README.md) | Admitted blocking file-input executor | Queue/source admission, cancellation, drain, runtime or verification requirements change |
| [../crates/pwright-backend-cdp-file-input/README.md](../crates/pwright-backend-cdp-file-input/README.md) | Chromium file-input mutation and live validation | DOM semantics, encoding/admission, native ownership, licensing or verification commands change |
| [../crates/pwright-composition-chromium-file-input/README.md](../crates/pwright-composition-chromium-file-input/README.md) | Single-use upload continuation and cleanup | Operation admission, settlement, cancellation, cleanup ownership or verification changes |
| [../crates/pwright-file-input-source/README.md](../crates/pwright-file-input-source/README.md) | Optional file-input content ownership | Materialization API, ownership or execution requirements change |
| [knowledge/pwright-locator-scroll-into-view.md](knowledge/pwright-locator-scroll-into-view.md) | Pwright Locator scroll-into-view design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-select-option.md](knowledge/pwright-locator-select-option.md) | Pwright Locator select-option design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-locator-set-checked.md](knowledge/pwright-locator-set-checked.md) | Pwright Locator set-checked design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-network-lifecycle.md](knowledge/pwright-network-lifecycle.md) | Pwright network lifecycle | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-extra-http-headers.md](knowledge/pwright-extra-http-headers.md) | Optional Rust Context/Page header policy, ownership, native installation design and pinned comparison | Header merging/referrer rules, budgets, native fanout/rollback, FFI selection or verification changes |
| [knowledge/pwright-object-model.md](knowledge/pwright-object-model.md) | Pwright browser object-model ownership | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-frame-publication.md](knowledge/pwright-frame-publication.md) | Retained Frame metadata, bounded tree reads/bindings, public collections and shared event-loop publication | Frame identity, parent/detach/close behavior, host/Node integration, JS cache ownership, publication sequencing, resource contracts or paired evidence changes |
| [knowledge/pwright-frame-lookup.md](knowledge/pwright-frame-lookup.md) | Version-pinned Frame lookup semantics, Rust/FFI design and implementation gates | Oracle version, matcher behavior, traversal/cache ownership, callback/exception handling, budgets, Context baseURL/order dependencies or verification changes |
| [knowledge/pwright-frame-navigation.md](knowledge/pwright-frame-navigation.md) | Frame-native routing/initiation, lifecycle/FFI design and pinned local/OOPIF evidence | Route/session identity, command schemas, navigation waits/errors/cancellation, resource ownership, public Frame.goto or differential evidence changes |
| [knowledge/pwright-page-close.md](knowledge/pwright-page-close.md) | Playwright-aligned Page close design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-close-cascade.md](knowledge/pwright-close-cascade.md) | Concurrent native close intents, ancestor retirement and serialized publication design | Native close scheduling, reservation/transaction ownership, ancestor proof, Context interruption semantics or differential evidence changes |
| [knowledge/pwright-node-lifecycle-events.md](knowledge/pwright-node-lifecycle-events.md) | Node lifecycle callback ordering, red differential evidence and stepwise Rust publication design | Event arguments/state/order, Rust publication ownership, FFI callback/admission, artifact evidence or implementation status changes |
| [knowledge/pwright-page-content.md](knowledge/pwright-page-content.md) | Playwright Page content semantics and Pwright boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-page-reload.md](knowledge/pwright-page-reload.md) | Typed reload, Node FFI, bounded host/raw-CDP close controls and loader/request correlation | Reload behavior, ownership, wire/API exposure, diagnostic bounds or paired evidence changes |
| [knowledge/pwright-page-creation.md](knowledge/pwright-page-creation.md) | Playwright-aligned Page creation design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-process-memory-metrics.md](knowledge/pwright-process-memory-metrics.md) | Pwright process-memory attribution | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-protocol-idl.md](knowledge/pwright-protocol-idl.md) | Pwright protocol IDL decision record | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-screenshot.md](knowledge/pwright-screenshot.md) | Playwright screenshot semantics and Pwright boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-tracing.md](knowledge/pwright-tracing.md) | Pwright tracing boundary | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/pwright-value-evaluation.md](knowledge/pwright-value-evaluation.md) | Playwright-aligned value evaluation design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/recipe-catalog.md](knowledge/recipe-catalog.md) | Recipe Catalog | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/recipes-design.md](knowledge/recipes-design.md) | Recipes Design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/script-runner-design.md](knowledge/script-runner-design.md) | Script Runner Design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [knowledge/testing-strategy.md](knowledge/testing-strategy.md) | Testing strategy, bounded lifecycle tracing and per-case/aggregate supervision | Capability, ownership, resource contract, supervisor limits or verification evidence changes |
| [knowledge/url-capture-design.md](knowledge/url-capture-design.md) | Raw URL Capture Design | The named capability, ownership boundary, resource contract, or verification evidence changes |
| [known-issues.md](known-issues.md) | Known Issues | Behavior, API, or architecture described in this document changes |
| [lightpanda-compat.md](lightpanda-compat.md) | Lightpanda Browser Compatibility | Behavior, API, or architecture described in this document changes |
| [node-ffi-compatibility-design.md](node-ffi-compatibility-design.md) | Playwright-compatible Node FFI design and bounded attachment/retained-world diagnostics | Node facade, Rust host/addon, runtime extension/lifecycle, compatibility, dependencies, attachment pressure, retained-world observations or differential evidence changes |
| [recipes-for-agents.md](recipes-for-agents.md) | Pre-compiled Browser Skills for AI Agents | Behavior, API, or architecture described in this document changes |
| [resource-boundaries-v2.md](resource-boundaries-v2.md) | Pwright v2 resource ownership and limits | Behavior, API, or architecture described in this document changes |
| [tech-debt.md](tech-debt.md) | Tech Debt | Behavior, API, or architecture described in this document changes |
| [todo.md](todo.md) | TODO | Behavior, API, or architecture described in this document changes |
| [Node package README](../packages/pwright/README.md) | Local native build, supported public subset, screenshot ownership and differential commands | Package scripts, loading, API coverage, supported targets, resource contracts or test setup changes |
| [Shared comparison README](../tests/README.md) | Shared Node-FFI paired corpus, legacy golden lanes, report classifications and oracle diagnostics | Shared adapters, suite catalog, fixtures, report schema, commands, limits or verification evidence changes |
| [MANIFEST.md](MANIFEST.md) | Documentation update routing | A document is added, removed, or changes responsibility |
