<!-- agent-updated: 2026-09-05T20:53:09Z -->

# Pwright Handle evaluation checkpoint

## Scope

This checkpoint adds the first bounded `evaluateHandle` creation path without
changing the independently installable, by-value `pwright.evaluation`
capability. It proves portable Handle identity, Chromium-native routing, and
explicit release as one composition transaction. It is not the complete
Playwright `JSHandle` API.

Version 1.1 adds Handle leaves to its own bounded recursive argument model.
The independently installable value-evaluation capability remains Handle-free.
Version 1.2 now adds the separately typed Handle-local `evaluate`,
`evaluateHandle`, `jsonValue`, `getProperty`, and `getProperties` surface
described in [Handle-local operations](pwright-handle-operations.md). The
separate [ElementHandle adoption](pwright-element-handle-adoption.md) decision
records why a low-level Chromium primitive must precede public cross-world
composition. That primitive is now implemented and verified against Chrome;
the public composition gates remain intentionally closed.

## Independent 1.2 composition leaf

`pwright-composition-chromium-handle-evaluation` now packages all six 1.2
commands without the aggregate Chromium composition. It installs the smaller
Handle-lifecycle leaf, owns only bounded Frame-to-execution-world routes, and
receives the runtime, ObjectId source, CDP clients, resource ledger, and two
caller-owned continuation queues as bindings. The leaf neither launches nor
downloads a browser and does not select Tokio in its normal dependency graph.

Single Handle creation reserves lifecycle capacity before CDP and publishes
the runtime object plus route under one lifecycle-registry commit. Bounded
`getProperties` reserves the complete output batch, prepares one runtime batch,
and publishes every route atomically. Any failed admission or commit releases
all newly returned native objects. Primitive results use the same portable
ownership path but require no native release.

Fifteen AAA integration tests cover the complete command family, exact-world
argument rejection before CDP, batch compensation, pre-CDP continuation
rejection, cleanup of an abandoned started creation, compensation when the
Frame world changes during CDP, and composition with the ordered remote plugin.
Its standalone normal closure is exactly 24 Pwright packages. A dedicated
Chrome 152 run composed the leaf with the remote plugin, observed physical
Create-before-Result and Dispose delivery, evaluated through the real object
Handle, and confirmed the externally owned browser remained alive.

The leaf exposes its exact application protocol, raw command service, and
lifecycle raw disposer. The separate
`pwright-plugin-ordered-handle-delivery` consumes those interfaces when a
remote server is selected. That keeps direct/in-process installation free of
transport policy while making the complete remote ownership chain reusable.
The aggregate Chromium remote-handler factory delegates to this same plugin;
its older decorator types remain public for compatibility but are no longer a
separate active default policy.

The Frame/world owner now also exposes two topology-injection forms. A
standalone owner can atomically `replace_all` one complete route snapshot;
invalid capacity, duplicate Frame, numeric ID, or stable ID preserves the old
snapshot. A larger composition can instead inject a live
`ChromiumHandleEvaluationRouteResolver`, so the command service reads that
composition's already-atomic topology table without keeping a second copy.
The aggregate `CdpPageServices` now implements this resolver and retains both
the numeric execution-context ID needed by DOM adoption and Chromium's stable
unique context ID for main, local-child, and OOPIF routes.

## Optional capability and API

`pwright.handle_evaluation` version 1.1 depends on
`pwright.handle_lifecycle` 1.0 and owns one command:

| Command | Method | Target | Result |
|---|---:|---|---|
| `evaluation.evaluate_handle` | `0x0008_0002` | Frame | `HandleInitializer` |

The initializer contains only portable state: the new ObjectId, its Frame
parent, its concrete `JsHandle` or `ElementHandle` kind, and a bounded optional
preview. A CDP `objectId`, session, or execution-context identifier never
crosses the portable wire.

The opt-in client extension exposes `evaluate_handle` on Frame and Page; Page
delegates to its main Frame. The result is an `EvaluatedHandle` enum containing
the concrete typed client Handle. In remote mode, materialization succeeds only
after the exact matching Create announcement has been received and claimed.
`EvaluatedHandle::as_evaluation_argument` turns that portable identity into a
Handle leaf without exposing CDP state.

The 1.1 value union adds tag 18, `handle { object_id }`. Arrays and objects may
mix serialized values, graph-cycle `Ref` leaves, and Handle leaves. Repeated
Handle IDs are deduplicated in first-appearance order for native dispatch, but
each leaf still counts against the existing depth/node/entry bounds. A 1.0
codec mode continues to accept value-only requests and rejects Handle leaves;
the remote handler selects this mode from the independently negotiated
capability version.

