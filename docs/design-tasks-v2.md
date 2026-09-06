<!-- agent-updated: 2026-09-05T22:05:06Z -->

# Pwright Rust v2 design tasks

> Status: planning the design work. This document identifies what must be
> designed; it does not approve an architecture or implementation.

## Purpose

Before choosing crates, traits, schemas, or transports, we need an ordered set
of design questions. A design task is complete only when its alternatives,
decision, consequences, rejected options, and validation evidence are written
down. Producing code is not part of these tasks unless a deliberately approved
prototype is needed to answer one narrow question.

## Agreed workflow

Before approving Pwright v2 design decisions, we will study Playwright one
topic at a time. Each lesson must cover the user-visible capability, the
internal design model, relevant upstream source locations, important costs or
limitations, and questions raised for Pwright. Learning notes are evidence for
later design tasks; they are not architecture decisions by themselves.

## Implementation checkpoint

On 2026-09-01 the user authorized implementation to begin. The first bounded,
exploratory slice is recorded in `architecture-v2.md` and implemented as the
new `pwright-core` crate. It contains only:

- dependency-free typed `Service`, `Layer`, and `Stack` composition;
- capability/plugin metadata and protocol-version identifiers;
- preflight validation for duplicate capabilities, duplicate command owners,
  missing dependencies, incompatible versions, and dependency cycles.

The slice passed eight release-profile unit tests, release-profile Clippy with
warnings denied, and a normal dependency-tree check containing only
`pwright-core`. It does not connect to a browser and does not alter legacy v1
behavior.

This checkpoint starts implementation but does not mark DT-001 through DT-072
complete. Until the gates below are resolved, further code remains exploratory;
the next browser-connected slice still requires explicit protocol, lifecycle,
backend, resource-limit, and acceptance decisions.

The next foundation checkpoint adds codec-independent typed commands, results,
events, lifecycle controls, classified operation failures, a bounded object
ownership graph, a bounded pending-call table, and a bounded event queue. The
implemented behavior now proves:

- numeric call, object, cancellation, method, event, and object-kind identities
  cannot be accidentally interchanged by the Rust type system;
- command parameters, results, error details, and event payloads remain tied to
  their static specifications;
- children require a live parent, duplicate IDs fail, adoption cannot create a
  cycle, and parent disposal produces a deterministic child-first cleanup plan;
- live objects and pending calls have hard count limits;
- cancellation must match the call's registered cancellation identity and the
  first cancellation reason wins repeated races;
- event overflow is explicit through `RejectNewest` or `DropOldest`, including
  zero-capacity behavior and attribution counters;
- no container starts tasks, reads a clock, or grows its live item count beyond
  the configured limit.

The expanded `pwright-core` suite passes 29 release-profile tests, Clippy with
warnings denied, rustdoc with warnings denied, and still has no normal
dependencies. The complete existing workspace also passes its release-profile
test suite after adding the new workspace member; environment-dependent Docker
browser tests remain explicitly ignored as before. This remains a foundation
only: no codec, transport, async runtime, backend, browser process, or public
Page API is implemented yet.

The first backend-connected checkpoint adds three provisional crates:

- `pwright-cap-navigation` defines the portable typed `BeginNavigation`
  primitive and depends only on `pwright-core`;
- `pwright-runtime` dispatches typed commands in process without serialization,
  rejects stale target objects, reserves bounded pending-call slots, correlates
  results, and releases pending entries when a dispatch future is dropped;
- `pwright-backend-cdp` maps that primitive to the existing `pwright-cdp`
  `Page.navigate` implementation and is the only new crate in the slice that
  imports CDP and Serde JSON.

The four v2 crates pass 40 non-browser release tests, including one complete
fake path from `Command<BeginNavigation>` through the runtime and CDP adapter.
An additional explicitly ignored live test passed against the dedicated Chrome
CDP profile and confirmed real navigation plus exact cleanup through an owned
incognito BrowserContext.

The live run found that Chromium returned `success: true` for
`Target.closeTarget` while two default-context test targets remained visible.
The lower-level implementation now at least validates the returned `success`
field, but the v2 live test deliberately uses owned-context disposal and polls
`Target.getTargets` until the exact target disappears. This is evidence for
DT-032 and DT-050: command acknowledgement is not resource-termination proof,
and an owned BrowserContext provides a stronger cleanup boundary. The two test
targets from the failed attempts were removed with Pwright's verified HTTP
closer; unrelated browser targets were preserved.

This checkpoint still does not implement public `Page::goto`. Commit/load
waiting, main-resource Response identity, redirects, deadlines, cancellation,
and page/frame close races remain required before that compatibility claim.

The browser-provider ownership checkpoints add four provisional crates:

- `pwright-browser-provider` is standard-library-only and defines distinct
  attach and launch contracts. External attachments contain only a connection;
  launched browsers additionally contain an explicit, consuming process lease.
- `pwright-provider-cdp` accepts only explicit, validated `ws://` or `wss://`
  endpoints and returns an external attachment through an injectable connector.
  It contains no discovery, launch, install, download, cache, or termination
  path.
- `pwright-provider-chromium` is separately opt-in. It discovers an already
  installed browser or accepts an explicit executable, requires either a fresh
  empty profile or an explicitly marked persistent profile, forces an
  ephemeral loopback CDP endpoint, waits for the browser-owned
  `DevToolsActivePort`, and returns a CDP connection plus a consuming process
  lease. It contains no browser bytes, download, installer, cache, or Node.js
  driver.
- `pwright-browser-install` is a separate standard-library-only contract. An
  exact engine/revision/target/integrity request consumes an explicit one-use
  download permission and composes injected resolve, fetch, verify, and install
  stages. It contains no stage implementation and no provider depends on it.

The v2 foundation now passes 46 non-browser release tests. A second explicit
live test attached to the dedicated Chrome profile, called
`Browser.getVersion`, dropped the external attachment, reattached to the same
endpoint, and called `Browser.getVersion` again. This demonstrates that
transport disconnect does not terminate the externally owned browser. The
test introduced no browser context or target and issued no close command.

The explicit-launch checkpoint now covers system executable discovery,
explicit-path launch, process exit observation, startup cancellation/failure
cleanup, and consuming termination authority. Fake process/waiter/connector
tests prove exact provider-owned flags, loopback-only endpoint derivation,
connection-failure cleanup, already-exited observation, and that dropping a
successfully delivered lease does not implicitly kill the browser. The next
DT-050 checkpoint adds a composable shutdown coordinator: bounded CDP
`Browser.close`, independent bounded process observation, force-kill escalation,
bounded post-kill observation, and cancellation-safe cleanup. Process exit,
not command acknowledgement, is authoritative. Persistent profiles require an
opaque token initialized in an empty directory plus an exact versioned marker;
ordinary nonempty Chrome profiles remain invalid. The provider never deletes
caller-owned profile data. The final DT-050 checkpoint adds the independent
`pwright-browser-install` contract. An installation root must be canonical and
empty both when selected and immediately before resolution. Verification is a
typed mandatory stage before installation; the installed receipt has neither
launch nor termination authority. Compile-fail tests require explicit download
permission and reject process control on a receipt. Dependency inversion proves
that existing attach and launch providers cannot reach this crate. This
completes DT-050's provider, ownership, profile, shutdown, and optional managed
installation boundaries; concrete download/catalog/archive implementations and
failure rollback remain later operational/plugin work.

The DT-051 backend checkpoint adds `pwright-browser-backend`, which depends only
on core. A backend descriptor names its engine and native protocol, then maps
each supported portable command to its owning capability, minimum version, and
native or emulated implementation classification. `BackendManifest` validates
all mappings against the installed `Registry`; `BackendCheckedService` binds a
typed `CapabilityCommandSpec` at setup and rejects unsupported or mismatched
features before its inner adapter can run. `ChromiumCdpBackend` declares direct
begin-navigation as native and the composed goto/click/fill behaviors as
emulated. CDP access requires the explicit
`NativeBackendAccess<ChromiumCdpBackend>` trait on adapter-owned services.
Portable capability/client/model crates neither depend on `pwright-cdp` nor
name its identities or messages. This completes DT-051's responsibility and
escape-hatch boundary; actual Firefox/WebKit/BiDi adapters remain independent
future implementations rather than changes to portable command contracts.

The DT-052 checkpoint adds the bounded `CdpTopology` recovery state machine to
the concrete Chromium adapter. The composition root supplies public object IDs;
the adapter maps CDP contexts, targets, sessions, frames, and selected execution
worlds to BrowserContext/Page/Frame lifecycle effects without exporting native
identities into portable crates. A page target may race ahead of its context,
and iframe children may race ahead of their parent; both paths defer within
explicit limits and are discarded cleanly if their target detaches first.
Popup notification waits for initialized opener and popup pages while ownership
remains BrowserContext→Page. Main-frame IDs survive Chromium process swaps, and
local iframe plus OOPIF events share one Frame identity. Ambiguous OOPIF detach
requires a synchronization confirmation before disposal. Crashes mark the
exact page, session detach and context close clean exact subtrees, and
connection epochs prevent stale pre-reconnect events from deleting rebound
objects. Only main and Pwright utility execution worlds are retained, keyed by
session plus execution-context ID. Generated CDP event normalizers, runtime
`Complete`-topology integration, and AAA race tests make these rules executable.
The state machine chooses no event pump, executor, transport, provider, browser
binary, or hidden object-ID allocator.

The DT-060 shared-budget checkpoint adds a deny-by-default, connection-local
`ResourceLedger` to core. Fourteen count/byte classes cover objects, calls,
commands, events, wire frames, response bodies, logs, traces, screenshots, DOM
content, selectors, handles, and backend state. Global and per-scope limits are checked
atomically; multi-class failure rolls back completely. Scope and lease metadata
have independent bounds, stream leases resize without release races, and
non-cloneable RAII leases release when cancellation drops a future. The first
real integration makes budgeted WireFrames consume aggregate `WireBytes`;
WireFrame clones share immutable storage and one lease. The allocation table,
eviction matrix, metrics schema, browser/runtime measurement separation, and
remaining enforcement work are canonicalized in
[resource-boundaries-v2.md](resource-boundaries-v2.md). This is a DT-060
foundation checkpoint, not completion of the memory investigation or every
future producer integration. The next wire checkpoint covers the inbound Tokio
path: a declared envelope is reserved before body allocation, decoded bytes are
reserved before copying, and the two leases overlap during the real two-buffer
peak. Route attribution survives in the returned frame, while cancellation and
malformed-input paths release staging capacity automatically. Outbound encoded
and length-prefixed buffers are also reserved before allocation; a cancelled
partial write keeps its lease in transport-owned state until it is completed.
Typed event handlers now optionally reserve `EventItem` and decoded
`EventBytes` together in the source-object scope. Reliable overflow consumes no
new budget; lossy overflow uses an atomic lease replacement so failure leaves
the previously queued event intact. Pop, clear, handler teardown, and protocol
extraction release all queue-owned capacity.
Runtime composition can now install that ledger only before state exists.
Create reserves object-scoped `LiveObject` capacity before graph mutation;
Adopt preserves the lease, while child-first Dispose and transport close
release exact subtrees. Runtime call admission reserves call-scoped
`PendingCall` capacity before queue/backend work and retains it through
cancellation or abandonment until terminal guard cleanup.
Connection admission now adds `ActiveCommand` only after a target head receives
an aggregate backend-execution slot. Connection waiters remain bounded
`PendingCall`s without double-counting active work. Immediate grants reserve
before scheduler mutation; FIFO promotion atomically moves the existing active
lease to the next call scope, and completion, cancellation, or future drop
releases it. AAA tests cover saturation, waiter exclusion, unpolled acquisition
drop, promotion, and injected promotion failure.
The encoded client can now install that ledger through direct, control-enabled,
or negotiated channel factories. It reserves a call-scoped `PendingCall` before
correlation or send mutation. Normal results release at correlation; a dropped
possibly sent future keeps the lease with its tombstone through cancellation
send until a terminal late result or connection close. Send failure, explicit
close, EOF, receive failure, and driver drop drain the same resource map. AAA
tests prove rejection atomicity and every implemented terminal boundary.

The first production-composition gate now separates declared requirements from
configured capacity. Each component can publish a zero-dependency
`ResourceRequirements`; merged plans take the strongest requirement in every
dimension, while all installed components still contend dynamically in one
connection-local ledger. `ProductionResourceLedger` rejects a plan that
exceeds configured class/scope/lease limits, rejects attachment to any ledger
with current or historical use, and rejects a component claim omitted from the
declared plan. Runtime limits now derive their exact `LiveObject`,
`PendingCall`, realizable `ActiveCommand`, lifecycle `EventItem`, scope, and
lease requirements. Both dispatchers expose a production constructor that
must claim those requirements before state exists.

The runtime lifecycle outbox now keeps an `EventItem` lease beside every
retained control message. Create and Adopt reserve before object-graph
mutation; a child-first Dispose reserves its whole ordered message batch before
disposing any node. A rolled-back claim keeps the lease, while pop, commit,
clear, or transport close releases the exact entry. This makes a saturated
control channel a typed, atomic rejection rather than a partially applied
lifecycle transition.

The explicit-close prerequisite adds a controlled-runtime two-phase disposal
transaction. Preparation validates the live subtree, reserves the complete
child-first control batch and all `EventItem` leases, and then leaves objects
and calls live while caller-owned asynchronous backend cleanup runs. Only one
lifecycle transaction may be active, so Create, Adopt, Dispose, and a competing
preparation cannot invalidate that plan. Dropping the preparation rolls every
reservation back. Commit revalidates the exact subtree, invalidates it
child-first, transfers the reserved messages to the outbox, and wakes admitted
calls with `ParentClosed`; transport close remains authoritative, drains state,
and invalidates the outstanding token. This closes the runtime half of the
backend-before-local-close ordering. A later composition transaction must also
serialize the Chromium topology and native-context route before a public
`BrowserContext.close` capability can claim end-to-end atomicity.

The next context-lifecycle checkpoint now supplies that first end-to-end
server-side transaction. The generated, independently selectable
`pwright.context_lifecycle` 1.0 surface owns only method `0x0003_0001`,
`browser-context.close`, targeted at BrowserContext. Separate contract, binary
codec, client-extension, and generated server-handler crates keep it absent
unless selected. The codec rejects attachments and rejects timeout or
cancellation metadata; the client exposes a no-argument close and treats an
already-disposed or racing `TargetClosed` context as success.

The Chromium backend publishes a bounded portable-context/native-context route
and closes only `Owned` contexts with `Target.disposeBrowserContext`. Default
and externally attached contexts fail before CDP I/O. Production topology now
has its own owned rollback-on-drop candidate and transaction gate, so event
mutations cannot replace its route while CDP is pending. The composed close
service prepares topology and runtime, awaits CDP, commits topology first,
commits deterministic Frame→Page→BrowserContext runtime disposal, and only
then returns success. Injected CDP failure proves both local owners and the
control outbox roll back. A dedicated Chrome 152 live proof removed the exact
temporary context/target while `Browser.getVersion` proved the externally
owned browser remained alive.

The remote completion checkpoint now closes the result/lifecycle race. The
ordinary runtime-control outbound and close service share one bounded priority
writer. Its capacity is the declared supervisor in-flight command limit plus
the singleton outbound lane. Close acquires the barrier before CDP, and after
successful local commit it drains every preceding message through the target
BrowserContext Dispose before the typed handler may write Result. A complete
memory-transport proof feeds those exact frames to the real encoded-client
driver: Frame, Page, and BrowserContext are invalidated child-first while the
close future remains pending, and only the following Result wakes it. Separate
AAA tests prove barrier priority, exclusion, drop release, and hard-capacity
rejection. A remote profile that advertises context lifecycle without ordered
control is rejected before connection work.

The attachment checkpoint also removes the hand-wiring gap.
`AttachedChromiumComposition` owns `ProductionCdpTopology` behind the exact
shared mutex consumed by the close service. Its factory binds that topology to
the attached CDP browser session and a controlled-runtime lifecycle handle,
and the attachment admits only one production runtime owner. The live Chrome
test now constructs the declared production plan, uses `attach`, mutates the
shared topology, obtains the factory service, closes the owned context, and
proves the external browser remains alive.

The cancellation-safe continuation checkpoint closes the remaining explicit-
close ownership gap. The attached factory requires a caller-supplied
`ReservedCleanupExecutor`; it reserves a bounded slot before any browser work.
Capacity failure is a typed pre-CDP rejection. An unpolled future remains lazy
and releases the slot when dropped, while a started pending future transfers
the exact remaining close transaction to that reservation. The caller can
poll the queued job and inspect completion/failure metrics; the composition
does not choose an executor or start a hidden task. AAA tests prove transfer,
laziness, and capacity rejection, and the dedicated-Chrome proof exercises the
continuable attached factory.

The Page-lifecycle checkpoint follows Playwright's two-contract design rather
than hiding it behind one boolean. Generated optional capability
`pwright.page_lifecycle` owns Page-only `page.close` and
`page.run_before_unload`, with separate codec, client-extension, and generated
server crates. Ordinary Chromium close uses browser-session
`Target.closeTarget`, bounded exact-target absence confirmation, paired
topology/runtime preparations, synchronized Page-route retirement, and the
same reserved continuation plus remote lifecycle-before-Result barrier.
Before-unload uses the exact Page session's `Page.close`, returns after
initiation, and leaves Page/Frame identity live. Tests cover method selection,
confirmation failure rollback, unpolled and started-future Drop, composition
advertisement, Frame→Page→Result remote application, and client idempotence. A
dedicated Chrome 152 proof closed only the selected Page while preserving the
parent context and external browser; the context live proof now also proves
descendant route-table retirement.

The following Browser-lifecycle audit records why the next command cannot be a
generic CDP adapter method. Playwright's launched Browser close consumes
process shutdown authority, while `connectOverCDP` close only closes its
synthetic transport and a remote prelaunched dispatcher ignores stop/kill.
Pwright will preserve that public outcome but keep external disconnect in
connection control and permit owned close only in a launched composition that
holds the unique `BrowserProcessLease` plus explicit remote authorization.
The separately opt-in launched composition now exists and owns the first half
of that boundary: a plan/ledger-proven CDP connection plus the unique process
lease and bounded local shutdown. The portable `pwright.browser_lifecycle`
command is now implemented as four independent
capability/codec/client/server crates. Only the launch leaf can add it to its
handshake manifest and convert an owner into a non-cloneable grant after the
accepted server session negotiates it and proves it shares the launch's exact
production connection ledger; a denied conversion returns the untouched owner.
Its caller-reserved continuation, terminal whole-subtree disposal, and
ordered Page/Context/Browser Dispose-before-Result path are covered by unit,
encoded-memory, compile-fail, and real-Chrome tests. The optional close reason
is now codec-bounded and propagated as one shared terminal diagnostic to every
already-pending descendant operation while backend cleanup remains
structurally `ParentClosed`. Public Browser disconnected-event ordering is now
implemented. Client-side reason persistence is also bounded and root-shared:
the first-polled close installs one `Arc<str>` before dispatch, descendants
reuse it, and operations first attempted after disposal report it without
retaining the connection. Source anchors and the remaining contract are
recorded in `knowledge/pwright-browser-close.md`.

The first disconnected-event foundation is now independently packaged as
capability, fixed-size codec, client helper, and queue-free one-shot server
publisher crates. A split runtime-control fence can send all descendant
Dispose messages while leaving Browser Dispose queued, establishing the only
valid insertion point for `browser.disconnected`. The publisher treats a
failed or dropped polled send as terminal rather than retrying a possibly
partial frame. The attach profile now advertises this capability only with a
matching handshake, ordered control lane, and sufficient fixed-size frame
limits; its direct publisher is built from the accepted protocol and shared
production ledger without an event queue. One composition factory shares that
same at-most-once publisher among the native-CDP liveness lane, connection
termination cleanup, and the optional owned-close wrapper. Encoded tests prove
descendant Dispose -> disconnected -> Browser Dispose -> Result, while native
loss proves the same order before ordinary connection cleanup.
The ignored-by-default live launch proof now negotiates lifecycle, Browser
events, and control against a separately spawned Chrome 152, verifies the full
five-frame terminal order, then polls native liveness and confirms that cleanup
coalesces the same close without a duplicate event or surviving process.

The BrowserContext-creation checkpoint implements the empty-options subset of
`Browser.newContext`, not the full Playwright options surface. Its design makes
creation depend on the already-implemented context lifecycle capability and
identifies two reusable prerequisites: rollback-on-drop prepared runtime object
creation, and bounded client identity that distinguishes a wire-announced
object from the one later claimed by a typed result. Native creation uses
`disposeOnDetach`; post-CDP publication failure must compensate by disposing
the exact new context. The four-owner transaction, non-reusing object-ID rule,
remote Create-before-Result fence, and acceptance gates are recorded in
`knowledge/pwright-context-creation.md`.

The Page-creation checkpoint extends that inverse lifecycle design to the first
related-object transaction. Optional capability `pwright.page_creation` owns
only BrowserContext `new_page` and depends on Page lifecycle. Its portable
result initializes a Page and its required main Frame together; CDP target,
session, context, and frame strings remain backend-private. Chromium creates
`about:blank` in the exact native context, uses a flattened target session, and
accepts only a bounded parentless `Page.getFrameTree` root as initialization
proof.

Runtime `PreparedObjectCreationBatch` validates and reserves Page→Frame graph,
`LiveObject`, and ordered lifecycle capacity without publishing either object.
Composition prepares the matching topology and Page route while their readers
are excluded, then commits runtime Page→Frame ownership before infallibly
publishing both prepared local views. Any post-target failure closes the exact
target and confirms bounded absence. A started operation abandoned by its
caller transfers to a pre-reserved, caller-owned continuation; exhausted
capacity rejects before ID allocation or CDP.

Remote completion physically orders Page Create, Frame Create, then Result.
The client claims both prior announcements as one state transition, so a
missing, mismatched, disposed, duplicate, or over-capacity second initializer
cannot expose a half-created Page. The dedicated Chrome 152 proof exercises the
production attach/runtime/factory path, verifies `about:blank`, main-Frame and
Page-route publication, closes Page and Context through their typed lifecycle
services, and confirms the externally owned browser survives. The detailed
contract and evidence are recorded in `knowledge/pwright-page-creation.md`.

The value-only evaluation checkpoint now implements the approved boundary.
Playwright's
Page convenience delegates to its main Frame, while by-value evaluation and
live JS/Element Handles remain different protocol operations. Pwright
therefore makes `pwright.evaluation` independently installable and leaves Handle
identity, disposal, and retention to a later dependent capability. Its tagged
value graph, Chromium main/local/OOPIF execution-world routing, bounded browser-side serializer,
typed JavaScript failures, production resource peak, explicit non-goals, and
acceptance tests and implementation evidence are recorded in
`knowledge/pwright-value-evaluation.md`. The separate capability/model,
generated schema/reference/server surface, bounded binary codec, Page/Frame
client extension, Chromium backend, production factory, exact session plus
`uniqueContextId` route, and composition selector are all present. Frame route
state is transactionally replaced with topology and is resource-declared only
when evaluation is selected. Fake, remote-wire, and dedicated Chrome
152 tests prove that no Handle object or lifecycle message appears; the live
production route also reaches a real cross-site OOPIF through its own session
and globally unique execution context. This
advances DT-020, DT-021, DT-031, DT-033, DT-040, DT-043,
DT-051, DT-052, DT-060, and DT-070 without treating Handle semantics as solved.

