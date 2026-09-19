<!-- agent-updated: 2026-09-07T01:45:30Z -->

# Playwright Page content semantics and Pwright boundary

## Optional Node execution-world precondition

Node's selected Chromium readiness component waits for the current utility
world before dispatching Page/Frame title and content. A temporarily missing
world on a live Frame is no longer classified as a missing installed
capability. Absent capabilities still reject; default Rust profiles retain
immediate call-construction route snapshots without waiting. The DOM leaf
itself and its standalone dependency closure do not gain evaluation or a driver.

The wait lives inside existing runtime admission/cancellation. Public
title/content still have no new action timeout; this change does not replace
their no-timeout contract with a private deadline. Close and transport loss
can interrupt it. See [NODE-021](../node-ffi-compatibility-design.md#node-021-bounded-execution-world-readiness)
for bounded accounting and post-navigation paired tests. Later document
replacement after dispatch is a separate boundary, not automatically retried.

## Decision

Pwright will expose Playwright-shaped `Frame::content()` and
`Page::content()` APIs, but only `Frame.content` is a protocol command. The
Page method resolves the Page's initialized main Frame locally and dispatches
the same Frame command. Child and OOPIF Frames remain independently routable.

The command belongs to a new opt-in `pwright.dom_content` 1.0 capability. It
does not depend on or install either `pwright.evaluation` or
`pwright.content` (the existing Page screenshot capability). This is an
intentional least-capability boundary:

- retrieving serialized DOM must not grant arbitrary JavaScript evaluation;
- retrieving HTML must not reserve screenshot memory or install rendering
  capture machinery;
- screenshot users must not acquire DOM-read access;
- a later `setContent` mutation will require a separate design and must not be
  smuggled into this read-only capability as a compatible minor.

The provisional command ID is `0x0006_0001`, and its only target object kind is
`pwright.browser.frame`. The request has no options. The public client call has
no action timeout metadata by default, matching Playwright's `kNoTimeout`
dispatch, while connection teardown and generic runtime cancellation still
terminate ownership of the operation.

## Upstream Playwright evidence

This design was checked against upstream Playwright commit
`de214f440b7e34937fe4886f046b78b757136087`:

- `packages/playwright-core/src/client/page.ts:410` delegates
  `Page.content()` to `_mainFrame.content()`;
- `packages/playwright-core/src/client/frame.ts:273` sends an empty Frame
  `content` request with `kNoTimeout` and returns its `value` string;
- `packages/playwright-core/src/server/dispatchers/frameDispatcher.ts:121`
  routes the protocol request to the semantic Frame;
- `packages/playwright-core/src/server/frames.ts:943` obtains that Frame's
  utility execution context and serializes the doctype with `XMLSerializer`
  followed by `document.documentElement.outerHTML`;
- `tests/page/page-set-content.spec.ts:20` verifies normalized full-document
  HTML, HTML5 and HTML4 doctypes, navigation races, and an empty document for
  a source-less iframe.

Therefore `content()` is neither a screenshot nor an HTTP response body nor a
snapshot protocol. It is a live DOM serialization in one exact Frame execution
world.

## Compatibility semantics

The successful value is exactly:

```text
(serialized document.doctype, when present)
+
(document.documentElement.outerHTML, when present)
```

No separator or synthetic newline is inserted. Browser normalization is
observable: for example, setting `<div>hello</div>` normally returns a complete
`<html><head>...</head><body>...</body></html>` document.

The operation is a single observation, not an auto-waiting action. During a
navigation it may observe the old document, observe the new document, or fail
with the stable "content changed during navigation" category. It must not
silently retry and combine two documents. Detached Frame, closed target,
browser crash, cancellation, JavaScript exception, and malformed backend
result retain their more specific typed failures.

`Page::content()` fails locally with a protocol-invariant error if the Page has
no initialized main Frame. It never invents a Page-scoped wire command.

## Chromium implementation boundary

The Chromium adapter evaluates a fixed library-owned serializer in the exact
Frame's Pwright utility world. No caller-provided source crosses this boundary,
so selecting DOM content does not grant the public evaluation capability.

DOM-content installation requires the internal Chromium utility-world plugin.
Existing and future main, local-child, and OOPIF Frame routes use the same
topology identities already materialized for the public object graph. The
composition root must share a utility execution-route record rather than
retaining a second copy of target, session, and unique-context strings for
each capability. A Page facade resolves to its main Frame before routing; a
direct Frame call uses that Frame's own route.

Context destruction caused by navigation is mapped once after the CDP command
fails. There is no backend retry loop. A newly established utility world is
used only by a later caller operation.

Firefox, WebKit, and future BiDi adapters implement the same portable Frame
command using their native isolated-world mechanism. CDP messages, execution
context IDs, and session IDs never enter the capability or client crates.

## Memory and wire ownership

DOM serialization is potentially multi-megabyte text and therefore receives a
dedicated `DomContentBytes` resource class. It must not be charged to
`ResponseBodyBytes`, because it is not a network response body, or to
`BackendStateBytes`, because the returned value outlives backend execution.

The portable result is a non-cloneable budgeted UTF-8 value. It exposes
`as_str`, `AsRef<str>`, length, emptiness, and `Display`, but no unaccounted
`into String` escape. Its resource lease lives exactly as long as the retained
text.

The production Chromium profile declares:

- a maximum UTF-8 byte length per result;
- at most one active content read per Frame;
- a configured connection-wide active-read limit;
- bounded shared execution-route storage;
- no admission waiter queue.

Before issuing the native evaluation, the adapter reserves the maximum
`DomContentBytes` result budget. It takes ownership of the returned JSON string
without another text copy, validates the configured byte ceiling, then shrinks
the lease to the exact retained UTF-8 length. Dropping the pending future or
returned value releases admission and bytes deterministically.

The remote protocol carries successful DOM text as one UTF-8 attachment plus a
small control record containing attachment index and exact byte length. This
keeps large data out of the generated control tree and permits reserve-before-
copy decoding. The source result, outbound `WireBytes` copy, inbound frame,
and client `DomContentBytes` result have distinct leases because their
lifetimes may overlap. The decoder requires exactly one attachment, validates
its index and declared length, verifies UTF-8 before publication, and rejects
oversized values. An error result carries no attachment.

## Required proof before compatibility is claimed

The first implementation slice must test:

1. Page dispatch uses only the main Frame object identity, while a child Frame
   dispatch uses its own identity.
2. Exact normalized HTML, no doctype, HTML5 doctype, HTML4 doctype, and a
   source-less iframe match Playwright behavior.
3. Main/local/OOPIF Frames use their exact utility execution contexts.
4. Navigation destruction produces old content, new content, or the stable
   typed race error, with no hidden retry.
5. Detached/closed/crashed Frames preserve the specific failure class.
6. Backend oversize, remote length/index/UTF-8 corruption, admission rejection,
   and dropped futures release all resource and concurrency state.
7. A negotiated server lacking `pwright.dom_content` rejects the method before
   backend work, while selecting DOM content does not advertise evaluation or
   screenshot.
8. A dedicated-Chrome live test captures doctype plus DOM from main and child
   Frames, disposes only its owned BrowserContext, and proves the externally
   owned browser remains usable.

The portable 1.0 schema/baseline, generated capability and server binding,
bounded UTF-8 attachment codec, Page/Frame client extensions, dedicated
resource class, Chromium serializer, shared locator/DOM utility route, direct
dispatcher, negotiated remote handler, and canonical all-capabilities recipe
are implemented. A dedicated Chrome 152 proof covers exact HTML5 and HTML4
doctypes, no doctype, independent `srcdoc` child DOM, the normalized empty
document of an iframe without a valid source, exact-target cleanup, and survival
of the externally owned browser. The same proof commits a cross-document
navigation: the stale utility context produces the stable navigation-race
category without retry, topology replaces that context under the same public
Frame identity, and the next call reads the new document. Fake-CDP tests retain
target-closed classification and release admission; synchronized crash state
rejects a later call as `BrowserCrashed` before CDP. Corrupt remote attachment
index, length, count, size, UTF-8, and caller-budget cases all fail without
publishing or retaining a result.

The controlled runtime now also races every admitted or queued command against
a generic typed subtree interruption. A child-Frame detach disposes only that
Frame subtree and terminates an already-running `content()` as `TargetClosed`.
A `PageCrashed` topology effect leaves Page/Frame identities live, interrupts
all calls in only that Page subtree as `BrowserCrashed`, drops the pending CDP
future, and makes later calls fail before CDP. This mechanism is runtime-level,
so it is reusable by every capability rather than special-cased inside DOM
content. A call that already accepted caller cancellation or a deadline keeps
that first terminal reason.

Remote application installation checks the accepted capability set before
constructing the DOM-content handler. An empty client capability request yields
no DOM-content handler, claims no handler resources, and has an explicit fake
CDP assertion that the backend call count remains zero. This has since become a
composition-wide invariant: every remote command handler and event lane checks
its owning accepted capability before codec, service, queue, or resource setup.
A control-only handshake traverses all current application installers and
leaves the resource ledger unchanged. Together with the selected-capability
transfer test, this closes the initial compatibility proof matrix above.

The compile-time boundary is now executable as well. The Chromium adapter was
extracted into `pwright-backend-cdp-dom-content`, while
`pwright-backend-cdp` compatibility-reexports the same types for aggregate
users. The isolated `pwright-composition-chromium-dom-content` leaf binds one
Frame and its exact bounded utility-world `uniqueContextId`, claims the full
route plus source-result, decoded-result, and wire peaks from one caller-owned
production ledger, and initially materialized the generated `Frame.content`
handler (1.1 also binds `Frame.title`). `examples/v2-dom-content-only` and its CI dependency allowlist prove
an exact 15-package Pwright closure with no public evaluation, screenshot,
navigation, aggregate backend/composition, client, provider, installer, or
runtime choice. Shared CDP failure classification lives in the command-free
`pwright-backend-cdp-common` leaf, so reuse does not merge capabilities.

## DOM-read 1.1: isolated Frame title

The Node FFI differential oracle (Playwright 1.58.2, installed
`playwright-core/lib/server/frames.js` `Frame.title`) reads `document.title` in
the utility world. Its prototype-interference fixture demonstrated why main-
world value evaluation is not an equivalent implementation.

Add `FrameTitle` (`frame.title`, method `0x0006_0002`, root wire tags 3/4) to
the existing read-only DOM capability as version 1.1. The preserved 1.0 baseline
and existing content tags/fields remain unchanged. `TitleParams` is empty and
does not accept arbitrary source. This adds a narrower projection of DOM text
already accessible to this capability, not a mutation or evaluation privilege.
Separate `FrameTitleExt`/`PageTitleExt` traits keep existing content-only client
channel bounds source-compatible. Page dispatch resolves its main Frame; both
reads carry no action deadline, while connection/cancellation budgets remain.

The Chromium implementation shares the exact utility execution route, finite
read/concurrency admission, and `DomContentBytes` result ownership with content.
Its only title source is the private fixed `document.title` expression; it does
not serialize the whole document to extract a title. The title result uses the
same bounded UTF-8 attachment codec with distinct command/result tags. It cannot
be decoded as a content result or vice versa. Native teardown and late-result
handling are the same as other fixed DOM reads.

The standalone DOM-content composition exposes both typed read handlers without
adding an evaluation, browser, provider, or runtime dependency. The Node host no
longer installs its former title-only evaluation profile. Its aggregate Rust
dependency graph is still broad; this does not claim the aggregate host is a
minimal distribution. Node conversion-result budgets remain a separate gate.

Required verification for 1.1: preserved 1.0 schema compatibility, title command
and attachment round trips with cross-result rejection, exact utility context
and fixed-source checks, byte-budget release/oversize rejection, client main-
Frame dispatch, shared route replacement/detach/crash behavior, standalone
dependency proof, and actual Playwright-paired prototype/concurrent-read cases.

Those title-specific unit, generated-schema and negotiated-server tests passed;
the existing shared-route/cancellation suite also passed. The real Chrome 152
Node lane now passes nineteen cases without a TODO, including prototype
interference and concurrent reads. Workspace release check, strict changed-
component Clippy and the unchanged fifteen-package DOM-read dependency proof
passed. The aggregate host remains a broad prototype, not a narrow distribution.
