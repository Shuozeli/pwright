<!-- agent-updated: 2026-09-05T22:05:06Z -->

# Pwright v2 dependency topology

## Decision

Runtime capability selection is not enough for the v2 product boundary. A
consumer that installs only one capability must also avoid compiling unrelated
capability, backend, client, provider, tracing, Artifact, and browser-install
crates.

The enforced profiles include eight attach-only Chromium CDP server
compositions plus independently installable backend capability and topology
foundations:

```text
consumer
  -> pwright-backend-cdp-topology
       -> pwright-browser-model + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-context-creation
       -> pwright-backend-cdp-common
       -> pwright-cap-context-creation + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-context-lifecycle
       -> pwright-backend-cdp-common + pwright-backend-cdp-topology
       -> pwright-cap-context-lifecycle + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-context-lifecycle
       -> pwright-backend-cdp-context-lifecycle + pwright-backend-cdp-topology
       -> pwright-cap-context-lifecycle + pwright-codec-context-lifecycle
       -> pwright-runtime + pwright-server-context-lifecycle
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-context-creation
       -> pwright-backend-cdp-context-creation
       -> pwright-composition-chromium-context-lifecycle
       -> pwright-cap-context-creation + pwright-codec-context-creation
       -> pwright-runtime + pwright-server-context-creation
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-page-lifecycle
       -> pwright-backend-cdp-common
       -> pwright-cap-page-lifecycle + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-page-creation
       -> pwright-backend-cdp-common + pwright-backend-cdp-page-lifecycle
       -> pwright-cap-page-creation + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-handle-lifecycle
       -> pwright-backend-cdp-common
       -> pwright-cap-handle-lifecycle + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-handle-lifecycle
       -> pwright-backend-cdp-handle-lifecycle
       -> pwright-cap-handle-lifecycle + pwright-codec-handle-lifecycle
       -> pwright-runtime + pwright-server-handle-lifecycle
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-handle-evaluation
       -> pwright-backend-cdp-evaluation + pwright-backend-cdp-common
       -> pwright-cap-handle-evaluation + pwright-cap-evaluation
       -> pwright-browser-model + pwright-cdp + pwright-core

consumer
  -> pwright-backend-cdp-page-events
       -> pwright-backend-cdp-topology
       -> pwright-cap-page-events + pwright-browser-model
       -> pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-page-events
       -> pwright-backend-cdp-page-events + pwright-backend-cdp-topology
       -> pwright-cap-page-events + pwright-codec-page-events
       -> pwright-connection-control + pwright-runtime-control
       -> pwright-runtime + pwright-server-events + pwright-server-wire
       -> pwright-browser-model + pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-network-events
       -> pwright-cap-network-events + pwright-network-model
       -> pwright-browser-model + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-network-events
       -> pwright-backend-cdp-network-events
       -> pwright-cap-network-events + pwright-codec-network-events
       -> pwright-network-model + pwright-runtime-control + pwright-runtime
       -> pwright-server-events + pwright-server-wire
       -> pwright-browser-model + pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-input
       -> pwright-input-model + pwright-cdp + pwright-core

consumer
  -> pwright-composition-chromium-navigation
       -> pwright-backend-cdp-navigation
          -> pwright-backend-cdp-common
       -> pwright-cap-navigation
       -> pwright-codec-navigation
       -> pwright-server-navigation
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-evaluation
       -> pwright-backend-cdp-evaluation
          -> pwright-backend-cdp-common
       -> pwright-cap-evaluation
       -> pwright-codec-evaluation
       -> pwright-server-evaluation
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-dom-content
       -> pwright-backend-cdp-dom-content
          -> pwright-backend-cdp-common
       -> pwright-cap-dom-content
       -> pwright-codec-dom-content
       -> pwright-server-dom-content
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-composition-chromium-content
       -> pwright-backend-cdp-content
          -> pwright-backend-cdp-common
       -> pwright-cap-content + pwright-screenshot-model
       -> pwright-codec-content + pwright-codec-screenshot
       -> pwright-server-content
       -> pwright-cdp + pwright-core + pwright-transport

consumer
  -> pwright-backend-cdp-selector-geometry
       -> pwright-backend-cdp-selector-runtime
       -> pwright-backend-cdp-locator-screenshot geometry contract
       -> pwright-cdp + pwright-core + pwright-selector-model
  -> pwright-composition-chromium-locator-screenshot
       -> pwright-backend-cdp-locator-screenshot
          -> pwright-backend-cdp-content + pwright-backend-cdp-common
       -> pwright-cap-locator-screenshot + selector/screenshot models
       -> pwright-codec-locator-screenshot + shared selector/screenshot codecs
       -> pwright-server-locator-screenshot
       -> pwright-cdp + pwright-core + pwright-transport
```

