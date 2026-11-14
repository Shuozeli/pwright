<!-- agent-updated: 2026-10-21T03:43:07Z -->

# Full upstream Playwright conformance

Status: inventory and collection baseline established; candidate execution is
not complete. The historical seven-case Python lane is only a bootstrap check
and cannot satisfy this conformance target.

The complete pinned Chromium `frame-hierarchy.spec.ts` file has 13 cases,
verified by a 4/4 source/ID/asset manifest checker. Its unchanged full-file
source-bound dedicated-CDP [candidate](../.work/official-conformance/results/core-frame-hierarchy13-rust-owner-candidate-20261021-v1.json)
passes 8/13, versus [Oracle](../.work/official-conformance/results/core-frame-hierarchy13-oracle-20261021-v1.json)
13/13; both provenance sidecars independently verify. Four candidate REDs
have a confirmed missing default-off `frame-events` exposure in this frozen
profile. The fifth fails at `page.goto` before frame-event assertions and also
reports a network Request frame absent from topology during teardown; its
underlying ordering/close cause is not yet proven. This is raw evidence, not
parity. The independently replayed immutable
[v22 paired receipt](../.work/official-conformance/outcomes/core-frame-hierarchy13-rust-owner-strict-paired-v22/promotion-receipt.json)
(SHA-256 `535148fa2b02b5af9e5519604cebef296f6e7daa1ff3c43dcba79139f933270a`)
updates only 13 rows per lane, preserves 48,464 other raw rows per lane,
retains the inherited v21 instability, and passes 48 verifier/mutation tests.
Current Core totals are candidate 4,674 expected / 87 unexpected / 21 skipped
/ 43,695 not-run; Oracle 4,452 / 11 / 20 / 43,994.
The separate [v22 reconciliation audit](../.work/official-conformance/audits/core-v22-reconciliation-v1.json)
(SHA-256 `7255a5171bad22ca14f7dd9097cec86646cef4849d3d447760807d25e2f16641`)
joins all 48,477 pinned IDs per lane with independently reviewed invocation
classes without changing either strict ledger. In **each** lane, 2,610
`expected` rows have `evidenceScope=upstream-private-source-only` and
`candidateRuntimeRequired=false`: 2,581 reviewed implementation-internal,
ten product-infrastructure, nine plain public API, and ten public API with a
private test helper. These are source-accounted rows, **not Pwright runtime
passes**; the 19 reviewed-public IDs remain applicable but runtime-unproven
in both lanes. Thus even the candidate's 4,674 `expected` rows cannot be read
as 4,674 executed passes. The other 2,064 `expected` rows are merely
non-source-only by this audit, not automatically independently proven runtime
passes. The standalone audit pins the frozen input hashes and passes eight
focused tests; it grants no promotion, exclusion, or parity credit.

A new default-off
`frame-events` bridge, separate `frame-detach-events`, and Rust-owned
main-world-only `frameElement` readiness path pass Node 225/225, host release
598 passed / eight ignored, and three focused composition release tests. The
first frozen combined profile omitted `frame-element`: it passed the full
[hierarchy file 13/13](../.work/official-conformance/results/core-frame-hierarchy13-frame-events-main-only-candidate-20261021-v2.json)
but [failed frameElement 0/6](../.work/official-conformance/results/core-frame-element6-frame-events-main-only-candidate-20261021-v2.json)
at the capability gate. The corrected frozen 41-feature addon (SHA-256
`1681505babfcf49f0e62233537f471af911fd73c247cef284ced648543c8c332`)
ran both complete files unchanged: hierarchy [10/13 first](../.work/official-conformance/results/core-frame-hierarchy13-frame-events-combined-candidate-20261021-v3a.json)
and [12/13 second](../.work/official-conformance/results/core-frame-hierarchy13-frame-events-combined-candidate-20261021-v3b.json),
frameElement [5/6 first](../.work/official-conformance/results/core-frame-element6-frame-events-combined-candidate-20261021-v3a.json)
and [6/6 second](../.work/official-conformance/results/core-frame-element6-frame-events-combined-candidate-20261021-v3b.json).
All six new raw provenance sidecars independently verify. Four focused release
characterization tests now prove that the independent network cursor can see
an available child Frame before topology attaches it, or publish its Request
and terminal after topology removes it. The current host's live-topology lookup
can fail in either ordering; the exact ordering/effect behind each browser
`TargetClosed` remains untraced. A narrow Node facade fix now retains an
already-known canonical Frame wrapper on the Request after detach, without
new `Page.frames()` enumeration at construction or notice adoption. Its
regression and focused package tests pass 87/87 with no TODO. Network events
that precede topology publication remain separate and unresolved; no browser
parity claim follows. The v22 strict
ledger is not superseded and no stable frame parity is claimed.

The complete 11-case Core Chromium Library
[`browsercontext-page-event.spec.ts` manifest](../tests/official/playwright-core-browsercontext-page-event-public-chromium-v1-manifest.json)
pins unchanged source, IDs, fixtures, and six assets; its source-only checker
passes 4/4. Two independently verified Oracle runs are each **6/11** in the
dedicated-CDP adapter, with the same five RED IDs. The same frozen page-check
candidate (addon SHA-256
`9be17a4945465b2355f9a1ee3f3ed8996ce6a5cf0b5596535ac1b56dc2e56a93`)
runs [10/11 first](../.work/official-conformance/results/core-browsercontext-page-event11-candidate-page-check-20261021-v1.json)
and [9/11 second](../.work/official-conformance/results/core-browsercontext-page-event11-candidate-page-check-20261021-v2.json),
with both sidecars verified. `should have an opener` times out in both lanes;
the second candidate run also times out on Ctrl-click, so stable parity is
unproved. Oracle's five failures mean its launch-to-CDP fixture does not
provide an all-green reference for this file. The dedicated Chrome CDP
endpoint stopped responding after the second candidate run; its process
remained alive and was not restarted. These four raw runs do not alter v22.

The complete Python
`tests/sync/test_network.py` source is separately
[pinned](../packages/pwright-python/tests/official/sync-network-full-v1-manifest.json):
six functions, 18 browser-expanded IDs, source-only checker 3/3. Its six
Chromium cases ran source-bound: [candidate v2](../.work/official-conformance/results/python-sync-network6-chromium-candidate-20261021-v2.xml)
1/6 versus [Oracle](../.work/official-conformance/results/python-sync-network6-chromium-oracle-20261021-v1.xml)
3/6. The earlier [candidate v1](../.work/official-conformance/results/python-sync-network6-chromium-candidate-20261021-v1.xml)
failed all six at setup due the dedicated CDP Page-lane capacity limit; it is
retained as infrastructure-invalid, not promoted as compatibility failures.
All three provenance sidecars verify. Oracle itself fails the upstream
loopback-only `server_addr` assertion under required Tailscale binding and
two service-worker cases, so they cannot be called candidate-only REDs. The
two security-details tests are candidate-only REDs. The independently replayed
immutable [Python v10 paired receipt](../.work/official-conformance/outcomes/python-sync-network6-chromium-strict-paired-v10/promotion-receipt.json)
(SHA-256 `c696c1cab0976ac45564c54814ebf12199268a904f243672203c1ababd9f7f13`)
updates only six rows per lane, preserves the other 6,520 rows per lane
byte-for-byte, and passes 44 verifier/mutation controls. Current cumulative
Python totals are candidate 1,651 passed / 124 failed / three errors / 37
skipped / 4,711 not-run; Oracle 1,436 / 13 / 37 / 5,040. The archived
candidate/harness snapshot does not include the hash-verified external Oracle
package or upstream checkout. The remaining twelve expanded IDs are unrun. A separate
[two-case Oracle-only HTTPS manifest](../packages/pwright-python/tests/official/sync-network-service-worker-secure-chromium-oracle-v1-manifest.json)
pins the same upstream Service Worker cases, fixtures, and assets. Under a
trusted Tailscale HTTPS fixture, both unchanged Oracle cases
[pass](../.work/official-conformance/results/python-sync-network-sw2-secure-chromium-oracle-20261021-v1.xml)
(2/2); its source-bound provenance independently verifies (report SHA-256
`ad2f8f651c2c29b3e500638d538a63788b9046fc6b6900da3f2553c73c08ffca`).
A separate source-pinned [paired HTTPS diagnostic](../packages/pwright-python/tests/official/sync-network-service-worker-secure-chromium-paired-v1-manifest.json)
then ran both lanes: [candidate](../.work/official-conformance/results/python-sync-network-sw2-secure-paired-candidate-20261021-v1.xml)
1/2 and [Oracle](../.work/official-conformance/results/python-sync-network-sw2-secure-paired-oracle-20261021-v1.xml)
2/2, with independently verified provenance. The candidate now passes the
response-from-Service-Worker case but lacks the BrowserContext `serviceworker`
event for the other. These diagnostics neither revise the original HTTP v10
receipt nor prove HTTPS alone explains every failure.

A separate all-capability Python wheel with the default-off
`navigation-response-finished` source slice (SHA-256
`a71e7a58c3bdc37658343839b3bb0fe21222b444b271553c7c92d2dda27a1f22`)
ran the same complete six Chromium IDs twice from an isolated installation:
[first](../.work/official-conformance/results/python-sync-network6-nav-finished-candidate-20261021-v1.xml)
and [second](../.work/official-conformance/results/python-sync-network6-nav-finished-candidate-20261021-v2.xml)
are both 1/6, with independently verified source-bound sidecars. The HTTPS
security-details case passes `Response.finished()` and advances to the absent
`Response.security_details()` API; the HTTP security case also lacks that
method. This interim join requires explicit `request-history`, unlike the
still-open independent bounded-owner design target. It does not change the
strict v10 receipt or claim full response lifecycle compatibility.
The [focused sync/async security manifest](../packages/pwright-python/tests/official/response-security-chromium-sync-async-v1-manifest.json)
pins four unchanged Chromium IDs from both upstream network files; its
source/inventory checker passes 3/3. It has no new browser outcome yet and
cannot by itself replace complete-file evidence.
The independent default-off `response-security-details` source slice now
retains five bounded TLS fields and exact navigation response identity.
An isolated installed all-capability wheel (SHA-256
`0c09817458bc9329c3bbc7c00153c635ae913d3a58567746a2666ed24fa2d2cc`)
passes 300/300 browserless Python contracts; release adapter 30/30, host
2/2, and facade 5/5 checks pass. The full Python addon dependency/release
feature matrix and independent security-only closure/compile proof pass.
Official security browser runs on this
wheel are still pending CDP recovery, so v10 is unchanged.

The default-off Rust/Node `light-selectors` profile is a narrow candidate
improvement, not general selector parity. The unchanged complete 21-case
Chromium `selectors-misc.spec.ts` file now has a fresh source-bound
[candidate result](../.work/official-conformance/results/core-selectors-misc21-light-candidate-20261021-v1.json)
of 11/21, compared with the frozen 10/21 candidate and unchanged 21/21
Oracle. Only the upstream `should work for open shadow roots` ID changed from
RED to passing; its `id:light` and `data-testid:light` page.$/$$ assertions
pass. The other 20 IDs have no outcome change. Ten candidate-only REDs remain.
The addon is isolated in the `official-conformance-page-check-light-v1`
profile; the default and former page-check profile do not enable it. The
later live Rust opt-in source adds the two other upstream light attribute
aliases with release selector tests 92/92 enabled and 87/87 default-off, but
the v20 addon snapshot does not contain them and no official browser rerun
has tested them. Locator/ElementHandle/evaluation operations remain
unsupported, and large browser-side `querySelectorAll`
materialization is not yet resource-proved. The independently replayed
immutable [v20 candidate-only receipt](../.work/official-conformance/outcomes/core-selectors-misc21-light-strict-paired-v20/promotion-receipt.json)
(SHA-256 `2889851a3db6106bb726018ade7b8f2f96e72852496d40123ea825a80f3e5bd9`)
changes exactly one candidate row, preserving the other 48,476 candidate
rows, all 48,477 Oracle rows, and the Oracle summary byte-for-byte. Its 39
verifier/mutation tests pass. At v20, Core totals were candidate 4,660 expected
/ 88 unexpected / 21 skipped / 43,708 not-run and Oracle 4,439 / 11 / 20 /
44,007. This remains a RED, adapted-CDP corpus.

The complete pinned Chromium `frame-frame-element.spec.ts` has six
public-behavior cases; its source/ID/asset checker passes 3/3. All six ran
unchanged in each source-bound dedicated-CDP lane with no skips, retries,
flakiness, or top-level errors: [candidate](../.work/official-conformance/results/core-frame-frame-element6-candidate-20261021-v1.json)
0/6, [Oracle](../.work/official-conformance/results/core-frame-frame-element6-oracle-20261021-v1.json)
6/6. Both provenance sidecars independently verify. Every candidate case
first fails because the JS/TS `Frame.frameElement()` API is absent; the later
identity, detached-frame, frameset, and shadow-root semantics remain untested
in candidate. The Rust implementation must resolve the owner node through the
parent CDP session and publish a bounded parent-world ElementHandle, including
closed-shadow cases. The independently verified immutable
[v19 paired receipt](../.work/official-conformance/outcomes/core-frame-frame-element6-strict-paired-v19/promotion-receipt.json)
(SHA-256 `9303f3e34a31f15b92f9f834b734392c88b8589c5628a3a118ba379d3d92ab4b`)
replaces exactly six target rows per lane and preserves 48,471 off-target raw
rows per lane byte-for-byte. At v19, Core totals were candidate 4,659 expected /
89 unexpected / 21 skipped / 43,708 not-run; Oracle 4,439 / 11 / 20 /
44,007. All six candidate-only REDs were explicit at v19.
The new default-off `frame-element` Rust/Node path resolves the owner in the
parent CDP session/main world. Release backend 5/5, composition 4/4,
Node adapter/contract 190/190, native-load, and TS gates pass. Its
source-consistent frozen addon SHA-256 is
`440ec056a5cfceeacbf851c698ee24c33c3db063ad92262c215d996a568db6a0`.
The unchanged six-case candidate [first run](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v1.json)
was 5/6, with frameset failing at `page.goto` before the new API. Three
subsequent complete-file [v2](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v2.json),
[v3](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v3.json),
and [v4](../.work/official-conformance/results/core-frame-frame-element6-rust-owner-candidate-20261021-v4.json)
runs using identical bytes were 6/6. All four source-bound provenance
sidecars independently verify. The first failure remains an observed
intermittent discrepancy; it is not silently discarded or promoted as stable
parity. The independently replayed immutable
[v21 candidate-only receipt](../.work/official-conformance/outcomes/core-frame-frame-element6-rust-owner-strict-paired-v21/promotion-receipt.json)
(SHA-256 `3f717d16e9cdca75e601ecdcea8185c82df9c42a6c4beebb94cb67ea76f5324d`)
requires all four raw reports and provenance sidecars, upgrades only six
candidate rows at five cumulative attempts, and preserves 48,471 other
candidate rows and all 48,477 Oracle rows byte-for-byte. Its 47 verifier/
mutation tests pass. The frameset ID remains explicitly unstable, the root
cause unproven, and `publicApiParityEvidence` false. At v21, Core totals were
candidate 4,666 expected / 82 unexpected / 21 skipped / 43,708 not-run;
Oracle 4,439 / 11 / 20 / 44,007. Next evidence requires bounded internal
navigation-close source tags and a controlled old-context-close/new-page-goto
barrier test; neither tab saturation nor cleanup overlap is established.

The complete Chromium `selectors-misc.spec.ts` file has 21 pinned
public-behavior assertions and a passing two-test manifest/source checker.
All 21 unchanged cases ran once in each source-bound dedicated-CDP lane:
[candidate](../.work/official-conformance/results/core-selectors-misc21-candidate-20261021-v1.json)
10/21 and [Oracle](../.work/official-conformance/results/core-selectors-misc21-oracle-20261021-v1.json)
21/21. Both provenance sidecars independently verify; no case was skipped or
flaky and neither report has a top-level error. Eleven candidate-only REDs
remain, involving several selector engine/operation families. The independently
verified immutable [v18 paired receipt](../.work/official-conformance/outcomes/core-selectors-misc21-strict-paired-v18/promotion-receipt.json)
(SHA-256 `0bcdeef5540013619a32bc6e5eabf9f456b2b7195dd5f05d2b1bb3e17054c261`)
replaces exactly 21 target rows per lane, preserving 48,456 off-target raw
rows per lane byte-for-byte. At v18, Core totals were candidate 4,659 expected /
83 unexpected / 21 skipped / 43,714 not-run; Oracle 4,433 / 11 / 20 /
44,013. All 11 candidate-only REDs remain explicit.
The shared adapted CDP fixture is not full or unadapted parity evidence.

The complete Chromium `selectors-css.spec.ts` file has 27 source/ID-pinned
public-behavior assertions. Its [manifest](../tests/official/playwright-core-selectors-css-public-chromium-v1-manifest.json)
and nine checker/mutation tests pass. All 27 unchanged cases ran once in each
dedicated-CDP lane on Chrome 155: the frozen clean
[candidate](../.work/official-conformance/results/core-selectors-css27-candidate-20261021-v1.json)
passes 14/27, while the
[Oracle](../.work/official-conformance/results/core-selectors-css27-oracle-20261021-v1.json)
passes 27/27. Both provenance sidecars independently verify against the pinned
upstream source and candidate artifact. There are no skipped/flaky cases or
top-level report errors. The independently verified immutable
[v17 paired receipt](../.work/official-conformance/outcomes/core-selectors-css27-strict-paired-v17/promotion-receipt.json)
(SHA-256
`8edfeb4781c19bdc2e9f1eddb5ebf7f9f7863396972a1a69f9c59e0cfd70fd18`)
replaces exactly 27 target rows per lane, preserving 48,450 other raw rows
per lane byte-for-byte. At v17, Core totals were candidate 4,649 expected /
72 unexpected / 21 skipped / 43,735 not-run; Oracle 4,412 / 11 / 20 /
44,034. The 13 candidate-only REDs remain open. Read-only failure triage
separates seven
cross-shadow composed-CSS cases, four scoped/relative-CSS cases, one
capture+XPath composition unsupported gate, and one `$eval` main/utility
world mismatch. The first open-shadow test fails on `css=div div span` at
source line 82; its preceding escaped-attribute assertions pass. No fixture
difference is established as a cause, but the world mismatch remains a
code-path inference until directly traced. This adapted browser execution
does not prove full or unadapted public-API parity.

The default-off Rust `network-raw-headers` value leaf now models the raw-versus-
provisional distinction and an explicit Python-unique versus Node-preserve
duplicate-value reader policy, matching their different pinned upstream
`RawHeaders` implementations. It has input, expanded-entry, retention and
FFI-copy limits. The release model suite passes 21/21 (nine new raw-header
tests), default-off tests pass 12/12, all-feature tests pass 71/71, and
release Clippy passes with warnings denied.
It is not yet connected to the
CDP extra-info hop tracker, public Response/Request owners, or Python/Node
facades; the five official Python network50 header REDs are unchanged.
The read-only adapter audit found request ExtraInfo consumed without a
publication batch, no response ExtraInfo event arm, and a Python addon
`BTreeMap` projection that loses duplicate array entries. The next opt-in
step needs exact per-session/per-request/per-hop pairing, ordered raw updates,
bounded owner and FFI copies, and both Python async/sync readers before an
unchanged official network50 rerun.

Python Chromium 2026-10-21 checkpoint: the default-off ABI131 frame-event
wheel passes the unchanged `window.stop()` navigation case alone, but its
first complete 80-case run is preserved as a [v7 RED successor](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v7/promotion-receipt.json)
(76 pass / two fail / two synthetic errors; receipt SHA-256
`027c37acdfb4326458e96489d306bfa5a5058a6573d5184195f60774611b8944`).
A second complete source-bound run on the same wheel records 78 pass / two
fail / zero errors; the extra target-close/teardown errors did not recur but
are not proven impossible. Its independently verified immutable
[v9 recheck receipt](../.work/official-conformance/outcomes/python-navigation-recheck-chromium-strict-paired-v9/promotion-receipt.json)
(SHA-256
`42738259dba662f23f324451ad094d0f31f2d9137ad01b1e3df14cd599bc4763`)
changes exactly 80 candidate navigation rows after v8 and preserves all
6,446 other candidate and all 6,526 Oracle raw rows. Current Python totals
are candidate 1,650 pass / 119 fail / three errors / 37 skip / 4,717 not-run,
Oracle 1,433 / ten / 37 / 5,046. The pinned 50-case `test_network.py` Chromium
manifest records candidate 35/50 and Oracle 47/50 in the shared HTTP/Tailscale
fixture: 12 candidate-only and three shared failures. The independently
verified [v8 strict paired receipt](../.work/official-conformance/outcomes/python-network50-chromium-strict-paired-v8/promotion-receipt.json)
(SHA-256
`b4acfb11c8af4305e55c9f314c8a55193ed6b930bde1cd43b2c0239ae8f27189`)
changes exactly 50 rows per lane, leaving 6,476 others per lane byte-identical;
historical v8 candidate totals were 1,648 pass / 119 fail / five errors / 37
skip / 4,717 not-run. In a separate trusted
HTTPS diagnostic, the two Service Worker cases pass 2/2 in the Oracle and
1/2 in the candidate; the remaining candidate failure is `Response.body`.
An `about:blank` task anchor was required for official CDP initialization;
two anchorless HTTPS setup-error reports are quarantined. These are adapted
Chromium slices, not complete or unadapted parity. The Python details and
source-bound reports are in the [Python ledger](python-official-conformance.md).
Read-only triage of the twelve candidate-only network50 REDs finds five
Request/Response raw-header fidelity or missing-reader cases, four navigation
Response lifecycle/security-metadata cases, one `Route.fulfill` failure of
unproven cause, one redirect-body error mismatch, and one Page extra-header
validation message already fixed in source but not rerun on an installed
wheel. The shared HTTP `server_addr` RED is fixture-sensitive, yet the
candidate also lacks that method. None is credited green from diagnosis.

Core Chromium `page-dialog.spec.ts` complete 10-case checkpoint (2026-10-21):
the [pinned manifest](../tests/official/playwright-core-page-dialog-public-chromium-v1-manifest.json)
and source/ID checker pass 8/8; all ten unchanged assertions then ran in
each source-bound dedicated-CDP lane. The frozen
[candidate](../.work/official-conformance/results/core-page-dialog10-candidate-20261021-v1.json)
passes 9/10 and the [Oracle](../.work/official-conformance/results/core-page-dialog10-oracle-20261021-v1.json)
10/10. The sole candidate-only RED is the source-line-67 open-alert case:
the candidate times out at `page.evaluate` after 30 seconds, then reports a
target-closed evaluation during cleanup, while the Oracle passes. The
independently verified immutable
[v15 paired receipt](../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v15/promotion-receipt.json)
(SHA-256
`b44fc60ac36d0db4b646b319bf2a430f6240511185aebcc74154857dac1d8110`)
replaces exactly ten target rows per lane and preserves all 48,467 other raw
rows per lane. Cumulative Core totals are candidate 4,634 expected / 60
unexpected / 21 skipped / 43,762 not-run and Oracle 4,385 / 11 / 20 /
44,061. This is adapted Chromium browser evidence, not unadapted public-API
parity or a full-corpus pass. A trace-only build with an upstream-shaped
synchronous non-Promise evaluation path still times out on the unchanged
line-67 case. Two traces place `Page.javascriptDialogOpening` 11 and 21
microseconds after the initial `Runtime.callFunctionOn` reply. The second
records a primitive-string result with no result or exception `objectId`,
followed by a `Runtime.releaseObjectGroup` send at +24 microseconds and no
release reply, driver Fence, or JS evaluation settlement before timeout.
Thus the awaited release is the demonstrated blocker for this corrected
candidate case; whether the old unconditional `await` also contributed to
the historical frozen RED is unproven. A narrow fast path skips group release
only for an inline string with no remote `objectId` or `exceptionDetails`;
retained and exception paths still await cleanup. The clean release addon
(SHA-256
`9be17a4945465b2355f9a1ee3f3ed8996ce6a5cf0b5596535ac1b56dc2e56a93`)
passes 21 Rust and 15 Node tests. Its source-bound unchanged
[candidate rerun](../.work/official-conformance/results/core-page-dialog10-candidate-sync-clean-20261021-v1.json)
passes 10/10, including line 67; provenance verification passes. The
independently verified immutable
[v16 candidate-only successor](../.work/official-conformance/outcomes/core-page-dialog10-strict-paired-v16/promotion-receipt.json)
(SHA-256
`60277dcb60601de2f890c8f3ced2d8475363d0aa9c1fcffab65c90d7e5df5c1f`)
changes only the historical candidate RED row, preserving 48,476 other
candidate raw rows and all Oracle JSONL/summary bytes. At v16, Core totals
were candidate 4,635 expected / 59 unexpected / 21 skipped / 43,762
not-run; Oracle 4,385 / 11 / 20 / 44,061. The v15 counts above are retained
as historical evidence. No new Oracle run or complete-corpus parity is
claimed. The release frame-tree unit gate
passed 80/80, with two separate real-Chrome tests still ignored.
Separate evaluation audit leaves memory and browser-world gates open in the
selected large-result route: cancellation may strand a CDP object group,
large-result admission occurs after the first retained browser object, inline
reply validation can exceed its reservation, and inherited
`c.constructor.prototype` is mutable by page code. An isolated dialog pass
would not close these broader gates.

Core Chromium `page-history.spec.ts` full-file checkpoint (2026-10-20): all 16
unchanged source-pinned assertions ran in each isolated lane using the
`official-conformance-page-check` candidate addon and dedicated CDP Chrome
155. Candidate passed 11/16; the Playwright Oracle passed 13/16. The
independently verified immutable [v14 paired receipt](../.work/official-conformance/outcomes/core-page-history16-strict-paired-v14/promotion-receipt.json)
(SHA-256
`07ce72583a7011139e9aeebc94c2be8c3573e29c619a4ed4107557ed44125b9e`)
replaces exactly 16 target rows per lane and preserves the other 48,461 raw
rows in each lane byte-for-byte. Cumulative Core totals are candidate 4,625
expected / 59 unexpected / 21 skipped / 43,772 not-run and Oracle 4,375 /
11 / 20 / 44,071. Three REDs are candidate-only (renderer-initiated reload,
renderer-initiated goBack, and an extra 404 console event); two are RED in
both lanes (BFCache and renderer-initiated goForward, the latter at different
steps); the `@smoke` goBack case is Oracle-only RED. Upstream Chromium launch
disables the back-forward cache, unlike this attached dedicated Chrome; that
is a plausible explanation for some both-lane timeouts, **not a verified
fix**. The cross-origin Tailscale fixture is narrowly enabled for this pinned
file. The receipt sets `publicApiParityEvidence=false` and
`fullCorpusExecution=false`; neither same-lane RED nor the complete adapted
file establishes unadapted parity.

Latest Core Chromium `Page.fill` full-file checkpoint (2026-10-20): all 45
source-pinned `page-fill.spec.ts` Chromium assertions ran in both isolated
candidate and Playwright Oracle lanes against dedicated CDP Chrome 155. The
earlier candidate RED was 44/45 because a navigation triggered by `fill`
could retire a context before the host's unnecessary post-action Page-binding
publication drain. The default-off `fill-navigation-commit` profile now
composes with the no-binding-source guard; the freshly frozen 68-file candidate
package (manifest SHA-256
`c1501af04093b8d6c197d190915eb6ab039b3ae7c3ffee394f8fcb371f92f037`,
native addon SHA-256
`12c0d93f9e66798023f7380a64fa835e9f5eae6887575d9864f50a74abb97636`)
passes [45/45](../.work/official-conformance/results/core-fill45-candidate-20261020-v2.json),
as does the [Oracle](../.work/official-conformance/results/core-fill45-oracle-20261020-v2.json).
The independently verified immutable
[v13 paired successor](../.work/official-conformance/outcomes/core-page-fill45-strict-paired-v13/promotion-receipt.json)
has receipt SHA-256
`8a10291c3301a8e6e0877387b9d6d279927eba31a1831ef0e3c420353d1c46de`:
only the historical source-line-388 candidate row moves from `unexpected` to
`expected` with two recorded attempts. All 48,476 other candidate rows and
all 48,477 Oracle rows remain byte-identical to v12. Core totals are now
candidate 4,614 expected / 54 unexpected / 21 skipped / 43,788 not-run;
Oracle remains 4,362 / eight / 20 / 44,087. The first Oracle rerun after the
Python shard encountered a browser-context setup failure and was excluded;
the fresh v2 Oracle run passed all 45. The launch fixture uses
`connectOverCDP`, so the receipt records browser execution of this adapted
Chromium slice, not unadapted public-API parity or full-corpus completion.
The global Page-binding guard still has a cross-Page scoping caveat.

Core Chromium `Page.selectOption` full-file checkpoint (2026-10-20):
the pinned upstream v1.58.0
[32-case manifest](../tests/official/playwright-core-page-select-option-public-chromium-v1-manifest.json)
(source SHA-256
`0dc2dd41232ee2ccf7a05e3ce302a50319a736efdc08fb8ec9dd971e2788dbbb`)
ran all 32 unchanged official assertions in each lane against the dedicated
CDP Chrome. The first immutable [v11 paired ledger](../.work/official-conformance/outcomes/core-page-select-option32-strict-paired-v11/promotion-receipt.json)
preserves the candidate's historical 31/32 RED result: navigation triggered
by `selectOption` retired the old execution context, then the Node host
unconditionally evaluated that context to flush Page binding publications
despite having no bindings. The Oracle passed 32/32. A default-off
`select-option-navigation-commit` capability, composed in the
`official-conformance-page-select-option-navigation-commit` profile, plus a
no-publication-source guard avoids that extra evaluation when no Page binding
is registered, pending, or early. The fresh frozen candidate
[v3 result](../.work/official-conformance/results/core-select-option32-candidate-20261020-v3.json)
(SHA-256
`829485d5e7267456f2dc4642325e37de13ab34fdc634cbad07a1069c9e1a57ae`)
passes 32/32; the Oracle [v1 result](../.work/official-conformance/results/core-select-option32-oracle-20261020-v1.json)
(SHA-256
`6ef7eb977b6058cee2792a8d3d5c2b9f21f9c243c01de1c8dfe84758f2851ef8`)
also passes 32/32. The independently verified immutable
[v12 paired successor](../.work/official-conformance/outcomes/core-page-select-option32-strict-paired-v12/promotion-receipt.json)
has receipt SHA-256
`40c861162bbec094b8729927366000ac3b3026586ce98d16e48dcedb1e6c2dc3`:
only that one candidate row changes from `unexpected` to `expected` with two
recorded attempts; 48,476 other candidate rows and all 48,477 Oracle rows
remain byte-identical to v11. The 48,477-row Core totals are now candidate
4,613 expected / 55 unexpected / 21 skipped / 43,788 not-run and Oracle
4,362 expected / eight unexpected / 20 skipped / 44,087 not-run. The
`connectOverCDP` launch fixture was adapted in both lanes; this is
Chromium-only browser-execution evidence, not unadapted public-API parity or
full-corpus completion. The guard is currently global: a binding on an
unrelated Page can still cause the post-navigation drain to evaluate a Page
without bindings. Per-Page scoping and broader browser coverage remain open.

Latest Python Chromium navigation full-file checkpoint (2026-10-20): all 80
source-pinned `tests/async/test_navigation.py` IDs ran again on frozen ABI130-v2
in the isolated candidate lane; the independent Oracle remains 77 passed /
three failed. The candidate records **77 passed / two failed / one synthetic
report error**. The new error is the `window.stop()` subframe case, whose
isolated pytest invocation emitted both a test failure and a teardown error;
the old ABI128 synthetic Frame redirect case passes in the new full run. The
independently verified immutable
[v6 paired successor](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v6/promotion-receipt.json)
has receipt SHA-256
`928da6d927abd7cf0667b63e88a89eb9fab260fe15ff0c50c04db358a295cb33`.
It changes only the 80 target candidate rows, preserves all 6,446 off-target
candidate raw rows and all 6,526 Oracle rows, and yields cumulative candidate
totals of **1,617 passed / 104 failed / four errors / 37 skipped / 4,764
not-run**. The preceding
[v5 URL-match successor](../.work/official-conformance/outcomes/python-navigation-url-match-chromium-strict-paired-v5/promotion-receipt.json)
(receipt SHA-256
`3c2d07e5d788b5ede07145e69ec78a6848045b3145a5772e61db55cf667145e6`)
proves three unchanged official URL-matched waiters; the earlier v4 history
successor and v2 Frame successor remain sealed, and the
[v1 RED baseline](../.work/official-conformance/outcomes/python-navigation-full-chromium-strict-paired-v1/promotion-receipt.json)
records ABI127 candidate 57 passed / 23 failed. The two explicit remaining
failures, back-navigation timeout and schemeless Tailscale fixture URL, also
fail in the Oracle; the `frameattached`/`framenavigated` subframe event is the
sole candidate-only unfinished navigation ID. Upstream URL completion
special-cases `localhost` and `127.0.0.1`, not the required MagicDNS fixture
host. These fixture-adapted Chromium slices establish neither unadapted
public-API parity nor complete Python corpus execution. See the
[Python ledger](python-official-conformance.md) for exact scope and residuals.
The source-only Python `Response.frame` fallback added after the ABI130-v2
wheel was frozen is not credited by v6. It passes 283/283 source-overlay
contract tests. The strict Pyright debt gate now explicitly selects its
isolated Python interpreter; 91 files still match exactly 66 known historical
errors, with 17/17 gate controls.

The opt-in Node `Page.goForward()` adapter now has two release native builds:
the isolated `history-navigation` addon (SHA-256
`25fa705710c835772e45c7bed33b836bded180109ec5f06425020f60a0f97905`)
and the full official page-check profile (SHA-256
`ce637639beec9343233441498739b24e642debd3334605f29e58eeab8bac75d0`).
Its browserless adapter/contract suite passes 220/220, native smoke and
TypeScript pass. The unchanged upstream Core
`page.goBack should work with HistoryAPI` case, which calls `goForward`,
passes [candidate 1/1](../.work/official-conformance/go-forward-abi129-browser/candidate-history-api.json)
and [Oracle 1/1](../.work/official-conformance/go-forward-abi129-browser/oracle-history-api.json)
in separately source-bound, fixture-adapted Chromium lanes. The adjacent
cross-document `@smoke` case is inconclusive: the candidate times out at
`goForward`, while the Oracle times out earlier at `goBack`. No Core ledger
row was promoted; the one-case result is not full history or browser parity.

Python public-API browserless scouting (earlier 2026-10-20 checkpoint): before
this navigation capture, the paired Python ledger had 4,357 candidate and
4,664 oracle `public-api-source` `not-run`
rows, including 4,355 shared IDs. A read-only source/fixture audit found no
honest next browserless public-API capture among those shared rows: remaining
tests use browser/page/context fixtures, launch/connect paths, or installation
that runs a browser. This is planning evidence, not a formal exclusion, test
pass, or replacement for executing the official corpus.

Python CI type debt (2026-10-20): a local exact-diagnostic gate retains the
full strict-Pyright `python tests` target set, pins Python 3.12 and Pyright
1.1.405, and accepts only 66 known diagnostics in five immutable historical
plugins across 91 analyzed files. The gate explicitly points Pyright at the
selected Python environment, independent of the macOS system interpreter;
17 mutation controls and the local gate pass. The pending CI workflow uses
this regression baseline and excludes only two hash-pinned plugins with Ruff
I001 debt from lint; it has not run remotely. A passing baseline means no new
type diagnostics, not that strict typing is clean. Versioned successor plugins
and evidence recapture are required to eliminate the debt. A separate
browser-denied diagnostic shows that a `test_cdp_session.py` Firefox marker
does not skip before session-scoped browser setup: candidate setup fails, so
the 22 similarly marked CDP-session nodes were not promoted as skips.

Relocatable Core historical snapshot (2026-10-20): the
[16-input packer/verifier](../scripts/package_playwright_core_paired_snapshot.py)
copied the pinned full manifest, five auxiliary manifests, inventory, and the
three candidate/oracle/receipt generations into an ignored local directory.
The `snapshot.json` SHA-256 is
`da9b093b5e914371f93137f72707f6574fd8a64f3dff13ac791349f888f74fc3`.
Verification from a different absolute directory, with only the package and
two checker scripts, replays the hash-pinned receipt chain and all 48,477
rows; the report SHA-256 is
`7e0da2ba408821e2ef275135eec61178d41f4c704071a16cc996de2e7dd148be`.
The deterministic gzip/USTAR archive is 8,215,934 bytes, SHA-256
`e5a7ab7af91eedb7b1341e8fb453402aeefe0eabb20ddc2887b8f68bb4362186`.
Its verifier requires that external hash pin and manually extracts a fixed,
bounded 17-file closure; 49 focused package/coverage tests pass, including
altered-file, symlink, traversal, duplicate, PAX/GNU metadata, oversized,
and extra-entry rejection. Historical absolute paths in receipts remain
opaque provenance text, never sources opened during relocation. This is a
historical snapshot-integrity check, not official test re-execution,
current-code parity, or a clean-CI gate: the raw bundle and archive remain
ignored, and the provenance paths need review before public distribution.

Full Python collection CI gate (2026-10-20): the
[inventory script](../scripts/inventory_playwright_python_official_tests.py)
now runs in the local CI workflow after the pinned upstream checkout, using
fresh Python 3.12 core and hash-locked Python 3.10 examples environments.
A local reproduction from those environments collected all 6,526 expanded
node IDs across 156 source files and produced the historical inventory
SHA-256 `3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41`.
The new CI step checks that digest and four inventory unit tests; it has not
yet run in remote CI. This
proves collection identity only, not that any upstream assertion passed or
that a Pwright candidate route executed.

Python upstream marker-skip diagnostic (2026-10-20): the
[strict 15-node manifest](../packages/pwright-python/tests/official/upstream-skip-markers-strict-v1-manifest.json)
pins the v1.58.0 source and candidate/oracle runtime identities. In the
ignored `python-upstream-skip-markers-strict-v1-final-v2` result directory,
both lanes produced 15/15 upstream `skipped` outcomes with zero case retries.
The process sandbox denied browser execution and outbound connections; a
pre-body guard recorded collection followed by skipped setup, and each JUnit
skip reason matched the upstream browser marker. All 15 pairs independently
reverify, and six negative controls pass (also wired into the local CI
workflow). An immutable [paired successor](../.work/official-conformance/outcomes/python-upstream-skip-markers-strict-paired-v1/promotion-receipt.json)
chains the predecessor receipt, inventory, policy, tool and all 30 final-v2
sidecar hashes; the receipt SHA-256 is
`21c65650e7f3b8c35c91e4d08b317227fe9ba94b06c0cc83c4fba0b2390287f1`.
Independent `--verify-existing` and raw-line comparison confirm that each
6,526-row lane changes exactly 15 `not-run` rows to `skipped`; all 6,511
other row bytes and every passed/failed/error count are unchanged. At this
historical v1 checkpoint, candidate had 4,853 `not-run` and 26 `skipped`;
oracle had 5,187 and 26. Ten promoter mutation controls pass. This is
skip-only accounting with zero new
public-behavior/parity credit, not a working browser/server. The earlier
diagnostic and `-final` sidecars are superseded and must not be used for
promotion. The corrected guard passes
strict Pyright. The earlier 67-error whole-project finding has since narrowed
to the 66-diagnostic interim baseline described above.

Python callable-fixture marker-skip successor (2026-10-20): an independent
[strict 11-node manifest](../packages/pwright-python/tests/official/upstream-skip-callable-fixtures-strict-v2-manifest.json)
selects only upstream-marked Firefox/WebKit cases whose setup supplies a
`launch_persistent` or `launch_persistent_context` callable but does not
launch a browser. All 11 candidate/oracle pairs independently verify the
upstream marker's setup-stage `skipped` result, the exact JUnit reason, and a
pre-body guard receipt under deny-exec/deny-outbound sandboxes. The immutable
[v2 paired successor](../.work/official-conformance/outcomes/python-upstream-skip-callable-fixtures-strict-paired-v2/promotion-receipt.json)
chains the preceding 15-node receipt and 22 new sidecars; its receipt SHA-256
is `1b8ce858fa3ab643fb10e8f12142c12b32e3164b8719e2c627d2f6e4a66dbb4a`.
Independent `--verify-existing` and byte comparison show exactly 11
`not-run`→`skipped` changes per 6,526-row lane; the other 6,515 row bytes and
all passed/failed/error counts are unchanged. Candidate totals are now 1,542
passed, 102 failed, three errors, 37 skipped, and 4,842 not-run; oracle totals
are 1,309 passed, four failed, 37 skipped, and 5,176 not-run. Ten promoter and
six verifier mutation controls pass. This earns no new public-behavior/parity
credit and is not browser or server support. A separate diagnostic on one of
22 CDP-session `only_browser("chromium")` nodes proves source markers alone are
insufficient: session-scoped `browser` setup runs before function-scoped
`skip_by_browser` and fails. All 22 have the browser-fixture dependency, so
none was promoted from its marker alone.

Browser-free Python ceiling (2026-10-20): read-only inspection of the pinned
upstream test bodies and original fixture closures found no further public API
case eligible for unchanged-fixture execution without a browser among the
current `not-run` rows. Each of the 102 candidate `failed` and three `error`
rows depends on a real Chromium browser fixture (including the indirect tap
fixture). Fifty-five failures have an oracle pass; 43 failures have no oracle
run; four failures also fail in the oracle. Of the three error rows, one has an
oracle pass and two have no oracle run, but their isolated JUnit reports are
invalid and must be recaptured before assigning behavioral fault. This is a
triage finding, not new execution or conformance credit. The dedicated CDP
endpoint remains unreachable, so a real-browser rerun is still pending.

Core version-mismatch browserless diagnostic (2026-10-20): the unchanged
official `browsertype-connect.spec.ts:1046` assertion runs against the pinned
upstream `run-server` fixture after an independently attested URL-only rewrite
of its incorrectly reported `localhost` endpoint to the verified Tailscale
MagicDNS address. The [v1 paired attestation](../.work/official-conformance/results/core-server-connect-version-mismatch-url-v1/strict-pair-20261020-v1/)
records oracle 1/1 but candidate failure before public `connect`, because the
official fixture reads a private BrowserType owner pointer. A separate
[v2 paired attestation](../.work/official-conformance/results/core-server-connect-version-mismatch-url-owner-v2/strict-pair-20261020-v2/)
uses an existing hash-pinned test-only owner adapter: oracle again passes 1/1;
candidate reaches public `connect` but returns a generic WebSocket error
instead of the upstream HTTP 428 version-mismatch explanation. Both versions
preserve the original test source, raw runner provenance, distinct immutable
envelopes, Tailscale bind/URL receipts, and 16 focused wrapper tests. They are
fixture-adapted diagnostics, not unadapted official parity; neither changed a
Core ledger row. The v2 discrepancy identified the need for bounded
no-Content-Length WebSocket rejection-body capture and safe typed
version-error mapping, rather than a Node-only message substitution.

The fresh [v3 isolated-package pair](../.work/official-conformance/results/core-server-connect-version-mismatch-url-owner-v3/strict-pair-20261020-v3/)
now passes the same unchanged official assertion in both lanes (oracle 1/1,
candidate 1/1). Its release-built, session-only native addon has SHA-256
`7a2664ef66516bcd1ada56b962a9f4a85abb05add8bcf3905113490fa9f39938`;
the candidate and oracle attestation SHA-256 values are respectively
`11ea37b4233f799f73f02ff6e7a0836f0635aadd7dd7af8b42ed9f3b7f998f8b`
and `54641e1250d83dcd657f9657b56549ecba80ffcfdf458d01863ead6f11ace230`.
Independent envelope replay verifies both; sealed v2 evidence still verifies
without changing its old addon. The fix opts only the Playwright Server live
connector into finite, 8 KiB-bounded HTTP 428 body capture and maps only
canonical numeric version pairs into the Node error. Transport release tests
pass 34/34, server-connect release tests 19/19, native Tailscale-bound mapper
tests 3/3, and strict release Clippy passes for those leaves and the addon.
This remains a URL/owner-fixture-adapted diagnostic, not an unadapted official
pass. Its new immutable [paired ledger successor](../.work/official-conformance/outcomes/core-server-connect-version-mismatch-url-owner-strict-paired-v3/promotion-receipt.json)
(receipt SHA-256
`5e6180a6b2e0a90d764276af84b933b14b6117cc4dd799a0f925622ea347e1b8`)
records exactly one `not-run` to `expected` transition per 48,477-row lane;
the other 48,476 raw lines remain byte-identical. An independent verifier and
49 mutation controls replay the sealed reports, predecessor and precise row
delta. This promotion is explicitly `implementation-internal`,
`fixtureAdapted=true`, and `publicApiParityEvidence=false`; the official test
starts a remote server but no browser. Candidate totals are 4,549 expected,
55 unexpected, 21 skipped and 43,852 not-run; oracle totals are 4,298
expected, eight unexpected, 20 skipped and 44,151 not-run. Legacy opt-in
WebSocket diagnostic ports can still display arbitrary remote bodies; the
narrow full-session mapping is not global error redaction.

The local CI workflow now runs Ruff and all 49 pure ledger-verifier mutation
controls without requiring the ignored `.work` evidence. It has not run
remotely, and it does not validate or re-execute the full outcome bundle in a
clean runner.

Core Firefox/WebKit browser-free DNS-error slice (2026-10-20): the pinned
`browsertype-connect.spec.ts:122` assertion passed in four previously unrun
Library invocations per lane (two Firefox, two WebKit), with one attempt each.
The [strict v3 pair](../.work/official-conformance/results/core-server-connect-ws-error-other-engines-strict-v3/)
rechecks source/fixture/adapter hashes, isolated candidate artifact and source,
Tailscale bind/host receipts, and exact spec identities. The existing common
runtime adapts the upstream server fixture in memory; test source and
assertions are unchanged. An immutable [paired ledger successor](../.work/official-conformance/outcomes/core-server-connect-ws-error-other-engines-strict-paired-v3/promotion-receipt.json),
receipt SHA-256
`59c7cbb11ad7284ab8aedadb8c850cd4552ffaa0b035b70430c8b4e2e4ef0ef7`,
changes exactly four `not-run` rows to `expected` per 48,477-row lane and
preserves the other 48,473 raw lines. At this predecessor checkpoint,
candidate totals were 4,548 expected,
55 unexpected, 21 skipped and 43,853 not-run; oracle totals are 4,297
expected, eight unexpected, 20 skipped and 44,152 not-run. The receipt retains
`implementation-internal`, `fixtureAdapted=true`, and
`publicApiParityEvidence=false`: these are recorded official diagnostic
outcomes, not browser execution or public-API parity credit.

Core Bidi-config browser-free WS DNS-error slice (2026-10-20): six further
unchanged `browsertype-connect.spec.ts:122` invocations, two each in
`bidi-chromium-library`, `bidi-chrome-stable-library`, and
`bidi-chrome-canary-library`, pass oracle and candidate 6/6 with one attempt
each. The [strict v4 pair](../.work/official-conformance/results/core-server-connect-ws-error-bidi-chromium-strict-v4/)
pins the upstream Bidi config, fixture closure, in-memory Tailscale server
adaptation, source assertions and candidate addon snapshot. The immutable
[paired successor](../.work/official-conformance/outcomes/core-server-connect-ws-error-bidi-chromium-strict-paired-v4/promotion-receipt.json)
has receipt SHA-256
`f7c72c003d4306ade21f077b8b424a60a21d265a72512c5b8ac249c21886d2c6`;
it changes exactly six `not-run` rows to `expected` in each 48,477-row lane,
leaving 48,471 other raw lines unchanged. Independent replay and 48 focused
mutation/strict controls pass. Candidate totals become 4,555 expected,
55 unexpected, 21 skipped and 43,846 not-run; oracle totals become 4,304
expected, eight unexpected, 20 skipped and 44,145 not-run. These rows remain
`implementation-internal`, fixture-adapted and diagnostic-only; the Bidi-named
project configuration here does not establish WebDriver BiDi protocol parity
or browser execution. The frozen v4 tools carry narrow strict-Ruff E501/I001
debt documented separately; their evidence-linked source bytes are not edited.

Core Bidi-config browser-free CDP Close slice (2026-10-20): the unchanged
`connect-over-cdp.spec.ts:572` custom WebSocket-close assertion passed once
per lane in `bidi-chromium-library`, `bidi-chrome-stable-library`, and
`bidi-chrome-canary-library` (3/3 oracle, 3/3 candidate). The
[strict v4 pair](../.work/official-conformance/results/core-cdp-close-bidi-chromium-strict-v4/)
pins the upstream Bidi config and test/fixture closure; its candidate uses a
frozen, attested older native CDP error-slice addon rather than the current
live package. The immutable
[v5 paired successor](../.work/official-conformance/outcomes/core-cdp-close-bidi-chromium-strict-paired-v5/promotion-receipt.json)
has receipt SHA-256
`13d7df7ccf58fb1838c5f523471a77fadd3b7ad92330d086f13c8d3ac64df068`.
Independent replay checks all 48,477 rows per lane: only indexes
6,527-6,529 change from `not-run` to `expected`; the other 48,474 raw lines
remain byte-identical. Thirty-six focused paired-verifier tests pass. Candidate
totals become 4,558 expected, 55 unexpected, 21 skipped and 43,843 not-run;
oracle totals become 4,307 expected, eight unexpected, 20 skipped and 44,142
not-run. This is `implementation-internal`, fixture-adapted,
diagnostic-only CDP-over-WebSocket evidence. The Bidi-named project config
does not make it WebDriver BiDi protocol evidence; neither browser execution,
public-API parity, nor current-live-package parity is claimed. The frozen
capture tools have separately tracked strict-Ruff E501 debt and are not
rewritten after sealing.

Core Bidi-config browser-free connect-timeout/redirect slice (2026-10-20):
five pinned `browsertype-connect.spec.ts` invocations (the two
`launchServer`/`run-server` variants at each of lines 221 and 228, plus line
1056) passed 5/5 in both oracle and candidate lanes, once per expanded ID.
The [strict v1 pair](../.work/official-conformance/results/core-server-connect-timeout-redirect-bidi-chromium-strict-v1/strict-pair-20261020-v1/)
pins the Bidi config, exact source assertions, a frozen older candidate
server-connect addon, two literal WebSocket URL-host rewrites, Tailscale
fixture binding and causal upgrade/redirect/socket-close receipts. No test
starts a remote browser server. The immutable
[v6 paired successor](../.work/official-conformance/outcomes/core-server-connect-timeout-redirect-bidi-chromium-strict-paired-v6/promotion-receipt.json)
has receipt SHA-256
`43b8f240c16d7c5517fcb16c90f77828c0818552be5e19e368a900356ca266ba`.
Independent replay confirms that only five `not-run` rows become `expected`
per 48,477-row lane; a separate byte comparison confirms all other 48,472
raw lines remain unchanged. Forty-five strict-capture tests and 46 paired
ledger tests pass locally. Candidate totals are now 4,563 expected, 55
unexpected, 21 skipped and 43,838 not-run; oracle totals are 4,312 expected,
eight unexpected, 20 skipped and 44,137 not-run. This is
`implementation-internal`, fixture-adapted, diagnostic-only evidence, not
browser execution, public-API, WebDriver BiDi-protocol, or current-package
parity. Full evidence replay still requires the ignored local bundle.

Core Bidi-config browser-free HTTP-401 slice (2026-10-20): the six pinned
`browsertype-connect.spec.ts:116` `launchServer`/`run-server` invocations
across three Bidi-named Chromium Library projects passed 6/6 in both strict
oracle and frozen candidate lanes, once per expanded ID. The
[strict v6 pair](../.work/official-conformance/results/core-server-connect-http-401-bidi-chromium-strict-v6/)
uses unchanged official assertions, a single literal localhost WebSocket
URL-host rewrite, Tailscale fixture binding, and guard receipts for the HTTP
401 responses; no browser or remote browser server starts. The immutable
[v7 paired successor](../.work/official-conformance/outcomes/core-server-connect-http-401-bidi-chromium-strict-paired-v7/promotion-receipt.json)
has receipt SHA-256
`ef2b8a1d16fd3758c784071e50777df0b35935e04b9c1a3eaa8bf3a60a339f9a`.
Independent replay checks all 48,477 rows per lane: only indexes 5,336-5,341
change from `not-run` to `expected`, and a separate byte comparison confirms
the other 48,471 raw lines remain unchanged. Candidate totals are 4,569
expected, 55 unexpected, 21 skipped, and 43,832 not-run; oracle totals are
4,318 expected, eight unexpected, 20 skipped, and 44,131 not-run. The capture
and promotion tools pass strict project-config Ruff and 115 focused local
tests, including mutation controls. This remains `implementation-internal`,
fixture-adapted diagnostic evidence with a frozen older addon: it does not
establish public-API, browser, WebDriver BiDi-protocol, or current-live-package
parity. The added clean-CI step checks source hashes and 114 pure tooling
tests; one upstream/snapshot pin test requires ignored local inputs and is
excluded there explicitly. The ignored evidence and official runs have not
been replayed remotely.

Core Bidi-config browser-free custom Close slice (2026-10-20): six pinned
`browsertype-connect.spec.ts:130` `launchServer`/`run-server` invocations
passed 6/6 in both the strict oracle and frozen candidate lanes. The
[strict v6 pair](../.work/official-conformance/results/core-server-connect-close-error-bidi-chromium-strict-v6/)
pins unchanged official assertions, a localhost WebSocket URL-host rewrite,
Tailscale fixture binding, and per-project ordered upgrade/message/Close
`4123 / Oh my!` receipts. Twelve real Close triplets were recorded in total;
the guard does not prove a cryptographic per-spec transport assignment. The
immutable
[v8 paired successor](../.work/official-conformance/outcomes/core-server-connect-close-error-bidi-chromium-strict-paired-v8/promotion-receipt.json)
has receipt SHA-256
`4c0a72a233440263a38bf718fa003fa2e8f8c36ef0243aaac9811c314e14dd68`.
Independent full replay confirms that only indexes 5,360-5,365 change from
`not-run` to `expected` per 48,477-row lane; all other 48,471 raw lines are
byte-identical. Candidate totals are 4,575 expected, 55 unexpected, 21
skipped, and 43,826 not-run; oracle totals are 4,324 expected, eight
unexpected, 20 skipped, and 44,125 not-run. Strict project-config Ruff and
98 local focused tests pass; a new clean-CI tooling step runs 97 pure tests,
explicitly excluding one local upstream/snapshot pin test. The source remains
`implementation-internal`, fixture-adapted diagnostic evidence using an
older frozen addon: no public-API, browser, WebDriver BiDi-protocol, or
current-package parity credit is claimed. Remote CI and portable raw-evidence
replay have not run.

Clean-CI evidence gap (2026-10-20): the existing relocatable 16-input Core
snapshot packages an older v1 chain, not the historical v8 ledger. A new
[ledger-only packer/verifier](../scripts/package_playwright_core_v8_ledger_snapshot.py)
created a deterministic, ignored local v8 archive with 11 sealed inputs plus
one sidecar: 5,870,164 bytes, SHA-256
`5d457866faaf95b7ed40130282de4ba00c07c1a49160f0c4aecb84614a0d9a95`.
Two local packs were byte-identical; an isolated, relocated verifier replayed
both 48,477-row lanes, with only indexes 5,360-5,365 changing, and 25 focused
tests plus two subtests passed. This proves historical row integrity only,
not official test re-execution or current-package compatibility. A clean-CI
tooling step pins the packer and runs 24 pure tests, explicitly excluding its
local-source replay test. The archive contains many local absolute paths and
remains private and unpublished; clean CI cannot fetch it, so no v8 archive
replay gate is wired or remotely run. The newer v9 successor is not in this
archive either. The full verifier still needs ignored raw captures, a frozen
addon and a Mac-specific upstream path; no portable
full-provenance replay exists yet.

Core Bidi-config public launch-failure slice (2026-10-20): all twelve v2
strict lanes of the unchanged official
`library/browsertype-launch.spec.ts:64` test completed. All six Playwright
oracles passed with a separately attested exit-1 dummy Node child, including
the BiDi Firefox `--profile` path and profile cleanup; all six frozen Pwright
candidates failed the original line-72 `Browser logs:` assertion before
starting that child. The received candidate error is that merged
`firefoxUserPrefs` is unsupported by the frozen native profile. The
[v2 evidence directory](../.work/official-conformance/results/core-dummy-node-bidi-strict-v2/)
passes a joint 12-sidecar independent verifier. The earlier v1 Firefox oracle
attempt was rejected because its guard accepted only `-profile` while BiDi
Firefox used `--profile`; its apparent assertion pass is **not** counted.
These are public launch API attempts under BiDi project configurations, not
successful candidate parity, actual browser execution, or BiDi protocol
conformance. Those six target rows remain `not-run` in the historical v8
ledger. A clean-CI tooling step pins the six v2 sources and runs 43 pure
tests; ignored official captures are unavailable to remote CI, which has not
run the new step.

The independently verified
[v9 paired successor](../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v9/promotion-receipt.json)
records those six public `BrowserType.launch` test attempts at inventory
indexes 6,110-6,115, with receipt SHA-256
`f35fa767281337803ea16ece4a12bb36de52523fe9e348fc7f4467402c3f215f`.
Exactly six previously `not-run` rows become `unexpected` in the candidate
lane and `expected` in the oracle lane; the other 48,471 raw lines per lane
remain byte-identical to v8. Each 48,477-row lane now has candidate totals
of 4,575 expected / 61 unexpected / 21 skipped / 43,820 not-run and oracle
totals of 4,330 expected / eight unexpected / 20 skipped / 44,119 not-run.
The receipt explicitly records no browser execution, public-API parity, or
WebDriver BiDi protocol parity. This is fixture-adapted evidence from a
frozen candidate addon, not a current-package compatibility result. It
remains the historical candidate-RED v9 ledger after the newer v3 capture
below. A new
clean-CI step pins three v9 tooling scripts and runs 30 pure focused tests,
including mutation controls;
the ignored paired evidence cannot be replayed there, and remote CI has not
run the step.

Opt-in BiDi startup-failure implementation checkpoint (2026-10-20): a new
`bidi-startup-failure` Node/Rust profile composes owned launch and trace-path
preparation with a browser-neutral Rust failure observer. It is selected only
for an explicitly supplied absolute `executablePath` and one of six pinned
`bidi-chromium`/`bidi-chrome-*` or `moz-firefox*` channels. The route accepts
inherited `firefoxUserPrefs` for this narrow pre-protocol case but does not
apply them. Rust observes at most two seconds of startup and 3 KiB of combined
logs, owns a temporary profile, and kills/reaps the process group before
returning the failure. It cannot return a usable Browser or establish a BiDi
session. Locally, the Rust leaf passes 8/8 release tests, the Node adapter
46/46, native integration 3/3, addon ABI/load smoke, the complete browserless
Node package 829/829, and strict TypeScript checks. Focused addon Clippy
passes; full-dependency strict Clippy is blocked by the pre-existing
`pwright-node-host` `too_many_arguments` warnings at lines 1268 and 1324.
The package-boundary check, including the new feature-specific exact Cargo
closure proof, passes locally: the new profile adds only its addon marker and
browser-neutral startup leaf beyond `playwright-trace`, and older profiles
remain unchanged. CI now has a browserless selected-profile build,
leaf-test, native-load/integration, adapter, and boundary step, but it has not
run remotely. The six unchanged official tests have now been rerun against
this profile in the separate strict v3 capture below. The implementation
tests alone provide no official conformance credit, and the frozen v9 ledger
does not change merely because a new capture exists.

Core Bidi-config public launch-failure v3 capture (2026-10-20): the
[strict v3 evidence directory](../.work/official-conformance/results/core-dummy-node-bidi-strict-v3/)
contains twelve isolated runs of the same unchanged upstream
`library/browsertype-launch.spec.ts:64` assertion: six candidate runs using
the new `bidi-startup-failure` profile and six pinned Playwright oracle runs.
All twelve have one `passed` attempt and an `expected` test status. The
independent verifier checks each sidecar plus the twelve-lane pairing; every
lane observes its actual exit-1 dummy Node descendant and profile cleanup,
including Firefox's real `--profile` spelling. Candidate receipts additionally
attest reaching the public `BrowserType.launch` route. The upstream assertion
is unchanged, but its executable is the deliberately substituted dummy Node
and the fixture is adapted. This proves only the bounded early-launch failure
behavior under those six BiDi-named configurations. It does not launch a
browser, return a Browser, establish a WebDriver BiDi session, or demonstrate
general public-API parity. The v9 candidate-RED paired ledger remains the
historical RED predecessor to the separately verified immutable v10 successor
below. A clean-CI step pins all six v3 tooling/fixture source hashes and
runs 58 browserless pure mutation tests locally. The frozen v3 Python sources
pass Ruff lint but are not project-config Ruff-format clean; CI hash-pins their
exact bytes and format-checks only the Firefox guard. The ignored captures
cannot be replayed in clean CI, and remote CI has not run.

The immutable
[v10 paired successor](../.work/official-conformance/outcomes/core-dummy-node-bidi-strict-paired-v10/promotion-receipt.json)
is now published locally with receipt SHA-256
`2ae0478daf066539db4c9904e344dc7c8b102ff65eda8ab9587f2b591f8903e2`.
Independent post-publication verification confirms that exactly the six target
rows at indexes 6,110-6,115 change in each 48,477-row lane, with the other
48,471 raw lines byte-identical to v9. Candidate outcomes move from six
`unexpected` to six `expected`; oracle target outcomes remain `expected` but
are refreshed from the separate v3 captures. Candidate totals are now 4,581
expected / 55 unexpected / 21 skipped / 43,820 not-run; oracle totals remain
4,330 expected / eight unexpected / 20 skipped / 44,119 not-run. The v9
receipt and ledgers remain immutable, hash-sealed historical evidence; the
old v9 verifier cannot replay its frozen v2 RED sidecars against the new live
candidate source, so v10 validates those predecessor bytes structurally and
replays the v3 sidecars live. The v10 receipt explicitly marks the fixture as
adapted and public-API, browser-execution, and BiDi-protocol parity evidence
as false. It proves only this early launch-failure assertion under six
BiDi-named configurations. A separate clean-CI step SHA-pins the three v10
tooling scripts and runs 35 pure tests locally; ignored official captures and
paired ledgers are unavailable in clean CI, and remote CI has not run.

Clean-CI source preflight (2026-10-20): the
[full Core verifier](../scripts/verify_playwright_core_full_source_inventory.py)
checks the pinned v1.58.0 commit, full manifest SHA-256, clean tracked source,
558 tracked spec files, 20 declared suites totaling 48,477 invocations,
tracked runner/config inputs, and four explicit non-collected fixture specs.
The generated `playwright-test` runner is not required before dependency
installation; its tracked package source is checked instead. The local pinned
checkout passes this preflight and 15 focused controls, and the same preflight
is wired into the pinned Core checkout CI lane. This is a source/declaration
integrity gate only: it executes no official assertions, does not validate the
48,477 expanded IDs or any candidate/oracle result, and does not rehydrate the
ignored `.work` outcome bundle. Full clean-CI ledger verification remains open.

ABI127 installed-wheel checkpoint (2026-10-20): a new independently default-off
`playwright-server-close-notices` Python feature composes the Rust
`ProjectCloseNotices` session policy and `close-state` browser operation
boundary. The additional close-only projection retains Browser/Context/Page
ancestry and generation identities, not Request/Response notices; it has
bounded object/notice queues. The base supervisor still retains validated
protocol objects under its separate 16,384-GUID/4 MiB limits. The native
async reader and a single Python facade watcher deliver server-originated Page
`close`, Context child-close, Browser close, then terminal `disconnected` in
wire order. Python wrapper registries key by GUID and generation and fence a
notice that arrives before its wrapper is published. The previous
whole-session-only profile remains available when this feature is absent.
A fresh ABI127 release wheel (SHA-256
`8b524b5b424ff2b5dd3a8f871af3a6dbae961fddfdaf895b6000ecc5ff919cb9`)
passes 84 focused installed-wheel tests and ten additional independent runs of
the Tailscale-bound synthetic WebSocket close fixture. That fixture pauses
between Page/Context `close` and `__dispose__` and verifies live inventories
exclude the closed identities, Page-only and Context-only callbacks, and
terminal Browser disconnection in order. All-feature Rust release tests pass
41/41 browser-ops, 22/22 page-events and 65/65 session; the selected Python
addon passes strict release Clippy. No real Playwright Server/browser or
unchanged official Python result is claimed here; no Python/Core ledger row
is promoted.
An independent close-notices-only ABI127 wheel (SHA-256
`02a13c04b0b1b9bee4e9e5fa9f0dbdb3a98bda8a61f53c05198f5af11885e4a4`)
omits the `frame-title` feature and native `NativeRemotePage.title` method;
its synthetic close, ABI contract and fake remote-connect tests pass 51/51.
This verifies title is not required by the close-notice feature, not
that the complete Python package has a minimal dependency footprint.

The separate default-feature ABI127 wheel (SHA-256
`731fd13c02b5cc716533f8f0f94027027d51cf94d860cd2656a18d039b2ea9c7`)
selects neither remote opt-in. Its complete browserless Python unit suite
passes 661/661, with 13 explicit skips for unselected remote or owned-process
profiles; `uv pip check` passes. Source-level targeted Ruff and Pyright pass
with zero errors. These package tests do not replace official upstream runs.

ABI126 remote Page checkpoint (2026-10-20): an explicit, default-off
`playwright-server-frame-title` Python feature composes the Rust
`frame-title` operation. A freshly installed release wheel (SHA-256
`ac2f027be220ce59160e6df716c38bb6bd74f48fcfa17f3fed8af22dbfa77f05`)
passes 70/70 focused Python tests, including a Tailscale-bound synthetic
WebSocket fixture that verifies `Page.title()` sends `Frame.title` to the
main Frame after its protocol `__adopt__` to the Page. A second fixture
aborts the socket while that typed command is pending and verifies a
`TargetClosedError` plus Page→Context→Browser notices; it passed ten
additional independent reruns against this wheel. The Rust actor now fences
the full Frame→Page→Context→Browser generation/parent chain and Frame adoption
epoch before wire send and at its ordered reply, and tombstones Page `close`
without prematurely removing its GUID. The selected Rust feature suites pass
37/37 browser-ops and 48/48 session release tests; both all-feature suites
pass 40/40 and 57/57. Addon release Clippy, Python Ruff/Pyright, and
the dependency-closure comparison pass with zero new normal addon packages.
This is a typed operation and protocol fixture, not an official upstream
test or live Playwright Server/browser test. The prior pre-fence ABI126 wheel
(`e4ef50d9b922530c7705022fd352e2b21741fc84e15c9587f0034d495ef0b713`)
is retained as historical evidence, not the current tested build. Server-
originated Page `close` was still missing from that ABI126 wheel while Context
and Browser remained connected; ABI127 synthetic installed-wheel checks address
that gap. The unchanged remote-kill RED/GREEN evidence has been
sealed with its historical source and wheel inputs; its verifier and 10/10
mutation controls pass despite later live-source drift. The local `.work`
seal still relies on absolute paths and upstream Git objects, so it is not
yet a clean-CI artifact. No Python/Core official ledger row changes here.

Current canonical snapshots still leave substantial work: the 48,477-row
[Core candidate ledger](../.work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1/candidate.jsonl)
records 4,544 expected, 55 unexpected, 21 skipped, and 43,857 not-run
outcomes. Its oracle partner records 4,293 expected, eight unexpected, 20
skipped, and 44,156 not-run. The latest immutable
[receipt](../.work/official-conformance/outcomes/core-dummy-node-other-engines-strict-paired-v1/promotion-receipt.json)
changes exactly the Firefox and WebKit Library dummy-executable launch rows:
the unchanged official assertion passes 1/1 per oracle engine and fails 1/1
per Pwright engine because their launch capability is not installed. The
candidate has no process/profile or native-runtime attestation for those
engines; the two rows are public-behavior RED, not parity credit. The earlier
ten pinned upstream skips remain explicit but likewise grant no parity.
The 6,526-row
[Python candidate ledger](../.work/official-conformance/outcomes/python-upstream-skip-callable-fixtures-strict-paired-v2/playwright-python-candidate-upstream-skip-callable-fixtures-strict-paired-v2.jsonl)
records 1,542 passed, 102 failed, three errors, 37 skipped, and 4,842
not-run.

The pinned, read-only [Core adapter coverage report](../scripts/report_playwright_core_adapter_coverage.py)
reconciles the 20-suite manifest, all 48,477 inventory rows, both ledgers,
the latest two-engine RED receipt, chained to the skip-only and strict-launch
predecessors (29/29 focused report tests pass). It identifies
33,188 candidate `not-run` rows in suites with no configured candidate
adapter, including 21,810 rows classified as public API candidates. Another
7,675 `not-run` rows have a configured `library` preload that only injects
Chromium CDP, not the selected Firefox/WebKit engine; 2,994 rows have a
configured Chromium route but remain unverified. All remain
applicability-unresolved, not excluded or passed. This report describes
the pinned ledger snapshot, not a new official execution. A separate audit
of the then 4,868 Python `not-run` rows found no public-behavior test directly
runnable without a browser while retaining its original `launch_server`
fixture. Twenty-six separate upstream marker skips have since been accounted
for; this source/fixture audit is a scheduling finding, not a formal
applicability decision.

A four-case, fixture-adapted [Python remote-kill RED predecessor](../.work/official-conformance/results/python-connect-remote-kill-synthetic-v1-oct20-final/attestation.json)
preserves the upstream async/sync assertion bodies: oracle 4/4, frozen ABI125
candidate 2/4. The failures lack the disconnected and ordered
Page→Context→Browser close events after abrupt remote termination. A rebuilt
ABI125 wheel with deferred Page coroutine rejection produces a separate
[GREEN diagnostic](../.work/official-conformance/results/python-connect-remote-kill-synthetic-v2-terminal-fix-oct20/attestation.json):
oracle 4/4 and candidate 4/4 in five independent runs. Its independent
verifier and 19 v2 mutation/control tests pass; the frozen RED evidence remains
unchanged. Both runs replace `launch_server` with a synthetic fixture, so no
Python ledger row is promoted. The repeated green result does not eliminate
the Rust actor's terminal-publication race in that pinned wheel. Subsequent
session-source work now publishes terminal state before returning command
terminal errors (53/53 selected page-events release tests and strict Clippy
pass), but the wheel has not been rebuilt from that source and a typed remote
Page operation plus live-server proof remain outstanding.

The prior browser-free Core public-launch slice changed exactly one row per
Core lane; the latest successor changes only ten upstream skip statuses per
lane. The Server work below does not change either official ledger. The
default-off Rust session now replays bounded pre-`Root.initialize` lifecycle
messages and events and rejects a pre-reply Browser close/disposal (19/19 fake-duplex
release tests). Its optional supervisor has 128-command and
128-event/approximately 4 MiB serialized-event queue limits in its default
queue policy. The explicit drop policy selected by the original Node and
pre-ABI127 Python profiles
still validates frames, projects the GUID graph and observes terminal state,
but does not retain unsupported ordinary events. A terminal
watcher observes disconnection without retaining the socket owner, and the
supervisor reports the exact pre-reply ingress count; supervisor-only release
tests passed 34/34 with strict Clippy before the new graph projection; the
earlier all-feature session checkpoint passed 40/40. It retains a bounded,
wire-ordered GUID initializer graph for later bindings. Two selected
Tailscale-bound real-WebSocket fixtures pass for raw lease and supervised
command/event/close paths against a synthetic server. A separate default-off
HTTP(S) `/json` endpoint-discovery crate resolves to WS(S) with bounded,
explicit live HTTP support (12/12 default
and 15/15 live-feature release tests, plus three selected Tailscale-bound live
HTTP fixture tests). A new explicit `pwright-playwright-server-connect` crate
composes discovery, checked WebSocket headers, bootstrap and supervisor. Its
default and all-feature release suites pass 9/9 each, plus one selected
synthetic Tailscale-bound HTTP-to-WebSocket fixture; the default closure omits
Reqwest and Tungstenite. A separate default-off browser-ops crate passes
19/19 release tests for checked `newContext`/`newPage` GUID handles and
empty-option Context close; it does not
yet validate the full Context/Page initializers or expose actions. The live
fetcher follows any valid-Location 3xx up to five
HTTP(S) redirects under one cumulative deadline, unlike the pinned upstream
helper's unbounded chain. Node's opt-in remote-session profile now connects
through the discovery/session composition and exposes Rust browser-ops-backed
empty-option `Browser.newContext`, `BrowserContext.newPage`, and explicit
`BrowserContext.close`, `Browser.newPage()` with an implicit Context, and
`Page.close()` on that implicit Page. Its graph-backed
inventories reject disposed and recycled GUID wrappers. Four Tailscale-bound
synthetic HTTP-to-WebSocket native tests pass, including 17 retained-wrapper
cycles, disposal, a 160-event unsupported-event burst, terminal Page→Context→Browser
close ordering, a cumulative deadline,
and outer URL-control normalization;
18 wrapper tests, strict release Clippy, native-class boundaries, and
TypeScript checks pass. An additional selected real-addon Tailscale fixture
passes 2/2 for explicit Context close and implicit-Context Page close while
Browser remains connected. Context-owned Page.close, navigation,
evaluation, rich Context options, and externally-created wrappers remain
unsupported. Python ABI125 independently opts into successful-connect Browser
lifecycle (version, state, close, and a single `disconnected` notification),
plus empty-option Context/Page creation. Its graph-backed inventories reject
unknown or recycled GUID generations; creation replies recheck parent
generation before publishing handles. The fresh installed selected wheel passes
49/49 focused Python tests on each CPython 3.9.6 and 3.12.14, including four
Tailscale-bound synthetic HTTP-to-WebSocket variants (one with 160 ordinary
events). A prior selected
Rust addon checkpoint passed 41/41 release tests; current binding source
passes strict release Clippy and builds into that wheel. Direct `cargo test`
of the current addon cannot link on this Mac because its Python 3.9 library
is absent. A new selected ABI125 wheel with empty-option Context close and
`Browser.new_page()` implicit Context ownership passes a fresh focused subset
of 65/65 on each of CPython 3.9.6 and 3.12.14 with Tailscale-bound fixtures
enabled, including 6/6 compiled-extension WebSocket cases.
The current whole-project strict Python Pyright gate remains RED (29 errors in
three byte-frozen historical test plugins); current binding/unit files have
zero targeted errors. Context-owned Page.close and arbitrary externally
initiated child close were unsupported in that ABI125 wheel; the opt-in
ABI127 synthetic installed-wheel checks address the latter only.
Context.close checks generation before send but the protocol lacks a
generation-aware compare-and-close operation. Neither binding has real Playwright
Server validation or new official successful-connect ledger results.
The browser-free [Core connect-timeout v2 attestation](../.work/official-conformance/results/core-server-connect-timeouts-strict-v2/core-server-connect-timeouts-strict-v1-fixed-v2-20261012/attestation.json)
records 36/36 expected outcomes in both candidate and independent oracle
lanes after a typed Node timeout mapping fix; v1 retains the prior candidate
18/36 RED. It adapts two literal fixture hosts, is implementation-internal,
and adds no official ledger credit.
The separate [Core HTTP-redirect timeout v1 attestation](../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v1/core-server-connect-redirect-timeout-strict-v1-second-20261012/attestation.json)
preserves the pre-fix RED: oracle 9/9, candidate 0/9. The selected source passes
`Connection: Close` and expects the 2-second discovery timeout plus socket
abort; Pwright rejects that WebSocket-reserved header before starting HTTP
discovery. The independently verified [v2 GREEN successor](../.work/official-conformance/results/core-server-connect-redirect-timeout-strict-v2/core-server-connect-redirect-timeout-strict-v2-fixed-20261012/attestation.json)
passes the same nine unchanged assertions in both oracle and candidate lanes.
Seven verifier mutation controls pass (nine verifier tests total). HTTP(S) discovery validates header
syntax and bounds immediately, but defers rejection of reserved WebSocket
handshake names until after discovery and before upgrade; direct WS(S) still
rejects them before I/O. The selected real-addon Tailscale fixture passes 3/3
for these ordering boundaries. Only the server fixture host/bind is adapted;
these implementation-internal diagnostics remain outside the official ledger.
The historical, independently verified [Core remote-Page waiter RED](../.work/official-conformance/results/core-connect-waiters-synthetic-v1-red-v5/attestation.json)
selects three unchanged pinned `run-server` assertions: oracle 3/3,
candidate 0/3 with the then-current context-close-capable addon. Both lanes created
a Context and Page without protocol errors; candidate then failed at its first
missing `waitForRequest` or `waitForEvent` call. It never reached the assertions,
did not invoke `waitForResponse` on the first row, and gives no positive event
delivery or terminal-order evidence. Only the server launcher is replaced by a
Tailscale-bound synthetic protocol fixture. These implementation-internal rows
remain `not-run` in the canonical Core ledgers.
The default-off
[`pwright-playwright-server-page-events` Rust leaf](../crates/pwright-playwright-server-page-events/src/lib.rs)
has 16 passing release tests for bounded, wire-ordered
Request/Response/Download and terminal notice projection, including
generation/parent checks, disposal survival, closed-parent rejection and queue
overflow. Its normal dependency closure is protocol-core plus `serde_json`,
without Node/CDP/WS. The separate session `ProjectPageNotices` policy now
projects inline after validation (50/50 selected release tests) while dropping
unrelated events; the default policy and dependency closure are unchanged.
Rust browser-ops acknowledges Page request/response subscriptions before a
selected Page is published (22/22 release tests). The new opt-in Node addon
profile builds and loads with a typed notice port; its JS adapter passes
25/25 focused tests, including early terminal-publication and terminal
predicate regressions. The default package suite now includes the adapter and
passes 816/816; strict TypeScript checks pass. An actor-side generation guard
rejects a target recycle observed before a command is sent, without claiming
protection from remote state the actor has not yet seen. The independently
verified [Core Page-events v5 GREEN diagnostic](../.work/official-conformance/results/core-connect-page-events-synthetic-v2-green-20261020-v5/attestation.json)
records the same three unchanged pinned `run-server` assertions (lines
396/412/423) passing 3/3 in each isolated oracle and real-addon candidate
lane, with zero skips. A separate positive probe in each lane delivers
Request→Response→Download to matching waiters and Page listeners after
request/response subscription acknowledgements, retaining Request identity.
The attestation pins the selected addon at
`0be0a9b59bbbff98e35be069f62ef7e455e641e23d302857b6546584510331f9`,
the package source and 13 harness inputs; its independent verifier and five
mutation controls pass. Only the server launcher is replaced by a
browser-free synthetic fixture. It is an internal, fixture-adapted diagnostic,
not an unadapted official pass, real Playwright Server/browser proof, full
public-API parity, or ledger credit. The historical waiter RED and both
canonical ledgers are unchanged.
The independently verified [Python navigation-after-close RED](../.work/official-conformance/results/python-connect-navigation-closed-synthetic-v1-red-v2/attestation.json)
preserves three unchanged upstream async assertions: oracle 3/3, selected
ABI125 candidate 0/3 because the not-yet-installed `Page.goto` error wins
over the already-closed target state. Only the server-launch fixture was
adapted; this is not official ledger credit or real-browser proof.
A new isolated [GREEN successor](../.work/official-conformance/results/python-connect-navigation-closed-synthetic-v2-green-v1/attestation.json)
uses the rebuilt ABI125 wheel after closed-target error precedence was fixed:
oracle and candidate each pass 3/3 unchanged assertions. It reuses the exact
pinned fixture sources and remains a fixture-adapted internal diagnostic,
without official ledger credit or browser-engine proof.
Python now emits Page `close`, Context `close`, then Browser `disconnected`
once on whole-session termination. Without a caller dispatcher, sync callbacks
are delivered at the next caller-thread API boundary, not promptly during an
otherwise idle caller. That ABI125 wheel did not support arbitrary
non-terminal child-close events; the ABI127 opt-in source above addresses
them, pending installed-wheel verification.
The fresh, independently verified
[Python synthetic lifecycle attestation](../.work/official-conformance/results/python-connect-lifecycle-synthetic-v1-green-v1/attestation.json)
records four unchanged upstream async/sync assertion bodies across all three
browser-engine parameters passing **12/12 in each oracle and earlier ABI125 selected-wheel
candidate lane**. Only the pinned server-launch fixture was replaced in memory
with a Tailscale-bound browser-free protocol server. This is an
official-derived fixture-adapted internal diagnostic, not an unchanged
official-server pass, Firefox/WebKit browser proof, or ledger promotion.
A new [six-case state-only ABI125 diagnostic](../.work/official-conformance/results/python-connect-lifecycle-synthetic-v2-closed-target-oct20/attestation.json)
pins the current closed-target wheel SHA-256
`32c414d3eb2a45417c4a3ba79705be96179dc2ec2dd20896cd0cda7d8169e3e8`.
The unchanged upstream async and sync
`test_browser_type_connect_set_browser_connected_state` bodies pass 6/6 in
each isolated oracle and installed-wheel candidate lane across Chromium,
Firefox and WebKit parameters; 13 verifier/mutation tests pass. The pinned
`launch_server` fixture is replaced by the browser-free synthetic protocol
server. The attestation explicitly says `fixtureAdapted=true`,
`publicApiParityEvidence=false`, `unadaptedOfficialPass=false`, and
`promotedToPythonLedger=false`: no actual engine or real Playwright Server
was used, and all six Python ledger rows remain `not-run` in both lanes.
The pinned Python sync Chromium close-event assertion body also passes
**1/1 in each oracle and fresh ABI125 wheel candidate lane** in an independently
verified [child-close attestation](../.work/official-conformance/results/python-connect-close-events-synthetic-v1-green-v1/attestation.json).
The upstream server-launch fixture was replaced with a browser-free protocol
server, so this proves only the terminal event slice and adds no official
ledger row.
The pinned Core line-382 child-close diagnostic has a frozen
[RED predecessor](../.work/official-conformance/results/core-connect-child-close-synthetic-v1-red-v1/attestation.json)
(oracle 1/1, candidate 0/1 timeout) and a verified
[GREEN successor](../.work/official-conformance/results/core-connect-child-close-synthetic-v1-green-v1/attestation.json)
(1/1 in each lane) on the same native addon. The JS binding now emits Page
`close`, Context `close`, then Browser `disconnected` once on whole-session
termination. This does not handle arbitrary non-terminal child-close events;
the server-launch fixture was replaced, so the result is internal diagnostic
evidence and earns no official ledger credit.
The first paired lifecycle success diagnostic exposed an endpoint discrepancy:
the pinned upstream `run-server` fixture retains a trailing newline, accepted
by official Playwright but initially rejected by Pwright. A Rust N-API boundary
fix now normalizes only outer URL controls/space; embedded controls remain
invalid. The fresh, independently verified
[synthetic lifecycle attestation](../.work/official-conformance/results/core-connect-lifecycle-synthetic-v1-green-v2/attestation.json)
records the unchanged Core line-304 and line-321 assertion bodies passing
**2/2 in both oracle and selected Pwright lanes**, with separate Tailscale-bound
synthetic servers and per-lane upgrade, `Root.initialize`, bootstrap, close,
and stop receipts. Its candidate addon SHA-256 is
`ef6093533f521893f9aba9c78f270bfce46237d13fa7255c2b0c838890a1547a`.
Only the pinned upstream server launcher was replaced in memory; this is an
official-derived fixture-adapted internal diagnostic, not an unchanged official
pass, real BrowserServer proof, public API parity, or ledger promotion.
The complete Core/Python browser-free public files are already green in both
lanes, so rerunning them would not establish a new discrepancy fix. The
prepared 17-case Chromium Page lane below remains unexecuted while this Mac's
dedicated Chrome CDP listener is unresponsive.

Next public-API paired lane is the complete 17-case Chromium-page
`page-wait-for-selector-1.spec.ts` file, pinned by
`tests/official/playwright-core-page-wait-for-selector-1-public-v1-manifest.json`.
The current default-off `official-conformance-page-retained-wait` candidate
package was built in release mode at
`.work/official-conformance/builds/node-retained-wait-readiness-ZIjgpA/candidate-package`
(addon SHA-256
`acd096f643a5373ad503bfd6bcc892f669fb93296401124efc073dda3ce82ee1`).
Its native load, 3 manifest controls, 27 browserless wait-adapter tests,
dependency boundary, and all-17 collect-only selection passed. This is
readiness evidence only: the dedicated Chrome CDP listener did not answer its
HTTP version request, so neither official lane ran, no outcome was promoted,
and the old candidate failures remain historical rather than current proof.

The latest browser-free Core connect-header post-refactor v2 diagnostic reruns
the same four pinned line-235/250 `launchServer` and `run-server` cases listed
below: candidate and independent Playwright 1.58.0 oracle each pass **4/4**.
The selected default-off `playwright-server-connect-headers` addon receives
runtime identity as one bounded three-string tuple and exports the profile-
specific ABI-shape marker `playwrightServerConnectHeadersAbiShape() === 2`;
the current loader rejects the older header addon. The frozen artifact SHA-256
is `0b2553d3137a77f162836d8fadeacbf1d5e47038b24ae97fe9d1c9519d068194`.
The strict [candidate](../.work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2/candidate-v1.strict.json)
(SHA-256 `e9d05121603d7fd8c30b2f30286d3afb78ccf8bb5a255a2c1be8fafff0f129ec`)
and [oracle](../.work/official-conformance/results/core-server-connect-headers-strict-v1/post-refactor-v2/oracle-v1.strict.json)
(SHA-256 `dcbb6c8974da20ac308541d518f87b6e307f7301050c6cff51e4e92b9de02e85`)
sidecars pin the candidate snapshot, ABI shape and four ordered upgrade
receipts per lane. They check explicit `User-Agent: Playwright`, default
`Playwright/1.58.0 (arm64; macOS 26.6) node/26.7`,
`x-playwright-browser: chromium`, and `foo: bar`. The fixture adaptation is
still limited to the two WebSocket URL hosts at lines 238 and 253, changed
from `localhost` to this Mac's full Tailscale MagicDNS host. Sanitized wire
receipts record asserted headers and protocol-header counts, not per-spec
socket IDs; mapping a receipt to a spec is an ordinal inference. They do not
prove complete header-set or proxy-header parity. The sidecars
set `publicApiParityEvidence=false`, `unadaptedOfficialPass=false`, and
`promotedToCoreLedger=false`. This proves neither a BrowserServer nor a
successful remote Browser or general `BrowserType.connect` parity.

The historical, browser-free Core connect-header v1 diagnostic selects the
four original `browsertype-connect.spec.ts` cases formed by lines 235 and 250
under both `launchServer` and `run-server`. The pinned source is Playwright
1.58.0 commit `961381ec73a9fa2661d92504eab1a6bd6eaffc75`. Candidate and
independent oracle each passed **4/4**. The selected spec IDs are:

| Kind | Line | Pinned spec ID | Header assertion |
| --- | ---: | --- | --- |
| `launchServer` | 235 | `ee6df4b3a2217bd356a7-9382fbb61fa92ab98ca5` | Explicit `User-Agent: Playwright` and `foo: bar` |
| `launchServer` | 250 | `ee6df4b3a2217bd356a7-80c13ccf5df0f8fe137d` | Default `User-Agent`, `x-playwright-browser`, and `foo: bar` |
| `run-server` | 235 | `ee6df4b3a2217bd356a7-604eb43298a85cc85a37` | Explicit `User-Agent: Playwright` and `foo: bar` |
| `run-server` | 250 | `ee6df4b3a2217bd356a7-07fdbb8cb8ad53bc3eaf` | Default `User-Agent`, `x-playwright-browser`, and `foo: bar` |

The selected default-off feature/profile is
`playwright-server-connect-headers`; the pre-ABI-arity-refactor v1 native
artifact SHA-256 is
`30d8478b0d74acfe556b5209633045434503738c4424483ab7ea7bb952f358aa`.
The strict [candidate](../.work/official-conformance/results/core-server-connect-headers-strict-v1/final-frozen-v1/candidate-v1.strict.json)
(SHA-256 `9cd89efe6495a99404d75a613a8e41870b31c87c1badb7cd5e9aed9306b3730b`)
and [oracle](../.work/official-conformance/results/core-server-connect-headers-strict-v1/final-frozen-v1/oracle-v1.strict.json)
(SHA-256 `32341cf62335ee5455a826a35993f0ff9c1ca3adcd0ee306420f107f51f31905`)
sidecars pin a frozen candidate package snapshot and verify four ordered
upgrade receipts per lane. The sanitized receipts prove asserted header values
and protocol-header counts, but have no per-spec socket ID; variant attribution
uses execution order and case counts. Only the fixture's WebSocket URL hosts
at lines 238 and 253 were changed from `localhost` to this Mac's full
Tailscale MagicDNS host. Thus the classification is
`official-derived-fixture-host-adapted-internal`, with
`publicApiParityEvidence=false`, `unadaptedOfficialPass=false`, and
`promotedToCoreLedger=false`. No BrowserServer, real browser, remote Browser
object, current-source parity, or general `BrowserType.connect` success is
proven by this historical v1 capture.

The separate Core HTTP 401 v5 strict diagnostic passes two original
`browsertype-connect.spec.ts:116` assertions in each frozen candidate and
Playwright oracle lane. It adapts only the hard-coded fixture URL host to a
full Tailscale MagicDNS name, and is explicitly
`official-derived-fixture-host-adapted-internal`, not an unchanged official
run or public API parity. The sidecars at
`.work/official-conformance/results/core-server-connect-http-401-strict-v5/`
verify two real fixture 401 sends per lane, but the sends carry no per-spec ID.
No ledger promotion occurred. An earlier, separate host-adapted Core
Close-error v4 diagnostic passed original assertions but is lifecycle RED:
candidate server receipts showed abnormal Close `1006` while the oracle
showed `4123 / Oh my!`. The v4 verifier's distinct-worker assumption rejects
both v4 sidecars; only raw hash-pinned receipts support this RED observation.
After a transport Close-reply flush fix, the separate
v5 host-adapted strict successor passes the two original line-130 assertions
in both candidate and oracle lanes (2/2 each). An independent pair verifier
checks two ordered server-side upgrade, message and Close `4123 / Oh my!`
triplets per lane. Both cases reused worker 0/port 8907. The receipt has no
spec ID or timestamp for each triplet; per-spec mapping is an order-based
inference. The new candidate artifact SHA-256 is
`39a8ecfdac8db8d0cf1eb0e100c3c17aaf59fdf9c4dbb4f9739602202f0843f1`.
This is internal, fixture-host-adapted diagnostic evidence only, not an
unadapted official pass, public API parity, or ledger promotion; v4 remains
preserved as RED.
The latest Core paired successor is a diagnostic GREEN slice:
`.work/official-conformance/outcomes/core-server-connect-ws-error-strict-green-paired-v2/`.
Two unchanged official `browsertype-connect.spec.ts:122` WebSocket-error
assertions pass 2/2 in each frozen Pwright candidate and Playwright 1.58.0
oracle lane. The [receipt](../.work/official-conformance/outcomes/core-server-connect-ws-error-strict-green-paired-v2/promotion-receipt.json)
has SHA-256 `e5921619fdfd88dd14ec71ab124ae952f34edc6678e7b914e46692ac0c754da4`;
the versioned [RED predecessor](../.work/official-conformance/outcomes/core-server-connect-ws-error-strict-red-paired-v1/promotion-receipt.json)
remains intact. Only two rows per 48,477-row lane change; the other 48,475
are byte-identical. Candidate is **4,543 expected / 53 unexpected / 11
skipped / 43,870 not run**; oracle is **4,290 expected / eight unexpected /
ten skipped / 44,169 not run**. The selected Node profile performs a bounded
Rust DNS/WebSocket error path and omits URL query parameters from the public
error. It still rejects after a successful WebSocket upgrade and cannot
return a remote Browser. These tests require a private upstream fixture, so
the receipt remains `implementation-internal`, `diagnosticOnly=true`, and
`publicApiParityEvidence=false`: this is not public API, BrowserServer, or
browser-execution parity. Also, 2,610 of the candidate's 4,543 `expected`
Core rows are `upstream-private-source-only` and did not load Pwright; the raw
ledger pass count is not a count of Pwright runtime-compatible cases.

The latest browser-free Python paired successor is
`.work/official-conformance/outcomes/python-connect-refused-strict-paired-v1/`.
The unchanged official async/sync `BrowserType.connect` closed-server
assertions pass **6/6 in each isolated candidate/oracle lane** on the ABI123
opt-in wheel, with 12 independently verified single-case sidecars. Its
versioned, Tailscale-bound fixture replaces only the upstream `RemoteServer`
launch with a real listener closed before the public call; no browser or
BrowserServer was launched. The paired receipt SHA-256 is
`4979cdc982cf716e5124bf00b972594c27a65fc26104c193ed12957cf5d4c060`.
Candidate is now **1,542 passed / 102 failed / three errors / 11 skipped /
4,868 not run** (ledger SHA-256
`3095771a0928ecc076ecdec8eee5d2946d70de9dbd1ea15e00f61ad7fa79e836`);
oracle is **1,309 passed / four failed / 11 skipped / 5,202 not run**
(ledger SHA-256 `11afbc5f482a3d9cefa36c59598f1771903b74f93749dcd11e8b5b178a7a6208`).
Exactly six rows per lane change; the other 6,520 are byte-identical to the
predecessor. The ledger's existing row schema does not encode the fixture
variant, so these `passed` rows must be read with the receipt's fixture-only
scope and `publicApiParityEvidence=false`; see the
[strict evidence note](knowledge/pwright-python-connect-refused-strict.md).

The preceding Python paired successor is
`.work/official-conformance/outcomes/python-connect-close-post-fix-strict-paired-v2/`.
The pinned, unchanged official Python async `BrowserType.connect` early
WebSocket-close assertions pass **3/3 in each isolated oracle/candidate lane**
for Chromium, Firefox and WebKit. Six independently verified strict sidecars
under `.work/official-conformance/results/python-connect-close-post-fix-strict-v2/`
prove the public call, actual inbound `Root.initialize` message and fixture
Close `4123 / Oh my!`. The ABI122 opt-in wheel SHA-256 is
`2ad72324788ed7ae24eeb6f265e035f64a654e75ebc342ddb2537730f659bc1d`;
fresh CPython 3.9.6 and 3.12.14 installs each pass 67/67 focused browserless
tests and `uv pip check`. The paired receipt SHA-256 is
`03b22f0bc8e986217e9c6d9c301139049a0d268795ba15197ed5f4716bb52b72`;
its read-only verifier passes. Candidate is now **1,536 passed / 102 failed /
three errors / 11 skipped / 4,874 not run** (ledger SHA-256
`1acbae02e2049b96d2556a3093b8a65cd43b12ac00f53745ee6f58c764990afb`);
oracle remains **1,303 passed / four failed / 11 skipped / 5,208 not run**
(byte-identical ledger SHA-256
`7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7`).
Only the candidate's three target rows changed; the receipt explicitly sets
`fullPlaywrightServerSupport=false` and `publicApiParityEvidence=false`.
No remote Browser/GUID graph, Node `connect`, or broad protocol compatibility is
proven by this failure-path slice.

The preceding browser-free Python dummy-engine successor is
`.work/official-conformance/outcomes/python-dummy-engine-post-fix-strict-paired-v2/`.
The unchanged official async/sync Firefox/WebKit launch-failure assertions
pass **4/4 in each isolated lane** with a real, nonce-bound dummy Node exit 1;
Firefox reports `Error` and WebKit `TargetClosedError`. The candidate uses the
frozen opt-in ABI121 wheel (SHA-256
`904db7f0af2082451edfbe589afcedd9fdc77295ce627cbc391d9e60b9780367`).
Fresh CPython 3.9.6 and 3.12.14 installs each pass 40/40 focused
browserless tests and `uv pip check`; both load the same ABI121 native bytes.
All eight strict sidecars under
`.work/official-conformance/results/python-dummy-engine-post-fix-strict-v2/`
and the immutable paired receipt (SHA-256
`009556238f14410c229cad349760bef962183fca33cf82569dc6a5c74d70a5bf`)
pass independent verification. Candidate is **1,533 passed / 105 failed /
three errors / 11 skipped / 4,874 not run** (ledger SHA-256
`5c88ae0e2cad7a0b5c10228e1435d220fb6263311b5d85cbd0f4c323e7eb32a3`);
oracle is **1,303 passed / four failed / 11 skipped / 5,208 not run**
(ledger SHA-256 `7d044a471c9ee530de390d676d50478d28c3df43a4f106b60528809c7a114ec7`).
Exactly four target rows change per lane; the other 6,522 rows are
byte-identical to the preserved red v1 predecessor. The receipt explicitly
records `fullFirefoxWebkitSupport=false` and `publicApiParityEvidence=false`:
the leaf handles only bounded early exit of an explicit executable, not a
working Firefox/WebKit browser protocol or full Python conformance.

The ABI121 browser-free Chromium CDP WebSocket shard uses the full
109-capability ABI121 Python wheel at
`.work/official-conformance/builds/python-abi121-ws-AE870c/shuozeli_pwright-0.1.0-cp39-abi3-macosx_11_0_arm64.whl`
(SHA-256 `20033ac820480165caf4c420961e22f0fd8c4c29732d2fbcde582c05adabe46e`).
Fresh CPython 3.9.6 and 3.12.14 installations of that wheel each pass
578/578 browserless Python units and `uv pip check`.
Against pinned official Python commit
`47a5d35ef4f815a2021349f86ae391f7c20c02d6`, the candidate and
independent Playwright oracle each pass the unchanged custom-header and
custom-WebSocket-close tests (2/2 per lane). Four independently verified strict
sidecars are at
`.work/official-conformance/outcomes/ws-strict-{candidate,oracle}-abi121-{header,close}-v1.provenance.json`.
The only fixture-transport adaptation forwards the official fixture's
hard-coded `127.0.0.1` destination to this Mac's verified full Tailscale
MagicDNS name on the same port; the sandbox permits only that ephemeral
fixture port. No Chrome process was used for this shard. The paired promoter
revalidated all four reports and published an immutable 6,526-row successor
at `.work/official-conformance/outcomes/python-cdp-ws-abi121-strict-paired-v1/`;
its read-only `--verify-existing` check passed. Only the two target Chromium
rows changed in each ledger, from `not-run` to `passed`.
The pinned Core JS/TS `connect-over-cdp.spec.ts` also checks explicit headers
on both WS and HTTP discovery plus a default `User-Agent` on WS. An earlier
source-pinned, browser-free official Core pair recorded the explicit-header
case passing in both candidate and oracle, and the default-header case
failing in candidate but passing in oracle. At that checkpoint, candidate
received
`undefined` where upstream expects its runtime `getUserAgent()` value. The
only source adaptation changed the fixture URL host from `localhost` to the
same-port full Tailscale MagicDNS host; the official calls and assertions are
unchanged. Both tests retain their inventoried `implementation-internal`
classification because their file imports an upstream private helper; the
actual failed result is still recorded, not excluded.
The header pair's immutable predecessor remains at
`.work/official-conformance/outcomes/core-cdp-handshake-headers-strict-paired-v1/`.
A separate strict, browser-free pair then ran the unchanged official URL
query tests at source lines 275 and 287: both candidate and oracle pass 2/2.
They verify that CDP HTTP discovery retains `?foo=bar` and appends
`/json/version/` with or without an existing path slash. Only the fixture
host was adapted to the same-port Tailscale MagicDNS name; no Chrome ran.
The preceding immutable 48,477-row Core paired successor is
`.work/official-conformance/outcomes/core-cdp-url-query-strict-paired-v1/`.
It adds only the two URL rows per lane; the other 48,475 rows, including
the earlier header failure and `implementation-internal` classifications,
are byte-identical to their predecessor. Read-only verification passes.
Candidate is **4,534 expected / 57 unexpected / 11 skipped / 43,875 not run**
(SHA-256 `8d4d6a5804657f9757b535c78caf293f41c059ee71375e46fc4413a46245df09`);
oracle is **4,282 expected / 11 unexpected / ten skipped / 44,174 not run**
(SHA-256 `58503837e2cc5fedd1e4055bc4703ec65d7d7cdc26475ad02ba3b2b871d75fb5`).
At that URL-query checkpoint, default WebSocket `User-Agent` parity remained open.
A newer strict, browser-free Core pair runs the unchanged official
`browserType.executablePath()` assertion for Chromium, Firefox and WebKit.
The earlier three candidate rows failed before the public call because the
official fixture reads private `BrowserType._playwright` metadata; the earlier
oracle rows failed because its browser-cache paths were absent then. The
current browser cache contains the three pinned browser executables. A
versioned, candidate-only fixture adapter now supplies just that private
backpointer; the native `browser-registry` profile still computes each public
path. Both fresh candidate and independent oracle pass all three unchanged
assertions. Source, selected artifact, fixture adapter, runner, result,
provenance and worker receipt are independently verified; eight verifier
negative/contract tests and three successor-integrity tests pass. The
browser-owner immutable 48,477-row Core pair is
`.work/official-conformance/outcomes/core-browser-type-owner-strict-paired-v2/`.
Only those three previously unexpected rows changed per lane, with the source
ledger's original classification bytes and every other row preserved. The
separate corrected invocation classification is
`implementation-internal`/`private-client-access`, not public API parity:
the test still requires an adapter for its private fixture backpointer. The
immutable seal at
`.work/official-conformance/outcomes/core-browser-type-owner-strict-paired-v2-seal-v1/attestation.json`
pins the strict verifier and promoter source hashes and explicitly records
`publicApiParityEvidence=false`; it does not rewrite the source ledger.
Candidate is **4,537 expected / 54 unexpected / 11 skipped / 43,875 not run** (SHA-256
`40625a1de093799e3ad19b0059721c95fd84f3fa04654f373d273ff80abd2e71`);
oracle is **4,285 expected / eight unexpected / ten skipped / 44,174 not run**
(SHA-256 `a03b9b2f9cf1ec06e58db2ce08bad2c770b8eb8620f73ee1aec82d0415f10fd5`).
The prior Core default-User-Agent failure remains unchanged in this successor.
A subsequent strict, browser-free official Core pair runs the unchanged
`connect-over-cdp.spec.ts:299` default WebSocket `User-Agent` assertion against
the pinned 1.58.0 source. Candidate and independent oracle each pass 1/1.
Only the fixture URL host changes to this Mac's full Tailscale MagicDNS name
on the same port; the official call and assertion are unchanged, and no Chrome
runs. The file's private-helper import keeps the row
`implementation-internal`; the strict promotion receipt records
`publicApiParityEvidence=false`, so this is not unadapted public API parity.
That immutable 48,477-row Core paired bundle is
`.work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2/`.
Its candidate target moves from `unexpected` to `expected`, while the oracle
target remains `expected`; all other rows are preserved. Candidate is
**4,538 expected / 53 unexpected / 11 skipped / 43,875 not run** (SHA-256
`b69852a0f582958f563cecdf055bda37373a3e222378fb4efdb1cea98ac79824`);
oracle is **4,285 expected / eight unexpected / ten skipped / 44,174 not run**
(SHA-256 `4198bf312aea62aada3e5f99f0eeb2daca13c980657d01fb81c1141a915b08a8`).
Its original verifier now rejects the evolved live package source, but a
separate [UA snapshot seal](../.work/official-conformance/outcomes/core-cdp-default-user-agent-strict-paired-v2-seal-v1/attestation.json)
(SHA-256 `e19b9a8821981cb6f9179de210a43696d34b7c2cf3ccced9ee381a5574370cff`)
reverified the unchanged UA sidecars and ledgers against the same frozen
capture-time candidate source used by the later error-slice seal.
The selected default-off Rust-owned UA composition is
`official-conformance-page-cdp-default-user-agent`, with release addon at
`packages/pwright/native/official-conformance-page-cdp-default-user-agent/pwright.node`
(SHA-256 `92f039f5530075841ca7d59ff523c1f6b7a5ec4884a29ff20042ef3c95826fb2`).
Focused Rust tests pass 7/7, real N-API boundary tests 4/4, both selected
and default browserless Node suites 788/788, strict TypeScript and release
checks pass, and the normal Cargo package closure adds zero names to the
prior Page-plus-headers composition (419 versus 419). These local checks do
not extend the single official Core result to full conformance.
The post-fix browser-free Core CDP error slice reran the unchanged official
`connect-over-cdp.spec.ts` assertions at lines 399, 411 and 572 against a
frozen 66-file Pwright package snapshot and an independent Playwright 1.58.0
oracle. Both lanes pass **3/3**, with zero skip or retry. The strict v3
[candidate](../.work/official-conformance/results/core-cdp-error-slice-strict-v3/candidate-v1.strict.json)
and [oracle](../.work/official-conformance/results/core-cdp-error-slice-strict-v3/oracle-v1.strict.json)
sidecars pass independent verification; the source snapshot manifest hashes
to `c90f2f9d9b7904610abfeb3a37ee720e988b37748bedf8849cf36c7658534115`.
The [versioned snapshot seal](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3-seal-v1/attestation.json)
(SHA-256 `87d852a37096b7b287c7c9c3090f4d46704deb8ffd8e2de93eb70602f4143841`)
replays the unchanged evidence without relying on the mutable live addon
path; five tamper/restoration controls pass.
The new immutable [paired promotion receipt](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3/promotion-receipt.json)
has SHA-256 `1f65ad5e2d26ec874fa7c386439c2ef28be58335233cfd5bf4d1f397356d555d`.
Each ledger still has 48,477 rows. Only the three target rows change per lane;
the other 48,474 are byte-identical to v2. The [candidate ledger](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3/candidate.jsonl)
is **4,541 expected / 53 unexpected / 11 skipped / 43,872 not run**
(SHA-256 `400f1e29f84da3f83076bd97d89ebcf76f20ae4cec48e5c518337fb1f7c56736`);
the [oracle ledger](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v3/oracle.jsonl)
is **4,288 expected / eight unexpected / ten skipped / 44,171 not run**
(SHA-256 `4c4a397cd098fd28e42b4e579217d808a034b8657a21e09228a1a7fd9dec844b`).
All three rows retain `implementation-internal` classification and the
receipt records `publicApiParityEvidence=false`. The opt-in
`official-conformance-page-cdp-errors` release addon has SHA-256
`0d7dcb931c42d5e912b594ec20fd12e202cc76590c7cab93187fe6829388f5c6`;
host Rust tests pass 14/14 plus two close-diagnostic tests, selected and
default browserless Node suites pass 791/791 each, native load and strict
TypeScript pass, and the normal addon package-name closure is 287 versus 287.
This is a narrow browser-free internal slice, not full Playwright conformance.

The historical red v2 Core CDP error slice runs three unchanged official
`connect-over-cdp.spec.ts` assertions at lines 399, 411 and 572. Candidate
genuinely fails all three; the independent Playwright 1.58.0 oracle passes
all three. The worker-restart-aware strict v2 sidecars are
[candidate](../.work/official-conformance/results/core-cdp-error-slice-strict-v2/candidate-v1.strict.json)
and [oracle](../.work/official-conformance/results/core-cdp-error-slice-strict-v2/oracle-v1.strict.json).
Only the fixture's same-port loopback host is adapted to the full Tailscale
MagicDNS name; no assertion was weakened and no Chrome was used. The v2
immutable paired bundle has a [candidate ledger](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/candidate.jsonl),
[oracle ledger](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/oracle.jsonl),
and [promotion receipt](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2/promotion-receipt.json)
(receipt SHA-256 `92c6720100cfbecdef6ad36e3bd8fdaf3f327251b4200f83dafeb0ef893e4cb4`).
Each ledger retains 48,477 IDs; only the three previously `not-run` rows
change per lane, and all other 48,474 rows are byte-identical to the UA v2
predecessor. Candidate is **4,538 expected / 56 unexpected / 11 skipped /
43,872 not run** (SHA-256
`c6304c593856a8a94b24ae173ea385609016139989dd079fc4ef4878f811facd`);
oracle is **4,288 expected / eight unexpected / ten skipped / 44,171 not run**
(SHA-256 `6478b726429f40bec7d8c22f943442c57345f246070f2c384386ba33cff376d2`).
The existing `implementation-internal` classification is preserved, and the
receipt records `publicApiParityEvidence=false`: these three genuine
discrepancies do not establish full Core or public API parity.
The original v2 verifier compares candidate provenance with the *live*
`packages/pwright` source, so it now rejects that historical run after the
next opt-in profile changed `package.json` and `src/native-artifact.cjs`.
An independent [snapshot seal](../.work/official-conformance/outcomes/core-cdp-error-slice-strict-paired-v2-seal-v1/attestation.json)
(SHA-256 `65e96b04c6e1cc0edcad15c63dd4ae67ecb4fca9441716a390de52016fef6d7b`)
preserves the exact capture-time package source digest
`041a4d60f4901b5f89ffbff492fa13c33e41f6c1ee695c1c81d120e6dba4f03a`
and native artifact. Its [versioned attestor](../scripts/attest_playwright_core_cdp_error_slice_strict_v2_snapshot.py)
reverified the unchanged official reports, sidecars, both ledgers, and receipt
against that snapshot; four negative controls pass. The post-fix v3 run above
uses a frozen candidate package snapshot.
Evidence-tooling quality remains open too: the new Python connect and first
Core header strict helpers are not yet in the CI Ruff file list, and a manual
check under the Python package's stronger Ruff configuration reports style
issues in those hash-pinned files. They cannot be reformatted in place
without invalidating their sealed sidecars; version and recapture them before
adding that gate. The later Core URL-query helpers passed the pinned Ruff
check and format before capture.
The separate browser-owner seal-v1 attestor has manual Ruff `EXE001`/`B023`
debt; its immutable self-hash requires versioning and resealing rather than an
in-place edit. The UA v2 promoter passes Ruff lint, and its 21 strict
verifier/promoter tests are in CI with a portable upstream-checkout fixture.
Manual `ruff format --check` still flags one split SHA literal in the sealed
promoter, which is deliberately omitted from CI's format list until a
versioned recapture.
The new Python dummy-engine evidence helpers have a separate manual Ruff debt:
the five runner/verifier/promoter and unit-test files collectively report
`EXE001`/`I001`/`F401`. Their hash-pinned capture sources cannot be edited in
place without invalidating the eight sealed results; version and recapture
before claiming a green lint gate for this shard.
The full 39-feature Node Page-plus-CDP-headers composition now also has a
release native artifact at
`packages/pwright/native/official-conformance-page-cdp-handshake-headers/pwright.node`
(SHA-256 `dfe9607df2c7cfa16d304338e88eb2fa01302b4736e4116c0c67b655a3482769`).
It passes native load 1/1, real N-API header boundary 3/3, strict TypeScript,
and the selected and default browserless package suites (783/783 each). Its
normal Cargo package closure is the same 338 packages as the Page base
(zero added). These are local checks, not official Core test passes.
Separately, the opt-in Node `cdp-handshake-headers` profile now has a release
native artifact at `packages/pwright/native/cdp-handshake-headers/pwright.node`
(SHA-256 `82b11e24df70eec254aaa4e751dfa978422265a9ae517932eb89c781160fabc9`).
It passes native-load 1/1, facade adapter 10/10, and real N-API boundary
3/3 without a browser; its selected normal dependency closure adds zero
packages. This standalone artifact does not contain the full Core Page
composition and has no official Core result.
The preserved red v1 paired Python ledgers each retain all 6,526 pinned IDs:
candidate **1,529 passed / 109 failed / three errors / 11 skipped / 4,874
not run** (SHA-256 `dd0609e85a704f97c55762587126c0790d80a444bc116cc9f7e3f55c8d0efb9d`)
and oracle **1,303 passed / four failed / 11 skipped / 5,208 not run**
(SHA-256 `f15b5292a64e27607bc632074fcac26457f879decf569ff8eb616435588e9b95`).
The immutable bundle at
`.work/official-conformance/outcomes/python-dummy-engine-abi121-strict-paired-v1/`
adds only four previously `not-run` Firefox/WebKit async/sync dummy-executable
launch cases per lane. The unchanged official assertions give oracle **4/4**
and ABI121 candidate **0/4**: each candidate call fails with
`UnsupportedError` before spawning the dummy asset, while each oracle call
launches the pinned Node asset and observes exit code 1. These are genuine
candidate failures, not passes or skips. Eight fresh `*-v2.provenance.json`
sidecars under
`.work/official-conformance/results/python-dummy-engine-abi121-strict-v1/`
bind the ABI121 wheel, public call, source and process receipts; the paired
promoter and read-only bundle verifier pass. No Chrome was started for this
shard. The preceding connect-close successor remains intact.
The preceding browser-free Python slice was the unchanged async
`test_browsertype_connect.py::test_should_print_custom_ws_close_error` for
Chromium, Firefox and WebKit (three expanded IDs; pinned source SHA-256
`d47290f5acb45246ce2bae150d3c583d340806821b660be08d798ac3572e7b11`).
Six source-pinned, independently verified official runs on the ABI121 wheel
record candidate **0/3** and oracle **3/3**. All three candidate failures are
the genuine missing `BrowserType.connect` method, not a fixture substitution.
This uses Playwright Server's wire protocol rather than CDP and must not be
redirected to `connect_over_cdp`; the [pinned protocol map and implementation
gates](knowledge/pwright-playwright-server-connect.md) record the required
independent connection and object lifecycle. The six strict sidecars are at
`.work/official-conformance/outcomes/connect-close-strict-{candidate,oracle}-abi121-{chromium,firefox,webkit}-v1.provenance.json`.
The strict promoter published and read-only verified the connect-close paired
successor at
`.work/official-conformance/outcomes/python-connect-close-abi121-strict-paired-v1/`:
only the three target rows changed in each lane;
all other 6,523 rows are byte-identical to the predecessor. Candidate
failures remain failures, and the previous CDP WebSocket bundle is intact.
The predecessor ABI119 dummy-node bundle remains intact and is historical
installed-wheel evidence for its two separate tests.
With the ABI119 candidate wheel installed, fresh Python 3.12.14 candidate and
official-oracle environments each collect all **6,526** pinned Python cases;
four paired CDPSession/tap manifests collect the same 21 selected IDs with
matching manifest digests. These reports are collection only, not browser
execution or strict-ledger pass credit. The earlier ABI119 wheel's embedded
README metadata predates its post-build verification claims, although its
packaged Python facade files matched the source at that checkpoint; that
historical metadata caveat does not negate the separately verified ABI121
transport wheel. Node now has
an opt-in Browser-root CDP session source implementation with bounded event
ordering for `Browser.newPage` and `BrowserContext.newPage`. The target fence
also honors CDP discovery filters that include Page targets, using
first-match-wins semantics; post-create event-flush failures compensate the
newly created Page or Context before rejecting. The selected release host
check and test compilation pass, and the focused CDP session tests pass
32/32. The separate topology-driver test target did not finish: its release
test compilation pushed swap from about 777 MiB to 1.4 GiB and was stopped.
It exposed a test-only ownership compile error, now corrected in source but
not recompiled. A later `cargo check --release --lib --tests` retry also had
to be interrupted: the test-analysis Rust process reached about 6 GiB RSS,
free/reclaimable system memory fell to 37%, and swap rose to about 1.88 GiB.
The corrected topology test target still has no completed compile or run.
Its earlier combined native artifact does not contain this new feature, and
no official Chromium session case has been run on the new source. The normal
Node browserless package list passes 773/773, including four new cleanup
cases; strict TypeScript checks pass. The full
`cdp-session`/`browser-cdp-session` Node dependency script passes on the
current source: both selected release checks and zero added normal packages
relative to the base addon. A combined native artifact still must be rebuilt.

The Python addon source passes selected release checks for `cdp-session` and
the ABI121 WebSocket transport features, with base-versus-selected normal
dependency comparisons. The full feature-by-feature dependency script remains
incomplete after memory-pressure interruptions.

The next paired browser shard is the pinned Python Chromium CDPSession files:
six async and five sync cases, each locked by its full-file manifest. The
existing strict ledger has candidate failures and oracle passes for all 11;
the ABI119 wheel has only collected them. A rerun on that wheel must be
labeled as wheel-specific evidence because it predates later shared Rust
source edits. The dedicated loopback CDP endpoint currently accepts a socket
but does not answer `/json/version`; restarting that separate Chrome process
requires user confirmation. No browser ledger row advances before the paired
run and provenance check.

The Python conformance/provenance harness passes 101/101 focused tests in
the pinned Python 3.12 official-wheel environment; the dummy-node diagnostic
and strict-ledger unit checks pass 10/10. Four selected Python manifest
checks pass 11/11, and the Core provenance/ledger unit checks pass 64/64.
The older dummy-node diagnostic remains non-promotable: its schema-1 receipts,
diagnostic-only scope, and stale native-artifact hash provide no strict credit.
A separate ABI119-specific policy now pins the capture runner and guard. The
unchanged official Chromium async/sync launch-failure tests each passed 1/1 in
candidate and oracle lanes; four sealed provenance files independently verify
the exact public `BrowserType.launch` call, sandbox exec/network denial, and
the pinned dummy Node process exiting with code 1. Candidate reported `Error`;
the oracle reported its `TargetClosedError` subclass, both satisfying the
official `pytest.raises(Error)` assertion. The paired promoter required all
four reports, revalidated evidence before exclusive atomic publication, and
changed only lines 2411 and 5627 in each 6,526-row ledger; adjacent Firefox
and WebKit variants and all other rows are byte-identical to their bases.
Its 27/27 negative/positive tests and the capture/verifier's 34/34 tests pass;
Ruff check/format pass. The four `*-first` diagnostic attempts were not used.
The pinned async Chromium `test_browsertype_connect_cdp.py` header-handshake
and custom-WebSocket-close tests (source lines 92-117) now have strict ABI121
candidate/oracle evidence and paired ledger credit as described above. Earlier
ABI119 candidate attempts failed 0/2 because the wheel lacked `headers`;
ABI120 then passed custom close but timed out on the header test when the
fixture sent a non-JSON text frame. Neither earlier attempt was promoted.
The ABI121 path fails fast on that invalid frame, preserves the custom close
reason, and satisfies both unchanged official assertions. These two
browser-free tests do not establish browser-backed CDP or full-suite parity.

The four strict Core `$eval`/`$$eval` `*`-capture failures now have a
source-level typed Rust selector plan. It retains the capture boundary so
later filters and descendant Locator composition apply to the post-capture
suffix before materializing the captured ancestor; this corrects an
audit-found wrong-element result in the first draft. Selected release host
source and test compilation pass with Browser CDP, handles, and selector
registration; formatting passes. Two further source-audit discrepancies were
fixed: a leading `visible=true` post-capture part now filters the captured
element, and the `*text=` quote tokenizer follows the pinned upstream split
rule. The focused selector module passes 91/91 release tests, the CDP
session module passes 32/32 release tests, and the selected release Node addon
check and strict host/addon Clippy pass after these changes. The Node
`cdp-session`/`browser-cdp-session` dependency proof still reports zero added
normal packages. The existing capture tests inspect selector
plans and generated JavaScript, not live DOM results. No rebuilt native
artifact or official browser result exists for these fixes.
Pinned Core source review found three capture cases still outside this typed
plan: `selectors-css.spec.ts` needs post-capture relative parent XPath
(`../..`), while two `selectors-frame.spec.ts` cases need the upstream
capture-before-frame error and frame-aware capture-after-enter routing.
The selected `eval-on-selector*` cases have structural support only; the
`selectors-css`, `selectors-text`, `selectors-frame`, and `locator-query`
capture files are not yet locked as complete-file run manifests.

Python ABI116 source and packaging checkpoint: the complete pinned async/sync
Chromium `test_tap.py` files contain ten cases (seven async, three sync), with
Page, Locator, ElementHandle and raw Touchscreen entry points. The two opt-in
addon features `touchscreen-input` and `element-handle-tap` now bind those
operations to the already verified Rust host tap paths. The repaired 105-feature
wheel SHA-256 is
`2473eae420bc850e5981643636405c84e1fab64b414960cabf474e1725146095`;
fresh Python 3.9.6 and 3.12.14 installs each pass 535/535 browserless package
units and `uv pip check`, and the minimal-feature release Rust addon suite
passes 44/44. The new capability adds exactly the three opt-in touchscreen
Pwright packages; retained tap adds no additional package. No unchanged
official browser case has been rerun on ABI116: the strict Python ledger and
its earlier 0/10 candidate versus 10/10 oracle tap evidence remain unchanged.
The full `scripts/check-python-addon-dependencies.sh` feature-by-feature
release check passes, as do full-profile strict release Clippy and rustfmt.
Whole-project strict Pyright passes with zero errors after typing-only repairs
to the offscreen-favicon fixture and three older unit files. Ruff check and
formatting pass across Python source and tests. The unchanged ABI116 wheel
passed 535/535 units on both Python 3.9 and 3.12. The new default-off
`cdp-session` owner, exact auxiliary topology discriminator, and Python FFI
pass nine focused release host tests and four release addon lease/ABI tests.
ABI118 retains cleanup ownership from host delivery through Python wrapper
registration; the frozen 106-feature wheel at
`.work/official-conformance/builds/python-abi118-cdp-final2-Aod8pr/`
has SHA-256 `4c747a33fd5bcca04bd7eee183cf07263a52f455f5a6f3a5e84c2532e321f3e3`.
Fresh Python 3.9.6 and 3.12.14 installs each pass 547/547 browserless units
and `uv pip check`. The feature-by-feature dependency script proves the same
108-package base closure for `cdp-session`; full-profile strict release Clippy,
Ruff, and whole-project strict Pyright pass. Two source-identical wheel builds
have equal payloads after normalizing only CycloneDX build identity fields.
The later ABI119 source change, subsequently packaged into a wheel, retains
late attach responses, replaces
the destructive Python event drain with per-event acknowledgement and rollback,
parks a generation-only watcher for idle events, and compensates an unclaimed
child after Page retirement. Focused release host and addon tests pass 25/25
and 27/27, with selected-feature strict Clippy. The current full 106-capability
ABI119 wheel (SHA-256
`86d1ff028301b4bd42f9f2a3a39b8cef172217e8a9600e1fcf0517c7b38b1254`)
passes 571/571 browserless installed-wheel tests and `uv pip check` in each
fresh Python 3.9.6/3.12.14 environment. A separate Node N-API and facade
bridge has a combined release artifact (SHA-256
`958155e45ca6162e05f2d6f13d4a13f6d76953af7658d8b1677bc6ce8c68ec9a`),
native-load smoke, 763/763 browserless tests, strict types and a zero-added-
package dependency proof. The pinned complete Core
`tests/library/chromium/session.spec.ts` file has 12 cases and a new
source-pinned `chromium-library` manifest. Browser-free official runner
collection selects all 12 tests in that file; this is not an execution result.
The Core candidate fixture calls `context.addInitScript` before every case,
so the standalone `cdp-session` profile cannot run this file. The older
`official-conformance-page-cdp-session` artifact passed native-load
verification before the new Browser-root feature was added; a rebuilt artifact
and browser execution are required. All 12 Core session IDs are still
`not-run` in both strict ledgers. The 11 Python CDPSession IDs and ten
Python tap IDs already record candidate failures versus oracle passes; a
successful rerun would replace those outcomes, not add first-time coverage.
The remaining known public-surface gap in that file is `Page.coverage` domain
toggling. Browser-root CDP session support is source-implemented but remains
unverified against the unchanged browser-backed cases. The other cases also
require that run before any parity claim.
No CDP-session or selector-capture strict-ledger row changed.

The pinned `Page.add_locator_handler` async/sync files also contain 13 cases
each. Source audit confirms mouseover/pointerover overlays arise *inside* the
Rust click/hover retry transaction, while visibility assertions have a
separate Rust poll loop. Full 26-case parity therefore needs a Page-owned
bounded handler registry and resumable native checkpoints before and after
pointer movement, plus a callback request/ack bridge; a Python-only precheck
cannot satisfy the unchanged tests. Its existing 0/26 candidate and 26/26
oracle result remains unchanged by this audit.

The latest browserless source checkpoint repairs the typed CSS `:visible` and
`:nth-match` parser's casing, CSS-number, and comment edges and changes the
selector runtime to Playwright's recursive Shadow Root CSS traversal and
composed DOM union order (runtime version 14). The focused release host and
runtime lanes pass 24/24 and 12/12, and the complete configured Python
103-capability release Clippy gate passes with warnings denied. These source
checks have no official browser-test credit; the dedicated CDP endpoint is
still unresponsive and has not been restarted without approval.

Latest source work adds Python `ElementHandle.select_text`, Node retained
Handle scroll timeout inheritance, `Page`/`Frame.waitForSelector`, and an
independent `ElementHandle.waitForSelector` leaf.
The lean Node scroll addon has a fresh native-load proof (SHA-256
`60d757a38a958732aff254e87bf92e9d1af2e7be34b301f795c07f312b338d3c`),
and its browserless Node suite passed 707/707 at that checkpoint. The earlier
stable Python ABI114 wheel
(SHA-256 `4850418950bac4dfe1fef27d34042d3b030345d2f0a32c030bcfb40f4c41b592`)
passed 518/518 browserless installed-wheel units on each of Python 3.9 and
3.12. The earlier repaired ABI114 wheel (SHA-256
`d66b803fbd03480657ba24b60b5c6953fa45d9ffa3bb516768defea3bcd4e5a0`)
exports the same 103 capabilities and passes 518/518 installed-wheel units and
dependency checks on each fresh Python 3.9.6 and 3.12.14 install. A
three-capability trial wheel was rejected because a command-line
`maturin --features` override replaced, rather than extended, the
`pyproject.toml` feature list. The new repaired ABI115 wheel (SHA-256
`5856bc486b4bc342ceb82a251de79779a20b4b5b2094c0bbdbe028883fda8e54`)
selects 103 capabilities including `element-handle-click`, loads from fresh
Python 3.9.6/3.12.14 site-packages, and passes 528/528 browserless units and
dependency checks in each environment. An initial ABI115 build selected only
102 capabilities because the configured feature list omitted the click leaf;
it was rejected and the list corrected before this wheel. The cumulative Node
`official-conformance-page-retained-wait` addon (SHA-256
`905b9b7dd8bc171e0aab2d401899730da640854d7f10d22084d6ac6dc676c533`)
passes native-load smoke, 735/735 package units, strict types, and dependency
boundaries. Its environment-wide long-wait admission guard reserves eight of
32 FFI Command slots: 24 simultaneous selector waits are allowed, and the
25th fails immediately. Focused release tests pass 3/3, and both focused and
selected full-profile strict release Clippy pass; browser-backed liveness
remains pending. XPath Browser-plan waits and atomic
state-to-Handle publication are implemented but not browser-verified;
detached-Frame CDP context-loss variants now map to the expected Frame error
only when detachment is confirmed, also without browser verification.
Playwright CSS `:visible`/`:nth-match` now have a bounded typed parser with
63/63 focused host release tests, but no official browser verification;
Registered/Layout selector waits and full element-preview diagnostic history
remain open. None of this source-only work changes an
official-browser ledger row. At that earlier checkpoint, the dedicated
loopback CDP endpoint was unreachable; the later source-bound browser runs
above used a verified loopback-only dedicated Chrome process.
Python `Locator.click(force=...)` now forwards through the existing Rust click
options with 12/12 focused release addon tests and 18/18 focused facade tests
on each of Python 3.9 and 3.12. The full-feature ABI115 wheel above is rebuilt,
and the 26 official locator-handler cases still fail at the absent handler
API; these checks add no browser pass credit.
At this earlier checkpoint, the complete pinned `selectors-misc.spec.ts`
Chromium-page manifest collected 21 unique public-API-candidate invocations
from one upstream file, with no grep or exclusions. Its browserless manifest
checks passed 2/2; the later complete browser run and its 10/21 candidate
versus 21/21 Oracle outcomes are recorded at the top of this document.
The opt-in retained `ElementHandle.selectOption` string-value slice has 5/5
focused Rust release tests and a cumulative release Node addon with SHA-256
`bae00851f127108b003264f46a4ea8982a9ee4ad1b3de1ab3b70ef2335faaa06`;
native-load smoke passes. Its descriptor/Handle overloads and navigation
commit behavior remain absent, and the complete official `elementhandle-misc`
file has not been rerun.

Earlier source-bound strict ledgers, before the ABI119 dummy-node and ABI121
WebSocket promotions: Python candidate **1,525 passed / 102
failed / three report errors / 11 skipped / 4,885 not run** out of 6,526 (SHA-256
`5c0f4078382dfcdbc0b9ed5b6e90ee7e3b0c4922755eff23b54f69f3b9023dfa`),
oracle **1,292 passed / four failed / 11 skipped / 5,219 not run** (SHA-256
`ed44f66be349a527e682aa2648fc7bf61a8a527af839411f45bbc606460f2a72`). The
unchanged 36-case Chromium async `Page.evaluate` file passed **36/36** on
its installed candidate wheel and **36/36** in the independent oracle;
the preceding frozen `Page.select_option` wheel passes **23/24 async and 22/23 sync**
versus official **24/24 and 23/23**. The latest complete Chromium popup file passes
**20/26** versus official **26/26**: first-request extra headers, two user-agent
inheritance cases, and context routing now pass. The complete click file passes **69/70**
in both lanes on the original fixture and **1/1** in both on an explicitly
versioned favicon fixture. The complete Python async and sync
`ElementHandle` state-wait files are **22/22 candidate and oracle**. The
complete async and sync Chromium `Page.add_locator_handler` files are **0/26
candidate versus 26/26 oracle**, all candidate failures on the absent public
method.
The complete async/sync BrowserContext request-intercept files now pass
**14/14 candidate and oracle** on a rebuilt 100-capability wheel; the source
fix retains a post-`route.continue_()` handler exception for the next
`Page.goto` only, not yet every public command. Paired Python pass evidence
is **1,234** IDs.
The new complete Python async/sync Chromium `tap` files are **0/10
candidate versus 10/10 oracle**; `CDPSession` files are **0/11 versus 11/11**.
These missing opt-in capabilities are counted as failures, not exclusions.
The new ABI 109 Python wheel has 100 selected capabilities, bundled OpenSSL
libraries, and **470/470** browserless units. Its complete async Chromium
`test_element_handle.py` candidate file records **13 passed / 44 failed / two
report errors** out of 59, with verified source-bound provenance and exactly
59 intended strict-ledger row changes. Its
corresponding oracle full-file attempt was interrupted when shared Chrome CDP
stopped responding, so there is no paired full-file oracle result; the later
DOM preview patch has gained no strict-ledger pass credit.
The subsequent repaired ABI 110 wheel also selects 100 capabilities, bundles
OpenSSL, and passes **477/477** installed-wheel browserless units; it has not
yet produced any official browser result.
Current source advances Python to ABI 113 with retained ElementHandle DOM
reads, owner-Frame lookup, and opt-in scroll-into-view. Its repaired wheel
(SHA-256 `f423668c31b274752dffc2a57c25f38cbf7e0fcd7852e53f56dc4cd4ba35424e`)
passes **508/508** installed-wheel browserless units with 102 selected
capabilities. Node adds an independent retained-scroll leaf and a separate
full-Page conformance profile. The release addon
(SHA-256 `86500fadb4e1a1a6ba885fb031e0766c8e01be28bea8ba080e69e973d882ec73`)
passes native-load smoke and 208 focused contract tests. Neither new build
has run against pinned official browser files. The dedicated loopback Chrome CDP
endpoint remains unreachable; no new browser pass credit is assigned while
it is unavailable.
At this earlier checkpoint, Core candidate had **4,531 expected /
56 unexpected / 11 skipped / 43,879 not run** out of 48,477 (SHA-256
`b1e8c83af4add22cab75f6876dedac3911c12d5ee796871083640c3555366e5c`),
and oracle had **4,278 expected / 11 unexpected / ten skipped / 44,178 not run**
(SHA-256 `c004ad9153787b5a00cb78aa00b1f8191d6c87452e1171ec2b0f4aa95fb6d2c7`).
Each now has 2,610 explicitly tagged upstream-private, browserless source-only
rows: 18 earlier Core455 rows, 2,556 Core460 rows, 25 Core461 image-tools
rows, eight Core462 MCP config rows, two Core463 fit-to-width rows, and one
Core464 Android glob row. They carry
`candidateRuntimeRequired=false` and are **not** public API
compatibility evidence. The first paired route browser rerun improved seven
candidate public-behavior outcomes; the later 47-feature candidate rerun
improves three more, with no classification change. Its first attempt was
refused at preflight while another task occupied the dedicated CDP Chrome;
the valid rerun completed after Chrome became free. The complete Python
`Page.select_option` async/sync pairs are now in the strict ledgers.
At that historical checkpoint the complete Core Chromium `Page.fill`
candidate rerun was **44/45**,
`Page.route` is now **53/55** including one failure and one skip, and the
opt-in `selectors-react` candidate is **57 expected / three skipped**, matching
a now-complete, current-runner independent oracle **57 expected / three skipped**
for exactly the same 60 IDs. The next complete Chromium
`eval-on-selector` file is **13/26 candidate versus 26/26 oracle**; the
complete companion `eval-on-selector-all` file is **6/9 candidate versus
9/9 oracle**. The complete `page-wait-for-selector-1` file is **0/17
candidate versus 17/17 oracle**. The latest
complete Python async popup file is **17/26 candidate versus 26/26 oracle**.
See the verified receipts below. Neither inventory is close to complete
execution.

The generic Core and Python `--result` ledger entrypoints now require current
source-bound provenance before promotion and bind the bytes actually parsed to
the verified result digest. Missing, altered, cross-lane, collection-only, or
bootstrap-only reports cannot gain strict-ledger credit. The separately
labeled source-only paths also bind parsed report bytes to the digest returned
by their verified receipts. The generic source-only path additionally rejects
cross-lane promotion and unadjudicated public inventory rows; exactly 441
Core460 event invocations per lane are allowed only by the pinned
invocation-level private-source audit. These gates add no pass.
Actual 55-case Core and 47-case Python promotions through the new gate
reconstructed their existing strict successors byte-for-byte; the focused
provenance and new manifest regressions pass 116/116 in the pinned
official-wheel Python environment. A transient environment without that wheel failed its
environment-identity prerequisites and produced no conformance result.

The ABI 110 browserless wheel, ABI 109 complete-file diagnostics, independent favicon fixture
pair, provenance hashes, and audited 6,526-row strict successors are recorded
in [the Python conformance log](python-official-conformance.md#abi-110-preview-and-bounding-box-browserless-checkpoint-2026-10-11).

## Complete Core ElementHandle misc file (2026-10-11)

The pinned public Chromium-page `tests/page/elementhandle-misc.spec.ts`
file ran unmodified in independent lanes: **one expected / nine unexpected**
candidate results on the frozen `official-conformance-page-goto-burst-20`
native profile, versus **10/10 expected** in the official oracle. Seven
candidate failures report absent ElementHandle methods (`hover`, `focus`,
`fill`, `check`, `uncheck`, `setChecked`, `selectOption`); two fail
while querying a handle because `Node` is unavailable in the evaluation
world. The candidate JSON/provenance SHA-256 pair is
`9c5f66aafe8a8d5c2b60b87c385dd516c1cbb944a96d3577eea43edd4653da24` /
`421b7de7f1281ce46022ca48c444539fdba96c54529153917cc6cfb0cad47eba`;
the oracle pair is
`751bf19f4371afac7cc1d000aae2ca54e061c97bf80110a9b84c1c0f2ed1cfac` /
`ca42d3cf9cb4a497d7b48e96fef71c6e7604465c2ce9cc79efcf2a307d8a14f6`.
Both source-bound receipts verify. Strict successors
`.work/official-conformance/outcomes/playwright-core-candidate-elementhandle-misc10-20261011-v1.jsonl`
(SHA-256 `5a9a29a3713f0047a2aa179f4da0c1a768ba77331dd721d8bbdc431dc0b34acb`)
and `.work/official-conformance/outcomes/playwright-core-oracle-elementhandle-misc10-20261011-v1.jsonl`
(SHA-256 `069bb83ac37346ce4000485d5bc16d7832d3712cecb070746c91e33402420dfc`)
change exactly the same ten previously not-run Chromium rows. All other
48,467 rows remain unchanged. No public-failure exclusion was made.

## Core464 private Android glob source-only checkpoint (2026-10-11)

The pinned `tests/page/interception.spec.ts:73` Android-page invocation
`should work with glob` calls an upstream-private URL-match helper. Independent
candidate-labeled and oracle-labeled no-I/O executions each pass **1/1**;
neither loads Pwright or a browser. The exact source-bound manifest is
`tests/official/playwright-core-android-glob-source-only-v1-manifest.json`.
The candidate result/provenance/guard SHA-256 values are
`e6ef72fbcddc802607568046f73b08f062bcfa8a17823a8e6c601fec433de6ce` /
`2a14c45b0a3a39e77e5c6b41422e743a8ff073cc5e348b6708f151d7fa56d7a1` /
`2621afa0df5f332f4718ff80c44392dd12efa493d6bc02af082d701f2ef4fda9`;
the oracle values are
`956db2412f19afcc65a546af2d6560d9d582239e3b0be35b1dac71fc008aba8d` /
`279d00209815a24661ae655d2e2056dba53e5016a043e507df56e4f287d968c3` /
`7fa4b3ce7ffb65adc8da34e8026cb5df6187580919f84747584a98e66c2ec8d0`.
The strict successors
`.work/official-conformance/outcomes/playwright-core-candidate-android-glob-source-only-20261011-v1.jsonl`
(SHA-256 `b1e8c83af4add22cab75f6876dedac3911c12d5ee796871083640c3555366e5c`)
and
`.work/official-conformance/outcomes/playwright-core-oracle-android-glob-source-only-20261011-v1.jsonl`
(SHA-256 `c004ad9153787b5a00cb78aa00b1f8191d6c87452e1171ec2b0f4aa95fb6d2c7`)
each change exactly that one previously not-run row. All 48,477 rows audit
cleanly, and nine negative verifier checks pass. This is expressly
`evidenceScope=upstream-private-source-only` with
`candidateRuntimeRequired=false`, **not public API or runtime compatibility**.

## Core463 private fit-to-width source-only checkpoint (2026-10-11)

The pinned `tests/playwright-test/fit-to-width.spec.ts` contains two
`playwright-test` project invocations that exercise an upstream-private
reporter helper. The exact no-I/O guard, source set, manifest, runner and
JSON receipts are fixed by
`tests/official/playwright-core-fit-to-width-source-only-v1-manifest.json`
(SHA-256 `2f32944cd8c2a971d2bae832b805d4fa1e11875f4730cc431b6728a2223c2785`).
Independent candidate-labeled and oracle-labeled executions pass **2/2**
each, but neither loads Pwright; `candidateRuntimeRequired=false` and
`evidenceScope=upstream-private-source-only` remain explicit. These rows are
product-infrastructure coverage, **not public API compatibility**.

The exact-byte verified candidate report/provenance SHA-256 values are
`1dd7da296164a639b7a20df6d216d8f659d4dbe9d04f43973eb77642ad30a723` /
`7b7a43a0de0eba2f3eab4f4d91841440553a675d78b135d6001cfbe5f29ccdeb`;
the oracle pair is
`0c6a0a73b0b5d489b2fe0eae277f2837d37a5aa0c490b2aea62c4a9691545685` /
`4e64f553e8e2f645d181ce4f92ddb3776f44253d40bfe04272fb27fbd059bd15`.
The strict successors are
`.work/official-conformance/outcomes/playwright-core-candidate-fit-to-width-source-only-20261011-v1.jsonl`
(SHA-256 `017fc15553e0358f1f6495cfb614274e886451001419d91f17d53f853a24d271`)
and its oracle counterpart
`.work/official-conformance/outcomes/playwright-core-oracle-fit-to-width-source-only-20261011-v1.jsonl`
(SHA-256 `c08dd5c73b61b12d9961bd642eff1097c98e0f4da61c1cdc0a017df860a4708e`).
Both derive from the then-current `elementhandle-misc10` strict ledgers,
changing only these two previously not-run rows and retaining all later
browser outcomes.

Source-only follow-up: the opt-in Node `ElementHandle.focus()` facade now
forwards its retained native owner to the existing Rust exact-Handle focus
operation. It accepts no options yet and rejects supplied options explicitly.
The Node package tests pass **693/693**, strict TypeScript and selected-addon
release `cargo check` / Clippy pass, and three focused facade tests cover
exact-owner forwarding and fail-closed behavior.
Separately, retained click now retargets a Text node to its current parent
Element inside the Rust actionability and both route checks, while keeping
all three CDP calls bound to the original object ID. The focused backend
release suite passes **146/146**. Neither change is in the frozen Node addon
above; no official Core or Python row has been reclassified or credited.

## Complete eval-on-selector and async popup pairs (2026-10-11)

The pinned upstream Core `tests/page/eval-on-selector.spec.ts` Chromium-page
file ran unmodified in both lanes. The independent oracle passed **26/26**;
the frozen 47-feature Rust-backed candidate passed **13/26**, with 13 explicit
unexpected outcomes. Both schema-2 receipts verify against the same pinned
source and 26-case manifest. Oracle result/provenance SHA-256:
`17f20266a03b7ccb9f9b98ef51dba3aabd82e53b172686a32f4f93ad337253be` /
`c885c79b8bfb0dada586a29491a7ed97ac8e3a5f52601ba252b81e12be2a7713`;
candidate result/provenance:
`2f24d70a30e0842bd84eff6a649874d15a544c2201fb64262a6c069879847ff5` /
`0284515fc0afede14ac39eb2727d9ebfadcbad37b7ca9ea5f166d308f9bac121`.
The strict successors are
`.work/official-conformance/outcomes/playwright-core-oracle-eval-on-selector26-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-core-candidate-eval-on-selector26-20261011-v1.jsonl`.
Complete 48,477-row audits show exactly 26 changes per lane: oracle
`not-run` to `expected` for all 26; candidate `not-run` to `expected` for 13
and to `unexpected` for 13. Candidate gaps span selector engines and capture,
ElementHandle arguments, no-match error parity, and the absent Node
`page.waitForSelector` facade; none is counted as passed.
A bounded source-only parser repair now covers the three exact-value
`data-test*=` engines and spaced `css =`, with all 53 selector-parser release
unit tests, workspace rustfmt, and diff checks passing. It is not in the frozen candidate artifact above and
earns no official pass until a rebuilt candidate rerun verifies it.

The complete pinned upstream `tests/page/eval-on-selector-all.spec.ts` file
adds nine Chromium-page public `Page.$$eval` invocations. The independent
oracle passed **9/9**; the same frozen candidate passed **6/9** and failed
XPath all-match plus two `*` capture cases. Both schema-2 receipts verify.
Oracle result/provenance SHA-256:
`a765b59ebdb060fb7c85b8fed7f72c6c14701b0abc0c6fee78822b0181fb9603` /
`74a37714cf421690d337439c8d26ed5dc584b1eedd250bdba12379d51e2ed731`;
candidate:
`9143fa5c533233c923a8c450b38389d72988581c176aaa996123c311f930e786` /
`db9fe0c285e81303ab231533126a3622ea78c735d83f8efe8a20de87cd34f1dc`.
The strict successors are
`.work/official-conformance/outcomes/playwright-core-oracle-eval-on-selector-all9-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-core-candidate-eval-on-selector-all9-20261011-v1.jsonl`.
Complete 48,477-row audits show exactly nine new results per lane: nine
oracle `expected`, and six candidate `expected` plus three `unexpected`.

The pinned upstream `tests/page/page-wait-for-selector-1.spec.ts` file adds
the complete 17-case Chromium-page public API selection, with no omissions or
source skips. The independent oracle passed **17/17**; the frozen candidate
failed **17/17**. Both schema-2 receipts verify against the same pinned file,
manifest, and runner. Oracle result/provenance SHA-256:
`36814ae0bb8c6ba6a9c2359493ea75dbd18ef407687d9537de90e87022924e76` /
`c6fb8b64eaf6bcbc209dfa05bbb11491f2590882fe41ed449f9592799d7689fd`;
candidate:
`7496ced6bbdc7b534375b6dd33d819fa5690661b898cf88811d60c6020e88b91` /
`43861e2032a4229962c6e8acb8d40ccc2ce5adaaa06469592ae8e6641021be0b`.
The strict successors are
`.work/official-conformance/outcomes/playwright-core-oracle-page-wait-for-selector-1-17-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-core-candidate-page-wait-for-selector-1-17-20261011-v1.jsonl`.
The full 48,477-row audit records exactly 17 `not-run` to `expected`
oracle changes and 17 `not-run` to `unexpected` candidate changes; all other
rows remain unchanged. Fifteen candidate failures lack Page, Frame, or
ElementHandle `waitForSelector` entrypoints; one has a click-visibility/log
mismatch and one closes the target during frame attachment. No candidate pass
credit is inferred from the oracle result.

The pinned upstream Python `tests/async/test_popup.py` Chromium file also ran
unchanged in both installed-package lanes, isolated at 30 seconds per case.
Oracle: **26/26 passed**; candidate: **16/26 passed**, nine failures, one
quarantined report error (a failed test plus teardown emitted two JUnit
records). Both schema-5 receipts verify. Oracle JUnit/provenance SHA-256:
`06a1dedb891d8d244a7276e793fafc3f6b70c5f0150a6b4ba3fecb6a9c0fe61a` /
`5e5b6796c17d9a0c9ae1ea795e1ff1ccb7f6b7b99d19fa473e93ce20a84741e9`;
candidate:
`49c159df02368d0dd4e40b0cef9b98215fa43c0a874954e172cf251de0970b6c` /
`7242d6e26546c228f3c2660731d6e698c2472eccc3b008c024b2e606e9ccbbde`.
The Python strict successors are
`.work/official-conformance/outcomes/playwright-python-oracle-popup26-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-python-candidate-popup26-20261011-v1.jsonl`.
Complete 6,526-row audits show 26 exact replacements per lane. Candidate
popup gaps include inherited user agent, headers, touch and viewport,
in-process init scripts, function exposure, child-frame `page`, and a
window-feature timeout. The doubled JUnit record earns no pass or failure
credit; its row remains an explicit report error.

## Fresh same-tooling Core Page.route pair (2026-10-11)

The unchanged 55-case Chromium `tests/page/page-route.spec.ts` file ran from
the pinned v1.58.0 source in independent oracle and exact 45-feature frozen
candidate lanes. Oracle: **53 passed / one failed / one skipped**; candidate:
**50 passed / four failed / one skipped**. Both schema-2 provenance sidecars
verify against the same current runner SHA-256
`21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b`,
the same source/manifest pin, and the candidate native artifact SHA-256
`f5345728c4e2f193af41cacd2e6897669a57a2ae09aed154796de87813fcf81e`.
The candidate result/provenance SHA-256 pair is
`f07f290a82b0d29ef830f16f783476f74024d10239e8278158d64e27deeec48c` /
`becd487ea02173535a706ab137308cbb977455950d83955b7042b058a63e4339`;
the oracle pair is
`f7fcd66db694c7b76a05d1484287cdb9e1ced0ef5ab4e3f437af1daaabc55696` /
`efd61562eff5583395fec2bb563b2928f3316372cf8ebd8b0c4843634c7314b5`.

The route-stage strict successors were
`.work/official-conformance/outcomes/playwright-core-candidate-route55-cookies-frozen-20261011-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-core-oracle-route55-blank-anchor-20261011-v2.jsonl`.
Each replaces exactly the same 55 `library/chromium-page` rows; complete
48,477-row invocation audits pass. Candidate lines 41, 155, 290, 417, 515,
998 and 1010 advance from unexpected to expected. The remaining candidate
failures are lines 276 (also oracle-failing Referer expectation), 646 and 672
(`Response.headerValue` is missing in the Node facade), and 1039 (this frozen
profile's evaluation input limit is 2 MiB). Line 578 is the upstream skip.
Neither the new standalone large-body patch nor the combined network-body
source patch is in this frozen candidate, so they earn no browser credit.

The first attempted fresh oracle report failed all 55 at CDP attachment
(`Browser.setDownloadBehavior: Browser context management is not supported`)
after the last Chrome tab had been closed. It is quarantined as infrastructure
failure, not merged. Leaving one task-owned `about:blank` tab open restored
CDP context initialization; both valid lanes then ran with that same blank
anchor and no foreign nonblank tabs. The anchor later disappeared; a fresh
task-owned blank tab is needed once the foreign browser task finishes. The
anchor is not evidence of a test pass by itself.

## Post-freeze route candidate and occupied-browser preflight (2026-10-11)

The cumulative 47-explicit-feature profile
`official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies-large-post-data`
keeps the prior 45-feature closure and opts into only
`large-evaluation-inputs` and `large-route-post-data`. Its release addon is
frozen at
`.work/official-conformance/builds/core-route-large-post-data-candidate-frozen-20261011-v1.uDzIiy/candidate/`
(native SHA-256 `cbc16676bafca0a1c5a79c9c00bb3413657fa3193348670803c5aaf6dd5b8845`).
The frozen package loads with ABI 1 and the exact 47-feature marker. The
combined Rust host release suite passed 629 tests with eight pre-existing
ignores; the network-events backend passed 25/25 and the standalone route
addon passed 9/9. Strict combined-profile Clippy (`-D warnings`), workspace
rustfmt, Node 689/689, TypeScript, and dependency-boundary checks pass. The
Node `Response.headerValue(name)` facade now uses only the bounded native
raw-header owner/port, with case-insensitive lookup and Playwright-style
duplicate folding. Default capability limits remain unchanged.

A second immutable freeze includes a same-request body-preservation repair in
the Node Request facade and raises only the opt-in large-route CDP event and
WebSocket caps to 8 MiB. It is at
`.work/official-conformance/builds/core-route-large-post-data-candidate-frozen-20261011-v2.2YMq5E/candidate/`
(native SHA-256 `0daec0da334a94e161a73809a243fae336933bb44092ec5d8458f23b0843d5ca`).
The frozen package loads with ABI 1 and 47 exact features. Combined Rust host
release tests pass 629 with eight existing ignores; strict release Clippy,
rustfmt, Node 690/690, TypeScript, dependency-boundary, and native-load checks
pass. The original v1 freeze remains unchanged as historical evidence. At
that checkpoint neither freeze had an official browser result for the new
2 MiB POST behavior; the later v2 result is recorded below.

The first official candidate invocation against the unchanged pinned 55-case
`Page.route` file exited before test execution: two nonblank pages belonging
to another task had appeared in the dedicated CDP Chrome. The runner refused
to interfere with them. This is neither a candidate failure nor a pass, and
it changed no strict-ledger row at that checkpoint. The same shared browser
occupancy also postpones the newly prepared installed-wheel Python
24-async/23-sync `Page.select_option` reruns.

## Opt-in React selector 60-case browser rerun (2026-10-11)

The unchanged, source-pinned Chromium `selectors-react.spec.ts` file ran all
60 cases against the frozen trailing-space React-only candidate at
`.work/official-conformance/builds/core-react-selector-trailing-space-candidate-frozen-20261011/candidate/`
(native SHA-256 `6cf196feca9f692ae9463149cf5ee5564eae94ae893f84504f11070351f8da2b`).
It records **57 expected / three skipped**, versus the old candidate's 57
unexpected / three skipped. Current-runner schema-2 provenance verifies the
new result (SHA-256
`631fbd59deeffef70e291509a0c2eedfee478a6e3040274d938b8c472fac42b8`)
and sidecar (SHA-256
`7547dd5e85224982dbcf4550e10621d2b76ca9dfb18b0c1b268406ee1dda1171`).
All 60 IDs and outcome categories match the earlier independent oracle
report, but that old sidecar pins an earlier runner hash and fails **current**
tooling verification. A fresh same-runner oracle attempt passed 37 tests, then
the dedicated CDP endpoint stopped responding during a `beforeEach`; later
attachment timed out. It was interrupted and the full-file runner rejected
the partial result, so **no fresh oracle pass or failure was promoted**.

The new strict candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-react60-trailing-space-frozen-20261011-v1.jsonl`
(SHA-256 `0cdb227361a396a0088abe0dcf261738a4067406a7d6a06a75cfd8267f4397a0`)
advances precisely the 57 previously unexpected React rows to expected; the
complete 48,477-row audit passes. Candidate totals are **4,508 expected /
14 unexpected / 11 skipped / 43,944 not run**. The oracle Core462 ledger is
unchanged. The original CDP endpoint remained unresponsive despite its Chrome
process listening on loopback. A different dedicated Chrome PID later appeared
and CDP became ready, without this task restarting it. The fresh 60-case
oracle retry was refused *before test execution* because another task opened
two nonblank XLQY pages in that shared browser. This task then closed only its
own blank anchor. No new oracle result or strict-ledger row was produced.

A later retry in the free dedicated Chrome completed the entire unchanged
60-case oracle file: **57 passed / three upstream skipped**. Its schema-2
provenance verifies the exact current runner SHA-256
`21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b`
and the same manifest digest as the frozen candidate. The oracle
result/provenance SHA-256 pair is
`47bc05b2394d71ed5e4b90fc075d15690d3e70fec828f1b960e4c6e2221f5f2c` /
`414e9d27079232f6ac90e370e66ef13d83abc32c78735b3eb021b5b9fdf2da73`.
All 60 `(specId, projectId, status)` triples equal the current-runner
candidate's; this is now a same-tooling paired public-behavior result, not a
comparison to an obsolete oracle sidecar. The strict oracle successor
`.work/official-conformance/outcomes/playwright-core-oracle-react60-current-runner-20261011-v3.jsonl`
has SHA-256 `8ac1f2361a6629e69a556830ec7ef7a9a6dade829ee7933bf71f8478b6f22b10`.
An all-48,477-row diff against the Core462 predecessor found exactly 60
replacement result references (57 expected-to-expected, three
skipped-to-skipped) and no other row change; aggregate oracle counts are
unchanged.

## Frozen 47-feature Page.route browser rerun (2026-10-11)

After the foreign pages left the dedicated Chrome, a new task-owned blank
anchor allowed the unchanged, source-pinned 55-case Chromium `Page.route` file
to run against the immutable v2 47-feature candidate. The result is **53
expected / one unexpected / one skipped**. Its schema-2 provenance verifies
against runner SHA-256
`21f1df10e77485bf29b67b31e548a1bc5126d50dc47cdeb115a9cf6798704a7b`
and frozen native SHA-256
`0daec0da334a94e161a73809a243fae336933bb44092ec5d8458f23b0843d5ca`.
The result/provenance SHA-256 pair is
`3088e9dbfc8a87214eb2bc43411f3df7614fc620b6a404d5972a04cbd1d4cbae` /
`f3d43b396d292773fc8fe79d02a423486f0d51b926106df2086d12431aa7a99a`.

All 55 invocation IDs and outcome categories now match the independently
verified prior oracle report (53 expected, one unexpected, one skipped). The
three candidate improvements over its 45-feature predecessor are lines 646
and 672 (`Response.headerValue`/CORS) and 1039 (over-1-MiB POST data). The
shared unexpected line 276 is **not equivalent behavior**: the candidate
reports `page.goto: Target page, context or browser has been closed`, while
the oracle reaches an assertion about a duplicated Referer header. Thus
matching pass counts do not establish full compatibility. The same CDP-adapted
oracle measured only one Referer value in two runs, so this expectation is not
evidence that Pwright should synthesize a duplicate. In the candidate, a
route-handler exception can close the session and mask the original assertion;
its actual header value was not captured. Keep the row unexpected until a
one-case diagnostic records both the header and handler error.

The strict candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-route55-large-post-data-frozen-20261011-v2.jsonl`
(SHA-256 `d82424206c85dbad97e604c541368cd15212e01474634a5fce2c67f425873e42`)
records exactly the three newly passing rows; the 48,477-row invocation audit
passes. Totals are **4,451 expected / 71 unexpected / 11 skipped / 43,944 not
run**. The oracle Core462 strict ledger remains unchanged at **4,213 expected
/ 11 unexpected / ten skipped / 44,243 not run**. The 2,607 source-only rows
remain separately tagged in both lanes.

## Core462 MCP config private source-only shard (2026-10-11)

The dedicated manifest
`tests/official/playwright-core-mcp-config-source-only-v1-manifest.json`
(SHA-256 `af29726a55278d44279731b506321aa9b9c616e6b1cbbc6263da4d43ea9451d5`)
pins two zero-fixture `tests/mcp/config.spec.ts` sandbox-configuration test
bodies, each collected in four upstream projects: **eight exact invocations**.
Both candidate-labeled and oracle-labeled runs executed the pinned upstream
v1.58.0 bodies with a guarded runner and worker, no browser or network I/O,
and passed **8/8**. Result/provenance SHA-256 pairs are:

| Lane | Result SHA-256 | Provenance SHA-256 |
| --- | --- | --- |
| Candidate-labeled | `401e6d88ea26e0b5b1212851364f89a678bae8871c588e9f5504be2c5ce3a640` | `9dd538f952adf9fa99120d42b74fc90a73c61ffce545ff5a666d7f16c1f110ca` |
| Oracle-labeled | `69291c0ae47883f6bcb1f1fb3491d3c46f23876e042dc73ae87100169a342f11` | `913800eb467e6323387d9c0299d4308e1c83fdf8798b789f702c7306d09e9b3c` |

The verifier binds source, manifest, exact IDs and statuses, runner/worker
no-I/O receipt, tooling hashes, and the result path and digest. A regression
test closes a same-stem alternate-result promotion gap found during review.
The immutable 48,477-row strict successors are
`.work/official-conformance/outcomes/playwright-core-candidate-core462-mcp-config-source-only-v1.jsonl`
(SHA-256 `2be92f4ea2c06093a96ad2494c9ce59e4941d8327715d0c3af96b1d53265adf0`)
and
`.work/official-conformance/outcomes/playwright-core-oracle-core462-mcp-config-source-only-v1.jsonl`
(SHA-256 `35fa34455be9a86f4e65dd90688c402595724a78f66e47accec035d07454689d`).
Exactly eight previously `not-run` rows become `expected` per lane, with no
other row changes; both complete invocation audits pass. These rows are
upstream-private source-only evidence (`candidateRuntimeRequired=false`),
**not** Pwright runtime, browser, or public API compatibility evidence. The
audit classifies the MCP suite as product infrastructure even though the
inventory marks these direct private-import rows as implementation-internal;
neither classification implies public API parity.

## Core461 image-tools private source-only shard (2026-10-11)

The dedicated v2 manifest
`tests/official/playwright-core-internal-source-only-image-tools-v2-manifest.json`
(SHA-256 `929328a2ce4f74a02ffa57c584bdd28ad27ca1617893d364d4acbe438d261d48`)
pins 25 implementation-internal `playwright-test`/`image_tools` invocations:
21 `tests/image_tools/fixtures.spec.ts` cases plus four
`tests/image_tools/unit.spec.ts` cases, disjoint from Core460. Both
candidate-labeled and oracle-labeled no-I/O runs executed the upstream
v1.58.0 test bodies at commit `961381ec73a9fa2661d92504eab1a6bd6eaffc75`
in one guarded worker and passed **25/25**. The pinned source/PNG digest is
`850aaf28f80268352daaeaed5f58fd20af593956eabd551dae6412804f5353bb`.
Their immutable result and independently verified source-bound provenance
SHA-256 pairs are:

| Lane | Result SHA-256 | Provenance SHA-256 |
| --- | --- | --- |
| Candidate-labeled | `7eb291e926f4b6766f63a06f26bf3375e72edf309b8aaf5a47e4ca6f0327b874` | `a8500cbff06457a2562fded7025066b4a8fd284820c8e574d245e37ca450bcee` |
| Oracle-labeled | `8aed211d76e71750a0de8f8fd7e8a3e3554c3e937889c9cec233a8d96f071a61` | `136c30a4c3daf3fa6a62600544e1a2cf5be257284ea5430625fdc5b47b50e825` |

The independent verifier checks the pinned source and 42 PNGs, exact IDs and
outcomes, AST/no-I/O guard receipt, and tooling/provenance hashes. The v2
manifest uses `sourceOnlyAdapter`, not `candidateAdapter`; both sidecars set
`candidateRuntimeAttested=false` and `publicApiParityEvidence=false`.

The new immutable strict successors are
`.work/official-conformance/outcomes/playwright-core-candidate-core461-image-tools-source-only-v2.jsonl`
(SHA-256 `63660ff6364254d93fa0b6d408bbb9e87575c631368d715d83edf0116c2a525f`)
and
`.work/official-conformance/outcomes/playwright-core-oracle-core461-image-tools-source-only-v2.jsonl`
(SHA-256 `d22c37309ebd1e87b740282dd1b4b45a80605b2c1fc63c8ee196e79e77d34419`).
In each 48,477-row ledger, exactly these 25 rows advanced from `not-run` to
`expected`: candidate totals are **4,440 expected / 74 unexpected / 11 skipped /
43,952 not run**; oracle totals are **4,205 expected / 11 unexpected / ten
skipped / 44,251 not run**. Fresh complete invocation audits passed for both
successors. The rows retain `sourceClassification=implementation-internal`,
`role=official-entrypoint`, `evidenceScope=upstream-private-source-only`, and
`candidateRuntimeRequired=false`. This is source-only accounting of official
upstream bodies, **not** Pwright runtime attestation, public API parity, or a
browser pass. The superseded v1 receipt is retained without mutation.

## Core460 private source-only batch (2026-10-11)

The pinned manifest
`tests/official/playwright-core-internal-source-only-batch-v1-manifest.json`
(SHA-256 `c6fd25f14df9f1f9355cf29c8f3534ddf4c6e3bcd7109cda434e918f33756e76`)
selects 284 zero-fixture upstream-private tests from 26 exact source files in
each of three Library projects (852 invocations) and six BiDi projects (1,704
invocations). A source-hash/AST guard and a no-I/O runner prohibit browser,
CDP and fixture-server access. The archived candidate environment and
independent oracle each pass 852/852 Library and 1,704/1,704 BiDi, while the
candidate sidecars explicitly record `candidateRuntimeAttested=false`.
Result and independently
verified schema-3 provenance SHA-256 pairs are:

| Lane | Suite | Result SHA-256 | Provenance SHA-256 |
| --- | --- | --- | --- |
| Candidate | Library | `fc68db16b3942cb642e30f3a60a06cf86f39cb1089e621bc6e7d736b10d929a2` | `a8a3b5c7518579e09bbb787b2ccb9ef042ddaadcd8fdf35ce7adf5d40938b41e` |
| Candidate | BiDi | `fa044442ddbff6d11c3b894b4432e25813064fba2dd6e58f0a6e821103ce5729` | `f314df386f15a7451956afcb9567eecfd75221f7846cb1bfe62d7aa35079f18a` |
| Oracle | Library | `5e0454e3b0a7efddd3c78c10543290baad421d74a2738b257fe833033eb0b5fd` | `cc92a16e16c971299cf3ce125ee2cdce616d02134cc45045e643bbd69c5e2972` |
| Oracle | BiDi | `5e7353a4c90b9700d5ac7953dcf3c3e550a8431d7b96e006e3426bfff185ba2d` | `c505edb483c671c7e186aa1af280d44f2e98ae2b3d4169b373e77c92e0fb291e` |

The four reports and sidecars are under `.work/official-conformance/results/`
with stems `playwright-core-{candidate,oracle}-core460-private-batch-{library,bidi}-v1`.
The immutable strict successors are
`.work/official-conformance/outcomes/playwright-core-candidate-core460-private-batch-v1.jsonl`
and
`.work/official-conformance/outcomes/playwright-core-oracle-core457-private-batch-v1.jsonl`;
their source-only rows are retained in the current Core461 successors above.
Full 48,477-row invocation audits pass. A line-paired predecessor/successor check found
exactly 2,556 changed `not-run` rows in each lane and zero other changed
rows. All new entries carry `evidenceScope=upstream-private-source-only` and
`candidateRuntimeRequired=false`. They account for execution of official
upstream internal test bodies but do **not** exercise or validate Pwright's
public API.

The original file-level inventory labels remain frozen: 2,115 of these rows
per lane say `implementation-internal`, while 441 say
`public-api-candidate`. Those 441 are 49 declarations across nine projects
in `tests/library/events/*.spec.ts`; each tests `EventEmitter` through the
local `./utils` helper, which subclasses a private Playwright Core client
implementation. The pinned invocation-level audit classifies all 441 as
`implementation-internal` with the exact
`local-implementation-under-test:tests/library/events/utils` reason. The
source-only promotion gate now verifies that audit's commit, inventory and
digest, and accepts only these exact event exceptions. No public API pass is
inferred from the conservative inventory label.

Read-only source audit of the 11 remaining candidate `Page.route` failures
found ten oracle-pass discrepancies and one independently oracle-failing
Referer case. Three candidate failures share a missing per-hop route Request
history owner (`request.response()`); two share absent CORS headers on the
final cross-origin `route.fulfill` response. Other rows concern regexp
route/unroute, cookies, redirect identity and a 2 MB POST limit. These are
diagnoses, not new test outcomes. The separate host-only opt-in
`route-fulfill-cors` patch now passes release Page.route tests 38/38 (including
nine new AAA cases), feature-off Page.route regression 29/29, and default
host tests 543 passed/eight ignored. The opt-in addon profile
`official-conformance-page-route-header-url-unroute-cors` now builds in release
mode, loads with the expected CORS marker and `pageRouteFulfill` binding, and
is frozen at
`.work/official-conformance/builds/core-route-cors-candidate-frozen-20261011/candidate/`
(native SHA-256 `0863434ffd82e778704544aac38a1d8d48448822ff4785c9398a2e68b85c67e5`).
Node browserless tests pass 657/657, TypeScript and dependency-boundary gates
pass. This isolated source fix targets the candidate-only official route cases
at lines 646 and 672. The later cumulative browser rerun still fails both
because `Response.headerValue` is absent from the Node facade; the isolated
frozen package itself was not browser-tested.

The separate opt-in `route-regexp` path addresses the immediate non-string
matcher rejection in pinned `Page.route` cases at lines 41 and 290. Node
accepts a real RegExp and `unroute` compares its source and flags while
retaining strict handler identity; Rust compiles a bounded full-URL matcher,
widens CDP interception only while a RegExp is registered, and restores glob
patterns after removal. It admits at most 32 regex registrations with a
4,096-byte source and 64 KiB compiled-program/DFA builder limits. This is an
explicit ECMAScript subset: `i`, `m`, `s`, `u` are admitted, `g`, `y`, `d`, `v`
and unsupported syntax fail closed. Focused Rust release tests pass 6/6,
the base route host and opt-in addon release checks pass, and the complete
Node browserless suite passes 670/670. The cumulative
`official-conformance-page-route-header-url-unroute-cors-regexp` release addon
loads with exact RegExp/CORS/unroute markers and is frozen at
`.work/official-conformance/builds/core-route-cors-regexp-candidate-frozen-20261011/candidate/`
(native SHA-256 `83ecbc92d780138c8d2bb1fbfe8821b41a8a6a20796eb11fbae84b0950d40c03`).
The later cumulative 55-case browser rerun confirms both pinned cases at
lines 41 and 290 pass. The isolated RegExp package itself was not rerun.

An additional opt-in `route-request-history` path now captures the exact
network history owner before a routed Request reaches the Node callback. This
avoids looking up a reused CDP request ID after `continue()` or a redirect.
The same charged owner backs `request.response()` and a bounded N-API
`responseRawHeaders()` port; the Node `Response.allHeaders()` adapter folds
ordered duplicate values with Playwright's `set-cookie` separator. Missing
correlation fails closed, and the Rust owner lookup has a 30-second bound.
Focused release host tests pass 3/3, addon conversion and feature-off tests
pass 1/1 each, the Node suite passes 676/676, and TypeScript and dependency
boundaries pass. The combined release profile
`official-conformance-page-route-header-url-unroute-cors-regexp-history` loads
with its exact marker and port from the frozen package at
`.work/official-conformance/builds/core-route-history-candidate-frozen-20261011/candidate/`
(native SHA-256 `dd335acc7f5d7e2045b9ce8ed78ab9bb95e94dfd179197c644e980dd512309e5`).
The later cumulative unchanged browser rerun confirms the three pinned
`Page.route` cases at lines 515, 998 and 1010 pass; the isolated history
package itself was not rerun.

A subsequent source-only redirect-identity repair addresses the candidate
failure at pinned line 417. The route-history addon now publishes the exact
portable request ID and predecessor ID from its charged history owner; the
Node route callback canonicalizes that Request with the existing network-event
stream. The handler-retained first stylesheet Request can therefore link to
subsequent redirect hops even though Chromium does not re-intercept them. The
focused route/request adapter suite passes **63/63**, including a new
three-hop identity regression, and a release addon build with
`route-request-history,request-events` loads with both expected markers and
ports. The earlier frozen route-history addon predates these fields, and the
later frozen route-history/cookies addon includes them. The unchanged 55-case
browser rerun confirms line 417 now passes.

The remaining route work is deliberately split by capability. A standalone
`context-cookies` profile now has an exact feature closure, a release-built
addon, native marker smoke check, 682/682 Node package tests, TypeScript
checks, and dependency-boundary checks. The cumulative
`official-conformance-page-route-header-url-unroute-cors-regexp-history-cookies`
source includes that `addCookies` adapter. Two release attempts stopped at
the final addon step when another task's Rust compiler caused severe memory
pressure; a later quiet-window build succeeded without changing the 8 KiB
creation-future assertion. The exact 45-feature package, source tree, and
native addon are frozen under
`.work/official-conformance/builds/core-route-cors-regexp-history-cookies-candidate-frozen-20261011-v1/candidate/`
(native SHA-256 `f5345728c4e2f193af41cacd2e6897669a57a2ae09aed154796de87813fcf81e`).
Native-load, ABI, marker, binding and package-source checks pass. The
unchanged official 55-case browser file has now run on this artifact as the
fresh paired result recorded above; its earlier archived reports had an older
runner hash and were not used for ledger promotion.

For pinned line 1039, an opt-in `route-large-post-data` profile adds a 3 MiB
evaluation input cap and a separately bounded route body path. Chromium may
omit `request.postData` for long bodies; the new source decodes bounded,
ordered `postDataEntries` base64 into one buffer, with at most four large
in-flight routes. A source audit found that the current opt-in Node watch
reserves its 12 MiB allowance even while idle: five watchers consume 60 MiB
of the 64 MiB environment FFI budget and a sixth is refused. The reservation
now grows only after a route result arrives in a follow-up. Four
focused AAA tests cover idle watchers, actual body charge and fail-closed
abort. The standalone addon profile's release suite passes 9/9 and strict
Clippy passes with `-D warnings`; two mechanical Clippy repairs were required
in the existing CDP binding and route-body code. This validates the local
budget boundary, not the official browser case. The
default 256 KiB input/route limits are unchanged. The opt-in CDP event and
WebSocket caps are now 8 MiB so a 3 MiB unescaped POST copy plus its 4 MiB
base64 copy have finite envelope headroom. Pathological JSON escaping can
still exceed that independent transport cap, so 3 MiB decoded is not an
end-to-end guarantee; the pinned 2 MiB body is the intended test case. A
later bodyless network notice also now preserves a same-request route-captured
body; a 2 MiB Node regression covers that ordering. Source formatting and
browserless package/type/boundary checks have run, as have the host
route-decoding and network-body release tests, strict Clippy, and cumulative
native packaging. In the opt-in full route-history composition, at most four
3 MiB network bodies per Page are allowed while ordinary requests remain
capped at 256 KiB. Its official browser rerun is still unverified because
the shared dedicated Chrome is occupied; no strict-ledger row is promoted
from these post-freeze changes.

At the 2026-10-11 pre-execution checkpoint, the then-next unchanged complete
Core Chromium `Page.selectOption` file had a
source-pinned 32-case manifest at
`tests/official/playwright-core-page-select-option-public-chromium-v1-manifest.json`
(SHA-256 `acc5e1b0ce2d6b3eff4c14a8b7ad650c76091ae457e293039311394231aead99`).
The oracle had collected all 32 IDs, but neither lane had a browser result;
another task's nonblank page occupied the dedicated CDP Chrome. These 32
cases were `not-run` in those historical ledgers. The v11/v12 executed
successors above supersede that state.
At the same earlier checkpoint, a pinned-source preflight classified 26 as
source-aligned but unverified, five as definite candidate gaps (one
`ElementHandle` value, two invalid-argument diagnostics, two disabled-option
timeout diagnostics), and one navigation-during-input case as live-test
uncertain. Those static classifications were not official outcomes and are
superseded by the complete v12 browser result. The JS invalid-argument
preflight had 15/15 focused browserless contract tests at that time.
The analogous Python sync `Page.select_option` file has a separate pinned
23-case manifest. At this earlier checkpoint both lanes had only collected
its 23 IDs; later execution reached candidate 22/23 and Oracle 23/23, as
recorded in the Python conformance history.

The opt-in `select-option-timeout-progress` Rust path passes focused release
checks at its contract, CDP backend and host layers (4/4, 8/8, 12/12).
Python's opt-in `Page.select_option` now has a release-built exact-target
`ElementHandle` guard (parser/guard tests 9/9) and a frozen full-feature
`cp39-abi3` wheel (SHA-256
`1a536d38e585ce73d8ab9c313bbc7da41f60dfb5c33485ced56d16c32790b533`).
Python 3.9.6 and 3.12.14 both load its native ABI 108 and each pass all
436/436 installed-wheel package units, with no deselections. See
`docs/python-official-conformance.md` and the isolated build receipt there
for the feature profile and limitations. At this earlier checkpoint neither
path had earned an official browser pass on that wheel. The later Core
32-case and Python 23-case outcomes are recorded separately; they do not
retroactively change the frozen wheel or addon evidence here.
The Node `Page.selectOption` ElementHandle identity bridge and invalid-input
preflight pass their browserless adapter and fake-CDP checks; the remaining
read-to-action DOM mutation race is explicit. The full opt-in
`official-conformance-page-select-option-timeout-progress` release addon now
builds after the N-API feature-gate repair and loads with exact profile markers.
Its frozen candidate is at
`.work/official-conformance/builds/core-select-option-candidate-frozen-20261011/candidate/`
(native SHA-256 `b05e525f3569a2ce6459e16b48b0c11b32ccd488bd822e88a17ede5b215a8f97`).
Node browserless tests passed 654/654, as did TypeScript and
dependency-boundary checks. At that checkpoint, the pinned Core 32-case file
had not been executed with this addon; the later v11/v12 results above replace
the `not-run` ledger state without changing this frozen artifact's history.

The opt-in `fill-navigation-commit` fix now has a release addon and frozen
candidate package at
`.work/official-conformance/builds/core-fill-navigation-commit-frozen-20261011/candidate/`
(native SHA-256 `9f019c54294bdc8ccc7f33a44796abb5df2bc10f4d7c5458abcca974d7f5c579`).
Rust release tests **11/11**, Node contract **127/127**, native-load,
TypeScript, and dependency-boundary gates passed. At that checkpoint the
unchanged official 45-case `Page.fill` browser rerun had not yet run, so the
strict ledger still recorded the navigation-context failure. The later 45/45
v13 browser-backed successor is recorded at the top.

## Earlier strict checkpoints (preserved chronology)

Current Python strict 6,526-row successors record candidate **1,400 passed /
15 failed / one report error / 11 skipped / 5,099 not run** (SHA-256
`89b9493d3c79982ad621071da51c3cf8d4b6b363ff5dd2d0132888852da718d2`)
and oracle **1,134 passed / four failed / 11 skipped / 5,377 not run**
(SHA-256 `90d6295861ceb2461ebe0ec82796b4520cc7cce9d212faf8a08af6bc620ebc78`).
Exactly **1,121** IDs have paired pass evidence, including one explicit
favicon fixture variant. The new complete 36-case Chromium async
`page.evaluate` slice is candidate **26 passed / ten failed**, oracle **36
passed**; eight gaps are Python value codecs and two are error-text
semantics. The installed post-repair wheel had completed the unchanged
70-case Chromium click file with **65 passed / four failed / one fail-closed
report error** versus oracle **69 passed / one original-fixture 404 failure**.
The candidate invocation audit still classifies 6,514 applicable / 12
non-applicable. See `docs/python-official-conformance.md` for artifacts and
remaining gaps.

A fresh source-bound paired run of the complete pinned **64-case Core
Chromium `Page.goto` file** first recorded candidate **61 expected / three
unexpected** and oracle **63 expected / one unexpected**. Candidate :217
cross-process iframe request frame failed in that run, then passed in an
isolated diagnostic and a second complete 64-case candidate run. The second
candidate run records **62 expected / two unexpected**. The Node `toHaveURL`
bridge now passes :492; :859 still lacks `framedetached`. Both lanes fail
:49 when the protocol-less MagicDNS host is parsed as a scheme. The latest
candidate Core strict ledger advances only :492 to expected: **1,765 expected /
12 unexpected / seven skipped / 46,693 not run**. The intermittent :217
failure remains a separate unresolved diagnostic, not a silently erased run.

The first exact **55-case Core Chromium `Page.route`** candidate run recorded
**30 passed / 24 failed / one skipped** with source-bound provenance. A
versioned origin-correct rerun plus the four-message JS fix recorded **36 passed /
18 failed / one skipped**. The newest opt-in header/URL/redirect route profile
passes **40 / 55**, with 14 failures and one skip. Its 48,477-row candidate
strict successor is **1,805 expected / 26 unexpected / eight skipped / 46,638
not run** (SHA-256
`51a46b47dcca314abf83bb6265f231e24385a95c21bdd80a57ffbfe0a2844174`).
After a transient CDP unavailability and `chrome-cdp start`, the independent
oracle completed the same origin-correct 55 cases: **53 passed / one failed /
one skipped**. The new oracle strict successor is **1,522 expected / 11
unexpected / seven skipped / 46,937 not run** (SHA-256
`5631c74c9d9d60030319696fbed9f4600430db392cc0a3ae1a29be9ef244fb3f`).
The latest exact paired route outcomes are 40 both expected, 13 candidate unexpected
while oracle expected, one both unexpected and one both skipped. Earlier Core
and Python oracle setup diagnostics remain recorded; the Chrome extension was
not disabled. A new full-feature Python wheel independently passed both exact
retained-click timeout tests; those two strict rows moved from failed to
passed. The fresh oracle retry was setup-blocked, while its earlier 70-case
report already passed both IDs.

The next complete public Chromium Core `page-fill.spec.ts` file has 45 newly
executed cases: candidate **40 passed / five failed**, official oracle **45
passed**. Incorporating this file, the latest Core strict ledgers are candidate
**1,845 expected / 31 unexpected / eight skipped / 46,593 not run** (SHA-256
`3b1a91ec1ef1c85d838ffc37841cb5a67e891b382f450c7e0dcbc3ec814e3415`)
and oracle **1,567 expected / 11 unexpected / seven skipped / 46,892 not run**
(SHA-256 `2d32b91be9b8110586a24183d9cc8c3169312641482cc7925cdcda89f0ccf705`).

## Verified combined Core reruns and selectors-react file (2026-10-11)

The frozen combined candidate package and release addon at
`.work/official-conformance/builds/core-combined-fill-route-candidate-frozen-20261011/candidate/`
use the opt-in `official-conformance-page-route-header-url-unroute` profile;
the selected native binary SHA-256 is
`bd82a77e23d8ea0972cde162165671d628d31fb709157bf579dcee2abe158fe4`.
The unchanged complete 45-ID `Page.fill` candidate report passes **44** and
fails only source line 388, where navigation destroys the utility execution
context after the input handler. Its JSON / verified provenance SHA-256 are
`43fce218e6956a7cd0689613ef99a47844f1f398ae491bc367c440f2912ef647` /
`01b60de056d278e049f632e3159baad5d3ee23f6ac26ccf39227f7e137dcb205`.
Exactly the old failures at lines 76, 273, 280 and 399 become expected;
no `fill` row regresses. The independent original oracle report remains
**45/45**.

The same frozen candidate and unchanged origin-correct 55-ID `Page.route`
manifest pass **43**, fail **11**, and skip **one**. JSON / verified provenance
SHA-256 are
`b82ddf0a212cd83f8ee999bb1e70398d7e6902bf2fea57a59ed4b0430d2f30eb` /
`a4f26d13142fba35e594219e5b91f3a49f32e206f779a7ca99d1d49cc9e7c636`.
Versus the promoted header/URL report, exactly lines 169 (Cookie override),
955 (`unroute` from another handler), and 988 (`allHeaders`) become expected,
with no regressions; the independent original oracle stays **53 expected /
one unexpected / one skipped**. The combined candidate strict successor is
`.work/official-conformance/outcomes/playwright-core-candidate-core459-page-goto64-route55-fill45-combined-v2.jsonl`
(SHA-256 `2b2a2b3cd85e39a7f087c2eb1a057c2a19dada00f62b94f41f734739cd16d450`):
**1,852 expected / 24 unexpected / eight skipped / 46,593 not run**.

The next pinned public Chromium `selectors-react.spec.ts` manifest
`tests/official/playwright-core-selectors-react-public-chromium-v1-manifest.json`
(SHA-256 `72d40ba44380b0c53dd76c6c82ce7e0c1818574ba14489546fa5c4befcd0ad4a`)
ran all **60** cases in both lanes. Oracle: **57 expected / three skipped**,
JSON / verified provenance SHA-256
`1199ac92aa10e6fe2dad69a3836ce05793315c4540753429aab2106418ff2126` /
`6896d13a6c5beda7b1cb4d1acd4b6d75f29bd70573207559ee47268a1ef1d1ae`.
Candidate: **57 unexpected / three skipped**, JSON / verified provenance SHA-256
`260a4a5405183da725404d6b5e2090bdfb12979b338757fb9e0f1873e5811435` /
`b6b635482563a7da3a3d208e007f6463faef7caf361fcc7bc7385a888fef46b9`.
Upstream first queries React component matches; on nonzero matches it reports
the deprecated `_react` engine and actual match count, while zero-match
queries succeed. Pwright currently forwards `_react=` to CSS and fails with a
syntax error. Merely changing error text cannot satisfy the semantic tests.
The resulting immutable candidate/oracle strict ledgers are respectively
`.work/official-conformance/outcomes/playwright-core-candidate-core459-page-goto64-route55-fill45-selectors-react60-v3.jsonl`
(SHA-256 `c35b16e1f8884845637cc9a3d4e5e6b882ed4c0a11090cc360a1496615f271ab`)
and `.work/official-conformance/outcomes/playwright-core-oracle-core456-page-goto64-route55-fill45-selectors-react60-v2.jsonl`
(SHA-256 `30ddde792127e284784d7c41353d753eb24dfafaadd0e4aeb200d15e69964370`).

The React repair needs an opt-in Rust-owned selector plan, not just an error
rewrite. The pinned file exercises React 15–18 Fiber/legacy roots, nested and
shadow roots, component props (nested paths, booleans, numbers, string
operators, regex), and selector chains. The browser helper must query the live
tree and return actual final DOM matches; only after all `>>` composition may
a nonzero result raise the deprecated `_react` diagnostic with its actual
count. Zero matches must still return zero. `Locator._expect('to.have.count')`
uses native `expect_count` and `LocatorRead::Count`, so routing only ordinary
`Locator.count()` through a browser program would miss the official tests.
Any implementation must keep parser/query bytes and Fiber traversal bounded,
keep the default profile unchanged, and rerun the complete 60-case pair.
An opt-in `react-selectors` source path is now implemented in the Rust host: it
selects a bounded browser plan, installs a separate versioned React tree
helper only for that plan, composes final matches before emitting the
deprecated-selector count error, and routes native `LocatorRead::Count`
through collection evaluation. Browserless JavaScript helper tests pass 10/10
(React 15 legacy, Fiber component counts/scopes, open shadow roots, attribute
operators and typed props, malformed selectors, scoped CSS, parent-first
components, shadow-first roots, and trailing whitespace after an attribute
selector). The opt-in Rust host release check and focused release tests pass
(3/3), and the shared
CSS selector runtime's own scoped-root release test passes (1/1). Its runtime
version is now 12 so an existing execution world cannot retain the prior CSS
behavior after reinstallation. The separate React helper is now version 3
so an existing page replaces its earlier child-first/light-first helper and
accepts the trailing space exercised by four pinned official React cases.
The full upstream deprecated-selector error text and other parser subtleties
are not yet reproduced. This path has no new official pass receipt; the
complete 60-case browser result remains the older 57-failure run.
The opt-in full-profile release addon is frozen at
`.work/official-conformance/builds/core-react-selector-candidate-frozen-20261011/candidate/`
(native SHA-256 `bd8d1766ee49cb4112d4b031c3b1bcb274354d2e13d882bc1367bd884e874fb6`).
It passes exact-profile native loading/marker checks; the complete Node
browserless package suite now passes 670/670 and TypeScript type checks pass.
The parent-first/shadow-first successor is separately frozen at
`.work/official-conformance/builds/core-react-selector-order-candidate-frozen-20261011/candidate/`
(native SHA-256 `6264203ad48234b3962d97ac0901a33d82f6a91d39c7725f8244720cea8e0abb`)
and also passes exact-profile native loading/marker checks.
The trailing-space successor is frozen separately at
`.work/official-conformance/builds/core-react-selector-trailing-space-candidate-frozen-20261011/candidate/`
(native SHA-256 `6cf196feca9f692ae9463149cf5ee5564eae94ae893f84504f11070351f8da2b`)
and passes exact-profile loading, including proof that `route-regexp` remains
off in this React-only build. The static 60-case audit now finds no further
definite source-level blocker among 57 runnable cases, but that is not an
official result.
Splitting three optional Page.route N-API methods into feature-gated impl
blocks also makes the previously failing base `page-routes` addon combination
pass `cargo check --release`; this is a build gate, not a route browser pass.

## Core Page.route paired full55 and earlier oracle setup diagnostic (2026-10-11)

The fixed public 55-case manifest SHA-256 is
`19683ee2c92a24d46efddc1c73683ba0d0d4f77c3480802ef8c501acb1d59dfb`.
The candidate used the frozen `official-conformance-page-goto-burst-20` native
addon, one worker, pinned upstream source, Tailscale-bound fixtures and the
same loopback Chrome CDP endpoint as the earlier paired goto run. Its result
JSON / source-bound sidecar SHA-256 are
`c8ba4c04bc1f0f9f09cbc25bf9591991abff4ba16cf9935e6f645a51fb89dfa1` /
`24a3a9180d707cd6a57fb4205166aae9fbd9de452662cae74c88177e1f97b6c5`.
The exact pre-edit candidate package and addon are archived at
`.work/official-conformance/builds/core-route55-pre-js-doublehandle-20261011/candidate/`;
the sidecar also verifies against this archive. The candidate failures include
unsupported non-string route matchers, `page.unroute`, override options,
`Request.response`, CORS/redirect differences, and four double-handling error
messages missing the official exclamation mark. The one skip is upstream's
conditional Chromium test, not a filtered-out invocation.
After freezing the report, current Node source aligns that exact message for
all four Route settlement methods. Browserless package tests pass 596/596,
types and dependency boundaries pass; no official browser row has been
reclassified yet. The route fixture also lacks the cross-origin forwarder used
by the Page.goto suite: ordinary and cross-process prefixes collapse to the
same MagicDNS origin. Thus some CORS/preflight failures are fixture-confounded
until a versioned origin-correct paired run, not established candidate-only
library defects.

All 55 rows were previously `not-run` and now have explicit candidate
outcomes. The new strict ledger SHA-256 is
`4830fd47de71ff990f26ae253bfe95ed3a189e280777dff0b4f6d10808556eec`;
its full invocation-audit summary SHA-256 is
`90791e9ee720345d538b5cca60a1e0e664c5253c0e74cc542f82172931ee3971`.
An earlier independent-oracle *one-case diagnostic*, not a 55-case comparator, failed
before the `should intercept @smoke` body on the same
`chrome-extension://.../thunk.js` service-worker target that blocked fresh
Python oracle runs. Its JSON / verified sidecar SHA-256 are
`02dddcdc87c741a5eba796dfcf3a3cca75b02b13cfeae4a717d73582708a8ea8` /
`6e2a9a4505c957a1064168a773b6cb3a361654c4f20dd9c8e962ff4b3ce5b432`.
At that diagnostic checkpoint no oracle strict route row changed and no
extension was disabled. The later complete oracle run below supersedes its
setup-blocked status. The final service-worker route case is upstream-skipped
in both lanes; it still needs a trusted-origin fixture for a meaningful run.

Read-only triage accounts for all 24 candidate failures in the frozen report.
These are *candidate observations*, not proven candidate-only discrepancies
without an origin-correct oracle comparator:

| Surface | Cases | Source lines | Qualification |
| --- | ---: | --- | --- |
| RegExp route matcher | 2 | 41, 290 | JS route facade rejects non-string matchers |
| `continue` header overrides | 2 | 117, 129 | Rust option exists, selected Node bridge omits it |
| Context cookies | 1 | 155 | Selected profile lacks `addCookies` |
| Handler cause masked by target close | 4 | 169, 276, 955, 988 | Missing `allHeaders` / `unroute` and referer behavior need separate attribution |
| Redirect route hops | 2 | 362, 417 | Selected addon omits existing `route-redirects` feature |
| `fallback` URL override | 1 | 397 | Rust option exists, Node bridge omits it |
| Request-to-response lifecycle | 3 | 515, 998, 1010 | Route-created Request lacks history owner/profile |
| CORS fulfill defaults | 2 | 646, 672 | ACAO behavior needs an origin-correct pair |
| Collapsed cross-origin fixture | 2 | 695, 739 | Same MagicDNS origin confounds both assertions |
| Duplicate Route settlement text | 4 | 1027 (four methods) | JS source fixed after report; official rerun pending |
| Large POST payload | 1 | 1039 | Current input and route body caps are 256 KiB |

Two *passing* rows, lines 546 and 784, may also be vacuous because the fixture
does not supply a distinct cross-process origin. Those five origin-sensitive
rows (546, 672, 695, 739, 784) required a separately versioned fixture run
before durable semantic credit. The historical 30-pass aggregate is a faithful
test-run outcome, not a claim that those two passes proved cross-origin support.

The new origin-correct manifest
`tests/official/playwright-core-page-route-public-origin-v1-manifest.json`
(SHA-256 `0480644208376a3996c0222b03570408ffcaf4fd6a70d049f3eb5a257a611353`)
keeps the exact source, 55 IDs and project while enabling the distinct-origin
forwarder already used for Page.goto. The runner admits only this exact pinned
route metadata or its existing goto suite; browserless runner tests pass 35/35.
The candidate rerun records **36 passed / 18 failed / one skipped**. Its JSON /
verified source-bound sidecar SHA-256 are
`56ed1bd13dce330328e0eab1a426c9bf64fc232b40f7bb91b96abcaa473e2bee` /
`56cb827b3ca25ebb79a0f6998da89488b402e66cb589649c6a2895431ff3b846`.
Exactly six rows change versus the historical report: lines 695 and 739
(origin fixture) and the four line-1027 duplicate-Route methods all become
expected. The immutable strict successor
`.work/official-conformance/outcomes/playwright-core-candidate-core459-page-goto64-route55-origin-v2.jsonl`
(SHA-256 `b68d9534680e4ceba8ceeaad2916ca41617b561f90cf1f776aa095e7ed881b22`)
records **1,801 expected / 30 unexpected / eight skipped / 46,638 not run**;
its full invocation-audit summary SHA-256 is
`120c08c754096dace573108b32e44110e0bd6a30ee1950b27ee3eb7c6c984b10`.
The original runner tooling is archived under
`.work/official-conformance/builds/core-route55-pre-origin-fixture-tooling-20261011/`;
old route and goto receipts reverify using it and their frozen candidate
package archives.

After CDP briefly became unavailable, `chrome-cdp start` restored the
dedicated Chrome endpoint. No extension was disabled. The independent official
Playwright v1.58.0 oracle then ran the same 55 pinned IDs with the
origin-correct fixture: **53 expected / one unexpected / one skipped**. Its
JSON and verified source-bound provenance sidecar are
`.work/official-conformance/builds/core-page-route-public55-origin-v1-20261011-oracle-v1.json`
(SHA-256 `7cc720deb29e3cef7395f14c8a57442f0b883c425588af0c625427e3d4ec836a`)
and the adjacent `.provenance.json` (SHA-256
`2d6a2787cf956ffbe19925db24a99edb365fa2e0a9ac9d138d19cb36af7a77ec`).
The only oracle failure is line 276, `should work with custom referer headers`:
the Chromium assertion expected a duplicate comma-separated Referer but
received one URL. The candidate also fails that row, but its root cause is
not established as the same. Line 578 is upstream-skipped in both lanes.

The immutable 48,477-row oracle successor
`.work/official-conformance/outcomes/playwright-core-oracle-core456-page-goto64-route55-origin-v1.jsonl`
(SHA-256 `5631c74c9d9d60030319696fbed9f4600430db392cc0a3ae1a29be9ef244fb3f`)
records **1,522 expected / 11 unexpected / seven skipped / 46,937 not run**.
Its full invocation-audit summary SHA-256 is
`60f5267aa8ac9b3867bcd3057d7a7cc4dfe42f3fd55faa99d8cc6e74352e4f27`.
Comparing the candidate-v2 and oracle origin-correct reports by exact
`(suite, specId, projectId)` yields:

| Candidate | Oracle | Cases |
| --- | --- | ---: |
| Expected | Expected | 36 |
| Unexpected | Expected | 17 |
| Unexpected | Unexpected | 1 (:276) |
| Skipped | Skipped | 1 (:578) |

The 17 candidate-only unexpected lines are 41, 117, 129, 155, 169, 290,
362, 397, 417, 515, 646, 672, 955, 988, 998, 1010 and 1039. These are
the candidate-v2 paired outcome discrepancies, not yet a proof that all 17 share a single
implementation cause. Neither this 55-case slice nor its 36 paired passes
settles the still-unrun rest of the Core inventory.

The newer opt-in `official-conformance-page-route-header-url` profile adds
Rust-owned `route.continue({headers})` and `route.fallback({url})`, plus the
existing redirect-reinterception feature, without changing the frozen burst
addon. Its selected native addon SHA-256 is
`5b9d820d89c5765bfc1119f68e5f0f761ae17a2679b260d804b122e473ede46d`;
release host route tests pass 30/30, full Node package tests 604/604, and
the addon native-load gate passes. The unchanged origin-correct 55-ID
candidate run passed **40**, failed
**14** and skipped **one**. Its JSON / source-bound sidecar SHA-256 are
`f0c20fdff003f24657c7a9ab906d5f1bdfc364e3f53a2b1d30e726a92f2c0653` /
`b23175ecbb786b4e41b79600c8b633ca04a8b1c9ca06672dc0cba382bd94fcf7`.
The exact package and selected addon are frozen under
`.work/official-conformance/builds/core-route55-headerurl-v3-candidate-frozen-20261011/candidate/`;
the same provenance sidecar verifies against this archive. Versus the previous
candidate run, precisely four rows change to expected: lines 117, 129, 362
and 397. Line 417 advances to the next observable gap, missing
`redirectedTo()` linkage, but still fails. The strict candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-core459-page-goto64-route55-headerurl-v3.jsonl`
(SHA-256 `51a46b47dcca314abf83bb6265f231e24385a95c21bdd80a57ffbfe0a2844174`)
records **1,805 expected / 26 unexpected / eight skipped / 46,638 not run**;
its full invocation-audit summary SHA-256 is
`0e2741a15ec8feb1b85feb13f55beb848f872a17c9fcc7f42ef4187dae1b988e`.
Paired against the same source-bound oracle report, this is **40 both passed /
13 candidate-only failed / one both failed (:276) / one both skipped (:578)**.
An ensuing Node-only `Request.allHeaders()` addition passes 607/607 package
tests and returns an independent, lowercased native-header snapshot. The
subsequent source-bound 55-ID diagnostic again had **40 passes / 14 failures /
one skip**, but not the same 40: line 988 newly passed, while line 695 hit a
network active-request capacity limit (65 over 64) and regressed. Its result /
sidecar SHA-256 are
`fcf1a72c5491a5f79a1c45dd46da925e92e49880f5bd4261f2583ddf1db60d4c` /
`f37340f8d9578f62b34e97c57d369dea8cbc2df1d4b1f8483434ec0b0146af89`.
The exact candidate package is frozen under
`.work/official-conformance/builds/core-route55-allheaders-v4-candidate-frozen-20261011/candidate/`.
This diagnostic was **not** merged into the strict ledger, so the newer line
988 pass and line 695 capacity failure both remain visible without silently
replacing the prior line 695 pass.

Separately, the pinned 36-ID Python `tests/async/test_page_evaluate.py` Chromium
manifest (SHA-256
`921519847572b45ed89160c88dca6f5f0bc14f6d5e6e325872fee81e5ff75a91`)
ran through isolated installed-wheel candidate and official-oracle CDP lanes.
The candidate XML / source-bound sidecar SHA-256 are
`2f031edd780b6d9fedc189c0d42ed3ed07c07d0dc89c5c382fa1ce433692c440` /
`b6c30fc07cf10c9bbed1d63ed5ff4b1dc9ddc3e157db8ebabbb379a42bd61cab`;
oracle values are
`2e997f4e6163e33af505e6e38ded5abd0c18613ffff36c057e9ec93589169426` /
`2b9592965838bcf786b3d7243fcb1a003bce1e09ac4eb96802cdb0e1a10ff8f3`.
All 36 IDs executed in both lanes, with **26 pass/pass and ten
candidate-fail/oracle-pass**. These outcomes are merged into the strict Python
successors at the top; the remaining 5,099 candidate rows are still not run.

## Complete public Core Page.fill file (2026-10-11)

`tests/official/playwright-core-page-fill-public-chromium-v1-manifest.json`
(SHA-256 `bcc762635dfd7f61a26e078484917b7fb6eb0e4c81030e2c7f71ecaa6bd53c09`)
pins the exact v1.58.0 source and all 45 public Chromium `page-fill.spec.ts`
IDs. Before execution both full-inventory lanes classified all 45 as not run;
browserless candidate/oracle collection listed the same 45. The independent
oracle then passed **45/45**; its report / verified source-bound sidecar
SHA-256 are
`b3cb7e4c82f0ead3e17f043f33e737b2a77444d0928dc1ef8d5cd9ae7011c180` /
`5c0378701f68dd0ae50e3150f1c2d48c284ac331330e6a2fd3cb6b1ec0abed1a`.
The candidate ran from the frozen pre-unroute package at
`.work/official-conformance/builds/core-route55-allheaders-v4-candidate-frozen-20261011/candidate/`,
with the opt-in route-header-url addon, and passed **40/45**. Its report /
verified source-bound sidecar SHA-256 are
`ba8e866c1fd41d58061762bd04fbe0c9555ec8d08272f4db7b504c710f238efb` /
`c1edefdd8f90178e4d70ae54bf2044c931a937d2670924e614c97854c67a6df3`.
The five candidate-only failures are line 76 (color-input case handling),
273 and 280 (error text/validation), 388 (fill/navigation lifecycle), and
399 (`id=one` selector syntax). The strict candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-core459-page-goto64-route55-fill45-v1.jsonl`
is **1,845 expected / 31 unexpected / eight skipped / 46,593 not run**
(SHA-256 `3b1a91ec1ef1c85d838ffc37841cb5a67e891b382f450c7e0dcbc3ec814e3415`);
the oracle successor
`.work/official-conformance/outcomes/playwright-core-oracle-core456-page-goto64-route55-fill45-v1.jsonl`
is **1,567 expected / 11 unexpected / seven skipped / 46,892 not run**
(SHA-256 `2d32b91be9b8110586a24183d9cc8c3169312641482cc7925cdcda89f0ccf705`).
Full invocation-audit summary SHA-256 values are candidate
`681e8c0f69fe7a7a557b0dcb7ae8d7e4db9dff7fff717dfb18a42f8643d95d4e`
and oracle
`5c4eb11146aab5e4c21f1fdbf426cf2216e43e49367f008eef372d9c31679b35`.
After freezing the candidate package, the Rust fill script now normalizes
valid color hex input to lowercase before DOM readback comparison; its focused
release test passes 1/1. The frozen official addon predates this edit, so line
76 remains unexpected in the strict ledger until a rebuilt addon passes an
unchanged official rerun. Line 388 is not an error-text issue: navigation can
destroy the evaluation context after the input event but before CDP returns,
and fill needs a bounded commit acknowledgement before treating that as
success.

The post-report Node boundary now labels `Page.fill` errors as `page.fill`,
formats only the exact invalid-target message with the upstream call-log
prefix, and rejects non-string values for both Page and Locator before native
admission. Focused tests pass 7/7; the package suite passes 627/627, with
types and dependency-boundary checks passing. These are source-only checks:
lines 273 and 280 stay unexpected until a source-frozen official rerun.
Separately, the new opt-in `official-conformance-page-route-header-url-unroute`
addon (SHA-256
`4245ff85f7ed744578a78cdd9571d90efce1886542f7e0dd55497355bbd90a27`)
implements string-matcher `Page.unroute` with Rust-owned registration removal.
Its browserless gates pass 620 Node tests, two release Rust host tests, types,
dependency boundaries and native loading. No `Page.route` row has been
reclassified from that addon; it also predates the fill-color edit.

The next source-pinned Chromium Core file is `selectors-react.spec.ts`: manifest
`tests/official/playwright-core-selectors-react-public-chromium-v1-manifest.json`
(SHA-256 `72d40ba44380b0c53dd76c6c82ce7e0c1818574ba14489546fa5c4befcd0ad4a`)
collects all 60 public API cases in the official oracle. Collection is not
execution, so all 60 retain their previous strict outcomes. Its browser run is
waiting for the dedicated Chrome profile to have no unrelated nonblank page;
the conformance runner refuses to attach while one is present.

## Fresh complete Core Page.goto retest (2026-10-11)

The exact 64-case manifest SHA-256 is
`9360becbc634ae4c524541b217acb0a6f476b8d8ba171a8970617dfa4593b3bb`.
The candidate reused the frozen opt-in burst addon (SHA-256
`378f5744e7316636ca26b9586f4a46750c394cb0fb959d3ec499f411051a1b91`)
with current Node source; the oracle used official Playwright 1.58.0. Both
ran sequentially with one worker, the same dedicated Chrome CDP endpoint,
Tailscale-bound fixture, pinned upstream source and 64 selected tests. The
schema-5 sidecars independently reverify against their exact result bytes.
The candidate package source and frozen addon as they stood during this run
are archived under
`.work/official-conformance/builds/core64-pre-js-detach-cleanup-20261011/candidate/`;
the candidate sidecar also re-verifies with this path as `--candidate` after
later JS source edits.
Candidate JSON / provenance SHA-256:
`b09b1fad8077bd03619fda20966067fd96b2ace4601630b7b6a82b1c703e63af` /
`69914402fea4fffdd6dde452f748aa8cdb21cd1741ccaa4b06094f98f702a440`;
oracle:
`6febef040642ba0914abb9f2c8d3f40de026eed07970ac829bbe7b86162f5839` /
`ae82a552bdaa6cee6c483bb37000074b97477f5902f8a58b2c4a5e76e35e37a8`.
Reports are under `.work/official-conformance/builds/` with prefix
`core-page-goto-full64-retest-20261011-`.

Candidate :492 now has official pass evidence, but the same complete run
introduces an unexpected :217. At :217, `page.goto` reports target closed;
teardown reports `available network Request frame is absent from topology`.
The oracle passes :217. This is not yet shown to be deterministic or caused by
the URL bridge. An isolated candidate-only :217 run passed 1/1, and a second
complete candidate run passed :217 and :492, ending **62 expected / two
unexpected** with the same shared :49 and candidate-only :859. The second
candidate JSON / provenance SHA-256 are
`88387cc274e016e067b664544bb4243d4e4d7a73a08a8c6be5bc1051407d28cb` /
`ff763aa1f358b7e27bd2fd0503aa6dc57785ea5706ac6291fa355d7e493f8db5`.
The immutable 48,477-row candidate strict successor
`.work/official-conformance/outcomes/playwright-core-candidate-core459-page-goto-full64-url-v1.jsonl`
(SHA-256 `5519d9e8bd3456edb6eca231961097c76565b6facc3e9f023342930333e114bb`)
changes exactly :492 from unexpected to expected versus core458. Its full
invocation audit retains classification SHA-256
`3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`
and summary SHA-256
`97231c286e6934be32b7190a90b9f119680ae75bc80d34f05bf9d1aa94dbe0eb`.
The selected frozen addon has no Frame-detach capability; :859 has no new
pass credit.

The earlier bounded Python v6 Chromium checkpoint established source-bound
candidate/oracle pairs for Context and focus cases. At that historical build,
the Context init-script callback failed in candidate but passed in oracle,
and the eight-case click selection passed 4/8 in candidate versus 8/8 in
oracle. The ABI108 v2 result below supersedes the init-script and five
click-timeout outcomes; the remaining missing `ElementHandle.click` surface
has not been credited as passing. The complete 70-case click file ran in the
oracle at 69/70, with one shared 404-console failure.
The newest immutable 6,526-row Python strict successors, after the frozen
ABI108 reload/click v2 refresh and an explicitly versioned paired favicon
fixture variant, are candidate
**1,320 passed / five failed / 11 skipped / 5,190 not run** (SHA-256
`a483af2b7379c3b1a7945497041be1dd3a53efcb40046d3966529a00b4140f2f`)
and oracle **1,098 passed / four failed / 11 skipped / 5,413 not run**
(SHA-256 `de156ebcf865eb589ad458eaaaf43c9fa0fae87aed9fe3a7543f7329d1e1363b`).
Both complete invocation audits pass, but full-corpus execution remains far
from complete. The frozen v2 full-profile wheel reports ABI 108 and 97
features on Python 3.9 and 3.12; each interpreter passes 418 package units
with one default-profile-only assertion excluded. The exact unchanged official
Context init-script and five prior click-timeout cases pass in both lanes;
the full click-file diagnostic is candidate 60 pass / nine fail / one
fail-closed report error and oracle 69 pass / one fail. Exact artifacts and
failure records are in
`docs/python-official-conformance.md`.
Cross-tabulating the two immutable Python ledgers by exact ID yields only
**1,041 paired passes**: 279 candidate passes still await oracle execution,
56 oracle passes still await candidate execution, and 5,134 rows remain
`not-run` in both lanes. The full matrix is in the Python evidence document.
An observer-only paired follow-up attributed the shared 404 console line to
Chrome's `/favicon.ico` request against the original fixture. A separately
versioned `offscreen-favicon-v1` fixture plugin, exact one-case manifest and
companion source-bound receipt now preserve the historical runtime hashes.
The unchanged official assertion passes 1/1 in each candidate/oracle lane
under this explicitly qualified variant, and only this one row changed in
each latest strict successor. Original-fixture reports remain failures; the
variant must not be described as a silent upstream test or library fix.
Fresh v6 candidate and independent-oracle full-corpus browserless collections
also each find 6,501 core plus 25 example IDs, with four verified source-bound
sidecars; they add no execution credit.

The complete click files have both been executed on the frozen v2 wheel, but
the candidate's malformed isolated handle-case report prevents whole-file
strict promotion; only the six bounded refresh cases and the separately
qualified one-case favicon variant have new paired strict credit. The
Python `tests/async/test_click.py` corrected v2 manifest (SHA-256
`e3f67855d66abb186fed974dc9b7e614e498acfc58741a557b5146aea373b02c`)
pins all 70 Chromium public-API cases with accurate primary-action labels;
the earlier v1 manifest remains immutable for its historical receipts.
Separate installed-v5 candidate and independent-oracle browserless
collections against v2 each find 70 with source-bound provenance. The exact
eight-case execution is pinned separately by
`packages/pwright-python/tests/official/async-click-eight-chromium-manifest.json`
(SHA-256 `bc6c6c05c77e087d1792d9571f0afa2ab953a6069a0e3df205b83526969f4a8d`);
the complete click-file strict rows are now candidate eight passed / one failed /
61 not run and oracle 69 passed / one failed.

The frozen v6 candidate later executed all 70 selected Python Chromium click
cases diagnostically: 48 passed, 19 failed and three fail-closed report errors.
That source-bound candidate report lacks an uncontested repeat: another task
opened four Chrome pages near the end, and memory pressure rose. It has **no
strict-ledger promotion** beyond the separately paired eight-case selection.
The independent oracle 70-case result is source-bound and promoted on its own
lane; its offscreen-button 404 failure matches the candidate diagnostic but
earns no pass credit. The rebuilt ABI108 candidate improves the paired
eight-case selection from 4/8 to 7/8, with the missing `ElementHandle.click`
still failing. Its complete 70-case click diagnostic records 53 passed /
14 failed / three fail-closed report errors, including five 30-second timeouts
(two previously unsupported offsets, one unsupported force, and two prior
fail-closed report errors);
another task's Chrome pages were present at the end, so this full-file run
has no strict-ledger promotion. The Context init-script case remains
candidate-only failed, now after reload instead of initial Page creation.

The Core `tests/page/page-click.spec.ts` public-only manifest
(`tests/official/playwright-core-page-click-public-manifest.json`, SHA-256
`375a0ef2376dbaed0f4a697e08f3e8f4054215a25c385f2aeada992abbfcf3e6`)
pins 81 `chromium-page` public cases and separately inventories six upstream
private-hook cases. Its fixed selection is fail-closed by exact source, IDs,
file and project. The pinned clean upstream source SHA-256 is
`30c7281301b93bf136a37164c16ffa6bc183be9bd64e0ebb359fcc380c7c0eba`;
selected 81-ID digest is
`57d177934f72fd878e8b76fb48ebb1392bc47936ec7aba745bc5d7bfc9f6bf4b`,
and the complete 87-ID digest is
`eb34a5eeb76d81f30a8ec3cf4b869fd93362d85bbc0df2d51f09013845df8424`.
The official runner's browserless listing finds 81 tests in one file; an
independent upstream JSON listing has exactly the same 81 IDs. Focused
runner/provenance tests pass 61/61, the broader browserless script suite
103/103, Ruff and diff-whitespace checks pass. This listing was not saved
with a provenance sidecar. No Core candidate collection or browser result is
claimed from this preparation.
Source inspection partitions these 81 public cases into 60 Page.click, two
Frame.click, six ElementHandle.click, two Page.dblclick and 11 Locator.click
primary actions. A thin Node Page/Frame selector-click default-first and
`dblclick` facade and the opt-in bounded `Locator.click({steps})` Node slice
now pass 574/574 package tests, focused release Rust tests 2/2 and
typechecking; they are not in the stale browser-tested native addon or an
official browser result. A separate source audit identified the retained
handle identity and frame-coordinate work required for `ElementHandle.click`.
An opt-in top-level backend leaf now retains the exact CDP object identity;
the opt-in host/addon/Python source path is wired. A newer current-source
slice retains an observed creation-world snapshot across the action and
cancellation, rechecks the exact object after pointer movement, validates
under the Page dispatch lock before mouse-down, and gives cancel cleanup an
owned mouse-press token. The current full feature-on backend suite passes
132/132; focused retained and input regression tests also pass. Host-level
world-retirement and cancellation proofs, modifier rollback, raw-Mouse overlap,
per-step movement, iframe geometry, a current-source
installed wheel and official browser verification remain open. No official
pass is claimed for that API.

The separate public-only Core Page-route manifest
(`tests/official/playwright-core-page-route-public-manifest.json`, SHA-256
`19683ee2c92a24d46efddc1c73683ba0d0d4f77c3480802ef8c501acb1d59dfb`)
pins all 55 `chromium-page` public cases from clean v1.58.0 source (SHA-256
`3f543741dc69d44b038024c150b6dbf789a1acc7c2c606594af9d3d442cc023b`;
sorted ID digest
`38fab03fad14cd6a47a9684295e13ca3b7946595dfe52a3129c1d2afef7b22d3`).
Before the paired origin-correct run, the oracle Core strict ledger still had
all 55 `not-run`; both current strict ledgers now include the 55 source-bound
outcomes described above. No private-hook
row is excluded. Official runner and independent upstream JSON browserless
listings each find the same 55 cases. The last test exercises service-worker activation:
plain HTTP on the full Tailscale fixture host may not be a secure origin, so
its eventual paired run must use a verified trusted HTTPS fixture or record
the failure explicitly, not silently drop the row.

Core full-file result preflight now rejects any selected test without a final
status and at least one recorded execution attempt; this prevents a
collect-only listing from masquerading as a browser result. The existing
`official-conformance-page-goto-burst-20` native addon originally predated
the Rust source repair. Two fresh, source-bound one-row paired browser runs
are now recorded below: the first exposes the next capacity limit; the
second passes after a bounded publication repair. Earlier Core
schema-2 sidecars may require their archived tooling for replay; current
tooling hashes alone cannot reverify them after the runner guard changed.

Fresh opt-in burst-21 checkpoint (2026-10-10): the current-source composite
release addon at
`packages/pwright/native/official-conformance-page-goto-burst-20/pwright.node`
has SHA-256
`cb612ebf609e31526efb03de3a094ba3037463662978fc97f5b8777975e6ad29`
and passes selected native-load 1/1. The exact pinned
`playwright-core-page-goto-burst-20` one-case manifest ran unchanged in the
dedicated Chrome candidate and independent oracle lanes. Source-bound reports
and sidecars in
`.work/official-conformance/builds/core-page-goto-burst-21.nWgLVk/`
verify: candidate **0/1**, report SHA-256
`816bca2476e95c06174a766ecf21b0747e4cf0997b9a52f7de822971b4d308df`;
oracle **1/1**, report SHA-256
`a9c35b79128616927112b472f13a97d1ab9171ae01f3093d627b1cfb3b8d28d1`.
The candidate error is `topology/lifecycle request capacity exceeded (8)`
from `page.waitForNavigation` and `page.goto`, not the earlier Frame
navigation 20/21 host-operation limit. This frozen first-run failure receives
no strict credit and is not overwritten by the later success.

The second current-source opt-in addon retains the eight-item lifecycle
queue, but publishes burst navigation fences with bounded async backpressure
and applies the same opt-in path to Page.goto; ordinary publication remains
fail-fast. Focused release burst tests pass 10/10, including 21 fences
through a saturated eight-item queue and stop/deadline cancellation; default
Frame navigation tests pass 5/5. The addon SHA-256 is
`378f5744e7316636ca26b9586f4a46750c394cb0fb959d3ec499f411051a1b91`
and selected native-load passes 1/1. The **same** pinned upstream line-629
case passes **1/1 candidate and 1/1 independent oracle**; exact v2 reports
and verified source-bound sidecars remain under
`.work/official-conformance/builds/core-page-goto-burst-21.nWgLVk/`:
candidate `candidate-core-page-goto-burst-fence-v2.json` SHA-256
`f22b06fd1bc778a2a98b89a6bb975525a0d6534c7909f950a7dd16c2e31d3c11`,
oracle `oracle-core-page-goto-burst-fence-v2.json` SHA-256
`a9cf3e7fae2a7afd4407d5161628250ef6afdfbbf4fd9cbc4018ac5389bb55b7`.
The manifest SHA-256 stays
`023527fbced185707d95a5513b34ae468cca6c65912f40f18773812ead3cab57`.
The immutable 48,477-row candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-core457-page-goto-burst-20-v1.jsonl`
(SHA-256 `07832667f3bf0f85c60e9685394992cae42ecceaa9fac47e9a8038cf8529bed3`)
changes exactly the chromium-page line-629 row from unexpected to expected:
**1,761 expected / 16 unexpected / seven skipped / 46,693 not run**. Its
summary SHA-256 is
`b3da6b6ca7017f59997ff64184e1a6a1f4b3806db5f670b1df69b3bf501f5af3`;
the already-expected oracle ledger is unchanged. This is one case of the
complete Core corpus, not full Page.goto-file or all-JS/TS conformance.

Fresh complete Core Page.goto checkpoint (2026-10-10): the unchanged pinned
64-case `tests/page/page-goto.spec.ts` Chromium selection ran with the same
opt-in addon SHA-256
`378f5744e7316636ca26b9586f4a46750c394cb0fb959d3ec499f411051a1b91`.
Both lanes started with no CDP tabs; their reports and source-bound provenance
in `.work/official-conformance/builds/core-page-goto-full64.BxRoe8/` pass
current verification. Candidate is **61 expected / three unexpected**, report
SHA-256 `b38d6352575703efd057b54ef085e43c6a7b517a30273f99469d8e64b8d1d141`;
independent oracle is **63 expected / one unexpected**, report SHA-256
`701f84d334d3c0930995d57103b4692901b477ce962b83f2fb393986419b8c43`.
The protocol-less MagicDNS URL (:49) fails in both lanes. Candidate-only
unexpected cases are the JavaScript-redirect `toHaveURL` assertion (:492),
which reaches missing Node Context `_options.baseURL`, and the
iframe-attach/detach `framedetached` wait (:859), for which no exact native
Frame-detach event reaches the Node Page emitter. The raw-body assertion
(:756) fails in both lanes but is annotated `test.fail` by upstream for
Chromium, so it is `expected`, **not** a candidate discrepancy.
After this frozen full-file report, an opt-in Node source bridge now exposes
Context `_options.baseURL` and supports `Frame._expect('to.have.url')` through
the existing Rust-owned `frameWaitForUrl` lifecycle/timeout loop. The default
profile remains unchanged; the official matcher Page identity, optional
`signal`/`title` fields and `received: { value: url }` result are covered.
Focused Node tests pass 12/12, package tests 587/587, types and dependency
boundaries pass. This is **source/unit evidence
only**: the pinned :492 official candidate/oracle assertion has not been
rerun, so its strict row remains unexpected.
The opt-in `frame-detach-events` source path now carries exact retired Frame
identity through bounded host publication and a Node event bridge, with
deterministic child-first and sequential-revision tests. It is not a new
official result: precommit child admission and reconnect cleanup remain
unresolved, the selected addon has not been rebuilt, and :859 has not been
rerun. See `docs/node-ffi-compatibility-design.md`.

The immutable 48,477-row candidate successor
`.work/official-conformance/outcomes/playwright-core-candidate-core458-page-goto-full64-v1.jsonl`
(SHA-256 `81b8dcb18287c2582cac834cef7414835acc8e86a8a6e34fac08512ca1c191f5`)
changes exactly three newly resolved rows, :471, :543 and :740, from
unexpected to expected versus core457; :629 was already promoted by its
one-case run. The full invocation audit SHA-256 is
`3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`.
Current strict candidate totals are **1,764 expected / 13 unexpected /
seven skipped / 46,693 not run**. This does not complete Core or Python
execution. Two unrelated XLQY tabs appeared after the paired run and were
left untouched; further shared-CDP browser validation requires a fresh
isolation preflight.

An earlier complete source-bound Python BrowserContext current-wheel rerun completed
all 39 selected cases: 32 passed / seven failed. Only the 35 cases passing in
the independent oracle are promoted to the strict candidate successor:
29 passed / six failed, improving seven earlier candidate failures. The four
oracle-failing popup cases remain `not-run` in both strict ledgers. A separate
complete 29-case independent keyboard oracle run passed 28 with one upstream
skip; a complete 18-case Page-route selection passed 18, of which 17
applicable public-API cases received new oracle credit and one private-helper
case was explicitly excluded. The historical v4 6,526-row successors at
this checkpoint recorded candidate **1,295 passed / nine failed / 11 skipped /
5,211 not run** and oracle **1,021 passed / four failed / 11 skipped /
5,490 not run**. The
source-bound receipts, hashes and per-row audits are in
`docs/python-official-conformance.md`. The unchanged Context forced-colors
case later passed 1/1 in both installed v4 candidate and independent oracle;
only its applicable candidate row changed from failed to passed. Five of the
historical BrowserContext candidate-only failures remain. A v4 39-case rerun
was interrupted after 13 isolated reports when binding execution timed out
and dedicated Chrome CDP became unreachable; it has no aggregate/provenance
or strict-ledger credit. The dedicated Chrome later recovered externally. A
subsequent unchanged seven-case Chromium `BrowserContext.clear_cookies`
selection passed 7/7 in both v4 candidate and independent oracle, with
source-bound provenance. Exactly seven previously unrun oracle rows advanced;
the candidate's seven pass outcomes were already recorded and gained only
current-v4 evidence. Its immutable strict oracle successor SHA-256 is
`accd58457eb6ff6ba7459e7fa1d11bf9f2f3b35956ac836865ed74d2eb94db4d`,
candidate successor SHA-256 is
`7d574909bc125ed0869fe0ec15fd1cc3d15aadc2c314fe2cc47bc9f33814161b`;
both invocation audits pass. The candidate-pass/oracle-not-run gap is 280.
Neither Python-corpus nor Core JS/TS conformance is complete.

At the historical v2 checkpoint, the 48,477-row Core candidate and oracle v1 ledgers align by ID:
1,463 rows are `expected` in both lanes, while 296 are candidate-`expected` /
oracle-`not-run`, six are candidate-`unexpected` / oracle-`expected`, and
46,693 are `not-run` in both. Other outcome pairs remain recorded in the
ledgers. `expected` is the upstream runner's classification, not a claim
that every such row is a new public-API browser pass.

Opt-in Core Page navigation composition checkpoint (2026-10-10): the new
`official-conformance-page-goto-burst-20` profile selects the existing
38-feature official Page union plus only `frame-navigation-burst-20`. Its
release native addon at
`packages/pwright/native/official-conformance-page-goto-burst-20/pwright.node`
has SHA-256
`3dfb2ac8698de70f5b2416688b2d15de879251d9d7bf5c5549474ecb9fbd5c83`.
Browserless native-load probes for this profile and both prior profiles pass;
the default and prior official/standalone burst selection remain unchanged.
Package tests pass 565/565 and focused contracts 122/122, with TypeScript and
dependency boundary checks passing. This is a build/load checkpoint, not an
official JS/TS browser result; the strict Core ledgers are unchanged.

The specific upstream `should not leak listeners during 20 waitForNavigation`
row (`page-goto.spec.ts:629`) now has a separate one-row pinned manifest at
`tests/official/playwright-core-page-goto-burst-20-manifest.json` (SHA-256
`023527fbced185707d95a5513b34ae468cca6c65912f40f18773812ead3cab57`).
The manifest-owned exact title filter, source hash, project, file and test-ID
are enforced by the runner and provenance replay; ad-hoc filters remain
rejected. Runner/provenance tests pass 50 cases, and browserless upstream
collection returns exactly one row. A paired candidate/oracle browser attempt
was refused before execution because the shared dedicated Chrome contained
two unrelated nonblank XLQY pages. Only this task's blank tab was closed;
those pages were left untouched. There are no result/provenance reports or
strict-ledger changes for this new manifest yet.

Prepared Python Chromium focus shard (2026-10-10): the exact
`packages/pwright-python/tests/official/emulation-focus-chromium-manifest.json`
(SHA-256 `c6e1449fbbfc55bfc6418d4cb51ee1b6e7bfeb04e123a9d173f72ea6cda708c3`)
pins the unchanged v1.58.0 `tests/async/test_emulation_focus.py` source
(SHA-256 `45bcb78e451c3f54eb729ec595b83cf4d628a1805d4d78399ef69030592bff7e`)
and fixture chain. Its seven public-API Chromium rows are `not-run` in both
latest Python strict ledgers. Source-bound browserless candidate and oracle
collections each find 7/7; no browser execution, promotion, or exclusion
occurred. Exact report/provenance pairs are in
`docs/python-official-conformance.md`.

Context init-script source repair checkpoint (2026-10-10): comparing the
unchanged failing official test with pinned upstream Chromium code exposed a
specific parameter gap. Already-registered Context scripts installed while a
new Page session initializes now send CDP `runImmediately: true`; dynamically
added scripts still target future documents only. Fake-CDP payload regressions
and focused release tests pass 6/6 in both standalone and combined Context-
binding profiles; strict release Clippy passes. The v4 wheel and its official
result reports predate this source repair. No official rerun, strict-ledger
credit, or resolution of the distinct exposed-function timeout is claimed.

The exact single official Python Chromium init-script callback case is also
prepared as a separate pinned manifest (SHA-256
`fdaecf074e6f043415fe74a93c4ae6f541b76102840aa280fc65c67600932f15`).
Candidate-v4 and independent oracle collect one item each with source-bound
sidecars; zero tests were executed in collect-only mode. This isolates a later
new-wheel browser rerun from the interrupted 39-case Context shard.

Prepared public Page set-content selection (2026-10-10): the existing
17-row upstream source remains intact and fully inventoried. A new manifest
`tests/official/playwright-core-page-set-content-public-manifest.json`
(SHA-256 `47309cfb41f978f065e74a4af687b64b50e1b343699bf9df1c6a4b1703df43f1`)
pins exactly 15 public-API Chromium spec IDs and separately lists only the
two implementation-internal `toImpl` timeout cases excluded from this bounded
run. The fixed manifest-owned invert filter, upstream source hash, project,
file, 15-ID digest and complete 17-ID digest are checked by runner/provenance;
ad-hoc invert filtering is rejected. Browserless official collection returns
15 tests in one file and its JSON ID set exactly matches the manifest;
runner/provenance tests pass 55 cases and 18 subtests. The collection JSON was
not saved with a provenance sidecar, and no browser case or strict Core ledger
row has run for this selection.

New Python candidate packaging checkpoint (2026-10-10): v5 fixes the
new-session init-script parameter in a fresh ABI107, v4-compatible 96-feature
wheel (SHA-256
`0229c917a9745f9161eb71f7171706788fa000d499660829effcb7730585c510`).
Its independent rebuild (SHA-256
`67f039c9e049a80d102b8915268bd35600b5dbc148b002b1a57b9b41a3057068`)
has the same normalized payload and native SHA-256
`5c580c78a239c352d59ad6abcd4a2335fef9bd9b4a`. Python 3.9 and 3.12
isolated installed package units pass 380/380 each. All 81 release
dependency/feature checks, addon strict Clippy, pinned Ruff and expanded CI
Pyright pass. Four formerly omitted official-tool test scripts are now in
the CI Pyright command; its expanded check reports zero errors. This newer
wheel subsequently ran the three bounded official browser selections
described at the top; the v4 outcomes above are historical, not the newest
strict ledgers. Other-task XLQY pages were left untouched.

Prepared Core Page set-content shard (2026-10-10): the unchanged upstream
`tests/page/page-set-content.spec.ts` source has SHA-256
`3909ec532e104dc20176328f3870c1768141b2a270facbaf187169fa73c312c4`.
The exact `tests/official/playwright-core-page-set-content-manifest.json`
(SHA-256 `384d2aed723b65859805c497d896e237510b8ebea45917d5a28009f07d6e39ac`)
pins the v1.58.0 commit, complete `chromium-page` file, 17 rows, and sorted
test-ID digest `8adebc2af56898da76a85c8b839bcf70d0acd9168f1616ff039e77e9bb3c8d5a`.
The pinned upstream checkout is clean and browserless oracle collection lists
17 tests in one file. Both current strict Core ledgers still have all 17 rows
at `not-run` with zero attempts; collection grants no execution credit. The
invocation audit classifies 15 rows as public-API candidates and the two
`toImpl` cases at lines 133 and 151 as implementation-internal. The latter
installs an intentional infinite loop in Chromium's utility world, so full-file
execution needs an uncontested dedicated-CDP window and bounded teardown
verification. No candidate or oracle browser execution has occurred for this
shard.

Full-file Core navigation shard (2026-10-10): the pinned upstream
`tests/page/page-goto.spec.ts` source (SHA-256
`0f44b2ac6700b97f50ce67051e8728cb4c7b19c914846e99e8deaca7de37eb87`)
has 64 `chromium-page` public-API rows, initially all with zero attempts and
`not-run` in both Core strict ledgers. The exact one-file
`tests/official/playwright-core-page-goto-manifest.json` (SHA-256
`9360becbc634ae4c524541b217acb0a6f476b8d8ba171a8970617dfa4593b3bb`)
pins the v1.58.0 checkout, project, file, expected 64 rows and exact sorted
test-ID digest (`c903711b56685777d92aabbbd82d92b5579b7b2b6a1111c7ecf868d4bf96d874`).
The runner now rejects source drift and ad-hoc test/grep filters; capture and
provenance replay reject any missing, substituted, duplicate or wrong-project
row. Runner/provenance unit tests pass 25/25 and 23/23. The official Core
oracle collection lists 64/64 from one file without launching a browser.
Collection alone gave zero conformance pass credit.

For this shard only, the server fixture now exposes `CROSS_PROCESS_PREFIX`
through a second, ephemeral Tailscale-bound TCP port sharing the primary test
server's route state. Its five no-browser tests pass against the pinned
upstream fixture, including teardown after startup failure. A different port
on the same full MagicDNS host is cross-origin but not necessarily cross-site
or a separate Chromium process: cross-process behavior remains unproven even
if a named upstream test passes. The complete candidate file subsequently ran
in an isolated shared-CDP window, with its results recorded below.

The unchanged official oracle then executed the complete pinned 64-case file
through the dedicated CDP adapter: 63 expected passes and one unexpected
failure, with no skips. The single failure is
`should use http for no protocol` (`page-goto.spec.ts:49`, ID
`4871301ada6321d1ce67-277321741469057ee766`): removing `http://` from the
full MagicDNS fixture URL produced `net::ERR_ABORTED`, unlike the upstream
localhost fixture. It remains a recorded failure, not a candidate discrepancy
or a hidden skip. Pinned upstream `completeUserURL` prepends `http://` only
when a protocol-less URL begins with `localhost` or `127.0.0.1`; the required
full MagicDNS hostname meets neither condition. A `localhost@` userinfo prefix
would change the test URL's semantics, so it is not a justified fixture fix or
pass credit. The exact JSON result is
`.work/official-conformance/results/playwright-core-oracle-page-goto-chromium-v1.json`
(SHA-256 `8706c5bd1f94efd00dd5b8d81d8170c12c82c5ad33da6cda757f663aa641b08a`);
its source-bound provenance sidecar SHA-256 is
`632890128ad9b8ca83990469a6be877c4b063cd1ed4801c49350144e2d02b8ff`
and verifies against the current pinned source and harness. The immediate
candidate attempt correctly refused two newly reopened, other-task nonblank
Chrome pages before executing a case or producing a result; this task did not
close those pages. The task-owned blank CDP tab was closed afterward.

The verified Core oracle result is now incorporated into the immutable strict
successor
`.work/official-conformance/outcomes/playwright-core-oracle-core456-page-goto-chromium-v1.jsonl`
(SHA-256 `b3bfe2c43c130198d326710bc42aab34df19ad685659200352fca24b05ae542e`).
Exactly the 64 previously `not-run` `chromium-page` rows changed: 63 to
`expected` and the protocol-less URL row to `unexpected`, with one attempt
each. Its 48,477-row totals are 1,469 expected / six skipped / ten unexpected /
46,992 not run; all other 48,413 rows are unchanged. The invocation audit at
`.work/official-conformance/audits/playwright-core-oracle-core456-page-goto-chromium-v1-invocations.jsonl`
(SHA-256 `3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`)
passes. This oracle ledger gives no candidate credit; the separate candidate
result is recorded below. A cross-origin same-site fixture remains
insufficient evidence for Chromium process isolation.

The candidate then ran that same complete pinned 64-case file with the
`official-conformance-page-dispatch-event` native profile (artifact SHA-256
`ff8e5db65ab83dca3e7f53fb22b493c81167ece9f7afcdb40e1424d2327ef34d`).
It recorded 45 expected / 19 unexpected / zero skipped. The full JSON report
is `.work/official-conformance/results/playwright-core-candidate-page-goto-chromium-v1.json`
(SHA-256 `fa7d07dd8e2274b7e5f01196a26af766f517fda9bd239e5e4aa79871a6ba6a73`);
its source-bound provenance sidecar SHA-256 is
`7cb293cb3abbb1856cb7cb762fbb9b6b1f6d00701eff13cf19adcdb2885efddb`,
and replay verification passes against current pinned sources. The one
protocol-less MagicDNS URL failure is shared with the oracle; the other 18
unexpected IDs are candidate-only discrepancies requiring triage and fixes.
They include wait-state option parsing, inherited navigation timeouts,
interrupted-navigation ordering, request/event and referer behavior, and
error shaping. An upstream `test.fail`-annotated raw-body case is `expected`
in both lanes despite its assertion failure; raw result statuses must not be
mistaken for unexpected outcomes. No candidate row is promoted without this
verified full-file report and the strict successor audit. That immutable
successor is
`.work/official-conformance/outcomes/playwright-core-candidate-core456-page-goto-chromium-v1.jsonl`
(SHA-256 `720ac506039bf4cc863baff4bb15b9bbb92b7bd00f0b67eb23063d398a56c5bd`):
1,748 expected / seven skipped / 29 unexpected / 46,693 not run across 48,477
rows. Only the 64 target `not-run` rows changed, to 45 `expected` and 19
`unexpected`; 48,413 unrelated rows are byte-identical. The complete
invocation-classification audit is
`.work/official-conformance/outcomes/playwright-core-candidate-core456-page-goto-chromium-invocation-classifications.jsonl`
(SHA-256 `3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`).
The first bounded repair accepts upstream's `networkidle0` alias, matches the
invalid `networkidle2` enum error, and uses the Rust-owned effective navigation
timeout for Node `Page.goto`, `Page.reload` and `Frame.goto` when the caller
omits an explicit value. A read-only review caught and corrected error mapping
and validation order in that new getter path. Five focused Rust release tests
and 29 Node adapter tests pass. The selected release native artifact has now
been rebuilt (SHA-256 `f9d8ff075d80193223160d7280b73fd36df8c9bf7cd723850390098745747d45`)
and its ABI/profile smoke test passes 1/1. Another task was actively using the
shared dedicated Chrome at this repair checkpoint, so no post-repair browser
rerun was performed then; the v1 64-case report and ledger were the pre-repair
baseline.

That artifact is historical. A cumulative release addon including the later
referer-policy, `data:` request suppression, and undefined navigation-timeout
reset changes has now built as SHA-256
`51b14eec29f4b858e611d7b8ce5b6ded95043f11bbcb0a660f65b860f6ca43dd`;
its ABI/profile smoke passes 1/1, browserless Node contract checks pass
563/563, and strict TypeScript passes. Focused timeout-reset Rust release and Node
adapter tests pass 9/9 each. Three error-presentation cases now have a thin
Node call-log fix with 4/4 focused tests. Fractional timeout and the other
navigation discrepancies remained open at that checkpoint. The shared Chrome
reopened two non-task nonblank pages when that complete rerun was being
prepared; the attempt stopped without new official result or strict-ledger
credit. A later complete v2 rerun is recorded below.

Read-only comparison of those discrepancies identified three independent
Rust-owned gaps, with the oracle `expected` in each case. The cross-origin
referer case (ID suffix `6b18938422f06c7e7d58`) observed no referer:
the tested top-level `Page.navigate` omitted CDP's
`referrerPolicy: unsafeUrl`, although the Frame route set it. The `data:`
navigation case (ID suffix
`ea231467226238b76c4a`) observed one request event rather than zero:
the CDP network-event projection creates a request for every URL, whereas
upstream drops `data:` before request creation. The 20-concurrent
`waitForNavigation` case (ID suffix `3a6624a37d0b3667dc13`) exhausted the
opt-in profile's four per-Page registration slots and eight host operation
slots. The top-level referer-policy path is now locally repaired through a
shared typed CDP navigation-parameter constructor; two new serialization
tests and the `pwright-cdp` release library suite (133/133) pass. This repair
is included in the cumulative artifact above. At that local-repair checkpoint
the official cross-origin referer case had no new pass credit; the v2 full-file
rerun below now records it as `expected`. The CDP network-event leaf locally
suppresses public Request creation and companion events for `data:` URLs. It keeps
bounded, resource-budgeted native-ID correlation through late ExtraInfo,
same-ID reuse and session/Page drain; default, request-frame/request-timing,
and model release suites pass 16/16, 24/24 and 12/12. Completed correlation
entries may be evicted at capacity, so arbitrarily late tails after eviction
remain an explicit limitation; a `data:` reuse of an active HTTP request ID
is rejected rather than asserted compatible. This change is also included in
the cumulative artifact. At that checkpoint it had no official-test credit;
the v2 rerun below records the `data:` case as `expected`. The 20-waiter
capacity gap remained at that checkpoint. A read-only path
audit counted 20 Page registration slots and 20 Frame host-operation slots;
the then-current `page.goto` used a separate command. Current source delegates
`Page.goto` through `mainFrame().goto()`, which needs an additional command
host-operation slot while the 20 waiters are pending. Thus the proposed narrow
profile had to preserve a **20-waiter registration limit** yet allow at least
**21 concurrent host operations** for this page; simply raising every waiter
limit to 21 would break the upstream 21st-waiter rejection invariant. The
Frame route-snapshot ceiling (eight), result permits (32), and FFI command
ceiling (32) are not the immediate blockers. A naive per-Page increase from
four to 20 slots across 32 possible active/escaped Page owners raises the
declared fixed-arena lower bound from 66.78 MiB to 333.91 MiB, before a
measured runtime peak. The newer source implements a typed, opt-in 20-waiter
pool plus one command host-operation slot; focused release navigation and
lifecycle tests pass, while a current-artifact runtime memory measurement,
cancellation/close reclamation proof and the unchanged official full-file
case remain separate gates. Capacity alone has not yet been shown to make
that case pass.

The opt-in Frame host-operation allowance itself is 21 × 48 KiB = 1008 KiB,
versus the ordinary eight × 48 KiB = 384 KiB (+624 KiB). This is a declared
capacity component, **not** measured RSS or the total lifecycle/FFI bound.

The subsequent complete pinned 64-case candidate v2 official rerun used the
cumulative release artifact SHA-256
`51b14eec29f4b858e611d7b8ce5b6ded95043f11bbcb0a660f65b860f6ca43dd`.
Its unchanged-source JSON result is
`.work/official-conformance/results/playwright-core-candidate-page-goto-chromium-v2.json`
(SHA-256 `236ba4368451a44bf53906e6c756deae1517b9103ea2b66cbbd6de3cf7201621`),
and its schema-2 provenance sidecar is
`.work/official-conformance/results/playwright-core-candidate-page-goto-chromium-v2.provenance.json`
(SHA-256 `f661975bb83e56008e173690e5aac0cd6439c30a6a646df6618e3abbc400cca6`);
provenance verification passed with the then-current runner. Its schema-2
sidecar no longer replays with today's changed runner source hash; the
archived verifier is needed before claiming current replayability. The
complete historical shard records **57 expected /
seven unexpected / zero skipped**, improving 12 v1 `unexpected` cases to
`expected` with no other status changes. One of the seven remaining failures
is the protocol-less MagicDNS URL case shared with the independent oracle.
The other six are candidate-only: replaced navigation, JavaScript redirect,
pending navigation, 20 concurrent `waitForNavigation` listeners, canceled
navigation, and iframe attach/detach load waiting. The cross-origin referer
and `data:` request cases are among the now-`expected` results; the 20-waiter
capacity failure remains open.

Read-only triage of those six candidate-only v2 rows identifies distinct
owners; it is not a new browser pass. In the frozen v2 artifact, replacement
(:471), pending-navigation ordering (:543), and cancellation (:740) enter the
older top-level `Page.goto` host/backend path rather than the separate Frame
navigation executor. Current source now delegates `Page.goto` to the native
main-Frame executor when that capability is installed, matching the upstream
client's dispatch while retaining the v1 Page fallback without it. The focused
Node adapter passes 30/30, the package passes 575/575, and TypeScript and
dependency boundaries pass. At that historical checkpoint this source-only
routing change had no rebuilt addon or official browser result, and the
later one-row burst pass does not prove those three other cases fixed. The
redirect assertion (:492) fails first because the Node Context
lacks upstream `_options.baseURL`, but adding that field alone is insufficient:
the official `toHaveURL` matcher then calls `mainFrame()._expect('to.have.url')`,
which the candidate Frame does not expose. The 20-listener row (:629) retains
its recorded admission-capacity failure; current `Page.goto` routing also
requires a command slot beyond the 20 waiter slots. A subsequent opt-in
current-source repair separates the 20 waiter permits from one command
host-operation slot; focused release Rust tests pass. The first fresh paired
rerun above exposed the lower topology/lifecycle eight-request cap; a second
opt-in bounded-publication repair passes the same one-row case and changes
only its strict candidate row. The iframe detach row (:859) cannot be repaired
with a JS-only event alias: Chromium topology consumes `Page.frameDetached`,
but the host lifecycle publication and Node addon do not carry an exact
detached Frame notice/identity. Mapping the existing main-frame navigation
notice to `framedetached` would emit false detaches. No Core outcome or ledger
is changed by this source review.

After that v2 official run, a narrow opt-in `frame-navigation-burst-20`
profile was implemented locally. A shared RAII permit pool caps concurrent
Frame navigation registrations at 20 across the connection, while the default
profile retains four Page slots and eight host operations. The Node native
profile has a distinct build-identity probe and rejects a stale addon. Rust
release plugin/host checks pass 3/3 and 560/560 (eight pre-existing Chrome-only
tests ignored); Node package tests pass 564/564, types pass, and the actual
release addon at `packages/pwright/native/frame-navigation-burst-20/pwright.node`
(SHA-256 `8971cab639d711e6e93013e9c5cb2c0b862b0d942c3b3d0da64b886723652fe4`)
passes its browserless native-load probe. The 21st permit is rejected and
cancellation releases permits in local tests. This is **not** a rerun of the
official 20-waiter case; the strict Core statuses above are unchanged. Python
does not yet select this burst profile.

The immutable 48,477-row strict candidate successor is
`.work/official-conformance/outcomes/playwright-core-candidate-core456-page-goto-chromium-v2.jsonl`
(SHA-256 `1e124b3170c786cfcb4377cd4792ac4346d2de914a71f350ec83770c5cfa1c8f`):
**1,760 expected / seven skipped / 17 unexpected / 46,693 not run**. Against
the v1 strict base, exactly 64 rows changed, all in
`tests/page/page-goto.spec.ts` for `chromium-page`; 12 changed outcome from
`unexpected` to `expected`, and the other 52 were refreshed by the full rerun
while retaining their outcome. No unrelated row changed. The complete invocation
audit passes with unchanged classification-ledger SHA-256
`3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`.
The oracle's page-goto shard remains at 63 expected / one unexpected from its
v1 run above; this candidate rerun does not add oracle credit or prove Chromium
cross-process isolation.

Current owned-launch checkpoint (2026-10-10): public Node and Python
Chromium `BrowserType.launch` are now opt-in Rust-backed paths with explicit
executables and temporary profiles. Python ABI107 and Node package units pass
330/330 and 511/511 respectively; their prior release/type/boundary checks
pass. The optional owned-launch Node addon and isolated Python wheel were
rebuilt from the latest host guard. The default Node addon remains unchanged;
the editable Python extension was later replaced only after the exact ABI109
package, harness and tooling snapshot below reverified.
At that checkpoint the Python strict 6,526-row ledgers contained 1,262 candidate and
819 oracle passes, with 5,249 and 5,693 respectively not run; these are
incremental rejection-path checks, not full-corpus completion.

The exact pinned Python v1.58.0 async numeric `proxy.server` test is a new
browserless `owned` tranche: both oracle and candidate pass all three engine
variants after a narrow Python protocol-shape check. It does not enable valid
proxy or Firefox/WebKit launch. The exact-source manifest SHA-256 is
`7e748448269ecbc69e9c82cf9cbf3d9b1f5c5f514e780e15194e3f495a56c43a`;
the runner pins test/fixture and pre-spawn implementation sources, disables CDP
and inherited injection hooks, and verifies schema-5 provenance against live
sources and a pre-run archive. No new browser PID appeared in process snapshots;
the official Node driver and Tailscale-bound fixture servers did run. The
candidate strict successor has 1,262 passed / 11 skipped / four failed /
5,249 not run (SHA-256
`9abb589487cadbb01d379876e6fdb4bd08d093bcaf10c246e4ab0845946841e8`);
oracle has 819 passed / ten skipped / four failed / 5,693 not run (SHA-256
`e396771db5ad1b646ac88214af3f5b05291e543a3459200c95bf2d92769d9523`).
Only the three previously `not-run` public-API keys changed; both complete
applicability audits retain SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The Python official `owned` harness disables both candidate and oracle CDP
launch adapters. Candidate and oracle each collected 12 selected launch
preflight invocations; collection-only JUnit records zero executions. Both
lanes then passed the same four unchanged Chromium async/sync argument and
nonexistent-executable tests, with verified schema-4 provenance. Eight more
Firefox/WebKit async/sync preflight rows ran separately without launching a
browser: each lane recorded six passes and two upstream Firefox skips, with
four more verified sidecars. At that preflight checkpoint the strict Python 6,526-row ledgers showed
candidate 1,256 passed / 10 skipped / four failed / 5,256 not run (SHA-256
`67e9d92c0ffb5a020bf9fa207cdac90871fc32789f2854cc3cef8f22cc8d4969`)
and oracle 812 passed / nine skipped / four failed / 5,701 not run (SHA-256
`1c7fba90cd6c29e030bf5bd5e63248bf7e8504c5b212bc61fdd79d1b56667686`).
These rejection/skip paths do not prove a successful owned browser launch. The
native default Context is hidden from ordinary fresh-session collections; a
later host-source guard covers many guessed-ID methods but is not a complete
isolation proof. Positive launch/close, browser
process-tree cleanup, Firefox/WebKit providers and valid BrowserServer remain
open. The separate Core promotion below does not change these Python counts.
Separately, the pinned browserless Python `test_should_work_with_glob` ran
3/3 in both lanes across Chromium/Firefox/WebKit with six verified schema-4
sidecars. The 6,526-row ABI109 successor ledgers change only two candidate
and three oracle `not-run` keys: candidate 1,258 passed / 10 skipped /
four failed / 5,254 not run (SHA-256
`865700ea62525460b5f64e267c43cea91a4e527e33a4f14dff3514bc05edfa95`),
oracle 815 passed / nine skipped / four failed / 5,698 not run (SHA-256
`1ee5d32fe559121c2d07aa051bec77a00fe6a78fa6fc4e31e0455d2d6dc70f18`).
The formal applicability audit labels all three glob rows
`implementation-internal`/`non-applicable` because the test imports upstream
private helpers: this is whole-corpus execution evidence, not additional
public-API compatibility evidence.
Candidate-only diagnostic has since launched installed Chrome through
Pwright and passed both pinned official Python Chromium async/sync
close-twice tests, with a verified schema-4 sidecar. The independent oracle
was not run because its genuine launch would control Chrome through official
Playwright rather than Pwright, conflicting with the global browser rule;
those two rows stay `not-run` in both strict ledgers. Browser-process/profile
identity and child-tree cleanup remain unproven by that diagnostic.
For Core, a separate exact-source `chromium-library` one-spec owned-launch
manifest/runner is prepared for the pinned `should be callable twice` case at
`tests/library/browsertype-launch.spec.ts:130`. It uses real public launch,
not the historical CDP attach adapter. Browserless official collection finds
exactly one spec, runner/preload checks pass 10/10 plus 31/31 existing Core
runner checks, and the opt-in release addon is built. No Core owned browser
or official spec has been executed; its prospective sidecar explicitly marks
diagnostic evidence without process/profile attestation. The global
`chrome-cdp` policy needs an explicit narrow exception before this true
owned-launch run. This positive-launch Core row remains `not-run`.
Separately, pinned Core `tests/library/browsertype-launch.spec.ts:75`
(`should reject if executable path is invalid`) passed 1/1 in both candidate
and oracle without starting a browser. Its exact one-row manifest is
`tests/official/playwright-core-browserless-launch-invalid-executable-manifest.json`
(SHA-256 `16be68d3b2368a5fff8a9f201cf697c886ec450c03cf1982f0c34ac7b382a98b`);
both result provenance sidecars verify. The 48,477-row core443 strict ledgers
promote only that key from `not-run` to `expected`: candidate 1,604 expected /
six skipped / seven unexpected / 46,860 not run (SHA-256
`5a74cb8955c1d717f399a2791ee56e55c6a7783306a77173355e2ef5c9a80bef`),
oracle 1,307 expected / five skipped / six unexpected / 47,159 not run
(SHA-256 `9a341bfca76a845f33aaeb1c61e2b75af131bd1ea104b9817205971ed5957d66`).
The full pinned invocation audit validates both 48,477-row ledgers and emits
the same classification SHA-256
`927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`.
This proves only public JS error preflight, not Rust process ownership or
successful launch. A second browserless Core tranche selects the unchanged
pinned Chromium Library tests `browsertype-launch.spec.ts:40`, `:56`, and
`proxy.spec.ts:23` (user-data-dir/page argument rejection and invalid proxy
server), with exact-source manifest SHA-256
`4f7f527c2f63ce4a1bc9bd8cc67f65d566cd5d1f54cd7a54651ef3d23fc63ab5`.
Candidate and oracle each pass 3/3 with reverified source-bound provenance;
the core444 48,477-row ledgers change only those three `not-run` keys to
`expected`: candidate 1,607 expected / six skipped / seven unexpected /
46,857 not run (SHA-256
`791eee8a7fb2c35d2151862adbf1f569eab24be9f35cda1e787d83fd2ac1a223`),
oracle 1,310 expected / five skipped / six unexpected / 47,156 not run
(SHA-256 `5a1ec13430fc57ccb1301e7b6eefd6647c1d354e96d6a19b80540ae3aec5deb2`).
These paths reject before spawning a browser and do not prove a positive
Rust-owned launch. A third browserless Core tranche executes the same pinned
invalid-executable test at `browsertype-launch.spec.ts:75` for Firefox and
WebKit Library. Each engine passed candidate/oracle 1/1, all four provenance
sidecars verify, and no browser started. Its manifest SHA-256 is
`455f923e5866f39055b802f996135ab9271dd0133543ec7c21f74fd66cc4d44b`.
The core445 48,477-row ledgers change only those two remaining engine keys:
candidate 1,609 expected / six skipped / seven unexpected / 46,855 not run
(SHA-256 `8f281beff4c1851717a237d903733082ffbd515455ccdb84ee336722f94ed821`),
oracle 1,312 expected / five skipped / six unexpected / 47,154 not run
(SHA-256 `3aa0fa8c99dfd9c62eacf67db0610b28adf869fcb66965ffe8c235072cbc850a`).
The full pinned invocation audits pass. A fourth Core browserless tranche
selects pinned Chromium `tests/library/chromium/launcher.spec.ts:19`:
`launchServer` rejects a `--remote-debugging-pipe` argument before browser
spawn. Candidate/oracle each pass 1/1, both source-bound sidecars verify,
and the manifest SHA-256 is
`aa9845826071c8794121536e0a58083b87ecb655ec0ff2a3bb49afab43be4d8c`.
The core446 48,477-row ledgers change only that key to `expected`:
candidate 1,610 expected / six skipped / seven unexpected / 46,854 not run
(SHA-256 `2e06cebde74d8643ba48ab31c8834c3282ccc0bf2c8fbb5548a62aff9fcca2c1`),
oracle 1,313 expected / five skipped / six unexpected / 47,153 not run
(SHA-256 `71fc385156b5612447037edeb69cef8a5db6d138ad202155fe44c848db17d7a9`).
The pinned invocation audits pass. Node package tests now pass 472/472 with
types green. All four new Core tranches are pre-spawn, not positive browser
or compatible BrowserServer proof.
An exact-source Bidi Library batch then selected four pre-spawn BrowserType
argument/error rows across six Bidi projects, 24 unchanged pinned official
invocations in total. Candidate and oracle each passed 24/24 without a
browser or CDP endpoint; both provenance sidecars verify. Its manifest
SHA-256 is
`4ae7f22502c2f3e65fc6f91b30c5d84280a24c62cb07a913efd34cae1efafbcb`.
The core447 48,477-row ledgers change exactly those 24 `not-run` keys to
`expected`: candidate 1,634 expected / six skipped / seven unexpected /
46,830 not run (SHA-256
`e9623f7b7e56e4e5330e971496d1ce632389398c7a703eefe1fff57e6ab360b5`),
oracle 1,337 expected / five skipped / six unexpected / 47,129 not run
(SHA-256 `4568ec15716616d37ce912c00a3c34a04ec5aa2f45f38be9eb54d8884c4184b0`).
The formal audit classifies all 24 as public-API candidates. Their success
proves pre-validation parity only, not Bidi browser launch compatibility.
A second Bidi browserless preflight batch selects 18 more pinned public-API
invocations: invalid `launchServer.channel` and proxy server values in six
projects, plus invalid Chromium launch arguments in three Bidi projects.
Candidate and oracle each passed 18/18; all ten Core schema-2 source-bound
provenance sidecars reverify, and no browser/CDP was used. Manifest SHA-256 is
`e645e86f85df3f61dda3481e153bc8662982bb06590d5c604e8d0d54ca5db2d9`.
The 48,477-row core448 ledgers change only these 18 `not-run` keys to
`expected`: candidate 1,652 expected / six skipped / seven unexpected /
46,812 not run (SHA-256
`099c97f1180d1c6e27d5d90b3d38ec8c162cce90030bf2558cc27a6061c371af`),
oracle 1,355 expected / five skipped / six unexpected / 47,111 not run
(SHA-256
`d2ad0a6c5f0aa882c4924d21f14307df0cf7fe05a551240c2b4663b86159af9b`).
This proves only pre-spawn argument parity, not a Bidi browser launch.
The next five pinned browserless Library invocations probe `browserType.name()`
for Firefox/WebKit and `browserType.executablePath()` for all three engines.
Candidate and oracle each recorded two expected results and three unexpected
results, with six verified schema-2 sidecars and no browser/CDP activity. The
manifest SHA-256 is
`ae93f4e445c4b1cce86e3c89e24dc714115b8718da3b91f8320867c503ee4bf3`.
The core449 48,477-row ledgers change exactly these five keys: candidate
1,654 expected / six skipped / ten unexpected / 46,807 not run (SHA-256
`890b27693fd6910a5379b7c79bf245ad08de9a222706922079eecb126b5aef80`);
oracle 1,357 expected / five skipped / nine unexpected / 47,106 not run
(SHA-256
`72cdf773d4ba9b2ee4b75e3277a78b8fcc66d2a20cdc4d7afdeff1ad248daf58`).
The three executable-path failures are **not** evidence that the candidate's
public `executablePath()` disagrees: pinned upstream line 24 first reads
private `_playwright._defaultLaunchOptions`, which is absent from Pwright, so
the candidate never reaches line 26. The oracle reaches the public method but
then fails the line-27 file-existence assertion because its v1.58.0 browser
cache is absent here. The fixed whole-corpus invocation audit classifies the
three Library invocations and six unrun Bidi variants as
`implementation-internal`/`private-client-access`, while the two `name()`
passes remain public-API evidence. Separately, source inspection finds that
Pwright's public JS `BrowserType` had no `executablePath()` method
or TypeScript declaration at core449. This was an API surface gap, but the unchanged
official test does not directly measure it because of its private precondition.
Candidate and oracle classification ledgers
each have 48,477 rows and SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`;
the correction reclassified 72 invocations without altering outcome ledgers.
The core450 browserless preflight tranche selected 12 more unchanged pinned
Library/Bidi invocations across eight project-fixed suites: Firefox/WebKit
profile and page-argument rejection plus three Bidi Chromium remote-pipe
rejections. Manifest SHA-256 is
`1e89c9547baaf719bfc288259f68ee6bbd8d082acf31858679435437b1ac9d49`.
Oracle passed 12/12; candidate passed three and failed nine because its
unsupported-`args` error preceded the upstream engine-specific rejection.
All 16 source-bound sidecars verify; all 12 invocations are public-API
candidates, with no browser/CDP use. The 48,477-row core450 ledgers change
exactly these 12 keys: candidate 1,657 expected / six skipped / 19 unexpected /
46,795 not run (SHA-256
`49c9c000cdc4e32eaf4ec0886deaa2a05bec0aa1c2123821af98bdaa11b502c0`);
oracle 1,369 expected / five skipped / nine unexpected / 47,094 not run
(SHA-256
`f6ee9a5b3c26c31bd8a0c6c6dd0d1440d9a33e56d8aa2a4562a809c395ebb5c1`).
After preserving that candidate source/addon under
`.work/official-conformance/pwright-node-owned-core450-before-preflight-fix.t1ghwh/`,
the Node façade now applies the pinned engine-specific preflight before the
unsupported provider boundary. Five new AAA adapter units pass. The exact
same 12 official candidate invocations then passed 12/12 in core451, matching
all 12 oracle `(suite,specId,projectId,outcome)` rows; eight new candidate
sidecars and eight unchanged oracle sidecars verify. The core451 candidate
48,477-row ledger has 1,666 expected / six skipped / ten unexpected /
46,795 not run (SHA-256
`a864e42b300b8a98dfbb65880bed7f1e9fce2cb1aaaab24acd5fc797a673b38e`).
It refreshes exactly 12 source-bound rows: nine unexpected become expected,
three already-passing rows receive current-source evidence. The corrected
whole-corpus audit still classifies all 12 as public-API candidates; its
classification ledger SHA-256 is
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
The current package/addon are archived at
`.work/official-conformance/pwright-node-owned-core451-launch-preflight.oPZl90/`
and the candidate sidecars verify against that archive. This repairs only
pre-spawn errors; positive Firefox/WebKit launch remains unsupported.
The five remaining core451 candidate `unexpected:public-api-candidate` rows
are all browser-backed and also `unexpected` in the strict oracle: one cached
image request wait times out in both lanes, three device orientation/motion
dispatch cases lack Chrome's legacy initializer methods in both lanes, and
one extra-header case passes in both lanes despite an upstream expected-fail
annotation (XPASS). They are not presently candidate-only discrepancies.
Their exact shared browser/fixture causes still need a policy-compliant
dedicated Chrome/CDP investigation; no expected-fail mark or outcome was
changed here.
An independently selected `browser-registry` Node profile now exposes
synchronous `BrowserType.executablePath()` through the existing Rust
browser-registry leaf, with a TypeScript string return. It does not install,
download, discover or launch a browser, and default/owned-launch profiles
reject the method before loading native code. Its release addon SHA-256 is
`5f8e08938a76769e783254d72447a991b83cf40ecd4ee48b9005150ea1dcfc32`.
Native metadata units pass 4/4, adapter units 13/13, and all 490 Node package
units pass in each of default, owned-launch and browser-registry profiles;
TypeScript, native-load and dependency-boundary checks pass. With an absolute
temporary registry path set before imports, a direct browserless comparison
matches pinned upstream `playwright-core@1.58.0` public return paths 3/3.
This is diagnostic evidence, **not** a core449 official promotion: its
unchanged test accesses private state before the public method and the oracle
browser cache remains absent. The Node addon still has its pre-existing host
dependency tree; this profile adds only the registry leaf, not a host-free
binary. At this initial checkpoint, the binding still read
`PLAYWRIGHT_BROWSERS_PATH` per method call, unlike upstream's import-time
registry snapshot. A later opt-in repair captures the direct/npm fallback,
`INIT_CWD`, cache roots, current directory and host-platform override at
module import, then sends that immutable input to Rust's path policy. The
new release addon SHA-256 is
`641dd2867c3ccb5a3961020bdd2dd3e738cee2a15da141c654b1261e2cb4dedb`.
Fresh-process public-method differential tests against pinned Core v1.58.0
pass 13/13, including environment and cwd changes after import; Rust leaf
tests pass 9/9 and each of the three Node profiles passes 511/511 package
units, with TypeScript, native-load and dependency boundaries green. An
archived two-argument addon is rejected by the new fail-closed profile check.
This is browserless diagnostic evidence, not a core449 official promotion;
host-platform parity was checked on this Mac, not older macOS or other hosts.
An additional source-derived, browserless persistent-Context differential
found that upstream protocol strings accept boxed JavaScript `String` values.
The Node preflight now unwraps boxed `channel` and `args` before its ordered
argument decisions. Its dedicated focused suite passes 9/9, including seven
public-method comparisons against pinned Core v1.58.0. These are synthetic
edge cases, not new official corpus rows. Client option ordering, protocol
type errors, TLS material parsing and proxy validation remain unproven.
The next exact-source, browserless Core tranche (`core452`) selected 18
pre-spawn persistent-Context validation invocations from nine Library/Bidi
projects: `defaultbrowsercontext-2.spec.ts:153` (page argument) and
`client-certificates.spec.ts:826` (two invalid certificate configurations
inside one test). Its manifest SHA-256 is
`084e6070d99aff63e6ea80b6372d9f7366ccb8427aa1de689fd3349f68a86f85`.
All 18 candidate and 18 oracle result sidecars verify, with no browser/CDP
launch. Oracle recorded 17 expected and the pinned Firefox Library skip;
candidate recorded 17 unexpected because it returned the unsupported
`launchPersistentContext` boundary before upstream's pre-spawn validation,
plus the same skip. The pre-fix candidate source/addon is archived at
`.work/official-conformance/pwright-node-owned-core452-before-preflight-fix.QcGtKL/`.
The new immutable 48,477-row strict candidate ledger has 1,666 expected /
seven skipped / 27 unexpected / 46,777 not run (SHA-256
`db08893e67ca7e7f49449d48c1a1d5130d34d087166d4609866fbc6d6c20af58`);
oracle has 1,386 expected / six skipped / nine unexpected / 47,076 not run
(SHA-256 `7b86ace6318713fbb73a96635bdb5559fa45361aecd0c38d9ad5af4ebaddfa4a`).
Each ledger changes exactly 18 previously `not-run` rows; full invocation
audits pass with the unchanged classification SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
The nine page-argument rows are ordinary `public-api-candidate`; the nine
certificate rows are formally `public-api-with-private-test-helper` because
their source file has an incidental private import not used by the selected
body or fixture. The red baseline is retained; a later candidate rerun must
have its own verified sidecars and ledger before claiming a repair.
After archiving that red candidate, a composable JS preflight leaf now checks
certificate material and engine launch arguments in the pinned order before
the still-unsupported valid persistent launch. The unchanged official fixture
preload inherits a non-enumerable module-scoped default-options setter, so
its `moz-firefox` channel reaches the same BiDi argument check without
changing test source or the earlier harness digest. Public adapter tests,
TypeScript and boundary checks pass; Node browserless package tests pass
507/507 under each default, owned-launch and browser-registry profile.
The same 18 source-fixed Core invocations were rerun candidate-only as
`core453`: 17 expected and the same one skipped, with zero unexpected; all
nine new source-bound sidecars verify, and every selected outcome matches the
unchanged core452 oracle. Current package source and owned addon are archived
byte-for-byte at
`.work/official-conformance/pwright-node-owned-core453-before-next-fix.UWHQui/`.
The new immutable 48,477-row strict candidate ledger has 1,683 expected /
seven skipped / ten unexpected / 46,777 not run, SHA-256
`14c3ffac26ae15897c93d81b35a9b7853add367448484de4a7bb96cac6207cb7`.
Exactly 17 red rows became expected; the one skip received refreshed evidence
and no other invocation changed. Full audit remains 48,477 rows at the same
classification SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
This demonstrates only validation parity, not positive persistent Context
launch, browser ownership or TLS parsing for malformed certificate material.
The next exact-source Core454 slice covers the two previously unrun
Firefox/WebKit `library/proxy.spec.ts:23` invalid-numeric-proxy invocations.
Both oracle and candidate report two expected outcomes, zero skips and zero
unexpected outcomes; the four source-bound schema-2 sidecars verify, including
the exact manifest
`tests/official/playwright-core-browserless-invalid-proxy-remaining-engines-manifest.json`
(SHA-256 `f4eb2595970997e1ccb166f229bfcc07c257fa406526c02c2616469a8dff96dd`)
and
the candidate's exact source/addon archive at
`.work/official-conformance/pwright-node-owned-core454-before-invalid-proxy.wOW0gt/`.
The immutable 48,477-row strict candidate ledger is now 1,685 expected /
seven skipped / ten unexpected / 46,775 not run, SHA-256
`b2362075a2f4f748388e4eabdd4a1fc9b799fc52babf816e9a2d3fa70a719292`;
the oracle is 1,388 expected / six skipped / nine unexpected / 47,074 not
run, SHA-256
`e4d41296a636a6d60000a0692e4f76ae39635af8967dd0f90ce71b0e1d2bf596`.
Exactly those two rows changed from `not-run` to `expected` in each ledger.
Both full audits retain 48,477 rows and classification SHA-256
`f5e63b043f85437a2cc3e6b5b5972a9db98ea0fb1b23f588aa84614f1eeae3d3`.
This pre-spawn validation result does not prove positive proxy or browser use.
A classification-only Core454 successor corrects one source-selection error:
`tests/library/browsercontext-viewport.spec.ts:50` directly iterates
`deviceDescriptors`, imported from the private
`packages/playwright-core/lib/server/deviceDescriptors` module at line 22.
Only that test body's nine Library/Bidi project invocations change from
`public-api-candidate` to `implementation-internal`; the other viewport
tests retain their prior classifications. Candidate and oracle v2 audits each
have 48,477 rows and SHA-256
`8084c4d5c1b75660cd709936e23aeb9f6ab812bd5556a1a38ed4ce13b2d2f365`.
The original audits remain historical snapshots. Both Core454 strict outcome
ledgers and their SHA-256 values above are unchanged; all nine corrected
invocations remain `not-run` with zero attempts. The corrected audit's
candidate cross-tab remains 1,685 expected / seven skipped / ten unexpected /
46,775 not run; the oracle remains 1,388 expected / six skipped / nine
unexpected / 47,074 not run. A bounded scan of unexecuted browserless Core
Library source-only utility/parser, APIRequest and launcher cases found no
additional safe public-API tranche: the source-only parser cases use private
implementation imports, and the public launcher/metadata pre-spawn cases are
already recorded.
A second classification-only Core454 audit follows direct use of private
imports within individual test bodies, without demoting whole files. The
`outofprocess.start` Selenium case at
`tests/library/browsertype-launch-selenium.spec.ts:181` contributes nine
`not-run` rows; `globToRegexPattern`/`urlMatches` at
`tests/page/interception.spec.ts:73` and `asLocator` at
`tests/page/page-aria-snapshot-ai.spec.ts:72` contribute 11 each. Those 31
rows change from `public-api-with-private-test-helper` to
`implementation-internal`; nearby tests keep their previous labels. Both
48,477-row v3 audits are byte-identical, SHA-256
`bfcb82ba51e95c9968daeb04d8c4a42f02eb9830ca64a265a02fb479b2f2be3a`.
Core454 outcome ledgers remain unchanged at candidate 1,685 expected / seven
skipped / ten unexpected / 46,775 not run (SHA-256
`b2362075a2f4f748388e4eabdd4a1fc9b799fc52babf816e9a2d3fa70a719292`)
and oracle 1,388 / six / nine / 47,074 (SHA-256
`e4d41296a636a6d60000a0692e4f76ae39635af8967dd0f90ce71b0e1d2bf596`).

The separate Core455 internal/source-only manifest
`tests/official/playwright-core-internal-source-only-manifest.json` (SHA-256
`401e5e519d0234a7c91fcfd6f063f4fe35149c5dc230fbf27e80550d0e4af337`)
selects exactly `browsercontext-viewport.spec.ts:50` and
`page/interception.spec.ts:73` across three Library and six Bidi projects
each, excluding Android and Electron. Both test bodies take zero fixture
arguments and exercise pinned upstream-private symbols. The distinct
`pinned-private-source-only-no-io` mode verifies source hashes, AST import and
body use, exact spec/project IDs, and starts the no-I/O preload before the
runner loads test or fixture modules. The preload blocks arbitrary child
spawn, network connect/listen and fetch in the runner, workers and loader
threads; only the pinned Playwright worker fork is allowed. Negative tests
confirm those blocks and that the parent test process is unaffected. No
browser, CDP connection, or fixture server is part of this mode.

Oracle and the frozen candidate package/addon at
`.work/official-conformance/pwright-node-owned-core455-internal-source-only.kPAe4a/candidate/`
each pass the four exact selections 3+6+3+6 = 18/18. Eight schema-3
sidecars verify the source, report and guarded-process receipts. The first
live-candidate reports were deliberately not promoted after the package
source changed and their sidecars stopped verifying; the frozen rerun is the
candidate evidence. The immutable Core454 predecessors remain intact. The
new 48,477-row strict candidate ledger has 1,703 expected / seven skipped /
ten unexpected / 46,757 not run, SHA-256
`5cc090bd05c928255cc9da69d9e535879c59af467c4ecb89445a827257cef14e`;
the oracle has 1,406 / six / nine / 47,056, SHA-256
`eb54dc153626706a97e54fad339f1c610e0d9f23232a7c614f74da59ef1bda6e`.
An all-row diff finds only those 18 previously `not-run` invocations changed
to `expected`, each marked `evidenceScope=upstream-private-source-only` and
`candidateRuntimeRequired=false`. Candidate and oracle complete audits retain
the v3 classification SHA-256 above and separately count 18
`expected:implementation-internal`. The strict outcome ledger deliberately
retains each row's original inventory `sourceClassification` and
`classificationReasons` (including `public-api-candidate`/empty reasons for
these two source files); the corrected symbol-use label belongs to the
separate invocation audit, not a rewrite of pinned inventory. Candidate
adapter loading appeared in
worker output, but the selected bodies do not require or exercise Pwright's
public API; the provenance deliberately sets
`candidateRuntimeAttested=false` and `publicApiParityEvidence=false`.
CI now includes a pnpm-only clean-checkout bootstrap and replay of all four
Core455 suites in both lanes, verifying each schema-3 sidecar and its
`publicApiParityEvidence=false` marker. The bootstrap derives pnpm locks
without modifying tracked upstream files, maps only pinned upstream build
subprocesses from npm/npx to pnpm, and runs no browser. Locally, the exact
Node 20 / pnpm 10 setup built a fresh pinned checkout and passed all eight
replayed runs (18/18 oracle, 18/18 candidate), with four AST guards, six
bootstrap unit tests, eight provenance verifications, and a clean tracked
upstream status. The prior source-only runner/verifier/summary/audit unit
tests (67/67) and TypeScript installation remain CI-gated. GitHub Actions
has not run this new gate yet; local replay is not public-API parity evidence.
A separate diagnostic targets pinned Core
`library/browsertype-launch.spec.ts:64`, whose explicit executable is a
short-lived Node script rather than Chrome. The ordinary Core runner rejects
this schema-2 manifest before any subprocess. The separate exact-source
wrapper uses a macOS process-exec allowlist, denies outbound networking,
preflights both restrictions, and requires public-launch and exact dummy-Node
exit receipts. It does not attest browser process/profile ownership or grant
strict-ledger promotion. The current manifest SHA-256 is
`e4d3a6881a57ddcbeab63cd34efd830bb51e61806e2c4b4f3e3faac3c4240645`.
Oracle passed 1/1 under this sandbox with one dummy exit-1 receipt (result
SHA-256 `589471f3d7b2711f6687ab129c695580e55f62096eee00e0b1d1c0ae68676af0`,
provenance SHA-256
`e7af5313095c4c829fc58d192f8bc02177d8693d2412ec41337f4d73dfcaedbb`).
Candidate ran the same official selection but failed before the dummy spawned:
after matching `handleSIGINT: false`, its inherited `tracesDir` remains
unsupported, so the expected `Browser logs:` substring was absent. The
candidate result SHA-256 is
`6d4759d0b9a7ed31a0dd170ccf83d7de5f79e423c4e507d95139305eae6b3e24`;
the runner correctly rejected missing process attestation and wrote no
provenance sidecar. Both strict-ledger keys remain `not-run` and this failure
earns no public-API parity credit. The dummy-specific Python guard and Python
fixture-blocker checks pass 17/17; Node package and type tests pass 529/529
after the narrow `handleSIGINT: false` change. No browser or CDP endpoint was
used by this diagnostic.
Separately, the Rust Chromium provider has an opt-in bounded stdout/stderr
collector, and Node host's owned-launch composition now selects it. Synthetic
shell-process release tests pass at provider (24/24), composition (16/16) and
focused host (1/1) boundaries; the default provider still discards output.
These are implementation tests, not official parity. The rebuilt owned Node
addon has SHA-256
`b37efd40f111e305cf976812c4048a6782ce76ab36418d4f13e6ac320f87788a`;
a browserless public Node-to-N-API-to-Rust synthetic-process smoke test passes
1/1, including bounded log output, profile cleanup and child reaping. CI now
builds this opt-in addon and runs the smoke test on Node 20; GitHub Actions has
not run the new step yet. At this `owned-launch`-only checkpoint, the official
candidate still rejected `tracesDir` before any process was spawned. No strict
ledger or provenance sidecar was promoted by the new log component.
Pinned Core source inspection narrows this discrepancy: the line-64 body only
launches an immediately exiting Node executable and checks `Browser logs:`;
it never calls a tracing method. Its worker fixture nevertheless injects
`tracesDir` even with tracing off, and upstream recursively creates that
caller-owned directory before allocating its temporary profile or spawning
the process. The new opt-in `pwright-trace-playwright-v8` Rust leaf has a
typed, non-deleting directory preparer plus a separately bounded v8 JSONL
context/API-event writer (11/11 release tests, strict Clippy); neither is a
public Context recorder yet. There is no official browserless public
`BrowserContext.tracing` case in the pinned Python/Core selections inspected:
even inactive stop/start-stop cases use browser-launching Context fixtures.
The two Core request-only trace cases call private `_tracing` and cannot earn
public-API parity credit. The adjacent third test also accesses the same
private field. The exact selected-profile rerun described next supersedes the
earlier failed diagnostic, but it still does not advance either strict ledger.

The separate `playwright-trace` Node profile now prepares a caller-supplied
nonempty trace directory in Rust before executable validation, temporary
profile allocation or child startup, and retains its canonical path without
deleting the caller directory on failed launch. An empty string is treated as
no explicit directory. Its browserless native tests pass 4/4, Node package
tests pass 533/533, and selected native-load, TypeScript and dependency-boundary
checks pass. The selected release addon SHA-256 is
`13d6db323b599d57608d99b7f18f8a06856c6425e1931dfdb718f74d356c765d`.
The new exact schema-2 [dummy-process manifest](../tests/official/playwright-core-dummy-node-launch-playwright-trace-manifest.json)
has SHA-256
`aa835db62c15030708580c5d4addee02bb7d2c0cbd9d7c90aeb22953c5493d13`.
Under the unchanged sandbox policy, its pinned Core line-64 oracle and selected
candidate each passed 1/1, with one guarded failed public launch and one
dummy Node exit-1 receipt. Their [oracle](../.work/official-conformance/outcomes/core-dummy-sandbox-oracle-trace-v2.provenance.json)
and [candidate](../.work/official-conformance/outcomes/core-dummy-sandbox-candidate-trace-v2.provenance.json)
diagnostic sidecars have SHA-256
`1a3cc11fd2fb8133a85716c83955754f2370ac475bd18e31a8037a952d851e6f`
and `33f85584b958b2af827113f6033cc086c46c482d0fe6d4829c0e4b2b09966385`.
The process guard does not attest browser profile ownership, and the test
does not exercise Context recording or network/ZIP output. Both strict Core
keys remain `not-run`; this is diagnostic evidence, not full tracing parity.

A fresh, separately verified [strict dummy-Node pair](../.work/official-conformance/outcomes/core-dummy-node-launch-strict-paired-v1/promotion-receipt.json)
supersedes that diagnostic for the exact Chromium Library line-64 row only.
The unchanged pinned upstream assertion passed 1/1 in both isolated official
oracle and Rust-backed candidate lanes with zero skips. The strict runner
captured the public `chromium.launch` invocation, live guard/CLI/child process
ancestry, one real dummy Node exit 1 and reap, temporary profile ownership
while live, and its removal after the failed launch. The candidate selected
`playwright-trace` addon SHA-256
`ab6be17625aadadbfce4bbc72866c72732191572c40b951821a68f947509cf3a`;
the paired manifest SHA-256 is
`6a6d866436c38b38a1b75c6a0e9a652cbfd7e0a4e9f3a8edb1746bef86db4b77`.
The independent verifier and 52 focused Python tests plus six Node preload
tests pass. The immutable successor changes only row 31,614 of each 48,477-row
ledger from `not-run` to `expected`, preserving all 48,476 other row bytes.
Candidate now records **4,544 expected / 53 unexpected / 11 skipped / 43,869
not run**; oracle records **4,291 expected / eight unexpected / ten skipped /
44,168 not run**. The test-mode module adapter and process guard are fixture
adaptations, explicitly marked `fixtureAdapted=true` and
`unadaptedOfficialPass=false`; this is scoped public launch behavior evidence,
not an unadapted whole-suite pass, Chrome execution, Context tracing parity,
or reproducible Rust-source-to-addon build proof. The older diagnostic pair
and predecessor ledgers remain intact.

The official invocation auditor now recognizes direct access to
`APIRequestContext._tracing` as implementation-internal. A same-auditor
baseline diff changes exactly 27 `not-run` invocations at the three request
tracing tests (lines 636, 662 and 699; nine projects each); the two initially
reviewed tests account for 18. Paired 48,477-row oracle/candidate audits
share SHA-256
`3e260fd6dca8c2c20a3b16c39c196c2bc34c191e1f1dbd59567f89056547a458`.
No outcome or public-API parity credit changes.
An additional browserless Python lifecycle repair now emits Context `close`
once when a fresh-owned Browser closes; async/sync and attached-CDP guard
units are green (Python package 308/308). The two pinned official Context
close-event cases are source-checked but unexecuted pending the browser-policy
decision. Earlier ABI107 captured package and harness trees are archived
before this source change so prior schema-4 evidence remains reconstructible.
The provenance verifier now has a candidate-only exact-package archive
override; both earlier ABI107 candidate sidecars reverify against the
archived package/harness/tooling (35/35 verifier units pass).
The Rust host now uses one-flight owned shutdown: concurrent `close()` calls
await one retained direct-child shutdown report, and canceling a waiter does
not cancel that shutdown. Two browserless release tests pass. The report is
not yet surfaced to conformance provenance; descendant reaping and profile
deletion remain unproven.
The host source now guards fresh-owned hidden default Context/Page/Frame IDs
at generic command admission and many direct routes, event-history, timeout,
binding, worker and download entrypoints; attach/persistent behavior is
preserved. The final Rust source passes an all-features release check and
76/76 focused browserless release tests across command admission, Worker,
Worker console/handles, storage-state and FrameTrees modules. Installed
default-profile Node native binary still predates these host edits; the opt-in
owned-launch addon and Python wheel are checked below.
Separately, a fresh ABI107 Python wheel from this source (SHA-256
`77b485d083121a08f0dea4fe1b9183c65ebda2b9a7e4ff3aa74f4d8726718c3a`)
loaded from an isolated Python 3.12 venv outside the source tree and passed
308/308 browserless Python package units. The wheel is archived under
`.work/official-conformance/builds/python-abi107-host-guard-20261010/`.
The older pre-close-event archive has an earlier source tree despite
containing the same old native `.so`. Before replacing the editable extension,
the exact ABI109 `python/pwright` tree, shim/runtime and four provenance-bound
tooling files were copied to
`.work/official-conformance/python-abi109-exact-archive.yO61kG/`.
All seven prior candidate sidecars bound to that ABI109 package digest verify
against the archive with
`--package-root`, `--harness-root` and `--tooling-root`. The wheel's ABI107
extension was then installed into the editable source tree with its two
bundled OpenSSL dylibs. Editable import resolves to that source extension,
reports ABI107, and its browserless Python units pass 308/308. The new
extension SHA-256 is
`e3ebe18e4b1577ee18de03a72b5d9b72f320d46986e3f5e92a38316f6f28c465`;
the bundled crypto/SSL dylib hashes are
`bfa24a48f50688de796e559941e03882248d3b26bc3be1f78e3b9b560f6badbd` /
`a5486ffa809746d423d1f47bc6319664aba9560805fe1c923b6be4d6d3c22069`.
The existing Python strict ledgers do not change. The new Python official
capture/verification schema 5 records every adjacent
`shuozeli_pwright.dylibs` member as an absolute path and SHA-256 in
`pwrightRuntimeLibraries` (oracle records an empty list), verifies exact
membership and bytes, and requires that sibling directory beside an archived
`python/pwright` passed to `--package-root`. The 57 focused provenance and
verifier units pass. The same pinned browserless Python
`test_should_work_with_glob` then passed candidate-owned 1/1 for each of
Chromium, Firefox and WebKit under ABI107. All three schema-5 sidecars verify
against both the live package and the exact archive at
`.work/official-conformance/python-abi107-schema5-glob-exact-archive.YE9OWt/`,
including its sibling dylibs, with no browser/CDP launch. The new immutable
6,526-row candidate ledger refreshes only those three already-passing keys:
1,258 passed / ten skipped / four failed / 5,254 not run, SHA-256
`778425cb062d329ef6ee0b3b24e62ce0f97696819abfe09944272dbaa1178639`.
The full audit remains 6,526 rows, SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`;
these three tests are `implementation-internal`/`non-applicable` due to their
private import, not public-API parity evidence. Historical schema-1 through
schema-4 sidecars are unchanged and do not retroactively claim
sibling-library binding.
The pinned Python persistent-Context page-argument manifest then exposed a
WebKit pre-spawn mismatch: the oracle passed, but candidate ABI107 failed
before checking `args`. A narrow facade ordering repair preserves unsupported
valid WebKit launch and passes the same unchanged WebKit official case 1/1;
Firefox remains an official skip in both lanes. Four schema-5 sidecars verify,
and the final candidate package/tooling snapshot is archived at
`.work/official-conformance/python-abi107-webkit-preflight-exact-archive.rEWC4a/`.
Python package units pass 312/312 with Ruff/Pyright green. The successor
6,526-row strict ledgers change only those two keys: candidate 1,259 passed /
11 skipped / four failed / 5,252 not run (SHA-256
`3bae07314d8d9b717a503e1b553d813faa5f11d01deec922cdf9d8d33ddf0c81`),
oracle 816 passed / ten skipped / four failed / 5,696 not run (SHA-256
`c622c0a3c4aa8ab9a0afdfbfa1aa89531b683ae50fc4e63f7fbeac38a54852b6`).
Both full applicability audits pass at unchanged SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The old manifest did not declare a harness-level browserless policy; pinned
source/fixture flow and process observation establish that this selection
rejected before browser spawn. A new manifest (SHA-256
`3e59ca6e06feea4101e87840c5bfe1f73e72f2d326fa9a8d738198245e66b869`)
now encodes the exact AST/fixture/package pre- and post-run guard, with no CDP,
channel, secure fixture or inherited injection options. The same WebKit pass
and Firefox official skip reproduce in both lanes; four new schema-5 sidecars
verify live and against
`.work/official-conformance/python-abi107-webkit-browserless-exact-archive.apuY9p/`.
Only those two evidence references change in the new immutable 6,526-row
ledgers; candidate remains 1,259 passed / 11 skipped / four failed / 5,252
not run (SHA-256 `24a36b73150a04012f1ca45911601758ef9ae61bad186b2b28af374995242888`),
oracle remains 816 passed / ten skipped / four failed / 5,696 not run
(SHA-256 `ae78115993a256ca90af4c6da4389b924422ce37d6149eba965c034e27cb39c3`).
Both full applicability audits retain SHA-256
`31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The oracle still starts a Node driver and may create temporary directories;
the claim is no browser process or CDP, not no subprocess or side effect.
Focused guard/provenance tests pass 81/81; two older profile tests fail with
this environment's pytest 8.4.2 because their fixture pins 8.4.1.
Async event publication and full entrypoint exhaustion also remain open.
A read-only follow-up found two conditional gaps in the earlier source:
Worker notices could originate from a hidden Page, and the storage-state
scratch Page/Frame could pass generic admission inside a visible Context.
The current Rust source now requires Page publication before fresh-owned
Worker attachment and Worker/console notice delivery, and before public
Page/Frame operations. A private exact-ID task-local permit keeps scratch
navigation, evaluation, route setup and cleanup functional. Storage-state
collection falls back to scratch for indexed but unpublished Pages; attached
and persistent CDP behavior is unchanged by these checks. The focused
browserless Rust release regressions pass. At that checkpoint installed
Node/Python binaries were not rebuilt. Page retirement between a publication check
and command admission is not yet runtime-proven, and Page/Context publication
timing still needs real-browser verification. No live exploit was induced.
The bounded direct-entrypoint audit found no additional high-confidence
Page/Frame operation bypass. Rust-only `HostSession::page_snapshot` can read
a non-internal Page from live topology before collection publication, whereas
the Node FFI uses the published projection; this timing window is untested.
The hidden default Page may also consume one of the current 16 total Page
slots; the public quota contract has not been tested.
After preserving the pre-guard Node `owned-launch` addon and exact package
source under `.work/official-conformance/pwright-node-owned-core449.dJYI61/`,
all nine latest core447–449 candidate sidecars reverified against that archive
both before and after rebuild. The old addon SHA-256 is
`9231bb5f4e0407c7984dcdbc77b8fb08d979f76c5e6df31b5b15e68ee22d4b43`;
the rebuilt addon SHA-256 is
`bfb401ad7a097cebf63f7d453732b18d401d3db41407e6482973f1a5044bf4ae`.
Its browserless native-load smoke passes 1/1, owned-launch adapter 11/11,
Node package suite 477/477 in both default and owned-launch profiles, and
strict TypeScript and dependency-boundary checks pass. The package test had
one profile-dependent contract assertion; it now checks the selected profile's
distinct rejection without a skip. This is an artifact/ABI check, not an
official browser-backed conformance run. Three older owned-launch sidecars
predate the archived source digest and still require their original source
snapshots for current re-verification.

Earlier launcher design audit (2026-10-10): the Rust Chromium provider and launch
composition already have an opt-in process lease, and the host exposes
*persistent* Chromium launch, but at that checkpoint the public Node addon/API did not expose
valid `BrowserType.launch` or `launchServer`. Upstream `launch` returns a
Browser without a default Context; persistent launch returns a Context;
`launchServer` additionally owns a Playwright WebSocket endpoint and browser
process. The current Pwright server wire is a different protocol, and a CDP
URL is not a compatible `BrowserServer.wsEndpoint()`. Successful provider
leases disable kill-on-drop until a host re-arms it, so any new owner must
take that authority immediately and cover startup cancellation, listener
failure, disconnect and process exit. Browserless validation parity below
does not prove any of these positive launch paths. The proposed feature and
conformance-lane sequence is recorded in [design tasks](design-tasks-v2.md).
The unchanged upstream `launch server > should work with host` passes
`host: '0.0.0.0'` and checks that literal endpoint. Its three default-engine
Library invocations conflict with the local Tailscale-only binding policy;
the six non-default BiDi invocations have an upstream runtime skip but no
recorded outcome yet. All nine remain `not-run`, not silently excluded.

Earlier owned-launch preparation, before the ABI107 promotion (2026-10-10): the optional
Chromium launch composition now immediately guards its successful process
lease before connection binding, with fake-process takeover/cancellation
tests. The Rust host has a fresh launch entrypoint that withholds Chromium's
native default Context from ordinary published collections. It is not yet a
complete encapsulation boundary: a caller that guesses native IDs can reach
some generic and direct host methods. The Python official harness now has a
distinct `owned` mode that disables both CDP launch adapters and source-binds
new reports with provenance schema 4 plus JUnit launch-mode labels. At that
checkpoint it failed closed before pytest while the candidate public launch
API was absent. All 74 focused browserless runner/provenance tests passed,
without a new official result or strict-ledger row then. The six earlier Python
ABI106 metadata sidecars still verify against archived pre-change tooling
and shim/runtime, including the candidate shim package itself.

Browserless BrowserType matrix checkpoint (2026-10-10): the pinned
`tests/official/playwright-core-browserless-browsertype-matrix-manifest.json`
(SHA-256 `de14b5a54017e9886da68ac69d5ad6575611a0463d1babf648a92d7f49e24486`)
selects exactly 15 previously unrun public-API invocations across Library and
BiDi projects. The source-bound independent oracle records 11 expected and
four upstream skips. The preserved candidate v1 recorded three expected,
eight unexpected and four skips: Firefox/WebKit BrowserType exports were
undefined. Thin CJS/ESM/types metadata exports and an early non-Chromium CDP
error repair those eight cases without launching a browser, adding a Rust
dependency, or claiming Firefox/WebKit automation support. Fresh candidate
and oracle v3 reports each record 11 expected/four skipped, with no
unexpected outcome; all four provenance sidecars verify under schema 2 and
bind the current capture-tool sources. The repaired strict 48,477-row ledgers
are candidate 1,591 expected / six skipped / eight unexpected / 46,872 not run
(SHA-256 `9f3e4e2efea5126d04acc6508e4ab4bf9f58497582e4010a6ff2fcfcba5e68d6`)
and oracle 1,295 expected / five skipped / six unexpected / 47,171 not run
(SHA-256 `f82dc473dffc1dbb5caf981742a3063853b3a1c5cc65199736ba4608800edc0f`).
The eight repaired candidate keys and no others changed outcome from the red
15-key merge. A later exact-row BrowserType `launchServer({channel: null})`
preflight capture also matches the pinned oracle without launching a browser:
both result reports classify the unchanged validation case as expected 1/1.
The thin Node facade reproduces only the upstream early validation error;
valid `launchServer` requests still return `PWRIGHT_UNSUPPORTED`, and no
BrowserServer or native launch capability is claimed. Node units pass 450/450,
type and dependency-boundary checks pass, and both schema-2 report sidecars
independently verify. The exact one-row strict merge then had candidate 1,592
expected / six skipped / seven unexpected / 46,872 not run (SHA-256
`49b053267f0a9f1e4c10702457ddc35c34fbc4aeef43db7810b71b26be962c33`);
oracle 1,295 expected / five skipped / six unexpected / 47,171 not run
(SHA-256 `138c163a8779a18d9fd0436c94163fbf84d50eb7d112128b6d4d2e3b8d02194c`).
This accumulated ledger combines evidence from successive candidate source
snapshots; final full-corpus verification must rerun against one frozen build.
As a scoped regression check, the exact same 15-key BrowserType matrix was
rerun after the `launchServer` patch under the same candidate-source SHA-256
`d6a1a8ed599c5f6993762e1f43112c50dba83f31a4e3923b926876d187c3c25e`
as that preflight report. Candidate and oracle again agree on 11 expected /
four upstream skips; all four fresh schema-2 sidecars verify. Replacing only
those 15 result references created the core440 strict ledgers, candidate
SHA-256 `77506afa5d1e55dcf762d41e1f7e09a2a8aaef8d1884ee26ec50b757d10c9519`
and oracle `c12b227cbc066e31ce324d9eef3aad825b171b6e02622d4d6fe39dd956ef1940`.
Outcome counts and all non-result-path row fields are unchanged from core439;
the classification audit remains SHA-256
`927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`.

The next exact-source browserless preflight manifest (SHA-256
`e197f6a0aad744b0f65cb04120d07e3f758cd58dec832f6b560602ac3d2c4a81`)
selects four BrowserType invalid-option cases in each of the three Library
engine projects. Candidate and oracle pass 12/12 with six verified schema-2
sidecars; 11 keys were previously `not-run`, while Chromium `launchServer`
was already green. Only early Node argument validation was added; valid
`launch`, `launchPersistentContext`, `launchServer` and `connect` remain
explicitly unsupported, with no browser or process start. Node units pass
458/458, types and dependency boundaries pass. A further rerun of the same
15-key matrix under the final candidate-source SHA-256
`69f3083cfbffe7b51ba8ccc5228db0aabf00bc1769e82bb8e7717b909f9660cf`
again matches the oracle at 11 expected/four skips, with four fresh verified
sidecars. The latest core442 strict ledgers count candidate 1,603 expected /
six skipped / seven unexpected / 46,861 not run (SHA-256
`f9d400c3aa5804fdb5375df3c4900fee0a83dec711e0ca76df126d7ed5cd380d`)
and oracle 1,306 expected / five skipped / six unexpected / 47,160 not run
(SHA-256 `449f2b33e4587794d85c2de89ea38281ac04c9ba56b5d3700f0486588d6ab7ea`).
Only the 15 matrix result-file references changed between core441 and
core442; all other row fields and outcomes are identical. This still is an
accumulated ledger, not a full-corpus single-build pass.
Two earlier Page click/navigation invocations have candidate-observed
unexpected outcomes but remain `not-run` in the strict oracle ledger; they
depend on upstream-private test-hook options, so a candidate-only public
behavior discrepancy has not been established. The other five candidate
unexpected outcomes are shared with this oracle, while the oracle has one
separate unexpected outcome.
The exact hook-dependent IDs are
`04fee55e65ef85c07406-149f69b4f8ca6bdb917e`
(`__testHookAfterStable`, click during navigation) and
`cfaf4d74fb499322b955-ba064d75cc610f34650d`
(`__testHookBeforePointerAction`, no click after timeout). Both old candidate
reports lack provenance sidecars. Pwright rejects the private options before
the Rust action; silently ignoring them would not test the asserted timing
semantics. A fresh isolated oracle comparison and explicit per-row
applicability decision are still required.

The Core invocation audit still retains all 48,477 keys. Its 6,148
product-infrastructure rows now each carry an explicit source/role reason
(audit SHA-256 `927498c7f312c32dfcaff5ee8b58b597f5af8f879f1a5cd5bcbb19341fdb4412`);
this is classification evidence, **not** a blanket non-applicability decision.
The 3,458 implementation-internal rows and 38,871 rows in the three public-API
audit classes remain separately visible; some product/stress rows also exercise
public APIs and still need per-row applicability review. Schema-1 historical
sidecars are labeled
`legacy-unbound` with respect to local capture-tool source hashes; they must
not be described as schema-2 source-bound evidence. Browser-cache path,
installed browser revisions and process execution remain separate provenance
gaps for the existing reports. Future exact Python `executable_path` captures
can use schema 3 to bind the current resolver environment, pinned registry
revision and path existence; historical schema-2 reports do not gain that
binding retroactively.
In particular, the 3,333 stress-tagged rows include 3,303 direct public
lifecycle/Page/Locator/launch obligations; treating all product rows as
non-applicable would discard required coverage. Formal product-scope decisions
for Playwright Test, MCP, installer, Electron, Android and examples remain open.

Python ABI106 now has an opt-in Rust registry for pinned BrowserType executable
path metadata. In the unchanged official metadata diagnostics, candidate and
oracle return identical Chromium/Firefox/WebKit path strings, but all six
path-existence assertions fail in both lanes because those exact browser
revisions are absent. The failures remain diagnostic, not strict-ledger
promotions; the latest strict Python ledger is still ABI105 at 1,246 passed /
eight skipped / four failed / 5,268 not run. The ABI106 wheel passes 297/297
package units; [the Python checkpoint](python-official-conformance.md#cross-engine-browsertype-metadata-checkpoint-2026-10-10)
records the source-bound reports, wheel hashes and `=0` cache-root limit.

Dispatch-event selected-file checkpoint (2026-10-10): The pinned Core
v1.58.0 `page-dispatchevent.spec.ts` source SHA-256 is
`0af6f0562a67f391939881030426b6c0c4c0b4d77ba7d88bd64160b75231e36e`;
`tests/official/playwright-core-page-dispatch-event-manifest.json` selects its
complete 17-case Chromium Page file (manifest SHA-256
`4bb9714f113992367330d0578f2862b0ced17d766c69310faaca655310b06137`).
The Python v1.58.0 async file has 12 exact Chromium cases, pinned in
`packages/pwright-python/tests/official/page-dispatch-event-manifest.json`
(manifest SHA-256
`acbe71f2b9922e5040b17c2bcbcb3946b9096964f314de0f63ddc909353d0128`,
source SHA-256
`a4905396220267094de57d7fc721c8d733d4f0e453f37295882338c9ebe31c19`).
There is no `tests/sync/test_dispatch_event.py` at that pinned Python tag;
the separate sync Locator dispatch case belongs to `test_locators.py`.

Node Page/ElementHandle dispatch facades enter Rust-owned Locator and retained
Handle paths. Rust adds event constructor mappings and an owned live-Handle
initializer route. The opt-in `official-conformance-page-dispatch-event`
artifact SHA-256 is
`ff8e5db65ab83dca3e7f53fb22b493c81167ece9f7afcdb40e1424d2327ef34d`.
Node 442/442 units, TypeScript, dependency boundaries, native smoke 1/1 and
strict all-feature release Clippy pass. A scoped 2/2 real-Chrome DataTransfer
smoke is not an official test result; a later registered-selector probe
reported 17 active leases and 8,844 retained event bytes at explicit close,
so cross-probe cleanup remains unresolved. A scoped `deviceorientation`
failure on `about:blank` was identical in candidate and installed Playwright
1.58.2 and does not classify the pinned v1.58.0 official case. Python dispatch
ABI105 wheels v2/v3 hash to
`229a2fd6c2bc024c3b06943d53538217d0c70168e0e01675b52f25302fd13165` /
`3892b19253d7ba4b050c14c437e26c2508eb8d9294d9e1a6a7debb3d3a9a0d4d`;
normalized payloads reproduce except generated CycloneDX fields and the
dependent SBOM hash in `RECORD`. A fresh v3 install exposes both owned
dispatch methods and passes ABI105/287 units, Ruff, strict Pyright and release
addon check.
The pinned Core oracle v1 and candidate v2 each classify the selected 17 as
14 expected / three unexpected; the three shared failures are sensor-event
cases on the HTTP fixture. Oracle report/provenance SHA-256 is
`e5cc8d623a30203ebf336790c98fe3d19ca241de4d4196a6c21959c612fc627f` /
`c1b0486ccefc1a170d252678a1ee9306bb3495f015d4666807aede7253e7e95d`;
candidate v2 is
`c62ed0843bc3e0cbe225bd2277d550eac6bb7d706959605af783023d2414e1b9` /
`88ce1497a6ec6d6d53c743778b0d2baf0e02bafc805c188641ec3dff8afda478`.
After the HTTP v2 dispatch merge, the strict 48,477-row candidate ledger was
1,579 expected / two skipped / seven unexpected / 46,889 not run (SHA-256
`c75186cd78e28259a48247e553b566a514944f9470bc103e98d57b8de6411f4b`);
the oracle ledger was 1,282 expected / one skipped / six unexpected /
47,188 not run (SHA-256
`65ec860c18b4b5e3e82e96f87167eb0fd36f170ca7f0c1e1e2f51b6886298f39`).
Candidate v2 remains a verified historical checkpoint. After the JS
request-cache retirement patch, candidate v3 again classified the same 17 as
14 expected / three unexpected, identical per spec to the pinned HTTP oracle.
Candidate v3 report/provenance SHA-256 is
`693db5b080912f67831f382c472bb17e7387d340ab33489307c5e23273b69522` /
`aca608efc75caa1497e07b6635f7b95f6aec968a6f06f82d3bf942d071df3b03`;
its strict 48,477-row candidate ledger SHA-256 is
`cc21a501b7a12e575c870ff1f74eb0a3e3db89b9b8b0a0aa438bf129bc2f4560`.
The v3 provenance verified at capture; the runner later changed for a secure
fixture lane, so current-harness provenance re-verification is not claimed.
Both Core and Python fresh-run entry points now reject existing result and
provenance-sidecar destinations before process launch; Python's explicit
isolated-resume mode is a separate exception. The secure Core result validator
requires the exact set of 17 unique pinned spec IDs instead of accepting an
arbitrary 17 tests. Historical sidecars do not hash runner/helper source, so
these new false-green guards do not retroactively prove a fresh historical run.

A separate browserless `chromium-library` manifest pins two unchanged
Playwright v1.58.0 BrowserType rows: `browserType.name should work` and
`launch server > should validate options` (manifest SHA-256
`6845bcc03f0e3e6e7753b07edce877338c6c3f56c7de95dee3e5f400e1862594`).
The first is expected in both installed candidate and independent oracle;
the second is expected only in the oracle, exposing the candidate's absent
`launchServer` API as an unexpected outcome. The exact two-key merge leaves
48,477 rows in each strict ledger: candidate 1,580 expected / two skipped /
eight unexpected / 46,887 not run (SHA-256
`556eafa00527dbf41cd7d494dcb2f2a9423272581af614ab6ea1a7f09f93faa7`);
oracle 1,284 expected / one skipped / six unexpected / 47,186 not run
(SHA-256 `1b08b44e3351da8442652333694aef39a0ac7ab07ff866dea7d3e5b5f10fdb3c`).
No browser or server was started for this slice.

The pinned Python async oracle and ABI105-wheel candidate each pass 12/12.
Oracle XML/provenance SHA-256 is
`45356b260b29ee850e5d32b38d0db94c07aea8332ddb27bc99c94490a33bcb1f` /
`3e8cfaa4e381ea3b8d5febeab9b5d65852ad123ea2a839af1c85c9ca25c469fd`;
candidate is
`71c1be307ec418904fadb30bb93edff99200ea86cb144936aaed5a35da233b36` /
`b742d93ac50ab46c1c93a0190719f6563c737aff2cfeeadff7ea86efbe8ac83f`.
After the async dispatch merge, the strict 6,526-row candidate ledger was
1,238 passed / eight skipped / four failed / 5,276 not run (SHA-256
`c5a31bf3be6d848b804a910a1de6409b09ded8fc8c361db8b1a12ad60c9039de`);
oracle was 792 passed / seven skipped / four failed / 5,723 not run
(SHA-256 `bcc3d23b593cc99fc1d753a6350e89f16b06dea3047afd495ee55fedffafcdef`).

Core fixture adaptation currently makes `PREFIX` and
`CROSS_PROCESS_PREFIX` the *same* HTTP MagicDNS origin. Thus a passing
cross-origin-titled Core case does not validate distinct-origin navigation.
The HTTP fixture is also not a secure context for sensor-event constructors;
the three shared failures do not prove secure-context parity. A revised
trusted-Tailscale-TLS dual-port fixture lane is staged with secure manifest
SHA-256 `efc2f9f131c245baf7ddca262cd08ca3b5cee3d9f8d3d7a4248cbe55a5a03de2`
and preload SHA-256
`f3243c9f73c33a07a8eb62428505bd612f6248384f02ded14ffe2d42d219b8c2`.
Browser-free checks pass, but **no secure official browser run exists** while
shared Chrome belongs to another task. A preceding trailing-dot TLS preflight
failed before tests. Different fixture ports establish distinct origins, not
necessarily distinct sites or browser processes. Selected-file outcome parity
is narrower than semantics parity, and full Core/Python corpus parity remains
open.

An independent formal browserless Python sync-lifecycle mode pins the six
expanded rows of the two unchanged `tests/sync/test_sync.py` functions with
manifest SHA-256 `dbad4ede06a7e3ae07d994a75e02a8868bc82191145e6c3746a880eb2fa2081f`
and same-thread asyncio wrapper SHA-256
`7a089d2a16cbeec46926374571e96cf72cd8d3a513c76b0281c988be470e63c7`.
Candidate ABI105-v3 wheel and oracle each pass 6/6; candidate
JUnit/provenance SHA-256 is
`6b08f1df918824cc60e95d9231ba4c6c84f2b0eb5bd81639e9890e88d4c85880` /
`085841dbbf0993c31a4b4c9942770e243115d03ecbe975b6c3b6cc46503c2e1d`,
and oracle is
`c1f3a1c464d78c225cca5cd10568017d1b6c0a27d9d10cfc7dd0810fbfddb2cd` /
`0c191f41c487998880d90608060c5fd3c8fc68f5e84612dab45e6cd956a01326`.
The older ordinary-pytest candidate/oracle 3-pass/3-fail baseline is retained.
The exact six-key formal-loop merge advances the 6,526-row candidate ledger
to 1,240 passed / eight skipped / four failed / 5,274 not run (SHA-256
`e7cd534dbd40309ed7d258431face26a1b40d0f2c14c27834b493ec3fff5695e`)
and oracle to 796 passed / seven skipped / four failed / 5,719 not run
(SHA-256 `07cf927aa0a241b2075d9ec0f26c88e58daff0bf50e3d6cd646588ba056fc1d8`).
The next official Python browserless tranche pins the unchanged async and
sync `test_launcher.py::test_browser_type_name_should_work` rows (manifest
SHA-256 `e34e466352e5bb94304ba5dfa21655255877f6330b5ba236313c99dc97e0dac8`).
Installed ABI105-v3 candidate and independent oracle each pass 2/2. Candidate
XML/provenance SHA-256 is
`de0479b59b4650342c82006b13406470241390cc25ebc34cab6665e0d1d142f2` /
`0703d2ff4b7ec441f1eec22553a10ac1de1dd5e43cd8fd8edf0a92bd8159a9e2`;
oracle is `9ed208f5c431d5b689e11eecdf7516695e961db965d1ae1545920ee4a6047611` /
`3bb3b3a9cfd6cd02f4c91b7f6a754e6ba94682c98777511dbdfde79df37df21d`.
Both sidecars verify. The source checker permits fixture arguments only for
this exact two-case, source/fixture-hash-pinned closure; it still denies all
other fixture-argument browserless cases. The tranche used Tailscale-bound
HTTP/HTTPS fixture servers, no Chrome/CDP. Exactly two keys change in the
6,526-row strict ledgers: candidate 1,242 passed / eight skipped / four
failed / 5,272 not run (SHA-256
`ed31f1074ab675e75b8828e27a141a858144e8f85e408b9955d36fd1f2c6e4e8`);
oracle 798 passed / seven skipped / four failed / 5,717 not run (SHA-256
`2c02dffe0d102fe0d2dd660b30318097e455018c074140f73db5707067511852`).

Python Page check/uncheck pins all 18 unchanged v1.58.0 Chromium async/sync
cases in `packages/pwright-python/tests/official/page-check-manifest.json`
(SHA-256
`7e9b34f4ed0b0c193e213375ad2fc8dad6fbd13b622cda3f77c5d2ecf51809ae`).
Candidate v1 failed all 18 at missing Page methods; the independent oracle
passed all 18. Their provenance-verified report SHA-256 values are
`e7bd53be1ebb5598ad2c32b81b48815ed45938c109d457436f8b76fcae46774c`
and
`ed02b29dc95adc83c2f08d3584fa8b4a041dbfd2b0385061bca8504e10ece72e`;
sidecars hash to
`c7f5a90298b02b16bed002db9765289a53a41f7d0542d686ec3cdbaf37b4028d`
and
`f12a1290f16071659a873fabe7937162e41d26c386802569c5be3d593f74c0ac`.
The red candidate 6,526-row ledger was 1,208 passed / eight skipped / 22
failed / 5,288 not run, SHA-256
`7181ca1d518c24c69f9db6a0c9a85d85fd6384feca748139b9287216b9790e70`.
The oracle was rebased to the profiled 6,526-ID inventory by adding exactly
25 pristine example IDs (rebase SHA-256
`a937e7f5400ed2a0a565673893f2c132aaf7f9be29ff0916078a6d1740bdb045`);
the 18-pass oracle merge is 780 passed / seven skipped / four failed / 5,735
not run, SHA-256
`1bc6a8b0dbaa1a79471e60d4c7576e0512043c737e2f70dea80db56fe13cea47`.

The thin Python async/sync Page facades now delegate to the existing Locator
checked-state owner. Repaired wheel SHA-256
`85913b81bcc9d9c65533554b4b6f7f9ca423216ea08f43d682bc1244b2d5d46b`
passes 271/271 installed units, Ruff and strict Pyright. Candidate v2 and
oracle both pass the selected 18/18; candidate v2 report SHA-256 is
`2bc0177b7a6cc8f1ca7c1d4a6e20acfd728dbf819224ee88e69a7f07ad633e14`
with verified sidecar
`8c374ce922b748f613d82980ba3aa6be3eb68cdcde76071d19af7e36c8a44573`.
The strict candidate merge reaches 1,226 passed / eight skipped / four failed /
5,288 not run, SHA-256
`82c8161800ec3acd18938d0f9d0ba80edf76ce66b0918d563a6923b93ef6ee74`.
This completes the Python selected-file comparison; full Python/Core
execution remains open.

At the 2026-10-10T05:01Z source snapshot, independent final-source wheel
builds hash to
`11d0a7f348145e8c4e2150a78dc3afc5638571cc271e938558f62093090a4adf`
and
`0086c99813ee9f8dad7804c32235f272473d5de3d483fa24f36a2b681a793474`.
Normalized payloads reproduce except CycloneDX `serialNumber`/timestamp and
the consequent SBOM hash in `RECORD`; a fresh installation passes ABI104 and
271/271 units. The paired official 18/18 comparison used the earlier
`85913b81...` wheel. These 05:01Z builds have not run official browser cases
because a foreign nonblank tab occupied shared CDP Chrome.

Core Page check/uncheck pins all 17 unchanged `chromium-page` cases in
`tests/official/playwright-core-page-check-manifest.json` (SHA-256
`41787c922fd4690cc96cbf778e918132af57c778321afd7c470acec2ff8432b5`).
The preserved candidate v1 baseline has 17 unexpected failures: 16 at
missing Page methods and one at missing `ElementHandle.boundingBox()`. Its
report SHA-256 is
`8ea21fc35c11196cabf5ee8a51ae2faac34ac97ce7ede4def10cc82a43bd31c1`,
with verified provenance sidecar
`854f70c28119023ab35abef4ab165a0351cb1ffd63896613e5b136a71533955c`.
The strict 48,477-row candidate baseline ledger was 1,548 expected / two skipped / 21
unexpected / 46,906 not run, SHA-256
`c9990d41d41b1f622d5ce99e8dbb0796e30d6b0c8ba6cd2678e463fb1e0524d1`.
The independent oracle later ran all 17 cases successfully. Its report and
verified provenance sidecar SHA-256 values are
`16b0b069164778d38e91b8979a0b7104aa1f8f2736f11abf32c1f69209689547`
and
`f8dde0aa3d09f2132e794f8e86e5f1e00b40c201777409afe29f1f97107701d3`.
Its strict 48,477-row ledger is 1,268 expected / one skipped / three
unexpected / 47,205 not run, SHA-256
`d4a2878876e885d77297e47fe3f6bdaa91e72a4bc735448305c0cb205f8ea0a2`.

Thin JS Page methods now delegate to the Rust-owned Locator checked-state
action, and `ElementHandle.boundingBox()` has a native bridge. Node 423/423,
TypeScript and dependency-boundary checks pass. The new
`official-conformance-page-check` native artifact hashes to
`a991b225d0a3dece24d0a457b05898e6aacb8f6217eb9d79f5d710c33ea30531`.
Its candidate v2 report passes all 17 selected cases, matching the oracle;
report and verified sidecar SHA-256 values are
`58a5de733c8d052f64919a5ad6a738672ca059f72e787ee0a54a14b321d19375`
and
`72895a0bdd0432f5d2f80877a98a98f47550d0caf0fbd56b06344f5b617d6a41`.
Strict replacement of the 17 baseline keys yields 1,565 expected / two
skipped / four unexpected / 46,906 not run across 48,477 rows, SHA-256
`f3176185ef59fc45cc49228c3bec688dcea6c935ab2451f21e138962156cdad8`.
The selected file has candidate/oracle outcome parity. The bounding-box
implementation uses `getBoundingClientRect()` rather than Chromium's exact
`DOM.getBoxModel`; behavioral equivalence beyond the selected case is
unproven, and full Core/Python execution remains open.

The existing pinned 19-case async Chromium BrowserContext add-cookies manifest
(SHA-256
`7c2cdf04985d4c7b869dc5ad79557a8383bcd499f2946388d4fa83f9a4109ca0`)
has a fresh paired run on the current repaired wheel. Candidate and oracle
each record 17 passed / two failed; both failures are the same hardcoded
`localhost` versus MagicDNS fixture-address mismatch already documented in
the historical NODE-350 result. Candidate/oracle report SHA-256 values are
`8c2d0cb78770560ef5b67c41b33a06ea1e2f4b7ddec6e6311e27f938309e86cc`
and
`5b859b0384ef1402b07d1abb943a8ff548b48f7857ae5115908a73f2da551fd4`;
verified provenance sidecars hash to
`cc56001104c23d193f8bdcc27507919a740225e261f667b8e4a705fc7c10cda9`
and
`896521026e5fec97c04f51b599446a80b5442b02a6a0b00d7c42a231a712883b`.
At that checkpoint, strict 19-key merges left the 6,526-row candidate ledger
at 1,208 passed /
eight skipped / four failed / 5,306 not run, SHA-256
`1876c267051f872f5cfb424c6865204861aa76c6a8b98ece4a227f108719b318`;
the oracle ledger was 762 passed / seven skipped / four failed / 5,753 not
run, SHA-256 `c1d661e878a21d33d3fd1956d90dfc544749fea4c6787c01d8c7fe9b45e141ba`.
The storage-state figures below are the preceding checkpoint; full Python
and Core execution remain open.

Python BrowserContext storage-state now has a fixed manifest for all nine
unchanged v1.58.0 Chromium async/sync cases. Candidate v1 failed all nine at
the absent storage-state APIs; the independent oracle passed seven and failed
two IndexedDB assertions that hardcode `localhost` against the required
MagicDNS fixture. The opt-in Rust owner and thin Python facades in the repaired
wheel (SHA-256
`8f5e3234cf2d46cc9ff4c2cd9179031e59d75b53bd276c23744317d713493cb1`)
bring the clean candidate v3 repeat to the oracle's same seven passes and two
fixture-address failures. Candidate/oracle report SHA-256 values are
`a43d954ff5a14ea64aa3a86de989d350f3aefe4059b7a5ca5eef82d366d2b5ad`
and
`ed29bcc8e528ef77aa621891aa60bb346fd90b7c9d5ab214f9bb1311ae5252b6`;
their verified provenance sidecars hash to
`1565dbc19591b3b757b96b8c0c2bb947d5bab28ab0cb34786e8a094f157178e6`
and
`efd85286116c94b981f7185c87e5df36d6fe0df0f375f14582f06d92e58f18ab`.
Strict nine-key merges preserve 6,526 rows: candidate 1,208 passed / eight
skipped / two failed / 5,308 not run, SHA-256
`d1ecfa2db3e440bae951c09c55e2da862f45f964579b7e69970d59ded510f5ed`;
oracle 745 passed / seven skipped / two failed / 5,772 not run, SHA-256
`3161add7ea5243cefa071dfc2c9845ddf6e18a5dee43bc9dc7c450d7735f6a20`.
Known-ID pending storage-state Contexts are now exactly hidden; before the ID
is returned, an unidentified-creation guard can still broadly hide unrelated
newly observed Contexts. Concurrent visibility parity remains open for that
interval. An earlier repaired-wheel v3 run timed out after a just-closed task
Chrome tab; its raw artifact remains preserved and its cause is unproven. The clean
full rerun used the same wheel. Neither the nine selected cases nor the full
Python/Core corpora are all green.

Core Page response-wait NODE-432 selects all nine unchanged Chromium Page
`page-wait-for-response.spec.ts` cases with no title filter. Candidate v1
passed 1/9. Oracle v1-v3 consistently recorded eight expected outcomes and one
unexpected cached-image result because Chrome 154 did not emit the second
request awaited by the upstream case. Candidate v2 passed seven expected cases
and reproduced the shared cached-image result, but its async body predicate
still raced terminal response readiness.

The final repair carries both portable and native request identities in the
bounded Response notice and adds an independent `response-finished` Rust
owner. `Response.finished()` and body reads share one wait; the body leaf calls
CDP only after success. Final candidate v3 and oracle v3 therefore have zero
normalized outcome diff across all nine rows: eight expected and the same one
unexpected. This is outcome parity, not a claim that the cached-image upstream
case passes on the current browser.

Candidate report/provenance hashes are `1184ec4...229d` and
`98c3ef4...9dd2`; oracle hashes are `3ae728c...ba59` and
`562462e...fd7`. The fixed manifest is `e68c644...596`, the official artifact
is `30bb029...853`, and current source is `07c4c8e...ae`. Exact merges reach
candidate 1,542 expected / one skipped / four unexpected / 46,930 not run
(`7bfaf60...de7a`) and oracle 1,245 expected / three unexpected / 47,229 not
run (`ae0fc98...0322`). Both audits retain `32520af...8af0`; complete Core and
Python execution remains open.

Core Page request-wait NODE-431 selects the complete unchanged eight-case
Chromium Page `page-wait-for-request.spec.ts` file with no title filter.
Oracle v1 passes 8/8; preserved candidate v1 passes 4/8. Its four failures
separately expose timeout wording, ignored Page-default timeout, absent matcher
log context and a candidate-runner builtins fixture that disappeared after
navigation. The runner now reproduces the pinned upstream UtilityScript input
with a Context init script; that test-only repair is not candidate runtime
behavior.

The runtime shares one synchronous Page event waiter, preserves JavaScript
predicate and cross-realm RegExp semantics, captures the public call stack,
reads Rust's effective Page default timeout and formats Playwright-compatible
timeout/log diagnostics. Rust continues to own request observation, bounded
lifecycle queues and URL-glob matching. Candidate v2 passes 7/8 and identifies
the last discrepancy as an ellipsis outside Playwright's total 50-code-point
log cap. Final candidate v3 and paired oracle v2 pass 8/8.

Candidate/oracle report SHA-256 values are
`e2d90b8701e5d7a9ff8c126165e8d632101e422339c32bc7addd63641789e5d1`
and `b7a837a697d6670d4bf3aaa71366eee8beac3586cea4f05ddf25915d3535df0`;
verified provenance hashes are
`5b3b143c23be31643a821a4e663256eb92322b683d10727a45adf00724a8075a`
and `7ae11e60cddd36f07b7058725a4bde63229d5ac2de7acedfc564dd05434264af`.
Strict diffs promote exactly the same eight keys. Candidate reaches 1,534
expected / one skipped / three unexpected / 46,939 not run
(`c19a233...7abc`); oracle reaches 1,237 expected / two unexpected / 47,238
not run (`4030917...5241`). Both audits retain `32520af...8af0` and the
complete Core and Python corpora remain open.

Core Request post-data NODE-430 selects the complete unchanged six-case
Chromium Page `network-post-data.spec.ts` file with no title filter. Oracle v2
records six expected outcomes. The preserved candidate v1 reports two expected
and four unexpected outcomes, but per-attempt inspection shows that all six
calls fail at the same missing `page.waitForRequest()` API; the two annotated
Chromium expected failures are therefore masked and are not compatibility
evidence.

The repair reuses the bounded Rust request-event installation and adds
independent `request-events` and `request-post-data-json` Node profiles. One
Browser watcher publishes immutable Request snapshots; the Page listener arms
synchronously, while URL-glob compilation, Context subscription activation,
queue ownership, lifecycle reconciliation and JSON/form parsing remain in
Rust. Both minimal native profiles build and native-load independently, and
their exact Cargo feature unions are CI compile targets. After the foreign
task pages exited naturally, candidate v3 and oracle v3 each recorded six
expected outcomes. The four ordinary cases pass; file/blob and `sendBeacon`
reach matching real assertions with null post data, satisfying their Chromium
expected-failure annotations honestly.

Candidate and oracle report SHA-256 values are
`3349ede3841a906b829991db279cfa09e9e4620b764cc65fa504e3e98aa67955`
and
`7452f56eaf54c107592850ce7fbdcabbe9d87b107be532506e70a6c0a328e2c6`;
their verified provenance sidecars hash to `6c82b6a...25319` and
`76294bb...4f58f`. Artifact `fc00727...54790`, source
`afb44a6...6757` and manifest `b652990...3794` are pinned in candidate
provenance. Strict diffs change exactly the same six normalized rows in both
lanes (`a4e5f6b...be509`). Candidate reaches 1,526 expected / one skipped /
three unexpected / 46,947 not run (`8e8e6b7...be91`); oracle reaches 1,229
expected / two unexpected / 47,246 not run (`55be77a...57aa`). Both audits
remain `32520af...8af0`. Full-corpus execution remains open.

Core extra HTTP headers NODE-429 completes the two unchanged Chromium files as
one seven-invocation checkpoint. Candidate and independent Playwright 1.58.0
oracle each record two expected Context outcomes and four expected Page
outcomes. Their fifth Page outcome is the same unexpected pass: Chrome 154 now
satisfies the non-duplicated Referer assertion although the pinned source still
marks it with `test.fail`.

The old candidate evidence had incorrectly classified that row as expected
because `Response.request()` threw before the assertion. The independent
`response-request` selection now attaches the existing Rust request-history
snapshot to navigation Responses, reserves its FFI copy budget, and wraps it
through the shared Request facade. Header mutation remains in the separate
`extra-headers` leaf; Node does not reconstruct observed headers from settings.

Artifact SHA-256 is
`4d06472f978dcd147624a68b9b9fa752aa4e37fbab581891bbdb82bb03569f17`;
candidate source is
`669a396ba3ff934c69f493e797198b465c9d1af49f02b620935a16345e0a77b5`,
and manifest is
`eb26ee25396dfc0268c418895da0cfa2317eddadbaab545020ea36feb38b2d8f`.
Candidate Context/Page reports hash to `578d91a...1dd0` /
`4bf8e53...0690`; oracle reports hash to `7a38326...5ad6` /
`8e1cd42...96cc`. Their four verified provenance hashes are
`204d4e9...c5f8`, `5d16c18...ecf9`, `f651491...e4e5`, and
`aaa0c6b...ac0f`.

Strict diffs touch only the seven intended inventory keys. Candidate corrects
one masked expected row and reaches 1,520 expected / one skipped / three
unexpected / 46,953 not run (`366378d...5da9`). Oracle executes five new Page
rows and reaches 1,223 expected / two unexpected / 47,252 not run
(`7642925...2ac`). Both 48,477-row audits remain
`32520af...8af0`. Full-corpus execution remains open.

Core Touchscreen NODE-428 completes all nine unchanged Chromium Library
invocations in `tap.spec.ts`. Preserved candidate v2 fails 0/9 because the
candidate fixture adapter omitted the official `hasTouch` setup; after that
explicit Context prerequisite is applied, v3-v5 pass 8/9 and isolate the only
remaining discrepancy to navigation non-waiting. The final candidate and
independent Playwright 1.58.0 oracle both pass 9/9 with no title filter.

Raw coordinate input is an independent typed Rust capability/backend/
composition leaf over the Page input coordinator. Selector-driven Page,
Frame, Locator and ElementHandle tap compose strict target resolution,
actionability, hit-target interception, trial behavior and temporary modifiers
over that leaf. Node retains only public validation and delegation. Both Rust
input policies submit Chromium `touchStart` and `touchEnd` before awaiting
either acknowledgement, matching the upstream driver, and retain bounded
active-touch cleanup state until the end succeeds.

The final navigation discrepancy was not missing native input: diagnostics
proved both touch commands and cleanup completed. Pwright then awaited generic
topology and Page-binding publication fences, which could sit behind a
non-committed navigation whose response the test deliberately held. Upstream
always invokes tap with `waitAfter: 'disabled'`, so tap alone now bypasses
those two post-action fences; other Locator actions keep them.

Artifact SHA-256 is
`5fb122e3837f9df2742aa6ebbf82c38e0f3e379b60e426f2d9f60853b9459b91`.
Candidate/oracle report SHA-256 values are `f165c1c...71086` and
`e3eb0bd...e664e`; verified provenance values are `63a1407...ce35` and
`e886e57...6e07b`. Strict ledger diffs promote the same nine keys only.
Candidate reaches 1,521 expected / one skipped / two unexpected / 46,953 not
run (`d084416...8bac`); oracle reaches 1,219 expected / one unexpected /
47,257 not run (`1589687...8b0`). Both audits remain `32520af...8af0`.
Full-corpus execution remains open.

Core Touch NODE-427 completes the unchanged Chromium Library invocation in
`locator-dispatchevent-touch.spec.ts`. Candidate and independent Playwright
1.58.0 oracle pass 1/1 with no title/assertion filtering. A Chrome safety
preflight blocked an earlier official attempt while an unrelated nonblank page
was active, so the only pre-fix red evidence is explicitly supplemental: the
raw browser rejected plain records in TouchEvent lists, and the repaired event
program then exposed the missing `hasTouch=true` Context capability. No
official red report is claimed.

The Rust static event program now converts all three touch lists to native
browser `Touch` objects and supplies the resolved Locator element as the
default target. The existing bounded Rust device-emulation registry gains an
independently selectable Node addon marker and narrow Context creation port;
the public facade validates and selects it but owns no browser behavior. The
official artifact is `69a91ea...de1`. Candidate/oracle report SHA-256 values
are `9b5a550...918b` / `2ed7cd4...be5e`, and verified provenance SHA-256 values
are `e3907f9...cb73` / `51acd91...bd3`.

Strict candidate and oracle diffs each promote exactly the same one row from
not-run to expected with one attempt and no inventory drift. Candidate reaches
1,512 expected / one skipped / two unexpected / 46,962 not run, ledger
`7d0d844121611a01a9678680b4a8c8ddad6e874c072c927c82cbee33ca941260`.
Oracle reaches 1,210 expected / one unexpected / 47,266 not run, ledger
`fb3aef00a4321084b3489f13d76f53c05be44a910ad42f9cb919d081333506ed`.
Both invocation audits remain `32520af...8af0`. Package 368/368, strict types,
native load, dependency boundaries, exact-profile release tests, formatting,
diff checks and strict host/addon Clippy pass. Complete-corpus execution stays
open.

Core Locator NODE-426 completes all 24 unchanged Chromium Page invocations in
`locator-query.spec.ts`, the last unexecuted file in the fixed 123-case
Locator Chromium tranche. The independent Playwright 1.58.0 oracle passes
24/24. The honest candidate baseline passes 12/24; its 12 failures are seven
regular-expression text-filter cases, three ordered text-array cases, one
nested FrameLocator-composition case and the private before-query hook case.

The public facade now recognizes cross-realm `RegExp` values and forwards only
their bounded source, flags and exclusion bit to the existing Rust Locator
recipe. Ordered exact-string arrays enter the existing Rust
`HostLocator::expect_texts` operation with public whitespace, case,
`useInnerText` and negation options; mixed regular-expression arrays remain an
explicit future boundary rather than being accepted or skipped silently.
Node owns neither selector matching nor expectation polling.

FrameLocator equivalence now compares typed immutable selector recipes rather
than wrapper identity. Equivalent separately-created wrappers compose; two
different lazy iframe owners retain both recipes in a Rust-owned deferred
combination and raise the official cross-frame diagnostic on the first async
query, not while `and()` or `or()` constructs the Locator. A preserved 23/24
intermediate run proves all browser behavior and isolates this error-timing
correction.

The private `__testHookBeforeQuery` compatibility hook is deliberately scoped
to `Locator.count()`. The addon converts it to one bounded thread-safe promise;
the Rust host fixes the current execution-world snapshot, awaits the hook and
evaluates in that original world. Navigation therefore invalidates the old
world and returns the same retriable context-loss count as Playwright. This is
classified as public-test compatibility with an implementation-private hook,
not as a public extension or an excluded test.

Final artifact SHA-256 is
`c12153d71a2307303bdf1d0823b69a569c6ae0e3c3be6cdfe59181d9af6bd1bc`.
Final candidate and oracle reports pass 24/24; SHA-256 values are
`54e5e205b0efff19814a248bbd3adea42ea8b45ebab495e25131f521bc61d913`
and `60508d7c1d44af351c5e4c7725058c8286482fadfcad0f9760c195b44cdf5f8c`.
Candidate provenance SHA-256 is
`deecead3c45eb443b11c428ac2a85c05f3cae1bbddc709b67c253ec219323ad7`;
it pins source `2b781de...f171`, manifest `ab279a1...fc65`, the artifact and
upstream commit `961381e...fc75`. Oracle provenance is
`fb1b7c826f21a6777289c373fd1e9b1d3de70c140aa6d55361659f61bbcf2413`;
both verify. Preserved red report/provenance values are
`f674c52...ca65` / `a995bd8...af9e`; the 23/24 intermediate values are
`442f10b...b79d` / `8c9c39e...6562`.

Strict candidate and oracle diffs each change exactly the same 24 rows from
not-run to expected, with one attempt per row and zero inventory-field
changes. Candidate reaches 1,511 expected / one skipped / two unexpected /
46,963 not run, ledger SHA-256
`083773bb02339e2e80adf1a2f8f7179e6c87633d9e29027f1da2c6ceba1a07d1`.
Oracle reaches 1,209 expected / one unexpected / 47,267 not run, ledger
SHA-256 `6961c30592209daf95412933dfce8a76e6593330c16d1e898d70f34682804eaf`.
Both invocation audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Package units pass 363/363. Strict TypeScript, native load, dependency
boundaries, formatting, diff check and exact feature-union Clippy pass. The
exact official-profile host library suite passes 795 with 22 explicit
browser-only ignores; all selected host targets pass 817 with 30 explicit
ignores. All 123 Locator Chromium cases are now accounted for; the remaining
official Python and Core corpora stay open.

NODE-425 completes all 16 unchanged Chromium Page invocations in
`locator-misc-2.spec.ts`. The independent Playwright 1.58.0 oracle passes
16/16. The honest candidate baseline passes 12/16; its four failures are the
absent `Locator.selectText()` method, absent `Page.setViewportSize()` blocking
both the screenshot and bounding-box cases, and lowercase unknown-key
diagnostics with an extra backend prefix.

The Node addon now exposes the pre-existing independently selectable
`page-viewport` and `locator-screenshot` leaves and adds `selectText` to the
already-selected `locator-dom-actions` port. Public facades validate only
option and scalar shapes. Rust retains Page-scoped viewport state, strict
Locator resolution, actionability, scroll/stability, final geometry, capture
bytes and timeout ownership. Locator screenshot normalization removes
Page-only `full_page` and `clip` fields entirely; `Some(false)` is not treated
as equivalent to absence at the capability boundary.

A preserved 15/16 intermediate candidate proves viewport, selectText,
screenshot and bounding-box behavior against real Chrome. Its only failure
showed that the typed unknown-key error used the keyboard-specific host route,
not the generic click mapper. That route now accepts an explicit mapper in
both its ordinary and drag-aware selections, so the same typed error becomes
Playwright's `Unknown key: ...` diagnostic without changing unrelated backend
messages.

Final artifact SHA-256 is
`3edcb5cecbb6393875ab0ef739d5c1a0c9691519b5fe9659c4b498189b8b71db`.
Final candidate and oracle reports pass 16/16; SHA-256 values are
`fe9ba30c3b589913eb2ecd57dd3b39c5c015b39f98b4902b3e2e6e6f4b98482c`
and `f7010a80ed3252e388570bc96887c770c06a658419bf314ff214e3fa21a05173`.
Candidate provenance SHA-256 is
`c53a52298ac890f5ba513fc37666be1c89a50ca7b023d89fdb8f18a981c1bc90`;
it pins source `abc5859...b4fdf`, manifest `ab279a1...fc65`, the artifact and
the upstream commit. Oracle provenance is
`969c1696a199191f015b1f6dbc39e77917899af5f7e6564e4528deb33c0906fd`;
both verify. Preserved red report/provenance values are
`8290864...96ad6` / `6e28a34...b050f`; the 15/16 intermediate values are
`eec922c...739b0d` / `7e0c89d...00661`.

Strict candidate and oracle diffs each change exactly the same 16 rows from
not-run to expected, with one attempt per row and zero inventory-field
changes. Candidate reaches 1,487 expected / one skipped / two unexpected /
46,987 not run, ledger SHA-256
`8c78583d81ea5432fd49c0fcce6ba07278f987f79109e475a47fdb23b1a2c841`.
Oracle reaches 1,185 expected / one unexpected / 47,291 not run, ledger
SHA-256 `7d758353c15aa6c0a776631673142317b580ae1053d9891f27bbb088ab3c4ef9`.
Both invocation audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Package units pass 360/360. Strict TypeScript, native load, dependency
boundaries, formatting, diff check and exact feature-union Clippy pass. The
exact official-profile host library suite passes 793 with 22 explicit
browser-only ignores; all selected host targets pass 815 with 29 explicit
ignores. The remaining 24 Locator Chromium cases and the full official corpora
remain open.

NODE-424 completes all 13 unchanged Chromium Page invocations in
`locator-misc-1.spec.ts`. The first oracle attempt is retained but excluded:
Chrome rejected `Browser.setDownloadBehavior` during fixture setup for every
case. After a clean dedicated-Chrome restart, the independent Playwright
1.58.0 oracle passes 13/13. The honest candidate baseline passes 11/13; its
only failures are the absent public `Locator.dispatchEvent()` method and an
explicit message that the existing file-input leaf is not selected.

The Node addon now exposes the already-existing `locator-dom-actions` feature
and one bounded `dispatchEvent` conversion port. The public facade validates
only the event type, timeout and options shape. Rust owns initializer
materialization, strict Locator resolution, the static event-constructor
program and the original timeout. Upload behavior is not reimplemented: the
existing `file-input` capability is composed only into the named
`official-conformance-page-bindings` artifact. Default and narrower profiles
remain feature-cold, and ABI verification proves both new profile markers.

Final artifact SHA-256 is
`356f84b4a64cec93b99636aef9120e7eb3ae709e8d70f39b04516db28282c39e`.
Final candidate and corrected oracle reports pass 13/13; SHA-256 values are
`566e6fd4634aade526b08fd0153d43c9b1369b6f76a4bbcbccde542d92a36a85`
and `28b9a9aaeec42e4491ef424da1afead3c3da2c28f6cacd7cc2681f53233f568b`.
Candidate provenance SHA-256 is
`59fbc531e336099483c5c74f4837087eebbae74a9e4b854f72525690d43f5ca0`;
it pins source `ecb629e...9ded`, manifest `ab279a1...fc65`, the artifact and
the upstream commit. Corrected-oracle provenance is
`26886bcc59da56437bb6bb7930231b41a57ba7165f885e7f0331b85dc83451ed`;
both verify. Preserved red report/provenance SHA-256 values are
`a780dca...7054` / `1879b6c...1106`.

Strict candidate and oracle diffs each change exactly the same 13 rows from
not-run to expected, with one attempt per row and zero inventory-field
changes. Candidate reaches 1,471 expected / one skipped / two unexpected /
47,003 not run, ledger SHA-256
`1afb6c70b80321802bd6e836db1974bd8d7af80c6b30c6d7bdf3e5452846ecaa`.
Oracle reaches 1,169 expected / one unexpected / 47,307 not run, ledger
SHA-256 `1a73f9122f1b051d15537a188e4d60c853be001f8d908b92d9a13190bf08ccb9`.
Both regenerated invocation audits retain SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Package units pass 355/355. Strict TypeScript, native load, dependency
boundaries, formatting, diff check and exact feature-union Clippy pass. The
expanded official-profile host library suite passes 790 with 22 explicit
browser-only ignores; all selected host targets pass 812 with 29 explicit
ignores. The remaining 40 Locator Chromium cases and the full official corpora
remain open.

NODE-423 completes all 22 unchanged Chromium Page invocations in
`locator-frame.spec.ts`. The first oracle attempt is retained but excluded: a
contaminated Chrome session made every case fail at
`Browser.setDownloadBehavior`, before test behavior. A clean independent
Playwright 1.58.0 rerun passes 22/22. The honest candidate baseline passes
3/22. Its 19 failures expose an absent FrameLocator facade, missing routed
iframe fulfillment, frame replacement/detach handling, missing-frame state
semantics, owner diagnostics and terminal CSS `:has-text()` compilation.

Node now performs only public option normalization and wrapper composition.
`Locator.contentFrame()` and `frameLocator()` re-root an immutable native
recipe; nested, indexed and semantic `FrameLocator` calls compose that recipe
without eagerly resolving a Frame. Rust owns frame lookup, strictness, retries
across replaced execution contexts, one original timeout, missing-frame reads,
action re-resolution and the bounded non-frame-owner diagnostic. The separate
Rust `Route.fulfill()` leaf validates status, headers and body and settles the
paused request once.

Final artifact SHA-256 is
`fbc48ff62b671b862f272cef761d0e99a7605b4c63cd1b0a438eab9eda51382b`.
Final candidate and corrected oracle reports pass 22/22; SHA-256 values are
`076f817fcde83b8d56e6ba81aae1698a73629fc20dbc93288ed747d15b1fb71a`
and `0135d607a3c3d53626fe26faf80ce351339273c27ceccdddb1cd4a664e9e6e55`.
Candidate provenance SHA-256 is
`ded1730ea175a2a53a16d747d1307ef5d3bec53d9a1cbfff0b4bf394b80b0502`;
it pins source `179cf89...f3c0`, manifest `ab279a1...fc65`, artifact and the
upstream commit. Corrected-oracle provenance is
`27230d7606f4e21d8d496c01901f93e90208d01affc7e008a20ce45a2e254f4a`;
both verify. Preserved red report SHA-256 is
`cdefa691db6d1bed704bef8aa1bb0802d15083adf28d1c6b8674563eba8fc520`.

Strict candidate and oracle diffs each change exactly 22 rows from not-run to
expected, with one attempt per row and zero inventory-field changes. Candidate
reaches 1,458 expected / one skipped / two unexpected / 47,016 not run,
ledger SHA-256 `176cc90f1ea7ccfdffd478f3ae0f7741a48501f2de06800d086bacda5de84b3b`.
Oracle reaches 1,156 expected / one unexpected / 47,320 not run, ledger
SHA-256 `6106eae8efda6e184d245777d369c4a1910e7779e5d16a0ee2566a1e0460a546`.
Both regenerated invocation audits have SHA-256
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0`.
Package units pass 352/352. Strict TypeScript, native load, dependency
boundaries, formatting, diff check and exact feature-union Clippy pass. The
exact official-profile host release suite passes 788 with 14 explicit
browser-only ignores; the additional upload leaf combination passes 790 with
22 explicit browser-only ignores. The remaining 53 Locator Chromium cases and
the full official corpora remain open.

NODE-422 completes all 22 unchanged Chromium Page invocations in
`locator-convenience.spec.ts`. The independent Playwright 1.58.0 oracle passes
22/22. The preserved candidate red baseline passes 13/22: the missing
`evaluateHandle` breaks the rich-preview case, one timeout lacks its Locator
call log, five description cases fail and two `toString` cases fail.

Node now owns only public structural descriptions, immutable custom diagnostic
labels and the final wrapper. Rust strictly resolves one exact Element,
materializes the deferred language conversion under a fixed state bound,
evaluates through the retained Handle capability and publishes only the final
result Handle. The temporary Element and an unpublished result are released on
all failure and cancellation paths. A 21/22 intermediate rerun caught and
preserved a lost rich text-Node preview; the final Rust path refreshes that
bounded preview before publication.

Final artifact SHA-256 is
`23f0bf48bb890c20be6ba2d56b07bade1e3ad5c2856cc0eaad58586665390f05`.
Final candidate and oracle reports both pass 22/22; SHA-256 values are
`b6597943b926f76d93af08eb7906dc24f83c76ca52460101e29d2576660146ae`
and `86d5d283017f160edc77e2aa8a4116f92e3658fe7109c906044241e17eb458ed`.
Candidate provenance SHA-256 is
`fa6b26090bbf7595980301b9a621db0f03617294bcfb44c979d82f4a1d55eafb`;
it pins source `e283df2...f405f`, manifest `092087b...98de`, artifact and
upstream commit. Oracle provenance is `13cebd2...73fff`; both verify.

Strict candidate and oracle diffs each change exactly 22 rows from not-run to
expected, with one attempt per row and zero inventory-field changes. Candidate
reaches 1,436 expected / one skipped / two unexpected / 47,038 not run,
ledger SHA-256 `6db265c...b8df`. Oracle reaches 1,134 expected / one unexpected /
47,342 not run, ledger SHA-256 `61f077a...862d`. Package units pass 349/349;
the full selected host union passes 807 non-ignored tests with 22 explicit
live-browser ignores. Strict TypeScript, native load, dependency boundary,
formatting, diff check and full feature-union Clippy pass. The remaining 75
Locator Chromium cases and the full official corpora remain open.

NODE-421 reruns five complete Locator files that earlier candidate batches had
already marked expected but the cumulative oracle had not yet executed:
evaluate 6, list 1, highlight 1, click 5 and is-visible 8. Two fixed manifests
collect exactly 21 Chromium Page tests with no title filtering. Independent
oracle and Rust-backed candidate both pass 21/21 against dedicated Chrome.

The evaluate/list/highlight candidate and oracle report SHA-256 values are
`70c5dc8aea647ea029c8eaaad06acd8a504fc81905150cb59264c82f44731ade`
and `89ac5496e104e188402f57f5843aca1d99a71ccc2f7d5e24d7f4c46de73f6bb8`.
The click/is-visible candidate and oracle report values are
`af5bb38144155b3a954d4d0cec4e4875674204e083c0b4e1b55d39e300a712d9`
and `0c9ca82a55caa78242834a7ec26b28b34e7546177b18af9a041a58c51e1eb7ed`.
All four provenance sidecars verify against pinned commit, manifest and result.

A strict candidate merge replaces evidence for exactly 21 expected rows and
keeps counts at 1,414 expected / one skipped / two unexpected / 47,060 not run;
ledger SHA-256 is
`06e0444447b121e522e46a41f12ea6a5a98d2b142effda1b7598697933163546`.
The oracle merge changes exactly those 21 rows from not-run to expected,
reaching 1,112 expected / one unexpected / 47,364 not run; ledger SHA-256 is
`a813017d9be0e579fa93856ec3a5c13514349df333ad76d3bfb42587e1f58324`.
No implementation or native artifact changed. All 11 Locator files are now
fixed in manifests with a verified 123-test Chromium inventory; full execution
of the remaining files is still open.

NODE-420 completes the unchanged five-case Chromium Page
`locator-element-handle.spec.ts` file. The preserved red candidate is 0/5,
report SHA-256
`cbf16291f4aa6b48659b70408b9dd668506da27bdef141320fdf3d652e489fae`,
while the independent Playwright 1.58.0 oracle passes 5/5, report SHA-256
`0f1655db8726aabcbf7e222b9b1afb0c4b76527c36402c598e7771d2d045ae0d`.
Three failures were missing public Locator Handle methods; two were the
explicit Rust boundary against typed-parent plus XPath descendant composition.

Node now exposes only thin `elementHandle()` and `elementHandles()` delegates.
Rust retains a bounded immutable sequence of typed and browser selector
segments, compiles descendant roots without serializing a typed selector back
to JavaScript strings, and rejects non-descendant browser-program composition.
The strict single-Handle path polls absence under the caller's one timeout,
rejects multiple matches and publishes the final Handle through the existing
owner; the batch path remains a single non-waiting Rust-owned snapshot.

Final artifact SHA-256 is
`8e704ccbf94524d85424914ea6aef548f29871e79437c84d034223ed80a6f00a`.
The final candidate passes 5/5 with verified report SHA-256
`7dfc27e89e46c0e3c85e027ecfc6b00623359e9d743e6cc931ca74ba7ae71a49`
and provenance-sidecar SHA-256
`36a737def5c17fc7756ef5b2b8f18a1f8c95a3d50cc41cb6b3cdec3bbd0593a8`.
That sidecar pins candidate source
`9a3131d8320a918d9249485b3b8b02b98fc5fd2bf1d2ea043173cf1ae348c492`,
manifest
`39d5683f688d0d4ef19de9de153d4e9db61bd737fe17e749754535abd03f45db`
and upstream commit `961381e...affc75`. Oracle provenance SHA-256 is
`23643ec6cfec6129fa39b7eceb690602f9a9300bddb19756140dcf2a0de2d6ae`.

The strict candidate and oracle ledger diffs each change only the same five
rows from not-run to expected. Candidate is now 1,414 expected / one skipped /
two unexpected / 47,060 not run, ledger SHA-256
`6fa68d36c846acc3ed2a6e6ee9c05124aa5588b160471e7ea59beb533459db39`.
Oracle is 1,091 expected / one unexpected / 47,385 not run, ledger SHA-256
`6da3adccd7c1b4cfbc934b0766a0ba27c1bc0b0e48ce8e34aa8bc9b28f383d6e`.
Host release units pass 615 with 14 explicit live-browser ignores; package
units pass 346/346. Strict TypeScript, native load, dependency boundary,
formatting, diff check and full feature-union Clippy pass. Full official-corpus
execution remains open.

NODE-418 executes three complete unchanged ElementHandle Chromium Page files.
Candidate and oracle both pass query-selector 7/7 and eval-on-selector 6/6.
The convenience file preserves a real candidate 5/15 versus oracle 15/15 red
baseline: two synchronous rich-preview mismatches and eight missing public
DOM/state/action surface families account for all ten candidate failures. The
candidate ledger is 1,399 expected / one skipped / 12 unexpected / 47,065 not
run, SHA-256
`5c1cd26cc8a8bf22bf57436627de682344245b8eba68ca61dbf1e5d3506c5766`;
the oracle ledger is 1,086 expected / one unexpected / 47,390 not run,
SHA-256
`8e87e050ba9b088b849d8296b19bed0856a92fce99c82ec2214a5aa28fc1add7`.

NODE-419 implements the eight non-preview families as one exact-owner Rust DOM
read capability plus thin addon and JavaScript facades. Artifact
`6290125fb3f659e07c506f51a8248102175854b87237ac5262386a764e978218`
advances the complete unchanged candidate file from 5/15 to 12/15; result
SHA-256 is
`98fa228cd117457ca23c2fd7f52868483ebc81caf6b9d0e268dfa6626c2e87a9`.
The remaining failures are exactly two rich previews and the `nth=0` selector
chain used by option/optgroup state.

Current source models `nth=<i32>` in the typed and registered Rust selector
compilers and computes Playwright-compatible rich previews with a fixed,
bounded Rust-owned browser program over the retained Node. JavaScript only
starts the async native refresh and caches its `JSHandle@...` result. After the
unrelated Page exited on its own, final artifact
`02475436552ed68237548b0c4d5840993fde0a1db5abdea21e875cfe778db47f`
passed the complete unchanged file 15/15 in otherwise empty dedicated Chrome.
Verified report SHA-256 is
`eee2d882cac73ba18522c244ee8706fadd2adb9aa4387818a6c0e15a6e214b68`.

A strict merge changes exactly 15 rows: five expected-to-expected and ten
unexpected-to-expected. The Core candidate ledger is now 1,409 expected / one
skipped / two unexpected / 47,065 not run, SHA-256
`b5532ffbcd2819662585b698b766bb3c4e9a434a8d4540f4b790a482eedc5940`.
Host release units pass 609 with 14 explicit live-browser ignores; selector
suites pass 38/38 and 11/11; package units pass 346/346. Strict TypeScript,
native load, dependency boundary, release check and full feature-union Clippy
pass. Full official-corpus execution remains open.

NODE-417 completes the unchanged Chromium `queryselector.spec.ts` file in both
the Pwright candidate and independent Playwright 1.58.0 oracle: 17/17 in each
lane with verified provenance and identical invocation IDs. The initial
candidate passed 10/17; all seven failures were XPath forms. Rust now retains
an explicitly tagged browser selector plan instead of misclassifying XPath as
CSS, compiles `xpath=`, `//`, `..`, parenthesized XPath and mixed `>>` chains,
and executes XPath through the versioned backend runtime. Page/Frame `$$`
publishes one indexed Rust-owned ElementHandle batch and releases its temporary
carrier; poisoned page `Array.from` is never used.

A strict ledger diff changes exactly the 17 Chromium library rows from
`not-run` to `expected`. The candidate ledger is now 1,396 expected / one
skipped / two unexpected / 47,078 not run, SHA-256
`675ed26f79d073b8fe1511b54b496065779313e99f0eba100fec632531a58a38`.
The oracle ledger is 1,073 expected / one unexpected / 47,403 not run,
SHA-256
`fdd3dc71b4560a66c04336a2cf68a20bd00a8d63a69d940bfcac1c81a9f2d950`.
Candidate and oracle report SHA-256 values are
`27ede51525cf29cfecfe73d1563fd3237410053aa8a367a8236ff81ed73c078b`
and `cc03d5f08cf700e958bbe3a7eb35384af2088b1f39d01956d9743871e8a84264`.
The exact official-profile artifact SHA-256 is
`386d03a6a893c931b6aae993056b5bfef1727fbcd4e4638158217a3c64a75d3c`;
335/335 package units, strict TypeScript, native load, dependency boundary,
release check, full-profile Clippy, formatting and diff checks pass. Full
official-corpus execution remains open.

NODE-416 replaces Page/Frame `$`'s split `count()` plus Handle read with one
Rust-owned optional ElementHandle program. A missing match consumes exact
JavaScript `null` without publishing a native or portable Handle; a present
utility-world Node is adopted into the canonical world; every non-Node result
fails with native cleanup. `Locator.elementHandle()` remains independently
strict and waiting. The first live run exposed and preserved a facade bug:
deriving `.first()` rejected registered browser programs. The repaired facade
delegates directly to the intrinsically non-strict Rust query.

The unchanged Playwright 1.58.0 candidate and oracle now pass the same five
remaining Chromium selector-registration rows: path source, main/isolated
worlds, public errors, non-root chaining and bad-return diagnostics. All ten
provenance sidecars verify. A strict ledger diff changes exactly those five
rows from `not-run` to `expected`, producing 1,379 expected / one skipped /
two unexpected / 47,095 not run with candidate ledger SHA-256
`8be091c0356f65de9a05b89b5a826d07ca6090342884fabb8296128a34ee8ea6`.
The independent oracle ledger is 1,056 expected / one unexpected / 47,420 not
run, SHA-256
`cb8be736213310b13a8f271913449e5ed693aea7d9b8eda1095f5ddb59e29418`.
The rebuilt official artifact SHA-256 is
`2018ea0912efd134c8e5b9984b168f722fcdd307900399e2b02a9250398d3d9e`;
331/331 package units, strict TypeScript, native load, boundary, formatting,
release check and full-profile Clippy pass. The new complete 17-row official
`queryselector.spec.ts` tranche is recorded in a separate manifest, but no
result was created because the safety preflight again found the unrelated
XLQY Page. It was neither reused nor closed.

NODE-415 now keeps positive registered-selector ElementHandle publication,
utility-to-main-world adoption and `Browser.newPage()` implicit-Context
ownership in Rust. Page/Frame `$` derives a non-strict typed `first()` recipe,
and Browser.newPage admits no options, `baseURL`, or explicit
`acceptDownloads: false`; download acceptance stays separately unselected.
The rebuilt official artifact SHA-256 is
`6196f520808b3ceb2059ff2a88cfa5fc85faa90ee34d728f25d4c9336d072dfe`.
Package units pass 329/329, strict TypeScript, native load, formatting, boundary,
release check and full-profile Clippy pass. The rebuilt artifact's browserless
official selector slice passes 3/3 library plus 6/6 BiDi with verified result
SHA-256 values
`7906d3150e173216c4aa042403964ded687e13b1142e2c0894087458d5914fb4`
and `797c48a2daf8bb0a13ee9a55ed4603b5df6c160cc1d9502549f7d076ca593631`,
but those rows were already present in the ledger. Browser-backed NODE-415
rows remain unrun because safety inventory still finds an unrelated nonblank
XLQY Page in dedicated Chrome; it was neither reused nor closed.

NODE-414 executes the first browser-backed registered-selector row against the
dedicated Chrome CDP profile. The pre-fix candidate failed at `tag=DIV`
because `$eval` sent the custom selector to `document.querySelector`; the
unchanged Playwright 1.58.0 oracle passed 1/1. Node now exposes the existing
Rust non-strict `evaluate_first_with` lane and routes Frame/Page `$eval` and
`$$eval` through the Rust-owned registered-selector plan. The repaired
candidate and oracle pass the same row 1/1 with verified report SHA-256 values
`64ebff250009fcfc02f759b5fdca0c8fa34b07f33b6ed67de438bb2d8ee17af4`
and `451b431e79c69c8f5c205c2aa4363ec2b5a1c87a6b92ff9afde7c078a3a7996e`.
The Page/Frame `$` facade now derives the typed Locator `first()` recipe before
the strict Handle primitive, fixing multiple-match behavior in 329/329 package
units. Atomic count-plus-Handle publication under concurrent DOM mutation is
still open and is not covered by the prior one-row official result.
The composed release artifact SHA-256 is
`264137762c0f112e6d91ff21a3da4eed9284d566992f0b125124c1ab0caa802b`.
Exactly the Chromium library line-29 row moves from `not-run` to `expected`,
producing 1,374 expected / one skipped / two unexpected / 47,100 not run with
ledger SHA-256
`f363803e73aeae00ca2226c76bd5f5b53f74d9e947fc7c65725bb19198f4a465`.
The NODE-414 checkpoint package units were 319/319; current NODE-415 package
units are 329/329. Strict TypeScript, package boundary, release build,
native load and full composed-profile Clippy pass. Positive custom-engine
`page.$` Handle publication, path/content-script semantics and the remaining
selector rows are still open.

NODE-413 closes the first public Core `playwright.selectors` discrepancy with
an independently selectable Node capability. Before the change, candidate
failed all three library and six BiDi project instances because `selectors`
was absent; the unchanged Playwright 1.58.0 oracle passed 9/9. The repaired
`selector-registration` release artifact and oracle now both pass those same
9/9 browserless instances. Candidate report SHA-256 values are
`81c401160cb27ab7cdc60663fe6cc1a0d48e25b4729314cf7c5cda8abfbed53d`
and `915c1eaea6198cbb1866662b06c4346aba83ac3b49d502ecf1caa10ad219f421`;
oracle values are
`7e83dd16375d146d035f7ae0af45388f5803b7bf0d9ace9baf544d20fbd35a7c`
and `3ae5ef3f8a409f545e55ce54c04b732f598a1ed7c13ad2cf9b19961869484b36`.
All four provenance sidecars verify. A strict diff changes only the nine line
148 rows to `expected`, producing 1,373 expected / one skipped / two
unexpected / 47,101 not run with ledger SHA-256
`60b1c6f95b95f7bddb2539aacf47c02496dd8e8b41238ab731f7bb63c829a3ab`.

The public CJS, ESM and TypeScript surface is thin: function/string/path
normalization and Node error casing remain language concerns, while the
existing bounded Rust registry owns duplicate/predefined-name validation,
64-engine and 8 MiB aggregate limits, test-id configuration and registered
selector compilation for later browser sessions. The registry is shared by
the lazily created environment and attached sessions instead of duplicated in
JavaScript. The opt-in adds no Pwright package dependency; default and selected
release checks, selected clippy, dependency/boundary proof, 317/317 Node units,
strict TypeScript, eight Rust registry units and native load verification pass.
This checkpoint proves registration/error parity only; the remaining
browser-backed custom-engine rows stay open until paired candidate/oracle
execution.

NODE-411 completes launcher-metadata execution across the BiDi configuration.
Candidate and oracle each pass 12/12 browserless instances of the unchanged
`playwright.errors` and `playwright.devices` assertions over six official
projects: three Chromium/Chrome channels and three Firefox channels. Verified
report SHA-256 values are
`0fe174854e824de943effc833a670790a9319c73dc3ccee76fec3d89d1e5f2d3`
and `f47172abcb9f91afebf86d368bb4855649fc5803d141fb0e360cffaefcccb83b`.
Exactly 12 rows move from `not-run` to `expected`; the Core ledger is now 1,364
expected / one skipped / two unexpected / 47,110 not run, SHA-256
`87d6753547bad1f5f21e84952a2a8ea7af56cc75921e0382ec1ea7486cd1245d`.
No browser, native addon or new implementation participates in this lane.

NODE-410 executes nine sync Chromium BrowserContext event rows whose async
counterparts were already green. The initial candidate passed 7/9 and failed
only the two popup cases at the missing public `context.expect_page()` method;
the independent oracle passed 9/9. Pwright adds only the thin async/sync
facades over its existing Rust-owned, bounded Context page-event path. The
repaired portable-wheel candidate and oracle both pass 9/9; their sidecars
verify, their case-ID digest is
`ba6df80c7d4bd8b2a5d10980281a76df4c4b3649b491bb8f7816e4fbe2419487`,
and report SHA-256 values are
`cc5cbb5c3fea0801bde4e4be6eb49aa777fc3970278a90ad8f2a2e492144ef2a`
and `a0cf4a49b4c33f9cce3fbacae743b06ea9e17478fd7c935af3cbc11dd3058984`.
Exactly nine rows advance the Python ledger to 1,201 passed / eight skipped /
5,317 not run, SHA-256
`554cd9e63214f3b920489849cabadfe8da1559d0719705399c68f65bf3ddbaee`.
ABI104/90, the native artifact and resource limits remain unchanged.

NODE-409 executes all four sync Chromium request-timing rows from the unchanged
ABI104/90 wheel. Candidate passes 4/4. The independent Playwright 1.58.0 oracle
passes SSL and subresource but reports `domainLookupStart=0` for ordinary
navigation and redirect under Chrome 154, which the pinned test rejects. The
full 4-case reports and provenance are retained as diagnostic evidence; only
the common two-case selection is rerun cleanly and merged. Candidate/oracle
paired report SHA-256 values are
`9638463eed022460b7286e2c41bbe4ac94c0f2790de81f453332e39c5be03b24`
and `b47b3294f44af7824015e9880d4a4d5d1be65eeab9f0ae2c2f7a3f999bc7c0aa`,
with shared case-ID SHA-256
`b3f6e6dae964d66c5ffdf12b1513e6d37c6973c69bdbbaee57fa1e4f4803f6e1`.
Exactly SSL and subresource move from `not-run` to `passed`; the Python ledger
is now 1,192 passed / eight skipped / 5,326 not run, SHA-256
`79d512f3af52c78e8e8e0cf79feb41c77a1ca04cb2791538f0b453503132069e`.
The two oracle-red rows remain explicit and unpromoted.

NODE-408 adds an exact browserless manifest slice for the two public launcher
metadata assertions across Chromium, Firefox and WebKit projects. The first
candidate run passed `playwright.errors` 3/3 and failed `playwright.devices`
3/3 because the registry was absent; the unchanged oracle passed 6/6. Pwright
now vendors all 143 descriptors from the pinned Playwright source, rather than
adding only the asserted iPhone entry. The snapshot SHA-256 is
`ceb47c6d0366b273151121ade0217f1e96a4043594c8188fcc12c0c9b30f9353`
and its Apache-2.0 license and exact commit provenance travel with the package.
Final candidate and oracle reports pass 6/6 and have SHA-256 values
`2a4ca9220591a7166d0a9dd74d0581631eb0400c40fda59299eb86f92af3ed07`
and `e5f6ceb209f5b00ad63ca513584861487e1755b62cf33ea4e4eae407b1650a75`;
both sidecars verify. A strict ledger diff changes only those six rows. The
Core candidate ledger is now 1,352 expected / one skipped / two unexpected /
47,122 not run, SHA-256
`f382b081ad8346ace986db9cf8254370b34a67b8c514bca9379f573d3f40f906`.
Static metadata loading neither starts Chrome nor loads the Rust addon.

NODE-407 completes the sync Chromium service-worker policy file without a code
or artifact change. Candidate and Playwright 1.58.0 oracle each pass default
allow and explicit block 2/2 under the same trusted Tailscale HTTPS fixture.
Their verified sidecars share case-ID SHA-256
`438c118112415b7d7c6ef2b5304035493eb61e0d6bbc16a06ba88b814f047fc0`;
report SHA-256 values are
`9a9e21e364401bf7ceed7a9ced73ad87c8a43618496e8ca9a63ee317c1ddac18`
and `6c9afb0ba0eae0c23a3838920129dbfbe6436e78bd3ea02be9a364cb63cd5dfe`.
Exactly two rows advance the ledger to 1,190 passed / eight skipped / 5,328
not run, SHA-256
`94f83882339cd9af0b64200beea2098b8b7ba3ee551c2228b52bf19e1ae61013`.
The equivalent async Chromium file was already 2/2. Prior ordinary-HTTP
service-worker timeouts remain excluded environment diagnostics.

NODE-406 promotes one unchanged Keyboard row from the NODE-403 wheel. Candidate
and Playwright 1.58.0 oracle each pass `Keyboard.press("+")` 1/1 with verified
provenance and the same case ID. Report SHA-256 values are
`0c982237fedbda5cf2383b0f29e91e9a5efe70501365c6b769aadc5ce76510fa`
and `9d696c762ba7726965289b186a4bdc24fdab15eb53bca0c4aee2c7ddd4490e07`.
Exactly one prior `not-run` row becomes `passed`; the ledger reaches 1,188
passed / eight skipped / 5,330 not run, SHA-256
`672a6829fc84a4d55ec587ad23880de96a9cef3dd318719a00f47caae3fe7c5a`.

The same checkpoint preserves two candidate/oracle shared-red diagnostics
without promotion: Chrome 154 exposes a Page for the early `javascript:` popup
dialog where the old test expects `None`, and the Tailscale-only fixture host
correctly becomes the cookie domain where the test hard-codes `localhost`.
Pwright and the independent oracle agree on both observations, so no product
behavior is changed to satisfy either environment-specific assertion.

NODE-403 repairs sync exception provenance without changing ABI104/90 or a
native capability. The bridge captures the caller thread only on error and
prepends it to the retained Rust/async stack. The final installed-wheel
candidate and Playwright 1.58.0 oracle each pass the same isolated 7/7 Chromium
Page rows with verified provenance and shared case-ID SHA-256
`483df180239c145e831a2145491cdc52f606484fd910efd21b4c2bfda2a69698`.
Candidate/oracle report SHA-256 values are
`3a22d05c8bdb2ec79e19211bd6fb577574fe7893c6d16f6b3d54efa7e7a6bf35`
and `b640c1136d468c6edd847723adf8f1c58ddb8b175ced6dc61f9707e59c66190b`.
Exactly seven `not-run` rows become `passed`; the ledger reaches 1,187 passed /
eight skipped / 5,331 not run, SHA-256
`036aed9d37c7e88dfbce0ffe4210e7a16e11b2f06b860a8200015a2e99257eda`.

The two remaining Chromium `Page.pause` rows are not promoted. A preserved
nine-row diagnostic shows the attached visible Chrome conflicts with
upstream's `headless=True` fixture assumption; the candidate correctly refuses
to fabricate a headed Inspector capability. Two repaired wheels have equal
normalized payloads, the installed wheel passes 250/250 units, and dependency,
release, format, lint, type and Python 3.9 gates pass.

NODE-402 promotes the two remaining sync request-fulfillment Chromium rows
without changing code or ABI. The NODE-401 wheel-backed candidate and
Playwright 1.58.0 oracle each pass 2/2 with verified provenance and the same
case-ID digest. Candidate/oracle report SHA-256 values are
`b95188b98d62986ba046d643cf76d4429566b2627aac68b280c596dd89d3aa78`
and `637626c1e57deda856ccab3b3c000139934fba966c38c228e43f4e46949342dd`.
Exactly two prior `not-run` rows become `passed`; the ledger is now 1,180
passed / eight skipped / 5,338 not run, SHA-256
`1846062292632f04561166ad19d4e8265a624f3eb837aa969ec14ebf6db55d78`.

NODE-401 closes and promotes the complete 25-row official TodoMVC examples
profile. The final portable-wheel candidate and Playwright 1.58.0 oracle each
pass the same isolated 25/25 Chromium cases with verified provenance and the
same case-ID digest. Candidate/oracle report SHA-256 values are
`19e27f3c63ecf13804581d9c079afc5393ca005560d7770d34db07be82b82967`
and `a8792fe4c0a1a19db6dcf3aaaf73e7cedb96c78edd70d14b9b0c2a4f59db8fa6`.
Exactly 25 prior `not-run` rows become `passed`; a strict ledger diff reports
no other change. The complete Python ledger is now 1,178 passed / eight skipped
/ 5,340 not run, SHA-256
`bfb8e95031d10240bc110b16ba18a800711f0e5e4cc513cef4e8cfd0f14198bd`.

The initial candidate diagnostic found two real compatibility gaps: teardown
needs a stable inactive `BrowserContext.tracing.stop()` no-op, and a nested
arrow inside an expression must not classify the whole evaluation source as a
function. Tracing recording/output remains explicitly unsupported. A separate
initial oracle diagnostic records Chrome 154 rejecting
`Browser.setDownloadBehavior` when no default Page exists; creating one
task-owned `about:blank` Page through Pwright restores the documented
precondition, after which the oracle passes 25/25. Neither diagnostic is
promoted. ABI remains 104 with 90 selected features. Two portable wheel builds
have identical normalized payloads, the installed wheel passes 249/249 units,
and dependency, release, format, lint, type and Python 3.9 gates pass.

NODE-400 closes the typed string-matcher discrepancy for Python and Node while
keeping matching, validation, DOM reads and retry/deadline ownership in Rust.
Python ABI advances to 104 with the existing 90-feature profile. The unchanged
async/sync Chromium `to_have_class` and `to_have_value` invocations pass 4/4 in
both isolated candidate and Playwright 1.58.0 oracle lanes, including regex and
mixed ordered class-array coverage. Candidate/oracle report SHA-256 values are
`ab19a7424ab038b1a1757966419b14aca7b92f449eb3e51843d860e9a201b826`
and `5fb83a36c9b1fb5a6e67d8cb2669a9a021330a2b20def1044e2237c49baa7458`.

Only the four green candidate rows are promoted. The complete 6,526-row Python
ledger is now 1,153 passed / eight skipped / 5,365 not run, SHA-256
`ab288ffacb37a4632adcf1be1d70b6beb4a7dee82f26f8525a43a0f3d922e020`.
The candidate runs from a fresh `site-packages` installation of the portable
ABI104 wheel, which also passes 246/246 units; two builds have identical
normalized payloads. Node passes
309/309, and dependency/boundary/type/format gates pass. An earlier candidate
report with four setup errors remains diagnostic only: it found the production
capability-count omission for the separately registered history leaf, and no
test body from that report was promoted. Full-corpus execution remains open.

NODE-399 implements the previously audited TodoMVC public-surface gaps without
promoting any upstream outcome: substring/regex text, checked, empty, exact
value, ordered exact class lists and an independently selectable Rust-owned
`Page.go_back` capability. Python and Node bindings are thin adapters over the
same host operation. The isolated runner also fixes directory-profile matching
with a path-segment-bounded AAA regression. Static, typing, schema, dependency
and release-build gates pass. The candidate is installed from an
auditwheel-repaired ABI3 wheel whose native module uses bundled OpenSSL dylibs;
two repaired builds reproduce byte-for-byte outside the CycloneDX build
identity fields, and the repaired install passes 246/246 units. The 25-row
candidate and oracle collect-only runs each report 25 Chromium cases and have
verified provenance, but those JUnit files contain no executed testcases. The
candidate/oracle execution remains
pending while another task owns the dedicated Chrome tab. The canonical Python
ledger therefore remains 1,149 passed / eight skipped / 5,369 not run.

NODE-398 resolves the corrected Python closure into 95 content-addressed
fixture definitions: 68 upstream, 26 frozen-environment and one pytest builtin.
No definition or row is unresolved. All 25 TodoMVC invocations resolve the
`pytest-playwright==0.7.2` Page provider and enter the
`fixture-creates-page` band. The 5,360 applicable Python rows still unrun are
now partitioned into 4,106 Page, 486 Browser, 453 Context, 292
BrowserType-only and 23 no-runtime-fixture rows. Binding and definition ledger
SHA-256 values are
`f34cc45de0bbb8452c7b7917b403d4c7e21c0604066cca4b6b744d9045ef0d24`
and `26dd34a136b9f76778317cc89784b83342e6a79a57a84e543cce58a3d7da4309`.
An independent two-profile rerun is byte-identical. Outcomes and ABI remain
unchanged.

NODE-397 collects complete fixture closures in the two official profiles. The
6,501 core rows retain upstream's autouse servers and asyncio fixture, while
the 25 examples rows use the separately frozen plugin environment and carry a
21-name plugin closure without the core server fixtures. The closure ledger
SHA-256 is
`3f317451434319c80dc643efc8f4be24ea097cdef2a0e0948eaa3a5c45e28903`.
No fixture or test body runs.

NODE-396 reconstructs upstream's separate `test_examples` CI job instead of
collecting the repository under one root environment. It records 6,501 core
rows plus 25 Python 3.10 TodoMVC rows from a hashed Playwright 1.58.0,
pytest 8.4.1 and `pytest-playwright==0.7.2` lock. Exactly those 25 IDs acquire
the official `[chromium]` parameter suffix; a strict rebaser moves only
pristine `not-run` outcomes. Canonical inventory, outcome and applicability
ledger SHA-256 values are
`3dfb11b6ad7eb4736166ee2c4f7fa32cda78063324ad8314ce12f3e2d49b9d41`,
`456fcb0f097261f71a24e4500308740684250a3c393cda5d3161ed52918c574b`
and `31195102f6404a92903d19f35df0b73223f7e078d082fc7e419e0ae99533a090`.
The total and outcomes remain 6,526 rows and 1,149 passed / eight skipped /
5,369 not run.

The following NODE-394/395 root-only evidence is retained as a superseded
diagnostic because it exposed the missing profile. Its unresolved example
classification is not the current canonical result.

NODE-395 resolved the old root-only Python closure into 77 content-addressed fixture
definitions and 6,526 per-invocation resolution records. It retains source,
line, scope, dependencies, autouse status, override chains and an explicit
semantic role for every resolvable definition. The sole unresolved definition
is the absent `page` provider for all 25 TodoMVC examples; those rows remain
applicable and `not-run` instead of being silently skipped or treated as valid
Page creation.

The status cross-tab partitions all 5,360 applicable Python rows still unrun:
4,081 fixture-created Page rows, 486 fixture-launched Browser rows, 453
fixture-created Context rows, 292 BrowserType-only rows, 23 rows with no
Playwright runtime fixture, and 25 unresolved examples. This is a fixture-only
classification; body-level launch still requires source analysis. Binding and
definition ledger SHA-256 values are
`36f993858ee5a01c0a11f002f6183bf8e521419fc1f857d7cc95e4e33cfa5ae4`
and `28340e32bf3c1ccdb6bdf72a3c9b03783ebe218512b0bcf4ceb14c89c99f80ad`.
An independent rerun is byte-identical. Outcomes and ABI remain unchanged.

NODE-394 collected direct, initial and transitive/autouse pytest fixture names
for every one of the 6,526 pinned Python invocations under `--collect-only`;
no fixture or test body executes. It proves exact one-to-one agreement with the
canonical inventory and exact ordered agreement between NODE-393 source
parameters and pytest's direct fixture names. `event_loop_policy` reaches all
6,526 rows; the 6,501 non-example rows all include upstream's `start_server`,
post-test hook, browser/platform skip machinery, browser name and request
fixture. The complete ledger and summary SHA-256 values are
`fde11844e88c4cadadbac096c0490ebb682eea9e97855806db13fb6925f3f988`
and `42c34b9584855113d75bfe7ad373c059081f2f10b310614019d741774c1d5260`.
NODE-395 adds definition locations, scopes, override chains and semantic roles;
the missing TodoMVC provider remains explicitly unresolved rather than hidden.

NODE-393 records the source-declared pytest inputs for every Python invocation
without pretending they are complete fixture closures. Thirty-six distinct
inputs are cross-tabulated with applicability and outcomes; the largest
unfinished overlapping queues are `page` 4,113, `server` 3,294, `context` 708
and `browser` 480. The 6,526-row audit and summary SHA-256 values are
`3c58942c03e7a1aefe41d472258ea0ed07a3388626e645587bce82f9cb93c2e3`
and `6c0eb8a7e7de67eb4d80b387808b1b83273630370fd4c6d05efb5ab5e804508a`.
NODE-394 preserves this direct-input layer and verifies it against pytest's
separate complete fixture closure.

NODE-392 introduces a manifest-bound browserless Python lane without relaxing
the default fixture isolation rules. Static source verification proves
the two selected sync lifecycle functions do not consume browser fixtures or
perform browser creation/navigation; the runner and independent provenance
verifier derive the same no-browser command. Because upstream starts fixture
servers through an autouse session fixture, the policy requires the server
runtime and verified Tailscale binding even for this selection. Candidate and official
oracle each pass the three `stop_multiple_times` parameter rows and fail the
three asyncio-loop-dependent `multiple_times` rows with the same missing-error
outcome. Only the shared green rows are promoted, adding two previously unrun
Firefox/WebKit outcomes. Python is now 1,149 passed / eight skipped / 5,369
not run, ledger SHA-256
`b7c38ba4cdb1cacb9b630e07e8f18b0a4f72f4fd8fc5cc92231a7ed613a35bf8`.
No browser tab, ABI or runtime capability changed; full Python
and Core execution remains incomplete.

NODE-391 stops treating an internal test helper as proof that the behavior
under test is implementation-internal. Public-root tests that use upstream
internals only for fixtures, platform adaptation or result inspection now
enter the explicit, applicable `public-api-with-private-test-helper` queue.
Known local implementation tests remain internal through exact source rules:
EventEmitter, image tools, selector/CSS parsers, debug controller, locator and
proxy generators, library unit algorithms and Page leak internals. Internal-
importing sources outside the public roots remain product infrastructure.

This moves 6,279 invocations into the applicable helper queue and 568 into
product infrastructure, reducing the implementation-internal class from
10,305 to 3,458 without changing an outcome. The helper queue already contains
684 expected results and 5,595 not-run rows. Complete counts are now 32,148
ordinary public candidates, 444 public-with-private-hook, 6,279
public-with-private-helper, 3,458 implementation-internal and 6,148 product
infrastructure. The 48,477-row ledger and summary SHA-256 values are
`32520af75e4ac14352a9060e6f7156900bb52c4a3d99fe927311ba7490328af0` and
`d2f43298716b10b67f9a15bd262c926a2f19796810c94da95121117a56453094`.

NODE-390 removes a source-file-wide private-access false exclusion. Thirteen
sources had all 1,951 expanded invocations marked implementation-internal only
because some code in the file used `toImpl` or a private client field. The
audit now recomputes that reason inside each bounded invocation. It keeps
direct private access internal and explicitly follows the shared
`expectScopeState` and crash fixtures, each covering 45 rows.

Of those source-internal rows, 1,367 now return to their actual queues: 1,334
ordinary public-API candidates, 27 public behaviors with private test hooks
and six Playwright Test product-infrastructure rows. The complete audit now
counts 32,148 public candidates, 10,305 implementation-internal, 5,580 product
infrastructure and 444 public-with-private-hook invocations. Its unchanged
outcome cross-tab is 1,346 expected / one skipped / two unexpected / 47,128
not run. The 48,477-row ledger and summary SHA-256 values are
`f8aff8d62e4ff1a32862eaa5b51967d1312b1c6636fe1baab021c53d48a54971` and
`4223ac1278b23bd6344c524932ba0a202c76456f54d73a55f7dcf8a21274fc5d`.

NODE-389 separates official example-application validation from Playwright
public-API conformance. The inventory already gives all 67 invocations across
48 `examples/**` files the `official-example` role. They assert the behavior of
five example applications, so the audit now preserves them as explicit
product-infrastructure rows with the reason
`product-infrastructure-role:example-application-validation`; all 67 remain
`not-run`, and no outcome changes. The audit ledger now also retains each
inventory role as first-class provenance.

The complete classification is 30,814 public-API candidates, 11,672
implementation-internal invocations, 5,574 product infrastructure invocations
and 417 public behaviors with private test hooks. The 48,477-row ledger and
summary SHA-256 values are
`fa4d18cf1a69e33a3b39b60fa03b6bfb5f920deec18a74f5c1a5f06b3a01b1c4` and
`53c1e34c4c8c6b543c602ef73d472e8864e1cc4f138fb196aa88e763cada8ea8`.
This refines applicability accounting only; all applicable execution remains
part of the open goal.

NODE-388 corrects one applicability-classification error without changing a
test outcome. All 19 files under `tests/library/events/` exercise the upstream
repository's copied `./utils` EventEmitter rather than the Playwright or
Pwright public API. Their 477 expanded invocations now carry the explicit
`local-implementation-under-test:tests/library/events/utils` reason: 36 retain
their prior implementation-internal class and 441 move from public candidate
to implementation-internal.

The regenerated, complete 48,477-row audit now classifies 30,881 public-API
candidates, 11,672 implementation-internal invocations, 5,507 product
infrastructure invocations and 417 public behaviors that depend on a private
test hook. It cross-tabulates the unchanged NODE-372 outcome ledger at 1,346
expected / one skipped / two unexpected / 47,128 not run. Audit ledger
SHA-256 is
`74baa2e47b58e90a9e0eac1021bf47ea7f2b01f7aa38af5730ca1bbf9d3b5674`;
summary SHA-256 is
`72d5ba6f2397718c095ffc3d4f6fef5d68d38ca1623b5d76452d740df3c35843`.
This closes one accounting defect, not Core execution or the full goal.

NODE-387 promotes 26 green rows and one upstream browser skip from the fixed
29-case async Chromium keyboard manifest. The preserved ABI 102 candidate was
16 passed / 12 failed / one skipped, the Playwright 1.58.0 oracle is 27 passed
/ one failed / one skipped, and the fresh ABI 103 installed candidate is 26
passed / two failed / one skipped. The candidate-specific public discrepancies
are closed by one Rust-owned ElementHandle focus/input boundary and thin
Page/Frame selector facades.

The two remaining red rows are not merged. Cross-origin input fails in both
complete lanes. A 30-process diagnostic of the other row reproduces the same
missing-initial-keydown failure 2/30 in both candidate and oracle, so it is a
shared persistent-Chrome/CDP reconnect flake rather than evidence for a second
input implementation. The cumulative candidate ledger advances only verified
green evidence to 1,147 passed / eight skipped / 5,371 not run, SHA-256
`607b552d6435948ed6e66fb8c9d0d75c92e78de51de007c20fb35f4f5a03ab8c`.
ABI 103 retains 89 selected features, passes 239/239 installed units and a
fresh 6,526/6,526 collection, and reproduces across two final-source wheels.
This is one Python tranche; the remaining Python rows and the Core JS/TS corpus
are still unfinished.

NODE-386 promotes the unchanged five-case async Chromium focus manifest from
candidate 3/5 versus oracle 5/5 to a fresh ABI 102 installed-wheel candidate
at 5/5. The two repaired rows use `Page.keyboard.type()` and
`Page.keyboard.press()`; the facade delegates all five public Keyboard
operations to the already bounded Rust host instead of implementing input in
Python. Candidate is now 1,121 passed / seven skipped / 5,398 not run with no
recorded failure, ledger SHA-256
`3cec1a0bd837082094831e4cf5b11e20625e5fb095c1d06ef6b5f643027f0fe0`.
The 89-feature wheel passes 232/232 installed units and reproduces from two
final-source builds. This closes the recorded focus discrepancy, not the
remaining 5,398 Python rows or the Core JS/TS corpus.

NODE-385 promotes the four unchanged async Chromium resource-timing rows
through a preserved 2/4 candidate baseline, a 4/4 official Playwright 1.58.0
oracle and a 4/4 fresh ABI 101 installed-wheel candidate. The repair is
composable: `request-timing` adds the Rust timing sidecar and CDP normalization,
while a bounded publication fence closes the request/response notice race.
The language facade owns neither protocol timing nor an independent network
queue.

The Python candidate ledger is 1,119 passed / two failed / seven skipped /
5,398 not run, SHA-256
`d5d26ca3c234e119808548b48cb6ca96e5802718f48adb619cac73006e2cae95`;
oracle is 738 passed / seven skipped / 5,781 not run, SHA-256
`1ff080b72543b70bd04a7c327e8dcfcc2260aa844953ce9de2ebb4dba8b60ba3`.
Both final result sidecars verify. The two focus failures remain explicit.
The ABI 101 / 88-feature wheel passes 226/226 units and two final-source wheels
reproduce after CycloneDX normalization. This advances four Python rows only;
the complete Python and 48,477-invocation Core JS/TS corpora remain unfinished.

NODE-384 executes the two remaining Chromium rows in upstream
`test_request_fulfill.py` against the fresh ABI 100 installed wheel and the
independent Playwright 1.58.0 oracle. Both lanes pass 2/2, closing Chromium
coverage for the file at 3/3 without a runtime change. Candidate is now 1,110
passed / seven skipped / 5,409 not run, SHA-256
`324dc918352b2afa46745ab26825f0b7d19db9f657b237b769c68ad2754585c1`;
oracle is 727 / seven / 5,792, SHA-256
`393bf16e0e001f6a7f264d1f852fdebd0abaa8fef4d6c6b47c464afc4f6a761a`.
This is evidence for existing Rust composition, not full-corpus completion.

NODE-383 promotes 15 unchanged async Chromium JSHandle rows through a preserved
6/15 candidate baseline, a 15/15 official Playwright 1.58.0 oracle and a 15/15
fresh ABI 100 installed-wheel candidate. The repair covers nested/repeated
Handle arguments, Date JSON values, property enumeration and official Handle
previews while keeping native Handle identity, admission and disposal in Rust.

The Python candidate ledger is 1,108 passed / seven skipped / 5,411 not run,
SHA-256
`3708cc9f5c27b9e1a4a40e37c1e12bf8fca016e55a7dfb0ad55462ca8be2611a`;
oracle is 725 / seven / 5,794, SHA-256
`01028d3a743ce7c31a6a8c517be3b6f8bf1c702a3c5209d86fb9e1b66a12f0f5`.
Applicable candidate behavior is 1,105 passed / seven skipped / 5,402 not
run. The ABI 100 / 87-feature wheel passes 221/221 units and reproduces after
CycloneDX normalization. This remains a Python checkpoint, not full Python or
Core JS/TS corpus completion.

NODE-382 advances Python to ABI 99 and promotes 17 more unchanged async input
rows from a fresh installed release wheel. The opt-in
`file-input-native-paths` leaf keeps native directory preparation, exact
target preflight, DOM mutation and cancellation cleanup in Rust without
changing ordinary remote file paths into colocated paths. Candidate passes
17/18 and official Playwright 1.58.0 passes 16/18 in the required CDP lane.
Both hit the same official 50 MiB guard for a 200 MiB single file; only the
oracle also hits its aggregate transport guard for ten 49 MiB chooser files,
which Pwright's native chooser path passes. All 16 oracle-executable rows
agree, and environment-limited rows remain visible and unmerged.

The Python candidate ledger is 1,093 passed / seven skipped / 5,426 not run,
SHA-256
`2e29c50908b763d1cf39defafc540557aa16fd7e1091d0c1ea6f28270ac544f7`;
oracle is 710 / seven / 5,809, SHA-256
`5dd92c0935ec10748d6990aa04f4866e63e47520fe718870394a59461a2d8ee3`.
Applicable public behavior is 1,090 passed / seven skipped / 5,417 not run.
The installed artifact reports ABI 99 / 87 features and passes 217/217 units.
This is a checkpoint, not full Python or JS/TS corpus completion.

NODE-378/379 promotes four unchanged Python async input rows through paired
source/oracle and final installed-wheel evidence. The Rust candidate now owns
utility-world chooser-node resolution and `DOM.setFileInputFiles` by object
identity behind the opt-in `file-chooser-native-paths` leaf. Collection
evaluation supplies the browser user gesture and terminal Page-binding
publications are fenced before upload returns. The final wheel and independent
official Playwright 1.58.0 oracle each pass 4/4.

The canonical Python candidate ledger is 1,076 passed / seven skipped / 5,443
not run, SHA-256
`0284d9c1750530e607c3b07daff759278ffae92964751c644f7244d801a80cbf`;
oracle is 694 / seven / 5,825, SHA-256
`d5fc04bc059fdab72dd20e88ea8157655aa1bf6e62ab4191c7152ec3fe635d69`.
The complete applicability cross-tab remains explicit at 1,073 applicable
passes, seven applicable skips and 5,434 applicable not-run rows. ABI 97 has
85 selected features; two release wheels reproduce after CycloneDX
normalization and the fresh installed artifact passes 213/213 units plus a
verified complete 6,526-ID collection. This checkpoint advances Python only;
the complete Python and Core JS/TS corpora are still unfinished.

NODE-374 turns the NODE-373 sidecar into verifiable evidence. Future records
also fingerprint the pinned manifest, both selected harness preloads, the
candidate package's `package.json` plus complete `src/` tree, and the exact
selected `.node` artifact. The independent
`scripts/verify_playwright_official_provenance.py` rejects schema drift,
non-loopback CDP, non-MagicDNS fixture hosts, invalid commands, manifest or
adapter drift, moved/tampered reports, modified harnesses, candidate-source
drift and native-artifact drift. Candidate profile names reject path syntax
and worker counts must be positive before execution. The runner and verifier
now pass 11 AAA tests.

NODE-373 adds a deterministic provenance sidecar for every future Core result
created by the guarded runner. It is written for green and red reports after
the report exists. Historical reports are not retroactively labeled; the
first authentic runtime-produced and independently verified sidecars remain
pending a memory-safe execution lane.

NODE-372 revalidates the complete unchanged nine-case
`page-add-init-script.spec.ts` file against the current explicit
`official-conformance-page-bindings` artifact and adds the missing independent
oracle run. Both lanes pass 9/9. Candidate/oracle report SHA-256 values are
`fc61da558be2f1a47f73487e220d89b53f1befd040417552ffd93a9aee46f6ab`
and `bde4ec0baa03113788407eecb02589f26d30227e140c67e3f0c4630c72d0be9e`.
Those candidate rows were already `expected`, so the candidate ledger remains
1,346 expected / one skipped / two unexpected / 47,128 not-run with its
NODE-370 SHA-256. Exactly nine oracle rows move from `not-run` to `expected`:
the oracle ledger is 1,051 expected / one unexpected / 47,425 not-run,
SHA-256
`5aa674c8f42d538f54b58caf0b506662f6cb3ced6b736c4052b4eaa0d010b842`.

The first NODE-372 diagnostic accidentally omitted the native profile and
therefore loaded the default artifact, producing 1/9 with eight explicit
missing-method errors. It is retained as diagnostic evidence and is not merged
into either ledger. The runner now fails closed before execution unless every
candidate run supplies `--candidate-profile` or `PWRIGHT_NATIVE_PROFILE`;
`default` must be named explicitly when it is intentional. Five AAA tests
cover explicit selection, environment selection, missing selection and oracle
isolation, plus deterministic result provenance.

NODE-371 adds a per-invocation classification audit without changing any test
outcome. `scripts/audit_playwright_official_invocations.py` first verifies the
pinned clean v1.58.0 checkout, the 48,477-row inventory count and its recorded
SHA-256, then bounds every test body by adjacent collected source locations.
It records implementation-private `__testHook...` dependencies separately
from ordinary public-API candidates instead of treating the whole source file
as the unit of applicability. Five AAA tests cover ordinary public calls,
private hooks, already-internal sources, reason deduplication and adjacent-test
source boundaries.

The complete audit classifies 31,322 invocations as public-API candidates,
11,231 as implementation-internal, 5,507 as product infrastructure and 417 as
public behavior that depends on an implementation-private test hook. Its
ledger SHA-256 is
`25ec81b43f47cdcb41e8d04b04f0d199d6733ea90e714a6f732dbb893018a90a`.
The audit validates and cross-tabulates the unchanged NODE-370 candidate
ledger: both current unexpected rows are in the hook-dependent class, one for
`__testHookAfterStable` and one for `__testHookBeforePointerAction`. They remain
unexpected outcomes; the audit does not silently convert them to skips,
expected results or exclusions. The candidate outcome ledger and its
`67dd35b67d392963e1848192c88ae1bbd5e811dee4856244022f924958eae508`
SHA-256 are unchanged.
After the unrelated page exited, the guarded endpoint contained only blank
pages and the same complete CSP file passed 4/4 in the independent oracle.
The oracle report SHA-256 is
`a60aba4f4226eb3d74eef621292a7dc059b81c8450e867c77057b2cb69dc2fbf`.
Exactly those four rows move from `not-run` to `expected`; the oracle ledger is
now 1,042 expected / one unexpected / 47,434 not-run, SHA-256
`626e596ce9e0579fdcb8de5025060c51290cd6d78fb641286f64748d6d9b0a30`.

NODE-370 adds an independently selected Node `bypass-csp` leaf over the existing
bounded Rust Context policy owner. The unchanged
`browsercontext-csp.spec.ts` candidate baseline failed 0/4 because the facade
explicitly rejected `bypassCSP: true`; the rebuilt candidate passes 4/4,
including meta/header CSP, cross-process navigation and iframe installation.
Baseline/final candidate report SHA-256 values are
`04ea4ea2f89767200229eb79e7137951b3b1a63de42c87a8ce116449fcd11450`
and `52762acf1fb0ba47924897e6827c3228df8fcb51700aba6de3d60d1a4ff89a2f`.
The candidate Core ledger advances to 1,346 expected / one skipped / two
unexpected / 47,128 not-run, SHA-256
`67dd35b67d392963e1848192c88ae1bbd5e811dee4856244022f924958eae508`.
The guarded harness initially refused the matching oracle while another task
owned a nonblank Bevy page. That page was not closed or modified. After its
renderer exited naturally and only blank pages remained, the unchanged oracle
also passed 4/4. NODE-370 therefore has paired evidence without taking over
another task's browser state.

NODE-369 runs the complete unchanged
`browsercontext-set-extra-http-headers.spec.ts` file through the already
selected Rust extra-header owner. Candidate and independent oracle each pass
2/2: Context headers combine with a Page override, and a non-string creation
header preserves Playwright's validation error. Candidate/oracle report
SHA-256 values are
`c5694f1ef1c40cbb50b0b397bff35a45fdc70d678747cfd230de31f0149fb099`
and `a9cdcafcc44f8c8da97d7420f7f4e6a15a3cde000c8082707ccfc7757e31131f`.
The paired checkpoint ledgers were candidate 1,342 expected / one skipped /
two unexpected / 47,132 not-run, SHA-256
`bb33fa9198ee0b02ba025eeb5562efc0617d1c2cc711cedf4239ddc066e66ab2`,
and oracle 1,038 expected / one unexpected / 47,438 not-run, SHA-256
`87c15b704e4028dc3f9af0139b79f9ae21084ea8fbfb3bbf68f1c4560005edef`.

NODE-367/368 closes the Dialog and first Page-routing gaps in the current
unchanged Page sample without moving either browser behavior into JavaScript.
Dialog action identity, one-shot accept/dismiss settlement and teardown
revocation remain Rust-owned; the Node facade supplies only Playwright-shaped
objects and `EventEmitter` delivery. Page route matching, bounded call
publication and continue/fallback/abort settlement reuse the existing Rust
host leaf. Node retains handler identity and invokes it, including the official
pattern where a handler returns before a later `route.continue()` call. The
current facade deliberately accepts only the backed string matcher and does
not claim regex/predicate, fulfill/fetch or override parity.

The complete unchanged `page-click-react.spec.ts` file passes 3/3 in candidate
and oracle, with report SHA-256 values
`b1ff64a4250f5552a69264c4669c989d7be70618d3383e3cf9202b52caa4d767`
and `55983f4402156e6b3871fa875cb062a7b988ce215a060ad9b5e100338e8f92a7`.
The complete unchanged `page-wait-for-load-state.spec.ts` file passes 18/18 in
both lanes, with report SHA-256 values
`2c67d3ce97507554e515de695b290a5a50e83cfbde36254924f6df3f8dfdedee`
and `5cee1d1965945b41b51b649e824737791dfc36d521bf5b5599516cafe5f9ce44`.
The canonical Core ledgers advance to candidate 1,340 expected / one skipped /
two unexpected / 47,134 not-run, SHA-256
`da4fe162d3dfeb7fa87017834c1d412fbbca6a84c026b5284d648156fdcca483`,
and oracle 1,036 expected / one unexpected / 47,440 not-run, SHA-256
`cd0ef619ca9c96955e5e283a5089a785e6a19214d2c846b601d28d256f50b0e0`.
The two executed candidate failures both invoke implementation-private
`__testHookAfterStable` or `__testHookBeforePointerAction` options through
`as any`; they remain explicit pending applicability/port decisions rather
than being deleted, reclassified as passes or implemented in JavaScript. The
oracle discrepancy remains the documented external-CDP `navigator.webdriver`
adapter limitation. Core and Python full-corpus execution remain incomplete.

NODE-366 completes the unchanged four-case `frame-goto.spec.ts` selection in
both candidate and oracle. After the initial document commits, Rust follows a
subsequent same-Frame client redirect through its lifecycle owner while keeping
the original navigation response as the public return value. Node adds only
the Playwright-facing typed timeout call-log presentation. Candidate/oracle
report SHA-256 values are
`4ab237b0a8a1d47cf9f03b24a49d6dd0d10f2e96f794aba857683e74fa01be86`
and `1dd1411a3aa357c80f43ddf3bf895f6085a796453f5700e732619e6f03058ee9`.
Its superseded candidate ledger recorded 1,337 expected / one skipped / five
unexpected / 47,134 not-run, SHA-256
`82f9322e290cd9ba1c4b08b9a0bd6eea8113ac8cf7de9ee84b8f8e32ec71ef51`;
the oracle recorded 1,015 expected / one unexpected / 47,461 not-run,
SHA-256
`8c53022b80fc2db1e741974181711414f37a1532e137994c85111f8c62eced18`.

NODE-365 eliminates the candidate discrepancy in the complete unchanged
`tests/page/page-basic.spec.ts` file. The failure was not keyboard dispatch:
a bounded diagnostic observed `keydown`, `keypress` and `keyup`, the focused
input, the installed page handler, and its DOM mutation, while the public
`console` listener received nothing. The Node facade had never mirrored its
first listener into the existing Rust-owned console subscription. Page and
BrowserContext now preserve JavaScript listener identity and ordering in
`EventEmitter` while synchronously forwarding only the zero-to-one and
one-to-zero `console` listener transitions to bounded native setters. The
selected artifact must export both setters, so a stale marker-only binary is
rejected before use.

The focused unchanged Enter row passes 1/1, report SHA-256
`a0271ed76ff4c9ce565c91c9ae75f01913080c0259dcb7d281fe591abd6d18aa`.
The complete candidate file passes 18/18, report SHA-256
`8374cbd51ea846d95f602f47938d7e55b7d3b13bde088444dc734f527b685357`.
The independent oracle records 17/18, report SHA-256
`6598840eaf040c945e222e1992156eec64e313ff5436c3bde650f24f544d5b2e`:
only `has navigator.webdriver set to true` fails because the shared official
adapter replaces Playwright-owned launch with `connectOverCDP` to the external
dedicated Chrome, which was not started with `--enable-automation`. Candidate
owned Context Page and OOPIF sessions explicitly install
`Emulation.setAutomationOverride({ enabled: true })`; external/default
Contexts remain untouched. This oracle result is an explicit adapter-baseline
limitation, not a hidden candidate pass.

The canonical Core ledgers advance to candidate 1,336 expected / one skipped /
six unexpected / 47,134 not-run, SHA-256
`cb22fece25ca573b34ed8a9ccf97aff18f8eb2f1966aa556b375f5b986fe085a`,
and oracle 1,011 expected / one unexpected / 47,465 not-run, SHA-256
`211d33f744085a2dd91fd13ed7ac3540b5abe619bda8f217a8e2752362ea6d92`.
Node passes 289/289 plus strict TypeScript and native-load smoke. The exact
selected host profile passes 710 tests with 14 explicit dedicated-environment
ignores; strict selected host/addon Clippy, release build, formatting and diff
checks pass. The remaining six candidate failures and every not-run row stay
explicit; neither the Core nor Python corpus is complete.

NODE-364 eliminates every candidate discrepancy in the complete unchanged
`tests/page/expect-boolean.spec.ts` file. Candidate and independent oracle each
pass 86/86 on the pinned Playwright v1.58.0 source at
`961381ec73a9fa2661d92504eab1a6bd6eaffc75`; both lanes use the guarded
loopback-only dedicated Chrome CDP endpoint and no assertion or upstream test
source is changed. Candidate/oracle report SHA-256 values are
`afe0d9c314f2023dd661ded91c3c5e079f00e2057a84ea7b6bb8352a1a841e89`
and `ce8dddd9243b84a13d32ce46eb6ef805e146e1d05263a418a6615175e5e2d771`.

The repair keeps matcher timing, selector diagnostics, response logs and
FrameLocator identity in their existing owners. Rust gives the first plain
Locator state probe a bounded budget independent of a 1 ms public timeout,
reports the unmatched CSS closing token, and retries a FrameLocator after the
old child execution context retires. `APIResponse._fetchLog()` delegates to
the bounded Rust response log. The Node addon exposes the existing
Rust-owned lazy `HostFrameLocator`; the JS facade no longer resolves and pins
one child Frame before starting a matcher. A real replacement repro and the
official `with frameLocator 2` row both pass.

The canonical Core ledgers advance to candidate 1,335 expected / one skipped /
seven unexpected / 47,134 not-run, SHA-256
`9ba07010bec8caae7a8a7129ae95a3b7c07684f10a03287b0572d8ca9c8b3eb5`,
and oracle 994 expected / 47,483 not-run, SHA-256
`94c37b2a46e9e86a9d1706ebe9b4d3e7397026abe4ab15d5d17b2a5accb4c705`.
Node passes 286/286 plus strict TypeScript; the selected host profile passes
709 tests with 14 explicit dedicated-environment ignores; formatting, strict
host/addon Clippy, release build and native-load gates pass. This is one
complete official file, not the complete Core or Python corpora.

NODE-363 introduces the first browser-backed official Context-proxy selection.
The fixed manifest pins `chromium-library` and selects the unchanged
`tests/library/fetch-proxy.spec.ts:64` Context-level proxy contract plus the
`:82` bypass contract. Candidate and independent oracle each pass 2/2 against
the pinned clean Playwright v1.58.0 checkout at
`961381ec73a9fa2661d92504eab1a6bd6eaffc75`. Both lanes use the dedicated
loopback-only Chrome CDP process; candidate stderr confirms that the fixture
loads the local Pwright package and adapts only `browserType.launch` to that
endpoint. No assertion or test source is changed.

Candidate/oracle report SHA-256 values are
`224228d64055330d5b042456070532fc0d90751e7e9731e3783297c55a8bdcb7`
and `a5e7e6ff1b2ab5caeb3f01558e0efe669f3692681f9509d3610032deba590ed3`.
The fixed manifest SHA-256 is
`7f87753ddc704d11eb519aaaec5b3e1eb3d4577c06fb1dc10f1e21917d5dcb76`.
Canonical Core ledgers advance to candidate 1,311 expected / one skipped / 31
unexpected / 47,134 not-run, SHA-256
`3fb4d1fef2ffe6863d192eef139e185d16c9efaea25e952f8f2e6a782170d7a4`,
and oracle 994 expected / 47,483 not-run, SHA-256
`83509103cc80e4a16b83a1dd87d0ab9dc9889ea2c383228325d5b52ae753f6c4`.
These two new outcomes do not imply full Context proxy parity: authenticated
browser page traffic, remaining proxy cases, other engines and all other
not-run invocations stay explicit.

NODE-362 runs the two previously unrun global-request client-certificate rows
through both proxy kinds. The unchanged HTTP CONNECT+mTLS and SOCKS5+mTLS rows
each pass 3/3 candidate and oracle in Library and 6/6 candidate and oracle in
Bidi: 18/18 new outcomes in each lane. Candidate logs confirm the local
Pwright adapter; no browser/CDP is used. A common source-only fixture adapter
changes only bind/address hosts for Tailscale and is applied identically to
both lanes; the pinned upstream remains clean at
`961381ec73a9fa2661d92504eab1a6bd6eaffc75`.

Candidate Library HTTP/SOCKS and Bidi HTTP/SOCKS report hashes are
`8c87099c9acd9e59cd94bb040aa4b4043018e64b611d1cc426c89c420d5b6331`,
`78a650374205f5bbe74876fe627459570d761b4f4951274385318b02831a9a2e`,
`0c956deb3e2f5a6bf2ff04955005ea8e42a38d299b807dc3c65a2121b019b8b3`
and `9500fe2f5a84a55309430c922fa79e02d89f5dae94b23c317aa08f057b32b168`.
Oracle hashes are
`d7506ebd85d61a3941d634fff2faf2c16bae2e84a1926d71b58cf6afdcdf681e`,
`60141488c43eae248e157ebf916f7f6c6d118056130c7a650863e631a5b5979c`,
`a313807bed3dc7a43d4b37b2821743711c04c9e1472ed92a2c9396307a66d5da`
and `d14fc8e00ed7edd1523aeb9904c28f8e2842e6f9539dc2702314cd6339e4ad7e`.
The browserless manifest hash is
`288f9bdddb43bb6961f30ed7e6799c233315bc5733ce18535775061b92d6c434`.
Canonical Core ledgers advance to candidate 1,309 expected / one skipped / 31
unexpected / 47,136 not-run, SHA-256
`cc0a0c565820c7716a806d78bfea26b8d9c477b65f12d67bbd6d90ee14355399`,
and oracle 992 expected / 47,485 not-run, SHA-256
`a28e1e3e553e05292860d374ceb368336f7a33da4bbe0ff826e6aa7cb877c6a1`.
These are selected official outcomes; every remaining invocation stays
explicitly `not-run` or retains its prior recorded outcome.

NODE-361 runs the eight already-supported global-request client-certificate
rows through all six official Bidi projects. The unchanged input-validation
row passes 6/6 and the seven direct-request rows pass 42/42 in both candidate
and independent oracle. The candidate artifact is rebuilt from current source
and load-verified first; every candidate project confirms the local Pwright
adapter, and no browser/CDP is used. Candidate validation/request report hashes
are
`709d386639f3d22ca09cf643da3be960f5479989dede78da32b3ad0ca7d8681b`
and `1aaad17f43aeaaa0266e89ec4d6fc3eeaf1eb064ec4b9bfc51b0f8f4a1c75af7`;
oracle hashes are
`64d8d2325899c2513d2079a3f894f88e321d1c8fd6c8161bf41c560c9483c231`
and `80c69567642c724d1078c2d949ef4a185a03494ffd46d2ba9c1e11fdbbe1742d`.
The browserless manifest SHA-256 is
`a1e82102dcc296a714b39abae240c22078c1a2f5247dba08f147099a5ff8f559`.
The canonical Core ledgers advance to candidate 1,291 expected / one skipped /
31 unexpected / 47,154 not-run, SHA-256
`45e50cac8c73c0fb08a84e85200a16599d08e8549e438f1e91ad1883fca454a6`,
and oracle 974 expected / 47,503 not-run, SHA-256
`cbc14a9f83502e1495fa752f5152f667d607263f11dd0615520051b4e870ac9b`.
The two client-certificate-through-proxy rows remain explicit `not-run` until
HTTPS target tunneling is implemented.

NODE-360 runs the same three unchanged global-request proxy rows through all
six official Bidi projects. Authenticated HTTP CONNECT, HTTPS proxy-server ALPN
and SOCKS5 each pass 6/6 candidate and 6/6 independent oracle, for 18/18 in
each lane. The candidate lane confirms the local Pwright adapter for every
invocation; neither lane launches or attaches to a browser. Candidate report
SHA-256 values are
`62dad11b718eb7862072ae21856247363983e1bb272ec44d97dcbc6fb3d3350a`,
`80f51f86472309ce8bc785eeb90279b84e73689d06755e269eae6afdc7b734b1`
and `de2cc43b6a23d4d453cd6e0658bf09362027860ce941e2e47b91f4e811ecbddf`;
oracle report hashes are
`ee9367190de82fbc0f2e28be01894feb2a0454c16af4ea46908875d12d4fe934`,
`7ad6386ef1e37d81634a0f88cd76969a8daabc163e8951fb427aff90a591d2a0`
and `87d6d8ff090abd7e2182e690a9d6f3b6f5442a0d6a0e63fb7b7c850f214b4f18`.
The browserless manifest SHA-256 is
`eb83e9b2ac407290b477cdd09664f896d2235974813d357d99cfc354e3a12744`.
The canonical Core ledgers advance to candidate 1,243 expected / one skipped /
31 unexpected / 47,202 not-run, SHA-256
`139a38098b6f813f7b5d5c5609edadb91217dd605698cc68b3494a4e1e29c34f`,
and oracle 926 expected / 47,551 not-run, SHA-256
`478bd8a36d8f9791f4d27fee90b99649cc1cd166947a825930ba72ed72ce7182`.

NODE-359 adds a separate `api-request-socks5-proxy` profile over a bounded
Rust SOCKS5 connector. The unchanged official
`tests/library/fetch-proxy.spec.ts:133` row passes 3/3 candidate and 3/3 oracle
across Chromium, WebKit and Firefox Library projects without CDP. Both lanes
use the same source-only fixture rewrite; assertions and the pinned Playwright
checkout at `961381ec73a9fa2661d92504eab1a6bd6eaffc75` remain unchanged.
Candidate report SHA-256 is
`19af5d94d8ccc7196d8200f229b7eafec4f0954c687dddf9c9309ef5d2428438`;
oracle report SHA-256 is
`7926ab68bdb6f9614781b8365b047d59176203adb913d5fa168a98edcbfb87e7`.
The fixed browserless manifest SHA-256 is
`e014123d1cd614864880e448d79a2b409f163295b0cf47185bd76b1e5be0a7aa`.
The canonical Core ledgers advance to candidate 1,225 expected / one skipped /
31 unexpected / 47,220 not-run, SHA-256
`8909256aa8074921a1782515809b57883b9dc2405991f0d5c6a4b37412632690`,
and oracle 908 expected / 47,569 not-run, SHA-256
`c1814cf9894b0e5b39d201c7e12962b7eb9b89f4f7d17cdcf382c1d5abe50e25`.
HTTPS target tunneling, authenticated SOCKS, bypass and the complete remaining
corpus are open.

NODE-358 extends the same optional HTTP CONNECT leaf to accept an `https:`
proxy server. The injected Rustls connector owns outer TLS, carries the
request's certificate-ignore policy and offers HTTP/1.1 ALPN; the target origin
remains plain HTTP. The unchanged official HTTPS-proxy ALPN row passes 3/3
candidate and 3/3 oracle across Chromium, WebKit and Firefox Library projects.
Candidate report SHA-256 is
`6dba12195eed6bf3f7ba71b3bc2713d97acf83449db4ed28e350aef5b821d74d`;
oracle report SHA-256 is
`429f9fa3ffec495127cc0f7310815d6eca336a488b74d197f74f97f11a5621c8`.
The fixed browserless manifest SHA-256 is
`fddf3bc2a25b84c2926baf0378822634052304ab20f471074b0fe1c5b4581bc1`.
The canonical Core ledgers advance to candidate 1,222 expected / one skipped /
31 unexpected / 47,223 not-run, SHA-256
`9d7b1785ed238f69fbf2931d604aa5bd25e19d0076ecbd9e2a1234f61ceed927`,
and oracle 905 expected / 47,572 not-run, SHA-256
`041648606bf73c419e2d40dd967983063616593bf81d1969756f912dbd873afc`.
HTTPS target tunneling, SOCKS, bypass and the complete remaining corpus are
open.

NODE-357 selects an independent `api-request-http-proxy` Node profile over a
new bounded Rust HTTP CONNECT connector. The unchanged official
`fetch-proxy.spec.ts` global-request credential row passes 3/3 candidate and
3/3 oracle across Chromium, WebKit and Firefox Library projects. Both lanes
use the same source-only infrastructure adapter so the official proxy and
destination servers bind to Tailscale and address each other through full
MagicDNS; assertions and the pinned checkout remain unchanged. Candidate
report SHA-256 is
`6d03f9eaeba8e6c18ad8ceb91c36a76a983e20760013bc451629706245ffeb68`;
oracle report SHA-256 is
`d8349d4e448041af91cb34403cab026b3428f3e5989792a759d2dd2414263c2f`.
The canonical Core ledgers advance to candidate 1,219 expected / one skipped /
31 unexpected / 47,226 not-run, SHA-256
`29c364c5da543b6a26da51ac48a3f9fe0d1481b1273871af3d12fbb59fde075a`,
and oracle 902 expected / 47,575 not-run, SHA-256
`a6170d5449f0024d04e9a51fe835f40de1f3e809040a68e43553b7d916cc299d`.
HTTPS-over-proxy, SOCKS, bypass and the complete remaining corpus are open.

NODE-356 keeps Python at ABI 96 and fixes the complete async/sync Page and
BrowserContext request-fallback files. A fixed source-pinned manifest covers
55 Chromium rows. Source candidate and independent Playwright 1.58.0 oracle
both pass 55/55 (26 async, 29 sync), with no assertion weakening or excluded
row. The provisional cumulative Python ledger is 1,070 passed, zero failed,
seven skipped and 5,449 not run, SHA-256
`14f2e04ad35fbfaff95109c326ff6064d955d737f192bee21eea60c062623256`.
Installed-wheel units pass 207/207 and the two release-wheel payloads
reproduce after normalizing CycloneDX identity. Final installed-wheel browser
promotion remains pending because the shared dedicated Chrome CDP endpoint
became unresponsive during another concurrent task and was not restarted.

NODE-355 advances Python to ABI 96 with Rust-owned BrowserContext/Page API
request and route-fetch composition. Two fixed source manifests expand to 108
Chromium rows. Final installed candidate and independent official Playwright
1.58.0 oracle both pass the exact same 51 async Context, 47 sync Context, 6
async route and 4 sync route rows. All 108 were `not-run` in NODE-353's
authoritative base and now move to `passed`; no existing outcome regresses.
The cumulative Python ledger is 1,015 passed, zero failed, seven skipped and
5,504 not run, SHA-256
`a0c33f08557de51ee7665720a393469fe3538c9c8f7aa421cc3f5b684e57e178`.
Full Python and Core JS/TS execution remain open.

NODE-354 advances Python to ABI 95 with an opt-in Rust-owned service-worker
policy layered only on Context init scripts. The final installed candidate
passes all four pinned async/sync Chromium service-worker-policy rows under the
secure fixture, plus isolated async 20/20 and sync 20/20 stress runs. The
official Playwright 1.58.0 oracle cannot initialize its CDP default Context on
Chrome 154: `Browser.setDownloadBehavior` is rejected before any selected
assertion executes. Those four setup errors are not product failures or
passes. All four rows remain `not-run`, so the Python ledger stays at 907
passed, zero failed, seven skipped and 5,612 not run. Complete Python and Core
JS/TS execution remain open.

NODE-353 keeps Python ABI 94 and fixes three Rust-owned lifecycle boundaries:
paused-popup initialization may publish provisional Page/popup/Dialog identity
before world creation resumes; immediate-close console handles finish at the
observer boundary before native detach compensation; and host teardown
revokes retained Dialog actions before transport shutdown. Final installed
candidate and independent official Playwright 1.58.0 on Chrome 154 agree at
11 passed / 1 failed over the fixed 12-row async Chromium BrowserContext-event
file. The shared failing JavaScript-URL popup row expects no Dialog Page, but
both lanes expose the popup Page. That source/oracle-drift row stays `not-run`;
the other 11 move to `passed`. The Python ledger is now 907 passed, zero
failed, seven skipped and 5,612 not run, SHA-256
`4a6c94e0a40e533243c0c19419685fdc30cb5f4fdbc9cde593917695879fc962`.

NODE-352 changes no runtime code. Final installed candidate and independent
official Playwright agree at 5 passed / 3 failed over the fixed eight-row
async Chromium BrowserContext cookie-read file. Each shared failure hardcodes
the expected domain as `localhost`, while the unchanged test executes against
the required full MagicDNS fixture hostname. The five environment-applicable
rows pass 5/5 in both lanes. They move from `not-run` to `passed`; the three
MagicDNS-inapplicable rows stay `not-run`. The Python ledger is now 896 passed,
zero failed, seven skipped and 5,623 not run, SHA-256
`330786f82e6d9682e744904c6a3a985b1a25f3f9711db1dd851636fb139dc3b4`.

NODE-351 changes no runtime code. A fixed source-pinned manifest covers all
seven async Chromium BrowserContext clear-cookie rows, including isolation and
name, regular-expression, domain and path filters. Final installed candidate
and independent official Playwright both pass 7/7. The cumulative Python
ledger is now 891 passed, zero failed, seven skipped and 5,628 not run,
SHA-256
`fd8aa762af6f316feb60d2f9a777ba5432c02f4256b01baf0f5085dff7b962c1`.
Full final-wheel collection remains 6,526/6,526; complete Python and Core
JS/TS execution remain open.

NODE-350 advances Python to ABI 94 by making Rust-classified invalid arguments
inherit the same public `Error` caught by official Playwright tests. This does
not move validation into Python: native error codes and messages remain Rust-
owned, while pure Python signature/type checks continue to raise `ValueError`.
The 19-row async Chromium BrowserContext add-cookie file moves from candidate
15 passed / 4 failed to 17 passed / 2 failed, exactly matching the independent
oracle. The two shared failures depend on an upstream `localhost` path/domain
assumption that is false for the required MagicDNS fixture. Candidate and
oracle pass the other 17 rows 17/17. Only those rows enter the ledger, which is
now 884 passed, zero failed, seven skipped and 5,635 not run, SHA-256
`193d0cfe5e6674effc3bd876ec3fc8299b7e0256986554a3785f66f9f4c0850d`.
Full final-wheel collection remains 6,526/6,526.

NODE-349 keeps Python at ABI 93 and distinguishes synchronized explicit Page
sessions from topology-discovered Page sessions. When browser-root auto-attach
reports a target already owned by an explicit `Target.attachToTarget` session,
the later duplicate is detached before it can install a second Fetch observer.
The fixed nine-row BrowserContext-route manifest produces the same 8 passed /
1 failed result under final installed candidate and independent official
Playwright. The only failure is in both reports: the Tailscale fixture correctly
returns the MagicDNS cookie domain while the upstream assertion hardcodes
`localhost`. A clean environment-applicable selection passes candidate/oracle
8/8. Those eight rows move from `not-run` to `passed`; the ninth remains
`not-run`. The cumulative ledger is 867 passed, zero failed, seven skipped and
5,652 not run, SHA-256
`2b6b9e6dc39344b6a86b334473b3860dd0207c49686a6bcd610e9a06b1bb8b3f`.
Final-wheel collection remains 6,526/6,526.

NODE-348 changes no runtime code. It pins the five async Chromium Page-route
rows that had remained outside the authoritative ledger and runs each in its
own process. Final installed candidate and independent oracle both pass 5/5.
Together with the earlier non-overlapping fixed manifests, all 44 async
Chromium Page-route rows now have passed evidence. The cumulative ledger is
859 passed, zero failed, seven skipped and 5,660 not run, SHA-256
`2f75e41c034568da65f7e0cc10fbac364a54f5778dc9ccc6ea5c875778435194`.
The earlier interrupted whole-file diagnostic remains excluded.

NODE-347 keeps Python at ABI 93 and narrows Chromium recursive auto-attach to
the target classes selected by the composition. Page/OOPIF discovery attaches
to `page` and `iframe`; selecting a worker-retaining observer also adds
`worker`. Every CDP filter ends with a catch-all exclusion, so an unselected
service worker remains Chromium-owned and can intercept before Page routing.
The unchanged official service-worker-precedence row passes the final
installed candidate 1/1 and final-wheel collection remains 6,526/6,526. That
row moves from `not-run` to `passed`, producing 854 passed, zero failed, seven
skipped and 5,665 not run, SHA-256
`5bc1fbb17929fb0dab2845f3041317382b3b7cab906d7d024198228b9d51b850`.
No oracle claim is made: a stale service-worker target invalidated the oracle
environment, and the later complete Page-route diagnostic lost CDP
responsiveness after 24 passing isolated cases.

NODE-346 advances Python to ABI 93. Its separately selectable
`route-redirects` feature is exactly `page-routes` plus `request-history`:
Rust auto-continues redirect pauses without duplicate callbacks or route-budget
consumption, retains at most 64 explicit header overrides, strips `Cookie`
before replay, and retires chain state on terminal events. Exact native
request-history identity supplies thin Python `redirected_from` /
`redirected_to` facades. The two unchanged redirect rows move from exact
candidate 0/2 to final installed candidate and independent oracle 2/2. The
complete async Chromium Page-route file improves to 43 passed / 1 failed and
final-wheel collection remains 6,526/6,526. Both rows move from `not-run` to
`passed`, so the ledger is 853 passed, zero failed, seven skipped and 5,666
not run, SHA-256
`f4dca295f673c6b680020e02c4f99d8a7a8cd2dd261696dcdf7638a32ce9a362`.
Remaining Python and complete Core JS/TS execution stay open.

NODE-345 advances Python to ABI 92. A separately selectable Rust adapter
composes the retained API-request response with the existing Page-route owner,
inherits status/headers/content type and enforces the existing 1 MiB body
bound before cloning. Python passes native identity rather than response body
bytes. The unchanged global-fetch row moves from exact candidate 0/1 to final
installed candidate and independent oracle 1/1. The complete async Chromium
Page-route file improves to 41 passed / 3 failed and final-wheel collection
remains 6,526/6,526. The row moves from `not-run` to `passed`, so the ledger is
851 passed, zero failed, seven skipped and 5,668 not run, SHA-256
`09089b9c798ccf3a4e3f93645839257783b16a39c03b8c737ade9f34b0d40d48`.
The three Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-344 keeps Python at ABI 91. Matching browser-generated CORS preflights
are recognized and fulfilled with Playwright's 204 response inside the
bounded Rust route owner, without publishing a Python callback or consuming
the user route's finite-use budget. The two unchanged POST/method rows move
from exact candidate 0/2 to final installed candidate and independent oracle
2/2. The complete async Chromium Page-route file improves to 40 passed / 4
failed and final-wheel collection remains 6,526/6,526. Both rows move from
`not-run` to `passed`, so the ledger is 850 passed, zero failed, seven skipped
and 5,669 not run, SHA-256
`5a0465a5fdeef895cae08b30b2babca1b794b28474a5cf32118962e1556ab0d4`.
The APIResponse fulfillment discrepancy is handed to NODE-345. The other
three Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-343 advances Python to ABI 91. Routed `Request.response()` uses
Chromium's exact bounded native network identity to wait in the existing Rust
request-history owner; it does not match URLs or retain a binding-owned
registry. The unchanged encoded-URL row moves from an ABI 90 candidate
failure to final installed candidate and independent oracle 1/1. The complete
async Chromium Page-route file improves to 38 passed / 6 failed and
final-wheel collection remains 6,526/6,526. The row moves from `not-run` to
`passed`, so the ledger is 848 passed, zero failed, seven skipped and 5,671
not run, SHA-256
`58c32ca19ac776657dcfd4e823ca58d9d57acf519b5c47d371f3c2deb12520d3`.
The six Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-342 advances Python to ABI 90. Thin binding matchers retain Python regex,
predicate and handler identity but delegate nonmatches to the bounded
Rust-owned route stack; Rust therefore remains authoritative for fallback,
settlement and native retirement. The opt-in `route-fulfill-path` leaf owns
bounded file validation, async reads and MIME selection in Rust. Final
installed candidate and independent oracle pass the two-row manifest 2/2.
The complete async Chromium Page-route file improves to 37 passed / 7 failed
and final-wheel collection remains 6,526/6,526. Both rows move from `not-run`
to `passed`, so the ledger is 847 passed, zero failed, seven skipped and 5,672
not run, SHA-256
`e423606ad57b9f2e0bdb95f4e4a74e71d65ea9582890b864e86ba76837817028`.
The seven Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-340 advances Python to ABI 89 with exact Request Frame identity through
bounded Rust route and network-event owners. Python reuses the Page Frame
registry for `Request.frame`, `Response.frame` and
`ElementHandle.content_frame()` rather than creating a parallel owner. Final
installed candidate and independent oracle pass the six-row request-frame
manifest 6/6 and the route-frame row 1/1. The complete async Chromium
Page-route file improves to 35 passed / 9 failed and fresh full collection is
6,526/6,526. Five request-frame rows and the route row move from `not-run` to
`passed`; the popup-first row already passed and receives replacement
evidence. The ledger is 845 passed, zero failed, seven skipped and 5,674 not
run, SHA-256
`673973f03e77d0291596cfbc971579c70d98831f634ab74606a43b37282375c7`.
The nine Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-339 advances Python to ABI 87 with the complete Page request lifecycle
listener trio over one bounded Rust event owner. The two unchanged abort rows
pass final installed candidate and independent oracle 2/2, plus candidate
repeat 10/10. The complete async Chromium Page-route file improves to 34
passed / 10 failed, and fresh full collection remains 6,526/6,526. Both rows
move from `not-run` to `passed`, so the ledger is 839 passed, zero failed,
seven skipped and 5,680 not run, SHA-256
`b8c7bebbeb3f2c16880fb7fc08dd30b15d56d504d091c264b732641ef8e81e0f`.
The 10 Page-route discrepancies, remaining Python and complete Core JS/TS
execution stay open.

NODE-338 keeps Python ABI 86 and makes the Rust route owner serialize
Playwright's pinned status reason into Chromium `Fetch.fulfillRequest`. The
unchanged 422 row moves from exact candidate failure to final installed
candidate and independent oracle 1/1; five candidate repeats pass 5/5. The
complete isolated Page-route file is 32 passed / 12 failed with no regression.
The ledger is 837 passed, zero failed, seven skipped and 5,682 not run, SHA-256
`8a6d6a9169086e482fbd8acf6d81162ef42ab209f18c2c174238e5feb85e2ef5`.
The remaining Python and complete Core JS/TS execution stay open.

NODE-337 keeps Python ABI 86 and changes no runtime code. Two additional
unchanged Page-route rows verify that native `Referer` and `Origin` header
names use Playwright's lowercase public shape. The exact ABI 84 selection
fails 2/2; final installed candidate and independent oracle pass 2/2, and five
candidate repeats pass 10/10. The full isolated async Chromium Page-route file
records 31 passed / 13 failed, while two known unstable rows that happened to
pass remain unpromoted. The ledger is 836 passed, zero failed, seven skipped
and 5,683 not run, SHA-256
`eb6b687f25461fa8d8f4182455215dbcf565a47296427680357866df79cc8553`.
Remaining Python and complete Core JS/TS execution stay explicit work.

NODE-336 advances Python to ABI 86 with thin async/sync Page and
BrowserContext extra-header facades over the bounded Rust header owner. Rust
also composes an implicit Page referer into Fetch route snapshots when Chrome
omits it and lowercases CDP header names at the public boundary. The exact ABI
84 baseline fails 3/3; the unchanged official Chromium manifest passes final
installed candidate and independent oracle 3/3, and five candidate repeats
pass 15/15. Full collection remains 6,526/6,526. All three selected Chromium
rows move from `not-run` to `passed`, so the ledger is 834 passed, zero failed,
seven skipped and 5,685 not run, SHA-256
`a0c1caa9d38619302ae8b0eaa49450d1c9db9a5db80cb44e4fa6fb5e69c4b783`.
Remaining Python and complete Core JS/TS execution stay explicit work.

NODE-335 advances Python to ABI 85 with Rust-owned composable
`Route.fallback()` and handler-specific Page/BrowserContext `unroute()`.
The exact ABI 84 baseline contains seven failures and one existing pass; the
unchanged official Chromium manifest passes final installed candidate and
independent oracle 8/8, and five candidate repeats pass 40/40. Full collection
remains 6,526/6,526. All eight selected rows move from `not-run` to `passed`,
so the ledger is 831 passed, zero failed, seven skipped and 5,688 not run,
SHA-256
`4303259b2a6813aa7e3fc8017d89d1cc5d62a38bed6b66eacc3af5600d576722`.
Regex/predicate filters, `unroute_all(behavior="ignoreErrors")`, remaining
Python and complete Core JS/TS coverage remain open.

NODE-334 advances Python to ABI 84 with atomic Rust-owned finite route-use
budgets for Page and BrowserContext. Three unchanged async Chromium rows move
from ABI 83 candidate 0/3 to final installed candidate and independent oracle
3/3; five additional candidate runs pass 15/15. Full collection remains
6,526/6,526. All three Chromium rows move from `not-run` to `passed`, so the
ledger is 823 passed, zero failed, seven skipped and 5,696 not run, SHA-256
`26023955d6cd01d96099cf7b07ef479d028e86a9fd60038e14c786479f467439`.
At that checkpoint, fallback and handler-specific unroute were still open.

NODE-333 keeps Python ABI 83 and makes question marks literal in both Rust
route matching and the compiled Chromium Fetch pattern. The unchanged official
row moves from candidate failure to installed candidate/oracle 1/1; candidate
repeat passes 5/5. The ledger is 820 passed, zero failed, seven skipped and
5,699 not run, SHA-256
`aaa91984391a096225cae1684c0f1879249b41feedd8efad7a5203053087e6d0`.
Full collection remains 6,526/6,526. The final 44-case Page-route diagnostic
records 22 passed / 22 failed; two apparent losses reproduce on the unchanged
NODE332 wheel and remain separately tracked instability, not exclusions.

NODE-332 advances Python to ABI 83 with bounded Rust-owned URL, method, header
and POST-data overrides for `Route.continue_()`. The unchanged ten-case async
Chromium file moves from 2 passed / 8 failed on ABI 82 to installed candidate
and independent oracle 10/10. All ten rows move from `not-run` to `passed`, so
the ledger is 819 passed, zero failed, seven skipped and 5,700 not run,
SHA-256
`43861212714734aa05343ef0ff48a87977136adbaecfe9d0c3c461bfb50af2f6`.
Full collection remains 6,526/6,526. The complete 44-case Page-route file now
records 23 passed / 21 failed / 0 errors; remaining discrepancies, the rest of
Python and the complete Core JS/TS corpus remain open.

NODE-331 executes the complete unchanged 44-case async Page-route Chromium
file with per-case isolation. The installed candidate records 18 passed, 25
failed and one error. A fixed manifest for the 18 green rows passes candidate
and oracle 18/18, advancing each from `not-run` to `passed`. The ledger is 809
passed, zero failed, seven skipped and 5,710 not run, SHA-256
`0408aef3e6fa958f60ce74998e4326e40aef2bc906851efa35e9b656e9b5940c`.
All 26 remaining discrepancies stay visible for implementation.

NODE-330 keeps Python ABI 82 and promotes six additional unchanged public
unroute-all rows. Installed candidate and independent oracle pass async/sync
default removal 4/4 plus in-flight wait/fulfill 2/2. All six advance from
`not-run` to `passed`, so the ledger is 791 passed, zero failed, seven skipped
and 5,728 not run, SHA-256
`d7255f3d41c76a26528036c0eaee54b788a48b2323650b3700f2da7441f7cb05`,
with no losses and no implementation change.

NODE-329 advances Python to ABI 82 with Rust-owned scoped
`unroute_all(behavior="wait")`. Two unchanged async Page/Context HAR-abort rows
pass final installed candidate and independent oracle 2/2; candidate repeat
stress passes 20/20. Full collection remains 6,526/6,526. Both rows advance
from `not-run` to `passed`, so the Python ledger is 785 passed, zero failed,
seven skipped and 5,734 not run, SHA-256
`b58f55b19a62286fdf0c1b06040c017c1d9b00666b3e139847eaf8cc42027b6f`,
with no losses. Handler-specific unroute, `ignoreErrors`, regex/predicate
filters, remaining Python and complete Core JS/TS coverage remain open.

NODE-328 advances Python to ABI 81 with Rust-owned ordinary Context routes.
Four unchanged async/sync Context HAR URL-filter/ordinary-route composition
rows pass final installed candidate and independent oracle 4/4; candidate
stress passes async 20/20 and sync 20/20. Full collection remains 6,526/6,526.
All four rows advance from `not-run` to `passed`, so the Python ledger is 783
passed, zero failed, seven skipped and 5,736 not run, SHA-256
`c0c16280690a496ffdc374fa77ad1792aedd6670a9c61026d70fc042bce77b17`,
with no losses. Regex/predicate route filters, external/ZIP/update HAR,
remaining Python and complete Core JS/TS coverage remain open.

NODE-327 keeps Python ABI 80 and aligns missing-entries HAR parsing with the
official fallback behavior. Two unchanged async/sync Chromium rows pass final
installed candidate and independent oracle 2/2; candidate stress passes async
10/10 and sync 10/10. Full collection remains 6,526/6,526. Both rows advance
from `not-run` to `passed`, so the Python ledger is 779 passed, zero failed,
seven skipped and 5,740 not run, SHA-256
`a545a96742ef986a132be551627450e08c8d1fd0ecbfca8a1fe8e5ab142328c8`,
with no losses.

NODE-326 advances Python to ABI 80 with Rust-owned Context HAR scope and a
bounded future-Page/OOPIF initializer. Six unchanged async/sync
method/redirect, fallback-continue and default-abort rows pass final installed
candidate and independent oracle 6/6; candidate stress passes async 30/30 and
sync 30/30. Full collection remains 6,526/6,526. All six rows advance from
`not-run` to `passed`, so the Python ledger is 777 passed, zero failed, seven
skipped and 5,742 not run, SHA-256
`c52d8ae2a549fc2cfa75e745969863363c8e10705daea947e46a9446d58b89a7`,
with no losses. Broader HAR, remaining Python and complete Core JS/TS coverage
remain open.

NODE-325 keeps Python ABI 79 and promotes the complete unchanged async/sync
Page HAR URL-filter/no-fallback pair. Installed candidate and independent
oracle pass 2/2; API-separated candidate stress passes async 10/10 and sync
10/10. Full collection remains 6,526/6,526. Both rows advance from `not-run`
to `passed`, so the Python ledger is 771 passed, zero failed, seven skipped and
5,748 not run, SHA-256
`fb11fa9a34ed364ec06254672ac32ef54a7a790362eeaa4f6a562887c2d1f03e`,
with no losses. This proves Page HAR URL filtering composes with the next
ordinary Page route; Context routing, external/ZIP/update HAR, remaining
Python and complete Core JS/TS coverage remain open.

NODE-324 keeps Python ABI 79 and promotes the complete unchanged async/sync
Page HAR method/redirect pair. Installed candidate and independent oracle pass
2/2; API-separated candidate stress passes async 10/10 and sync 10/10. Full
collection remains 6,526/6,526. Both rows advance from `not-run` to `passed`,
so the Python ledger is 769 passed, zero failed, seven skipped and 5,750 not
run, SHA-256
`dbc28e3a20089d56d2b937d7175f19e4562db76da59477b789e8c14fc375cefd`,
with no losses. The mixed-API repeated harness diagnostic is excluded; broader
HAR, remaining Python and complete Core JS/TS coverage remain open.

NODE-323 advances Python to ABI 79 with an independent bounded plain-HAR replay
leaf composed over the existing Page-route Fetch driver. The unchanged
persistent-Context Chromium HAR row passes final installed candidate and
independent oracle 1/1; candidate same-process stress passes 10/10. Full
collection remains 6,526/6,526. The row advances from `not-run` to `passed`, so
the Python ledger is 767 passed, zero failed, seven skipped and 5,752 not run,
SHA-256
`e390e350e8995a48569133ed5be757cd9040a6d1592ab1b86f0ad5ca046476e2`,
with no losses. Broader HAR coverage, the two cookie applicability gaps, the
remaining Python corpus and the complete Core JS/TS corpus remain open.

NODE-322 advances Python to ABI 78 with independent ordinary/persistent
Context HTTP credentials and navigation response-body leaves. Eleven unchanged
Chromium rows pass final installed candidate and independent oracle 11/11;
candidate same-process stress passes 110/110. Full collection remains
6,526/6,526. All eleven rows advance from `not-run` to `passed`, so the Python
ledger is 766 passed, zero failed, seven skipped and 5,753 not run, SHA-256
`294ab4a8fee1ea783d5f5b8043a54c52648fe5e9539fdecbd7c42c6000b9c2fa`,
with no losses. This is not completion of the Python corpus, and the complete
Core JS/TS corpus remains open.

NODE-317 advances Python to ABI 72 with independent owned-Context CSP bypass
and persistent-default composition in one bounded Rust owner. Four unchanged
Chromium rows pass installed candidate and independent oracle 4/4; candidate
stress passes 40/40. Full collection remains 6,526/6,526. The Python ledger
advances to 748 passed, zero failed, seven skipped and 5,771 not run, SHA-256
`713d4047df50050430e4bfec6c843b171806653db19e8c0df6da6e66cd942508`.
Eight rows remain from the original default-BrowserContext diagnostic,
including two MagicDNS/`localhost` applicability gaps. A separate iframe CSP
row remains unclaimed behind `ElementHandle.content_frame`. This is not
completion of the Python corpus, and the complete Core JS/TS corpus remains
open.

NODE-316 advances Python to ABI 71 by composing persistent launch with the
existing Rust-owned header leaf. The unchanged extra-HTTP-header Chromium row
passes installed candidate and independent oracle 1/1; candidate stress passes
10/10. Full collection remains 6,526/6,526. The Python ledger advances to 744
passed, zero failed, seven skipped and 5,775 not run, SHA-256
`19d1e6e8365cf3c56b86e96edc76d10a6a619b797f967caedb477cacfbd69411`.
Nine rows remain from the original default-BrowserContext diagnostic,
including the two MagicDNS/`localhost` applicability gaps. This is not
completion of the Python corpus, and the complete Core JS/TS corpus remains
open.

NODE-315 advances Python to ABI 70 by composing persistent launch with the
existing Rust-owned media leaf. The unchanged color-scheme, contrast and
reduced-motion Chromium rows pass installed candidate and independent oracle
3/3; candidate stress passes 30/30. Full collection remains 6,526/6,526. The
Python ledger advances to 743 passed, zero failed, seven skipped and 5,776 not
run, SHA-256
`8a7fe5975b0bbbf792cc8c13a0b8e81a11a70bb8f17dcb0e8f439abe30e67efb`.
Ten rows remain from the original default-BrowserContext diagnostic, including
the two MagicDNS/`localhost` applicability gaps. This is not completion of the
Python corpus, and the complete Core JS/TS corpus remains open.

NODE-314 keeps Python ABI 69 and adds bounded pre-launch validation for the
public persistent-Context `args` option. The unchanged Chromium row passes
candidate/oracle 1/1 and candidate stress 10/10 without issuing a browser
command. The Python ledger advances to 740 passed, zero failed, seven skipped
and 5,779 not run, SHA-256
`b4037c98e0b0d8ae979df04a7c8f033cfb3da4514f8693f25ced9bda469de5ca`.
Fresh collection remains 6,526/6,526. Arbitrary Chromium switches remain
explicitly unsupported rather than being counted as compatibility.

NODE-313 advances the Python binding to ABI 69 with an independently
selectable Rust-owned BrowserContext cookie leaf. The fixed four-row Chromium
manifest has exact candidate/oracle parity: two fixture-independent rows pass
and two rows expose the same mandatory-MagicDNS versus hardcoded-`localhost`
fixture mismatch. Only the two applicable passes enter the 6,526-row ledger,
which is now 739 passed, zero failed, seven skipped and 5,780 not run, SHA-256
`dbd8fc8f8e5c3e10121da0421fdb7f0b16bd4c68176615875c9f0eeaba051638`.
Fresh candidate collection remains 6,526/6,526. This advances one bounded
Python capability family and is not a full Python or Core compatibility claim.

NODE-292 selects the shared Rust `element-handle-dom` owner in Node and reruns
the unchanged Chromium query-selector, eval-on-selector and inner-text
selections. Candidate and independent oracle each pass 15/15. Rust owns
root-relative selector compilation, first-match identity/publication,
no-match release and `HTMLElement.innerText`; Node only constructs wrappers and
delegates. `Page.innerText` and `Frame.innerText` reuse the Rust Locator owner.
`ElementHandle.textContent`, `$$` and `$$eval` remain explicit extraction work,
so the focused result is not a complete ElementHandle or corpus claim.

NODE-293 moves the remaining `ElementHandle.textContent`, `$$` and `$$eval`
semantics into that Rust owner, adds Page/Frame `textContent` facades over the
existing Locator, and aligns scoped XPath prefixing and Element-only filtering
with upstream. Source/package gates pass, but no official result is added:
the required zero-tab preflight found an unrelated Gemini page in the shared
dedicated Chrome. The unchanged 7 + 6 + 4 candidate/oracle rows and ledger
merge remain pending; NODE-292 counts are still current.

NODE-294 advances Python to ABI 57 and projects the existing bounded Rust
console observer through the ordered Page lifecycle fence. Async/sync Python
only construct public wrappers and dispatch callbacks. Type, text, location
and representation are source-complete; real `ConsoleMessage.args` remains
open because its official values require retained Rust-owned argument Handles,
not the existing diagnostic JSON snapshot. Release build, strict Clippy,
dependency and Python static/unit gates pass. The official sync console rows
have not been rerun because the same zero-tab guard still sees the unrelated
Gemini page, so Python candidate/oracle ledgers remain unchanged.

## Current execution checkpoint

Python ABI 57 retains the same browser-independent Rust request owner through
thin async and sync PyO3 facades, the optional Rust client-certificate leaf,
the Page/BrowserContext init-script leaf, and the existing bounded Rust Handle,
wait-for-function, Page-binding, Page-lifecycle-event, Page-error and async
Page-route leaves, including bounded terminal receipts after Page retirement,
bounded `Request.sizes()` settlement and the selected
Rust-owned Page/Locator drag leaf, terminal Page waiter cleanup and the
installed-Chromium persistent-Context launch leaf, headless Page-pause timeout
guard, independently selectable bounded Page timeout wait, bounded terminal
Response completion, explicit Page garbage collection, and
bounded Rust-owned Page-error, console-message and request histories, the
complete immutable device catalog and Context-scoped device emulation, plus
independently selectable bounded console-event publication through the Page
lifecycle fence, plus
bounded Rust-owned Page response publication, collection selector evaluation,
typed `internal:and` intersection parsing and bounded global selector
registration, native selector validation, bounded registered-engine execution
with main/utility-world selection, and independently selectable bounded layout-
selector parsing/geometry evaluation, plus thin async/sync basic Locator
facades over the existing Rust read/action/evaluation owners and Rust-owned
bounded count-expectation polling, double-click, bounded file-input,
element-handle publication, independently selectable highlight and separately
selectable Locator DOM actions. It also composes exact-session
`Page.fileChooserOpened` publication with the existing bounded file-input
payload/path owner, while Python retains only public waiter/callback wrappers.
It selects a bounded Page viewport
registry, exposes the existing Rust Locator geometry owner, and installs the
independently selectable Rust Locator-screenshot composition. Selector
resolution, stability, scrolling, iframe clip translation, capture admission
and output-byte limits stay in Rust. `Locator.evaluate_handle` reuses the
existing bounded Rust Handle owner rather than creating another implementation.
Typed root/chained/nth Locator recipes, semantic `get_by_*` leaves and lazy
FrameLocator owners are now
retained by immutable native owners under the existing Rust query budget;
Python does not construct selector-chain strings or preserve DOM/frame
identity between operations. Rust also owns attribute-expectation polling and
comparison; Python only adapts the assertion result. Native Locator
`evaluate_all` reuses the existing bounded Rust collection-evaluation owner
without reconstructing a selector string in Python.
The complete unchanged
`tests/async/test_fetch_global.py` and `tests/sync/test_fetch_global.py` files
pass 231/231 candidate invocations. Separate upstream-compatible oracle lanes
pass 138/138 async and 93/93 sync; stop-idempotency and pending-request
cancellation pass 6/6 candidate and oracle. The fixed async/sync official
client-certificate selection expands to 12 invocations per lane: eight pass
and four retain the upstream macOS WebKit skips. The latest cumulative
NODE-289 also completes the five applicable Chromium rows in unchanged async
`test_asyncio.py`. NODE-290 then executes all 25 previously unrecorded
Chromium rows in unchanged sync `test_sync.py`. The independent oracle is
24 passed / one failed; the same sync-inside-asyncio assertion is red in the
candidate. The candidate baseline is 9 passed / 16 failed. Exact public repr
facades and publication of the existing Rust request-history association
through `Response.request` repair five rows, producing 14 passed / 11 failed.
Together with two earlier green rows, every one of the file's 27 Chromium rows
now has a recorded outcome. NODE-291 then repairs the two ElementHandle rows
through the optional Rust `element-handle-dom` composition without moving
selector or DOM semantics into Python. The sync file is now 18/27 candidate
versus 26/27 oracle. The latest candidate ledger records 682 passed, nine
failed, seven skipped and 5,828 not run, SHA-256
`a5a1ce1effe030bc97ad9cb9c2405c0f7ee20ed9ba0961f34770f314945501a1`.
The oracle ledger records 671 passed, one failed, seven skipped and 5,847 not
run, SHA-256
`5e0133285e5a74230173ec2df55f001c5424413a3b10f676cf9a2b3973c33a93`.
These totals include complete accounting for both Locator files, the sync
semantic-Locator file, the async semantic-selector file, both fill files and
both context-manager files, the two applicable HTTPS-error rows, all five
applicable sync/async PDF rows, the three Chromium screenshot rows, the async
legacy-text and selector-generator files, and the shared thread-lifecycle,
event-listener and signal-lifecycle files, both Page network-response files,
the async Page network-request file, the sync input file, two applicable async
issue rows, three selector-wait compatibility rows, the applicable Chromium
asyncio rows and every Chromium row in sync `test_sync.py`, not the complete
Python corpus. Nine sync rows remain red and explicit.

NODE-258 executes all 163 Chromium rows in the unchanged async/sync Locator
files. The original candidate passes 60 and fails 103; the independent oracle
passes all 163. NODE-259 through NODE-270 keep every row selected and raise the
candidate result to 145 passed and 18 failed. NODE-271 closes the remaining 16
FrameLocator/content-frame rows plus two focus/blur callback rows through a
lazy Rust-owned frame recipe and the official thin Python handler-arity
adapter. The final result is 86/86 async plus 77/77 sync, matching the 163/163
oracle. NODE-267's first sync attempt is retained as invalid
infrastructure evidence after a browser closure; the clean rerun selects all
77 rows, so setup errors are not recast as candidate failures.
NODE-272 advances to ABI 45 and runs every Chromium row in the complete
unchanged sync `test_locator_get_by.py`. Typed semantic selectors and
attribute-expectation polling remain Rust-owned; candidate and independent
oracle each pass 10/10. The pinned upstream tree contains no async counterpart.
NODE-273 advances to ABI 46 and runs the complete unchanged async
`test_selectors_get_by.py`. Its initial diagnostic exposes the missing native
collection-evaluation export; the final candidate reuses the existing bounded
Rust owner and matches the oracle 3/3.
NODE-274 keeps ABI 46 and runs both complete unchanged fill files plus both
complete unchanged context-manager files. Existing Rust-owned fill passes 5/5.
Thin language context-manager protocols delegate to existing close owners; a
symmetric CDP fixture adapter hides only pre-launch attached Contexts. Candidate
and oracle each pass the remaining 4/4. Initial candidate and oracle adapter
diagnostics remain preserved and excluded from the green ledger.
NODE-275 advances to ABI 47 and runs the complete unchanged async
`test_ignore_https_errors.py` inventory. The two Chromium rows pass in
candidate and oracle. A bounded Rust-owned 16-Context registry installs the
TLS policy on every owned Page/OOPIF session before publication and removes it
on Context close; Python forwards only the strict boolean option. The file's
four Firefox/WebKit expansions remain explicit provider-gap `not-run` rows.
NODE-276 advances to ABI 48 and runs the complete unchanged sync
`test_pdf.py` inventory. The two applicable Chromium rows pass in candidate
and oracle; the four Firefox/WebKit expansions remain explicit upstream
Chromium-only non-applicable `not-run` rows. A separately selectable bounded
Rust owner performs Page/session routing, exact `Page.printToPDF`
`ReturnAsStream` dispatch, a bounded `IO.read` loop, raw/base64 chunk decoding,
mandatory `IO.close` and 2 MiB retained-output accounting. Python only forwards
public Page/path values and rejects uninstalled PDF keyword options.
NODE-277 advances to ABI 49 and runs the complete unchanged async `test_pdf.py`
inventory. Its three applicable Chromium rows pass candidate and oracle; six
Firefox/WebKit expansions remain upstream Chromium-only non-applicable
`not-run` rows. Python strictly validates two newly installed booleans while
Rust maps them to `generateTaggedPDF` and `generateDocumentOutline` in the
same bounded stream owner.
NODE-278 advances to ABI 50 and runs the complete unchanged async
`test_screenshot.py` inventory. Its three Chromium rows pass candidate and
oracle through bounded Rust-owned Page, Locator and exact retained
ElementHandle capture. The source has no Chromium-only marker, so its six
Firefox/WebKit expansions remain explicit provider-gap `not-run` rows.
Attached default/external Context network traffic is no longer eagerly
subscribed into unrelated candidate sessions; borrowed pre-existing Context
callback/history activation remains an explicit open boundary. This checkpoint
still leaves 5,875 Python rows and all 48,477 Core JS/TS invocations without
candidate outcomes.
NODE-279 keeps ABI 50 and runs the complete unchanged async
`test_selectors_text.py` inventory. Its two Chromium rows pass candidate and
oracle through typed Rust-owned immediate legacy-text matching, quote-aware
selector tokenization and bounded Page handle publication. The source has no
Chromium-only marker, so four Firefox/WebKit expansions remain explicit
provider-gap `not-run` rows. This checkpoint leaves 5,873 Python rows and all
48,477 Core JS/TS invocations without candidate outcomes.
NODE-280 keeps ABI 50 and runs the complete unchanged async
`test_selector_generator.py` inventory. Its Chromium row passes candidate and
oracle through bounded Rust-owned test-id configuration and strict-error DOM
previews. Firefox/WebKit remain explicit provider-gap `not-run` rows. This
checkpoint leaves 5,872 Python rows and all 48,477 Core JS/TS invocations
without candidate outcomes.
NODE-281 keeps ABI 50 and runs the complete unchanged shared
`test_threads.py` inventory. Its Chromium row passes candidate and oracle
without product changes, proving that the existing sync lifecycle and Browser
convenience-Page owner work from an upstream-created worker thread.
Firefox/WebKit remain explicit provider-gap `not-run` rows. This checkpoint
leaves 5,871 Python rows and all 48,477 Core JS/TS invocations without
candidate outcomes.
NODE-282 keeps ABI 50 and runs the complete unchanged shared `test_events.py`
inventory. Its initial diagnostic invokes and logs the response-listener
exception but fails to propagate it. A one-error-bounded sync bridge then
returns the first pending listener exception after the public call completes.
The final Chromium row passes candidate and oracle; Firefox/WebKit remain
explicit provider-gap `not-run` rows. This checkpoint leaves 5,870 Python rows
and all 48,477 Core JS/TS invocations without candidate outcomes.
NODE-283 advances to ABI 51 and runs the complete unchanged shared
`test_signals.py` inventory. Its async and sync Chromium rows pass candidate
and oracle through the same dedicated CDP Chrome. A separately selectable
Rust leaf admits 16 concurrent timeout waits and 12 KiB total state; Python
only validates and forwards milliseconds. The macOS spawn runtime exposes
only the pinned upstream `tests` package and installs the oracle CDP adapter in
spawned children, so neither lane launches an untracked browser. Four
Firefox/WebKit expansions remain explicit provider-gap `not-run` rows. This
checkpoint leaves 5,868 Python rows and all 48,477 Core JS/TS invocations
without candidate outcomes.

NODE-284 advances to ABI 52 and runs the complete unchanged async and sync
`test_page_network_response.py` inventories. All four Chromium rows pass in
candidate and oracle. Rust owns bounded terminal Response success, failure and
closure state; Python forwards only an opaque native owner. The eight
Firefox/WebKit expansions remain explicit provider-gap `not-run` rows. This
checkpoint leaves 5,864 Python rows and all 48,477 Core JS/TS invocations
without candidate outcomes.

NODE-285 advances to ABI 53 and runs the complete unchanged async
`test_page_network_request.py` inventory. Both Chromium rows pass candidate
and oracle through bounded native Context request waits, exact shared request-
history ownership, post-data JSON and same-CDP-cursor Frame availability. Four
Firefox/WebKit expansions remain explicit provider-gap `not-run` rows. This
checkpoint leaves 5,862 Python rows and all 48,477 Core JS/TS invocations
without candidate outcomes.

NODE-286 advances to ABI 54 and runs the complete unchanged sync
`test_input.py` inventory. Both Chromium rows pass candidate and oracle: one
verifies the existing file-input timestamp path and one verifies
`Page.expect_file_chooser` plus `FileChooser.set_files`. Four Firefox/WebKit
expansions remain explicit provider-gap `not-run` rows. The independently
selectable Rust composition intercepts each exact Page/OOPIF session, maps the
event's backend node through that session's current utility route, publishes
through the existing Page lifecycle fence and reuses the bounded file-input
payload/path implementation. The first implementation retained the native CDP
connection strongly and passed the body but hung during fixture teardown; the
final owner stores a weak connection plus exact route, so the unchanged test
exits cleanly. This checkpoint leaves 5,860 Python rows and all 48,477 Core
JS/TS invocations without candidate outcomes.

NODE-286's final repaired-wheel candidate and independent-oracle reports pass
2/2, SHA-256
`2b2f38ee68d026f3ea1d6eb6f46f8e30edfb8a2a0882921e2cd57622a50d1d38`
and
`0c84f8e75291bdae4bd87685ba9efbc86c12e598b5859a1b7e1f8d7e42e15685`.
The source SHA-256 is
`c203939164958aca0babcee63f2d9ae58afce16199fc2b6b5adbc4d6b2284fed`.
Normalized-reproducible repaired ABI 54 wheel SHA-256 values are
`ff2b00f7524235c3dbdfb1f93f3e29b7d9b3da885b550819c8be2b8df0eb6752`
and
`6584f4b92359e0fd1cddcfaf0bb814980b682eec5a2033457077c97c005d385c`.
Candidate/oracle tranche-ledger SHA-256 values are
`f39fc100ccaf2601cf852b32ca3e8c6543f816e0c34b6ff82eb251673f777b89`
and
`ab9953fae7ddd08720f454d84fb5be75ec0b61b3abf48f6bbd95a2eefc37ebea`.

NODE-287 advances to ABI 55 and selects two applicable Chromium rows from the
unchanged async `test_issues.py`: cancellation of a pending selector wait and
device-descriptor Context creation. The candidate baseline already passes the
Context row and exposes only the missing `Page.wait_for_selector` surface.
The final thin facade derives the native first-match Locator recipe and calls
one Rust-owned wait under one total deadline; attached/visible results publish
a real Handle and detached/hidden results return `None`. The cancellation row
passes without a teardown hang. The file's third Chromium row directly calls
generic `BrowserType.launch(ignore_default_args=...)`; the dedicated-Chrome
oracle adapter rejects that unsupported option, so the preserved red whole-
file diagnostic is an explicit launch-provider applicability gap, not a green
product claim. A separate service-worker-policy diagnostic is also kept red
and outside the ledger because its block rows time out on the mandatory
MagicDNS HTTP fixture.

NODE-287 candidate and independent-oracle reports pass 2/2, SHA-256
`18b7130d89320ae47ee72ee45c452d4d6a7b7204885921a1b5b7b624b6851718`
and
`991b5bf6cc6fc47d38cf7f4ce02fbca7964ef3b6c2c59bad9ec71cff2bcacb1f`.
The source SHA-256 is
`5d99e3d911d610cdb116e6b2c9e7d7ac7de8df051cfbe823712297465c375bea`.
Candidate/oracle tranche-ledger SHA-256 values are
`67542a752ae8dbd54d91dfab1ed69ca7ae59995a59bbb12894337493f05b00d0`
and
`c59574200b25fb454bccc03e7ad041ebe0bfc652c1fb8213db3ef93b192aad96`.
Normalized-reproducible repaired ABI 55 wheel SHA-256 values are
`11487ff7a6afb5b7d4ba021cc746dea4003bb646bfc3301b7a74a4b79a674869`
and
`4b493951e935460f9b3d752424f41b60bf58b10315306fdfb3633ffd141981aa`.

NODE-288 keeps ABI 55 and selects three Chromium rows from unchanged async
asyncio/expect-misc and sync API files. Candidate and independent oracle each
pass cancellation without an unretrieved future and sync Page selector wait.
The baseline's only discrepancy is that the already correctly classified
native `TimeoutError` lacks the official public `.name` property. Python adds
that Playwright-shaped error attribute without changing native ABI, protocol
or resource ownership. The final candidate and oracle pass 3/3; report SHA-256
values are
`95061944d9d6e0f66207653b10171d7ebd9d4881683efa7f8efdac77a7c3c611`
and
`18763889805affdb62fd0dc6951e665ba662fa67068d7f03b153820c1e6798c4`.
Candidate/oracle tranche-ledger SHA-256 values are
`1aa8ced79b0daeb738c681fb6a8903f3ceaf33ca6266b984efc4892343dd810a`
and
`9f9143a3dfcad40b0b9d572bd87d59f066fde4d3ff154e083e9ce6a447f27dd4`.
Normalized-reproducible repaired ABI 55 NODE-288 wheel SHA-256 values are
`5c2dcc36835eb0c5b26b60b6499824a843175910eba503f0d9ec6b6244f973f1`
and
`53016151567480d1af17f39111fb380b3e6148a88ab78f1aeaca1e5f733db0db`.

NODE-285's final installed-candidate and independent-oracle reports pass 2/2,
SHA-256
`c6fd93810787dea30f3beb58ec44202bdff7aad5975f1259852f363faae454cc`
and
`1d0416cb3d1e95cb2eab41e3118b7444fe2e47403a6bd98039d70837b392cba6`.
Three consecutive NODE-281-through-NODE-285 candidate regressions pass 10/10,
SHA-256
`60b7992906b838a93a98f8c61aa2a803dbc7485e9b3a358530aaea3f562b26f2`
and
`ac10e36d1bc833396547276ac1af8e9c79ff0932a5e2eaae9201936621bb34f7`
and
`cfe2c9dc9e7733e09e9b26bce562837a1b0a0dd75fe44347f0a2c6d81c7adac9`.
The source SHA-256 is
`6ead05641f44950db30e7ab038c973226ebe726312f39b9d7d7de2aacaf4a388`.
Normalized-reproducible ABI 53 wheel SHA-256 values are
`da1fcbdee1c0e1bfac900f3d054851942c792b5fd5958d2f15405c1c49d4b41c`
and
`0e51fcf2a4996266d7c19b070b2515fea5f335c00ec3414150b30c6b9f52fad0`.
Fresh full candidate collection remains 6,526/6,526, SHA-256
`66affc14f9da9eb21d94b750855572592ea92d1f0b039fbf6f18cdaeb01687b7`.

NODE-284's final installed-candidate and independent-oracle reports pass 4/4,
SHA-256
`98a5f800bdeb271affa5947bd91722e615129835d384ac5cb19e0ae81fe52b00`
and
`807cc17edba597a72a058041ce73de864942c36dbbc2def5179f2c5445fafa06`.
The NODE-281-through-NODE-284 candidate regression passes 8/8, SHA-256
`04ebd36660f93213091f0ac17235007f68fe471c477f3c0c7c96305f4086a65b`.
The async/sync source SHA-256 values are
`707b20a48b1b89529228697d5a87cb42a965a8afb9a485a5c951839c67dc2cba`
and
`82e3f5cfd0581f92085d1b56c06936fb890627dd505189b4894e36d7949e39fd`.
Normalized-reproducible ABI 52 wheel SHA-256 values are
`6eec71b2bcc8cc2fe658a0e28a5adab537370e74f20ddb62928ec521d23b102c`
and
`bd406d7ea915d9f95cb3d79c7ef13433ae8ef632c6b77d34059a82268b94b3aa`.
Fresh full candidate collection remains 6,526/6,526, SHA-256
`73608b054e7dd88df4ca79accebffa6f69561faa30a0868da4d18f5e014c2c2f`.

NODE-283's final installed-candidate and independent-oracle signal reports pass
2/2, SHA-256
`15327d9f5ab947fd6c9090f7f33c1be479c55355c6ab317e2cbd9985dae6f91e`
and
`a2b8d0ba5c8f9f3ebc78bcf4293042578af96805a4145305d008a0f7aa01300a`.
The three-file candidate regression passes NODE-281 through NODE-283 4/4,
SHA-256
`48997deecaa3332fcf8ba1a1cdafd7437fde07bac3b69b7cacd61322d063449d`.
The signal source SHA-256 is
`7f3d9cfc43ca8c5431df25b25ef75f1b38b0f9bac881e893384f622a3dc73267`.
Normalized-reproducible ABI 51 wheel SHA-256 values are
`39bf27fbf7d6bdd9d6c1513f11b6c0b37ba2928b2a70c7f921f67593dd3dbfe1`
and
`54b6323011e22a19f6b6beceba01fab317fc1bfbc11869f73f305e88acc1f0f0`.
Fresh full candidate collection remains 6,526/6,526, SHA-256
`d2b169846ada9162934a60b9e542e99f55096f43fa5a1a2d988cf2d935e372c8`.

NODE-282's final installed-candidate and independent-oracle event reports pass
1/1, SHA-256
`b0cb31378655ab7131bda8d57255e28b8378f2ace1275933e61230aa173c4a42`
and
`940665a60c4023e1f2e05a32fa3045339f6052406ddc71df1a7f88b2ffab4fef`.
The two-file candidate regression passes NODE-281/NODE-282 2/2, SHA-256
`04e6bf146f45099b4b56f3ab80f78daadeec5a580185b67ab7c79aeeeeec046e`.
The event source SHA-256 is
`34a3a26326764909a9fd0e1b97282ea5ca689c4a8c45f142725d10c42ee519ce`.
Normalized-reproducible ABI 50 wheel SHA-256 values are
`d70c33a033f01ff57dddcf6cee6bca90b31203e62e9a67cf6877b669202a051c`
and
`5e09beec25d3982f14a70550475fb0bb6f0c6bcf50c6aeba92e364181678ea7d`.
Fresh full candidate collection remains 6,526/6,526, SHA-256
`5a8c37f74beaee9487a5028138ae35af821bdd18ffa731d776251550e5587f7a`.

NODE-281's final installed-candidate and independent-oracle reports pass 1/1,
SHA-256
`1fdf1cf683d7ca5bba9f12db51f81bf2fb083d1d230520a25117f43c39ab254a`
and
`4b10bc1a7187c08b13e25a639cbd33baa45306edc04f0c6916913ddc3c48de59`.
Its source SHA-256 is
`8e42fa88bfa47b5f6ba501c2731d3d9c1812fddb28b4451aba923fb93bfe8c64`.
No product code or wheel changed; the latest normalized-reproducible ABI 50
wheel SHA-256 values remain
`0ae0b675aefe9479cc66b551a815e95a479be8419a09a839402a17d07badccec`
and
`db9345fce03a55a770371532d84e71e04824994cafd650ca8492adac96db0353`.
Fresh full candidate collection remains 6,526/6,526, SHA-256
`c4871850fb0b392fbabcac9feef6d144d50dc68e0233597d2207ca22e0da04dd`.

NODE-271's final installed-wheel reports pass 86/86 and 77/77, SHA-256
`49d736923082374a0d3eab81bea3cbaa539cf364105579856bc513c4cd2dde77`
and
`2cbb448b0651490286df3d7d559211ffac1b012527c42ff9176a3d72f6d07e02`.
The complete Locator ledger is 163 passed and 6,363 not run, SHA-256
`9b99936aa66ed493f322ab6e44adf7e5bad788780593caf6b46cf49d5ae01a2e`.
NODE-272's final installed-wheel sync semantic-Locator report passes 10/10,
SHA-256
`95362db9ab9b86874567fcee6bda9871f5c23e849005c0e9d03023ffc07abc73`.
The matching independent oracle passes 10/10, SHA-256
`62ec0bb876133ab0fc1be15665e5b637215c0120e263de6fdf5ffc9cb23118f5`.
Two normalized-reproducible ABI 45 wheels have SHA-256 values
`e0321e22e0ab05b57608f48d180689009d6113a55525a0b81a7a5776e04b71b0`
and
`0c30faaf48699e0b95426204f8c28e3e868abb5e94b2dccc004050ba61ac098b`.
NODE-273's final candidate and oracle reports pass 3/3, SHA-256
`cc1b66557ae5510345f4dae1163f86a4dce8af8c344d81530586fc0f3fdddf12`
and
`c0150e1af71e0e262d2a4edf5c5bfa3463aaef0272a5f6bac4aa00e975b32d02`.
Two normalized-reproducible ABI 46 wheels have SHA-256 values
`a665627bdb6bdda5ce0e8e2a72a37999b222c10ffa5e8ba44637f0f52da52900`
and
`e7a4c8933d3167ad24fea61e3e6354fa06a0df8cb6ccbb44662cee467712b20d`.
NODE-274's fill candidate/oracle reports pass 5/5, SHA-256
`36d2010856b1863f656c33ee6acd760b7afc04efbcf52b8e1e064e51d5d1a2d0`
and
`68b88b17bf5f1f3e6bbb59d7579a362b5b66a8c3d7c6126efbddce244a40665e`.
Its final context-manager candidate/oracle reports pass 4/4, SHA-256
`6f74927d6e6b67aef4e2387bab2b8f7981a7caf60d787a06a359089fca6910b5`
and
`22d67c00f17238be8514532a357afe1c5567658f0494df71646b58aa993b4679`.
The latest normalized-reproducible ABI 46 wheels have SHA-256 values
`6f2f3e4668083b081608d8ce92e1cdbe1f6040ea8d225f4bcfff44c9a62bc3ed`
and
`51c25e0d628d2dac7acf07c596c6815637000b86efba9056d6eac504d6b9096e`.
NODE-275's final candidate/oracle HTTPS-error reports pass 2/2, SHA-256
`cdd4eb40b5c8c40ede8c84f73056c5faf229837c497bf889d66545612510bea8`
and
`1f3179fc8f7deee06eb07ea775d238c9896bad297dd74f0afa1732732c4bef59`.
The latest normalized-reproducible ABI 47 wheels have SHA-256 values
`4074ac94f00f9463cf337e67af4b73671cf9d152ea1f6582735c9ad3265d49e7`
and
`5f4a2a2d71a0c6290344bd35b2b5ac2305e7653e21ce466d202e7dc584cdba7c`.
NODE-276's final candidate/oracle PDF reports pass 2/2, SHA-256
`c36fb38329822687850d1900d4b24a4b10361d423011ea8efea3c9adf619ac3e`
and
`036bffd3a26facd620b478f49ec81f98a70f01274cd51c1d925d370e1975d908`.
The latest normalized-reproducible ABI 48 wheels have SHA-256 values
`48593cfdd4a22249e2eee14186faceb95235670657e52aa808680d6e9a2fe348`
and
`b9acf998197b093ec2597c996193e6b3256fc97dfa23e73f3299de85c59bd2af`.
The earlier ABI 48 base64-return candidate pass is preserved as a superseded
diagnostic and is not part of the final cumulative ledger.
NODE-277's final candidate/oracle async PDF reports pass 3/3, SHA-256
`248379bc35eb7c2b78e6e3293ec7a7b9ada3a2f4327883765880e03153ea3355`
and
`647a7ca744aa2a3899d3f3756d1b7c5df38a213faf4f4e052b4b99948d3a6077`.
The latest normalized-reproducible ABI 49 wheels have SHA-256 values
`e837346a03487651337816b4adb8e93c6f48860b94252be8d00c8a33088d7e34`
and
`68ecf94ec7845f836f0b09dd5f8dffd8fab64ed0e64cbd49029f77fcba9cec28`.
NODE-278's final candidate/oracle async screenshot reports pass 3/3, SHA-256
`ad7e0c2b043641fa39a4fcac775945a193c4fef63a507f17a4572150bd270c58`
and
`d3f45897a07e19a2ac3aa2ac7a737bbaf67d1b3088bb12b8d819632142ca11e4`.
The latest normalized-reproducible ABI 50 wheels have SHA-256 values
`21017fae60321b12ab56bcc1b000e01e18c8892caa48e085a5622fa84a1da9ce`
and
`6d8369d2bb334fb28f650f244172a269bb2f0f694e14c42c5f4b7f06cbc3dfd7`.
The cumulative 6,526-row ledger above still leaves every one of the 5,875
unexecuted Python rows and all 48,477 Core JS/TS invocations explicit; it is not
a full-suite result.

NODE-269 first advanced to ABI 42 with a fixed 64-Page / 8 KiB Rust viewport
registry, CDP window/metrics override and session-initialization replay. Its
four-row diagnostic remains red because viewport mutation succeeded before the
same rows reached missing Locator screenshot and bounding-box methods. NODE-270
advances to ABI 43 and closes those downstream methods through Rust-owned
capabilities rather than declaring NODE-269 complete prematurely.

NODE-257 pins four async and two sync Chromium Locator composition rows from
the unchanged upstream files. The ABI 34 candidate fails all six with missing
`locator` / `nth` members. The final installed ABI 35 candidate and independent
oracle each pass 4/4 async plus 2/2 sync. Rust owns root query admission,
typed within-composition, nth derivation and all subsequent read/count
execution; Python only wraps the immutable native owner and optional
description metadata. Exactly six rows change from `not-run` to `passed` in
each ledger. This does not establish unselected Locator strictness,
assertions, actions, non-Chromium providers or full-suite parity.

NODE-256 pins the three real async-only Chromium Locator description rows from
the unchanged upstream file. The initial candidate records three missing-
member failures; the final installed ABI 34 candidate and independent oracle
each pass 3/3. `description` / `describe` is immutable Python facade metadata
over the same Page and selector, not browser behavior or retained native state,
so no native ABI or capability edge changes. The two chained-description rows
were intentionally left `not-run` in that checkpoint and are covered by the
later typed Rust composition gate in NODE-257.

NODE-255 pins ten additional unchanged async/sync Locator Chromium
invocations: click, click after node replacement, collection evaluation with
present and missing selectors, and count after main-world `Map` deletion. The
final installed ABI 34 candidate and independent official oracle each pass
5/5 async plus 5/5 sync. The preserved 4/5 candidate diagnostic proved that
the underlying count read worked while the Python `expect` facade was still a
placeholder. Rust now owns typed count comparison, the total deadline,
navigation/context-replacement retry and bounded sleep; Python and Node only
forward assertion inputs. Exactly ten rows changed from `not-run` to `passed`
in each ledger. This does not establish remaining Locator assertions,
actionability, providers, or full-suite parity.

NODE-254 pins 34 basic async/sync Locator Chromium invocations. All pass in
the final installed-wheel candidate and independent oracle. Python only
forwards public values to existing Rust Locator read, collection, action and
evaluation owners. The one semantic discrepancy after facade addition was
Playwright `:text("...")`; Rust now compiles it as a typed exact text leaf
instead of passing it to DOM CSS. Initial 34-failure and intermediate 16/17
reports remain outside the green ledger. Remaining Locator APIs and provider
expansions are still explicit `not-run` rows.

NODE-253 adds an optional Rust-owned layout-selector parser/compiler without
changing ABI 33. The exact unchanged async/sync Chromium
`test_should_work_with_layout_selectors` rows pass 2/2 through the final
installed candidate wheel and 2/2 through the independent official oracle.
Rust owns bounded clause parsing, geometry score formulas, ordering and the
browser-world program; Python only forwards selector text. The initial
`selector script threw` candidate reports remain diagnostics outside the green
ledger. Arbitrary nested CSS/custom-pseudo layout shapes, retry polling,
positive browser-program `Page.query_selector` handle publication and the
Firefox/WebKit providers remain open.

NODE-252 adds Python ABI 33 registered selector-engine execution. Two
manifests pin the unchanged async/sync `test_queryselector.py` sources and fix
exactly the base registration function plus the path and main/isolated-world
functions. Their six Chromium expansions pass 6/6 through the final installed
candidate wheel and 6/6 through the independent official oracle. Rust owns the
bounded registry, immutable compiled program, distinct-engine installation,
single/all query semantics, resource ceilings and execution-world choice;
Python only forwards public calls. The prior unknown-engine, accidental strict
single lookup, and final custom-part `queryAll` failures remain diagnostic
reports outside the green ledger. Positive registered `Page.query_selector`
handle publication, typed Locator filters/composition, retry polling, and the
12 Firefox/WebKit expansions remain open.

NODE-251 adds Python ABI 32 native selector validation. The unchanged async
and sync Chromium `test_selectors_register_should_handle_errors` rows pass 2/2
in the final installed-wheel candidate and 2/2 in the independent oracle. The
preserved diagnostic proves the old path leaked a DOM CSS `SyntaxError`;
ABI 32 now surfaces the Rust parser's exact unknown-engine error before browser
execution. The four Firefox/WebKit expansions and custom-engine execution stay
`not-run`.

NODE-250 adds Python ABI 31 global selector registration. The exact unchanged
Chromium duplicate-registration invocation from
`tests/async/test_selectors_misc.py` passes 1/1 in the final installed-wheel
candidate and 1/1 in the independent oracle. Rust owns the bounded registry,
source retention and error semantics; Python only performs bounded source/path
normalization. The selected row imports upstream-private symbols for type/error
identity but asserts public `playwright.selectors.register` behavior, so it is
applicable rather than excluded. Custom selector-engine browser execution and
the Firefox/WebKit expansions remain open.

NODE-249 runs the async twin of ABI 30's collection-selector test instead of
inferring binding parity. The exact Chromium invocation from the complete
unchanged `tests/async/test_selectors_misc.py` source passes 1/1 in the
installed-wheel candidate and 1/1 in the independent oracle. This changes no
ABI, feature selection or Rust dependency closure. Its Firefox/WebKit rows
remain explicit `not-run` provider gaps.

NODE-248 adds Python ABI 30 collection selector evaluation. The exact
Chromium invocation from the complete unchanged
`tests/sync/test_selectors_misc.py` source passes 1/1 in the final
installed-wheel candidate and 1/1 in the independent oracle. Python forwards
only Frame identity, selector, function source and argument; Rust owns typed
`internal:and` intersection, collection resolution, utility-world evaluation
and resource admission. The production connection admits 8,192 event items
while retaining the prior 512 MiB aggregate event payload ceiling, preventing
small console-history replays from multiplying the byte declaration. Two ABI
30 wheel payloads are reproducible, local tests pass 29/29, listener
regressions pass, and collection remains 6,526/6,526. The all-feature host
profile also proves `network-events` and `concurrent-close` preserve the
maximum of their independent runtime object declarations.

NODE-247 completes the Python ABI 29 listener checkpoint for the sync binding.
The exact Chromium invocation from the complete unchanged
`tests/sync/test_listeners.py` source passes 1/1 in the final installed-wheel
candidate and 1/1 in the independent oracle. Its full-file candidate
diagnostic preserves the Firefox/WebKit provider errors and is not merged into
the green ledger. The async row also passes 1/1 again on the same final wheel.
Reconnect preserves existing network subscription lanes while the
authoritative snapshot is incomplete, then reconciles adapter sessions and
retires only Pages absent from the completed snapshot. The bounded event
replay window is 2,048 entries for the declared host profile; recovery remains
one reconstruction and one epoch advance. Two final wheel payloads are
reproducible, local tests pass 28/28, and collection remains 6,526/6,526.

NODE-246 adds Python ABI 29 response-listener publication. The exact pinned
Chromium invocation from the unchanged `tests/async/test_listeners.py` source
passes 1/1 in the final installed-wheel candidate and 1/1 in the independent
oracle. Rust owns subscription activation, a 16-Context interest set, a
256-notice FIFO and the shared 16 MiB network byte budget; Python owns callback
identity and public `Response` wrappers only. Full-file diagnostics retain the
Firefox/WebKit provider errors, while the canonical ledgers leave those two
rows `not-run`. Two final wheel payloads are reproducible, local tests pass
28/28, and collection remains 6,526/6,526. Failed candidate attempts remain
preserved outside the green ledger: they exposed sticky topology event loss
and a conflation of materialized Page membership with transient command-session
routing. The repaired host performs one authoritative same-connection
reconstruction and one monotonic epoch advance, without an unbounded retry or
dependency-closure change.

NODE-245 adds Python ABI 28 device descriptors and Context-scoped device
emulation. The complete unchanged official source file records one Chromium
pass plus its two upstream Firefox/WebKit skips in both final-wheel candidate
and independent oracle lanes. Rust embeds all 143 v1.58.0 descriptors, owns
the 16-Context/64-KiB settings registry and applies CDP emulation before Page
publication; Python only converts public descriptor keys and forwards bounded
values. Two final wheel payloads are reproducible, the installed wheel passes
26/26 local tests, and collection remains 6,526/6,526.

NODE-244 adds Python ABI 27 bounded request history over the existing
network-events package closure. The pinned unchanged async and sync Chromium
last-100 rows pass 2/2 in the final-wheel candidate and 2/2 in the independent
oracle. Rust owns last-100 eviction, 4,096 retained owner identities, 64 MiB
of event bytes and response/body settlement; Python owns only façade wrappers
and callback identity. Two final wheel payloads are reproducible, the installed
wheel passes 25/25 local tests, and collection remains 6,526/6,526.

NODE-243 adds Python ABI 26 bounded console-message history as a separate leaf
from console callback publication. The pinned unchanged async and sync
Chromium newest-200 rows pass 2/2 in the final-wheel candidate and 2/2 in the
independent oracle. Rust owns recording, newest-200-per-Page eviction,
3,200-item/16-MiB admission and Page/session cleanup. Two final wheel payloads
are reproducible; the installed wheel passes 23/23 local tests and
6,526/6,526 collection. Retained console argument Handles are not claimed.

NODE-242 adds Python ABI 25 bounded Page-error history. The pinned unchanged
async and sync Chromium rows pass 2/2 in the final-wheel candidate and 2/2 in
the independent oracle. Rust owns recording, newest-200-per-Page eviction,
3,200-item/16-MiB global admission and Page/session cleanup; Python only
converts the native snapshot to public `Error` objects. Two final wheels have
reproducible payloads and the installed wheel passes 22/22 local tests plus
6,526/6,526 collection.

NODE-241 adds Python ABI 24 over the existing Rust `request-gc` leaf. The
pinned complete async and sync Chromium files pass 2/2 in the final-wheel
candidate and 2/2 in the independent oracle. Python forwards only Page identity;
Rust owns session lookup, command admission and exact heap-collection dispatch.
Two final wheels have reproducible payloads and the installed wheel passes
21/21 local tests plus 6,526/6,526 collection.

NODE-240 fixes terminal route settlement and executes the complete 125-row
async Chromium Page file with per-case process isolation in both lanes. The
candidate and independent oracle each record 124 passes and the same upstream
FIXME skip, with zero status differences. Candidate report SHA-256 is
`709000c785522a979b7f56b344216a9ed0a4f8b9d783ffc725dea8e15c9ba9f3`;
oracle report SHA-256 is
`d8058d713c4cc1fcefbbab295b4eb9aa5647dc98eb9f8453a8e877319be7dade`.
Rust retains at most 64 resource-accounted terminal route receipts when Page
retirement removes pending interceptions; one later terminal action consumes
the receipt and duplicate processing still fails.

NODE-239 adds resumable per-expanded-case pytest subprocess isolation. It reads
the pinned inventory, respects explicit browser filters, validates each exact
JUnit identity, checkpoints per-case XML and synthesizes an explicit row when a
process crashes or exceeds the outer deadline. A historical regression runs
the former route-close hang first: it fails at 15 seconds, while the following
unchanged title test passes in its fresh subprocess. The deterministic two-row aggregate
SHA-256 is
`4899c51e6a63307f5ff89884869b50ab054437264c6e64fd2b273568c984c885`.
This proves runner isolation, not product compatibility for that historical
failed row; NODE-240 supplies the repaired current product evidence.

NODE-238 adds Python ABI 23 and a separate Rust headless Page-pause
composition. The fixed two-function manifest passes 2/2 in the repaired
candidate wheel and 2/2 in the independent official oracle with unchanged
assertions on the same task-owned headless Chrome. Candidate report SHA-256 is
`7ffe205309fc5864bf6c9acae98c4e05a1b3d99be3e72fd3569a0090e718ad2c`;
oracle SHA-256 is
`1c5d32e8553abefefa9fa7b52b6a3cc6d3df0a7adedf21f80cc667e09cd69ff5`.
Rust owns scoped Page/Context operation/navigation timeout reset and exact
drop-safe restoration; headed Inspector pause remains unsupported. Exact
replacement leaves the Page report at 124 passes and one upstream skip. This
closes two rows only; 6,093 candidate invocations remain unexecuted.

NODE-237 adds Python ABI 22 and a separate Rust installed-Chromium persistent
launch composition. The fixed one-function manifest passes 1/1 in the repaired
candidate wheel and 1/1 in the independent official oracle with unchanged
assertions. Candidate report SHA-256 is
`57c6a49de8942866cae9f67b480cae5c553ecb672606fce14f3370a5396ff862`;
oracle SHA-256 is
`0b68621888607947312b52b2341c7e7c366c7312fc3d64b7f068a063026d86ad`.
Both lanes select installed `channel=chrome` because the official cached
headless shell is absent; the original missing-shell report remains an
infrastructure failure. Rust owns strict profile admission, executable
selection, loopback CDP, process authority and bounded shutdown. This closes
one row only: 21 other collected persistent-Context invocations and 6,093
candidate invocations overall remain unexecuted.

NODE-233 adds Python ABI 19 and a separate Rust Page/Locator drag composition.
The fixed eight-function manifest expands to six async and two sync Chromium
invocations; the final candidate wheel and independent official oracle each
pass 8/8. Candidate report SHA-256 values are
`3069011993af14bceb5f2d367f23227bd5dbae727d8b1264617ac2acde1ed2f9`
and `4287ae3d14813d32edccaa4ae422581f141745a85e06037dbed805313d177f8f`;
oracle values are
`5ef118e81c96770c699f817e464db6151798e7445306c504ceea5cb339347c07`
and `dcf390058a973cbf86ded8efef0c03cdc20e90d2a823912376f6110e0d0e381d`.
Exact JUnit selection replaces four Page outcomes and adds four previously
not-run rows once. The canonical Page report is 119 passed / five failed / one
upstream skip, SHA-256
`ab5d9e713f0e465c008e004aa06ac82bab218362230de2779d797ea230700d2a`.
Rust owns ownership validation, scroll/geometry and bounded Mouse/drag input;
Python only maps public options and wrapper identity. Full actionability,
force, non-Chromium and full-corpus claims remain open.

NODE-232 adds Python ABI 18 and Rust-owned `Request.sizes()` settlement to the
existing network-events leaf. The fixed two-case unchanged-upstream manifest
covers a five-byte POST body, an empty GET body and raw on-wire request-header
sizes. The final candidate wheel and independent official oracle each pass
2/2. Candidate report SHA-256 is
`6506a861160f462eca4b6b104b0f8c9a11f7307b1518c1a592ed9d1804b8aa47`;
oracle SHA-256 is
`bb71eed6453fe9c8d2fc14563877804f44893df23e227cf2c8f6b01ade66de51`.
The exact-replacement Page report is 115 passed / 9 failed / one upstream skip,
SHA-256
`07cf64b7c9cf428c8c95daf6fea98a47d17a0d33ca92c60c19b9cf8eaaae2e0c`.
Rust owns bounded body/header capture, completion/error/close wakeup and raw
CDP extra-info correlation; Python retains only an opaque owner. This does not
claim complete Core response-size/redirect parity or full-corpus completion.

NODE-231 adds Python ABI 17 and a separate Rust Page request-routing leaf. The
fixed five-case unchanged-upstream manifest covers async caller stacks,
set-content navigation timeout/resource waits and continue during/after Page
close. The final candidate wheel and independent oracle each pass 5/5.
Candidate report SHA-256 is
`b8da7c43b37f0c7cabd9cd740348b907fe20ad4899b749d821c72818f6d3b3c3`;
oracle SHA-256 is
`eea2184bd73833dbb532c3ba252e7259c3b918d06b8cdcbc8c92d1bc42546490`.
The exact-replacement Page report is 113 passed / 11 failed / one upstream
skip, SHA-256
`188e6bcd09ae3dad589a24e1d2d1632ad8d01424cdb65bfc75e1f5308e93da30`.
Rust owns bounded interception, paused-call state, selection and terminal
actions; Python only invokes retained handlers. This does not claim regex,
predicate, `times`, override, fulfill/fallback/unroute, Context or sync routes.

NODE-230 adds Python ABI 16 and a separate Rust Page-error observer. The fixed
five-case unchanged-upstream manifest covers Error name/message/stack, custom
name, primitive values, objects and Window. Candidate and independent oracle
lanes each pass 5/5. Candidate report SHA-256 is
`28cd246e9902b1b36494b7e6ffbee0b8fede241426b834e455e2e85f10f2548d`;
oracle SHA-256 is
`e341777d61bc4d02dd11d8e4233103202cf55905ebcb658902020a618288cc2f`.
The exact-replacement Page report is 108 passed / 16 failed / one upstream
skip, SHA-256
`1414096c00e3905b6d0f080e8357eb6fa2cd7b23047125cfd16c2b24bceb52d2`.
Rust owns native event observation, bounded conversion and publication; Python
only constructs the public Error wrapper from a committed descriptor.

NODE-229 selects Python ABI 15 over the existing bounded Rust Page-lifecycle
publication leaf. The fixed six-case unchanged-upstream manifest covers load,
DOMContentLoaded, explicit close, external `window.close()` and opener
behavior. Candidate and independent oracle lanes each pass 6/6. Candidate
report SHA-256 is
`27c46dc49101b3047f5ece6b57c774319576f1fd47f714fa16412028ac9a9081`;
oracle SHA-256 is
`a1b2e6080dcb7e406c2a9a903d9d24340a0edddd0ff00a4547e8c640e2a0e621`.
The canonical 125-case Page report is 103 passed / 21 failed / one upstream
skip, SHA-256
`672e39ff6b08068e4470386a0e70c925d7b52de8c1aa091bd0cb042c7b6a0c41`.
Rust owns observation, ordering, publication and close/network retirement;
Python schedules immutable notices only after native commit, acknowledgement
and drain release.

NODE-228 selected Python ABI 14 over the existing bounded Rust Page-binding
owner. Its fixed manifest passes 12/12 candidate and 12/12 oracle. Candidate
and oracle report SHA-256 values are
`85de412561f1139288706adfb2969e8a9c5685ea66c6eba3ab6b54ee0e7d1920`
and `65cc682d7e21a35d41d3a5a2c36e745e1f7215ad14d7d69dd14f2be4de29c16b`.
That historical checkpoint is superseded by ABI 15.

NODE-227 selects Python ABI 13 over the already bounded Rust Handle and
wait-for-function capabilities. Python owns wrapper identity, path reads and
public overload validation only; Rust owns remote-object capacity, exact
session provenance, truthy polling, publication, JSON reads and disposal.
The fixed unchanged-upstream tag manifest passes all 18 Page script/style URL,
path, content, module, sourceURL, CSP and load-error nodes in both candidate
and independent oracle lanes. A separate fixed JSHandle manifest passes 7/7
in both lanes for direct Handle arguments, primitives, unserializable values,
cyclic JSON and element classification. Candidate report SHA-256 values are
`eef16558d22d914910da1f76adb24c21713401537952d8b1f5aa8605175a0797`
and `73785780841f80f6887998f7a0dec34dc3a69e12e5bf7cb95ed464d8bb387e8c`;
oracle values are
`c3c4ee77734fafffbfc31cde4dd9a8783ea80c6af5c9949e1838bba49b5b8c7c`
and `045fe6fc13956e23e5cb1fc8342310385528b37a5da1da2d071ef6e73492ca91`.
The candidate Page report is reproducibly superseded case-by-case rather than
double-counted; its 125 records are now 85 passed, 39 failed and one upstream
skip. A separate full-file diagnostic that entered terminal state after the
known beforeunload gap is excluded from the ledger.

NODE-226 adds Python ABI 12 and extends the shared Rust `init-scripts` leaf
from Page-only registrations to explicit Page or BrowserContext scope. Rust
owns the fixed 128-script store, exact current-session fanout, future Page and
out-of-process Frame installation, and partial-install rollback. Python only
normalizes a string or reads a path; Node now exposes the same Context method.
The complete unchanged upstream async and sync `test_add_init_script.py` files
pass 9/9 each in the candidate and 9/9 each in independent official oracle
lanes. Candidate report SHA-256 values are
`43fc0287f84a6b789145d852feb5a123ab495730d332146d084c925dbee130f4` and
`8d4d8e0cc0d07c934fcda6633e96b689cabfcb70fa6dd43fc339b68b69f1dadf`;
oracle values are
`451b9626c54141ede463d71d71b62bb7ed0bd24deaad93690e25862e2522a9bc` and
`61a7bd9a8abdc6b7194e5ef63c560eb66ddb881e21e8f3ff326350fdae0a2ba7`.
The installed repaired wheel passes 13/13 local tests and the Node package
passes 269/269.

NODE-225 corrects three Python Page-fill discrepancies. Rust emits the pinned
invalid-target capitalization and element labels, while the thin Python facade
maps a non-string value to Playwright's public `Error` wording before PyO3.
The complete unchanged 125-case async Page file now records 67 passed, 57
failed and one upstream skip; candidate report SHA-256 is
`53848c57654b5d3397edae116701e77b75a65f9bb53b9a44aac14b835c841c48`.
The three corrected nodes pass 3/3 in an independent official oracle report,
SHA-256
`7cdef633380f69c0fb2043e93ea42b16c2dbb1c39db28c3fe71ce047feb821a8`.
A full oracle diagnostic reached the final debugger-only `page.pause()` rows,
but the visible shared Chrome is not equivalent to the fixture's
`headless=True`; that interrupted diagnostic is not in the oracle ledger.

NODE-223 adds a separately selectable Rust client-certificate leaf and Node
profile. Node performs public shape validation and bounded file/Buffer
conversion; Rust owns exact-origin selection, lazy PEM/PFX decoding, TLS
configuration and bounded per-origin caching. Certificate material is not
decoded for an unmatched origin. The exact official validation row passes 3/3
candidate and oracle. Seven direct-request behaviors pass 21/21 in each lane:
no certificate, HTTP unaffected, untrusted certificate, trusted PEM, trusted
PFX, wrong PFX passphrase and unsupported legacy PFX. The candidate request
report SHA-256 is
`10a549ed1ff3094493189a926b64e35ae5c770c046fa0e1acb83494803109514`;
oracle SHA-256 is
`0aac6b69dedc4e56c6f63131ac10f7a07b9a71f0cceccbd762c8cec73cb15844`.
Both lanes use the same source-only fixture adaptation to bind official TCP/TLS
servers to the verified Tailscale IP and address them through full MagicDNS.
The nine request crates pass 96/96 release tests, Node passes 268/268, strict
types/Clippy and the dependency proof pass, and a real Tailscale-bound PEM/PFX
mutual-TLS smoke passes. Proxy rows, browser-context/persistent-context rows,
browser interception and multiple identities for one origin
remain open.

NODE-221 exposes the optional Rust API-request stack through a thin Node
profile and runs complete official browser-independent files rather than a
smoke subset. Candidate and independent oracle each pass all 198 Library and
396 Bidi `global-fetch.spec.ts` invocations, plus all 60 Library and 120 Bidi
`global-fetch-cookie.spec.ts` invocations. The candidate therefore has 774
official API-request expected outcomes with matching oracle outcomes. The
fixed browserless manifest forbids ad-hoc `--test` overrides; Library and Bidi
project identities remain distinct in the canonical ledger. Rust owns request
execution, cookies, origin retention and at most 64 request-scoped resolver
addresses; the original hostname remains Host/SNI. API-request release tests
passed 91/91 and Node package tests passed 266/266 at this checkpoint, with
strict types, targeted strict Clippy and a real native seven-verb HTTP smoke.
Proxy, BrowserContext request sharing and the remaining official corpus were
still open.

NODE-220 implements the pinned Playwright 1.58.0 Happy Eyeballs ordering and
300 ms launch cadence in the optional Tokio TCP leaf. At most 64 interleaved
IPv6/IPv4 attempts are call-owned and every loser is dropped on success or
cancellation; Rustls reuses that TCP path. The eight API-request crates pass 88
release tests, doctests, rustdoc and all/pedantic strict Clippy. The official
`browsercontext-fetch-happy-eyeballs.spec.ts` cases were not executed because
the public profile and test hook are not wired, so every official ledger count
and hash below remains unchanged.

NODE-219 adds bounded final-head events to the HTTP/1 codec. Transport now
stores Set-Cookie at headers and discards followed-redirect or handled-Basic
challenge exchanges without consuming their bodies; terminal responses retain
the configured body and decompression limits. The eight API-request crates pass
83 release tests, doctests, rustdoc and all/pedantic strict Clippy. No Node or
Python profile exists for this capability, no official test was executed, and
all ledger counts and hashes below remain unchanged.

NODE-218 adds an optional bounded HTTP Basic provider. Rust now owns exact
origin matching, UTF-8 Basic encoding, default challenge retry, preemptive
`send: always` and explicit Authorization precedence. The eight API-request
crates pass 78 release tests, rustdoc and all/pedantic strict Clippy. The public
language profiles and header-first challenge handling remain missing. No
official test was executed; all ledger counts and hashes below remain
unchanged.

NODE-217 adds an optional bounded global cookie owner behind a store port.
Rust now owns structured import/export, Set-Cookie parsing, URL matching,
redirect recomposition and explicit caller Cookie precedence. The seven
API-request crates pass 66 release tests, rustdoc and all/pedantic strict
Clippy. BrowserContext sharing, full origin storage state, header-first redirect
handling and language bindings remain missing. No official test was executed;
all ledger counts and hashes below remain unchanged.

NODE-216 adds independently selectable concrete Tokio TCP, Rustls HTTPS and
bounded standard-decompression leaves. Request-scoped `ignoreHTTPSErrors`,
HTTP/1.1 ALPN, cancellable I/O, 64-answer DNS bounds, gzip/x-gzip, deflate and
Brotli now stay in Rust. The six crates pass 53 release tests, including an
in-memory self-signed TLS handshake, plus rustdoc and all/pedantic strict
Clippy. Happy Eyeballs, proxy/credentials/client certificates, cookies and
public bindings remain missing, so no official test was executed and every
ledger count/hash remains unchanged.

NODE-215 adds an injected-connection transport-policy layer over the raw HTTP/1
codec. Rust now owns redirect budgets and method/header rewriting, reset-only
backoff retries, cancellation, fixed read memory and bounded non-fatal logs.
Eight release tests, rustdoc and all/pedantic strict Clippy pass. There is still
no concrete DNS/socket/TLS/proxy/decompressor/cookie or language-binding path,
so no official test was executed and every ledger count/hash remains unchanged.

NODE-214 adds a separately selectable raw HTTP/1 codec beneath the future
API-request transport. It preserves exact response header case, duplicates and
order, handles bounded fragmented framing, and rejects non-Latin-1 request
headers, truncation and trailing exchanges. The base and codec crates pass 17
+ 11 release tests, rustdoc and all/pedantic strict Clippy. It owns no socket,
TLS, proxy, decompression, redirect, cookie or language-binding path, so no
official test was executed and every ledger count/hash below remains unchanged.

NODE-213 moves official request-construction semantics into the shared Rust
owner: encoded/repeated query parameters, default/context/request header
precedence, raw/JSON/form/multipart serialization, MIME inference and exact
content headers are bounded before the transport port. The crate passes sixteen
release tests and strict Clippy. No public binding or transport is selected, so
this changes no official outcome or ledger hash.

NODE-212 starts the browser-independent `APIRequestContext` discrepancy family
with a shared Rust ownership crate and injected transport port. The foundation
matches pinned timeout-zero, status-failure and disposal semantics; it bounds
request/response inputs, aggregate retained bodies/logs, concurrency and
cancellation. Eleven release tests and strict Clippy pass. There is no concrete
HTTP backend, cookie/storage adapter, Node/Python binding or official
oracle/candidate API-request result yet. Therefore ledger counts and hashes
below are unchanged, and this work is not an official conformance pass.
[Design and remaining gates](api-request-design.md).

NODE-211 adds the source-side diagnostic observation needed by the remaining
boolean matcher failures: Rust retains the last valid received state and may
collect one bounded element preview on the first mismatch, only while the
original expectation deadline still has time. The addon exposes explicit
preview/missing fields; Node formats their public call log without DOM or
browser access. Package 257/257, focused Rust expectation 12/12, strict
TypeScript, selected native-load and selected-profile Clippy pass. This has not
changed any official result or ledger: the runtime guard refuses a browser run
while the restored unrelated page is present. The 62/24 candidate report below
therefore predates both the guard and this source change and still requires an
exclusive empty-browser rerun.

The current Core candidate ledger records 1,222 expected, one explicit upstream
skip, 31 unexpected and 47,223 explicitly `not-run` invocations. Its
SHA-256 is
`9d7b1785ed238f69fbf2931d604aa5bd25e19d0076ecbd9e2a1234f61ceed927`.
The exact official client-certificate validation reports pass 3/3 candidate,
SHA-256
`86f4c44e7af6683153b444f0b93fa43c068f1335c4b1f21fd3aa9154f5cebe6e`,
and 3/3 oracle, SHA-256
`a6b92910ce85cdafe025cc00f58ce47d531d4bffcd71bbaf952213859944cb2d`.
The exact direct-request client-certificate reports pass 21/21 candidate,
SHA-256
`10a549ed1ff3094493189a926b64e35ae5c770c046fa0e1acb83494803109514`,
and 21/21 oracle, SHA-256
`0aac6b69dedc4e56c6f63131ac10f7a07b9a71f0cceccbd762c8cec73cb15844`.
The complete unchanged `page/expect-boolean.spec.ts` oracle file passes 86/86,
report SHA-256
`c7ee31f961819363811fb80669a11f3d69f9cfcaa98ca75543fa998a8dfea0cf`.
The corresponding candidate file progresses from 37 expected / 49 unexpected
to 62 expected / 24 unexpected, report SHA-256
`e9bfbcd0a4cb8dfc95990a27ea52d9cd14f13b89016c076d666536ee72fe3b5d`.
The independent Core oracle ledger is 905 expected / 47,572 not-run, SHA-256
`041648606bf73c419e2d40dd967983063616593bf81d1969756f912dbd873afc`.
These reports cover complete selected files and project matrices, not the full
corpus.

The focused NODE-292 candidate reports are query-selector 7/7, SHA-256
`26daf190d3b76fe3b74f7775e6ed7e26ba6587e50ce8ff4df9c40a564e8c641f`,
eval-on-selector 6/6, SHA-256
`da103daf6b9ad6cabd363423ad5a77fdd57275a8acd337bd4095faf0fa4f0c91`,
and inner-text 2/2, SHA-256
`c216deea53fd876729ba8699c0a0e0d1510ba8fdc5148c3d8022407ff3a69b0f`.
The corresponding oracle SHA-256 values are
`bf69f295c4b9f7b1dfea6c7b78c22ab35c623efc48837ee1ef057f5a1b5477e2`,
`4de4f288a4e00d3df628f756f23ca994609393283cd8376d19b67e83119b026f`
and
`d5561e8c0563123d457b3d36c418996d0cf1e5be648fcbc2ac10043f3271182c`.
`summarize_playwright_official_results.py --base` validates every prior ledger
row against the pinned inventory before replacing outcomes and still rejects
overlap between new reports.

The first 62/24 candidate run began while a nonblank page from another task
was present in the dedicated Chrome; fixture cleanup retired it, and the page
was restored from its recorded exact URL. The runtime runner now inventories
the loopback CDP `/json/list` endpoint and refuses candidate or oracle
execution while any nonblank page exists. The 62/24 result remains discrepancy
progress but must be repeated under exclusive empty-browser ownership before it
can serve as final isolation evidence.

The final-source complete unchanged `page-click-timeout-2.spec.ts` run passes
2/2, report SHA-256
`24082ff695e75d44e9779e335143283b74c85fa8c596097cc6e25b8419e57370`.
A controlled five-file, 12-invocation Page regression batch records nine
expected and three unexpected outcomes, report SHA-256
`87880edd0948895b22b6ce90e3448baf4cbbe495383e4cfe57f8649610558860`.
The three batch discrepancies are client-redirect/network-idle lifecycle,
dialog events and the upstream implementation-private
`__testHookBeforePointerAction`. These counts are execution progress only:
47,231 Core invocations and 6,091 Python invocations remain `not-run`, so no
full-suite claim is permitted.

The actionability slice preserves one original runtime deadline. Deadline
cancellation and browser cleanup complete before a synchronous/no-I/O Rust
hook consumes the exact Locator route's last bounded actionability enum. The
slot is cleared at call start, on ready/runtime replacement and normal
completion. Node only formats fixed keys; it performs no DOM probe, browser
call, second timer or retry loop. Package 256/256, runtime 114/114, Locator
backend 115/115, Chromium composition 397/16, selected Node host 617/14 plus
host integrations 22/8, strict TypeScript/native-load and selected host/addon
Clippy pass.

## Completion rule

Pwright may claim full upstream conformance only after all of the following are
true:

1. Every test from the pinned Playwright core and Playwright Python sources has
   a stable inventory identity.
2. The unchanged upstream assertion runs in isolated official-oracle and
   Pwright-candidate lanes.
3. Every invocation has a recorded pass, fail, skip, or explicitly reviewed
   non-applicable result. There are no silent exclusions.
4. Public API failures are eliminated through composable Rust capabilities and
   thin Node/Python bindings. Upstream implementation-internal tests remain in
   the ledger and require a specific classification and reason.
5. Packaging, ABI, dependency-boundary, cleanup, memory, documentation, and CI
   gates pass.

Collection success is not execution success. An oracle-only pass is not a
candidate pass, and a source-file count is not an expanded test count.

## Pinned sources

| Corpus | Tag | Commit |
| --- | --- | --- |
| `microsoft/playwright` | `v1.58.0` | `961381ec73a9fa2661d92504eab1a6bd6eaffc75` |
| `microsoft/playwright-python` | `v1.58.0` | `47a5d35ef4f815a2021349f86ae391f7c20c02d6` |

The core manifest is `tests/official/playwright-core-manifest.json`. The Python
manifest is `packages/pwright-python/tests/official/manifest.json`. Both source
checkouts must match their commits and have no tracked modifications.

## Inventory baseline

### Playwright core JavaScript and TypeScript

The pinned repository contains 558 tracked spec files: 518 `.spec.ts`, 25
`.spec.tsx`, and 15 `.spec.js`. Of these, 510 are under `tests/` and 48 are
official examples.

Twenty runner/config entries expand to 48,477 invocation records:

| Role | Expanded invocations |
| --- | ---: |
| Nine official test entrypoints | 47,873 |
| Six nested component configs | 537 |
| Five official example configs | 67 |
| Total | 48,477 |

The inventory directly collects 554 spec files. Four files below are explicitly
ignored by the installation config and are copied/executed by parent
installation tests, so they are recorded as nested fixture scripts rather than
silently omitted:

- `tests/installation/fixture-scripts/docker.spec.js`
- `tests/installation/fixture-scripts/failing.spec.js`
- `tests/installation/fixture-scripts/plugin.spec.ts`
- `tests/installation/fixture-scripts/sample.spec.js`

The generated JSONL ledger has 48,477 lines and SHA-256
`2f7fe371e87267225d195ce537db18621728902ebb5e343ab98c2bc8144b8c30`.
It is local evidence under `.work/official-conformance/`, not a checked-in
replacement for the source manifest. Its initial conservative source-level
classification records 31,739 `public-api-candidate`, 11,231
`implementation-internal`, and 5,507 `product-infrastructure` invocations.
These labels are review queues, not exclusions or final applicability rulings.

### Playwright Python

The pinned repository contains 157 `test_*.py` source modules: 149 in `tests/`
and eight in `examples/todomvc/`. Pytest expands them to 6,526 node IDs across
the default Chromium, Firefox, and WebKit matrix:

| API family | Expanded tests |
| --- | ---: |
| Async | 4,503 |
| Sync | 1,977 |
| Shared/package tests | 21 |
| TodoMVC examples | 25 |
| Total | 6,526 |

There are 156 collected files. The remaining source file,
`tests/common/test_collect_handles.py`, is a pinned zero-byte placeholder and is
explicitly recorded as non-collected. The generated ledger SHA-256 is
`5ed8bb46510be560e9fc9287e7549d86af3bc5841f3c05c89f7b532ab3b787f5`.
The initial source-level classification records 5,806 `public-api-source`
tests and 720 tests whose modules import upstream-private helpers. A private
helper import does not itself make a test non-applicable; each of those tests
remains in the execution and review ledger.

The execution matrix also records Linux, macOS, Windows, Python 3.9 through
3.13, and stable Chrome/Edge channels. The current local collection is macOS;
it does not substitute for those later platform executions.

## Current execution state

- Full Python oracle collection passes: 6,526 tests collected.
- Full Python oracle and candidate collection both pass at 6,526 tests. The
  candidate shim is positively verified as the loaded public package. Its
  explicit annotation-only placeholders fail if executed and therefore let
  collection enumerate the corpus without claiming missing behavior. Both
  collection XML files are preserved under
  `.work/official-conformance/results/`.
- The earlier false-green candidate collection was rejected because pytest was
  loading the upstream source package from its working directory. The runner
  now starts from the Pwright repository, uses importlib mode, and asserts the
  loaded package path before running pytest.
- Seven official Python Chromium files plus the complete async/sync global
  request files, six stop/cancel rows and 12 certificate rows have recorded
  424 unchanged-upstream outcomes. The two added files are the async and sync
  init-script suites, 9/9 each.
  `tests/async/test_browser.py` passes 4/4 after Rust-owned
  `Browser.new_page` lifecycle was added; its sync counterpart passes 1/1.
  `tests/async/test_page_base_url.py` now passes 7/7 after URL construction,
  route fulfillment and installed-Chromium persistent launch moved into the
  Rust host. `tests/async/test_wait_for_url.py` passes 9/9
  through optional Rust URL-wait and set-content leaves. The complete
  `tests/async/test_page.py[chromium]` shard first recorded 67 pass, 57 fail,
  and one upstream skip with zero runner/fixture errors. The ABI 13 fixed tag
  rerun reproducibly supersedes exactly 18 rows, so its canonical report is 85
  pass, 39 fail and one skip. Rust locator actions/reads,
  selector evaluation, same-document URL publication, published Frame-tree
  lookup, optional media/timeout state, and the bounded network-event leaf
  raised that file from its first clean 13/125 baseline. After the bounded
  route-retirement fix, fresh isolated candidate and oracle runs each record
  124 passes and the same upstream FIXME skip across all 125 rows, with zero
  status differences. Its unchanged upstream
  request/response/timeout/close subset passes 14/14. The fixture
  adapter attaches the official launch fixture to dedicated CDP Chrome and
  binds the unchanged upstream HTTP/HTTPS fixtures only to this Mac's Tailscale
  IP. The complete unchanged async/sync global request files pass 231/231
  candidate, with separate oracle results of 138/138 and 93/93. Six additional
  stop/cancel lifecycle rows pass candidate and oracle. The fixed certificate
  selection records eight passes and four upstream macOS WebKit skips in both
  lanes. The complete async/sync init-script files add 18/18 candidate and
  18/18 oracle passes. The tag manifest passes 18/18 and the JSHandle manifest
  passes 7/7 in candidate and oracle. The fixed drag manifest also passes 8/8
  in candidate and oracle. The persistent-Context manifest passes 1/1 in both
  lanes. The fixed Page-pause manifest passes 2/2 in both lanes. The async and
  sync request-GC files add 2/2 in both lanes, the async/sync Page-error
  history rows add 2/2, the async/sync console-message-history rows add 2/2,
  and the async/sync request-history rows add 2/2 in both lanes. The complete
  device-descriptor file adds one pass and two upstream browser skips. The
  6,526-record candidate outcome ledger is 437 passed, seven skipped, and 6,082
  `not-run`, SHA-256
  `c02c13e38b6b3dae4d83ec8ad280088dabd7432c30b70ecfeb83e83bf9e5cfc3`.
  The oracle outcome ledger is 418 passed, seven skipped and 6,101 `not-run`,
  SHA-256
  `a60219448e4b669fd5c3520186e93579b2e3fea6692492d483c0845a7fd3c644`.
  The historical isolated-runner regression separately records the pre-fix
  route-close hang and one following pass without contaminating the second
  pytest process; it is diagnostic infrastructure evidence, not current
  product evidence.
- Core JS/TS oracle inventory collection passes for all 20 entries. Full
  candidate execution has started but is nowhere near complete. A test-only
  preload redirects only `tests/config/testMode.ts`'s `playwright-core` lookup
  to the local Pwright package; the Playwright Test runner keeps its own
  official implementation. The preload supplies explicit empty fixture-only
  instrumentation views and maps its `launch` fixture to Pwright
  `connectOverCDP` so the base profile can obey external-browser ownership.
- The first full official Chromium file, `tests/page/page-basic.spec.ts`, now
  records 17 passes and one failure out of 18 after optional Rust-owned
  set-content, Page lifecycle and Node event-adapter leaves were added. The
  remaining assertion checks `navigator.webdriver`; the official fixture uses
  `launch`, while this constrained candidate lane currently maps launch to
  `connectOverCDP` against the dedicated external Chrome.
- `tests/page/page-event-console.spec.ts` progressed from 9/13 to 13/13 in its
  latest complete-file run. Binding-produced logs, console time/timeLog/timeEnd,
  native Log entries, detached iframes, object/error previews, unhandled
  rejections and the bounded 200-message history all pass together.
- The unchanged 24-test `tests/page/page-expose-function.spec.ts` file now
  passes 24/24 against the candidate. The first complete run was 21/24 and
  exposed missing `page.addInitScript`, `page.waitForNavigation`, and
  `page.click`. After those independent leaves were added, the second complete
  run reached 23/24 and exposed the missing `ElementHandle.innerText` facade in
  the click callback. The third complete run passes all binding values,
  Handles, callback errors, frames, cross-origin navigation, cycles, aliases,
  overridden builtins, navigation races and parallel registration. The
  navigation ticket is registered synchronously during N-API admission before
  the returned Promise can yield.
- The unchanged nine-test `tests/page/page-add-init-script.spec.ts` candidate
  file progressed from 8/9 to 9/9 after Page/Frame script-tag was added as a
  derived combinator over Rust-owned Frame evaluation and retained
  ElementHandle publication. The first complete unchanged
  `tests/page/page-add-script-tag.spec.ts` run was 9/12: two inline-module
  insertions incorrectly waited for a load event, and inline CSP did not reject.
  After matching the official immediate inline publication and observing the
  page's CSP violation event, the second complete run reached 10/12. An
  independent `wait-for-function` native feature now owns fixed
  eight-operation admission, Rust deadline/cancellation and exact truthy Handle
  publication. Each poll is a one-shot Frame evaluation; Rust probes the
  returned Handle, disposes each falsy attempt, and leaves no browser Promise
  alive after timeout or cancellation. The third complete script-tag run now
  passes 12/12, including URL, path, content, module insertion, sourceURL, CSP,
  failure-message and wait-for-function cases. The unchanged 26-test
  `page-wait-for-function.spec.ts` source also requires `page.$`,
  `waitForTimeout`, and inherited default timeouts. Those are now respectively
  a Handle-evaluation facade, an independent bounded Rust timer leaf, and the
  existing fixed Rust Context/Page timeout table exposed to Node. Their local
  gates initially raised the package suite to 214/214. The complete wait-for-function
  progression was 22/26, 24/26, 25/26, then 26/26. Context-loss polling now
  follows live Frame replacement across navigation, while a detached Frame
  returns the normalized execution-context-destroyed diagnostic. The final
  missing dependency, `ElementHandle.contentFrame`, directly describes the
  exact retained element and maps its bounded frame id through the existing
  topology owner; it creates no temporary Handle or second store. The
  wait-for-function leaf's 6/6 Rust AAA, timer leaf's 1/1 Rust AAA, timeout
  settings' 2/2 Rust AAA, package contracts, strict TypeScript, strict
  Clippy, release build and native-load gates pass. The cumulative core ledger
  is updated below with the later complete-file runs.
- The complete `page-event-load.spec.ts` progression was 0/2, 1/2 and 2/2.
  Rust now seeds the initial document and publishes each main-Frame
  loader/phase once from a fixed 16-Page table, so initial or duplicate CDP
  lifecycle events cannot masquerade as user navigation and iframe navigation
  cannot emit Page `load`. `FrameLocator` is a lazy language combinator over
  Rust wait-for-function, exact retained-element `contentFrame`, the existing
  Frame-scoped Locator, and a Rust-owned text-expectation polling operation.
  It releases the iframe owner Handle after resolution and retains no browser
  identity or timer in Node.
- The complete one-case `locator-list.spec.ts` progressed 0/1, 0/1 and 1/1.
  `Locator.all()` performs one Rust count then derives at most 1024 existing
  Rust `nth` recipes. Playwright's `div >> p` syntax is parsed into one bounded
  Rust `Within` selector tree while the original source remains under scratch
  admission; Node does not concatenate CSS or query the DOM.
- Complete Handle files now record `jshandle-evaluate` 2/2,
  `jshandle-as-element` 4/4, `jshandle-json-value` 3/3,
  `jshandle-to-string` 6/6, and `jshandle-properties` 7/7. The complete
  `page-evaluate-handle.spec.ts` file also passes 11/11.
- The complete unchanged `page-evaluate.spec.ts` file now passes 96/96. Its
  selected `large-evaluation-results` profile keeps ordinary results in the
  initial CDP reply and retains only a top-level string above 256 KiB for
  fixed-size projections. This preserves results when evaluation itself
  navigates while transferring the official 100 MiB string without embedding
  Chrome in the Node package. Result-dependent FFI admission is added only
  after Rust observes a large value; ordinary and nested evaluations retain
  the normal 8 MiB boundary allowance. The same run covers destroyed contexts,
  detached Frames, Handle arguments/jsonValue, CSP, overwritten globals and
  broken `Array.prototype` methods. The JSON report SHA-256 is
  `ff3777120fe00703ab86bd5840256bf78dbad60a21bfd25e177b8b16adb91e34`.
- The complete unchanged `locator-evaluate.spec.ts` file passes 6/6. Ordinary
  Locator evaluation keeps strict single-element resolution in Rust;
  `evaluateAll` uses a separately admitted Rust collection-evaluation lane that
  installs the selector runtime in the exact utility world, compiles the opaque
  selector recipe, and invokes one user function over the exact bounded match
  array. No selector graph or DOM identity crosses into Node.
- The complete unchanged `elementhandle-eval-on-selector.spec.ts` file passes
  6/6. `$eval` preserves the official missing-selector prefix and `$$eval`
  passes a real Array to the user callback while retaining/releasing every
  element owner through the existing Rust Handle path.
- The complete unchanged `locator-is-visible.spec.ts` file progressed from 6/8
  to 8/8. Rust now owns `Locator.waitFor`, visibility/hidden reads and matcher
  polling. In line with Playwright's `isVisibleInternal`, execution-context loss
  during navigation is a retriable read failure represented as not visible;
  selector parsing failures remain errors. Unknown engine detection is retained
  in the bounded Rust selector recipe and reports the exact Playwright message.
  The passing report SHA-256 is
  `722110906ed1e56592ddee8547ed1c6937fe5cff4f3baeb480d133cf455be309`.
- The complete unchanged `locator-click.spec.ts` file passes 5/5 through the
  existing typed Rust Locator action, including an overwritten page `Node`
  global, double-click dispatch, and targets removed by their pointerdown or
  pointerup handlers.
- The complete unchanged `elementhandle-type.spec.ts` and
  `elementhandle-press.spec.ts` files pass 5/5 and 5/5. Page and Frame `type`
  are thin language facades over the existing typed Rust Locator action; key
  parsing, sequential input, focus and browser work do not move into Node.
  The complete `page-focus.spec.ts` progression was 5 pass / one upstream skip /
  four fail, then 8 / 1 / 1, and finally 9 / 1 / 0. Page and Frame `focus` also
  delegate to the Rust Locator action. The final discrepancy showed that DOM
  focus completed before a binding callback was observable in Node. The Rust
  binding installation now provides a bounded, cancellation-aware publication
  fence: after topology publication, a successful action waits only until each
  admitted binding call through its watermark is acknowledged after callback
  invocation. It does not await the user's callback result. The complete-file
  report SHA-256 is
  `27c9e58d1269159714dba66985032ba84f9cbaec44a3a3f7bbccb115aa2e9a86`.
- The complete unchanged `page-emulate-media.spec.ts` file progressed from
  0/11 with an absent Node method to 11/11. `emulate-media` remains an
  independently selectable addon/host feature: Node validates Playwright enum
  values and preserves omitted versus `null` overrides, while Rust owns the
  fixed 64-Page state table, operation serialization and exact CDP command.
  Its passing report SHA-256 is
  `cd3ee047a67816c8209e414232299a157a95596f0028d10afcdcced243fbe4da`.
  Optional N-API Page methods are now grouped in feature-level impl blocks, so
  selecting only `emulate-media` compiles without exporting unrelated optional
  methods. The existing independent `extra-headers` leaf was then composed into
  the conformance artifact. At that historical checkpoint, the complete
  unchanged `page-set-extra-http-headers.spec.ts` file had a green 5/5 process
  report (SHA-256
  `430901f0b526c2a9e4ecafbcae1ac831d9a900b7f63b5dcbf4120dfc068f0751`).
  This is superseded as outcome evidence by the later paired NODE-429 run:
  both candidate and oracle classify four Page outcomes as expected and the
  Referer case as one unexpected PASS, because Chrome satisfies its assertion
  despite upstream's Chromium `test.fail` annotation. The earlier candidate
  had masked that assertion when `Response.request()` threw.
- The complete unchanged `page-wait-for-url.spec.ts` file progressed from
  0/11 with absent Page/Frame methods to 11/11. Rust owns the bounded
  event-driven wait, selected Frame identity, lifecycle state, cancellation
  and timeout. Node invokes only the language matcher, preserving string-glob,
  JavaScript `RegExp`, and `URL` predicate semantics. The passing file includes
  child Frames, anchors, push/replace/back/forward history, `commit`,
  `domcontentloaded`, `load`, and the exact timeout diagnostic. Its baseline
  and passing report SHA-256 values are
  `82171341c8fe4a00f7064bd6c5504fcbd6d0025bc30bf1d630a75e13ac736136` and
  `43a5dafb0d6642cffb61061b17a9c01c3640118ac774753838b439ace300f523`.
- The complete unchanged `page-request-gc.spec.ts` file progressed from 0/1
  with an absent Node method to 1/1. `request-gc` is an independently
  selectable zero-persistent-state host/addon feature. Rust resolves the
  Page's current Chromium session and sends the exact
  `HeapProfiler.collectGarbage` command with an empty payload; Node contributes
  only the Page facade. Its baseline and passing report SHA-256 values are
  `e16d57c318a3c12cb15770d0ff47ae20e9d642ecd2f97943aeacf8e094b5a35c` and
  `18bce9f2941cb1b6be55d7101b9fe8cb71656d535f8e782a30a1f10f48449f88`.
- The complete unchanged `page-navigation.spec.ts` file progressed from 0/3 to
  0/3, 0/3, 2/3, and finally 3/3. The first run exposed quoted Playwright text
  shorthand being passed to CSS; the second and third exposed a paused popup
  whose Context settings could not finish before debugger resume; the fourth
  exposed Playwright's text semantics for submit-input values. Rust now parses
  quoted/text selector leaves and bounded `>>` chains, starts caller settings,
  lifecycle setup, Page enable, recursive auto-attach and debugger resume as
  one fixed five-owner discovery batch, and publishes the Page only after every
  startup and world-creation fence succeeds. Selector runtime version 8 reads
  `input[type=submit|button]` values while keeping browser text extraction in
  the Rust-owned runtime. The five complete-file report SHA-256 values are
  `25fb09d38d21c08f52f44328d03bcece9705a383d7db4e25dc2a4e0d1e6caef6`,
  `596440b59814b9688c4cbc9b3ee6329fb9bb62f929da534400e2973b463943a5`,
  `456bf056d2ee49e9f97ab77c988a03e11b4d97452b0f02dfc47b5a75dba3ecb4`,
  `57aee70805b5d80a0a8a7635629f4d0c9a28039803ebd35292510402c3cbcde9`,
  and
  `de9f28ea77db394e6b6d18f272e0fa81eda03ce2a0e00c22b61adc98b48d61b2`.
  A post-Page-facade regression run remains 3/3, SHA-256
  `6c7049be945943dff9a3c8a0571d59282eebde7d641fc0230b6ebef60beeb144`,
  and is the report currently referenced by the formal ledger.
- The complete one-case `page-click-during-navigation.spec.ts` candidate run
  records one unexpected result. The unchanged test casts a private
  `__testHookAfterStable` option through `any`; the pinned implementation reads
  that option only in `packages/playwright-core/src/server/dom.ts`, outside the
  public API types. Pwright rejects the private option rather than silently
  moving an action-phase callback into Node. The report SHA-256 is
  `68c8a779232a099e93a98e2201da2164c17dccd94f735420c02c67d7bfea6dd6`.
  It remains an unexpected candidate result until the invocation receives a
  reproducible applicability adjudication or a composable Rust test-hook port.
- The complete two-case `page-click-timeout-2.spec.ts` progressed from 0/2 to
  0/2. Page/Frame selector actions now pass their actual public API identity
  into the same Rust Locator action, and standard timeout punctuation matches
  Playwright. Both unchanged tests advance past that assertion and now expose
  the same missing capability: bounded Rust actionability progress diagnostics
  must retain the last visible/enabled/stable retry reason for timeout
  formatting. No generic or fabricated reason is appended. The two report
  SHA-256 values are
  `c9ddf024a83464b62d946f50a37aec273403be0f72308e61e9334c756132e329`
  and
  `1725da197a26561620454387ff82f5d50d3a690d008e16c5db0cabf279490a0e`.
- A controlled five-file Page batch covers 12 unchanged upstream cases in
  `frame-goto.spec.ts`, `locator-highlight.spec.ts`, `matchers.misc.spec.ts`,
  `page-click-react.spec.ts`, and `page-click-timeout-1.spec.ts`. The first run
  recorded 3 expected / 9 unexpected, SHA-256
  `e27017c69f3f138bb0796ce59025ba16b4819fc91833a05e8755290062525a74`.
  Rust click actionability now asks the shared selector runtime for the
  `enabled` state, which retargets a text-matched descendant to its disabled
  interactive ancestor without changing click geometry. The second run stays
  3/9 because the unchanged test also requires rich retry diagnostics, but it
  proves the disabled button is no longer clicked; SHA-256
  `3032e14977b9f593edfb021b3719b06f4d855459d5f41e5b96c67bd78de8892c`.
  Rust host error projection now preserves the typed `Frame was detached`
  message instead of flattening it to generic target closure. A focused
  complete `frame-goto.spec.ts` run moves from 1/4 to 2/4, SHA-256
  `a61c7c0a8f84b27a4ba4efad98c1f56b5f0c1ee63a033601aae62e7aa7866fd2`,
  and the replacement 12-case batch records 4 expected / 8 unexpected,
  SHA-256
  `f7a49cdfc0cddc8c020314fc7b762ef4ed7adfd2d05a6bdc3f2c2e3881aceaf6`.
  The independently selectable Rust `response-body` leaf then preserves the
  completed document's bounded request identity, resolves its exact Page and
  Frame, sends `Network.getResponseBody`, decodes bounded base64 and retains
  its lease through FFI conversion. Node only exposes and caches the copied
  Buffer for `body`, `text` and `json`. The focused complete Frame goto file
  moves from 2/4 to 3/4, SHA-256
  `5c7b5ced18dfffdaf20b7e9b5b114b408d23169c5fe339e0dbec3d200876ee30`,
  and the replacement batch records 5 expected / 7 unexpected, SHA-256
  `c8f221f387a54bb17aeeb7bec6770f0db4e1ca7c748ea0c4d902f81f2fad11d5`.
  Rust-owned state expectations first move the complete unchanged
  `matchers.misc.spec.ts` file to 1/2. Deep property comparison then advances
  the second test to its negative text assertion, where the run remains 1/2
  and exposes an incorrect missing-element shortcut. Browser-RegExp matching
  and corrected missing-element polarity move the complete file to 2/2; the
  passing report SHA-256 is
  `d36423fbd9755b7c4d9ff912c921994407525aad31ae47ea8c7fd48f2b2884ed`.
  The replacement controlled batch records 7 expected / 5 unexpected,
  SHA-256
  `89a8344a244e83bb0721b9701d595fa690b23fdee5b3bae8cdc6407fa060bb1f`.
  Independently selectable Rust `locator-bounding-box` and
  `locator-highlight` leaves then reuse the exact Locator evaluation owner;
  highlight explicitly composes bounding-box, installs one bounded overlay
  state in the selected Frame utility world, replaces it on a repeated call,
  and relies on world teardown for navigation cleanup. Node retains no browser
  logic or overlay owner. The complete unchanged `locator-highlight.spec.ts`
  passes 1/1, report SHA-256
  `3e5b15bf7242fd3457978739932f0f243d74a007ba26ad48343207310457dec6`,
  and the replacement controlled batch records 8 expected / 4 unexpected,
  SHA-256
  `72d08fa3d73c9e17f5f6e7157e2313b6c7d53ffc6c3ac534321d68e64d168643`.
- The cumulative core ledger now records 351 expected, one explicit upstream
  skip, 10 unexpected, and 48,115 explicitly `not-run`, SHA-256
  `d95617557ce968078630912a613502516b185af752d88866702b4b3e24e28d8f`.
  The prior six unexpected results remain explicit: `page-basic.spec.ts`'s
  `navigator.webdriver` assertion in the external-CDP launch adapter, plus the
  noopener popup/network-URL timeout and missing `page.route` cases in
  `page-wait-for-load-state.spec.ts`, the private-hook
  `page-click-during-navigation.spec.ts` case, and both missing rich-action-log
  cases in `page-click-timeout-2.spec.ts`. The four remaining results in the
  controlled batch are client-redirect continuation, dialog events, one
  private pointer-action hook, and one missing rich enabled-state retry log.
  None is hidden or reclassified.
- After this slice, the Node package contract suite passes 256/256, strict
  TypeScript, the official-profile native-load smoke and strict selected-profile
  Clippy pass. The full selected host release library suite records 616 passed / 14
  explicitly ignored in its library target, plus 22 passed / 8 explicitly
  ignored across its integration targets. The selector backend passes 114/114
  and the Chromium composition library passes 397 / 16 explicitly ignored;
  ignored tests retain their dedicated Chrome, fixture, or diagnostic
  prerequisites.
- The Page-binding leaf now filters CDP events by its exact binding name and
  versioned payload, retains at most 64 registrations/calls, serializes browser
  values (including cycles and Window/Document/Node aliases) in the Rust-owned
  injected controller, and routes callback results through the existing Rust
  evaluation/Handle owner. `handle: true` uses a separately owned native
  JSHandle/ElementHandle rather than exporting a CDP object ID to Node. These
  account for the binding-specific results in the final 24/24 complete-file
  run. `ElementHandle.innerText` is a thin facade over the existing retained
  Rust Handle-evaluation owner; it does not add another browser-state layer.
- A focused post-pass process probe still reports two retained native leases at
  Node environment teardown. Functional 24/24 evidence does not satisfy the
  separate lifecycle-cleanup gate; this warning remains open and must be
  eliminated before the profile is considered production-ready.
- The browser run left no test page or incognito context visible through CDP;
  only the pre-existing XLQY page remained. Dedicated Chrome nevertheless
  measured about 3.36 GiB aggregate RSS with a 1.57 GiB renderer and no swap.
  Because that renderer owns the pre-existing shared page, this is a memory
  warning and attribution limit, not a Pwright-test leak claim.

## Reproduction

Generate the core ledger after preparing the pinned upstream runner
dependencies:

```bash
uv run --no-project python scripts/inventory_playwright_official_tests.py \
  /Users/cyuan/projects/thirdparty/playwright-v1.58.0
```

Generate the Python ledger in an environment containing the pinned editable
source and its test requirements:

```bash
uv run --no-project \
  --with-editable /Users/cyuan/projects/thirdparty/playwright-python \
  --with-requirements /Users/cyuan/projects/thirdparty/playwright-python/local-requirements.txt \
  --with-requirements /Users/cyuan/projects/thirdparty/playwright-python/requirements.txt \
  python scripts/inventory_playwright_python_official_tests.py \
  /Users/cyuan/projects/thirdparty/playwright-python
```

`run_playwright_python_official_conformance.py` now selects the full corpus by
default. `--bootstrap-only` is the only way to invoke the historical seven-case
harness.

## Next gates

1. Review and refine every initial core and Python source classification into
   a reproducible applicability decision without removing any test from the
   inventory.
2. Continue complete unchanged upstream-file execution from the recorded 96/96
   Page evaluation result; every new slice must merge
   into the 48,477-record ledger rather than replace or silently filter it.
3. Diagnose and eliminate the two retained native leases reported by the
   focused Page-binding process probe, then preserve a zero-retention teardown
   result alongside the completed 24/24 functional report.
4. Implement the next Python discrepancy families as independent composable
   capabilities: persistent Context launch, request/network events and routing,
   bindings, script/style tags, and before-unload handling. Init scripts are
   complete for the pinned async/sync files but remain subject to the final
   all-corpus and lifecycle gates.
5. Expand the Node package-substitution adapter beyond the declared `library`
   suite only when each additional official config can positively prove that
   it loaded Pwright.
6. Execute both corpora in bounded shards, preserving upstream assertions and recording all
   failures, skips, cleanup state, and memory measurements.
7. Implement missing behavior as composable Rust capabilities with thin
   bindings, then rerun the affected shards and the cumulative full corpus.