The command codec preserves a remote channel's cancellation identity while
continuing to reject timeout metadata. This is required because the negotiated
client channel synthesizes a cancellation ID before encoding a cancellable
call; dropping the public future can then address the exact registered server
operation without making timeout behavior implicit.

## Chromium transaction

For a routed Frame, the CDP leaf executes `Runtime.callFunctionOn` with
`returnByValue: false`, `awaitPromise: true`, and the Frame route's exact
`uniqueContextId`. Its first by-value argument is the bounded source/value
envelope; subsequent `CallArgument`s carry resolved native objects. A returned
node becomes an ElementHandle; all other returned values become JSHandles.
Object-backed results retain a bounded native CDP object ID. Primitive results
retain a bounded reconstructable CDP call argument but no native object ID, so
later release is an explicit no-op. An unbound low-level service preserves the
old `Runtime.evaluate` path only for requests containing no Handle leaf.

`ChromiumEvaluateHandleCommandService` composes creation in this order:

1. validate the live Frame and exact evaluation route;
2. collect portable Handle leaves and allocate the next output ID;
3. under one route lock, resolve the whole input set and prepare the output
   lifecycle transaction atomically, excluding concurrent disposal;
4. reject a missing/disposed Handle or one owned by another Frame, Page, CDP
   client, or execution world before native I/O or object publication;
5. call the function in the bound CDP session and execution context;
6. commit the runtime Create and the native Handle route together;
7. compensate with native release if publication cannot commit.

`ChromiumHandleDisposeCommandService` performs the inverse transaction. It
releases the exact native object when one exists, commits runtime disposal, and
removes the route. An already-stale portable Handle remains an idempotent
success. The negotiated remote composition installs ordered Create and Dispose
decorators only when both Handle capabilities were accepted. A successful
creation cannot reach its typed Result before Create crosses the shared control
stream, and successful disposal cannot complete before Dispose does the same.

The route table now has an optional production profile that must be present in
the declared connection plan before construction. Each live route owns one
`RemoteHandle` unit and exact retained native-argument/preview bytes in the
shared ledger. Local metrics report current/peak Handles and bytes, total creations,
explicit disposals, parent-driven retirements, topology invalidations, and
connection-close retirements. Capacity and omitted-plan failures occur before
browser evaluation.

Explicit Page and owned BrowserContext close now prepare retirement for every
Handle in the runtime's child-first disposal order. After native destruction
makes those remote objects unreachable, runtime disposal and route retirement
commit together without issuing redundant per-Handle CDP releases.

Each committed Handle route also records its portable owner Frame and Page.
The ordinary topology transaction uses those owners to add Handle Dispose
messages when a main execution world changes or disappears, a Page crashes, or
reconnect begins. Normal Frame/Page/Context topology disposal is expanded by
the runtime object graph and folded into the same prepared route retirement.
`ChromiumHandleAwareRuntime` is the opt-in remote-control target that couples a
Pwright transport close to complete Handle-route and lease retirement. These
implicit invalidations do not issue `Runtime.releaseObject`: the renderer,
target, or CDP connection has already made the native identities unusable.

Direct Handle creation also has a caller-owned bounded continuation decorator.
It reserves a cleanup slot before CDP interaction. If a started public future
is dropped, the continuation finishes evaluation and, when that creates a
Handle, immediately invokes the same explicit disposal transaction. The user
never receives that ObjectId, so retaining the result would be a leak. Missing
continuation capacity rejects before CDP. Combining this owner with the remote
Create-before-Result and Dispose-before-completion fences is now part of the
negotiated handler factory through the reusable ordered-delivery plugin. The
plugin retains the original pre-admitted cleanup reservation after service
completion. If the handler is dropped while its Result send is pending, that
reservation performs the same ordered disposal, closing the final
Create-before-Result abandonment window without allocating emergency cleanup
capacity.

Handle lifecycle and evaluation are separate optional installation profiles.
Selecting evaluation requires lifecycle, remote selection requires ordered
control, and neither profile installs value evaluation. Lifecycle owns the
shared native-route budget; evaluation owns a separate CDP evaluation-route
budget. Setup rejects mismatched native-ID/preview bounds, duplicate resource
scopes, insufficient live-object capacity for topology plus Handles, or a
missing production resource claim before a browser request can start.

## Verified behavior

Release-profile tests prove:

- schema generation proves the additive 1.1 Handle variant against the frozen
  1.0 baseline;
- codec round trips preserve initializer identity, kind, preview, and errors;
- mixed nested Handle/value arguments round trip with portable IDs, while a
  negotiated 1.0 codec rejects Handle leaves;
