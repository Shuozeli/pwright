<!-- agent-updated: 2026-09-05T18:49:06Z -->

# pwright - Project Guide

This is the authoritative reference for understanding pwright. Read this
before proposing changes.

## What is pwright?

A lightweight Rust CDP (Chrome DevTools Protocol) bridge. It connects to an
already-running Chrome instance and provides browser automation via:

- **Rust library** (`pwright-bridge`): Playwright-compatible Page/Locator API
- **gRPC server** (`pwright-server`): 21 RPCs for tabs, navigation, actions, content
- **CLI** (`pwright-cli`): snapshot-act-snapshot workflow for terminal/agent use
- **Script runner** (`pwright-script`): declarative YAML automation scripts

## Crate Map

```
pwright-cdp          Raw bounded WebSocket CDP client
                     Connection, Session, domain wrappers (DOM, Runtime, etc.);
                     cancellation-safe pending map, shared event payloads,
                     opt-in production resource profile, and an owned terminal
                     connection-closure signal for v2 composition

pwright-js           Centralized JS snippets (string constants, no runtime)

pwright-bridge       High-level browser operations
                     Page, Locator, Keyboard, Mouse, Touchscreen
                     Actions, navigation, snapshot, cookies, evaluate
                     Clock trait for DI in time-dependent code
                     TabCloser trait + ChromeHttpClient for HTTP tab lifecycle

pwright-server       gRPC server wrapping pwright-bridge
                     21 RPCs, protobuf schemas

pwright-cli          CLI wrapping pwright-bridge
                     Stateless: persists session in .pwright/state.json

pwright-script       Declarative YAML script runner (Phase 1 done)
                     Parser, validator, executor, JSONL output
                     Protobuf schema for script input/output

pwright-fake         In-memory browser fake for testing
                     DOM tree, CSS selector engine, HTML parser
                     FakeCdpClient implementing CdpClient trait

pwright-cdp-gen      CDP protocol code generator (standalone binary)
                     Generates typed Rust structs from protocol JSON spec
```

## Experimental v2 Foundation

The current v1 crates above remain supported and unchanged in role. An isolated
v2 implementation is being developed alongside them:

