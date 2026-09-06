<!-- agent-updated: 2026-09-05T05:12:38Z -->

# Playwright design and capability course

> Status: active learning notes. We study Playwright before deciding Pwright
> v2 architecture. Statements about Playwright are evidence, not automatic
> decisions for Pwright.

## Course map

- Lesson 1: Product layers and top-level object model
- Lesson 2: BrowserType, browser launch, connect, and browser distribution
- Lesson 3: Browser, BrowserContext, isolation, and persistent profiles
- Lesson 4: Page, Frame, popup, navigation, and lifecycle
- Lesson 5: Selectors, Locator, FrameLocator, and strictness
- Lesson 6: Actions, actionability, auto-waiting, and timeouts
- Lesson 7: Assertions and retry semantics
- Lesson 8: JavaScript evaluation, JSHandle, and ElementHandle
- Lesson 9: Network Request, Response, Route, WebSocket, HAR, and downloads
- Lesson 10: Authentication, cookies, permissions, storage, geolocation, and
  emulation
- Lesson 11: Events, dialogs, workers, bindings, and exposed functions
- Lesson 12: Screenshots, PDF, video, tracing, console, and artifacts
- Lesson 13: APIRequestContext and browser-independent HTTP
- Lesson 14: Test runner, fixtures, projects, retries, reporters, and parallelism
- Lesson 15: Channel protocol, generated schema, dispatchers, and transports
- Lesson 16: Chromium CDP, Firefox Juggler, WebKit protocol, and Electron/Android
- Lesson 17: Installation, driver processes, remote server, and language clients
- Lesson 18: Resource lifecycle, object retention, memory, and failure handling
- Lesson 19: Extension points and the limits of Playwright composability
- Lesson 20: Compatibility inventory and candidate lessons for Pwright

## Lesson format

Each lesson records:

- What users can do
- Public objects and their relationships
- The internal design logic
- Source locations
- Costs and limitations
- Questions for Pwright, without deciding them prematurely

## Recording status

- Lessons 1 through 20: full notes recorded and ordered.
- Learner answers and corrections are recorded under the relevant lesson when
  an answer was provided.
- The compact answer key below also preserves review answers that were supplied
  in conversation without a learner response.
- Every future lesson must be written here before it is presented as complete.

## Checkpoint answer key for Lessons 1-12

- Lesson 1: the Node driver/runtime performs Playwright auto-waiting for
  non-JavaScript clients; Playwright Test is not required to use the automation
  library; CDP is Chromium-specific and too low-level to be the portable API.
- Lesson 2: `chromium` is a BrowserType; `connect` targets a Playwright Server;
  `connectOverCDP` is Chromium-only; `launch` returns a Browser with temporary
  contexts, while `launchPersistentContext` returns a persistent Context.
- Lesson 3: Pages in one Context share session state; different Contexts do not
  share cookies by default; reusing a Browser avoids repeated process startup;
  Contexts should close first so pages and artifacts finish gracefully.
- Lesson 4: Page navigation delegates to the main Frame; Page identity usually
  survives document navigation; network idle does not prove application
  readiness; event waits must be armed before the triggering action.
- Lesson 5: Locator construction normally does not query the DOM; Locator can
  re-resolve after node replacement while ElementHandle binds one node; a
  single-target action with multiple matches fails strictness; a normal Locator
  does not automatically cross an iframe boundary.
- Lesson 6: an overlaid element is not normally clickable; stable positioning
  avoids stale coordinates; one timeout bounds the whole action; force does not
  prove the application processed the input.
- Lesson 7: asserting a previously read value does not re-read the DOM; a
  successful click does not prove a business result; a positive expected state
  is stronger than merely excluding one transient state; poll callbacks should
  be read-only because they can execute repeatedly.
- Lesson 8: page functions cannot capture ordinary client locals; `evaluate`
  returns a non-live copy; ElementHandle binds a concrete node; live handles
  can prevent browser GC and occupy client/driver object tables.
- Lesson 9: HTTP 500 is a Response rather than request-failed; an unresolved
  Route stalls; Response headers can precede body completion; WebSocket queues
  need bounds; download waits are armed before the triggering action.
- Lesson 10: Pages in one Context share cookies; storageState is not a complete
  profile; geolocation also needs permission; HTTP credentials do not fill an
  application login form; viewport alone is not phone emulation.
- Lesson 11: event waits are armed first; a user-handled Dialog must be accepted
  or dismissed; a Worker has no Page document; Service Workers affect network
  and cache behavior; exposing host file access to page code crosses a dangerous
  trust boundary.
- Lesson 12: screenshot pixels do not replace DOM structure; PDF uses print
  rendering by default; video/trace need orderly finalization; a remote runtime
  path is not a client-local path; retained console argument Handles can keep
  browser objects alive.

## Lesson 1: Product layers and top-level object model

### What Playwright is

Playwright is more than a browser command library. Its product is composed of
several separable layers:

```text
Playwright Test runner                         optional Node.js product layer
                 |
Language API: JavaScript / Python / Java / .NET
                 |
Playwright object and channel protocol
                 |
Playwright driver/runtime
                 |
Chromium / Firefox / WebKit backend protocols
                 |
Browser process tree
```

A user may use the automation library without Playwright Test. Non-JavaScript
language packages act as clients of the Playwright driver. The JavaScript API
normally uses the same client/runtime separation in one Node.js environment.

### Main automation object hierarchy

```text
Playwright
  |-- chromium: BrowserType
  |-- firefox: BrowserType
  |-- webkit: BrowserType
  |-- request: APIRequest
  |-- selectors: Selectors
  `-- devices and shared configuration data

BrowserType
  `-- launch/connect -> Browser

Browser
  `-- newContext -> BrowserContext

BrowserContext
  |-- newPage -> Page
  |-- pages
  |-- cookies, permissions, storage, routing, tracing
  `-- context-wide events and configuration

Page
  |-- mainFrame and child Frames
  |-- Locator and FrameLocator query values
  |-- Keyboard, Mouse, Touchscreen input helpers
  |-- Request, Response, Route, WebSocket network objects
  |-- Worker, Dialog, Download, Video, ConsoleMessage
  `-- navigation, content, evaluation, screenshots, PDF, events
