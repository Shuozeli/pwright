<!-- agent-updated: 2026-10-12T01:26:00Z -->

# Playwright-compatible Python FFI design

Status: the base Chromium-CDP slice is implemented and verified. Compatibility
is limited to the supported surface below; APIs explicitly reported as
unsupported are not part of the claim.

## Locator click force forwarding (wheel-verified, browser-unverified, 2026-10-12)

Async and sync `Locator.click(force=...)` now validate an optional boolean
and pass explicit values through the native `NativeLocator.click` call into
Rust `ClickOptionsV2::with_force`. Omitting `force` preserves the older native
one- or two-argument call shape; `steps` remains independent. Selector-backed
Locators forward the same value to the existing `Page.click` path. The focused
Rust release tests pass 12/12, strict addon Clippy and formatting pass, and
facade tests pass 18/18 on both Python 3.9 and 3.12. The native handshake is
now ABI115 so an old ABI114 extension fails at import. The repaired full-feature
wheel (SHA-256
`5856bc486b4bc342ceb82a251de79779a20b4b5b2094c0bbdbe028883fda8e54`)
exports 103 selected capabilities and passes 528/528 installed-wheel units and
dependency checks in each fresh Python 3.9.6 and 3.12.14 environment. The
first ABI115 build selected only 102 because the project configuration omitted
`element-handle-click`; it was rejected and the configured list corrected. The
official handler `force_true` cases still require the separate
`Page.add_locator_handler` capability below and have no pass credit.

## Proposed Page locator-handler boundary (not implemented)

The pinned upstream async and sync `test_page_add_locator_handler.py` files
contain 13 cases each. The current candidate fails all 26 at the absent public
`Page.add_locator_handler` method, while the independent oracle passes 26/26.
This is a single substantial capability gap, not 26 passing opportunities from
a facade-only method. In particular, a one-time Python overlay check before an
action cannot observe an overlay created by `mouseover` during a Rust-owned
click or hover retry, and cannot run during Rust-owned assertion polling.

The proposed opt-in `page-locator-handlers` capability has the following
ownership boundary. Rust owns a per-Page, resource-bounded registry of immutable
locator queries, invocation limits (`times`), active-handler state, Page-close
and Frame-detach state, and the original action/expectation deadline. It
checks for matching overlays before each action or visibility-assertion attempt
and after moving the mouse but before pressing it. A bounded request/acknowledge
channel asks Python to run the user callback, without holding the Rust command
slot or Page lock needed by that callback's own Page operations. A handler is
not re-entered on the same Page while it is executing; default post-callback
waiting for the overlay to hide shares the original deadline, whereas
`no_wait_after=True` omits that wait. Python retains only the callable and
original Locator object and provides async/sync API adapters. The sync bridge
must dispatch callbacks off its own event-loop thread to avoid deadlock.

Acceptance requires bounded registration count, outstanding callbacks, query
bytes and command slots, including cancellation and Page close; `force=True`
must bypass handler invocation while preserving its other click semantics.
Tests must cover the Rust state machine and post-mousemove hit testing, Python
callback reentrancy and lifetime, async/sync facade parity, strict feature
closure, and then all 26 unmodified official browser cases. This design has
not been implemented or granted official conformance credit.

## Retained ElementHandle select-text artifact (2026-10-11)

ABI 114 adds async/sync `ElementHandle.select_text()` as an independent opt-in
Handle leaf. The Rust host evaluates only the original retained Handle,
retries visibility within one timeout budget unless `force` is set, and
selects input, textarea or ordinary element contents using the upstream
operation order. Retained scrolling and text selection share one four-action
limit, leaving four of eight Handle-call slots for other work. The stable
repaired wheel (SHA-256
`4850418950bac4dfe1fef27d34042d3b030345d2f0a32c030bcfb40f4c41b592`)
loads ABI 114 with 103 selected capabilities on fresh Python 3.9.6 and
3.12.14 installs; each passes 518/518 package unit tests and dependency
checks. The installed `locator_read` signature remains unchanged after a
no-suppression Rust Clippy fix. Strict selected release Clippy, formatting and
the shared-slot release invariant pass. None is official browser evidence.
The earlier repaired ABI114 wheel after the shared Host wait-signature change
(SHA-256 `d66b803fbd03480657ba24b60b5c6953fa45d9ffa3bb516768defea3bcd4e5a0`)
exports the same ABI114/103-capability selection and passes 518/518 installed
units plus dependency checks on each fresh Python 3.9.6 and 3.12.14 install.
An incorrectly built three-capability wheel was quarantined: Maturin CLI
`--features` replaces the configured feature selection rather than appending.

## Retained ElementHandle scroll source checkpoint (2026-10-11)

ABI 113 exposes async/sync `ElementHandle.scroll_into_view_if_needed()` through
a narrow opt-in `element-handle-scroll-into-view` addon feature. Its dependency
edges are the foundational `handles` binding and the same-named host feature;
the Pwright package closure is unchanged from `handles`. PyO3 passes the exact
retained Handle and one integer timeout to the host, without a Python DOM
observer, selector recipe or scroll implementation. The host owns repeated
stable/layout/intersection observations and the final same-node scroll commit.
With a Page owner, the facade uses the effective Page/Context timeout; without
one it uses the default 30,000 ms. Explicit `timeout=0` removes the host
deadline; positive timeouts bound the whole action, including retries and
evaluation. Detached nodes fail terminally, while hidden-by-display or
unstable nodes retry until ready or timeout. Host finite-timeout errors retain
action-progress text. This source checkpoint has focused fake-session facade
tests and a feature-closure check; no ABI113 wheel, installed-unit run, or
independent official browser evidence has been produced.

## Retained ElementHandle owner Frame source checkpoint (2026-10-11)

ABI 112 adds async/sync `ElementHandle.owner_frame()` under the existing
opt-in `element-handle-dom` feature, with no new selected feature or Cargo
dependency. PyO3 passes the original retained Handle to
`HostSession::handle_owner_frame`, which acquires the current DOM owner and
reconciles its native ID against current topology within the source
BrowserContext. The Python facade refreshes that Context's current Pages and
Frames before returning a canonical Frame object; it does not infer ownership
from the Handle's frozen source Page. The sync facade wraps the Frame through
its actual Page's canonical wrapper, including cross-Page adoption. An orphan
document remains `None`; a native Frame ID absent from the current Context
fails closed. The existing `element-handle-dom` package-closure check proves
the selected path remains within the `handles` dependency closure, and the
full wheel already selects it explicitly. Focused source tests cover these
facade identities and fail-closed paths; wheel build and pinned official
Chromium async seven/sync six owner-Frame cases remain unverified.

## Retained ElementHandle geometry source checkpoint (2026-10-11)

The opt-in `element-handle-dom` host profile now composes the package-free
`locator-bounding-box` leaf, allowing its existing
`HostSession::element_handle_bounding_box` method to measure an exact retained
Handle. PyO3 converts the Rust `Option<LocatorBoundingBox>` into the official
four-coordinate dictionary or `None`; the async and sync facades only forward
the call. They do not reselect the element, serialize a DOM node, or own a
second browser geometry implementation. A new native method advances the
source ABI from 109 to 110, so older extensions fail closed at import. This
checkpoint is source-only until a new release wheel and complete pinned
async/sync ElementHandle browser files are rerun.

## Context binding and media lifecycle checkpoint (2026-10-10)

The experimental opt-ins `context-bindings` and `context-forced-colors` reuse
the Page-binding and media Rust owners; the ABI remains 107. Python stores
only language callbacks and wraps their Context/Page/Frame source identities.
Rust owns scoped registration, browser script installation, pending call
routing, Handle transfer, media policy and future-session inheritance.

The sole Python binding queue consumer now dispatches callbacks independently
so a callback can await another exposed function. A separate 64-dispatch
admission bound remains effective after Page retirement releases Rust pending
entries. Each dispatcher owns its callback child across acknowledgement
failure and cancellation. A callback initiating Browser close is excluded
from canceling/waiting on itself; other callbacks are canceled with a bounded
five-second wait. Cancellation-resistant user code retains its dispatcher
slot and records an error rather than being reported as successfully joined.
Page/Context registration transactions continue native installation or
rollback after caller cancellation, retaining the callback if installation
commits. An independent Browser-shared bound admits at most 64 running or
queued Page/Context registration transactions before constructing their
coroutines or tasks. Caller cancellation retains the slot until native
commit/rollback finishes; separate Contexts cannot bypass this bound. A close
marker is retained only for a dispatcher owned by the Browser being closed.

The focused language regressions pass 22/22. The v4 ABI107 wheel used for
strict browser results passes 379/379 installed package units on both Python 3.9 and 3.12, including the
formerly stale native-profile assertion. A prior wheel exposed eager
`asyncio.Lock` construction on Python 3.9 after an earlier loop closed; the
Context lock now initializes on its owning loop. The sync `Browser.contexts`
property and direct prior-loop-close regressions cover this path. Strict
types, Ruff and Python 3.9 runtime type checks pass. The wheel and complete
independent browserless collections are recorded in
`python-official-conformance.md`. The previous v3 wheel failed an ordinary
official Page creation with duplicate Page-binding initialization despite
these local gates. A narrow topology replay fix in v4 passes the unchanged
official Chromium navigation case 1/1. The unchanged Context forced-colors
case then passed in both the installed v4 candidate and independent oracle,
and exactly one candidate strict-ledger row advanced from failed to passed.
An attempted 39-case Context run also exposed an init-script callback failure
and a basic exposed-function timeout before dedicated Chrome CDP became
unreachable; no partial-run rows were promoted. Dedicated Chrome later
recovered externally, and a paired seven-case clear-cookie selection passed
in candidate and oracle, as recorded in `python-official-conformance.md`.
A browserless fake-NativeSession regression with separately gated Context
`add` → Page `mul` → Context `sub` callback stages passes with the entire
25-case facade unit file, but does not reproduce the official binding timeout.
These results do not confer full-corpus conformance credit. Current-document
rollback after partial installation, raw native-Future cancellation and the
shared 32-command acknowledgement/delivery lane remain unresolved risks.
That lane also shares a 64 MiB budget: seven ordinary evaluations reserve
56 MiB before future-body charges, while the binding watcher holds at least
32 KiB, leaving less than the 8 MiB required by either result or error
delivery. The native eight-active evaluation scheduler is another shared
dependency: publication drains awaiting acknowledgements can occupy the
scheduler those acknowledgements require. These are source-level progress
risks; no browser-level reproduction is claimed here.

The official init-script callback failure has a separate concrete CDP
parameter discrepancy: upstream runs already-registered Context scripts in
the new Page's initial document using `runImmediately: true`, whereas dynamic
registration does not. Rust source now sends this flag only for new-session
installation. Exact fake-CDP regressions, standalone and combined six-case
release tests, and strict Clippy pass. The v4 wheel used in strict results was
built before this source change, so its official failure and strict ledger stand until a
new wheel is browser-tested. This does not explain the independent
exposed-function timeout; a missing-route binding event remains unconfirmed.

An isolated v5 ABI107 wheel now packages that source repair with the same
96-feature profile as v4. Its release and independent rebuild have equal
normalized payloads/native hash; Python 3.9 and 3.12 installed package
tests each pass 380/380, 81 release dependency checks and strict Clippy pass,
and the four newly type-corrected official-tool test scripts are included in
the CI Pyright path list with zero errors. This is not a browser confirmation:
the v5 wheel has no official report yet, and the v4 failure remains in the
strict ledger.

Two Rust host integration tests reproduce ordinary-command count and byte
admission exhaustion through the real `FfiBudget` implementation. They pass
as expected rejection tests; they do not establish a liveness fix or a
dedicated settlement service. CI now runs them with the Python release
boundary gates.

A future settlement composition would need independent count, byte, native
scheduler and CDP pending/wire ownership for admitted binding calls, not just
a new FFI lane or Python retry. Bounded receipt metadata cannot conceal 64
uncharged converted result trees. Conversion/preparation must retain explicit
ownership, and accepted settlement must survive foreign cancellation or be
revoked on exact Page/Context retirement. This is a proposal to validate, not
an accepted or implemented design; the current artifact has no such service.
See `python-official-conformance.md` for immutable official outcomes and final
artifact verification; this checkpoint does not extend the verified-parity
claim above.

## Page check and uncheck facade (2026-10-10)

The pinned 18-case Chromium async/sync `test_check.py` manifest has SHA-256
`7e9b34f4ed0b0c193e213375ad2fc8dad6fbd13b622cda3f77c5d2ecf51809ae`.
Before the facade change, all 18 candidate cases failed at the missing
`Page.check()`/`Page.uncheck()` methods while the independent v1.58.0 oracle
passed all 18; both baseline provenance sidecars verified. The async methods
now select a Locator with Playwright's non-strict first-match default and
delegate to its existing Rust-owned checked-state action; sync delegates
through the bounded bridge. Unsupported position and no-wait-after options
remain explicit errors rather than silent behavior.

The repaired `abi3-py39` wheel SHA-256 is
`85913b81bcc9d9c65533554b4b6f7f9ca423216ea08f43d682bc1244b2d5d46b`.
It passes 271/271 installed package units, Ruff and strict Pyright. The final
candidate v2 report passes 18/18, as does the oracle; candidate report and
verified sidecar SHA-256 values are
`2bc0177b7a6cc8f1ca7c1d4a6e20acfd728dbf819224ee88e69a7f07ad633e14`
and
`8c374ce922b748f613d82980ba3aa6be3eb68cdcde76071d19af7e36c8a44573`.
The exact 18-row merge advances the candidate 6,526-row ledger from 1,208
passed / eight skipped / 22 failed / 5,288 not run to 1,226 passed / eight
skipped / four failed / 5,288 not run, SHA-256
`82c8161800ec3acd18938d0f9d0ba80edf76ce66b0918d563a6923b93ef6ee74`.
The oracle's 18 passes follow a 25-pristine-ID rebase to the profiled inventory;
its merged 6,526-row ledger is 780 passed / seven skipped / four failed /
5,735 not run, SHA-256
`1bc6a8b0dbaa1a79471e60d4c7576e0512043c737e2f70dea80db56fe13cea47`.
This is selected-file parity, not full-corpus completion.

## BrowserContext storage-state composition (2026-10-10)

The explicit `context-storage-state` profile now supplies
`BrowserContext.storage_state(path=..., indexed_db=...)` and
`Browser.new_context(storage_state=...)` in both async and sync Python. The
snapshot returns Playwright-shaped cookies and origins, and optional `path`
writes JSON. New-Context restore accepts a mapping or JSON path before
publishing the Context. Python validates options, marshals JSON and performs
optional file I/O; Rust owns cookie, localStorage and simple-JSON IndexedDB
capture/restore, origin tracking, bounded operation admission and temporary
Pages. Those Pages are hidden from public Page/network notices and have
cancellation cleanup. The feature depends on `context-cookies` and the host's
`page-routes` leaf, while staying independently selectable; the native ABI
constant remains 104. State is limited to 8 MiB total, 128 origins, 256 KiB
of JSON per origin and 4,096 localStorage items per origin.

The unchanged official v1.58.0 nine-case async/sync Chromium selection had a
9/9 candidate red baseline from the missing API. The repaired `abi3-py39`
wheel SHA-256 is
`8f5e3234cf2d46cc9ff4c2cd9179031e59d75b53bd276c23744317d713493cb1`.
Its clean candidate v3 repeat and independent oracle v3 each pass seven and
fail the same two IndexedDB assertions, which compare upstream's hardcoded
`localhost` to the MagicDNS fixture origin. Candidate/oracle report SHA-256
values are
`a43d954ff5a14ea64aa3a86de989d350f3aefe4059b7a5ca5eef82d366d2b5ad`
and
`ed29bcc8e528ef77aa621891aa60bb346fd90b7c9d5ab214f9bb1311ae5252b6`;
their verified provenance sidecars hash to
`1565dbc19591b3b757b96b8c0c2bb947d5bab28ab0cb34786e8a094f157178e6`
and
`efd85286116c94b981f7185c87e5df36d6fe0df0f375f14582f06d92e58f18ab`.
Exact nine-key merges yield candidate 1,208 passed / eight skipped / two
failed / 5,308 not run and oracle 745 passed / seven skipped / two failed /
5,772 not run. Their 6,526-row ledger hashes are
`d1ecfa2db3e440bae951c09c55e2da862f45f964579b7e69970d59ded510f5ed`
and
`3161add7ea5243cefa071dfc2c9845ddf6e18a5dee43bc9dc7c450d7735f6a20`.

The first repaired-wheel v3 run timed out after a task Chrome tab had just
closed; its raw artifact is retained at
`.work/official-conformance/playwright-python-candidate-browsercontext-storage-state-v3.xml`,
and the cause remains unproven. A one-case control and clean nine-case rerun
of that same wheel established the paired result above. The installed wheel
passes 263 package units and 11 focused facade tests; the official
harness/provenance suite passes 75 tests.
Strict Pyright, Ruff, selected Rust release, Clippy and formatting gates pass.
Further parity for encoded IndexedDB keys/values, large-store browser-side
bounds and lifecycle stress remains open. Known-ID pending storage-state
Contexts are now hidden by exact ID; before native creation returns the ID,
the unidentified-creation guard can still broadly suppress newly observed
Contexts, leaving that concurrent visibility interval open. The full Python
and Core corpora also remain open.

## ABI 104 BrowserContext expect_page facade (2026-10-06)

NODE-410 adds no native method or capability. Async
`BrowserContext.expect_page()` delegates to the existing `page` branch of
`expect_event()`, whose bounded lifecycle waiter is already fed by Rust-owned
Context topology publication. It fails closed if the internal waiter type ever
drifts. The sync facade wraps any predicate argument into the canonical sync
Page before invoking user code and lets the existing Context event manager
wrap the resolved Page, preserving one public identity.

The first installed-wheel candidate run passed 7/9 selected sync
BrowserContext event rows and failed only two popup cases because the facade
was absent; the official Playwright 1.58.0 oracle passed 9/9. Two AAA units
cover async delegation and sync predicate/result identity. The repaired wheel
passes 252/252 installed units, Ruff and strict Pyright. Independent repaired
wheel archives hash to
`5b8914bdf0ebc2559a058dd0cabaff201c3083b9216251eadca27cad35234fac`
and `40beaa8e20e392a167abdc5e85a9ca53c602bfb52b5352e98df4f5dae7a4658b`;
their payloads match after normalizing only CycloneDX build identity/time.
ABI104/90 and native SHA-256
`6e3aef33a88b1e056931d4bbde336c57403d95cb88dc1fabaed5469d19a85d1b`
are unchanged.

Final candidate and oracle each pass the identical nine cases. Report SHA-256
values are
`cc5cbb5c3fea0801bde4e4be6eb49aa777fc3970278a90ad8f2a2e492144ef2a`
and `a0cf4a49b4c33f9cce3fbacae743b06ea9e17478fd7c935af3cbc11dd3058984`;
both provenance sidecars verify. Exactly nine rows advance the cumulative
ledger to 1,201 passed / eight skipped / 5,317 not run, SHA-256
`554cd9e63214f3b920489849cabadfe8da1559d0719705399c68f65bf3ddbaee`.

## ABI 104 sync request-timing evidence (2026-10-06)

NODE-409 changes no code, wheel, ABI or feature selection. The existing
`request-timing` capability remains a Rust-owned refinement of network events;
the sync facade reads the same immutable timing snapshot already exercised by
async. All four unchanged sync Chromium rows pass in the candidate.

The independent Playwright 1.58.0 oracle passes SSL and subresource, but Chrome
154 reports `domainLookupStart=0` for ordinary navigation and redirect. The
pinned assertion accepts only `-1` or a positive value, so those two oracle
rows remain diagnostic-only even though Pwright's existing normalization makes
the candidate green. Only the common SSL/subresource selection is promoted.
Candidate/oracle paired report SHA-256 values are
`9638463eed022460b7286e2c41bbe4ac94c0f2790de81f453332e39c5be03b24`
and `b47b3294f44af7824015e9880d4a4d5d1be65eeab9f0ae2c2f7a3f999bc7c0aa`;
both sidecars verify. Exactly two rows advance the ledger to 1,192 passed /
eight skipped / 5,326 not run, SHA-256
`79d512f3af52c78e8e8e0cf79feb41c77a1ca04cb2791538f0b453503132069e`.

## ABI 104 sync service-worker policy evidence (2026-10-06)

NODE-407 reuses the existing independently selectable
`service-worker-policy` composition and unchanged ABI104/90 wheel. The sync
facade delegates the same default-allow and explicit-block Context options,
init-script policy and Console event path already proven by async. Candidate
and Playwright 1.58.0 oracle each pass the two unchanged Chromium rows 2/2
under trusted Tailscale HTTPS. Report SHA-256 values are
`9a9e21e364401bf7ceed7a9ced73ad87c8a43618496e8ca9a63ee317c1ddac18`
and `6c9afb0ba0eae0c23a3838920129dbfbe6436e78bd3ea02be9a364cb63cd5dfe`;
both provenance sidecars verify. Exactly those rows advance the ledger to
1,190 passed / eight skipped / 5,328 not run, SHA-256
`94f83882339cd9af0b64200beea2098b8b7ba3ee551c2228b52bf19e1ae61013`.
The ordinary-HTTP diagnostic is not reused because service-worker registration
requires a secure context. No dependency or resource boundary changes.

## ABI 104 Keyboard plus evidence (2026-10-06)

NODE-406 reuses the unchanged NODE-403 wheel. The existing Rust-owned Keyboard
mapping emits the Playwright-compatible `+`/`Equal` down, press and up sequence;
the isolated candidate and Playwright 1.58.0 oracle each pass 1/1. Their report
SHA-256 values are
`0c982237fedbda5cf2383b0f29e91e9a5efe70501365c6b769aadc5ce76510fa`
and `9d696c762ba7726965289b186a4bdc24fdab15eb53bca0c4aee2c7ddd4490e07`,
and both provenance sidecars verify. Exactly that row advances the ledger to
1,188 passed / eight skipped / 5,330 not run, SHA-256
`672a6829fc84a4d55ec587ad23880de96a9cef3dd318719a00f47caae3fe7c5a`.
ABI104/90, the native artifact and all resource bounds remain unchanged.

Paired popup-dialog and fulfilled-cookie diagnostics fail identically in
candidate and oracle under Chrome 154 plus the mandatory Tailscale fixture
host. They remain unpromoted environment evidence rather than compatibility
changes. Full Python and Core JS/TS execution remains open.

## ABI 104 sync caller-stack parity (2026-10-06)

NODE-403 keeps the native ABI, 90 selected capabilities and all Rust owners
unchanged. When the background asyncio loop returns a Playwright `Error`, the
sync bridge captures the currently blocked caller thread and prepends that
Python stack to the existing native/async stack. Capture happens only on the
error path; non-Playwright exceptions are untouched, and successful calls do
not perform stack formatting. One AAA unit verifies both the caller source and
retained native suffix.

The final portable-wheel candidate and independent Playwright 1.58.0 oracle
pass the same seven isolated Chromium Page rows 7/7. Their reports hash to
`3a22d05c8bdb2ec79e19211bd6fb577574fe7893c6d16f6b3d54efa7e7a6bf35`
and `b640c1136d468c6edd847723adf8f1c58ddb8b175ced6dc61f9707e59c66190b`;
both provenance sidecars verify. Exactly those seven passes advance the ledger
to 1,187 passed / eight skipped / 5,331 not run, SHA-256
`036aed9d37c7e88dfbce0ffe4210e7a16e11b2f06b860a8200015a2e99257eda`.

The two `Page.pause` tests remain explicitly unrun because the visible,
dedicated CDP Chrome is headed while upstream's fixture declares headless.
Pwright therefore keeps rejecting the absent headed Inspector capability. Two
repaired wheels reproduce after CycloneDX normalization, and the installed
wheel passes 250/250 units plus all dependency, release, formatting, lint,
strict-type and Python 3.9 gates. Full Python and Core JS/TS execution remains
open.

## ABI 104 sync request-fulfill parity (2026-10-06)

NODE-402 verifies that the sync facade composes the same Rust-owned API request,
route and response boundaries already proven by async. Fetching the original
request through `page.request` and fulfilling a route from that response passes;
typed JSON fulfillment preserves status, headers, content type and decoded body.
No sync-side HTTP client, route state or body serializer is added.

The installed NODE-401 wheel candidate and Playwright 1.58.0 oracle each pass
the two remaining sync Chromium rows 2/2. Candidate/oracle report SHA-256 values
are `b95188b98d62986ba046d643cf76d4429566b2627aac68b280c596dd89d3aa78`
and `637626c1e57deda856ccab3b3c000139934fba966c38c228e43f4e46949342dd`.
Exactly those two passes advance the ledger to 1,180 passed / eight skipped /
5,338 not run, SHA-256
`1846062292632f04561166ad19d4e8265a624f3eb837aa969ec14ebf6db55d78`.
ABI104/90 and every package/resource boundary remain unchanged.

## ABI 104 complete TodoMVC profile (2026-10-06)

NODE-401 keeps ABI 104 and the 90-feature profile. Rust's evaluation-source
classifier now finds only top-level arrows with balanced delimiter and quoted
string tracking, and treats `function` / `async function` as bounded tokens.
An expression such as `JSON.parse(...).map(i => i.title)` is therefore sent as
an expression instead of being called as though the entire source were a
function. Three AAA Rust tests cover nested arrows, top-level destructured
async arrows and false `functionality` prefixes.

The async and sync Python Context facades now expose one stable `tracing`
object. `stop()` without a path is the Playwright-compatible inactive no-op
required by the examples plugin teardown. `stop(path=...)` remains an explicit
`UnsupportedError`; no trace file or recording capability is fabricated. This
is a language compatibility shape over the existing Context lifetime, not a
new native method, owner, queue or selected feature.

The installed-wheel candidate and independent Playwright 1.58.0 oracle pass
the exact same isolated TodoMVC selection 25/25. Only those paired passes enter
the ledger, which reaches 1,178 passed / eight skipped / 5,340 not run,
SHA-256
`bfb8e95031d10240bc110b16ba18a800711f0e5e4cc513cef4e8cfd0f14198bd`.
The final candidate and oracle reports hash to
`19e27f3c63ecf13804581d9c079afc5393ca005560d7770d34db07be82b82967`
and `a8792fe4c0a1a19db6dcf3aaaf73e7cedb96c78edd70d14b9b0c2a4f59db8fa6`.
Two portable wheels reproduce after CycloneDX normalization; the fresh install
passes 249/249 units and all strict Rust/Python dependency, release, format,
lint, type and Python 3.9 gates. Full Python and Core JS/TS execution remains
open.

## ABI 104 composable typed string matchers (2026-10-06)

NODE-400 retains the 90-feature profile and moves the native contract to ABI
104. One Rust-owned typed matcher now backs exact and regular-expression value
and class expectations. Ordered class arrays may mix exact and regex entries.
The Rust host validates JavaScript regex flags before browser access, follows a
label's associated form control for value semantics and reads ordered classes
through `classList.toString()` so SVG and HTML share the same path. Python and
Node only translate public matcher shapes and assertion term names.

The four unchanged upstream async/sync class/value Chromium invocations pass
4/4 in both isolated candidate and official Playwright 1.58.0 oracle lanes.
Only those verified candidate passes advance the ledger to 1,153 passed /
eight skipped / 5,365 not run, SHA-256
`ab288ffacb37a4632adcf1be1d70b6beb4a7dee82f26f8525a43a0f3d922e020`.
The official candidate itself loads the portable wheel from a fresh
`site-packages` install; it reports ABI 104 and 90 features and also passes
246/246 units. Node passes 309/309, all dependency
and capability-boundary proofs pass, and independent wheel builds have equal
payloads after normalizing only CycloneDX build-identity fields. Full-corpus
Python and Core JS/TS conformance remains open.

## ABI 103 composable TodoMVC prerequisites (2026-10-06)

NODE-399 keeps ABI 103 and adds a ninetieth selected wheel feature,
`history-navigation`. `Page.go_back()` forwards only the Page identity,
lifecycle mode and effective navigation timeout. Rust retrieves bounded native
history, selects the previous entry and waits through either same-document or
replacement-document lifecycle. No Python history cache or retry loop exists.

Locator assertions now cover substring and regex text, checked, empty, exact
value and ordered exact class-list states. Static browser programs read the DOM;
Rust owns retry/deadline semantics, normalization flags, collection bounds and
result conversion. Regex value and regex class-list expectations remain
explicitly unsupported. The installed 90-feature wheel passes 246 units,
strict types/lint and the Python 3.9 runtime-cast audit. The final candidate
artifact is repaired with bundled OpenSSL dylibs, has SHA-256
`5dd5bb093a84e6a71f9c6d90a5610a41f10abecf79af503a928bb124521d8979`,
and reproduces against a second repaired build after normalizing only the two
CycloneDX identity fields. Candidate and oracle collect-only each report all
25 TodoMVC Chromium cases and pass strict provenance verification, without
executing a fixture or test body. Live upstream TodoMVC
execution remains pending and is not part of this checkpoint's compatibility
claim.

## ABI 103 composable ElementHandle keyboard checkpoint (2026-10-06)

NODE-387 composes `element-handle-dom` with `keyboard` instead of adding a
second input path. One PyO3 call retains the exact native ElementHandle,
admits one input start, focuses that exact element in Rust and then runs the
existing Page-owned `press` or `type` operation. Cancellation, Page identity,
key mapping, delay validation and input serialization stay in the Rust host.
The async/sync ElementHandle facades only forward public arguments and rewrite
the public API-name prefix.

Page and Frame `type()` reuse Locator sequential input; Frame `query_selector()`
and selector evaluation carry the exact Frame ID into the existing native
locator/evaluation owners. `eval_on_selector_all()` keeps collection creation
and release behind the same native boundary. These language facades add no
protocol client, native registry, worker or capability feature.

The fixed 29-row candidate moves from 16 passed / 12 failed / one skipped to
26 passed / two failed / one skipped; the independent oracle is 27 passed /
one failed / one skipped. Cross-origin input fails in both lanes, and a paired
30-process diagnostic reproduces the other missing-initial-keydown event 2/30
in each lane. Only 26 passes and the upstream skip enter the cumulative ledger,
which reaches 1,147 passed / eight skipped / 5,371 not run. ABI 103 retains 89
selected features, passes 239/239 installed units, the 108-package proof,
strict Rust/Python gates, normalized wheel reproducibility and a verified
6,526/6,526 collection. Full-corpus completion remains open.

## ABI 102 composable Page Keyboard checkpoint (2026-10-06)

NODE-386 exposes the existing Rust Page Keyboard installation through one
optional native method. The operation tag selects `down`, `up`, `press`,
`insert_text` or `type`; the PyO3 boundary validates and owns the bounded key
or text plus delay before dispatch. The Rust host retains Page binding,
platform key mapping, admission, cancellation and CDP input serialization.
The async and sync Python objects only delegate and preserve Playwright's
public method shapes.

The fixed focus candidate moves from 3/5 to 5/5, matching the independent
5/5 oracle. ABI 102 exposes 89 selected features, passes 232/232 installed
units and advances the candidate ledger to 1,121 passed / seven skipped /
5,398 not run with no failure. Strict Rust/Python gates, the 108-package proof
and normalized wheel reproducibility pass. Full-corpus completion remains
open.

## ABI 101 composable resource-timing checkpoint (2026-10-06)

NODE-385 adds `request-timing` as an explicit refinement of `network-events`.
The backend-neutral model stores one optional `Arc<RequestTiming>` in the
bounded Request and Response records. The codec includes the nine signed
microsecond fields only when the feature is selected. Chromium supplies the
wall/monotonic origin and response phases; Rust handles cached fallback,
exact-zero unavailable connection phases, positive submicrosecond values,
redirect terminal time and terminal snapshot sharing. No timing computation
or browser identity moves into Python.

The PyO3 notice tuples carry checked monotonic sequence numbers. The host
reports a snapshot fence for each selected request/response stream, and the
single Python Browser watcher acknowledges a sequence only after dispatch.
`Page.set_content()`, `goto()` and `reload()` wait until both streams reach the
captured fence. This is a bounded publication barrier, not `sleep(0)`, a poll
loop or a wait for traffic created after the operation.

The fixed candidate baseline passed 2/4; the final installed candidate and
official oracle pass 4/4. Candidate advances to 1,119 passed / two failed /
seven skipped / 5,398 not run, while oracle reaches 738 / seven / 5,781. ABI
101 exposes 88 selected features and passes 226/226 installed units, the
108-package proof, strict Rust/Python gates and normalized wheel
reproducibility. The two focus failures remain explicit and full-corpus
completion remains open.

## ABI 100 request-fulfill evidence checkpoint (2026-10-06)

NODE-384 changes no signature, feature or owner. The two remaining Chromium
rows in upstream `test_request_fulfill.py` exercise the already composed
BrowserContext API-request owner, retained APIResponse fulfillment path and
bounded JSON fulfill path. Candidate/oracle pass 2/2, so the file is 3/3 for
Chromium. Candidate advances to 1,110 / seven / 5,409 and oracle to 727 /
seven / 5,792. This is new differential evidence for the existing ABI 100
artifact, not a broader compatibility claim.

## ABI 100 composite JSHandle checkpoint (2026-10-06)

NODE-383 keeps Handle-bearing evaluation as an opt-in branch of the existing
`handles` composition. The async and sync facades perform one bounded,
identity-preserving container walk only to replace exact public JSHandle
wrappers with native wrappers. PyO3 then records ordered preorder slots in the
bounded evaluation tree; the Rust host validates the complete value graph,
ordered unique placeholders, exact live owners and native argument resolution
before browser dispatch. When there is no Handle, Page evaluation still uses
the original non-Handle route.

`JSHandle.get_properties()` forwards to the existing Rust batch-publication
owner, so all returned properties reserve capacity atomically and each result
has an independent disposal owner. Python creates only public wrappers. Date
values use Python 3.9-compatible, timezone-aware ISO decoding. Public string
previews preserve native Window, primitive and Promise descriptions without
changing element-markup normalization or native object identity.

The fixed official tranche moves from candidate 6/15 versus oracle 15/15 to
installed candidate 15/15. Candidate advances to 1,108 / seven / 5,411 and
oracle to 725 / seven / 5,794. ABI 100 retains 87 selected features, passes
221/221 installed units and the 108-package dependency proof; two release
wheels have reproducible normalized payloads.

## ABI 99 native directory input checkpoint (2026-10-06)

NODE-382 adds `file-input-native-paths` as a strict opt-in refinement of
`file-input`. The Rust file-system owner classifies directory sources and
retains their prepared paths; the preflight owner validates the exact input's
`multiple` and `webkitdirectory` flags before any mutation; the native leaf
uses the retained element identity for `DOM.setFileInputFiles`. Call/source
leases and cleanup reservations keep already-started work bounded through
cancellation. The Python facade only normalizes the public argument and waits
for the native operation.

The route is deliberately narrow: only native directory plans use it.
Ordinary file paths remain payload inputs and retain the official remote-CDP
50 MiB guard. The feature has the same 108-package closure as `file-input`,
and host/addon no-default profiles pass strict Clippy.

The ABI 99 wheel passes 17/18 fixed async input cases; the oracle passes
16/18. Both reject the 200 MiB single file with the same official non-colocated
error, while only the oracle rejects the aggregate ten-by-49-MiB transfer.
Every oracle-executable row agrees and every directory behavior passes.
Candidate advances to 1,093 / seven / 5,426 and oracle to 710 / seven / 5,809.
The installed 87-feature artifact passes 217/217 units; two release wheels
reproduce after CycloneDX normalization.

## ABI 97 native file-chooser path checkpoint (2026-10-06)

NODE-378/379 adds `file-chooser-native-paths` as a strict opt-in refinement of
`file-chooser-events`; the dependency proof shows an identical 108-package
Pwright closure. Rust owns native path normalization, event backend-node
identity, utility-world resolution, `RemoteObject` validation,
`DOM.setFileInputFiles` dispatch and object-group cleanup. The direct-upload
future transfers already-started work to the bounded cleanup executor when a
Python waiter is cancelled, so neither the object group nor source slot is
orphaned.

The Playwright-shaped facade keeps no DOM/CDP client. It only retains the
interception lease while awaiting the Rust mutation and serializes stale
enable/disable updates. The collection-evaluation production profile enables
a user gesture because programmatic `input.click()` otherwise stops producing
a second chooser event in current Chrome. Successful ElementHandle upload
flushes already-published Page-binding callbacks before returning, matching
the official input/change ordering without making JavaScript own event
delivery.

The fixed four-row selection passes source candidate 4/4, official oracle 4/4
and fresh installed-wheel candidate 4/4. Canonical candidate is 1,076 passed /
seven skipped / 5,443 not run; oracle is 694 / seven / 5,825. Applicable
public behavior is 1,073 / seven / 5,434. The final candidate/oracle ledger
SHA-256 values are
`0284d9c1750530e607c3b07daff759278ffae92964751c644f7244d801a80cbf`
and `d5fc04bc059fdab72dd20e88ea8157655aa1bf6e62ab4191c7152ec3fe635d69`.

The installed artifact reports ABI 97 and 85 features, passes 213/213 units
and reproduces the full 6,526-ID collection. Two normalized-payload-
reproducible wheel archives hash to
`4cb27befae61e6c3f99343d38e0f68519b361a2d70bd2e99de2fb7bbae10266a`
and `81b3fdff4a27c9ab3e7e8afb9474c53be0c6145ce8b7132164b2a6d44161c1c8`.
Strict Clippy for the exact host/addon leaf, Rust formatting, Ruff, Pyright,
Python 3.9 runtime-cast audit, 30 harness tests and the full dependency proof
all pass.

## ABI 96 request-fallback identity checkpoint (2026-10-06)

NODE-356 changes no native signature, Cargo feature or package edge. The Rust
route owner still performs LIFO matching, effective URL rematching, field-wise
fallback composition and the final CDP settlement. Python only adapts public
object semantics: Request header keys are lowercase, `all_headers()` returns a
copy of the Rust snapshot and `header_value()` performs a case-insensitive
lookup. The effective URL, method, headers and post data already emitted by
Rust are retained across later native Request publication in a 64-entry cache
that matches the Rust live-call bound and is retired on terminal request
events. Response URL delegates to its exact linked Request identity.

Sync Page and BrowserContext route handlers execute on a worker rather than the
bridge loop. Nested sync calls such as `page.wait_for_timeout()` therefore
submit work to the still-running bounded loop; route commands still settle on
that same bridge before the callback completes. Four AAA regressions cover
async/sync header helpers, weak Request identity reconstruction and nested sync
callback calls. Complete units pass 207/207.

The source-pinned 55-case manifest hashes to
`126f1bf54d8189c344eedd70a4e5d271d2bed14d7a831c8bb0a3dc88c61a5b00`.
Source candidate and independent Playwright 1.58.0 oracle both pass all 55
Chromium rows: 26 async and 29 sync. The provisional cumulative ledger reaches
1,070 passed, zero failed, seven skipped and 5,449 not run, SHA-256
`14f2e04ad35fbfaff95109c326ff6064d955d737f192bee21eea60c062623256`.
Two release wheels reproduce after normalizing CycloneDX identity and retain
the exact ABI 96 / 83-feature profile. Final installed-wheel browser promotion
is pending because another task left the shared dedicated Chrome CDP endpoint
unresponsive; preserving unknown tabs forbids restarting it without explicit
authorization.

## ABI 96 BrowserContext request and Route.fetch checkpoint (2026-10-06)

NODE-355 composes two independently selectable capabilities without moving an
HTTP implementation into Python. `context-api-request` joins the existing
Rust API-request owner to the owning BrowserContext's live cookie port and
bounded snapshot of base URL, browser user agent, TLS-error policy and HTTP
credentials. `page.request` aliases that exact owner. Context close disposes
the request owner, cancels later work and carries the public close reason.
Typed params, bytes/JSON data, form and multipart values cross the ABI as
bounded values; Rust performs URL and HTTP wire serialization.

`route-fetch` composes that owner with `page-routes` and
`route-fulfill-api-response`. Rust replays the paused request with bounded
method, URL, header and body overrides, redirect/timeout policy and exact
settlement. `Route.fulfill(json=...)` JSON-encodes through the typed native
body path. Async unhandled handlers return without an indefinite route-action
wait, and sync callback code reads its retained `APIResponse` through a helper
worker rather than blocking the bridge event loop.

The two manifests hash to
`94067ae8c5817c7c21e7e18186065cb34501a56938fb302eee6b721983958f86`
and `5c2a3ca541d9eb221a8440be377d06d6c1f35e457730999e0d4103e82722fbfd`.
Final installed candidate and independent official Playwright 1.58.0 oracle
both pass every selected expanded Chromium row: 51 async Context, 47 sync
Context, 6 async route and 4 sync route, 108/108 total. Exactly those 108 rows
advance the cumulative ledger to 1,015 passed, zero failed, seven skipped and
5,504 not run, SHA-256
`a0c33f08557de51ee7665720a393469fe3538c9c8f7aa421cc3f5b684e57e178`.
The installed wheel reports ABI 96 and 83 selected features; its units pass
203/203. All-feature host tests pass 1,038 with 26 intentional ignores and
addon tests pass 2/2. The two final wheel payloads reproduce after normalizing
only CycloneDX identity; full Python and Core JS/TS execution remain open.

## ABI 95 service-worker policy checkpoint (2026-10-06)

NODE-354 adds one independently selectable `service-worker-policy` feature
whose only feature dependency is `init-scripts`. `ServiceWorkerPolicy` is a
Rust enum. For `Block`, Rust installs Playwright's exact guarded init script
before returning the Context; failure closes that exact Context and preserves
both the installation and cleanup diagnostics. Successful Context close
removes only its own init-script registrations. `Allow` installs nothing.
Async and sync Python accept a `Literal["allow", "block"]`, reject any other
value before native allocation and forward only the resulting boolean.

The four-row manifest hashes to
`ddaf461669b0da9cbf71239b4ebeacc5b7770128eabd83eb1ff2d604ccbe03f6`.
With the required secure fixture, the final installed candidate passes 4/4,
SHA-256
`48a4f35bd1a6be2620c2024a0d0e32755436910a67e4945ffb0222c56b9643a3`;
isolated async and sync stress runs each pass 20/20. A plain-HTTP diagnostic is
excluded because `navigator.serviceWorker` is unavailable there, so it cannot
exercise the upstream registration contract. The independent oracle also
cannot be promoted: Playwright 1.58.0 fails in fixture setup when Chrome 154
rejects its `Browser.setDownloadBehavior` request, before any assertion runs.
Thus all four rows remain `not-run` and the ledger remains 907 passed, zero
failed, seven skipped and 5,612 not run.

The installed wheel reports ABI 95 and the exact 81-feature profile. Units
pass 201/201; full source validation and collection remain 6,526/6,526. Rust
host tests pass 1,037 with 26 intentional ignores, Python-addon tests pass
2/2, and formatting, Clippy, Ruff, strict Pyright and the 108-package
dependency proof pass. The two final wheels hash to
`585b730bc06939ad33467f9062b897ad4f749592858b38d1e62d27d5b715fe0d`
and `58c3504a3495bc82a937f9a031906535af66b3a94f4d3188def7ff0371fb3203`;
their payloads match after normalizing only CycloneDX serial/timestamp fields.

## ABI 94 popup-event lifecycle checkpoint (2026-10-06)

NODE-353 changes internal Rust lifecycle composition without changing an FFI
signature or the ABI. During recursive discovered-Page initialization, one
temporary exact-session cursor races the initializer against
`Page.javascriptDialogOpening`. A dialog-first result publishes the bounded
provisional Page, popup link and Dialog identities needed by the public waiter,
then the same initializer resumes after `Page.javascriptDialogClosed`; it does
not create a second topology owner or detach a valid popup session. Immediate-
close console publication now settles its event Handle batch before detach
compensation, preserving the observer boundary while native cleanup continues.

Every published Dialog action has one mutex-protected state: ready with its
`CdpSession`, handled or closed. The installation retains only weak action
references, revokes the exact action on `Page.javascriptDialogClosed`, and
revokes every remaining action during host teardown before CDP transport
shutdown. A closed action releases its client and returns the existing target-
closed error without issuing a protocol command. This removes the shutdown
cycle caused by an unhandled popup prompt while keeping accept/dismiss single-
settlement semantics.

The 12-row manifest pins source SHA-256
`3fc5a5edcfcd82548a376039812a40da6c8b1b4c94fced6f806d36457fbec927`
and hashes to
`a0715af7ce9c9558df72b0823dabcb8e4a4f8161638ae19eb32b7b001f8fcc61`.
Final candidate and oracle full reports each contain 11 passes and the same
one failure, with SHA-256 values
`bfc2917d26a6b942bed4777e0483c695ead5943b0bee37878419ba43515961b1`
and
`91735e70b44f159ff211e00fac93d658d779c35f7ed18d651fdf7784eb58ee6f`.
The shared JavaScript-URL popup row expects `dialog.page is None`; official
Playwright 1.58.0 and Pwright both expose the popup Page on Chrome 154. It is
therefore preserved as source/oracle drift and remains `not-run`, not a
candidate pass, failure or skip. The explicit 11-row candidate and oracle
selections pass 11/11, with hashes
`60bf343053df493e65838a13fb3296f032b34a4388652d0521ef0f6b155edc28`
and
`a29c18999fb45ef47c692877713f21694ad4e093f6c1716eb4b50c62a2059d5c`.

The immediate-popup stress run passes 20/20, SHA-256
`64521851034441673fcc487094ef25396a4f6febd8567eab1d5ef600892e50c9`.
Installed-wheel units pass 200/200, SHA-256
`62d48e6592c13be8560e2747c5224312b7135328109e311aa8f8ac63053bf058`.
Fresh full collection remains 6,526/6,526, SHA-256
`276178d4db5f9ae0249eaa944cd4a15a1a616cccee76cb10b95a7ffc6439cef6`.
The cumulative ledger advances to 907 passed, zero failed, seven skipped and
5,612 not run, SHA-256
`4a6c94e0a40e533243c0c19419685fdc30cb5f4fdbc9cde593917695879fc962`;
installed-wheel and system-Python generation reproduce byte-for-byte.

Final wheel SHA-256 values are
`70d5533547b51ce3218638d98a1987a3837965b2c69d81462737cf4dc237c30f`
and `c71ea48d5ee5ed1a8157296a158ff0e58a3195803f615eafd755b0436b263cfd`;
their payloads reproduce after normalizing only CycloneDX serial/timestamp
identity. The installed wheel reports ABI 94, version 0.1.0 and the exact
80-feature profile. It loads only system libraries plus wheel-vendored OpenSSL,
never a Homebrew path. Source pinning, formatting, strict Rust/Python gates and
the 108-package dependency proof pass. The remaining Python rows and complete
Core JS/TS execution stay open.

## ABI 94 cookie-read conformance checkpoint (2026-10-05)

NODE-352 changes no FFI or runtime code. Final installed candidate and the
official oracle produce the same 5 passed / 3 failed result over all eight
async Chromium BrowserContext cookie-read rows. All three shared failures are
literal `localhost` domain assertions under the required MagicDNS fixture.
The clean five-row selection passes 5/5 in both lanes and verifies pristine
storage, `HttpOnly`, strict/lax `SameSite` and multi-URL filtering. The ledger
advances to 896 passed, zero failed, seven skipped and 5,623 not run,
SHA-256
`330786f82e6d9682e744904c6a3a985b1a25f3f9711db1dd851636fb139dc3b4`.

## ABI 94 clear-cookie conformance checkpoint (2026-10-05)

NODE-351 changes no FFI or runtime code. The current installed wheel and the
official oracle both pass all seven async Chromium BrowserContext clear-cookie
rows. The selection exercises whole-context clearing, Context isolation and
exact/regular-expression name, domain and path filters. A source-pinned fixed
manifest makes the evidence reproducible, and the cumulative ledger advances
to 891 passed, zero failed, seven skipped and 5,628 not run, SHA-256
`fd8aa762af6f316feb60d2f9a777ba5432c02f4256b01baf0f5085dff7b962c1`.

## ABI 94 Playwright error inheritance checkpoint (2026-10-05)

NODE-350 changes only the Python exception hierarchy at the native boundary.
Rust remains the source of `PWRIGHT_INVALID_ARGUMENT` classification and the
bounded diagnostic. The exported `InvalidArgumentError` now derives from the
public Playwright-shaped `Error`, so official callers catch protocol/domain
validation through the same base class as upstream. Python-side argument and
signature preflight still raises ordinary `TypeError`/`ValueError`; no error
classification is recovered by parsing strings.

The source-pinned 19-row BrowserContext add-cookie file improves from ABI 93
candidate 15 passed / 4 failed to final installed ABI 94 candidate 17 passed /
2 failed, exactly matching the independent oracle. The remaining two failures
are shared MagicDNS applicability cases, not product discrepancies. Clean
candidate and oracle selections pass 17/17. The ledger advances to 884 passed,
zero failed, seven skipped and 5,635 not run, SHA-256
`193d0cfe5e6674effc3bd876ec3fc8299b7e0256986554a3785f66f9f4c0850d`.
Installed-wheel units pass 196/196 and full collection remains 6,526/6,526.
Two repaired wheel payloads reproduce after normalizing only CycloneDX
identity. Strict Rust/Python gates, source pinning and the 108-package
dependency proof pass.

## ABI 93 Rust-owned route redirect checkpoint (2026-10-05)

NODE-346 adds the separately selectable `route-redirects` feature as exactly
the dependency union of `page-routes` and `request-history`. On
`Fetch.requestPaused` with `redirectedRequestId`, Rust advances the existing
redirect-chain state and continues the hop without selecting a handler,
publishing another Python callback or consuming a finite-use budget. Explicit
`Route.continue_()` headers are retained in a fixed 64-chain Rust owner,
reapplied to every hop after removing `Cookie`, and retired on terminal
network events, target detach and Page reconciliation. Capacity exhaustion is
an explicit error rather than unbounded growth.

The existing bounded request-history owner exposes an exact native-ID lookup;
the route callback waits for that owner and receives the same public Request
identity used by Page request history. Python adds only facade relationships:
it links `redirected_from` and `redirected_to` in native insertion order,
selects the final request for the navigation Response, and preserves the
complete Fetch header snapshot. Async and sync Route adapters share the same
Context wrapper, so redirect identity is not duplicated at the language
boundary.

The fixed upstream manifest hashes to
`0d745e50edccc9db198df552fd0eae83947c53e2a4a6706326ae7d873f6eef14`.
The exact ABI 92 red lane fails 2/2, SHA-256
`5395181269a7dace0d878dfd9f91b7ff88fb5183175b08f212b5ecf2f707edd8`.
Final installed candidate and independent oracle pass 2/2, with report
SHA-256 values
`dcd31ae1b4590970cb7c46d5aed5df923902ecbefe6983e1dd460a759e9d2339`
and
`a96f71c0e5b63e71e37c02df982895951ba63711f68c47f2bf910a04a08153cf`.
The complete async Chromium Page-route file is 43 passed / 1 failed, SHA-256
`3f3ef3d3f0f8b3823451e4ce8f76129e9a43d324ff482a8fa2ab3f813822cac9`;
the sole remaining row is service-worker interception.

Installed-wheel units pass 195/195, SHA-256
`47e070ff47cbdbce1b40c79de85677a4818e89cb6c5ac027003c9b863f1ab500`.
The all-feature host library passes 1,030 tests with 26 declared ignores and
the addon passes 2/2 with its explicit project Python linker. Focused redirect,
capacity and exact-native-request tests, minimal release checking, strict
host/addon Clippy, formatting, Ruff, Pyright, source validation and the exact
dependency-union proof pass. Final repaired ABI3 wheel SHA-256 values are
`95512f3ea04a9c8147fe6441acae1b95705d00b99597cf77186a4c62bb2bd4bc`
and
`9833b1fa82f1659adb555f627fce2a8f302627006a51526670c93b728da8fcf2`;
their payloads reproduce after normalizing only CycloneDX identity.

Fresh final-wheel collection is 6,526/6,526, SHA-256
`d8744abdf5035700aef6b32d821c0fe51fbc168c52c64c208dc2c032cb3f0961`.
Both selected Chromium rows move from `not-run` to `passed`. The cumulative
ledger is 853 passed, zero failed, seven skipped and 5,666 not run, SHA-256
`f4dca295f673c6b680020e02c4f99d8a7a8cd2dd261696dcdf7638a32ce9a362`;
its summary hashes to
`d18de51495c3cd3c109afddedd418f78e3c65782587ad1a0f3c8f314915bc0fc`.
Service-worker interception, the remaining Python corpus and the complete Core
JS/TS corpus stay open.

## ABI 92 Rust-owned APIResponse route fulfillment checkpoint (2026-10-05)

NODE-345 composes the existing bounded Page-route owner with the independent
Rust API-request client. The new `route-fulfill-api-response` Cargo feature is
separately selectable and has exactly the dependency union of `page-routes`
plus `api-request`. `NativeApiResponse::route_fulfill` inherits the retained
response's status and headers, applies an explicit content-type override, and
constructs the existing `RouteFulfill` action. `ApiResponse::body_bounded`
checks the stored body length against the existing 1 MiB route limit before
cloning, returns a capacity error without consuming or mutating the response,
and therefore keeps an over-limit response reusable.

The async binding accepts only its public `APIResponse` wrapper and passes the
native response object directly when the caller does not supply a replacement
body or path. Explicit body/path calls keep their existing normalization
paths; sync mode unwraps the same async response identity. No response bytes,
route stack or response registry is added to Python. Page `Response.json()`
uses the existing Rust-owned `body()`/`text()` boundary and performs only the
language-native JSON conversion after the bounded body crosses the requested
public read API.

The fixed upstream manifest hashes to
`1a1ec11a564aee020317fdcb62e220fbe5b6f950e1687622e2f428b8d737f42c`.
The exact ABI 91 candidate red lane fails 1/1, SHA-256
`6e224bf6ebca1f40440f79062103e8c32dc13c357078570e5e1506889219f300`.
Final installed candidate and independent oracle pass 1/1, with report
SHA-256 values
`1f03ff84db73ce99fe0b3c60122d087b1682a7431121318889771336f7354a2a`
and
`ee4517f65b628199cc353ae00aa31837280ab452d201e2121cd44d69aa47eed8`.
The complete async Chromium Page-route file is 41 passed / 3 failed, SHA-256
`8362386e58f0fcf6641ef5fdb80410aa840f516a7335be84da3e973dd7f8c76e`.

Installed-wheel units pass 194/194, SHA-256
`e2bcb1f6dfa243b1ca3a561f13a6240b9ff20f6a8b9325cea0366bc02cef3bad`.
The all-feature host library passes 1,026 tests with 26 declared ignores and
the addon passes 2/2. The entire API-request crate passes 22/22; its focused
Arrange-Act-Assert bounded-body test proves that an oversized body is rejected
before cloning and that the response remains readable. Minimal-feature and
all-feature strict Clippy, formatting, Ruff, Pyright, source validation and
the exact dependency-union proof pass. Final repaired ABI3 wheel SHA-256
values are
`20cddd19a6d96ab92b9c93d4d24b27c473cffb67682ecbef6f98e3a6b343c041`
and
`a171ac67d754fe24767c4a9d65dba550fb42bb62d687db885b1cd4c48edf5b3c`;
their payloads reproduce after normalizing only CycloneDX identity.

Fresh final-wheel collection is 6,526/6,526, SHA-256
`1c0a18e6a8f361627a2fba0bd1ebff8e9c64b6480289865feceb5717ec01f26d`.
The selected Chromium row moves from `not-run` to `passed`. The cumulative
ledger is 851 passed, zero failed, seven skipped and 5,668 not run, SHA-256
`09089b9c798ccf3a4e3f93645839257783b16a39c03b8c737ade9f34b0d40d48`;
its summary hashes to
`f04485e49607318120ef4753ecb61f902187eb8ddc7ac571ddffb8a96fdc399a`.
The three remaining Page-route discrepancies are service-worker interception,
navigation redirect interception and subresource redirect chains. They and
the broader Python and Core JS/TS corpora stay open.

## ABI 91 native CORS preflight checkpoint (2026-10-05)

NODE-344 aligns Chromium routing with Playwright's hidden preflight behavior
without widening the binding. Before selecting or consuming a user route, the
Rust route owner reads the bounded Fetch method and headers. An `OPTIONS`
request is treated as a browser-generated CORS preflight only when it carries
`Access-Control-Request-Method` and an existing Page or Context registration
matches its URL. Rust then sends one direct `Fetch.fulfillRequest` with status
204, an empty body, the request origin (or `*`), requested method, credentials
and optional requested headers. It neither enqueues a public `RouteCall` nor
decrements `times`. An ordinary `OPTIONS` request stays visible to the user's
handler, and unmatched preflights are not claimed by this installation.

Python APIs and ABI remain unchanged at 91. Three deterministic
Arrange-Act-Assert Rust tests cover exact header echoing, ordinary `OPTIONS`
classification and finite route-budget preservation. The fixed upstream
manifest hashes to
`8c17ca7e9acffb0fdad0f3523a4f83eadf2f3649371a3c42d889f75e921c01d2`.
The exact ABI 91 red lane fails 2/2, SHA-256
`87541e590afec054f4d0d6fbc49ddd80104de19c6d55e119f1f8726ca3081694`.
Final installed candidate and independent oracle pass 2/2, with report
SHA-256 values
`61ae975f214727ccd92c77ba760d8e2b6b9936d1a249c32091c07fcb99d8ee4e`
and
`f596e5b7e0c4b44df14a7f2c15e86396e81a0c3e87661a6d2a31cdbc03dd2991`.
The complete async Chromium Page-route file is 40 passed / 4 failed, SHA-256
`91158df512ed9ba3b86c511b037758ffb6e82c193380604849622cd11ac8da5d`.

Installed-wheel units pass 190/190, SHA-256
`3c65b99599a6969e81bf1e769f911c8681c55dcd1bed7f2a0c8d5e028dfd5590`.
The all-feature host library passes 1,026 tests with 26 declared ignores and
the addon passes 2/2 with CPython 3.12 pinned for linking. Focused preflight
tests, strict all-feature Clippy, formatting, Ruff, Pyright, source validation
and the unchanged 108-package dependency proof pass. Final repaired ABI3 wheel
SHA-256 values are
`60468918bafa82f30f78a95268939ff118de0c5286d8bdf8ffc6e4c2ac0a16b5`
and
`fda32335a6717e0ae4a67ad15ba91e6ddd72b3938b3edaffb977c88af60f2bbb`;
their payloads reproduce after normalizing only CycloneDX identity.

Fresh final-wheel collection is 6,526/6,526, SHA-256
`c9150dc8b476679a5b659cceafd84cdf6b7093f34df891bed514f6d5ff2bf132`.
Both selected Chromium rows move from `not-run` to `passed`. The cumulative
ledger is 850 passed, zero failed, seven skipped and 5,669 not run, SHA-256
`5a0465a5fdeef895cae08b30b2babca1b794b28474a5cf32118962e1556ab0d4`.
The `Route.fulfill(response=...)` discrepancy is handed to NODE-345. The other
three Page-route discrepancies and the broader Python and Core JS/TS corpora
stay open.

## ABI 91 exact routed Request.response checkpoint (2026-10-05)

NODE-343 correlates a routed public `Request` with the network lifecycle owner
without building a second registry. Chromium supplies `Fetch.networkId` on
`Fetch.requestPaused`; the bounded Rust route call preserves that exact value
through fallback and exposes it only as an internal ABI key. The existing
request-history owner searches its at-most-100 native records for that ID,
waits on a `Notify` when the corresponding `Network.requestWillBeSent` record
has not arrived yet, and then awaits the existing settled Response owner.
Record, reconciliation and close transitions wake waiters. Session stop and
Page close terminate the operation; URL, method and timing are never used as
identity substitutes.

The Python wrapper stores only that optional bounded key on a routed Request.
`Request.response()` delegates it to the ABI 91 watch-lane operation and wraps
the returned descriptor with the same public Request object, preserving
`response.request is request`. Ordinary Request history continues through the
existing portable Rust owner. Wheels without the request-history feature keep
the operation explicitly unsupported.

The fixed upstream manifest hashes to
`bb1d45e9ed7cce9eb55e00f1b90d07c96cb6f8d6fcf3b772f6844a2229afe83f`.
The exact ABI 90 candidate red lane fails 1/1, SHA-256
`e49f5cc30cbdcbe4a5f17c52b354d5719eaf859917602d160790887f1e752492`.
Final installed candidate and independent oracle pass 1/1, with report
SHA-256 values
`1b35b052b10603a7a11d2976e8b7dbb71d23781919e99f3023317170c1ab3142`
and
`be3df59d21eac14f3587b90a52d1df8291b5ac7664a8794091c3c29b49bb4d3f`.
The complete async Chromium Page-route file is 38 passed / 6 failed, SHA-256
`876cbfb7d06cbdff10d5485425f3d0ddbfed8f3c175fdf784c1a5862a68ea108`.

Installed-wheel units pass 190/190, SHA-256
`37c173a84740b5e8f628c01477344a0ebac431f31d772cdbda583ca920baa61f`.
The all-feature host library passes 1,023 tests with 26 declared ignores and
the addon passes 2/2 with CPython 3.12 pinned for linking. Focused
exact-identity tests, strict all-feature Clippy, formatting, Ruff, Pyright,
source validation and the 108-package dependency proof pass. Final repaired
ABI3 wheel SHA-256 values are
`8854082265f32fb36a8de0b8507fcde80a354b7c8968b88cb8d4e18ad6b138e4`
and
`b7bbd2251e26ee574d9521c87b0418b74955e1c7451277c3fb99f0ca89d5544d`;
their payloads reproduce after normalizing only CycloneDX identity.

Fresh final-wheel collection is 6,526/6,526, SHA-256
`ce7d3084b28837993310a53bb6635d49d25cdf60f042f683c645321fc6ee6bd4`.
The selected Chromium row moves from `not-run` to `passed`. The cumulative
ledger is 848 passed, zero failed, seven skipped and 5,671 not run, SHA-256
`58c32ca19ac776657dcfd4e823ca58d9d57acf519b5c47d371f3c2deb12520d3`.
The six remaining Page-route discrepancies and the broader Python and Core
JS/TS corpora stay open.

## ABI 90 composable route binding and fulfill path checkpoint (2026-10-05)

NODE-341 makes Playwright's language-native route matchers composable without
moving Python callable execution into Rust. A regex or predicate registration
retains its original matcher and handler identities in the binding and
installs a bounded native `**/*` route. A nonmatch calls Rust-owned
`Route.fallback()`, so LIFO traversal, effective overrides and final
settlement continue through the same Rust route owner. Binding-owned `times`
counts only matching invocations, removes the Python registration before its
last handler runs, and defers native unroute until settlement. Dispatch waits
for an actual route action, including handlers that schedule it in a task.

NODE-342 adds a separately selectable `route-fulfill-path` Rust leaf. It
accepts a UTF-8 path bounded to 4 KiB, opens only a regular file with Tokio,
reads no more than the existing 1 MiB route-body limit and replaces any
caller-supplied content type with extension-derived MIME (or
`application/octet-stream`). Python passes only path, headers and status to
this operation; it never reads the file body. Wheels that do not select the
feature keep the operation unsupported.

The fixed two-row upstream manifest hashes to
`4090d9b980004ab051b3f3078d4fee0fec7f2360fc7c7d802788cd8b45e72dad`.
The final installed candidate and independent oracle pass 2/2, with report
SHA-256 values
`35a123a706cf2c645b7db4afab7d67b50f8253aaa76b359a13b508fb2289d451`
and
`0ba256ac00477dd62dee767a5e7b513e2e49fc60e889629e5382a4b318304d35`.
The complete async Chromium Page-route file is 37 passed / 7 failed, SHA-256
`489b44b15c599c0e7bf7ae83c5a6d5057fded99c8ee8f23b30e57424451c3dfa`.

Installed-wheel units pass 189/189, SHA-256
`68eb30632c02df8ecc93d924cca1473f5dd944ed824132fed50cb6091c23b228`.
The all-feature host library passes 1,022 tests with 26 declared ignores and
the addon passes 2/2 with CPython 3.12 pinned for linking. Focused feature
tests, strict all-feature Clippy, formatting, Ruff, Pyright, source validation
and the 108-package dependency proof pass. Final repaired ABI3 wheel SHA-256
values are
`7652eeab5827eccb57c120dc0efcf2351f83b45a3fc9f087525b958459c99a34`
and
`4aed15bf47721a69bea6dbfd4e5b48de4eaecbca9fbbf5be01ce19416a50f2b7`;
their payloads reproduce after normalizing only CycloneDX identity.

Fresh final-wheel collection is 6,526/6,526, SHA-256
`705b261e7ad2ded37e160f55c8efb4ef226121869df74d636b1c3e7395567254`.
Both selected Chromium rows move from `not-run` to `passed`. The cumulative
ledger is 847 passed, zero failed, seven skipped and 5,672 not run, SHA-256
`e423606ad57b9f2e0bdb95f4e4a74e71d65ea9582890b864e86ba76837817028`.
The seven remaining Page-route discrepancies and the broader Python and Core
JS/TS corpora stay open.

## ABI 89 exact request Frame identity checkpoint (2026-10-05)

NODE-340 keeps Frame ownership composable while matching Playwright's exact
object identity. Routed requests resolve Chromium `Fetch.requestPaused`
`frameId` through the bounded topology before publishing one `RouteCall`.
Ordinary network events keep a bounded native Frame descriptor in the existing
Rust network adapter and transfer only the resolved portable Frame ID across
FFI. Python looks that ID up in the Page's existing Frame registry; it does not
create a second Frame owner or guess from URL, resource type or timing.

Availability is cursor-relative. An explicitly created Page seeds the network
adapter with Frames that existed before its independent CDP cursor. A newly
discovered popup does not seed future topology into its already-buffered first
request, preserving Playwright's `Request.frame` unavailable error at that
boundary. Later Page replacement may seed the complete current Frame tree.
`Response.frame` delegates to the exact Request. `ElementHandle.content_frame`
uses the existing Rust Handle-to-Frame operation and resolves the returned
portable ID through the same Page Frame registry. Async and sync facades retain
only the Page identity needed for that lookup.

The six-row request-frame manifest and one-row route-frame manifest hash to
`8f091b09f62bc27caac9e620f5df9664898e273fc58e8e0b4c45adaf5d063579`
and
`79dc12c8f2b1bb809a6bb060b7eb03796af8e5ae8061fa9a0b00752dab42a8ef`.
Final installed-candidate and independent-oracle reports pass 6/6, with
SHA-256 values
`0ce2544dc09b85e6e8d7574fda6dbaa88194f2a06b1321527fc8747fbb4249d7`
and
`fd4961dc9f534971a2d8805592ac222b8ae6d4ca62695c56593dede70ae75b0d`.
The route reports pass 1/1, with hashes
`12b08216247c408f5023fb11fb1caf8df9f0219c74502a9f62ce032f8a1cc503`
and
`e69ad4eaa22ab49b41664a9488260158bc79bfebb0d4ac0b9f2d9bf8156e87b2`.

The complete isolated async Chromium Page-route file is 35 passed / 9 failed,
SHA-256
`122d5db740b95603aecee9d70713d465df19be09c60daac55d1898510cb32e4d`.
Installed-wheel units pass 185/185, SHA-256
`1c76a5b0370195fd58cebf2cd62cde5253eb92e4fe3b064d4010c37d2f12c3bb`.
The CDP network leaf passes 14/14, the exact composition seed regression passes
1/1, the all-feature host passes 1,043 tests with 36 declared ignores, and the
addon passes 2/2 with CPython 3.12 pinned for linking. Strict Clippy,
formatting, Ruff, Pyright, source validation and the 108-package dependency
proof pass.

Two final repaired ABI3 wheels have SHA-256 values
`506b898759a6247ac3fea50349db724186511af015d40f80b1912351d05b4551`
and
`ad67b81fb4c767b2631b6ad827f4ef23934e79c07526d243abe8c0dd8a985b71`;
their payloads reproduce after normalizing only CycloneDX identity. Fresh full
collection remains 6,526/6,526, SHA-256
`8c2185ef4d17709499633e9c7e1a9d442ecc595adfe1d6c3f2c8f57ceab2d299`.
Five request-frame rows plus the route row move from `not-run` to `passed`; the
popup-first row was already passed at NODE-300 and is replaced with stronger
evidence. The cumulative ledger is therefore 845 passed, zero failed, seven
skipped and 5,674 not run, SHA-256
`673973f03e77d0291596cfbc971579c70d98831f634ab74606a43b37282375c7`.

## ABI 87 request lifecycle events checkpoint (2026-10-05)

NODE-339 completes the Page request listener lifecycle without adding another
network implementation. The existing Rust `request-events` installation now
subscribes to request, request-finished and request-failed kinds and publishes
all three through its one bounded FIFO. A failure notice includes Chromium's
failure text. Python keeps one Browser watcher, reuses the Page weak request
map by portable Rust request ID and dispatches only `request`,
`requestfinished` or `requestfailed`; it owns no protocol subscription or
terminal-event queue.

Navigation `Response.request` cannot be correlated from Chromium's loader ID
to the portable request ID by comparing URLs. Rust request history therefore
retains the exact native request ID beside its portable identity and exposes
that value only to the internal FFI descriptor. Python uses the exact match to
select the already wrapped Request; the public object remains keyed by the
portable ID. Request-history, request-event and response-event Context
subscriptions share the same native activation and retirement path.

The fixed two-row manifest hashes to
`f5cb093c7321da1f8a0ad597fb108cbe26f636dabe37956495cefc2ce3c5e0d4`.
An intermediate installed candidate passed the `requestfailed` assertion but
failed the navigation request-identity assertion, recording 1/2 with SHA-256
`69fc97709bfeb8d65bedb6d219795ad39013f090cf2e7f659ef40fc31de4b36a`.
The final installed candidate and independent oracle pass 2/2, with report
SHA-256 values
`daa8311542664a25242b7c0877c98b03302b5b7542f13c0b05c4ca6cad050811`
and
`a8f35888e8ba83c2b8eca51df9ef6b975c860084a77ebdf571a502d86c1b281c`;
five further candidate executions pass 10/10.

The final complete async Chromium Page-route file records 34 passed and 10
failed, exactly two more passes than NODE-338, SHA-256
`046feb19615e7e5aee2c37271108f4f0c3705dad7b7c0112dc61ee2ddca87ae8`.
Installed-wheel units pass 182/182, SHA-256
`4391fd06e07e6057c76385894676a84249bdd6be3692d213c7636b7b274afdaf`.
The all-feature host passes 1,043 tests with 36 declared ignores, the addon
passes 2/2 with the ABI 87 Python pinned for linking, and strict Clippy,
formatting, Ruff, Pyright, source validation and the 108-package dependency
proof pass. Repaired wheel SHA-256 values are
`b575ae94a422984d97182d332895860468612c95353a70c122e7e4c65286aaa7`
and `39258c83dacd2fc18277fbbd2cf893c55a698ee2d3a04c4b7f3d3d9b2d7112d5`;
normalized payload comparison passes. A fresh collection remains 6,526/6,526.
The cumulative ledger is 839 passed, zero failed, seven skipped and 5,680 not
run, SHA-256
`b8c7bebbeb3f2c16880fb7fc08dd30b15d56d504d091c264b732641ef8e81e0f`.

## ABI 86 Route.fulfill status-text checkpoint (2026-10-05)

NODE-338 changes Rust behavior without changing the language ABI. Upstream
Playwright sends its own fixed HTTP reason phrase with every Chromium
`Fetch.fulfillRequest`; relying on Chrome's default makes status 422 surface
the newer `Unprocessable Content` instead of Playwright 1.58.0's
`Unprocessable Entity`. The Rust route owner now selects the complete pinned
Playwright mapping and serializes `responsePhrase` beside `responseCode`.
Unlisted but valid 100-999 statuses use the same `Unknown` fallback. Python
still transfers only status, headers and body and owns no protocol policy.

The one-row manifest hashes to
`5db202122947113cacf31d4ac4854f9c6b2715d092a09cd36efd1303936a0d79`.
The exact ABI 86 red selection fails 1/1, SHA-256
`64085cc07ee62904297a5ebd94dc72aff436dffa04fe2c7d9338d9a2779e2614`.
The final installed candidate and independent oracle pass 1/1, with report
SHA-256 values
`03c00a71727f7ec0bf7d0e33c902176c9a079a79b0888c5af87a138dc057b3d4`
and
`40cc799e2e10b0938defd95d5e22de0b4aafb9e34711f9e23a3d211455473cba`;
five further candidate executions pass 5/5.

The final complete isolated async Chromium Page-route file records 32 passed
and 12 failed with no regression, SHA-256
`b1cbd5689624ca8b3e24a5c301d5522b4a511781f0bbbd58dca02ba9611e6769`.

Three focused Rust regressions pass. The exact `page-routes` host profile
passes 517 tests with eight declared live ignores, and installed-wheel units
pass 180/180, SHA-256
`4d8901e610cbf4788e582dace13d1f9a564a5bd7181041d1acfe1e5154713515`.
All-feature strict Clippy, formatting, source validation and the 108-package
dependency proof pass. Repaired wheel SHA-256 values are
`68a3b7f6022a39ea9d238cce6a5227dea0c471cd405a4b13a1dfb06f8a6335df`
and `3ce3dc51e847b16e9e397b9f0cd1f8bc9a2d2db3cd56bfabab813b0822e8e1a6`;
normalized payload comparison passes. The unchanged full collection remains
6,526/6,526. The cumulative ledger is 837 passed, zero failed, seven skipped
and 5,682 not run, SHA-256
`8a6d6a9169086e482fbd8acf6d81162ef42ab209f18c2c174238e5feb85e2ef5`.

## ABI 86 public route-header evidence checkpoint (2026-10-05)

NODE-337 changes no ABI or runtime code. It promotes two further consequences
of NODE-336's Rust-owned request-header normalization: Chromium-native
`Referer` and `Origin` names are exposed through Playwright's lowercase public
mapping. The fixed two-row manifest hashes to
`1e705cd72b24e45d93d4242d8698c43d55c302c2676fcb0736beaec1c1b724ce`.

The exact ABI 84 baseline fails 2/2, SHA-256
`70f093d70526a1aa906dd0a64f044f230993dccc0db334b69bb1ee5fc71c0f9a`.
The final installed candidate and independent oracle pass 2/2, with report
SHA-256 values
`c40e734175d114ff40ba403eb90072d38bc2f20fceb90e484f7ef5eaa38f6676`
and
`ea9d449ef2876572f092f361adeff15c70ee982ae7292edc979b55385d9cc4f2`;
five further candidate executions pass 10/10.

The complete isolated async Chromium Page-route file now records 31 passed and
13 failed, SHA-256
`c1839f9643e9a6c411cb15966a73587bb8f7484db46454f1caf4fb5bb88eef92`.
The data-URL and equal-request rows happened to pass but remain outside this
checkpoint because earlier diagnostics identified them as unstable. The
cumulative ledger is 836 passed, zero failed, seven skipped and 5,683 not run,
SHA-256
`eb6b687f25461fa8d8f4182455215dbcf565a47296427680357866df79cc8553`.
The 13 Page-route discrepancies, remaining Python and complete Core JS/TS
corpora stay open.

## ABI 86 Page and Context extra HTTP headers checkpoint (2026-10-05)

NODE-336 exposes async/sync `Page.set_extra_http_headers()` and
`BrowserContext.set_extra_http_headers()` as thin language facades over the
existing independently selectable Rust header installation. Python validates
one string mapping, transfers owned entries once and retains no parallel
header registry. Rust owns Context/Page precedence, exact Page/Context
identity, native session fanout, replacement, clearing, compensation and
retirement.

Playwright uses a Page `referer` header as the implicit `goto()` referrer and
also exposes it through `Route.request.headers`. Chromium can omit that value
from `Fetch.requestPaused`, or publish it as `Referer`. The Rust route event
driver therefore reads the Page-specific original header snapshot only when
the native event has no referer, adds one bounded value before publishing the
route call and normalizes every CDP request-header name to the lowercase
Playwright public shape. Native values still win, Context values are not
mistaken for Page navigation referrers and Python never fabricates a request
header.

The fixed three-row manifest hashes to
`53eca6f3debffcd3cc338be1fc229effc2e5d6440c3087060f76bc1fb3b40c86`.
The exact ABI 84 baseline fails 3/3, SHA-256
`774e9c951f6ce2f2e784f6a6f4d5d1fb984bd31c8666f1fc5b88007a385be4fd`.
The final installed candidate and independent oracle pass 3/3, with report
SHA-256 values
`ac965349486711d65999665466d6509c4fdc76eb6347c563bc47bb4488f98cd1`
and
`9c6b093150fac06db4032e62446601a7510035f96782967ae04e50b6363135e5`.
Five further candidate executions pass 15/15.

Installed-wheel units pass 180/180, SHA-256
`e900850fb3d13d3763e58db2bb8414e84597724700c692f884104d730961b770`.
The exact `page-routes,extra-headers` host profile passes 552 library tests
with eight declared live ignores. The all-feature host passes 1,018 library
tests plus 22 integration probes, with 26 plus 10 declared live ignores, and
the addon passes 2/2. Strict Host/Addon Clippy, formatting, Ruff, Pyright,
source validation, the 108-package dependency proof, normalized
repaired-wheel comparison and full 6,526/6,526 collection pass. Collection
SHA-256 is
`88818b94cdd28d394110a631df95a7d31f272db6da91320ebb626e9ef5ec26a1`.
The cumulative ledger is 834 passed, zero failed, seven skipped and 5,685 not
run, SHA-256
`a0c1caa9d38619302ae8b0eaa49450d1c9db9a5db80cb44e4fa6fb5e69c4b783`.
Regex/predicate matchers, `unroute_all(behavior="ignoreErrors")`, remaining
Python and the complete Core JS/TS corpora stay open.

## ABI 85 route fallback and handler-unroute checkpoint (2026-10-05)

NODE-335 keeps interception control in the existing bounded Rust owner. Each
dispatched call carries a traversal cursor after its selected registration,
the original request fields and one accumulated `RouteContinue`. Fallback
walks Page registrations newest-to-oldest before Context registrations
newest-to-oldest, rematches each candidate against the effective overridden
URL and replaces only explicitly supplied URL, method, header or POST-data
fields. Fulfill and abort stop the chain; exhaustion emits one native continue
with the accumulated overrides. Python never recursively selects handlers or
owns the paused request.

Handler-specific Page/Context `unroute()` resolves the public string pattern
to opaque Rust registration IDs and verifies their exact scope before removal.
Async Python stores the public handler key beside the callback. Sync Python
passes the original user callback as that key rather than its async adapter,
so identity removal matches Playwright without duplicating route state. The
existing finite `times` cleanup removes both callback and key metadata.

Chromium may independently request `/favicon.ico` between the two navigations
in the Page handler-removal case. The pinned upstream Playwright server aborts
that favicon before route interceptors; the Rust event driver now applies the
same internal abort before user-handler matching.

The unchanged eight-row manifest hashes to
`ae8c779a13000d341ed5a6ec32c82249c096f2ac0bfb317f5df2a7fd309676c3`.
The exact ABI 84 Page and Context baseline selections hash to
`dc8b0f6daf9036adeba4cf8ea439a85fede9d72f4770644eaa16c707be01e787`
and `8fcfb19ada84f4004f458ff21122416a03ee86ea1ed5d7506c15511bcb0dcb34`
and contain seven failures plus one existing abort pass. The independent
oracle and final candidate both pass 8/8; their SHA-256 values are
`be6df6716b3b913e20d40a0cf94185ecdf6d750d5ca67489a7bc46afbc2fd379`
and `07784087e2516f57e39a0d5567c2fcc8266c58842f6d6f4fb939c0e67734179f`.
Five further candidate runs pass 40/40.

Installed-wheel units pass 176/176; the all-feature host passes 1,015 library
tests and 22 integration probes with 26 and 10 explicit live ignores, and the
Python addon passes 2/2. Strict Clippy, formatting, Ruff, Pyright, source
validation, the 108-package dependency proof, normalized repaired-wheel
comparison and full 6,526/6,526 collection pass. The collection SHA-256 is
`985e35d5b9645cf76942d0922d72231beb7ec8b43cd188094fdeca09dac4dcc6`.
The cumulative ledger is 831 passed, zero failed, seven skipped and 5,688 not
run, SHA-256
`4303259b2a6813aa7e3fc8017d89d1cc5d62a38bed6b66eacc3af5600d576722`.
Regex/predicate matchers, `unroute_all(behavior="ignoreErrors")`, remaining
Python and the complete Core JS/TS corpora stay open.

## ABI 84 finite route-use checkpoint (2026-10-05)

NODE-334 keeps the route-use budget in the same bounded Rust owner that owns
matching and paused-call settlement. A registration stores `Option<u64>`;
positive `times` values decrement while the state mutex is held and the final
match removes the registration atomically. `times=0` is normalized to
unlimited, matching the upstream binding's falsy-zero behavior. Matching is
still evaluated even when only one registration survives, so a transient
stale CDP pattern cannot select a nonmatching handler.

The final finite use still owns its paused request until `continue_`, `abort`,
`fulfill` or callback-error settlement completes. Only then does the host
refresh Fetch patterns across the affected current Page or Context sessions.
If Chromium emits a pause from its old pattern during that interval and Rust
finds no registration, the event driver issues the existing native
`Fetch.continueRequest` action; it does not allocate a Python callback call.
The ABI tuple carries one `expires` boolean so Python can discard the handler
identity before invoking its last callback. Direct `not_to_have_*` aliases for
the already-supported assertions simply delegate to `.not_to`.

The unchanged three-row manifest moves from ABI 83 candidate 0/3 to independent
oracle and final installed ABI 84 candidate 3/3; five further candidate runs
pass 15/15. Manifest, red, oracle and final-candidate SHA-256 values are
respectively
`9c90aed2f4cb8c20c3d61bf5905935d8d7bd2a5d87e7634208c346713f4435db`,
`08ca9be33001a14352dfab498a58370a45f74cdca9fde1f4c4b5d2ce6f675c80`,
`c980906a95028171bb31e63cd68a9443c53476f52da01dd68c0187a179d9feba`
and `d8e9a73a823ebd2134de4b21dfaed9687426af02e17572d6ce386d35b81e0faf`.

Installed-wheel units pass 173/173; the all-feature host passes 1,011 with 26
explicit live ignores. Strict Clippy, formatting, Ruff, Pyright, source
validation, the 108-package dependency proof, normalized repaired-wheel
comparison and full 6,526/6,526 collection pass. The cumulative ledger is 823
passed, zero failed, seven skipped and 5,696 not run, SHA-256
`26023955d6cd01d96099cf7b07ef479d028e86a9fd60038e14c786479f467439`.
Regex/predicate matchers, route fallback, handler-specific unroute and the
remaining Python and complete Core JS/TS corpora stay open.

## ABI 83 route-glob question-mark checkpoint (2026-10-05)

NODE-333 aligns the two matcher layers instead of hiding their disagreement.
Playwright route globs do not use `?` as a single-character wildcard. Rust
therefore compares it literally, while the CDP compiler parity-escapes every
unescaped question mark before `Fetch.enable`. Already escaped question marks
remain single escaped literals, and even backslash runs gain exactly one slash
so CDP still sees a literal question mark. A matcher-only intermediate was
intentionally rejected after Chromium paused `/index1hello` while Rust did
not match it; the final design makes native prefiltering and authoritative
selection identical.

The unchanged official row moves from candidate failure to independent oracle
and final installed candidate 1/1; final candidate repeat passes 5/5. Manifest,
red, oracle, matcher-only diagnostic and final-candidate SHA-256 values are
respectively
`9b8fda4e880a63e197a938b79cc32816f04a77ef56c5e167137083f5007b19ca`,
`e18f320aad2fdc7d251ebdc778151c6236ba07d7c006fd41c7cc79cc90bc0428`,
`151de8226947e0f5b3c6a1c5b1678b334acca6b415e24a15986fc57c443051f5`,
`e92ca82802ab51fd848ec1362f83a2e7339a523f725af2470dd5e99b3927093c`
and `54ddbab3174694286c83fa4b3c44c4e133ba507aa28ea734a929a532c625d0bd`.

The cumulative ledger advances to 820 passed, zero failed, seven skipped and
5,699 not run, SHA-256
`aaa91984391a096225cae1684c0f1879249b41feedd8efad7a5203053087e6d0`.
The final full Page-route diagnostic is 22 passed / 22 failed. Its two
apparent losses relative to NODE332 reproduce on the unchanged NODE332 wheel,
including an unstable equal-request row, so they remain diagnostics rather
than attributed regressions. The broader Python and complete Core JS/TS goals
remain open.

## ABI 83 Route continue-override checkpoint (2026-10-05)

NODE-332 makes `Route.continue_()` a composable value instead of a collection
of binding-side special cases. Rust owns optional URL, method, headers and
POST-data overrides, validates their independent and aggregate limits, checks
that a replacement URL preserves the original request protocol, and emits one
`Fetch.continueRequest`. Explicit empty headers stay distinguishable from an
omitted override and POST bytes are base64-encoded only at the CDP boundary.
The pending call retains only the bounded original request scheme needed for
same-protocol validation. Python normalizes public values and retains no CDP
request id, request scheme, paused call or override state.

The ten unchanged Chromium rows in async `test_request_continue.py` are pinned
by a full-file source hash. The ABI 82 red report records 2 passed / 8 failed;
independent oracle and final installed ABI 83 candidate both pass 10/10.
Manifest, red, oracle and final-candidate SHA-256 values are respectively
`3a53fb8247e6e84bf7b4bfc29d332dd9b7e7794f3bab5e024c28b73763b49a31`,
`d8667b8e430005fe7de4ffe9296339dad7f69c5f302db315a2f7ccd5ce4de4de`,
`cf8e21961e1ddda7a958a4e95d7b016bdc4cfa5f7ea5555df13442a674ea49c9`
and `94d93a7649c66c65076ea7f15d500cf9fefe9cb4aa1bbaa3cd13cd60e88449c3`.

The complete 44-case async Page-route file now records 23 passed / 21 failed /
0 errors, up from 18 / 25 / 1. Installed-wheel units pass 172/172; the all-
feature host passes 1,007 with 26 explicit live ignores and the addon passes
2/2. Strict Clippy, formatting, Ruff, Pyright, source validation, the 108-
package dependency proof, normalized repaired-wheel comparison and full
6,526/6,526 collection pass. The cumulative ledger advances all ten rows from
`not-run` to `passed`: 819 passed, zero failed, seven skipped and 5,700 not
run, SHA-256
`43861212714734aa05343ef0ff48a87977136adbaecfe9d0c3c461bfb50af2f6`.
Remaining Page-route, Python and complete Core JS/TS discrepancies stay open.

## ABI 82 full Page-route baseline checkpoint (2026-10-05)

NODE-331 changes no implementation or ABI. It executes all 44 Chromium rows
from upstream async `test_page_route.py` in separate subprocesses, with a
30-second hard timeout on every row. The installed-candidate baseline is 18
passed, 25 failed and one error. The full report SHA-256 is
`1f40d74f878e0729ddbb7acf7dbd0a3f98c11cabc1530d78860a91ef983a50f4`.

The 18 supported rows are fixed in a source-hashed manifest and independently
rerun against candidate and oracle, both 18/18. Manifest, candidate and oracle
SHA-256 values are respectively
`da5aacd59a19ee1852ffe7d7c0918a32cd8115895ce770ee409523297f8cdece`,
`262b71d1c0a5b814bd0e4a8560d84077d19a60e29cad6ebb00f8783c06981102`
and `bf78f4ccd0177fada706551b22190f7c3af3bdd6db00a62a2bde20121908070e`.
The ledger advances to 809 passed, zero failed, seven skipped and 5,710 not
run, SHA-256
`0408aef3e6fa958f60ce74998e4326e40aef2bc906851efa35e9b656e9b5940c`.

The remaining 26 rows stay explicit. Their first-order clusters are continue
overrides; regex, handler-specific unroute and `times`; requestfailed and
redirect-chain publication; service-worker/data-URL interception rules; Page
headers; CORS preflight; fulfill path/response/status-text handling; and one
resource-profile setup error. No failure was converted to a skip or weakened.

## ABI 82 expanded unroute-all evidence checkpoint (2026-10-05)

NODE-330 changes no implementation or ABI. It promotes six unchanged official
rows that exercise NODE-329's public callback path directly: four async/sync
Page/Context default removals and two async in-flight waits whose active
handlers fulfill after three seconds. Candidate and independent oracle both
pass 6/6, proving that default removal disables the exact scopes and that
`"wait"` neither returns early nor implicitly continues a paused request.

The default and wait manifest SHA-256 values are
`f8b3136e07261dcad3ea631ace025ba68309c8db5c6e1f1b3ddc37be73a58c00`
and `329eee6cd757a0fa7c8f03f0b61343c7df627bb9717a9b4f0bf143eb47816998`.
Candidate async-default, sync-default and wait reports hash to
`fc09e9ec6ac97192b57d196370a7d36538ee3b2a9b8eb95b769b3b8dac0f3784`,
`aa3a0118bfd83ae64365e373ffca2296b6a89aeae34b208d777f1e813dc8d195`
and `6ae928c2191dd83930721db5ae54b65e62901637d5e95d7b56067242d1274505`.
Oracle async-default, sync-default and wait reports hash to
`fd9b71ac2c245daf83ada3c94fb05eb86ae18742e1a807990d219c95127f83e4`,
`1d48ce9de9358807754f32653326ccafabae9e32e553bdf39e64d745856118f3`
and `c1b746273ecf5984442759529e21a8b47c27b8a64f2532d0759f8dd9be9363d4`.
The cumulative ledger is 791 passed, zero failed, seven skipped and 5,728 not
run, SHA-256
`d7255f3d41c76a26528036c0eaee54b788a48b2323650b3700f2da7441f7cb05`.

## ABI 82 scoped unroute-all checkpoint (2026-10-05)

NODE-329 adds async/sync Page and BrowserContext `unroute_all()`. Rust removes
every registration in the exact Page or Context scope, derives the affected
sessions from authoritative topology and refreshes their union of Fetch
patterns. `behavior="wait"` also waits for both paused route calls and actions
already dispatched to Chromium. Those actions carry their original
registration scope in a fixed 64-entry settling table; a cancellation-safe
guard always retires the entry and wakes waiters. Python clears only its local
callback identities after the native removal completes. Default behavior is
supported; `behavior="ignoreErrors"` is rejected explicitly until its error-
suppression contract is independently designed and tested.

The Page case exposed Chromium's late error-document race after an aborted HAR
navigation. The navigation leaf now tracks a bounded three-state newest-
document marker from `Page.frameStartedNavigating` and Document
`Network.requestWillBeSent`. A mismatched error commit is treated as stale only
when it predates the current cursor or the expected loader is newest; an
observed competing loader still produces the existing interruption error.

The fixed two-row manifest SHA-256 is
`9288dc65838089a70eae6b2cf258e6efaf82727286da0432d6f0125ac0cee429`.
The red baseline failed 2/2 because both public `unroute_all` methods were
absent. Final installed candidate and independent oracle pass 2/2, and the
candidate repeat report passes 20/20. Candidate, oracle and stress report
SHA-256 values are respectively
`d00a82c9a9c449d4f74e322273aae6c6be4fd2cfe4283d764bd1a063385b96ed`,
`e336fb07d6f408e376fe1f83256c419f1d4137cae151d4238a938ca63651dbe1`
and `873eb71d2854082eb257add3d3ea9a40753c74bb8a16a66d199f2b53793b5b04`.

Installed-wheel units pass 171/171; the all-feature host passes 1,004 tests
with 26 explicit live ignores. Strict Clippy, formatting, Ruff, Pyright,
source validation and the 108-package dependency proof pass. Full collection
remains 6,526/6,526. The cumulative ledger is 785 passed, zero failed, seven
skipped and 5,734 not run, SHA-256
`b58f55b19a62286fdf0c1b06040c017c1d9b00666b3e139847eaf8cc42027b6f`.
Installed-unit and collection report SHA-256 values are
`47a06e7fcc8ddb619ff4c6780478cb46fa7f96ae795046322900f7a5ea44bb1c`
and `d019672bb39ca41a5c91f97150722632dc0edc1d4d57b090cca522fb915ea7b7`.

Two normalized-payload-reproducible repaired wheels have SHA-256 values
`7323c7265dd3c3ac5d230391c57e7547d95de459ded20bd8009f9a84157c91b8`
and `af51f4861ec3723518203610fa673b948ce38ebb52f1e62ef0772140c58d1c9e`;
their byte-identical native extension SHA-256 is
`d3175b42fe1ebf395c0c07e9285a99d104d17fee19b77162cc980e9c26df7465`.
Handler-specific unroute, `ignoreErrors`, regex/predicate matchers, `times` and
external/ZIP/update HAR remain future capabilities.

## ABI 81 BrowserContext route composition checkpoint (2026-10-05)

NODE-328 adds async/sync `BrowserContext.route()` without a second matcher,
Fetch driver or topology. Rust registers an ordinary binding with explicit
Context scope, resolves exact paths through the authoritative Context
`base_url`, initializes every current/future Page and OOPIF session, and
selects Page scope before Context scope. Python retains only one handler per
opaque registration ID and dispatches the exact registration selected by Rust.

The fixed four-row manifest SHA-256 is
`18679b37e9b90eb925e3732f129d06eed488211246b912eb2770a803ed1622cd`.
The red baseline failed 4/4 because `BrowserContext.route` was absent. Final
installed candidate and independent oracle pass 4/4; candidate stress passes
async 20/20 plus sync 20/20. Candidate, oracle, async-stress and sync-stress
report SHA-256 values are respectively
`9d47307120b6db7534e0d472e5e9c7fd7d460daa993dceb091e0cc0bf7639648`,
`a122f8861f9e1ec313e426e40d876bab7581c4272d1274a9b028f590c1507af3`,
`58b080019c7abdfaedc5180ad2b7f4c32f86c9a4748d4692dde45e7146ae16d9`
and `326124751fe9e276c07af296f25f6dd68f384ace35f6f295fcfd08b998d61502`.

Installed-wheel units pass 169/169; the all-feature host passes 1,003 tests
with 26 explicit live ignores. Strict Clippy, formatting, source validation
and the 108-package dependency proof pass. Full collection remains 6,526/6,526.
The cumulative ledger is 783 passed, zero failed, seven skipped and 5,736 not
run, SHA-256
`c0c16280690a496ffdc374fa77ad1792aedd6670a9c61026d70fc042bce77b17`.
Installed-unit and collection report SHA-256 values are
`81d61edc55aa158cf023229182cf6aa8497ee12ffd5dfb65426f2931aa227158`
and `c745b3f8bd9deebc6deb2e36a4567137b54e9589f5a28910bb822f6df8d086d0`.

Two normalized-payload-reproducible repaired wheels have SHA-256 values
`8e094a57d7b1e407fbad9a4c8adb3fe119f59f856e53d2cca905db20cad7108f`
and `f446b28128ff4d9dead74b4abc2f3d3613459e9fa5e90a0468f2df8dcab2fc56`;
their byte-identical native extension SHA-256 is
`0a3f95aaddc7d406562603d533f8933c7603ac566c0c5e4b58b9f0d7770a2155`.
Regex/predicate matchers, `times`, unroute variants and external/ZIP/update HAR
remain explicit future capabilities.

## ABI 80 missing-entries HAR fallback checkpoint (2026-10-04)

NODE-327 keeps ABI 80 and changes one backend-neutral parse rule: a present
HAR `log` object whose `entries` member is absent becomes an empty archive.
Lookup therefore returns `NoEntry`, after which the existing Rust not-found
policy either falls back or aborts. Invalid JSON, missing `log`, malformed
entries, external content and every archive/field/count bound still return an
explicit registration error.

The fixed async/sync manifest SHA-256 is
`6d26bdd0caa16f37de43ec9c45ddc6d3534260f12d54dea5cf2e39349ceccc6a`.
Final installed candidate and independent oracle pass 2/2; candidate stress
passes async 10/10 plus sync 10/10. Candidate, oracle, async-stress and sync-
stress report SHA-256 values are respectively
`407c2a8b3c60d4044c56cd3cab6c3ecc2bb2185980a7f24ab324d0359313cc3b`,
`09b8ac3378150efea8cfe01f0d09cc81169eb9ba520e1d78a7a966c69297f500`,
`db2cfd2bb97faa0327784954bd913377ceb5d86b2a7f2f8d229750193cbc0d5b`
and `e7df792c9dabb9a14aef8bb5eeb71e825c458b164a13ee412840d2a595303a0f`.

The HAR policy crate passes 5/5, installed-wheel units pass 168/168 and the
all-feature host passes 1,003 tests with 26 explicit live ignores. Strict
Clippy, formatting, source validation and the 108-package dependency proof
pass. Fresh collection remains 6,526/6,526. Installed-unit and collection
report SHA-256 values are
`d17371760e41463a92414619034300932367f6beef78373b37d2648c2b346589`
and `eaa5f82eb7652df1d0c02f97a182dd27eb988a676225b3403787ab7de8b3cb04`.
The cumulative ledger is 779 passed, zero failed, seven skipped and 5,740 not
run, SHA-256
`a545a96742ef986a132be551627450e08c8d1fd0ecbfca8a1fe8e5ab142328c8`.

Two normalized-payload-reproducible repaired wheels have SHA-256 values
`c0c288d765c1afa1099093bd63743b5b5be72b26c1296561d374c60140733d74`
and `c15e0eb94e6d7ed6701e0dcc7948119c996cee4b9f0cfa9409d93754f51d2fe2`;
their byte-identical native extension SHA-256 is
`d5c19d2c34536a8dd6a0c6a8ccc72b4d168fe8dd0b0124847a6fa7eef473b7be`.
Repair copies the required OpenSSL dylibs into each wheel.

## ABI 80 BrowserContext HAR replay checkpoint (2026-10-04)

NODE-326 adds async/sync `BrowserContext.route_from_har()` without adding a
second archive parser or Fetch driver. Rust registrations carry an explicit
Page or Context scope. Page scope always precedes Context scope, newest
registration wins within one scope, and Context scope remains registered while
temporarily page-less so every future Page/OOPIF session receives its Fetch
patterns. Initialization publishes a bounded 64-entry exact Page/Context/
session bridge before native Fetch setup; it catches an early
`Fetch.requestPaused` event and retires on initialization error, target detach
or authoritative topology reconciliation.

The fixed manifest pins the unchanged async/sync method/redirect, fallback-
continue and default-abort rows. Its SHA-256 is
`7ed9f484469317a0f4e979d9314ad7b4d203a30e056ea9f2d4e4a24b5219c4ea`.
Final installed candidate and independent oracle pass 6/6. Candidate stress is
split by public API and passes async 30/30 plus sync 30/30. Candidate, oracle,
async-stress and sync-stress report SHA-256 values are respectively
`c1e79d7c9ad98826a6d39460e70161390de47d45b64dcd0dbc5acd6aacf8608d`,
`2d52861f82472f55e8d56f0ae286a322564921f0f2053ed6e75683402c058914`,
`b49a090ca4da67775d9d1ebbd2ea028ba43039841a1fbe8c1a921c3d03e0566a`
and `3acb6037da23c23c6b1505e8e4b7b3091cb9ea8f8f9e563b39f30ad80ffb8a92`.

Installed-wheel units pass 168/168; the all-feature host passes 1,003 tests
with 26 explicit live ignores. Strict Clippy, formatting, targeted Ruff,
Pyright and the 108-package dependency proof pass. Fresh collection remains
6,526/6,526. The cumulative ledger advances to 777 passed, zero failed, seven
skipped and 5,742 not run, SHA-256
`c52d8ae2a549fc2cfa75e745969863363c8e10705daea947e46a9446d58b89a7`.
Installed-unit and collection report SHA-256 values are
`e3ca609d17b6c108dc059d20bf2e656f3e70620d3c15d7fe19afa4109e3fa034`
and `c77f442acb9602f67b5f036a9905b92808d88cdb5f9f599e68718da3fba6abba`.

Two normalized-payload-reproducible repaired wheel archives have SHA-256
values
`615eaa8e7106b726d966ab9b2340cdcffb16e9a7bfb8ca381750377fa9203ca7`
and `e964e43fba00fead4e473fc7bbf4aa474adc9da808a4cdf2375af6c23e93a745`;
their byte-identical native extension SHA-256 is
`381914acb56091eafcc140cb85644900e1d1897b60954e517fd3008fb2d52e02`.
Repair copies the required OpenSSL dylibs into each wheel. External `_file`
content, ZIP/update HAR, regex/predicate filters, ordinary Context routing/URL
filter composition and `unroute_all` remain explicit gaps.

## ABI 79 Page HAR URL-filter composition checkpoint (2026-10-04)

NODE-325 makes no architecture or ABI change. It fixes the complete unchanged
async/sync `test_should_only_handle_requests_matching_url_filter_no_fallback_page`
pair in `page-har-url-filter-manifest.json`, SHA-256
`ad9293c598db398137c56ccb9405311db2ef4fff9bfe13cecd61a260f58067fe`,
and executes both public APIs against the installed NODE-323 wheel. The test
registers an ordinary Page route after the HAR route: matching traffic is
fulfilled from the HAR, while nonmatching traffic falls through to that next
route without a Python HAR callback. Candidate and oracle pass 2/2;
API-separated candidate stress lanes pass async 10/10 and sync 10/10.

Candidate, oracle, async-stress and sync-stress report SHA-256 values are
respectively
`eca2cd0c052686b795e94d5dbdd59a17468e0b952a33dd073a39745ffeba1e94`,
`807bb7adf649c77448c0963b6f4542c22a674371b341160beffb68a1a0dc23f0`,
`f460365ce417c1e205cdb6435708fee906c4d2fb9f2afcbb6808d92d64840474`
and `7ee65e04701576ef9b4d90058bcf8fadb364ec4f46a849e75b06684963b43c19`.
The ledger advances to 771 passed, zero failed, seven skipped and 5,748 not
run, SHA-256
`fb11fa9a34ed364ec06254672ac32ef54a7a790362eeaa4f6a562887c2d1f03e`.

## ABI 79 async and sync Page HAR evidence checkpoint (2026-10-04)

NODE-324 makes no architecture or ABI change. It fixes the complete unchanged
async/sync Page HAR method/redirect pair in
`page-har-replay-manifest.json`, SHA-256
`51e666d86132e4d88790025a98dff4ddb114eec22eb3df71e51bf2a23afd7f33`,
and executes both public APIs against the installed NODE-323 wheel. Candidate
and oracle pass 2/2; API-separated candidate stress lanes pass async 10/10 and
sync 10/10.

The mixed async/sync repeated diagnostic is not a product result and is not a
ledger input: pytest-repeat runs the synchronous bridge between repetitions,
then pytest-asyncio tries to call its session `Runner.run()` from that running
loop. Candidate/oracle final reports remain clean and the isolated stress lanes
exercise the same unchanged tests repeatedly.

Candidate, oracle, async-stress and sync-stress report SHA-256 values are
respectively
`2340bfc952cf7c2ec843e2e6166372a32d531e46587f8c080b04aa1235ec9958`,
`950219a444ee63edccce9d63cc8fade9323dce703b9c05bf97b43c6162437654`,
`17bea22d89ca4dc5cb862ba3820b8ad39dd42aa5bcb43684d6ebcc4b6f86f964`
and `ad8c1a66288ec6d2cde1dcdc7ba7e42cac4d5f5c9983ee48d895fa90405004ec`.
The ledger advances to 769 passed, zero failed, seven skipped and 5,750 not
run, SHA-256
`dbc28e3a20089d56d2b937d7175f19e4562db76da59477b789e8c14fc375cefd`.

## ABI 79 persistent-Context plain-HAR replay checkpoint (2026-10-04)

NODE-323 adds an independently selectable `pwright-har-replay` policy crate
over the existing Page-route host feature. The crate contains no filesystem,
CDP, runtime or language-binding dependency. It parses bounded plain HAR,
matches exact URL/method and optional POST data, chooses the first entry with
the greatest stable request-header overlap, follows bounded redirects and
returns an embedded plain/base64 response action. The host owns bounded file
reading and resolves internal HAR actions directly through its existing Fetch
event driver; Python forwards only path, optional string URL filter and the
`abort`/`fallback` not-found choice.

The selected package allows at most eight archives, 8 MiB per input, 4,096
entries, 8 MiB retained archive bytes, 256 request/response headers per message,
256 KiB POST bodies, 1 MiB response bodies and 20 redirects. External `_file`
content is rejected with an explicit separate-content-source diagnostic. ZIP
HAR, update recording, Context routing, regex/predicate URL filters and
`unroute_all` are not claimed by this slice.

The same unchanged upstream row requires
`expect(locator).to_have_css()`. Python forwards bounded property/value/regex
inputs to a Rust-selected `getComputedStyle(...).getPropertyValue(...)`
program; Rust owns regex validation, retry, deadline and missing-element
semantics. No browser logic or polling loop is copied into Python.

The one-row manifest SHA-256 is
`a3e57ce38826a608a730a6e382c60e65967cc4b81891aa6cf5118cc0790adf08`.
Final installed candidate/oracle pass 1/1 and candidate stress passes 10/10.
Installed-wheel units pass 167/167, the all-feature host library passes 1,001
tests with 26 explicit ignores, the HAR crate passes 4/4 and the addon passes
2/2. Strict Clippy, Ruff, Pyright and the dependency proof pass. Fresh
collection remains 6,526/6,526 and the ledger advances to 767/0/7/5752,
SHA-256
`e390e350e8995a48569133ed5be757cd9040a6d1592ab1b86f0ad5ca046476e2`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`74bc73ec033aa2bdd914f94d752846615ea2a4b0801f8e8c9e3ef390fcdafc31`,
`b48a9c6aa3325a2663fe987ae118058586b0f917c27babf9ba0cc2609893cc02`,
`300abda8e41433e285970a456c32acd412688f194af3d57e8e77abda36861f5d`,
`3c33650324d96ed74eaf5562bf795c15fbc5c6bad9e3117d817b43b9806277df`
and `a3619b07314bb55898cbd6110bb25eb4af3b746c95216f36fe1371dfc9059799`.
The two normalized-payload-reproducible wheel SHA-256 values are
`c1954548f523af59abc861656595738e8b6ae82c8b02b8f2deaa4e2de6616473`
and `d0bcdc98b69ddaf9fdcf2d5df482e32fcd2801b9ec43b28f8aa314dae72ebde8`;
their byte-identical native extension SHA-256 is
`243f6a6771e86e4d8bce4b7bd96293ea37c558fd2ef79344dfd76b683b3cd2c3`.

## Goal

Ship a typed `pwright` Python package whose public shape follows Playwright
Python while browser behavior, protocol I/O, ownership and resource accounting
remain in Rust. The package must let users install the smallest capability set
they need instead of pulling a Node driver, bundled browser, or a monolithic
Python implementation into every process.

The first compatibility oracle is Playwright Python `1.58.0`. PyPI did not
publish a `1.58.2`; the repository's existing Node differential corpus remains
pinned to the separate npm `playwright@1.58.2` release. Chromium CDP attach is
the first transport. ABI 22 also selects an installed-Chromium persistent
launch leaf, ABI 23 selects a headless Page-pause timeout guard, ABI 24 selects
explicit Page garbage collection, and ABI 25 adds bounded Rust-owned Page-error
history. ABI 26 adds separately selectable bounded Rust-owned console-message
history, ABI 27 adds bounded Rust-owned request history plus retained
response/body ownership, ABI 28 adds the full immutable device catalog plus
bounded Context-scoped device emulation, and ABI 29 adds bounded Rust-owned
Page response publication with thin async and sync listener adaptation. ABI 30
adds the native collection-selector evaluation export and Playwright
`internal:and` intersection parsing over the existing Rust selector/runtime
closure. ABI 31 adds independently selectable bounded Rust-owned global
selector registration with thin async and sync facades. ABI 32 exports native
selector validation so Page query errors stay Rust-classified. ABI 33 executes
bounded registered-engine selector plans in Rust-owned main or utility worlds
and optionally selects a bounded layout-selector parser/compiler and browser-
world geometry scorer without changing the native ABI. NODE-254 adds thin
basic Locator facades over existing Rust owners and typed Rust `:text()`
parsing, also without changing the ABI. NODE-255 advances to ABI 34 for one
native Locator count-expectation export whose retry/deadline owner remains in
Rust; Python and Node expose only thin assertion adapters. NODE-256 keeps ABI
34 and adds immutable Python Locator description metadata without browser or
native state. NODE-257 advances to ABI 35 for an immutable native Locator
owner: root query admission, typed within-composition and nth derivation remain
in bounded Rust state, while Python retains only the facade reference and
optional description metadata.
Firefox,
WebKit, generic nonpersistent launch, headed Inspector pause, and
Playwright-protocol connect are explicit future providers, not fake aliases
for CDP.

NODE-258 executes the complete 163-row Chromium Locator corpus before further
surface selection. NODE-259 reuses ABI 35's immutable native Locator owner for
bounded all/nth derivation and additional actions; NODE-260 prevents borrowed
default/external Context network streams from being eagerly captured by an
attached session. NODE-261 through NODE-270 raise the stable candidate to
145/163 while the official oracle remains 163/163. Borrowed pre-existing
Context response/history activation must become demand-driven before that
behavior is compatible again. NODE-271 advances to ABI 44 with a lazy
Rust-owned FrameLocator recipe and the official Python public-handler arity
adapter, bringing the complete Locator candidate to 163/163. This closes that
tranche only. NODE-272 advances to ABI 45 with typed semantic-Locator leaves
and Rust-owned attribute-expectation polling; the complete unchanged sync
semantic-Locator file passes 10/10 in candidate and oracle. Neither checkpoint
changes the still-unexecuted Python or Core corpus into a compatibility claim.
NODE-273 advances to ABI 46 so an immutable native Locator can execute
`evaluate_all` through the existing bounded Rust collection-evaluation owner;
the complete async semantic-selector file passes 3/3 in candidate and oracle.
NODE-274 keeps ABI 46: existing Rust-owned fill passes both complete fill files
5/5, and thin Page/BrowserContext/Browser context-manager protocols over the
existing close owners pass both complete context-manager files 4/4. Candidate
and independent oracle match across all nine rows.
NODE-275 advances to ABI 47 with an independently selectable, bounded
Rust-owned HTTPS-error Context policy. The two Chromium rows from the complete
unchanged async file pass candidate and oracle; the four Firefox/WebKit
expansions remain explicit provider gaps.
NODE-276 advances to ABI 48 with an independently selectable, bounded
Rust-owned PDF capture/output owner. Both applicable Chromium rows from the
complete unchanged sync file pass candidate and oracle; the four non-Chromium
inventory expansions remain explicit upstream Chromium-only non-applicable
rows.
NODE-277 advances to ABI 49 by adding typed tagged/outline booleans to that
same Rust owner. All three applicable Chromium rows from the complete unchanged
async PDF file pass candidate and oracle; its six non-Chromium expansions
remain explicit upstream Chromium-only non-applicable rows.
NODE-278 advances to ABI 50 with bounded Rust-owned Page, Locator and exact
retained ElementHandle screenshot paths. All three Chromium rows from the
complete unchanged async screenshot file pass candidate and oracle; its six
Firefox/WebKit expansions remain explicit provider gaps because the source has
no Chromium-only marker.
NODE-279 keeps ABI 50 and adds typed public legacy-text/regex selectors,
quote-aware selector-chain boundaries, bounded Page selector handle
publication, non-strict Page selector evaluation and thin ID assertions. Both
Chromium rows from the complete unchanged async legacy-text file pass candidate
and oracle; its four Firefox/WebKit expansions remain explicit provider gaps.
NODE-280 keeps ABI 50 and adds bounded Playwright-scoped Rust ownership for
mutable test-id configuration plus finite strict-error DOM/test-id previews.
The complete unchanged async selector-generator file passes its Chromium row
in candidate and oracle; Firefox/WebKit remain explicit provider gaps.
NODE-281 keeps ABI 50 and verifies the existing sync lifecycle from an
upstream-created worker thread. The complete unchanged shared thread file
passes its Chromium row in candidate and oracle without product changes;
Firefox/WebKit remain explicit provider gaps.
NODE-282 keeps ABI 50 and adds a one-error-bounded sync bridge slot. A Page
listener exception is rethrown after the public call finishes instead of being
log-only. The complete unchanged shared event file passes its Chromium row in
candidate and oracle; Firefox/WebKit remain explicit provider gaps.
NODE-283 advances to ABI 51 and exports the existing independently selectable
bounded Rust timeout wait through thin async/sync Page facades. The complete
unchanged shared signal file passes both Chromium rows in candidate and oracle
through the same dedicated CDP Chrome; Firefox/WebKit remain four explicit
provider gaps.
NODE-284 advances to ABI 52 with an independently selectable bounded
Rust-owned terminal Response completion owner. Both complete unchanged async
and sync Page network-response files pass all four Chromium rows in candidate
and oracle; eight Firefox/WebKit expansions remain explicit provider gaps.
NODE-285 advances to ABI 53 with independently selectable Rust-owned Context
request waits, exact shared request-history ownership, post-data JSON and
same-CDP-cursor request Frame availability. The complete unchanged async Page
network-request file passes both Chromium rows in candidate and oracle; four
Firefox/WebKit expansions remain explicit provider gaps.
NODE-286 advances to ABI 54 with the independently selectable
`file-chooser-events` composition. The complete unchanged sync input file
passes both Chromium rows in the final repaired-wheel candidate and oracle;
four Firefox/WebKit expansions remain explicit provider gaps. Rust owns exact-
session interception, event routing, publication, backend-node identity and
bounded payload/path mutation. Python owns only public wrappers, waiters and
callback identity.
NODE-287 advances to ABI 55 with thin async/sync `Page.wait_for_selector`
facades over the existing Rust Locator wait and Handle publication owners. Two
applicable Chromium rows from unchanged async `test_issues.py` pass in final
installed candidate and independent oracle, including cancellation of a
pending selector wait. Python performs no polling or DOM retention. Generic
launch options and an environment-sensitive service-worker diagnostic remain
explicit gaps outside the green ledger.
NODE-288 keeps ABI 55 and verifies three more selector-wait rows spanning
async cancellation, timeout exception shape and sync execution. The native
timeout classification was already correct; Python adds Playwright-shaped
`.name` access to the existing exception classes without changing protocol or
resource ownership. Candidate and independent oracle pass 3/3.
NODE-289 keeps ABI 55 and completes the applicable Chromium rows in unchanged
async `test_asyncio.py`. The TaskGroup row already passes. For the remaining
discrepancy, Rust still classifies `PWRIGHT_JAVASCRIPT`; a thin Python public
boundary rebuilds the same exception type with the `Page.evaluate:` API name
and preserves code, name and stack. Candidate and independent oracle pass the
two newly selected rows 2/2. Firefox/WebKit expansions remain provider gaps.
NODE-290 keeps ABI 55 and records all 25 previously unexecuted Chromium rows
in unchanged sync `test_sync.py`. Four exact repr facades remain pure public
formatting. `Response.request` is fed by the existing Rust
`RequestHistoryOwner` across both network waits and response notices, so
Python never correlates request IDs. Candidate repairs five rows but remains
red at 14 passed / 11 failed for the tranche.
NODE-291 advances to ABI 56 with a separately selectable
`element-handle-dom` composition. Rust compiles a typed selector relative to
the retained element, publishes or releases the first result and reads
`HTMLElement.innerText`; Python only wraps the native methods. The two
official sync rows pass and the tranche advances to 16 passed / nine failed.

## Non-goals

- Reimplement protocol or browser semantics in Python.
- Load Node.js, the Playwright Node driver, or a bundled Chromium.
- Claim full Playwright compatibility from import/signature compatibility.
- Make an unbounded thread, task, callback, object-cache, or byte queue.
- enable optional Rust capabilities in the base wheel by accident.
- make wrapper finalizers the only cleanup mechanism.

## Boundary

```text
user code
  -> pwright.async_api / pwright.sync_api       Python adaptation only
  -> pwright._native                            PyO3 ABI boundary
  -> pwright-node-host                          reusable Rust session host
  -> selected Rust capability/plugin closure   browser semantics
  -> transport/provider                         Chromium CDP first
```

Python owns only:

- Playwright-shaped names, positional/keyword validation, type hints and
  Python exceptions;
- identity-preserving facade caches over Rust-issued IDs;
- asyncio awaitable adaptation and the bounded sync event-loop thread;
- conversion of Python scalar/container values into bounded Rust values;
- user callback invocation after Rust has admitted and queued an event.

Rust owns:

- endpoint validation and discovery, CDP attachment and command execution;
- Context/Page/Frame identity, topology, lifecycle and native cleanup;
- waiting, timeout classification, selector/action/evaluation semantics;
- capability selection and exact Cargo dependency closure;
- admission before foreign-value copies, pending-result ownership, byte
  reservations, cancellation, close barriers and resource metrics.

No `Py<PyAny>`, coroutine, callback, Python exception, or GIL-bound value may be
stored in `pwright-node-host`. The PyO3 crate may retain Python objects only in
an explicitly bounded delivery registry and must drop them on the Python
runtime side.

## Repository layout

```text
crates/pwright-python-addon/
  Cargo.toml                 cdylib, feature forwarding, no default capability
  src/lib.rs                 ABI, error mapping, native environment/session

packages/pwright-python/
  pyproject.toml             pinned PEP 517 build requirements
  README.md                  supported surface and reproducible commands
  python/pwright/
    __init__.py
    _abi.py                  native ABI validation
    _errors.py               public exception taxonomy
    async_api.py             async Playwright-shaped facade
    sync_api.py              sync adapter over one bounded loop thread
    py.typed
  tests/
    unit/                    no-browser facade/ABI/error tests
    differential/            normalized oracle/candidate cases
    smoke/                   wheel-load and child-process teardown tests
```

The distribution name is `shuozeli-pwright`; the import name is `pwright`.
This avoids shadowing the upstream `playwright` distribution while allowing
mechanical migration from `playwright.async_api` to `pwright.async_api` and
from `playwright.sync_api` to `pwright.sync_api`.

Maturin uses the mixed Rust/Python layout with
`module-name = "pwright._native"` and an explicit Cargo manifest path. The
native module targets PyO3 `abi3-py39`, so one platform wheel supports CPython
3.9 and newer. Build-system and differential dependencies are exact pins.

## ABI contract

The current native ABI is integer `94`. The selected wheel includes the
optional URL-wait, set-content, media-emulation, timeout-settings,
network-events, Handle, wait-for-function, init-script, Page-binding and
Page-lifecycle-event, Page-dialog, Page-error, Page-route, drag-and-drop and
device-emulation, Page-viewport, Page-PDF, Locator-bounding-box and
Locator-screenshot
leaves plus selector registration
leaves over the original base object/lifecycle/evaluation slice. ABI 18 extends
the network-events selection with bounded `Request.sizes()` ownership; ABI 19
adds Rust-owned drag; ABI 20 adds ordered dialog ownership and beforeunload;
ABI 21 adds bounded route fulfillment and Rust-owned fulfilled-response bodies.
ABI 22 adds separately selectable Rust-owned persistent Chromium launch,
strict profile initialization and bounded owned-process shutdown.
ABI 23 adds separately selectable Rust-owned headless Page pause with scoped
timeout reset/restoration and typed internal-text selector parsing.
ABI 24 adds separately selectable Rust-owned Page garbage collection over the
existing CDP session owner.
ABI 25 extends the separately selectable Page-error leaf with bounded,
resource-accounted per-Page history and a native snapshot read.
ABI 26 adds a separately selectable console-history leaf with bounded,
resource-accounted per-Page snapshots; it reuses the console parser without
selecting console callback publication.
ABI 27 adds a separately selectable request-history leaf over network-events,
with last-100 snapshots, retained evicted owners, response/body settlement and
an independently bounded FFI observation lane.
ABI 28 adds a separately selectable device-emulation leaf with the immutable
143-entry upstream catalog, bounded Context-owned settings and application in
the existing pre-publication Chromium session-initialization fence.
ABI 29 adds a separately selectable response-events leaf with Context-scoped
interest, a 256-notice FIFO, shared 16 MiB network bytes and explicit
subscription coexistence with request-history.
ABI 30 changes no capability selection. It adds the exported
`eval_on_selector_all` method, thin async/sync Page facades and typed
`internal:and` parsing over the already selected locator-evaluation path. The
ABI increments because the native exported method surface changed.
ABI 31 adds the separately selectable `selector-registration` host leaf and
the exported `register_selector_engine` method. Rust owns registry admission,
source retention, duplicate/name errors and teardown; Python owns only bounded
source/path normalization and async/sync call adaptation.
ABI 32 adds no capability selection; it exports `validate_selector` over the
existing bounded `LocatorQuery` path so `Page.query_selector` surfaces parser
issues before its browser operation. The ABI increments because the native
method surface changed.
ABI 33 initially added no capability edge. It exports browser-program match
counting under the ABI-stable internal method name
`registered_selector_match_count` over the selected registry and locator-
evaluation paths. Rust compiles and executes bounded immutable programs, while
Python uses the result only to return `None` or report the explicit positive-
handle gap. NODE-253 adds the separately selectable `layout-selectors` leaf
behind that same export, so no further ABI increment is needed. The ABI
increment from 32 to 33 was caused by the original native exported method.
ABI 34 adds no capability selection or dependency edge. It exports
`locator_expect_count` over `HostLocator`; Rust owns typed count reads, the
expected/negated comparison, total deadline, navigation/context-replacement
retry and bounded sleep. Python forwards selector, expected count, timeout and
negation and retains no polling state. The ABI increments because the native
exported method surface changed.
ABI 42 adds the independently selectable `page-viewport` leaf and exported
`set_viewport_size` method. Rust owns the 64-Page / 8 KiB registry, CDP window
and metrics overrides, exact teardown and session-initialization replay; Python
only validates two finite integer dimensions.
ABI 43 exports `Locator.bounding_box` over the existing Rust geometry owner and
selects the independently installable `locator-screenshot` leaf. Rust owns
strict selector resolution, stability, scrolling, iframe clip translation,
capture admission, two concurrent captures and the 2 MiB output ceiling.
Python converts bounded geometry and bytes and retains no capture state.
ABI 44 exports the immutable Rust-owned FrameLocator recipe and native methods
for owner/content-frame derivation, nested frame composition, typed Locator
re-rooting and operation-time frame resolution. Python owns only wrapper
identity and its language-specific callback-arity adapter; it does not retain
DOM identities, parse selector chains, serialize browser events or run frame
retry loops.
ABI 45 makes typed text, label, placeholder, alt-text, title, test-id and role
leaves available consistently from Page, Frame, Locator and FrameLocator. It
also exports attribute expectations whose total deadline, retry, missing-node
behavior and exact/regular-expression comparison are Rust-owned. Python passes
one bounded match-options value and adapts the result; it contains no selector
grammar or polling loop.
ABI 46 exports collection evaluation directly on an immutable native Locator.
The typed recipe, selector compilation, exact collection resolution,
utility-world execution, FFI admission and bounded result conversion all stay
in Rust; Python forwards only the public function and serializable argument.
ABI 47 exports the independently selectable ignore-HTTPS-errors Context policy.
ABI 48 exports the independently selectable bounded Page-PDF stream owner.
ABI 49 extends that native method with typed tagged/outline booleans; Rust
constructs the Chromium request while Python only validates and forwards the
public option values.
ABI 50 adds bounded Rust-owned Page, Locator and exact retained ElementHandle
screenshot exports. ABI 51 exports the independent bounded Page timeout wait;
ABI 52 adds bounded terminal Response completion; ABI 53 adds Context request
wait/history, post-data JSON and request Frame availability; ABI 54 adds exact-
session file-chooser publication; ABI 55 adds Page selector waits and their
Handle publication. ABI 56 adds retained ElementHandle DOM operations, ABI 57
adds bounded Page console publication, ABI 58 adds retained Page-console
argument Handles/subscriptions, ABI 59 adds Page request publication, ABI 60
adds download publication and ABI 61 adds dedicated-Worker ownership and
evaluation. ABI 62 adds exact-Worker console publication and its native notice
read. ABI 63 adds exact-Worker Handle evaluation and Handle property dispatch
without routing Worker values through a Frame. ABI 64 atomically converts an
ordered Worker-console RemoteObject batch into retained Handles and dispatches
their JSON conversion in the exact Worker world. ABI 65 adds bounded
Context-owned Page/OOPIF locale initialization without introducing a
language-side policy owner; Workers inherit renderer locale. ABI 66 composes
that owner with persistent launch, admits the default locale before initial
topology and sends Chromium's paired `acceptLanguage` plus ICU overrides. Each
added export advances the exact package/native handshake; Python-only fixes
between these checkpoints keep the previous ABI.
Every transition remains an exact package/native handshake; an older wheel is
rejected at import instead of being used through a partially compatible ABI.

- `_native.native_abi_version()` returns the loaded extension ABI.
- Python expects one exact ABI and fails import with a diagnostic containing
  both expected and observed values.
- The ABI smoke test imports from an installed wheel in a clean virtual
  environment and in a child process whose current directory is outside the
  source tree.
- Native capability metadata is an immutable value: ABI, package version,
  selected features, and resource limits. Python never infers capabilities by
  catching arbitrary operation errors.

## Public compatibility slices

### Slice PY-001: load and ownership

- `from pwright.async_api import async_playwright`
- `from pwright.sync_api import sync_playwright`
- async and sync context-manager/start/stop shapes
- `Playwright.chromium`; `firefox` and `webkit` report a typed unsupported
  error on operations rather than masquerading as Chromium
- clean environment shutdown with zero native/FFI metrics

### Slice PY-002: Chromium CDP attachment

- `chromium.connect_over_cdp(endpoint_url, timeout=30000)`
- `Browser.version`, `Browser.is_connected()`, `Browser.contexts`,
  `Browser.close()`
- default attached Context and current Page enumeration
- endpoint URL and timeout validation before an async task is spawned

Unsupported Playwright `connect_over_cdp` arguments are rejected by name with
`NotImplementedError`; they are not silently ignored. The initial list is
`headers`, `slow_mo`, and `is_local` until native support exists.

### Slice PY-003: core object graph

- `Browser.new_context()` and `BrowserContext.new_page()`
- `Browser.new_page()` with an implicit Rust-owned Context that closes with its
  Page, including failure compensation
- `BrowserContext.pages`, `new_page()`, `close()`
- `Page.context`, `is_closed()` and `close()`
- stable wrapper identity for the same Rust object ID
- close cascade removes objects only after native confirmation

`Page.main_frame` and `Page.frames` remain gated on the Frame facade rather
than returning incomplete placeholders.

### Slice PY-003P: installed-Chromium persistent Context

- `chromium.launch_persistent_context(user_data_dir, ...)` returns the exact
  default Context created by the launched browser.
- Rust accepts an empty caller-owned directory or a profile carrying Pwright's
  marker; it never adopts nonempty unmarked state.
- Rust selects the explicit executable/channel or an already-installed Chrome,
  launches on an ephemeral loopback CDP port, owns the process lease and
  performs bounded `Browser.close`, graceful wait and forced cleanup.
- The installed slice accepts only `headless`, `base_url`, `executable_path`,
  `channel`, and `timeout`; every other Playwright option is rejected by name.
- No browser binary, Node driver or browser downloader is part of the wheel.

This slice proves one pinned Chromium row. It does not claim the remaining 21
collected persistent-Context invocations, Firefox/WebKit, extensions,
downloads, video, proxy, permissions or the complete persistent API surface.

### Slice PY-003Q: headless Page pause

- `Page.pause()` forwards only Page identity to the optional Rust
  `page-pause` leaf.
- Rust rejects headed sessions until an Inspector capability is installed.
- For headless sessions, Rust atomically snapshots Page/Context operation and
  navigation timeouts, sets all four effective values to zero, yields, and
  restores the exact previous values on completion, cancellation, or drop.
- The official post-pause locator assertion uses typed `internal:text` parsing;
  Python only serializes the public `get_by_text` string/exact option.

This slice proves two pinned Chromium rows. It does not claim headed Inspector
UI, generic browser launch, Firefox/WebKit pause, or the complete Page surface.

## Official Python corpus lane

The package-level paired cases are complemented by exact pytest node IDs from
the pinned `microsoft/playwright-python` source. Source identity, public versus
upstream-private import inventory, candidate namespace adaptation and the
initial 7/7 oracle/candidate checkpoint are specified in
[Official Playwright Python conformance](python-official-conformance.md).

An official-source pass means that the unmodified selected test function ran;
it does not promote unselected functions from the same module. Candidate-only
expectation changes and blanket skips are forbidden.

### Slice PY-004: page essentials

- `Page.goto`, `reload`, `title`, `content`, `evaluate`, `screenshot`
- `Page.locator` and the already-supported locator query/action subset
- keyboard and mouse facades over existing native capability leaves
- `Playwright.selectors.register` registration and errors; browser execution
  of registered engines remains explicitly unsupported

Later slices add events, handles, file input, navigation variants, headers and
other optional leaves only after their dependency and cleanup proofs pass.

## Async and sync execution

The async API is primary. PyO3 converts a Rust future to the caller's asyncio
loop. The future owns an `FfiExecution` admitted before endpoint strings or
argument payloads are copied. Completion is converted under the GIL, then the
ticket and byte reservation are released.

Cancellation has two separately tested outcomes:

1. cancelling a Python awaiter does not orphan Rust work or its reservation;
2. environment/session close cancels and joins native work without needing a
   Python callback to run.

The sync API owns exactly one lazily started event-loop thread per started
`sync_playwright()` object. Calls are submitted with a bounded in-flight limit.
`stop()` closes admission, cancels or drains submitted work according to the
operation contract, stops the loop and joins the thread. Calling sync API from
that loop thread is rejected instead of deadlocking.

## Ownership and identity

```text
Playwright owner
  owns Environment admission + delivery registries
    owns weak Session records and their native retirement signals
      Browser facade owns Arc<HostSession>
        Context/Page/Frame facades own IDs plus their Browser facade
```

Explicit close is authoritative. `__del__` is a best-effort signal only and
must not block, await, call arbitrary Python, or be required for correctness.
Environment stop performs this barrier:

1. seal new Python and FFI admission;
2. cancel environment work;
3. join Rust FFI execution;
4. request every published session to stop;
5. await each native driver;
6. discard publication and facade caches;
7. assert FFI and session runtime metrics are zero;
8. stop/join the sync thread, if present.

Borrowed CDP attachment never closes the browser process. `Browser.close()`
means detach this Pwright session; owned browser-process shutdown will require
a separate launch provider and ownership token.

## Bounded resources

The Python environment starts with the same conservative limits as the Node
environment, reported through capability metadata rather than hidden:

| Resource | Limit |
|---|---:|
| sessions including pending connects | 16 |
| pending connects | 16 |
| pending commands | 32 |
| pending closes | 16 |
| pending watches | 16 |
| registered selector engines | 64 |
| selector engine name | 64 bytes |
| selector engine source | 1 MiB |
| aggregate selector registry | 8 MiB |
| aggregate FFI state | 64 MiB |
| connect future body | 1 MiB |
| command future body | 128 KiB |
| close/watch future body | 16 KiB |

Every input has an additional semantic bound before conversion: endpoint and
ID length, collection cardinality, serialized value bytes, screenshot bytes,
and event/callback queue depth. An over-limit operation returns
`ResourceExhaustedError`; it does not wait on an unbounded queue.

## Error mapping

Rust is the source of classification. The addon maps stable `HostError::code()`
values directly to Python classes and attaches `.code`:

| Rust code | Python class |
|---|---|
| `PWRIGHT_INVALID_ARGUMENT` | `InvalidArgumentError` (`Error`); `ValueError` at Python signature boundary |
| `PWRIGHT_RESOURCE_EXHAUSTED` | `ResourceExhaustedError` |
| `PWRIGHT_TIMEOUT` | `TimeoutError` |
| `PWRIGHT_TARGET_CLOSED` | `TargetClosedError` |
| `PWRIGHT_UNSUPPORTED` | `NotImplementedError` |
| `PWRIGHT_JAVASCRIPT` | `Error` with bounded JS name/stack metadata |
| `PWRIGHT_BACKEND` | `Error` |

Python must never recover a class by parsing an error message. Diagnostics and
JS stacks remain byte-bounded before crossing the FFI.

## Feature composition

`pwright-python-addon` has `default = []`. Each Cargo feature forwards to the
same host leaf and has an exact dependency allowlist test:

- `handles`
- `file-input`
- `extra-headers`
- `concurrent-close`
- `frame-lifecycle`
- `frame-navigation`
- `selector-registration`
- `drag-source`
- `drag-runtime`

Release wheel profiles are explicit (`base`, then reviewed additive profiles).
A wheel name/metadata record must identify its profile. Python extras alone do
not select compiled Cargo features; separate published artifacts or a local
source build selects the native profile reproducibly.

The current `frame-navigation` closure is a known non-green repository input
and is excluded from the first base wheel. It cannot be declared supported
until its isolated feature build is repaired and green.

## Verification gates

All tests use Arrange-Act-Assert sections. A slice is complete only after the
relevant rows pass:

| Gate | Required evidence |
|---|---|
| format/lint | `cargo fmt --check`, Ruff format/check, Pyright strict package surface |
| base Rust | release `cargo check` and strict `cargo clippy` for `pwright-python-addon`; exact shared-host unit gates for Rust behavior |
| feature isolation | one release check per selected feature and exact dependency closure scripts |
| reproducible wheel | exact build pins, locked Rust graph, wheel built twice from clean trees with explained metadata-only differences or matching payload hashes |
| load/ABI | clean-venv wheel install; source tree absent; child-process import; exact ABI/profile assertions |
| Python unit | async and sync shapes, argument/error mapping, identity cache, cancellation and double-close |
| differential | same normalized cases against `playwright==1.58.0` and local wheel, no candidate-specific expectation weakening |
| real browser | dedicated Chrome CDP profile; attach, enumerate, navigate/read/action slice, detach without browser-process close |
| cleanup | repeated attach/close, cancelled await, failed connect, worker/thread/process exit; zero FFI/session metrics and joined drivers |
| memory | bounded repeated-case RSS/physical-footprint trend recorded separately from shared browser memory |
| docs | this design, package README, MANIFEST and project index agree with actual green surface |

Real-browser runs follow the repository rule: start and verify `chrome-cdp`,
use `http://127.0.0.1:9222` only for the loopback CDP control endpoint, clear
unrelated test tabs, and isolate each run's `.pwright` state.

## Differential case format

The Python runner records only stable, normalized observations:

```json
{
  "case": "attach/enumerate/default-context",
  "oracle": "playwright==1.58.0",
  "candidate": "shuozeli-pwright local wheel",
  "oracle_result": {"status": "ok", "value": {}},
  "candidate_result": {"status": "ok", "value": {}},
  "classification": "match"
}
```

Classifications are `match`, `candidate_mismatch`, `candidate_unsupported`,
`oracle_error`, or `harness_error`. Unsupported is visible and fails a slice
that claims support. Timings, generated target IDs and browser patch versions
are removed only by shared normalization.

## Ordered implementation tasks

1. `PY-000` freeze this design, oracle pin and completion gates.
2. `PY-001` create the no-default-feature PyO3 crate and mixed Python package.
3. `PY-002` implement ABI/profile metadata, stable exceptions, environment
   ownership, async start/stop and load/cleanup tests.
4. `PY-003` implement bounded Chromium `connect_over_cdp` and Browser facade.
5. `PY-004` publish Context/Page snapshots with stable Python identity.
6. `PY-005` add Context/Page create/close and close-cascade tests.
7. `PY-006` add the first page command vertical slice and differential corpus.
8. `PY-007` add sync facade and thread teardown tests.
9. `PY-008` add base wheel reproducibility, clean-venv ABI and process-smoke CI.
10. `PY-009` run dedicated real-Chrome behavior/cleanup/memory gates and record
    exact evidence.

## Completion rule

The Python FFI goal is not complete when the extension merely compiles, when a
source-tree import works, or when signatures resemble Playwright. Completion
requires a wheel-installed async and sync supported slice, pinned differential
agreement, explicit unsupported results for everything outside that slice,
isolated feature/dependency proof, and zero-resource cleanup evidence under
success, failure, cancellation and interpreter/process teardown.

## Base-slice verification checkpoint (2026-09-22)

Implemented files are `crates/pwright-python-addon` and
`packages/pwright-python`. The selected artifact is profile `base`, currently
native ABI `3`, CPython ABI `abi3-py39`, with no optional capability feature
selected.
The implemented public slice is async/sync lifecycle, Chromium CDP attachment,
Browser/Context/Page enumeration, Context/Page creation, Page
`goto`/`title`/`content`/`evaluate`, and explicit Page/Context/Browser close.
ABI 2 added `Browser.new_page` with Rust-owned implicit-Context cleanup and
native base-URL construction for Page navigation. ABI 3 adds bounded Python
value conversion over the existing Rust evaluation capability. APIs outside
the recorded slice remain explicit typed unsupported results.

Green gates on macOS arm64, Python 3.12.14 and dedicated Chrome
153.0.8010.53:

- release `cargo check` and strict `cargo clippy` for
  `pwright-python-addon`;
- exact `FfiBudget::run_sync` release unit gate with `--lib`;
- exact base dependency proof: 108 Pwright packages, with file-input and Frame
  lifecycle leaves absent;
- Ruff format/lint and Pyright strict: zero errors or warnings;
- two release wheels have identical native/Python/metadata payloads. Maturin's
  CycloneDX `serialNumber` and `metadata.timestamp`, plus their RECORD hash,
  are the only normalized build-identity differences;
- clean `/tmp` venv install from the wheel and child-process import resolved
  `pwright/_native.abi3.so`, ABI `3`, profile `base`;
- five wheel-installed unit cases pass;
- the same attach/enumerate/goto/title/content/detach observations match upstream
  `playwright==1.58.0` through both async and sync APIs, including stable
  BrowserType/Browser/Context/Page object identity;
- success, failed connect, cancelled awaiter, repeated detach and owner-stop
  cleanup report zero FFI/session resources;
- a controlled two-tab check retained both Chrome targets after candidate
  `Browser.close()`, proving that CDP detach does not close the shared browser
  pages;
- after warmup, 20 attach/enumerate/detach iterations grew the Python process
  maximum RSS by 1,310,720 bytes (baseline 78,741,504 bytes), below the 64 MiB
  regression ceiling. Browser memory is deliberately excluded.

The Python oracle's signed bundled Node executable encountered a first-launch
macOS provenance delay in the disposable test venv; after that one-time launch,
the differential itself completed normally. No Node driver is loaded by the
candidate package.

## Official-corpus checkpoint (2026-09-23)

The Python FFI now exposes the existing Rust-owned `HostSession::new_context`
and `HostSession::new_page` operations. An isolated official-source lane runs
seven exact upstream navigation/Page tests from commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`, covering HTTP, `about:`, `data:`,
URL/title and 200/404 Response semantics. Oracle and candidate both pass 7/7
without editing the upstream assertions. Context/Page child-first cleanup
returns both native session and FFI metrics to zero.

## ABI 6 full-corpus expansion checkpoint (2026-09-26)

ABI 6 keeps Python as a value/lifecycle adapter while exposing more existing
Rust-owned services: locator actions and reads, strict selector evaluation,
same-document URL publication, and published Frame-tree enumeration/lookup.
The optional wheel profile remains `wait-for-url+set-content`; locator and Frame
metadata operations reuse the base host composition rather than adding a
Python browser implementation.

The unchanged upstream `tests/async/test_page.py[chromium]` shard now records
47 pass, 77 fail, and one skip, up from the first clean 13/125 baseline. With
the browser, base-URL, and URL-wait shards, the 6,526-record Python ledger is 66
passed, 79 failed, one skipped, and 6,380 not run. This is an execution
checkpoint, not a conformance claim; remaining gaps are recorded in
[Official Playwright Python conformance](python-official-conformance.md).

## ABI 7 media-emulation checkpoint (2026-09-26)

ABI 7 adds the default-off `emulate-media` leaf. Rust owns the bounded
per-Page override table, enum validation, serialized CDP command admission,
post-acknowledgement state commit, closed-Page reconciliation, and shutdown
settlement. Python only converts keyword arguments into an optional-value
tuple and publishes `Page.emulate_media`; it keeps no browser-state cache.
Unspecified values preserve prior overrides while explicit `None` resets only
the named override to the Context default.

The wheel profile is now `wait-for-url+set-content+emulate-media`. The three
unchanged upstream media tests pass 3/3, and the complete unchanged
`tests/async/test_page.py[chromium]` shard records 50 pass, 74 fail, and one
skip. The cumulative 6,526-record ledger is 69 passed, 76 failed, one skipped,
and 6,380 not run, SHA-256
`bb53e7a98265a0869fe89e97f5f49101d0711de89351da6a4851f17f94898560`.
These results are still a partial execution checkpoint, not a full-conformance
claim.

## ABI 8 timeout-settings checkpoint (2026-09-26)

ABI 8 adds the default-off `timeout-settings` leaf. Rust owns fixed tables for
16 Contexts and 64 Pages, an 8 KiB backend-state reservation, Playwright-style
Page-over-Context and navigation-over-generic precedence, and retirement of
closed objects. Setters issue no browser command and start no task or timer;
Python only forwards validated milliseconds and asks Rust for the effective
deadline at operation admission.

## ABI 9 network-events checkpoint (2026-09-26)

ABI 9 adds the default-off `network-events` leaf and selects wheel profile
`wait-for-url+set-content+emulate-media+timeout-settings+network-events`. The
existing Chromium single-cursor normalizer owns request correlation, Page
registration, subscription activation, lifecycle publication and exact batch
acknowledgement. The host adds at most 32 waiters, 16 queued events per waiter,
and 16 MiB aggregate retained event payload. Rust owns absolute deadlines and
wakes waiters on Page/session retirement; Python invokes only admitted
string/regex/callable matchers after receiving a typed event record.

The unchanged upstream network subset passes 14/14, including request and
response predicates, explicit/default/Context/no-timeout behavior, predicate
exceptions, and Page-close waiter termination. A teardown regression found by
the official run was fixed by releasing the host's final network-installation
owner before awaiting connection I/O completion. The complete unchanged
`tests/async/test_page.py[chromium]` shard is now 64 pass, 60 fail, and one
skip. The cumulative 6,526-record ledger is 83 passed, 62 failed, one skipped,
and 6,380 not run, SHA-256
`1adacc12bfcc8b37711368c2e14c0ca637611986318569aba21f3d18378e05fa`.
This remains a partial execution checkpoint, not a full-conformance claim.

## ABI 10 APIRequest checkpoint (2026-09-27)

ABI 10 adds the default-off `api-request` leaf and selects wheel profile
`wait-for-url+set-content+emulate-media+timeout-settings+network-events+api-request`.
The PyO3 module converts bounded tuples, bytes and structured cookies only. The
shared Rust `pwright-api-request` owner performs HTTP/HTTPS, redirects,
reset-only retries, Basic credentials, cookie matching, response retention,
timeouts and cancellation. Async and sync Python facades provide the public
Playwright shape, JSON conversion and awaited storage-state file I/O; they do
not own a second HTTP client or cookie jar. `Playwright.stop()` disposes every
remaining request context before closing the browser environment.

The complete unchanged upstream `tests/async/test_fetch_global.py` and
`tests/sync/test_fetch_global.py` matrices pass 231/231 candidate invocations
across the three upstream browser-name parameters. Oracle runs are separated
as required by the upstream greenlet and pytest-asyncio loop models and pass
138/138 async plus 93/93 sync. The candidate-only fixture adapter replaces
only the upstream sync test server's greenlet-coupled request waiter with a
thread-safe equivalent for Pwright's dedicated bounded loop thread; test
bodies and assertions are unchanged. The three-browser stop-idempotency and
pending-request cancellation rows also pass 6/6 candidate and 6/6 oracle. The
cumulative 6,526-record candidate ledger is 320 passed, 62 failed, one skipped
and 6,143 not run, SHA-256
`5fd1e08175b2c9e43aa8c9d567f9d749784136520ab66276a75044b76d2f3f9e`.
The oracle ledger records 237 passed and 6,289 not run, SHA-256
`7e75e4ee9e53d251b48273017a1ff2f6fb2233b5136c63f944ba7ee8147f3298`.
The declared minimum CPython 3.9.6 now builds the ABI3 wheel and passes the
same 8/8 unit tests. That runtime gate found and removed the final evaluated
PEP 604 union inside `typing.cast`; Python 3.12 had not exposed the failure.
BrowserContext request sharing, proxy and client certificates remain open.

## ABI 11 client-certificate checkpoint (2026-09-27)

ABI 11 adds `api-request-client-certificates` on top of `api-request`. Python
validates the Playwright public shapes, reads paths asynchronously or copies
bytes, and forwards bounded immutable tuples through PyO3. The shared Rust
identity store owns exact-origin matching, lazy PEM/PFX decoding and cached
Rustls client configurations. Invalid material for an unmatched origin stays
unparsed, matching the official behavior without adding a Python TLS client.

A fixed manifest pins the exact async and sync upstream source digests and
four global-request test functions. Their three-browser parametrization yields
12 invocations per lane. Candidate and oracle each record eight passes and the
same four upstream macOS WebKit skips. Candidate report SHA-256 values are
`4463e607226aa0c3af0cec2e3cae99577bace3ec80afc4ff4c285faa2eeb961f`
and `f5a7c2be118cd51d3f2d646bfcb2cc33f027adb82da2ff47fb7eac9538be2044`;
oracle report hashes are
`ca53d8e6e5eab175febf84aba266a6880fda2c8c6870785d14ae3ee4f8ccf96b`
and `ef9e1d79eb07d911c29eb85dde6c5a71f8e5df4db00de7e0e0060012a56d64ed`.
The cumulative candidate ledger is 328 passed, 62 failed, five skipped and
6,131 not run, SHA-256
`6efa1096d74982be6519ac8f40d78699aa65132b4698bd6e7bc9172e06f3297c`;
the oracle ledger is 245 passed, four skipped and 6,277 not run, SHA-256
`9ee940e834215add7521eb767eb62034e937d85a5fb7a1c93140611e78717b53`.

The repaired `abi3-py39` wheel vendors its OpenSSL dynamic libraries and two
independent builds have identical payloads after normalizing only the SBOM
build UUID and timestamp. The installed wheel passes 11/11 local unit tests,
Ruff, strict Pyright, the exact dependency proof and strict selected-feature
Clippy. It also collects the complete pinned corpus at 6,526/6,526. The
official browser-context and persistent-context certificate rows, proxy paths
and all remaining corpus invocations remain open.

## ABI 11 Page-fill parity follow-up (2026-09-27)

ABI stays at 11 because no native signature changed. Rust's fill browser
program now emits Playwright's exact invalid input-type capitalization and
non-input element labels. The Python facade validates the public value before
PyO3 so a number raises Playwright `Error` with `expected string, got number`
instead of an incidental conversion `TypeError`.

The complete unchanged `tests/async/test_page.py[chromium]` rerun records 67
passed, 57 failed and one upstream skip, improving the previous 64/60/1 result.
Report SHA-256 is
`53848c57654b5d3397edae116701e77b75a65f9bb53b9a44aac14b835c841c48`.
The same three corrected nodes pass 3/3 under the independent official oracle,
report SHA-256
`7cdef633380f69c0fb2043e93ea42b16c2dbb1c39db28c3fe71ce047feb821a8`.
At this checkpoint, the candidate ledger was 331 passed, 59 failed, five
skipped and 6,131 not
run, SHA-256
`7719fe223c9d586a9d2581cafa00f02b53cf131b70163f385bee1b1ee4eeab33`;
the oracle ledger was 248 passed, four skipped and 6,274 not run, SHA-256
`794a42b48e89967142e300c8e3b7347ab22149ae9be05dd48589186e052c4eb5`.

The official oracle launch adapter maps only the upstream
`BrowserType.launch(headless=...)` fixture call to that package's own
`connect_over_cdp` against the dedicated Chrome. A full oracle diagnostic was
interrupted at `page.pause()` because a visible shared Chrome is not equivalent
to the fixture's headless browser; it is not a canonical report. Two repaired
final-source wheels again have identical payloads after normalizing only the
CycloneDX build UUID/timestamp, and the installed wheel passes 12/12 local
tests.

## ABI 12 Page and BrowserContext init-script checkpoint (2026-09-27)

ABI 12 adds two native methods backed by one optional Rust `init-scripts`
capability: Page registration and BrowserContext registration. The Rust store
uses an explicit typed scope and retains at most 128 sources of at most 64 KiB
each. It selects every current target-local session for the owning Page or
Context, installs future Page and out-of-process Frame sessions through the
existing initialization plugin, and rolls back a partially installed current
batch. Python owns only Playwright-shaped string/path validation and awaited
file reading; sync delegates to the same async facade. Node exposes the same
Context operation instead of creating a second language-owned implementation.

The complete unchanged pinned async and sync `test_add_init_script.py` files
pass 9/9 each in the candidate and 9/9 each in the official oracle. Candidate
report SHA-256 values are
`43fc0287f84a6b789145d852feb5a123ab495730d332146d084c925dbee130f4` and
`8d4d8e0cc0d07c934fcda6633e96b689cabfcb70fa6dd43fc339b68b69f1dadf`;
oracle values are
`451b9626c54141ede463d71d71b62bb7ed0bd24deaad93690e25862e2522a9bc` and
`61a7bd9a8abdc6b7194e5ef63c560eb66ddb881e21e8f3ff326350fdae0a2ba7`.
The cumulative candidate ledger is 349 passed, 59 failed, five skipped and
6,113 not run, SHA-256
`49007c845eb0b27fe254d8e58c3374df0f6246642ba7e49e4423ba98c498ad6f`;
the oracle ledger is 266 passed, four skipped and 6,256 not run, SHA-256
`4d2cfb1af8920e6606c5211ba753e872fd693168e1507cd426e8242851b1be8a`.

Two repaired wheels have payload parity after normalizing only CycloneDX
identity fields. Their archive SHA-256 values are
`bde45291f911e0a39069244ca3834aa7fbd3c679192193a521d0721a8c028c98` and
`3a95bd34b73bb068597ad5814065070512174e1f38dc24b5e9795531300188e4`.
The installed first wheel reports ABI 12 with `init-scripts`, passes 13/13
local unit tests, Ruff and strict Pyright; the shared Rust tests pass 3/3 and
the Node package passes 269/269 with strict TypeScript.

## ABI 13 Handle, wait-for-function and tag checkpoint (2026-09-27)

ABI 13 selects the existing optional Rust `handles` and `wait-for-function`
leaves. The PyO3 object retains an `Arc<PendingHandle>` until explicit dispose
or wrapper loss, converts disposal to the Rust cleanup receipt, and exposes no
remote object ID. The first public slice supports `evaluate_handle`,
`json_value`, `as_element`, one direct Handle argument and bounded truthy
polling. Nested Handle graphs, properties and the wider ElementHandle surface
remain explicit follow-up work. Page script/style tags compose those owners;
Python only reads paths and appends the public sourceURL marker.

The fixed unchanged-upstream tag selection passes 18/18 candidate and 18/18
oracle. The fixed JSHandle selection passes 7/7 in both lanes. Candidate report
SHA-256 values are
`eef16558d22d914910da1f76adb24c21713401537952d8b1f5aa8605175a0797`
and `73785780841f80f6887998f7a0dec34dc3a69e12e5bf7cb95ed464d8bb387e8c`;
oracle values are
`c3c4ee77734fafffbfc31cde4dd9a8783ea80c6af5c9949e1838bba49b5b8c7c`
and `045fe6fc13956e23e5cb1fc8342310385528b37a5da1da2d071ef6e73492ca91`.
The canonical candidate ledger is 374 passed, 41 failed, five skipped and
6,106 not run, SHA-256
`ec82609ea8fc60e09eee81026f708201117a78f3c65b788bdd05906d13255123`;
the oracle ledger is 291 passed, four skipped and 6,231 not run, SHA-256
`a0b896ddf90d86ef18470ef348ce23f5b8ca158c205a090613bdc4da61b536b8`.

Two final-source repaired wheels have archive SHA-256 values
`78d6c20536237771aa3d1ed32f457187c7c62393657e0f86badfef005e6b8e23`
and `91c5fcc4319f29eb1092d6565f608b55d21ee24cd827426e5baae0f6b593c668`;
their payloads match after normalizing only CycloneDX serial/timestamp fields.
The installed wheel passes 13/13 local tests, strict Pyright/Ruff and selected
strict Clippy. Rust wait-for-function tests pass 6/6, and the dependency proof
keeps the 108-package Pwright closure unchanged for Handle selection.

## ABI 14 Page binding checkpoint (2026-09-27)

ABI 14 selects the existing optional Rust `page-bindings` owner. Rust retains
bounded registrations, publishes invocation values, validates result
serialization and owns cleanup. Python retains public callable identity and
executes the callable, but it does not observe CDP, install browser scripts or
create a second invocation registry.

The fixed unchanged-upstream manifest passes 12/12 candidate and 12/12
independent oracle invocations. Candidate report SHA-256 is
`85de412561f1139288706adfb2969e8a9c5685ea66c6eba3ab6b54ee0e7d1920`;
oracle report SHA-256 is
`65cc682d7e21a35d41d3a5a2c36e745e1f7215ad14d7d69dd14f2be4de29c16b`.
After exact Page-report replacement, the 125 cases were 97 passed / 27 failed /
one upstream skip, SHA-256
`608d1d0940997edfb312f01d4f580b14d1148ed4bf18404c5058d60de217b641`.
The historical canonical ledgers were candidate 386 passed / 29 failed / five
skipped / 6,106 not run, SHA-256
`3de873326fc79f7faed48dd7ee9ed08259c9489052619a903376a7613a5ffa21`,
and oracle 303 passed / four skipped / 6,219 not run, SHA-256
`ce99afaf3c96fdaa39dafee861f29cad89cf277a7fcd271117714cee55cd09fb`.
ABI 15 supersedes this checkpoint.

## ABI 15 Page lifecycle event checkpoint (2026-09-27)

ABI 15 selects the existing bounded Rust Page-lifecycle publication leaf.
Rust owns Page creation, load, DOMContentLoaded and close observation, native
ordering, close confirmation, publication capacity and network-adapter
retirement. PyO3 commits and acknowledges each notice, releases the exclusive
drain, and only then invokes the Python scheduling callback. The callback
projects immutable context/page/opener/frame/URL descriptors onto the facade's
single wrapper registry; it never polls CDP or re-enters native publication.
Browser teardown cancels the one lifecycle watcher before closing the native
session.

The fixed manifest pins six unchanged `tests/async/test_page.py` functions for
load, DOMContentLoaded, explicit close, `window.close()` and opener behavior.
All 6/6 pass in the candidate report, SHA-256
`27c46dc49101b3047f5ece6b57c774319576f1fd47f714fa16412028ac9a9081`,
and all 6/6 pass in the independent official oracle, SHA-256
`a1b2e6080dcb7e406c2a9a903d9d24340a0edddd0ff00a4547e8c640e2a0e621`.
Exact replacement produces a 125-case Page report with 103 passed / 21 failed /
one upstream skip, SHA-256
`672e39ff6b08068e4470386a0e70c925d7b52de8c1aa091bd0cb042c7b6a0c41`.

The canonical candidate ledger is now 392 passed / 23 failed / five skipped /
6,106 not run, SHA-256
`c7d473aa7497edd89fde62987e929eb0dcceddb6be7472c4b34483502609aa98`.
The oracle ledger is 309 passed / four skipped / 6,213 not run, SHA-256
`15206a3c9f20193e317acd7ab2ff8a3e4e74adaa6f9a740941421101b145bafc`.
Two final-source repaired wheels have archive SHA-256 values
`b9fae36315280fbc80da0178d738fdca9485c1baa6263fe6c74bc557481a98f8`
and `e1c1fc914fdad0fb1af78dfc9364056d8b72c2d87012426c1a4c938cbba259c6`.
Their payloads match after normalizing only CycloneDX serial/timestamp fields.
The installed first wheel passes 13/13 local tests. Ruff, strict Pyright,
feature-cold and selected-feature Clippy, Rust formatting and the exact Python
dependency proof pass. Neither the six green rows nor the 420 attempted
candidate rows are a full-corpus pass.

## ABI 16 Page error checkpoint (2026-09-27)

ABI 16 adds a separately selectable Rust `page-errors` leaf over
`Runtime.exceptionThrown`. The native observer maps main-frame, OOPIF and
target-session exceptions to their Page, bounds name to 256 bytes, message to
16 KiB and stack to 64 KiB, reserves exact payload storage, and publishes the
error through the existing ordered lifecycle fence. Python receives name,
message and stack only after commit and constructs the public Error wrapper;
it owns no CDP observer, error queue or polling loop.

The executable dependency proof records the exact package boundary:
`page-events` adds only the existing
`pwright-plugin-chromium-frame-lifecycle` package to the feature-cold
108-package closure, while `page-errors` adds no package beyond the
`page-events` closure.

Four focused Rust tests cover Error stacks/custom names, primitive thrown
values, object descriptions and stack construction without a RemoteObject.
The fixed unchanged-upstream Page-error manifest passes 5/5 candidate and 5/5
independent oracle invocations. Candidate report SHA-256 is
`28cd246e9902b1b36494b7e6ffbee0b8fede241426b834e455e2e85f10f2548d`;
oracle SHA-256 is
`e341777d61bc4d02dd11d8e4233103202cf55905ebcb658902020a618288cc2f`.
Exact replacement produces a 125-case Page report with 108 passed / 16 failed /
one upstream skip, SHA-256
`1414096c00e3905b6d0f080e8357eb6fa2cd7b23047125cfd16c2b24bceb52d2`.

The canonical candidate ledger is 397 passed / 18 failed / five skipped /
6,106 not run, SHA-256
`14bbdd2c73805097b84f98f965c0bd8e33639c0a4436c898dedb747a400297eb`.
The oracle ledger is 314 passed / four skipped / 6,208 not run, SHA-256
`423e862b8fee7e8e8d458eeee58a0b1f230d778fda18c42264603452b9f2a5f0`.
Two final-source repaired wheels have archive SHA-256 values
`aab4fcd451491ff9199c43c064b5c09c94ab60fd1f656f9cb4fdaf2ba569f8e4`
and `65d3eb6ca1cd0553c384c1165d19806181eade685900920a3ba288b2fd2d710d`.
Their payloads match after normalizing only CycloneDX serial/timestamp fields,
and the installed first wheel passes 13/13 local tests. Ruff, strict Pyright,
feature-cold and selected-feature Clippy, Rust formatting, workflow parsing
and the corrected exact dependency proof pass. This is not full-corpus
completion.

## ABI 17 Page request-routing checkpoint (2026-09-27)

ABI 17 adds a separately selectable Rust `page-routes` leaf. Rust installs
`Fetch` patterns on current and future Page/OOPIF sessions, maps each paused
request through the authoritative topology, applies last-registration priority
for multiple string globs/paths, and owns continue/abort plus explicit
retired-session terminal receipts. Python retains handler identity, constructs
the public `Route`/`Request` wrappers and awaits the already selected handler;
it owns no CDP session, route registry or interception engine.

The leaf admits at most 64 registrations and 64 pending calls. Pattern, URL,
method, resource type, aggregate headers and POST data are capped at 4 KiB,
64 KiB, 64 bytes, 64 bytes, 64 KiB and 256 KiB respectively. The delivery
queue is fixed at 64, glob scratch allocation is fallible, and Page shutdown
cancels the single Browser route watcher before native teardown. The executable
dependency proof keeps the feature-cold closure at 108 Pwright packages:
`page-events` adds exactly `pwright-plugin-chromium-frame-lifecycle`, while
`page-routes` adds no package beyond that closure.

Four focused Rust AAA tests cover Playwright-shaped glob/path matching,
leading-path CDP projection, abort-code validation and continue after Chromium
retires the Page session. The fixed unchanged-upstream manifest passes 5/5 in
the final-wheel candidate lane and 5/5 in the independent oracle. Candidate
report SHA-256 is
`b8da7c43b37f0c7cabd9cd740348b907fe20ad4899b749d821c72818f6d3b3c3`;
oracle SHA-256 is
`eea2184bd73833dbb532c3ba252e7259c3b918d06b8cdcbc8c92d1bc42546490`.
Exact replacement produces a 125-case Page report with 113 passed / 11 failed /
one upstream skip, SHA-256
`188e6bcd09ae3dad589a24e1d2d1632ad8d01424cdb65bfc75e1f5308e93da30`.

The canonical candidate ledger is 402 passed / 13 failed / five skipped /
6,106 not run, SHA-256
`918c3da79ac696ef05a504693d3b5a09cab43e97b6ceab82fa29438dab3e3fd8`.
The oracle ledger is 319 passed / four skipped / 6,203 not run, SHA-256
`882eb96a2267c98589fb6811d972f67f1d0d814f51e0945e3ef6c778d8436036`.
Two final-source repaired wheels have archive SHA-256 values
`440691bb254993669babecc422be9eac6d1cdbec3d91d7b314524c967fa3289c`
and `4527aee3a74b4c3f8b865500dc1f90e4365df36325f28501db0b5c536943a6fa`.
Their payloads match after normalizing only CycloneDX serial/timestamp fields,
and the installed first wheel passes 13/13 local tests. Ruff, strict Pyright,
feature-cold and selected-feature Clippy, Rust formatting, workflow parsing
and the exact dependency proof pass.

This checkpoint claims only async Page string patterns and `continue_()` /
`abort()`. Regex or predicate matchers, `times`, continue overrides,
`fulfill`, `fallback`, `unroute`, BrowserContext routing and sync routing remain
unsupported. The five green rows and 420 attempted candidate rows are not a
full-corpus pass.

## ABI 18 Request sizes checkpoint (2026-09-27)

ABI 18 exposes async and sync `Request.sizes()` while keeping all browser and
settlement state in Rust. Every published Request may carry one opaque
`RequestSizeOwner`; Python cannot inspect a native ID and only awaits
`NativeSession.request_sizes(owner)`. The owner admits through a 64-slot Rust
semaphore, wakes on success, failed request or session/Page close, and returns
the five Playwright-shaped integer fields. Weak host indexing prevents a
completed Request wrapper from creating retained history.

The Chromium adapter decodes bounded `request.postDataEntries[*].bytes`, with
the legacy `postData` string as a fallback. It correlates
`Network.requestWillBeSentExtraInfo` raw request headers into the terminal
immutable Request snapshot. Extra-info that arrives before its Request uses a
separately bounded pending-header table and never consumes active-request
capacity; stale attach-boundary response/finished/failed tails with no observed
start are ignored without allocating a portable object. Favicon requests stay
in native lifecycle bookkeeping but are not published as public Page request
events, matching the official Playwright filter.

The fixed unchanged-upstream manifest passes 2/2 from the final candidate
wheel and 2/2 in the independent official oracle. Candidate and oracle report
SHA-256 values are
`6506a861160f462eca4b6b104b0f8c9a11f7307b1518c1a592ed9d1804b8aa47`
and `bb71eed6453fe9c8d2fc14563877804f44893df23e227cf2c8f6b01ade66de51`.
Exact replacement produces a 125-case Page report with 115 passed / 9 failed /
one upstream skip, SHA-256
`07cf64b7c9cf428c8c95daf6fea98a47d17a0d33ca92c60c19b9cf8eaaae2e0c`.
The candidate ledger is 404 passed / 11 failed / five skipped / 6,106 not run,
SHA-256
`0fd2ac2ba00ed0526deb10238cfd9be7a4c0e159c8522d230e2a796befc23c63`;
the oracle ledger is 321 passed / four skipped / 6,201 not run, SHA-256
`31773fcf24b25fc5a4ec7dad75db083e514b20ec38dcc9362167b54ac7a8315c`.

Two final repaired wheels have archive SHA-256 values
`ac314f048915df9f7f8a2303a4d88eec8c353d29b0821b961e5b241e9e4db0a9`
and `2bb81c1e7f96e0d32f4f4ce346075ab4c125be481aaa175971ad794e2d5954b1`.
Their payloads match after normalizing only CycloneDX serial/timestamp fields;
the installed first wheel passes 13/13 local tests and recollects 6,526/6,526.
This checkpoint closes only the two Python Page request-size rows. Complete
Core response-size, redirect and multi-browser conformance remains open.

## ABI 19 drag-and-drop checkpoint (2026-09-27)

ABI 19 adds a separately selectable `drag-and-drop` capability to the Rust
host and PyO3 addon. The async and sync facades expose `Page.locator`,
`Page.drag_and_drop` and `Locator.drag_to`; Python converts public position,
step, timeout, trial and force values into one fixed-size request but owns no
selector resolver, input queue or CDP state. The native boundary rejects
`force=true` because force semantics are not installed rather than silently
weakening actionability.

Rust verifies that both locator recipes refer to the supplied live Page and
Frame. It scrolls each element into view, obtains bounded viewport geometry,
uses the center by default or a finite top-left-relative public position, and
delegates the one source move, left-button down, bounded target move and
left-button up sequence to the existing retained Mouse/HTML5-drag runtime. A
started drag attempts button release even if target movement fails. `trial`
performs the resolution and geometry phase without input. This slice does not
yet implement every upstream stable/actionable/hit-target retry or force rule.

The dependency proof keeps the feature-cold Python closure at 108 Pwright
packages and shows exactly one `drag-and-drop` addition:
`pwright-backend-cdp-drag`. Three Rust AAA tests cover center-point selection,
explicit relative positions and invalid fractional/excessive steps. The host
library passes 469 release tests with ten explicit real-browser ignores. The
installed final wheel passes 15/15 local tests, strict Ruff and Pyright pass,
Rust formatting and selected strict Clippy pass, and the final wheel recollects
all 6,526 upstream Python invocations.

The fixed unchanged-upstream manifest pins four source digests and eight exact
functions. Candidate and independent oracle lanes each pass 6/6 async plus 2/2
sync. Candidate report SHA-256 values are
`3069011993af14bceb5f2d367f23227bd5dbae727d8b1264617ac2acde1ed2f9`
and `4287ae3d14813d32edccaa4ae422581f141745a85e06037dbed805313d177f8f`;
oracle values are
`5ef118e81c96770c699f817e464db6151798e7445306c504ceea5cb339347c07`
and `dcf390058a973cbf86ded8efef0c03cdc20e90d2a823912376f6110e0d0e381d`.
The checked selector extracts exact expanded JUnit node IDs: four existing Page
rows replace their ABI 18 outcomes and four Locator/sync rows are added once.
The resulting Page report is 119 passed / five failed / one upstream skip,
SHA-256
`ab5d9e713f0e465c008e004aa06ac82bab218362230de2779d797ea230700d2a`.

The candidate ledger is 412 passed / seven failed / five skipped / 6,102 not
run, SHA-256
`0f3ac26e7edb0d83c11a6e56e28a3adfcfbcb022e9976ea44796111dce9713ee`;
the oracle ledger is 329 passed / four skipped / 6,193 not run, SHA-256
`66a85bab5620c6611f3cc4503443f0a0a6961738951e3c1500003be32e018cc2`.
Two final repaired wheels have archive SHA-256 values
`0766475c32dbc5f86128c57347c4311caf505a069d012699ada916b3d035255f`
and `99a5ca07d5e9e0e628a9fceac1c2db2743b289926f007f915bc4c80723ddca3c`.
Their payloads match after normalizing only CycloneDX serial/timestamp fields.
This is an eight-row compatibility checkpoint, not a full-corpus pass.

## ABI 20 Page-dialog checkpoint (2026-09-27)

ABI 20 adds a separately selectable `page-dialogs` capability to the Rust host
and PyO3 addon. The host observes `Page.javascriptDialogOpening` on the ordered
Page lifecycle fence, resolves Page ownership from the event frame, and retains
the exact CDP session that emitted the event for its eventual
`Page.handleJavaScriptDialog` command. This matters because Chromium can emit
the same opening through an auto-attached observer session while the Page's
ordinary commands use another session. Rust deduplicates those notifications
to one active dialog per Page before allocating an ID.

Each native dialog action is single use and owns its resource lease until it is
accepted, dismissed or dropped. The leaf admits at most 16 live dialog owners,
charges 1,024 backend bytes per owner plus 2,048 installation bytes, and bounds
the incoming message, default value and outgoing prompt text at 16 KiB each.
The Python async and sync facades retain only public wrapper and callback
identity. They choose Playwright's default action when no listener exists
(accept beforeunload, dismiss other dialogs) and delegate the action to the
opaque Rust owner. `Page.close(run_before_unload=True)` similarly crosses one
typed FFI request into the Rust lifecycle command service.

Four Rust AAA tests cover descriptor preservation, exact prompt dispatch,
capacity exhaustion and duplicate-session suppression. The dependency proof
keeps the feature-cold Python closure at 108 Pwright packages and shows that
`page-dialogs` adds no package beyond `page-events`. The installed final wheel
passes 15/15 local tests, strict Ruff and Pyright pass, Rust formatting and
selected strict Clippy pass, and the final wheel recollects all 6,526 upstream
Python invocations.

The fixed unchanged-upstream manifest pins three source digests and 11 exact
functions. Candidate final-wheel and independent oracle lanes each pass 10/10
async plus 1/1 sync. Candidate report SHA-256 values are
`80c253a841995ba47251a9a3cb5040479573a12cc2bbf87de378db8569c1086f`
and `746df62a98ab2ccb722a36607e6864a05ed1bcbfd5059f4947fe1b8256162b13`;
oracle values are
`dd8f414e0e9c1e38865826ce8d7b165f77c8de9c18ec5254aba16aadedef3718`
and `b0a1b0e3aa68ca2c92fea9c856364f47195933cd63e1e345e969fd325d9d450e`.
Exact selection replaces two existing beforeunload failures and adds the other
nine rows once. The resulting Page report is 121 passed / three failed / one
upstream skip, SHA-256
`70eef0e767ad5b3385d9bdbcd9b2a71e5ebbf998827a25070e76f805f95cfd36`.

The candidate ledger is 423 passed / five failed / five skipped / 6,093 not
run, SHA-256
`d139b5697935e328a9a6d3b1a2a3a9b0181c3a07139872fe875630d1b0a0bb3c`;
the oracle ledger is 340 passed / four skipped / 6,182 not run, SHA-256
`2f2543db5a58b082d70a305928138d03136860b0c6b2f35582f31278d59fb817`.
Two final repaired wheels have archive SHA-256 values
`4c057a55597213a7842caa670c27bd38643811df30e6282a7a8229d3509b9d27`
and `68bf970bb1ed3480ccc66c1b6a116c7fe2367fe48318c28edd85da22ad847c04`.
Their payloads match after normalizing only CycloneDX serial/timestamp fields.
This is an 11-row Chromium compatibility checkpoint, not a full-corpus or
Firefox/WebKit pass.

## ABI 21 route fulfill and response body checkpoint (2026-09-27)

ABI 21 extends the separately selectable Rust `page-routes` and
`network-events` composition without moving browser behavior into Python.
Relative route strings are resolved against the Rust-owned Context base URL.
`Route.fulfill` validates status, headers and body before Rust dispatches
`Fetch.fulfillRequest`; Python performs only public argument normalization.
The installed facade supports status, headers, `content_type` and UTF-8/string
or byte bodies. JSON, path and response-derived fulfillment are explicitly
rejected until their native capabilities exist.

Rust admits at most 256 fulfillment headers, 64 KiB of aggregate header bytes
and a 1 MiB body. At most eight fulfilled bodies are retained, keyed by the
exact CDP session plus native network request ID. The network batch exposes
those bounded transient correlation keys only to the host, which consumes the
body exactly once when publishing the matching Response. Python receives an
opaque `NativeResponseBody` and `Response.body()` asks Rust for the bytes; it
does not maintain a parallel response-body registry. Session retirement,
terminal receipts and Page teardown remove pending calls and retained bodies.

The full dependency proof keeps the feature-cold Python closure at 108 Pwright
packages and adds no Pwright package for `page-routes` beyond the existing
`page-events` closure. Targeted Rust AAA tests cover fulfill validation, exact
body correlation and retained-byte charging. Strict selected-feature Clippy,
Rust formatting, Ruff and Pyright pass. The installed wheel passes 17/17 local
tests and recollects all 6,526 official Python invocations.

The fixed unchanged-upstream manifest pins one exact base-URL function.
Candidate and independent oracle lanes each pass 1/1 on Chromium. Candidate
report SHA-256 is
`1263c7aa7848e919c22291670bd863f88a1f23dcf704c40b613f4bfcbc1884c8`;
oracle SHA-256 is
`e8b3e46a074804cb92f50fd2a7f173bac76f570daa31b30df506a1fc6d14fde6`.
Exact replacement produces a seven-case base-URL report with six passes and
one failure, SHA-256
`785164e10c8aba918e6a4f00c6da3b7ee3d4529344a9c8421c49822329a49a6e`.

The candidate ledger is 424 passed / four failed / five skipped / 6,093 not
run, SHA-256
`e997512cfb5be3986ab2fad04deaf7c114a2da4b9db675f97243f442fb11357b`;
the oracle ledger is 341 passed / four skipped / 6,181 not run, SHA-256
`eb9ffdee9f86cce60cc115c2c0ca60ee4f299d588665d24893856e3036373deb`.
Two repaired wheels have archive SHA-256 values
`88015c44fe0967b531ee3d8ca01d9b8537c07088c7bfb3ec3edd6aecc74e71ea`
and `751c905bcf7c4d1fedfee569f9ce2e795da2d342b62887625aae78f98f360af0`;
their payloads match after normalizing only CycloneDX serial/timestamp fields.

An earlier development run passed its assertion but hung during teardown. The
cause was a task-local `Arc` that retained the route controller and therefore
the CDP connection while the host awaited transport completion. The canonical
host explicitly drops that owner before joining I/O. Only the repaired-wheel
candidate and oracle reports that exit cleanly enter the ledger. This closes
one row, not the full corpus: two pause rows, one disconnect waiter and one
persistent-context row remain failed, while 6,093 candidate rows remain
unexecuted.

### ABI 21 Page-close waiter extension (2026-09-27)

The Rust Page-lifecycle stream already owns native close observation and
ordered publication. The thin Python event layer now applies that terminal
notice to every pending non-close Page-event waiter and constructs the stable
`TargetClosedError` expected by the public API. It retains at most 64 waiter
Future/predicate identities per Page and rejects further admission before
allocating another Future. `expect_download` exposes this terminal path, but
successful download publication, `Download` objects and artifacts remain
explicitly uninstalled.

One Python AAA test proves close rejection and another proves fixed-capacity
admission. The installed repaired wheel passes 19/19 local tests. The fixed
unchanged-upstream Chromium terminal row passes 1/1 in both candidate and
independent oracle lanes. Candidate report SHA-256 is
`03a028943034b7cbf6962c34e99cc68be07878b9463dec831e46f6d4fe2e7872`;
oracle SHA-256 is
`48b06660ac0af81f99edb46372a6ba6f6058fbc930b497eefcefff3ad387bd53`.
Exact replacement produces a 125-case Page report with 122 passes, two
failures and one upstream skip, SHA-256
`da99fb0ac032a12a9507c5467b257cd38349932bb0c6d707172d86c11fe79fb3`.

The same final wheel reruns the route-fulfill row 1/1, candidate report
SHA-256
`e8d9a3f803797cfff3d914bb47866a4ecda4d92888a7ddd3f41ad87179469f6b`,
and produces the six-pass/one-failure base-URL report SHA-256
`3e465814e57aa12de0c535441e1597a7134a871f25ac4eaeda6e6cc8c75251fb`.
The candidate ledger is 425 passed / three failed / five skipped / 6,093 not
run, SHA-256
`b026f46fa2a4934023b788545ddd769c1cead87e767aea8339a2dd810a98a7e7`;
the oracle ledger is 342 passed / four skipped / 6,180 not run, SHA-256
`4d6186441568d6d6ca19a51e58c36f1439177a2e78a44619d9983d3479581953`.
Two repaired wheels have reproducible payloads and archive SHA-256 values
`015f986890fe67d01c70401206961d623fafde3dcf587d70a4938833d16ed216`
and `f2a99efc425981fa515fa2e6e51e92f9fcd1c935a5b9bbb17567ed260346a6f9`.
The installed first wheel recollects 6,526/6,526, collection artifact SHA-256
`07ec6afbf0e47c4860e05e7650d0900016c3806a4255b8df81b05f83e6fe4a86`.
This closes only the terminal waiter discrepancy: two debugger-only pause rows,
one persistent-context row and 6,093 unexecuted candidate rows remain.

## ABI 22 persistent Chromium Context checkpoint (2026-09-27)

ABI 22 adds `persistent-context` as a separate Python-addon/host launch
feature. `ChromiumPersistentProfile::open_or_initialize` admits only an empty
directory or an existing Pwright-marked profile. The provider supplies the
installed executable and ephemeral loopback CDP endpoint; the host stores the
connection and process lease as one noncloneable shutdown unit. Closing the
default Context consumes that unit before joining transport I/O and performs a
bounded graceful-close/force-cleanup sequence. Attach sessions still have no
process authority.

The Context base URL is selected before topology publication through a stable
external default-Context settings snapshot. The snapshot is cleared during
teardown, so resource metrics return to zero. Focused Rust AAA tests cover
unmarked-profile rejection, pre-publication base-URL stability and lease
release. The dependency proof keeps the feature-cold Python closure at 108
Pwright packages; `persistent-context` adds exactly
`pwright-cap-browser-lifecycle`, `pwright-codec-browser-lifecycle`,
`pwright-composition-chromium-launch`, `pwright-provider-chromium`, and
`pwright-server-browser-lifecycle`.

The fixed unchanged-upstream manifest passes 1/1 in the final repaired-wheel
candidate and 1/1 in the independent official oracle. Both canonical lanes
use installed `channel=chrome` because this machine lacks Playwright's cached
Chromium headless shell 1208; the missing-shell oracle report remains a
separate infrastructure failure. Candidate report SHA-256 is
`57c6a49de8942866cae9f67b480cae5c553ecb672606fce14f3370a5396ff862`;
oracle report SHA-256 is
`0b68621888607947312b52b2341c7e7c366c7312fc3d64b7f068a063026d86ad`.
The complete base-URL source passes 7/7 in the candidate, SHA-256
`9723e98d9361ab85d663479bf02015ff7834d878783a6f0e8cd9e32b5f723c6a`.

The candidate ledger is 426 passed / two failed / five skipped / 6,093 not
run, SHA-256
`4481a1b424e997bbe258493b8075e2d7e7f776e4c4fb9a61739a9884f26bd076`;
the oracle ledger is 343 passed / four skipped / 6,179 not run, SHA-256
`d250b8d62dc4e699741b1a930de51279e38e291de6657c8bade46fa05d360dfc`.
Two repaired wheels have reproducible payloads and archive SHA-256 values
`449735fcd3e26786396a2bb101483ec57f4533408dc2b96aa0b25e819e9ca7c7`
and `8acfaa118c5d633198161a12a23d71c143e1390659ae5fcf400d640cdd001743`.
The installed first wheel passes 19/19 local tests and recollects 6,526/6,526;
the collection XML SHA-256 is
`102dc195db37a1f6d0c54f5846448a85b7ffd3701d1ea5165ab489135cfa39eb`.
Only the two unchanged `page.pause()` rows remain failed; 6,093 candidate
invocations remain explicitly unexecuted, so this is not a full-suite pass.

## ABI 23 headless Page pause checkpoint (2026-09-27)

ABI 23 adds `page-pause` as a separate Python-addon/host feature over the
existing `timeout-settings` leaf. The host stores whether the session is
headful, rejects headed pause explicitly, and uses a drop-restoring
`PauseTimeoutReset` guard for the Page and its Context. Focused Rust AAA tests
cover default and custom timeout reset/restoration, including guard drop.
The typed internal-text selector parser preserves the JSON string and exact or
substring match mode. The dependency proof keeps the feature-cold closure at
108 Pwright packages; `page-pause` adds no package.

The fixed unchanged-upstream manifest passes 2/2 in the installed final-wheel
candidate and 2/2 in the independent official oracle on the same task-owned
headless Chrome launched through Pwright. Candidate report SHA-256 is
`7ffe205309fc5864bf6c9acae98c4e05a1b3d99be3e72fd3569a0090e718ad2c`;
oracle report SHA-256 is
`1c5d32e8553abefefa9fa7b52b6a3cc6d3df0a7adedf21f80cc667e09cd69ff5`.
That two-row checkpoint initially completed a merged 125-row Page artifact.
After the route-close repair, fresh isolated full-file candidate and oracle
runs each record 124 passes and the same upstream FIXME skip with zero status
differences. Candidate report SHA-256 is
`709000c785522a979b7f56b344216a9ed0a4f8b9d783ffc725dea8e15c9ba9f3`;
oracle report SHA-256 is
`d8058d713c4cc1fcefbbab295b4eb9aa5647dc98eb9f8453a8e877319be7dade`.
The Rust route owner retains at most 64 resource-accounted terminal receipts
when Page retirement removes pending interceptions; exactly one later terminal
action consumes each receipt.

The candidate ledger is 428 passed / five skipped / 6,093 not run, SHA-256
`a8b7ce8ed00ed75676cad32d68c6487685839b8fae29ba8f1b79e5cbd0689980`;
the oracle ledger is 409 passed / five skipped / 6,112 not run, SHA-256
`c0858c03e07687bd7786d5f713b548fcbc763b9028b41af50579a5e3384c7755`.
Two final repaired wheels have reproducible payloads and archive SHA-256 values
`a65ac8096484f11a1a71c1b7e13dddbb88b6300c764a16648fd94dc9fc3a95fd`
and `f42c71803b4fd925409c39d0f6bb35055738f9a555826434795e46112ce3fc7e`.
The installed first wheel passes 20/20 local tests and recollects 6,526/6,526;
the collection XML SHA-256 is
`5aa8de7c224b57c35c3eebd0f38030863bdf63a03b843be339d140d6e0106661`.
The remaining 6,093 candidate invocations are explicitly unexecuted, so this
checkpoint is not a full-suite pass.

## ABI 24 Page request-GC checkpoint (2026-09-27)

ABI 24 selects `pwright-node-host/request-gc` through one matching addon
feature. The leaf already owns Page/session lookup, bounded command admission
and the exact empty `HeapProfiler.collectGarbage` payload. PyO3 validates and
copies only the bounded Page ID; async and sync Python facades are one-hop
adapters. The dependency proof remains at 108 feature-cold Pwright packages.

`page-request-gc-manifest.json` pins the unchanged async and sync upstream files
and their exact `test_should_work` functions. Candidate and independent oracle
Chromium lanes each pass 1/1 async and 1/1 sync. Candidate report SHA-256 values
are `f4a28daac38c4c122401c171e71e157852ebab003c7dc4c7dae04f8ab454053f`
and `33dcc10908cfb8923d4b9fbc1dfcfd6639e9f93897515d9b8458d2d06b38e1ae`;
oracle values are
`7bba1bcbf3dc9f90736279a394932fb1bf9b0761f45d7ef2620a8b504a03f81d`
and `b800a26881354e55a9b465a3a67db8db565db5755e2b3e66c6dd49a016699b47`.

The ABI 24 candidate ledger was 430 passed / five skipped / 6,091 not run,
SHA-256
`fce03db46415b6bed7e1d97484aec3abbcbfa9b3fa051a52d1ef73eccf659241`;
the ABI 24 oracle ledger was 411 passed / five skipped / 6,110 not run,
SHA-256
`d0bf59a7f917f8c19d6be9064ebc41e295825d8825459e4fda29e7b717f3fb71`.
Two final repaired wheels have reproducible payloads and archive SHA-256 values
`d9a43bd387517005d0248e85ccd7082ba4f2fe356594a8e28df9af5e7403a000`
and `9c572ea47613ad581e5a5e14f2b6cea4ee6fd04a3fc7aec813b1977ebeabff99`.
The installed first wheel passes 21/21 local tests and recollects 6,526/6,526;
the collection XML SHA-256 is
`22029eb6ad67012f47a8db33a1be3d55ed08aed2646269840d663edfbfd5e91f`.
The remaining 6,091 candidate invocations are explicitly unexecuted, so this
checkpoint is not a full-suite pass.

## ABI 25 Page-error history checkpoint (2026-09-27)

ABI 25 keeps the public history behind the already selectable
`pwright-node-host/page-errors` leaf. Rust owns Page lookup, recording, eviction,
snapshotting and teardown. Python forwards only the bounded Page ID and converts
the returned `(name, message, stack)` values into public `Error` objects; it
retains no history, event queue or protocol state.

The history preallocates 16 Page slots and 200 entries per Page. The resource
plan admits at most 3,200 `EventItem`s and 16 MiB of `EventBytes`, accounts its
fixed storage as `BackendStateBytes`, replaces the oldest per-Page lease in
place at the 200-entry limit, and globally evicts the oldest record if the byte
budget requires it. Page retirement drops that Page's leases; session sealing
clears all histories. Because the official test emits 301 errors before the
sync binding reads them, the selected lifecycle publication/revision queue is
512 when Page errors are installed; the old 256-entry queue was a real bounded
capacity mismatch, not a reason to weaken or modify the official test.

`page-errors-history-manifest.json` pins the unchanged async and sync
`test_page_errors_should_work[chromium]` rows. Candidate and independent oracle
lanes each pass 1/1 async and 1/1 sync. Candidate report SHA-256 values are
`bb61ff804068782deec28497cd7a7a9375d6f2d175a309c202f73d12a38f8ec9`
and `23f9308f0801de4f988c777b1e2004bff06522178e6d8a9be3f0d726c3e65a37`;
oracle values are
`bb61ff804068782deec28497cd7a7a9375d6f2d175a309c202f73d12a38f8ec9`
and `25e87e795543447f398a2967796dc2f9bed3dd97f3574c85c5d70081e85a0034`.

The candidate ledger is 432 passed / five skipped / 6,089 not run, SHA-256
`6c3d2dad4271da574f924a305cadcb8aeb801e4650079249e524df02be0e1293`;
the oracle ledger is 413 passed / five skipped / 6,108 not run, SHA-256
`59aad3d302b5c05d06a3a3fe8898ee881076658ab8b0bb9edc5de2ef91e5e00d`.
Two final repaired wheels have reproducible payloads and archive SHA-256 values
`a15dda04b51b2f05875fdb700d03e54014b411c6d80fd1d37d8aff8e44ebbe4c`
and `3b3498b1fa79fd3168ee53a87ee8adb3eda45bb68bd5ef0010089cfe809b20bf`.
The installed first wheel passes 22/22 local tests and recollects 6,526/6,526;
the collection XML SHA-256 is
`30d96008e1afb4ade52b1bc2aaae4e10838b97e1f024d63e2e9936b5b821743e`.
The remaining 6,089 candidate invocations are explicitly unexecuted, so this
checkpoint is not a full-suite pass.

## ABI 26 console-message history checkpoint (2026-09-27)

ABI 26 selects `pwright-node-host/console-history` independently from
`console-events`. Both leaves reuse the same Chromium `Runtime.consoleAPICalled`
observation and conversion, but history-only installation does not publish
callbacks or create a Python event queue. Rust owns Page lookup, admission,
recording, eviction, snapshotting and teardown. PyO3 returns bounded tuples;
the async/sync facades only construct public `ConsoleMessage` value wrappers.

The history preallocates 16 Page slots and retains the newest 200 messages per
Page, with at most 3,200 `EventItem`s and 16 MiB of `EventBytes`. Fixed storage
is charged as `BackendStateBytes`; replacement and global oldest eviction
release the exact old leases. Page retirement releases that Page's entries and
session sealing clears every remaining lease. `type`, `text`, `location`,
`page`, and `worker` are public. `args` raises `UnsupportedError` because
bounded retained remote-object/`JSHandle` ownership has not been installed;
this checkpoint does not claim that surface.

`console-history-manifest.json` pins the unchanged upstream async source at
SHA-256
`55d06d0935481623e6143e0120ba7921c4fcbef61f8072e7976db925e82660dd`
and sync source at
`098c324a79cf8140669195083a146ae3f1b9eb1b1ea282ed1f1331abf8365ecc`.
Both exact `test_console_messages_should_work[chromium]` rows emit 301 console
messages and assert that the newest 100 are exactly `message201` through
`message300`. Before implementation, the ABI 25 candidate failed at the public
method call with `AttributeError`, while the independent oracle passed. The
final ABI 26 candidate and oracle each pass 1/1 async and 1/1 sync unchanged.
Candidate report SHA-256 values are
`95c417b6bdcaf56255146a92374fd0c95cdc2828cf9f746e416661175bf320fb`
and `2298b65eb48feb4292db52d2ce9258aa7d2e79308189485d6e25bb1932050be6`;
oracle values are
`5978e92a8f7862bd3781ad491deaabfc3d66264157dda652b44b37bc23435a02`
and `c5bd148d8e79655d955a5b33783d60310b368213dd9a5b8f6fd3029cb659c722`.

The candidate ledger is 434 passed / five skipped / 6,087 not run, SHA-256
`9445e7a0c6d424cee6b604a8fc43804ddb4304ab4d97f1eac7371e3398e4d41e`;
the oracle ledger is 415 passed / five skipped / 6,106 not run, SHA-256
`354fadad72ee68e599644973aa648e941c4619499f3b0a12b30049441a080773`.
Final repaired wheel archive SHA-256 values are
`0e84de688da9c80f1be5ad0b04bba89b4782643bd112a5bf3830b80ba49ee9ed`
and `4ba5129f7df145b45dfae0b56a4ad406d40a812ef5492a27d732d5c034b4a8ff`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 23/23 local tests and recollects
6,526/6,526; collection XML SHA-256 is
`28d4b289870ab589f86c153e95bcf1a3f44bdb2f9c63dbacb3b9ca5564467797`.
The remaining 6,087 candidate invocations are explicitly unexecuted, so this
checkpoint is not a full-suite pass.

## ABI 27 request-history checkpoint (2026-09-27)

ABI 27 selects `pwright-node-host/request-history` independently while reusing
the exact `network-events` package closure and its four normalized lifecycle
tags. BrowserContext-scoped activation subscribes existing and future Pages;
an owned empty Context synchronizes its exact topology before activation so
its aggregate route cannot omit a Page that already exists. Partial activation
and Context/Page/session teardown compensate and release the same Rust owners.

Rust retains the newest 100 Requests per Page across 16 Page slots and
separately permits 256 live owner identities per Page, 4,096 owners globally
and 64 MiB of event bytes. Eviction from the history does not invalidate a
Request still held by user code: its Response and body settle on the same
owner. The official sync row creates 199 concurrent retained reads, so the
selected FFI observation lane admits 256 reads and charges each body by its
actual length plus 4 KiB. The unrelated general command lane remains capped at
32. The Page route table is bounded at 256 entries. Python exposes only thin
async/sync `Page.requests()`, `Request.response()`, `Response.text()` and
string-route wrappers; it owns no request history, response body or protocol
state.

`request-history-manifest.json` pins the unchanged upstream async source at
SHA-256
`f0e5eef63b0d9707bc622e06453869e3d29c093398b4e1c52378c79ca2920435`
and sync source at
`2206d2fcec8f038c87803157be86f7e0e4bbf183be9ac3b8909b0f8755d87bb5`.
The exact async and sync `test_should_return_last_requests[chromium]` rows each
issue 200 routed requests, assert the last 100 URLs, then read all 199 retained
responses and bodies. Final-wheel candidate and independent oracle each pass
1/1 async and 1/1 sync unchanged. Candidate report SHA-256 values are
`cb01e3973a1f9a0235bc5954b70766271cf369013a2203170f7cd6a302a8ac45`
and `f30f8e87a722844dbb552133faa52fc1a836f4db8ce25073cb333cda0506aaae`;
oracle values are
`3ea6ac97cf691a63cf736784434e2221e3f47d8690fa289d66cd135331e53ec6`
and `363f49ae1e654259ac5f94e9df72d9f7610281fa304a3de1f1287d14b3986d4f`.

The candidate ledger is 436 passed / five skipped / 6,085 not run, SHA-256
`2fb546a564e528dfc3788466aae16bb6fc556163e950eff2f26df2bd90e4042e`;
the oracle ledger is 417 passed / five skipped / 6,104 not run, SHA-256
`9ba7a572ab3c66a18b0c409ba09ac4b32c23db36f84d9c73c3be6db7b6040a4c`.
Final repaired wheel archive SHA-256 values are
`6eb888eb3a453e7d1894fbaeffd43ba7fab9ad803cbb2535ccf38645ef2cedbb`
and `8780570e0e93f4dfc9bab79013769b77a31a8c1b340de288484cf35a0463b03a`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 25/25 local tests and recollects
6,526/6,526; collection XML SHA-256 is
`7376edf2b4493552e0d5b2bedad0969d31ff01c6fe5ad3306a9feaac0acd5be0`.
The remaining 6,085 candidate invocations are explicitly unexecuted, so this
checkpoint is not a full-suite pass.

## ABI 28 device-descriptor checkpoint (2026-09-27)

ABI 28 selects `pwright-node-host/device-emulation` independently. The Rust
host embeds all 143 Playwright v1.58.0 descriptors as immutable package data;
the embedded JSON is semantically equal to upstream
`deviceDescriptorsSource.json`, whose raw SHA-256 is
`ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353`.
The local formatting-only representation has SHA-256
`dd2323534c84e53ad9bfde2d18bf9cd05ff294803e14e767ad6ea0a4227abf0c`.
Python converts the official camel-case keys to the public snake-case typed
dictionary and forwards only validated device values during Context creation;
it does not own device state or issue CDP commands.

Rust admits at most 16 configured Contexts and 64 KiB of retained settings,
limits a user agent to 4,096 bytes and each viewport dimension to 10,000,000,
and charges the Context lease before retention. The settings are keyed by the
Rust Context ID, retained across in-flight session initializers, removed on
Context close and compensated if creation fails. The initializer applies
window bounds, device metrics, screen orientation, touch emulation and user
agent after the existing defaults but before a Page is resumed or published.

`device-descriptors-manifest.json` pins the complete unchanged upstream
`tests/async/test_device_descriptors.py` at SHA-256
`0ce770e824526029a38991b7bad0ce6176dac8eb2593faf793252a394411c560`.
The source expands to Chromium, Firefox and WebKit invocations. Final-wheel
candidate and independent official oracle each record one Chromium pass plus
the same two upstream `only_browser("chromium")` skips, with no changed
assertions. Candidate report SHA-256 is
`e73a060a2ae5c4991c91bfb16f18c76cbad10a970754d094473aa11700edecae`;
oracle report SHA-256 is
`af89d5aeb07177c83bca47e491c99e28b9abc12b31006bea06d5463fcb8be65c`.

The candidate ledger is 437 passed / seven skipped / 6,082 not run, SHA-256
`c02c13e38b6b3dae4d83ec8ad280088dabd7432c30b70ecfeb83e83bf9e5cfc3`;
the oracle ledger is 418 passed / seven skipped / 6,101 not run, SHA-256
`a60219448e4b669fd5c3520186e93579b2e3fea6692492d483c0845a7fd3c644`.
Final repaired wheel archive SHA-256 values are
`42a1d574c079f5c1f890ef6e44272ffccf91ee532e9ed3d17f3ae126552d7273`
and `5f36e05126c219480c62d425e3da79ff3d6fc7180c42efa4e2fb35e7f97f321a`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 26/26 local tests and recollects
6,526/6,526; collection XML SHA-256 is
`45baee9da8d37c09e052db9e760a7e65b605bd868fedce562c1447a1c6ec393d`.
The remaining 6,082 candidate invocations and the Core JS/TS execution are
explicitly incomplete; this is not a full-suite pass.

## ABI 29 response-listener checkpoint (2026-09-27)

ABI 29 selects `pwright-node-host/response-events` independently over the
existing `network-events` composition. Rust activates the response tag at
BrowserContext scope before the Context is published, admits at most 16 live
Contexts and 256 queued notices, and charges retained response values to the
same 16 MiB byte budget as network waiters. Page retirement removes queued
notices and session close wakes blocked consumers. When `request-history` is
selected concurrently, explicit interest coordination prevents either leaf
from disabling the other's shared response subscription.

The PyO3 boundary exposes one bounded `next_response_notice` observation call.
Async Python runs one watcher per Browser, converts the native response value,
and retains only public callback identity; sync Python wraps that same path and
preserves identity for `remove_listener`. Neither facade owns browser network
state, protocol dispatch or an independent response queue.

The first final-wheel retries preserved a second systems discrepancy: the
bounded CDP ingress reported sticky topology event loss, and the initial
authoritative reconstruction then treated a live Page with a transiently
missing session route as absent. The host now performs exactly one
same-connection reconstruction when the driver reports `resync_required`,
advances the connection epoch exactly once, and propagates every non-loss or
reconstruction error. The topology backend separately exposes materialized
Page-to-Context membership, so reconnect may remove command capability without
falsely closing the portable Page identity. There is no unbounded retry loop
and no new dependency closure.

`async-listeners-manifest.json` pins the complete unchanged
`tests/async/test_listeners.py` source at SHA-256
`dbda3cea63f1820b2d0ac4587e9bcaf938c43c318ca815294f00bcabbbbba4a9`.
Its three-browser diagnostic reports keep the Firefox/WebKit provider errors
explicit; diagnostic SHA-256 values are
`68f03a0cc3aad76a36aaa443a71ce68724c25b027e5f33b4bcd530195f062363`
and `b2e852f865bee4c16490c5754c5405ef7b56bd78b2e6abf57830db9f2b30fb38`.
The applicable Chromium invocation passes unchanged 1/1 in the final-wheel
candidate and 1/1 in the independent official oracle. Final report SHA-256
values are
`31ffbee02631a56fecb4e5ffa387db13317445f6b10656e939d9865bd98c9261`
and `9fee5338e25b6927933343356e88f4ff5fafa5b01fc67e02866d79bf7b1fcf8a`.

The candidate ledger is 438 passed / seven skipped / 6,081 not run, SHA-256
`382e3c5a4fa98f8bd22e9503066d21a0236ded7f7f823a10b8e7ee3d17e439c2`;
the oracle ledger is 419 passed / seven skipped / 6,100 not run, SHA-256
`a0d41d0fe578361d80998cc60cdc25b7c07edb44e455061eccf64640785ec13f`.
Final repaired wheel archive SHA-256 values are
`05bf16e458ed4a68dd669e99b50b56f7792602c4717548ecf73ce630c3c85b0f`
and `0be5b154d2c3647f63fb7ab5a66b69908009f41f28968d1d4acd16177ac9b234`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 28/28 local tests and recollects
6,526/6,526; collection XML SHA-256 is
`d9fb0c10e98f170e45b614d56d02a6605d3ffdb14dbfd5f71b02e662c8bcaa0e`.
The final Chromium pass was obtained without closing or mutating the unrelated
shared-Chrome application tab. Failed diagnostic/fix-attempt reports remain
preserved as evidence and are not inputs to the green ledger.
The remaining 6,081 candidate invocations, Firefox/WebKit providers and Core
JS/TS execution are explicitly incomplete; this is not a full-suite pass.

## ABI 29 sync-listener completion checkpoint (2026-09-27)

The public ABI stays at 29 because this checkpoint changes no exported native
shape. `sync-listeners-manifest.json` pins the complete unchanged
`tests/sync/test_listeners.py` source at SHA-256
`24c14d60da2cc007ca29c68de83982644b3a82320249145953e0a7125fa88658`.
Its applicable Chromium invocation passes unchanged 1/1 in the installed-wheel
candidate and 1/1 in the independent official oracle. Final report SHA-256
values are
`98f92bdeb7672c3511bc08d52a6c1f5140fb044bcd511f8f7bca584bcc1e9e05`
and `0e28ea198745648d83b047914d4241ae02d4bcb0f2861129ddecf1895ada04c7`.
The full-file diagnostic SHA-256 is
`1e5f7f58a6cc14e774b0de1027d500d8aaf9fe97217ded561275d97c2db3fdc1`;
its Firefox/WebKit provider errors remain explicit and outside the green
ledger. The async listener regression passes 1/1 on this same wheel, report
SHA-256
`d87385340c9481be56d0b0f8fd4c8ce7295dea83bca0819d3b626d22a2754418`.

The initial sync attempts showed that topology identity recovery alone did not
preserve the network subscription adapter. During an incomplete authoritative
snapshot, route reconciliation now retains the existing Page lane. After
`finish_reconnect`, the registration handle validates the rebound CDP session
and releases adapters only for Pages missing from the completed snapshot. The
host's bounded replay window is 2,048 CDP events for the declared
16-Page/128-session/256-world profile. This increases the declared connection
resource budget but adds neither an unbounded queue nor a retry loop.

The candidate ledger is 439 passed / seven skipped / 6,080 not run, SHA-256
`4da01fe20cf788ee1a12f28509708166ed31b5f3eb3cefd78672ba4cddc3ffee`;
the oracle ledger is 420 passed / seven skipped / 6,099 not run, SHA-256
`f654d55f1bb21a0382a2bca2636438070a0c14e3b92cdcd1d7c2e643b6f9ea90`.
Final repaired wheel archive SHA-256 values are
`f2eea005f003fa036d8cb6e0e2d2080066fec5e01f92d2c460098181dccd5f96`
and `91a51dd7db6c5b9d068ed47064f0de520fe66fd2995d8c52904cac538dd6a14a`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 28/28 local tests and recollects
6,526/6,526; collection XML SHA-256 is
`839fff4356fd0b066257194825f1e9547a5eeb31336f8195da6324cfb7815547`.
The remaining 6,080 candidate invocations, Firefox/WebKit providers and Core
JS/TS execution remain explicitly incomplete.

## ABI 30 collection selector evaluation checkpoint (2026-09-27)

ABI 30 pins the complete unchanged `tests/sync/test_selectors_misc.py` source
in `packages/pwright-python/tests/official/sync-selectors-misc-manifest.json`
at SHA-256
`940c7576586bc22381f260de40349d8ac3632ddb05ff0415b1af5ed3beeaf532`.
The exact Chromium invocation passes 1/1 in the final installed-wheel
candidate and 1/1 in the independent official oracle without changing any of
its six assertions. Candidate report SHA-256 is
`ee4fb9da609e9296d3d6ab2ee20a2dd07addcad2c39ae5e954e7e930e498ed3f`;
oracle report SHA-256 is
`1a81025077242fd922719234d3e557d4a6a8aee3e75014f3a60b0e94f30e975b`.
Firefox and WebKit remain explicit missing providers, so their four async/sync
expanded rows stay `not-run` rather than being converted to passes or skips.

The first candidate diagnostic exposed two independent discrepancies. During
attachment, `Runtime.enable` replayed more than 2,048 historical console
events and advanced the caller-driven topology cursor beyond its bounded
window. The final production profile admits 8,192 event items but retains the
prior 512 MiB aggregate payload ceiling independently, plus one maximum-size
ingress overlap; it therefore handles many small replay events without
declaring 2 GiB of retained payload. Once attachment completed, the official
test exposed the actual public gap: Python lacked
`Page.eval_on_selector_all`. The new facade forwards only Frame identity,
selector, function source and argument. Rust still owns selector parsing,
collection resolution, utility-world evaluation, resource admission and
result conversion. `internal:and=<JSON selector>` becomes typed
`Selector::And`; malformed or nested-chain values fail explicitly.
The all-feature profile additionally composes `network-events` with
`concurrent-close` by taking the maximum declared runtime object capacity;
native close no longer replaces the network-owned 1,088-object bound with
its 256-topology-object traversal bound.

Final ABI 30 wheel archive SHA-256 values are
`cbaa50066517cbe4910cb83cebdc8e0bd5434decaa9026c47772654d125a60d5`
and `b79770f9191b444142917f03d3bf8262089dbe223cc2177bdc513596a7a5b8a5`.
Their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed native module SHA-256 is
`78ae030a220444fe7a549333f751acdb87d5ad0718c941c6ec134739bc29f30c`;
it reports ABI 30 and the selected feature list exactly. The installed wheel
passes 29/29 local unit tests, both prior listener regressions pass 1/1, and
the full collection remains 6,526/6,526. Collection report SHA-256 is
`73c16a52febce9c58f1a5b13d5537f9fa16958de095386084a4d0fb56b2c5a12`.

The 6,526-record candidate ledger is now 440 passed, seven skipped and 6,079
`not-run`, SHA-256
`65bd9d4261400421fa079ceea93780052d25c11b19e92a3ad8d2ea25fa75862b`.
The oracle ledger is 421 passed, seven skipped and 6,098 not run, SHA-256
`c4f10b1c972140a3414e48902f610e4a99f8b18384daac925bad89bd9e6db116`.
The remaining rows and the Core JS/TS execution are explicitly incomplete;
this is not a full-suite pass.

## ABI 30 async selector regression checkpoint (2026-09-27)

NODE-249 changes no native ABI or capability graph. It pins the complete
unchanged `tests/async/test_selectors_misc.py` source in
`packages/pwright-python/tests/official/async-selectors-misc-manifest.json` at
SHA-256
`44746147b5adaf40bdd879468b34d9fb2d31b4c64d3a397c9c94a47961b41b0f`
and executes the exact Chromium expansion through the already-installed ABI 30
wheel. Candidate and independent oracle pass 1/1 with all six upstream
assertions unchanged. Report SHA-256 values are
`e6fa55d7b30db92b8d1e2fe3618f75157af502f461711f7df0ed7d7d452f9c4f`
and `c989503c090d5c1e36bcf45f6b4302c6eb051c509f6592c5cf71060ac449bb71`.

The candidate ledger is now 441 passed, seven skipped and 6,078 `not-run`,
SHA-256
`39d6bda545c2f7ce3a21252ee63903b8624732de1fbb29aaaaab6133259d0c86`.
The oracle ledger is 422 passed, seven skipped and 6,097 not run, SHA-256
`318ff5c4cfdb01fdeb833c027fed4ead7765dc32fad7d881f8512a01f78e7be8`.
Firefox/WebKit remain explicit provider gaps. This is a binding regression
checkpoint, not completion of the 6,526-row Python corpus or the 48,477-row
Core JS/TS corpus.

## ABI 31 selector registration checkpoint (2026-09-27)

NODE-250 pins the complete unchanged `tests/async/test_selectors_misc.py`
source in
`packages/pwright-python/tests/official/selector-registration-manifest.json`
at SHA-256
`44746147b5adaf40bdd879468b34d9fb2d31b4c64d3a397c9c94a47961b41b0f`
and selects exactly the Chromium duplicate-registration row. Although the file
imports upstream-private annotation/error types, the unchanged assertion is on
public `playwright.selectors.register`; it is therefore applicable public
behavior rather than an exclusion. The final installed-wheel candidate and
independent oracle pass 1/1. Report SHA-256 values are
`0b39eedf5fb3296af525cefe3c727c2a945153c9f1a03c7b9902b1778140797c`
and `74a1343156cf5890c4bfa73e49c71a71197c4672bf574d7c74c7b4fc546da24d`.

The initial ABI 30 candidate diagnostic fails before the test because
`Playwright.selectors` is absent. The first ABI 31 retry exposes a candidate
shim collection error. Both reports remain preserved outside the green ledger.
The final shim replaces only the oracle-private `Error` annotation identity;
the candidate still raises its own Rust-backed public error.

The independently selectable Rust `SelectorRegistry` owns engine name, source
and `content_script`, admits at most 64 engines, 64-byte names, 1 MiB per
source and 8 MiB aggregate retained bytes, and releases all ownership on
environment stop. Python has no registry. For `path=`, it reads at most the
source ceiling plus one rejection byte before crossing the native boundary.
The leaf reuses the feature-cold 108-package Pwright closure and adds no package
edge. Registering an engine is implemented; executing its JavaScript during
selector resolution is not yet implemented or claimed.

Final ABI 31 wheel archive SHA-256 values are
`8500f8303647d918de19c72763e4095ba69a12117b016173a8a6060eb502aaf9`
and `5ef86dea2cff30a84501d0e47242c496a4375f23fb18aac2b5af607c990f3126`.
Their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed wheel passes 32/32 local unit tests; four release Rust
registry tests, strict Ruff/Pyright, feature-cold/selected checks, exact wheel-
profile Clippy and the dependency proof pass. Full candidate collection remains
6,526/6,526 with XML SHA-256
`c718815c1ab71842546b1b088046088ab6ef51a39466cbe5a227713e66dd2edb`.

The candidate ledger is now 442 passed, seven skipped and 6,077 `not-run`,
SHA-256
`80a4c3855a5a064ef26c467eae02b50b960e93e0faff250fda1693aa8f0a1445`.
The oracle ledger is 423 passed, seven skipped and 6,096 not run, SHA-256
`cddd8fff7571cd00a132fc91ac1c3a722d9bbf4aba1807efc1c55390a788ab7b`.
Firefox/WebKit provider coverage, custom-engine execution, the remaining
Python corpus and the Core JS/TS corpus are explicitly incomplete.

## ABI 32 selector validation checkpoint (2026-09-27)

NODE-251 pins unchanged async/sync `test_queryselector.py` sources through
`selector-registration-errors-manifest.json` and runs exactly the two public
registration-error functions. The first candidate diagnostic is retained with
SHA-256
`099d1dcffe4f85bc2e032a5961e8dcc0a726f3ea5c99ff0b33320d8adc40cec9`:
`neverregister=ignored` bypassed Rust parsing and became a browser CSS
`SyntaxError`, while Playwright reports an unknown selector engine.

ABI 32 exposes synchronous native selector validation over the same immutable,
bounded Rust query recipe used by Locator commands. The Page facade forwards
Frame identity and the selector before its existing handle-evaluation call; it
contains no parser or error-message translation. The final unchanged async and
sync Chromium rows pass 2/2 candidate and 2/2 oracle. Candidate report hashes
are `6f73dd92d3eb989ffb240e14e5c4fdfb3aad7dbcf444b5af7dad20be430fa2f1`
and `309a7f2018905a679456f3f0bdc158e896c719ff0c99a865022038c60b89db93`;
oracle hashes are
`fe4ce4a31d2b9d8f0acf097ce10d19c6a3a443959cb5c71beacdd076d21d1e0f`
and `7405bd869dfb433da3a0d640b001a3bb91fcebd83da26e707e0a98704c21ecf2`.

Final ABI 32 wheel SHA-256 values are
`d2a99b6f90fb87ac0e82310bd1b88f89ea4b6bd8c79089f698b5f86487c38ffc`
and `2f16684d27b517199caad2ad24fb070649b624d6206817f0e22e622ed183a2af`;
their normalized payloads are reproducible. Installed unit tests pass 32/32,
strict Python and Rust gates pass, and full collection is 6,526/6,526 with XML
SHA-256
`8b191388c0d883749adadc61b93a0aefa3aa928c7f781737c1abdcbacdc10c29`.

The candidate ledger is 444 passed, seven skipped and 6,075 `not-run`, SHA-256
`7b3fb579c03cf79a5869613890e06506903142ae8e4627de877e2f8bc4f0d6f3`.
The oracle ledger is 425 passed, seven skipped and 6,094 not run, SHA-256
`89d888ddfe687e446614a35d5a24467e7b3123678242d1191129d93fb0d9b727`.
This closes error validation, not registered-engine execution or the remaining
Python and Core JS/TS suites.

## ABI 33 registered selector-engine checkpoint (2026-09-27)

NODE-252 pins unchanged async/sync `test_queryselector.py` sources in
`selector-custom-engine-manifest.json` and
`selector-custom-engine-worlds-manifest.json`. The first fixes both
`test_selectors_register_should_work` APIs; the second fixes both APIs for
`test_selectors_register_should_work_with_path` and
`test_selectors_register_should_work_in_main_and_isolated_world`. All six
Chromium rows pass unchanged in the final installed-wheel candidate and in a
separate official Playwright oracle.

The `SelectorRegistry` remains the sole owner of engine names, source and
`content_script`. It admits at most 64 engines, 64-byte names, 1 MiB per source
and 8 MiB of accounted name/source payload plus fixed 64-entry structural
overhead. A registered selector becomes an immutable Rust program only if at
least one part names a registered engine; ordinary selectors continue through
the existing parser and keep its unknown/builtin-engine errors. Programs admit
at most 256 parts and 2 MiB of emitted JavaScript, and each distinct referenced
engine source is installed once.

Execution uses `queryAll` for intermediate custom parts, a custom engine's
`query` for the final single lookup and `queryAll` for collection evaluation.
Any engine registered with `content_script=False` requires the main world;
otherwise single/query-probe operations may use the utility world. Collection
evaluation remains in the main world, matching upstream `$$eval` behavior.
Python does not parse selectors, retain source or choose an execution context.

The preserved diagnostics prove three separate discrepancies before the final
implementation: ABI 32 rejected the unimplemented engine; the first ABI 33
single-evaluation attempt accidentally imposed Locator strictness; and the
first path/world attempt used `queryAll` for the final custom part. These
reports are evidence only and are not ledger inputs.

ABI 33 also exports `registered_selector_match_count`. For
`Page.query_selector`, zero matches return `None`; a positive registered match
raises an explicit unsupported error because native ElementHandle publication
is not implemented yet. Typed Locator filters/composition over registered
plans and Playwright-style retry polling remain explicit gaps rather than
Python-side approximations.

Final ABI 33 wheel SHA-256 values are
`0b589f576b783c5e5df854a6bd93777ecb55a0342d1b10872fa2cacda0a465f7`
and `a497d0316731b7d9ab94620ac0ea25fbe960f1b60c039669b3c7a1b38b4a80b2`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. Installed unit tests pass 33/33. Strict Python/Rust gates, feature-cold
and exact wheel profiles and the dependency proof pass. Full collection is
6,526/6,526 with XML SHA-256
`db9363d4fc7ae146c997a17efb15e9123ab5aef94e769a92781d4370a8865d7a`.

The candidate ledger is 450 passed, seven skipped and 6,069 `not-run`, SHA-256
`52fe9d0c64ecf31fb34098e468053265f79eddd68a8cb29dcddab2f75e45c675`.
The oracle ledger is 431 passed, seven skipped and 6,088 not run, SHA-256
`e38018a3c4207b9fdb0995bf8b121d2c455510c49a9440275b2c329cefde8bd3`.
Exactly the six intended rows changed from the ABI 32 checkpoint in each lane.
Firefox/WebKit providers, remaining Python rows and the complete Core JS/TS
execution remain unfinished.

## ABI 33 layout-selector checkpoint (2026-09-27)

NODE-253 keeps the ABI at 33 and adds the optional feature edge
`pwright-python-addon/layout-selectors ->
pwright-node-host/layout-selectors + locator-evaluation`. The feature adds no
Pwright package: the feature-cold dependency proof remains 108 packages.
Python continues to call the ABI-stable internal
`registered_selector_match_count` method, whose implementation now accepts
either a registered-engine program or a layout program. The public facade does
not parse a layout selector, read element geometry or reproduce scoring.

Rust's `LayoutProgram` parser recognizes the official test's top-level
selector chain and the `left-of`, `right-of`, `above`, `below` and `near`
clauses, including optional maximum distance and top-level adjacent-sibling
`+`. Admission is bounded by the existing 64 KiB selector source limit, at
most 256 structural parts and at most 256 KiB of emitted JavaScript. The
compiler emits the upstream rectangle-distance score formulas and stable
score ordering into a one-shot utility-world program. Malformed argument lists
and layout clauses in first position are rejected in Rust before browser
execution.

The exact unchanged async/sync Chromium layout rows pass 2/2 candidate and 2/2
independent oracle. Candidate report SHA-256 values are
`f3d65c5e570917d5a7af2cd795596675128a7375beadcd7d1dfce75293820c09`
and `183b137fa3389396845dbcb72a659e6098e4e933fbb888344556fe926b30338c`;
oracle values are
`1c2747486b1151b39c58190cad9556268e46ecc354d35ee85bf82a4085df31c1`
and `672a2228f53f7725deaf07a0a0fe65eaeba0125926e0363e6a21bc7a61cb3e88`.
The two initial candidate reports that failed at the first
`div:right-of(#id6)` are retained outside the ledger with SHA-256 values
`b4b4df76f54f387bb73ece47c8dd06522ac33ee9c6b5ef38151d840f2d9e938b`
and `73ab134c6f71921d626392b03286595f3e9a523dea33df269d7e8d9f26e537ad`.

Final wheel SHA-256 values are
`d2cab4e3d21671f29a6836811c24aa1c7c48ecdf0bac93cd493c0b7e05d27a33`
and `79e8652366a1bcb82fd4e726715f6cc71a79b9cc0289aaae4c95efd71318f369`;
normalized payloads are reproducible. The installed wheel passes 33/33 unit
tests and explicitly reports `layout-selectors`, 256 parts and 256 KiB through
the native capability handshake. Strict Ruff/Pyright/Clippy, feature-cold
compilation and dependency proof pass. Full collection is 6,526/6,526 with
XML SHA-256
`7d333d7b06713585dc11ffb097354efac1abb5720434f15acc1f6d542f013bed`.
The candidate ledger is 452/7/6067, SHA-256
`295722cd54450259098ff79cd59a5f47cb59926f2d4403f8292d47f481f373df`;
the oracle ledger is 433/7/6086, SHA-256
`263f3100d25d77d1de6b705e6b4cf152ac439857da9f5caa769f1bf055ed6a98`.
The only outcome changes from NODE-252 are the intended async/sync layout rows.

This checkpoint does not claim arbitrary nested CSS/custom-pseudo layout
grammar, Playwright retry polling, positive browser-program
`Page.query_selector` handle publication, Firefox/WebKit providers or full-
suite parity.

## ABI 33 basic Locator facade checkpoint (2026-09-27)

NODE-254 adds no native export, ABI, feature or dependency edge. Async Locator
methods forward selector, public arguments and options to Page's existing
native-backed path; sync methods adapt the same async object through the
bounded loop thread. Basic scalar reads and `all_inner_texts` /
`all_text_contents` use typed Rust `LocatorRead` variants. Fill, check,
uncheck, set-checked, focus and press use existing Rust action owners, while
element evaluation uses the existing bounded locator-evaluation owner. Python
does not author collection JavaScript or retain browser state.

The fixed `locator-basic-manifest.json` selects 34 unchanged async/sync
Chromium invocations. Initial candidate reports preserve all 34 missing-method
failures. An intermediate async run passed 16/17 and exposed the only browser-
semantic discrepancy: `:text("button1")` reached DOM CSS. The bounded Rust
query parser now produces a typed exact text selector for quoted Playwright
`:text()` syntax; an AAA unit test fixes that parse contract.

Final candidate reports pass 17/17 async and 17/17 sync with SHA-256 values
`90699323c687a4a3ad2cc14c163c8ca14bf28846740f9b6193e9e2ffb8f0ddd8`
and `bcb01f7e3d67d98181a8fd9efc00eeb9b308d045921b5e59e0a159b20459fa6c`.
Independent-oracle hashes are
`9b47f044f078e702b9496eec1a601bd724bbb05ff8f4967a0b387d66ce147f92`
and `144d77608332d94d99be317250ef88fe7bfd5aef7cf2e2dfc2c872c8b22545ed`.
Final repaired wheel SHA-256 values are
`ce15aa154b33e075e79b01c52a3de837fb16d5fc6ef78718c35a075b6319dbcd`
and `54582aa165bae4f10567727ad71b6890bb0e3ea96e40dc8e4c5d33a59a74b667`;
their normalized payloads are reproducible. Installed tests pass 34/34 and
full collection is 6,526/6,526 with XML SHA-256
`a961d2252f98f5dfc6037e29d4eb52c1ba169be601d8161fb2761d56e048fc97`.

The candidate ledger is 486/7/6033, SHA-256
`2a956ea3e610f1a8ddad4b92cce0dde918a86aaa3e46a0eb915a5525cc925437`;
the oracle ledger is 467/7/6052, SHA-256
`7a5412b7212fdf7c4e3a6828d411febc65e922dc4be5bfa046210e67e2b3563b`.
Exactly 34 intended rows changed from NODE-253 in each lane. Locator
composition, strictness, remaining actions, non-Chromium providers and full-
suite parity remain explicit gaps.

## ABI 34 Locator execution and count expectation checkpoint (2026-09-28)

NODE-255 adds one native export and no Cargo feature, Pwright package or
dependency edge. `HostLocator::expect_count` performs typed collection-size
reads, compares expected or negated values, owns one total deadline, retries
navigation/context-replacement races, and sleeps for bounded 10 ms intervals.
The async and sync Python assertion facades forward only selector, expected
integer, timeout and negation. Node `_expect('to.have.count')` calls the same
Rust owner and contains no JavaScript polling loop.

The fixed `locator-execution-manifest.json` SHA-256 is
`ed03f8d21b9d27cd400b3a3cb98203a20a35fdc4c7f57aa6e03ec00c92411e09`.
It selects ten unchanged async/sync Chromium invocations for click, click after
node replacement, collection evaluation with present and missing selectors,
and count after main-world `Map` deletion. The initial invalid-channel setup
diagnostic and the following 4/5 placeholder-assertion report are preserved
outside the green ledger.

Final candidate reports pass 5/5 async and 5/5 sync with SHA-256 values
`5e7c4c6caece6cff6db69c80ecbd61c2009fc45f5306257f306cebfcc4d2eab3`
and `7f4e00becab1e465b04f6ce4ef8010d31db720152ad7dd6c06d0253014b5dcf3`.
Independent-oracle hashes are
`8c1646d91536fc53e24cabc5ffc05e96c8530f7103ff9bf281e153898db302ed`
and `d3f183ef9b809b4f6f1cebd9f36597e766b7c9a276213e4066f4864922053001`.
Final repaired wheel SHA-256 values are
`4c159b469a2a0a03efb9da2a89ef3625f4f3d7458163e41f18e7b898a4b2d305`
and `c4120502dbb9ad26dea5e21448eb3340d5ce2a2c68a10644dbd1e98be4b7e6e7`;
their normalized payloads are reproducible. Installed Python tests pass 36/36,
Node adapter tests pass 270/270, and full collection is 6,526/6,526 with XML
SHA-256
`40d9970bf98628c3ca3ff3ec6d369948e5d1693e1fca3b3f396ddfac4f3f874f`.

The candidate ledger is 496/7/6023, SHA-256
`0562df16dd47427a0aa6bd6e6071b833c4a29dc88c5998d8e88aff5a23276649`;
the oracle ledger is 477/7/6042, SHA-256
`14912795ba9cbb4efc8a6c3713506dc039094c497ab0f14806f5cd8ee160ba8b`.
Exactly ten intended rows changed from NODE-254 in each lane. Remaining Locator
composition, strictness, assertions, actionability, non-Chromium providers,
6,023 candidate Python rows and the 48,477-row Core JS/TS inventory remain
explicit gaps.

## ABI 36-41 typed Locator repair checkpoints (2026-09-28)

NODE-261 advances to ABI 36 and derives filter, has/has-not, visible, semantic
and boolean-combination nodes from the immutable native Locator. Each derived
value remains in the original Rust query budget; cross-owner composition is
rejected before browser work. The complete candidate moves from 79/163 to
83/163, exactly two async and two sync gains with no losses.

NODE-262 advances to ABI 37 and keeps Locator state waits, text/property
expectations, regex flags and their single total deadlines in Rust. The
complete candidate moves to 107/163 (58/86 async and 49/77 sync), exactly 24
gains with no losses.

NODE-263 advances to ABI 38. `get_by_role` and `get_by_test_id` create bounded
typed Rust leaves, while `Locator.evaluate` delegates to the existing
`HostLocator::evaluate_with` owner and bounded value conversion. The complete
candidate moves to 110/163 (60/86 async and 50/77 sync), exactly three gains
with no losses. Its cumulative 6,526-row ledger is 559 passed, 53 failed,
seven skipped and 5,907 not run, SHA-256
`d966961d4b507e49bc8f7ead28fa133d490083a0cdf05d77b7b6877110b0462c`.

NODE-264 keeps ABI 38 and adds no FFI export. Playwright `visible=true/false`
selector parts become typed `FilterVisible` nodes rather than invalid CSS.
Selector runtime version 9 includes an element root for user-facing relative
engines, allowing a filtered Locator to continue with `get_by_text` without a
Python selector encoder. The four targeted rows pass, then the complete files
pass 62/86 async plus 52/77 sync: 114/163, exactly four gains and no losses.
Report SHA-256 values are
`f30425320c534d604f3d0112844d43966ba1f77b32f212cf89efe183a88e44e3`
and
`c44938d3feb5ed4938a213e901d065bd3579e01a5bcfce5ee3684b683b861df9`.
The cumulative 6,526-row ledger is 563 passed, 49 failed, seven skipped and
5,907 not run, SHA-256
`281d5cfa976077f38c3c49be2493171edf21e56c41c62b26036b8a44ded1a309`.
The unchanged independent Locator oracle remains 163/163. This is still not a
full Python or Core JS/TS execution claim.

NODE-265 advances to ABI 39. Double-click reuses the typed Rust click options;
path and payload uploads cross bounded Python conversions before Rust owns
filesystem preparation, payload admission and CDP file selection; element
handles use the existing Rust selector-resolution and lifecycle registries.
Both complete Locator files pass 125/163, exactly 11 gains and no losses. The
cumulative ledger is 574 passed, 38 failed, seven skipped and 5,907 not run,
SHA-256
`2356b169e48248f6609fcbd097d9f3205dd87ad2b26ec45a89eb7149a84b7d6e`.

NODE-266 advances to ABI 40 and exposes the existing `HostLocator::highlight`
through a separate `locator-highlight` feature. The feature adds no Pwright
package to the 108-package base closure. Python forwards only the optional
bounded description; Rust owns selector resolution, overlay installation and
replacement. Targeted async/sync highlight rows pass, then the complete files
pass 68/86 async plus 59/77 sync: 127/163, exactly two gains and no losses.

The first NODE-266 async all-row attempt was interrupted by a dedicated Chrome
restart. The base report is preserved; seven setup-affected rows were rerun
exactly and merged with the checked JUnit replacement tool. The repaired async
report SHA-256 is
`e097350da9203f67c3bf88b9fd73620acff9d797ad64bce89538f72f935bd59a`;
the sync report SHA-256 is
`42db7f85e94db0e26386fa3370876a0350d0f90e4d8215a4157967ec86c7a8ae`.
The Locator ledger is 127 passed, 36 failed and 6,363 not run, SHA-256
`074b438d329801fd69ac700e7da53571364b560a55f15af3e75116db430c2b9e`.
The cumulative ledger is 576 passed, 36 failed, seven skipped and 5,907 not
run, SHA-256
`6976549c14fa4000f16617a0226b53753f54580907a5294aa413eea1d40f2cce`.

Strict release Clippy, Ruff and Pyright pass. The current affected Rust host
gate passes 551 tests with 22 explicit live/diagnostic ignores; Python units
pass 58/58 and addon units 2/2. Repaired ABI 40 wheel SHA-256 values are
`7c3a265b9de1825a302ce8bc6d7a26f85c61e212e8605293457f96221b161ff6`
and
`65b5d843a576e78c015d765e6c94774367c0be8b3b26fc5e7793c19abe5aa686`;
their payloads are reproducible after normalizing only CycloneDX identity
fields.

NODE-267 advances to ABI 41. The separately selectable `locator-dom-actions`
feature owns the static event-dispatch and text-selection programs, strict
Locator resolution, bounded event type/initializer ingress and timeout
execution in Rust; Python exposes only async/sync `dispatch_event` and
`select_text` adapters. Its Pwright package closure remains the same 108
packages as the base addon. `Locator.evaluate_handle` is not a second DOM-
action capability: it resolves a temporary element handle in Rust and composes
the existing bounded Handle evaluation/publication owner, including a bounded
Rust-computed public preview.

All six targeted official async/sync rows pass. Complete Locator execution is
71/86 async plus 62/77 sync, 133/163 total, exactly six gains and no losses.
The valid report SHA-256 values are
`7c231d815995b070495dc4f4f30c060b934e500f363e142a4368bf6977c66d2c`
and
`0c5aa8d72106042bfbbea686dde9a55910a2e404ba1d55bdf13b85e19cddbd10`.
The first sync attempt is retained as invalid infrastructure evidence after a
browser closure and is not counted. The complete Locator ledger SHA-256 is
`89154dea59026809606c087f49786fdbf9ba3bb708f9dc78a72349e379108292`;
the cumulative 6,526-row ledger is 582 passed, 30 failed, seven skipped and
5,907 not run, SHA-256
`1feb3bb3ec44812ee930defa5471cd8e8454e2d8c49e9502059273f1c360fe41`.

Strict release Clippy, Ruff and Pyright pass. The affected Rust host gate passes
551 tests with 22 explicit live/diagnostic ignores; installed Python units pass
54/54 and addon units pass 2/2. ABI 41 wheel SHA-256 values are
`b5532c550883212610bf0991c9a555e021f0ef2862c882a5cb9f84c0d70b8665`
and
`ecfb2d1df595cd00a0be48d650c4ed893c9ab0eefa86fb3d76784fd429386d74`;
normalized payloads are reproducible after excluding only the permitted
CycloneDX identity fields. The remaining 30 Locator discrepancies and all
unexecuted Python/Core rows remain open.

NODE-268 keeps ABI 41 and the 108-package base closure unchanged. The Rust
selector recipe now preserves a capture marker so capture-plus-`nth` fails with
Playwright's exact diagnostic instead of reaching CDP. Rust also renders
Playwright-shaped role selectors and bounded `Locator.click` timeout call logs,
preserves Unicode locator text, and admits only the simple relative
`xpath=./TAG` subset needed by `has=` as a typed scoped query. It does not claim
general XPath support.

All eight targeted official async/sync rows pass. Complete Locator execution
is 75/86 async plus 66/77 sync, 141/163 total, exactly eight gains and no
losses. Report SHA-256 values are
`c403694a37a45224c4cca636f0a733ba5713e6480cb45e6c1aa1e0c94e42da95`
and
`8e6c90f179b35c843860d4d9f501c552d27ee606cb104c3e7f1dae2502fa7e64`.
The Locator ledger is 141 passed, 22 failed and 6,363 not run, SHA-256
`cf145b50fa1a45bc018d043058c92d37b783b788f442713a817ba9593f96de56`;
the cumulative 6,526-row ledger is 590 passed, 22 failed, seven skipped and
5,907 not run, SHA-256
`6180ffa49de77b48b2a61d13a8fe3f159654c4e5c4529d841082377112d0ca46`.

Strict release Clippy, Ruff and Pyright pass. The affected Rust host gate
passes 554 tests with 22 explicit live/diagnostic ignores, installed Python
units pass 60/60, selector-model units pass 4/4 and addon units pass 2/2. The
two ABI 41 wheels are normalized-reproducible with SHA-256 values
`0036073c4cce16844e7c5cd35e3d437d472f52e57959741da1e150cba2e3e6d8`
and
`1e905da9fcc741493b93739c2dfe28ad315bd91821c751f34ad3bd7d0f601b69`.
The remaining viewport, FrameLocator and callback-timing discrepancies and all
unexecuted Python/Core rows remain open.

NODE-269 advances to ABI 42 with the independently selectable `page-viewport`
leaf. Rust retains a fixed 64-Page / 8 KiB viewport registry, issues the CDP
window/metrics override, replays it in the existing session-initialization
fence, and composes it with device emulation. Python exposes only thin async and
sync `Page.set_viewport_size` facades. The final normalized-reproducible wheel
SHA-256 values are
`f17b710f3fe0ff90cc9a24a3fc7b4359218214b54462f1e96e2a4d10794ab840`
and
`ba9b0698c6ce62759ccd804ec1f605e0fc8648436918804115acedadbd2e6d74`;
installed units pass 63/63 and strict Rust/Python/addon gates pass. The four
targeted official rows remain red at this node because successful viewport
mutation next reaches missing `Locator.screenshot` or `Locator.bounding_box`.

NODE-271 advances to ABI 44. `HostFrameLocator` retains one immutable iframe
owner Locator and composes nested owners without storing a browser object.
Each command resolves the current frame through a strict temporary
ElementHandle, asks the existing Handle capability for its content Frame, and
disposes the handle before continuing. The existing selector-depth, query,
timeout and resource budgets bound the entire chain. Root Locators passed into
`FrameLocator.locator` are re-rooted as typed Rust recipes, and strict errors
are formatted from those recipes as Playwright Locator call logs.

Python adds only async/sync facade objects, public option/value conversion and
Playwright Python's generic handler-arity rule. The browser binding still
serializes the DOM `Event`; the public Python adapter truncates delivered
arguments to the callback's declared signature. Rust separately owns callback
publication admission and final result/error delivery so an action can observe
that a callback started without making Python own protocol timing.

The final installed wheel passes all 86 async and 77 sync official Chromium
Locator rows. Report SHA-256 values are
`49d736923082374a0d3eab81bea3cbaa539cf364105579856bc513c4cd2dde77`
and
`2cbb448b0651490286df3d7d559211ffac1b012527c42ff9176a3d72f6d07e02`.
The Locator ledger is 163 passed and 6,363 not run, SHA-256
`9b99936aa66ed493f322ab6e44adf7e5bad788780593caf6b46cf49d5ae01a2e`;
the cumulative 6,526-row ledger is 612 passed, seven skipped and 5,907 not run,
SHA-256
`89eabed04a3a7804637229ff077cf0898cc1aa903c190611bfb2d1553650aefe`.
The independent Locator oracle remains 163/163.

Strict release Clippy, Ruff format/lint and Pyright pass. The all-feature host
gate passes 875 tests with 26 explicit live/diagnostic ignores, installed
Python units pass 74/74, page-binding units pass 8/8 and addon units pass 2/2.
The 108-package dependency proof passes. Two normalized-reproducible ABI 44
wheel SHA-256 values are
`bfdbcb48279fbc5377e84dbc2d40b101c186cbb74b25fade4582646db848e100`
and
`ef00f6f69fe0dfed04818d0580e9d2e564c6b75c2e08cb2526f7a7aee1c778af`.
All 5,907 unexecuted Python rows and all 48,477 Core JS/TS invocations remain
open.

## ABI 78 HTTP credentials and response-body checkpoint (2026-10-04)

NODE-322 advances the package/native handshake to ABI 78. Async and sync
ordinary and persistent Context creation expose Playwright's optional
`http_credentials` mapping with username, password and optional origin. Python
validates the public mapping shape and forwards one opaque native policy; it
owns no credentials registry, challenge history, request identity, Fetch
configuration or CDP session.

Rust owns a fixed 16-Context credentials installation. Each of username,
password and optional normalized origin is bounded to 65,536 UTF-8 bytes.
Pending actions, attempted authentications, network/request correlations and
initializing sessions are each fixed at 64 entries. Fetch interception is
installed only for selected sessions and is composed with Page routes without
replacing either feature's patterns. Both features share one caller-driven
reclaiming event cursor. Correct credentials use `ProvideCredentials`; a
second challenge uses `CancelAuth`, with credential fields omitted rather than
serialized as invalid null values; an unmatched origin uses `Default`.

The resource-body official row selects a separate `response-body` feature over
`frame-navigation`. Navigation results expose their Rust-correlated request ID
to the addon, which validates the portable Page/Frame/request route and calls
`Network.getResponseBody` on the authoritative session. The leaf admits eight
operations, bounds both encoded and decoded data to 4 MiB, and charges the
temporary Rust-to-Python copy. Public `Response.body()` retains only the three
bounded identifiers and delegates; no body cache or protocol routing moves
into Python.

The eleven-row manifest SHA-256 is
`154cd2e468af69fd4a8365e46a8959e3fb84f933626dff388c4e64970a241150`.
Final installed candidate/oracle pass 11/11 and candidate stress passes
110/110. Installed-wheel units pass 165/165, the all-feature host library
passes 999 tests with 26 explicit ignores, 22 non-live integration tests and
two addon tests pass, and strict Clippy, Ruff, Pyright plus the 108-package
dependency proof pass. Fresh collection remains 6,526/6,526 and the ledger
advances to 766/0/7/5753, SHA-256
`294ab4a8fee1ea783d5f5b8043a54c52648fe5e9539fdecbd7c42c6000b9c2fa`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`cb7809748be7cc915f6125e973c1149b252b11fcf255c7b528489d7ba93a2de2`,
`7652ef246c7f7007159d663a640e1fa9cc14591465de8a34f2961a035d6b3cc0`,
`ffe3600bd95491851f7a623388780fb744a51c5fdae49322ab43f44040cf1004`,
`023b372456d7759abf411a19d06a0352c6a961ef8a313aacde3da503e6e4cf8b`
and `ec96b3f4af79891a2b2049537ac2650702e83d41ee7ca5539bbb06ce4037ccdb`.
The two normalized-payload-reproducible wheel SHA-256 values are
`473db3f3170c2124418f5bbd17dae19d8c5cc4f933938f1887233ce862c968b4`
and `68ee94ea983be2ed9794b21fbba030299dcc8a3e00c4eac320099f23d1dc98aa`;
their byte-identical native extension SHA-256 is
`f5b18b328da7c6d6a21e2335151da6289eec167fd9a0652a7818579a53662eca`.

## ABI 76 Context geolocation and permission checkpoint (2026-10-04)

NODE-321 advances the package/native handshake to ABI 76. Async and sync
`Browser.new_context` and `launch_persistent_context` expose Playwright's
optional `geolocation` and `permissions` values. Python converts the mapping
to three numbers, supports the separately installed `geolocation` permission,
and owns no Context registry, native ID, CDP session or initialization order.

Rust owns independent fixed 16-Context geolocation and permission
installations. The geolocation initializer sends exact
`Emulation.setGeolocationOverride` to every selected Page/OOPIF session before
publication. Permission uses the browser session and exact
`Browser.grantPermissions`; the default persistent Context omits an ID while
an owned Context uses its authoritative native topology ID. Close and failed
creation release ordinary Context ownership idempotently.

A third leaf forwards bounded persistent Chromium arguments into the existing
provider request: 256 switches, 65,536 UTF-8 bytes per value and 1,048,576
bytes total. Profile, page-opening and remote-debugging switches remain under
Pwright's authority. The conformance runtime's explicit secure-HTTP-fixture
mode adds only the exact dynamic Tailscale fixture origin so the unchanged
upstream localhost-trust test has equivalent secure-context semantics.

The fixed manifest SHA-256 is
`60d3b32cabfcfc13b2d951f891da0321e4d590c06956f40262bc13ce2b35e10e`.
Final installed candidate and independent oracle pass 1/1; candidate stress
passes 10/10. Installed-wheel units pass 161/161, the all-feature host library
passes 992 tests with 26 explicit ignores, 22 non-live integration tests and 2
addon tests pass, and strict Clippy, Ruff, Pyright plus the 108-package
dependency proof pass. Fresh collection remains 6,526/6,526 and the ledger
advances to 755/0/7/5764, SHA-256
`f1288cda0a0873a31ca5a5915a661605f57a42cd316df68f0bd8fc51c155fb7f`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`1a4b619753b5fc589702c5f138cb6f6d19a51ef20e022f3e39c7568fa1b53c39`,
`428ac84f06a7b6f89e8a0aa065c076e31dca56f07545c09c6bf46dfd7d921f47`,
`0397db4f1e7adde6c912cc89cb2bb93d4b1e39d0c67572ae8e14a02a8b4d451a`,
`a6da39178d8fc49d105b87529539098a9500811b85b8116885f9f91e62101882`
and `31366226f4778d8d7a75be3051559b01e0b0b9754b6cd214991e904a629fcffb`.

Two normalized-payload-reproducible wheel SHA-256 values are
`8a56c3f671f288aa7d461f20d1eed9097425531b7772e9f6719fed1ab099fc81`
and `7d8a754afea1a822684c1ffc6daad029fe1f9335a10e21ee6f099502992096aa`;
their byte-identical native extension SHA-256 is
`f4808c9bd38bce42cb670c752d6edea3c39fc19328a81213782648a206758cc1`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 75 Context offline checkpoint (2026-10-04)

NODE-320 advances the package/native handshake to ABI 75 and selects
`context-offline` plus `persistent-context-offline`. Async and sync
`Browser.new_context` and `launch_persistent_context` expose Playwright's
optional `offline` boolean. Python checks the public boolean and forwards it;
it owns no offline registry, Context identity, protocol session or
initialization ordering.

Rust owns a fixed 16-Context `ContextOfflineInstallation`. Explicit owned
Context creation registers the portable identity before browser work and
releases it on compensation or close. Persistent launch stores one immutable
default selection and matches only `CdpContextOwnership::Default`; attached
external Contexts never inherit the option. Every selected Page/OOPIF
initializer sends the exact Playwright Chromium offline
`Network.emulateNetworkConditions` payload before publication.

The ordinary and persistent FFI paths carry one composable Context-policy
value instead of adding unbounded positional parameters. The fixed manifest
SHA-256 is
`00268111cd781614c53806e3f90903151be78e1db662680f547189c6996b065f`.
Final installed candidate and independent oracle pass 1/1; candidate stress
passes 10/10. Installed-wheel units pass 159/159, all-feature host tests pass
983 with 26 explicit ignores, addon tests pass 2/2, and strict Clippy, Ruff,
Pyright plus the 108-package dependency proof pass. Fresh collection remains
6,526/6,526 and the ledger advances to 754/0/7/5765, SHA-256
`c38eea63e7096c771a8faa0912c0b1499158c6a11489dfb4848ac75cc4079592`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`27570a9e7750228f4b60c287573b38d015e0daa944ba6bb1624b7bb0de08cf99`,
`f8e99a025c088eba2042729572fb97ae047be0c6862e528cfdf4aa4b115c7527`,
`61bce407c655347e3308e6c5384a8f4c2edb48248562181a9a26c2daced268de`,
`df4b303b88ec2458834baf4c54489c4773da2e871db87ac7e30a60a41d522bad`
and `d925042c8bcdb55a7592543f2bd0f7814c592902eafa634e2d719f1bb9869b1c`.

Two normalized-payload-reproducible wheel SHA-256 values are
`cdec69d543da109a3071b0b0b52d3e5c35d8344ddcbbdcee2a5931fde18538f9`
and `2be2930d675c736375cebb3506228792c5eac0065c10bde7f5956311e8ad32a4`;
their byte-identical native extension SHA-256 is
`e73bb6233334972f3e58d38370672af658a0c791517862ecb3e50e7c87a123f4`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 74 Context timezone checkpoint (2026-10-04)

NODE-319 advances the package/native handshake to ABI 74 and selects
`context-timezone` plus `persistent-context-timezone`. Async and sync
`Browser.new_context` and `launch_persistent_context` expose Playwright's
optional `timezone_id` string. Python checks the bounded public shape and
forwards it; it owns no timezone registry, Context identity, protocol session
or initialization ordering.

Rust owns a fixed 16-Context `ContextTimezoneInstallation`. Each retained
identifier is nonempty, control-free and at most 256 UTF-8 bytes. Explicit
owned Context creation registers the portable identity before browser work and
releases it on compensation or close. Persistent launch stores one immutable
default selection and matches only `CdpContextOwnership::Default`; attached
external Contexts never inherit the option.

Every selected Page/OOPIF initializer sends exact
`Emulation.setTimezoneOverride {"timezoneId": ...}` before publication. Only
Chromium's shared-renderer duplicate response is idempotent. Invalid timezone
identifiers become stable `InvalidArgument` errors and unrelated protocol
failures propagate. The ordinary Context path now accepts one public Rust
`ContextPolicyOptions` builder, so future independently selected policies can
compose without extending an unbounded positional FFI-adjacent host call.

The fixed manifest SHA-256 is
`68c4d90a61e471f7d904c27b1831bbc65280f80fd91c645c6172fd9c00fa25dc`.
Final installed candidate and independent oracle pass 1/1; candidate stress
passes 10/10. Installed-wheel units pass 157/157, all-feature host tests pass
978 with 26 explicit ignores, addon tests pass 2/2, and strict Clippy, Ruff,
Pyright plus the 108-package dependency proof pass. Fresh collection remains
6,526/6,526 and the ledger advances to 753/0/7/5766, SHA-256
`a48b4d1c0e8bdce904b95194393ee47709c01313c121843e81e1410f524b27fc`.

Candidate, oracle, stress, installed-unit and collection report SHA-256 values
are respectively
`25b4741916039a3841c6e0416875befd52b46c06eafae057eb4fdcd6b4707e39`,
`fec89b423b6582adf9e7754e980500e4b6266c17c722e0043eeaf0e0c4093c73`,
`1a3674e68420813a531778093500877aac49868cea700c60d9592ae69a716c9d`,
`1ea73cebd5b5a44578cd2af4816d8697e5f6381461bc55884cfdaa7304568d9f`
and `46ef609d96e1cdd9be58f41f7d3878f2d4b6da63928cbcd03d55e04895b54420`.

Two normalized-payload-reproducible wheel SHA-256 values are
`a8b4ae9a06dde9cf9d7f766cb7d326ca3b0e69d773c23b9fdbb14d8c19519632`
and `377d5981d1d76bf121bf0db45222a0b82e9610bb9f6e9b638f075a67f452f49f`;
their byte-identical native extension SHA-256 is
`8b1dd78a036c331ae027e2c1cbb49caa51008ead5b9d4e022c729607f8ba6d02`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 73 JavaScript-disabled Context checkpoint (2026-10-04)

NODE-318 advances the package/native handshake to ABI 73 and selects
`javascript-disabled` plus `persistent-context-javascript-disabled`. Async and
sync `Browser.new_context` and `launch_persistent_context` expose Playwright's
`java_script_enabled` boolean. Python validates that exact public shape and
forwards it; it owns no Context policy, CDP session or initialization timing.

Rust owns a fixed 16-Context `JavascriptDisabledInstallation`. Explicit owned
Context creation records the portable Context identity before browser work and
removes it on compensation or close. Persistent launch stores one immutable
default choice and selects only the exact default Context; attached external
Contexts never inherit the option. Every selected Page/OOPIF initializer sends
`Emulation.setScriptExecutionDisabled {"value": true}` before publication.

The unchanged click test requires navigation waiting that was not previously
on the Python surface. ABI 73 therefore adds one thin native method plus async
and sync `Page.expect_navigation` context managers over the existing bounded
Rust frame-navigation operation. `__aenter__`/`__enter__` arms the wait before
the action; completion adapts the existing optional Response and refreshes the
Page URL. URL predicates are rejected before native work instead of being
silently ignored.

That same row exposed a real actionability discrepancy for a transformed link
wrapped across two line fragments: the union-box center hit its parent `DIV`.
Default Locator click now clips `getClientRects()` to the viewport and uses the
first nonempty fragment center. Explicit positions keep the existing border-
box coordinates. A separate attach failure came from `Runtime.enable` replaying
main-world console RemoteObjects before utility-world creation; the exact
main-world route now stages them while the paired-world native-retirement proof
remains optional, rather than failing the whole topology.

The fixed manifest SHA-256 is
`7f2e326e8c7d5ac9623279fac00277468ee9ae25394f11a10b6ac7d8d90641c4`.
Final installed candidate and independent oracle pass 4/4; candidate stress
passes 40/40. Installed-wheel units pass 157/157, all-feature host tests pass
974 with 26 explicit ignores, addon tests pass 2/2, and strict Clippy, Ruff,
Pyright plus the 108-package dependency proof pass. Fresh collection remains
6,526/6,526 and the ledger advances to 752/0/7/5767, SHA-256
`93f22527ff4e8f89d95a525a17393d3d5d3463432e6e9e75f7135adcea7a81a3`.

Two normalized-payload-reproducible wheel SHA-256 values are
`7c2e48a4705d597aba896763e50b67ddfd6d8d03c562312c5dcd3f24ba687859`
and `89b60d08b5e5691aeb4de049a2d10a74f0a42947f42fdf57c3693d0eabd1ab9a`;
their byte-identical native extension SHA-256 is
`26e341524d4f6bf5703089a1db57343cc885e8c243ed2bfc6448b72dfd9a41a0`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 72 Context CSP bypass checkpoint (2026-10-04)

NODE-317 advances the package/native handshake to ABI 72 and selects
`bypass-csp` plus `persistent-context-bypass-csp`. Async and sync
`Browser.new_context` and `launch_persistent_context` expose one boolean each.
Python validates the exact public boolean shape and delegates; no CSP policy,
session identity or browser command remains in Python.

Rust owns one fixed 16-Context `BypassCspInstallation`. Explicit owned Context
creation admits the portable Context identity before browser work and removes
it on every compensation or close path. Persistent launch stores one immutable
default bit in `PersistentContextDefaults`, consuming one slot only when
selected. Attached external Contexts are never selected. The normal Page/OOPIF
initializer sends exact `Page.setBypassCSP {"enabled": true}` before session
publication, including after cross-process navigation.

The fixed manifest SHA-256 is
`6748a04a6e0e1f0340d7e7c6e14ef6ba5ddd4e0e91361dd3fe2feb9f57fc4c64`.
Final installed candidate and independent oracle pass 4/4; candidate stress
passes 40/40. Installed-wheel units pass 152/152, fresh full collection is
6,526/6,526 and the ledger advances to 748/0/7/5771, SHA-256
`713d4047df50050430e4bfec6c843b171806653db19e8c0df6da6e66cd942508`.
Two normalized-payload-reproducible wheel SHA-256 values are
`44a9ec5ee2be17cab8036084dfa46c63fc717904692175e6fa66bafd8ef21f70`
and `319a9d9c11cf43d55a1b34eb498b9e24dcc6048a5e4ca43931a880a76c5ca7eb`;
their byte-identical native extension SHA-256 is
`0a2ce2ad114c52ff3cb51328557386db291e922f6684ffa8b2d50133b8fe721d`.
The wheels remain Mac-local while Homebrew OpenSSL remains an external runtime
assumption. The iframe CSP row is not claimed because the unchanged test first
requires the independent `ElementHandle.content_frame` capability.

## ABI 71 persistent-Context extra-HTTP-headers checkpoint (2026-10-04)

NODE-316 advances the package/native handshake to ABI 71 and selects
`persistent-context-extra-headers`, a feature composition over the existing
persistent launcher and `extra-headers` owner. The ABI launch tuple adds one
optional list of string pairs. Python admits at most 256 headers / 64 KiB and
the native boundary keeps a fixed 512 KiB call allowance. Native code converts
once to `PersistentContextExtraHeaders` and moves the values into the existing
`HeaderPool`.

Before initial Page publication, the first exact default-Context session
atomically commits the launch seed into the existing `HeaderSettings` registry
and applies it through the normal Page/OOPIF initializer. Owned and external
Contexts do not inherit it. Explicit later Context/Page sets and clears,
failure compensation and session retirement continue through the same bounded
pool, registry and job owners; there is no second state store or Python mirror.
The default consumes one existing 16-Context header slot.

The fixed manifest SHA-256 is
`5345d32db44ce2f21a3c8ebc6fec811a498c695f68972b66564a39677d2d1041`.
Final installed candidate and independent oracle pass 1/1; candidate stress
passes 10/10. Installed-wheel units pass 150/150, fresh full collection is
6,526/6,526 and the ledger advances to 744/0/7/5775, SHA-256
`19d1e6e8365cf3c56b86e96edc76d10a6a619b797f967caedb477cacfbd69411`.
Two normalized-payload-reproducible wheel SHA-256 values are
`f954e83ec2343be3b52fe4ad0f8a3393c343211f4779bea8f6a3d5a7857ec7e2`
and `1f0a64b3d822ca9b018b7ba64c94d8852f53aaf0de6069f33ab4ef1bb2960ebc`;
their byte-identical native extension SHA-256 is
`a5b9fcb1d52d72986a017de082b0ee342fa57b5e64b413e52827c6d0c22392d2`.
The wheels remain Mac-local until their external Homebrew OpenSSL dylibs are
repaired and reverified.

## ABI 70 persistent-Context media-defaults checkpoint (2026-10-04)

NODE-315 advances the package/native handshake to ABI 70 and selects
`persistent-context-emulate-media`, a feature composition over the existing
persistent launcher and `emulate-media` owner. The ABI launch tuple adds one
optional three-string media tuple. Native code bounds and converts it to
`PersistentContextMediaOptions`; the Rust host retains typed enums rather than
public strings.

The default is admitted before the initial topology is initialized. The
existing media initializer resolves exact Context ownership and applies it only
to `CdpContextOwnership::Default`, then replays the current stored state for
every Page/OOPIF session before publication. Owned and external Contexts do not
inherit it. Initial replay and `Page.emulate_media` share the existing async
gate and 64-Page state table; a browser receipt precedes every state commit.

Async and sync `launch_persistent_context` expose only `color_scheme`,
`reduced_motion` and `contrast`. Python validates public enum spellings before
native work and delegates sync to the same async path. It retains no media
mirror, session identity, initializer, browser process or cleanup owner.

The fixed manifest SHA-256 is
`a2da1fb06d4bd6039f7255d1f95e92f853ef5260b491f11fa1236a19f2c956f9`.
Final installed candidate and independent oracle pass 3/3; candidate stress
passes 30/30. Installed-wheel units pass 149/149, fresh full collection is
6,526/6,526 and the ledger advances to 743/0/7/5776, SHA-256
`8a7fe5975b0bbbf792cc8c13a0b8e81a11a70bb8f17dcb0e8f439abe30e67efb`.
Two normalized-payload-reproducible wheel SHA-256 values are
`3b7ddb9ea476955dcf442a357846a9856ef1e625f304c435ac970685f1a737a8`
and `e99fc988705d6daafce55e60aa930b49f7964aa3902fb200307b3111cd05f353`;
their byte-identical native extension SHA-256 is
`6a44342c2790626304fa7483f5537d2f611223a22068bbb667149c48c5654c53`.
The wheels remain Mac-local until their external Homebrew OpenSSL dylibs are
repaired and reverified.

## ABI 69 persistent-Context launch-argument preflight checkpoint (2026-10-04)

NODE-314 adds an explicit async/sync `args: Sequence[str] | None` parameter to
`BrowserType.launch_persistent_context` while keeping ABI 69. The adapter
validates at most 256 strings, 64 KiB each and 1 MiB total. It applies the same
ordered Chromium ownership checks as upstream for `--user-data-dir`,
`--remote-debugging-pipe` and bare arguments that would open a page. The
selected official error therefore occurs before environment access or native
launch.

An empty sequence is semantically absent. Non-empty switch-only sequences
still raise `UnsupportedError`; they are not silently ignored and are not
forwarded to Rust. Full custom-argument support requires a future selectable
Rust launch provider that owns process construction and argument conflicts.
This checkpoint creates no FFI value, native export, session, process, task,
timer, retained byte or dependency edge.

The unchanged official row passes candidate/oracle 1/1 and candidate stress
10/10. Installed-wheel units pass 148/148, Ruff and strict Pyright pass, and
fresh collection remains 6,526/6,526. The cumulative ledger is
740/0/7/5779. Two normalized-payload-reproducible wheel SHA-256 values are
`d17207c54c374c072df4d752df38959ecd376c5918dcc215fe3b46a22cb93b07`
and `76bfdf8d21cb28d268139106d225cf1d989ae53e629eb0805bfd9278efc0b052`;
native SHA-256 remains
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.

## ABI 69 BrowserContext cookie checkpoint (2026-10-04)

NODE-313 advances the package/native handshake to ABI 69 and selects the
optional `context-cookies` host leaf. The native boundary adds exactly three
session methods: `context_cookies`, `context_add_cookies` and
`context_clear_cookies`. Async and sync BrowserContext facades expose the
corresponding Playwright Python methods and public `Cookie` / `SetCookieParam`
shapes. They validate and copy bounded values only; neither facade retains a
cookie cache or protocol identity.

The Rust host maps an exact public Context ID to `CdpContextOwnership`, then
uses a browser-root `CdpSession`. Default Context requests omit a CDP browser
context ID and owned Context requests pass the exact native ID. Reads, writes
and full clear are direct Storage-domain commands, independent of Page
topology. Filtered clear reads a bounded authoritative snapshot, matches in
Rust and writes bounded expired cookies. This preserves Chromium's private
partition ancestor bit while Python receives only Playwright's public
top-level partition site.

Input conversion follows upstream truthiness and normalization: a non-empty
URL is mutually exclusive with domain/path and derives domain, directory path
and secure defaults; an empty URL uses the supplied domain/path pair. Blank
and data URLs are rejected. Cookie URL filtering uses upstream domain, path,
secure and localhost behavior. Empty exact clear matchers match every value;
regex flags and sources are admitted into bounded Rust matchers.

Bounds are enforced twice where allocation crosses languages: 1,024 cookies,
16 KiB ordinary fields, 4 MiB cookie batches/results, 256 filter URLs, 64 KiB
per URL, 1 MiB URL batches, 16 KiB regex sources and one 10 MiB FFI allowance.
The leaf adds no persistent cookie store, Page lane, task, timer, transport,
browser process or dependency edge. The dependency proof remains 108 Pwright
packages.

Release validation passes 130 CDP tests, 962 all-feature host tests with 26
explicit ignores, 2 addon tests and 146 installed-wheel units, plus strict
Clippy, Ruff and Pyright. The fixed official manifest has exact
candidate/oracle parity: two applicable passes and two identical hardcoded-
`localhost` fixture gaps. The formal ledger advances only the passes to
739/0/7/5780. Two normalized-payload-reproducible wheel archives have SHA-256
values
`174a30499a3c21c746d4ed5ffb5461247afe012bc9207c32834c68c0801382d4`
and `faa8f553d1643ed4594997acf1fd06dd75f730c3981cbf99fe12d3dda0d69c58`;
native SHA-256 is
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
The remaining OpenSSL dylib repair is packaging work, not cookie behavior.

## ABI 68 BrowserContext event-facade checkpoint (2026-10-04)

NODE-312 keeps ABI 68 and adds no native export. Async and sync
`BrowserContext.on`, `once` and `remove_listener` adapt the existing Rust-owned
Page/Context lifecycle publication into Playwright-compatible `page` and
`close` callbacks; `console` is exposed only when its independent feature is
selected. Python retains callback identity but no topology, protocol session,
payload owner or second watcher. The sync facade maps Page, ConsoleMessage and
the closing Context to their exact existing sync wrapper identities and uses
the bridge's existing callback-error handoff.

Context console listeners reuse the already bounded Rust subscription owner.
The first listener acquires one token and the last removal releases that token;
no new queue, task, timer, command lane, session registry, byte allowance,
transport or dependency is introduced. The official persistent-close event is
therefore still caused only by Rust's normal Context-close notice and is
delivered before the existing close fence completes.

All 141 installed-wheel units, formatting, Ruff, strict Pyright and the
108-package dependency proof pass. Two final ABI3 wheels are normalized-
reproducible, with SHA-256 values
`2487ed3dbbac1e4fa7047e2cfbf7b3c6e5f0978b94eb972db27b9f6f60a4acf3`
and `21d9631c7e361584cad656a7eb5332d9332cdd86db7fb1314c08fae8f26e778b`.
Their native extensions are byte-identical, SHA-256
`890e6b4d7023d0ee13ff1a4bfa1b823416297a5ad273dbc5c1d84c2ceabc3f17`.

The unchanged official async Chromium row passes 1/1 in the final installed
candidate and independent Playwright 1.58.0 oracle; candidate same-process
stress passes 10/10. The prior missing-API result and two invalid repeat
attempts remain diagnostic and are excluded from the ledger. Fresh collection
remains 6,526/6,526. The cumulative ledger advances to 737 passed, zero failed,
seven skipped and 5,782 not run, SHA-256
`d405874b563e49f99ece20095fffd86e4ff6bb74ebdad303049ae9a05ea80f65`.
Exact comparison with NODE-311 changes only the persistent-close Chromium row
from not run to passed and has no losses. The remaining 16 Chromium rows from
the original default-BrowserContext diagnostic, the rest of Python and all
Core JS/TS corpus work remain open.

## ABI 68 persistent-Context device-emulation checkpoint (2026-10-04)

NODE-311 advances the package/native handshake to ABI 68. The independently
selectable `persistent-context-device-emulation` feature composes
`persistent-context` and `device-emulation`, transferring one validated
device-policy value into host construction before the initial Page topology
snapshot. It reuses the existing device installation, user-agent composition
and Page/OOPIF initializer rather than adding a second emulation path.

The host groups optional persistent locale, HTTPS and device selections in one
`PersistentContextDefaults` composition value. Device lookup distinguishes
topology ownership: the immutable default selection applies only to
`CdpContextOwnership::Default`; later `Owned` Contexts still require their
existing registry entry, and attached `External` Contexts never inherit it.
The default selection consumes one of the existing 16 policy slots and its
actual bytes from the existing 64 KiB budget, leaving at most 15 later selected
owned Contexts. No task, queue, timer, session registry, transport, process
allowance or dependency is added. Dependency proof confirms that the new leaf
has exactly the persistent-launch Pwright package closure.

The persistent-launch tuple is now `(base_url, locale, ignore_https_errors,
device, executable_path, channel, timeout_ms)`. Async and sync
`BrowserType.launch_persistent_context` validate and forward viewport, screen,
user agent, scale, mobile and touch values. The public `Page.viewport_size`
facades retain only the two-integer policy snapshot and update it after a
successful Rust Page resize; Rust remains authoritative for CDP commands,
Page/OOPIF replay, resource admission and cleanup.

The final gates pass 954 all-feature host tests with 26 explicit
live/diagnostic ignores, 2/2 addon tests, 137 installed-wheel units,
selected/all-feature release Clippy, formatting, Ruff, strict Pyright and the
108-package dependency proof. Two final ABI3 wheels are normalized-
reproducible, with SHA-256 values
`a1ca0c2bb6d80a0e9ac6fadfa3e778077acc3f42cd6ef9d3e0a55101ba798059`
and `654386396048318f4fb8fdfbf6839da2b8c7ee69be19b74a921569370a30987f`.
Their native extensions are byte-identical, SHA-256
`c4d78c799f17dc1955514593fe8132173bf48c273d14a1a2ac4a74cd36703fe0`.

The five unchanged official async Chromium rows pass 5/5 in the final
installed candidate and independent Playwright 1.58.0 oracle; candidate same-
process stress passes 50/50. The original ABI 67 option rejection, the first
candidate missing-property result and the oracle attempt without installed
Chrome selection remain diagnostic and are excluded from the ledger. Fresh
collection remains 6,526/6,526. The cumulative ledger advances to 736 passed,
zero failed, seven skipped and 5,783 not run, SHA-256
`d866df9d52c72da0a9b8548c5ddbf5b0daed9010e9adc769f33a64bdc69aa4c3`.
Exact comparison with NODE-310 changes only the selected five Chromium rows
from not run to passed and has no losses. The remaining 17 Chromium rows from
the original default-BrowserContext diagnostic, the rest of Python and all
Core JS/TS corpus work remain open.

## ABI 67 persistent-Context `ignore_https_errors` checkpoint (2026-10-04)

NODE-310 advances the package/native handshake to ABI 67. The independently
selectable `persistent-context-ignore-https-errors` feature composes
`persistent-context` and `ignore-https-errors`, transferring the public boolean
into host construction before the initial Page topology snapshot. It reuses
the existing Rust policy installation and Chromium
`Security.setIgnoreCertificateErrors` initializer rather than adding a second
HTTPS path.

The installation now distinguishes topology ownership. The immutable default
selection applies only to `CdpContextOwnership::Default`; later `Owned`
Contexts still require their existing explicit registry entry, and attached
`External` Contexts never inherit it. The default selection consumes one of
the existing 16 policy slots, leaving at most 15 later selected owned Contexts.
No task, queue, timer, session registry, transport, process allowance or
dependency is added. Dependency proof confirms that the new leaf has exactly
the persistent-launch Pwright package closure.

The persistent-launch tuple is now `(base_url, locale, ignore_https_errors,
executable_path, channel, timeout_ms)`. Async and sync
`BrowserType.launch_persistent_context` validate and forward the boolean;
Python retains no native policy state. Four focused Rust regressions, 952 all-
feature host tests with 26 explicit live/diagnostic ignores, 2/2 addon tests,
136 installed-wheel units, selected/all-feature release Clippy, formatting,
Ruff, strict Pyright and the 108-package dependency proof pass.

Two final ABI3 wheels are normalized-reproducible, with SHA-256 values
`c153464a16a2083899e44a3637d48d2cf512a9d669675feed27a244df2f65880`
and `f78552d51c1ecc9c2cac8a8b8193896663c71213d487a57903c8d45c184f1387`.
Their native extensions are byte-identical, SHA-256
`7f32e973622c1b7e1edbd1465637bebbc94af840972bf5457ee13df500c265eb`.

The unchanged official async Chromium row passes 1/1 in the final installed
candidate and independent Playwright 1.58.0 oracle; candidate same-process
stress passes 10/10. The original ABI 66 rejection remains diagnostic and is
excluded from the ledger. Fresh collection remains 6,526/6,526. The cumulative
ledger advances to 731 passed, zero failed, seven skipped and 5,788 not run,
SHA-256
`9100fc0c035aefbb4326bf7a3be6a8a037ba9c332254b12592e2eb75aef4e8d1`.
Exact comparison with NODE-309 changes only the selected Chromium row from not
run to passed and has no losses. The remaining 22 Chromium rows from the
original default-BrowserContext diagnostic, the rest of Python and all Core
JS/TS corpus work remain open.

## ABI 66 persistent-session release checkpoint (2026-10-04)

NODE-309 keeps the package/native handshake at ABI 66. It changes Python
lifetime routing only: the Browser constructed behind
`launch_persistent_context` is marked as the unique owner of its default
Context, while a Browser constructed by `connect_over_cdp` remains shared.

Persistent `BrowserContext.close()` first executes the existing Context close,
then calls the existing Browser/native close. The Rust HostSession is joined
and removed from the Python environment's fixed 16-entry session vector. On
successful Browser close, the language Browser is removed from
`Playwright._browsers`, releasing its closed Context/Page graph. Normal owned-
Context close does not close the shared Browser. The sync facade reuses this
same async path, so there is no duplicate policy or cleanup implementation.

The first same-process ten-repeat exposed the bug: five persistent launches per
repeat accumulated because Context close never reached native Browser close.
The 17th admission failed with the explicit capacity error and the run finished
10/30 passed. The repaired final wheel completes all 30 testcase invocations
and 50 launches. Three new AAA ownership regressions and all 135 installed-
wheel units pass. All-feature addon release Clippy, formatting, the 108-package
dependency proof, Ruff and strict Pyright pass.

Two final ABI3 wheels are normalized-reproducible, with SHA-256 values
`1bdd9b424f21d52def2b82901df4b4069e8c605e946b4476aa8312f8519120ba`
and `58cc07b09356c442130238faf876336164964e9d6ff17429dcdf8024b66ae8d0`.
Their unchanged native extensions are byte-identical, SHA-256
`45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.

Final installed candidate and independent oracle each pass the three unchanged
persistent-profile baseline rows 3/3; the final candidate repeat passes 30/30.
Fresh collection remains 6,526/6,526. The cumulative ledger advances to 730
passed, zero failed, seven skipped and 5,789 not run, SHA-256
`271e88d71232b76b93e28d186739e1c4bb701008526da99e0bf833844fe7b761`.
Exact comparison with NODE-308 changes only those three rows from not run to
passed and has no losses. The remaining 23 Chromium rows from the diagnostic,
the rest of Python and all Core JS/TS corpus work remain open.

## ABI 66 `wait_for_function` timeout acknowledgment checkpoint (2026-10-04)

NODE-308 keeps the package/native handshake at ABI 66 because timeout
acknowledgment is an internal Rust completion-order repair. It adds no native
symbol, feature, public option or Python owner. Async and sync Python facades
continue to call the same native `wait_for_function` path.

The outer deadline now owns a cancellation token passed into the existing poll
operation. When the deadline fires, it cancels polling and awaits that operation
only until the current already-admitted predicate command and its ordinary
publication fence acknowledge cancellation. Checks immediately after Handle
creation, after truthiness completion and inside the interval delay prevent a
new attempt. If Handle creation completes after cancellation, Rust requests
normal disposal instead of publishing it. Stop or transport loss continues to
return authoritative `TargetClosed`; the deadline result remains `Timeout`.

This closes a real lifetime gap. The first installed-candidate ten-repeat run
passed 58/60, but two post-timeout-side-effect rows observed one extra
`console.log`: dropping an in-flight controlled dispatch preserved eventual
native cleanup but skipped the caller's normal publication fence. The final
wheel passes the six unchanged official rows 6/6 and the ten-repeat 60/60; the
independent Playwright 1.58.0 oracle passes 6/6. The first red report remains
diagnostic and is not merged into the ledger.

Eight focused Rust regressions, 950 all-feature host tests with 26 explicit
live/diagnostic ignores, 2/2 addon tests, formatting, strict selected/all-
feature release Clippy, the 108-package dependency proof, Ruff, strict Pyright
and 132/132 installed-wheel Python units pass. Two final ABI3 wheels are
normalized-reproducible, with SHA-256 values
`752e68fd89b54c8dd4cb5a5552ff6c847cf46736fb4d9c70465e9c5b58e57320`
and `a1827bb7227124bb298d94d3b885dc317d7dc16c61fce7aa9560246c5f0a0f86`.
Their native extensions are byte-identical, SHA-256
`45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.

Fresh collection remains 6,526/6,526. The cumulative ledger advances to 727
passed, zero failed, seven skipped and 5,792 not run, SHA-256
`96c068d07bcc0ccce5bb2cea05c5721349f982aa604cd4c2b48488487a83eb2f`.
Exact comparison with NODE-307 changes only the six selected Chromium rows
from not run to passed and has no losses. The remaining Python and Core JS/TS
corpora remain open.

## ABI 66 persistent-Context locale checkpoint (2026-10-04)

NODE-307 advances the package/native handshake to ABI 66. The independently
selectable `persistent-context-locale` feature composes `persistent-context`
and `context-locale`, passing the optional locale into host construction before
the initial topology snapshot. The existing default Context receives the same
bounded policy as later owned Contexts; attached external Contexts do not.
Dependency proof shows no Pwright package beyond the persistent launch closure.

The ABI persistent-launch options tuple is now `(base_url, locale,
executable_path, channel, timeout_ms)`. Python's async and sync
`BrowserType.launch_persistent_context` facades forward that tuple and retain no
locale, process, CDP session or recovery task. Rust validates nonempty,
control-free values at 256 UTF-8 bytes, counts the default entry against the
existing 16-Context registry and releases it with host shutdown.

Each selected Page/OOPIF initialization sends the upstream Chromium pair. A
stateless shared initializer projects the optional device and locale policies
and emits exactly one `Emulation.setUserAgentOverride` when either exists. It
preserves a selected device `userAgent` while adding `acceptLanguage`; without
a device UA it uses Chromium's empty-UA sentinel. The locale initializer then
sends `Emulation.setLocaleOverride`. The former determines
`navigator.language`; the latter determines ICU/`Intl`. Only the exact shared-
renderer duplicate-locale error is idempotent. A real provisional failure that
returned `en-US` identified the missing user-agent language override; the final
wheel passes both the unchanged official row and a combined device-UA-plus-
locale real-Chrome probe.

Seven locale regressions, four user-agent composition regressions, 948 all-
feature host tests with 26 explicit live/diagnostic ignores, 2/2 addon tests,
formatting, strict selected/all-feature release Clippy, the 108-package
dependency proof, Ruff and strict Pyright pass. Two final repaired ABI3 wheels
are normalized-reproducible, with
SHA-256 values
`d05505434a588149a878a1d06657cbfbecd4ff9046ad6cf79787e26fd1c5f870`
and `e3cbc6a377ce20492652baeb8f46ac49f79088dced638bcd8a8bddf40ebabb79`.
Installed-wheel units pass 132/132.

The final candidate and independent oracle each pass the unchanged Chromium
persistent-locale row 1/1; the final candidate repeats it 10/10 and keeps the
Context-locale Worker regression green. Fresh collection is 6,526/6,526. The
cumulative ledger is 721 passed, zero failed, seven skipped and 5,798 not run,
SHA-256
`352660d65526e5c2cbabfdf3d7115eb32e3e75ed5dc484b9f18efee66914f053`.
Exact NODE-306 comparison changes only this Chromium row from not run to passed
and has no losses. Remaining Python and Core JS/TS corpus work stays open.

## ABI 65 Context locale checkpoint (2026-10-04)

NODE-306 advances the package/native handshake to ABI 65. The independently
selectable `context-locale` feature validates and retains one locale per owned
Context. The selected package exposes only the existing async/sync
`Browser.new_context` option; Python stores no duplicate locale registry and
sends no CDP command.

Rust caps the registry at 16 live Contexts and each locale at 256 UTF-8 bytes.
Empty strings and control characters are rejected, duplicate installation
fails, and Context close removes the policy. Page and OOPIF
sessions receive `Emulation.setLocaleOverride` after downstream session setup
and before publication. Because Pages in one renderer share locale, only the
exact upstream `Another locale override is already in effect` protocol error
is treated as idempotent; every other CDP failure propagates. Dedicated and
nested Workers inherit renderer locale and require no locale command, session
registry, initialization hook or separate feature.

Five locale ownership/bounds/error-classification/payload regressions pass.
The all-feature host passes 942 tests with 26 explicit live/diagnostic ignores;
the addon passes 2/2. Formatting, strict selected/all-feature release Clippy,
the 108-package dependency proof, Ruff and strict Pyright pass. Two repaired
ABI3 wheels are normalized-reproducible; their SHA-256 values are
`5748b3dd7310752b8bac8a7a39588b06e706f104f647cc1ac6be30aec3da24b1`
and `7865ee31d2dbdebcabbf18b9e51dc9fd47dcf45e8968a9c5e1f3743785200abb`.
Installed-wheel units pass 131/131, report SHA-256
`293ddb1ba023c74ef2e3682c0632de12e2138488626ab8506a491a5e7069987f`.
Every final-wheel runtime member is byte-identical to the browser-tested
corrected wheel, including native extension SHA-256
`3cfc824663b084756e8c4db9b3a34b07222947768eb26d26e813796b084108ee`;
only metadata, RECORD and CycloneDX identity changed with the final README.

The unchanged official Context-locale Worker row passes installed candidate
and independent oracle, report SHA-256 values
`adc3bef79f6fcf3a7d74525f9b5e22fccf627218c13d7ae7c255aa9758aa78b5`
and `2e7cc0bf2da134da5fc3a6666f77e52dfe425626407e19f627e84647ffc897a1`.
The candidate repeat passes 10/10, SHA-256
`42662339aad777d1c042dfd7880325c5a59a20778443bdab2f5008b84bf9ad4f`,
and the complete unchanged Worker file passes 13/13, SHA-256
`523fa108aaa13c1f7ff529fe50b26c2f6bae46d924ef83fd7ff4c963566ca278`.
Fresh collection is 6,526/6,526. The cumulative ledger is 720 passed, zero
failed, seven skipped and 5,799 not run, SHA-256
`20e36211f8f1bc6027c7c6998aa7060cb9d43262eadcaf7eb60561bfde4a66f3`,
with exactly one NODE-305 not-run row becoming passed and no losses. Earlier
provisional wheels and red duplicate-override attempts remain diagnostic only.

## ABI 64 Worker console argument Handle checkpoint (2026-10-04)

NODE-305 advances the package/native handshake to ABI 64. The independently
selectable `worker-console-argument-handles` feature composes
`worker-console-events` and `worker-handle-evaluation`; executable dependency
proof shows that it adds no Pwright package beyond the latter feature's
closure. Python accepts the previous 11-field notice for unselected builds and
the selected 12-field notice that appends ordered Handle descriptors.

The native event owner arms RemoteObject recovery before decoding the first
argument. It enforces the existing maximum of 16 console arguments and
4,096-byte object ids, validates every RemoteObject and pre-admits all resource
leases before publishing any Handle. One state-lock transfer then installs the
complete ordered batch into the existing 64-entry Worker Handle registry. If
validation, admission or queue publication fails, no partial batch escapes and
each untransferred object is released exactly once. The notice drops its
adopted Handles before its strong Worker owner so queue retirement can still
submit exact-session native cleanup.

Worker-created `JSHandle.json_value` reuses the existing Handle-evaluation
call gate and bounded JSON serializer. A retained primitive returns without a
browser round trip; an object is evaluated in its exact Worker CDP session and
Runtime execution-context unique id. The host admits at most eight concurrent
Worker Handle calls and 64 cleanup reservations. Console publication retains
its independent 128-notice and 16 MiB FIFO limits. Python owns none of these
maps, limits, protocol operations or cleanup decisions.

The provisional installed ABI 64 wheel passes 131/131 Python units, Ruff and
strict Pyright. Five new Rust regressions pass; the host passes 937 all-feature
tests with 26 explicit live/diagnostic ignores and the addon passes 2/2.
Selected/all-feature strict release Clippy, Rust formatting and the 108-package
dependency proof pass.

The unchanged official async Chromium Worker console-Handle row passes 1/1 in
the installed candidate and 1/1 in the independent Playwright 1.58.0 oracle.
Their report SHA-256 values are
`a35e11498bf94b74445e4b490ff848b56ba5dfbb32ec7248901166c5af909e51`
and `1e8aa023fca1ebf396ead20ca624f03b884f2b608470e621b6e9fdeff1a17747`.
A ten-repeat candidate run passes 10/10, SHA-256
`857416b027e0c898741d07daa5b81343611d1c55b39289789b391c73ec1d1938`.
The complete unchanged Chromium Worker file passes 12/13, SHA-256
`c15afdc0e2f9bb7cefba76d1c89610455f8a70e5c44a5a59132cf7aed689ceb1`;
only independently scoped BrowserContext locale remains.

The cumulative 6,526-row ledger is 719 passed, zero failed, seven skipped and
5,800 not run, SHA-256
`51b9b9c6e380774f82c279b7032131ff0fa0aefb59b059ec3deba014e67f3e69`.
Exact comparison with NODE-304 changes only the selected Chromium row from not
run to passed. Fresh 6,526/6,526 collection SHA-256 is
`642a9f5f2fcfd338486db1fce733a596618a1586831f6af58c308f3aabb3e81b`.
The provisional behavior-evidence wheel SHA-256 is
`47956878f7a246693a24b31cbf8aa493f6c3aba148b11192760a0cdad99743ba`.
The two final repaired-wheel SHA-256 values are
`6b0ce7e210c44031f228625dcdb5a5850a786298f36fbd1b3d54698a44c3c3c6`
and `870707e752cd95af55b70651aa4cbdb54861158fcaf6cc03fdf3a930e0788a60`;
their normalized payloads are reproducible after normalizing only CycloneDX
identity. The final installed-wheel unit report passes 131/131, SHA-256
`eadd928bedb83495537abda01706fe910140bc67d577d09ca04b28ded94aa24c`.

Three final-wheel browser attempts are setup-only exclusions: one clean venv
lacked `pyee`, one shared-CDP target had no ready main world, and another
task's noisy page had filled pending console Handle capacity. None reached the
selected test body, and the other task's page was left untouched. Full Python
and Core JS/TS parity remain open.

## ABI 63 independent Worker Handle evaluation checkpoint (2026-10-04)

NODE-304 advances the package/native handshake to ABI 63. The independently
selectable `worker-handle-evaluation` feature composes `worker-events` and
`handles`; executable dependency proof shows its Pwright package closure is
identical to Page events plus Handles. Selecting neither prerequisite leaves
the base wheel and the other Worker leaves unchanged.

The native Worker owner retains its exact CDP session and Runtime execution-
context unique id. `Worker.evaluate_handle` resolves and evaluates in that
pair, adopts the RemoteObject into the existing opaque Handle registry and
compensates every failed publication through the bounded cleanup executor.
`JSHandle.get_property` uses the same exact owner path. The implementation
does not synthesize a Frame or borrow the Page main world. Python receives only
opaque ids and adapts async/sync public objects, arguments and error shapes.

Handle-to-Worker ownership is deliberately weak. A live Worker owns the native
session needed by its Handles, while disposal clears the Handle's native owner
without keeping the Worker, CDP connection or transport shutdown barrier
alive. Rust admits at most 64 retained Handles, eight concurrent Handle calls
and 64 cleanup reservations. Python adds no remote-object map, protocol
reader, polling loop, task or cleanup queue.

The final installed ABI 63 wheel passes 131/131 Python units, Ruff and strict
Pyright. Five focused Worker Handle regressions pass; the host passes 932
all-feature tests with 26 explicit live/diagnostic ignores and the addon passes
2/2. Selected/all-feature strict release Clippy, Rust formatting and the
108-package dependency proof pass. Two repaired wheel SHA-256 values are
`88481e8e363eddd22de51e075579918158b3900aef1aeed3de6e9ec7f1a47215`
and `7163f0e7dd52412ab3d78e8a65c94d8702ba58de0c14ebe806a00fd0eb609692`;
their payloads are reproducible after normalizing only CycloneDX identity.

The unchanged official async Chromium Worker lifecycle row passes 1/1 in the
final installed candidate and 1/1 in the independent Playwright 1.58.0 oracle.
Their report SHA-256 values are
`aead603f57778f8cd6939a1a340006ba768a0c33aa475dff4dce5ae379cfb415`
and `512eb5d9416563d9166ff74ef0b8e394c863fe36f64ee4200c492ed2d812fc6a`.
A final-wheel ten-repeat run passes 10/10, SHA-256
`fdd3c72e42ec4112b13a504308893e5bf923236701610bec27d1db6803adfa98`.
The complete unchanged Chromium Worker file passes 11/13, SHA-256
`1ded7bec96dbd0ae616d70f7a5b7ddca404f3e355c85fcf5bb6f0652aa430d17`;
Worker console argument Handles and Context locale remain explicit failures.
Other inherited operations on Worker-created Handles remain outside this
selected compatibility slice.

The cumulative 6,526-row ledger is 718 passed, zero failed, seven skipped and
5,801 not run, SHA-256
`e0dfeb150e8e410f6c4b4f62968330a9313023300d745d20f762def57bcc9501`.
Exact comparison with NODE-303 changes only the selected Chromium row from not
run to passed. Fresh 6,526/6,526 collection SHA-256 is
`642a9f5f2fcfd338486db1fce733a596618a1586831f6af58c308f3aabb3e81b`.
Setup-only final-v1/final-v2 attempts, the zero-test stress-final-v1 selector
attempt and earlier development/hang diagnostics are excluded from final
behavior evidence. Full Python and Core JS/TS parity remain open.

## ABI 62 independent Worker-network-event checkpoint (2026-10-04)

NODE-303 keeps the exact package/native handshake at ABI 62 because it adds no
native method or descriptor. The optional `worker-network-events` feature
composes `worker-events`, `request-events` and `response-events`. Unselected
Worker sessions retain their prior Runtime-only initialization. The executable
dependency proof shows that the selected leaf equals the combined
network/page-events package closure; it creates neither a second network
adapter nor another language-side watcher.

The selected Worker observer enables `Network` after Runtime and recursive
auto-attach initialization but before `Runtime.runIfWaitingForDebugger`. The
single Page network driver observes `Target.attachedToTarget` on its existing
cursor and admits at most 64 Worker or nested-Worker related-session routes.
Each route retains a bounded native session identifier, its parent session and
the exact owning Page. It does not allocate another Page adapter, event cursor,
task or active-request allowance.

The network tracker now keys native correlation by the exact
`(session, request-id)` pair. Ordinary Page traffic uses the empty scope;
Worker traffic retains its original session and request metadata while sharing
the Page's existing limit of 64 active requests. A Worker detach recursively
retires its descendants, drains only their scoped requests and publishes the
normal Page cleanup batch. Page retirement removes every related route. All
scopes share the same rejection and event-byte budgets, so one Worker cannot
multiply Page capacity.

Python continues to run one Browser-scoped network watcher and does not learn
CDP session identity. Page owns weak Request and Response identity maps keyed
by portable IDs; BrowserContext event/waiter conversion resolves through the
owning Page. The sync facade mirrors those identities with weak wrappers. This
makes an official Worker `response.request` the exact same public object as the
matching request without creating an unbounded strong history.

The final installed ABI 62 wheel passes 130/130 Python units, Ruff and strict
Pyright. Six focused Rust regressions cover scoped IDs, related-session
routing/retirement, resource accounting, Network initialization order and host
profile bounds. The host passes 927 all-feature tests with 26 explicit
live/diagnostic ignores; the addon passes 2/2 tests. Selected/all-feature
strict release Clippy, Rust formatting and the 108-package dependency proof
pass. Two repaired wheel SHA-256 values are
`4b97facee24ba3484b484b5361cf309dfa615a5f53098601feee81bb6950f9b1`
and `06986a96e172727ef31de0bc56ebeb72b04d8cb0be270c32a6f85ca52f016997`;
their payloads are reproducible after normalizing only CycloneDX identity.

The unchanged official async Chromium rows
`test_workers_should_report_network_activity` and
`test_workers_should_report_network_activity_on_worker_creation` pass 2/2 in
the final installed candidate and 2/2 in the independent Playwright 1.58.0
oracle. Their report SHA-256 values are
`42e5087226d06cb439d66132b3aada80a1d68586821bbc243949355cf4b29dab`
and `7b88bbb11f719ed0330c05eddb610c8950782b832de73483b56f4060a6f7e65d`.
A ten-repeat final candidate run passes 20/20, SHA-256
`cc5c71028a5c0ff50c45458011050e8ebdbc0ebe2521724a6e6df252e8505c8a`.
The complete unchanged Chromium Worker file passes 10/13, SHA-256
`b13c2b50c32e45a4ed5bdd4518b211583ebe1458625b09bf4cf717718f5b8d3c`;
the three remaining failures are Worker Handle evaluation, Worker console
argument Handles and Context locale, not network regressions.

The cumulative 6,526-row ledger is 717 passed, zero failed, seven skipped and
5,802 not run, SHA-256
`361e34e330c5fd5908802e561f126743e2a42f911f34d762283633a0333efbff`.
Exact comparison with NODE-302 changes only the two Chromium Worker-network
rows from not run to passed and has no losses. Fresh 6,526/6,526 collection
has SHA-256
`d43ff5a1d5478273618fec81cb0ee5d23c81d5f60ae67590eb5f3815c8ed9efe`.
The first final-named setup attempt is excluded because a stale blank Chrome
target prevented fixture setup before target behavior. A second diagnostic
report reached the target and exposed the now-fixed Python identity mismatch;
neither report is final evidence. Full Python and Core parity remain open.

## ABI 62 independent Worker-error-event checkpoint (2026-10-04)

NODE-302 keeps ABI 62: `worker-error-events` adds no native export or public
descriptor. It is independently selectable over `worker-events` plus
`page-errors`, and the executable dependency proof shows that it adds no
Pwright package beyond the existing `page-events` closure. Unselected
`page-errors` retains its prior Frame/Page-only behavior.

The selected Rust observer keeps the existing ownership order. It first maps
`Runtime.exceptionThrown` through an exact Frame execution context, then an
exact Page target session, and only then consults the retained Worker registry
for an exact session-to-Page fallback. Unknown sessions remain ignored. The
resolved exception reuses `PageErrorPlan`, ordered Page lifecycle publication
and the existing bounded history: 200 errors per Page and 16 MiB total event
bytes. The leaf adds no task, FIFO, protocol subscription, cleanup job or
resource allowance.

The Python layer is unchanged. Its existing Browser-scoped Page-error watcher
reads the same native PageError history and emits through the same public Page
and Context facade path. It neither identifies Worker sessions nor retains a
second queue. This preserves the rule that protocol ownership and correlation
stay in Rust while the language package only adapts public objects.

The final installed ABI 62 wheel passes 129/129 Python units, Ruff and strict
Pyright. The host passes 927 all-feature tests with 26 explicit
live/diagnostic ignores; the addon passes 2/2 tests with its PyO3 interpreter
pinned to the final Python 3.12 environment. Selected/all-feature strict
release Clippy and the 108-package dependency proof pass. Two repaired wheel
SHA-256 values are
`2db7c264dcc10c34f52103ca575df0a28812084c3423630ef2b098c98dc08054`
and `5e8f6cd0d851b9ab841b104230ee958ee4c631e77c2b46cfc5e445bc555ab9c5`;
their payloads are reproducible after normalizing only CycloneDX identity.

The unchanged official async Chromium Worker-error row passes 1/1 in the
final installed candidate and 1/1 in the independent Playwright 1.58.0 oracle;
their report SHA-256 values are
`4b426debbd1724ad71de797c2f539bed0b2e6915a990e9a8bbf5e9d721bd83fb`
and `2b71de897545134ca5ff8727743bad9eaf38b875866188c9322a3e88220b9b3c`.
A ten-repeat candidate run passes 10/10, SHA-256
`90571679bf40d10f6a7592f4301f7b867e73f0cfdebcd2fe6bb27066a8fc7c81`.
The cumulative ledger is 715 passed, zero failed, seven skipped and 5,804 not
run, SHA-256
`caf76e34c05bd6ce6da5b3467fc142f7abdaf05c545f8cf94f69578e0fe2a539`;
exactly one NODE-301 not-run row becomes passed and there are no losses. Worker
console argument Handles, `Worker.evaluate_handle`, Context locale, Worker
network attribution and full Python/Core parity remain open.

## ABI 62 independent Worker-console-event checkpoint (2026-10-04)

NODE-301 advances the native handshake to ABI 62 because it exports
`NativeSession.next_worker_console_notice`. The optional
`worker-console-events` feature composes `worker-events` and `console-events`;
it does not widen either prerequisite when unselected. Dependency proof shows
the selected leaf adds no Pwright package beyond the existing `page-events`
closure.

The Rust observer forwards every topology event to the existing Worker owner
first, then accepts `Runtime.consoleAPICalled` only when its session ID maps to
an exact retained Worker. The existing bounded console plan owns public type,
text, diagnostic argument JSON, location and timestamp. One installation owns
a 128-notice FIFO, 16 MiB `EventBytes`, 4 KiB fixed state and 128 cleanup
reservations. Each event admits at most 16 argument descriptors and 4 KiB of
RemoteObject IDs. Because this leaf deliberately does not imply Worker
argument Handles, those object IDs are released asynchronously through the
exact retained Worker session; closed/detached/protocol-terminal receipts are
accepted as cleanup completion. Host close clears this dependent queue before
closing the Worker owner registry. The maximum native tuple fits the existing
256 KiB Python conversion reservation and has a compile-time proof.

Async Python owns no protocol or retained event queue. One Browser task awaits
the native FIFO, reuses or publishes the exact Worker facade, creates one
`ConsoleMessage`, and emits that same object through Worker, Page and Context.
`ConsoleMessage.worker` exposes the exact Worker facade. Sync Python wraps the
same event identity, predicates and expectation context managers. Neither
binding fabricates JSHandles from diagnostic JSON; the official Worker console-
argument Handle row remains open until a separate retained-Handle leaf exists.

The final installed ABI 62 wheel passes 129/129 Python units, Ruff and strict
Pyright. The host passes 926 all-feature tests with 26 explicit
live/diagnostic ignores; the addon passes 2/2 tests after pinning its PyO3 test
interpreter to the final Python 3.12 environment. Selected/all-feature strict
release Clippy and the 108-package dependency proof pass. Two repaired wheel
SHA-256 values are
`739d49ccdffd7c2e1685e0dd7d2ae6948718fe6b59758924826b7be2027217a5`
and `efd120f8b456e3734f0eae414fa35e4b4de71c9dd9a6b42904214917f8f0af46`;
their payloads are reproducible after normalizing only CycloneDX identity.

Three unchanged official async Chromium Worker-console rows pass 3/3 in the
final installed candidate and 3/3 in the independent Playwright 1.58.0 oracle;
their report SHA-256 values are
`d5a556b3377b7c34f73f2fb9be9f088988c317569cfa9097a019f75973415987`
and `e7128df0d47ad0936b15dbfdf903ff15a6db45c077f697819cc2f7ac82f90829`.
A ten-repeat candidate run passes 30/30, SHA-256
`5c1320a89c96fa8c4355940f5ff7bc1b11d65b7e2b1c1111cf4c2f629836dbef`.
The cumulative ledger is 714 passed, zero failed, seven skipped and 5,805 not
run, SHA-256
`ef648ecece0f94e5f7a3968e6d652a08d1bb5eb56b8bf8c1afd7045f6d6b902c`;
exactly three NODE-300 not-run rows become passed and there are no losses.
Worker errors, evaluation/console argument Handles, Context locale, Worker
network attribution and full Python/Core parity remain open.

## ABI 61 Worker-retirement checkpoint (2026-10-04)

NODE-300 makes no native descriptor or ABI change. It completes the facade-side
retirement contract for ABI 61's existing independently selectable
`worker-events` capability. Rust still owns target retention, exact detach,
session commands and transport shutdown. Python only revokes an escaped native
owner when the containing public Page, Context or Browser has already reached
a terminal state.

The triggering race was Page removal before delivery of a Worker close notice.
Treating the notice as an unknown-Page error retained a Python traceback whose
locals included the native Worker owner and its CDP connection. That prevented
Rust's Browser owner from reaching `io_finished`. Terminal paths now retire all
facades synchronously, clear the native reference, publish public close at most
once and reject subsequent evaluation with `TargetClosedError`. An exact late
notice for the removed Page is ignored as a stale receipt. The adapter does not
invent a detach, synthesize browser state or take ownership of CDP cleanup.

This lifecycle repair is deliberately separate from future Worker features.
Worker console/error publication, retained argument and evaluation Handles,
Context locale and Worker network attribution can each be added as separately
selectable capability leaves instead of widening base `worker-events`.

The final installed wheel passes Ruff, strict Pyright and 128/128 Python units.
Four unchanged official async Chromium Worker rows pass with focused report
SHA-256
`601621d526157d550fbdfd92dabf4546fb1574cef2da5f241382900a827470b2`.
The cumulative ledger is 711 passed, zero failed, seven skipped and 5,808 not
run, SHA-256
`cfcddd468eef3ef754910e1c7055ffbbbbc4c6454546589ff74323999acbf4fb`;
exactly four NODE-299 not-run rows become passed and there are no losses. Two
normalized-reproducible wheel SHA-256 values are
`aecd520722b35db70ee096eb030fbb55662e37d4c479a3efbffc050b6c26329c`
and `be4ed3912c802d2b30ce11d99361428c67740386266c5acd8e0301fa13913032`.
The wider Worker diagnostic is not authoritative because timeout pollution
affected later cases; a separately clean run confirms Worker network
attribution remains absent. Full Python and Core JS/TS parity remains open.

## ABI 61 caller-dispatcher checkpoint (2026-10-04)

NODE-299 does not change the Rust/native descriptor ABI. It adds the missing
sync scheduling adapter around ABI 61. The owner thread has a small asyncio
dispatcher compatible with official Playwright's `_dispatcher_fiber` contract;
`greenlet` suspends child facade calls while the submitted browser coroutine
continues on the existing Pwright loop thread. Root-greenlet calls keep the
ordinary blocking path. Native/browser ownership, CDP routing, event queues and
all capability logic stay in Rust. Concurrent suspended calls remain covered
by the existing 32-call admission semaphore.

The dispatcher publishes a running loop so official caller-side helpers can
use `asyncio.create_task`, advances helper work without polling native state,
and wakes an exact greenlet only from its completed concurrent future. Stop
clears the loop marker, cancels remaining helper tasks, joins the Pwright loop
thread and closes caller state. `greenlet>=3.1.1,<4` is an explicit wheel
dependency. AAA tests cover helper-loop pumping, two genuinely overlapping
background coroutines and marker cleanup; installed-wheel units pass 127/127,
and Ruff/Pyright pass.

The installed final wheel passes the unchanged focused close/rejection row and
the complete unchanged Chromium sync file 27/27. Their report SHA-256 values
are `d5f6c4cedff7275d14559de7ccd33acd8af488c42584d8609742375419ff2a07`
and `6a77352e928c17da5ef05c3eb0e17c23c1a74089573df8697bb8473af3dd29ec`.
The cumulative ledger is 707 passed, zero failed, seven skipped and 5,812 not
run, SHA-256
`a43bef1f0b558f324191e12cb2dc28d713d9108466c3bf68bf0edf09bc9bddc3`;
only the caller-dispatcher row changes from NODE-298 and there are no losses.
Two repaired wheels have SHA-256 values
`eb078b9c0da926a28dd263fe831336839176e700ee6b90fa00fec207e8241d68`
and `ea38b110a1af14b0d88ae310b8c6c783a41dbe3ba37758475b5cc864d0115705`,
with normalized-reproducible payloads. Full Python and Core JS/TS parity
remains open.

## ABI 61 independent Worker-event checkpoint (2026-10-04)

NODE-298 advances the Python native ABI to 61 and adds `worker-events` as an
independently selectable leaf over `page-events`. Rust owns the complete
dedicated-Worker lifecycle. The topology observer opts in to retaining only
children of known Page/OOPIF routes or already retained Worker sessions. Each
retained session enables Runtime, installs recursive auto-attach and only then
resumes from the debugger pause. The owner retains exact Page identity,
monotonic creation order, URL and session client; evaluation readiness requires
both a Runtime execution context and `Inspector.workerScriptLoaded`. Exact
detach marks the owner closed and publishes closure. Limits are explicit: 64
live owners, 128 queued notices, 1 MiB URLs and eight concurrent evaluations
through the existing bounded value codec. The dependency proof shows exactly
the existing `page-events` package closure.

Async Python runs one Browser-scoped `next_worker_notice` watcher and exposes
only the public `Worker`, Page snapshot/event/expectation and evaluation
facades. Sync Python wraps the same identity for callbacks, predicates,
`EventInfo.value`, snapshot reads and evaluation. Python does not attach or
detach targets, subscribe to CDP, decide readiness or retain a second event
queue.

The unchanged official Worker test passes from the installed final wheel,
report SHA-256
`bb036594265e433f36cbfe291872531b1a723addbe05a72f187eee3d60bffd72`.
The complete unchanged Chromium sync file passes 26/27, report SHA-256
`6dd94f8af4853667f7254b8f85fbe52e1c3510196d771430b9f17d4ffdb91298`,
against 27/27 in the independent oracle. The cumulative candidate ledger is
706 passed, one failed, seven skipped and 5,812 not run, SHA-256
`67413210a814c04723bc62aa595210856f43891ed23e615622817b5465736a36`.
Compared with ABI 60, exactly the Worker row changes from failed to passed and
there are no losses. The remaining sync failure is caller-loop `sync_gather`
behavior.

Two repaired ABI3 wheels have SHA-256 values
`dd3fc5ef46cac61ee3a54321f04fffe72d86fba962fcc69194e0bb65d0846158`
and `07bd3a2845afc60027a69a488714d7cf2e163ec2369b9fc23c2257bd011850d9`;
their normalized payloads are reproducible. Python units pass 125/125, the
standalone Worker gate passes 3/3, the topology retention/initialization gate
passes 1/1, and strict selected/all-feature release Clippy, Ruff, Pyright and
the 108-package dependency proof pass. Full Python and Core JS/TS parity
remains open.

## ABI 60 independent download-event checkpoint (2026-10-04)

NODE-297 advances the Python native ABI to 60 and adds `download-events` as an
independently selectable leaf over `frame-lifecycle`. It does not imply request
or response events. Rust installs exact-Context Chromium download policy with
events enabled, owns one private temporary directory per installation and one
GUID-named artifact per download, correlates native Browser events through the
Frame/Page/Context topology, and settles the exact retained owner on completed,
canceled or terminal Browser state. Context close removes its directory and
releases policy state. Limits are explicit: 16 Contexts, 64 owners, 64 queued
notices and 4,096-byte paths. The dependency proof shows exactly the existing
`page-events` package closure.

Async Python runs one Browser-scoped `next_download_notice` watcher and emits a
public `Download` to the exact Page. Sync Python wraps the same object in event
callbacks, predicates and `EventInfo.value`; `Download.path()` only crosses the
native boundary to await the Rust owner. Python does not read CDP, select a
download directory, infer completion or retain an artifact queue. Separately,
the existing Rust Locator click barrier recognizes a same-Frame
`Browser.downloadWillBegin` as the terminal success of a requested navigation,
preventing a download-producing click from waiting forever for a navigation
commit that Chromium will not emit.

The unchanged official download test passes from the installed final wheel,
report SHA-256
`e7fbbce7744aabfea9cdf1bd9c6cfa2d0e02c6306e7e4b579c94ff327e2b261a`.
The complete unchanged Chromium sync file passes 25/27, report SHA-256
`a102e15152609462005de228fa92cdac804f3fc24de6cfb5a589ccbd4e92ff26`,
against 27/27 in the independent oracle. The cumulative candidate ledger is
705 passed, two failed, seven skipped and 5,812 not run, SHA-256
`c7c6e0f6a827db762291614bbfa127205cec572b1c1328250ad1dc1e71da6f9b`.
Compared with ABI 59, exactly the download row changes from failed to passed
and there are no losses. The remaining sync failures are Worker events and
caller-loop `sync_gather` behavior.

Two repaired ABI3 wheels have SHA-256 values
`e84ee8646f093d676b28fcb8a507040cbefb2c1802dc519f90f496120c6744f4`
and `0b152147e6eb49901b113ec7541d88a0343248466373b806c68750be29d294e0`;
their normalized payloads are reproducible. Python units pass 123/123, the
standalone download gate passes 3/3, the Locator download-navigation regression
passes 1/1, and strict selected/all-feature release Clippy, Ruff, Pyright and
the 108-package dependency proof pass. Full Python and Core JS/TS parity
remains open.

## ABI 59 independent request-event checkpoint (2026-10-04)

NODE-296 advances the Python native ABI to 59 and adds `request-events` as an
independently selectable leaf over `network-events`. It does not imply
`response-events` or `request-history`; all three may compose over the same
Chromium network observer. Enabling or disabling request history preserves a
Request subscription already owned by request events, and Context creation
activates the new leaf with rollback if any installation step fails.

Rust owns the complete event lane. At most 16 Context subscriptions and 256
FIFO notices are live. Request and response notices share the existing 16 MiB
network queue budget, so enabling both cannot double the retained-byte ceiling.
Each notice carries the exact Page, immutable request record and optional
request-history/size owner. Topology reconciliation releases notices for
retired Pages, Context disablement releases its subscription, and Browser
shutdown clears the queue before closing the observer. The PyO3 boundary only
converts the bounded descriptor. Async Python runs one native watcher per
Browser and emits the existing public `Request` wrapper to the exact Page;
sync Python wraps that same object and keeps exact callback identity for
removal. There is no Python CDP reader, polling loop per Page or second retained
network queue.

The unchanged official
`tests/sync/test_sync.py::test_sync_network_events[chromium]` passes from the
installed final wheel, report SHA-256
`a96e49de4e1d893f7683ab82deeac0aebeb02616faaa128dc93bf991293c16ff`.
The complete unchanged Chromium sync file passes 24/27, report SHA-256
`e8e09ddb45aaf4ddc68653bdba6431df7c44fe0b589af393789d84a33e2fd320`,
against 27/27 in the independent oracle. The cumulative candidate ledger is
704 passed, three failed, seven skipped and 5,812 not run, SHA-256
`5a4815de54d0993e383f53e960378cec14199a86753379702ddf167b23aba983`.
Compared with ABI 58, exactly the network-event row changes from failed to
passed and there are no losses. The remaining sync failures are download
publication, Worker events and caller-loop `sync_gather` behavior.

Two repaired ABI3 wheels have SHA-256 values
`c4d90b75345d401d91820a8df0e7125ed6c8f042c0a7e969904d74577e636f50`
and `a335c4f2804328ec1923b9b170a58305a7052ba3780ffaa5cf7eeea726951a6d`;
their normalized payloads are reproducible. Python units pass 120/120. The
standalone request-event Rust gate passes 8/8, the combined network-feature
gate passes 16/16, and strict selected/all-feature release Clippy, Ruff,
Pyright and the 108-package dependency proof pass. Full Python and Core JS/TS
parity remains open.

## ABI 58 console Handle and subscription checkpoint (2026-10-04)

NODE-295 advances the Python native ABI to 58. A console event now stages each
CDP argument through the existing Rust Handle owner, preserves argument order,
and publishes the completed `ConsoleMessage` only after every retained Handle
is ready. At most eight console Handle publications may be pending through the
shared call bound. Failure recovery releases every remote object which was not
successfully transferred. Python receives native Handle facades and exposes
the official async/sync `ConsoleMessage.args`; it does not reconstruct values
from `arguments_json` or own remote-object lifetime.

Live console interest is also native bounded state: at most 16 Page and 16
BrowserContext subscription entries, each with a checked listener count and at
most 32 uniquely tokened action waiters. Persistent listeners are active
immediately. A real-browser regression schedules a delayed `console.log`,
registers `page.on("console")`, performs no further Pwright command and still
receives the message. Action waiters begin pending; ordinary commands activate
them at their command boundary, while `goto` and `reload` activate them on the
next main-frame `Page.frameNavigated` observation. Per-waiter tokens make
listener removal and overlapping waiter cancellation release the exact owner.
The observer snapshots whether a message is publishable when the CDP event
arrives, while history remains independent of live delivery. This prevents a
favicon 404 emitted after the prior navigation returned from satisfying a
waiter whose action is the next navigation. Subscription pruning follows Rust
topology and browser teardown drops the console observer before joining the I/O
task.

The unchanged upstream Playwright Python 1.58.0 async and sync console modules
both pass 8/8 against the final installed wheel. Their report SHA-256 values
are `d96db0250be5b89956f49e346d67af04fde065ef1224130f85ee5ce1a1876faf`
and `f4e16451571da8029f0d5695e9ca99b8e12201bd35b07a196de4a99fd56184f0`.
They exercise listener order, `expect_console_message`, every console type,
location, detached-frame cleanup and real Handle `json_value()` for string,
number and object arguments.

The official async oracle passes 8/8. Its sync module exposed the same favicon
race once at 7/8, then the exact failed row and a complete retry both passed.
The failed report is retained; the exact passing retry was merged into a
canonical 8/8 oracle report rather than discarded. This documents an upstream
oracle flake, not a candidate waiver. The canonical sync oracle report SHA-256
is `cae865b9f04f32e4148dc810233b6fb6168ab9b0f187e650fba152f21f49b929`;
the async oracle report SHA-256 is
`f95ac8f7734c44381d45277059a21938fcd63954b092c628d1ca20d710892172`.

The complete unchanged Chromium `tests/sync/test_sync.py` now passes 23/27 in
the candidate, report SHA-256
`5064cb02833dcc487feb4aa2f06f329944f72765c70230458492e0b834e49df5`,
versus 27/27 in the independent oracle, report SHA-256
`ffce16c779b1317d27b1eab2526c18dcbdfb27d57686258c1baa7105466bf0fe`.
The remaining candidate failures are explicit: request-event publication,
successful download/artifact publication, Worker events and the upstream
`sync_gather` helper's assumption that the dispatcher loop runs on the caller
thread. The sync facade now matches the official nested/running-loop guard,
event-timeout exception and post-stop error shapes.

The final candidate ledger is 703 passed, four failed, seven skipped and 5,812
not run, SHA-256
`e1a09cc45a5eb609aac2884804f4514eea66dcdf17c80d3c063b44f55d8d2b35`.
The oracle ledger is 688 passed, zero failed, seven skipped and 5,831 not run,
SHA-256
`62346332e34528c04ffe1dc84cadf130be41911c01311922e3d5485e3e082ec3`.
An exact outcome-only audit records 21 candidate improvements with no losses:
the 16 console rows move from not-run to passed and five `test_sync.py` rows
move from failed to passed. The corresponding oracle audit records the same 16
console additions plus the nested-sync row repair, again with no losses.

Two repaired ABI3 wheels have SHA-256 values
`d973dd55efd049dfea9e4a0281fc9c472842e192343acd28139c270645f0c50d`
and `33b4787e562704afb02077a4c1b7240690fd98ef9099cb1ff9e7550f03c2042e`;
their payloads are reproducible after excluding only CycloneDX UUID/timestamp
metadata. Python units pass 118/118, both focused Rust subscription tests pass,
strict all-feature/all-target host and addon release Clippy, Ruff, Pyright and
the 108-package dependency proof pass. Full Python and Core JS/TS parity
remains open.

## ABI 57 console-event projection checkpoint (2026-10-03)

NODE-294 adds `console-events = ["page-events"]` to the Python addon and
selects the existing host `console-events` leaf. Chromium console observation,
Page identity, message bounds and publication order remain in Rust. The PyO3
lifecycle descriptor carries one bounded console tuple; async Python restores
the public `ConsoleMessage`, and sync Python wraps the same object without a
second event queue or protocol path. Page-error strings were folded into one
descriptor at the same ABI boundary so callback arity remains supported.

The lifecycle conversion reservation is now 256 KiB rather than 32 KiB.
Compile-time assertions prove that it covers two conservative metadata URL
copies plus the maximum selected console or Page-error payload and descriptor
overhead. The profile remains composable: `console-events` and
`console-history` may be selected independently, and the dependency proof
shows that event publication adds no Pwright package beyond `page-events`.

The complete release extension build, all-feature strict Clippy, the
108-package dependency proof, Ruff format/lint, strict Pyright and 112/112
Python units pass. This is source/package evidence only. The bounded
`arguments_json` field supports formatting and diagnostics, but is not exposed
as `ConsoleMessage.args`: official argument values are retained JSHandles with
their own identity, lifetime and `json_value()` behavior. That Rust-owned
Handle publication remains open.

The unchanged official Chromium rows `test_console_repr` and
`test_console_should_work` have not been rerun because the zero-tab preflight
still finds an unrelated Gemini page in the shared dedicated Chrome. No
candidate/oracle report or ledger movement is claimed; ABI 56 counts remain
current.

## ABI 56 ElementHandle DOM checkpoint (2026-10-03)

NODE-291 adds one narrow host feature, `element-handle-dom = ["handles"]`, and
the matching addon feature. Descendant queries reuse `LocatorQuery`, the
bounded selector runtime and `PendingHandle` ownership. Rust holds the query
lease across native evaluation, returns only an element result, and waits for
native release before returning `None`. `inner_text` validates HTMLElement in
the browser and returns only the typed Rust `EvaluationValue::String` result.
The async and sync Python APIs contain no selector parsing, browser script or
handle classification.

The two unchanged Chromium rows
`test_sync_query_selector[chromium]` and
`test_sync_nested_query_selector[chromium]` pass 2/2 from the final ABI 56
wheel. Final report SHA-256 is
`55449eed1e22968a614813cec9eb8daab124a1aec6b22b1342c6deed361f3dbd`.
Exact replacement in the complete 25-row NODE-290 report yields 16 passed /
nine failed, SHA-256
`c118e64d7029d39463f1d1a8212398b1b545bc76f6cb1f88bcff5a1871619fb8`.
Including the two earlier passes, candidate is 18/27 for the file; oracle is
unchanged at 26/27.

The tranche ledger is 16 passed, nine failed and 6,501 not run, SHA-256
`6fb7ddee2fc902d4f142d8f6da3e7b888ea1f2f07f1c84055c4618ae266c6f1e`.
The cumulative candidate ledger is 682 passed, nine failed, seven skipped and
5,828 not run, SHA-256
`a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`.
The independent oracle ledger is unchanged.

Normalized-reproducible ABI 56 wheel SHA-256 values are
`faba2f1c41352d1548e3e94505a3c4ff1ce4b2da39dc8d1b26861355e74888b7`
and
`c5501536fbdfc473c71fde14c313d68f6051645c6865fcd6df72c1d509364349`.
Installed Python tests pass 111/111 with five expected skips; focused release
Rust tests pass 2/2; release Clippy, Ruff and strict Pyright pass. Fresh
candidate collection remains 6,526/6,526, report SHA-256
`f3374fe294c20aff18ae1d279acf92ebf70cbe2d0af13813173151eafd5f6a42`.
Nine sync failures, the other 5,828 Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 55 sync API inventory checkpoint (2026-10-03)

NODE-290 pins unchanged sync `tests/sync/test_sync.py` at SHA-256
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
It executes all 25 Chromium rows absent from the cumulative ledger; the other
two file rows were already green. The oracle passes 24 and fails the
sync-inside-asyncio assertion in this isolated harness, SHA-256
`3ce79c1d557e9593ba2c4f63db88e3d17db4231e7544516baa19b1e7c73bcb63`.
The candidate baseline passes nine and fails 16, SHA-256
`d8bb9824057c1381c2d32158d84c5d92570ea743f69f60a5ebc68ed36dc99626`.

The repr repair lives entirely in the public facades. Page reads its current
URL; BrowserContext, Browser and BrowserType format already-owned metadata.
Because Pwright does not download or bundle a browser,
`BrowserType.executable_path` is empty. The official shim's `_impl_obj`
property exists only to run an unchanged upstream assertion that constructs
its expected value through private implementation state.

The Response repair preserves the ownership boundary. Rust's bounded request
history already retains the exact `NetworkRequestRecord` and settles it with
the Response. `ResponseNotice` and `NetworkWaitRecord` now carry the same
owner, PyO3 publishes that native record, and Python constructs only the
public Request facade. Both response-wait and listener paths share this
association; there is no Python request cache, ID join or browser semantics.

The final five-row repair report passes 5/5. Exact replacement yields a
14 passed / 11 failed tranche, SHA-256
`d9b84034f306be4f0bc61ceb9438a2aa79e19ce470c0906981ba17507514696f`.
Candidate/oracle tranche-ledger SHA-256 values are
`092b98461c8def4cb2d54b3a463f42b9231e7a8ab74636b9299604a8f2f07177`
and
`cf0b46f3d4d2166e08e3353a4a56fee35ee4f3b8ec8d32bb9e9d682e8cf074ea`.
The cumulative candidate is 680 passed, 11 failed, seven skipped and 5,828 not
run, SHA-256
`099db2a6f4937871240a3fb7c329606cb2f9c81fccb58932534bc3c0ef45116e`;
the cumulative oracle is 671 passed, one failed, seven skipped and 5,847 not
run, SHA-256
`5e0133285e5a74230173ec2df55f001c5424413a3b10f676cf9a2b3973c33a93`.

Normalized-reproducible ABI 55 wheel SHA-256 values are
`bcb27c0bdf17422d0589426c1d8e992aa008f24ff8fe4e49c1bd9a8469d4c99a`
and
`f96c9dbbe14c606f7bca41f8310e03947007814e7e2039671703f247c1e8d9ae`.
Installed tests pass 109/109 with five expected skips; focused Rust tests,
release Clippy, Ruff, strict Pyright and the fresh 6,526/6,526 candidate
collection pass. Collection report SHA-256 is
`823aa604740e27cd7b0bde70e86dd8d8098077cc4dc7781344942db8139f5fd1`.
The 11 sync failures, other 5,828 Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 55 Page evaluate API-name checkpoint (2026-10-03)

NODE-289 pins unchanged async `tests/async/test_asyncio.py` at SHA-256
`544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`.
Earlier ledgers already pass all three browser expansions for idempotent stop
and pending request cancellation plus the Chromium selector-cancellation row.
This checkpoint selects the two remaining Chromium rows: TaskGroup cancellation
must not leak the cancelled Locator error, and JavaScript evaluation errors
must start with the originating `Page.evaluate:` public API name.

The independent oracle's first attempt is retained as red infrastructure
evidence, SHA-256
`a5ea2c01f248e1fafa01df574c8cef8627fa02724d0911735b9c4f0d18536dae`:
without a Pwright marker Page, fixture setup cannot create a Context through
the dedicated-CDP launch adapter. Restoring the marker makes both unchanged
oracle rows pass, SHA-256
`6328485903ac03a3226871803ca6380c99f6e2369a9c94b6f98c529a28abbb0f`.
The candidate baseline passes TaskGroup cancellation and fails only the API
prefix assertion, SHA-256
`6cb059ea2647f9388c61921b22ada302dc7b52213c0bca5f359303d5f9c099c0`.

The repair mirrors Playwright's public-call boundary without moving browser
semantics into Python. Rust continues to classify the browser exception as
`JavascriptError` with stable code `PWRIGHT_JAVASCRIPT`. Python constructs the
same exception type with the public API prefix, carries any explicit public
name and browser stack, suppresses internal exception chaining, and avoids a
second prefix if an outer boundary has already shaped the message. Cancellation
and unrelated Python exceptions are not caught. The native ABI, dependency
graph, protocol behavior and resource limits do not change.

Final installed candidate and independent oracle pass 2/2 with identical
report SHA-256
`6328485903ac03a3226871803ca6380c99f6e2369a9c94b6f98c529a28abbb0f`.
Candidate/oracle tranche-ledger SHA-256 values are
`a8571298097874f152b85105f5b6e7b41d3ded9aa7811405d454cb0bdd20adba`
and
`6b7d99b7dd58f684e11e6ea191671a02a231f3b6e46663c3f1b58866edae5b88`.
The cumulative candidate is 666 passed, seven skipped and 5,853 not run,
SHA-256
`2e8f750a7e2a16af688489ef6aff90fc9467946109b18a007eccfb5e4626fd0d`;
the cumulative oracle is 647 passed, seven skipped and 5,872 not run, SHA-256
`6d54d97cd037a0e17c8b8f1ce923a6c8459343e04b41ffa19eb5cc5ee98d3215`.
Two normalized-reproducible repaired ABI 55 NODE-289 wheel SHA-256 values are
`933297c96848dbbe99381ef1a353100bfae8ac9db12c9ec21bba16d3f007aef3`
and
`d11377c024ed993dd37b345e95481e0fdc91fb6074e4a4d30a499b80e6ee12e2`.
Installed Python tests pass 107/107 with five expected skips; Ruff, strict
Pyright and fresh 6,526/6,526 candidate collection pass. The collection report
SHA-256 is
`2d4e36a6ae0dc5b78fd6b9250ee743d43fb372ebc85e8c7884734556df7b1d12`.
The remaining 5,853 Python rows and all 48,477 Core JS/TS invocations stay open.

## ABI 55 selector-wait error-shape checkpoint (2026-10-03)

NODE-288 selects three Chromium rows from unchanged async `test_asyncio.py`,
async `test_expect_misc.py` and sync `test_sync.py`, source SHA-256 values
`544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`,
`61e5d93df621e59a44717532192194bce1a85cc858d5a1f2e69ba724ce0945a6`
and
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
They verify cancellation of a selector wait after native dispatch without an
unretrieved future, `.name == "TimeoutError"` on a 1 ms selector timeout, and
sync Page evaluate/content/wait interoperability. Oracle passes 3/3.

The ABI 55 candidate baseline passes cancellation and sync execution. The
timeout is already a native `TimeoutError`, but its Python exception lacks
`.name`. The repair installs `message` and `name` properties together on the
native exception classes. The name defaults to the public class name and has a
setter backed by a private override, preserving exact browser error names for
`Page.page_errors()`. No error is reclassified in Python; Rust remains the
source of the exception taxonomy and stable native code.

Final installed candidate and independent-oracle reports pass 3/3, SHA-256
`95061944d9d6e0f66207653b10171d7ebd9d4881683efa7f8efdac77a7c3c611`
and
`18763889805affdb62fd0dc6951e665ba662fa67068d7f03b153820c1e6798c4`.
Candidate/oracle tranche ledgers are
`1aa8ced79b0daeb738c681fb6a8903f3ceaf33ca6266b984efc4892343dd810a`
and
`9f9143a3dfcad40b0b9d572bd87d59f066fde4d3ff154e083e9ce6a447f27dd4`.
The cumulative candidate is 664 passed, seven skipped and 5,855 not run,
SHA-256
`7f8c62ece39f4634eba752e80441a53aabf4f4272cffd5ce84689a75becddfef`;
the cumulative oracle is 645 passed, seven skipped and 5,874 not run, SHA-256
`b4bd2c9c36f3508b9a5b359d95d7d7685801bede216a55474222b1507ea21565`.
Two normalized-reproducible repaired ABI 55 NODE-288 wheel SHA-256 values are
`5c2dcc36835eb0c5b26b60b6499824a843175910eba503f0d9ec6b6244f973f1`
and
`53016151567480d1af17f39111fb380b3e6148a88ab78f1aeaca1e5f733db0db`.
Installed Python units pass 106/106 with five expected skips; Ruff, strict
Pyright and fresh 6,526/6,526 candidate collection pass. The Rust graph,
native ABI and dependency proof remain unchanged from NODE-287. The remaining
5,855 Python rows and all 48,477 Core JS/TS invocations stay open.

## ABI 55 Page wait-for-selector checkpoint (2026-10-03)

NODE-287 advances the native handshake to ABI 55 and pins unchanged async
`test_issues.py`, source SHA-256
`5d99e3d911d610cdb116e6b2c9e7d7ac7de8df051cfbe823712297465c375bea`.
The selected rows are the Chromium invocations of
`test_connection_task_cancel` and `test_issue_195`. The ABI 54 baseline passes
the device-Context row and fails the cancellation row only because the Page
facade lacks `wait_for_selector`.

The new Page method validates state, strictness and timeout, constructs an
immutable native Locator recipe, and derives native `nth(0)` when strictness is
false. Rust resolves the Frame and invokes the existing Locator `WaitFor`
program under one deadline. Timeout zero remains unbounded; an expired second
phase reports the original total timeout. `attached` and `visible` publish
through the existing bounded Handle owner, while `detached` and `hidden`
return `None`. Sync delegates to the same async/native route. No polling,
selector concatenation, frame identity or ElementHandle identity lives in
Python.

The unchanged cancellation test races a visible input against an unresolved
selector, cancels the pending task and observes `CancelledError`; the final
candidate exits normally. The whole-file oracle diagnostic is retained red
because `test_issue_189` calls generic
`BrowserType.launch(ignore_default_args=...)`, which the mandatory dedicated-
Chrome attach adapter intentionally rejects. A separate service-worker-policy
diagnostic is also excluded: default-allow passes async and sync, while block
times out awaiting a console event under the MagicDNS HTTP fixture. Neither
diagnostic is relabeled as product conformance.

Final installed candidate and independent-oracle reports pass 2/2, SHA-256
`18b7130d89320ae47ee72ee45c452d4d6a7b7204885921a1b5b7b624b6851718`
and
`991b5bf6cc6fc47d38cf7f4ce02fbca7964ef3b6c2c59bad9ec71cff2bcacb1f`.
Candidate/oracle tranche ledgers are
`67542a752ae8dbd54d91dfab1ed69ca7ae59995a59bbb12894337493f05b00d0`
and
`c59574200b25fb454bccc03e7ad041ebe0bfc652c1fb8213db3ef93b192aad96`.
The cumulative candidate is 661 passed, seven skipped and 5,858 not run,
SHA-256
`169dfbabd9bf984d59082cdf4c0b68a84941a67c7e04c8997cc63645243561e9`;
the cumulative oracle is 642 passed, seven skipped and 5,877 not run, SHA-256
`a2f4c2118f7136c5f607e594638b68acc95fb88edd1e7f308f76cd799b4f4499`.
Two normalized-reproducible repaired ABI 55 wheel SHA-256 values are
`11487ff7a6afb5b7d4ba021cc746dea4003bb646bfc3301b7a74a4b79a674869`
and
`4b493951e935460f9b3d752424f41b60bf58b10315306fdfb3633ffd141981aa`.
Installed Python units pass 105/105 with five expected skips; selected Rust
units pass 497 with 16 explicit live ignores. Strict Clippy, Ruff/Pyright,
formatting and the 108-package dependency proof pass. The remaining 5,858
Python rows and all 48,477 Core JS/TS invocations stay open.

## ABI 54 Page file-chooser checkpoint (2026-10-03)

NODE-286 advances the native handshake to ABI 54 and pins the complete
unchanged sync `test_input.py` file, source SHA-256
`c203939164958aca0babcee63f2d9ae58afce16199fc2b6b5adbc4d6b2284fed`.
The thin async/sync `FileChooser` facade forwards `is_multiple`, Page identity
and payload/path inputs. It does not retain CDP session, target, execution-
context, backend-node or filesystem ownership. `FileChooser.element()` remains
explicitly unsupported rather than synthesizing an ElementHandle.

The optional Rust composition installs
`Page.setInterceptFileChooserDialog(true)` on each selected Page/OOPIF session.
Its event observer maps `Page.fileChooserOpened` through the exact session's
current utility route, creates an opaque action owner and publishes the result
through the existing Page lifecycle fence. The action reuses the shared
`CdpFilePayloadService`: memory payloads and filesystem paths have the same
admission, MIME, encoding and cleanup rules as Locator file input. Direct
upload resolves the event's backend node into a unique utility-world object
group and always releases that group; a dropped public future transfers any
started mutation to the reserved cleanup executor.

The composition admits at most 16 live chooser owners. Each charges one live-
object slot and 2 KiB of backend state; installation state charges 1 KiB. The
owner retains a weak `CdpConnection`, exact session/target strings, utility
context and backend node. This weak route is required: the first strong-
connection version passed the official assertion but prevented fixture
teardown from closing, producing a hang after the test body. The final weak
owner passes the unchanged case and exits normally.

Final repaired-wheel candidate and independent-oracle reports pass 2/2,
SHA-256
`2b2f38ee68d026f3ea1d6eb6f46f8e30edfb8a2a0882921e2cd57622a50d1d38`
and
`0c84f8e75291bdae4bd87685ba9efbc86c12e598b5859a1b7e1f8d7e42e15685`.
Candidate/oracle tranche ledgers are
`f39fc100ccaf2601cf852b32ca3e8c6543f816e0c34b6ff82eb251673f777b89`
and
`ab9953fae7ddd08720f454d84fb5be75ec0b61b3abf48f6bbd95a2eefc37ebea`.
The cumulative candidate is 659 passed, seven skipped and 5,860 not run,
SHA-256
`557e937c00ccc8e9065c37df2c14fde9b7d71ae0c1b9e011f02171b52d71730a`;
the cumulative oracle is 640 passed, seven skipped and 5,879 not run, SHA-256
`d8bb9714f250d46c17c2afd6e5468eb98f8acf8a7a0f84f0a016bffd8d553534`.
Two normalized-reproducible repaired ABI 54 wheel SHA-256 values are
`ff2b00f7524235c3dbdfb1f93f3e29b7d9b3da885b550819c8be2b8df0eb6752`
and
`6584f4b92359e0fd1cddcfaf0bb814980b682eec5a2033457077c97c005d385c`.
Installed Python units pass 103/103; selected Rust units pass 497 with 16 live
tests ignored; strict Clippy, Ruff/Pyright, formatting, repaired-wheel
reproducibility and the 108-package dependency proof pass. The remaining 5,860
Python rows and all 48,477 Core JS/TS invocations stay open.

## ABI 53 Page network-request checkpoint (2026-10-03)

NODE-285 advances the native handshake to ABI 53 and pins the complete
unchanged async `test_page_network_request.py` file, source SHA-256
`6ead05641f44950db30e7ab038c973226ebe726312f39b9d7d7de2aacaf4a388`.
The Python surface adds no lifecycle authority: Context waiters submit bounded
native waits, and Request methods forward an immutable request record plus an
opaque shared history owner. The exact owner that publishes the request is
later settled with its response, so `Request.response()` cannot accidentally
query a different cache entry.

Frame availability stays inside the separately selectable Rust
`request-frame` closure. The Page/session network adapter receives every CDP
event from one cursor, tracks `Page.frameAttached`, `Page.frameNavigated` and
detach events before mapping `Network.requestWillBeSent`, and annotates each
lifecycle effect with the request-time result. The state is finite: 64 Frame
IDs and one boolean for each of the production profile's at-most-64 active
requests per Page, all included in retained-byte accounting. The host no
longer consults a later published-Page snapshot. This matches Playwright's
popup-first-request ordering and removes the regression-only race without
adding a package to the network-events dependency closure.

The final installed candidate and independent oracle pass 2/2, report
SHA-256 values
`c6fd93810787dea30f3beb58ec44202bdff7aad5975f1259852f363faae454cc`
and
`1d0416cb3d1e95cb2eab41e3118b7444fe2e47403a6bd98039d70837b392cba6`.
Three consecutive 10-case NODE-281-through-NODE-285 candidate regressions
pass, SHA-256
`60b7992906b838a93a98f8c61aa2a803dbc7485e9b3a358530aaea3f562b26f2`
and
`ac10e36d1bc833396547276ac1af8e9c79ff0932a5e2eaae9201936621bb34f7`
and
`cfe2c9dc9e7733e09e9b26bce562837a1b0a0dd75fe44347f0a2c6d81c7adac9`.

Candidate/oracle tranche-ledger SHA-256 values are
`e7b97ab2482e90363debe305044ccd21efc08e7eaaa95f368ffb571abe02ba7f`
and
`3e7525c78dabbb19ab31e069074b0fab1258afae97d6e8dfebca627525c481c2`.
The cumulative candidate is 657 passed, seven skipped and 5,862 not run,
SHA-256
`83ed08bf4d1f711e6d506db5a2315bc6e7aed49dff4717d0db95750074c0318b`;
the cumulative oracle is 638 passed, seven skipped and 5,881 not run, SHA-256
`eb03126624a1c047420fa30d1a8ab1b8d5ceacd793bcca2e0f081d495f0b1d96`.

Two normalized-reproducible ABI 53 wheel SHA-256 values are
`da1fcbdee1c0e1bfac900f3d054851942c792b5fd5958d2f15405c1c49d4b41c`
and
`0e51fcf2a4996266d7c19b070b2515fea5f335c00ec3414150b30c6b9f52fad0`.
Installed units pass 103/103; selected Rust tests, strict Clippy/Ruff/Pyright,
Rust formatting, wheel normalization and the 108-package dependency proof
pass. Fresh candidate collection records 6,526/6,526, SHA-256
`66affc14f9da9eb21d94b750855572592ea92d1f0b039fbf6f18cdaeb01687b7`.
All 5,862 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 52 bounded Response completion checkpoint (2026-10-03)

NODE-284 advances the native handshake to ABI 52 and selects the independent
`response-finished` feature on top of `response-events`. It adds no Pwright
package beyond the response-event closure and does not require the separately
selectable request-history capability.

Every published Response may carry a native terminal owner with one of four
states: Pending, Finished, Failed or Closed. Rust subscribes to the Response,
RequestFinished and RequestFailed events for that response context, settles
the owner once, and wakes its waiters. Page/session retirement closes any
pending owner, while dropping an owner returns its fixed admission slot. The
host admits at most 256 owners and reserves 64 KiB total owner state. There is
no Python registry, polling loop, network-state mirror or unbounded task.

Async `Response.finished()` forwards the opaque native owner. The sync facade
submits that exact coroutine once through the existing bounded loop. Successful
completion returns `None`; a failed request raises the bounded native failure;
and retired Page/session state raises the established target-closed error.

The complete unchanged async/sync Page network-response sources have SHA-256
values
`707b20a48b1b89529228697d5a87cb42a965a8afb9a485a5c951839c67dc2cba`
and
`82e3f5cfd0581f92085d1b56c06936fb890627dd505189b4894e36d7949e39fd`.
The initial ABI 51 diagnostic proves the four Chromium rows reach only the
missing member, SHA-256
`08c1a8411006bac153e900a086961b54c23a9828dddff646548e2e7e77154895`.
Final installed candidate and independent oracle pass 4/4 with report SHA-256
values
`98a5f800bdeb271affa5947bd91722e615129835d384ac5cb19e0ae81fe52b00`
and
`807cc17edba597a72a058041ce73de864942c36dbbc2def5179f2c5445fafa06`.
The NODE-281-through-NODE-284 candidate regression passes 8/8, SHA-256
`04ebd36660f93213091f0ac17235007f68fe471c477f3c0c7c96305f4086a65b`.
Eight Firefox/WebKit rows remain explicit provider gaps.

Candidate/oracle tranche-ledger SHA-256 values are
`28ec4b14143f3c74959798890494949f1e6e3cafd85bc09da1be9375717c121d`
and
`ae2b24671287c846f9be434f9a0261780378578a32cbaae39db8570b10ef9197`.
The cumulative candidate is 655 passed, seven skipped and 5,864 not run,
SHA-256
`82dad9ef03972b03a4174dad73b07409b59776839d2e9885a01e81fb35f6f0c7`;
the cumulative oracle is 636 passed, seven skipped and 5,883 not run, SHA-256
`f05f91ebfaafaa1766eef99bda1e2123a87684ad544ce20c715bc5fd6d76b6e7`.

Two normalized-reproducible ABI 52 wheel SHA-256 values are
`6eec71b2bcc8cc2fe658a0e28a5adab537370e74f20ddb62928ec521d23b102c`
and
`bd406d7ea915d9f95cb3d79c7ef13433ae8ef632c6b77d34059a82268b94b3aa`.
Installed Python units pass 98/98; 12 targeted Rust host tests, strict
Clippy/Ruff/Pyright, Rust formatting, wheel normalization and the 108-package
dependency proof pass. Fresh candidate collection records 6,526/6,526,
SHA-256
`73608b054e7dd88df4ca79accebffa6f69561faa30a0868da4d18f5e014c2c2f`.
All 5,864 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 51 bounded Page timeout wait checkpoint (2026-10-03)

NODE-283 advances the native handshake to ABI 51. The wheel selects the
independent `wait-for-timeout` feature, which adds no package to the
108-package Pwright closure. The Rust host reserves 4 KiB of leaf storage and
at most sixteen 512-byte operation leases. Every operation selects between
session stop, transport disconnect and its Tokio timer, so shutdown cancels
outstanding waits without an unbounded Python timer/task registry.

Async `Page.wait_for_timeout(timeout)` validates a finite non-boolean value in
the unsigned 32-bit millisecond range and forwards it to the native session.
The sync facade submits that same coroutine to the existing bounded loop. No
Page, browser or protocol state is implemented in Python. Capability metadata
publishes the 16-operation and 12 KiB total limits.

The complete unchanged `tests/common/test_signals.py` source SHA-256 is
`7f3d9cfc43ca8c5431df25b25ef75f1b38b0f9bac881e893384f622a3dc73267`.
Its first product-reaching diagnostic fails on the absent Page method,
SHA-256
`9bc32a9b769b80391f54801fd91bc9e901020c4f65dafdcd47e4d5f67e4c4ee2`.
The runner also closes two infrastructure gaps: spawned macOS children see the
pinned upstream `tests` package without shadowing the installed oracle
`playwright`, and the oracle launch adapter is installed inside those children.
Candidate and oracle adapters accept only the explicitly configured channel,
then both attach the same dedicated loopback CDP Chrome.

Final installed candidate and independent oracle pass both Chromium rows,
with report SHA-256 values
`15327d9f5ab947fd6c9090f7f33c1be479c55355c6ab317e2cbd9985dae6f91e`
and
`a2b8d0ba5c8f9f3ebc78bcf4293042578af96805a4145305d008a0f7aa01300a`.
The NODE-281-through-NODE-283 candidate regression passes 4/4, SHA-256
`48997deecaa3332fcf8ba1a1cdafd7437fde07bac3b69b7cacd61322d063449d`.
Firefox/WebKit remain four explicit provider-gap rows. Candidate/oracle
signal-tranche ledger SHA-256 values are
`2bd05bee87df53c356065d66262d3688a943f113566596671f4bbaf490e65c4a`
and
`1d3e8009024c507900cd1ae4d15ecf087d61d6ea58f4e895c62797199a2a8971`.
The cumulative candidate is 651 passed, seven skipped and 5,868 not run,
SHA-256
`b64fcdc19f027fd37718a339b122ecb6ea240890e12b4c56c147baac4f3ae0e4`;
the cumulative oracle is 632 passed, seven skipped and 5,887 not run, SHA-256
`48dd00b23f5df8ad5fb26bf3b4a74dc4ac57c55f9b5ac85f54ff38b69c7789e8`.

Two normalized-reproducible ABI 51 wheel SHA-256 values are
`39bf27fbf7d6bdd9d6c1513f11b6c0b37ba2928b2a70c7f921f67593dd3dbfe1`
and
`54b6323011e22a19f6b6beceba01fab317fc1bfbc11869f73f305e88acc1f0f0`.
Installed Python units pass 96/96; the targeted Rust capacity test, strict
Clippy/Ruff/Pyright, Rust formatting, wheel normalization and dependency proof
pass. Fresh candidate collection records 6,526/6,526, SHA-256
`d2b169846ada9162934a60b9e542e99f55096f43fa5a1a2d988cf2d935e372c8`.
All 5,868 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 sync event-error propagation checkpoint (2026-10-03)

NODE-282 keeps the native handshake at ABI 50. The sync loop owns one
lock-protected pending-listener-error slot. A wrapped Page handler records the
first immediate exception and rethrows it into the async event layer, which
retains its existing diagnostic logging. The sync call always completes its
submitted coroutine before taking and raising the pending exception. This
preserves cleanup while preventing the listener error from disappearing into
an event-loop log; later errors cannot create an unbounded queue.

The complete unchanged `tests/common/test_events.py` source SHA-256 is
`34a3a26326764909a9fd0e1b97282ea5ca689c4a8c45f142725d10c42ee519ce`.
The initial installed-candidate diagnostic invokes the listener but fails with
`DID NOT RAISE`, report SHA-256
`f694bc243739cb29f7e83140749b89f64432a1dec489870f2617793a8199264d`.
Final installed candidate and independent oracle pass the Chromium row, with
report SHA-256 values
`b0cb31378655ab7131bda8d57255e28b8378f2ace1275933e61230aa173c4a42`
and
`940665a60c4023e1f2e05a32fa3045339f6052406ddc71df1a7f88b2ffab4fef`.
Firefox/WebKit remain explicit provider-gap `not-run` rows. The two-file final
candidate regression passes NODE-281/NODE-282 2/2, SHA-256
`04e6bf146f45099b4b56f3ab80f78daadeec5a580185b67ab7c79aeeeeec046e`.
Candidate/oracle event-tranche ledger SHA-256 values are
`5d0c9e0aac75f181785220fab4962664e562c815b2f9ceed0933cd320a0de2a5`
and
`f86168a3b461572ae0a8e2bbe6a99ec5503c416fcb89e9551cac77be9ba3b846`.
The cumulative candidate is 649 passed, seven skipped and 5,870 not run,
SHA-256
`3e75c9588a73cd741ac571800a430e290affb178aee3bed5ab0951b46987602d`;
the cumulative oracle is 630 passed, seven skipped and 5,889 not run, SHA-256
`c895df37d1112f67e24f5e31b8d7bb2a13982261e792b3d45c966faf65e05b6e`.

Two normalized-reproducible ABI 50 wheel SHA-256 values are
`d70c33a033f01ff57dddcf6cee6bca90b31203e62e9a67cf6877b669202a051c`
and
`5e09beec25d3982f14a70550475fb0bb6f0c6bcf50c6aeba92e364181678ea7d`.
Installed Python units pass 94/94. Ruff/Pyright, exact wheel-profile Clippy,
Rust formatting, normalized wheel comparison and the 108-package dependency
proof pass. Fresh candidate collection passes 6,526/6,526, SHA-256
`5a8c37f74beaee9487a5028138ae35af821bdd18ffa731d776251550e5587f7a`.
All 5,870 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 cross-thread sync lifecycle checkpoint (2026-10-03)

NODE-281 keeps the native handshake and product implementation unchanged. The
complete upstream worker thread enters and exits `sync_playwright`, attaches
Chromium through the symmetric launch adapters, creates a convenience Page,
closes the Browser and reports success. This verifies the existing bounded
sync loop and Rust-owned Browser/Context/Page cleanup across the public thread
boundary; it does not introduce an unbounded thread or a Python-owned browser
runtime.

The complete unchanged `tests/common/test_threads.py` source SHA-256 is
`8e42fa88bfa47b5f6ba501c2731d3d9c1812fddb28b4451aba923fb93bfe8c64`.
Final installed candidate and independent oracle pass the Chromium row, with
report SHA-256 values
`1fdf1cf683d7ca5bba9f12db51f81bf2fb083d1d230520a25117f43c39ab254a`
and
`4b10bc1a7187c08b13e25a639cbd33baa45306edc04f0c6916913ddc3c48de59`.
Firefox/WebKit remain explicit provider-gap `not-run` rows. Candidate/oracle
tranche ledger SHA-256 values are
`ee364ba1464514d18d20d6160a14062b048c9c967bee4867760a86152e5aef09`
and
`c61fc526cfc53f6927d9322c0aa4c22944464186ae4516952da809674ff22a42`.
The cumulative candidate is 648 passed, seven skipped and 5,871 not run,
SHA-256
`4febdfe050081de20ef509f153f4e77669eb2672d22be1a07853d9e2e1916345`;
the cumulative oracle is 629 passed, seven skipped and 5,890 not run, SHA-256
`d9fb4a9faf432e4cc60c473b373ac9076afb98eb0dd4d9ee3ed62beb0d14b06e`.

No wheel was rebuilt because no product code changed. The installed
normalized-reproducible ABI 50 wheel pair remains
`0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
and
`db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
Fresh candidate collection passes 6,526/6,526, SHA-256
`c4871850fb0b392fbabcac9feef6d144d50dc68e0233597d2207ca22e0da04dd`.
All 5,871 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 test-id strict-error checkpoint (2026-10-03)

NODE-280 keeps the native handshake at ABI 50. The Rust selector registry owns
a 1,024-byte-bounded test-id setting. Semantic test-id query construction
takes an immutable snapshot while action-time strict diagnostics read the
current Playwright-scoped setting. Async/sync Python expose only a thin
`Selectors.set_test_id_attribute` forwarding method.

Strict action errors recognize the exact portable and CDP CSS strict prefixes,
then use Rust-owned non-strict collection evaluation to produce at most five
previews. Each preview retains at most 512 HTML UTF-16 units and 256 test-id
UTF-16 units. Preview failure preserves a Playwright-shaped base strict error
instead of hiding the original action failure.

The complete unchanged source SHA-256 is
`60ae2e5324fd30a820402d4feaa28dac2243caaef227a28ad9b1773e3d403b8d`.
Independent oracle and final installed candidate pass the Chromium row, with
report SHA-256 values
`881bcdbf424fab522b7eccd9ba0a4c9555b419b209efe29e025a659d6c1b6c2f`
and
`c38cfd4ee833a79dd43ae44e6574ea15d3cf10c3321846c088bdcc570f597537`.
Firefox/WebKit remain explicit provider-gap `not-run` rows. Candidate/oracle
tranche ledger SHA-256 values are
`4fff7e510e7cc37945776930e3bc03a2e133cf00d5a1d600ea729b79884c4c2b`
and
`ecb0fc1580fd76e5dc05245fe9fe66b0d9014ac00e53230cb4a8e9dbd6079ff4`.
The cumulative candidate is 647 passed, seven skipped and 5,872 not run,
SHA-256
`38ab5abac559594a16e9faedab66525f7a168e4bddcf765da1b294a30139a10d`;
the cumulative oracle is 628 passed, seven skipped and 5,891 not run, SHA-256
`27c233a777873c1b97d4cb503768603de325c23c23c0d699b322bd130575a6a6`.

Two normalized-reproducible ABI 50 wheel SHA-256 values are
`0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
and
`db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
Installed Python units pass 93/93; the all-feature host passes 895 tests with
26 explicit live/diagnostic ignores and the addon passes 2/2. Strict release
Clippy, Ruff, Pyright, normalized wheel comparison and the 108-package
dependency proof pass. Fresh 6,526/6,526 candidate collection has SHA-256
`ea207f49af313c2b31f6c26d85ac6cd1e63d66f3afa7f24ad160b7c1b4e01499`.
All 5,872 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 legacy text selector checkpoint (2026-10-03)

NODE-279 keeps the native handshake at ABI 50. Public quoted `text=` and bare
quoted selectors compile to `LegacyText`; public regex text compiles to a
typed regex selector; internal semantic text retains its descendant-text
behavior. The Rust browser runtime evaluates legacy exact matches against
immediate normalized text-node chunks. A zero-allocation selector-chain
iterator preserves `>>` inside quotes, backticks, escapes and regexes.

Async/sync `Page.query_selector_all` reuses bounded Locator handle publication
with a 64-handle maximum. `Page.query_selector` and Page selector evaluation
use Playwright's non-strict-first behavior without falling back to raw DOM CSS;
`Locator.evaluate` remains strict. `to_have_id` delegates to the existing
Rust-owned attribute expectation. Selector protocol 1.1 and Locator protocols
1.24/2.7 carry the new variant; older 1.23/2.6 decoders reject it explicitly.

The complete unchanged source SHA-256 is
`d8b841099bddcb972bf81846f050cac8880f65fe5c6f73b1c44a26d556d39332`.
Independent oracle and final installed candidate pass both Chromium rows, with
report SHA-256 values
`d3204cf9be77ba876630627f7f9a81f88017bde7f54f64ebbe7b2f3f58828efa`
and
`214264e2a9babd5277c448a82d86ba11ace901635120cdd047eb4437a76595c3`.
Four Firefox/WebKit expansions remain explicit provider-gap `not-run` rows.
Candidate/oracle tranche ledger SHA-256 values are
`ed9435fad8c7248c38d19f726a7afc1afe3c1d3d3f7a7125d42913bc1c049c87`
and
`0e3764ca45a20c458185b06f5ec83b4942eb723019dcf6f746aff19789d224c6`.
The cumulative candidate is 646 passed, seven skipped and 5,873 not run,
SHA-256
`603945f33ceadde881b175508db8402092fe71f246b3a025ea2b9277227fde25`;
the cumulative oracle is 627 passed, seven skipped and 5,892 not run, SHA-256
`d19ab2d5f567fd275b46cb93e37b07dd631db0175345208fe3365fd0a6c2e98a`.

Two normalized-reproducible ABI 50 wheel SHA-256 values are
`8c2739a837b625d8a8da4f394752dbf6e40f7d39a62a2b319baf3c1f9a063f85`
and
`45266c5d9c8ad7e2f3c4552cacb460365640447cd6131777030d83d25c134e04`.
Installed Python units pass 92/92; the all-feature host passes 891 tests with
26 explicit live/diagnostic ignores and the addon passes 2/2. Strict release
Clippy, Ruff, Pyright, schema generation/checks, wheel normalization and the
108-package dependency proof pass. Fresh 6,526/6,526 candidate collection has
SHA-256
`9d2d60c194ec015e2e141a29eac2479063bef342f22dbc80c4ed1afe8228f283`.
All 5,873 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 screenshot checkpoint (2026-10-03)

NODE-278 advances the native handshake to ABI 50. Async/sync Python expose
Page, Locator and ElementHandle `screenshot` methods and perform only public
argument validation, path/type normalization and wrapper-to-native identity
conversion. The native host reuses the finite screenshot capture queue and
bounded output lease. It admits at most 64 mask Locators and 256 matches per
Locator, installs transaction-scoped closed-shadow-root overlays, and cleans
them up on success or failure without replacing an earlier capture error.

Locator capture keeps selector resolution, stability, scrolling and clip
translation inside the existing Rust owners. ElementHandle capture retains
the exact native handle, measures its document coordinates, marks the internal
clip as document-space and lets the content backend choose
`captureBeyondViewport`; the internal flag is not accepted by the public
Locator codec. Page capture shares the same mask and output owners. This keeps
browser semantics and resource accounting out of Python.

The complete unchanged async source SHA-256 is
`1ffd1344ba75e3f481b3d448373791145737757503a14f4e55a84288e1a00840`.
The ABI 49 diagnostic fails 3/3 at the missing Page method, report SHA-256
`f38c3a716fae71f63efcc81352fdf4e3805a14e03a3e42ca228f3f89b44e0b59`.
Independent oracle and final installed ABI 50 candidate pass all three
Chromium rows, with report SHA-256 values
`d3f45897a07e19a2ac3aa2ac7a737bbaf67d1b3088bb12b8d819632142ca11e4`
and
`ad7e0c2b043641fa39a4fcac775945a193c4fef63a507f17a4572150bd270c58`.
The six Firefox/WebKit expansions remain explicit provider-gap `not-run` rows.

Candidate/oracle tranche ledger SHA-256 values are
`46b1f1fafba03863f5da38fb691e1afa37296b590990d5c750ee1c9153fd22d3`
and
`06c53a9fa7b23cd239f0bb9a8189bad35a6576cfdc7450b52daaa41c1364a6d3`.
The cumulative candidate is 644 passed, seven skipped and 5,875 not run,
SHA-256
`f898f89d210cfb1c73d37b5817b68688050fae9f273c54c92480f4e0375aaf5c`;
the cumulative oracle is 625 passed, seven skipped and 5,894 not run, SHA-256
`aa53c89869c367c05976854317d9d0e50964b6cc7b0617602eba54af5a4b84b8`.

Two normalized-reproducible ABI 50 wheel SHA-256 values are
`21017fae60321b12ab56bcc1b000e01e18c8892caa48e085a5622fa84a1da9ce`
and
`6d8369d2bb334fb28f650f244172a269bb2f0f694e14c42c5f4b7f06cbc3dfd7`.
Installed Python units pass 88/88; the all-feature host passes 886 tests with
26 explicit live/diagnostic ignores and the addon passes 2/2. Strict Clippy,
Ruff, Pyright, the 108-package dependency proof and normalized wheel comparison
pass. Fresh candidate collection records 6,526/6,526, XML SHA-256
`6980ab9cfc863becc420c497f98a2a9aec3622935dfd0b542092dcc32288d0cf`.
All 5,875 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 49 tagged PDF outline checkpoint (2026-09-28)

NODE-277 keeps the same independently selectable `page-pdf` capability and
advances the native handshake to ABI 49. Python accepts only the newly
installed optional `tagged` and `outline` booleans, validates their public
types, and forwards normalized booleans. Rust maps them to Chromium's
`generateTaggedPDF` and `generateDocumentOutline` request fields. Page/session
routing, `ReturnAsStream`, bounded `IO.read`, `IO.close`, the 2 MiB retained
output lease and optional path persistence remain in the existing Rust owner.

The complete unchanged async source SHA-256 is
`3a43a35697d70ad4a9ce6cec014a906a47e35b05471756e6eb42f403ca129188`.
The ABI 48 diagnostic passes capture/save and fails only the tagged-outline row
at the explicit uninstalled-option boundary, report SHA-256
`59e0bff3910ee10e21c2d61a11f6ddd6bc5fb04a5f88f7e63defba159848b33b`.
Independent oracle and final installed ABI 49 candidate pass all three
applicable Chromium rows, with report SHA-256 values
`647a7ca744aa2a3899d3f3756d1b7c5df38a213faf4f4e052b4b99948d3a6077`
and
`248379bc35eb7c2b78e6e3293ec7a7b9ada3a2f4327883765880e03153ea3355`.
The six Firefox/WebKit expansions remain explicit upstream Chromium-only
non-applicable `not-run` rows.

Candidate/oracle tranche ledger SHA-256 values are
`75bdbdbf039c1238e80219a7a7aae0f6d828fa87077cef35b2c21a845a4142cf`
and
`c67b9b11e5b598023ef28f3d7f6ef5ca63cc9742cab089acf94985e33f11c653`.
The cumulative candidate is 641 passed, seven skipped and 5,878 not run,
SHA-256
`311867644fb8f62e11af82b1cdb08ce58392f1a4dfe2a03bc0bd7cc1ff387cd8`;
the cumulative oracle is 622 passed, seven skipped and 5,897 not run, SHA-256
`895826486a6d1831cbabaf2eef041d822ab520836232682b680d69d1df01ccfe`.

Two ABI 49 wheel SHA-256 values are
`e837346a03487651337816b4adb8e93c6f48860b94252be8d00c8a33088d7e34`
and
`68ecf94ec7845f836f0b09dd5f8dffd8fab64ed0e64cbd49029f77fcba9cec28`;
their normalized payloads are reproducible after removing only permitted
CycloneDX identity. Installed Python units pass 86/86; the all-feature host
passes 883 tests with 26 explicit live/diagnostic ignores and the addon passes
2/2. Strict Clippy/Ruff/Pyright and the 108-package dependency proof pass.
Fresh candidate collection records 6,526/6,526, XML SHA-256
`bad14cc3e732be2f400aa34ce4a27f2b447b2f121193ccd3294da3a13c2a0007`.
All 5,878 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 48 Page PDF checkpoint (2026-09-28)

NODE-276 advances the native handshake to ABI 48 and adds the independently
selectable `page-pdf` feature. `pwright-node-host` resolves the exact Page and
CDP session, reuses the finite per-Page capture queue, dispatches the exact
default `Page.printToPDF` `ReturnAsStream` command, runs a bounded `IO.read`
loop, decodes raw/base64 chunks and always issues `IO.close`. It reserves the
2 MiB document ceiling before streaming, shrinks that lease to exact output
and admits at most two concurrent output owners. The PyO3 layer optionally
persists the bounded bytes to the normalized path; async/sync Python only
reject uninstalled keyword options and forward Page/path values. The feature
adds no Pwright package to the 108-package base closure.

The unchanged sync source SHA-256 is
`38c95f81587739897da1e364e0f18e6964a7db327775d0deccc7b76c49684520`.
The ABI 47 diagnostic fails the two applicable Chromium rows with missing
`Page.pdf`, SHA-256
`677f6e364d147cf8c03837544076285a620fdac02cf561eb5eec9fd1962b3476`.
The independent oracle and final installed candidate each pass 2/2, report
SHA-256 values
`036bffd3a26facd620b478f49ec81f98a70f01274cd51c1d925d370e1975d908`
and
`c36fb38329822687850d1900d4b24a4b10361d423011ea8efea3c9adf619ac3e`.
The four Firefox/WebKit inventory expansions remain explicit upstream
Chromium-only non-applicable `not-run` rows.

Candidate/oracle tranche ledger SHA-256 values are
`18b846b9dc946359f730e4fa6e1f74faad79747975c9fa72621e113bc26b1e45`
and
`54635dc63be374697eebd7a068ed199ea8e0f5f0e3b00505d3c7f6b1d2c1270a`.
The cumulative candidate is 638 passed, seven skipped and 5,881 not run,
SHA-256
`801181534ae92df917d69d02f0d03708981c14ab8a171fefa7086e12acae3b02`;
the cumulative oracle is 619 passed, seven skipped and 5,900 not run, SHA-256
`a9f7b78946734baef302d23a8cada3da5d3d707865957a41fcdb1e3041828a01`.

Wheel SHA-256 values are
`48593cfdd4a22249e2eee14186faceb95235670657e52aa808680d6e9a2fe348`
and
`b9acf998197b093ec2597c996193e6b3256fc97dfa23e73f3299de85c59bd2af`;
their payloads are reproducible after normalizing only CycloneDX identity.
Installed Python units pass 85/85. The all-feature host passes 882 tests with
26 explicit live/diagnostic ignores; the addon passes 2/2. Selected and
all-feature strict Clippy, Ruff/Pyright, the dependency proof and fresh
6,526/6,526 collection pass. Collection SHA-256 is
`3d7e92c62715dbc8dafb3d15cb9af030e0876c2e778e80fde9b13ac97cf95645`.
The earlier ABI 48 base64-return pass is retained as a superseded diagnostic;
it is excluded from the final report, tranche and cumulative ledger.
All 5,881 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 47 ignore-HTTPS-errors checkpoint (2026-09-28)

NODE-275 advances the exact native handshake to ABI 47. The
`ignore-https-errors` feature installs a fixed-capacity 16-Context policy
registry in the Rust host. A policy-bearing Context initializes every newly
owned Page/OOPIF session with `Security.setIgnoreCertificateErrors` before
that session is published, and successful Context close removes the registry
entry. Context-creation failure compensates by closing the native Context.
Python performs strict public-boolean validation and passes the option through
PyO3; it does not replay sessions or retain TLS/browser state.

The complete unchanged async source SHA-256 is
`4209a74e94509e340b5e22f2e057541473835321bcdd81a49788f48cfdf3fd3c`.
The ABI 46 candidate diagnostic passes the default negative row and rejects
the positive row as unsupported, report SHA-256
`cb5e9506dbf789968653f5525dd58f74689c26dae80110fa055e9c68efa69bd5`.
The final installed candidate and independent oracle each pass both Chromium
rows. Report SHA-256 values are
`cdd4eb40b5c8c40ede8c84f73056c5faf229837c497bf889d66545612510bea8`
and
`1f3179fc8f7deee06eb07ea775d238c9896bad297dd74f0afa1732732c4bef59`.
The four Firefox/WebKit expansions remain explicit provider-gap `not-run`
rows.

The candidate/oracle tranche ledger SHA-256 values are
`7c01e4a2ba7309e4217c2904543031bc7cca71bdf77d34c7ec2f257ff7786a97`
and
`20dd4bfaf1a218151b3aa4b800d0eefc5e869d4d3ef61b92dfdb6b13675ffa7e`.
The cumulative candidate is 636 passed, seven skipped and 5,883 not run,
SHA-256
`690f4b86c509932323cd613f2f40fede63a06e86133209bf98944421ae3d70d8`;
the cumulative oracle is 617 passed, seven skipped and 5,902 not run, SHA-256
`647b129c3901df6eb3337e7d886e88f8cbc7322653a71b38d7e99ac1c65bd204`.

Two repaired wheel SHA-256 values are
`4074ac94f00f9463cf337e67af4b73671cf9d152ea1f6582735c9ad3265d49e7`
and
`5f4a2a2d71a0c6290344bd35b2b5ac2305e7653e21ce466d202e7dc584cdba7c`;
their normalized payloads are reproducible. The installed wheel passes 83/83
Python units. The all-feature host passes 879 tests with 26 explicit
live/diagnostic ignores; the addon passes 2/2. Strict selected/all-feature
Clippy, Ruff/Pyright, the 108-package dependency proof and fresh 6,526/6,526
candidate collection pass. Collection XML SHA-256 is
`27895f0aaceecb67c057553193581e6d931b7c750059c18ba88bf2546b3e8fb7`.
All 5,883 candidate Python rows without outcomes and all 48,477 Core JS/TS
invocations remain open.

## ABI 46 fill and context-manager checkpoint (2026-09-28)

NODE-274 changes no Rust capability or native ABI. The complete unchanged
async/sync fill files exercise five Chromium rows through the existing
Rust-owned Locator fill owner; candidate and oracle each pass 5/5. Their report
SHA-256 values are
`36d2010856b1863f656c33ee6acd760b7afc04efbcf52b8e1e064e51d5d1a2d0`
and
`68b88b17bf5f1f3e6bbb59d7579a362b5b66a8c3d7c6126efbddce244a40665e`.

The complete unchanged async/sync context-manager files initially distinguish
two separate discrepancies. Candidate lacks the Python language protocols and
fails 4/4 before the close owners are reached. Oracle passes 2/4 because its
CDP `launch()` fixture adapter exposes the already-attached default Context;
that is adapter distortion rather than an official Playwright failure. The
diagnostic reports remain preserved and are not merged into the canonical
ledger. Page, BrowserContext and Browser now expose thin async/sync enter/exit
methods; exit delegates to each existing close implementation. The corrected
oracle adapter hides only Contexts that predate the adapted launch, symmetric
with the candidate fixture. Candidate and oracle then pass 4/4. Final report
SHA-256 values are
`6f74927d6e6b67aef4e2387bab2b8f7981a7caf60d787a06a359089fca6910b5`
and
`22d67c00f17238be8514532a357afe1c5567658f0494df71646b58aa993b4679`.

The combined candidate/oracle nine-row tranche ledger SHA-256 values are
`8c727149b0e6c9b2b8712845a3a55dc81413a0888a4cd2799193246f8f95e0ab`
and
`db9950cb4d39a64ddc86e4adaf9fedf40f25202ef8852172e4811254debb3651`.
The cumulative candidate ledger is 634 passed, seven skipped and 5,885 not
run, SHA-256
`8bb2f5b5437477779c6eb6690da2f247d3f377422f62b1d8bd3ee0cafb5f3908`;
the cumulative oracle is 615 passed, seven skipped and 5,904 not run, SHA-256
`b02be1e0e0f789d575725098853e060d596514ddc7031fd2c0060883a53d080d`.

Two normalized-reproducible repaired wheel SHA-256 values are
`6f2f3e4668083b081608d8ce92e1cdbe1f6040ea8d225f4bcfff44c9a62bc3ed`
and
`51c25e0d628d2dac7acf07c596c6815637000b86efba9056d6eac504d6b9096e`.
Installed Python units pass 81/81. Strict Ruff/Pyright, the 108-package
dependency proof, Rust formatting and fresh 6,526/6,526 candidate collection
pass; collection XML SHA-256 is
`889533a3ce28682f6ba5f1f10c23b3b16bad8618c985bb3e51eaf889aedff168`.
NODE-273's unchanged ABI 46 Rust host/addon/Clippy gates remain green. The other
5,885 candidate Python rows and all 48,477 Core JS/TS invocations remain open.

## ABI 46 native Locator collection evaluation checkpoint (2026-09-28)

NODE-273 adds one native method and advances the exact handshake to ABI 46.
`NativeLocator.evaluate_all` calls the existing `HostLocator::evaluate_all_with`
owner, so the immutable typed semantic recipe is never decoded or concatenated
in Python. Rust keeps collection resolution, browser-world execution, FFI
capacity and value-tree bounds; the Python async/sync facades only forward and
adapt values.

The pinned unchanged async source SHA-256 is
`edc40e9699d9092abc5a5a9ec0e5849933d46b028dbdda6601d8a5b1f9ca6269`.
The initial ABI 45 candidate passes 1/3; the two role rows fail at the missing
native collection-evaluation path before testing role semantics. The final
installed ABI 46 candidate and independent oracle each pass 3/3. Report
SHA-256 values are
`cc1b66557ae5510345f4dae1163f86a4dce8af8c344d81530586fc0f3fdddf12`
and
`c0150e1af71e0e262d2a4edf5c5bfa3463aaef0272a5f6bac4aa00e975b32d02`.
The candidate tranche ledger SHA-256 is
`8070a6116e7b900319f4199e010ef4af2a285b050a5090abfa331c1c337a217a`;
the cumulative candidate ledger is 625 passed, seven skipped and 5,894 not
run, SHA-256
`39fc2c66a96066e5d0275b146688a48a7dbb1e8a3aeaf37f3971d598cbee4e4b`.

Two normalized-reproducible repaired wheel SHA-256 values are
`a665627bdb6bdda5ce0e8e2a72a37999b222c10ffa5e8ba44637f0f52da52900`
and
`e7a4c8933d3167ad24fea61e3e6354fa06a0df8cb6ccbb44662cee467712b20d`.
Installed Python units pass 79/79; all-feature host tests pass 877 with 26
explicit live/diagnostic ignores; addon units pass 2/2. Strict release Clippy,
Ruff/Pyright, the 108-package dependency proof and fresh 6,526/6,526 candidate
collection pass. The other 5,894 candidate Python rows and all 48,477 Core
JS/TS invocations remain open.

## ABI 45 semantic Locator checkpoint (2026-09-28)

NODE-272 advances the native handshake to ABI 45. Page, Frame, Locator and
FrameLocator expose the existing typed Rust text, label, placeholder, alt-text,
title, test-id and role recipes through thin async/sync facades. FrameLocator
derivation is native, so Python neither concatenates selectors nor stores DOM
or frame identities. `to_have_attribute` reuses the shared native expectation
loop: Rust owns the total deadline, serial retry, missing-element behavior and
exact/regular-expression comparison, including explicit `ignore_case`
precedence.

The pinned unchanged sync source SHA-256 is
`a7c0133231235e63ba397175265cdef5ccd221d96b40181e0d9ff2d6a0325f19`;
there is no async source file with the same name in Playwright Python v1.58.0.
The complete ten-row Chromium file passes 10/10 in the independent oracle and
final installed candidate. Report SHA-256 values are
`62ec0bb876133ab0fc1be15665e5b637215c0120e263de6fdf5ffc9cb23118f5`
and
`95362db9ab9b86874567fcee6bda9871f5c23e849005c0e9d03023ffc07abc73`.
The candidate tranche ledger SHA-256 is
`e159fec1a40bf0ab0c157e4595890c6caab343cb0a5a2aee7e19bce8fc776932`;
the cumulative candidate ledger is 622 passed, seven skipped and 5,897 not
run, SHA-256
`e37434aa74c4dea2f6f8001ae27d657e833cf0c197207b0186b5d133b9a40813`.

Two normalized-reproducible repaired wheel SHA-256 values are
`e0321e22e0ab05b57608f48d180689009d6113a55525a0b81a7a5776e04b71b0`
and
`0c30faaf48699e0b95426204f8c28e3e868abb5e94b2dccc004050ba61ac098b`.
Installed Python units pass 78/78; the all-feature host gate passes 877 tests
with 26 explicit live/diagnostic ignores; addon units pass 2/2. Strict release
Clippy, Ruff format/lint, strict Pyright, the 108-package dependency proof and
fresh 6,526/6,526 candidate collection pass. The other 5,897 candidate Python
rows and all 48,477 Core JS/TS invocations remain open.

NODE-270 advances to ABI 43. `Locator.bounding_box` reuses the existing Rust
geometry owner. The independently selectable `locator-screenshot` host profile
reuses the existing screenshot capability and codec while keeping selector
resolution, stability, scrolling, iframe clip translation, the 2 MiB byte
ceiling and two-capture concurrency bound in Rust. Python validates only the
small public enum/numeric option shape and copies the bounded result.

All four targeted official rows pass. Complete Locator execution is 77/86
async plus 68/77 sync, 145/163 total, exactly four gains and no losses. Report
SHA-256 values are
`04cc14bf1adfd352d8e3eb5cf1139b9b8e82e239229be596bf7b4d8d69e74720`
and
`437120717f7314a7bdd65df31720c3bcb81f033989f2d749c62118e2dfc66f7e`.
The Locator ledger is 145 passed, 18 failed and 6,363 not run, SHA-256
`1276b628d8b778e7c947bb06fb5f32f43ff98ee064bce3e958e97ff1d88e6169`;
the cumulative 6,526-row ledger is 594 passed, 18 failed, seven skipped and
5,907 not run, SHA-256
`811c14419d0fee688717168bfb0118d07cfaa82c27d9d8c2e0f14650d01348fe`.
Strict release Clippy, Ruff and Pyright pass. The selected host gate passes 635
tests with 24 explicit live/diagnostic ignores, installed Python units pass
66/66 and addon units pass 2/2. The 108-package dependency proof passes. The
two normalized-reproducible ABI 43 wheel SHA-256 values are
`14924b545aaa14e9a4096897d59cc7cce45cd1fd8c3b5d07ef2ac3a7ef8b11da`
and
`fcf9958339d7f54ec190d85227f957d057abc0c253c6e5f52cf44cad1f852fa9`.
The remaining 16 FrameLocator and two callback-completion discrepancies and all
unexecuted Python/Core rows remain open.

## ABI 35 complete Locator diagnostic and action checkpoint (2026-09-28)

The exhaustive NODE-258 baseline runs 86 async and 77 sync Chromium Locator
rows. Candidate 60/163 versus oracle 163/163 establishes 103 real
discrepancies. NODE-259 keeps the files intact and moves 19 rows to pass by
adding bounded `all`, Frame-rooted Locator creation, immutable descriptions,
strictness wording and Rust-owned blur/hover/clear/select/scroll/sequential
typing actions. The final stable candidate is 43/86 async plus 36/77 sync.

NODE-260 changes attach-time ownership, not Locator semantics. Initial
request-history and response-event activation selects only Contexts owned by
this host, unless the Rust host owns the browser process itself. This prevents
an unrelated long-lived page from exhausting the candidate session's bounded
active-request table. Page-scoped network waits remain exact and demand-driven.
An attached borrowed Page's callback/history compatibility requires a later
demand-driven activation API and remains explicitly open.

The complete evidence, hashes, ledger totals and remaining clusters are in
[Official Playwright Python conformance](python-official-conformance.md#complete-locator-diagnostic-and-repair-checkpoint-2026-09-28).

## ABI 35 typed Locator composition checkpoint (2026-09-28)

NODE-257 advances the native ABI to 35 by exporting a frozen `NativeLocator`
and `NativeSession.locator`. The native value retains the Rust `HostLocator`,
its environment/query-budget owner and lifecycle callback. `locator(selector)`
admits a bounded leaf query and combines it with typed
`QueryCombination::Within`; `first`, `last` and `nth` derive typed recipe
nodes. Reads and count expectations execute from that retained owner. Python
does not serialize a selector chain, interpret selector grammar, run a retry
loop or retain browser/protocol state. Description remains immutable facade
metadata copied onto the new Python wrapper.

The fixed six-row manifest SHA-256 is
`78e5b01d4582222168c51fea88add9a89987374914de5bac4711e2a7e4d4755f`.
ABI 34 initially fails 4/4 async and 2/2 sync with missing methods; diagnostic
report SHA-256 values are
`b2795c2e534a7dc84e5635f4d789476af1e5384b1db89acfe297ef07259185a7`
and
`c2acd8c0b15cec16305441b62ed5c964e6ba6173ec473a770f3d51ba8e0e1b86`.
The final ABI 35 candidate passes 4/4 plus 2/2, report SHA-256 values
`2090acd65225bd9dee0bfd8e6cc61dc8adf6cfe943d21dfaae6cac456717834b`
and
`f6e70033b933f2393a73e581ff041fc06e099dc920251f36470688f6e2ace53b`.
The independent oracle passes the same rows, report SHA-256 values
`5ca3b4dea00d65d1372017e17f26eaa1cabe9a7df43ca98c03d3faee46ac5576`
and
`9d709ae09e8795e64ff300ec18ffff7e8216d09c1df46aaee08d6889f4f00327`.

No Cargo feature, Pwright package or dependency edge is added. Final repaired
wheel SHA-256 values are
`d95c176835f2e557d7ff6780e398ae0e8db372a8edb9fa608b113de3c20afa11`
and
`2a33c5cba4b0379e74673f302190b6e3ec829beb7a29031de77999a3c73aa47a`;
normalized payloads are reproducible. Installed unit tests pass 39/39, Node
adapter tests pass 270/270, strict Rust/Python and 108-package dependency
gates pass, and full collection is 6,526/6,526 with XML SHA-256
`f88de9f75105191dba6db541ba21dd5e69fc3e67610b3da29562454d728e1f7c`.

The candidate ledger is 505/7/6014, SHA-256
`791513dec404b04ed5521aa031524c418703dc78f2f59adced1016c0efed07a7`;
the oracle ledger is 486/7/6033, SHA-256
`efdd8cd287445c9dadd84942ebd64f4e2bb9f565f06928bda3c4eeaa3423d775`.
Exactly six intended rows change from NODE-256 in each lane. Full Python and
Core JS/TS parity remains open.

## ABI 34 Locator description checkpoint (2026-09-28)

NODE-256 adds no native export, feature, package or dependency edge.
`Locator.description` is optional immutable Python facade metadata and
`Locator.describe` returns a new wrapper over the same Page and selector. The
operation issues no browser/native call and retains no protocol or resource
state. Chained Locator composition is deliberately not encoded as a selector
string in Python; its two description tests remain open until the typed Rust
query owner is exported to this binding.

The async-only manifest SHA-256 is
`d578cddad1cbb56d29d098b4fb1c19a20bd1d24a739809a25bdacb03d6591374`.
The pinned sync source has no equivalent tests. The initial candidate fails
3/3 with missing members, SHA-256
`6108a1105a88af0f21ef27c6f298485fc86812b4e5d36f9a84dd1d2cdbb1da55`.
The independent oracle passes 3/3, SHA-256
`b0ae068ac5dbce8747cddffd41601a250fb2da163f77f64d43973714a1fd1764`,
and the final candidate passes 3/3, SHA-256
`e6ffb953b1f1877017f4203e170f257a21456e576aaed922bbb9150086e1dbf0`.

Final repaired wheel SHA-256 values are
`5d8b1d17925414c497f5fa81a0fcba7d769e1a997700709e5624015d95c1dd7f`
and `f43a4098f6d5e9bccf25cf403c9a8798e4aa1db8eea63e9f653ae6b308923231`;
their normalized payloads are reproducible. Installed tests pass 38/38 and
full collection is 6,526/6,526 with XML SHA-256
`c394acf10375f0239312607628925b40fba8f3b5ff95a091d4493ac2fae398e2`.
The candidate ledger is 499/7/6020, SHA-256
`c5e89f60ce3e252855f80dd5ca8bcd9444cb089f62b61cb60eee04586d29cc67`;
the oracle ledger is 480/7/6039, SHA-256
`100d1c8ef02411cbfd506042600f9b7c90ea54f53b74478c571fc276a7a151f6`.
Exactly three intended rows change from NODE-255 in each lane; full-suite
parity remains open.