```text
pwright                  No-default-features v2 umbrella; explicit contract,
                         client, CDP/backend, runtime, and transport reexports;
                         no implicit capability, Node.js, or browser acquisition

pwright-core             Zero-dependency Service/Layer, protocol identity,
                         capability validation, bounded object/call/event state,
                         shared scoped resource ledger, production requirement
                         plans/claims, and RAII byte leases

pwright-input-model      Command-free shared input bounds; currently owns the
                         maximum physical keys retained by one chord

pwright-process-metrics  Optional zero-dependency bounded process-table sampler;
                         separates one Pwright runtime RSS from Chromium parent,
                         renderer, GPU, utility, other-child, and total RSS sums;
                         opt-in core-only workload recorder correlates ledger and
                         object counts across warm/steady/teardown phases

pwright-browser-model    Core-only canonical Browser/Context/Page/Frame/Handle
                         plus Request/Response/Artifact/Stream IDs, wire tags,
                         parent topology, marker types, and catalog

pwright-artifact-model   Core-only bounded ArtifactBytes source and two-phase
                         independent Stream cursors over one shared source,
                         explicit EOF/abort, and no I/O, runtime, or task

pwright-artifact-service Task-free preallocated Artifact/Stream registry and
                         five command services with count/byte limits,
                         injected IDs, and child-first teardown plans

pwright-artifact-runtime Opt-in controlled-runtime transaction and ordered-wire
                         adapter; exact Create/Dispose fences plus failed-Create
                         Stream compensation, with no selected executor

pwright-cap-artifact     Generated opt-in Artifact/Stream 1.0 contract with
                         three Artifact commands and two Stream commands

pwright-codec-artifact   Bounded binary Artifact codec; one exact read
                         attachment copied into an independent ArtifactBytes
                         result ledger, explicit empty EOF, and bounded errors

pwright-client-artifact  Opt-in Playwright-shaped Artifact/Stream facade; all
                         five typed methods, no hidden I/O or installation

pwright-server-artifact  Generated opt-in five-service Artifact handler chain;
                         capability is absent unless this chain is composed

pwright-network-model    Core-only bounded HTTP metadata and active native-ID
                         lifecycle correlation; no body, backend, queue, or task

pwright-trace-model      Core-only bounded tracing session/chunk/group state,
                         TraceBytes ownership, exact production profile, and
                         single-detached-chunk resource gate; no I/O or task

pwright-screenshot-model Core-only command-free screenshot format/scale/clip/options
                         values and non-cloneable budgeted byte result

pwright-selector-model   Core-only command-free immutable CSS/text/role/attribute/
                         test-id selector recipes and validated matcher values

pwright-cap-tracing      Generated opt-in BrowserContext tracing 1.0 contract;
                         six lifecycle/group commands, discard or injected-sink
                         stop mode, and no Artifact or contributor authority

pwright-codec-tracing    Bounded binary tracing-control codec for all six
                         commands and typed failures; no attachments or bodies

pwright-client-tracing   Opt-in Playwright-shaped BrowserContext tracing facade

pwright-server-tracing   Generated opt-in six-service tracing handler chain

pwright-cap-tracing-artifact
                         Independent `pwright.tracing_artifact` 1.0 producer
                         contract; one Context command and explicit tracing +
                         Artifact dependencies

pwright-codec-tracing-artifact
                         Bounded binary producer command/result codec

pwright-client-tracing-artifact
                         Opt-in Context method returning typed Artifact plus
                         bounded trace summary

pwright-server-tracing-artifact
                         Generated opt-in one-service producer handler

pwright-tracing-service  Task-free Context recorder registry shared by all six
                         tracing services; injected clock/sink and mandatory
                         Context/connection sink cleanup

pwright-trace-format     Shared PWRTRACE v1 size/writer; exact pre-allocation
                         ArtifactBytes admission and owned-buffer handoff

pwright-trace-sink-file  Optional bounded synchronous file sink; fresh-directory
                         ownership, exact pre-admission, create-new staging,
                         file sync, and atomic versioned `.pwtrace` publication

pwright-trace-sink-pull  Optional bounded task-free FIFO for complete trace
                         chunks; explicit backpressure, abort cleanup, and
                         ownership transfer to a caller-selected transport

pwright-plugin-tracing-navigation
                         Independently installable `navigation.goto` trace
                         layer with injected Page-to-Context routing and
                         correlated before/after/error entries

pwright-plugin-tracing-artifact
                         Task-free exact-size trace-to-Artifact handoff with
                         ordered Create fence and failed-fence compensation

pwright-runtime          Executor-independent direct/controlled dispatch,
                         resource-accounted lifecycle/pending/active calls,
                         lifecycle fan-out, bounded calls, cancellation,
                         injected deadlines, per-target FIFO admission,
                         connection-wide admission, terminal transport state,
                         bounded cleanup queue, rollback-on-drop prepared
                         creation/disposal, resource-accounted lifecycle outbox, and
                         validated production resource claim

pwright-runtime-control  Caller-driven bidirectional control handling;
                         lifecycle claims, cancellation, transactional async
                         backend subscription activation with rollback,
                         cancellable in-flight compensation, retryable Dispose
                         and EOF deactivation, resource-accounted routes,
                         supervisor outbound lane, and bounded priority
                         lifecycle-before-result completion barriers

pwright-connection-control Core-only, allocation-free Close/CloseAck state
                         machines; explicit cleanup/send/shutdown actions and
                         cancellation-safe retry phases

pwright-runtime-tokio    Optional bounded Tokio deadline/cleanup execution;
                         not pulled by core or generic runtime

pwright-transport        Core-only bounded frames, version policy, pluggable
                         codec/transport contracts, response correlation,
                         bounded hello/capability negotiation, optional shared
                         frame/fragment accounting, opt-in cancellation-safe
                         write shutdown, and production ledger proof

pwright-transport-memory Standard-library-only bounded full-duplex frame pair;
                         lazy caller-driven send/receive/shutdown, explicit
                         overflow and peer-close errors, and typed client/server
                         transport conformance

pwright-transport-stream Standard-library canonical binary WireFrame envelope;
                         validates all declared sizes, derives attainable
                         full-duplex retention bounds, and reserves before copy

pwright-transport-tokio  Optional caller-owned AsyncRead/AsyncWrite transport;
                         independent reader/writer locks, generic clean-EOF
                         shutdown, bidirectional staging accounting and
                         production claim

pwright-endpoint-tokio   Optional private Unix, Tailnet-only mutual-PSK TCP,
                         and caller-configured general mTLS endpoints; each
                         yields proof-carrying identity before Pwright
                         handshake, with bounded TLS handshakes/buffers and no
                         implicit accept loop, task, runtime, clock, or deadline

pwright-composition-chromium-tokio
                         Optional leaf joining a caller-pre-bound authenticated
                         Unix/Tailnet/mTLS endpoint to fresh Chromium
                         compositions; each explicit accept authenticates one
                         peer before CDP, statically pairs its gate and exact
                         ledger-claimed transport through handshake, and adds
                         no listener loop, task, retry, deadline, or browser authority

pwright-codec-binary     Shared allocation-aware binary reader/writer and
                         bounded generated-value codecs plus typed record/union
                         and command schema metadata, constraints, and compatibility

pwright-codec-common     Generated reusable call metadata, duration, and
                         failure-kind wire library; no capability root

pwright-codec-selector   Generated reusable recursive selector wire library;
                         no Locator commands or capability root

pwright-codec-screenshot Command-free shared screenshot attachment metadata,
                         validation, copy, and result-lease accounting

pwright-schema-gen       Development-only strict YAML-to-Rust capability
                         or library model/codec/catalog generator with direct
                         imports, static routing metadata, Markdown references,
                         release baselines, and drift checks

pwright-codec-json       Optional Serde JSON payload codec; route-checked and
                         intentionally rejects binary attachments

pwright-codec-control    Serde-free canonical lifecycle, cancellation, and
                         subscription payloads using the shared binary contract

pwright-codec-handshake  Serde-free canonical ClientHello/ServerHello payloads;
                         generated segmented layout, bounded capability ranges,
                         and legacy hello decoding

pwright-codec-navigation Serde-free bounded BeginNavigation/Page.goto command
                         and result payloads generated from capability YAML;
                         borrowed encoding avoids intermediate wire trees

pwright-codec-context-lifecycle
                         Serde-free bounded BrowserContext.close command/result
                         codec generated from the lifecycle capability schema

pwright-codec-context-creation
                         Serde-free bounded Browser.newContext command/result
                         codec generated from the creation capability schema

pwright-codec-page-creation
                         Serde-free bounded BrowserContext.newPage codec with
                         one Page plus required main-Frame initializer

pwright-codec-page-lifecycle
                         Serde-free bounded Page.close/run-before-unload codec
                         generated from the Page lifecycle capability schema

pwright-codec-browser-lifecycle
                         Serde-free bounded owned Browser.close codec; rejects
                         client timeout/cancellation metadata and attachments

pwright-codec-locator    Serde-free bounded Locator 1.22/2.5 click/hover/focus/blur/press/scroll-into-view/set-checked/select-option/count/fill/state/visibility/wait/raw-text/rendered-text/markup/attribute/value
                         payloads; shared schema, canonical ECMAScript regex,
                         user-facing attribute/test-ID leaves, and selector
                         validation

pwright-codec-page-events
                         Fixed-size binary PageLoad, BrowserContext.page, and
                         Page.popup codecs with sequence, source/related Page
                         identities, and exact route validation

pwright-codec-network-events
                         Bounded canonical binary Request/Response/terminal
                         payload family; four public tags share one sequence
                         and reject route/payload mismatches and attachments

pwright-codec-browser-events
                         Fixed-size binary Browser.disconnected codec with
                         sequence, Browser identity, and exact route validation

pwright-codec-content    Serde-free `pwright.content` Page screenshot codec;
                         validates options and one binary attachment, then
                         retains decoded client bytes under an explicit lease

pwright-codec-locator-screenshot
                         Serde-free selector/options payload plus one bounded
                         screenshot attachment and retained result lease

pwright-codec-dom-content
                         Serde-free `pwright.dom_content` Frame serialization
                         codec; validates one UTF-8 attachment and retains the
                         caller result under a dedicated lease

pwright-codec-handle-lifecycle
                         Serde-free bounded Handle.dispose command/result codec

pwright-codec-handle-evaluation
                         Serde-free bounded Frame.evaluateHandle codec with a
                         portable concrete-kind initializer and no native IDs

pwright-client           Lightweight typed Browser/Context/Page/Frame handles,
                         weak connection ownership, generic local Locator
                         values, bounded announced/claimed identity state, and
                         non-wrapping call IDs; each root family can share one
                         bounded terminal-reason allocation

pwright-client-navigation
                         Independent opt-in begin-navigation and Page.goto
                         extension traits over the portable capability

pwright-client-context-lifecycle
                         Independent opt-in BrowserContext.close extension;
                         idempotent for already-stale or racing-closed targets

pwright-client-context-creation
                         Independent opt-in no-options Browser.newContext
                         extension returning one typed announced Context

pwright-client-page-creation
                         Independent opt-in BrowserContext.newPage extension;
                         atomically claims Page and main Frame

pwright-client-page-lifecycle
                         Independent opt-in ordinary Page.close and strict
                         before-unload initiation extension traits

pwright-client-browser-lifecycle
                         Independent opt-in Browser.close extension; treats an
                         already-stale Browser as idempotent success and
                         installs a bounded reason before first dispatch

pwright-client-content  Independent opt-in Page screenshot extension returning
                         budgeted bytes; filesystem output remains client-local

pwright-client-locator-screenshot
                         Independent opt-in Locator screenshot extension over
                         the Locator's Page/Frame scope

pwright-client-dom-content
                         Independent opt-in Frame.content and Page-to-main-Frame
                         facade returning non-cloneable budgeted UTF-8

pwright-client-handle-lifecycle
                         Independent explicit async JSHandle/ElementHandle
                         disposal with idempotent stale-target handling

pwright-client-handle-evaluation
                         Independent Page/Frame evaluateHandle extensions;
                         remote results require the exact prior Create

pwright-client-locator  Independent opt-in selector factories, local Locator
                         composition including regex and user-facing attribute
                         matchers, typed 1.x/2.x click/focus/blur/press and 1.x hover/count/fill/state/visibility/wait/content/attribute/value extensions,
                         and client-composed hidden state

pwright-client-wire      Executor-neutral encoded command channel, unique
                         receive/correlation driver, shared pending-call budget,
                         serialized explicit transport shutdown, composable
                         frame handlers, and production transport/backlog
                         resource profiles

pwright-client-control   Opt-in server-to-client lifecycle handler; forwards
                         Create/Adopt and applies Dispose invalidation

pwright-client-events    Opt-in per-event typed bounded queues with shared
                         item/decoded-byte budgets and bounded first/last-
                         listener subscription edge tracking; sequence/gap and
                         reconnect-epoch diagnostics distinguish wire loss from
                         local overflow; production profiles enforce local
                         decoded-size contracts

pwright-client-page-events
                         Opt-in PageLoad, BrowserContext.page, and Page.popup
                         handler/subscription helpers with reliable
                         RejectNewest overflow

pwright-client-network-events
                         Opt-in routed Request/Response/terminal handler with
                         one shared continuity counter and production byte/item
                         budget; dynamic listener activation remains separate

pwright-client-browser-events
                         Opt-in Browser.disconnected handler/subscription
                         helper with reliable terminal-event overflow

pwright-server-wire      Unique-reader caller-driven server, composable async
                         handlers and ordered reasoned termination barriers,
                         certified one-frame prefetch for EOF responsiveness,
                         handshake-first gating, typed command
                         inventory, resource-accounted retained commands and
                         hello encoding, fair wakeable outbound lanes, bounded
                         drain deadline, dual-error preservation, explicit
                         async cleanup/transport close, and result sending

pwright-server-events    Optional one-EventSpec producer/outbound channels;
                         bounded typed queues, cancellation-safe front claims,
                         monotonic non-wrapping sequences, canonical event-route
                         checks, production item/byte/encode accounting, and
                         static outbound composition

pwright-server-navigation
                         Generated opt-in complete navigation typed-handler
                         chain; injects services and chooses no runtime/backend

pwright-server-context-lifecycle
                         Generated opt-in BrowserContext.close typed-handler
                         chain; injects services and chooses no backend

pwright-server-context-creation
                         Generated opt-in Browser.newContext typed-handler
                         chain; injects its service and chooses no backend

pwright-server-page-creation
                         Generated opt-in BrowserContext.newPage typed-handler
                         chain; injects its service and chooses no backend

pwright-server-page-lifecycle
                         Generated opt-in Page close/before-unload typed-handler
                         chain; injects services and chooses no backend

pwright-server-browser-lifecycle
                         Generated opt-in Browser.close typed-handler chain;
                         injects one authority-bearing service and no backend

pwright-server-browser-events
                         Task-free, queue-free, cancellation-safe one-shot
                         Browser.disconnected publisher plus native-loss
                         ordered-disposal control decorator

pwright-server-content  Generated opt-in Page screenshot typed-handler chain;
                         injects one target-aware service and chooses no backend

pwright-server-locator-screenshot
                         Generated one-command Locator screenshot handler chain;
                         advertises neither Locator nor Page screenshot

pwright-server-dom-content
                         Generated opt-in Frame.content typed-handler binding;
                         injects one target-aware service and chooses no backend

pwright-server-handle-lifecycle
                         Generated opt-in Handle.dispose typed-handler binding

pwright-server-handle-evaluation
                         Generated opt-in Frame.evaluateHandle typed-handler
                         binding; Chromium composition adds ordered creation,
                         cancellation continuation, and Result-delivery cleanup

pwright-cap-context-lifecycle
                         Core-only Playwright-shaped BrowserContext.close
                         contract; no CDP, codec, transport, or runtime coupling

pwright-cap-context-creation
                         Core-only no-options Browser.newContext contract;
                         explicitly depends on the lifecycle capability

pwright-cap-page-creation
                         Core-only no-options BrowserContext.newPage contract;
                         returns Page plus main Frame and depends on Page close

pwright-cap-page-lifecycle
                         Core-only split ordinary-close/before-unload Page
                         contract; no CDP, codec, transport, or runtime coupling

pwright-cap-browser-lifecycle
                         Core-only owned Browser.close contract with typed
                         authorization/shutdown/cleanup failures

pwright-cap-browser-events
                         Core-only optional Browser.disconnected event contract;
                         no backend, codec, queue, or runtime dependency

pwright-cap-handle-lifecycle
                         Core-only explicit JSHandle/ElementHandle disposal
                         contract

pwright-cap-handle-evaluation
                         Core-only Frame.evaluateHandle contract; its protocol
                         depends on Handle lifecycle and reuses bounded value
                         argument types

pwright-cap-navigation   Core-only begin-navigation and Playwright-shaped goto
                         contracts

pwright-cap-page-events  Core-only Playwright-shaped PageLoad,
                         BrowserContext.page, and Page.popup event contracts
                         (v1.2); no backend, codec, queue, or runtime dependency

pwright-cap-network-events
                         Core-only Request/Response/requestfinished/requestfailed
                         contracts for Page or BrowserContext subscription; no
                         body, routing, WebSocket, HAR, codec, queue, or runtime

pwright-cap-content      Core-only optional Page screenshot command identity and
                         typed failures; compatibility-reexports the shared model

pwright-cap-locator-screenshot
                         Core-only selector plus screenshot-value command for
                         one cancellation-bounded Locator capture operation

pwright-cap-dom-content  Core-only optional read-only Frame DOM serialization,
                         budgeted UTF-8 result, errors, and generated metadata

pwright-cap-locator      Core-only Locator command identities, params, results,
                         and failures; compatibility-reexports the shared selector model,
                         plus independent 1.22 and 2.5 click/focus/blur/press/scroll-into-view/set-checked/select-option tracks and
                         1.x hover/count/fill/state/visibility/wait/content/attribute/value contracts; no
                         DOM/backend identities

pwright-backend-cdp-common
                         Command-free CDP failure classification shared by
                         isolated adapters; owns no capability or service

pwright-backend-cdp-navigation
                         Isolated attach-only CDP navigation adapter; bounded
                         event-wait and response state, no aggregate backend

pwright-backend-cdp-topology
                         Command-free bounded Browser/Context/Page/Frame/World
                         CDP state, reconnect and lifecycle effects, plus
                         resource-accounted rollback transactions

pwright-backend-cdp-context-creation
                         Isolated no-options BrowserContext creation with
                         bounded native identity and exact compensation

pwright-backend-cdp-context-lifecycle
                         Isolated owned-BrowserContext close adapter; default
                         and external ownership fail before CDP

pwright-backend-cdp-page-creation
                         Isolated about:blank Page creation/attachment/main-
                         Frame discovery with exact-target compensation

pwright-backend-cdp-page-lifecycle
                         Isolated ordinary Page close/absence confirmation and
                         separate run-before-unload Page.close adapter

pwright-backend-cdp-handle-lifecycle
                         Isolated bounded JavaScript Handle disposal adapter;
                         exactly one Runtime.releaseObject operation

pwright-backend-cdp-handle-evaluation
                         Isolated retained Handle creation, Handle-local
                         evaluation, value extraction, and bounded properties

pwright-backend-cdp-input
                         Isolated Page-local Playwright US-key resolution and
                         serialized keyboard/mouse transaction foundation;
                         no selector, Locator command, runtime, or browser owner

pwright-backend-cdp-locator
                         Complete attach-only Chromium Locator backend over
                         selector-runtime and Page input; no screenshot,
                         aggregate backend, runtime, provider, or process owner

pwright-composition-chromium-locator
                         Minimal generated Locator 1.22/2.5 server composition
                         for one explicit Page/Frame route; caller-owned CDP,
                         Page input, transport driver, runtime, and browser

pwright-composition-chromium-context-lifecycle
                         Minimal generated BrowserContext.close server leaf;
                         stages topology/runtime ownership, reserves bounded
                         continuation, and injects descendant-retirement policy
                         without linking Page routes, Handles, or network

pwright-composition-chromium-context-creation
                         Paired generated Browser.newContext and
                         BrowserContext.close server leaf; caller-owned CDP,
                         topology, runtime, identities, continuations, and
                         pluggable pre-admitted publication/retirement state

pwright-composition-chromium-page-lifecycle
                         Minimal generated Page.close and before-unload server
                         leaf; confirms exact native target disappearance,
                         stages topology/runtime retirement, and injects Page
                         session resolution plus optional route cleanup

pwright-composition-chromium-page-creation
                         Paired BrowserContext.newPage and Page-lifecycle leaf;
                         atomically publishes Page/Frame identity, supports
                         provisional route plugins, and compensates the exact
                         native target on publication failure

pwright-composition-chromium-handle-lifecycle
                         Minimal generated Handle.dispose server leaf; owns
                         bounded producer reservations/routes, accepts opaque
                         backend payloads and injected native disposers, and
                         transfers abandoned release to caller-owned cleanup

pwright-composition-chromium-handle-evaluation
                         Generated Handle evaluation 1.2 server leaf; composes
                         all six Frame/Handle commands over lifecycle-owned
                         routes with bounded Frame worlds, atomic single/batch
                         publication, native compensation, and caller-owned
                         abandoned-result cleanup

pwright-plugin-ordered-handle-delivery
                         Optional backend-neutral remote Handle fence; reserves
                         cleanup before browser work, orders single/batch Create
                         before Result, tracks physical Result delivery, and
                         performs ordered abandonment/explicit disposal

pwright-backend-cdp-page-events
                         Isolated Page load/topology-effect normalization over
                         the bounded CDP topology leaf

pwright-backend-cdp-network-events
                         Isolated bounded Page/session HTTP lifecycle
                         normalization with no response-body ownership

pwright-backend-cdp-content
                         Isolated attach-only Page screenshot adapter; bounded
                         native capture and base64 decode, with no DOM,
                         evaluation, navigation, or full backend

pwright-backend-cdp-locator-screenshot
                         Isolated Locator screenshot capture orchestration and
                         Frame clip translation; consumes an injected geometry
                         resolver and imports no public Locator commands

pwright-backend-cdp-selector-runtime
                         Command-free bounded selector compiler and versioned
                         web-platform runtime shared by full and leaf backends

pwright-backend-cdp-selector-geometry
                         Independently installable strict Chromium selector-to-
                         viewport-geometry plugin with explicit resource bounds

pwright-backend-cdp-dom-content
                         Isolated attach-only read-only Frame DOM adapter;
                         fixed utility-world serializer, bounded result and
                         route ownership, with no public evaluation

pwright-backend-cdp-evaluation
                         Isolated attach-only CDP value-evaluation adapter;
                         bounded typed serialization and retained execution-
                         context routes, with no navigation or full backend

pwright-backend-cdp      Compatibility aggregate over extracted navigation/
                         topology/screenshot/DOM/evaluation plus Locator, no-options
                         context/Page creation, owned-context close, and split Page lifecycle services,
                         caller-driven PageLoad lifecycle activation plus pure
                         topology discovery-event normalization,
                         bounded HTTP lifecycle normalization with stable
                         Request/Response identities and production state lease,
                         bounded context/target/session/frame/world recovery map,
                         and rollback-on-drop production topology transactions

pwright-browser-backend  Core-only backend identity, support manifest, typed
                         preflight binding, and explicit native-access boundary

pwright-browser-provider Standard-library-only attach/launch ownership types

pwright-browser-install  Zero-dependency explicit managed-install pipeline;
                         no downloader, runtime, launcher, or process authority

pwright-provider-cdp     Explicit WebSocket CDP attachment; never owns process;
                         optional plan-checked production connector

pwright-provider-chromium
                         Opt-in installed/explicit Chromium launch; dedicated
                         fresh/marked-persistent profiles, loopback CDP,
                         consuming process lease, and bounded orderly shutdown

pwright-composition-chromium-content
                         Minimal attach-only one-Page screenshot server leaf;
                         generated one-command handler, combined source-result/
                         decoded-result/wire plan, and no DOM, evaluation,
                         Locator, navigation, client, provider, installer,
                         runtime choice, Node.js, or browser bytes

pwright-composition-chromium-locator-screenshot
                         Minimal Page/Frame Locator screenshot server leaf;
                         generated handler, capture/result/wire plan, and
                         pluggable selector-to-geometry resolution

pwright-composition-chromium-dom-content
                         Minimal attach-only one-Frame DOM-content server leaf;
                         generated one-command handler, exact utility route,
                         combined result/route/wire plan, and no evaluation,
                         screenshot, navigation, client, provider, installer,
                         runtime choice, Node.js, or browser bytes

pwright-composition-chromium-evaluation
                         Minimal attach-only one-Frame evaluation server leaf;
                         generated one-command handler, combined backend/route/
                         wire plan, no aggregate backend, navigation, Handle,
                         client, provider, installer, runtime choice, Node.js,
                         or browser bytes

pwright-composition-chromium-navigation
                         Minimal attach-only Chromium navigation server leaf;
                         dedicated bounded CDP backend, generated two-command
                         handlers, combined backend/wire resource plan, and no
                         aggregate backend, unrelated capabilities, provider,
                         launcher, installer, runtime, transport choice,
                         Node.js, or browser bytes

pwright-composition-chromium-launch
                         Optional owned-process leaf; explicit launch request,
                         plan/ledger-proven CDP binding, unique process lease,
                         launch-only negotiated Browser.close grant,
                         caller-reserved continuation, ordered whole-tree
                         disconnected-before-root-dispose wrapper, and
                         cancellation-safe bounded shutdown

pwright-composition-chromium
                         Explicit attach-only production root; capability-subset
                         manifests, reusable stateless attach recipe with a
                         fresh ledger/topology/ObjectId namespace per call,
                         staged ledger-verified remote transport and
                         caller-installed runtime ownership unit, consuming
                         same-supervisor handshake and application-install
                         stages, a negotiated pre-provisioned-Page navigation
                         recipe with explicit omitted lanes, plus an owned
                         control/topology automatic-discovery assembly with a
                         client-observed live navigation proof, mandatory
                         native-liveness backend wrapping, optional negotiated
                         queue-free Browser disconnected terminal bundle, and an opt-in
                         complete current-capability recipe with every command,
                         activated control, typed Page-event outbound, topology,
                         independent PageLoad backend, Handle evaluation/disposal,
                         and mandatory Handle-aware runtime statically chained,
                         additive resource plan, bounded Page-to-CDP
                         routing, direct/controlled runtime and transactional
                         topology/lifecycle publication, coordinated owned-
                         context/Page creation and close with remote client-applied ordering,
                         cancellation-safe owner-supplied continuation, first-
                         class attached shared-topology/service factory,
                         selected generated
                         server-handler factories, optional remote handshake,
                         negotiated role-aware control with composable graceful
                         close/outbound factories, typed-event profiles/parts,
                         attached negotiated factories for every implemented
                         capability handler, including independent Handle
                         lifecycle/evaluation profiles with shared route ownership,
                         ordered topology-creation/event publication fence,
                         optional bounded Page network cursors with atomic
                         Request/Response runtime admission and final-Create
                         fencing plus one negotiated cross-event ordered remote
                         network codec/outbound (generic per-tag lanes rejected),
                         bounded topology loss/termination pump and fair
                         four-lane connection driver,
                         budgeted protocols/supervisor, and no implicit browser,
                         Node.js, download, launch, or listener choice
```

