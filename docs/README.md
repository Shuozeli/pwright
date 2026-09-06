<!-- agent-updated: 2026-09-05T12:38:10Z -->

# pwright Documentation

Lightweight Rust CDP bridge exposing Chrome browser control via gRPC.

> [!WARNING]
> pwright grants full browser control (navigation, JS execution, screenshots, cookies) to any client that can reach its gRPC port. **By default, gRPC binds to `127.0.0.1` only.** Use `--bind-all` with caution and always firewall both the CDP (9222) and gRPC (50051) ports. See [Security Considerations](getting-started.md#security-considerations).

## User Guides

- [Architecture](architecture.md) — Crate structure, layer design, concurrency model
- [Getting Started](getting-started.md) — What is CDP, Chrome setup, `.pwright` folder, troubleshooting
- [CLI Guide](cli-guide.md) — Full command reference, snapshot workflow, agent setup
- [gRPC API Reference](grpc-api.md) — All RPCs with request/response schemas
- [Codelabs](codelabs.md) — Usage examples with Rust, CLI, and gRPC

## Concepts

- [Pre-compiled Browser Skills for Agents](recipes-for-agents.md) -- Why recipes beat runtime exploration

## Script Runner & Recipes

- [Script Examples](../examples/scripts/) — Ready-to-use YAML automation scripts
- [Recipes](../examples/recipes/) — Pre-built recipes for search, monitoring, email, automation
- [Codelabs: Script Runner](codelabs.md#11-script-runner) — Tutorial with examples
- [Script Runner Design](knowledge/script-runner-design.md) — Full design with protobuf schema and phases
- [Recipes Design](knowledge/recipes-design.md) — Recipe specifications and design principles

## Guides

- [Connecting to Remote Chrome](guides/remote-cdp.md) — Direct bind, Caddy, nginx, SSH tunnel

## Agent Integration

- [Agent Skill](../skill/pwright/SKILL.md) — Skill definition for AI agent frameworks

## Network Capture

- [Network Capture Design](knowledge/network-capture-design.md) — Second CDP session listener architecture
- [Raw URL Capture Design](knowledge/url-capture-design.md) — CORS-independent, pre-PDF-viewer response acquisition
- CLI commands: `network-listen`, `network-list`, `network-get`

## Site Explorations

- [Reddit Structure](exploration/reddit-structure.md) — `shreddit-post` web component attributes (tested 2026-03-21)
- [X.com Structure](exploration/x-structure.md) — `data-testid` tweet selectors (tested 2026-03-21)

## Planning

- [Composable Architecture v2](architecture-v2.md) — Authorized exploratory
  slices, dependency boundaries, execution control, browser ownership,
  lifecycle, and resource invariants
- [Pwright Rust v2 Design Tasks](design-tasks-v2.md) — Ordered design gates and
  verified implementation checkpoints
- [Playwright Design Course](knowledge/playwright-design-course.md) — Source-linked
  reference behavior studied before v2 decisions
- [Network Lifecycle](knowledge/pwright-network-lifecycle.md) — Optional bounded
  Request/Response identity and event design, separated from bodies, routing,
  WebSocket, and HAR
- [Process-memory Attribution](knowledge/pwright-process-memory-metrics.md) —
  Optional bounded RSS sampling and caller-driven warm/steady/teardown recorder
  that correlates Pwright ledger/object state with Chromium process roles
- [Tracing Boundary](knowledge/pwright-tracing.md) — Base recorder, optional
  sinks/contributors, and the independently installed trace-to-Artifact producer
- [Artifacts and Streams](knowledge/pwright-artifacts-and-streams.md) — Bounded
  pull transfer, runtime lifecycle, and ordered Create/Dispose publication
- [Dependency Topology](knowledge/pwright-dependency-topology.md) — Independent
  navigation, value-evaluation, DOM-content, and Page-screenshot Chromium Cargo
  leaves, no-default-features
  umbrella, isolated client/server consumers, resource boundaries, and CI
  allowlists
- [Generated Tracing-Artifact Protocol Reference](../crates/pwright-cap-tracing-artifact/schema/protocol-reference.md) —
  One opt-in Context producer command with explicit tracing and Artifact dependencies
- [Generator Output Ownership](knowledge/pwright-generator-output-ownership.md) —
  Capability/API/codec/server/docs generation boundaries and install graph
- [Browser Object-Model Ownership](knowledge/pwright-object-model.md) — Stable
  semantic IDs, compact wire tags, parent topology, generated command targets,
  and runtime admission
- [BrowserContext Creation](knowledge/pwright-context-creation.md) — Optional
  no-options capability, prepared ownership transaction, remote ordering, and
  dedicated-Chrome evidence
- [Page Creation](knowledge/pwright-page-creation.md) — Optional
  `BrowserContext.newPage`, atomic Page/main-Frame publication, exact CDP
  compensation, remote ordering, and dedicated-Chrome evidence
- [Value Evaluation](knowledge/pwright-value-evaluation.md) — Implemented
  optional Page/main-Frame evaluation, bounded serialized values, and the explicit
  boundary that keeps live JS/Element Handles out of the default capability
- [Handle Lifecycle](knowledge/pwright-handle-lifecycle.md) — Optional explicit
  `JsHandle`/`ElementHandle` disposal contract and native CDP release leaf;
  foundation used by Handle-producing capabilities
- [Handle Evaluation](knowledge/pwright-handle-evaluation.md) — Optional
  `evaluateHandle` capability, portable Handle initializer, bounded Chromium
  native-route transaction, explicit release, and remaining compatibility gates
- [Page Screenshot](knowledge/pwright-screenshot.md) — Optional
  `pwright.content` 1.0 Page capture, binary attachment transfer, explicit
  format/clip/scale semantics, and backend/wire/client resource lifetimes
- [Page Content](knowledge/pwright-page-content.md) — Implemented optional
  `Page.content → main Frame.content` delegation, independent read-only
  `pwright.dom_content` capability, utility-world routing, navigation-race
  semantics, and budgeted UTF-8 ownership
- [Chromium Topology Events](knowledge/pwright-chromium-topology-events.md) —
  Optional caller-driven default Context, existing/new Page, popup,
  Frame/OOPIF/world synchronization with complete initial frame trees,
  transactional publication, bounded state, sticky lag, and caller-driven
  authoritative same-connection reconstruction, plus the optional bounded
  `BrowserContext.page`/`Page.popup` publication bridge and caller-owned
  terminal/reconstruction pump integrated into the fair four-lane server
  connection driver; native CDP disconnect is independently monitored and
  terminal; a reusable attach factory creates a fresh ledger, CDP topology,
  ObjectId namespace, and epoch per call; staged transport/runtime plugins own
  one ledger-verified remote connection unit; a consuming same-supervisor
  handshake and owned application installer compose selected negotiated
  handlers/outbound publishers, require transfer of the unique topology
  driver, wrap every backend with mandatory native liveness, and own exact
  transport shutdown; the optional Tokio composition leaf keeps an accepted
  Unix/Tailnet/mTLS peer gate paired with its exact ledger-claimed transport
  through that handshake; the first packaged recipe installs only navigation
  over a caller-pre-provisioned Page and rejects unnegotiated navigation or an
  unowned topology-event driver; a second assembly owns the controlled runtime,
  lifecycle control, and topology pump needed for automatic discovery without
  adding Page/popup event queues, fences topology-created identities even after
  the ordinary outbound lane reports idle, and has a live client-observed
  discovery-to-navigation proof; a caller-driven endpoint server now accepts
  exactly one authenticated Unix/Tailnet/mTLS peer per explicit call before CDP
  attachment, with a live packaged-Unix-listener proof and no hidden accept
  loop, task, retry, or deadline; the opt-in all-capability recipe now requires
  and statically chains every current command, control, Page-event outbound,
  topology, and PageLoad backend plugin, while leaving cleanup draining and
  Page-close polling to explicit caller policy; the optional Chromium utility
  profile creates a Page-unique isolated world in existing and future frames,
  routes Page/main/local/OOPIF locator internals through exact Frame sessions
  there, and leaves user evaluation in the main world
- [Browser Close](knowledge/pwright-browser-close.md) — Browser close,
  disconnect, provider ownership, process authority boundaries, and the
  implemented optional owned-launch and session-authorized remote-close
  checkpoint with whole-tree Dispose-before-Result ordering, bounded close
  reasons, plus negotiated queue-free disconnected ordering shared by owned
  close and native CDP loss, and bounded root-shared client reason persistence
- [Generated Browser-Lifecycle Protocol Reference](../crates/pwright-cap-browser-lifecycle/schema/protocol-reference.md) —
  Schema-derived owned Browser.close command and typed authorization, shutdown,
  and cleanup failures
- [Page Close](knowledge/pwright-page-close.md) — Ordinary versus before-unload
  lifecycle semantics and cleanup ordering
- [Generated Context-Creation Protocol Reference](../crates/pwright-cap-context-creation/schema/protocol-reference.md) —
  Schema-derived Browser.newContext initializer and failure variants
- [Generated Page-Creation Protocol Reference](../crates/pwright-cap-page-creation/schema/protocol-reference.md) —
  Schema-derived BrowserContext.newPage Page/main-Frame initializer and failures
- [Generated Navigation Protocol Reference](../crates/pwright-cap-navigation/schema/protocol-reference.md) —
  Schema-derived commands, models, wire variants, and constraints
- [Generated Locator Protocol Reference](../crates/pwright-cap-locator/schema/protocol-reference.md) —
  Schema-derived Click/Count/Fill/state/visibility/wait/raw-text/rendered-text/markup commands and recursive selector
  model including
  Playwright-style string/regex text, role, user-facing attributes, test IDs,
  has, and visible filters, wire variants, and constraints
- [Generated Selector Library Reference](../crates/pwright-codec-selector/schema/protocol-reference.md) —
  Command-free recursive selector wire types shared by Locator-derived capabilities
- [Generated Locator Screenshot Protocol Reference](../crates/pwright-cap-locator-screenshot/schema/protocol-reference.md) —
  Independent Frame/Page-scoped selector capture command, options, errors, and attachment metadata
- [Locator SelectOption](knowledge/pwright-locator-select-option.md) —
  Locator 1.21/2.4 string/descriptor selection semantics, bounded wire and
  renderer results, Chromium retry/commit behavior, and the explicit deferred
  ElementHandle adoption boundary
- [Locator ScrollIntoViewIfNeeded](knowledge/pwright-locator-scroll-into-view.md) —
  Locator 1.22/2.5 strict stable-only scrolling, full-intersection no-op,
  retry/commit semantics, resource bounds, and Browser.version initializer boundary
- [Locator Screenshot Composition](knowledge/pwright-locator-screenshot.md) —
  Implemented Playwright-aligned strict resolution, stable geometry, conditional
  scroll, bounded capture, independent local/remote profile, Rust cancellation,
  and dedicated-Chrome proof
- [Generated Content Protocol Reference](../crates/pwright-cap-content/schema/protocol-reference.md) —
  Schema-derived Page screenshot options, binary attachment metadata, and
  typed failure variants
- [Generated DOM-Content Protocol Reference](../crates/pwright-cap-dom-content/schema/protocol-reference.md) —
  Schema-derived Frame-only read request, UTF-8 attachment metadata, and typed
  navigation-race/resource failures
- [Generated Control Protocol Reference](../crates/pwright-codec-control/schema/protocol-reference.md) —
  Schema-derived lifecycle, cancellation, and subscription wire variants
- [Generated Handshake Protocol Reference](../crates/pwright-codec-handshake/schema/protocol-reference.md) —
  Schema-derived ClientHello/ServerHello variants and split-count segments
- [Improvements](knowledge/pwright-improvements.md) — Proposed improvements (P0-P3, prioritized)
- [Recipe Catalog](knowledge/recipe-catalog.md) — Full proposed recipe list (EN + CN sites)

## Reference

- [Playwright API Mapping](knowledge/playwright-api.md) — Full Playwright API compatibility matrix
- [API Parity](knowledge/api-parity.md) — gRPC vs CLI vs Playwright bridge coverage gaps
- [Feature Comparison](knowledge/feature-comparison.md) — Playwright vs PinchTab vs pwright side-by-side
- [Testing Strategy](knowledge/testing-strategy.md) — FakeCdpClient + Docker integration tests
- [Known Issues](known-issues.md) — Bug tracker and fixed issues
- [Bug Reports](bugs/index.md) — Detailed investigation reports for significant bugs
- [Code Quality Findings](code-quality-findings.md) — Codebase audit results
- [TODO](todo.md) — Planned features and roadmap
