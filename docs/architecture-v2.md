<!-- agent-updated: 2026-09-05T22:05:06Z -->

# Composable architecture v2 (discussion draft)

> Status: design discussion with exploratory foundation slices authorized on
> 2026-09-01. The slices validate core mechanics only; broader crate
> names, public browser contracts, protocol, backends, and migration strategy
> remain open for review.

## Authorized exploratory slice

The first implementation slice is deliberately narrower than a browser
vertical slice. It may add a new `pwright-core` workspace crate containing:

- dependency-free `Service<Request>`, `Layer<Inner>`, and `Stack` composition,
  plus a target-aware runtime service boundary that preserves validated
  `ObjectId` routing for browser backends;
- stable capability, command, plugin, and protocol-version identifiers;
- a metadata registry that rejects duplicate capabilities, duplicate command
  ownership, missing dependencies, incompatible dependency versions, and
  dependency cycles;
- deterministic unit tests following Arrange-Act-Assert.

The slice must have no normal dependencies, must not modify legacy v1 crate
behavior, must not introduce a runtime, transport, codec, browser backend, or
browser download, and must not be treated as approval of dynamic loading or the
future public browser API. Its acceptance evidence is a clean release-profile
test run and a dependency-tree check showing only `pwright-core` itself.

Until the design gates in `design-tasks-v2.md` are complete, implementation is
exploratory and may be revised or discarded. This authorization overrides the
older blanket statement that no implementation work may begin, but only for
the bounded slice above.

### Protocol and lifecycle foundation slice

The second exploratory slice may extend `pwright-core` with protocol model and
runtime object-ownership primitives, under these decisions:

- `CallId`, `ObjectId`, `CancellationId`, method tags, event tags, and object-kind
  tags are separate numeric newtypes; ID allocation remains injected and is not
  performed through global randomness or process state.
- A typed command is parameterized by a `CommandSpec` that fixes its parameter,
  result, and structured-error-detail types. Events follow the same pattern.
- Call metadata carries a relative timeout budget and optional cancellation ID.
  It does not carry a local monotonic timestamp across machines.
- Codec-independent control messages cover create, adopt, dispose, cancel, and
  subscription changes. No JSON, Serde, WebSocket, or executor enters core.
- Operation failures have distinct kinds for timeout, cancellation, target
  closure, browser crash, unsupported capability, resource exhaustion,
  transport closure, protocol violation, backend failure, and cleanup failure.
- The runtime object graph has a hard live-object limit. A child requires a live
  parent, adoption cannot create a cycle, parent disposal returns descendants in
  child-first order, and disposing an absent object is an idempotent no-op.

This slice specifies logical envelopes and synchronous ownership transitions;
it does not approve a wire codec, transport framing, reconnect semantics, async
cleanup executor, public browser API, or CDP adapter. A later runtime must run
the returned child-first cleanup plan and only then report close completion.

### Resource-boundary foundation slice

The same foundation phase may add two standard-library-only bounded containers:

- a pending-call table with an explicit hard limit, duplicate-ID rejection,
  cancellation-token validation, first-cancellation-reason retention, explicit
  completion removal, and deterministic connection-close draining;
- an event queue with a fixed capacity and caller-selected `RejectNewest` or
  `DropOldest` overflow policy, including accepted, rejected, dropped, current,
  and peak counters.

Neither container starts tasks, reads a clock, chooses an executor, or decides
which event class gets which overflow policy. The runtime must inject IDs and
time, classify event types, and translate overflow into protocol diagnostics.
Zero capacity is valid and rejects every event explicitly. No unbounded queue
or implicit spawn fallback is permitted.

The DT-060 shared-budget checkpoint adds a connection-local `ResourceLedger`
to `pwright-core`. It atomically enforces global and per-scope units across
objects, calls, events, wire data, bodies, logs, traces, screenshots, selectors,
handles, and adapter state. Its non-cloneable RAII leases release on normal
completion, cancellation, unwinding, or dropped futures, and its independent
scope/lease caps bound the accounting metadata itself. `BudgetedBytes` couples
immutable retained bytes to a byte-class lease.

`WireFrame::new_budgeted` is the first concrete consumer. A
`WireProtocol::with_resource_ledger` composition charges encoded payload plus
attachments to the route's connection, call, or object scope. WireFrame clones
share immutable storage and one lease instead of copying buffers. Per-frame
`FrameLimits` still reject one oversized frame; the ledger additionally rejects
multiple individually valid frames whose combined retention exceeds the
connection budget. Production encoding reserves twice the maximum frame size
before invoking the codec, covering the bounded typed source and newly encoded
buffers at the same time. It atomically shrinks that staging lease to the actual
frame bytes and moves it to the final route scope; codec or validation failure
releases the full reservation. The Tokio stream adapter can use the same injected ledger
for inbound data: it reserves a declared envelope before body allocation, the
stream codec reserves the decoded frame before copying payload bytes, and both
leases deliberately overlap while both allocations exist. Cancellation,
malformed input, and final frame drop release the exact owner through RAII. The
outbound adapter likewise reserves its encoded envelope and length-prefixed
pending-write buffer before allocation. It releases the temporary envelope
after copying while the pending buffer retains its lease across a cancelled
partial send until a subsequent send or shutdown finishes it. The
typed client event handler can also opt into the connection ledger with a
capability-supplied decoded-size function. Each queued event atomically owns an
`EventItem` and its nonempty `EventBytes`; reliable overflow reserves nothing,
while lossy overflow atomically moves the old lease to the replacement without
losing the old event when admission fails. Dequeue, clear, or handler teardown
releases the lease. The
full inventory and eviction rules are recorded in
[Pwright v2 resource ownership and limits](resource-boundaries-v2.md).

Production composition does not receive an implicit permissive ledger.
`ResourceRequirements` is the zero-dependency declaration made by each
component, and merged declarations form the minimum production plan.
`ProductionResourceLedger` validates that plan against the configured hard
limits and lets an installed component claim only requirements present in the
plan. Both configured-limit failures and omitted-plan failures are typed setup
errors. Claims share one ledger rather than partitioning or duplicating
capacity, preserving connection-wide contention and metrics.

The direct and controlled runtime dispatchers can install the same ledger only
before retaining state. Create reserves an object-scoped `LiveObject` after all
lifecycle validation but before mutation; Adopt keeps that identity; Dispose
and transport close release the child-first subtree. Call admission similarly
reserves a call-scoped `PendingCall` before table or scheduler mutation. Its
lease remains through queueing, execution, cancellation, and abandonment until
the terminal guard completes or drops. Parent close therefore releases object
leases immediately but correctly retains cancelled-call leases through backend
cleanup.

An enabled lifecycle outbox also reserves one object- or call-scoped
`EventItem` per retained control message. Create and Adopt reserve before graph
mutation; Dispose reserves its complete child-first message batch before
disposing anything. A peek claim keeps ownership through rollback, while pop,
claim commit, queue clear, and transport close return the exact lease.
`RuntimeLimits::resource_requirements` derives these runtime minima, including
only the realizable active-command count. The dispatchers'
`with_production_resources` constructors must successfully claim them before a
production runtime exists. Raw-ledger and unbudgeted constructors remain
explicit low-level/test/v1 compatibility paths, not production defaults.

An explicit backend close cannot use that one-step Dispose path: the native
operation is asynchronous and may fail. The controlled dispatcher therefore
offers a two-phase `PreparedObjectDisposal`. Prepare runs all graph and outbox
capacity checks, reserves the complete child-first `EventItem` batch, records a
non-reusable transaction token, and deliberately leaves the graph live. While
the caller awaits backend cleanup, every lifecycle mutator is rejected, but
ordinary commands may still be admitted; commit cancels those calls as
`ParentClosed`, removes the unchanged subtree, and transfers the reserved
messages to the outbox. Dropping the value rolls back without changing
liveness. Transport close clears the token and remains terminal, so a later
commit is rejected rather than resurrecting or double-disposing state. This
runtime transaction does not itself freeze a backend topology map; the
Chromium composition root must coordinate both owners around the CDP close.

That coordinator now exists for the first BrowserContext lifecycle slice.
`pwright-cap-context-lifecycle` is a core-only 1.0 contract;
`pwright-codec-context-lifecycle`, `pwright-client-context-lifecycle`, and
`pwright-server-context-lifecycle` are separate optional layers. Its only
command is `browser-context.close` (`0x0003_0001`) and its generated target is
`pwright.browser.browser_context`. The binary codec duplicates and verifies
outer call/target identities, accepts no attachments, bounds every retained
error string, and enforces empty timeout/cancellation metadata. This makes the
Playwright no-timeout close semantics structural rather than a client default.

`CdpBrowserContextRoute` exposes a borrowed, bounded native ID and explicit
Default/Owned/External ownership from topology. The native service substitutes
nothing: Owned sends `Target.disposeBrowserContext`; Default and External
return `UnsupportedCapability` before I/O. An owned
`PreparedCdpBrowserContextClose` reserves a complete topology shadow, holds a
non-reusable transaction gate across the caller-owned future, and rolls back on
drop. `ChromiumBrowserContextCloseCommandService` combines that candidate with
a cloneable controlled-runtime lifecycle handle. Backend failure drops both;
success publishes topology, then the reserved child-first runtime batch, then
the typed result. No Node driver, process launch, browser download, timer, or
hidden executor is introduced.

The same transaction is now available without the aggregate composition as
`pwright-composition-chromium-context-lifecycle`. It installs only the exact
1.0 Context-lifecycle codec and generated handler around caller-owned CDP,
topology, runtime lifecycle, and bounded continuation inputs. One object-safe
retirement plugin receives the staged post-close topology and child-first
disposal order before native I/O; it can pre-admit Page-route, Handle, network,
or composite cleanup and returns an infallible commit token. Consequently none
of those optional owners becomes a dependency of the lifecycle leaf.

Page lifecycle now has the same independent composition boundary in
`pwright-composition-chromium-page-lifecycle`. Ordinary `Page.close` validates
the portable Page, stages topology and child-first runtime retirement, and
prepares one injected route-retirement plugin before issuing
`Target.closeTarget`. It publishes only after bounded `Target.getTargets`
polling proves the exact target disappeared. The polling policy and abandoned
close continuation are caller-owned; the leaf starts no timer or executor.
`Page.run_before_unload` instead resolves the exact target-local session,
sends `Page.close`, and deliberately preserves topology/runtime liveness
because a dialog may keep the Page open.

`pwright-composition-chromium-page-creation` pairs that entire lifecycle leaf
with `BrowserContext.newPage`. It allocates Page and main-Frame identities as
one runtime batch, reserves an optional publication plugin before CDP, creates
and attaches `about:blank` in the exact Context, then binds the returned
target/session/frame route provisionally. Only after topology and runtime
publication succeed does the plugin commit. Every failure after target
creation compensates the exact returned target through the lifecycle backend;
creation and close have separate caller-owned continuation queues.

The remote close promise now uses one `OrderedRuntimeControl` for both the
ordinary lifecycle lane and command-completion barriers. Its hard writer limit
is derived from every supervisor-retained command plus the singleton outbound
lane. A close obtains barrier ownership before native work, queued barriers
take priority over ordinary writers, and success drains every earlier control
through the target BrowserContext's own child-last Dispose. Only then does the
generated typed handler send Result. The transport therefore contains
Frame→Page→BrowserContext Dispose before Result; the client's unique reader
applies each control synchronously before it can correlate the later Result.
The close future cannot wake while its context handle remains live. Selecting
remote context lifecycle without negotiated remote control is rejected during
composition. No acknowledgement message, Node driver, hidden task, or second
reader is needed.

`AttachedChromiumComposition` now owns that same topology as the first-class
`Arc<Mutex<ProductionCdpTopology>>` used by close and event updates. Its close
factory binds the exact attached CDP browser session, shared topology,
controlled-runtime lifecycle handle, and owner-supplied reserved cleanup
executor; callers no longer reconstruct that tuple.
Only one production runtime may be installed from an attachment, preventing a
second owner from claiming a parallel object graph. Topology guards are
synchronous and must be dropped before awaiting browser work. The dedicated
Chrome proof now exercises this attach/runtime/factory path rather than a
hand-assembled equivalent.

`ContinuableBrowserContextCloseCommandService` closes the caller-abandonment
gap without selecting Tokio, spawning a task, or hiding an unbounded queue. It
reserves one continuation slot synchronously before browser interaction. If no
slot is available, close fails as resource exhaustion before CDP or either
local transaction begins. Dropping an unpolled future releases the reservation
and preserves service laziness. Dropping a future after its first pending poll
atomically transfers that exact remaining state machine—including its topology
and runtime preparations—to the reserved, caller-owned cleanup queue. The
queue's completion/failure metrics make the eventual outcome observable. AAA
tests prove all three cases, while the live attached proof uses this same
factory and continuation path against the dedicated Chrome process.

The first BrowserContext creation slice is the inverse lifecycle transaction,
not a shortcut around it. `pwright-cap-context-creation` declares the optional
Browser-only `browser.new_context` command and a hard dependency on context
lifecycle; independent codec, client-extension, and generated-server crates
mean an embedded user can omit the entire feature. The initial 1.0 surface has
no options. Chromium alone adds `Target.createBrowserContext` with
`disposeOnDetach: true`, and never exposes its native string as a portable ID.

A connection-local `AtomicObjectIdSource` allocates nonzero, monotonic IDs and
never wraps or reuses a failed attempt. `PreparedObjectCreation` reserves the
runtime object lease, lifecycle outbox entry, graph insertion, and exclusive
transaction before browser work while leaving the object absent until commit.
After CDP succeeds, composition prepares the production topology candidate,
verifies its exact BrowserContext Create effect, commits runtime and topology
under the topology publication lock, and compensates every publication failure
with `Target.disposeBrowserContext`. A synchronously reserved continuation
owns an already-started operation if the caller drops its future.

Remote creation uses the same priority control writer with an exact creation
fence: the physical Create frame must be accepted before the generated handler
may send Result. Client state distinguishes announced, claimed, and
disposed-before-claim identities. Remote Results can claim only an exact prior
announcement and cannot revive a context already disposed; direct in-process
clients may claim directly. A dedicated Chrome 152 proof creates a real
context, uses it for a target, closes it through the portable lifecycle path,
and verifies that the externally owned browser survives. This slice adds no
Node.js process, browser launcher, downloader, timer, or hidden executor.

The same creation transaction is now independently selectable as
`pwright-composition-chromium-context-creation`. Installation is deliberately
paired: its generated `Browser.newContext` handler and the required
`BrowserContext.close` handler share one caller-owned CDP session, topology,
runtime lifecycle handle, and browser identity. The caller separately supplies
creation and close continuation capacity. A publication plugin pre-admits all
fallible optional state before CDP and returns a rollback-on-drop token whose
final commit is infallible. The order is therefore ObjectId allocation → plugin
admission → runtime preparation → native creation → topology preparation →
runtime/topology/plugin commit. Any failure after native creation disposes the
exact returned native Context through the lifecycle backend. The leaf's exact
normal Cargo closure is 22 Pwright packages and contains neither aggregate
backend/composition, client, provider, launcher, Tokio, Node.js, nor browser
acquisition.

Explicit Page creation is now the first multi-object inverse lifecycle
transaction. The optional `pwright.page_creation` 1.0 capability depends only
on Page lifecycle and exposes BrowserContext-only `new_page`; separate codec,
client-extension, generated-server, CDP-backend, and composition pieces remain
absent when not installed. Its portable success initializer contains a Page
and required main Frame, never CDP target/session/frame strings.

Chromium resolves the exact Context route, reserves two monotonic ObjectIds
and one `PreparedObjectCreationBatch`, creates `about:blank`, flatten-attaches
the target, and requires `Page.getFrameTree` to return a bounded parentless
root. Publication prepares topology attachment plus main-frame rebinding and a
Page-route replacement before atomically committing Page→Frame runtime
ownership. Failure after native creation closes the exact target and uses the
bounded Page-close absence proof. A dropped started future transfers unchanged
to caller-owned reserved continuation capacity.

Remote completion physically writes Page Create, main-Frame Create, then
Result. Client state preflights and claims the pair as one transaction, so a
missing/mismatched second announcement or insufficient capacity cannot expose
a half-created Page. The dedicated Chrome 152 proof creates Context=2,
Page=3, Frame=4 through the portable services, verifies the live
`about:blank` target and route, closes Page and Context, and confirms the
externally owned browser survives. Popup discovery remains a later event
capability with separate opener and race semantics.

The implemented value-only JavaScript evaluation slice keeps optional
`pwright.evaluation` as one Frame-targeted command; a Page API delegates only
to its initialized main Frame. The portable value model preserves special
numbers, BigInt, Date, URL, RegExp, Error, typed arrays, object order, and cyclic
references within explicit depth/node/property/string/binary limits. Chromium
routes each observed Frame main world to the exact Page or OOPIF session using
CDP's globally unique context identity. Asking for a child Frame before that
world exists must fail rather than running in the wrong document. World
replacement/removal and route publication share the prepared topology/runtime
transaction, so a stale numeric context ID cannot redirect work after
navigation. Production admission reserves a local concurrency slot and
the conservative transient backend peak before returning a future. The Rust
decoder rechecks all graph limits before cloning page-controlled strings or
allocating decoded child collections.

This capability returns no `objectId` and creates no runtime object. A returned
CDP object identity is a protocol violation. `JSHandle` and `ElementHandle`
belong to a later `pwright.evaluation_handles` capability with explicit
creation/disposal, frame-navigation invalidation, and live-handle budgets. The
detailed design and gates are recorded in
`knowledge/pwright-value-evaluation.md`.

Fake-CDP composition tests prove exact main/local/OOPIF session and unique-world
selection and reject unknown-Frame fallback. The encoded remote path produces one bounded Result with
no object lifecycle controls. A dedicated Chrome 152 run proves Promise
waiting, one explicit `undefined` argument when omitted, cyclic and special
typed values, structured JavaScript errors, browser-side limit failure, and
post-navigation execution while leaving the external browser alive.
An additional Chrome 152 production-composition proof uses a `serve-lib`
cross-site fixture to create a real OOPIF, then evaluates through its exact
iframe session and globally unique main-world context.

The next Chromium integration checkpoint removes manual target and child-Frame
topology assembly. `ChromiumTopologyEventDriverProductionProfile` is optional
infrastructure, not a portable capability. Its attached driver owns one fresh
bounded selected CDP event subscription but starts no task; the composition owner
calls `receive_once(runtime)`. Before and after each receive it reconciles the
bounded Page/OOPIF session set, reads the complete initial frame tree, and
recursively enables Page, Runtime, and flatten auto-attach domains. A separate
`ChromiumPageDiscoveryProductionProfile` selects browser-root auto-attach;
without it the same driver maintains only explicitly admitted targets. Selected
discovery uses the Playwright-style `Target.getTargetInfo` barrier, bounded
non-default-context enumeration, a unique default Context, paused Page setup,
and explicit resume. A discovered Page, its complete frame tree, and Ready
state enter topology/runtime/routes in one transaction only after CDP setup
succeeds. Generated Frame, Page/popup target, execution-context,
detach, and crash events feed the existing prepared topology/runtime/route
transaction.
Event loss is sticky: ordinary receive cannot advance again until the caller
requests a strictly newer reconstruction epoch. On a live transport,
reconstruction toggles browser-root auto-attach around a fresh subscription,
uses the Chromium barrier as an authoritative snapshot, reuses ObjectIds for
targets seen again with new sessions, and disposes missing Contexts, Pages, and
Frames before returning `ReconnectFinished`.
Local-to-OOPIF swaps preserve portable identity, late old-world destruction
cannot erase a replacement route, and OOPIF target detach waits for Page detach
synchronization before disposal. Broadcast lag returns a sticky mandatory
resync error; selected discovery can perform a caller-driven same-connection
reconstruction, while a reconnect epoch invalidates an older driver before it
consumes another event. All initialization and event effects are returned as one ordered
update, so `PageReady` and `PopupLinked` are observable. Its fixed backend-state
lease is derived from exact connection and topology limits, including maximum
frame-tree response bytes. The detailed boundary and remaining reconnect/event
delivery work are recorded in
`knowledge/pwright-chromium-topology-events.md`.

Native CDP disconnect is terminal for the attached Browser object graph. This
matches Playwright rather than attempting to preserve native sessions across a
replacement WebSocket: `CRConnection._onClose` disposes its root session,
`CRBrowser._didDisconnect` closes every Page and worker, the Browser base closes
Contexts and emits `disconnected`, and `BrowserDispatcher` sends close then
disposes its channel object. Pwright exposes `CdpConnection::closed()` as an
owned, task-free signal. `ChromiumCdpLivenessBackend` turns that signal into a
terminal backend error even when topology-events are not installed. The generic
`ConcurrentServerBackendChain` fairly composes liveness with topology or future
backend plugins and forwards supervisor termination to every child. A new CDP
attachment therefore creates a new Browser graph; it never rebinds old Page or
Context handles. `AttachedChromiumComposition::server_backend` is the canonical
constructor and always places mandatory liveness before the caller's optional
plugin. Same-live-CDP event reconstruction and creation of a fresh remote
Pwright client event view remain separate operations.

`ChromiumAttachFactory` is the reusable boundary above that one-shot
attachment. It retains only the validated declarative production profile,
per-connection limits, and Browser root ID. Every `prepare` allocates a new
`ProductionResourceLedger` and production connector; every `attach` then opens
a new CDP connection and constructs a new topology, monotonic ObjectId source,
and epoch-1 history. Equal numeric ObjectIds belong to independent wire
namespaces and do not make the graphs interchangeable. Factory construction
performs all profile, resource-plan, and root-ID exhaustion checks without I/O.
`attach_remote` is the next staged boundary. It requires the remote-handshake
plugin before CDP I/O, lends the fresh ledger to a caller-supplied one-shot
transport builder, and accepts only a `ProductionFrameTransport` proving the
same ledger identity and exact declared transport requirements. It then owns
the handshake-first supervisor and claims at most one topology-event driver for
that attachment. `ChromiumRemoteConnectionSeed::install_runtime` consumes a
caller-supplied runtime-installer plugin and refuses to enter the final
`ChromiumRemoteConnection` state unless the installer actually used one of the
attachment's one-shot runtime constructors. The final value therefore owns one
fresh CDP attachment, one remote transport/supervisor, one runtime graph, and
one optional topology-event baseline. It starts no task, listener, executor, or
handshake. Post-handshake typed event publishers and the complete capability
handler/backend stack remain explicit assembly work.
Chrome 152 production attachments verify the runtime-owning transition, the
one-driver rule, and rejection of a runtime installer that returns without
creating the graph against the dedicated external browser process.

The next lifecycle slice implements Playwright's deliberately split Page close
contract without coupling it to navigation or locators. The generated
`pwright.page_lifecycle` 1.0 capability owns Page-only `page.close`
(`0x0003_0002`) and `page.run_before_unload` (`0x0003_0003`). Separate
capability, binary-codec, client-extension, and generated-server crates keep
the slice absent unless selected. Ordinary client close treats local staleness
or a racing `TargetClosed` result as success; before-unload preserves that
failure because initiation itself is the requested effect. Neither command
accepts timeout/cancellation metadata or attachments.

Chromium ordinary close sends browser-session `Target.closeTarget` and then
uses a bounded caller-injected polling policy to prove the exact target is no
longer returned by `Target.getTargets`. Its composed transaction prepares the
Page topology removal and runtime subtree disposal before CDP. After native
termination it prepares the Page-route replacement from the topology
candidate, publishes topology and routes in one synchronous lock window, then
commits Frame→Page runtime disposal. A failed CDP call or exhausted confirmation
budget drops both prepared owners and leaves Page identity live. The
before-unload service instead sends page-session `Page.close` and deliberately
keeps topology, routes, and runtime objects live for a later dialog/target
event.

Remote ordinary Page close reuses the bounded priority writer and emits
Frame Dispose, Page Dispose, then Result. A real encoded-client proof keeps the
close future pending through both invalidations and wakes it only on Result.
The attached Page-runtime factory binds the exact connection, topology, route
table, lifecycle handle, polling policy, and caller-owned continuation. A
dedicated Chrome 152 proof closes only the selected target while its context
and external browser survive. The context-close target-aware factory now also
retires every descendant Page route, so topology, route, and runtime ownership
cannot silently diverge on a successful close.

### First typed backend slice

The first backend-connected slice is split into five relevant crates so its
dependency direction is executable evidence rather than a diagram:

```text
pwright-cap-navigation -> pwright-core
pwright-runtime        -> pwright-core
pwright-browser-backend
                       -> pwright-core
pwright-backend-cdp    -> pwright-browser-backend + pwright-browser-model
                          + pwright-core + capability crates + pwright-cdp
pwright-codec-navigation
                       -> pwright-core + pwright-cap-navigation + pwright-transport
```

`pwright-cap-navigation` began with a typed, portable `BeginNavigation` backend
primitive. It now retains that primitive and adds a distinct `Goto` contract;
the later goto slice below documents exactly which Playwright semantics are
implemented and which remain open.

`pwright-runtime` provides typed in-process dispatch. It accepts a
`Command<Spec>`, validates that the target object is live, reserves a bounded
pending-call entry, invokes the matching typed `Service<Params>` without a
codec, correlates the result, and removes the pending entry even if the dispatch
future is dropped. It does not spawn a task or select an executor.

`pwright-browser-backend` defines backend identity, engine/protocol identity,
and validated command-support manifests without selecting a concrete browser
protocol. A feature declaration references the portable capability owner,
minimum protocol version, command ID, and whether the adapter implementation is
native or emulated. Manifest construction cross-checks those declarations with
the installed capability registry. `BackendCheckedService<Command, Inner>`
then binds one typed `CapabilityCommandSpec` to one feature at setup; missing
support, a wrong command, or a wrong capability fails before the inner adapter
can receive browser work.

`pwright-backend-cdp` is the only new slice allowed to depend on legacy
`pwright-cdp`. It adapts the narrow navigation service to `Page.navigate`, maps
CDP failures into portable failure categories, and preserves Playwright's rule
that HTTP error status pages are navigations rather than transport failures.
Its `ChromiumCdpBackend` descriptor currently declares four implemented
portable commands. Direct begin-navigation is native; Playwright-shaped goto,
locator click, and locator fill are explicitly classified as emulated from
multiple CDP/JavaScript operations. Native access is available only through
`NativeBackendAccess<ChromiumCdpBackend>` implemented on adapter-owned service
values. Portable capability values and client handles do not implement that
trait and never name `CdpClient`, target IDs, session IDs, or CDP messages.
The capability and runtime crates must not depend on CDP, Tokio, Serde, or a
browser binary. They never launch, download, bundle, or terminate a browser;
those effects belong only to explicitly selected provider crates.

### Chromium object-mapping and recovery state machine