The explicit Handle-lifecycle checkpoint now establishes the first dependent
foundation without changing value-only evaluation. `pwright-browser-model`
adds stable `JsHandle` and `ElementHandle` kinds under `Frame`, while the
independently installable `pwright.handle_lifecycle` capability owns only
`handle.dispose`. Its generated schema, bounded binary codec, generated server
handler, and opt-in client extension are separate crates. Rust `Drop` still
sends no I/O; the explicit async request has no action timeout/cancellation
metadata and treats a locally stale handle or a racing `TargetClosed` response
as idempotent success. The Chromium adapter binds a bounded native object ID to
its exact CDP session and sends exactly one `Runtime.releaseObject` with no
hidden retry. Child-first client invalidation proves that disposing a Frame
family makes both Handle types locally stale. See
`knowledge/pwright-handle-lifecycle.md`.

The next Handle-evaluation checkpoint implements a bounded creation and
explicit-disposal foundation. The independently installable
`pwright.handle_evaluation` capability depends on Handle lifecycle and owns
`evaluation.evaluate_handle` without changing value-only evaluation. Its
schema, binary codec, generated server binding, and Page/Frame client extension
produce a portable initializer with concrete JSHandle or ElementHandle kind;
no CDP object ID crosses the wire. Remote client materialization requires the
exact prior Create announcement.

The Chromium leaf evaluates with `returnByValue: false` in the route's exact
session and `uniqueContextId`. Composition commits runtime Handle ownership and
a bounded native route as one synchronous publication step, compensates a
failed publication with native release, and disposes by releasing the same
native object before committing runtime removal. A dedicated Chrome 152 proof
creates and releases a DOM ElementHandle, proves primitive JSHandle release is
a no-op, and leaves the external browser alive. This advances DT-022, DT-031,
DT-040, DT-043, DT-051, DT-052, DT-060, and DT-070.

The additive 1.1 checkpoint now implements Handle-valued arguments without
adding Handle knowledge to `pwright.evaluation`. A separate recursive graph
supports nested portable Handle leaves, retains the original depth/node/entry/
string/binary bounds, and deduplicates native arguments in first-appearance
order. The 1.0 schema is frozen as its compatibility baseline; a connection
that negotiates 1.0 can still use value-only `evaluateHandle` but its codec
rejects Handle leaves. The Chromium service resolves all referenced IDs under
one route lock, atomically prepares output lifecycle ownership to exclude
concurrent disposal, and requires the same Frame, Page, CDP client, and current
world before native I/O or publication. Missing/disposed arguments are
`TargetClosed`; cross-Frame arguments are protocol/value failures. The backend
uses exact-context `Runtime.callFunctionOn` and never serializes native IDs into
source text. Chrome 152 proves an ElementHandle can enter a second call and
produce `SECTION`. Generic JSHandle context semantics are implemented. The
cross-world ElementHandle decision now fixes Playwright's Element-only rule,
Chromium describe/resolve mapping, temporary ownership, and the prerequisites
that must close before public composition. Its low-level CDP primitive is now
implemented with pre-I/O JSHandle/cross-session rejection, malformed-result
compensation, and a dedicated Chrome 152 main-to-isolated-world proof. Numeric
plus stable Frame-world routes and persistent Handle source-world identity are
now implemented with same-session replacement rejection before I/O. An opt-in
production adoption batch now prevalidates and pre-reserves the complete
temporary-object peak, owns exact `RemoteHandle` and backend-byte leases, and
exhaustively compensates partial failure. Its composition wrapper reserves
caller-owned cleanup capacity before adoption and transfers dropped adoption,
result-owner, and release futures to that continuation. Eight new AAA tests and
a second Chrome 152 main-to-isolated-world proof return every resource and
reservation to zero. The default command installation remains exact-world; a
separate `install_with_element_adoption` path now declares the additional peak
and composes pre-I/O argument classification, temporary substitution, Runtime
execution, topology revalidation, and exhaustive release. Three command-level
AAA tests cover child-Frame success, JSHandle rejection, and an OOPIF-shaped
different-session rejection. A third Chrome 152 adoption proof passes a real
child-Frame ElementHandle into its parent through the public service and
returns every owner to baseline. A fourth Chrome 152 proof creates a real
auto-attached OOPIF ElementHandle, rejects it before temporary-resource
admission, and preserves external Chrome. Chrome's OOPIF frame-tree root parent
shape is now accepted only for non-Page sessions, with positive and negative
AAA coverage. Aggregate migration now uses the tested live Frame/world resolver
for main, local-child, and OOPIF routes and the lifecycle leaf's two-phase
connection, parent, and topology retirement owners. The complete recipe has
moved its Handle route registry and all six Handle 1.2 commands to the
independent leaves; the older aggregate route/command types remain available
only for source compatibility. Cross-world adoption is now an explicit
aggregate backend option that contributes its own temporary-object resource
peak without adding another negotiated capability. The
Handle-local design is
now locked as the additive `pwright.handle_evaluation` 1.2 surface: five
commands cover `evaluate`, `evaluateHandle`, `jsonValue`, `getProperty`, and
`getProperties` on both Handle kinds, with atomic property-batch publication,
bounded browser-side enumeration, and inherited Frame/world ownership. See
`knowledge/pwright-handle-operations.md`. The 1.2 schema, generated server,
binary codec, typed client, bounded CDP backend, optional composition, ordered
Create-before-Result fences, cancellation cleanup, and real-Chrome duplex proof
are now implemented. Cross-world ElementHandle adoption remains an independent
compatibility decision.

The independent `pwright-composition-chromium-handle-evaluation` leaf now
implements the six-command 1.2 direct server composition over the smaller
lifecycle leaf. It adds bounded Frame/world routes, pre-CDP lifecycle
reservations, atomic single/batch runtime-plus-route publication, native
compensation, and caller-owned abandoned-result cleanup. Nineteen direct tests and a
dedicated Chrome 152 test pass, while a 24-package exact allowlist proves the
normal closure omits aggregates, topology, Context/Page commands, clients,
providers, launchers, installers, Tokio, Node.js, and browser bytes. The
backend-neutral `pwright-plugin-ordered-handle-delivery` now composes the
remote boundary from the leaf's exposed protocol/raw services: bounded
pre-admission, ordered single/batch Create-before-Result, physical Result-send
ownership, abandonment cleanup, and ordered Dispose. Six plugin AAA tests and
a 15-package exact allowlist cover that opt-in policy without adding it to the
direct leaf. The dedicated Chrome 152 proof now exercises the combined leaf and
plugin, including physical Create-before-Result and Dispose delivery.

The Handle route
table now has declared `RemoteHandle` and exact retained-byte ledger ownership,
current/peak and release-reason metrics, pre-CDP capacity rejection, and
prepared descendant retirement integrated with explicit Page and owned-Context
close. Route ownership by Frame/Page also folds main-world replacement, Frame
detach, Page crash, and reconnect into the topology/runtime lifecycle batch.
An opt-in Handle-aware runtime target retires all remaining routes and leases
when the Pwright connection closes. The complete application recipe now makes
that wrapper its concrete runtime type and delegates the common cancellation
and ordered-control traits through it. A caller-reserved continuation rejects
before CDP when saturated and completes plus disposes a Handle created after
its direct command future is abandoned.

The production root now makes lifecycle and evaluation explicit optional
profiles. Evaluation requires lifecycle and remote Handle selection requires
ordered control, but value evaluation remains absent unless separately
selected. Lifecycle owns the shared Handle-route budget, evaluation owns its
own CDP route budget, and preparation rejects mismatched limits, scope reuse,
or runtime capacity below topology plus maximum Handles. Handshake selection
then installs only the accepted codecs, backend routes, and generated handlers.
Its default remote-handler factory delegates creation, physical Result delivery,
and Dispose ordering to `pwright-plugin-ordered-handle-delivery`; the aggregate
keeps its older public decorators only for source compatibility, not as a
second active delivery policy. Dropping while Result send is pending performs
one ordered Dispose, and failed Create publication compensates the native
object. The all-capability profile and static handler chain now include both
Handle plugins; its runtime installer requires the lifecycle-leaf route table,
uses the evaluation leaf for both Frame and Handle-local commands, and makes
connection-close cleanup unskippable. Chrome 152 verifies the expanded
13-protocol handshake and that
external browser ownership is preserved. A second duplex Chrome 152 proof now
negotiates Handle evaluation 1.1, drives lifecycle Create messages into the
typed client, passes the returned ElementHandle through a second encoded
command, and completes graceful close without taking browser ownership. The
codec preserves the channel-generated cancellation ID while still rejecting
unsupported timeout metadata. See `knowledge/pwright-handle-evaluation.md`.

The Chromium topology-event checkpoint now replaces manual Page/popup and
child-Frame/OOPIF publication with optional production components. It is deliberately
caller-driven: selecting it declares one fixed session/decode memory profile,
while every `receive_once` initializes newly visible target-local sessions,
consumes at most one bounded broadcast event, and commits through the existing
topology/runtime/route transaction. A separate Page-discovery profile installs
browser-root flattened auto-attach and its Chromium ordering barrier without
adding a portable capability or task. It enumerates bounded non-default
contexts, represents the unique default Context without an invented native ID,
initializes the complete existing frame tree while a target is paused, resumes
it explicitly, then atomically publishes Page, frame tree, and Ready state. It
returns `PageReady`/`PopupLinked` in the same ordered update.
Generated Page/popup, local Frame, OOPIF target, execution-world, detach,
root-navigation, and Page-crash events are normalized.
Fake races prove existing/new Page admission, popup gating, foreign-context
rejection, malformed-tree rollback, local-to-OOPIF identity continuity, late
old-world destruction, two-source detach synchronization, sticky lag,
same-connection authoritative Context/Page/Frame reconstruction, and
stale-epoch rejection. A Chrome 152 production-composition run discovers its
OOPIF and unique main world without test-side child-event parsing, then
evaluates through the portable child Frame. Repeated Chrome 152 discovery runs
also prove the browser's opaque default-context ID, an existing target, popup
unpause/link ordering, a 1,100-event overflow of a 1,024-slot ring, ObjectId
preservation across new sessions, cleanup, and external browser survival. This advances
DT-032, DT-034,
DT-052, DT-060, DT-061, and DT-070. Browser/default-context reconstruction,
Page/popup discovery, initial full-tree setup, live-transport event-loss
recovery, and the optional portable `BrowserContext.page`/`Page.popup`
contracts, codecs, subscriptions, and bounded publication bridge are
implemented. The bridge can also hold ordered control through the final
required `Create` before admitting a discovery event. A caller-owned topology
pump now applies a bounded terminate/reconstruct policy, rejects ambiguous
same-connection replay when portable discovery callbacks are installed,
detects an abandoned polled turn, and terminates the supervisor as
`BackendEventFailed` through both cleanup barriers. The supervisor now polls
that pump as a retained fourth lane beside its unique reader, bounded command
set, and serial outbound lane with rotating priority. Its complete application
run performs reasoned cleanup and write-half shutdown while preserving dual
failures. Constructing a new remote Pwright transport together with a fresh CDP
attachment, runtime object graph, and unambiguous event view remains open, so
DT-052 and DT-061 are not complete. Native CDP sessions will not be replaced
inside an existing Browser graph.
The boundary is recorded in
`knowledge/pwright-chromium-topology-events.md`.

The next DT-052/DT-061 checkpoint resolves an earlier ambiguity by inspecting
the local Playwright sources. Chromium protocol closure is terminal there:
`CRConnection._onClose` disposes the root session, `CRBrowser._didDisconnect`
closes Pages and workers, Browser closes Contexts and emits `disconnected`, and
`BrowserDispatcher` closes and disposes the channel object. Pwright now exposes
one owned `CdpConnection::closed()` future and an independent,
caller-polled `ChromiumCdpLivenessBackend`. It makes native closure a terminal
supervisor backend failure even when no topology-event capability is selected.
A generic `ConcurrentServerBackendChain` rotates ready backend plugins fairly,
never cancels their retained turns, and forwards terminal cleanup to both.
`AttachedChromiumComposition::server_backend` is the canonical constructor and
always composes mandatory liveness with the caller's optional plugin.
Tests prove sticky connection closure, sticky topology-stream closure, plugin
fairness, and release of a pending topology turn when liveness terminates the
connection. Transparent CDP WebSocket replacement was removed: a new native
attachment must create a new Browser/object graph. Establishing a complete
fresh remote Pwright client attachment, runtime graph, and event view remains
open, so DT-052 and DT-061 are not complete.

The following checkpoint adds the reusable `ChromiumAttachFactory` above the
one-shot preparation. Factory construction validates the capability profile,
complete per-connection resource limits, and Browser-root ObjectId namespace
without I/O. Each `prepare` creates a distinct production ledger and connector;
each `attach` consumes that fresh preparation to create a distinct CDP
connection, topology, ObjectId source, and epoch-1 history. Tests compare ledger
identity across two preparations and prove invalid capability selection and an
exhausted root namespace fail before CDP. The factory intentionally stores no
runtime, topology, connection, event receiver, task, or browser process.

The next staged checkpoint now binds that attachment to a caller-built
production transport. `attach_remote` rejects a missing handshake before CDP,
passes the fresh ledger into the one-shot transport-builder plugin, verifies
the returned transport's exact plan and ledger identity, constructs the
handshake-first supervisor, and claims the attachment's only topology-event
driver. Consuming `install_runtime` accepts a caller-selected runtime installer
but enters `ChromiumRemoteConnection` only when the attachment confirms a real
one-shot runtime was installed. The final unit owns the fresh CDP attachment,
remote supervisor/transport, runtime graph, and optional unique topology-event
baseline without choosing a task, listener, executor, deadline, cleanup, or
capability service stack. Tests prove exact transport acceptance, foreign-ledger
and plan-drift rejection, and missing-handshake failure before endpoint I/O.
Dedicated Chrome 152 release tests also prove the staged runtime has only its
fresh Browser root, a second topology-driver claim is rejected, and a no-op
runtime installer cannot produce the final connection state.
Post-handshake typed event publication and complete handler/backend assembly
remain open, so DT-052 and DT-061 are not complete.

This is executable evidence toward DT-032, DT-033, DT-050, and DT-061, not
their completion. The optional Tokio executor now supports pre-reserved
continuations plus a caller-selected nonzero job deadline. Deadline expiration
drops the stuck job, releases its permit, records `timed_out`, and lets an owned
Chromium shutdown's kill-on-drop guard escalate; runtime shutdown records the
distinct `cancelled` outcome. Shutdown-wide graceful drain, retry policy,
application reporting callbacks, and end-user API stabilization remained open
at that checkpoint.

The application-policy checkpoint now closes the executor supervision gap.
Shutdown atomically closes new admission and drains both preaccepted
reservations and running jobs; an optional nonzero drain deadline reports
outstanding counts without cancelling them, and eventual completion publishes
one drained transition. Retry is opt-in per logical job and requires a factory
for a fresh future on each finite attempt, so the executor never retries a
unique consuming continuation such as owned Browser shutdown. One permit spans
the complete attempt/backoff sequence. A synchronous callback reports
admission, retry, terminal, and shutdown events; callback panics are contained
and counted. Deterministic Tokio tests cover shutdown/admission races, retained
reservations, retry accounting, deadline then eventual drain, and callback
failure. A launch-composition test proves a dropped, already-started
`Browser.close` continuation is included in application shutdown and empties
the runtime tree before drain resolves. End-user API stabilization remains
open, so this is still opt-in foundation rather than a frozen public v2 release
promise.

The wire production checkpoint now calculates that previously open overlap.
`FrameLimits` first derives the largest structurally attainable frame body.
The stream envelope profile then optimizes attachment count together with its
header cost and shrinking body room. One outbound operation includes its
budgeted frame, encoded envelope, and length-prefixed pending write; one inbound
operation includes its envelope and decoded frame; both are added for the
independent full-duplex reader/writer case. Production Tokio construction must
claim the resulting `WireBytes`, scope, and lease profile.

Budgeted frames now retain the identity of their exact connection ledger even
when they contain zero bytes. Production send rejects raw frames and frames
from another ledger before envelope allocation. The codec-neutral
`WireProtocol` also has a production claim and stamps its output with that
identity. This closes the protocol/stream production gate without making core
depend on Tokio.

The result-handoff audit found that the earlier protocol reserved only after a
codec had already allocated its payload. Production encode now reserves twice
the structurally attainable frame size before invoking the codec, explicitly
covering the bounded typed source and encoded buffer overlap. Successful encode
atomically shrinks the same lease to exact payload/attachment bytes and moves
it from the staging connection scope to the final call/object scope. Codec,
frame-validation, and resize failures drop the reservation. Tests inspect the
ledger from inside the codec to prove admission precedes allocation and prove
failure release. Direct in-process results returned to application code are
caller-owned; the remote runtime does not retain a separate result history.

The encoded-client production checkpoint adds a transport-plugin marker rather
than specializing the client for Tokio. A `ProductionFrameTransport` exposes
its complete profile and actual ledger. `ClientWireProductionProfile` combines
that profile with directional frame maxima and the pending-call bound. It
covers both full-duplex I/O with remaining queued frames and the distinct state
where encoding precedes a rejected registration against a full pending table.
Production direct, cancellation-control, negotiated, and negotiated-control
constructors claim the combined plan and reject a transport from another
connection ledger.

Typed event queues now have their own production profile. Capacity determines
`EventItem`, scope, and lease maxima; capacity multiplied by an explicit
maximum decoded size determines `EventBytes`. The existing event-specific
sizing callback is checked against that local declaration before ledger
mutation, preventing one plugin from exceeding its contract by borrowing
another plugin's unused global capacity. Zero-byte events still own an item
lease, and omitted event plans fail during construction.

Chromium topology now has the next production profile. Its derived byte ceiling
counts all duplicated native strings across bidirectional indexes, pending and
materialized page/frame records, session owners, crash state, and execution
worlds. Before a production mutation clones the pure topology, it reserves a
maximum-sized shadow; successful application shrinks that lease to the exact
retained string capacity and atomically replaces the old state/lease. Failed
topology or resource admission leaves the live map unchanged. Inputs are
compacted before retention, closing the short-string/large-capacity loophole.
The composition root must assign an extension scope; non-extension attribution
is rejected before construction.
AAA tests cover deferred materialization, crash/world inventory, detach,
reconnect, exact release, undeclared plans, and contended shadow rejection.

The locator backend audit found no variable-size long-lived service state; it
retains only a client handle, duration, `OnceCell<()>`, and atomic metrics. Its
active action future is the real owner of selector/value input, display label,
generated JavaScript, and possible runtime-install source. The new
`CdpLocatorProductionProfile` derives a conservative transient peak and scales
it by an explicit concurrency limit. Production calls validate spare string
capacity, acquire a clone-shared local slot, and reserve `SelectorBytes` before
returning a future. An unpolled future is charged, drop/completion releases, an
overprovisioned shared ledger cannot bypass the plugin's local concurrency, and
an omitted plan fails construction. Other backend services and future
capability factories remain DT-060 work.

The navigation DT-060 checkpoint closes the action-local growth paths found by
the audit. `CdpNavigationLimits` bounds input/result URLs, native identifiers,
simultaneously retained request IDs, response-header count and aggregate bytes,
and status/error text. `CdpNavigationProductionProfile` derives a conservative
per-action `BackendStateBytes` peak and scales it by an explicit concurrency
limit. Production calls acquire a clone-shared local slot and reserve the full
peak synchronously before returning a future. Request IDs are checked before
set insertion; response header count, types, and aggregate bytes are checked
before owned copying; retained strings are compacted. Tests prove construction
failure for an omitted plan, unpolled and polled drop release, completion
release, local concurrency enforcement despite an overprovisioned ledger, and
failure-before-retention for request and header floods. Result ownership after
backend completion remains a distinct DT-060 boundary, so the remote production
server profile remains intentionally incomplete.

The following raw-CDP checkpoint closes the transport-side event queue and
pending-command paths. `ConnectionConfig` now bounds pending commands, channel
items, command/response/event/error/WebSocket bytes, and rejects zero or
inconsistent capacities instead of allowing a channel panic. Command
registration and the production `WireBytes` reservation happen synchronously;
an RAII guard removes abandoned entries, and connection shutdown drains them
even when cancellation drops the reader loop. Bounded JSON serialization stops
before an oversized command is retained. Outgoing queue items share the command
lease, so caller cancellation cannot release bytes the writer still owns.

Events share one immutable JSON payload rather than deep-cloning per subscriber.
In production each unique event carries one exact shared
`EventItem`/`EventBytes` lease; retaining a delivered value consumes the finite
budget and makes the connection fail closed. `CdpConnectionProductionProfile`
declares the fixed reader overlap, concurrent command peaks, event capacity,
and one allocation-before-admission transient. Legacy v1 keeps the Tokio
broadcast receiver. The opt-in v2 path adds a task-free, preallocated fanout
that drops events with no subscriber and reclaims each slot after every current
cursor advances. Its subscriber capacity and fixed bookkeeping are explicit;
the Chromium composition can derive the former from selected plugins and
reject an undersized plan before CDP I/O. The attach-only
`ProductionCdpConnector` verifies plan inclusion and exact config/profile
identity before network access. Raw connection and live connector constructors
remain explicit compatibility paths.

The runtime execution-control checkpoint turns command metadata into enforced
behavior while keeping the generic runtime executor-independent:

- `OperationContext` and `CommandCancellation<Command>` provide a typed,
  backend-neutral cancellation hook without retaining command parameters;
- `ControlledDispatcher` races operation completion, a validated external
  cancellation signal, and an injected relative-deadline future;
- an accepted cancellation wakes the dispatch future, drops the operation
  future, awaits backend cleanup, and only then removes its pending-call entry;
- wrong cancellation identities do not wake or alter the running operation,
  and the first accepted reason wins cancellation/deadline races;
- observable operation completion wins if it is ready in the same poll;
- cleanup failures are classified separately as `Cleanup`;
- `DirectDispatcher` rejects execution-control metadata instead of silently
  ignoring it;
- `pwright-runtime-tokio` supplies an explicitly installed Tokio timer adapter,
  while `pwright-runtime` continues to depend only on `pwright-core`.

The CDP navigation adapter now implements its portable cancellation hook using
`Page.stopLoading`. The v2 foundation passes 58 non-browser release tests plus
the compile-fail ownership doctest. A third explicit live test used a hanging
HTTP socket bound to this Mac's Tailscale interface: real Chromium began a
pending `Page.navigate`, the runtime accepted the matching cancellation ID,
the CDP cleanup completed, the command returned `Cancelled`, the pending table
returned to zero, and the owned incognito context and exact target disappeared.

This is executable evidence toward DT-033 and DT-061, not completion of either.
Dropping a future still cannot await backend cleanup without a bounded
connection-level cleanup executor; transport-close fan-out, nested deadline
inheritance, and shutdown draining remain future slices.

The lifecycle and target-admission checkpoint joins the previously separate
object graph, pending-call table, and command dispatcher:

- controlled subtree disposal first computes a deterministic child-first plan,
  removes the objects, marks their active and queued calls `ParentClosed`, and
  wakes those calls so admitted operations run their backend cleanup hook;
- lifecycle cancellation already accepted by the runtime wins a simultaneous
  operation-result race, while caller cancellation and deadline expiry preserve
  the earlier operation-completion-first rule;
- the direct dispatcher refuses disposal of a subtree with pending calls rather
  than removing objects it cannot safely clean up;
- a standard-library-only target scheduler admits one command per target,
  preserves per-target FIFO order, and lets distinct targets run concurrently;