The navigation leaf accepts an existing `Arc<dyn CdpClient>` and one pre-provisioned Page
`ObjectId`. It creates the bounded CDP navigation service, a production-budgeted
wire protocol, and the generated two-command server handler chain. It starts no
task and does not select a runtime or transport. Browser discovery, browser
launch, managed installation, Node.js, Locator, tracing, and Artifact transfer
are absent.

The evaluation leaf binds one pre-provisioned Frame. A main Frame uses its
target-local default execution context; a child Frame can bind an explicit
CDP `uniqueContextId` whose retained route is included in the production
resource claim. It exposes only by-value evaluation: no navigation, Handle,
client, provider, installer, runtime, or browser acquisition crate is selected.

The DOM-content leaf also binds exactly one pre-provisioned Frame, but requires
that Frame's bounded utility-world CDP `uniqueContextId`. It evaluates only the
library-owned doctype-plus-`outerHTML` serializer and exposes only
`Frame.content`; public evaluation, screenshot, navigation, client, provider,
installer, runtime, and browser acquisition crates are absent. The retained
utility route is part of the production resource claim.

The content leaf binds exactly one pre-provisioned Page and exposes only
`Page.screenshot`. It supports the already-frozen non-mutating 1.0 option set,
returns one budgeted binary attachment, and validates the exact Page identity
before CDP. DOM access, public evaluation, Locator screenshot, navigation,
client, provider, installer, runtime, and browser acquisition crates are
absent.

The Locator-screenshot leaf binds one Page/Frame object to an injected
selector-to-geometry resolver and a target-local CDP capture client. The
resolver returns only validated viewport geometry; no DOM/CDP handle crosses
it. The first-party Chromium plugin is a separate dependency from the generic
capture composition and shares its command-free compiler/runtime with the full
Locator backend. Consequently the executable exact 23-package profile contains
neither `pwright-cap-locator` nor `pwright-backend-cdp`. Selector plugins
declare and claim their resources separately, and applications combine those
requirements concurrently with the capture profile before constructing the
shared ledger.

`pwright-backend-cdp-common` contains only command-free CDP failure
classification shared by navigation, evaluation, DOM content, and Page
screenshot. It owns no capability, service, route, resource ledger, runtime, or
provider authority.

`pwright-backend-cdp-topology` is the separately installable bounded
Browser→BrowserContext→Page→Frame/World state machine. It normalizes generated
CDP topology events, rejects stale/future epochs, emits portable lifecycle
effects, preserves popup/crash/reconnect semantics, and reserves exact
native-string state plus a transactional shadow. It performs no CDP command or
I/O and selects no capability, codec, client, server, runtime, provider,
launcher, installer, browser process, or aggregate.

The four creation/lifecycle leaves preserve Playwright-aligned native
boundaries without owning composition transactions. Context creation creates
one no-options, dispose-on-detach context and compensates an invalid returned
identity. Context lifecycle permits only owned contexts to reach
`Target.disposeBrowserContext`. Page lifecycle separates ordinary
`Target.closeTarget` plus bounded absence confirmation from
`Page.close` run-before-unload semantics. Page creation creates
`about:blank`, attaches its exact target, reads its main Frame, and delegates
all failure compensation to the Page lifecycle leaf. This explicit
page-creation → page-lifecycle edge has no reverse edge.

Handle lifecycle is a six-package leaf that retains one bounded native object
ID and issues exactly one `Runtime.releaseObject`. Retained Handle creation,
Handle-local value/Handle evaluation, `jsonValue`, `getProperty`, and bounded
`getProperties` remain together in the ten-package Handle-evaluation leaf
because they share private invocation, response parsing, and cleanup. The leaf
also contains the low-level Chromium ElementHandle-adoption primitive because
it reuses the same bounded native representation and parser. Adoption adds two
CDP methods but no package edge; public composition remains gated. The leaf
reuses the value-evaluation adapter; that dependency is one-way. Neither Handle
leaf selects navigation, Locator, content/screenshot, network events, topology,
Context/Page creation or lifecycle, the aggregate backend, codecs,
client/server, runtime, provider, installer, Node.js, or browser acquisition.

The Page-event leaf activates Page lifecycle delivery for one topology-routed
Page and normalizes only current main-Frame `load`, `PageReady`, and
`PopupLinked`. The network-event leaf maps one bounded Page/session HTTP lane
into atomic portable lifecycle/event/retirement batches, including redirects,
without retaining response bodies. Both own caller-driven normalization only:
no CDP receiver, queue, reader task, runtime, transport, browser session, or
process. Page events has the intentional topology edge; network events does
not. Their normal Pwright closures contain seven packages each.