The navigation extension now includes a Playwright-shaped `Page::goto` slice
alongside its lower-level primitive. See [Composable architecture v2](architecture-v2.md),
[v2 design tasks](design-tasks-v2.md), and the
[Playwright design course](knowledge/playwright-design-course.md), plus the
[resource ownership and limits](resource-boundaries-v2.md),
[protocol IDL decision record](knowledge/pwright-protocol-idl.md) and
[generator output ownership](knowledge/pwright-generator-output-ownership.md), for the
implemented wait/response boundary, remaining parity work, and source evidence.
The independently installable value-evaluation slice is implemented and verified in
[value evaluation](knowledge/pwright-value-evaluation.md): Page delegates to
its main Frame, by-value serialization is bounded, and live JS/Element Handles
remain a separate capability rather than an implicit memory cost.
The third isolated server slice is read-only
[Page/Frame content](knowledge/pwright-page-content.md): one exact utility
world, a fixed serializer, explicit result/route/wire accounting, and no public
evaluation, screenshot, or navigation authority.
The fourth is [Page screenshot](knowledge/pwright-screenshot.md): native capture
returns a bounded binary attachment while backend, decoded-protocol, and wire
owners are budgeted separately, without installing DOM or evaluation.
Value evaluation now has a first explicit
[Handle disposal checkpoint](knowledge/pwright-handle-lifecycle.md): stable
Handle object kinds, independently installable wire/client/server pieces, and
a one-shot native CDP release leaf. The dependent
[Handle evaluation checkpoint](knowledge/pwright-handle-evaluation.md) adds a
portable initializer, concrete JSHandle/ElementHandle client materialization,
bounded Chromium native routes, atomic runtime creation/disposal, and live
Chrome evidence. Handle arguments and the live negotiated Handle-command
round trip are now verified. The optional
[ElementHandle adoption](knowledge/pwright-element-handle-adoption.md) contract
now separates a bounded Chromium primitive from the still-gated public
composition. The
Handle-local surface is designed as the additive 1.2 capability in
[Handle-local operations](knowledge/pwright-handle-operations.md). Optional production and
post-handshake installation, Create/Dispose ordering, Result-delivery
cancellation cleanup, route metrics, plus
explicit Page/Context, main-world replacement, Frame detach, Page crash,
reconnect, Handle-aware connection-close retirement, and direct abandoned-
evaluation continuation cleanup are now implemented. The complete recipe now
requires that Handle-aware wrapper and its 14-protocol Chrome 152 handshake is
verified without taking ownership of the external browser.
Its optional [Chromium topology-event driver](knowledge/pwright-chromium-topology-events.md)
now turns generated Page/popup, local-Frame, OOPIF, and execution-world events
into the same transactional topology/runtime/evaluation-route publication. A
separate discovery selection owns browser-root auto-attach, bounded default
Context classification, complete paused-target frame-tree initialization, and
explicit resume. It is caller-driven, owns no executor task, and treats
broadcast lag as a sticky resynchronization boundary. On a live transport it
can rebuild an authoritative target snapshot under a new epoch, preserving
portable ObjectIds while replacing CDP sessions and disposing absent objects.
Its explicit topology publisher forwards ready Page and popup effects only to
installed bounded typed-event lanes and reports queue loss as requiring
resynchronization. The fresh remote factory now continues through a consuming
same-supervisor handshake and an owned application-install context. That
context lets plugins assemble negotiated handler and typed-event lanes, must
transfer a selected unique topology driver, and produces a final four-lane
application connection whose caller backend is always wrapped by native CDP
liveness and whose terminal run shuts down the exact remote transport.
The optional Tokio endpoint server holds only a caller-pre-bound authenticated
endpoint, the stateless factory, and an explicit external CDP URL. Every
caller-polled `accept_connection` authenticates exactly one peer before
creating that peer's fresh ledger and CDP attachment; accept loops, admission
concurrency, retry, and deadlines remain deployment-owned.
The complete opt-in Chromium recipe refuses partial installation and chains all
currently implemented Context/Page and Handle creation and lifecycle, evaluation,
navigation, locator, control, Page-event, topology, and native-liveness pieces.
PageLoad and topology ingestion use independent cursors on the same bounded CDP
broadcast rather than competing readers; both remain caller-polled and make
loss terminal. Cleanup draining and Page-close polling stay injected policies.
Its explicitly selected Chromium utility-world profile creates one
Page-unique isolated world in every current Page/OOPIF frame tree and registers
it for future documents before target resume. Locator services are published
per Frame and bind the exact local or OOPIF session plus utility
`uniqueContextId`; Page calls delegate to the main Frame and user Frame
evaluation remains routed to the main world. World replacement atomically
replaces the locator service and its selector-runtime cell under a separately
reserved route-memory bound.

