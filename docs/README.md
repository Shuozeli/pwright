<!-- agent-updated: 2026-10-10T07:11:56Z -->

# pwright Documentation

Current [Core](upstream-playwright-conformance.md) and
[Python](python-official-conformance.md#cross-engine-browsertype-metadata-checkpoint-2026-10-10)
conformance ledgers retain every pinned official invocation. The latest
browserless BrowserType tranche is paired-green on 15 Core matrix rows, 12
Core preflight rows and four additional Python name rows. The six ABI106
Python path-existence assertions remain red in both lanes; full-corpus
execution and lifecycle gates remain open.

NODE-432: [Response waiter and terminal body readiness](node-ffi-compatibility-design.md#response-waiter-and-terminal-body-readiness-node-432-2026-10-07)
matches all nine normalized Chromium Page outcomes: eight expected plus one
shared Chrome-154 cached-image unexpected. Response events, terminal ownership
and body reads remain independently selectable Rust leaves; full-corpus
conformance remains open.

NODE-431: [Page request waiter parity](node-ffi-compatibility-design.md#page-request-waiter-parity-node-431-2026-10-07)
passes the complete unchanged eight-case Chromium Page file 8/8 in candidate
and oracle lanes. Language-level diagnostics and call stacks share one waiter;
request observation, bounded queues, URL globbing and timeout state remain in
Rust. Full-corpus conformance remains open.

NODE-430: [bounded Page request waiting and post-data decoding](node-ffi-compatibility-design.md#bounded-page-request-waiting-and-post-data-decoding-node-430-2026-10-06)
adds independently compiled request-event and Rust parser profiles. The Page
waiter arms synchronously while subscriptions, bounded lifecycle storage, URL
glob compilation and decoding stay in Rust. Candidate and oracle pass the
complete six-case Chromium file 6/6; full-corpus conformance remains open.

NODE-429: [Navigation Response request snapshots](node-ffi-compatibility-design.md#navigation-response-request-snapshots-node-429-2026-10-06)
repairs the masked `Response.request()` failure through an independent Rust
request-history composition. Candidate and oracle agree on all seven complete
extra-header cases, including the same Chrome-154 unexpected pass; full-corpus
conformance remains open.

NODE-428: [Composable Touchscreen and tap actions](node-ffi-compatibility-design.md#composable-touchscreen-and-tap-actions-node-428-2026-10-06)
passes the complete unchanged Chromium Library tap file 9/9 in candidate and
oracle lanes. Coordinate input and selector actions stay in independently
selected Rust compositions; full-corpus conformance remains open.

NODE-427: [Touch event materialization and Context emulation](node-ffi-compatibility-design.md#touch-event-materialization-and-context-emulation-node-427-2026-10-06)
passes the complete unchanged Chromium touch-dispatch file 1/1 in candidate
and oracle lanes. Native `Touch` construction and `hasTouch` Context state stay
in independently selectable Rust capabilities; full-corpus conformance remains
open.

NODE-416: [atomic optional Page query](node-ffi-compatibility-design.md#atomic-optional-page-query-node-416-2026-10-06)
removes Page/Frame `$`'s split count/Handle race, passes the remaining five
Chromium selector-registration rows 5/5 in both candidate and oracle, and
advances cumulative Core execution to 1,379 expected rows.

NODE-414: [browser-backed selector execution](node-ffi-compatibility-design.md#browser-backed-selector-execution-node-414-2026-10-06)
routes `$eval` and `$$eval` through Rust-owned registered selector plans,
passes the first official Chromium execution row 1/1 in candidate and oracle,
and advances cumulative Core execution to 1,374 expected rows.

NODE-413: [Node selector registration](node-ffi-compatibility-design.md#node-selector-registration-capability-node-413-2026-10-06)
adds a separately selectable bounded Rust registry facade, repairs the first
Core `playwright.selectors` slice from candidate 0/9 versus oracle 9/9 to 9/9
in both lanes, and advances cumulative Core execution to 1,373 expected rows.

NODE-411: [BiDi launcher metadata](upstream-playwright-conformance.md)
passes all 12 remaining browserless `errors`/`devices` rows in candidate and
oracle lanes, bringing cumulative Core execution to 1,364 expected outcomes.

NODE-410: [sync BrowserContext event parity](python-official-conformance.md#sync-browsercontext-event-checkpoint-2026-10-06)
adds the missing `expect_page()` facade over the existing lifecycle waiter,
repairs candidate 7/9 to candidate/oracle 9/9 and reaches 1,201 cumulative
Python passes without changing ABI104/90 or the native artifact.

NODE-409: [sync request timing](python-official-conformance.md#sync-request-timing-checkpoint-2026-10-06)
promotes the two paired-green SSL/subresource rows while preserving two
candidate-green/Chrome-154-oracle-red DNS timing rows as unmerged diagnostics.

NODE-408: [complete device registry](node-ffi-compatibility-design.md#complete-device-registry-checkpoint-node-408-2026-10-06)
closes the official launcher metadata slice at candidate/oracle 6/6 with all
143 pinned descriptors and advances exactly six Core rows to expected.

NODE-407: [sync service-worker policy](python-official-conformance.md#sync-service-worker-policy-checkpoint-2026-10-06)
passes both remaining rows in candidate and oracle under trusted Tailscale
HTTPS, bringing cumulative Python execution to 1,190 passes.

NODE-406: [Keyboard-plus and shared-environment evidence](python-official-conformance.md#keyboard-plus-and-shared-environment-checkpoint-2026-10-06)
promotes the paired-green `Keyboard.press("+")` row to reach 1,188 cumulative
passes while keeping two candidate/oracle shared reds unpromoted.

NODE-403: [sync Page and caller-stack parity](python-official-conformance.md#sync-page-and-caller-stack-checkpoint-2026-10-06)
passes the same seven Chromium rows in the installed-wheel candidate and
Playwright 1.58.0 oracle, advancing cumulative Python execution to 1,187
passes. The two headed `Page.pause` environment mismatches remain unrun.

NODE-402: [sync request-fulfill parity](python-official-conformance.md#sync-request-fulfill-parity-checkpoint-2026-10-06)
passes the two remaining Chromium rows in candidate and oracle lanes without a
runtime change, bringing cumulative Python execution to 1,180 passes.

NODE-401: [complete official TodoMVC execution](python-official-conformance.md#complete-todomvc-execution-checkpoint-2026-10-06)
passes the same isolated 25/25 cases in the portable-wheel candidate and
Playwright 1.58.0 oracle. Exactly 25 prior not-run rows become passes, bringing
the cumulative Python ledger to 1,178 passes / eight skips / 5,340 not run.

NODE-400: [Python ABI104 typed string matchers](python-ffi-compatibility-design.md#abi-104-composable-typed-string-matchers-2026-10-06)
keeps exact/regex value and class matching in Rust, including mixed ordered
class arrays. The four unchanged upstream Chromium cases pass in candidate and
oracle lanes, and the cumulative Python ledger reaches 1,153 passes.

NODE-398: [profile-aware Python fixture definitions](python-official-conformance.md#profile-aware-fixture-definition-checkpoint-2026-10-06)
resolves 95 definitions across all 6,526 rows with zero unresolved providers.
All 25 TodoMVC cases receive the plugin Page closure; every current outcome is
partitioned by fixture-induced runtime without confusing it with body-level
browser launch.

NODE-397: [profile-aware Python fixture closure](python-official-conformance.md#profile-aware-pytest-fixture-closure-checkpoint-2026-10-06)
collects core and TodoMVC jobs in their separate frozen environments without
executing fixtures. Core-only server/asyncio dependencies and plugin-only
example dependencies remain explicit.

NODE-396: [official Python CI profiles](python-official-conformance.md#official-ci-profile-reconstruction-checkpoint-2026-10-06)
replaces 25 obsolete root-only TodoMVC IDs with official `[chromium]` IDs,
strictly rebases only pristine not-run rows and makes the profiled inventory
canonical. Historical NODE-394/395 evidence remains marked superseded.

NODE-393: [direct pytest-input inventory](python-official-conformance.md#direct-pytest-input-inventory-checkpoint-2026-10-06)
adds ordered source parameters plus applicability/outcome cross-tabs to all
6,526 Python audit rows. NODE-394 verifies those direct lists exactly against
pytest while retaining the complete closure as a separate evidence layer.

NODE-392: [browserless Python official evidence](python-official-conformance.md#browserless-sync-lifecycle-checkpoint-2026-10-06)
adds a manifest- and source-verified no-browser lane while preserving mandatory
Tailscale fixture servers and ordinary Chrome isolation. Candidate and oracle agree on three green stop-
lifecycle rows and three topology-dependent failures; only the green evidence
advances cumulative Python execution to 1,149 passes / eight skips.

NODE-391: [Core applicability evidence](upstream-playwright-conformance.md)
keeps 6,279 public behaviors with private test helpers in the required
execution queue instead of silently treating them as internal. The exact
implementation class contracts to 3,458 explicitly reasoned rows.

NODE-390: [Core applicability evidence](upstream-playwright-conformance.md)
recomputes private client access per invocation instead of hiding an entire
source file. It restores 1,361 public-behavior rows and six product rows while
keeping shared private fixtures explicit and preserving every outcome.

NODE-389: [Core applicability evidence](upstream-playwright-conformance.md)
keeps all 67 official example-application invocations in the complete ledger
while separating them from public-API conformance. The generated audit now
retains inventory roles and records 5,574 product-infrastructure rows.

NODE-388: [Core applicability evidence](upstream-playwright-conformance.md)
corrects the copied EventEmitter classification without changing any outcome.
All 477 repository-local EventEmitter invocations remain explicit; 441 false
public candidates move to implementation-internal, and the full 48,477-row
audit remains one-to-one with the official inventory.

NODE-387: [Python ABI 103 composable ElementHandle keyboard](python-ffi-compatibility-design.md#abi-103-composable-elementhandle-keyboard-checkpoint-2026-10-06)
keeps exact-element focus and Page input in one Rust command while thin
Page/Frame selector facades reuse existing native owners. The fixed keyboard
tranche closes every deterministic candidate discrepancy; 26 passes and one
upstream skip advance cumulative execution to 1,147 passes / eight skips.
Two shared-environment red rows remain explicit and unmerged.

NODE-386: [Python ABI 102 composable Page Keyboard](python-ffi-compatibility-design.md#abi-102-composable-page-keyboard-checkpoint-2026-10-06)
exposes the existing bounded Rust Keyboard through thin async/sync facades.
The fixed focus tranche moves from candidate 3/5 versus oracle 5/5 to fresh
installed candidate 5/5; cumulative Python execution reaches 1,121 passes
with no recorded failure.

NODE-385: [Python ABI 101 composable resource timing](python-ffi-compatibility-design.md#abi-101-composable-resource-timing-checkpoint-2026-10-06)
keeps timing snapshots and publication sequencing in bounded Rust owners. The
fixed official tranche moves from candidate 2/4 versus oracle 4/4 to fresh
installed candidate 4/4; cumulative Python execution reaches 1,119 passes
with the two known focus failures still explicit.

NODE-384: [Python ABI 100 request-fulfill evidence](python-ffi-compatibility-design.md#abi-100-request-fulfill-evidence-checkpoint-2026-10-06)
closes the two previously unrun Chromium rows at candidate/oracle 2/2, making
the upstream file 3/3 without a runtime change. Cumulative Python candidate
execution reaches 1,110 passes.

NODE-383: [Python ABI 100 composite JSHandle evaluation](python-ffi-compatibility-design.md#abi-100-composite-jshandle-checkpoint-2026-10-06)
keeps nested Handle arguments and property batches in bounded Rust owners while
the thin facade handles only public wrappers and result shapes. The fixed
official tranche moves from candidate 6/15 versus oracle 15/15 to installed
candidate 15/15; cumulative Python execution reaches 1,108 passes.

NODE-382: [Python ABI 99 native directory input](python-ffi-compatibility-design.md#abi-99-native-directory-input-checkpoint-2026-10-06)
keeps directory preflight, native DOM mutation and cancellation cleanup in an
opt-in Rust leaf without treating ordinary remote paths as colocated. The
installed candidate passes 17/18 fixed input rows; all 16 oracle-executable
rows agree, and cumulative Python execution reaches 1,093 passes.

NODE-362: [Composable proxy target TLS](api-request-design.md) injects the
existing Rustls owner over established HTTP CONNECT or SOCKS5 tunnels without
adding a TLS dependency to either proxy leaf. The unchanged official
proxy+mTLS rows pass 18/18 candidate and 18/18 oracle across Library and Bidi;
full-corpus completion, bypass and authenticated SOCKS remain open.

NODE-361: [Bidi client-certificate evidence](upstream-playwright-conformance.md)
runs the eight already-supported global-request rows through all six Bidi
projects. Candidate and oracle each pass validation 6/6 plus direct requests
42/42 with a freshly built selected artifact and no browser/CDP.

NODE-360: [Bidi global-request proxy evidence](upstream-playwright-conformance.md)
runs the existing HTTP CONNECT, HTTPS proxy-server and SOCKS5 profiles through
all six Bidi projects. Candidate and oracle each pass 18/18 without a runtime
change or browser/CDP dependency.

NODE-359: [Node SOCKS5 proxy composition](api-request-design.md) adds an
independently selectable bounded Rust connector with remote hostname
resolution. The unchanged official SOCKS row passes 3/3 candidate and 3/3
oracle without CDP. HTTPS target tunneling, authenticated SOCKS, bypass and
full-corpus completion remain open.

NODE-358: [HTTPS proxy-server composition](api-request-design.md) reuses the
optional CONNECT leaf with the injected Rustls connector. The unchanged
official ALPN row passes 3/3 candidate and 3/3 oracle. This covers TLS to a
proxy for a plain-HTTP target, not HTTPS target tunneling; SOCKS, bypass and
full-corpus completion remain open.

NODE-357: [Node HTTP CONNECT proxy](api-request-design.md) adds a separately
selectable bounded Rust connector under the existing API request owner. The
unchanged official authenticated global-request proxy row passes 3/3 candidate
and 3/3 oracle; HTTPS-over-proxy, SOCKS, bypass and full-corpus completion
remain open.

NODE-353: [Python ABI 94 popup-event lifecycle](python-ffi-compatibility-design.md#abi-94-popup-event-lifecycle-checkpoint-2026-10-06)
publishes paused-popup identity before dialog-blocked world initialization
resumes, settles immediate-close console Handles at the observer boundary and
revokes unhandled Dialog actions before transport shutdown. Candidate and
official Playwright agree on 11 applicable passes; the one shared source/oracle
drift remains `not-run`, and the ledger reaches 907 passes.

NODE-346: [Python ABI 93 Rust-owned route redirects](python-ffi-compatibility-design.md#abi-93-rust-owned-route-redirect-checkpoint-2026-10-05)
keeps redirect continuation, bounded header replay and exact native Request
identity in Rust while Python only links public redirect facades.
Candidate/oracle pass 2/2; the full Page-route file reaches 43 pass / 1 fail
and the ledger reaches 853 passes.

NODE-345: [Python ABI 92 Rust-owned APIResponse route fulfillment](python-ffi-compatibility-design.md#abi-92-rust-owned-apiresponse-route-fulfillment-checkpoint-2026-10-05)
composes the existing Page-route and API-request owners without copying body
bytes through Python. Candidate/oracle pass 1/1; the full Page-route file
reaches 41 pass / 3 fail and the ledger reaches 851 passes.

NODE-344: [Python ABI 91 native CORS preflight](python-ffi-compatibility-design.md#abi-91-native-cors-preflight-checkpoint-2026-10-05)
keeps browser-generated preflight detection, 204 fulfillment and finite route
budgets in the bounded Rust route owner. Candidate/oracle pass 2/2; the full
Page-route file reaches 40 pass / 4 fail and the ledger reaches 850 passes.

NODE-343: [Python ABI 91 exact routed Request response](python-ffi-compatibility-design.md#abi-91-exact-routed-requestresponse-checkpoint-2026-10-05)
correlates a routed Request to its network Response by exact bounded native
identity in Rust, without URL guessing or a Python registry. Candidate/oracle
pass 1/1; the full Page-route file reaches 38 pass / 6 fail and the ledger
reaches 848 passes.

NODE-342: [Python ABI 90 composable route binding and Rust fulfill-path](python-ffi-compatibility-design.md#abi-90-composable-route-binding-and-fulfill-path-checkpoint-2026-10-05)
keeps language-native regex/predicate identity in the thin binding while Rust
owns route fallback and settlement, then adds bounded Rust file fulfillment.
Candidate/oracle pass 2/2; the full Page-route file reaches 37 pass / 7 fail
and the ledger reaches 847 passes.

NODE-340: [Python ABI 89 exact Request Frame identity](python-ffi-compatibility-design.md#abi-89-exact-request-frame-identity-checkpoint-2026-10-05)
keeps native Frame descriptors and cursor-relative availability in bounded
Rust owners while Python reuses the Page Frame registry. Candidate/oracle pass
request-frame 6/6 and route-frame 1/1; the full Page-route file reaches 35
pass / 9 fail and the ledger reaches 845 passes.

NODE-339: [Python ABI 87 request lifecycle events](python-ffi-compatibility-design.md#abi-87-request-lifecycle-events-checkpoint-2026-10-05)
keeps request start/finish/failure publication and exact native correlation in
one bounded Rust owner. Candidate/oracle pass 2/2 and repeat 10/10; the full
Page-route file reaches 34 pass / 10 fail and the ledger reaches 839 passes.

NODE-338: [ABI 86 Route.fulfill status text](python-ffi-compatibility-design.md#abi-86-routefulfill-status-text-checkpoint-2026-10-05)
keeps the fixed Playwright reason-phrase mapping and CDP serialization in
Rust. The unchanged 422 row passes candidate/oracle 1/1 and repeat 5/5,
the full Page-route file reaches 32 pass / 12 fail and the ledger reaches 837
passes.

NODE-337: [ABI 86 public route-header evidence](python-ffi-compatibility-design.md#abi-86-public-route-header-evidence-checkpoint-2026-10-05)
promotes two additional native `Referer`/`Origin` lowercase rows without a
runtime change. Candidate/oracle pass 2/2 and repeat 10/10; the complete
Page-route file is 31 pass / 13 fail and the ledger reaches 836 passes.

NODE-336: [Python ABI 86 Page and Context extra HTTP headers](python-ffi-compatibility-design.md#abi-86-page-and-context-extra-http-headers-checkpoint-2026-10-05)
keeps header precedence, fanout, referrer composition and request-header
normalization in bounded Rust ownership. Three unchanged rows pass
candidate/oracle 3/3 and repeat 15/15, moving the ledger to 834 passes;
remaining Python and complete Core JS/TS coverage stay open.

NODE-335: [Python ABI 85 route fallback and handler unroute](python-ffi-compatibility-design.md#abi-85-route-fallback-and-handler-unroute-checkpoint-2026-10-05)
keeps traversal, effective overrides, rematching and settlement in bounded
Rust ownership. Eight unchanged rows pass candidate/oracle 8/8 and repeat
40/40, moving the ledger to 831 passes; regex/predicate and broader corpora
remain open.

NODE-334: [Python ABI 84 finite route uses](python-ffi-compatibility-design.md#abi-84-finite-route-use-checkpoint-2026-10-05)
keeps Page/Context `times` matching, decrement and expiration in one bounded
Rust owner. Three unchanged rows pass candidate/oracle 3/3 and repeat 15/15,
moving the ledger to 823 passes; fallback/unroute and broader corpora remain
open.

NODE-333: [ABI 83 route-glob question marks](python-ffi-compatibility-design.md#abi-83-route-glob-question-mark-checkpoint-2026-10-05)
makes `?` literal in Rust and bounded CDP Fetch patterns. The unchanged row
passes candidate/oracle 1/1 and repeat 5/5, moving the ledger to 820 passes;
broader Page-route, Python and Core JS/TS work remains open.

NODE-332: [Python ABI 83 Route continue overrides](python-ffi-compatibility-design.md#abi-83-route-continue-override-checkpoint-2026-10-05)
adds bounded URL, method, header and POST-data override composition to the Rust
route owner. Ten unchanged rows pass candidate/oracle 10/10, the ledger
reaches 819 passes and the full Page-route file improves to 23 pass / 21 fail
/ 0 errors while the broader goal remains open.

NODE-331: [ABI 82 full Page-route baseline](python-ffi-compatibility-design.md#abi-82-full-page-route-baseline-checkpoint-2026-10-05)
runs all 44 async Chromium rows in isolation. Candidate records 18 pass / 25
fail / 1 error; the 18 green rows pass oracle parity and move the ledger to 809
passes while every remaining discrepancy stays explicit.

NODE-330: [ABI 82 expanded unroute-all evidence](python-ffi-compatibility-design.md#abi-82-expanded-unroute-all-evidence-checkpoint-2026-10-05)
promotes six unchanged default-removal and in-flight-wait rows. Candidate and
oracle pass 6/6 and the ledger reaches 791 passes without an implementation
change.

NODE-329: [Python ABI 82 scoped unroute-all](python-ffi-compatibility-design.md#abi-82-scoped-unroute-all-checkpoint-2026-10-05)
adds Page/Context removal and `behavior="wait"` settlement to the bounded Rust
route owner. Two unchanged HAR-abort rows pass candidate/oracle 2/2 and stress
20/20; the ledger reaches 785 passes.

NODE-328: [Python ABI 81 BrowserContext route composition](python-ffi-compatibility-design.md#abi-81-browsercontext-route-composition-checkpoint-2026-10-05)
adds ordinary async/sync Context routes to the existing bounded Rust route
owner. Four unchanged HAR URL-filter composition rows pass candidate/oracle
4/4 and stress async 20/20 plus sync 20/20; the ledger reaches 783 passes.

NODE-327: [ABI 80 missing-entries HAR fallback](python-ffi-compatibility-design.md#abi-80-missing-entries-har-fallback-checkpoint-2026-10-04)
aligns a present `log` without `entries` to a bounded empty archive. The two
unchanged rows pass candidate/oracle 2/2 and stress 10/10 per API; the ledger
advances to 779 passes without broad parse-error suppression.

NODE-326: [Python ABI 80 BrowserContext HAR replay](python-ffi-compatibility-design.md#abi-80-browsercontext-har-replay-checkpoint-2026-10-04)
adds Rust-owned Context scope, Page-over-Context precedence and a bounded
future-Page/OOPIF initializer. Six unchanged rows pass candidate/oracle 6/6;
stress passes async 30/30 plus sync 30/30 and the ledger reaches 777 passes.

NODE-325: [ABI 79 Page HAR URL-filter composition](python-ffi-compatibility-design.md#abi-79-page-har-url-filter-composition-checkpoint-2026-10-04)
promotes the complete unchanged async/sync URL-filter/no-fallback pair on the
installed NODE-323 wheel. Candidate/oracle pass 2/2 and API-separated stress
passes 10/10 each; the ledger advances to 771 passes with no code/ABI change.

NODE-324: [ABI 79 async/sync Page HAR evidence](python-ffi-compatibility-design.md#abi-79-async-and-sync-page-har-evidence-checkpoint-2026-10-04)
promotes the complete unchanged method/redirect pair on the installed NODE-323
wheel. Candidate/oracle pass 2/2 and API-separated stress passes 10/10 each;
the ledger advances to 769 passes with no losses or code/ABI change.

NODE-323: [Python ABI 79 persistent-Context plain-HAR replay](python-ffi-compatibility-design.md#abi-79-persistent-context-plain-har-replay-checkpoint-2026-10-04)
adds a backend-neutral bounded HAR policy over the existing Page-route Fetch
driver plus Rust-owned computed-style assertion polling. The unchanged
Chromium row passes installed candidate/oracle 1/1 and candidate stress 10/10;
the ledger advances to 767 passes with no losses. Broader HAR capability,
remaining Python and complete Core JS/TS corpora remain open.

NODE-322: [Python ABI 78 HTTP credentials and response bodies](python-ffi-compatibility-design.md#abi-78-http-credentials-and-response-body-checkpoint-2026-10-04)
adds independent ordinary/persistent Context authentication and a bounded
navigation response-body leaf. Eleven unchanged Chromium rows pass installed
candidate/oracle 11/11 and candidate stress 110/110; the ledger advances to
766 passes with no losses. The remaining Python and complete Core JS/TS
corpora remain open.

NODE-317: [Python ABI 72 Context CSP bypass](python-ffi-compatibility-design.md#abi-72-context-csp-bypass-checkpoint-2026-10-04)
adds independent owned-Context and persistent-default composition through one
bounded Rust policy owner. Four unchanged Chromium rows pass final installed
candidate/oracle 4/4 and candidate stress 40/40; the ledger advances to 748
passed with no losses. Eight original default-Context rows remain, and the
separate iframe CSP row stays unclaimed behind `ElementHandle.content_frame`.

NODE-316: [Python ABI 71 persistent-Context extra HTTP headers](python-ffi-compatibility-design.md#abi-71-persistent-context-extra-http-headers-checkpoint-2026-10-04)
composes persistent launch with the existing Rust header owner instead of
creating a second header store. The unchanged Chromium row passes installed
candidate/oracle 1/1 and candidate stress 10/10; the ledger advances to 744
passed with no losses. Nine original default-Context rows remain.

NODE-315: [Python ABI 70 persistent-Context media defaults](python-ffi-compatibility-design.md#abi-70-persistent-context-media-defaults-checkpoint-2026-10-04)
composes the existing Rust media owner with persistent launch so color scheme,
contrast and reduced motion are installed before the initial Page is
published. The three unchanged Chromium rows pass candidate/oracle 3/3 and
candidate stress 30/30; the ledger advances to 743 passed with no losses.

NODE-314: [Python ABI 69 persistent launch-argument preflight](python-ffi-compatibility-design.md#abi-69-persistent-context-launch-argument-preflight-checkpoint-2026-10-04)
adds bounded binding-side validation for page-opening Chromium arguments before
any native launch. The unchanged official row passes candidate/oracle 1/1 and
candidate stress 10/10; the ledger advances to 740 passed with no ABI change.
Non-empty switch-only arguments remain explicitly unsupported.

NODE-313: [Python ABI 69 BrowserContext cookies](python-ffi-compatibility-design.md#abi-69-browsercontext-cookie-checkpoint-2026-10-04)
adds an independently selectable Rust `context-cookies` leaf over Chromium's
browser-root Storage domain. The two fixture-independent official rows pass
candidate/oracle 2/2 and stress 20/20; two `localhost` assertions reproduce in
both lanes as MagicDNS applicability gaps. The ledger advances to 739 passed
with no losses and without claiming those two rows.

NODE-312: [Python ABI 68 BrowserContext event facade](python-ffi-compatibility-design.md#abi-68-browsercontext-event-facade-checkpoint-2026-10-04)
adds async/sync `on`, `once` and `remove_listener` over the existing bounded
Rust lifecycle path, including exact sync facade identity and one existing
Context-console subscription lease. The unchanged official persistent-close
row passes candidate/oracle 1/1 and candidate stress 10/10; the ledger advances
to 737 passed with no losses.

NODE-311: [Python ABI 68 persistent-Context device emulation](python-ffi-compatibility-design.md#abi-68-persistent-context-device-emulation-checkpoint-2026-10-04)
admits the default viewport, screen, scale, mobile, touch and user-agent policy
before initial Page topology while reusing the existing bounded device
initializer. Five unchanged official rows pass candidate/oracle 5/5 and
candidate stress 50/50; the ledger advances to 736 passed with no losses.

NODE-310: [Python ABI 67 persistent-Context HTTPS-error bypass](python-ffi-compatibility-design.md#abi-67-persistent-context-ignore_https_errors-checkpoint-2026-10-04)
admits the default policy before initial Page topology while reusing the
existing bounded HTTPS initializer and 16-Context capacity. The unchanged
official row passes candidate/oracle 1/1 and candidate stress 10/10; the ledger
advances to 731 passed with no losses.

NODE-309: [ABI 66 persistent-session release](python-ffi-compatibility-design.md#abi-66-persistent-session-release-checkpoint-2026-10-04)
makes default `BrowserContext.close()` release its unique persistent native
Browser session and the closed Python Browser owner. Three unchanged official
profile-baseline rows pass candidate/oracle 3/3 and same-process stress 30/30;
the ledger advances to 730 passed with no losses.

NODE-308: [ABI 66 `wait_for_function` timeout acknowledgment](python-ffi-compatibility-design.md#abi-66-wait_for_function-timeout-acknowledgment-checkpoint-2026-10-04)
cancels polling at the public deadline but waits for the current admitted
predicate attempt and publication fence to settle before returning `Timeout`.
All six official Chromium rows pass candidate and oracle, the final stress
passes 60/60, and the ledger advances to 727 passed with no losses.

NODE-307: [Python ABI 66 persistent-Context locale](python-ffi-compatibility-design.md#abi-66-persistent-context-locale-checkpoint-2026-10-04)
admits the default locale before initial Page topology and composes Chromium's
single user-agent/language override without erasing a selected device UA,
followed by the ICU locale override. The installed-wheel composition probe and
the unchanged official row pass; the latter also passes oracle and 10/10
repeat, and the ledger advances to 721 passed with no losses.

NODE-306: [Python ABI 65 Context locale](python-ffi-compatibility-design.md#abi-65-context-locale-checkpoint-2026-10-04)
retains one bounded locale per owned Context and applies it during Page/OOPIF
initialization. Dedicated and nested Workers inherit the renderer locale as in
upstream Playwright. The official row passes candidate, oracle and 10/10
repeat; the complete Worker file passes 13/13 and the ledger advances to 720
passed with no losses.

NODE-305: [Python ABI 64 Worker console argument Handles](python-ffi-compatibility-design.md#abi-64-worker-console-argument-handle-checkpoint-2026-10-04)
atomically transfers ordered console RemoteObjects into the existing bounded
Worker Handle registry and dispatches `json_value` in the exact Worker world.
The unchanged official row passes in candidate and oracle, the complete Worker
file advances to 12/13, and the ledger advances to 719 passed with no losses.

NODE-304: [Python ABI 63 Worker Handle evaluation](python-ffi-compatibility-design.md#abi-63-independent-worker-handle-evaluation-checkpoint-2026-10-04)
creates and evaluates Handles in the exact retained Worker session through a
bounded Rust owner and a weak Handle-to-Worker backreference. The unchanged
official lifecycle row passes in final installed candidate and oracle, the
complete Worker file advances to 11/13, and the ledger advances to 718 passed
with no losses. This is a selected slice, not full Worker Handle parity.

NODE-303: [Python ABI 62 Worker network attribution](python-ffi-compatibility-design.md#abi-62-independent-worker-network-event-checkpoint-2026-10-04)
routes retained Worker sessions through the existing Rust-owned Page network
lane using exact scoped native IDs and a bounded related-session registry.
Both unchanged async Chromium rows pass in candidate and oracle, the complete
Worker file advances to 10/13, and the ledger advances to 717 passed with no
losses and no ABI change.

NODE-302: [Python ABI 62 Worker error events](python-ffi-compatibility-design.md#abi-62-independent-worker-error-event-checkpoint-2026-10-04)
maps an exact retained Worker session's exception to its owning Page through
the existing Rust-owned PageError pipeline. The unchanged async Chromium row
passes in candidate and oracle; the ledger advances to 715 passed with no
losses and no ABI change.

NODE-301: [Python ABI 62 Worker console events](python-ffi-compatibility-design.md#abi-62-independent-worker-console-event-checkpoint-2026-10-04)
adds an independent Rust-owned Worker-session console FIFO and publishes one
shared message identity through thin Worker/Page/Context Python facades. Three
unchanged async Chromium rows pass in candidate and oracle; the ledger advances
to 714 passed with no losses. Worker console argument Handles remain separate.

NODE-300: [Python ABI 61 Worker retirement](python-ffi-compatibility-design.md#abi-61-worker-retirement-checkpoint-2026-10-04)
releases native Worker owners on every Page/Context/Browser terminal path and
tolerates exact late detach notices after Page removal. Four unchanged async
Chromium Worker rows pass; the ledger advances to 711 passed with no losses.
Worker console/error, Handle, locale and network-attribution capabilities are
still explicit gaps.

NODE-299: [Python ABI 61 caller dispatcher](python-ffi-compatibility-design.md#abi-61-caller-dispatcher-checkpoint-2026-10-04)
adds the official caller-loop/greenlet sync scheduling contract without moving
browser work out of Rust. The complete unchanged Chromium sync file now passes
27/27; the wider Python and Core JS/TS corpora remain unfinished.

NODE-298: [Python ABI 61 Worker events](python-ffi-compatibility-design.md#abi-61-independent-worker-event-checkpoint-2026-10-04)
retains and evaluates dedicated Workers through bounded Rust owners with thin
async/sync Python facades. The unchanged Worker row passes and the complete
Chromium sync file is 26/27; caller-loop `sync_gather` remains open.

NODE-297: [Python ABI 60 download events](python-ffi-compatibility-design.md#abi-60-independent-download-event-checkpoint-2026-10-04)
owns download policy, event correlation, artifact lifetime and click-navigation
completion in Rust. The unchanged download row passes and the complete sync
file advances to 25/27 at that checkpoint.

NODE-296: [Python ABI 59 request events](python-ffi-compatibility-design.md#abi-59-independent-request-event-checkpoint-2026-10-04)
projects bounded Rust request notices to exact async/sync Pages without adding
a Python CDP reader or retained queue. The unchanged network-event row passes.

NODE-295: [Python ABI 58 console Handles and subscriptions](python-ffi-compatibility-design.md#abi-58-console-handle-and-subscription-checkpoint-2026-10-04)
publishes ordered Rust-owned console argument Handles and bounded native
subscriptions. Both complete pinned console modules pass in candidate.

NODE-294: [Python ABI 57 console-event projection](python-ffi-compatibility-design.md#abi-57-console-event-projection-checkpoint-2026-10-03)
projects the existing bounded Rust console observer through the ordered Page
lifecycle fence into thin async/sync Python callbacks. Source, package, type,
Clippy and dependency gates pass; retained argument Handles and the unchanged
official browser comparison remain open, so the Python ledger has not moved.

NODE-293: [Node ElementHandle DOM source completion](node-ffi-compatibility-design.md#node-elementhandle-dom-source-completion-2026-10-03)
moves `textContent`, `$$` and `$$eval` query-array ownership into Rust and
aligns scoped XPath filtering with upstream. Package, type, native-load,
Clippy and boundary gates pass; the unchanged 17-row browser comparison and
ledger merge are pending because the shared dedicated Chrome is not empty.

NODE-292: [Node ElementHandle DOM composition](node-ffi-compatibility-design.md#node-elementhandle-dom-checkpoint-2026-10-03)
adds an independently selectable native profile for Rust-owned first-descendant
query and `innerText`. The unchanged official focused selection passes 15/15
in both candidate and oracle; the Core candidate still has 31 unexpected and
47,229 not-run invocations, so this is not a full-suite claim.

NODE-291: [Python ABI 56 ElementHandle DOM composition](python-ffi-compatibility-design.md#abi-56-elementhandle-dom-checkpoint-2026-10-03)
moves descendant query and `inner_text` semantics into an optional Rust
capability with thin Python bindings. The two official Chromium regressions
pass 2/2; the repaired sync file advances to 18/27 candidate while nine
failures and all 48,477 Core JS/TS invocations remain open.

NODE-290: [Python ABI 55 sync API inventory](python-ffi-compatibility-design.md#abi-55-sync-api-inventory-checkpoint-2026-10-03)
records all 25 previously unexecuted Chromium rows in unchanged
`test_sync.py`. Five thin-boundary repairs raise the tranche from 9/25 to
14/25, while 11 failures remain explicit; together with two earlier passes,
all 27 Chromium rows now have outcomes. The other 5,828 candidate Python rows
and all 48,477 Core JS/TS invocations remain open.

NODE-289: [Python ABI 55 Page evaluate error shape](python-ffi-compatibility-design.md#abi-55-page-evaluate-api-name-checkpoint-2026-10-03)
completes the five applicable Chromium rows in async `test_asyncio.py`. The
two newly selected rows pass in candidate and oracle after the thin Python
public boundary adds `Page.evaluate:` to the existing native
`JavascriptError` without changing its Rust classification; the other 5,853
candidate Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-288: [Python ABI 55 selector-wait error shape](python-ffi-compatibility-design.md#abi-55-selector-wait-error-shape-checkpoint-2026-10-03)
adds public `.name` access to the existing native exception taxonomy. Three
Chromium selector-wait rows pass in candidate and oracle across cancellation,
timeout naming and sync execution; the other 5,855 candidate Python rows and
all 48,477 Core JS/TS invocations remain open.

NODE-287: [Python ABI 55 Page selector-wait parity](python-ffi-compatibility-design.md#abi-55-page-wait-for-selector-checkpoint-2026-10-03)
adds thin async/sync `Page.wait_for_selector` facades over one Rust-owned total
deadline and existing Handle publication. Two applicable Chromium issue rows
pass in candidate and oracle, including pending-task cancellation; the other
5,858 candidate Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-286: [Python ABI 54 Page file-chooser parity](python-ffi-compatibility-design.md#abi-54-page-file-chooser-checkpoint-2026-10-03)
composes exact-session Rust-owned chooser interception/publication with the
existing bounded file-input owner. The complete sync input file passes both
Chromium rows in final repaired-wheel candidate and oracle; its four provider
gaps, the other 5,860 candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

NODE-285: [Python ABI 53 Page request parity](python-ffi-compatibility-design.md#abi-53-page-network-request-checkpoint-2026-10-03)
adds bounded Rust-owned Context request waits, exact request-history ownership,
post-data parsing and same-CDP-cursor Frame availability. The complete async
Page network-request file passes both Chromium rows in candidate and oracle;
its four provider gaps, the other 5,862 candidate Python rows and all 48,477
Core JS/TS invocations remain open.

NODE-284: [Python ABI 52 bounded response completion](python-ffi-compatibility-design.md#abi-52-bounded-response-completion-checkpoint-2026-10-03)
adds Rust-owned terminal success/failure/closure state behind thin async/sync
`Response.finished()` facades. Both complete Page network-response files pass
their four Chromium rows in candidate and oracle; eight provider gaps, the
other 5,864 candidate Python rows and all 48,477 Core JS/TS invocations remain
open.

NODE-283: [Python ABI 51 bounded timeout waits](python-ffi-compatibility-design.md#abi-51-bounded-page-timeout-wait-checkpoint-2026-10-03)
exports the existing independently selectable Rust timeout leaf through thin
async/sync `Page.wait_for_timeout` facades. The complete shared signal file
passes both Chromium rows in candidate and oracle through the dedicated CDP
Chrome; its four provider gaps, the other 5,868 candidate Python rows and all
48,477 Core JS/TS invocations remain open.

NODE-282: [Python ABI 50 sync event-error propagation](python-ffi-compatibility-design.md#abi-50-sync-event-error-propagation-checkpoint-2026-10-03)
adds a one-error-bounded sync bridge slot so Page listener exceptions surface
at a public call boundary after the call completes. The complete shared event
file passes its Chromium row in candidate and oracle; its two provider gaps,
the other 5,870 candidate Python rows and all 48,477 Core JS/TS invocations
remain open.

NODE-281: [Python ABI 50 cross-thread sync lifecycle](python-ffi-compatibility-design.md#abi-50-cross-thread-sync-lifecycle-checkpoint-2026-10-03)
records the complete unchanged shared thread file. Candidate and oracle pass
its Chromium row with the existing bounded sync loop and Rust owners; its two
provider gaps, the other 5,871 candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

NODE-280: [Python ABI 50 test-id strict-error parity](python-ffi-compatibility-design.md#abi-50-test-id-strict-error-checkpoint-2026-10-03)
adds bounded Rust-owned `set_test_id_attribute` state and finite strict-error
DOM/test-id suggestions. The complete async selector-generator file passes its
Chromium row in candidate and oracle; its two provider gaps, the other 5,872
candidate Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-279: [Python ABI 50 legacy text selector parity](python-ffi-compatibility-design.md#abi-50-legacy-text-selector-checkpoint-2026-10-03)
adds typed immediate-text legacy selectors, quote-aware selector-chain
tokenization, bounded Page query handle publication and thin ID assertions.
Both Chromium rows from the complete unchanged async selector file pass in
candidate and oracle; its four Firefox/WebKit provider rows, the other 5,873
candidate Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-278: [Python ABI 50 bounded screenshots](python-ffi-compatibility-design.md#abi-50-screenshot-checkpoint-2026-10-03)
adds Page, Locator and exact retained ElementHandle screenshot facades over
bounded Rust owners, including mask cleanup and document-space clipping. All
three Chromium rows from the complete unchanged async screenshot file pass in
candidate and oracle; its six Firefox/WebKit provider rows, the other 5,875
candidate Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-277: [Python ABI 49 tagged PDF outlines](python-ffi-compatibility-design.md#abi-49-tagged-pdf-outline-checkpoint-2026-09-28)
extends the existing Rust PDF owner with two typed booleans and closes all
three applicable rows from the complete unchanged async PDF file in candidate
and independent-oracle lanes. The other 5,878 Python rows and all 48,477 Core
JS/TS invocations remain open.

NODE-276: [Python ABI 48 bounded Page PDF](python-ffi-compatibility-design.md#abi-48-page-pdf-checkpoint-2026-09-28)
adds a separately selectable Rust capture/output owner and closes both
applicable rows from the complete unchanged sync PDF file in candidate and
independent-oracle lanes. The other 5,881 Python rows and all 48,477 Core
JS/TS invocations remain open.

NODE-275: [Python ABI 47 HTTPS-error policy](python-ffi-compatibility-design.md#abi-47-ignore-https-errors-checkpoint-2026-09-28)
adds a bounded Rust-owned Context/session policy and a thin boolean Python
option. Both Chromium rows from the complete unchanged async file pass in
candidate and oracle; its four Firefox/WebKit rows, 5,883 other candidate
Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-274: [Python ABI 46 fill and context managers](python-ffi-compatibility-design.md#abi-46-fill-and-context-manager-checkpoint-2026-09-28)
runs both complete unchanged fill files and both complete context-manager files.
Candidate and oracle each pass 9/9; 5,885 candidate Python rows and all 48,477
Core JS/TS invocations remain open.

NODE-273: [Python ABI 46 native collection evaluation](python-ffi-compatibility-design.md#abi-46-native-locator-collection-evaluation-checkpoint-2026-09-28)
lets a typed native Locator execute `evaluate_all` through the existing bounded
Rust owner. The complete unchanged async semantic-selector file passes 3/3 in
candidate and oracle; 5,894 candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

NODE-272: [Python ABI 45 semantic Locators](python-ffi-compatibility-design.md#abi-45-semantic-locator-checkpoint-2026-09-28)
adds typed Rust-owned `get_by_*` leaves and attribute-expectation polling. The
complete unchanged sync semantic-Locator file passes 10/10 in candidate and
oracle; 5,897 candidate Python rows and all 48,477 Core JS/TS invocations
remain open.

NODE-257: [Python ABI 35 typed Locator composition](python-ffi-compatibility-design.md#abi-35-typed-locator-composition-checkpoint-2026-09-28)
keeps chained locator and nth recipes in bounded Rust state. Six unchanged
async/sync Chromium rows pass candidate and oracle; 6,014 candidate Python
rows and the 48,477-row Core JS/TS corpus remain open.

NODE-256: [Python ABI 34 Locator description metadata](python-ffi-compatibility-design.md#abi-34-locator-description-checkpoint-2026-09-28)
adds immutable `description` / `describe` facade metadata. The three upstream
async Chromium rows pass candidate and oracle; 6,020 candidate Python rows and
the 48,477-row Core JS/TS corpus remain open.

NODE-255: [Python ABI 34 Locator execution and count expectation](python-ffi-compatibility-design.md#abi-34-locator-execution-and-count-expectation-checkpoint-2026-09-28)
adds Rust-owned bounded polling for `expect(locator).to_have_count()` and thin
Python/Node assertion adapters. Ten fixed unchanged async/sync Chromium rows
pass candidate and oracle; 6,023 candidate Python rows, Firefox/WebKit and the
48,477-row Core JS/TS corpus remain open.

NODE-254: [Python ABI 33 basic Locator facades](python-ffi-compatibility-design.md#abi-33-basic-locator-facade-checkpoint-2026-09-27)
forwards basic reads, collection reads, fill/check/focus/press and evaluation to
the existing Rust owners, with Rust-owned Playwright `:text()` parsing. The 34
fixed async/sync Chromium rows pass candidate and oracle; 6,033 candidate rows
and Core JS/TS remain open.

NODE-253: [Python ABI 33 layout selectors](python-ffi-compatibility-design.md#abi-33-layout-selector-checkpoint-2026-09-27)
adds an optional bounded Rust parser/compiler and browser-world geometry
scorer. The unchanged async/sync Chromium layout rows pass candidate and
oracle; 6,067 candidate rows, broader layout grammar, retry polling, positive
query-selector handle publication, Firefox/WebKit and Core JS/TS remain open.

NODE-252: [Python ABI 33 registered selector engines](python-ffi-compatibility-design.md#abi-33-registered-selector-engine-checkpoint-2026-09-27)
executes bounded custom-engine chains in Rust-owned main/utility worlds. The
six unchanged async/sync Chromium base, path, and world rows pass candidate
and oracle; 6,069 candidate rows, positive query-selector handle publication,
Firefox/WebKit and the Core JS/TS corpus remain open.

NODE-251: [Python ABI 32 selector validation](python-ffi-compatibility-design.md#abi-32-selector-validation-checkpoint-2026-09-27)
routes query-selector parsing errors through Rust before browser execution. The
unchanged async/sync Chromium registration-error rows pass candidate and
oracle; 6,075 candidate rows, custom engines, Firefox/WebKit and Core JS/TS
execution remain open.

NODE-250: [Python ABI 31 selector registration](python-ffi-compatibility-design.md#abi-31-selector-registration-checkpoint-2026-09-27)
adds a bounded Rust-owned global registry with thin async/sync Python facades.
The unchanged Chromium duplicate-registration row passes candidate and oracle;
6,077 candidate rows, custom-engine execution, Firefox/WebKit providers and the
Core JS/TS execution remain explicitly open.

NODE-249: [Python ABI 30 async selector regression](python-ffi-compatibility-design.md#abi-30-async-selector-regression-checkpoint-2026-09-27)
runs the unchanged async twin of the official `internal:and` test through the
installed candidate wheel and independent oracle. Both Chromium rows pass;
6,078 candidate rows and the incomplete Core JS/TS corpus remain open.

NODE-248: [Python ABI 30 collection selector evaluation](python-ffi-compatibility-design.md#abi-30-collection-selector-evaluation-checkpoint-2026-09-27)
adds thin async/sync `Page.eval_on_selector_all` facades over Rust-owned
collection evaluation and correct `internal:and` intersection semantics. The
unchanged Chromium row passes candidate and oracle; 6,079 candidate rows and
the Core JS/TS execution remain explicitly open.

NODE-247: [Python ABI 29 sync-listener completion](python-ffi-compatibility-design.md#abi-29-sync-listener-completion-checkpoint-2026-09-27)
passes the exact unchanged Chromium row from the complete sync listener file in
candidate and oracle. Reconnect keeps live subscription lanes until the
authoritative snapshot finishes, then retires only missing Pages; the replay
window remains explicitly bounded at 2,048 entries. The remaining 6,080
candidate rows and the Core JS/TS execution are still open.

NODE-246: [Python ABI 29 response listeners](python-ffi-compatibility-design.md#abi-29-response-listener-checkpoint-2026-09-27)
adds independently selectable, bounded Rust response publication with thin
Python callback adaptation. Its live candidate lane also proves one-shot
topology event-loss reconstruction without treating transient loss of a Page's
command route as Page removal. The applicable unchanged Chromium row passes in
candidate and oracle; 6,081 candidate rows, Firefox/WebKit providers and the
Core JS/TS execution remain open.

NODE-245: [Python ABI 28 device descriptors](python-ffi-compatibility-design.md#abi-28-device-descriptor-checkpoint-2026-09-27)
embeds the complete 143-entry v1.58.0 catalog in Rust and applies bounded
Context-scoped Chromium device settings before Page publication. The unchanged
official file is one pass plus the same two upstream browser skips in candidate
and oracle; 6,082 candidate rows and the Core JS/TS execution remain open.

NODE-244: [Python ABI 27 request history](python-ffi-compatibility-design.md#abi-27-request-history-checkpoint-2026-09-27)
adds bounded Rust-owned last-100 Request snapshots while retained evictions
remain functional through `Request.response()` and `Response.text()`. The
unchanged async/sync rows pass 2/2 candidate and 2/2 oracle; 6,085 candidate
rows and the incomplete Core JS/TS corpus remain explicitly open.

NODE-243: [Python ABI 26 console-message history](python-ffi-compatibility-design.md#abi-26-console-message-history-checkpoint-2026-09-27)
adds an independently selectable bounded Rust history leaf without forcing
console callback publication. The unchanged async/sync newest-200 tests pass
2/2 candidate and 2/2 oracle; 6,087 candidate rows and the incomplete Core
JS/TS corpus remain explicitly open.

NODE-233: [Python ABI 19 drag and drop](python-ffi-compatibility-design.md#abi-19-drag-and-drop-checkpoint-2026-09-27)
adds a bounded Rust locator-to-locator drag leaf and thin Page/Locator facades.
The fixed official selection passes 8/8 candidate and 8/8 oracle; seven
candidate failures and 6,102 not-run rows remain, so full-suite completion is
still open.

NODE-232: [Python ABI 18 Request sizes](python-ffi-compatibility-design.md#abi-18-request-sizes-checkpoint-2026-09-27)
adds bounded Rust settlement for `Request.sizes()`, including raw Chromium
request-header correlation and POST body accounting. Two fixed official rows
pass 2/2 in candidate and oracle; 11 candidate failures and 6,106 not-run rows
remain.

NODE-231: [Python ABI 17 Page request routing](python-ffi-compatibility-design.md#abi-17-page-request-routing-checkpoint-2026-09-27)
adds bounded Rust string-pattern interception and paused-call ownership. Five
fixed official async Chromium rows pass 5/5 in candidate and oracle; 13
candidate failures and 6,106 not-run rows remain. Wider routing and sync
routing are still explicit gaps.

NODE-230: [Python ABI 16 Page errors](python-ffi-compatibility-design.md#abi-16-page-error-checkpoint-2026-09-27)
adds a bounded Rust `Runtime.exceptionThrown` observer and ordered publication
leaf. Five fixed official rows pass 5/5 in candidate and oracle; 18 candidate
failures and 6,106 not-run rows remain.

NODE-229: [Python ABI 15 Page lifecycle events](python-ffi-compatibility-design.md#abi-15-page-lifecycle-event-checkpoint-2026-09-27)
selects the bounded Rust publication and close-retirement path. Six fixed
official lifecycle rows pass 6/6 in candidate and oracle; the full 6,526-case
corpus remains incomplete at 23 candidate failures and 6,106 not-run rows.

NODE-228: [Python ABI 14 Page bindings](python-ffi-compatibility-design.md#abi-14-page-binding-checkpoint-2026-09-27)
selects the bounded Rust binding owner through thin async/sync facades. Twelve
fixed official rows pass 12/12 in candidate and oracle; ABI 15 supersedes this
historical checkpoint.

NODE-227: [Python ABI 13 Handles and tags](python-ffi-compatibility-design.md#abi-13-handle-wait-for-function-and-tag-checkpoint-2026-09-27)
selects bounded Rust remote-object ownership and truthy polling. Fixed official
tag and JSHandle manifests pass 25/25 in candidate and oracle; the full corpus
remains incomplete.

NODE-226: [Python ABI 12 init scripts](python-ffi-compatibility-design.md#abi-12-page-and-browsercontext-init-script-checkpoint-2026-09-27)
extends one bounded Rust leaf to explicit Page and BrowserContext ownership.
The complete unchanged async and sync files pass 18/18 candidate and 18/18
oracle; the full 6,526-case corpus remains incomplete.

NODE-225: [Python Page-fill parity](python-official-conformance.md) moves three
unchanged official nodes from failure to pass and improves the complete Page
file to 67 passed / 57 failed / one upstream skip. The corrected nodes pass
3/3 in the independent official oracle; full-corpus execution remains open.

NODE-224: [Python ABI 11 client certificates](python-ffi-compatibility-design.md)
selects the same exact-origin Rust PEM/PFX leaf through thin async/sync facades.
The fixed official selection records eight passes and four upstream macOS
WebKit skips in both candidate and oracle lanes; the complete 6,526-case corpus
remains incomplete.

NODE-223: [Node APIRequest client certificates](api-request-design.md) adds an
optional Rust-owned, exact-origin PEM/PFX mutual-TLS leaf. The exact official
validation row passes 3/3 and seven direct-request behaviors pass 21/21 in both
candidate and oracle lanes. Proxy, browser-context and same-origin
multi-identity coverage remain open.

NODE-222: [Python ABI 10 APIRequest compatibility](python-ffi-compatibility-design.md)
selects the same browser-independent Rust request stack through thin async/sync
facades. The complete unchanged Python global-fetch files pass 231/231 candidate
and 138/138 plus 93/93 oracle; the full 6,526-record corpus remains incomplete.

NODE-221: [Node APIRequest compatibility](api-request-design.md) passes every
unchanged official global-fetch and global-fetch-cookie invocation in Library
and Bidi projects, 774/774 candidate and 774/774 oracle. The full
48,477-record Core corpus remains incomplete.

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
- [Playwright-compatible Python FFI](python-ffi-compatibility-design.md) — PyO3
  boundary, async/sync facade ownership, composable wheel profiles, and gated
  differential verification plan
- [Official Playwright Python conformance](python-official-conformance.md) —
  pinned upstream source, public/private test inventory, exact-test runner and
  oracle/candidate checkpoints
- [Full upstream Playwright conformance](upstream-playwright-conformance.md) —
  pinned Playwright core JS/TS plus Python inventories, expanded invocation
  ledgers, explicit exclusions, completion rule and current red candidate gates
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
  there, and leaves user evaluation in the main world; the optional Page/OOPIF
  session-initialization hook is a separate five-package plugin contract, so
  host settings implementations do not depend on the aggregate composition;
  synchronous Frame event, URL, and ownership projection contracts share a
  second independent five-package leaf used directly by host adapters; bounded
  Frame lifecycle and navigation-wait observation now form a third, exact
  ten-package leaf consumed directly by the Node host
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