`pwright-composition-chromium-page-events` binds that normalization to three
independently budgeted event queues, the ordered discovery fence, one
caller-polled load cursor, and an injected transport. Its exact 15-package
closure adds only the Page-event codec, generic event/server-wire machinery,
runtime control needed by the creation fence, and transport boundary. It has
no aggregate Chromium backend, command capability, client, provider, Tokio,
launcher, installer, Node.js, or browser acquisition edge. The aggregate
complete recipe consumes and compatibility-reexports this leaf.

`pwright-composition-chromium-network-events` owns the single cross-event
queue, monotonic sequence, routed codec, production resources, creation fence,
and retained receive/publish/acknowledge backend. Its generic batch-driver port
depends on the normalized CDP batch contract but not a receiver; its object-safe
scope selector lets the aggregate preserve dynamic Page/BrowserContext listener
semantics without moving topology or listener mutation into the leaf. The exact
16-package standalone closure includes runtime/control and server-wire for this
causal turn, but has no aggregate Chromium backend, client, provider, Tokio
executor, launcher, installer, Node.js, or browser acquisition. The complete
recipe consumes it, and the aggregate implements both ports while retaining its
old publisher/backend as compatibility APIs.

The Page-input leaf keeps keyboard layout resolution and the serialized
keyboard/mouse transaction state together because each requires the other's
modifier representation. Their shared 16-key chord bound is independently
owned by command-free `pwright-input-model`, so the portable Locator contract
and CDP backend agree without either depending on the other. The leaf accepts a
caller-owned CDP client only when beginning a transaction and owns no selector,
Locator command, capability, runtime, transport, task, browser session, or
process. Its normal Pwright closure contains exactly five packages.

The complete Locator backend is the independently installable
`pwright-backend-cdp-locator` leaf. It composes the portable Locator contract,
command-free selector runtime, and Page-input foundation over a caller-owned
target CDP client. Screenshot capture remains a separate capability: the
compatibility aggregate alone adapts the Locator leaf's bounded internal
geometry result to the screenshot resolver contract. The Locator leaf owns no
screenshot, aggregate backend, runtime, transport, server, provider, launcher,
browser session, or process. Its normal Pwright closure contains exactly nine
packages.

`pwright-composition-chromium-locator` adds only the generated Locator server
and bounded wire codec around that backend. Each installation retains exactly
one protocol table—Locator 1.22 or 2.5—and one explicit Page or Frame route.
The caller injects the target CDP client and Page-owned input coordinator;
unique-context Frame installations additionally hold the declared route lease.
It owns no runtime, transport task, screenshot/navigation/evaluation
capability, provider, launcher, browser session, or process. Its standalone
normal Pwright closure contains exactly seventeen packages.

`pwright-composition-chromium-context-lifecycle` closes the gap between the
native Context-lifecycle backend and a usable generated server leaf. It stages
the exact owned-context topology transition, child-first runtime disposal, and
one caller-owned continuation slot before issuing
`Target.disposeBrowserContext`. A single injected retirement plugin can
prepare Page-route, Handle, network, or composite cleanup without adding those
crates to the leaf; its commit token is infallible and publishes only after the
native close and core ownership commits. The standalone normal Pwright closure
contains exactly seventeen packages and excludes the aggregate backend and
composition, clients, providers, launchers, installers, Tokio, Node.js, and
browser acquisition.

`pwright-composition-chromium-context-creation` pairs the generated
`Browser.newContext` handler with that required lifecycle installation. It
allocates portable identity, pre-admits an injected publication plugin,
prepares runtime ownership, creates the native Context, and commits the exact
topology/runtime/plugin publication. Any post-CDP publication failure disposes
the returned native identity through the lifecycle backend. Creation and close
have separate caller-owned continuation capacity. Its exact 22-package normal
closure contains no aggregate backend/composition, client, provider, launcher,
installer, Tokio, Node.js, or browser acquisition.

`pwright-composition-chromium-page-lifecycle` is the parallel Page-only leaf.
It composes the generated two-command server, bounded binary codec, native
close confirmation, production topology, and controlled runtime in exactly
seventeen Pwright packages. A target-session resolver supplies only the
Page-local client needed by before-unload, and one retirement plugin owns any
optional Page-route, Handle, or network cleanup. Its exact dependency proof
excludes the aggregate backend/composition, clients, providers, launchers,
installers, Tokio, Node.js, and browser acquisition.