`CdpTopology` is an adapter-owned, I/O-free state machine inside
`pwright-backend-cdp`. It is the only layer in this slice that knows CDP browser
context IDs, target IDs, session IDs, frame IDs, or execution-context IDs. The
composition root supplies globally unique `ObjectId` reservations; the mapping
does not hide a second allocator. Successful changes return portable
`ControlMessage::Create` or `ControlMessage::Dispose` effects plus narrow
adapter events. Those lifecycle effects have been verified against the
runtime's `Complete` Browser→BrowserContext→Page→Frame topology.

The current Chromium mapping rules are explicit:

- the default BrowserContext is unique and uses no invented CDP identity.
  Chrome may still report an opaque context ID on its Page targets; discovery
  distinguishes it from the bounded `Target.getBrowserContexts` non-default
  set before mapping it to the default object;
- a page target belongs to its BrowserContext; an opener is an event
  relationship and never becomes the page's ownership parent;
- a target arriving before its context is retained in a bounded pending table,
  while a detach before context admission releases its session and reserved
  object IDs without creating a phantom page;
- `PageReady` and `PopupLinked` are emitted only after initialization, and a
  popup link waits until both popup and opener are ready;
- a Page owns one stable main-frame `ObjectId`. A changed Chromium root frame
  ID during cross-process navigation rebinds that object and disposes only its
  previous descendants;
- local iframe events and OOPIF target sessions converge on the same stable
  Frame object. If `detachedFromTarget` races a remote-to-local swap, the map
  requests one browser-session synchronization round trip; confirmation
  disposes the frame only when no local attachment arrived;
- execution worlds are keyed by `(session ID, execution-context ID)`. Only the
  frame's main and Pwright utility worlds are retained; unrelated isolated
  worlds are ignored, and navigation replaces the prior world in place;
- a crash marks the exact Page as crashed but does not manufacture a detach.
  An exact session detach or context close owns subtree disposal;
- reconnect increments a connection epoch, clears session/world bindings, and
  preserves public IDs for native targets seen again. Events from an older
  epoch are ignored, and reconciliation disposes targets absent from the new
  connection.

Context, page, frame, pending, execution-world, total-object, and native-ID
byte limits are mandatory constructor inputs. Every capacity or malformed
generated-CDP-event failure occurs before the corresponding topology mutation.
The state machine starts no task, timer, transport, browser process, or event
pump; those remain independently composable runtime/connection concerns.

Utility worlds are a separate explicit infrastructure selection,
`ChromiumUtilityWorldProfile`; minimal discovery/evaluation compositions do not
pay for or create them. The Playwright-compatible profile derives a unique
bounded name from each connection-local portable Page ID, registers an empty
`Page.addScriptToEvaluateOnNewDocument` in that named world for future
documents, and explicitly creates the world in every Frame returned by the
initial Page or OOPIF frame tree before resuming a paused target. Creation is
sequential and caller-driven. A failed creation is tolerated only when a fresh
authoritative frame tree proves that exact Frame disappeared; if it remains,
the original CDP error terminates initialization. The returned execution
context ID is not trusted as routing state: the corresponding bounded Runtime
event establishes the session-scoped unique-context route.

Production composition wraps that pure state machine in
`ProductionCdpTopology`. `CdpTopologyProductionProfile` derives the worst-case
native-string bytes across duplicate indexes, records, sessions, crash state,
and worlds. The composition root assigns its adapter extension scope; other
scope kinds are rejected. It declares two copies because each mutation pre-reserves a bounded
shadow before cloning; the shadow transition is committed only after success,
then its lease is shrunk to the exact retained `String::capacity()` total and
the previous state/lease is released. Inputs are compacted before insertion,
so oversized caller allocation capacity is not retained behind a short native
ID. Resource exhaustion therefore cannot partially attach, materialize,
detach, crash, or reconnect topology state. The raw `CdpTopology` constructor
remains the explicit pure/test/low-level path, not the production default.

The Chromium locator service retains no growing cache or resolved DOM node.
Its production profile therefore charges active operation memory to
`SelectorBytes`, not `BackendStateBytes`. The composition root chooses the
scope and maximum concurrent actions. Before returning an action future, the
service validates selector/value allocation capacity, acquires a clone-shared
local slot, and reserves a conservative bound for inputs, generated scripts,
runtime-install source, and formatting overlap. An unpolled or cancelled future
owns and then releases that same permit; exhaustion occurs before any CDP call.
The local slot remains necessary even when the shared ledger is larger, because
one plugin may not silently borrow another plugin's declared capacity.

When utility worlds are selected, the router withholds each Frame locator route
until that Frame's utility context event has arrived, then binds selector
runtime installation and every actionability retry to the exact target-local
session and `uniqueContextId`. This covers main Frames, local iframes, and
OOPIFs. Page locator calls delegate to the stable main Frame and therefore
share its selector-runtime cell. User `Frame.evaluate` continues to use the
main-world route; the two services cannot accidentally share globals. A world
replacement rebuilds only that Frame's locator service and once-only runtime
cell, without falling back to another document.

Frame locator routes have their own topology-derived production budget. It
reserves target/session/context strings plus target-local client copies and a
second complete set for atomic route-table replacement. The base Page route
therefore returns to four target/session string slots and retains no duplicate
utility context.

Locator protocol 1.3 completes Playwright's current filter shape without
turning Locators into remote DOM objects. `filter_text`, `filter_not_text`,
`filter_has`, `filter_has_not`, and `filter_visible` are immutable selector AST
nodes, cross the bounded recursive codec, and resolve only when an action runs.
The descendant forms preserve each base candidate and evaluate the inner query
relative to it. Client composition rejects inner locators from another Frame.
When a Page has an initialized main Frame, its locator scope is normalized to
that Frame, so Page and main-Frame locators correctly satisfy the same-frame
rule; legacy manually initialized Pages retain their Page route.

Locator protocol 1.4 adds canonical ECMAScript regex branches for text,
labels, filters, and accessible name/description. Locator 1.5 then adds the
remaining user-facing attribute families: placeholder, alt text, title, and
test ID. Attribute strings match the raw DOM attribute value (case-sensitive
equality for `exact`, case-insensitive substring otherwise), while regexes run
against that same raw value. Standard test ID uses `data-testid`; custom
comma-separated attribute names are validated, capped at 16, and carried in
the immutable selector rather than stored in a process-global registry. The
version-4 Chromium selector runtime preserves these semantics across open
shadow roots without retaining nodes between attempts.

Locator protocol 1.6 introduces the first multi-match read, `count`. Unlike
single-target actions, it is non-strict and takes one immediate snapshot: zero
matches returns `0`, many matches return their bounded `u32` count, and neither
case enters the actionability retry loop. The same immutable Locator is
re-resolved against the current document on every call. Chromium retains no
nodes or result set; if navigation replaced the selector runtime, the backend
allows one bounded reinstall and one second evaluation before reporting a
typed protocol failure.

Locator protocol 1.7 adds the strict, immediate `is_visible` state query.
Missing resolves to `false`, one match is measured with the same backend-owned
visibility primitive used by visible filters, and multiple matches remain a
strictness error. `is_hidden` is intentionally a client-only composition that
negates the typed visibility result, so it adds no second command, codec route,
server handler, backend feature, or browser evaluation. Both methods re-resolve
the immutable Locator on each call and never retain an ElementHandle.

Locator protocol 1.8 adds three independently routable strict state getters:
`is_checked`, `is_editable`, and `is_enabled`. Unlike visibility, these wait
for one target to exist and remain connected, but return the current boolean
as soon as that target is available; they do not wait for the desired state to
become true. `is_disabled` is a client-only negation of `is_enabled`. Chromium
selector runtime v6 owns label retargeting plus native and applicable ARIA
state rules, while runtime deadlines bound missing/detached retry loops.

Locator protocol 1.9 adds the separately installable `wait_for` route. Its
state is `attached`, `detached`, `visible`, or `hidden`, with `visible` as the
semantic default. Every attempt re-resolves the strict Locator; a missing
target therefore satisfies `hidden` and `detached` immediately, while
`visible` and `attached` retry under the one runtime-owned deadline. A strict
violation, invalid selector, or malformed browser response remains terminal.
Success returns unit and never creates or retains an ElementHandle.

Locator protocol 1.10 adds the independently installable `text_content` read.
It follows Playwright's strict, retrying selector resolution, then returns the
exact nullable DOM `Node.textContent` string without whitespace normalization.
The portable result is bounded to 1 MiB of UTF-8, malformed or oversized
browser values fail terminally, and neither the client nor Chromium backend
creates an ElementHandle or retains the resolved node.

Locator protocol 1.11 adds the separate `inner_text` read rather than folding
rendered text into `text_content`. It follows Playwright's strict retrying
resolution, accepts only HTML-namespace elements, and returns the browser's
non-null `HTMLElement.innerText`; a non-HTMLElement fails with the typed
`Node is not an HTMLElement` error. The 1 MiB UTF-8 result ceiling, caller
deadline, utility-world isolation, and handle-free lifetime match the existing
content-read boundary while remaining independently installable.

Locator protocol 1.12 adds independently installable `inner_html`. It uses the
same strict retrying resolution but returns non-null `Element.innerHTML`
without applying `inner_text`'s HTMLElement restriction, so SVG elements are
valid targets. Browser-serialized markup is preserved exactly and bounded to
1 MiB of UTF-8 under the existing deadline, utility-world, operation-lease,
and handle-free content-read boundary.

Locator protocol 1.13 adds independently installable `get_attribute`. It
matches Playwright's strict retrying resolution and returns exact nullable
`Element.getAttribute(name)` output, so a missing attribute is `None` and SVG
elements are valid targets. The attribute name is bounded to 64 KiB and a
present result to 1 MiB of UTF-8. It runs in the locator utility world under
the caller deadline, retains no ElementHandle, and gives attribute evaluation
its own observable counter.

Locator protocol 1.14 adds independently installable `input_value`. Like
Playwright, it strictly waits and re-resolves the Locator, then applies the
shared follow-label retargeting rule before accepting only `input`, `textarea`,
or `select`. It returns the control's current non-null value exactly, bounds it
to 1 MiB of UTF-8, runs in the locator utility world under the caller deadline,
and retains no ElementHandle. Selector runtime v6 exports the same retarget
primitive used by state semantics so label behavior does not fork between
commands.

Locator protocol 1.15 adds independently installable `all_text_contents`.
Matching Playwright's `$$eval` implementation, it performs one non-strict
snapshot, returns an empty vector for zero matches, maps each current match to
`textContent || ''`, and preserves selector order without waiting or creating
ElementHandles. The collection is bounded to 4096 values, 1 MiB per UTF-8
value, and 4 MiB of UTF-8 in aggregate. Chromium evaluates it in the same
utility world, permits one runtime reinstall after navigation, and records an
independent collection-evaluation metric.

Locator protocol 1.16 adds independently installable `all_inner_texts`.
Matching Playwright's `$$eval` implementation, it is another immediate,
non-strict, selector-ordered snapshot, but reads rendered
`HTMLElement.innerText` rather than raw nullable DOM text. The upstream
TypeScript cast is unchecked and can produce JavaScript `undefined` for SVG;
Pwright keeps the promised `Vec<String>` honest by rejecting any
non-`HTMLElement` match as typed `InvalidTarget`. It shares the same 4096-item,
1 MiB-per-item, and 4 MiB aggregate bounds as `all_text_contents`, records an
independent metric, re-resolves after navigation, and retains no handles.

Locator protocol 1.17 adds independently installable `hover`. It follows
Playwright's strict, lazy resolution and waits for visible, stable geometry and
an unobscured hit point, but intentionally does not require enabled state.
Options cover force, auto/no scrolling, an optional set of at most five
canonical modifiers, an optional finite fractional point relative to the
padding box, and trial mode. Omitted modifiers preserve current Page keyboard
state; an explicit empty set temporarily clears it and later restores it.
`ControlOrMeta` resolves against the browser-reported platform. A fixed-size
coordinator belongs to the Page and is shared by its main, local-frame, and
OOPIF Locator routes, so modifier key-downs, the optional mouse move, and key-up
restoration cannot interleave. Trial performs the requested key transitions and
restoration but sends no mouse move. Controlled cancellation cleans pending
restoration; the next input transaction also heals state left by direct future
drop before doing new input.

Locator protocol 1.18 adds independently installable `focus` and `blur` as one
focus-state checkpoint. Both strictly and lazily resolve one Page- or
Frame-owned Locator in the existing target-local utility world. Missing or
disconnected targets retry under the runtime deadline; multiple matches,
invalid selectors, non-elements, script exceptions, and malformed responses
fail terminally. Focus invokes native `focus()` twice and Blur invokes native
`blur()` once, matching Playwright without adding visibility, stability,
enabled, scrolling, input-dispatch, or active-element verification. Neither
route retains a DOM handle or changes the existing Locator resource peak. A
connection-selected 1.17 or 2.0 codec rejects both new commands before
dispatch; those frozen checkpoints remain 1.18 and 2.1.

The implemented Press checkpoint advances those tracks together to locator 1.19
and 2.2. It keeps Playwright's Locator-facing API while extending the shared
Page input coordinator with a fixed 16-slot pressed-key record. A bounded key
parser and static US layout resolve key/code/text/modifier data without a
runtime map; Chromium receives complete key events and macOS editing commands.
The gate spans final focus and ordered key transitions, and cancellation
releases acknowledged keys in reverse order. Dedicated Chrome 152 proofs cover
selection-sensitive text input, shortcuts, Enter/form navigation waiting, and
controlled delayed-chord cancellation with reverse release while leaving the
external browser alive. Full details and acceptance gates are in
`docs/knowledge/pwright-locator-press.md`.

The SetChecked checkpoint advances them again to locator 1.20 and 2.3. One
primitive carries a desired boolean plus force, position, scroll, and trial;
`check()` and `uncheck()` compose locally over it. The backend reads native
checked state through label retargeting, returns immediately on a matching
state, rejects direct radio uncheck, performs a real left pointer click when a
change is needed, and verifies the postcondition without replaying committed
input. Both majors use the same bounded extensible option record because this
is a new route rather than a mutation of legacy Click bytes. See
`docs/knowledge/pwright-locator-set-checked.md`.

The SelectOption checkpoint advances the independently negotiated tracks to
locator 1.21 and 2.4. It adds one strict, lazy command for bounded strings and
conjunctive value/label/index descriptors, including empty-list deselection,
DOM-order results, option-enabled checks, and Playwright's exact input/change
event sequence. The Locator capability remains independently installable: the
ElementHandle overload is deliberately deferred until cross-world Handle
adoption has its own negotiated resource contract. See
`docs/knowledge/pwright-locator-select-option.md`.

The same Page owner now serializes the existing Click route. Click moves the
pointer to the actionability point before pressing, records the current point
and pressed button in fixed-size state, and releases before completing. This
makes cancellation after mouse-down recoverable and prevents Click from
interleaving with Hover input in another Frame or OOPIF. Cleanup releases a
button before restoring temporary modifiers.

Richer Playwright Click parity cannot be represented as an in-place locator
1.x edit. Its positional `click_options` record is intentionally layout-exact,
so appending position, modifiers, delay, scroll, or steps would make a 1.17
payload undecodable by the new codec. The architecture must first select a
versioned extensible-options representation or a new negotiated command route;
until then the legacy wire layout remains unchanged.

The preceding client-composition checkpoint deliberately left locator
protocol 1.16 unchanged. `clear()` dispatches the existing Fill command with an
empty value, so it inherits the same actionability, deadline, cancellation,
backend, and resource behavior without adding an installable route.
`all()` dispatches Count once and pairs that snapshot length with local lazy
`nth(i)` selectors. Rust returns a `LocatorList` that stores one base selector
instead of eagerly cloning it into a `Vec<Locator>`; `get` and `iter`
materialize indices on demand. The list length is stable while each returned
Locator still re-resolves against the current document, matching Playwright's
observable behavior across navigation.

### Browser-provider ownership foundation slice

The browser-provider slices use four provisional crates:

```text
pwright-browser-provider    (standard library only)
pwright-browser-install     (standard library only; explicit install pipeline)
pwright-provider-cdp       -> pwright-browser-provider + pwright-cdp + pwright-core + url
pwright-provider-chromium  -> both providers + pwright-cdp + optional Tokio process/time/fs
pwright-composition-chromium -> explicit Chromium attach/profile assembly over selected v2 crates
pwright-composition-chromium-tokio -> optional authenticated endpoint/Chromium adapter
```

`pwright-browser-provider` separates attachment and launch at the type level.
An `AttachBrowserProvider<Request>` returns `AttachedBrowser<Connection>`,
which contains no process-control token. A `LaunchBrowserProvider<Request>`
returns `LaunchedBrowser<Connection, Lease>`, and only its explicit
`BrowserProcessLease` may request process termination. Termination consumes the
lease so one acquisition cannot issue two termination requests. Dropping an
attachment may disconnect its transport but must not terminate the browser.

`pwright-provider-cdp` is attach-only. It validates and accepts explicit
`ws://` or `wss://` endpoints, delegates connection creation through an
injectable connector, and returns an external attachment. It contains no
executable discovery, process spawning, browser installer, downloader, cache,
or process termination API. HTTP discovery is intentionally deferred to a
separate optional provider because it introduces another protocol and security
boundary.

The raw CDP connection now has count and byte limits for pending commands,
write and event channels, serialized commands, responses, events, protocol
errors, and WebSocket messages. Command admission happens synchronously before
the future is returned. A registration guard removes the pending entry when an
unpolled or active future is dropped, while shutdown drains every entry even
when the reader future itself is cancelled. Outgoing serialization uses a
bounded writer. Incoming response/event parsing moves JSON members instead of
deep-cloning them, and `CdpEvent` clones share one immutable payload across all
subscribers.

`CdpConnectionProductionProfile` adds the production ledger boundary. It holds
a fixed `WireBytes` reader lease for maximum raw-message/parsed-JSON overlap,
reserves a complete command/result peak for every pending command, and attaches
exact `EventItem`/`EventBytes` leases to unique events. Event capacity includes
one transient item/maximum payload because an incoming value is allocated before
bounded admission. A queued command shares its lease with the pending entry, so
cancelling the caller does not make bytes still owned by the writer queue
disappear from accounting.

Legacy v1 subscribers retain the Tokio broadcast receiver contract. An opt-in
v2 production profile instead installs a task-free fanout with preallocated
event and subscriber slots. It retains no event without a subscriber, reclaims
each event after every current cursor advances, reports bounded overwrite as
explicit lag, and drains queued items before reporting close. Its fixed
bookkeeping owns one `BackendStateBytes` lease. The Chromium composition can
derive the subscriber bound from topology, PageLoad, Page-network lanes,
navigation concurrency, and Locator concurrency, and rejects an undersized
explicit bound before connecting. Metrics retain both current subscribers and
the lifetime peak. The full profile's bound of 10 is a safe compositional
ceiling: the canonical recipe shares one network cursor and a real Chrome run
with two slow navigations plus two Locator clicks observed a peak of 6 or 7,
then returned to its three steady cursors with an empty, non-lagged fanout.

In either strategy all delivered clones share the event lease; a consumer
retaining a delivered value causes new admission to fail rather than creating
unbounded distinct payloads.
`ProductionCdpConnector` rejects an omitted plan and request/profile config
drift before opening the endpoint. Raw constructors remain explicit v1/test
compatibility paths.

`pwright-composition-chromium` is the first explicit production composition
root. `PreparedChromiumAttach` validates a selected navigation-only,
locator-only, or combined capability set; builds the exact portable registry,
filtered Chromium backend manifest, and negotiated wire manifest; and checks a
single additive connection/topology/page-route/runtime/backend/codec resource
plan. Additive composition is required because these components can hold their
declared peaks at the same time. The root can then produce budgeted
per-capability wire protocols, complete-topology direct or caller-policy
controlled runtimes rooted at the Browser `ObjectId`, and a handshake-required
server supervisor using the same declared ledger. Preparation accepts no
endpoint and starts no socket, task, runtime, process, browser discovery,
download, Node.js driver, or Chrome instance. `attach` is a distinct operation
that accepts only an explicit `ws://` or `wss://` endpoint and constructs the
production topology before opening it.
`ChromiumAttachFactory` makes that preparation reusable without making
`PreparedChromiumAttach` cloneable: each factory call receives a distinct
connection-local ledger, and each successful attach receives a distinct CDP
connection, topology, ObjectId namespace, and initial epoch.

The aggregate root is no longer the only usable Cargo installation boundary.
`pwright-composition-chromium-navigation` is a separate attach-only leaf over
`pwright-backend-cdp-navigation`, which shares only the command-free CDP
failure classifier with other extracted adapters. It binds one caller-
provisioned Page and injected CDP client to the generated two-command
navigation server chain, claims backend and wire peaks from one production
ledger, and starts no task. The aggregate backend delegates through a thin
compatibility wrapper but is absent from the leaf's graph. A standalone
consumer manifest and CI `cargo tree` allowlist prove that no unrelated
Pwright capability, aggregate backend, client, provider, tracing, Artifact,
browser-install, full composition, or v1 crate enters its normal dependency
graph. This is the first concrete compile-time half of the capability-
composition design; runtime negotiation remains a separate, complementary
boundary. See
`knowledge/pwright-dependency-topology.md`.

The command-free Chromium topology is now an independent foundation at
`pwright-backend-cdp-topology`. It owns the bounded
Browser→BrowserContext→Page→Frame/World mapping, reconnect epochs, popup and
crash state, lifecycle effects, exact native-string accounting, and
rollback-on-drop production transactions, but it sends no CDP command and
starts no task. Its normal dependency closure is exactly the topology leaf,
browser model, generated CDP event model, and core. The aggregate backend
compatibility-reexports the same public types, so existing composition code
keeps its API while future context/Page creation and lifecycle leaves can
depend on topology without importing the command aggregate. A standalone
five-package consumer and exact CI allowlist prevent that foundation from
acquiring capability, client, server, codec, runtime, provider, launcher,
installer, or aggregate dependencies.

BrowserContext/Page creation and lifecycle are now four adjacent backend
leaves rather than private aggregate modules:
`pwright-backend-cdp-context-creation`,
`pwright-backend-cdp-context-lifecycle`,
`pwright-backend-cdp-page-creation`, and
`pwright-backend-cdp-page-lifecycle`. Context creation performs exact
invalid-ID compensation; Context close rejects default/external ownership
before CDP; ordinary Page close confirms target disappearance while
run-before-unload addresses only the Page session; and Page creation depends
on the Page lifecycle leaf for exact-target compensation. Their normal Pwright
closures are respectively 6, 8, 8, and 6 packages. Each has a standalone
consumer and exact CI allowlist excluding the aggregate, unrelated
capabilities, codecs, clients, servers, runtimes, providers, launchers,
installers, and v1. The aggregate compatibility-reexports all four unchanged.
Composition still owns the cross-state topology/runtime/route transaction;
these leaves own only native CDP operations and bounded validation.
Dedicated Chrome 152 tests exercise both a real explicit Page close and the
complete Context-create → Page-create → Page-close → Context-close path; each
confirms exact local retirement while leaving the externally owned browser
alive.

JavaScript Handle lifecycle and retained-Handle operations are two further
backend leaves. `pwright-backend-cdp-handle-lifecycle` owns one bounded native
object ID and issues exactly one `Runtime.releaseObject`, with no evaluation,
topology, runtime, or browser ownership. Its exact normal Pwright closure is six
packages. `pwright-backend-cdp-handle-evaluation` deliberately keeps retained
Handle creation, Handle-local value/Handle evaluation, `jsonValue`,
`getProperty`, and bounded `getProperties` together: these operations share one
private invocation, response parsing, and cleanup transaction. It depends in
one direction on the by-value evaluation leaf and has an exact ten-package
closure; evaluation has no reverse Handle dependency. Both leaves have
standalone consumers and CI allowlists excluding navigation, Locator,
content/screenshot, network events, topology, Context/Page creation and
lifecycle, aggregate backend, codecs, client/server, runtime, provider,
installer, Node.js, and browser acquisition. The aggregate preserves its public
surface with compatibility reexports. Eleven AAA tests remain with the two
implementation owners. A dedicated Chrome 152 test creates real element and
primitive Handles, passes an element Handle back into JavaScript, explicitly
releases all three results, and confirms the externally owned browser survives.

The first Handle server composition is now a separate leaf as well.
`pwright-composition-chromium-handle-lifecycle` owns a bounded route registry
that counts live and in-flight producer reservations. Optional producers attach
opaque typed backend state and one object-local disposer, so lifecycle does not
import evaluation. `Handle.dispose` pre-admits caller-owned continuation
capacity, performs at most one native release, then retires runtime and route
ownership. Dropping a started future transfers that exact transaction; backend
failure preserves both owners for a visible retry. Its exact normal closure is
15 Pwright packages, and a dedicated Chrome 152 proof releases a real retained
object without taking browser ownership.
The same registry reserves and publishes a whole Handle batch atomically around
an injected runtime commit, providing the all-or-nothing primitive required by
bounded `getProperties`.

`pwright-composition-chromium-handle-evaluation` is the corresponding
independent 1.2 server leaf. It composes all six Frame/Handle commands over that
lifecycle registry, owns a separate bounded Frame-to-CDP-world registry, and
admits runtime, route, native-result, codec, and continuation resources before
use. Single and property-batch results commit runtime identity and lifecycle
routes together; every failed publication compensates native objects. Its
normal dependency closure is exactly 24 Pwright packages and excludes the
aggregate composition/backend, topology, Context/Page commands, clients,
providers, launchers, installers, Tokio, Node.js, and browser acquisition. The
direct-command abandonment window is closed. The separate backend-neutral
`pwright-plugin-ordered-handle-delivery` now owns the remote boundary: it
pre-reserves cleanup, fences every single or batch `Create` before `Result`,
retains that reservation through the physical Result send, and performs
ordered disposal if execution or delivery is abandoned. The leaf exposes its
exact protocol and raw services for this opt-in stack, so direct/in-process
users do not inherit transport policy. The aggregate Chromium remote-handler
factory now delegates this boundary to the same plugin; its older public
decorator types remain compatibility APIs, but no second delivery policy is
active in the default composition.

The CDP Handle-evaluation backend also exposes an opt-in
ElementHandle-adoption primitive. It follows Chromium's
`DOM.describeNode(objectId)` then
`DOM.resolveNode(backendNodeId, executionContextId)` mapping, rejects JSHandles
and cross-session nodes before DOM I/O, validates the returned node under the
existing native-ID/preview bounds, and returns a must-use temporary owner with
explicit release. Its production batch prevalidates and reserves the complete
worst-case set before DOM I/O, charges one `RemoteHandle` plus exact bounded
backend bytes per temporary object, and exhaustively cleans up partial failure.
The composition wrapper holds one caller-reserved cleanup continuation from the
first adoption poll through final release, including dropped adoption, owner,
and release futures. Seventeen backend tests, four composition-owner tests, and
two Chrome 152 main-to-isolated-world proofs pass. A separate adoption-aware
installation now joins argument classification/substitution and
evaluation/topology cleanup into the public command transaction without
changing the default exact-world profile.