Browser attachment and browser launch are separate provider traits. An
external CDP attachment can disconnect its transport but has no process lease,
so it cannot terminate the user's browser. Installed-browser launch is now an
explicit optional v2 leaf whose connection must prove the current resource
plan and ledger; managed download remains a separate explicit contract and is
never invoked by attachment or launch.

## Architecture Decisions

These are intentional. Do NOT propose changing them.

### 1. Legacy/default path remains attach-only
The existing CLI/server and minimal v2 composition do not download, install,
or manage Chrome. The optional v2 launch leaf is a separate dependency and
requires an explicit installed executable plus dedicated profile.

### 2. No gRPC authentication
Binds `127.0.0.1` by default. Documented. `--bind-all` requires firewall.
Auth is a future feature request, not a missing security fix.

### 3. Tab lifecycle is caller-managed
`Browser::new_tab` returns a `TabHandle` with explicit `close()`. Callers
own the tab lifecycle. There is no auto-close convenience wrapper --
callers must close tabs themselves to avoid leaks.

### 4. Ref cache staleness is expected
CLI's snapshot-act workflow stores refs in `.pwright/state.json`. DOM changes
between invocations invalidate refs. This is the documented design. The gRPC
server uses per-tab locks for concurrent access.

### 5. `Page::close()` takes `&self`
Uses `AtomicBool` for thread-safe close from multiple tasks sharing `Arc<Page>`.