`pwright-composition-chromium-page-creation` explicitly depends on that leaf
and exposes the generated creation and lifecycle handlers together. Its
pre-CDP publication reservation can provisionally bind the returned native
target/session/main-frame route, while drop remains rollback and final commit
is infallible. Runtime publishes Page and Frame as one batch; every later
failure closes the exact native target. The standalone normal closure contains
exactly twenty-two Pwright packages and excludes aggregates, clients,
providers, launchers, installers, Tokio, Node.js, and browser acquisition.

`pwright-composition-chromium-handle-lifecycle` owns the bounded shared route
registry consumed by optional Handle producers. A producer pre-admits one
route and its maximum retained bytes, then publishes exact ownership, an
opaque typed backend payload, and an injected object-local disposer. The leaf
therefore installs generated `Handle.dispose`, runtime retirement, and
caller-owned cancellation continuation without depending on Handle or value
evaluation. Its standalone normal closure contains exactly fifteen Pwright
packages and excludes topology, Context/Page capabilities, aggregate crates,
clients, providers, launchers, installers, Tokio, Node.js, and browser bytes.

`pwright-composition-chromium-handle-evaluation` depends on that lifecycle leaf
and adds bounded Frame execution-world routes plus all six generated 1.2
handlers. Its exact 24-package normal closure excludes aggregate crates,
topology, Context/Page commands, clients, providers, launchers, installers,
Tokio, Node.js, and browser acquisition. Lifecycle remains independently
selectable and value-only evaluation still has no reverse Handle dependency.

`pwright-plugin-ordered-handle-delivery` is the independent remote policy above
those services. Its exact 15-package closure contains only the Handle
capability/codec/server path, core runtime/control, and transport traits. It
contains no CDP/backend/provider, Chromium composition, client, launcher,
installer, Tokio adapter, Node.js, or browser bytes. Direct Handle evaluation
therefore stays a 24-package Chromium leaf, while remote consumers opt into the
ordering and Result-delivery fence explicitly.

The existing `pwright-backend-cdp` and `pwright-composition-chromium` remain
compatibility aggregates. The aggregate backend reexports the extracted
topology, Context/Page creation and lifecycle, Handle, Page/network event,
evaluation, DOM-content, and Page-screenshot adapters and wraps the extracted
navigation adapter for existing users. The composition aggregate depends
downward on the independent Handle-evaluation leaf: its complete recipe uses
the leaf registry and command services, while aggregate Page services provide
the live Frame/world resolver and parent/topology retirement bridge. Optional
ElementHandle adoption remains a separately budgeted backend choice. No
isolated leaf depends upward on either aggregate.

## Representative consumer

`examples/v2-topology-only/Cargo.toml`,
`examples/v2-context-creation-only/Cargo.toml`,
`examples/v2-context-lifecycle-only/Cargo.toml`,
`examples/v2-context-lifecycle-composition-only/Cargo.toml`,
`examples/v2-context-creation-composition-only/Cargo.toml`,
`examples/v2-page-creation-only/Cargo.toml`,
`examples/v2-page-creation-composition-only/Cargo.toml`,
`examples/v2-page-lifecycle-only/Cargo.toml`,
`examples/v2-page-lifecycle-composition-only/Cargo.toml`,
`examples/v2-handle-lifecycle-only/Cargo.toml`,
`examples/v2-handle-lifecycle-composition-only/Cargo.toml`,
`examples/v2-handle-evaluation-only/Cargo.toml`,
`examples/v2-handle-evaluation-composition-only/Cargo.toml`,
`examples/v2-ordered-handle-delivery-only/Cargo.toml`,
`examples/v2-input-only/Cargo.toml`,
`examples/v2-locator-only/Cargo.toml`,
`examples/v2-locator-composition-only/Cargo.toml`,
`examples/v2-page-events-only/Cargo.toml`,
`examples/v2-network-events-only/Cargo.toml`,
`examples/v2-navigation-only/Cargo.toml`,
`examples/v2-evaluation-only/Cargo.toml`, and
`examples/v2-dom-content-only/Cargo.toml`,
`examples/v2-content-only/Cargo.toml`, and
`examples/v2-locator-screenshot-only/Cargo.toml` are intentionally separate
workspaces so the root workspace cannot unify optional features into any
proof:

Each manifest has exactly one dependency:

```toml
# v2-topology-only
pwright-backend-cdp-topology = { path = "../../crates/pwright-backend-cdp-topology" }

# v2-context-creation-only
pwright-backend-cdp-context-creation = { path = "../../crates/pwright-backend-cdp-context-creation" }

# v2-context-lifecycle-only
pwright-backend-cdp-context-lifecycle = { path = "../../crates/pwright-backend-cdp-context-lifecycle" }

# v2-context-lifecycle-composition-only
pwright-composition-chromium-context-lifecycle = { path = "../../crates/pwright-composition-chromium-context-lifecycle" }

# v2-context-creation-composition-only
pwright-composition-chromium-context-creation = { path = "../../crates/pwright-composition-chromium-context-creation" }

# v2-page-creation-only
pwright-backend-cdp-page-creation = { path = "../../crates/pwright-backend-cdp-page-creation" }

# v2-page-lifecycle-only
pwright-backend-cdp-page-lifecycle = { path = "../../crates/pwright-backend-cdp-page-lifecycle" }

# v2-handle-lifecycle-only
pwright-backend-cdp-handle-lifecycle = { path = "../../crates/pwright-backend-cdp-handle-lifecycle" }

# v2-handle-evaluation-only
pwright-backend-cdp-handle-evaluation = { path = "../../crates/pwright-backend-cdp-handle-evaluation" }

# v2-input-only
pwright-backend-cdp-input = { path = "../../crates/pwright-backend-cdp-input" }

# v2-locator-only
pwright-backend-cdp-locator = { path = "../../crates/pwright-backend-cdp-locator" }

# v2-locator-composition-only
pwright-composition-chromium-locator = { path = "../../crates/pwright-composition-chromium-locator" }

# v2-page-events-only
pwright-backend-cdp-page-events = { path = "../../crates/pwright-backend-cdp-page-events" }

# v2-network-events-only
pwright-backend-cdp-network-events = { path = "../../crates/pwright-backend-cdp-network-events" }

# v2-navigation-only
pwright-composition-chromium-navigation = { path = "../../crates/pwright-composition-chromium-navigation" }

# v2-evaluation-only
pwright-composition-chromium-evaluation = { path = "../../crates/pwright-composition-chromium-evaluation" }

# v2-dom-content-only
pwright-composition-chromium-dom-content = { path = "../../crates/pwright-composition-chromium-dom-content" }

# v2-content-only
pwright-composition-chromium-content = { path = "../../crates/pwright-composition-chromium-content" }

# v2-locator-screenshot-only
pwright-backend-cdp-selector-geometry = { path = "../../crates/pwright-backend-cdp-selector-geometry" }
pwright-composition-chromium-locator-screenshot = { path = "../../crates/pwright-composition-chromium-locator-screenshot" }
```

Every isolated composition now links a dedicated backend directly:

```toml
pwright-backend-cdp-navigation = { workspace = true }
pwright-backend-cdp-evaluation = { workspace = true }
pwright-backend-cdp-dom-content = { workspace = true }
pwright-backend-cdp-content = { workspace = true }
pwright-backend-cdp-locator = { workspace = true }
pwright-backend-cdp-context-lifecycle = { workspace = true }
pwright-backend-cdp-context-creation = { workspace = true }
```

## Enforcement