Page and network event normalization are also independent backend leaves.
`pwright-backend-cdp-page-events` consumes the command-free topology leaf to
activate only Page lifecycle delivery, map current main-Frame `load`, and map
topology `PageReady`/`PopupLinked` effects. `pwright-backend-cdp-network-events`
owns one bounded Page/session HTTP correlation lane and converts request,
response, redirect, finish, and failure events into atomic portable lifecycle
batches. Neither leaf owns a CDP receiver, output queue, reader task, runtime,
transport, response body, browser session, or process. Each has an exact
seven-package normal Pwright closure, standalone consumer, and CI allowlist;
Page events depends on topology, while network events deliberately does not.
The aggregate compatibility-reexports both APIs. Their ten AAA tests remain
with the implementation owners. Three Chrome 152 tests additionally prove a
caller-owned cursor receives the current main-Frame load, a bounded real HTTP
request/response/finish sequence does not read response bodies, and redirects,
HTTP errors, and transport failures remain distinct while external Chrome
survives.

Page input is now an independent backend foundation at
`pwright-backend-cdp-input`. It keeps the mutually dependent keyboard layout
and serialized keyboard/mouse transaction state together, while moving the
shared 16-key chord bound into the command-free `pwright-input-model` leaf.
The input backend owns no selector, Locator command, capability, runtime,
transport, task, browser session, or process; it accepts only a caller-owned
CDP client when a transaction starts. Public chord parsing rejects empty,
trailing-empty, and oversized input, and key resolution rejects unknown input,
all before dispatch. Dropped or cancelled transactions restore pending mouse
buttons, physical keys, and modifiers in reverse order. Its standalone
consumer has an exact five-package
normal Pwright closure: consumer, input backend, input model, CDP, and core.
The aggregate preserves its three existing public input coordinator types by
compatibility reexport, and the Locator adapter imports the leaf in one
direction. Its 15 AAA tests cover parsing, Playwright US-key resolution,
serialization, interpolation, auto-repeat, failure rollback, and drop cleanup;
real Chrome 152 Locator action and cancellation tests exercise the extracted
foundation without transferring browser ownership.

Value evaluation is the second compile-time slice. Its implementation was
extracted into `pwright-backend-cdp-evaluation`, while the complete backend
reexports it for compatibility. `pwright-composition-chromium-evaluation`
binds one pre-provisioned Frame to an injected target-local CDP client and the
one-command generated evaluation handler. Main-Frame installation forbids a
spurious execution-context route; child-Frame installation requires a bounded
`uniqueContextId` and holds the declared route lease. The combined profile
accounts backend operation state, persistent route state, and one simultaneous
wire frame. Standalone and umbrella CI consumers reject navigation, Handles,
the aggregate backend/composition, client/provider/installer, runtime choice,
Node.js, and browser acquisition from this server leaf.

Read-only DOM content is the third compile-time slice. Its implementation is
owned by `pwright-backend-cdp-dom-content`, and the aggregate backend
compatibility-reexports it without becoming a dependency of the leaf.
`pwright-composition-chromium-dom-content` binds one pre-provisioned Frame and
its exact bounded utility-world `uniqueContextId`, evaluates only the fixed
library serializer, and installs the generated `Frame.content` handler. Its
profile jointly accounts persistent route storage, backend and decoded
`DomContentBytes` results, and wire encoding. Standalone and umbrella proofs
exclude public evaluation, screenshot, navigation, aggregate backend and
composition, clients, providers, installers, runtimes, Node.js, and browser
acquisition. Evaluation and DOM content share only
`pwright-backend-cdp-common`, a command-free CDP failure classifier with no
capability, service, route, resource, runtime, or provider ownership.

Page screenshot is the fourth compile-time slice. The native CDP capture
implementation moved to `pwright-backend-cdp-content`, while the aggregate
backend reexports it and Locator screenshot explicitly reuses its opaque
capture-admission transaction. `pwright-composition-chromium-content` binds
one pre-provisioned Page, validates exact object identity before CDP, and
installs only the generated `Page.screenshot` handler. Its plan covers a
backend-owned binary result, a separately decoded public-protocol result, and
the outbound attachment's wire buffer as overlapping owners. Standalone and
umbrella proofs exclude DOM content, public evaluation, Locator screenshot,
navigation, aggregate backend/composition, clients, providers, installers,
runtimes, Node.js, and browser acquisition.

Locator screenshot is the fifth compile-time slice.
`pwright-backend-cdp-locator-screenshot` owns capture orchestration and Frame
clip translation but receives selector geometry through the command-free
`LocatorScreenshotGeometryResolver` plugin contract.
`pwright-composition-chromium-locator-screenshot` installs only that command,
codec, handler, and capture resource plan. The first-party
`pwright-backend-cdp-selector-geometry` leaf supplies strict Chromium
resolution without importing public Locator commands, while
`pwright-backend-cdp-selector-runtime` is the single compiler/runtime used by
both that leaf and the full Locator backend. Its exact 23-package executable
profile rejects both `pwright-cap-locator` and the aggregate CDP backend; the
aggregate retains existing behavior through a small `CdpLocatorService`
compatibility adapter.

Backend descriptors represent everything an adapter could implement, whereas
`BackendManifest::for_installed_capabilities` now contains only features owned
by capabilities selected for this composition. The older strict constructor
still rejects any missing backend capability. This separation is necessary:
installing Chromium CDP support must not force navigation and locator into the
same binary or advertised protocol.

The runtime now calls `TargetedService::call_for` after semantic object-kind
validation, preserving the command's `ObjectId` through backend dispatch.
Existing parameter-only `Service` implementations use a compatibility blanket
implementation. `CdpPageServices` is the concrete bounded multi-page resolver:
it retains only materialized Page routes, maps each portable Page identity to
its exact target/session pair, preserves target-local locator installation state
across unchanged synchronizations, and retires detached or reconnecting routes.
Per-page navigation and locator services are created by factories whose local
concurrency counters remain shared across every bound CDP session. The route
profile accounts six native-ID string slots per Page: one target/session pair
for stable route identity, one pair owned by the concrete CDP session, and two
overlapping utility-context values during locator-route replacement.

`transact_topology_update` is the preferred three-state mutation boundary. The
production topology first builds a resource-accounted shadow whose drop is a
rollback. Page routing then builds its complete replacement while holding the
route lock. The runtime validates and applies the update's entire Create/Dispose
batch under one state lock, reserving the complete lifecycle outbox and every
new object lease before mutation; controlled runtimes also request cancellation
for calls in disposed subtrees. Only after that batch succeeds are the
infallible route and topology moves published. Failure injection proves that a
two-object Page/main-Frame update rejected by a one-slot outbox changes none of
the three live views. `synchronize_topology_update` remains available for event
adapters that already own a committed topology update, but new composition code
should transact instead of committing first.

The composition now also materializes generated concurrent server handlers only
for selected capabilities. Navigation and locator factories accept a shared
runtime and return independently chainable handler pairs backed by protocols
from the same production ledger. Applications can therefore install either
capability without constructing the other's codec or handler chain. A shared
`Arc<runtime>` is also a valid lifecycle outbox source, allowing command,
client-control, and outbound lifecycle components to refer to one runtime
without moving it or introducing a global singleton.

`pwright-provider-chromium` is the separate opt-in process-owning provider. It
discovers only executables already installed by the user or OS, or accepts an
explicit validated absolute path; it has no downloader, installer, browser
archive, cache, or Node.js driver. Fresh launch requires a caller-created
absolute, empty user-data directory. Persistent launch accepts only an opaque
`ChromiumPersistentProfile` initialized in an empty directory and carrying the
exact versioned Pwright marker on every reopen; an arbitrary nonempty Chrome
profile cannot be silently selected. Both modes remain caller-owned and are
never recursively deleted by the provider. Before persistent-profile spawn,
the provider removes only Chromium's stale `DevToolsActivePort` runtime file;
the version marker and browser state remain intact. Provider-owned arguments
force CDP to `127.0.0.1` with an OS-selected port; caller arguments may not
override the debug address, port/pipe, or user-data directory. The newly
written `DevToolsActivePort` is read from that dedicated profile, validated as
a browser WebSocket path, and turned into a loopback-only endpoint before the
injected CDP connector runs.

Process ownership has two phases. Until endpoint discovery and connection both
succeed, a cancellation guard terminates the newly spawned process; a startup
or connection failure also kills and waits for that owned child. After success,
the guard is disabled and ordinary `LaunchedBrowser`/lease Drop does not kill
the browser. Calling the consuming `BrowserProcessLease::terminate` reenables
cancellation cleanup, observes an already-exited process without sending a
kill, or kills and waits exactly once. The lease retains the profile path for
caller cleanup.

Orderly owned-browser shutdown is a separate composition over the connection
and consuming lease. `ChromiumBrowserShutdown` uses an injected graceful closer
and process-exit waiter. Its live closer sends CDP `Browser.close` with an
explicit command bound, then independently waits for process exit with a
second bound. A missing response is recorded but is not treated as proof that
shutdown failed: the owned process state is authoritative. If the graceful
bound expires or observation fails, shutdown sends a force kill and waits with
a third independent bound. The report distinguishes already-exited, graceful,
and forced completion. Dropping the shutdown future at any point reactivates
the child kill guard, so cancellation cannot silently orphan an owned process.

`pwright-composition-chromium-launch` is now the separately selected leaf that
joins this process authority to the v2 production composition. The attach-only
root still has no launch-provider dependency. A `ProductionCdpConnection`
retains the connection plan and originating ledger identity after the launch
provider connects; `PreparedChromiumAttach::bind_connection` consumes that
proof and rejects either plan drift or a connection charged to another
attachment ledger. Only the owned leaf returns
`OwnedChromiumComposition<Child>`, which retains the unique process lease.
External `AttachedChromiumComposition` has no shutdown method by compile-time
proof. Owned shutdown borrows the owner, so dropping an unpolled future leaves
the lease available; after the first poll, the provider's kill-on-drop guard
makes cancellation cleanup authoritative. A live Chrome 152 test launches a
separate installed browser with a fresh profile, reaches it through the
budgeted connection, consumes the lease through bounded shutdown, observes the
same process ID and profile in the report, and confirms the unrelated dedicated
CDP Chrome remains reachable. No executable discovery, browser installation,
or download is implicit in the composition recipe.

Owned-browser memory teardown has a distinct final observation. Normal process
attribution requires both runtime and browser roots to exist, while successful
owned shutdown requires the opposite for the browser. The process probe can
therefore issue a typed exit proof only from a bounded snapshot that still
contains the runtime PID and omits the exact owned browser PID. The workload
recorder combines that proof with post-drop resource/object metrics, checks the
original root identities and deadline, and deliberately exposes no synthetic
post-teardown browser RSS. A separate bounded series recorder reports runtime
delta and measured pre-teardown browser RSS ranges. Three live Chrome 152 runs
completed gracefully, released all ledger leases, proved three distinct PIDs
absent, and removed their test-owned temporary profiles.

Remote owned close is a separate optional leaf, not an attach capability.
`pwright-cap-browser-lifecycle`, `pwright-codec-browser-lifecycle`,
`pwright-client-browser-lifecycle`, and
`pwright-server-browser-lifecycle` package one portable `browser.close`
operation without pulling process or Chromium types into the public contract.
Selecting it reserves its codec/wire resources and augments only the launch
factory's handshake; the nested attach manifest is unchanged, and missing
handshake configuration fails before spawn. The unique owner becomes a
non-cloneable close grant only after the exact accepted server session
negotiates the capability. “Exact” is enforced by connection-root identity:
the accepted handshake retains the production supervisor's shared resource
ledger, and grant conversion requires it to be the same ledger retained by the
launched composition. Equal limits, equal plans, equal numeric Browser IDs,
and the same negotiated capability on a different connection are insufficient.
An unbound supervisor is also insufficient. No random or global session
registry is introduced. Rejection returns the owner unchanged, while an
external attachment cannot construct a grant by compile-time proof.

The owned command service validates the exact Browser target and prepares its
whole runtime subtree before claiming authority. Once admitted, shutdown owns
the consuming lease even if its caller drops the command future: the exact
future transfers into capacity synchronously reserved from a caller-provided
cleanup queue. Provider completion or terminal shutdown failure commits
descendant-first disposal, and an ordered control barrier publishes through
Browser Dispose before the command Result. Encoded client/server testing proves
the client sees Page, Context, and Browser handles become stale before close
resolves. A live Chrome 152 proof obtains actual privileged and unprivileged
accepted handshakes, rejects both an unbound privileged handshake and a
privileged handshake from an independently prepared connection budget,
preserves the same process owner after every rejection, and permits only the
launched composition's own privileged session to reap the exact process and
empty its runtime tree.

Portable close success remains exactly unit-valued. Provider diagnostics do
not enter capability, codec, client, server, or wire crates. The launch
composition instead exposes `OwnedBrowserShutdownReportSink<Report>` as an
optional synchronous plugin, defaulting to a zero-state discard sink. A sink is
called only after runtime disposal commits and can receive the concrete
`ChromiumShutdownReport` containing process, exit, method, acknowledgement, and
profile diagnostics. `OwnedBrowserCloseExecution` groups that independent sink
with the independently selected continuation executor, keeping the handler
recipe composable without widening the portable operation. Encoded tests prove
a provider-shaped report reaches the sink while the result frame still decodes
as `()`. The live Chrome proof additionally matches the observed report's PID
and profile to the exact launched process.

The terminal-event slice packages `browser.disconnected` independently as
`pwright-cap-browser-events`, `pwright-codec-browser-events`,
`pwright-client-browser-events`, and `pwright-server-browser-events`. The
public payload is unit-valued, matching Playwright; source identity lives in
the event route. The client queue rejects overflow because terminal lifecycle
state cannot be sampled or replaced. The server publisher is an at-most-once,
queue-free caller-driven send whose first polled attempt becomes terminal even
if dropped, preventing unsafe retry after a partial stream write. Runtime
control now supports a split Browser-disposal fence: send every preceding
child-first Dispose, stop before Browser Dispose, publish disconnected, then
resume through Browser Dispose while one ordered permit remains held. This
path is selected through `BrowserEventsInstallationProfile`, which is invalid
without remote handshake, ordered control, and a 17-byte outbound frame bound.
The composition constructs its publisher from the accepted protocol and shared
production ledger, then returns it alongside an event-aware CDP liveness lane
and a control-cleanup decorator. The same clone is passed to the owned-close
wrapper, so native loss and explicit close contend on one at-most-once state.
Native loss orders descendant Dispose, disconnected, and Browser Dispose
before delegating to ordinary connection cleanup; explicit close additionally
keeps the correlated Result last.
The live owned-launch proof exercises that exact bundle against a separately
spawned Chrome 152 and then polls the shared liveness lane after process exit,
proving a single publication wins across both terminal observations.

Client close-reason state follows the same root-sharing rule without coupling
the core client to Browser lifecycle. Each root family owns one bounded
terminal slot configured by `ClientLimits`; descendants clone only that slot.
The optional Browser-close extension installs the first non-empty reason on
the close future's first poll, rejects an oversize value before dispatch, and
post-disposal command admission includes that reason in its local
`TargetClosed` failure. A production client recipe must align the client byte
bound with or below the negotiated lifecycle codec bound.

`pwright-browser-install` is the separate managed-installation contract. It has
no dependencies and contains no network client, runtime, archive decoder,
browser bytes, launcher, or cache. A request names an exact engine, validated
revision, OS/architecture target, expected SHA-256 or SHA-512 digest, and an
existing canonical empty destination. It also consumes a non-default,
non-cloneable `BrowserDownloadPermission`. `BrowserInstallPipeline` composes
four injected stages in a fixed order: resolve, fetch, verify, install. A root
that became nonempty is rejected before resolution, and installation cannot run
unless the typed verification stage succeeded. The resulting
`InstalledBrowser` is only a verified package/path receipt; it carries no
connection or process-control authority. Consumers may explicitly adapt its
executable path into a launch request, but no provider depends on the installer
crate and no launch path can invoke downloading transitively.

These slices complete the DT-050 provider, ownership, shutdown, persistent
profile, and optional-installation package boundaries. They do not yet define
the future public `BrowserType`, `Browser`, reconnect API, a concrete artifact
catalog/fetcher/verifier/extractor, installation rollback, or cache retention.

### Runtime execution-control foundation slice

The next exploratory runtime slice may turn timeout and cancellation metadata
into enforced behavior without selecting Tokio or reading a system clock:

- a typed command cancellation hook receives a compact operation context and a
  classified cancellation reason; capability backends implement the hook and
  perform operation-specific cleanup;
- a `ControlledDispatcher` races the capability future against an explicit
  cancellation signal and an optional caller-injected relative-deadline future;
- an external cancellation request must match both call ID and cancellation ID,
  and the first accepted reason wins a cancellation/deadline race;
- successful operation completion wins when it is observable in the same poll
  as cancellation or deadline expiry;
- after cancellation wins, the operation future is dropped before the backend
  cancellation hook runs; the pending-call entry is removed only after that
  hook finishes;
- a failed cancellation hook is reported as `Cleanup`, while successful hooks
  produce `Cancelled`, `Timeout`, `TargetClosed`, or `TransportClosed` according
  to the winning reason;
- the existing `DirectDispatcher` rejects timeout or cancellation metadata
  explicitly, so an uncontrolled dispatcher can never pretend to enforce it.

The deadline scheduler is a trait returning a future for a relative duration.
The standard-library-only runtime supplies no real clock and starts no task;
production timer implementations belong in executor-specific crates. A narrow
`pwright-runtime-tokio` adapter may supply `tokio::time::sleep` without making
Tokio a dependency of core, capabilities, or the generic runtime. The CDP
navigation adapter may implement cancellation with `Page.stopLoading`, but CDP
types remain below the portable cancellation contract.

The later encoded-client cancellation checkpoint makes dropping a sent public
API method future enqueue a bounded caller-cancellation message; `Drop` itself
still performs no I/O and never awaits a backend hook. It does not define
timeout inheritance across nested
Playwright operations or guarantee that a backend protocol has a perfect
operation-specific abort primitive.

### Lifecycle fan-out and target-admission foundation slice

The object lifecycle, pending-call, and command-ordering slice follows these
rules:

- disposing an object in a controlled dispatcher removes its subtree in
  child-first order, marks every affected pending call `ParentClosed`, and
  wakes each call so its typed backend cancellation hook performs cleanup;
- lifecycle and transport termination already accepted by the runtime preempt
  a simultaneously observable operation result; ordinary caller cancellation
  and deadline expiry retain operation-completion-first race semantics;
- an uncontrolled direct dispatcher must refuse to dispose a subtree that
  still owns pending calls because it has no backend cancellation hook;
- a standard-library-only `TargetScheduler` admits at most one active command
  per target, preserves FIFO order for that target, and permits independent
  targets to progress concurrently;
- target lanes and queued waiters have explicit hard limits. Duplicate call
  IDs and capacity exhaustion fail before a capability handler runs;
- dropping a queued acquisition removes that waiter, while dropping or
  explicitly releasing an active permit wakes exactly the next waiter and
  removes an empty lane;
- queue metrics report current and peak target lanes and waiters.

The scheduler is an admission primitive, not an executor: it creates no task,
thread, timer, or unbounded channel. A dispatcher awaiting admission retains
the command in the already-bounded pending-call table, so explicit cancellation
and object disposal can still find and wake queued commands. If cancellation
wins before admission, no backend cleanup hook runs because no backend
operation started.

This target-only slice does not define which Playwright operations may safely
bypass a page lane or multi-command transactions such as action plus navigation
waiting. Cross-target fairness and aggregate concurrency are added by the
connection-wide admission slice below.

### Connection termination and abandoned-call cleanup foundation slice

The next exploratory runtime slice may make connection ownership terminal and
make dropped operation cleanup explicit under these rules:

- transport closure is a one-way `Open -> Closed` transition. It rejects later
  object registration and command admission, invalidates every live object in
  deterministic child-first order, requests `TransportClosed` for every
  pending call, and wakes active and queued calls while preserving any earlier
  accepted cancellation reason;
- an accepted transport close preempts a simultaneously ready operation result.
  A call that had not reached its backend completes without a cancellation
  hook, and an active call drops its backend future but also does not invoke a
  transport-dependent abort command after that transport is gone;
- repeated close notification is idempotent and produces no second cleanup
  plan or wake-up;
- dropping a controlled dispatch future removes its pending-call and target
  admission state synchronously. If its backend operation had started and the
  transport remains open, the runtime submits exactly one owned cancellation
  future with the first accepted reason, defaulting to `Caller`;
- cleanup submission goes through an injected `CleanupExecutor`. The default
  implementation is a standard-library-only `CleanupQueue` with a fixed
  capacity, FIFO extraction, explicit saturation errors, and current/peak,
  accepted, rejected, completed, and failed metrics;
- neither the queue nor the generic dispatcher starts a task or selects an
  async runtime. An executor integration must drain submitted jobs. Submission
  rejection is counted and observable; it is never converted into an
  unbounded spawn or queue.

This slice does not yet define graceful protocol shutdown handshakes, backend
object-disposal commands, cleanup retry policy, process termination, or how a
Tokio integration supervises and drains queued jobs at application shutdown.

### Optional Tokio cleanup execution slice

Applications already using Tokio may inject `TokioCleanupExecutor` instead of
manually draining `CleanupQueue`. It requires a currently active Tokio runtime,
uses a fixed semaphore capacity for reserved plus active cleanup jobs, rejects
immediately on capacity exhaustion, and exposes reserved/in-flight peaks plus
accepted, rejected, completed, failed, timed-out, and runtime-cancelled metrics.
It never constructs a runtime or worker thread itself.

The adapter now implements `ReservedCleanupExecutor`: irreversible operations
can claim a permit before backend work, later submissions cannot steal it, Drop
of an unused reservation releases it, and consuming submission does not perform
a second capacity check. `new_with_job_timeout` additionally requires a
caller-selected nonzero wall-clock bound. Expiration drops the opaque cleanup
future and releases its permit. This deliberately delegates escalation to the
job's cancellation-safe ownership contract. For owned Chromium close, dropping
the admitted shutdown activates the provider's kill-on-drop guard; the rolled
back runtime lifecycle transaction remains available for the mandatory native
disconnect lane to dispose. Runtime shutdown is independently classified as
`cancelled`, not `timed_out`.

The application policy is now an opt-in part of this same Tokio adapter. A
one-way `begin_shutdown` closes new submission and reservation admission while
honoring every reservation accepted before that transition. `shutdown` waits
for both reserved and in-flight work; `shutdown_with_timeout` applies a
caller-selected nonzero drain bound and returns the still-live counts without
cancelling cleanup when that bound expires. A later completion or shutdown wait
still produces one `ShutdownDrained` transition. Admission and the drain
transition share a small synchronous gate, preventing a job from appearing
after a zero-work shutdown observation.

Retry remains narrower than cleanup execution. `CleanupJob` and reserved
continuations always execute once. Only `try_submit_retrying` can retry, and it
requires a caller-provided factory that constructs a fresh future for each
attempt plus a finite nonzero attempt count and fixed delay. The logical job
holds one semaphore permit across attempts and delay, and its optional job
deadline bounds the whole retry sequence. This prevents the runtime from
duplicating a unique Browser-process authority or guessing whether a partially
completed operation is idempotent.

An optional synchronous `TokioCleanupReporter` receives accepted, rejected,
retry, terminal-job, shutdown, timeout, and drained events with monotonic local
job IDs. Its default is zero-state discard. Reporter panics are isolated and
counted so cleanup ownership and drain accounting remain authoritative; a
reporter that needs asynchronous I/O must perform its own bounded nonblocking
admission. Metrics now distinguish logical jobs, attempts, retries, job
timeouts, runtime cancellation, drain timeouts, and reporter panics. A
composition test drops a polled owned `Browser.close`, begins application
shutdown, releases the exact reserved continuation, and proves the drain
finishes only after the Browser runtime tree reaches zero live objects.

### Connection-wide command admission foundation slice

Per-target serialization does not cap the aggregate work of many independent
pages. The next runtime slice therefore composes two executor-neutral permits:

1. a command first becomes the FIFO head for its target;
2. that target head then enters a connection-wide FIFO for one of a fixed
   number of active backend-command permits;
3. both permits remain owned until operation completion or cancellation cleanup
   finishes.

This order prevents followers for one busy target from reserving scarce global
capacity while waiting behind their target head. Connection fairness is thus
FIFO among admitted target heads, not absolute submission order across all
commands. Cancellation, deadlines, lifecycle disposal, transport closure, and
future drop may interrupt either admission stage and must release every permit
already held. No backend cancellation hook runs until the backend operation has
actually started.

The connection scheduler has fixed active and waiting limits, rejects duplicate
call IDs and zero active capacity, reports current/peak counters, and starts no
task or timer. `RuntimeLimits::new` defaults the active-command bound to the
already-bounded pending-call count; callers may lower it explicitly to enforce
a tighter memory or backend-pressure budget.

When a dispatcher is composed with a shared `ResourceLedger`, every call owns
`PendingCall` from runtime admission through terminal cleanup, including both
scheduler queues. Only a granted connection permit additionally owns one
call-scoped `ActiveCommand`; target heads waiting for connection capacity do not.
The connection scheduler reserves before granting and atomically moves the same
lease to the oldest FIFO waiter during promotion, so a saturated handoff neither
briefly exceeds the global limit nor reports a false lower current/peak value.
Dropping a granted-but-unpolled acquisition, an active permit, or the enclosing
dispatch future releases that ownership through RAII.

### Codec-neutral wire boundary foundation slice

The logical typed protocol remains in `pwright-core`; encoded transport state
belongs in a separate `pwright-transport` crate depending only on core. The
boundary uses these invariants:

- a `FrameRoute` structurally distinguishes command, result, event, control,
  and handshake routing. Call, object, method, and event identities remain
  typed rather than being reinterpreted from one generic integer field;
- a `WireFrame` owns one codec payload and zero or more out-of-band binary
  attachments. Construction validates payload bytes, attachment count,
  per-attachment bytes, and aggregate bytes against caller-provided hard limits
  before the frame can enter a transport queue;
- fragmented input is assembled only after its declared size is checked. Every
  chunk is checked for overflow and completion requires the exact declared
  length, so malformed framing cannot trigger unbounded growth. An optional
  shared-ledger constructor reserves the full declaration before allocation,
  and the non-cloneable `AssembledBytes` output retains that `WireBytes` lease
  after successful assembly;
- a generic `WireCodec<Message>` translates between application messages and
  bounded frames. A `FrameTransport` sends and receives frames through GAT
  futures without choosing Tokio, WebSocket, JSON, or a thread model. Dropping
  a send future must either write nothing or retain enough transport-owned
  state to finish that frame before the next one;