```

The hierarchy separates browser-process ownership, isolated browser sessions,
tabs, document frames, queries, input devices, and transient events. A method
is placed on the narrowest object whose lifetime and scope it affects.

### Capability families

At the broadest level Playwright supports:

- Browser acquisition: launch local browsers, persistent profiles, connect to
  a Playwright server, or connect to Chromium over CDP.
- Isolation: multiple BrowserContexts with independent cookies, storage,
  permissions, routes, locale, timezone, proxy, and emulation settings.
- Page lifecycle: create pages, popups, navigate, reload, history, wait for load
  states, and observe crashes or closure.
- Element discovery: CSS and extended selectors, text, role, label, test ID,
  Locator composition, FrameLocator, filtering, and strict matching.
- Interaction: click, hover, fill, type, press, select, check, drag, upload,
  focus, mouse, keyboard, and touchscreen operations.
- Synchronization: actionability checks, automatic retrying, navigation waits,
  explicit event waits, deadlines, and cancellation through the driver.
- Inspection and execution: DOM state, attributes, text, accessibility-facing
  queries, JavaScript evaluation, JSHandle, ElementHandle, and workers.
- Network: request/response observation, routing, modification, abort,
  fulfillment, authentication, WebSockets, HAR, cookies, and downloads.
- Browser environment: viewport, device scale, user agent, locale, timezone,
  geolocation, permissions, color scheme, media, offline mode, and headers.
- Artifacts and diagnostics: screenshots, PDF on Chromium, video, tracing,
  console messages, errors, source metadata, and call logs.
- Direct HTTP: APIRequestContext for requests that may share cookies with a
  browser context but do not require a page.
- Higher-level testing: fixtures, projects, browser matrices, parallel workers,
  retries, assertions, reports, snapshots, and test traces.
- Special targets: Electron and Android support in addition to desktop browser
  automation, with platform and stability differences.

This is a capability inventory, not a claim that every feature works identically
on every browser or language. Later lessons establish those differences.

### Internal design logic introduced by this hierarchy

Playwright does not expose CDP as its primary abstraction. BrowserContext,
Page, Frame, and Locator describe portable automation semantics. The driver
translates those semantics to Chromium CDP, Firefox Juggler, or WebKit's
protocol. This lets the same public operation have backend-specific machinery.

Not every public value is a long-lived remote object. Locator is primarily a
description of how to find an element again when an action occurs, while Page,
Frame, Request, Response, Worker, and handles can carry identity across calls or
events. This distinction is central to Playwright's retry and lifecycle model.

### Source anchors

- Public Page API: `packages/playwright-core/types/types.d.ts`
- Public BrowserContext API: `packages/playwright-core/types/types.d.ts`
- Public Browser and BrowserType APIs: `packages/playwright-core/types/types.d.ts`
- Protocol domains: `packages/protocol/spec/*.yml`
- Client object implementations: `packages/playwright-core/src/client/`
- Server implementations: `packages/playwright-core/src/server/`
- Dispatch boundary: `packages/playwright-core/src/server/dispatchers/`

### Questions raised for Pwright

- Which compatibility target matters: names, behavior, tests, or wire protocol?
- Which top-level objects must exist in the first supported profile?
- Which objects need runtime identity, and which should remain cheap values?
- Is a test runner part of Pwright, a separate product, or a non-goal?
- Which browser and special-target families are actual requirements?
- Which capability families must be independently installable?

No answer is selected in this lesson.

## Lesson 2: BrowserType, browser acquisition, launch, and connection

### BrowserType is a factory, not a running browser

Playwright exposes `chromium`, `firefox`, and `webkit` as `BrowserType`
objects. A BrowserType describes one browser family and the supported ways to
obtain a Browser. Calling `launch` or a connection method produces the running
or connected `Browser` object.

```text
chromium: BrowserType -- launch/connect --> Browser
firefox:  BrowserType -- launch/connect --> Browser
webkit:   BrowserType -- launch/connect --> Browser
```

### Four acquisition modes

- `launch()` starts an ephemeral browser owned by Playwright and returns a
  `Browser`. The caller then creates one or more isolated BrowserContexts.
- `launchPersistentContext(userDataDir)` starts a browser around one persistent
  profile and returns that BrowserContext directly. Closing it closes its
  browser, and profile state may remain in `userDataDir`.
- `connect()` connects a client to a Playwright Server using the Playwright
  channel protocol. The remote runtime preserves full Playwright semantics and
  can manage Chromium, Firefox, or WebKit.
- `connectOverCDP()` attaches the local Playwright runtime directly to an
  existing Chromium CDP endpoint. It is Chromium-only and officially described
  as lower fidelity than a Playwright protocol connection.

```text
connect:
Client -> Playwright protocol -> Playwright Server -> browser protocol -> Browser

connectOverCDP:
Playwright runtime -> CDP -> existing Chromium
```

### Browser distribution

A Playwright release is associated with known browser builds so the driver can
rely on tested browser behavior and protocol support. Browser installation is a
separate operational concern and can consume significant disk and deployment
space. System browser channels and explicit executables are possible in some
configurations, but they do not replace Playwright's need for controlled builds
across Chromium, patched Firefox, and WebKit.

### Ownership distinction

`launch` and `launchPersistentContext` create a browser process that Playwright
owns. `connect` delegates browser ownership to the remote Playwright Server.
`connectOverCDP` attaches to an existing Chromium and must distinguish closing
the automation connection from terminating a browser that may belong to
another user or system.

### Source anchors

- Public interface: `packages/playwright-core/types/types.d.ts`, `BrowserType`
- Channel schema: `packages/protocol/spec/browserType.yml`
- Client implementation: `packages/playwright-core/src/client/browserType.ts`
- Server implementation: `packages/playwright-core/src/server/browserType.ts`

### Learner checkpoint

- Question 1 answer: initially “browser.” Correction: `chromium` is a
  `BrowserType`; `await chromium.launch()` returns the Browser.
- Question 2 answer: Playwright Server. Correct for `connect()`.
- Question 3 answer: only Chromium uses CDP. Correct; Firefox and WebKit use
  different Playwright backend protocols.
- Question 4: needs reinforcement. The key distinction is that `launch()`
  returns a Browser with temporary-profile contexts created afterward, while
  `launchPersistentContext(userDataDir)` immediately returns the single
  persistent BrowserContext whose stored browser data can survive restarts.

### Questions raised for Pwright

- Should browser acquisition be expressed as BrowserType, provider components,
  or both?
- How should Rust encode owned versus borrowed browser processes?
- Should attach, launch, remote runtime, and managed download be independently
  installable?
- Does Pwright need both its own remote protocol and direct CDP attachment?

No Pwright decision is selected in this lesson.

## Lesson 3: Browser, BrowserContext, isolation, and ownership

### Browser is the automation connection to one browser instance

The public `Browser` represents one launched or connected browser and records
its browser family, version, connection state, and BrowserContexts. It is not a
renderer, page, or profile. Its principal portable responsibility is creating
and owning contexts.

```text
Browser
  |-- BrowserContext A
  |     |-- Page A1
  |     `-- Page A2
  |-- BrowserContext B
  |     `-- Page B1
  `-- BrowserContext C
```

Browser-level CDP sessions and tracing exist as Chromium-oriented or diagnostic
escape hatches, not as the main portable automation model.

### BrowserContext is an isolated browser session

Each non-persistent context behaves like a fresh incognito-style profile. Its
pages share context state with one another while remaining isolated from pages
in other contexts. Context-scoped state and policy include cookies, origin
storage, permissions, geolocation, locale, timezone, proxy configuration,
extra HTTP headers, HTTP credentials, offline mode, initialization scripts,
request routes, tracing, HAR, and video configuration.

```text
Context A pages share A's cookies and policy
Context B pages share B's cookies and policy
A and B do not share those browser-session states
```

This is browser-level logical isolation, not a claim that every context owns a
separate operating-system process or forms a hostile-code security boundary.
The browser may still share process infrastructure, caches, GPU, network
services, and renderer processes according to its own architecture.

### Why contexts exist

Starting a complete browser for every independent task is expensive. Contexts
provide cheap isolation within one browser process tree. Playwright Test can
therefore reuse a worker's Browser while creating a fresh BrowserContext for
each test. This reduces startup cost without intentionally sharing cookies or
storage between tests.

### Scope determines method placement

- `Browser.newContext()` creates a new isolated session.
- `BrowserContext.newPage()` creates a tab inside that session.
- `BrowserContext.route()` applies interception across all pages in the
  context; `Page.route()` has page scope.
- Context events can observe pages, requests, responses, downloads, dialogs,
  workers, and errors across the session.
- `BrowserContext.storageState()` exports reusable cookies and origin storage;
  a later context can be initialized from that state without reusing the whole
  persistent profile.

Playwright also exposes `Browser.newPage()` as a short-script convenience. It
implicitly creates a context, which hides ownership; production code and test
frameworks are advised to create the context and page explicitly.

### Lifecycle order matters

The graceful shutdown order is child scope before parent scope:

```text
finish operations
  -> close BrowserContext
       -> close its pages
       -> flush HAR/video/artifacts
  -> close Browser
       -> end the browser connection/process as ownership permits
```

Calling `Browser.close()` first is closer to force-quitting the whole browser.
Explicitly closing contexts first gives them a chance to finish context-owned
artifacts and emit orderly lifecycle events.

### Persistent context versus storage state

A persistent context binds to a full user-data directory. A normal ephemeral
context can instead import and export selected automation state with
`storageState`. Storage state is lighter and more portable, while a persistent
profile preserves a broader set of browser data and browser-specific behavior.
They are related but not equivalent.

### Source anchors

- Public Browser API: `packages/playwright-core/types/types.d.ts`, `Browser`
- Public context API: `packages/playwright-core/types/types.d.ts`,
  `BrowserContext`
- Browser protocol schema: `packages/protocol/spec/browser.yml`
- Context protocol schema: `packages/protocol/spec/browserContext.yml`
- Client implementations: `packages/playwright-core/src/client/browser.ts` and
  `browserContext.ts`
- Server implementations: `packages/playwright-core/src/server/browser.ts` and
  `browserContext.ts`

### Questions raised for Pwright

- Is Browser primarily a connection, an owned process, or a typed combination
  that makes ownership explicit?
- What exact state must Pwright isolate per BrowserContext on a CDP backend?
- Can capability-specific context state remain absent when that capability is
  not installed?
- Which cleanup failures should make context closure fail, warn, or remain
  recoverable?
- How should memory and concurrency budgets divide across Browser, Context,
  Page, and backend session?

No Pwright decision is selected in this lesson.

### Later Pwright checkpoint: connection-scoped process authority

The implemented owned-launch leaf preserves this lesson's ownership
distinction structurally. Negotiating `browser.close` is necessary but not
sufficient to terminate an owned process: the accepted server handshake must
come from a production supervisor sharing the launched composition's exact
connection resource ledger. Therefore another session with the same protocol
version, capability set, limits, plan, and numeric Browser ID cannot borrow the
process lease. Unbound and foreign-session rejection return the still-live
owner; only the owning connection can consume it. This resolves the initial
multi-session policy without a global session table or random identity token.

An abandoned owned close also retains structural authority. Before browser work
starts, the selected continuation executor must reserve bounded capacity. The
standard-library adapter leaves polling to its caller; the optional Tokio
adapter can execute the transferred future immediately and apply an explicit
job deadline. Deadline expiration drops that exact future, activating the
owned provider's kill-on-drop escalation. It is not modeled as a client
`Browser.close` timeout, because Playwright-shaped close remains terminal and
cannot safely return process authority for retry.

Provider diagnostics form a separate composition concern. Public close success
is still unit-valued, while an explicitly selected launch-side sink may receive
the concrete shutdown report after lifecycle disposal. This lets operators
observe graceful versus forced termination without making Chromium process
details part of the Playwright-compatible Browser contract or requiring every
client to install that diagnostic feature.

## Lesson 4: Page, Frame, popup, navigation, and lifecycle

### Page is a tab-like browsing surface

A Page usually corresponds to one tab or popup within a BrowserContext. It owns
page-scoped input devices, viewport and media behavior, screenshots, dialogs,
downloads, workers, console events, and a tree of Frames. It is “tab-like”
rather than defined as a particular Chromium target because Playwright keeps
the public model portable across browser backends.

### Frame owns document-scoped operations

Every Page has one main Frame and may have child Frames for iframes. Navigation,
document URL, content, selector resolution, DOM actions, and JavaScript
evaluation are fundamentally Frame-scoped. Many convenient Page methods simply
delegate to the current main Frame.

```text
Page
  `-- Main Frame
        |-- Child Frame A
        |     `-- Child Frame A1
        `-- Child Frame B
```

The Page may survive many navigations while its documents and execution
contexts are replaced. The main Frame retains its logical identity across
normal top-level navigations; document-bound handles may become stale.

### Navigation is a state machine, not one command

`goto` starts navigation and then waits for a selected lifecycle milestone.
Playwright recognizes `commit`, `domcontentloaded`, `load`, and `networkidle`.
The default for navigation methods is normally `load`.

```text
navigation requested
  -> response/redirect chain
  -> commit: browser accepts a new document
  -> domcontentloaded: DOMContentLoaded fires
  -> load: load fires
  -> later network activity may become idle
```

Same-document navigation, such as a history-state or fragment change, does not
create a new document and may return no main-resource Response. Failed requests,
downloads, redirects, frame detachment, page closure, and browser crashes are
distinct navigation outcomes.

`networkidle` is not a universal definition of application readiness. Long
polling, analytics, service workers, or background requests can prevent it,
while an idle network does not prove that the desired UI state is ready. A
web-first assertion or explicit application condition is often stronger.

### Page and Frame events maintain a changing tree

The runtime observes frame attachment, detachment, navigation, and load-state
changes and updates both server and client representations. Consumers may query
the main Frame, all Frames, parents, names, and URLs. Operations against a Frame
must detect detachment instead of continuing against an unrelated document.

### Popup is another Page

Opening a new window creates another Page in the same BrowserContext. It may
refer to its opener Page, but it has its own main Frame, URL, events, and
lifecycle. Because popup creation races with the triggering action, consumers
usually arm a Page or Context event wait before clicking.

```text
begin waiting for popup
  -> click the control that calls window.open
  -> receive the new Page
  -> wait for the popup's desired state
```

Context-level page events are useful when the opener is unknown or when policy
must apply before the popup's first request. Page-level popup events express the
opener relationship.

### Lifecycle ownership

Closing a Page closes that browsing surface and invalidates its Frames,
workers, handles, and pending operations. Closing its BrowserContext closes all
Pages in the context. A crash and an orderly close are different events and
should produce different diagnostics.

The `runBeforeUnload` option changes close semantics: the close request may
initiate unload handling without guaranteeing immediate closure if the page
opens a dialog. This is one example of why close is an asynchronous lifecycle
operation rather than merely dropping a client object.

### Source anchors

- Page protocol: `packages/protocol/spec/page.yml`
- Frame protocol: `packages/protocol/spec/frame.yml`
- Client Page: `packages/playwright-core/src/client/page.ts`
- Client Frame: `packages/playwright-core/src/client/frame.ts`
- Server frame/navigation state: `packages/playwright-core/src/server/frames.ts`

### Questions raised for Pwright

- Should Page map one-to-one to a CDP target, or remain a portable logical
  object with backend-specific target recovery?
- Which navigation milestones are compatibility requirements?
- How should Pwright represent a stable Frame identity across document changes
  without retaining stale execution contexts or handles?
- Which Page events require lossless delivery, and which can be bounded or
  coalesced?
- How should popup waiting and first-request routing avoid races?

### Learner checkpoint

- Question 1 answer: initially “Page.” Correction: navigation and document
  lifecycle belong to the main Frame; Page exposes convenience methods that
  delegate to it.
- Question 2 answer: the Page is not necessarily replaced when navigating from
  one site to another. Correct; the Page remains while its document and
  execution contexts change.
- Question 3 answer: analytics may keep generating traffic. Correct; background
  traffic can prevent network idle, and network idle also cannot prove the
  desired application state is ready.
- Question 4: needs reinforcement. Arm the popup wait first, then perform the
  action, then await the already-created wait; otherwise the event can occur
  before a listener exists.

No Pwright decision is selected in this lesson.

### Pwright implementation decision: explicit Page creation

The first `BrowserContext.newPage()` slice must return an initialized Page and
its required stable main Frame together. Chromium creation therefore includes
`Target.createTarget`, flattened attachment, and `Page.getFrameTree`; a target
ID alone is not success. Runtime publication is one Page+Frame creation batch,
and remote publication sends both Create controls before Result. Explicit
creation is kept separate from popup/event discovery and opener semantics.
The complete transaction and acceptance gates are recorded in
[`pwright-page-creation.md`](pwright-page-creation.md).

Implementation checkpoint: the optional capability is now wired end to end.
AAA tests prove two-object runtime rollback, atomic client claims, exact
Page→Frame→Result remote ordering, topology/route publication, compensation,
and caller-abandonment continuation. A dedicated Chrome 152 run also proved
real Context creation, `about:blank` Page plus initialized main Frame, typed
Page close, Context close, and survival of the externally owned browser.

## Lesson 5: Selectors, Locator, FrameLocator, and strictness

### Selector describes matching; Locator describes a reusable query

A selector is a description that the selector engine can evaluate against a
document. A Locator combines a Frame with a selector pipeline. Constructing a
Locator normally performs no DOM query and creates no server-side element
object.

```text
Locator = Frame reference + selector description
```

Locator methods such as `getByRole`, `getByText`, `filter`, `locator`, `and`,
`or`, `first`, and `nth` return new query descriptions. The query is resolved
when an action, read, wait, or assertion needs current DOM elements.

### User-facing locator families

- `getByRole` matches accessibility role and accessible name.
- `getByLabel` connects form controls to their labels.
- `getByPlaceholder`, `getByAltText`, and `getByTitle` use user-facing
  attributes.
- `getByText` matches rendered text semantics.
- `getByTestId` uses a configurable test-id attribute.
- `locator` accepts CSS and Playwright selector composition.

Role, label, and text locators express how a user perceives the interface and
often survive implementation refactors better than CSS paths. Test IDs provide
an explicit stable testing contract when user-facing semantics are insufficient.

### Lazy resolution enables retries

The same Locator can resolve to different concrete DOM nodes over time. On an
action, Playwright resolves it, checks strictness and actionability, and may
resolve it again after a DOM update. A concrete ElementHandle instead refers to
one particular node and can become detached or stale.

```text
Locator: reusable recipe for finding the current element
ElementHandle: reference to one concrete element instance
```

### Composition is relative and immutable

Chaining narrows within the previous result. Filters constrain candidates by
text or descendant locators. `and` computes an intersection, while `or`
combines alternatives. These operations create new Locators rather than
mutating the original.

```text
page.getByRole(listitem)
    .filter(hasText = "Product 2")
    .getByRole(button, name = "Add")
```

Inner `has`/`hasNot` locators and locator intersections must belong to the same
Frame, because selector evaluation cannot silently cross document boundaries.

Pwright checkpoint: locator protocol 1.3 represents `hasText`, `hasNotText`,
`has`, `hasNot`, and `visible` as bounded recursive AST nodes. Page locators
normalize to an initialized main Frame, so Page and main-Frame locators compare
as the same scope while another Frame is rejected before dispatch. Chromium
keeps base candidates for `has`/`hasNot`, evaluates the inner selector relative
to each candidate, and does not retain a resolved DOM node between actions.

Pwright checkpoint: locator protocol 1.4 adds Playwright's string-or-`RegExp`
branch without treating Rust regex syntax as equivalent. The portable value is
ECMAScript source plus canonical JavaScript flags; string matching retains its
normalization/exact rules, while regex matching tests complete text and ignores
`exact`. New tags cover text, label, positive/negative text filters, and role
name/description patterns without changing 1.3 bytes. Chromium compiles the
expression per selector evaluation, returns invalid source as
`InvalidSelector`, and retains neither compiled regex nor a DOM match between
retries.

Pwright checkpoint: locator protocol 1.5 adds `get_by_placeholder`,
`get_by_alt_text`, `get_by_title`, and `get_by_test_id`, including regex
branches. Placeholder/alt/title string matching uses the raw attribute value:
`exact` is case-sensitive equality, while the default is a case-insensitive
substring. Unlike text locators, it does not normalize whitespace. Test-ID
strings are always exact and default to `data-testid`. Pwright carries any
explicit comma-separated test-ID attribute names inside the immutable Locator
(maximum 16), avoiding Playwright's mutable selector-registry setting while
keeping the same query behavior. Chromium runtime v6 and a real Chrome 152
test cover the string, regex, default-name, and multi-name paths.

Pwright checkpoint: locator protocol 1.6 adds `Locator.count` as the first
multi-match read. It follows Playwright's `queryCount` shape: resolve with
strictness disabled, take one snapshot, return `0` immediately when nothing
matches, and return the number of matches without waiting for actionability.
The portable result is a checked `u32`; Chromium retains no node or result
array after the evaluation. The same lazy Locator resolves again after a page
navigation.

Pwright checkpoint: locator protocol 1.7 adds Playwright's immediate
`isVisible` behavior. It is strict for multiple matches, returns `false` for a
missing target, and does not poll for a later visible state. `isHidden` is the
logical inverse and is composed in the Rust client over the same typed command,
showing that API conveniences need not become separate backend plugins. A real
Chrome 152 proof covers visible, CSS-hidden, missing, strict, and post-navigation
resolution.

Pwright checkpoint: locator protocol 1.8 adds `isChecked`, `isEditable`, and
`isEnabled` as separate typed routes, while `isDisabled` is composed by
negating enabled. Playwright treats these differently from visibility: they
wait for one strict connected target, then return its current state rather
than waiting for the state to become true. Chromium runtime v6 implements
label retargeting, native disabled/readonly/checked behavior, and applicable
ARIA state behavior without retaining the resolved node. A real Chrome 152
proof includes a target inserted after the query begins.

Pwright checkpoint: locator protocol 1.9 adds Playwright's `waitFor` timing
contract as a separate optional route. The four states are attached, detached,
visible, and hidden; visible is the default. The operation is strict and
re-resolves the Locator on every attempt. With no current match, hidden and
detached are already true, while visible and attached continue retrying until
the caller deadline. Success returns no element, so this waiting primitive
does not convert a Locator into a retained ElementHandle. Strictness, invalid
selectors, and malformed responses remain terminal rather than retryable.

Pwright checkpoint: locator protocol 1.10 adds `textContent` as the first
single-target DOM-content read. Like Playwright, it waits for one strict target
and re-resolves the lazy Locator while waiting. It returns `Node.textContent`
exactly, including whitespace, and preserves the nullable result rather than
inventing an empty string. Pwright bounds the UTF-8 result to 1 MiB and keeps
the operation handle-free; an assertion such as `toHaveText` remains a
different polling contract layered above this one-shot content read.

Pwright checkpoint: locator protocol 1.11 adds `innerText` as a distinct
optional route. It waits for one strict connected target like `textContent`,
but then requires an `HTMLElement` and returns the browser's rendered text:
hidden descendants and layout whitespace rules can therefore make it differ
from raw `Node.textContent`. The result is non-null, bounded to 1 MiB, and
handle-free. An SVG target is a typed `Node is not an HTMLElement` failure,
matching the relevant Playwright server boundary.

Pwright checkpoint: locator protocol 1.12 adds `innerHTML` as another distinct
optional route. It has the same strict waiting and re-resolution behavior, but
returns the browser's serialized child markup rather than text. Playwright
reads `Element.innerHTML` directly, so SVG is valid here even though SVG is
invalid for `innerText`. Pwright preserves comments and browser escaping,
bounds the non-null UTF-8 result to 1 MiB, and retains no ElementHandle. This
contract was checked against the local upstream Playwright snapshot at commit
`de214f440b7e34937fe4886f046b78b757136087`.

Pwright checkpoint: locator protocol 1.13 adds `getAttribute(name)` as its own
optional route. Playwright's client forces strict selector resolution and its
server waits/re-resolves before directly calling `Element.getAttribute(name)`.
That means an existing attribute returns its exact string, a missing attribute
returns `null`/Rust `None`, and SVG is valid because no HTMLElement-only check
is involved. Pwright bounds the name to 64 KiB and a present value to 1 MiB,
runs the read in the utility world under the caller deadline, and retains no
ElementHandle. The same local upstream commit above is the source reference.

Pwright checkpoint: locator protocol 1.14 adds `inputValue()` as its own
optional route. Playwright's client forces strict resolution; its server then
uses `retarget(node, 'follow-label')`, accepts only `INPUT`, `TEXTAREA`, or
`SELECT`, and returns the current `.value`. Pwright mirrors that boundary with
selector runtime v6's shared retarget primitive, a typed non-control error,
strict retrying resolution, a 1 MiB UTF-8 result ceiling, caller deadline, and
no ElementHandle. Real Chrome 152 verifies delayed label retargeting plus exact
input, textarea, and select values. The local upstream commit above is the
source reference.

Pwright checkpoint: locator protocol 1.15 adds `allTextContents()` as a
separate collection route. At upstream commit
`de214f440b7e34937fe4886f046b78b757136087`, the client implementation calls
`frame.$$eval(selector, ee => ee.map(e => e.textContent || ''))`. Therefore it
does not wait for a match, does not apply strictness, returns `[]` immediately
for zero matches, converts a nullable `textContent` to `""`, and preserves the
current selector order. Pwright mirrors those semantics while bounding the
handle-free result to 4096 values, 1 MiB per string, and 4 MiB total UTF-8; a
real Chrome 152 proof also reuses the same lazy Locator after navigation.

Pwright checkpoint: locator protocol 1.16 adds `allInnerTexts()` as a separate
rendered-text collection route. At the same upstream commit, the client calls
`frame.$$eval(selector, ee => ee.map(e => (e as HTMLElement).innerText))`.
It therefore has the same immediate, zero-or-many, non-strict snapshot shape
as `allTextContents()`, but `innerText` is layout-aware and omits hidden
descendant text. The TypeScript cast does not check runtime element type, so an
SVG match can yield `undefined` despite the declared `string[]`. Pwright makes
that edge explicit: every match must be an `HTMLElement`, otherwise the call
returns typed `InvalidTarget`. The result remains handle-free and uses the
same 4096-item, 1 MiB-per-item, and 4 MiB aggregate bounds; Chrome 152 verifies
rendered output, the SVG failure, and lazy re-resolution after navigation.

Pwright client-composition checkpoint: Playwright implements `clear()` by
calling `fill('')`, and implements `all()` by awaiting `count()` once and
creating `nth(i)` Locators for that count. Neither behavior needs a new browser
protocol command. Pwright follows that dependency shape: Clear installs only
the Fill service and All installs only Count. The Rust result is a lightweight
`LocatorList`, not an eagerly allocated array: it records the count snapshot
and one base selector, then creates `nth(i)` values through `get` or `iter`.
The indices are a snapshot but the Locators remain lazy, so DOM changes or
navigation between `all()` and use affect what each index resolves to. Chrome
152 verifies both that behavior and the empty Fill used by Clear.

Pwright checkpoint: locator protocol 1.17 adds `hover()` as a separate action
route. At upstream commit `de214f440b7e34937fe4886f046b78b757136087`, hover
uses strict resolution, waits for visible and stable geometry, scrolls unless
disabled, chooses either the visible center or a padding-box-relative position,
and checks the hit target unless forced. Unlike click it does not wait for
enabled state or navigation. Pwright preserves finite fractional coordinates,
canonical bounded modifiers, platform-sensitive `ControlOrMeta`, force, scroll,
and trial. Playwright distinguishes omitted modifiers from explicit `[]`:
omission leaves current Page keys alone, while a supplied set is ensured for
the action and the prior set is restored. This also happens during trial even
though trial sends no mouse move. Pwright now follows that ownership model with
one fixed-size Page coordinator shared across main, local-frame, and OOPIF
Locator routes. Chrome 152 verifies a disabled target, fractional positioned
input, Meta plus Shift on macOS, the actual key events and restoration, and the
absence of a trial mouse move.

### Strictness protects single-element operations

Locator actions that require one target run in strict mode. Zero matches may be
waited for until the deadline; more than one current match causes a strict-mode
violation because Playwright cannot safely guess which element the user meant.
Operations designed for multiple results, such as counting, do not require a
single match. They also have different waiting semantics: `count()` is a
snapshot query, so callers that need an eventual count should use an explicit
web-first assertion or wait condition rather than expecting count itself to
poll.

`first`, `last`, and `nth` explicitly select one candidate, but can hide an
unstable or overly broad locator. A uniquely meaningful role/name or test ID is
usually a stronger contract.

### FrameLocator crosses an iframe boundary lazily

FrameLocator identifies an iframe element and then evaluates descendant
locators inside that iframe's content Frame. It remains lazy, so replacement or
navigation of the iframe can be handled at operation time.

```text
Page main Frame
  -> locate iframe[name=payment]
  -> enter its content Frame
  -> locate button named Pay
```

A normal Locator does not cross into an iframe document automatically. Explicit
FrameLocator composition makes the document boundary visible and subject to
strictness.

### Source anchors

- Client Locator and FrameLocator: `packages/playwright-core/src/client/locator.ts`
- Locator selector builders: `packages/isomorphic/locatorUtils.ts`
- Server selector resolution: `packages/playwright-core/src/server/frameSelectors.ts`
- Frame action protocol: `packages/protocol/spec/frame.yml`

### Questions raised for Pwright

- Should the selector representation be an inspectable typed AST, a compatible
  string language, or both?
- Which locator engines belong to independently installable capabilities?
- How can role/text semantics stay compatible without embedding unnecessary
  server-side state?
- Should Locator remain entirely client-side and allocation-light?
- Which strictness and composition behaviors are required for compatibility?

No Pwright decision is selected in this lesson.

## Lesson 6: Actions, actionability, auto-waiting, and timeouts

### An action is a retrying state machine

Locator actions do not immediately emit browser input. A typical click resolves
one current element, performs page-level prechecks, checks actionability,
scrolls, verifies the hit target, emits input, and waits for consequences such
as navigation. Recoverable failures cause the whole resolution/check sequence
to retry until the operation deadline.

```text
resolve strict Locator
  -> check current element state
  -> scroll into view
  -> verify pointer hit target
  -> dispatch input
  -> wait for action consequences
```

If the DOM replaces the element, an overlay intercepts the point, or the
element moves during the sequence, Playwright can re-resolve and retry instead
of continuing with stale coordinates.

### Actionability states

- Visible: the current element has a rendered, non-empty box and is not hidden
  according to Playwright's visibility rules.
- Stable: its relevant bounding box remains stable across animation frames.
- Receives events: the intended hit point is not intercepted by another
  element and belongs to the target's hit-test path.
- Enabled: it is not disabled according to applicable HTML/ARIA semantics.
- Editable: it is an input-like editable target and is not readonly.

Actions require different combinations. Click waits for visible, stable,
enabled, and pointer-event reception. Hover does not require enabled. Fill
waits for visible, enabled, and editable. Select option waits for visible and
enabled. There is no single universal checklist for all actions.

Current Playwright Locator click options include button, click count, delay,
force, modifiers, position, scroll mode, interpolated movement steps, trial,
and navigation-wait policy. Its mouse implementation moves first, then emits
each down/up pair; a nonzero delay is awaited between down and up and between
multiple clicks. Modifier setup and restoration surround the pointer action,
including trial's temporary modifier behavior.

Pwright checkpoint: the existing Click route now shares Hover's Page-scoped
input transaction and emits move, down, then up. The owner remembers only fixed
pointer coordinates, one pressed button, and modifier bitsets, so cancellation
can release a button before later Page input proceeds. Richer options are not
yet added because locator 1.x uses exact positional records; extending that
record requires an explicit protocol-versioning design rather than silently
breaking older payloads.

Pwright design experiment: a generated option bag can instead use a bounded
extensible record. Each present option has a stable nonzero tag and an exact
borrowed value slice; absent and present-empty remain distinct, while old
decoders skip future tags without allocating. The complete envelope keeps one
fixed byte ceiling across compatible minors, and only new optional tags may be
introduced. This makes Playwright-like growth of Click options technically
possible without a new command per option, but adopting it still requires a
locator major-version and migration decision because the current Click bytes
are positional.

### Auto-waiting and retry

Auto-waiting is bounded polling of a complete operation precondition, not a
fixed sleep. Recoverable states include no match yet, current element
detachment, movement, invisibility, disabled state, or hit-target obstruction.
Retries use the same overall deadline and produce a call log explaining what
was awaited. Nonrecoverable errors and strictness violations fail rather than
silently selecting another meaning.

### Force and trial are deliberate escape hatches

`force` skips important safety checks so Playwright can attempt the underlying
operation even when normal actionability is not satisfied. It does not make an
operation semantically successful; the application may ignore the input or a
backend error may still occur.

`trial` performs the checks without the final input action. It can be used to
wait until an element would be actionable while avoiding the side effect.

### Timeout is an operation deadline

An action timeout bounds locator resolution, state checks, retries, scrolling,
input, and associated waiting as one operation. It is not a separate full
timeout for every retry. Per-call timeout overrides Page/BrowserContext
defaults. Navigation has a separately configurable navigation timeout because
its lifecycle may have different operational expectations.

A useful timeout error reports the API operation, selector, waited state,
attempt history, and final obstruction or lifecycle condition. Cancellation,
page closure, frame detachment, browser crash, and deadline expiry are distinct
termination causes.

### Source anchors

- Server element actions and checks: `packages/playwright-core/src/server/dom.ts`
- Locator re-resolution and retries: `packages/playwright-core/src/server/frames.ts`
- Injected DOM state checks: `packages/injected/src/injectedScript.ts`
- Client timeout precedence: `packages/playwright-core/src/client/timeoutSettings.ts`
- Action flags and typed parameters: `packages/protocol/spec/frame.yml`

### Questions raised for Pwright

- Which actionability definitions must be behaviorally compatible with
  Playwright, including shadow DOM, SVG, CSS transforms, and iframes?
- Which checks belong to reusable capability components versus an indivisible
  action engine?
- How can retries retain useful diagnostics without unbounded logs or object
  history?
- How should one Rust deadline and cancellation token cover all backend work
  spawned by an action?
- Which force/trial semantics are safe to expose and test?

No Pwright decision is selected in this lesson.

## Lesson 7: Assertions and retry semantics

### Assertion asks whether an observable condition becomes true

Playwright Test extends ordinary value assertions with asynchronous web-first
assertions for Locator, Page, and Response objects. A web-first assertion
repeatedly observes current browser state until it passes or reaches its own
deadline.

```text
expect(locator).toHaveText("Saved")
  -> resolve current Locator
  -> read current text
  -> compare
  -> retry if it does not yet match
```

This differs from reading once and asserting the captured value. An ordinary
assertion such as `expect(await locator.textContent()).toBe("Saved")` observes
one moment and does not automatically re-read the DOM.

### Assertions and actions wait for different facts

An action waits for the preconditions needed to safely perform its side effect.
An assertion waits for an expected postcondition. A successful click proves
that Playwright performed the click under its actionability contract; it does
not prove that the application saved data or displayed the expected message.

```text
click action: wait until Submit can be clicked, then click
assertion:    wait until status becomes Saved
```

### Common web-first assertions

- Locator state: visible, hidden, enabled, disabled, editable, checked,
  focused, attached, empty, or in viewport.
- Locator content: text, accessible name, accessible description, value,
  attribute, CSS, class, count, and list contents.
- Page state: URL and title.
- Response state: success status.
- Visual/accessibility state: screenshot and ARIA snapshot comparisons.

Positive and negated assertions both retry their own condition. Negation is not
an instruction to sleep; it observes until the forbidden condition is absent or
the assertion deadline expires.

### Expect timeout is separate

Playwright Test's asynchronous expect timeout defaults to 5000 milliseconds in
the inspected source and can be configured globally or per assertion. It is
separate from action, navigation, and whole-test timeouts. The assertion cannot
outlive the remaining test deadline even if its configured expect timeout is
longer.

### Polling general conditions

`expect.poll` repeatedly invokes a user-provided observation function and
applies a matcher to its returned value. `toPass` retries a callback containing
assertions. These tools cover eventual conditions outside built-in Locator/Page
matchers, but broad callbacks can repeat side effects and require deliberate
use.

### Failure diagnostics and soft assertions

A failed assertion reports the matcher, expected value, most recent received
state, timeout, and call log or diff where applicable. Screenshot assertions
can produce expected, actual, and diff artifacts.

A soft assertion records the failure and allows the test to continue. It does
not turn failure into success; the test remains failed. Continuing after an
invalid prerequisite can create misleading secondary failures, so soft mode is
best for independent checks.

### Source anchors

- Expect orchestration and polling: `packages/playwright/src/matchers/expect.ts`
- Locator/Page matcher definitions: `packages/playwright/src/matchers/matchers.ts`
- Server-side retried expect query: `packages/playwright-core/src/server/frames.ts`
- Public assertion types and timeout configuration: `packages/playwright/types/test.d.ts`

### Questions raised for Pwright

- Are assertions part of the core automation library, an optional Rust testing
  crate, or adapters for existing Rust assertion frameworks?
- Which query/result protocol can actions and assertions share without coupling
  their retry policies?
- How can failure diagnostics retain the last useful state without retaining an
  unbounded history?
- Should assertion deadlines be a separate typed policy from action and
  navigation deadlines?
- Which comparison artifacts require optional capabilities?

No Pwright decision is selected in this lesson.

## Lesson 8: JavaScript evaluation, JSHandle, and ElementHandle

### Evaluation crosses two JavaScript environments

`page.evaluate` and `frame.evaluate` execute a supplied function inside the
page's JavaScript execution context, not in the Playwright client process. Its
arguments cross into the page and its result crosses back through Playwright's
serialization protocol.

```text
client value -> serialize -> page execution context
page result  -> serialize -> client value
```

Page functions cannot close over arbitrary client variables. Data must be
passed explicitly as serializable values or live handles. Playwright's value
format preserves more than plain JSON, including undefined, NaN, infinities,
negative zero, BigInt, dates, URLs, regular expressions, errors, typed arrays,
arrays, objects, and circular references where supported by the protocol.

### evaluate returns by value

`evaluate` requests a serialized copy of the result. This is appropriate for
small data that has a meaningful client representation: strings, numbers,
booleans, arrays, plain objects, and extracted page state. The returned client
value is detached from later changes to the page object.

DOM nodes, functions, browser-native objects, and large cyclic object graphs do
not generally have a useful by-value representation. For live identity,
Playwright offers `evaluateHandle`.

### JSHandle retains a page-side object

A JSHandle represents one live JavaScript value in one execution context. The
browser/runtime keeps that object reachable while the handle exists. The
handle can evaluate functions against the object, access properties as more
handles, or request a serialized `jsonValue` when possible.

JSHandle is tied to its execution context. Navigation destroys document
execution contexts, so associated handles become unusable. Handles from an
incompatible context cannot be passed arbitrarily without backend-supported
adoption.

### ElementHandle specializes JSHandle

ElementHandle is a JSHandle whose referenced value is a DOM node. It adds DOM
geometry and element actions, but remains bound to one concrete node. If React
or the browser replaces or detaches that node, the handle does not become a
recipe for finding the replacement. Playwright therefore discourages routine
ElementHandle use in favor of Locator.

```text
Locator       = repeatable query, resolves current node at operation time
ElementHandle = live reference to one specific node
```

### Disposal and memory

A live handle can prevent the referenced browser-side object and related graph
from being garbage-collected. It also occupies Playwright client/server object
tables and underlying protocol remote-object state. `dispose` releases the
reference explicitly; context destruction also invalidates and cleans up its
handles.

Creating handles for many nodes or object properties without disposal can grow
memory in the browser and driver simultaneously. Temporary internal handles
must be disposed on success, error, timeout, and cancellation paths.

### Source anchors

- Handle protocol: `packages/protocol/spec/handles.yml`
- Value serialization schema: `packages/protocol/spec/serialized.yml`
- Client handle implementation: `packages/playwright-core/src/client/jsHandle.ts`
- Server evaluation and handle lifecycle: `packages/playwright-core/src/server/javascript.ts`
- ElementHandle implementation: `packages/playwright-core/src/server/dom.ts`
- Public API guidance: `packages/playwright-core/types/types.d.ts`

### Questions raised for Pwright

- Is arbitrary JavaScript evaluation a core feature or an optional capability?
- Which typed value format should Rust use without depending on Serde JSON in
  the core model?
- Can common extraction APIs return typed values without creating handles?
- How should Rust make handle disposal deterministic while supporting async
  backend release from synchronous `Drop`?
- What hard limits and metrics are required for live handles per context?

### Pwright implementation checkpoints (2026-09-04)

The first Handle slice deliberately implemented release before creation.
`JsHandle` and `ElementHandle` have stable portable object kinds under Frame,
and the optional `pwright.handle_lifecycle` capability provides explicit async
`dispose`. Its Chromium leaf maps object-backed Handles to exactly one
`Runtime.releaseObject` call on the bound CDP session. Rust `Drop` still
performs no I/O.

The next slice adds an independently installable `evaluateHandle` foundation.
Page delegates to its main Frame; the result is a concrete typed JSHandle or
ElementHandle with portable identity, while CDP object/session/context IDs stay
inside a bounded Chromium route. Creation commits runtime Create plus native
route together and compensates failure with release. Explicit disposal releases
the exact native object before removing runtime identity. Remote client
materialization requires the matching Create before Result, and a dedicated
Chrome 152 run verified real DOM-node creation/release while preserving the
externally owned browser.

The 1.1 follow-up now mirrors Playwright's important argument boundary: a
recursive serialized graph may contain portable Handle leaves, while native
object IDs remain server-local CDP `CallArgument`s. Pwright deliberately checks
that a generic JSHandle and target Frame share one still-current execution
world; cross-Frame and disposed values fail before CDP. A dedicated Chrome 152
test passes a real ElementHandle into a second evaluation and reads `SECTION`.
The independently installable value-evaluation capability remains Handle-free,
and 1.0 Handle-evaluation peers are prevented from sending the new leaf.

This still does not equal Playwright's complete JSHandle surface. Cross-world
ElementHandle adoption remains a lesson-derived gate. Handle-local evaluation,
`jsonValue`, `getProperty`, and `getProperties` are now specified together as
the additive `pwright.handle_evaluation` 1.2 contract in
`pwright-handle-operations.md`; implementation remains open. The live-Chrome negotiated Handle-command round
trip now passes through the complete typed client/server recipe at capability
version 1.1, including lifecycle Create ordering, a Handle-valued second
command, graceful close, and external-browser preservation. It also exposed
and closed the codec boundary where the cancellation-aware client channel adds
an operation identity before command encoding.
The production
route profile now accounts every retained Handle and byte. Explicit Page/
Context close, main execution-world replacement, Frame detach, Page crash,
reconnect, and the Handle-aware Pwright connection-close target all retire the
corresponding runtime objects, routes, and leases without redundant native
release calls. See `pwright-handle-evaluation.md`.
Direct creation also reserves caller-owned continuation capacity before CDP;
abandoning a started future completes evaluation and explicitly disposes its
otherwise unreachable result.

The production composition now selects Handle lifecycle and Handle evaluation
independently from value evaluation. Evaluation requires lifecycle and, in
remote mode, ordered control. Negotiation constructs only accepted codecs and
handlers. The remote handler preserves the same reserved cleanup ownership
after Create and through Result delivery: if Result transmission is abandoned,
it performs one ordered Dispose. This turns the lesson's “cleanup on every
success, error, timeout, and cancellation path” rule into a tested ownership
transition rather than a best-effort convention.

The canonical complete recipe now includes both Handle command trees and uses
the Handle-aware runtime wrapper as a type-level requirement. Connection close
therefore retires the route table even if application code forgets to select a
cleanup decorator. A Chrome 152 run accepted all twelve application
capabilities plus control and preserved the externally owned browser; the live
test still needs to execute a Handle command through that negotiated channel.

### Learner checkpoint

- Question 1 answer: a page function cannot directly read an ordinary client
  local variable. Correct; the value must be passed as an explicit argument or
  compatible handle.
- Question 2 answer: initially “it will update.” Correction: `evaluate` returns
  a serialized client-side copy, so later mutations to the page object do not
  update the returned value. Live identity requires `evaluateHandle`.
- Question 3 answer: ElementHandle binds one concrete DOM node. Correct;
  Locator is a repeatable query description.
- Question 4 answer: garbage collection cannot reclaim the object. Correct in
  essence; a live handle roots browser-side state and also occupies protocol,
  driver, and client object tables until disposal or context destruction.

No Pwright decision is selected in this lesson.

## Lesson 9: Request, Response, Route, WebSocket, HAR, and Download

### HTTP traffic is represented as related lifecycle objects

For a successful HTTP exchange, Playwright emits a Request, then a Response
when response headers arrive, and finally request-finished after the body is
complete. A transport failure emits request-failed instead. HTTP error status
codes such as 404 and 500 are still valid HTTP responses and normally complete
through the response/finished path.

```text
success: Request -> Response(headers) -> RequestFinished(body complete)
failure: Request ----------------------> RequestFailed
```

Request contains URL, method, headers, post data, resource type, navigation
status, Frame or service-worker association, timing, failure, and redirect
links. Response points back to its Request and contains status, headers, timing,
service-worker provenance, security/server details, sizes, and an on-demand
body operation.

Each HTTP redirect is a new Request/Response pair linked through
`redirectedFrom` and `redirectedTo`; Playwright does not mutate one Request into
all stages of the chain.

### Observation and interception are different

Request/Response events observe traffic without deciding its outcome. Routing
pauses a matching request and creates a Route that must resolve exactly once:

- `continue` sends it to the network, optionally overriding request data.
- `fallback` gives another registered handler an opportunity to process it.
- `fulfill` supplies a synthetic or modified response.
- `abort` terminates it with a network error.

Routes may be Page- or BrowserContext-scoped. Context routing can cover popups
and all pages early enough to avoid first-request races. Service workers can
introduce traffic and caching paths that do not behave like ordinary page
requests, so their policy must be explicit in routing-heavy scenarios.

### Bodies are expensive and intentionally on demand

Response headers and metadata are available before the body finishes.
`response.body()` waits for or retrieves the body and transfers binary data
through the protocol. Reading every response body, base64-encoding it, retaining
network object histories, or recording HAR with content can consume substantial
browser, driver, client memory, and disk. Navigation can also make an unread
body unavailable in some backend conditions.

### WebSocket is a long-lived stream

Playwright can observe WebSocket open, sent/received frames, errors, and close.
WebSocket routing can connect, proxy, inject, or close messages between the page
and server. Unlike HTTP, it has no one-response completion point, so frame
queues, payload retention, subscriptions, and backpressure require explicit
bounds.

### HAR records or replays HTTP exchanges

HAR recording captures request/response metadata and optionally content for
later inspection. HAR routing can use recorded entries to fulfill matching
requests, enabling deterministic offline or mocked scenarios. Completeness,
update mode, matching rules, redirects, service workers, and large/binary bodies
are important limitations. Context closure is needed to flush a recording.

### Download is an Artifact, not a normal response body

A download event yields a Download associated with its Page, URL, suggested
filename, progress/failure state, and backing Artifact. Consumers can save it
to an explicit path, cancel it, or delete it. The suggested filename is data,
not authorization to overwrite a destination; safe path and overwrite policy
belong to the caller or delivery surface.

Downloads are typically stored temporarily under runtime ownership and are
removed when their BrowserContext closes unless saved elsewhere. Waiting for a
download follows the same race rule as popup: arm the download event wait before
triggering the action.

### Source anchors

- Network protocol objects: `packages/protocol/spec/network.yml`
- Client Request/Response/Route: `packages/playwright-core/src/client/network.ts`
- Server network lifecycle: `packages/playwright-core/src/server/network.ts`
- Context/Page event routing: `packages/playwright-core/src/client/browserContext.ts`
- Download and Artifact lifecycle: `packages/playwright-core/src/client/download.ts`,
  `packages/playwright-core/src/server/download.ts`, and
  `packages/playwright-core/src/server/artifact.ts`

### Questions raised for Pwright

- Which network objects require stable remote identity, and for how long?
- Can metadata observation remain lightweight while body capture is a separate
  installable capability?
- What queue and payload limits apply to HTTP events and WebSocket frames?
- How should multiple route handlers compose without hidden ordering or stalled
  requests?
- Which artifact store, filename, overwrite, range, and cleanup policies belong
  outside the core?

### Learner checkpoint

- Question 1 answer: initially `requestfailed` for HTTP 500. Correction: 500 is
  a valid HTTP Response, so it follows response and normally request-finished;
  request-failed is for transport-level failure or abortion.
- Question 2 answer: the request will not pass. Correct; an intercepted Route
  remains paused until exactly one terminal decision is made or the surrounding
  target/operation terminates.
- Question 3 answer: the body stream may not be complete when response fires.
  Correct; response represents headers, while completion occurs later.
- Question 4: needs reinforcement. A WebSocket can produce frames indefinitely
  and faster than a consumer can process them; an unbounded pending-event queue
  therefore creates unbounded memory growth.
- Question 5 answer: create the download wait first. Correct; then trigger the
  action and await the already-armed wait to avoid an event race.

No Pwright decision is selected in this lesson.

## Lesson 10: Context state, authentication, permissions, and emulation

### BrowserContext is the environment boundary

Playwright places browser-session identity and environment policy on
BrowserContext so every Page created inside it begins with a coherent world.
Creation-time options include viewport/screen, user agent, locale, timezone,
geolocation, permissions, proxy, HTTP credentials, client certificates, extra
headers, offline mode, JavaScript/CSP policy, touch/mobile traits, media
preferences, downloads, service workers, storage state, video, and selectors.

Some properties can be updated later, such as cookies, permissions,
geolocation, headers, credentials, offline mode, and selected page media.
Others are most reliable when fixed before the first Page so the initial
request, scripts, layout, and feature detection see one consistent environment.

### Authentication has multiple independent layers

- Application session authentication usually lives in cookies and origin
  storage and can often be restored through `storageState`.
- HTTP authentication handles protocol challenges such as Basic or Digest and
  is configured with HTTP credentials.
- Proxy authentication belongs to proxy configuration.
- Mutual TLS uses client certificates associated with origins.
- WebAuthn credentials are a separate browser credential model.

These mechanisms are not interchangeable. A username/password form normally
creates application cookies; configuring HTTP credentials does not fill that
form or manufacture its application session.

### Cookies and origin storage

Context cookie APIs can inspect, add, and clear cookies with domain/path and
modern partitioning semantics. Pages in one Context share those cookies while
other Contexts do not by default.

`storageState` captures cookies plus per-origin localStorage, with optional
IndexedDB and newer credential data in the inspected source. It is a selected,
portable automation snapshot rather than a complete browser profile. It does
not generally represent every cache, service worker, extension, preference,
session-only runtime object, or full user-data-directory detail.

### Permissions and geolocation are coupled

Setting a geolocation value supplies coordinates, but a site normally also
needs geolocation permission for its origin. Permission grants and resets are
Context-scoped. Camera, microphone, notifications, clipboard, and other
permissions have browser/backend differences and may require actual device or
media configuration in addition to permission.

### Device emulation is a bundle of signals

A device profile combines multiple independent signals: viewport, screen,
device scale factor, touch support, mobile layout behavior, user agent, and
often locale-related defaults. Changing only the viewport does not turn desktop
Chrome into a complete phone model. Emulation affects the signals exposed by
the browser; it does not change the underlying browser engine, operating
system, CPU, GPU, network, or all hardware behavior.

Media preferences such as color scheme, reduced motion, forced colors, and
contrast influence CSS media queries. Locale and timezone influence formatting
and time behavior but are distinct from geolocation and network location.

### Offline, headers, proxy, and service workers

Offline mode instructs the backend to simulate unavailable network access; it
does not erase previously cached or application-visible state. Extra HTTP
headers apply to requests but are not equivalent to browser JavaScript-visible
properties. Proxy determines network routing and may need bypass/auth policy.
Service workers can serve cached responses or initiate traffic, complicating
routing, offline tests, and deterministic state; contexts can allow or block
them when supported.

### State files are secrets

Storage-state and persistent-profile files may contain authenticated cookies,
tokens, local data, credentials, and personally identifiable information. They
must not be committed, logged, served, or shared as ordinary test artifacts.
Their ownership, encryption, expiry, redaction, and deletion are security
requirements rather than convenience details.

### Source anchors

- Context option schema: `packages/protocol/spec/mixins.yml`, `ContextOptions`
- Context commands: `packages/protocol/spec/browserContext.yml`
- Client context API: `packages/playwright-core/src/client/browserContext.ts`
- Server state capture/restore: `packages/playwright-core/src/server/browserContext.ts`
- Public context options: `packages/playwright-core/types/types.d.ts`

### Questions raised for Pwright

- Which context policies belong to separately installable emulation,
  authentication, storage, network, and credential capabilities?
- How can a backend report unsupported or partially supported settings instead
  of silently pretending they worked?
- What portable storage-state subset should Pwright support, and how are secret
  files protected?
- Which creation-time settings form one consistency contract and cannot be
  enabled lazily after a Page already exists?
- How do proxy, service worker, cache, and offline policies interact
  deterministically?

### Learner checkpoint

- Question 1 answer: Pages in one Context should share cookies. Correct; that
  shared session state is one reason Context is the isolation boundary.
- Question 2: unknown. Reinforcement: storageState is a selected portable
  snapshot, not a complete Chrome profile; it does not capture all cache,
  service-worker, extension, preference, and browser-profile state.
- Question 3 answer: setting coordinates does not necessarily grant access.
  Correct; the origin also needs geolocation permission.
- Question 4 answer: HTTP credentials cannot fill an ordinary application login
  form. Correct; they answer an HTTP authentication challenge.
- Question 5 answer: viewport alone is not complete phone emulation. Correct;
  touch, mobile layout behavior, user agent, scale, engine, OS, and hardware
  signals remain separate.

No Pwright decision is selected in this lesson.

## Lesson 11: Events, dialogs, workers, bindings, and exposed functions

### Events express asynchronous browser state changes

Page, BrowserContext, Browser, Worker, WebSocket, and other objects emit typed
events for lifecycle and activity. Persistent listeners observe a stream until
removed; `waitForEvent` installs a temporary listener, optionally filters it,
and resolves once. For an event caused by an action, the wait must be armed
before the action to avoid a race.

Playwright lazily enables selected protocol event subscriptions when the first
client listener appears and disables them after the last listener is removed.
This avoids producing and transporting some high-volume events nobody consumes,
but listener removal and connection cleanup must be reliable.

### Dialogs block page execution

JavaScript `alert`, `confirm`, `prompt`, and `beforeunload` dialogs can stall
page evaluation and actions until handled. With no interested listener,
Playwright normally auto-handles them to prevent accidental hangs. Once a user
registers a dialog listener, that listener must accept or dismiss each dialog.
A prompt may be accepted with text; a dialog can be handled only once.

`beforeunload` is special because accepting allows navigation/closure while
dismissing cancels it. Asking Page.close to run before-unload handlers can
therefore turn close into an interaction that needs dialog handling.

### Workers have independent execution contexts

A dedicated Web Worker belongs to a Page; a Service Worker belongs to a
BrowserContext/origin scope and may outlive an individual Page. Worker objects
expose URL, lifecycle, console, and JavaScript evaluation in the worker's own
context. They do not have a DOM or Page document.

Worker Handles and evaluation values follow the same execution-context and
disposal constraints as Page handles. Service workers also affect network
routing, caching, offline behavior, and request attribution.

### exposeFunction creates a page-to-client RPC

`exposeFunction(name, callback)` installs a callable function in page frames.
When page JavaScript calls it, arguments are serialized to Playwright, the
client callback runs, and its result or error is serialized back to the page as
the resolution or rejection of a Promise. Context-level exposure applies to
existing and future Pages; Page-level exposure is narrower.

```text
page JavaScript -> binding call -> Playwright client callback
page Promise    <- result/error <- callback completion
```

`exposeBinding` additionally supplies source metadata such as the calling Page,
Frame, and Context. Exposed callbacks cross a trust boundary: untrusted page
code can invoke them concurrently, repeatedly, with large data, or during
teardown. Names, authorization, argument limits, concurrency, cancellation,
reentrancy, and disposal all require policy.

### Source anchors

- Public Page/Context/Worker/Dialog APIs: `packages/playwright-core/types/types.d.ts`
- Lazy subscriptions: `packages/playwright-core/src/client/channelOwner.ts`
- Dialog client/server lifecycle: `packages/playwright-core/src/client/dialog.ts`
  and `packages/playwright-core/src/server/dialog.ts`
- Worker client lifecycle: `packages/playwright-core/src/client/worker.ts`
- Binding dispatch: `packages/playwright-core/src/client/page.ts`,
  `packages/playwright-core/src/client/browserContext.ts`, and
  `packages/playwright-core/src/server/page.ts`

### Questions raised for Pwright

- Which event types must be lossless, coalescible, or droppable under pressure?
- Can a Rust Stream/subscription API make removal and cancellation more
  deterministic than callback listener bookkeeping?
- Which dialog policy is a core invariant versus configurable behavior?
- Are Worker evaluation and exposed functions optional JavaScript capabilities?
- How are page-to-host callbacks authenticated, bounded, cancelled, and drained
  during Context shutdown?

No Pwright decision is selected in this lesson.

## Lesson 12: Screenshots, PDF, video, tracing, console, and artifacts

### Screenshots capture visual state on demand

Page and Locator screenshots ask the browser backend to render current pixels
and return binary image data, optionally writing it to a client path. Page
options cover viewport versus full-page capture, clipping, image type/quality,
scale, transparency, animations, caret, style injection, and masking Locators.
Locator screenshot adds actionability-like resolution and element geometry.

A screenshot is not a DOM snapshot. It preserves appearance but not semantic
structure, event listeners, network state, or JavaScript objects. Full-page and
high-DPI captures can allocate large browser surfaces plus encoded and decoded
buffers across backend, runtime, protocol, and client.

### PDF is print rendering and backend-specific

`page.pdf` is Chromium-oriented in Playwright. It renders using print media by
default and supports paper size, margins, orientation, scale, backgrounds,
page ranges, and header/footer templates. Screen and print rendering may differ;
the caller can deliberately emulate screen media before printing.

Like screenshot, PDF may return the entire binary as a client Buffer as well as
supporting a path, so large documents need streaming and size policy in a remote
or memory-constrained architecture.

### Video is configured at Context creation

Video recording applies to Pages created in a BrowserContext configured with a
recording directory and optional size. `page.video()` exposes a Video whose
backing file is an Artifact. The recording must be finalized before its complete
path/content is reliable; orderly Page/Context closure is important. A remote
client cannot assume the server's filesystem path is locally meaningful and
uses `saveAs` to transfer a copy.

### Tracing is a multi-stream diagnostic recorder

Playwright tracing can record API calls and metadata, network resources, DOM
snapshots, ARIA snapshots, screencast screenshots, and optional source files.
It supports start/stop and chunks so a long session can retain only selected
segments. A trace archive can be inspected in Trace Viewer to reconstruct call
timing, logs, page state, and network context.

Tracing is not free observability. Snapshots, screenshots, sources, bodies,
call stacks, logs, and resource deduplication consume CPU, memory, disk, and
protocol bandwidth and may capture secrets. It must remain opt-in, bounded, and
redacted where appropriate.

### Console and page errors cross the execution boundary

Page/Context console events report type, text, source location, timestamp, and
argument JSHandles. Text is cheap compared with retaining live argument handles;
consumers should serialize only what they need and dispose handles. Page errors
represent uncaught exceptions in page JavaScript, which differ from failed
Playwright API calls and network failures.

Historical console/error query APIs are convenient but require explicit
retention bounds and filters such as since-navigation. Unbounded diagnostic
history would turn logging into a memory leak.

### Artifact abstracts runtime-owned binary output

Download, video, trace, and related outputs use an Artifact lifecycle with
completion/failure, save, stream, cancel, and delete operations. A local client
may access a local path; a remote client streams bytes from the runtime to a
chosen local destination. Paths belong to the filesystem on which the artifact
was produced and cannot be treated as portable identifiers.

Artifact operations must define ownership, temporary storage, destination
overwrite policy, byte limits, partial-file behavior, cancellation, context
shutdown, remote disconnect, and cleanup. An artifact may contain screenshots,
source code, credentials, cookies, request bodies, or user data and therefore
inherits security and retention requirements.

### Source anchors

- Page screenshot/PDF/console client API: `packages/playwright-core/src/client/page.ts`
- Artifact protocol: `packages/protocol/spec/artifact.yml`
- Trace protocol: `packages/protocol/spec/tracing.yml`
- Artifact client/server lifecycle: `packages/playwright-core/src/client/artifact.ts`
  and `packages/playwright-core/src/server/artifact.ts`
- Video client: `packages/playwright-core/src/client/video.ts`
- Trace client/recorder: `packages/playwright-core/src/client/tracing.ts` and
  `packages/playwright-core/src/server/trace/recorder/tracing.ts`

### Questions raised for Pwright

- Which screenshot, PDF, video, trace, and console features are separate
  installable capabilities?
- Should all large binary results use one streaming Artifact contract rather
  than materializing Buffers by default?
- What byte, duration, history, and disk quotas apply per Context and artifact
  type?
- How are partial outputs committed atomically and recovered after disconnect?
- Which secrets and page data must tracing and diagnostics redact by default?

No Pwright decision is selected in this lesson.

## Lesson 13: APIRequestContext and browser-independent HTTP

### APIRequestContext is an HTTP client, not a hidden Page

Playwright can send HTTP requests without creating a Page or executing browser
JavaScript. `playwright.request.newContext()` creates an isolated
APIRequestContext. It supports the usual HTTP verbs, arbitrary headers, query
parameters, redirects, retries, authentication, timeouts, JSON, form data,
multipart bodies, and response inspection.

This path does not create a DOM, run page scripts, perform actionability checks,
or obey a Page's CORS policy. It is useful for API tests, preparing server state,
testing endpoints directly, and checking a browser-visible result against a
backend response. It is not equivalent to calling `window.fetch` because the
request originates in the Playwright runtime rather than the web page.

### Cookie ownership determines whether HTTP and browser state interact

There are two important construction paths:

```text
playwright.request.newContext()
  `-- isolated APIRequestContext cookie storage

browserContext.request
  `-- APIRequestContext sharing the BrowserContext cookie storage
```

The isolated form has its own cookies and can export `storageState`. The
BrowserContext-owned form reads the browser context's cookies before a request
and applies compatible response cookies back to that context. This permits an
API login to authenticate a subsequent Page, but it also means a supposedly
read-only API helper can mutate browser session state.

The two forms should therefore not be treated as interchangeable. Cookie-jar
ownership, domain/path matching, SameSite behavior, secure transport, proxy,
credentials, and disposal are observable semantics.

### A response has headers first and a retained body later

An API response exposes status, status text, URL, headers, body bytes, text,
and parsed JSON. Headers may be available before an application chooses to read
the body, but Playwright's current implementation stores the completed response
body in the server-side APIRequestContext so later `body()`, `text()`, and
`json()` calls can retrieve it by response identifier.

```text
request -> runtime HTTP stack -> status/headers/body
                                  |
                                  `-- fetchResponses: Map<response id, Buffer>
                                                   |
client APIResponse -------------------------------+
```

This is convenient but creates a concrete retention rule: each response body
remains in memory until that APIResponse is disposed or its owning
APIRequestContext is disposed. A test that downloads many or large responses
without disposal can therefore grow the Node runtime even when no browser Page
exists. `maxRedirects`, `maxRetries`, and request timeout bound some work, but
they do not impose a total response-byte or context-memory quota.

### Disposal is part of correct usage

`APIResponse.dispose()` asks the runtime to remove the stored body for that
response. `APIRequestContext.dispose()` releases all retained response bodies
and makes the context unusable. The latter is the ownership boundary and should
run during teardown even after failed tests.

For a remote Rust implementation, returning all bodies as in-memory byte arrays
would multiply buffering across the HTTP library, server object store, channel,
and client. Streaming, backpressure, maximum body sizes, spill-to-disk policy,
and early cancellation are therefore protocol concerns rather than optional
optimizations.

### Source anchors

- Protocol surface: `packages/protocol/spec/api.yml`
- Client objects: `packages/playwright-core/src/client/fetch.ts`
- HTTP implementation and retained response map:
  `packages/playwright-core/src/server/fetch.ts`
- Public API and memory/disposal contract:
  `packages/playwright-core/types/types.d.ts`
- BrowserContext cookie-sharing entry point:
  `packages/playwright-core/src/client/browserContext.ts`

### Checkpoint questions

1. Is `APIRequestContext` a browser Page running `fetch`?
2. Which construction path shares cookies with Pages in a BrowserContext?
3. Can an APIResponse body occupy runtime memory before the client calls
   `body()`?
4. What releases one retained response body, and what releases all of them?
5. Why would a streaming response protocol matter even if the public API also
   offers a convenient `bytes()` helper?

### Questions raised for Pwright

- Is browser-independent HTTP a separate crate/capability rather than browser
  core?
- Does a response default to streaming, bounded buffering, or explicit choice?
- How does a shared cookie jar synchronize without races with Page traffic?
- Which quotas apply to response bytes, redirects, retries, concurrency, and
  temporary files?
- Can a client disconnect cancel the upstream HTTP body immediately?

No Pwright decision is selected in this lesson.

## Lesson 14: Test runner, fixtures, projects, retries, reporters, and parallelism

### Playwright Test is optional orchestration around the library

The automation library launches browsers and controls Pages; Playwright Test
discovers tests, constructs fixtures, schedules workers, retries failures,
collects artifacts, and reports results. A program can use `playwright-core`
without the test runner, and an alternative runner can use the automation API.

The separation matters for Pwright: browser semantics should not depend on a
particular test scheduler, while a runner may add strong lifecycle policy on
top of otherwise flexible library objects.

### Fixtures form a dependency graph with two scopes

Fixtures are named resources whose setup functions may depend on other
fixtures. `test.extend()` creates a new test type with additional or overridden
fixtures. The runner resolves the dependency graph, sets up dependencies first,
injects values into the test, and tears resources down in reverse dependency
order.

```text
worker process
  `-- worker fixtures: browser, shared services, worker options
        `-- test fixtures: context -> page -> test body
                              |
                              `-- reverse-order teardown
```

Worker-scoped fixtures are created once for a compatible worker and cannot
depend on test-scoped fixtures. Test-scoped fixtures are created separately for
each test. This prevents a long-lived Browser fixture from retaining a
short-lived Page fixture indefinitely. Automatic fixtures can run even when the
test does not name them, which is useful for logging but can also hide cost.

### Projects are configuration matrices and dependency nodes

A project is a named group of tests plus configuration. Projects commonly vary
browser engine, device emulation, locale, authentication state, or feature
flags. They may depend on setup projects, and teardown projects can run after a
dependency group. Filtering, sharding, repetition, and per-project worker limits
operate on this scheduled test graph.

Projects avoid copying test source for every environment, but multiplication is
real: three browsers by two roles is six executions, each with browser/process,
artifact, and timeout cost.

### Workers isolate execution and failure

Playwright Test uses operating-system worker processes. Tests in a file normally
run in order within one worker while independent files can run in parallel.
Fully parallel and serial modes change scheduling guarantees. After an
unexpected test failure, the runner normally discards the affected worker and
starts a clean one for later work, reducing cross-test contamination from
unknown process state.

A retry is a new test attempt, commonly in a fresh worker environment; it is not
continuation from the failed instruction. Retry-aware policies can collect a
trace only on the first retry or a video only for failures. Retries may reduce
noise, but they can also hide nondeterminism unless the report preserves every
attempt and classifies flaky outcomes.

### Timeouts belong to different operations

The runner has a test timeout covering the test body and fixture work, while
assertions, individual actions, and navigation have their own timeout policy.
Teardown also needs time after a failure. Treating every timeout as one global
duration makes it impossible to distinguish a slow assertion from a hung
fixture or artifact flush.

Cancellation must propagate from the test budget into pending browser commands,
then still allow bounded cleanup. A timeout error without command cancellation
would leave work running after the test result is known.

### Reporters consume runner events

Built-in reporters include terminal formats and machine-readable JSON, JUnit,
HTML, blob, GitHub, and Perfetto output. A custom reporter implements event
callbacks over suites, tests, steps, errors, attachments, standard output, and
the final run. Multiple reporters can consume the same event stream.

Reporters are extension points, but their behavior affects reliability:
blocking callbacks can slow the run, attachments can be huge, and swallowed
reporter errors may hide output failures. A durable event log and a presentation
reporter are conceptually different responsibilities.

### Source anchors

- Test and project public types: `packages/playwright/types/test.d.ts`
- Custom reporter contract: `packages/playwright/types/testReporter.d.ts`
- `test.extend` and test-type composition:
  `packages/playwright/src/common/testType.ts`
- Fixture dependency and teardown execution:
  `packages/playwright/src/worker/fixtureRunner.ts`
- Built-in browser/context/page fixtures: `packages/playwright/src/index.ts`
- Worker scheduling: `packages/playwright/src/runner` and
  `packages/playwright/src/worker`

### Checkpoint questions

1. Must an application use Playwright Test to use Playwright Pages?
2. May a worker-scoped Browser fixture depend on a test-scoped Page fixture?
3. Is a retry a continuation inside the failed test's exact process state?
4. Why does teardown need a separate bounded opportunity after a timeout?
5. What information must remain visible if a retry later passes?

### Questions raised for Pwright

- Is a Rust runner part of the first compatibility target or a separately
  installable layer?
- What fixture trait expresses dependencies, scopes, async setup, and guaranteed
  teardown without dynamic `any` values?
- Can worker isolation use processes by default while allowing lighter tasks?
- Which events are durably recorded before reporter plugins observe them?
- How do retries distinguish infrastructure failure, product failure, and test
  nondeterminism?

No Pwright decision is selected in this lesson.

## Lesson 15: Channel protocol, generated schema, dispatchers, and transports

### Playwright uses two different protocol boundaries

It is important not to call every message “CDP.” Playwright first has its own
object/channel protocol between a language client and the Playwright runtime.
The runtime then speaks a browser-specific protocol to Chromium, Firefox, or
WebKit.

```text
language API
    |
    | Playwright channel protocol
    v
Playwright runtime/driver
    |
    | CDP, Firefox protocol, or WebKit protocol
    v
browser
```

The channel layer creates one portable object model even though browser
backends differ. CDP exists only on the lower Chromium path and is also exposed
as an expert escape hatch.

### The schema is the source of truth

YAML files in `packages/protocol/spec` declare channel interfaces, methods,
events, parameters, results, enums, object references, and binary fields. The
generator reads the full schema and emits TypeScript channel declarations,
validators, structure types, and protocol metadata.

This catches mismatches before dispatch and keeps client/server declarations in
sync. It also makes protocol evolution centrally coordinated: adding a method
is not just adding a handler; its schema, validation, lifecycle, generated
types, compatibility, and every language binding must agree.

### Messages carry command correlation and object identity

The conceptual wire shapes are:

```text
command:  { id, guid, method, params, metadata }
response: { id, result | error, log }
event:    { guid, method, params }
```

`id` correlates one command with one response. `guid` names the target protocol
object. Object parameters and results are references to other GUID-backed
objects rather than serialized copies. Metadata carries call context used for
tracing, diagnostics, and cancellation.

Special lifecycle messages create, adopt, and dispose objects. Adoption moves
an object under a different protocol owner without inventing a new browser
object. Disposal removes it from the connection maps and invalidates future
calls. Abort/wait-info control messages coordinate long-running calls and
diagnostic progress.

### Client ChannelOwners mirror server Dispatchers

On the client, a Connection owns a map from GUID to `ChannelOwner` and another
map from command ID to pending callback. A ChannelOwner exposes a proxy channel,
validates outgoing parameters and incoming results/events, and participates in
an ownership tree.

On the server, `DispatcherConnection` maps GUIDs to Dispatchers and server
objects to their dispatchers. It validates the command, locates a dispatcher,
creates progress/cancellation instrumentation, runs the method, serializes its
result, and sends either a response or structured error.

This mirroring has a cost: one logical browser object may have a browser-side
object, runtime model object, server Dispatcher, serialized GUID reference,
client ChannelOwner, and public wrapper. Lifetimes must agree across all of
them.

### Transport is deliberately thinner than semantics

The same channel messages can travel in process, through stdin/stdout pipes, or
over WebSocket. Remote JSON transports encode binary data for JSON, while local
paths can use native buffers. Transport close rejects pending callbacks and
causes connection-wide cleanup; it cannot silently reconnect and pretend object
GUIDs still refer to the same runtime state.

Large bodies and artifacts expose a weakness in request/response-shaped RPC:
if the protocol materializes one giant result, transport choice alone does not
provide streaming, quotas, or backpressure.

### Source anchors

- Protocol schema: `packages/protocol/spec/*.yml`
- Schema loader: `utils/protocol_spec.js`
- Generator: `utils/generate_channels.js`
- Generated validation: `packages/protocol/src/validator.ts`
- Generated channel types: `packages/playwright-core/src/client/channels.d.ts`
  and `packages/playwright-core/src/server/channels.d.ts`
- Client connection/object model:
  `packages/playwright-core/src/client/connection.ts` and `channelOwner.ts`
- Server dispatch: `packages/playwright-core/src/server/dispatchers/dispatcher.ts`

### Checkpoint questions

1. Is the Playwright channel protocol the same thing as CDP?
2. What does a message GUID identify, and what does a command ID identify?
3. Why can a Page not be serialized as a plain JSON value and recreated later?
4. What should happen to pending calls and object references when transport
   closes?
5. Does changing WebSocket to a pipe automatically solve large-body memory use?

### Questions raised for Pwright

- Which parts of the portable API require stable, versioned wire contracts?
- Can capability schemas be generated independently and composed without one
  monolithic registry?
- How are object ownership, adoption, leases, and disposal represented in Rust
  types and on the wire?
- Does every command carry cancellation and deadline information end to end?
- Which payloads require stream channels rather than unary RPC results?

No Pwright decision is selected in this lesson.

## Lesson 16: Chromium CDP, Firefox Juggler, WebKit protocol, and Electron/Android

### One public model is implemented by different backend adapters

Playwright's shared server layer defines Browser, BrowserContext, Page, Frame,
network, input, and artifact semantics. Browser-specific directories implement
those abstractions with different launchers, connections, sessions, and
delegates.

```text
shared Page/Frame/Context semantics
       |               |               |
 Chromium adapter   Firefox adapter   WebKit adapter
       |               |               |
      CDP         patched Firefox    Playwright WebKit
                     protocol           protocol
```

Portable behavior therefore comes from translation and normalization, not from
all browsers natively speaking one protocol.

### Chromium uses CDP

Chromium's adapter maintains CDP connections and sessions for browser targets,
pages, frames, workers, network requests, and execution contexts. It can launch
Playwright's pinned Chromium build or attach to an existing Chromium browser
through `connectOverCDP`.

CDP attachment is Chromium-only and has lower fidelity than connecting to a
Playwright Server because Playwright did not create and configure every piece
of browser state. Raw CDPSession APIs are useful for Chromium-specific domains,
but code using them has left the cross-browser contract.

### Firefox and WebKit do not use CDP

Firefox uses a Playwright-controlled browser build with a Firefox-specific
automation protocol often called Juggler in Playwright architecture discussions.
WebKit uses a Playwright WebKit protocol and controlled WebKit build. Their
connection/session code is analogous in responsibility to the Chromium adapter,
not wire-compatible with CDP.

Pinned and patched browser builds let Playwright deliver features and fixes that
ordinary installed browsers may not expose. The cost is browser distribution,
disk use, version coupling, update work, and the need to test each backend as a
real implementation. An emerging BiDi path may become another adapter, but it
does not erase current backend differences.

### Compatibility is an intersection plus explicit exceptions

Most Page/Locator/Context operations target a portable contract, yet some
features remain backend-specific. Examples include Chromium CDP sessions,
Chromium-oriented PDF generation, service-worker coverage, launch arguments,
and browser-specific media or permission behavior. A portable API should expose
capability discovery or precise unsupported errors rather than silently emulate
incorrect behavior.

The backend owns more than method translation: it also normalizes target
lifecycle, request attribution, frame attachment, dialogs, input coordinates,
execution contexts, downloads, crashes, and browser-process cleanup.

### Electron and Android add different object models

Electron support is Chromium-based but adds an ElectronApplication that can
evaluate in Electron's main process and discover renderer Pages/windows. Native
operating-system UI is not automatically equivalent to a web Page.

Android support combines device control with browser automation. It can list
devices, interact with Android UI selectors, install applications, forward
sockets, and connect to Chrome or WebViews. This requires an Android driver and
ADB-style lifecycle in addition to a browser backend. Both are marked as more
specialized/experimental surfaces and should not be forced into browser core.

### Source anchors

- Shared browser abstractions: `packages/playwright-core/src/server/browser.ts`,
  `browserContext.ts`, `page.ts`, and `frames.ts`
- Chromium backend: `packages/playwright-core/src/server/chromium`
- Firefox backend: `packages/playwright-core/src/server/firefox`
- WebKit backend: `packages/playwright-core/src/server/webkit`
- Pinned browser revisions: `packages/playwright-core/browsers.json`
- Electron support: `packages/playwright-core/src/server/electron`
- Android support: `packages/playwright-core/src/server/android`

### Checkpoint questions

1. Do Firefox and WebKit receive Playwright commands through CDP?
2. What does the browser adapter normalize besides method names?
3. Why can `connectOverCDP` have lower fidelity than Playwright `connect`?
4. Should a Chromium-only feature pretend to succeed on Firefox?
5. Why are Electron and Android poor candidates for unconditional browser-core
   dependencies?

### Questions raised for Pwright

- What is the smallest backend trait that supports shared semantics without
  becoming one enormous interface?
- Are engine adapters separate crates loaded at compile time, runtime, or both?
- How does capability discovery distinguish unsupported, unavailable, and not
  installed?
- Can users supply a system Chromium while pinned builds remain optional?
- Which compatibility tests run identically against every backend, and where do
  backend-specific suites begin?

No Pwright decision is selected in this lesson.

## Lesson 17: Installation, driver processes, remote server, and language clients

### Package installation and browser installation are separate operations

Playwright ships library/driver code through language packages, while browser
binaries are managed by its browser registry and install commands. The registry
pins known browser revisions and platform downloads. `playwright install` fetches
selected browsers; `install --with-deps` can also install operating-system
dependencies; `install-deps` handles those dependencies separately.

This avoids assuming every machine already has compatible engines, but package
installation can still imply several large browser downloads. Cache location,
shared installations, garbage collection, offline mirrors, proxies, platform
support, checksum trust, and browser selection are part of the product design.

### Non-JavaScript clients launch a Node driver

JavaScript normally loads the client/runtime into the Node environment. Python,
Java, and .NET packages expose native-language APIs but delegate core Playwright
semantics to the Node driver. The package starts the hidden `run-driver` command
and exchanges Playwright channel messages over stdin/stdout pipes.

```text
Python / Java / .NET client process
            |
            | pipe: Playwright channel protocol
            v
        Node driver process
            |
            | browser-specific protocol
            v
        browser process tree
```

The language wrapper is therefore not an independent reimplementation of all
auto-waiting and browser behavior. Its version must be compatible with the
driver protocol and bundled runtime. Process startup, stderr handling, crash
diagnostics, shutdown, and orphan prevention are required parts of the binding.

### A browser is itself a process tree

Launching Chromium commonly creates a browser parent plus renderer, GPU,
network, crash-handler, and utility processes. Firefox and WebKit have their own
multi-process arrangements. Saying “Playwright uses 1 GB” without separating
client, Node driver, browser parent, renderer, GPU, and shared-memory accounting
does not identify the owner of that memory.

The driver launches the browser with pipes/sockets, profiles, environment,
arguments, temporary directories, and artifact directories, then monitors exit.
Graceful close differs from kill; a browser crash differs from driver crash;
disconnecting from an externally owned browser must not always kill it.

### Playwright Server makes the channel remote

`browserType.launchServer()` launches a browser and exposes a WebSocket endpoint.
A compatible client uses `browserType.connect()` to speak the Playwright channel
protocol to that server. The server can prelaunch and share browser resources,
while clients create their own contexts according to server policy.

Remote operation changes path and ownership semantics. A file chosen on the
server is not automatically present on the client. Uploads, downloads, videos,
traces, and certificates may need byte transfer. Network trust, authentication,
tenant isolation, connection limits, version compatibility, quotas, and cleanup
after disconnect become server concerns.

`connectOverCDP()` is a separate Chromium attachment path. It connects the
runtime to an existing CDP endpoint; it is not a remote Playwright Server and
does not expose the same portable fidelity.

### Source anchors

- Browser registry: `packages/playwright-core/browsers.json` and
  `packages/playwright-core/src/server/registry`
- Install commands: `packages/playwright-core/src/cli/program.ts`
- Out-of-process driver startup:
  `packages/playwright-core/src/outofprocess.ts`
- Hidden driver entry point: `packages/playwright-core/src/cli/driver.ts`
- Browser launch/process ownership:
  `packages/playwright-core/src/server/browserType.ts`
- Remote server: `packages/playwright-core/src/remote/playwrightServer.ts`
- Server launch/client connect: `packages/playwright-core/src/browserServerImpl.ts`
  and `packages/playwright-core/src/client/connect.ts`

### Checkpoint questions

1. Does installing a Python Playwright package mean Python reimplements the
   browser runtime?
2. Is a Playwright Server WebSocket the same protocol endpoint as Chromium CDP?
3. Can a remote client's local path be passed directly as a server artifact
   path?
4. Which processes must be measured separately in a memory investigation?
5. When a client disconnects from an externally owned browser, should the
   browser always be killed?

### Questions raised for Pwright

- Can Rust be both the native runtime and a language-neutral server, eliminating
  a mandatory Node driver?
- Which crates install with no browser, and which commands explicitly acquire an
  engine?
- How are downloaded engines verified, cached, selected, updated, and removed?
- What authentication, tenancy, quotas, and version negotiation protect a
  remote Pwright server?
- How are server-local paths replaced with typed upload/download/artifact
  resources?

No Pwright decision is selected in this lesson.

## Lesson 18: Resource lifecycle, object retention, memory, and failure handling

### Memory belongs to a graph, not one object

A Page participates in several connected graphs: browser targets and execution
contexts, runtime models, server Dispatchers, protocol maps, client
ChannelOwners, event listeners, pending callbacks, network records, Handles,
artifacts, and test-runner fixtures. Dropping one public variable does not prove
every layer released the object.

```text
browser process objects
       <-> runtime Page/Frame/Request models
       <-> Dispatcher + GUID map
       <-> ChannelOwner + client GUID map
       <-> user listeners/handles/tasks
```

Correct cleanup needs a defined ownership hierarchy plus explicit termination
signals across process and transport boundaries.

### Handles deliberately retain remote JavaScript objects

JSHandle and ElementHandle represent live browser objects. As long as a handle
is valid, the browser may need to keep its underlying remote object alive and
both client and runtime keep protocol entries. Navigation, frame detach, context
destruction, explicit `dispose`, or connection close may invalidate it.

Playwright includes dispatcher GC safety thresholds. Current code allows buckets
up to 100,000 JSHandles and 100,000 ElementHandles, while many other dispatcher
types use 10,000. When a bucket exceeds its threshold, the runtime disposes a
fraction of the oldest entries. This prevents completely unbounded maps, but it
is a late safety valve, not precise ownership; tens of thousands of live handles
can already be expensive and GC disposal may surprise code retaining old ones.

A concrete network case is now verified in the local Playwright
`1.63.0-next` checkout at commit `de214f440b7e34937fe4886f046b78b757136087`.
`RequestDispatcher` and `ResponseDispatcher` use their type names as the default
GC buckets. `requestFinished` and `requestFailed` dispatch events through the
existing Request/Response dispatchers but do not dispose them. Therefore a
connection can retain roughly 10,000 Request dispatchers and 10,000 Response
dispatchers before the generic threshold trims the oldest 10% of an overflowing
bucket. This is a concrete plausible contributor to long-running Playwright
memory growth, not proof that it is the only or dominant cause.

Pwright's base network-events plugin now makes a different, explicit tradeoff.
While a request is active it may expose temporary Request/Response identities,
but `requestfinished` and `requestfailed` carry complete immutable Request and
optional Response value snapshots. Once every selected bounded event queue has
accepted that terminal batch, the runtime disposes the Request root and its
Response child immediately. A real Chrome persistent-Page test holds the
Browser/Context/Page/Frame baseline at `[4, 4, 4]` across three completed
requests. Code that needs Playwright-compatible post-terminal methods must opt
into a future acknowledged handle-lease capability and pay its declared object
and byte budget.

### Several non-handle stores can retain large data

- APIRequestContext retains completed response bodies until response/context
  disposal.
- Network and diagnostic features retain request metadata, headers, bodies,
  console arguments, errors, and timing.
- Tracing, HAR, video, screenshots, and downloads retain buffers, snapshots,
  temporary files, streams, and deduplication tables.
- Pages retain Frames and Workers; Contexts retain Pages, service workers,
  routes, bindings, permissions, downloads, and background state.
- Pending commands and event queues retain closures and referenced objects while
  waiting for a result or consumer.

An idle-looking client may therefore still own a large browser graph. Metrics
need counts and bytes by resource class, not only one process RSS value.

### The browser often dominates total memory

Modern browsers isolate sites and services into multiple processes. Renderer
heaps, image surfaces, media, GPU buffers, browser caches, JavaScript heaps, and
shared memory may outweigh the driver. RSS can double-count shared pages across
processes, while process-manager numbers may omit memory mapped elsewhere.

A useful investigation records at least client RSS, driver RSS/heap, browser
parent and child RSS/PSS, number of contexts/pages/frames/workers/targets,
handle/dispatcher counts, pending commands, queued events, retained body bytes,
artifact bytes, and values before/after deterministic teardown.

### Close is a state machine, not a destructor call

The normal hierarchy closes Pages/Contexts before Browser so routes and
listeners stop, pending operations cancel, videos and traces finalize, downloads
settle, temporary files clean up, protocol objects dispose, and browser processes
exit. Each step needs a deadline and idempotent behavior.

Failures are not interchangeable:

- Page close invalidates objects scoped to that Page.
- Frame detach invalidates its execution contexts and Handles.
- Browser crash means graceful protocol cleanup may be impossible.
- Transport loss rejects pending calls and leaves server-side cleanup to lease
  or disconnect policy.
- Timeout/cancellation should abort one operation without corrupting unrelated
  objects.
- Client process death provides no language-level destructor opportunity.

Rust RAII helps only inside one process. A dropped Rust value cannot by itself
guarantee a remote message was delivered, so explicit async close plus server
leases/finalizers remain necessary.

### Source anchors

- Dispatcher ownership and GC thresholds:
  `packages/playwright-core/src/server/dispatchers/dispatcher.ts`
- Client object and pending-callback maps:
  `packages/playwright-core/src/client/connection.ts`
- Handle disposal: `packages/playwright-core/src/client/jsHandle.ts` and
  `packages/playwright-core/src/server/javascript.ts`
- API response retention: `packages/playwright-core/src/server/fetch.ts`
- Context close coordination:
  `packages/playwright-core/src/server/browserContext.ts`
- Trace resource retention:
  `packages/playwright-core/src/server/trace/recorder`

### Checkpoint questions

1. Does dropping the last user variable for a JSHandle immediately prove the
   browser remote object was released?
2. Is the 100,000-handle dispatcher threshold a precise lifetime design?
3. Why should browser child processes be measured separately from the driver?
4. Can Rust `Drop` guarantee cleanup of a server object after network loss?
5. Which counters would distinguish handle leakage from response-body retention?

### Questions raised for Pwright

- What is the explicit ownership tree and close state machine for every resource?
- Which resources use RAII only, explicit async close, leases, or all three?
- What hard byte/count/concurrency budgets exist per client, Context, and server?
- How are event streams bounded, coalesced, backpressured, and cancelled?
- Can built-in metrics attribute memory to browser, runtime, protocol objects,
  bodies, and artifacts before a production leak investigation is needed?

No Pwright decision is selected in this lesson.

## Lesson 19: Extension points and the limits of Playwright composability

### Playwright is extensible at several policy boundaries

Users can customize important behavior without forking Playwright:

- `selectors.register()` installs a selector engine evaluated in page contexts.
- Locators compose queries, filters, frames, and assertions at the API level.
- `page.route()` and `context.route()` install request interception policy; HAR
  replay is implemented on this mechanism.
- `exposeFunction()` and `exposeBinding()` add page-to-host calls.
- `test.extend()` composes fixture graphs and reusable test types.
- Custom reporters consume runner events and attachments.
- Projects, devices, launch options, proxies, headers, and storage state compose
  configuration.
- CDPSession exposes Chromium-specific protocol commands as an escape hatch.

These are valuable extension points, but “extensible behavior” is not the same
as “independently installable capability.”

### The engine and protocol core remain centrally assembled

Browser backends live inside the main runtime build. A third party cannot
register an arbitrary new browser backend through the same public mechanism as
a selector engine. The portable schema is generated as one coordinated channel
protocol, and Dispatchers, validators, client wrappers, and language bindings
must understand its object types.

Likewise, Page, network, tracing, screenshots, API requests, server modes, and
many utilities ship as part of the runtime/package graph even when a user only
needs a subset. Browser downloads can be selected, but that is coarser than
installing only Locator plus Chromium CDP plus screenshots.

### Hooks have different trust and failure properties

A selector engine runs inside the page and can be affected by hostile DOM or
page code. A route callback runs in the client/runtime path and can stall a
request indefinitely. A binding lets page code invoke host behavior. A fixture
can retain worker-wide resources. A reporter can process secrets and large
attachments. A CDP command bypasses portability guarantees.

Calling all of these “plugins” would hide essential differences. Each extension
surface needs an execution location, typed inputs/outputs, capability grant,
deadline, concurrency policy, panic/error isolation, version contract, cleanup,
and resource budget.

### Composability has several independent meanings

For Pwright, the phrase “everything is composable and pluggable” must be split
into testable design dimensions:

| Dimension | Question |
| --- | --- |
| Packaging | Can a user compile/install only the required capability? |
| Dependency | Can a capability avoid pulling unrelated browser/runtime code? |
| Construction | Can typed builders combine capabilities without global state? |
| Runtime | Can a server discover/load a capability safely after compilation? |
| Protocol | Can capability messages evolve without rebuilding one global schema? |
| Policy | Can behavior be overridden through hooks or middleware? |
| Observability | Can cost and failures be attributed to the responsible plugin? |
| Lifecycle | Can unload/cancel/close release every plugin-owned resource? |

A design can succeed in one dimension and fail in another. Cargo features offer
compile-time packaging but not safe runtime loading. Trait objects offer runtime
substitution but do not define wire compatibility. Middleware composes policy
but cannot automatically add a new browser protocol object family.

### Source anchors

- Custom selectors: `packages/playwright-core/src/client/selectors.ts` and
  `packages/protocol/spec/browserContext.yml`
- Network routing: `packages/playwright-core/src/client/page.ts`,
  `browserContext.ts`, and `harRouter.ts`
- Bindings: `packages/playwright-core/src/client/page.ts` and
  `packages/playwright-core/src/server/page.ts`
- Fixture extension: `packages/playwright/src/common/testType.ts`
- Reporter extension: `packages/playwright/types/testReporter.d.ts`
- Chromium escape hatch:
  `packages/playwright-core/src/client/cdpSession.ts`
- Central protocol generation: `utils/generate_channels.js`
- Compiled browser backends: `packages/playwright-core/src/server/chromium`,
  `firefox`, and `webkit`

### Checkpoint questions

1. Does registering a selector engine mean Playwright can load a new browser
   backend at runtime?
2. Is an optional browser download the same as a capability not being linked or
   installed?
3. Why should a route handler and a reporter not share one vague Plugin trait?
4. What composability dimensions do Cargo features solve, and which do they not?
5. What must happen to plugin-owned tasks and resources when a Context closes?

### Questions raised for Pwright

- Which extension classes exist, and what narrow trait belongs to each class?
- Is runtime dynamic loading required initially, or are separately installable
  crates plus compile-time composition sufficient?
- How does a capability declare dependencies, permissions, wire schema, quotas,
  and lifecycle hooks?
- Can generated protocol fragments be namespaced and negotiated independently?
- Which core invariants are deliberately not overrideable by plugins?

No Pwright decision is selected in this lesson.

## Lesson 20: Compatibility inventory and candidate lessons for Pwright

### Compatibility must be defined by observable behavior

“Compatible with Playwright” can mean API naming, source compatibility, wire
compatibility, or behavior compatibility. They are different promises. A Rust
API cannot be source-compatible with TypeScript, but it can preserve the same
object scopes, action semantics, event ordering, timeout behavior, errors, and
artifact outcomes. Speaking Playwright's private channel protocol is another,
much stricter goal and may couple releases tightly.

The first design task is therefore an inventory with explicit status rather
than a claim of complete compatibility.

| Capability family | Important observable contract | Likely separable boundary |
| --- | --- | --- |
| Browser acquisition | launch/connect/persistent ownership and close | engine launcher/connector |
| Context isolation | cookies, storage, permissions, proxy, emulation | browser-context core plus options |
| Page and Frame | identity, navigation, popup, lifecycle, events | portable page model |
| Locator | lazy re-resolution, strictness, frame scope | query/selector capability |
| Actions | actionability, scrolling, input, one deadline | action policy plus input backend |
| Assertions | repeated observation and diagnostics | optional assertion layer |
| Evaluation | serialization, contexts, Handles, disposal | JavaScript capability |
| Network | events, routing, bodies, WebSocket, HAR | network capability family |
| State/emulation | cookie/storage schemas and browser overrides | context options/capabilities |
| Artifacts | streaming, finalization, remote ownership | common Artifact protocol plus codecs |
| HTTP client | cookies, redirects, retained/streamed bodies | independent request capability |
| Test runner | fixtures, workers, retries, projects, reporters | separate runner product |
| Browser backends | normalize engine protocols and process lifecycle | per-engine crates |
| Remote operation | versioning, auth, quotas, streams, disconnect | server/transport layer |
| Diagnostics | tracing, console, metrics, failure attribution | opt-in observability capabilities |

Each row still needs a support level such as required, later, intentionally
different, backend-specific, or out of scope. That status belongs in the design
tasks and compatibility specification, not in marketing language.

### Candidate lessons from Playwright

The following are evidence-backed candidates to evaluate, not approved Pwright
architecture decisions:

1. Preserve the Browser -> BrowserContext -> Page -> Frame ownership model
   unless a concrete Rust alternative improves an identified behavior.
2. Keep Locator lazy and re-resolving; do not make ElementHandle the default
   element abstraction.
3. Treat actionability, assertions, cancellation, deadlines, and diagnostics as
   defined semantics rather than client-side convenience loops.
4. Separate portable Playwright-like semantics from browser-specific protocols;
   CDP is a Chromium backend and escape hatch, not the universal public API.
5. Make BrowserContext the primary isolation and cleanup unit.
6. Model large bodies and artifacts as bounded streams with optional collection,
   not unconditional in-memory results.
7. Give every remote object an explicit owner, terminal state, close path, and
   disconnect policy; do not rely only on Rust `Drop`.
8. Instrument resource counts and bytes from the beginning so browser memory,
   runtime memory, handles, responses, events, and artifacts can be separated.
9. Keep the test runner optional so automation core is useful without runner
   dependencies.
10. Define several narrow extension contracts rather than one universal Plugin
    interface.
11. Separate compile-time package composition, runtime backend substitution,
    protocol negotiation, and policy hooks in both vocabulary and tests.
12. Do not bundle or download a browser unless the selected backend requires it
    and the user has explicitly chosen the acquisition policy.

### Applied design note: cleanup retry is not command retry

The lifecycle lessons imply an important distinction for Pwright. Retrying a
high-level read or a Locator observation can be part of its defined semantics;
replaying an already-started resource cleanup may duplicate a consuming side
effect. An owned Browser shutdown, for example, transfers unique process
authority on first poll and cannot be reconstructed from a cloned request.

The optional Tokio cleanup policy therefore treats every ordinary or reserved
continuation as one attempt. Retry exists only when the application explicitly
provides a factory that creates a fresh cleanup future for each attempt and a
finite maximum. The same bounded executor permit spans all attempts and delay.
Application shutdown closes new admission, waits for preaccepted reservations
and jobs, and may use a separate drain deadline that observes without silently
cancelling work. Reporting is another independent synchronous plugin rather
than a field added to the portable Browser result. This applies lessons 7, 8,
10, and 11 without claiming that Playwright's internal executor has the same
API.

### Candidate differences that require experiments

Some goals cannot be settled from source reading alone:

- Whether a native Rust runtime materially reduces non-browser memory compared
  with the Node driver under equivalent workloads.
- Which fraction of reported memory is browser process cost versus Playwright
  object/body/artifact retention.
- Whether direct system-Chromium CDP can match the required portable feature set
  without a patched browser.
- How much latency and allocation a typed Rust channel adds locally and remotely.
- Whether compile-time crate composition is sufficient for users, or runtime
  dynamic plugins are an actual requirement.
- Which Playwright edge cases users depend on and which are accidental behavior.

Each question needs a reproducible benchmark or compatibility test with process
and resource attribution. Anecdotal total-memory numbers are not enough.

### Source anchors

Lesson 20 synthesizes the preceding source studies. The primary cross-cutting
anchors are:

- Public automation contract: `packages/playwright-core/types/types.d.ts`
- Public test-runner contract: `packages/playwright/types/test.d.ts`
- Portable protocol schema: `packages/protocol/spec`
- Runtime object lifecycle:
  `packages/playwright-core/src/server/dispatchers/dispatcher.ts`
- Browser backend implementations: `packages/playwright-core/src/server/chromium`,
  `firefox`, and `webkit`
- Browser acquisition data: `packages/playwright-core/browsers.json`
- Client connection/object lifecycle:
  `packages/playwright-core/src/client/connection.ts`

### Design artifacts to produce before implementation

The course now supplies input for, but does not replace, the remaining design
work:

1. A capability inventory with required/deferred/out-of-scope status.
2. A compatibility contract defining API, behavior, and wire goals separately.
3. Ownership and lifecycle state machines for Browser, Context, Page, Frame,
   Handle, Request/Response, stream, and Artifact.
4. A crate/dependency graph proving minimal installations do not pull unrelated
   capabilities.
5. Narrow extension contracts and their trust/resource models.
6. A versioned protocol and streaming/backpressure design.
7. Browser acquisition and backend-selection policy.
8. A resource budget and observability specification.
9. Failure, cancellation, retry, and remote-disconnect semantics.
10. Benchmarks and cross-backend conformance suites with explicit acceptance
    thresholds.

Only after these artifacts are reviewed should implementation tasks be split
into milestones and code. This preserves the user's original constraint: design
the problem and its decisions before starting the rewrite.

### Final review questions

1. Which Playwright behavior category is the first compatibility milestone?
2. Does “pluggable” initially require runtime dynamic loading, or only separately
   installable and composable Rust crates?
3. Is using an existing system Chromium a default, an option, or unsupported?
4. Which measured memory budget applies separately to runtime and browser
   processes?
5. Which candidate lesson above needs an experiment before it can become a
   decision?

No final Pwright architecture decision is selected by this course. The next
step is to turn the inventory and candidate lessons into explicit design options,
tradeoffs, experiments, and acceptance criteria.