`scripts/check-v2-topology-dependencies.sh`,
`scripts/check-v2-context-creation-dependencies.sh`,
`scripts/check-v2-context-lifecycle-dependencies.sh`,
`scripts/check-v2-context-lifecycle-composition-dependencies.sh`,
`scripts/check-v2-context-creation-composition-dependencies.sh`,
`scripts/check-v2-page-creation-dependencies.sh`,
`scripts/check-v2-page-lifecycle-dependencies.sh`,
`scripts/check-v2-handle-lifecycle-dependencies.sh`,
`scripts/check-v2-handle-evaluation-dependencies.sh`,
`scripts/check-v2-ordered-handle-delivery-dependencies.sh`,
`scripts/check-v2-input-dependencies.sh`,
`scripts/check-v2-locator-dependencies.sh`,
`scripts/check-v2-locator-composition-dependencies.sh`,
`scripts/check-v2-page-events-dependencies.sh`,
`scripts/check-v2-page-events-composition-dependencies.sh`,
`scripts/check-v2-network-events-dependencies.sh`,
`scripts/check-v2-network-events-composition-dependencies.sh`,
`scripts/check-v2-navigation-dependencies.sh`,
`scripts/check-v2-evaluation-dependencies.sh`, and
`scripts/check-v2-dom-content-dependencies.sh`,
`scripts/check-v2-content-dependencies.sh`, and
`scripts/check-v2-locator-screenshot-dependencies.sh` inspect only normal Cargo edges
from their representative consumers. Each allowlists every permitted Pwright
crate, so a future dependency fails rather than silently expanding the
distribution. CI runs all twenty-seven dependency proofs after workspace
checking. The topology
foundation contains exactly five Pwright packages. Context creation and Page
lifecycle contain 6 each; Context lifecycle and Page creation contain 8 each.
Handle lifecycle contains 6; Handle evaluation contains 10 because it reuses
the value-evaluation backend and its portable evaluation/browser contracts.
Page events and network events contain 7 each; only Page events includes the
topology foundation.
The complete Page-event composition contains 15 packages and retains no
aggregate, command, client, provider, Tokio, browser-launch, or install edge.
The Network-event lifecycle composition contains 16 packages. It intentionally
includes the normalized CDP batch contract and runtime/control creation fence,
but retains no CDP receiver, aggregate, client, provider, Tokio executor,
browser-launch, or install edge. Dynamic Page/BrowserContext subscription
selection and the single-cursor driver are injected ports; their state and
mutation remain aggregate-owned.
Page input contains 5: its consumer, input backend, input model, CDP, and core.
Locator contains 9: its consumer, Locator backend, input and selector-runtime
foundations, Locator contract, CDP, core, and the input/selector value models.
The complete Locator composition contains 17 packages and still excludes the
aggregate backend, runtime, provider, launcher, installer, and unrelated
capabilities.
The complete Context-lifecycle composition also contains 17 packages; runtime
and topology are intentional ownership dependencies, while Page routes,
Handles, network events, clients, providers, browser launch, and the aggregate
backend remain absent.
The complete paired Context-creation composition contains 22 packages. Its five
additional creation capability/codec/backend/server/composition packages reuse
the Context-lifecycle leaf as an explicit dependency instead of duplicating or
hiding close authority.
The current exact normal
Pwright closures contain 15 packages each for navigation,
evaluation, and DOM content, while Page screenshot contains 17 because its
portable binary result and attachment codec are separately reusable leaves.
Locator screenshot contains 23 because its independently installable
selector-runtime and first-party selector-geometry leaves join the separately
reusable selector/screenshot models and codecs, while public Locator commands
and the aggregate backend remain forbidden.

The topology leaf retains the 36 pure and production-state tests that formerly
lived inside the aggregate backend. They cover bounded identity maps, deferred
materialization, OOPIF swaps/detach, popup ordering, crash and reconnect
reconciliation, generated-event validation, complete portable runtime effects,
exact native-byte accounting, resource admission, and rollback/commit
transactions. Aggregate backend tests compile against its compatibility
reexports.

The 17 creation/lifecycle AAA tests stay with their new implementation owners:
4 for Context creation, 5 for Context lifecycle, 2 for Page creation, and 6 for
Page lifecycle. They cover exact CDP calls, ownership rejection before I/O,
native-ID bounds, invalid-response compensation, target-absence confirmation,
run-before-unload separation, and portable error classification.
Two dedicated Chrome 152 tests additionally pass through aggregate
compatibility reexports: one closes an explicit Page and preserves its parent
Context/browser; the other creates and closes a real Context and Page while
verifying main-Frame initialization and external-browser survival.
The independent Context-lifecycle composition has five additional AAA tests
for coordinated commit, complete rollback, unowned-context rejection, plugin
ordering, generated-handler construction, and cancellation-safe continuation.
Its dedicated Chrome 152 test closes one owned temporary Context through the
new leaf and then proves the external browser is still alive.
The independent paired Context-creation composition owns six additional AAA
tests for success, rollback, compensation, plugin admission, and both
continuation boundaries. Its dedicated Chrome proof creates and closes the
same Context through the paired services, observes native removal, and leaves
the externally owned browser running.

The two Handle composition leaves own 33 direct tests: 14 for lifecycle route
admission/disposal/retirement and 19 for retained Handle creation, local
operations, bounds, adoption, atomic cleanup, and failure classification. The
aggregate live Handle test now runs the complete leaf-backed recipe against
Chrome 152 and verifies typed lifecycle announcements, Handle-valued invocation,
native release, graceful close, and external-browser survival.

The Page/network event leaves own 10 AAA tests: 4 for lifecycle activation,
main-Frame load filtering, and topology-effect mapping; 6 for HTTP lifecycle
ordering, redirects, failure/session distinction, input bounds, teardown, and
production resource release.
Three aggregate-compatibility live tests pass against Chrome 152: Page load is
read through a caller-owned cursor; bounded HTTP request/response/finish is
observed without response-body ownership; and redirect, HTTP-error, and
transport-failure semantics remain distinct. Each test disposes only its
temporary context and verifies external Chrome remains live.

