<!-- agent-updated: 2026-09-05T21:28:33Z -->

# Chromium topology-event driver

> Status: implemented and verified for optional browser-root discovery,
> default Context, existing/new Pages, popup ordering, complete initial frame
> trees, OOPIF sessions, opt-in utility-world creation and locator routing,
> world replacement, detach, crash, sticky lag, and same-connection
> authoritative reconstruction.

## Decision

Chromium topology ingestion is optional infrastructure in
`pwright-composition-chromium`, not a portable capability and not a hidden
runtime task. Selecting `ChromiumTopologyEventDriverProductionProfile` adds its
fixed memory declaration to the production plan. After attachment,
`topology_event_driver()` claims the attachment's single event subscription;
a second claim is rejected instead of manufacturing two competing event
baselines. The owner explicitly calls `receive_once(runtime)` whenever its
scheduler permits one event to advance.

This keeps executor, cancellation, fairness, shutdown, and retry policy outside
the browser adapter. A composition that does not select topology events has no
driver, subscription table, or retained driver budget.

Browser-root discovery is a second explicit selection:
`ChromiumPageDiscoveryProductionProfile`. It requires topology events but adds
no portable capability and no task. Without it, the driver can maintain Pages
that another component explicitly created or admitted without claiming every
browser target.

## Browser-root discovery

On its first caller-owned receive, selected discovery performs:

1. browser-root `Target.setAutoAttach(autoAttach=true,
   waitForDebuggerOnStart=true, flatten=true)`;
2. `Target.getTargetInfo` as the Chromium ordering barrier used by Playwright;
3. bounded `Target.getBrowserContexts` enumeration; and
4. transactional creation of one portable default BrowserContext.

The event subscription exists before these calls, so attachments for targets
that predate the driver remain queued. Chrome 152 demonstrated that default
Pages can carry an opaque `browserContextId`; absence is not a safe default
test. The driver therefore records the bounded set returned by
`Target.getBrowserContexts` (contexts created through CDP). A Page whose exact
ID is already registered uses that context. An unregistered ID in the
non-default set is detached. The one remaining stable ID, or a missing ID, maps
to the unique default Context. A later unfamiliar ID triggers a fresh bounded
enumeration before it can be classified. Unsupported root targets are detached
so an auto-attached worker cannot remain paused.

The topology represents the default Context with an internal empty native key,
not a fabricated CDP ID. Ownership is `Default`, close remains forbidden, and
at most one such Context may be registered.

## Session initialization

Before every receive, the driver snapshots the topology's bounded target-local
session routes. Each newly observed Page or OOPIF session receives:

1. `Page.enable`;
2. complete `Page.getFrameTree` acquisition;
3. `Runtime.enable`;
4. when `ChromiumUtilityWorldProfile` is selected, one empty
   `Page.addScriptToEvaluateOnNewDocument` registration with a Page-unique
   world name followed by `Page.createIsolatedWorld` for every current frame;
5. recursive `Target.setAutoAttach(autoAttach=true, flatten=true)`; and
6. when discovery owns paused targets,
   `Runtime.runIfWaitingForDebugger`.

Discovery uses `waitForDebuggerOnStart=true`; maintenance-only use keeps it
false. Existing local descendants from the returned frame tree are normalized
parent-first. Page root identity reuses the reserved main-Frame ObjectId, while
OOPIF roots preserve their already-established Frame identity. Duplicate IDs,
an invalid root parent, and contradictory tree parents fail explicitly.
The utility-world name is `__pwright_utility_` plus the fixed-width hexadecimal
portable Page ID. A creation failure triggers one fresh frame-tree read. It is
tolerated only if that exact frame is now absent, after which worlds are
created for the refreshed tree; a still-present frame or any retry failure
preserves the CDP error. The profile starts no task and retains no name map.

The same synchronization runs after applying an event. For a discovered Page,
the driver uses the event's session to finish every CDP initialization command
first, then admits the Page, complete frame tree, and ready transition in one
topology/runtime/route transaction. Thus an initialization or tree-validation
failure creates no internal or externally visible Page object and emits no
`PageReady`. Explicitly admitted Page and OOPIF routes use the before/after
synchronization pass and are initialized before `receive_once` returns. A later
caller retry repeats idempotent setup where a route still exists. Detached or
rebound sessions are removed from the retained table.

