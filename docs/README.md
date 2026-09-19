<!-- agent-updated: 2026-09-09T14:38:00Z -->

# pwright Documentation

NODE-092: [Child disappears between resume and detach](knowledge/pwright-chromium-topology-events.md#node-092-child-disappears-between-resume-and-detach)
reproduces the native -32602 connection failure and records the Rust cleanup fix
with dead/live Worker lifecycle controls. Full compatibility remains unfinished.

NODE-091: [Native session closure during header fanout](knowledge/pwright-extra-http-headers.md#node-091-native-session-closure-during-header-fanout)
records the public mismatch, bounded Rust close watches and main/child negative
controls. Full Node compatibility remains unfinished.

NODE-090: [Missing-session retirement before publication](knowledge/pwright-chromium-topology-events.md#node-090-missing-session-retirement-before-page-publication)
reproduces the connection failure with real Chrome receipts and adds scoped Rust
retirement plus lifecycle-owner cleanup. Attachment tests pass 7/7 plus three
repeats, headers 35/35, CDP 78, composition 373 and host 269. Remaining attachment
and API compatibility work stays open.

Latest NODE-089 gates: 35/35 public header tests, three 4/4 native receipt-gate
repetitions, 269 Rust host tests and isolated native cleanup. Earlier missing-
session attachment failures remain open; the complete Node API goal is not done.

NODE-089: [Native receipt gates and Context sequencing](knowledge/pwright-extra-http-headers.md#node-089-native-receipt-gates-and-context-page-sequencing)
corrects Page-group ordering, later-Page value capture and native error messages.
Four controlled public comparisons pass; full compatibility remains open.

NODE-088: [Caller-polled native jobs and early rejection](knowledge/pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection)
adds bounded native fanout and retained compensation without a second driver.
Combined public tests pass 31/31; native timing/retirement proof remains open.

NODE-087: [Synchronous Rust preparation and frozen dispatch](knowledge/pwright-extra-http-headers.md#node-087-synchronous-rust-preparation-and-frozen-dispatch)
connects ordered header/navigation preparation to N-API; combined public tests
pass 30/30. Native concurrent fanout and early settlement remain separate work.

NODE-086: [Pending setter race and overlapping model](knowledge/pwright-extra-http-headers.md#node-086-pending-setter-race-and-bounded-overlapping-model)
records an unresolved public ordering bug and the verified Rust model prerequisite.
Earlier green sequential gates do not establish concurrent compatibility.

NODE-085: [Navigation referrer reconciliation](knowledge/pwright-extra-http-headers.md#node-085-navigation-referrer-reconciliation)
connects the Page-only Rust policy to public Page/Frame.goto, with native request
and pre-dispatch conflict comparisons.

NODE-084: [Context creation headers](knowledge/pwright-extra-http-headers.md#node-084-context-creation-headers)
adds selected Rust-owned initial headers and a 21-test public differential gate.
Earlier checkpoint notes below describe historical coverage.

NODE-083: [Deferred header syntax and failed-Page recovery](knowledge/pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure)
corrects Context storage/merge timing and keeps invalid Page initialization local.
The public Context creation header option remains unfinished.

NODE-082: [Experimental public Rust-backed header setters](knowledge/pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential)
now have a selected addon and a ten-test public/admission gate. Creation options,
referer reconciliation, failure timing and worker/OOPIF verification remain open.

NODE-081: [Selected native Rust header host](knowledge/pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation)
passes isolated live Page wire/retirement tests. Public Node setters, failure
timing, worker support and OOPIF header gates remain unfinished.

NODE-078: [Extra HTTP header policy and native installation design](knowledge/pwright-extra-http-headers.md).
The optional Rust model passes 25 crate tests and 484 internal comparisons;
public Node setters and native session fanout remain unfinished.

NODE-050: selected Node FFI passes 44/44 Mouse, 24/24 Keyboard and 10/10 drag
comparisons; the internal native Frame lifecycle gate passes 18/18. Default
artifact and browser ownership remain unchanged.
[Selection, evidence and remaining work](knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049: the optional Frame-to-Page input lifetime handoff fixes the prior
detach discrepancy; the expanded native local/OOPIF gate matches 12/12.
[Contract, evidence and remaining gates](knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048: [real Frame input and newly proven lifetime discrepancy](knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy).
Ten-case gate is failing at 8/10; explicit Frame-to-Page continuation is next.

NODE-047: [Locator keyboard and exact owning Page selection](knowledge/pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page)
passes ten native host/oracle scenarios. Public addon selection, real child/OOPIF
and native destruction/interruption gates remain open.

NODE-046: [optional Rust Keyboard Escape cancellation](knowledge/pwright-chromium-drag.md#node-046-optional-rust-keyboard-escape-interception)
matches five native oracle scenarios. Locator-owned Escape, Page destruction
and public addon drag selection remain pending.

NODE-045: [maximum click batches and rebuilt Node artifact](knowledge/pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission).
Host raw Mouse is 25/25; public Node is 39/44, with drag still not selected.

NODE-044: [opt-in HostSession drag integration](knowledge/pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration)
now owns Page/session dispatch and close cancellation. Addon selection and
remaining Escape/lifecycle/concurrency compatibility gates are still pending.

NODE-043 adds an opt-in host `drag-source`: fresh Page utility snapshots now
remain owned through movement results, retirement and quarantine. Exact world
destruction drains started listener requests without issuing new cleanup there.
Default/selected host closures are **234/235**; only the drag leaf is added.
HostSession Mouse/close/Escape selection and public differential checks remain
pending; the addon and public **36/41** baseline are unchanged.
[Production source and evidence](knowledge/pwright-chromium-drag.md#node-043-retained-production-page-drag-source).

NODE-042 adds immediate, budgeted snapshots of a Page's existing utility worlds,
including child Frames/OOPIFs without waiting for a main world. Utility-only
native destruction receipts retain snapshot admission and remain distinct from
route invalidation and owned-session retirement. Production drag-source/host
integration is still pending; no addon rebuild or public **36/41** change.
[Snapshot ownership and verification](knowledge/pwright-chromium-drag.md#node-042-existing-page-utility-world-snapshots).

NODE-041 implements the explicit `mouse-dispatch` feature and Page-owned cleanup
driver: **82** unit tests and **4** selected-Mouse native cases pass, alongside
the prior **10** native cases. Default/feature closures are **3/5** Pwright
packages. Host Frame-source/lifecycle and Keyboard Escape integration remain
pending; no addon rebuild or public **36/41** change.
[Selected dispatcher and evidence](knowledge/pwright-chromium-drag.md#node-041-selected-mouse-dispatcher-and-page-cleanup-driver).

NODE-040 adds Page-owned drag state and retained native enter/over/drop/cancel.
**70** unit tests and **10** native scenarios pass, including actual drop and
cancel with dragend checks. The optional leaf remains **3** Pwright packages;
Mouse dispatch/forClick/down suppression, Keyboard Escape and Node host selection
are still pending. The addon/public **36/41** evidence is unchanged.
[Page state and verification](knowledge/pwright-chromium-drag.md#node-040-page-drag-state-and-native-event-owners).

NODE-039 adds [composed intercepted movement](knowledge/pwright-chromium-drag.md#node-039-composed-intercepted-movement):
**57** unit tests and **9** native scenarios pass. Page drag state and a real
Mouse dispatcher/host selection are next; the addon is unchanged.

NODE-038 adds [retained drag capture lifecycle](knowledge/pwright-chromium-drag.md#node-038-retained-capture-lifecycle):
**44** unit tests and **6** native listener/capture cases pass. Full Mouse
provider/host composition is still pending; the addon/public result is unchanged.

NODE-037 adds [bounded typed drag-data ownership](knowledge/pwright-chromium-drag.md#node-037-admitted-intercepted-data):
**27** unit tests and the existing **6** native listener scenarios pass, with
NativeDrag using the new storage. Full provider/host integration remains pending;
the addon and public **36/41** result are unchanged.

The optional [Chromium drag listener leaf](knowledge/pwright-chromium-drag.md)
now passes **13** unit and **6** real-browser scenarios (NODE-036). Full drag
dispatcher/host integration is still pending; public compatibility remains red.

NODE-035 adds native/Page dispatcher selection and standalone button retirement;
default and selected passthrough browser lanes both pass **22/22**. Real drag and
host Page-scoped integration remain pending; no addon rebuild/public gate change.
[Current checkpoint](knowledge/pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

NODE-034 adds the optional Rust Mouse result/retirement dispatch seam; **10**
new deterministic tests pass. This is not a Chromium drag implementation or
rebuilt Node addon. [Selection, resource limits and evidence](knowledge/pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

NODE-033 expands public Mouse to **41** concurrent/serial cases: **33/41** before
Rust first-poll ordering, **36/41** after. Five left-held moves remain red because
the drag delegate is absent. Host **115 passed / 6 ignored**, including seven
new ordering/lifetime tests. [Current scope and evidence](knowledge/pwright-raw-mouse.md#same-turn-input-ordering-node-033).

Final ordering artifact: Mouse **36/41**, Worker/profile **3/3**, Keyboard **24/24**,
typing **30/38**, actions **24/27**, screenshot **12/12**, lifecycle **45/45**.
Failed typing/actions stop at attachment on both providers; NODE-013 remains open.

Previous Mouse addon regression: **26/26** public Mouse, **3/3** Mouse profile/Worker,
Keyboard **24/24**, typing **38/38**, actions **27/27**, screenshot **12/12**,
lifecycle **45/45**, reload **16/18**. Shared **53 match / 2 unsupported /
1 oracle-error / 1 setup-error**; the 5 s topology-attach failure and original
reload-close failures remain red. [Artifact evidence](knowledge/pwright-raw-mouse.md#same-artifact-regression).

Optional Mouse 1.0 now has six generated Page commands and independent
capability/client/codec/server packages, with **24** release tests and exact
**3/6/7/10** dependency proofs. The native starter factory now passes **13**
unit tests and **22/22** browser cases with an **8-package** normal closure.
The bounded Page route adds **18** tests and **22/22** full-wire browser matches
with a **16-package** closure. The optional receipt-aware runtime adapter adds
**15** tests and **22/22** controlled browser matches, with a **17-package** closure.
The safe Rust host now selects Mouse with bounded independent retirement workers:
**22/22** host-path matches plus a real close-during-delay probe. The new release
addon exposes all six public `page.mouse` methods: **26/26** paired cases and
**3/3** profile/owned-and-borrowed Worker tests pass. Final nested allocation
audit and artifact-wide regression remain open.
[Typed Mouse contract](knowledge/pwright-raw-mouse.md#optional-mouse-10-contracts).

[Raw Mouse scheduling checkpoint](knowledge/pwright-raw-mouse.md): **22/22**
native event/state cases match, including click/dblclick, delay and shared input.
Result and native retirement have separate Rust owners' boundaries. Node
`page.mouse` is now wired; concurrent error ordering, HTML drag and the final
allocation audit remain open.

Current full Keyboard-artifact regression: input **38/38**, actions **27/27**,
screenshots **12/12**, lifecycle **45/45**, reload **16/18**. The two reload
close-boundary failures remain; see [evidence](knowledge/pwright-raw-keyboard.md).

Latest [Keyboard checkpoint](knowledge/pwright-raw-keyboard.md): all five public
`page.keyboard` methods run through Rust host/N-API and pass **24/24** paired
Chrome cases. Shared coverage advances to **52/3/2**, with no other errors.
Earlier primitive/adapter notes below are historical, not current API gaps.

[Raw Page Keyboard primitives](knowledge/pwright-raw-keyboard.md) now match
**14/14** native event/state cases. Keyboard 1.0 capability/client/codec/server
now pass **19** contract tests; its optional native backend adds **18** tests and
**14/14** typed Chrome comparisons. Its optional Page wire recipe adds **11** tests
and **14/14** wire-to-Chrome cases. Aggregate lifecycle routing and public Node
Keyboard remain open. Shared coverage has not changed.
The optional controlled adapter now adds seven route/lifecycle/deadline tests
(**18** composition tests total); default/runtime closures remain **16/17**.
The regenerated wire path passes a fresh **14/14** Chrome comparison.
Independent Page input selection and retained-state admission are now implemented
in the aggregate root and host source, with **294/89** unit tests and a real-host
attach/close resource gate passing. This is not a rebuilt Node addon result.

The new [Rust-backed Page.reload](knowledge/pwright-page-reload.md) uses navigation
1.1 through the controlled runtime and FFI. Both shared reload scenes now match:
the corpus is **48 match / 7 unsupported / 2 oracle-error**. The full reload lane
is **16/18**: response-arrival Page close fails, while a separately proven
post-commit close passes. The new artifact's lifecycle is **44/45** on window
geometry and screenshots are **0/12** at attachment; compatibility is incomplete.

The [sequential-typing Node binding](knowledge/pwright-sequential-typing.md)
exposes both `pressSequentially` and `type` over typed Rust commands. Node now
explicitly selects interleaved Page input; existing Rust callers keep their
default exclusive policy. The new artifact passes **30/30 typing + 27/27 action**
tests, including overlap, same-key repeat, waiting-target cancellation and the
NODE-022 Shift case. Historical red artifacts are retained; full compatibility
and broader native cancellation/Frame-boundary coverage remain open.

The same artifact's latest full lifecycle rerun passes **45/45 in 192.715 seconds**
after separating aggregate test supervision from an independent per-case bound;
no API timeout or Rust code changed. Earlier incomplete lifecycle failures and
the screenshot attachment failure/recheck pair remain evidence of intermittent
behavior, not erased by this green run.
[Full evidence](node-ffi-compatibility-design.md#node-013-lifecycle-supervision-evidence-2026-09-07).

The preceding opt-in recursive-attachment artifact passes screenshot 12/12,
lifecycle 45/45 and the serial/selected OOPIF cases 2/2. Its shared run is
45 match / 10 unsupported / 2 oracle-error, with no value/candidate/setup/cleanup
errors; overall compatibility remains incomplete. [Current artifact evidence](node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

The Node API additionally exposes Rust-backed `page.screenshot()` with a bounded
queue and Node-owned Buffer handoff. [Historical screenshot evidence](node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07)
retains the earlier attachment failures separately from the current green gate.

The experimental Node API now exposes Rust-backed Locator.evaluate with delayed
Node conversion. Owned-session retirement now passes the 23-case Worker/API gate,
including borrowed Page/Promise survival. Overall compatibility remains incomplete;
see [current FFI status](node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07).

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
- [Playwright-compatible Node FFI](node-ffi-compatibility-design.md) — Node-API
  boundary, compatibility levels, lifecycle/resource ownership, and
  differential verification plan and current discrepancy register
- [Experimental Node package](../packages/pwright/README.md) — Local release
  build, attach/navigation/evaluation, Locator query/read/action and Context/Page lifecycle subset,
  bounded query/binary ownership, Worker cleanup and real-browser comparisons
- [Shared API comparison](../tests/README.md) — Unchanged 19-suite corpus on
  owned oracle/candidate Contexts, explicit unsupported/error results, and
  independently recorded history/BFCache oracle diagnostics
- [Locator evaluation](knowledge/pwright-locator-evaluation.md) — Pinned timeout
  and conversion timing, strict element waiting, main-world adoption/single-use
  Rust evaluation, paired selection/adoption, route ownership through group
  cleanup, production world access, single-command host continuation and the
  optional native-destruction receipts / pending Node conversion gates
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
- [CDP Event Admission](knowledge/pwright-cdp-event-admission.md) — Explicit
  stateless pre-admission policy for selected consumers, conservative Chromium
  Frame-world classification, unchanged resource limits and unfiltered defaults
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