The extracted navigation backend descriptor exposes only
`navigation.begin` and `navigation.goto`, with no event features. Leaf and
aggregate-compatibility tests cover production-ledger installation, real
service delegation through a fake CDP client, cancellation, bounded event
state, error classification, wrong-Page rejection, and construction of the
generated handler chain. A dedicated Chrome 152 test calls the isolated
backend directly, completes a real data-URL navigation, disposes only its
temporary browser context, and verifies that the externally owned browser
remains available.

The extracted evaluation backend descriptor exposes only
`evaluation.evaluate-value`, with no event features. Composition tests cover
main-Frame and unique-context routing, held route resources, wrong-Frame
rejection before CDP, and generated-handler construction.

The extracted DOM-content backend descriptor exposes only `frame.content`.
Composition tests cover exact utility-context routing and serialization,
retained result and route accounting, rejection of a wrong Frame before CDP,
required route budgeting, and generated-handler construction.

The extracted Page-screenshot backend descriptor exposes only the native
`page.screenshot` feature. Composition tests cover exact Page routing, retained
source-result accounting, backend/codec limit agreement, and generated-handler
construction.

## Umbrella profiles

The public `pwright` crate is now a no-default-features re-export layer. It does
not replace independently usable leaf crates. Its first executable feature
axes are:

```text
navigation            portable command contract
evaluation            portable by-value evaluation contract
content               portable Page screenshot contract
context-lifecycle     portable BrowserContext close contract
context-creation      portable Context creation + required lifecycle contract
page-lifecycle        portable Page close/before-unload contract
page-creation         portable Page creation + required lifecycle contract
handle-lifecycle      portable JSHandle/ElementHandle disposal contract
handle-evaluation     portable retained-Handle creation and local operations
ordered-handle-delivery
                      remote Handle ordering and Result-abandonment policy
locator-screenshot    portable Locator screenshot contract
dom-content           portable read-only Frame DOM contract
client-navigation     client + navigation + Page extension traits
client-evaluation     client + evaluation + Frame/Page extension traits
client-content        client + Page screenshot extension trait
client-context-lifecycle
                      client + BrowserContext close extension trait
client-context-creation
                      client + Browser creation + required lifecycle client
client-page-lifecycle client + Page close/before-unload extension traits
client-page-creation  client + Page creation + required lifecycle client
client-handle-lifecycle
                      client + JSHandle/ElementHandle dispose extension trait
client-handle-evaluation
                      client + Handle evaluation + required lifecycle client
client-locator        client + Playwright-shaped Locator factories/actions
client-locator-screenshot
                      client + Locator screenshot extension trait
client-dom-content    client + DOM content + Frame/Page extension traits
cdp                   raw CDP client
chromium-navigation   CDP + navigation + minimal Chromium server leaf
chromium-evaluation   CDP + evaluation + minimal Chromium server leaf
chromium-content      CDP + Page screenshot + minimal Chromium server leaf
chromium-context-lifecycle
                      CDP + runtime/topology Context-close server leaf
chromium-context-creation
                      CDP + runtime/topology paired create/close server leaf
chromium-page-lifecycle
                      CDP + runtime/topology Page-close server leaf
chromium-page-creation
                      CDP + runtime/topology paired Page create/close server leaf
chromium-handle-lifecycle
                      CDP + runtime + bounded Handle-route/disposal server leaf
chromium-handle-evaluation
                      CDP + runtime + Handle 1.2 + required lifecycle server leaf
chromium-locator      CDP + Locator 1.22/2.5 + minimal Chromium server leaf
chromium-locator-screenshot
                      CDP + Locator screenshot orchestration + first-party selector plugin
chromium-selector-geometry
                      CDP + standalone first-party selector-to-geometry plugin
chromium-dom-content  CDP + DOM content + minimal Chromium server leaf
runtime-tokio         executor-neutral runtime + Tokio adapter
transport-tokio       transport contract + Tokio byte-stream adapter
tokio                 convenience selection of both Tokio adapters
```

`examples/v2-umbrella-profiles` contains thirty isolated packages: empty;
client Context/Page creation, Context/Page/Handle lifecycle, Handle evaluation,
Locator, content, Locator screenshot, DOM content, evaluation, navigation, and
Page/network events;
the backend-neutral ordered Handle-delivery plugin; plus their currently
packaged Chromium server counterparts, including Handle lifecycle and Handle
evaluation and Page/network events.
`scripts/check-v2-umbrella-dependencies.sh`
compares each package's complete normal Pwright dependency closure with an
exact allowlist and checks its feature edges. In particular:

- the empty umbrella has no Pwright dependency beyond itself;
- client Context creation/lifecycle, Locator, content, Locator screenshot, DOM content,
  evaluation, and navigation
  have no CDP, backend, codec, or server crate;