- version policy accepts only the configured major version and an inclusive
  minor range. Version failure is distinct from codec failure;
- the remote-client side uses a bounded call correlator. It rejects duplicate
  outgoing call IDs, unknown results, and capacity exhaustion, and drains in
  deterministic call-ID order on transport loss.

The base crate does not select a concrete header encoding, JSON schema,
WebSocket fragmentation format, authentication method, reconnection policy, or
binary streaming protocol. Optional transports must consume these bounded
contracts rather than weakening them. The first stream option follows below.

### Canonical stream envelope and optional Tokio transport slice

`pwright-transport-stream` defines a deterministic, standard-library-only
binary envelope for `WireFrame`. A fixed header carries a magic value, envelope
format version, typed route fields, protocol version, payload length, and
attachment count. A bounded attachment-length table precedes payload and
attachment bytes. Integers use network byte order and unused route fields must
be zero, preventing multiple byte representations of one logical frame.

The codec validates the outer envelope bound, payload length, attachment count,
each attachment length, and aggregate bytes before copying their bodies. It
rejects truncation, trailing bytes, unknown routes, noncanonical fields, and
arithmetic or platform-size overflow. This envelope version is executable v2
prototype framing; it does not replace the still-open payload IDL and
compatibility-generation decision.

`pwright-transport-tokio` is a separate optional adapter implementing
`FrameTransport` over caller-owned `AsyncRead` and `AsyncWrite` halves. Every
envelope has an eight-byte length prefix that is checked against the configured
limit before receive allocation. Independent Tokio mutexes allow one reader and
one writer to progress concurrently while serializing multiple writers. Clean
EOF is accepted only before a new prefix; partial prefixes and bodies are I/O
errors. Explicit `shutdown` owns only the supplied write half.

Write-half closure is an optional transport capability:
`ShutdownFrameTransport` extends `FrameTransport` only for adapters that own a
real connection boundary. It must serialize with sends, finish and flush a
transport-retained partial frame, and make the peer observe clean EOF after all
preceding frames. Dropping its future must leave shutdown retryable. The Tokio
adapter and production wrapper implement this contract, and the blanket
`Arc<T>` implementation keeps shared transport composition generic.

`pwright-transport-memory` is a second, standard-library-only implementation of
both transport traits. A pair owns two independent bounded FIFO directions.
Each direction retains at most the constructor-supplied number of bounded
frames and one receive waker; full queues reject the newest frame explicitly.
Send and shutdown are lazy caller-driven futures, so dropping either before its
first poll changes no transport state. Closing a sender lets its peer drain all
preceding frames before clean EOF, while dropping a receiver clears that queue
and makes later peer sends fail. A typed client/server command contract runs
unchanged over this transport, proving that command semantics do not depend on
the Tokio stream adapter.

Protocol-level graceful close now keeps its version separate from the outer
application-frame version. The existing handshake's bounded, named version
range is reused with the reserved `pwright.control` entry, so the bootstrap
wire format does not change. Absence means the historical 1.0 baseline; only an
explicitly returned 1.1 selection enables `Close` and `CloseAck`. The helper
requirement/support entries are optional for compatibility, but a validated
ServerHello cannot invent the entry or select outside the offered range.

`NegotiatedControlCodec` is constructed from that ServerHello with a local
Client or Server role. It keeps the selected control minor internally while
`WireProtocol` continues to put the independently selected application version
in the frame header. Both encoding and decoding reject Close/Ack below 1.1,
client-sent Ack, server-sent Close, and the inverse receive directions before
the message reaches runtime state.

`pwright-connection-control` owns allocation-free client and server close
machines. They expose the next ordered side effect without starting a task or
timer. The client sends Close, awaits Ack, then shuts down its write half. The
server receives Close, runs the existing reasoned cleanup barrier, sends Ack,
then shuts down its write half. Client wire state prevents new commands before
Close is submitted and validates/discards already preceding frames one at a
time while waiting. The server adapter decorates any existing control handler,
so browser/runtime cleanup remains injected. State lives on unique connection
owners, allowing a cancelled Ack wait or shutdown to resume without a second
Close. A full exchange and that cancellation path pass over the bounded memory
transport with no Node.js, browser, socket, task, or executor dependency in the
state-machine crate.

The Chromium production root now closes the gap between those primitives and
an actual server composition. Selecting remote control adds the reserved
`pwright.control` support range to its handshake manifest; omitting that plugin
adds nothing. Each accepted connection derives a server-role codec from the
returned ServerHello for the runtime receiver, lifecycle outbound lane, and
graceful-close decorator. The factories remain separate so an activated Page
subscription receiver or another control decorator can be assembled without a
fixed stack, while the convenience parts factory returns the standard stack.
Once the outer decorator completes cleanup, Ack, and write shutdown, it reports
`PeerGracefulClose`; the supervisor observes the control event and terminates
with that reason before any later EOF can erase the distinction.

Production stream composition uses a calculated retention profile, not the
largest numbers copied independently from each limit. The stream codec finds
the attainable maximum while each attachment adds an eight-byte header and
reduces envelope body room. The outbound term covers the already-budgeted
input frame, encoded envelope, and length-prefixed pending write; the inbound
term covers its envelope and decoded frame. Those terms are added because the
independent reader and writer can be live together. The resulting
`WireBytes`, scope, and lease requirements are claimed before production
transport construction.

A budgeted `WireFrame` carries the identity of its connection ledger even when
its body is empty. A production Tokio transport checks this identity before
outbound encoding, so a raw frame or a frame charged to an equal-looking but
different ledger cannot bypass connection accounting. `WireProtocol` has its
own production claim and emits frames bearing that proof. Low-level raw-ledger
constructors remain for isolated adapters and v1 compatibility.

`ProductionFrameTransport` is the codec-neutral capability boundary between a
transport plugin and a production client. It exposes both its complete
retention requirements and the ledger it actually uses. The encoded client
derives a combined profile from those requirements, its directional frame
limits, and its pending-call bound. This profile takes the maximum of the
full-duplex-with-backlog state and the encode-before-registration rejection
state, then adds the corresponding pending/frame/staging metadata ownership.
Production channel construction accepts only this trait, claims the combined
plan, and rejects a transport built from another connection ledger. Direct,
cancellation-control, and post-handshake factories share that rule without
making the generic client or transport crates depend on Tokio.

Neither crate opens a TCP/Unix socket, chooses client or server roles, creates a
Tokio runtime, spawns a task, or defines authentication. Endpoint factories,
endpoint identity extraction, concrete authentication schemes, and WebSocket
mapping remain separate composition layers.

`pwright-endpoint-tokio` is the first such optional leaf layer. On Unix it
binds a caller-owned domain-socket listener without starting an accept loop,
sets the socket to owner-only mode `0600`, and converts each explicitly accepted
stream into the bounded Tokio frame transport. Both peers read kernel-supplied
UID/GID/PID credentials. The client requires an expected server UID before it
exposes the transport, while the accepted server connection can be converted
into a `UnixPeerGate` that authorizes the client UID before ClientHello
negotiation. No secret enters the hello payload.

Binding never removes an existing pathname. The endpoint records the created
socket's device and inode and removes it on explicit close or Drop only if both
still match and the path is still a socket; a replacement file is preserved.
The caller controls the accept loop and therefore connection admission.