All updates committed before, during, and after one received event are combined
in order into its single return value. Callers therefore cannot lose
`PageReady` or `PopupLinked` merely because those effects arose during session
synchronization rather than direct event decoding.

Recursive auto-attach is required because Chromium reports cross-process child
Frames on their own target-local sessions. It does not imply browser discovery,
browser launch, or ownership of Chrome.

## Event normalization

Generated CDP event types are the decode boundary. The driver recognizes:

| CDP event | Topology transition |
| --- | --- |
| `Page.frameAttached` | attach or rebind a local child Frame under the reporting Page |
| `Page.frameDetached` | apply exact `remove` or `swap` semantics |
| root `Page.frameNavigated` | rebind the stable main-Frame object to its new native ID |
| `Runtime.executionContextCreated` | attach only a main or named Pwright utility world |
| `Runtime.executionContextDestroyed` | remove the exact session/context pair |
| `Runtime.executionContextsCleared` | remove every world owned by that exact session |
| iframe `Target.attachedToTarget` | attach or rebind an OOPIF Frame to its child session |
| browser-root Page `Target.attachedToTarget` | admit an existing/new Page into its exact or default Context |
| `Target.detachedFromTarget` | retire a Page session or put an OOPIF into detach synchronization |
| Page `Target.targetCrashed` | publish the typed Page crash effect |

Unknown event methods and events from already-retired sessions are ignored.
Malformed recognized events fail explicitly. A child-Frame navigation event is
not treated as a second attachment source: local attachment and OOPIF target
events own host transitions, while execution-context events own document-world
replacement.

Main-world and utility-world routes are enumerated separately. Portable
`Frame.evaluate` binds only the main world. When the utility profile is
selected, locator service publication waits for each Frame's matching utility
event, then selector installation and actionability evaluation use that
Frame's exact target-local session and `uniqueContextId`. Page calls delegate
to the main Frame route; local child Frames and OOPIF Frames keep independent
routes. Page-authored isolated worlds with any other name are ignored.

## Atomicity and races

Every mutation uses `transact_topology_update`. It stages a complete production
topology shadow, derives Page plus Frame evaluation and locator routes from that
candidate, applies one runtime lifecycle batch, and only then publishes
topology and routes. A failure before runtime admission leaves all live views
unchanged. A destroyed selected world removes its locator route rather than
falling back to another document; its replacement receives a fresh
selector-runtime installation cell.

Local-to-OOPIF swaps preserve the portable Frame `ObjectId`. The new main world
replaces the previous same-kind route even though it belongs to a different
session. A late destruction event for the old session/context pair therefore
cannot remove the new route. OOPIF target detach removes its worlds and requests
Frame-detach synchronization; the later Page `frameDetached(remove)` owns final
portable disposal.

The driver is bound to one `ConnectionEpoch`. Once topology begins reconnecting
under another epoch, the old driver rejects work before consuming another
event. A new connection must construct a new driver and perform explicit state
resynchronization.

Cross-WebSocket rebinding is deliberately forbidden. The local Playwright
source establishes native disconnect as terminal: Chromium
`CRConnection._onClose` disposes the root protocol session,
`CRBrowser._didDisconnect` closes all Pages and workers, the shared Browser
base closes Contexts and emits `disconnected`, and `BrowserDispatcher` closes
then disposes the public channel object. Pwright retains each `CdpSession` on
its exact concrete `CdpConnection` and follows the same rule.

Because topology-events are optional, closure detection cannot live only in
the topology driver. `CdpConnection::closed()` returns an owned future over its
shutdown token without retaining the connection. The attached composition can
create `ChromiumCdpLivenessBackend`, a task-free supervisor backend lane that
turns closure into a sticky terminal error. `ConcurrentServerBackendChain`
combines it fairly with the optional topology backend and forwards the final
termination reason to both, which drops any retained topology turn.
`AttachedChromiumComposition::server_backend` is the canonical constructor, so
native liveness is present even when the supplied plugin is `NoServerBackend`.
A new CDP attachment gets a new Browser graph and new object identities.

