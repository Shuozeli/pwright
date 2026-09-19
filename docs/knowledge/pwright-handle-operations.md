<!-- agent-updated: 2026-09-17T07:39:41Z -->

# Pwright Handle-local operations design

## NODE-167 current DOM owner contract (2026-09-17)

The handles-only Node profile now exposes `ElementHandle.ownerFrame()`. Rust
resolves the retained node's current owner document, temporarily retains its
document element, reads that root with direct raw `DOM.describeNode`, and awaits
native cleanup settlement. Creation Frame identity and DOM attachment are not
substitutes for this lookup: adoption can change the owner, a removed node can
keep its owner, and an orphan document has none. After topology publication,
the host selects a current Frame in the source BrowserContext; JavaScript only
returns its canonical cached wrapper or null. `evaluateHandle`, `getProperty`
and `getProperties` descendants retain the same context-scoped resolver.

Only this ownerFrame acquisition validates and copies one protected receiver ID
after evaluation admission. A source-alias success is rejected; malformed or
exception compensation skips the borrowed ID while attempting every distinct
new owner. Other acquisition paths are unchanged, so this is neither generic
borrowed-ID protection nor a release pin against explicit source disposal.

The host also records the immutable source-world fingerprint at scalar and
batch publication (Page ID, numeric context ID and `Arc<str>` unique context ID)
inside the existing 16 KiB descriptor allowance, with no client, receipt, permit
or native lease retained. Only that same Page in Ready state with an observed
replacement main world permits the canonical navigation error before lookup or
refinement of ordinary TargetClosed. Missing worlds, crashes and closes do not
prove navigation, and Cleanup errors are not reclassified. Live verification of
this refinement belongs in the canonical record, not this contract checkpoint.

A raw caller must pre-own continuation and cleanup admission, retaining the
source and entire future through settlement even after cancellation. The host
uses its eight shared call slots plus two transient `RemoteHandle` units per
query, sixteen aggregate at pending scope `extension(154, 1)`; it consumes none
of the 64 persistent-result slots. Unaddressable raw IDs/pre-owned groups and
conservative Cleanup-versus-detach outcomes remain open. Retained
`ElementHandle.setInputFiles` is not implemented by this checkpoint.
[Exact verification and remaining gates](pwright-node-handles.md) are canonical;
this implementation checkpoint makes no live-pass claim. NODE-166 and earlier
entries below remain historical evidence within their stated scopes.

## NODE-166 staged batch and property-settlement contract (2026-09-17)

The existing `native-creation` selection adds
`stage_get_properties(target, metadata, maximum_results)`. Caller-admitted N
result slots become N reserved routes and distinct ObjectIds before browser
I/O; unused routes trim downward. One unpublished batch owns every native result
until an all-or-none runtime/route commit or explicit attempt-all compensation.
Every publication retry revalidates the target and frozen source world, and a
busy lifecycle transaction does not repeat evaluation. Primitive targets may
return an empty N=0 result with no browser I/O; nonprimitive N=0 is rejected.
Sibling ownership is independent and uses ObjectId cleanup identities.

The backend uses explicit raw `runtime_get_properties_to_settlement` and keeps
the original evaluation allowance through acquisition, property read, carrier
release and failure compensation. Its normal temporary native peak is one
carrier plus N+1 overflow-detection values; at most N results transfer to publication.
Before parser copies, bounded-depth structural accounting covers String/Vec
capacities, sparse map storage and simultaneous staging. Raw response allocation
is still constrained by CDP limits. Malformed, exception, overflow and unexpected
descriptor-owner responses compensate all addressable IDs without discarding
later failures; missing/invalid/oversized identities remain Cleanup. The parser
itself remains pure and ordinary CDP methods retain their old contracts.