- the client cannot materialize a remote Handle without the exact prior Create;
- a prior Create produces the correct concrete JSHandle or ElementHandle;
- the Chromium transaction publishes a runtime child and native route, then
  disposal releases the same CDP object and removes both records;
- production Handle units and exact retained bytes return to zero on explicit
  disposal or parent retirement while their peaks remain observable;
- a third Handle at a two-route limit is rejected before another CDP call;
- same-world routes resolve to their exact native argument; cross-Frame and
  already-retired arguments fail before a CDP request;
- Frame routes retain numeric and stable execution-context identity, while
  each persistent Handle payload records its source world; same-session world
  replacement rejects an old Handle before a CDP request;
- Page and BrowserContext destruction retire their descendant routes without
  redundant `Runtime.releaseObject` calls;
- main-world replacement, Frame detach, Page crash, and reconnect retire the
  exact affected Handle routes in the topology/runtime transaction;
- the Handle-aware transport-close wrapper drains every remaining route and
  resource lease with the runtime object graph;
- a dropped started direct evaluation transfers to bounded continuation,
  releases its exact native object, and leaves no runtime object or route;
- exhausted continuation capacity rejects before any CDP request;
- Handle lifecycle and evaluation negotiate and install without value
  evaluation, while omitted Handle capabilities create no codec, handler,
  backend route, or native-route table;
- a failed remote Create fence compensates the exact native Handle;
- dropping a remote call after Create was delivered but while Result delivery
  is pending performs one ordered Dispose and leaves no live Handle;
- malformed, oversized, or mismatched native results are rejected without
  leaking native identity into the wire;
- the Chromium backend advertises Handle evaluation separately from value
  evaluation and Handle lifecycle;
- the independent 1.2 leaf plus ordered-delivery plugin reaches Chrome 152,
  sends Create before the typed Result and Dispose before completion, evaluates
  through the returned Handle, and leaves external browser ownership intact;
- the low-level Chromium adoption primitive maps a main-world ElementHandle
  through `DOM.describeNode` and `DOM.resolveNode` into a real isolated world,
  evaluates through it, releases the temporary wrapper, and leaves external
  browser ownership intact;
- the opt-in production adoption batch reserves every temporary object's
  `RemoteHandle` and bounded backend bytes before DOM I/O, while a caller-owned
  continuation permit spans adoption through exhaustive release; eight AAA
  tests and a second Chrome 152 proof return every lease and reservation to
  zero;
- a dedicated Chrome 152 run created a real DOM ElementHandle, passed that
  Handle into a second `evaluateHandle`, received the primitive `SECTION`,
  released the native object exactly once, and left the externally owned Chrome
  process alive after test cleanup;
- a second dedicated Chrome 152 run negotiated Handle evaluation 1.1 through
  the complete client/server recipe, applied Browser/Context/Page/Frame Create
  announcements in the typed client, returned a real ElementHandle, passed its
  portable ID through a second encoded command, completed graceful close, and
  left external Chrome reachable;
- that duplex proof also confirms that a Frame Create may precede its renderer
  execution-context route. Handle creation waits for world discovery in this
  setup; a Handle retained on the temporary fallback route is correctly
  invalidated when the exact world replaces it;
- cancellation metadata survives the Handle codec round trip, closing the
  previous incompatibility between the cancellation-aware client channel and
  this optional command.

## Aggregate recipe adoption

The default installation remains exact-world. The separately selected
`install_with_element_adoption` path now adds the exact production budget,
exhaustive batch owner, cancellation-safe continuation owner, and public
command transaction. It deduplicates and classifies arguments before DOM I/O,
rejects JSHandles and cross-session handles, substitutes temporary native
arguments, and releases them across success, backend failure, cancellation,
publication failure, and topology revalidation. Fake command tests prove
same-Page child-Frame success plus JSHandle and OOPIF-shaped two-session
rejection; dedicated Chrome 152 proves real same-process child-Frame success.
Another Chrome 152 proof creates a real auto-attached OOPIF ElementHandle and
confirms cross-session rejection without temporary-resource admission. Generic
JSHandle arguments remain exact-execution-context only. The complete aggregate
recipe now installs the independent lifecycle-leaf registry and uses the leaf
service for all six Handle 1.2 commands. It reuses `CdpPageServices` as the live
Frame/world resolver and commits leaf route retirement with Page/Context close,
topology invalidation, and transport close. `with_handle_element_adoption`
selects adoption explicitly and adds its separate temporary-object resource
peak; omission retains exact-world behavior and installs no adoption backend.