- Chromium Context lifecycle contains its intentional topology/runtime
  ownership boundaries and generated server but no Page-route, Handle,
  network, aggregate-backend, client, provider, launcher, or installer crate;
- Chromium Context creation contains the required lifecycle leaf and its
  topology/runtime boundaries but no aggregate backend, client, provider,
  launcher, installer, Tokio, Node.js, or browser acquisition;
- Chromium Page creation/lifecycle preserve the same paired dependency edge
  and omit Context, Handle, aggregate, client, provider, launcher, installer,
  Tokio, Node.js, and browser acquisition crates;
- Chromium Handle lifecycle owns runtime and its bounded route/disposal leaf,
  but contains no Handle evaluation, value evaluation, topology, Page/Context,
  aggregate, client, provider, launcher, installer, Tokio, Node.js, or browser
  acquisition crate;
- client Page events contains only its portable capability, codec, generic
  client event/wire machinery, core, and transport; Chromium Page events owns
  the exact 15-package server composition above and no aggregate, unrelated
  command, client, provider, Tokio, launcher, installer, Node.js, or browser
  acquisition crate;
- client Network events contains only its portable capability/model, codec,
  generic client event/wire machinery, core, and transport; Chromium Network
  events owns the exact 16-package lifecycle composition, including the
  normalized CDP batch contract and runtime/control fence, but no CDP receiver,
  aggregate, unrelated command, client, provider, Tokio executor, launcher,
  installer, Node.js, or browser acquisition crate;
- Chromium Locator contains its dedicated backend and generated server but no
  aggregate backend, client, runtime, provider, launcher, installer, or
  unrelated capability;
- Chromium Locator screenshot has neither public Locator commands nor the
  aggregate CDP backend; selector resolution stays an injected plugin;
- Chromium content has no client, DOM content, public evaluation, Locator,
  navigation, aggregate CDP backend, provider, launcher, or installer;
- Chromium DOM content has no client, evaluation, screenshot, navigation,
  aggregate CDP backend, provider, launcher, or installer;
- Chromium evaluation has no client, navigation, aggregate CDP backend,
  provider, launcher, or installer;
- Chromium navigation has no client, provider, launcher, installer, unrelated
  capability, or aggregate backend.

The umbrella intentionally has no `full` feature while the supported feature
surface is incomplete.

Each optional umbrella dependency has both a repository path and an exact
current-series `0.1.0` version requirement. Cargo can normalize and list the
package contents now. Upload verification cannot resolve the package until the
unpublished v2 leaf crates exist in the registry, so release order is leaves
first and umbrella last. Current users should use the Git dependency shown in
the umbrella README.

## Resource boundary

All eight minimal composition profiles combine their CDP adapter's bounded
operation-state peak with one simultaneous wire-encoding peak. Evaluation also
includes its optional retained unique-context route. DOM content includes the
persistent utility route, one potentially retained backend `DomContentBytes`
result, and one separately decoded public-protocol `DomContentBytes` result;
the codec therefore has no hidden private result ledger. Installation claims
everything from the caller's shared `ProductionResourceLedger`; an omitted or
undersized plan fails before a command can run.

Page screenshot similarly accounts the backend-owned `ScreenshotBytes` result,
one separately decoded public-protocol result, and the binary wire attachment
as simultaneously live owners. Backend and codec byte ceilings must match, and
installation rejects an outbound attachment profile that cannot carry the
declared maximum.

Locator screenshot adds the same capture/result/wire overlap but deliberately
does not absorb selector-plugin resources. The application must combine the
plugin's declared requirements concurrently with this composition profile.

Creation/lifecycle leaves retain only bounded native identities. Page close and
Page-creation compensation also cap target-absence confirmation attempts.
Their caller composition owns the larger transactional topology/runtime/route
resource plan; the native leaves do not create a second ledger or queue.
The Context-lifecycle composition claims only its wire resources directly;
the injected topology and runtime retain their own production claims, and an
optional retirement plugin must pre-admit its resources before native close.
The paired Context-creation profile combines both simultaneously retained
protocol tables. Its publication plugin must reserve fallible optional state
before native creation and expose only an infallible final commit.

## Current limitation

This checkpoint proves command-free topology, native creation/lifecycle, and
capability-level Cargo modularity, not the final minimum binary. `pwright-cdp`,
`pwright-server-wire`, and their transport dependencies remain shared
infrastructure and can be split further only after their own independently
useful boundaries are designed. Remaining capability-specific Chromium leaves
and umbrella features remain future DT-041/DT-043 work.