The same optional crate now has a distinct Tailnet TCP adapter. It never binds
wildcard, loopback, LAN, or public addresses: bind, connect, and accepted peer
addresses must be inside Tailscale's
[documented IPv4 or IPv6 device ranges](https://tailscale.com/docs/reference/reserved-ip-addresses).
Address membership alone is not treated as application authorization. Before
either side can obtain `TokioStreamTransport`, a versioned fixed-size exchange
uses a fresh server nonce and direction-separated HMAC-SHA256 proofs of a
caller-supplied 32-byte PSK. Thus the credential is not placed in ClientHello
or sent over the connection, captured proofs cannot be replayed for a fresh
nonce, and a client proof cannot be reflected as a server proof. The accepted
value is the only public constructor of `TailnetPeerGate`, coupling the
Pwright authorization context to a completed endpoint proof.

This adapter relies on the Tailnet's authenticated encrypted network channel
for traffic confidentiality and adds application-level mutual access control;
it is not an Internet TLS replacement. The caller still owns accept
concurrency, authentication and drain deadlines, PSK provisioning, and
rotation. A public endpoint uses the separate TLS/mTLS adapter rather than
relaxing this address policy.

The general TCP alternative in the same leaf crate is caller-configured mutual
TLS. `MutualTlsEndpoint` and `connect_mutual_tls` accept rustls configurations
instead of owning a CA, certificate store, server-name policy, or private-key
lifecycle. Both configurations must offer the exact `pwright/1` ALPN. After
normal rustls verification, both peers must present a certificate whose leaf
DER SHA-256 fingerprint occurs in a nonempty, duplicate-free
`CertificatePins<N>`. The const-generic capacity makes an old/new rotation
overlap explicit and bounded. Transport construction occurs only after TLS,
ALPN, and pin checks; the accepted connection is the sole constructor of a
`MutualTlsPeerGate` carrying the authenticated address and certificate
fingerprint into the canonical server handshake.

The listener requires one explicit interface address and rejects wildcard
binding. A semaphore is acquired before `accept`, so at most the declared
number of accepted TLS handshake futures can exist through this endpoint even
when callers poll several `accept` calls concurrently. The endpoint contains
no accept loop, task, runtime, clock, retry, or implicit timeout. Caller policy
must supply handshake/drain deadlines and owns certificate issuance,
revocation, key storage, and rotation scheduling.

rustls 0.23 owns TLS record and handshake parsing. Its locked implementation
caps one buffered handshake message at 65,535 bytes and one certificate at
65,536 bytes. Pwright additionally calls `set_buffer_limit` on both client and
server connections with the larger of 65,536 bytes and the configured
length-prefixed Pwright envelope. These TLS allocations are documented as a
third-party boundary rather than counted as exact `WireBytes`; after
authentication, the existing stream codec and shared ledger govern every
Pwright frame. Duplex adversarial tests and an opt-in live Tailnet-interface
test prove that TLS consumes exactly its own handshake bytes and preserves the
following ClientHello frame boundary.

`pwright-composition-chromium-tokio` is the optional leaf that joins these
authenticated endpoints to the Chromium composition without adding them to the
base crate. An already-accepted Unix, Tailnet, or mTLS connection yields its raw
Tokio transport and proof-carrying gate as one consuming value. The adapter
claims the transport's exact production plan from the fresh Chromium ledger,
keeps the gate paired through runtime installation, and consumes it during the
same supervisor's handshake. `ChromiumEndpointServer` may own a caller-created,
pre-bound endpoint, but it neither chooses nor binds that listener and never
starts an accept loop. Each explicit `accept_connection` awaits exactly one
endpoint-authenticated peer before opening its fresh CDP attachment. The pure
`production_resource_requirements_for_codec` function lets deployment profiles
declare that transport cost before a socket exists. Rejection-order tests prove
endpoint authentication failure precedes CDP I/O. A dedicated Chrome test
carries a kernel-derived Unix UID through authorization from the packaged
listener, then drops the connection and reattaches to prove that this server
path never acquires authority to terminate externally owned Chrome.

### Optional JSON codec proof slice

A separate `pwright-codec-json` crate may prove codec plugability using Serde
JSON without making JSON a core protocol dependency. Generated or application
message types implement `RoutedMessage`; the codec serializes their payload and
derives the outer `FrameRoute` independently. Decode must reject malformed JSON
and a decoded route that differs from the trusted outer route.

This first JSON codec deliberately rejects binary attachments instead of
base64-encoding them. Attachment-aware schemas and transports require a later
explicit mapping. The JSON representation is a proof vehicle, not yet the
stable Pwright IDL or Playwright wire-compatibility promise.

### Shared binary payload contract slice

`pwright-codec-binary` is a standard-library-oriented primitive layer shared by
canonical binary capability codecs. It owns allocation-aware network-order
integer writing, borrowed bounded reading, exact truncation/trailing-byte
diagnostics, checked host-size conversion, and static positional schema
metadata. The schema records stable IDs, versions, ordered variants, ordered
fields, field kinds, and cardinality; validation rejects malformed names,
duplicate fields, and noncanonical variant tags. A deterministic layout
fingerprint excludes the semantic minor version. Within one major version, the
compatibility checker permits appending new higher-tagged variants but forbids
modifying, reordering, or removing an existing positional variant.

This began as a contract primitive beneath generated IDL.
`pwright-codec-control`,
`pwright-codec-handshake`, `pwright-codec-navigation`, and
`pwright-codec-locator` now use the shared reader/writer and publish their
top-level layout schemas; control, hello, navigation, and locator golden bytes
remain stable. Navigation response decoding also preflights the minimum header
bytes and reserves fallibly before constructing entries. Nested `Message`
fields now carry stable referenced type IDs. `BinaryRecordSchema`, tagged-union
schemas, and a canonically ordered `BinarySchemaCatalog` distinguish the two
shapes, validate recursive references without recursion during validation, and
reject missing referenced types. Navigation now publishes a complete generated
catalog and checked-in source baseline. Locator now also publishes its complete
generated catalog and checked-in 1.12 source baseline. Control now publishes a
generated catalog and checked-in 1.0 protocol-family baseline. Handshake now
publishes the same generated artifacts and a checked-in 1.0 baseline. The new
`pwright-codec-common` source library
owns call metadata, duration, and failure-kind types. Navigation imports and
reexports those exact generated models and schema constants, and its catalog
is composed with the common entries. Unknown-field evolution and
cross-language fixtures remain DT-031 work.

### Capability-owned source IDL slice

The first generator checkpoint selects a strict subset of Playwright-shaped
YAML as a provisional, language-neutral source. Each capability owns its source
and committed generated output. `pwright-schema-gen` is a workspace development
tool, not a dependency of a generated capability; generation is an explicit
command rather than a `build.rs`, so installing one client capability does not
compile YAML, Serde, a generator, or Node.

The generator emits Rust wire models, bounded fallible value encoders/decoders,
a complete static schema catalog, an optional capability-specific typed server
chain, and a deterministic human-readable protocol reference; all requested
outputs share one non-mutating drift check.
`pwright-codec-navigation` is the first representative migration: its twelve
enum/record/union types now come from
`pwright-cap-navigation/schema/navigation.yml`. Its production
commands and results use the generated bounded codec while the published golden
bytes remain unchanged. Catalog compatibility checks all old type IDs by
identity rather than array position, permits new types and appended union tags,
and rejects record changes, removal, retyping, nested-reference changes, and
version regression. A checked-in navigation 1.0 source baseline and the
generator's `--previous` mode enforce the same compatibility policy before
output is accepted.

Locator is the second capability-owned migration. Its schema preserves the
published Click/Fill IDs, methods, root tags, field order, and golden bytes,
then appends Count, state, wait, text-content, inner-text, inner-HTML,
attribute, form-value, collection reads, and hover through 1.17. It generates the core-only
capability metadata, recursive selector wire tree, complete catalog, protocol
reference, and optional static server chain.
Recursive message edges become `Box` values and a cycle without a finite wire
value is invalid. Locator's validated options, params, typed errors, and unit
result are declared `public_external`: their construction policy stays
hand-written while their stable names and wire mappings remain schema-owned.
Immutable selector and matcher values are now owned by the zero-dependency,
command-free `pwright-selector-model` crate and reexported unchanged by
`pwright-cap-locator`. The corresponding nine-type recursive wire closure is
published independently as the command-free `pwright.selector` 1.0 schema
library and generated `pwright-codec-selector` crate. Source-level parity tests
prove that every definition exactly matches both existing Locator majors;
consumer-generation tests prove it imports the selector types without importing
Locator commands. Existing 1.x/2.x schemas retain their published inline copies
until a future major can switch imports without violating compatibility.

Non-capability top-level traffic uses the separate `protocol` source identity.
It requires a tagged-union root but cannot declare capability commands or
capability dependencies. Control is the first migration: its five lifecycle,
cancellation, and subscription variants are generated from
`pwright-codec-control/schema/control.yml`, while reviewed conversions preserve
the semantic `ControlMessage` types and canonical absent-parent rule.

Handshake is the second protocol-family migration. Its ClientHello root uses a
split header: both capability-list counts precede both element segments. A
repeated field may therefore declare `counted_by` to reference an earlier
required `u32` field. Validation rejects missing, later, non-`u32`, optional,
or non-repeated uses; the relationship is part of catalog identity, layout
fingerprints, generated references, and exact minor compatibility. Generated
owned and borrowed encoders verify that the declared count equals the supplied
sequence length and omit the ordinary adjacent length prefix. Generated decode
uses the earlier count to preflight minimum bytes and reserve fallibly. This
preserves the established count/count/items/items bytes rather than
approximating them with ordinary repeated fields.

`pwright-codec-handshake/schema/handshake.yml` and its checked-in 1.0 baseline
now own all seven handshake wire types. Production encoding uses borrowed
sequence adapters over the already-canonical semantic capability maps and
slices, so it does not clone IDs or build an intermediate wire tree. Decode
uses the generated owned root, then performs policy-bearing conversions:
capability-ID validation, canonical order, version-range validation, the
combined required-plus-optional negotiation ceiling, and semantic set
uniqueness. Legacy minimum-only ClientHello remains accepted. Empty and
populated ClientHello/ServerHello golden fixtures prove exact byte preservation,
while the normal dependency graph remains binary codec, core, and transport
only.

Generated codecs share an explicit value context. Callers choose hard limits
for individual strings and binary values, repeated-item counts, nesting depth,
and total decoded/encoded message nodes. Declared lengths are checked before
copying, allocations use fallible reservation, and repeated values preflight a
schema-derived minimum byte count before reserving storage. A generated goto
fixture matches the established navigation payload byte for byte.

The grammar and binary catalog distinguish strict booleans from integers and
model enums with stable numeric tags. Enum decoding rejects unknown tags, and
minor compatibility permits only append-only higher tags. An enum may select
one declared variant as its semantic default; public and wire Rust models then
derive the same `Default`, and changing that selection is minor-incompatible.
Navigation uses this for `WaitUntil::Load`. Navigation converts
between its public command/result types and these generated wire models,
including responses and typed errors. Duration nanoseconds, aggregate response
header bytes, and canonical header order were initially capability conversion
checks; duration nanoseconds have since moved into generated numeric constraints.
Types that transitively contain strings, bytes, or repeated values also receive
generated `*Ref<'a>` encoding models. Generated code owns count/limit and item
layout encoding while an object-safe borrowed sequence supplies repeated
values. Navigation uses those views directly; its canonical header adapter
orders even duplicate headers without allocating or cloning an intermediate
wire-model vector. Decode still returns owned models at the transport ownership
boundary.

Integer fields can carry inclusive source-level ranges spanning the full signed
and unsigned wire domain. Schema and source validation reject ranges on
non-integers, empty/reversed ranges, and bounds outside the declared integer
kind. Generated owned/borrowed encoders and decoders enforce the same range;
the catalog fingerprints it and minor compatibility treats any change as a
contract change. Navigation duration nanoseconds use this path, so invalid
values fail before construction of `std::time::Duration` rather than reaching a
hand-written semantic branch.

UTF-8/byte fields can also declare inclusive byte-length ranges, and repeated
fields can declare item-count ranges. Generated encoders enforce both before
writing; decoders enforce them before copying strings/bytes or reserving
vectors. These semantic ranges are catalog metadata and compatibility
identity, while caller-selected `BinaryValueLimits` remain a separate hard
resource ceiling. Repeated primitive element-length rules are deferred until
the borrowed sequence boundary can enforce each opaque element.

Capability source now owns binary command metadata as well: stable command ID,
method tag, and the command/result tags in its root union. Generated
`BinaryCommandSchema` constants are minor-version compatibility identity.
Navigation asserts them against the typed `BeginNavigation` and `Goto`
contracts, closing silent drift between wire tags and the installed capability.
The same source now declares each command's semantic params, result, and typed
error models. The capability output owns their public Rust types, typed marker
structs, and `CommandSpec`/`CapabilityCommandSpec` implementations; the codec
output owns wire models and bidirectional owned conversions. Navigation's root
union embeds the generated params/results as inline positional messages, so the
semantic boundary is explicit without changing the established bytes.

The first ownership migration is now concrete. Navigation's canonical schema
and baseline live under `pwright-cap-navigation/schema`; one generator run
emits the binary binding into the codec crate and core-only identity, method,
command-list, and `CapabilityDescriptor` code into the capability crate. The
formerly hand-written metadata was removed. `cargo tree --edges normal`
confirms the portable capability still depends only on `pwright-core`. Its
remaining hand-written module is intentionally limited to Rust ergonomics such
as constructors/defaults. The navigation codec keeps borrowed zero-copy
adapters and policy validation, but mechanical owned mappings are generated.

Capability dependencies now use the same source without being confused with
codec imports. A dependency is a capability ID plus an exact minimum protocol
version emitted into `CapabilityDescriptor`; libraries cannot declare one and
a capability cannot depend on itself. The list is canonical and any change is
minor-incompatible, because adding, removing, or raising a requirement can
change what an existing installation can compose.

This is not yet a complete generated protocol. Recursive Rust model boxing,
transitive library imports, declarative cross-field constraints, dispatch
tables, user-facing generated documentation, generated lifecycle/object
references, and a second language remain open. Command object targets are now
schema-owned compatibility identity and are emitted on every generated
`CommandSpec`. The capability output also exports an allocation-free ordered
command descriptor table containing ID, method, and targets; the generic server
inventory exposes validated lookups without introducing a global registry.
The option analysis and exact commands are in
`knowledge/pwright-protocol-idl.md`.

### Codec-neutral handshake and capability negotiation slice

Remote peers negotiate before object creation or browser interaction. A client
hello advertises one supported major/minor range, its inbound frame limits, and
bounded required and optional capability ranges. Server configuration supplies
its own version range, inbound frame limits, and bounded installed capability
ranges.

Negotiation selects the highest minor in each common major-version interval,
both for the connection protocol and independently for every capability.
Frame limits remain directional: client sends obey server receive limits and
server sends obey client receive limits. The server enables only capabilities
the client explicitly requested. A missing or incompatible required capability
rejects the handshake; an unavailable optional capability is omitted. Results
are ordered by capability ID for deterministic diagnostics.

`WireProtocol` represents that directionality directly. Its backward-compatible
constructor applies one limit in both directions, while `new_directional` uses
a peer receive limit for encoding and a local receive limit for decoding.
`new_exact_directional` additionally pins all application frames to the single
version selected by the handshake.

The compatibility `CapabilityManifest::new` constructor treats each historical
single installed version as support from minor zero through that version;
`from_support` expresses an exact range. Likewise the minimum-only request
constructor means that minimum through the end of its major, while `within`
expresses both bounds. Wire capability IDs are owned strings rather than core's
compile-time `&'static str` identifiers. Their byte length, character validity,
total count, and uniqueness are validated at construction and revalidated when
inserted in a manifest or request. A manifest may now contain several distinct
major tracks for the same logical capability ID. Duplicate entries within one
ID and major remain invalid. A request still names exactly one same-major
interval per capability, and negotiation looks up only that major; it never
falls across a wire-incompatible major boundary. The server hello contains at
most one exact selected version per capability ID, so every accepted connection
has one unambiguous codec track. `support(id)` reports the newest installed
track for inventory UI, while protocol selection uses
`support_major(id, major)`.

This slice does not define authentication, authorization, resume tokens,
version-selected runtime dispatch, or runtime plugin loading.

`pwright-codec-handshake` supplies the concrete Serde-free hello encoding. New
ClientHello values carry both capability bounds; the decoder continues to
accept the legacy minimum-only tag and expands it to the documented compatible
range. ServerHello keeps an exact selected version for each enabled capability. It
uses fixed-width network-order integers, u64 frame limits with checked host
conversion, u32 counts and string lengths, and separate Client/Server tags.
Capability counts and ID lengths are rejected before entry allocation or string
copying. Encoders emit sorted IDs; decoders reject noncanonical order,
duplicates, invalid UTF-8/IDs, invalid version ranges, truncation, trailing
bytes, attachments, wrong routes, and wrong hello direction. The same codec is
usable by both peers without becoming a dependency of `pwright-transport`.

### Typed client-handle foundation slice

The first client slice adds lightweight `pwright-client`, depending only on
`pwright-core` and the core-only `pwright-browser-model` vocabulary.
`Client` strongly owns a typed channel and an injected call-ID source. Remote
`Page` handles contain their typed `ObjectId` plus weak references to those
owners, so stale handles cannot keep an abandoned connection alive. Handle
equality requires both the same object ID and the same connection allocation.

Calls continue through `Service<Command<Spec>>`; an in-process runtime therefore
receives typed values directly without JSON or a duplicate object graph. Call
IDs come from an injected source. The provided atomic source has explicit
exhaustion and no random or process-global state.

The opt-in navigation extension exposes only the backend primitive as
`PageNavigationExt::begin_navigation`. It must not name that operation `goto`:
commit/load waiting, redirect and main-resource `Response` identity,
same-document rules, downloads, nested deadline budgets, and page/frame close
races remain required before a Playwright-shaped `Page::goto` is truthful.
Remote-handle `Drop` sends no lifecycle command. The later Handle-lifecycle
foundation adds explicit async disposal for `JsHandle` and `ElementHandle`
without changing this rule; complete runtime integration is described below.

A later navigation checkpoint adds that distinct high-level operation rather
than renaming the primitive. `PageGotoExt::goto` dispatches `Goto`, whose
portable parameters include URL, optional referrer, and Playwright's four wait
states. Timeout and cancellation remain `CallMetadata`, so nested layers share
one deadline instead of resetting per CDP step. See the dedicated goto slice
below for its current response and backend semantics.

### Typed object-topology slice

The implemented client slice introduces typed `Browser`, `BrowserContext`,
`Page`, `Frame`, `JsHandle`, and `ElementHandle` handles without adding a
client-side mirror of the runtime
ownership graph. Handles are constructed from a codec-neutral remote initializer that
contains an object ID, optional parent ID, and object-kind tag. Each typed
factory validates its expected kind and exact parent before producing a child:
a browser is a root, a context belongs to that browser, a page belongs to that
context, a main frame belongs to its page, a child frame belongs to its parent
frame, and both Handle kinds belong to their creating Frame. Construction also
refuses an already-disconnected connection.
Parent ownership, adoption, and child-first disposal remain authoritative in
the runtime `ObjectGraph`; the client must not duplicate that state merely to
make in-process dispatch look remote. A later client liveness table rejects a
duplicate live initializer as invalid protocol input without storing parents.

### Stable object-kind vocabulary and command admission

Object identity separates stable semantic names from compact lifecycle tags.
`pwright-core` owns only generic `ObjectKindId`, descriptor, catalog, and marker
abstractions. The core-only `pwright-browser-model` crate owns the stable
Playwright-shaped Browser, BrowserContext, Page, Frame, Handle,
Request/Response, and Artifact/Stream IDs and tags; the client reexports marker
types only when it exposes the corresponding handle. The vocabulary is not a
negotiated capability, allocates no runtime object by itself, and cannot
acquire, launch, bundle, or download a browser.

Capability schemas declare the semantic object kinds accepted by each remote
command. The generator emits those targets on `CommandSpec`, and changing the
target list is minor-incompatible. A remote server inventory rejects commands
with no target metadata. When configured with the static catalog, direct and
controlled dispatchers reject unknown lifecycle tags at registration and
reject a command/object mismatch before admission, scheduling, or backend
invocation. An explicit `Partial` topology mode permits a known externally
attached object to be represented as a local root but still validates every
present parent; `Complete` requires the Browser→BrowserContext→Page→Frame
chain. Create and Adopt share this validation. Server handshake configuration
can preflight the composed handler inventory against the installed catalog, so
a capability cannot be advertised when one of its command target kinds is
unavailable. Adding a vocabulary entry does not install its lifecycle or
commands. The detailed decision is recorded in
`knowledge/pwright-object-model.md`.

`Locator` is introduced only as a generic client-side query value scoped to a
typed `Page` or `Frame`. It has no `ObjectId`, sends no lifecycle traffic, and
does not consume runtime object capacity. Its query representation is a type
parameter owned by a selector capability, so the base client does not hard-code
CSS, accessibility, text, or backend selector syntax. Mapping a query is purely
local; combining two locator queries requires the same scope and connection.
The concrete selector/action checkpoints add `pwright-cap-locator` and
`pwright-client-locator` without changing this storage type. The capability
owns validated CSS, string/ECMAScript-regex text, label, placeholder, alt-text,
title, test-ID, and typed ARIA-role leaves plus lazy `nth`, text-filter,
intersection, union, and descendant-chain nodes plus independently
dispatchable typed `Click` and `Fill` commands. `LocatorFactoryExt` adds optional Playwright-shaped factories
to `Page` and `Frame`; `LocatorCompositionExt` performs `nth`, filters,
`and`/`or`, and chained `get_by_*` construction locally. The click and fill
extensions accept any query value convertible into the selector AST, while
applications that do not install locator actions retain the generic base
value.

`TextMatcher` gives text, label, and accessibility string branches one portable
contract: whitespace is normalized; exact matching is case-sensitive and
whole-string; non-exact matching is a case-insensitive substring. User-facing
attribute leaves carry the same value/exact shape but intentionally operate on
raw attribute values without whitespace normalization. Test-ID strings are
always exact, matching Playwright, while regexes operate on the raw value.
`TestIdAttributes` defaults to `data-testid` and embeds an immutable, validated
list of at most 16 names for explicit configuration. `RegexMatcher` separately carries an
ECMAScript source and the canonical `dgimsuvy` flag string. Duplicate,
out-of-order, unknown, and simultaneous `u`/`v` flags fail locally or during
decode; the browser compiles the source because ECMAScript syntax must not be
silently replaced by Rust-regex semantics. Regex matching uses the complete
text without string-mode whitespace normalization, matching Playwright's
`RegExp` branch; `exact` therefore does not apply. A browser syntax failure is
returned as typed `InvalidSelector`. `Within { base, descendant }`
evaluates the descendant expression independently below every base match,
deduplicates the result, and preserves composed traversal order. This differs
from `And`, which evaluates both operands from the same root and intersects by
element identity. `AriaRole` is a closed typed set matching the roles exposed
by Playwright. `RoleOptions` covers name, description, checked, disabled,
expanded, hidden inclusion, level, pressed, and selected; invalid role/state
combinations fail while constructing or decoding the selector.

The CDP click adapter re-resolves the selector on every attempt and enforces
strict single-element resolution. It retries missing, detached, invisible,
unstable, disabled, and hit-target-obscured states. Stability is checked across
two animation frames after scrolling when the document is visible. Chromium
suspends animation-frame callbacks in background Pages, so a hidden document
uses a zero-delay timer yield instead of waiting forever; the foreground path
keeps the two-frame stability check. The click point is clipped to the visible
viewport and verified with `elementFromPoint`. Input is emitted as a matching
mouse pressed/released pair. Portable options currently include button, nonzero
click count, `force`, and Playwright-style `trial`. The runtime owns the single
deadline and cancellation budget; dropping the action future prevents later
input dispatch and requires no persistent CDP cleanup operation.

Richer action option bags now have a proven codec building block, separate from
the still-published positional locator records. An extensible record is one
bounded length-delimited envelope containing canonically ordered, nonzero
`u16` field tags and independently length-delimited values. Decoders borrow
each field body, require exact consumption for known tags, and discard unknown
tags without allocation. Encoders reserve and backpatch lengths directly in
the final buffer. The schema catalog treats this as a distinct type kind with
a fixed encoded-size range: minor releases retain every existing tag exactly
and may add only optional tags. Generator YAML expresses the choice explicitly
with `encoding: extensible`, `encoded_size`, and per-field `tag`; positional
records reject those properties. This prototype does not reinterpret any
locator 1.x bytes. Click migration still requires an explicit locator-major
and negotiation plan.

### Locator Click major transition

The locator option-bag transition uses the same logical capability ID,
`pwright.locator`, with independently installed 1.x and 2.x tracks. It does not
rename the capability and does not accumulate `click_v2`, `click_v3`, and later
parallel RPCs. A v1 client requests only locator 1.x and keeps the published
positional bytes. A v2 client requests only locator 2.x and receives the
extensible Click option bag. A server installation may carry both generated
artifacts, but one connection activates exactly one locator major.

The reusable negotiation foundation supports this shape and proves both that
a client selects the requested major rather than the newest installed major
and that a server hello cannot enable two majors for one capability. The
Chromium composition now assembles its advertised manifest from separate 1.x
and 2.x inventories and crosses a version-selected handler boundary only after
the handshake has been accepted. The boundary enforces these invariants:

1. The accepted hello selects the locator codec and handler table before any
   application frame is decoded.
2. Command IDs and method tags may be reused across locator majors because
   only the selected table is installed for that connection; they may not be
   registered twice in one unversioned table.
3. The 1.x and 2.x codec crates have independent golden fixtures and never
   decode one another's Click payload.
4. Removing or disabling the 2.x track is a server configuration rollback;
   locator 1.x negotiation and bytes remain unchanged.
5. A v2 request against a v1-only server fails during the handshake, before
   object creation or browser I/O.

An alias capability or connection-wide hidden flag was rejected because either
fragments the public capability identity or changes Click decoding without the
owning capability version expressing that incompatibility. Advertising v2
through today's fixed v1 handler was also rejected: negotiation truth must
follow an actually installed decoder and service table.

The locator 2.0 through 2.4 baselines and current 2.5 artifacts exist beside,
rather than over, the locator 1.21 baseline and current 1.22 artifacts. Their Click option bag has ten optional tags
matching
Playwright's current portable surface: button, click count, delay, force,
modifiers, deprecated navigation-wait opt-out, relative position, scroll mode,
mouse-move steps, and trial. Timeout and cancellation remain in shared call
metadata. The empty four-byte record means all defaults; the record body is
bounded at 128 bytes and pointer movement is capped at 10,000 sequential
events. Release fixtures prove all ten tags are canonical,
unknown tag 11 is skipped, both catalogs validate, and 1.x remains a positional
record through version 1.22 while 2.x keeps the extensible record introduced
at version 2.0.

Generated 2.x public command markers, server bindings, `LocatorV2Codec`, and a
same-named client `click` extension now execute through the real remote
client/server stream. The CDP service implements absent defaults, force,
relative position, scroll suppression, temporary modifiers, trial, delayed
multi-click pairs, bounded interpolated mouse movement, cancellation cleanup,
and Chromium's `Page.enable` ordering barrier around requested main-frame
navigation commits. `LocatorInstallationProfile::with_v2_track` advertises the
2.5 track beside the independently evolving 1.22 track; the default profile remains v1-only.
After acceptance, `remote_selected_locator_handlers` constructs only the exact
1.x or 2.x codec and handler tree selected by `ServerHello`. The legacy
`remote_locator_handlers` entry point refuses a 2.x connection rather than
decoding it with the v1 codec. A v1-only profile has the same resource
requirements and one-track manifest as before, so disabling v2 is a real
configuration rollback.

The canonical all-capabilities server recipe uses the same selected-major
handler type and installer entry point; it no longer hard-codes the v1-only
builder. Its live external-Chrome conformance case is compiled for a locator
2.x request while the default all-capabilities profile remains locator 1.x,
so recipe adoption is opt-in and reversible too.

Generated multi-command server bindings use balanced binary handler trees and
split capabilities above sixteen commands into two typed groups. The prior
left-deep locator tree caused extreme compiler type and optimization growth
when both majors were callable through one selected boundary. Grouping and
balancing preserve command order, routing, wire bytes, runtime allocation, and
the one-handler-per-command composition model. A production test now performs
an accepted locator 2.5
handshake, invokes the selected outer handler, validates the runtime Page
object, routes through `CdpPageServices`, and observes v2 Click, Focus, Blur,
Press, SetChecked, SelectOption, and ScrollIntoViewIfNeeded as CDP input or
utility-world events. The selected 2.x codec receives the exact
negotiated capability minor, so a 2.0 peer cannot invoke the 2.1 focus routes.
The outer frame protocol version remains an independent concern.

### Page screenshot capability slice

`pwright.content` 1.0 is a separate, optional capability whose first command is
Page-scoped `page.screenshot` (`0x0005_0001`). Its portable contract, generated
wire model, client extension, generated server binding, and Chromium adapter
live in separate crates. Selecting locator, navigation, evaluation, or Page
events does not select screenshot capture. The canonical all-capabilities
recipe opts it in explicitly and uses one shared frame envelope that permits a
single binary attachment; capability-specific payloads remain independently
encoded and negotiated.

The 1.0 surface contains PNG/JPEG/WebP, JPEG/WebP quality, viewport/full-page
selection, an optional finite clip, and CSS/device scale. It intentionally
does not accept path, mask, temporary style, animation, caret, background
mutation, or Locator screenshot options. Paths belong to the client machine;
the rendering mutations require a later minor with a cancellation-safe
reversible transaction; Locator capture must compose locator resolution and
geometry instead of embedding a selector into the Page command.

Chromium obtains layout metrics only when full-page, clip, or CSS scale needs
them, invokes typed `Page.captureScreenshot`, rejects oversized base64 output
before decoding, and returns owned decoded bytes. One capture may be active per
Page and a configured connection-wide maximum is enforced without a waiter
queue. The adapter reserves the maximum `ScreenshotBytes` before decode, then
atomically shrinks that lease to the exact retained result. Dropping a pending
future releases Page admission and bytes even though CDP exposes no local
per-command screenshot cancellation primitive.

The wire result contains one attachment plus control metadata for attachment
index, exact length, and effective format. The client decoder validates all
three before copying into its own `ScreenshotBytes` lease. Production tests
cover direct Page routing, accepted-handshake remote transfer through the
generated handler, exact client result retention, oversize rejection, and
drop/retry cleanup. A dedicated-profile live test also creates an isolated
Chromium Context, captures a real full-page CSS-scale PNG, validates the PNG
dimensions and exact lease release, disposes the Context, and proves the
externally owned browser remains usable.

The screenshot values and result ownership now live in the command-free
`pwright-screenshot-model` crate. It depends only on `pwright-core` and owns no
command, protocol, transport, runtime, backend, filesystem, or browser
authority. `pwright-cap-content` compatibility-reexports those exact types, so
this extraction changes neither the 1.0 wire catalog nor downstream source
types. Future Page, ElementHandle, and Locator capture capabilities can share
one retained-byte type without depending on one another.

Locator screenshot is a separate `pwright.locator_screenshot` 1.0 capability
rather than a new route in `pwright.locator` or `pwright.content`. Its schema,
frozen baseline, portable contract, bounded codec, typed Locator extension, and
generated server binding are implemented at method `0x0009_0001`. The command
targets the Locator's Page or Frame scope, imports the command-free
`pwright.selector` wire library, and carries one selector plus only
format/quality/scale options. Page-only clip and full-page fields are rejected
before encoding rather than ignored. A successful result uses one attachment
and a separately admitted `ScreenshotBytes` lease; the standalone generated
server manifest advertises neither ordinary Locator commands nor Page capture.
Both capture codecs delegate attachment index/count/length/limit validation and
the exact retained result copy to command-free `pwright-codec-screenshot`;
their command roots, options, errors, and negotiation remain independent.

The complete Chromium composition selects Locator screenshot with its own
`LocatorScreenshotInstallationProfile`. It declares selector and Frame-route
resources for its chosen full-Locator resolver plus capture, codec, and wire
resources, registers only
`pwright.locator_screenshot`, and constructs its own generated local or
post-handshake handler. Page objects alias their main Frame context; Frame
routes retain the exact execution-world identity while every Frame belonging
to a Page shares that Page's capture admission. Ordinary Locator and
`page.screenshot` remain absent unless independently selected.

The smaller `pwright-composition-chromium-locator-screenshot` leaf does not
choose a selector engine. It accepts an already bound geometry resolver and
owns only capture/result/wire resources; the application must combine the
resolver plugin's requirements concurrently before creating the shared ledger.
`pwright-backend-cdp-selector-geometry` is the bounded first-party choice. Its
profile separately declares concurrent `SelectorBytes` and optional persistent
Frame utility-world `BackendStateBytes`; alternate resolvers remain valid
because the generic composition does not depend on this implementation.

Child-Frame viewport rectangles are translated before top-level Page capture,
following Playwright's Chromium strategy. For each bounded topology ancestor,
the backend sends `DOM.getFrameOwner` on the parent renderer session, reads the
owner's native border box, and adds its origin to the Frame-local element clip.
Translation nodes form a shared linked chain, so nested routes retain one node
per Frame rather than duplicating every ancestry path. Selector resolution
remains on the exact child execution world, including OOPIF sessions, while
the final screenshot admission and capture remain Page-scoped. Local child and
nested OOPIF unit proofs verify exact coordinates and command ownership; a
dedicated Chrome same-process iframe proof verifies an exact 70x50 result and
cleanup. Cross-site OOPIF live and pixel-content checks remain conformance
follow-ups.

Unlike Playwright's JavaScript client-side temporary-handle sequence, its Rust
backend must keep
strict resolution, actionability, scrolling, geometry, capture, and cleanup in
one server command: dropping a Rust future cannot asynchronously execute a
client-side `finally` to dispose a published temporary handle. The detailed
boundary and acceptance gates are recorded in
[`pwright-locator-screenshot.md`](knowledge/pwright-locator-screenshot.md).
The first backend does this without publishing a DOM handle: it strictly
resolves one connected visible Element, waits across two animation frames,
scrolls only when the intersection ratio is not one, re-resolves, computes an
enclosing integer viewport clip, and delegates to the Page screenshot
coordinator. RAII releases capture and selector admission on success, error,
cancellation, or future drop. Local, negotiated-remote, and dedicated-Chrome
tests cover this transaction and retained-byte lifetime.

### Page and Frame DOM-content design

`Page.content()` is a client facade over a Frame-scoped operation, following
Playwright rather than creating a second Page wire command. The Page resolves
its initialized main Frame locally; direct child and OOPIF calls retain their
own Frame identities. The exact upstream semantics and proof matrix are
recorded in [pwright-page-content.md](knowledge/pwright-page-content.md).

DOM serialization is a separate opt-in `pwright.dom_content` 1.0
capability, not as an addition to `pwright.content` screenshot or
`pwright.evaluation`. This keeps read-only DOM access independent from pixels
and from arbitrary JavaScript authority. Chromium executes one fixed
library-owned serializer in the exact Frame utility world, with no retry across
navigation.

The result retains a dedicated `DomContentBytes` lease in a non-cloneable
UTF-8 wrapper and travels remotely as one validated UTF-8 attachment. The
Chromium composition owns one Frame-context route entry whose context identity
is shared by locator and DOM-content services. When both are installed, DOM
content owns the single utility-route reservation and locator retains only its
action budget, so target/session/context storage is neither copied nor planned
twice. Direct typed dispatch, negotiated server installation, and the canonical
all-capabilities recipe use the same route. A dedicated Chrome 152 proof reads
doctype-plus-DOM from the main Frame and normalized DOM from a `srcdoc` child,
cleans up only its temporary target, and proves the externally owned browser
remains reachable. The live matrix also covers HTML4, no doctype, a source-less
iframe's normalized empty document, and stale-to-replacement utility-context
navigation. Known topology crash state is copied only as a boolean into the
shared route, allowing `Frame.content` to return `BrowserCrashed` before CDP
without retaining another crash-status string.

Topology synchronization also translates a `PageCrashed` effect into a generic
controlled-runtime subtree interruption. Unlike disposal, this keeps the Page
and Frame graph live while waking both queued and active commands with a typed
`BrowserCrashed` result; the backend future is dropped and its ordinary cleanup
hook receives `ParentClosed`. The signal is scoped to the crashed Page subtree,
so another Page on the same connection is unaffected. Frame disposal continues
to use the same runtime race with `TargetClosed`. Direct runtimes cannot race an
in-flight future, so they reject a crash synchronization while that Page
subtree is busy.

Remote DOM-content handlers are constructed only when `pwright.dom_content`
appears in the accepted handshake. Merely offering the capability in the
server profile is insufficient. An omitted capability therefore has no command
entrypoint and cannot reach the CDP adapter; a production-composition test
asserts both the absent handler and zero fake-CDP calls.

The CDP fill adapter uses the same lazy resolution, strictness, deadline,
cancellation, and selector-runtime boundary. Unless `force` is set, it retries
until the target is visible, enabled, and editable. A selected `label` is
retargeted to its associated control. Text-like inputs, textareas, and
contenteditable elements are focused, selected, and filled through CDP input;
an empty value dispatches Delete after selection. Constrained input types such
as date, time, month, range, and color use the native value setter, validate the
browser-accepted value, and emit `input` plus `change`. Unsupported input types,
non-editable element kinds, and malformed constrained values are terminal typed
`InvalidTarget` or `InvalidValue` failures. Direct calls reject fill values over
1 MiB before JavaScript evaluation; encoded calls also obey the configured
wire string limit.

The semantic selector runtime is installed lazily into the current page
execution context with one `Runtime.evaluate`; cloned services share a
`OnceCell`, so concurrent first actions perform one installation. Actionability
retries then send only the compact selector/action script rather than embedding
and recompiling the complete role/text/label implementation every time. A full
document navigation naturally discards that global. The compact script returns
an explicit `runtime-missing` state, causing an idempotent reinstall in the new
context before retrying. Pwright intentionally does not call
`Page.addScriptToEvaluateOnNewDocument`, so attaching to an externally owned
page does not leave a persistent new-document script registration behind.
`SelectorRuntimeMetrics` exposes only three fixed counters—successful
installations, compact actionability evaluations, and post-navigation misses;
it retains no scripts, calls, or DOM identities.

The CDP compiler preserves ordered-set behavior: `and` intersects by element
identity, `or` deduplicates then restores traversal order, negative `nth`
indexes from the end, and text filters use the same matching contract as text
leaves. Regex instances are constructed once per selector evaluation, so flags
such as `g` and `y` preserve JavaScript `RegExp.test` state exactly within that
evaluation. CSS and semantic leaves traverse open shadow roots. Text locators keep
the smallest matching element; label locators inspect native labels,
`aria-label`, and `aria-labelledby`; role locators recognize explicit and
common implicit roles, accessible names/descriptions, hidden state, and every
portable role-state filter. The compiler rejects more than 32 levels, 256
nodes, or 65,536 aggregate selector-string bytes before evaluating JavaScript.
A real-Chrome test exercises page factories plus chained dialog/button role,
positive/negative regex filters, regex role/name matching, and browser-side
invalid-regex classification, then performs a full navigation and uses the
same service for regex label and text clicks in the replacement execution
context. It also proves that an
out-of-scope same-name button is not selected. The same test verifies fill
auto-waiting on a temporarily readonly control, label-to-control retargeting,
contenteditable input, and constrained date input with observable `input` and
`change` events.

This is deliberately not full DT-022/DT-023 completion. The full W3C
accessible-name and implicit-role algorithms, closed
shadow roots, cross-frame chaining, click positions/modifiers, delay, navigation
coupling, detailed timeout call logs, and the remaining action state machines
are still open. The CDP semantic runtime is an explicit tested subset rather
than a claim of complete accessibility-spec conformance.

`pwright-codec-locator` is the separate optional wire representation for this
slice. It canonically encodes the selector, click/fill/wait options, fill value,
bounded count, visibility, element-state, nullable bounded text-content,
bounded rendered inner-text, bounded serialized inner-HTML, nullable bounded
attribute results, bounded current form-control values, and bounded ordered
text-content collections,
timeout/cancellation metadata, success,
every portable failure kind, and each locator error detail.
The current compatible additions advance the independently negotiated locator
capability to 1.22; they do not change the common frame protocol. The preserved
1.21 baseline proves ScrollIntoViewIfNeeded method `131094` and root tags 43/44 are appended
rather than rewriting an established selector,
action, query, wait, content, attribute, collection, or result payload.
It recursively encodes every AST node and rejects oversized or empty selectors,
invalid UTF-8, unknown ARIA roles, illegal role/state combinations, malformed
tri-state values, zero ARIA levels, zero click counts, unknown
selector/option/button/error tags, noncanonical durations, attachments,
truncation, and trailing bytes. Depth and total-node limits apply during both
encode and decode. The payload repeats call and target identities; the typed
server verifies them against the outer route before invoking the backend.

The locator 2.x track independently freezes 2.0 through 2.4 and advances to 2.5
with the same ScrollIntoViewIfNeeded method and tags. This preserves v2 Click/Focus/Blur/Press/SetChecked/SelectOption bytes
and all 1.x bytes while making minor-version gating explicit at the connection-selected
`LocatorV2Codec` boundary.

ScrollIntoViewIfNeeded is one strict, lazy Locator command with timeout and
cancellation carried by call metadata. Its Chromium adapter resolves in the
utility world, requires an Element with layout, compares its rectangle across
two animation frames with a 0.25 CSS-pixel tolerance, and uses an
IntersectionObserver ratio to preserve the element's position when it is
already fully visible. A stable partial intersection advances to a separate
synchronous evaluation that re-resolves and commits one centered instant
scroll, so cancelling the asynchronous wait cannot cause a late mutation.
Missing, detached, layout-less, and unstable targets re-resolve;
strictness, invalid selectors, non-Elements, script errors, and malformed
results are terminal. The operation creates no DOM handle, input transaction,
navigation barrier, queue, task, or new resource class.

The codec's normal dependency tree is only `pwright-cap-locator`, the shared
common/binary codecs, `pwright-core`, and `pwright-transport`. Its integration
tests carry click, hover, focus, blur, press, scroll-into-view, set-checked, fill, wait, text-content, inner-text, inner-HTML, attribute,
input-value, select-option, all-text-contents, and all-inner-texts extensions through the
client channel, stream envelope, server handler, and bounded runtime, and
separately prove that a tampered outer call ID is rejected without reaching
the action service.

### Client object-liveness slice

The client requires a bounded liveness table, but not a second ownership graph.
It stores one small shared liveness token per initialized remote object, keyed
by `ObjectId`, without parent, backend, payload, or event state. Every handle
for that object shares the token. A codec or in-process control adapter applies
runtime dispose notifications to this table; removal is idempotent and marks
all existing clones stale before returning. Runtime child-first disposal is
applied as an ordered sequence of these notifications rather than recomputed
on the client.

The table has an explicit live-object limit and current/peak metrics. Duplicate
live initializers and capacity exhaustion are protocol/resource errors. Remote
method dispatch distinguishes an absent connection from a disposed target, so
the portable error is respectively `TransportClosed` or `TargetClosed`.
Neither handle nor client `Drop` emits protocol traffic. The opt-in
`pwright-client-handle-lifecycle` crate now supplies explicit async `dispose`
for `JsHandle` and `ElementHandle`; local notification application remains a
separate, synchronous channel concern.

### Explicit Handle-lifecycle foundation

`pwright.handle_lifecycle` version 1.0 is an independently installable
capability containing only `handle.dispose` (method `0x0008_0001`). It targets
the two stable Handle object kinds and depends on neither value evaluation nor
the Chromium backend. Its options and success result are unit, attachments are
forbidden, and timeout/cancellation metadata is rejected because disposal is a
lifecycle barrier rather than an action. Structured failures preserve
not-owned, backend, and compound-cleanup information.

The codec, client extension, generated concurrent server handler, and
capability model live in separate crates. Client disposal is idempotent for an
already-stale local handle and for a remote `TargetClosed` race, while other
failures remain visible. Runtime child-first invalidation remains authoritative:
the client does not infer descendants when only a parent ID is reported.

The Chromium backend foundation adds `Runtime.releaseObject` to the injected
CDP boundary and provides `CdpHandleDisposeService`. The service owns a bounded
native object ID, is bound to the exact CDP session supplied by composition,
sends one native release request, performs no hidden retry, and preserves CDP
failure classification. The dependent creation slice now consumes this leaf:
it atomically reserves the portable object, retains the native route under a
production resource budget, publishes Create before the command Result, and
makes disposal commit native release and runtime child-first Dispose. This
smaller checkpoint remains the independently installable lifecycle contract;
the creation slice below owns route and parent/topology cleanup.

### Handle-evaluation creation and disposal transaction

The next optional slice adds `pwright.handle_evaluation` 1.1, dependent on
`pwright.handle_lifecycle` 1.0 but independent of value-only evaluation. Its
single Frame-targeted `evaluation.evaluate_handle` command owns a bounded
recursive argument model that adds portable Handle leaves without changing the
value-only capability, and returns a portable `HandleInitializer`.
That initializer contains ObjectId, Frame parent, concrete JSHandle or
ElementHandle kind, and a bounded preview; native CDP identity is backend-only.
Page convenience delegates to its main Frame.

The Chromium service calls `Runtime.callFunctionOn` with `returnByValue: false`,
`awaitPromise: true`, and the exact routed `uniqueContextId`. Its bounded
by-value envelope carries source and serialized structure; resolved Handle
leaves become native CDP object or primitive call arguments. A node subtype
becomes ElementHandle; other object and primitive results become JSHandle.
Object-backed Handles retain their bounded native object ID in a
`ChromiumHandleRoutes` entry bound to the exact CDP client and source execution
world. Frame routes carry both the positive numeric `executionContextId` needed
by Chromium DOM adoption and the stable `uniqueContextId` used by Runtime
calls. Primitive Handles
retain only a bounded reconstructable call argument and release as an explicit
no-op.

After output ID allocation, resolution and output lifecycle preparation occur
atomically under one Handle-route lock before CDP I/O or publication. This
also excludes concurrent disposal. Generic JSHandles must belong to the target
Frame, Page, native CDP client, and still-current execution world. Cross-Frame use is a protocol/value
failure; missing or disposed identity is `TargetClosed`. A topology world
replacement retires old routes, and source-world comparison independently
rejects an old Handle before I/O even when the replacement uses the same CDP
session. Cross-world ElementHandle adoption is not implicit in the default
public commands. `install_with_element_adoption` explicitly composes its
independently budgeted backend resource and continuation owners. It permits
only a current ElementHandle from another Frame in the same Page and exact CDP
target session; JSHandles, stale worlds, different Pages, and different
sessions fail before adoption I/O. Protocol 1.1 adds only tagged-union
variant 18 and is checked against the frozen 1.0 schema. A codec bound to a
negotiated 1.0 capability accepts value-only requests but rejects Handle leaves.

Creation prepares the runtime child before CDP evaluation, then commits the
concrete Handle Create and native route synchronously. Failure after native
creation compensates with `Runtime.releaseObject`. Disposal first releases that
exact native object, then commits runtime disposal and removes its route; a
stale portable object remains idempotent. `OrderedEvaluateHandleCommandService`
provides the server-side Create-before-Result fence, and the typed remote client
refuses to materialize a Handle unless it can claim that exact prior Create.

The route table now also has an optional production profile. Construction must
claim its declared plan; every committed route retains one `RemoteHandle` unit
and its exact native-argument/preview bytes in the shared ledger. Its own metrics keep
current/peak counts and bytes plus explicit-dispose, parent-retirement,
topology-invalidation, and connection-close totals. Reservation precedes CDP
evaluation, and dropping any unpublished route reservation restores the ledger.

Explicit Page and owned BrowserContext close pass the runtime's prepared
child-first disposal order to the same route table. Native parent destruction
first makes every descendant remote object unreachable; runtime Dispose and
route/lease retirement then commit without redundant per-Handle CDP calls.
The same routes retain their portable Frame and Page owners. Main-world
replacement/removal and reconnect synthesize Handle disposal; Frame/Page/
Context topology removal contributes runtime-expanded descendants; Page crash
selects every route owned by that Page. All are committed with the prepared
topology, Page routes, and runtime lifecycle batch. The opt-in
`ChromiumHandleAwareRuntime` implements remote cancellation/transport-close
target traits and clears remaining Handle routes and leases whenever its inner
runtime drains the connection object graph.

`ContinuableEvaluateHandleCommandService` closes the direct native-evaluation
abandonment window without choosing an executor. It synchronously reserves one
caller-owned cleanup slot before its inner service can reach CDP. Dropping an
unpolled future only releases that reservation; dropping a started future
transfers the remaining evaluation. A successful orphan result is immediately
fed through `Handle.dispose`, because no user can ever receive its ObjectId.
Failed creation needs no second cleanup, and failed orphan disposal becomes an
observable cleanup-job failure rather than a silent leak.

The production root now exposes Handle lifecycle and Handle evaluation as two
independent optional installation profiles. Evaluation requires lifecycle, but
neither implies value evaluation. Compatibility runtime constructors retain
the older aggregate `ChromiumHandleRoutes`; the complete recipe instead
installs the independent lifecycle-leaf registry and routes the full Handle
1.2 command family through the evaluation leaf. Both paths reuse the same
aggregate Frame/world snapshot rather than copying topology state.
Profile validation proves route-limit agreement, unique scopes, wire
compatibility, remote-control availability, and runtime live-object capacity
for the fixed topology plus the maximum Handle population.

After handshake, only accepted Handle codecs and handlers are constructed.
Remote creation composes raw evaluation, ordered Create publication, bounded
continuation ownership, and a Result-delivery ticket. Once service completion
has transferred the Handle to that ticket, a failed or dropped Result send
performs ordered disposal using the original reserved slot. Remote explicit
dispose likewise crosses the shared Dispose control fence before success.
Thus cancellation is covered before CDP, during CDP, during Create, and between
Create and Result without an unbounded fallback queue.

The complete-capability recipe now statically includes both Handle handler
trees and wraps its shared dispatcher plus lifecycle-leaf registry in
`ChromiumHandleAwareRuntime`. The wrapper delegates cancellation and ordered
lifecycle claims, but owns transport close, so the recipe cannot accidentally
strand native routes or their leases. Page close, owned-Context close, main-
world replacement, Frame removal, Page crash, and reconnect also prepare and
commit retirement against that same leaf registry.
A dedicated Chrome 152 run verifies the expanded 12-capability plus control
handshake. A second live proof creates an ElementHandle in one exact world,
passes it through native `callFunctionOn` to obtain `SECTION`, and confirms
attachment teardown does not terminate external Chrome.

The typed duplex proof now negotiates Handle evaluation 1.1 through the complete
client/server recipe against dedicated Chrome. It applies lifecycle Create
announcements, sends an ElementHandle as the second encoded command's portable
argument, completes graceful close, and preserves external browser ownership.
The Handle codec carries the channel-synthesized cancellation identity but still
rejects unsupported timeout metadata. Frame creation may precede exact renderer-
world readiness; retaining on the temporary fallback route is invalidated when
the current world arrives, so callers or recipes must wait for world readiness
before creating long-lived Handles.

The optional cross-world ElementHandle backend transaction now has bounded
temporary ownership plus opt-in public argument classification, substitution,
and topology revalidation. Fake tests cover real command semantics for
same-Page child Frames, JSHandles, and cross-session/OOPIF-shaped routes; Chrome
152 covers real same-process child-Frame success and real auto-attached OOPIF
rejection. The aggregate keeps its older route and command types only as source-
compatibility APIs. The complete recipe now uses the leaf's injected live
Frame/world resolver with numeric plus stable context identity, its two-phase
connection/parent/topology retirement owners, and its single command policy.
Cross-world adoption is an explicit aggregate backend option selected through
`with_handle_element_adoption`; when omitted it installs no adoption service or
temporary-object resource budget. Selection does not add a wire capability,
because it changes only how the existing Handle 1.2 commands resolve eligible
same-Page ElementHandle arguments.
The five Handle-local APIs are designed as the coherent
additive `pwright.handle_evaluation` 1.2 surface. That version inherits the
target route's Frame/Page/client/world, uses bounded by-value serialization,
and requires atomic batch creation for `getProperties`. Its schema, generated
server dispatch, binary codec, typed client extensions, bounded CDP leaf,
optional composition, Create-before-Result ordering, cancellation cleanup, and
dedicated-Chrome acceptance path are implemented. Runtime lifecycle
transactions begin only after awaited CDP work, so concurrent topology events
cannot fail merely because a renderer operation is pending. See
`knowledge/pwright-handle-operations.md`.

### Opt-in typed client capability slice

`pwright-client` must depend only on `pwright-core` and the core-only
`pwright-browser-model`; even navigation is not a mandatory client dependency.
The base `Remote<Kind, Channel, Ids>` exposes one
generic typed command-dispatch primitive that owns call-ID allocation, stale
handle checks, connection checks, and portable failure mapping. It knows no
navigation parameter or result type.

Each public capability can add a separate client extension crate. The first,
`pwright-client-navigation`, depends on `pwright-client` and
`pwright-cap-navigation` and implements a Rust extension trait for `Page`.
`PageNavigationExt` exposes the backend primitive, while the independently
bounded `PageGotoExt` exposes only the high-level command. A channel can
implement either Service contract without implementing the other; importing
neither trait leaves both methods absent. This is static composition, requires
no `dylib`, and uses the same direct or encoded typed channel chosen by the
application.

### Executor-neutral encoded client channel slice

Remote execution is an opt-in `pwright-client-wire` adapter rather than a
dependency of `pwright-client`. Construction returns two roles sharing one
bounded connection state: a cloneable command channel implementing typed
`Service<Command<Spec>>`, and one non-cloneable receive driver. The application
must continuously poll the driver on its chosen executor. No call future reads
from the transport, so concurrent calls cannot create competing socket readers,
and the adapter never starts a task, thread, or runtime.

The command channel encodes a typed command with `WireProtocol`, verifies that
the codec produced the expected command route, registers the call before send,
then awaits a raw correlated result frame. The receive driver performs the only
`FrameTransport::receive`, routes result frames by `CallId`, and returns event,
control, and handshake frames to the caller as unhandled input for future
composable subsystems. The waiting call performs its own typed result decode
and the base client independently rechecks the response call ID.

Pending calls plus abandoned-call tombstones share one explicit capacity bound.
An opt-in shared-ledger constructor additionally reserves one call-scoped
`PendingCall` before correlation, pending-map, send-gate, or transport mutation.
A delivered result ends this logical count before the waiting future decodes its
separately bounded frame. Dropping a possibly sent future transfers ownership
to its tombstone; flushing cancellation does not release it because the server
still owes a terminal result. Late result, send/receive failure, EOF, explicit
close, or receive-driver drop releases all exact leases. The negotiated
transport exposes matching ledger-aware factories so handshake-derived channel
limits do not require an unaccounted construction path.
With optional control enabled, command and control protocols must use identical
version policy, send version, and both directional frame limits. The wire
channel synthesizes a per-call cancellation identity only when the caller did
not provide one.
Dropping a possibly sent call future atomically moves its ID to the bounded
tombstone set and queues one `Caller` cancellation under the same limit; `Drop`
does no I/O. Codec failure preserves the queued cancellation for retry, while
send failure terminally closes the channel and releases all state.

All command and control writers first acquire one shared, executor-neutral,
bounded send gate. Command waiters are FIFO; control waiters take priority so a
drop-triggered cancellation is not stranded behind unrelated queued commands.
A call dropped before acquiring the gate is known to be unsent and rolls its
registration back without a tombstone or cancellation. Once the unique permit
is acquired, the transport send is the only writer and a dropped active send is
conservatively treated as possibly sent. A successful frame send releases its
permit before waiting for the result, preserving concurrent in-flight calls.

`flush_cancellation_once` is the explicit one-message driver primitive.
`drive_once` and `drive_once_with` additionally keep the single receive future
pinned while flushing newly queued cancellations on the independent write
path. This matters because discarding a partially read length prefix would
corrupt the stream. The result of a cancelled runtime call is then consumed as
one bounded late result, releases its tombstone, and is not misclassified as
protocol corruption. Metrics expose queued, peak queued, and successfully sent
cancellations without retaining payload history. EOF,
receive failure, explicit logical close, or receive-driver drop terminally wakes
all waiters. Logical `EncodedClientChannel::close` still performs no I/O. The
unique driver may instead call `close_transport` when its transport implements
`ShutdownFrameTransport`: it closes logical state, enters the priority send
gate, then shuts down the write half. A queued command rechecks closed state
after acquiring its permit and cannot write behind that barrier. A cancelled
close can be retried without reopening the channel.

Remote construction can now start with the consuming
`EncodedClientHandshake` gate instead of constructing an application channel
directly. It checks that the bootstrap protocol matches ClientHello, sends
exactly one canonical hello, requires a Handshake response, compares the outer
version with the selected payload version, and validates that the server did
not change the client limits, invent a capability, omit a required capability,
or select an incompatible version. Only success yields
`NegotiatedClientTransport`; its channel factories derive exact application
protocols with the negotiated directional limits. Dropping a pending handshake
future drops its owned transport handle, so that handle cannot restart after a
possibly partial bootstrap frame. Legacy and in-process callers may still use
the original direct channel constructor explicitly.

The Tokio stream transport now gives `send` the matching cancellation-safety
property. It stores at most one bounded envelope and byte offset in writer
state. If a send future is dropped after a partial write, the next send or
explicit shutdown first completes and flushes that exact envelope. No writer
task or hidden runtime is introduced.

### Typed real-Chrome vertical proof

The live navigation contract test exercises the full in-process v2 path rather
than constructing a command by hand. It attaches through
`CdpAttachProvider`, creates a caller-owned temporary incognito context and
target, registers matching Browser/Context/Page objects in the runtime, creates
the typed client handles, imports `PageGotoExt`, and invokes
`goto` through `Remote::dispatch`, `DirectDispatcher`, and
`CdpNavigationService` into real Chrome.

Cleanup explicitly disposes the temporary browser context. Dropping the typed
client and attachment carries no process lease; the test verifies the external
browser remains responsive with `Browser.getVersion`. The live test now proves
session-filtered document commit and load waiting for a real `data:` navigation,
the Playwright-compatible null response for that scheme, exact context cleanup,
and the external-browser ownership boundary. It does not prove another browser
backend or a remote Response object.

### Playwright-shaped goto slice

`GotoParams` defaults to `load` and also supports `commit`,
`domcontentloaded`, and `networkidle`, matching Playwright's public wait-state
set. The CDP adapter subscribes before enabling Page lifecycle and Network
events and before sending `Page.navigate`, so a fast commit cannot race ahead
of its waiter. Events are filtered by the exact flattened CDP session and then
by main-frame and loader identities. A replacement loader interrupts the
operation; same-document navigation waits for its commit event and returns a
null response. Referrer is forwarded through typed `Page.navigate` parameters.

For a new document, the adapter waits for `Page.frameNavigated` plus the chosen
lifecycle event. HTTP(S) navigation additionally waits for and returns the
main-resource response snapshot; 4xx/5xx remain successful navigations. Data
and about-style schemes return no response, matching Playwright behavior even
if Chromium emits a protocol response. Response headers are returned in stable
name order. Network idle tracks document, subresource, and child-frame requests
in the page session, excludes EventSource and favicon traffic as Playwright
does, and requires a continuous 500 ms zero-inflight window. Broadcast lag is
a bounded-resource failure rather than silently dropping navigation evidence.

Production navigation composition now requires `CdpNavigationLimits` and a
`CdpNavigationProductionProfile`. The limits cover input and committed URLs,
native identifiers, simultaneously tracked requests, response-header count and
aggregate bytes, and status/error text. The profile multiplies a conservative
per-operation retained-state peak by an explicit concurrency bound and claims
`BackendStateBytes` in a composition-selected scope. Every call acquires a
clone-shared local operation slot and reserves that complete peak before it
returns a future, so an unpolled call is already charged and a globally
overprovisioned ledger cannot weaken the plugin's own contract.

Event handling rejects a request identifier before inserting it into the
inflight set and validates response-header count, value type, and aggregate
bytes before creating the owned response. Retained strings are compacted.
Completion, cancellation, or dropping either an unpolled or polled future
releases the lease. This boundary covers state retained by the navigation
backend action. The CDP connection separately owns its bounded incoming event
queue and shares each event's lease through backend receivers. Remote result
encoding immediately reserves the typed-result plus output-buffer peak before
the codec allocates and transfers the shrunken lease into the frame. Results
returned directly to application code are caller-owned values rather than
internally queued Pwright state.

The portable result is currently an immutable response value, not a lifecycle
managed `Response` remote object with request/body methods. Redirect-chain
objects, child-frame detach bookkeeping during network-idle, WebKit/Firefox
backends, and full upstream page-goto conformance remain follow-up work.

### Optional navigation wire codec

`pwright-codec-navigation` is the independently installable wire representation
for `BeginNavigation` and `Goto`; in-process users do not need it. It uses a
canonical Serde-free binary payload, carries timeout and cancellation metadata,
encodes every portable failure category, sorts response headers, and rejects
attachments. String length, header count, aggregate header bytes, all enum and
option tags, UTF-8, truncation, trailing bytes, and canonical header order are
validated explicitly.

The codec's normal dependency tree is only `pwright-cap-navigation`,
`pwright-core`, and `pwright-transport`; Tokio and the client/server/runtime
layers are test-only. A Tokio duplex contract test calls `PageGotoExt::goto`,
crosses the encoded client channel and canonical stream envelope, dispatches
through the typed server handler and bounded runtime, and correlates the typed
response back to the client. The server and client layers independently compare
the payload identities with the outer route, so the codec cannot weaken route
correlation.

### Composable client frame handling slice

The encoded receive driver can route any non-result frame through a synchronous
`FrameHandler`. A handler returns either a typed handled event or the original
owned `WireFrame`; it cannot accidentally consume a frame it does not support.
`FrameHandlerExt::or_else` builds a deterministic chain whose output and errors
identify the handling branch. `receive_once_with` performs no spawn and leaves
handler scheduling on the caller's chosen executor.

The first opt-in handler crate, `pwright-client-control`, recognizes only
server-to-client lifecycle control. Create becomes a `RemoteInitializer` for a
typed object capability to bind, Adopt is forwarded without copying the runtime
parent graph, and Dispose synchronously applies client liveness invalidation.
Cancel and SetSubscription are client-to-runtime messages and are rejected if
received in this direction. Non-control frames remain available to later
handlers. A matching runtime-to-transport slice is described below.

### Bounded runtime lifecycle emission slice

Lifecycle emission is opt-in through
`RuntimeLimits::with_control_messages`; the default in-process dispatcher has
no control outbox. When enabled, object registration, adoption, and disposal
produce codec-neutral `ControlMessage::Create`, `Adopt`, and `Dispose` values
inside the same runtime state lock that protects the object graph.

The outbox uses reject-newest backpressure. Before disposing a subtree, the
runtime verifies enough free capacity for the entire child-first Dispose
batch. If the whole batch cannot fit, it changes no object state and, for a
controlled dispatcher, accepts no parent-close cancellation. Create and Adopt
use the same admission-before-mutation rule. Re-adopting an object to its
current parent is an idempotent no-op and consumes no slot. Current, peak, and
capacity metrics remain observable.

An exclusive `ControlMessageClaim` separates reading from acknowledgement.
Dropping a claim preserves the oldest message; committing it removes exactly
that message. The opt-in `pwright-runtime-control` driver composes this claim
with any `WireCodec<ControlMessage>` and `FrameTransport`. Each caller-driven
`send_once` encodes and validates one Control route, awaits one transport send,
and commits only after the send succeeds. Codec, route, or send failure rolls
the claim back. The driver creates no executor, task, thread, timer, or socket.

Transport closure remains terminal: the runtime clears unsent lifecycle
messages because client EOF/connection closure invalidates every remote handle.
At that checkpoint, a concrete control-message schema, socket transport, and
server connection loop that multiplexes results, events, and control frames
fairly were still open; the following slices address the first two boundaries.

### Concrete bidirectional control protocol slice

`pwright-codec-control` provides a Serde-free canonical binary payload for all
five `ControlMessage` variants. It uses fixed-width network-order identities,
explicit discriminants, canonical optional-parent and boolean encodings, exact
payload lengths, and no attachments. Unknown variants, invalid cancellation
reasons, noncanonical values, truncation, trailing bytes, wrong outer routes,
and attachments are rejected rather than normalized.

The control 1.0 YAML and published baseline now own the root tags, positional
fields, strict booleans, and cancellation-reason enum. Generated models perform
the production binary encode/decode; only conversion into semantic core IDs and
the cross-field absent-parent invariant remain hand-written. The generated
catalog and protocol reference are drift-checked with the codec output.

The runtime control component now also has a client-to-runtime receiver.
`Cancel` is applied only to a `ControlledDispatcher`; caller and deadline are
the only accepted remote reasons because parent and transport termination are
runtime-owned. `SetSubscription` validates a live target and updates a bounded
set of `(ObjectId, EventTag)` routes. Duplicate enable and absent disable are
idempotent. Enabling a stale object or exceeding the route limit fails, while a
late disable for an already-disposed object is a safe no-op. The basic handler
still returns changed routes for local composition. Its activated wrapper adds
an async `SubscriptionActivator` transaction: enable reserves before awaiting
the backend and inserts only after success; disable retains the route and lease
until backend deactivation succeeds. Either backend failure leaves the exact
prior logical and resource state. Duplicate edges call no backend. In
production, every retained subscription owns an object-scoped `EventItem`
lease, and the profile also declares one maximum control encode so inbound and
outbound control use the same negotiated ledger. A single
`PendingSubscriptionTransition` survives cancellation of an awaiting backend
future: uncertain enable keeps its new lease; uncertain disable leaves its
committed route untouched. No later control can mutate around that state, and
Dispose or EOF performs a compensating disable.

Directionality is enforced on both peers: the client lifecycle handler rejects
Cancel/SetSubscription, and the runtime receiver rejects Create/Adopt/Dispose.
Duplex contract tests carry runtime Create through the control codec, canonical
stream envelope, Tokio transport, and client handler, and carry client
SetSubscription through the reverse path. `deactivate_and_forget_object`
commits successful backend edges individually and retains a failed route plus
all later routes for retry. Chromium's topology helper feeds every Dispose
effect from an already-committed update through that operation. If route
retirement already removed the exact CDP session, disable is complete because
there is no browser-side subscription left; enable still rejects that stale
route.

### Typed server event-output slice

`pwright-server-events` is the server-side inverse of a typed client event
handler, but it does not couple event production to transport work. A
`TypedEventSender<Spec>` is cloneable and may be injected into the backend
component that owns that event. The corresponding
`TypedEventOutbound<Spec, Codec, Transport>` is the only queue consumer and
implements the supervisor's caller-driven outbound contract. Neither side
selects an executor or starts a background pump.

Each installed event type owns an independent bounded queue and overflow
policy. An outbound future claims the oldest event by moving it out of the
container, but the queue treats that claim as a reserved slot. This preserves
FIFO and prevents `DropOldest` from dropping an event whose send is already in
progress. Failure and cancellation put the same value back at the front for
bounded ownership; success alone commits it. A polled send future must run to
completion or its transport must close—a partial generic transport write makes
retry on the same connection unsafe. Encoding is accepted only when its outer
route is exactly `Event { source: event.object_id(), event: Spec::EVENT }`.

Production construction combines maximum queue `EventItem` and `EventBytes`
retention with one maximum source/output `WireBytes` encode. The event-specific
retained-size function is checked before reservation. The transport must expose
production evidence for the exact same ledger, so a numerically identical but
independent budget cannot enter the remote path.

The Chromium root records installed event profiles separately from command
capabilities. No event queue or codec is present by default. Installed event
tags must be unique, require the remote handshake, and use a wire profile
consistent with that handshake. After acceptance, the generic factory derives
the actual directional protocol from `AcceptedServerHandshake`, allowing a
client's smaller receive ceiling while never exceeding the declared maximum.
Multiple outbound plugins compose statically; their chain alternates after
successful sends and does not start another lane while the selected lane owns
an active transport send. Once all lanes report idle, the chain races their
readiness futures without starting a writer. A typed producer stores at most
one replacement waker in its already bounded state and wakes it after an
accepted enqueue. The supervisor then performs the send itself; no notifier
task or runtime is hidden in the event component.

### Composable typed server command slice

`pwright-server-wire` completes the generic command path without introducing a
dynamic runtime registry. A `TypedCommandHandler<Spec, Codec, Service>` claims
only `FrameRoute::Command` values whose method equals `Spec::METHOD`; every
other owned frame remains available to the next handler. Static `or_else`
chains install exactly the command capabilities selected by the application.

Every remotely advertised command implements the stronger
`CapabilityCommandSpec`, which binds its method to the owning static
`Capability` and `CommandId`. `ServerCapabilityInventory` walks either the
sequential or concurrent handler chain and is the preferred source for
`ServerHandshakeConfig`. It rejects duplicate command handlers, duplicate wire
method tags, commands absent from their descriptor, conflicting descriptors,
incomplete capability handler sets, and invalid capability dependency graphs.
Only after those checks does it create the bounded, deterministically ordered
wire manifest. The older caller-supplied manifest constructor remains for API
compatibility and low-level tests, but production composition no longer needs
an independently maintained capability list.

`pwright-server-navigation` is the first generated capability-specific server
adapter. Its constructor accepts a shared navigation wire protocol and one
injected service for each schema command, then returns the complete static
concurrent chain. It starts no task and selects no runtime, backend, browser,
provider, or concrete transport. Consequently applications may install remote
navigation without making the core or generic server depend on navigation, and
may omit the adapter entirely for local-only use.

For a claimed frame, the handler decodes `Command<Spec>`, verifies decoded call
and target identity against the outer route, invokes a typed runtime
`Service<Command<Spec>>`, checks response correlation, encodes the
`CommandResult<Spec>`, verifies its Result route, and awaits transport send.
Decode, identity, service, encode, route, and send failures remain distinct.
No partially validated command reaches the runtime and no mismatched result is
sent.

`EncodedServerDriver` owns the connection's only receive path and advances one
frame per caller-driven invocation. Clean EOF, receive failure, explicit close,
and handler failure become terminal logical states. It performs no implicit
socket shutdown; the owner still controls the concrete transport. The runtime
control receiver implements the same async handler contract, so sequential
command and control handlers can share this one reader rather than race on the
stream.

The concurrent server checkpoint adds an application-phase
`EncodedServerSupervisor` without changing that smaller sequential API.
`ConcurrentTypedCommandHandler` turns a validated typed handler invocation into
owned work, and `ConcurrentServerHandlerChain` statically composes independently
installed command capabilities without a dynamic registry. The supervisor owns
the unique reader, alternates ready inbound work with all active command
futures, and treats `ServerControlFrameHandler` as an ordered async barrier
while commands remain pending. It polls a claimed control once before looking
at later transport input. A handler may explicitly guarantee that a pending
first poll released its input frame; only then may the supervisor retain one
next frame to observe EOF. That frame remains undispatched until control
success, so a command cannot overtake backend activation. EOF cancels the
control and invokes its async close compensation. At most one control future
and one prefetched bounded frame exist. Conservative handlers declare no such
release and receive no speculative poll. The supervisor creates no executor
task, thread, timer, runtime, socket, or hidden queue.

The admitted-command limit is nonzero and explicit. Every accepted command is
polled before another inbound frame, so a controlled runtime registers its
cancellation identity before a following Cancel can arrive. If another command
arrives at capacity, the supervisor closes rather than buffering work that the
runtime cannot yet find and cancel. Metrics report current and peak active work,
accepted/completed commands, handled controls, unhandled frames, capacity
failures, terminally dropped work, and the close reason. Receive, protocol,
command, control, observer, outbound, command-capacity, and resource failures
remain distinct. Before returning any of those terminal failures, the
supervisor awaits the same reasoned connection-termination hook used for EOF,
until one attempt completes. If that hook also fails, `TerminationCleanup` owns
the boxed primary error and the cleanup error. Dropping the run future is terminal,
releases active work, records `RunFutureDropped`, and prevents an unsafe restart
after a possibly partial receive; because `Drop` cannot await, an owner must
subsequently call the async close path with controls to perform backend cleanup.
An interrupted cleanup barrier is not marked complete and can be retried by
that owner call.

The supervisor's retained future vector is now a production resource boundary,
not only a logical length check. Each admitted command reserves one
call-scoped `PendingCall` lease before its handler future exists; a runtime may
hold a second lease for the same call, and the composition plan adds both.
Duplicate active call IDs and undeclared capacity fail before additional work
is retained. Completion, connection failure, drain expiry, and dropped run
futures release the leases with the owned futures.

An optional `ConcurrentServerOutbound` lane extends the same loop without
creating a task. Receive, round-robin command polling, and outbound polling use
a rotating three-lane cursor. A successful lifecycle send becomes an observed
supervisor event; failure closes with `OutboundFailed`. A ready idle result is
parked on `wait_until_ready`, preventing a busy loop. The default readiness
future never wakes, preserving connection-progress retries for simple sources.
Typed event producers override it with a bounded waker, so an accepted enqueue
can reactivate the supervisor without a task or executor.

Runtime lifecycle output may additionally be wrapped in
`OrderedRuntimeControl`. One priority gate serializes its normal outbound send
with command-completion barriers. The gate retains at most the supervisor's
declared active commands plus its one outbound future; those futures are
already covered by the supervisor's active/pending command boundary and the
fixed outbound lane. A barrier is acquired before its command performs native
work, so no ordinary writer can steal a later Dispose. It sends until the exact
target Dispose or reports a typed invariant failure, and releasing or
cancelling it wakes the next barrier before an ordinary writer. This is the
remote BrowserContext/Page ordinary-close lifecycle-before-Result guarantee;
unrelated capabilities continue using the generic handler and need not depend
on it.

The public `Locator.fill` drop-cancellation duplex proof now runs through this
supervisor itself: the unique reader accepts the owned command, keeps polling
while its backend is pending, applies the canonical Cancel through
`RuntimeControlReceiver`, lets the controlled command finish its cleanup and
send `Cancelled`, and leaves both peers with zero live call state. Handshake
gating and graceful half-close/drain are described below. Concrete endpoint
identity extraction remains connection-composition work.

A separate Tokio duplex contract test exercises a typed client Remote dispatch
through `pwright-client-wire`, the canonical stream envelope, the server
driver, `DirectDispatcher`, result encoding, client correlation, and typed
result decoding.

The same crate now provides `HandshakeFirstHandler`, which wraps the complete
application handler chain. Before negotiation it accepts only one Handshake
route, validates the frame against the server's receive limits, decodes a
codec-pluggable `ClientHello`, negotiates, encodes and sends `ServerHello`, and
only then unlocks commands and controls. The initial frame header version is
not trusted because the peers have not selected one yet; the version ranges in
the hello payload are authoritative. Afterward, every frame must use the exact
selected version and pass the negotiated server-side receive limits before an
application decoder sees it. A duplicate handshake is a protocol failure.

The production supervisor now owns this phase transition directly as well.
Constructing it with `new_handshake_required` makes application `run` reject
without consuming a frame until `accept_handshake` succeeds. A pluggable async
`ServerHandshakeGate` receives the bounded decoded ClientHello before
negotiation; its returned authorization context is handed back to the caller so
that only authorized capability handlers need be installed. Endpoint adapters
may capture Unix peer credentials, mTLS identity, or tailnet identity in the
gate without placing secrets in the canonical hello. The deliberately named
`AllowUnauthenticated` gate is an explicit opt-in for trusted local or
in-process use, not an implicit remote default. Rejection sends no ServerHello.

After authorization, the supervisor negotiates, sends ServerHello, stores the
selected version and inbound limits, and validates every application frame
before routing. Application-before-handshake, duplicate handshake, version
drift, frame-limit violation, authorization rejection, negotiation failure,
codec failure, and transport failure are distinct. Dropping a pending
authorization/handshake future is terminal so a possibly partial receive or
send is never restarted.

Production handshake installation additionally declares the maximum overlap of
the typed server hello and its encoded frame. Because the client advertises the
server's outbound limit inside an untrusted hello, the profile caps that offer
before authorization and encoding. The response reserves before codec output
allocation and remains tagged with the same ledger required by a production
transport. Chromium composition installs this handshake, its transport
requirements, and control handling only when the remote profiles are selected;
local attach users do not inherit those components.

Remote navigation and locator handlers are built from
`AcceptedServerHandshake::application_protocol`, not from an independently
guessed connection version. Preparation requires every installed capability's
wire version, server receive limits, and peer frame ceiling to equal the
handshake contract; a mismatch fails before an endpoint is opened.

The accepted server value builds the inverse exact directional application
protocol: server sends are bounded by the client's advertised receive limits
and server receives by its own advertised limits. A Tokio duplex contract test
now runs the consuming client gate and handshake-required supervisor together,
then sends a typed command over that same stream and receives its correlated
typed result. This proves that negotiation is the gate to application use, not
an isolated codec round trip.

EOF policy is likewise explicit. `DropActive` preserves the small immediate
close behavior. `DrainActive` first invokes the control handler's
transport-close transition, stops receiving, and continues polling admitted
commands until their terminal results have been written. The
`ServerControlFrameHandler` close future is an async GAT barrier, and its
reasoned `handle_connection_terminated` entry point defaults to the original
transport-close behavior. EOF or any explicit terminal failure cannot complete
while selected backend cleanup is pending. The plain
`RuntimeControlReceiver` maps it to `ControlledDispatcher::close_transport`.
The activated receiver signals that runtime transition first, then disables
every retained subscription in deterministic order with constant additional
memory. Because the peer is already gone, it releases every local route even
when backend cleanup fails and reports the first failure plus the count of
additional failures. A duplex proof half-closes the client write side with a
pending Fill, observes `TransportClosed`, drains the typed error through the
still-open read side, and verifies zero pending calls and live objects. Backend
operation futures are dropped; the runtime deliberately sends no browser
cleanup command over the dead owner transport.

The activated receiver removes each pending or active subscription route only
after its backend disable future resolves. Cancelling that close future
therefore retains the current route and its lease for a later cleanup retry;
completed cleanup failures still consume every route because peer ownership is
already terminal and the handler has attempted every deactivation.

`EncodedServerSupervisor::close_with_controls` is the explicit asynchronous
close API and invokes that barrier once with `Explicit`. The legacy synchronous
`close` only records logical closure because it has no control handler to await;
a later `run` or `close_with_controls` call supplies the missing boundary and
performs the cleanup attempt. Cleanup completion events are observable on the
normal EOF path. On a primary failure they are intentionally suppressed so a
failed observer is never re-entered and the original failure remains dominant.

For transports with an owned write half, `close_transport` preserves any prior
supervisor terminal reason and then emits clean EOF. The combined
`close_with_controls_and_transport` always attempts both the reasoned
control/backend barrier and transport shutdown. Its typed error distinguishes
either individual failure and preserves both when they fail together. Thus
logical lifecycle cleanup and physical endpoint closure are composable but
cannot accidentally replace one another's diagnostics.

Callers that require a bounded graceful close use
`run_with_drain_deadline` and inject their executor's deadline future. It is
not polled before EOF or when no command remains, so the generic supervisor
still owns no clock or runtime. After EOF, ready command results are processed
before an simultaneously ready deadline. Expiry clears all remaining owned
work, increments a dedicated metric, and closes with
`DrainDeadlineExceeded`; the original `run` remains the compatibility path
with an unbounded drain future.

The older handler test still uses a test command codec; its handshake
integration uses the canonical hello codec. The concrete Unix endpoint test
feeds kernel peer identity into the real server handshake gate; the Tailnet and
mTLS endpoint tests feed their authenticated identities through that same gate.
Generated production command payload types, additional backend-to-event
adapters, operational public-listener policy, and production drain-duration
policy remain open.

### Typed client event and subscription slice

Event handling remains an optional capability layer. A
`TypedEventHandler<Spec, Codec>` recognizes exactly the matching
`FrameRoute::Event` tag, decodes `SequencedEvent<Spec>`, verifies that the
decoded source matches the outer route, and preserves all other owned frames
for another handler. Each handler owns one explicitly sized `BoundedQueue`;
installing the handler is what installs that event payload type and its codec
requirement.

Overflow is observable data, not a hidden callback side effect. A reliable
event queue may use `RejectNewest`, returning the rejected typed event to its
driver. A telemetry queue may use `DropOldest`, returning the displaced event
while retaining the newest bounded window. The handler reports queue capacity,
current and peak depth, accepted events, rejections, and drops. It creates no
task and requires neither Tokio nor Serde.

Ordering metadata belongs to the typed server/client transport boundary rather
than to backend-native events. Each installed `EventSpec` server channel owns
one positive, non-wrapping sequence. It assigns a sequence before admission so
any server reject, drop, or resource failure is detectable when a later frame
arrives. The first frame observed in a connection epoch establishes a baseline;
subsequent frames are classified as consecutive or as a gap with exact expected,
received, and missing counts. Zero, duplicate, and regressing values are
protocol errors. A new connection epoch clears queued old-epoch events and
resets the baseline while preserving cumulative diagnostics. Client sequence
observation happens before local queue admission, keeping transport loss and
local backpressure separate.

Production installation additionally uses a `TypedEventProductionProfile`.
Queue capacity determines the maximum `EventItem`, source-scope, and lease
counts; capacity multiplied by an explicit per-event decoded-size limit
determines `EventBytes`. The same event-specific sizing callback is checked
against that local limit before ledger mutation. Thus independently installed
event plugins share one connection budget without borrowing spare bytes to
violate their own declarations, and a decoded event with no variable bytes
still consumes fixed item capacity.

`SubscriptionTracker` separately bounds unique `(ObjectId, EventTag)` routes
and reference-counts local listeners. It produces
`SetSubscription(enabled=true)` only on zero-to-one and
`SetSubscription(enabled=false)` only on one-to-zero. Additional listeners and
partial removal create no wire traffic. Disposing an object can forget all its
local listener counts without sending an unsubscribe to an already-dead remote
object.

This proves typed routing, bounded delivery, and lazy subscription edges, but
does not yet choose the overflow class for any Playwright event, send the
client-to-runtime control message, activate or deactivate a browser backend
domain, or define fair multiplexing among result, lifecycle, and event frames.

The first production-directed event family is now the separately installable
`pwright.page-events` capability at version 1.2. `PageLoad` mirrors Playwright's
payload-free Page `load` listener. `BrowserContextPage` carries a ready Page
from `browser_context.on("page")`, while `PagePopup` uses the opener as its
source and carries the popup Page from `page.on("popup")`. Their fixed-size
wire payloads are kind + `EventSequence` + duplicated source identity, followed
by the related Page identity for discovery events. Version 1.2 is the additive
discovery-event surface. Capability descriptors and registries own stable
event IDs independently from numeric route tags; server
inventories require every declared event surface before advertising the
capability. Backend manifests likewise keep event features distinct from
command features, so an event-only Chromium composition is valid and does not
pull navigation or locator code into its public capability set.

`CdpPageEventAdapter` is deliberately caller-driven. Subscription activation
enables the Page domain and lifecycle events for the exact current CDP session;
deactivation disables lifecycle events but does not disable the shared Page
domain. Native `Page.lifecycleEvent` values emit `PageLoad` only when the event
belongs to the current session, its name is `load`, and its frame ID equals the
topology's current main-frame native ID. Child-frame and stale-session events
are ignored. The adapter starts no receiver task and returns the typed event to
the composition root for its explicitly bounded `pwright-server-events` queue.

`ChromiumPageSubscriptionActivator` resolves through `CdpPageServices`, whose
route entries retain the exact `CdpClient` and session identity established by
the latest topology synchronization. The accepted-composition factory wraps
the production-budgeted runtime control receiver only when Page events were
negotiated. Thus an event-only binary installs neither navigation nor locator,
and an omitted Page event capability cannot activate CDP lifecycle events.
Context-Page and popup subscriptions are browser-side no-ops because the
already-selected topology driver produces them; they never enable another CDP
domain.

`ChromiumTopologyPageEventPublisher` is the explicit, task-free bridge from
committed `PageReady`/`PopupLinked` effects to independently selected typed
server-event senders. Missing senders mean missing queues. Installed senders
retain fixed-size events under their own production budgets. Rejecting the
newest event or evicting an older event is returned as a resynchronization-
required publication error; the bridge never silently presents an incomplete
Playwright discovery stream. Because topology is already committed when it is
published, its owner must close the event view or reconstruct it before
continuing after such an error.
Remote Context-Page/popup profiles are rejected unless ordered lifecycle
control and Page discovery are also selected. The publisher's async fenced
entrypoint scans the same topology update for the final creation required by
its discovery events, holds the ordered-control barrier, physically writes
through that exact `Create`, and only then admits the events to their lanes.
Existing objects replayed during reconstruction require no new creation fence.

The complete server-side surface is now independently installable through
`pwright-composition-chromium-page-events`. One caller-provided CDP event
cursor, adapter, shared production topology, transport, profile, and resource
ledger produce three separately bounded `RejectNewest` queues for `PageLoad`,
`BrowserContextPage`, and `PagePopup`, plus the creation-fenced topology
publisher and caller-polled load backend. Each lane keeps its own capacity and
retained-byte limit; installation rejects zero capacity and rejects discovery
lanes unable to retain one related `ObjectId`. The leaf owns no task, executor,
topology, transport reader, browser, or process. Its standalone dependency
proof contains exactly 15 Pwright packages. The aggregate compatibility API
converts only a complete three-lane selection with one identical wire profile;
partial or mixed-wire selections remain aggregate-only and cannot silently
drift into this leaf.

`ChromiumTopologyConnectionPump` now owns that caller-driven topology turn. It
either terminates on CDP cursor loss or performs an explicitly bounded
same-connection reconstruction under the next non-wrapping epoch. The replay
policy is rejected at construction when a portable Context-Page or popup lane
is present: topology replay cannot prove whether a callback was already
delivered, so the only correct recovery is a fresh remote connection. An inert
publisher may reconstruct because it has no user-visible callback history.

One polled pump future must be retained through completion. Dropping it leaves
a detectable in-flight marker; the next call makes the pump terminal instead
of guessing whether CDP initialization, topology publication, or a lifecycle
write partially completed. Terminal pumps can close the server supervisor with
the classified `BackendEventFailed` reason, running reason-aware control
cleanup and transport half-close while preserving both failures. The pump
still creates no task.

The application connection loop now has an optional fourth
`ConcurrentServerBackend` lane. The supervisor rotates receive, completed
command work, outbound delivery, and backend work so an always-ready lane
cannot starve the others. A backend implementation retains its one pending
turn internally; other lane progress never destroys that future. The
`ChromiumTopologyServerBackend` adapter owns exactly one pump turn and restores
the pump after completion, while `NoServerBackend` preserves compositions that
install no browser event adapter. Backend failure enters the supervisor's
existing reasoned cleanup path as `BackendEventFailed`. The complete
application-run entrypoint then attempts transport write-half shutdown for
every terminal result and preserves both the run and shutdown errors when both
fail. A synchronous backend termination hook drops any pending topology turn
on EOF, failure, or abandonment of the complete run, so connection-scoped CDP
work cannot outlive its remote event view. Handshake remains an explicit
preceding phase and no listener, task, executor, or second transport reader is
selected.

The fresh remote unit now carries that boundary through application assembly.
`ChromiumRemoteConnection::accept_handshake` consumes the runtime-installed
stage and can obtain its `AcceptedServerHandshake` only from the supervisor in
the same value. The resulting accepted stage moves its attachment, supervisor,
runtime, authorization, and optional unique topology driver into one
`ChromiumApplicationInstallContext`; plugins can build only the capability
handlers and typed outbound lanes they selected. If topology ingestion was
selected, the installer must take its one driver or assembly fails and drops
the entire connection unit. The root then wraps every caller backend, including
`NoServerBackend`, with mandatory `ChromiumCdpLivenessBackend`. The final
application type exposes one complete `run` path, which owns the four-lane
supervisor through terminal cleanup and transport write-half shutdown. It
still creates no listener, executor, task, browser process, or browser download.

Profile installation and peer negotiation are now separate, enforced gates for
every remote application surface. Each Context, Page, Frame, navigation,
evaluation, screenshot, DOM-content, and locator handler checks its owning
capability in the accepted hello before constructing a codec, wrapping a
service, or claiming production resources. Each remote event profile carries
both its stable event ID and owning capability; preparation rejects an event
whose owner is absent from the installed registry, and connection assembly
creates its queue/outbound lane only when that owner was negotiated. A
control-only handshake regression traverses every current application handler
and event installer and proves all return omission with an unchanged resource
ledger. Therefore “the binary offers this plugin” can never be mistaken for
“this peer may call or receive it.”

The first packaged subset above that assembly boundary is the
pre-provisioned-Page navigation recipe. Its runtime installer creates one
shared target-aware direct dispatcher; its application installer contributes
only generated `BeginNavigation`/`Goto` handlers. The control, portable
outbound, and optional backend lanes use public zero-state omission types, so
their absence is visible in the final Rust type. Installation also checks the
accepted hello rather than treating a profile selection as peer negotiation.
Continuous topology events are rejected because discarding their unique driver
would create a stale object graph. Consequently, this recipe is valid only
when the caller has already provisioned the Page topology/runtime identity.
Automatic Page discovery is a larger recipe containing lifecycle control and
an owned topology backend, not hidden behavior in navigation. That larger
assembly is now representable without self-reference: the topology backend
owns an `Arc` to the controlled Page runtime and a clone of the ordered control
writer, and moves both into its retained pump future. The corresponding recipe
requires page discovery, topology events, remote control, and negotiated
navigation; it emits lifecycle `Create` messages but deliberately installs no
portable Page/popup event queues. Missing pieces fail at installation rather
than degrading to stale topology. Every topology update containing lifecycle
creation now holds the ordered-control barrier through its final `Create`, even
when the portable Page/popup publisher is inert. This is necessary because a
separately polled outbound lane may already have reported idle before backend
discovery adds objects. A full-duplex live test now proves the complete path:
negotiated client handshake, control-frame discovery, typed client Page,
navigation result, graceful remote close, and survival of external Chrome.
The optional Tokio leaf can now place that same fresh connection behind any
statically typed `ChromiumEndpointAcceptor`. Its server value holds the
stateless attach factory, explicit external CDP endpoint, and caller-pre-bound
listener; each caller-polled accept future admits one connection only after
endpoint authentication. It adds no loop, task, runtime, retry, admission
policy, or deadline.

The opt-in `ChromiumAllCapabilitiesApplicationInstaller` now supplies the
complete recipe for every v2 Chromium capability implemented at this
checkpoint. It requires all twelve application capability IDs plus control in
the accepted handshake, then constructs one static command chain for Context/
Page creation and lifecycle, value evaluation, Handle evaluation/disposal,
navigation, locator, screenshot, and DOM content; an activated Page
subscription control receiver; ordered lifecycle output chained with bounded
PageLoad, Context-page, and popup event writers; and a backend chain containing
topology discovery plus PageLoad normalization. It also refuses installation
unless the explicit Playwright-compatible utility-world infrastructure profile
was selected, so the included locator cannot silently fall back to the user's
main world. Its runtime installer also requires the selected Handle route table
and wraps the dispatcher in `ChromiumHandleAwareRuntime`, making transport-close
route retirement structural. Native CDP liveness is still added by the
composition root outside that recipe.

PageLoad uses a second cursor on the existing bounded CDP event delivery path,
not a second socket reader or semantic queue. The topology and PageLoad backends each
retain at most one caller-polled future and are fairly composed by the generic
backend chain. Cursor lag, event-queue loss, or mapping failure terminates the
connection instead of silently degrading its Playwright-shaped event view.
The complete recipe now obtains its publisher, load backend, and all three
outbounds from `ChromiumPageEventsInstallation`; it no longer assembles those
queues through aggregate internals. The aggregate reexports the leaf types for
source compatibility, while `pwright` exposes independent `client-page-events`
and `chromium-page-events` features so either side can be selected without the
complete recipe.
The complete-capability profile derives 10 maximum cursors from topology,
PageLoad, four Page-network lanes, two navigations, and two Locator actions. Its
dedicated Chrome assembly proof retains only the three steady cursors actually
installed by topology, the shared network driver, and PageLoad; transient
navigation/Locator capacity remains available without retaining payloads. A
second dedicated Chrome 152 proof, rerun three consecutive times after the leaf
migration, creates four Pages, overlaps two slow
navigations and two Locator clicks, observes PageLoad delivery, and drains to
zero queued events without lag. The profile configures four supervisor ingress
slots as well as four runtime active-command slots; either boundary being
smaller would reject an otherwise admitted composition. The observed fanout
peak is 6 or 7 because the canonical network driver is shared, while 10 remains
the safe bound for an alternative per-Page network-lane assembly.
The runtime cleanup queue remains externally drained, Page-close polling is
injected, and the recipe adds no task, executor, timer, retry, browser launch,
or browser download.

### Optional HTTP lifecycle foundation

HTTP observation is now separated from the expensive authorities that are
often grouped under “network.” `pwright-network-model` depends only on core and
owns bounded backend-neutral Request/Response metadata plus a native-request
correlation state machine. `pwright-cap-network-events` adds only the four
Playwright-shaped `request`, `response`, `requestfinished`, and
`requestfailed` event contracts. It does not install body reads, routing,
WebSocket frames, HAR, a codec, a queue, a runtime, or a task.

The base browser catalog now includes Request and Response. A Request may be a
child of BrowserContext, Page, or Frame; Response is a child of Request. The
Chromium normalizer injects portable object IDs, filters the exact flattened
CDP session, validates every ID/string/header/count before a second owned copy,
sorts headers, and maps `requestWillBeSent`, `responseReceived`,
`loadingFinished`, and `loadingFailed`. A redirect closes the previous
Request/Response pair and starts a new Request identity even though CDP reuses
its native request ID. HTTP error status remains response-plus-finished, not
request-failed.

Every batch separates ordered lifecycle `Create` controls from typed events.
The Chromium composition now atomically applies those controls to the shared
runtime, physically writes through the final Create, and only then admits the
semantic effects to four independently optional bounded typed queues. Queue
rejection/eviction is terminal, and disposing the Page retires its Request and
Response descendants. The tracker
retains only active native correlation and removes it at the terminal event;
it never stores completed history or body bytes. Its production profile
pre-reserves a conservative full `BackendStateBytes` ceiling for the declared
active-request and native-ID limits, and drop releases the lease. Exhaustion,
unknown transitions, duplicates, wrong redirect order, or oversized metadata
fail explicitly without extending the active map.

A release-profile Chrome 152 proof used a fixture bound to this Mac's
Tailscale interface and addressed by full MagicDNS. It observed the real
`request -> response -> requestfinished` sequence, redirect closure before the
next Request, HTTP 500 as response-plus-finished, and a truncated response as
response-plus-failed. It also proves zero remaining native correlation,
release of the production backend-state lease, exact owned-context cleanup,
and survival of the externally owned browser. The adapter has no `CdpClient`,
so it cannot issue `Network.getResponseBody` implicitly.

Cross-event remote delivery and its causal lifecycle turn are now a separate
`pwright-composition-chromium-network-events` leaf. Its exact profile binds one
wire version, directional frame limits, `NetworkEventsCodecLimits`, queue
capacity, and maximum retained decoded bytes. Installation creates one
`RejectNewest` queue and one monotonic sequence shared by request, response,
finished, and failed routes; zero capacity, a frame too small for the largest
terminal snapshot, or an insufficient retained-byte bound fails before any
event is admitted. The lifecycle publisher atomically applies normalized
Create controls, physically fences through the final Create, selects Page and
BrowserContext delivery through an injected object-safe port, publishes Page
before Context with shared portable identity, and only then retires terminal
roots. A generic single-cursor batch-driver port and retained supervisor backend
make receive -> publish -> exact-Page acknowledgement one caller-polled turn;
connection termination drops both the retained future and driver. The leaf
starts no task and owns no CDP receiver, normalizer, subscription mutation,
topology registry, executor, browser, or process. Its standalone closure is
exactly 16 Pwright packages, including the CDP batch contract and runtime/control
fence. The complete recipe consumes this full leaf, while the old aggregate
publisher/backend remain compatibility APIs and the aggregate Page-route
registry implements the two injected ports.

The attach profile selects this Page-scoped lane explicitly, budgets its
adapter count, subscription registry, and runtime children, and installs no
task. A cursor is registered for the exact synchronized Page/session before an
ordered first-listener edge may send `Network.enable`. Four Page/tag interests
share one bit mask; the final edge sends `Network.disable` only when navigation
is not a lifetime shared owner. The remote publisher consults the committed
mask, while independently installed local queues remain unaffected. A dedicated
network profile builds one negotiated canonical binary codec, queue, and
monotonic sequence across all four event tags; generic per-event outbounds
remain rejected. The opt-in client uses one routed handler, typed announced
Request/Response handles, and Page/BrowserContext first/last-listener helpers. 4,096
completed lifecycle and wire iterations prove no completed native history and
a one-slot production event peak. One optional caller-polled attachment driver
now owns the single cursor and a bounded exact-session normalizer table. The
registry retains empty Context masks
separately from Page masks, Page domain ownership uses their union, and remote
fan-out emits Page then Context frames with shared portable identities. The
Context activator retains one pre-budgeted exact-session plan, commits the bit
only after every Page succeeds, compensates definite failure in reverse, and
turns a cancelled enable into an idempotent all-session disable cleanup. The
explicit `BrowserContext.newPage` service can take the driver's non-owning
registration handle: it prepares and enables a provisional exact-session lane,
holds any racing event for that session, and atomically clears the provisional
barrier only after Page/Frame runtime, topology, and route publication. Failure
disables and retires the lane before exact-target cleanup; cancelled activation
has an explicit idempotent recovery path. Automatic new-Page discovery uses the
same handle and detaches the auto-attached session after rollback. Session
replacement reserves one shadow adapter beyond the public Page-lane count,
preserves direct and inherited masks, enables it before staging the new route,
and adds old active Request roots to the runtime rebind lifecycle. Commit swaps
the exact adapter; failure restores the old session, disables the shadow, and
detaches the incoming session; cancelled activation is explicitly recoverable.
The client now also exposes identical first/last edges on BrowserContext. The
driver's owned network turn normalizes, publishes lifecycle/control/events, and
acknowledges the exact Page before another turn can begin. The application
installer treats a selected network driver as unique required ownership,
attaches its registration handle to topology independent of take order, and the
all-capabilities recipe composes its remote publisher/outbound and retained
supervisor backend beside topology and PageLoad. That backend drops its cursor
and exact Page registrations on terminal connection notification. Explicit
Page and BrowserContext close now join the same non-owning registration handle:
successful local publication retires the affected adapters, while failed close
preserves them. Each driver lane retains one resource-budgeted parent Context
identity, so Context close filters in place without a second Page-ID vector or
holding the subscription lock across route publication. A dedicated Chrome 152
network workload performs one complete warmup lifecycle, reuses its single Page
lane, and runs three settled Request/Response lifecycles with zero native
correlation and control backlog at every steady sample. Terminal events now
carry complete immutable Request and optional Response snapshots. After all
selected queues accept a batch, the runtime immediately disposes its Request
root and cascades through the Response child; the base capability promises live
handles only while the exchange is in flight. The real persistent-Page
workload now holds `LiveObject.current` at `[4, 4, 4]` rather than the previous
`[6, 8, 10]`. A bounded repeated driver then ran three independent
attachment/runtime instances per series. The current production workload
selects a task-free reclaim-after-all-read fanout with two derived subscribers:
the shared network driver and one admitted navigation. A command-response
barrier and nonblocking drain require zero queued raw events after every
navigation and teardown. Two consecutive three-run series kept all six runs at
`[4, 4, 4]` and admitted them only after every resource class returned to its
warm baseline plus browser-process settlement; `EventBytes` is no longer a
variable-payload exception. RSS ranges remain diagnostics rather than budgets.
The navigation workload now uses the same bounded three-run series boundary.
Warmup waits for the process count to hold steady and may establish a count
above cold startup; only measured teardown is required to return to that warm
comparison count. This distinguishes legitimate first-use browser helpers from
processes retained by the measured operation.
The legacy v1 receiver keeps Tokio broadcast semantics. The network codec also
derives the maximum terminal payload and retained metadata
from its lifecycle limits; production preparation now rejects undersized frame
or event-byte profiles before attaching to CDP. The local Playwright
`1.63.0-next` source confirms that terminal network events do
not dispose Request/Response dispatchers and that generic per-type GC waits for
10,000 entries before trimming the oldest 10%. Pwright deliberately does not
copy that retention policy into its base network-events capability. A separate,
hard-bounded client-acknowledged lease capability remains the required design
for users who opt into Playwright-style post-terminal handles. Page-session
detach already
holds the network state guard across topology/runtime publication and retires
the exact lane only after the parent subtree commits; failed publication leaves
the adapter untouched. A normalized network batch creates an explicit
publication barrier; replacement, retirement, and another receive fail closed
until the owner acknowledges the exact Page after publishing the batch. Route preparation rejects any
active-Context Page not admitted by such a positive participant.
Response body, routing, WebSocket, and HAR each require their own capability
and resource decision. The complete decision is recorded in
`knowledge/pwright-network-lifecycle.md`.

## Goal

Pwright v2 must let a consumer install exactly the browser capabilities and
delivery surfaces it uses. The minimal core must not pull Tokio, WebSockets,
Chrome DevTools Protocol, serde, CLI parsing, gRPC, script execution, browser
launching, or test fakes into the dependency graph.

Pwright should stay close to Playwright's public API semantics and channel
protocol model so that its object model, automatic waiting behavior, errors,
and user experience remain familiar. This does not imply copying Playwright's
runtime architecture or implementation language.

## Playwright alignment and hard constraints

The design should borrow Playwright's typed client/runtime boundary, remote
object identity, commands, events, cancellation, validation, and explicit
object lifecycle. The following parts of the Playwright distribution model are
not acceptable defaults for Pwright:

1. Pwright must not require or launch a Node.js driver.
2. Pwright must not download, bundle, or silently install Chrome/Chromium by
   default.
3. Browser acquisition must be a pluggable provider. System executables,
   explicit executable paths, and remote CDP endpoints are independent options;
   a managed browser downloader may exist only as an explicit installation.
4. Browser processes and the Pwright runtime must be measured separately.
5. Runtime object tables, event queues, tracing data, network history, logs,
   screenshots, and response bodies must all have bounded retention.
6. Heavy observability features such as tracing, video, HAR, screencasting, DOM
   snapshots, and response-body retention are opt-in capabilities.
7. Resources must support deterministic close/dispose behavior. Rust `Drop`
   may enqueue cleanup, but it must not be the only lifecycle mechanism.

Playwright's high memory use has not yet been attributed to one confirmed root
cause. Possible contributors include the Node/V8 runtime, the Chrome process
tree, duplicated client/server object graphs, retained channel objects and
handles, event listeners, tracing, network records, and unclosed contexts or
sessions. Pwright must expose enough metrics to distinguish these sources
instead of treating total process-tree memory as one number.

The locator backend now avoids one concrete source of repeated allocation: the
full semantic selector runtime is evaluated once per execution context instead
of being concatenated into every retry. Its fixed counters make installations,
compact evaluations, and navigation-triggered reinstalls observable. This is a
verified resource-boundary improvement, not yet a substitute for the complete
RSS/allocation workloads required by DT-003 and DT-060.

Tracing follows the same composition rule. The base optional capability is a
BrowserContext-scoped, core-only recorder for API metadata, balanced groups,
and chunk lifecycle. DOM/ARIA snapshots, screenshots, screencast, network
bodies, HAR, sources, console, and video are separate contributors; Playwright
exposes them through one tracing facade but its server internally owns distinct
snapshotter, HAR, screencast, filesystem, and Artifact machinery. Pwright does
not make that aggregation implicit. The base recorder has fixed entry/group/
chunk count and byte limits, charges retained payloads to `TraceBytes`, starts
no task and performs no I/O. A caller-selected bounded sink owns filesystem,
archive, or remote-stream policy. See `knowledge/pwright-tracing.md`.

That base is now split into two portable crates. `pwright-trace-model` owns the
bounded Idle/Started/Recording state machine and non-cloneable detached chunk;
its production profile declares one session label, two chunk labels, one
payload ceiling, one scope, and the exact maximum lease count. The detached
chunk shares only a task-free release gate with its recorder, preventing a
second chunk or session from overlapping the declared peak.
`pwright-cap-tracing` owns the generated BrowserContext-only 1.0 contract for
start/start-chunk/group/group-end/stop-chunk/stop. Stop-chunk chooses discard
or an injected sink and returns only numeric summary metadata, so base tracing
does not accidentally authorize Artifact, ZIP, filesystem, or bulk trace
transfer behavior.

The next three optional crates keep transport ergonomics separate from the
recorder. `pwright-codec-tracing` implements bounded binary round trips for all
six commands and diagnostics, rejects attachments, and has no trace-body wire
variant. `pwright-client-tracing` contributes only a typed
`BrowserContext::tracing()` facade. `pwright-server-tracing` is the generated
six-service handler chain. Merely installing any of them still creates no
recorder, sink, runtime, browser connection, or task; those appear only in an
explicit composition.

`pwright-tracing-service` is that first explicit composition. It uses one
fallibly preallocated, count-bounded Context vector and one caller-injected
recorder per registered BrowserContext. A caller-injected monotonic clock
timestamps groups and contributor entries; a caller-injected synchronous sink
receives the non-cloneable chunk. The same service instance implements all six
generated command services. Context teardown aborts and releases the recorder
before invoking the sink's required `abort_context`; connection teardown has a
corresponding `abort_all`. Sink cleanup errors remain visible without keeping
base trace memory alive. This crate still selects no executor, task,
filesystem, archive, or browser backend.

`pwright-trace-format` is the shared PWRTRACE v1 size/writer leaf. Its Artifact
path calculates the exact size and reserves a caller-supplied `ArtifactBytes`
lease before allocating, returning that same owned buffer without a staging
copy. `pwright-trace-sink-file` is independently selected above it and the
tracing service. It accepts only a fresh caller-granted directory, preallocates
its bounded Context accounting table, computes the complete encoded size before creating a
chunk path, streams to a create-new temporary file, syncs it, and atomically
renames it. Per-file, per-Context byte, per-Context chunk, Context-count, and
directory-path limits are separate from the recorder's `TraceBytes` budget.
The sink starts no task and never scans, takes over, rotates, archives, serves,
or deletes an existing directory. Its versioned `.pwtrace` stream is a private
sink contract rather than the control protocol or a Playwright Artifact.

`pwright-trace-sink-pull` provides the other minimal output policy: a bounded,
fallibly preallocated FIFO of complete chunks. It admits by global item count,
per-Context item count, per-chunk retained bytes, and aggregate retained bytes.
Queued chunks continue to own their original `TraceBytes` leases and recorder
gates. Popping transfers the whole non-cloneable chunk to the caller without
pretending its memory was released. Context/connection abort drains only chunks
still owned by the queue. A future Tokio, WebSocket, gRPC, or custom sender must
therefore declare and bound its own post-pop in-flight ownership; none is a
dependency of the pull sink.

Playwright's remote tracing path confirms that the next protocol boundary is
not an unsolicited trace push. A remote stop produces an optional Artifact;
the client opens a child Stream and drives bounded `read(size)` calls. Pwright
adopts that object/lifecycle shape as an independently installed
artifact-transfer capability. The new command-free `pwright-artifact-model`
provides the prerequisite: one immutable source lease shared by independent
Stream cursors, explicit EOF, one prepared read per Stream, rollback-on-drop
before cursor commit, and explicit close/abort. The generated
`pwright.artifact` 1.0 contract adds three Artifact commands and two Stream
commands on their exact stable object kinds. Its independent binary codec
requires one bounded attachment for every successful read, copies it into a
caller-supplied `ArtifactBytes` ledger, and represents empty EOF explicitly.
Its generated five-service server chain advertises the capability only when
composed. Typed base handles plus the separate `pwright-client-artifact` facade
add no hidden I/O. The task-free `pwright-artifact-service` preallocates bounded
Artifact/Stream tables, injects ID ownership, accounts source and retained
result overlap, and cleans Artifact subtrees child-first. None of those crates
adds filesystem, runtime, task, trace, or browser dependencies. The separate
`pwright-artifact-runtime` layer holds registry publication behind
controlled-runtime lifecycle preparations, verifies exact child-first disposal
topology, compensates a Stream whose Create fence fails, and uses the shared
ordered-control barrier so Create/Dispose is accepted by the transport before
command success. The raw registry remains task-free and is not a valid remote
lifecycle composition by itself.

Tracing production is now the separate `pwright.tracing_artifact` 1.0
capability rather than an expansion of tracing 1.0. Its generated contract,
binary codec, server handler, and client facade are separate crates. The leaf
`pwright-plugin-tracing-artifact` joins the recorder's public detach seam to the
shared exact-size encoder and runtime Artifact registry. It allocates Artifact
and Stream identities from the same connection-global source, transfers the
single owned PWRTRACE allocation without copying, and fences exact Artifact
`Create` before returning success. Encoding failure drops the chunk and opens
the recorder gate; control publication failure retires the unpublished runtime
and registry object and releases its bytes. This composition still owns no
filesystem, ZIP, Node.js, browser, executor, or task. See
`knowledge/pwright-artifacts-and-streams.md` and
`knowledge/pwright-tracing.md`.

`pwright-plugin-tracing-navigation` is a leaf above the portable service. It
uses the ordinary `Layer` contract to wrap only `navigation.goto`, receives a
Page-to-BrowserContext route explicitly, and emits correlated call-ID entries
only while a user-started chunk is active. This establishes the plugin rule:
contributors decorate selected semantic commands and share the recorder, but
cannot cause base tracing, another contributor, a browser backend, or a sink to
appear. Its generated-server-independent unit tests and dedicated-Chromium CDP
test prove the layer can wrap either an in-memory command service or a real
backend path.

Before implementation is approved, the design must define measurable budgets
for idle runtime RSS, per-connection and per-page overhead, remote-object table
size, event-queue depth, retained trace/network data, and browser-process
memory. Exceeding a bound must cause explicit backpressure, eviction, or a
diagnostic error rather than unbounded growth.

## Why the current boundaries fail

The current `pwright-bridge` owns unrelated concerns: CDP connection access,
tab lifecycle, HTTP fallback, concurrency policy, reference caching, content,
navigation, input, and the Playwright-compatible facade. The CLI then depends
directly on `pwright-cdp`, `pwright-bridge`, `pwright-js`, and `pwright-script`.
These edges make features technically separate crates but not independently
installable components.

## Composition model

V2 uses three primitives:

```text
typed Request -> Service<Request> -> typed Response
                    ^
                    |
               Layer<Inner>
```

- `Service<Request>` is a single typed capability contract.
- `Layer<Inner>` wraps a service with optional policy such as retry, tracing,
  timeout, rate limiting, recording, or tab leasing.
- `Stack` composes layers through ordinary Rust generics.
- `PluginInfo` supplies stable metadata for diagnostics.

This is compile-time plugin composition. Rust does not promise a stable ABI
for arbitrary dynamic libraries, so native `dylib` loading is not the default
plugin mechanism. Process plugins can be added later behind a separately
versioned protocol without contaminating the core.

## Proposed Rust architecture

The Rust version has four explicit layers:

```text
Playwright-shaped Rust API
Browser / BrowserContext / Page / Frame / Locator
                         |
                         v
Typed channel client and object handles
                         |
          +--------------+---------------+
          |                              |
          v                              v
  in-process dispatch             encoded transport
  typed Rust values               pipe / WebSocket
          |                              |
          +--------------+---------------+
                         v
Capability registry and runtime dispatcher
                         |
                         v
Browser backend: CDP first, other backends optional
```

The client/runtime boundary always exists, including in one process. However,
the in-process path does not serialize commands to JSON and does not duplicate
the complete runtime object graph. It passes typed Rust requests directly. A
wire codec is selected only when a process or network boundary exists.

### Public API objects

The public API follows Playwright's object hierarchy and naming where Rust
semantics allow it:

```text
Playwright
  `-- BrowserType
        `-- Browser
              `-- BrowserContext
                    |-- Page
                    |     |-- Frame
                    |     |-- Locator (client-side query value)
                    |     |-- Keyboard / Mouse / Touchscreen (views)
                    |     `-- Worker
                    |-- Request / Response / Route
                    `-- Download / Artifact
```

Remote resource objects are cheap typed handles containing an `ObjectId` and a
shared connection handle. `Locator`, selectors, and option builders are values,
not registered runtime objects. `Keyboard`, `Mouse`, and `Touchscreen` are
lightweight views over a `Page`; they do not consume object-table entries.
Requests and responses become registered objects only while a subscribed
network capability requires identity across events.

```rust
pub struct Page {
    object: Remote<PageKind>,
}

pub struct Remote<T> {
    id: ObjectId,
    connection: Weak<Connection>,
    marker: PhantomData<fn() -> T>,
}
```

The exact API signatures remain a separate design topic, but asynchronous
methods should resemble Playwright rather than expose CDP concepts:

```rust
let browser = chromium.connect(cdp_endpoint).await?;
let context = browser.new_context(ContextOptions::default()).await?;
let page = context.new_page().await?;
page.goto("https://example.com", GotoOptions::default()).await?;
page.get_by_role(
    AriaRole::Button,
    RoleOptions::new().with_name("Submit", true),
)?
.click(ClickOptions::default(), CallMetadata::default())
.await?;
context.close().await?;
```

### Typed channel protocol

The logical envelope follows Playwright while using numeric identifiers and
method tags internally:

```text
Command  { call_id, object_id, method, params, metadata }
Result   { call_id, value | error, diagnostics }
Event    { object_id, event, value }
Control  { create | adopt | dispose | cancel | subscribe }
```

`CallId`, `ObjectId`, `CapabilityId`, and method/event identifiers are distinct
newtypes. String names exist for diagnostics and external compatibility, not
for internal dispatch. Every command has a statically associated parameter,
result, and error-details type.

The initial codec may use Serde JSON for debuggability, but the protocol model
must not depend on JSON. Binary payloads use out-of-band buffers or binary
frames rather than base64 when the transport supports them.

Protocol metadata includes deadline, cancellation identity, user-visible API
name, source location when available, and trace correlation. It must not retain
full user stacks unless tracing is enabled.

### Capability registry

A capability is a compile-time Rust component, not a native dynamic library.
Each component declares stable metadata and registers its commands and events:

```rust
pub trait Capability: Send + Sync + 'static {
    const ID: CapabilityId;
    const VERSION: ProtocolVersion;
    const DEPENDENCIES: &'static [CapabilityId];

    fn register(self, registry: &mut Registry) -> Result<(), RegistryError>;
}
```

The registry rejects duplicate command ownership, missing dependencies,
dependency cycles, and incompatible protocol versions at startup. An umbrella
crate can provide Cargo features, but capabilities remain independently usable
without that crate. Runtime negotiation exposes the installed capability set
to remote clients; an unsupported method fails before browser interaction.

Capabilities may contribute API extension traits, typed channel messages,
runtime handlers, and backend adapters. They may not modify the core object
layout or depend on delivery surfaces such as CLI and gRPC.

### Runtime ownership and concurrency

One runtime owns each browser connection and is the sole writer to its backend
transport. It stores compact state records keyed by `ObjectId`; public handles
never own backend sessions directly. Command execution is ordered per target,
while an explicitly bounded scheduler permits parallel work across targets.

The runtime must not create one Tokio task per API object. Tasks belong to
connections and active operations. Events use bounded queues with a declared
overflow policy. Event production is enabled lazily on the first subscription
and disabled after the last subscription, following Playwright's subscription
model.

Cancellation is end-to-end: dropping a method future does not silently abandon
the backend operation. A cancellation token or explicit cancel control message
reaches the runtime handler, which performs operation-specific cleanup.

### Object lifecycle

Runtime objects form an owned tree. Creating a child requires a live parent;
closing a parent deterministically closes its descendants. Explicit `close` or
`dispose` is the authoritative lifecycle operation. `Drop` only schedules a
best-effort release and must never block.

Each object category has a configurable hard limit and retention policy. The
runtime reports live counts, peak counts, pending operations, queue depths, and
dispose reasons. It must not wait for a high global threshold before attempting
cleanup of short-lived handles.

### Browser providers and backends

Browser acquisition and browser automation are separate extension points:

```text
BrowserProvider                       BrowserBackend
  system executable                    Chromium CDP
  explicit executable path             future WebDriver BiDi
  existing remote endpoint             future Firefox/WebKit adapter
  optional managed download
