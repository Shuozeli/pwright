<!-- agent-updated: 2026-09-17T08:34:48Z -->

NODE-169 (2026-09-17): The default-off safe-host `retained-file-input` feature
selects this leaf's borrowed retained-source view together with the resolver's
group-owned adoption path. It combines Handles and file input without exposing
an addon feature or public Node retained-upload API. A retained descriptor is
not a native-use pin: the host checks source/world identity around adoption,
using the original creation Frame's utility world rather than the current DOM
owner's Frame.

Separate admitted, untimed phase cleanup owns adoption and submitted upload.
Abandoned adoption continues acquisition and group cleanup only, never a new
mutation; cleanup failures remain visible during shutdown. The Node preparation
ticket and observable deferred ingress remain integration work.
[Canonical NODE-169 contract, exact verification and remaining gates](../../docs/knowledge/pwright-node-handles.md#node-169-safe-host-retained-upload-and-pinned-ingress-oracle)
own the evidence. The pinned offline ingress oracle is not a live browser pass;
earlier entries below retain their historical scope.

NODE-168 (2026-09-17): Legacy ElementHandle-adoption abandonment now preserves
an incoming Cleanup result as continuation failure instead of silently marking
it successful. This does not convert that legacy adoption API into the new
group-owned retained-target path. The separate resolver leaves expose optional
`retained-reference`, returning their actual `ResolvedElement`/`OwnedElement`
without querying a selector and retaining caller source/route ownership through
the pre-admitted cleanup continuation.

That path receives coherent numeric and unique utility IDs from one Frame
snapshot. Its freshly resolved group may accept a destruction proof only after
native argument validation in the exact unique world; before validation,
cleanup requires explicit group release through settlement. Numeric reuse must
not borrow the expected old world's proof. Node preparation/rendezvous wiring
and public retained upload remain absent.
[Canonical NODE-168 contract and verification](../../docs/knowledge/pwright-node-handles.md#node-168-group-owned-retained-adoption-and-deferred-upload-ingress)
owns exact results and remaining gates; no live-pass claim is made here.
NODE-167 and earlier entries below remain historical checkpoints.

NODE-167 implementation contract (2026-09-17):
`owner_frame_native_id(target)` queries the retained node's current DOM owner,
not its creation Frame. It temporarily retains the current document element,
uses direct raw `DOM.describeNode` on that root, and settles its explicit native
release before returning the bounded native Frame ID. Removed nodes can retain
an owner; orphan documents return no owner. The embedding host maps the result
through current topology within the source BrowserContext after publication.

The ownerFrame-only acquisition validates and copies one protected receiver ID
after evaluation admission. It rejects an otherwise successful source-alias
reply and skips that borrowed ID during malformed/exception compensation,
while still attempting every distinct newly owned result/exception release.
Existing acquisition callers are unchanged: this is not generic protection for
all borrowed IDs, nor a native-release pin against concurrent source disposal.

For public navigation errors, the host captures an immutable source-world
fingerprint at scalar and batch publication: Page ID, numeric context ID and
`Arc<str>` unique context ID, within the existing 16 KiB descriptor allowance.
It retains no client, receipt, permit or native lease. Only the same Ready Page
with an observed replacement main world (changed numeric or unique ID) permits
the canonical navigation error before lookup or refinement of ordinary
TargetClosed. Missing worlds, crashes and closes are not inferred to be
navigation; Cleanup remains untouched. This source change is not live evidence.

The raw caller must pre-own a continuation and retain the source, future and
native admission through acquisition, describe and cleanup settlement, including
cancellation. The selected Node host shares its eight call slots and admits two
transient `RemoteHandle` units per query: sixteen aggregate at pending scope
`extension(154, 1)`. No slot from the 64 persistent-result owners is consumed.
The handles-only Node API returns canonical Frame wrappers and propagates that
resolver through Handle descendants. This adds no retained upload API.
Unaddressable raw IDs/pre-owned groups and conservative Cleanup-versus-detach
outcomes remain open. [Exact verification and remaining gates](../../docs/knowledge/pwright-node-handles.md)
are canonical; this is not a live-pass claim. NODE-166 and earlier entries below
are historical checkpoints.

NODE-166 (2026-09-17): The existing default-off `native-creation` feature now
also exposes `stage_get_properties(target, metadata, maximum_results)`. The
caller admits N result owners; the leaf reserves N routes and distinct
ObjectIds before browser I/O, trims unused admission downward, and returns a
`PendingNativeHandleBatchCreation`. Its `commit_or_defer` publishes all siblings
atomically with their runtime identities, or retains/rejects the entire batch.
Every attempt checks the original source world and target. Zero admission is
valid only for a primitive's no-I/O empty result. Each published sibling is
Frame-owned, independently disposable, and has its own ObjectId cleanup identity.

The backend now selects raw `runtime_get_properties_to_settlement` and holds
the same evaluation lease from carrier acquisition through property read and
carrier/failure releases. Normal native temporary peak is one carrier plus
N+1 overflow-detection values, distinct from N publishable results. Bounded-depth
structural accounting checks simultaneous parser/name/value staging before
copying; the incoming raw response remains subject to CDP response limits.
Failure cleanup borrows/deduplicates addressable response IDs and attempts all
releases, including exception and unexpected descriptor owners. Unaddressable
IDs remain honest Cleanup failures. The shared parser stays pure.

The raw caller still owns cancellation/recovery: retain its future through
settlement, pre-admit per-sibling cleanup plus aggregate compensation and the
started-call continuation, and explicitly release abandoned batch owners.
Typed batch compensation cannot hide a generic release failure behind a
different sibling's TargetClosed result. No new package, feature or portable
schema is introduced. Legacy portable `getProperties` still admits its route
batch after native retrieval; it does not gain staged pre-I/O route admission.
The browser snapshot's `Object.keys` allocation is not bounded by these host
or native-wrapper limits. Unaddressable-ID group design, conservative
compensation-versus-detach outcomes and full-corpus compatibility remain open.
[Exact Node verification and remaining gates](../../docs/knowledge/pwright-node-handles.md)
are canonical; the NODE-165 and earlier entries below are historical checkpoints.

NODE-165 (2026-09-17): The existing backend now explicitly selects raw
retained-result CDP acquisition and failure-compensation release through settlement. It preserves
exception IDs, borrows and deduplicates at most two result/exception IDs,
and compensates parse/JavaScript failures while its evaluation lease remains
held. The shared parser stays pure. Unaddressable IDs and refused cleanup
remain Cleanup, including in abandoned-creation continuation health.

Raw futures still require caller-owned continuation through settlement;
the new CDP methods do not add drop-triggered cleanup. Ordinary CDP methods
are unchanged, and custom clients must implement the explicit methods instead
of relying on a lossy fallback. No package, feature, portable schema or Node
API is added. [Canonical verification and follow-ups](../../docs/knowledge/pwright-node-handles.md)
retain the Runtime.getProperties late-settlement/full-pipeline-admission gap,
the pre-owned-group design for unaddressable IDs, and unfinished Node
batch/full-corpus work.

Custom clients implement `runtime_evaluate_as_object_to_settlement`,
`runtime_call_function_as_object_in_unique_context_to_settlement` and
`runtime_release_object_to_settlement` for this boundary. This is not a change
to ordinary `CdpEvaluatedHandle::release` or `Runtime.getProperties` settlement.

NODE-164 (2026-09-17): Portable HandleGetProperty now uses the identity-preserving
property snapshot backend, admits its persistent route before I/O, and shares
single-result publication/compensation. Abandoned Cleanup outcomes propagate
failure to the continuation owner. This supersedes the previous portable
property implementation gap without adding a feature or dependency package.
The Node addon separately implements module-local raw-class TypeTag checks.
[Exact verification and remaining gates](../../docs/knowledge/pwright-node-handles.md)
are recorded canonically; Node getProperties is not implemented.

NODE-163: native-creation adds stage_evaluate_program_for_handle and
stage_get_property. Each result owns route admission before native execution,
remains parented to the source Frame, and revalidates its target/arguments at
local publication. Property reads preserve Promise/thenable identity instead
of applying evaluateHandle's awaited-result semantics. The host retains raw
futures and abandonment recovery; this leaf adds no executor or automatic
drop-triggered cleanup. Batch staging remains unimplemented.

# Chromium Handle evaluation composition

NODE-162: CdpHandleEvaluationProgram separates native invocation mode from the
unchanged portable 1.2 model. It supports expressions with bounded arguments
that must still be validated/resolved, without rewriting source or wrapping the
graph. With native-creation, stage_evaluate_program has the same unpublished
owner/recovery contract as stage_evaluate_handle. With frame-value,
evaluate_program_for_frame adds native by-value programs. Independent default-off
handle-value adds evaluate_program_for_handle; no result Handle is allocated.
These APIs reuse route/adoption resolution and existing call/resource limits.
Their raw futures still require caller-owned cancellation and retained-argument
lifetimes. The Node host provides those obligations through its handles profile.
Portable wire codecs and their supported parameter shapes do not change.

NODE-161: The independent, default-off `frame-value` feature adds
`evaluate_value_for_frame` for Frame by-value evaluation with native Handle
arguments. It reuses existing argument/adoption resolution, value backend and
bounded resource configuration; it allocates no temporary result Handle, adds
no dependency package and registers no extra wire protocol. Cancellation and
host ownership remain the caller's responsibility, as for other raw value calls.
The Node safe host selects this feature only with its explicit handles profile.

This crate is the independently installable Chromium server composition for
`pwright.handle_evaluation` 1.2. It implements Frame `evaluateHandle` plus
Handle `evaluate`, `evaluateHandle`, `jsonValue`, `getProperty`, and bounded
`getProperties` over the separate Handle-lifecycle leaf.

The caller supplies the controlled runtime, ObjectId source, production
resource ledger, CDP clients and execution-world IDs, and bounded continuation
queues. Installation does not launch or download Chromium, start an executor,
or select a transport.

Single creation and staged native batches reserve Handle-route capacity before
CDP; legacy portable `getProperties` reserves after native retrieval. A
successful single result or property batch publishes runtime object identity
and every lifecycle route as one all-or-nothing commit; failures compensate
all returned native objects.
Handle arguments must belong to the exact same Frame, Page, and CDP client.

The in-process creation continuation completes a dropped, started creation and
attempts disposal of every unobserved result. One release failure does not skip
the rest of a property batch; cleanup still reports failure, preserving refused
native routes for explicit retry or parent retirement. This continuation does
not own a successfully returned Ready result: that responsibility belongs to
the caller's delivery/publication owner.

The installation exposes its exact application protocol and raw command
service, and the lifecycle installation exposes its raw disposer. Remote
servers can pass those three pieces to
`pwright-plugin-ordered-handle-delivery`, which adds bounded continuation,
ordered Create-before-Result, Result-abandonment cleanup, and ordered Dispose
without adding transport policy to this CDP composition leaf.

## Optional staged native creation

The empty `native-creation` feature exposes
`ChromiumHandleEvaluationCommandService::stage_evaluate_handle` for Frame
creation. It executes native evaluation once and returns an unpublished owner.
At the caller's serialized lifecycle boundary, `commit_or_defer` returns
`Published`, `Deferred` with the same owner, or `Rejected` with native cleanup
still owned. A deferred retry performs no browser I/O. Every commit attempt
revalidates the frozen source world and referenced Handles.

The `release_native` methods additionally expose typed native compensation;
rejected publication preserves the primary failure separately. A host can use
this with its own exact world-retirement witness, without parsing strings or
treating arbitrary detach errors as success. NODE-159 selects this staged path
in the safe Rust host and supplies its bounded publication/recovery queue.

The caller must pre-admit future/queue/compensation storage, retain native work
through settlement and explicitly release abandoned or rejected results.
Dropping an unpublished owner does not send browser cleanup. This leaf installs
no scheduler or automatic abandonment recovery, and existing continuation
wrappers do not wrap this separate raw staging API. `stage_get_properties`
returns the corresponding batch owner with `len`, `is_empty`, `world_identity`,
`commit_or_defer`, `release`, and `release_native`. A failed batch preserves its
primary error separately from attempt-all native compensation. The selected
Node handles profile supplies caller-owned publication and recovery for both
single and batch entrypoints.
See [ownership and verification](../../docs/knowledge/pwright-node-handles.md#node-158-staged-native-creation-before-portable-publication).