For event loss on a still-live CDP transport, the same driver exposes
`resynchronize_after_event_loss(runtime, new_epoch)`. It is valid only after a
reported lag and requires browser-root discovery. The operation disables
auto-attach, waits for Chromium to detach the old sessions, replaces the
broadcast subscription, begins a strictly newer topology epoch, and re-enables
auto-attach. Chromium then republishes all existing targets. The browser-root
`Target.getTargetInfo` response is the snapshot barrier: every earlier
attachment is drained and initialized before `finish_reconnect` disposes
Contexts, Pages, and non-main Frames absent from the snapshot. Surviving native
target identities retain their portable ObjectIds while receiving new CDP
sessions. Main and utility worlds are rebuilt from new Runtime events.

The reconstruction remains caller-driven and returns one ordered update from
`ReconnectStarted` through the replayed `PageReady`/`PopupLinked` effects to
`ReconnectFinished`. A failed reconstruction remains in the sticky resync
state and can only be retried with another newer epoch; ordinary receive cannot
silently resume from a partial snapshot.

## Backpressure and memory

The underlying CDP connection owns a bounded broadcast ring and exact event
item/byte leases. Its production profile includes one extra transient event:
Tokio allocates the incoming event before a full ring evicts its oldest slot.
Broadcast lag is never interpreted as "skip and continue": `receive_once`
returns `ResyncRequired { skipped }` without applying the next visible event,
and repeats that error without consuming anything until explicit
reconstruction succeeds. A missing detach or world-clear event can therefore
never be silently skipped.

The driver profile is derived from the exact connection and topology profiles
and is rejected if those profiles drift. Its one fixed `BackendStateBytes`
lease covers:

- the initialized-session table;
- bounded non-default Context identities and one observed default native ID;
- one complete Page/OOPIF session-route snapshot;
- normalized frame attachments and duplicate-detection identities;
- one temporary target-local client during initialization; and
- conservative generated-event and frame-tree-response decoding overlap based
  on the connection's maximum event and response bytes.

Session count, native-ID bytes, event bytes, resource scope, and profile
identity are all validated before state is retained.

## Complete application recipe

`ChromiumAllCapabilitiesApplicationInstaller` now chooses and statically chains
every currently implemented command, activated control, typed portable event,
topology, PageLoad, and native-liveness plugin after the consuming
same-supervisor handshake. It requires every component to have been selected
and negotiated rather than degrading to a smaller recipe. PageLoad has an
independent broadcast cursor so it cannot steal native events from topology
ingestion, and both backends remain caller-polled without spawning.

The Page-event portion is no longer constructed inside the aggregate. The
independent `ChromiumPageEventsInstallation` accepts the caller's cursor,
adapter, shared topology, transport, exact profile, and production ledger, then
returns the topology publisher, caller-polled PageLoad backend, and three
separately bounded portable outbounds. Its ordered creation fence remains the
only edge between discovery mutation and publication. The aggregate converts
to this profile only when all three Page-event lanes are selected with the same
wire profile, and compatibility-reexports the leaf types without becoming an
owner of their implementation. Release tests cover exact retained-byte bounds,
lane independence, incomplete selections, and mixed-wire rejection; the
complete recipe's four-Page Chrome 152 test passed three consecutive runs after
the migration.

## Explicit remaining work

- Firefox/WebKit or WebDriver BiDi event adapters; and
- memory/latency soak measurements under navigation and iframe churn.

These remain separate design and capability work. The event driver reconstructs
the selected Page topology on its current live Chromium connection; it does not
yet claim transparent transport reconnection.

## Portable discovery-event bridge

`pwright.page-events` version 1.2 now defines fixed-size portable
`BrowserContextPage` and `PagePopup` events. The former maps `PageReady` to its
Context source; the latter maps `PopupLinked` to its opener source. Both carry
the discovered Page `ObjectId`, preserving the stable topology identity rather
than leaking a CDP target or session ID.

The composition root can install either bounded typed sender independently and
pass it to `ChromiumTopologyPageEventPublisher`. No sender means no event queue.
RejectNewest and DropOldest loss are both terminal publication results that
require the caller to close or reconstruct its event view. This is deliberately
not hidden inside the topology driver: topology mutation, capability
installation, codec choice, transport I/O, and executor ownership remain
separate components.

For remote delivery, composition rejects these discovery-event profiles unless
ordered lifecycle control and Page discovery are present. The connection owner
uses `publish_after_creation_fence` to scan the same update, send ordered
control through the final required initializer, and only then enqueue the
events. Existing objects reconstructed without a new `Create` bypass that
fence. Invoking the bridge and selecting connection shutdown/resync behavior
remain explicit composition choices.

