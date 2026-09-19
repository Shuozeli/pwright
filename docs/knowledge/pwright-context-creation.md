<!-- agent-updated: 2026-09-10T10:36:28Z -->

NODE-119: Native ready events now publish Context.page, then their ordered popup linkage, using the precommit converter. New Context membership can be published independently while new Page descriptors remain hidden until ready. Creation rollback restores unused ready-source capacity; a throwing Context.page callback suppresses its dependent popup even when linkage arrives later.
[Native ready-source contract and evidence](pwright-node-lifecycle-events.md#node-119-native-ready-and-popup-source).

NODE-118: The addon can now prepare a missing Context/Page from a native ready notice before publication, preserving existing wrapper identity and Context settings. JS cache insertion occurs only after Rust commits. Context.page and Page.popup native production remain pending; conversion support alone is not public feature completion.
[Creation-conversion contract and evidence](pwright-node-lifecycle-events.md#node-118-precommit-creation-wrapper-conversion).

NODE-117: Creation/discovery admission now replenishes external-close capacity before adding visible objects; abandoned explicit creation restores prior capacity. Native Context disposal produces Page groups then Context close. This does not implement Context.page or Page.popup creation notifications; external Context cascade has native unit evidence, not a new dedicated live differential case.
[External-close source contract and evidence](pwright-node-lifecycle-events.md#node-117-pre-admitted-external-close-source).

NODE-116: Successful owned Context.close now expands the committed native parent disposal into visible Page-close groups followed by Context-close. Empty Contexts also receive their close group. Context/page creation and popup event production remain pending.
[Explicit-close source contract and evidence](pwright-node-lifecycle-events.md#node-116-explicit-close-source-integration).

NODE-115: Terminal Context-close groups now reach Node after the Context's visible Page-close groups. This does not add Context.page, Page.popup, or ordinary explicit Context.close production; those source joins remain pending.
[Terminal dispatcher contract and evidence](pwright-node-lifecycle-events.md#node-115-environment-thread-terminal-dispatch).

# Playwright-aligned BrowserContext creation design

NODE-114: Terminal reservations exist before Context/Page discovery or creation. The selected shutdown producer removes each Context after its visible Pages, including empty Contexts. Ordinary creation and explicit Context-close event sources remain pending.
[Terminal source contract and evidence](pwright-node-lifecycle-events.md#node-114-pre-admitted-native-terminal-source).

NODE-113: PageReady groups expose Context/page membership before context.page and an optional dependent popup. Deferred PopupLinked groups emit no duplicate context.page. ContextClosed requires its Pages already hidden. These are Rust primitives, not selected public Node events.
[Group contract and evidence](pwright-node-lifecycle-events.md#node-113-bounded-dependent-publication-groups).

NODE-112: Owned Context teardown now has an optional pre-admitted native close receipt.
Default/external Context rejection does not create a destruction receipt. A
successful receipt follows descendant and selected network retirement; Context
callback ordering and host journal integration remain incomplete.
[NODE-112 close receipt contract and verification](pwright-node-lifecycle-events.md#node-112-pre-admitted-explicit-close-receipts).

NODE-111: Driver PageReady and PopupLinked effects are now available as borrowed native
input. Explicit Context creation still uses direct topology publication, and
Context close effects need their own committed input. No Context event support
is inferred from a later coherent ownership snapshot.
[NODE-111 source contract and verification](pwright-node-lifecycle-events.md#node-111-borrowed-committed-driver-effects).

NODE-110 proves a synchronous Context page listener failure skips the remainder
of that emit and the dependent opener popup, without rolling back membership.
Later independent creations can still notify when the diagnostic child handles
the exception. This constrains native cause grouping, not JS diff synthesis.
[NODE-110 contracts, fix and verification](pwright-node-lifecycle-events.md#node-110-listener-failure-boundaries-and-scoped-exception-reporting).

NODE-109 adds exclusive Rust publication ownership but does not change creation
visibility or dispatch Context/Page events. A fence captured during a held
collection batch cannot publish its partial changes, even after that batch
commits. The selected fixed queue allowance includes the single active owner.
[NODE-109 contract and verification](pwright-node-lifecycle-events.md#node-109-exclusive-fixed-watermark-publication-owner).

NODE-107 preserves Context metadata through revision rebasing: configured settings
must share the original Arc, while borrowed unconfigured settings share a ledger.
New Contexts/Pages stay hidden until explicitly revealed; this does not dispatch
creation events or authorize reordering native causes.
[NODE-107 contracts and evidence](pwright-node-lifecycle-events.md#node-107-validated-cross-revision-visibility-rebasing).

NODE-106 can construct a hidden immutable revision view and reveal a Context
before revealing its Pages; an invisible parent causes a typed rejection.
Current production creation fences still install full views, not intermediate
creation steps. Context page / popup notifications are not implemented here.
[View foundation and event work](pwright-node-lifecycle-events.md#node-106-immutable-single-revision-visibility-foundation).

NODE-105 proves that correct Context/Page collections do not supply event parity.
Context page events must expose the exact created Page before newPage resolution;
Context close must follow its individual Page-close callbacks. The Node candidate
currently emits neither notification. Both profiles fail the new seven-case event
gate; no production fix is included.
[Callback states and Rust publication design](pwright-node-lifecycle-events.md).

NODE-104 verifies blocked-loop creation and retirement/replacement collection
visibility: single and concurrent Context/Page creation plus Worker-confirmed
borrowed discovery pass 7/7 twice on default and combined NODE-103 artifacts.
Concurrent Page invocation order is not a stable cross-run collection contract.
No production code changed; Context page/close event dispatch is still missing.
[Exact test scope and harness corrections](pwright-frame-publication.md#node-104-creation-and-discovery-publication-characterization).

NODE-103 reserves collection revisions and replenishes retirement capacity before
native Context/Page creation. A Rust guard restores newly replenished capacity
when later admission/native steps fail, with existing exact-object compensation.
Selected Node getters now consume published settings/membership. Rust also emits
TypeError for oversized UTF-8 baseURL at the synchronous N-API boundary, without
catching user header getter exceptions. Both profiles pass baseURL 16/16 plus
four resource checks; creation/discovery blocked-loop timing remains uncharacterized.
[Current implementation and tests](pwright-frame-publication.md#node-103-rust-collection-producers-and-published-node-reads).
The NODE-102 not-yet-connected statements below describe that historical checkpoint.

NODE-102 implements a Rust collection reservation that owns its queue slot,
fixed vectors and Arc allocation before a future producer starts native creation.
Context records retain settings under their existing leases; duplicate/missing
ownership and capacity are validated. Creation/discovery producers are not yet
connected, so this checkpoint does not change newContext behavior or fix
collection timing.
[Reservation contract and integration requirements](pwright-frame-publication.md#node-102-bounded-rust-collection-revision-storage).

NODE-101 distinguishes native commit visibility from language publication.
ContextMetadata::visible and native route enumeration do not preserve Context
membership while Node is blocked during close. The new seven-case oracle passes
twice; default/combined candidates each fail six lifecycle cases. Creation and
discovery timing still require characterization; no creation behavior changed.
[Evidence and published-membership design gate](pwright-frame-publication.md#node-101-context-and-page-collection-publication-discrepancy).

NODE-094 fixes late foreign-Context attachment classification, not Context
creation/disposal authority. Existing registered Contexts retain their mapping;
only unregistered IDs use the selected Playwright default-view fallback.
[Contract and live control](pwright-chromium-topology-events.md#node-094-queued-attachments-from-disposed-contexts).

NODE-084 implements selected `newContext({ extraHTTPHeaders })`: pre-native Rust
input admission, desired-header installation before descriptor publication, and
exact-Context compensation on post-create installation failure. HTTP syntax stays
deferred until Page initialization; creation works alongside baseURL. Header-only
public tests pass 21/21; compensation fault injection remains unproven.
[Current contract and evidence](pwright-extra-http-headers.md#node-084-context-creation-headers).

NODE-083 verifies upstream creation accepts invalid header strings and rejects
later at Page initialization. The Rust setter/model now supports empty Context
storage and clearing, plus masked invalid Context values. Do not validate HTTP
syntax in a future creation-option parser. Creation still needs admitted desired
settings installed before descriptor publication and exact Context compensation;
`newContext({ extraHTTPHeaders })` remains unsupported.
[Characterization and prerequisite fixes](pwright-extra-http-headers.md#node-083-deferred-syntax-and-page-local-initialization-failure).

NODE-082 exposes experimental `context.setExtraHTTPHeaders` through selected
Rust FFI and verifies inheritance by existing/new Pages, Page overrides and
Context clearing. This does not add `newContext({ extraHTTPHeaders })`; creation
option validation/admission and rollback are still required separately.
[Public setter checkpoint](pwright-extra-http-headers.md#node-082-public-selected-setters-and-language-phase-differential).

NODE-081 installs mutable Context/Page headers separately from baseURL metadata.
The native Context setter initializes future Pages and updates existing Page
routes, with real owned-Context wire tests. This does not yet add creation-time
`extraHTTPHeaders` or Node conversion. Tests must create host-owned Contexts:
the compatible CDP fallback can group foreign Pages into the default Context,
so disposing a separately created native Context cannot clean up its new Pages.
[Native scope and fixture correction](pwright-extra-http-headers.md#node-081-selected-rust-host-and-native-session-reconciliation).

NODE-080 implements the standalone admitted header settings registry. It remains
separate from immutable baseURL metadata and is not installed for Context
creation yet. Host creation must seed it before request-capable session startup;
retirement must follow verified native closure, not merely a failed setter.
[Transaction model](pwright-extra-http-headers.md#node-080-bounded-desired-settings-transactions).

NODE-078's optional header snapshots are separate from the existing immutable
baseURL settings. Context creation does not accept extraHTTPHeaders yet. A future
registry must seed original Context headers before request-capable Page/session
initialization and coordinate update/rollback across current and future sessions.
[Header ownership and required integration](pwright-extra-http-headers.md).

> Status: NODE-062 Rust-owned baseURL metadata and Node Page navigation pass
> the pinned live comparison and retention gates. Broader Context options,
> Frame.goto, public Frame lookup and connection reliability remain open.

## NODE-062: bounded Context settings and Node baseURL integration

The Rust host now owns an independent metadata ledger (extension scope 121/4):
16 active registry slots, 256 combined settings/binding states, 128 conversion
slots, 16 KiB per configured baseURL, and 2 MiB logical retained/conversion bytes.
The browser topology's own Context cap still applies; these are upper bounds,
not a promise that 16 additional native Contexts can be created.

Before native creation, reserve a registry slot, the full reservation body,
optional settings text/body/Arc counters and one returned native binding. The
driver request boxes only that already-admitted reservation. Commit settings
after native creation; hide owned routes from enumeration until commit so a
racing reader cannot cache an empty setting. Commit failure compensates by
closing the created Context. Abandonment and any admission failure unwind the
reservation. Close/disconnect and driver topology refresh prune registry entries.

`ContextSnapshot` and the N-API `NativeContextSettings` binding retain metadata
and ledger leases only, never the registry, HostSession, runtime or transport.
Missing and explicitly empty baseURL remain distinct. Closed Context/Page
wrappers retain the original base; last-reference release reclaims it. Native
diagnostic formatting deliberately excludes the URL, which may hold credentials.

Node `browser.newContext({ baseURL })` accepts the one string option, with a
16 KiB pre-copy limit; unimplemented options still fail explicitly. Older addons
reject baseURL instead of silently ignoring it. `page.goto` delegates resolution
to the retained native settings wrapper, using the shared scoped Node WHATWG URL
value port. Pinned Playwright 1.58.2 constructs URL, calls toString, then reads
origin within the fallback try/catch; the glob and navigation consumers now
share that native helper. There is no JS URL algorithm, CDP driver or launcher.
Navigation input and resolved output remain capped at the existing 16 KiB.
Node/V8 URL/parser allocations are foreign-owned and are **not** bounded process
memory under these logical Rust quotas.

`contextSnapshot(knownIDs)` is cache-aware: reserve conversion storage before
copying up to 256 bounded IDs, bind only newly discovered Contexts and return
the complete current ID order. Page and Context conversions each have their
own compile-time size proof against 32 KiB; the two DTO families do not coexist
in one read. `browser.pwrightContextMetadataMetrics()` reports states,
conversions, bytes, activeContexts and pendingCreations independently of Frame
metadata. The fixed Context registry byte charge is separate from the historical
7672-byte Frame baseline.

### Verification status and remaining gates

The resumed verification now passes on Chrome 152.0.7977.83 with the original
observer-session-before layout and unchanged five-second connection deadline.
The final selected addon SHA-256 is
`9c36434e6cd02914b1835e7f04cc716692322f9b5a4ce54da994f77a4d58c7e3`
(`build-final.log`, built before the memory-investigation pause). It includes
the compile-time proof that NativeContextSettings has exactly its pre-admitted
ContextBinding body size. The prior artifact and failed attempts below remain
historical evidence, not current-artifact passes.

- `contexts-final.json`: **16/16** public observations match Playwright 1.58.2;
  **four** internal boundary groups pass. Racing enumeration asserts that every
  newly observed wrapper is one of the actual creation results, rather than
  accepting a vacuous later membership check.
- Invalid/non-string/oversized baseURL and unsupported options reject without
  changing native Context inventory or metadata metrics.
- Retaining **127** closed configured Contexts reaches **255 states** (including
  the cached default binding), **36629 bytes**, zero active entries/conversions/
  pending creations. The next configured creation fails on pre-admission.
- Native URL constructor exceptions preserve fallback; oversized foreign output
  fails; recursive foreign callbacks reach resource exhaustion at observed
  depth **62** and unwind all conversions. Depth 62 is a sample, not the API cap.
- Retained settings still resolve after browser disconnect. Last GC returns
  Context metadata to **0 states / 0 conversions / 560 fixed-index bytes**,
  Frame metadata to **1 state / 0 revisions / 7672 bytes**, and all live host
  resource metrics to zero. These are logical native charges, not whole-process
  RSS measurements or a bound on Node-owned parser memory.
- `pages-resume.json`: public Page regression **7/7**; `glob-resume.json`:
  shared URL-helper glob regression **1647/1647** plus **seven** boundary checks.
  `node-final.log` and `types-final.log`: **43** Node unit tests and strict types pass.

The shared Chrome inventory changed while work was stopped; the unrelated
blank target previously associated with slow Page.enable was no longer present
at preflight. This is not a controlled causal experiment and no connection fix
is claimed. All original timeout reports remain intact. Verification used the
existing serve-lib daemon, not an additional preview-server process; the temporary
fixture route and this task's preflight tab are removed after the run.

Evidence: `.work/node062-context-base/`. Rust selected host: **225 passed / 9
ignored**; default host: **223 passed / 7 ignored**. Twelve metadata tests cover
commit visibility, empty/missing settings, abandonment, binding/count/byte
exhaustion, retained ownership, shutdown/late commit, duplicate IDs, default
contexts and pruning. Strict selected host/addon Clippy, 43 Node unit tests,
TypeScript and dependency/JS-driver boundary checks pass.

Earlier selected addon SHA-256 (before the final compile-time size proof build):
`4a9adda9fe8536710c545542525fef35151d50f56178eb1ece4bc2a25a0ce6f4`.
Historical default artifact remains
`8bb3c7ff91b8b4e545b4b5d7aa9378d137cc79f0384b6d142a8d085deb0083ca`.
No commit or push was added after the foundation commit.

The new supervised `pnpm test:context-base` corpus covers directory/file/query
relative URLs, opaque hashes, empty/invalid/missing bases, concurrent Context
isolation, cached enumeration, retained-settings pressure and final reclamation.
Set loopback `PWRIGHT_TEST_CDP`, selected `PWRIGHT_NATIVE_PROFILE`, a verified
serve-lib fixture directory URL in `PWRIGHT_CONTEXT_BASE_FIXTURE`, and an unused
absolute `PWRIGHT_CONTEXT_BASE_REPORT`. The local fixture directory was registered
as `/pwright-node062-context-base` on the Tailscale serve-lib daemon and its
`empty.html` returned 200; route lifetime is temporary, not a permanent service.

**Historical failures, not discarded:** `contexts-first` fails upstream attachment;
`contexts-second` reaches candidate attachment and fails its topology-initialization
deadline; the labelled `PWRIGHT_CONTEXT_OBSERVER_SESSION=after` experiment also
fails upstream attachment. The refreshed Page regression fails upstream
attachment too. `native-isolated.log` overlapped that regression and is not an
isolation experiment; a subsequent serial native-only attempt also times out.
Chrome is now 152.0.7977.83 with unrelated live application Pages. No shared
browser restart, unrelated Page close, or timeout increase was used.

Those early attempts did not verify live Node baseURL behavior, last-GC/pressure
or the shared-helper glob regression. The resumed results above now cover those
gates on the final artifact. Public Frame lookup and Frame.goto remain absent;
do not infer their support from Page.goto.
The rebuilt attachment collector also fails both standalone Rust attempts;
four observer-layout attempts fail in the oracle's own connection setup.
`attachment-commands.json` correlates the slow command to existing target
`08178DA3BC37B01AAEABFBE835562EEC` (`about:blank`, ownership unknown):
`Page.enable` takes 4760 ms and 3984 ms in the two standalone rounds. Subsequent
initialization commands have no observed response by the deadline. This narrows
the location of the delay but does not prove browser CPU, competing auto-attach,
or any particular cause. The target was not closed. Preserve these failures
separately from historical NODE-061 passes.

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

Owned close now additionally exposes `CdpContextCloseOutcome` to composition
callers. Actual native disposal success returns `AcknowledgedDestruction`;
an error classified as an idempotent missing Context returns `AlreadyClosed`.
The original `Service<()>` and `close_route` still return the same public unit
result. Only actual acknowledgement marks the opt-in temporary Locator-group
destruction receipts, under the route transaction before runtime cancellation.
This fixes the reproduced NODE-026 cleanup gate without treating an error string
as proof. See [the lifetime contract](pwright-locator-evaluation.md#optional-native-destruction-receipts-2026-09-07).

The experimental Node host now selects this no-options capability and its
paired owned-Context close service. Native creation/disposal and topology
publication remain in Rust. It additionally selects a separate pre-resume
session-initialization port for owned Context viewport/window, focus and media
defaults, so a raw incognito Context is not mistaken for Playwright's entire
default initialization. The original slice accepted only empty options;
NODE-062 adds host-side baseURL metadata without changing the no-options typed
backend command. Other non-empty options fail before native mutation.
Downloads, headless fonts and broader default/options
parity are explicitly unfinished in NODE-014. See
[the Node Context checkpoint](../node-ffi-compatibility-design.md#owned-context-lifecycle-and-initialization-checkpoint)
for exact native inventory, first-document and isolation evidence.

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
