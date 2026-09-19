<!-- agent-updated: 2026-09-10T06:55:30Z -->

# Playwright-aligned Page creation design

NODE-111: Existing synchronized initialization marks pre-registered Pages ready and now
exposes that native effect to selected observers. Deferred popup linkage can
arrive in an opener's later transaction; do not invent duplicate Page creation
or assume a transaction is already one dependent callback group.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 characterizes listener mutation, async gates and popup listener failure.
An async page listener does not delay newPage resolution; a synchronous Context
page failure suppresses its dependent popup notification, not the already
published Page membership. Candidate Page/popup notifications remain missing.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109's exclusive metadata drain reuses the queue pre-admitted before Page
creation, adding one active guard's allowance. It does not synthesize Page/popup
events from membership revisions or change Node creation publication. Native
cause ordering and atomic event-visible state remain required before dispatch.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-105 characterizes explicit creation and popup events. Context page precedes
newPage resolution; for a popup, Context page precedes opener popup with the same
Page argument and registered membership. Native candidate creation/discovery
works but these notifications are absent. Reuse existing typed Page-event and
opener effects through a bounded Rust language-publication port, not JS inference.
[Red evidence and proposed integration](pwright-node-lifecycle-events.md).

NODE-104 verifies explicit single/concurrent Page creation, replacement and
independent borrowed Page discovery while Node is blocked. Membership waits for
publication; returned/main-Frame identities and retained arrays remain coherent.
The seven-case lane passes twice on each NODE-103 profile. Concurrent creation
completion can reorder insertion relative to invocation; stable per-connection
enumeration is tested without imposing a stronger ordering contract.
This does not implement Context page events, popup/opener semantics or Browser.newPage.
[Current evidence and remaining event work](pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-088 enables the selected header Network domain at scoped attachment, not
per setter. The host keeps existing header jobs progressing while Page creation
or initialization awaits native capacity. Expected header-syntax failure retains
the exact-Page cleanup path; concurrent handoff/fault injection remains open.
[Initialization and backpressure](pwright-extra-http-headers.md#node-088-caller-polled-native-jobs-and-early-primary-rejection).

NODE-083 isolates expected header-syntax initialization rejection. The driver
finishes topology initialization without sealing shared observers, then closes
the exact newly created Page through the existing lifecycle service before
rejecting creation. Context remains usable. This follows upstream newPage's
failure cleanup, not merely its intermediate Page.reportAsNew event. Other
initializer failures and exact transient Page events remain unverified.
[Header lifecycle evidence](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

> Status: implemented and verified for the first explicit
> `BrowserContext.newPage()` slice, including Chrome 152. Popup discovery and
> additional browser backends remain separate work.

## Node FFI integration checkpoint

The experimental Node host now exposes no-options `BrowserContext.newPage()`
through this existing service. Its single topology driver serializes explicit
creation with native event ingestion, initializes the new route/world snapshot,
and replies with stable Page/main-Frame wrappers. It reserves the main Frame's
metadata entry and initial URL bytes before creating the native target; a
returned descriptor retains that exact snapshot even after external retirement.
This does not add `Browser.newPage()` or Context creation to the Node API.

The default Context uses an internal empty key. The production CDP client now
omits `browserContextId` for that key instead of sending the invalid empty ID;
explicit non-default IDs are preserved. Both encodings have release tests.
See [the Node checkpoint](../node-ffi-compatibility-design.md#explicit-page-creation-and-ordinary-close-checkpoint)
for differential evidence, FFI budgets and unfinished lifecycle gates.

## Upstream contract

At Playwright commit `de214f440b7e34937fe4886f046b78b757136087`,
`BrowserContext.newPage()` is a no-action-timeout channel command. The server
calls the backend page factory, waits for `Page.waitForInitializedOrError()`,
rejects a page that closed during initialization, and attempts to close a
partially created page after failure. Chromium sends `Target.createTarget` for
`about:blank` in the exact browser context; its target-attachment path creates
the Page and main Frame before initialization completes.

The Page protocol initializer contains a required `mainFrame`. Returning only
a target or Page identity would therefore be observably weaker than
Playwright. A popup is also a Page, but it arrives from browser events and may
have an opener; it is not the result of this explicit command.

Source anchors:

- `packages/protocol/spec/browserContext.yml` (`newPage`);
- `packages/protocol/spec/page.yml` (`Page.initializer.mainFrame`);
- `packages/playwright-core/src/client/browserContext.ts` (`newPage`);
- `packages/playwright-core/src/server/browserContext.ts` (`newPage` and
  failure cleanup);
- `packages/playwright-core/src/server/dispatchers/browserContextDispatcher.ts`;
- `packages/playwright-core/src/server/chromium/crBrowser.ts`
  (`doCreateNewPage` and target attachment); and
- `packages/playwright-core/src/server/page.ts` (`reportAsNew` and
  `waitForInitializedOrError`).

## First capability slice

Pwright will add an optional `pwright.page_creation` 1.0 capability containing
one BrowserContext-only `browser-context.new_page` command. It depends on
`pwright.page_lifecycle >= 1.0`, so every page it creates has an explicit close
path. It does not depend on navigation, locator, events, BrowserContext
creation, or a browser provider.

The first command has no parameters, timeout, or cancellation metadata and
always requests `about:blank`. Its result carries two portable initializers:

```text
Page { id, parent = BrowserContext, kind = Page }
MainFrame { id, parent = Page, kind = Frame }
```

The client returns one typed Page whose `main_frame()` is already available.
Neither target ID, session ID, browser-context ID, nor CDP Frame ID crosses the
portable boundary. `Browser.newPage()` convenience remains out of this slice
because its hidden context ownership requires a separate design.

## Chromium readiness boundary

The Chromium backend resolves the parent Context's borrowed native route, then
performs this bounded sequence:

```text
Target.createTarget(url = about:blank, browserContextId = exact parent)
  -> Target.attachToTarget(flatten = true)
  -> Page.getFrameTree on the returned target session
  -> validate a nonempty bounded target/session/root-frame identity
  -> return the compact native triple
```

The root of `Page.getFrameTree` must have no parent. Obtaining that root is the
first-slice initialization proof: the target has a usable flattened session
and an actual main Frame. Context policies, init scripts, viewport emulation,
downloads, service workers, execution worlds, and child-frame replay are not
claimed by this slice and remain owned by their future capabilities.

If any step after `Target.createTarget` fails, the exact target must be closed
and confirmed absent using the bounded ordinary Page-close policy. A cleanup
failure preserves both the primary initialization error and cleanup error.

## Three-owner publication transaction

Page creation publishes runtime identity, Chromium topology, and the Page
command route together:

```text
allocate two non-reusable ObjectIds
  -> prepare one runtime creation batch: Page then MainFrame
  -> create and initialize the native target/session/root frame
  -> lock topology and prepare attach_page + main-frame rebind
  -> prepare the Page route table from that topology candidate
  -> verify exact Page Create and Frame Create effects
  -> commit runtime batch while topology and routes remain locked
  -> commit the infallible topology and route candidates
  -> remote: send Page Create, Frame Create, then Result
  -> client: atomically claim both announcements and return initialized Page
```

The two ObjectIds are connection-local, monotonic, and burned on every failed
attempt. `PreparedObjectCreationBatch` validates the complete candidate graph
before reserving two `LiveObject` leases and two lifecycle `EventItem` entries.
It leaves both objects absent until commit, uses the existing exclusive
lifecycle transaction, and rolls back all reservations on Drop or transport
closure.

Route synchronization must be prepared before runtime commit. During commit,
the topology mutex and Page-route mutex exclude readers; runtime becomes live
first, after which both already-prepared local commits are infallible. A Page
command racing this short synchronous publication window blocks on the route
mutex rather than observing a live Page without a route.

Any failure after native target creation invokes exact-target close and
absence confirmation. The whole already-started future transfers to a
synchronously reserved, caller-owned continuation if the original caller
drops it. Exhausted continuation capacity rejects before ID allocation or CDP.

## Remote and client ordering

The ordered control barrier drains through the exact Page Create and then the
exact main-Frame Create. Only then may the generated handler send Result.

The client object table claims the Page and main Frame as one batch. In remote
mode both exact announcements must exist and still be live; direct in-process
mode both are inserted together. Wrong kind/parent, duplicate identity,
missing announcement, Dispose-before-Result, or capacity exhaustion changes
neither phase. This prevents a half-constructed Page if the second initializer
is invalid.

Popup and Context `page` events will reuse the same Page+main-Frame initializer
shape, but their event ordering, opener relationship, first-request routing,
and subscription backpressure belong to a later capability.

## Implemented acceptance evidence

- schema/codegen proves BrowserContext-only targeting and the page-lifecycle
  dependency;
- omitting the capability adds none of its codec, client, handler, backend, or
  resource requirements;
- runtime batch creation commits or rolls back both objects atomically;
- backend errors after target creation perform exact confirmed cleanup;
- topology and route-capacity failures leave no local object or native target;
- remote bytes are Page Create, Frame Create, then Result;
- client result claims both objects atomically and exposes the main Frame;
- a saturated continuation rejects before CDP;
- dedicated Chrome 152 proves Page and main Frame creation, a live Page route,
  `about:blank`, typed Page close, Context close, and external Browser survival;
  and
- the slice introduces no Node.js, browser launch/download, hidden task,
  executor, timer, or unbounded collection.

The concrete implementation is split across optional capability, codec,
client-extension, generated-server, CDP-backend, and Chromium-composition
crates. Runtime adds a general `PreparedObjectCreationBatch`; client state adds
an atomic related-initializer claim. The live proof is
`crates/pwright-composition-chromium/tests/live_page_creation.rs` and remains
ignored unless the caller explicitly supplies the dedicated CDP endpoint.
