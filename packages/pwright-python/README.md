<!-- agent-updated: 2026-10-21T03:34:14Z -->

# shuozeli-pwright

Experimental Playwright-shaped Python API backed by Pwright's Rust host. The
base wheel attaches to an existing Chromium CDP endpoint; it does not load
Node.js, download a browser, or pretend that Firefox/WebKit use CDP.

An isolated all-capability release wheel now includes a default-off
`navigation-response-finished` join and passes the `Response.finished()`
assertion in the unchanged official HTTPS security-details case. The next
assertion fails because `Response.security_details()` is absent. Two complete
source-bound runs of the six Chromium `tests/sync/test_network.py` cases both
remain 1/6; neither is promoted into the strict Python ledger. This interim
join explicitly requires `request-history`, while the independent bounded
navigation-owner design remains open. See the
[Python conformance record](../../docs/python-official-conformance.md).

A second isolated release wheel now selects the independent default-off
`response-security-details` leaf alongside the navigation join. Its rebuilt
SHA-256 is `0c09817458bc9329c3bbc7c00153c635ae913d3a58567746a2666ed24fa2d2cc`;
300/300 installed browserless contracts and the full addon dependency/release
feature matrix pass. The unchanged official
sync/async security-detail cases have not yet run on this wheel because the
dedicated Chrome CDP endpoint is unresponsive. The strict v10 outcome remains
unchanged; see the [conformance record](../../docs/python-official-conformance.md).

Current official Chromium checkpoint: an installed ABI131 wheel adds
default-off `frameattached`/`framenavigated` and passes 290/290 Python
browserless contracts. The official 80-case navigation file first ran 76
pass / two fail / two synthetic errors on that wheel; the independently
verified [v7 RED receipt](../../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v7/promotion-receipt.json)
preserves this result and all off-target rows. A second complete run on the
same wheel ran 78 pass / two fail / zero errors, with the previously errored
cases passing; the intermittent target-close issue remains to diagnose. Its
independently verified immutable
[v9 recheck receipt](../../.work/official-conformance/outcomes/python-navigation-recheck-chromium-strict-paired-v9/promotion-receipt.json)
changes only those 80 candidate rows after v8; current candidate totals are
1,650 pass / 119 fail / three errors / 37 skip / 4,717 not-run. The
next pinned 50-case network manifest ran 35 pass / 15 fail in the candidate
and 47 pass / three fail in the Oracle; 12 failures are candidate-only. Its
independently verified [v8 paired receipt](../../.work/official-conformance/outcomes/python-network50-chromium-strict-paired-v8/promotion-receipt.json)
updates only those 50 rows per lane in that historical predecessor. A
separate trusted-HTTPS Service Worker check passes 2/2 Oracle and 1/2
candidate, with the remaining candidate failure at `Response.body`. No
complete Python corpus or unadapted Playwright parity is claimed. A further
source-only fix matches the official Page extra-header value error and passes
293/293 browserless contracts, but it is not credited to the frozen network
run.

The earlier full Chromium async navigation-file candidate run on a frozen
ABI130-v2 wheel records **77 passed, two failed, one conservative report
error** across all 80 unchanged official tests; the independent Playwright
Oracle records 77 passed and three failed. The independently verified
[strict v6 receipt](../../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v6/promotion-receipt.json)
preserves every off-target candidate row and every Oracle row. ABI129 added
Rust-owned `Page.go_forward()` under the optional `history-navigation`
feature; the strict v4 focused receipt proves its official history-API case.
ABI130 adds a bounded, default-off `navigation-url-match` capability: the
three unchanged URL-filtered official waiters pass 3/3 in the verified
[v5 focused receipt](../../.work/official-conformance/outcomes/python-navigation-url-match-chromium-strict-paired-v5/promotion-receipt.json)
and remain passed in the complete-file v6 run. At v6, the sole candidate-only
unfinished navigation ID needed subframe events; two other cases fail in both
lanes under the Tailscale fixture. A later source-only facade fix makes
Page navigation `Response.frame` available without `request-history`, with
283/283 source-overlay contract tests; it is not in the frozen ABI130 wheel
but is included in the installed ABI131 wheel.
See the [Python conformance ledger](../../docs/python-official-conformance.md).

ABI127 adds a separately default-off
`playwright-server-close-notices` profile for successful remote
`BrowserType.connect`. It projects only ordered Page/Context/Browser close
notices plus terminal disconnection through an additional Rust close-only
projection. The base session still retains validated protocol objects under
its separate 16,384-GUID/4 MiB bounds. One Python watcher uses
GUID/generation-keyed wrappers to deliver server-origin Page and Context
`close` callbacks while the Browser remains connected; the older profile
keeps its whole-session-only close behavior. The independently default-off
ABI126 `playwright-server-frame-title` profile already has an installed-wheel
synthetic WebSocket check for `Page.title()` after main-Frame adoption, with
Rust actor lineage and reply fences. A fresh ABI127 release wheel (SHA-256
`8b524b5b424ff2b5dd3a8f871af3a6dbae961fddfdaf895b6000ecc5ff919cb9`)
passes 84 focused installed-wheel tests plus ten independent reruns of the
Tailscale-bound synthetic WebSocket close test, including close-before-dispose
inventory checks. No real Playwright Server or unchanged official
successful-connect test is claimed; see the
[conformance record](../../docs/upstream-playwright-conformance.md).
A separate close-notices-only ABI127 wheel (SHA-256
`02a13c04b0b1b9bee4e9e5fa9f0dbdb3a98bda8a61f53c05198f5af11885e4a4`)
passes 51 focused tests without selecting `frame-title` or exporting native
`NativeRemotePage.title`; this does not yet mean the full wheel has a minimal
Rust dependency footprint.
The current default-feature ABI127 wheel (SHA-256
`731fd13c02b5cc716533f8f0f94027027d51cf94d860cd2656a18d039b2ea9c7`)
passes 661 browserless Python units with 13 explicit skips for unselected
remote or owned-process profiles; it has no remote opt-in installed.

The native ABI is pinned in `python/pwright/_abi.py`; an earlier default-off `cdp-session`
profile with a Rust-owned auxiliary CDP attachment. Its Python async/sync
facades expose `BrowserContext.new_cdp_session`, `CDPSession.send`, raw event
listeners, and `detach`. A PyO3 delivery lease retains cancellation cleanup
until the Python wrapper is registered. ABI119 source adds retained FIFO event
claims with per-event acknowledgement and rollback, one Browser-owned
generation watch for idle events, and unclaimed child cleanup after Page
retirement. Focused release host and addon suites pass 25/25 and 27/27. The
previous full 106-capability ABI119 wheel imports its own native ABI119 module in fresh
Python 3.9.6 and 3.12.14
environments; both pass 571/571 browserless installed-wheel units and
`uv pip check`. Whole-project Ruff check/format and strict Pyright pass.
The current candidate and oracle environments each collect the pinned full
6,526-case Python corpus, and their four focused CDP/tap manifests select the
same 21 cases. Collection is not browser execution or an official pass.
The 11 unchanged official CDPSession browser cases have not been rerun;
installed-wheel checks do not prove browser parity.
ABI123 adds an independently default-off `playwright-server-initialize` path for
the public async/sync `BrowserType.connect` failure case. Its selected release
wheel passes 290/290 focused browserless tests on each of Python 3.9.6 and
3.12.14. Six pinned official closed-server tests pass 6/6 in both the
candidate and independent Playwright oracle with an explicitly versioned,
browser-free replacement for the upstream server-launch fixture. This proves
only the refused-connection error path, not BrowserServer startup, successful
remote connection, or Firefox/WebKit browser support; see the
[strict evidence scope](../../docs/knowledge/pwright-python-connect-refused-strict.md).
The later ABI124 `playwright-server-connect-session` profile returned a real
remote Browser lease through the Rust HTTP-discovery/WebSocket/session
composition. Async/sync facades expose version, connected state, close, and a
single `disconnected` notice. At that checkpoint the selected wheel passed 41
focused Python tests, including three Tailscale-bound synthetic
HTTP-to-WebSocket close modes; the selected Rust addon passed 41/41 release
tests and strict Clippy.
ABI125 adds empty-option remote Context/Page creation with graph-backed
inventories that reject unknown or recycled GUID generations. Its focused
fresh installed-wheel suite passes 49/49 on both CPython 3.9.6 and 3.12.14,
including four Tailscale-bound synthetic variants and a 160-event
unsupported-event burst; the Rust browser-ops
suite passes 15/15. The binding validates and projects protocol events but
does not queue ordinary remote events until listeners are supported.
The later selected ABI125 wheel added empty-option remote Context close and
`Browser.new_page()` with an implicit Context. Closing that Page closes its
Context while retaining the Browser connection; Page creation failure cleans
up the implicit Context. A fresh installed-wheel subset passes 65/65 on each
of CPython 3.9.6 and 3.12.14 with Tailscale fixtures enabled, including
six compiled-extension WebSocket cases and async/sync implicit-Context checks.
The current whole-project strict
Pyright gate still reports 29
errors in three byte-frozen historical test plugins; the owned-process test
and current binding tests pass targeted strict Pyright with zero errors. The
legacy plugins cannot be changed in place without invalidating their pinned
attestations; the CI gate remains red pending an explicit archival policy.
On whole-session termination it emits Page `close`, Context `close`, then
Browser `disconnected` once. Direct sync callers without a caller dispatcher
receive queued callbacks at the next caller-thread API boundary, not during
an otherwise idle wait; arbitrary non-terminal child-close events were still
unsupported in that ABI125 wheel.
Remote Page.close on an explicitly created Context remains unsupported;
arbitrary externally initiated child-close callbacks were unsupported in that
ABI125 wheel and are addressed by the opt-in ABI127 synthetic wheel described above.
Context.close checks GUID generation before sending but the wire
protocol lacks a generation-aware compare-and-close, so a later server-side
reuse race is not eliminated. These are synthetic-server checks, not
unchanged official successful-connect tests or proof of a real BrowserServer.
The [Python lifecycle diagnostic](../../.work/official-conformance/results/python-connect-lifecycle-synthetic-v1-green-v1/attestation.json)
passes 12/12 unchanged assertion bodies in each oracle/earlier ABI125 wheel
candidate lane after
replacing only the upstream server-launch fixture. It is browser-free,
fixture-adapted evidence and gives no official ledger credit.
The [sync Chromium child-close diagnostic](../../.work/official-conformance/results/python-connect-close-events-synthetic-v1-green-v1/attestation.json)
passes its pinned unchanged assertion 1/1 in each oracle/candidate lane,
also with a replaced server-launch fixture and no ledger credit.
The [navigation-after-close RED](../../.work/official-conformance/results/python-connect-navigation-closed-synthetic-v1-red-v2/attestation.json)
and independently verified [GREEN successor](../../.work/official-conformance/results/python-connect-navigation-closed-synthetic-v2-green-v1/attestation.json)
record candidate 0/3 then 3/3 against oracle 3/3 on unchanged async
assertions. Only the server launcher is adapted; neither grants ledger credit.
ABI121 source adds three independent, default-off CDP WebSocket capabilities:
`cdp-handshake-headers` forwards bounded caller headers to HTTP endpoint
discovery and the WebSocket upgrade, while `cdp-ws-close-reason` retains a
bounded peer close reason for the initial browser-version error, and
`cdp-ws-invalid-frame` fails that attachment promptly on an invalid binary CDP
frame without exposing its contents. The Python
async/sync facade forwards `BrowserType.connect_over_cdp(headers=...)` only
when the header capability is installed. The previous ABI119 wheel does not
include these capabilities. The strict browser-free oracle passes the header
and custom-close tests 2/2; the ABI119 candidate fails both before the
WebSocket handshake, and an ABI120 wheel passes custom-close but times out on
the invalid binary frame in the header case. An ABI121 source change earns
formal conformance credit
only after a fresh installed-wheel run of the unchanged official tests and
paired evidence verification; see
[the conformance record](../../docs/upstream-playwright-conformance.md) for
the latest outcome.
Current source schedules awaitable CDP event handlers after delivery and
reports asynchronous failures without replaying acknowledged events; focused
fake-native tests cover its bounded task cleanup, including in the installed
wheel suite. Browser verification remains pending. A synchronous CDP callback
still cannot safely call another synchronous API method from the event-loop
thread. This gap is beyond the unchanged 11-case Chromium CDPSession files,
whose callbacks only collect event data.
The earlier ABI116 wheel selected
the independent `touchscreen-input` and `element-handle-tap` features, which
expose Rust-owned
Page/Locator/retained-ElementHandle tap and raw Page touchscreen input through
thin async/sync Python methods. The repaired 105-feature wheel at
`.work/official-conformance/builds/python-abi116-touch-final-Zs7Dqb/`
(SHA-256 `2473eae420bc850e5981643636405c84e1fab64b414960cabf474e1725146095`)
imports in fresh Python 3.9.6 and 3.12.14 environments; both pass 535/535
browserless package units and dependency checks. The 44-test minimal-feature
release Rust addon suite also passes. The ten unchanged official Chromium
Python tap cases have not been rerun on this wheel, so the strict ledger still
records 0/10 candidate and 10/10 oracle; this is not a browser-parity claim.
Whole-project strict Pyright and Python-source Ruff check/format now pass;
the previous ABI119 installed-wheel facade suite passes 571/571 on Python 3.9 and 3.12.
Its CDPSession adapter tests are browserless, not official browser
conformance evidence.

ABI 115 previously added async/sync `Locator.click` explicit `force`: it
now forwards explicit `force=True` or `force=False` through a third optional
native argument to Rust's `ClickOptionsV2::with_force`, while omitted `force`
keeps the older one- or two-argument call shape. Selector-backed Locators
forward `force` to `Page.click`. An ABI114 native extension is incompatible
with that facade and must fail at import, before any browser action. The
focused binding and facade tests pass. The repaired ABI115 wheel (SHA-256
`5856bc486b4bc342ceb82a251de79779a20b4b5b2094c0bbdbe028883fda8e54`)
selects all 103 configured capabilities, including `element-handle-click`;
fresh Python 3.9.6 and 3.12.14 installs each pass 528/528 browserless package
units and dependency checks. The first ABI115 build selected only 102
capabilities because `pyproject.toml` omitted that click feature; it was
rejected and the configured list corrected before this wheel. Official
browser parity remains pending.

ABI 114 added async/sync
`ElementHandle.select_text(force=..., timeout=...)` under the independent
`element-handle-select-text` profile. Rust evaluates the original retained
Handle, waits for visibility unless forced, follows Playwright's label/control
retargeting, and selects input, textarea, or ordinary element contents. One
whole-action limit of four is shared with retained scrolling and leaves room
in the existing eight Handle-call slots; no selector re-query or persistent
page observer is introduced. The
Python facade uses the Page/Context effective timeout, and zero disables the
host deadline. The minimal addon feature selects `handles` but not the click,
scroll, or locator DOM action leaves. Ten fake-session async/sync tests cover
the bridge. The stable repaired ABI114 wheel (SHA-256
`4850418950bac4dfe1fef27d34042d3b030345d2f0a32c030bcfb40f4c41b592`)
loads from fresh Python 3.9.6 and 3.12.14 installs; each passes **518/518**
package units and dependency checks. Official browser parity remains
unverified.
The earlier repaired ABI114 wheel after the shared Host wait-signature change
(SHA-256 `d66b803fbd03480657ba24b60b5c6953fa45d9ffa3bb516768defea3bcd4e5a0`)
exports the same ABI114/103-capability selection; fresh Python 3.9.6 and
3.12.14 installs each pass **518/518** units and dependency checks. It has no
official browser pass credit.

ABI 113 added async/sync
`ElementHandle.scroll_into_view_if_needed()` under a separate opt-in
`element-handle-scroll-into-view` addon feature. This selects only the base
Handle binding and the same-named Rust host feature; the host acts on the
original retained node without reselecting. The Python facade forwards the
Page/Context effective timeout (30,000 ms without a Page); explicit
`timeout=0` disables the host deadline, while a positive timeout bounds the
whole observe/retry/scroll action. Detachment is terminal. The profile adds
no Pwright package beyond the existing `handles` dependency closure. Nine
focused fake-session facade tests cover the bridge. Its installed release
wheel passes 508/508 browserless unit tests, but independent official
Chromium browser cases remain pending.

ABI 112 added async/sync
`ElementHandle.owner_frame()` through the already selected opt-in
`element-handle-dom` profile. PyO3 forwards the retained Handle to Rust's
existing current-DOM owner query, then Python resolves its canonical Frame ID
against the current BrowserContext Page/Frame collection. This also handles a
node adopted into another Page of the same Context and preserves the sync
Page/Frame wrapper identity. No new Cargo feature, package dependency, or
full-wheel feature selection is needed: `element-handle-dom` already depends
on `handles` and is explicitly selected in `pyproject.toml`; the package
closure is checked by `scripts/check-python-addon-dependencies.sh`. The current
ABI119 full wheel includes this source, but the pinned seven async/six sync
official owner-Frame cases still need browser verification on that wheel.

ABI 111 added four
`ElementHandle` reads (`get_attribute`, `inner_html`, `input_value`, and
`is_checked`) through the opt-in `element-handle-dom-reads` profile. It depends
only on `element-handle-dom`, uses one bounded PyO3 method, and delegates to
Rust's exact retained-Handle DOM read path. The current ABI119 full wheel
includes these methods and passes its installed-unit suite; independent
official browser runs remain necessary. `input_value` accepts and validates
its optional timeout,
but does not yet enforce a per-read deadline. Its current official test also
stops earlier at missing `ElementHandle.fill`.

ABI 110 added
`ElementHandle.bounding_box()` through the opt-in `element-handle-dom` profile.
The PyO3 method forwards the exact retained Handle to the existing Rust host
geometry path and returns its `{x, y, width, height}` result or `None`; neither
Python facade reconstructs a selector or runs browser code. An ABI110 wheel
has been frozen and installed-unit verified, but this method still needs an
official browser rerun.

ABI 109 added
`JSHandle.evaluate_handle()` (inherited by `ElementHandle`) through the
existing opt-in `handles` capability. Python preserves the target and nested
argument owners; the Rust host creates and retains the result Handle. Four
focused facade tests and a release compile check pass. The current ABI119
full wheel includes this source, but it has not yet passed the corresponding
official browser tests. Worker-owned Handles remain outside this Frame-owned
evaluation path.

The opt-in `element-handle-dom` source now captures a Rust-computed, bounded
DOM-node preview before returning Handles from `Locator.element_handle` and
`JSHandle.evaluate_handle`; the base `handles` profile retains its prior
description behavior. Python only caches the preview for synchronous
`str(handle)`. This wiring is newer than the frozen ABI109 wheel, is included
in the current ABI119 full wheel, and still requires an official browser
rerun; focused facade tests pass 8/8.

The latest browser-tested full-feature wheel exports ABI 108 and includes
`Locator.click(steps)`, additive `Page.click` options, and the `Page.dblclick`
facade. It passes seven of eight exact pinned official Chromium click cases
against eight of eight in the independent oracle; `ElementHandle.click` is
still missing. The Context init-script callback remains a candidate failure
after reload. Opt-in fresh Chromium
`BrowserType.launch()` and explicit-profile Chromium
`launch_persistent_context()` are implemented. Firefox/WebKit positive launch
remains unsupported. The latest strict 6,526-row candidate ledger has 1,313
passes, six failures, 11 skips and 5,196 not run; the independent oracle has
1,097 passes, five failures, 11 skips and 5,413 not run. These are not paired
full-corpus results. A complete ABI108 click-file candidate diagnostic is
53 passed / 14 failed / three report errors and is not promoted. See the
[full Python checkpoint](../../docs/python-official-conformance.md).

The ABI106 notes below are historical checkpoints.

ABI 106 exposes opt-in Rust-backed `BrowserType.executable_path` metadata for
Chromium, Firefox, and WebKit using the pinned Playwright v1.58.0 registry. It
returns the expected path even when the binary is absent; it does not install
or launch any browser. `PLAYWRIGHT_BROWSERS_PATH=0` is a known compatibility
limit: its `.local-browsers` root is this Pwright Python package rather than
the official Playwright Node driver package, so the resulting absolute path
is not identical in that mode. The default shared cache root and explicit
absolute or relative browser-cache roots do not have this package-root
difference.