The Node host admits sibling tickets, one aggregate compensation slot, and its
existing started-call continuation before I/O. A shared call allowance covers
names through language conversion; partial addon conversion revokes already
wrapped siblings and drops remaining Rust owners without relying on GC.
No new package, feature or portable 1.2 schema is introduced. This source contract
does not assert completed compatibility: use [the canonical Node checkpoint](pwright-node-handles.md)
for exact verification and pending gates.

Scope limits remain explicit: legacy portable `getProperties` still admits its
route batch after native retrieval; the browser algorithm's `Object.keys`
allocation is not bounded by these host/native-wrapper limits; pre-owned groups
for unaddressable IDs, conservative compensation-versus-detach failures and the
full Playwright corpus remain follow-ups. NODE-165 and older entries below are
historical checkpoints superseded only within the scope stated here.

## NODE-165 acquisition checkpoint (2026-09-17)

Carrier acquisition now uses the explicit raw retained-result CDP contract,
so malformed result and exception identities survive until failure
compensation. At most two borrowed IDs are deduplicated, each addressable
release is awaited through settlement while evaluation admission is held,
and unresolved IDs remain Cleanup. The shared parser is still pure;
ordinary CDP methods and portable 1.2 schemas do not change.

This supersedes the addressable-ID parse-before-owner implementation gap
recorded below. It does not cover late Runtime.getProperties responses or
evaluation admission across the entire successful property pipeline.
A separately admitted pre-owned group for unaddressable IDs, staged Node
getProperties batch/sibling ownership, and full-corpus compatibility remain
open. Use [the canonical checkpoint](pwright-node-handles.md) for exact
verification rather than carrying historical test counts forward.

## NODE-164 checkpoint (2026-09-17)

Portable HandleGetProperty now uses identity-preserving property snapshots,
pre-admits the persistent route before native I/O, and shares single-result
publication and compensation. This supersedes the portable-command gap in the
historical NODE-163 entry below. Abandoned Cleanup outcomes remain failures.
The separate priority raw-class audit also now has a crate-wide module-local
TypeTag implementation using the actual N-API checks. Exact verification and
remaining gates are recorded in [the canonical checkpoint](pwright-node-handles.md).
Node getProperties still requires staged batch admission, sibling cleanup
identities and complete publication rollback; broader compatibility remains open.

Historical NODE-164 audit priority (addressable-ID acquisition implementation
superseded by NODE-165 above): close the malformed RemoteObject
parse-before-owner cleanup gap; route native-release ownership and failure
evidence to [the canonical checkpoint](pwright-node-handles.md).

## NODE-163 native Node integration checkpoint

