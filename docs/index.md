<!-- agent-updated: 2026-10-10T07:11:56Z -->

# pwright - Project Guide

The [Core conformance checkpoint](upstream-playwright-conformance.md) now
records the exact 15-case browserless BrowserType matrix with 11 expected and
four upstream skips in both candidate and oracle lanes. An additional
12-case Library preflight matrix passes in both lanes; the latest strict
Core ledger still has 46,861 candidate `not-run` rows. The
[Python checkpoint](python-official-conformance.md#cross-engine-browsertype-metadata-checkpoint-2026-10-10)
adds four paired-green Firefox/WebKit name rows. ABI106 now matches official
path strings, but executable-path existence cases remain red diagnostics.
These selected outcomes do not establish full-corpus parity.

Core Page response-wait NODE-432 matches all nine unchanged Chromium Page
outcomes in candidate and independent oracle lanes: eight expected plus the
same Chrome-154 cached-image unexpected. The bounded Rust response stream,
separately selected terminal owner and optional body reader compose without
adding a second CDP observer. Exact nine-row merges reach candidate 1,542 and
oracle 1,245 expected outcomes; complete corpora remain open. [Architecture
and evidence](node-ffi-compatibility-design.md#response-waiter-and-terminal-body-readiness-node-432-2026-10-07).

Core Page request-wait NODE-431 passes all eight unchanged Chromium Page cases
in candidate and independent oracle lanes. One shared synchronous language
waiter owns public predicates, cross-realm RegExp, caller stacks and diagnostic
formatting; Rust retains request observation, bounded queues, URL globbing and
Page-default timeout state. Exact eight-row merges reach candidate 1,534 and
oracle 1,237 expected outcomes; complete corpora remain open. [Architecture
and evidence](node-ffi-compatibility-design.md#page-request-waiter-parity-node-431-2026-10-07).

Core Request post-data NODE-430 composes `page.waitForRequest()` from one
bounded Rust request-notice stream and the existing Rust URL-glob compiler.
The optional parser leaf adds Rust JSON/form decoding without selecting Page
routes, response bodies or request history. Node only arms the public listener
synchronously, evaluates language predicates and wraps the immutable snapshot.
Candidate and independent oracle pass the complete unchanged six-case Chromium
file 6/6 with verified provenance; exact six-row merges bring candidate to
1,526 expected and oracle to 1,229 expected. Full corpora remain open.
[Architecture and evidence](node-ffi-compatibility-design.md#bounded-page-request-waiting-and-post-data-decoding-node-430-2026-10-06).

Core extra HTTP headers NODE-429 completes the seven unchanged Chromium
Context/Page cases with candidate/oracle outcome parity. The new independently
selected `response-request` composition returns the actual Rust-captured
navigation Request snapshot and removes the old masked TypeError. Chrome 154
passes the Referer assertion still marked as an upstream expected failure, so
both lanes honestly record the same one unexpected pass. [Architecture and
evidence](node-ffi-compatibility-design.md#navigation-response-request-snapshots-node-429-2026-10-06).

Core Touchscreen NODE-428 completes all nine unchanged Chromium Library
`tap.spec.ts` cases in candidate and independent oracle lanes. Rust keeps raw
coordinate input separate from selector/Handle actionability, hit-target,
trial and modifier semantics. Final artifact `5fb122e...b91` has verified
provenance; strict nine-row merges bring candidate to 1,521 expected and oracle
to 1,219 expected. Full corpora remain open. [Architecture and
evidence](node-ffi-compatibility-design.md#composable-touchscreen-and-tap-actions-node-428-2026-10-06).

Core Touch NODE-427 completes the unchanged Chromium Library
`locator-dispatchevent-touch.spec.ts` case 1/1 in candidate and independent
oracle lanes. The Rust dispatch program materializes native `Touch` objects,
and the existing Context device-emulation registry is selected through an
independent thin Node feature/port for `hasTouch`. Final artifact
`69a91ea...de1` has verified provenance; strict one-row merges bring candidate
to 1,512 expected and oracle to 1,210 expected. Full corpora remain open.
[Architecture and evidence](node-ffi-compatibility-design.md#touch-event-materialization-and-context-emulation-node-427-2026-10-06).

Core Locator NODE-426 completes all 24 unchanged Chromium Page cases in
`locator-query.spec.ts`. Oracle is 24/24; an honest candidate 12/24 baseline
drove typed regex filters, existing Rust ordered-text expectations,
structural/deferred FrameLocator composition and a bounded private before-query
rendezvous. Final candidate/oracle pass 24/24 with artifact
`c12153d...d1bc` and verified provenance. Strict 24-row merges bring the
candidate ledger to 1,511 expected and the oracle to 1,209 expected. All 123
Locator Chromium cases are now accounted for; the full corpora remain open.
[Architecture and
evidence](node-ffi-compatibility-design.md#locator-query-composition-node-426-2026-10-06).

NODE-425 completes all 16 unchanged Chromium Page cases in
`locator-misc-2.spec.ts`. Oracle is 16/16; an honest candidate 12/16 baseline
drove thin public viewport, Locator screenshot and text-selection ports over
existing Rust capabilities. A 15/16 intermediate run isolated the last issue
to typed unknown-key presentation. Final candidate/oracle pass 16/16 with
artifact `3edcb5c...71db` and verified provenance. Strict 16-row merges bring
the candidate ledger to 1,487 expected and the oracle to 1,185 expected; 24
Locator Chromium cases remain. [Architecture and
evidence](node-ffi-compatibility-design.md#locator-miscellaneous-viewport-capture-and-selection-node-425-2026-10-06).

NODE-424 completes all 13 unchanged Chromium Page cases in
`locator-misc-1.spec.ts`. Corrected oracle is 13/13; an honest candidate 11/13
baseline showed only the missing thin `dispatchEvent` port and the unselected
existing upload leaf. Rust retains strict resolution, bounded event conversion
and the static browser program; `file-input` is composed only into the explicit
official profile. Final candidate/oracle pass 13/13 with artifact
`356f84b...c39e` and verified provenance. Strict 13-row merges bring the
candidate ledger to 1,471 expected and the oracle to 1,169 expected; 40 Locator
Chromium cases remain. [Architecture and
evidence](node-ffi-compatibility-design.md#locator-miscellaneous-actions-node-424-2026-10-06).

NODE-423 completes all 22 unchanged Chromium Page cases in
`locator-frame.spec.ts`. Corrected oracle is 22/22; an honest candidate 3/22
baseline drove thin nested/indexed/semantic FrameLocator composition, the
independent `Route.fulfill()` leaf and Rust-owned frame replacement, retry,
missing-frame and owner-diagnostic semantics. Final candidate/oracle pass
22/22 with artifact `fbc48ff...1382b` and verified provenance. Strict 22-row
merges bring the candidate ledger to 1,458 expected and the oracle to 1,156
expected; 53 Locator Chromium cases remain. [Architecture and
evidence](node-ffi-compatibility-design.md#locator-framelocator-composition-node-423-2026-10-06).

NODE-422 completes all 22 unchanged Chromium Page cases in
`locator-convenience.spec.ts`. An honest candidate 13/22 versus oracle 22/22
baseline drove thin public Locator description formatting and
`evaluateHandle`; strict resolution, bounded deferred conversion, rich Node
preview, result ownership and cleanup remain in Rust. Final candidate/oracle
pass 22/22 with artifact `23f0bf4...390f05`. Strict 22-row merges bring the
candidate ledger to 1,436 expected and the oracle to 1,134 expected; 75 Locator
Chromium cases remain. [Architecture and evidence](node-ffi-compatibility-design.md#locator-convenience-composition-node-422-2026-10-06).

NODE-421 reverifies five complete Locator files: evaluate 6, list 1, highlight
1, click 5 and is-visible 8. Candidate and oracle pass 21/21 with verified
provenance and no artifact change. The candidate ledger refreshes exactly the
same 21 expected rows and stays at 1,414 expected; the oracle promotes the same
21 rows to reach 1,112 expected. All 11 Locator files now have fixed manifests
covering 123 Chromium Page tests; the remaining 97 are not yet claimed as
passing. [Evidence](node-ffi-compatibility-design.md#supported-locator-file-reverification-node-421-2026-10-06).

NODE-420 completes the unchanged five-case Chromium
`locator-element-handle.spec.ts` file. Candidate red evidence is 0/5 against
oracle 5/5. Node now exposes thin public `Locator.elementHandle(s)` delegates;
Rust composes typed parents with XPath descendants as one bounded immutable
program and owns strict waiting, batch snapshots and Handle publication. Final
candidate/oracle pass 5/5 with verified provenance. Exactly five rows advance
the Core ledgers: candidate reaches 1,414 expected / one skipped / two
unexpected / 47,060 not run; oracle reaches 1,091 expected / one unexpected /
47,385 not run. [Architecture and evidence](node-ffi-compatibility-design.md#locator-elementhandle-composition-node-420-2026-10-06).

NODE-418 records complete ElementHandle query-selector 7/7 and eval-on-selector
6/6 candidate/oracle passes, plus an honest convenience candidate 5/15 versus
oracle 15/15 baseline. NODE-419 moves attribute, HTML, input-value and
visibility/state behavior into one exact-owner Rust capability with thin
ElementHandle/Page/Frame facades. The complete candidate file now has a
preserved 12/15 result; Rust `nth=<i32>` composition and retained-Node rich
preview then close all three reds. Final artifact `0247543...db47f` passes the
complete unchanged candidate file 15/15 with verified provenance. Exactly ten
unexpected rows become expected, bringing the Core ledger to 1,409 expected /
one skipped / two unexpected / 47,065 not run. Host release is 609 passed plus
14 explicit live-browser ignores; package units are 346/346 with strict
TypeScript and full feature-union Clippy green. [Architecture and
evidence](node-ffi-compatibility-design.md#elementhandle-dom-reads-and-official-red-baseline-node-418419-2026-10-06).

NODE-417 adds composable Page/Frame `$` and `$$` selector plans. Rust retains
XPath separately from typed CSS, compiles explicit and automatic XPath plus
mixed chains, and publishes `$$` as one ordered native Handle batch. The full
official Chromium query-selector file passes candidate/oracle 17/17; exactly
17 rows advance the Core ledger to 1,396 expected / one skipped / two
unexpected / 47,078 not run. [Architecture and evidence](node-ffi-compatibility-design.md#composable-page-and-frame-query-selectors-node-417-2026-10-06).

NODE-416 replaces the Page/Frame `$` count-plus-Handle split with one
Rust-owned optional ElementHandle program. Candidate and oracle pass the same
five remaining selector-registration rows 5/5, and exactly five rows advance
the Core ledger to 1,379 expected / one skipped / two unexpected / 47,095 not
run. [Architecture and evidence](node-ffi-compatibility-design.md#atomic-optional-page-query-node-416-2026-10-06).

NODE-414 routes browser-backed `$eval`/`$$eval` through the Rust registered
selector plan and passes the first official Chromium execution row 1/1 in both
candidate and oracle lanes. Exactly one row advances the Core ledger to 1,374
expected / one skipped / two unexpected / 47,100 not run. Positive custom
`page.$` Handle publication and the remaining selector rows stay open.
[Evidence](node-ffi-compatibility-design.md#browser-backed-selector-execution-node-414-2026-10-06).

NODE-413 adds an independent Node `selector-registration` profile over the
existing bounded Rust registry. The initial candidate 0/9 versus oracle 9/9
browserless discrepancy is repaired to 9/9 in both lanes across three library
and six BiDi projects. Exactly nine rows advance the Core ledger to 1,373
expected / one skipped / two unexpected / 47,101 not run. Browser-backed
custom-engine rows remain open. [Evidence](node-ffi-compatibility-design.md#node-selector-registration-capability-node-413-2026-10-06).

NODE-411 proves the NODE-408 static launcher metadata across all six official
BiDi/Firefox library projects at candidate/oracle 12/12 without starting a
browser. Exactly 12 rows advance the Core ledger to 1,364 expected / one
skipped / two unexpected / 47,110 not run. [Evidence](upstream-playwright-conformance.md).

NODE-410 adds thin async/sync `BrowserContext.expect_page()` facades over the
existing bounded lifecycle owner. The initial candidate 7/9 discrepancy is
repaired to candidate/oracle 9/9; exactly nine rows advance the Python ledger
to 1,201 passes / eight skips / 5,317 not run without an ABI or native-artifact
change. [Architecture and evidence](python-ffi-compatibility-design.md#abi-104-browsercontext-expect-page-facade-2026-10-06).

NODE-409 verifies the unchanged Rust request-timing owner across all four sync
Chromium rows. Candidate passes 4/4, while the Chrome 154 oracle is 2/4 due to
zero DNS timing on ordinary navigation and redirect. Only the common SSL and
subresource passes advance the Python ledger to 1,192 passes / eight skips /
5,326 not run. [Evidence](python-official-conformance.md#sync-request-timing-checkpoint-2026-10-06).

NODE-408 adds the complete pinned 143-entry Playwright device registry as
licensed static compatibility data. The official browserless launcher slice
moves from candidate 3/6 versus oracle 6/6 to 6/6 in both lanes; exactly six
rows advance the Core ledger to 1,352 expected / one skipped / two unexpected /
47,122 not run. [Architecture and evidence](node-ffi-compatibility-design.md#complete-device-registry-checkpoint-node-408-2026-10-06).

NODE-407 completes sync service-worker policy at candidate/oracle 2/2 under
trusted Tailscale HTTPS. Both async and sync Chromium files are now complete,
and the Python ledger reaches 1,190 passes / eight skips / 5,328 not run.
[Evidence](python-official-conformance.md#sync-service-worker-policy-checkpoint-2026-10-06).

NODE-406 confirms the existing Rust Keyboard `+` mapping in candidate and
oracle, promoting exactly one row and bringing the Python ledger to 1,188
passes / eight skips / 5,330 not run. Two candidate/oracle shared environment
reds remain unpromoted. [Evidence](python-official-conformance.md#keyboard-plus-and-shared-environment-checkpoint-2026-10-06).

NODE-403 preserves caller-thread provenance on sync Playwright errors without
changing ABI104/90. Candidate and oracle pass the same seven Chromium Page rows
7/7; exactly seven rows advance the Python ledger to 1,187 passes / eight skips
/ 5,331 not run. Two headed-Inspector `Page.pause` rows remain explicitly
unrun. [Architecture and evidence](python-ffi-compatibility-design.md#abi-104-sync-caller-stack-parity-2026-10-06).

NODE-402 verifies the two remaining sync request-fulfillment Chromium rows at
candidate/oracle 2/2 without a code or ABI change. Both async and sync files
are now 3/3, and the cumulative Python ledger reaches 1,180 passes / eight
skips / 5,338 not run. [Evidence](python-official-conformance.md#sync-request-fulfill-parity-checkpoint-2026-10-06).

NODE-401 completes the official 25-row TodoMVC Chromium profile at candidate
25/25 and Playwright 1.58.0 oracle 25/25. Rust fixes nested-arrow expression
classification, Python supplies only the inactive tracing teardown shape, and
exactly 25 rows advance the Python ledger to 1,178 passes / eight skips / 5,340
not run. [Architecture and evidence](python-ffi-compatibility-design.md#abi-104-complete-todomvc-profile-2026-10-06).

NODE-400 advances Python to ABI104 with Rust-owned exact/regex value and class
expectations, including mixed ordered class arrays. Four unchanged upstream
Chromium cases pass in both candidate and oracle lanes, advancing the Python
ledger to 1,153 passes / eight skips / 5,365 not run. [Architecture and
evidence](python-ffi-compatibility-design.md#abi-104-composable-typed-string-matchers-2026-10-06).

NODE-398 resolves the two official Python profiles to 95 fixture definitions
and all per-invocation override chains with zero unresolved providers. The 25
TodoMVC cases now carry the plugin Page closure, and all current outcomes are
partitioned by fixture-induced runtime. [Definition evidence](python-official-conformance.md#profile-aware-fixture-definition-checkpoint-2026-10-06).

NODE-397 records direct, initial and complete fixture-name closures under each
frozen official profile. It separates core-only server/asyncio dependencies
from the TodoMVC plugin closure without executing fixtures. [Fixture
evidence](python-official-conformance.md#profile-aware-pytest-fixture-closure-checkpoint-2026-10-06).

NODE-396 reconstructs upstream's separate core and examples CI jobs, replaces
exactly 25 obsolete root-only IDs with official `[chromium]` IDs, and rebases
only pristine not-run evidence. [Profile evidence](python-official-conformance.md#official-ci-profile-reconstruction-checkpoint-2026-10-06).

NODE-393 records all source-declared pytest inputs across the 6,526 Python
invocations and cross-tabulates their outcomes without mislabeling them as the
separate transitive fixture closure. NODE-397 now verifies both layers in the
correct official profiles. [Audit
evidence](python-official-conformance.md#direct-pytest-input-inventory-checkpoint-2026-10-06).

NODE-392 adds a fail-closed browserless official-Python lane for exact
manifest cases while retaining the upstream autouse Tailscale fixture servers.
Candidate and oracle agree on three lifecycle passes and three
shared topology failures; two newly green Firefox/WebKit rows move the ledger
to 1,149 passed / eight skipped / 5,369 not run without an ABI or runtime
change. [Evidence and policy](python-official-conformance.md#browserless-sync-lifecycle-checkpoint-2026-10-06).

NODE-391 introduces an applicable Core queue for 6,279 public behaviors that
use private test helpers. It preserves exact local implementation tests and
product infrastructure separately, leaving 3,458 explicitly internal rows.
[Evidence and completion status](upstream-playwright-conformance.md).

NODE-390 replaces source-wide private-client exclusion with bounded
per-invocation evidence. The full audit restores 1,361 public-behavior rows
and six product rows without changing outcomes; current counts are 32,148
public candidates, 10,305 internal, 5,580 product and 444 public-with-hook.
[Evidence and completion status](upstream-playwright-conformance.md).

NODE-389 preserves the 67 official example-application invocations while
classifying their assertions as product infrastructure rather than public API
compatibility. The role-bearing 48,477-row audit now records 30,814 public
candidates and 5,574 product-infrastructure invocations.
[Evidence and completion status](upstream-playwright-conformance.md).

NODE-388 corrects Core applicability accounting for 477 upstream
repository-local EventEmitter tests without changing an outcome. The complete
audit now records 30,881 public candidates and 11,672
implementation-internal invocations while preserving all 48,477 rows.
[Evidence and completion status](upstream-playwright-conformance.md).

NODE-387 advances the optional Python package to ABI 103 with one Rust-owned
ElementHandle focus/input command and thin Page/Frame selector facades. The
fresh installed wheel closes every deterministic discrepancy in the complete
async Chromium keyboard tranche; only verified green evidence advances the
ledger to 1,147 passes / eight skips / 5,371 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-103-composable-elementhandle-keyboard-checkpoint-2026-10-06).

NODE-386 advances the optional Python package to ABI 102 with a composable
Page Keyboard facade over the existing Rust host. The fixed official focus
tranche passes 5/5 from the final installed wheel and oracle; cumulative
candidate execution reaches 1,121 passes with no recorded failure.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-102-composable-page-keyboard-checkpoint-2026-10-06).

NODE-385 advances the optional Python package to ABI 101 with composable
Rust-owned Request timing and an exact bounded notice-publication fence. The
fixed official Chromium tranche passes 4/4 from the final installed wheel and
oracle; cumulative candidate execution reaches 1,119 passes while two focus
failures remain explicit.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-101-composable-resource-timing-checkpoint-2026-10-06).

NODE-384 closes the two remaining Chromium rows in upstream
`test_request_fulfill.py` at candidate/oracle 2/2 without a runtime change;
cumulative candidate execution reaches 1,110 passes.
[Evidence](python-ffi-compatibility-design.md#abi-100-request-fulfill-evidence-checkpoint-2026-10-06).

NODE-383 advances the optional Python package to ABI 100 with bounded,
Rust-owned composite JSHandle arguments and atomic property batches. The fixed
official tranche passes 15/15 after a preserved 6/15 candidate baseline;
cumulative candidate execution reaches 1,108 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-100-composite-jshandle-checkpoint-2026-10-06).

NODE-382 advances the optional Python package to ABI 99 with a separately
selectable native-directory input route. It passes every directory behavior
in the fixed official tranche while preserving ordinary remote payload
limits; cumulative candidate execution reaches 1,093 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-99-native-directory-input-checkpoint-2026-10-06).

NODE-346 advances the optional Python package to ABI 93 with Rust-owned
redirect continuation, bounded header replay and exact request-history
identity. Candidate/oracle pass 2/2; the full Page-route file is 43 pass / 1
fail and cumulative Python execution reaches 853 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-93-rust-owned-route-redirect-checkpoint-2026-10-05).

NODE-345 advances the optional Python package to ABI 92 with separately
selectable Rust-owned `APIResponse` route fulfillment. Candidate/oracle pass
1/1; the full Page-route file is 41 pass / 3 fail and cumulative Python
execution reaches 851 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-92-rust-owned-apiresponse-route-fulfillment-checkpoint-2026-10-05).

NODE-344 keeps the optional Python package at ABI 91 and handles matching
browser-generated CORS preflights entirely in the bounded Rust route owner.
Candidate/oracle pass 2/2; the full Page-route file is 40 pass / 4 fail and
cumulative Python execution reaches 850 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-91-native-cors-preflight-checkpoint-2026-10-05).

NODE-343 advances the optional Python package to ABI 91 with exact routed
Request-to-Response correlation through the bounded Rust request-history
owner. Candidate/oracle pass 1/1; the full Page-route file is 38 pass / 6 fail
and cumulative Python execution reaches 848 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-91-exact-routed-requestresponse-checkpoint-2026-10-05).

NODE-342 advances the optional Python package to ABI 90 with composable
regex/predicate route matching at the language boundary and separately
selectable bounded Rust file fulfillment. Candidate/oracle pass 2/2; the full
Page-route file is 37 pass / 7 fail and cumulative Python execution reaches
847 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-90-composable-route-binding-and-fulfill-path-checkpoint-2026-10-05).

NODE-340 advances the optional Python package to ABI 89 with exact routed and
ordinary Request Frame identity, `Response.frame` and
`ElementHandle.content_frame()`. Candidate/oracle pass the selected rows 6/6
plus 1/1; the complete Page-route file is 35 pass / 9 fail and cumulative
Python execution reaches 845 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-89-exact-request-frame-identity-checkpoint-2026-10-05).

NODE-339 advances the optional Python package to ABI 87 with one bounded
Rust-owned Page request lifecycle stream. The two unchanged Playwright Python
1.58.0 route-abort rows pass installed candidate/oracle 2/2 and repeat 10/10;
the complete Page-route file is 34 pass / 10 fail and cumulative Python
execution reaches 839 passes.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-87-request-lifecycle-events-checkpoint-2026-10-05).

NODE-338 keeps the optional Python package at ABI 86 and moves Playwright's
fixed HTTP reason-phrase mapping into Rust-owned route fulfillment. The
unchanged Chromium 422 row passes installed candidate/oracle 1/1 and repeat
5/5; the full Page-route file reaches 32 pass / 12 fail and cumulative Python
execution reaches 837 passes.
[Evidence](python-ffi-compatibility-design.md#abi-86-routefulfill-status-text-checkpoint-2026-10-05).

NODE-337 keeps the optional Python package at ABI 86 and promotes two more
unchanged Playwright Python 1.58.0 Page-route rows for native `Referer` and
`Origin` lowercase publication. Candidate/oracle pass 2/2 and repeat 10/10;
the complete Page-route file is 31 pass / 13 fail and cumulative Python
execution reaches 836 passes.
[Evidence](python-ffi-compatibility-design.md#abi-86-public-route-header-evidence-checkpoint-2026-10-05).

NODE-336 advances the optional Python package to ABI 86 with bounded
Rust-owned Page and BrowserContext extra HTTP headers. Three unchanged
Playwright Python 1.58.0 rows pass installed candidate/oracle 3/3 and repeat
15/15; cumulative execution is 834 passed, zero failed, seven skipped and
5,685 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-86-page-and-context-extra-http-headers-checkpoint-2026-10-05).

NODE-335 advances the optional Python package to ABI 85 with bounded Rust-
owned `Route.fallback()` traversal and handler-specific Page/BrowserContext
`unroute()`. Eight unchanged Playwright Python 1.58.0 rows pass installed
candidate/oracle 8/8 and repeat 40/40; cumulative execution is 831 passed,
zero failed, seven skipped and 5,688 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-85-route-fallback-and-handler-unroute-checkpoint-2026-10-05).

NODE-334 advances the optional Python package to ABI 84 with atomic Rust-owned
finite route-use budgets for Page and BrowserContext. Three unchanged
Playwright Python 1.58.0 rows pass installed candidate/oracle 3/3 and repeat
15/15; cumulative execution is 823 passed, zero failed, seven skipped and
5,696 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-84-finite-route-use-checkpoint-2026-10-05).

NODE-333 keeps ABI 83 and makes route-glob `?` literal in both the Rust
matcher and bounded Chromium Fetch-pattern compiler. The unchanged official
row passes installed candidate/oracle 1/1 and repeat 5/5; cumulative Python
execution is 820 passed, zero failed, seven skipped and 5,699 not run.
[Semantics, bounds and evidence](python-ffi-compatibility-design.md#abi-83-route-glob-question-mark-checkpoint-2026-10-05).

NODE-332 advances the optional Python package to ABI 83 with bounded Rust-
owned URL, method, header and POST-data overrides for `Route.continue_()`.
Ten unchanged Playwright Python 1.58.0 rows pass installed candidate and
oracle 10/10; cumulative execution is 819 passed, zero failed, seven skipped
and 5,700 not run. The full Page-route file is now 23 pass / 21 fail / 0
errors.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-83-route-continue-override-checkpoint-2026-10-05).

NODE-331 executes the complete 44-case async Page-route Chromium file. The
candidate baseline is 18 passed, 25 failed and one error; the 18 existing green
rows pass candidate/oracle 18/18 and raise cumulative Python execution to 809
passed, zero failed, seven skipped and 5,710 not run.
[Baseline and evidence](python-ffi-compatibility-design.md#abi-82-full-page-route-baseline-checkpoint-2026-10-05).

NODE-330 promotes six additional unchanged ABI 82 unroute-all rows without an
implementation change. Candidate and oracle pass async/sync default removal
4/4 plus in-flight wait/fulfill 2/2; cumulative Python execution is 791
passed, zero failed, seven skipped and 5,728 not run.
[Evidence](python-ffi-compatibility-design.md#abi-82-expanded-unroute-all-evidence-checkpoint-2026-10-05).

NODE-329 advances the optional Python package to ABI 82 with Rust-owned Page
and BrowserContext `unroute_all()` plus cancellation-safe wait settlement. Two
unchanged Playwright Python 1.58.0 HAR-abort rows pass installed
candidate/oracle 2/2 and candidate stress 20/20; cumulative Python execution
is 785 passed, zero failed, seven skipped and 5,734 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-82-scoped-unroute-all-checkpoint-2026-10-05).

NODE-328 advances the optional Python package to ABI 81 with bounded Rust-owned
ordinary BrowserContext routes. Four unchanged Playwright Python 1.58.0 HAR
URL-filter composition rows pass installed candidate/oracle 4/4 and stress
passes async 20/20 plus sync 20/20; cumulative Python execution is 783 passed,
zero failed, seven skipped and 5,736 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-81-browsercontext-route-composition-checkpoint-2026-10-05).

NODE-327 keeps Python ABI 80 and maps a missing HAR `log.entries` member to an
empty bounded archive so the existing fallback policy continues the request.
Candidate/oracle pass 2/2 and stress passes 10/10 per API; cumulative Python
execution is 779 passed, zero failed, seven skipped and 5,740 not run.
[Semantics and evidence](python-ffi-compatibility-design.md#abi-80-missing-entries-har-fallback-checkpoint-2026-10-04).

NODE-326 advances the optional Python package to ABI 80 with bounded Rust-owned
BrowserContext HAR scope and future-Page/OOPIF initialization. Six unchanged
Playwright Python 1.58.0 rows pass installed candidate/oracle 6/6 and stress
passes async 30/30 plus sync 30/30; cumulative Python execution is 777 passed,
zero failed, seven skipped and 5,742 not run.
[Architecture, bounds and evidence](python-ffi-compatibility-design.md#abi-80-browsercontext-har-replay-checkpoint-2026-10-04).

NODE-325 promotes the complete unchanged async/sync Page HAR URL-filter/no-
fallback pair on the same ABI 79 wheel. Candidate/oracle pass 2/2 and separate
stress lanes pass 10/10 each; cumulative Python execution is 771 passed, zero
failed, seven skipped and 5,748 not run.
[Evidence and route-composition boundary](python-ffi-compatibility-design.md#abi-79-page-har-url-filter-composition-checkpoint-2026-10-04).

NODE-324 promotes the complete unchanged async/sync Page HAR method/redirect
pair on the same ABI 79 wheel. Candidate/oracle pass 2/2 and separate stress
lanes pass 10/10 each; cumulative Python execution is 769 passed, zero failed,
seven skipped and 5,750 not run.
[Evidence and harness boundary](python-ffi-compatibility-design.md#abi-79-async-and-sync-page-har-evidence-checkpoint-2026-10-04).

NODE-323 advances the optional Python package to ABI 79 with bounded plain-HAR
replay composed over the existing Page-route Fetch owner. The fixed official
Playwright Python 1.58.0 row passes installed candidate/oracle 1/1 and candidate
stress 10/10; cumulative Python execution is 767 passed, zero failed, seven
skipped and 5,752 not run.
[Architecture, resource bounds and evidence](python-ffi-compatibility-design.md#abi-79-persistent-context-plain-har-replay-checkpoint-2026-10-04).

NODE-322 advances the optional Python package to ABI 78 with bounded
Rust-owned Context HTTP credentials and navigation response-body retrieval.
The fixed official Playwright Python 1.58.0 selection passes installed
candidate/oracle 11/11 and candidate stress 110/110; cumulative Python
execution is 766 passed, zero failed, seven skipped and 5,753 not run.
[Architecture, resource bounds and evidence](python-ffi-compatibility-design.md#abi-78-http-credentials-and-response-body-checkpoint-2026-10-04).

NODE-050: the opt-in Node addon is now built and verified through real public
Mouse/Keyboard/Locator calls. Internal Frame lifecycle coverage expands to 18.
[Selection, evidence and remaining work](knowledge/pwright-chromium-drag.md#node-050-native-lifetime-gates-and-selected-node-addon).

NODE-049: selected Rust Frame input now hands cancellation lifetime to Page
after focus. The expanded local/OOPIF native gate matches 12/12.
[Contract, evidence and remaining gates](knowledge/pwright-chromium-drag.md#node-049-optional-frame-to-page-lifetime-handoff).

NODE-048: [native local/OOPIF Frame gate](knowledge/pwright-chromium-drag.md#node-048-native-frame-gate-exposes-a-lifetime-discrepancy)
finds premature Rust cancellation after iframe removal. It remains failing (8/10).

NODE-047: [Locator keyboard / Page drag integration](knowledge/pwright-chromium-drag.md#node-047-locator-keyboard-selects-the-owning-page).
Ten native host/oracle matches, including timeout; addon remains unchanged.

NODE-046: [Rust Escape interception and retirement](knowledge/pwright-chromium-drag.md#node-046-optional-rust-keyboard-escape-interception)
shares the Page drag controller without adding a default drag dependency.

NODE-045: [derived click-batch admission](knowledge/pwright-chromium-drag.md#node-045-maximum-click-batches-and-matching-host-admission)
fixes the maximum count boundary in the rebuilt Node addon. Public Mouse remains
incomplete at 39/44.

NODE-044: [Rust HostSession drag integration](knowledge/pwright-chromium-drag.md#node-044-opt-in-hostsession-mouse-integration)
is available behind `drag-runtime`. It does not yet change the Node addon or
public Mouse baseline.

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

NODE-039 composes drag listeners, interception and lazy native movement in Rust;
**57** unit tests and **9** native cases pass. Full Page/Mouse and Node integration
remain open. [Current checkpoint](knowledge/pwright-chromium-drag.md#node-039-composed-intercepted-movement).

NODE-038 adds a fixed-session capture owner for subscriptions and interception
enable/disable, preserving cleanup across cancellation. **44** unit tests and
**6** native cases pass; Page dispatcher and Node host integration remain open.
[Current checkpoint](knowledge/pwright-chromium-drag.md#node-038-retained-capture-lifecycle).

NODE-037 adds bounded typed intercepted-data retention in Rust, with **14** new
tests (**27** total) and real captured-text delivery verified. Subscription epochs,
the full drag provider and host integration remain pending.
[Current checkpoint](knowledge/pwright-chromium-drag.md#node-037-admitted-intercepted-data).

NODE-036 implements a separate Rust-owned utility-world drag observer, with
**13** unit tests and **6** real Chrome scenarios. It is not yet the production
Mouse drag provider. [Design and next integration](knowledge/pwright-chromium-drag.md).

Native/Page bindings now select an optional Mouse dispatcher across five
pointer APIs; standalone button results retain independent cleanup (NODE-035).
Default and test passthrough paths pass **22/22** each. Drag and host integration
remain open. [Selection and evidence](knowledge/pwright-raw-mouse.md#page-scoped-dispatcher-selection-node-035).

The optional Mouse dispatcher now has separate action and cleanup polling,
live Page-state readers and fixed-slot retirement tests (NODE-034). Chromium
drag and factory/host injection remain pending; the addon is not rebuilt.
[Current source checkpoint](knowledge/pwright-raw-mouse.md#optional-mouse-dispatch-ownership-node-034).

Latest concurrency gate is red: Mouse **33/41 -> 36/41** after a bounded Rust
first-poll ordering boundary. This preserves overlapping results but is not a
whole-operation lock. Five ordinary left-held moves require the missing drag
delegate. [NODE-033](knowledge/pwright-raw-mouse.md#same-turn-input-ordering-node-033).

Final ordering artifact also records typing **30/38** and actions **24/27** due
to oracle/candidate attachment timeouts. Mouse Worker/profile **3/3**, Keyboard
**24/24**, screenshot **12/12** and lifecycle **45/45** pass. Prior reload/shared
results were not rerun on this final artifact.

Previous Mouse release addon: public **26/26** and profile/Worker **3/3**; Keyboard 24,
typing 38, actions 27, screenshot 12 and lifecycle 45 all pass. Reload remains
**16/18**, shared **53 match / 2 unsupported / 1 oracle-error / 1 setup-error**.
The topology-attach timeout and native reload-close failures remain open.
[Artifact-specific evidence](knowledge/pwright-raw-mouse.md#same-artifact-regression).

`pwright-backend-cdp-mouse` now provides six admitted native starters, explicit
result/retirement owners, **13** unit tests and **22/22** real-browser matches.
Its normal closure is **8** Pwright packages. The optional Page route adds bounded
caller-polled retirement, **18** tests and **22/22** full-wire browser matches,
with a **16-package** closure. The explicit `runtime` adapter adds **15** tests,
**22/22** controlled browser matches and only `pwright-runtime` (**17** packages).
The safe Rust host now installs bounded retirement workers and all six typed
methods: **22/22** host browser matches and a real close-during-delay probe pass.
Host source closure is **234**. The rebuilt addon exposes all six public Mouse
methods: **26/26** paired cases and **3/3** profile/Worker tests pass. Final
nested allocation and artifact-wide regression gates remain open.

New optional packages: `pwright-cap-mouse`, `pwright-client-mouse`,
`pwright-codec-mouse`, `pwright-server-mouse`. Six typed Page commands and their
normal closures **3/6/7/10** pass; no native/Node installation is pulled in.
[Contract ownership and next integration gates](knowledge/pwright-raw-mouse.md#optional-mouse-10-contracts).

[Raw Mouse and click scheduling](knowledge/pwright-raw-mouse.md) now match
**22/22** pinned event/state cases. Rust retains in-flight click calls through
explicit retirement. Node `page.mouse` and owned/borrowed Worker close now pass;
concurrent error ordering, HTML drag and final allocation validation remain open.

Same-artifact full gates now pass typing **38/38**, actions **27/27**, screenshot
**12/12**, lifecycle **45/45**. Reload remains **16/18** at the unchanged short
response/close boundary. See the [regression checkpoint](knowledge/pwright-raw-keyboard.md).

Keyboard's opt-in controlled adapter now passes **18** composition tests, with
default/runtime closures **16/17**, and a fresh regenerated-wire Chrome comparison
matches **14/14** cases. The subsequent host/N-API installation exposes all five
public Keyboard methods and passes **24/24**, with shared coverage **52/3/2**;
see the [checkpoint](knowledge/pwright-raw-keyboard.md).

This is the authoritative reference for understanding pwright. Read this
before proposing changes.

[Raw Keyboard primitives](knowledge/pwright-raw-keyboard.md) pass **14/14**
native oracle cases and preserve the shared Page input boundary. Keyboard 1.0
contract layers now pass **19** tests, with no native/backend/Node dependency.
The separate native backend passes **18** tests and **14/14** typed Chrome cases.
The optional Page wire composition adds **11** tests and **14/14** wire-to-Chrome
cases. Aggregate lifecycle routing and public Node integration remain pending;
no shared reclassification.
Aggregate Page input is now independently selected and retained-state-budgeted,
including leases held by escaped coordinator clones and transactions. Node host
source selects it; the unchanged addon has not yet incorporated this source change.

[Rust-backed reload](knowledge/pwright-page-reload.md) now has typed navigation
1.1 and Node FFI bindings. Both shared reload cases match (corpus **48/7/2**);
its public held-response close gate and broader window geometry remain red.

New [sequential-typing binding evidence](knowledge/pwright-sequential-typing.md)
records typed Locator 1.23/2.6 aliases and Node's new opt-in interleaved input
policy. The new artifact passes **30/30 typing and 27/27 action** tests, including
the previous overlap and NODE-022 failures. Default Rust exclusivity remains;
native in-flight cancellation and broader Frame/timeout cases still need
verification. The full goal is not complete.

The preceding Node artifact opts into a fixed recursive-attachment prerequisite join.
Screenshot passes 12/12, lifecycle 45/45 and serial/selected OOPIF checks 2/2.
Shared results are 45 match / 10 unsupported / 2 oracle-error, not a full
compatibility pass. [Latest checkpoint](node-ffi-compatibility-design.md#node-013-recursive-attachment-prerequisite-join-2026-09-07).

Node `page.screenshot()` now composes the Rust content command, bounded queue and
Buffer handoff. Earlier screenshot/shared attachment failures remain preserved
as [artifact-specific evidence](node-ffi-compatibility-design.md#node-page-screenshot-binding-and-ownership-checkpoint-2026-09-07).

Current Node work exposes Locator.evaluate through the existing Rust command,
with admitted Node-thread source/argument conversion. Its native running-Worker
cleanup gate now passes through acknowledged owned-session retirement (NODE-027),
including preservation of a borrowed Page and user Promise. Other discrepancies
and unsupported APIs remain; [the checkpoint](node-ffi-compatibility-design.md#node-027-host-integration-and-worker-verification-2026-09-07)
supersedes earlier unexposed-method and native Worker-cleanup failure statements.

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
                     opt-in production resource and stateless event-admission
                     policies, and an owned terminal
                     connection-closure signal plus owned I/O-cleanup barrier
                     for v2 composition

pwright-js           Centralized JS snippets (string constants, no runtime)

pwright-node-host    Experimental safe Rust v2 attach/topology and controlled
                    navigation/DOM-read/value-evaluation, Locator query/read/action
                    and Context/Page lifecycle host;
                    owned Context defaults through optional session initialization;
                    serial lifecycle/topology transactions, metadata admission
                    before creation, bounded native URL snapshots
                    and ordered JS-loop publication, plus environment-owned
                    FFI execution/result admission and separate lazy-query budget
                    with one controlled Locator selection/conversion/evaluation
                    continuation (native retirement and public Node binding pending)
pwright-node-addon   Bounded Node-API conversion, Node-thread binary staging,
                     and environment cleanup (no browser implementation)
packages/pwright    Local-build-only ESM/CJS facade and differential tests;
                    see node-ffi-compatibility-design.md for incomplete gates
tests/              Shared API catalog and owned-Context Node-FFI/oracle runner;
                    reports unsupported and oracle failures without golden updates

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
                     shared CDP call recorder for fake and opt-in bridge mocks;
                     ordered error/response/fallback script for Runtime fakes

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
                         Serde-free `pwright.dom_content` Frame content/title
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
                         Independent opt-in Frame content/title and Page-to-main-Frame
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

pwright-cap-dom-content  Core-only optional read-only Frame DOM content/title,
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

pwright-backend-cdp-locator-resolution
                         Opt-in strict utility-world query into a unique native
                         object group; bounded temporary element, no polling,
                         user evaluation, public Handle or browser ownership

pwright-composition-chromium-locator-resolution
                         Pre-admitted caller-owned continuation cleanup for
                         resolution/element/release; fail-closed admission after
                         failed or abandoned native cleanup; optional bounded
                         strict waiting with injected routes, pacing and one
                         termination signal; not installed by Node yet

pwright-composition-chromium-locator-evaluation
                         Opt-in selected-element adoption into main world and
                         one Rust value evaluation; both wrappers remain in
                         the source group; paired strict preparation preserves
                         the original deadline and route owner through cleanup;
                         no public Handle or Node installation

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

pwright-plugin-chromium-session-initialization
                         Optional five-package Chromium Page/OOPIF setup
                         contract; owns no connection, task, executor, process,
                         or topology driver and is consumed directly by the
                         Node host instead of through the aggregate

pwright-plugin-chromium-frame-observer
                         Optional five-package synchronous Frame event, URL,
                         and ownership projection contracts; owns no connection,
                         queue, task, executor, native domain, or topology driver
                         and is consumed by host adapters without the aggregate

pwright-plugin-chromium-frame-lifecycle
                         Optional ten-package bounded Frame lifecycle and
                         navigation-wait observer; consumes the frame-observer
                         contract directly and is selected by the Node host
                         without routing its lifecycle adapter through the aggregate

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
                         fixed utility-world reads, bounded result and
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
                         generated content/title handlers, exact utility route,
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
[v2 design tasks](design-tasks-v2.md), the
[Playwright-compatible Node FFI design](node-ffi-compatibility-design.md), and the
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

An independently installed `ChromiumFrameWorldAccess` port pairs the observed
main and utility worlds for one exact Frame, retains an explicitly budgeted
snapshot, and provides caller-polled invalidation through the existing commit
notification. Renderer handoff/swap gaps cannot export old-session world
routes, and late previous-renderer announcements cannot replace current worlds.
The snapshot is not an atomic browser document lease or native group-cleanup
proof. The safe Node host now installs the port and separately selects bounded
native-destruction receipts: exact Context acknowledgement or both execution
worlds' destruction can discharge group cleanup. Receipt clones retain snapshot
admission, and default profiles do not install the registry. Node-thread deferred
conversion remains pending; public Locator.evaluate is unavailable. See
[its ownership and validation contract](knowledge/pwright-locator-evaluation.md#optional-native-destruction-receipts-2026-09-07).

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
| `docs/api-request-design.md` | Optional browser-independent API request owner, transport boundary, resource limits and full-conformance gates |
| `docs/node-ffi-compatibility-design.md` | Playwright-compatible Node-API boundary and differential verification plan |
| `docs/python-ffi-compatibility-design.md` | Playwright-compatible Python/PyO3 boundary, packaging, ownership and differential verification plan |
| `docs/python-official-conformance.md` | Pinned official Playwright Python corpus, public/private inventory, exact-test runner and checkpoints |
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