The isolated ABI106 wheel passes 297/297 Python package units. In the pinned
official 12-row metadata diagnostic, candidate and oracle agree on all three
returned engine paths, but both fail the six unchanged executable-existence
assertions because the matching browser binaries are not installed; those
rows have not been promoted into the strict conformance ledger. See the
[full Python checkpoint](../../docs/python-official-conformance.md#cross-engine-browsertype-metadata-checkpoint-2026-10-10).

`BrowserType.launch` is not installed. The opt-in Chromium
`launch_persistent_context` requires an explicit caller-owned profile;
an empty string is now rejected before native work because silently
resolving it to the current directory could select the wrong profile.
Upstream treats that empty input as a temporary profile, so this guard is a
safety divergence pending a separate owned-temporary-profile feature.
Current-source Python units pass 299/299 against the installed ABI106
native module. The existing official `launch` fixture adapters attach to CDP
in both candidate and oracle lanes and do not prove owned-launch parity.

Async/sync `Page.dispatch_event()`, `Locator.dispatch_event()` and
`ElementHandle.dispatch_event()` are implemented in source over the Rust-owned
dispatch paths. Page dispatch chooses the first match unless `strict=True`;
nested live JSHandle/DataTransfer initializers use an owned-argument path,
while plain values keep the atomic Locator evaluation path. The pinned
Playwright Python v1.58.0 async file supplies 12 exact Chromium cases in
`packages/pwright-python/tests/official/page-dispatch-event-manifest.json`
(SHA-256 `acbe71f2b9922e5040b17c2bcbcb3946b9096964f314de0f63ddc909353d0128`).
There is no upstream sync `test_dispatch_event.py` at this tag. Stable-source
ABI105 `abi3-py39` wheels v2/v3 hash to
`229a2fd6c2bc024c3b06943d53538217d0c70168e0e01675b52f25302fd13165` /
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`.
Normalized payloads reproduce except CycloneDX `serialNumber`/timestamp and
the resulting SBOM hash in `RECORD`. A fresh v3 install passes ABI105, proves
both owned native dispatch methods, and passes 287/287 units; Ruff, strict
Pyright and release addon check pass. The pinned official async oracle and
ABI105-wheel candidate each pass all 12 selected Chromium cases. Oracle
XML/provenance SHA-256 is
`45356b260b29ee850e5d32b38d0db94c07aea8332ddb27bc99c94490a33bcb1f` /
`3e8cfaa4e381ea3b8d5febeab9b5d65852ad123ea2a839af1c85c9ca25c469fd`;
candidate is
`71c1be307ec418904fadb30bb93edff99200ea86cb144936aaed5a35da233b36` /
`b742d93ac50ab46c1c93a0190719f6563c737aff2cfeeadff7ea86efbe8ac83f`.
After the async dispatch merge, the strict 6,526-row candidate ledger was
1,238 passed / eight skipped / four failed / 5,276 not run (SHA-256
`c5a31bf3be6d848b804a910a1de6409b09ded8fc8c361db8b1a12ad60c9039de`);
the oracle ledger was 792 passed / seven skipped / four failed /
5,723 not run (SHA-256
`bcc3d23b593cc99fc1d753a6350e89f16b06dea3047afd495ee55fedffafcdef`).
This is selected-file outcome parity, not full Python corpus parity. The
separate Core fixture adapter currently collapses its cross-process URL to
the same HTTP origin, so its passing cross-origin-titled case does not prove
distinct-origin behavior; no secure Core fixture lane has run.

A separate formal browserless same-thread asyncio-loop mode selects the six
expanded `tests/sync/test_sync.py` lifecycle rows with manifest SHA-256
`dbad4ede06a7e3ae07d994a75e02a8868bc82191145e6c3746a880eb2fa2081f`
and wrapper SHA-256
`7a089d2a16cbeec46926374571e96cf72cd8d3a513c76b0281c988be470e63c7`.
Candidate ABI105-v3 wheel SHA-256 is
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`.
Candidate and independent oracle each collect and pass 6/6. Verified
candidate JUnit/provenance SHA-256 is
`6b08f1df918824cc60e95d9231ba4c6c84f2b0eb5bd81639e9890e88d4c85880` /
`085841dbbf0993c31a4b4c9942770e243115d03ecbe975b6c3b6cc46503c2e1d`;
oracle is
`c1f3a1c464d78c225cca5cd10568017d1b6c0a27d9d10cfc7dd0810fbfddb2cd` /
`0c191f41c487998880d90608060c5fd3c8fc68f5e84612dab45e6cd956a01326`.
The exact six-key strict merge advances the full candidate ledger to 1,240
passed / eight skipped / four failed / 5,274 not run (SHA-256
`e7cd534dbd40309ed7d258431face26a1b40d0f2c14c27834b493ec3fff5695e`)
and oracle to 796 passed / seven skipped / four failed / 5,719 not run
(SHA-256 `07cf927aa0a241b2075d9ec0f26c88e58daff0bf50e3d6cd646588ba056fc1d8`).
The older ordinary-pytest paired 3-pass/3-fail reports remain preserved as
historical artifacts; the new ledger promotion is for the formal loop mode
only. No browser/CDP was used for this selection, and full Python/Core
corpus parity remains open.
Fresh official Python and Core runs now reject occupied result/sidecar paths
before process launch (Python's explicit isolated-resume mode is separate).
Historical provenance sidecars do not bind runner/helper source hashes; the
new guard does not retroactively establish a fresh historical execution. The
separate browserless Core BrowserType two-row slice now leaves 48,477-row
strict ledgers at candidate 1,580 expected / two skipped / eight unexpected /
46,887 not run (SHA-256
`556eafa00527dbf41cd7d494dcb2f2a9423272581af614ab6ea1a7f09f93faa7`)
and oracle 1,284 expected / one skipped / six unexpected / 47,186 not run
(SHA-256 `1b08b44e3351da8442652333694aef39a0ac7ab07ff866dea7d3e5b5f10fdb3c`).
`browserType.name` is expected in both; invalid-channel `launchServer` is
unexpected only in the candidate. The secure Core fixture manifest SHA-256 is
`efc2f9f131c245baf7ddca262cd08ca3b5cee3d9f8d3d7a4248cbe55a5a03de2`;
it now pins the exact 17 unique spec IDs but has no official browser result
while the shared dedicated Chrome has a foreign tab.
The next browserless Python tranche pins the exact async/sync
`test_launcher.py::test_browser_type_name_should_work` pair (manifest SHA-256
`e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`).
Candidate ABI105-v3 and independent oracle both pass 2/2; verified
candidate XML/provenance SHA-256 is
`de0479b59b4650342c82006b13406470241390cc25ebc34cab6665e0d1d142f2` /
`0703d2ff4b7ec441f1eec22553a10ac1de1dd5e43cd8fd8edf0a92bd8159a9e2`,
oracle `9ed208f5c431d5b689e11eecdf7516695e961db965d1ae1545920ee4a6047611` /
`3bb3b3a9cfd6cd02f4c91b7f6a754e6ba94682c98777511dbdfde79df37df21d`.
The source checker admits fixture arguments for this exact source/fixture-
hash-pinned two-case closure only; all others remain denied. Only Tailscale-
bound HTTP/HTTPS fixture servers ran, no Chrome/CDP. Exactly two keys advance
the 6,526-row ledgers: candidate 1,242 passed / eight skipped / four failed /
5,272 not run (SHA-256
`ed31f1074ab675e75b8828e27a141a858144e8f85e408b9955d36fd1f2c6e4e8`),
oracle 798 passed / seven skipped / four failed / 5,717 not run (SHA-256
`2c02dffe0d102fe0d2dd660b30318097e455018c074140f73db5707067511852`).

Async and sync `Page.check()` and `Page.uncheck()` now delegate to the
Rust-backed Locator checked-state action. The pinned unchanged Playwright
Python v1.58.0 Chromium async/sync `test_check.py` selection has 18 cases:
candidate v1 failed all 18 at missing Page methods, while the oracle passed
18/18. The repaired `abi3-py39` wheel SHA-256 is
`85913b81bcc9d9c65533554b4b6f7f9ca423216ea08f43d682bc1244b2d5d46b`;
it passes 271/271 package units, Ruff and strict Pyright. Its official
candidate v2 rerun passes 18/18 with verified report SHA-256
`2bc0177b7a6cc8f1ca7c1d4a6e20acfd728dbf819224ee88e69a7f07ad633e14`
and provenance SHA-256
`8c374ce922b748f613d82980ba3aa6be3eb68cdcde76071d19af7e36c8a44573`.
The exact 18-row merge yields 1,226 passed / eight skipped / four failed /
5,288 not run in the 6,526-row candidate ledger, SHA-256
`82c8161800ec3acd18938d0f9d0ba80edf76ce66b0918d563a6923b93ef6ee74`.
This is selected-file parity, not full Python/Core conformance.

At the 2026-10-10T05:01Z source snapshot, independently built final-source
`abi3-py39` wheels A and B hash to
`11d0a7f348145e8c4e2150a78dc3afc5638571cc271e938558f62093090a4adf`
and
`0086c99813ee9f8dad7804c32235f272473d5de3d483fa24f36a2b681a793474`.
Normalized wheel payloads reproduce except generated CycloneDX
`serialNumber`/timestamp and the dependent SBOM hash in `RECORD`. A fresh
installed wheel passes ABI104 and 271/271 package units. The 18/18 official
browser result above belongs to the earlier `85913b81...` wheel, not these
05:01Z builds. Their official browser rerun remains unperformed because a
foreign nonblank tab occupied shared CDP Chrome.

The opt-in `context-storage-state` feature supports async and sync
`BrowserContext.storage_state(path=..., indexed_db=...)` and
`Browser.new_context(storage_state=...)`. The latter accepts a state mapping
or a JSON file path; the former returns a state mapping and can write the same
JSON to a path. Rust owns bounded cookies, localStorage and simple-JSON
IndexedDB capture/restore. Python validates public options and performs file
I/O. The feature bounds state to 8 MiB total, 128 origins, 256 KiB of JSON per
origin and 4,096 localStorage items per origin; it cleans up temporary origin
Pages on cancellation.

Known-ID pending storage-state Contexts are hidden exactly from public
visibility. Before native creation returns an ID, the unidentified-creation
guard can also hide unrelated newly observed Contexts; concurrent visibility
parity for that interval remains open.

The repaired `abi3-py39` wheel SHA-256 is
`8f5e3234cf2d46cc9ff4c2cd9179031e59d75b53bd276c23744317d713493cb1`.
The nine unchanged Playwright Python v1.58.0 Chromium storage-state cases
improved from nine candidate failures at the missing API to seven passes and
two IndexedDB assertion failures. The independent oracle has the same seven
passes and two failures: those assertions hardcode `localhost`, while the
required fixture uses its MagicDNS origin. The clean candidate v3-repeat
report SHA-256 is
`a43d954ff5a14ea64aa3a86de989d350f3aefe4059b7a5ca5eef82d366d2b5ad`
(verified provenance
`1565dbc19591b3b757b96b8c0c2bb947d5bab28ab0cb34786e8a094f157178e6`);
oracle v3 is
`ed29bcc8e528ef77aa621891aa60bb346fd90b7c9d5ab214f9bb1311ae5252b6`
(verified provenance
`efd85286116c94b981f7185c87e5df36d6fe0df0f375f14582f06d92e58f18ab`).
At this storage-state checkpoint, the 6,526-row candidate ledger was 1,208
passed / eight skipped / two failed / 5,308 not run, SHA-256
`d1ecfa2db3e440bae951c09c55e2da862f45f964579b7e69970d59ded510f5ed`;
the oracle ledger was 745 passed / seven skipped / two failed / 5,772 not run,
SHA-256 `3161add7ea5243cefa071dfc2c9845ddf6e18a5dee43bc9dc7c450d7735f6a20`.
An earlier repaired-wheel run timed out after a just-closed task Chrome tab;
its artifact is preserved and the cause is unproven. The same wheel passed a
one-case control and produced the clean full rerun. Full Python/Core corpus
execution and broader IndexedDB/lifecycle parity remain open.

NODE-410 adds async/sync `BrowserContext.expect_page()` as a thin facade over
the existing bounded Context lifecycle waiter. Sync predicates and results
retain the canonical Page identity. The initial installed-wheel candidate
passes 7/9 selected BrowserContext event rows and fails only the two popup
cases at the absent method; the repaired wheel and Playwright 1.58.0 oracle
both pass 9/9 with verified provenance. Exactly nine rows advance the ledger
to 1,201 passed / eight skipped / 5,317 not run. The fresh installed wheel
passes 252/252 units plus Ruff and strict Pyright; two repaired wheel builds
have normalized-identical payloads. ABI104/90 and the native artifact are
unchanged.

NODE-409 changes no code, ABI, feature or wheel. The existing Rust-backed
request-timing path passes all four sync Chromium rows. The Playwright 1.58.0
oracle passes SSL and subresource but fails ordinary navigation and redirect
because Chrome 154 exposes a zero DNS timing rejected by the old assertion.
Only the common two passes are promoted, bringing the ledger to 1,192 passed /
eight skipped / 5,326 not run. The other two remain explicit unmerged
diagnostics.

NODE-407 changes no implementation or artifact. The existing wheel and
Playwright 1.58.0 oracle each pass both remaining sync Chromium service-worker
policy rows under trusted Tailscale HTTPS, completing async and sync files at
2/2 each. Exactly two rows advance the ledger to 1,190 passed / eight skipped
/ 5,328 not run. Earlier ordinary-HTTP timeouts remain environment diagnostics,
not product evidence.

NODE-406 changes no code or ABI. The existing wheel and Playwright 1.58.0
oracle each pass the remaining environment-independent Chromium
`Keyboard.press("+")` case 1/1 with verified provenance. Exactly that row
advances the ledger to 1,188 passed / eight skipped / 5,330 not run. Two paired
popup-dialog and fulfilled-cookie failures are retained only as environment
diagnostics: official Playwright and Pwright agree under Chrome 154 and the
mandatory Tailscale fixture hostname. Neither is promoted or papered over.

NODE-403 keeps ABI104/90 and the native artifact unchanged. On a Playwright
error, the sync bridge now prepends the blocked caller thread's Python stack to
the retained native/async stack; stack capture occurs only on failure. The
fresh installed wheel and Playwright 1.58.0 oracle each pass the same seven
isolated Chromium Page cases 7/7 with verified provenance. Exactly those seven
rows advance the cumulative ledger to 1,187 passed / eight skipped / 5,331 not
run. Two `Page.pause` rows remain unrun because upstream declares a headless
fixture while the required dedicated Chrome is visibly headed; the library
does not fabricate the absent Inspector capability. The wheel passes 250/250
units, two repaired builds have equal normalized payloads, and all dependency,
release, formatting, lint, strict-type and Python 3.9 gates pass.

NODE-402 changes no code or ABI. The two remaining sync request-fulfillment
Chromium cases pass 2/2 from the NODE-401 wheel and 2/2 in the Playwright 1.58.0
oracle, completing both async and sync Chromium files at 3/3. Exactly two
prior not-run rows become passes; the cumulative ledger reaches 1,180 passed /
eight skipped / 5,338 not run.

NODE-401 completes the official 25-row TodoMVC Chromium profile without an ABI
or feature-count change. Rust no longer mistakes an expression containing a
nested arrow for a whole function. Python supplies the stable inactive
`BrowserContext.tracing.stop()` no-op required by `pytest-playwright` teardown;
trace recording and output remain explicitly unsupported. The fresh portable
ABI104/90 wheel and official Playwright 1.58.0 oracle each pass the same
isolated 25/25 cases with verified provenance. Exactly those 25 passes advance
the cumulative ledger to 1,178 passed / eight skipped / 5,340 not run. The
wheel passes 249/249 units, two builds have identical normalized payloads, and
all dependency, release, format, lint, strict type and Python 3.9 gates pass.
Full-corpus conformance remains incomplete.

NODE-400 advances the native contract to ABI 104 with the same 90 selected
features. `to_have_value()` and `to_have_class()` now accept exact or regex
expectations, and ordered class arrays may mix both. Matching, label/control
value resolution, SVG-safe class reads, regex validation and polling remain in
Rust; async/sync Python objects only normalize public matcher shapes. The four
unchanged upstream async/sync class/value Chromium cases pass 4/4 in both the
isolated wheel-backed candidate and Playwright 1.58.0 oracle. The cumulative
ledger is now 1,153 passed / eight skipped / 5,365 not run. The same fresh
installed portable wheel reports ABI104/90 features and passes 246/246 units;
normalized independent
wheel payloads match. Full-corpus conformance remains incomplete.

NODE-399 keeps ABI 103 and expands the installed profile from 89 to 90 explicit
features with `history-navigation`. `Page.go_back()` and the TodoMVC assertion
surface for substring/regex text, checked, empty, exact value and ordered exact
class lists delegate to bounded Rust owners. Regex value/class-list matchers
remain explicitly unsupported. The fresh installed wheel passes 246/246 unit
tests, strict Pyright/Ruff, the Python 3.9 cast audit and single-feature Rust
clippy. The profile-aware isolated runner now expands directory selections
without accepting similar path prefixes. TodoMVC candidate/oracle browser
execution is still pending and no outcome ledger has been changed. The
candidate artifact is an auditwheel-repaired ABI3 wheel with bundled OpenSSL
dylibs and SHA-256
`5dd5bb093a84e6a71f9c6d90a5610a41f10abecf79af503a928bb124521d8979`;
a second repaired build proves normalized payload reproducibility. Candidate
and oracle collect-only each report the exact 25-row profile and have verified
sidecars; they do not execute or promote any outcome.

NODE-396 through NODE-398 correct the official Python collection topology
without changing ABI 103 or runtime behavior. The canonical inventory now
collects 6,501 core rows in the core oracle and 25 TodoMVC `[chromium]` rows
in an isolated Python 3.10 / `pytest-playwright==0.7.2` examples oracle, matching
upstream's separate CI job. A strict rebase preserves all 1,149 passes, eight
skips and 5,369 not-run outcomes.

The profile-aware closure resolves 95 definitions with zero unresolved rows.
Every TodoMVC case receives the plugin's 21-name Page/Context/launch/artifact
closure and is classified as `fixture-creates-page`. The old NODE-394/395
root-only evidence is retained only as the diagnostic that exposed the missing
profile. Collection runs no fixture or test body; full execution remains open.

NODE-392 leaves ABI 103, the 89-feature wheel and public runtime behavior
unchanged. A fixed official manifest can now declare a source-verified
browserless execution policy; upstream's autouse fixture servers remain
mandatory and bind only to the verified Tailscale address. The runner and
independent provenance verifier both fail closed on policy or command drift. Candidate and Playwright
1.58.0 oracle each pass all three parameterized stop-lifecycle rows and share
three asyncio-topology failures. Only the green evidence is promoted, yielding
1,149 passed / eight skipped / 5,369 not run in the candidate ledger.

NODE-387 advances the native contract to ABI 103 while retaining 89 selected
features. `ElementHandle.press()` and `type()` execute exact retained-element
focus plus existing Page Keyboard input inside one native command. Page/Frame
typing, Frame query and selector evaluation are thin facades over existing
Locator/evaluation owners.

The complete async Chromium keyboard tranche improves from an installed ABI
102 baseline of 16 passed / 12 failed / one skipped to 26 passed / two failed /
one skipped, versus the official oracle at 27 passed / one failed / one skipped.
The cross-origin row fails in both lanes; paired 30-process diagnostics
reproduce the other missing-first-keydown failure 2/30 in both lanes. Only 26
passes and the upstream skip are promoted, yielding 1,147 passed / eight
skipped / 5,371 not run with no recorded failure. The wheel passes 239/239
installed units, the 108-package proof, strict gates, normalized-payload
reproducibility and fresh 6,526/6,526 collection. Full Python and Core JS/TS
execution remains incomplete.

NODE-386 advances the native contract to ABI 102 with the opt-in `keyboard`
feature. The fresh 89-feature wheel and official Playwright 1.58.0 oracle each
pass all five unchanged async Chromium focus rows after the preserved
candidate baseline exposed two missing-`Page.keyboard` failures. All five
Keyboard operations delegate to the existing bounded Rust host.

The candidate ledger is now 1,121 passed / seven skipped / 5,398 not run with
no recorded failure, SHA-256
`3cec1a0bd837082094831e4cf5b11e20625e5fb095c1d06ef6b5f643027f0fe0`.
The wheel passes 232/232 units, the 108-package dependency proof, strict Rust
and Python gates and normalized-payload reproducibility. Full Python and Core
JS/TS execution remains incomplete.

NODE-385 advances the native contract to ABI 101 with the opt-in
`request-timing` feature. The fresh 88-feature wheel and official Playwright
1.58.0 oracle each pass all four unchanged async Chromium resource-timing
rows. Rust carries one stable timing sidecar through request/response/terminal
snapshots, handles cached responses and redirects, and sequences bounded
request/response notices. Python only preserves public object identity and
waits on the exact native publication fence after navigation-like operations.

The final candidate ledger is 1,119 passed / two failed / seven skipped /
5,398 not run, SHA-256
`d5d26ca3c234e119808548b48cb6ca96e5802718f48adb619cac73006e2cae95`;
oracle is 738 passed / seven skipped / 5,781 not run, SHA-256
`1ff080b72543b70bd04a7c327e8dcfcc2260aa844953ce9de2ebb4dba8b60ba3`.
The two failures remain the explicit `Page.keyboard` focus discrepancy. The
wheel passes 226/226 units, the 108-package dependency proof, strict Rust and
Python gates and normalized-payload reproducibility. Full Python and Core
JS/TS execution remains incomplete.

NODE-384 verifies the two previously unrun Chromium rows in upstream
`test_request_fulfill.py` without changing ABI 100. The fresh installed wheel
and official Playwright 1.58.0 oracle each pass 2/2 for fetch-original-then-
fulfill and JSON fulfillment. Together with the existing override case, the
upstream file is 3/3 for Chromium. The candidate ledger reaches 1,110 passed /
seven skipped / 5,409 not run; oracle reaches 727 / seven / 5,792. Full Python
and Core JS/TS execution remains incomplete.

NODE-383 advances the native contract to ABI 100 and closes the fixed 15-case
remaining async Chromium JSHandle tranche. The unchanged baseline candidate
passed 6/15; official Playwright 1.58.0 and the fresh installed Pwright wheel
each pass 15/15. Composite Handle arguments retain list/tuple/dict identity at
the thin facade, then Rust validates their bounded value-tree slots and live
native owners before dispatch. No-Handle arguments keep the ordinary Page
evaluation route. Rust-backed `get_properties()` publishes an atomic batch of
independently disposable Handles; Date values and Handle previews match the
official public shapes.

The candidate ledger reaches 1,108 passed / seven skipped / 5,411 not run,
SHA-256
`3708cc9f5c27b9e1a4a40e37c1e12bf8fca016e55a7dfb0ad55462ca8be2611a`;
the oracle is 725 / seven / 5,794, SHA-256
`01028d3a743ce7c31a6a8c517be3b6f8bf1c702a3c5209d86fb9e1b66a12f0f5`.
The 87-feature artifact loads from `site-packages`, passes 221/221 units and
comes from two normalized-payload-reproducible release wheels. Full Python and
Core JS/TS execution remains incomplete.

NODE-382 advances the native contract to ABI 99 with an independently
selectable `file-input-native-paths` leaf. Native directory upload now stays
inside the bounded Rust preparation, target-preflight, DOM mutation and
cleanup owners. Regular paths still use the payload route and keep its 50 MiB
remote-CDP transfer guard; selecting this leaf does not widen colocated-path
trust to arbitrary files.

The fresh installed wheel passes 17/18 unchanged async input rows. Official
Playwright 1.58.0 passes 16/18 under the same connect-over-CDP environment:
both reject the 200 MiB single file, and the oracle additionally rejects the
ten-by-49-MiB chooser transfer that Pwright completes through its native
chooser route. All 16 oracle-executable rows agree and every directory row is
green. The cumulative candidate ledger is 1,093 passed / seven skipped /
5,426 not run, SHA-256
`2e29c50908b763d1cf39defafc540557aa16fd7e1091d0c1ea6f28270ac544f7`;
the oracle is 710 / seven / 5,809, SHA-256
`5dd92c0935ec10748d6990aa04f4866e63e47520fe718870394a59461a2d8ee3`.

The installed artifact reports ABI 99 and 87 selected features, passes
217/217 units, and comes from two normalized-payload-reproducible wheels. The
new leaf adds no Pwright package beyond `file-input`; its exact no-default
profile passes Rust unit tests and strict Clippy. Full Python and Core JS/TS
execution remains incomplete.

NODE-378/379 advances the native contract to ABI 97 with the separately
selectable `file-chooser-native-paths` capability. Rust resolves the chooser's
backend node into its exact utility-world execution context and bounded object
group, validates the returned DOM `RemoteObject`, calls
`DOM.setFileInputFiles` by `objectId`, and retains cancellation-safe cleanup
until the object group settles. Locator collection evaluation now carries a
real user gesture, ElementHandle upload waits for already-published binding
callbacks, and the Python facade keeps chooser interception leased through the
native mutation. The feature adds no package beyond the existing
`file-chooser-events` closure.

The unchanged four-row official async input selection progressed from a red
baseline and an intermediate 2/4 ABI 97 diagnostic to 4/4 in the source repair,
4/4 in the independent Playwright 1.58.0 oracle and 4/4 from a fresh installed
release wheel. The final candidate/oracle JUnit SHA-256 values are
`e99fe0edb6c9b50e5fed8ff4571bcee4d9fe27bf30e3c6b2a2c6f9d012ffcff8`
and `e2b8145706ad78aade6dd83abf851b017d01cb82a14bb3ef2d36d9335032e231`;
both provenance sidecars verify independently. Only the installed-wheel
candidate is canonical. Its ledger is 1,076 passed / seven skipped / 5,443 not
run, SHA-256
`0284d9c1750530e607c3b07daff759278ffae92964751c644f7244d801a80cbf`;
the oracle is 694 / seven / 5,825, SHA-256
`d5fc04bc059fdab72dd20e88ea8157655aa1bf6e62ab4191c7152ec3fe635d69`.
Of the 6,514 applicable rows, 1,073 pass, seven are skipped and 5,434 remain
not run.

Two ABI 97 / 85-feature release wheels have archive SHA-256 values
`4cb27befae61e6c3f99343d38e0f68519b361a2d70bd2e99de2fb7bbae10266a`
and `81b3fdff4a27c9ab3e7e8afb9474c53be0c6145ce8b7132164b2a6d44161c1c8`;
their payloads reproduce after normalizing only CycloneDX identity and time.
The fresh wheel loads from `site-packages`, passes 213/213 units, and collects
the complete pinned 6,526-ID expanded inventory with verified provenance.
Rust formatting and strict Clippy, Ruff, Pyright, the 11-file Python 3.9 audit,
30/30 harness tests, the 108-package dependency proof and the new CI feature
gate all pass.

NODE-377 classifies the full 6,526-row official Python inventory at expanded
invocation scope without changing outcomes. The AST audit identifies 24 rows
with direct private runtime access; an exact eight-base-test policy reviews
every one instead of excluding them mechanically. Twelve are still applicable
public `repr` or selector-registration behavior whose upstream oracle/fixture
is private, while 12 directly test private Error fields, Python helper
functions or raw selector protocol. In total 6,514 rows are applicable: 1,068
currently pass, seven are skipped and 5,439 remain not run. The 12 reviewed
non-applicable rows remain visible at three passed and nine not run. The audit
is reproducible from the clean pinned checkout and cross-checks the inventory,
policy and outcome hashes.

The first applicable private-oracle row now has complete differential proof.
The unchanged async JSHandle evaluate test failed the baseline candidate on
the missing `_impl_obj`, while official Playwright 1.58.0 passed. A read-only
facade adapter plus official JSHandle `repr` fixes the row without moving
Handle identity, preview or lifetime out of Rust. Source and installed release
wheel reruns pass with independently verified provenance. Only the latter is
merged: the candidate ledger is 1,071 passed / seven skipped / 5,448 not run,
SHA-256
`c408e3fcd2abc439830a570302227ea08b4b9729a876b24416928174cb6807e4`;
the oracle is 689 passed / seven skipped / 5,830 not run, SHA-256
`9f8b5dce21bf2b02ccff5fd4adecae2b7a938828e7b4a6b815d0c6ac407b8d88`.
Two release builds reproduce at the payload level after expected CycloneDX
normalization, installed-wheel units pass 208/208, and the Python tooling gate
passes 30/30 AAA tests, Ruff, Python 3.9 source audit and strict Pyright.

NODE-376 adds strict provenance to every future official Python JUnit result.
The sidecar fingerprints the interpreter, pinned manifest and checkout,
official/candidate package trees, ABI3 artifact, harnesses, fixture and browser
selection, exact command and result. Isolated runs additionally bind the
inventory-derived expanded IDs and complete per-case report directory. The
independent verifier reconstructs these inputs and fails closed on drift.
Builder, verifier and runner pass 17 focused tests. Existing reports are not
backfilled. Fresh candidate and oracle full collections each verify all
6,526/6,526 IDs through authentic sidecars (JUnit SHA-256
`69df905bd92005f89fa2e7a59f01bb2b7095816f59791fb1aa57f63b04d0e535`
and `1dbc26cda8dc7b7a1740bd626b94bd1839ec6f20f70dda905a0aa6b3148fb24e`).
Those collection reports prove inventory identity, not execution parity.

NODE-375 keeps ABI 96 and restores the declared Python 3.9 floor. Twenty
facade `typing.cast()` calls used an unquoted PEP 604 union as a runtime
argument; postponed annotations do not defer that expression on Python 3.9.
They now use forward-reference strings, and the official runner removes its
two equivalent casts. The 11-file source audit reports zero runtime union
casts and CPython 3.9.6 imports the package, async facade and sync facade with
the existing ABI3 extension. The official runner now also rejects non-loopback
CDP, non-Tailscale fixture binds/hosts and non-MagicDNS server prefixes before
execution, and refuses a non-collection run while any unrelated nonblank Page
exists in the dedicated Chrome. Twelve focused tests pass; Pyright, wheel and
browser lanes remain pending under current memory pressure.

NODE-356 keeps ABI 96 and completes all 55 Chromium rows in the four official
async/sync Page and BrowserContext request-fallback files. The thin facade now
provides Playwright's lowercase Request header map, `all_headers()` and
case-insensitive `header_value()`, retains Rust's effective fallback snapshot
through later network-object publication with a 64-entry bound, and dispatches
sync route handlers outside the bridge event-loop thread. Source candidate and
independent oracle pass 26/26 async and 29/29 sync. The provisional cumulative
ledger is 1,070 passed, zero failed, seven skipped and 5,449 not run. Release
wheel payloads reproduce after CycloneDX normalization, ABI remains 96 with 83
features, and installed-wheel units pass 207/207. The final release-wheel
browser rerun is still pending because the shared dedicated Chrome CDP endpoint
became unresponsive during another concurrent task; no browser restart was
performed.

NODE-355 advances the native contract to ABI 96. `context-api-request` is the
exact composition of the existing Rust API-request owner with Context cookies,
device/user-agent settings, HTTP credentials and TLS-error policy;
`route-fetch` composes it with Page routing and response-backed fulfillment.
Context/Page `.request`, typed params/data/form/multipart, `Route.fetch()` and
`Route.fulfill(json=...)` remain thin async/sync facades. Request disposal
tracks its owning Context and preserves the close reason; sync route callbacks
can inspect an `APIResponse` without blocking the bridge loop. Final installed
candidate and official Playwright 1.58.0 oracle each pass the 108 selected
expanded Chromium rows (51 async Context, 47 sync Context, 6 async route and 4
sync route). Cumulative execution is 1,015 passed, zero failed, seven skipped
and 5,504 not run. Installed-wheel units pass 203/203, the wheel reports the
exact 83-feature ABI 96 profile, and full collection remains 6,526/6,526.

NODE-354 advances the native contract to ABI 95. The optional
`service-worker-policy` feature depends only on `init-scripts`; Rust owns the
official guarded block script, rollback and exact Context cleanup. Async/sync
Python expose `service_workers="allow" | "block"` as a thin typed facade. The
final installed candidate passes all four pinned Chromium rows with a secure
fixture and isolated async/sync block-policy stress passes 20/20 each.
Official Playwright 1.58.0 cannot create its oracle Context on Chrome 154
because `Browser.setDownloadBehavior` is rejected during fixture setup. No
oracle assertion ran, so all four rows remain `not-run` and cumulative
execution remains 907 passed, zero failed, seven skipped and 5,612 not run.
Full collection remains 6,526/6,526 and installed-wheel units pass 201/201.

NODE-353 keeps ABI 94 and repairs Rust-owned paused-popup initialization,
immediate-close console Handle settlement and unhandled-Dialog teardown. The
complete 12-row async Chromium BrowserContext-event file matches official
Playwright 1.58.0 on Chrome 154 at 11 passed / 1 failed. The shared failing
JavaScript-URL popup row expects `dialog.page is None`, while both libraries
expose the popup Page; it remains explicitly `not-run` as source/oracle drift.
The other 11 rows advance cumulative execution to 907 passed, zero failed,
seven skipped and 5,612 not run. Full collection remains 6,526/6,526.

NODE-352 changes no runtime code. The fixed eight-row async Chromium
BrowserContext cookie-read file gives the same 5 passed / 3 failed result in
the final installed candidate and official oracle. The three shared failures
expect the literal domain `localhost` after navigating on the required
MagicDNS fixture; they remain `not-run`. Both lanes pass the other five rows
5/5, advancing cumulative execution to 896 passed, zero failed, seven skipped
and 5,623 not run.

NODE-351 changes no runtime code. The complete seven-row async Chromium
BrowserContext clear-cookie file passes 7/7 under both the final installed
candidate and independent official Playwright. It verifies whole-context
clearing, isolation and name, regular-expression, domain and path filters.
Cumulative execution reaches 891 passed, zero failed, seven skipped and 5,628
not run.

NODE-350 advances the native contract to ABI 94. Rust-originated
`InvalidArgumentError` now inherits Playwright's public `Error`, so callers can
catch it through the official base class while Python-side signature/type
validation remains `ValueError`. The complete async Chromium BrowserContext
add-cookie file improves from candidate 15 passed / 4 failed to 17 passed / 2
failed, exactly matching the oracle. The two shared failures hardcode
`localhost` cookie path/domain behavior against the required MagicDNS fixture
and remain `not-run`; the other 17 rows advance cumulative execution to 884
passed, zero failed, seven skipped and 5,635 not run.

NODE-349 keeps ABI 93 and removes duplicate Fetch publication when an explicit
`BrowserContext.new_page()` session is followed by browser-root auto-attach for
the same target. Rust topology retains whether an initialized session came
from discovery and detaches only the later duplicate; genuine discovered Page
replacement remains supported. Final installed candidate and official oracle
both pass the eight environment-applicable BrowserContext-route rows. Their
ninth result also agrees, but fails because the upstream test hardcodes a
`localhost` cookie domain while the required fixture is served on MagicDNS, so
that row remains `not-run`. Cumulative execution reaches 867 passed, zero
failed, seven skipped and 5,652 not run.

NODE-348 changes no runtime code and promotes five previously excluded async
Chromium Page-route rows through fixed, isolated candidate and oracle runs.
Both implementations pass 5/5. All 44 rows in that Chromium file now have
non-overlapping passed evidence in the cumulative ledger, which reaches 859
passed, zero failed, seven skipped and 5,660 not run. The earlier interrupted
whole-file diagnostic remains outside the ledger.

NODE-347 keeps ABI 93 and filters recursive Chromium auto-attach by selected
composition. The base topology installs `page`/`iframe` filters with a final
catch-all exclusion; `worker` is added only for a selected worker-retaining
observer. Unselected service workers are therefore never paused in an
unowned Pwright session and remain free to intercept before Page routing. The
unchanged official precedence row passes the final installed candidate 1/1;
cumulative execution reaches 854 passed, zero failed, seven skipped and 5,665
not run. Oracle attempts polluted by a stale Chrome service-worker target and
an incomplete whole-file run after CDP became unresponsive are explicitly not
counted.

NODE-346 advances the native contract to ABI 93 with the separately selectable
`route-redirects` composition, exactly the dependency union of `page-routes`
and `request-history`. Rust recognizes Chromium redirect pauses, continues
them without consuming another user callback or finite-use budget, reapplies
explicit continued headers after removing `Cookie`, and bounds retained header
state to 64 live chains. The existing Rust request-history owner supplies the
exact public Request identity; thin Python links `redirected_from` and
`redirected_to` and keeps the complete Fetch header snapshot. The two
unchanged official redirect rows move from candidate 0/2 to candidate/oracle
2/2, the complete Page-route file improves to 43 passed / 1 failed, and
cumulative execution reaches 853 passed, zero failed, seven skipped and 5,666
not run.

NODE-345 advances the native contract to ABI 92 and adds the separately
selectable `route-fulfill-api-response` adapter over Page routing and the API
request client. Rust inherits a retained `APIResponse`'s status, headers and
content type, copies its body only when it fits the existing 1 MiB route bound,
and applies the fulfillment without moving body bytes through Python. Page
`Response.json()` remains a thin language conversion over the Rust-owned body.
The unchanged official global-fetch row moves from candidate 0/1 to
candidate/oracle 1/1, the complete Page-route file improves to 41 passed / 3
failed, and cumulative execution reaches 851 passed, zero failed, seven
skipped and 5,668 not run.

NODE-344 keeps ABI 91 and aligns Chromium CORS preflight routing. The Rust
route owner recognizes browser-generated `OPTIONS` requests by their bounded
`Access-Control-Request-Method` header, answers them natively with Playwright's
204 CORS response and never publishes them to Python or consumes a user route
budget. Ordinary `OPTIONS` requests remain routable. The two unchanged
official POST/method rows move from candidate 0/2 to candidate/oracle 2/2, the
complete Page-route file improves to 40 passed / 4 failed, and cumulative
execution reaches 850 passed, zero failed, seven skipped and 5,669 not run.

NODE-343 advances the native contract to ABI 91. A routed Request now carries
Chromium's bounded `Fetch.networkId` only as an internal correlation key; the
Rust request-history owner waits for the matching CDP network record by exact
identity and then returns its settled Response. Python retains no parallel
request registry and never guesses by URL. The unchanged official encoded-URL
row passes installed candidate/oracle 1/1, the complete Page-route file
improves to 38 passed / 6 failed, and cumulative execution reaches 848 passed,
zero failed, seven skipped and 5,671 not run.

NODE-342 advances the native contract to ABI 90. Python regex and predicate
filters retain language-native identity over a bounded Rust catch-all route;
nonmatches use Rust-owned fallback and matching `times` retire only after
settlement. The optional `route-fulfill-path` feature reads regular files
asynchronously in Rust with 4 KiB path and 1 MiB body limits and Rust-selected
MIME. The two unchanged official rows pass installed candidate/oracle 2/2;
cumulative execution is 847 passed, zero failed, seven skipped and 5,672 not
run.

NODE-340 advances the native contract to ABI 89. Routed and ordinary Requests
now expose the exact existing Frame identity through bounded Rust descriptors;
`Response.frame` delegates to its Request and `ElementHandle.content_frame()`
uses the existing Handle-to-Frame operation. Cursor-relative popup-first
unavailability remains exact. Candidate/oracle pass request-frame 6/6 and
route-frame 1/1; cumulative official execution is 845 passed, zero failed,
seven skipped and 5,674 not run.

NODE-339 advances the native contract to ABI 87. Page `request`,
`requestfinished` and `requestfailed` listeners now share one bounded Rust
notice stream and one Browser watcher. Failed notices publish Playwright-shaped
failure text, and navigation responses reuse the exact portable Request object
through Rust-retained native correlation rather than URL guessing. The two
unchanged abort rows pass candidate/oracle 2/2 and repeat 10/10; cumulative
official execution is 839 passed, zero failed, seven skipped and 5,680 not run.

The currently implemented slice is intentionally small:

- async and sync `playwright` lifecycle shapes;
- Chromium `connect_over_cdp`;
- separately selectable Chromium `launch_persistent_context` for an explicit
  empty or Pwright-marked user-data directory, returning the browser's default
  Context without downloading or bundling a browser;
- Browser/Context/Page enumeration, `Browser.new_context`,
  `BrowserContext.new_page`, and Rust-owned `Browser.new_page` convenience
  Context cleanup;
- optional `Browser.new_context(ignore_https_errors=True)` and persistent-
  Context launch policy through a bounded Rust-owned 16-Context installation
  applied to every selected Page/OOPIF session;
- optional `Browser.new_context(bypass_csp=True)` and persistent-Context
  launch policy through a bounded Rust-owned 16-Context installation that
  applies Chromium's CSP bypass to every selected Page/OOPIF session before
  public Page use;
- optional `Browser.new_context(java_script_enabled=False)` and persistent-
  Context launch policy through a separate bounded Rust-owned 16-Context
  installation that disables script execution on every selected Page/OOPIF
  session before public Page use;
- optional `Browser.new_context(timezone_id=...)` and persistent-Context
  launch policy through a separate bounded Rust-owned 16-Context installation
  that applies the exact Chromium timezone override to every selected
  Page/OOPIF session before public Page use;
- optional `Browser.new_context(offline=True)` and persistent-Context launch
  policy through a separate bounded Rust-owned 16-Context installation that
  applies Chromium's exact offline network conditions to every selected
  Page/OOPIF session before public Page use;
- optional geolocation plus the separately selectable geolocation permission
  for ordinary and persistent Contexts, with exact Chromium session and
  browser commands owned by two bounded Rust installations;
- optional HTTP credentials plus exact-origin scoping for ordinary and
  persistent Contexts, with selected Chromium Fetch interception, bounded
  authentication state and repeated-challenge cancellation owned by Rust;
- bounded persistent Chromium `args` forwarding with at most 256 switches and
  1 MiB of aggregate UTF-8 input; Pwright-owned profile and remote-debugging
  switches remain rejected;
- optional persistent-Context viewport, screen, scale, mobile, touch and user-
  agent policy through the same bounded Rust-owned device installation used by
  later owned Contexts;
- async/sync Page, BrowserContext and Browser context-manager protocols whose
  exit path delegates to the existing close owner;
- Context and Browser convenience-page `base_url` resolution for navigation;
- complete immutable `playwright.devices` catalog plus bounded Rust-owned
  Context device emulation for viewport, screen, scale, mobile, touch and user
  agent settings;
- Page `goto`, `expect_navigation` without a URL predicate, optional
  `go_back` / `go_forward` through Rust-owned history navigation, `title`,
  `content`, bounded value `evaluate`, optional `set_content` /
  `wait_for_url` / `emulate_media` / `set_viewport_size`,
  Playwright-compatible `viewport_size` snapshots, and explicit close;
- optional navigation `Response.body()` / `text()` through the exact
  Rust-correlated request and authoritative Page/Frame session, bounded to
  eight concurrent 4 MiB bodies with explicit FFI-copy accounting;
- optional Page, Locator and exact retained ElementHandle screenshots through
  bounded Rust capture/output owners, including bounded masks, document-space
  clipping, type inference/override and optional Rust-side path persistence;
- Page `query_selector`, bounded `query_selector_all`, `eval_on_selector` and
  `eval_on_selector_all` over typed Rust-owned selector/collection evaluation,
  including Playwright-compatible non-strict Page selection, typed legacy
  text/regex matching and `internal:and` intersection;
- async and sync `playwright.selectors.register` over a bounded Rust-owned
  global registry, with bounded Rust-owned custom-engine browser execution;
- async and sync `playwright.selectors.set_test_id_attribute` over bounded
  Playwright-scoped Rust state, with immutable semantic-Locator snapshots and
  finite Rust-owned strict-error suggestions;
- optional Rust-owned layout-selector parsing and browser-world geometry
  scoring for the pinned left/right/above/below/near syntax; arbitrary nested
  layout grammar and retry polling remain open;
- basic async/sync Locator scalar and collection reads, fill/check/focus/press,
  event-dispatch and text-selection actions, bounding boxes and screenshots,
  plus value/Handle evaluation as thin facades over existing Rust owners;
- async/sync `expect(locator).to_have_count()` with typed comparison,
  total-deadline retry and bounded sleep owned by Rust;
- async/sync `expect(locator).to_have_css()` through a Rust-selected computed-
  style program with bounded exact/regular-expression values and Rust-owned
  retry/deadline semantics;
- immutable Locator `description` / `describe` facade metadata with no browser
  or native state;
- immutable native Locator recipe owners for typed chained `locator()` and
  `first` / `last` / `nth` derivation without Python selector concatenation;
- typed Rust-owned `get_by_text`, `get_by_label`, `get_by_placeholder`,
  `get_by_alt_text`, `get_by_title`, `get_by_test_id` and `get_by_role`
  recipes across Page, Frame, Locator and FrameLocator, plus Rust-owned
  `to_have_attribute` retry and comparison;
- lazy Rust-owned FrameLocator recipes for Page/Frame/Locator entrypoints,
  nested frames, owner/content-frame conversion, typed Locator re-rooting and
  operation-time frame resolution without retaining DOM identities in Python;
- Rust-owned `evaluate_handle`, direct Handle arguments, `json_value`, element
  classification and `wait_for_function`, plus Page script/style tags composed
  from that capability;
- Rust-owned Page binding registration/publication with thin Python callable
  execution for `expose_function` and `expose_binding`;
- Rust-owned Page lifecycle publication for `load`, `domcontentloaded`,
  `close`, `popup` and `opener`, with Python retaining only public wrapper,
  predicate and callback identity;
- async/sync BrowserContext `on`, `once` and `remove_listener` over the same
  Rust-owned `page` / `close` publication and optional Context console stream;
  the first console callback owns one existing bounded subscription token and
  the last removal releases it;
- independently selectable Rust-owned dedicated-Worker lifecycle, value and
  Handle evaluation, and console publication. Worker Handles retain exact
  Worker-session and execution-context ownership under the shared 64-Handle /
  eight-call bounds. Worker console values use an exact-session bounded FIFO
  and one shared message identity across Worker, Page and Context; console
  argument Handles remain a separate unsupported capability;
- independently selectable Rust-owned download policy, Browser-event
  correlation, bounded download publication and completion/path ownership for
  `Page.expect_download` and `Page` download listeners, including terminal
  Page/Context/Browser cleanup;
- Rust-owned Page dialog publication and single-use `accept` / `dismiss`
  actions for alert, confirm, prompt and beforeunload, including default
  handling when no listener is installed;
- independently selectable Rust-owned file-chooser interception/publication on
  each exact Page/OOPIF session, with `Page.expect_file_chooser` and
  `FileChooser.set_files` reusing the bounded file-input payload/path owner;
  `FileChooser.element()` remains explicitly unsupported;
- bounded Rust observation/publication of Page errors, including Chromium
  name, message, stack and non-Error thrown-value conversion;
- Rust-owned `Page.page_errors()` history, retaining at most the latest 200
  errors per Page within global item/byte budgets and releasing Page history
  on close;
- independently selectable Rust-owned `Page.console_messages()` history,
  retaining the latest 200 console values per Page within global item/byte
  budgets without forcing console callback publication; retained argument
  `JSHandle`s remain explicitly unsupported;
- async/sync Page and BrowserContext `set_extra_http_headers()` through the
  existing bounded Rust header owner, including Page-over-Context native
  precedence, implicit Page navigation referrers and lowercase route request
  header snapshots;
- async/sync Page and BrowserContext request routing for bounded string
  glob/path patterns, with Rust owning Fetch interception, Page-over-Context
  precedence, paused-call capacity, continue/abort/fulfill, bounded URL,
  method, header and POST-data continue overrides, same-protocol URL
  validation, composable fallback traversal and overrides, optional finite
  `times` budgets, handler-specific unroute, relative-pattern base-URL
  resolution, native browser-generated CORS-preflight fulfillment and terminal
  Page/Context-close retirement; the separate `route-redirects` composition
  suppresses repeated user callbacks for redirect hops, retains at most 64
  explicit header overrides in Rust, removes `Cookie` before reapplying them,
  and links the existing request-history identities into public redirect
  chains;
  fulfilled response bodies remain in bounded Rust owners and Python retains
  only handler identity, invocation and public wrappers;
- separately selectable Rust-owned `APIResponse`-to-Route fulfillment over
  `page-routes` plus `api-request`, inheriting status, headers and content type
  while rejecting bodies above the existing 1 MiB route bound before cloning;
  Python passes only native response identity unless the caller explicitly
  supplies a replacement body or path;
- optional async/sync `Page.route_from_har()` for bounded plain embedded HAR
  archives. A backend-neutral Rust leaf owns parsing, exact URL/method/body/
  header selection, redirects and response bytes; the host composes internal
  HAR fulfillment with the existing Page-route Fetch driver;
- optional async/sync `BrowserContext.route_from_har()` over the same bounded
  archive owner. Context registrations apply to existing and future Pages and
  OOPIF sessions, while Page registrations always take precedence;
- optional Rust-owned Page and BrowserContext `add_init_script`, with Python
  limited to string/path normalization;
- Rust-owned Page/Context default timeouts and bounded request/response event
  waits with string, regex, or Python predicate matching;
- independently selectable Rust-owned Page `request`, `requestfinished` and
  `requestfailed` event publication, bounded at 16 live Contexts, 256 queued
  notices and the shared 16 MiB network queue budget, with thin async/sync
  listener identity adaptation and exact native-to-portable request
  correlation;
- independently selectable Rust-owned Page `response` event publication,
  bounded at 16 live Contexts, 256 queued notices and the shared 16 MiB network
  queue budget, with thin async/sync listener identity adaptation;
- exact Page-registry identity for routed and ordinary `Request.frame`,
  `Response.frame` and `ElementHandle.content_frame()`, with bounded Rust-owned
  native descriptors and popup-first cursor-relative unavailability;
- optional headless `Page.pause()` with Rust-owned scoped reset and exact
  restoration of Page/Context operation and navigation timeouts; headed
  Inspector mode remains explicitly unsupported until that capability is
  installed;
- optional `Page.wait_for_timeout()` through a separately selectable Rust
  timer leaf bounded at 16 concurrent waits and 12 KiB total retained state;
- async/sync `Page.wait_for_selector()` through the existing Rust Locator wait
  and Handle publication owners, with one total deadline, native first-match
  derivation by default and no Python polling or DOM retention;
- optional `Page.request_gc()` with Rust-owned Page/session lookup, command
  admission and exact Chromium heap-collection dispatch;
- optional `Page.pdf()` with Rust-owned Page/session lookup, finite capture
  admission, exact Chromium `ReturnAsStream` PDF dispatch, bounded `IO.read`,
  raw/base64 chunk decoding, mandatory `IO.close`, 2 MiB retained-output
  accounting, typed tagged/outline generation and optional Rust-side path
  persistence;
- Rust-owned `Request.sizes()` settlement with bounded pending owners, POST
  body decoding and Chromium raw-request-header correlation;
- Rust-owned Page/Locator drag-and-drop with same-Page/same-Frame validation,
  bounded source/target geometry, finite positions and Mouse step limits;
- Rust-owned Page Keyboard `down`, `up`, `insert_text`, `type` and `press`
  through an optional Python native boundary;
- Rust locator-backed `click`, `fill`, `press`, `set_checked`, `focus`, selector
  evaluation, and common selector reads;
- published Frame enumeration, name/URL lookup, and Frame `evaluate` / `press`;
- browser-independent `playwright.request` contexts and responses, with seven
  verbs, redirects, retries, Basic auth, cookies, storage state, decompression,
  TLS policy, lazy exact-origin PEM/PFX client certificates, disposal, and
  bounded resolver output owned by Rust;
- stable native exception codes and resource metrics.

Everything outside that list is unsupported until its differential and cleanup
gates pass. See [the Python FFI design](../../docs/python-ffi-compatibility-design.md).
The pinned official-source lane is documented in
[Official Playwright Python conformance](../../docs/python-official-conformance.md).

## Local release wheel

```bash
uv run --with maturin==1.15.0 maturin build --release --locked \
  --auditwheel repair --out dist
```

For a conformance wheel with an extra opt-in capability, `maturin --features`
replaces the `[tool.maturin].features` list; it does not append to it. Pass the
entire configured list plus the extra capability explicitly, then verify
`pwright._native.selected_features()` against the intended selection before
running installed-wheel tests. A three-capability wheel produced by passing
only `--features element-handle-click` was rejected for this reason.

For an editable development install from this directory:

```bash
uv run --with maturin==1.15.0 maturin develop --release
```

The current source declares native ABI version `123` and PyO3 `abi3-py39`.
Import fails if the Python package and extension ABI do not match; older
wheels cannot serve the new retained ElementHandle DOM reads, `owner_frame`,
`scroll_into_view_if_needed`, `select_text`, or the new native
`Locator.click(force=...)` argument.

ABI 62 adds independently selectable `worker-console-events` over
`worker-events` plus `console-events`. Rust correlates events with the exact
Worker session, owns a 128-notice/16 MiB FIFO and releases unexported argument
RemoteObjects through bounded cleanup. Python runs one Browser watcher and
publishes the same `ConsoleMessage` identity to Worker, Page and Context;
`ConsoleMessage.worker` returns that exact Worker facade. Argument Handles are
not fabricated from diagnostic values.

NODE-302 keeps ABI 62 and adds independently selectable `worker-error-events`
over `worker-events` plus `page-errors`. The Rust Page-error observer resolves
normal Frame and Page sessions first, then uses the exact retained Worker
session as the owning-Page fallback. It reuses the existing bounded PageError
publication queue and history; Python needs no new watcher, queue or native
method.

NODE-303 keeps ABI 62 and adds independently selectable `worker-network-events`
over `worker-events`, `request-events` and `response-events`. Rust enables the
Network domain on retained Worker sessions before debugger resume, maps their
events to the owning Page through a bounded related-session registry, and keys
native requests by the exact `(session, request-id)` pair. Worker traffic shares
the Page's existing 64-active-request bound and publication lane; detaching a
Worker drains only that session's requests. Python reuses the existing network
watcher and weak Page identity maps so a published `response.request` is the
same public object as the matching request without unbounded retention.

NODE-304 advances to ABI 63 and adds independently selectable
`worker-handle-evaluation` over `worker-events` plus `handles`. Rust creates
and evaluates Handles in the exact retained Worker's execution context,
publishes only opaque owner ids, limits the host to 64 retained Handles and
eight concurrent Handle calls, and drains at most 64 cleanup reservations.
Each public Handle keeps only a weak reference back to its Worker owner so a
disposed Handle cannot retain the Worker session or CDP connection through
browser shutdown. Python only adapts `Worker.evaluate_handle` and
`JSHandle.get_property`; inherited Worker-Handle operations not named by this
slice remain outside the compatibility claim.

NODE-305 advances to ABI 64 and adds independently selectable
`worker-console-argument-handles` over `worker-console-events` plus
`worker-handle-evaluation`. Rust pre-admits and validates the complete ordered
argument batch before publishing any Handle, transfers successful batches into
the existing 64-entry Worker Handle registry and compensates every
untransferred object exactly once. The console FIFO stays bounded at 128
notices/16 MiB. `JSHandle.json_value` dispatches Worker-created Handles through
the exact Worker execution context; retained primitives require no browser
round trip. Python converts only opaque descriptors and accepts both the legacy
11-field and ABI 64 12-field notice shapes.

NODE-306 advances to ABI 65 and adds independently selectable
`context-locale`. Rust retains at most 16 owned-Context locales of 256 UTF-8
bytes each. Dedicated and nested Workers inherit the renderer locale as in
upstream Playwright; no Worker locale command, session registry or separate
feature exists. Python only forwards the locale during Context creation.

NODE-307 advances to ABI 66 and adds independently selectable
`persistent-context-locale` over `persistent-context` plus `context-locale`.
NODE-339 advances to ABI 87 and extends the existing Rust request-event owner
to publish request-start, request-finished and request-failed through one
bounded FIFO. The exact native request ID remains internal correlation data;
Python reuses the portable Request identity for Page events and navigation
responses. The unchanged abort rows pass installed candidate/oracle 2/2 and
repeat 10/10. The complete Page-route file is 34 passed / 10 failed, and the
ledger is 839 passed, zero failed, seven skipped and 5,680 not run, SHA-256
`b8c7bebbeb3f2c16880fb7fc08dd30b15d56d504d091c264b732641ef8e81e0f`.

NODE-338 keeps ABI 86 and makes Rust serialize Playwright's fixed HTTP reason
phrase with every Chromium route fulfillment. The unchanged 422 row moves
from exact candidate failure to installed candidate/oracle 1/1 and repeat
5/5. The complete isolated Page-route file is 32 passed / 12 failed with no
regression. The ledger is 837 passed, zero failed, seven skipped and 5,682 not
run, SHA-256
`8a6d6a9169086e482fbd8acf6d81162ef42ab209f18c2c174238e5feb85e2ef5`.

NODE-337 keeps ABI 86 and changes no runtime code. Two additional unchanged
Page-route rows verify that native `Referer` and `Origin` names use the
lowercase public header mapping. They move from an exact ABI 84 candidate 0/2
baseline to installed candidate/oracle 2/2 and repeat 10/10. The complete
isolated Page-route file is 31 passed / 13 failed; the known unstable data-URL
and equal-request rows remain unpromoted. The ledger is 836 passed, zero
failed, seven skipped and 5,683 not run, SHA-256
`eb6b687f25461fa8d8f4182455215dbcf565a47296427680357866df79cc8553`.

The default Context policy is admitted before the initial Page topology is
initialized. Every selected Page/OOPIF session receives Playwright's Chromium
pair: `Emulation.setUserAgentOverride` with `acceptLanguage`, followed by
`Emulation.setLocaleOverride`. The former controls `navigator.language`; the
latter controls ICU/`Intl`. Only Chromium's exact shared-renderer duplicate-
locale error is idempotent, and every other protocol failure propagates.

NODE-336 advances to ABI 86 and adds async/sync Page and BrowserContext
`set_extra_http_headers()` without moving header state into Python. The
existing bounded Rust installation owns Page-over-Context precedence, native
session fanout, replacement, clearing and retirement. Navigation reads the
Page-specific `referer`; route publication adds the same value only when
Chromium omits it and normalizes CDP request-header names to lowercase.

The unchanged three-row async Chromium manifest moves from an exact ABI 84
candidate baseline of 0/3 to independent oracle and final installed candidate
3/3; five additional candidate runs pass 15/15. Installed units pass 180/180,
the exact composed host profile passes 552 with eight declared live ignores,
the all-feature host passes 1,018 library tests plus 22 integration probes
with 26 plus 10 declared live ignores, and the addon passes 2/2. Full
collection remains 6,526/6,526. The ledger is 834 passed, zero failed, seven
skipped and 5,685 not run, SHA-256
`a0c1caa9d38619302ae8b0eaa49450d1c9db9a5db80cb44e4fa6fb5e69c4b783`.

NODE-335 advances to ABI 85 and adds handler-specific Page/BrowserContext
`unroute()` plus `Route.fallback()` without moving routing state into Python.
Rust traverses Page handlers before Context handlers and newest before oldest,
rematches each older handler against the effective fallback URL, merges URL,
method, headers and POST data field by field, and terminates the chain on
continue, fulfill or abort. Sync wrappers retain the original user callback
identity for precise unroute. Chromium favicon pauses are aborted internally,
matching the pinned upstream server behavior before user interception.

The unchanged eight-row async Chromium manifest has an ABI 84 baseline of
seven failures and one existing pass. The independent oracle and final
installed candidate pass 8/8, and five additional candidate runs pass 40/40.
Installed units pass 176/176; the all-feature host passes 1,037 tests with 36
explicit live ignores across library and integration probes, and the Python
addon passes 2/2. Full collection remains 6,526/6,526. The ledger is 831
passed, zero failed, seven skipped and 5,688 not run.

NODE-334 advances to ABI 84 and adds Page/BrowserContext route `times` without
moving routing state into Python. Rust stores an optional finite remaining
count per registration, matches and decrements it under one state lock, and
atomically removes the last use. After its terminal action, the host refreshes
the affected bounded session set. A request paused by Chromium's stale Fetch
pattern during that refresh is continued directly in Rust and never enters
the Python callback queue. `times=0` retains Playwright's unlimited behavior.

The unchanged three-row async Chromium manifest moves from candidate 0/3 to
independent oracle and final installed candidate 3/3; five additional
candidate runs pass 15/15. The companion direct negated assertion aliases,
including `not_to_have_text`, delegate to the existing `.not_to` owner rather
than adding a second assertion implementation. Installed units pass 173/173,
the all-feature host passes 1,011 with 26 explicit live ignores, full
collection remains 6,526/6,526, and the ledger is 823 passed, zero failed,
seven skipped and 5,696 not run.

NODE-333 keeps ABI 83 and aligns route-glob question marks with Playwright.
Rust treats `?` as a literal rather than a one-character wildcard and compiles
the same meaning into Chromium Fetch patterns by adding parity-aware escaping.
The compiler is fallible and bounded: 4 KiB public patterns become at most
8,193-byte transient CDP patterns, and the fixed 256-pattern vector reserves
explicitly. The unchanged official row moves from candidate failure to final
installed candidate/oracle 1/1; final candidate repeat passes 5/5. The ledger
is 820 passed, zero failed, seven skipped and 5,699 not run.

The final complete 44-case Page-route diagnostic records 22 passed / 22
failed. Its two apparent losses relative to NODE332 reproduce on the unchanged
NODE332 wheel in contemporaneous controls, including equal-request instability
across repeats, so they are recorded as unstable diagnostics rather than
attributed regressions. The remaining Python and complete Core JS/TS corpora
remain open.

NODE-332 advances to ABI 83 and implements the complete public
`Route.continue_()` override value: optional URL, method, headers and POST data
are validated and retained only within bounded Rust ownership, then serialized
into one `Fetch.continueRequest`. URL replacement must preserve the original
request protocol; POST data is base64 at the protocol boundary. Python only
normalizes mappings, strings, bytes and JSON-compatible values.

The unchanged ten-case async Chromium `test_request_continue.py` file moves
from an ABI 82 red baseline of 2 passed / 8 failed to installed candidate and
independent oracle 10/10. The cumulative ledger is 819 passed, zero failed,
seven skipped and 5,700 not run. Re-running the complete 44-case async Page-
route file records 23 passed and 21 failed with no setup errors, up from 18
passed / 25 failed / 1 error. Those 21 discrepancies, the remaining Python
corpus and the complete Core JS/TS corpus remain open.

NODE-331 keeps ABI 82 and executes all 44 Chromium rows in the unchanged async
`test_page_route.py` file, one subprocess per row with a hard timeout. The
complete candidate baseline is 18 passed, 25 failed and one error. All 18
existing supported behaviors then pass a fixed candidate selection and the
independent Playwright 1.58.0 oracle 18/18. They cover continue/abort/fulfill,
main- and cross-process navigation, hashes/data URLs, cancelled interceptions,
cookies, CORS GET, POST-302, encoded URLs, synchronous-XHR redirect, binary/SVG
bodies, response-header stringification and glob matching. The cumulative
ledger is 809 passed, zero failed, seven skipped and 5,710 not run.

The preserved failing rows cluster around continue overrides, regex/unroute/
times composition, requestfailed and redirect-chain models, service-worker/data-URL edge
rules, Page headers, CORS preflight, fulfill path/response, status text and one
resource-profile setup error. Those are implementation inputs, not exclusions.

NODE-330 keeps ABI 82 and promotes six additional unchanged official
`test_unroute_behavior.py` rows. Async/sync Page and BrowserContext default
removal pass 4/4, while the two async in-flight request rows prove that
`behavior="wait"` waits for the handler's later `fulfill()` instead of
implicitly continuing the request. Installed candidate and independent
Playwright 1.58.0 oracle both pass 6/6. The cumulative ledger is 791 passed,
zero failed, seven skipped and 5,728 not run. Handler-specific unroute,
`fallback()`, regex/predicate matchers and `ignoreErrors` remain separate gaps.

NODE-329 advances to ABI 82 and adds async/sync Page and BrowserContext
`unroute_all()`. Rust removes the exact registration scope, reconciles Fetch
patterns across the affected sessions and, for `behavior="wait"`, waits until
both queued and already-dispatched actions from that scope settle. A
cancellation-safe settling guard prevents a dropped FFI future from stranding
the wait. Python clears only its callback identities after native removal;
`ignoreErrors` remains explicitly unsupported.

The two unchanged async HAR-abort rows pass final installed candidate and
independent oracle 2/2; candidate repeat stress passes 20/20. A Chromium
navigation race found by the Page row is handled in the Rust navigation leaf:
a late error-document commit is ignored only when no newer competing loader
has started, while a `frameStartedNavigating` or Document request for another
loader still interrupts. Installed-wheel units pass 171/171, the all-feature
host passes 1,004 tests with 26 explicit live ignores, and collection remains
6,526/6,526. The cumulative ledger is 785 passed, zero failed, seven skipped
and 5,734 not run. Handler-specific unroute, `behavior="ignoreErrors"`,
regex/predicate matchers, `times`, external/ZIP/update HAR, remaining Python
and complete Core JS/TS coverage remain explicit gaps.

NODE-328 advances to ABI 81 and adds async/sync `BrowserContext.route()` over
the existing Rust route owner. Page registrations still win over Context
registrations regardless of order; newest wins within one scope. Context
registrations apply to existing and future Pages/OOPIFs and retire with their
Context. Python retains only the handler keyed by the bounded Rust registration
ID and does not mirror match order, Fetch state, topology or paused requests.

The four unchanged Context HAR URL-filter/ordinary-route composition rows pass
final installed candidate and independent oracle 4/4. API-separated candidate
stress passes async 20/20 and sync 20/20; installed-wheel units pass 169/169,
the all-feature host passes 1,003 tests with 26 explicit live ignores, and full
collection remains 6,526/6,526. The cumulative ledger is 783 passed, zero
failed, seven skipped and 5,736 not run. Regex/predicate matchers, `times`,
unroute variants, external/ZIP/update HAR, remaining Python and complete Core
JS/TS coverage remain explicit gaps.

NODE-327 keeps ABI 80 and aligns the official bad-HAR fallback behavior. A HAR
whose `log` object omits `entries` is a bounded empty archive, so the existing
Rust `not_found="fallback"` policy continues the request. This is not blanket
error suppression: malformed JSON, malformed entries, external content and
all size/count violations still fail registration explicitly.

The unchanged async/sync rows pass installed candidate and independent oracle
2/2; candidate stress passes async 10/10 plus sync 10/10. The HAR policy crate
passes 5/5, installed units pass 168/168 and the all-feature host remains 1,003
passed with 26 explicit live ignores. Full collection is 6,526/6,526 and the
cumulative ledger advances to 779 passed, zero failed, seven skipped and 5,740
not run.

NODE-326 advances to ABI 80 and adds async/sync
`BrowserContext.route_from_har()`. Page and Context registrations are explicit
Rust scopes: Page wins over Context regardless of registration order, newest
wins within one scope, and a Context registration remains live while it has no
Pages so it can initialize future Pages/OOPIFs. A fixed 64-entry initializing-
session registry bridges early `Fetch.requestPaused` events before topology
publication and is reclaimed on initialization failure, target detach or
normal reconciliation.

The six unchanged async/sync method/redirect, fallback-continue and default-
abort rows pass installed candidate and independent oracle 6/6. API-separated
candidate stress passes async 30/30 and sync 30/30; installed-wheel units pass
168/168 and the all-feature host passes 1,003 tests with 26 explicit live
ignores. Full collection remains 6,526/6,526 and the ledger advances to 777
passed, zero failed, seven skipped and 5,742 not run. External `_file` content,
ZIP/update HAR, regex/predicate filters, ordinary Context routes/URL filtering
and `unroute_all` remain explicit gaps.

NODE-325 keeps ABI 79 and proves the complete unchanged async/sync Page HAR URL-
filter/no-fallback pair composed with an ordinary `page.route()`. Installed
candidate and independent oracle pass 2/2; API-separated candidate stress lanes
pass 10/10 each. The ledger advances only those two clean official rows to 771
passed, zero failed, seven skipped and 5,748 not run. Code, feature closure,
ABI and the NODE-323 wheel are unchanged.

NODE-324 keeps ABI 79 and promotes the complete unchanged async/sync Page HAR
method/redirect pair. Installed candidate and independent oracle pass 2/2;
the API-separated candidate stress lanes pass 10/10 each. The mixed async/sync
repeat diagnostic is excluded because pytest-asyncio attempts `Runner.run()`
inside the sync bridge's running loop after the first pair. The ledger advances
only the two clean official rows to 769 passed, zero failed, seven skipped and
5,750 not run.

NODE-323 advances to ABI 79 and adds independently selectable `har-replay`
over `page-routes`. The backend-neutral Rust archive owner accepts at most
eight plain HAR files, 8 MiB per archive, 4,096 entries and 8 MiB retained
archive bytes. Exact URL/method/POST matching, stable best-header selection,
redirect following and embedded plain/base64 response decoding remain in Rust;
the host resolves internal HAR requests directly through its existing Fetch
driver. Python forwards only path, URL filter and `not_found` policy.

The unchanged Playwright Python 1.58.0 persistent-Context HAR row passes the
installed candidate and independent oracle 1/1; candidate stress passes 10/10.
Installed-wheel units pass 167/167, full collection remains 6,526/6,526 and
the cumulative ledger advances to 767 passed, zero failed, seven skipped and
5,752 not run. External `_file` content, ZIP HAR, update recording, Context-
level routing and broad HAR tests remain explicitly unsupported.

NODE-322 advances to ABI 78 and adds independently selectable
`context-http-credentials`, `persistent-context-http-credentials` and
`response-body` leaves. Rust retains credentials for at most 16 selected
Contexts; username, password and optional origin are each bounded to 65,536
UTF-8 bytes. It owns exact-origin matching, selected-session Fetch setup,
64-entry action/attempt/request/initialization bounds and exact-session
`ContinueWithAuth` settlement. `page-routes` and HTTP authentication share one
reclaiming event cursor without overwriting each other's Fetch patterns.

Navigation Responses preserve their Rust request identity and retrieve at
most 4 MiB through `Network.getResponseBody`, with eight concurrent
operations and explicit FFI-copy accounting. Python retains only the bounded
page/frame/request route needed by `Response.body()`; it owns no credentials,
challenge state, CDP session or response bytes. Eleven unchanged official
Playwright Python 1.58.0 Chromium rows pass installed candidate/oracle 11/11
and candidate stress 110/110. Full collection remains 6,526/6,526 and the
ledger advances to 766 passed, zero failed, seven skipped and 5,753 not run.

NODE-321 advances to ABI 76 and adds independently selectable
`context-geolocation`, `context-geolocation-permission` and their persistent-
default compositions. Rust retains at most 16 selected geolocation policies
and 16 permission grants, applies exact `Emulation.setGeolocationOverride`
initialization to every selected Page/OOPIF session, and sends exact
`Browser.grantPermissions` at the browser session for the default or native
owned Context. A separate `persistent-context-launch-arguments` leaf moves
bounded, validated Chromium switches into the existing provider request;
profile, page-opening and remote-debugging ownership cannot be overridden.
Python only validates and forwards the public `geolocation`, `permissions`
and `args` values. The unchanged official Playwright Python 1.58.0 Chromium
geolocation/permission row passes installed candidate/oracle 1/1 and candidate
stress 10/10.

NODE-320 advances to ABI 75 and adds independently selectable
`context-offline` plus `persistent-context-offline`. Rust owns a fixed
16-Context boolean registry; the persistent default consumes one slot, owned
Contexts use only their explicit setting and attached external Contexts inherit
no policy. Every selected Page/OOPIF initializer sends the exact Playwright
Chromium `Network.emulateNetworkConditions` payload with `offline: true`, zero
latency and both throughput fields set to `-1` before Page publication. Python
only validates and forwards `offline` through the same composable native
Context-policy value used by locale, timezone, HTTPS, CSP and JavaScript
policies. The unchanged official Playwright Python 1.58.0 Chromium offline row
passes the installed candidate and independent oracle, including a ten-repeat
candidate stress lane.

NODE-319 advances to ABI 74 and adds independently selectable
`context-timezone` plus `persistent-context-timezone`. Rust owns a fixed
16-Context registry of validated, nonempty timezone identifiers bounded to 256
UTF-8 bytes each. An explicit owned Context records its portable identity;
the persistent default is selected independently; attached external Contexts
inherit neither policy. Every selected Page/OOPIF initializer sends exact
`Emulation.setTimezoneOverride {"timezoneId": ...}` before publication. Only
Chromium's duplicate shared-renderer override is idempotent; invalid timezone
identifiers become stable argument errors and every other protocol failure
propagates. Python only validates and forwards `timezone_id` through a
composable native Context-policy options value. The unchanged official
Playwright Python 1.58.0 Chromium timezone row passes the installed candidate
and independent oracle, including a ten-repeat candidate stress lane.

NODE-318 advances to ABI 73 and adds independently selectable
`javascript-disabled` plus `persistent-context-javascript-disabled`. Rust
retains at most 16 selected owned Context policies and one immutable
persistent-default choice, then sends exact
`Emulation.setScriptExecutionDisabled {"value": true}` initialization to each
selected Page/OOPIF session before publication. Attached external Contexts
inherit no policy. The Python facades only validate and forward
`java_script_enabled`.

The pinned click row also installs thin async/sync `Page.expect_navigation`
over the existing bounded Rust frame-navigation owner; URL filtering remains
explicitly unsupported. Locator click now chooses the center of the first
visible `getClientRects()` fragment when no explicit position is supplied, so
wrapped or transformed inline links are hit rather than the center of their
union box. CDP attachment accepts main-world console replay before the utility
world exists without turning that optional native-lifetime proof into a global
topology failure. The four pinned official Chromium rows pass installed
candidate and independent Playwright 1.58.0 oracle 4/4, candidate same-process
stress passes 40/40, and installed-wheel units pass 157/157.

NODE-317 advances to ABI 72 and adds independently selectable `bypass-csp`
plus `persistent-context-bypass-csp`. Rust admits at most 16 selected Contexts
and applies exact Chromium `Page.setBypassCSP {"enabled": true}` initialization
to every Page/OOPIF session before the session is published. Owned Contexts use
their explicit policy, the persistent default Context uses only its launch
policy and attached external Contexts inherit neither; the persistent default
consumes one fixed Context slot and close releases it idempotently. Python only
validates and forwards the two public booleans. The four pinned official
Chromium rows pass installed candidate and independent Playwright 1.58.0
oracle 4/4, while candidate same-process stress passes 40/40 and installed-
wheel units pass 152/152. The separate upstream iframe CSP row is not claimed:
its baseline currently stops first at the independently missing
`ElementHandle.content_frame` capability.

NODE-316 advances to ABI 71 and adds independently selectable
`persistent-context-extra-headers` over persistent launch plus the existing
Rust extra-header owner. Launch input is bounded to 256 headers / 64 KiB and
transferred into the existing HeaderPool. The first session belonging to the
persistent default Context atomically registers that Context in the existing
HeaderSettings registry before applying `Network.setExtraHTTPHeaders`; owned
and attached external Contexts do not inherit it. Later explicit Context/Page
updates, compensation and session retirement continue through the same owner.
The default consumes one of the existing 16 Context header slots, leaving 15
for later explicit Context header owners, and the native launch call keeps a
fixed 512 KiB copy allowance. The unchanged official Chromium row passes
installed candidate and independent Playwright 1.58.0 oracle 1/1, candidate
same-process stress passes 10/10, and installed-wheel units pass 150/150.

NODE-315 advances to ABI 70 and adds independently selectable
`persistent-context-emulate-media` over `persistent-context` plus the existing
Rust media leaf. The immutable default policy is admitted before initial Page
publication and is replayed by Rust for every Page/OOPIF session belonging to
the persistent default Context. Owned and externally attached Contexts do not
inherit it. Python validates and forwards only `color_scheme`,
`reduced_motion` and `contrast`; it retains no protocol identity or media
state. The three unchanged official Chromium rows pass installed candidate
and independent Playwright 1.58.0 oracle 3/3, while candidate same-process
stress passes 30/30. Installed-wheel units pass 149/149 and full collection
remains 6,526; the cumulative ledger is 743 passed, zero failed, seven skipped
and 5,776 not run. Complete reproducibility hashes stay in the repository
conformance documents so the wheel does not contain a self-referential hash.

NODE-314 keeps ABI 69 and adds bounded public launch-argument preflight for
`BrowserType.launch_persistent_context`. The binding admits at most 256
arguments, 64 KiB each and 1 MiB total, and reproduces Chromium Playwright's
pre-launch errors for user-data-dir ownership, remote-debugging-pipe ownership
and bare arguments that would open a page. An empty sequence remains a no-op;
non-empty switch-only arguments stay explicitly unsupported until a Rust launch
provider owns and tests their behavior. No browser command, ABI entry, native
state or dependency is added.

NODE-313 advances to ABI 69 and adds independently selectable
`context-cookies`. Rust routes exact BrowserContext ownership to Chromium's
browser-root `Storage.getCookies`, `Storage.setCookies` and
`Storage.clearCookies`; it owns URL/domain/path validation, cookie filtering,
filtered expiry, partition identity and all count/byte bounds. Async and sync
Python expose Playwright-shaped `cookies`, `add_cookies` and `clear_cookies`
without retaining a second cookie store.

The fixed four-row official manifest produces the same result in the final
installed candidate and independent Playwright 1.58.0 oracle: clear-cookies
and third-party-cookie behavior pass 2/2; the read/add assertions each fail
only because the mandatory Tailscale fixture has a real MagicDNS cookie domain
while the unchanged upstream test hardcodes `localhost`. Those two rows remain
explicit provider-applicability gaps, not product passes or failures. A
ten-repeat run completes all 40 persistent launches, with the same 20 green
and 20 hostname-only outcomes. Installed-wheel units pass 146/146, fresh
collection remains 6,526, and the candidate ledger advances only the two green
rows to 739 passed, zero failed, seven skipped and 5,780 not run.

Two normalized-payload-reproducible wheel archives have SHA-256 values
`174a30499a3c21c746d4ed5ffb5461247afe012bc9207c32834c68c0801382d4`
and `faa8f553d1643ed4594997acf1fd06dd75f730c3981cbf99fe12d3dda0d69c58`;
their native extension SHA-256 is
`d2332cab6551ab9c189acd77678bf7ba43d20c167e5bc4d9c27cebcdd71b430f`.
Maturin still reports Homebrew OpenSSL dylibs as external, so this evidence is
Mac-local and not yet a self-contained distributable wheel.

NODE-312 keeps ABI 68 and adds async/sync BrowserContext `on`, `once` and
`remove_listener` over the existing Rust-owned lifecycle publication. Sync
preserves exact Page, ConsoleMessage and closing Context facade identity;
Context console callbacks reuse one existing bounded native subscription. The
unchanged official persistent-close Chromium row passes final installed
candidate and independent Playwright 1.58.0 oracle 1/1; candidate same-process
stress passes 10/10. Installed-wheel units pass 141/141. Fresh collection
remains 6,526 and the cumulative ledger is 737 passed, zero failed, seven
skipped and 5,782 not run, with exactly that row changing and no losses.

NODE-311 advances to ABI 68 and adds independently selectable
`persistent-context-device-emulation` over `persistent-context` plus
`device-emulation`. Rust admits the default policy before initial Page
topology, applies it only to the persistent default Context and counts it
against the existing fixed 16-Context / 64 KiB device registry. The persistent
tuple now carries `(base_url, locale, ignore_https_errors, device,
executable_path, channel, timeout_ms)`; Python only validates/forwards the
public options and retains the two-integer `Page.viewport_size` snapshot.
The final installed ABI 68 wheel passes five unchanged official Chromium rows
and the independent Playwright 1.58.0 oracle 5/5; candidate same-process stress
passes 50/50. Installed-wheel units pass 137/137. Fresh collection remains
6,526 and the cumulative ledger is 736 passed, zero failed, seven skipped and
5,783 not run, with exactly those five rows changing and no losses.

NODE-310 advances to ABI 67 and adds independently selectable
`persistent-context-ignore-https-errors` over `persistent-context` plus
`ignore-https-errors`. Rust admits the default policy before initial Page
topology and applies the existing Chromium Security command only to that
default Context. It counts against the same fixed 16-Context registry; Python
forwards one validated boolean and retains no protocol or cleanup owner.
The final installed ABI 67 wheel passes the unchanged official Chromium row
and the independent Playwright 1.58.0 oracle 1/1; candidate same-process stress
passes 10/10. Installed-wheel units pass 136/136. Fresh collection remains
6,526 and the cumulative ledger is 731 passed, zero failed, seven skipped and
5,788 not run, with exactly that row changing and no losses.

ABI 61 adds an independently selectable `worker-events` leaf over
`page-events`. Rust owns dedicated-Worker session retention, Page association,
creation order, readiness, exact detach retirement and value evaluation. The
leaf is bounded at 64 live owners, 128 notices, 1 MiB Worker URLs and eight
concurrent evaluations. Python runs one Browser-scoped native watcher and only
adapts `Worker`, `page.workers`, Worker events and async/sync evaluation.

NODE-300 keeps native ABI 61 and completes Worker retirement across Page,
Context and Browser shutdown. Each terminal facade path closes public Worker
objects once and releases their native owners before Rust waits for transport
shutdown. Post-close evaluation raises `TargetClosedError`; an exact late
detach notice for a Page already removed by terminal cleanup is tolerated
without inventing a detach or retaining an error traceback and CDP connection.

NODE-299 keeps native ABI 61 and adds the caller-thread dispatcher contract
used by Playwright's sync facade. Greenlets suspend only facade calls; every
browser coroutine remains on Pwright's single background loop under the
existing 32-call admission bound. The caller helper loop and running-loop
marker are retired on stop. `greenlet>=3.1.1,<4` is therefore an explicit
wheel dependency, not an alternate browser runtime.

The NODE-307 installed ABI 66 wheel passes the three unchanged official async
Chromium Worker-console rows, the Worker-error row and both Worker-network rows
in candidate and independent oracle. Ten-repeat candidate runs pass 30/30 for
Worker console, 10/10 for Worker errors and 20/20 for Worker network. The
unchanged Worker Handle lifecycle row passes in candidate and independent
oracle, with a 10/10 repeated candidate run. It retains the NODE-300 Worker
lifecycle evidence and NODE-299's complete 27/27 Chromium sync-file evidence.
The Worker console argument-Handle row also passes in candidate and independent
oracle, with a 10/10 repeated candidate run. The Context-locale Worker row now
also passes installed candidate and independent oracle, with a 10/10 repeated
candidate run. The complete Chromium Worker file passes 13/13. The unchanged
persistent-Context locale row passes candidate and independent oracle, plus
10/10 repeated candidate runs. The 6,526-row candidate ledger is 721 passed,
zero failed, seven skipped and 5,798 not run, with exactly that Chromium row
changing from not run to passed since NODE-306 and no losses. Installed-wheel
Python units pass 132/132.
This is partial-corpus evidence, not a full Playwright compatibility claim.

ABI 60 adds an independently selectable `download-events` leaf. Rust installs
exact-Context Chromium download policy, correlates Browser download events to
the portable Page, owns GUID-named temporary artifacts and completion state,
and bounds the capability at 16 Contexts, 64 owners, 64 notices and 4,096-byte
paths. Python runs one Browser-scoped native watcher and only exposes the
async/sync `Download` facade. The Rust Locator click navigation barrier also
recognizes a same-Frame `Browser.downloadWillBegin` as the terminal outcome of
a navigation that became a download.

The final ABI 60 wheel passes the unchanged official
`test_sync_download[chromium]` and 25/27 of the complete unchanged Chromium
`tests/sync/test_sync.py`, versus 27/27 in the independent official oracle.
The two remaining sync-file gaps are Worker events and the official
`sync_gather` helper's caller-loop assumption. The 6,526-row candidate ledger
is 705 passed, two failed, seven skipped and 5,812 not run, with exactly the
download row changing from failed to passed since ABI 59 and no losses. Local
Python units pass 123/123. This remains partial-corpus evidence, not a full
Playwright compatibility claim.

ABI 59 adds an independently selectable `request-events` leaf over the
existing Rust network observer. Rust owns Context subscription, request
capture, Page association, retained request/history ownership, a 256-notice
FIFO and the shared 16 MiB queue budget. Python runs one bounded native watcher
per Browser and only publishes async/sync `Request` wrappers to exact Page
listeners; it does not poll CDP or retain a second event queue.

The final ABI 59 wheel passes the unchanged official
`test_sync_network_events[chromium]` and 24/27 of the complete unchanged
Chromium `tests/sync/test_sync.py`, versus 27/27 in the independent official
oracle. The three remaining sync-file gaps are successful download
publication, Worker events and the official `sync_gather` helper's caller-loop
assumption. The 6,526-row candidate ledger is 704 passed, three failed, seven
skipped and 5,812 not run, with exactly one ABI 58 failed-to-passed change and
no losses. Local Python units pass 120/120. This remains partial-corpus
evidence, not a full Playwright compatibility claim.

ABI 58 completes `ConsoleMessage.args` with ordered Rust-owned argument Handle
publication and bounded pending/event storage. Page and BrowserContext console
subscriptions are bounded native state: Python only registers or releases
listener interest and wraps the published Handles. Persistent listeners are
active immediately; action waiters have unique native tokens. Navigation
waiters activate at the main-frame commit boundary so a late favicon error
from the previous document cannot satisfy a waiter for the next navigation.
Type, text, location, representation and argument Handles are available
through the same ordered event projection. ABI 57 introduced the separately
selectable `console-events` projection and remains wire-incompatible with this
descriptor.

The final ABI 58 wheel passed both complete pinned Chromium console modules,
8/8 async and 8/8 sync, and 23/27 of the complete unchanged Chromium
`tests/sync/test_sync.py` versus 27/27 in the independent official oracle. The
four remaining sync-file gaps are request events, successful download
publication, Worker events and the official `sync_gather` helper's caller-loop
assumption. The 6,526-row candidate ledger is 703 passed, four failed, seven
skipped and 5,812 not run; the oracle ledger is 688 passed, zero failed, seven
skipped and 5,831 not run. Local Python units pass 118/118. This remains
partial-corpus evidence, not a full Playwright compatibility claim.

ABI 56 adds the separately selectable `element-handle-dom` composition. Rust
compiles descendant selectors relative to the retained ElementHandle,
publishes or releases the first result, and reads `HTMLElement.innerText`.
The async/sync Python methods only forward arguments and wrap the native
result.

ABI 55 adds thin async/sync `Page.wait_for_selector` facades. Python validates
the public arguments and derives native `nth(0)` for default non-strict Page
selection. Rust resolves the Frame and runs the existing Locator wait under one
total deadline, returning an existing bounded `ElementHandle` owner for
attached/visible or `None` for detached/hidden. The unchanged upstream
cancellation case passes without Python polling or a teardown hang.

NODE-288 keeps ABI 55 and completes the public timeout-exception shape used by
selector waits. Rust still classifies `PWRIGHT_TIMEOUT`; Python exposes the
official `.name` property on the existing native exception classes and keeps a
setter for browser-originated Page-error names. No wait, protocol or resource
owner moves into Python.

NODE-289 keeps ABI 55 and completes the applicable Chromium rows in async
`test_asyncio.py`. Rust still classifies evaluation failures as
`PWRIGHT_JAVASCRIPT`; the thin Page facade rebuilds the same native exception
type with Playwright's `Page.evaluate:` API prefix while retaining code, name
and browser stack. No evaluation, protocol or resource owner moves into Python.

NODE-290 keeps ABI 55 and records all 25 previously unexecuted Chromium rows
in sync `test_sync.py`. Exact public object representations stay in the thin
facades. `Response.request` uses the request-history owner published by Rust
with the Response rather than deriving request identity in Python. The
repaired tranche is still red at 14 passed / 11 failed.

ABI 54 adds the separately selectable `file-chooser-events` composition. Rust
enables Chromium interception on each selected Page/OOPIF session, maps
`Page.fileChooserOpened` to the exact Page, utility world and backend node, and
publishes through the existing Page lifecycle fence. At most 16 opaque chooser
owners are retained, with 2 KiB per owner plus 1 KiB installation state. The
owner keeps only a weak native connection so a retained chooser cannot prevent
Context/Browser teardown. Async/sync Python retain public waiter, predicate and
wrapper identity and forward payloads or paths to the existing bounded Rust
file-input service.

ABI 53 adds separately selectable Context request waits, exact shared request-
history ownership, `Request.frame` and `Request.post_data_json`. The optional
frame leaf observes `Page.frame*` and `Network.*` on the same CDP cursor,
bounded to 64 Frame IDs and 64 active-request flags per Page, so the popup's
first navigation request preserves Playwright's unavailable-Frame error even
if topology publication advances before the waiter consumes the request.

ABI 52 adds the separately selectable `response-finished` leaf on top of
bounded response publication. Rust retains at most 256 terminal owners and
64 KiB total owner state, settles success/failure/closure from terminal
network events, and releases capacity when owners drop. Async and sync
`Response.finished()` facades only forward the opaque native owner.

ABI 51 added the separately selectable `wait-for-timeout` leaf. Async and sync
`Page.wait_for_timeout()` facades validate milliseconds and forward them to
Rust. The host admits 16 concurrent waits, retains at most 12 KiB for the leaf,
and cancels waits on session stop or transport disconnect.

ABI 22's persistent-Context slice currently accepts `headless`, `base_url`,
`executable_path`, `channel`, and `timeout`. Rust selects and launches an
already-installed Chromium executable, owns the process lease, and performs a
bounded graceful-close/force-cleanup sequence. Other persistent-Context
options, non-Chromium browsers, and generic `BrowserType.launch` remain
unsupported rather than being silently ignored.

ABI 23 adds the separately selectable `page-pause` leaf. Headless pause yields
after Rust temporarily resets both timeout kinds to zero and restores the exact
prior values on normal completion, cancellation, or drop. The thin Python
facade forwards only Page identity; it does not implement pause timing or
selector semantics.

ABI 24 adds the separately selectable `request-gc` leaf. Async and sync Python
facades forward only Page identity; the existing Rust host resolves the exact
Chromium session and dispatches `HeapProfiler.collectGarbage` without retaining
Page state.

ABI 25 extends the separately selectable `page-errors` leaf with Rust-owned
history. Async and sync `Page.page_errors()` facades forward only Page identity
and convert the returned bounded `(name, message, stack)` values into public
`Error` objects. Rust retains the newest 200 errors per Page, admits at most 16
Pages and 3,200 history items, caps retained error bytes at 16 MiB, and releases
the history on Page/session teardown.

ABI 26 adds a separately selectable `console-history` leaf. Async and sync
`Page.console_messages()` facades forward only Page identity and convert the
bounded Rust snapshot into public value wrappers. Rust retains the newest 200
messages per Page across 16 Page slots and 3,200 items with a 16 MiB byte cap.
The leaf does not install console callback publication. Public `type`, `text`,
`location`, `page`, and `worker` values are available; `args` stays explicitly
unsupported until bounded retained remote-object ownership is implemented.

ABI 27 adds a separately selectable `request-history` leaf over the existing
network-events composition. Async and sync `Page.requests()` return the newest
100 Requests per Page; Rust preserves user-held evictions so all retained
objects keep working through `Request.response()` and `Response.text()`. The
leaf admits 16 Pages, 256 retained owners per Page, 4,096 owners globally and
64 MiB of event bytes. Its FFI observation lane admits 256 concurrent reads;
the unrelated command lane remains capped at 32.

ABI 28 adds a separately selectable `device-emulation` leaf. Rust embeds the
complete 143-entry Playwright v1.58.0 catalog, admits at most 16 configured
Contexts and 64 KiB of retained settings, and applies the settings in the
existing Chromium session-initialization fence before Page publication. The
async and sync facades only expose typed descriptor mappings, browser-type
lookup and bounded argument conversion; they do not issue CDP commands.

ABI 29 adds a separately selectable `response-events` leaf over the existing
network-events composition. Rust activates response interest before Context
publication, coordinates the shared response tag with `request-history`, and
owns the bounded FIFO and byte accounting. Python runs one bounded native
watcher per Browser and retains only public `Response` wrappers plus callback
identity for `on`, `once`, and `remove_listener`. The host recovers sticky CDP
topology loss with one authoritative same-connection reconstruction and one
epoch advance; Page membership is not inferred from transient command-session
routing. Sync-listener completion also preserves live network adapters until
that snapshot finishes, then retires only missing Pages. The connection replay
window is bounded at 2,048 events for the declared host profile.

ABI 30 adds no capability feature or dependency edge. It exports native
`eval_on_selector_all`, exposes thin async/sync Page facades and parses
Playwright `internal:and=<JSON selector>` into Rust's typed intersection. The
connection profile independently bounds 8,192 queued event items and 512 MiB
of retained event payloads, preserving a finite byte ceiling while admitting
small `Runtime.enable` replay bursts. The ABI increments because the native
method surface changed. The all-feature profile also keeps
`network-events + concurrent-close` composable by taking the maximum declared
runtime object capacity instead of letting native close replace the network
owner's bound.

ABI 31 adds the separately selectable `selector-registration` leaf. Rust owns
the global registry, duplicate/predefined/invalid-name errors and teardown,
bounded at 64 engines, 64-byte names, 1 MiB per source and 8 MiB aggregate
bytes. Async and sync Python facades only normalize string or bounded `path=`
input and forward the call. This implements registration, not execution of a
custom JavaScript selector engine inside browser queries.

ABI 32 adds no capability feature. It exports native selector validation over
the existing bounded Rust Locator query recipe. `Page.query_selector` forwards
Frame identity and selector for validation before its browser call, preserving
Playwright's unknown-engine error without implementing a Python selector
parser. Registered-engine browser execution remains unsupported.

ABI 33 originally added no capability feature or dependency edge. Rust compiles
registered-engine chains into bounded immutable programs, installs each
distinct referenced engine once and selects main or utility execution from
`content_script`. Python only forwards public calls. The registry accounts at
most 8 MiB of name/source payload plus fixed 64-entry structural overhead;
programs admit 256 parts and 2 MiB. Positive registered
`Page.query_selector` handle publication, typed Locator composition and retry
polling remain explicit unsupported gaps.

NODE-253 keeps ABI 33 and adds the independently selectable
`layout-selectors` leaf. Rust parses and compiles the official
left/right/above/below/near layout clauses and top-level adjacent-sibling
composition into a bounded one-shot browser-world scoring program. The
existing selector source limit, 256 structural parts and 256 KiB emitted-
program ceiling apply. Python reuses the ABI-stable internal browser-program
count call and contains no grammar or geometry implementation. Arbitrary
nested layout shapes, retry polling and positive ElementHandle publication
remain unsupported.

NODE-254 keeps ABI 33 and adds no feature or dependency edge. Basic Locator
methods forward to existing Rust read/action/evaluation owners; collection
text reads use typed Rust operations rather than Python-authored browser
scripts. Rust also recognizes quoted Playwright `:text()` as a typed exact
text selector. Locator composition, strictness and remaining actions are not
implied by this slice.

NODE-255 advances to ABI 34 because it exports `locator_expect_count`; it adds
no feature or dependency edge. Rust owns typed count reads, expected/negated
comparison, one total deadline, retry after navigation/context replacement
and bounded 10 ms sleep. Async/sync Python assertions forward only selector,
expected count, timeout and negation and retain no polling state.

NODE-256 keeps ABI 34 and adds no feature or dependency edge. Locator
description is immutable Python facade metadata; `describe` returns a new
wrapper over the same Page and selector without issuing browser/native work.
Typed Locator chaining remains unsupported rather than being encoded as a
Python-authored selector string.

NODE-257 advances to ABI 35 for a frozen native Locator owner and adds no
feature or dependency edge. Root selectors are admitted into the existing
bounded Rust query registry; chained `locator()` uses typed within-composition,
and first/last/nth derive typed Rust recipes. Reads and count expectations
execute through that native owner. Python retains only the facade reference
and optional description metadata.

NODE-258 through NODE-260 run the complete unchanged async/sync Chromium
Locator files instead of a green manifest: the original candidate is 60/163,
the independent official oracle is 163/163, and the repaired candidate is
79/163. Rust now also owns bounded `all`, Frame-rooted Locator creation and
additional actions. CDP attach does not eagerly capture request/response
traffic from borrowed default/external Contexts; their response/history
activation remains an explicit demand-driven compatibility task.

NODE-261 through NODE-263 advance the native ABI to 38. Typed filter,
has/has-not, visible, boolean combination, wait, expectation, role, test-id and
single-Locator evaluation operations all retain immutable Rust query/evaluation
owners. Python performs bounded public-value conversion and facade adaptation;
it does not parse selector chains or run polling loops. The successive complete
Locator results are 83/163, 107/163 and 110/163 with no regressions.

NODE-264 keeps ABI 38. The Rust selector parser maps `visible=true/false` to
`FilterVisible`, and selector runtime version 9 includes an element root for
relative user-facing engines. The complete candidate is now 114/163 (62/86
async, 52/77 sync) versus the unchanged 163/163 oracle. The cumulative
6,526-row ledger is 563 passed, 49 failed, seven skipped and 5,907 not run;
this is not a full-suite compatibility claim.

NODE-265 advances to ABI 39 for Rust-owned double-click, bounded path/payload
file input and element-handle publication. Both complete Locator files pass
125/163, exactly 11 gains and no losses. The cumulative 6,526-row ledger is
574 passed, 38 failed, seven skipped and 5,907 not run.

NODE-266 advances to ABI 40 and adds the independently selectable
`locator-highlight` facade over the existing Rust overlay implementation. The
feature adds no package to the base Pwright dependency closure. Both targeted
highlight rows pass, then all 163 Locator rows pass 68/86 async plus 59/77
sync: 127/163, exactly two gains and no losses. A dedicated Chrome restart is
recorded in the raw async report; seven setup-affected rows were rerun exactly
and merged into the repaired report. The cumulative 6,526-row ledger is now
576 passed, 36 failed, seven skipped and 5,907 not run, SHA-256
`6976549c14fa4000f16617a0226b53753f54580907a5294aa413eea1d40f2cce`.
The independent Locator oracle remains 163/163. This is not a claim that the
remaining Python or Core JS/TS corpus passes.

NODE-267 advances to ABI 41. The independently selectable
`locator-dom-actions` feature keeps `dispatch_event` and `select_text` source,
strict resolution, timeout and bounded input admission in Rust without adding
a Pwright package to the 108-package base closure. `evaluate_handle` composes
the existing bounded Handle owner and returns an owned native Handle with a
Rust-computed preview. All six targeted official rows pass; the complete
Locator files pass 71/86 async plus 62/77 sync, 133/163 total, exactly six
gains and no losses. The cumulative 6,526-row ledger is 582 passed, 30 failed,
seven skipped and 5,907 not run, SHA-256
`1feb3bb3ec44812ee930defa5471cd8e8454e2d8c49e9502059273f1c360fe41`.
This remains a partial corpus result; all 48,477 Core JS/TS invocations and the
unexecuted Python rows remain open.

NODE-268 keeps ABI 41 and the 108-package dependency closure unchanged. Rust
now owns Playwright-shaped capture-plus-`nth` rejection, bounded Locator click
call logs and role descriptions, Unicode timeout diagnostics, and the admitted
simple relative-XPath `has=` subset. All eight targeted official rows pass;
the complete Locator files pass 75/86 async plus 66/77 sync, 141/163 total,
exactly eight gains and no losses. The cumulative 6,526-row ledger is 590
passed, 22 failed, seven skipped and 5,907 not run, SHA-256
`6180ffa49de77b48b2a61d13a8fe3f159654c4e5c4529d841082377112d0ca46`.
The independent Locator oracle remains 163/163. The remaining Python rows and
all 48,477 Core JS/TS invocations remain open; this is not a full-suite claim.

NODE-269 advances to ABI 42 for an independently selectable `page-viewport`
leaf. Rust owns a fixed 64-Page / 8 KiB registry, CDP window/metrics override,
teardown and session-initialization replay; async and sync Python only validate
and forward width/height. The four targeted official rows remain red at this
checkpoint because the successful viewport operation next exposes missing
Locator screenshot and bounding-box facades.

NODE-270 advances to ABI 43. `Locator.bounding_box` reuses the existing bounded
Rust geometry owner, while the separately selectable `locator-screenshot`
profile owns strict resolution, stability, scrolling, iframe clip translation,
the 2 MiB output ceiling and at most two concurrent captures. All four targeted
viewport-dependent rows pass. The complete Locator files pass 77/86 async plus
68/77 sync, 145/163 total. The cumulative 6,526-row ledger is 594 passed, 18
failed, seven skipped and 5,907 not run, SHA-256
`811c14419d0fee688717168bfb0118d07cfaa82c27d9d8c2e0f14650d01348fe`.
The independent Locator oracle remains 163/163. The remaining 16 FrameLocator
and two callback-completion discrepancies, the other Python rows and all
48,477 Core JS/TS invocations remain open.

NODE-271 advances to ABI 44. The lazy native FrameLocator owner resolves every
iframe hop at operation time through a temporary, explicitly disposed Handle;
selector composition, strictness, retries and deadlines remain bounded in
Rust. The thin Python facades add Page/Frame/Locator entrypoints, owner/content-
frame conversion and visibility assertions. The final focus/blur rows use the
same public-handler arity adaptation as official Playwright Python while Rust
continues to serialize the browser `Event` and own binding publication.

The final installed wheel passes the complete unchanged Locator files: 86/86
async plus 77/77 sync, matching the independent 163/163 oracle. The cumulative
6,526-row ledger is 612 passed, seven skipped and 5,907 not run, SHA-256
`89eabed04a3a7804637229ff077cf0898cc1aa903c190611bfb2d1553650aefe`.
Two normalized-reproducible ABI 44 wheel SHA-256 values are
`bfdbcb48279fbc5377e84dbc2d40b101c186cbb74b25fade4582646db848e100`
and
`ef00f6f69fe0dfed04818d0580e9d2e564c6b75c2e08cb2526f7a7aee1c778af`.
The other 5,907 Python rows and all 48,477 Core JS/TS invocations remain open;
this is not a full-suite claim.

NODE-272 advances to ABI 45. The semantic Locator entry points compose typed
Rust selector recipes from Page, Frame, Locator and FrameLocator. Attribute
expectations keep their total deadline, retry, missing-element behavior and
exact/regular-expression matching in Rust; Python only validates public values
and adapts the bounded result.

The complete unchanged sync `test_locator_get_by.py` Chromium file passes
10/10 in both the final installed candidate and independent oracle. The pinned
upstream tree has no async counterpart. Candidate and oracle report SHA-256
values are
`95362db9ab9b86874567fcee6bda9871f5c23e849005c0e9d03023ffc07abc73`
and
`62ec0bb876133ab0fc1be15665e5b637215c0120e263de6fdf5ffc9cb23118f5`.
The cumulative candidate ledger is 622 passed, seven skipped and 5,897 not
run, SHA-256
`e37434aa74c4dea2f6f8001ae27d657e833cf0c197207b0186b5d133b9a40813`.
Two normalized-reproducible ABI 45 wheel SHA-256 values are
`e0321e22e0ab05b57608f48d180689009d6113a55525a0b81a7a5776e04b71b0`
and
`0c30faaf48699e0b95426204f8c28e3e868abb5e94b2dccc004050ba61ac098b`.
Installed units pass 78/78 and full candidate collection remains 6,526/6,526.
The other 5,897 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-273 advances to ABI 46 by exporting native Locator collection evaluation
over the existing bounded Rust owner. A typed semantic recipe now flows from
construction through exact collection resolution and utility-world execution
without being serialized back into a Python selector string.

The complete unchanged async `test_selectors_get_by.py` Chromium file passes
3/3 in both final installed candidate and independent oracle. Candidate and
oracle report SHA-256 values are
`cc1b66557ae5510345f4dae1163f86a4dce8af8c344d81530586fc0f3fdddf12`
and
`c0150e1af71e0e262d2a4edf5c5bfa3463aaef0272a5f6bac4aa00e975b32d02`.
The cumulative candidate ledger is 625 passed, seven skipped and 5,894 not
run, SHA-256
`39fc2c66a96066e5d0275b146688a48a7dbb1e8a3aeaf37f3971d598cbee4e4b`.
Two normalized-reproducible ABI 46 wheel SHA-256 values are
`a665627bdb6bdda5ce0e8e2a72a37999b222c10ffa5e8ba44637f0f52da52900`
and
`e7a4c8933d3167ad24fea61e3e6354fa06a0df8cb6ccbb44662cee467712b20d`.
Installed units pass 79/79 and full candidate collection remains 6,526/6,526.
The other 5,894 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-274 keeps ABI 46 and executes the complete unchanged async/sync fill and
context-manager files. The five fill rows use the existing Rust-owned Locator
fill owner. Page, BrowserContext and Browser add only thin language-level
context-manager protocols; exit delegates to their existing close paths. The
independent-oracle CDP fixture hides only Contexts that predate adapted launch,
matching candidate fixture semantics without weakening upstream assertions.

Candidate and oracle each pass all nine Chromium rows. The combined tranche
ledger SHA-256 values are
`8c727149b0e6c9b2b8712845a3a55dc81413a0888a4cd2799193246f8f95e0ab`
and
`db9950cb4d39a64ddc86e4adaf9fedf40f25202ef8852172e4811254debb3651`.
The cumulative candidate ledger is 634 passed, seven skipped and 5,885 not
run, SHA-256
`8bb2f5b5437477779c6eb6690da2f247d3f377422f62b1d8bd3ee0cafb5f3908`.
The cumulative oracle ledger is 615 passed, seven skipped and 5,904 not run,
SHA-256
`b02be1e0e0f789d575725098853e060d596514ddc7031fd2c0060883a53d080d`.
Two normalized-reproducible ABI 46 wheel SHA-256 values are
`6f2f3e4668083b081608d8ce92e1cdbe1f6040ea8d225f4bcfff44c9a62bc3ed`
and
`51c25e0d628d2dac7acf07c596c6815637000b86efba9056d6eac504d6b9096e`.
Installed units pass 81/81; strict Ruff/Pyright and the dependency proof pass;
full candidate collection remains 6,526/6,526. The other 5,885 Python rows and
all 48,477 Core JS/TS invocations remain open.

NODE-275 advances to ABI 47 with a separately selectable Rust-owned
`ignore-https-errors` policy. Rust preallocates a fixed 16-Context registry,
installs `Security.setIgnoreCertificateErrors` on every owned Page/OOPIF
session before publication, and removes the policy on Context close. Python
strictly validates and forwards the public boolean; it owns no TLS or session
state.

The complete unchanged async HTTPS-error file expands to six rows. Candidate
and independent oracle each pass both Chromium rows; the two Firefox and two
WebKit rows remain explicit provider-gap `not-run` outcomes. Candidate/oracle
report SHA-256 values are
`cdd4eb40b5c8c40ede8c84f73056c5faf229837c497bf889d66545612510bea8`
and
`1f3179fc8f7deee06eb07ea775d238c9896bad297dd74f0afa1732732c4bef59`.
The cumulative candidate ledger is 636 passed, seven skipped and 5,883 not
run, SHA-256
`690f4b86c509932323cd613f2f40fede63a06e86133209bf98944421ae3d70d8`.
The cumulative oracle ledger is 617 passed, seven skipped and 5,902 not run,
SHA-256
`647b129c3901df6eb3337e7d886e88f8cbc7322653a71b38d7e99ac1c65bd204`.
Two normalized-reproducible ABI 47 wheel SHA-256 values are
`4074ac94f00f9463cf337e67af4b73671cf9d152ea1f6582735c9ad3265d49e7`
and
`5f4a2a2d71a0c6290344bd35b2b5ac2305e7653e21ce466d202e7dc584cdba7c`.
Installed units pass 83/83; strict Rust/Python gates and the dependency proof
pass; fresh candidate collection remains 6,526/6,526. The other 5,883 Python
rows and all 48,477 Core JS/TS invocations remain open.

NODE-291 advances to ABI 56 and repairs the two ElementHandle rows from the
NODE-290 sync tranche. Both pass from the final wheel; the repaired tranche is
16 passed / nine failed and the complete file is 18/27 candidate versus 26/27
oracle. Rust compiles the selector relative to the retained element, publishes
or releases the first result and reads `HTMLElement.innerText`; Python only
forwards and wraps. Final report and merged-report SHA-256 values are
`55449eed1e22968a614813cec9eb8daab124a1aec6b22b1342c6deed361f3dbd`
and
`c118e64d7029d39463f1d1a8212398b1b545bc76f6cb1f88bcff5a1871619fb8`.
The cumulative candidate ledger is 682 passed, nine failed, seven skipped and
5,828 not run, SHA-256
`a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`.
Normalized-reproducible ABI 56 wheel SHA-256 values are
`faba2f1c41352d1548e3e94505a3c4ff1ce4b2da39dc8d1b26861355e74888b7`
and
`c5501536fbdfc473c71fde14c313d68f6051645c6865fcd6df72c1d509364349`.
Installed tests pass 111/111 with five expected skips; focused Rust tests,
release Clippy, Ruff, strict Pyright and fresh 6,526/6,526 collection pass.
Collection report SHA-256 is
`f3374fe294c20aff18ae1d279acf92ebf70cbe2d0af13813173151eafd5f6a42`.

NODE-290 keeps ABI 55 and executes all 25 previously unrecorded Chromium rows
from unchanged sync `test_sync.py`, source SHA-256
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
The independent oracle passes 24 and fails
`test_sync_playwright_multiple_times[chromium]` in this isolated harness,
report SHA-256
`3ce79c1d557e9593ba2c4f63db88e3d17db4231e7544516baa19b1e7c73bcb63`.
The candidate baseline is 9 passed / 16 failed, report SHA-256
`d8bb9824057c1381c2d32158d84c5d92570ea743f69f60a5ebc68ed36dc99626`.

Four repr rows now use exact public Page, BrowserContext, Browser and
BrowserType shapes. The official shim alone exposes `_impl_obj` for the
upstream test's private expected-value construction; the product does not add
that private API. At this ABI 55 snapshot, `BrowserType.executable_path` was
empty because this package did not bundle a browser. ABI 106 later added
registry metadata without bundling a browser. A fifth repair carries the
existing Rust request-history owner through Response wait and notice publication, so thin
async/sync `Response.request` wrappers preserve the native Request identity.

The five repaired rows pass from the final wheel. Exact replacement produces
14 passed / 11 failed for the 25-row tranche, report SHA-256
`d9b84034f306be4f0bc61ceb9438a2aa79e19ce470c0906981ba17507514696f`.
Together with two earlier green rows, every one of the file's 27 Chromium rows
now has an outcome; the file is not green. Candidate/oracle tranche-ledger
SHA-256 values are
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
release Clippy, Ruff and strict Pyright pass. Fresh candidate collection
remains 6,526/6,526, report SHA-256
`823aa604740e27cd7b0bde70e86dd8d8098077cc4dc7781344942db8139f5fd1`.
The 11 sync failures, other 5,828 Python rows and all 48,477 Core JS/TS
invocations remain open.

NODE-289 keeps ABI 55 and selects the two remaining Chromium rows from
unchanged async `test_asyncio.py`, source SHA-256
`544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`.
TaskGroup cancellation passes in the baseline; the API-name row initially
fails only because the native `JavascriptError` has no `Page.evaluate:` public
prefix. The thin Python repair preserves its type, stable code, public name
and stack. Final installed candidate and independent oracle pass 2/2 with
identical report SHA-256
`6328485903ac03a3226871803ca6380c99f6e2369a9c94b6f98c529a28abbb0f`.
Candidate/oracle tranche-ledger SHA-256 values are
`a8571298097874f152b85105f5b6e7b41d3ded9aa7811405d454cb0bdd20adba`
and
`6b7d99b7dd58f684e11e6ea191671a02a231f3b6e46663c3f1b58866edae5b88`.
The cumulative candidate is 666 passed, seven skipped and 5,853 not run,
SHA-256
`2e8f750a7e2a16af688489ef6aff90fc9467946109b18a007eccfb5e4626fd0d`;
the cumulative oracle is 647 passed, seven skipped and 5,872 not run,
SHA-256
`6d54d97cd037a0e17c8b8f1ce923a6c8459343e04b41ffa19eb5cc5ee98d3215`.
Normalized-reproducible repaired ABI 55 NODE-289 wheel SHA-256 values are
`933297c96848dbbe99381ef1a353100bfae8ac9db12c9ec21bba16d3f007aef3`
and
`d11377c024ed993dd37b345e95481e0fdc91fb6074e4a4d30a499b80e6ee12e2`.
Installed tests pass 107/107 with five expected skips; Ruff, strict Pyright and
fresh 6,526/6,526 candidate collection pass. The other 5,853 Python rows and
all 48,477 Core JS/TS invocations remain open.

NODE-288 keeps ABI 55 and selects three Chromium rows from unchanged async
`test_asyncio.py`, async `test_expect_misc.py` and sync `test_sync.py`, source
SHA-256 values
`544f9193333ed0df5d87e0f5978b00f04b709aaba80e477cae1e5d1701b1a0ae`,
`61e5d93df621e59a44717532192194bce1a85cc858d5a1f2e69ba724ce0945a6`
and
`f371ee96a8eacc0721e7e3514223992d37ecba8fec86e6396584de4049e03579`.
Candidate and independent oracle pass cancellation without an unretrieved
future, timeout error naming and sync selector wait, 3/3. Report SHA-256 values
are
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
the cumulative oracle is 645 passed, seven skipped and 5,874 not run,
SHA-256
`b4bd2c9c36f3508b9a5b359d95d7d7685801bede216a55474222b1507ea21565`.
Normalized-reproducible repaired ABI 55 NODE-288 wheel SHA-256 values are
`5c2dcc36835eb0c5b26b60b6499824a843175910eba503f0d9ec6b6244f973f1`
and
`53016151567480d1af17f39111fb380b3e6148a88ab78f1aeaca1e5f733db0db`.
Installed units pass 106/106 with five expected skips; Ruff, strict Pyright and
fresh 6,526/6,526 candidate collection pass. The other 5,855 Python rows and
all 48,477 Core JS/TS invocations remain open.

NODE-287 advances to ABI 55 and pins unchanged async `test_issues.py`, source
SHA-256
`5d99e3d911d610cdb116e6b2c9e7d7ac7de8df051cfbe823712297465c375bea`.
The two applicable Chromium rows pass in final installed candidate and
independent oracle, including cancellation of a pending selector wait; report
SHA-256 values are
`18b7130d89320ae47ee72ee45c452d4d6a7b7204885921a1b5b7b624b6851718`
and
`991b5bf6cc6fc47d38cf7f4ce02fbca7964ef3b6c2c59bad9ec71cff2bcacb1f`.
The generic-launch row and the environment-sensitive service-worker-policy
diagnostic remain explicit gaps outside the green ledger.

Candidate/oracle tranche-ledger SHA-256 values are
`67542a752ae8dbd54d91dfab1ed69ca7ae59995a59bbb12894337493f05b00d0`
and
`c59574200b25fb454bccc03e7ad041ebe0bfc652c1fb8213db3ef93b192aad96`.
The cumulative candidate is 661 passed, seven skipped and 5,858 not run,
SHA-256
`169dfbabd9bf984d59082cdf4c0b68a84941a67c7e04c8997cc63645243561e9`;
the cumulative oracle is 642 passed, seven skipped and 5,877 not run,
SHA-256
`a2f4c2118f7136c5f607e594638b68acc95fb88edd1e7f308f76cd799b4f4499`.
Normalized-reproducible repaired ABI 55 wheel SHA-256 values are
`11487ff7a6afb5b7d4ba021cc746dea4003bb646bfc3301b7a74a4b79a674869`
and
`4b493951e935460f9b3d752424f41b60bf58b10315306fdfb3633ffd141981aa`.
Installed units pass 105/105 with five expected skips; selected Rust units pass
497 with 16 explicit live ignores; strict Rust/Python and the 108-package
dependency proof pass. The other 5,858 Python rows and all 48,477 Core JS/TS
invocations remain open.

NODE-286 advances to ABI 54 and pins the complete unchanged sync
`test_input.py` file, source SHA-256
`c203939164958aca0babcee63f2d9ae58afce16199fc2b6b5adbc4d6b2284fed`.
The two Chromium rows pass in final repaired-wheel candidate and independent
oracle through the same dedicated CDP Chrome; report SHA-256 values are
`2b2f38ee68d026f3ea1d6eb6f46f8e30edfb8a2a0882921e2cd57622a50d1d38`
and
`0c84f8e75291bdae4bd87685ba9efbc86c12e598b5859a1b7e1f8d7e42e15685`.
Four Firefox/WebKit rows remain explicit provider gaps. Rust owns interception,
exact session/target/backend-node routing, Page-fenced publication and bounded
payload/path mutation. The initial strong-connection owner passed the test body
but hung teardown; the final weak route exits cleanly.

Candidate/oracle tranche-ledger SHA-256 values are
`f39fc100ccaf2601cf852b32ca3e8c6543f816e0c34b6ff82eb251673f777b89`
and
`ab9953fae7ddd08720f454d84fb5be75ec0b61b3abf48f6bbd95a2eefc37ebea`.
The cumulative candidate is 659 passed, seven skipped and 5,860 not run,
SHA-256
`557e937c00ccc8e9065c37df2c14fde9b7d71ae0c1b9e011f02171b52d71730a`;
the cumulative oracle is 640 passed, seven skipped and 5,879 not run,
SHA-256
`d8bb9714f250d46c17c2afd6e5468eb98f8acf8a7a0f84f0a016bffd8d553534`.
Normalized-reproducible repaired ABI 54 wheel SHA-256 values are
`ff2b00f7524235c3dbdfb1f93f3e29b7d9b3da885b550819c8be2b8df0eb6752`
and
`6584f4b92359e0fd1cddcfaf0bb814980b682eec5a2033457077c97c005d385c`.
Installed units pass 103/103; selected Rust units pass 497 with 16 explicit
live ignores; strict Rust/Python and the 108-package dependency proof pass.
The other 5,860 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-285 advances to ABI 53 and pins the complete unchanged async
`test_page_network_request.py` file, source SHA-256
`6ead05641f44950db30e7ab038c973226ebe726312f39b9d7d7de2aacaf4a388`.
The two Chromium rows pass in final installed candidate and independent oracle
through the same dedicated CDP Chrome; report SHA-256 values are
`c6fd93810787dea30f3beb58ec44202bdff7aad5975f1259852f363faae454cc`
and
`1d0416cb3d1e95cb2eab41e3118b7444fe2e47403a6bd98039d70837b392cba6`.
Four Firefox/WebKit rows remain explicit provider gaps. Three consecutive
NODE-281-through-NODE-285 candidate regressions pass 10/10, SHA-256
`60b7992906b838a93a98f8c61aa2a803dbc7485e9b3a358530aaea3f562b26f2`
and
`ac10e36d1bc833396547276ac1af8e9c79ff0932a5e2eaae9201936621bb34f7`
and
`cfe2c9dc9e7733e09e9b26bce562837a1b0a0dd75fe44347f0a2c6d81c7adac9`.
The cumulative candidate is 657 passed, seven skipped and 5,862 not run,
SHA-256
`83ed08bf4d1f711e6d506db5a2315bc6e7aed49dff4717d0db95750074c0318b`;
the cumulative oracle is 638 passed, seven skipped and 5,881 not run,
SHA-256
`eb03126624a1c047420fa30d1a8ab1b8d5ceacd793bcca2e0f081d495f0b1d96`.
Normalized-reproducible ABI 53 wheel SHA-256 values are
`da1fcbdee1c0e1bfac900f3d054851942c792b5fd5958d2f15405c1c49d4b41c`
and
`0e51fcf2a4996266d7c19b070b2515fea5f335c00ec3414150b30c6b9f52fad0`.
Installed units pass 103/103, strict Rust/Python gates and the 108-package
dependency proof pass, and fresh 6,526/6,526 collection has SHA-256
`66affc14f9da9eb21d94b750855572592ea92d1f0b039fbf6f18cdaeb01687b7`.
The other 5,862 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-284 advances to ABI 52 and pins both complete unchanged
`test_page_network_response.py` files, with async/sync source SHA-256 values
`707b20a48b1b89529228697d5a87cb42a965a8afb9a485a5c951839c67dc2cba`
and
`82e3f5cfd0581f92085d1b56c06936fb890627dd505189b4894e36d7949e39fd`.
The four Chromium rows pass in final installed candidate and independent
oracle through the same dedicated CDP Chrome; report SHA-256 values are
`98a5f800bdeb271affa5947bd91722e615129835d384ac5cb19e0ae81fe52b00`
and
`807cc17edba597a72a058041ce73de864942c36dbbc2def5179f2c5445fafa06`.
Eight Firefox/WebKit rows remain explicit provider gaps. The cumulative
candidate is 655 passed, seven skipped and 5,864 not run, SHA-256
`82dad9ef03972b03a4174dad73b07409b59776839d2e9885a01e81fb35f6f0c7`;
the cumulative oracle is 636 passed, seven skipped and 5,883 not run,
SHA-256
`f05f91ebfaafaa1766eef99bda1e2123a87684ad544ce20c715bc5fd6d76b6e7`.
Normalized-reproducible ABI 52 wheel SHA-256 values are
`6eec71b2bcc8cc2fe658a0e28a5adab537370e74f20ddb62928ec521d23b102c`
and
`bd406d7ea915d9f95cb3d79c7ef13433ae8ef632c6b77d34059a82268b94b3aa`.
Installed units pass 98/98, targeted Rust tests pass 12/12, strict gates and
the 108-package dependency proof pass, and fresh 6,526/6,526 collection has
SHA-256
`73608b054e7dd88df4ca79accebffa6f69561faa30a0868da4d18f5e014c2c2f`.
The other 5,864 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-283 advances to ABI 51 and pins the complete unchanged shared
`test_signals.py` file, SHA-256
`7f3d9cfc43ca8c5431df25b25ef75f1b38b0f9bac881e893384f622a3dc73267`.
The async and sync Chromium rows pass in final installed candidate and
independent oracle through the same dedicated CDP Chrome; report SHA-256
values are
`15327d9f5ab947fd6c9090f7f33c1be479c55355c6ab317e2cbd9985dae6f91e`
and
`a2b8d0ba5c8f9f3ebc78bcf4293042578af96805a4145305d008a0f7aa01300a`.
Four Firefox/WebKit rows remain explicit provider gaps. The cumulative
candidate is 651 passed, seven skipped and 5,868 not run, SHA-256
`b64fcdc19f027fd37718a339b122ecb6ea240890e12b4c56c147baac4f3ae0e4`;
the cumulative oracle is 632 passed, seven skipped and 5,887 not run,
SHA-256
`48dd00b23f5df8ad5fb26bf3b4a74dc4ac57c55f9b5ac85f54ff38b69c7789e8`.
Normalized-reproducible ABI 51 wheel SHA-256 values are
`39bf27fbf7d6bdd9d6c1513f11b6c0b37ba2928b2a70c7f921f67593dd3dbfe1`
and
`54b6323011e22a19f6b6beceba01fab317fc1bfbc11869f73f305e88acc1f0f0`.
Installed units pass 96/96, strict gates and the 108-package dependency proof
pass, and fresh 6,526/6,526 collection has SHA-256
`d2b169846ada9162934a60b9e542e99f55096f43fa5a1a2d988cf2d935e372c8`.
The other 5,868 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-282 keeps ABI 50 and pins the complete unchanged shared
`test_events.py` file, SHA-256
`34a3a26326764909a9fd0e1b97282ea5ca689c4a8c45f142725d10c42ee519ce`.
The initial candidate runs the response handler but fails because its
`Exception("fail")` remains log-only. A one-error-bounded sync bridge slot now
raises the first pending listener exception after the public call completes.
Final installed candidate and independent oracle pass the Chromium row;
report SHA-256 values are
`b0cb31378655ab7131bda8d57255e28b8378f2ace1275933e61230aa173c4a42`
and
`940665a60c4023e1f2e05a32fa3045339f6052406ddc71df1a7f88b2ffab4fef`.
Firefox and WebKit remain explicit provider gaps. The cumulative candidate is
649 passed, seven skipped and 5,870 not run, SHA-256
`3e75c9588a73cd741ac571800a430e290affb178aee3bed5ab0951b46987602d`;
the cumulative oracle is 630 passed, seven skipped and 5,889 not run,
SHA-256
`c895df37d1112f67e24f5e31b8d7bb2a13982261e792b3d45c966faf65e05b6e`.
Normalized-reproducible ABI 50 wheel SHA-256 values are
`d70c33a033f01ff57dddcf6cee6bca90b31203e62e9a67cf6877b669202a051c`
and
`5e09beec25d3982f14a70550475fb0bb6f0c6bcf50c6aeba92e364181678ea7d`.
Installed units pass 94/94, strict gates and the 108-package dependency proof
pass, and fresh collection remains 6,526/6,526. The other 5,870 Python rows
and all 48,477 Core JS/TS invocations remain open.

NODE-281 keeps ABI 50 and pins the complete unchanged shared
`test_threads.py` file, SHA-256
`8e42fa88bfa47b5f6ba501c2731d3d9c1812fddb28b4451aba923fb93bfe8c64`.
Final installed candidate and independent oracle pass the Chromium worker-
thread row without product changes; report SHA-256 values are
`1fdf1cf683d7ca5bba9f12db51f81bf2fb083d1d230520a25117f43c39ab254a`
and
`4b10bc1a7187c08b13e25a639cbd33baa45306edc04f0c6916913ddc3c48de59`.
Firefox and WebKit remain explicit provider-gap rows. The cumulative candidate
ledger is 648 passed, seven skipped and 5,871 not run, SHA-256
`4febdfe050081de20ef509f153f4e77669eb2672d22be1a07853d9e2e1916345`.
The cumulative oracle is 629 passed, seven skipped and 5,890 not run,
SHA-256
`d9fb4a9faf432e4cc60c473b373ac9076afb98eb0dd4d9ee3ed62beb0d14b06e`.
The unchanged normalized-reproducible ABI 50 wheel hashes remain
`0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
and
`db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
Fresh collection remains 6,526/6,526. The other 5,871 Python rows and all
48,477 Core JS/TS invocations remain open.

NODE-280 keeps ABI 50 and pins the complete unchanged async
`test_selector_generator.py` file, SHA-256
`60ae2e5324fd30a820402d4feaa28dac2243caaef227a28ad9b1773e3d403b8d`.
Final installed candidate and independent oracle pass the Chromium row;
report SHA-256 values are
`c38cfd4ee833a79dd43ae44e6574ea15d3cf10c3321846c088bdcc570f597537`
and
`881bcdbf424fab522b7eccd9ba0a4c9555b419b209efe29e025a659d6c1b6c2f`.
Firefox and WebKit remain explicit provider-gap rows. A 1,024-byte-bounded
Rust setting drives immutable semantic test-id snapshots and current-setting
strict diagnostics; each strict error publishes at most five bounded DOM
previews. The cumulative candidate ledger is 647 passed, seven skipped and
5,872 not run, SHA-256
`38ab5abac559594a16e9faedab66525f7a168e4bddcf765da1b294a30139a10d`.
The cumulative oracle is 628 passed, seven skipped and 5,891 not run,
SHA-256
`27c233a777873c1b97d4cb503768603de325c23c23c0d699b322bd130575a6a6`.
Normalized-reproducible wheel SHA-256 values are
`0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
and
`db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
Installed units pass 93/93, strict gates and the 108-package dependency proof
pass, and fresh collection remains 6,526/6,526. The other 5,872 Python rows
and all 48,477 Core JS/TS invocations remain open.

NODE-279 keeps ABI 50 and pins the complete unchanged async
`test_selectors_text.py` file, SHA-256
`d8b841099bddcb972bf81846f050cac8880f65fe5c6f73b1c44a26d556d39332`.
Final installed candidate and independent oracle pass both Chromium rows;
report SHA-256 values are
`214264e2a9babd5277c448a82d86ba11ace901635120cdd047eb4437a76595c3`
and
`d3204cf9be77ba876630627f7f9a81f88017bde7f54f64ebbe7b2f3f58828efa`.
Four Firefox/WebKit rows remain explicit provider gaps. Selector protocol 1.1
and Locator protocols 1.24/2.7 carry typed immediate legacy-text semantics;
older 1.23/2.6 peers reject the new variant. The cumulative candidate ledger
is 646 passed, seven skipped and 5,873 not run, SHA-256
`603945f33ceadde881b175508db8402092fe71f246b3a025ea2b9277227fde25`.
The cumulative oracle is 627 passed, seven skipped and 5,892 not run, SHA-256
`d19ab2d5f567fd275b46cb93e37b07dd631db0175345208fe3365fd0a6c2e98a`.
Normalized-reproducible wheel SHA-256 values are
`8c2739a837b625d8a8da4f394752dbf6e40f7d39a62a2b319baf3c1f9a063f85`
and
`45266c5d9c8ad7e2f3c4552cacb460365640447cd6131777030d83d25c134e04`.
Installed units pass 92/92, strict gates and the 108-package dependency proof
pass, and fresh collection remains 6,526/6,526. The other 5,873 Python rows
and all 48,477 Core JS/TS invocations remain open.

NODE-278 advances to ABI 50 with Page, Locator and exact retained ElementHandle
screenshot facades. Python validates and forwards path/type/mask values; Rust
owns capture admission, output leases, exact native identity, mask setup and
cleanup, document-space clip translation and Chromium
`captureBeyondViewport` selection. One capture admits at most 64 mask Locators
and 256 elements per Locator.

The complete unchanged async screenshot file expands to nine inventory rows.
The ABI 49 candidate diagnostic fails all three Chromium rows at the missing
Page method, SHA-256
`f38c3a716fae71f63efcc81352fdf4e3805a14e03a3e42ca228f3f89b44e0b59`.
Final installed ABI 50 candidate and independent oracle each pass 3/3, report
SHA-256 values
`ad7e0c2b043641fa39a4fcac775945a193c4fef63a507f17a4572150bd270c58`
and
`d3f45897a07e19a2ac3aa2ac7a737bbaf67d1b3088bb12b8d819632142ca11e4`.
The six Firefox/WebKit expansions remain explicit provider-gap `not-run`
outcomes. The cumulative candidate ledger is 644 passed, seven skipped and
5,875 not run, SHA-256
`f898f89d210cfb1c73d37b5817b68688050fae9f273c54c92480f4e0375aaf5c`.
The cumulative oracle ledger is 625 passed, seven skipped and 5,894 not run,
SHA-256
`aa53c89869c367c05976854317d9d0e50964b6cc7b0617602eba54af5a4b84b8`.
Two normalized-reproducible ABI 50 wheel SHA-256 values are
`21017fae60321b12ab56bcc1b000e01e18c8892caa48e085a5622fa84a1da9ce`
and
`6d8369d2bb334fb28f650f244172a269bb2f0f694e14c42c5f4b7f06cbc3dfd7`.
Installed units pass 88/88; strict Rust/Python gates, the dependency proof and
fresh 6,526/6,526 candidate collection pass. The other 5,875 Python rows and
all 48,477 Core JS/TS invocations remain open.

NODE-277 advances to ABI 49 by extending the existing Rust-owned `page-pdf`
owner with typed `tagged` and `outline` booleans. Python strictly validates
and forwards those values; Rust maps them to Chromium's `generateTaggedPDF`
and `generateDocumentOutline` fields while preserving the bounded stream,
close and retained-output ownership from ABI 48.

The complete unchanged async PDF file expands to nine inventory rows. The ABI
48 diagnostic passes capture/save but fails the outline row at the explicit
unsupported-option boundary, SHA-256
`59e0bff3910ee10e21c2d61a11f6ddd6bc5fb04a5f88f7e63defba159848b33b`.
Final installed ABI 49 candidate and independent oracle each pass all three
applicable Chromium rows, report SHA-256 values
`248379bc35eb7c2b78e6e3293ec7a7b9ada3a2f4327883765880e03153ea3355`
and
`647a7ca744aa2a3899d3f3756d1b7c5df38a213faf4f4e052b4b99948d3a6077`.
The six Firefox/WebKit expansions remain explicit upstream Chromium-only
non-applicable `not-run` outcomes. The cumulative candidate ledger is 641
passed, seven skipped and 5,878 not run, SHA-256
`311867644fb8f62e11af82b1cdb08ce58392f1a4dfe2a03bc0bd7cc1ff387cd8`.
The cumulative oracle ledger is 622 passed, seven skipped and 5,897 not run,
SHA-256
`895826486a6d1831cbabaf2eef041d822ab520836232682b680d69d1df01ccfe`.
Two normalized-reproducible ABI 49 wheel SHA-256 values are
`e837346a03487651337816b4adb8e93c6f48860b94252be8d00c8a33088d7e34`
and
`68ecf94ec7845f836f0b09dd5f8dffd8fab64ed0e64cbd49029f77fcba9cec28`.
Installed units pass 86/86; the all-feature host passes 883 tests plus 26
explicit live/diagnostic ignores and the addon passes 2/2. Strict Rust/Python
gates and the 108-package dependency proof pass. Fresh candidate collection
remains 6,526/6,526, SHA-256
`bad14cc3e732be2f400aa34ce4a27f2b447b2f121193ccd3294da3a13c2a0007`.
The other 5,878 Python rows and all 48,477 Core JS/TS invocations remain open.

NODE-276 advances to ABI 48 with a separately selectable Rust-owned
`page-pdf` capture/output owner. Rust performs exact Page/session lookup,
finite per-Page capture admission, default `Page.printToPDF`
`ReturnAsStream` dispatch, a bounded `IO.read` loop, raw/base64 chunk decoding,
mandatory `IO.close`, a 2 MiB per-document bound and optional path persistence.
Python rejects uninstalled keyword options and forwards only Page/path values.

The complete unchanged sync PDF file expands to six inventory rows. Candidate
and independent oracle each pass both applicable Chromium rows; the two
Firefox and two WebKit rows remain explicit upstream Chromium-only
non-applicable `not-run` outcomes. Candidate/oracle report SHA-256 values are
`c36fb38329822687850d1900d4b24a4b10361d423011ea8efea3c9adf619ac3e`
and
`036bffd3a26facd620b478f49ec81f98a70f01274cd51c1d925d370e1975d908`.
The cumulative candidate ledger is 638 passed, seven skipped and 5,881 not
run, SHA-256
`801181534ae92df917d69d02f0d03708981c14ab8a171fefa7086e12acae3b02`.
The cumulative oracle ledger is 619 passed, seven skipped and 5,900 not run,
SHA-256
`a9f7b78946734baef302d23a8cada3da5d3d707865957a41fcdb1e3041828a01`.
Two normalized-reproducible ABI 48 wheel SHA-256 values are
`48593cfdd4a22249e2eee14186faceb95235670657e52aa808680d6e9a2fe348`
and
`b9acf998197b093ec2597c996193e6b3256fc97dfa23e73f3299de85c59bd2af`.
Installed units pass 85/85; the all-feature host passes 882 tests plus 26
explicit ignores; strict Rust/Python gates, the dependency proof and fresh
6,526/6,526 candidate collection pass. The other 5,881 Python rows and all
48,477 Core JS/TS invocations remain open.

## Verification

```bash
cd ../..
scripts/check-python-addon-dependencies.sh
CARGO_INCREMENTAL=0 cargo check --release --locked -p pwright-python-addon
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-python-addon --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-node-host --no-default-features --features console-history \
  --lib console_events::tests
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-node-host --no-default-features --features console-history \
  --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-python-addon --no-default-features --features console-history \
  --all-targets --no-deps -- -D warnings
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-node-host --no-default-features --features request-history \
  --lib network_events::tests
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-node-host --no-default-features --features request-history \
  --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-python-addon --no-default-features --features request-history \
  --all-targets --no-deps -- -D warnings
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-node-host --no-default-features --features response-events \
  --lib network_events::tests
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-node-host --no-default-features --features response-events \
  --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-python-addon --no-default-features --features response-events \
  --all-targets --no-deps -- -D warnings
CARGO_INCREMENTAL=0 cargo test --release --locked \
  -p pwright-node-host --no-default-features --features device-emulation \
  --lib device_emulation::tests
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-node-host --no-default-features --features device-emulation \
  --all-targets -- -D warnings
CARGO_INCREMENTAL=0 cargo clippy --release --locked \
  -p pwright-python-addon --no-default-features --features device-emulation \
  --all-targets --no-deps -- -D warnings
CARGO_INCREMENTAL=0 cargo test --release --locked -p pwright-node-host --lib \
  ffi_budget::tests::synchronous_reads_stay_charged_through_conversion -- --exact

cd packages/pwright-python
uv run --no-project --with ruff==0.13.2 ruff format --check \
  python tests ../../scripts/check_playwright_python_official_source.py \
  ../../scripts/run_playwright_python_official_conformance.py \
  ../../scripts/select_playwright_python_junit.py
uv run --no-project --with ruff==0.13.2 ruff check \
  python tests ../../scripts/check_playwright_python_official_source.py \
  ../../scripts/run_playwright_python_official_conformance.py \
  ../../scripts/select_playwright_python_junit.py
uv run --no-project --with pyright==1.1.405 \
  --with pytest==8.4.2 --with pytest-asyncio==1.2.0 \
  --with playwright==1.58.0 python -m pyright \
  python tests ../../scripts/check_playwright_python_official_source.py \
  ../../scripts/run_playwright_python_official_conformance.py \
  ../../scripts/select_playwright_python_junit.py
```

Build two repaired wheels into separate directories and compare them with
`scripts/check_python_wheel_reproducibility.py`. It requires every payload to
match and normalizes only CycloneDX's per-build UUID/timestamp and the
corresponding RECORD hash. `--auditwheel repair` vendors the selected OpenSSL
libraries and rewrites the native extension to load them relative to the wheel;
an unrepaired local wheel is not a distributable artifact.

Real-browser tests require the dedicated loopback CDP Chrome and a wheel
installed outside the source tree. The navigation rows also require a
task-owned marker tab created with `pwright open`; set its exact URL in
`PWRIGHT_TEST_PAGE_URL`, then close that tab after the run:

```bash
chrome-cdp start
chrome-cdp status
PWRIGHT_BROWSER_WORK=$(mktemp -d /tmp/pwright-python-browser.XXXXXX)
cd "$PWRIGHT_BROWSER_WORK"
PWRIGHT_TEST_PAGE_URL='data:text/html,<title>pwright-python-owned-marker-20260922</title><body>owned</body>'
pwright --cdp http://127.0.0.1:9222 open "$PWRIGHT_TEST_PAGE_URL"
# Set this to the `Tab:` identifier printed by the preceding command.
PWRIGHT_TEST_TAB_ID=tab_00000000
PWRIGHT_CDP_ENDPOINT=http://127.0.0.1:9222 \
  PWRIGHT_TEST_PAGE_URL="$PWRIGHT_TEST_PAGE_URL" python -m pytest -q \
  tests/differential/test_attach.py tests/differential/test_cleanup.py
pwright --cdp http://127.0.0.1:9222 tab-close "$PWRIGHT_TEST_TAB_ID"
```

The official-source runner selects the full verified
`microsoft/playwright-python` v1.58.0 corpus by default: 6,526 expanded pytest
node IDs from 156 non-empty files, plus one explicitly recorded zero-byte
placeholder. Oracle and candidate use separate environments:

```bash
uv run --no-project python scripts/check_playwright_python_official_source.py \
  /Users/cyuan/projects/thirdparty/playwright-python
<venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  oracle /Users/cyuan/projects/thirdparty/playwright-python --collect-only
<venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python --collect-only

# Run each half of the fixed client-certificate selection in a fresh process.
<venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --manifest packages/pwright-python/tests/official/client-certificates-manifest.json \
  --manifest-cases --manifest-api async --result <async-report.xml>
<venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --manifest packages/pwright-python/tests/official/client-certificates-manifest.json \
  --manifest-cases --manifest-api sync --result <sync-report.xml>

# Bound and isolate every expanded selected row; resume validates saved XML.
<venv>/bin/python scripts/run_playwright_python_official_conformance.py \
  candidate /Users/cyuan/projects/thirdparty/playwright-python \
  --browser chromium --case-timeout 45 --isolate-cases \
  --test tests/async/test_page.py --result <isolated-page-report.xml>
# Repeat the same command with --resume-isolated after an interruption.
```

The full oracle and candidate collections are green at 6,526/6,526. Candidate
collection uses explicit test-only annotation placeholders; it does not claim
that those public capabilities are implemented. The historical pre-Locator
baseline covered 451 inventory records: the complete async/sync
global API-request files pass 231/231, the async browser file is
4/4, the sync browser file is 1/1, the async base-URL file is 7/7, the URL-wait
file is 9/9, and the complete isolated async Chromium Page file is 124 passed /
one upstream FIXME skip in both candidate and oracle with zero status
differences. The fresh full-file run supersedes the earlier merged fixed-row
report covering tag, binding, lifecycle, Page-error, Page-route, Request-size,
drag and dialog/beforeunload rows, and
the browser-independent stop/cancel lifecycle rows pass 6/6. The fixed official
client-certificate selection adds 12 expanded invocations in each lane: eight
pass and four are the unchanged upstream macOS WebKit skips. The complete
async/sync init-script files add 18/18 candidate passes. Fixed tag and JSHandle
manifests pass 18/18 and 7/7; Page bindings pass 12/12, Page lifecycle events
pass 6/6, Page errors pass 5/5, the fixed async Page-route slice passes 5/5 and
the fixed Request-size slice passes 2/2. The fixed drag selection passes 6/6
async plus 2/2 sync. The fixed dialog selection passes 10/10 async plus 1/1
sync. The Page-close terminal-waiter row, persistent-Context row and both
headless Page-pause rows pass in candidate and oracle, producing the 124 passed
/ one upstream skip Page report. ABI 21 additionally passes the unchanged
relative-base-URL route/fulfill/body row in both candidate and oracle lanes.
The complete async/sync Chromium request-GC files add 2/2 in both lanes. The
async/sync Page-error-history rows add 2/2, and the async/sync
console-message-history rows add another 2/2 in both lanes, the async/sync
request-history rows add 2/2, and the complete device file adds one pass plus
two upstream browser skips in both lanes. The applicable async and sync
listener Chromium rows add 2/2 in candidate and oracle; their Firefox/WebKit
rows remain explicit provider gaps. The applicable sync/async collection-
selector rows and async duplicate-registration row add 3/3 in candidate and
oracle; their Firefox/WebKit rows remain explicit provider gaps.
The two async/sync registration-error rows add 2/2 in candidate and oracle.
The six async/sync custom-engine base, path and main/isolated-world Chromium
rows add 6/6 in candidate and oracle.
The two async/sync layout-selector Chromium rows add 2/2 in candidate and
oracle. The 34 basic async/sync Locator rows add 34/34, and the ten Locator
click/evaluate/count rows add 10/10 in candidate and oracle.
The three async-only Locator description rows add 3/3 in candidate and oracle.
The typed Locator composition rows add 4/4 async plus 2/2 sync in candidate
and oracle. NODE-258 through NODE-277 then finish both complete Locator files,
the semantic-Locator/selector, fill and context-manager files and the two
applicable HTTPS-error rows plus the sync and async PDF files. The current
cumulative candidate ledger is 641 passed, seven skipped, and 5,878 `not-run`,
SHA-256
`311867644fb8f62e11af82b1cdb08ce58392f1a4dfe2a03bc0bd7cc1ff387cd8`;
this is not a full-suite pass. Independent global-request oracle lanes pass
138/138 async and 93/93 sync; the same lifecycle rows pass 6/6 oracle, and its
client-certificate selection is also eight passed plus four upstream skips;
the init-script files add 18/18 oracle passes, and the same tag/JSHandle
manifests pass 25/25, Page bindings pass 12/12, Page lifecycle events pass 6/6
and Page errors pass 5/5; the same Page-route slice passes 5/5 and the same
Request-size slice passes 2/2, the drag selection passes 8/8, the dialog
selection passes 11/11, the terminal waiter and persistent rows pass 1/1 each,
and Page pause passes 2/2. The async and sync Page request-GC files add 2/2,
the Page-error-history rows add 2/2, the console-message-history rows add 2/2,
the request-history rows add 2/2, the complete device file adds one pass plus
two upstream browser skips, the async/sync listener Chromium rows add 2/2, and
the three earlier selector rows add 3/3. The six custom-engine rows add 6/6.
The two layout-selector rows add 2/2, the basic Locator slice adds 34/34, the
Locator execution/count slice adds 10/10, the description slice adds 3/3 and
typed composition adds 6/6. The later complete-file oracle lanes match their
candidate selections.
The cumulative oracle ledger is 619 passed, seven skipped, and 5,900
not run, SHA-256
`a9f7b78946734baef302d23a8cada3da5d3d707865957a41fcdb1e3041828a01`.
Current routing claims cover async/sync Page and BrowserContext bounded string
patterns, finite positive `times` budgets, handler-specific unroute,
continue/abort/fallback and bounded status/header/body fulfillment plus
URL/method/header/POST-data continuation or fallback overrides. Regex or
predicate matchers, JSON/path/response fulfillment and other route overloads
remain unsupported. The
historical seven-case browser lane is still 7/7 in both modes but now
requires explicit `--bootstrap-only`; it is not full-suite evidence. Its server
prefix must be the verified full Tailscale MagicDNS URL of the `serve-lib` route
for the pinned upstream `tests/assets` directory.

The oracle is PyPI `playwright==1.58.0` (PyPI has no `1.58.2`; that separate
version remains the repository's npm oracle). The 2026-09-22 base checkpoint
passed async and sync parity plus cleanup/cancellation gates. Twenty warm
attach/enumerate/detach cycles increased Python-process maximum RSS by
1,310,720 bytes from a 78,741,504-byte baseline; browser memory was excluded.
