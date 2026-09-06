<!-- agent-updated: 2026-09-03T09:34:20Z -->

# Playwright-aligned BrowserContext creation design

> Status: first no-options Chromium slice implemented and verified; context
> options and non-Chromium backends remain future design work.

## Upstream behavior

At Playwright commit `de214f440b7e34937fe4886f046b78b757136087`,
`Browser.newContext(options)` is a no-action-timeout channel command that
returns a newly created BrowserContext channel object. The server validates all
options, creates the backend context, applies initialization such as storage
state, emits the Browser context event, and closes the partially created
context if later initialization fails. Chromium creates an incognito context
with `Target.createBrowserContext` and `disposeOnDetach: true`.

Relevant anchors are:

- `packages/playwright-core/src/client/browser.ts` (`newContext`,
  `_innerNewContext`);
- `packages/playwright-core/src/server/browser.ts` (`newContext` and failure
  cleanup);
- `packages/playwright-core/src/server/chromium/crBrowser.ts`
  (`doCreateNewContext`);
- `packages/protocol/spec/browser.yml` (`Browser.newContext`).

## First capability slice

Pwright will introduce an optional `pwright.context_creation` capability with
one Browser-only `browser.new_context` command. Version 1.0 has no options and
therefore claims only the empty incognito-context subset. It does not pretend
to implement viewport, proxy, credentials, permissions, locale, storage state,
downloads, video, HAR, tracing, or client certificates. Those are later
additive fields or independent capabilities after their memory and cleanup
contracts exist.

Context creation depends on `pwright.context_lifecycle` at registry and
composition time. A profile may install close alone for already-existing or
externally discovered contexts, but it may not install creation without an
explicit close path. The client extension returns a typed BrowserContext, not
a native CDP ID. CDP strings remain inside the Chromium topology.

## Four-owner transaction

Creation spans four owners and one compensation path:

```text
allocate non-reusable portable ObjectId
  -> prepare runtime BrowserContext creation and Create control capacity
  -> Target.createBrowserContext(disposeOnDetach = true)
  -> prepare/register exact native route in production topology
  -> while holding the topology publication lock, commit runtime Create
  -> commit the already-prepared topology candidate
  -> remote: publish Create before Result
  -> client: claim announced initializer as typed BrowserContext
```

The runtime preparation leaves the object absent while reserving one
`LiveObject`, one optional `EventItem`, graph capacity, and the unique lifecycle
transaction gate. Drop rolls it back. Commit revalidates the unchanged parent
and inserts the object plus Create control without further allocation.

The portable ID source is monotonic, connection-local, starts after the Browser
root, never returns zero, never wraps, and does not reuse IDs after failed
native creation. Burning an ID is safer than allowing a late frame from a
failed attempt to address a later object.

If CDP creation succeeds but topology publication cannot complete, the exact
new native context must be disposed before the command may return. The
compensation future is admitted by the same synchronously reserved,
caller-owned continuation rule as close operations. A failed compensation is
a cleanup failure carrying both the primary publication error and cleanup
diagnostic; it must not leak the native ID into the portable API.

## Remote creation ordering and client identity

The existing ordered control writer needs a creation fence parallel to its
disposal fence. The typed server handler may encode Result only after the exact
`ControlMessage::Create { object_id, parent: Browser, kind: BrowserContext }`
has been accepted by the transport.

Client lifecycle state must distinguish two phases:

1. **announced** — Create has reserved a bounded object-table entry containing
   exact ID, parent, kind, and live token;
2. **claimed** — the command result has constructed the typed handle from that
   same entry.

An exact announced initializer may be claimed once. A mismatched parent/kind,
duplicate announcement, second claim, capacity overflow, Result without prior
announcement in remote mode, or Result for a disposed announcement is a
protocol failure. Direct in-process clients may claim the result initializer
without an earlier wire announcement. Dispose invalidates either phase. A lost
Result can therefore retain only one already-budgeted announced entry until
Dispose or connection teardown, never an unbounded orphan list.

## Cancellation and completion

Like Playwright, the public first-slice command carries no action timeout or
cancellation metadata. User cancellation by dropping the future is still a
Rust ownership event. Before CDP starts, Drop releases preparations. Once CDP
may have created the context, the exact remaining transaction transfers to a
reserved continuation so it either publishes the context or disposes the
native context; no hidden task is selected.

Successful local completion means topology and runtime already contain the
context. Successful remote completion additionally means the client has
received the Create frame before Result and can claim the typed handle.

## Implemented checkpoint

The first slice is split so every layer remains optional:

- `pwright-cap-context-creation` owns the core-only 1.0 contract and declares
  its context-lifecycle dependency;
- `pwright-codec-context-creation` owns the bounded binary wire mapping;
- `pwright-client-context-creation` adds only `Browser::new_context`;
- `pwright-server-context-creation` is the generated typed handler;
- `pwright-backend-cdp` owns the exact no-options CDP adapter; and
- `pwright-composition-chromium` opts those pieces into one attached Chromium
  root and supplies the runtime/topology/compensation transaction.

`pwright-core` now provides a connection-local `AtomicObjectIdSource`, and
`pwright-runtime` provides rollback-on-drop `PreparedObjectCreation`. Remote
composition uses an exact creation fence in `pwright-runtime-control`. The
client lifecycle table has distinct announced, claimed, and
disposed-before-claim phases; constructing its lifecycle handler permanently
enables the remote rule that a command Result cannot synthesize a missing
announcement. Direct in-process composition remains able to materialize the
same typed initializer without wire lifecycle traffic.

AAA tests cover schema identity, codec bounds and error round trips, monotonic
ID exhaustion, runtime commit/rollback/transport closure, CDP error rollback,
topology-capacity compensation, continuation admission, and the physical
`Create -> Result` stream order. The dedicated Chrome 152 test
`live_context_creation` creates a real isolated context, opens a target inside
it, closes it through the paired portable lifecycle service, proves the target
and both local identities are gone, and verifies the externally owned browser
still answers `Browser.getVersion`.

## Acceptance gates

- strict schema/codegen proves Browser-only targeting and the dependency on
  context lifecycle;
- prepared runtime creation is allocation-safe, rollback-on-drop, and excludes
  competing lifecycle mutation;
- object IDs are nonzero, monotonic, non-reused, and non-wrapping;
- client announced/claimed state is bounded and validates exact identity;
- CDP failure leaves no local object; post-CDP publication failure invokes
  exact-context compensation;
- remote Create physically precedes Result and the typed client returns one
  live BrowserContext;
- a saturated continuation rejects before browser interaction;
- dedicated Chrome proves context creation, typed close, target absence, and
  external browser survival;
- omitting context creation adds no schema, codec, client extension, handler,
  backend feature, allocation, task, Node.js, or browser bytes.