- target lanes and queued calls have hard limits, duplicate call IDs fail, and
  current/peak metrics remain bounded and observable;
- dropping or cancelling a queued acquisition removes it cleanly and does not
  call backend cancellation because the capability operation never started.

The v2 foundation now passes 69 non-browser release tests plus the compile-fail
ownership doctest. `pwright-runtime` still has only `pwright-core` as a normal
dependency; the optional Tokio timer remains isolated in
`pwright-runtime-tokio`. This is executable evidence toward DT-032, DT-033,
DT-060, and DT-061, not completion: transport-close fan-out, a bounded cleanup
executor, connection-wide admission/fairness, object-backend disposal ordering,
and multi-command transactions remain open.

The connection-termination and abandoned-call-cleanup checkpoint makes runtime
shutdown and dropped futures observable rather than implicit:

- transport closure is terminal and idempotent; it invalidates every object in
  deterministic child-first order, requests `TransportClosed` for all pending
  calls, wakes active and queued calls, and rejects later registration or
  dispatch;
- an accepted transport close preempts a simultaneously ready backend result,
  but preserves an earlier cancellation reason under the existing first-wins
  rule;
- transport loss never attempts `Page.stopLoading` or another abort over the
  already-dead transport; active operation futures are still dropped;
- dropping an admitted controlled-dispatch future removes its pending and
  target-admission state, drops the backend operation, and then submits exactly
  one owned cancellation job; dropping a queued or never-polled call submits no
  backend cleanup;
- the default `CleanupQueue` is standard-library-only, bounded, FIFO, and
  externally drained. Saturation rejects explicitly and reports accepted,
  rejected, queued, peak, completed, and failed counts; no runtime task is
  spawned.

The optional `pwright-runtime-tokio` adapter now also supplies a
`TokioCleanupExecutor`. It requires an active caller-owned Tokio runtime, bounds
active cleanup jobs with a fixed semaphore, rejects saturation or missing
runtime state, and reports current/peak, accepted, rejected, completed, and
failed counts. It does not construct a hidden runtime or thread.

The connection-wide admission checkpoint adds a second bounded scheduler above
backend execution. Commands first obtain their per-target FIFO position and
only target heads enter the connection FIFO. This prevents followers on one
busy page from reserving global capacity, caps aggregate work across independent
targets, and defines fairness as FIFO among target heads. Cancellation and
deadline races remain active in both admission stages and release all acquired
permits without invoking a backend hook before the backend starts.

`RuntimeLimits::new` uses the pending-call bound as its default active-command
bound; `with_active_commands` can lower it independently. Both direct and
controlled dispatchers expose current and peak connection admission metrics.

The codec-neutral wire checkpoint adds `pwright-transport`, which depends only
on `pwright-core`. It provides typed frame routes; hard payload, attachment, and
aggregate byte limits; exact-length fragmented-byte assembly; inclusive
same-major version policy; generic codec and executor-neutral transport traits;
and a bounded client-side call correlator. Both outgoing codec output and
incoming frames are checked against the local connection limits before a codec
decoder receives them. No JSON, Serde, WebSocket, Tokio, authentication, or
reconnection policy is selected by this crate.
The generic exact-length assembler now also accepts an optional shared ledger:
it reserves the full declared `WireBytes` before buffer allocation and transfers
the lease into its non-cloneable finished output. Rejection, incomplete input,
assembler drop, and output drop have deterministic accounting, so custom
non-Tokio stream adapters no longer need an unbudgeted fragment path.

The optional JSON-codec checkpoint adds `pwright-codec-json` as the only new
crate importing Serde and `serde_json`. It serializes any generated or
application message implementing `RoutedMessage`, verifies that decoded routing
matches the outer frame route, and explicitly rejects malformed JSON and binary
attachments. Its dependency tree is separate: `pwright-core` remains
dependency-free, while `pwright-runtime` and `pwright-transport` still depend
only on core.

The handshake checkpoint extends `pwright-transport` with bounded owned wire
capability IDs, installed manifests, required/optional requests, and
codec-neutral client/server hello values. Negotiation chooses the highest minor
in the common major range, preserves directional receive limits, fails on
missing or incompatible required capabilities, omits unavailable optional
capabilities, enables no unrequested capability, and returns deterministic
capability ordering. No browser operation can start as part of negotiation.

The typed-client checkpoint adds `pwright-client`, whose normal dependency
graph contains only `pwright-core` and the core-only
`pwright-browser-model` vocabulary.
`Client` owns the channel and injected call-ID source; typed `Page` handles keep
only weak references, compare connection identity as well as object identity,
and fail deterministically after disconnect. The atomic ID source cannot wrap,
and typed in-process commands reach `DirectDispatcher` without a codec.

The typed-object checkpoint extends the client with `Browser`,
`BrowserContext`, `Page`, and `Frame` handles. Codec-neutral initializers must
match the expected kind, root status, and exact typed parent; disconnected
parents cannot create children. The client deliberately does not mirror the
runtime object graph. A generic `Locator<Query>` is a local value scoped to a
page or frame, allocates no remote object ID, lets selector capabilities own the
query representation, and combines only when both values share exact remote
scope and connection identity.

The object-kind checkpoint removes the client-owned numeric kind table.
`pwright-core` provides generic semantic IDs, descriptors, catalogs, and marker
traits; the core-only `pwright-browser-model` owns the canonical Browser,
BrowserContext, Page, and Frame vocabulary. Capability schemas declare
canonical command targets, which are emitted on `CommandSpec` and included in
minor compatibility. Remote server inventories reject missing target metadata,
and catalog-configured runtimes reject unknown lifecycle tags and wrong-kind
dispatch before scheduler/backend work. Parent descriptors declare the
Browser→BrowserContext→Page→Frame topology. Explicit `Partial` mode supports
externally attached subgraphs while validating every present parent;
`Complete` mode requires non-root kinds to have a valid parent, and both Create
and Adopt use the same check. This adds no Node.js, browser binary, download,
provider, backend, transport, or executor dependency.

The server-configuration checkpoint threads that catalog into capability
inventory construction before handshake. Every target declared by an
advertised command must resolve to an installed descriptor; otherwise manifest
construction fails before capability negotiation or command traffic. The
catalog remains local configuration rather than a browser acquisition request.

The client-liveness checkpoint adds a bounded object-ID-to-token table with a
1,024-object default and an explicit configurable limit. It reports current,
peak, and limit metrics, rejects duplicate live initializers, and invalidates
existing handle clones synchronously and idempotently from runtime disposal
notifications. Ordered bulk application consumes the runtime's child-first
result without copying its parent graph. A stale operation now reports
`TargetClosed`, while a dead client owner reports `TransportClosed`; neither
handle nor client `Drop` sends protocol traffic.

The opt-in client-capability checkpoint moves navigation into
`pwright-client-navigation`. Its `PageNavigationExt` trait adds
`begin_navigation` only when the extension crate is installed and imported;
the base client knows no navigation types. Generic `Remote::dispatch` owns
call-ID allocation, handle/connection validation, portable failure mapping,
and response-ID correlation for every future capability. The navigation
extension still deliberately exposes `begin_navigation`, not a false
Playwright-compatible `goto`.

The later goto checkpoint keeps that primitive intact and adds a separate
`PageGotoExt`, so channels may install either command contract independently.
The portable high-level command defaults to `load`, supports Playwright's four
wait states, carries referrer, and leaves timeout/cancellation in shared call
metadata.

The encoded-client checkpoint adds opt-in `pwright-client-wire`, depending only
on `pwright-core` and `pwright-transport`. Its cloneable typed command channel
and unique caller-driven receive driver share a bounded pending/tombstone table;
they start no executor task. Commands register before send, non-result frames
are returned for composition, and typed decode happens after raw call-ID
correlation. Send failure, receive failure, EOF, explicit close, driver drop,
and codec protocol violations terminally wake waiters. Dropped send attempts
retain a bounded tombstone so one late result is ignored and releases its slot.

The public-future cancellation checkpoint upgrades that tombstone into a
bounded cancellation outbox when an application explicitly supplies a control
codec. Matching command/control version policy and limits are validated during
construction. Missing cancellation identities are synthesized per wire call;
dropping a possibly sent future queues one `Caller` cancellation without doing
I/O in `Drop`. Commands and control share an executor-neutral bounded send gate:
dropping a command still waiting for that gate rolls back without a tombstone,
while only the unique active sender is classified as possibly sent. Control
waiters take priority over already queued commands, and a completed frame send
releases the gate before its result wait. The cancellation-aware driver keeps
its receive future pinned, flushes control on the independent write path, and
later consumes the cancelled result to release the tombstone. Fixed metrics
expose queued, peak, and sent cancellations. Encoding failure rolls back, send
failure closes and clears the channel, and the no-control profile retains its
previous smaller behavior.

The Tokio transport now retains one bounded partially written envelope and its
offset, so dropping a send future cannot split the stream; the next send or
shutdown completes it first. A Tokio duplex test carries a dropped public
`Locator.fill` future through the canonical control codec into a
`RuntimeControlReceiver`, proves the runtime cancellation hook receives the
same call/object/method identity, observes `Cancelled`, and drains all client
and runtime state. A separate 16-byte-pipe test interrupts a frame mid-write
and proves the resumed first and following frame both decode in order.

The bounded server-supervision checkpoint replaces the cancellation proof's
manually separated command/control polling with the actual application-phase
connection loop. `ConcurrentTypedCommandHandler` produces owned typed work and
its static chain preserves capability-by-capability composition. One
caller-driven `EncodedServerSupervisor` retains the only receive path, polls
every active command fairly, and applies synchronous control transitions while
commands are pending; it starts no task or runtime. Its nonzero in-flight limit
is explicit. Accepted commands are polled before the next receive so their
cancellation identity exists in the runtime. A command received at capacity
closes the connection rather than entering an uncancellable hidden queue.
Current/peak/accepted/completed/control/unhandled/capacity/dropped metrics and
typed terminal reasons are exposed. Dropping the run future releases all work,
records a terminal state, and forbids restarting a potentially partial reader.

The handshake-and-half-close checkpoint makes the same supervisor an explicit
connection state machine. A handshake-required constructor prevents
application reads until a bounded ClientHello passes an injected async
authorization gate, capability negotiation, ServerHello encoding, and send.
The returned authorization context lets the application install only handlers
allowed for that peer; `AllowUnauthenticated` is a named opt-in rather than a
remote default. Negotiated version and receive limits guard every later frame,
and rejection sends no ServerHello. Dropping pending authorization is terminal.
EOF policy is either immediate drop or control-driven drain. The drain path
stops reading, marks the controlled runtime transport closed, wakes admitted
commands, writes their terminal results through the remaining half-connection,
and closes only after active work reaches zero. A real Tokio duplex test proves
a pending Fill returns `TransportClosed` and releases every runtime object and
call after the client write half closes.

The client-handshake checkpoint closes the other half of that state machine.
`WireProtocol` now has separate send and receive limits while its original
constructor remains a symmetric compatibility path. A consuming
`EncodedClientHandshake` owns the bootstrap transport and is the only route to
`NegotiatedClientTransport`; application channel factories are therefore
unavailable until ServerHello passes exact outer/selected-version, echoed
client-limit, requested-capability, required-capability, and minimum-version
checks. The resulting client and server protocol factories invert the same two
directional limits and pin application traffic to one version. Dropping a
pending client handshake drops its transport handle. A real Tokio duplex test
performs both peer handshakes and a typed runtime round trip on the same stream.

The authenticated-local-endpoint checkpoint adds optional
`pwright-endpoint-tokio` above the generic transport and server layers. A
private Unix listener uses mode `0600`, starts no task or accept loop, and reads
kernel peer UID/GID/PID. The client verifies an injected expected server UID;
the server converts the accepted identity into a UID authorization gate that
runs before capability negotiation. Existing paths are never removed, and
device/inode-checked cleanup cannot delete a replacement path. Tests cover
permission mode, both peer identities, UID rejection, bounded frame I/O,
pre-hello authorization, and safe pathname ownership. That same-UID Unix
profile does not authenticate a network peer.