### 6. JS selectors use `returnByValue: false`
`get_by_text/label/role` and `filter_by_text` evaluate JS to find elements,
then `DOM.requestNode` for nodeIds. Must call `DOM.getDocument` first to
enable the DOM domain. Uses `runtime_evaluate_as_object` (not `runtime_evaluate`).

### 7. CdpConnection uses CancellationToken
On drop, cancels reader/writer tasks via `tokio::select!`. Writer sends
WebSocket Close frame. Do not revert to `drop(writer_handle)`.

### 8. DOM methods use `nodeId` not `backendNodeId`
Callers pass nodeIds from `querySelector`. These are NOT backendNodeIds.
CDP accepts both as separate parameters - they're different ID spaces.

### 9. Download race condition is acceptable
Chrome's `Browser.downloadProgress` provides the filename. Predicting it
for a symlink attack is not practical.

### 10. Snapshot depth O(n*d) is acceptable
Typical DOM depth <20. BFS optimization not worth the complexity.

## How to Learn This Project

1. Read this file first
2. Read `docs/architecture.md` for crate internals
3. Read `docs/known-issues.md` for current bugs and their status
4. Read `docs/knowledge/testing-strategy.md` for test architecture
5. Run `cargo test --workspace` to verify your environment
6. Run integration tests against real Chrome to see CDP in action