The optional native-creation path now stages Handle-targeted evaluateHandle
and getProperty with exact target/world revalidation and independent
Frame-owned results. The selected Node profile publishes these through its
existing Rust driver and owned cleanup continuation. Property extraction uses
a bounded null-prototype carrier so a Promise-valued property remains a Promise
Handle. See [the canonical evidence](pwright-node-handles.md#node-163-handle-targeted-producers-and-failure-safe-publication).

This does not expose Node getProperties: atomic batch admission, sibling owner
identities and partial-language-publication rollback remain required. Portable
1.2 schemas are unchanged. Its older direct get_property command still delegates
to awaited Handle evaluation; this turn fixes and verifies the separate staged
Node path, not that legacy command's Promise behavior. Keep that distinction
explicit until the portable command is repaired and independently tested.

## Decision

Pwright will add the Playwright `JSHandle` local surface as
`pwright.handle_evaluation` version 1.2. It is one coherent optional capability,
not five separately installed micro-capabilities and not part of the base
client.

The version boundary is:

- 1.0: Frame `evaluateHandle` with value-only arguments;
- 1.1: Frame `evaluateHandle` with nested Handle arguments;
- 1.2: Handle-local `evaluate`, `evaluateHandle`, `jsonValue`, `getProperty`,
  and `getProperties`.

This keeps the user-visible installation choice useful: applications that only
need by-value Frame evaluation still install neither Handle lifecycle nor
Handle evaluation. Applications that opt into retained objects get one complete
evaluation/introspection unit. Version negotiation prevents a 1.0 or 1.1 peer
from receiving the new commands.

The capability continues to depend on `pwright.handle_lifecycle` 1.0 because
three of the new commands can create retained Handles and every retained output
must have explicit disposal semantics. It does not depend on the independent
`pwright.evaluation` capability.

## Playwright evidence

Playwright exposes these operations on both `JSHandle` and its `ElementHandle`
specialization:

| Public operation | Server operation | Result |
|---|---|---|
| `evaluate` | `evaluateExpression` | serialized value |
| `evaluateHandle` | `evaluateExpressionHandle` | live Handle |
| `jsonValue` | `jsonValue` | serialized value |
| `getProperty` | `getProperty` | live Handle, including `undefined` |
| `getProperties` | `getPropertyList` | ordered name/Handle pairs |

The authoritative source anchors in the checked-out Playwright tree are:

- `packages/protocol/spec/handles.yml`;
- `packages/playwright-core/src/client/jsHandle.ts`;
- `packages/playwright-core/src/server/javascript.ts`;
- `packages/playwright-core/src/server/dispatchers/jsHandleDispatcher.ts`;
- `packages/playwright-core/src/server/chromium/crExecutionContext.ts`;
- `packages/protocol/src/serializers.ts`.

Important observed semantics are:

- the target Handle is the first argument to the user's function and the
  optional caller argument is second;
- a Handle argument must resolve in the same execution context, except for
  Playwright's separate ElementHandle adoption path;
- `getProperty` returns a Handle even when the property is absent, in which
  case it wraps `undefined`;
- `getProperties` returns only own enumerable data properties and returns an
  empty collection for a primitive Handle;
- every returned property value is a new live Handle;
- serialized values preserve special numbers, dates, URLs, regular
  expressions, errors, typed arrays, and cyclic/shared object graphs;
- `jsonValue` serializes the represented value rather than calling an object's
  user-defined `toJSON` method.

Pwright will preserve those semantics subject to explicit production bounds.
Generic cross-context JSHandle use remains rejected. Cross-world ElementHandle
adoption stays a separate optional compatibility gate.

## Portable commands

Version 1.2 adds five methods to the existing Handle method family:

| Command | Method | Targets | Result |
|---|---:|---|---|
| `handle.evaluate` | `0x0008_0003` | JSHandle, ElementHandle | `EvaluationValue` |
| `handle.evaluate_handle` | `0x0008_0004` | JSHandle, ElementHandle | `HandleInitializer` |
| `handle.json_value` | `0x0008_0005` | JSHandle, ElementHandle | `EvaluationValue` |
| `handle.get_property` | `0x0008_0006` | JSHandle, ElementHandle | `HandleInitializer` |
| `handle.get_properties` | `0x0008_0007` | JSHandle, ElementHandle | ordered property initializers |

Handle-local evaluation accepts a function source and one optional bounded
`HandleEvaluationValue`. It does not expose an expression mode: the target
Handle must always be supplied as the first function parameter. Nested Handle
leaves use the same 1.1 value graph and same first-appearance native argument
resolution.

By-value results use the Handle-free `EvaluationValue` public model. The wire
schema may share the existing tagged representation, but the codec and backend
must reject a Handle leaf in a result. A property-list result is an ordered
`Vec` rather than a map so browser enumeration order is retained and duplicate
names can be rejected instead of silently overwritten.

The typed client methods exist only when the
`pwright-client-handle-evaluation` extension crate is imported. Both typed
Handle kinds receive the same operations. A returned initializer must be
preceded by its exact runtime Create announcement; all property-list Create
announcements must precede the one Result that names them.

## Ownership and routing

Every command starts from the target Handle route. That route supplies the
owner Frame, owner Page, exact target-local CDP client, and current execution
world. The composition holds the Handle-route lock only while it validates and
clones the target plus all Handle-valued arguments. It must release that lock
before native dispatch and must not hold a runtime lifecycle transaction across
awaited CDP I/O. After CDP succeeds, it prepares and commits the output
lifecycle transaction against the then-current parent. If concurrent topology
retirement won the race, that commit fails and the new native Handle is
released instead of being published.

All Handle-valued arguments must match the target's Frame, Page, CDP client,
and current world. A disposed target or argument is `TargetClosed`; a live but
foreign argument is a protocol/value failure. A result Handle inherits the
target's Frame parent and exact route. No native object ID, session ID, target
ID, or execution-context ID crosses the portable wire.

One Handle output uses the existing prepared object-creation transaction.
`getProperties` uses the runtime's prepared batch-creation transaction and an
equivalent batch route publication. Either every property Handle becomes live
in enumeration order or none does. A partial publish is forbidden.

## Chromium leaf

Handle-local `evaluate` and `evaluateHandle` use
`Runtime.callFunctionOn` in the target route's exact `uniqueContextId`. The
browser-side runtime reconstructs the bounded argument graph, substitutes
native Handle slots, and invokes `fn(target, argument)`. The by-value form then
uses Pwright's bounded Playwright-shaped serializer; the retained form parses
the returned `RemoteObject` into a JSHandle or ElementHandle route.

`jsonValue` runs the same serializer on the target value. It must not delegate
to JSON stringification and must not invoke `toJSON`.

`getProperty` reads the named property and retains its value as one Handle.
Property-name bytes are bounded before browser I/O.

`getProperties` must not let an arbitrarily large CDP property-descriptor
response become an unbounded driver allocation. The browser-side operation
therefore snapshots at most `max_properties + 1` own enumerable data
properties into a null-prototype temporary object without invoking accessors.
Receiving the sentinel extra entry is `ResourceExhausted`. The temporary object
is always released. The bounded property descriptors are converted to native
Handles, and every native result is released if validation, resource
reservation, lifecycle preparation, publication, cancellation, or Result
delivery fails.

## Resource and cancellation rules

The production profile must declare and enforce:

- source and generated-script bytes;
- evaluation graph depth, nodes, entries, string bytes, and binary bytes;
- property-name bytes and maximum property count;
- native object-ID and preview bytes;
- concurrent operation count;
- live Handle count and exact retained route bytes;
- continuation and Result-delivery cleanup capacity.

Single-output creation reserves one route slot before native evaluation.
Property-list creation reserves the complete route/resource batch before
portable publication. The native discovery response is independently bounded
by the browser-side snapshot and the CDP transport's frame limit.

Timeout metadata remains unsupported. A remote cancellation identity is
accepted because the controlled channel injects it for operation ownership.
CDP cannot cancel one `Runtime.callFunctionOn` safely, so dropping a started
future transfers it to the already bounded continuation owner. By-value orphan
results are discarded. Every orphan native Handle, including a whole property
batch, is explicitly released and never published.

## Acceptance gates

Version 1.2 is not complete until release-profile evidence proves:

1. schema compatibility against the frozen 1.1 baseline and rejection by a
   negotiated 1.1 codec;
2. codec and typed-client round trips for all five methods and both Handle
   target kinds;
3. target-first function calling, special/cyclic value serialization, and no
   `toJSON` invocation;
4. absent-property `undefined`, primitive empty properties, own/enumerable/data
   filtering, stable order, and duplicate rejection;
5. same-world success plus disposed, cross-Frame, cross-Page, cross-client, and
   replaced-world rejection before inappropriate native I/O;
6. atomic multi-Handle Create-before-Result publication and full compensation
   at every failure/cancellation boundary;
7. exact route counts, bytes, peaks, releases, and preconfigured capacity
   behavior;
8. omission of 1.2 code paths when the capability is absent or negotiates
   below 1.2;
9. a real dedicated-Chrome typed client/server round trip covering all five
   methods while preserving external browser ownership.

Until these gates pass, Pwright has a Handle creation/disposal foundation but
must not claim the complete Playwright `JSHandle` surface.