The authenticated-Tailnet checkpoint adds the first non-Unix adapter without
weakening the generic transport or handshake. `TailnetEndpoint` rejects every
bind, server, and accepted-peer address outside Tailscale's
[`100.64.0.0/10` and `fd7a:115c:a1e0::/48` ranges](https://tailscale.com/docs/reference/reserved-ip-addresses).
Before constructing a frame
transport, both peers prove one fixed 32-byte PSK using a CSPRNG 32-byte server
nonce and direction-separated HMAC-SHA256 proofs. The key is never transmitted
or formatted and its owned allocation is zeroized. The fixed versioned exchange
is exactly 105 bytes; it has no length-controlled allocation, retry queue,
accept loop, task, or implicit deadline. A proof-carrying `TailnetPeerGate`
then supplies the authenticated source address to the unchanged Pwright
handshake gate.

Tests reject loopback/LAN/public binds before socket creation, cover the exact
IPv4/IPv6 boundaries, wrong client and server proofs, reflection, replay under
a fresh nonce, and preservation of the following frame boundary. A manually
selected integration test bound this Mac's actual Tailscale address and ran the
real PSK exchange followed by the canonical ClientHello/ServerHello handshake.
This PSK adapter remains expressly Tailnet-only.

The authenticated-mTLS checkpoint adds a separate general TCP adapter rather
than weakening that Tailnet policy. The caller supplies rustls client/server
configuration, trust roots, server name, and a fixed-capacity set of authorized
SHA-256 leaf-certificate fingerprints. Both sides require the exact
`pwright/1` ALPN, normal rustls certificate verification, a presented peer
certificate, and an exact pin before a frame transport exists. A two-entry pin
set permits a bounded old/new overlap during rotation; empty and duplicate sets
are rejected. Only a successfully accepted value can construct
`MutualTlsPeerGate`, so the unchanged Pwright handshake receives proof-carrying
peer identity rather than credentials in ClientHello.

The endpoint rejects wildcard binds, acquires one of a caller-declared number
of handshake slots before accepting another socket, and starts no accept loop,
task, runtime, or clock. rustls 0.23 defensively caps one buffered handshake
message at 65,535 bytes and one certificate at 65,536 bytes; each TLS
connection also receives an explicit outbound buffer ceiling equal to the
larger of 65,536 bytes and the configured length-prefixed Pwright envelope.
Those rustls-owned buffers are a documented third-party boundary, not falsely
charged as exact Pwright `WireBytes`. The caller still owns handshake/drain
deadlines, PKI issuance and revocation, durable key storage, and the operational
rotation schedule.

Tests cover mutual authentication, absent client certificates, a CA-valid but
unpinned server, exact ALPN, bounded pin rotation, bounded output buffers, and
preservation of the first Pwright frame after TLS. A selected live test bound
this Mac's actual Tailscale interface and completed mTLS, pin verification, and
the canonical ClientHello/ServerHello exchange. This completes DT-035's
transport/authentication boundary foundation; deploying a public listener
still requires an explicit application policy under DT-063.

The authenticated Chromium endpoint checkpoint adds the optional
`pwright-composition-chromium-tokio` leaf rather than making the base Chromium
root depend on endpoint policy. Its `AcceptedChromiumEndpoint` boundary covers
Tailnet and mTLS directly and wraps Unix with one explicit allowed UID. It
keeps the endpoint-derived gate paired with the exact accepted transport,
claims the predeclared Tokio stream retention plan from the fresh Chromium
ledger, carries the pair through runtime installation, and consumes the gate
inside same-supervisor handshake acceptance. No constructor exposes a way to
substitute another gate between those stages. Release tests prove the concrete
authenticated endpoint families produce statically typed production
transports and share the caller's exact ledger. A dedicated Chrome 152 test
proves a real kernel Unix peer identity reaches authorization and that dropping
the resulting connection leaves externally owned Chrome attachable. Listener
loops, admission concurrency, deadlines, and full application plugins remain
caller-owned. This advances DT-035, DT-041, DT-043, DT-051, DT-052, DT-060,
DT-061, and DT-063 without selecting a universal deployment transport.

The bounded-drain checkpoint adds `run_with_drain_deadline` without importing a
clock or executor into `pwright-server-wire`. Its injected future is dormant
until EOF leaves admitted work active. Commands that become ready during the
transport-close transition win before an simultaneously ready deadline;
otherwise expiry drops the remaining bounded work, records one deadline metric,
and terminates with `DrainDeadlineExceeded`. Tests cover both permanent pending
work and close-released work. The original `run` keeps its unbounded
compatibility behavior, so a production endpoint policy must deliberately
choose and inject a duration.

The capability-range and binary-contract checkpoint removes the assumption
that an installed capability is represented by only one minimum version.
Client requests and server manifests can now publish inclusive ranges, and
negotiation selects the highest common version. Disjoint required ranges fail;
disjoint optional ranges are omitted; the client rejects a selected version
above its maximum. The ranged ClientHello has a new canonical tag while the
decoder retains the legacy minimum-only representation.

The new `pwright-codec-binary` crate centralizes allocation-aware network-order
writing, borrowed bounded reading, exact malformed-input diagnostics, and
static positional schema metadata. Control, handshake, navigation, and locator
codecs now use it and publish validated top-level layouts, with golden-byte
coverage proving their encodings did not drift. Existing positional variants
cannot change within a major, while higher-tagged variants may be appended in a
minor after capability negotiation. Navigation header decoding now preflights
the minimum bytes and uses fallible reservation before allocating entries.
This is evidence toward DT-031 and DT-035, not completion of DT-031: nested
messages are still opaque, and an evaluated source IDL, generated Rust and
cross-language types, compatibility fixtures, and codegen remain open.

The handler-inventory checkpoint binds every remotely advertised command spec
to its owning capability and command ID. Sequential and concurrent static
handler chains can now derive the bounded handshake manifest directly. The
inventory rejects missing handlers for any command declared by a capability,
duplicate command or wire-method handlers, descriptor conflicts, undeclared
commands, and invalid dependency graphs before a ClientHello is accepted.
This closes the configuration drift where a server could maintain its handler
chain and advertised capability list independently; the raw manifest path
remains only as a compatibility and low-level construction API.

The nested-schema checkpoint replaces untyped `Message` placeholders with
stable referenced type IDs and adds distinct record/tagged-union descriptors
plus a canonically ordered catalog. Catalog validation accepts intentional
recursive types such as selector nodes without recursively walking values, and
rejects unknown references before generation or negotiation. Existing codec
top-level schemas now name their nested types. Complete per-capability catalogs,
an external source syntax, generators, and release-to-release nested
compatibility remain open, so DT-031 is still not complete.

The first source-IDL checkpoint records the local Playwright schema/generator
evidence and evaluates YAML, Protocol Buffers, FlatBuffers, and Rust-only
declarations. A strict, smaller Playwright-shaped YAML grammar is selected
provisionally, owned independently by each capability. The development-only
`pwright-schema-gen` crate rejects unknown keys, noncanonical IDs/tags,
duplicate fields, missing references, excessive shapes, and unsupported
recursive Rust models, then deterministically emits Rust wire models and a
complete static catalog. It has an explicit non-mutating `--check` mode; there
is no `build.rs`, and generated capability crates acquire no YAML, Serde, Node,
or generator dependency.

Navigation is the first representative source and generated catalog. Its
existing canonical bytes did not change. The grammar now includes strict
booleans and stable-tag enums; enum evolution is append-only within a compatible
minor release. Catalog-level compatibility now
allows new types and appended higher-tag union variants but rejects removed or
retyped types, record layout changes, existing union changes, version
regression, and changed nested references. The evaluated alternatives,
commands, and remaining gates are recorded in
`knowledge/pwright-protocol-idl.md`. At that checkpoint DT-031 remained open for
declarative string/collection/cross-field validators, dispatch metadata,
user-facing docs, transitive imports, recursive models, generated public-model
conversions, and a second-language fixture.

The generated-value-codec checkpoint adds shared `BinaryEncode` and
`BinaryDecode` contracts plus an explicit `BinaryValueContext`. String bytes,
binary bytes, repeated items, nesting depth, and total message nodes all have
caller-supplied hard limits. Decode validates lengths before copying, performs
fallible string/byte/vector reservation, and uses a schema-derived minimum item
size to reject impossible repeated counts before allocating. Generated
navigation models now implement these codecs; a simple generated goto is
byte-for-byte identical to the published hand-written payload, while tests
prove field-attributed string/depth failures and header-body preflight.

`pwright-schema-gen --previous` now compares the previous published source with
the candidate before generation or drift checking. Navigation keeps a checked-in
1.0 baseline. New types and appended union variants pass; record changes and
mutated old variants fail in generator tests. At that checkpoint DT-031
remained open for string/collection/cross-field constraint generation,
generated public-command conversions, dispatch/docs generation, transitive
imports, recursive models, and cross-language fixtures.

The navigation production codec now uses the generated models and bounded
generated codecs for BeginNavigation and Goto commands and results. This covers
success values, response headers, failure-kind enums, strict booleans, and typed
error details; the parallel hand-written navigation payload implementation has
been removed. At that checkpoint the conversion boundary still owned aggregate
header limits, canonical header ordering, and mechanical public-model mapping.

That memory checkpoint now emits lifetime-parameterized borrowed models for
every schema type that transitively contains UTF-8, bytes, or repeated values.
Repeated values cross an object-safe sequence reference; generated owner code
still applies the count bound and emits the field layout. Navigation commands,
successes, errors, and responses now encode through these views. The canonical
header sequence uses deterministic allocation-free selection and preserves
duplicates, eliminating cloned strings and the intermediate owned header
vector. Owned generated models remain the decode boundary. At that point DT-031
no longer listed borrowed encoding as open; generated public-model conversions,
declarative cross-field constraints, transitive imports, recursion,
dispatch/docs, and another language remain.

The numeric-constraint checkpoint adds inclusive `minimum`/`maximum` metadata
without narrowing the `i64 | u64` source domain. Source and catalog validation
reject empty, reversed, wrong-kind, and out-of-wire-kind ranges. Generated
owned and borrowed encoders plus decoders enforce the bounds, and ranges enter
layout fingerprints and exact minor compatibility. Navigation duration
nanoseconds now declare `maximum: 999999999`; generated decode performs the
check before `Duration` construction and the previous hand-written validation
branch is gone.

The shared-library checkpoint separates reusable wire values from capability
roots. A source declares exactly one `library`, `capability`, or non-installable
`protocol` family; capabilities list direct imports and the generator requires
the matching library sources.
`pwright-codec-common` now owns call metadata, duration, and failure kind.
Navigation reexports those exact generated Rust types and stable schema
constants, while its composed catalog and compatibility checks include the
imported graph. Consumers still install only the capability and its small
normal codec dependencies: no YAML parser, generator, Node runtime, transport,
executor, or browser is pulled in. Transitive imports and recursive import
graphs remain deliberately unsupported while their version policy is designed.
The CLI accepts separate repeatable `--import` and `--previous-import` paths,
so a capability compatibility check resolves the actual new and published
dependency graphs rather than comparing both sides against the newest library.

The size-constraint checkpoint adds inclusive UTF-8/byte `minimum_length` and
`maximum_length` plus repeated `minimum_items` and `maximum_items`. Source and
catalog validation reject empty, reversed, or shape-incompatible ranges.
Generated encode checks before writing; decode checks the wire length/count
before copying or reserving. These semantic bounds participate in layout
fingerprints and exact minor compatibility while remaining independent of
caller-selected hard resource ceilings. Per-element length constraints on an
opaque borrowed primitive sequence and cross-field aggregate rules remain open.

The command-metadata checkpoint makes each capability source declare stable
command IDs and method tags together with its command/result root variants.
The generator validates canonical ordering, unique methods, distinct existing
root variants, and forbids commands in a library. It emits public
`BinaryCommandSchema` constants; navigation tests compare both generated
mappings with the typed capability constants. Removing or changing a published
mapping is a minor-compatibility error. Typed marker/spec generation and server
handler-table generation originally remained open because they require separate
output ownership rather than placing capability APIs inside a codec module.

The multi-output ownership decision now fixes the next generator boundary.
The canonical schema moves under the portable capability crate. Generated API,
codec binding, optional static server glue, and protocol reference each land in
their owning crate or docs directory. The portable capability remains core-only;
generic core/runtime/client/server crates remain capability-agnostic; no output
uses `build.rs`. Commands will declare semantic params/results/errors and the
generator will synthesize common call/result envelopes while preserving the
current navigation golden bytes. The exact dependency graph, atomic emission
contract, rejected ownership models, and migration order are recorded in
`knowledge/pwright-generator-output-ownership.md`.

The first multi-output implementation moves navigation's canonical source and
baseline into `pwright-cap-navigation/schema`. Its symbolic command names now
drive a second committed generated artifact containing capability/command IDs,
methods, the command list, and `NavigationCapability::DESCRIPTOR`; the former
hand-written definitions are gone. `--capability-output` renders this core-only
surface alongside the codec output, and check mode detects drift in either.
The verified normal dependency tree for `pwright-cap-navigation` still contains
only `pwright-core`.

The reference-output checkpoint adds a third committed artifact generated from
that same navigation source. `--reference-output` emits deterministic Markdown
covering command IDs/methods/targets, semantic models, wire variants,
positional fields, cardinality, enum defaults, and declared constraints.
Write mode renders every requested artifact before writing; check mode reports
the reference together with stale Rust outputs without modifying any file.

The static-routing checkpoint adds core-only `CapabilityCommandDescriptor`
values and a generated ordered descriptor slice per capability. Each entry
binds command ID, numeric method, and semantic target kinds without containing
a service, codec, executor, backend, or allocation. The generic server
inventory records those descriptors while registering the statically composed
typed handler chain and can resolve them by command ID or method tag. Locator
publishes the same metadata shape while its schema migration remains pending.

The generated-server checkpoint adds a fourth committed output for the same
navigation source. `--server-output` emits a capability-specific crate with
one injected service per command, one shared `WireProtocol<NavigationCodec>`,
and a complete static `ConcurrentServerHandlerChain`. Its normal dependency
tree contains only the portable navigation contract, navigation codec,
generic server boundary, transport contract, and their portable prerequisites;
it imports no runtime, executor, backend, CDP, Node.js, or browser provider.
The chain itself supplies the checked handshake manifest, so adding a schema
command without regenerating the installed server composition is detected as
output drift rather than becoming a silently unadvertised route.

The semantic-command checkpoint completes the portable half of that migration.
Every command now names parameter, result, and typed-error models; published
model names are validated for uniqueness and compatibility. The capability
emitter owns the public records/enums/unions, typed marker structs, and
`CommandSpec`/`CapabilityCommandSpec` implementations, so their former
hand-written navigation definitions are gone. Navigation's root wire union now
embeds those command parameter/result records directly. Because positional
messages are inline, this removes field duplication without adding an envelope
or changing the established bytes. The codec emitter imports only the portable
public types and generates owned `From` conversions in both directions.
Navigation retains only policy-bearing borrowed adapters and validation for
aggregate header bytes and canonical header order.

The capability-dependency checkpoint adds a separate declarative
`dependencies` list to capability sources; it is deliberately independent of
wire-library `imports`. The generator rejects invalid, duplicate/out-of-order,
self, and library-owned dependencies, emits exact minimum versions into
`CapabilityDescriptor`, and treats any dependency change as a major-version
operation. A capability with no dependencies, including navigation, keeps the
same generated surface and dependency tree.

The locator-schema checkpoint exercises the same design with a recursive and
policy-rich capability. `pwright-cap-locator/schema/locator.yml` is now 1.22;
its preserved 1.21 baseline proves the ScrollIntoViewIfNeeded route is a compatible
append-only command after the 1.21 SelectOption checkpoint.
It generates AllInnerTexts/AllTextContents/Blur/Click/Count/Fill/Focus/GetAttribute/Hover/InnerHtml/InnerText/InputValue/IsChecked/IsEditable/IsEnabled/IsVisible/Press/ScrollIntoViewIfNeeded/SelectOption/SetChecked/TextContent/WaitFor IDs, methods, targets, the
recursive boxed selector wire tree, a complete catalog, protocol reference,
and the optional `pwright-server-locator` handler chain. The generator rejects
recursive models that have no finite wire value. `public_external` keeps
validated options, params, errors, and the unit result hand-written in the
core-only capability while preserving their stable schema names and mappings.
The immutable selector and matcher values now live in zero-dependency,
command-free `pwright-selector-model` and are reexported unchanged by
`pwright-cap-locator`. Both protocol majors and all downstream layers retain
the same type identity. The nine-type wire closure is also published as the
command-free `pwright.selector` 1.0 library and generated
`pwright-codec-selector` crate. Generator tests compare each definition exactly
with both published Locator tracks and compile a consumer that imports selector
types without importing Locator commands. Existing fill golden bytes remain unchanged, while round trips now
cover string and regex text/label/role/attribute matching, exact and regex test
IDs, plus `hasText`, `hasNotText`, `has`, `hasNot`, and visibility filters.

The corresponding locator checkpoint also fixes same-Frame identity and
backend behavior. An initialized Page locator uses its main Frame scope, inner
`has`/`hasNot` locators from another Frame fail locally, and all filters remain
lazy AST transformations. The Chromium runtime preserves base candidates for
relative descendant filters and implements visible/hidden selection with
Chromium visibility APIs. A dedicated Chrome 152 test combines all four new
nodes, clicks only the intended card, and proves hidden, excluded, and
text-rejected alternatives were untouched.

The pointer-action checkpoint adds Playwright-shaped `Locator.hover` without
coupling the portable capability to CDP. IDL `f64` support preserves fractional
positions in network byte order and removes `Eq` only from generated models
that transitively contain floats. Hover strictly re-resolves, waits for visible
and stable geometry, optionally scrolls, checks the hit target unless forced,
and never requires enabled state. Its bounded modifiers are canonicalized in
an allocation-free bitset; `ControlOrMeta` resolves from the browser platform.
Chromium emits one mouse-move event, while `trial` performs no mouse move.
Modifier presence is semantic: omission preserves the Page's current keys,
whereas an explicit empty set temporarily releases them. A fixed-size,
Page-scoped input coordinator serializes all main/local/OOPIF Locator routes,
dispatches canonical key-down transitions before the action, and restores the
prior state afterward. Trial still performs those modifier transitions, matching
Playwright's ability to expose modifier-dependent UI without firing the pointer
action. Controlled cancellation restores a dropped transaction; a directly
dropped backend future leaves bounded pending-restore state that the next input
transaction heals before proceeding.

The regex checkpoint keeps Playwright's `RegExp` branch distinct from string
matching. `RegexMatcher` validates canonical ECMAScript flags without pulling a
regex engine into the portable capability, while Chromium compiles the source
inside the bounded selector attempt and maps syntax failures to
`InvalidSelector`. Appended tags cover text, label, positive/negative text
filters, and mixed string/regex role name or description options. A dedicated
Chrome 152 proof exercises each route, runtime-v3 replacement, and invalid
source classification without retaining DOM handles.

The user-facing attribute checkpoint completes the current factory families
without introducing CSS interpolation or global configuration. Placeholder,
alt-text, and title strings compare raw attribute values using exact
case-sensitive equality or non-exact case-insensitive substring semantics;
their regex branches also use the raw value. Test-ID strings are exact and
default to `data-testid`. Explicit comma-separated names are validated and
capped at 16, then stored in each immutable selector so independently composed
clients cannot race on mutable process-global state. Schema 1.5 appends four
selector tags and one fixed attribute enum; runtime v6 retains those leaves.
A dedicated Chrome 152 proof clicks all five routes after navigation and shows
that exact placeholder matching does not collapse doubled whitespace.

The snapshot-query checkpoint separates reads from single-target actions.
`Locator.count` resolves the current selector once with strictness disabled,
returns zero without waiting, accepts multiple matches, and carries a bounded
`u32` result through the generated codec, client extension, server handler,
target router, and Chromium backend. It shares the locator operation budget
but never retains a result array or DOM identity. Unit, duplex wire, exact
utility-world routing, and real Chrome 152 tests cover this boundary.

The visibility-query checkpoint adds one strict immediate state command rather
than duplicating two backend paths. `Locator.is_visible` returns false for a
missing element, returns the backend-owned visibility state for one element,
and preserves strict failure for multiple matches. `Locator.is_hidden` is
implemented by the optional client extension as the negation of the same typed
command. Unit, duplex wire, utility-world, malformed-response, navigation, and
real Chrome 152 tests prove this composition.

The element-state checkpoint adds three separately installable command routes:
`Locator.is_checked`, `Locator.is_editable`, and `Locator.is_enabled`.
`Locator.is_disabled` composes over enabled without a fourth backend route.
These getters follow Playwright's distinct timing contract: missing or
detached targets are retried under the one caller deadline, strict violations
and invalid target kinds fail immediately, and an available target returns its
current state without polling for `true`. Selector runtime v6 centralizes
label retargeting and native/applicable ARIA state rules. Unit, generated
codec, duplex wire, exact utility-world, deadline, malformed-response, and
real Chrome 152 tests cover the new boundary.

The locator-wait checkpoint adds a separately installable
`Locator.wait_for` route at method `131080` and root tags 15/16. The public
state vocabulary is attached, detached, visible, and hidden, with visible as
the semantic default. Every attempt strictly re-resolves the Locator; zero
matches immediately satisfy hidden or detached, while attached and visible
retry under the runtime-owned caller deadline. Strictness, invalid selectors,
and malformed responses are terminal. Success returns unit and retains no
ElementHandle. Generated-codec, client, server, Chromium routing, bounded
deadline, malformed-response, and real Chrome 152 tests cover the route, and
the backend exposes a separate `wait_evaluations` metric without adding a
second operation lease.

The text-content checkpoint adds a separately installable
`Locator.text_content` route at method `131081` and root tags 17/18. It waits
for one strict connected target under the runtime-owned caller deadline, then
copies the exact nullable DOM `textContent` without normalization. Schema,
codec, and backend limits cap the result at 1 MiB of UTF-8; invalid selectors,
strictness, malformed values, and oversized results are terminal. The client
extension depends only on this command service, and unit, duplex wire,
utility-world, deadline, malformed-response, resource-boundary, and real
Chrome 152 tests prove the route without retaining a DOM handle.

The inner-text checkpoint adds a separately installable `Locator.inner_text`
route at method `131082` and root tags 19/20. It strictly waits for one
connected target, requires the HTML namespace, and copies the browser-rendered
non-null `HTMLElement.innerText`; this deliberately differs from raw nullable
`Node.textContent`. Schema, codec, and backend limits cap the UTF-8 result at
1 MiB. Typed strictness, invalid-target, malformed-result, oversized-result,
deadline, duplex wire, utility-world, resource-boundary, and real Chrome 152
proofs cover the route without retaining an ElementHandle.

The inner-HTML checkpoint adds a separately installable `Locator.inner_html`
route at method `131083` and root tags 21/22. It strictly waits for one
connected element and copies the browser's non-null serialized
`Element.innerHTML`, including comments, escaping, and SVG child markup;
unlike `inner_text`, it has no HTMLElement namespace restriction. The result
is bounded to 1 MiB, shares the existing content-read operation peak rather
than adding it, and is covered by typed, codec, duplex wire, deadline,
utility-world, strictness, malformed/oversized-result, and real Chrome 152
tests without retaining an ElementHandle.

The get-attribute checkpoint adds a separately installable
`Locator.get_attribute` route at method `131084` and root tags 23/24. It
strictly waits for one connected element and calls the browser's
`Element.getAttribute(name)` without an HTMLElement namespace restriction;
present values are exact strings, missing values remain `None`, and SVG is a
valid target. Attribute names are bounded to 64 KiB and results to 1 MiB. The
input, generated script, and result participate in mutually exclusive peak
branches rather than being summed with fill and content reads. Typed, codec,
duplex wire, deadline, utility-world, strictness, malformed/oversized-result,
pre-CDP oversized-name, and real Chrome 152 proofs cover the route without
retaining an ElementHandle.

The input-value checkpoint adds a separately installable `Locator.input_value`
route at method `131085` and root tags 25/26. It strictly waits and re-resolves,
uses selector runtime v6's shared follow-label retargeting, accepts only
`input`, `textarea`, and `select`, and returns the current non-null value
without normalizing it. Non-control targets remain typed failures. The result
is bounded to 1 MiB and shares the mutually exclusive locator-result peak.
Typed, codec, duplex wire, deadline, utility-world, strictness,
malformed/oversized-result, and real Chrome 152 proofs cover label, input,
textarea, and select behavior without retaining an ElementHandle.

The all-text-contents checkpoint adds a separately installable
`Locator.all_text_contents` route at method `131086` and root tags 27/28. It
follows Playwright's immediate `$$eval` snapshot rather than the strict getter
retry loop: zero matches return an empty vector, multiple matches are expected,
and each value is exact `textContent || ''` in selector order. Schema and codec
cap the collection at 4096 items; codec and backend additionally cap each UTF-8
item at 1 MiB and the aggregate at 4 MiB. Typed, codec, duplex wire,
utility-world, navigation replacement, malformed/oversized-result, resource,
and real Chrome 152 proofs cover the handle-free route.

The all-inner-texts checkpoint adds a separately installable
`Locator.all_inner_texts` route at method `131087` and root tags 29/30. Like
Playwright it performs one immediate, non-strict `$$eval`-shaped snapshot and
returns rendered `HTMLElement.innerText` values in selector order. Unlike the
unchecked upstream TypeScript cast, Pwright rejects SVG or any other
non-`HTMLElement` match as typed `InvalidTarget`, preserving the declared
`Vec<String>` result. It shares the reviewed 4096-item, 1 MiB-per-item, and
4 MiB aggregate limits with raw text collections, but production accounting
takes the maximum mutually exclusive collection branch rather than summing
both APIs. Typed, codec, duplex wire, cancellation, utility-world, navigation
replacement, malformed/oversized-result, resource, and real Chrome 152 proofs
cover the handle-free route.

The locator client-composition checkpoint adds Playwright-shaped `clear()` and
`all()` without advancing the locator protocol or adding backend commands.
Clear is exactly one existing Fill call with an empty value. All is exactly one
Count snapshot followed by local `nth(i)` construction; its returned Rust
`LocatorList` keeps one base selector and materializes locators on demand
instead of hiding an N-fold selector-AST allocation. It consumes the source
Locator to move that base; callers clone explicitly only when they need both.
Counts above the signed nth-index domain fail as `ResourceExhausted` before
list construction. Unit tests prove the minimal service bounds and exact
dispatches; Chrome 152 proves clear's Fill behavior and that an nth Locator
obtained before navigation resolves lazily in the replacement document.

The hover checkpoint advances locator protocol to 1.17 with method `131088`
and root tags 31/32. The public action keeps selector resolution lazy and adds
force, auto/no scrolling, a canonical allocation-free modifier set, an optional
finite fractional point relative to the padding box, and trial. The backend
waits for strict visible/stable geometry and an unobscured hit point without an
enabled check, then sends exactly one mouse move. `ControlOrMeta` is resolved
from the browser platform and trial dispatches no mouse input, while any
explicit modifiers are still pressed and restored. Generated-model,
binary-codec, typed-client, duplex-wire, backend, malformed-response, and real
Chrome 152 proofs cover the route. The optional modifier wrapper preserves
omitted versus explicit-empty semantics. Chromium uses a fixed-size Page owner
shared by main, local-frame, and OOPIF routes to serialize key transitions,
mouse dispatch, restoration, and cancellation cleanup. General keyboard,
mouse, and touch capabilities can reuse this owner, but remain separate future
capabilities.

The focus-management checkpoint advances locator protocol to 1.18 with Focus
method `131089`, Blur method `131090`, and root tags 33 through 36. Both are
strict lazy actions with selector-only parameters and generic runtime-owned
timeout/cancellation metadata. Chromium retries only missing or disconnected
targets, invokes native `focus()` twice or `blur()` once, and performs no
visibility, stability, enabled, scroll, keyboard, or pointer work. The codec
rejects both commands when 1.17 was negotiated. Unit and dedicated-profile
Chrome proofs cover typed dispatch, strict violations, focus/blur events,
active-element transitions, non-focusable success, already-blurred success,
temporary-target cleanup, and preservation of the externally owned browser.

The cross-major parity checkpoint freezes locator 2.0, advances the independent
2.x track to 2.1, and appends those same Focus and Blur IDs, tags, parameters,
and semantics without changing its extensible Click record. The exact selected
capability minor now reaches `LocatorV2Codec`: 2.0 rejects Focus/Blur before
encoding or backend work, while 2.1 enables both. The local protocol also keeps
its outer frame version independent from the Locator capability version.

The Press checkpoint is specified in
`docs/knowledge/pwright-locator-press.md` and advances both tracks together to
1.19 and 2.2 with one Press route, a bounded Playwright key-expression grammar,
full static US-layout resolution, rich CDP key events, and fixed-capacity
Page-owned pressed-key cleanup. It reuses the existing navigation barrier and
adds no Node.js or browser-distribution dependency. Generated compatibility,
typed client/server streams, the Chromium target router, selection-reset focus,
reverse-order failure/cancellation cleanup, and dual-major production dispatch
are implemented. Dedicated-profile Chrome 152 now also proves
selection-sensitive Arrow input, printable/shifted text, the platform-smart
select-all shortcut, Enter/form navigation waiting, and controlled cancellation
with reverse key release while preserving the external browser. This closes the
Press checkpoint's designed live gate.

The SetChecked checkpoint is specified in
`docs/knowledge/pwright-locator-set-checked.md` and advances the same tracks to
1.20 and 2.3. It adds one desired-state primitive with client-composed Check
and Uncheck, bounded extensible options in both majors, Playwright-compatible
idempotence and radio behavior, and post-click verification. Generated
compatibility, old-minor rejection, typed remote streams, selected-major
production dispatch, shared Page pointer cleanup, and dedicated-profile Chrome
coverage are implemented without Node.js or browser-distribution ownership.

The SelectOption checkpoint is specified in
`docs/knowledge/pwright-locator-select-option.md` and advances the tracks to
1.21 and 2.4. One bounded command accepts strings or conjunctive
value/label/index descriptors, preserves empty-list deselection and DOM-order
results, retries every pre-commit actionability and option-availability state,
and fires Playwright-compatible input/change events after one atomic mutation.
Locator stays independently installable: ElementHandle selection remains a
separate future capability-composition decision until Handle adoption across
execution worlds has a bounded lifecycle contract.

The ScrollIntoViewIfNeeded checkpoint is specified in
`docs/knowledge/pwright-locator-scroll-into-view.md` and advances the tracks to
1.22 and 2.5. The command remains Locator-only and strict, waits for stable
geometry but not visibility, enabled/editable state, or hit-target reception,
and avoids repositioning at full intersection. Missing, detached, layout-less,
and unstable targets retry before the one non-replayable scroll commit. Frozen
1.21/2.4 rejection, cross-major bytes, typed remote streams, selected-major
dispatch, bounded cancellation, and dedicated-profile Chrome geometry are its
acceptance gates. Browser.version was deliberately not substituted here: it is
cached browser initializer data in Playwright and awaits a bounded initializer
extension/publication design rather than a misleading per-call RPC.

The control-schema checkpoint adds a third schema identity: a non-installable
`protocol` family has a tagged-union root but cannot declare capability
commands or dependencies. Control 1.0 now owns its five exact root tags,
positional fields, strict booleans, cancellation-reason enum, baseline, catalog,
and generated reference in YAML. Production encoding and decoding use the
generated model; hand-written code is limited to conversion into semantic core
IDs and the cross-field canonical absent-parent rule. Existing control golden
bytes and every typed malformed-payload error remain stable.

The external-count checkpoint resolves the handshake grammar decision.
ClientHello writes the required and optional capability counts as one header
before either variable-length capability segment, whereas an ordinary repeated
field writes its count immediately before its own items. The new `counted_by`
relationship is legal only on a repeated field referencing an earlier required
`u32`; it is validated, fingerprinted, compatibility-checked, documented, and
used by both owned and borrowed generated codecs without emitting an adjacent
count. `pwright-codec-handshake` now has a canonical 1.0 YAML source, baseline,
catalog, generated reference, and production generated codec. Borrowed adapters
encode canonical capability maps and slices without an intermediate owned wire
tree. Generated decode is followed only by semantic ID/order, version-policy,
combined-count, and set-uniqueness checks. Legacy minimum-only decoding and
both empty and populated golden layouts remain exact.

The semantic-default checkpoint makes enum defaults part of the reviewed
source rather than a Rust-only extension. The generator rejects a missing
default variant, derives `Default` on both public and wire enums, marks the
selected variant, and rejects default changes within a compatible minor
release. Navigation now declares `wait_until: load` in its schema and has no
hand-written `WaitUntil::default` implementation.

The typed real-Chrome checkpoint upgrades the live navigation test to cover the
attach-only CDP provider, real temporary incognito context and target, matching
runtime Browser/Context/Page graph, typed client handle initialization,
`PageNavigationExt`, direct dispatch, and the CDP navigation backend in one
path. It passed against the dedicated Chrome 152 CDP profile. The test disposes
only its temporary context and confirms `Browser.getVersion` still succeeds
after the typed client is dropped, preserving external browser ownership.

The composable-driver checkpoint adds synchronous `FrameHandler` routing to
`pwright-client-wire`. Deterministic `or_else` chains identify which optional
subsystem handled a frame and preserve the original frame if none did. The new
opt-in `pwright-client-control` handler decodes only control frames, forwards
Create/Adopt as typed lifecycle events, applies idempotent Dispose invalidation,
rejects Cancel/SetSubscription in the wrong direction, and leaves other routes
untouched. Neither layer starts executor work.

The runtime-lifecycle-emission checkpoint adds an opt-in bounded control outbox
to both dispatchers. Create, changed Adopt, and child-first Dispose mutations
preflight their required capacity while holding the object-state lock. A full
outbox therefore rejects before changing the graph or accepting controlled
parent-close cancellation. Exclusive rollback-on-drop claims keep encoding
failures from losing the oldest message. The new `pwright-runtime-control`
sidecar sends one validated Control frame per caller-driven `send_once` and
acknowledges it only after transport success; it brings no executor or socket.

The typed-event checkpoint adds opt-in per-event handlers and queues. A handler
decodes only its registered `EventSpec`, validates the route source, and leaves
unrelated frames available for composition. Queue overflow returns either the
rejected reliable event or the dropped oldest telemetry event. A separate
bounded subscription tracker emits SetSubscription only on first-listener and
last-listener edges and forgets disposed-object listener state without invalid
unsubscribe traffic.

The bounded-stream checkpoint adds a canonical binary WireFrame envelope in a
standard-library-only crate. It rejects malformed routes, truncation, trailing
bytes, and every oversized declared length before body copying. A separate
optional Tokio adapter length-prefixes envelopes, checks the outer length before
allocation, serializes writers independently from its single reader, recognizes
clean frame-boundary EOF, and explicitly shuts down only caller-owned I/O.

The bidirectional-control checkpoint adds a canonical Serde-free codec for all
lifecycle, cancellation, and subscription controls. The runtime receiver
accepts only caller/deadline cancellation and bounded live-object subscription
changes; it rejects runtime-owned lifecycle messages in the client direction.
Duplex tests now cover runtime-to-client Create and client-to-runtime
SetSubscription across the binary stream and optional Tokio transport.

The typed-server checkpoint adds a unique-reader server driver and statically
composable async frame handlers. Each typed command handler validates outer and
decoded identities, dispatches through the runtime Service boundary, verifies
result correlation and route, and sends exactly one response. Runtime control
implements the same handler contract, allowing one inbound chain. A Tokio
duplex test proves typed Remote dispatch through client wire, server wire,
DirectDispatcher, and back to typed result decoding.

The handshake-first checkpoint wraps that entire inbound chain with a strict
connection state machine. Application frames before negotiation, duplicate
hello frames, and post-negotiation frames using any version other than the
selected one are terminal handler failures. The initial outer frame version is
deliberately non-authoritative; bounded `ClientHello` contents drive
negotiation, then the server reply and all later traffic use the selected
version. Both the initial and established receive paths apply the server's
advertised frame limits before delegation.

The concrete-hello checkpoint adds a separate Serde-free binary codec for both
handshake directions. Counts and capability ID lengths are checked before
allocation, capability ordering is canonical, and malformed versions, IDs,
UTF-8, ordering, routes, attachments, truncation, and trailing data fail
explicitly. A server integration test decodes the canonical ClientHello,
negotiates, and decodes the emitted canonical ServerHello.

The Playwright-goto checkpoint adds CDP-session-filtered commit,
DOMContentLoaded, load, and 500 ms whole-page network-idle waiting. It returns
the HTTP(S) main-resource response snapshot, preserves HTTP error statuses as
successful navigations, returns null for same-document and data-style
navigations, forwards referrer, detects replacement loaders, and maps event
stream lag/closure explicitly. Its dedicated real-Chrome test passed through
typed Page dispatch and verified context cleanup plus external-browser
survival. Cross-browser adapters and lifecycle-managed Response objects remain
open parity work.

The navigation-wire checkpoint adds `pwright-codec-navigation` as a separate
optional crate. It canonically encodes both the low-level navigation primitive
and Playwright-shaped goto commands/results without Serde, validates bounded
strings and response headers before allocation, rejects malformed tags,
truncation, trailing bytes, attachments, and noncanonical header order, and
preserves all operation failure kinds and navigation error details. A Tokio
duplex contract test proves a typed `PageGotoExt::goto` request and response
through client wire, stream framing, server wire, and bounded runtime dispatch.
Its normal dependency tree contains only the navigation capability, core, and
transport crates.

The first locator-action checkpoint adds core-only `pwright-cap-locator` and
the independent `pwright-client-locator` extension. A Locator remains a lazy
value and sends its selector expression with each typed click; the Chromium adapter
re-resolves every retry and checks strictness, attachment, visibility,
two-animation-frame stability, enabled state, viewport intersection, and hit
target before dispatching a mouse move followed by a balanced press/release.
The whole sequence now uses the same fixed-size Page input transaction as
Hover, including local-frame and OOPIF routes. A dropped transaction retaining
a pressed button is released by controlled cancellation or repaired before the
next Page input transaction; a failed release is retried during cleanup before
the original error returns. `force`, `trial`, mouse button, and nonzero click
count are explicit portable options. A controlled-runtime
test proves that one `CallMetadata` deadline terminates the retry loop without
input, and a dedicated real-Chrome test proved typed client dispatch, an actual
DOM click, context cleanup, and survival of the externally owned browser.
The selector expression now supports validated CSS plus first-class text,
label, and typed ARIA-role leaves; positive/negative `nth`; normalized
exact/substring text filters; element-identity intersection; deduplicated
ordered union; and `Within` descendant chaining. Page, Frame, and Locator gain
opt-in Playwright-shaped `get_by_*` factories without adding these dependencies
to the base client. Role options cover Playwright's name, description, checked,
disabled, expanded, include-hidden, level, pressed, and selected filters, and
invalid role/state combinations fail before backend dispatch. Both the CDP
compiler and wire decoder bound recursive complexity; the CDP compiler also
bounds aggregate string bytes. A real-Chrome test executes chained dialog and
button roles plus label and text clicks while excluding an out-of-scope
same-name button. Regex matchers, complete W3C accessible-name/implicit-role
conformance, cross-frame chaining, other actions, navigation coupling, and
call-log-rich timeouts remain open.

The following Click-parity audit found a protocol-evolution constraint before
adding more fields. At Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`, public Locator click also accepts
position, modifiers, delay, scroll, and interpolated mouse-move steps, while
navigation waiting remains part of the action. Pwright's existing
`pwright.locator.click_options` is a positional record published since locator
1.0. Such records have no enclosing length or unknown-field representation;
the compatibility checker therefore correctly rejects even appended optional
fields within major 1. This checkpoint does not mutate that layout, falsely
claim 1.18 compatibility, or expose options the backend cannot honor. The next
protocol design must choose between a locator major-version transition to an
extensible tagged option bag and an appended versioned click command that keeps
the legacy route. The decision must also cover unknown-field bounds,
canonical ordering, negotiation, navigation-wait composition, and avoiding a
new command for every later Playwright option.

The bounded extensible-record prototype now removes the wire-format unknown
from that decision. Its envelope is `u32 record_bytes`, followed by fields in
strictly increasing nonzero `u16 tag` order; every field carries its own `u32
value_bytes`. The writer backpatches both lengths in one destination buffer,
and the reader checks the declared record bound before borrowing field slices.
Known fields must consume their exact slice; unknown fields are skipped without
allocation. Schema metadata requires a fixed maximum encoded size and stable
tags. Within one major, an existing tag, field contract, and record-size range
cannot change or disappear; only a new optional tag is compatible. The YAML
generator prototype accepts `encoding: extensible`, a bounded `encoded_size`,
and tagged fields, then generates the codec and catalog metadata. A separately
compiled generated option bag round-trips while skipping an injected unknown
tag. This establishes a viable primitive but deliberately does not yet migrate
the published locator 1.x Click record or choose its new major-version route.

The major-transition decision is now narrowed: locator keeps the single
logical ID `pwright.locator`, while an installation may offer separate 1.x and
2.x tracks. Each client request and accepted connection selects exactly one
track. This avoids both replacing old clients and permanently accumulating
version-suffixed Click RPCs. The transport manifest now keys support by
capability ID plus major, rejects duplicate tracks, negotiates only the
requested major, and still rejects multiple selected majors in one server
hello. Release tests cover 1.x selection when 2.x is also installed.

The Chromium production composition now closes this publication gate. An
opt-in locator profile advertises separate 1.22 and 2.5 support ranges assembled
from separate generated inventories. After handshake acceptance it resolves
the selected locator major and constructs only that major's codec and handler
tree before any application frame is decoded. Unknown majors fail closed, and
the legacy v1-only builder returns no handler for a v2 connection. Removing the
v2 opt-in restores the original one-track manifest and leaves resource
requirements unchanged. A production round trip proves that accepted v2
Click, Focus, Blur, Press, SetChecked, SelectOption, and ScrollIntoViewIfNeeded commands pass through
the selected outer handler, runtime object validation, Page routing, and the
CDP backend.
The canonical all-capabilities installer now consumes that selected-major
handler instead of the legacy v1-only builder, and its external-Chrome
conformance recipe is wired for an opt-in locator 2.x handshake.

The frozen locator 2.0/2.1/2.2/2.3/2.4 wire artifacts and current 2.5 artifact remain separate
from locator 1.x. Their bounded extensible Click record uses optional
tags 1 through 10 for the Playwright options button, click count, delay, force,
modifiers, `no_wait_after`, position, scroll, steps, and trial; timeout and
cancellation continue through call metadata. A zero-field record encodes the
default option set in exactly four bytes. Release fixtures round-trip the full
record, skip an injected future tag, validate the 128-byte body bound, and
assert the v1/v2 versions and positional/extensible kinds separately. Steps
are additionally capped at 10,000 so one small request cannot create
unbounded sequential CDP work.

The public v2 mapping preserves absent values separately from explicit false,
zero delay, and empty modifiers. `LocatorV2Codec`, generated v2 server
bindings, and the same-named client `click` method round-trip all ten fields
through the real stream. The CDP adapter now covers actionability, position,
scroll, force, temporary modifiers, trial, delay, correct multi-click pairs,
bounded interpolated steps, cancellation cleanup, and requested main-frame
navigation commit waiting. Generated multi-command server tables are balanced
binary trees rather than left-deep chains and split capabilities above sixteen
commands into two typed groups; this preserves routing order and keeps the
current dual-major production build tractable.

The SetChecked checkpoint adds command 20 on both tracks without duplicating
Check and Uncheck routes. Their client extensions fix the desired boolean and
dispatch the one typed primitive. Both majors encode the same bounded
four-field extensible option record, while frozen locator 1.19 and 2.2 peers
reject it before payload creation. Chromium performs strict follow-label state
inspection, idempotent no-op, radio protection, shared Page pointer input,
trial actionability, and a terminal post-click assertion without replaying an
acknowledged click. The detailed source evidence, state machine, resource
boundary, and remaining acceptance gates are recorded in
`docs/knowledge/pwright-locator-set-checked.md`.

The SelectOption checkpoint adds command 21 on both tracks. Its request is
bounded to 1,024 descriptors, 64 KiB per string, and 1 MiB aggregate string
bytes; its result has the same count, item, and aggregate limits. The codec
rejects old 1.20/2.3 peers before payload creation, and the Chromium adapter
commits only after all requested options are found and enabled. It uses no
pointer input, navigation barrier, DOM-handle retention, or mandatory Handle
capability. The cross-world ElementHandle overload remains explicitly open.

The selector-runtime lifecycle checkpoint removes the full semantic runtime
from the actionability retry script. One shared `OnceCell` lazily installs it in
the current JavaScript execution context, concurrent cloned services share the
installation, and retries use a compact script. A document replacement returns
an explicit missing-runtime state and triggers an idempotent reinstall without
registering `Page.addScriptToEvaluateOnNewDocument` on an externally owned
browser. Three fixed counters expose successful installs, compact evaluations,
and post-navigation misses without retaining call history. Unit tests cover
concurrent first use, malformed installation acknowledgement, compact retries,
and simulated context replacement; the real-Chrome test now navigates between
the role click and the label/text clicks using the same service.

The full-recipe concurrency checkpoint exposed a browser-scheduling edge that
single foreground-Page tests missed: Chrome suspends `requestAnimationFrame` in
background Pages, so an unconditional two-frame stability wait can retain a
Locator command indefinitely. Pointer and scroll actionability scripts now use
the two-frame check only for visible documents and a zero-delay timer yield for
hidden documents. The same proof also aligns the full profile's supervisor
in-flight limit with its four runtime active-command slots, then overlaps two
Locator clicks and two slow navigations without lag or external-browser
ownership leakage.

The first non-click locator-action checkpoint adds the typed `Fill` command,
`FillOptions`, and opt-in `LocatorFillExt` without changing generic Locator
storage. Its CDP state machine re-resolves on every retry, retargets a label to
its associated control, waits for visible/enabled/editable state, uses CDP text
insertion for text-like controls and contenteditable elements, and uses native
constrained-value semantics plus `input`/`change` for date-like inputs. Empty
fill is selection plus Delete. `force` bypasses actionability checks but not
target/type validation. Invalid targets and values are distinct portable
errors; direct fill values are bounded at 1 MiB. Unit tests cover retry, input
dispatch, clearing, constrained fill, typed terminal errors, metrics, and
resource bounds. The dedicated real-Chrome test verifies readonly auto-waiting,
label retargeting, contenteditable fill, date fill/events, cleanup, and survival
of the externally owned browser.

The locator-wire checkpoint adds `pwright-codec-locator` without making it a
dependency of the capability, typed client extension, or in-process backend.
Its canonical bounded payload preserves click/fill options and values, count
and visibility results, execution metadata, success, every failure category,
and typed locator error details. The selector and command vocabulary advances
this independently negotiated capability through 1.7 without changing the common
frame protocol. Malformed routes, attachments, strings, selector values, ARIA
roles/state combinations, recursive depth/node counts, durations, enum/option
tags, click counts, truncation, and trailing bytes fail explicitly. Tokio
duplex tests prove the complete typed remote click, count, fill, and
client-composed hidden paths,
while a tampered-identity test proves the server rejects inner/outer call-ID
disagreement before backend dispatch. The codec adds only the capability,
shared common/binary codecs, core, and transport as normal dependencies. The
capability itself still depends only on core, and the optional generated
locator server selects no runtime, backend, CDP, browser provider, Node.js, or
browser download.

The first explicit Chromium production root now lives in
`pwright-composition-chromium`. Its preparation phase selects navigation,
locator, or both; derives one additive concurrent resource plan; filters the backend's
potential feature set to the capabilities actually installed; constructs the
wire capability inventory; and provides plan-checked factories for capability
protocols, the complete-topology runtime, and the handshake-first supervisor.
Only the separate `attach` method receives an endpoint or performs I/O, and it
uses `ProductionCdpConnector`; the composition has no Node.js, browser install,
discovery, launch, or process-termination dependency.
The reusable `ChromiumAttachFactory` holds only that immutable recipe and
per-connection limits. It creates a new ledger for every preparation and never
clones a prepared or attached object graph.

This checkpoint also fixes a prerequisite routing flaw: runtime dispatch used
to validate the target `ObjectId` and then discard it before backend service
invocation. `TargetedService` now carries that identity through both direct and
controlled dispatch while an automatic compatibility implementation preserves
ordinary `Service` users. A backend descriptor can now be strictly validated
as a complete declaration or filtered against an installed capability
registry. Tests prove navigation-only advertisement, omitted locator resource
rejection, exact ledger identity on encoded frames, initial runtime/supervisor
state, target forwarding, and preparation without an endpoint.

The page-routing checkpoint makes that root execute against multiple concrete
CDP sessions. A bounded `CdpPageServices` table maps each materialized portable
Page `ObjectId` to the exact target/session pair and removes detached or
reconnecting routes. Navigation and locator service factories share each
profile's concurrency counter across every bound Page while locator runtime
installation remains target-local. The route profile accounts the two native
ID pairs retained by the routing entry and `CdpSession`; the composition plan
uses saturating additive combination because connection, topology, routes,
runtime, backend calls, and wire staging may all be live together.

Both direct and caller-policy controlled page runtimes are available. The
controlled path composes deadline and bounded-cleanup plugins and forwards
command cancellation to the Page's session. `synchronize_topology_update`
applies emitted Create/Dispose ownership to the runtime and refreshes routes
from the same committed topology snapshot; normal callers no longer need to
maintain those two views independently. Tests cover exact-session routing,
unchanged-route reuse, navigation cleanup routing, detached-route rejection,
runtime subtree disposal, and cross-Page local-concurrency enforcement.

The topology transaction checkpoint turns that ordered bridge into a
rollback-capable composition boundary. `ProductionCdpTopology` exposes a
resource-accounted prepared shadow that publishes only on `commit` and releases
its candidate lease on drop. Runtime Create/Dispose batches now preflight the
complete object graph, semantic parent kinds, pending-call policy, lifecycle
outbox, object leases, and cancellation signals before replacing live state.
The route table similarly prepares its bounded replacement while excluding
command lookups. `transact_topology_update` composes those pieces so every
fallible step precedes the runtime commit and only infallible moves follow it.
AAA tests prove prepared-topology rollback, runtime batch rollback, successful
three-state publication, and composition-level rejection without a partial
Page, main Frame, or route.

The selected-handler composition checkpoint adds plan-checked navigation and
locator handler factories to the Chromium root. Each returns `None` for an
omitted capability and otherwise binds both generated commands to the caller's
shared runtime using a protocol from the same ledger. The handler crates add no
runtime, provider, browser, Node.js, or download choice. `Arc<runtime>` now also
implements the lifecycle control-source boundary, so the command handlers,
client-control receiver, and outbound control driver can share exactly one
runtime instance.

The remote-server resource checkpoint now charges every command future retained
by `EncodedServerSupervisor` as a call-scoped `PendingCall`; the composition
adds that lease to the runtime's lease for the same call instead of taking the
old maximum. Duplicate active call IDs therefore fail at the resource boundary,
and every terminal path drops the supervisor lease. An optional remote
handshake profile adds the transport declaration and server-hello encoding
peak only when selected. It caps the peer's advertised receive frame size
before authorization or response allocation and emits a frame owned by the
same ledger. An optional control profile similarly accounts its outbound
protocol plus every retained `(ObjectId, EventTag)` subscription route. Remote
control requires the handshake component and both share one peer frame bound.

The caller-driven remote-loop checkpoint adds a generic outbound lane to the
unique-reader supervisor. Receive, active commands, and outbound lifecycle
sends rotate priority after every completion; commands retain their existing
round-robin order. An idle outbound lane is parked on an optional caller-driven
readiness future, so it cannot create a ready-loop, while an outbound error is a
distinct terminal reason. Sources without readiness notification retain the
connection-progress fallback. `RuntimeControlOutbound` adapts the runtime lifecycle source without a
task, listener, executor, or socket choice. The Chromium root can now build
inbound control and outbound lifecycle halves from the accepted handshake, one
shared runtime, transport, and production ledger. Navigation and locator also
have accepted-handshake handler factories; preparation rejects any installed
capability whose exact version, inbound limits, or peer frame ceiling diverges
from the remote handshake contract.

The typed server-event checkpoint adds the missing producer boundary as the
independent `pwright-server-events` crate. One channel installs one static
`EventSpec`; its cloneable producer retains typed payloads in an explicitly
bounded queue and its single outbound consumer performs codec validation and
transport I/O. `RejectNewest` and `DropOldest` remain observable outcomes. A
temporarily claimed oldest event continues to occupy logical queue capacity,
so lossy overflow cannot displace an event already being encoded or sent.
Encoding or transport failure, and cancellation before completion, restores
the claim at the front for ownership and accounting; only a successful send
releases its `EventItem` and `EventBytes` leases. Because a generic transport
may already own a partial write, cancellation or transport failure is terminal
for that connection and does not authorize retry there. The codec's emitted
route must exactly match the typed source and event tag.

`ServerEventProductionProfile` adds the complete retained queue to the maximum
typed-source/encoded-frame overlap. Production construction rejects a protocol
whose bound differs from the declared profile and rejects a transport backed
by another ledger. Static outbound chains alternate successful plugin lanes;
they never start the next lane while the selected lane has an active send.
Chromium composition accepts zero or more explicit
`ChromiumRemoteEventProfile` installations. Their tags must be unique, they
require the remote handshake, their declared wire contract must match it, and
the accepted-handshake factory returns `None` for an omitted event type.

The BrowserContext-creation checkpoint implements the approved no-options
slice without making it a mandatory dependency. Separate capability, codec,
client-extension, generated-server, CDP-backend, and Chromium-composition
layers implement `Browser::new_context`; the capability declares that context
lifecycle must also be installed. The runtime now has rollback-on-drop prepared
creation and the connection owns a nonzero, non-wrapping monotonic ObjectId
source. Any failure after native creation performs exact-context compensation.
The remote ordered writer physically sends Create before Result, and the
client's bounded announced/claimed state rejects a missing, mismatched,
duplicate, or already-disposed initializer. Saturated continuation capacity
rejects before CDP. A dedicated Chrome 152 proof created a real context and
target, closed them through the portable lifecycle service, and verified the
external browser survived. This is evidence toward DT-020, DT-021, DT-030,
DT-032, DT-033, DT-040, DT-041, DT-043, DT-052, DT-060, DT-061, and DT-070;
options, other backends, and broader API parity remain open.

The post-handshake composition checkpoint now makes the fresh remote unit a
staged state machine rather than a bag of helpers. Handshake acceptance consumes
the runtime-installed connection and advances only its own supervisor. A
one-shot application installer receives an owned, non-constructible context
containing that accepted handshake, exact transport, attachment, runtime, and
optional unique topology-event driver. Selected navigation, evaluation,
locator, Context/Page creation, and Context/Page lifecycle factories are
available on the attached root, while typed event sender/outbound pairs remain
independent plugins. A selected topology driver must be transferred out of the
context before installation can succeed. The root always places native CDP
liveness before the caller backend, and the final application `run` always uses
the complete supervisor path that shuts down its exact transport. Dedicated
Chrome 152 checks prove same-supervisor handshake acceptance, negotiated
navigation-handler construction, topology-driver consumption/rejection,
mandatory liveness typing, and one exact transport shutdown on EOF. This is
evidence toward DT-021, DT-030, DT-032, DT-040, DT-041, DT-043, DT-051,
DT-052, DT-060, DT-061, and DT-070.

The first concrete capability-subset recipe now installs an
`Arc<DirectDispatcher<CdpPageServices>>` and only the two negotiated navigation
handlers. `NoServerControl`, `NoServerOutbound`, and `NoServerBackend` make all
three omitted lanes explicit; the composition root still adds mandatory native
CDP liveness. Installation rejects a profile with no navigation capability, a
peer that did not negotiate it, or a selected topology-event driver that this
minimal recipe cannot own correctly. A dedicated Chrome 152 test pre-provisions
one Page, then proves handshake, `BeginNavigation`, typed result decoding,
drain-on-EOF, exact transport shutdown, and external-browser survival in one
connection. This is deliberately a pre-provisioned-Page recipe: automatic Page
discovery requires lifecycle control plus an owned topology backend.

The following automatic-discovery assembly checkpoint removes the
self-reference that previously blocked that larger recipe.
`ChromiumTopologyServerBackend` now owns an `Arc` runtime and a cloned ordered
control writer while retaining exactly one in-flight pump future. The
discovering-navigation runtime installer selects a controlled Page dispatcher
with caller-bounded cleanup capacity. Its application installer requires the
navigation, remote-control, topology-event, and page-discovery components,
consumes the unique topology driver, installs an inert portable-event publisher,
and places the owned topology pump in the final backend lane. Existing tests
still prove retained-turn and terminal-drop semantics; a dedicated Chrome 152
test proves two-capability negotiation, final type assembly, unique-driver
consumption, and external-browser survival. The next live proof found and fixed
an important compositional wake-up gap: an inert portable-event publisher must
still fence every topology update through its final lifecycle `Create`, because
an ordinary outbound lane may already have observed an empty runtime queue.
A ledger-verified full-duplex memory transport now runs a real client handshake
and lifecycle driver beside the complete server supervisor. The client learns
Browser/Context/Page identities only from control frames, constructs a typed
Page, navigates real Chrome, completes `Close`/`CloseAck`, and leaves the
external browser reachable.

The caller-driven endpoint-server checkpoint packages the remaining listener
edge without introducing a service loop. `ChromiumEndpointAcceptor` gives the
pre-bound Unix, Tailnet-PSK, and mTLS endpoints one static one-connection
accept boundary; `AllowedUnixEndpoint` keeps the Unix listener and its allowed
UID together. `ChromiumEndpointServer` combines one such caller-selected
endpoint with the stateless attach factory and an explicit external CDP URL.
Each explicit `accept_connection` first accepts and authenticates one peer,
then creates a fresh ledger and CDP attachment for only that peer. It creates
no task, runtime, retry policy, accept loop, or default deadline. A rejection
ordering test proves failed endpoint authentication cannot reach CDP, and a
dedicated Chrome 152 test proves that the packaged Unix listener derives the
real kernel UID, completes the Pwright handshake on the same connection, and
leaves externally owned Chrome reachable afterward. Admission concurrency and
deadlines remain deployment policy.

The complete current-capability application checkpoint adds
`ChromiumAllCapabilitiesRuntimeInstaller` and
`ChromiumAllCapabilitiesApplicationInstaller` as an opt-in recipe above the
same staged connection. It refuses partial installation: Context creation and
lifecycle, Page creation and lifecycle, value evaluation, navigation, locator,
Handle evaluation and lifecycle, screenshot, DOM content, Page events, remote
control, Page discovery, topology events, the explicit Playwright-compatible
Chromium utility world, and all three remote Page-event queues must be selected,
while all twelve application capability IDs plus control must be negotiated.
The runtime installer requires the Handle route table and wraps its dispatcher
in the Handle-aware transport-close owner. The final static type chains every
command handler, activated subscription control, ordered lifecycle output,
three bounded event outbounds, topology discovery, native liveness, and a new
independent `PageLoad` backend. The PageLoad backend owns only another broadcast
cursor, never another CDP reader or task, so it and topology ingestion observe
the same bounded native event stream without stealing events from each other.
Cleanup remains a caller-driven bounded queue, and Page-close polling remains
an injected policy. Unit proof covers the independent cursor and exact profile;
a dedicated Chrome 152 proof assembles all thirteen negotiated protocols and
then reattaches after drop to prove external-browser ownership is unchanged.

The Chromium utility-world checkpoint is an explicit infrastructure profile,
not a hidden consequence of attaching to a browser. For each Page and OOPIF
session it follows Playwright's ordering: enable Page, acquire the complete
frame tree, enable Runtime, register a named new-document world, create that
world for every existing frame, install recursive auto-attach, then resume a
paused target. Names include the portable Page ID instead of using one global
spoofable constant. A fresh frame tree may forgive one creation error only when
it proves that exact frame disappeared; arbitrary CDP failures remain visible.
The router waits for each matching Runtime event before publishing that
Frame's locator route. Main, local-child, and OOPIF Frames bind their exact
session and utility `uniqueContextId`; Page calls delegate to the stable main
Frame. World removal fails closed, and replacement constructs a fresh
selector-runtime cell. Portable Frame evaluation remains in the main world.
Fake-CDP tests cover ordering, the disappearing-frame race, strict failure,
all three Frame host shapes, world replacement, and routing isolation. A
dedicated Chrome 152 test writes a global in the utility world, proves it is
absent from the main world, then creates a button only inside a child iframe
and resolves a trial click through that Frame's portable object ID. It cleans
up the exact target and confirms the externally owned browser remains
reachable.

The exact suite count is reported by the verification command rather than
frozen here. The v2 foundation passes its non-browser release tests plus the
compile-fail contract doctests. The changed v2 crates pass release Clippy with all warnings
denied, and the full workspace passes rustdoc with warnings denied. A full
workspace Clippy run on the current toolchain is separately blocked by the
legacy tonic server's newly reported `result_large_err` lint, including
generated tonic code; this checkpoint does not change that v1 ABI.
`pwright-runtime`, `pwright-transport`, and `pwright-browser-backend` retain only
`pwright-core` as a normal dependency. This is further evidence toward DT-032,
DT-033, DT-050, DT-051, DT-052, DT-060, and DT-061, not completion: complete
cleanup retry/drain supervision, complete generated codecs and cross-language types,
safe concurrency bypass rules,
production drain-duration policy, multi-command transactions, remaining
selector conformance, optional cross-world ElementHandle adoption,
event-class assignments and backend subscription activation, cross-language
payload compatibility fixtures, cross-browser goto backends,
response/request object lifecycles, and
full Playwright navigation conformance remain open. The explicit
real-Chrome tests remain ignored by default unless a caller supplies the
required browser endpoint; the typed locator action test was rerun against
Chrome 152 at this checkpoint and covered click, selector-runtime recovery,
all implemented fill paths, immediate state getters, the complete
missing/attached/visible/hidden/detached wait cycle, and strict retrying
text-content reads with exact whitespace preservation, rendered inner-text
that excludes hidden descendants, typed inner-text rejection of an SVG target,
and exact HTML/SVG inner-HTML serialization.

## Phase 0: establish the evidence

### DT-001: Current-system baseline

- Question: What does today's Pwright actually support, depend on, allocate,
  and expose?
- Deliverable: verified capability, dependency, process, memory, and lifecycle
  inventory of the existing workspace.
- Done when: every current public surface and important operational behavior is
  mapped to its owning crate, and unknowns are explicitly listed.

### DT-002: Playwright reference map

- Question: Which Playwright behaviors are contractual targets, useful design
  references, or intentional non-goals?
- Deliverable: source-linked map covering API objects, channel protocol,
  dispatchers, auto-waiting, selectors, events, errors, browser backends,
  installation, and resource lifecycle.
- Done when: “Playwright-compatible” has separate, testable meanings for API
  shape, observable behavior, protocol compatibility, and browser coverage.

### DT-003: Memory investigation plan

- Question: How will we distinguish Pwright runtime memory from browser memory
  and identify retained state?
- Deliverable: measurement protocol and workloads for idle, navigation,
  repeated page creation, handles, network traffic, tracing, and teardown.
- Done when: measurements can separately report the client, runtime, browser
  parent, renderer, GPU, and utility processes, plus internal object counts.
- Checkpoint: the optional, standard-library-only `pwright-process-metrics`
  crate now reads a bounded process-table snapshot, retains no command lines,
  validates the PID graph, and separately reports one Pwright runtime plus the
  browser parent, renderer, GPU, utility, other-child, and total RSS-sum groups.
  Its opt-in `ledger` feature adds a bounded caller-driven recorder without an
  executor, timer, browser provider, CDP client, or background task. The plan
  fixes workload kind, cold baseline, cleaned warmup observations, steady-state
  observations, sample interval, and teardown deadline. Every observation
  correlates the same runtime/browser PID roots with OS attribution,
  `ResourceMetrics`, and browser object counts; phase order, monotonic time,
  root stability, capacity, completeness, and teardown duration fail closed.
  A release test against dedicated Chrome 152 performed four real portable
  BrowserContext+Page create/close cycles and returned all logical state to its
  cleaned warm baseline. A second driver performs a complete lifecycle warmup,
  three Playwright-shaped `goto(wait_until=load)` operations, and final
  teardown. It exposed and fixed an invalid immediate-post-close measurement:
  the drivers now bound and poll Chrome process settlement, and the report
  independently verifies that temporary browser helpers returned to the warm
  process count. Warmup itself waits for a stable count rather than requiring
  equality with cold startup, because first use may legitimately start a
  persistent helper. The navigation driver now repeats the complete fresh
  attachment/runtime workload three times; a Chrome 152 run held six browser
  processes throughout and observed runtime RSS deltas of
  `+131,072..+1,310,720` bytes and browser RSS-sum deltas of
  `+229,376..+4,947,968` bytes. A third driver installs Handle evaluation/lifecycle
  independently, proves composition rejects topology-plus-Handle capacity that
  exceeds runtime capacity, then correlates three retained ElementHandles
  across route, runtime-object, and `RemoteHandle` ledger boundaries. Explicit
  disposal releases one while Page teardown parent-retires the other two. A
  fourth driver measures the minimal idle external attachment from pre-attach
  cold state through three steady samples and explicit runtime/CDP teardown. It
  proves the sole Browser object and connection leases release, the sticky CDP
  closed signal resolves, and a fresh attachment still reaches the externally
  owned Chrome. A fifth driver installs the single-cursor network component
  before Page creation, performs one complete Context/Page/network/close
  warmup, reuses the exact one-lane capacity, and then runs three data-URL
  network lifecycles on a persistent Page. It samples only after ordered
  lifecycle publication, exact batch acknowledgment, zero native request
  correlation, and an empty control outbox. Explicit Page and Context close
  retire their driver adapters only after local publication; failed Page close
  preserves the live adapter. The first run exposed portable retention directly:
  steady `LiveObject.current` rose `6 -> 8 -> 10` as three completed
  Request/Response pairs remained beneath the Page. The corrected base contract
  now sends complete terminal value snapshots and retires each Request root
  after every selected event queue accepts the batch. A subsequent Chrome 152
  run held the same persistent Page at `[4, 4, 4]`, returned logical state to
  baseline, reduced the browser tree from 10 to 8 processes, and observed
  post-minus-cold RSS deltas of `+1,671,168` runtime bytes and `-231,620,608`
  browser RSS-sum bytes. These are one-run diagnostics, not
  budgets. The bounded series model now requires an identical plan and exact
  run count, rejects any run before logical cleanup and browser-process
  settlement, and reports RSS delta ranges without applying an unreviewed
  threshold. The current driver selects the task-free reclaiming fanout with
  two plugin-derived cursors, places a CDP response barrier after navigation
  and teardown, drains unrelated tail events, and asserts zero queued raw
  events at each boundary. Two consecutive three-run Chrome 152 network series
  kept every persistent Page at `[4, 4, 4]` and returned every resource class,
  including `EventBytes`, exactly to the warm baseline. The process-stable
  second series observed `+98,304..+1,425,408` runtime bytes and
  `+884,736..+5,357,568` browser RSS-sum bytes. Owners, scopes, leases, objects,
  and native correlation remain strict. A later warmup-settlement run held six
  browser processes across all three reports with runtime deltas of
  `+81,920..+1,490,944` bytes and browser deltas of
  `+212,992..+4,849,664` bytes. A separate complete-recipe Chrome 152
  proof now overlaps two slow navigations and two Locator clicks while topology,
  PageLoad, and shared network cursors remain active. Its fanout peak metric
  observed 6 or 7 cursors beneath the safe derived capacity of 10, then returned
  to three current cursors, zero queued events, and zero lag. This validates the
  allocation boundary but is not a tracing or RSS workload. The
  network codec now derives the largest terminal payload and
  retained snapshot size from the same metadata limits, and preparation rejects
  a smaller remote frame or event-byte budget before CDP I/O. Owned-browser
  teardown now has a truthful terminal observation: a non-constructible proof
  comes from one bounded process snapshot containing the runtime PID but not
  the exact owned Browser PID. The same workload recorder accepts that proof
  after ordinary two-root baseline/warmup/steady/pre-teardown samples, checks
  roots, deadline, resources, and objects, and never invents zero browser RSS.
  A bounded three-run series launched distinct Chrome 152 processes; all exited
  gracefully, released every ledger lease, proved their exact PIDs absent, and
  removed their temporary test profiles. Runtime teardown deltas were
  `+32,768..+589,824` bytes and measured pre-teardown browser RSS sums were
  `1,204,158,464..1,208,762,368` bytes. Tracing,
  the remaining non-network benchmark series, Linux
  PSS, and reviewed budgets remain open; see
  `knowledge/pwright-process-memory-metrics.md`. The first network prerequisite
  now exists: a core-only bounded lifecycle model and optional event contract,
  plus a Chromium normalizer that reserves its complete active-correlation
  `BackendStateBytes` ceiling. A Chrome 152 release proof observed one real
  request/response/finished lifecycle with zero retained native correlation
  and no body read. This is implementation evidence, not the DT-003 network
  memory workload by itself. Local production composition now adds a caller-polled Page
  cursor, atomic runtime lifecycle admission, final-Create wire fencing, four
  bounded fail-closed typed queues, Page-parent retirement, and explicit lane
  plus runtime-child capacity. Remote delivery now uses one dedicated canonical
  codec/outbound and one sequence across all four public event tags; generic
  per-event outbounds remain rejected. A one-slot production client handler ran
  4,096 deliveries with one peak lease and zero retained leases, while the
  native tracker ran 4,096 completed exchanges with no retained history. A
  second Chrome 152 proof covers redirect ordering, HTTP 500, and truncated-body
  transport failure. Page-scoped dynamic server activation now requires an
  exact pre-registered cursor, keeps four tags in one bounded mask, filters the
  ordered remote stream, disables only the final network-only owner, and
  preserves a selected navigation owner. One attachment-level caller-polled
  driver now routes one CDP cursor through a bounded exact-session adapter
  table. Empty Context masks are retained independently, Page native ownership
  uses the direct/inherited union, and dual Page/Context publication is Page
  first with shared portable identities. Context first/last edges now run one
  bounded exact-session batch, commit only after all sessions succeed,
  compensate definite failure in reverse, and retain an idempotent inverse
  cleanup plan when the future is cancelled. Explicit
  `BrowserContext.newPage` now registers and, when inherited interest exists,
  enables a provisional exact-session adapter before Page/Frame publication;
  the cursor holds a racing provisional-session event until commit, while
  publication failure disables the domain, retires the lane, and closes the
  target. Cancellation recovery is explicit and idempotent. Automatic new-Page
  discovery now uses the same transaction and detaches its session after
  publication rollback. Session replacement uses one resource-declared shadow
  adapter, preserves direct/Context masks, joins old active-Request cleanup to
  runtime rebind, and has success, rollback, and cancellation-recovery proofs.
  Parent detach retires the lane only after topology/runtime commit. The client
  exports Page and BrowserContext first/last-listener helpers, while the server
  driver owns a receive/publish/exact-acknowledgment turn. Application installer
  assembly and the first process-RSS network workload are implemented. Repeated
  network series across other named hosts, tracing/owned-browser teardown
  series, and reviewed budgets remain open.

  The upstream comparison now identifies one concrete retention candidate in
  the local Playwright `1.63.0-next` checkout: terminal events reuse but do not
  dispose Request/Response dispatchers, while the generic per-type bucket waits
  until 10,000 entries before trimming its oldest 10%. Pwright's base
  `pwright.network-events` capability now removes native correlation, publishes
  a complete terminal Request/optional-Response value snapshot, and then
  disposes the temporary Request root plus Response child. Post-terminal live
  handles are intentionally deferred to a separate capability with hard
  object/byte limits, explicit client acknowledgment, disconnect cleanup, and
  admission backpressure.

## Phase 1: define product boundaries

### DT-010: Users and installation profiles

- Question: Who uses Pwright, and what is the smallest install for each user?
- Deliverable: a short set of profiles such as embedded Rust library, agent
  CLI, remote client, remote runtime, system-Chrome attachment, and optional
  managed browser.
- Done when: every profile lists required and forbidden dependencies, binaries,
  browser ownership, and delivery surfaces.

### DT-011: Compatibility contract

- Question: How closely must Rust Pwright match Playwright?
- Deliverable: compatibility levels for naming, API shape, behavior, errors,
  events, selectors, auto-waiting, protocol, and test reuse.
- Done when: each level is classified as exact, adapted for Rust, deferred, or
  intentionally incompatible.

### DT-012: Scope and non-goals

- Question: What belongs in v2, and what must remain outside it?
- Deliverable: explicit v2 scope, later scope, and non-goals.
- Done when: Chromium-only versus multi-browser, attach versus launch, local
  versus remote, and library versus CLI/server responsibilities are settled.

### DT-013: Success budgets

- Question: What quantitative limits define a successful rewrite?
- Deliverable: budgets for binary size, dependency count, idle RSS, per-page
  overhead, object counts, event queues, startup time, and operation latency.
- Done when: each budget has a workload, measurement method, target, and failure
  response rather than an unmeasurable word such as “lightweight.”

## Phase 2: design the public contract

### DT-020: Rust API object model

- Question: Which Playwright objects exist in Rust, and which are resources,
  values, builders, or views?
- Deliverable: object hierarchy and ownership table for Playwright,
  BrowserType, Browser, BrowserContext, Page, Frame, Locator, handles, network
  objects, workers, downloads, and artifacts.
- Done when: clone, Send/Sync, lifetime, close/dispose, and equality semantics
  are defined for every public object category.
- Checkpoint: `Page.content()` is designed as local delegation to the Page's
  initialized main `Frame`; only `Frame.content` is a wire command, so child and
  OOPIF Frames remain independently addressable without a duplicate Page
  protocol surface.

### DT-021: Rust method conventions

- Question: How should async calls, options, builders, return values, streams,
  callbacks, and errors feel idiomatic without drifting from Playwright?
- Deliverable: representative signatures for navigation, locator actions,
  evaluation, events, contexts, downloads, and shutdown.
- Done when: the conventions handle cancellation and optional capabilities
  consistently and do not require CDP types.

### DT-022: Selector and locator semantics

- Question: What exactly makes a Locator lazy, strict, retryable, composable,
  and compatible with Playwright?
- Deliverable: selector AST, locator composition rules, resolution boundary,
  strictness behavior, and supported engines.
- Done when: CSS, text, role, label, filter, nth, and/or composition have
  unambiguous observable semantics.

### DT-023: Actionability and auto-waiting

- Question: What conditions and retry rules govern click, fill, navigation,
  assertions, and page transitions?
- Deliverable: action state machines and deadline rules.
- Done when: visible, stable, enabled, editable, event-receiving, navigation,
  detachment, retry, and timeout behavior can be tested independently.

## Phase 3: design internal boundaries

### DT-030: Client/runtime boundary

- Question: Is every call logically a channel command, and how does the
  in-process path avoid serialization and duplicated state?
- Deliverable: comparison of direct typed dispatch, an actor boundary, and an
  encoded channel, including costs and remote extensibility.
- Done when: object ownership and call flow are clear in local and remote modes.

### DT-031: Protocol schema and code generation

- Question: What is the source of truth for objects, commands, results, events,
  errors, metadata, and controls?
- Deliverable: evaluated IDL options and a minimal representative schema.
- Done when: Rust types, validators, dispatch tables, documentation, and
  compatibility metadata can be generated without binding the model to JSON.

### DT-032: Object identity and lifecycle protocol

- Question: Which entities require identity across calls, and how are create,
  adopt, dispose, collection, and disconnection represented?
- Deliverable: object-tree invariants and lifecycle state machines.
- Done when: leaked clients, duplicate dispose, parent close, stale handles,
  reconnect, and bounded collection have specified outcomes.

### DT-033: Calls, cancellation, deadlines, and errors

- Question: How does one API operation travel end to end and terminate?
- Deliverable: call state machine and typed error taxonomy.
- Done when: timeout, caller cancellation, target close, browser crash, protocol
  violation, unsupported capability, transport loss, and cleanup failure are
  distinguishable.
- Checkpoint: controlled runtimes expose a generic typed subtree interruption
  for non-disposal lifecycle failures. Chromium Page crash uses it to terminate
  queued and active calls as `BrowserCrashed` while keeping object identities
  live; Frame disposal remains `TargetClosed`, first terminal reason wins, and
  direct runtimes require the affected subtree to be idle.

### DT-034: Events and backpressure

- Question: How are subscriptions activated, queued, delivered, and stopped?
- Deliverable: subscription lifecycle and overflow policies by event class.
- Done when: slow or abandoned consumers cannot produce unbounded memory and
  critical lifecycle events cannot be silently treated like telemetry.

Implementation checkpoint (2026-09-03): the first concrete event slice is
`pwright.page-events/page.load`. Event capability metadata now has the same
stable ownership discipline as commands: semantic `EventId`, numeric wire
`EventTag`, target object kinds, duplicate-owner rejection, and server
inventory preflight. The event-only capability can be negotiated without
navigation or locator. Its fixed-size codec duplicates Page identity inside
the payload and rejects route/payload disagreement. Client installation uses
`RejectNewest`, because a Page lifecycle transition must not silently become a
telemetry window.

The Chromium adapter activates `Page.enable` followed by
`Page.setLifecycleEventsEnabled(true)`, reverses only the lifecycle toggle so
other Page-domain capabilities remain usable, and normalizes only current
main-frame `load` events to the stable portable Page ID. It owns no reader,
queue, or task. The server control path is now an ordered async barrier:
`SetSubscription` reserves capacity, awaits the selected backend activator, and
commits logical subscription state only after success. Failed enable and
disable operations preserve the exact prior route and resource state;
idempotent listener edges do not call the backend. Chromium composition wires
the PageLoad activator to the same synchronized Page/CDP-session route used by
commands, so a later command cannot overtake activation on the wire reader.

Object cleanup is now retryable and caller-driven: successful deactivation
edges release their logical route one at a time, while a failed edge and every
later route remain leased for another attempt. Chromium composition consumes a
committed topology update's Dispose effects; a Page route already retired by
that update makes disable a safe success rather than redirecting work to a new
session. Clean EOF uses a second async supervisor barrier, signals runtime
transport closure first, attempts every backend deactivation, releases every
local route even when cleanup fails, and reports the first plus an additional-
failure count. Both cleanup loops use constant additional memory.

An activation or deactivation awaiting CDP is now represented by one explicit
in-flight transaction. If its future is cancelled, pending enable retains its
reserved lease and pending disable retains the active route. Clean EOF can then
compensate either uncertain edge with disable before releasing ownership. The
supervisor polls each control once before considering later transport input,
preserving stream order. If the handler declares that this pending poll released
the input frame, it may retain exactly one next frame while watching for EOF;
that frame is not dispatched until the control succeeds. EOF instead drops the
control future and runs the async close compensation barrier. Conservative
third-party handlers opt out of prefetch by default.

The same async cleanup barrier now reaches one completed attempt for every explicit supervisor
terminal path, including receive, protocol-validation, command, control,
observer, outbound, command-capacity, and shared-resource failures. The hook
receives the typed close reason. `close_with_controls` is the explicit async
close API for owners that have a control/backend boundary; the older synchronous
`close` remains a logical close only. If the primary operation and cleanup both
fail, `TerminationCleanup` retains both errors instead of replacing the primary
cause. Terminal cleanup events are not delivered through an observer that is
already failing. A dropped run future still cannot await Rust async cleanup;
the next owner call with controls retries an unfinished barrier. The activated
receiver retains the route and its resource lease until each backend disable
future resolves, so cancelling cleanup cannot erase its retry state.

Event ordering is now an explicit portable transport contract. Each installed
typed event channel owns one monotonic `EventSequence`, starts at one, and
assigns the number before queue admission. A server-side reject, drop, or
resource failure therefore consumes a position instead of hiding loss. The
sequence never wraps: exhausting `u64` terminates emission with a typed error.
Backend adapters still return plain semantic `Event<Spec>` values; only the
server channel and codec handle `SequencedEvent<Spec>`, so ordering is neither
CDP-specific nor part of a browser-native payload.

The client accepts any positive first sequence as the connection-epoch
baseline, then distinguishes consecutive delivery from an explicit gap.
Duplicate, regressing, and zero sequences are protocol errors with separate
metrics. Sequence observation precedes local bounded-queue admission, so a
local `RejectNewest` remains visible as local backpressure instead of being
misreported as wire loss. Starting a new connection epoch releases stale queued
events and resets only the baseline; cumulative gap/error counters remain
observable. The fixed-size Page-event codec carries the eight-byte sequence and
source identity; Context-Page and popup variants add one related Page identity.
`pwright.page-events` is version 1.2 for the additive discovery-event surface.
The caller-owned topology publisher forwards only installed lanes and treats
queue reject/eviction as explicit event loss requiring reconstruction.

This completes the DT-034 foundation: subscription edges, backend activation,
bounded delivery, reasoned cleanup, ordering, and loss detection all have
portable contracts and executable tests. Individual future event capabilities
must still classify their own overflow policy and decoded-size limit.

Network-event design checkpoint (2026-09-04): Page network listeners require a
pre-registered caller-polled lane for the exact Page/session before the first
`Network.enable`. Four public tags share one bounded per-Page interest mask and
one ordered remote queue; the publisher filters only that remote queue from the
committed mask, while optional local queues remain independently installed.
The first edge enables the domain before committing, later edges only change
the mask, and failed or cancelled native transitions retain the prior state.
The last edge sends `Network.disable` only when network events are the sole
installed domain owner. A selected navigation plugin is initially a
connection-lifetime shared owner because its current `goto` operation enables
Network without an operation-scoped release. Page retirement forgets the exact
lane and logical mask after control cleanup. BrowserContext fan-out over
current Pages, explicit `newPage` inheritance, and automatic new-Page discovery
is implemented. Existing-Page discovery prepares and enables one separately
budgeted shadow adapter, carries the old direct/Context interests forward, and
cleans active Request roots in the runtime rebind. Failed publication restores
the old lane and disables/detaches the incoming session; cancelled activation
is explicitly recoverable. Page-session detach holds
the network state guard across topology/runtime publication, committing lane
retirement only after the Page subtree is disposed and preserving it on
transaction failure.

BrowserContext network design checkpoint (2026-09-04): Context fan-out uses
one optional caller-polled connection driver with one CDP cursor and a bounded
Page/session normalizer table. The driver, exact attachment identity checks,
explicit retirement, empty Context retention, separate direct/inherited masks,
effective native union, and Page-first dual-scope wire publication with shared
Request/Response identities are implemented. Context activation now snapshots
one bounded exact-session plan before native I/O, commits after every action,
compensates definite failure in reverse, and preserves a cancelled enable for
an exact idempotent disable cleanup edge. Explicit `BrowserContext.newPage`
now prepares and, when needed, enables a provisional adapter before publishing
Page/Frame runtime, topology, or routes. Its non-owning registration handle
does not block the driver cursor; a racing provisional-session event is held
until commit. Publication failure reverses Network ownership and retires the
lane before exact-target cleanup. Automatic new-Page discovery now joins the
same transaction and detaches the auto-attached session after rollback. Session
replacement preserves old logical interest, enables one replacement-only shadow
adapter before publication, disposes old active Request roots in the same
runtime batch, and atomically commits the new exact session. The adapter table
is capped by the network Page-lane limit plus one shadow;
Context masks have an independently configurable limit; an in-flight fan-out
may retain at most one bounded Page/session/action plan. Public Context
listener helpers now target both Page and BrowserContext portable identities.
The owned driver turn performs network-batch publication and exact Page
acknowledgment; one unacknowledged normalized batch blocks the next driver turn,
replacement, and retirement. Parent retirement has atomic success and
failed-publication preservation proofs. A selected network driver must now be
consumed by the application installer; its registration handle joins topology
independent of installer take order. The all-capabilities recipe owns the
negotiated network queue, publisher, outbound, and one retained supervisor
backend turn, and terminal connection notification drops the driver and clears
its exact Page lanes. Unit teardown and dedicated-Chrome
install/drop/external-browser-survival proofs pass. The first network workload
also proves warmup lane reuse, three settled Request/Response lifecycles, zero
native correlation at every steady sample, and Page/Context close adapter
retirement. Its corrected terminal-snapshot contract additionally proves a
stable `[4, 4, 4]` live-object series on the persistent Page. The bounded
three-run series driver now repeats the complete attachment/runtime workload,
requires cleanup before admitting each report, and exposes diagnostic RSS-delta
ranges; cross-host repetition and reviewed budgets remain.
Route preparation continues to reject any active
Context Page whose exact lane was not prepared by a positive participant;
silent event loss is not an interim behavior.

### DT-035: Transport and codec boundaries

- Question: Which semantics belong to the protocol versus in-process, pipe,
  WebSocket, or future transports?
- Deliverable: transport and codec contracts, framing, binary handling, limits,
  authentication boundary, and connection negotiation.
- Done when: adding a transport cannot change Page or Locator behavior.

Implementation checkpoint (2026-09-03): write-half closure is now a separate
opt-in `ShutdownFrameTransport` capability rather than an assumption hidden in
`FrameTransport`. Its contract requires serialization with frame sends,
completion and flushing of any transport-owned partial frame, clean peer EOF
after preceding frames, and cancellation-safe retry. The Tokio adapter and its
production wrapper implement it; `Arc<T>` preserves the capability without a
runtime-specific wrapper.

The unique encoded-client receive driver exposes `close_transport`. It first
closes bounded logical state and wakes/releases pending calls, then enters the
same priority send gate used by commands and cancellation controls before
shutting down the write half. Commands already queued behind that barrier
recheck logical closure after acquiring the gate and never write after EOF.
Cancellation of the close future leaves the channel logically closed and a
later owner call can retry shutdown. The server supervisor exposes both a
plain transport close and a combined control/backend-cleanup plus transport
close. The combined error retains both failures when cleanup and shutdown fail.

Tests prove that shutdown waits behind an active client writer, prevents a
queued command from writing, completes a transport-owned partial Tokio frame
before EOF, preserves the server's prior terminal reason, drains a result before
server EOF, and reports dual close failures.

The second-transport checkpoint adds `pwright-transport-memory`, a
standard-library-only full-duplex `FrameTransport` with an explicit queue bound
per direction. Send and shutdown mutate state only when their caller-owned
future is polled; unpolled sends have no side effect. Overflow, local write
closure, peer receiver loss, and poisoned state are distinct errors. Each
direction retains at most the declared number of already-bounded `WireFrame`
values plus one replacement receive waker. Shutdown drains preceding frames
before EOF, endpoint drop releases queued frames, and no task or executor is
created. The same typed client/channel and typed server handler conformance test
that crosses the transport boundary now passes without a socket or Tokio
transport dependency.

The graceful-close checkpoint upgrades `pwright.control` to schema 1.1 while
retaining the checked-in 1.0 baseline. It reuses the unchanged handshake's
bounded named-version negotiation with a reserved `pwright.control` entry:
absence selects the legacy 1.0 baseline, while only an explicitly returned 1.1
entry unlocks Close/Ack. This avoids a bootstrap-format flag day. A
role-bound `NegotiatedControlCodec` receives that exact selected minor and
independently keeps the outer application version on every WireFrame. It gates
both encoding and decoding by minor and direction, so a 1.0 peer is never sent
tag 5/6 and invalid Client-Ack or Server-Close traffic fails locally.

The new core-only `pwright-connection-control` crate contains allocation-free,
caller-driven client and server machines. Side effects are explicit actions:
Client Close -> Ack wait -> write shutdown; Server Close -> reasoned cleanup ->
Ack -> write shutdown. Client wire keeps its machine on the unique driver,
closes pending logical calls before submitting Close, validates and discards
pre-barrier frames one at a time, and exposes ignored/woken counts. The server
decorator composes over an arbitrary existing control handler and guarantees
cleanup before Ack. Tests cover schema compatibility, independent negotiation,
version and role rejection, exact outer/application version preservation,
legacy failure before local close, a complete memory-transport exchange, and
cancelling/resuming Ack wait without a duplicate Close.

The endpoint checkpoint completes the non-Unix authentication foundation with
two distinct adapters: Tailnet-only mutual PSK for an already encrypted
Tailnet, and general mTLS with caller-owned PKI plus bounded certificate pins.
Neither changes Page/Locator semantics or the generic transport contract.

The production-composition checkpoint installs the reserved support range only
when Chromium remote control is selected. The handshake limit must therefore
hold every installed browser capability plus this one protocol-family entry;
an undersized profile fails during preparation. Accepted connections construct
server-role negotiated codecs for the inbound runtime receiver, independent
outbound lifecycle lane, and composable graceful-close decorator. The control
profile declares a second simultaneous encode peak for CloseAck while an
outbound send is retained. After the ordered Close barrier is observed, the
supervisor now terminates immediately with `PeerGracefulClose`, drops bounded
active command work, and does not wait to reinterpret the exchange as EOF.

## Phase 4: design composition

### DT-040: Capability definition

- Question: What is the smallest independently installable capability, and what
  may it contribute?
- Deliverable: capability anatomy covering public API, protocol messages,
  runtime handlers, backend support, dependencies, tests, and diagnostics.
- Done when: navigation, DOM, input, network, tracing, and browser launching can
  be classified without circular ownership.
- Checkpoint: DOM serialization is assigned to a new read-only
  `pwright.dom_content` capability rather than screenshot or arbitrary
  evaluation. The granularity decision follows independently useful authority,
  backend machinery, and resource cost—not one capability per method and not a
  broad Page grab bag. Its 1.0 schema, generated manifest/server binding,
  attachment codec, typed Page/Frame client facades, Chromium utility-world
  adapter, shared locator/DOM Frame route, direct dispatch, negotiated remote
  handler, all-capabilities recipe, and dedicated-Chrome proof are implemented.

### DT-041: Composition and dependency rules

- Question: Are capabilities composed with Cargo features, generics, a runtime
  registry, generated manifests, process plugins, or a deliberate combination?
- Deliverable: alternatives analysis including compile time, binary size,
  diagnostics, versioning, and downstream ergonomics.
- Done when: missing dependencies, duplicate ownership, incompatible versions,
  and cycles all fail predictably.
- Checkpoint: runtime subset selection now has a matching compile-time proof.
  `pwright-composition-chromium-navigation` depends on the CDP backend with
  default features disabled and only its navigation surface selected, while
  preserving the complete aggregate composition. It binds an injected CDP
  client and pre-provisioned Page to production-budgeted generated handlers;
  no task, executor, transport, browser provider, launcher, installer, Node.js,
  or unrelated capability is selected. An isolated consumer plus a CI Cargo-
  tree allowlist fails closed on any new Pwright dependency. Runtime
  negotiation and Cargo selection are therefore complementary rather than
  substitutes. Other capability leaves and duplicate/incompatible selection
  diagnostics remain open.
- Checkpoint: value evaluation is the second independently selectable Chromium
  server slice. Its adapter was extracted into
  `pwright-backend-cdp-evaluation`; the aggregate backend reexports it without
  becoming a dependency of `pwright-composition-chromium-evaluation`. The leaf
  binds one Frame, distinguishes default and explicit unique execution
  contexts, retains the bounded context-route lease, and installs only the
  generated evaluation handler. Its exact consumer graph excludes navigation,
  Handles, the aggregate composition/backend, clients, providers, installers,
  runtimes, Node.js, and browser acquisition.
- Checkpoint: read-only DOM content is the third independently selectable
  Chromium server slice. `pwright-backend-cdp-dom-content` owns only the fixed
  Frame serializer and exact utility-world route; the aggregate backend
  compatibility-reexports it. The one-Frame composition installs only the
  generated `Frame.content` handler and jointly budgets its persistent route,
  backend result, decoded protocol result, and wire peak. Its exact graph
  excludes public evaluation, screenshot, navigation, aggregate
  backend/composition, clients, providers, installers, and runtime choice.
  Evaluation and DOM content share only the command-free
  `pwright-backend-cdp-common` failure classifier, so shared mechanism grants
  no command authority.
- Checkpoint: Page screenshot is the fourth independently selectable Chromium
  server slice. `pwright-backend-cdp-content` owns the native capture and
  bounded base64 decode, while the aggregate backend compatibility-reexports
  it and Locator screenshot explicitly reuses its opaque admission
  transaction. The one-Page composition validates the exact Page before CDP,
  installs only `Page.screenshot`, and jointly budgets backend result, decoded
  protocol result, and binary wire attachment. Its graph excludes DOM content,
  public evaluation, Locator screenshot, navigation, aggregate
  backend/composition, clients, providers, installers, and runtime choice.
- Checkpoint: Locator screenshot is the fifth independently selectable
  Chromium server slice. Its capture/translation backend accepts a
  command-free `LocatorScreenshotGeometryResolver` plugin. The first-party
  Chromium resolver is now an independent bounded leaf and shares one extracted
  selector compiler/runtime with the complete Locator backend, so the exact
  23-package consumer graph excludes public Locator commands and the aggregate
  CDP backend. The generic capture composition still accepts alternate
  resolvers without linking the first-party implementation.
- Checkpoint: the public `pwright` umbrella now exists with an empty default
  feature set. Its explicit axes separate portable evaluation/navigation
  contracts, Playwright-shaped clients, raw CDP, their minimal Chromium server
  leaves, executor-neutral runtime/transport contracts, and Tokio adapters. It
  only reexports selected crates and contributes no hidden policy or
  implementation. Eleven isolated downstream packages prove that the empty
  profile has no Pwright dependency, content/Locator-screenshot/DOM/evaluation/navigation clients
  have no CDP/server, Chromium content has no client/DOM/evaluation/Locator/
  navigation/aggregate backend, Chromium DOM content has no client/evaluation/
  screenshot/navigation/aggregate backend, Chromium evaluation has no client/
  navigation/aggregate backend, and Chromium navigation has no client/provider/
  installer/aggregate-backend dependency.
  The Chromium Locator-screenshot profile additionally rejects both
  `pwright-cap-locator` and the aggregate CDP backend.
  A default-build compile-fail doctest also proves an omitted capability has no
  public module. A `full` feature is deliberately postponed until the complete
  supported topology is known.

### DT-042: Core versus policy layers

- Question: Which behaviors are invariant automation semantics, and which are
  optional retry, tracing, recording, leasing, rate-limit, or timeout policy?
- Deliverable: responsibility table and composition order rules.
- Done when: optional policy cannot silently alter protocol or object-lifecycle
  invariants.
- Checkpoint: Playwright tracing source was decomposed into a base
  BrowserContext metadata/group/chunk capability and independently selected
  DOM snapshot, ARIA snapshot, screencast, network body, HAR, source, console,
  video, filesystem/ZIP, and Artifact contributors. The base recorder owns no
  I/O or executor, and an injected sink must declare bounded admission. The
  state and resource contract is recorded in
  `knowledge/pwright-tracing.md`. The core-only bounded recorder and its exact
  production profile are implemented in `pwright-trace-model`; a detached
  chunk gates any next chunk/session until release. `pwright-cap-tracing`
  freezes the six BrowserContext-only 1.0 commands and exposes only discard or
  an injected sink, not Artifact/ZIP or contributor flags. The bounded binary
  codec, Playwright-shaped opt-in client facade, and generated six-service
  server chain are implemented and transfer no trace-body attachment. The
  task-free `pwright-tracing-service` explicitly composes one pre-registered
  bounded recorder per Context with an injected monotonic clock and sink, uses
  the same instance for all six generated services, and requires Context/all
  sink cleanup after releasing base recorder state. All six commands now pass
  a full bounded typed-client/server in-memory duplex proof, and server
  manifests omit `pwright.tracing` when its handlers are absent. The optional
  `pwright-plugin-tracing-navigation` leaf wraps `navigation.goto` only and
  contributes correlated before/after/error entries through an injected
  Page-to-Context route. A dedicated-Chrome test proves one real CDP `goto`
  and external-browser survival. A DT-003 run repeats one warmup plus three
  measured chunks, returns `TraceBytes.current` to zero after every sink
  handoff, and passes logical/browser-process teardown checks. The optional
  `pwright-trace-sink-file` leaf now turns one detached chunk into a versioned
  `.pwtrace` file without adding a task, runtime, browser, downloader, archive,
  or network dependency. It requires a fresh caller-granted directory,
  preallocates bounded per-Context accounting, checks exact encoded file and
  aggregate disk quotas before path creation, streams to a create-new temporary
  file, syncs, and atomically renames. Six AAA tests cover the exact bytes
  through `TracingService`, rejection before file creation, independent
  Context/chunk/aggregate limits, and refusal to adopt an existing directory.
  The separate `pwright-trace-sink-pull` leaf is the task-free boundary before
  remote streaming. Its fallibly preallocated FIFO rejects on global chunk,
  per-Context chunk, per-chunk byte, or aggregate-byte pressure. A queued chunk
  keeps its original `TraceBytes` leases and recorder gate; `pop` releases queue
  admission but transfers that ownership intact to the caller. Context/all
  abort drops only chunks the sink still owns. Six AAA tests cover service
  handoff, transfer-without-release, all admission axes, and selective cleanup.
  Rich trace contributors, a concrete remote transport adapter, repeated
  series, and reviewed budgets remain.
  Source inspection then corrected the remote-transfer direction: Playwright
  returns an Artifact and lets the client pull a child Stream with `read(size)`;
  it does not push trace chunks unsolicited. Pwright records that contract in
  `knowledge/pwright-artifacts-and-streams.md`. The first prerequisite is now
  implemented without expanding tracing 1.0: `ArtifactBytes` is an independent
  ledger class, Artifact/Stream have stable object vocabulary, and the
  command-free `pwright-artifact-model` owns one bounded source shared by
  independent Stream cursors, with explicit EOF and a two-phase prepared read.
  Commit advances one cursor, drop rolls back, a second prepared read on that
  Stream fails, and close releases that Stream's source owner. Seven AAA tests
  cover cancellation, overlap accounting, bounds, close, empty EOF, shared
  source accounting, and independent cursors.
  The next checkpoint freezes `pwright.artifact` 1.0 with generated
  `Artifact.open_stream/failure/delete` and `Stream.read/close` contracts on
  exact object kinds. Its independent codec enforces metadata, request-size,
  attachment-count/index/length, result-byte, and failure-string bounds, copies
  successful read bytes into a separate `ArtifactBytes` ledger, and preserves
  explicit empty EOF. Six codec tests and two generated-server tests prove all
  typed shapes plus capability presence/absence. Typed Artifact/Stream handles
  now reuse the shared bounded client object table, while the independent
  client facade adds all five Playwright-shaped methods and proves remote
  Create-announcement and direct capacity checks. The task-free service
  preallocates bounded Artifact/Stream tables, injects Stream IDs, separately
  limits source/read/failure/retained-result bytes, cleans subtrees child-first,
  invalidates prepared reads on teardown, and completes one client-to-registry
  binary duplex pull. The separate `pwright-artifact-runtime` layer now couples
  registry preparations to controlled-runtime Create/Dispose transactions,
  verifies child-first topology, fences exact lifecycle control before command
  success, and compensates a Stream after failed Create publication. Five AAA
  tests cover normal ordered wire control, runtime-capacity pre-admission,
  transport-close rollback, and failed-fence cleanup. Its memory prerequisite
  is now implemented separately: `pwright-trace-format` reserves exact
  `ArtifactBytes` before allocation and returns that same owned PWRTRACE buffer;
  Artifact model/registry/runtime adopt it without copying or acquiring another
  lease, while the file sink reuses the same format writer. The fifth runtime
  test proves `TraceBytes` reaches zero after handoff while the exact
  `ArtifactBytes` lease survives until delete.
  The next checkpoint freezes the independent `pwright.tracing_artifact` 1.0
  capability with one BrowserContext command and explicit dependencies on base
  tracing and Artifact transfer. Generated capability, schema, binary codec,
  server handler, and opt-in client facade do not modify either dependency's
  1.0 surface. The leaf producer detaches one chunk, encodes directly into the
  exact final `ArtifactBytes` allocation, registers it transactionally, and
  fences the exact Artifact Create before success. Artifact and Stream IDs
  share one source. Failed encoding releases `TraceBytes` and reopens the
  recorder; failed control publication compensates runtime and registry state.
  Four producer AAA tests plus client, codec, and server-manifest tests cover
  successful PWRTRACE readback, exact handoff accounting, a complete typed
  client/binary/generated-server path, announcement claiming, bounded
  rejection, failed-fence cleanup, dependency completeness, and handler
  presence/absence. Rich contributors, ZIP compatibility, repeated series, and
  reviewed budgets remain.

### DT-043: Crate and feature topology

- Question: After boundaries are understood, how should they map to crates and
  Cargo features?
- Deliverable: dependency graph and representative consumer manifests.
- Done when: minimal profiles contain only their approved dependencies and the
  graph has no cycles. Crate names are deliberately postponed until this task.
- Checkpoint: the first representative minimal manifest is executable at
  `examples/v2-navigation-only`. Its only direct dependency is
  `pwright-composition-chromium-navigation`; the leaf selects the dedicated
  `pwright-backend-cdp-navigation`, the command-free shared failure classifier,
  the navigation capability/codec/generated server, and common CDP/core/wire
  foundations. `scripts/check-v2-navigation-dependencies.sh` allowlists its
  exact 15-package normal Pwright closure and runs in CI, so Locator,
  screenshots, network events, tracing, Artifacts, providers, managed browser
  installation, the aggregate backend/composition, and v1 crates are forbidden.
  Shared-infrastructure granularity and remaining capability leaves remain
  open; see
  `knowledge/pwright-dependency-topology.md`.
- Checkpoint: the shared Chromium topology is independently installable as
  `pwright-backend-cdp-topology`. The leaf owns only bounded
  Browser/Context/Page/Frame/World state, generated-CDP-event normalization,
  lifecycle effects, reconnect reconciliation, and resource-accounted
  rollback transactions; it issues no command and starts no task. The
  aggregate backend compatibility-reexports its unchanged public surface.
  `examples/v2-topology-only` and
  `scripts/check-v2-topology-dependencies.sh` enforce an exact five-package
  normal Pwright closure: consumer, topology leaf, browser model, CDP model,
  and core. Capability, codec, client, server, runtime, provider, launcher,
  installer, full composition, aggregate backend, and v1 crates are forbidden.
  The leaf's 36 AAA tests remain attached to their implementation owner.
- Checkpoint: Context/Page creation and lifecycle are four independent CDP
  backend leaves. Context creation has an exact 6-package closure; Context
  lifecycle has 8 because it consumes topology ownership; Page lifecycle has
  6; and Page creation has 8 because exact-target compensation explicitly
  depends on Page lifecycle. Each has a standalone consumer and CI allowlist.
  The leaves preserve all 17 existing AAA tests, issue only their documented
  Target/Page calls, bound every retained native ID and close-confirmation
  loop, and import neither the aggregate nor unrelated capabilities. The
  aggregate compatibility-reexports the same API. Composition remains
  responsible for atomic topology/runtime/route publication around these
  native operations. Dedicated Chrome 152 tests pass for explicit Page close
  and the complete Context-create/Page-create/Page-close/Context-close path;
  both prove that the externally owned browser remains alive.
- Checkpoint: Handle lifecycle is an independent six-package CDP backend leaf.
  It retains one bounded native object ID and issues exactly one
  `Runtime.releaseObject`, with no hidden retry or evaluation dependency.
  Retained Handle creation, Handle-local value/Handle evaluation, `jsonValue`,
  `getProperty`, and bounded `getProperties` form one ten-package
  `pwright-backend-cdp-handle-evaluation` leaf because they share private
  invocation, parsing, and cleanup transactions. That leaf depends on the
  existing by-value evaluation backend; no reverse edge exists. Two standalone
  consumers and exact CI allowlists exclude navigation, Locator,
  content/screenshot, network events, topology, Context/Page creation and
  lifecycle, the aggregate backend, codecs, client/server, runtime, provider,
  installer, Node.js, and browser acquisition. The aggregate compatibility-
  reexports both APIs, and all 11 AAA tests remain with their implementation
  owners. A dedicated Chrome 152 test passes for real element/primitive Handle
  creation, Handle-valued invocation, explicit native release, and preservation
  of the externally owned browser.
- Checkpoint: Handle lifecycle is also an independently selectable 15-package
  Chromium server composition. The lifecycle-owned route registry counts both
  live and in-flight producer reservations, pre-admits maximum retained native
  ID/preview bytes, shrinks to exact publication bytes, and carries an opaque
  typed payload so evaluation plugins do not become lifecycle dependencies.
  Generated `Handle.dispose` performs one injected native release, then retires
  runtime and route ownership. Backend failure preserves both owners; primitive
  Handles perform no native I/O; a dropped started release transfers to a
  caller-owned bounded continuation queue; and missing continuation capacity
  rejects before native work. Multi-Handle creation now reserves the complete
  batch and publishes its routes under one registry lock around the runtime
  batch commit; injected commit failure leaves both sides unpublished and
  releases every reservation. Nine release AAA tests and a dedicated Chrome
  152 proof pass. Exact standalone and umbrella allowlists exclude evaluation,
  topology, other capabilities, aggregates, clients/providers, Tokio, Node.js,
  and browser acquisition. The next slice is the Handle-evaluation composition
  that consumes this registry and lifecycle installation.
- Checkpoint: Page and network event normalization are independent CDP backend
  leaves with exact seven-package closures. Page events explicitly depends on
  the command-free topology state needed to identify current Page/main-Frame
  routes and maps `PageReady`/`PopupLinked`; network events owns a bounded
  Page/session request correlation lane and has no topology dependency. Neither
  leaf owns a receiver, queue, task, runtime, transport, response body, browser
  session, or process. Standalone consumers and CI allowlists forbid unrelated
  commands, codecs, clients, servers, providers, installers, aggregate backend,
  Node.js, and browser acquisition. The aggregate compatibility-reexports both
  APIs, and all 10 AAA normalization/resource tests stay with their owners.
  Three dedicated Chrome 152 tests pass for caller-owned Page-load delivery,
  bounded request/response/finish without body reads, redirect plus HTTP-error
  semantics, transport failure, cleanup, and external-browser survival.
- Checkpoint: the complete Chromium Page-event server surface is also an
  independently selectable 15-package composition leaf. It installs three
  separately bounded `RejectNewest` queues for `PageLoad`,
  `BrowserContextPage`, and `PagePopup`, one independent caller-polled CDP
  cursor for load normalization, and the ordered topology creation fence. Its
  profile retains each lane's exact capacity and decoded-byte bound and rejects
  zero capacity, under-budgeted discovery payloads, incomplete aggregate
  selections, and mixed wire profiles. The leaf owns no task, executor,
  topology, transport reader, browser, process, Node.js, or browser
  acquisition. The complete recipe now consumes this leaf, the aggregate only
  compatibility-reexports it, and the umbrella exposes separate client and
  Chromium Page-event features. Three consecutive Chrome 152 runs pass for
  overlapping navigation/Locator cursors, PageLoad delivery, complete drain,
  graceful close, and preservation of the external browser. The next event
  composition boundary is the larger ordered network lifecycle surface.
- Checkpoint: the cross-event network delivery and causal lifecycle turn are
  now an independently selectable 16-package composition leaf. One queue and one sequence source
  preserve `request -> response -> requestfinished/requestfailed` ordering
  across all four public event tags. Its exact profile rejects zero capacity,
  outbound frames that cannot hold the largest terminal snapshot, and decoded
  retention below the codec's derived maximum. Aggregate conversion also
  rejects mismatched normalization/codec limits. Its generic publisher and
  retained backend own the Create fence and one receive -> publish -> exact-Page
  acknowledgement turn through injected batch-driver and object-safe dynamic
  subscription-selection ports. The leaf owns no CDP receiver, listener
  mutation, topology registry, executor task, browser, process, Node.js, or
  browser acquisition. AAA proofs cover routed decoding, Page-before-Context
  fan-out, post-publication acknowledgement, and terminal retained-future
  release. The complete recipe consumes the new backend; old aggregate types
  remain compatibility APIs. Separate `client-network-events` and
  `chromium-network-events` umbrella profiles keep the isolated matrix at 30.
- Checkpoint: Page input is an independent five-package CDP backend foundation.
  `pwright-input-model` owns only the shared 16-key chord bound, while
  `pwright-backend-cdp-input` keeps the necessarily coupled Playwright US-key
  resolver and serialized keyboard/mouse transaction state together. It owns
  no selector, Locator command, capability, runtime, transport, task, browser
  session, or process. The aggregate compatibility-reexports the previous
  coordinator, transaction, and modifier types; the full Locator adapter has a
  one-way dependency on the input leaf and the input leaf has no reverse
  Locator edge. `examples/v2-input-only` plus an exact CI allowlist reject any
  sixth Pwright package. Fifteen AAA tests cover bounded parsing, key mapping,
  serialization, dispatch, rollback, and reverse-order cleanup, while two
  dedicated Chrome 152 tests exercise typed Locator input and explicit
  cancellation without terminating the external browser.
- Checkpoint: the complete CDP Locator implementation is now the independent
  `pwright-backend-cdp-locator` leaf. It composes only the portable Locator
  contract, command-free selector runtime, Page-input foundation, CDP, core,
  and their value models; it owns no screenshot capability, aggregate backend,
  runtime, transport, server, provider, browser launcher, or process. The
  aggregate preserves its existing Locator type reexports, native-CDP access,
  and a narrow screenshot-geometry compatibility adapter. The standalone
  `examples/v2-locator-only` consumer and exact nine-package CI allowlist make
  that boundary executable, while all 96 Locator AAA tests remain beside the
  implementation owner.
- Checkpoint: `pwright-composition-chromium-locator` now binds one explicit
  Page or Frame identity to the independent backend and one generated Locator
  protocol table. Its typed profile selects exactly Locator 1.22 or 2.5, shares
  a caller-supplied Page input coordinator, and reserves unique execution-world
  routes only for Frame installations. The 17-package standalone dependency
  proof excludes the aggregate backend/composition, screenshots, navigation,
  public evaluation, runtime, provider, launcher, installer, and browser
  acquisition. Three AAA tests prove v1 routing, v2 unique-context routing,
  shared input, resource release, complete handler construction, and rejection
  before CDP for an unbound object. A dedicated Chrome 152 integration test
  covers the new v2 composition directly.
- Checkpoint: `pwright-composition-chromium-context-lifecycle` is the first
  independently selectable ownership-changing server leaf. It stages an exact
  topology close, child-first runtime disposal, and bounded caller-owned
  continuation before native `Target.disposeBrowserContext`. One injected
  retirement plugin prepares any Page-route, Handle, network, or composite
  cleanup without linking those capability crates, and returns an infallible
  post-close commit token. Its exact 17-package consumer graph rejects the
  aggregate backend/composition, clients, providers, launchers, installers,
  Tokio, Node.js, and browser acquisition. Five AAA tests prove full commit,
  rollback, unowned rejection, plugin ordering, handler construction, and
  cancellation-safe continuation; a dedicated Chrome 152 test closes one
  temporary owned Context and preserves the external browser.
- Checkpoint: `pwright-composition-chromium-context-creation` independently
  installs the Playwright-shaped creation/lifecycle pair instead of exposing a
  creation-only half. One pre-CDP publication plugin reserves optional state;
  runtime and topology publication follow native creation, and any later
  failure disposes the exact native Context through the lifecycle backend. Two
  caller-owned continuation queues separately bound creation and close. Six
  AAA tests prove paired create/close, backend rollback, exact compensation,
  plugin-capacity rejection before CDP, started-future transfer, and missing
  continuation rejection. Its standalone exact 22-package graph excludes the
  aggregate backend/composition, clients, providers, launchers, installers,
  Tokio, Node.js, and browser acquisition.
- Checkpoint: `pwright-composition-chromium-page-lifecycle` independently
  installs ordinary Page close plus before-unload initiation. Ordinary close
  pre-admits topology, child-first runtime disposal, an injected retirement
  plugin, and caller-owned continuation capacity before native I/O, then waits
  for bounded exact-target absence before committing. Before-unload resolves
  only the exact Page session and preserves local liveness. Seven AAA tests
  cover commit, rollback, plugin rejection, session routing, cancellation,
  capacity, and invalid setup; a dedicated Chrome 152 test confirms Page-only
  closure while preserving its Context and the external browser. Its exact
  17-package closure rejects aggregates, clients, providers, launchers,
  installers, Tokio, Node.js, and browser acquisition.
- Checkpoint: `pwright-composition-chromium-page-creation` pairs
  `BrowserContext.newPage` with the required Page-lifecycle leaf. It reserves
  optional publication state before CDP, binds the exact native route
  provisionally, publishes Page and main Frame as one topology/runtime unit,
  and compensates the exact target on every post-create failure. Creation and
  close have independent bounded continuations. Six AAA tests and one Chrome
  152 test cover the paired path, two-stage plugins, capacity rollback,
  cancellation transfer, exact compensation, and external-browser survival.
  Its exact 22-package graph excludes aggregates, clients, providers,
  launchers, installers, Tokio, Node.js, and browser acquisition.
- Checkpoint: `examples/v2-umbrella-profiles` adds twenty-one representative
  downstream profiles over the new `pwright` package: no features,
  `client-context-creation`, `client-context-lifecycle`, `client-page-lifecycle`,
  `client-page-creation`,
  `client-locator`, `client-content`,
  `client-locator-screenshot`,
  `client-dom-content`, `client-evaluation`, `client-navigation`,
  `chromium-context-creation`, `chromium-context-lifecycle`,
  `chromium-page-lifecycle`, `chromium-page-creation`, `chromium-locator`, `chromium-content`,
  `chromium-locator-screenshot`,
  `chromium-dom-content`,
  `chromium-evaluation`, and `chromium-navigation`. A second CI verifier compares
  each normal Cargo graph against an exact allowlist and separately checks
  feature edges, including complete rejection of the aggregate CDP backend
  from every minimal Chromium profile. The umbrella feature graph is therefore
  concrete and executable,
  not only a proposed crate list. Remaining capability families must earn
  equivalent leaf and consumer proofs before being added.
- Checkpoint: `examples/v2-evaluation-only` is the second standalone minimal
  manifest. Its sole direct dependency is
  `pwright-composition-chromium-evaluation`, whose normal 15-package Pwright
  closure uses the extracted evaluation backend rather than the aggregate CDP
  backend. `scripts/check-v2-evaluation-dependencies.sh` allowlists that whole
  graph in CI, making absence of navigation, Handle, provider, installer,
  runtime, full composition, and v1 crates executable evidence.
- Checkpoint: `examples/v2-dom-content-only` is the third standalone minimal
  manifest. Its sole direct dependency is
  `pwright-composition-chromium-dom-content`; its exact normal 15-package
  Pwright closure includes only the command-free shared CDP classifier plus
  DOM-content capability, codec, generated server, adapter, composition, and
  common CDP/core/wire foundations. Its CI allowlist rejects public evaluation,
  screenshot, navigation, aggregate backend/composition, clients, provider,
  installer, runtime, and every v1 crate.
- Checkpoint: `examples/v2-content-only` is the fourth standalone minimal
  manifest. Its sole direct dependency is
  `pwright-composition-chromium-content`; its exact normal 17-package Pwright
  closure includes separately reusable screenshot value and attachment-codec
  leaves but excludes DOM content, public evaluation, Locator screenshot,
  navigation, aggregate backend/composition, clients, providers, installers,
  runtimes, and every v1 crate. CI rejects any future graph expansion.
- Checkpoint: `examples/v2-locator-screenshot-only` is the fifth standalone
  minimal manifest. Its exact 23-package closure combines the independent
  capture backend with the command-free selector runtime and first-party
  selector-geometry plugin, selector/screenshot models, reusable codecs, and
  the generated one-command server. CI still forbids `pwright-cap-locator`,
  the aggregate CDP backend/composition, clients, providers, installers,
  runtimes, Node.js, and browser acquisition.

## Phase 5: design browser integration

### DT-050: Browser provider and ownership

- Question: How are system browsers, explicit executables, remote endpoints,
  and optional managed downloads discovered and owned?
- Deliverable: provider contract and process-lease lifecycle.
- Done when: Pwright can never terminate a browser it does not own, and browser
  download is impossible without explicit installation and invocation.
- Checkpoint: `pwright-composition-chromium-launch` is an optional leaf above
  the attach-only root. It accepts only an explicit launch request, uses a
  production connector whose connection retains plan and ledger identity, and
  returns a composition paired with one consuming process lease. Binding
  rejects foreign plans and foreign ledgers. External attachment has no owned
  shutdown method by compile-time proof. A live installed-Chrome test proves an
  unpolled shutdown remains lazy, a polled shutdown reaps the exact launched
  process/profile, and the unrelated external Chrome remains live. The leaf has
  no installer/downloader dependency and leaves managed installation explicit.
- Checkpoint: an additionally selected `pwright.browser_lifecycle` leaf reserves
  its own codec/wire resources and augments only the owned launch handshake;
  the attach-only manifest remains unchanged. Missing handshake configuration
  fails before process spawn. Only a real accepted session that negotiated the
  capability can consume the process owner into a non-cloneable close grant;
  acceptance must also come from a production supervisor sharing the launch's
  exact connection resource ledger. An unbound handshake and an equally
  configured foreign connection are rejected; rejection returns the exact
  owner. The remote service validates the Browser target, prepares complete
  runtime disposal, consumes shutdown authority once, transfers started
  cancellation into caller-reserved continuation capacity, and publishes
  descendant-first Dispose through Browser Dispose before Result. A real
  Chrome proof exercises unprivileged, unbound, foreign-session, and exact-owner
  authorization paths before the one valid remote close reaps the exact
  process. Portable success remains `()` across capability, codec, client,
  server, and wire. An optional composition-only report sink receives the typed
  provider report after runtime disposal; the default sink retains nothing.
  Encoded and live-Chrome proofs show the sink receives provider diagnostics
  while no PID, exit status, shutdown method, or profile path enters the remote
  result.

### DT-051: Backend abstraction

- Question: Where is the boundary between Playwright semantics and Chromium
  CDP, future WebDriver BiDi, Firefox, or WebKit implementations?
- Deliverable: backend responsibilities and escape-hatch policy.
- Done when: CDP identifiers and domain messages cannot leak into portable API
  contracts unintentionally.

### DT-052: Browser context, target, and session mapping

- Question: How do public BrowserContext/Page/Frame objects map to backend
  contexts, targets, sessions, execution worlds, and process events?
- Deliverable: Chromium mapping and recovery state machines.
- Done when: creation races, target crashes, detach, popup, iframe, reconnect,
  and exact-target cleanup have defined behavior.

## Phase 6: operational design

### DT-060: Memory ownership and limits

- Question: What state exists, who owns it, how large may it grow, and when is
  it released?
- Deliverable: allocation inventory, limit table, eviction policy, and memory
  metrics schema.
- Done when: object tables, event queues, bodies, logs, traces, screenshots,
  selectors, handles, and pending calls all have explicit bounds.
- Checkpoint: `pwright.content` 1.0 now proves screenshot admission without an
  unbounded queue. The Chromium adapter reserves its configured maximum before
  base64 decode, rejects an oversized encoded response before allocation,
  shrinks the lease to exact retained bytes, and releases Page/global admission
  on completion or dropped future. The remote frame owns a separate
  `WireBytes` copy and the client result owns a separate `ScreenshotBytes`
  lease, so source, transport, and consumer lifetimes are not conflated.
- Checkpoint: Frame DOM serialization uses a dedicated `DomContentBytes` lease
  retained by a non-cloneable UTF-8 value, one active read per Frame, bounded
  connection concurrency, and one UTF-8 wire attachment. Chromium locator and
  DOM content share one utility execution-route record, and an unnegotiated
  remote DOM-content capability installs no handler and reaches no backend.
- Checkpoint: accepted-capability gating now covers every remote application
  handler and typed event lane, not only DOM content and locator. The gate runs
  before codec construction, service wrapping, queue creation, or production
  resource claims. Remote event profiles also retain their stable capability
  owner and are rejected during preparation when that owner is not installed.
  A control-only handshake proves all ten current application surfaces remain
  absent and the complete resource metrics snapshot is unchanged.

### DT-061: Runtime and concurrency model

- Question: Which runtime services and tasks exist, and what ordering is
  guaranteed per browser, context, page, session, and call?
- Deliverable: task/actor model, scheduler rules, fairness, and shutdown flow.
- Done when: the design has no task per API object, no unbounded spawn path, and
  no hidden global runtime dependency.

### DT-062: Observability and diagnostics

- Question: How can users explain memory, latency, retries, hangs, object leaks,
  browser failures, and missing capabilities?
- Deliverable: stable metrics, structured events, diagnostic snapshots, and
  opt-in tracing rules.
- Done when: diagnostics are useful without retaining full payloads or stacks
  by default.

### DT-063: Security and trust boundaries

- Question: What changes when attaching to local, tailnet, or untrusted remote
  browsers and runtimes?
- Deliverable: threat model for endpoints, credentials, browser data, file
  access, downloads, evaluation, plugins, and remote transport.
- Done when: authentication, authorization, origin, secret redaction, and unsafe
  escape hatches have explicit owners.

## Phase 7: prove and evolve the design

### DT-070: Testing and conformance strategy

- Question: How will behavior be verified without coupling tests to one backend
  or relying primarily on mocks?
- Deliverable: test pyramid covering generated protocol tests, deterministic
  fakes, backend contract tests, real-browser tests, Playwright comparison
  tests, failure injection, and memory soak tests.
- Done when: every compatibility and resource claim maps to an automated test.
- Checkpoint: screenshot tests cover schema drift, command metadata/options,
  invalid combinations, direct Page routing, typed CDP parameters, exact base64
  padding bounds, oversize rejection, dropped in-flight cleanup, generated
  remote handler transfer as one binary attachment, and client-side retained
  result accounting. A dedicated-profile live test verifies real Chromium
  full-page PNG dimensions, lease release, isolated Context cleanup, and
  preservation of the externally owned browser. JPEG/WebP and pixel-fidelity
  conformance remain open.
- Checkpoint: the command-free `pwright-screenshot-model` now owns format,
  scale, clip, options, and retained screenshot bytes while
  `pwright-cap-content` reexports those exact types. The first Locator
  screenshot design is a separate capability and one cancellation-bounded
  server command, not an addition to Locator/Page capture or a droppable
  two-command temporary-handle sequence. The semantic selector model has now
  moved into zero-dependency `pwright-selector-model`, and its recursive wire
  closure is published as `pwright.selector` 1.0. The independent
  `pwright.locator_screenshot` 1.0 schema, frozen baseline, portable contract,
  bounded codec, typed client extension, and generated server binding are now
  implemented. The codec rejects Page-only clip/full-page options before
  encoding and retains decoded bytes under `ScreenshotBytes`. The command-free
  `pwright-codec-screenshot` helper now gives Page and Locator capture the same
  attachment index/count/length/limit checks and exact result-copy accounting.
  The complete Chromium profile owns its full-Locator resolver's selector and
  Frame-route budgets plus capture, codec, and wire budgets without advertising
  ordinary Locator or Page screenshot. The independently installable capture
  leaf instead accepts a command-free geometry resolver plugin and excludes
  `pwright-cap-locator`; applications combine that plugin's declared resources
  with the capture profile. Backend and composition tests cover strictness,
  post-scroll geometry, future-drop cleanup, local routing, and negotiated
  binary transfer. A dedicated-Chrome proof verifies exact visible and
  below-fold PNG dimensions, Context/target cleanup, zero retained screenshot
  bytes, and continued external-browser reachability. Child-Frame capture now
  follows Playwright's native owner translation: each ancestor is resolved by
  `DOM.getFrameOwner` on its parent session and contributes its native border
  origin. Shared linked translation nodes remain bounded by the Frame topology.
  Unit proofs cover local and nested OOPIF routing, including exact child,
  parent, and Page session ownership; a dedicated-Chrome same-process iframe
  proof verifies an exact 70x50 PNG and cleanup. Cross-site OOPIF live and
  decoded pixel fidelity remain open; see
  `docs/knowledge/pwright-locator-screenshot.md`.

### DT-071: Versioning and compatibility

- Question: How do API, capability, protocol, codec, and backend versions
  evolve independently?
- Deliverable: compatibility and negotiation rules plus deprecation policy.
- Done when: old/new local and remote combinations have a defined result and
  capabilities can evolve without forcing an unrelated full-stack upgrade.
- Checkpoint: locator 1.21 and 2.4 now prove independent generated artifacts,
  a frozen 2.0 baseline, exact-major negotiation, exact selected-minor codec
  gating, one selected per-connection handler tree, v2-on production routing,
  and v2-off rollback isolation. Cross-capability release and deprecation
  policy remains broader DT-071 work.
- Checkpoint: locator 1.22 and 2.5 append ScrollIntoViewIfNeeded behind exact
  old-minor rejection while preserving byte-identical v1/v2 payloads. The
  selected-major server route reaches a two-phase Chromium utility-world wait
  and synchronous commit, waits only for stable geometry, skips repositioning
  at intersection ratio one, and retains no DOM handle, input transaction, or
  new resource class.

### DT-072: Migration and release strategy

- Question: How can v2 be validated without destabilizing current Pwright?
- Deliverable: isolation, comparison, adoption, rollback, and release plan.
- Done when: implementation can begin with a bounded vertical slice, current
  users retain a supported path, and no legacy code must be rewritten in place.
- Checkpoint: the new `pwright` umbrella is additive beside every v1 package and
  changes no legacy default. All optional dependencies carry matching path and
  `0.1.0` version requirements, and Cargo successfully normalizes/lists the
  package. Registry upload verification correctly remains unavailable until
  those currently unpublished v2 leaf crates are released; the required order
  is foundational leaves, composition leaves, then umbrella. Repository users
  can select the verified Git features before that release without Node.js or
  browser acquisition.
- Checkpoint: adding the evaluation contract/client/server feature trio remains
  additive and no-default-features. The complete backend continues to compile
  and pass its existing test suite through a compatibility reexport, while
  downstream consumers can select the smaller evaluation backend and
  composition directly.
- Checkpoint: the DOM-content contract/client/server feature trio follows the
  same additive path. The aggregate backend compatibility-reexports the
  extracted adapter and preserves its prior DOM tests, while isolated consumers
  select the smaller backend and composition directly. No v1 default, API, or
  package was replaced.
- Checkpoint: Page screenshot adds the same no-default-features contract,
  client, and Chromium-server trio. The aggregate backend continues to expose
  the moved service, including its existing Locator-screenshot reuse, while an
  isolated consumer can select the smaller backend and composition. No v1
  default, API, or package was replaced.
- Checkpoint: navigation now follows the same extraction rule. The dedicated
  `pwright-backend-cdp-navigation` owns its two-feature descriptor, bounded
  resource profile, event wait state, and injected-CDP services. The aggregate
  backend preserves its public constructors, cancellation, and native-access
  behavior through a thin compatibility wrapper, while minimal consumers never
  link the aggregate crate. A direct dedicated-Chrome test proves the leaf can
  navigate a real target without acquiring or closing the external browser.

## Ordering and gates

```text
Evidence
  -> product boundary and measurable success
  -> public behavioral contract
  -> client/runtime and protocol semantics
  -> capability composition
  -> backend mapping
  -> operational limits and security
  -> conformance, versioning, and migration
  -> implementation proposal
```

- Gate A: DT-001 through DT-013 must finish before approving an architecture.
- Gate B: DT-020 through DT-035 must finish before approving protocol or API
  implementation.
- Gate C: DT-040 through DT-063 must finish before approving crate topology or
  a runtime vertical slice.
- Gate D: DT-070 through DT-072 must finish before implementation is treated as
  production-directed rather than exploratory.

## Decision record template

Every completed design task should produce a short decision record containing:

- Context and evidence
- Requirements and non-goals
- Alternatives considered
- Decision and rationale
- Consequences and risks
- Resource and compatibility impact
- Validation plan
- Unresolved questions