```

The core API never assumes that Pwright owns the browser process. A provider
returns a connection description plus an optional process lease. Only a lease
owner may terminate the browser process. Connecting to an existing endpoint
must never close the user's browser when the Pwright client disconnects.

CDP types and sessions remain below the backend interface and cannot leak into
the Playwright-shaped public API. Backend-specific escape hatches live in an
explicit optional extension. The verified responsibility boundary is:

| Concern | Portable capability/runtime | Backend contract | Concrete adapter |
| --- | --- | --- | --- |
| Playwright request/result/error shapes | owns | references IDs/version | implements |
| Command support and native/emulated classification | consumes | owns manifest | declares |
| CDP/BiDi/session/target/native message types | never owns | never owns | owns |
| Provider discovery, launch, process ownership | never owns | never owns | never owns |
| Unsupported command rejection | receives typed failure | validates before dispatch | not called |
| Native escape hatch | no method | explicit generic trait only | supplies native type |

## Planned crate boundaries

```text
pwright-core                 dependency-free Service/Layer kernel
pwright-browser-backend      core-only backend identity/support manifests
pwright-browser-provider     dependency-free attach/launch ownership contracts
pwright-browser-install      explicit zero-dependency managed-install pipeline
pwright-provider-cdp         attach-only explicit CDP WebSocket provider
pwright-provider-chromium    opt-in installed/explicit Chromium process owner
pwright-backend-cdp-common   command-free shared CDP failure classification
pwright-backend-cdp-content  isolated attach-only Page screenshot adapter
pwright-backend-cdp-locator-screenshot
                             isolated capture orchestration with injected selector geometry