## Testing

### Tier 1: MockCdpClient (call sequence verification)
Records which CDP methods are called and in what order.
Use for: "click dispatches mousePressed then mouseReleased"

### Tier 2: FakeCdpClient (behavior verification)
In-memory DOM with real CSS selector matching.
Use for: "is_checked returns true for checked checkbox"

### Tier 3: Docker integration (end-to-end)
Real Chrome via `chromedp/headless-shell` with in-process test server.
96 tests across 15 files. Concurrency tests with 5 parallel tabs.

**Integration test gotchas:**
- Chrome rejects non-IP `Host` headers (resolve hostname to IP)
- Chrome returns internal WS URL (use `rewrite_ws_url`)
- Test server/CDP connection need dedicated threads (survive `#[tokio::test]` runtime boundaries)
- Share one CDP WebSocket per test binary via `OnceLock`

## CDP Protocol Notes

- `DOM.getDocument` must be called before `DOM.requestNode` (enables DOM domain)
- `Target.attachToTarget` with `flatten: true` multiplexes sessions on one WS
- `Runtime.evaluate` with `returnByValue: true` serializes DOM nodes as null
- `Runtime.evaluate` with `returnByValue: false` returns objectId for DOM nodes
- Chrome's HTTP endpoint rejects Host headers that aren't localhost or an IP

