<!-- agent-updated: 2026-10-21T03:39:01Z -->

# Official Playwright Python conformance

Status: full source inventory and both 6,526-test collections are established;
full-corpus execution remains incomplete. The newest immutable
[v10 sync-network paired receipt](../.work/official-conformance/outcomes/python-sync-network6-chromium-strict-paired-v10/promotion-receipt.json)
(SHA-256 `c696c1cab0976ac45564c54814ebf12199268a904f243672203c1ababd9f7f13`)
promotes the complete six-case Chromium `tests/sync/test_network.py` HTTP
run: candidate 1/6, Oracle 3/6. It replaces only six rows per lane and
preserves 6,520 off-target rows per lane byte-for-byte; 44 verifier/mutation
controls pass. The earlier six candidate setup errors from the CDP Page-lane
cap are retained as infrastructure-invalid, not test failures. Current
cumulative totals are candidate **1,651 passed / 124 failed / three errors /
37 skipped / 4,711 not run** and Oracle **1,436 / 13 / 37 / 5,040**. Two
security-details failures are candidate-only; three HTTP fixture cases fail
in both lanes. The archived candidate/harness bytes still depend on the
external hash-verified Oracle package and pinned upstream checkout. A
separate source-pinned trusted-HTTPS Service Worker diagnostic runs the same
two official IDs: [candidate](../.work/official-conformance/results/python-sync-network-sw2-secure-paired-candidate-20261021-v1.xml)
1/2 versus [Oracle](../.work/official-conformance/results/python-sync-network-sw2-secure-paired-oracle-20261021-v1.xml)
2/2, with both provenance sidecars independently verified. Candidate now
passes `Response.from_service_worker`; BrowserContext `serviceworker` event
remains unimplemented. This does not revise the original HTTP v10 ledger.

A separate all-capability release wheel with the source-only, default-off
`navigation-response-finished` join was built without replacing the ABI131
wheel used by v10. Its SHA-256 is
`a71e7a58c3bdc37658343839b3bb0fe21222b444b271553c7c92d2dda27a1f22`.
The unchanged complete six-case Chromium file ran twice from that isolated
installation: [first report](../.work/official-conformance/results/python-sync-network6-nav-finished-candidate-20261021-v1.xml)
and [second report](../.work/official-conformance/results/python-sync-network6-nav-finished-candidate-20261021-v2.xml),
both **one pass / five failures / zero errors** with independently verified
source-bound provenance. The HTTPS security-details test now passes its
`response.finished()` assertion and fails next because
`Response.security_details()` is absent; the HTTP security-details case fails
at the same missing method. The other four outcomes have not improved. This
join currently selects `request-history`, whereas the open design target is
an independently bounded final-response join without that requirement; its
terminal, redirect, and close behavior is not yet official-test proved. These
diagnostics do not alter the v10 strict ledger or establish Python parity.
A separate [four-case Chromium security manifest](../packages/pwright-python/tests/official/response-security-chromium-sync-async-v1-manifest.json)
pins the two unchanged sync and two unchanged async official IDs, both
source files, fixture code, and HTTPS certificate assets. Its source/inventory
checker passes 3/3; no browser outcome is yet credited to this focused
manifest. It is diagnostic until both complete source files are rerun.
The mirrored `cases.capabilities` values are coarse upstream-topic labels,
not an exhaustive candidate feature closure. All four cases require normal
`Page.goto()` navigation returning a Response and the security-details reader;
the HTTPS pair additionally set `ignore_https_errors=True` and call
`Response.finished()` on that navigation Response. In the current candidate,
that means `ignore-https-errors` plus `navigation-response-finished` (which
selects `response-finished`) alongside `response-security-details`. This is
an explicit prerequisite inventory, not proof of a minimal dependency closure.

The new independent default-off `response-security-details` source slice
retains only five bounded CDP TLS fields, correlates the exact Rust
request/navigation identity, and exposes thin sync/async Python readers.
The rebuilt isolated all-capability release wheel has SHA-256
`0c09817458bc9329c3bbc7c00153c635ae913d3a58567746a2666ed24fa2d2cc`.
Its installed native module reports 113 selected features, including both
navigation-response-finished and response-security-details; all 300 installed
browserless Python contract tests pass. Focused release adapter 30/30, host
2/2, Python facade 5/5, and opt-in/default-off addon checks also pass. The
full addon dependency/release-feature matrix passes after correcting its
stale four-package server-initialize expectation and a page-events-only
compile error; the security-only closure and release compile pass separately.
The four official browser cases and complete-file reruns remain **unrun on
this wheel** because the dedicated Chrome CDP endpoint became unresponsive;
no v10 ledger credit or full parity is claimed.

The rebuilt wheel also has a fresh source-bound paired `core` collection:
[candidate](../.work/official-conformance/results/python-core6501-security-v2-candidate-collect-20261021-v2.xml)
and [Oracle](../.work/official-conformance/results/python-core6501-security-v2-oracle-collect-20261021-v2.xml)
each collect all 6,501 `tests/` IDs, and both provenance sidecars verify.
The collection-only JUnit files contain zero executed cases. The isolated
candidate collection environment includes upstream Playwright 1.58.0
distribution metadata solely for the pinned profile check; provenance proves
the loaded `playwright` module is the candidate shim and `pwright` is the
installed wheel. The Oracle loaded its separate official package. These
collections do not change any strict outcome row; the 25 TodoMVC examples
are a separate Python 3.10 profile and were not recollected on this wheel.

The installed ABI131 wheel adds
default-off `frameattached`/`framenavigated` events. The unchanged official
`window.stop()` subframe navigation case passes in a focused source-bound
run, but the first complete 80-case navigation run on that same wheel was
**76 passed / two failed / two synthetic errors**. The independently verified
[v7 RED receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v7/promotion-receipt.json)
(SHA-256
`027c37acdfb4326458e96489d306bfa5a5058a6573d5184195f60774611b8944`)
replaces exactly 80 candidate rows, preserves the other 6,446 candidate and
all 6,526 Oracle rows byte-for-byte, and explicitly records that focused
green is not full-file green. Candidate cumulative totals at v7 are **1,616
passed / 104 failed / five errors / 37 skipped / 4,764 not run**. A second
complete run on the same wheel produced [78 passed / two failed / zero errors](../.work/official-conformance/results/navigation-candidate-full80-abi131-recheck-oct21.xml)
with all 80 IDs source-bound and no skips; its two failures match the prior
candidate failures. The two extra `TargetClosedError` reports did not recur,
but that does not prove the race is fixed. The independently verified immutable
[v9 recheck receipt](../.work/official-conformance/outcomes/python-navigation-recheck-chromium-strict-paired-v9/promotion-receipt.json)
(SHA-256
`42738259dba662f23f324451ad094d0f31f2d9137ad01b1e3df14cd599bc4763`)
promotes exactly those 80 candidate navigation rows from the v8 predecessor;
all other 6,446 candidate rows and all 6,526 Oracle raw rows remain
byte-identical. The then-current candidate ledger was **1,650 passed / 119 failed /
three errors / 37 skipped / 4,717 not run**; the Oracle remains **1,433 /
ten / 37 / 5,046**. This is a newer measured outcome, not proof of a
reliability fix or complete conformance.

The 50-case pinned Chromium Python manifest from `test_network.py` ran all its
unchanged public-API IDs in each HTTP/Tailscale lane: [candidate](../.work/official-conformance/results/python-network50-candidate-abi130-v2-oct21.xml)
**35 pass / 15 fail**, [Oracle](../.work/official-conformance/results/python-network50-oracle-oct21.xml)
**47 pass / three fail**. All 15 candidate failures include the exact three
Oracle failures, leaving 12 candidate-only discrepancies. The independently
verified [v8 paired receipt](../.work/official-conformance/outcomes/python-network50-chromium-strict-paired-v8/promotion-receipt.json)
(SHA-256
`b4acfb11c8af4305e55c9f314c8a55193ed6b930bde1cd43b2c0239ae8f27189`)
replaces exactly 50 target rows per lane and retains all 6,476 other raw
rows per lane. Historical v8 candidate totals were **1,648 passed / 119 failed /
five errors / 37 skipped / 4,717 not run**; Oracle totals are **1,433 passed /
ten failed / 37 skipped / 5,046 not run**. The receipt marks complete
manifest execution, not complete Python file or corpus execution. A separate
trusted-HTTPS two-case Service Worker check passes [2/2 in the Oracle](../.work/official-conformance/results/python-network-sw2-https-oracle-anchored-oct21.xml)
and [1/2 in the candidate](../.work/official-conformance/results/python-network-sw2-https-candidate-anchored-oct21.xml):
the remaining candidate failure is missing `Response.body`. The HTTP
`server_addr` assertion expects loopback while the Tailscale fixture exposes
its actual Tailscale IP. Neither fixture variant is proof of full-corpus or
unadapted compatibility. The first two HTTPS Oracle attempts failed in CDP
setup after the last blank Chrome tab closed; they are quarantined as
infrastructure errors, not Service Worker test results.

Read-only classification of the 12 candidate-only network50 REDs finds five
raw Request/Response header fidelity or missing-reader cases, four navigation
Response lifecycle/security-metadata cases, one `Route.fulfill` navigation
failure whose exact handler cause is unproven, one redirect-body error-message
mismatch, and one Page extra-header indexed-validation message already fixed
in source but not rerun on an installed wheel. The shared HTTP
`server_addr` test expects loopback even under Tailscale, but the candidate
also lacks `server_addr()`; it is not only a fixture discrepancy. These
classifications do not change either frozen report or grant passing credit.

The earlier complete pinned Chromium `tests/async/test_navigation.py` run used
the frozen ABI130-v2 wheel (SHA-256
`40889db525656b5410a994c81e79530f873531c28ac0e8c9052e8603a87efb78`)
and executed all 80 exact IDs against dedicated CDP Chrome 155: **77 passed,
two failed, one synthetic report error**. The independent Oracle remains
**77 passed / three failed**. The source-bound, immutable, independently
verified [v6 full-file receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v6/promotion-receipt.json)
(SHA-256
`928da6d927abd7cf0667b63e88a89eb9fab260fe15ff0c50c04db358a295cb33`)
replaces exactly the 80 target candidate rows, preserving all 6,446
off-target candidate raw rows and all 6,526 Oracle raw rows. Cumulative
candidate totals are **1,617 passed / 104 failed / four errors / 37 skipped /
4,764 not run**; Oracle totals remain **1,386 passed / seven failed / 37
skipped / 5,096 not run**. The new synthetic error is the
`window.stop()` subframe case: its isolated pytest invocation emitted both a
test failure and a teardown error, so the runner rejected the duplicate-ID
report. The prior ABI128 synthetic error, a Frame redirect case, passed in
the new full run. The two remaining explicit failures, back-navigation timeout
and a schemeless Tailscale fixture URL, occur in both lanes; the subframe
`frameattached`/`framenavigated` event is the sole candidate-only unfinished
navigation ID. The upstream URL completion special-cases `localhost` and
`127.0.0.1`, while this fixture uses a Tailscale MagicDNS host, so the
schemeless failure is not evidence of a candidate-only regression. Fixture-
adapted Chromium execution is not proof of unadapted Playwright compatibility:
the receipt sets `publicApiParityEvidence=false` and
`fullCorpusExecution=false`.

ABI130 adds the default-off, single-registration `navigation-url-match`
capability. Three unchanged official URL-matched waiters pass **3/3** in a
focused source-bound run. The independently verified immutable
[v5 focused receipt](../.work/official-conformance/outcomes/python-navigation-url-match-chromium-strict-paired-v5/promotion-receipt.json)
(SHA-256
`3c2d07e5d788b5ede07145e69ec78a6848045b3145a5772e61db55cf667145e6`)
promotes only those three candidate rows; the other 6,523 candidate and all
Oracle raw rows are unchanged. The complete-file v6 run above supersedes v5
for the current navigation outcome, not for the historical evidence chain.
After the ABI130-v2 wheel was frozen, a separate source-only Python facade
fix made Page navigation `Response.frame` use its canonical main Frame even
without optional `request-history`. It passes 283/283 source-overlay contract
tests but is **not** present in that wheel or credited by v6.

The separately installed ABI129 wheel adds Rust-owned `Page.go_forward()` to
the optional history-navigation capability. The unchanged official
`test_go_back_should_work_with_history_api[chromium]` passes **1/1** in a
source-bound focused run. The independently verified immutable
[v4 history-API receipt](../.work/official-conformance/outcomes/python-navigation-history-api-chromium-strict-paired-v4/promotion-receipt.json)
(SHA-256
`541e82e5805207de17299295b9e9dfe3ca4c04c20771f36981b1db5e2ef0c4b6`)
promotes exactly that one candidate row, preserves the other 6,525 candidate
rows (including the synthetic error) and all 6,526 Oracle rows byte-for-byte.
Cumulative candidate totals at v4 were **1,613 passed / 108 failed / four
errors / 37 skipped / 4,764 not run**. That focused success was **not** itself
a fresh 80-case full-file rerun; `test_go_back_should_work[chromium]` still
fails at the pre-existing second-back navigation timeout in the later v6 full
run and in the Oracle. The earlier frozen ABI128 complete-file
[v3 receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v3/promotion-receipt.json)
(SHA-256
`2255d24bdeee2fc9b9a9d4c12c742d1e65c55793a8562c8b65cf978eb6346f1b`)
records 72 passed / seven failed / one synthetic report error. Its separate
focused rerun of the then-synthetic Frame redirect case passed without
rewriting the v3 result; the later v6 full run records that case as passed.

The preceding [v2 Frame successor](../.work/official-conformance/outcomes/python-navigation-frame-chromium-strict-paired-v2/promotion-receipt.json)
(SHA-256
`a35ba350fdd40a0e91251980fc98cc7d0f461f009268c616fe54cdf052660139`)
strictly promotes five unchanged official `Frame.goto`,
`Frame.expect_navigation`, and Frame load-state assertions from candidate
failure to pass, using a separately frozen ABI128 wheel. Its independent
verifier and 78 mutation/acceptance controls pass; the other 6,521 candidate
rows and all Oracle rows are preserved. The original complete-file
[v1 RED receipt](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v1/promotion-receipt.json)
(SHA-256
`6998ef7e55f2e24e879f515eb1e1f533bc4e427f67a68b911dc0a6701428f5e1`)
records ABI127 candidate **57 passed / 23 failed** versus Oracle **77 passed /
three failed**; it remains available rather than being rewritten.

A preceding immutable browser-free paired successor is
`.work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/`.
The unchanged official async/sync `BrowserType.connect` closed-server
assertions pass **6/6 in each isolated ABI123 candidate/oracle lane** with 12
independently verified sidecars. The versioned fixture replaces the original
server launch with a real Tailscale listener closed before the public call;
no browser or BrowserServer was launched. The paired receipt SHA-256 is
`4979cdc982cf716e5124bf00b972594c27a65fc26104c193ed12957cf5d4c060`.
Candidate is **1,542 passed / 102 failed / three errors / 11 skipped /
4,868 not run** (ledger SHA-256
`3095771a0928ecc076ecdec8eee5d2946d70de9dbd1ea15e00f61ad7fa79e836`);
oracle is **1,309 passed / four failed / 11 skipped / 5,202 not run**
(ledger SHA-256 `11afbc5f482a3d9cefa36c59598f1771903b74f93749dcd11e8b5b178a7a6208`).
Only six rows per lane changed. Because the canonical row schema does not
encode the fixture variant, read these `passed` rows with the receipt's
fixture-only scope and `publicApiParityEvidence=false`, not as full
BrowserServer or remote Browser support; see the
[strict evidence note](knowledge/pwright-python-connect-refused-strict.md).

The preceding immutable browser-free paired successor is
`.work/official-conformance/outcomes/python-connect-close-post-fix-strict-paired-v2/`.
Against unchanged official Python `BrowserType.connect` custom WebSocket-close
assertions, the independent oracle and opt-in ABI122 candidate each pass
**3/3** across Chromium, Firefox and WebKit. Six strict sidecars under
`.work/official-conformance/results/python-connect-close-post-fix-strict-v2/`
independently verify the real public call, inbound `Root.initialize` frame and
fixture Close `4123 / Oh my!`; no browser was launched. The wheel SHA-256 is
`2ad72324788ed7ae24eeb6f265e035f64a654e75ebc342ddb2537730f659bc1d`.
The paired receipt SHA-256 is
`03b22f0bc8e986217e9c6d9c301139049a0d268795ba15197ed5f4716bb52b72`;
read-only verification passes. Candidate now has **1,536 passed / 102 failed /
three errors / 11 skipped / 4,874 not run** (ledger SHA-256
`1acbae02e2049b96d2556a3093b8a65cd43b12ac00f53745ee6f58c764990afb`);
oracle remains **1,303 passed / four failed / 11 skipped / 5,208 not run**
(unchanged ledger SHA-256
`7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7`).
Only candidate lines 845–847 changed; the other 6,523 candidate rows and all
6,526 oracle rows are byte-identical to the predecessor. The receipt records
`fullPlaywrightServerSupport=false` and `publicApiParityEvidence=false`:
there is no connected Browser or remote GUID object graph.

The preceding paired successor is
`.work/official-conformance/outcomes/python-dummy-engine-post-fix-strict-paired-v2/`.
Against unchanged upstream async/sync Firefox/WebKit dummy-executable tests,
the independent oracle and the repaired ABI121 candidate each pass **4/4**.
All eight strict reports prove one real dummy Node exit 1 and the exact public
exception class in each lane; no Chrome was used for this shard. The repaired wheel
SHA-256 is `904db7f0af2082451edfbe589afcedd9fdc77295ce627cbc391d9e60b9780367`.
Fresh CPython 3.9.6 and 3.12.14 installs each pass 40/40 focused
browserless tests, `uv pip check`, and the ABI121 native import with all 110
selected features.
The paired receipt SHA-256 is
`009556238f14410c229cad349760bef962183fca33cf82569dc6a5c74d70a5bf`;
read-only verification passes. Candidate now has **1,533 passed / 105 failed /
three errors / 11 skipped / 4,874 not run** (ledger SHA-256
`5c88ae0e2cad7a0b5c10228e1435d220fb6263311b5d85cbd0f4c323e7eb32a3`);
oracle has **1,303 passed / four failed / 11 skipped / 5,208 not run**
(ledger SHA-256 `7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7`).
Only the four target rows per lane change from the preserved red v1 bundle;
6,522 other rows are byte-identical. This proves launch-failure behavior only,
not a working Firefox/WebKit protocol or full public API parity.

The earlier ABI119 full 106-capability wheel at
`.work/official-conformance/builds/python-abi119-cdp-full-iWlHU8/`
(SHA-256 `86d1ff028301b4bd42f9f2a3a39b8cef172217e8a9600e1fcf0517c7b38b1254`)
imports from fresh CPython 3.9.6 and 3.12.14 environments. Both pass 571/571
installed-wheel browserless units and `uv pip check`. Whole-project Ruff
check/format and strict Pyright pass.

Fresh Python 3.12.14 candidate and oracle environments both collect the
complete 6,526-case pinned corpus on this wheel; paired async/sync CDPSession
and tap manifests collect the same 21 IDs with matching manifest digests.
The full reports are
`.work/official-conformance/results/python-abi119-{candidate,oracle}-full-collect-v1.xml`;
each report and all eight focused collection reports have verified,
source-bound provenance sidecars. These are collect-only results, not browser
passes. Its packaged Python facade files match current Python source, but
its embedded README metadata is from before post-build
verification; rebuilding and repeating installed-wheel checks remain necessary
for a metadata-coherent release artifact.

The opt-in owned-process-startup Rust leaf and selected Python addon pass
targeted release Clippy with `-D warnings`; 40 focused installed-wheel Python
tests and the full Python addon dependency check pass. The default addon
excludes the new leaf, while selecting it adds exactly one Pwright package.
At an earlier CDPSession checkpoint, the full feature-by-feature dependency
script was interrupted under memory pressure; that historical result is not
the current dependency check.

The opt-in CDPSession source adds
claim/ack/rollback event delivery, a parked idle-event generation watch, and
native cleanup after Page retirement; focused release host and addon tests
pass 25/25 and 27/27. The unchanged 11 official Chromium CDPSession browser
cases have not been rerun on this wheel. The async facade now schedules
awaitable CDP handlers after event delivery, reports their errors through the
event loop, and bounds/cancels owned tasks; 36 focused fake-native tests also
pass in the installed-wheel suite. A synchronous callback still cannot
reenter a synchronous API call safely. The pinned 11-case CDPSession files
use only simple collection callbacks, so this additional behavior is not
counted as an official pass.
The current strict ledger already records all 11 Chromium CDPSession cases
and all ten Chromium tap cases as candidate failures with oracle passes;
new successful runs must refresh those same rows rather than claim new
coverage.
The earlier ABI 113 wheel
selects 102 capabilities and passes **508/508** installed-wheel browserless
units; it has no official browser result yet. The earlier ABI 114 wheel (SHA-256
`4850418950bac4dfe1fef27d34042d3b030345d2f0a32c030bcfb40f4c41b592`)
selects 103 capabilities including opt-in `ElementHandle.select_text` and
passes **518/518** full installed-wheel units in each fresh Python 3.9.6 and
3.12.14 environment. The earlier repaired ABI114 wheel (SHA-256
`d66b803fbd03480657ba24b60b5c6953fa45d9ffa3bb516768defea3bcd4e5a0`)
also exports exactly the same 103 capabilities and passes **518/518** units in
each fresh 3.9.6/3.12.14 install, with clean dependency checks and each
venv loading its own installed ABI-114 module. A trial wheel built with only
`--features element-handle-click` exported just three capabilities because
Maturin replaced the configured feature list; it was rejected before ledger
use. The repaired ABI115 wheel (SHA-256
`5856bc486b4bc342ceb82a251de79779a20b4b5b2094c0bbdbe028883fda8e54`)
exports 103 capabilities including `element-handle-click` and passes all
528/528 installed-wheel units plus dependency checks in each fresh Python
3.9.6 and 3.12.14 environment. The first ABI115 build exported only 102
capabilities because `pyproject.toml` omitted the click leaf; that build was
rejected and the configured list repaired. These are browserless packaging
checks, not new official-test outcomes. The preceding ABI 109 wheel's
complete async Chromium `test_element_handle.py` candidate run is
**13/59 passed, 44 failed, and two report errors** and has been promoted to a
new candidate strict successor. The independent full-file oracle attempt was
interrupted when the shared CDP Chrome stopped responding, so it has no
full-file oracle promotion. The preceding combined
wheel's complete Chromium click file is **69/70** on the original fixture,
matching the oracle's **69/70**; both pass the explicitly versioned favicon
fixture variant **1/1**. The latest complete popup file is **20/26**
candidate versus **26/26** oracle: first-request extra headers, two user-agent
inheritance cases, and context routing now pass. Both complete async and sync
ElementHandle state files pass **11/11** candidate and oracle. The
unchanged 36-case Chromium async
`Page.evaluate` shard passed **36/36** on its installed evaluation wheel,
matching the independent oracle's **36/36**. The earlier complete Chromium
async `Page.select_option` run recorded **24 candidate failures** because the
public method was absent. A fresh run on the preceding combined wheel still passes
**23/24 async** and **22/23 sync**; independent official oracle runs pass
**24/24** and **23/23**. Both candidate failures involve selection events that
immediately navigate away. Python 3.12 candidate venv has the pinned upstream
local test requirements installed, including pytest 8.4.1, pytest-asyncio
1.1.0, and test-only `pyee==13.0.0` required by upstream `conftest.py`. It still
originally loaded the frozen SHA-256 `1a536d38e585ce73d8ab9c313bbc7da41f60dfb5c33485ced56d16c32790b533`
wheel, and passed all 436/436 installed-wheel browserless units. A later
navigation-commit wheel also passes 436/436 browserless units, but its full
official 24/23-case rerun still has the same two navigation failures. The pinned
24-async/23-sync browser reruns completed after the other task's pages left
the dedicated CDP Chrome. The first async candidate attempt lacked `pyee`,
failed during upstream fixture import, and was quarantined as setup failure.

Generic `--result` promotion now requires current source-bound execution
provenance for the exact report bytes and lane; collection/bootstrap reports
cannot be promoted. This gate changes no existing ledger outcome.

At an earlier checkpoint, the immutable
candidate strict successor contained **1,525 passes / 102 failures / three report
errors / 11 skips / 4,885 not run** (SHA-256
`5c0f4078382dfcdbc0b9ed5b6e90ee7e3b0c4922755eff23b54f69f3b9023dfa`);
the independent oracle successor contained **1,292 passes / four failures /
11 skips / 5,219 not run** (SHA-256
`ed44f66be349a527e682aa2648fc7bf61a8a527af839411f45bbc606460f2a72`). These are separate
lane ledgers, not a paired full-corpus result. Earlier artifact names containing
`ABI110` were build-series labels, not proof of an exported ABI 110; the new
wheel below was checked through its native `native_abi_version()` call.
Final full-corpus **execution** still requires one frozen build to run across
the applicable corpus in independent lanes. The strict browser outcomes
now include the complete oracle click-file run, paired binding row and the
ABI108 bounded shard alongside earlier v6 Chromium selections; most recorded
results still use v4 or older builds. Four candidate failures also occur in
the independent oracle: two cookie-domain assertions and two IndexedDB
origin assertions hardcode
`localhost` while the required fixture uses a Tailscale MagicDNS origin. They
remain failures with zero pass credit; changing library domain or origin
semantics to satisfy this fixture mismatch would be incorrect. At an earlier
ABI108 checkpoint, two candidate failures passed in the independent oracle:
the missing `ElementHandle.click` API and the Context init-script callback.
Both have since passed on rebuilt wheels. The original click fixture still
produces a shared 404-console failure in both lanes. Only the explicitly
versioned favicon fixture variant below passes that exact
unchanged official assertion in both lanes. Historical original-fixture
reports remain unchanged and are not silently reclassified.

Those earlier immutable ledgers align by all 6,526 expanded IDs. Their
**paired** status is more informative than either lane's pass total:

| Candidate | Oracle | Rows | Interpretation |
| --- | --- | ---: | --- |
| passed | passed | 1,234 | Paired pass evidence, including one explicit fixture variant |
| passed | not-run | 291 | Oracle execution still required, including 13 ElementHandle cases |
| not-run | passed | 2 | Candidate execution still required |
| failed | passed | 55 | Candidate discrepancies, including 26 locator-handler, ten tap, 11 CDP-session, and one preview case |
| error | passed | 1 | Fail-closed two-record isolated popup report; underlying result unproven |
| failed | failed | 4 | Shared hardcoded-origin fixture failures; zero pass credit |
| failed | not-run | 43 | Candidate ElementHandle failures awaiting oracle run |
| error | not-run | 2 | Candidate ElementHandle report errors awaiting oracle run |
| skipped | skipped | 11 | Recorded skip in both lanes |
| not-run | not-run | 4,883 | Neither lane executed in these strict successors |

This cross-tabulation includes the paired two-case detached-handle run; it is not
a full-corpus run or an applicability decision. The independently audited
applicability split remains 6,514 / 12.

## ABI 113 installed-wheel ElementHandle bridge checkpoint (2026-10-11)

At the historical ABI113 checkpoint, source had advanced beyond the previous
ABI110 wheel. ABI111
adds an opt-in exact-Handle Python bridge for `get_attribute`, `inner_html`,
`input_value` and `is_checked`; ABI 112 adds `owner_frame` under the existing
`element-handle-dom` selection, resolving the returned Rust Frame ID against
the current BrowserContext rather than the Handle's original Page. ABI 113
adds `scroll_into_view_if_needed` as a separate opt-in Handle leaf. Its Rust
path checks stability on the exact Handle and sends Chromium's native
`DOM.scrollIntoViewIfNeeded` using the original CDP object ID, with no selector
requery. Source formatting, Ruff, narrow facade/type checks, focused host
release tests, and addon release checks pass. The repaired release wheel at
`.work/official-conformance/builds/python-retained-scroll-abi113-20261011.TuKNlN/`
has SHA-256 `f423668c31b274752dffc2a57c25f38cbf7e0fcd7852e53f56dc4cd4ba35424e`.
A fresh Python 3.12 environment with the pinned upstream requirements passes
`uv pip check` for 68 packages, exports native ABI 113 and 102 selected features
(including `element-handle-dom-reads`, `element-handle-scroll-into-view`, and
`element-handle-click`), and passes **508/508** installed-wheel browserless
units. The extension links its bundled OpenSSL dylibs via `@loader_path`.
These package tests include the DOM-read, owner-frame, and scroll fake-session
checks, but no ABI 111-113 official browser run or strict-ledger pass is
claimed. The `input_value` official row still reaches
missing `ElementHandle.fill` first; explicit read timeout enforcement remains
an open behavior gap. A separate Rust retained-click repair for Text nodes
passes 146/146 locator release tests but also awaits a rebuilt wheel and
official browser proof.

## ABI 110 preview and bounding-box browserless checkpoint (2026-10-11)

The repaired release wheel at
`.work/official-conformance/builds/python-preview-bbox-abi110-20261011.GrxAGK/`
has SHA-256 `149a795a245577c3e8d3f345d81eca8adc91dd0beed961bb55bd33b8dae83e4c`.
Its installed native extension SHA-256 is
`6434ba2e41a828aa92fbd2406089911aff99f987a5935708898fd4a643253ead`.
A fresh Python 3.12 environment passes `uv pip check` for 68 packages,
reports native ABI 110 and 100 selected features, and passes **477/477**
installed-wheel browserless units. The wheel bundles OpenSSL dylibs and its
native references point to the bundled paths, not Homebrew runtime paths.

The new source includes opt-in Rust-owned DOM previews for returned handles,
Python async/sync `ElementHandle.bounding_box`, and the retained-click script
change that avoids a mutable page-global `Node`. These are packaging and unit
results only. The dedicated Chrome CDP endpoint remained unresponsive after
the parallel XLQY browser task stopped using it; a read-only sample showed
Chrome's main thread blocked in macOS LocalAuthentication. No ABI 110 official
browser result, source-bound receipt, or strict-ledger pass is claimed.

## ABI 109 ElementHandle full-file checkpoint (2026-10-11)

The repaired release wheel at
`.work/official-conformance/builds/python-evaluate-handle-abi109-20261011.oRzriD/`
has SHA-256 `7ef45e47830a5122cb038f0cf06a4fcba8b061896db51083093212ee63b56794`.
It bundles the required OpenSSL dylibs, imports from a fresh Python 3.12
environment with 68 compatible packages, exports native ABI 109 and 100
selected features, and passes all **470/470** installed-wheel browserless
units. The wheel predates the later Rust-owned DOM preview return-path change.

The pinned complete Chromium `tests/async/test_element_handle.py` manifest
contains 59 upstream cases. On this installed wheel, 13 pass, 44 fail, and two
fail closed as report errors because pytest emitted both a test failure and a
teardown error for one isolated ID. The dominant missing public surfaces are
`scroll_into_view_if_needed` (seven cases), `bounding_box` (six),
`owner_frame` (six), and `select_text` (five). The report SHA-256 is
`3d649663e8415cae65cd437796fe73d02834753d87fc69187b840161f976605b`;
its schema-5 source-bound provenance SHA-256 is
`c6141302ce4a3b85dd475659e12840b2179e1f5e2ef2f40853ace429355e2811`.
Both were verified against the pinned upstream source, manifest, installed
wheel, and bundled libraries. The immutable candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi109-elementhandle-async59-20261011-v11.jsonl`
(SHA-256 `5c0f4078382dfcdbc0b9ed5b6e90ee7e3b0c4922755eff23b54f69f3b9023dfa`)
changes exactly the 59 pinned async Chromium rows; all other 6,467 rows are
byte-for-byte unchanged. A separate one-case `evaluate_handle` preview
run reached the official assertion but failed because `str(ElementHandle)`
still used a CSS-like selector string instead of Playwright's DOM preview.

The corresponding oracle full-file run reached `content_frame` before the
shared Chrome CDP endpoint stopped responding during another task's browser
activity. It was interrupted (exit 130) and remains a **quarantined partial
diagnostic**, not a full-file oracle outcome or strict-ledger promotion.
The two-case preview oracle passed and was separately promoted in oracle v8;
one preview candidate case now records a failure in the strict successor, and
the other remains not-run. Complete
independent oracle and sync-file runs must be repeated once the browser lane is
healthy and uncontested.

## Deferred route-callback error repair (2026-10-11)

A new isolated release wheel at
`.work/official-conformance/builds/python-route-deferred-full-features-20261011.lklWjB/`
(wheel SHA-256 `b723bda3c6f63ccb3e1ebe497f8d4695fe2e711fc4db7f94fbd637ebc976053f`,
installed native SHA-256
`abad89d0c03034fee69fec50bb2b29b294cec9314c1260564270d943e4f9874a`)
reports ABI 108 and 100 selected capabilities. The fresh Python 3.12
environment passes `uv pip check` for 68 packages and **466/466** frozen-wheel
browserless units. Four `evaluate_handle` tests added after this wheel was
built were excluded from that frozen-wheel check; they target a separate
source capability change.

This wheel reruns the complete pinned BrowserContext request-intercept files:
candidate **8/8 async and 6/6 sync**, matching the already verified official
oracle **8/8 and 6/6**. The prior two candidate failures now pass because a
route handler's post-`continue_()` exception is retained and raised once
before the next `Page.goto` native command. This is narrower than upstream's
next-*any-command* error boundary; that generalization remains open. The
candidate async JUnit/provenance SHA-256 are
`98adf26d11932a95dfe435a80fb2998f845ed672f7444f806a34371ca6e147c9` /
`350131553ac54dda323876a3673d98489eab29990f7b3ad92695f5be3905911f`;
sync are `a2cfcfb1a4ddbf599acd8b3643d65af20ea0a7c9d413ef4ac91835cc4066718d` /
`7eac51830d3690528e88247a46d1c2076a4a61ee41da541f245f774af7e9a8a1`.
Both schema-5 source-bound receipts verify. The immutable candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-route-deferred-intercept14-20261011-v10.jsonl`
(SHA-256 `606d5b659b49753f70631beb68ff03c4f89e0e73c02a520864d59ddd5bd7d156`)
updates exactly the 14 intended rows: two failed-to-passed transitions and
twelve passed-to-passed refreshed receipts. All other 6,512 candidate rows
and the complete independent oracle ledger remain unchanged.

A preliminary CLI-feature-override build in
`.work/official-conformance/builds/python-route-deferred-full-20261011.1VR6Lh/`
selected only three capabilities; preflight rejected it, and it was not used
for browser conformance. The subsequent build explicitly selected the 96
pyproject features plus `element-handle-click`. Local wheel loading succeeds;
the Maturin build warned that external Homebrew OpenSSL libraries were not
bundled, so distributable-wheel repair remains a packaging gate.

## Complete tap and CDP-session files (2026-10-11)

New complete async/sync Chromium `test_tap.py` manifests contain seven and
three public cases. The frozen 100-capability candidate wheel fails **7/7
async and 3/3 sync**; the independent official oracle passes **7/7 and 3/3**.
Most candidate failures are absent `tap` or `touchscreen` methods; four
cases first encounter absent `ElementHandle.evaluate_handle`. The
JUnit/provenance SHA-256 pairs (async candidate, async oracle, sync candidate,
sync oracle) are
`60fdc08795d2b2e584a5ccde38ac07738b76dcef4302f3f8f88e8246247f8d98` /
`caa0a4ba75a3d3f8931b4469e2c9ce79885658bb6ce029916bc33179566c2edd`,
`0864b297d3c2ada2487f0f5cf2990582ebfa7c11f285e2586b3d0a3f489a248b` /
`960ca8b0a6c9f90188d011a18e2654bc722079dc7a2b8c34b82b12bcc0ca6ae1`,
`3387f6dae09dddd3b6c5a42b83be502abd6cb11193d51140a786b5f262edfe93` /
`b387fbdac4ce965f4e540ff42ae50c86e679b7a20dea0f5404e08be93d4c259d`,
and `48a4dcae656710c2409b9160b9738c6d3de257d2ce26a3552f76264481325673` /
`3c720c73ecf806c986bb61549b0497b47f8f28fd095af5c083b570702480d824`.
All four source-bound receipts verify. The tap-only strict successors are
`.work/official-conformance/outcomes/playwright-python-candidate-fence-pred-intercept14-tap10-20261011-v8.jsonl`
(SHA-256 `9aa7a7de669671f19b0abed3110348b261aba2fb5bfb69d6d676e3546eefa000`)
and `.work/official-conformance/outcomes/playwright-python-oracle-intercept14-tap10-20261011-v6.jsonl`
(SHA-256 `9027b70da2015680469e2bbb694b1cdcbe57e6805162752bad5e525ec4772ca6`).
Exactly ten intended rows changed in each lane; all others remained equal to
their verified predecessors.

The complete Chromium `test_cdp_session.py` manifests add six async and five
sync public cases. Candidate fails **6/6 and 5/5** because
`BrowserContext.new_cdp_session` is absent; official oracle passes **6/6 and
5/5**. The JUnit/provenance SHA-256 pairs in the same lane order are
`baba693be5448e9453eb984ab244804f6cacc8ff9bdf85f4486c607a546b5b18` /
`6f86fca278c605b86a9bd9f94f372393787f2692d89ade2cc064b859472fd5c7`,
`f41e0fd84476b2cfdc95a342f8bd8a5b0f3ae60bc4eda2e06e4a6d345658f589` /
`5d1b5fa9fbd47c66ed5c3e4e4942d2df7afd94f73686a1fb74b95376174b2306`,
`424b03f36ebd146c64efa4bda2d9eeefab440b4df96fff58622514e6155aa6fd` /
`b1565bb5860a45c75626c078b39f792b8e69864621986d82492b85b6d2c4996a`,
and `74a2e4888dfe93f973762bc9a9f6a7c327405d0b23338ef044adbdf458c54b13` /
`094ff3e6e8d7c78aa9fe3a95cf7ec424a1850c2e20afbf5d09fb90db032d01ae`.
All four receipts verify. The combined strict successors are
`.work/official-conformance/outcomes/playwright-python-candidate-fence-pred-intercept14-tap10-cdp11-20261011-v9.jsonl`
(SHA-256 `5abfe926c0459c78374b019c073d5d180bd535519f3d8c7b601de47ddb932e82`)
and `.work/official-conformance/outcomes/playwright-python-oracle-intercept14-tap10-cdp11-20261011-v7.jsonl`
(SHA-256 `432161a0b87f366002303a3b0685220491d06fd0e12bb584a03d761066494ce4`).
Exactly 11 intended rows changed in each lane; all other 6,515 rows were
unchanged. These are mixed historical ledgers, not a full-corpus run on one
wheel.

## Complete BrowserContext request-intercept files (2026-10-11)

Two newly pinned complete Chromium files contain eight async and six sync
public cases, all previously unrun in the strict Python ledgers. On the
installed 100-capability wheel, the candidate passes **7/8 async** and
**5/6 sync**, while the independent official oracle passes **8/8** and
**6/6**. The same `test_should_show_exception_after_fulfill` case fails in
both candidate APIs: despite its name, the callback calls `route.continue_()`
and then raises; the official test expects the next `page.goto` to surface
that handler exception, but Pwright does not. This is a
paired candidate-only behavior discrepancy, not a fixture skip.

The candidate async JUnit/provenance SHA-256 are
`f2f4ceafd890d73a9480170de7f71dfd858d851c0b1a17460495474e38ee2935` /
`03f73d79616202bddd010858e6e8f9d1d62032bed068488fd3e1c75bcfea3cd6`;
oracle values are
`ed293341c949c30e263269dad84d99d67585484dff5f654902e6930c15b8ce83` /
`d3cb4242efb4f627ecba690c392031db2d121e1ca0b87c439f0e6861e6cf2c2d`.
The candidate sync pair is
`4fa9ca64b7e225c63542418f7584361be33a9b824e51a461c02ea6ecd5401ee2` /
`3e3d093c61b223a3a1da08a91581c07fc94e9a1e90d1f7de5c918cda5782130d`;
oracle values are
`d25f278b4bb3ee62f1902340c0f66bbc971ffb887b34a770965bfcc501f4f77e` /
`2a7cc13d566c4e4df445726691086de44013a6c3b8d5550d3d48810b374b23a3`.
All four schema-5 receipts and exact 8/6 case sets verify. The immutable
successors are
`.work/official-conformance/outcomes/playwright-python-candidate-fence-pred-intercept14-20261011-v7.jsonl`
(SHA-256 `ba7440aa69e82351d015b49d7413ad3d1894dd9f14e289538f1be1db2a572348`)
and
`.work/official-conformance/outcomes/playwright-python-oracle-intercept14-20261011-v5.jsonl`
(SHA-256 `06e351e71a403ef67e4e6163cd4bb7940e666721a4a6395a57a0b4a933cf31b2`).
Exactly the same 14 intended IDs changed in both 6,526-row ledgers; all
other rows remained equal to their respective predecessors. The new paired
totals are **1,232 pass/pass**, **35 candidate-fail/oracle-pass**, and
**4,964 not-run/not-run**. The same new wheel also reran the complete
async/sync `Page.select_option` files with **23/24** and **22/23** candidate
passes, retaining the two previously identified navigation-context failures;
those same-status diagnostic reports were not substituted into the strict
successor.

## Frozen popup-fence and state-predicate wheel (2026-10-11)

The repaired release wheel at
`.work/official-conformance/builds/python-fence-pred-full-20261011.E6krcB/`
has SHA-256
`312c67ae10ec8070d3b9a86e96f48ea7f562569962c62b4014ffa312971373af`;
its freshly installed native extension has SHA-256
`fc4a02e8883bd0514d5daece63cf486b3d651ce3356170b9b4acf69992679c2a`.
The build used 96 explicit pyproject features plus opt-in
`element-handle-click`, release mode, no defaults, and Maturin 1.15.0.
It reports ABI 108 and 100 selected capabilities; `uv pip check` verifies 68
installed packages, and **462/462** installed-wheel browserless units pass.

The unchanged complete Chromium async and sync
`ElementHandle.wait_for_element_state` files each pass **11/11** on this
candidate wheel, matching the prior independent oracle's **11/11** in each
lane. The sync file improved from **5/11** after adding opt-in public state
predicates. The unchanged complete popup file is **20 passes / five failures /
one fail-closed report error**, compared with **26/26** in the independent
official oracle. First-request extra headers remain passing; both user-agent
inheritance tests and the context-route test changed from failure to pass
relative to the preceding frozen wheel. No claim is made that the remaining
popup behavior or all browser context options match Playwright.
A second complete popup26 run on the same frozen wheel reproduced the exact
same 20-pass/five-fail/one-error status for every ID. Its independent
JUnit/provenance SHA-256 are
`3fb68ec6cafa7ab9ba7fc022715fcf9f44562053b6ead97d8772e9dedfa94b0f` /
`1af56d82ba64276e3af2e2764a5dba35412def68a7b680826ee434f5c472b760`;
the schema-5 receipt verifies. The repeat was not substituted into the strict
ledger.

The three new candidate JUnit/provenance SHA-256 pairs are: popup26
`4041f9ece269a658b7eedffefacd0c3ade0ed010a5acf1ef1a006cedafd975cb` /
`fbf1a5530126b248a5b000d0f6e19d2c3c281927741ea69ff88bb074c9add50d`;
async state11
`36facce5ddd2c22364860abf312f14dc3d618eb17a17806167aeaa27006cf9a6` /
`f13df322138708aa7701782750d075d054f3747635ff7cf2755dc051361c0074`;
sync state11
`f630f48b7e07453fb008a6095735f7871b79006f37b57fe35e757779183fcdb3` /
`86a0ba84ad3cef6eeed8cdacd19d5dc76ebf651299ed943cf06b3c618a0f4f04`.
All three schema-5 receipts and exact 26/11/11 Chromium ID sets verify.
The new strict candidate successor is
`.work/official-conformance/outcomes/playwright-python-candidate-fence-pred-popupstate-locator-handler26-20261011-v6.jsonl`
(SHA-256
`7664c8b7739f382ab51cc0950321326aeab120d75d1856184f8c699cb038ef2b`):
**1,498 passed / 37 failed / one error / 11 skipped / 4,979 not run**.
Exactly 48 intended rows changed, the other 6,478 were unchanged; nine
candidate failures became passes, and the oracle successor was untouched.
Paired pass/pass rises to **1,220**; candidate-fail/oracle-pass falls to
**33**. This is still a mixed historical strict ledger, not full-corpus
execution on one wheel.

## Frozen popup-and-state ABI 108 wheel (2026-10-11)

The repaired release wheel at
`.work/official-conformance/builds/python-popup-state-full-20261011.LY3JjF/`
has SHA-256
`ce645223baa5fccd2cfa06ede7a98f405e6af3137674ba0d90a35acecc08ef80`;
its freshly installed native extension has SHA-256
`762c64cc686a4384655412ace3659d145b055ddd93af2801d1c25e0334802765`.
The build used 95 explicit pyproject features plus opt-in
`element-handle-click`, release mode, no defaults, and pinned Maturin 1.15.0.
The extension reports ABI 108 and 99 selected capabilities; `uv pip check`
verifies 68 installed packages, and all **450/450** installed-wheel browserless
units pass.

The complete pinned async Chromium `ElementHandle.wait_for_element_state`
file passes **11/11** on this candidate wheel, matching the existing
independent oracle's **11/11**. Its complete sync counterpart passes **5/11**;
the remaining six failures call missing public `ElementHandle.is_disabled`,
`is_editable`, `is_enabled`, `is_hidden`, or `is_visible` methods. A separate
opt-in source implementation now exists, but has no installed-wheel browser
pass credit. The complete popup file is **17 passes / eight failures / one
fail-closed report error**, versus the prior independent oracle's **26/26**.
The first-request `extra_http_headers` case changed from failure to pass;
`test_should_respect_routes_from_browser_context` changed from pass to failure
(`intercepted` remains empty). This is not a net popup-file improvement, and
the route ordering requires a new build and browser verification. The popup
header case alone does not establish locale, user-agent, or routing parity.

The three new candidate JUnit/provenance SHA-256 pairs are: popup26
`e4c064e424f24c31194cb944154a6376e2526d37de0e66eafb22bb7df51f5ac5` /
`320ac0673a9e90e0a464c7383588f50155e9bb27f242b9501036b4b8245814b5`;
async state11
`481868a8dc1727545ea59c2a637e30b50610136835c49b99b20122c6a328a63f` /
`3c78bfef2c5cca097f27058041f82f60b70c2f9ca3b0ce73d94eda8f870e2b3d`;
sync state11
`6e87ba3364cf3253035d8ddd0a86ee769dc85b1e53ee236b9efd7d36bcb25335` /
`e841617e50cde51851750506350a1cd7321ffa883ea95e59b45c3a83c88e65ef`.
All three schema-5 receipts and their exact 26/11/11 Chromium case lists verify
against the pinned source. The strict candidate successor is
`.work/official-conformance/outcomes/playwright-python-candidate-popup-state-full-20261011-v3.jsonl`
(SHA-256
`bf95da4ce61737460473a85b2baf69637deaf7f2c6641d574ccb135e2773d2c2`):
**1,489 passed / 20 failed / one error / 11 skipped / 5,005 not run**.
Exactly 48 intended rows changed from the preceding candidate ledger, with
6,478 other rows untouched; the oracle successor and its SHA remained
unchanged. Paired passes are **1,211**, paired candidate-fail/oracle-pass
discrepancies **16**, and **5,004** IDs remain unrun in both lanes.

The complete 13-case async Chromium `Page.add_locator_handler` file was also
run on this wheel: **0/13** candidate, all failing on the absent public method,
versus **13/13** independent official oracle. Its schema-5 source-bound
candidate JUnit/provenance SHA-256 are
`6b7176a3fa83a3e1bb50cfada30c41ea511540a1dfcdcccc220a2b8ab7b8bb49` /
`a2be7eee54fe91133fa6983cdd76e9910f4f069d814a815f84ea7c43234a49e6`.
The oracle pair is
`386d1012a9eb7578c7e7699cc1e73a872f702a72d73699094aa9616bd83514c1` /
`f3cf7443a339aef7ff4868e3098fd220a1e731ebdbd1a28ab7391183ba424b29`.
Both receipts and their exact 13 case IDs verify. The newer immutable
candidate/oracle successors are
`.work/official-conformance/outcomes/playwright-python-candidate-popupstate-locator-handler13-20261011-v4.jsonl`
(SHA-256 `83ab33d8c8cb88b37a6630721c417edcdc903cb619990eb04e3d3f888b3d101a`)
and
`.work/official-conformance/outcomes/playwright-python-oracle-locator-handler13-20261011-v3.jsonl`
(SHA-256 `c2410554441a30ad8068b2a356bc7f51474c34bd904288519aff71aff5ded847`).
Each changes exactly the same 13 intended IDs and no other row. The sync
counterpart was subsequently rerun in both lanes: **0/13 candidate** (all
missing the same public method) versus **13/13 independent oracle**. Its
candidate JUnit/provenance SHA-256 are
`bb58a6894975d656058ac362e6010dde3f7445f15f21c986b1707381e9d19a10` /
`2251d72d915cd192856bb5f2b6661899b03d7dab12c63435e57b28ad704efb54`;
oracle values are
`6c64b871facb650b854a60ecc959fe0ab0bbb53aee6d4aefa0d5f595d1013072` /
`43c5dfbc7735a53d2e70739852b315ff234b48a8dd5cb2ca173db5bd82453c81`.
Both receipts and exact 13 IDs verify. The latest immutable successors are
`.work/official-conformance/outcomes/playwright-python-candidate-popupstate-locator-handler26-20261011-v5.jsonl`
(SHA-256 `ecf1a657bc69f6d387829a448c3582163b1dcaccdcff7a8d3446007c06032dee`)
and
`.work/official-conformance/outcomes/playwright-python-oracle-locator-handler26-20261011-v4.jsonl`
(SHA-256 `5cbe4f5967fd877587df1df654fcc00b7a13d63778ccd502329c1b4de95e16ca`).
The independent 6,526-ID audit shows exactly 13 more changed rows in each
lane, no other rows changed, **1,211 pass/pass**, **42 candidate-fail/oracle-pass**,
and **4,978 not-run/not-run**. The first sync attempt was refused at the shared
Chrome occupancy preflight and is not counted.

Implementation audit for the missing locator-handler API: the pinned async and
sync files each contain the same 13 behaviors. Upstream registers handlers per
Page and rechecks overlays before actions and during action retries; callbacks
may take zero or one Locator argument, have optional invocation limits, and
normally require the matching overlay to become hidden before the action
continues. Removal compares the original frame/selector identity. The current
Pwright `Locator.click`/hover retry and visibility-assertion loops are native,
below Python, so a one-time Python precheck would not handle overlays raised
mid-action. An opt-in implementation needs a native action checkpoint with
deadline/close cancellation and nonblocking Python callback dispatch, plus
public `force` click support. No locator-handler source implementation or pass
credit is claimed yet.

## Header-capable ABI 108 wheel and ElementHandle state file (2026-10-11)

The repaired release wheel at
`.work/official-conformance/builds/python-headers-full-20261011.65Y9v0/repaired/`
has SHA-256
`7e203031ba1ae05d2d6a1230cd2ffe2cea3dfd6c2d08175d1c04224a3173a5a1`;
the freshly installed native extension has SHA-256
`9b85884b7c00b3e50ec948773d13f1aa5886c56d25046b73a01304abb0e3309e`.
The build used the exact 94 pyproject features plus `element-handle-click`,
release mode, no defaults, pinned Maturin 1.15.0, and external OpenSSL library
repair. The extension reports ABI 108 and 98 selected capabilities;
`uv pip check` verifies all 68 installed packages and **445/445** browserless
unit tests pass. The un-repaired first packaging artifact was not used for
official tests.

The complete pinned async Chromium popup file still gives the candidate
**17 passes / eight failures / one quarantined two-record report error**.
The official first-request `extra_http_headers` case fails because the popup
request lacks `foo: bar`; Python argument forwarding and host option-composition
unit tests alone did not prove that network ordering. The independent oracle's
unchanged earlier complete file passed **26/26**. Current candidate
JUnit/provenance SHA-256 are
`a28850c2c858e3b9ce822f7135489a9061ebd019f970b6d74eb0cf6f7e2aa113` /
`1176059df696e783fb55358655b7b5cceded04850e3e02e4640ffbf7e69cb6e0`;
the schema-5 receipt verifies against the pinned manifest and package. No
header pass is credited.

The new complete 11-case pinned async Chromium
`test_element_handle_wait_for_element_state.py` file has manifest SHA-256
`25f9792ad3ac6521403b9bee224e3d2c51693a7d5f3b33563098642e441c1571`.
The independent official oracle passed **11/11**; the installed Rust-backed
candidate failed **11/11**, every case because `ElementHandle` lacks
`wait_for_element_state`. Candidate JUnit/provenance SHA-256 are
`3224b99754edc1ccac3db0b52d6d590d52b4372122fcca6662fc364fdae2185b` /
`9f3ac023d6ca283fd6672e2ee51be4c550f8e3ecaa5ee98358d0f242e413948b`;
oracle values are
`11db17b7c5ba7c05c013d87acea5459ccddcc3ca49389249c707391ef4f015f0` /
`6cfc18857cd78df22a9403fc86320375b657b8649bbcd44a10e5d8e1a0306d7d`.
Both schema-5 receipts verify against the unchanged upstream source and exact
case list.

The strict successors are
`.work/official-conformance/outcomes/playwright-python-candidate-headers-popup26-element-state11-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-python-oracle-element-state11-20261011-v1.jsonl`.
The full 6,526-ID audit shows exactly 37 candidate result-reference changes:
26 popup replacements with unchanged statuses, plus 11 `not-run` to `failed`;
the oracle has exactly 11 `not-run` to `passed`. All other rows remain unchanged.
That v1 checkpoint has **21** paired candidate-fail / oracle-pass discrepancies
and **5,015** IDs unrun in both lanes.

The complete pinned sync Chromium counterpart adds 11 more public cases with
manifest SHA-256
`31f1f178609fa9d37b2c43ffaf2e9881d020ddae865bcaa57c7dc6651e432693`.
The independent oracle passed **11/11**; the same installed candidate failed
**11/11**, all on the absent `ElementHandle.wait_for_element_state` method.
Candidate JUnit/provenance SHA-256 are
`01e057f0d9cb3dd2099f9a12bca019cc93bfa38731e9602dc351a425b8e85530` /
`e6b3e50633b5bce6f166116098c40fc750bcd89f947c7027be87a35cc346290a`;
oracle values are
`27d46b76ad543fcf28defe5946307a29333d718ae1be90c77e78b4b6acf9b876` /
`5445bd4a406295a09169a141fe452602c2ebbf2ba1c6bdec3bf4989b995d14ed`.
Both schema-5 receipts verify the exact unchanged source and case list. The
current strict successors are
`.work/official-conformance/outcomes/playwright-python-candidate-headers-popup26-element-state22-20261011-v2.jsonl`
and
`.work/official-conformance/outcomes/playwright-python-oracle-element-state22-20261011-v2.jsonl`.
The full 6,526-ID audit shows precisely 11 more candidate `not-run` to
`failed` and 11 oracle `not-run` to `passed`; all other outcomes and result
references are unchanged from v1. Paired passes remain **1,195**; there are
now **32** paired candidate-fail / oracle-pass discrepancies and **5,004** IDs
unrun in both lanes.

## Previous frozen ABI 108 Python wheel and full-file reruns (2026-10-11)

The release wheel at
`.work/official-conformance/builds/python-combined-full-20261011.OmVW2J/`
has SHA-256
`0b7ec154298e8803fe8307b0e954dcebbb2aa14ebac47ee4642ad2349ef84711`;
its freshly installed native extension has SHA-256
`1dcf6fd0530a43222d1db8bdbb9ff1fd4ab8957927d69530b7a02b49f5ba12a5`.
The build used pinned Maturin 1.15.0, release mode, no default features,
the exact 94 explicit pyproject features plus opt-in `element-handle-click`,
and a single Cargo job. The installed extension reports ABI 108 and the
expected 98-feature closure; `uv pip check` and all **438/438** browserless
units pass. An earlier wheel in
`.work/official-conformance/builds/python-combined-select-child-popup-20261011.ujp196/`
had only three selected features because its build command supplied only the
click feature. Its wheel SHA-256 is
`c190405aee3b7f1c25ca3431cc9a6483f8ae768c0c64021c82606d8fea5abcdd`;
it was quarantined before any official test or ledger promotion.

All current browser reports used the same frozen installed candidate and the
unchanged pinned upstream tests, with schema-5 source-bound receipts:

| Complete file | Candidate | Independent oracle | Candidate JUnit / provenance SHA-256 |
| --- | --- | --- | --- |
| async popup, 26 cases | 17 passed / eight failed / one report error | 26 passed | `90ce9513f202071a644516bdd2164b8ec36abcfd97733ed88de6d69b130e6af7` / `af88467eecd40e2da8060d628089d911538cf1426d47fedfe7c24d6f890af725` |
| async select-option, 24 cases | 23 passed / one failed | 24 passed | `e936c650b8eee40927411820c0922b2596b26c69614dddc9ccbb62b269287a33` / `5504f42b74e02d8bf35efbcd03a3f05522b6876dc1fe1c54b2dd151f7e169511` |
| sync select-option, 23 cases | 22 passed / one failed | 23 passed | `aa9b4e146645b37e2b7362e112c7596c95d73b9488cfc1fcbdcd00ebf6fab951` / `5f5f5228e64c767c116ea6ca5fb0fee4bbb0559b170e1c14d1836e3e1d7b0a5b` |
| async click, 70 cases, original fixture | 69 passed / one failed | 69 passed / one failed | `683f347d7002c111ab511eb9fae453fc7c6069289ede4752d28bb89f5e7cab12` / `cda1698b76eb5daa36c59cd3eb28be9693b40670fa27ecc1d446154685ac497c` |

The popup `Frame.page` case, child-Frame click, and DPR child-Frame click now
pass on the real browser. Both select-option navigation cases still fail with
CDP `-32000 Cannot find context with specified id`; the receipt source patch
has no official pass credit. The original click file's sole remaining failure
is the same favicon 404 console assertion seen in the oracle, not an
implementation pass. The versioned `offscreen-favicon-v1` fixture was run
again on the current candidate and a fresh official Python oracle, **1/1**
each, without editing the official assertion. Candidate JUnit/base provenance/
fixture receipt SHA-256 are
`39babf6ab0524956c2a0015cb1effcf6ad26595e39ceb4d0016052de69ea7bf2` /
`5636f408338823aa10c837eff4f59a673dfede27948597ebf5e68a372eac521f` /
`73f1eea3fc391cab40b90e07575e388cb29f38e467a245a8c4661b7dfe931779`;
oracle pairs are
`5074b96d18265f5f9cbfc71107866523d4f19c1a1f465be22fc7a08d59102cbd` /
`f449a0b68a8ecd02e95a884cd525dcc88f3b55979aa804b2a1e1d74c61fec2b3` /
`742b7dfec3e752f52f8c6af9aa2149b620792b68e5fe3e87ac9df18e6de1705d`.
Both base and fixture-specific receipts verify.

The exact original-fixture candidate successor is
`.work/official-conformance/outcomes/playwright-python-candidate-combined-full-popup26-select47-click70-20261011-v1.jsonl`
(SHA-256 `2815c4dbca4c4b0c974a5aacc3bd21db14adf617e2add755c75269f12231875a`):
1,472 passes / 15 failures / one error / 11 skips / 5,027 not run. Applying
the fresh, explicitly qualified favicon result last produces the current
candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-combined-full-popup26-select47-click70-favicon-v1-20261011-v2.jsonl`
(SHA-256 `25c395d898b9e2bf936fb25c84821c6db49399601a440b4c0d9e1c775a004617`).
The fresh oracle fixture receipt replaces only its previous variant proof in
`.work/official-conformance/outcomes/playwright-python-oracle-popup26-current-favicon-v1-20261011-v1.jsonl`
(SHA-256 `f27e438afe0489f5ee8c95c18a9574ab39d8abf1c81585313f887894218b6779`).
All 6,526 invocation IDs align. Exactly 143 candidate result references
change: 26 popup, 24 async select, 23 sync select, 69 original click, and
one versioned favicon. Exactly three statuses improve: iframe click
`failed` to `passed`, DPR click `error` to `passed`, and target-blank popup
`failed` to `passed`; the other 140 retain their prior status. The oracle
successor changes only the variant result reference, `passed` to `passed`.
Paired passes are now **1,195**. This composite is not a full-corpus run.

## Complete async popup Chromium pair (2026-10-11)

The pinned upstream `tests/async/test_popup.py` file ran all 26 Chromium
cases, without assertion edits, in both installed-package lanes. The oracle
passed **26/26**; the Rust-backed candidate passed **16/26**, failed nine, and
had one explicit report error. The report error is a candidate test failure
followed by a teardown error, yielding two JUnit records for one invocation;
the strict ledger rejects both records rather than choosing a result. Both
schema-5 receipts verify the unchanged source, exact case list, package,
runner, browser, and result bytes. Oracle JUnit/provenance SHA-256:
`06a1dedb891d8d244a7276e793fafc3f6b70c5f0150a6b4ba3fecb6a9c0fe61a` /
`5e5b6796c17d9a0c9ae1ea795e1ff1ccb7f6b7b99d19fa473e93ce20a84741e9`;
candidate:
`49c159df02368d0dd4e40b0cef9b98215fa43c0a874954e172cf251de0970b6c` /
`7242d6e26546c228f3c2660731d6e698c2472eccc3b008c024b2e606e9ccbbde`.

The strict successors are
`.work/official-conformance/outcomes/playwright-python-oracle-popup26-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-python-candidate-popup26-20261011-v1.jsonl`.
All 6,526 invocation IDs align in both complete successor audits. The oracle
promotes 26 `not-run` rows to `passed`; the candidate replaces 25 previously
unrun cases with 15 passes, nine failures, and one error, while re-proving its
one prior popup pass. Candidate discrepancies include inherited user agent,
headers, touch/viewport, in-process init scripts, function exposure, child
frame `page`, and a window-feature timeout. No failure is counted as a pass.
The missing `Frame.page` property is now added to both Python source facades,
with two direct identity tests passing, but this source change is not in the
frozen wheel above and has no official browser credit yet. A source-overlay
unit run passed 437 tests with one native profile-identity test deselected;
that identity test cannot validate a mixed source tree plus older compiled
addon. The exact installed-wheel profile check remains a separate required
packaging gate for the next immutable wheel.

## Installed-wheel Page.select_option async/sync pairs (2026-10-11)

The frozen ABI 108 wheel ran the unchanged 24 async and 23 sync Chromium
official tests as isolated cases with 30-second per-case deadlines. After
installing upstream's pinned test-only `pyee==13.0.0`, the source-bound
candidate and independent oracle results are:

| API | Candidate | Oracle | Candidate JUnit / provenance SHA-256 | Oracle JUnit / provenance SHA-256 |
| --- | --- | --- | --- | --- |
| async | 23 passed / one failed | 24 passed | `41fa83f3ac739b426351d1c7df91a42670104edf5d0d7b0554c7a38e84ad5068` / `55f102c5a8a707a7a02286945ecc56a8930a1764a62d30451c265ff38bb56f53` | `cf151fa2dbe1c1624aac11b667b21a3903ac903da23fd2e28dffe4508ad9a8ac` / `02d5789599adcd7527ca78b151639397c745b235609941acb10ec0a67152ff95` |
| sync | 22 passed / one failed | 23 passed | `ee0d81335716a76f83af87ccc29d0e6c1ffbcb6291841468401203d4d47f6db0` / `0d7e9433702d8ded3f9a94b8e1d7d0757ccf4c9ab903c95ef7498a85228e0d7c` | `9d9b9210b3b8c8669bbc1aebecc024fb0ef0448efd5f2c992dadd0a48e2bbb9e` / `21a6284fbc813302d46052e2be625848f3b1cc5f474811c25eab3bd31b5765a0` |

All four schema-5 provenance sidecars verify against the pinned source,
manifest, installed candidate or official package, exact case list, and
isolated reports. The two candidate-only failures are the same test behavior:
an `input` event navigates to `/empty.html` before the candidate's CDP
evaluation returns, which loses its execution context (`CDP -32000: Cannot
find context with specified id`). The oracle completes the call. A narrow
opt-in navigation-commit source repair now passes 14/14 focused release backend
tests, the release Python-addon feature-chain check, and strict lint on the
touched CDP, locator, composition, and host crates. Strict addon lint remains
blocked by an unrelated pre-existing `locator_read` argument-count warning.
No new wheel or official browser result exists: two isolated build attempts
were interrupted when another project compiled under memory pressure. The
dedicated CDP Chrome later recovered with a different PID, but another task's
nonblank pages now occupy it; no further browser test was run. These frozen
results and the strict ledger remain unchanged.

Later, one isolated single-job release build completed the opt-in
navigation-commit wheel at
`.work/official-conformance/builds/python-select-option-nav-commit-20261011.tbMCWO/`
(wheel SHA-256
`515662d405a1d1799c8fc2c1be8163c29e8690bae19b7458c59462bcdf644a38`,
native SHA-256
`f594b8965d3ebe8566905d5e8cb3003879958c39d7646c093d6494d605cfcc70`).
It loads as ABI 108 with 98 selected features in an isolated Python 3.12
venv and passes 436/436 browserless units. The official fixture environment
was installed separately from the pinned upstream requirements. Its complete
unchanged Chromium reruns remain **23/24 async** and **22/23 sync**: both
`test_select_option_should_not_throw_when_select_causes_navigation` calls
still fail with CDP `-32000 Cannot find context with specified id`. Both
schema-5 sidecars verify against the new installed native artifact and exact
case lists. Async JUnit/provenance SHA-256 are
`ca7299e1d9a4673e34890c2a168d5b3ac089a49bde0623752250d52ee10580f1` /
`9564eae5d7c9c5f3714d30dc938fd4028c45184e070a38585a41f94ca34e575c`;
sync are
`0b0c36247cb0d9879255c4eaf2dac54a38223e1b51d8d7cef55e6bf45da2cc2f` /
`3a140f8d2cb8fb08e720a2a00cd0b4f0592df502a3ee396a66f6f7861337b87c`.
The strict candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-select-option-nav-commit-async24-sync23-v1.jsonl`
has SHA-256 `8dc97b4340f1bbd24d496c6877cc987f2f9dcdf87a6754dc19fc25bc16497c7b`.
All 6,526 IDs align with the predecessor; exactly 47 report references changed
with **45 passed-to-passed and two failed-to-failed**. This build has no new
official pass credit. The recovery path is under diagnosis, not declared
fixed.

The earlier immutable strict 6,526-row ledgers are
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-select-option-async24-sync23-v1.jsonl`
(SHA-256 `ba230f183d88455d0ad603d08e8375713e45a8b408b1a9acffd7dea2f5fe6665`)
and
`.work/official-conformance/outcomes/playwright-python-oracle-abi108-select-option-async24-sync23-v1.jsonl`
(SHA-256 `eae68b1fd2b2df27b54e6135633241fe82090064cee803128527bf220c32a951`).
At that checkpoint, exactly those 47 async/sync rows changed their result provenance per lane;
complete invocation audits pass. The candidate ledger has 1,455 passes /
seven failures / one error / 11 skips / 5,052 not run. The oracle has 1,181
passes / four failures / 11 skips / 5,330 not run; paired passes are 1,176.
Neither corpus is completely executed.

## Browserless Page.select_option full-feature wheel (2026-10-11)

The opt-in Python `Page.select_option` async/sync facades and bounded native
descriptor bridge are installed in an isolated release wheel. A Rust
same-target guard rejects an `ElementHandle` OPTION from a different SELECT;
the focused release parser/guard tests passed **9/9**. The wheel was built
with the 94 explicit features in `pyproject.toml` plus the conformance-only
`element-handle-click` feature (95 explicit, 98 selected after dependency
closure). It targets `cp39-abi3`, exports ABI **108**, and includes the
Homebrew OpenSSL dylibs used during this local build.

The frozen wheel is
`.work/official-conformance/builds/python-select-option-20261011.GVCufI/pinned-full-wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `1a536d38e585ce73d8ab9c313bbc7da41f60dfb5c33485ced56d16c32790b533`;
packaged `_native.abi3.so` SHA-256
`256a102ccf8374b8879e597dea164bcd50a75c27aaf9fe2e339672a9a6ab2681`).
Isolated installed-wheel Python **3.9.6** and **3.12.14** both load that
native module, report ABI 108 and 98 selected features, and each pass the
complete package unit suite **436/436**, with zero deselections. The formerly
stale profile-contract test now strictly asserts ABI 108 and accepts only the
default feature profile or that profile plus `element-handle-click`. Rustfmt
and Ruff checks pass. The detailed build and native-load receipt is
`.work/official-conformance/builds/python-select-option-20261011.GVCufI/build-receipt.md`.

The build and unit checks in this section are browserless evidence only;
the later official async/sync browser results are recorded above. The
option-handle identity check still has an index-to-action DOM
race, so one same-select handle test must not be generalized to full
`ElementHandle` parity. Portability to another Mac remains unverified.

## Complete async Page.select_option Chromium file (2026-10-11)

The new manifest
`packages/pwright-python/tests/official/async-page-select-option-chromium-v1-manifest.json`
(SHA-256 `ab367fb75f37224180b1f31e9885cf56c71b3b4e12b46087f24ad6170d70ecc0`)
pins all 24 public Chromium cases in the unchanged upstream
`tests/async/test_page_select_option.py`. Candidate and oracle both collected
exactly 24 IDs, then each ran every ID in an isolated pytest process with a
30-second deadline and the same Tailscale-bound fixture policy. The earlier
candidate wheel failed **24/24** because its Python `Page` façade had no
`select_option` method; the independent official oracle passed **24/24**.
These are genuine missing-API discrepancies, not test setup failures or
assertion changes.

Candidate JUnit / verified source-bound provenance SHA-256 are
`05b4c90332856479f39b6ffa1944b080b42b9c9b78c7530585789d66b0787b68` /
`fd0d6a6f3cfe104240fa1b8ae5f1540b7884cc32591b8f50a3e55458c8dcefcc`;
oracle values are
`d205551431ec949379e950540f9dd437efb57ad9ab42a13c0d0ce974a88f2e8a` /
`e7a6112f222f73e874376e64a52c9dce0361c07b9709cd4365919303480bb59c`.
Reports are under `.work/official-conformance/builds/` with prefix
`python-async-select-option24-20261011-`. Applying the two complete reports
changes exactly those 24 prior `not-run` rows in each 6,526-row ledger;
the resulting candidate/oracle SHA-256 values and counts are at the top.
This slice creates a concrete implementation target but gives no candidate
pass credit.

The corresponding unchanged official sync file has **23** Chromium cases
(the async-only null-item case is absent). Its separate source-pinned manifest
is `packages/pwright-python/tests/official/sync-page-select-option-chromium-v1-manifest.json`
(SHA-256 `6cb985f0b8c4d74df0368f1b6cbe0e21b660f2b25882edf56d3191ecc1261026`).
Both candidate and oracle collected exactly 23 IDs. No sync test body has run;
all 23 remain `not-run` in the strict ledgers while the dedicated Chrome has
another task's nonblank page.
The immutable candidate/oracle collection JUnit reports are
`.work/official-conformance/builds/python-sync-select-option23-20261011-candidate-collection-v1.xml`
and `...-oracle-collection-v1.xml`; their verified source-bound JUnit SHA-256
values are respectively
`7c67d9df191f37c5590266426afdbe6d85814e2bb83ebaf477b2284fe1da6456` and
`c1a41382ae0e28262c41b87818ce747bffe01d06668e94984ca57c311ff379a5`.
These collect-only records provide no execution evidence.

## Installed-wheel async Page.evaluate 36/36 rerun (2026-10-11)

The exact original 36-ID manifest and independent oracle report below remain
unchanged. The first full-profile codec wheel passed **33/36**; it still lost
the official exception's `_name` and converted `urllib.parse.ParseResult` to a
plain tuple before the Rust URL codec could see it. The second pinned
`maturin==1.15.0` release wheel fixes those narrow boundaries and passes
**36/36** in isolated official Chromium cases without changed assertions or
test selection. The wheel SHA-256 is
`60a869353086726bfb9910f5daa891a50540f0c2d3b87c599c495013f39903a8`;
the installed native module SHA-256 is
`2458c562db2a167bd056139be66ff73971e37a8d8af0dcfcd17d34981c10b5f0`.
The build receipt and Python 3.9/3.12 ABI checks are at
`.work/official-conformance/builds/python-evaluate-codec-v2-20261011.6HeDZH/build-receipt.md`.

The candidate JUnit and source-bound provenance are
`.work/official-conformance/builds/python-async-evaluate36-20261011-candidate-codec-v5.xml`
(SHA-256 `aeebcc2b75f5bfd1fa92dbf25f72318e7cf9e3bd5186b710b9f269b65d778140`)
and its adjacent `.provenance.json` (SHA-256
`c676ea1087419eb03a74f0f7a94e1301308013d85a28a02d5adfcccda1577dd4`).
The provenance verifier matched the exact result bytes, pinned upstream source,
and original manifest. The strict successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-click70-timeoutfix2-evaluate36-codec-v3.jsonl`
changes exactly the three prior remaining evaluate failures to passes and no
other outcome. Its historical 6,526-row counts were 1,410 passed / five failed /
one error / 11 skipped / 5,099 not run. A misconfigured diagnostic
invocation expanded the same 36-case manifest to 108 three-browser cases;
it was interrupted, and none of its partial results entered the ledger.
The completed run explicitly selected `--browser chromium` and recorded all
36 intended cases.

## Earlier paired async Page.evaluate shard and browserless codec repair (2026-10-11)

The pinned, unchanged 36-case Chromium selection in
`packages/pwright-python/tests/official/async-page-evaluate-chromium-v1-manifest.json`
(SHA-256 `921519847572b45ed89160c88dca6f5f0bc14f6d5e6e325872fee81e5ff75a91`)
ran one case per process with a 30-second deadline. The ABI108 candidate
passed 26/36 and failed ten; the independent official oracle passed 36/36.
Both execution sidecars independently verify against pinned upstream source
and exact JUnit bytes. Candidate XML / provenance SHA-256:
`2f031edd780b6d9fedc189c0d42ed3ed07c07d0dc89c5c382fa1ce433692c440` /
`b6c30fc07cf10c9bbed1d63ed5ff4b1dc9ddc3e157db8ebabbb379a42bd61cab`;
oracle:
`2e997f4e6163e33af505e6e38ded5abd0c18613ffff36c057e9ec93589169426` /
`2b9592965838bcf786b3d7243fcb1a003bce1e09ac4eb96802cdb0e1a10ff8f3`.
The reports are under
`.work/official-conformance/builds/python-async-evaluate36-20261011-{candidate,oracle}-v1.xml`.

The immutable 6,526-row candidate successor is
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-click70-timeoutfix2-evaluate36-v1.jsonl`
(SHA-256 `89b9493d3c79982ad621071da51c3cf8d4b6b363ff5dd2d0132888852da718d2`);
the oracle successor is
`.work/official-conformance/outcomes/playwright-python-oracle-abi108-favicon-evaluate36-v1.jsonl`
(SHA-256 `90d6295861ceb2461ebe0ec82796b4520cc7cce9d212faf8a08af6bc620ebc78`).
The full invocation audits preserve 6,514 applicable / 12 non-applicable.

Eight of the ten candidate-only failures are Python evaluation codec gaps:
JavaScript Error and TypedArray results are unsupported; Python datetime,
Playwright Error/Exception, and urllib `ParseResult` inputs lose their types.
The other two are disposed-handle and reload/navigation error-message
differences. None is a payload limit or test setup/teardown failure. A
subsequent Rust/PyO3 codec slice now covers the eight value cases and has
20/20 release codec unit tests; a narrow Python facade change covers the two
error messages. The rebuilt 97-feature ABI108 wheel, installed unit result
(427 passed / one known default-profile assertion deselected), exact source
hashes, and wheel SHA-256 are recorded in
`.work/official-conformance/builds/python-evaluate-codec-20261011.teLwLU/build-receipt.md`.
This is **browserless repair evidence only**: the official 36-case candidate
shard must be rerun on that exact wheel before any of the ten failures can
be promoted to passes. The oracle report need not be rerun for that purpose.

## Paired retained-click detachment result (2026-10-11)

The unchanged pinned Chromium cases
`test_fail_when_element_detaches_after_animation` and
`test_report_nice_error_when_element_is_detached_and_force_clicked` each pass
in the candidate and independent oracle lanes (2/2 in each). The candidate
uses the source-frozen post-repair ABI108 wheel below; both lanes use the same
two-case manifest (SHA-256
`dde103ff58ab8402af2926bfa5310f77161c0c4f13c97451915b3b0c8bbf59fd`),
isolated 30-second case deadlines, installed Chrome via loopback CDP, and
Tailscale-bound upstream fixture servers. Both schema-5 execution sidecars
reverify against the pinned source and exact JUnit bytes. Candidate XML /
provenance SHA-256:
`ade55472f60716b1f310a73c3f477519fc6fd737291d07ebb5d7a4119955c606` /
`8a68db51092c3b896553410bcc48fa5331f09d8eefd38cf3ea9a0eee880652b3`;
oracle:
`b1792a7ffb3b974f38e1cf96a41b04b99835b3bfc2459c89d83f92bf5e0e45c1` /
`487a30dace0a7e9629355995dfc6899adaa50957bc025375ab4838b6ca658c6d`.
Reports are under
`.work/official-conformance/builds/python-retained-click-detachedfix-20261010.Zw4S2S/`.
The previous oracle strict ledger already counted both rows as passed. The
new candidate-only immutable 6,526-row successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-reloadfix-v2-click5-init-script-favicon-detached-click2-v1.jsonl`
(SHA-256 `5015b54bd58c16f0f345af5ae6ef034f6999194031471b1c2e7fbe60941677d6`)
changes exactly those two candidate rows from `not-run` to `passed`; all
other 6,524 rows are byte-identical. Its summary SHA-256 is
`478f83df9db0103b670347d271af6af7f077e5d85da7bf8e0b338871e3e6d266`.
The complete 70-case rerun and its separately verified favicon variant are
recorded below; the newer strict successor includes their exact results.

## Complete post-repair Chromium click file (2026-10-11)

The installed post-repair ABI108 wheel above executed all 70 unchanged pinned
Chromium `test_click.py` cases, one case per process with a 30-second deadline:
**65 passed / four failed / one fail-closed report error**. The XML and
schema-5 provenance SHA-256 are
`857f4fe177a4731c60537011274c7a8de4b626a6755922625223f4fcb55504e6` /
`bf778a49873503ab91089fa31e18351cf783a5a26297c797fbec57dd5f24c461`.
The source-bound checker verified the pinned manifest, upstream source and
result bytes. The earlier independent oracle full-file report for this exact
manifest has **69 passed / one failed** on the original fixture. The shared
failure is `test_click_offscreen_buttons`, whose console list includes a
favicon 404. Candidate-only failures are child-Frame `ElementHandle.click`
geometry and the stable-position and hit-target timeout-message assertions.
`test_click_the_button_with_device_scale_factor_set` remains a fail-closed
error in the complete report because isolated pytest emitted two testcase
records for one ID. A later source-bound, single-case diagnostic with the
hardened runner preserved the original JUnit bytes: its test-body record fails
at the same explicit child-Frame click boundary, and its teardown record
reports retained native resources plus a sealed lifecycle registry. The
diagnostic does not retroactively reclassify the complete report's error.
The official test first sets DPR=5, then clicks a button inside a child Frame;
the selected backend explicitly rejects `globalThis !== top` before pointer
dispatch. It is not evidence of wrong DPR arithmetic. The rejected click skips
the test's later explicit `context.close()`. Fixture `Browser.close()` then
fails on a sealed lifecycle registry, so `NativeSession.close()` does not
remove that host; environment drain observes three leases and two scopes.
The initial registry-sealing trigger and ownership of each retained lease are
not established by JUnit, and the two records remain fail-closed. Repair must
follow the existing child-Frame content-quad/CSS-coordinate design and add
an unclosed-Context failure-cleanup regression; it must not multiply pointer
coordinates by DPR or relax the report invariant.

The opt-in `retained-element-click` source now has a bounded one-link,
same-session child-Frame content-quad path. Four focused release fake-CDP tests
pass for CSS-coordinate translation and fail-closed geometry, overlay, and
retired-route cases; backend and host release checks pass. Nested Frames,
OOPIFs, and cross-origin parent-hit access fail closed. No assembled HostSession
child-handle/world integration proof, wheel, or official browser rerun exists,
so neither this failed test nor the fail-closed DPR diagnostic gains pass credit.

Diagnostic XML / provenance SHA-256:
`fa14f18540389c10079ff0c0aa7374f01d1208bfeded05c23fc1c19dcef990a7` /
`520bc721ff40c0fd6d9b3d09a193378abb78f5443033ae7093840bd0314bf478`;
raw isolated JUnit SHA-256:
`adafec28903a6e5ce64ffe3050d2fdef698da4659f6596898b36dd223c05f214`.
The runner's new fail-closed path stores this `.xml.invalid.raw` alongside the
normalized case report, and its schema-5 sidecar binds the raw bytes.

Applying the complete report to the preceding 6,526-row candidate ledger adds
49 previously not-run passes, records the three newly exposed candidate-only
failures and one report error, and restores the original-fixture favicon
failure. The explicit, already source-verified `offscreen_favicon_v1` one-case
result is then applied last to preserve its **fixture-variant-qualified** pass;
it is not a claim that the original fixture passes. This immutable successor is
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-reloadfix-v2-click5-init-script-favicon-detached-click2-click70-favicon-v1.jsonl`
(SHA-256 `9d6851f599101d90ab531ed9442e1381253e53bd3c65f55ea8fb7712464b44b4`);
its summary SHA-256 is
`4c144d7818005330a2730e805718d1228433104402ca69c89bf08132f902b959`.
The oracle strict ledger is unchanged. These composites mix explicitly
recorded build/fixture revisions and are **not** a single frozen-build
full-corpus pass.
The fresh full invocation audit for this exact candidate successor passes:
6,514 applicable / 12 non-applicable, with all seven failures and the one
report error in public-API candidate invocations. Its summary is
`.work/official-conformance/outcomes/playwright-python-candidate-click70-postrepair-invocation-audit.summary.json`
(SHA-256 `74018bffeb7d1cb5e2d9cd72649ef19ec96c0e85e6989982dc999a5f6f6be6fe`);
the classification ledger SHA-256 remains
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The full70 report predates that diagnostic-runner hardening. Its exact
pre-hardening tooling inputs are preserved under
`.work/official-conformance/builds/python-click70-pre-hardening-tooling.Mw1bep/`;
the original candidate and oracle sidecars reverify with that explicit
`--tooling-root`, without changing either historical result.

## Retained-click timeout wheel and browser checkpoint (2026-10-11)

The opt-in backend/host now source-implements bounded per-invocation
actionability logs and a sanitized blocker preview for retained-click
deadline errors. Feature-on release backend tests pass 136/136, default
backend 123/123, host 643 passed / 14 ignored, strict release Clippy and
Python-addon release check pass. This source is newer than the installed
post-repair wheel used for the full70 report. A second, complete 97-feature
release wheel (SHA-256
`3d6f18613e34234d22a6cc2e1b2f10228ecd3f195a72d1e41a51c8a6d54c8de8`)
loads ABI 108 and the opt-in native click method in an isolated Python 3.12
venv. Its normalized payload reproduces; 4/4 facade tests and 422 package
units pass with one documented default-profile ABI assertion excluded. An
earlier accidental 3-feature diagnostic build is quarantined and has no
official test credit.

An exact two-case timeout manifest under
`.work/official-conformance/builds/python-retained-click-timeout-two-manifest.json`
(SHA-256 `c78824d60d24ab945d3d4df7ddd8406710e29f6dece6533252dc933008224a3e`)
passes pinned source validation. A fresh oracle-only attempt selected both
unchanged cases, but both failed during `BrowserType.connect_over_cdp` setup:
the dedicated Chrome profile exposed a `chrome-extension://` service-worker
target that official Playwright 1.58.0's CDP adapter rejected, closing its
driver connection. The source-bound XML / provenance SHA-256 are
`e20a6ed7f696fb731b9a58a772fe2fddcaa2f2aa4eced93cadbbbe53d2bc2e59` /
`0aa53bef56af0e5f77b6a388d113c0b9957e53d2c2c1562ee305474ab571abc8`.
This new setup-error report is diagnostic only and does not replace the
earlier independent oracle full70 passes for these two IDs. The extension or
shared browser has not been changed or restarted by this task.

The complete-wheel candidate then ran the exact two unchanged Chromium
timeout tests and passed **2/2**. Its XML / source-bound provenance SHA-256
are `8129b8012671ea788d0a8a742f50122ec3764838b945c474c406a176af191e78` /
`9152cf587c1287263a36b8b711f4cfd02f215b35b6f1cb471acd6fc7b8131783`.
The immutable strict successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-click70-timeoutfix2-v1.jsonl`
(SHA-256 `5dff8e00888dad238e40fb4167a1a8a3976111ad197a6450b89d8e49b652b9a1`)
changes exactly those two rows from failed to passed: **1,374 passed / five
failed / one error / 11 skipped / 5,135 not run**. Its full invocation audit
still reports 6,514 applicable / 12 non-applicable, summary SHA-256
`979334ea88ea687e6c49851585e6efc518cafa3f1c4a6f800288a1577a4e0d10`.
The earlier full70 oracle result already passed both IDs, so paired pass
evidence rises to **1,095**; this successor still mixes frozen builds rather
than proving a single-build full corpus. A second fresh oracle two-case run
again failed during CDP setup on the same extension service worker, without
entering either test body. Its XML / verified sidecar SHA-256 are
`79b4de8bfa8c99fc78491c1b16e5362b1208d29342baf2eba1a5f1ba9f4083e3` /
`f6f6ba76bab8f31350dadb70f9f5a4c2cdfa71f373a8e54cca6f65834d101050`;
this diagnostic does not replace earlier oracle pass evidence.

## Retained-click installed wheel, without official credit (2026-10-10)

An isolated **pre-detachment-repair** full-feature wheel plus opt-in
`element-handle-click` is frozen at
`.work/official-conformance/builds/python-retained-click-source-20261010.ra00oP/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `79f4e2eaec461dd4dd8f492d0ad9fc6c472825fbb989b6938806cb122e6f67c4`).
Its normalized payload reproduces on a second build. Isolated Python 3.9.6
and 3.12.14 installs each load ABI 108, report 97 selected features and the
native retained-click method, pass 4/4 facade checks, and pass 422/422
package units with the one documented default-profile ABI-107 assertion
explicitly deselected. An unfiltered run on each interpreter records that
assertion as its sole failure. The release PyO3 option parser passes 3/3
after correcting the local Python link path. No browser or upstream official
test used this wheel, so the 6,526-row strict ledgers above are unchanged.

Subsequent **source-only** repair separates a detached DOM Element from an
actually closed Page/Frame and preserves the official
`Element is not attached to the DOM` error message on the opt-in click path.
Backend feature-on release tests pass 132/132 and focused host tests 4/4;
the frozen wheel above predates this source repair. A separate **post-repair**
wheel at
`.work/official-conformance/builds/python-retained-click-detachedfix-20261010.Zw4S2S/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `3a820d641bf4ffd41b1fb9386f6203a639207a38b3ee9ac34a83f062c0e86258`)
has source-frozen normalized-payload reproducibility and installed Python
3.9.6/3.12.14 gates: both report ABI 108, 97 features including the opt-in
click method, 4/4 facade tests and 422 package units with one documented
default-profile-only assertion deselected. Both installed native libraries
hash to `e597806f4c505b524ca1d7f439b175cbc113dcb28f4c8eb44dada70b0cf20482`.
The paired two-case official browser run above now uses this exact wheel;
only those two rows gain strict candidate pass credit.
Stable-position and hit-target timeout-detail behavior was unresolved in this
older wheel; the newer complete wheel and 2/2 official result above supersede
that point.
The two-case manifest at
`.work/official-conformance/builds/python-retained-click-detachedfix-20261010.Zw4S2S/detached-click-two-manifest.json`
(SHA-256 `dde103ff58ab8402af2926bfa5310f77161c0c4f13c97451915b3b0c8bbf59fd`)
retains the nine-case manifest's pinned source and fixture hashes and selects
only the two detached-handle Chromium IDs. The pinned upstream source checker
passes with exactly two selected cases. An isolated Python 3.12.14 official
venv installs the exact post-repair wheel and pinned upstream dependencies
(68 packages, `uv pip check` clean); installed ABI/features/native method and
native `.so` hash match the wheel gate above. Candidate and oracle
`--collect-only` runs each select exactly the two `[chromium]` IDs, and both
source-bound provenance sidecars verify. Candidate collection XML/provenance
SHA-256 values are
`ff9ffde5d150927069050f1cfccf2deb038bf6cfb457006567b6c018f94b64ab` /
`8676d1d7e4a27d6cbc8d26865b2b5cd45ee7fd9261bc997421d5ec7ce0651f3c`;
oracle values are
`4d33852e6574638f9ac7ef329f25c7f91c001d67a8de2d6283f521a17bb25bfe` /
`73c3859a93bcf2bceae6887b57b2524aa850c055b8fa191d6bfc2d1d3f5b7019`.
Collect-only JUnit has zero executed tests and earns **no pass credit**. At
that earlier checkpoint, two unrelated nonblank tabs occupied the dedicated
CDP Chrome, so the browser runner's isolation preflight rejected execution.
They were not closed or reused by this task and were later cleared externally
before the paired browser runs above.

## Paired offscreen favicon fixture variant (2026-10-10)

The unchanged pinned official
`tests/async/test_click.py::test_click_offscreen_buttons[chromium]` assertion
passed **1/1** in both candidate and independent oracle lanes on the same
frozen ABI108 v2 wheel and dedicated Chrome. The explicit
[`offscreen_favicon_v1` manifest](../packages/pwright-python/tests/official/fixture_variants/offscreen_favicon_v1/manifest.json)
(SHA-256 `2c199fffa98bf2960fdb07c2e6216213343598ec9b9692d27132505b6405bc0c`)
pins this one ID from the 6,526-case upstream inventory. A case-scoped plugin
serves a valid ICO at `/favicon.ico` and confirms the route was used; it does
not modify the official test body. The
[capture/verify wrapper](../scripts/playwright_python_official_favicon_v1.py)
(SHA-256 `68b2eaa6df0f4340dca53ef962c67f410ad7905f5c78833f195ba20902b384a1`)
binds the plugin, icon, manifest, source-checked base schema-5 sidecar and
result bytes into a separate verifiable companion receipt. Both base sidecars
and companion receipts verify. This is **fixture-variant-qualified** credit,
not a claim that the original 404 fixture passes; its historical reports and
hashes are preserved.

The paired XML files and companion receipts are in
`.work/official-conformance/builds/python-abi108-reloadfix-v2/favicon-v1-paired.8vvJ4l/`.
Candidate XML / receipt SHA-256:
`70ffb8c0199d58a240bb8a4cbe02f578a7e5ac310ff440296b2fe441e29a9275` /
`7b3e91818d89586f2e3e2dd1894c1b8ac7b483ec6f5842a588fae5b7d56b1826`.
Oracle XML / receipt SHA-256:
`0d20ae387be2d63ca8b9e145ff8e6333d62cc2259d2f01d0133f28c0917de04b` /
`272097ef1cea75b449ad042a33be06e825f1fdd66d58c49a6cfe09710f6fe49c`.
The manifest-aware runner selected exactly one case in each lane and both
independent full invocation audits pass.

New immutable 6,526-row successors are
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-reloadfix-v2-click5-init-script-favicon-v1-chromium-v1.jsonl`
(SHA-256 `a483af2b7379c3b1a7945497041be1dd3a53efcb40046d3966529a00b4140f2f`;
**1,320 passed / five failed / 11 skipped / 5,190 not run**) and the same-named
oracle lane (SHA-256
`de156ebcf865eb589ad458eaaaf43c9fa0fae87aed9fe3a7543f7329d1e1363b`;
**1,098 / four / 11 / 5,413**). Each differs from its preceding strict ledger
by exactly this one ID: candidate `not-run` to `passed`; oracle `failed` to
`passed`, with the fixture-variant result file recorded. All other 6,525
rows remain byte-for-byte unchanged. The audits retain **6,514 applicable /
12 non-applicable** and classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.

## Frozen ABI108 reload/click refresh (2026-10-10)

The rebuilt full-feature wheel at
`.work/official-conformance/builds/python-abi108-reloadfix-v2/` has archive
SHA-256 `862c7074e3bbd6a9782b11af0e1ff66f9ffd84c13e0a40b1490b2858d48fa027`.
Its installed native library has SHA-256
`3a12b8631d9f0fc192f9cc42cb69cfb37671c5b7a085b84dcb910f3100013e2f`
on both Python 3.9 and 3.12. Both interpreters report ABI 108 and 97 selected
features; each passes 418 local units with one default-profile-only assertion
deselected. The release backend click library passes 123/123, and the focused
reload/binding-selection release tests pass 4/4. These package/source gates are
not substitutes for official browser results.

The unchanged pinned official Context init-script case now passes **1/1** in
both candidate and independent oracle lanes. The exact five previously timed-
out Chromium click IDs also pass **5/5** in each lane. All four schema-5
provenance sidecars verify source-bound against the original test bodies,
manifest IDs, installed packages and isolated reports. The wheel contains the
same-main-frame reload binding-publication fence and scoped click-actionability
repairs; neither assertion nor fixture was weakened.

| Selection | Candidate XML / provenance SHA-256 | Oracle XML / provenance SHA-256 |
| --- | --- | --- |
| Context init script, one case | `534fb5bdc1011f67bfc26f05b73a45e522e8079873e87c36666c97781d35755a` / `f4e23096d2b32cee568b472ede5f83061011f2810b0446ff12ec9b7d501d4379` | `011adbb47f53760304f64f7d7f8d4998567e32e77a982bce697504415582f9d5` / `d3db25b3043e879e93574c26a2a5cad2d1b2e450a2b1a5bafef55a91a5790533` |
| Five prior click timeouts | `feba02d0da8fd9ab2a5ecf4ac1585c471ca8ecd71ced87bca4d164ca04b61614` / `84a7c52a1b667fd0d75a4f2ea5758efe220b000361d7fcbd19c231c8bbfa513b` | `24ac9006510b3862f59b4a756ed78a4f9caaf582c976b4e962813b142cb14a98` / `0094692faae5d7bf98236effea9e65c7c0d264f4458ad87f1b757d731f6b324a` |

Fresh immutable 6,526-row successors are
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-reloadfix-v2-click5-init-script-chromium-v1.jsonl`
(SHA-256 `cb72136362e3a7a7d5d30bc9a26edb292619a0a56c8a7069c71d5de7dfcf6bcf`;
**1,319 passed / five failed / 11 skipped / 5,191 not run**) and the
same-named oracle lane (SHA-256
`4db7294848f6e920aba3a930da0f8eaa7809a55cfd37912120e22a534ee3b823`;
**1,097 / five / 11 / 5,413**). Each updates exactly six selected rows from
its previous strict ledger and preserves the other 6,520 rows byte-for-byte.
Fresh full invocation audits retain **6,514 applicable / 12 non-applicable**,
policy SHA-256 `f8e33a8161af2eedd3274f4a17e37f15ba20af14ff37e229ac5f923138ff3231`
and classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.

The same wheel also ran all 70 corrected Chromium click cases in each lane.
Candidate: **60 passed / nine failed / one fail-closed report error** (XML /
provenance SHA-256 `9812f60867bab681389106a0d62ab3dbd61de1c4d1ad31fac7bd158f9e22fba9` /
`e7a15d285f59830e36a4c1c37bb8b19cff6bb1f18e5c356511899734625e6852`).
Oracle: **69 passed / one failed** (XML / provenance SHA-256
`0fd066ac7e806bdf54156f1f10e449d088f9fce174633964a17dc28387408343` /
`b3c77b398cc2748ee7ee511c824ad073fbdb151324e3939243e2db09edab489b`).
Both sidecars verify source-bound, but the candidate's device-scale-factor
`ElementHandle.click` isolated run yielded two pytest testcase records for
one expected ID; the runner correctly replaced it with a fail-closed error.
The nine failures are one shared 404-console assertion and eight missing
`ElementHandle.click` rows. **Neither full 70-case report is merged into these
six-case strict successors.** The malformed raw isolated JUnit was overwritten
by the runner, so its exact dual-phase cause remains unproven. No report error
is silently treated as a pass or ordinary failure.

A separate **non-official** observer run in both lanes established the shared
404's URL: Chrome requested `/favicon.ico` from the test fixture while on
`/offscreenbuttons.html`; the fixture returned 404, and the console error's
location was `/favicon.ico:0:0`. All eleven button-click messages followed.
The repaired observer plugin SHA-256 is
`024474a3b171d67612dcbf17b1ea05f5c112444843e0a16869781dd1fa36dc07`;
its [diagnostic instructions](../.work/official-conformance/builds/python-abi108-full.hSJKx2/diagnostics/README.md)
explicitly carry no official credit. A precise favicon route could eliminate
this shared fixture failure, but the existing schema-5 reports hash the entire
current runtime plugin tree. Do not mutate that tree in place and strand the
historical sidecars; first establish a versioned, source-bound fixture variant,
then rerun the unchanged test in both lanes without the observer.

A `offscreen-favicon-v1` variant was prepared **outside** the historically
hashed `official/runtime` directory. Its exact-case pytest plugin
serves a valid 70-byte ICO at `/favicon.ico` with HTTP 200 and emits a JUnit
marker only after the route was used; a separate companion receipt binds the
plugin, icon, manifest, fixed runner, base schema-5 sidecar and result hashes. The
[capture/verify script](../scripts/playwright_python_official_favicon_v1.py)
has SHA-256 `68b2eaa6df0f4340dca53ef962c67f410ad7905f5c78833f195ba20902b384a1`;
the [plugin](../packages/pwright-python/tests/official/fixture_variants/offscreen_favicon_v1/pwright_official_favicon_v1.py)
has SHA-256 `eeda065fc24ba6a98e3b4e388b1c0af811f58103187e352ea6c02b246f71fff6`.
Eight offline tests, Ruff and the pinned upstream source checker pass; the
historical candidate/oracle sidecars still verify. An earlier preflight found
four unrelated XLQY pages and declined to run. Once those pages were externally
cleared, the paired official variant ran and received only the explicitly
qualified strict-ledger credit recorded above.

An opt-in `element-handle-click` source path now joins the top-level retained
backend to the Rust host, PyO3 and async/sync Python facades. It preserves the
exact `PendingHandle` and rejects child/OOPIF Frames rather than using wrong
coordinates. A current-source review added an observed creation-world guard
held through cancellation, exact-object re-evaluation after pointer movement,
a checked mouse-down under the Page input lock, and a Page-bound owned-press
cleanup token. The current full feature-on backend release suite passes 132/132;
current retained tests pass 9/9, host tests 2/2, input-token tests 7/7 and
queued-lock world tests 2/2. These local checks do not prove modifier-key
rollback on cancellation
or a live click. The frozen ABI108 v2 wheel used by every official report above
does not contain this feature. The separate post-repair wheel described near
the top has installed-unit evidence only: no real-browser handle-click
verification or official nine-case rerun has been completed. The strict
6,526-row ledgers and
the failed iframe row remain unchanged. The remaining host-level retirement,
runtime and Frame geometry checks are tracked in
[the retained-click design](retained-handle-click-design.md).
Local review also corrected the source-only retained `trial=True` path so it
cannot dispatch pointer input. This changes no frozen-wheel or official outcome.

## Paired ABI108 click-eight and init-script refresh (2026-10-10)

Four source-bound schema-5 reports in
`.work/official-conformance/builds/python-abi108-full.hSJKx2/` verify against
the unchanged upstream Playwright 1.58.0 source, exact manifests, installed
packages, report bytes and isolated case reports. The installed candidate
wheel's archive SHA-256 is
`bcfa4da75e539534701d41c5a9bcca4347e993bf98aab211cd8ad9c32f6d796e`.
The same installed full-feature wheel imports on Python 3.9 and 3.12; each
interpreter passes 418 local unit cases with one default-profile-only ABI and
feature assertion deselected. This does not prove the default-profile build or
wheel reproducibility.
Fresh local JUnit reports in the same build directory are
`python312-unit-abi108.xml` (SHA-256
`068d43c64059eb0c47a4f1b1c27ede94c5555b75289f6fbaf2b1a6ed579f3b73`)
and `python39-unit-abi108.xml` (SHA-256
`8efe054ca6496f37cd404828fd087803116abad4e5277d9b2b3c2945560d58d5`);
they are package-unit evidence, not official upstream browser credit.
The eight-case click manifest SHA-256 is
`bc6c6c05c77e087d1792d9571f0afa2ab953a6069a0e3df205b83526969f4a8d`;
the one-case init-script manifest SHA-256 is
`fdaecf074e6f043415fe74a93c4ae6f541b76102840aa280fc65c67600932f15`.

| Official report | Result | XML SHA-256 | Provenance SHA-256 | Isolated-case tree SHA-256 |
| --- | --- | --- | --- | --- |
| Candidate click eight | 7 passed / 1 failed | `4358e84050248d16b451ef4f0faa6b34292e4d2edf5c8e79e12f38ae94457a69` | `601d9259ef2008b31c6a7582f245e353efbb7264372ff6d9bdad983837f05b66` | `61af32abc1b4a39474404f298caea87a815aef910e74f8a35e1daaaee5a40303` |
| Oracle click eight | 8 passed | `a9baf6120c6fd91cba733aeb20b1efd129c98fd7bd7497ebc626ad0e9195c292` | `ac34ff62be1e5827206b85af5e00555ce5f22327e29080717282bb93f487ff1d` | `617f3e8116f0efb59aa64bcd8e5f583a0bd5eb34a1517011e344934e9ccd729d` |
| Candidate init script | 1 failed | `752fec19e2e48da1afbb79773e5ad1df2fe9f85dabc5508022763f26d5075422` | `4daf324b403b03ab967cf68cbdf14fea8631892e5db53f8362916e1d54dafb69` | `3144866370d1102dde8c60cc4d92f4c57b593206329b60dbbdad5720a7f42aea` |
| Oracle init script | 1 passed | `4817afb45d504f30a789bd1bbd1d579ee83d74a88b44ff981054097cece1e568` | `de14119b110d761ae9e44de65b0560743eb938913c70a0239338a2c70d0ad13d` | `5492c7fc1b937a327533814e3109635824bc1103da423d1efa39accbec9e69f8` |

The candidate's `Page.click(position=...)`, `Locator.click(steps=...)` and
`Page.dblclick` rows change from failed to passed. The iframe click row still
fails because `ElementHandle.click` is absent. The Context init-script row
still fails, now with `['context']` versus expected `['context', 'page']`
instead of the earlier `[]` versus `['context']`; the independent oracle passes
that exact case. Within the complete 70-case Chromium click file, candidate
strict accounting is now **eight passed / one failed / 61 not run**, while the
oracle remains **69 passed / one failed**.

An uncontested, non-official installed-ABI108 probe at
`.work/official-conformance/builds/python-abi108-full.hSJKx2/probe_init_reload.py`
isolated the init-script ordering: initial callbacks `['context']`; immediately
after `page.reload()`, `['context']`; both Context and Page script markers true;
after one `page.evaluate()`, `['context', 'page']`. The Page script does execute,
but its binding callback publication can lag behind reload completion. This
diagnostic does not change the failed official row. A same-main-frame reload
publication fence, limited by the original deadline and without the global
binding watermark, is pending implementation and official rerun.

The immutable 6,526-row candidate successor is
`.work/official-conformance/outcomes/playwright-python-candidate-abi108-click8-init-script-chromium-v1.jsonl`
(SHA-256 `afbdfb0460759c8639d9e93c37e6ed6c42c11df95dd9bc85fbe76945d2520653`):
**1,313 passed / six failed / 11 skipped / 5,196 not run**. Its oracle counterpart is
`.work/official-conformance/outcomes/playwright-python-oracle-abi108-click8-init-script-chromium-v1.jsonl`
(SHA-256 `e0c902c39d7b039740feb28f6d694d6b1cb06be90394193a6ae29d996df4c8ca`):
**1,097 passed / five failed / 11 skipped / 5,413 not run**. Each differs from
its previous strict ledger in exactly the nine report-selected rows; all other
6,517 rows are unchanged. The candidate gains exactly three passes, while all
nine oracle rows retain their passing status with refreshed evidence.

Full invocation audits of both successors retain **6,514 applicable / 12
non-applicable**, applicability-policy SHA-256
`f8e33a8161af2eedd3274f4a17e37f15ba20af14ff37e229ac5f923138ff3231`,
and unchanged classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
Their summaries are
`.work/official-conformance/playwright-python-abi108-click8-init-script-candidate-invocation-classifications.summary.json`
and
`.work/official-conformance/playwright-python-abi108-click8-init-script-oracle-invocation-classifications.summary.json`.

## Complete ABI108 click-file candidate diagnostic, not promoted (2026-10-10)

The same installed ABI108 wheel executed all 70 exact Chromium cases in the
unchanged `async-click-chromium-v2-manifest.json` with isolated 30-second
deadlines. The aggregate records **53 passed / 14 failed / three fail-closed
report errors**. Its XML SHA-256 is
`e8cf924b957cb2a699c25934e7d281c67ad842e7c29bab145490a1f76b4606b6`;
the schema-5 source-bound provenance SHA-256 is
`01ed9d9760db317e3afee6889a41bef9b3c7f11aaef845da3f96531cb57ccee1`.
Reports are under `.work/official-conformance/builds/python-abi108-full.hSJKx2/`.

Eight failures are the absent `ElementHandle.click` API; one is the same
offscreen-button 404 console assertion seen in the independent oracle. Five
cases hit the 30-second deadline: two position-offset cases and one force
case rejected immediately as unsupported by v6, plus two pointer-events
cases whose v6 isolated reports were fail-closed errors. The earlier diagnostic
does not prove any of these five passed before ABI108. They require a focused
investigation, not proof that the new click options work across
the file. Three isolated pytest reports contain two testcase records where
one exact case was required, and remain fail-closed errors. Four unrelated
XLQY pages were present in the shared dedicated Chrome at the end of this
run, so the full-file candidate report has no uncontested repeat. It is
**diagnostic only**; no row from it is promoted into either strict ledger.
The task-owned blank tab was closed without touching the unrelated pages.
The backend now has local source-only repairs for the five timeout symptoms:
requested-point scrolling, clickable-ancestor hit retargeting and terminal
forced-hidden errors. Formatting and a synthetic DOM exercise pass, but this
code is absent from the installed ABI108 wheel above. Its backend release
library suite passes 123/123 and strict release Clippy passes; a new release
wheel and exact official cases must still verify it before any
strict-ledger promotion. `ElementHandle.click` remains unimplemented pending
an exact-retained-handle backend pointer primitive with iframe coordinate
translation; selector reacquisition or a DOM `click()` would not preserve the
official action semantics.

## Complete click-file oracle and paired binding promotion: prior v6 checkpoint (2026-10-10)

The independent official Playwright 1.58.0 oracle completed all 70 exact
Chromium cases in `async-click-chromium-v2-manifest.json` (SHA-256
`e3f67855d66abb186fed974dc9b7e614e498acfc58741a557b5146aea373b02c`):
**69 passed / one failed**. The failed ID is
`tests/async/test_click.py::test_click_offscreen_buttons[chromium]`; its
asserted console list contains an extra 404 resource message. XML /
provenance SHA-256 is
`9c2213a7c76b25cfd6d9e638ecbafda1721b6709a70620a8021b02086c1f71c2` /
`9bb40ad8f8650ab82050168fce7d82e1ae97ae070e9d78ba729e9d32c1b05a66`;
the 70 isolated case reports have tree SHA-256
`a6d9eda5d7276ebed7899d03b233c32d27105d69152d4b448ecc6182580fdcf9`.
The pinned page fixture has no external asset references and the upstream
server returns 404 for absent files. A browser request for `/favicon.ico` is a
plausible cause, but neither JUnit report records the failed URL; this is not
yet established. A next diagnostic must capture request path/status and
console location for this one unchanged case in both lanes. Only if the icon
request is confirmed should the shared fixture runtime serve a real icon with
`200 image/x-icon`, followed by new source-bound paired reports. Do not alter
the upstream assertion or count this row as passed in the meantime.

The paired `test_expose_binding_should_work[chromium]` reports pass 1/1 in
each lane under the exact `browsercontext-expose-binding-chromium-manifest.json`
(SHA-256 `5c1aadd954664c4d248d114e5e3b7238065e5841285b27e0cc2dd5d811f4f886`).
Candidate XML / provenance SHA-256 is
`980b66695e92660e0c0f0171f05f1ba0c00264c418604c62e765eec8ca5e6076` /
`b802f4d6854e92129eea442751a7e578bbaadffd2e9e54c233cb0eded837b37e`;
oracle is
`91ad80cc153c7966d74b49e1853d3f818fb13227c508e55d95bba2b4721b839d` /
`c0916108548ef5c815bb755de79a339df07bf3975dfa4948bf18a0398ebccfaa`.
Their isolated case-report tree SHA-256 values are respectively
`0f64bfc61f249018a7802b7f79fe28da1fca5ee4328a8335b1ec93e44d345d75`
and `0533b809ec6f80ea3d27ac1c0810e52356b9532b48de72dcac4a226b5bedecb3`.
All three schema-5 sidecars verify source-bound, including pinned manifests,
upstream source, inventory, installed packages and report bytes; aggregate and
isolated outcomes match the exact manifest IDs. Reports are under
`.work/official-conformance/builds/python-oracle-click70-v6.Aza6FP/`.

The immutable 6,526-row candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-v6-binding-chromium-v1.jsonl`
(SHA-256 `832bc4c6268df9b156fd74d35536430b231a38af75fb8b3161057c410c216c0c`)
changes only the binding row from failed to passed: **1,310 passed / nine failed /
11 skipped / 5,196 not run**. Its other 6,525 records equal the prior
v6 click-eight ledger. The oracle successor
`.work/official-conformance/outcomes/playwright-python-oracle-v6-click70-binding-chromium-v1.jsonl`
(SHA-256 `99306b9b60c57a17e275e30e22605ba6e74d987fa913e0eabcb3cfc6c92c6cee`)
refreshes the eight earlier click passes and the previously passing binding
row, adds 61 passes and the one 404 failure, and leaves the other 6,455
records unchanged: **1,097 passed / five failed / 11 skipped / 5,413 not run**.
Every selected row matches its exact JUnit testcase and report path.
Full invocation audits of both successors retain 6,514 applicable / 12
non-applicable, applicability-policy SHA-256
`f8e33a8161af2eedd3274f4a17e37f15ba20af14ff37e229ac5f923138ff3231`,
and unchanged classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
Audit summaries are
`.work/official-conformance/playwright-python-v6-binding-candidate-invocation-classifications.summary.json`
and
`.work/official-conformance/playwright-python-v6-click70-binding-oracle-invocation-classifications.summary.json`.

The complete v6 candidate click-file report remains diagnostic and earns no
additional candidate strict-ledger credit. Three isolated reports are
fail-closed errors, and later cases ran amid unrelated Chrome pages and memory
pressure. The new oracle run establishes independent outcomes for all 70 IDs
but does not resolve those candidate execution conditions. At this prior v6
checkpoint, candidate strict click-file accounting was five passed, four
failed and 61 not run; oracle accounting was 69 passed and one failed. No
exclusions or applicability decisions changed.

## Paired ABI107 v6 eight-case official Python click shard (2026-10-10)

The exact eight-case manifest
`packages/pwright-python/tests/official/async-click-eight-chromium-manifest.json`
(SHA-256 `bc6c6c05c77e087d1792d9571f0afa2ab953a6069a0e3df205b83526969f4a8d`)
is a bounded subset of the corrected 70-case click manifest; the same clean
v1.58.0 source and fixture hashes are pinned. The source checker passed.
An attempted ad-hoc `--test` selection against the 70-case manifest was
rejected by the runner's `--manifest-cases` policy before tests or JUnit, so
the eight cases were rerun through this exact manifest with unchanged upstream
assertions. Both lanes used the same installed v6 environment, explicit Chrome
channel, loopback CDP, Tailscale-bound fixture and isolated 30-second cases.
Candidate passed 4/8; independent official Playwright passed 8/8. All
schema-5 execution sidecars verify source-bound.

| Candidate outcome | Exact test suffix | Observed discrepancy |
| --- | --- | --- |
| passed | `test_check_the_box`, `test_click_links_which_cause_navigation`, `test_click_on_checkbox_input_and_toggle`, `test_click_the_button` | None in these cases |
| failed | `test_click_the_button_inside_an_iframe` | `ElementHandle.click` absent |
| failed | `test_click_the_button_with_px_border_with_offset` | `Page.click(position=...)` absent |
| failed | `test_click_with_tweened_mouse_movement` | Python `Locator.click(steps=...)` absent in the frozen v6 wheel |
| failed | `test_double_click_the_button` | Python `Page.dblclick` absent in the frozen v6 wheel |

The candidate XML / provenance SHA-256 pair is
`79712fbb125561c524f5742bc7a444a6a1d271cb4b32818c2b8eff6fb853987c` /
`22c61521edf886beaab6d724a484b196287a404c297ca872db8e9144223516b2`;
the oracle pair is
`296809fd378db680e5957573c043f40a40ff0f97d24f5e71118015ab39f30644` /
`4ed8dbbca05f55631b86454b090d0dca5d85ae8a2ea30d8e260608516f472cb7`.
Reports are under
`.work/official-conformance/builds/python-context-strict-v6-full.SqYkNL/`.
The immutable 6,526-row candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-v6-click8-chromium-v1.jsonl`
(SHA-256 `5c0bdcb6538b5fa8ac946df6762b5b8996b1a7141bb7b7befd4ca6f81cf8b514`)
is **1,309 passed / ten failed / 11 skipped / 5,196 not run**;
the oracle successor
`.work/official-conformance/outcomes/playwright-python-oracle-v6-click8-chromium-v1.jsonl`
(SHA-256 `5ad9aef3152b7f8c4a73ee7a40744952bbe04010197ae8af81e5a5581b0da310`)
is **1,036 passed / four failed / 11 skipped / 5,475 not run**.
Only the eight selected Chromium rows change from `not-run` in each lane;
complete invocation audits retain 6,514 applicable / 12 non-applicable and
unchanged classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
At this ABI107 checkpoint, the full 70-case Chromium click file had five
candidate passes, four failures and 61 not run; oracle had eight passes and
62 not run. This is not
full click-file conformance, let alone full-corpus conformance.

## Complete 70-case click-file candidate diagnostic, not promoted (2026-10-10)

The same frozen v6 candidate wheel subsequently ran every selected Chromium
case in the corrected 70-case `async-click-chromium-v2-manifest.json` with the
same isolated-case deadline: **48 passed / 19 failed / three error reports**.
Its schema-5 source-bound provenance verifies, with XML SHA-256
`4ee0161bf928a7f96da155304d8b5e464061fa452fb0025d2e3b16930965d22a`
and provenance SHA-256
`10beba459cd1a59a4df6ce76d5f9c90a32a8e278ce7ffeaa001e0dedfde677a5`.
The 19 failures split into eight missing `ElementHandle.click`, five missing
`Page.click(position)`, one each for `Page.click(button)`,
`Page.click(click_count)`, `Page.click(force)`, `Page.dblclick`, and
`Locator.click(steps)` in that wheel, and one offscreen-button assertion with
an unexpected 404 console line. Three isolated pytest
reports contained more than the one expected testcase, so the fail-closed
runner records them as errors rather than inferring their outcomes. Four
unrelated XLQY pages appeared in shared dedicated Chrome near the end of the
candidate run and raised memory pressure. The task-owned blank tab was closed;
the unrelated pages were not touched. The later independent full-file oracle
report is recorded above, but there is still no uncontested candidate repeat.
**None of these 70 candidate observations has been promoted** beyond the
separately verified eight-case strict-ledger successor above.

## Paired ABI107 v6 official Chromium browser results (2026-10-10)

The isolated v6 wheel at
`.work/official-conformance/builds/python-context-strict-v6-full.SqYkNL/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `bf0edae04560a7945ea08164a86552d924eac75a12d03a91531241765d6ebda2`)
retains ABI 107 and the prior 96 selected features plus only the opt-in
`context-strict-selectors` feature, for 97 actual selected features. Its
installed native payload SHA-256 is
`70af86be1fbce299cc0e0897391bf19355388299adb726ac877b8ef80b8048b0`;
installed Python source was byte-identical to the capture-time source,
including thin async/sync `Frame.focus`; subsequent Page.dblclick and
Locator.click(steps) source edits are not in this wheel. Fresh installed
browserless tests pass 35/35
focused and 389/389 full v6-profile units. One frozen default-profile-only
assertion is separately 1/1 in the prior 96-feature v5 environment, not
silently counted as a v6 test.

Three unchanged pinned selections ran in isolated candidate and independent
Playwright 1.58.0 oracle lanes with the same Chrome channel, loopback CDP
endpoint and Tailscale-bound fixture. All six schema-5 execution sidecars
verify source-bound; every selected case has a bounded 30-second attempt.
Reports are under
`.work/official-conformance/builds/python-context-strict-v6-full.SqYkNL/`.

| Selection | Candidate | Independent oracle | Candidate XML / provenance SHA-256 | Oracle XML / provenance SHA-256 |
| --- | --- | --- | --- | --- |
| Strict Context + duplicate exposed-function registration, two cases | 2/2 | 2/2 | `9bbab35d403be093dcd31025cbd187b1aaf27b8fde6d8823020752ac89224d26` / `9002027bc73f349f63e2dc8909e11464919aa8a8c848200242930cd8fe1cbbe0` | `1ae2501dcc6f71bb503b4d8e669a6ffd2faaaed424ff662d819dcbf33c9662fd` / `cc9f4fe1d6236ad0aa8db38fc4417902a675dc5224c54b6903e64285f4990258` |
| Complete `emulation_focus` file, seven cases | 7/7 | 7/7 | `43bf84cab90bf02341a3ab2923c43d6c2ecc984760903086c5425b5b10c99bde` / `34688186351f97a884d635cc17ee06d406f243fc3646d9034b6638385e2468c5` | `d8dbdbe3a1eb070790220d5f24d5c8d0cda85052eed8992c0d2b8d1a0034d211` / `ec1522cd034b7159d84f1ff10e5c0d3f5c66cede22612d8312000b182ae6a2f4` |
| Context init-script exposed-function callback, one case | 0/1, `[] != ['context']` | 1/1 | `674398a57176461dbc04e0ef8a913809dc3078cd0ce6f95291997ef9441efc74` / `813e36fe23bdd77bcb8f2b24cd6cd0fdfd0f5790d0469f3e8057dd679a187284` | `3d607deb173a062cd4961d971beafe7195459110b4fe14efe650fd97c2125b3f` / `f63416327445a8791bd49cbe01e873852f64ef01a6fad5db74dbdfff7f7de2bd` |

A separate, non-official ABI107 v6 diagnostic over the dedicated loopback CDP
profile narrows the init-script discrepancy without changing the official
assertion or ledger: the script recorded `typeof woof === "function"`, ran
past `woof("context")`, and left no Python callback (`[]`). Calling the same
function after Page readiness delivered `"later"` (`["later"]`). This rules
out a missing script or permanently broken binding installation. The frozen
wheel's Rust binding observer returned without admission when a
`Runtime.bindingCalled` event had no published frame/page/context route.
Working source now stages a bounded early event for replay when that route is
ready; 34 focused Rust release tests pass, including exact-session cancellation
cleanup, but this has not been rebuilt into
a Python wheel or exercised by the official callback case. An early-event
routing race remains a hypothesis rather than proven root cause or verified
fix. The diagnostic earned no official test credit and its borrowed browser
and created context were closed; the task-owned blank CDP tab was closed too.
Source review also found a narrower untested gap in the new staging path:
discovered-Page initialization can intentionally be canceled by a synchronous
JavaScript dialog while a synthetic Page is retained for retry. The generic
cancellation guard then deletes that session's staging record, so a binding
event queued before the dialog can arrive during the gap without a record.
The retry after dialog close does not recover an already discarded call.
This is a source-level risk, not proof that the frozen wheel's observed
callback failure used this ordering. It needs a bounded, exact-session
retry path and an official browser regression before the fix is credited.

The new exact two-case manifest
`packages/pwright-python/tests/official/browsercontext-strict-duplicate-chromium-manifest.json`
has SHA-256 `7b011a5db7d7af5151c30fd4bcc6c77dca2370034b9f0405947a19a27f22d049`.
Its separate candidate and oracle collect-only reports each identify exactly
two but earned zero pass credit. `Frame.focus`, strict Context selectors, and
duplicate registration each move one previous candidate failure to passed;
the init-script failure stays failed with new v6 evidence. Both immutable
6,526-row strict successors change exactly ten selected evidence rows and no
others: candidate
`.work/official-conformance/outcomes/playwright-python-candidate-v6-focus-context-bindings-chromium-v1.jsonl`
(SHA-256 `fcd87c82d42af74b3749a41dd2d7b3d060fc51af8e36a34daf677d46635cce7d`)
is **1,305 passed / six failed / 11 skipped / 5,204 not run**; oracle
`.work/official-conformance/outcomes/playwright-python-oracle-v6-focus-context-bindings-chromium-v1.jsonl`
(SHA-256 `8d11467c197f0f224b64cf20ba7cef7baf88e2d9150402248b3e3183258493c4`)
is **1,028 passed / four failed / 11 skipped / 5,483 not run**. Complete
invocation audits pass with the unchanged classification digest
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`;
5,197 applicable candidate rows are still not run. Broader selector API
coverage remains incomplete despite the one passing strict-Context case.

Fresh v6 installed candidate and independent-oracle environments also
recollected the complete pinned Python corpus without opening a browser:
6,501 core cases under Python 3.12 and 25 TodoMVC examples under the pinned
Python 3.10 / pytest-playwright 0.7.2 profile. All four schema-5 collection
sidecars verify source-bound. XML / provenance SHA-256 pairs are candidate
core `cc0624312bb73659e817a1f5bb00bf40556c40d13ba81bca061e39f8acfe20fa` /
`c20dcbd484b67f2f9bd5838706a4a2ecec84f927c09bc94d7acde1091b8548f6`,
candidate examples `a6d318f21b0568af3692115e8c27586e7da9a0a41bc169ae3c1886dd7e8c98e8` /
`4ba08572a0d7bbe891a2d6a6baa5eeb113dc4f15ac60b48f3f22987bc22b3d30`,
oracle core `ad4addc451d45c2d223ee6228b884ed5cef9b499d35d24b23aa8ee8144444e68` /
`fac680b65f193db08edd6569d9b229bbd82bfd648861aa92008dbf96854fe9d7`,
and oracle examples `22e5b56510340fc46b728dfa8b8c5e1b4bbccf6dfb2deb36e93987ae5211a011` /
`38086fe8b241c5a8db357c6a1cf81b5897848ead69bedaa8242f9437e896caa5`.
An initial examples collection under Python 3.12 failed the pinned runtime
preflight and produced no result; the successful examples runs used separate
fresh Python 3.10 environments. Collection earns no execution credit.

## Historical paired ABI107 v5 official Chromium browser results (2026-10-10)

The installed v5 candidate wheel and independent official Playwright 1.58.0
oracle ran three unchanged, source-pinned selections in an uncontested
dedicated-CDP window. All six schema-5 result/provenance sidecars verify
source-bound; no assertions were changed. Reports are under
`.work/official-conformance/builds/python-v5-full-collect.or0N29/`.

| Selection | Candidate | Independent oracle | Candidate XML / provenance SHA-256 | Oracle XML / provenance SHA-256 |
| --- | --- | --- | --- | --- |
| Context init-script binding, one case | 0/1, `[] != ['context']` | 1/1 | `64bf043f3cbd8b7f51dce12667942938fd0cef5f7cbe470d874c810548090d43` / `28d8347b7b2d161ebce9cd9187cc1e92bc4c01a17f3ba984839f204cd989e4f9` | `d13a6d733a20887408b8e1145731e7eeef642b7a8bfd18b76937e62fd70ed0ed` / `a1e4174a956f49bc6f1fcb1f7f64783084dcc54c9ed370fe3b04257d5c16b3f9` |
| `emulation_focus`, seven cases | 6/7; `Frame.focus` facade absent | 7/7 | `d0a58639dd1f6915a541b79a6acf99e3a50cd7285e33519b5f92e3f226448ea6` / `29ca3f47d389ae5ade80118a03d02046b0bd2bc3102797294e8238af68c53015` | `e09a6e7b61ee464d31b5cb40f1abaebcf937a20efb7fc5c758e172e89d32ae0c` / `3b841adc9eda79e94e2b6d7c53d2e7209dd75512fa9863d2051326eb1d9dbbbb` |
| Basic Context exposed function, one case | 1/1 | 1/1 | `f717f179e63b6db40b91afc9d52826db08c2aa6202bf13e0b8026077f1c7a687` / `3fea6fcc3e6f2a4b04a1b5ad97709432d51595b2f69ccb8c92d34df9e79322f1` | `1a002fa09fbeb3f98f108e1558a4d0e28f68c8fb8d8fc549a3eb716fbb5024df` / `f9027df30cd2e0db2340adc501bf512a52b5f6ec5379d8714b77a8115c5b3556` |

The one-case basic binding manifest is
`packages/pwright-python/tests/official/browsercontext-expose-function-basic-manifest.json`
(SHA-256 `5e710318670b098a4848cace5f8e084dfe0fb15e09f7a0e7d3af8135889b4d15`).
Its success demonstrates that the prior interrupted v4 90-second timeout is
not deterministic in this v5 attempt; its phase and root cause remain unknown.
Conversely, adding `runImmediately: true` to inherited scripts did not repair
the init-script callback discrepancy. The focus failure is the exact
`test_should_change_focused_iframe[chromium]` row and raises
`AttributeError: 'Frame' object has no attribute 'focus'` before native focus.
Source subsequently added thin async/sync `Frame.focus` facades that
route the child-frame ID into the existing Rust-owned native focus action;
five focused browserless tests, Ruff and touched-file Pyright pass. This
source edit was not in the installed v5 wheel; the installed v6 paired rerun
above supplies the later passing official evidence.
Read-only source comparison narrows the init-script failure without proving
its event-stage cause: pinned upstream Chromium `crPage.ts` awaits Page
initialization before resolving a `Runtime.bindingCalled` route, whereas the
current Rust `page_bindings.rs` observer can silently return when the frame
execution route is not yet present. Our topology resume and world/frame setup
can overlap, and Page evaluation fences topology but not callback
publication. A bounded stage probe must distinguish an early event drop from
a later publication/settlement race before either repair is claimed.
An additional read-only lifecycle audit found a second early-publication
hazard: Python binding source resolution may request the native Page snapshot
before its frame tree and metadata are public. Any later replay must therefore
wait for both the exact execution-context route and the owning Page's `Ready`
state, without awaiting the user callback inside topology initialization.
For OOPIFs, the owning Page target must be resolved through the frame-to-Page
and Page-session routes rather than treating the remote-frame target as the
Page target. This is a proposed bounded fix seam, not a proven root cause or
an implemented repair.
Another candidate failure retained in this v5 snapshot,
`test_expose_function_should_throw_for_duplicate_registrations`, was recorded
against an older installed package that raised `AttributeError` because its
`BrowserContext.expose_function` facade was absent. Current source has that
facade and Rust owns the exact duplicate-registration error variants. The
fresh installed v6 two-case paired run above now passes this unchanged row;
the v5 record remains historical.

Immutable 6,526-row strict successors are
`.work/official-conformance/outcomes/playwright-python-candidate-v5-focus-bindings-chromium-v1.jsonl`
(SHA-256 `b1282b4cf0861be0ecf4b6ce65310165ac0fa4b66e79275b15b76dfe236328ff`)
and
`.work/official-conformance/outcomes/playwright-python-oracle-v5-focus-bindings-chromium-v1.jsonl`
(SHA-256 `a206cfebb2c517839c09c330b75bc113ce52c355e6c12fac96dd61460ff3d95e`).
They change only nine selected rows per lane: candidate basic binding moves
failed to passed, six focus rows move not-run to passed, one focus row moves
not-run to failed, and the unchanged init-script failure gets current-v5
evidence; oracle seven focus rows move not-run to passed and its two existing
passes receive current evidence. Both complete invocation audits pass with
unchanged classification digest
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
Of 6,526 inventory rows, 6,514 are applicable and 12 are explicitly
non-applicable; the latest candidate still has 5,197 applicable rows not run.

## Prepared complete Python async click shard (collection only, 2026-10-10)

The canonical corrected selection,
`packages/pwright-python/tests/official/async-click-chromium-v2-manifest.json`
(SHA-256 `e3f67855d66abb186fed974dc9b7e614e498acfc58741a557b5146aea373b02c`),
pins the clean upstream v1.58.0 `tests/async/test_click.py` source (SHA-256
`ea614c3891174eda1f74964c8c1d4ba84b214b74e03fa194ad878636938338e7`),
the fixture chain, and all 70 exact Chromium public-API cases in that file.
Its primary-action labels are 48 `Page.click`, nine `ElementHandle.click`,
nine `Page.check/uncheck`, two mouse actions, one `Page.dblclick`, and one
`Locator.click`; these are not an exhaustive dependency proof. The original
`async-click-chromium-manifest.json` (SHA-256
`6e15f9209ca55589875a883ee7a84c14f0d339da7439f8754efa3dbd5c379e74`)
is preserved only to reverify its already-created collection artifacts: it
mislabels 13 non-Page-click primary cases as `page.click`.
At this earlier installed-v5 collection checkpoint, the strict candidate
ledger already passed one case
(`test_click_with_disabled_javascript`); its other 69 and all 70 oracle rows
are `not-run`. The corrected manifest source check and separate installed-v5
candidate/independent-oracle `--collect-only` runs each identify exactly 70
cases. Both schema-5 provenance sidecars verify source-bound. Their XML /
provenance SHA-256 pairs under
`.work/official-conformance/builds/python-v5-full-collect.or0N29/` are
candidate `0234f07ffe20f296ca234d4cb413f545d8678615abc0172938070ce7dac0cf8b` /
`43fc2ef0e9e11dd57379aad2376bc20180bf9e8492566078f2cc30d1f3d78c43`
and oracle `70035ac79ff97927f0b2ff8f6ae18356b8acc22d6569e1414ac73ae8ecc3af0f` /
`5abcdc69c7da633687b6f876b49c096d5e3a12a8ee40fde97ac33daeb93f2ff5`.
Five cases create custom contexts, two use iframe helpers, one has a distinct
origin, and one opens an alert. Three `timeout=0` actionability cases require
an outer runner deadline in a future browser run. There is no current evidence
that any case needs HTTPS, a persistent context, or a direct browser launch.
No browser executed, so neither strict ledger changes from this preparation.

## ABI107 v5 init-script wheel: browserless release checkpoint (2026-10-10)

The new isolated repaired wheel is
`.work/official-conformance/builds/python-init-scripts-v5.PeiLyY/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `0229c917a9745f9161eb71f7171706788fa000d499660829effcb7730585c510`),
with native payload SHA-256
`5c580c78a239c352d59ad6abcd4a2335fef9bd9b4a`.
It retains ABI 107 and the v4 96-feature opt-in profile while adding the
new-session inherited init-script `runImmediately: true` source repair.
A second release wheel at
`.work/official-conformance/builds/python-init-scripts-v5.PeiLyY/repro-wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `67f039c9e049a80d102b8915268bd35600b5dbc148b002b1a57b9b41a3057068`)
has an equal normalized payload; only CycloneDX build identity/timestamp
differs. Both carry the same native hash, and the packaged Python source
matched the source at this build checkpoint; later facade edits are not in
either wheel. The `init_scripts.rs` source SHA-256 is
`b5c7a037864ffbc7ef2898efac472390f488ef5e538c27ef76acad017119b951`.

Fresh isolated Python 3.9 and 3.12 installations each pass **380/380**
package units, including the new staged binding facade regression. All 81
release dependency/feature checks, addon strict release Clippy, pinned Ruff
lint/format, Python 3.9 runtime cast audit, package Pyright and current CI
Pyright pass. Four previously omitted test scripts were made strictly typed
and added to the CI Pyright path list; the expanded pinned check now reports
zero errors, with 101 focused pytest and 208 CI unittest cases passing.
These are browserless gates; the later bounded official v5 browser results and
strict successors are recorded above. Other-task nonblank XLQY pages are
left untouched whenever present in dedicated Chrome.

The isolated v5 installed-wheel candidate and independent official-oracle
environments have now each completed the entire pinned **browserless
collection**: 6,501 core plus 25 examples IDs. All four collection reports
have source-bound schema-5 provenance verified. The reports and sidecars live
under `.work/official-conformance/builds/python-v5-full-collect.or0N29/`;
their SHA-256 pairs (XML / provenance) are candidate core
`c6749fb25657c9c2472a8ba1bf3e72dd2cd2a2996feb50fd098cf6f48bb9ff11` /
`c5c3c05c0bfab60989d81209613013a7a16977cce7a06c3a40fd23863bebfcb2`,
candidate examples
`cfd4e4a940b35c75d246d38387cb2134787bdd74fb9490cea0d507a5b5bbea8f` /
`8c5e14b1826f631a3beb94530c15a94c8f83d81b5d93d5d887741bd0f6365a7a`,
oracle core
`eed421944d76fe8710bc4565b92230f703239a1f6c68e297057dabb5a4903f7c` /
`e2bbbcaf0baa790692a66fabb5c99d969c83070b2263d3e68067de1ca0396313`,
and oracle examples
`26b12069949c1363cdfdfda8e3f2f5115442f350d165efc6ff4c7d00d8964fd7` /
`53384ea0bf175d0880f66a3b3863ecffbb2b396b67fd9c5b7287187db0060d73`.
Both complete 6,526-ID inventory files are byte-identical to each other and
the existing pinned inventory (SHA-256
`3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41`).
The candidate provenance binds the installed v5 native payload SHA-256 above;
the oracle binds the independent official Playwright 1.58.0 package. This is
not browser execution and changes neither strict ledger.

## Context strict-selector compatibility gap and source implementation (2026-10-10)

The unchanged official Chromium
`tests/async/test_browsercontext.py::test_strict_selectors_on_context` row is
`failed` in the then-current strict candidate ledger and `passed` in the independent
oracle ledger. Its recorded candidate failure is immediate argument rejection:
`Browser.new_context options are not installed: strict_selectors`; it is not
yet evidence about selector matching or Chrome behavior. The installed v5
wheel still catches that option in `**kwargs` and rejects it before calling
native Rust. The upstream v1.58.0 selector parser uses a Context
`strictSelectors` default only when the operation has no explicit `strict`
boolean; explicit true or false must take precedence.

The adjacent pre-change facade audit found that `Page.query_selector` and
`Frame.query_selector` chose the first match whenever `strict` was omitted,
whereas `Page.text_content` used a strict Rust Locator and exposed no optional
`strict` argument. Current source now introduces an independent opt-in
`context-strict-selectors` Rust/FFI policy with a bounded per-Context table,
explicit true/false precedence, and thin Python `new_context`/selector paths.
The feature is **not** in the default or installed v5 wheel profile, and some
selector entry points still need policy integration. Release Rust policy tests
pass 4/4; focused Python facade/legacy/focus tests pass 19/19; touched-file
Pyright, Ruff and rustfmt pass. The full local Python unit run is 389 passed /
one failed, with the remaining failure caused by the pre-existing installed
`.venv` native profile not matching current source. No fresh wheel or official
browser rerun has occurred, so the failed strict row remains failed.

## ABI107 v4 topology replay repair and paired official browser cases (2026-10-10)

The earlier v4 `cp39-abi3` wheel is
`.work/official-conformance/builds/python-context-bindings-media-final-v4.3aZRMy/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `7dc721ea74d0b1fc277634738fe7b46f1cf738f69aa44451f8f81da8c161d8bd`,
installed native library SHA-256
`5fba690beadab487ce3167bb8bcd6c95ef379c50f140a80ceb27348cc5a55ddd`).
It changes only the Rust topology handling of an already-initialized explicit
Page's exact target/session replay: a duplicate `Target.attachedToTarget` no
longer initializes Page bindings twice or detaches the live Page. A different
redundant session still detaches. The two focused release fake-event tests
pass, as does strict release Clippy for the composition crate. Two rebuilt
wheels have equal normalized payloads (CycloneDX identity/timestamp aside).
Installed-wheel package units pass **379/379 each** on Python 3.9 and 3.12.
The complete release composition library suite now passes **411/411 executed**
with 16 explicitly ignored cases and zero failures. Three initially failing
cases had stale test-only expectations: one malformed network response fixture
had no active request, and two Locator-version assertions still expected the
old capability ceilings. Their fixtures/assertions were corrected without
changing production behavior; full-suite release Clippy, formatting, and
diff-whitespace checks pass. This is local Rust evidence, not additional
official Playwright execution credit.

The v3 wheel below had passed all those browserless gates but failed both a
basic official navigation case and a pinned Context forced-colors case at
`context.new_page()`; native teardown reported `Page binding session
initialized twice`. The forced-colors oracle passed its unchanged case 1/1.
The fixed one-case selection is
`packages/pwright-python/tests/official/context-forced-colors-chromium-manifest.json`
(SHA-256 `113f829c578cd49fa503aa16b31cfcc3b3483097978cb9c0862e9ce9b756c001`).
The v3 candidate/oracle report SHA-256 values are
`c3c055c584cd416929019fd8ff4ba3c460d6f0a7fabbe7c4ce395ca88bd2a114` /
`98516bef546943c3761d664d752ff01b6a9ab7b3e9df6b036d682f0fdb44da19`;
both schema-5 sidecars verify source-bound. These are diagnostic comparisons,
not changes to the strict 6,526-row ledgers.

With v4, the unchanged official Chromium
`tests/async/test_navigation.py::test_goto_should_work` case passes **1/1**
from the isolated installed-wheel candidate environment. Its source-bound
report/provenance SHA-256 pair is
`eda3d2d266ad4663fca7a84ad2e3014d965dfb99e11e8cd84f53da695bca221c` /
`a2073163519880dde9adbfa6a7c8a2330ff887d0d3ba8e3435fc8e29005714db`.
This proves the earlier generic new-Page crash is repaired for that case; it
does not by itself prove the forced-colors assertions. An initial v4
forced-colors attempt was refused **before execution** when other-task pages
appeared in the shared dedicated Chrome; those pages were left untouched.
After Chrome was empty, the unchanged pinned Chromium
`test_should_support_forced_colors` passed **1/1 in both the installed v4
candidate and independent official Playwright 1.58.0 oracle**. Their
source-bound report/provenance SHA-256 pairs are
`5999577917552e0dc8ec5efa655b4a1e0c5eff6606e4081e7c7065c8302e2951` /
`d4278b5d7c94e0c9bfa92fb3a93fb8755efa469b61b592a4c6aeea3d5dc2423f`
and `b8e8aa1bb94e4c6cc2b2a92bb5e629a451398eb0c640f9795ddfa55ee95d2203` /
`0a889b33a835b98891ddf2ab50cb8d73510c2576a7fc3c3fa5799c85f3786c14`.
The applicable public-API row was already passed in the oracle ledger. A new
immutable candidate successor changes **only that row** from `failed` to
`passed`: 1,295 passed / nine failed / 11 skipped / 5,211 not run, SHA-256
`0995a98a25cba216fd94757249f5e8c42e2e0827cf20783444c84e0f96a88748`.
The complete invocation/applicability audit passes with unchanged classification
digest `31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.

After the dedicated CDP service recovered externally, the unchanged pinned
seven-case Chromium `BrowserContext.clear_cookies` manifest
(`packages/pwright-python/tests/official/browsercontext-clear-cookies-manifest.json`,
SHA-256 `d83292f138c49e3a0c2734dc27e0a3f0ef152dc39b55d6267327688b0bfaed4e`)
passed **7/7 in both the installed v4 candidate and independent oracle**.
Candidate JUnit/provenance SHA-256 values are
`1e475dcba30c89a1c6d9612b16c87c6c217be9c9aaf342d5b91b9cd761bda435` /
`417d3a1703d69a7e20b50193f1616482ef0139eb1f5a0bb6a061c0dbe611d013`;
oracle values are
`97de7630e69efaf850fb21425954d0cb3dc31d7e357c521384750c853cbc7716` /
`99e97bb08fb2cb303c7644752811a12ffd7011fb5b27f2e7105cc79b54b7c68e`.
All reports are in the v4 build directory above and verify source-bound.
Exactly seven previously unrun public-API oracle rows advanced to `passed`:
the immutable oracle successor
`.work/official-conformance/outcomes/playwright-python-oracle-clearcookies7-chromium-v1.jsonl`
(SHA-256 `accd58457eb6ff6ba7459e7fa1d11bf9f2f3b35956ac836865ed74d2eb94db4d`)
contains 1,021 passed / four failed / 11 skipped / 5,490 not run. The seven
already-passing candidate rows only gained current-v4 evidence in
`.work/official-conformance/outcomes/playwright-python-candidate-v4-forced-colors-clearcookies7-chromium-v1.jsonl`
(SHA-256 `7d574909bc125ed0869fe0ec15fd1cc3d15aadc2c314fe2cc47bc9f33814161b`):
1,295 passed / nine failed / 11 skipped / 5,211 not run. Both complete
invocation audits pass, the classification digest remains unchanged, and the
candidate-pass/oracle-not-run gap is now 280 rows. This is a seven-case paired
selection, not full-corpus parity.

The bounded Chromium focus selection was pinned in
`packages/pwright-python/tests/official/emulation-focus-chromium-manifest.json`
(SHA-256 `c6e1449fbbfc55bfc6418d4cb51ee1b6e7bfeb04e123a9d173f72ea6cda708c3`).
It pins the unchanged upstream test file and fixture chain and selects all
seven public-API cases in `tests/async/test_emulation_focus.py`; each was
`not-run` in both then-current strict ledgers. Isolated candidate and independent
oracle browserless collections each find exactly seven Chromium cases, with
source-bound XML/provenance pairs under
`.work/official-conformance/builds/python-emulation-focus-collect.UYBDEq/`.
Their candidate SHA-256 pair is
`fdc6dde14b7e72355cb897da70e495dfd11f45355acb59eab4f4c5b74d4f9902` /
`df35ec5952dec9c83a2068f17e714afd7994717e227b0bf5619d2a9cc28afb87`;
oracle pair is
`1b8e8d503808403e9179abb5fd7b83c4e46e2edd3867bf77a18c2513a34a8db2` /
`92ae42986f55177fd7c5b16cdb3743ed3375ee93ff79eb40807b2a0b8ec8d3e8`.
Collection was not execution and earned no strict-ledger credit. The later
paired browser execution and immutable successor are recorded above.

A later v4 run of the fixed 39-case Chromium BrowserContext manifest was
stopped after 13 isolated case reports: ten passed, the init-script binding
case failed `[] != ['context']`, a basic exposed-function case exceeded the
90-second outer bound, and the next case errored in setup when dedicated
Chrome CDP became unreachable. No complete aggregate JUnit or source-bound
sidecar exists for this interrupted run, so **none of its rows receives
strict-ledger credit**. The partial reports are under
`.work/official-conformance/builds/python-context-bindings-media-final-v4.3aZRMy/candidate-browsercontext-fullgap39-v1.xml.cases/`.
Only task-owned runner/pytest processes were stopped; shared Chrome was not
restarted or closed. CDP was unreachable at that checkpoint despite its
loopback listener. A separate Chrome process later recovered externally; the
seven-case selection above was run only after the dedicated CDP reported ready.

Read-only comparison with pinned upstream Chromium code identified one concrete
init-script parameter difference relevant to the `[]` symptom: when initializing a
new Page with already-registered Context scripts, upstream sends CDP
`Page.addScriptToEvaluateOnNewDocument` with `runImmediately: true`; dynamic
script registration does not. The Rust source now distinguishes these phases
and sends that flag only during new-session initialization. Fake CDP command
tests assert both exact payloads. Focused release tests pass 6/6 with
`init-scripts` alone and with `context-bindings,init-scripts`; strict release
Clippy passes for both configurations after a separate no-behavior lint
cleanup in Locator timeout handling. **This source change is not in the
already-built v4 wheel and has not rerun the official failing case**; no
strict-ledger row advances. The independent basic exposed-function timeout
still lacks a measured phase/root cause; a possible silently dropped binding
event remains a hypothesis, not a confirmed fix.

For a later safe-window browser check, the unchanged failing official
Chromium case now has its own manifest,
`packages/pwright-python/tests/official/browsercontext-expose-function-init-script-manifest.json`
(SHA-256 `fdaecf074e6f043415fe74a93c4ae6f541b76102840aa280fc65c67600932f15`).
It is the exact case from the pinned 39-case Context selection, with the
same upstream test/fixture hashes. Candidate-v4 and independent-oracle
`--collect-only` runs each report one selected `[chromium]` item; their
source-bound XML/provenance SHA-256 pairs are
`ab3a1b4c71f4007eb2389373f6aad4682af330d2a510379850f0f00e699e70f7` /
`e3c37e8f30e80bdd09fa8121598a577be9420bcbb26848791e6584e19b70d116`
and `ae83293448f22d732bec74d416e3793d9b7c083f4253a912c237240a14a88504` /
`6f6b695fce7616370721fdc22207d00a31dae2e7e8fcb8186279f4dd62c72bd5`.
The reports are under `.work/official-conformance/results/` with
`playwright-python-{candidate,oracle}-browsercontext-expose-function-init-script-collection-v1`
stems. Collect-only JUnit has zero executed tests; neither lane receives
strict-ledger credit from this preparation.

The v4 candidate's complete core (6,501) and examples (25) **browserless
collections** verify source-bound with separate Python 3.12/3.10 environments
and the pinned examples requirements. Their XML/provenance SHA-256 pairs are
`c5d2f7296dfe7cb0e65ef098dc773fa04ea53587ac6c501d2ea14a1314cf872f` /
`e4b5a796865efee852a4cd00b224a6ede7aa9f6c25e379027a8afd670f6b68ab`
for core and
`090603fcddcadc2a09e4a9d9cd1bbd70d12bef863e6eae7435fce7ce22e20f58` /
`45bd99306b4588a571af132f2e601f4701865369a574e136a1e7d2b73d92f994`
for examples. Collection still earns zero execution credit; the independent
oracle source-bound collections remain those recorded below.

## Archived ABI107 v3 wheel and complete browserless collection (2026-10-10)

The v3 diagnostic `cp39-abi3` wheel is
`.work/official-conformance/builds/python-context-bindings-media-final-v3.n1jD84/wheels/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `3efd9af5d27f4e776c5ccea55edfbbb3757cf83136eadd51935515b044dc37f1`).
It selects the opt-in Page/Context bindings and media/forced-colors features,
exports ABI107, and has the same normalized payload as a second release
build; the CycloneDX serial and timestamp are intentionally normalized in
that comparison. The installed native library SHA-256 is
`8bf8094b516cc426d367f9964b2a145eed875d6050d586ef80b1d3a078edd290`.
The installed Python facade matched the then-current source byte-for-byte. Separate
Python 3.9 and 3.12 installed-wheel runs each pass **379/379** package unit
tests, including the formerly stale native-profile assertion. Strict Pyright,
Ruff check/format and the Python 3.9 runtime-type check pass. The first
candidate wheel exposed a Python 3.9 eager-`asyncio.Lock` construction bug
after a prior loop closed; v3 creates the lock on its owning
loop. A test fake's cross-loop `asyncio.Event` construction was corrected too.

The pinned upstream source is Playwright Python v1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. Under the manifest's
independent candidate and oracle environments, all four **browserless
collections** are source-bound and verified (schema-5 provenance); paths below
are relative to the build directory above:

| Lane | Collection | Collected | XML SHA-256 | Provenance SHA-256 |
|------|------------|----------:|------------|-------------------|
| Candidate | `candidate-core-collection` | 6,501 | `d41bb6a223905d2531ecd70ffe69dd5bc81393864f8ad3df10f326cfd319a6ef` | `af4421a9b8378ce3223ec6cde5d0a63d042da5c69648ac1a4c2fc60e74227030` |
| Candidate | `candidate-examples-collection` | 25 | `15469ecb5f9eb6ae7ffea1a9b11b2a6320f11e6a68dd879c0c9b82edb2f65787` | `6edd650134a57303294e996f414d7df9e1d91dc671369e76189468314c4a09d8` |
| Oracle | `oracle-core-collection` | 6,501 | `8c74541127615d931fa2ed850c555b2e8e05e3a5ddfc68fa8974ab0f99d89968` | `83eb7f9eedb48e3b6674af9dfefdaf56f9b2d5820ba38ae2f61cec56cf1f5b3c` |
| Oracle | `oracle-examples-collection` | 25 | `61b4d84ac35357c51d700ecd6f486a02c0e1db972e9f8a4e6d3e956f093edf15` | `37ef5f07e446275df1c69d54c0114f6db42c06c329cafbb95cf0cce4e2dfc5a5` |

The examples environments use Python 3.10 and the exact SHA-pinned upstream
requirements; the oracle imports official Playwright 1.58.0, not this wheel.
**Collection is not execution:** none of these 6,526 cases received new
candidate/oracle strict-ledger credit. The ledgers at the top still describe
earlier artifacts. Shared dedicated Chrome currently holds other-task pages,
so this build did not undergo full independent official browser execution;
its later pinned single-case failures are described above.
Its Context binding current-document rollback, raw native registration-Future
cancellation, and binding settlement resource-cycle risks remain open. The
new Rust FFI admission regressions pass 2/2 but prove rejection at the shared
byte/count budget, not browser-level progress or a settlement fix.

## ABI110 BrowserContext.request oracle catch-up (2026-10-10)

The existing pinned v1.58.0 exact-source
`packages/pwright-python/tests/official/fetch-browser-context-manifest.json`
(SHA-256 `94067ae8c5817c7c21e7e18186065cb34501a56938fb302eee6b721983958f86`)
selects unchanged async and sync `BrowserContext.request` tests. The latest
candidate ledger already recorded 51 async and 47 sync Chromium passes from
an earlier candidate build, but the independent oracle still recorded all 98
as `not-run`. With the required dedicated-Chrome default `about:blank` Page
kept open and the upstream fixture bound only to the verified Tailscale IP,
the official oracle passed **51/51 async and 47/47 sync** isolated cases. Its
actual source package loaded from the independent oracle venv. Both schema-5
provenance sidecars verify source-bound against current sources and the exact
capture-tooling/harness archive at
`.work/official-conformance/python-abi110-browsercontext-lifecycle-exact-archive.Bgg4kY/`.

The async report and sidecar are
`.work/official-conformance/results/playwright-python-oracle-abi110-fetch-browser-context-catchup-async-chromium-v2.{xml,provenance.json}`
with SHA-256 values
`7e071f5b3529da4e8ea39b7696e8e02f87ef5c49c85a45ccd760ea764224b774` /
`af691e346ddf28e509af4b67e60ec0578b7cb5057e8e948d8d3f0f300ee1abe6`.
The sync pair is
`.work/official-conformance/results/playwright-python-oracle-abi110-fetch-browser-context-catchup-sync-chromium-v1.{xml,provenance.json}`
with SHA-256 values
`1371107b4c0f857ab91f3506e177deef3a07a0ab27a5a5b5ed80ef828cd319bb` /
`9e0e7074b7d6551adf487146b1074fd194614a5240eba2d9310bace817dd4680`.
The first async diagnostic was interrupted and excluded: without a default
Page, Chrome rejected oracle attachment at `Browser.setDownloadBehavior`
before test bodies. Its partial case files received no ledger credit.

Only the 98 intended `tests/{async,sync}/test_fetch_browser_context.py`
Chromium IDs changed from `not-run` to `passed` in the 6,526-row oracle ledger;
ID order is unchanged and no other outcome changed. The strict oracle
successor is
`.work/official-conformance/outcomes/playwright-python-oracle-abi110-fetch-browser-context-catchup-both-chromium-v1.jsonl`:
934 passed / ten skipped / four failed / 5,578 not run, SHA-256
`fc9798778600c0a2ee8bea947a0c9e48dc37f3deb42fdada5516d50f9754bb2b`.
Its 6,526-row applicability audit retains classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
These oracle-only additions are not new candidate passes. The first
current-wheel candidate rerun correctly refused before test execution because
two non-task, nonblank pages appeared in the shared dedicated CDP browser.
Those pages were left untouched. Once they closed, the isolated current-wheel
candidate passed the same 51/51 async cases. Its JUnit/provenance SHA-256 pair
is `1d1e09e657a85f5da7e3504c47cdf3de069b50e96e5849d134bcef18ec4b5097` /
`0213c774489a3c97a37257349fbf5080382411fcf84a87e3a1968b859442e5bb`,
at `.work/official-conformance/results/playwright-python-candidate-abi110-fetch-browser-context-current-async-chromium-v1.{xml,provenance.json}`.
The schema-5 sidecar verifies source-bound against current and archived
capture tooling. Exactly those 51 existing candidate pass rows received a
current-wheel result reference, with zero outcome changes. The successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-fetch-browser-context-current-async-chromium-v1.jsonl`
still records 1,279 passed / 11 skipped / four failed / 5,232 not run, SHA-256
`a16c28e8e8d6b68b7ffc06fbc661f9e76ab3169968bfd2e041883fdc1b0f88b6`.
The isolated current-wheel sync candidate rerun subsequently passed the same
47/47 cases. Its JUnit/provenance SHA-256 pair is
`726a0f41723c33b124ecaa0baf9accdd40bd5543e56b8af42b9d4c4fccaeb794` /
`80c03f819f15b56c9dcfc27e7cf0ab0c1972150c07a7fd602768acb926cf5f25`,
at `.work/official-conformance/results/playwright-python-candidate-abi110-fetch-browser-context-current-sync-chromium-v1.{xml,provenance.json}`.
Its schema-5 sidecar verifies against the same installed wheel SHA-256
`f64f8730eac5fe8f3ea5cb5b997caa8fb3b6c4488a7947115788e7f071cd5dc2`.
The immutable candidate strict successor updates only `resultFile` on these
47 previously passed sync IDs:
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-fetch-browser-context-current-both-chromium-v1.jsonl`
(SHA-256 `51cb64812fa743773f78288c6d2b96b5bf1d1c66385b665f1002511280094e83`).
All 6,479 other rows are byte-identical, the 6,526-row invocation audit
passes, and totals remain 1,279 passed / 11 skipped / four failed / 5,232
not run. At this pre-verified35 checkpoint, 345 candidate-passed /
oracle-`not-run` rows remained; the later promotion reduces that gap to 331.
Full-corpus execution and final frozen-build parity remain open.

## Paired whole-file BrowserContext gap (diagnostics and selective promotion, 2026-10-10)

The outcome-independent selector is every oracle-`not-run` Chromium row in
the pinned `tests/async/test_browsercontext.py` file: 39 IDs, including all
25 candidate-`not-run` Chromium rows. The new
`packages/pwright-python/tests/official/browsercontext-full-gap-chromium-manifest.json`
(SHA-256 `f78304a22030dad32462220b455c6b77168a7ee96272d337b0e1f7e49748c3f7`)
matches that ledger-derived ID list exactly and pins the unchanged test,
fixture, server and helper source hashes. The source checker passed, and the
independent oracle collected 39/39 unchanged tests; its collection-only JUnit
and schema-5 sidecar are
`.work/official-conformance/results/playwright-python-oracle-browsercontext-full-gap-chromium-collection-v1.{xml,provenance.json}`
with SHA-256 values
`109742224e05e525e0f8c3daf1005d85aafc9c396d6a107f1e7b14b28c260a75` /
`9aa9c51277ffc50e93a0d3af61475c644ab2460e99a557fd99d45aa28168e53f`.
The sidecar verifies against current sources. Collection is not execution; the
subsequent paired runs were initially diagnostic, with only the verified
35-case subset later promoted as recorded below.

An initial isolated-case independent oracle execution then completed all
39 unchanged tests: 29 passed and ten failed. Its JUnit/provenance SHA-256
pair is `7465741314a2b80a332685eef67e823f5c2d71462d489ee1f70e4bce4b4c0b75` /
`14c61954143b5ee3621e2fc37b31a80e717c82fae22ddad6e314cca86cfe01af`
at `.work/official-conformance/results/playwright-python-oracle-browsercontext-full-gap-async-chromium-v1.{xml,provenance.json}`;
schema-5 source-bound verification passes. The ten failures are not silently
skipped. Four negative HTTP-auth tests receive Chrome's
`ERR_INVALID_AUTH_CREDENTIALS`, but the upstream fixture labels the headed
full-CDP Chrome as headless shell and therefore sets `fails_on_401=False`;
they required an explicit `--browser-channel chrome` oracle rerun so the
recorded fixture identity matches the actual dedicated Chrome. Two
null-viewport tests receive the right
error content with `AsyncLaunchedBrowser.new_context:` instead of upstream's
public `Browser.new_context:` prefix because the oracle adapter reclasses the
Browser object. Four popup tests remain unresolved: one page-event timeout
and three new Pages with `url=''`. The dedicated Chrome lacks Playwright's
`--disable-popup-blocking` launch flag, a possible but unproved cause; the
cross-process fixture hostname resolves to the same Tailscale IP. A non-task
Chrome page was present immediately after the run, so exclusive CDP state
cannot be established throughout. Treat this as a diagnostic run, not a
strict-ledger promotion, until the popup fixture is understood and uncontended
paired reruns establish valid outcomes. Do not restart the shared Chrome while
another task is using it.

The oracle CDP adapter now preserves the public `Browser.new_context` name
while still hiding fixture-owned Contexts; async and sync browserless
regressions and the relevant 83-test harness suite pass. Its runtime-tree
pin was updated for future runs. The diagnostic 39-case report remains bound
to the archived pre-fix runtime, not this changed adapter: it is not
retroactively verified or promoted.

A second complete 39-case independent oracle run used the corrected adapter
and explicit `--browser-channel chrome`, matching the dedicated full Chrome.
The CDP inventory contained only this task's `about:blank` Page before and
afterward. It recorded 35 passes / four failures / zero skips: the four
remaining failures are precisely the popup page-event timeout and three
`url=''` assertions. All four negative HTTP-auth cases and both
null-viewport prefix cases that failed in the first diagnostic now pass.
The source-bound result is
`.work/official-conformance/results/playwright-python-oracle-browsercontext-full-gap-async-chromium-v2.xml`
(SHA-256 `0f17a39b482acd1839914b1ce3133ea35251364ac14c41f7eecb78d7ad931197`),
and its sibling sidecar is
`.work/official-conformance/results/playwright-python-oracle-browsercontext-full-gap-async-chromium-v2.provenance.json`
(SHA-256 `ac0ae63b722da512aede6d0dd358237a5b0dee331fe5cac7da813fcb28f8b40b`).
Independent schema-5 verification passes. This does not relabel the first
diagnostic. At this checkpoint it had not yet been promoted; 35 of its 39
cases were subsequently promoted through the source-bound derivation below.

The paired ABI110 current-wheel candidate run used the same pinned 39-case
manifest, async Chromium selection and explicit `--browser-channel chrome`.
It completed with 25 passes / 14 failures / zero skips. Its result is
`.work/official-conformance/results/playwright-python-candidate-abi110-browsercontext-full-gap-async-chromium-v1.xml`
(SHA-256 `2959f793ffab73c4e511ca20c0088424dfa1d882dcb167475afac264d6812bac`),
and its sibling source-bound sidecar is
`.work/official-conformance/results/playwright-python-candidate-abi110-browsercontext-full-gap-async-chromium-v1.provenance.json`
(SHA-256 `85c334c9bfda61209a6f81eb937d18bbe2ce6815b54b885f247b983f690dc33a`).
Both this candidate sidecar and the oracle v2 sidecar independently verify
against their recorded schema-5 sources; both XML files contain exactly the
39 manifest cases. Comparing the two reports case by case yields 22 passes in
both, 13 oracle-pass/candidate-fail cases, one failure in both
(`test_page_event_should_have_an_opener`), and three
oracle-fail/candidate-pass cases. The three oracle-only failures are the popup
URL assertions, not evidence that candidate popup behavior is correct.

The remaining oracle popup behavior has two distinct symptoms: an opener
page-event timeout and three new Pages observed with `url=''`. Popup blocking
is a hypothesis because the dedicated Chrome launch lacks Playwright's
`--disable-popup-blocking` flag, but it does not yet explain every observed
event. Early event publication before the popup's first navigation is another
unproven hypothesis. The dotted cross-process fixture host appears in only
one of these cases, so it cannot alone explain all four. The strict 6,526-row
ledgers remained at 934 oracle passes and 1,279 candidate passes at this
diagnostic checkpoint. The four popup cases have not been promoted; the other
35 cases now have source-bound selective strict-ledger outcomes below. This
does not establish the popup fixture's cause or correctness.

### Verified 35/39 strict-ledger promotion

The immutable source-bound derivation receipt is
`.work/official-conformance/results/playwright-python-browsercontext-verified35-v1.receipt.json`
(SHA-256 `2858b98b3b8ca5be99496367c1746f54b782a946fe9acc01a26e101e83c4f77b`).
It re-verifies both unchanged 39-case schema-5 sidecars, their aggregate and
isolated case IDs/statuses, the pinned source manifest and the exact 6,526-row
base ledgers (candidate SHA-256
`51cb64812fa743773f78288c6d2b96b5bf1d1c66385b665f1002511280094e83`,
oracle SHA-256
`fc9798778600c0a2ee8bea947a0c9e48dc37f3deb42fdada5516d50f9754bb2b`).
Exactly 35 manifest IDs were selected; the four oracle-failing popup IDs were
explicitly excluded and remain `not-run` in **both** strict ledgers. The
receipt records every original and selected status. The filtered candidate
JUnit is
`.work/official-conformance/results/playwright-python-browsercontext-verified35-candidate-abi110-v1.xml`
(SHA-256 `53230ef07e05a979012d0fa481c281d97bb615c59838b01afdb2143dcc2d24af`):
22 passed, 13 failed. The filtered oracle JUnit is
`.work/official-conformance/results/playwright-python-browsercontext-verified35-oracle-v2.xml`
(SHA-256 `ce1796a622116280a1d9659fb90ff9bbbc0e0cd5a83da420e46c1204007ab82d`):
35 passed. Neither filtered report relabels or overwrites its 39-case parent.

The candidate successor
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-browsercontext-verified35-chromium-v1.jsonl`
has SHA-256 `a44a39ac38f15afb07bd249f7a9f748cc1c47fd5b1c1d068dce574b5b74c21f6`:
eight newly passed and 13 newly failed cases, with 14 earlier passes refreshed,
yielding **1,287 passed / 17 failed / 11 skipped / 5,211 not run**. The oracle
successor
`.work/official-conformance/outcomes/playwright-python-oracle-browsercontext-verified35-chromium-v1.jsonl`
has SHA-256 `63b2962367ce219c0d174514f21fcd5c3a7b18940b6d829b3e55fbd9ba11cd68`:
35 newly passed cases, yielding **969 passed / four failed / ten skipped /
5,543 not run**. Both 6,526-row summaries and invocation-applicability audits
pass. Read-only `verify-ledgers` rechecks the receipt, both filtered reports,
base SHA pins, exact selected-row results and `resultFile` paths, unchanged
other rows, excluded popup rows and projected counts. Its focused tool suite
passes **17/17** tests; this is validation of the derivation and merge, not
full-corpus conformance. The candidate-passed / oracle-`not-run` gap is now
**331** rows.

The 13 candidate-only failures (oracle passed) are the Context
binding/function cases (four), offline cases (two), iframe CSP case (one),
null-viewport option checks (two), modifier-click popup cases (two), forced
colors case (one) and strict-selectors case (one). The receipt contains their
exact pytest IDs. The four excluded popup cases are not counted as candidate
successes or failures in this promotion despite their diagnostic 39-case
results.

Local source now adds a thin async/sync `Frame.add_script_tag` facade over
the existing Frame evaluation/Handle owner; six browserless unit cases pass.
It targets the iframe CSP discrepancy but was not present in the installed
ABI110 wheel used for the paired official run. No official outcome has been
changed on the basis of those local tests. Read-only option triage shows the
forced-colors and strict-selectors cases currently fail at `Browser.new_context`
keyword admission. Both need Context-scoped Rust policy and lifecycle tests;
accepting the Python keywords alone would not establish browser parity.

### Local post-promotion candidate wheel (build and earlier refused attempt)

The current source was built in release mode with the new opt-in
`locator-click-modifiers` native method and thin async/sync `Frame.add_script_tag`
facades. The portable repaired wheel is
`.work/official-conformance/builds/python-context-gap-click-frame-v1.pO5u1c/repaired.X2v9sv/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `ca97ca7b2387f69155170ad5044c264e7039eb8df934153b1083505883c977ae`).
It loads from a new isolated environment, advertises the modifier capability,
and passes the complete 351/351 Python unit suite. A second portable build
has a different archive SHA-256 solely because its generated CycloneDX SBOM
contains a new UUID/timestamp and `RECORD` hashes that SBOM; all other
extracted wheel files, including the native binary and Python source, compare
byte-identically. An initial official 39-case candidate rerun with this wheel
was refused before test execution because two non-task Chrome pages appeared
after the task-only preflight. This task closed only its own `about:blank`
Page; that attempt produced no report or strict-ledger outcome. The paired
25/14 report and the first verified35 successor above describe the older
installed wheel; a later uncontended rerun is recorded immediately below.

### Current-wheel BrowserContext 39-case rerun and verified35 candidate refresh

The same pinned 39-case async Chromium manifest ran to completion from an
exclusive task-owned dedicated-CDP window with explicit
`--browser-channel chrome`. The portable wheel above produced **32 passed /
seven failed / zero skipped**. Its complete unchanged-test report is
`.work/official-conformance/results/playwright-python-candidate-context-gap-click-frame-current-async-chromium-v2.xml`
(SHA-256 `98fcd60681bdf26b38a075cc3526e0fc3e89fecfb02b2e89773e4ac0bac7e7d4`),
with schema-5 source-bound sidecar SHA-256
`d9c82dd0527bb25504d6020c8f93dd67b4cc73de2776fc22617f828de39f3638`.
Its native artifact SHA-256 is
`9b89957193a8a1c0aef2075eb25698b60e62a7dd61f57359cbc8e417fc6bb4ff`.
The sidecar verifies against the pinned upstream and capture environment;
all 39 aggregate and isolated results are accounted for. The task-owned tab
was closed after the run, leaving its Pwright tab list empty.

At this historical current-wheel checkpoint, seven of the 13 earlier
oracle-pass/candidate-fail cases passed: both
offline cases, both null-viewport conflict cases, the iframe CSP case and
both modifier-click popup cases. The six then-remaining candidate-only
failures were four Context binding/function cases, forced colors and strict
selectors; forced colors is now a source-bound v4 pass above.
The seventh diagnostic failure, popup opener, also failed in the independent
oracle. The other three oracle-failing popup cases happened to pass in this
candidate run; none of these four receives strict-ledger pass or failure
credit until oracle behavior is understood.

`scripts/refresh_playwright_python_browsercontext_candidate.py` replays the
earlier 35-of-39 receipt, both earlier strict ledgers, the new complete
schema-5 report and all isolated statuses against pinned source hashes. Its
receipt at
`.work/official-conformance/results/playwright-python-browsercontext-currentwheel-verified35-candidate-v2.receipt.json`
has SHA-256 `c19d8054cb5a386d2a0efc25192f9ef997165f8ee614e05ff48d6d586ca4bcf1`;
the derived 35-case candidate JUnit SHA-256 is
`0fff716536063fb634c791d5c3979594199f8ac308ec0b106e3c5cc3d1309f66`:
**29 passed / six failed**. Only these 35 rows were refreshed, including
seven `failed` to `passed` transitions; the four excluded popup rows and all
other rows remained unchanged. The new immutable candidate ledger is
`.work/official-conformance/outcomes/playwright-python-candidate-currentwheel-browsercontext-verified35-chromium-v2.jsonl`
(SHA-256 `402c08878c6888f76cab8a7a6f9e4c5ee73d971b232fd1f3288737aa943ac21c`):
**1,294 passed / ten failed / 11 skipped / 5,211 not run** across 6,526 rows.
Receipt, successor and invocation-applicability audits pass; the new tool's
five hermetic tests pass. The independent oracle verified35 ledger remains
unchanged at 969 passed here. Two of the six remaining feature families now
had local source repairs, but no new official-browser pass credit at that
checkpoint.

The opt-in `context-forced-colors` Rust Context policy and thin Python
async/sync option pass eight focused Rust release tests and an isolated
Python-addon release check. Context defaults initialize Page/OOPIF sessions
before resume; explicit Page media overrides remain separate bounded state.
The opt-in `context-bindings` Rust registration/dispatch path and thin Python
async/sync facade pass 19 focused Rust release tests, seven Python units and a
combined addon release check with forced colors; its dependency closure adds
no new Pwright package beyond `page-bindings`. Neither feature was in that
historical portable wheel; both are in v4, with forced colors now passing and
binding discrepancies exposed by the interrupted 39-case run above. Binding
`runImmediately` installation can leave
a current-document function if a later CDP installation step fails; dropped
in-flight registration/initialization remains under review. Existing Page
media updates do not fan out to already-attached OOPIFs. None of these
limits is hidden by the local unit results.

The sixth remaining case, `test_strict_selectors_on_context`, is design-only
so far. Upstream uses a tri-state explicit strict override falling back to
Context policy. The current Pwright facade rejects the Context option, and
the legacy `Page.query_selector`/`Page.text_content` behavior is not uniformly
compatible, so admitting the keyword alone would not fix the case. This
requires a separate opt-in Rust Context selector-policy slice and tests for
default, strict and explicit non-strict override behavior.

A local Python 3.9 runtime audit also caught evaluated PEP-604 unions inside
`typing.cast` in the source-tree storage-state path and official provenance
verifier. Quoting those cast annotations now passes the 11-file audit, 46
checker/provenance tests and a real Python 3.9 `Path` storage-state smoke.
This source fix postdates the current portable wheel and adds no official
test or strict-ledger credit.

### Independent official keyboard oracle 29-case promotion

The complete pinned `tests/async/test_keyboard.py` async Chromium selection
contains 29 unchanged official cases, manifest SHA-256
`35c8c97c20affae0129c14b7e7f0917d592825b235cd3612013ced7cfd776197`.
An independent Playwright package (tree SHA-256
`d7655707035107f13a81db0e1d52d839c2bf42bf2008478fba610299cd6da149`)
ran every case through the same verified Tailscale fixture and dedicated-CDP
Chrome with explicit channel: **28 passed, one upstream skip, zero failures**.
The report
`.work/official-conformance/results/playwright-python-oracle-keyboard-async-chromium-v2.xml`
has SHA-256 `e5862c438f7faa375b300c0ab2b5018132e2ef001b68d2803025e4e316e572b5`;
its independently verified schema-5 provenance sidecar SHA-256 is
`8f4af08d2e3c9cc691ccea2fde90b6d7e5418f106bd7a6f6d69028a58d3050a6`.
This complete v2 execution supersedes neither the older diagnostic run nor
its reported cross-origin failure; it is separate evidence.

`scripts/promote_playwright_python_keyboard_oracle.py` checks the exact
manifest, upstream sources, sidecar, isolated reports, prior 6,526-row
oracle ledger and every intended row transition. Its source-bound receipt is
`.work/official-conformance/results/playwright-python-oracle-keyboard-async-chromium-v2.receipt.json`
(SHA-256 `fd4bad76676aa205394b1fc95d3aa669eaed3cbc0a3b5e9467dd3513486ff4f0`);
five hermetic tests pass. Exactly 29 previously `not-run` rows changed to
28 `passed` and one `skipped`; all other rows are byte-identical. The strict
oracle successor
`.work/official-conformance/outcomes/playwright-python-oracle-keyboard-async-chromium-v2.jsonl`
(SHA-256 `ab24acc36857ae684080ff0d7eb3300bb3f5c6befba96d93a888427fb1fa86c5`)
records **997 passed / four failed / 11 skipped / 5,514 not run**. Its full
invocation audit passes. Aligning that successor with the new candidate
successor above left **304 candidate-passed/oracle-`not-run` rows** at that
checkpoint; the later Page-route oracle reduces this to 287. No
candidate pass was inferred from this oracle-only run.

### Independent complete Page-route selection and public17 promotion

The existing exact-source
`packages/pwright-python/tests/official/page-route-existing-coverage-manifest.json`
(SHA-256 `da5aacd59a19ee1852ffe7d7c0918a32cd8115895ce770ee409523297f8cdece`)
pins 18 unchanged async Chromium cases from `tests/async/test_page_route.py`.
The independent official package ran all **18/18 passed**, in isolated
subprocesses with explicit Chrome channel and the verified Tailscale fixture.
The complete report at
`.work/official-conformance/results/playwright-python-oracle-page-route-existing-async-chromium-v1.xml`
has SHA-256 `fb5b3e0df88f636525808096e7b2755dec0653e2098d99a8fc8db3cdfc11d0b7`;
its schema-5 provenance sidecar SHA-256 is
`eb5e90242f5ff294acd97408044f3c176f4e772b54dd5cbac08a400fa37cfec5`.
The sidecar verifies against pinned source and all 18 isolated reports.
The task-owned `about:blank` tab was closed afterward.

The full 6,526-row invocation audit classifies 17 cases as applicable public
API behavior. The 18th, `test_should_work_with_glob`, directly calls private
`playwright._impl._glob`/`_helper` functions; it is
`implementation-internal`, `non-applicable` and already had a historical
`passed` row in both strict ledgers. Its new diagnostic pass is recorded in
the complete report but is **not** refreshed or counted as new public-API
credit. `scripts/promote_playwright_python_manifest_oracle.py` verifies that
classification against the exact audit ledger, derives a 17-case filtered
report (SHA-256 `f791a436d61bf6cf75623c18ee12027885ca277d50d45cb0597169da748c5ee0`)
without modifying the parent, and records an explicit exclusion in its
source-bound receipt (SHA-256
`fb0f456fbf3cf2915ddce9b59d16632e1d2e3a5d651fa4cbbfda43025afe7303`)
at
`.work/official-conformance/results/playwright-python-oracle-page-route-existing-public17-async-chromium-v1.receipt.json`.
Its five hermetic tests pass.

Exactly 17 oracle rows changed from `not-run` to `passed`; all other 6,509
rows, including the excluded private-helper case, are byte-identical. The
strict oracle successor
`.work/official-conformance/outcomes/playwright-python-oracle-page-route-existing-public17-async-chromium-v1.jsonl`
has SHA-256 `1dead4360f1813878780093ea80c836f937ba636a51076f818bd5a36110f36d1`:
**1,014 passed / four failed / 11 skipped / 5,497 not run**. Receipt,
selected-row and complete invocation audits pass; classification digest remains
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
Aligned with the unchanged latest candidate successor, exactly **287**
candidate-passed/oracle-`not-run` rows remain. This oracle-only run does not
freeze or rerun the candidate build and does not establish full-file or
full-corpus parity.

The next independent-oracle slice is prepared, not executed: the pinned
`browsercontext-clearcookies-oracle-manifest.json` (SHA-256
`213b26975799497697269be83c333b814fdcf1ea17606b3e445d900afb9321d3`)
selects exactly the seven Chromium cases in the complete upstream
`test_browsercontext_clearcookies.py` file. All seven are public-API,
candidate-`passed`, oracle-`not-run` rows in the current ledgers. The
independent v1.58.0 oracle collected 7/7 with no browser launched; its
collection JUnit/provenance pair is SHA-256
`9a745df347d173eb44170dad34e770cc82bcc6a500d9f30b02c29ae8b87d8016` /
`99259c2625943a5fdb529aed25f8f0a6469951d3629121f66e0308ac762fa4f6`
and source-bound provenance replay passes. The existing-wheel candidate also
collects the same 7/7 with a source-bound JUnit/provenance pair
`4304dc7361fec0fc3a6e482fe2513f04a046104e994f71c5a258c2105809ad0a` /
`50d7a25f62dacf51395d340bfe17c0de636615de7f752c016f766dd70eebe24c`.
Collection alone earns **zero**
new strict-ledger credit; real isolated execution must wait until the shared
dedicated Chrome has no other task's pages.

Static inspection originally identified 12 of the 25 candidate-unrun cases
with definite public API gaps: four Context binding/function cases (only
Page-scoped binding exists), two runtime `BrowserContext.set_offline` cases
(only initial offline policy existed), four `Browser.new_context` option cases
(`no_viewport`, `strict_selectors`, `forced_colors`), and two popup clicks
requiring `Page.click(modifiers=...)`. The two exact upstream `no_viewport`
conflict cases now have matching async/sync preflight errors before native
context creation; five local unit cases pass. The parallel Node entrypoint
has the same conflict preflight with three adapter checks and one entrypoint
integration check passing, but the general valid
`no_viewport=True` capability remains unsupported. This was pre-execution
implementation triage, not official execution evidence. The remaining cases
are included in the paired diagnostic reports above; no failure or pass is
inferred from static inspection or local unit tests.

The opt-in `context-bindings` Rust registry and thin async/sync Python facade
are now present locally, but the strict candidate ledger above still reflects
the older wheel. A read-only review found that Python's single binding watcher
serialized callback completion and could deadlock when an exposed async
function evaluates another exposed function. The watcher now dispatches
independent tracked tasks; Rust's 64-pending-call cap bounds native entries,
not by itself the callback tasks after Page retirement. Close cancels
outstanding callbacks and waits up to five seconds for cleanup. Both Page and
Context registration
facades now shield the entire native registration/rollback transaction from
caller cancellation, retaining a live callback if native installation commits
after cancellation. Nested-call, close, and cancellation unit regressions pass.
At that earlier checkpoint, 366 Python source units passed with one stale
native-profile identity assertion excluded. The current source unit suite now
passes **376 cases with that same assertion excluded**; the preserved source
native binary predates the opt-ins. Strict Pyright with the complete dependency
environment and Ruff pass. Browser behavior still requires independent
validation against the final repaired wheel. These local checks provide no
new official pass credit.

The follow-up read-only lifecycle audit found three additional Python gaps:
the callback child could escape during native acknowledgement, a callback
calling `Browser.close()` could cancel its own close operation, and Page churn
could retire Rust pending entries without retiring Python callback tasks.
The implemented repair adds independent Python 64-dispatch admission and
explicit child-task ownership; Rust's pending count alone does not bound the
language tasks. Callback-initiated close does not cancel or wait on itself,
and only a dispatch belonging to that Browser becomes its close marker. A
fourth gap allowed canceled registration callers to accumulate shielded tasks
behind a Context lock. Browser-shared 64-transaction admission now precedes
task/coroutine construction and remains held through native completion.
The 22 focused binding regressions include Page/Context/cross-Context
saturation, failure/reuse and cross-Browser ownership. This is local lifecycle
evidence, not official test execution.

Two Rust/FFI risks remain open and are not covered by the successful local
regressions. A handled partial Context installation failure removes future
scripts but can leave the failed function in current documents. Calls to that
stale function can remain pending, and a retry with different Handle mode can
reuse the old browser-side controller entry. A correct rollback must remove
or reject only the failed registration generation in every affected execution
context, without racing a retry. Separately, binding acknowledgement/delivery
uses the shared 32-command FFI admission lane and its 64 MiB byte budget: a
settlement can be rejected before its Rust delivery guard exists, leaving the
pending entry until Page retirement. Ordinary evaluations reserve 8 MiB each;
byte starvation can occur before command-count exhaustion. Acknowledgement
and delivery also use the same controlled native scheduler as ordinary
evaluation, so work awaiting callbacks can occupy the capacity needed to
unblock them. Merely relabeling the FFI lane or retrying in Python is not a
proven liveness fix. Python registration shielding also does not prove safety for a
raw native registration Future dropped mid-install or an ambiguous CDP reply.
No zero-leak, atomic-rollback, or complete compatibility claim is made.

The Python dependency proof now anchors Page bindings to the exact
Handle/Page-event closure, Context bindings to Page bindings, media to the base
closure, and Context forced colors to media. Standalone and combined release
checks plus focused host regressions and strict Clippy gates are wired into
CI. The exact anchored closure comparisons pass (108 base/media packages,
109 Page-event/Page-binding/Context-binding packages), as do all four new
addon release checks, combined host strict Clippy and combined/standalone
Page-binding addon no-deps strict Clippy. The same-profile host unit suite
passes 661 with 14 explicitly ignored cases; focused binding/selector gates
pass 27/27 and 42/42. The final Context-only enum cfg repair preserves the
Context/test code paths of that full suite. The broad dependency script was
not rerun in full; these are its newly added exact profile gates.

The opt-in Context offline owner has since gained a mutable Rust
`set_context_offline` path that updates live Page/OOPIF sessions and records
the state for future sessions. Thin Node and Python async/sync APIs now expose
the toggle; focused Rust, Node and Python local tests pass 7/7, 21/21 and 3/3,
respectively, with targeted release/type/boundary checks passing. These two
official Python cases have diagnostic paired execution above; the verified35
promotion records both as candidate failures and oracle passes. That outcome
predates the local repair and does not validate it.

The current topology does not initialize service-worker CDP sessions for this
toggle, whereas upstream Playwright updates those sessions separately. Thus
an allowed service worker's own fetch is not yet proven to obey Context offline
state. A read-only review also found partial-failure, ambiguous-CDP-reply,
cancellation and close/toggle risks. The bounded Rust owner now compensates
attempted sessions on failure, defers policy commit, serializes cleanup and
marks incomplete broadcasts for same-value reconciliation. Focused release
tests pass 18/18, including rollback failure, timeout-after-apply and dropped
future. Caller cancellation can still leave native sessions temporarily
inconsistent until another toggle reconciles them. No post-repair browser or
official test has validated this repair; the strict statuses above remain
evidence from the earlier source-bound paired execution.

## ABI110 Chromium BrowserContext multipage promotion (2026-10-10)

The third exact-source manifest
`packages/pwright-python/tests/official/browsercontext-lifecycle-multipage-chromium-manifest.json`
(SHA-256 `281319eac629f5c879edd11184c66296a628407816a949b007c7ad54ebe28a67`)
pins four more unchanged async Chromium public-API cases: Page lifecycle
event ordering, cookie/localStorage isolation across two Contexts, initialized
Page notification, and popup/parent Context identity. The source checker
selected exactly four expanded IDs; all were `not-run` in both previous
6,526-row ledgers. Candidate and independent oracle each passed 4/4 under the
same isolated dedicated-Chrome CDP and Tailscale fixture setup as below. The
tab list was empty after both runs.

Reports reside in `.work/official-conformance/browsercontext-multipage-abi110.Ss3TNh/`.
Candidate JUnit/provenance SHA-256 values are
`e147d59b82a11f4ced201a80717b1a9d78b7bdc47890410650dd97cefe4170a0` /
`73e775e099b441ec9b3d7267f1b17bd00741968c9b39974faaa4092a186d5d81`;
oracle values are
`adaf2ea12f4ed536b63c988dd7a23df636786d3d0efa0640d8e2504bc7b25adf` /
`bd41215d59a35be377b07e6e9e4298b692f3114c1147084c920ed9e4c9ca8a42`.
Both schema-5 sidecars verify source-bound against the same capture-time
tooling/harness archive
`.work/official-conformance/python-abi110-browsercontext-lifecycle-exact-archive.Bgg4kY/`.

Only these four public-API Chromium IDs changed from `not-run` to `passed`.
Candidate
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-browsercontext-lifecycle-multipage-chromium-v1.jsonl`
records 1,279 passed / 11 skipped / four failed / 5,232 not run, SHA-256
`f1b3290a9321004934c6219fb237a8307700b75e7a6a023752afa03fa6d5d1b4`.
Oracle
`.work/official-conformance/outcomes/playwright-python-oracle-abi110-browsercontext-lifecycle-multipage-chromium-v1.jsonl`
records 836 passed / ten skipped / four failed / 5,676 not run, SHA-256
`a5d269049c709e20ba1c56cf487ec08b0e9ac76b23120182d7f1a7cbe5b6ed9b`.
Both applicability audits retain 6,526 rows and classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The three Context manifests together add 11 paired candidate/oracle passes,
not complete BrowserContext or full-corpus compatibility.

## ABI110 Chromium BrowserContext lifecycle follow-up (2026-10-10)

The separate exact-source
`packages/pwright-python/tests/official/browsercontext-lifecycle-followup-chromium-manifest.json`
(SHA-256 `8483d7c56eff097a77c02262670551c0a1ae93911b02e40941030e686c3a5141`)
pins four additional unchanged async Chromium cases: closing a Context aborts
its pending page-event waiter; a new Page belongs to its new Context; default
viewport propagates to the Page; and device scale factor reaches the page.
The pinned v1.58.0 source checker selected exactly four expanded IDs, all
previously `not-run` in both strict ledgers.

Candidate and independent oracle each passed 4/4 under the same dedicated
Chrome CDP adapter, isolated-case timeout and Tailscale fixture rules as the
three-case lane below. The reports are under
`.work/official-conformance/browsercontext-followup-abi110.AKp7q6/`.
Candidate JUnit/provenance SHA-256 values are
`b9f3480b31b21e53a6d94e23441f1e9653d68717822d879bf350afda8ba28ff3` /
`467d6fa9923b4d20ba7ecbc255b420c23102bc60f3726e0efd7a882eef43d8b4`;
oracle values are
`1e0782140bbac3b18bda9b94579e99325ae26dd56e66c869efd39b83fd290edf` /
`44959ee31b61b6198479ce8d15305b336d0bcf02c90c450dcc12f6a4f31f0b16`.
All four schema-5 sidecars across both Context lanes verify against the same
exact capture-time tooling/harness archive
`.work/official-conformance/python-abi110-browsercontext-lifecycle-exact-archive.Bgg4kY/`.
The dedicated Chrome tab list was empty after both runs.

Only these four public-API IDs changed from `not-run` to `passed` in the new
6,526-row strict ledgers. Candidate
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-browsercontext-lifecycle-followup-chromium-v1.jsonl`
has 1,275 passed / 11 skipped / four failed / 5,236 not run, SHA-256
`a51ed018358f528437c7684b9769f7affd7d567c84cbf60859800ca7bc7288ce`.
Oracle
`.work/official-conformance/outcomes/playwright-python-oracle-abi110-browsercontext-lifecycle-followup-chromium-v1.jsonl`
has 832 passed / ten skipped / four failed / 5,680 not run, SHA-256
`bd339cdfd6b1db64377fe13c0e10fd525d0caa4da07ff00521824e54610e5d95`.
Both applicability audits retain 6,526 rows and classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
These first two Context manifests together added seven paired passes before
the multipage successor above; cross-engine compatibility remains unproven.

## ABI110 Chromium BrowserContext lifecycle promotion (2026-10-10)

`packages/pwright-python/tests/official/browsercontext-lifecycle-chromium-manifest.json`
(SHA-256 `ad583439cbfebb64f51efa915696dec53ed4fa0b17e70127b869bc4de35140a7`)
pins the unchanged v1.58.0 `tests/async/test_browsercontext.py` source plus
its async/top-level fixture and server sources. It selects three public API
functions: `test_close_should_work_for_empty_context`,
`test_pages_should_return_all_of_the_pages`, and
`test_pages_should_close_all_belonging_pages_once_closing_context`. With
`--browser chromium`, their three expanded IDs were `not-run` in **both**
pre-run 6,526-row strict ledgers. The source checker passed for the clean
pinned commit.

The manifest declares both `requiresBrowser` and `requiresFixtureServers`.
The upstream `browser` fixture launches through the existing dedicated-Chrome
CDP adapter; the autouse upstream server fixture still requires a verified
Tailscale IP and full MagicDNS hostname even for the empty-context case.
Oracle and candidate ran separately with a clean dedicated-CDP tab state,
`--manifest-cases --manifest-api async --browser chromium --case-timeout 30
--isolate-cases`, distinct result paths and the Tailscale fixture flags.
The unchanged three official cases passed 3/3 in each lane, testing
Context/Page identity and close behavior without changing test bodies. Both
schema-5 provenance sidecars verify source-bound against the capture-time
tooling/harness archive at
`.work/official-conformance/python-abi110-browsercontext-lifecycle-exact-archive.Bgg4kY/`.
The candidate JUnit/provenance SHA-256 pair is
`a480bba2ebc3f42df97933f78fc309a4a3a5146ca26e4576a4f8735299200656` /
`699574297ec99b7b3689b8cebafb79693e20f8ec680f43bcaf2b59a309f14722`;
the oracle pair is
`22c3f6814124ab0382c8ed3d728b6817acff2e9bada774ca91e8b344950cdff7` /
`7cb66d9778b4934bcf0b7b62ecaf936b4a163da42f7d57182eeaa442812ba7da`.
Reports reside under
`.work/official-conformance/browsercontext-lifecycle-abi110.runXaW/`.

Only these three public-API Chromium IDs changed from `not-run` to `passed`
when the reports were merged into new immutable 6,526-row ledgers. Candidate
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-browsercontext-lifecycle-chromium-v1.jsonl`
now records 1,271 passed / 11 skipped / four failed / 5,240 not run, SHA-256
`5dc187f795c70934492890f1b8081158631ca409b0730e109ba02f2b0eabc228`.
Oracle
`.work/official-conformance/outcomes/playwright-python-oracle-abi110-browsercontext-lifecycle-chromium-v1.jsonl`
records 828 passed / ten skipped / four failed / 5,684 not run, SHA-256
`e33c80e0ae686cc8cce0e3fccac4a2c326aea9a956fe8f39636347ad196fa892`.
Both applicability audits retain 6,526 rows and classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The dedicated Chrome tab list was empty before and after the run; this is a
bounded lifecycle result, not full BrowserContext or full-corpus parity.

## ABI110 all-engine executable-path browserless promotion (2026-10-10)

With the pinned official Playwright Python 1.58.0 Firefox revision 1509 and
WebKit revision 2248 executables installed in this Mac's browser cache, the
unchanged exact-source metadata manifest (SHA-256
`65005ab446c31b0871e114d29a9699173ffd5f0539b50936743e4695858a1274`)
ran separately for each engine against the same isolated candidate wheel and
independent oracle as the Chromium checkpoint below. Candidate and oracle
each passed 4/4 Firefox and 4/4 WebKit metadata cases, including async/sync
`BrowserType.executable_path`; the adjacent `name` rows were already passing.
All four schema-5 provenance sidecars bind the pinned `browsers.json` (SHA-256
`6b6e0538d6e46eef71d3e1a0dcb65f8f5ea7914033c8150529d8e207afb6030c`),
the engine revision, resolved executable and `exists: true`. Their
source-bound verification replays against the exact capture tooling/harness
archived at
`.work/official-conformance/python-abi110-ff-wk-registry-exact-archive.74AEJf/`.
Live-source verification is not claimed after subsequent tooling formatting.
The candidate's installed wheel package is verified from the retained
isolated venv, not by the source-package-only `--package-root` override.

The full JUnit/provenance reports are
`.work/official-conformance/outcomes/playwright-python-{candidate,oracle}-abi110-browser-registry-{firefox,webkit}-v1.{xml,provenance.json}`.
Their SHA-256 pairs are candidate Firefox
`62ef2d91af25b332da4394efc79061de832efb866baba0065fe6fba5d0d3a9de` /
`ecb05f60b241fdf517918eca21792e1f5a18652532c8afdb5d30e64d96d9b820`,
candidate WebKit
`afc3557a605f8242a2664f413a54dc83ed574aa35092e00eb6d096ad28a1ec1e` /
`896dcec568a99540c214f538c0c1ecf0c0c93921ce6f5b26ce415274bf3c2074`,
oracle Firefox
`46023167e9d2d470a10a593d66baa269de03cdbceab411fc1eab0fe5341d8356` /
`4c7a6f9ee9dbc1faed4754923ac6353bda6294f5c6c23e18e063cb93bddacd01`,
and oracle WebKit
`7592824c6d1a9d5408501988879f7396df19a7d16cfa93b1d95515a6c8489830` /
`0244d8abde96f5e05d7c68e3e58ed0cfd9234c0f2cdae01efeade37bbb3db1b2`.

Only the four previously `not-run` Firefox/WebKit async/sync path rows were
selected from these reports and merged into the prior Chromium-only strict
ledgers. The new immutable 6,526-row candidate ledger is
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-browser-registry-all-engines-v1.jsonl`:
1,268 passed / 11 skipped / four failed / 5,243 not run, SHA-256
`e6ab49adfd8a2f64b9d2736e43328917faab17437d2c373076f0ceccb5d14e22`.
The oracle ledger is
`.work/official-conformance/outcomes/playwright-python-oracle-abi110-browser-registry-all-engines-v1.jsonl`:
825 passed / ten skipped / four failed / 5,687 not run, SHA-256
`cabcfc78429d12e962fefd8df8ddc3e4c9e77ff842e551b02dc4c4e814be92a1`.
Both strict audits retain all 6,526 rows and classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`;
the diff changes exactly those four public-API path keys from `not-run` to
`passed`. Together with the two Chromium path keys below, all six official
`executable_path` metadata rows are now recorded passing in both lanes. The
oracle starts its Node driver, but no browser was launched. This proves path
metadata/existence for the installed cache, not successful Firefox/WebKit
launch, `PLAYWRIGHT_BROWSERS_PATH=0` root parity, or full-corpus compatibility.

## ABI110 Chromium executable-path browserless promotion (2026-10-10)

The unchanged pinned v1.58.0 browserless metadata manifest (SHA-256
`65005ab446c31b0871e114d29a9699173ffd5f0539b50936743e4695858a1274`)
selected async/sync Chromium `BrowserType.executable_path` and adjacent
`BrowserType.name` tests. A fresh isolated candidate wheel (SHA-256
`17618675c4d9fd777fd844c724a8e7bdc3200f399d016904c97e6a76faa0e2b6`)
and the independent oracle each passed all four unchanged official cases.
`abi110` labels this evidence checkpoint; the candidate native ABI remains
107. The pinned Chromium revision 1208 executable now exists in this Mac's
browser cache. Both schema-5 provenance sidecars bind the official
`browsers.json`, resolved executable path and `exists: true`, and verified
against the capture-time environment. Candidate full JUnit/provenance SHA-256 values
are `c195f1b092fb4e254fac8952d03ee90e5d15d0aa64db7a7f94e63521e5b2f658` /
`4d5b0fc9ad6402caa4526cd0da4e0e44c9a794e2bd7f885da655e615db404213`;
oracle values are `3747819c61afae40d375fa326ec5b3548bf2677662ef945b0f231d0c4ba29cb1` /
`f41bcb76d86dd46293b17b7112fa8cae87ce2fa79eeb41451104cbdd0c228249`.
The reports are
`.work/official-conformance/outcomes/playwright-python-{candidate,oracle}-abi110-browser-registry-chromium-v2.xml`
with same-stem `.provenance.json` sidecars. The oracle starts its Node driver,
but these metadata tests do not launch Chromium; executable existence is not
positive browser-launch parity.

Only the two previously `not-run` Chromium async/sync `executable_path` rows
were selected from those four-case reports for strict promotion; the two
`name` rows were already passing and their evidence was not overwritten. The
new immutable 6,526-row candidate ledger is
`.work/official-conformance/outcomes/playwright-python-candidate-abi110-browser-registry-executable-path-chromium-v2.jsonl`:
1,264 passed / 11 skipped / four failed / 5,247 not run, SHA-256
`97693577b5a02e2c2358372fcef6db9b575af58e3fb67492ee01f83a50b184a2`.
The corresponding oracle ledger is
`.work/official-conformance/outcomes/playwright-python-oracle-abi110-browser-registry-executable-path-chromium-v2.jsonl`:
821 passed / ten skipped / four failed / 5,691 not run, SHA-256
`71bf7132c80a286de32064b461f27228e14653fc6fa105c3b2291f9f357e0108`.
Each strict diff changes exactly those two public-API keys from `not-run` to
`passed`; both 6,526-row applicability audits retain classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
At this Chromium-only checkpoint, the four Firefox/WebKit async/sync
executable-path rows were still `not-run`; the all-engine successor above
promotes them. `PLAYWRIGHT_BROWSERS_PATH=0` exact-root parity remains unproven.

The browser-registry provenance probe now locates the pinned installed
Playwright wheel via isolated Python and its recorded wheel files rather than
accepting an upstream-checkout `playwright.egg-info` shadow. The focused
provenance/tool regression set passes 59/59. This verifier repair changes no
official test outcome on its own.

The exact capture tooling and shim/runtime trees were also copied to
`.work/official-conformance/python-abi110-browser-registry-exact-archive.Le9Fym/`.
Both v2 sidecars verify with that archived tooling and harness; the candidate
still verifies its installed wheel package from the retained isolated venv.
The current verifier's `--package-root` override accepts source-package
layouts only, so the copied wheel package is not claimed as a portable
archived-package verification.

## Four shared fixture-host failures: separate evidence check (2026-10-10)

`packages/pwright-python/tests/official/python-fixture-environment-blockers.json`
(SHA-256 `7d1bf9f68d6d721a40d0fc41d363db9dd96485fe0ce346dbf94b8799086d0c9f`)
pins the four current public-API failures: two async add-cookie tests and the
async/sync IndexedDB storage-state tests. The read-only
`scripts/check_playwright_python_environment_blockers.py` checks exact upstream
source/manifest, both 6,526-row strict ledgers and summaries, historical
JUnit/provenance digests, matching failed IDs in both lanes, and the observed
MagicDNS-versus-literal-`localhost` assertion signatures. It starts no server
or browser. The exact check currently reports four shared environment blockers
and **zero conformance-pass credit**. All four raw outcomes remain `failed` and
`public-api-source`; the checker does not alter the applicability audit or
upgrade schema-1 historical sidecars to newer provenance claims. Their
unchanged upstream assertions cannot pass when the required fixture URL uses
MagicDNS instead of `localhost`. A separate explicitly labeled adapted-test
diagnostic or a user-authorized fixture-policy exception would be required to
test those assumptions differently; neither is an unchanged-official pass.

## Guarded dummy-Node launch diagnostic (2026-10-10)

The unchanged pinned v1.58.0 Chromium async and sync
`test_browser_type_launch_should_reject_if_launched_browser_fails_immediately`
passed in both oracle and candidate: four selected executions, each 1/1.
The separate exact-source manifest SHA-256 is
`7f191e7c75bf0decd1a19f065f54a96bff5d5dd57bd43db6513fe242bae9c50b`.
The macOS process-exec allowlist permits the pinned 38-byte
`dummy_bad_browser_executable.js`, its Node interpreter and required harness
executables; the runner denies outbound networking and records the public
launch call/failure plus the
dummy Node's observed exit code 1. It forbids real browser executables and
CDP; no browser was started. Firefox/WebKit are excluded because candidate
`UnsupportedError` would not prove launch-process semantics.

The four JUnit / provenance SHA-256 pairs, in oracle async, candidate async,
oracle sync, candidate sync order, are
`234f5a21dbe75980c94f50655e9a64a225c6ac8ecbe1fdb95f589da5405c7615` /
`7dccb59e21768e06bb22e21413707632d0e655764643398ee054d5e8388eada6`,
`efa24d60dbd7ee001d19c28d684ba49c4b5a66f1040cce66423ad78282a07973` /
`6de04d9f9d3db2ff68bdf58b5553ae7b1ac89dae2a695d5c1fee319eef3d6a2d`,
`6cb1fc73ac8cdd8e314feecf96edf3ff179744a75aa0d9f1cd354423cca4e6aa` /
`cd3f1fd7ed60318f74e956a0b1f49a2f4e8f62d15efd04689eede2620ec47ad6`,
and `3542f36111e567fa345e63474589ff8da9cf1828ee0fee708f61da709ffc1042` /
`64106ebe631ba0b00c724a57da17a55a8466ede2a65ca1c1a98af14e10d17162`.
They live under `.work/official-conformance/results/` with the
`playwright-python-{oracle,candidate}-abi107-dummy-node-sandbox-{async,sync}`
stems. The first oracle async attempt was a fixture-import setup error caused
by an incorrect `PYTHONPATH`; its retained v1 JUnit SHA-256 is
`ccd6ff26dcb4475d6e202712c674c27b6f391eee40b04f26b5642fdb74a91b65`,
with no launch receipts or sidecar, and earns no credit. All four green
sidecars explicitly say `diagnostic-only-no-general-conformance-credit`:
neither strict ledger was changed, and this does not establish successful
browser launch, broader process handling, or full-suite compatibility.

## Exact browserless proxy type preflight (2026-10-10)

The unchanged pinned v1.58.0 async
`tests/async/test_proxy.py::test_should_throw_for_bad_server_value` now passes
3/3 for both the official oracle and Pwright candidate in `owned` mode across
Chromium, Firefox and WebKit. Its exact-source browserless manifest has SHA-256
`7e748448269ecbc69e9c82cf9cbf3d9b1f5c5f514e780e15194e3f495a56c43a`.
The `browser_factory` fixture is lazy; the official Node protocol checks the
numeric `proxy.server` before dispatching a browser launch. The runner pins
the selected test AST, transitive fixture sources, oracle validator/dispatcher
package and candidate façade, forbids CDP and extra pytest scope, scrubs
inherited injection options, and rechecks runtime sources after execution.
The upstream autouse HTTP/HTTPS fixtures bind only to this Mac's Tailscale IP;
the oracle starts its Node driver, but no browser/CDP process was launched.
Before/after process snapshots retained only the existing dedicated Chrome
PID 23384 and its same helper PIDs.

The pre-fix candidate failed all three cases at its unsupported provider or
option gate; that red JUnit and schema-5 provenance remain verifiable against
`.work/official-conformance/python-abi107-proxy-bad-server-red-exact-archive.uwR3KL/`.
The correction adds only a public protocol-shape check for a non-string
`proxy.server` before those gates; **valid proxy launch remains unsupported**,
as do positive Firefox/WebKit launches. The new candidate and oracle schema-5
sidecars verify both live and against the exact post-fix archive at
`.work/official-conformance/python-abi107-proxy-bad-server-green-exact-archive.3SyOGW/`.
Candidate JUnit/provenance SHA-256 values are
`7d7b96bf6eed66fbcdb39be5892bfe14d79a6f4ee174c581f420d4684c7226af` /
`1d30775e9f3daebae767532003cf39ecdcf8013512a61e2bbcaf2b722598a53f`;
oracle values are
`61f989777cb775f5275be2402907233bfb2a23bcd896e10775859080fe2e51fc` /
`9a51dcc412eaec4a6cbdba9ace7fcb881184aaa18e5d17590ddd7d0eff5c4d43`.
Python browserless units pass 330/330; focused source/runner tests pass 47/47,
with Ruff and strict Pyright green.

The immutable 6,526-row strict successors change only these three public-API
`not-run` keys to `passed`: candidate 1,262 passed / 11 skipped / four failed /
5,249 not run, SHA-256
`9abb589487cadbb01d379876e6fdb4bd08d093bcaf10c246e4ab0845946841e8`;
oracle 819 passed / ten skipped / four failed / 5,693 not run, SHA-256
`e396771db5ad1b646ac88214af3f5b05291e543a3459200c95bf2d92769d9523`.
Both complete invocation audits retain classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
This is pre-spawn error parity, not proof that a valid proxy or browser can
start.

## Owned-launch boundary checkpoint (2026-10-10)

The official runner now accepts `--launch-mode=owned` as a separate,
fail-closed lane; its default remains `cdp-adapter` for historical reports.
Owned mode removes any inherited CDP endpoint, disables both the candidate
and independent oracle `BrowserType.launch -> connect_over_cdp` patches, and
requires real public async and sync `BrowserType.launch` before running pytest.
The public Chromium launch path now exists through opt-in Rust-backed Python
ABI107; Firefox/WebKit positive launch remains unsupported. New captures use
provenance schema 4 with `launchMode`, a matching JUnit suite property, and an
isolated-run `launch-mode.json` receipt; owned resume rejects unlabeled or
mismatched cases. The runner/provenance browserless regression set passes
74/74 with pinned pytest 8.4.1. Candidate and oracle each collected all 12
expanded preflight rows under a stable interpreter, with verified schema-4
sidecars; collection-only JUnit has zero executed tests. In genuine `owned`
mode, the unchanged official async/sync Chromium invalid-page-argument and
nonexistent-executable cases passed 4/4 for both candidate and oracle. Both
execution sidecars verified. No Chrome process was started by those error-path
tests. Six historical ABI106 schema-2 metadata sidecars were reverified using
the exact archived capture tooling plus archived pre-schema-4 shim/runtime.
Candidate reports also require the archived `shim/playwright` package tree,
because it is the captured `playwrightPackage`, not merely a harness input.
Schema-2 reports remain historical CDP-adapter evidence, not owned-launch
proof.

The first four paired-green Chromium preflight keys were promoted
from `not-run` to `passed` in each 6,526-row strict ledger. At that checkpoint candidate had
1,250 passed / eight skipped / four failed / 5,264 not run, SHA-256
`e47bb3fb7b82985ce3b6d956b6ac57b4e2363bfefc3deca2fe0de7bb57fc6917`;
oracle had 806 passed / seven skipped / four failed / 5,709 not run,
SHA-256 `3c6a1fe198573c937757574e9c4337155cd00a4cc4d2180bcee65ac714803ff2`.
The candidate JUnit/provenance digests are
`b1ed30fc5c66c6bcf29079b280d660d7e7142618700584f78098fe002d20693b` /
`3ef57255b5f882767519a3f9fef025c7f1f66f4b1698c29bd4c25e8aa7dcf618`;
the oracle digests are
`85f946076882b662aea27cabd37bafccd2f147b5e05e1a2dc756363f83ec9571` /
`9ff13f4200854a59969f8c70d8a5a6e9e154522f65ce8bd173db6bf00061f31a`.
The manifest SHA-256 is
`edc46b5f30f159ed5619bcb8e82f97caccecb10ca17e29db860fd45b57261279`.
These are launch *rejection* checks, not proof that a successful owned browser
can start, use Pages or shut down cleanly.

The remaining eight pinned Firefox/WebKit async/sync preflight rows then ran
in separate one-engine `owned` invocations without spawning a browser:
candidate and oracle each recorded six passes and two upstream Firefox skips.
Four schema-4 source-bound sidecars reverify. Each strict 6,526-row successor
ledger changes exactly eight more keys from `not-run`: candidate 1,256 passed /
10 skipped / four failed / 5,256 not run (SHA-256
`67e9d92c0ffb5a020bf9fa207cdac90871fc32789f2854cc3cef8f22cc8d4969`),
oracle 812 passed / nine skipped / four failed / 5,701 not run (SHA-256
`1c7fba90cd6c29e030bf5bd5e63248bf7e8504c5b212bc61fdd79d1b56667686`).
The filename label `abi108` denotes this ledger checkpoint; the installed
candidate native ABI remains 107. An initial combined Firefox/WebKit pytest
run encountered an async-after-sync fixture-order error; that report is
diagnostic only and was not promoted. None of these preflight outcomes proves
Firefox/WebKit positive launch support.

An additional unchanged pinned browserless test,
`tests/async/test_page_route.py::test_should_work_with_glob`, passed 3/3 in
both candidate and oracle across Chromium, Firefox and WebKit, with six
verified source-bound schema-4 sidecars and no browser process. Its manifest
SHA-256 is
`70c6c6cad3d032e4e03136d4d5e3c2c7029e0e80aa8144a8dff21858f6620af3`.
The ABI109 successor ledgers changed only two candidate and three oracle
`not-run` keys: candidate 1,258 passed / 10 skipped / four failed /
5,254 not run, SHA-256
`865700ea62525460b5f64e267c43cea91a4e527e33a4f14dff3514bc05edfa95`;
oracle 815 passed / nine skipped / four failed / 5,698 not run, SHA-256
`1ee5d32fe559121c2d07aa051bec77a00fe6a78fa6fc4e31e0455d2d6dc70f18`.
The formal applicability audit classifies all three rows
`implementation-internal`/`non-applicable`, because the upstream test imports
private helpers. This is whole-corpus execution, not public API parity.

A separate exact-source two-case manifest now selects pinned official async
and sync `test_browser_close_should_be_callable_twice` (SHA-256
`b6f8c788e55b88c0aec8e8ae02ca901e51973f388993f6af6650031dff2ff854`).
With `--browser=chromium --browser-channel=chrome`, the candidate `owned` lane
launched installed Chrome through Pwright using temporary profiles, then
passed both unchanged official close-twice tests. Its schema-4 sidecar
verified; JUnit/provenance SHA-256 values are
`80a1d7293dfc9a65a7c4dafbd51cca0e75593639566d3e4ef85c856501c1a089` /
`e1021c7fdece10d9162451684bfca72dc4c9ff08350948d1fe9d7ff17e9a0019`.
No matching temporary-profile Chrome process remained in the post-test
process snapshot. This is candidate-only diagnostic evidence: the genuine
official oracle launch conflicts with the standing Pwright-only browser
control rule, so no oracle was run and neither row was promoted to the strict
ledger. Source/JUnit provenance also does not yet bind the observed process
identity and profile path, so child-tree cleanup is not proven.

The adjacent pinned async/sync `test_browser_close_should_fire_close_event_for_all_contexts`
cases are frozen in a second two-case source-checked manifest (SHA-256
`2789e4e98f5cbde14f680c899bb157bb8e3f403bfdaca991cf0c9f2a50c6887c`).
A browserless unit audit found that fresh-owned `Browser.close()` suppressed
its lifecycle watcher before emitting Context `close`; the façade now emits
that event once after successful native shutdown, only for fresh-owned
browsers. Attached CDP and persistent paths retain their prior behavior.
Python package units pass 308/308, with Ruff and Pyright green. These official
close-event cases have not run, and no strict row is promoted. The exact
pre-repair ABI107 package/shim/runtime/tooling tree was archived at
`.work/official-conformance/owned-launch-abi107-pre-close-event/` with tree
hashes matching the earlier owned-mode provenance. The candidate-only
verifier `--package-root` option checks the exact recorded source location,
archived tree and original native-artifact digests without changing recorded
paths. Using that archive plus `--tooling-root`/`--harness-root`, both earlier
ABI107 candidate sidecars (four preflight passes and two close-twice passes)
verify again after the façade edit; verifier tests pass 35/35.
The Rust host now coordinates owned-browser shutdown through one retained
provider report: concurrent close callers await the same completion, and a
cancelled waiter does not abandon shutdown. Two browserless release tests
pass. The report is not yet exposed to Python/conformance provenance and
does not prove descendant reaping or profile deletion.

The final host guard source now passes an all-features Rust release check and
76/76 focused browserless release tests. A new ABI107 Python wheel was built
in a separate output directory without replacing the historical installed
extension, then installed into an isolated Python 3.12 venv. Import from
outside the source tree loaded that wheel's `_native.abi3.so` and reported
ABI107; the wheel's browserless Python unit suite passed 308/308. The wheel
is archived at
`.work/official-conformance/builds/python-abi107-host-guard-20261010/`
with SHA-256
`77b485d083121a08f0dea4fe1b9183c65ebda2b9a7e4ff3aa74f4d8726718c3a`.
At that packaging checkpoint the editable/source installation was unchanged.
Subsequently, its exact ABI109 `python/pwright` tree, shim/runtime and four
capture-tooling files were archived at
`.work/official-conformance/python-abi109-exact-archive.yO61kG/` and the
seven prior candidate sidecars bound to that package digest reverified
against the archive. The wheel's
ABI107 `_native.abi3.so` (SHA-256
`e3ebe18e4b1577ee18de03a72b5d9b72f320d46986e3f5e92a38316f6f28c465`)
and its two bundled OpenSSL dylibs were then installed into the editable
source tree. Its import path and ABI107 were checked and 308/308 browserless
units passed. The existing official strict ledger has not advanced. For new
captures, provenance schema 5 binds every adjacent
`shuozeli_pwright.dylibs` file by absolute path and SHA-256 under
`pwrightRuntimeLibraries`; the oracle records an empty list. Verification
checks exact directory membership and file content. A `--package-root`
archive must place the matching dylib directory beside the archived
`python/pwright`. Focused provenance/verifier tests pass 57/57. The pinned
browserless `test_should_work_with_glob` was rerun in candidate-owned ABI107
for Chromium, Firefox and WebKit: 3/3 passed, and all three schema-5 sidecars
verify against both current source and the exact archive at
`.work/official-conformance/python-abi107-schema5-glob-exact-archive.YE9OWt/`.
No browser/CDP launched; the fixture bound only to the Tailscale address.
The immutable 6,526-row successor candidate ledger refreshes only those
three already-passing keys: 1,258 passed / ten skipped / four failed /
5,254 not run, SHA-256
`778425cb062d329ef6ee0b3b24e62ce0f97696819abfe09944272dbaa1178639`.
The full audit has 6,526 rows, SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`;
all three glob invocations remain `implementation-internal`/`non-applicable`,
not public-API parity proof. Historical schema-1 through schema-4 sidecars
retain their original scope and do not gain retroactive dylib proof.

The existing pinned persistent-Context page-argument manifest (SHA-256
`4af8748fab33f52ce7ee71742b94265a6655a8d7b12688b5a3b90a42ce32dd86`)
then ran its two previously unrecorded async engine keys in separate `owned`
processes. WebKit oracle passed 1/1, while the pre-repair candidate failed
1/1 because its non-Chromium unsupported guard preceded page-argument
validation; Firefox was the same official skip in both lanes. A narrow
Python facade repair now checks WebKit arguments before that guard without
enabling valid WebKit launch or changing Chromium/Firefox launch behavior.
Python package units pass 312/312, Ruff and Pyright pass, and the unchanged
WebKit candidate official case now passes 1/1. All four final schema-5
sidecars verify; the fixed candidate sidecar also verifies against the exact
package/tooling archive at
`.work/official-conformance/python-abi107-webkit-preflight-exact-archive.rEWC4a/`,
while the old red report remains verifiable against its pre-fix archive.
The final candidate JUnit/provenance SHA-256 values are
`b70502924b0d1eb419d22f71d65480e31f7d1813bf0074bee2987c72c54a3465` /
`d761a5e50d6a43c0c1f9da570f5f7f76876d2a7eb41cdad5521723a123d16561`.
No browser/CDP process was started. The historical manifest lacks an explicit
browserless execution policy, so the no-spawn claim is grounded in its pinned
test/fixture flow and process observation, not a harness-certified guard.
The immutable 6,526-row successor ledgers change exactly those two `not-run`
keys: candidate 1,259 passed / 11 skipped / four failed / 5,252 not run,
SHA-256 `3bae07314d8d9b717a503e1b553d813faa5f11d01deec922cdf9d8d33ddf0c81`;
oracle 816 passed / ten skipped / four failed / 5,696 not run, SHA-256
`c622c0a3c4aa8ab9a0afdfbfa1aa89531b683ae50fc4e63f7fbeac38a54852b6`.
Both 6,526-row applicability audits pass at unchanged classification SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`;
the two keys are applicable public-API candidates, including the official
Firefox skip.

A new, separately pinned browserless manifest now certifies that same two-key
selection without changing the historical manifest or its evidence:
`packages/pwright-python/tests/official/persistent-context-page-argument-browserless-manifest.json`
(SHA-256 `3e59ca6e06feea4101e87840c5bfe1f73e72f2d326fa9a8d738198245e66b869`).
The runner permits only one async WebKit or Firefox `owned` case, checks exact
test AST/fixture and installed package hashes before and after execution,
rejects CDP/channel/secure-fixture variants, and removes inherited Node and
pytest injection options. WebKit again passes 1/1 in both lanes; Firefox is
the same upstream skip. All four new schema-5 sidecars verify against live
sources and the immutable
`.work/official-conformance/python-abi107-webkit-browserless-exact-archive.apuY9p/`.
The refreshed 6,526-row ledgers change only those two evidence references,
not statuses: candidate 1,259 passed / 11 skipped / four failed / 5,252 not
run, SHA-256 `24a36b73150a04012f1ca45911601758ef9ae61bad186b2b28af374995242888`;
oracle 816 passed / ten skipped / four failed / 5,696 not run, SHA-256
`ae78115993a256ca90af4c6da4389b924422ce37d6149eba965c034e27cb39c3`.
Both complete applicability audits remain SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The oracle starts its Node driver and may create temporary directories; the
source-bound guard keeps this selected path before browser process launch/CDP,
not before all subprocesses or file side effects. Focused guard/provenance
tests pass 81/81; two older profile tests need the pinned pytest 8.4.1 and
fail under this environment's pytest 8.4.2.

The historical/default Python `BrowserType.launch` test lanes are CDP-attach
adapter lanes, not owned-launch parity: both the candidate shim and independent
oracle monkeypatch `BrowserType.launch` to call `connect_over_cdp` before
ordinary official execution. The new opt-in `owned` lane disables those
adapters; positive launch comparison must also bind executable/revision,
process, profile and fixture provenance. Existing adapted reports remain
useful for Page and Context behavior, but cannot prove that either lane
launched a browser.

At the earlier ABI106 checkpoint, executable-path metadata existed but
`BrowserType.launch` did not exist in the public async/sync façade or native
signature. The existing opt-in Chromium `launch_persistent_context` returns a Context using
a caller-owned, marked profile. It is not a nonpersistent `launch` returning
a Browser. A current-source safety repair now rejects an empty string
`user_data_dir` before native code: previously `Path("").resolve()` selected
the working directory, while upstream uses a temporary profile for this
input. This deliberate fail-closed behavior is not upstream empty-profile
parity. Two new sync/async tests pass; the full Python unit suite passes
299/299 using current façade source with an installed ABI106 native module.
Ruff and strict Pyright pass. No browser was launched or official Python case
was promoted by this repair; all six earlier metadata diagnostic sidecars
remain evidence for their captured source snapshot.

The formerly proposed optional Chromium owned-launch capability is now wired
through public Node/Python surfaces with a unique temporary profile and
Rust-held process lease. Positive process lifetime, child-tree cleanup and
native default Context unaddressability still require proof. Successful
Firefox and WebKit launches require real engine providers; matching path
strings and preflight errors alone do not establish those capabilities. The
current 16-session native cap is not a limit on browser process memory.

## Cross-engine BrowserType metadata checkpoint (2026-10-10)

An isolated, opt-in Rust browser-registry leaf now exposes the pinned v1.58.0
Chromium/Firefox/WebKit executable-path metadata through Python ABI106. It
performs no browser download, launch, or filesystem existence check. A fresh
ABI106 wheel (pre-final-format diagnostic SHA-256
`fe1637d8ccc957f6ab12040b7ab1280109f215e3c6c20a51107ad739ba683688`)
imports at ABI106 and passes 297/297 package units; release feature and
dependency-boundary checks, Ruff, and strict Pyright pass. In six fresh,
per-engine source-bound schema-2 captures of the unchanged 12-row official
metadata manifest, the candidate and independent oracle each pass all six
async/sync `BrowserType.name` assertions and fail all six
`BrowserType.executable_path` assertions solely at the upstream
`os.path.exists` check. The two lanes return the same pinned path string for
each engine: Chromium revision 1208, Firefox 1509, WebKit 2248. None of those
exact binaries is installed in the capture environment. All six sidecars
independently verified; these six failed path rows remain diagnostic and are
**not** promoted into the strict 6,526-row ledger. The `=0` browser-path mode
has a known cross-package root difference between Pwright and the official
Node driver, so exact parity is not claimed for that mode. Schema 2 binds
capture-tool source, not the child browser-cache environment; that additional
binding is still required before a path result can be promoted.

The `PLAYWRIGHT_BROWSERS_PATH=0` limitation was checked again through both
public async APIs on macOS arm64 with pinned Playwright Python 1.58.0 and
Pwright ABI107, without downloading or launching a browser or attaching CDP.
For Chromium, Firefox and WebKit, both APIs returned the same relative
revision/executable suffix (`chromium-1208`, `firefox-1509`, `webkit-2248`),
but the official wheel rooted it at
`playwright/driver/package/.local-browsers` while Pwright rooted it at its
own `pwright/.local-browsers`. The installed official `registry/index.js` and
`browsers.json` have pinned SHA-256 values `15c1a6174cc9d631ffcc908ed8d040f3f5ee543a77711a8062fa3037f7cda75f`
and `6b6e0538d6e46eef71d3e1a0dcb65f8f5ea7914033c8150529d8e207afb6030c`.
The Oracle starts its Node driver to read the public property, but no browser
process is required. Pwright deliberately keeps the `=0` root within its own
package so installing the official Playwright wheel is not a prerequisite;
pointing at that wheel would break independently installable capabilities.
This diagnostic is **not** exact absolute-path parity, does not establish that
either executable exists, and does not promote any official test row.

The unchanged pinned async/sync `BrowserType.name` cases were run in fresh
per-engine processes for Firefox and WebKit, with the same exact-source and
fixture-hash-guarded browserless manifest used for the earlier Chromium pair
(SHA-256 `e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`).
The installed ABI105 candidate and independent oracle each pass all four new
rows; all four result sidecars verify. Exactly those four previously `not-run`
keys were promoted, preserving 6,526 rows: candidate 1,246 passed / eight
skipped / four failed / 5,268 not run (SHA-256
`6afce40303ca829b69e71caa3274ef60b028a7fba4225c19ad7cfc974fa905b6`);
oracle 802 passed / seven skipped / four failed / 5,713 not run (SHA-256
`62b38b62de425e89b4d88bb3e075984d24699cd933b59c3b7ca77ed0144a1759`).
The exact-source `browser-type-metadata-browserless-manifest.json` separately
selects 12 name/path rows (SHA-256
`65005ab446c31b0871e114d29a9699173ffd5f0539b50936743e4695858a1274`).
At that earlier ABI106 checkpoint, all six `executable_path` rows were
diagnostic and unpromoted: the oracle environment lacked the v1.58.0 browser
revisions its returned paths required. The earlier ABI105 candidate returned
an empty path; the later opt-in ABI106 candidate matched the official string
but still lacked the binaries at that checkpoint. A
string-only or fabricated path would not satisfy the unchanged upstream
`os.path.exists` assertion. Per-engine
execution avoids the mixed-engine pytest event-loop interaction observed in
the first combined diagnostic. The earlier ABI105 schema-1 sidecars are
`legacy-unbound` for local runner/helper source; the new ABI106 schema-2
diagnostics above bind those sources but not browser-cache environment or
installed revision state.

Two later wheels from the frozen production source hash to
`d9b7c15db53b3cf0e19e9f68b52fb3645174b1c025da7c0a4cbc505de71d2e16`
and `22e75ee9502558fe70bbe59b898b7254f5c967d5b985195e4a33c00a6c1cca90`.
Their native library bytes match each other and the diagnostic wheel (SHA-256
`806f05c4e140d6e9762136ba43ee7973f2369d4345d7237eee996b5bd4327515`);
all 16 wheel entries match apart from generated CycloneDX serial/timestamp
and its dependent `RECORD` hash. Normalizing those generated fields produces
an identical payload. The later wheel also passes the 297/297 package suite.
A root-clamping Rust regression test was added after these builds and passes,
but no production logic changed; an exact post-test-source rebuild was
deferred during another project's multi-GiB Rust compilation.

The subsequent pinned `executable_path` capture tooling opted into provenance schema 3:
the runner records the resolver-relevant child environment, mode-specific
package root and cache root, pinned official `browsers.json` digest/revisions,
and exact path/existence before and after each run. Any drift rejects the
capture. This is limited to exact manifest-selected path assertions; other
captures stay schema 2. The schema-3 implementation passes 66 browser-free
focused tests and Ruff. Its verifier intentionally compares with the current
environment and file existence, so changing cache variables or installing a
browser later can make an old schema-3 sidecar unverifiable; it is not a
portable archival attestation. The six ABI106 diagnostic reports above remain
schema 2, and all six were reverified with both current and archived tooling
at `.work/official-conformance/browser-registry-abi106.bhaA0F/tooling-schema2`.

Four official Python `BrowserType.launch` preflight test functions
(async/sync invalid page argument and missing executable, 12 expanded engine
rows) are frozen in `browser-type-launch-preflight-manifest.json` (SHA-256
`edc46b5f30f159ed5619bcb8e82f97caccecb10ca17e29db860fd45b57261279`),
and the pinned v1.58.0 source checker accepts all four selected functions.
The source checker forbids `.launch` only for manifests declared browserless;
this selection conservatively declares browser and fixture requirements.
Upstream preflight creates temporary directories and its autouse fixture
starts HTTP/HTTPS servers. The Chromium-only execution produced four paired
passes and a strict four-key promotion; the remaining eight Firefox/WebKit
selected rows were collected but not executed here. The Firefox page-argument
pair is upstream-skipped, but that skip has not been captured in this owned
lane. No cross-engine launch support is inferred.

## Dispatch-event selected-file checkpoint (2026-10-10)

The pinned Playwright Python v1.58.0
`tests/async/test_dispatch_event.py` SHA-256 is
`a4905396220267094de57d7fc721c8d733d4f0e453f37295882338c9ebe31c19`.
The exact 12 Chromium cases are in
`packages/pwright-python/tests/official/page-dispatch-event-manifest.json`
(SHA-256 `acbe71f2b9922e5040b17c2bcbcb3946b9096964f314de0f63ddc909353d0128`).
The upstream Git tree at `v1.58.0` has no
`tests/sync/test_dispatch_event.py`; the sync Locator dispatch test is in
`tests/sync/test_locators.py`, outside this manifest. The existing 6,526-row
profiled inventory contains 36 expanded async rows across Chromium, Firefox
and WebKit, but no rows from a nonexistent sync dispatch-event module.

Async and sync `Page.dispatch_event()`, `Locator.dispatch_event()` and
`ElementHandle.dispatch_event()` now use the Rust-owned dispatch paths. The
Page facade chooses the first match unless `strict=True`; nested live JSHandle
initializers take a separate owned-argument path, while plain values retain
the existing atomic Locator path. Stable-source ABI105 `abi3-py39` wheels
v2/v3 hash to
`229a2fd6c2bc024c3b06943d53538217d0c70168e0e01675b52f25302fd13165` /
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`.
Normalized payloads reproduce except generated CycloneDX
`serialNumber`/timestamp and the corresponding SBOM hash in `RECORD`. A fresh
v3 installation passes ABI105, exposes the Locator and ElementHandle owned
dispatch methods, and passes 287/287 units; Ruff, strict Pyright and release
addon check pass. The official pinned-source oracle and ABI105-wheel
candidate each pass all 12 selected Chromium cases. Oracle XML/provenance
SHA-256 is
`45356b260b29ee850e5d32b38d0db94c07aea8332ddb27bc99c94490a33bcb1f` /
`3e8cfaa4e381ea3b8d5febeab9b5d65852ad123ea2a839af1c85c9ca25c469fd`;
candidate is
`71c1be307ec418904fadb30bb93edff99200ea86cb144936aaed5a35da233b36` /
`b742d93ac50ab46c1c93a0190719f6563c737aff2cfeeadff7ea86efbe8ac83f`.
The exact 12-key async dispatch merge preserved 6,526 rows: candidate 1,238 passed /
eight skipped / four failed / 5,276 not run (SHA-256
`c5a31bf3be6d848b804a910a1de6409b09ded8fc8c361db8b1a12ad60c9039de`);
oracle 792 passed / seven skipped / four failed / 5,723 not run (SHA-256
`bcc3d23b593cc99fc1d753a6350e89f16b06dea3047afd495ee55fedffafcdef`).
This establishes selected-file outcome parity, not full Python corpus parity.
The separate Core HTTP fixture adapter collapses its cross-process URL to
the same origin; Core cross-origin semantics are therefore not validated by
its passing case. Do not generalize the Core fixture caveat to this Python
runtime, which uses a trailing-dot hostname to preserve a distinct origin.

## Browserless sync lifecycle, same-thread loop (2026-10-10)

The new exact manifest selects the six browser-parameterized rows from the
two unchanged pinned v1.58.0 `tests/sync/test_sync.py` lifecycle functions.
Manifest SHA-256 is
`dbad4ede06a7e3ae07d994a75e02a8868bc82191145e6c3746a880eb2fa2081f`;
upstream source SHA-256 is
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
The fail-closed execution mode wraps pytest in an asyncio loop on the same
thread; wrapper SHA-256 is
`7a089d2a16cbeec46926374571e96cf72cd8d3a513c76b0281c988be470e63c7`.
The candidate used the ABI105-v3 wheel SHA-256
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`
and native artifact SHA-256
`c92884f312695621be09dc373f10285a3df80b73ab0196ed6502d967c609564d`.
Candidate and independent oracle each collect six and pass 6/6. Verified
candidate execution JUnit/provenance SHA-256 is
`6b08f1df918824cc60e95d9231ba4c6c84f2b0eb5bd81639e9890e88d4c85880` /
`085841dbbf0993c31a4b4c9942770e243115d03ecbe975b6c3b6cc46503c2e1d`;
oracle is
`c1f3a1c464d78c225cca5cd10568017d1b6c0a27d9d10cfc7dd0810fbfddb2cd` /
`0c191f41c487998880d90608060c5fd3c8fc68f5e84612dab45e6cd956a01326`.
Candidate collection JUnit/provenance SHA-256 is
`389703deccc021f703747a8c92fbc421314031f010f1a6848cf100e1c567f314` /
`e9c881d7f7ceafe10139fa1987761bcdf23f4c0e383922fdad3461d417e95a72`;
oracle collection is
`28acb4cc904cb2d2f5dc9acfd9d25f820a997d6a68a78f907e776105f7f0e696` /
`69d988747790349befea61930882b3f58e1c393a9cd76d7b128df7a006a29449`.
The earlier ordinary-pytest candidate/oracle reports remain three passes and
three `DID NOT RAISE Error` failures each; this alternate loop topology does
not overwrite them. After independent sidecar verification, the exact six
loop-mode keys, and no others, were merged into new 6,526-row strict ledgers:
candidate 1,240 passed / eight skipped / four failed / 5,274 not run
(SHA-256 `e7cd534dbd40309ed7d258431face26a1b40d0f2c14c27834b493ec3fff5695e`);
oracle 796 passed / seven skipped / four failed / 5,719 not run
(SHA-256 `07cf927aa0a241b2075d9ec0f26c88e58daff0bf50e3d6cd646588ba056fc1d8`).
No browser/CDP execution occurred for this selection.
Fresh official Python/Core runs now reject occupied result or provenance-
sidecar destinations before process launch (Python's explicit isolated-resume
mode remains separate). Historical provenance sidecars do not bind the
runner/helper source hashes, so the new guard is not retroactive evidence
that earlier reports were freshly executed. A separate browserless Core
BrowserType two-row slice advances the 48,477-row strict ledgers to candidate
1,580 expected / two skipped / eight unexpected / 46,887 not run (SHA-256
`556eafa00527dbf41cd7d494dcb2f2a9423272581af614ab6ea1a7f09f93faa7`)
and oracle 1,284 expected / one skipped / six unexpected / 47,186 not run
(SHA-256 `1b08b44e3351da8442652333694aef39a0ac7ab07ff866dea7d3e5b5f10fdb3c`).
The public `browserType.name` row is expected in both lanes; invalid-channel
`launchServer` is unexpected only in the candidate. Its secure Core manifest
now pins exactly 17 unique spec IDs (SHA-256
`efc2f9f131c245baf7ddca262cd08ca3b5cee3d9f8d3d7a4248cbe55a5a03de2`),
but has no official browser result while shared CDP Chrome has a foreign tab.

## Browserless BrowserType.name async/sync pair (2026-10-10)

The fixed manifest selects only the unchanged pinned async/sync
`tests/*/test_launcher.py::test_browser_type_name_should_work` rows (SHA-256
`e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`).
The source checker allows the exact pair's `browser_type` and browser-kind
fixture arguments only while both the source files and fixture definitions
match their pinned hashes; all other fixture-argument browserless cases stay
denied. Installed ABI105-v3 candidate and independent oracle pass 2/2.
Candidate XML/provenance SHA-256 is
`de0479b59b4650342c82006b13406470241390cc25ebc34cab6665e0d1d142f2` /
`0703d2ff4b7ec441f1eec22553a10ac1de1dd5e43cd8fd8edf0a92bd8159a9e2`;
oracle `9ed208f5c431d5b689e11eecdf7516695e961db965d1ae1545920ee4a6047611` /
`3bb3b3a9cfd6cd02f4c91b7f6a754e6ba94682c98777511dbdfde79df37df21d`.
Both sidecars independently verify. Tailscale-bound HTTP/HTTPS fixture servers
ran, but no Chrome/CDP. The exact two-key merge advances the 6,526-row
candidate ledger to 1,242 passed / eight skipped / four failed / 5,272 not
run (SHA-256 `ed31f1074ab675e75b8828e27a141a858144e8f85e408b9955d36fd1f2c6e4e8`)
and oracle to 798 passed / seven skipped / four failed / 5,717 not run
(SHA-256 `2c02dffe0d102fe0d2dd660b30318097e455018c074140f73db5707067511852`).

## Page check and uncheck checkpoint (2026-10-10)

The fixed `packages/pwright-python/tests/official/page-check-manifest.json`
(SHA-256
`7e9b34f4ed0b0c193e213375ad2fc8dad6fbd13b622cda3f77c5d2ecf51809ae`)
pins all 18 unchanged Playwright Python v1.58.0 Chromium async/sync
`test_check.py` cases. The preserved candidate v1 failed 18/18 at missing
`Page.check()` or `Page.uncheck()`; the independent oracle passed 18/18.
Candidate v1 report/provenance SHA-256 values are
`e7bd53be1ebb5598ad2c32b81b48815ed45938c109d457436f8b76fcae46774c`
and
`c7f5a90298b02b16bed002db9765289a53a41f7d0542d686ec3cdbaf37b4028d`;
oracle values are
`ed02b29dc95adc83c2f08d3584fa8b4a041dbfd2b0385061bca8504e10ece72e`
and
`f12a1290f16071659a873fabe7937162e41d26c386802569c5be3d593f74c0ac`.
Both baseline provenance sidecars verified. The strict red candidate merge
retained 6,526 rows: 1,208 passed / eight skipped / 22 failed / 5,288 not
run, SHA-256 `7181ca1d518c24c69f9db6a0c9a85d85fd6384feca748139b9287216b9790e70`.

The oracle ledger was first rebased from the older unprofiled inventory by
adding exactly 25 pristine TodoMVC example IDs, without changing recorded
outcomes. That profiled 6,526-row rebase has SHA-256
`a937e7f5400ed2a0a565673893f2c132aaf7f9be29ff0916078a6d1740bdb045`.
Merging the 18 oracle passes yields 780 passed / seven skipped / four failed /
5,735 not run, SHA-256
`1bc6a8b0dbaa1a79471e60d4c7576e0512043c737e2f70dea80db56fe13cea47`.

Thin async/sync `Page.check()` and `Page.uncheck()` facades now delegate to
the existing Locator checked-state owner. The repaired `abi3-py39` wheel
(SHA-256
`85913b81bcc9d9c65533554b4b6f7f9ca423216ea08f43d682bc1244b2d5d46b`)
passes 271/271 installed package units, Ruff and strict Pyright. Its official
candidate v2 rerun passes all 18 selected cases, matching the oracle. The
candidate report SHA-256 is
`2bc0177b7a6cc8f1ca7c1d4a6e20acfd728dbf819224ee88e69a7f07ad633e14`,
with verified provenance sidecar
`8c374ce922b748f613d82980ba3aa6be3eb68cdcde76071d19af7e36c8a44573`.
The strict 18-key merge makes the current candidate ledger 1,226 passed /
eight skipped / four failed / 5,288 not run, SHA-256
`82c8161800ec3acd18938d0f9d0ba80edf76ce66b0918d563a6923b93ef6ee74`.
This completes the selected 18-case Python comparison, not the full Python
or Core corpus.

## BrowserContext add-cookies paired follow-up (2026-10-10)

The existing source-pinned
`packages/pwright-python/tests/official/browsercontext-add-cookies-manifest.json`
(SHA-256
`7c2cdf04985d4c7b869dc5ad79557a8383bcd499f2946388d4fa83f9a4109ca0`)
selects all 19 unchanged async Chromium cases. A fresh candidate run on the
current repaired wheel and an independent oracle run each record 17 passed /
two failed. The same two failures assert a hardcoded `localhost` path/domain
against the required MagicDNS fixture. This re-executes the historical
NODE-350 17/2 result; it is not a new 19/19 pass or a code repair.

Candidate/oracle report SHA-256 values are
`8c2d0cb78770560ef5b67c41b33a06ea1e2f4b7ddec6e6311e27f938309e86cc`
and
`5b859b0384ef1402b07d1abb943a8ff548b48f7857ae5115908a73f2da551fd4`;
verified provenance sidecars hash to
`cc56001104c23d193f8bdcc27507919a740225e261f667b8e4a705fc7c10cda9`
and
`896521026e5fec97c04f51b599446a80b5442b02a6a0b00d7c42a231a712883b`.
The strict 19-key merges retain 6,526 rows. At that checkpoint, the candidate
ledger was 1,208 passed / eight skipped / four failed / 5,306 not run, SHA-256
`1876c267051f872f5cfb424c6865204861aa76c6a8b98ece4a227f108719b318`;
the oracle ledger was 762 passed / seven skipped / four failed / 5,753 not
run, SHA-256 `c1d661e878a21d33d3fd1956d90dfc544749fea4c6787c01d8c7fe9b45e141ba`.
The storage-state totals below are the preceding checkpoint. Full Python and
Core execution remain open.

## BrowserContext storage-state checkpoint (2026-10-10)

The fixed
`packages/pwright-python/tests/official/browsercontext-storage-state-manifest.json`
pins all nine Chromium invocations in the unchanged Playwright Python v1.58.0
async (five) and sync (four) files, at upstream commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. Its SHA-256 is
`5c18d9019844547ae881d988a81bc3d1f515cc87bc6fccc1a773e9e8ea14cc12`.
The preserved candidate v1 fails all nine: seven reach the absent
`BrowserContext.storage_state()` and two reject the unsupported
`Browser.new_context(storage_state=...)` argument. The independent v1 oracle
passes seven. Its two IndexedDB assertions fail because upstream hardcodes a
`localhost` origin while the required Tailscale fixture reports its MagicDNS
origin.

The repaired `abi3-py39` wheel, SHA-256
`8f5e3234cf2d46cc9ff4c2cd9179031e59d75b53bd276c23744317d713493cb1`,
selects the opt-in Rust `context-storage-state` owner and thin async/sync
Python facades. The clean candidate v3 repeat and independent oracle v3 each
pass seven of the exact nine and fail only the same two hardcoded-origin
IndexedDB assertions. Candidate report SHA-256 is
`a43d954ff5a14ea64aa3a86de989d350f3aefe4059b7a5ca5eef82d366d2b5ad`,
with verified provenance sidecar
`1565dbc19591b3b757b96b8c0c2bb947d5bab28ab0cb34786e8a094f157178e6`.
Oracle report SHA-256 is
`ed29bcc8e528ef77aa621891aa60bb346fd90b7c9d5ab214f9bb1311ae5252b6`,
with verified provenance sidecar
`efd85286116c94b981f7185c87e5df36d6fe0df0f375f14582f06d92e58f18ab`.

Strict nine-key merges retain the full 6,526-row inventories. Candidate:
1,208 passed / eight skipped / two failed / 5,308 not run, ledger SHA-256
`d1ecfa2db3e440bae951c09c55e2da862f45f964579b7e69970d59ded510f5ed`.
Oracle: 745 passed / seven skipped / two failed / 5,772 not run, ledger SHA-256
`3161add7ea5243cefa071dfc2c9845ddf6e18a5dee43bc9dc7c450d7735f6a20`.
The first repaired-wheel v3 run timed out after a just-closed task Chrome tab;
its raw artifact is preserved at
`.work/official-conformance/playwright-python-candidate-browsercontext-storage-state-v3.xml`
(report SHA-256
`1e863732092719b145c1cfce082fa8c019d67d4b2061f388849f7107f300bd1b`),
and the cause is unproven. A one-case control and the clean full rerun used
the same wheel. This is selected-case outcome parity, not a 9/9 pass or
completion of the Python or Core corpora. Encoded IndexedDB values,
large-store browser-side limits and lifecycle stress remain open. Pending
storage-state Contexts with known IDs are now excluded exactly from public
visibility; before an ID is returned, a broad unidentified-creation guard
may also hide unrelated newly observed Contexts, so concurrent visibility
parity remains open for that interval.

## Sync BrowserContext event checkpoint (2026-10-06)

NODE-410 selects nine still-unrun Chromium rows from
`tests/sync/test_browsercontext_events.py` whose async counterparts already
passed. They cover Context console publication, popup publication, dialog
publication and WebError publication. The initial portable-wheel candidate
passes seven rows and fails only
`test_console_event_should_work_in_immediately_closed_popup` and
`test_console_event_should_work_in_popup_2`, both at the missing public
`BrowserContext.expect_page()` method. The independent Playwright 1.58.0
oracle passes all nine, establishing a candidate-only API-shape discrepancy.

The repair adds thin async/sync facades over the already installed bounded
Context `page` lifecycle waiter; it does not add a native call, queue or
retained object. Sync predicates and resolved values use the canonical Page
facade. Two AAA units raise the installed-wheel total to 252/252. Ruff and
strict Pyright pass, two auditwheel-repaired builds have normalized-identical
payloads, ABI104 retains 90 selected features, and the native artifact SHA-256
remains
`6e3aef33a88b1e056931d4bbde336c57403d95cb88dc1fabaed5469d19a85d1b`.

The repaired candidate and oracle both pass 9/9 in isolated processes. Their
shared case-ID SHA-256 is
`ba6df80c7d4bd8b2a5d10980281a76df4c4b3649b491bb8f7816e4fbe2419487`;
candidate/oracle report SHA-256 values are
`cc5cbb5c3fea0801bde4e4be6eb49aa777fc3970278a90ad8f2a2e492144ef2a`
and `a0cf4a49b4c33f9cce3fbacae743b06ea9e17478fd7c935af3cbc11dd3058984`.
Both provenance sidecars verify. A strict ledger diff changes exactly those
nine rows from `not-run` to `passed`, reaching 1,201 passed / eight skipped /
5,317 not run. The authoritative ledger and summary SHA-256 values are
`554cd9e63214f3b920489849cabadfe8da1559d0719705399c68f65bf3ddbaee`
and `f9170816984a2f0d7d40c2f70a662e9660a8bfaf5d2fbb00e621a39b51c59b6e`.

## Sync request-timing checkpoint (2026-10-06)

NODE-409 executes the four still-unrun Chromium rows in
`tests/sync/test_resource_timing.py`. The unchanged installed wheel passes all
four. The Playwright 1.58.0 oracle passes `test_should_work_for_ssl` and
`test_should_work_for_subresource`, but fails ordinary navigation and redirect
because Chrome 154 returns `domainLookupStart=0`; the pinned helper accepts
only `-1` or a strictly positive timing. Full diagnostic candidate/oracle
report SHA-256 values are
`e76315db5a762729d44de3efce24fcf1f119e36088d6ed0901319e418fd56bbc`
and `86d3974a712dcd891776b57ff5a99dacc778b9d87b51a43c3f6b44fa5038c4e8`,
with shared four-case ID SHA-256
`9e19887c26799bc8b6611790223f9d1248debe2e4f4cc47dfab4b64782553879`.

Only the two common passes are rerun as a clean promotion report. Their shared
case-ID SHA-256 is
`b3f6e6dae964d66c5ffdf12b1513e6d37c6973c69bdbbaee57fa1e4f4803f6e1`;
candidate/oracle report SHA-256 values are
`9638463eed022460b7286e2c41bbe4ac94c0f2790de81f453332e39c5be03b24`
and `b47b3294f44af7824015e9880d4a4d5d1be65eeab9f0ae2c2f7a3f999bc7c0aa`.
All four sidecars verify. A strict diff promotes exactly SSL and subresource,
bringing cumulative execution to 1,192 passed / eight skipped / 5,326 not run,
SHA-256
`79d512f3af52c78e8e8e0cf79feb41c77a1ca04cb2791538f0b453503132069e`.
The other two rows remain `not-run`; no assertion or timing implementation is
changed to manufacture oracle parity.

## Sync service-worker policy checkpoint (2026-10-06)

NODE-407 executes both still-unrun Chromium rows in unchanged
`tests/sync/test_browsercontext_service_worker_policy.py`. The runner replaces
the ordinary fixture with trusted TLS on the verified Tailscale hostname, so
service-worker registration has the secure context required by the platform.
The final NODE-403 wheel and independent Playwright 1.58.0 oracle each pass
default allow and explicit registration block 2/2. Their shared case-ID
SHA-256 is
`438c118112415b7d7c6ef2b5304035493eb61e0d6bbc16a06ba88b814f047fc0`;
candidate/oracle report SHA-256 values are
`9a9e21e364401bf7ceed7a9ced73ad87c8a43618496e8ca9a63ee317c1ddac18`
and `6c9afb0ba0eae0c23a3838920129dbfbe6436e78bd3ea02be9a364cb63cd5dfe`.
Both provenance sidecars verify and record `secureHttpFixture=true`.

A strict diff promotes exactly two `not-run` rows. Cumulative execution is now
1,190 passed / eight skipped / 5,328 not run, SHA-256
`94f83882339cd9af0b64200beea2098b8b7ba3ee551c2228b52bf19e1ae61013`.
The equivalent async Chromium file was already 2/2, so both public facade files
are complete. Earlier MagicDNS HTTP timeouts are retained only as the
diagnostic that established the secure-context requirement; they are not
outcome evidence. ABI104/90 and all implementation/resource boundaries remain
unchanged.

## Keyboard-plus and shared-environment checkpoint (2026-10-06)

NODE-406 executes the still-unrun Chromium
`tests/async/test_keyboard.py::test_should_press_plus` row from the unchanged
NODE-403 portable wheel. Candidate and Playwright 1.58.0 oracle each pass 1/1,
share case-ID SHA-256
`e01f427d8ca9a3bc3e7415eac485c0f27582d5f6641d927962e2054f38d365cd`,
and have verified provenance. Candidate/oracle report SHA-256 values are
`0c982237fedbda5cf2383b0f29e91e9a5efe70501365c6b769aadc5ce76510fa`
and `9d696c762ba7726965289b186a4bdc24fdab15eb53bca0c4aee2c7ddd4490e07`.
A strict diff promotes exactly that one row. Cumulative execution is now 1,188
passed / eight skipped / 5,330 not run, SHA-256
`672a6829fc84a4d55ec587ad23880de96a9cef3dd318719a00f47caae3fe7c5a`.

NODE-404 and NODE-405 retain two paired shared-red diagnostics. For
`test_dialog_event_should_work_in_popup_2`, Chrome 154 publishes a Page with an
empty URL before the `javascript:` prompt; candidate and oracle both therefore
disagree with the old `dialog.page is None` assertion. Their report SHA-256
values are
`d639905f5a620814aa7522c6bd37e1a28a7c2b3147cbd4b0c76b39d1473ba610`
and `92659ade74c40820917c7f01105a1fb4e25d68ce70bd1740d162d38cc763ea92`.
For `test_should_use_set_cookie_header_in_future_requests`, both lanes return
the mandated Tailscale fixture hostname as the cookie domain while the source
asserts `localhost`; report SHA-256 values are
`8ec825676b02a603eee9eabbdfd6067b19d21f67d968c086e713ce3a9ba1a9ad`
and `0c7fb9b5e3aa52043b2582db604be3bce2afbe34cfba08cb699d2b443a3ff418`.
All four sidecars verify. Neither shared failure is promoted or used to change
Pwright behavior.

## Sync Page and caller-stack checkpoint (2026-10-06)

NODE-403 runs the nine still-unrun Chromium rows in
`tests/sync/test_page.py`. The initial installed-wheel diagnostic passes seven
and fails only the two `Page.pause` cases. Those tests receive
`headless=True` from the upstream fixture, but the mandated dedicated Chrome is
visibly headed; Pwright correctly reports that headed Inspector support is not
installed. The diagnostic report SHA-256 is
`d4fdf55696004c033d13b26728f2519bb3bda270f66a65481b052caf35e71603`.
It remains diagnostic-only, and the two pause rows remain `not-run`.

The one deterministic discrepancy was sync error provenance: the navigation
error stack came only from the background loop. The bridge now attaches the
blocked caller thread while retaining the native/async suffix, only on an
error path. A fresh repaired wheel then passes a clean seven-row selection 7/7;
the independent Playwright 1.58.0 oracle passes the exact same 7/7. Their
shared case-ID SHA-256 is
`483df180239c145e831a2145491cdc52f606484fd910efd21b4c2bfda2a69698`,
candidate/oracle report SHA-256 values are
`3a22d05c8bdb2ec79e19211bd6fb577574fe7893c6d16f6b3d54efa7e7a6bf35`
and `b640c1136d468c6edd847723adf8f1c58ddb8b175ced6dc61f9707e59c66190b`,
and both provenance sidecars verify.

A strict ledger diff records exactly seven `not-run` to `passed` changes.
Cumulative execution is now 1,187 passed / eight skipped / 5,331 not run,
SHA-256
`036aed9d37c7e88dfbce0ffe4210e7a16e11b2f06b860a8200015a2e99257eda`.
ABI104/90 and the native artifact are unchanged. The primary/rebuild wheels
hash to
`ce59434415f403caa193ac686c5468271d902380df938587f7cffab57b270f5c`
and `e9745ab64a6d67ed2cabe1b522a6cd1f26644756ab76737bd4067daad898d2e7`
and have equal normalized payloads. The installed wheel passes 250/250 units;
all dependency, release, formatting, lint, strict-type and Python 3.9 gates
pass. Full Python execution remains incomplete.

## Sync request-fulfill parity checkpoint (2026-10-06)

NODE-402 executes the two still-unrun Chromium rows in unchanged
`tests/sync/test_request_fulfill.py`. The final NODE-401 portable-wheel
candidate and independent Playwright 1.58.0 oracle each pass 2/2 in isolated
processes under the same Tailscale-only fixture servers. Both provenance
sidecars verify and share case-ID SHA-256
`ba72fca2dff7a5abd0c999df4bc5198fb1a5c059341346a5bd466788342f0280`.
Candidate/oracle report SHA-256 values are
`b95188b98d62986ba046d643cf76d4429566b2627aac68b280c596dd89d3aa78`
and `637626c1e57deda856ccab3b3c000139934fba966c38c228e43f4e46949342dd`.

The first row fetches the original intercepted Request through the Page's API
request context and fulfills with the retained APIResponse. The second verifies
JSON status, headers, content type and decoded body. The previously promoted
override row makes the sync Chromium file 3/3; the equivalent async file is
also 3/3. A strict ledger diff records exactly two `not-run` to `passed`
changes. Cumulative execution is now 1,180 passed / eight skipped / 5,338 not
run, SHA-256
`1846062292632f04561166ad19d4e8265a624f3eb837aa969ec14ebf6db55d78`.
No implementation, ABI, feature or resource boundary changes.

## Complete TodoMVC execution checkpoint (2026-10-06)

NODE-401 executes all 25 Chromium rows in upstream's separately frozen
TodoMVC examples profile. The final candidate loads the auditwheel-repaired
ABI104/90 wheel from `site-packages`; the independent oracle loads official
Playwright 1.58.0. Both lanes run each case in an isolated process, pass 25/25,
share case-ID SHA-256
`455cee73934771c213f5f55da7a4aa56e6cb802f5a5f40a84a7a47a4744bc8a8`,
and have independently verified provenance. Candidate/oracle report SHA-256
values are
`19e27f3c63ecf13804581d9c079afc5393ca005560d7770d34db07be82b82967`
and `a8792fe4c0a1a19db6dcf3aaaf73e7cedb96c78edd70d14b9b0c2a4f59db8fa6`.

The first candidate diagnostic is excluded: all 25 rows reported teardown
errors because `pytest-playwright` calls inactive `context.tracing.stop()`, and
five routing rows also found that a nested arrow was misclassified as a whole
function. The repaired candidate supplies only the inactive tracing no-op and
fixes classification in Rust; trace production/output remains unsupported.
The first oracle diagnostic is also excluded: with no default Page, Chrome 154
rejects Playwright 1.58.0's `Browser.setDownloadBehavior` during attach before
any test body. One task-owned `about:blank` Page restores that known
precondition; it is closed after the final oracle run.

The strict ledger diff reports exactly 25 `not-run` to `passed` changes and no
other mutation. Cumulative execution is 1,178 passed / eight skipped / 5,340
not run, SHA-256
`bfb8e95031d10240bc110b16ba18a800711f0e5e4cc513cef4e8cfd0f14198bd`.
The fresh installed wheel passes 249/249 units, its two portable builds have
equal normalized payloads, and every dependency/release/format/lint/type/runtime
gate passes. Full Python execution remains incomplete.

## Typed string-expectation checkpoint (2026-10-06)

NODE-400 implements Playwright-compatible exact/regex value and class
expectations plus ordered mixed class arrays through one Rust-owned matcher.
The unchanged async and sync Chromium `to_have_class` and `to_have_value`
invocations pass 4/4 in the isolated candidate and 4/4 in the independent
Playwright 1.58.0 oracle. Both provenance sidecars verify. Candidate/oracle
report SHA-256 values are
`ab19a7424ab038b1a1757966419b14aca7b92f449eb3e51843d860e9a201b826`
and `5fb83a36c9b1fb5a6e67d8cb2669a9a021330a2b20def1044e2237c49baa7458`.

Only the four green candidate rows are promoted. The cumulative ledger reaches
1,153 passed / eight skipped / 5,365 not run with SHA-256
`ab288ffacb37a4632adcf1be1d70b6beb4a7dee82f26f8525a43a0f3d922e020`.
ABI104 retains 90 selected features. The candidate loads its portable wheel
from a fresh `site-packages` installation, and that install also passes 246/246
units; normalized rebuild
payloads match. The first candidate attempt is preserved as a setup diagnostic
because it exposed a capability-selection undercount before executing any test
body. Full Python execution remains incomplete.

## TodoMVC implementation checkpoint (2026-10-06)

NODE-399 implements the public APIs identified by the 25-row TodoMVC audit:
substring and regex text assertions, checked and empty states, exact values,
ordered exact class lists and `Page.go_back()`. History traversal is a separate
Rust capability selected as the wheel's ninetieth feature; Python contains no
history state, browser program or retry loop. Regex value and class-list
matchers remain explicit unsupported cases outside the TodoMVC requirement.

The isolated runner now expands a profile directory such as
`examples/todomvc` while rejecting `examples/todomvc-other`, with browser
parameter filtering covered by AAA. The installed wheel passes 246 units and
all strict non-browser gates. A first execution attempt stopped before any test
body because the new candidate environment lacked the runner-only
`pytest-timeout` plugin; that environment was repaired to match the existing
oracle's 2.4.0 instrumentation. Browser execution is still pending because an
unrelated task reopened a nonblank tab in the dedicated CDP profile. No result
from this checkpoint has been promoted into the outcome ledger.

## Profile-aware fixture-definition checkpoint (2026-10-06)

NODE-398 resolves the corrected NODE-397 closure to 95 content-addressed
definitions: 68 from the pinned upstream checkout, 26 from the two frozen
collection environments and one synthetic pytest `request` builtin. There are
zero unresolved definitions. Forty-eight definitions are function scoped and
47 session scoped. Every selected definition dependency is present in its
invocation closure, and all inventory, closure, applicability and outcome rows
join in exact canonical order.

All 25 TodoMVC invocations now resolve the `page` fixture supplied by
`pytest-playwright==0.7.2`; each has a 21-name closure and belongs to
`fixture-creates-page`. The complete runtime partition is 4,795 Page, 588
Context, 534 Browser, 321 BrowserType-only, 246 Playwright-runtime-only and 42
no-runtime-fixture rows. The 5,360 applicable `not-run` rows comprise 4,106
Page, 486 Browser, 453 Context, 292 BrowserType-only and 23 no-runtime-fixture
rows. These bands describe fixture effects, not browser calls in test bodies.

The binding and definition ledgers have SHA-256 values
`f34cc45de0bbb8452c7b7917b403d4c7e21c0604066cca4b6b744d9045ef0d24`
and `26dd34a136b9f76778317cc89784b83342e6a79a57a84e543cce58a3d7da4309`;
the summary SHA-256 is
`3a7daa387def3ac08613c4bd7a32ce48841bac703909034630ea63acddf8ce1a`.
The collector SHA-256 is
`c886315cc37d3fa2134fe19065f3a91e051b9a9d41ac5f6b967bd3386522a879`.
A second independent two-profile collection produced byte-identical ledgers.
Fifteen focused AAA tests and all strict Python gates pass. No fixture or test
body ran; outcomes, ABI and runtime capabilities are unchanged.

## Profile-aware pytest fixture-closure checkpoint (2026-10-06)

NODE-397 collects the two official CI profiles independently and joins them
back to NODE-396 canonical order. The 6,501 `tests/**` rows use the core oracle
and retain upstream's autouse `start_server`, lifecycle hooks, browser/platform
gates and asyncio event-loop policy. The 25 `examples/todomvc/**` rows use the
frozen examples oracle and include the plugin's Page/Context factories,
artifact lifecycle, URL verification and launch-option fixtures. They do not
inherit the core-only server or asyncio fixtures.

The 6,526-row closure ledger SHA-256 is
`3f317451434319c80dc643efc8f4be24ea097cdef2a0e0948eaa3a5c45e28903`;
its summary SHA-256 is
`276d7c07b98e16215b1edd1199771d6a49985cb3b806cf89eb4f13c2539aecfb`.
The collector SHA-256 is
`5f827d602b4f35fea08815035fb7e0f3796d9eed04fdef35842c18046e06e251`.
Seven focused AAA tests and all strict Python gates pass. Collection uses
`--collect-only`; no fixture or test body executes.

## Official CI-profile reconstruction checkpoint (2026-10-06)

NODE-396 corrects the assumption that the repository has one root-wide pytest
environment. The pinned upstream workflow runs `tests/**` as its core job and
`examples/todomvc/**` as a separate Python 3.10 job whose requirements install
`pytest-playwright`. The canonical inventory now records both profiles: 6,501
core rows with Python 3.12.14, Playwright 1.58.0 and pytest 8.4.1, plus 25
example rows with Python 3.10.21, Playwright 1.58.0, pytest 8.4.1 and the frozen
`pytest-playwright==0.7.2` release reconstruction. The generated hashed lock
has SHA-256
`28a04972055dcaedcc64fa5b23d7bf0b31544cbb2bd79fccae388636fa866594`.

Exactly 25 unsuffixed TodoMVC IDs from the obsolete root-only collection are
replaced by their official `[chromium]` IDs; the other 6,501 IDs and the total
remain unchanged. A fail-closed rebaser permits only those 25 pristine
`not-run`, zero-attempt rows to move. The canonical inventory, outcome and
applicability ledger SHA-256 values are
`3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41`,
`456fcb0f097261f71a24e4500308740684250a3c393cda5d3161ed52918c574b`
and `31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
Counts remain 1,149 passed / eight skipped / 5,369 not run and 6,514
applicable / 12 non-applicable. Recollection is byte-identical; four inventory
and five rebase AAA tests plus strict gates pass.

## Superseded root-only fixture-definition diagnostic (2026-10-06)

NODE-395 was the diagnostic that exposed the collection-profile error. The
following numbers are historical evidence from collecting the whole repository
in the core oracle. They must not be used as the current canonical topology;
NODE-396 through NODE-398 replace them without erasing the audit trail.

NODE-395 resolves every name in the NODE-394 closure to its complete pytest
definition chain. Each content-addressed definition records the source kind and
path, line, scope, base ID, dependencies, autouse and direct-parameter status,
and one explicit reviewed semantic role. Each of the 6,526 canonical invocation
rows records all visible definitions in resolution order and the selected
definition. Selected definition dependencies must be present in that row's
closure; the inventory, closure, applicability and current outcome ledgers must
all match in exact canonical order.

The registry contains 77 definitions: 68 from the pinned upstream checkout,
seven from the fixed oracle environment, one synthetic pytest `request`
builtin, and one explicitly unresolved definition. Forty-three definitions are
function scoped, 33 session scoped and the unresolved definition has unknown
scope. Twenty-six definitions are autouse. Across all invocation bindings,
107,625 resolution chains have depth one and 102 have depth two, preserving
fixture overrides instead of flattening them to names.

The unresolved definition is `page` for all 25 TodoMVC example invocations.
Those example modules collect without the plugin that supplies `page`, so they
would fail at fixture setup in the fixed oracle environment. They are kept as
applicable `not-run` rows in a dedicated `unresolved-fixture-definition` band;
they are neither silently skipped nor misreported as an automatically created
Page.

Fixture-induced runtime bands now partition the complete corpus and current
outcomes. The 5,360 applicable `not-run` rows comprise 4,081 whose fixtures
create a Page, 486 whose fixtures launch a Browser, 453 whose fixtures create a
Context, 292 that resolve only through BrowserType, 23 with no Playwright
runtime fixture, and the 25 unresolved example rows. All 246
`playwright-runtime-only` rows are already accounted for as 240 passes and six
skips. These bands describe fixture effects only: a test in a no-runtime or
BrowserType band can still launch a browser in its body, so source verification
remains mandatory before selecting a browserless lane.

The binding ledger, definition ledger and summary SHA-256 values are
`36f993858ee5a01c0a11f002f6183bf8e521419fc1f857d7cc95e4e33cfa5ae4`,
`28340e32bf3c1ccdb6bdf72a3c9b03783ebe218512b0bcf4ceb14c89c99f80ad`
and `ab313b9a6d8bd13ffd67c4155ab6c2e222b95ec45a141dbee16b1ea1aeae180d`.
The collector SHA-256 is
`ff980036e45c75b810ded14d60f89915912c6a905764fc0de9208650b4e890d8`.
A second independent collect-only run produced byte-identical binding and
definition ledgers. Fourteen focused AAA tests and all strict Python gates pass.
No fixture or test body ran; outcomes, ABI and runtime capabilities are
unchanged.

## Superseded root-only pytest fixture-closure diagnostic (2026-10-06)

NODE-394 is retained as the root-only diagnostic that led to the separate
official-profile reconstruction. Its 25 TodoMVC rows did not load the plugin
used by upstream's examples CI job. NODE-397 is the current closure evidence.

NODE-394 collects pytest's direct, initial and complete fixture-name views for
all 6,526 pinned invocations without executing a fixture or test body. The
collector runs only `pytest --collect-only` in the exact official-oracle
environment, verifies the clean pinned checkout and canonical inventory hash,
and preserves canonical invocation order. Every source-level
`pytestParameters` sequence from NODE-393 matches pytest's direct fixture names
exactly. Pytest's public `item.fixturenames` view also matches its collection
fixture-closure metadata for every row.

The result makes the global harness dependencies explicit. `event_loop_policy`
is in all 6,526 closures. For all 6,501 non-example invocations,
`start_server`, `after_each_hook`, `skip_by_browser`, `skip_by_platform`,
`browser_name` and `request` are present. `playwright` appears in 6,459
closures, `browser_type`, `launch_arguments` and `headless` in 6,213,
`browser` in 5,892, `context` in 5,358, `page` in 4,795,
`browser_factory` in 4,128, `server` in 4,080 and `context_factory` in 3,711.
Closure sizes range from three through 22. This independently proves that the
browserless NODE-392 selection still inherits upstream's autouse fixture-server
runtime.

The fixture ledger SHA-256 is
`fde11844e88c4cadadbac096c0490ebb682eea9e97855806db13fb6925f3f988`;
its summary SHA-256 is
`42c34b9584855113d75bfe7ad373c059081f2f10b310614019d741774c1d5260`.
The collector SHA-256 is
`828794a1493ecff475d0e35dd105ea11c80e7c7a49ea3d853288ef4c4a913c19`.
Six focused AAA tests and all strict Python gates pass. Fixture names and
dependency closure are complete; NODE-395 adds the separate definition, scope
and semantic-role layer while retaining the unresolved example fixture
explicitly. Outcomes, ABI and runtime capabilities are unchanged.

## Direct pytest-input inventory checkpoint (2026-10-06)

NODE-393 adds `pytestParameters` to every one of the 6,526 expanded invocation
audit rows and cross-tabulates each source-declared input against applicability
and the current outcome. This is deliberately not called a fixture closure:
parameterized values, direct fixtures and factory fixtures all appear as
function parameters, while transitive and autouse fixtures do not. NODE-394
collects that separate pytest closure without rewriting this source-level
evidence.

The audit finds 36 distinct direct inputs. The largest unfinished overlapping
queues are `page` at 4,113 `not-run`, `server` at 3,294, `context` at 708,
`browser` at 480 and `tmp_path` at 303. Twelve expanded invocations declare no
parameters: nine are applicable and three non-applicable; eight have passed
and four remain `not-run`. The 6,526-row audit ledger SHA-256 is
`3c58942c03e7a1aefe41d472258ea0ed07a3388626e645587bce82f9cb93c2e3`;
the outcome-bearing summary SHA-256 is
`6c0eb8a7e7de67eb4d80b387808b1b83273630370fd4c6d05efb5ab5e804508a`.
Twelve focused AAA tests, Ruff, Python 3.9 runtime-type checks and Pyright pass.
No outcome, ABI, package or capability changes.

## Browserless sync-lifecycle checkpoint (2026-10-06)

This section preserves the ordinary-pytest topology. The later same-thread
asyncio-loop 6/6 paired evidence is documented above, separately and without
replacing the original 3-pass/3-fail reports; only the formal loop-mode
outcomes were merged into the new strict ledgers.

NODE-392 adds a fail-closed execution policy for fixed official selections that
provably need no browser. The policy is accepted only with
`--manifest-cases`; every browserless case must declare
`playwright.lifecycle.browserless`, and the source verifier rejects declared
pytest inputs while following same-module helpers and rejecting browser launch,
CDP connection, Context/Page creation and navigation calls.

The upstream root `conftest.py` nevertheless installs `start_server` as a
session-wide autouse fixture. The runner therefore refuses every manifest that
sets `requiresFixtureServers` false. The authoritative rerun loads the fixture
runtime, binds both upstream servers only to this Mac's Tailscale IPv4 and
addresses them through the full MagicDNS hostname; it skips only the dedicated
Chrome preflight. The initial NODE-426/NODE-427 no-fixture evidence is preserved
as superseded diagnostic output and is not the canonical ledger source.

The corrected manifest and pinned source SHA-256 values are
`bbe0748cef7a2cc0bc6c96ed9aca1fb96c74535e752c17491d122cc2b4beb8e1`
and `f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
The candidate and oracle each pass all three browser-parameterized
`test_sync_playwright_stop_multiple_times` rows and fail all three
`test_sync_playwright_multiple_times` rows with `DID NOT RAISE Error`. The
latter assertion requires an asyncio loop that this standalone selection does
not provide; identical candidate and oracle failures remain diagnostic and are
not merged.

Complete candidate/oracle JUnit SHA-256 values are
`c7fa89b1ed3a392a7f61da7275c0641beee4d9ac6a6ed4f117d74cf2fc310f76`
and `5d6c1cb69070528904559ea5a6aa6c1c3bcb178b3aa1c983387374d5603dff50`;
their provenance sidecars independently verify and have SHA-256 values
`8cc4f9eb6bf39cf3fc3848cc430d5f4664a46d4cc08500ffae1b44102f7f2203`
and `c6ddf77b52a9a7cc2e807dc965720d6893e133b64218691bf61b29c89f48215a`.
Exact green selections have candidate/oracle SHA-256 values
`89500e7c323740493f8284d3efc34f575b99c8abbd10bb2afe544ffb852b037a`
and `40bccf480a46d037a7bc966b41df29d165e626363ea5e1b009cabec12e6b3f0b`.

Chromium was already green, so the selection adds exactly the Firefox and
WebKit outcome rows. The cumulative candidate ledger is now 1,149 passed,
eight skipped and 5,369 `not-run`, with no recorded failure and SHA-256
`b7c38ba4cdb1cacb9b630e07e8f18b0a4f72f4fd8fc5cc92231a7ed613a35bf8`.
Of those, 1,146 passes, all eight skips and 5,360 unrun rows are applicable;
three passes and nine unrun rows are reviewed non-applicable. ABI 103, the
89-feature wheel and every runtime capability are unchanged.

NODE-387 expands the unchanged async Chromium keyboard tranche from the
preserved ABI 102 installed baseline of 16 passed / 12 failed / one skipped to
a fresh ABI 103 installed-wheel candidate at 26 passed / two failed / one
skipped. The independent Playwright 1.58.0 oracle is 27 passed / one failed /
one skipped. The repair adds `ElementHandle.press()` and `type()` through one
native focus-plus-input command, Page/Frame selector typing and query facades,
exact-Frame `eval_on_selector()` / `eval_on_selector_all()`, and the official
Keyboard API error prefix. Python does not own focus-to-input sequencing,
native Handle identity or an input queue.

Neither remaining red row is a candidate-only discrepancy. The cross-origin
navigation row fails identically in the oracle. The candidate's isolated
`test_should_press_plus` failure lost its first keydown; a diagnostic run of
the exact upstream row in 30 fresh processes reproduced the identical failure
2/30 in both candidate and oracle against the same persistent Chrome. No sleep,
retry or Rust semantic workaround was added. The complete red reports remain
preserved; only the 26 green rows plus the upstream browser skip were selected
for promotion. The cumulative ledger is therefore 1,147 passed / eight skipped
/ 5,371 not run, with no recorded failure, SHA-256
`607b552d6435948ed6e66fb8c9d0d75c92e78de51de007c20fb35f4f5a03ab8c`.

The 29-case manifest SHA-256 is
`35c8c97c20affae0129c14b7e7f0917d592825b235cd3612013ced7cfd776197`.
Final candidate JUnit/provenance SHA-256 values are
`1fdefa473d8b956c34d225b84fb930071242940a48592378bce24b5dd8f5ff06`
and `c7191c9bb3b98cd27ac03af821940f3d7373b3721b8b5185a191ed1d9e6005e1`;
the selected 27-row report hashes to
`50a65f241492594fadd5b2c768a8a41e27321d8b52319798de53eeda4d2e8ffa`.
Oracle JUnit/provenance SHA-256 values are
`521471928545630c734b7c4d90a3a840e0a6c5dc19735617abcb3eb7cedd94a3`
and `796ec9b17297110f89e280548cdcaed50e6f9b5099a97f9cc6e65d764a8aeae8`.
Both complete sidecars verify.

The installed artifact loads from `site-packages`, reports ABI 103 and 89
selected features, and passes 239/239 Python units. Two final-source wheels
have archive SHA-256 values
`e36c0321d3001f1dfcb1fe6f313d4772fb4992599079379edb73ca72d008eeab`
and `19e3198c29b51989b6a16d4ba445757636476d22b72803c48f584db4af727415`;
their payloads reproduce after normalizing only CycloneDX identity/time. The
108-package proof, release Rust tests, strict Clippy, Ruff, strict Pyright and
a fresh verified 6,526/6,526 candidate collection pass. Full Python and Core
JS/TS execution remains open.

NODE-386 closes the two recorded focus failures by exposing the existing
Rust-owned Page Keyboard through one optional PyO3 boundary and thin async/sync
facades. The unchanged five-case async Chromium focus manifest moves from the
preserved installed candidate baseline of 3/5 versus oracle 5/5 to a fresh
ABI 102 installed-wheel candidate at 5/5. `down`, `up`, `insert_text`, `type`
and `press` all retain Rust admission, native Page identity, key mapping,
delay handling and cancellation; Python owns no input protocol or queue.

The manifest SHA-256 is
`a8356082dfa848db418c89fa67739b6a3a4573bc5db5890e13ba9e77dd0d4e3f`.
Candidate JUnit/provenance SHA-256 values are
`13adb094a397aae69bf2fad378b1b23b7a090034af44f9af90aea68085d86c3e`
and `c2f3de8d306b7960952c5c359ccdc81c9bd29269c64a81c46ab4ee7f81725357`;
the reused oracle JUnit is
`4d3b1c711fb47fe25cb7aeea0d9124921696ab9a9be1ece6bff4a97c2e6bb76f`.
Both sidecars verify. Candidate is now 1,121 passed / seven skipped / 5,398
not run with no recorded failure, ledger SHA-256
`3cec1a0bd837082094831e4cf5b11e20625e5fb095c1d06ef6b5f643027f0fe0`.

The installed artifact reports ABI 102 and 89 selected features and passes
232/232 units. Two final-source wheels have archive SHA-256 values
`13443f5f68f6da9e3fd3b2d60ac9e50dddcd61014248f7352cb3ac5fe84caccd`
and `4a999b838864f72df22a832bb9a2572c181d72b27fcda8daddccdff833b66c7d`;
their payloads reproduce after normalizing only CycloneDX identity/time. Full
Python and Core JS/TS execution remains open.

NODE-385 adds the separately selectable `request-timing` capability and closes
the fixed four-case async Chromium `test_resource_timing.py` tranche. The
preserved ABI 101 baseline passed 2/4: redirect timing could be overwritten by
a stale request snapshot, and `goto()` could return before subresource terminal
notices reached Python. The final fresh installed wheel and official Playwright
1.58.0 oracle each pass 4/4 without changing upstream assertions.

Rust now owns the stable optional timing sidecar through the network model,
codec, Chromium adapter, composition, host and PyO3 boundary. It normalizes
cached and exact-zero connection phases to Playwright's unavailable value,
preserves positive submicrosecond observations, finishes redirects from the
next-hop timestamp and shares one terminal timing snapshot across Request and
Response. Python only merges newer snapshots into the existing public Request
identity. A bounded per-stream sequence fence makes `set_content()`, `goto()`
and `reload()` wait for all notices published through the native operation;
it does not poll, sleep or wait for future traffic.

The manifest SHA-256 is
`46a5e4b59d760daaf5dd63f7cd31fd03207ed20bec8349541e808d76ca1418fb`.
Final candidate/oracle JUnit SHA-256 values are
`cc8ec9dc6b49f40976d7f6672aea779f5ef70421549626117a09acd5b442a56f`
and `46b177107aca6c246073563ce466c28ab86bbb6223cc01ec96258ea8035bc04c`;
both provenance sidecars verify. Candidate is 1,119 passed / two failed /
seven skipped / 5,398 not run, SHA-256
`d5d26ca3c234e119808548b48cb6ca96e5802718f48adb619cac73006e2cae95`;
oracle is 738 passed / seven skipped / 5,781 not run, SHA-256
`1ff080b72543b70bd04a7c327e8dcfcc2260aa844953ce9de2ebb4dba8b60ba3`.
The two candidate failures are the already recorded focus rows whose Python
`Page.keyboard` facade is still missing; this tranche does not hide them.

The installed artifact reports ABI 101 and 88 selected features and passes
226/226 units. Two final-source release wheels have archive SHA-256 values
`4b5eae899bedc877edea4baf43bbb711c64938f646c38a1f3d19852667e71ec8`
and `ae9ef143fef21e943c1980eb147d2113f7c4b9a2fe86968f10e0a1037b7f9154`;
their payloads reproduce after normalizing only CycloneDX identity/time. The
108-package proof, Rust formatting and strict Clippy, Ruff and strict Pyright
pass. Full Python and Core JS/TS execution remains open.

NODE-384 closes the two remaining Chromium rows in the unchanged upstream
`test_request_fulfill.py` file without a runtime or ABI change. The fresh ABI
100 installed-wheel candidate and official Playwright 1.58.0 oracle both pass
2/2: `page.request.fetch(route.request)` fulfills from the Rust-retained
APIResponse, and JSON fulfillment returns the official status, content type,
header and decoded body. Together with the previously promoted override row,
Chromium coverage for this upstream file is now 3/3.

The fixed manifest SHA-256 is
`a8e7f4004b567fd6d12c9e4f0f9791405a8a35f548488f88414e46e98b4c52f0`.
Candidate/oracle JUnit SHA-256 values are
`3fa160ad3813072712aa645cadb852962569e3e4f305103cad61131e690f2f98`
and `afc6b25477b04ed913b218fc99d1376cc22142539455eb3eae6233d2e9bfa85b`;
both provenance sidecars verify. Candidate is now 1,110 passed / seven skipped /
5,409 not run, SHA-256
`324dc918352b2afa46745ab26825f0b7d19db9f657b237b769c68ad2754585c1`;
oracle is 727 / seven / 5,792, SHA-256
`393bf16e0e001f6a7f264d1f852fdebd0abaa8fef4d6c6b47c464afc4f6a761a`.
Among 6,514 applicable rows, candidate is 1,107 passed / seven skipped /
5,400 not run. Full Python and Core JS/TS execution remains open.

NODE-383 closes the fixed 15-case remaining async Chromium JSHandle tranche.
The unchanged baseline candidate passed 6/15 while official Playwright 1.58.0
passed 15/15. ABI 100 then passes 15/15 from a fresh installed release wheel.
The repair keeps composite Handle arguments in Rust: the thin facade unwraps
public wrappers while preserving container identity, and the PyO3/host boundary
binds ordered Handle owners into the already bounded value graph. Plain values
retain the original evaluation route. Rust also publishes `get_properties()` as
one atomic bounded Handle batch; Date decoding and public Handle previews are
value-shape compatibility at the Python boundary, not browser ownership.

The manifest SHA-256 is
`6e312d954906f7368d65ee2efe7edd8ab34ff4477586c0777953799407532718`.
Baseline, oracle and final candidate JUnit SHA-256 values are
`888671c948ee114e471a948db16c548c3545d2796fc91abb99ef8f6660b58bb7`,
`e24c9e9c41cb8a7e41ed2543a720084fbdd55dcaa6a2c5e853e8a64853af265b`
and `f1c296d03e29f83b00f668c671bd3416329a3bd458f21d85cfb762df9d548b23`.
The final candidate provenance sidecar hashes to
`9717d01654f8583f6f605961907a137bcb2db8fe81c237fbce12443a5ea8c6b3`;
candidate and oracle sidecars verify independently.

The candidate ledger is now 1,108 passed / seven skipped / 5,411 not run,
SHA-256
`3708cc9f5c27b9e1a4a40e37c1e12bf8fca016e55a7dfb0ad55462ca8be2611a`;
oracle is 725 / seven / 5,794, SHA-256
`01028d3a743ce7c31a6a8c517be3b6f8bf1c702a3c5209d86fb9e1b66a12f0f5`.
Among 6,514 applicable rows, candidate is 1,105 passed / seven skipped /
5,402 not run; the 12 reviewed implementation-internal rows remain separately
visible.

The installed artifact loads from `site-packages`, reports ABI 100 and 87
features, and passes 221/221 units. Two release wheels reproduce after
normalizing only CycloneDX identity/time; archive SHA-256 values are
`feb65ddff87d4104997cce0b6ede9803a6f894c831988d459263717441dabdab`
and `a8d014654c45db42a19ed57ef0be55f1f08da26e4d1867c794f075e2c837762f`.
Rust formatting, `git diff --check`, strict Clippy, Ruff, strict Pyright, the
Python 3.9 audit, 30/30 harness tests and the 108-package dependency proof pass.
Full Python and Core JS/TS execution remains open.

NODE-382 closes the remaining candidate-specific failures in the fixed
18-case async input tranche. The separately selectable
`file-input-native-paths` leaf reuses the bounded file-input preparation and
retained-preflight owners: Rust validates the exact input target before
mutation, keeps the prepared directory and source slots alive, and dispatches
`DOM.setFileInputFiles` against the retained native element. Ordinary file
paths still use the payload route and its 50 MiB remote-CDP transfer guard;
the feature does not pretend every CDP browser is colocated.

The fresh ABI 99 installed-wheel candidate passes 17/18 unchanged Chromium
rows. Its only failure is the 200 MiB single-file case, with the exact same
official `Cannot transfer files larger than 50Mb to a browser not co-located
with the server` error as the Playwright 1.58.0 oracle. The oracle passes
16/18: it also rejects the ten-by-49-MiB chooser case at its aggregate remote
transport guard, while Pwright passes that row through native chooser paths.
Thus all 16 oracle-executable rows agree, every directory and
`webkitdirectory` behavior is green, and there is no candidate-only failure;
the two environment-limited rows remain explicit rather than becoming
synthetic skips. Candidate/oracle report SHA-256 values are
`53a396e74a23a615f99e931474a06b3d9b102a346e1860c2fdc7cedc346a823f`
and `9bcee12e021280b75bde6da43d180c421d0835c5ec2471a0dc2702cbf0a5ad0b`;
both provenance sidecars verify.

Only the 17 green candidate rows and 16 green oracle rows are selected for
their cumulative ledgers. Candidate is now 1,093 passed / seven skipped /
5,426 not run, SHA-256
`2e29c50908b763d1cf39defafc540557aa16fd7e1091d0c1ea6f28270ac544f7`;
oracle is 710 / seven / 5,809, SHA-256
`5dd92c0935ec10748d6990aa04f4866e63e47520fe718870394a59461a2d8ee3`.
Among 6,514 applicable rows, 1,090 pass, seven are skipped and 5,417 remain
not run; the 12 reviewed implementation-internal rows remain three passed and
nine not run.

The installed artifact loads from `site-packages`, reports ABI 99 and 87
features, and passes 217/217 units. Two release wheels reproduce after
normalizing only CycloneDX identity/time; archive SHA-256 values are
`02aefe4e73f95a7ac28fe855cae5790709759293d0bfa956a813e9bdc7ced7ce`
and `6dbf1f5ca708d606a2587e769db8b390c89c2eeaf30b9e04ea8195f595dedf3a`.
Strict Pyright, Ruff, the 13-file Python 3.9 audit, 30/30 harness tests,
no-default-feature Rust tests/Clippy and the 108-package dependency proof all
pass. Full-corpus execution remains open.

NODE-378/379 closes four official async file-chooser/input discrepancies while
preserving the red evidence. ABI 96 could read one selected file but timed out
resetting an empty selection and lacked the two ElementHandle path mutations.
The first ABI 97 source attempt reached 2/4; it proved native path dispatch but
also exposed a missing user gesture for collection evaluation and an ordering
gap between upload completion and Page-binding callbacks.

The final opt-in `file-chooser-native-paths` composition resolves the event's
backend node inside its exact utility-world execution context and bounded
object group, validates the node object, calls `DOM.setFileInputFiles` by
`objectId`, and releases the group even when the language waiter is cancelled.
Collection evaluation uses `userGesture: true`; ElementHandle upload flushes
already-published binding results before returning; Python retains the chooser
interception lease through the mutation and ignores only expected
`TargetClosedError` during asynchronous disable.

The unchanged read, empty-reset, input/change-event and upload cases pass 4/4
in the final source candidate, independent official oracle and fresh installed
wheel. Final candidate/oracle report SHA-256 values are
`e99fe0edb6c9b50e5fed8ff4571bcee4d9fe27bf30e3c6b2a2c6f9d012ffcff8`
and `e2b8145706ad78aade6dd83abf851b017d01cb82a14bb3ef2d36d9335032e231`.
The release candidate ledger advances to 1,076 passed / seven skipped / 5,443
not run, SHA-256
`0284d9c1750530e607c3b07daff759278ffae92964751c644f7244d801a80cbf`;
oracle advances to 694 / seven / 5,825, SHA-256
`d5fc04bc059fdab72dd20e88ea8157655aa1bf6e62ab4191c7152ec3fe635d69`.
The applicability cross-tab is 1,073 passed / seven skipped / 5,434 not run.

Two release wheels reproduce after normalizing only CycloneDX identity/time;
their SHA-256 values are
`4cb27befae61e6c3f99343d38e0f68519b361a2d70bd2e99de2fb7bbae10266a`
and `81b3fdff4a27c9ab3e7e8afb9474c53be0c6145ce8b7132164b2a6d44161c1c8`.
The fresh wheel loads ABI 97 and 85 selected features from `site-packages`,
passes 213/213 units and verifies the complete 6,526-ID collection. The
dependency proof confirms that the native-path leaf adds no package beyond
`file-chooser-events`.

NODE-377 replaces the file-level private-import approximation with an
outcome-preserving audit of every expanded Python invocation. The audit parses
the clean pinned source, resolves parameterized pytest IDs to their exact AST
function, follows same-module helper functions and assignment dependencies,
and detects direct private imports, private object attributes and `internal:`
selector engines. It then joins the unchanged 6,526-row inventory and latest
candidate outcome ledger by ordered ID and verifies both recorded hashes.

The syntax audit finds 6,472 plain public candidates, 21 private-helper rows,
nine private-annotation rows and 24 rows with direct private runtime access.
The pinned applicability policy reviews all eight base tests behind those 24
rows: nine expanded rows use private state only as a public `repr` assertion
oracle, three use private fixtures to reach public selector-registration
behavior, and 12 directly test private input fields, helpers or selector
protocol. Therefore 6,514 rows remain applicable public behavior; their
current outcomes are 1,068 passed, seven skipped and 5,439 not run. The 12
non-applicable implementation rows remain explicit at three passed and nine
not run; no result was rewritten. The policy SHA-256 is
`f8e33a8161af2eedd3274f4a17e37f15ba20af14ff37e229ac5f923138ff3231`
and the reviewed classification ledger SHA-256 is
`748a48d459053b03db92de1d048abfc16649bb5a4fc0841340056db834bd8ace`.

The first semantically applicable direct-private row is now a complete red,
oracle, repair and release-wheel checkpoint. Unchanged
`tests/async/test_jshandle.py::test_jshandle_evaluate_work[chromium]` failed the
installed baseline candidate because `JSHandle._impl_obj` was absent (JUnit
SHA-256 `4524ad75633b45728d7464f4e472c51e22051aca54fd018b575076eb7dbf4304`),
while the official 1.58.0 oracle passed (JUnit SHA-256
`b74e92d666619e9e5bb586bcbaa677f26acc46a25de03802aa290ae47e689a98`).
The narrow compatibility repair exposes a read-only facade adapter and the
official `<JSHandle preview=...>` representation; Handle identity, preview and
lifetime remain Rust-owned. The source candidate and final installed release
wheel both pass the unchanged row, with JUnit SHA-256 values
`1207db183abbe5eacf17b7be4069fb550b480aa5a297d7f5b84c210456f03dfd` and
`7746d54754e3372a0779871d0d5cfed546724769c959c28edd312b9254695f5b`.
All four reports have independently verified provenance sidecars.

Only the final installed-wheel result is merged. The candidate ledger is now
1,071 passed / seven skipped / 5,448 not run, SHA-256
`c408e3fcd2abc439830a570302227ea08b4b9729a876b24416928174cb6807e4`;
the oracle is 689 passed / seven skipped / 5,830 not run, SHA-256
`9f8b5dce21bf2b02ccff5fd4adecae2b7a938828e7b4a6b815d0c6ac407b8d88`.
Two independently built release wheels have archive hashes
`64b734eab401a3beb4869617cbf0031b664a19293785dcaef2ed4b161f32402d`
and `210581a0901040c2284a24b414c311924a66c0d0094b4bc8de8f020ce6812068`;
their payloads reproduce after expected CycloneDX serial/time normalization.
Installed-wheel units pass 208/208. The expanded Python tooling gate passes
30/30 AAA tests, Ruff, the 11-file Python 3.9 source audit and strict Pyright.

NODE-376 gives future Python candidate and oracle reports the same fail-closed
provenance standard as Core. After a JUnit result exists, the guarded runner
writes a sibling `.provenance.json` that hashes the pinned manifest, selected
runtime/shim/harness trees, loaded official `playwright` package, candidate
`pwright` package and exact ABI3 artifact, inventory and per-case reports for
isolated runs, and the aggregate JUnit. It also records the exact interpreter
and version, mode/scope/API/browser/channel selection, fixture addresses,
timeouts, exact command and exit code.

`scripts/verify_playwright_python_official_provenance.py` independently checks
the clean pinned checkout, reconstructs the pytest command, recomputes fixed
manifest selections and expanded isolated IDs from the inventory, parses the
JUnit, and rejects source, package, artifact, harness or report drift. Builder,
verifier and guarded-runner suites pass 17 AAA tests; the applicability suite
adds ten and the Python 3.9 source audit adds three, so the Python tooling
checkpoint passes 30/30. Historical reports are not backfilled. Authentic
full-collection sidecars now verify 6,526/6,526 candidate and oracle IDs. Their
JUnit SHA-256 values are
`69df905bd92005f89fa2e7a59f01bb2b7095816f59791fb1aa57f63b04d0e535`
and `1dbc26cda8dc7b7a1740bd626b94bd1839ec6f20f70dda905a0aa6b3148fb24e`;
the corresponding sidecar SHA-256 values are
`d7ecfa96e25ba83563a5cf9fbe578c725c7c0aca0c3f55c67e6a4fb8852cda20`
and `f9d137a9f54777391740c31e8ec699352a52ab41a26d1a25a27d6d538480ea78`.
Collection proves identity coverage only, not execution of remaining rows.

NODE-375 repairs a Python 3.9 compatibility bug without changing ABI 96 or the
native feature graph. Although annotations are postponed, `typing.cast()`
still evaluates its first argument at runtime; 20 facade sites and two official
runner sites passed unquoted PEP 604 unions that Python 3.9 cannot construct.
The facade casts now use forward-reference strings and the runner no longer
performs the unnecessary union casts. A source audit checks all 11 package,
runner, provenance and invocation-audit Python files and reports zero runtime
union casts. A real CPython
3.9.6 import of `pwright`, `pwright.async_api` and `pwright.sync_api` succeeds
with the existing ABI3 extension.

The same checkpoint makes the Python official runner fail closed on remote
CDP endpoints, loopback/unspecified fixture binds, non-MagicDNS fixture hosts
and local/ad-hoc bootstrap server prefixes. Before any non-collection run it
also inventories the dedicated Chrome endpoint and refuses to proceed when an
unrelated nonblank Page exists, matching the Core isolation guard. Twelve AAA
audit and input-guard tests pass. Ruff and Python 3.9 compilation/import pass. Pyright, installed-wheel
tests and browser-backed official lanes remain pending because system memory
pressure is still unsafe for those heavier gates; the earlier unrelated Chrome
task was allowed to finish without interruption.

NODE-356 fixes the complete four-file Page/BrowserContext request-fallback
selection without changing the ABI or native feature graph. The fixed manifest
pins 55 Chromium rows and hashes to
`126f1bf54d8189c344eedd70a4e5d271d2bed14d7a831c8bb0a3dc88c61a5b00`.
The clean source candidate and independent Playwright 1.58.0 oracle each pass
26/26 async plus 29/29 sync. These 55 rows move the provisional candidate
ledger to 1,070 passed, zero failed, seven skipped and 5,449 not run, SHA-256
`14f2e04ad35fbfaff95109c326ff6064d955d737f192bee21eea60c062623256`.
Release wheels reproduce after CycloneDX normalization and installed-wheel
units pass 207/207. The fixed-manifest installed-wheel browser promotion is
not yet recorded: the shared dedicated Chrome CDP endpoint became unresponsive
during another concurrent task, and preserving unknown tabs precluded an
unapproved restart.

NODE-355 adds Rust-owned BrowserContext/Page request composition and
`Route.fetch()`, including live Context cookie sharing, browser user agent,
base URL, HTTP credentials, TLS policy, typed query/body values, Context-close
lifetime and JSON response fulfillment. The Python async/sync facade retains
no protocol client and does not serialize HTTP wire bytes.

The source-pinned BrowserContext-request and route-fetch selections expand to
108 Chromium rows. Final installed candidate and independent Playwright 1.58.0
oracle both pass 51/51 async Context, 47/47 sync Context, 6/6 async route and
4/4 sync route rows. All 108 were `not-run` in the prior ledger and now become
passed. The cumulative candidate ledger is 1,015 passed, zero failed, seven
skipped and 5,504 not run, SHA-256
`a0c33f08557de51ee7665720a393469fe3538c9c8f7aa421cc3f5b684e57e178`;
system-Python regeneration is byte-identical. Installed-wheel units pass
203/203, all-feature host tests pass 1,038 with 26 ignored and addon tests pass
2/2. The full 6,526-row Python corpus and complete Core JS/TS corpus remain
unfinished.

NODE-354 adds the separately selectable Rust `service-worker-policy` feature
over `init-scripts`. It owns exact block-script installation, rollback and
Context-scoped cleanup; the Python facades only validate and forward the
official `service_workers="allow" | "block"` option. The source-pinned four-row
async/sync Chromium selection passes 4/4 in the final installed candidate with
the required secure fixture. Independent per-API block-policy stress passes
async 20/20 and sync 20/20.

The independent oracle does not reach a selected test body. Official
Playwright 1.58.0 sends `Browser.setDownloadBehavior` while initializing its
CDP default Context, which Chrome 154 rejects with Context management not
supported. Its four results are fixture-setup errors, not candidate/oracle
behavior comparisons. No row enters the ledger: it remains 907 passed, zero
failed, seven skipped and 5,612 not run, SHA-256
`4a6c94e0a40e533243c0c19419685fdc30cb5f4fdbc9cde593917695879fc962`.
Installed-wheel units pass 201/201, source validation and fresh collection
remain 6,526/6,526, Rust host tests pass 1,037 with 26 ignored, addon tests
pass 2/2, and the 108-package dependency proof remains exact.

NODE-353 repairs popup lifecycle, console-handle settlement and Dialog-action
teardown without changing the ABI. The source-pinned 12-row async Chromium
BrowserContext-event file gives the same 11 passed / 1 failed result under the
final installed candidate and independent official Playwright 1.58.0 on
Chrome 154. The shared failing row expects `dialog.page is None` for a
JavaScript-URL popup, while both implementations now expose that popup Page;
it is recorded as source/oracle drift and remains `not-run`. The 11 unchanged,
applicable rows pass 11/11 in both lanes and advance the ledger to 907 passed,
zero failed, seven skipped and 5,612 not run, SHA-256
`4a6c94e0a40e533243c0c19419685fdc30cb5f4fdbc9cde593917695879fc962`.
Final-wheel collection remains 6,526/6,526.

NODE-352 changes no runtime code. The fixed eight-row async Chromium
BrowserContext cookie-read file produces 5 passed / 3 failed under both final
installed candidate and independent oracle. All three shared failures assert
the literal cookie domain `localhost` after the required fixture served the
page from the full Tailscale MagicDNS hostname. The five environment-applicable
rows pass 5/5 in both implementations and cover pristine storage, `HttpOnly`,
strict/lax `SameSite` and multi-URL filtering. Only those five rows enter the
ledger, which is now 896 passed, zero failed, seven skipped and 5,623 not run,
SHA-256
`330786f82e6d9682e744904c6a3a985b1a25f3f9711db1dd851636fb139dc3b4`.
Full final-wheel collection remains 6,526/6,526.

NODE-351 changes no runtime code. The source-pinned async Chromium
BrowserContext clear-cookie file covers whole-context clearing, Context
isolation and exact/regular-expression name, domain and path filters. Final
installed candidate and the independent oracle both pass all seven rows 7/7.
Those rows advance the cumulative ledger to 891 passed, zero failed, seven
skipped and 5,628 not run, SHA-256
`fd8aa762af6f316feb60d2f9a777ba5432c02f4256b01baf0f5085dff7b962c1`.
Full final-wheel collection remains 6,526/6,526.

NODE-350 makes native `InvalidArgumentError` a subclass of Playwright's public
`Error`; Python signature preflight still uses ordinary `ValueError`. Before
the fix, the BrowserContext add-cookie file was candidate 15 passed / 4 failed
while the oracle was 17 passed / 2 failed. Final installed candidate and the
oracle now agree at 17 passed / 2 failed. Both remaining failures are fixture
applicability: one cookie is explicitly scoped to `localhost` but navigation
uses MagicDNS, and one assertion hardcodes the `localhost` default domain. The
clean 17-row candidate/oracle selection passes 17/17. Those rows advance the
ledger to 884 passed, zero failed, seven skipped and 5,635 not run, SHA-256
`193d0cfe5e6674effc3bd876ec3fc8299b7e0256986554a3785f66f9f4c0850d`.
Full final-wheel collection remains 6,526/6,526.

NODE-349 fixes duplicate Chromium sessions for an explicitly created Page.
The manual `Target.attachToTarget` session is synchronized first; if browser-
root auto-attach later reports the same Page target, topology now detaches that
duplicate instead of initializing a second Fetch observer. The source-pinned
BrowserContext-route manifest contains nine rows. Final installed candidate
and independent oracle both report 8 passed / 1 failed, with the same sole
MagicDNS-versus-hardcoded-`localhost` cookie-domain assertion; the eight
environment-applicable rows pass 8/8 in both implementations. Only those eight
advance the ledger, which is now 867 passed, zero failed, seven skipped and
5,652 not run, SHA-256
`2b6b9e6dc39344b6a86b334473b3860dd0207c49686a6bcd610e9a06b1bb8b3f`.
The unchanged ABI remains 93 and full final-wheel collection remains
6,526/6,526.

NODE-348 changes no runtime code and replaces five previously excluded
Page-route observations with a fixed manifest and per-case processes. Final
installed candidate and independent oracle both pass 5/5, including the
redirect-header row that hung only after the earlier shared browser became
polluted. The cumulative ledger now contains all 44 async Chromium Page-route
rows as passed: 859 passed overall, zero failed, seven skipped and 5,660 not
run, SHA-256
`2f75e41c034568da65f7e0cc10fbac364a54f5778dc9ccc6ea5c875778435194`.
This is non-overlapping fixed-case evidence, not a retroactive claim that the
earlier interrupted whole-file process completed.

NODE-347 keeps ABI 93 and makes Chromium topology auto-attach match the
selected composition: Page/OOPIF topology attaches only to `page` and
`iframe`, adding `worker` only when a selected observer retains workers. The
CDP filter ends with a catch-all exclusion, so an unselected service worker is
left to Chromium instead of being paused in an unowned Pwright session. The
unchanged official service-worker-precedence row passes the final installed
candidate 1/1, final-wheel collection remains 6,526/6,526, and the ledger is
854 passed, zero failed, seven skipped and 5,665 not run, SHA-256
`5bc1fbb17929fb0dab2845f3041317382b3b7cab906d7d024198228b9d51b850`.
The independent oracle attempts are not product evidence: a stale Chrome
service-worker target without a browser Context ID made official Playwright
abort during connection. A later whole-file diagnostic passed its first 24
isolated cases, then lost CDP responsiveness during an unrelated redirect
header case; it is also excluded rather than promoted to a 44/44 claim.

NODE-346 composes Page routing with the existing Rust request-history owner
through a separately selectable feature. Rust auto-continues redirect pauses,
replays bounded explicit headers without `Cookie`, retires terminal chain
state and preserves the exact native Request identity. The exact ABI 92
candidate baseline fails both unchanged redirect-chain rows; final installed
candidate and independent oracle pass 2/2. The complete async Chromium
Page-route file improves to 43 passed / 1 failed, final-wheel collection is
6,526/6,526, and the ledger is 853 passed, zero failed, seven skipped and
5,666 not run, SHA-256
`f4dca295f673c6b680020e02c4f99d8a7a8cd2dd261696dcdf7638a32ce9a362`.
Remaining Python and complete Core JS/TS execution stay open.

NODE-345 composes Page routing with the Rust API-request owner through a
separately selectable feature. The exact ABI 91 candidate baseline fails the
unchanged global-fetch `Route.fulfill(response=...)` row; final installed
candidate and independent oracle pass 1/1 without copying response bytes
through Python. The complete async Chromium Page-route file improves to 41
passed / 3 failed, final-wheel collection is 6,526/6,526, and the ledger is
851 passed, zero failed, seven skipped and 5,668 not run, SHA-256
`09089b9c798ccf3a4e3f93645839257783b16a39c03b8c737ade9f34b0d40d48`.
The three Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-344 handles matching browser-generated CORS preflights entirely in the
bounded Rust route owner. The exact ABI 91 candidate baseline fails both
unchanged POST/method rows; final installed candidate and independent oracle
pass 2/2 without an extra Python callback or consumed route budget. The
complete async Chromium Page-route file improves to 40 passed / 4 failed,
final-wheel collection is 6,526/6,526, and the ledger is 850 passed, zero
failed, seven skipped and 5,669 not run, SHA-256
`5a0465a5fdeef895cae08b30b2babca1b794b28474a5cf32118962e1556ab0d4`.
The APIResponse fulfillment discrepancy is handed to NODE-345. The other
three Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-343 carries Chromium's exact bounded native network identity from a
routed Request into the existing Rust request-history owner. The fixed
encoded-URL row moves from an ABI 90 candidate failure to final installed
candidate and independent oracle 1/1 without URL guessing or a Python request
registry. The complete async Chromium Page-route file improves to 38 passed /
6 failed, final-wheel collection is 6,526/6,526, and the ledger is 848 passed,
zero failed, seven skipped and 5,671 not run, SHA-256
`58c32ca19ac776657dcfd4e823ca58d9d57acf519b5c47d371f3c2deb12520d3`.

NODE-342 composes Python regex/predicate route identity with Rust-owned
fallback and settlement, then adds opt-in bounded Rust
`Route.fulfill(path=...)` I/O. The fixed two-row selection passes final
installed candidate and independent oracle 2/2. The complete async Chromium
Page-route file improves to 37 passed / 7 failed, final-wheel collection is
6,526/6,526, and the ledger is 847 passed, zero failed, seven skipped and
5,672 not run, SHA-256
`e423606ad57b9f2e0bdb95f4e4a74e71d65ea9582890b864e86ba76837817028`.

NODE-340 adds exact Playwright Frame identity to routed and ordinary Requests,
derives `Response.frame` from its exact Request and exposes
`ElementHandle.content_frame()` through the existing Rust Handle owner. The
fixed request-frame and route-frame manifests pass final installed candidate
and independent oracle 6/6 plus 1/1. Five request-frame rows and the route row
advance from `not-run`; the popup-first row was already passed and receives
stronger replacement evidence. The complete async Chromium Page-route file
improves to 35 passed / 9 failed, full collection remains 6,526/6,526, and the
ledger is 845 passed, zero failed, seven skipped and 5,674 not run, SHA-256
`673973f03e77d0291596cfbc971579c70d98831f634ab74606a43b37282375c7`.

NODE-339 completes Page request lifecycle listener publication through the
existing bounded Rust owner. The two unchanged route-abort rows pass final
installed candidate and independent oracle 2/2, and five candidate repeats
pass 10/10. The complete async Chromium Page-route file improves from 32/12
to 34 passed / 10 failed with no loss. A fresh collection is 6,526/6,526. The
ledger is 839 passed, zero failed, seven skipped and 5,680 not run, SHA-256
`b8c7bebbeb3f2c16880fb7fc08dd30b15d56d504d091c264b732641ef8e81e0f`.

NODE-338 makes Rust send Playwright's fixed HTTP reason phrase in
`Fetch.fulfillRequest` rather than accepting Chrome's current default. The
unchanged 422 row moves from exact candidate failure to installed candidate
and independent oracle 1/1; five further candidate runs pass 5/5. The complete
isolated Page-route file is 32 passed / 12 failed with no regression. The
ledger is 837 passed, zero failed, seven skipped and 5,682 not run, SHA-256
`8a6d6a9169086e482fbd8acf6d81162ef42ab209f18c2c174238e5feb85e2ef5`.

NODE-337 changes no runtime code and promotes native `Referer`/`Origin`
lowercase publication already fixed by NODE-336. The exact ABI 84 baseline
fails 2/2; installed candidate and independent oracle pass 2/2, with five
further candidate runs passing 10/10. The complete isolated async Chromium
Page-route file is 31 passed / 13 failed; two previously recorded unstable
rows that happened to pass stay outside the ledger. The ledger is 836 passed,
zero failed, seven skipped and 5,683 not run, SHA-256
`eb6b687f25461fa8d8f4182455215dbcf565a47296427680357866df79cc8553`.

NODE-336 adds thin async/sync Page and BrowserContext extra-header bindings
over the existing bounded Rust owner, composes Page referers into route
snapshots when Fetch omits them and normalizes native request-header names to
Playwright's lowercase public shape. The exact ABI 84 candidate baseline fails
3/3; the unchanged official Chromium selection passes installed candidate and
independent oracle 3/3, with five further candidate runs passing 15/15. The
ledger is 834 passed, zero failed, seven skipped and 5,685 not run, SHA-256
`a0c1caa9d38619302ae8b0eaa49450d1c9db9a5db80cb44e4fa6fb5e69c4b783`.
Full collection remains 6,526/6,526. Remaining Python and complete Core JS/TS
execution remain explicit work rather than exclusions.

NODE-335 adds Rust-owned composable fallback traversal and handler-specific
unroute for Page and BrowserContext. The exact ABI 84 baseline is seven
failures plus one existing pass; the unchanged official Chromium manifest
passes final installed candidate and independent oracle 8/8, with five more
candidate executions passing 40/40. The ledger is 831 passed, zero failed,
seven skipped and 5,688 not run, SHA-256
`4303259b2a6813aa7e3fc8017d89d1cc5d62a38bed6b66eacc3af5600d576722`.
Full collection remains 6,526/6,526. Regex/predicate matching,
`unroute_all(behavior="ignoreErrors")`, remaining Python and complete Core
JS/TS execution remain explicit work rather than exclusions.

NODE-334 adds atomic Rust-owned `times` budgets for bounded string routes on
Page and BrowserContext. Three unchanged async Chromium rows move from an ABI
83 candidate 0/3 baseline to final installed candidate and independent oracle
3/3; five additional candidate executions pass 15/15. The ledger is 823
passed, zero failed, seven skipped and 5,696 not run, SHA-256
`26023955d6cd01d96099cf7b07ef479d028e86a9fd60038e14c786479f467439`.
Full collection remained 6,526/6,526 at that checkpoint.

NODE-333 makes route-glob `?` literal in Rust and in Chromium's Fetch pattern.
The unchanged official row moves from candidate failure to installed
candidate/oracle 1/1; final candidate repeat passes 5/5. The ledger is 820
passed, zero failed, seven skipped and 5,699 not run, SHA-256
`aaa91984391a096225cae1684c0f1879249b41feedd8efad7a5203053087e6d0`.
The final complete Page-route diagnostic is 22 passed / 22 failed. Two
apparent losses relative to NODE332 reproduce on its unchanged wheel and are
kept as unstable diagnostics, not attributed regressions or exclusions.

NODE-332 implements bounded Rust-owned `Route.continue_()` URL, method,
headers and POST-data overrides. The unchanged ten-case async Chromium file
moves from an ABI 82 candidate baseline of 2 passed / 8 failed to final
installed candidate and independent oracle 10/10. All ten rows advance from
`not-run` to `passed`: the ledger is 819 passed, zero failed, seven skipped and
5,700 not run, SHA-256
`43861212714734aa05343ef0ff48a87977136adbaecfe9d0c3c461bfb50af2f6`.
The complete Page-route file improves to 23 passed / 21 failed / 0 errors;
remaining failures are not skipped or reclassified. Full collection remains
6,526/6,526.

NODE-331 executes the complete 44-case async Page-route Chromium file with
per-case isolation and a 30-second hard timeout. The candidate baseline is 18
passed, 25 failed and one error. The 18 existing supported rows pass a fixed
installed-candidate selection and independent oracle 18/18, advancing the
ledger to 809 passed, zero failed, seven skipped and 5,710 not run, SHA-256
`0408aef3e6fa958f60ce74998e4326e40aef2bc906851efa35e9b656e9b5940c`.
The remaining rows are recorded discrepancies rather than exclusions.
NODE-330 promotes six additional unchanged unroute-all rows on the same final
ABI 82 wheel. Candidate and independent oracle both pass async/sync default
removal 4/4 and in-flight wait/fulfill 2/2. The cumulative ledger is 791
passed, zero failed, seven skipped and 5,728 not run, SHA-256
`d7255f3d41c76a26528036c0eaee54b788a48b2323650b3700f2da7441f7cb05`.
Handler-specific unroute, `fallback()`, regex/predicate matching and
`behavior="ignoreErrors"` remain explicit gaps.
NODE-329 adds async/sync Page and BrowserContext
`unroute_all(behavior="wait")` over the same bounded Rust route owner. The two
unchanged HAR-abort rows pass final installed candidate and independent oracle
2/2; candidate repeat stress passes 20/20. The cumulative ledger is 785
passed, zero failed, seven skipped and 5,734 not run, SHA-256
`b58f55b19a62286fdf0c1b06040c017c1d9b00666b3e139847eaf8cc42027b6f`.
Handler-specific unroute and `behavior="ignoreErrors"` remain explicit gaps.
NODE-328 adds async/sync `BrowserContext.route()` over the same bounded Rust
scope and proves its composition with Context HAR URL filters. The four
unchanged official rows pass final installed candidate and independent oracle
4/4; candidate stress passes async 20/20 and sync 20/20. The cumulative ledger
is 783 passed, zero failed, seven skipped and 5,736 not run, SHA-256
`c0c16280690a496ffdc374fa77ad1792aedd6670a9c61026d70fc042bce77b17`.
NODE-327 treats a missing `log.entries` member as a bounded empty HAR and lets
the existing not-found policy decide abort versus fallback. The unchanged
async/sync bad-HAR fallback rows pass final installed candidate and independent
oracle 2/2; candidate stress passes async 10/10 and sync 10/10. The cumulative
ledger is 779 passed, zero failed, seven skipped and 5,740 not run, SHA-256
`a545a96742ef986a132be551627450e08c8d1fd0ecbfca8a1fe8e5ab142328c8`.
NODE-326 adds async/sync `BrowserContext.route_from_har()` with Rust-owned
Context scope, Page-over-Context precedence and a bounded initializer for
future Pages/OOPIFs. The unchanged method/redirect, fallback-continue and
default-abort rows pass final installed candidate and independent oracle 6/6;
candidate stress passes async 30/30 and sync 30/30. The cumulative ledger is
777 passed, zero failed, seven skipped and 5,742 not run, SHA-256
`c52d8ae2a549fc2cfa75e745969863363c8e10705daea947e46a9446d58b89a7`.
External-content/ZIP HAR, recording updates, regex/predicate URL filters,
handler-specific unroute and broader routing coverage remain open.
NODE-325 promotes the complete unchanged async/sync Page HAR URL-filter/no-
fallback pair, including composition with an ordinary `page.route()`, on the
same installed wheel. Candidate and oracle pass 2/2; separate candidate stress
lanes pass async 10/10 and sync 10/10. The cumulative ledger is 771 passed,
zero failed, seven skipped and 5,748 not run, SHA-256
`fb11fa9a34ed364ec06254672ac32ef54a7a790362eeaa4f6a562887c2d1f03e`.
NODE-324 promotes the complete unchanged async/sync Page HAR method/redirect
pair on the same installed wheel. Candidate and oracle pass 2/2; separate
candidate stress lanes pass async 10/10 and sync 10/10. The cumulative ledger
is 769 passed, zero failed, seven skipped and 5,750 not run, SHA-256
`dbc28e3a20089d56d2b937d7175f19e4562db76da59477b789e8c14fc375cefd`.
The mixed-API repeat diagnostic is excluded because pytest-asyncio attempts to
enter its session Runner from the synchronous bridge's already-running loop.
NODE-323 adds independently selectable bounded plain-HAR replay over the
existing Page-route driver plus Rust-owned computed-style assertion polling.
The unchanged persistent-Context Chromium HAR row passes final installed
candidate and independent oracle 1/1; candidate stress passes 10/10. The
cumulative candidate ledger is 767 passed, zero failed, seven skipped and
5,752 not run, SHA-256
`e390e350e8995a48569133ed5be757cd9040a6d1592ab1b86f0ad5ca046476e2`.
External-content/ZIP HAR, recording updates, Context routing and the broader
HAR corpus remain open; the two original cookie rows remain MagicDNS versus
hardcoded-`localhost` applicability gaps.
NODE-322 adds independently selectable ordinary and persistent-default Context
HTTP credentials, shared bounded Fetch-authentication dispatch and navigation
response-body retrieval. Eleven unchanged Chromium rows pass final installed
candidate and independent oracle 11/11; candidate stress passes 110/110. The
cumulative candidate ledger is 766 passed, zero failed, seven skipped and
5,753 not run. Three original default-Context rows remain: two recorded
MagicDNS/`localhost` cookie applicability gaps and HAR.
NODE-321 adds independently selectable ordinary and persistent-default Context
geolocation and permission, plus bounded persistent Chromium launch arguments.
The unchanged Chromium row passes final installed candidate and independent
oracle 1/1 and candidate stress 10/10. The cumulative candidate ledger is 755
passed, zero failed, seven skipped and 5,764 not run; four original
default-Context rows remain, including the two recorded MagicDNS/`localhost`
applicability gaps.
NODE-320 adds independently selectable ordinary and persistent-default Context
offline policies through a bounded Rust owner. The unchanged Chromium row
passes final installed candidate and independent oracle 1/1 and candidate
stress 10/10. The cumulative candidate ledger is 754 passed, zero failed,
seven skipped and 5,765 not run; five original default-Context rows remain,
including the two recorded MagicDNS/`localhost` applicability gaps.
NODE-319 adds independently selectable ordinary and persistent-default Context
timezone policies through a bounded Rust owner. The unchanged Chromium row
passes final installed candidate and independent oracle 1/1 and candidate
stress 10/10. The cumulative candidate ledger is 753 passed, zero failed,
seven skipped and 5,766 not run at that checkpoint; six original default-Context rows remain,
including the two recorded MagicDNS/`localhost` applicability gaps.
NODE-318 adds Context JavaScript disabling and persistent-default composition
through one bounded Rust owner, thin `Page.expect_navigation` bindings over the
existing frame-navigation owner, fragment-aware default click targeting and
nonfatal early console replay. Four unchanged Chromium rows pass final
installed candidate and independent oracle 4/4 and candidate stress 40/40. The
cumulative candidate ledger is 752 passed, zero failed, seven skipped and 5,767
not run at that checkpoint. NODE-317 adds Context CSP
bypass and persistent-default composition through one bounded Rust owner. Its
four unchanged Chromium rows pass final installed candidate and independent
oracle 4/4 and candidate stress 40/40. The separate iframe CSP row remains
unclaimed behind `ElementHandle.content_frame`.
NODE-316 composes persistent launch with the existing Rust header owner. Its
unchanged Chromium row passes final installed candidate and independent oracle
1/1 and candidate stress 10/10. The cumulative candidate ledger is 744 passed,
zero failed, seven skipped and 5,775 not run. The two
NODE-313 hardcoded-`localhost` rows remain explicit MagicDNS applicability
gaps. This is not a full-corpus compatibility claim. The
ABI 35 baseline passes
the complete unchanged async and sync global API-request files, 231/231
candidate with 231 matching oracle outcomes, and the fixed official
client-certificate selection with eight passes plus four upstream macOS WebKit
skips in both candidate and oracle lanes. It also passes both complete
unchanged async and sync init-script files, 18/18 candidate with 18 matching
oracle outcomes. It additionally passes a fixed 18-case Page script/style-tag
selection and a fixed seven-case JSHandle selection with matching independent
oracle outcomes. ABI 14 additionally passes 12 fixed Page-binding rows and ABI
15 passes six fixed Page-lifecycle rows, each with matching independent oracle
outcomes. ABI 16 adds five fixed Page-error rows and ABI 17 adds five fixed
async Chromium Page-route rows. ABI 18 adds two fixed `Request.sizes()` rows,
and ABI 19 adds eight Page/Locator drag rows across the async and sync APIs.
ABI 20 adds ten async Page dialog/beforeunload rows and one sync Dialog row.
ABI 21 adds the exact async Chromium relative-base-URL route/fulfill/body row;
it also adds the Page-close terminal outcome for a pending download waiter.
ABI 22 adds one exact Chromium persistent-Context/base-URL row through a
Rust-owned installed-browser launch and bounded process lease.
ABI 23 adds the two exact headless `Page.pause()` timeout-reset rows through a
Rust-owned scoped reset/restore guard and a task-owned headless Chrome lane.
After fixing terminal Page-route settlement, the complete 125-row async
Chromium Page file was executed one row per process in both candidate and
independent oracle lanes: each records 124 passes and the same upstream FIXME
skip, with zero per-test status differences. These fixed selections and the
full Page file have matching independent oracle outcomes. ABI 24 additionally
passes the complete async and sync Chromium `test_page_request_gc.py` files,
2/2 candidate with 2/2 matching oracle outcomes, through the existing
independently selectable Rust GC leaf.
ABI 25 additionally passes the unchanged async and sync Chromium
`Page.page_errors()` history rows, 2/2 candidate with 2/2 matching oracle
outcomes. The history and its item/byte leases are bounded and Rust-owned.
ABI 26 additionally passes the unchanged async and sync Chromium
`Page.console_messages()` newest-200 rows, 2/2 candidate with 2/2 matching
oracle outcomes. The history is bounded and Rust-owned, and is selectable
without console callback publication.
ABI 27 additionally passes the unchanged async and sync Chromium
`Page.requests()` last-100 rows, 2/2 candidate with 2/2 matching oracle
outcomes. Rust owns eviction and every retained Request/Response/body owner;
the sync façade reuses the same native route/history path.
ABI 28 additionally passes the complete unchanged device-descriptor file: one
Chromium pass plus the same two upstream Firefox/WebKit skips in candidate and
oracle. Rust embeds all 143 descriptors and owns bounded Context-scoped device
settings and pre-publication CDP emulation.
ABI 29 additionally passes the exact Chromium invocation from the complete
unchanged async and sync listener files in candidate and oracle. Rust owns the
bounded response notice queue and Context-scoped subscription; Firefox/WebKit
remain explicit unimplemented providers and their four expanded rows remain
`not-run`.
ABI 30 additionally passes the exact Chromium invocation from both complete
unchanged async and sync selector-misc files. Rust owns collection evaluation
and typed `internal:and` intersection; Python only forwards the public call.
The four non-Chromium expansions remain `not-run`.
ABI 31 additionally passes the exact Chromium duplicate-registration row from
the complete unchanged async selector-misc file. Rust owns the bounded global
registry and Python only forwards bounded source input. The Firefox/WebKit
expansions and custom selector-engine execution remain open.
ABI 32 additionally passes the unchanged async and sync Chromium
registration-error rows. Rust now validates selector syntax before browser
execution, preserving Playwright's unknown-engine error.
ABI 33 additionally passes the six unchanged async/sync Chromium custom-engine
base, path and main/isolated-world rows. Rust owns bounded program compilation,
engine installation, query semantics and execution-world selection. Positive
registered `Page.query_selector` handle publication, typed Locator composition
and retry polling remain open.
The same ABI additionally passes the unchanged async/sync Chromium layout-
selector rows through an independently selectable Rust parser/compiler and
browser-world geometry scorer. Broader nested layout grammar, retry polling and
positive browser-program handle publication remain open.
It also passes 34 fixed unchanged async/sync Chromium Locator rows covering
basic reads, state, collection reads, fill/check/focus/press and evaluation.
Python is a thin facade over existing Rust owners; unselected Locator methods
remain open.
ABI 34 additionally passes ten fixed unchanged async/sync Chromium Locator
click, collection-evaluation and count rows. Rust owns the typed count
comparison, total-deadline retry and bounded sleep; Python only adapts the
assertion shape. Remaining assertions, actionability and providers remain
open.
The same ABI additionally passes the three real async-only Chromium Locator
description rows. Description is immutable facade metadata and performs no
browser/native work; chained-description tests remain open until typed Locator
chaining is exported to Python.
The former seven-test lane remains a historical bootstrap check only.
NODE-258 then executes the complete unchanged async and sync Chromium Locator
files. The original ABI 35 candidate passes 60/163 while the independent
official lane passes 163/163. NODE-259/NODE-260 repair 19 rows without deleting
the remaining failures. NODE-261 through NODE-264 then add typed filters,
waits/expectations, role/test-id/evaluation and visible-chain semantics through
Rust-owned recipes. NODE-265 adds Rust-owned double-click, bounded file input
and element-handle publication; NODE-266 exports the pre-existing Rust-owned
highlight capability as a separate feature. NODE-267 adds independently
selectable DOM actions and Handle composition; NODE-268 closes
capture-plus-`nth`, relative-XPath `has=`, Unicode timeout and Locator click
call-log discrepancies in bounded Rust owners. NODE-269 adds Rust-owned Page
viewport state but preserves its four-row report as red when the rows expose
missing downstream Locator geometry APIs. NODE-270 then composes Rust-owned
bounding-box and screenshot capabilities. NODE-271 adds the lazy Rust-owned
FrameLocator recipe and matches the official Python callback-arity adapter.
The final installed ABI 44 wheel passes 163/163 (86/86 async and 77/77 sync),
matching the independent 163/163 Locator oracle. Its complete cumulative
6,526-row ledger records 612 passed, seven skipped and 5,907 not run, SHA-256
`89eabed04a3a7804637229ff077cf0898cc1aa903c190611bfb2d1553650aefe`.
This is exhaustive accounting for the two Locator files, not a claim that all
6,526 Python rows pass.
NODE-272 next executes the complete unchanged sync semantic-Locator file. All
ten Chromium invocations pass in installed-candidate and independent-oracle
lanes; the pinned upstream source tree contains no async file with that name.
The cumulative candidate ledger is now 622 passed, seven skipped and 5,897 not
run. This remains a partial-corpus result.
NODE-273 adds the complete unchanged async semantic-selector file: its three
Chromium invocations pass in installed-candidate and independent-oracle lanes
through native Locator collection evaluation. The cumulative candidate ledger
is now 625 passed, seven skipped and 5,894 not run.
NODE-274 adds both complete unchanged fill files and both complete unchanged
context-manager files. Candidate and independent oracle each pass all nine
Chromium invocations. The cumulative candidate ledger is now 634 passed,
seven skipped and 5,885 not run.
NODE-275 adds the complete unchanged async HTTPS-error file. Its two Chromium
invocations pass in installed-candidate and independent-oracle lanes through a
bounded Rust-owned Context/session policy. The four Firefox/WebKit expansions
remain explicit provider-gap `not-run` rows. The cumulative candidate ledger
is now 636 passed, seven skipped and 5,883 not run.
NODE-276 adds the complete unchanged sync PDF file. Its two applicable
Chromium invocations pass in the final installed-candidate and independent-
oracle lanes through a bounded Rust-owned capture/output owner. The four
Firefox/WebKit inventory expansions are explicitly non-applicable under the
upstream `only_browser("chromium")` marker and remain `not-run`. The cumulative
candidate ledger is now 638 passed, seven skipped and 5,881 not run.
NODE-277 adds the complete unchanged async PDF file. Its three applicable
Chromium invocations pass in the final installed-candidate and independent-
oracle lanes after typed tagged/outline options are added to the same bounded
Rust owner. The six Firefox/WebKit expansions remain upstream Chromium-only
non-applicable `not-run` rows. The cumulative candidate ledger is now 641
passed, seven skipped and 5,878 not run.
NODE-278 adds the complete unchanged async screenshot file. Its three Chromium
invocations pass in the final installed-candidate and independent-oracle lanes
through bounded Rust-owned Page, Locator and exact retained ElementHandle
capture. The source has no Chromium-only marker, so its six Firefox/WebKit
expansions remain explicit provider-gap `not-run` rows. The cumulative
candidate ledger is now 644 passed, seven skipped and 5,875 not run.
NODE-279 adds the complete unchanged async legacy-text selector file. Its two
Chromium invocations pass in the final installed-candidate and independent-
oracle lanes through typed Rust-owned legacy selectors and bounded Page handle
publication. The four Firefox/WebKit expansions remain explicit provider-gap
`not-run` rows. The cumulative candidate ledger is now 646 passed, seven
skipped and 5,873 not run.
NODE-280 adds the complete unchanged async selector-generator file. Its one
Chromium invocation passes in the final installed-candidate and independent-
oracle lanes through bounded Rust-owned test-id configuration and strict-error
previews. Firefox/WebKit remain explicit provider-gap `not-run` rows. The
cumulative candidate ledger is now 647 passed, seven skipped and 5,872 not
run.
NODE-281 adds the complete unchanged shared thread-lifecycle file. Its
Chromium invocation passes in the final installed-candidate and independent-
oracle lanes without product changes. The upstream worker constructs and
stops the sync Playwright lifecycle, attaches Chromium, creates a convenience
Page and closes the Browser. Firefox/WebKit remain explicit provider-gap
`not-run` rows. The cumulative candidate ledger is now 648 passed, seven
skipped and 5,871 not run.
NODE-282 adds the complete unchanged shared event-listener file. Its Chromium
invocation passes in final installed-candidate and independent-oracle lanes
after the sync bridge propagates a response-listener exception through the
public call boundary. Firefox/WebKit remain explicit provider-gap `not-run`
rows. The cumulative candidate ledger is now 649 passed, seven skipped and
5,870 not run.
NODE-283 adds the complete unchanged shared signal-lifecycle file. Its async
and sync Chromium invocations pass in final installed-candidate and
independent-oracle lanes through the same dedicated CDP Chrome. ABI 51 exports
the existing bounded Rust timeout leaf to support the upstream child loop;
Firefox/WebKit remain four explicit provider-gap `not-run` rows. The cumulative
candidate ledger is now 651 passed, seven skipped and 5,868 not run.
NODE-284 adds both complete unchanged Page network-response files. Their four
Chromium invocations pass in final installed-candidate and independent-oracle
lanes through bounded Rust-owned terminal Response completion. The eight
Firefox/WebKit expansions remain explicit provider-gap `not-run` rows. The
cumulative candidate ledger is now 655 passed, seven skipped and 5,864 not
run.
NODE-285 adds the complete unchanged async Page network-request file. Its two
Chromium invocations pass in final installed-candidate and independent-oracle
lanes through bounded Rust-owned Context request waits, exact request-history
ownership, post-data parsing and same-cursor Frame availability. Four
Firefox/WebKit expansions remain explicit provider-gap `not-run` rows. The
cumulative candidate ledger is now 657 passed, seven skipped and 5,862 not
run.
NODE-286 adds the complete unchanged sync input file. Its two Chromium
invocations pass in final repaired-wheel candidate and independent-oracle
lanes through exact-session Rust-owned file-chooser publication plus the
existing bounded file-input mutation owner. The four Firefox/WebKit expansions
remain explicit provider-gap `not-run` rows. The cumulative candidate ledger
is now 659 passed, seven skipped and 5,860 not run.
NODE-287 selects the two applicable Chromium rows from unchanged
`tests/async/test_issues.py`: cancellation of a pending selector wait and
device-descriptor Context creation. Both pass in the ABI 55 installed-wheel
candidate and independent oracle. Rust owns one total selector deadline,
default non-strict first-match selection and ElementHandle publication; Python
only validates arguments and wraps the result. Direct
`BrowserType.launch(ignore_default_args=...)` remains an explicit launch-
provider gap, so the third Chromium row is not recast as product conformance.
The cumulative candidate ledger is now 661 passed, seven skipped and 5,858 not
run.
NODE-288 selects three additional Chromium selector-wait compatibility rows
from unchanged async asyncio/expect-misc and sync API files. Candidate and
independent oracle each pass cancellation without an unretrieved future,
timeout error naming and sync `wait_for_selector`. The only baseline
discrepancy was the missing public `TimeoutError.name`; a Python error-shape
property fixes it without changing native ABI 55. The cumulative candidate
ledger is now 664 passed, seven skipped and 5,855 not run.
NODE-289 selects the two remaining Chromium rows from unchanged async
`test_asyncio.py`. TaskGroup cancellation already passes. The API-name row
passes after the thin Python Page boundary adds `Page.evaluate:` to the
existing native `JavascriptError` while preserving its Rust type and code.
The five applicable Chromium rows in that file are now complete; browser-using
Firefox/WebKit expansions remain explicit provider gaps. The cumulative
candidate ledger is now 666 passed, seven skipped and 5,853 not run.
NODE-290 executes all 25 previously unrecorded Chromium rows from unchanged
sync `test_sync.py`. Oracle is 24 passed / one failed; candidate starts at
9 passed / 16 failed. Four public repr repairs plus native request-history
publication for thin `Response.request` wrappers produce 14 passed / 11
failed. With two earlier green rows, all 27 Chromium rows in the file now have
outcomes. The cumulative candidate is 680 passed, 11 failed, seven skipped and
5,828 not run; this is explicit red accounting, not a green-file claim.
NODE-291 moves the two ElementHandle discrepancies into the optional Rust
`element-handle-dom` composition. Both official rows pass from the final ABI
56 wheel. The file is now 18/27 candidate versus 26/27 oracle; cumulative
candidate accounting is 682 passed, nine failed, seven skipped and 5,828 not
run.
The cross-language completion criteria and core JS/TS inventory are in
[Full upstream Playwright conformance](upstream-playwright-conformance.md).

## Source of truth

The upstream repository is `microsoft/playwright-python`, tag `v1.58.0`, commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. The selected source files and their
SHA-256 digests are recorded in
`packages/pwright-python/tests/official/manifest.json`.

ABI 33 custom-engine execution uses
`packages/pwright-python/tests/official/selector-custom-engine-manifest.json`
and
`packages/pwright-python/tests/official/selector-custom-engine-worlds-manifest.json`.
Both pin the exact async/sync source hashes; together they fix six Chromium
invocations without permitting an ad-hoc green-node filter.

NODE-253 layout execution uses
`packages/pwright-python/tests/official/selector-layout-manifest.json`. It pins
the same exact async/sync source hashes and fixes only
`test_should_work_with_layout_selectors`; browser selection expands it to the
two Chromium invocations recorded here. `--manifest-cases --manifest-api`
forbids an ad-hoc green-node filter.

NODE-254 basic Locator execution uses
`packages/pwright-python/tests/official/locator-basic-manifest.json`. It pins
unchanged async/sync `test_locators.py` source hashes and 34 exact function
rows. `--manifest-cases --manifest-api async|sync --browser chromium` is the
only accepted green selection.

NODE-255 Locator execution and count expectation uses
`packages/pwright-python/tests/official/locator-execution-manifest.json`,
SHA-256
`ed03f8d21b9d27cd400b3a3cb98203a20a35fdc4c7f57aa6e03ec00c92411e09`.
It pins the same unchanged async/sync source hashes and ten exact Chromium
invocations. `--manifest-cases --manifest-api async|sync --browser chromium`
is the only accepted green selection.

NODE-256 Locator description execution uses
`packages/pwright-python/tests/official/locator-description-manifest.json`,
SHA-256
`d578cddad1cbb56d29d098b4fb1c19a20bd1d24a739809a25bdacb03d6591374`.
The pinned upstream sync source contains no corresponding description tests;
the manifest therefore fixes only the three real async Chromium invocations
instead of manufacturing a sync lane.

NODE-257 typed Locator composition uses
`packages/pwright-python/tests/official/locator-composition-manifest.json`,
SHA-256
`78e5b01d4582222168c51fea88add9a89987374914de5bac4711e2a7e4d4755f`.
It pins the unchanged async/sync `test_locators.py` hashes and exactly four
async plus two sync Chromium invocations covering chained descriptions and
first/last/nth derivation. `--manifest-cases --manifest-api async|sync
--browser chromium` is the only accepted green selection.

Client-certificate execution additionally uses
`packages/pwright-python/tests/official/client-certificates-manifest.json`.
That manifest pins both source-file hashes and exactly four test functions;
their browser parametrization expands to 12 invocations. `--manifest-cases`
forbids ad-hoc test filters, while `--manifest-api async|sync` keeps the
upstream async and sync session fixtures in separate processes.

ABI 13 Handle/tag execution uses
`packages/pwright-python/tests/official/handles-tags-manifest.json` and
`packages/pwright-python/tests/official/jshandle-core-manifest.json`. Both pin
the exact upstream file digest and function list. They expand to 18 and seven
Chromium invocations respectively; no ad-hoc green-node filter is accepted.

ABI 14 Page-binding execution uses
`packages/pwright-python/tests/official/page-bindings-manifest.json`; it pins
the exact Page source digest and 12 explicit functions. ABI 15 lifecycle
execution uses
`packages/pwright-python/tests/official/page-lifecycle-events-manifest.json`,
which pins the same source digest and exactly six load, DOMContentLoaded,
close and opener functions. Both require `--manifest-cases`; neither permits
an ad-hoc passing-node filter.

ABI 16 Page-error execution uses
`packages/pwright-python/tests/official/page-errors-manifest.json`. It pins the
same source digest and exactly five Error name/message/stack, primitive, object
and Window functions. It also requires `--manifest-cases`.

ABI 17 Page-route execution uses
`packages/pwright-python/tests/official/page-routes-manifest.json`. It pins the
same source digest and exactly five caller-stack, set-content resource/timeout,
and continue-during/after-close functions. It requires `--manifest-cases` and
the canonical lane additionally selects `--browser chromium`; no ad-hoc green
node filter is accepted.

ABI 18 Request-size execution uses
`packages/pwright-python/tests/official/request-sizes-manifest.json`. It pins
the same Page source digest and exactly the two async body/header-size
functions. It requires `--manifest-cases` and the canonical lane selects
Chromium; the manifest is a fixed replacement slice, not the complete Core
network-size suite.

ABI 19 drag execution uses
`packages/pwright-python/tests/official/drag-and-drop-manifest.json`. It pins
the exact async/sync Page and Locator source digests and eight explicit
functions. `--manifest-cases --manifest-api async|sync` executes the two API
halves in separate processes; the canonical lane also selects Chromium. Four
of the expanded rows replace cases already present in the complete Page report
and four are newly recorded, so exact JUnit selection is required before ledger
merging to prevent double counting.

ABI 20 dialog execution uses
`packages/pwright-python/tests/official/page-dialogs-manifest.json`. It pins
the exact async Dialog, async Page and sync source digests and 11 explicit
functions. `--manifest-cases --manifest-api async|sync` executes the API halves
in separate processes and the canonical lane selects Chromium. Two
beforeunload rows replace failed cases already present in the complete Page
report; eight async Dialog rows and one sync row are newly recorded. Exact
JUnit selection and Page-report replacement prevent double counting.

ABI 21 route-fulfill execution uses
`packages/pwright-python/tests/official/route-fulfill-manifest.json`. It pins
the exact base-URL source digest and one explicit function. The canonical lane
selects Chromium and proves relative string-route matching, `Route.fulfill`,
`expect_request`, `expect_response` and `Response.body` without editing the
upstream assertion. The result replaces the existing failed base-URL row once.

ABI 21 terminal-waiter execution uses
`packages/pwright-python/tests/official/download-terminal-manifest.json`. It
pins the exact Page source digest and the one explicit function that arms
`expect_download`, closes the Page and requires Playwright's target-closed
error. It does not claim successful download publication or artifacts.

ABI 22 persistent-Context execution uses
`packages/pwright-python/tests/official/persistent-context-manifest.json`. It
pins the exact base-URL source digest and the one explicit function that
launches Chromium with a user-data directory, requires the returned default
Context to resolve `base_url`, and closes it. The canonical candidate and
oracle lanes both select installed `channel=chrome` because the official
Playwright cache on this Mac does not contain Chromium headless shell 1208.
The earlier missing-shell oracle report is retained as an infrastructure
failure; it is not converted into a product failure or a skip.

ABI 23 Page-pause execution uses
`packages/pwright-python/tests/official/page-pause-manifest.json`. It pins the
exact Page source digest and the two functions that require default and custom
operation/navigation timeouts to be restored after a headless pause. The
candidate and official oracle attach to the same task-owned headless Chrome
launched through Pwright; the visible shared browser is not used as a fake
headless oracle.

The source checker rejects a different commit, a dirty checkout, a changed
source digest, or a missing selected test function. At this pin the upstream
`tests/` tree contains 149 `test_*.py` modules and 1,990 top-level test
functions. Of those, 141
modules / 1,750 tests avoid imports from `playwright._*`; 8 modules / 240 tests
import upstream-private namespaces. Avoiding a private import is only an
inventory category, not evidence that the test's required public capabilities
are installed. Eight TodoMVC example modules add 25 expanded tests. Pytest's
full Chromium/Firefox/WebKit collection is 6,526 node IDs across 156 non-empty
files; the 157th source module is a recorded zero-byte placeholder.

## Full no-weakened-assertion runner

The runner selects the entire upstream checkout by default, loads its real
`conftest.py` infrastructure, and positively verifies the imported
`playwright` package path before collection or execution. Oracle and candidate
collection are both 6,526/6,526. The candidate test adapter exports explicit
annotation-only placeholders for unimplemented public types and delegates the
suite's fixture-only `playwright._impl` imports to the pinned oracle package.
Every placeholder fails if execution reaches its constructor; collection
success therefore does not claim implementation support.

The runner starts outside the upstream checkout and uses importlib mode. This
prevents the upstream `playwright/` source directory from shadowing the
candidate package and turning candidate collection into a false oracle pass.

`--case-timeout <seconds>` supplies an explicit pytest signal timeout so one
candidate hang is recorded instead of blocking a shard forever. It does not
make a timed-out pytest process safe for same-process continuation: interrupted
async teardown can contaminate later cases. `--isolate-cases` expands the
selected functions/files from the pinned 6,526-row inventory, runs every row in
its own pytest subprocess, validates the exact per-case JUnit identity and
builds one deterministic aggregate. `--resume-isolated` reuses only validated
per-case reports from the result's `.cases` directory. A missing, malformed,
crashed or outer-deadline report becomes an explicit failure/error row rather
than disappearing from the ledger.

Async and sync files are separate oracle execution lanes. Combining both in
one pytest process leaves the official sync greenlet loop marked running and
causes later pytest-asyncio tests to fail before their bodies. Candidate may
run both together because its sync facade owns a separate bounded loop thread,
but canonical oracle evidence preserves the upstream separation.

## Historical bootstrap runner

With `--bootstrap-only`, the conformance runner executes seven pytest node IDs
directly from the verified
upstream checkout. It does not copy or edit the test body. Oracle and candidate
run in separate virtual environments:

- oracle installs `playwright==1.58.0`;
- candidate installs only the built `shuozeli-pwright` wheel plus a test-only
  `playwright.async_api` namespace adapter;
- both use the same fixture plugin, attach to the same dedicated Chromium CDP
  endpoint, create an owned Context and Page, and run the same upstream
  assertions;
- `--noconftest` excludes upstream infrastructure fixtures that launch the
  bundled Node/browser stack; `--import-mode=importlib` prevents the source
  checkout from shadowing the installed oracle wheel;
- the shared fixture exposes only `PREFIX` and `EMPTY_PAGE` for this slice. Its
  URLs point at the exact pinned upstream `tests/assets` tree served through
  `serve-lib`; the pinned file hashes prevent those assumptions from silently
  drifting.

Placeholder annotation types in the candidate adapter exist only so pytest can
import tests before their execution outcome is known. They do not implement or
claim those capabilities. Actual candidate runs keep the upstream assertions
unchanged and record any placeholder or unsupported-method use as a failure.

For real candidate execution, a separate fixture adapter maps the official
`BrowserType.launch(headless=...)` session fixture to Pwright's dedicated
Chromium `connect_over_cdp`. It rejects other launch options and non-Chromium
engines. The unchanged upstream HTTP and HTTPS fixture servers bind only to the
specified Tailscale IP and publish the full MagicDNS hostname; this adaptation
is test infrastructure, not a Pwright public capability.

The independent oracle has the symmetric launch-only adapter: its official
Playwright Python `BrowserType.launch(headless=...)` fixture call is mapped to
that package's own `connect_over_cdp` against the same dedicated Chrome. It
rejects every other launch option and non-Chromium engine. This prevents a
bundled-browser launch from changing the comparison while preserving the
official oracle implementation for every exercised API and assertion.

The upstream sync server helper assumes the official binding's greenlet loop
is running on the test thread. Candidate execution replaces only that helper's
request waiter with a thread-safe waiter compatible with Pwright's dedicated
sync loop thread. It preserves the same path registration, request observation
and post-request value assertion; no test body or assertion is changed.

## Bootstrap-only official cases

1. `tests/async/test_navigation.py::test_goto_should_work`
2. `tests/async/test_navigation.py::test_goto_should_navigate_to_about_blank`
3. `tests/async/test_navigation.py::test_goto_should_work_when_navigating_to_valid_url`
4. `tests/async/test_navigation.py::test_goto_should_work_when_navigating_to_data_url`
5. `tests/async/test_navigation.py::test_goto_should_work_when_navigating_to_404`
6. `tests/async/test_page.py::test_url_should_work`
7. `tests/async/test_page.py::test_title_should_return_the_page_title`

Together they cover Chromium CDP attachment, Rust-owned Context/Page creation,
HTTP, `about:` and `data:` navigation, Page URL/title, successful and 404
Response status/`ok`, child-first Page/Context close, and browser detach. Python
only constructs stable facade objects from native descriptors.

## Verification

```bash
uv run --no-project python scripts/check_playwright_python_official_source.py \
  /Users/cyuan/projects/thirdparty/playwright-python

<oracle-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  oracle /Users/cyuan/projects/thirdparty/playwright-python --collect-only

<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python --collect-only

# Run async and sync certificate selections in separate processes.
<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --manifest packages/pwright-python/tests/official/client-certificates-manifest.json \
  --manifest-cases --manifest-api async --result <async-report.xml>
<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --manifest packages/pwright-python/tests/official/client-certificates-manifest.json \
  --manifest-cases --manifest-api sync --result <sync-report.xml>

# One bounded candidate execution shard:
<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --browser chromium --test tests/async/test_browser.py \
  --tailscale-ip "$TAILSCALE_IP" --tailscale-host "$TAILSCALE_HOST"

# The fixed ABI 17 async Page-route selection:
<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --browser chromium \
  --manifest packages/pwright-python/tests/official/page-routes-manifest.json \
  --manifest-cases --server-prefix "$PWRIGHT_OFFICIAL_SERVER_PREFIX" \
  --result <page-routes-report.xml>

# The fixed ABI 18 async Request-size selection:
<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --browser chromium \
  --manifest packages/pwright-python/tests/official/request-sizes-manifest.json \
  --manifest-cases --server-prefix "$PWRIGHT_OFFICIAL_SERVER_PREFIX" \
  --result <request-sizes-report.xml>

# Historical seven-case execution only:
<candidate-venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --bootstrap-only --server-prefix "$PWRIGHT_OFFICIAL_SERVER_PREFIX"
```

`PWRIGHT_OFFICIAL_SERVER_PREFIX` must be a verified full Tailscale MagicDNS URL
for a `serve-lib` registration of the pinned `tests/assets` directory. The
2026-09-23 clean-wheel checkpoint is oracle 7/7 and candidate 7/7. After native
artifacts were assessed, the final same-runner executions completed in 3.51
seconds for oracle and 2.63 seconds for candidate. A freshly installed oracle
also paused in its signed vendored Node executable's macOS provenance check;
those timings are environment diagnostics, not a performance comparison. The
extended cleanup gate creates and closes a Rust-owned Context/Page, then
observes zero session and FFI resources. Twenty warm attach/enumerate/detach
iterations increased the mixed differential Python process maximum RSS by
1,310,720 bytes from a 78,741,504-byte baseline; shared browser memory is
excluded.

The 2026-09-27 APIRequest checkpoint executes both complete global-fetch
files. Candidate passes 231/231 in one process. Independent oracle lanes pass
138/138 async and 93/93 sync. The candidate report SHA-256 is
`b9e1488ae018205350bb0d303c1cb8f637171e8a218c7252367f060d48434c90`;
oracle report hashes are
`d818d51b0f4f06d096ecad0ad2be055fad0f073928ef12a4c1f5643db5072af6`
and
`b4dc129b67f26377c83d38d3057d708ae057ad77f8a28c363f697e8ce618e413`.
The three-browser stop-idempotency and pending-request cancellation rows pass
6/6 candidate and 6/6 oracle. Their report SHA-256 values are
`b49022ec8e96c21404528a6325f7a629ffe927a8c8f3f39d6ca1b95ad8e2d872`
and `acbc68295177462d34d3ff9c373dc4ef9ec0bf1b8c13948e58153d7f2ecc9992`.

The ABI 11 client-certificate reports record four passes and two unchanged
upstream macOS WebKit skips in each async/sync process. Candidate report
SHA-256 values are
`4463e607226aa0c3af0cec2e3cae99577bace3ec80afc4ff4c285faa2eeb961f`
and `f5a7c2be118cd51d3f2d646bfcb2cc33f027adb82da2ff47fb7eac9538be2044`;
oracle report hashes are
`ca53d8e6e5eab175febf84aba266a6880fda2c8c6870785d14ae3ee4f8ccf96b`
and `ef9e1d79eb07d911c29eb85dde6c5a71f8e5df4db00de7e0e0060012a56d64ed`.

The ABI 12 init-script reports cover each complete nine-test async/sync file.
Candidate report SHA-256 values are
`43fc0287f84a6b789145d852feb5a123ab495730d332146d084c925dbee130f4` and
`8d4d8e0cc0d07c934fcda6633e96b689cabfcb70fa6dd43fc339b68b69f1dadf`;
oracle report hashes are
`451b9626c54141ede463d71d71b62bb7ed0bd24deaad93690e25862e2522a9bc` and
`61a7bd9a8abdc6b7194e5ef63c560eb66ddb881e21e8f3ff326350fdae0a2ba7`.
All 18 candidate and all 18 oracle invocations pass unchanged, including
Page and BrowserContext scripts, already-created and future Pages, path input,
multiple scripts and public error wording. At this checkpoint, the candidate ledger was
349 passed, 59 failed, five skipped and 6,113 not run, SHA-256
`49007c845eb0b27fe254d8e58c3374df0f6246642ba7e49e4423ba98c498ad6f`.
The oracle ledger was 266 passed, four skipped and 6,256 not run, SHA-256
`4d2cfb1af8920e6606c5211ba753e872fd693168e1507cd426e8242851b1be8a`.

CI must clone the exact tag, verify commit/digest/function identity, and perform
full candidate collection from the clean installed wheel. The current ABI 29
candidate collection is 6,526/6,526. The runner checks that exact count before
returning success; pytest's collect-only JUnit contains no executed testcase
rows and is not used as an execution ledger.
Real-browser
execution remains a dedicated-Chrome gate because the repository browser
policy forbids substituting an unrelated CI browser setup. CI is not yet wired
to the full execution matrix.

Current candidate execution accounts for 445 inventory records. The complete
async/sync global request files, six browser-independent stop/cancel rows, 12
certificate rows, two init-script files, the Page shard, two request-GC rows,
seven JSHandle rows, 11 dialog rows, two Page-error-history rows and two
console-message-history rows, two request-history rows, all three expanded
device-descriptor rows and the applicable Chromium listener row remain
explicit in the ledger. The Page-close terminal
waiter is also recorded as an exact replacement inside the Page shard. The
init-script files pass 9/9 async and 9/9 sync. The async
browser file passes 4/4 after `Browser.new_page` gained Rust-owned implicit
Context lifecycle and compensation; its sync counterpart passes 1/1. The
async base-URL file passes 7/7 after relative navigation URL construction,
relative route-pattern matching and installed persistent-Context launch moved
into the Rust host. The nine-test URL-wait file passes
9/9 through the
optional Rust URL-wait and set-content capabilities.

The complete 125-case `tests/async/test_page.py[chromium]` shard initially ran
without runner or fixture errors at 67 passed, 57 failed and one upstream
FIXME skip. Rust-backed locator actions/reads, selector evaluation,
same-document URL publication, published Frame-tree lookup, optional bounded
media and timeout state, plus the subscription-driven network-event leaf
accounted for 51 passes over the first clean 13/125 baseline. The unchanged
upstream request/response/timeout/close subset passes 14/14.

ABI 13 then reran all 18 script/style-tag rows through fixed candidate and
oracle manifests. The deterministic JUnit merger replaces exactly those cases
in the Page report rather than double-counting them. The canonical Page report
is now 85 passed, 39 failed and one skip, SHA-256
`f784b1b673aabd303f26158528091c27506cb1cd06256fe5eb5c5c9bf6b3c66d`.
Remaining failures stay visible around non-network Page events, routing,
bindings, request-size metadata, drag, error-text parity and beforeunload.
Three Page-fill discrepancies also remain corrected: Rust uses the official
invalid-target capitalization and element labels, and Python maps non-string
fill values to the public `Error` text before PyO3. The historical pre-ABI-13
complete-file report SHA-256 was
`53848c57654b5d3397edae116701e77b75a65f9bb53b9a44aac14b835c841c48`.
Those same three nodes pass 3/3 with the official oracle through its symmetric
CDP launch adapter, report SHA-256
`7cdef633380f69c0fb2043e93ea42b16c2dbb1c39db28c3fe71ce047feb821a8`.
That historical full oracle diagnostic is deliberately not merged: its final
`page.pause()` rows blocked because the shared visible Chrome was not
equivalent to the upstream fixture's `headless=True` browser. ABI 23 supersedes
only those two rows with a task-owned headless lane.

An ABI 13 full-file diagnostic is also excluded. The known beforeunload gap
placed that candidate attachment in terminal state, so its subsequent setup
errors were cascading rather than testcase outcomes. Only the clean fixed
manifest results and deterministic replacements enter the ledger.

ABI 14 then reran 12 Page-binding rows through its fixed manifest. Candidate
and independent oracle lanes pass 12/12, with report SHA-256 values
`85de412561f1139288706adfb2969e8a9c5685ea66c6eba3ab6b54ee0e7d1920`
and `65cc682d7e21a35d41d3a5a2c36e745e1f7215ad14d7d69dd14f2be4de29c16b`.
Exact replacement produced a 97 passed / 27 failed / one-skip Page report,
SHA-256
`608d1d0940997edfb312f01d4f580b14d1148ed4bf18404c5058d60de217b641`.

ABI 15 reran six Page lifecycle rows for load, DOMContentLoaded, close and
opener behavior. Candidate and independent oracle lanes pass 6/6, with report
SHA-256 values
`27c46dc49101b3047f5ece6b57c774319576f1fd47f714fa16412028ac9a9081`
and `a1b2e6080dcb7e406c2a9a903d9d24340a0edddd0ff00a4547e8c640e2a0e621`.
Exact replacement produces the current 103 passed / 21 failed / one-skip Page
report, SHA-256
`672e39ff6b08068e4470386a0e70c925d7b52de8c1aa091bd0cb042c7b6a0c41`.

ABI 16 reran five Page-error rows. Candidate and independent oracle lanes pass
5/5, with report SHA-256 values
`28cd246e9902b1b36494b7e6ffbee0b8fede241426b834e455e2e85f10f2548d`
and `e341777d61bc4d02dd11d8e4233103202cf55905ebcb658902020a618288cc2f`.
Exact replacement produces the current 108 passed / 16 failed / one-skip Page
report, SHA-256
`1414096c00e3905b6d0f080e8357eb6fa2cd7b23047125cfd16c2b24bceb52d2`.

ABI 17 reran five async Chromium Page-route rows from the final repaired wheel.
Candidate and independent oracle lanes pass 5/5, with report SHA-256 values
`b8da7c43b37f0c7cabd9cd740348b907fe20ad4899b749d821c72818f6d3b3c3`
and `eea2184bd73833dbb532c3ba252e7259c3b918d06b8cdcbc8c92d1bc42546490`.
Exact replacement produces the current 113 passed / 11 failed / one-skip Page
report, SHA-256
`188e6bcd09ae3dad589a24e1d2d1632ad8d01424cdb65bfc75e1f5308e93da30`.

ABI 18 reran the two async Chromium `Request.sizes()` rows from the final
repaired wheel. Candidate and independent oracle lanes pass 2/2, with report
SHA-256 values
`6506a861160f462eca4b6b104b0f8c9a11f7307b1518c1a592ed9d1804b8aa47`
and `bb71eed6453fe9c8d2fc14563877804f44893df23e227cf2c8f6b01ade66de51`.
Exact replacement produces the ABI 18 115 passed / 9 failed / one-skip Page
report, SHA-256
`07cf64b7c9cf428c8c95daf6fea98a47d17a0d33ca92c60c19b9cf8eaaae2e0c`.

ABI 19 reran four async Page and two async Locator drag functions plus one
sync Page and one sync Locator function. Candidate final-wheel and independent
oracle lanes each pass 6/6 async plus 2/2 sync. Candidate report SHA-256 values
are `3069011993af14bceb5f2d367f23227bd5dbae727d8b1264617ac2acde1ed2f9`
and `4287ae3d14813d32edccaa4ae422581f141745a85e06037dbed805313d177f8f`;
oracle values are
`5ef118e81c96770c699f817e464db6151798e7445306c504ceea5cb339347c07`
and `dcf390058a973cbf86ded8efef0c03cdc20e90d2a823912376f6110e0d0e381d`.
The checked JUnit selector extracts exact expanded node IDs before merge. Four
Page replacements produce 119 passed / five failed / one upstream skip,
SHA-256
`ab5d9e713f0e465c008e004aa06ac82bab218362230de2779d797ea230700d2a`;
the other four rows were previously not run. The remaining seven candidate
failures are explicit: two pause rows, two beforeunload/dialog rows, one
disconnect waiter, one base-URL route/fulfill row and one persistent-context
row.

ABI 20 reran eight async Dialog functions, two async Page beforeunload
functions and one sync Dialog representation function. Candidate final-wheel
and independent oracle lanes each pass 10/10 async plus 1/1 sync. Candidate
report SHA-256 values are
`80c253a841995ba47251a9a3cb5040479573a12cc2bbf87de378db8569c1086f`
and `746df62a98ab2ccb722a36607e6864a05ed1bcbfd5059f4947fe1b8256162b13`;
oracle values are
`dd8f414e0e9c1e38865826ce8d7b165f77c8de9c18ec5254aba16aadedef3718`
and `b0a1b0e3aa68ca2c92fea9c856364f47195933cd63e1e345e969fd325d9d450e`.
Exact replacement converts the two Page failures to passes and produces 121
passed / three failed / one upstream skip, SHA-256
`70eef0e767ad5b3385d9bdbcd9b2a71e5ebbf998827a25070e76f805f95cfd36`.
The remaining five candidate failures are explicit: two pause rows, one
disconnect waiter, one base-URL route/fulfill row and one persistent-context
row.

ABI 21 reran the exact async Chromium relative-base-URL route/fulfill function
from the final repaired wheel. Candidate and independent oracle lanes each pass
1/1 with unchanged assertions. Candidate report SHA-256 is
`e8d9a3f803797cfff3d914bb47866a4ecda4d92888a7ddd3f41ad87179469f6b`;
oracle report SHA-256 is
`e8b3e46a074804cb92f50fd2a7f173bac76f570daa31b30df506a1fc6d14fde6`.
Exact replacement produces a seven-case base-URL report with six passes and
one failure, SHA-256
`3e465814e57aa12de0c535441e1597a7134a871f25ac4eaeda6e6cc8c75251fb`.
The remaining four candidate failures are explicit: two debugger-only pause
rows, one disconnect waiter and one persistent-context base-URL row.

The ABI 21 terminal-waiter extension reran the exact Page-close function from
the same final repaired wheel. Candidate and independent oracle lanes each
pass 1/1 with unchanged assertions. Candidate report SHA-256 is
`03a028943034b7cbf6962c34e99cc68be07878b9463dec831e46f6d4fe2e7872`;
oracle report SHA-256 is
`48b06660ac0af81f99edb46372a6ba6f6058fbc930b497eefcefff3ad387bd53`.
Exact replacement produces a 125-case Page report with 122 passes, two
failures and one upstream skip, SHA-256
`da99fb0ac032a12a9507c5467b257cd38349932bb0c6d707172d86c11fe79fb3`.
The remaining three candidate failures are explicit: two debugger-only pause
rows and one persistent-context base-URL row. Successful download events and
artifacts are not claimed by this terminal-only checkpoint. Python retains at
most 64 public Page-event waiter/predicate identities per Page and rejects
further admission before allocating another Future.

ABI 22 reran the exact async Chromium persistent-Context base-URL function
from the repaired wheel. The fixed manifest passes 1/1 in both candidate and
independent oracle lanes with unchanged assertions. Candidate report SHA-256
is `57c6a49de8942866cae9f67b480cae5c553ecb672606fce14f3370a5396ff862`;
oracle report SHA-256 is
`0b68621888607947312b52b2341c7e7c366c7312fc3d64b7f068a063026d86ad`.
The candidate's complete seven-case base-URL source passes 7/7 without a
channel override, report SHA-256
`9723e98d9361ab85d663479bf02015ff7834d878783a6f0e8cd9e32b5f723c6a`.
This closes only one row; the other 21 collected
`launch_persistent_context` invocations remain unverified unless another
recorded result already covers them.

ABI 23 reran both exact async Chromium headless `page.pause()` functions from
the repaired wheel. Candidate and independent official-oracle lanes pass 2/2
with unchanged assertions. Candidate report SHA-256 is
`7ffe205309fc5864bf6c9acae98c4e05a1b3d99be3e72fd3569a0090e718ad2c`;
oracle report SHA-256 is
`1c5d32e8553abefefa9fa7b52b6a3cc6d3df0a7adedf21f80cc667e09cd69ff5`.
That two-row checkpoint initially completed a merged Page report. The fresh
full-file evidence below supersedes that merged artifact.

## Full Page isolated checkpoint (2026-09-27)

The complete pinned `tests/async/test_page.py[chromium]` expansion was run with
`--isolate-cases` against the repaired installed wheel and independently
against official Playwright Python 1.58.0. Every one of the 125 rows has a
validated per-case JUnit report. Candidate and oracle each record 124 passes,
the same upstream FIXME skip, zero failures, zero errors, and zero status
differences. Candidate report SHA-256 is
`709000c785522a979b7f56b344216a9ed0a4f8b9d783ffc725dea8e15c9ba9f3`;
oracle report SHA-256 is
`d8058d713c4cc1fcefbbab295b4eb9aa5647dc98eb9f8453a8e877319be7dade`.

The discrepancy was in the Rust Page-route owner: topology reconciliation
removed a pending intercepted request when its Page closed, so the handler's
subsequent `route.continue_()` incorrectly failed as already handled. The
owner now retains at most 64 resource-accounted terminal receipts. The first
continue, abort, or fulfill after Page retirement consumes its receipt as a
successful no-op; a duplicate action still fails. Focused AAA tests cover Page
retirement and single consumption.

The rebuilt 6,526-record candidate ledger is 428 passed, five skipped and 6,093
`not-run`, SHA-256
`a8b7ce8ed00ed75676cad32d68c6487685839b8fae29ba8f1b79e5cbd0689980`.
The independent oracle ledger is 409 passed, five skipped and 6,112 not run,
SHA-256
`c0858c03e07687bd7786d5f713b548fcbc763b9028b41af50579a5e3384c7755`.
The candidate count is unchanged because all Page rows already had historical
candidate outcomes; their evidence is now a fresh current-source full-file
run. The oracle gains the Page rows not previously executed in its selected
manifests. Neither ledger contains duplicate rows.
The historical final-source ABI 15 repaired-wheel archive SHA-256 values are
`b9fae36315280fbc80da0178d738fdca9485c1baa6263fe6c74bc557481a98f8`
and `e1c1fc914fdad0fb1af78dfc9364056d8b72c2d87012426c1a4c938cbba259c6`;
their payloads are reproducible after normalizing only CycloneDX identity
fields, and the installed first wheel passes 13/13 local tests.
The final-source ABI 16 repaired-wheel archive SHA-256 values are
`aab4fcd451491ff9199c43c064b5c09c94ab60fd1f656f9cb4fdaf2ba569f8e4`
and `65d3eb6ca1cd0553c384c1165d19806181eade685900920a3ba288b2fd2d710d`;
their payloads are reproducible after normalizing only CycloneDX identity
fields, and the installed first wheel passes 13/13 local tests. Strict Ruff,
Pyright, feature-cold and selected-feature Clippy, Rust formatting, workflow
parsing and exact dependency gates pass.
The final-source ABI 17 repaired-wheel archive SHA-256 values are
`440691bb254993669babecc422be9eac6d1cdbec3d91d7b314524c967fa3289c`
and `4527aee3a74b4c3f8b865500dc1f90e4365df36325f28501db0b5c536943a6fa`;
their payloads are reproducible after normalizing only CycloneDX identity
fields, and the installed first wheel passes 13/13 local tests. The final-wheel
official route lane is 5/5 and leaves no task tab or serving route.
The final-source ABI 18 repaired-wheel archive SHA-256 values are
`ac314f048915df9f7f8a2303a4d88eec8c353d29b0821b961e5b241e9e4db0a9`
and `2bb81c1e7f96e0d32f4f4ce346075ab4c125be481aaa175971ad794e2d5954b1`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 13/13 local tests, recollects
6,526/6,526, and the final-wheel Request-size lane passes 2/2.
The final-source ABI 19 repaired-wheel archive SHA-256 values are
`0766475c32dbc5f86128c57347c4311caf505a069d012699ada916b3d035255f`
and `99a5ca07d5e9e0e628a9fceac1c2db2743b289926f007f915bc4c80723ddca3c`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 15/15 local tests, recollects
6,526/6,526, and the final-wheel drag lane passes 8/8. The selected drag leaf
does not yet claim full stability/actionability/hit-target retries, force mode
or Firefox/WebKit behavior.
The final-source ABI 20 repaired-wheel archive SHA-256 values are
`4c057a55597213a7842caa670c27bd38643811df30e6282a7a8229d3509b9d27`
and `68bf970bb1ed3480ccc66c1b6a116c7fe2367fe48318c28edd85da22ad847c04`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 15/15 local tests, recollects
6,526/6,526, and the final-wheel dialog lane passes 11/11. Firefox/WebKit
dialog behavior remains in the ledger as not run, not as a hidden pass.
The final-source ABI 21 repaired-wheel archive SHA-256 values are
`015f986890fe67d01c70401206961d623fafde3dcf587d70a4938833d16ed216`
and `f2a99efc425981fa515fa2e6e51e92f9fcd1c935a5b9bbb17567ed260346a6f9`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 19/19 local tests and recollects
6,526/6,526; the collection artifact SHA-256 is
`07ec6afbf0e47c4860e05e7650d0900016c3806a4255b8df81b05f83e6fe4a86`.
The final route-fulfill and terminal-waiter lanes each exit cleanly after 1/1.
An earlier development
run passed the assertion but hung during teardown because the driver task kept
an extra route-controller `Arc` across its transport join. The host now drops
that task-local owner before awaiting I/O termination; diagnostic reports are
excluded from the canonical ledger.
The final-source ABI 22 repaired-wheel archive SHA-256 values are
`449735fcd3e26786396a2bb101483ec57f4533408dc2b96aa0b25e819e9ca7c7`
and `8acfaa118c5d633198161a12a23d71c143e1390659ae5fcf400d640cdd001743`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The first wheel vendors its OpenSSL dylibs, passes 19/19 installed-wheel
tests, and recollects 6,526/6,526. The collection artifact SHA-256 is
`102dc195db37a1f6d0c54f5846448a85b7ffd3701d1ea5165ab489135cfa39eb`;
collection is inventory coverage, not execution evidence. Native launch
accepts only an empty or already Pwright-marked profile and shutdown reaches
zero owned process/connection resources.
The final-source ABI 23 route-close-repaired wheel archive SHA-256 values are
`a65ac8096484f11a1a71c1b7e13dddbb88b6300c764a16648fd94dc9fc3a95fd`
and `f42c71803b4fd925409c39d0f6bb35055738f9a555826434795e46112ce3fc7e`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The first wheel vendors its OpenSSL dylibs, passes 20/20 installed-wheel
tests, and recollects 6,526/6,526. The collection artifact SHA-256 is
`5aa8de7c224b57c35c3eebd0f38030863bdf63a03b843be339d140d6e0106661`;
collection remains inventory coverage, not execution evidence. Before the
product fix, a broad Page diagnostic exposed the route-close hang and
cascading failures after signal timeout; those non-isolated results remain
excluded from the canonical ledger. The isolated-runner regression records
that historical case as one 15-second failure and the following title case as
a fresh-process pass. Its two-row aggregate SHA-256 is
`4899c51e6a63307f5ff89884869b50ab054437264c6e64fd2b273568c984c885`;
resumption validates and reuses both exact case reports. The repaired full
Page candidate/oracle reports above are the current product evidence.

ABI 24 pins both complete request-GC files in
`packages/pwright-python/tests/official/page-request-gc-manifest.json` and reruns
their exact unchanged functions. Candidate and independent official-oracle
Chromium lanes pass 1/1 async and 1/1 sync. Candidate report SHA-256 values are
`f4a28daac38c4c122401c171e71e157852ebab003c7dc4c7dae04f8ab454053f`
and `33dcc10908cfb8923d4b9fbc1dfcfd6639e9f93897515d9b8458d2d06b38e1ae`;
oracle values are
`7bba1bcbf3dc9f90736279a394932fb1bf9b0761f45d7ef2620a8b504a03f81d`
and `b800a26881354e55a9b465a3a67db8db565db5755e2b3e66c6dd49a016699b47`.

At the ABI 24 checkpoint, the 6,526-record candidate ledger was 430 passed,
five skipped and 6,091
`not-run`, SHA-256
`fce03db46415b6bed7e1d97484aec3abbcbfa9b3fa051a52d1ef73eccf659241`.
The ABI 24 oracle ledger was 411 passed, five skipped and 6,110 not run,
SHA-256
`d0bf59a7f917f8c19d6be9064ebc41e295825d8825459e4fda29e7b717f3fb71`.
The final ABI 24 wheel archive SHA-256 values are
`d9a43bd387517005d0248e85ccd7082ba4f2fe356594a8e28df9af5e7403a000`
and `9c572ea47613ad581e5a5e14f2b6cea4ee6fd04a3fc7aec813b1977ebeabff99`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 21/21 local tests and recollects
6,526/6,526. Collection XML SHA-256 is
`22029eb6ad67012f47a8db33a1be3d55ed08aed2646269840d663edfbfd5e91f`.

ABI 25 pins the unchanged async and sync
`test_page_errors_should_work[chromium]` rows in
`packages/pwright-python/tests/official/page-errors-history-manifest.json`.
Candidate and independent official-oracle lanes each pass 1/1 async and 1/1
sync. Candidate report SHA-256 values are
`bb61ff804068782deec28497cd7a7a9375d6f2d175a309c202f73d12a38f8ec9`
and `23f9308f0801de4f988c777b1e2004bff06522178e6d8a9be3f0d726c3e65a37`;
oracle values are
`bb61ff804068782deec28497cd7a7a9375d6f2d175a309c202f73d12a38f8ec9`
and `25e87e795543447f398a2967796dc2f9bed3dd97f3574c85c5d70081e85a0034`.

The 6,526-record candidate ledger is now 432 passed, five skipped and 6,089
`not-run`, SHA-256
`6c3d2dad4271da574f924a305cadcb8aeb801e4650079249e524df02be0e1293`.
The oracle ledger is 413 passed, five skipped and 6,108 not run, SHA-256
`59aad3d302b5c05d06a3a3fe8898ee881076658ab8b0bb9edc5de2ef91e5e00d`.
The final ABI 25 wheel archive SHA-256 values are
`a15dda04b51b2f05875fdb700d03e54014b411c6d80fd1d37d8aff8e44ebbe4c`
and `3b3498b1fa79fd3168ee53a87ee8adb3eda45bb68bd5ef0010089cfe809b20bf`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 22/22 local tests and recollects
6,526/6,526. Collection XML SHA-256 is
`30d96008e1afb4ade52b1bc2aaae4e10838b97e1f024d63e2e9936b5b821743e`.
The remaining 6,089 candidate rows are explicitly unexecuted; this is not a
full-suite pass.

ABI 26 pins the unchanged async and sync
`test_console_messages_should_work[chromium]` rows in
`packages/pwright-python/tests/official/console-history-manifest.json`.
Candidate and independent official-oracle lanes each pass 1/1 async and 1/1
sync. Candidate report SHA-256 values are
`95c417b6bdcaf56255146a92374fd0c95cdc2828cf9f746e416661175bf320fb`
and `2298b65eb48feb4292db52d2ce9258aa7d2e79308189485d6e25bb1932050be6`;
oracle values are
`5978e92a8f7862bd3781ad491deaabfc3d66264157dda652b44b37bc23435a02`
and `c5bd148d8e79655d955a5b33783d60310b368213dd9a5b8f6fd3029cb659c722`.

The 6,526-record candidate ledger is now 434 passed, five skipped and 6,087
`not-run`, SHA-256
`9445e7a0c6d424cee6b604a8fc43804ddb4304ab4d97f1eac7371e3398e4d41e`.
The oracle ledger is 415 passed, five skipped and 6,106 not run, SHA-256
`354fadad72ee68e599644973aa648e941c4619499f3b0a12b30049441a080773`.
Final ABI 26 wheel archive SHA-256 values are
`0e84de688da9c80f1be5ad0b04bba89b4782643bd112a5bf3830b80ba49ee9ed`
and `4ba5129f7df145b45dfae0b56a4ad406d40a812ef5492a27d732d5c034b4a8ff`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 23/23 local tests and recollects
6,526/6,526. Collection XML SHA-256 is
`28d4b289870ab589f86c153e95bcf1a3f44bdb2f9c63dbacb3b9ca5564467797`.
The remaining 6,087 candidate rows are explicitly unexecuted; this is not a
full-suite pass.

## ABI 27 request-history checkpoint (2026-09-27)

ABI 27 pins the unchanged async and sync
`test_should_return_last_requests[chromium]` rows in
`packages/pwright-python/tests/official/request-history-manifest.json`. The
manifest records async source SHA-256
`f0e5eef63b0d9707bc622e06453869e3d29c093398b4e1c52378c79ca2920435`
and sync source SHA-256
`2206d2fcec8f038c87803157be86f7e0e4bbf183be9ac3b8909b0f8755d87bb5`.
Candidate and independent official-oracle lanes each pass 1/1 async and 1/1
sync without changing the upstream assertions. Candidate report SHA-256 values
are `cb01e3973a1f9a0235bc5954b70766271cf369013a2203170f7cd6a302a8ac45`
and `f30f8e87a722844dbb552133faa52fc1a836f4db8ce25073cb333cda0506aaae`;
oracle values are
`3ea6ac97cf691a63cf736784434e2221e3f47d8690fa289d66cd135331e53ec6`
and `363f49ae1e654259ac5f94e9df72d9f7610281fa304a3de1f1287d14b3986d4f`.

The 6,526-record candidate ledger is now 436 passed, five skipped and 6,085
`not-run`, SHA-256
`2fb546a564e528dfc3788466aae16bb6fc556163e950eff2f26df2bd90e4042e`.
The oracle ledger is 417 passed, five skipped and 6,104 not run, SHA-256
`9ba7a572ab3c66a18b0c409ba09ac4b32c23db36f84d9c73c3be6db7b6040a4c`.
Final ABI 27 wheel archive SHA-256 values are
`6eb888eb3a453e7d1894fbaeffd43ba7fab9ad803cbb2535ccf38645ef2cedbb`
and `8780570e0e93f4dfc9bab79013769b77a31a8c1b340de288484cf35a0463b03a`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 25/25 local tests and recollects
6,526/6,526. Collection XML SHA-256 is
`7376edf2b4493552e0d5b2bedad0969d31ff01c6fe5ad3306a9feaac0acd5be0`.
The remaining 6,085 candidate rows are explicitly unexecuted; this is not a
full-suite pass.

## ABI 28 device-descriptor checkpoint (2026-09-27)

ABI 28 pins the complete unchanged
`tests/async/test_device_descriptors.py` source in
`packages/pwright-python/tests/official/device-descriptors-manifest.json` at
SHA-256
`0ce770e824526029a38991b7bad0ce6176dac8eb2593faf793252a394411c560`.
The file expands to three browser invocations. Final-wheel candidate and
independent official-oracle lanes each record the Chromium pass plus the same
two upstream `only_browser("chromium")` skips. Candidate report SHA-256 is
`e73a060a2ae5c4991c91bfb16f18c76cbad10a970754d094473aa11700edecae`;
oracle report SHA-256 is
`af89d5aeb07177c83bca47e491c99e28b9abc12b31006bea06d5463fcb8be65c`.

The 143-entry embedded catalog is semantically equal to the pinned Playwright
v1.58.0 `deviceDescriptorsSource.json` whose raw SHA-256 is
`ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353`.
The 6,526-record candidate ledger is now 437 passed, seven skipped and 6,082
`not-run`, SHA-256
`c02c13e38b6b3dae4d83ec8ad280088dabd7432c30b70ecfeb83e83bf9e5cfc3`.
The oracle ledger is 418 passed, seven skipped and 6,101 not run, SHA-256
`a60219448e4b669fd5c3520186e93579b2e3fea6692492d483c0845a7fd3c644`.
Final ABI 28 wheel archive SHA-256 values are
`42a1d574c079f5c1f890ef6e44272ffccf91ee532e9ed3d17f3ae126552d7273`
and `5f36e05126c219480c62d425e3da79ff3d6fc7180c42efa4e2fb35e7f97f321a`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The freshly installed first wheel passes 26/26 local tests and
recollects 6,526/6,526. Collection XML SHA-256 is
`45baee9da8d37c09e052db9e760a7e65b605bd868fedce562c1447a1c6ec393d`.
The remaining 6,082 candidate rows are explicitly unexecuted; this is not a
full-suite pass.

## ABI 29 response-listener checkpoint (2026-09-27)

ABI 29 pins the complete unchanged `tests/async/test_listeners.py` source in
`packages/pwright-python/tests/official/async-listeners-manifest.json` at
SHA-256
`dbda3cea63f1820b2d0ac4587e9bcaf938c43c318ca815294f00bcabbbbba4a9`.
The final installed-wheel candidate and independent official oracle each pass
the applicable Chromium invocation 1/1 without changing the assertion. The
candidate report SHA-256 is
`31ffbee02631a56fecb4e5ffa387db13317445f6b10656e939d9865bd98c9261`;
the oracle report SHA-256 is
`9fee5338e25b6927933343356e88f4ff5fafa5b01fc67e02866d79bf7b1fcf8a`.
Full-file diagnostics continue to expose Firefox/WebKit as missing providers,
so those two expanded rows remain `not-run` rather than being reclassified as
passes or skips.

Failed candidate attempts are preserved outside the green ledger. They exposed
sticky topology event loss followed by a false Page disappearance when a live
Page temporarily lacked a command-session route. The final host uses one
authoritative same-connection reconstruction with one monotonic epoch advance,
and observes materialized Page membership independently of session routing.

The 6,526-record candidate ledger is now 438 passed, seven skipped and 6,081
`not-run`, SHA-256
`382e3c5a4fa98f8bd22e9503066d21a0236ded7f7f823a10b8e7ee3d17e439c2`.
The oracle ledger is 419 passed, seven skipped and 6,100 not run, SHA-256
`a0d41d0fe578361d80998cc60cdc25b7c07edb44e455061eccf64640785ec13f`.
Final ABI 29 wheel archive SHA-256 values are
`05bf16e458ed4a68dd669e99b50b56f7792602c4717548ecf73ce630c3c85b0f`
and `0be5b154d2c3647f63fb7ab5a66b69908009f41f28968d1d4acd16177ac9b234`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The freshly installed first wheel passes 28/28 local tests and
recollects 6,526/6,526. Collection XML SHA-256 is
`d9fb0c10e98f170e45b614d56d02a6605d3ffdb14dbfd5f71b02e662c8bcaa0e`.
The final Chromium lane passed while the unrelated shared-Chrome application
tab remained open and untouched.
The remaining 6,081 candidate rows are explicitly unexecuted; this is not a
full-suite pass.

## ABI 29 sync-listener completion checkpoint (2026-09-27)

The public native ABI remains 29. This checkpoint pins the complete unchanged
`tests/sync/test_listeners.py` source in
`packages/pwright-python/tests/official/sync-listeners-manifest.json` at
SHA-256
`24c14d60da2cc007ca29c68de83982644b3a82320249145953e0a7125fa88658`.
The final installed-wheel candidate and independent official oracle each pass
the applicable Chromium invocation 1/1 without changing the assertion.
Candidate report SHA-256 is
`98f92bdeb7672c3511bc08d52a6c1f5140fb044bcd511f8f7bca584bcc1e9e05`;
oracle report SHA-256 is
`0e28ea198745648d83b047914d4241ae02d4bcb0f2861129ddecf1895ada04c7`.
The full-file candidate diagnostic, SHA-256
`1e5f7f58a6cc14e774b0de1027d500d8aaf9fe97217ded561275d97c2db3fdc1`,
records the Chromium pass plus explicit Firefox/WebKit provider errors. It is
preserved but not merged. The async listener regression also passes 1/1 on the
same final wheel; report SHA-256 is
`d87385340c9481be56d0b0f8fd4c8ce7295dea83bca0819d3b626d22a2754418`.

The first attempts exposed that a reconnect-safe Page identity also needs its
live network adapter preserved until the authoritative snapshot finishes. The
final host holds those lanes during reconstruction, reconciles their live CDP
sessions after `finish_reconnect`, and releases only Pages absent from the
completed snapshot. The CDP event replay window is explicitly bounded at
2,048 entries for the declared 16-Page/128-session/256-world profile; recovery
still performs one reconstruction and one epoch advance, not a retry loop.

The 6,526-record candidate ledger is now 439 passed, seven skipped and 6,080
`not-run`, SHA-256
`4da01fe20cf788ee1a12f28509708166ed31b5f3eb3cefd78672ba4cddc3ffee`.
The oracle ledger is 420 passed, seven skipped and 6,099 not run, SHA-256
`f654d55f1bb21a0382a2bca2636438070a0c14e3b92cdcd1d7c2e643b6f9ea90`.
Final wheel archive SHA-256 values are
`f2eea005f003fa036d8cb6e0e2d2080066fec5e01f92d2c460098181dccd5f96`
and `91a51dd7db6c5b9d068ed47064f0de520fe66fd2995d8c52904cac538dd6a14a`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed first wheel passes 28/28 local tests and recollects
6,526/6,526. Collection XML SHA-256 is
`839fff4356fd0b066257194825f1e9547a5eeb31336f8195da6324cfb7815547`.
The remaining 6,080 candidate rows and the Core JS/TS execution are explicitly
incomplete; this is not a full-suite pass.

## ABI 30 collection selector evaluation checkpoint (2026-09-27)

ABI 30 pins the complete unchanged `tests/sync/test_selectors_misc.py` source
at SHA-256
`940c7576586bc22381f260de40349d8ac3632ddb05ff0415b1af5ed3beeaf532`.
Its exact Chromium invocation passes 1/1 in the installed-wheel candidate and
1/1 in the independent official oracle, preserving all six upstream
`eval_on_selector_all` assertions. Candidate and oracle report SHA-256 values
are `ee4fb9da609e9296d3d6ab2ee20a2dd07addcad2c39ae5e954e7e930e498ed3f`
and `1a81025077242fd922719234d3e557d4a6a8aee3e75014f3a60b0e94f30e975b`.
The async counterpart and Firefox/WebKit expansions remain explicit
`not-run` rows; they were not silently removed or reclassified.

The candidate first needed a bounded topology-ingress repair: the connection
now admits 8,192 event items while retaining the previous 512 MiB aggregate
payload ceiling independently. It then exposed the actual missing API. ABI 30
adds thin async/sync `Page.eval_on_selector_all` facades while Rust owns the
typed `internal:and` intersection, collection resolution, utility-world
evaluation and value conversion. Two final wheel payloads are reproducible;
the installed wheel passes 29/29 local unit tests, both listener regressions,
and 6,526/6,526 collection. Collection report SHA-256 is
`73c16a52febce9c58f1a5b13d5537f9fa16958de095386084a4d0fb56b2c5a12`.

The candidate ledger is now 440 passed, seven skipped and 6,079 `not-run`,
SHA-256
`65bd9d4261400421fa079ceea93780052d25c11b19e92a3ad8d2ea25fa75862b`.
The oracle ledger is 421 passed, seven skipped and 6,098 not run, SHA-256
`c4f10b1c972140a3414e48902f610e4a99f8b18384daac925bad89bd9e6db116`.
This closes one official invocation, not the complete Python corpus.

## ABI 30 async selector regression checkpoint (2026-09-27)

The independent async source is pinned in
`packages/pwright-python/tests/official/async-selectors-misc-manifest.json` at
SHA-256
`44746147b5adaf40bdd879468b34d9fb2d31b4c64d3a397c9c94a47961b41b0f`.
Its exact Chromium invocation passes 1/1 through the installed ABI 30 wheel
without changing any of the six upstream `eval_on_selector_all` assertions;
the official oracle passes the same row 1/1. Candidate and oracle report
SHA-256 values are
`e6fa55d7b30db92b8d1e2fe3618f75157af502f461711f7df0ed7d7d452f9c4f`
and `c989503c090d5c1e36bcf45f6b4302c6eb051c509f6592c5cf71060ac449bb71`.

The 6,526-record candidate ledger is now 441 passed, seven skipped and 6,078
`not-run`, SHA-256
`39d6bda545c2f7ce3a21252ee63903b8624732de1fbb29aaaaab6133259d0c86`.
The oracle ledger is 422 passed, seven skipped and 6,097 not run, SHA-256
`318ff5c4cfdb01fdeb833c027fed4ead7765dc32fad7d881f8512a01f78e7be8`.
The Firefox/WebKit expansions remain `not-run`; this checkpoint proves async
binding parity for one capability and is not a full-suite pass.

## ABI 31 selector registration checkpoint (2026-09-27)

`packages/pwright-python/tests/official/selector-registration-manifest.json`
pins the complete unchanged `tests/async/test_selectors_misc.py` source at
SHA-256
`44746147b5adaf40bdd879468b34d9fb2d31b4c64d3a397c9c94a47961b41b0f`
and fixes exactly
`test_should_throw_already_registered_error_when_registering`. The Chromium
expansion passes 1/1 through the final installed ABI 31 wheel and 1/1 through
the independent official oracle. Candidate and oracle report SHA-256 values
are `0b39eedf5fb3296af525cefe3c727c2a945153c9f1a03c7b9902b1778140797c`
and `74a1343156cf5890c4bfa73e49c71a71197c4672bf574d7c74c7b4fc546da24d`.

The source imports upstream-private types for annotations and error identity,
but the unchanged assertion exercises the public
`playwright.selectors.register` contract. It is therefore applicable public
behavior, not a file-level exclusion. The initial ABI 30 setup failure and the
first ABI 31 shim-collection failure remain preserved as diagnostics and are
not merged into the green ledger.

Rust owns registration state, duplicate/predefined/invalid-name errors and
teardown. It admits 64 engines, 64-byte names, 1 MiB per source and 8 MiB
aggregate retained bytes. Python retains no registry; `path=` reads are capped
at the source limit plus one rejection byte before the native call. This
checkpoint does not implement or claim custom selector-engine browser
execution.

Final wheel archive SHA-256 values are
`8500f8303647d918de19c72763e4095ba69a12117b016173a8a6060eb502aaf9`
and `5ef86dea2cff30a84501d0e47242c496a4375f23fb18aac2b5af607c990f3126`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed wheel passes 32/32 local unit tests. Four release Rust
registry tests, strict Ruff, Pyright, feature-cold and feature-selected checks,
exact wheel-profile Clippy and the 108-package dependency proof pass. The full
candidate collection remains 6,526/6,526; its XML SHA-256 is
`c718815c1ab71842546b1b088046088ab6ef51a39466cbe5a227713e66dd2edb`.

The candidate ledger is now 442 passed, seven skipped and 6,077 `not-run`,
SHA-256
`80a4c3855a5a064ef26c467eae02b50b960e93e0faff250fda1693aa8f0a1445`.
The oracle ledger is 423 passed, seven skipped and 6,096 not run, SHA-256
`cddd8fff7571cd00a132fc91ac1c3a722d9bbf4aba1807efc1c55390a788ab7b`.
Firefox/WebKit remain explicit provider gaps; the Python corpus and 48,477-row
Core JS/TS corpus are still incomplete.

## ABI 32 selector validation checkpoint (2026-09-27)

`packages/pwright-python/tests/official/selector-registration-errors-manifest.json`
pins the unchanged async and sync `test_queryselector.py` sources at SHA-256
`da8882e3b969ad83a159bec555c05971939d6b419e4c5b9be16165b60aa28708`
and `6799dd725cfb7c99a1e90ba0cfc5dc4a83943a9fb673a22a4fb1f00f78f9bda1`.
It fixes exactly the two async/sync
`test_selectors_register_should_handle_errors` functions. Their Chromium
expansions pass 2/2 through the installed ABI 32 candidate wheel and 2/2 in
the independent official oracle. Candidate report SHA-256 values are
`6f73dd92d3eb989ffb240e14e5c4fdfb3aad7dbcf444b5af7dad20be430fa2f1`
and `309a7f2018905a679456f3f0bdc158e896c719ff0c99a865022038c60b89db93`;
oracle values are
`fe4ce4a31d2b9d8f0acf097ce10d19c6a3a443959cb5c71beacdd076d21d1e0f`
and `7405bd869dfb433da3a0d640b001a3bb91fcebd83da26e707e0a98704c21ecf2`.

The preserved candidate diagnostic has SHA-256
`099d1dcffe4f85bc2e032a5961e8dcc0a726f3ea5c99ff0b33320d8adc40cec9`.
It shows `Page.query_selector("neverregister=ignored")` bypassing the Rust
selector parser and leaking a browser DOM CSS `SyntaxError`. ABI 32 exports a
native `validate_selector` call over the existing bounded Rust `LocatorQuery`.
The thin Page facade forwards only Frame identity and selector before the
existing handle-evaluation path; it does not parse syntax or reconstruct error
text in Python. This validates errors only and does not claim registered-engine
browser execution.

Final wheel archive SHA-256 values are
`d2a99b6f90fb87ac0e82310bd1b88f89ea4b6bd8c79089f698b5f86487c38ffc`
and `2f16684d27b517199caad2ad24fb070649b624d6206817f0e22e622ed183a2af`;
their payloads are reproducible after normalizing CycloneDX identity fields.
The installed wheel passes 32/32 unit tests, strict Ruff/Pyright, feature-cold
and exact-profile Clippy, dependency proof and full 6,526/6,526 collection.
Collection XML SHA-256 is
`8b191388c0d883749adadc61b93a0aefa3aa928c7f781737c1abdcbacdc10c29`.

The candidate ledger is now 444 passed, seven skipped and 6,075 `not-run`,
SHA-256
`7b3fb579c03cf79a5869613890e06506903142ae8e4627de877e2f8bc4f0d6f3`.
The oracle ledger is 425 passed, seven skipped and 6,094 not run, SHA-256
`89d888ddfe687e446614a35d5a24467e7b3123678242d1191129d93fb0d9b727`.
Four Firefox/WebKit expansions, custom engines and the remaining Python/Core
corpora are explicitly incomplete.

## ABI 33 registered selector-engine checkpoint (2026-09-27)

NODE-252 pins unchanged async/sync `test_queryselector.py` sources at SHA-256
`da8882e3b969ad83a159bec555c05971939d6b419e4c5b9be16165b60aa28708`
and `6799dd725cfb7c99a1e90ba0cfc5dc4a83943a9fb673a22a4fb1f00f78f9bda1`.
The base manifest fixes async/sync
`test_selectors_register_should_work`; the worlds manifest fixes async/sync
`test_selectors_register_should_work_with_path` and
`test_selectors_register_should_work_in_main_and_isolated_world`. Their six
Chromium expansions pass 6/6 through the installed ABI 33 candidate wheel and
6/6 through the independent official oracle. Candidate report SHA-256 values
are
`a0a181fb7cba3a19a243c9a92d7e17afa16cba2a1f4d425d66f8c43c297dbef0`,
`e42c11f66dd34b0c546d45e0e7360c2951b1d69df31acd1162a4b7e4cb2ad1e3`,
`9c184c8741f3b89035b6dc3f50960c24371405ec552ce5cebea15e5a33a00fd4`
and `1ef83d955601463de7573212d4c4dcaf0428fc3b7e0e6d32972c7fe6729abba2`;
oracle values are
`4daf3e870a14828062d6634afa017a7f7a9130df5ae142b72ddf39aea1210320`,
`25284ec1b1408550ea4ba4d5e2e2b34841e593581ce44d271dff609bfd812271`,
`8750c2ca46eeaecb8b454edc8f4f9f50ac9c923030bf7eb43c1b8ee3b3621d45`
and `7c86df708ab889418e4d7f1b3bc4453060913dadb8d43527166cc938234bcee0`.

The ABI 32 unknown-engine candidate diagnostics remain preserved. ABI 33 also
preserves an accidental Locator-strictness diagnostic and two diagnostics in
which the final custom part incorrectly used `queryAll`; none is merged into
the green ledger. Rust now compiles a registered plan only when a selector
actually names a registered engine, uses `queryAll` for intermediate custom
parts, `query` for a final single lookup and `queryAll` for a collection, and
installs each distinct engine source once in the selected execution world.
The registry admits at most 64 engines, 64-byte names, 1 MiB per source and
8 MiB of accounted name/source payload plus fixed 64-entry structural
overhead. A compiled program admits at most 256 parts and 2 MiB. Python stores
no engine source or parser state.

`Page.query_selector` now asks Rust whether a registered selector matches.
Zero matches return `None`; a positive match reports an explicit unsupported
handle-publication error instead of manufacturing a Python-side handle. Typed
Locator filters/composition on a registered plan and Playwright-style retry
polling also remain explicit gaps.

Final ABI 33 wheel archive SHA-256 values are
`0b589f576b783c5e5df854a6bd93777ecb55a0342d1b10872fa2cacda0a465f7`
and `a497d0316731b7d9ab94620ac0ea25fbe960f1b60c039669b3c7a1b38b4a80b2`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed wheel passes 33/33 unit tests, strict Ruff/Pyright and
Rust gates. Full candidate collection is 6,526/6,526 with XML SHA-256
`db9363d4fc7ae146c997a17efb15e9123ab5aef94e769a92781d4370a8865d7a`.

The candidate ledger is now 450 passed, seven skipped and 6,069 `not-run`,
SHA-256
`52fe9d0c64ecf31fb34098e468053265f79eddd68a8cb29dcddab2f75e45c675`.
The oracle ledger is 431 passed, seven skipped and 6,088 not run, SHA-256
`e38018a3c4207b9fdb0995bf8b121d2c455510c49a9440275b2c329cefde8bd3`.
Exactly six rows changed from the ABI 32 checkpoint in each lane, all from
`not-run` to `passed`. The 12 Firefox/WebKit expansions, remaining Python
corpus and 48,477-row Core JS/TS corpus are still incomplete.

## ABI 33 layout-selector checkpoint (2026-09-27)

NODE-253 pins the same unchanged async/sync `test_queryselector.py` sources in
`selector-layout-manifest.json`, SHA-256
`62190ad1ae4660cf737c02ce5a3274d68f3bf5234e06e286b6d77114af1fc378`.
It fixes only `test_should_work_with_layout_selectors`; the two Chromium
expansions pass unchanged through the final installed ABI 33 candidate wheel
and the independent official Playwright oracle. Candidate report SHA-256
values are
`f3d65c5e570917d5a7af2cd795596675128a7375beadcd7d1dfce75293820c09`
and `183b137fa3389396845dbcb72a659e6098e4e933fbb888344556fe926b30338c`;
oracle values are
`1c2747486b1151b39c58190cad9556268e46ecc354d35ee85bf82a4085df31c1`
and `672a2228f53f7725deaf07a0a0fe65eaeba0125926e0363e6a21bc7a61cb3e88`.
The initial candidate failures are retained outside the green ledger at
SHA-256
`b4b4df76f54f387bb73ece47c8dd06522ac33ee9c6b5ef38151d840f2d9e938b`
and `73ab134c6f71921d626392b03286595f3e9a523dea33df269d7e8d9f26e537ad`;
both failed at the first `div:right-of(#id6)` with a selector-program error.

The optional `layout-selectors` feature admits the existing bounded selector
source, at most 256 structural parts and a 256 KiB emitted browser program.
Rust parses and compiles `left-of`, `right-of`, `above`, `below` and `near`,
including an optional near-distance argument and the official test's top-level
adjacent-sibling `+` composition. It emits the upstream geometry score formulas
and stable score ordering into the utility-world execution program. Python has
no selector parser, score implementation or retained layout state. The
existing ABI 33 internal native browser-program count method is reused, so no
native export or ABI number changes.

This is deliberately a narrow claim. The parser does not yet accept every
arbitrary nested CSS/custom-pseudo layout shape, execution is one-shot rather
than Playwright-style retry polling, and a positive layout
`Page.query_selector` match still cannot publish an ElementHandle. Firefox and
WebKit providers remain unexecuted.

Final wheel archive SHA-256 values are
`d2cab4e3d21671f29a6836811c24aa1c7c48ecdf0bac93cd493c0b7e05d27a33`
and `79e8652366a1bcb82fd4e726715f6cc71a79b9cc0289aaae4c95efd71318f369`;
their normalized payloads are reproducible. The installed wheel passes 33/33
unit tests, exposes `layout-selectors` and both layout limits in native
capability metadata, and passes strict Python/Rust gates, feature-cold and
exact wheel profiles and the 108-package dependency proof. Full inventory collection remains
6,526/6,526 with XML SHA-256
`7d333d7b06713585dc11ffb097354efac1abb5720434f15acc1f6d542f013bed`.

The candidate ledger is 452 passed, seven skipped and 6,067 `not-run`, SHA-256
`295722cd54450259098ff79cd59a5f47cb59926f2d4403f8292d47f481f373df`.
The oracle ledger is 433 passed, seven skipped and 6,086 not run, SHA-256
`263f3100d25d77d1de6b705e6b4cf152ac439857da9f5caa769f1bf055ed6a98`.
Exactly the two intended layout rows changed from the NODE-252 checkpoint in
each lane. Remaining Python execution and the 48,477-row Core JS/TS corpus are
still incomplete.

## ABI 33 basic Locator facade checkpoint (2026-09-27)

NODE-254 pins unchanged async/sync `test_locators.py` sources at SHA-256
`6761dfaee418ceb9dc3cd21937d936db36db20ddeae15cd0c677586c5900ed77`
and `5f4e47a3225669ef2328ada0c873114b2a9238a9a337b5dcb5328a3d4fc3c1e6`.
The fixed manifest SHA-256 is
`922ff55720db9018e43b0fc886a1459f4fb7118b26c9d04221dbaf1421622753`
and selects 34 Chromium invocations for attribute/input/HTML/text/state and
collection reads plus fill, check/uncheck, set-checked, focus, press and
element evaluation.

Initial candidate reports record 34/34 missing-facade failures at SHA-256
`82f2ffafa215f02c4d3722bbbbcece86214342da44f0fb32924ee88b5c758aa5`
and `7750f91f3dfe8adfde60b66b42fb4d0c2a0d68ed9fa1c0b60db53893acf5d2b1`.
After the thin facade was added, the async lane reached 16/17; its preserved
report SHA-256 is
`7423770a82749c4540214aa3bda2c5338b55e26bf413090f317ad29f25017ae9`.
The remaining row proved that Playwright `:text("button1")` was still leaking
to DOM `querySelectorAll`. Rust now recognizes quoted `:text()` in the bounded
query recipe and compiles it to a typed exact text selector.

The final installed-wheel candidate passes 17/17 async and 17/17 sync; report
SHA-256 values are
`90699323c687a4a3ad2cc14c163c8ca14bf28846740f9b6193e9e2ffb8f0ddd8`
and `bcb01f7e3d67d98181a8fd9efc00eeb9b308d045921b5e59e0a159b20459fa6c`.
The independent official oracle passes the same 34 rows, with report hashes
`9b47f044f078e702b9496eec1a601bd724bbb05ff8f4967a0b387d66ce147f92`
and `144d77608332d94d99be317250ef88fe7bfd5aef7cf2e2dfc2c872c8b22545ed`.

Python forwards only selector, public arguments and options. Scalar and
collection reads execute existing typed Rust `LocatorRead` variants; actions
and evaluation use the existing Rust owners. No native method, ABI, feature or
dependency edge changes. The final two repaired wheel SHA-256 values are
`ce15aa154b33e075e79b01c52a3de837fb16d5fc6ef78718c35a075b6319dbcd`
and `54582aa165bae4f10567727ad71b6890bb0e3ea96e40dc8e4c5d33a59a74b667`;
normalized payloads are reproducible. Installed unit tests pass 34/34. Full
inventory collection remains 6,526/6,526 with XML SHA-256
`a961d2252f98f5dfc6037e29d4eb52c1ba169be601d8161fb2761d56e048fc97`.

The candidate ledger is 486 passed, seven skipped and 6,033 `not-run`, SHA-256
`2a956ea3e610f1a8ddad4b92cce0dde918a86aaa3e46a0eb915a5525cc925437`.
The oracle ledger is 467 passed, seven skipped and 6,052 not run, SHA-256
`7a5412b7212fdf7c4e3a6828d411febc65e922dc4be5bfa046210e67e2b3563b`.
Exactly the intended 34 rows changed from NODE-253 in each lane. Locator
composition, strictness, remaining actions, Firefox/WebKit and the complete
remaining Python/Core corpora are unfinished.

## ABI 34 Locator execution and count expectation checkpoint (2026-09-28)

NODE-255 pins the same unchanged async/sync `test_locators.py` sources at
SHA-256
`6761dfaee418ceb9dc3cd21937d936db36db20ddeae15cd0c677586c5900ed77`
and `5f4e47a3225669ef2328ada0c873114b2a9238a9a337b5dcb5328a3d4fc3c1e6`.
The fixed `locator-execution-manifest.json` SHA-256 is
`ed03f8d21b9d27cd400b3a3cb98203a20a35fdc4c7f57aa6e03ec00c92411e09`.
It selects five exact functions in both APIs: click, click after node
replacement, collection evaluation with a present selector, collection
evaluation with a missing selector, and count after deleting the main-world
`Map` constructor.

The first candidate invocation incorrectly supplied `--browser-channel` to a
CDP-attach fixture and is retained only as a setup diagnostic. The following
async candidate report passed 4/5 and failed only
`expect(locator).to_have_count(0)`, proving the typed count read worked while
the shim assertion was still a placeholder. That report also remains outside
the green ledger.

The final installed ABI 34 candidate passes 5/5 async and 5/5 sync. Report
SHA-256 values are
`5e7c4c6caece6cff6db69c80ecbd61c2009fc45f5306257f306cebfcc4d2eab3`
and `7f4e00becab1e465b04f6ce4ef8010d31db720152ad7dd6c06d0253014b5dcf3`.
The independent official oracle passes the same rows, with report SHA-256
values `8c1646d91536fc53e24cabc5ffc05e96c8530f7103ff9bf281e153898db302ed`
and `d3f183ef9b809b4f6f1cebd9f36597e766b7c9a276213e4066f4864922053001`.

Rust owns the expected-count comparison, total deadline, typed `Count` reads,
retry on navigation/context replacement and bounded 10 ms sleep. Python's
async/sync `expect` facades forward only selector, expected integer, timeout
and negation. Python ABI advances from 33 to 34 because the native export
`locator_expect_count` was added; no Cargo feature, Pwright package or
dependency edge changes. Node `_expect('to.have.count')` reuses the same Rust
owner and adds no JavaScript polling loop.

Final repaired wheel SHA-256 values are
`4c159b469a2a0a03efb9da2a89ef3625f4f3d7458163e41f18e7b898a4b2d305`
and `c4120502dbb9ad26dea5e21448eb3340d5ce2a2c68a10644dbd1e98be4b7e6e7`;
their normalized payloads are reproducible. Installed unit tests pass 36/36,
and Node adapter tests pass 270/270. Full inventory collection remains
6,526/6,526 with XML SHA-256
`40d9970bf98628c3ca3ff3ec6d369948e5d1693e1fca3b3f396ddfac4f3f874f`.

The candidate ledger is 496 passed, seven skipped and 6,023 `not-run`, SHA-256
`0562df16dd47427a0aa6bd6e6071b833c4a29dc88c5998d8e88aff5a23276649`.
The oracle ledger is 477 passed, seven skipped and 6,042 `not-run`, SHA-256
`14912795ba9cbb4efc8a6c3713506dc039094c497ab0f14806f5cd8ee160ba8b`.
Exactly the ten selected rows changed from NODE-254 in each lane. Remaining
Locator composition, strictness, assertions, actionability, Firefox/WebKit,
6,023 candidate Python rows and the 48,477-row Core JS/TS inventory remain
unfinished.

## Complete Locator diagnostic and repair checkpoint (2026-09-28)

NODE-258 executes every Chromium invocation in the unchanged upstream
`tests/async/test_locators.py` and `tests/sync/test_locators.py`: 86 async plus
77 sync, 163 rows total. The original candidate records 33/86 async and 27/77
sync passes. Its report SHA-256 values are
`60214c4a0009b969ad4ee6f3124d7e57dd16b2f5d548638a40e3d287d19c8501`
and
`a48dd15a0ec55296b923bfc7dd7f1c43f8b187cb62234bf317edf42df6fb0077`.
The independent official Playwright lane passes 86/86 and 77/77, report
SHA-256 values
`be1918c6db838d202aed0142c6ab0e3fd721761cb4868c28b00829e36b2c7dd4`
and
`caab68477b05e769b3ce7e8f78063c0d5ec3178605bde306f1a9c25321389053`.
Therefore all 103 original candidate failures are genuine discrepancies.

NODE-259 keeps the same complete files and adds bounded `Locator.all()`,
`Frame.locator()`, repr/str, Playwright-shaped strictness wording, and direct
Rust-owned Locator actions for blur, hover, clear, single-string option
selection, scroll-into-view and sequential typing/type. It also routes the
existing click/fill/press/checked/focus methods through the immutable native
Locator owner. Ten async and nine sync rows change from failure to pass; no
passing row regresses in the stable reports.

NODE-260 removes attached-browser cross-Context pollution: default/external
Contexts borrowed through CDP attach no longer receive eager request-history
or response-event subscriptions. Owned Contexts still do, and a Rust-owned
browser process still eagerly activates its default Context. Exact Page network
waiters remain demand-driven. Demand-driven callback/history activation for a
borrowed pre-existing Context remains open and is not claimed here.

The final isolated candidate records 43/86 async and 36/77 sync passes with
zero infrastructure errors. Report SHA-256 values are
`682637e35861bfa034ff45f3b09a7d0cf273b691cdb461e1a5f79075dcd6439f`
and
`5e5da15e9c54683cbcbbc0d7f11bc3e8f84de156bf39992af813d7234b581d71`.
The complete candidate ledger is 528 passed, 84 failed, seven skipped and
5,907 not run, SHA-256
`b617aeabdd66aa3f0f25b9d318d1aea3bb876e576b4ac7c134fd3f86c89d8216`.
The unchanged oracle checkpoint is 593 passed, seven skipped and 5,926 not
run, SHA-256
`20d3d46481e347aaf78222dd5d6c240f99d5a541aa75899444bf1b321b4f49da`.

Two release wheels have SHA-256 values
`5337ba78e6de10591d88d34c49d2455f4714f13712e1a37243cad9d9e9b410cd`
and
`a6a8c8045edba937bd1ecf2bcb77cba0c6bca6d98534933b860d0776ae64229e`;
their normalized payloads are reproducible. Python units pass 47/47, Node
adapter tests pass 270/270, strict Clippy/Ruff/Pyright pass, and full candidate
collection remains 6,526/6,526 with XML SHA-256
`bdbbdd9e587bddbb31d42bf0a257968cee647f918d0e94cb8141db56a009bb46`.

The remaining 84 Locator discrepancies cluster around filter/has/has-not and
regular-expression text, FrameLocator/content-frame ownership, ElementHandle
and evaluateHandle conversion, additional actions, screenshots/bounding boxes,
wait/highlight, text/class assertions and complete error/call-log parity.

NODE-261 through NODE-264 keep the complete files selected while adding native
typed filter/composition, wait/expectation, role/test-id/evaluation and visible
chain semantics. The successive totals are 83/163, 107/163, 110/163 and
114/163, with no losses.

NODE-265 advances to ABI 39. Double-click, path/payload file input and Locator
element-handle publication delegate to Rust-owned action, filesystem/payload,
selector-resolution and handle-lifecycle capabilities. The complete files pass
67/86 async plus 58/77 sync: 125/163, exactly 11 gains and no losses. The
cumulative ledger is 574 passed, 38 failed, seven skipped and 5,907 not run,
SHA-256
`2356b169e48248f6609fcbd097d9f3205dd87ad2b26ec45a89eb7149a84b7d6e`.

NODE-266 advances to ABI 40 and exposes the existing Rust-owned highlight leaf
through the separate `locator-highlight` feature. Both targeted rows pass. The
dedicated Chrome process restarted during the complete async run; the raw
86-row report is retained, its seven setup-affected rows are rerun exactly,
and the checked JUnit merger produces the final evidence instead of hiding the
interruption. Final reports pass 68/86 async plus 59/77 sync: 127/163, exactly
two gains and no losses. Their SHA-256 values are
`e097350da9203f67c3bf88b9fd73620acff9d797ad64bce89538f72f935bd59a`
and
`42db7f85e94db0e26386fa3370876a0350d0f90e4d8215a4157967ec86c7a8ae`.
The complete Locator ledger is 127 passed, 36 failed and 6,363 not run,
SHA-256
`074b438d329801fd69ac700e7da53571364b560a55f15af3e75116db430c2b9e`.
The cumulative 6,526-row ledger is 576 passed, 36 failed, seven skipped and
5,907 not run, SHA-256
`6976549c14fa4000f16617a0226b53753f54580907a5294aa413eea1d40f2cce`.

Two repaired ABI 40 wheels have SHA-256 values
`7c3a265b9de1825a302ce8bc6d7a26f85c61e212e8605293457f96221b161ff6`
and
`65b5d843a576e78c015d765e6c94774367c0be8b3b26fc5e7793c19abe5aa686`;
their normalized payloads are reproducible. Python units pass 58/58, the
affected Rust host gate passes 551 tests with 22 explicit live/diagnostic
ignores, addon units pass 2/2 and strict Clippy/Ruff/Pyright plus the dependency
proof pass. The remaining 36 Locator discrepancies and all 5,907 unexecuted
Python rows remain open; no complete-suite claim is made.

## ABI 41 Locator DOM actions and Handle checkpoint (2026-09-28)

NODE-267 adds a separate `locator-dom-actions` feature for Rust-owned
`dispatch_event` and `select_text`; selecting it adds no Pwright package to the
108-package base closure. `Locator.evaluate_handle` instead reuses the existing
Handle feature: Rust performs strict Locator resolution, executes against the
temporary element handle, publishes the owned result handle and computes its
bounded preview. Async and sync Python remain thin facade adapters.

The six unchanged targeted upstream rows pass 6/6, report SHA-256
`3578853c90e57c8e7b15dbc36287d2ccf347a5dda26abf7a7590fc24373a3dc1`.
The complete async Locator file passes 71/86, report SHA-256
`7c231d815995b070495dc4f4f30c060b934e500f363e142a4368bf6977c66d2c`.
The valid clean sync run passes 62/77, report SHA-256
`0c5aa8d72106042bfbbea686dde9a55910a2e404ba1d55bdf13b85e19cddbd10`.
Together they pass 133/163, exactly six gains and no losses from NODE-266.
The first sync attempt remains preserved as invalid infrastructure evidence:
it reached 26 passes before browser closure produced 52 fixture setup errors.
Those setup errors are not candidate outcomes and were replaced by the complete
clean 77-row report.

The complete Locator ledger is 133 passed, 30 failed and 6,363 not run,
SHA-256
`89154dea59026809606c087f49786fdbf9ba3bb708f9dc78a72349e379108292`.
The cumulative 6,526-row ledger is 582 passed, 30 failed, seven skipped and
5,907 not run, SHA-256
`1feb3bb3ec44812ee930defa5471cd8e8454e2d8c49e9502059273f1c360fe41`.

ABI 41 wheel SHA-256 values are
`b5532c550883212610bf0991c9a555e021f0ef2862c882a5cb9f84c0d70b8665`
and
`ecfb2d1df595cd00a0be48d650c4ed893c9ab0eefa86fb3d76784fd429386d74`;
their normalized payloads are reproducible after excluding only the permitted
CycloneDX serial number and timestamp. Python units pass 54/54, addon units
pass 2/2, the affected Rust host gate passes 551 tests with 22 explicit
live/diagnostic ignores, and strict Clippy/Ruff/Pyright plus dependency gates
pass. The remaining 30 Locator discrepancies, all 5,907 unexecuted Python rows
and all 48,477 Core JS/TS invocations remain open. This is not a full-corpus
compatibility claim.

## ABI 41 Locator diagnostic-shape and relative-XPath checkpoint (2026-09-28)

NODE-268 keeps the ABI and dependency closure unchanged. Rust now retains a
captured selector as a typed recipe long enough to reject capture-plus-`nth`
with Playwright's exact error, formats bounded `Locator.click` timeout call
logs with the API prefix and Playwright-shaped role descriptions, preserves
Unicode locator text, and converts only the admitted `xpath=./TAG` subset used
by `has=` into a scoped typed query. Broader XPath remains unsupported.

The eight unchanged targeted async/sync Chromium rows pass 8/8, report
SHA-256
`e009754dd6d9cc27279875d84079f7ed72f1aa09ce53158433c0b7ea46e7140f`.
The complete async Locator file passes 75/86, report SHA-256
`c403694a37a45224c4cca636f0a733ba5713e6480cb45e6c1aa1e0c94e42da95`;
the complete sync file passes 66/77, report SHA-256
`8e6c90f179b35c843860d4d9f501c552d27ee606cb104c3e7f1dae2502fa7e64`.
Together they pass 141/163, exactly eight gains and no losses from NODE-267.
Two earlier targeted setup attempts are retained but excluded: one lacked
Pillow and one supplied the candidate adapter an unsupported `channel` launch
option. Neither reached target behavior.

The complete Locator ledger is 141 passed, 22 failed and 6,363 not run,
SHA-256
`cf145b50fa1a45bc018d043058c92d37b783b788f442713a817ba9593f96de56`.
The cumulative 6,526-row ledger is 590 passed, 22 failed, seven skipped and
5,907 not run, SHA-256
`6180ffa49de77b48b2a61d13a8fe3f159654c4e5c4529d841082377112d0ca46`.

The two reproducible ABI 41 wheel SHA-256 values are
`0036073c4cce16844e7c5cd35e3d437d472f52e57959741da1e150cba2e3e6d8`
and
`1e905da9fcc741493b93739c2dfe28ad315bd91821c751f34ad3bd7d0f601b69`.
Installed Python units pass 60/60, selector-model units 4/4, addon units 2/2,
and the affected Rust host gate passes 554 tests with 22 explicit
live/diagnostic ignores. Strict Clippy, Ruff, Pyright, wheel normalization and
the 108-package dependency proof pass. The remaining 22 Locator failures are
four viewport rows, 16 FrameLocator rows and two focus/blur callback-timing
rows. All 5,907 unexecuted Python rows and all 48,477 Core JS/TS invocations
remain open; this is not a full-corpus compatibility claim.

## ABI 78 HTTP credentials and response-body checkpoint (2026-10-04)

NODE-322 selects eleven unchanged async Chromium rows from Playwright Python
v1.58.0 commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6`: unauthenticated,
correct, incorrect and resource-body Basic-auth cases; exact-origin matching
and scheme/host/port mismatch cases; persistent-default credentials; and popup
inheritance. The three upstream source SHA-256 values are
`1eb6981f36e917dba202d0d0a324a4d2efcafc0b9a591c3d96f7ad7c99809cdc`,
`61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`
and `c7f0c08e57a874ffb7332663615123814b0e0a2ceef465e203206eff03a0cc3e`.
The eleven-row manifest SHA-256 is
`154cd2e468af69fd4a8365e46a8959e3fb84f933626dff388c4e64970a241150`.

ABI 78 exposes ordinary and persistent-default HTTP credentials through a
fixed Rust owner. It retains at most 16 selected Context policies, bounds each
username, password and optional origin to 65,536 UTF-8 bytes, and bounds each
pending-action, authentication-attempt, request-mapping and initializing-
session collection to 64 entries. Exact-origin matching is case-insensitive
over the normalized origin but remains strict across scheme, hostname and
port. Chromium Fetch is enabled only for selected sessions, every challenge is
settled on its emitting session, and a repeated challenge sends
`CancelAuth` without invalid null credential fields. Page routing and HTTP
authentication share one reclaiming event cursor while preserving both Fetch
patterns.

The resource-body row also selects a separate navigation `response-body`
leaf. The Rust navigation result preserves its request identity; the leaf
routes that identity back to the authoritative Page/Frame session and performs
one `Network.getResponseBody`. It admits at most eight concurrent operations,
bounds encoded and decoded output to 4 MiB each, validates base64 and charges
the Python copy boundary. Python retains only a bounded
page/frame/request-id tuple until `Response.body()` and never owns the body,
authentication state or CDP session.

Final installed candidate and independent oracle pass 11/11; candidate repeat
stress passes 110/110. Report SHA-256 values are respectively
`cb7809748be7cc915f6125e973c1149b252b11fcf255c7b528489d7ba93a2de2`,
`7652ef246c7f7007159d663a640e1fa9cc14591465de8a34f2961a035d6b3cc0`
and `ffe3600bd95491851f7a623388780fb744a51c5fdae49322ab43f44040cf1004`.
The final installed-wheel unit report passes 165/165 at SHA-256
`023b372456d7759abf411a19d06a0352c6a961ef8a313aacde3da503e6e4cf8b`.
The all-feature host library passes 999 tests with 26 explicit ignores; 22
non-live integration tests and two addon tests pass. Strict Clippy, Ruff,
Pyright and the 108-package dependency proof, including the exact
`response-body` closure, pass.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`ec96b3f4af79891a2b2049537ac2650702e83d41ee7ca5539bbb06ce4037ccdb`.
All eleven Chromium rows change from `not-run` to `passed`, so the cumulative
candidate ledger is 766 passed, zero failed, seven skipped and 5,753 not run,
SHA-256
`294ab4a8fee1ea783d5f5b8043a54c52648fe5e9539fdecbd7c42c6000b9c2fa`,
with no losses. The remaining Python rows and complete Core JS/TS corpus are
still required by the active conformance goal.

Two final wheel archives have SHA-256 values
`473db3f3170c2124418f5bbd17dae19d8c5cc4f933938f1887233ce862c968b4`
and `68ee94ea983be2ed9794b21fbba030299dcc8a3e00c4eac320099f23d1dc98aa`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`f5b18b328da7c6d6a21e2335151da6289eec167fd9a0652a7818579a53662eca`.
They remain Mac-local while Homebrew OpenSSL is an external runtime
assumption.

## ABI 76 Context geolocation and permission checkpoint (2026-10-04)

NODE-321 selects the unchanged async Chromium
`test_should_support_geolocation_and_permission_option` row from Playwright
Python v1.58.0 commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6`. The upstream
`tests/async/test_defaultbrowsercontext.py` SHA-256 is
`61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`;
the one-row manifest SHA-256 is
`60d3b32cabfcfc13b2d951f891da0321e4d590c06956f40262bc13ce2b35e10e`.

The ABI 75 red candidate rejected public geolocation and permissions while the
independent official oracle passed once the HTTP fixture had equivalent
secure-context semantics. Repository rules prohibit localhost fixture serving, while
the unchanged upstream test relies on localhost being trustworthy. The runner
therefore has an explicit mode that gives candidate and oracle the same exact
dynamic Tailscale HTTP origin via Chromium's secure-origin allowlist; no other
origin or assertion is changed.

ABI 76 exposes geolocation and the separately installed geolocation permission
on async/sync ordinary and persistent Context APIs. Rust owns fixed 16-entry
coordinate and permission registries. It sends exact
`Emulation.setGeolocationOverride` before selected Page/OOPIF publication and
exact browser-session `Browser.grantPermissions`, omitting a Context ID only
for the persistent default and using the authoritative native ID for an owned
Context. Python keeps no policy or session mirror.

The secure-origin adapter also required an independently selectable bounded
persistent launch-argument leaf. It forwards at most 256 switches, 65,536
UTF-8 bytes each and 1,048,576 bytes total into the existing provider request,
while rejecting page-opening, profile and remote-debugging overrides.

Final installed candidate and independent oracle pass 1/1; candidate repeat
stress passes 10/10. Report SHA-256 values are respectively
`1a4b619753b5fc589702c5f138cb6f6d19a51ef20e022f3e39c7568fa1b53c39`,
`428ac84f06a7b6f89e8a0aa065c076e31dca56f07545c09c6bf46dfd7d921f47`
and `0397db4f1e7adde6c912cc89cb2bb93d4b1e39d0c67572ae8e14a02a8b4d451a`.
The final installed-wheel unit report passes 161/161 at SHA-256
`a6da39178d8fc49d105b87529539098a9500811b85b8116885f9f91e62101882`.
The all-feature host library passes 992 tests with 26 explicit ignores; 22
non-live integration tests and 2 addon tests pass. Strict Clippy, Ruff, Pyright
and the 108-package dependency proof pass.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`31366226f4778d8d7a75be3051559b01e0b0b9754b6cd214991e904a629fcffb`.
The cumulative candidate ledger changes only this row from `not-run` to
`passed`: 755 passed, zero failed, seven skipped and 5,764 not run, SHA-256
`f1288cda0a0873a31ca5a5915a661605f57a42cd316df68f0bd8fc51c155fb7f`,
with no losses. Four Chromium rows from the original default-Context diagnostic
remain: two MagicDNS/`localhost` cookie applicability gaps plus HAR and
HTTP-credentials product capabilities. Collection is not execution; remaining
Python rows and the complete Core JS/TS corpus are still required by the active
conformance goal.

Two final wheel archives have SHA-256 values
`8a56c3f671f288aa7d461f20d1eed9097425531b7772e9f6719fed1ab099fc81`
and `7d8a754afea1a822684c1ffc6daad029fe1f9335a10e21ee6f099502992096aa`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`f4808c9bd38bce42cb670c752d6edea3c39fc19328a81213782648a206758cc1`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 75 Context offline checkpoint (2026-10-04)

NODE-320 selects the unchanged async Chromium
`test_should_support_offline_option` row from Playwright Python v1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. The upstream
`tests/async/test_defaultbrowsercontext.py` SHA-256 is
`61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`;
the one-row manifest SHA-256 is
`00268111cd781614c53806e3f90903151be78e1db662680f547189c6996b065f`.

The ABI 74 red candidate rejected public `offline` as unsupported while the
independent official oracle passed unchanged. ABI 75 exposes the same optional
boolean on async/sync `Browser.new_context` and `launch_persistent_context`.
Python validates and forwards the public value but owns no policy, Context
identity, CDP session or initialization timing. The persistent FFI shape groups
all Context policies into one composable tuple, and the native type stub now
matches that runtime contract.

Rust owns a fixed 16-entry `ContextOfflineInstallation`. Explicit owned Context
creation records the portable Context identity before browser work and removes
it on compensation or close. Persistent launch stores one immutable default
choice and selects only the exact default Context; attached external Contexts
never inherit the option. Every selected Page/OOPIF initializer sends exact
`Network.emulateNetworkConditions {"offline": true, "latency": 0,
"downloadThroughput": -1, "uploadThroughput": -1}` before publication.

Final installed candidate and independent oracle pass 1/1; candidate repeat
stress passes 10/10. Report SHA-256 values are respectively
`27570a9e7750228f4b60c287573b38d015e0daa944ba6bb1624b7bb0de08cf99`,
`f8e99a025c088eba2042729572fb97ae047be0c6862e528cfdf4aa4b115c7527`
and `61bce407c655347e3308e6c5384a8f4c2edb48248562181a9a26c2daced268de`.
The final installed-wheel unit report passes 159/159 at SHA-256
`df4b303b88ec2458834baf4c54489c4773da2e871db87ac7e30a60a41d522bad`.
All-feature host tests pass 983 with 26 explicit ignores and addon tests pass
2/2; strict Clippy, Ruff, Pyright and the 108-package dependency proof pass.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`d925042c8bcdb55a7592543f2bd0f7814c592902eafa634e2d719f1bb9869b1c`.
The cumulative candidate ledger changes only the offline row from `not-run` to
`passed`: 754 passed, zero failed, seven skipped and 5,765 not run, SHA-256
`c38eea63e7096c771a8faa0912c0b1499158c6a11489dfb4848ac75cc4079592`,
with no losses. Five Chromium rows from the original default-Context diagnostic
remain: two MagicDNS/`localhost` cookie applicability gaps plus geolocation and
permission, HAR and HTTP-credentials product capabilities. Collection is not
execution; the remaining Python rows and complete Core JS/TS corpus are still
required by the active conformance goal.

Two final wheel archives have SHA-256 values
`cdec69d543da109a3071b0b0b52d3e5c35d8344ddcbbdcee2a5931fde18538f9`
and `2be2930d675c736375cebb3506228792c5eac0065c10bde7f5956311e8ad32a4`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`e73bb6233334972f3e58d38370672af658a0c791517862ecb3e50e7c87a123f4`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 74 Context timezone checkpoint (2026-10-04)

NODE-319 selects the unchanged async Chromium
`test_should_support_timezone_id_option` row from Playwright Python v1.58.0
commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6`. The upstream
`tests/async/test_defaultbrowsercontext.py` SHA-256 is
`61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`;
the one-row manifest SHA-256 is
`68c4d90a61e471f7d904c27b1831bbc65280f80fd91c645c6172fd9c00fa25dc`.

The ABI 73 red candidate rejected public `timezone_id` as unsupported while
the independent official oracle passed unchanged. ABI 74 exposes the same
optional string on async/sync `Browser.new_context` and
`launch_persistent_context`. Python validates and forwards the public value but
owns no policy, Context identity, CDP session or initialization timing.

Rust owns a fixed 16-entry `ContextTimezoneInstallation`; identifiers are
nonempty, control-free and bounded to 256 UTF-8 bytes. Explicit owned Context
creation records the portable Context identity before browser work and removes
it on compensation or close. Persistent launch stores one immutable default
choice and selects only the exact default Context; attached external Contexts
never inherit the option. Every selected Page/OOPIF initializer sends exact
`Emulation.setTimezoneOverride {"timezoneId": ...}` before publication. The
shared-renderer duplicate override is idempotent, invalid identifiers become
stable argument errors and all other protocol failures propagate.

Final installed candidate and independent oracle pass 1/1; candidate repeat
stress passes 10/10. Report SHA-256 values are respectively
`25b4741916039a3841c6e0416875befd52b46c06eafae057eb4fdcd6b4707e39`,
`fec89b423b6582adf9e7754e980500e4b6266c17c722e0043eeaf0e0c4093c73`
and `1a3674e68420813a531778093500877aac49868cea700c60d9592ae69a716c9d`.
The final installed-wheel unit report passes 157/157 at SHA-256
`1ea73cebd5b5a44578cd2af4816d8697e5f6381461bc55884cfdaa7304568d9f`.
All-feature host tests pass 978 with 26 explicit ignores and addon tests pass
2/2; strict Clippy, Ruff, Pyright and the 108-package dependency proof pass.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`46ef609d96e1cdd9be58f41f7d3878f2d4b6da63928cbcd03d55e04895b54420`.
The cumulative candidate ledger changes only the timezone row from `not-run`
to `passed`: 753 passed, zero failed, seven skipped and 5,766 not run,
SHA-256
`a48b4d1c0e8bdce904b95194393ee47709c01313c121843e81e1410f524b27fc`,
with no losses. Six Chromium rows from the original default-Context diagnostic
remain: two MagicDNS/`localhost` cookie applicability gaps plus geolocation and
permission, HAR, HTTP credentials and offline product capabilities. Collection
is not execution; the remaining Python rows and complete Core JS/TS corpus are
still required by the active conformance goal.

Two final wheel archives have SHA-256 values
`a8b4ae9a06dde9cf9d7f766cb7d326ca3b0e69d773c23b9fdbb14d8c19519632`
and `377d5981d1d76bf121bf0db45222a0b82e9610bb9f6e9b638f075a67f452f49f`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`8b1dd78a036c331ae027e2c1cbb49caa51008ead5b9d4e022c729607f8ba6d02`.
They remain Mac-local while Homebrew OpenSSL is an external runtime assumption.

## ABI 73 JavaScript-disabled Context checkpoint (2026-10-04)

NODE-318 fixes four unchanged async Chromium rows: two ordinary Context
JavaScript-disabled cases, clicking and navigating a wrapped link with
JavaScript disabled, and persistent-default launch. The manifest pins
Playwright Python v1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. Exact source SHA-256 values are
`1eb6981f36e917dba202d0d0a324a4d2efcafc0b9a591c3d96f7ad7c99809cdc`
for `test_browsercontext.py`,
`ea614c3891174eda1f74964c8c1d4ba84b214b74e03fa194ad878636938338e7`
for `test_click.py`, and
`61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`
for `test_defaultbrowsercontext.py`. The manifest SHA-256 is
`7f2e326e8c7d5ac9623279fac00277468ee9ae25394f11a10b6ac7d8d90641c4`.

The red ABI 72 baseline first failed all four rows because the public Context
option was absent. ABI 73 then exposed two independent prerequisites without
changing upstream assertions: `Page.expect_navigation` was missing, and the
default Locator click point used the union-box center of a transformed wrapped
link, which hit its parent `DIV`. The binding now arms the existing bounded
Rust navigation wait before the action, while the locator backend uses the
first visible clipped client-rect fragment. URL-filtered navigation waits stay
explicitly unsupported.

A later diagnostic ran while an unrelated live Page was emitting console
objects during attachment. `Runtime.enable` replay can precede utility-world
creation even though the exact main-world route is current. That optional
paired-world lifetime proof no longer aborts topology initialization. The final
candidate passed with that unrelated Page preserved, so the fix is evidence
against global attach fragility rather than a test-environment cleanup.

Final installed candidate and independent oracle pass 4/4; candidate repeat
stress passes 40/40. Report SHA-256 values are respectively
`991bbd1aa335772d0b59d33cd13e0afe4a980a431f8f72886473cf4261a7affe`,
`1068243f7512a4b54849b6cfb8ac0c733a197c41bb48db8d4a1c87794f96e286`
and `1ca6b2b46ad43527177358dc66947350d1cef8307a17630933039a0f0f8c40ce`.
The final installed-wheel unit report passes 157/157 at SHA-256
`b3ed8cb9a09f9945236a3949e0b5bdc255f0790ab1471c414f105fdfbcd16960`.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`f440d9f4e46739f28b5484f5c5f0bcd427fe2e3dee37be575f989f64d82fade6`.
The cumulative candidate ledger advances only these four rows to 752 passed,
zero failed, seven skipped and 5,767 not run, SHA-256
`93f22527ff4e8f89d95a525a17393d3d5d3463432e6e9e75f7135adcea7a81a3`.
Seven Chromium rows from the original default-Context diagnostic remain,
including two MagicDNS/`localhost` applicability gaps. Collection is not
execution: the remaining Python rows and complete Core JS/TS corpus are still
required by the active conformance goal.

Two final wheel archives have SHA-256 values
`7c2e48a4705d597aba896763e50b67ddfd6d8d03c562312c5dcd3f24ba687859`
and `89b60d08b5e5691aeb4de049a2d10a74f0a42947f42fdf57c3693d0eabd1ab9a`.
After normalizing only CycloneDX's build UUID/timestamp and matching RECORD
hash, every payload entry is identical; both native extensions have SHA-256
`26e341524d4f6bf5703089a1db57343cc885e8c243ed2bfc6448b72dfd9a41a0`.
Maturin still reports external Homebrew OpenSSL dylibs, so these are verified
Mac-local artifacts rather than self-contained distributable wheels.

## ABI 72 Context CSP bypass checkpoint (2026-10-04)

NODE-317 fixes four unchanged async Chromium rows: CSP meta, CSP response
header, cross-process navigation and persistent-default launch. The manifest
pins Playwright Python v1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6` and the two exact source hashes;
its SHA-256 is
`6748a04a6e0e1f0340d7e7c6e14ef6ba5ddd4e0e91361dd3fe2feb9f57fc4c64`.
Candidate runs from the final installed ABI 72 wheel. The independent official
Playwright 1.58.0 oracle explicitly selects installed Chrome because this Mac
lacks that environment's optional `chromium_headless_shell-1208` artifact; the
default-channel failure is retained only as an environment diagnostic.

Candidate and oracle pass 4/4. Candidate repeat stress passes 40/40. Report
SHA-256 values are respectively
`3760b066bee25591941fc8216c93c3d245a494a08ce8bd9de5ec3a096a241e36`,
`a0776de441de54867fcea680f928e37203ba97779bde912b178cb3c670c83459`
and `7aec8021bd753d172a696cb717cdb6dd1e0b55769cc0fce29b9d0eebdb6f5f7e`.
The installed-wheel unit report passes 152/152 at SHA-256
`865dcf1f721c0758472d18a07d64283ae31c84ae5269893fb18782ba3fbc846b`.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`de5a014d6818e1f613f08a74ddac3d70a23fc1f720b17d273f626ac0290d32c1`.
The cumulative candidate ledger advances only these four rows to 748 passed,
zero failed, seven skipped and 5,771 not run, SHA-256
`713d4047df50050430e4bfec6c843b171806653db19e8c0df6da6e66cd942508`.
The iframe CSP row is deliberately excluded because its unchanged baseline
first fails on the independently absent `ElementHandle.content_frame` API.
Eight Chromium rows from the original default-Context diagnostic remain,
including two MagicDNS/`localhost` applicability gaps. Collection is not
execution: the remaining Python rows and complete Core JS/TS corpus are still
required by the active conformance goal.

## ABI 71 persistent-Context extra-HTTP-headers checkpoint (2026-10-04)

NODE-316 fixes the unchanged async Chromium
`test_should_support_extra_http_headers_option` row from
`test_defaultbrowsercontext.py`. The manifest pins Playwright Python v1.58.0
commit `47a5d35ef4f815a2021349f86ae391f7c20c02d6` and source hash, and has
SHA-256
`5345d32db44ce2f21a3c8ebc6fec811a498c695f68972b66564a39677d2d1041`.
Candidate runs from the final installed ABI 71 wheel; oracle runs from the
independent official Playwright 1.58.0 environment.

Candidate and oracle pass 1/1. Candidate repeat stress passes 10/10. Report
SHA-256 values are respectively
`f7fe00394cd78f6745be76c28400a87eeb3e55c071a22aa04a151592f708fab0`,
`b0249488f1be47e6685e739ead9c63b958e8544bae3a67708211aafd15b183f2`
and `14302892b311ab2e884592c1364f90e0c34e939719b65ebc860f3a9e22b0ff7a`.
The installed-wheel unit report passes 150/150 at SHA-256
`30a6769c34931b6202d01b8ab39031cb54eef3b5a4fee1127f14ba18e9f4262c`.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`5a702bfa1b9a911fe704d98b4fcfe0ebbc4741fe428edc3f085afbf10e0d58fc`.
The cumulative candidate ledger advances only this row to 744 passed, zero
failed, seven skipped and 5,775 not run, SHA-256
`19d1e6e8365cf3c56b86e96edc76d10a6a619b797f967caedb477cacfbd69411`.
Nine Chromium rows from the original default-Context diagnostic remain open,
including two recorded MagicDNS/`localhost` applicability gaps. Collection is
not execution: the remaining Python rows and complete Core JS/TS corpus are
still required by the active conformance goal.

## ABI 70 persistent-Context media-default checkpoint (2026-10-04)

NODE-315 fixes three unchanged async Chromium rows from
`test_defaultbrowsercontext.py`: color scheme, contrast and reduced motion.
The manifest pins Playwright Python v1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6` and source hash, and has SHA-256
`a2da1fb06d4bd6039f7255d1f95e92f853ef5260b491f11fa1236a19f2c956f9`.
Candidate runs from the final installed ABI 70 wheel; oracle runs from the
independent official Playwright 1.58.0 environment.

Candidate and oracle pass 3/3. Candidate repeat stress passes 30/30. Report
SHA-256 values are respectively
`166ede1c3ec3e6aff90032bfa6b763ca7fbc6cc3f3798be5d8901af4ac7eea30`,
`341fa62124ef00ab42f2ff1094f6f5a1f4e38e32efafa6ee7846096a3ffe95df`
and `00c6284f51bd7267efd0084f9622cfd56485f135c5a90cc1786db4bc8a698f5e`.
The installed-wheel unit report passes 149/149 at SHA-256
`6cbd6ccefdf913083334e16b70b8a4b9a4cefcc66b1d3e889050c7b10f147262`.

Fresh unfiltered collection records 6,526/6,526 rows; its JUnit SHA-256 is
`12e4f2d08c5d297064f46dcec10213f276c6812d4a38a1a5f5993b4c1c86825e`.
The cumulative candidate ledger advances only these three rows to 743 passed,
zero failed, seven skipped and 5,776 not run, SHA-256
`8a7fe5975b0bbbf792cc8c13a0b8e81a11a70bb8f17dcb0e8f439abe30e67efb`.
Ten Chromium rows from the original default-Context diagnostic remain open,
including two recorded MagicDNS/`localhost` applicability gaps. Collection is
not execution: the remaining Python rows and the complete Core JS/TS corpus
are still required by the active conformance goal.

## ABI 69 persistent-Context launch-argument preflight checkpoint (2026-10-04)

NODE-314 pins the unchanged async Chromium
`test_should_throw_if_page_argument_is_passed` row from
`test_defaultbrowsercontext.py`. The fixed manifest SHA-256 is
`4af8748fab33f52ce7ee71742b94265a6655a8d7b12688b5a3b90a42ce32dd86`.
The implementation keeps ABI 69 and performs no browser command: it bounds the
public argument sequence, then reproduces Chromium Playwright's ownership and
page-opening errors before any native launch. Empty args remain a no-op and
non-empty switch-only args remain explicitly unsupported.

The final installed candidate passes 1/1, report SHA-256
`1e021993ba9b0ee312fcac31911a70585d9cd85f542543e39d03193854bcf7b8`;
the independent Playwright 1.58.0 oracle passes 1/1, report SHA-256
`b80e542edce7c010c72601ebfb3b06adb3bbe9aae8929bc212241aeda9748562`.
A same-process ten-repeat passes 10/10, report SHA-256
`3ad83c5bc02e6e9253caeead0fec68217dbc73e3c2673ce7a592a7117ba04a0b`.
All 148 installed-wheel units pass, report SHA-256
`dc4a5aaa5953dad432a34a8b4cb6d489631a915c374cc4f46cc38b930a01758c`;
Ruff and strict Pyright also pass.

Two wheel builds have normalized-identical payloads, with archive SHA-256
values
`d17207c54c374c072df4d752df38959ecd376c5918dcc215fe3b46a22cb93b07`
and `76bfdf8d21cb28d268139106d225cf1d989ae53e629eb0805bfd9278efc0b052`.
Their native extension remains byte-identical to NODE-313 at SHA-256
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
The Homebrew OpenSSL dylibs are still not vendored, so cross-machine packaging
remains open.

Fresh candidate collection is 6,526/6,526, report SHA-256
`70bc9e624d2fb4714d202e0345706c207f2e84e3b403f478d9d3a103cc10ed25`.
The cumulative ledger advances exactly one row from NODE-313 to 740 passed,
zero failed, seven skipped and 5,779 not run, SHA-256
`b4037c98e0b0d8ae979df04a7c8f033cfb3da4514f8693f25ced9bda469de5ca`.
The cumulative summarizer's redundant Python-3.10-only
`zip(..., strict=True)` was removed after its explicit equal-length check;
strict Pyright passes and system Python 3.9.6 reproduces the exact ledger hash.
The first oracle attempt used the candidate-only environment and could not
import official Playwright; it is an environment diagnostic and is excluded.
Thirteen Chromium rows from the original default-BrowserContext diagnostic,
the rest of Python and all Core JS/TS work remain open.

## ABI 69 BrowserContext cookie checkpoint (2026-10-04)

NODE-313 pins four unchanged async Chromium functions from
`test_defaultbrowsercontext.py`. The source SHA-256 is
`61dc8a12d0346c7ad07d11dfb1e351582d82e0a71665964c100968478e8c2ac0`
and `context-cookies-manifest.json` SHA-256 is
`fa3a6e73e333ed1fb1623c8f636cb13d0d4e63e8ee60f9b28432b216e742757d`.
The manifest selects read, add, clear and third-party cookie behavior without
changing an upstream assertion.

ABI 69 selects the optional `context-cookies` leaf. Rust resolves the exact
BrowserContext owner and sends browser-root `Storage.getCookies`,
`Storage.setCookies` and `Storage.clearCookies` commands. Chromium remains the
only source of truth. Rust owns URL rewriting, URL/domain/path validation,
filtering, clear matchers, expiry and partition identity; filtered clear
preserves the internal `hasCrossSiteAncestor` bit. Python exposes thin
async/sync methods, typed cookie values and pre-copy resource checks.

The final installed candidate records two passes and two failures, report
SHA-256
`8fdf3d90b4c67340f8c35b7dbff6eac31a656eb8929b73f9635a31003291546f`.
The independent Playwright 1.58.0 oracle records the exact same statuses and
failure values, report SHA-256
`ea7eb1f6c0bc3ac4ffe38a663a781021e2ecf73a9a0c611677b670d8f9cabf32`.
In each lane, read and add return the real
`macbookpro-yuacx.tail8f3b66.ts.net` cookie domain while the unchanged test
expects `localhost`; clear and third-party rows pass. The two failures are
therefore fixture/provider applicability gaps, not Pwright discrepancies, and
remain `not-run` in the formal ledger.

The exact applicable candidate report passes 2/2, SHA-256
`93e3e0ff2b645006f56e8a308c3ef406c050b231701165ec9ac1277c61e2bdbd`;
the oracle counterpart passes the same 2/2, SHA-256
`814de4fed049eca50e0a7f685c06ab60d26bd06657e1d2d2cbce47f8b5e0bdf0`.
A ten-repeat installed-candidate run completes all 40 persistent launches:
20 pass and the same 20 hostname-only outcomes recur, report SHA-256
`5e31a4facf4d751e856bb64cc29a2e924ea483bf7fe3f7b0d75ca3b3322a5f25`.
This exercises repeated teardown beyond the fixed 16-session capacity without
leaking an admitted session.

The all-feature release gates pass 130 CDP tests, 962 host tests with 26
explicit ignores and 2 addon tests. All 146 installed-wheel units pass, report
SHA-256
`6dfba7a1e79c0aaa228ce0b56cc491790f43c414580637d4e43875a6dd459b4f`.
Formatting, strict release Clippy, Ruff, strict Pyright and the unchanged
108-package dependency proof pass. Fresh candidate collection is 6,526/6,526,
report SHA-256
`25079fc004c4d1fd31aaf638aade99e395c39a0edb69733471a6c9623972276c`.

Two ABI3 wheel builds have normalized-identical payloads. Their archive
SHA-256 values are
`174a30499a3c21c746d4ed5ffb5461247afe012bc9207c32834c68c0801382d4`
and `faa8f553d1643ed4594997acf1fd06dd75f730c3981cbf99fe12d3dda0d69c58`;
the byte-identical native extension SHA-256 is
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
Maturin reports external Homebrew OpenSSL dylibs, so the wheels are validated
on this Mac but are not yet claimed as cross-machine self-contained releases.

The cumulative ledger advances from 737 to 739 passed, zero failed, seven
skipped and 5,780 not run, SHA-256
`dbd8fc8f8e5c3e10121da0421fdb7f0b16bd4c68176615875c9f0eeaba051638`.
Only the two applicable rows change. Early generated-cookie decoding without
`sameParty`, an environment without Pillow, an oracle attempt that expected a
missing headless shell and a Cargo-manifest-only wheel with the wrong package
name remain diagnostics and are excluded. Fourteen Chromium rows from the
original default-BrowserContext diagnostic remain open, including the two
applicability gaps; the rest of Python and all Core JS/TS work also remain
open.

## ABI 68 BrowserContext event-facade checkpoint (2026-10-04)

NODE-312 starts from the unchanged official async Chromium
`test_should_fire_close_event_for_a_persistent_context` row in
`test_defaultbrowsercontext.py`. The prior ABI 68 candidate performs the
persistent close but lacks `BrowserContext.on`; that red report remains
diagnostic and is excluded from the ledger. The independent Playwright 1.58.0
oracle passes 1/1, report SHA-256
`37c2be0db8968e5b422455edb779b2843a8dfb074b77227121d98ef9cb17b8c1`.

The package keeps ABI 68. Async and sync BrowserContext now expose `on`, `once`
and `remove_listener` for the already installed `page`, `close` and optional
`console` events. No event is synthesized: the existing Rust-owned topology
notice reaches the existing Browser lifecycle watcher, which publishes the
same closing Context identity before `close()` returns. Sync wraps Page,
ConsoleMessage and Context values into their existing public facade identities
and routes callback failures through the existing bridge error owner. The
first Context-console callback acquires one existing bounded native
subscription token and the last removal releases it.

The final installed candidate passes 1/1, report SHA-256
`72d8852dc88c130d263000d557588c89a4d7437b9bfbe7c170fdd85a4b4b9c91`.
A same-process ten-repeat passes 10/10, report SHA-256
`d56408e389a1eea09707f21da2076924f70e51b7668e6281db15624c3561b578`.
All 141 installed-wheel units pass, report SHA-256
`62b70dd3f905daa576b5e8d0447e972bd7c95c1b43d70854d423977929eff460`.
Formatting, Ruff, strict Pyright and the 108-package dependency proof pass.

Two final ABI3 wheels are normalized-reproducible, with SHA-256 values
`2487ed3dbbac1e4fa7047e2cfbf7b3c6e5f0978b94eb972db27b9f6f60a4acf3`
and `21d9631c7e361584cad656a7eb5332d9332cdd86db7fb1314c08fae8f26e778b`.
Their native extensions are byte-identical, SHA-256
`890e6b4d7023d0ee13ff1a4bfa1b823416297a5ad273dbc5c1d84c2ceabc3f17`.
Fresh collection remains 6,526/6,526, report SHA-256
`c4af5b3d9d20b43af6fd6e64aed4611a5db3c75843b9780aca355ad920ad983e`.

The cumulative ledger advances from 736 to 737 passed, zero failed, seven
skipped and 5,782 not run, SHA-256
`d405874b563e49f99ece20095fffd86e4ff6bb74ebdad303049ae9a05ea80f65`.
Exact comparison changes only the selected Chromium row from not run to passed
and has no losses. The zero-test repeat-ID attempt and the repeat launched from
the upstream checkout, which bypassed the candidate shim and lacked the
upstream driver executable, remain diagnostics and are excluded. The remaining
16 Chromium rows from the original 27-row diagnostic, the rest of Python and
all Core JS/TS work remain open.

## ABI 68 persistent-Context device-emulation checkpoint (2026-10-04)

NODE-311 starts from five unchanged official async Chromium rows in
`test_defaultbrowsercontext.py`: viewport, device scale factor, user agent,
touch and the persistent-Context baseline. The ABI 67 candidate rejects the
new persistent device options before launch. That red report remains
diagnostic and is excluded from the ledger. The first oracle attempt lacked a
bundled Chromium headless shell and is likewise diagnostic; the explicit
installed-Chrome oracle passes 5/5, report SHA-256
`357e9f4a691b7ba3a2a61c157a94b91fdbf73bbc107dfa67b5952f7cf7a08053`.

ABI 68 adds the independently selectable
`persistent-context-device-emulation` leaf and extends only the persistent-
launch tuple. Rust admits the default device policy before initial Page
topology, applies the existing device/user-agent initializer only to that
default Context and counts it against the existing fixed 16-Context / 64 KiB
device registry. Later owned Contexts remain explicitly selected and external
Contexts remain unaffected. The leaf adds no Pwright package beyond the
persistent launch closure.

The first candidate pass exposed one binding discrepancy after browser
behavior already matched: `Page.viewport_size` was absent. The final facade
keeps a two-integer public snapshot, returns a fresh mapping and updates it only
after Rust completes `page_set_viewport_size`; browser emulation, Page/OOPIF
replay, resource admission and cleanup remain Rust-owned.

The final installed candidate passes 5/5, report SHA-256
`5e6b0e6ac472c149f15249550effafc9e226010a77222707aedc9415d01753cd`.
A same-process ten-repeat passes 50/50, report SHA-256
`f6a0c2adc94663adc341b7f41ef5aee025ef94b47c52d35420e34c7e6eb1155e`,
proving persistent Browser sessions, policy capacity and Page snapshots are
reclaimed. The all-feature host gate passes 954 tests with 26 explicit
ignores; addon tests pass 2/2 and all 137 installed-wheel units pass. The unit
report SHA-256 is
`ef11604914e0357f3407cc90b0a66a1361153de19f83ec725bd9f8c9a28c8603`.
Formatting, selected/all-feature release Clippy, Ruff, strict Pyright and the
108-package dependency proof also pass.

Two final ABI3 wheels are normalized-reproducible, with SHA-256 values
`a1ca0c2bb6d80a0e9ac6fadfa3e778077acc3f42cd6ef9d3e0a55101ba798059`
and `654386396048318f4fb8fdfbf6839da2b8c7ee69be19b74a921569370a30987f`.
Their native extensions are byte-identical, SHA-256
`c4d78c799f17dc1955514593fe8132173bf48c273d14a1a2ac4a74cd36703fe0`.
Fresh collection remains 6,526/6,526, report SHA-256
`c568254b65d0976629004a1f7c080fbd9a669a8260741ba95938920023f1b35f`.

The cumulative ledger advances from 731 to 736 passed, zero failed, seven
skipped and 5,783 not run, SHA-256
`d866df9d52c72da0a9b8548c5ddbf5b0daed9010e9adc769f33a64bdc69aa4c3`.
Exact comparison changes only the selected five Chromium rows from not run to
passed and has no losses. The remaining 17 Chromium rows from the original
27-row diagnostic, the rest of Python and all Core JS/TS work remain open.

## ABI 67 persistent-Context `ignore_https_errors` checkpoint (2026-10-04)

NODE-310 starts from the unchanged official async Chromium
`test_should_support_ignore_https_errors_option` row in
`test_defaultbrowsercontext.py`. The ABI 66 candidate rejects the option before
launch; that red report remains diagnostic and is excluded from the ledger.
The independent Playwright 1.58.0 oracle passes 1/1, report SHA-256
`33b552df7d00245aec91b400892594e169660c5e1552457b2886dcdaa6374535`.

ABI 67 adds the independently selectable
`persistent-context-ignore-https-errors` leaf and extends only the persistent-
launch tuple. Rust admits the persistent default policy before initial Page
topology, applies the existing Chromium Security initializer only to that
default Context and counts it against the existing fixed 16-policy capacity.
Later owned Contexts remain explicitly selected and external Contexts remain
unaffected. The leaf adds no Pwright package beyond the persistent launch
closure.

The final installed candidate passes the unchanged row 1/1, report SHA-256
`c483c5ffa8d5fcf9642c7af2da24180f9041e572ee9448866f018dc2d6d363b1`.
A same-process ten-repeat passes 10/10, report SHA-256
`bba576cd1bf1efd2f3a515995412209e2f49a2a113f3800fbe1020c5b395193a`,
proving that both persistent Browser sessions and policy capacity are
reclaimed. Four focused Rust regressions, 952 all-feature host tests with 26
explicit ignores, 2/2 addon tests and all 136 installed-wheel units pass. The
unit report SHA-256 is
`98b9a0e3124a6b9fa0f131318aea5af102fea8724e24437b76a7d329ae81ea57`.
Formatting, selected/all-feature release Clippy, Ruff, strict Pyright and the
108-package dependency proof also pass.

The two normalized-reproducible ABI3 wheel SHA-256 values are
`c153464a16a2083899e44a3637d48d2cf512a9d669675feed27a244df2f65880`
and `f78552d51c1ecc9c2cac8a8b8193896663c71213d487a57903c8d45c184f1387`.
Their byte-identical native extension SHA-256 is
`7f32e973622c1b7e1edbd1465637bebbc94af840972bf5457ee13df500c265eb`.
Fresh collection remains 6,526/6,526, report SHA-256
`f191f88bb6b65eeedb05e2c6deb7892283769cf51372a5d5db03eb5f7cb3c498`.

The cumulative ledger advances from 730 to 731 passed, zero failed, seven
skipped and 5,788 not run, SHA-256
`9100fc0c035aefbb4326bf7a3be6a8a037ba9c332254b12592e2eb75aef4e8d1`.
Exact comparison changes only the selected Chromium row from not run to passed
and has no losses. The remaining 22 Chromium rows from the original 27-row
diagnostic, the rest of Python and all Core JS/TS work remain open.

## ABI 66 persistent-session release checkpoint (2026-10-04)

NODE-309 keeps the exact package/native handshake at ABI 66. It changes no
native export, feature, option, Rust limit or dependency. The correction is in
Python ownership routing shared by async and sync facades.

An isolated diagnostic ran all 27 unchanged async Chromium rows in
`test_defaultbrowsercontext.py`: four passed and 23 failed or errored through
explicit missing product behavior. The already-recorded locale row is one of
the four. The three new baseline rows verify that a user-data directory is
created, profile state survives close and relaunch, and the initial Page URL is
`about:blank`. The remaining 23 rows stay `not-run` in the canonical ledger.

The isolated diagnostic could not reveal cross-case retention because every
case used a new process. A ten-repeat same-process run did: each repeat performs
five persistent launches across the three selected tests. The unrepaired wheel
passed the first ten testcase invocations, then reached
`Python environment session capacity exceeded (16)` during repeat four. It
finished 10/30 passed. This red report is diagnostic evidence and is excluded
from the ledger.

Python now marks only the Browser created for `launch_persistent_context` as a
persistent owner. After its default Context completes ordinary retirement,
`BrowserContext.close()` calls the existing Browser/native close path. Rust
joins the HostSession and removes its exact session from the fixed environment
vector. Successful Browser close also removes the language Browser from
`Playwright._browsers`. Ordinary Context close still leaves its shared
connected Browser open. No background cleanup task, retry or second owner is
added.

Three new AAA ownership regressions and all 135 installed-wheel units pass.
All-feature addon release Clippy, formatting, the 108-package dependency proof,
Ruff and strict Pyright pass. Two final ABI3 wheels are normalized-
reproducible; their SHA-256 values are
`1bdd9b424f21d52def2b82901df4b4069e8c605e946b4476aa8312f8519120ba`
and `58cc07b09356c442130238faf876336164964e9d6ff17429dcdf8024b66ae8d0`.
Their unchanged native extensions are byte-identical, SHA-256
`45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.
The unit report SHA-256 is
`e60812305baa10582a5edf921c0b15301187c27db4c9c18cc8e7e430fc9ec571`.

The independent Playwright 1.58.0 oracle and final installed candidate each
pass the three unchanged baseline rows. Their report SHA-256 values are
`f80814a6fffc245aac4656848e1f75d56eed9abbe186d2144a194b7f3f508a97`
and `6769700527838f035e130c6f3eb3f211f46ff442e6239dd596ebe0404e547653`.
The final candidate ten-repeat passes 30/30 testcase invocations and 50
persistent launches in one process, report SHA-256
`30c8904760185177149aa04a0d83c0761b23d038fbf4ebe70bc1506455fdc75d`.

Fresh final candidate collection is 6,526/6,526, report SHA-256
`193911b5d5396511a9721d7b41224be84680cab6c64a173b2bbee819e7f5176e`.
The cumulative ledger is 730 passed, zero failed, seven skipped and 5,789 not
run, SHA-256
`271e88d71232b76b93e28d186739e1c4bb701008526da99e0bf833844fe7b761`.
Exact comparison with NODE-308 changes only the three selected Chromium rows
from not run to passed and has no losses. Remaining Python and all Core JS/TS
corpus work stays open.

## ABI 66 `wait_for_function` timeout acknowledgment checkpoint (2026-10-04)

NODE-308 keeps the exact package/native handshake at ABI 66. It changes no
native export, feature, public option or Python lifecycle owner. The official
async Chromium file contains six unchanged rows covering timeout, string
predicate, reload/context disposal, interval polling, post-timeout side effects
and invalid polling.

The first ten-repeat installed-candidate stress passed 58/60. Both failures
were `test_should_avoid_side_effects_after_timeout`: one `console.log` from the
current predicate arrived after the public timeout returned. The public
deadline had dropped an in-flight `frame_prepare_handle`; controlled dispatch
correctly preserved native cleanup, but the canceled caller no longer waited
for the command/publication fence that every ordinary attempt uses.

Rust now gives the poll operation a cancellation token. At the public deadline
it cancels that token and awaits only the current operation long enough for the
already-admitted predicate command and ordinary publication fence to settle.
Checks after Handle preparation, after truthiness completion and in the poll
delay prevent another attempt. A Handle that becomes ready after cancellation
is disposed. Stop/disconnect remains authoritative `TargetClosed`. No task,
queue, registry, retained session, transport or dependency is added.

Eight focused Rust regressions pass. The all-feature host passes 950 tests with
26 explicit live/diagnostic ignores and the addon passes 2/2. Formatting,
strict selected/all-feature release Clippy, the 108-package dependency proof,
Ruff and strict Pyright pass. Installed-wheel Python units pass 132/132, report
SHA-256
`94ecb5124e8a05e872db56c9c6f8c3c681892c540e049d0ccdc3288176ed8784`.

Two final ABI3 wheels are normalized-reproducible; their SHA-256 values are
`752e68fd89b54c8dd4cb5a5552ff6c847cf46736fb4d9c70465e9c5b58e57320`
and `a1827bb7227124bb298d94d3b885dc317d7dc16c61fce7aa9560246c5f0a0f86`.
Their native extensions are byte-identical, SHA-256
`45fc56e1138798ad239e06c066b16e6e6ed8c5eab2064e9a7a9e1e92cf975024`.

The independent Playwright 1.58.0 oracle and final installed candidate each
pass all six unchanged rows. Their report SHA-256 values are
`36a7519af81807cd54489285f4fda9537a8f07ac98311fe92d823d74de2b2f3f`
and `a5ca9fe63121b6b6c759cce378cd4c20122139f5ed6b8f49c5ee155fe0324e3a`.
The final candidate ten-repeat passes 60/60, report SHA-256
`9e9ecf7060b6fe91501de9edffba8b58b6a1688b0a298d513c8f9fdbba6da84a`.
The initial 58/60 report is retained as diagnostic evidence and excluded from
the ledger.

Fresh final candidate collection is 6,526/6,526, report SHA-256
`1641542885cda33a6ba4337de7af215e718405a01b0b088929de4d631ad3393d`.
The cumulative ledger is 727 passed, zero failed, seven skipped and 5,792 not
run, SHA-256
`96c068d07bcc0ccce5bb2cea05c5721349f982aa604cd4c2b48488487a83eb2f`.
Exact comparison with NODE-307 changes only the six selected Chromium rows
from not run to passed and has no losses. Remaining Python and all Core JS/TS
corpus work stays open.

## ABI 66 persistent-Context locale checkpoint (2026-10-04)

NODE-307 advances the exact package/native handshake to ABI 66 because the
persistent launch tuple now carries an optional default-Context locale. The
independently selectable `persistent-context-locale` feature composes
`persistent-context` and `context-locale`; executable dependency proof shows
that it adds no Pwright package beyond the existing persistent-Context closure.
Python async and sync facades only forward `locale` before native launch.

Rust pre-admits the default policy before initial Page topology initialization
and counts it against the existing limit of 16 live locale entries, each at
most 256 UTF-8 bytes. Chromium couples UA and language in one method, so a
stateless shared initializer projects the independently retained device and
locale policies and sends at most one `Emulation.setUserAgentOverride` per
selected Page/OOPIF. It preserves a selected device `userAgent`, adds the
locale as `acceptLanguage`, or uses the empty-UA sentinel when locale is the
only policy. The locale initializer separately sends
`Emulation.setLocaleOverride`. The first command controls
`navigator.language`; the second controls ICU/`Intl`. The initial real-browser
run returned `en-US` despite a successful ICU override and exposed the missing
first command. That red report is diagnostic evidence and is excluded from the
ledger.

Seven locale ownership, capacity and exact-ICU-payload regressions plus four
user-agent composition regressions pass. The all-feature host passes 948 tests
with 26 explicit live/diagnostic ignores and the addon passes 2/2. Rust
formatting, strict selected/all-feature release Clippy, the 108-package
dependency proof, Ruff and strict Pyright pass. Two final repaired ABI3 wheels
are normalized-reproducible; their SHA-256 values are
`d05505434a588149a878a1d06657cbfbecd4ff9046ad6cf79787e26fd1c5f870`
and `e3cbc6a377ce20492652baeb8f46ac49f79088dced638bcd8a8bddf40ebabb79`.
The native extension SHA-256 is
`f8e49c7d08b22a639fadc5f1a2b6e44aad1c3ce46de83f9f29763ac379105b3b`.
Installed-wheel units pass 132/132, report SHA-256
`80f3e242669d64d3195713ce91a8e96c40fd8e54540ebc8472638edbd0a5ef51`.
The installed-wheel real-Chrome device-UA-plus-locale composition probe passes
1/1, report SHA-256
`605c4add93edc657048e4d988e2fecbb6bf6517dcc811b380259e95fbaa2bf19`.

The unchanged official Chromium row
`test_should_support_locale_option` passes 1/1 in the final installed candidate
and 1/1 in the independent Playwright 1.58.0 oracle. Their report SHA-256
values are
`0f13bf6b8273589d96d516c78fcd1905c28f24cb714a7f41b0746db3de129fee`
and `22b9dffeb3e442e78ca2cf533b5f91e7c4a20ac4c2b49607bed050324fdafcf9`.
A ten-repeat final candidate run passes 10/10, report SHA-256
`f94944d72ca9336a2c66362650da9b0b6a7a106277e8ab95308634f6aaead108`.
The NODE-306 Context-locale Worker regression stays green on the final wheel,
report SHA-256
`b92e05f862463d0d1681151e837957ce7930e59886c62a91616786ddf90be57d`.

Fresh final candidate collection is 6,526/6,526, report SHA-256
`82626b59cdef96fd9096daf9229205867b65fe0def874103f5f57aeaa0ab6f9c`.
The cumulative ledger is 721 passed, zero failed, seven skipped and 5,798 not
run, SHA-256
`352660d65526e5c2cbabfdf3d7115eb32e3e75ed5dc484b9f18efee66914f053`.
Exact comparison with NODE-306 changes only the selected persistent-locale
Chromium row from not run to passed and has no losses. This remains partial-
corpus evidence; the remaining Python and all unexecuted Core JS/TS rows stay
open.

## ABI 65 Context locale checkpoint (2026-10-04)

NODE-306 advances the exact package/native handshake to ABI 65 because
`Browser.new_context` now admits an optional locale. The independently
selectable `context-locale` feature owns validation, Context-scoped retention,
Page/OOPIF session initialization and removal. Dependency proof shows that it
adds no Pwright package. Python only forwards the optional string through thin
async and sync facades.

Rust admits at most 16 live locale policies of at most 256 UTF-8 bytes each.
Values must be nonempty and contain no control characters. A selected owned
Context sends the exact `Emulation.setLocaleOverride` payload to every Page and
OOPIF session. Chromium shares locale across Pages in one renderer, matching
the upstream implementation's documented constraint; only the exact protocol
error containing `Another locale override is already in effect` is suppressed,
and every other CDP failure propagates. Dedicated and nested Workers inherit
renderer locale and receive no locale command.

Five locale ownership, bounds, error-classification and exact-payload
regressions pass. The all-feature host passes 942 tests with 26 explicit
live/diagnostic ignores and the addon passes 2/2. Rust formatting, strict
selected/all-feature release Clippy, the 108-package dependency proof, Ruff
and strict Pyright pass. Two repaired ABI3 wheels are normalized-reproducible;
their SHA-256 values are
`5748b3dd7310752b8bac8a7a39588b06e706f104f647cc1ac6be30aec3da24b1`
and `7865ee31d2dbdebcabbf18b9e51dc9fd47dcf45e8968a9c5e1f3743785200abb`.
Installed-wheel units pass 131/131, report SHA-256
`293ddb1ba023c74ef2e3682c0632de12e2138488626ab8506a491a5e7069987f`.
The final archive was rebuilt after finalizing its README. Every runtime member
is byte-identical to the browser-tested corrected archive, including native
extension SHA-256
`3cfc824663b084756e8c4db9b3a34b07222947768eb26d26e813796b084108ee`;
only `METADATA`, `RECORD` and CycloneDX identity differ.

The focused unchanged official Chromium row
`test_workers_should_format_number_using_context_locale` passes 1/1 in the
installed candidate and 1/1 in independent Playwright 1.58.0 oracle. Their
report SHA-256 values are
`adc3bef79f6fcf3a7d74525f9b5e22fccf627218c13d7ae7c255aa9758aa78b5`
and `2e7cc0bf2da134da5fc3a6666f77e52dfe425626407e19f627e84647ffc897a1`.
A ten-repeat candidate run passes 10/10, report SHA-256
`42662339aad777d1c042dfd7880325c5a59a20778443bdab2f5008b84bf9ad4f`.
The complete unchanged Chromium Worker file passes 13/13, report SHA-256
`523fa108aaa13c1f7ff529fe50b26c2f6bae46d924ef83fd7ff4c963566ca278`.

Fresh candidate collection is 6,526/6,526, report SHA-256
`87de9dcf0da92a1da6fa886c70947f92747931f04759aff0bd59f67faf73084f`.
The cumulative ledger is 720 passed, zero failed, seven skipped and 5,799 not
run, SHA-256
`20e36211f8f1bc6027c7c6998aa7060cb9d43262eadcaf7eb60561bfde4a66f3`.
Exact comparison with NODE-305 changes only the selected Chromium row from not
run to passed and has no losses. Earlier provisional wheels and setup-only/red
browser attempts remain diagnostic evidence and are excluded from the ledger.
Two final-archive reruns were also setup-only because another active task's
shared-CDP `about:blank` target lacked a ready main world; it never entered the
selected test and does not contradict the byte-identical runtime proof.

## ABI 64 Worker console argument Handle checkpoint (2026-10-04)

NODE-305 advances the exact package/native handshake to ABI 64 because the
selected console notice gains an ordered Handle-descriptor field and
`handle_json_value` now dispatches retained Worker Handles. The independently
selectable `worker-console-argument-handles` feature composes
`worker-console-events` and `worker-handle-evaluation`; executable dependency
proof shows that it adds no Pwright package beyond the existing Worker Handle
closure. The legacy 11-field notice shape remains accepted by Python, while
the selected ABI 64 profile returns the 12-field shape.

Rust arms RemoteObject recovery before parsing any console argument, validates
the complete bounded batch and pre-admits its resource leases before
publishing any Handle. Successful batches are transferred atomically into the
existing Worker Handle registry in source order and retain the exact Worker
owner. Validation, capacity and closed-queue failures leave no partial public
batch and compensate every untransferred object exactly once. Dropping a
queued event releases its adopted Handles before dropping their strong Worker
owner, so native cleanup still has a live target.

`JSHandle.json_value` returns a retained primitive without protocol I/O. An
object Handle invokes the existing bounded serializer in the exact Worker CDP
session and Runtime execution-context unique id; it is never routed through a
Frame or Page main-world alias. Python only converts opaque descriptors and
adapts the public `ConsoleMessage.args` and `JSHandle` facade.

The unchanged official async Chromium row
`test_workers_should_have_JSHandles_for_console_logs` passes 1/1 from the
installed provisional ABI 64 repaired wheel and 1/1 in the independent
Playwright 1.58.0 oracle. Candidate report SHA-256 is
`a35e11498bf94b74445e4b490ff848b56ba5dfbb32ec7248901166c5af909e51`;
oracle report SHA-256 is
`1e8aa023fca1ebf396ead20ca624f03b884f2b608470e621b6e9fdeff1a17747`.
A ten-repeat installed-candidate run passes 10/10, report SHA-256
`857416b027e0c898741d07daa5b81343611d1c55b39289789b391c73ec1d1938`.

The complete unchanged Chromium `tests/async/test_worker.py` candidate run
passes 12/13, report SHA-256
`c15afdc0e2f9bb7cefba76d1c89610455f8a70e5c44a5a59132cf7aed689ceb1`.
The sole remaining failure is independently scoped BrowserContext locale. It
is not converted to a skip or counted as a NODE-305 outcome.

The cumulative 6,526-row ledger is 719 passed, zero failed, seven skipped and
5,800 not run, SHA-256
`51b9b9c6e380774f82c279b7032131ff0fa0aefb59b059ec3deba014e67f3e69`.
Exact comparison with NODE-304 changes only the selected Chromium row from not
run to passed and has no losses. Fresh candidate collection remains
6,526/6,526 rows, SHA-256
`642a9f5f2fcfd338486db1fce733a596618a1586831f6af58c308f3aabb3e81b`.

The installed provisional wheel passes 131/131 Python units, Ruff and strict
Pyright. Five new focused Rust regressions cover ordered atomic adoption,
primitive no-I/O JSON conversion, exact Worker-context object conversion,
queue-close release and capacity-failure recovery. The host passes 937
all-feature tests with 26 explicit live/diagnostic ignores; the addon passes
2/2. Selected/all-feature strict release Clippy, Rust formatting and the
108-package dependency proof pass. The selected profile retains the existing
64-Handle, eight-concurrent-call and 64-cleanup-reservation bounds plus the
console lane's 128 notices, 16 MiB total bytes, 16 arguments per event and
4,096-byte object-id limit.

The provisional behavior-evidence wheel SHA-256 is
`47956878f7a246693a24b31cbf8aa493f6c3aba148b11192760a0cdad99743ba`.
The two final repaired-wheel SHA-256 values are
`6b0ce7e210c44031f228625dcdb5a5850a786298f36fbd1b3d54698a44c3c3c6`
and `870707e752cd95af55b70651aa4cbdb54861158fcaf6cc03fdf3a930e0788a60`;
their normalized payloads are reproducible after normalizing only CycloneDX
identity. The final installed-wheel unit report passes 131/131, SHA-256
`eadd928bedb83495537abda01706fe910140bc67d577d09ca04b28ded94aa24c`.

Three final-wheel browser attempts are excluded from behavior evidence because
none reached the selected test body. The first clean venv lacked the official
oracle package's `pyee` dependency. After that dependency was installed, one
attempt found a pre-existing target without a ready main world and the next
found that another task's noisy shared-CDP page had exhausted pending console
Handle capacity. The other task's page was not closed or altered. Full
BrowserContext locale, remaining Python execution and all Core JS/TS execution
remain unfinished.

## ABI 63 independent Worker Handle evaluation checkpoint (2026-10-04)

NODE-304 advances the exact package/native handshake to ABI 63 because it adds
native Worker Handle evaluation and descriptor dispatch. The optional
`worker-handle-evaluation` feature composes `worker-events` plus `handles`;
the executable dependency proof shows that its Pwright package closure is
exactly Page events plus Handles. Unselected Worker and Handle profiles retain
their previous behavior.

Rust retains each Worker's exact CDP session and Runtime execution-context
unique id. `Worker.evaluate_handle` evaluates in that pair, publishes an
opaque Handle owner and compensates an unpublishable RemoteObject through the
bounded cleanup executor. `JSHandle.get_property` reuses the native Handle
evaluation path. No Worker value is routed through a Frame or Page main-world
alias. The public Handle core references its Worker owner weakly, so a disposed
descriptor cannot retain the Worker session, CDP connection or Browser
shutdown barrier. Python only adapts the two public calls and error shapes.

The unchanged official async Chromium row
`test_workers_should_emit_created_and_destroyed_events` passes 1/1 from the
final installed repaired wheel and 1/1 in the independent Playwright 1.58.0
oracle. Candidate report SHA-256 is
`aead603f57778f8cd6939a1a340006ba768a0c33aa475dff4dce5ae379cfb415`;
oracle report SHA-256 is
`512eb5d9416563d9166ff74ef0b8e394c863fe36f64ee4200c492ed2d812fc6a`.
A final-wheel ten-repeat candidate run passes 10/10, report SHA-256
`fdd3c72e42ec4112b13a504308893e5bf923236701610bec27d1db6803adfa98`.

The complete unchanged Chromium `tests/async/test_worker.py` candidate run
passes 11/13, report SHA-256
`1ded7bec96dbd0ae616d70f7a5b7ddca404f3e355c85fcf5bb6f0652aa430d17`.
The two remaining failures are explicit: Worker console argument Handles and
BrowserContext locale. They are not converted to skips or counted as
NODE-304 outcomes. Worker-created Handle methods beyond the selected lifecycle
and `get_property` slice also remain outside the compatibility claim.

The cumulative 6,526-row ledger is 718 passed, zero failed, seven skipped and
5,801 not run, SHA-256
`e0dfeb150e8e410f6c4b4f62968330a9313023300d745d20f762def57bcc9501`.
Exact comparison with NODE-303 changes only the selected Chromium row from not
run to passed and has no losses. Fresh candidate collection verifies
6,526/6,526 rows, SHA-256
`642a9f5f2fcfd338486db1fce733a596618a1586831f6af58c308f3aabb3e81b`.

The final installed wheel passes 131/131 Python units, Ruff and strict
Pyright. Five focused Rust regressions cover exact Worker context ownership,
bounded publication, property acquisition, disposal and weak-owner teardown.
The host passes 932 all-feature tests with 26 explicit live/diagnostic ignores;
the addon passes 2/2 tests. Selected/all-feature strict release Clippy, Rust
formatting and the 108-package dependency proof pass. The selected profile
advertises 64 Worker Handles, eight concurrent Worker Handle calls and 64
cleanup reservations. Two repaired ABI3 wheel SHA-256 values are
`88481e8e363eddd22de51e075579918158b3900aef1aeed3de6e9ec7f1a47215`
and `7163f0e7dd52412ab3d78e8a65c94d8702ba58de0c14ebe806a00fd0eb609692`;
their normalized payloads are reproducible after normalizing only CycloneDX
identity.

The first two final-named focused attempts are excluded because fixture setup
stopped before target behavior: one inherited an unready blank target and one
inherited a noisy restored automation page that exhausted console Handle
publication. The first final stress selector collected zero tests and is also
excluded. Earlier development/hang diagnostics remain excluded. Full Python
and Core JS/TS execution remains unfinished.

## ABI 62 independent Worker-network-event checkpoint (2026-10-04)

NODE-303 keeps ABI 62 and selects `worker-network-events` over
`worker-events`, `request-events` and `response-events`. It changes no upstream
test, native method or public descriptor. Rust enables Network on retained
Worker sessions before debugger resume and registers each Worker or nested
Worker as a bounded related session of its owning Page. The existing Page
driver handles those events through one cursor and one adapter; native request
keys are exact `(session, request-id)` pairs and all scopes share the Page's
64-active-request limit. Detach drains only the retired scope.

Python reuses the existing Browser network watcher. Weak Page identity maps
make Request and Response publication canonical across Page/Context callbacks,
waiters and `response.request` without retaining completed traffic strongly.
The sync facade preserves the corresponding wrapper identity. No Python CDP
reader, polling loop or second network queue was added.

The two unchanged official async Chromium rows
`test_workers_should_report_network_activity` and
`test_workers_should_report_network_activity_on_worker_creation` pass 2/2
from the final installed candidate and 2/2 in the independent Playwright
1.58.0 oracle. Candidate report SHA-256 is
`42e5087226d06cb439d66132b3aada80a1d68586821bbc243949355cf4b29dab`;
oracle report SHA-256 is
`7b88bbb11f719ed0330c05eddb610c8950782b832de73483b56f4060a6f7e65d`.
A ten-repeat candidate run passes 20/20, report SHA-256
`cc5c71028a5c0ff50c45458011050e8ebdbc0ebe2521724a6e6df252e8505c8a`.

The complete unchanged Chromium `tests/async/test_worker.py` file now passes
10/13, report SHA-256
`b13c2b50c32e45a4ed5bdd4518b211583ebe1458625b09bf4cf717718f5b8d3c`.
The remaining failures are explicit: `Worker.evaluate_handle`, Worker console
argument Handles and BrowserContext locale. They are not converted to skips
and are not counted as NODE-303 outcomes.

The cumulative 6,526-row ledger is 717 passed, zero failed, seven skipped and
5,802 not run, SHA-256
`361e34e330c5fd5908802e561f126743e2a42f911f34d762283633a0333efbff`.
Exact comparison with NODE-302 changes only the two selected Chromium rows
from not run to passed and has no losses. Fresh candidate collection verifies
6,526/6,526 rows, SHA-256
`d43ff5a1d5478273618fec81cb0ee5d23c81d5f60ae67590eb5f3815c8ed9efe`.

The final installed wheel passes 130/130 Python units, Ruff and strict
Pyright. Rust verification passes six focused regressions, 927 all-feature
host tests with 26 explicit live/diagnostic ignores and 2/2 addon tests.
Selected/all-feature release Clippy, formatting and the 108-package dependency
proof pass. Two repaired ABI3 wheel SHA-256 values are
`4b97facee24ba3484b484b5361cf309dfa615a5f53098601feee81bb6950f9b1`
and `06986a96e172727ef31de0bc56ebeb72b04d8cb0be270c32a6f85ca52f016997`;
their normalized payloads are reproducible.

Three development reports are excluded from final claims: the first
final-named attempt stopped during fixture setup because a stale blank Chrome
target had no ready main world; the next reached the target and exposed the
fixed `response.request` wrapper-identity mismatch; the passing preflight used
a wheel built before the final README and Clippy-only source cleanup. They are
retained as diagnostic evidence. Full Python and Core JS/TS execution remains
unfinished.

## ABI 62 independent Worker-error-event checkpoint (2026-10-04)

NODE-302 keeps the exact package/native handshake at ABI 62 because it adds no
native method or descriptor. The optional `worker-error-events` feature
composes `worker-events` and `page-errors`; neither prerequisite changes when
the leaf is unselected, and dependency proof shows that the selected leaf adds
no Pwright package beyond the existing `page-events` closure.

The Rust Page-error observer continues to resolve an exception through its
Frame execution route or Page target session first. If neither route owns the
event, the selected leaf looks up the exact retained Worker session and uses
that owner's Page. The exception then traverses the existing bounded PageError
plan, ordered Page lifecycle publication fence and 200-errors-per-Page/
16 MiB history. There is no second event FIFO, polling task or resource
allowance. The existing Python Browser-scoped Page-error watcher converts and
publishes the same public error path, so Python gains no CDP reader or new
queue.

The unchanged official async Chromium row
`test_workers_should_report_errors` passes 1/1 from the final installed
candidate and 1/1 in the independent Playwright 1.58.0 oracle. Candidate
report SHA-256 is
`4b426debbd1724ad71de797c2f539bed0b2e6915a990e9a8bbf5e9d721bd83fb`;
oracle report SHA-256 is
`2b71de897545134ca5ff8727743bad9eaf38b875866188c9322a3e88220b9b3c`.
A ten-repeat final candidate run passes 10/10, report SHA-256
`90571679bf40d10f6a7592f4301f7b867e73f0cfdebcd2fe6bb27066a8fc7c81`.

The cumulative 6,526-row ledger is 715 passed, zero failed, seven skipped and
5,804 not run, SHA-256
`caf76e34c05bd6ce6da5b3467fc142f7abdaf05c545f8cf94f69578e0fe2a539`.
Exact comparison with NODE-301 changes only that Chromium row from not run to
passed and has no losses. The final installed wheel passes 129/129 Python
units, Ruff and strict Pyright. All-feature Rust verification passes 927 host
tests with 26 explicit live/diagnostic ignores and 2/2 addon tests;
selected/all-feature strict release Clippy and the 108-package dependency
proof also pass.

Two repaired ABI3 wheel SHA-256 values are
`2db7c264dcc10c34f52103ca575df0a28812084c3423630ef2b098c98dc08054`
and `5e8f6cd0d851b9ab841b104230ee958ee4c631e77c2b46cfc5e445bc555ab9c5`;
their normalized payloads are reproducible after excluding only CycloneDX
UUID/timestamp metadata. Worker console argument Handles,
`Worker.evaluate_handle`, Context locale, Worker network attribution, the
other 5,804 Python rows and the complete Core JS/TS corpus remain unresolved
or unexecuted as recorded.

## ABI 62 independent Worker-console-event checkpoint (2026-10-04)

NODE-301 advances the exact package/native handshake because it exports a new
bounded Worker-console notice read. `worker-console-events` is independently
selectable over `worker-events` and `console-events`; dependency proof shows
that it adds no Pwright package beyond the existing `page-events` closure.

Rust observes `Runtime.consoleAPICalled` only on an exact retained Worker
session, reuses the bounded console value/location codec, and owns the entire
FIFO. The leaf admits at most 128 notices and 16 MiB of event bytes, plus 128
bounded cleanup jobs. It accepts at most 16 argument descriptors and 4 KiB of
retained object IDs per event. RemoteObjects not yet exported as public Handles
are released through the exact Worker session; teardown clears the console
leaf before the Worker owner registry. Python runs one Browser-scoped native
watcher, reuses or publishes the exact Worker facade, and emits one shared
`ConsoleMessage` instance to Worker, Page and Context. The public
`ConsoleMessage.worker` points to that facade. Python does not subscribe to CDP,
retain a second FIFO, or fabricate argument Handles.

The three unchanged official async Chromium rows
`test_workers_should_report_console_logs`,
`test_worker_should_report_console_event` and
`test_worker_should_report_console_event_when_not_listening_on_page_or_context`
pass 3/3 from the final installed candidate and 3/3 in the independent
Playwright 1.58.0 oracle. Candidate report SHA-256 is
`d5a556b3377b7c34f73f2fb9be9f088988c317569cfa9097a019f75973415987`;
oracle report SHA-256 is
`e7128df0d47ad0936b15dbfdf903ff15a6db45c077f697819cc2f7ac82f90829`.
A ten-repeat candidate stress run passes 30/30, report SHA-256
`5c1320a89c96fa8c4355940f5ff7bc1b11d65b7e2b1c1111cf4c2f629836dbef`.

The cumulative 6,526-row ledger is 714 passed, zero failed, seven skipped and
5,805 not run, SHA-256
`ef648ecece0f94e5f7a3968e6d652a08d1bb5eb56b8bf8c1afd7045f6d6b902c`.
Exact comparison with NODE-300 changes only those three Chromium rows from not
run to passed and has no losses. The final installed wheel passes 129/129
Python units, Ruff and strict Pyright. All-feature Rust verification passes 926
host tests with 26 explicit live/diagnostic ignores and 2/2 addon tests;
selected/all-feature strict release Clippy and the 108-package dependency proof
also pass.

Two repaired ABI3 wheel SHA-256 values are
`739d49ccdffd7c2e1685e0dd7d2ae6948718fe6b59758924826b7be2027217a5`
and `efd120f8b456e3734f0eae414fa35e4b4de71c9dd9a6b42904214917f8f0af46`;
their normalized payloads are reproducible after excluding only CycloneDX
UUID/timestamp metadata. `test_workers_should_have_JSHandles_for_console_logs`
remains open: Worker argument Handles are a separate capability and empty
facades are not counted as compatible. Worker errors, `Worker.evaluate_handle`,
Context locale, Worker network attribution, the other 5,805 Python rows and the
complete Core JS/TS corpus remain unresolved or unexecuted as recorded.

## ABI 61 Worker-retirement checkpoint (2026-10-04)

NODE-300 keeps native ABI 61 and closes a cross-boundary ownership leak during
shutdown. The exact race is Page removal before a late Worker detach notice.
The Python watcher previously classified that notice as an unknown-Page error;
the retained exception traceback then held its local native Worker owner and
CDP connection. Page and Context close appeared complete, but
`browser.close()` could not cross Rust's `io_finished` barrier.

Page, Context and Browser terminal paths now synchronously retire every public
Worker facade: the facade is marked closed, its native owner is released and
the public close event is emitted at most once. Evaluation after retirement
raises `TargetClosedError`. A late native close notice releases an existing
facade normally; a notice for a Page already removed by an earlier terminal
path is discarded as an exact stale lifecycle receipt instead of becoming a
stored watcher error. No synthetic CDP detach is fabricated, and Rust remains
the owner of actual target/session detach and transport shutdown.

The final installed wheel passes 128/128 Python units, Ruff format/lint and
strict Pyright. Two repaired ABI3 wheel SHA-256 values are
`aecd520722b35db70ee096eb030fbb55662e37d4c479a3efbffc050b6c26329c`
and `be4ed3912c802d2b30ce11d99361428c67740386266c5acd8e0301fa13913032`;
their normalized payloads are reproducible after excluding only CycloneDX
UUID/timestamp metadata.

Four unchanged official async Chromium rows pass from that installed wheel:
`test_workers_page_workers`,
`test_workers_should_clear_upon_cross_process_navigation`,
`test_workers_should_clear_upon_navigation` and
`test_workers_should_evaluate`. The focused report SHA-256 is
`601621d526157d550fbdfd92dabf4546fb1574cef2da5f241382900a827470b2`.
The cumulative 6,526-row ledger is 711 passed, zero failed, seven skipped and
5,808 not run, SHA-256
`cfcddd468eef3ef754910e1c7055ffbbbbc4c6454546589ff74323999acbf4fb`.
Exact comparison with NODE-299 changes only these four rows from not run to
passed and has no losses.

The wider 13-case Worker run remains diagnostic evidence and is not merged
into the authoritative ledger: earlier per-case timeouts polluted later cases
in its shared browser. A separate clean execution confirms that Worker network
request/response attribution is genuinely missing. Worker console and error
publication, `Worker.evaluate_handle`, BrowserContext locale and Worker network
attribution remain explicit capability gaps. The other 5,808 Python rows and
the complete Core JS/TS corpus remain unexecuted or unresolved as recorded.

## ABI 61 caller-dispatcher checkpoint (2026-10-04)

NODE-299 keeps native ABI 61 and closes the last discrepancy in the unchanged
Chromium `tests/sync/test_sync.py`. A caller-thread asyncio loop now exposes the
same `_dispatcher_fiber.switch()` contract used by the official sync fixture.
When a sync call is made from a child greenlet, the facade submits its
coroutine to Pwright's existing background loop, yields to the caller loop and
resumes only after the concurrent future completes. Browser/CDP work does not
move into Python. The existing semaphore limits these pending calls to 32;
ordinary root-greenlet calls retain the direct blocking path. Stop and failed
start clear the running-loop marker, cancel caller helper tasks, close both
loops and release the owning thread.

`greenlet>=3.1.1,<4` is now an explicit distribution dependency. A fresh
environment installed it transitively from the repaired wheel, reported ABI
61 and passed 127/127 Python units. Ruff and Pyright pass. Two repaired wheels
have SHA-256 values
`eb078b9c0da926a28dd263fe831336839176e700ee6b90fa00fec207e8241d68`
and `ea38b110a1af14b0d88ae310b8c6c783a41dbe3ba37758475b5cc864d0115705`;
their payloads are reproducible after excluding only CycloneDX UUID/timestamp
metadata.

The unchanged focused
`test_close_should_reject_all_promises[chromium]` passes from the installed
wheel, report SHA-256
`d5f6c4cedff7275d14559de7ccd33acd8af488c42584d8609742375419ff2a07`.
The complete unchanged Chromium sync file passes 27/27, report SHA-256
`6a77352e928c17da5ef05c3eb0e17c23c1a74089573df8697bb8473af3dd29ec`,
matching the independent oracle's 27/27 result for that file. The candidate
cumulative ledger is 707 passed, zero failed, seven skipped and 5,812 not run,
SHA-256
`a43bef1f0b558f324191e12cb2dc28d713d9108466c3bf68bf0edf09bc9bddc3`.
Exact comparison with NODE-298 changes only this row from failed to passed and
has no losses. This is not a full-corpus result: the other 5,812 Python rows and
the complete Core JS/TS corpus remain unexecuted or unresolved as recorded.

## ABI 61 Worker-event checkpoint (2026-10-04)

NODE-298 adds independently selectable dedicated-Worker publication and
evaluation without moving target ownership or CDP traffic into Python.
`worker-events` has the same 108-package Pwright dependency closure as
`page-events`. Rust retains only Worker children of a known Page/OOPIF or an
already retained Worker, records exact Page ownership and creation order,
enables Runtime plus recursive auto-attach before debugger resume, and waits
for both the execution context and `Inspector.workerScriptLoaded` before
evaluation. Exact `Target.detachedFromTarget` retires the owner and publishes
closure. The leaf is bounded at 64 live owners, 128 FIFO notices, 1 MiB Worker
URLs and eight concurrent bounded value evaluations.

Python runs one native watcher per Browser and adapts only public async/sync
`Worker` objects, Page snapshots, callbacks, predicates and evaluation. The
final installed ABI 61 candidate passes the unchanged official
`tests/sync/test_sync.py::test_sync_workers_page_workers[chromium]`; focused
report SHA-256 is
`bb036594265e433f36cbfe291872531b1a723addbe05a72f187eee3d60bffd72`.
The complete unchanged Chromium `tests/sync/test_sync.py` passes 26/27,
report SHA-256
`6dd94f8af4853667f7254b8f85fbe52e1c3510196d771430b9f17d4ffdb91298`,
versus the unchanged independent-oracle result of 27/27 at SHA-256
`ffce16c779b1317d27b1eab2526c18dcbdfb27d57686258c1baa7105466bf0fe`.
The only remaining candidate failure is the upstream `sync_gather` helper's
assumption that the dispatcher loop runs on the caller thread.

The candidate cumulative ledger is now 706 passed, one failed, seven skipped
and 5,812 not run, SHA-256
`67413210a814c04723bc62aa595210856f43891ed23e615622817b5465736a36`.
The oracle cumulative ledger remains 688 passed, zero failed, seven skipped and
5,831 not run, SHA-256
`62346332e34528c04ffe1dc84cadf130be41911c01311922e3d5485e3e082ec3`.
An exact outcome-only audit against ABI 60 finds one change,
`test_sync_workers_page_workers[chromium]` from failed to passed, and no
losses. The ledger uses the non-overlapping complete sync-file report.

Two repaired ABI3 wheels have SHA-256 values
`dd3fc5ef46cac61ee3a54321f04fffe72d86fba962fcc69194e0bb65d0846158`
and `07bd3a2845afc60027a69a488714d7cf2e163ec2369b9fc23c2257bd011850d9`;
their payloads are reproducible after excluding only CycloneDX UUID/timestamp
metadata. Python units pass 125/125. The standalone Worker owner/readiness gate
passes 3/3 and the topology retention/initialization regression passes 1/1.
Strict selected and all-feature release Clippy, Ruff, Pyright and the
108-package dependency proof pass. The complete Python and Core JS/TS corpora
remain unfinished.

## ABI 60 download-event checkpoint (2026-10-04)

NODE-297 adds independently selectable successful download publication without
moving download policy, event correlation or artifact lifetime into Python.
`download-events` has the same 108-package Pwright dependency closure as
`page-events`. Rust installs `Browser.setDownloadBehavior` for each opted-in
Context, maps `Browser.downloadWillBegin` from its native Frame to the portable
Page/Context, retains GUID-named temporary artifacts, settles exact owners from
`Browser.downloadProgress`, and removes Context storage during cleanup. The
leaf is bounded at 16 Contexts, 64 owners, 64 FIFO notices and 4,096-byte paths.
Python runs one native watcher per Browser and adapts only public async/sync
`Download` objects. The shared Locator click navigation barrier now treats a
same-Frame download as the successful terminal outcome of the requested
navigation, so a download-producing click does not wait for a nonexistent
`Page.frameNavigated` event.

The final installed ABI 60 candidate passes the complete unchanged official
`tests/sync/test_sync.py::test_sync_download[chromium]`, including suggested
filename, on-disk path and `Hello world` payload assertions. Its focused report
SHA-256 is
`e7fbbce7744aabfea9cdf1bd9c6cfa2d0e02c6306e7e4b579c94ff327e2b261a`.
The complete unchanged Chromium `tests/sync/test_sync.py` passes 25/27,
report SHA-256
`a102e15152609462005de228fa92cdac804f3fc24de6cfb5a589ccbd4e92ff26`,
versus the unchanged independent-oracle result of 27/27 at SHA-256
`ffce16c779b1317d27b1eab2526c18dcbdfb27d57686258c1baa7105466bf0fe`.
The two remaining candidate failures are Worker events and the upstream
`sync_gather` helper's assumption that the dispatcher loop runs on the caller
thread.

The candidate cumulative ledger is now 705 passed, two failed, seven skipped
and 5,812 not run, SHA-256
`c7c6e0f6a827db762291614bbfa127205cec572b1c1328250ad1dc1e71da6f9b`.
The oracle cumulative ledger remains 688 passed, zero failed, seven skipped and
5,831 not run, SHA-256
`62346332e34528c04ffe1dc84cadf130be41911c01311922e3d5485e3e082ec3`.
An exact outcome-only audit against ABI 59 finds one change,
`test_sync_download[chromium]` from failed to passed, and no losses. The
focused report is supporting evidence only; the ledger uses the non-overlapping
complete sync-file report and contains no stale ABI 59 sync-file rows.

Two repaired ABI3 wheels have SHA-256 values
`e84ee8646f093d676b28fcb8a507040cbefb2c1802dc519f90f496120c6744f4`
and `0b152147e6eb49901b113ec7541d88a0343248466373b806c68750be29d294e0`;
their payloads are reproducible after excluding only CycloneDX UUID/timestamp
metadata. Python units pass 123/123. The standalone download owner/publication
gate passes 3/3 and the Locator download-navigation regression passes 1/1.
Strict selected and all-feature release Clippy, Ruff, Pyright and the
108-package dependency proof pass. The complete Python and Core JS/TS corpora
remain unfinished.

## ABI 59 request-event checkpoint (2026-10-04)

NODE-296 adds independently selectable Page request-event publication without
coupling it to response events or request history. `request-events` selects the
existing `network-events` transport and nothing else; the dependency proof
shows the same 108-package closure as `network-events`. Rust owns the 16-live-
Context subscription set, Page association, retained request/history owner,
256-notice FIFO, shared 16 MiB network queue accounting, topology cleanup and
terminal Browser cleanup. Python runs one native watcher per Browser and only
maps a notice to the exact Page plus async/sync `Request` wrapper.

The final installed ABI 59 candidate passes the unchanged official
`tests/sync/test_sync.py::test_sync_network_events[chromium]`. Its focused
report SHA-256 is
`a96e49de4e1d893f7683ab82deeac0aebeb02616faaa128dc93bf991293c16ff`.
The complete unchanged Chromium `tests/sync/test_sync.py` passes 24/27,
report SHA-256
`e8e09ddb45aaf4ddc68653bdba6431df7c44fe0b589af393789d84a33e2fd320`,
versus the unchanged independent-oracle result of 27/27 at SHA-256
`ffce16c779b1317d27b1eab2526c18dcbdfb27d57686258c1baa7105466bf0fe`.
The three remaining candidate failures are successful download/artifact
publication, Worker events and the upstream `sync_gather` helper's assumption
that the dispatcher loop runs on the caller thread.

The candidate cumulative ledger is now 704 passed, three failed, seven skipped
and 5,812 not run, SHA-256
`5a4815de54d0993e383f53e960378cec14199a86753379702ddf167b23aba983`.
The oracle cumulative ledger remains 688 passed, zero failed, seven skipped and
5,831 not run, SHA-256
`62346332e34528c04ffe1dc84cadf130be41911c01311922e3d5485e3e082ec3`.
An exact outcome-only audit against the ABI 58 candidate ledger finds one
change, `test_sync_network_events[chromium]` from failed to passed, and no
losses. The focused one-row report is supporting evidence only; the cumulative
ledger uses the non-overlapping complete sync-file report and contains no stale
ABI 58 sync-file rows.

Two repaired ABI3 wheels have SHA-256 values
`c4d90b75345d401d91820a8df0e7125ed6c8f042c0a7e969904d74577e636f50`
and `a335c4f2804328ec1923b9b170a58305a7052ba3780ffaa5cf7eeea726951a6d`;
their payloads are reproducible after excluding only CycloneDX UUID/timestamp
metadata. Python units pass 120/120. The standalone request-event Rust gate
passes 8/8 and the combined request-history/request-events/response-events/
response-finished gate passes 16/16. Strict selected and all-feature release
Clippy, Ruff, Pyright and the 108-package dependency proof pass. The complete
Python and Core JS/TS corpora remain unfinished.

## ABI 58 complete console-module checkpoint (2026-10-04)

NODE-295 runs the complete unchanged Chromium expansions of
`tests/async/test_console.py` and `tests/sync/test_console.py` from pinned
Playwright Python 1.58.0 commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`. The source SHA-256 values are
`f4c8a2b339e845cdea9230da5ce1697e0348a9d9bd62f48593359524b801a869`
and `4b4249cb0f12fe45869bf31884dfc541346c49c153ea94d5f032af71dff5ac04`.
No assertion, fixture or source test is changed.

The final installed ABI 58 candidate passes 8/8 async and 8/8 sync. Report
SHA-256 values are
`d96db0250be5b89956f49e346d67af04fde065ef1224130f85ee5ce1a1876faf`
and `f4e16451571da8029f0d5695e9ca99b8e12201bd35b07a196de4a99fd56184f0`.
This includes real retained argument Handles whose `json_value()` results are
`"hello"`, `5` and `{ "foo": "bar" }`, the full console-type order, CORS Log
translation, location, duplicate events and detached-iframe cleanup.

The independent official oracle passes 8/8 async, report SHA-256
`f95ac8f7734c44381d45277059a21938fcd63954b092c628d1ca20d710892172`.
Its first sync module run records 7/8 because the location waiter receives the
late favicon 404 from the preceding empty page; that report is retained at
SHA-256
`a1fdcc2aaeae1e86f406cb1ef6d7ae9a02de0e44a6895b68265aaeb4645895dd`.
The exact failed oracle row immediately passes at SHA-256
`021e90ab4463352680e87826794059ac6afcded79376789fcb59ea11b80898ac`,
and a complete sync retry passes 8/8. The formal merge of the failed run with
that one exact retry is 8/8 at SHA-256
`cae865b9f04f32e4148dc810233b6fb6168ab9b0f187e650fba152f21f49b929`.
This keeps the upstream timing flake auditable; it is not silently removed or
charged as a candidate discrepancy.

The final wheel also runs the complete unchanged Chromium
`tests/sync/test_sync.py`: candidate 23/27, report SHA-256
`5064cb02833dcc487feb4aa2f06f329944f72765c70230458492e0b834e49df5`,
versus oracle 27/27, report SHA-256
`ffce16c779b1317d27b1eab2526c18dcbdfb27d57686258c1baa7105466bf0fe`.
The four remaining candidate failures are request events, successful download
publication, Worker events and the upstream `sync_gather` helper's assumption
that Playwright's dispatcher event loop runs on the caller thread. The fixed
rows cover console representation/delivery, nested sync-API rejection,
wait-for-event timeout mapping and post-stop error mapping.

The candidate cumulative ledger is 703 passed, four failed, seven skipped and
5,812 not run, SHA-256
`e1a09cc45a5eb609aac2884804f4514eea66dcdf17c80d3c063b44f55d8d2b35`.
The oracle cumulative ledger is 688 passed, zero failed, seven skipped and
5,831 not run, SHA-256
`62346332e34528c04ffe1dc84cadf130be41911c01311922e3d5485e3e082ec3`.
An exact outcome-only audit shows 21 candidate improvements and no losses: 16
async/sync Chromium console rows change from not-run to passed and five
`test_sync.py` rows change from failed to passed. The oracle audit has 17
improvements and no losses: the same 16 console rows plus the nested-sync row.
Firefox and WebKit expansions stay not-run provider gaps.

ABI 58 publishes console arguments through the existing bounded Rust Handle
owner and adds uniquely tokened Page and BrowserContext waiter state plus
immediately active persistent listeners. Navigation waiters become active on
the next main-frame commit; ordinary actions use their command boundary. A
real-browser no-further-command smoke proves delayed console events reach a
registered Page listener. Python only manages listener identity, exact waiter
tokens and native value wrappers. Two final repaired wheels have SHA-256 values
`d973dd55efd049dfea9e4a0281fc9c472842e192343acd28139c270645f0c50d`
and `33b4787e562704afb02077a4c1b7240690fd98ef9099cb1ff9e7550f03c2042e`;
their normalized payloads are reproducible. Python units pass 118/118, the
two focused Rust subscription tests pass, strict release Clippy/Ruff/Pyright and
the 108-package dependency proof pass. The remaining Python rows and complete
Core JS/TS corpus remain unfinished.

## ABI 57 console-event projection checkpoint (2026-10-03)

NODE-294 selects the existing bounded Rust console observer through the
ordered Page lifecycle publication and adds thin async/sync Python event
wrappers. ABI 57 carries type, text, diagnostic argument JSON, location and
Page identity in one bounded native descriptor. `ConsoleMessage.args` is not
claimed: the official property requires retained Rust-owned JSHandles, not
deserialization of the diagnostic JSON snapshot.

Release extension build, all-feature strict Clippy, the 108-package dependency
proof, Ruff, strict Pyright and 112/112 Python units pass. The unchanged
Chromium rows `test_console_repr` and `test_console_should_work` remain
unexecuted in candidate and oracle because the shared dedicated Chrome's
zero-tab preflight still sees an unrelated Gemini page. Therefore the latest
recorded candidate remains 682 passed / nine failed / seven skipped / 5,828
not run with SHA-256
`a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`;
no browser or ledger claim is inferred from source gates.

## ABI 56 ElementHandle DOM checkpoint (2026-10-03)

NODE-291 reruns the two ElementHandle failures from the unchanged sync file
against the final installed wheel. Rust owns relative selector compilation,
existing-handle evaluation, result classification, acknowledged no-match
release and `HTMLElement.innerText`. Python exposes only thin async/sync
methods. Both rows pass; final report SHA-256 is
`55449eed1e22968a614813cec9eb8daab124a1aec6b22b1342c6deed361f3dbd`.

Replacing those exact outcomes in the 25-row report produces 16 passed / nine
failed, SHA-256
`c118e64d7029d39463f1d1a8212398b1b545bc76f6cb1f88bcff5a1871619fb8`.
The complete file is now candidate 18/27 and oracle 26/27. Tranche and
cumulative candidate ledger SHA-256 values are
`6fb7ddee2fc902d4f142d8f6da3e7b888ea1f2f07f1c84055c4618ae266c6f1e`
and
`a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`.
The cumulative candidate is 682 passed, nine failed, seven skipped and 5,828
not run. Oracle accounting is unchanged.

Normalized-reproducible ABI 56 wheel SHA-256 values are
`faba2f1c41352d1548e3e94505a3c4ff1ce4b2da39dc8d1b26861355e74888b7`
and
`c5501536fbdfc473c71fde14c313d68f6051645c6865fcd6df72c1d509364349`.
Installed Python tests pass 111/111 with five expected skips; focused release
Rust tests pass 2/2; release Clippy, Ruff and strict Pyright pass. Fresh
candidate collection remains 6,526/6,526, report SHA-256
`f3374fe294c20aff18ae1d279acf92ebf70cbe2d0af13813173151eafd5f6a42`.
Nine sync failures, 5,828 not-run Python rows and all 48,477 Core JS/TS
invocations remain explicit.

## ABI 55 sync API inventory checkpoint (2026-10-03)

NODE-290 pins unchanged sync `tests/sync/test_sync.py` at SHA-256
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`
and selects all 25 Chromium rows still marked `not-run`. Two other Chromium
rows in the 27-row file were already green. The complete new oracle report is
24 passed / one failed: `test_sync_playwright_multiple_times[chromium]` does
not raise its expected sync-inside-asyncio error in the isolated official
harness. Oracle report SHA-256 is
`3ce79c1d557e9593ba2c4f63db88e3d17db4231e7544516baa19b1e7c73bcb63`.
The installed candidate baseline is 9 passed / 16 failed, SHA-256
`d8bb9824057c1381c2d32158d84c5d92570ea743f69f60a5ebc68ed36dc99626`.

The repair is deliberately narrow. Page, BrowserContext, Browser and
BrowserType gain the exact public repr shapes. Pwright has no bundled browser,
so `BrowserType.executable_path` is the honest empty string. The official shim
alone exposes `_impl_obj` because the unchanged upstream assertion uses that
private object to construct its expected text. Separately, Rust already owns
the exact Request associated with a Response through `RequestHistoryOwner`.
The host now attaches that owner to response waits and response notices; PyO3
publishes its Request record and async/sync Python only wrap it as
`Response.request`. Python does not correlate IDs or reconstruct network
semantics.

All five repaired rows pass from the final wheel. Merging those exact results
into the full baseline yields 14 passed / 11 failed, SHA-256
`d9b84034f306be4f0bc61ceb9438a2aa79e19ce470c0906981ba17507514696f`.
Together with the two earlier passes, every Chromium row in the file has an
outcome: candidate 16/27 passed and oracle 26/27 passed. Candidate/oracle
tranche-ledger SHA-256 values are
`092b98461c8def4cb2d54b3a463f42b9231e7a8ab74636b9299604a8f2f07177`
and
`cf0b46f3d4d2166e08e3353a4a56fee35ee4f3b8ec8d32bb9e9d682e8cf074ea`.
The cumulative candidate is 680 passed, 11 failed, seven skipped and 5,828 not
run, SHA-256
`099db2a6f4937871240a3fb7c329606cb2f9c81fccb58932534bc3c0ef45116e`;
the cumulative oracle is 671 passed, one failed, seven skipped and 5,847 not
run, SHA-256
`5e0133285e5a74230173ec2df55f001c5424413a3b10f676cf9a2b3973c33a93`.

Two normalized-reproducible ABI 55 wheel SHA-256 values are
`bcb27c0bdf17422d0589426c1d8e992aa008f24ff8fe4e49c1bd9a8469d4c99a`
and
`f96c9dbbe14c606f7bca41f8310e03947007814e7e2039671703f247c1e8d9ae`.
Installed Python tests pass 109/109 with five expected skips; focused Rust
network-history tests pass 10/10; release Clippy, Ruff and strict Pyright pass.
Fresh candidate collection remains 6,526/6,526, report SHA-256
`823aa604740e27cd7b0bde70e86dd8d8098077cc4dc7781344942db8139f5fd1`.
The remaining 11 sync failures stay explicit: one is oracle-red and ten are
candidate-specific stopped-loop, concurrent-close, console, download,
ElementHandle, request-event, waiter-precedence and worker gaps. The other
5,828 candidate Python rows and all 48,477 Core JS/TS invocations remain open.

## ABI 55 Page evaluate API-name checkpoint (2026-10-03)

NODE-289 pins unchanged async `tests/async/test_asyncio.py` at SHA-256
`544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`.
The selected Chromium rows are
`test_should_not_throw_with_taskgroup[chromium]` and
`test_should_return_proper_api_name_on_error[chromium]`. Earlier evidence
already covers the file's stop-idempotency, pending-request cancellation and
Chromium selector-cancellation rows.

The first independent-oracle attempt is retained as infrastructure failure,
not a product result: with no marker Page, fixture setup reports that Context
management is unsupported by the dedicated-CDP adapter. Its report SHA-256 is
`a5ea2c01f248e1fafa01df574c8cef8627fa02724d0911735b9c4f0d18536dae`.
With the marker restored, oracle passes 2/2, SHA-256
`6328485903ac03a3226871803ca6380c99f6e2369a9c94b6f98c529a28abbb0f`.
The installed candidate baseline passes TaskGroup cancellation and fails only
because `JavascriptError` begins with the browser's `ReferenceError` text
instead of the official `Page.evaluate:` public API name. Its report SHA-256 is
`6cb059ea2647f9388c61921b22ada302dc7b52213c0bca5f359303d5f9c099c0`.

The repair is confined to the Python API boundary. It rebuilds the same native
exception type with one API-name prefix, preserves the stable
`PWRIGHT_JAVASCRIPT` code plus explicit name and browser stack, and does not
catch cancellation. Rust remains the source of classification; ABI 55,
protocol behavior, feature selection, dependency closure and resource limits
are unchanged.

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
Installed Python tests pass 107/107 with five expected skips; Ruff and strict
Pyright pass. Fresh candidate collection remains 6,526/6,526, report SHA-256
`2d4e36a6ae0dc5b78fd6b9250ee743d43fb372ebc85e8c7884734556df7b1d12`.
All 5,853 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 55 selector-wait error-shape checkpoint (2026-10-03)

NODE-288 pins three unchanged source files: async `test_asyncio.py` at SHA-256
`544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`,
async `test_expect_misc.py` at
`61e5d93df621e59a44717532192194bce1a85cc858d5a1f2e69ba724ce0945a6`,
and sync `test_sync.py` at
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
The selected Chromium rows verify cancellation of a protocol-backed selector
wait without an unretrieved future, the public timeout exception name, and an
existing sync Page executing `set_content`, `evaluate` and
`wait_for_selector`. The independent oracle passes all 3/3.

The ABI 55 candidate baseline passes both cancellation and sync rows. Its only
failure is `AttributeError` when the official test reads
`TimeoutError.name`. Rust already classifies the failure as
`PWRIGHT_TIMEOUT`; the repair adds Playwright-shaped `name` access to the
existing Python-visible native exception classes while retaining mutable name
overrides used by `Page.page_errors()`. It changes no protocol, wait owner,
resource limit, native feature or handshake version.

The final installed candidate and independent oracle pass 3/3. Report SHA-256
values are
`95061944d9d6e0f66207653b10171d7ebd9d4881683efa7f8efdac77a7c3c611`
and
`18763889805affdb62fd0dc6951e665ba662fa67068d7f03b153820c1e6798c4`.
Candidate/oracle tranche-ledger SHA-256 values are
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

Installed Python units pass 106/106 with five expected skips; Ruff and strict
Pyright pass. The candidate full collection remains 6,526/6,526. The native
dependency closure and Rust implementation are unchanged from NODE-287. All
5,855 unexecuted candidate Python rows and all 48,477 Core JS/TS invocations
remain open.

## ABI 55 Page wait-for-selector checkpoint (2026-10-03)

NODE-287 pins unchanged `tests/async/test_issues.py` at SHA-256
`5d99e3d911d610cdb116e6b2c9e7d7ac7de8df051cfbe823712297465c375bea`.
The applicable selection contains
`test_connection_task_cancel[chromium]` and
`test_issue_195[chromium]`. The first cancels a still-pending
`Page.wait_for_selector`; the second creates and closes an iPhone 11 Context.
Both independent-oracle rows pass. The ABI 54 candidate baseline passes the
Context row and fails only because `Page.wait_for_selector` is absent.

ABI 55 exposes async/sync `Page.wait_for_selector` as a thin facade over the
existing Rust Locator wait owner. Rust resolves the Frame and waits under one
total deadline, preserves timeout zero as unbounded, applies Playwright's
default non-strict first-match behavior through a native `nth(0)` recipe, and
publishes a real `ElementHandle` for `attached`/`visible` or `None` for
`detached`/`hidden`. Python performs no polling and retains no DOM identity.
The unchanged cancellation case completes and the cancelled sibling task
raises `CancelledError` without hanging fixture teardown.

The whole-file oracle diagnostic is intentionally red: its third Chromium row,
`test_issue_189`, directly calls generic
`BrowserType.launch(ignore_default_args=...)`, while the mandatory dedicated-
Chrome oracle adapter rejects that unsupported launch option. This is a launch-
provider applicability gap and is excluded rather than relabeled as a product
pass or failure. A separate async/sync service-worker-policy diagnostic is also
preserved outside the ledger: both default-allow rows pass, while both block
rows time out waiting for a console event under the mandatory MagicDNS HTTP
fixture. No service-worker claim is made from that environment-sensitive run.

The final installed candidate and independent oracle pass 2/2. Report SHA-256
values are
`18b7130d89320ae47ee72ee45c452d4d6a7b7204885921a1b5b7b624b6851718`
and
`991b5bf6cc6fc47d38cf7f4ce02fbca7964ef3b6c2c59bad9ec71cff2bcacb1f`.
Candidate/oracle tranche-ledger SHA-256 values are
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
units pass 497 with 16 live tests explicitly ignored. Strict Clippy, Ruff and
Pyright, Rust formatting, the official cancellation case and the 108-package
dependency proof pass. All 5,858 unexecuted candidate Python rows and all
48,477 Core JS/TS invocations remain open.

## ABI 54 Page file-chooser checkpoint (2026-10-03)

NODE-286 pins the complete unchanged `tests/sync/test_input.py` source at
SHA-256
`c203939164958aca0babcee63f2d9ae58afce16199fc2b6b5adbc4d6b2284fed`.
It expands to two tests for each browser. Both Chromium rows execute unchanged:
one verifies `Page.expect_file_chooser` and `FileChooser.set_files`, and the
other preserves a file payload's `lastModified`. The four Firefox/WebKit rows
remain explicit provider-gap `not-run` outcomes.

ABI 54 adds the independently selectable `file-chooser-events` composition.
Every selected Page/OOPIF session enables Chromium file-chooser interception.
Rust maps `Page.fileChooserOpened` to the exact public Page and current utility
world on the same event cursor, retains the exact session, target and backend
node in an opaque owner, and publishes through the existing bounded Page
lifecycle fence. `FileChooser.set_files` then reuses the same bounded Rust
payload/path preparation and native mutation implementation as Locator file
input. Python retains only listener, predicate and wrapper identities.

At most 16 live chooser owners are admitted globally. Each owner charges one
live-object slot and 2 KiB of backend state; the installation charges 1 KiB.
Direct mutation obtains a unique utility-world object group and releases it on
success, failure, timeout or cancellation. The first implementation retained a
strong `CdpConnection`: the official assertion passed, but fixture teardown
hung. The final owner retains only a weak connection plus exact route, so the
unchanged official process exits cleanly. `FileChooser.element()` remains an
explicit unsupported surface and is not claimed by this checkpoint.

The final repaired-wheel candidate and independent oracle pass 2/2. Report
SHA-256 values are
`2b2f38ee68d026f3ea1d6eb6f46f8e30edfb8a2a0882921e2cd57622a50d1d38`
and
`0c84f8e75291bdae4bd87685ba9efbc86c12e598b5859a1b7e1f8d7e42e15685`.
Candidate/oracle tranche-ledger SHA-256 values are
`f39fc100ccaf2601cf852b32ca3e8c6543f816e0c34b6ff82eb251673f777b89`
and
`ab9953fae7ddd08720f454d84fb5be75ec0b61b3abf48f6bbd95a2eefc37ebea`.
The cumulative candidate is 659 passed, seven skipped and 5,860 not run,
SHA-256
`557e937c00ccc8e9065c37df2c14fde9b7d71ae0c1b9e011f02171b52d71730a`;
the cumulative oracle is 640 passed, seven skipped and 5,879 not run, SHA-256
`d8bb9714f250d46c17c2afd6e5468eb98f8acf8a7a0f84f0a016bffd8d553534`.

Two normalized-reproducible, repaired ABI 54 wheel SHA-256 values are
`ff2b00f7524235c3dbdfb1f93f3e29b7d9b3da885b550819c8be2b8df0eb6752`
and
`6584f4b92359e0fd1cddcfaf0bb814980b682eec5a2033457077c97c005d385c`.
Installed Python units pass 103/103; selected Rust units pass 497 with 16 live
tests explicitly ignored; strict Clippy, Ruff and Pyright, Rust formatting,
normalized repaired-wheel comparison and the 108-package dependency proof
pass. All 5,860 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 53 Page network-request checkpoint (2026-10-03)

NODE-285 pins the complete unchanged
`tests/async/test_page_network_request.py` source at SHA-256
`6ead05641f44950db30e7ab038c973226ebe726312f39b9d7d7de2aacaf4a388`.
It expands to two tests for each browser. Both Chromium rows execute unchanged:
the popup's first navigation request must reject `Request.frame`, while the
form-urlencoded row verifies decoded `post_data_json`. The four Firefox/WebKit
rows remain explicit provider-gap `not-run` outcomes.

ABI 53 exposes thin Context request-wait and Request frame/post-data/response
facades. Rust owns the fixed waiter queues and the exact shared request-history
owner, so `Request.response()` observes the response published after the
request event. The optional `request-frame` backend observes `Page.frame*` and
`Network.*` on one ordered CDP cursor. It retains at most 64 Frame IDs and one
flag for each of at most 64 active requests per Page, includes that state in
the production resource claim, and passes event-time availability to the host.
This removes the later-topology-snapshot race that could make the popup Frame
appear available only under a larger regression run.

The final installed candidate and independent oracle pass 2/2. Report SHA-256
values are
`c6fd93810787dea30f3beb58ec44202bdff7aad5975f1259852f363faae454cc`
and
`1d0416cb3d1e95cb2eab41e3118b7444fe2e47403a6bd98039d70837b392cba6`.
Three consecutive NODE-281-through-NODE-285 candidate regressions pass 10/10;
their SHA-256 values are
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
Installed Python units pass 103/103; selected Rust unit tests, strict Clippy,
Ruff and Pyright, Rust formatting, normalized wheel comparison and the
108-package dependency proof pass. Fresh candidate collection records
6,526/6,526, SHA-256
`66affc14f9da9eb21d94b750855572592ea92d1f0b039fbf6f18cdaeb01687b7`.
All 5,862 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 52 bounded Response completion checkpoint (2026-10-03)

NODE-284 pins the complete unchanged
`tests/async/test_page_network_response.py` and
`tests/sync/test_page_network_response.py` sources at SHA-256
`707b20a48b1b89529228697d5a87cb42a965a8afb9a485a5c951839c67dc2cba`
and
`82e3f5cfd0581f92085d1b56c06936fb890627dd505189b4894e36d7949e39fd`.
Their full inventory expands to 12 rows. The four Chromium rows execute; the
eight Firefox/WebKit rows remain explicit provider-gap `not-run` outcomes.

The first product-reaching ABI 51 diagnostic completes setup, navigation and
fetch, then all four selected rows fail only at the missing
`Response.finished` member, report SHA-256
`08c1a8411006bac153e900a086961b54c23a9828dddff646548e2e7e77154895`.
An earlier TodoMVC example report, SHA-256
`1270fffc821160baef6e55d2c976a2a3f14a32e24474fc990a484265a5697f73`,
cannot load upstream test fixtures and is retained only as setup diagnostics;
it contributes no product outcome or ledger row.

ABI 52 selects `response-finished` on top of bounded response publication.
Rust owns terminal Pending/Finished/Failed/Closed state, subscriptions and
wakeups. At most 256 owners retain 64 KiB total state; owner slots return on
drop, and Page/session retirement closes pending owners. The terminal feature
does not pull in request history or any Pwright package beyond the existing
response-event closure. Async Python forwards only the native Response owner;
the sync facade submits that coroutine once through the existing bounded loop.

Final installed candidate and independent oracle pass all four Chromium rows
through the same dedicated loopback CDP Chrome. Their report SHA-256 values
are
`98a5f800bdeb271affa5947bd91722e615129835d384ac5cb19e0ae81fe52b00`
and
`807cc17edba597a72a058041ce73de864942c36dbbc2def5179f2c5445fafa06`.
The NODE-281-through-NODE-284 candidate regression passes 8/8, SHA-256
`04ebd36660f93213091f0ac17235007f68fe471c477f3c0c7c96305f4086a65b`.
Candidate/oracle tranche-ledger SHA-256 values are
`28ec4b14143f3c74959798890494949f1e6e3cafd85bc09da1be9375717c121d`
and
`ae2b24671287c846f9be434f9a0261780378578a32cbaae39db8570b10ef9197`.
The cumulative candidate is 655 passed, seven skipped and 5,864 not run,
SHA-256
`82dad9ef03972b03a4174dad73b07409b59776839d2e9885a01e81fb35f6f0c7`;
the cumulative oracle is 636 passed, seven skipped and 5,883 not run, SHA-256
`f05f91ebfaafaa1766eef99bda1e2123a87684ad544ce20c715bc5fd6d76b6e7`.

Normalized-reproducible ABI 52 wheel SHA-256 values are
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

NODE-283 pins the complete unchanged shared `tests/common/test_signals.py`
source at SHA-256
`7f3d9cfc43ca8c5431df25b25ef75f1b38b0f9bac881e893384f622a3dc73267`.
It expands to async and sync rows for each browser. Both Chromium rows install
an application SIGINT handler in an upstream-spawned child, launch with
`handle_sigint=False`, create a Context and Page, and loop through
`Page.wait_for_timeout(100)`. The parent sends SIGINT to the child's process
group and asserts ordered Context, Browser and Playwright cleanup plus exit 0.

The first adapter diagnostic stops before startup on `handle_sigint`; the next
reaches the product and exposes the missing `Page.wait_for_timeout`, report
SHA-256
`9bc32a9b769b80391f54801fd91bc9e901020c4f65dafdcd47e4d5f67e4c4ee2`.
ABI 51 exports the existing independent Rust `wait-for-timeout` leaf. The host
admits 16 concurrent operations and 12 KiB total state, then selects between
session stop, disconnect and a Tokio timer. Python only validates public
milliseconds and forwards the async/sync call.

The runner now exposes only the pinned upstream `tests` package to macOS spawn
children, so the installed oracle package cannot be shadowed by upstream
source. Spawned oracle children install the same CDP-attach launch adapter as
their pytest parent, and both oracle/candidate adapters validate the explicit
`chrome` channel. The pre-fix import and channel reports remain diagnostic and
are excluded from the green ledgers.

Final installed candidate and independent oracle pass 2/2 Chromium rows
through `http://127.0.0.1:9222`, with report SHA-256 values
`15327d9f5ab947fd6c9090f7f33c1be479c55355c6ab317e2cbd9985dae6f91e`
and
`a2b8d0ba5c8f9f3ebc78bcf4293042578af96805a4145305d008a0f7aa01300a`.
Firefox/WebKit remain four provider-gap `not-run` rows. The final candidate
reruns NODE-281 through NODE-283 as 4/4, SHA-256
`48997deecaa3332fcf8ba1a1cdafd7437fde07bac3b69b7cacd61322d063449d`.
Candidate/oracle signal-tranche ledger SHA-256 values are
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
Installed Python units pass 96/96. The targeted Rust capacity test, strict
Clippy/Ruff/Pyright, Rust formatting, normalized wheel comparison and the
108-package dependency proof pass. Fresh candidate collection passes
6,526/6,526, SHA-256
`d2b169846ada9162934a60b9e542e99f55096f43fa5a1a2d988cf2d935e372c8`.
All 5,868 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 sync event-error propagation checkpoint (2026-10-03)

NODE-282 pins the complete unchanged shared `tests/common/test_events.py`
source at SHA-256
`34a3a26326764909a9fd0e1b97282ea5ca689c4a8c45f142725d10c42ee519ce`.
It expands to one row per browser. The initial installed-candidate diagnostic
executes and logs `Exception("fail")` from each response callback but fails
because the exception does not return through `page.goto()`, report SHA-256
`f694bc243739cb29f7e83140749b89f64432a1dec489870f2617793a8199264d`.

The repaired sync loop has one lock-protected pending-listener-error slot.
Wrapped Page handlers retain only the first pending exception. Every sync
public call executes its coroutine before taking and raising that exception;
therefore an otherwise-idle failure can surface after the next cleanup call
without preventing that cleanup. The async event layer continues logging the
handler failure, while Python retains no browser or protocol state.

Final installed candidate and independent oracle pass the Chromium row, with
report SHA-256 values
`b0cb31378655ab7131bda8d57255e28b8378f2ace1275933e61230aa173c4a42`
and
`940665a60c4023e1f2e05a32fa3045339f6052406ddc71df1a7f88b2ffab4fef`.
Firefox/WebKit remain explicit provider-gap `not-run` rows. The final
candidate reruns both shared thread and event files as 2/2, SHA-256
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

NODE-281 pins the complete unchanged shared
`tests/common/test_threads.py` source at SHA-256
`8e42fa88bfa47b5f6ba501c2731d3d9c1812fddb28b4451aba923fb93bfe8c64`.
It expands to one row per browser. Final installed candidate and independent
oracle pass the Chromium row, report SHA-256 values
`1fdf1cf683d7ca5bba9f12db51f81bf2fb083d1d230520a25117f43c39ab254a`
and
`4b10bc1a7187c08b13e25a639cbd33baa45306edc04f0c6916913ddc3c48de59`.
Firefox/WebKit remain explicit provider-gap `not-run` outcomes.

The unchanged upstream body constructs and stops `sync_playwright`, attaches
Chromium, creates a convenience Page and closes the Browser from its own
worker thread. The existing bounded sync loop and Rust-owned
Browser/Context/Page cleanup pass without product changes. Candidate/oracle
tranche ledger SHA-256 values are
`ee364ba1464514d18d20d6160a14062b048c9c967bee4867760a86152e5aef09`
and
`c61fc526cfc53f6927d9322c0aa4c22944464186ae4516952da809674ff22a42`.
The cumulative candidate is 648 passed, seven skipped and 5,871 not run,
SHA-256
`4febdfe050081de20ef509f153f4e77669eb2672d22be1a07853d9e2e1916345`;
the cumulative oracle is 629 passed, seven skipped and 5,890 not run, SHA-256
`d9fb4a9faf432e4cc60c473b373ac9076afb98eb0dd4d9ee3ed62beb0d14b06e`.

No product wheel changed. The normalized-reproducible ABI 50 wheel hashes
remain
`0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
and
`db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
Fresh 6,526/6,526 candidate collection has SHA-256
`c4871850fb0b392fbabcac9feef6d144d50dc68e0233597d2207ca22e0da04dd`.
All 5,871 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 test-id strict-error checkpoint (2026-10-03)

NODE-280 pins the complete unchanged async
`tests/async/test_selector_generator.py` source at SHA-256
`60ae2e5324fd30a820402d4feaa28dac2243caaef227a28ad9b1773e3d403b8d`.
It expands to one row per browser. The final installed candidate and
independent oracle pass the Chromium row, report SHA-256 values
`c38cfd4ee833a79dd43ae44e6574ea15d3cf10c3321846c088bdcc570f597537`
and
`881bcdbf424fab522b7eccd9ba0a4c9555b419b209efe29e025a659d6c1b6c2f`.
Firefox/WebKit remain explicit provider-gap `not-run` outcomes.

The bounded Rust selector registry owns the mutable test-id attribute. New
semantic test-id Locators retain immutable setting snapshots; strict action
diagnostics read the current Playwright-scoped setting. Rust recognizes only
the structured strict-selector/CDP-CSS prefixes and publishes at most five DOM
previews, 512 HTML UTF-16 units and 256 test-id UTF-16 units per preview. Python's
async/sync setter only validates/forwards the public string. Candidate v1 and
v2 diagnostics preserve the missing setter and missing strict-CSS prefix
normalization failures respectively.

Candidate/oracle tranche ledger SHA-256 values are
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

NODE-279 pins the complete unchanged async
`tests/async/test_selectors_text.py` source at SHA-256
`d8b841099bddcb972bf81846f050cac8880f65fe5c6f73b1c44a26d556d39332`.
It expands to two Chromium, two Firefox and two WebKit rows. Final installed
candidate and independent oracle pass both Chromium rows, report SHA-256
values
`214264e2a9babd5277c448a82d86ba11ace901635120cdd047eb4437a76595c3`
and
`d3204cf9be77ba876630627f7f9a81f88017bde7f54f64ebbe7b2f3f58828efa`.
The four Firefox/WebKit expansions remain explicit provider-gap `not-run`
outcomes because the source has no Chromium-only marker.

Public quoted `text=` selectors and bare quoted selectors compile to a typed
legacy-text variant whose Rust browser runtime matches immediate normalized
text-node chunks. Semantic `get_by_text` remains descendant-text based and
public regex text is separately typed. The allocation-free selector-chain
boundary iterator respects quotes, backticks, escapes and regex-contained
`>>`. Async/sync Page query methods publish at most 64 retained handles through
the existing bounded Locator owner; Page selector evaluation uses Playwright's
non-strict-first behavior while Locator evaluation remains strict. `to_have_id`
is a thin adapter over the existing Rust attribute expectation owner.

The selector schema advances to 1.1 and Locator schemas to 1.24/2.7; peers at
1.23/2.6 reject the new variant. Candidate/oracle tranche ledger SHA-256
values are
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
Clippy, Ruff, Pyright, schema checks, normalized wheel comparison and the
108-package dependency proof pass. Fresh candidate collection records
6,526/6,526, XML SHA-256
`9d2d60c194ec015e2e141a29eac2479063bef342f22dbc80c4ed1afe8228f283`.
All 5,873 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 50 screenshot checkpoint (2026-10-03)

NODE-278 pins the complete unchanged async
`tests/async/test_screenshot.py` source at SHA-256
`1ffd1344ba75e3f481b3d448373791145737757503a14f4e55a84288e1a00840`.
It expands to three Chromium, three Firefox and three WebKit rows. The ABI 49
candidate diagnostic fails all three Chromium rows at the missing
`Page.screenshot` facade, report SHA-256
`f38c3a716fae71f63efcc81352fdf4e3805a14e03a3e42ca228f3f89b44e0b59`.

ABI 50 installs async/sync Page and ElementHandle screenshot facades and
extends the Locator facade with public path and mask options. Rust owns the
shared capture queue, bounded output lease, exact Locator/ElementHandle
resolution, mask installation and cleanup, document-space element clipping,
Chromium `captureBeyondViewport` selection and optional path persistence. One
capture admits at most 64 mask Locators and 256 matching elements per Locator;
Python only validates path/type/mask values and forwards native identities.

Final installed candidate and independent oracle pass all three Chromium
rows, report SHA-256 values
`ad7e0c2b043641fa39a4fcac775945a193c4fef63a507f17a4572150bd270c58`
and
`d3f45897a07e19a2ac3aa2ac7a737bbaf67d1b3088bb12b8d819632142ca11e4`.
These cover Page, Locator and ElementHandle mask captures, path-extension type
inference, and explicit type overriding the extension. Because the upstream
source has no Chromium-only marker, the six Firefox/WebKit rows are provider
gaps and remain explicit `not-run` outcomes.

Candidate/oracle tranche ledger SHA-256 values are
`46b1f1fafba03863f5da38fb691e1afa37296b590990d5c750ee1c9153fd22d3`
and
`06c53a9fa7b23cd239f0bb9a8189bad35a6576cfdc7450b52daaa41c1364a6d3`.
The cumulative candidate is 644 passed, seven skipped and 5,875 not run,
SHA-256
`f898f89d210cfb1c73d37b5817b68688050fae9f273c54c92480f4e0375aaf5c`;
the cumulative oracle is 625 passed, seven skipped and 5,894 not run, SHA-256
`aa53c89869c367c05976854317d9d0e50964b6cc7b0617602eba54af5a4b84b8`.

Two ABI 50 wheel SHA-256 values are
`21017fae60321b12ab56bcc1b000e01e18c8892caa48e085a5622fa84a1da9ce`
and
`6d8369d2bb334fb28f650f244172a269bb2f0f694e14c42c5f4b7f06cbc3dfd7`;
their normalized payloads are reproducible after removing only permitted
CycloneDX identity. Installed Python units pass 88/88; the all-feature host
passes 886 tests with 26 explicit live/diagnostic ignores and the addon passes
2/2. Strict Clippy/Ruff/Pyright and the 108-package dependency proof pass.
Fresh candidate collection records 6,526/6,526, XML SHA-256
`6980ab9cfc863becc420c497f98a2a9aec3622935dfd0b542092dcc32288d0cf`.
All 5,875 remaining candidate Python rows and all 48,477 Core JS/TS
invocations remain open.

## ABI 49 tagged PDF outline checkpoint (2026-09-28)

NODE-277 pins the complete unchanged async `tests/async/test_pdf.py` source at
SHA-256
`3a43a35697d70ad4a9ce6cec014a906a47e35b05471756e6eb42f403ca129188`.
It expands to three Chromium, three Firefox and three WebKit rows. The ABI 48
candidate diagnostic passes capture/save but fails the outline row only at the
explicit unsupported-option boundary, report SHA-256
`59e0bff3910ee10e21c2d61a11f6ddd6bc5fb04a5f88f7e63defba159848b33b`.

ABI 49 installs typed `tagged` and `outline` booleans on the existing
Rust-owned PDF capability. Python strictly validates and forwards the option
values. Rust constructs `generateTaggedPDF` and `generateDocumentOutline` and
keeps Page/session routing, stream reads, close, output admission and path
persistence. Final candidate and independent oracle pass all three applicable
Chromium rows, report SHA-256 values
`248379bc35eb7c2b78e6e3293ec7a7b9ada3a2f4327883765880e03153ea3355`
and
`647a7ca744aa2a3899d3f3756d1b7c5df38a213faf4f4e052b4b99948d3a6077`.
The six Firefox/WebKit rows remain explicit upstream Chromium-only
non-applicable `not-run` outcomes.

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

NODE-276 advances the exact native handshake to ABI 48. The separately
selectable `page-pdf` feature keeps Page/session routing, exact
`Page.printToPDF` dispatch, response decoding and retained output accounting
inside Rust. It reuses the finite per-Page capture queue, admits at most two
concurrent retained documents, and rejects a document larger than 2 MiB before
decoding it into retained storage. The Rust PyO3 layer optionally writes the
bounded bytes to the normalized output path. Async/sync Python facades only
reject uninstalled keyword options and forward Page identity plus path.

The complete unchanged sync source SHA-256 is
`38c95f81587739897da1e364e0f18e6964a7db327775d0deccc7b76c49684520`.
The ABI 47 candidate diagnostic fails both applicable Chromium rows with a
missing `Page.pdf`, report SHA-256
`677f6e364d147cf8c03837544076285a620fdac02cf561eb5eec9fd1962b3476`.
The independent official oracle passes 2/2, report SHA-256
`036bffd3a26facd620b478f49ec81f98a70f01274cd51c1d925d370e1975d908`.
The final installed ABI 48 candidate passes the same 2/2, report SHA-256
`c36fb38329822687850d1900d4b24a4b10361d423011ea8efea3c9adf619ac3e`.
The remaining four Firefox/WebKit inventory rows are explicitly non-applicable
under the unchanged upstream Chromium-only marker and remain `not-run`.

Candidate/oracle tranche ledger SHA-256 values are
`18b846b9dc946359f730e4fa6e1f74faad79747975c9fa72621e113bc26b1e45`
and
`54635dc63be374697eebd7a068ed199ea8e0f5f0e3b00505d3c7f6b1d2c1270a`.
The cumulative candidate is 638 passed, seven skipped and 5,881 not run,
SHA-256
`801181534ae92df917d69d02f0d03708981c14ab8a171fefa7086e12acae3b02`;
the cumulative oracle is 619 passed, seven skipped and 5,900 not run, SHA-256
`a9f7b78946734baef302d23a8cada3da5d3d707865957a41fcdb1e3041828a01`.

Two ABI 48 wheel SHA-256 values are
`48593cfdd4a22249e2eee14186faceb95235670657e52aa808680d6e9a2fe348`
and
`b9acf998197b093ec2597c996193e6b3256fc97dfa23e73f3299de85c59bd2af`;
their normalized payloads are reproducible after removing only permitted
CycloneDX identity. The installed wheel passes 85/85 Python units. The
all-feature host passes 882 tests with 26 explicit live/diagnostic ignores;
the addon passes 2/2. Strict selected/all-feature Clippy, Ruff/Pyright, the
108-package dependency proof and fresh 6,526/6,526 candidate collection pass.
Collection XML SHA-256 is
`3d7e92c62715dbc8dafb3d15cb9af030e0876c2e778e80fde9b13ac97cf95645`.
The prior ABI 48 base64-return candidate report remains diagnostic evidence
only; the final report and ledgers above use Playwright's streamed protocol.
All 5,881 candidate Python rows without outcomes and all 48,477 Core JS/TS
invocations remain open.

## ABI 47 ignore-HTTPS-errors checkpoint (2026-09-28)

NODE-275 pins the complete unchanged async
`tests/async/test_ignore_https_errors.py` source at SHA-256
`4209a74e94509e340b5e22f2e057541473835321bcdd81a49788f48cfdf3fd3c`.
The file expands to two Chromium, two Firefox and two WebKit rows. Only the
Chromium provider exists in this profile, so all six rows stay accounted: the
two Chromium rows execute and the four non-Chromium rows remain explicit
provider-gap `not-run` outcomes.

The ABI 46 diagnostic passes the negative/default-TLS row but rejects the
positive row at unsupported `ignore_https_errors`, report SHA-256
`cb5e9506dbf789968653f5525dd58f74689c26dae80110fa055e9c68efa69bd5`.
The independent official oracle passes both Chromium rows, report SHA-256
`1f3179fc8f7deee06eb07ea775d238c9896bad297dd74f0afa1732732c4bef59`.
After ABI 47, the installed candidate passes the same 2/2 with unchanged
upstream assertions, report SHA-256
`cdd4eb40b5c8c40ede8c84f73056c5faf229837c497bf889d66545612510bea8`.

The optional Rust `ignore-https-errors` leaf owns a preallocated 16-Context
registry. Context creation registers policy before Page publication; every
owned Page/OOPIF session is initialized with
`Security.setIgnoreCertificateErrors`, and successful Context close removes
the policy. Python validates and forwards only the public boolean. It owns no
TLS policy, session replay, protocol I/O or browser-state cache.

The two-row candidate tranche ledger is two passed and 6,524 not run,
SHA-256
`7c01e4a2ba7309e4217c2904543031bc7cca71bdf77d34c7ec2f257ff7786a97`;
the matching oracle tranche has the same counts, SHA-256
`20dd4bfaf1a218151b3aa4b800d0eefc5e869d4d3ef61b92dfdb6b13675ffa7e`.
The cumulative candidate ledger is 636 passed, seven skipped and 5,883 not
run, SHA-256
`690f4b86c509932323cd613f2f40fede63a06e86133209bf98944421ae3d70d8`.
The cumulative oracle ledger is 617 passed, seven skipped and 5,902 not run,
SHA-256
`647b129c3901df6eb3337e7d886e88f8cbc7322653a71b38d7e99ac1c65bd204`.

Two repaired ABI 47 wheel archive SHA-256 values are
`4074ac94f00f9463cf337e67af4b73671cf9d152ea1f6582735c9ad3265d49e7`
and
`5f4a2a2d71a0c6290344bd35b2b5ac2305e7653e21ce466d202e7dc584cdba7c`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. Installed Python units pass 83/83, all-feature host units pass 879
with 26 explicit live/diagnostic ignores, and addon units pass 2/2. Strict
Clippy/Ruff/Pyright, the 108-package dependency proof, Rust formatting and a
fresh 6,526/6,526 candidate collection pass; collection XML SHA-256 is
`27895f0aaceecb67c057553193581e6d931b7c750059c18ba88bf2546b3e8fb7`.

All 5,883 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open. This checkpoint is not a full-suite claim.

## ABI 46 fill and context-manager checkpoint (2026-09-28)

NODE-274 pins the complete unchanged async/sync fill sources at SHA-256
`bb916700cbf5f8675d6291bcc5bfb419561c37a540e90e06f59fb75260489945`
and
`40a8295324eaef73570da321758e68bba56f199614f7899bd6317c6a1c98d2e8`,
and the complete unchanged async/sync context-manager sources at SHA-256
`55921bc26646976cb7af7e7b3cfaa533e3e8ff38753153d34bd3b219470317eb`
and
`b13a84c2227feaf1e91d17d4ccf10404a293efa645e716cab88c80c8cb16d38d`.
The five fill rows pass 5/5 in candidate and oracle through the existing
Rust-owned Locator fill capability. Candidate/oracle report SHA-256 values are
`36d2010856b1863f656c33ee6acd760b7afc04efbcf52b8e1e064e51d5d1a2d0`
and
`68b88b17bf5f1f3e6bbb59d7579a362b5b66a8c3d7c6126efbddce244a40665e`.

The initial context-manager candidate stops 4/4 at missing Page,
BrowserContext and Browser context-manager protocols. The initial oracle passes
2/4 because adapting `launch()` to attach over CDP exposed the already-present
default Context in `browser.contexts`. Both reports remain diagnostic and are
excluded from the canonical ledger. Thin async/sync context-manager methods now
delegate exit to the existing close path. The corrected oracle fixture adapter
hides only Contexts that predate the adapted launch, matching the candidate
fixture model without changing upstream assertions. Final candidate and oracle
each pass all four rows. Their report SHA-256 values are
`6f74927d6e6b67aef4e2387bab2b8f7981a7caf60d787a06a359089fca6910b5`
and
`22d67c00f17238be8514532a357afe1c5567658f0494df71646b58aa993b4679`.

The combined nine-row candidate tranche ledger is nine passed and 6,517 not
run, SHA-256
`8c727149b0e6c9b2b8712845a3a55dc81413a0888a4cd2799193246f8f95e0ab`;
the matching oracle tranche has the same counts, SHA-256
`db9950cb4d39a64ddc86e4adaf9fedf40f25202ef8852172e4811254debb3651`.
The cumulative candidate ledger is 634 passed, seven skipped and 5,885 not
run, SHA-256
`8bb2f5b5437477779c6eb6690da2f247d3f377422f62b1d8bd3ee0cafb5f3908`.
The cumulative oracle ledger is 615 passed, seven skipped and 5,904 not run,
SHA-256
`b02be1e0e0f789d575725098853e060d596514ddc7031fd2c0060883a53d080d`.

The ABI remains 46. Two repaired wheel archive SHA-256 values are
`6f2f3e4668083b081608d8ce92e1cdbe1f6040ea8d225f4bcfff44c9a62bc3ed`
and
`51c25e0d628d2dac7acf07c596c6815637000b86efba9056d6eac504d6b9096e`;
their payloads are reproducible after normalizing only CycloneDX identity
fields. The installed wheel passes 81/81 Python units. Strict Ruff/Pyright, the
108-package dependency proof, Rust formatting and a fresh 6,526/6,526 candidate
collection pass; collection XML SHA-256 is
`889533a3ce28682f6ba5f1f10c23b3b16bad8618c985bb3e51eaf889aedff168`.
NODE-274 changes no Rust source and retains NODE-273's green ABI 46 host/addon
and strict-Clippy gates.

All 5,885 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open. This checkpoint is not a full-suite claim.

## ABI 46 native Locator collection evaluation checkpoint (2026-09-28)

NODE-273 pins the complete unchanged async
`tests/async/test_selectors_get_by.py` source at SHA-256
`edc40e9699d9092abc5a5a9ec0e5849933d46b028dbdda6601d8a5b1f9ca6269`.
Its three Chromium rows exercise escaped semantic strings and regular
expressions, exact and substring role accessible-name matching, and hidden-role
inclusion. The initial ABI 45 diagnostic passes the escaping row but the two
role rows stop at the missing native `Locator.evaluate_all` path; that report
is retained as the discrepancy evidence.

ABI 46 exports `NativeLocator.evaluate_all` over the pre-existing Rust
collection-evaluation owner. The immutable typed selector recipe remains in
Rust from construction through collection resolution and utility-world
execution. Rust owns selector compilation, FFI admission and bounded result
conversion; Python forwards only the public function and serializable
argument. No selector recipe is serialized back into Python.

The final installed candidate and independent oracle each pass 3/3. Candidate
report SHA-256 is
`cc1b66557ae5510345f4dae1163f86a4dce8af8c344d81530586fc0f3fdddf12`;
oracle report SHA-256 is
`c0150e1af71e0e262d2a4edf5c5bfa3463aaef0272a5f6bac4aa00e975b32d02`.
The candidate tranche ledger is three passed and 6,523 not run, SHA-256
`8070a6116e7b900319f4199e010ef4af2a285b050a5090abfa331c1c337a217a`;
the oracle tranche ledger has the same counts, SHA-256
`6bd0f33bb1ba6a6134dc71b0bbc3b237b58ce7b52be9ba850458ab67a0bc3855`.
The cumulative candidate ledger is 625 passed, seven skipped and 5,894 not
run, SHA-256
`39fc2c66a96066e5d0275b146688a48a7dbb1e8a3aeaf37f3971d598cbee4e4b`.
The cumulative oracle ledger is 606 passed, seven skipped and 5,913 not run,
SHA-256
`3715fb13273109c65e780df7b8435752f488c2649e6e7ca9c23f64a7cf3eab42`.

Two repaired ABI 46 wheel SHA-256 values are
`a665627bdb6bdda5ce0e8e2a72a37999b222c10ffa5e8ba44637f0f52da52900`
and
`e7a4c8933d3167ad24fea61e3e6354fa06a0df8cb6ccbb44662cee467712b20d`;
their normalized payloads differ only in permitted CycloneDX identity. The
installed wheel passes 79/79 Python units; the all-feature host gate passes
877 tests with 26 explicit live/diagnostic ignores; addon units pass 2/2.
Strict release Clippy, Ruff format/lint, strict Pyright and the 108-package
dependency proof pass. Fresh candidate collection records 6,526/6,526, XML
SHA-256
`79e6f9ef4217bc219b5a790a783cda09021b2771a14b2c110bb1a45239be788c`.

All 5,894 unexecuted candidate Python rows and all 48,477 Core JS/TS
invocations remain open. This checkpoint is not a full-suite claim.

## ABI 45 semantic Locator checkpoint (2026-09-28)

NODE-272 pins the unchanged upstream
`tests/sync/test_locator_get_by.py` source at SHA-256
`a7c0133231235e63ba397175265cdef5ccd221d96b40181e0d9ff2d6a0325f19`.
The pinned v1.58.0 tree has no `tests/async/test_locator_get_by.py`; that
absence is recorded rather than inventing an async counterpart. The complete
sync file expands to ten Chromium rows covering alt text, escaping, label and
nested-label resolution, placeholder, role, test id, text and title across
Page, Frame and Locator entry points.

ABI 45 exposes the existing typed Rust semantic-selector leaves consistently
through Page, Frame, Locator and FrameLocator. FrameLocator semantic and role
composition remains a native recipe. Rust also owns attribute-expectation
deadline, retry, missing-element behavior and exact/regular-expression
comparison, including `ignore_case` overriding a regular expression's `i`
flag. Python validates and converts public values, then adapts the bounded
native result into Playwright's assertion shape; it contains no selector
grammar or polling loop.

The independent oracle and final installed candidate each pass 10/10 in
per-case isolated processes. Oracle report SHA-256 is
`62ec0bb876133ab0fc1be15665e5b637215c0120e263de6fdf5ffc9cb23118f5`;
candidate report SHA-256 is
`95362db9ab9b86874567fcee6bda9871f5c23e849005c0e9d03023ffc07abc73`.
The candidate tranche ledger is ten passed and 6,516 not run, SHA-256
`e159fec1a40bf0ab0c157e4595890c6caab343cb0a5a2aee7e19bce8fc776932`;
the oracle tranche ledger has the same counts, SHA-256
`2b68866af60f6f3fb9bb2f6e880f37214655b54dfaa1d28304031baa4bd2b20f`.
The cumulative candidate ledger is 622 passed, seven skipped and 5,897 not
run, SHA-256
`e37434aa74c4dea2f6f8001ae27d657e833cf0c197207b0186b5d133b9a40813`.
The cumulative oracle ledger is 603 passed, seven skipped and 5,916 not run,
SHA-256
`d164ca2a7dd060d16ced2277062a21cb60b695aa36df8320a551355e64ffee4d`.

Two repaired ABI 45 wheels have archive SHA-256 values
`e0321e22e0ab05b57608f48d180689009d6113a55525a0b81a7a5776e04b71b0`
and
`0c30faaf48699e0b95426204f8c28e3e868abb5e94b2dccc004050ba61ac098b`;
their payloads differ only in permitted CycloneDX serial/timestamp identity.
The installed wheel passes 78/78 Python units. The all-feature host release
gate passes 877 tests with 26 explicit live/diagnostic ignores; addon units
pass 2/2, strict release Clippy, Ruff format/lint, strict Pyright and the
108-package dependency proof pass. A fresh candidate collection records all
6,526 rows, XML SHA-256
`a7e0fe86ae3f2c8478f0ad0100a66d751d41308ae1944f45d19fc10fdd209586`.

The first batched oracle attempt is retained as invalid infrastructure
evidence because Chrome exited during the run. The canonical isolated rerun
above has all ten outcomes. All 5,897 unexecuted candidate Python rows and all
48,477 Core JS/TS invocations remain open; this checkpoint is not a full-suite
claim.

## ABI 44 complete Locator checkpoint (2026-09-28)

NODE-271 keeps every row from the complete unchanged async and sync Chromium
Locator files selected. The new lazy `HostFrameLocator` owns its iframe Locator
recipe in Rust and resolves every frame hop at operation time through a
temporary ElementHandle that is explicitly disposed. Nested frame depth,
selector storage, retries, deadlines, strictness and existing-Locator re-
rooting therefore remain in bounded Rust owners; the async and sync Python
classes retain only native recipe references and facade metadata.

The final focus/blur discrepancy was not a special browser-event serialization
rule. The official browser binding also serializes the listener's `Event`
argument. Playwright Python's public handler wrapper truncates delivered
arguments to the callback's declared signature, so a zero-argument callback is
invoked with zero arguments. Pwright now applies the same thin Python-language
adapter while Rust continues to own binding serialization, publication
admission and final-result delivery. The current independent oracle rerun of
the async row passes 1/1, and the existing complete oracle reports remain
163/163.

The final installed ABI 44 wheel passes all 86 async rows and all 77 sync rows.
Report SHA-256 values are
`49d736923082374a0d3eab81bea3cbaa539cf364105579856bc513c4cd2dde77`
and
`2cbb448b0651490286df3d7d559211ffac1b012527c42ff9176a3d72f6d07e02`.
The complete Locator ledger is 163 passed and 6,363 not run, SHA-256
`9b99936aa66ed493f322ab6e44adf7e5bad788780593caf6b46cf49d5ae01a2e`.
The cumulative 6,526-row ledger is 612 passed, seven skipped and 5,907 not run,
SHA-256
`89eabed04a3a7804637229ff077cf0898cc1aa903c190611bfb2d1553650aefe`.
The independent Locator oracle ledger is 163 passed and 6,363 not run,
SHA-256
`775a01c7414c5af25f65e9ac775aeaeff265f3d51252709c976d41b9c61278d6`.

Two independent ABI 44 wheel builds have archive SHA-256 values
`bfdbcb48279fbc5377e84dbc2d40b101c186cbb74b25fade4582646db848e100`
and
`ef00f6f69fe0dfed04818d0580e9d2e564c6b75c2e08cb2526f7a7aee1c778af`;
their payloads differ only in permitted CycloneDX serial/timestamp identity.
The installed wheel passes 74/74 Python units. The all-feature host release
gate passes 875 tests with 26 explicit live/diagnostic ignores, page-binding
units pass 8/8, addon units pass 2/2, and strict release Clippy, Ruff format and
lint, strict Pyright and the 108-package dependency proof pass. The first addon
attempt that implicitly searched for absent `libpython3.9` is retained as an
environment diagnostic; the explicit Python 3.12 test lane passes.

All 5,907 unexecuted Python rows and all 48,477 Core JS/TS invocations remain
open. Completing the Locator tranche is not a full-corpus compatibility claim.

## ABI 43 viewport and Locator geometry checkpoint (2026-09-28)

NODE-269 advances to ABI 42 for an independently selectable `page-viewport`
feature. Rust owns a fixed 64-Page / 8 KiB viewport registry, CDP window and
metrics overrides, teardown, and replay through the existing session-
initialization fence. The final two ABI 42 wheel SHA-256 values are
`f17b710f3fe0ff90cc9a24a3fc7b4359218214b54462f1e96e2a4d10794ab840`
and
`ba9b0698c6ce62759ccd804ec1f605e0fc8648436918804115acedadbd2e6d74`;
their normalized payloads are reproducible. Installed Python units pass 63/63.
The four targeted viewport rows do not pass at this checkpoint: viewport
mutation succeeds, then two rows reach missing `Locator.screenshot` and two
reach missing `Locator.bounding_box`. That red report is retained rather than
misclassifying downstream failures as viewport success.

NODE-270 advances to ABI 43. The Python facades forward Locator identity and
validated public options only. Rust's existing bounding-box owner resolves the
selector and returns bounded geometry; the independently selectable
`locator-screenshot` composition owns strict resolution, stability, scrolling,
iframe clip translation, capture and the 2 MiB output ceiling, with at most two
concurrent captures.

The four unchanged targeted async/sync Chromium rows pass 4/4, report SHA-256
`d38a756b260f5434bbd38c41956f1c8148facdd7c4c781f507a166b78075c61d`.
The clean complete async Locator report passes 77/86, SHA-256
`04cc14bf1adfd352d8e3eb5cf1139b9b8e82e239229be596bf7b4d8d69e74720`;
the complete sync report passes 68/77, SHA-256
`437120717f7314a7bdd65df31720c3bcb81f033989f2d749c62118e2dfc66f7e`.
Together they pass 145/163, exactly four gains and no losses from NODE-268.
The complete Locator ledger is 145 passed, 18 failed and 6,363 not run,
SHA-256
`1276b628d8b778e7c947bb06fb5f32f43ff98ee064bce3e958e97ff1d88e6169`.
The cumulative 6,526-row ledger is 594 passed, 18 failed, seven skipped and
5,907 not run, SHA-256
`811c14419d0fee688717168bfb0118d07cfaa82c27d9d8c2e0f14650d01348fe`.

The two normalized-reproducible ABI 43 wheel SHA-256 values are
`14924b545aaa14e9a4096897d59cc7cce45cd1fd8c3b5d07ef2ac3a7ef8b11da`
and
`fcf9958339d7f54ec190d85227f957d057abc0c253c6e5f52cf44cad1f852fa9`.
Installed Python units pass 66/66, selected host tests pass 635 with 24
explicit live/diagnostic ignores, addon units pass 2/2, and strict
Clippy/Ruff/Pyright plus the 108-package dependency proof pass. The remaining
18 Locator failures are 16 FrameLocator/content-frame rows and two focus/blur
callback-completion timing rows. All 5,907 unexecuted Python rows and all
48,477 Core JS/TS invocations remain open; this is not a full-corpus claim.

## ABI 35 typed Locator composition checkpoint (2026-09-28)

NODE-257 pins the unchanged async `test_locators.py` source at SHA-256
`6761dfaee418ceb9dc3cd21937d936db36db20ddeae15cd0c677586c5900ed77`
and sync source at SHA-256
`5f4e47a3225669ef2328ada0c873114b2a9238a9a337b5dcb5328a3d4fc3c1e6`.
The fixed `locator-composition-manifest.json` SHA-256 is
`78e5b01d4582222168c51fea88add9a89987374914de5bac4711e2a7e4d4755f`.
It selects the two async chained-description rows, async first/last and nth,
and the matching sync first/last and nth rows for Chromium.

The ABI 34 candidate fails 4/4 async and 2/2 sync with missing `locator` /
`nth` members; diagnostic report SHA-256 values are
`b2795c2e534a7dc84e5635f4d789476af1e5384b1db89acfe297ef07259185a7`
and
`c2acd8c0b15cec16305441b62ed5c964e6ba6173ec473a770f3d51ba8e0e1b86`.
The final installed ABI 35 candidate passes 4/4 async plus 2/2 sync, with
report SHA-256 values
`2090acd65225bd9dee0bfd8e6cc61dc8adf6cfe943d21dfaae6cac456717834b`
and
`f6e70033b933f2393a73e581ff041fc06e099dc920251f36470688f6e2ace53b`.
The independent official oracle passes the same 4/4 plus 2/2, with report
SHA-256 values
`5ca3b4dea00d65d1372017e17f26eaa1cabe9a7df43ca98c03d3faee46ac5576`
and
`9d709ae09e8795e64ff300ec18ffff7e8216d09c1df46aaee08d6889f4f00327`.

ABI 35 exports an immutable native Locator owner. Root selector admission,
typed `Within` composition, nth derivation and subsequent read/count execution
stay under Rust's existing bounded query/FFI owners. Python stores only the
native facade reference plus optional description metadata; it contains no
selector-chain grammar or polling loop. No Cargo feature, Pwright package or
dependency edge is added, and the executable dependency proof remains 108
base Pwright packages.

Final repaired wheel SHA-256 values are
`d95c176835f2e557d7ff6780e398ae0e8db372a8edb9fa608b113de3c20afa11`
and
`2a33c5cba4b0379e74673f302190b6e3ec829beb7a29031de77999a3c73aa47a`;
their normalized payloads are reproducible. Installed unit tests pass 39/39,
Node adapter tests pass 270/270, strict Rust Clippy plus Ruff/Pyright pass, and
full collection remains 6,526/6,526 with XML SHA-256
`f88de9f75105191dba6db541ba21dd5e69fc3e67610b3da29562454d728e1f7c`.

The candidate ledger is 505 passed, seven skipped and 6,014 `not-run`,
SHA-256
`791513dec404b04ed5521aa031524c418703dc78f2f59adced1016c0efed07a7`.
The oracle ledger is 486 passed, seven skipped and 6,033 `not-run`, SHA-256
`efdd8cd287445c9dadd84942ebd64f4e2bb9f565f06928bda3c4eeaa3423d775`.
Exactly the six selected rows changed from NODE-256 in each lane. Remaining
Locator strictness, assertions, actions, Firefox/WebKit, 6,014 candidate
Python rows and the 48,477-row Core JS/TS inventory remain unfinished.

## ABI 34 Locator description checkpoint (2026-09-28)

NODE-256 pins the unchanged async `test_locators.py` source at SHA-256
`6761dfaee418ceb9dc3cd21937d936db36db20ddeae15cd0c677586c5900ed77`.
The fixed async-only `locator-description-manifest.json` SHA-256 is
`d578cddad1cbb56d29d098b4fb1c19a20bd1d24a739809a25bdacb03d6591374`.
It selects the exact no-description, simple-description and special-character
description Chromium invocations. The pinned sync source contains no
corresponding test functions.

The initial candidate fails all three rows with missing `description` /
`describe` members; diagnostic report SHA-256 is
`6108a1105a88af0f21ef27c6f298485fc86812b4e5d36f9a84dd1d2cdbb1da55`.
The independent oracle passes 3/3, SHA-256
`b0ae068ac5dbce8747cddffd41601a250fb2da163f77f64d43973714a1fd1764`.
After the facade adaptation, the installed candidate passes the same 3/3,
SHA-256
`e6ffb953b1f1877017f4203e170f257a21456e576aaed922bbb9150086e1dbf0`.

Description is immutable Python facade metadata: `describe` returns a new
wrapper over the same Page and selector. It issues no browser command and
retains no native/protocol/resource state. ABI stays 34 and no Cargo feature,
Pwright package or dependency edge changes. Final repaired wheel SHA-256
values are
`5d8b1d17925414c497f5fa81a0fcba7d769e1a997700709e5624015d95c1dd7f`
and `f43a4098f6d5e9bccf25cf403c9a8798e4aa1db8eea63e9f653ae6b308923231`;
their normalized payloads are reproducible. Installed unit tests pass 38/38.
Full collection remains 6,526/6,526 with XML SHA-256
`c394acf10375f0239312607628925b40fba8f3b5ff95a091d4493ac2fae398e2`.

The candidate ledger is 499 passed, seven skipped and 6,020 `not-run`, SHA-256
`c5e89f60ce3e252855f80dd5ca8bcd9444cb089f62b61cb60eee04586d29cc67`.
The oracle ledger is 480 passed, seven skipped and 6,039 `not-run`, SHA-256
`100d1c8ef02411cbfd506042600f9b7c90ea54f53b74478c571fc276a7a151f6`.
Exactly the three intended rows changed from NODE-255 in each lane. The two
chained-description cases remain unselected until Python can compose typed
Rust Locator recipes; the remaining Python and Core JS/TS corpora remain
unfinished.

## Expansion order

1. Execute the remaining 5,439 applicable public-behavior rows, including the
   private-fixture, private-assertion-oracle, private-helper and
   private-annotation categories. Keep the 12 reviewed non-applicable rows in
   both ledgers with their existing outcomes.
2. Execute bounded shards through the candidate and implement missing Rust
   capabilities in discrepancy order.
3. Use per-case pytest-process isolation for exhaustive shards; extend drag to
   complete actionability/force semantics and the current routing
   leaf with regex/predicate matching, `times`, request overrides,
   JSON/path/response fulfillment, fallback/unroute, BrowserContext routing
   and the remaining sync overloads without moving browser behavior into
   Python.
4. Preserve the seven-case lane only as a fast bootstrap regression test.