`ChromiumTopologyConnectionPump` now combines one driver turn, the ordered
creation fence, publication, and loss policy without spawning. Same-connection
reconstruction has a nonzero maximum-attempt bound and advances via checked
epochs. It is available only for an inert portable publisher. When discovery
callbacks are installed, CDP loss requires a new client connection so replay
cannot duplicate callbacks whose prior delivery is unknowable. Publication
loss and a dropped polled pump future are likewise terminal. The pump can then
terminate the server supervisor under `BackendEventFailed`, which performs
reasoned control cleanup and transport shutdown.

`ChromiumTopologyServerBackend` retains one pending pump turn and implements
the supervisor's optional backend lane. The same caller-owned supervisor now
rotates its unique remote transport reader, bounded command futures, serial
outbound delivery, and this backend lane. A ready lane advances the starting
priority to the next lane, preventing a target-event flood from starving
commands or lifecycle output. Terminal backend failure uses
`BackendEventFailed`; the complete application-run method then shuts down the
write half and reports both the primary and shutdown failures if necessary.
No `select` outside the supervisor can accidentally cancel its reader or the
topology turn. EOF, failure, or dropping the complete run invokes the backend
termination hook and releases the retained turn; no task is spawned.

## Evidence

Release-profile fake tests prove default Context uniqueness, complete initial
frame-tree publication, existing/new Page admission, popup ready/link ordering,
foreign-context rejection, failed-initialization gating, local Frame/world publication, recursive OOPIF
session initialization, local↔OOPIF identity preservation, harmless late world
destruction, two-stage OOPIF removal, sticky broadcast-lag failure,
authoritative Context/Page/Frame reconciliation, portable-ID preservation,
post-recovery event consumption, and stale-epoch rejection. Strict Clippy
passes with warnings denied.

A dedicated Chrome 152 test uses a `serve-lib` cross-site fixture and the
production `AttachedChromiumComposition`. No test code parses or manually
applies the child target/world events. Repeated caller-owned `receive_once`
calls discover the OOPIF, initialize its exact session, publish its main-world
`uniqueContextId`, and successfully evaluate `location.hostname` through the
portable child `Frame` service. Cleanup removes the temporary context while the
externally owned Chrome remains alive.

Current Chrome reports the root of an OOPIF session's `Page.getFrameTree` with
the embedding Frame as `parentId`. Validation accepts this only when the route
is a non-Page target session; a Page session root with any parent remains a
typed failure. Positive OOPIF and negative Page-root AAA tests freeze that
distinction, and the cross-site Chrome proof exercises the real parent-bearing
shape.

The utility-world Chrome 152 proof additionally creates a same-process iframe
whose `#inside` button does not exist in the main document. It waits for the
child utility route and completes a trial click through the child Frame's
portable object ID. Unit proofs cover shared Page/main-Frame routing, distinct
local-frame and OOPIF sessions, fail-closed world removal, and fresh runtime
installation after world replacement.

A second repeated Chrome 152 test selects browser-root discovery, creates a
default-context target before auto-attach, and performs no manual topology
setup. It proves the opaque default-context ID classification, existing Page
initialization, caller-owned concurrent popup unpause, ordered popup linkage,
and then produces 1,100 Runtime events against a 1,024-slot ring. The resulting
lag is recovered twice by real auto-attach re-publication: opener and popup keep
their ObjectIds, receive new sessions, clean up exactly, and leave the external
browser alive.

The fresh-remote-connection Chrome 152 checks use the reusable attachment
factory and a production-ledger-aware transport builder. One transition
installs a new runtime containing only its Browser root while the supervisor
remains handshake-first. A second selected topology profile is claimed by the
remote seed, and the attached composition rejects another driver rather than
creating a competing event baseline. A third check proves a runtime-installer
plugin that returns without constructing the graph cannot enter the final
connection state. Four post-handshake checks then prove that acceptance comes
from the stored supervisor, a negotiated navigation handler can be built from
the accepted context, an unconsumed selected topology driver rejects assembly,
the transferred driver remains the unique baseline, mandatory CDP liveness
wraps even a no-op backend, and EOF shuts down exactly the transport stored in
that connection once.