## Key Files

| File | Purpose |
|------|---------|
| `CLAUDE.md` | Slim rules for agents (loaded every conversation) |
| `docs/index.md` | This file (full project guide) |
| `docs/known-issues.md` | Prioritized bugs and improvements |
| `docs/architecture.md` | Crate internals and design principles |
| `docs/architecture-v2.md` | Composable v2 architecture checkpoints and boundaries |
| `docs/design-tasks-v2.md` | Ordered v2 design questions and implementation evidence |
| `docs/knowledge/playwright-design-course.md` | Playwright architecture lessons and source anchors |
| `docs/knowledge/pwright-tracing.md` | Optional base tracing boundary, contributor separation, state machine, resource contract, atomic file-sink format, and Playwright source evidence |
| `docs/knowledge/pwright-artifacts-and-streams.md` | Playwright remote Artifact/Stream evidence, Pwright pull-transfer decisions, two-phase read semantics, resources, and remaining protocol gates |
| `docs/knowledge/pwright-dependency-topology.md` | Command-free topology, Context/Page creation and lifecycle, independently selectable paired Context creation/lifecycle, Handle lifecycle and retained evaluation, Page/network events and input, complete Locator backend/composition, navigation, evaluation-only, DOM-content-only, Page-screenshot-only, and Locator-screenshot Chromium Cargo leaves, no-default-features umbrella, isolated consumers, production resources, and exact CI dependency allowlists |
| `docs/knowledge/pwright-page-close.md` | Ordinary versus before-unload Page close contracts and acceptance gates |
| `docs/knowledge/pwright-browser-close.md` | Browser close versus disconnect and provider/process authority boundaries |
| `docs/knowledge/pwright-context-creation.md` | BrowserContext creation ownership, rollback, remote ordering, and client identity |
| `docs/knowledge/pwright-protocol-idl.md` | Pwright source-schema options, decision, rules, and generator workflow |
| `docs/knowledge/pwright-generator-output-ownership.md` | Multi-output generator ownership, dependency direction, and migration sequence |
| `docs/knowledge/pwright-handle-lifecycle.md` | Explicit Handle disposal contract, CDP release leaf, and relationship to dependent Handle creation |
| `docs/knowledge/pwright-handle-evaluation.md` | Optional evaluateHandle creation/disposal transaction, Handle-valued arguments, exact-world rules, live Chrome evidence, and remaining compatibility gates |
| `docs/knowledge/pwright-element-handle-adoption.md` | Playwright ElementHandle cross-context semantics, Chromium describe/resolve mapping, temporary ownership, and public-composition gates |
| `docs/knowledge/pwright-handle-operations.md` | Playwright-aligned Handle-local 1.2 command set, ownership, bounded property enumeration, atomic publication, and acceptance gates |
| `docs/knowledge/pwright-locator-focus.md` | Playwright-aligned Locator 1.18/2.1 focus/blur semantics, compatibility boundaries, CDP mapping, resources, and acceptance gates |
| `docs/knowledge/pwright-locator-press.md` | Playwright-aligned Locator 1.19/2.2 Press contract, key grammar, Page-owned input state, CDP mapping, resources, and acceptance gates |
| `docs/knowledge/pwright-locator-set-checked.md` | Playwright-aligned Locator 1.20/2.3 SetChecked primitive, Check/Uncheck composition, idempotence, radio rules, shared input, resources, and acceptance gates |
| `docs/knowledge/pwright-locator-select-option.md` | Playwright-aligned Locator 1.21/2.4 descriptor/string SelectOption semantics, Handle boundary, resources, state machine, and acceptance gates |
| `docs/knowledge/pwright-locator-scroll-into-view.md` | Playwright-aligned Locator 1.22/2.5 ScrollIntoViewIfNeeded stability, intersection, retry/commit, initializer-boundary, resource, and acceptance decisions |
| `crates/pwright-cap-handle-lifecycle/schema/protocol-reference.md` | Generated Handle.dispose command, wire layout, failure details, and constraints |
| `crates/pwright-cap-handle-evaluation/schema/protocol-reference.md` | Generated evaluateHandle command, initializer, wire layout, failure details, and constraints |
| `crates/pwright-cap-context-lifecycle/schema/protocol-reference.md` | Generated BrowserContext.close command, wire layout, failure details, and constraints |
| `crates/pwright-cap-context-creation/schema/protocol-reference.md` | Generated no-options Browser.newContext command, initializer, failure details, and constraints |
| `crates/pwright-cap-page-creation/schema/protocol-reference.md` | Generated no-options BrowserContext.newPage command, Page/main-Frame initializer, failure details, and constraints |
| `crates/pwright-cap-page-lifecycle/schema/protocol-reference.md` | Generated Page close and run-before-unload commands, wire layout, failure details, and constraints |
| `crates/pwright-cap-browser-lifecycle/schema/protocol-reference.md` | Generated owned Browser.close command, wire layout, authorization, shutdown, and cleanup failures |
| `crates/pwright-cap-navigation/schema/protocol-reference.md` | Generated navigation commands, models, wire layout, and constraints |
| `crates/pwright-cap-locator/schema/protocol-reference.md` | Generated locator 1.x commands, recursive selector model, wire layout, and constraints |
| `crates/pwright-cap-locator/schema/protocol-reference-v2.md` | Generated locator 2.x commands, extensible Click record, focus management, wire layout, and constraints |
| `crates/pwright-codec-control/schema/protocol-reference.md` | Generated lifecycle/control message wire layout and constraints |
| `crates/pwright-codec-handshake/schema/protocol-reference.md` | Generated ClientHello/ServerHello segmented wire layout and constraints |
| `docs/knowledge/script-runner-design.md` | Script runner 5-phase plan |
| `docs/knowledge/testing-strategy.md` | Fake + Docker test architecture |
| `docs/knowledge/playwright-api.md` | Playwright API compatibility matrix |
| `docs/knowledge/feature-comparison.md` | pwright vs Playwright vs PinchTab |
| `docs/knowledge/chrome-devtools-mcp-comparison.md` | pwright CLI vs Google's chrome-devtools-mcp CLI |
| `docs/knowledge/cdp-codegen-design.md` | CDP protocol codegen crate design |
| `docs/knowledge/pwright-object-model.md` | Stable browser object IDs/tags, schema targets, and runtime admission |
| `docs/knowledge/network-capture-design.md` | Network capture CLI design (second CDP session) |
| `docs/knowledge/url-capture-design.md` | Raw browser URL capture before page/CORS/PDF transformation |
| `docs/codelabs.md` | Usage examples (Rust, CLI, gRPC, scripts) |