pwright-backend-cdp-selector-runtime
                             command-free bounded selector compiler and injected-page runtime
pwright-backend-cdp-locator  complete attach-only Locator action/read backend;
                             composes selector runtime and Page input, but no screenshot/runtime/process owner
pwright-backend-cdp-selector-geometry
                             first-party strict Chromium selector-to-geometry plugin;
                             no public Locator command authority
pwright-backend-cdp-evaluation
                             isolated attach-only value-evaluation adapter
pwright-backend-cdp-dom-content
                             isolated attach-only read-only DOM adapter
pwright-composition-chromium-evaluation
                             one-Frame generated evaluation server leaf
pwright-composition-chromium-content
                             one-Page generated screenshot server leaf
pwright-composition-chromium-locator
                             one-Page/Frame generated Locator 1.22 or 2.5 server leaf;
                             caller supplies CDP client and shared Page input owner
pwright-composition-chromium-context-lifecycle
                             generated Context.close server leaf over staged topology/runtime;
                             descendant retirement is one injected plugin
pwright-composition-chromium-handle-lifecycle
                             generated Handle.dispose server leaf with bounded
                             producer routes and injected native disposers
pwright-composition-chromium-locator-screenshot
                             Page/Frame capture leaf with selector plugin boundary
pwright-composition-chromium-dom-content
                             one-Frame generated DOM-content server leaf
pwright-composition-chromium-navigation
                             one-Page generated navigation server leaf
pwright-composition-chromium explicit attach-only production assembly; selected
                             capabilities, one resource plan, no implicit I/O
pwright-composition-chromium-launch
                             opt-in owned-process launch/bind/shutdown and
                             session-authorized remote-close leaf;
                             no dependency from the attach-only root
pwright-transport-ws         optional WebSocket transport
pwright-transport-http       optional Chrome debugging HTTP transport
pwright-protocol-cdp         typed CDP request/response adapter
pwright-codec-binary         shared bounded binary and schema primitives
pwright-schema-gen           development-only capability schema generator
pwright-codec-navigation     optional canonical navigation wire payloads
pwright-codec-locator        optional canonical locator-action wire payloads
pwright-codec-selector       command-free reusable recursive selector wire types
pwright-codec-screenshot     shared command-free screenshot attachment boundary
pwright-codec-locator-screenshot
                             optional bounded Locator screenshot payload/attachment codec
pwright-codec-browser-lifecycle
                             optional canonical Browser.close wire payloads
pwright-codec-browser-events optional fixed-size Browser.disconnected payload
pwright-cap-browser-lifecycle
                             core-only portable Browser.close contract
pwright-cap-browser-events   core-only Browser.disconnected event contract
pwright-client-browser-lifecycle
                             optional typed Browser.close client extension
pwright-client-browser-events
                             optional typed Browser event client helpers
pwright-server-browser-lifecycle
                             generated typed Browser.close handler binding
pwright-server-browser-events
                             one-shot terminal Browser event publisher
pwright-server-events        optional typed event producers and outbound lanes
pwright-server-locator       generated optional locator typed-handler chain
pwright-server-locator-screenshot
                             generated optional Locator screenshot handler
pwright-server-navigation    generated optional navigation typed-handler chain
pwright-screenshot-model     command-free screenshot values and retained bytes
pwright-selector-model       command-free immutable selector and matcher values
pwright-browser              explicit browser and tab handles
pwright-cap-navigation       goto/reload/history requests
pwright-cap-locator          locator command contracts and selector-model reexports
pwright-cap-dom              locator and DOM requests
pwright-cap-input            keyboard/mouse/touch requests
pwright-cap-content          optional Page screenshot request/result contract
pwright-cap-locator-screenshot
                             optional one-command Locator screenshot contract
pwright-cap-network          interception/cookies/download requests
pwright-plugin-retry         generic retry layer
pwright-plugin-timeout       runtime-specific timeout layer
pwright-plugin-tracing-navigation
                             optional correlated navigation API-call layer
pwright-trace-format         shared bounded PWRTRACE v1 size/writer and
                             zero-copy owned ArtifactBytes encoder
pwright-trace-sink-file      optional bounded synchronous atomic `.pwtrace`
                             file sink for a fresh caller-granted directory
pwright-trace-sink-pull      optional bounded task-free complete-chunk FIFO;
                             explicit ownership transfer to caller transport
pwright-plugin-leases        opt-in durable tab ownership
pwright-runtime-tokio        Tokio executor and timers
pwright-cli                  optional command-line composition
pwright-script               optional declarative scripting
pwright-server-grpc          optional gRPC delivery surface
pwright-fake                 optional behavioral test components
pwright                      no-default-features umbrella crate
```

Dependency direction is always downward toward `pwright-core`. Capability
crates do not depend on the CLI, server, script runner, or one another.

## Installation profiles

```toml
# Current repository form; the umbrella itself installs nothing by default
pwright = { git = "https://github.com/Shuozeli/pwright", default-features = false }

# Portable navigation contracts only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["navigation"] }

# Portable value-evaluation contracts only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["evaluation"] }

# Portable Page screenshot contract only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["content"] }

# Portable Locator screenshot contract only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["locator-screenshot"] }

# Portable read-only Frame DOM-content contract only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["dom-content"] }

# Portable JSHandle/ElementHandle disposal contract only
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["handle-lifecycle"] }

# Playwright-shaped navigation client, without a server or CDP
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-navigation"] }

# Playwright-shaped Frame/Page by-value evaluation without a server or CDP
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-evaluation"] }

# Playwright-shaped Page screenshot client without a server or CDP
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-content"] }

# Playwright-shaped Locator screenshot client without a server or CDP
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-locator-screenshot"] }

# Playwright-shaped Frame/Page DOM content without a server or CDP
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-dom-content"] }

# Playwright-shaped JSHandle/ElementHandle disposal without a server or CDP
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["client-handle-lifecycle"] }

# Attach-only Chromium navigation server, without a client or browser provider
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-navigation"] }

# Attach-only one-Frame Chromium evaluation server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-evaluation"] }

# Attach-only one-Page Chromium screenshot server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-content"] }

# Attach-only Chromium Locator screenshot server with injected selector resolution
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-locator-screenshot"] }

# Attach-only one-Frame Chromium DOM-content server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-dom-content"] }

# Attach-only bounded Chromium Handle lifecycle server
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["chromium-handle-lifecycle"] }

# Executor and byte-stream adapters remain separate choices
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["runtime-tokio"] }
pwright = { git = "https://github.com/Shuozeli/pwright", features = ["transport-tokio"] }

# Separate explicit managed-installation capability; no launch is implied
pwright-browser-install = { git = "https://github.com/Shuozeli/pwright" }
```

The umbrella deliberately has no `full` feature yet. Adding one before every
supported capability and policy boundary is classified would turn a temporary
subset into an accidental compatibility promise. Individual capability crates
remain usable without the umbrella.

## Invariants

1. All features are disabled by default.
2. Core contains no runtime or protocol choice.
3. Tab creation and closure remain explicit.
4. Optional policy is expressed as layers, not booleans on a central browser.
5. Capability request and response types are public and exhaustively typed.
6. No capability crate can introduce a dependency cycle.
7. Fakes implement the same services rather than a monolithic mock trait.
8. Attach-only remains the smallest distribution; installed-browser launch is
   a separate opt-in component, and managed downloading is not implied by it.
9. Managed download requires a separate crate, an explicit one-use permission,
   an exact package/integrity request, and direct pipeline invocation.

## Migration strategy

The exploratory `pwright-core` slice is isolated from legacy crates and does
not migrate current behavior. Protocol, lifecycle, browser-provider, and
memory-budget designs still precede a browser-connected vertical slice. After
those decisions are reviewed, a later implementation plan can connect one
capability end to end without rewriting v1 in place. Compatibility facades
should eventually be adapters over v2 services, not dependencies of the new
core.
